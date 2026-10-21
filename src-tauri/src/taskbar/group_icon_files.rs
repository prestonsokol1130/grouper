//! Where a group's `.ico` files live, which one is current, and removing
//! them all when the group is deleted.
//!
//! Every build writes a new `{group_id}-v{N}.ico` (N one past the newest)
//! and never rewrites an existing file. Explorer and the taskbar cache a
//! shortcut's icon by its path: a file rewritten in place kept showing a
//! stale blank icon, while a new path is always read fresh. Older files
//! are kept while the group exists, because taskbar pins made earlier
//! still point at them (a pin must never point at a missing file). This
//! replaces the stable `{group_id}.ico` path, per Preston, 2026-10-20;
//! such a file is likewise kept until the group is deleted.

use std::fs;
use std::path::{Path, PathBuf};

use crate::icons::cache::check_id;
use crate::state::paths::GROUPS_DIR;

/// `{groups_dir}/{group_id}-v{version}.ico`.
pub fn group_icon_path(groups_dir: &Path, group_id: &str, version: u32) -> PathBuf {
    groups_dir.join(format!("{group_id}-v{version}.ico"))
}

/// `{groups_dir}/{group_id}.ico`, the unversioned path older builds wrote.
pub fn stable_icon_path(groups_dir: &Path, group_id: &str) -> PathBuf {
    groups_dir.join(format!("{group_id}.ico"))
}

/// The `N` of `{group_id}-v{N}.ico` when `name` is one of the group's
/// versioned icons.
pub fn icon_version(name: &str, group_id: &str) -> Option<u32> {
    let digits = name
        .strip_prefix(group_id)?
        .strip_prefix("-v")?
        .strip_suffix(".ico")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The group's versioned icons in `groups_dir` (regular files only), as
/// (version, path).
fn versioned_icons(groups_dir: &Path, group_id: &str) -> Vec<(u32, PathBuf)> {
    let Ok(entries) = fs::read_dir(groups_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name();
            let version = icon_version(name.to_str()?, group_id)?;
            Some((version, entry.path()))
        })
        .collect()
}

/// The group's newest versioned icon in `groups_dir`, if it has one.
pub fn latest_group_icon(groups_dir: &Path, group_id: &str) -> Option<(u32, PathBuf)> {
    versioned_icons(groups_dir, group_id)
        .into_iter()
        .max_by_key(|(version, _)| *version)
}

/// The group's current icon, `{cache_dir}/groups/{group_id}-v{N}.ico` with
/// the highest `N`, if any was built.
pub fn current_icon(cache_dir: &Path, group_id: &str) -> Option<PathBuf> {
    latest_group_icon(&cache_dir.join(GROUPS_DIR), group_id).map(|(_, path)| path)
}

/// Deletes every cached `.ico` of group `group_id`: each
/// `{group_id}-v{N}.ico` and the old `{group_id}.ico` in
/// `{cache_dir}/groups`. Called only when the group is deleted, after its
/// pins were unpinned. Only regular files are removed; an unsafe id or a
/// missing folder touches nothing; failures are logged, never returned.
pub fn remove_group_icons(cache_dir: &Path, group_id: &str) {
    if let Err(error) = check_id(group_id) {
        log::warn!("not removing icons: {error}");
        return;
    }
    let groups_dir = cache_dir.join(GROUPS_DIR);
    let stable = stable_icon_path(&groups_dir, group_id);
    let stable = fs::symlink_metadata(&stable)
        .is_ok_and(|meta| meta.is_file())
        .then_some(stable);
    let paths = versioned_icons(&groups_dir, group_id)
        .into_iter()
        .map(|(_, path)| path)
        .chain(stable);
    for path in paths {
        if let Err(error) = fs::remove_file(&path) {
            log::warn!("could not remove {}: {error}", path.display());
        }
    }
}
