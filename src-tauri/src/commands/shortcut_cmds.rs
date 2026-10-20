//! Frontend commands that create a group's Start Menu shortcut and show it
//! in Explorer.
//!
//! Windows does not let apps pin to the taskbar; the user pins the
//! shortcut themselves (right-click, Pin to taskbar).

use std::sync::Arc;

use tauri::{AppHandle, Manager};

use super::shortcut_worker::create_and_wait;
use crate::state::Store;
use crate::taskbar::shortcut::shortcut_dir;
use crate::taskbar::shortcut_write::find_group_shortcut;
use crate::win32::reveal::reveal_in_explorer;

/// The loaded store, or an error when setup has not managed one.
fn store(app: &AppHandle) -> Result<Arc<Store>, String> {
    app.try_state::<Arc<Store>>()
        .map(|store| Arc::clone(&store))
        .ok_or_else(|| "the app state is not loaded".to_owned())
}

/// Builds group `group_id`'s icon, then creates (or rewrites) its Start
/// Menu shortcut, `{FOLDERID_Programs}\Grouper\<group name>.lnk`, pointing
/// at this executable with `--flyout <group_id>`. Marks the group
/// `has_shortcut`, so later renames and icon changes keep the shortcut in
/// step, and returns its path.
#[tauri::command]
#[specta::specta]
pub async fn create_group_shortcut(app: AppHandle, group_id: String) -> Result<String, String> {
    // The icon build and the COM work block; keep them off the async
    // runtime's worker threads.
    let lnk = tauri::async_runtime::spawn_blocking(move || create_and_wait(&app, group_id))
        .await
        .map_err(|e| format!("creating the shortcut did not finish: {e}"))??;
    lnk.into_os_string()
        .into_string()
        .map_err(|lnk| format!("shortcut path {lnk:?} is not valid Unicode"))
}

/// Opens Explorer on Grouper's Start Menu folder with group `group_id`'s
/// shortcut selected. `Err` when the shortcut has not been created yet.
#[tauri::command]
#[specta::specta]
pub async fn reveal_group_shortcut(app: AppHandle, group_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = store(&app)?.snapshot();
        let lnk = find_group_shortcut(&state, &group_id, &shortcut_dir()?)?
            .ok_or_else(|| format!("No shortcut for group {group_id} yet"))?;
        reveal_in_explorer(&lnk)
    })
    .await
    .map_err(|e| format!("showing the shortcut did not finish: {e}"))?
}
