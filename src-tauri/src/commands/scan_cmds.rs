//! Start Menu scans: the `rescan_apps` command and the first-run scan.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::state_cmds::emit_state_changed;
use crate::scanner::{self, merge::merge_scan};
use crate::state::{AppEntry, Store};
use crate::util::spawn::spawn_logged;

/// Merges `apps` into the store and emits `state:changed`. Returns how many
/// apps the scan found.
fn apply_scan(app: &AppHandle, store: &Store, apps: Vec<AppEntry>) -> Result<u32, String> {
    // u32, not usize: the TypeScript bindings refuse 64-bit integers.
    let found = u32::try_from(apps.len()).unwrap_or(u32::MAX);
    let summary = store.update_and_publish(
        |state| Ok(merge_scan(state, apps)),
        |state| emit_state_changed(app, state),
    )?;
    log::info!(
        "app scan merged: {found} found, {} added, {} updated, {} now missing",
        summary.added,
        summary.updated,
        summary.missing
    );
    Ok(found)
}

/// Scans the Start Menu again, merges the result into the app list and
/// returns how many apps were found.
#[tauri::command]
#[specta::specta]
pub async fn rescan_apps(app: AppHandle, store: State<'_, Arc<Store>>) -> Result<u32, String> {
    let store = Arc::clone(&store);
    // The scan blocks while it waits for its own thread; keep that off the
    // async runtime's worker threads.
    let apps = tauri::async_runtime::spawn_blocking(scanner::scan)
        .await
        .map_err(|e| format!("app scan did not finish: {e}"))??;
    apply_scan(&app, &store, apps)
}

/// On startup, scans in the background if the app list is still empty.
pub fn scan_if_empty(app: &AppHandle, store: &Arc<Store>) -> Result<(), String> {
    if !store.snapshot().apps.is_empty() {
        return Ok(());
    }
    let app = app.clone();
    let store = Arc::clone(store);
    spawn_logged("startup-scan", move || {
        let apps = scanner::scan()?;
        apply_scan(&app, &store, apps).map(|_| ())
    })
    .map(|_| ())
}
