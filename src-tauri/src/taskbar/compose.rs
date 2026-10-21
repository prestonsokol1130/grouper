//! Composes a folder image: up to four app icons on the folder background.

use image::imageops;
use image::{GrayImage, Luma, Rgba, RgbaImage};

use super::squircle::{
    apply_mask, inset_rounded_square_mask, inset_squircle_mask, rounded_square_mask,
};
use super::style::{FolderStyle, Outline};
use crate::icons::pixels::fit_square;

/// Most icons a folder shows; the rest are left out.
pub const MAX_FOLDER_ICONS: usize = 4;

/// How much bigger than a grid cell a folder's only icon is drawn.
const SINGLE_ICON_SCALE: f32 = 1.6;

/// Where one icon is drawn: its top-left corner and side, in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub x: i64,
    pub y: i64,
    pub size: u32,
}

/// Draws a `style.canvas` x `style.canvas` folder image.
///
/// The layout depends on the number of icons ([`layout`]): none gives the
/// empty folder, the background with the outlined 2 x 2 slots of
/// [`draw_empty_slots`]; only the first four are used. Each icon is
/// fitted to its cell with Lanczos3 (on premultiplied pixels), its corners
/// rounded by `style.inner_icon_corner_ratio`, and drawn source-over
/// (straight alpha) on the background. The whole canvas is then masked by
/// the folder outline.
pub fn compose_folder(icons: &[RgbaImage], style: &FolderStyle) -> RgbaImage {
    let size = style.canvas;
    let mut canvas = RgbaImage::from_pixel(size, size, Rgba(style.background));
    if let Some((colour, width)) = style.border {
        draw_border(&mut canvas, style.outline, colour, width);
    }
    if icons.is_empty() {
        draw_empty_slots(&mut canvas, style);
    }
    let shown = &icons[..icons.len().min(MAX_FOLDER_ICONS)];
    for (icon, cell) in shown.iter().zip(layout(shown.len(), style)) {
        let mut fitted = fit_square(icon.clone(), cell.size);
        let radius = style.inner_icon_corner_ratio * cell.size as f32;
        apply_mask(&mut fitted, &rounded_square_mask(cell.size, radius));
        imageops::overlay(&mut canvas, &fitted, cell.x, cell.y);
    }
    apply_mask(&mut canvas, &outline_mask(size, style.outline, 0.0));
    canvas
}

/// The cells for `count` icons (more than four counts as four):
///
/// - 0: none.
/// - 1: one icon, 1.6 times a grid cell, in the centre.
/// - 2: side by side in the two grid columns, centred vertically.
/// - 3: two in the top grid row, one centred below.
/// - 4: the 2 x 2 grid.
///
/// The grid sits `padding_ratio * canvas` in from the edge, with
/// `gap_ratio * canvas` between its columns and rows.
pub fn layout(count: usize, style: &FolderStyle) -> Vec<Cell> {
    let canvas = style.canvas as f32;
    let padding = canvas * style.padding_ratio;
    let gap = canvas * style.gap_ratio;
    let cell = ((canvas - 2.0 * padding - gap) / 2.0).max(0.0);
    let near = padding;
    let far = padding + cell + gap;
    let centre = (canvas - cell) / 2.0;
    let at = |x: f32, y: f32| Cell {
        x: x.round() as i64,
        y: y.round() as i64,
        size: cell.round() as u32,
    };
    match count.min(MAX_FOLDER_ICONS) {
        0 => Vec::new(),
        1 => {
            let big = cell * SINGLE_ICON_SCALE;
            let offset = ((canvas - big) / 2.0).round() as i64;
            vec![Cell {
                x: offset,
                y: offset,
                size: big.round() as u32,
            }]
        }
        2 => vec![at(near, centre), at(far, centre)],
        3 => vec![at(near, near), at(far, near), at(centre, far)],
        _ => vec![at(near, near), at(far, near), at(near, far), at(far, far)],
    }
}

/// The mask of `outline` on a `size` canvas, pulled in by `inset` pixels.
fn outline_mask(size: u32, outline: Outline, inset: f32) -> GrayImage {
    match outline {
        Outline::Squircle(n) => inset_squircle_mask(size, n, inset),
        Outline::RoundedSquare(ratio) => {
            inset_rounded_square_mask(size, ratio * size as f32, inset)
        }
    }
}

/// Draws a `width`-pixel outline of `colour` just inside the folder's
/// edge. The final outline mask trims its outer side.
fn draw_border(canvas: &mut RgbaImage, outline: Outline, colour: [u8; 4], width: f32) {
    let size = canvas.width();
    let inset = if width.is_finite() {
        width.max(0.0)
    } else {
        0.0
    };
    let outer = GrayImage::from_pixel(size, size, Luma([255]));
    let inner = outline_mask(size, outline, inset);
    imageops::overlay(canvas, &ring(&outer, &inner, colour), 0, 0);
}

/// The empty folder's mark: each cell of the 2 x 2 grid drawn as an
/// outlined rounded slot (corners as the mini icons'), in
/// `style.empty_mark`'s colour with its stroke as a fraction of the canvas.
fn draw_empty_slots(canvas: &mut RgbaImage, style: &FolderStyle) {
    let (colour, stroke_ratio) = style.empty_mark;
    let stroke = stroke_ratio * style.canvas as f32;
    for cell in layout(MAX_FOLDER_ICONS, style) {
        let radius = style.inner_icon_corner_ratio * cell.size as f32;
        let outer = rounded_square_mask(cell.size, radius);
        let inner = inset_rounded_square_mask(cell.size, radius, stroke);
        imageops::overlay(canvas, &ring(&outer, &inner, colour), cell.x, cell.y);
    }
}

/// `colour` wherever `outer` covers and `inner` does not (the two masks
/// are the same size), clear elsewhere.
fn ring(outer: &GrayImage, inner: &GrayImage, colour: [u8; 4]) -> RgbaImage {
    let (width, height) = outer.dimensions();
    RgbaImage::from_fn(width, height, |x, y| {
        let covered = u32::from(outer.get_pixel(x, y)[0]);
        let inside = u32::from(inner.get_pixel(x, y)[0]);
        let coverage = covered.saturating_sub(inside);
        let alpha = (u32::from(colour[3]) * coverage + 127) / 255;
        let [r, g, b, _] = colour;
        Rgba([r, g, b, u8::try_from(alpha).unwrap_or(u8::MAX)])
    })
}
