//! Tests for [`super::compose`].

use image::{Rgba, RgbaImage};

use super::compose::{compose_folder, layout, Cell};
use super::style::{folder_style, FolderStyle, Outline, FOLDER_STYLE};
use crate::state::settings::FolderShape;

const RED: Rgba<u8> = Rgba([220, 40, 40, 255]);
const GREEN: Rgba<u8> = Rgba([40, 180, 70, 255]);
const BLUE: Rgba<u8> = Rgba([40, 90, 220, 255]);
const YELLOW: Rgba<u8> = Rgba([240, 200, 30, 255]);
const PURPLE: Rgba<u8> = Rgba([140, 60, 200, 255]);
const CYAN: Rgba<u8> = Rgba([30, 200, 210, 255]);

fn solid(colour: Rgba<u8>) -> RgbaImage {
    RgbaImage::from_pixel(256, 256, colour)
}

fn icons(colours: &[Rgba<u8>]) -> Vec<RgbaImage> {
    colours.iter().map(|&colour| solid(colour)).collect()
}

fn centre(cell: Cell) -> (u32, u32) {
    let half = i64::from(cell.size / 2);
    let x = u32::try_from(cell.x + half).unwrap();
    let y = u32::try_from(cell.y + half).unwrap();
    (x, y)
}

fn assert_corners_clear(image: &RgbaImage) {
    let last = image.width() - 1;
    for (x, y) in [(0, 0), (last, 0), (0, last), (last, last)] {
        assert_eq!(image.get_pixel(x, y)[3], 0, "corner ({x}, {y})");
    }
}

#[test]
fn output_is_canvas_sized_with_clear_corners_for_every_count() {
    let all = icons(&[RED, GREEN, BLUE, YELLOW, PURPLE, CYAN]);
    for count in 0..=all.len() {
        let image = compose_folder(&all[..count], &FOLDER_STYLE);
        assert_eq!(image.dimensions(), (256, 256), "{count} icons");
        assert_corners_clear(&image);
    }
}

#[test]
fn zero_icons_outline_four_empty_slots() {
    let image = compose_folder(&[], &FOLDER_STYLE);
    let background = Rgba(FOLDER_STYLE.background);
    // The gap between the slots and each slot's middle stay background.
    assert_eq!(*image.get_pixel(128, 128), background);
    for cell in layout(4, &FOLDER_STYLE) {
        let (x, y) = centre(cell);
        assert_eq!(*image.get_pixel(x, y), background, "slot middle");
        // The stroke (4% of 256 = 10 px) on the slot's left edge is grey.
        let edge = image.get_pixel(u32::try_from(cell.x + 4).unwrap(), y);
        assert!(edge[0] < 120 && edge[0] == edge[1], "slot edge {edge:?}");
        assert!(edge[3] > background[3], "slot edge {edge:?}");
    }
}

#[test]
fn folders_with_icons_have_no_empty_slots() {
    let one = compose_folder(&icons(&[RED]), &FOLDER_STYLE);
    // Where the top-left empty slot's edge would be, outside the one icon.
    assert_eq!(*one.get_pixel(40, 78), Rgba(FOLDER_STYLE.background));
}

#[test]
fn centre_of_each_cell_is_its_icon_colour() {
    let colours = [RED, GREEN, BLUE, YELLOW];
    for count in 1..=colours.len() {
        let image = compose_folder(&icons(&colours[..count]), &FOLDER_STYLE);
        let cells = layout(count, &FOLDER_STYLE);
        assert_eq!(cells.len(), count);
        for (cell, colour) in cells.into_iter().zip(colours) {
            let (x, y) = centre(cell);
            assert_eq!(*image.get_pixel(x, y), colour, "{count} icons");
        }
    }
}

#[test]
fn six_icons_use_only_the_first_four() {
    let six = compose_folder(
        &icons(&[RED, GREEN, BLUE, YELLOW, PURPLE, CYAN]),
        &FOLDER_STYLE,
    );
    let four = compose_folder(&icons(&[RED, GREEN, BLUE, YELLOW]), &FOLDER_STYLE);
    assert_eq!(six, four);
    assert!(!six.pixels().any(|p| *p == PURPLE || *p == CYAN));
}

