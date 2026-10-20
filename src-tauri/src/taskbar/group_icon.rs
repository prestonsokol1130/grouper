//! Builds a group's taskbar `.ico` from its apps' cached icons.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use image::{ImageFormat, RgbaImage};

use super::compose::compose_folder;
use super::ico::encode_ico;
use super::icon_inputs::group_icon_inputs;
use super::style::FOLDER_STYLE;
use crate::icons::cache::{check_id, icon_path, ICON_SIZE};
use crate::icons::fallback::fallback_icon;
use crate::state::disk_write::{reject_symlink, write_atomic};
use crate::state::paths::{GROUPS_DIR, ICONS_DIR};
use crate::state::AppState;

/// How often a write over an `.ico` that another process (usually
/// Explorer reading the icon) briefly holds open is tried.
const WRITE_TRIES: u32 = 3;

/// The pause between those tries.
const RETRY_PAUSE: Duration = Duration::from_millis(150);

/// `{groups_dir}/{group_id}.ico`. The path never changes, so a taskbar pin
/// (a copy of the group's shortcut) always points at a file that exists.
pub fn group_icon_path(groups_dir: &Path, group_id: &str) -> PathBuf {
    groups_dir.join(format!("{group_id}.ico"))
}

/// Builds the `.ico` for `group_id` and returns its path,
/// `{cache_dir}/groups/{group_id}.ico`.
///
/// `cache_dir` is `{app_data_dir}/cache` (see
/// [`cache_dir`](crate::state::paths::cache_dir)). Each app's icon is read
/// from `{cache_dir}/icons/{app_id}.png`; one that is missing or unreadable
/// is replaced by the fallback icon. The file is written atomically over
/// the previous one (temp file plus rename, never through a symlink), so
/// the path stays valid the whole time; nothing is deleted.
///
/// Returns `Err` for an unknown group, a group id that is not a safe file
/// name, or a failed encode or write.
pub fn build_group_icon(
    state: &AppState,
    group_id: &str,
    cache_dir: &Path,
) -> Result<PathBuf, String> {
    check_id(group_id).map_err(|_| format!("group id {group_id:?} is not a safe file name"))?;
    if state.group_by_id(group_id).is_none() {
        return Err(format!("no group with id {group_id}"));
    }
    let icons_dir = cache_dir.join(ICONS_DIR);
    let icons: Vec<RgbaImage> = group_icon_inputs(state, group_id)
        .iter()
        .map(|app_id| load_icon(&icons_dir, app_id))
        .collect();
    let bytes = encode_ico(&compose_folder(&icons, &FOLDER_STYLE))?;

    let groups_dir = cache_dir.join(GROUPS_DIR);
    reject_symlink(&groups_dir)?;
    fs::create_dir_all(&groups_dir).map_err(|e| format!("create {}: {e}", groups_dir.display()))?;
    let dest = group_icon_path(&groups_dir, group_id);
    let mut result = write_atomic(&groups_dir, &dest, &bytes);
    for _ in 1..WRITE_TRIES {
        if result.is_ok() {
            break;
        }
        std::thread::sleep(RETRY_PAUSE);
        result = write_atomic(&groups_dir, &dest, &bytes);
    }
    result.map(|()| dest)
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

/// Deletes every cached `.ico` of group `group_id`: `{group_id}.ico` and
/// any legacy `{group_id}-v{N}.ico` in `{cache_dir}/groups`. Called only
/// when the group is deleted: until then an old taskbar pin may still
/// show a legacy file, so nothing else removes them. Only regular files
/// are removed; failures are logged, never returned.
pub fn remove_group_icons(cache_dir: &Path, group_id: &str) {
    if remove_legacy_icons(cache_dir, group_id) {
        let path = group_icon_path(&cache_dir.join(GROUPS_DIR), group_id);
        if path.is_file() {
            if let Err(error) = fs::remove_file(&path) {
                log::warn!("could not remove {}: {error}", path.display());
            }
        }
    }
}

/// Deletes group `group_id`'s legacy `{group_id}-v{N}.ico` files (named
/// by version before the icon path became stable). Only regular files are
/// removed; failures are logged. False for an unsafe group id or a missing
/// folder, where nothing is touched.
fn remove_legacy_icons(cache_dir: &Path, group_id: &str) -> bool {
    if let Err(error) = check_id(group_id) {
        log::warn!("not removing icons: {error}");
        return false;
    }
    let groups_dir = cache_dir.join(GROUPS_DIR);
    let entries = match fs::read_dir(&groups_dir) {
        Ok(entries) => entries,
        Err(error) => {
            log::debug!("could not list {}: {error}", groups_dir.display());
            return false;
        }
    };
    let prefix = format!("{group_id}-v");
    for entry in entries.flatten() {
        let name = entry.file_name();
        let legacy = name
            .to_str()
            .and_then(|name| name.strip_prefix(&prefix))
            .and_then(|rest| rest.strip_suffix(".ico"))
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        if legacy && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            if let Err(error) = fs::remove_file(entry.path()) {
                log::warn!("could not remove {}: {error}", entry.path().display());
            }
        }
    }
    true
}
