//! Encodes a composed 256 x 256 folder image as a multi-size `.ico`.

use ::ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::RgbaImage;

use crate::icons::pixels::{fit_square, premultiply, unpremultiply};

/// Every size in the `.ico`, in pixels per side. 256 is stored as PNG, the
/// rest as 32-bit BMP.
pub const ICO_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// The source image's side.
const SOURCE_SIZE: u32 = 256;

/// Sizes up to this one get a light unsharp pass after downscaling.
const SHARPEN_MAX: u32 = 32;

/// How much of the detail lost by a blur is added back (1.0 is a full
/// unsharp mask).
const SHARPEN_AMOUNT: f32 = 0.35;

/// Encodes `img256` (must be 256 x 256) as an `.ico` with every size in
/// [`ICO_SIZES`]. Smaller sizes are downscaled from it with Lanczos3 (on
/// premultiplied pixels); 32 and below then get a light unsharp pass.
///
/// The directory is laid out like the icons in `shell32.dll`: largest
/// frame first, and every entry (the PNG one too) with one colour plane
/// and 32 bits per pixel.
pub fn encode_ico(img256: &RgbaImage) -> Result<Vec<u8>, String> {
    if img256.dimensions() != (SOURCE_SIZE, SOURCE_SIZE) {
        let (width, height) = img256.dimensions();
        return Err(format!("icon must be 256 x 256, not {width} x {height}"));
    }
    let mut dir = IconDir::new(ResourceType::Icon);
    for size in ICO_SIZES.into_iter().rev() {
        let mut frame = fit_square(img256.clone(), size);
        if size <= SHARPEN_MAX {
            sharpen(&mut frame);
        }
        let image = IconImage::from_rgba_data(size, size, frame.into_raw());
        let entry = if size == SOURCE_SIZE {
            IconDirEntry::encode_as_png(&image)
        } else {
            // The ico crate picks 32 bpp for any image with partial alpha,
            // which the anti-aliased folder outline always has.
            IconDirEntry::encode_as_bmp(&image)
        };
        dir.add_entry(entry.map_err(|e| format!("encode {size} px icon: {e}"))?);
    }
    let mut bytes = Vec::new();
    dir.write(&mut bytes)
        .map_err(|e| format!("write ico: {e}"))?;
    set_planes(&mut bytes);
    Ok(bytes)
}

/// The size of the `ICONDIR` header and of each `ICONDIRENTRY`.
const DIR_HEADER: usize = 6;
const DIR_ENTRY: usize = 16;

/// Sets `wPlanes` to 1 in every directory entry that has 0. The `ico`
/// crate writes 0 for PNG frames; Windows' own icons use 1.
fn set_planes(bytes: &mut [u8]) {
    let count = bytes
        .get(4..DIR_HEADER)
        .map_or(0, |n| usize::from(u16::from_le_bytes([n[0], n[1]])));
    for index in 0..count {
        let at = DIR_HEADER + index * DIR_ENTRY + 4;
        if let Some(planes) = bytes.get_mut(at..at + 2) {
            if planes == [0, 0] {
                planes.copy_from_slice(&1_u16.to_le_bytes());
            }
        }
    }
}

/// A light unsharp mask: each colour channel gets `SHARPEN_AMOUNT` times
/// its difference from a 3 x 3 blur added back. It runs on premultiplied
/// pixels (so clear pixels do not darken edges), keeps colour within
/// alpha, and leaves alpha itself alone.
fn sharpen(image: &mut RgbaImage) {
    let (width, height) = image.dimensions();
    premultiply(image.as_chunks_mut::<4>().0);
    let source = image.clone();
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let alpha = f32::from(pixel[3]);
        for channel in 0..3 {
            let original = f32::from(pixel[channel]);
            let blurred = blur_at(&source, width, height, x, y, channel);
            let sharpened = original + SHARPEN_AMOUNT * (original - blurred);
            pixel[channel] = sharpened.round().clamp(0.0, alpha) as u8;
        }
    }
    unpremultiply(image.as_chunks_mut::<4>().0);
}

/// One channel of a 3 x 3 `[1 2 1]` blur at `(x, y)`, clamping at edges.
fn blur_at(image: &RgbaImage, width: u32, height: u32, x: u32, y: u32, channel: usize) -> f32 {
    const WEIGHTS: [f32; 3] = [1.0, 2.0, 1.0];
    let mut sum = 0.0;
    for (dy, wy) in (0_u32..).zip(WEIGHTS.iter()) {
        for (dx, wx) in (0_u32..).zip(WEIGHTS.iter()) {
            let sx = x
                .saturating_add(dx)
                .saturating_sub(1)
                .min(width.saturating_sub(1));
            let sy = y
                .saturating_add(dy)
                .saturating_sub(1)
                .min(height.saturating_sub(1));
            sum += wx * wy * f32::from(image.get_pixel(sx, sy)[channel]);
        }
    }
    sum / 16.0
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{encode_ico, sharpen, ICO_SIZES};

    #[test]
    fn every_size_is_present_and_decodes() {
        let source = RgbaImage::from_fn(256, 256, |x, _| {
            Rgba([200, 30, 30, if x < 4 { 128 } else { 255 }])
        });
        let bytes = encode_ico(&source).unwrap();
        let dir = ::ico::IconDir::read(std::io::Cursor::new(bytes)).unwrap();
        let mut sizes: Vec<u32> = dir.entries().iter().map(|e| e.width()).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, ICO_SIZES);
        for entry in dir.entries() {
            assert_eq!(entry.width(), entry.height());
            assert_eq!(entry.is_png(), entry.width() == 256);
            if !entry.is_png() {
                assert_eq!(entry.bits_per_pixel(), 32, "{} px", entry.width());
            }
            let decoded = entry.decode().unwrap();
            assert_eq!(decoded.width(), entry.width());
        }
    }

    #[test]
    fn wrong_size_is_an_error() {
        assert!(encode_ico(&RgbaImage::new(128, 128)).is_err());
        assert!(encode_ico(&RgbaImage::new(0, 0)).is_err());
    }

    #[test]
    fn sharpen_keeps_flat_areas_and_alpha() {
        let mut flat = RgbaImage::from_pixel(8, 8, Rgba([100, 150, 200, 255]));
        let original = flat.clone();
        sharpen(&mut flat);
        assert_eq!(flat, original);

        let mut edge = RgbaImage::from_fn(8, 8, |x, _| {
            if x < 4 {
                Rgba([200, 200, 200, 255])
            } else {
                Rgba([100, 100, 100, 200])
            }
        });
        sharpen(&mut edge);
        // The light side gets lighter at the edge, the dark side darker.
        assert!(edge.get_pixel(3, 4)[0] > 200);
        assert!(edge.get_pixel(4, 4)[0] < 100);
        assert!(edge.pixels().all(|p| p[3] == 255 || p[3] == 200));
    }
}
