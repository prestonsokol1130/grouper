//! Waiting for a folder to change, without polling.

use std::path::Path;
use std::time::Duration;

/// A change notification on one folder and everything under it: files
/// and folders created, deleted or renamed. Closed on drop.
#[derive(Debug)]
pub struct DirWatch {
    handle: imp::Handle,
}

impl DirWatch {
    /// Starts watching `dir`. `Err` when it cannot be watched (it is
    /// missing, or on other platforms).
    pub fn new(dir: &Path) -> Result<Self, String> {
        imp::watch(dir).map(|handle| Self { handle })
    }

    /// Blocks until something under the folder changes (`Ok(true)`) or
    /// `timeout` passes first (`Ok(false)`); `None` waits as long as it
    /// takes. Changes made while nobody waits are kept for the next call.
    /// `Err` when the watch broke (the folder itself went away).
    pub fn wait(&self, timeout: Option<Duration>) -> Result<bool, String> {
        imp::wait(&self.handle, timeout)
    }
}

impl Drop for DirWatch {
    fn drop(&mut self) {
        imp::close(&self.handle);
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::time::Duration;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::Storage::FileSystem::{
        FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification,
        FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
    };
    use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};

    /// The raw notification handle.
    #[derive(Debug)]
    pub(super) struct Handle(HANDLE);

    // SAFETY: a change-notification handle is a kernel handle; any thread
    // may wait on it, re-arm it or close it.
    unsafe impl Send for Handle {}

    pub(super) fn watch(dir: &Path) -> Result<Handle, String> {
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
        let filter = FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME;
        // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the
        // call; the returned handle is owned by the Handle and closed once.
        unsafe { FindFirstChangeNotificationW(PCWSTR(wide.as_ptr()), true, filter) }
            .map(Handle)
            .map_err(|e| format!("FindFirstChangeNotificationW({}): {e}", dir.display()))
    }

    pub(super) fn wait(handle: &Handle, timeout: Option<Duration>) -> Result<bool, String> {
        let ms = timeout.map_or(INFINITE, |t| {
            u32::try_from(t.as_millis()).unwrap_or(INFINITE - 1)
        });
        // SAFETY: the handle is a live change notification owned by us.
        let event = unsafe { WaitForSingleObject(handle.0, ms) };
        if event == WAIT_TIMEOUT {
            return Ok(false);
        }
        if event != WAIT_OBJECT_0 {
            return Err(format!("WaitForSingleObject returned {:#x}", event.0));
        }
        // SAFETY: same live handle; re-arms it for the next change.
        unsafe { FindNextChangeNotification(handle.0) }
            .map_err(|e| format!("FindNextChangeNotification: {e}"))?;
        Ok(true)
    }

    pub(super) fn close(handle: &Handle) {
        // SAFETY: the handle came from FindFirstChangeNotificationW and
        // this runs once, from Drop.
        if let Err(error) = unsafe { FindCloseChangeNotification(handle.0) } {
            log::debug!("FindCloseChangeNotification: {error}");
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;
    use std::time::Duration;

    #[derive(Debug)]
    pub(super) struct Handle;

    pub(super) fn watch(_dir: &Path) -> Result<Handle, String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn wait(_handle: &Handle, _timeout: Option<Duration>) -> Result<bool, String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn close(_handle: &Handle) {}
}

#[cfg(all(test, windows))]
mod tests {
    use std::time::Duration;

    use super::DirWatch;

    #[test]
    fn wakes_for_a_new_file_deep_inside_and_times_out_when_quiet() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("cache").join("icons");
        std::fs::create_dir_all(&deep).unwrap();
        let watch = DirWatch::new(dir.path()).unwrap();
        assert!(!watch.wait(Some(Duration::from_millis(50))).unwrap());
        std::fs::write(deep.join("a.png"), b"x").unwrap();
        assert!(watch.wait(Some(Duration::from_secs(5))).unwrap());
        std::fs::remove_dir_all(dir.path().join("cache")).unwrap();
        assert!(watch.wait(Some(Duration::from_secs(5))).unwrap());
    }

    #[test]
    fn a_missing_folder_cannot_be_watched() {
        let dir = tempfile::tempdir().unwrap();
        assert!(DirWatch::new(&dir.path().join("gone")).is_err());
    }
}
