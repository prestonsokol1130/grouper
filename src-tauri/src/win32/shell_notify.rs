//! Tells Explorer and the taskbar that a shortcut and its icon changed.

use std::path::Path;

/// Notifies the Shell that the shortcut at `lnk` and the icon file `icon`
/// it shows (index 0) were written, so Explorer, the Start menu and the
/// taskbar (including pins that show the same icon file) reload the icon.
/// Called after both a first write and a rewrite.
///
/// In order:
/// 1. `SHUpdateImageW(icon, 0, 0, index)` when the Shell already caches an
///    image for `icon,0`: that cached image (possibly the old picture, or a
///    blank one cached by a failed read) is thrown away.
/// 2. `SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW | SHCNF_FLUSHNOWAIT)`
///    for `icon`, then for `lnk`.
/// 3. `SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST | SHCNF_FLUSHNOWAIT)`,
///    which makes the Shell drop its cached icons.
///
/// `FLUSHNOWAIT` hands the events over at once without waiting for every
/// listener, so a hung window cannot block the shortcut worker.
///
/// Never fails; does nothing on non-Windows platforms.
pub fn notify_shortcut_changed(lnk: &Path, icon: &Path) {
    imp::notify(lnk, icon);
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHChangeNotify, SHUpdateImageW, Shell_GetCachedImageIndexW, SHCNE_ASSOCCHANGED,
        SHCNE_UPDATEITEM, SHCNF_FLUSHNOWAIT, SHCNF_IDLIST, SHCNF_PATHW,
    };

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub(super) fn notify(lnk: &Path, icon: &Path) {
        update_image(icon);
        update_item(icon);
        update_item(lnk);
        // SAFETY: SHCNE_ASSOCCHANGED takes no items; both are NULL, as
        // documented for this event with SHCNF_IDLIST.
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED,
                SHCNF_IDLIST | SHCNF_FLUSHNOWAIT,
                None,
                None,
            )
        };
    }

    /// Drops the Shell's cached image for `icon,0`, if it has one.
    fn update_image(icon: &Path) {
        let wide = wide(icon);
        // SAFETY: wide is a NUL-terminated UTF-16 path that outlives the
        // call; the Shell only reads it. -1 means nothing is cached.
        let cached = unsafe { Shell_GetCachedImageIndexW(PCWSTR(wide.as_ptr()), 0, 0) };
        if cached >= 0 {
            // SAFETY: same string as above; the Shell copies what it needs
            // and does not keep the pointer.
            unsafe { SHUpdateImageW(PCWSTR(wide.as_ptr()), 0, 0, cached) };
        }
    }

    fn update_item(path: &Path) {
        let wide = wide(path);
        // SAFETY: with SHCNF_PATHW, dwItem1 is a NUL-terminated UTF-16
        // path; wide is NUL-terminated and outlives the call, and the
        // Shell does not keep the pointer. dwItem2 is unused (NULL).
        unsafe {
            SHChangeNotify(
                SHCNE_UPDATEITEM,
                SHCNF_PATHW | SHCNF_FLUSHNOWAIT,
                Some(wide.as_ptr().cast()),
                None,
            )
        };
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub(super) fn notify(_lnk: &Path, _icon: &Path) {}
}
