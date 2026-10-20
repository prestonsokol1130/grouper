//! Tests for [`super::placement`].

use super::placement::{place, taskbar_edge, Edge, MonitorInfo, Rect};

const EDGES: [Edge; 4] = [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right];
const SCALES: [f64; 4] = [1.0, 1.25, 1.5, 2.0];
/// Flyout size and margin in logical pixels.
const SIZE: (f64, f64) = (360.0, 300.0);
const MARGIN: f64 = 12.0;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
    Rect {
        left,
        top,
        right,
        bottom,
    }
}

fn px(logical: f64, scale: f64) -> i32 {
    (logical * scale).round() as i32
}

/// The band a 48-logical-px taskbar takes on `edge` of `monitor`.
fn bar(monitor: Rect, edge: Edge, scale: f64) -> Rect {
    let (t, mut band) = (px(48.0, scale), monitor);
    match edge {
        Edge::Bottom => band.top = monitor.bottom - t,
        Edge::Top => band.bottom = monitor.top + t,
        Edge::Left => band.right = monitor.left + t,
        Edge::Right => band.left = monitor.right - t,
    }
    band
}

/// A monitor with a docked taskbar on `edge`: the work area excludes it.
fn docked(monitor: Rect, edge: Edge, scale: f64) -> MonitorInfo {
    let b = bar(monitor, edge, scale);
    let mut work = monitor;
    match edge {
        Edge::Bottom => work.bottom = b.top,
        Edge::Top => work.top = b.bottom,
        Edge::Left => work.left = b.right,
        Edge::Right => work.right = b.left,
    }
    MonitorInfo {
        monitor,
        work,
        scale,
        taskbar: None,
    }
}

/// A monitor with an auto-hide taskbar on `edge`: work fills the monitor.
fn auto_hide(monitor: Rect, edge: Option<Edge>, scale: f64) -> MonitorInfo {
    MonitorInfo {
        monitor,
        work: monitor,
        scale,
        taskbar: edge.map(|edge| bar(monitor, edge, scale)),
    }
}

/// The area the flyout must stay inside, with the margin applied.
fn allowed(m: &MonitorInfo) -> Rect {
    // An auto-hide taskbar takes the same band a docked one would.
    let area = match m.taskbar {
        Some(_) if m.work == m.monitor => docked(m.monitor, taskbar_edge(m), m.scale).work,
        _ => m.work,
    };
    let margin = px(MARGIN, m.scale);
    rect(
        area.left + margin,
        area.top + margin,
        area.right - margin,
        area.bottom - margin,
    )
}

fn corners(r: Rect) -> [(i32, i32); 5] {
    let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    [
        (r.left, r.top),
        (r.right - 1, r.top),
        (r.left, r.bottom - 1),
        (r.right - 1, r.bottom - 1),
        (cx, cy),
    ]
}

/// Places the flyout from each corner and the centre of the monitor and
/// checks it stays inside, hugs the taskbar and centres on the cursor
/// when there is room.
fn check_matrix(m: &MonitorInfo) {
    let edge = taskbar_edge(m);
    let (w, h) = (px(SIZE.0, m.scale), px(SIZE.1, m.scale));
    let inside = allowed(m);
    for cursor in corners(m.monitor) {
        let (x, y) = place(cursor, SIZE, m, MARGIN);
        let ctx = format!("{edge:?} x{} cursor {cursor:?} -> ({x}, {y})", m.scale);
        assert!(x >= inside.left && x + w <= inside.right, "{ctx}");
        assert!(y >= inside.top && y + h <= inside.bottom, "{ctx}");
        match edge {
            Edge::Bottom => assert_eq!(y + h, inside.bottom, "{ctx}"),
            Edge::Top => assert_eq!(y, inside.top, "{ctx}"),
            Edge::Left => assert_eq!(x, inside.left, "{ctx}"),
            Edge::Right => assert_eq!(x + w, inside.right, "{ctx}"),
        }
        let centred_x = cursor.0 - w / 2;
        let centred_y = cursor.1 - h / 2;
        let fits_x = centred_x >= inside.left && centred_x + w <= inside.right;
        let fits_y = centred_y >= inside.top && centred_y + h <= inside.bottom;
        if matches!(edge, Edge::Bottom | Edge::Top) && fits_x {
            assert_eq!(x, centred_x, "{ctx}");
        }
        if matches!(edge, Edge::Left | Edge::Right) && fits_y {
            assert_eq!(y, centred_y, "{ctx}");
        }
    }
}

fn primary(scale: f64) -> Rect {
    rect(0, 0, px(1920.0, scale), px(1080.0, scale))
}

#[test]
fn edge_follows_the_inset_side_of_the_work_area() {
    for edge in EDGES {
        assert_eq!(taskbar_edge(&docked(primary(1.0), edge, 1.0)), edge);
    }
}

