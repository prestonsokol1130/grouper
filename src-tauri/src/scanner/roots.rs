//! The Start Menu folders that the scanner walks.

use std::path::PathBuf;

/// One Start Menu "Programs" folder to scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    pub path: PathBuf,
    /// True for the current user's folder, false for the all-users one.
    pub per_user: bool,
}

/// How many folders [`scan_roots`] returns when every lookup works.
pub const ROOT_COUNT: usize = 2;

/// The per-user and all-users Start Menu "Programs" folders, per-user first.
///
/// A folder whose path cannot be read is logged and left out, so one bad
/// folder never stops the other from being scanned; fewer than
/// [`ROOT_COUNT`] folders means a partial scan. On non-Windows platforms
/// both lookups fail, so this returns `Err`.
pub fn scan_roots() -> Result<Vec<ScanRoot>, String> {
    imp::scan_roots()
}

/// The current user's Start Menu "Programs" folder (`FOLDERID_Programs`).
/// On non-Windows platforms this returns `Err("unsupported platform")`.
pub fn programs_dir() -> Result<PathBuf, String> {
    imp::programs_dir()
}

mod imp {
    use std::path::PathBuf;

    use super::{ScanRoot, ROOT_COUNT};
    use crate::win32::known_folder;

    /// A known-folder lookup.
    type Lookup = fn() -> Result<PathBuf, String>;

    pub(super) fn scan_roots() -> Result<Vec<ScanRoot>, String> {
        let mut roots = Vec::new();
        let folders: [(Lookup, bool, &str); ROOT_COUNT] = [
            (known_folder::programs, true, "Programs"),
            (known_folder::common_programs, false, "CommonPrograms"),
        ];
        for (lookup, per_user, label) in folders {
            match lookup() {
                Ok(path) => roots.push(ScanRoot { path, per_user }),
                Err(error) => log::warn!("skipping Start Menu folder {label}: {error}"),
            }
        }
        if roots.is_empty() {
            return Err("no Start Menu folder could be found".to_owned());
        }
        Ok(roots)
    }

    pub(super) fn programs_dir() -> Result<PathBuf, String> {
        known_folder::programs().map_err(|e| format!("Start Menu Programs folder: {e}"))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{programs_dir, scan_roots, ROOT_COUNT};

    #[test]
    fn finds_both_start_menu_folders() {
        let roots = scan_roots().unwrap();
        assert_eq!(roots.len(), ROOT_COUNT);
        assert!(roots[0].per_user);
        assert!(!roots[1].per_user);
        for root in &roots {
            assert!(root.path.is_absolute(), "{}", root.path.display());
            assert!(root.path.ends_with("Programs"), "{}", root.path.display());
        }
    }

    #[test]
    fn programs_dir_is_the_per_user_root() {
        assert_eq!(programs_dir().unwrap(), scan_roots().unwrap()[0].path);
    }
}
