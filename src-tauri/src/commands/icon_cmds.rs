//! The app icon cache: the `get_cache_dir` command, the background worker
//! and the `icons:updated` event.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use super::shortcut_worker::{queue, Job};
use crate::icons::cache::{self, Ensured};
use crate::icons::extract::extract_icon;
use crate::icons::gc;
use crate::icons::worker::{IconWorker, THREADS};
use crate::state::paths::{ensure_dirs, group_cache_dir, icon_cache_dir};
use crate::state::{AppEntry, AppState, Store};
use crate::util::spawn::spawn_logged;

/// How often the cache watch checks that the cache folders still exist.
const CACHE_CHECK_INTERVAL: Duration = Duration::from_secs(3);

/// Emitted to every window with the ids of the apps whose icon PNG was just
/// written, once per finished batch (never with an empty list).
pub const ICONS_UPDATED_EVENT: &str = "icons:updated";

/// `{app_data_dir}/cache/icons`.
fn icon_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?;
    Ok(icon_cache_dir(&data))
}

/// Returns the icon cache folder, `{app_data_dir}/cache/icons`.
// App icons are `{appId}.png` in it and the fallback icon is
// `_fallback.png`.
#[tauri::command]
#[specta::specta]
pub fn get_cache_dir(app: AppHandle) -> Result<String, String> {
    let dir = icon_dir(&app)?;
    dir.to_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("icon cache dir {} is not valid Unicode", dir.display()))
}

/// Writes the fallback icon if it is missing, then starts the icon pool.
/// Each pool thread checks one app's PNG and extracts it only when it is
/// missing or stale; it reports the app when a new PNG was written.
pub fn start_icon_worker(app: &AppHandle) -> Result<Arc<IconWorker>, String> {
    let dir = icon_dir(app)?;
    if let Err(error) = cache::ensure_fallback(&dir) {
        log::warn!("could not write the fallback icon: {error}");
    }
    let worker = IconWorker::start(
        THREADS,
        Arc::new(move |entry: &AppEntry| {
            match cache::ensure_icon_with(entry, &dir, &extract_icon) {
                Ok(Ensured::Written(_)) => Some(entry.id.clone()),
                Ok(Ensured::Fresh(_) | Ensured::Fallback(_)) => None,
                Err(error) => {
                    log::warn!("could not cache the icon of app {}: {error}", entry.id);
                    None
                }
            }
        }),
    )?;
    Ok(Arc::new(worker))
}

/// Queues `apps` for the icon pool and returns at once. Apps whose PNG is
/// already fresh are skipped by the pool. When the batch is done,
/// `icons:updated` is emitted with the ids whose PNG was written.
pub fn queue_icons(app: &AppHandle, apps: Vec<AppEntry>) {
    queue_icons_then(app, apps, || {});
}

/// [`queue_icons`], then runs `after` once the batch is done (at once for
/// no apps; never when the batch could not be queued).
fn queue_icons_then(app: &AppHandle, apps: Vec<AppEntry>, after: impl FnOnce() + Send + 'static) {
    let Some(worker) = app.try_state::<Arc<IconWorker>>() else {
        log::warn!("the icon worker is not running; app icons were not queued");
        return;
    };
    if apps.is_empty() {
        after();
        return;
    }
    let emitter = app.clone();
    let queued = worker.queue(apps, move |ids| {
        if !ids.is_empty() {
            log::info!("cached {} app icons", ids.len());
            if let Err(error) = emitter.emit(ICONS_UPDATED_EVENT, ids) {
                log::warn!("could not emit {ICONS_UPDATED_EVENT}: {error}");
            }
        }
        after();
    });
    if let Err(error) = queued {
        log::warn!("could not queue app icons: {error}");
    }
}

/// Deletes cached icons nothing in `state` uses any more (see
/// [`gc::collect`]). Run at startup, before the icon worker starts.
pub fn collect_cache_garbage(app: &AppHandle, state: &AppState) {
    match app.path().app_data_dir() {
        Ok(data) => {
            let removed = gc::collect(&data, state);
            log::info!(
                "icon cache clean-up removed {} PNG(s) and {} .ico file(s)",
                removed.pngs,
                removed.icos
            );
        }
        Err(error) => log::warn!("no icon cache clean-up: {error}"),
    }
}

/// True when the cache folders or the fallback icon are gone (someone
/// deleted the cache while Grouper runs).
pub fn cache_needs_rebuild(app_data_dir: &Path) -> bool {
    let icons = icon_cache_dir(app_data_dir);
    !icons.is_dir()
        || !group_cache_dir(app_data_dir).is_dir()
        || !cache::fallback_path(&icons).is_file()
}

/// Watches, every few seconds, for a deleted cache. When it is gone the
/// folders and the fallback icon are written again, every app icon is
/// queued (`icons:updated` then reloads them in every window) and, once
/// they are back, the shortcut repair rebuilds each tracked group `.ico`.
/// A failure to start is logged.
pub fn start_cache_watch(app: &AppHandle, store: &Arc<Store>) {
    let Ok(data) = app.path().app_data_dir() else {
        log::warn!("no app data dir; the icon cache is not watched");
        return;
    };
    let (app, store) = (app.clone(), Arc::clone(store));
    let started = spawn_logged("cache-watch", move || loop {
        std::thread::sleep(CACHE_CHECK_INTERVAL);
        if cache_needs_rebuild(&data) {
            log::warn!("the icon cache was deleted; rebuilding it");
            if let Err(error) = ensure_dirs(&data) {
                log::warn!("could not recreate the cache folders: {error}");
                continue;
            }
            if let Err(error) = cache::ensure_fallback(&icon_cache_dir(&data)) {
                log::warn!("could not write the fallback icon: {error}");
            }
            let repair = app.clone();
            queue_icons_then(&app, store.snapshot().apps, move || {
                queue(&repair, Job::Repair)
            });
        }
    });
    if let Err(error) = started {
        log::warn!("the icon cache is not watched: {error}");
    }
}

/// Stops the icon pool on exit without waiting for its threads.
pub fn stop_icon_worker(app: &AppHandle) {
    if let Some(worker) = app.try_state::<Arc<IconWorker>>() {
        drop(worker.shutdown());
    }
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
