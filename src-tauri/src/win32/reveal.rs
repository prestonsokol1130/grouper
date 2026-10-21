//! Shows a file in Explorer with the file selected.

use std::ffi::OsString;
use std::path::Path;

use super::known_folder;

/// Starts the error of [`reveal_in_explorer`] when neither way of opening
/// Explorer worked. The frontend matches on it to say where the file is.
pub const EXPLORER_UNAVAILABLE: &str = "explorer-unavailable";

/// Opens the folder holding `path` in Explorer and selects `path`.
///
/// First asks the Shell (`SHOpenFolderAndSelectItems`, which reuses an
/// open window of that folder). That goes through the `Folder` open verb,
/// so a broken file manager registered there (a removed Directory Opus,
/// say) makes it fail with "Operation aborted". Then `explorer.exe
/// /select,"<path>"` is started directly from the Windows folder, which
/// does not use that verb. The Shell's error is logged.
///
/// Holds its own `ComGuard` for the Shell call, so it can run on any
/// thread that may block briefly (not the async runtime's workers).
/// `Err` when `path` does not exist, or, starting with
/// [`EXPLORER_UNAVAILABLE`], when both ways failed. Off Windows always
/// `Err`.
pub fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("{} does not exist", path.display()));
    }
    reveal_with(path, imp::reveal, explorer_select)
}

/// Tries `shell`, then `fallback`, as [`reveal_in_explorer`] describes.
fn reveal_with(
    path: &Path,
    shell: impl FnOnce(&Path) -> Result<(), String>,
    fallback: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let Err(shell_error) = shell(path) else {
        return Ok(());
    };
    log::warn!("{shell_error}; starting explorer.exe /select instead");
    fallback(path).map_err(|error| format!("{EXPLORER_UNAVAILABLE}: {shell_error}; {error}"))
}

/// `/select,"<path>"`, the argument that makes Explorer select `path`.
/// Passed to Explorer as is: it does not parse standard quoting.
fn select_argument(path: &Path) -> OsString {
    let mut argument = OsString::from("/select,\"");
    argument.push(path.as_os_str());
    argument.push("\"");
    argument
}

/// Starts `{Windows folder}\explorer.exe /select,"<path>"` and does not
/// wait for it (it hands the window to the running Explorer and exits).
fn explorer_select(path: &Path) -> Result<(), String> {
    let explorer = known_folder::windows_dir()
        .map_err(|e| format!("find the Windows folder: {e}"))?
        .join("explorer.exe");
    imp::spawn(&explorer, &select_argument(path))
        .map_err(|e| format!("start {}: {e}", explorer.display()))
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};

    use crate::win32::com::ComGuard;

    pub(super) fn reveal(path: &Path) -> Result<(), String> {
        let _com = ComGuard::new()?;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide is NUL-terminated and outlives the call. The returned
        // ID list is owned here and freed with ILFree below.
        let pidl = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
        if pidl.is_null() {
            return Err(format!("no Shell item for {}", path.display()));
        }
        // SAFETY: pidl is a valid, fully qualified ID list of the item to
        // select; with no child list (cidl 0) the Shell opens its parent
        // folder and selects it, as documented. COM is initialised above.
        let shown = unsafe { SHOpenFolderAndSelectItems(pidl, None, 0) };
        // SAFETY: pidl came from ILCreateFromPathW, is freed exactly once
        // and is not used after this.
        unsafe { ILFree(Some(pidl.cast_const())) };
        shown.map_err(|e| format!("show {} in Explorer: {e}", path.display()))
    }

    /// Starts `exe` with `argument` written verbatim on its command line.
    /// The child is not waited for; dropping it only closes its handles.
    pub(super) fn spawn(exe: &Path, argument: &OsStr) -> Result<(), String> {
        Command::new(exe)
            .raw_arg(argument)
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    use std::ffi::OsStr;
    use std::path::Path;

    pub(super) fn reveal(_path: &Path) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn spawn(_exe: &Path, _argument: &OsStr) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::Path;

    use super::{reveal_with, select_argument, EXPLORER_UNAVAILABLE};

    #[test]
    fn explorer_gets_the_path_quoted_after_select() {
        let path = Path::new(r"C:\Start Menu\Programs\Grouper\AI (2).lnk");
        let argument = select_argument(path);
        assert_eq!(
            argument.to_str(),
            Some(r#"/select,"C:\Start Menu\Programs\Grouper\AI (2).lnk""#)
        );
    }

    #[test]
    fn the_fallback_runs_only_when_the_shell_fails() {
        let path = Path::new("x.lnk");
        let fell_back = Cell::new(false);
        let fallback = |_: &Path| {
            fell_back.set(true);
            Ok(())
        };
        assert_eq!(reveal_with(path, |_| Ok(()), fallback), Ok(()));
        assert!(!fell_back.get());

        let aborted = |_: &Path| Err("Operation aborted (0x80004004)".to_owned());
        assert_eq!(reveal_with(path, aborted, fallback), Ok(()));
        assert!(fell_back.get());
    }

    #[test]
    fn both_failing_is_marked_and_keeps_both_errors() {
        let path = Path::new("x.lnk");
        let error = reveal_with(
            path,
            |_| Err("Operation aborted".to_owned()),
            |_| Err("no explorer.exe".to_owned()),
        )
        .unwrap_err();
        assert!(error.starts_with(EXPLORER_UNAVAILABLE), "{error}");
        assert!(error.contains("Operation aborted"), "{error}");
        assert!(error.contains("no explorer.exe"), "{error}");
    }
}
