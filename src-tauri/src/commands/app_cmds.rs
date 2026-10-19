//! Frontend commands that add apps by hand and remove apps.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use super::icon_cmds::queue_icons;
use super::state_cmds::emit_state_changed;
use crate::apps::add::{add_paths, PROGRAM_EXTENSIONS};
use crate::apps::remove::remove_or_hide;
use crate::state::{AppEntry, Store};

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

/// Opens the native file dialog (Programs: `.exe`, `.lnk`, several at
/// once) and adds the picked programs. Returns their entries; an empty list
/// when the dialog was cancelled.
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
        let picked = app
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter("Programs", PROGRAM_EXTENSIONS)
            .blocking_pick_files()
            .unwrap_or_default();
        let paths: Vec<PathBuf> = picked
            .into_iter()
            .filter_map(|file| match file.into_path() {
                Ok(path) => Some(path),
                Err(error) => {
                    log::warn!("skipping a picked file that is not a path: {error}");
                    None
                }
            })
            .collect();
        add_and_publish(&app, &store, &paths)
    })
    .await
    .map_err(|e| format!("adding apps did not finish: {e}"))?
}

/// Adds the program at `path` (an `.exe` or `.lnk`) and returns its entry,
/// or the existing entry when that app is already in the list.
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

/// Removes app `app_id`: a Manual app is deleted from the list, its groups
/// and the Launchpad; a Start Menu app is hidden instead.
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
