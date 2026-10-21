//! The app icon cache: the `get_cache_dir` command, the background worker
//! and the `icons:updated` event.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use super::state_cmds::mutate;
use crate::icons::cache::{self, Ensured};
use crate::icons::extract::extract_icon;
use crate::icons::worker::{IconWorker, THREADS};
use crate::icons::{format, gc};
use crate::state::paths::icon_cache_dir;
use crate::state::{AppEntry, AppState, Store};

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

/// Brings the icon cache up to the current format ([`format::upgrade`]):
/// app PNGs from an older build are deleted so the pool redraws them.
/// Returns true when it did. Run at startup, before the icon worker.
pub fn upgrade_icon_cache(app: &AppHandle) -> bool {
    match icon_dir(app) {
        Ok(dir) => format::upgrade(&dir).is_some(),
        Err(error) => {
            log::warn!("no icon cache upgrade: {error}");
            false
        }
    }
}

/// Queues every app's icon, as [`queue_icons`]. After an icon cache
/// upgrade (`redrawn`), once they are all drawn again, every group's icon
/// version is bumped so each taskbar `.ico` is rebuilt from the new PNGs.
pub fn queue_startup_icons(app: &AppHandle, store: &Arc<Store>, redrawn: bool) {
    let apps = store.snapshot().apps;
    if !redrawn {
        queue_icons(app, apps);
        return;
    }
    let (handle, store) = (app.clone(), Arc::clone(store));
    queue_icons_then(app, apps, move || {
        let bumped = mutate(&handle, &store, |state| {
            for group in &mut state.groups {
                group.icon_version = group.icon_version.wrapping_add(1);
            }
            Ok(())
        });
        if let Err(error) = bumped {
            log::warn!("could not rebuild the group icons: {error}");
        }
    });
}

/// Queues `apps` for the icon pool and returns at once. Apps whose PNG is
/// already fresh are skipped by the pool. When the batch is done,
/// `icons:updated` is emitted with the ids whose PNG was written.
pub fn queue_icons(app: &AppHandle, apps: Vec<AppEntry>) {
    queue_icons_then(app, apps, || {});
}

/// [`queue_icons`], then runs `after` once the batch is done (at once for
/// no apps; never when the batch could not be queued).
pub(super) fn queue_icons_then(
    app: &AppHandle,
    apps: Vec<AppEntry>,
    after: impl FnOnce() + Send + 'static,
) {
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

/// Deletes cached app PNGs nothing in `state` uses any more (see
/// [`gc::collect_pngs`]). Run at startup, before the icon worker starts.
/// Deleted groups' `.ico` files go later, on the shortcut worker, after
/// their leftover shortcuts are unpinned.
pub fn collect_cache_garbage(app: &AppHandle, state: &AppState) {
    match app.path().app_data_dir() {
        Ok(data) => {
            let removed = gc::collect_pngs(&data, state);
            log::info!("icon cache clean-up removed {removed} PNG(s)");
        }
        Err(error) => log::warn!("no icon cache clean-up: {error}"),
    }
}

/// Stops the icon pool on exit without waiting for its threads.
pub fn stop_icon_worker(app: &AppHandle) {
    if let Some(worker) = app.try_state::<Arc<IconWorker>>() {
        drop(worker.shutdown());
    }
}
