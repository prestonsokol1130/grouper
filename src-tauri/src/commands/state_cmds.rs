//! Frontend commands that read and change the app state.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::shortcut_worker::{queue, Job};
use crate::state::ids::new_group_id;
use crate::state::{AppState, Group, LaunchpadLayout, Settings, SettingsPatch, Store};
use crate::taskbar::shortcut_sync::Change;

/// Emitted to every window with the full new [`AppState`] after a change.
pub const STATE_CHANGED_EVENT: &str = "state:changed";

/// Sends the new state to all windows. A failed emit is logged, not fatal:
/// the change is already applied and will be saved.
pub fn emit_state_changed(app: &AppHandle, state: &AppState) {
    if let Err(error) = app.emit(STATE_CHANGED_EVENT, state) {
        log::warn!("could not emit {STATE_CHANGED_EVENT}: {error}");
    }
}

/// Applies `change` to the store, then emits `state:changed`.
fn mutate<T>(
    app: &AppHandle,
    store: &Store,
    change: impl FnOnce(&mut AppState) -> Result<T, String>,
) -> Result<T, String> {
    // Emitting inside the store's publish section keeps every window's
    // state:changed events in the same order the changes were committed.
    store.update_and_publish(change, |state| emit_state_changed(app, state))
}

/// Trims a user-entered name and rejects an empty one.
fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("Group name cannot be empty".into())
    } else {
        Ok(name.to_string())
    }
}

/// The group with `id`, or an error naming it.
fn group_mut<'a>(state: &'a mut AppState, id: &str) -> Result<&'a mut Group, String> {
    state
        .group_by_id_mut(id)
        .ok_or_else(|| format!("Group not found: {id}"))
}

/// Returns the whole current state.
#[tauri::command]
#[specta::specta]
pub fn get_state(store: State<'_, Arc<Store>>) -> Result<AppState, String> {
    Ok(store.snapshot())
}

/// Creates an empty group called `name`.
#[tauri::command]
#[specta::specta]
pub fn create_group(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    name: String,
) -> Result<Group, String> {
    let name = clean_name(&name)?;
    mutate(&app, &store, |state| {
        let taken: Vec<&str> = state.groups.iter().map(|g| g.id.as_str()).collect();
        let group = Group {
            id: new_group_id(&taken)?,
            name,
            app_ids: Vec::new(),
            icon_version: 0,
            has_shortcut: false,
            shortcut_path: None,
        };
        state.groups.push(group.clone());
        Ok(group)
    })
}

/// Renames group `id`.
// When the name changed, a shortcut update is queued; if the group has a
// shortcut when it runs, the shortcut is renamed in the background.
#[tauri::command]
#[specta::specta]
pub fn rename_group(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: String,
    name: String,
) -> Result<Group, String> {
    let name = clean_name(&name)?;
    let (group, renamed) = mutate(&app, &store, |state| {
        let group = group_mut(state, &id)?;
        let renamed = group.name != name;
        group.name = name;
        Ok((group.clone(), renamed))
    })?;
    // Queued even before the group has a shortcut: a shortcut still being
    // created gets the new name once it is recorded.
    if renamed {
        let change = Change {
            renamed: true,
            icon: false,
        };
        queue(
            &app,
            Job::Update {
                group_id: id,
                change,
            },
        );
    }
    Ok(group)
}

/// Deletes group `id`; its Launchpad folder unfolds into its apps.
// Its shortcut and cached `.ico` files are removed afterwards in the
// background; a failure there is logged and never undoes the deletion.
#[tauri::command]
#[specta::specta]
pub fn delete_group(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: String,
) -> Result<(), String> {
    mutate(&app, &store, |state| {
        state
            .remove_group(&id)
            .map(|_| ())
            .ok_or_else(|| format!("Group not found: {id}"))
    })?;
    queue(&app, Job::Remove { group_id: id });
    Ok(())
}

/// Replaces the apps in group `id` (unknown and repeated ids are dropped).
// When its first four visible apps change, its icon version is bumped
// and, if it has a shortcut, its `.ico` and shortcut are rebuilt in the
// background.
#[tauri::command]
#[specta::specta]
pub fn set_group_apps(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: String,
    app_ids: Vec<String>,
) -> Result<Group, String> {
    // A bumped icon version reaches the shortcut worker through the
    // store's icon listener (see `queue_icon_updates`).
    mutate(&app, &store, |state| {
        state
            .set_group_apps(&id, app_ids)
            .ok_or_else(|| format!("Group not found: {id}"))
    })
}

/// Puts the groups in the order of `ids` (every group id exactly once).
#[tauri::command]
#[specta::specta]
pub fn reorder_groups(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    ids: Vec<String>,
) -> Result<(), String> {
    mutate(&app, &store, |state| state.reorder_groups(&ids))
}

/// Changes only the settings present in `patch`; the rest are kept.
// The shortcut is changed with `set_hotkey`, which registers it first.
#[tauri::command]
#[specta::specta]
pub fn update_settings(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let settings = mutate(&app, &store, |state| {
        state.settings.apply(patch);
        Ok(state.settings.clone())
    })?;
    crate::hot_corners::update(&app, settings.hot_corners);
    Ok(settings)
}

/// Replaces the Launchpad layout (dangling ids are dropped).
#[tauri::command]
#[specta::specta]
pub fn set_launchpad_layout(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    layout: LaunchpadLayout,
) -> Result<LaunchpadLayout, String> {
    mutate(&app, &store, |state| {
        state.launchpad = layout;
        state.validate();
        Ok(state.launchpad.clone())
    })
}
