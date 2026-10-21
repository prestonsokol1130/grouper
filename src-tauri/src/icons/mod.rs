//! App icons: extraction through the Windows Shell ([`extract`]), with a
//! generic fallback icon ([`fallback`]) when extraction fails, a PNG cache on
//! disk ([`cache`]) and the background pool that fills it ([`worker`]).

pub mod cache;
pub mod extract;
pub mod fallback;
pub mod format;
pub mod gc;
pub mod ico_file;
pub mod pixels;
pub mod plate;
pub mod trim;
pub mod worker;

#[cfg(test)]
mod cache_lnk_tests;
#[cfg(test)]
mod cache_tests;
#[cfg(all(test, windows))]
mod extract_tests;
#[cfg(test)]
mod ico_file_tests;
#[cfg(test)]
mod worker_tests;

use std::path::Path;

use image::RgbaImage;

/// Largest icon size, in pixels per side, that can be asked for.
pub const MAX_ICON_SIZE: u32 = 1024;

/// Extracts the icon at `path` as a `size` x `size` image, or returns the
/// fallback icon if extraction fails (the failure is logged).
///
/// Like [`extract::extract_icon`], the caller's thread must hold a live
/// [`ComGuard`](crate::win32::com::ComGuard).
pub fn icon_or_fallback(path: &Path, size: u32) -> RgbaImage {
    extract::extract_icon(path, size).unwrap_or_else(|error| {
        log::warn!("using the fallback icon: {error}");
        fallback::fallback_icon(size)
    })
}
