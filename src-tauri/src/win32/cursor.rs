//! The cursor and the rectangle of the monitor it is on, cheaply enough to
//! poll (the hot-corner watcher reads it every 50ms).

use crate::taskbar::placement::Rect;

/// The cursor position and its monitor's full rectangle, in physical
/// pixels: `GetCursorPos`, `MonitorFromPoint` and `GetMonitorInfoW`, and
/// nothing else (no DPI or taskbar queries). `Err` while the cursor cannot
/// be read (for example on the secure desktop) and on other platforms.
pub fn cursor_monitor() -> Result<((i32, i32), Rect), String> {
    imp::cursor_monitor()
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    use crate::taskbar::placement::Rect;

    pub(super) fn cursor_monitor() -> Result<((i32, i32), Rect), String> {
        let mut cursor = POINT::default();
        // SAFETY: cursor is a valid, writable POINT for the call.
        unsafe { GetCursorPos(&mut cursor) }.map_err(|e| format!("GetCursorPos: {e}"))?;
        // SAFETY: plain call; NEAREST always returns a monitor handle.
        let hmonitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
        if hmonitor.is_invalid() {
            return Err("MonitorFromPoint returned no monitor".to_owned());
        }
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: hmonitor came from MonitorFromPoint above; info is a
        // writable MONITORINFO whose cbSize is set as the API requires.
        if !unsafe { GetMonitorInfoW(hmonitor, &mut info) }.as_bool() {
            return Err("GetMonitorInfoW failed".to_owned());
        }
        let r = info.rcMonitor;
        let rect = Rect {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        };
        Ok(((cursor.x, cursor.y), rect))
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::taskbar::placement::Rect;

    pub(super) fn cursor_monitor() -> Result<((i32, i32), Rect), String> {
        Err("unsupported platform".to_owned())
    }
}
