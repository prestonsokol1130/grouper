//! Writes and finds a group's Start Menu shortcut without ever replacing
//! or picking another group's shortcut.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::group_icon::build_group_icon;
use super::shortcut::{
    candidate_names, description, flyout_args, group_aumid, numbered_file_name, shortcut_name_parts,
};
use crate::icons::cache::check_id;
use crate::state::AppState;
use crate::win32::com::ComGuard;
use crate::win32::lnk_aumid::read_lnk_aumid;
use crate::win32::lnk_read::resolve_lnk;
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

/// Whether a shortcut belongs to group `group_id`: its AppUserModelID
/// (`aumid_read`) is `aumid`. Only when the ID cannot be read at all are
/// its arguments (`args`, read lazily) checked for `--flyout <group_id>`.
pub fn owned_by(
    aumid_read: Result<Option<String>, String>,
    args: impl FnOnce() -> Option<String>,
    aumid: &str,
    group_id: &str,
) -> bool {
    match aumid_read {
        Ok(found) => found.as_deref() == Some(aumid),
        Err(_) => args().is_some_and(|args| args.trim() == flyout_args(group_id)),
    }
}

/// The first of `{base}.lnk`, `{base} (2).lnk`, ... (from `start`) in `dir`
/// that is not foreign: where a new shortcut for this group is written.
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

/// The first of `candidates` (file names in `dir`, see
/// [`candidate_names`]) that `owned` accepts.
pub fn find_path(
    dir: &Path,
    candidates: &[String],
    mut owned: impl FnMut(&Path) -> bool,
) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|name| dir.join(name))
        .find(|path| owned(path))
}

/// The file names in `dir`. Names that are not valid Unicode are skipped.
fn list_names(dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("list {}: {e}", dir.display()))?;
    Ok(entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect())
}

/// This group's base name, start number and the candidate names in `dir`.
fn candidates(
    state: &AppState,
    group_id: &str,
    dir: &Path,
) -> Result<(String, u32, Vec<String>), String> {
    let (base, start) = shortcut_name_parts(&state.groups, group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    let names = candidate_names(&list_names(dir)?, &base, start);
    Ok((base, start, names))
}

/// Builds group `group_id`'s icon in `cache_dir` (`{app_data_dir}/cache`,
/// see [`build_group_icon`]), then writes its shortcut into `dir` and
/// returns the shortcut's path.
///
/// The shortcut starts `exe` with `--flyout <group_id>` in `exe`'s folder,
/// shows the group's `.ico`, and carries the AppUserModelID
/// `com.grouper.group.<group_id>`. An existing shortcut of this group
/// (same AppUserModelID) is replaced atomically. Another group's shortcut,
/// or a file whose ID cannot be read, is never replaced: the next ` (n)`
/// name is used instead.
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
    let (base, start, names) = candidates(state, group_id, dir)?;
    let icon_path = build_group_icon(state, group_id, cache_dir)?;
    let aumid = group_aumid(group_id);
    // Every COM interface lives inside classify and write_group_lnk, so
    // all are released before this guard drops.
    let _com = ComGuard::new()?;
    let own = find_path(dir, &names, |path| classify(path, &aumid) == Slot::Ours);
    let lnk_path = match own {
        Some(path) => path,
        None => choose_path(dir, &base, start, |path| classify(path, &aumid))?,
    };
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

/// Group `group_id`'s own shortcut in `dir` (see [`owned_by`]): its
/// computed name when that file is its own, else any ` (n)` variant that
/// is. `None` when it has not been written yet; never another group's
/// file. Holds its own `ComGuard`. `Err` for an unknown group.
pub fn find_group_shortcut(
    state: &AppState,
    group_id: &str,
    dir: &Path,
) -> Result<Option<PathBuf>, String> {
    let (_, _, names) = candidates(state, group_id, dir)?;
    let aumid = group_aumid(group_id);
    let _com = ComGuard::new()?;
    Ok(find_path(dir, &names, |path| {
        let args = || resolve_lnk(path).ok().map(|link| link.args);
        owned_by(read_lnk_aumid(path), args, &aumid, group_id)
    }))
}
