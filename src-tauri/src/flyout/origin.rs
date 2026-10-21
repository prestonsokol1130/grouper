//! Where the flyout's open animation starts (DESIGN.md section 10: the
//! group's taskbar icon): the point on the flyout's taskbar-side edge
//! nearest the cursor that clicked it. Pure math, no Win32.

use serde::Serialize;
use specta::Type;

use super::layout::FlyoutSize;
use crate::taskbar::placement::Edge;

/// The animation origin, in logical pixels from the flyout's top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct FlyoutOrigin {
    // The taskbar's edge of the monitor, which is also the flyout side the
    // animation grows from. (Plain comments keep the generated bindings,
    // which count against the 250-line rule, short.)
    pub edge: Edge,
    pub x: u32,
    pub y: u32,
}

/// The origin for a flyout of logical `size` placed with its top-left at
/// `top_left` (physical) at `scale`, opened from `cursor` (physical) with
/// the taskbar on `edge`. Along the taskbar it follows the cursor, kept on
/// the flyout; across it, it sits on the taskbar-side edge.
pub fn origin(
    edge: Edge,
    cursor: (i32, i32),
    top_left: (i32, i32),
    size: FlyoutSize,
    scale: f64,
) -> FlyoutOrigin {
    let along = |cursor: i32, start: i32, length: u32| {
        let offset = (f64::from(cursor) - f64::from(start)) / sane(scale);
        // `as` saturates, and NaN becomes 0.
        (offset.round().max(0.0) as u32).min(length)
    };
    let x = along(cursor.0, top_left.0, size.width);
    let y = along(cursor.1, top_left.1, size.height);
    let (x, y) = match edge {
        Edge::Bottom => (x, size.height),
        Edge::Top => (x, 0),
        Edge::Left => (0, y),
        Edge::Right => (size.width, y),
    };
    FlyoutOrigin { edge, x, y }
}

fn sane(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: FlyoutSize = FlyoutSize {
        width: 232,
        height: 236,
    };

    fn at(edge: Edge, cursor: (i32, i32), scale: f64) -> (u32, u32) {
        let o = origin(edge, cursor, (1000, 500), SIZE, scale);
        assert_eq!(o.edge, edge);
        (o.x, o.y)
    }

    #[test]
    fn sits_on_the_taskbar_side_under_the_cursor() {
        assert_eq!(at(Edge::Bottom, (1116, 900), 1.0), (116, 236));
        assert_eq!(at(Edge::Top, (1116, 0), 1.0), (116, 0));
        assert_eq!(at(Edge::Left, (0, 600), 1.0), (0, 100));
        assert_eq!(at(Edge::Right, (3000, 600), 1.0), (232, 100));
    }

    #[test]
    fn converts_physical_offsets_with_the_scale() {
        assert_eq!(at(Edge::Bottom, (1150, 900), 1.5), (100, 236));
    }

    #[test]
    fn a_clamped_flyout_keeps_the_origin_on_itself() {
        assert_eq!(at(Edge::Bottom, (900, 900), 1.0), (0, 236));
        assert_eq!(at(Edge::Bottom, (5000, 900), 1.0), (232, 236));
        assert_eq!(at(Edge::Left, (0, i32::MIN), f64::NAN), (0, 0));
    }
}
