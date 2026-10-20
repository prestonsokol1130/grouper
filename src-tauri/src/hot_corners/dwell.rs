//! Whether the cursor is resting in a hot corner: pure, no Win32.
//!
//! A corner is the [`CORNER_PX`]-square at the top-left or top-right of a
//! monitor. It fires once the cursor has stayed in the same corner of the
//! same monitor for [`DWELL`], and not again until the cursor leaves it.

use std::time::{Duration, Instant};

use crate::state::HotCorners;
use crate::taskbar::placement::Rect;

/// How long the cursor must rest in a corner (DESIGN.md section 12).
pub const DWELL: Duration = Duration::from_millis(250);

/// The corner's side, in physical pixels. The cursor stops at the outer
/// edges of the desktop, so a small square is easy to hit there.
pub const CORNER_PX: i32 = 2;

/// A screen corner that can open the Launchpad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
}

/// The enabled corner of `monitor` that `cursor` is in, if any.
pub fn corner_at(cursor: (i32, i32), monitor: Rect, enabled: HotCorners) -> Option<Corner> {
    let (x, y) = cursor;
    if y < monitor.top || y >= monitor.top + CORNER_PX {
        return None;
    }
    if enabled.top_left && x >= monitor.left && x < monitor.left + CORNER_PX {
        return Some(Corner::TopLeft);
    }
    if enabled.top_right && x < monitor.right && x >= monitor.right - CORNER_PX {
        return Some(Corner::TopRight);
    }
    None
}

/// The dwell timer for the corner the cursor is in.
#[derive(Debug, Default)]
pub struct Dwell {
    /// The corner and monitor the cursor is in, and since when.
    current: Option<(Corner, Rect, Instant)>,
    /// This stay in the corner has already fired.
    fired: bool,
}

impl Dwell {
    /// Feeds one cursor sample taken at `now`: the corner it is in (and
    /// its monitor), or `None`. Returns the corner when this sample
    /// completes a dwell.
    pub fn step(&mut self, now: Instant, hit: Option<(Corner, Rect)>) -> Option<Corner> {
        let Some((corner, monitor)) = hit else {
            self.reset();
            return None;
        };
        match self.current {
            Some((c, m, since)) if c == corner && m == monitor => {
                let rested = now.saturating_duration_since(since) >= DWELL;
                if rested && !self.fired {
                    self.fired = true;
                    return Some(corner);
                }
                None
            }
            _ => {
                self.current = Some((corner, monitor, now));
                self.fired = false;
                None
            }
        }
    }

    /// Forgets the corner, as if the cursor had left it.
    pub fn reset(&mut self) {
        self.current = None;
        self.fired = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);
    const MON: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };
    const RIGHT_MON: Rect = Rect {
        left: 1920,
        top: -200,
        right: 4480,
        bottom: 1240,
    };
    const BOTH: HotCorners = HotCorners {
        top_left: true,
        top_right: true,
    };

    #[test]
    fn finds_each_corner_of_each_monitor() {
        assert_eq!(corner_at((0, 0), MON, BOTH), Some(Corner::TopLeft));
        assert_eq!(corner_at((1, 1), MON, BOTH), Some(Corner::TopLeft));
        assert_eq!(corner_at((1919, 0), MON, BOTH), Some(Corner::TopRight));
        assert_eq!(
            corner_at((1920, -200), RIGHT_MON, BOTH),
            Some(Corner::TopLeft)
        );
        assert_eq!(
            corner_at((4479, -199), RIGHT_MON, BOTH),
            Some(Corner::TopRight)
        );
    }

    #[test]
    fn outside_the_square_is_no_corner() {
        assert_eq!(corner_at((2, 0), MON, BOTH), None);
        assert_eq!(corner_at((0, 2), MON, BOTH), None);
        assert_eq!(corner_at((1917, 0), MON, BOTH), None);
        assert_eq!(corner_at((960, 0), MON, BOTH), None);
        assert_eq!(corner_at((0, 1079), MON, BOTH), None);
        assert_eq!(corner_at((1920, 0), MON, BOTH), None, "right is exclusive");
    }

    #[test]
    fn disabled_corners_never_match() {
        let left_only = HotCorners {
            top_left: true,
            top_right: false,
        };
        assert_eq!(corner_at((1919, 0), MON, left_only), None);
        assert_eq!(corner_at((0, 0), MON, left_only), Some(Corner::TopLeft));
        let none = HotCorners {
            top_left: false,
            top_right: false,
        };
        assert_eq!(corner_at((0, 0), MON, none), None);
    }

    #[test]
    fn fires_after_resting_for_the_dwell() {
        let t = Instant::now();
        let mut d = Dwell::default();
        let hit = Some((Corner::TopLeft, MON));
        assert_eq!(d.step(t, hit), None);
        assert_eq!(d.step(t + 200 * MS, hit), None);
        assert_eq!(d.step(t + DWELL, hit), Some(Corner::TopLeft));
    }

    #[test]
    fn fires_once_until_the_cursor_leaves() {
        let t = Instant::now();
        let mut d = Dwell::default();
        let hit = Some((Corner::TopRight, MON));
        d.step(t, hit);
        assert_eq!(d.step(t + 300 * MS, hit), Some(Corner::TopRight));
        assert_eq!(d.step(t + 600 * MS, hit), None);
        assert_eq!(d.step(t + 5000 * MS, hit), None);
        assert_eq!(d.step(t + 5050 * MS, None), None);
        assert_eq!(d.step(t + 5100 * MS, hit), None);
        assert_eq!(d.step(t + 5350 * MS, hit), Some(Corner::TopRight));
    }

    #[test]
    fn passing_through_never_fires() {
        let t = Instant::now();
        let mut d = Dwell::default();
        let hit = Some((Corner::TopLeft, MON));
        for i in 0..10 {
            let at = t + i * 100 * MS;
            let sample = if i % 2 == 0 { hit } else { None };
            assert_eq!(d.step(at, sample), None);
        }
    }

    #[test]
    fn another_corner_or_monitor_restarts_the_timer() {
        let t = Instant::now();
        let mut d = Dwell::default();
        d.step(t, Some((Corner::TopRight, MON)));
        let next = Some((Corner::TopLeft, RIGHT_MON));
        assert_eq!(d.step(t + 200 * MS, next), None);
        assert_eq!(d.step(t + 300 * MS, next), None);
        assert_eq!(d.step(t + 450 * MS, next), Some(Corner::TopLeft));
    }

    #[test]
    fn reset_rearms() {
        let t = Instant::now();
        let mut d = Dwell::default();
        let hit = Some((Corner::TopLeft, MON));
        d.step(t, hit);
        assert_eq!(d.step(t + DWELL, hit), Some(Corner::TopLeft));
        d.reset();
        d.step(t + DWELL + MS, hit);
        assert_eq!(d.step(t + 2 * DWELL + MS, hit), Some(Corner::TopLeft));
    }
}
