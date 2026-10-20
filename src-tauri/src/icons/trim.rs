//! Normalises every cached app icon so they all look the same size in
//! tiles, the flyout and a group's 2x2 icon (Preston's feedback,
//! 2026-10-20: Cursor and Builder.io looked smaller than the rest).
//!
//! Two things made icons look small: a wide transparent margin (Store
//! icons), and the frame the Shell draws when an `.exe` has no 256 px icon
//! (a faint 38/77-alpha box around a 144 px icon in the middle). Both are
//! removed: the frame is dropped ([`frame_width`]), the content box is
//! found by alpha ([`content_box`], ignoring faint shadows), and the
//! content is scaled, keeping its aspect ratio, to fill [`FILL_PERCENT`] of
//! the canvas, centred: a 5% padding on each side for every icon.
//!
//! Opaque icons drawn on their own plate (Cursor's dark rounded square,
//! Zed's) are left as they are: the plate is part of the artwork and fills
//! the canvas like any full-bleed icon, so they already match. Removing a
//! "flat plate" would make them differ from icons of the same style.

use image::{imageops, RgbaImage};

use super::pixels::fit_square;

/// A pixel with at least this alpha counts as part of the icon; fainter
/// ones (soft shadows, glows) do not.
const SOLID_ALPHA: u8 = 32;

/// How much of the side the icon's content fills: 90%, so a 5% padding.
pub const FILL_PERCENT: u32 = 90;

/// The Shell's frame is no more opaque than this...
const FRAME_MAX_ALPHA: u8 = 96;
/// ...and no wider than this share of the side (8 px at 256).
const FRAME_MAX_SHARE: u32 = 32;
/// Inside the frame a ring this faint or fainter is the clear gap.
const CLEAR_ALPHA: u8 = 8;

/// `image` as a `size` x `size` icon whose content fills [`FILL_PERCENT`]
/// of the side, centred (see the file comment). An image with no content
/// is only fitted to `size`.
pub fn trim_margin(mut image: RgbaImage, size: u32) -> RgbaImage {
    if let Some(width) = frame_width(&image) {
        clear_border(&mut image, width);
    }
    if content_box(&image).is_none() {
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

/// The highest alpha on the one-pixel ring `inset` px in from the edge.
fn ring_alpha(image: &RgbaImage, inset: u32) -> u8 {
    let (width, height) = image.dimensions();
    if inset.saturating_mul(2) >= width.min(height) {
        return 0;
    }
    let (right, bottom) = (width - 1 - inset, height - 1 - inset);
    let alpha = |x, y| image.get_pixel(x, y)[3];
    let across = (inset..=right).map(|x| alpha(x, inset).max(alpha(x, bottom)));
    let down = (inset..=bottom).map(|y| alpha(inset, y).max(alpha(right, y)));
    across.chain(down).max().unwrap_or(0)
}

/// The width of a faint frame around the image (the Shell's box around a
/// small icon): outer rings no more opaque than `FRAME_MAX_ALPHA`, then a
/// clear ring. `None` when the edge is clear or solid (no frame).
pub(crate) fn frame_width(image: &RgbaImage) -> Option<u32> {
    let side = image.width().min(image.height());
    let widest = (side / FRAME_MAX_SHARE).max(1);
    for inset in 0..=widest {
        let alpha = ring_alpha(image, inset);
        if alpha <= CLEAR_ALPHA {
            return (inset > 0).then_some(inset);
        }
        if alpha > FRAME_MAX_ALPHA {
            return None;
        }
    }
    None
}

/// Makes the outer `width` px of `image` fully transparent.
fn clear_border(image: &mut RgbaImage, width: u32) {
    let (w, h) = image.dimensions();
    for (x, y, px) in image.enumerate_pixels_mut() {
        let inside = x >= width && y >= width && x + width < w && y + width < h;
        if !inside {
            px.0 = [0, 0, 0, 0];
        }
    }
}

#[cfg(test)]
#[path = "trim_tests.rs"]
mod tests;
