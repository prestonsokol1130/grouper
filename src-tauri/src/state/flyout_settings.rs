//! The flyout's own settings (DESIGN.md sections 7 and 8, changed per
//! Preston's feedback, 2026-10-20). Pure data. Plain `//` comments on the
//! types keep the generated bindings short.

use serde::{Deserialize, Serialize};
use specta::Type;

// "Flyout layout": columns x rows on one flyout page. 3 x 3 by default,
// paged like an iPhone folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub enum FlyoutGrid {
    #[default]
    #[serde(rename = "3x3")]
    G3x3,
    #[serde(rename = "4x4")]
    G4x4,
    #[serde(rename = "5x5")]
    G5x5,
    #[serde(rename = "4x2")]
    G4x2,
    #[serde(rename = "3x2")]
    G3x2,
    #[serde(rename = "2x2")]
    G2x2,
}

impl FlyoutGrid {
    /// Every option, in the order Settings lists them.
    pub const ALL: [Self; 6] = [
        Self::G3x3,
        Self::G4x4,
        Self::G5x5,
        Self::G4x2,
        Self::G3x2,
        Self::G2x2,
    ];

    /// `(columns, rows)` of one page.
    pub fn shape(self) -> (u32, u32) {
        match self {
            Self::G3x3 => (3, 3),
            Self::G4x4 => (4, 4),
            Self::G5x5 => (5, 5),
            Self::G4x2 => (4, 2),
            Self::G3x2 => (3, 2),
            Self::G2x2 => (2, 2),
        }
    }
}

// "Flyout icon size": match the taskbar's icons (default), or a fixed size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FlyoutIconSize {
    #[default]
    MatchTaskbar,
    Small,
    Medium,
    Large,
}

impl FlyoutIconSize {
    /// The fixed icon side in px (DESIGN.md section 7: 40, 48, 64), or
    /// `None` for [`Self::MatchTaskbar`], which is read from Windows.
    pub fn fixed_px(self) -> Option<u32> {
        match self {
            Self::MatchTaskbar => None,
            Self::Small => Some(40),
            Self::Medium => Some(48),
            Self::Large => Some(64),
        }
    }
}
