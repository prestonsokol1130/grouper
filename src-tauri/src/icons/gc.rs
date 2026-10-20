//! Startup clean-up of the icon caches: PNGs of apps no longer in the app
//! state, and `.ico` files of deleted groups.
//!
//! Safe by construction: it only lists Grouper's own two cache folders
//! (never recursing), skips them when they are symlinks, and deletes only
//! regular files (never a symlink) whose names match what Grouper writes:
//! `{app_id}.png` (16 lowercase hex digits), `{group_id}.ico` and
//! `{group_id}-v{N}.ico`. Anything else is left alone.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::cache::check_id;
use crate::state::paths::{group_cache_dir, icon_cache_dir};
use crate::state::AppState;

/// How many files [`collect`] deleted from each folder.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Collected {
    pub pngs: usize,
    pub icos: usize,
}

/// Deletes stale cache files under `{app_data_dir}/cache`, keeping
/// everything `state` still uses. Failures are logged and skipped.
pub fn collect(app_data_dir: &Path, state: &AppState) -> Collected {
    let apps: HashSet<&str> = state.apps.iter().map(|app| app.id.as_str()).collect();
    let groups: HashSet<&str> = state.groups.iter().map(|g| g.id.as_str()).collect();
    Collected {
        pngs: sweep(&icon_cache_dir(app_data_dir), |name| stale_png(name, &apps)),
        icos: sweep(&group_cache_dir(app_data_dir), |name| {
            stale_ico(name, &groups)
        }),
    }
}

/// True for `{app_id}.png` (an id as [`crate::state::ids::app_id`] makes
/// it) whose app is not in `apps`.
fn stale_png(name: &str, apps: &HashSet<&str>) -> bool {
    let Some(stem) = name.strip_suffix(".png") else {
        return false;
    };
    let is_app_id = stem.len() == 16
        && stem
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    is_app_id && !apps.contains(stem)
}

/// True for `{group_id}.ico` or `{group_id}-v{N}.ico` whose group is not
/// in `groups`. A name that could be either keeps the file when either
/// reading names a group.
fn stale_ico(name: &str, groups: &HashSet<&str>) -> bool {
    let Some(stem) = name.strip_suffix(".ico") else {
        return false;
    };
    if check_id(stem).is_err() {
        return false;
    }
    let versioned = stem.rsplit_once("-v").and_then(|(id, digits)| {
        let digits_ok = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
        (digits_ok && check_id(id).is_ok()).then_some(id)
    });
    !groups.contains(stem) && versioned.is_none_or(|id| !groups.contains(id))
}

/// Deletes each regular file directly in `dir` whose UTF-8 name `stale`
/// accepts. Returns how many went. A missing or symlinked folder is left
/// alone.
fn sweep(dir: &Path, stale: impl Fn(&str) -> bool) -> usize {
    match fs::symlink_metadata(dir) {
        Ok(meta) if meta.file_type().is_dir() => {}
        Ok(_) => {
            log::warn!("not cleaning {}: not a plain folder", dir.display());
            return 0;
        }
        Err(_) => return 0,
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!("could not list {}: {error}", dir.display());
            return 0;
        }
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|name| stale(name)) else {
            continue;
        };
        // DirEntry::file_type does not follow symlinks.
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        match fs::remove_file(entry.path()) {
            Ok(()) => removed += 1,
            Err(error) => log::warn!("could not remove stale {name}: {error}"),
        }
    }
    removed
}

#[cfg(test)]
#[path = "gc_tests.rs"]
mod tests;
