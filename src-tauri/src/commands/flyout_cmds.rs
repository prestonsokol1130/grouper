//! Frontend commands for the group flyout window.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::launch_cmds::launch_app;
use crate::flyout::{window, FlyoutShow, FlyoutState};
use crate::state::Store;

/// Hides the flyout (Escape in the flyout window).
#[tauri::command]
#[specta::specta]
pub fn hide_flyout(app: AppHandle) {
    window::hide(&app);
}

/// What the flyout shows now: the last `flyout:show` payload, or `null`
/// before any. Lets a flyout page that loaded late catch up.
#[tauri::command]
#[specta::specta]
pub fn get_flyout(state: State<'_, FlyoutState>) -> Option<FlyoutShow> {
    state.last()
}

/// Hides the flyout, then launches app `app_id` like `launch_app`.
#[tauri::command]
#[specta::specta]
pub async fn launch_from_flyout(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    app_id: String,
) -> Result<(), String> {
    // Hidden first so the launched app's window comes up over a clear
    // screen and the flyout does not linger.
    window::hide(&app);
    launch_app(app, store, app_id).await
}