#[test]
fn auto_hide_defaults_to_bottom_and_follows_a_known_taskbar() {
    assert_eq!(
        taskbar_edge(&auto_hide(primary(1.0), None, 1.0)),
        Edge::Bottom
    );
    for edge in EDGES {
        let m = auto_hide(primary(1.5), Some(edge), 1.5);
        assert_eq!(taskbar_edge(&m), edge);
    }
    // A taskbar on another monitor is ignored.
    let mut m = auto_hide(primary(1.0), Some(Edge::Top), 1.0);
    m.taskbar = Some(rect(-1920, 0, 0, 48));
    assert_eq!(taskbar_edge(&m), Edge::Bottom);
}

#[test]
fn matrix_of_edges_scales_and_corners_on_the_primary_monitor() {
    for scale in SCALES {
        for edge in EDGES {
            check_matrix(&docked(primary(scale), edge, scale));
        }
    }
}

#[test]
fn matrix_on_a_secondary_monitor_at_negative_coordinates() {
    for scale in SCALES {
        let monitor = rect(-px(2560.0, scale), -px(300.0, scale), 0, px(1140.0, scale));
        for edge in EDGES {
            let m = docked(monitor, edge, scale);
            check_matrix(&m);
            let (x, y) = place((-10, -10), SIZE, &m, MARGIN);
            assert!(x < 0, "{edge:?} x{scale}: x {x} stays on the left monitor");
            assert!(y < px(1140.0, scale));
        }
    }
}

#[test]
fn matrix_with_auto_hide_keeps_clear_of_the_taskbar() {
    for scale in SCALES {
        check_matrix(&auto_hide(primary(scale), None, scale));
        for edge in EDGES {
            let m = auto_hide(primary(scale), Some(edge), scale);
            check_matrix(&m);
            let b = bar(m.monitor, edge, scale);
            for cursor in corners(m.monitor) {
                let (x, y) = place(cursor, SIZE, &m, MARGIN);
                let (w, h) = (px(SIZE.0, scale), px(SIZE.1, scale));
                let flyout = rect(x, y, x + w, y + h);
                assert!(!flyout.intersects(&b), "{edge:?} x{scale} {cursor:?}");
            }
        }
    }
}

#[test]
fn exact_position_above_a_bottom_taskbar_at_150_percent() {
    let m = docked(primary(1.5), Edge::Bottom, 1.5);
    // Work bottom 1620 - 72 = 1548; minus margin 18 and height 450.
    assert_eq!(m.work.bottom, 1548);
    assert_eq!(place((1000, 1600), SIZE, &m, MARGIN), (730, 1080));
    // Near the right corner: clamped to 2880 - 18 - 540.
    assert_eq!(place((2870, 1600), SIZE, &m, MARGIN), (2322, 1080));
}

#[test]
fn oversized_flyout_is_pinned_top_left_plus_margin() {
    for edge in EDGES {
        let m = docked(primary(2.0), edge, 2.0);
        let (x, y) = place((500, 500), (5000.0, 5000.0), &m, MARGIN);
        assert_eq!((x, y), (m.work.left + 24, m.work.top + 24), "{edge:?}");
    }
    // Too wide only: x pinned, y still hugs the bottom taskbar.
    let m = docked(primary(1.0), Edge::Bottom, 1.0);
    let (x, y) = place((900, 1000), (4000.0, 300.0), &m, MARGIN);
    assert_eq!((x, y), (12, m.work.bottom - 12 - 300));
}

#[test]
fn bad_inputs_never_panic_and_stay_sane() {
    let m = docked(primary(1.0), Edge::Bottom, 1.0);
    let bottom = m.work.bottom - 12;
    // NaN, negative and infinite sizes count as zero.
    assert_eq!(
        place((100, 100), (f64::NAN, -5.0), &m, MARGIN),
        (100, bottom)
    );
    assert_eq!(
        place((100, 100), (f64::INFINITY, f64::NEG_INFINITY), &m, MARGIN),
        (100, bottom)
    );
    // A bad margin counts as zero; a bad scale counts as 1.0.
    let edge_y = m.work.bottom - 300;
    assert_eq!(place((960, 0), SIZE, &m, f64::NAN), (780, edge_y));
    for scale in [f64::NAN, 0.0, -2.0, f64::INFINITY] {
        let bad = MonitorInfo { scale, ..m };
        assert_eq!(place((960, 0), SIZE, &bad, 0.0), (780, edge_y));
    }
    // Extreme coordinates and a huge margin neither overflow nor panic.
    let huge = MonitorInfo {
        monitor: rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX),
        work: rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX - 1),
        scale: 2.0,
        taskbar: None,
    };
    let _ = place((i32::MAX, i32::MIN), (1e300, 1e300), &huge, 1e300);
    let _ = place((i32::MIN, i32::MAX), SIZE, &huge, MARGIN);
    let empty = MonitorInfo {
        monitor: rect(0, 0, 0, 0),
        work: rect(10, 10, -10, -10),
        scale: 1.0,
        taskbar: Some(rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX)),
    };
    assert_eq!(place((0, 0), SIZE, &empty, MARGIN), (22, 22));
}
