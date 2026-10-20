//! Writes a group shortcut (`.lnk`) with its own AppUserModelID.

use std::path::{Path, PathBuf};

/// Everything written into one group shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LnkSpec {
    /// Where the shortcut is saved.
    pub lnk_path: PathBuf,
    /// The program the shortcut starts (Grouper itself).
    pub exe_path: PathBuf,
    /// Command-line arguments, for example `--flyout <group_id>`.
    pub args: String,
    /// The `.ico` shown for the shortcut (index 0).
    pub icon_path: PathBuf,
    /// The shortcut's comment (its tooltip).
    pub description: String,
    /// Stored as `System.AppUserModel.ID`, so the taskbar treats each group
    /// as its own app.
    pub aumid: String,
}

/// Writes the shortcut described by `spec`.
///
/// The working directory is the folder holding `exe_path`. The shortcut is
/// saved to a uniquely named `*.tmp.lnk` next to `lnk_path` first and then
/// renamed over it, so a half-written shortcut never exists at `lnk_path`;
/// on failure the temp file is removed and `lnk_path` is left as it was.
///
/// The caller's thread must hold a live
/// [`ComGuard`](super::com::ComGuard) for the whole call. Every COM
/// interface used here is released before this returns. On non-Windows
/// platforms this returns `Err("unsupported platform")`.
pub fn write_group_lnk(spec: &LnkSpec) -> Result<(), String> {
    let dir = spec
        .lnk_path
        .parent()
        .ok_or_else(|| format!("{} has no parent folder", spec.lnk_path.display()))?;
    crate::state::disk_write::reject_symlink(&spec.lnk_path)?;
    let temp = temp_lnk_path(&spec.lnk_path)?;
    let result = imp::save_lnk(spec, &temp).and_then(|()| {
        std::fs::rename(&temp, &spec.lnk_path).map_err(|e| {
            format!(
                "rename {} over {}: {e}",
                temp.display(),
                spec.lnk_path.display()
            )
        })
    });
    if result.is_err() {
        crate::state::disk_write::discard(&temp);
    }
    result.map_err(|error| format!("write shortcut in {}: {error}", dir.display()))
}

/// `{dir}/{stem}.{random}.tmp.lnk` next to `lnk_path`. Unique, so two
/// writes of the same shortcut never share a temp file.
fn temp_lnk_path(lnk_path: &Path) -> Result<PathBuf, String> {
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|e| format!("random temp name: {e}"))?;
    let suffix: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let stem = lnk_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(lnk_path.with_file_name(format!("{stem}.{suffix}.tmp.lnk")))
}

