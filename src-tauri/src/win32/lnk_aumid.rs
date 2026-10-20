//! Reads the AppUserModelID stored in a shortcut (`.lnk`) file.

use std::path::Path;

/// The `System.AppUserModel.ID` of the shortcut at `path`: `Ok(None)` when
/// it has none (or an empty one), `Ok(Some(id))` for a string ID.
///
/// The caller's thread must hold a live
/// [`ComGuard`](super::com::ComGuard) for the whole call. Every COM
/// interface is released and the PROPVARIANT cleared before this returns.
/// `Err` when the file cannot be loaded as a shortcut, the property cannot
/// be read, or it holds something other than a string. Always
/// `Err("unsupported platform")` on non-Windows platforms.
pub fn read_lnk_aumid(path: &Path) -> Result<Option<String>, String> {
    imp::read_lnk_aumid(path)
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
    use windows::Win32::System::Com::{
        CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
    };
    use windows::Win32::System::Variant::{VT_EMPTY, VT_LPWSTR};
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    pub(super) fn read_lnk_aumid(path: &Path) -> Result<Option<String>, String> {
        let shown = path.display();
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: COM is initialised on this thread (the caller holds a
        // ComGuard, otherwise this returns CO_E_NOTINITIALIZED as Err).
        let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| format!("create ShellLink: {e}"))?;
        let file: IPersistFile = link
            .cast()
            .map_err(|e| format!("query IPersistFile: {e}"))?;
        // SAFETY: wide is NUL-terminated and outlives the call.
        unsafe { file.Load(PCWSTR(wide.as_ptr()), STGM_READ) }
            .map_err(|e| format!("load {shown}: {e}"))?;
        let store: IPropertyStore = link
            .cast()
            .map_err(|e| format!("query IPropertyStore: {e}"))?;
        // SAFETY: the key is a live constant. The returned PROPVARIANT is
        // owned here; it is cleared below, and its Drop clears it on any
        // other path.
        let mut value = unsafe { store.GetValue(&PKEY_AppUserModel_ID) }
            .map_err(|e| format!("read AppUserModelID of {shown}: {e}"))?;
        let vt = value.vt();
        let text = if vt == VT_LPWSTR {
            // SAFETY: vt is VT_LPWSTR, so pwszVal is the NUL-terminated
            // string (or NULL) that the PROPVARIANT owns; it is copied
            // before the PROPVARIANT is cleared.
            let raw = unsafe { value.Anonymous.Anonymous.Anonymous.pwszVal };
            if raw.is_null() {
                Ok(None)
            } else {
                // SAFETY: raw is non-NULL and NUL-terminated (see above).
                unsafe { raw.to_string() }
                    .map(|text| (!text.is_empty()).then_some(text))
                    .map_err(|e| format!("AppUserModelID of {shown} is not UTF-16: {e}"))
            }
        } else if vt == VT_EMPTY {
            Ok(None)
        } else {
            Err(format!("AppUserModelID of {shown} has type {}", vt.0))
        };
        // SAFETY: value is a PROPVARIANT from GetValue; PropVariantClear
        // frees what it owns and zeroes it (VT_EMPTY), so its Drop has
        // nothing left to free.
        if let Err(error) = unsafe { PropVariantClear(&mut value) } {
            log::warn!("PropVariantClear failed: {error}");
        }
        text
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub(super) fn read_lnk_aumid(_path: &Path) -> Result<Option<String>, String> {
        Err("unsupported platform".to_owned())
    }
}
