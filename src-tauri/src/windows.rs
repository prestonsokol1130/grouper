//! Show/hide helpers and close-to-hide behaviour for Grouper's windows.

use tauri::{AppHandle, Manager, Runtime, Window, WindowEvent};

/// Shows, unminimizes and focuses the window `label`. Failures are logged.
pub fn show_window<R: Runtime>(app: &AppHandle<R>, label: &str) -> Result<(), String> {
    let result = app
        .get_webview_window(label)
        .ok_or_else(|| format!("window {label:?} not found"))
        .and_then(|window| {
            window.show().map_err(|e| format!("show {label:?}: {e}"))?;
            window
                .unminimize()
                .map_err(|e| format!("unminimize {label:?}: {e}"))?;
            window
                .set_focus()
                .map_err(|e| format!("focus {label:?}: {e}"))
        });
    // A shortcut that failed at startup is shown by the first window opened.
    if result.is_ok() && label == "manager" {
        crate::hotkey::notify_pending(app, label);
    }
    if let Err(error) = &result {
        log::warn!("show_window failed: {error}");
    }
    result
}

/// Hides the window `label`. Failures are logged.
pub fn hide_window<R: Runtime>(app: &AppHandle<R>, label: &str) -> Result<(), String> {
    let result = app
        .get_webview_window(label)
        .ok_or_else(|| format!("window {label:?} not found"))
        .and_then(|window| window.hide().map_err(|e| format!("hide {label:?}: {e}")));
    if let Err(error) = &result {
        log::warn!("hide_window failed: {error}");
    }
    result
}

/// Turns every close request into a hide so windows live as long as the app.
pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        // hide_window already logs a failure; nothing more to do here.
        let _ = hide_window(window.app_handle(), window.label());
    }
}
