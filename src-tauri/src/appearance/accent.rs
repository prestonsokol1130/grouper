//! The Windows accent colour, read at startup and kept current.
//!
//! Fluent uses a darker shade of the accent on light backgrounds and a
//! lighter one on dark backgrounds, so both are sent. The frontend falls
//! back to DESIGN.md's default blue when the colour cannot be read.

use std::sync::Mutex;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::win32::accent::{watch_accent, AccentWatcher, Rgb};

/// Emitted to every window with the new [`AccentColors`] when the Windows
/// accent changes.
pub const ACCENT_CHANGED_EVENT: &str = "accent:changed";

/// The Windows accent as `#RRGGBB` strings, one per theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccentColors {
    /// The accent on light backgrounds (`UIColorType::AccentDark1`).
    pub light: String,
    /// The accent on dark backgrounds (`UIColorType::AccentLight2`).
    pub dark: String,
}

impl AccentColors {
    fn from_rgb(light: Rgb, dark: Rgb) -> Self {
        Self {
            light: to_hex(light),
            dark: to_hex(dark),
        }
    }
}

/// `#RRGGBB` for `rgb`.
pub fn to_hex(rgb: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2)
}

/// The latest accent read, plus the watcher that keeps it current.
#[derive(Default)]
pub struct AccentState {
    current: Mutex<Option<AccentColors>>,
    watcher: Mutex<Option<AccentWatcher>>,
}

impl AccentState {
    /// The last accent read, or `None` when it could not be read.
    pub fn current(&self) -> Option<AccentColors> {
        self.current
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn set(&self, colors: Option<AccentColors>) {
        *self.current.lock().unwrap_or_else(|p| p.into_inner()) = colors;
    }
}

/// Reads the accent and starts watching it. Call once at startup, on the
/// main thread. A failure is logged and leaves the frontend on its
/// fallback accent.
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    app.manage(AccentState::default());
    let emitter = app.clone();
    let watched = watch_accent(move |read| {
        let colors = match read {
            Ok((light, dark)) => Some(AccentColors::from_rgb(light, dark)),
            Err(error) => {
                log::warn!("could not read the changed accent colour: {error}");
                return;
            }
        };
        if let Some(state) = emitter.try_state::<AccentState>() {
            state.set(colors.clone());
        }
        if let Err(error) = emitter.emit(ACCENT_CHANGED_EVENT, colors) {
            log::warn!("could not emit {ACCENT_CHANGED_EVENT}: {error}");
        }
    });
    let Some(state) = app.try_state::<AccentState>() else {
        return;
    };
    match watched {
        Ok(((light, dark), watcher)) => {
            let colors = AccentColors::from_rgb(light, dark);
            log::info!("accent colour {} / {}", colors.light, colors.dark);
            state.set(Some(colors));
            *state.watcher.lock().unwrap_or_else(|p| p.into_inner()) = Some(watcher);
        }
        Err(error) => log::warn!("could not read the accent colour: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{to_hex, AccentColors};
    use crate::win32::accent::Rgb;

    #[test]
    fn hex_is_upper_case_and_padded() {
        assert_eq!(to_hex(Rgb(0, 103, 192)), "#0067C0");
        assert_eq!(to_hex(Rgb(1, 2, 3)), "#010203");
    }

    #[test]
    fn serializes_camel_case() {
        let colors = AccentColors::from_rgb(Rgb(0, 0, 0), Rgb(255, 255, 255));
        let json = serde_json::to_string(&colors).unwrap();
        assert_eq!(json, r##"{"light":"#000000","dark":"#FFFFFF"}"##);
    }
}
