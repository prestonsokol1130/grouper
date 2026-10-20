//! The flyout's size for a group: pure math, no Win32.
//!
//! Every number lives in [`LAYOUT`], taken from DESIGN.md section 7 and 8
//! and the frontend's `AppTile` (S4-01), so a design change is one edit.

use serde::Serialize;
use specta::Type;

use crate::state::settings::IconSize;

/// The flyout's sizing constants, in logical (CSS) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlyoutLayout {
    /// The grid never gets wider than this many columns.
    pub max_columns: u32,
    /// One app cell's width: the icon plus the tile's side inset.
    pub cell_width: u32,
    /// One app cell's height: inset, icon, label gap, label line, inset.
    pub cell_height: u32,
    /// Space between cells, both ways.
    pub gap: u32,
    /// Space between the content and the flyout's edge, on every side.
    pub padding: u32,
    /// The group name row above the grid, including its space below.
    pub header: u32,
    /// Gap kept between the flyout and the taskbar or screen edge.
    pub margin: u32,
}

/// DESIGN.md: 3 across, padding 12, gap 8, a Subtitle (28px line) group
/// name, 12px off the taskbar. A cell is `AppTile` at the Medium flyout
/// icon (48px): 48 + 2 * 8 inset wide, and 8 + 48 + 4 + 16 (Caption
/// line) + 8 tall.
pub const LAYOUT: FlyoutLayout = FlyoutLayout {
    max_columns: 3,
    cell_width: 64,
    cell_height: 84,
    gap: 8,
    padding: 12,
    header: 28 + 8,
    margin: 12,
};

/// [`LAYOUT`] for the "Icon size" setting: a cell is the flyout icon plus
/// the same inset (2 * 8 wide) and inset, label gap and label (36 tall).
pub fn layout_for(icon: IconSize) -> FlyoutLayout {
    let px = icon.flyout_px();
    FlyoutLayout {
        cell_width: px + 16,
        cell_height: px + 36,
        ..LAYOUT
    }
}

/// A flyout size in logical (CSS) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct FlyoutSize {
    pub width: u32,
    pub height: u32,
}

/// Grid columns for `apps` apps: `min(max_columns, apps)`, at least 1. An
/// empty group gets the full width so its message has room.
pub fn columns(apps: usize, layout: &FlyoutLayout) -> u32 {
    let max = layout.max_columns.max(1);
    if apps == 0 {
        return max;
    }
    u32::try_from(apps).unwrap_or(max).clamp(1, max)
}

/// Grid rows for `apps` apps laid out in [`columns`] columns; at least 1.
pub fn rows(apps: usize, layout: &FlyoutLayout) -> u32 {
    let cols = u64::from(columns(apps, layout));
    let apps = u64::try_from(apps).unwrap_or(u64::MAX);
    let rows = apps.div_ceil(cols).max(1);
    u32::try_from(rows).unwrap_or(u32::MAX)
}

/// `cells` cells of `cell` with `gap` between them. Saturates.
fn span(cells: u32, cell: u32, gap: u32) -> u32 {
    cells
        .saturating_mul(cell)
        .saturating_add(cells.saturating_sub(1).saturating_mul(gap))
}

/// The flyout size for a group of `apps` apps: header and grid plus
/// padding. Saturates at `u32::MAX` rather than overflowing.
pub fn flyout_size(apps: usize, layout: &FlyoutLayout) -> FlyoutSize {
    let edges = layout.padding.saturating_mul(2);
    let grid_width = span(columns(apps, layout), layout.cell_width, layout.gap);
    let grid_height = span(rows(apps, layout), layout.cell_height, layout.gap);
    FlyoutSize {
        width: grid_width.saturating_add(edges),
        height: grid_height
            .saturating_add(layout.header)
            .saturating_add(edges),
    }
}

/// `size` with its height capped so the flyout fits an area `work_height`
/// physical pixels tall (placement's usable height) at `scale`, `margin`
/// logical pixels off each end; the frontend scrolls the rest. Never below
/// one row.
pub fn fit_height(
    size: FlyoutSize,
    work_height: i32,
    scale: f64,
    layout: &FlyoutLayout,
) -> FlyoutSize {
    let scale = sane_scale(scale);
    let work = (f64::from(work_height.max(0)) / scale).floor();
    // `as` saturates for floats, so a huge work area cannot wrap.
    let room = (work as u32).saturating_sub(layout.margin.saturating_mul(2));
    let one_row = flyout_size(1, layout).height;
    FlyoutSize {
        width: size.width,
        height: size.height.min(room.max(one_row)),
    }
}

fn sane_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// Logical to physical pixels at `scale`, rounded, at least 1 pixel. A
/// non-finite or non-positive scale counts as 1.0.
pub fn to_physical(logical: u32, scale: f64) -> u32 {
    let physical = (f64::from(logical) * sane_scale(scale)).round();
    if !physical.is_finite() || physical < 1.0 {
        return 1;
    }
    // `as` saturates for floats; the bound keeps it inside i32 for Win32.
    (physical as u32).min(i32::MAX.unsigned_abs())
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
