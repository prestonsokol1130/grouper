//! A shortcut's own `.ico` file, read straight from disk.
//!
//! Chrome web app shortcuts (Gemini, Password Manager, Jules) target
//! `chrome_proxy.exe` and keep the real icon in an `.ico` under
//! `%LOCALAPPDATA%\Google\Chrome\User Data\...`. Asking the Shell for the
//! shortcut's icon gave a blank white page, the same way it does for
//! Grouper's own group icons kept under AppData (bug 9, 2026-10-21). So
//! when a shortcut names an `.ico` file as its icon, that file is decoded
//! here instead, and the Shell is only asked when this fails.

use std::path::{Path, PathBuf};

use image::{ImageFormat, RgbaImage};

use super::pixels::fit_square;
use crate::win32::env::expand_env;
use crate::win32::lnk_read::resolve_lnk;

/// Largest `.ico` file read. Windows' own icons stay well under 1 MB; the
/// cap keeps a bad file from forcing a huge read.
const MAX_ICO_BYTES: u64 = 16 * 1024 * 1024;

/// The `.ico` file a shortcut's stored icon location names, environment
/// variables expanded, when it is one (index 0) and exists as a file.
pub fn ico_path(icon_location: Option<(String, i32)>) -> Option<PathBuf> {
    let (raw, index) = icon_location?;
    let path = PathBuf::from(expand_env(raw.trim()));
    let is_ico = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ico"));
    (index == 0 && is_ico && path.is_file()).then_some(path)
}

/// Decodes `bytes` as an `.ico` (its largest image) into a `size` x `size`
/// image. Rejects an image with no visible pixel.
pub fn decode_ico(bytes: &[u8], size: u32) -> Result<RgbaImage, String> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::Ico)
        .map_err(|e| format!("decode .ico: {e}"))?
        .to_rgba8();
    if image.pixels().all(|pixel| pixel.0[3] == 0) {
        return Err("the .ico is fully transparent".into());
    }
    Ok(fit_square(image, size))
}

/// The icon of the shortcut at `lnk` read from its own `.ico` file, or
/// `None` when it names none. The caller's thread must hold a live
/// [`ComGuard`](crate::win32::com::ComGuard) (reading the shortcut needs it).
pub fn shortcut_ico(lnk: &Path, size: u32) -> Option<Result<RgbaImage, String>> {
    let ico = ico_path(resolve_lnk(lnk).ok()?.icon_location)?;
    Some(read_ico(&ico, size).map_err(|e| format!("{}: {e}", ico.display())))
}

/// Reads and decodes the `.ico` file at `path`.
pub fn read_ico(path: &Path, size: u32) -> Result<RgbaImage, String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_ICO_BYTES {
        return Err(format!("{len} bytes is too large for an icon"));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    decode_ico(&bytes, size)
}
