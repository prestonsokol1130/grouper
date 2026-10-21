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
    /// A folder the Shell does not watch, on the same volume as `lnk_path`
    /// (Grouper's icon cache), where the shortcut is saved before it is
    /// moved into place.
    pub stage_dir: PathBuf,
}

/// Writes the shortcut described by `spec`.
///
/// The working directory is the folder holding `exe_path`. The shortcut is
/// saved complete to a uniquely named `*.tmp` file in `stage_dir`, then
/// moved over `lnk_path`, so a half-written shortcut never exists at
/// `lnk_path`. On failure the temp file is removed and `lnk_path` is left
/// as it was.
///
/// The temp file is kept out of the shortcut's folder on purpose. Explorer,
/// the Start menu and the taskbar watch that folder: a temp shortcut that
/// appears there and is renamed a few milliseconds later has its icon read
/// after it is gone, so the Shell caches the blank document icon for the
/// group's shortcut. Only when the move crosses volumes (`stage_dir` is
/// on another volume, as with a redirected Start Menu) is the temp file
/// saved next to `lnk_path` instead, still named `*.tmp`, never `*.lnk`.
/// A `lnk_path` that Explorer holds open is waited for
/// ([`move_over`](super::file_replace::move_over)), not worked around.
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
    let result = match save_and_move(spec, &spec.stage_dir) {
        Err(Staged::NotMoved(error, true)) => {
            log::warn!("{error}; saving the shortcut in its own folder");
            save_and_move(spec, dir).map_err(Staged::into_message)
        }
        other => other.map_err(Staged::into_message),
    };
    result.map_err(|error| format!("write shortcut in {}: {error}", dir.display()))
}

/// Why [`save_and_move`] failed.
enum Staged {
    /// The shortcut could not be saved.
    NotSaved(String),
    /// It was saved, but could not be moved into place; true when the
    /// move crossed volumes.
    NotMoved(String, bool),
}

impl Staged {
    fn into_message(self) -> String {
        match self {
            Self::NotSaved(message) | Self::NotMoved(message, _) => message,
        }
    }
}

/// Saves the shortcut to a fresh temp file in `stage`, then moves it over
/// `spec.lnk_path`. The temp file is removed on any failure.
fn save_and_move(spec: &LnkSpec, stage: &Path) -> Result<(), Staged> {
    crate::state::disk_write::reject_symlink(stage).map_err(Staged::NotSaved)?;
    let temp = temp_lnk_path(stage, &spec.lnk_path).map_err(Staged::NotSaved)?;
    if let Err(error) = imp::save_lnk(spec, &temp) {
        crate::state::disk_write::discard(&temp);
        return Err(Staged::NotSaved(error));
    }
    super::file_replace::move_over(&temp, &spec.lnk_path).map_err(|e| {
        crate::state::disk_write::discard(&temp);
        let (from, to) = (temp.display(), spec.lnk_path.display());
        let cross_volume = super::file_replace::is_cross_volume(&e);
        Staged::NotMoved(format!("move {from} over {to}: {e}"), cross_volume)
    })
}

/// `{stage}/{stem}.{random}.tmp` for `lnk_path`. Unique, so two writes of
/// the same shortcut never share a temp file, and never named `*.lnk`, so
/// the Shell does not take it for a shortcut.
fn temp_lnk_path(stage: &Path, lnk_path: &Path) -> Result<PathBuf, String> {
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|e| format!("random temp name: {e}"))?;
    let suffix: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let stem = lnk_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(stage.join(format!("{stem}.{suffix}.tmp")))
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
        let final_wide = wide_path(&spec.lnk_path);

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
        // The file is saved elsewhere and then moved, so the target's
        // relative path is worked out from where the shortcut ends up.
        // SAFETY: final_wide is NUL-terminated and outlives the call.
        unsafe { link.SetRelativePath(PCWSTR(final_wide.as_ptr()), 0) }
            .map_err(|e| format!("set relative path: {e}"))?;

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
