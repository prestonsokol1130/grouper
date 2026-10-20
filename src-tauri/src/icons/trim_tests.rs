//! Tests for [`super`], the icon normaliser, on small sample bitmaps.

use image::{Rgba, RgbaImage};

use super::*;

const RED: Rgba<u8> = Rgba([200, 10, 10, 255]);
const CLEAR: Rgba<u8> = Rgba([0, 0, 0, 0]);

/// A `side` x `side` image with an opaque `w` x `h` box at `(x, y)`.
fn boxed(side: u32, (x, y, w, h): (u32, u32, u32, u32)) -> RgbaImage {
    RgbaImage::from_fn(side, side, |px, py| {
        let inside = (x..x + w).contains(&px) && (y..y + h).contains(&py);
        if inside {
            RED
        } else {
            CLEAR
        }
    })
}

/// A `side` x `side` image with an opaque `inner` px square centred.
fn padded(side: u32, inner: u32) -> RgbaImage {
    let start = (side - inner) / 2;
    boxed(side, (start, start, inner, inner))
}

/// The 90% fill at 256 px, and where it starts.
const WANT: u32 = 256 * FILL_PERCENT / 100;
const START: u32 = (256 - WANT) / 2;

fn assert_fills(out: &RgbaImage, width: u32, height: u32) {
    let (x, y, w, h) = content_box(out).unwrap();
    assert!(
        w.abs_diff(width) <= 2 && h.abs_diff(height) <= 2,
        "{w}x{h}, want {width}x{height}"
    );
    // Centred: as much space left as right, above as below.
    assert!((256 - x - w).abs_diff(x) <= 2 && (256 - y - h).abs_diff(y) <= 2);
}

#[test]
fn a_small_icon_is_scaled_up_to_the_fill() {
    let out = trim_margin(padded(256, 128), 256);
    assert_eq!(out.dimensions(), (256, 256));
    assert_fills(&out, WANT, WANT);
}

#[test]
fn a_full_bleed_icon_gets_the_same_padding() {
    let out = trim_margin(padded(256, 256), 256);
    assert_fills(&out, WANT, WANT);
    assert_eq!(out.get_pixel(START - 2, 128)[3], 0);
    assert_eq!(out.get_pixel(128, 128), &RED);
}

#[test]
fn off_centre_content_is_centred_and_keeps_its_aspect_ratio() {
    // 100 x 50 in the top-left corner.
    let out = trim_margin(boxed(256, (4, 10, 100, 50)), 256);
    assert_fills(&out, WANT, WANT / 2);
}

#[test]
fn faint_shadows_do_not_count_as_content() {
    let mut image = padded(256, 128);
    for (x, y, px) in image.enumerate_pixels_mut() {
        // A soft halo 20 px around the box.
        let near = (44..212).contains(&x) && (44..212).contains(&y);
        if near && px[3] == 0 {
            *px = Rgba([0, 0, 0, 24]);
        }
    }
    assert_fills(&trim_margin(image, 256), WANT, WANT);
}

/// The Shell's jumbo frame as cached for Cursor: a 38-alpha black ring,
/// a 77-alpha white ring and a fainter one, a clear gap, and the real
/// 144 px icon in the middle.
fn shell_framed() -> RgbaImage {
    let mut image = padded(256, 144);
    for (x, y, px) in image.enumerate_pixels_mut() {
        let ring = x.min(y).min(255 - x).min(255 - y);
        *px = match ring {
            0 | 1 => Rgba([0, 0, 0, 38]),
            2 => Rgba([255, 255, 255, 77]),
            3 => Rgba([255, 255, 255, 26]),
            _ => *px,
        };
    }
    image
}

#[test]
fn the_shell_frame_is_dropped_and_the_icon_fills() {
    let framed = shell_framed();
    assert_eq!(frame_width(&framed), Some(4));
    let out = trim_margin(framed, 256);
    assert_fills(&out, WANT, WANT);
    assert_eq!(out.get_pixel(0, 128)[3], 0, "the frame is gone");
}

#[test]
fn solid_or_clear_edges_are_no_frame() {
    assert_eq!(frame_width(&padded(64, 64)), None);
    assert_eq!(frame_width(&padded(64, 32)), None);
    assert_eq!(frame_width(&RgbaImage::new(0, 0)), None);
    // A faint edge with no clear gap inside it is part of the artwork.
    let glow = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 60]));
    assert_eq!(frame_width(&glow), None);
}

#[test]
fn an_opaque_plate_icon_keeps_its_plate() {
    // Cursor-style: a dark full-bleed plate with a light glyph in it.
    let dark = Rgba([20, 18, 16, 255]);
    let plate = RgbaImage::from_fn(256, 256, |x, y| {
        let glyph = (100..156).contains(&x) && (100..156).contains(&y);
        if glyph {
            Rgba([230, 230, 230, 255])
        } else {
            dark
        }
    });
    let out = trim_margin(plate, 256);
    assert_fills(&out, WANT, WANT);
    assert_eq!(out.get_pixel(START + 4, START + 4), &dark);
    assert_eq!(out.get_pixel(128, 128).0, [230, 230, 230, 255]);
}

#[test]
fn an_empty_icon_stays_empty() {
    let out = trim_margin(RgbaImage::new(64, 64), 256);
    assert_eq!(out.dimensions(), (256, 256));
    assert!(content_box(&out).is_none());
}

/// Normalises copies of real cached icons for a visual check:
/// `GROUPER_NORMALIZE_DIR=<dir> cargo test -- --ignored normalise_a_folder`
/// writes `<dir>/after/<name>.png` for every `<dir>/*.png`.
#[test]
#[ignore = "manual visual check on copied icons"]
fn normalise_a_folder() {
    let dir = std::path::PathBuf::from(std::env::var("GROUPER_NORMALIZE_DIR").unwrap());
    let after = dir.join("after");
    std::fs::create_dir_all(&after).unwrap();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "png") {
            let image = image::open(&path).unwrap().to_rgba8();
            let out = trim_margin(image, 256);
            out.save(after.join(path.file_name().unwrap())).unwrap();
        }
    }
}
