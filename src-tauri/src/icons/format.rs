//! The icon cache's format version. When the way icons are drawn changes
//! (for example the normalising in [`super::trim`]), the version is bumped
//! and every cached app PNG from an older build is deleted once at
//! startup, so the icon worker extracts it again the new way.
//!
//! The version lives in `{cache_dir}/format.txt`; a cache without one is
//! version 1 (every build before the normaliser).

use std::fs;
use std::path::{Path, PathBuf};

use super::cache::{check_id, FALLBACK_STEM};
use crate::state::disk_write::write_atomic;

/// The format this build writes. 2: icons normalised to a 5% padding.
pub const ICON_FORMAT: u32 = 2;

/// `{cache_dir}/format.txt`.
fn marker_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("format.txt")
}

/// The format the cache in `cache_dir` was written in: 1 when unmarked.
pub fn cache_format(cache_dir: &Path) -> u32 {
    fs::read_to_string(marker_path(cache_dir))
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(1)
}

/// Brings the cache in `cache_dir` up to [`ICON_FORMAT`]: deletes every
/// app PNG (regular files named `{app_id}.png`, never a symlink or the
/// fallback icon) and records the new format. Returns how many PNGs were
/// deleted, or `None` when the cache was already current. Failures are
/// logged; a PNG that cannot be deleted is just redrawn later.
pub fn upgrade(cache_dir: &Path) -> Option<usize> {
    let found = cache_format(cache_dir);
    if found == ICON_FORMAT {
        return None;
    }
    let mut deleted = 0;
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_app_png(&path) && remove(&path) {
                deleted += 1;
            }
        }
    }
    if let Err(error) = fs::create_dir_all(cache_dir)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            let text = format!("{ICON_FORMAT}\n");
            write_atomic(cache_dir, &marker_path(cache_dir), text.as_bytes())
        })
    {
        log::warn!("could not record the icon cache format: {error}");
    }
    log::info!("icon cache format {found} -> {ICON_FORMAT}: {deleted} PNG(s) to redraw");
    Some(deleted)
}

/// A regular file (not a symlink) named `{app_id}.png`.
fn is_app_png(path: &Path) -> bool {
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".png"));
    let named = stem.is_some_and(|s| s != FALLBACK_STEM && check_id(s).is_ok());
    named && fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file())
}

fn remove(path: &Path) -> bool {
    match fs::remove_file(path) {
        Ok(()) => true,
        Err(error) => {
            log::warn!("could not delete {}: {error}", path.display());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_cache_is_cleared_once() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path();
        for name in [
            "0123456789abcdef.png",
            "_fallback.png",
            "notes.txt",
            "a.b.png",
        ] {
            fs::write(cache.join(name), b"x").unwrap();
        }
        assert_eq!(cache_format(cache), 1);
        assert_eq!(upgrade(cache), Some(1));
        assert!(!cache.join("0123456789abcdef.png").exists());
        for kept in ["_fallback.png", "notes.txt", "a.b.png"] {
            assert!(cache.join(kept).exists(), "{kept}");
        }
        assert_eq!(cache_format(cache), ICON_FORMAT);
        // Already current: nothing more is deleted.
        fs::write(cache.join("0123456789abcdef.png"), b"x").unwrap();
        assert_eq!(upgrade(cache), None);
        assert!(cache.join("0123456789abcdef.png").exists());
    }

    #[test]
    fn a_missing_cache_folder_is_marked_current() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("icons");
        assert_eq!(upgrade(&cache), Some(0));
        assert_eq!(cache_format(&cache), ICON_FORMAT);
    }
}
