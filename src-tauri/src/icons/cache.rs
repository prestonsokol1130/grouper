//! The on-disk PNG cache of app icons, `{app_data_dir}/cache/icons`.
//!
//! Every function here takes `cache_dir`, the icon cache folder itself (see
//! [`icon_cache_dir`](crate::state::paths::icon_cache_dir)).

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use image::{ImageFormat, RgbaImage};

use super::{extract, fallback};
use crate::state::disk_write::{discard, reject_symlink, write_synced};
use crate::state::AppEntry;

/// Pixels per side of every cached icon.
pub const ICON_SIZE: u32 = 256;

/// File stem of the shared fallback icon. App ids are hex, so no app's icon
/// can have this name.
pub const FALLBACK_STEM: &str = "_fallback";

/// Extracts the icon at a path as a `size` x `size` image:
/// [`extract::extract_icon`] in the app, a fake in tests.
pub type Extractor<'a> = &'a dyn Fn(&Path, u32) -> Result<RgbaImage, String>;

/// What [`ensure_icon_with`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ensured {
    /// The cached PNG was fresh and is reused.
    Fresh(PathBuf),
    /// A new PNG was extracted and written.
    Written(PathBuf),
    /// The app has no usable icon; this is the fallback PNG.
    Fallback(PathBuf),
}

impl Ensured {
    /// The PNG to show for the app.
    pub fn into_path(self) -> PathBuf {
        match self {
            Self::Fresh(path) | Self::Written(path) | Self::Fallback(path) => path,
        }
    }
}

/// `{cache_dir}/{app_id}.png`.
pub fn icon_path(cache_dir: &Path, app_id: &str) -> PathBuf {
    cache_dir.join(format!("{app_id}.png"))
}

/// `{cache_dir}/_fallback.png`.
pub fn fallback_path(cache_dir: &Path) -> PathBuf {
    icon_path(cache_dir, FALLBACK_STEM)
}

/// Returns the PNG to show for `app`, extracting and caching it when needed.
///
/// The caller's thread must hold a live
/// [`ComGuard`](crate::win32::com::ComGuard). See [`ensure_icon_with`] for
/// the rules.
pub fn ensure_icon(app: &AppEntry, cache_dir: &Path) -> Result<PathBuf, String> {
    ensure_icon_with(app, cache_dir, &extract::extract_icon).map(Ensured::into_path)
}

/// Like [`ensure_icon`], with the extractor passed in.
///
/// - A cached PNG is reused when it is newer than the target's mtime and,
///   when `lnk_path` is a regular file, the shortcut's mtime too. When
///   the target's mtime cannot be read (the target is missing), a cached PNG
///   is reused as it is the best icon left; with no cached PNG the fallback
///   is used and nothing is extracted.
/// - Otherwise the icon is extracted from `lnk_path` if set (falling back
///   to the target if that fails, for example a deleted shortcut), else from
///   the target, and written atomically (a uniquely named temp file, then a
///   rename). If extraction fails the failure is logged, any stale PNG is
///   left in place, and the fallback is returned.
///
/// Returns `Err` if the app id is not a safe file name or a PNG cannot be
/// written (for example the cache folder or the PNG is a symlink).
pub fn ensure_icon_with(
    app: &AppEntry,
    cache_dir: &Path,
    extract: Extractor<'_>,
) -> Result<Ensured, String> {
    check_id(&app.id)?;
    let cached = icon_path(cache_dir, &app.id);
    let target = Path::new(&app.target);
    let target_mtime = fs::metadata(target).and_then(|meta| meta.modified()).ok();
    let lnk = app.lnk_path.as_deref().filter(|lnk| !lnk.is_empty());
    if let Some(cached_at) = regular_file_mtime(&cached) {
        let fresh = match target_mtime {
            // The target is missing: the cached PNG is the best icon left.
            None => true,
            Some(target_at) => {
                let lnk_mtime = lnk.and_then(|lnk| regular_file_mtime(Path::new(lnk)));
                cached_at > target_at && lnk_mtime.is_none_or(|lnk_at| cached_at > lnk_at)
            }
        };
        if fresh {
            return Ok(Ensured::Fresh(cached));
        }
    }
    if target_mtime.is_none() {
        return ensure_fallback(cache_dir).map(Ensured::Fallback);
    }
    let extracted = match lnk {
        Some(lnk) => extract(Path::new(lnk), ICON_SIZE).or_else(|lnk_error| {
            extract(target, ICON_SIZE).map_err(|e| format!("{lnk_error}; then the target: {e}"))
        }),
        None => extract(target, ICON_SIZE),
    };
    match extracted {
        Ok(image) => {
            write_png(cache_dir, &cached, &image)?;
            Ok(Ensured::Written(cached))
        }
        Err(error) => {
            log::warn!("no icon for app {} ({}): {error}", app.id, app.name);
            ensure_fallback(cache_dir).map(Ensured::Fallback)
        }
    }
}

/// Writes the fallback icon to `{cache_dir}/_fallback.png` unless it is
/// already there, and returns its path.
pub fn ensure_fallback(cache_dir: &Path) -> Result<PathBuf, String> {
    let path = fallback_path(cache_dir);
    if regular_file_mtime(&path).is_none() {
        write_png(cache_dir, &path, &fallback::fallback_icon(ICON_SIZE))?;
    }
    Ok(path)
}

/// App ids become file names, so they must be non-empty `[A-Za-z0-9_-]`.
fn check_id(app_id: &str) -> Result<(), String> {
    let safe = !app_id.is_empty()
        && app_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if safe {
        Ok(())
    } else {
        Err(format!("app id {app_id:?} is not a safe icon file name"))
    }
}

/// The mtime of `path` if it is a regular file (not a symlink).
fn regular_file_mtime(path: &Path) -> Option<SystemTime> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_file() {
        return None;
    }
    meta.modified().ok()
}

/// Encodes `image` as PNG and writes it to `dest` through a temp file in the
/// same folder plus a rename. Never writes through a symlinked cache folder
/// or PNG.
fn write_png(cache_dir: &Path, dest: &Path, image: &RgbaImage) -> Result<(), String> {
    reject_symlink(cache_dir)?;
    reject_symlink(dest)?;
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| format!("encode {}: {e}", dest.display()))?;
    let temp = temp_path(dest)?;
    let result = write_synced(&temp, &bytes).and_then(|()| {
        fs::rename(&temp, dest)
            .map_err(|e| format!("rename {} over {}: {e}", temp.display(), dest.display()))
    });
    if result.is_err() {
        discard(&temp);
    }
    result
}

/// `{dest}.{random}.tmp`: unique, so two threads writing the same icon never
/// share a temp file.
fn temp_path(dest: &Path) -> Result<PathBuf, String> {
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|e| format!("random temp name: {e}"))?;
    let suffix: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let mut name = dest.as_os_str().to_owned();
    name.push(format!(".{suffix}.tmp"));
    Ok(PathBuf::from(name))
}
