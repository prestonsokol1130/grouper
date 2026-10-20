//! How a taskbar folder icon looks.
//!
//! The values in [`FOLDER_STYLE`] are placeholders until `DESIGN.md`
//! settles the look; that one constant is the only place to change them.

/// The look of a composed folder icon. Ratios are fractions of `canvas`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FolderStyle {
    /// Pixels per side of the composed image.
    pub canvas: u32,
    /// The superellipse exponent `n` of the folder outline,
    /// `|x|^n + |y|^n <= 1`: 2 is a circle, larger is squarer.
    pub corner_exponent: f32,
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
}

/// The folder style Grouper uses. Placeholder values until `DESIGN.md`
/// exists; S3-02 changes them here.
pub const FOLDER_STYLE: FolderStyle = FolderStyle {
    canvas: 256,
    corner_exponent: 5.0,
    // Translucent light grey.
    background: [235, 235, 240, 230],
    border: None,
    padding_ratio: 0.14,
    gap_ratio: 0.06,
    inner_icon_corner_ratio: 0.22,
};

impl Default for FolderStyle {
    fn default() -> Self {
        FOLDER_STYLE
    }
}

#[cfg(test)]
mod tests {
    use super::{FolderStyle, FOLDER_STYLE};

    #[test]
    fn default_is_the_const_table() {
        assert_eq!(FolderStyle::default(), FOLDER_STYLE);
        assert_eq!(FOLDER_STYLE.canvas, 256);
    }
}
