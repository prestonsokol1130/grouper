//! Removing Grouper's Start Menu folder and, when asked, its app data.
//! Both refuse symlinks and junctions, so nothing outside Grouper's own
//! folders is ever deleted.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::scanner::walk::OWN_FOLDER;
use crate::taskbar::group_icon_files::GROUP_ICONS_FOLDER;

/// What a removal did.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The folder is gone.
    Removed,
    /// There was nothing to remove.
    NotFound,
    /// The shortcuts are gone but these other entries were left, so the
    /// folder was kept.
    Kept(Vec<String>),
}

/// True when `path` is a symlink or a junction (a reparse point).
pub(crate) fn is_link(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        /// `FILE_ATTRIBUTE_REPARSE_POINT`.
        const REPARSE_POINT: u32 = 0x400;
        meta.file_attributes() & REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

/// `path`'s metadata without following links, `None` when it is missing.
fn metadata(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("read {}: {error}", path.display())),
    }
}

/// Unpins from Start and the taskbar, then deletes, the `.lnk` files in
/// `{programs_dir}\Grouper`, then deletes the folder itself if nothing
/// else is in it. Anything that is not a plain `.lnk` file (a subfolder,
/// another file, a link) is left alone and reported. A failed unpin (most
/// often: the group was never pinned) is logged and the shortcut still
/// goes.
pub fn remove_start_menu_folder(
    programs_dir: &Path,
    unpin: &dyn Fn(&Path) -> Result<(), String>,
) -> Result<Outcome, String> {
    let dir = programs_dir.join(OWN_FOLDER);
    let Some(meta) = metadata(&dir)? else {
        return Ok(Outcome::NotFound);
    };
    if is_link(&meta) || !meta.is_dir() {
        return Err(format!("refusing to clean {}: not a folder", dir.display()));
    }
    let entries = fs::read_dir(&dir).map_err(|e| format!("list {}: {e}", dir.display()))?;
    let mut kept = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("list {}: {e}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let shortcut = name.to_lowercase().ends_with(".lnk");
        let plain_file = metadata(&path)?.is_some_and(|m| m.is_file() && !is_link(&m));
        if shortcut && plain_file {
            if let Err(error) = unpin(&path) {
                log::info!("cleanup: {error}");
            }
            fs::remove_file(&path).map_err(|e| format!("delete {}: {e}", path.display()))?;
        } else {
            kept.push(name);
        }
    }
    if !kept.is_empty() {
        kept.sort();
        return Ok(Outcome::Kept(kept));
    }
    fs::remove_dir(&dir).map_err(|e| format!("delete {}: {e}", dir.display()))?;
    Ok(Outcome::Removed)
}

/// Deletes `app_data_dir` (app_state.json and the icon caches) with
/// everything in it. Refuses a folder not named `identifier`, and one
/// that is itself a link. `remove_dir_all` does not follow links inside.
pub fn remove_app_data(app_data_dir: &Path, identifier: &str) -> Result<Outcome, String> {
    let named_right = app_data_dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(identifier));
    if !named_right {
        return Err(format!(
            "refusing to delete {}: not Grouper's app data folder",
            app_data_dir.display()
        ));
    }
    let Some(meta) = metadata(app_data_dir)? else {
        return Ok(Outcome::NotFound);
    };
    if is_link(&meta) || !meta.is_dir() {
        return Err(format!(
            "refusing to delete {}: not a folder",
            app_data_dir.display()
        ));
    }
    fs::remove_dir_all(app_data_dir)
        .map_err(|e| format!("delete {}: {e}", app_data_dir.display()))?;
    Ok(Outcome::Removed)
}

/// Deletes the group icon folder with everything in it, then its parent
/// (`Programs\Grouper`) if that is left empty. Refuses a folder not named
/// [`GROUP_ICONS_FOLDER`], and one that is itself a link.
pub fn remove_group_icons(dir: &Path) -> Result<Outcome, String> {
    let named_right = dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(GROUP_ICONS_FOLDER));
    if !named_right {
        return Err(format!(
            "refusing to delete {}: not Grouper's group icon folder",
            dir.display()
        ));
    }
    let Some(meta) = metadata(dir)? else {
        return Ok(Outcome::NotFound);
    };
    if is_link(&meta) || !meta.is_dir() {
        return Err(format!(
            "refusing to delete {}: not a folder",
            dir.display()
        ));
    }
    fs::remove_dir_all(dir).map_err(|e| format!("delete {}: {e}", dir.display()))?;
    // Only removes the parent when it is empty; the install folder never is.
    if let Some(parent) = dir.parent() {
        if fs::remove_dir(parent).is_ok() {
            log::info!("cleanup: removed empty {}", parent.display());
        }
    }
    Ok(Outcome::Removed)
}
