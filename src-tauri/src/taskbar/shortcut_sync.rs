//! Keeps a group's shortcut and `.ico` in step with the group: rewrite on
//! a rename or an icon change, repair when files are missing, and clean up
//! when the group is deleted.
//!
//! Every function here blocks and holds its own `ComGuard` where it reads
//! or writes shortcuts; run them on a background thread.

use std::path::{Path, PathBuf};

use super::group_icon::{group_icon_path, remove_group_icons};
use super::shortcut::{group_aumid, shortcut_name_parts};
use super::shortcut_write::{
    classify, find_group_shortcut, write_group_shortcut, write_shortcut_with_icon, Slot,
};
use crate::icons::cache::check_id;
use crate::state::paths::GROUPS_DIR;
use crate::state::AppState;
use crate::win32::com::ComGuard;
use crate::win32::lnk_read::resolve_lnk;

/// What [`repair_group_shortcut`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair {
    /// The shortcut and its `.ico` were both there; nothing was written.
    Intact(PathBuf),
    /// Something was missing or stale, so the shortcut was written again.
    Rebuilt(PathBuf),
}

/// What changed about a group that has a shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    /// The name changed: a shortcut with the new name is written, then the
    /// old one is deleted.
    pub renamed: bool,
    /// The folder icon changed: the `.ico` is rebuilt in place.
    pub icon: bool,
}

/// The `.lnk` files in `dir` whose AppUserModelID is group `group_id`'s.
/// Files whose ID cannot be read are never included. Holds its own
/// `ComGuard`. An unlistable folder is logged and gives no files.
pub fn own_shortcuts(dir: &Path, group_id: &str) -> Result<Vec<PathBuf>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!("could not list {}: {error}", dir.display());
            return Ok(Vec::new());
        }
    };
    let aumid = group_aumid(group_id);
    let _com = ComGuard::new()?;
    Ok(entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
        })
        .filter(|path| classify(path, &aumid) == Slot::Ours)
        .collect())
}

/// Deletes every shortcut in `dir` that belongs to group `group_id` (by
/// its AppUserModelID; another group's file, or one whose ID cannot be
/// read, is never touched), except `keep` (its file name compared ignoring
/// letter case, as Windows does). Failures are logged. Returns what was
/// deleted.
pub fn remove_own_shortcuts(dir: &Path, group_id: &str, keep: Option<&Path>) -> Vec<PathBuf> {
    let own = match own_shortcuts(dir, group_id) {
        Ok(own) => own,
        Err(error) => {
            log::warn!("could not look for group {group_id}'s shortcuts: {error}");
            return Vec::new();
        }
    };
    let name = |path: &Path| path.file_name().map(|n| n.to_string_lossy().to_lowercase());
    let kept = keep.and_then(name);
    own.into_iter()
        .filter(|path| kept.is_none() || name(path) != kept)
        .filter(|path| match std::fs::remove_file(path) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("could not remove {}: {error}", path.display());
                false
            }
        })
        .collect()
}

/// Group `group_id`'s `.ico`, `{cache_dir}/groups/{group_id}.ico`.
pub fn icon_file(cache_dir: &Path, group_id: &str) -> PathBuf {
    group_icon_path(&cache_dir.join(GROUPS_DIR), group_id)
}

