//! Puts a Store app's white-only icon on its tile colour.
//!
//! Some Store apps (ChatGPT, for one) ship only a white glyph for dark
//! themes and leave the colour to the tile behind it. On Grouper's light
//! folder and tiles such a glyph would vanish, so it gets a rounded plate
//! in the tile colour the app declares, the way older Start menus drew it.

use image::{imageops, Rgba, RgbaImage};

use super::pixels::fit_square;
use crate::taskbar::squircle::{apply_mask, rounded_square_mask};

/// Plate colour when an app declares none: the grey Windows uses.
pub const DEFAULT_PLATE: [u8; 3] = [0x76, 0x76, 0x76];

/// Part of the plate's side the glyph fills.
const GLYPH_PERCENT: u32 = 62;

/// Plate corner radius, as a fraction of its side.
const CORNER_RATIO: f32 = 0.22;

/// Pixels with at least this alpha count as part of the glyph.
const SOLID_ALPHA: u8 = 128;

/// Part of the solid pixels that must be light (a small coloured badge,
/// such as a "beta" mark, is allowed).
const LIGHT_PERCENT: u64 = 75;

/// Most of its box a glyph fills; a white tile icon (Microsoft Store,
/// Notion) fills far more and keeps its own look.
const MAX_FILL_PERCENT: u64 = 60;

/// A channel at least this bright counts as white.
const LIGHT: u8 = 200;

/// `icon` unchanged unless it is a light glyph ([`is_light_glyph`]); then
/// the glyph, `GLYPH_PERCENT` of the side, centred on a `size` x `size`
/// rounded plate of colour `plate` (RGB), or [`DEFAULT_PLATE`] if `None`.
pub fn plate_light_glyph(icon: RgbaImage, size: u32, plate: Option<[u8; 3]>) -> RgbaImage {
    if !is_light_glyph(&icon) {
        return icon;
    }
    let [r, g, b] = plate.unwrap_or(DEFAULT_PLATE);
    let mut canvas = RgbaImage::from_pixel(size, size, Rgba([r, g, b, u8::MAX]));
    let inner = u32::try_from(u64::from(size) * u64::from(GLYPH_PERCENT) / 100)
        .unwrap_or(size)
        .max(1);
    let glyph = fit_square(super::trim::crop_to_content(icon), inner);
    let offset = i64::from((size - inner.min(size)) / 2);
    imageops::overlay(&mut canvas, &glyph, offset, offset);
    apply_mask(
        &mut canvas,
        &rounded_square_mask(size, CORNER_RATIO * size as f32),
    );
    canvas
}

/// True when most solid pixels of `icon` are white or light grey and they
/// fill at most `MAX_FILL_PERCENT` of their box: a line glyph made to sit
/// on a coloured tile. An icon with no solid pixel is not one.
pub fn is_light_glyph(icon: &RgbaImage) -> bool {
    let (mut solid, mut light) = (0_u64, 0_u64);
    for px in icon.pixels() {
        if px[3] < SOLID_ALPHA {
            continue;
        }
        solid += 1;
        if px[0] >= LIGHT && px[1] >= LIGHT && px[2] >= LIGHT {
            light += 1;
        }
    }
    let Some((_, _, width, height)) = super::trim::content_box(icon) else {
        return false;
    };
    let area = u64::from(width) * u64::from(height);
    solid > 0 && light * 100 >= solid * LIGHT_PERCENT && solid * 100 <= area * MAX_FILL_PERCENT
}

/// The RGB of a `System.Tile.Background` value (`0xAABBGGRR`); `None` when
/// it is fully transparent.
pub fn tile_rgb(value: u32) -> Option<[u8; 3]> {
    let [r, g, b, a] = value.to_le_bytes();
    (a != 0).then_some([r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ring: a glyph that fills about half its box.
    fn glyph(colour: [u8; 4]) -> RgbaImage {
        RgbaImage::from_fn(64, 64, |x, y| {
            let ring = |v: u32| (16..20).contains(&v) || (44..48).contains(&v);
            let inside = (16..48).contains(&x) && (16..48).contains(&y);
            if inside && (ring(x) || ring(y)) {
                Rgba(colour)
            } else {
                Rgba([0, 0, 0, 0])
            }
        })
    }

    #[test]
    fn a_white_glyph_gets_a_plate() {
        let out = plate_light_glyph(glyph([255, 255, 255, 255]), 128, Some([0x31, 0x43, 0xff]));
        assert_eq!(out.dimensions(), (128, 128));
        // An edge pixel is the plate colour; a corner is cut away.
        assert_eq!(out.get_pixel(64, 4).0, [0x31, 0x43, 0xff, 255]);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        // The glyph's ring is white, its hole the plate.
        assert!(out.get_pixel(64, 29).0.iter().all(|&c| c >= 240));
        assert_eq!(out.get_pixel(64, 64).0, [0x31, 0x43, 0xff, 255]);
    }

    #[test]
    fn a_white_tile_icon_is_left_alone() {
        let tile = RgbaImage::from_pixel(64, 64, Rgba([250, 250, 250, 255]));
        assert_eq!(plate_light_glyph(tile.clone(), 64, None), tile);
    }

    #[test]
    fn a_coloured_icon_is_left_alone() {
        let orange = glyph([217, 119, 87, 255]);
        assert_eq!(plate_light_glyph(orange.clone(), 128, None), orange);
        assert!(!is_light_glyph(&RgbaImage::new(8, 8)));
    }

    #[test]
    fn tile_colours_are_read_as_abgr() {
        assert_eq!(tile_rgb(0xffff_4331), Some([0x31, 0x43, 0xff]));
        assert_eq!(tile_rgb(0x00ff_ffff), None);
    }
}
