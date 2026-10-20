//! Pixel fix-ups for icons: 32-bit GDI pixels to straight-alpha RGBA, and
//! fitting an image into a square.

use image::imageops::{self, FilterType};
use image::RgbaImage;

use super::MAX_ICON_SIZE;

/// Turns the premultiplied 32-bit BGRA pixels read from GDI into
/// straight-alpha RGBA, in place (see [`straighten_alpha`] for the alpha
/// rules).
///
/// Returns `Err` for a fully blank bitmap (every byte zero), so the caller
/// can use a fallback icon instead.
pub fn gdi_to_rgba(buf: &mut [u8]) -> Result<(), String> {
    bgra_to_rgba(buf);
    straighten_alpha(buf);
    if buf.iter().all(|&byte| byte == 0) {
        return Err("shell returned a blank icon".to_owned());
    }
    Ok(())
}

/// Swaps the blue and red bytes of every 4-byte pixel: BGRA becomes RGBA.
fn bgra_to_rgba(buf: &mut [u8]) {
    for px in buf.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
}

/// Turns premultiplied RGBA pixels into straight (not premultiplied) alpha.
///
/// - If every alpha is 0 but some pixel has colour, the bitmap had no alpha
///   channel (an old-style icon), so every pixel is made opaque.
/// - Otherwise each colour channel is un-premultiplied as `c * 255 / a`
///   (clamped to 255) where `0 < a < 255`. Pixels with alpha 0 or 255 are
///   left as they are.
fn straighten_alpha(buf: &mut [u8]) {
    let pixels = buf.as_chunks_mut::<4>().0;
    if pixels.iter().all(|px| px[3] == 0) {
        if pixels.iter().any(|px| px[..3] != [0, 0, 0]) {
            for px in pixels {
                px[3] = u8::MAX;
            }
        }
        return;
    }
    unpremultiply(pixels);
}

/// Multiplies each colour channel by its pixel's alpha: `c * a / 255`.
pub(crate) fn premultiply(pixels: &mut [[u8; 4]]) {
    for px in pixels {
        let alpha = u16::from(px[3]);
        for channel in &mut px[..3] {
            // c and a are both at most 255, so the result is too.
            let premultiplied = u16::from(*channel) * alpha / 255;
            *channel = u8::try_from(premultiplied).unwrap_or(u8::MAX);
        }
    }
}

/// Divides each colour channel by its pixel's alpha: `c * 255 / a`, clamped
/// to 255, only where `0 < a < 255`.
pub(crate) fn unpremultiply(pixels: &mut [[u8; 4]]) {
    for px in pixels {
        let alpha = u16::from(px[3]);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut px[..3] {
            let straight = u16::from(*channel) * 255 / alpha;
            *channel = u8::try_from(straight).unwrap_or(u8::MAX);
        }
    }
}

/// Returns `image` as a `size` x `size` image. One of another size is
/// scaled with Lanczos3 so its longer side is `size`, keeping its aspect
/// ratio, and centred on a transparent canvas.
///
/// The scaling runs on premultiplied pixels (Lanczos3 assumes them), so
/// transparent pixels do not darken the edges; the result is straight
/// alpha again.
///
/// `size` is capped at [`MAX_ICON_SIZE`]. An image with no pixels gives a
/// fully transparent result.
pub fn fit_square(mut image: RgbaImage, size: u32) -> RgbaImage {
    let size = size.min(MAX_ICON_SIZE);
    let (width, height) = image.dimensions();
    if (width, height) == (size, size) {
        return image;
    }
    let mut canvas = RgbaImage::new(size, size);
    if width == 0 || height == 0 || size == 0 {
        return canvas;
    }
    let (new_width, new_height) = if width >= height {
        (size, scale(height, size, width))
    } else {
        (scale(width, size, height), size)
    };
    premultiply(image.as_chunks_mut::<4>().0);
    let mut resized = imageops::resize(&image, new_width, new_height, FilterType::Lanczos3);
    unpremultiply(resized.as_chunks_mut::<4>().0);
    let x = i64::from((size - new_width) / 2);
    let y = i64::from((size - new_height) / 2);
    imageops::replace(&mut canvas, &resized, x, y);
    canvas
}

/// `value * to / from`, rounded and kept within `1..=to`. `value` is at
/// most `from`, and `from` is never 0. Computed in u64, so it cannot
/// overflow.
fn scale(value: u32, to: u32, from: u32) -> u32 {
    let from = u64::from(from);
    let scaled = (u64::from(value) * u64::from(to) + from / 2) / from;
    u32::try_from(scaled).unwrap_or(to).clamp(1, to)
}

#[cfg(test)]
mod tests {
    use image::imageops::{self, FilterType};
    use image::{Rgba, RgbaImage};

