//! Win32 tweaks for the flyout window: tool-window styles and focus.
//!
//! Window handles are passed as raw pointers (`HWND.0`) so callers do not
//! depend on which `windows` crate version Tauri was built with. On other
//! platforms every function is a no-op or returns
//! `Err("unsupported platform")`.

use std::ffi::c_void;

/// `WS_EX_TOOLWINDOW`.
const TOOLWINDOW: u32 = 0x0000_0080;
/// `WS_EX_TOPMOST`.
const TOPMOST: u32 = 0x0000_0008;
/// `WS_EX_APPWINDOW`.
const APPWINDOW: u32 = 0x0004_0000;

/// The flyout's extended style for a requested one: `WS_EX_TOOLWINDOW` and
/// `WS_EX_TOPMOST` added, `WS_EX_APPWINDOW` removed, every other bit kept.
pub fn flyout_ex_style(requested: u32) -> u32 {
    (requested | TOOLWINDOW | TOPMOST) & !APPWINDOW
}

/// Gives window `hwnd` the [`flyout_ex_style`], so it never gets a taskbar
/// button (and never takes over a group's AppUserModelID button), and puts
/// it in the topmost band without moving, sizing or activating it.
///
/// tao rewrites the whole extended style from its own flags on every show
/// and hide (putting `WS_EX_APPWINDOW` back), so the window is also
/// subclassed to keep these bits through every later change
/// (`WM_STYLECHANGING`). Call once, on the thread that owns the window.
pub fn apply_flyout_styles(hwnd: *mut c_void) -> Result<(), String> {
    imp::apply_flyout_styles(hwnd)
}

/// Asks DWM for Windows 11's rounded corners on `hwnd` (8px,
/// `DWMWCP_ROUND`, DESIGN.md section 5). Windows 10 has no such attribute,
/// so callers only log an `Err`.
pub fn round_corners(hwnd: *mut c_void) -> Result<(), String> {
    imp::round_corners(hwnd)
}

/// Lets any process take the foreground once. Called at startup by every
/// Grouper process, so a second launch from a taskbar click (which holds
/// the foreground right) passes it on to the running instance it forwards
/// to.
pub fn allow_any_foreground() {
    imp::allow_any_foreground();
}

