//! Moves a finished temp file over a file that Explorer may hold open.
//!
//! Explorer, the Start menu, the taskbar and the search indexer open a
//! shortcut to read its icon and properties, often without
//! `FILE_SHARE_DELETE`. While they do, replacing it fails with "Access is
//! denied" (os error 5) or a sharing violation (os error 32). The handle
//! is let go within moments, so the move is tried again a few times, as
//! the group `.ico` write does. A read-only target fails the same way,
//! so its read-only flag is cleared before the first retry.
//!
//! `ReplaceFileW` was considered: it needs the same `DELETE` access to the
//! old file, so it fails in the same cases, and it would carry the old
//! file's attributes (read-only included) over to the new one.

use std::io;
use std::path::Path;
use std::time::Duration;

/// The pauses before each retry, about 0.75 s in all.
const BACKOFF: [Duration; 4] = [
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
];

/// `ERROR_ACCESS_DENIED`, `ERROR_SHARING_VIOLATION`, `ERROR_LOCK_VIOLATION`:
/// the target is open in another process, or read-only.
const BUSY: [i32; 3] = [5, 32, 33];

/// `ERROR_NOT_SAME_DEVICE`: the two paths are on different volumes.
const NOT_SAME_DEVICE: i32 = 17;

/// True when `error` says the target is busy (open elsewhere or read-only).
pub fn is_busy(error: &io::Error) -> bool {
    error
        .raw_os_error()
        .is_some_and(|code| BUSY.contains(&code))
}

/// True when `error` says the move crossed volumes.
pub fn is_cross_volume(error: &io::Error) -> bool {
    error.raw_os_error() == Some(NOT_SAME_DEVICE)
}

/// Renames `from` over `to`. While `to` is busy (see [`is_busy`]) the
/// rename is tried again after each pause in the backoff; a read-only `to`
/// is made writable first. Returns the last error when every try failed.
/// `from` is never removed here; the caller discards it on `Err`.
pub fn move_over(from: &Path, to: &Path) -> io::Result<()> {
    let mut pauses = BACKOFF.iter();
    let mut first = true;
    loop {
        let error = match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) if is_busy(&error) => error,
            Err(error) => return Err(error),
        };
        if std::mem::take(&mut first) {
            clear_read_only(to);
        }
        let Some(pause) = pauses.next() else {
            return Err(error);
        };
        log::debug!("{} is busy ({error}); trying again", to.display());
        std::thread::sleep(*pause);
    }
}

/// Clears the read-only flag of the regular file `path`, if it has one.
/// Failures are logged; the next rename reports the real error.
#[cfg(windows)]
fn clear_read_only(path: &Path) {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return;
    };
    if !meta.is_file() || !meta.permissions().readonly() {
        return;
    }
    let mut permissions = meta.permissions();
    // On Windows this only clears FILE_ATTRIBUTE_READONLY; the lint is
    // about Unix modes, where it would make the file world-writable.
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    match std::fs::set_permissions(path, permissions) {
        Ok(()) => log::info!("cleared the read-only flag of {}", path.display()),
        Err(error) => log::warn!("could not make {} writable: {error}", path.display()),
    }
}

#[cfg(not(windows))]
fn clear_read_only(_path: &Path) {}

#[cfg(all(test, windows))]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::{Duration, Instant};

    use super::{is_busy, move_over};

    /// `FILE_SHARE_READ` only: no `FILE_SHARE_DELETE`, as Explorer often
    /// opens a shortcut.
    const SHARE_READ: u32 = 1;

    #[test]
    fn waits_for_a_handle_on_the_target_to_close() {
        let temp = tempfile::tempdir().unwrap();
        let (from, to) = (temp.path().join("new.tmp"), temp.path().join("x.lnk"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();
        let held = OpenOptions::new()
            .read(true)
            .share_mode(SHARE_READ)
            .open(&to)
            .unwrap();
        // A plain rename fails while the handle is open.
        let error = fs::rename(&from, &to).unwrap_err();
        assert!(is_busy(&error), "{error}");
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            drop(held);
        });
        move_over(&from, &to).unwrap();
        release.join().unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
        assert!(!from.exists());
    }

    #[test]
    fn gives_up_while_the_target_stays_open() {
        let temp = tempfile::tempdir().unwrap();
        let (from, to) = (temp.path().join("new.tmp"), temp.path().join("x.lnk"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();
        let _held = OpenOptions::new()
            .read(true)
            .share_mode(SHARE_READ)
            .open(&to)
            .unwrap();
        let started = Instant::now();
        let error = move_over(&from, &to).unwrap_err();
        assert!(is_busy(&error), "{error}");
        assert!(started.elapsed() >= Duration::from_millis(700));
        assert!(from.exists(), "the caller discards the temp file");
    }

    #[test]
    fn replaces_a_read_only_target() {
        let temp = tempfile::tempdir().unwrap();
        let (from, to) = (temp.path().join("new.tmp"), temp.path().join("x.lnk"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();
        let mut permissions = fs::metadata(&to).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&to, permissions).unwrap();
        move_over(&from, &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
    }
}
