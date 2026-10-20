//! Where a group flyout goes on screen: pure math, no Win32.
//!
//! Everything is in physical pixels except the flyout size and margin,
//! which are logical (CSS) pixels converted with the monitor's scale. The
//! Win32 side ([`crate::win32::monitor`]) gathers a [`MonitorInfo`] for the
//! monitor under the cursor; the flyout window (S2-06) feeds it to
//! [`place`].
//!
//! All arithmetic runs in `i64` and saturates back into `i32`, and
//! non-finite or negative sizes, margins and scales are treated as zero
//! (or a scale of 1), so no input can panic or overflow.

use serde::Serialize;
use specta::Type;

/// A rectangle in physical pixels; `right` and `bottom` are exclusive, as
/// in Win32's `RECT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    /// True if the two rectangles share at least one pixel.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }
}

/// The monitor edge the taskbar is docked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

/// One monitor, as Win32 reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorInfo {
    /// The whole monitor.
    pub monitor: Rect,
    /// The monitor minus docked (not auto-hidden) taskbars and app bars.
    pub work: Rect,
    /// Effective DPI / 96 (1.0, 1.25, 1.5, 2.0, ...).
    pub scale: f64,
    /// The taskbar's own rectangle, when known. Only consulted while the
    /// work area fills the monitor (auto-hide), where it picks the edge and
    /// is kept clear so the flyout never sits under the shown taskbar. A
    /// rectangle that misses this monitor is ignored.
    pub taskbar: Option<Rect>,
}

/// The edge the taskbar is docked to: the side where the work area is inset
/// from the monitor (the largest inset wins). With no inset (auto-hide) the
/// [`MonitorInfo::taskbar`] rectangle decides, and without one it is
/// [`Edge::Bottom`].
pub fn taskbar_edge(m: &MonitorInfo) -> Edge {
    let insets = [
        (
            Edge::Bottom,
            i64::from(m.monitor.bottom) - i64::from(m.work.bottom),
        ),
        (Edge::Top, i64::from(m.work.top) - i64::from(m.monitor.top)),
        (
            Edge::Left,
            i64::from(m.work.left) - i64::from(m.monitor.left),
        ),
        (
            Edge::Right,
            i64::from(m.monitor.right) - i64::from(m.work.right),
        ),
    ];
    let mut best = (Edge::Bottom, 0_i64);
    for inset in insets {
        if inset.1 > best.1 {
            best = inset;
        }
    }
    if best.1 > 0 {
        return best.0;
    }
    match auto_hide_bar(m) {
        Some(bar) => bar_edge(&bar, &m.monitor),
        None => Edge::Bottom,
    }
}

/// Top-left corner, in physical pixels, for a flyout of `size_logical`
/// (width, height) opened from `cursor` on monitor `m`.
///
/// The flyout sits against the taskbar: above it for a bottom taskbar and
/// below it for a top one, centred horizontally on the cursor; beside it
/// for a left or right taskbar, centred vertically on the cursor. It is
/// then clamped to stay `margin_logical` inside the work area (and clear of
/// an auto-hide taskbar). A flyout too big for an axis is pinned to that
/// axis's top or left edge plus the margin.
pub fn place(
    cursor: (i32, i32),
    size_logical: (f64, f64),
    m: &MonitorInfo,
    margin_logical: f64,
) -> (i32, i32) {
    let scale = sanitize_scale(m.scale);
    let width = to_physical(size_logical.0, scale);
    let height = to_physical(size_logical.1, scale);
    let margin = to_physical(margin_logical, scale);
    let area = usable_area(m);
    let (cx, cy) = (i64::from(cursor.0), i64::from(cursor.1));

    let (x, y) = match taskbar_edge(m) {
        Edge::Bottom => (cx - width / 2, area.bottom - margin - height),
        Edge::Top => (cx - width / 2, area.top + margin),
        Edge::Left => (area.left + margin, cy - height / 2),
        Edge::Right => (area.right - margin - width, cy - height / 2),
    };
    let x = clamp_axis(x, area.left, area.right, width, margin);
    let y = clamp_axis(y, area.top, area.bottom, height, margin);
    (saturate(x), saturate(y))
}

/// The height, in physical pixels, of the area [`place`] keeps the
/// flyout inside: the work area, minus an auto-hide taskbar at the top or
/// bottom (one at the side leaves the height alone). Never negative.
pub fn usable_height(m: &MonitorInfo) -> i32 {
    let area = usable_area(m);
    saturate((area.bottom - area.top).max(0))
}

/// The rectangle the flyout must stay inside, widened to `i64`.
struct Area {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

/// The work area, minus the auto-hide taskbar's band when there is one.
fn usable_area(m: &MonitorInfo) -> Area {
    let work = m.work;
    let mut area = Area {
        left: i64::from(work.left),
        top: i64::from(work.top),
        right: i64::from(work.right),
        bottom: i64::from(work.bottom),
    };
    if let Some(bar) = auto_hide_bar(m) {
        match bar_edge(&bar, &m.monitor) {
            Edge::Bottom => area.bottom = area.bottom.min(i64::from(bar.top)),
            Edge::Top => area.top = area.top.max(i64::from(bar.bottom)),
            Edge::Left => area.left = area.left.max(i64::from(bar.right)),
            Edge::Right => area.right = area.right.min(i64::from(bar.left)),
        }
    }
    area
}

/// The taskbar rectangle, if the work area fills the monitor (auto-hide)
/// and the rectangle is on this monitor.
fn auto_hide_bar(m: &MonitorInfo) -> Option<Rect> {
    if m.work != m.monitor {
        return None;
    }
    m.taskbar.filter(|bar| bar.intersects(&m.monitor))
}

/// Which monitor edge a taskbar rectangle hugs: a wide bar is at the top or
/// bottom, a tall one at the left or right, whichever side it is nearer.
fn bar_edge(bar: &Rect, monitor: &Rect) -> Edge {
    let width = i64::from(bar.right) - i64::from(bar.left);
    let height = i64::from(bar.bottom) - i64::from(bar.top);
    if width >= height {
        let above = i64::from(bar.top) - i64::from(monitor.top);
        let below = i64::from(monitor.bottom) - i64::from(bar.bottom);
        if above < below {
            Edge::Top
        } else {
            Edge::Bottom
        }
    } else {
        let left = i64::from(bar.left) - i64::from(monitor.left);
        let right = i64::from(monitor.right) - i64::from(bar.right);
        if left < right {
            Edge::Left
        } else {
            Edge::Right
        }
    }
}

/// Keeps `pos..pos + size` inside `lo + margin..hi - margin`. If it cannot
/// fit, pins it to `lo + margin`.
fn clamp_axis(pos: i64, lo: i64, hi: i64, size: i64, margin: i64) -> i64 {
    let min = lo + margin;
    let max = hi - margin - size;
    if max < min {
        min
    } else {
        pos.clamp(min, max)
    }
}

/// A usable scale: non-finite or non-positive scales become 1.0.
fn sanitize_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// Logical to physical pixels, rounded. NaN, infinite and negative values
/// become 0; huge values saturate at `i32::MAX`.
fn to_physical(logical: f64, scale: f64) -> i64 {
    let physical = logical * scale;
    if !physical.is_finite() || physical <= 0.0 {
        return 0;
    }
    // `as` saturates for floats, and the bound keeps sums far from i64::MAX.
    (physical.round() as i64).min(i64::from(i32::MAX))
}

fn saturate(value: i64) -> i32 {
    let clamped = value.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
    // Always fits after the clamp; the fallback is never used.
    i32::try_from(clamped).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}