#[test]
fn layouts_match_the_ticket() {
    let style = FOLDER_STYLE;
    // 256 * 0.14 = 35.84 padding, 15.36 gap, so cells are 84.48 px.
    let grid = layout(4, &style);
    let cell = |x, y| Cell { x, y, size: 84 };
    assert_eq!(
        grid,
        [cell(36, 36), cell(136, 36), cell(36, 136), cell(136, 136)]
    );
    // One icon: 1.6 cells, centred.
    let one = layout(1, &style)[0];
    assert_eq!(one.size, 135);
    assert_eq!((one.x, one.y), (60, 60));
    // Two: side by side, centred vertically.
    let two = layout(2, &style);
    assert_eq!((two[0].x, two[1].x), (36, 136));
    assert_eq!((two[0].y, two[1].y), (86, 86));
    // Three: two on top, one centred below.
    let three = layout(3, &style);
    assert_eq!((three[0].y, three[1].y, three[2].y), (36, 36, 136));
    assert_eq!(three[2].x, 86);
    assert_eq!(layout(9, &style), grid);
    assert!(layout(0, &style).is_empty());
}

#[test]
fn transparent_icon_edges_have_no_dark_fringe() {
    // A red disc on a transparent square.
    let disc = RgbaImage::from_fn(256, 256, |x, y| {
        let (dx, dy) = (f64::from(x) - 127.5, f64::from(y) - 127.5);
        if dx * dx + dy * dy < 100.0 * 100.0 {
            RED
        } else {
            Rgba([0, 0, 0, 0])
        }
    });
    let style = FolderStyle {
        background: [0, 0, 0, 0],
        border: None,
        ..FOLDER_STYLE
    };
    let image = compose_folder(&[disc], &style);
    for pixel in image.pixels().filter(|p| p[3] > 16) {
        assert!(pixel[0] > 180, "dark fringe pixel {pixel:?}");
    }
}

#[test]
fn border_draws_inside_the_edge() {
    for shape in [FolderShape::Square, FolderShape::Squircle] {
        let style = FolderStyle {
            border: Some(([0, 0, 0, 255], 4.0)),
            ..folder_style(shape)
        };
        let image = compose_folder(&[], &style);
        assert_eq!(*image.get_pixel(128, 1), Rgba([0, 0, 0, 255]), "{shape:?}");
        assert_eq!(*image.get_pixel(128, 128), Rgba(FOLDER_STYLE.background));
        assert_corners_clear(&image);
    }
}

#[test]
fn the_two_shapes_differ_only_near_the_corners() {
    let square = compose_folder(&[], &folder_style(FolderShape::Square));
    let squircle = compose_folder(&[], &folder_style(FolderShape::Squircle));
    assert_ne!(square, squircle);
    assert_eq!(square.get_pixel(128, 128), squircle.get_pixel(128, 128));
    // 18% corners keep (14, 14) inside the square; n = 5 cuts it.
    assert!(square.get_pixel(14, 14)[3] > squircle.get_pixel(14, 14)[3]);
}

#[test]
fn odd_styles_do_not_panic() {
    let tiny = FolderStyle {
        canvas: 1,
        ..FOLDER_STYLE
    };
    assert_eq!(compose_folder(&icons(&[RED]), &tiny).dimensions(), (1, 1));
    let zero = FolderStyle {
        canvas: 0,
        ..FOLDER_STYLE
    };
    assert_eq!(compose_folder(&icons(&[RED]), &zero).dimensions(), (0, 0));
    let cramped = FolderStyle {
        padding_ratio: 0.6,
        outline: Outline::Squircle(f32::NAN),
        border: Some(([0, 0, 0, 255], f32::NAN)),
        ..FOLDER_STYLE
    };
    let image = compose_folder(&icons(&[RED, GREEN]), &cramped);
    assert_eq!(image.dimensions(), (256, 256));
    let odd_square = FolderStyle {
        outline: Outline::RoundedSquare(f32::NAN),
        ..cramped
    };
    let image = compose_folder(&icons(&[RED]), &odd_square);
    assert_eq!(image.dimensions(), (256, 256));
}
