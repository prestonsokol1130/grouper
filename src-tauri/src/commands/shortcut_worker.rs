//! The background thread that writes, updates, repairs and removes group
//! shortcuts, one job at a time, so a shortcut's changes land in order.
//!
//! The thread holds its own `ComGuard`. Each job reads the latest app
//! state when it runs, so a burst of changes ends on the newest one.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::shortcut_jobs::{create, run};
use crate::state::Store;
use crate::taskbar::shortcut_sync::Change;
use crate::util::spawn::spawn_logged;
use crate::win32::com::ComGuard;

/// Emitted with a [`ShortcutUpdated`] after a group's shortcut is
/// rewritten to follow the group (rename, icon change or repair).
pub const SHORTCUT_UPDATED_EVENT: &str = "group:shortcut-updated";

/// The payload of [`SHORTCUT_UPDATED_EVENT`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutUpdated {
    pub group_id: String,
    pub shortcut_path: String,
}

/// One piece of shortcut work.
pub enum Job {
    /// Write the group's shortcut, mark the group `has_shortcut`, and send
    /// the result back.
    Create {
        group_id: String,
        reply: Sender<Result<PathBuf, String>>,
    },
    /// The group changed; rewrite its shortcut if it has one.
    Update { group_id: String, change: Change },
    /// The group was deleted; remove its shortcut and `.ico` files.
    Remove { group_id: String },
    /// Rebuild every tracked shortcut whose `.lnk` or `.ico` is missing or
    /// that shows another icon file. Legacy versioned `.ico`s are kept.
    Repair,
}

/// The running worker, managed as Tauri state.
pub struct ShortcutWorker {
    sender: Mutex<Option<Sender<Job>>>,
}

impl ShortcutWorker {
    /// Starts the `shortcut-sync` thread.
    pub fn start(app: &AppHandle) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel::<Job>();
        let app = app.clone();
        spawn_logged("shortcut-sync", move || {
            let _com = ComGuard::new()?;
            while let Ok(job) = receiver.recv() {
                run(&app, job);
            }
            Ok(())
        })?;
        Ok(Self {
            sender: Mutex::new(Some(sender)),
        })
    }

    /// Stops taking jobs; the thread ends after its current one.
    pub fn stop(&self) {
        drop(lock_sender(self).take());
    }

    fn send(&self, job: Job) -> Result<(), String> {
        lock_sender(self)
            .as_ref()
            .ok_or("the shortcut worker is stopped")?
            .send(job)
            .map_err(|_| "the shortcut worker thread has ended".to_owned())
    }
}

fn lock_sender(worker: &ShortcutWorker) -> MutexGuard<'_, Option<Sender<Job>>> {
    worker.sender.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Queues `job`. When there is no worker the job is dropped and logged.
pub fn queue(app: &AppHandle, job: Job) {
    let sent = app
        .try_state::<ShortcutWorker>()
        .ok_or_else(|| "the shortcut worker is not running".to_owned())
        .and_then(|worker| worker.send(job));
    if let Err(error) = sent {
        log::warn!("shortcut job dropped: {error}");
    }
}

/// Starts the worker and manages it, tells it about every icon version
/// `store` bumps from now on, and queues the startup repair. A failed
/// start is logged; shortcuts then stay as they are.
pub fn start_shortcut_sync(app: &AppHandle, store: &Store) {
    match ShortcutWorker::start(app) {
        Ok(worker) => {
            app.manage(worker);
            let handle = app.clone();
            store.set_icon_listener(Box::new(move |ids| queue_icon_updates(&handle, ids)));
            queue(app, Job::Repair);
        }
        Err(error) => log::error!("could not start the shortcut worker: {error}"),
    }
}

/// Queues an icon update for each group in `group_ids` (groups whose icon
/// version a committed change bumped). The update does nothing for a group
/// that has no shortcut when it runs.
pub fn queue_icon_updates(app: &AppHandle, group_ids: &[String]) {
    for group_id in group_ids {
        let change = Change {
            renamed: false,
            icon: true,
        };
        let group_id = group_id.clone();
        queue(app, Job::Update { group_id, change });
    }
}

/// Creates group `group_id`'s shortcut on the worker and waits for it.
/// Without a worker the work runs on the calling thread. Blocks.
pub fn create_and_wait(app: &AppHandle, group_id: String) -> Result<PathBuf, String> {
    let Some(worker) = app.try_state::<ShortcutWorker>() else {
        let _com = ComGuard::new()?;
        return create(app, &group_id);
    };
    let (reply, answer) = mpsc::channel();
    worker.send(Job::Create { group_id, reply })?;
    answer
        .recv()
        .map_err(|_| "the shortcut worker stopped before writing the shortcut".to_owned())?
}
