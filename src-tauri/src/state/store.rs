//! The in-memory app state plus debounced, atomic saves to disk.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::disk::{read_state, write_state, Loaded, StatePaths};
use super::migrate::migrate;
use super::model::AppState;
use crate::util::spawn::spawn_logged;

/// Saves are batched: at most one write per this interval.
pub const SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

/// Owns the app state and writes it to `{dir}/app_state.json`.
pub struct Store {
    paths: StatePaths,
    state: Mutex<AppState>,
    dirty: AtomicBool,
    /// False while the main file on disk is known bad (loaded from `.bak`).
    main_is_good: AtomicBool,
    /// Serialises writers (the flusher thread and the exit flush).
    save_lock: Mutex<()>,
    wake: Mutex<Option<Sender<()>>>,
}

/// Locks a mutex, recovering the data if a panicking thread poisoned it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Store {
    /// Loads the state from `dir` (created if missing), validating and
    /// migrating it. Bad files fall back to `.bak`, then to defaults.
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        let paths = StatePaths::new(dir);
        // Anything but a clean main file is rewritten on the first flush.
        let (state, main_is_good, dirty) = match read_state(&paths) {
            Loaded::Fresh => (AppState::default(), true, true),
            Loaded::Main(state) => (state, true, false),
            Loaded::Backup(state) => (state, false, true),
            Loaded::Corrupt => (AppState::default(), true, true),
        };
        let mut state = migrate(state);
        let dropped = state.validate();
        if dropped > 0 {
            log::warn!("dropped {dropped} dangling id(s) from app state");
        }
        Ok(Self {
            paths,
            state: Mutex::new(state),
            dirty: AtomicBool::new(dirty),
            main_is_good: AtomicBool::new(main_is_good),
            save_lock: Mutex::new(()),
            wake: Mutex::new(None),
        })
    }

    /// Path of `app_state.json`.
    pub fn path(&self) -> &PathBuf {
        &self.paths.main
    }

    /// A copy of the current state.
    pub fn snapshot(&self) -> AppState {
        lock(&self.state).clone()
    }

    /// Applies `change`, validates, marks the store dirty and returns the
    /// change's value plus the new state. A failed change is rolled back.
    pub fn update<T>(
        &self,
        change: impl FnOnce(&mut AppState) -> Result<T, String>,
    ) -> Result<(T, AppState), String> {
        let mut guard = lock(&self.state);
        let mut next = guard.clone();
        let value = change(&mut next)?;
        next.validate();
        *guard = next.clone();
        drop(guard);
        self.mark_dirty();
        Ok((value, next))
    }

    /// Writes the current state to disk now.
    pub fn save(&self) -> Result<(), String> {
        let _writer = lock(&self.save_lock);
        self.dirty.store(false, Ordering::SeqCst);
        let state = self.snapshot();
        let backup_current = self.main_is_good.load(Ordering::SeqCst);
        match write_state(&self.paths, &state, backup_current) {
            Ok(()) => {
                self.main_is_good.store(true, Ordering::SeqCst);
                Ok(())
            }
            Err(error) => {
                self.dirty.store(true, Ordering::SeqCst);
                Err(error)
            }
        }
    }

    /// Saves only if something changed since the last save.
    pub fn flush(&self) -> Result<(), String> {
        if self.dirty.load(Ordering::SeqCst) {
            self.save()
        } else {
            Ok(())
        }
    }

    /// Records a change; the flusher thread saves it within [`SAVE_DEBOUNCE`].
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::SeqCst);
        if let Some(wake) = lock(&self.wake).as_ref() {
            // A closed channel means the flusher is gone; flush() on exit
            // still saves, so there is nothing more to do here.
            let _ = wake.send(());
        }
    }

    /// Starts the background thread that performs debounced saves.
    pub fn start_flusher(self: &Arc<Self>) -> Result<(), String> {
        let (sender, receiver) = mpsc::channel();
        if self.dirty.load(Ordering::SeqCst) {
            // Changes made before the flusher existed still get saved.
            let _ = sender.send(());
        }
        *lock(&self.wake) = Some(sender);
        let store = Arc::clone(self);
        spawn_logged("state-flusher", move || {
            flush_loop(&store, &receiver);
            Ok(())
        })
        .map(|_| ())
    }

    /// Stops the flusher thread and saves any pending change.
    pub fn shutdown(&self) -> Result<(), String> {
        lock(&self.wake).take();
        self.flush()
    }
}

/// Waits for a change, lets more changes pile up for [`SAVE_DEBOUNCE`], then
/// saves once. Ends when the sender is dropped.
fn flush_loop(store: &Store, receiver: &Receiver<()>) {
    while receiver.recv().is_ok() {
        std::thread::sleep(SAVE_DEBOUNCE);
        while receiver.try_recv().is_ok() {}
        if let Err(error) = store.flush() {
            log::error!("saving app state failed: {error}");
        }
    }
}
