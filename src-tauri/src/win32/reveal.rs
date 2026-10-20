//! Shows a file in Explorer with the file selected.

use std::path::Path;

/// Opens the folder holding `path` in Explorer and selects `path`
/// (`SHOpenFolderAndSelectItems`). An already open window of that folder
/// is reused.
///
/// Holds its own `ComGuard` for the call, so it can run on any thread that
/// may block briefly (not the async runtime's workers). Returns `Err` when
/// `path` does not exist or the Shell fails; always
/// `Err("unsupported platform")` on non-Windows platforms.
pub fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("{} does not exist", path.display()));
    }
    imp::reveal(path)
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

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
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub(super) fn reveal(_path: &Path) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}
