//! Writes and finds a group's Start Menu shortcut without ever replacing
//! another group's shortcut.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::group_icon::build_group_icon;
use super::shortcut::{
    description, flyout_args, group_aumid, numbered_file_name, shortcut_name_parts,
};
use crate::icons::cache::check_id;
use crate::state::AppState;
use crate::win32::com::ComGuard;
use crate::win32::lnk_aumid::read_lnk_aumid;
use crate::win32::lnk_write::{write_group_lnk, LnkSpec};

/// How many numbered names are tried before giving up.
const MAX_TRIES: u32 = 1000;

/// What is at one candidate shortcut path, for one group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Nothing is there.
    Missing,
    /// A shortcut with this group's AppUserModelID.
    Ours,
    /// A file with no AppUserModelID (or an empty one): may be replaced.
    Unclaimed,
    /// Another group's shortcut, or a file whose ID cannot be read: never
    /// replaced.
    Foreign,
}

/// What is at `path` for the group with `aumid`. The caller's thread must
/// hold a `ComGuard`. Unreadable files are logged and count as foreign.
pub fn classify(path: &Path, aumid: &str) -> Slot {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Slot::Missing,
        Err(error) => {
            log::warn!("not replacing {}: {error}", path.display());
            return Slot::Foreign;
        }
        Ok(meta) if !meta.file_type().is_file() => return Slot::Foreign,
        Ok(_) => {}
    }
    match read_lnk_aumid(path) {
        Ok(Some(found)) if found == aumid => Slot::Ours,
        Ok(Some(_)) => Slot::Foreign,
        Ok(None) => Slot::Unclaimed,
        Err(error) => {
            log::warn!("not replacing {}: {error}", path.display());
            Slot::Foreign
        }
    }
}

/// The first of `{base}.lnk`, `{base} (2).lnk`, ... (from `start`) in `dir`
/// that is not foreign: where this group's shortcut is written.
pub fn choose_path(
    dir: &Path,
    base: &str,
    start: u32,
    mut slot: impl FnMut(&Path) -> Slot,
) -> Result<PathBuf, String> {
    for number in start..start.saturating_add(MAX_TRIES) {
        let path = dir.join(numbered_file_name(base, number));
        if slot(&path) != Slot::Foreign {
            return Ok(path);
        }
    }
    Err(format!(
        "no free shortcut name for {base:?} in {}",
        dir.display()
    ))
}

/// This group's existing shortcut: the first path from `start` that is
/// ours, skipping other groups' files and stopping at a missing one.
pub fn find_path(
    dir: &Path,
    base: &str,
    start: u32,
    mut slot: impl FnMut(&Path) -> Slot,
) -> Option<PathBuf> {
    for number in start..start.saturating_add(MAX_TRIES) {
        let path = dir.join(numbered_file_name(base, number));
        match slot(&path) {
            Slot::Ours => return Some(path),
            Slot::Missing => return None,
            Slot::Unclaimed | Slot::Foreign => {}
        }
    }
    None
}

/// The base name and start number of group `group_id`'s shortcut.
fn name_parts(state: &AppState, group_id: &str) -> Result<(String, u32), String> {
    shortcut_name_parts(&state.groups, group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))
}

/// Builds group `group_id`'s icon in `cache_dir` (`{app_data_dir}/cache`,
/// see [`build_group_icon`]), then writes its shortcut into `dir` and
/// returns the shortcut's path.
///
/// The shortcut starts `exe` with `--flyout <group_id>` in `exe`'s folder,
/// shows the group's `.ico`, and carries the AppUserModelID
/// `com.grouper.group.<group_id>`. It replaces this group's own shortcut
/// atomically, and never another group's: when the name is held by a
/// different AppUserModelID (or one that cannot be read), the next
/// ` (n)` name is used.
///
/// Holds its own `ComGuard` while reading and writing shortcuts. Blocks;
/// call it off the async runtime. `Err` for an unknown group, an unsafe
/// group id, or a failed icon or shortcut write.
pub fn write_group_shortcut(
    state: &AppState,
    group_id: &str,
    cache_dir: &Path,
    dir: &Path,
    exe: &Path,
) -> Result<PathBuf, String> {
    check_id(group_id).map_err(|_| format!("group id {group_id:?} is not a safe file name"))?;
    let group = state
        .group_by_id(group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    let (base, start) = name_parts(state, group_id)?;
    let icon_path = build_group_icon(state, group_id, cache_dir)?;
    let aumid = group_aumid(group_id);
    // Every COM interface lives inside classify and write_group_lnk, so
    // all are released before this guard drops.
    let _com = ComGuard::new()?;
    let lnk_path = choose_path(dir, &base, start, |path| classify(path, &aumid))?;
    write_group_lnk(&LnkSpec {
        lnk_path: lnk_path.clone(),
        exe_path: exe.to_path_buf(),
        args: flyout_args(group_id),
        icon_path,
        description: description(&group.name),
        aumid,
    })?;
    Ok(lnk_path)
}

/// Group `group_id`'s shortcut in `dir`, or `None` when it has not been
/// written yet. Holds its own `ComGuard`. `Err` for an unknown group.
pub fn find_group_shortcut(
    state: &AppState,
    group_id: &str,
    dir: &Path,
) -> Result<Option<PathBuf>, String> {
    let (base, start) = name_parts(state, group_id)?;
    let aumid = group_aumid(group_id);
    let _com = ComGuard::new()?;
    Ok(find_path(dir, &base, start, |path| classify(path, &aumid)))
}
