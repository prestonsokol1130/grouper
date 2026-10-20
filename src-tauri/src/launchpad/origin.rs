//! Where the Launchpad's open animation starts (DESIGN.md section 10): the
//! centre bottom of the screen, or the hot corner that opened it. Pure.

use super::Opener;
use crate::flyout::origin::FlyoutOrigin;
use crate::hot_corners::dwell::Corner;
use crate::taskbar::placement::{Edge, Rect};

/// The origin for a Launchpad covering `monitor` (physical) at `scale`,
/// in logical pixels from its top-left. `edge` is the side the genie
/// grows from: the bottom for the shortcut, the top for a corner.
pub fn launchpad_origin(opener: Opener, monitor: Rect, scale: f64) -> FlyoutOrigin {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let logical = |physical: i32| {
        // `as` saturates, and NaN becomes 0.
        (f64::from(physical.max(0)) / scale).round() as u32
    };
    let width = logical(monitor.right.saturating_sub(monitor.left));
    let height = logical(monitor.bottom.saturating_sub(monitor.top));
    let (edge, x, y) = match opener {
        Opener::Shortcut => (Edge::Bottom, width / 2, height),
        Opener::Corner(Corner::TopLeft) => (Edge::Top, 0, 0),
        Opener::Corner(Corner::TopRight) => (Edge::Top, width, 0),
    };
    FlyoutOrigin { edge, x, y }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: Rect = Rect {
        left: 1920,
        top: 0,
        right: 4480,
        bottom: 1440,
    };

    #[test]
    fn shortcut_starts_at_the_centre_bottom_in_logical_pixels() {
        let origin = launchpad_origin(Opener::Shortcut, MON, 1.25);
        assert_eq!(
            origin,
            FlyoutOrigin {
                edge: Edge::Bottom,
                x: 1024,
                y: 1152
            }
        );
    }

    #[test]
    fn corners_start_at_their_corner() {
        let left = launchpad_origin(Opener::Corner(Corner::TopLeft), MON, 1.0);
        assert_eq!(
            left,
            FlyoutOrigin {
                edge: Edge::Top,
                x: 0,
                y: 0
            }
        );
        let right = launchpad_origin(Opener::Corner(Corner::TopRight), MON, 2.0);
        assert_eq!(
            right,
            FlyoutOrigin {
                edge: Edge::Top,
                x: 1280,
                y: 0
            }
        );
    }

    #[test]
    fn a_bad_scale_counts_as_one() {
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let origin = launchpad_origin(Opener::Shortcut, MON, scale);
            assert_eq!((origin.x, origin.y), (1280, 1440), "{scale}");
        }
    }
}
