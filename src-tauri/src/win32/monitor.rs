//! The monitor under the cursor, for flyout placement
//! ([`crate::taskbar::placement`]).

use crate::taskbar::placement::MonitorInfo;

/// The cursor position and the monitor nearest to it, in physical pixels.
///
/// Reads the cursor (`GetCursorPos`), its monitor
/// (`MonitorFromPoint(MONITOR_DEFAULTTONEAREST)`, `GetMonitorInfoW`) and
/// that monitor's effective DPI (`GetDpiForMonitor`) as a scale (DPI / 96;
/// 1.0 with a warning if the DPI cannot be read). When the work area fills
/// the monitor (an auto-hide taskbar), it also asks the Shell for the
/// taskbar rectangle (`SHAppBarMessage(ABM_GETTASKBARPOS)`) so placement
/// can pick the edge and keep clear of it.
///
/// Coordinates are physical only in a per-monitor DPI aware (v2) thread;
/// Grouper's process is made one by Tauri's event loop. Always
/// `Err("unsupported platform")` on non-Windows platforms.
pub fn current_monitor_info() -> Result<((i32, i32), MonitorInfo), String> {
    imp::current_monitor_info()
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    use crate::taskbar::placement::{MonitorInfo, Rect};

    pub(super) fn current_monitor_info() -> Result<((i32, i32), MonitorInfo), String> {
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
        let monitor = to_rect(info.rcMonitor);
        let work = to_rect(info.rcWork);
        let taskbar = if work == monitor {
            taskbar_rect()
        } else {
            None
        };
        let m = MonitorInfo {
            monitor,
            work,
            scale: monitor_scale(hmonitor),
            taskbar,
        };
        Ok(((cursor.x, cursor.y), m))
    }

    /// Effective DPI / 96, or 1.0 if the DPI cannot be read.
    fn monitor_scale(hmonitor: HMONITOR) -> f64 {
        let (mut dpi_x, mut dpi_y) = (0_u32, 0_u32);
        // SAFETY: hmonitor is a valid monitor handle; both out-pointers
        // are valid, writable u32s for the call.
        let read = unsafe { GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        match read {
            Ok(()) if dpi_x > 0 => f64::from(dpi_x) / 96.0,
            Ok(()) => {
                log::warn!("GetDpiForMonitor returned 0 DPI; using scale 1.0");
                1.0
            }
            Err(e) => {
                log::warn!("GetDpiForMonitor failed ({e}); using scale 1.0");
                1.0
            }
        }
    }

    /// The (primary) taskbar's rectangle from the Shell, if it answers.
    fn taskbar_rect() -> Option<Rect> {
        let mut data = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };
        // SAFETY: data is a writable APPBARDATA with cbSize set; for
        // ABM_GETTASKBARPOS the Shell only fills in its rc field.
        let found = unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) };
        if found == 0 {
            log::debug!("ABM_GETTASKBARPOS found no taskbar");
            return None;
        }
        Some(to_rect(data.rc))
    }

    fn to_rect(r: RECT) -> Rect {
        Rect {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::taskbar::placement::MonitorInfo;

    pub(super) fn current_monitor_info() -> Result<((i32, i32), MonitorInfo), String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };

    use super::current_monitor_info;

    #[test]
    fn reads_a_sane_monitor_under_the_cursor() {
        // The test binary has no Tauri event loop, so make this thread
        // per-monitor aware v2 the way Grouper's process is.
        // SAFETY: plain call; it only changes this test thread's awareness.
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let ((x, y), m) = current_monitor_info().unwrap();
        let (mon, work) = (m.monitor, m.work);
        assert!(mon.left < mon.right && mon.top < mon.bottom, "{m:?}");
        assert!(work.left >= mon.left && work.right <= mon.right, "{m:?}");
        assert!(work.top >= mon.top && work.bottom <= mon.bottom, "{m:?}");
        assert!(m.scale >= 1.0 && m.scale.is_finite(), "{m:?}");
        log::info!("cursor ({x}, {y}) on {m:?}");
    }
}
