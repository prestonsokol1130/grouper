//! How a taskbar folder icon looks (DESIGN.md section 5).
//!
//! [`FOLDER_STYLE`] is the Windows square recipe; [`folder_style`] swaps
//! in the iPhone squircle outline for the "Folder shape" setting. Those
//! two places are the only ones to change the look.

use crate::state::settings::FolderShape;

/// The outline of the folder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    /// A rounded square; the corner radius as a fraction of the canvas.
    RoundedSquare(f32),
    /// The superellipse `|x|^n + |y|^n <= 1` with exponent `n`: 2 is a
    /// circle, larger is squarer.
    Squircle(f32),
}

/// The look of a composed folder icon. Ratios are fractions of `canvas`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FolderStyle {
    /// Pixels per side of the composed image.
    pub canvas: u32,
    /// The folder's outline.
    pub outline: Outline,
    /// Straight-alpha RGBA fill of the folder.
    pub background: [u8; 4],
    /// An optional outline: straight-alpha RGBA colour and width in canvas
    /// pixels, drawn just inside the folder's edge.
    pub border: Option<([u8; 4], f32)>,
    /// Space between the folder's edge and the icon grid.
    pub padding_ratio: f32,
    /// Space between the two columns and the two rows of the grid.
    pub gap_ratio: f32,
    /// Corner radius of each inner icon, as a fraction of its cell's side.
    pub inner_icon_corner_ratio: f32,
    /// An empty folder's outlined 2 x 2 slots: straight-alpha RGBA colour
    /// and stroke width as a fraction of `canvas`.
    pub empty_mark: ([u8; 4], f32),
}

/// DESIGN.md section 5: the Windows square (corner radius 18% of the
/// tile) on the neutral `rgba(235,235,240,0.92)` the taskbar `.ico` uses,
/// with a 1px `rgba(0,0,0,0.12)` inner border so it reads on light and
/// dark taskbars. Padding 14%, gap 6%, mini icon corners 22%. An empty
/// folder outlines its four slots in the light `--text-secondary` grey,
/// `rgba(0,0,0,0.606)`, which reads on the fixed light tile.
pub const FOLDER_STYLE: FolderStyle = FolderStyle {
    canvas: 256,
    outline: Outline::RoundedSquare(0.18),
    background: [235, 235, 240, 235],
    border: Some(([0, 0, 0, 31], 1.0)),
    padding_ratio: 0.14,
    gap_ratio: 0.06,
    inner_icon_corner_ratio: 0.22,
    empty_mark: ([0, 0, 0, 155], 0.04),
};

/// The iPhone squircle exponent (DESIGN.md section 5).
pub const SQUIRCLE_EXPONENT: f32 = 5.0;

/// The style for the "Folder shape" setting.
pub fn folder_style(shape: FolderShape) -> FolderStyle {
    match shape {
        FolderShape::Square => FOLDER_STYLE,
        FolderShape::Squircle => FolderStyle {
            outline: Outline::Squircle(SQUIRCLE_EXPONENT),
            ..FOLDER_STYLE
        },
    }
}

impl Default for FolderStyle {
    fn default() -> Self {
        FOLDER_STYLE
    }
}

#[cfg(test)]
mod tests {
    use super::{folder_style, FolderStyle, Outline, FOLDER_STYLE};
    use crate::state::settings::FolderShape;

    #[test]
    fn default_is_the_windows_square() {
        assert_eq!(FolderStyle::default(), FOLDER_STYLE);
        assert_eq!(FOLDER_STYLE.canvas, 256);
        assert_eq!(folder_style(FolderShape::Square), FOLDER_STYLE);
    }

    #[test]
    fn squircle_changes_only_the_outline() {
        let squircle = folder_style(FolderShape::Squircle);
        assert_eq!(squircle.outline, Outline::Squircle(5.0));
        assert_eq!(
            FolderStyle {
                outline: FOLDER_STYLE.outline,
                ..squircle
            },
            FOLDER_STYLE
        );
    }
}