/// Brings group `group_id`'s shortcut up to date after `change` and returns
/// its path.
///
/// On an icon change the group's `.ico` is rebuilt in place and the
/// shortcut rewritten to show it; otherwise the `.ico` is reused, and
/// built only if it is missing. On a rename the shortcut with the new name
/// is written first (same AppUserModelID); only when that worked are the
/// group's other own shortcuts deleted. A failed write leaves every
/// existing shortcut as it was.
pub fn update_group_shortcut(
    state: &AppState,
    group_id: &str,
    change: Change,
    cache_dir: &Path,
    dir: &Path,
    exe: &Path,
) -> Result<PathBuf, String> {
    check_id(group_id).map_err(|_| format!("group id {group_id:?} is not a safe file name"))?;
    let (base, _) = shortcut_name_parts(&state.groups, group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    let icon = icon_file(cache_dir, group_id);
    let lnk = if change.icon || !icon.is_file() {
        write_group_shortcut(state, group_id, cache_dir, dir, exe)?
    } else {
        write_shortcut_with_icon(state, group_id, &icon, dir, exe)?
    };
    if !change.renamed {
        return Ok(lnk);
    }
    let lnk = match_name_case(lnk, &base);
    for path in remove_own_shortcuts(dir, group_id, Some(&lnk)) {
        log::info!("removed group {group_id}'s old shortcut {}", path.display());
    }
    Ok(lnk)
}

/// After a rename that only changed letter case, the writer reuses the old
/// file (Windows file names ignore case). This renames it so its name
/// starts with `base` exactly. Returns the path the shortcut ends up at;
/// a failed rename is logged and keeps the old name.
fn match_name_case(lnk: PathBuf, base: &str) -> PathBuf {
    let Some(name) = lnk.file_name().and_then(|name| name.to_str()) else {
        return lnk;
    };
    let fixed = match name.get(..base.len()) {
        Some(head) if head != base && head.to_lowercase() == base.to_lowercase() => {
            format!("{base}{}", &name[base.len()..])
        }
        _ => return lnk,
    };
    let target = lnk.with_file_name(fixed);
    match std::fs::rename(&lnk, &target) {
        Ok(()) => target,
        Err(error) => {
            log::warn!("could not rename {}: {error}", lnk.display());
            lnk
        }
    }
}

/// Checks group `group_id`'s shortcut and its `.ico`. When either is
/// missing, or the shortcut shows another icon file (such as a legacy
/// `{group_id}-v{N}.ico`), the shortcut is written again showing
/// `{group_id}.ico` (built first when missing).
pub fn repair_group_shortcut(
    state: &AppState,
    group_id: &str,
    cache_dir: &Path,
    dir: &Path,
    exe: &Path,
) -> Result<Repair, String> {
    if state.group_by_id(group_id).is_none() {
        return Err(format!("Group not found: {group_id}"));
    }
    let icon = icon_file(cache_dir, group_id);
    let lnk = find_group_shortcut(state, group_id, dir)?;
    match lnk {
        Some(lnk) if icon.is_file() && shows_icon(&lnk, &icon)? => Ok(Repair::Intact(lnk)),
        Some(_) if icon.is_file() => {
            write_shortcut_with_icon(state, group_id, &icon, dir, exe).map(Repair::Rebuilt)
        }
        _ => write_group_shortcut(state, group_id, cache_dir, dir, exe).map(Repair::Rebuilt),
    }
}

/// Whether the shortcut at `lnk` shows the icon file `icon` (paths compared
/// ignoring letter case). An unreadable shortcut counts as not showing it.
/// Holds its own `ComGuard`.
fn shows_icon(lnk: &Path, icon: &Path) -> Result<bool, String> {
    let _com = ComGuard::new()?;
    let wanted = icon.to_string_lossy().to_lowercase();
    Ok(resolve_lnk(lnk)
        .ok()
        .and_then(|link| link.icon_location)
        .is_some_and(|(path, _)| path.to_lowercase() == wanted))
}

/// Removes a deleted group's files: its own shortcuts in `dir` (when the
/// folder exists) and its `.ico` (plus legacy versions) in `cache_dir`. Never
/// fails; problems are logged.
pub fn remove_group_files(group_id: &str, cache_dir: &Path, dir: Option<&Path>) {
    if let Some(dir) = dir.filter(|dir| dir.is_dir()) {
        for path in remove_own_shortcuts(dir, group_id, None) {
            log::info!(
                "removed deleted group {group_id}'s shortcut {}",
                path.display()
            );
        }
    }
    remove_group_icons(cache_dir, group_id);
}
