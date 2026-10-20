//! Group shortcuts in the Start Menu: their folder, file names and
//! AppUserModelIDs, and writing one for a group.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::group_icon::build_group_icon;
use crate::icons::cache::check_id;
use crate::scanner::roots::programs_dir;
use crate::scanner::walk::OWN_FOLDER;
use crate::state::disk_write::reject_symlink;
use crate::state::{AppState, Group};
use crate::win32::com::ComGuard;
use crate::win32::lnk_write::{write_group_lnk, LnkSpec};

/// Every group's AppUserModelID is this plus the group id.
pub const AUMID_PREFIX: &str = "com.grouper.group.";

/// Characters Windows does not allow in a file name.
const INVALID_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Longest shortcut name kept from a group name, in characters, so the
/// full Start Menu path stays well under `MAX_PATH`.
pub const MAX_NAME_CHARS: usize = 80;

/// Longest shortcut description in UTF-16 units (`INFOTIPSIZE` minus NUL).
const MAX_DESCRIPTION_UNITS: usize = 1023;

/// Device names Windows reserves, with or without an extension.
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// `com.grouper.group.<group_id>`.
pub fn group_aumid(group_id: &str) -> String {
    format!("{AUMID_PREFIX}{group_id}")
}

/// The shortcut's arguments: `--flyout <group_id>`.
pub fn flyout_args(group_id: &str) -> String {
    format!("--flyout {group_id}")
}

/// A file name (without `.lnk`) for a group called `name`: the characters
/// `<>:"/\|?*` and control characters are removed, trailing dots and spaces
/// are stripped, and the result is cut to [`MAX_NAME_CHARS`]. Falls back to
/// `group_id` when nothing is left or the result is a reserved device name
/// such as `CON`.
pub fn sanitise_name(name: &str, group_id: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !INVALID_CHARS.contains(c) && !c.is_control())
        .collect();
    let cut: String = strip_trailing(&kept).chars().take(MAX_NAME_CHARS).collect();
    let cleaned = strip_trailing(&cut);
    if cleaned.is_empty() || is_reserved(cleaned) {
        group_id.to_owned()
    } else {
        cleaned.to_owned()
    }
}

/// `text` without trailing dots and spaces.
fn strip_trailing(text: &str) -> &str {
    text.trim_end_matches(['.', ' '])
}

/// True when the part before the first dot is a reserved device name.
fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    RESERVED_NAMES
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// The `.lnk` file name for group `group_id`, or `None` for an unknown
/// group.
///
/// Names are given out in the order of `groups`: the first group with a
/// name gets it as is, and later groups whose name is already taken (any
/// letter case) get ` (2)`, ` (3)` and so on.
pub fn shortcut_file_name(groups: &[Group], group_id: &str) -> Option<String> {
    let mut taken = HashSet::new();
    for group in groups {
        let base = sanitise_name(&group.name, &group.id);
        let mut name = base.clone();
        let mut next = 2_u32;
        while taken.contains(&name.to_lowercase()) {
            name = format!("{base} ({next})");
            next = next.saturating_add(1);
        }
        taken.insert(name.to_lowercase());
        if group.id == group_id {
            return Some(format!("{name}.lnk"));
        }
    }
    None
}

/// `{dir}/<shortcut file name>` for group `group_id` (see
/// [`shortcut_file_name`]). `Err` for an unknown group.
pub fn shortcut_path(dir: &Path, state: &AppState, group_id: &str) -> Result<PathBuf, String> {
    shortcut_file_name(&state.groups, group_id)
        .map(|name| dir.join(name))
        .ok_or_else(|| format!("Group not found: {group_id}"))
}

/// Grouper's Start Menu folder, `{FOLDERID_Programs}\Grouper`, created if
/// it is missing. The app scanner skips this folder, so these shortcuts
/// never come back as apps. `Err` when it is a symlink or cannot be made.
pub fn shortcut_dir() -> Result<PathBuf, String> {
    let dir = programs_dir()?.join(OWN_FOLDER);
    reject_symlink(&dir)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// The shortcut's description: the group name, cut to fit `INFOTIPSIZE`.
fn description(name: &str) -> String {
    let mut units = 0;
    name.chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= MAX_DESCRIPTION_UNITS
        })
        .collect()
}

/// Builds group `group_id`'s icon in `cache_dir` (`{app_data_dir}/cache`,
/// see [`build_group_icon`]), then writes its shortcut into `dir` and
/// returns the shortcut's path.
///
/// The shortcut starts `exe` with `--flyout <group_id>` in `exe`'s folder,
/// shows the group's `.ico`, and carries the AppUserModelID
/// `com.grouper.group.<group_id>`. An existing shortcut at that path is
/// replaced atomically.
///
/// Holds its own `ComGuard` while writing. Blocks; call it off the async
/// runtime. `Err` for an unknown group, an unsafe group id, or a failed
/// icon or shortcut write.
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
    let lnk_path = shortcut_path(dir, state, group_id)?;
    let icon_path = build_group_icon(state, group_id, cache_dir)?;
    let spec = LnkSpec {
        lnk_path: lnk_path.clone(),
        exe_path: exe.to_path_buf(),
        args: flyout_args(group_id),
        icon_path,
        description: description(&group.name),
        aumid: group_aumid(group_id),
    };
    // Every COM interface lives inside write_group_lnk, so all are
    // released before this guard drops.
    let _com = ComGuard::new()?;
    write_group_lnk(&spec)?;
    Ok(lnk_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aumid_and_args_use_the_group_id() {
        assert_eq!(group_aumid("aB3_-x"), "com.grouper.group.aB3_-x");
        assert_eq!(flyout_args("aB3_-x"), "--flyout aB3_-x");
    }

    #[test]
    fn description_is_the_name_cut_to_infotipsize() {
        assert_eq!(description("Work apps"), "Work apps");
        let long = "x".repeat(2000);
        assert_eq!(description(&long).len(), MAX_DESCRIPTION_UNITS);
        // A surrogate pair is never split.
        let emoji = "📁".repeat(600);
        assert_eq!(description(&emoji).chars().count(), 511);
    }
}
