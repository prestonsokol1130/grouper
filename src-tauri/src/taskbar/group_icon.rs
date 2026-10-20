//! Builds a group's taskbar `.ico` from its apps' cached icons.

use std::fs;
use std::path::{Path, PathBuf};

use image::{ImageFormat, RgbaImage};

use super::compose::{compose_folder, MAX_FOLDER_ICONS};
use super::ico::encode_ico;
use super::style::FOLDER_STYLE;
use crate::icons::cache::{check_id, icon_path, ICON_SIZE};
use crate::icons::fallback::fallback_icon;
use crate::state::disk_write::{reject_symlink, write_atomic};
use crate::state::paths::{GROUPS_DIR, ICONS_DIR};
use crate::state::{AppState, Group};

/// The app ids whose icons a group's folder shows: the first four of the
/// group's apps that exist and are neither missing nor hidden. Empty for
/// an unknown group.
pub fn group_icon_inputs(state: &AppState, group_id: &str) -> Vec<String> {
    state
        .group_by_id(group_id)
        .map(|group| icon_inputs_of(state, group))
        .unwrap_or_default()
}

/// [`group_icon_inputs`] for `group` itself, which need not be in `state`
/// (only `state`'s apps are looked at).
fn icon_inputs_of(state: &AppState, group: &Group) -> Vec<String> {
    group
        .app_ids
        .iter()
        .filter(|id| {
            state
                .app_by_id(id)
                .is_some_and(|app| !app.missing && !app.hidden)
        })
        .take(MAX_FOLDER_ICONS)
        .cloned()
        .collect()
}

/// Whether going from `old_group` to `new_group` changes the group's
/// folder icon: true when the first four visible apps (see
/// [`group_icon_inputs`], judged by the apps in `state`) differ, including
/// their order. Apps after the fourth visible one, and hidden or missing
/// apps, never count.
pub fn needs_icon_rebuild(old_group: &Group, new_group: &Group, state: &AppState) -> bool {
    icon_inputs_of(state, old_group) != icon_inputs_of(state, new_group)
}

/// `{groups_dir}/{group_id}-v{version}.ico`.
pub fn group_icon_path(groups_dir: &Path, group_id: &str, version: u32) -> PathBuf {
    groups_dir.join(format!("{group_id}-v{version}.ico"))
}

/// Builds the `.ico` for `group_id` and returns its path,
/// `{cache_dir}/groups/{group_id}-v{icon_version}.ico`.
///
/// `cache_dir` is `{app_data_dir}/cache` (see
/// [`cache_dir`](crate::state::paths::cache_dir)). Each app's icon is read
/// from `{cache_dir}/icons/{app_id}.png`; one that is missing or unreadable
/// is replaced by the fallback icon. The version in the file name makes
/// Windows' icon cache pick up a changed group. The file is written
/// atomically (temp file plus rename, never through a symlink), and only
/// then are the group's other versions deleted.
///
/// Returns `Err` for an unknown group, a group id that is not a safe file
/// name, or a failed encode or write.
pub fn build_group_icon(
    state: &AppState,
    group_id: &str,
    cache_dir: &Path,
) -> Result<PathBuf, String> {
    check_id(group_id).map_err(|_| format!("group id {group_id:?} is not a safe file name"))?;
    let group = state
        .group_by_id(group_id)
        .ok_or_else(|| format!("no group with id {group_id}"))?;
    let icons_dir = cache_dir.join(ICONS_DIR);
    let icons: Vec<RgbaImage> = group_icon_inputs(state, group_id)
        .iter()
        .map(|app_id| load_icon(&icons_dir, app_id))
        .collect();
    let bytes = encode_ico(&compose_folder(&icons, &FOLDER_STYLE))?;

    let groups_dir = cache_dir.join(GROUPS_DIR);
    reject_symlink(&groups_dir)?;
    fs::create_dir_all(&groups_dir).map_err(|e| format!("create {}: {e}", groups_dir.display()))?;
    let dest = group_icon_path(&groups_dir, group_id, group.icon_version);
    write_atomic(&groups_dir, &dest, &bytes)?;
    remove_versions(&groups_dir, group_id, Some(group.icon_version));
    Ok(dest)
}

/// The cached PNG for `app_id`, or the fallback icon if there is none or it
/// cannot be read (logged; never an error).
fn load_icon(icons_dir: &Path, app_id: &str) -> RgbaImage {
    if let Err(error) = check_id(app_id) {
        log::warn!("using the fallback icon: {error}");
        return fallback_icon(ICON_SIZE);
    }
    let path = icon_path(icons_dir, app_id);
    let decoded = fs::read(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))
        .and_then(|bytes| {
            image::load_from_memory_with_format(&bytes, ImageFormat::Png)
                .map_err(|e| format!("decode {}: {e}", path.display()))
        });
    match decoded {
        Ok(image) => image.into_rgba8(),
        Err(error) => {
            log::info!("using the fallback icon for app {app_id}: {error}");
            fallback_icon(ICON_SIZE)
        }
    }
}

/// Deletes every cached `.ico` of group `group_id`
/// (`{cache_dir}/groups/{group_id}-v{N}.ico`, any version). Only regular
/// files are removed; failures are logged, never returned.
pub fn remove_group_icons(cache_dir: &Path, group_id: &str) {
    if let Err(error) = check_id(group_id) {
        log::warn!("not removing icons: {error}");
        return;
    }
    let groups_dir = cache_dir.join(GROUPS_DIR);
    if groups_dir.is_dir() {
        remove_versions(&groups_dir, group_id, None);
    }
}

/// Deletes every `{group_id}-v{N}.ico` in `groups_dir` other than version
/// `keep` (all of them for `None`). Only regular files are removed;
/// failures are logged.
fn remove_versions(groups_dir: &Path, group_id: &str, keep: Option<u32>) {
    let entries = match fs::read_dir(groups_dir) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!("could not list {}: {error}", groups_dir.display());
            return;
        }
    };
    let prefix = format!("{group_id}-v");
    for entry in entries.flatten() {
        let name = entry.file_name();
        let version = name
            .to_str()
            .and_then(|name| name.strip_prefix(&prefix))
            .and_then(|rest| rest.strip_suffix(".ico"))
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|digits| digits.parse::<u32>().ok());
        let stale = version.is_some_and(|version| Some(version) != keep);
        if stale && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            if let Err(error) = fs::remove_file(entry.path()) {
                log::warn!("could not remove {}: {error}", entry.path().display());
            }
        }
    }
}
