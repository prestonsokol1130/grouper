//! Frontend commands that make and edit Launchpad folders by drag and drop.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::state_cmds::{mutate, remove_group_files};
use crate::launchpad::layout::PER_PAGE;
use crate::state::ids::new_group_id;
use crate::state::{LaunchpadItem, LaunchpadSlot, Store};

/// Drops app `dragged_app_id` onto an app (new folder) or a folder; returns its group id.
// One state change (see `AppState::merge_on_launchpad`). A folder whose
// icon now shows other apps gets its icon version bumped by the store,
// which tells the shortcut worker to rebuild a taskbar icon it has.
#[tauri::command]
#[specta::specta]
pub fn apply_launchpad_merge(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    dragged_app_id: String,
    target: LaunchpadItem,
) -> Result<String, String> {
    mutate(&app, &store, |state| {
        state.merge_on_launchpad(&dragged_app_id, &target, &new_group_id)
    })
}

/// Moves app `app_id` out of folder `group_id` to stored slot `to`.
// A folder left with one app (or none) is deleted like `delete_group`
// does: `AppState::remove_group` in the same change, then its shortcut
// and `.ico` files are removed in the background.
#[tauri::command]
#[specta::specta]
pub fn remove_app_from_folder(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    group_id: String,
    app_id: String,
    to: LaunchpadSlot,
) -> Result<(), String> {
    let deleted = mutate(&app, &store, |state| {
        state.take_out_of_folder(&group_id, &app_id, to, PER_PAGE)
    })?;
    if deleted {
        remove_group_files(&app, group_id);
    }
    Ok(())
}
