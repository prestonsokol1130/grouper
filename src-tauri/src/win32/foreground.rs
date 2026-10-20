//! Which window is in front.

use std::ffi::c_void;

/// True when the foreground window is `hwnd` or a window inside it. False
/// when another window, or none, is in front, and always on other
/// platforms.
pub fn is_foreground(hwnd: *mut c_void) -> bool {
    imp::is_foreground(hwnd)
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GA_ROOT};

    pub(super) fn is_foreground(hwnd: *mut c_void) -> bool {
        if hwnd.is_null() {
            return false;
        }
        // SAFETY: plain call with no arguments; it may return a null HWND.
        let front = unsafe { GetForegroundWindow() };
        if front.is_invalid() {
            return false;
        }
        // SAFETY: GetAncestor accepts any HWND, even one that has just
        // been destroyed (it then returns null).
        let root = unsafe { GetAncestor(front, GA_ROOT) };
        front == HWND(hwnd) || root == HWND(hwnd)
    }
}

#[cfg(not(windows))]
mod imp {
    use std::ffi::c_void;

    pub(super) fn is_foreground(_hwnd: *mut c_void) -> bool {
        false
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::is_foreground;

    #[test]
    fn a_null_window_is_never_in_front() {
        assert!(!is_foreground(std::ptr::null_mut()));
    }
}
