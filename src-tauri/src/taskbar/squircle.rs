//! Anti-aliased alpha masks: the superellipse ("squircle") outline of a
//! folder and the rounded square of each icon inside it.
//!
//! Every mask is `size` x `size`, 255 inside the shape, 0 outside and in
//! between on the edge. Each pixel is sampled on a 4 x 4 grid (16 samples).

use image::{GrayImage, Luma, RgbaImage};

/// Samples per pixel side.
const SUPERSAMPLE: u32 = 4;

/// The superellipse `|x|^n + |y|^n <= 1` filling the whole square.
pub fn squircle_mask(size: u32, n: f32) -> GrayImage {
    inset_squircle_mask(size, n, 0.0)
}

/// Like [`squircle_mask`], with the outline pulled in by `inset` pixels on
/// every side. An empty mask if nothing is left, or if `n` is not a
/// positive number.
pub fn inset_squircle_mask(size: u32, n: f32, inset: f32) -> GrayImage {
    let half = size as f32 / 2.0;
    let radius = half - inset;
    if !(radius > 0.0 && n > 0.0 && n.is_finite()) {
        return GrayImage::new(size, size);
    }
    coverage_mask(size, |x, y| {
        let u = ((x - half) / radius).abs().powf(n);
        let v = ((y - half) / radius).abs().powf(n);
        u + v <= 1.0
    })
}

/// A square with its corners rounded to `radius` pixels (clamped to
/// `0..=size / 2`; a radius that is not a number counts as 0).
pub fn rounded_square_mask(size: u32, radius: f32) -> GrayImage {
    let side = size as f32;
    let r = if radius.is_finite() {
        radius.clamp(0.0, side / 2.0)
    } else {
        0.0
    };
    coverage_mask(size, |x, y| {
        // Distance past the straight edges, towards the nearest corner.
        let dx = (r - x).max(x - (side - r)).max(0.0);
        let dy = (r - y).max(y - (side - r)).max(0.0);
        dx * dx + dy * dy <= r * r
    })
}

/// Multiplies each pixel's alpha by the mask's value there (`a * m / 255`,
/// rounded). Pixels outside the mask are left alone.
pub fn apply_mask(image: &mut RgbaImage, mask: &GrayImage) {
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        if let Some(Luma([m])) = mask.get_pixel_checked(x, y) {
            let scaled = (u32::from(pixel[3]) * u32::from(*m) + 127) / 255;
            pixel[3] = u8::try_from(scaled).unwrap_or(u8::MAX);
        }
    }
}

/// Builds a mask from `inside(x, y)`, sampled at 4 x 4 points per pixel.
fn coverage_mask(size: u32, inside: impl Fn(f32, f32) -> bool) -> GrayImage {
    let step = 1.0 / SUPERSAMPLE as f32;
    let total = SUPERSAMPLE * SUPERSAMPLE;
    GrayImage::from_fn(size, size, |px, py| {
        let mut hits = 0;
        for sy in 0..SUPERSAMPLE {
            for sx in 0..SUPERSAMPLE {
                let x = px as f32 + (sx as f32 + 0.5) * step;
                let y = py as f32 + (sy as f32 + 0.5) * step;
                if inside(x, y) {
                    hits += 1;
                }
            }
        }
        let value = (hits * 255 + total / 2) / total;
        Luma([u8::try_from(value).unwrap_or(u8::MAX)])
    })
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{apply_mask, inset_squircle_mask, rounded_square_mask, squircle_mask};

    #[test]
    fn squircle_is_clear_in_the_corners_and_solid_in_the_middle() {
        let mask = squircle_mask(256, 5.0);
        assert_eq!(mask.dimensions(), (256, 256));
        for (x, y) in [(0, 0), (255, 0), (0, 255), (255, 255)] {
            assert_eq!(mask.get_pixel(x, y)[0], 0);
        }
        assert_eq!(mask.get_pixel(128, 128)[0], 255);
        // The middle of each side touches the edge of the square.
        assert_eq!(mask.get_pixel(128, 0)[0], 255);
        assert_eq!(mask.get_pixel(0, 128)[0], 255);
    }

    #[test]
    fn squircle_edge_is_anti_aliased_and_symmetric() {
        let mask = squircle_mask(64, 5.0);
        assert!(mask.pixels().any(|p| p[0] > 0 && p[0] < 255));
        for y in 0..64 {
            for x in 0..64 {
                let value = mask.get_pixel(x, y)[0];
                assert_eq!(value, mask.get_pixel(63 - x, y)[0]);
                assert_eq!(value, mask.get_pixel(x, 63 - y)[0]);
            }
        }
    }

    #[test]
    fn inset_and_bad_inputs() {
        let inset = inset_squircle_mask(64, 5.0, 4.0);
        assert_eq!(inset.get_pixel(32, 1)[0], 0);
        assert_eq!(inset.get_pixel(32, 5)[0], 255);
        for mask in [
            inset_squircle_mask(16, 5.0, 8.0),
            squircle_mask(16, f32::NAN),
            squircle_mask(16, 0.0),
        ] {
            assert!(mask.pixels().all(|p| p[0] == 0));
        }
        assert_eq!(squircle_mask(0, 5.0).dimensions(), (0, 0));
    }

    #[test]
    fn rounded_square_rounds_only_the_corners() {
        let mask = rounded_square_mask(40, 10.0);
        assert_eq!(mask.get_pixel(0, 0)[0], 0);
        assert_eq!(mask.get_pixel(20, 0)[0], 255);
        assert_eq!(mask.get_pixel(0, 20)[0], 255);
        assert_eq!(mask.get_pixel(20, 20)[0], 255);
        for radius in [0.0, f32::NAN] {
            let square = rounded_square_mask(8, radius);
            assert!(square.pixels().all(|p| p[0] == 255));
        }
    }

    #[test]
    fn mask_scales_alpha_only() {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 200]));
        apply_mask(&mut image, &squircle_mask(64, 5.0));
        assert_eq!(*image.get_pixel(0, 0), Rgba([10, 20, 30, 0]));
        assert_eq!(*image.get_pixel(32, 32), Rgba([10, 20, 30, 200]));
    }
}
