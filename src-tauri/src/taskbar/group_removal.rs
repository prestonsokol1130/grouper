//! Removing a deleted group's files, in the order that leaves no blank
//! pin behind: unpin its shortcut from Start and the taskbar, delete its
//! Start Menu shortcut, and only then delete its `.ico` files. While a pin
//! that Windows has not dropped yet still exists, it keeps showing an icon.
//!
//! The taskbar's own copies of pinned shortcuts (in `User Pinned\TaskBar`)
//! are only read, to find the ones carrying the group's AppUserModelID;
//! they are unpinned through the Shell, never edited or deleted here.

use std::path::{Path, PathBuf};

use super::group_icon_files::remove_group_icons;
use super::shortcut_sync::own_shortcuts;
use crate::win32::known_folder;

/// Unpins one shortcut from Start and the taskbar
/// ([`crate::win32::unpin::unpin`] in the app; a recorder in tests).
pub type Unpin<'a> = &'a dyn Fn(&Path) -> Result<(), String>;

/// Where pinned copies live and how to unpin.
pub struct Pins<'a> {
    /// The taskbar's pinned shortcuts, `User Pinned\TaskBar`; `None` when
    /// it cannot be found. Read only.
    pub taskbar_dir: Option<&'a Path>,
    pub unpin: Unpin<'a>,
}

/// `{FOLDERID_UserPinned}\TaskBar`.
pub fn taskbar_pins_dir() -> Result<PathBuf, String> {
    Ok(known_folder::user_pinned()?.join("TaskBar"))
}

/// Group `group_id`'s shortcuts in `dir` (by AppUserModelID), or none when
/// `dir` is missing or cannot be read (logged).
fn shortcuts_in(dir: Option<&Path>, group_id: &str) -> Vec<PathBuf> {
    let Some(dir) = dir.filter(|dir| dir.is_dir()) else {
        return Vec::new();
    };
    own_shortcuts(dir, group_id).unwrap_or_else(|error| {
        log::warn!("could not look for group {group_id}'s shortcuts: {error}");
        Vec::new()
    })
}

/// Calls `pins.unpin` on each of `paths`. A failure (most often: it was
/// not pinned) is logged and the rest still run.
pub fn unpin_each(paths: &[PathBuf], pins: &Pins<'_>) {
    for path in paths {
        match (pins.unpin)(path) {
            Ok(()) => log::info!("unpinned {}", path.display()),
            Err(error) => log::info!("could not unpin {}: {error}", path.display()),
        }
    }
}

/// Unpins group `group_id`'s shortcuts in Grouper's Start Menu folder
/// `dir` and its pinned copies in `pins.taskbar_dir`. Returns the Start
/// Menu ones, which the caller deletes.
pub fn unpin_group(group_id: &str, dir: Option<&Path>, pins: &Pins<'_>) -> Vec<PathBuf> {
    let own = shortcuts_in(dir, group_id);
    let pinned = shortcuts_in(pins.taskbar_dir, group_id);
    unpin_each(&own, pins);
    unpin_each(&pinned, pins);
    own
}

/// Removes a deleted group's files: unpins (see [`unpin_group`]), deletes
/// its own shortcuts in `dir` (when the folder exists), then every `.ico`
/// version in `cache_dir`. Never fails; problems are logged. Both
/// `delete_group` and a folder dissolved by `remove_app_from_folder` end
/// up here.
pub fn remove_group_files(group_id: &str, cache_dir: &Path, dir: Option<&Path>, pins: &Pins<'_>) {
    for path in unpin_group(group_id, dir, pins) {
        match std::fs::remove_file(&path) {
            Ok(()) => log::info!(
                "removed deleted group {group_id}'s shortcut {}",
                path.display()
            ),
            Err(error) => log::warn!("could not remove {}: {error}", path.display()),
        }
    }
    remove_group_icons(cache_dir, group_id);
}
