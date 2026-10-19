//! Frontend commands that launch an app or a whole group.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::state_cmds::emit_state_changed;
use crate::apps::launch::{launch_app_with, launch_group_with, LaunchSummary, GROUP_LAUNCH_GAP};
use crate::state::Store;
use crate::win32::launch::launch;

/// Launches app `app_id` through its shortcut (or its target when the
/// shortcut is gone). An app with neither on disk is marked missing and
/// the error is `App not found: <name>`.
#[tauri::command]
#[specta::specta]
pub async fn launch_app(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    app_id: String,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    // The Shell call blocks until the app has started; keep it off the
    // async runtime's worker threads.
    tauri::async_runtime::spawn_blocking(move || {
        launch_app_with(&store, &app_id, &launch, &|state| {
            emit_state_changed(&app, state);
        })
    })
    .await
    .map_err(|e| format!("app launch did not finish: {e}"))?
}

/// Launches every app of group `group_id` in order, 150 ms apart, and
/// reports how many started and which (by name) could not.
#[tauri::command]
#[specta::specta]
pub async fn launch_group(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    group_id: String,
) -> Result<LaunchSummary, String> {
    let store = Arc::clone(&store);
    tauri::async_runtime::spawn_blocking(move || {
        let publish = |state: &crate::state::AppState| emit_state_changed(&app, state);
        launch_group_with(&store, &group_id, &launch, &publish, GROUP_LAUNCH_GAP)
    })
    .await
    .map_err(|e| format!("group launch did not finish: {e}"))?
}
