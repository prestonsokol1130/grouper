//! Start Menu scans: the `rescan_apps` command and the startup scan.

use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, State};

use super::icon_cmds::queue_icons;
use super::state_cmds::emit_state_changed;
use crate::cli::LaunchCommand;
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

/// Scans the Start Menu again and returns how many apps were found.
// The result is merged into the app list.
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

/// Why a startup scan runs, or `None` when the launch must not scan.
///
/// Every normal start scans once (a sign-in `--background` launch too), so
/// apps installed since the last run show up without a manual Rescan; an
/// empty app list is the first run. The uninstaller's `--cleanup` never
/// scans (it exits before the app starts; this is a second guard). A second
/// launch that only forwards to the running Grouper never gets here.
/// `--bench flyout` never scans either: a scan and its icon extraction in
/// the background would skew the timings it measures.
pub fn startup_scan_reason(command: &LaunchCommand, known_apps: usize) -> Option<&'static str> {
    match command {
        LaunchCommand::Cleanup { .. } | LaunchCommand::BenchFlyout => None,
        LaunchCommand::Flyout { .. }
        | LaunchCommand::Launchpad
        | LaunchCommand::Manager
        | LaunchCommand::Background
        | LaunchCommand::None => Some(if known_apps == 0 {
            "the app list is empty"
        } else {
            "refreshing the app list"
        }),
    }
}

/// On startup, scans once in the background (see [`startup_scan_reason`])
/// with the same lock, merge, `state:changed` and icon caching as
/// `rescan_apps`. A failed scan is logged and the known apps are kept.
pub fn scan_at_startup(
    app: &AppHandle,
    store: &Arc<Store>,
    command: &LaunchCommand,
) -> Result<(), String> {
    let Some(reason) = startup_scan_reason(command, store.snapshot().apps.len()) else {
        return Ok(());
    };
    log::info!("startup app scan: {reason}");
    let app = app.clone();
    let store = Arc::clone(store);
    spawn_logged("startup-scan", move || {
        scan_and_apply(&app, &store).map(|_| ())
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::{scan_lock, startup_scan_reason};
    use crate::cli::LaunchCommand;

    #[test]
    fn every_normal_start_scans_once() {
        let starts = [
            LaunchCommand::None,
            LaunchCommand::Background,
            LaunchCommand::Launchpad,
            LaunchCommand::Manager,
            LaunchCommand::Flyout {
                group_id: "g1".into(),
            },
        ];
        for command in &starts {
            assert!(startup_scan_reason(command, 12).is_some(), "{command:?}");
            assert!(startup_scan_reason(command, 0).is_some(), "{command:?}");
        }
        assert_ne!(
            startup_scan_reason(&LaunchCommand::None, 0),
            startup_scan_reason(&LaunchCommand::None, 12)
        );
    }

    #[test]
    fn cleanup_never_scans() {
        for delete_data in [false, true] {
            let command = LaunchCommand::Cleanup { delete_data };
            assert_eq!(startup_scan_reason(&command, 0), None);
            assert_eq!(startup_scan_reason(&command, 12), None);
        }
    }

    #[test]
    fn the_flyout_bench_never_scans() {
        assert_eq!(startup_scan_reason(&LaunchCommand::BenchFlyout, 0), None);
        assert_eq!(startup_scan_reason(&LaunchCommand::BenchFlyout, 12), None);
    }

    #[test]
    fn a_held_scan_lock_blocks_other_scans() {
        let held = scan_lock().lock().unwrap();
        assert!(scan_lock().try_lock().is_err());
        drop(held);
        assert!(scan_lock().try_lock().is_ok());
    }
}
