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

/// The last `flyout:show` payload (`null` before any), for a late page.
#[tauri::command]
#[specta::specta]
pub fn get_flyout(state: State<'_, FlyoutState>) -> Option<FlyoutShow> {
    state.last()
}

/// Launches app `app_id`, then hides the flyout; on failure it stays open.
#[tauri::command]
#[specta::specta]
pub async fn launch_from_flyout(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    app_id: String,
) -> Result<(), String> {
    launch_app(app.clone(), store, app_id).await?;
    window::hide(&app);
    Ok(())
}