#[cfg(windows)]
mod imp {
    use std::mem::ManuallyDrop;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    use windows::Win32::System::Com::StructuredStorage::{
        PropVariantClear, PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
    };
    use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER};
    use windows::Win32::System::Variant::VT_LPWSTR;
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{IShellLinkW, SHStrDupW, ShellLink};

    use super::LnkSpec;
    use crate::win32::wide::to_wide;

    /// NUL-terminated UTF-16 for a path, without a lossy round trip.
    fn wide_path(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Builds and saves the shortcut to `temp`. Runs under the caller's
    /// ComGuard; `link`, `store` and `file` are released on return.
    pub(super) fn save_lnk(spec: &LnkSpec, temp: &Path) -> Result<(), String> {
        let exe = wide_path(&spec.exe_path);
        let work_dir = wide_path(spec.exe_path.parent().unwrap_or(Path::new("")));
        let args = to_wide(&spec.args);
        let icon = wide_path(&spec.icon_path);
        let description = to_wide(&spec.description);
        let temp_wide = wide_path(temp);

        // SAFETY: COM is initialised on this thread (the caller holds a
        // ComGuard, otherwise this returns CO_E_NOTINITIALIZED as Err).
        let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| format!("create ShellLink: {e}"))?;
        // SAFETY: exe is NUL-terminated and outlives the call.
        unsafe { link.SetPath(PCWSTR(exe.as_ptr())) }.map_err(|e| format!("set path: {e}"))?;
        // SAFETY: args is NUL-terminated and outlives the call.
        unsafe { link.SetArguments(PCWSTR(args.as_ptr())) }
            .map_err(|e| format!("set arguments: {e}"))?;
        // SAFETY: work_dir is NUL-terminated and outlives the call.
        unsafe { link.SetWorkingDirectory(PCWSTR(work_dir.as_ptr())) }
            .map_err(|e| format!("set working directory: {e}"))?;
        // SAFETY: icon is NUL-terminated and outlives the call.
        unsafe { link.SetIconLocation(PCWSTR(icon.as_ptr()), 0) }
            .map_err(|e| format!("set icon: {e}"))?;
        // SAFETY: description is NUL-terminated and outlives the call.
        unsafe { link.SetDescription(PCWSTR(description.as_ptr())) }
            .map_err(|e| format!("set description: {e}"))?;

        let store: IPropertyStore = link
            .cast()
            .map_err(|e| format!("query IPropertyStore: {e}"))?;
        set_string(&store, &spec.aumid)?;
        // SAFETY: a plain call on a live property store.
        unsafe { store.Commit() }.map_err(|e| format!("commit properties: {e}"))?;

        let file: IPersistFile = link
            .cast()
            .map_err(|e| format!("query IPersistFile: {e}"))?;
        // SAFETY: temp_wide is NUL-terminated and outlives the call;
        // fRemember TRUE only makes the temp file the link's current file.
        unsafe { file.Save(PCWSTR(temp_wide.as_ptr()), true) }
            .map_err(|e| format!("save {}: {e}", temp.display()))
    }

    /// Sets `PKEY_AppUserModel_ID` on `store` to `aumid` as a `VT_LPWSTR`.
    /// The PROPVARIANT is cleared on every path.
    fn set_string(store: &IPropertyStore, aumid: &str) -> Result<(), String> {
        let mut value = string_variant(aumid)?;
        // SAFETY: the key and value are live for the whole call; the store
        // copies the value and does not keep the pointer.
        let set = unsafe { store.SetValue(&PKEY_AppUserModel_ID, &value) };
        // SAFETY: value holds a VT_LPWSTR allocated by SHStrDupW, which
        // PropVariantClear frees with CoTaskMemFree. It then zeroes value
        // (VT_EMPTY), so the PROPVARIANT's own Drop has nothing left to free.
        if let Err(error) = unsafe { PropVariantClear(&mut value) } {
            log::warn!("PropVariantClear failed: {error}");
        }
        set.map_err(|e| format!("set AppUserModelID: {e}"))
    }

    /// The equivalent of `InitPropVariantFromString` (an inline helper in
    /// `propvarutil.h` that the windows crate does not export): a
    /// `VT_LPWSTR` holding a copy made by `SHStrDupW`. The windows crate's
    /// `PROPVARIANT::from(&str)` would make a `VT_BSTR` instead, which is
    /// not the documented type of `System.AppUserModel.ID`.
    fn string_variant(text: &str) -> Result<PROPVARIANT, String> {
        let wide = to_wide(text);
        // SAFETY: wide is NUL-terminated and outlives the call. The copy is
        // allocated with CoTaskMemAlloc and owned by the PROPVARIANT below,
        // whose Drop (PropVariantClear) frees it on every path.
        let copy = unsafe { SHStrDupW(PCWSTR(wide.as_ptr())) }
            .map_err(|e| format!("copy AppUserModelID: {e}"))?;
        Ok(PROPVARIANT {
            Anonymous: PROPVARIANT_0 {
                Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                    vt: VT_LPWSTR,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: PROPVARIANT_0_0_0 { pwszVal: copy },
                }),
            },
        })
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    use super::LnkSpec;

    pub(super) fn save_lnk(_spec: &LnkSpec, _temp: &Path) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}
