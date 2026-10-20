//! Composes a folder image: up to four app icons on a squircle background.

use image::imageops;
use image::{Rgba, RgbaImage};

use super::squircle::{apply_mask, inset_squircle_mask, rounded_square_mask, squircle_mask};
use super::style::FolderStyle;
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
/// empty folder background; only the first four are used. Each icon is
/// fitted to its cell with Lanczos3 (on premultiplied pixels), its corners
/// rounded by `style.inner_icon_corner_ratio`, and drawn source-over
/// (straight alpha) on the background. The whole canvas is then masked by
/// the squircle outline.
pub fn compose_folder(icons: &[RgbaImage], style: &FolderStyle) -> RgbaImage {
    let size = style.canvas;
    let mut canvas = RgbaImage::from_pixel(size, size, Rgba(style.background));
    if let Some((colour, width)) = style.border {
        draw_border(&mut canvas, style.corner_exponent, colour, width);
    }
    let shown = &icons[..icons.len().min(MAX_FOLDER_ICONS)];
    for (icon, cell) in shown.iter().zip(layout(shown.len(), style)) {
        let mut fitted = fit_square(icon.clone(), cell.size);
        let radius = style.inner_icon_corner_ratio * cell.size as f32;
        apply_mask(&mut fitted, &rounded_square_mask(cell.size, radius));
        imageops::overlay(&mut canvas, &fitted, cell.x, cell.y);
    }
    apply_mask(&mut canvas, &squircle_mask(size, style.corner_exponent));
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

/// Draws a `width`-pixel outline of `colour` just inside the squircle's
/// edge. The final squircle mask trims its outer side.
fn draw_border(canvas: &mut RgbaImage, n: f32, colour: [u8; 4], width: f32) {
    let size = canvas.width();
    let inner = inset_squircle_mask(size, n, width.max(0.0));
    let ring = RgbaImage::from_fn(size, size, |x, y| {
        let outside_inner = 255 - u32::from(inner.get_pixel(x, y)[0]);
        let alpha = (u32::from(colour[3]) * outside_inner + 127) / 255;
        let [r, g, b, _] = colour;
        Rgba([r, g, b, u8::try_from(alpha).unwrap_or(u8::MAX)])
    });
    imageops::overlay(canvas, &ring, 0, 0);
}
