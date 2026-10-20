//! The global Launchpad shortcut (`Settings::hotkey`, DESIGN.md section
//! 12), through `tauri-plugin-global-shortcut`.
//!
//! A shortcut another app already holds is not fatal: Grouper logs it,
//! keeps running without one and emits [`HOTKEY_FAILED_EVENT`]. The same
//! event is sent once more to the next Manager or Launchpad window that
//! opens, since at startup no page may be listening yet.

use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, EventTarget, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::launchpad::{window, Opener};

/// Emitted when the shortcut could not be registered.
pub const HOTKEY_FAILED_EVENT: &str = "hotkey:failed";

/// The [`HOTKEY_FAILED_EVENT`] payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyFailed {
    /// The shortcut as stored, for example `Alt+\``.
    pub hotkey: String,
    /// The plugin's error, for the log.
    pub error: String,
}

/// The registered shortcut and a failure no window has been shown yet.
#[derive(Debug, Default)]
pub struct HotkeyState {
    registered: Mutex<Option<String>>,
    pending: Mutex<Option<HotkeyFailed>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

/// The plugin, with a handler that toggles the Launchpad on key down.
/// Grouper registers only one shortcut, so every press is the Launchpad's.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state == ShortcutState::Pressed {
                log::info!("shortcut {shortcut} pressed");
                window::toggle(app, Opener::Shortcut);
            }
        })
        .build()
}

/// Checks that `hotkey` is a shortcut the plugin understands.
pub fn parse(hotkey: &str) -> Result<Shortcut, String> {
    hotkey
        .parse::<Shortcut>()
        .map_err(|e| format!("{hotkey:?} is not a valid shortcut: {e}"))
}

fn register<R: Runtime>(app: &AppHandle<R>, hotkey: &str) -> Result<(), String> {
    let shortcut = parse(hotkey)?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("could not register {hotkey:?}: {e}"))
}

/// Registers the saved shortcut at startup. On failure (usually another
/// app holds it) the error is logged and [`HOTKEY_FAILED_EVENT`] emitted.
pub fn register_at_startup<R: Runtime>(app: &AppHandle<R>, hotkey: &str) {
    app.manage(HotkeyState::default());
    let Some(state) = app.try_state::<HotkeyState>() else {
        return;
    };
    match register(app, hotkey) {
        Ok(()) => {
            log::info!("Launchpad shortcut {hotkey:?} registered");
            *lock(&state.registered) = Some(hotkey.to_owned());
        }
        Err(error) => {
            log::warn!("Launchpad shortcut unavailable: {error}");
            let failed = HotkeyFailed {
                hotkey: hotkey.to_owned(),
                error,
            };
            if let Err(e) = app.emit(HOTKEY_FAILED_EVENT, &failed) {
                log::warn!("could not emit {HOTKEY_FAILED_EVENT}: {e}");
            }
            crate::tray::notify_hotkey_failed(app, hotkey);
            *lock(&state.pending) = Some(failed);
        }
    }
}

/// Sends a startup failure no window has shown yet to window `label`,
/// once. Call right after showing the Manager or the Launchpad.
pub fn notify_pending<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let Some(state) = app.try_state::<HotkeyState>() else {
        return;
    };
    let Some(failed) = lock(&state.pending).take() else {
        return;
    };
    let target = EventTarget::webview_window(label);
    if let Err(error) = app.emit_to(target, HOTKEY_FAILED_EVENT, &failed) {
        log::warn!("could not emit {HOTKEY_FAILED_EVENT}: {error}");
    }
}

/// Swaps the registered shortcut for `hotkey` (it must pass
/// [`crate::hotkey_rules::check`]): the old one is
/// unregistered and `hotkey` registered. If that fails the old one is
/// registered again and the error returned. The caller saves `hotkey`
/// only on `Ok`.
pub fn replace<R: Runtime>(app: &AppHandle<R>, hotkey: &str) -> Result<(), String> {
    crate::hotkey_rules::check(&parse(hotkey)?)?;
    let state = app
        .try_state::<HotkeyState>()
        .ok_or("the shortcut is not set up yet")?;
    let old = lock(&state.registered).clone();
    if old.as_deref() == Some(hotkey) {
        return Ok(());
    }
    if let Some(old) = &old {
        if let Err(error) = app.global_shortcut().unregister(old.as_str()) {
            log::warn!("could not unregister {old:?}: {error}");
        }
    }
    match register(app, hotkey) {
        Ok(()) => {
            log::info!("Launchpad shortcut changed to {hotkey:?}");
            *lock(&state.registered) = Some(hotkey.to_owned());
            *lock(&state.pending) = None;
            Ok(())
        }
        Err(error) => {
            let restored = old.as_deref().map(|old| register(app, old));
            if let Some(Err(again)) = &restored {
                log::warn!("could not restore the old shortcut: {again}");
                *lock(&state.registered) = None;
            }
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn the_default_and_common_shortcuts_parse() {
        for hotkey in ["Alt+`", "Ctrl+Alt+Space", "Shift+Alt+L", "Super+F12"] {
            assert!(parse(hotkey).is_ok(), "{hotkey}");
        }
    }

    #[test]
    fn nonsense_is_refused_with_its_name() {
        let error = parse("Alt+Nope").unwrap_err();
        assert!(error.contains("Alt+Nope"), "{error}");
        assert!(parse("").is_err());
    }
}
