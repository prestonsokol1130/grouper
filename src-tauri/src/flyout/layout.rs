//! The flyout's size for a group: pure math, no Win32.
//!
//! The flyout shows fixed pages of columns x rows apps (the "Flyout
//! layout" setting, 3 x 3 by default), paged like an iPhone folder. The
//! window is one page big; page dots below the grid appear only when the
//! group needs more than one page. Every spacing number lives in
//! [`LAYOUT`] (DESIGN.md section 8 and the frontend's `AppTile`), so a
//! design change is one edit.

use serde::Serialize;
use specta::Type;

use crate::state::settings::FlyoutGrid;

/// The flyout's sizing, in logical (CSS) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlyoutLayout {
    /// Apps across and down on one page.
    pub columns: u32,
    pub rows: u32,
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
    /// The page dots row below the grid (only with more than one page).
    pub dots: u32,
    /// Gap kept between the flyout and the taskbar or screen edge.
    pub margin: u32,
}

/// Width `AppTile` adds to the icon: an 8px inset on each side.
const CELL_EXTRA_WIDTH: u32 = 16;
/// Height `AppTile` adds: 8 inset, 4 label gap, 16 Caption line, 8 inset.
const CELL_EXTRA_HEIGHT: u32 = 36;

/// DESIGN.md: the default 3 x 3 page of Medium (48px) icons, padding 12,
/// gap 8, a Subtitle (28px line) group name with 8 below it, a 32px dots
/// row (room for the optional 24px page arrows), 12px off the taskbar.
pub const LAYOUT: FlyoutLayout = FlyoutLayout {
    columns: 3,
    rows: 3,
    cell_width: 48 + CELL_EXTRA_WIDTH,
    cell_height: 48 + CELL_EXTRA_HEIGHT,
    gap: 8,
    padding: 12,
    header: 28 + 8,
    dots: 32,
    margin: 12,
};

/// [`LAYOUT`] for the "Flyout layout" setting and an icon of `icon_px`.
pub fn layout_for(grid: FlyoutGrid, icon_px: u32) -> FlyoutLayout {
    let (columns, rows) = grid.shape();
    FlyoutLayout {
        columns,
        rows,
        cell_width: icon_px.saturating_add(CELL_EXTRA_WIDTH),
        cell_height: icon_px.saturating_add(CELL_EXTRA_HEIGHT),
        ..LAYOUT
    }
}

/// A flyout size in logical (CSS) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct FlyoutSize {
    pub width: u32,
    pub height: u32,
}

/// Apps one page holds: columns x rows, at least 1.
pub fn per_page(layout: &FlyoutLayout) -> u64 {
    (u64::from(layout.columns) * u64::from(layout.rows)).max(1)
}

/// Pages `apps` apps need; an empty group still has one page.
pub fn pages(apps: usize, layout: &FlyoutLayout) -> u32 {
    let apps = u64::try_from(apps).unwrap_or(u64::MAX);
    let pages = apps.div_ceil(per_page(layout)).max(1);
    u32::try_from(pages).unwrap_or(u32::MAX)
}

/// `cells` cells of `cell` with `gap` between them. Saturates.
fn span(cells: u32, cell: u32, gap: u32) -> u32 {
    cells
        .saturating_mul(cell)
        .saturating_add(cells.saturating_sub(1).saturating_mul(gap))
}

/// The flyout size for a group of `apps` apps: one full page of cells,
/// the header, padding, and the dots row when there is more than one
/// page. Saturates at `u32::MAX` rather than overflowing.
pub fn flyout_size(apps: usize, layout: &FlyoutLayout) -> FlyoutSize {
    let edges = layout.padding.saturating_mul(2);
    let grid_width = span(layout.columns.max(1), layout.cell_width, layout.gap);
    let grid_height = span(layout.rows.max(1), layout.cell_height, layout.gap);
    let dots = if pages(apps, layout) > 1 {
        layout.dots
    } else {
        0
    };
    FlyoutSize {
        width: grid_width.saturating_add(edges),
        height: grid_height
            .saturating_add(layout.header)
            .saturating_add(dots)
            .saturating_add(edges),
    }
}

/// `layout` with as many of its rows as fit an area `work_height`
/// physical pixels tall (placement's usable height) at `scale`, `margin`
/// logical pixels off each end. Never below one row; fewer rows just
/// means more pages.
pub fn fit_rows(layout: FlyoutLayout, apps: usize, work_height: i32, scale: f64) -> FlyoutLayout {
    let scale = sane_scale(scale);
    let work = (f64::from(work_height.max(0)) / scale).floor();
    // `as` saturates for floats, so a huge work area cannot wrap.
    let room = (work as u32).saturating_sub(layout.margin.saturating_mul(2));
    let mut fitted = layout;
    while fitted.rows > 1 && flyout_size(apps, &fitted).height > room {
        fitted.rows -= 1;
    }
    fitted.rows = fitted.rows.max(1);
    fitted
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
