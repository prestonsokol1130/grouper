//! Trims the wide empty margin of Store-app icons, so they sit at the same
//! size as desktop-app icons in tiles and in a group's 2x2 icon.

use image::{imageops, RgbaImage};

use super::pixels::fit_square;

/// A pixel with at least this alpha counts as part of the icon.
const SOLID_ALPHA: u8 = 16;

/// How much of the side the icon's content fills after trimming. Desktop
/// icons at 256 px fill about this much.
pub const FILL_PERCENT: u32 = 88;

/// `image` as a `size` x `size` icon whose content (its pixels with alpha
/// of at least [`SOLID_ALPHA`]) fills [`FILL_PERCENT`] of the longer side,
/// centred. An image whose content already fills that much, or that has
/// none, is only fitted to `size`.
pub fn trim_margin(image: RgbaImage, size: u32) -> RgbaImage {
    let Some((_, _, width, height)) = content_box(&image) else {
        return fit_square(image, size);
    };
    let longest = u64::from(width.max(height));
    let (image_w, image_h) = image.dimensions();
    let side = u64::from(image_w.max(image_h));
    if longest * 100 >= side * u64::from(FILL_PERCENT) {
        return fit_square(image, size);
    }
    let content = crop_to_content(image);
    let inner = u32::try_from(u64::from(size) * u64::from(FILL_PERCENT) / 100)
        .unwrap_or(size)
        .max(1);
    let fitted = fit_square(content, inner);
    let mut canvas = RgbaImage::new(size, size);
    let offset = i64::from((size - inner.min(size)) / 2);
    imageops::replace(&mut canvas, &fitted, offset, offset);
    canvas
}

/// `image` cropped to its content box; unchanged when it has no content.
pub fn crop_to_content(image: RgbaImage) -> RgbaImage {
    match content_box(&image) {
        Some((x, y, width, height)) => imageops::crop_imm(&image, x, y, width, height).to_image(),
        None => image,
    }
}

/// The smallest `(x, y, width, height)` box holding every solid pixel, or
/// `None` when there is none.
pub(crate) fn content_box(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (x, y, px) in image.enumerate_pixels() {
        if px[3] < SOLID_ALPHA {
            continue;
        }
        found = Some(match found {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }
    found.map(|(x0, y0, x1, y1)| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use super::*;

    /// A `side` x `side` image with an opaque square of `inner` px centred.
    fn padded(side: u32, inner: u32) -> RgbaImage {
        let start = (side - inner) / 2;
        RgbaImage::from_fn(side, side, |x, y| {
            let inside = (start..start + inner).contains(&x) && (start..start + inner).contains(&y);
            if inside {
                Rgba([200, 10, 10, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        })
    }

    #[test]
    fn a_small_icon_is_scaled_up_to_the_fill() {
        let out = trim_margin(padded(256, 128), 256);
        assert_eq!(out.dimensions(), (256, 256));
        let (_, _, width, height) = content_box(&out).unwrap();
        let want = 256 * FILL_PERCENT / 100;
        assert!(
            width.abs_diff(want) <= 2 && height.abs_diff(want) <= 2,
            "{width}x{height}"
        );
    }

    #[test]
    fn a_full_icon_is_left_alone() {
        let full = padded(256, 240);
        assert_eq!(trim_margin(full.clone(), 256), full);
    }

    #[test]
    fn an_empty_icon_stays_empty() {
        let empty = RgbaImage::new(64, 64);
        let out = trim_margin(empty, 256);
        assert_eq!(out.dimensions(), (256, 256));
        assert!(content_box(&out).is_none());
    }
}
