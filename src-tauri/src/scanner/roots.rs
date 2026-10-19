//! The Start Menu folders that the scanner walks.

use std::path::PathBuf;

/// One Start Menu "Programs" folder to scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    pub path: PathBuf,
    /// True for the current user's folder, false for the all-users one.
    pub per_user: bool,
}

/// The per-user and all-users Start Menu "Programs" folders, per-user first.
///
/// A folder whose path cannot be read is logged and left out, so one bad
/// folder never stops the other from being scanned. On non-Windows
/// platforms this returns `Err("unsupported platform")`.
pub fn scan_roots() -> Result<Vec<ScanRoot>, String> {
    imp::scan_roots()
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    use windows::core::GUID;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_CommonPrograms, FOLDERID_Programs, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
    };

    use super::ScanRoot;

    pub(super) fn scan_roots() -> Result<Vec<ScanRoot>, String> {
        let mut roots = Vec::new();
        for (id, per_user, label) in [
            (FOLDERID_Programs, true, "Programs"),
            (FOLDERID_CommonPrograms, false, "CommonPrograms"),
        ] {
            match known_folder(&id) {
                Ok(path) => roots.push(ScanRoot { path, per_user }),
                Err(error) => log::warn!("skipping Start Menu folder {label}: {error}"),
            }
        }
        if roots.is_empty() {
            return Err("no Start Menu folder could be found".to_owned());
        }
        Ok(roots)
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
    use super::ScanRoot;

    pub(super) fn scan_roots() -> Result<Vec<ScanRoot>, String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::scan_roots;

    #[test]
    fn finds_both_start_menu_folders() {
        let roots = scan_roots().unwrap();
        assert_eq!(roots.len(), 2);
        assert!(roots[0].per_user);
        assert!(!roots[1].per_user);
        for root in &roots {
            assert!(root.path.is_absolute(), "{}", root.path.display());
            assert!(root.path.ends_with("Programs"), "{}", root.path.display());
        }
    }
}
