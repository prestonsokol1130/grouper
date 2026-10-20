//! Moving and sizing a window in one step.

use std::ffi::c_void;

use crate::taskbar::placement::Rect;

/// Gives window `hwnd` the outer rectangle `rect` (physical pixels) with
/// one `SetWindowPos`, keeping its z-order and activation. Does nothing
/// when it is already there. Returns whether it moved. Only for a move
/// that stays on monitors of the same DPI: crossing onto another DPI
/// makes Windows rescale the window afterwards.
pub fn set_window_rect(hwnd: *mut c_void, rect: Rect) -> Result<bool, String> {
    imp::set_window_rect(hwnd, rect)
}

/// The rectangle a window at `(x, y)` sized `width` x `height` covers.
pub fn rect_at(x: i32, y: i32, width: u32, height: u32) -> Rect {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    let height = i32::try_from(height).unwrap_or(i32::MAX);
    Rect {
        left: x,
        top: y,
        right: x.saturating_add(width),
        bottom: y.saturating_add(height),
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER,
    };

    use crate::taskbar::placement::Rect;

    pub(super) fn set_window_rect(hwnd: *mut c_void, rect: Rect) -> Result<bool, String> {
        let hwnd = HWND(hwnd);
        let mut now = RECT::default();
        // SAFETY: hwnd is a live window of this process; `now` is a
        // writable RECT for the call.
        if unsafe { GetWindowRect(hwnd, &mut now) }.is_ok()
            && (now.left, now.top, now.right, now.bottom)
                == (rect.left, rect.top, rect.right, rect.bottom)
        {
            return Ok(false);
        }
        // SAFETY: hwnd is a live window of this process, called on the
        // thread that owns it; the flags leave z-order and focus alone.
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
            )
        }
        .map_err(|e| format!("SetWindowPos: {e}"))?;
        Ok(true)
    }
}

#[cfg(not(windows))]
mod imp {
    use std::ffi::c_void;

    use crate::taskbar::placement::Rect;

    pub(super) fn set_window_rect(_hwnd: *mut c_void, _rect: Rect) -> Result<bool, String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::rect_at;
    use crate::taskbar::placement::Rect;

    #[test]
    fn a_rect_from_its_corner_and_size() {
        let rect = rect_at(-1920, 40, 300, 200);
        let want = Rect {
            left: -1920,
            top: 40,
            right: -1620,
            bottom: 240,
        };
        assert_eq!(rect, want);
        assert_eq!(rect_at(i32::MAX - 1, 0, 10, 10).right, i32::MAX);
    }
}
