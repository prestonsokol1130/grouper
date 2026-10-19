//! The in-memory app state plus debounced, atomic saves to disk.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::disk::{read_state, Loaded, StatePaths};
use super::disk_write::write_state;
use super::migrate::migrate;
use super::model::{AppState, CURRENT_SCHEMA};
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
    /// Why changes are refused, if they are (newer schema, or no usable file).
    read_only: Option<String>,
    /// Serialises writers (the flusher thread and the exit flush).
    save_lock: Mutex<()>,
    /// Held across an update and its publish so emits arrive in commit order.
    /// Always taken before `state`.
    publish_lock: Mutex<()>,
    wake: Mutex<Option<Sender<()>>>,
}

/// Locks a mutex, recovering the data if a panicking thread poisoned it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Store {
    fn new(dir: &Path, state: AppState, dirty: bool, main_good: bool, ro: Option<String>) -> Self {
        Self {
            paths: StatePaths::new(dir),
            state: Mutex::new(state),
            dirty: AtomicBool::new(dirty),
            main_is_good: AtomicBool::new(main_good),
            read_only: ro,
            save_lock: Mutex::new(()),
            publish_lock: Mutex::new(()),
            wake: Mutex::new(None),
        }
    }

    /// Loads the state from `dir` (created if missing), migrating and
    /// validating it. Bad JSON falls back to `.bak`, then to defaults. Errors
    /// (unreadable files, an unparsable newer schema) leave every file as is.
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        let paths = StatePaths::new(dir);
        let (state, mut dirty, main_good) = match read_state(&paths)? {
            Loaded::Fresh | Loaded::Corrupt => (AppState::default(), true, true),
            Loaded::Main(state) => (state, false, true),
            Loaded::Backup(state) => (state, true, false),
            Loaded::Newer(state) => (state, false, true),
        };
        if state.schema_version > CURRENT_SCHEMA {
            let reason = format!(
                "app_state.json is schema v{} and this build only knows v{CURRENT_SCHEMA}",
                state.schema_version
            );
            log::error!("app state is read-only: {reason}");
            return Ok(Self::new(dir, state, false, main_good, Some(reason)));
        }
        let version = state.schema_version;
        let mut state = migrate(state);
        let dropped = state.validate();
        if dropped > 0 {
            log::warn!("dropped {dropped} dangling id(s) from app state");
        }
        // Rewrite a file that migrate or validate had to change.
        dirty |= dropped > 0 || state.schema_version != version;
        Ok(Self::new(dir, state, dirty, main_good, None))
    }

    /// A read-only store with default state, for when `load` failed. Grouper
    /// still runs, but nothing is saved and the files on disk are untouched.
    pub fn in_memory(dir: &Path, reason: String) -> Self {
        log::error!("app state is in memory only: {reason}");
        Self::new(dir, AppState::default(), false, false, Some(reason))
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
    /// change's value plus the new state. A failed change is rolled back, and
    /// a read-only store refuses every change.
    pub fn update<T>(
        &self,
        change: impl FnOnce(&mut AppState) -> Result<T, String>,
    ) -> Result<(T, AppState), String> {
        if let Some(reason) = &self.read_only {
            return Err(format!("Changes cannot be saved: {reason}"));
        }
        let mut guard = lock(&self.state);
        let mut next = guard.clone();
        let value = change(&mut next)?;
        next.validate();
        *guard = next.clone();
        drop(guard);
        self.mark_dirty();
        Ok((value, next))
    }

    /// Like [`Store::update`], then calls `publish` with the new state while
    /// no other update can run, so publishes happen in commit order.
    pub fn update_and_publish<T>(
        &self,
        change: impl FnOnce(&mut AppState) -> Result<T, String>,
        publish: impl FnOnce(&AppState),
    ) -> Result<T, String> {
        let _order = lock(&self.publish_lock);
        let (value, state) = self.update(change)?;
        publish(&state);
        Ok(value)
    }

    /// Writes the current state to disk now.
    pub fn save(&self) -> Result<(), String> {
        if let Some(reason) = &self.read_only {
            return Err(format!("not saving app state: {reason}"));
        }
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
