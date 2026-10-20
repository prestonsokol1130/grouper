//! Keeps a group's shortcut and `.ico` in step with the group: rewrite on
//! a rename or an icon change, repair when files are missing, and clean up
//! when the group is deleted.
//!
//! Every function here blocks and holds its own `ComGuard` where it reads
//! or writes shortcuts; run them on a background thread.

use std::path::{Path, PathBuf};

use super::group_icon::{group_icon_path, remove_group_icons};
use super::shortcut::group_aumid;
use super::shortcut_write::{
    classify, find_group_shortcut, write_group_shortcut, write_shortcut_with_icon, Slot,
};
use crate::icons::cache::check_id;
use crate::state::paths::GROUPS_DIR;
use crate::state::AppState;
use crate::win32::com::ComGuard;

/// What [`repair_group_shortcut`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair {
    /// The shortcut and its `.ico` were both there; nothing was written.
    Intact(PathBuf),
    /// Something was missing, so both were written again.
    Rebuilt(PathBuf),
}

/// What changed about a group that has a shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    /// The name changed: the old shortcut is deleted and one with the new
    /// name is written.
    pub renamed: bool,
    /// The folder icon changed: a new `.ico` version is built.
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
/// read, is never touched). Failures are logged. Returns what was deleted.
pub fn remove_own_shortcuts(dir: &Path, group_id: &str) -> Vec<PathBuf> {
    let own = match own_shortcuts(dir, group_id) {
        Ok(own) => own,
        Err(error) => {
            log::warn!("could not look for group {group_id}'s shortcuts: {error}");
            return Vec::new();
        }
    };
    own.into_iter()
        .filter(|path| match std::fs::remove_file(path) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("could not remove {}: {error}", path.display());
                false
            }
        })
        .collect()
}

/// Brings group `group_id`'s shortcut up to date after `change` and returns
/// its path.
///
/// On a rename the group's own old shortcut is deleted first, then one
/// with the new name is written (same AppUserModelID). On an icon change
/// the `.ico` for the group's current `icon_version` is built (older
/// versions are deleted) and the shortcut rewritten to show it. When only
/// the name changed the current `.ico` is reused, and built only if it is
/// missing.
pub fn update_group_shortcut(
    state: &AppState,
    group_id: &str,
    change: Change,
    cache_dir: &Path,
    dir: &Path,
    exe: &Path,
) -> Result<PathBuf, String> {
    check_id(group_id).map_err(|_| format!("group id {group_id:?} is not a safe file name"))?;
    let group = state
        .group_by_id(group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    if change.renamed {
        for path in remove_own_shortcuts(dir, group_id) {
            log::info!("removed group {group_id}'s old shortcut {}", path.display());
        }
    }
    let icon = group_icon_path(&cache_dir.join(GROUPS_DIR), group_id, group.icon_version);
    if change.icon || !icon.is_file() {
        write_group_shortcut(state, group_id, cache_dir, dir, exe)
    } else {
        write_shortcut_with_icon(state, group_id, &icon, dir, exe)
    }
}

/// Checks group `group_id`'s shortcut and its current `.ico`; when either
/// is missing, builds the `.ico` and writes the shortcut again.
pub fn repair_group_shortcut(
    state: &AppState,
    group_id: &str,
    cache_dir: &Path,
    dir: &Path,
    exe: &Path,
) -> Result<Repair, String> {
    let group = state
        .group_by_id(group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    let icon = group_icon_path(&cache_dir.join(GROUPS_DIR), group_id, group.icon_version);
    match find_group_shortcut(state, group_id, dir)? {
        Some(lnk) if icon.is_file() => Ok(Repair::Intact(lnk)),
        _ => write_group_shortcut(state, group_id, cache_dir, dir, exe).map(Repair::Rebuilt),
    }
}

/// Removes a deleted group's files: its own shortcuts in `dir` (when the
/// folder exists) and every cached `.ico` version in `cache_dir`. Never
/// fails; problems are logged.
pub fn remove_group_files(group_id: &str, cache_dir: &Path, dir: Option<&Path>) {
    if let Some(dir) = dir.filter(|dir| dir.is_dir()) {
        for path in remove_own_shortcuts(dir, group_id) {
            log::info!(
                "removed deleted group {group_id}'s shortcut {}",
                path.display()
            );
        }
    }
    remove_group_icons(cache_dir, group_id);
}
