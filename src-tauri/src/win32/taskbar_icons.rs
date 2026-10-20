//! Reads the taskbar's button icon size from the registry (read-only):
//! `TaskbarSi` (Windows 11) and `TaskbarSmallIcons` (Windows 10) under
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced`.

use crate::flyout::icon_size::TaskbarIcons;

/// The taskbar's icon size now; the default when nothing can be read.
pub fn taskbar_icons() -> TaskbarIcons {
    let (si, small) = imp::read();
    TaskbarIcons::from_registry(si, small)
}

#[cfg(windows)]
mod imp {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    /// `(TaskbarSi, TaskbarSmallIcons)`, each `None` when missing.
    pub(super) fn read() -> (Option<u32>, Option<u32>) {
        (dword(w!("TaskbarSi")), dword(w!("TaskbarSmallIcons")))
    }

    fn dword(name: PCWSTR) -> Option<u32> {
        let mut value: u32 = 0;
        let mut size = u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4);
        // SAFETY: the key name is a static NUL-terminated wide string and
        // `name` is one too (from `w!`); `value` is a valid, writable u32
        // and `size` holds its size in bytes, as RRF_RT_REG_DWORD requires.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced"),
                name,
                RRF_RT_REG_DWORD,
                None,
                Some(std::ptr::from_mut(&mut value).cast()),
                Some(&mut size),
            )
        };
        (status == ERROR_SUCCESS).then_some(value)
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn read() -> (Option<u32>, Option<u32>) {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_taskbar_size_can_be_read() {
        // Whatever the machine's setting, reading never fails.
        let px = super::taskbar_icons().logical_px();
        assert!([16, 24, 32].contains(&px), "{px}");
    }
}
