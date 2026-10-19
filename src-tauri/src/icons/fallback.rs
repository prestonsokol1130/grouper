//! The generic icon shown for an app whose own icon cannot be extracted.
//!
//! It is Grouper's app icon, `src-tauri/icons/128x128@2x.png` (256 x 256),
//! embedded in the binary.

use image::{ImageFormat, RgbaImage};

use super::pixels::fit_square;

/// The embedded PNG.
const APP_ICON_PNG: &[u8] = include_bytes!("../../icons/128x128@2x.png");

/// Returns the fallback icon at `size` x `size` (`size` is capped at
/// [`MAX_ICON_SIZE`](super::MAX_ICON_SIZE)), scaled with Lanczos3.
///
/// Never fails: if the embedded PNG cannot be decoded (a test checks that it
/// can), the error is logged and a fully transparent image is returned.
pub fn fallback_icon(size: u32) -> RgbaImage {
    match image::load_from_memory_with_format(APP_ICON_PNG, ImageFormat::Png) {
        Ok(decoded) => fit_square(decoded.into_rgba8(), size),
        Err(error) => {
            log::error!("could not decode the embedded fallback icon: {error}");
            fit_square(RgbaImage::new(0, 0), size)
        }
    }
}

#[cfg(test)]
mod tests {
    use image::ImageFormat;

    use super::{fallback_icon, APP_ICON_PNG};

    #[test]
    fn embedded_png_decodes_at_256() {
        let decoded = image::load_from_memory_with_format(APP_ICON_PNG, ImageFormat::Png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (256, 256));
    }

    #[test]
    fn fallback_has_the_requested_size_and_visible_pixels() {
        for size in [256, 48] {
            let icon = fallback_icon(size);
            assert_eq!(icon.dimensions(), (size, size));
            assert!(icon.pixels().any(|p| p[3] > 0));
        }
    }
}
