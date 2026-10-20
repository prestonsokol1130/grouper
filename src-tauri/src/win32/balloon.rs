//! A notification balloon on Grouper's own tray icon (`NIF_INFO`).
//!
//! The tray icon belongs to the `tray-icon` crate, which gives each icon
//! its own hidden window but keeps the icon's id (`uID`) private. Its ids
//! come from a counter that starts at 1, so the first few ids on that
//! window are tried in turn: `NIM_MODIFY` only succeeds for an icon that
//! window really added, so a wrong id changes nothing.

/// Ids tried on the tray icon's window, see the module comment.
const IDS_TRIED: u32 = 16;

/// Shows a warning balloon with `title` and `text` on the tray icon owned
/// by window `hwnd`. `Err` when no icon of that window took it (or on
/// other platforms).
pub fn show_balloon(hwnd: isize, title: &str, text: &str) -> Result<(), String> {
    imp::show(hwnd, title, text)
}

/// Copies `text` into a fixed UTF-16 buffer, cut to leave the final NUL.
#[cfg_attr(not(windows), allow(dead_code))]
fn fill<const N: usize>(buffer: &mut [u16; N], text: &str) {
    for (slot, unit) in buffer
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(text.encode_utf16())
    {
        *slot = unit;
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_INFO, NIIF_WARNING, NIM_MODIFY, NOTIFYICONDATAW,
    };

    use super::{fill, IDS_TRIED};

    pub(super) fn show(hwnd: isize, title: &str, text: &str) -> Result<(), String> {
        let mut data = NOTIFYICONDATAW {
            // The struct is under 1 KB, so the cast is exact.
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: HWND(hwnd as *mut std::ffi::c_void),
            uFlags: NIF_INFO,
            dwInfoFlags: NIIF_WARNING,
            ..Default::default()
        };
        fill(&mut data.szInfoTitle, title);
        fill(&mut data.szInfo, text);
        for id in 1..=IDS_TRIED {
            data.uID = id;
            // SAFETY: data is a fully initialised NOTIFYICONDATAW with its
            // cbSize set; its strings are NUL-terminated fixed buffers
            // inside the struct, which outlives the call. NIM_MODIFY with
            // NIF_INFO only changes the balloon of an existing icon.
            if unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
                return Ok(());
            }
        }
        Err("no tray icon took the balloon".to_owned())
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn show(_hwnd: isize, _title: &str, _text: &str) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn text_is_cut_to_leave_the_nul() {
        let mut buffer = [0u16; 4];
        fill(&mut buffer, "Grouper");
        assert_eq!(buffer, [71, 114, 111, 0]);
        let mut buffer = [0u16; 8];
        fill(&mut buffer, "Hi");
        assert_eq!(&buffer[..3], &[72, 105, 0]);
    }
}
