//! Frontend commands that add apps by hand and remove apps.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State, WebviewWindow};

use super::icon_cmds::queue_icons;
use super::state_cmds::emit_state_changed;
use crate::apps::add::add_paths;
use crate::apps::remove::remove_or_hide;
use crate::state::{AppEntry, Store};
use crate::win32::file_dialog::pick_programs;

/// Adds `paths`, emits `state:changed` and queues the icons of the added
/// apps. Blocks.
fn add_and_publish(
    app: &AppHandle,
    store: &Store,
    paths: &[PathBuf],
) -> Result<Vec<AppEntry>, String> {
    let added = add_paths(store, paths, |state| emit_state_changed(app, state))?;
    if !added.is_empty() {
        queue_icons(app, added.clone());
    }
    Ok(added)
}

/// Opens the file dialog over the calling window and adds the picked programs.
// Programs means `.exe` and `.lnk`, several at once. Returns their
// entries; an empty list, with nothing changed, when it was cancelled.
#[tauri::command]
#[specta::specta]
pub async fn pick_and_add_apps(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Arc<Store>>,
) -> Result<Vec<AppEntry>, String> {
    let store = Arc::clone(&store);
    // The dialog and the shortcut reads block; keep them off the async
    // runtime's worker threads.
    tauri::async_runtime::spawn_blocking(move || {
        let paths = pick_programs(owner_handle(&window))?;
        add_and_publish(&app, &store, &paths)
    })
    .await
    .map_err(|e| format!("adding apps did not finish: {e}"))?
}

/// The `HWND` of `window` as a number, for parenting the file dialog. A
/// window without one gets an unowned dialog.
#[cfg(windows)]
fn owner_handle(window: &WebviewWindow) -> Option<isize> {
    match window.hwnd() {
        Ok(hwnd) => Some(hwnd.0 as isize),
        Err(error) => {
            log::warn!("no window handle for the file dialog: {error}");
            None
        }
    }
}

#[cfg(not(windows))]
fn owner_handle(_window: &WebviewWindow) -> Option<isize> {
    None
}

/// Adds the program at `path` (an `.exe` or `.lnk`) and returns its entry.
// When that app is already in the list, its existing entry is returned.
#[tauri::command]
#[specta::specta]
pub async fn add_app_from_path(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    path: String,
) -> Result<AppEntry, String> {
    let store = Arc::clone(&store);
    tauri::async_runtime::spawn_blocking(move || {
        add_and_publish(&app, &store, &[PathBuf::from(&path)])?
            .into_iter()
            .next()
            .ok_or_else(|| format!("Could not add {path}"))
    })
    .await
    .map_err(|e| format!("adding an app did not finish: {e}"))?
}

/// Removes app `app_id`: a Manual app is deleted, a Start Menu app hidden.
// A deleted app also leaves its groups and the Launchpad.
#[tauri::command]
#[specta::specta]
pub fn remove_app(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    app_id: String,
) -> Result<(), String> {
    store.update_and_publish(
        |state| remove_or_hide(state, &app_id).map(|_| ()),
        |state| emit_state_changed(&app, state),
    )
}
