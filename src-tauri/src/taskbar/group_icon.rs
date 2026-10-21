//! Builds a group's taskbar `.ico` from its apps' cached icons.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use image::{ImageFormat, RgbaImage};

use super::compose::compose_folder;
use super::group_icon_files::{group_icon_path, group_icons_dir, latest_group_icon};
use super::ico::encode_ico;
use super::icon_inputs::group_icon_inputs;
use super::style::folder_style;
use crate::icons::cache::{check_id, icon_path, ICON_SIZE};
use crate::icons::fallback::fallback_icon;
use crate::state::disk_write::{reject_symlink, write_atomic};
use crate::state::paths::ICONS_DIR;
use crate::state::AppState;

/// How often a write that another process (usually the indexer or an
/// antivirus scan) briefly blocks is tried.
const WRITE_TRIES: u32 = 3;

/// The pause between those tries.
const RETRY_PAUSE: Duration = Duration::from_millis(150);

/// Builds the `.ico` for `group_id` and returns its path, a new
/// `{group_icons_dir}/{group_id}-v{N}.ico` one version past the newest
/// (see [`super::group_icon_files`] for why a path is never rewritten).
///
/// `cache_dir` is `{app_data_dir}/cache` (see
/// [`cache_dir`](crate::state::paths::cache_dir)). Each app's icon is read
/// from `{cache_dir}/icons/{app_id}.png`; one that is missing or unreadable
/// is replaced by the fallback icon. The file is written atomically (temp
/// file plus rename, never through a symlink); nothing is deleted.
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
    let bytes = encode_ico(&compose_folder(
        &icons,
        &folder_style(state.settings.folder_shape),
    ))?;

    let groups_dir = group_icons_dir(cache_dir);
    reject_symlink(&groups_dir)?;
    fs::create_dir_all(&groups_dir).map_err(|e| format!("create {}: {e}", groups_dir.display()))?;
    let next = latest_group_icon(&groups_dir, group_id).map_or(1, |(n, _)| n.saturating_add(1));
    let dest = group_icon_path(&groups_dir, group_id, next);
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
