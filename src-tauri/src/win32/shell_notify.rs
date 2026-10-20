//! Tells Explorer and the taskbar that a shortcut changed.

use std::path::Path;

/// Notifies the Shell that the shortcut at `path` was rewritten, so
/// Explorer, the Start menu and the taskbar reload its icon:
/// `SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, path)` and then
/// `SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, NULL, NULL)`, which
/// makes the Shell drop its cached icons.
///
/// Never fails; does nothing on non-Windows platforms.
pub fn notify_shortcut_changed(path: &Path) {
    imp::notify(path);
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::UI::Shell::{
        SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNE_UPDATEITEM, SHCNF_IDLIST, SHCNF_PATHW,
    };

    pub(super) fn notify(path: &Path) {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: with SHCNF_PATHW, dwItem1 is a NUL-terminated UTF-16
        // path; wide is NUL-terminated and outlives the call, and the
        // Shell does not keep the pointer. dwItem2 is unused (NULL).
        unsafe {
            SHChangeNotify(
                SHCNE_UPDATEITEM,
                SHCNF_PATHW,
                Some(wide.as_ptr().cast()),
                None,
            )
        };
        // SAFETY: SHCNE_ASSOCCHANGED takes no items; both are NULL, as
        // documented for this event with SHCNF_IDLIST.
        unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub(super) fn notify(_path: &Path) {}
}