/// Moves keyboard focus to `hwnd` with `SetForegroundWindow`. `Err` when
/// Windows refused (focus stays where it was).
pub fn bring_to_foreground(hwnd: *mut c_void) -> Result<(), String> {
    imp::bring_to_foreground(hwnd)
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, ASFW_ANY, GWL_EXSTYLE, HWND_TOPMOST, STYLESTRUCT,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, WM_DISPLAYCHANGE,
        WM_NCDESTROY, WM_STYLECHANGING,
    };

    use super::flyout_ex_style;

    /// This module's id for its subclass of the flyout window.
    const SUBCLASS_ID: usize = 0x6772_6f75;

    /// Keeps the flyout's extended style through tao's rewrites, and hides
    /// the window when the monitors change: it was placed for the old
    /// layout (a monitor it covered may be gone). The blur that follows
    /// tells Rust it is hidden.
    unsafe extern "system" fn keep_flyout_style(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_STYLECHANGING && wparam.0 as i32 == GWL_EXSTYLE.0 {
            let change = lparam.0 as *mut STYLESTRUCT;
            // SAFETY: for WM_STYLECHANGING, lParam points to a STYLESTRUCT
            // the system owns for the duration of this call (or is null).
            if let Some(change) = unsafe { change.as_mut() } {
                change.styleNew = flyout_ex_style(change.styleNew);
            }
        }
        if msg == WM_DISPLAYCHANGE {
            // SAFETY: hwnd is the live window this proc subclasses, and
            // this runs on its own thread. Hiding a hidden window is a no-op.
            let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
        }
        if msg == WM_NCDESTROY {
            // SAFETY: removes this proc's own subclass from hwnd on the
            // window's thread, as required before the window goes away.
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(keep_flyout_style), SUBCLASS_ID) };
        }
        // SAFETY: passes the message (with the adjusted STYLESTRUCT, if
        // any) on to the next handler in the subclass chain.
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }

    pub(super) fn apply_flyout_styles(hwnd: *mut c_void) -> Result<(), String> {
        let hwnd = HWND(hwnd);
        // SAFETY: hwnd is a live window owned by this process (Tauri's
        // flyout, which is never destroyed); GWL_EXSTYLE is a valid index.
        let old = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        if old == 0 {
            return Err("GetWindowLongPtrW(GWL_EXSTYLE) returned 0".to_owned());
        }
        // SAFETY: hwnd is live and this is the thread that created it (the
        // main thread, in Tauri's setup); keep_flyout_style has the
        // SUBCLASSPROC signature and cannot unwind.
        let subclassed =
            unsafe { SetWindowSubclass(hwnd, Some(keep_flyout_style), SUBCLASS_ID, 0) };
        if !subclassed.as_bool() {
            log::warn!("SetWindowSubclass failed; tao may restore WS_EX_APPWINDOW");
        }
        // Extended styles live in the low 32 bits; the truncation is the API's.
        let new = flyout_ex_style(old as u32) as isize;
        if new != old {
            // SAFETY: same live hwnd and index; the value is the window's
            // own extended style with only the flyout bits changed.
            unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new) };
        }
        // SAFETY: hwnd is live; HWND_TOPMOST is a valid insert-after
        // value, and the flags keep position, size and activation as is.
        // WS_EX_TOPMOST only takes effect through SetWindowPos.
        unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
        }
        .map_err(|e| format!("SetWindowPos(HWND_TOPMOST): {e}"))?;
        // SAFETY: plain read of the same live window's style.
        let applied = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        log::info!("flyout ex-style {old:#x} -> {applied:#x}");
        Ok(())
    }

    pub(super) fn round_corners(hwnd: *mut c_void) -> Result<(), String> {
        let preference = DWMWCP_ROUND;
        // SAFETY: hwnd is a live window of this process; the pointer and
        // size describe `preference`, a DWM_WINDOW_CORNER_PREFERENCE that
        // outlives the call, as DWMWA_WINDOW_CORNER_PREFERENCE requires.
        unsafe {
            DwmSetWindowAttribute(
                HWND(hwnd),
                DWMWA_WINDOW_CORNER_PREFERENCE,
                std::ptr::from_ref(&preference).cast(),
                size_of_val(&preference) as u32,
            )
        }
        .map_err(|e| format!("DwmSetWindowAttribute(corner preference): {e}"))
    }

    pub(super) fn allow_any_foreground() {
        // SAFETY: plain call with no pointers. It fails harmlessly when this
        // process holds no foreground right to give away.
        if let Err(error) = unsafe { AllowSetForegroundWindow(ASFW_ANY) } {
            log::debug!("AllowSetForegroundWindow(ASFW_ANY): {error}");
        }
    }

    pub(super) fn bring_to_foreground(hwnd: *mut c_void) -> Result<(), String> {
        // SAFETY: hwnd is a live window owned by this process.
        if unsafe { SetForegroundWindow(HWND(hwnd)) }.as_bool() {
            Ok(())
        } else {
            Err("SetForegroundWindow was refused".to_owned())
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::ffi::c_void;

    pub(super) fn apply_flyout_styles(_hwnd: *mut c_void) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn round_corners(_hwnd: *mut c_void) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn allow_any_foreground() {}

    pub(super) fn bring_to_foreground(_hwnd: *mut c_void) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::flyout_ex_style;

    #[test]
    fn tool_window_on_top_without_a_taskbar_button() {
        // tao's style for this window: no-redirection, on-taskbar, topmost.
        assert_eq!(flyout_ex_style(0x0024_0118), 0x0020_0198);
        assert_eq!(flyout_ex_style(0), 0x88);
        assert_eq!(flyout_ex_style(0x88), 0x88);
    }
}
