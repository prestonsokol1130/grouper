//! The app data folders Grouper needs on disk.

use std::fs;
use std::path::{Path, PathBuf};

/// Name of the app icon folder inside the cache folder.
pub const ICONS_DIR: &str = "icons";

/// Name of the group icon folder inside the cache folder.
pub const GROUPS_DIR: &str = "groups";

/// `{app_data_dir}/cache`.
pub fn cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cache")
}

/// `{app_data_dir}/cache/icons`.
pub fn icon_cache_dir(app_data_dir: &Path) -> PathBuf {
    cache_dir(app_data_dir).join(ICONS_DIR)
}

/// `{app_data_dir}/cache/groups`.
pub fn group_cache_dir(app_data_dir: &Path) -> PathBuf {
    cache_dir(app_data_dir).join(GROUPS_DIR)
}

/// Creates the app data dir and its cache folders if they are missing.
pub fn ensure_dirs(app_data_dir: &Path) -> Result<(), String> {
    for dir in [
        app_data_dir.to_path_buf(),
        icon_cache_dir(app_data_dir),
        group_cache_dir(app_data_dir),
    ] {
        fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    Ok(())
}
