//! Start Menu scans: the `rescan_apps` command and the first-run scan.

use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, State};

use super::icon_cmds::queue_icons;
use super::state_cmds::emit_state_changed;
use crate::scanner::{self, merge::merge_scan, ScanOutcome};
use crate::state::Store;
use crate::util::spawn::spawn_logged;

/// Held from the start of a scan until its merge is done, so two scans never
/// overlap and an older scan can never be merged over a newer one.
static SCAN_LOCK: Mutex<()> = Mutex::new(());

/// The one lock every scan takes; see [`SCAN_LOCK`].
fn scan_lock() -> &'static Mutex<()> {
    &SCAN_LOCK
}

/// Scans, then merges the result, all while holding the scan lock. A second
/// caller waits and scans after the first merge has finished. Blocks.
fn scan_and_apply(app: &AppHandle, store: &Store) -> Result<u32, String> {
    // A scan that panicked cannot have left a half-made merge behind (the
    // store rolls back failed updates), so a poisoned lock is safe to reuse.
    let _scanning = scan_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let scan = scanner::scan()?;
    apply_scan(app, store, scan)
}

/// Merges `scan` into the store, emits `state:changed` and queues the apps
/// for the icon cache. Returns how many apps the scan found.
fn apply_scan(app: &AppHandle, store: &Store, scan: ScanOutcome) -> Result<u32, String> {
    // u32, not usize: the TypeScript bindings refuse 64-bit integers.
    let found = u32::try_from(scan.apps.len()).unwrap_or(u32::MAX);
    let summary = store.update_and_publish(
        |state| Ok(merge_scan(state, scan)),
        |state| emit_state_changed(app, state),
    )?;
    log::info!(
        "app scan merged: {found} found, {} added, {} updated, {} now missing",
        summary.added,
        summary.updated,
        summary.missing
    );
    queue_icons(app, store.snapshot().apps);
    Ok(found)
}

/// Scans the Start Menu again, merges the result into the app list and
/// returns how many apps were found.
#[tauri::command]
#[specta::specta]
pub async fn rescan_apps(app: AppHandle, store: State<'_, Arc<Store>>) -> Result<u32, String> {
    let store = Arc::clone(&store);
    // The scan blocks (on the lock and on its own thread); keep that off the
    // async runtime's worker threads.
    tauri::async_runtime::spawn_blocking(move || scan_and_apply(&app, &store))
        .await
        .map_err(|e| format!("app scan did not finish: {e}"))?
}

/// On startup, scans in the background if the app list is still empty.
pub fn scan_if_empty(app: &AppHandle, store: &Arc<Store>) -> Result<(), String> {
    if !store.snapshot().apps.is_empty() {
        return Ok(());
    }
    let app = app.clone();
    let store = Arc::clone(store);
    spawn_logged("startup-scan", move || {
        scan_and_apply(&app, &store).map(|_| ())
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::scan_lock;

    #[test]
    fn a_held_scan_lock_blocks_other_scans() {
        let held = scan_lock().lock().unwrap();
        assert!(scan_lock().try_lock().is_err());
        drop(held);
        assert!(scan_lock().try_lock().is_ok());
    }
}
