//! Frontend commands behind the Manager's Settings page.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State};

use super::state_cmds::{emit_state_changed, mutate};
use crate::appearance::material;
use crate::autostart::{self, PluginLauncher};
use crate::state::{Settings, SettingsPatch, Store};
use crate::win32::launch::open_with_shell;

/// Grouper's source code, opened from Settings > About.
pub const REPO_URL: &str = "https://github.com/prestonsokol1130/grouper";

// What `open_place` opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    Logs,
    Data,
    Repo,
}

/// Changes only the settings present in `patch`; the rest are kept.
// The shortcut and autostart have their own commands. Hot corners, the
// window background and the folder shape take effect at once: a new shape
// bumps every group's icon version, so the shortcut worker rebuilds each
// taskbar `.ico`.
#[tauri::command]
#[specta::specta]
pub fn update_settings(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let (settings, background_changed) = mutate(&app, &store, |state| {
        let before = state.settings.clone();
        state.settings.apply(patch);
        if state.settings.folder_shape != before.folder_shape {
            for group in &mut state.groups {
                group.icon_version = group.icon_version.wrapping_add(1);
            }
        }
        let background_changed = state.settings.background != before.background;
        Ok((state.settings.clone(), background_changed))
    })?;
    crate::hot_corners::update(&app, settings.hot_corners);
    if background_changed {
        material::reapply(&app, settings.background);
    }
    Ok(settings)
}

/// Turns "Start with Windows" on or off; saves what Windows then reports.
#[tauri::command]
#[specta::specta]
pub fn set_autostart(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    enabled: bool,
) -> Result<Settings, String> {
    let enabled = autostart::set(&PluginLauncher(&app), enabled)?;
    store.update_and_publish(
        |state| {
            state.settings.autostart = enabled;
            Ok(state.settings.clone())
        },
        |state| emit_state_changed(&app, state),
    )
}

/// Opens the logs folder, the data folder or the repo page.
#[tauri::command]
#[specta::specta]
pub async fn open_place(app: AppHandle, place: Place) -> Result<(), String> {
    let target = match place {
        Place::Logs => folder(app.path().app_log_dir())?,
        Place::Data => folder(app.path().app_data_dir())?,
        Place::Repo => REPO_URL.to_owned(),
    };
    // The Shell call blocks briefly; keep it off the async workers.
    tauri::async_runtime::spawn_blocking(move || open_with_shell(&target))
        .await
        .map_err(|e| format!("open did not finish: {e}"))?
}

/// The folder as a string, if it resolved and exists.
fn folder(dir: tauri::Result<PathBuf>) -> Result<String, String> {
    let dir = dir.map_err(|e| format!("resolve folder: {e}"))?;
    if !dir.is_dir() {
        return Err(format!("{} does not exist", dir.display()));
    }
    Ok(dir.display().to_string())
}
