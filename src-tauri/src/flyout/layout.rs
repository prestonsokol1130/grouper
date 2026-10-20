//! The flyout's size for a group: pure math, no Win32.
//!
//! Every number lives in [`LAYOUT`] so the design ticket (DESIGN.md, S4-04)
//! can change them in one place.

use serde::Serialize;
use specta::Type;

/// The flyout's sizing constants, in logical (CSS) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlyoutLayout {
    /// The grid never gets wider than this many columns.
    pub max_columns: u32,
    /// The side of one square app cell.
    pub cell: u32,
    /// Space between the grid and the flyout's edge, on every side.
    pub padding: u32,
    /// Gap kept between the flyout and the taskbar or screen edge.
    pub margin: u32,
}

/// The sizing in use until DESIGN.md replaces it.
pub const LAYOUT: FlyoutLayout = FlyoutLayout {
    max_columns: 4,
    cell: 88,
    padding: 16,
    margin: 12,
};

/// A flyout size in logical (CSS) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct FlyoutSize {
    pub width: u32,
    pub height: u32,
}

/// Grid columns for `apps` apps: `min(max_columns, ceil(sqrt(apps)))`, and
/// at least 1 so an empty group still gets a one-cell flyout.
pub fn columns(apps: usize, layout: &FlyoutLayout) -> u32 {
    let mut side: u64 = 0;
    // Integer ceil(sqrt(n)): the smallest side with side * side >= n.
    while side.saturating_mul(side) < apps as u64 {
        side += 1;
        if side >= u64::from(layout.max_columns) {
            break;
        }
    }
    let max = layout.max_columns.max(1);
    u32::try_from(side).unwrap_or(max).clamp(1, max)
}

/// Grid rows for `apps` apps laid out in [`columns`] columns; at least 1.
pub fn rows(apps: usize, layout: &FlyoutLayout) -> u32 {
    let cols = u64::from(columns(apps, layout));
    let rows = (apps as u64).div_ceil(cols).max(1);
    u32::try_from(rows).unwrap_or(u32::MAX)
}

/// The flyout size for a group of `apps` apps: the grid plus padding.
/// Saturates at `u32::MAX` rather than overflowing.
pub fn flyout_size(apps: usize, layout: &FlyoutLayout) -> FlyoutSize {
    let grid = |cells: u32| {
        cells
            .saturating_mul(layout.cell)
            .saturating_add(layout.padding.saturating_mul(2))
    };
    FlyoutSize {
        width: grid(columns(apps, layout)),
        height: grid(rows(apps, layout)),
    }
}

/// Logical to physical pixels at `scale`, rounded, at least 1 pixel. A
/// non-finite or non-positive scale counts as 1.0.
pub fn to_physical(logical: u32, scale: f64) -> u32 {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let physical = (f64::from(logical) * scale).round();
    if !physical.is_finite() || physical < 1.0 {
        return 1;
    }
    // `as` saturates for floats; the bound keeps it inside i32 for Win32.
    (physical as u32).min(i32::MAX as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_follow_ceil_sqrt_capped_at_four() {
        let expected = [
            (0, 1),
            (1, 1),
            (2, 2),
            (4, 2),
            (5, 3),
            (9, 3),
            (10, 4),
            (16, 4),
            (17, 4),
            (100, 4),
        ];
        for (apps, cols) in expected {
            assert_eq!(columns(apps, &LAYOUT), cols, "{apps} apps");
        }
    }

    #[test]
    fn rows_fill_the_columns() {
        let expected = [(0, 1), (1, 1), (2, 1), (3, 2), (5, 2), (7, 3), (17, 5)];
        for (apps, r) in expected {
            assert_eq!(rows(apps, &LAYOUT), r, "{apps} apps");
        }
    }

    #[test]
    fn size_is_cells_plus_padding() {
        let one = flyout_size(1, &LAYOUT);
        assert_eq!((one.width, one.height), (120, 120));
        let four = flyout_size(4, &LAYOUT);
        assert_eq!((four.width, four.height), (208, 208));
        let seven = flyout_size(7, &LAYOUT);
        assert_eq!((seven.width, seven.height), (296, 296));
        let ten = flyout_size(10, &LAYOUT);
        assert_eq!((ten.width, ten.height), (384, 296));
    }

    #[test]
    fn other_layouts_are_respected() {
        let three = FlyoutLayout {
            max_columns: 3,
            ..LAYOUT
        };
        assert_eq!(columns(16, &three), 3);
        assert_eq!(rows(16, &three), 6);
        let zero = FlyoutLayout {
            max_columns: 0,
            ..LAYOUT
        };
        assert_eq!(columns(5, &zero), 1);
        assert_eq!(rows(5, &zero), 5);
    }

    #[test]
    fn huge_counts_do_not_overflow() {
        assert_eq!(columns(usize::MAX, &LAYOUT), 4);
        assert!(rows(usize::MAX, &LAYOUT) > 1);
        assert_eq!(flyout_size(usize::MAX, &LAYOUT).height, u32::MAX);
    }

    #[test]
    fn physical_rounding() {
        assert_eq!(to_physical(120, 1.0), 120);
        assert_eq!(to_physical(120, 1.25), 150);
        assert_eq!(to_physical(208, 1.5), 312);
        assert_eq!(to_physical(120, f64::NAN), 120);
        assert_eq!(to_physical(120, -2.0), 120);
        assert_eq!(to_physical(0, 1.0), 1);
        assert_eq!(to_physical(120, f64::INFINITY), 120);
        assert_eq!(to_physical(u32::MAX, 4.0), i32::MAX as u32);
    }
}
