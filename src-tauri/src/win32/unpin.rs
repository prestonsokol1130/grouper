//! Unpinning a shortcut from Start and the taskbar before it is deleted.
//!
//! Windows does not unpin anything when a shortcut or its program is
//! removed: the pin stays, with a blank icon. `IStartMenuPinnedList::
//! RemoveFromList` is the documented way for an uninstaller to remove an
//! item from the Start pinned list and, since Windows 7, the taskbar too.
//! The Tauri NSIS uninstaller uses the same call for its own shortcut.

use std::path::Path;

/// Unpins `shortcut` from Start and the taskbar. The caller holds a
/// [`super::com::ComGuard`]. `Err` off Windows or when the Shell refuses.
pub fn unpin(shortcut: &Path) -> Result<(), String> {
    imp::unpin(shortcut)
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::{
        IShellItem, IStartMenuPinnedList, SHCreateItemFromParsingName, StartMenuPin,
    };

    pub(super) fn unpin(shortcut: &Path) -> Result<(), String> {
        let wide: Vec<u16> = shortcut
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `wide` is NUL-terminated and outlives the call; COM is
        // initialised on this thread by the caller's ComGuard.
        let item: IShellItem = unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) }
            .map_err(|e| format!("open {}: {e}", shortcut.display()))?;
        // SAFETY: StartMenuPin is the documented in-process class for
        // IStartMenuPinnedList; the interface is released on drop.
        let list: IStartMenuPinnedList =
            unsafe { CoCreateInstance(&StartMenuPin, None, CLSCTX_INPROC_SERVER) }
                .map_err(|e| format!("StartMenuPin: {e}"))?;
        // SAFETY: `item` is a live IShellItem for the call's duration.
        unsafe { list.RemoveFromList(&item) }
            .map_err(|e| format!("unpin {}: {e}", shortcut.display()))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub(super) fn unpin(_shortcut: &Path) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}
