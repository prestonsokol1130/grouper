//! Frontend commands for the Launchpad window and its shortcut.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::state_cmds::emit_state_changed;
use crate::hotkey;
use crate::launchpad::window;
use crate::state::{Settings, Store};

/// Hides the Launchpad; `Err` when Windows could not hide it.
#[tauri::command]
#[specta::specta]
pub fn hide_launchpad(app: AppHandle) -> Result<(), String> {
    window::hide(&app)
}

/// Registers shortcut `hotkey` in place of the old one; saved only if it works.
#[tauri::command]
#[specta::specta]
pub fn set_hotkey(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    hotkey: String,
) -> Result<Settings, String> {
    let hotkey = hotkey.trim().to_owned();
    hotkey::replace(&app, &hotkey)?;
    store.update_and_publish(
        |state| {
            state.settings.hotkey = hotkey;
            Ok(state.settings.clone())
        },
        |state| emit_state_changed(&app, state),
    )
}
