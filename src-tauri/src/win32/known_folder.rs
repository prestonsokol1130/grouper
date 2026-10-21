//! The current user's known folders (Start Menu, AppData), read with
//! `SHGetKnownFolderPath`. Off Windows every lookup returns
//! `Err("unsupported platform")`.

use std::path::PathBuf;

/// The current user's Start Menu "Programs" folder (`FOLDERID_Programs`).
pub fn programs() -> Result<PathBuf, String> {
    imp::programs()
}

/// The common (all users) Start Menu "Programs" folder.
pub fn common_programs() -> Result<PathBuf, String> {
    imp::common_programs()
}

/// `%APPDATA%` (`FOLDERID_RoamingAppData`). Tauri's `app_data_dir` is
/// this folder plus the bundle identifier.
pub fn roaming_app_data() -> Result<PathBuf, String> {
    imp::roaming_app_data()
}

/// `%LOCALAPPDATA%` (`FOLDERID_LocalAppData`). Tauri's `app_log_dir` is
/// this folder plus the bundle identifier and `logs`.
pub fn local_app_data() -> Result<PathBuf, String> {
    imp::local_app_data()
}

/// `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned`
/// (`FOLDERID_UserPinned`). Its `TaskBar` folder holds copies of the
/// shortcuts the user pinned to the taskbar.
pub fn user_pinned() -> Result<PathBuf, String> {
    imp::user_pinned()
}

/// The Windows folder, usually `C:\Windows` (`FOLDERID_Windows`).
pub fn windows_dir() -> Result<PathBuf, String> {
    imp::windows_dir()
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    use windows::core::GUID;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_CommonPrograms, FOLDERID_LocalAppData, FOLDERID_Programs, FOLDERID_RoamingAppData,
        FOLDERID_UserPinned, FOLDERID_Windows, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
    };

    pub(super) fn programs() -> Result<PathBuf, String> {
        known_folder(&FOLDERID_Programs)
    }

    pub(super) fn common_programs() -> Result<PathBuf, String> {
        known_folder(&FOLDERID_CommonPrograms)
    }

    pub(super) fn roaming_app_data() -> Result<PathBuf, String> {
        known_folder(&FOLDERID_RoamingAppData)
    }

    pub(super) fn local_app_data() -> Result<PathBuf, String> {
        known_folder(&FOLDERID_LocalAppData)
    }

    pub(super) fn user_pinned() -> Result<PathBuf, String> {
        known_folder(&FOLDERID_UserPinned)
    }

    pub(super) fn windows_dir() -> Result<PathBuf, String> {
        known_folder(&FOLDERID_Windows)
    }

    /// The path of known folder `id` for the current user.
    fn known_folder(id: &GUID) -> Result<PathBuf, String> {
        // SAFETY: `id` points to a live GUID for the whole call, and a None
        // token asks for the current user's folder as documented. On
        // failure the windows crate returns Err without handing back the
        // pointer, which the API leaves NULL.
        let raw = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }
            .map_err(|e| format!("SHGetKnownFolderPath: {e}"))?;
        // SAFETY: on success `raw` is a valid NUL-terminated wide string
        // owned by us until the CoTaskMemFree below; the slice is copied
        // into an OsString before that.
        let path = unsafe { OsString::from_wide(raw.as_wide()) };
        // SAFETY: `raw` was allocated by SHGetKnownFolderPath with the COM
        // task allocator, is freed exactly once, and is not used after.
        unsafe { CoTaskMemFree(Some(raw.0 as *const _)) };
        if path.is_empty() {
            return Err("empty path".to_owned());
        }
        Ok(PathBuf::from(path))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::PathBuf;

    fn unsupported() -> Result<PathBuf, String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn programs() -> Result<PathBuf, String> {
        unsupported()
    }

    pub(super) fn common_programs() -> Result<PathBuf, String> {
        unsupported()
    }

    pub(super) fn roaming_app_data() -> Result<PathBuf, String> {
        unsupported()
    }

    pub(super) fn local_app_data() -> Result<PathBuf, String> {
        unsupported()
    }

    pub(super) fn user_pinned() -> Result<PathBuf, String> {
        unsupported()
    }

    pub(super) fn windows_dir() -> Result<PathBuf, String> {
        unsupported()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{local_app_data, roaming_app_data};

    #[test]
    fn app_data_folders_resolve() {
        let roaming = roaming_app_data().unwrap();
        let local = local_app_data().unwrap();
        assert!(roaming.ends_with("Roaming"), "{}", roaming.display());
        assert!(local.ends_with("Local"), "{}", local.display());
    }
}
