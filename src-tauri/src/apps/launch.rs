//! Launching one app or a whole group, with missing apps flagged.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::state::{AppState, Store};
use crate::win32::launch::{launch_target, not_found};

/// Starts one app; normally [`crate::win32::launch::launch`]. Tests pass a
/// fake one.
pub type Launcher<'a> = &'a dyn Fn(&crate::state::AppEntry) -> Result<(), String>;

/// The pause between two apps of a group, so they open in order.
pub const GROUP_LAUNCH_GAP: Duration = Duration::from_millis(150);

/// What a group launch did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSummary {
    /// How many apps were started (u32: the bindings refuse 64-bit ints).
    pub launched: u32,
    /// Names of the apps that could not be started, in group order.
    pub failed: Vec<String>,
}

/// Launches app `app_id` with `launcher`.
///
/// When neither the app's `.lnk` nor its target exists, the app is marked
/// `missing` (the new state is passed to `publish`) and
/// `Err("App not found: <name>")` is returned without calling `launcher`.
/// When one exists but the app is marked missing, the mark is cleared and
/// published before `launcher` runs, whatever the launcher then returns: a
/// Shell failure is not a missing file.
pub fn launch_app_with(
    store: &Store,
    app_id: &str,
    launcher: Launcher<'_>,
    publish: &dyn Fn(&AppState),
) -> Result<(), String> {
    let entry = store
        .snapshot()
        .app_by_id(app_id)
        .cloned()
        .ok_or_else(|| format!("Unknown app: {app_id}"))?;
    let found = launch_target(&entry, std::path::Path::exists).is_some();
    // Publish only when the flag actually changes.
    if entry.missing != !found {
        set_missing(store, app_id, !found, publish);
    }
    if !found {
        return Err(not_found(&entry.name));
    }
    launcher(&entry)
}

/// Sets `missing` on app `app_id` to `missing` and publishes the new state.
/// A store that refuses changes only gets a log line: the launch result
/// says enough.
fn set_missing(store: &Store, app_id: &str, missing: bool, publish: &dyn Fn(&AppState)) {
    let changed = store.update_and_publish(
        |state| {
            if let Some(app) = state.apps.iter_mut().find(|app| app.id == app_id) {
                app.missing = missing;
            }
            Ok(())
        },
        publish,
    );
    if let Err(error) = changed {
        log::warn!("could not set app {app_id} missing={missing}: {error}");
    }
}

/// Launches every app of group `group_id` in order, waiting `gap` between
/// two apps. A failed app is recorded in the summary and the rest still
/// launch. `Err` only when the group does not exist.
pub fn launch_group_with(
    store: &Store,
    group_id: &str,
    launcher: Launcher<'_>,
    publish: &dyn Fn(&AppState),
    gap: Duration,
) -> Result<LaunchSummary, String> {
    let state = store.snapshot();
    let group = state
        .group_by_id(group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    let mut summary = LaunchSummary::default();
    for (at, app_id) in group.app_ids.iter().enumerate() {
        if at > 0 && !gap.is_zero() {
            std::thread::sleep(gap);
        }
        match launch_app_with(store, app_id, launcher, publish) {
            Ok(()) => summary.launched = summary.launched.saturating_add(1),
            Err(error) => {
                log::warn!("group {group_id}: could not launch app {app_id}: {error}");
                let name = state
                    .app_by_id(app_id)
                    .map_or(app_id.as_str(), |app| &app.name);
                summary.failed.push(name.to_owned());
            }
        }
    }
    Ok(summary)
}
