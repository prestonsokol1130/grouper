//! Placing the flyout window and giving it keyboard focus.

use tauri::{PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

use super::window::raw_hwnd;
use crate::win32::flyout_window::bring_to_foreground;
use crate::win32::window_rect::{rect_at, set_window_rect};

/// Moves keyboard focus to the flyout so a click elsewhere blurs it.
/// `Err` only when both `SetForegroundWindow` and `set_focus` failed.
pub(super) fn focus<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
    // Allowed by the AllowSetForegroundWindow every Grouper launch makes.
    let foreground = raw_hwnd(window).and_then(bring_to_foreground);
    if let Err(error) = &foreground {
        log::warn!("flyout foreground: {error}");
    }
    let set_focus = window.set_focus().map_err(|e| e.to_string());
    if let Err(error) = &set_focus {
        log::warn!("flyout set_focus: {error}");
    }
    either_focused(foreground, set_focus)
}

/// Focus moved if either attempt succeeded; otherwise both errors.
fn either_focused(
    foreground: Result<(), String>,
    set_focus: Result<(), String>,
) -> Result<(), String> {
    match (foreground, set_focus) {
        (Err(fg), Err(sf)) => Err(format!("focus failed: foreground: {fg}; set_focus: {sf}")),
        _ => Ok(()),
    }
}

/// Moves and sizes the (hidden) flyout. When the window already has the
/// target monitor's DPI `scale`, one `SetWindowPos` does both, and none
/// when it is already there. Moving onto a monitor with another DPI
/// rescales the window, so then the size is set after the move and the
/// position once more after that.
pub(super) fn place<R: Runtime>(
    window: &WebviewWindow<R>,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    scale: f64,
) -> Result<(), String> {
    if window.scale_factor().ok() == Some(scale) {
        let rect = rect_at(position.x, position.y, size.width, size.height);
        match raw_hwnd(window).and_then(|hwnd| set_window_rect(hwnd, rect)) {
            Ok(_) => return Ok(()),
            Err(error) => log::debug!("flyout one-step placement: {error}"),
        }
    }
    window
        .set_position(position)
        .map_err(|e| format!("set_position: {e}"))?;
    window
        .set_size(size)
        .map_err(|e| format!("set_size: {e}"))?;
    window
        .set_position(position)
        .map_err(|e| format!("set_position: {e}"))
}

#[cfg(test)]
mod tests {
    use super::either_focused;

    fn err() -> Result<(), String> {
        Err("no".to_owned())
    }

    #[test]
    fn focus_succeeds_if_either_attempt_does() {
        assert_eq!(either_focused(Ok(()), Ok(())), Ok(()));
        assert_eq!(either_focused(err(), Ok(())), Ok(()));
        assert_eq!(either_focused(Ok(()), err()), Ok(()));
    }

    #[test]
    fn focus_fails_only_if_both_do() {
        let error = either_focused(Err("fg".into()), Err("sf".into())).unwrap_err();
        assert!(error.contains("fg") && error.contains("sf"), "{error}");
    }
}
