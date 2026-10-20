//! Hiding the flyout window. A hide that a caller asked for (Escape, a
//! launch) reports a native failure; the others only log it.

use tauri::{AppHandle, Emitter, EventTarget, Manager, Runtime};

use super::window::flyout_window;
use super::{FlyoutState, FLYOUT_HIDE_EVENT, FLYOUT_LABEL};

/// Hides the flyout on purpose (Escape, a launch). The next request for
/// any group shows it again. `Err` when the native hide failed: the
/// flyout is still showing, so it stays marked as shown.
pub fn hide<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.try_state::<FlyoutState>();
    // Marked hidden first, so the blur the hide causes changes nothing.
    let before = state.as_ref().map(|state| {
        let mut toggle = state.toggle();
        let before = toggle.clone();
        toggle.hidden();
        before
    });
    let result = hide_window(app);
    if let (Err(_), Some(state), Some(before)) = (&result, &state, before) {
        *state.toggle() = before;
    }
    result
}

/// Hides the flyout where no caller can be told (a toggle, a blur, a
/// failed show): a failure is only logged.
pub(super) fn hide_or_log<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = hide_window(app) {
        log::warn!("{error}");
    }
}

/// Hides the window and tells the page, so its next show starts empty.
/// `Err` when the native hide failed; the window is then still showing,
/// so the page is not told to reset.
fn hide_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = flyout_window(app)?;
    hide_result(window.hide().map_err(|e| e.to_string()))?;
    let target = EventTarget::webview_window(FLYOUT_LABEL);
    if let Err(error) = app.emit_to(target, FLYOUT_HIDE_EVENT, ()) {
        log::warn!("could not emit {FLYOUT_HIDE_EVENT}: {error}");
    }
    Ok(())
}

/// The error a failed native hide is reported with.
fn hide_result(native: Result<(), String>) -> Result<(), String> {
    native.map_err(|error| format!("could not hide the flyout: {error}"))
}

#[cfg(test)]
mod tests {
    use super::hide_result;

    #[test]
    fn a_native_hide_failure_is_an_err_naming_it() {
        assert_eq!(hide_result(Ok(())), Ok(()));
        let error = hide_result(Err("ShowWindow failed".into())).unwrap_err();
        assert_eq!(error, "could not hide the flyout: ShowWindow failed");
    }
}
