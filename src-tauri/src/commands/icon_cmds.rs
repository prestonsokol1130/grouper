//! The app icon cache: the `get_cache_dir` command, the background worker
//! and the `icons:updated` event.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::icons::cache::{self, Ensured};
use crate::icons::extract::extract_icon;
use crate::icons::worker::{IconWorker, THREADS};
use crate::state::paths::icon_cache_dir;
use crate::state::AppEntry;

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

/// Returns the icon cache folder, `{app_data_dir}/cache/icons`. App icons
/// are `{appId}.png` in it and the fallback icon is `_fallback.png`.
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
    let Some(worker) = app.try_state::<Arc<IconWorker>>() else {
        log::warn!("the icon worker is not running; app icons were not queued");
        return;
    };
    let emitter = app.clone();
    let queued = worker.queue(apps, move |ids| {
        if ids.is_empty() {
            return;
        }
        log::info!("cached {} app icons", ids.len());
        if let Err(error) = emitter.emit(ICONS_UPDATED_EVENT, ids) {
            log::warn!("could not emit {ICONS_UPDATED_EVENT}: {error}");
        }
    });
    if let Err(error) = queued {
        log::warn!("could not queue app icons: {error}");
    }
}

/// Stops the icon pool on exit without waiting for its threads.
pub fn stop_icon_worker(app: &AppHandle) {
    if let Some(worker) = app.try_state::<Arc<IconWorker>>() {
        drop(worker.shutdown());
    }
}
