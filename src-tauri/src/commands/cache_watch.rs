//! Rebuilding the icon cache when someone deletes it while Grouper runs.
//!
//! The watch thread sleeps until something under the app data folder is
//! created, deleted or renamed (a change notification, so an idle Grouper
//! never wakes), lets a burst of changes settle, then checks the cache.
//! If the folder cannot be watched it checks every
//! [`CACHE_CHECK_INTERVAL`] instead, and tries to watch again.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use super::icon_cmds::queue_icons_then;
use super::shortcut_worker::{queue, Job};
use crate::icons::cache;
use crate::state::paths::{ensure_dirs, group_cache_dir, icon_cache_dir};
use crate::state::Store;
use crate::util::spawn::spawn_logged;
use crate::win32::dir_watch::DirWatch;

/// How often the cache is checked when the folder cannot be watched, and
/// the longest a burst of changes can put a check off.
const CACHE_CHECK_INTERVAL: Duration = Duration::from_secs(3);

/// A change is acted on once the folder has been quiet this long.
const SETTLE: Duration = Duration::from_millis(500);

/// True when the cache folders or the fallback icon are gone (someone
/// deleted the cache while Grouper runs).
pub fn cache_needs_rebuild(app_data_dir: &Path) -> bool {
    let icons = icon_cache_dir(app_data_dir);
    !icons.is_dir()
        || !group_cache_dir(app_data_dir).is_dir()
        || !cache::fallback_path(&icons).is_file()
}

/// Watches for a deleted cache. When it is gone the folders and the
/// fallback icon are written again, every app icon is queued
/// (`icons:updated` then reloads them in every window) and, once they are
/// back, the shortcut repair rebuilds each tracked group `.ico`. A failure
/// to start is logged.
pub fn start_cache_watch(app: &AppHandle, store: &Arc<Store>) {
    let Ok(data) = app.path().app_data_dir() else {
        log::warn!("no app data dir; the icon cache is not watched");
        return;
    };
    let (app, store) = (app.clone(), Arc::clone(store));
    let started = spawn_logged("cache-watch", move || {
        watch(&app, &store, &data);
        Ok(())
    });
    if let Err(error) = started {
        log::warn!("the icon cache is not watched: {error}");
    }
}

fn watch(app: &AppHandle, store: &Store, data: &Path) {
    let mut watcher = watch_folder(data);
    loop {
        let Some(folder) = &watcher else {
            std::thread::sleep(CACHE_CHECK_INTERVAL);
            if check(app, store, data) {
                watcher = watch_folder(data);
            }
            continue;
        };
        match folder.wait(None) {
            Ok(_) => {
                settle(folder);
                // A watch keeps a deleted folder from going away, so drop
                // it when the folders cannot be written again.
                if !check(app, store, data) {
                    watcher = None;
                }
            }
            Err(error) => {
                log::info!("icon cache watch stopped ({error}); checking every few seconds");
                watcher = None;
            }
        }
    }
}

/// A watch on the app data folder, or `None` (logged) to check by time.
fn watch_folder(data: &Path) -> Option<DirWatch> {
    DirWatch::new(data)
        .map_err(|error| log::info!("icon cache checked every few seconds: {error}"))
        .ok()
}

/// Waits until the folder has been quiet for [`SETTLE`], but no longer than
/// [`CACHE_CHECK_INTERVAL`] in all.
fn settle(folder: &DirWatch) {
    let start = Instant::now();
    while start.elapsed() < CACHE_CHECK_INTERVAL {
        if !matches!(folder.wait(Some(SETTLE)), Ok(true)) {
            return;
        }
    }
}

/// Rebuilds the cache if it is gone. False when its folders could not be
/// written again.
fn check(app: &AppHandle, store: &Store, data: &Path) -> bool {
    if !cache_needs_rebuild(data) {
        return true;
    }
    log::warn!("the icon cache was deleted; rebuilding it");
    if let Err(error) = ensure_dirs(data) {
        log::warn!("could not recreate the cache folders: {error}");
        return false;
    }
    if let Err(error) = cache::ensure_fallback(&icon_cache_dir(data)) {
        log::warn!("could not write the fallback icon: {error}");
    }
    let repair = app.clone();
    queue_icons_then(app, store.snapshot().apps, move || {
        queue(&repair, Job::Repair)
    });
    true
}

#[cfg(test)]
mod tests {
    use super::cache_needs_rebuild;
    use crate::icons::cache::ensure_fallback;
    use crate::state::paths::{cache_dir, ensure_dirs, icon_cache_dir};

    #[test]
    fn a_deleted_cache_needs_a_rebuild_until_it_is_written_again() {
        let data = tempfile::tempdir().unwrap();
        assert!(cache_needs_rebuild(data.path()));
        ensure_dirs(data.path()).unwrap();
        assert!(cache_needs_rebuild(data.path()), "no fallback icon yet");
        ensure_fallback(&icon_cache_dir(data.path())).unwrap();
        assert!(!cache_needs_rebuild(data.path()));
        std::fs::remove_dir_all(cache_dir(data.path())).unwrap();
        assert!(cache_needs_rebuild(data.path()));
    }
}
