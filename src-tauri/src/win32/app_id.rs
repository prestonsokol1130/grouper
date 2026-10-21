//! The process's own AppUserModelID.
//!
//! Windows gives a process started from a shortcut that shortcut's
//! AppUserModelID. When a group pin (`com.grouper.group.<id>`) starts
//! Grouper, every Grouper window would then sit on the taskbar as that
//! group, with its name and a blank icon ("IDE's", 2026-10-20). Setting
//! Grouper's own ID first keeps its windows apart from the group pins.

/// Sets the current process's explicit AppUserModelID to `id`. Must run
/// before any window is created. Failures are logged; does nothing on
/// other platforms.
pub fn set_process_app_id(id: &str) {
    if let Err(error) = imp::set(id) {
        log::warn!("could not set the AppUserModelID {id}: {error}");
        eprintln!("grouper: could not set the AppUserModelID {id}: {error}");
    }
}

#[cfg(windows)]
mod imp {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    pub(super) fn set(id: &str) -> Result<(), String> {
        // SAFETY: the HSTRING is a valid NUL-terminated wide string that
        // outlives the call; Windows copies it.
        unsafe { SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(id)) }
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn set(_id: &str) -> Result<(), String> {
        Ok(())
    }
}