    use super::{
        bgra_to_rgba, fit_square, gdi_to_rgba, premultiply, straighten_alpha, unpremultiply,
    };

    #[test]
    fn gdi_pixels_become_straight_rgba() {
        // BGRA, premultiplied: B 0, G 32, R 64, A 128; then transparent.
        let mut buf = vec![0, 32, 64, 128, 0, 0, 0, 0];
        gdi_to_rgba(&mut buf).unwrap();
        assert_eq!(buf, vec![127, 63, 0, 128, 0, 0, 0, 0]);
    }

    #[test]
    fn gdi_colour_without_alpha_is_not_blank() {
        let mut buf = vec![9, 8, 7, 0, 0, 0, 0, 0];
        gdi_to_rgba(&mut buf).unwrap();
        assert_eq!(buf, vec![7, 8, 9, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn blank_gdi_bitmap_is_an_error() {
        let mut buf = vec![0; 16];
        assert_eq!(
            gdi_to_rgba(&mut buf),
            Err("shell returned a blank icon".to_owned())
        );
    }

    #[test]
    fn swaps_blue_and_red() {
        let mut buf = vec![1, 2, 3, 4, 5, 6, 7, 8];
        bgra_to_rgba(&mut buf);
        assert_eq!(buf, vec![3, 2, 1, 4, 7, 6, 5, 8]);
    }

    #[test]
    fn unpremultiplies_partly_transparent_pixels() {
        let mut buf = vec![
            64, 32, 0, 128, // half transparent: 64*255/128 = 127, 32*255/128 = 63
            10, 20, 30, 255, // opaque: unchanged
            0, 0, 0, 0, // transparent: unchanged
            51, 0, 51, 51, // a = 51: 51*255/51 = 255
            200, 0, 0, 100, // bad data (c > a): clamped to 255
        ];
        straighten_alpha(&mut buf);
        assert_eq!(
            buf,
            vec![127, 63, 0, 128, 10, 20, 30, 255, 0, 0, 0, 0, 255, 0, 255, 51, 255, 0, 0, 100]
        );
    }

    #[test]
    fn colour_with_no_alpha_becomes_opaque() {
        let mut buf = vec![0, 0, 0, 0, 9, 8, 7, 0];
        straighten_alpha(&mut buf);
        assert_eq!(buf, vec![0, 0, 0, 255, 9, 8, 7, 255]);
    }

    #[test]
    fn fully_blank_bitmap_stays_transparent() {
        let mut buf = vec![0; 16];
        straighten_alpha(&mut buf);
        assert_eq!(buf, vec![0; 16]);
    }

    #[test]
    fn square_image_of_the_right_size_is_unchanged() {
        let image = RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 4]));
        assert_eq!(fit_square(image.clone(), 8), image);
    }

    #[test]
    fn wide_image_is_scaled_and_centred() {
        let image = RgbaImage::from_pixel(64, 32, Rgba([255, 0, 0, 255]));
        let fitted = fit_square(image, 32);
        assert_eq!(fitted.dimensions(), (32, 32));
        // Scaled to 32 x 16, so rows 8..24 hold the image.
        assert_eq!(fitted.get_pixel(16, 16)[3], 255);
        assert_eq!(fitted.get_pixel(16, 0)[3], 0);
        assert_eq!(fitted.get_pixel(16, 31)[3], 0);
    }

    #[test]
    fn resizing_runs_on_premultiplied_pixels() {
        // Left half opaque red, right half transparent black.
        let red = Rgba([255, 0, 0, 255]);
        let clear = Rgba([0, 0, 0, 0]);
        let image = RgbaImage::from_fn(8, 8, |x, _| if x < 4 { red } else { clear });

        let mut expected = image.clone();
        premultiply(expected.as_chunks_mut::<4>().0);
        let mut expected = imageops::resize(&expected, 5, 5, FilterType::Lanczos3);
        unpremultiply(expected.as_chunks_mut::<4>().0);

        let fitted = fit_square(image, 5);
        assert_eq!(fitted, expected);
        // The edge is partly transparent, but stays pure red: no dark fringe.
        assert!(fitted.pixels().any(|p| p[3] > 0 && p[3] < 255));
        let mut visible = fitted.pixels().filter(|p| p[3] > 0);
        assert!(visible.all(|p| p.0[..3] == [255, 0, 0]));
    }

    #[test]
    fn small_and_empty_images_fill_the_size() {
        let tall = RgbaImage::from_pixel(1, 4, Rgba([0, 0, 255, 255]));
        assert_eq!(fit_square(tall, 16).dimensions(), (16, 16));
        let empty = RgbaImage::new(0, 0);
        let fitted = fit_square(empty, 16);
        assert_eq!(fitted.dimensions(), (16, 16));
        assert!(fitted.pixels().all(|p| p[3] == 0));
    }
}
