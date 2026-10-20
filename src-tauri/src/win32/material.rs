//! Window backdrops (Mica, Acrylic) through the `window-vibrancy` crate,
//! and the system switches that rule them out. On other platforms no
//! backdrop is supported.

use tauri::{Runtime, WebviewWindow};

/// A backdrop the window manager can draw behind a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    Mica,
    Acrylic,
}

/// Whether Windows allows translucent backdrops now: "Transparency
/// effects" is on and battery saver is off. Unknown counts as allowed.
pub fn backdrops_allowed() -> bool {
    imp::transparency_enabled() && !imp::battery_saver_on()
}

/// Applies `backdrop` to `window`. `Err` when this Windows version (or
/// platform) does not support it.
pub fn apply_backdrop<R: Runtime>(
    window: &WebviewWindow<R>,
    backdrop: Backdrop,
) -> Result<(), String> {
    imp::apply(window, backdrop)
}

#[cfg(windows)]
mod imp {
    use tauri::{Runtime, WebviewWindow};
    use windows::core::w;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    use super::Backdrop;

    /// `HKCU\...\Themes\Personalize\EnableTransparency`; missing means on.
    pub(super) fn transparency_enabled() -> bool {
        let mut value: u32 = 1;
        let mut size = u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4);
        // SAFETY: the key and value names are static NUL-terminated wide
        // strings; `value` is a valid, writable u32 and `size` holds its
        // size in bytes, as RRF_RT_REG_DWORD requires.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
                w!("EnableTransparency"),
                RRF_RT_REG_DWORD,
                None,
                Some(std::ptr::from_mut(&mut value).cast()),
                Some(&mut size),
            )
        };
        status != ERROR_SUCCESS || value != 0
    }

    /// `SystemStatusFlag` is 1 while battery saver is on.
    pub(super) fn battery_saver_on() -> bool {
        let mut status = SYSTEM_POWER_STATUS::default();
        // SAFETY: `status` is a valid, writable SYSTEM_POWER_STATUS.
        match unsafe { GetSystemPowerStatus(&mut status) } {
            Ok(()) => status.SystemStatusFlag == 1,
            Err(_) => false,
        }
    }

    pub(super) fn apply<R: Runtime>(
        window: &WebviewWindow<R>,
        backdrop: Backdrop,
    ) -> Result<(), String> {
        let result = match backdrop {
            Backdrop::Mica => window_vibrancy::apply_mica(window, None),
            Backdrop::Acrylic => window_vibrancy::apply_acrylic(window, None),
        };
        result.map_err(|error| format!("{backdrop:?}: {error}"))
    }
}

#[cfg(not(windows))]
mod imp {
    use tauri::{Runtime, WebviewWindow};

    use super::Backdrop;

    pub(super) fn transparency_enabled() -> bool {
        false
    }

    pub(super) fn battery_saver_on() -> bool {
        false
    }

    pub(super) fn apply<R: Runtime>(
        _window: &WebviewWindow<R>,
        _backdrop: Backdrop,
    ) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn system_switches_can_be_read() {
        // Either answer is fine; reading must simply not fail or panic.
        let _ = super::backdrops_allowed();
    }
}
