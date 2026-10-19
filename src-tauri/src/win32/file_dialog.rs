//! The native "pick programs" dialog (`IFileOpenDialog`).

use std::path::PathBuf;

/// Shows a modal multi-select open dialog filtered to Programs (`*.exe`,
/// `*.lnk`), owned by the window `owner` (an `HWND` value) when given.
/// Shortcuts are returned as the `.lnk` files themselves, not their targets.
///
/// Blocks until the dialog closes; call it on a background thread. It holds
/// its own `ComGuard` for the whole call. Returns `Ok(vec![])` when the user
/// cancels, and `Err` for any other failure (always
/// `Err("unsupported platform")` on non-Windows platforms).
pub fn pick_programs(owner: Option<isize>) -> Result<Vec<PathBuf>, String> {
    imp::pick_programs(owner)
}

#[cfg(windows)]
pub use imp::picker_options;

#[cfg(windows)]
mod imp {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FILEOPENDIALOGOPTIONS, FOS_ALLOWMULTISELECT,
        FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_NODEREFERENCELINKS,
        FOS_PATHMUSTEXIST, SIGDN_FILESYSPATH,
    };

    use crate::win32::com::ComGuard;
    use crate::win32::wide::to_wide;

    /// The dialog's `current` options plus the ones this picker needs:
    /// several files, existing files in existing folders only, the process
    /// working folder left alone, shortcuts returned as `.lnk` files, and
    /// file-system paths only.
    pub fn picker_options(current: FILEOPENDIALOGOPTIONS) -> FILEOPENDIALOGOPTIONS {
        current
            | FOS_ALLOWMULTISELECT
            | FOS_FILEMUSTEXIST
            | FOS_PATHMUSTEXIST
            | FOS_NOCHANGEDIR
            | FOS_NODEREFERENCELINKS
            | FOS_FORCEFILESYSTEM
    }

    pub(super) fn pick_programs(owner: Option<isize>) -> Result<Vec<PathBuf>, String> {
        let _com = ComGuard::new()?;
        // Every COM object lives inside show(), so all are released before
        // the guard is dropped.
        show(owner.map(|hwnd| HWND(hwnd as *mut core::ffi::c_void)))
    }

    fn show(owner: Option<HWND>) -> Result<Vec<PathBuf>, String> {
        let class = &FileOpenDialog;
        // SAFETY: COM is initialised on this thread by the caller's guard.
        let created = unsafe { CoCreateInstance(class, None, CLSCTX_INPROC_SERVER) };
        let dialog: IFileOpenDialog =
            created.map_err(|e| format!("create the file dialog: {e}"))?;
        // SAFETY: a plain call on a live dialog.
        let current = unsafe { dialog.GetOptions() }.map_err(|e| format!("dialog options: {e}"))?;
        // SAFETY: a plain call on a live dialog with a valid options value.
        unsafe { dialog.SetOptions(picker_options(current)) }
            .map_err(|e| format!("set dialog options: {e}"))?;
        // The filter strings are NUL-terminated and outlive the dialog's use
        // of them (it copies them in SetFileTypes).
        let name = to_wide("Programs");
        let spec = to_wide("*.exe;*.lnk");
        let filters = [COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        }];
        // SAFETY: filters points to valid NUL-terminated strings (above).
        unsafe { dialog.SetFileTypes(&filters) }.map_err(|e| format!("set dialog filter: {e}"))?;
        // SAFETY: owner is a window handle from Tauri or None.
        match unsafe { dialog.Show(owner) } {
            Ok(()) => {}
            Err(error) if error.code() == ERROR_CANCELLED.to_hresult() => return Ok(Vec::new()),
            Err(error) => return Err(format!("show the file dialog: {error}")),
        }
        // SAFETY: a plain call on a dialog that was just confirmed.
        let items = unsafe { dialog.GetResults() }.map_err(|e| format!("dialog results: {e}"))?;
        // SAFETY: a plain call on a live item array.
        let count = unsafe { items.GetCount() }.map_err(|e| format!("result count: {e}"))?;
        let mut paths = Vec::new();
        for at in 0..count {
            // SAFETY: at is below the count the array reported.
            let item = unsafe { items.GetItemAt(at) }.map_err(|e| format!("result {at}: {e}"))?;
            // SAFETY: a plain call on a live item; the returned string is
            // freed below with CoTaskMemFree, as documented.
            let text = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
                .map_err(|e| format!("path of result {at}: {e}"))?;
            // SAFETY: text is a valid NUL-terminated string from the Shell
            // and is freed exactly once, right after it is copied.
            let path = unsafe {
                let path = PathBuf::from(OsString::from_wide(text.as_wide()));
                CoTaskMemFree(Some(text.0.cast_const().cast()));
                path
            };
            paths.push(path);
        }
        Ok(paths)
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::PathBuf;

    pub(super) fn pick_programs(_owner: Option<isize>) -> Result<Vec<PathBuf>, String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use windows::Win32::UI::Shell::{
        FILEOPENDIALOGOPTIONS, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
        FOS_NOCHANGEDIR, FOS_NODEREFERENCELINKS, FOS_PATHMUSTEXIST, FOS_STRICTFILETYPES,
    };

    use super::picker_options;

    #[test]
    fn the_picker_adds_all_six_options_and_keeps_the_current_ones() {
        let options = picker_options(FOS_STRICTFILETYPES);
        for flag in [
            FOS_ALLOWMULTISELECT,
            FOS_FILEMUSTEXIST,
            FOS_PATHMUSTEXIST,
            FOS_NOCHANGEDIR,
            FOS_NODEREFERENCELINKS,
            FOS_FORCEFILESYSTEM,
            FOS_STRICTFILETYPES,
        ] {
            assert_eq!(options.0 & flag.0, flag.0, "{flag:?} missing");
        }
        // Nothing else is switched on.
        let six = picker_options(FILEOPENDIALOGOPTIONS(0));
        assert_eq!(six.0 | FOS_STRICTFILETYPES.0, options.0);
    }
}
