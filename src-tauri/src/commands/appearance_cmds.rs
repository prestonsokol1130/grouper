//! Frontend commands for the accent colour and window materials.

use tauri::{State, Window};

use crate::appearance::accent::{AccentColors, AccentState};
use crate::appearance::material::{Material, MaterialState};

/// The Windows accent colour, or `null` when it could not be read.
// The frontend then uses DESIGN.md's fallback blue. Changes arrive as
// `accent:changed` events.
#[tauri::command]
#[specta::specta]
pub fn get_accent_color(state: State<'_, AccentState>) -> Option<AccentColors> {
    state.current()
}

/// The material the calling window is drawn on.
// `solid` means the page must paint its own background.
#[tauri::command]
#[specta::specta]
pub fn get_window_material(window: Window, state: State<'_, MaterialState>) -> Material {
    state.get(window.label())
}
