//! Tests for [`super`], the flyout size math.

use super::*;
use crate::state::settings::FlyoutGrid;

#[test]
fn pages_hold_columns_times_rows() {
    let expected = [(0, 1), (1, 1), (9, 1), (10, 2), (18, 2), (19, 3), (20, 3)];
    for (apps, p) in expected {
        assert_eq!(pages(apps, &LAYOUT), p, "{apps} apps");
    }
    let two = layout_for(FlyoutGrid::G2x2, 48);
    assert_eq!(pages(5, &two), 2);
    assert_eq!(pages(usize::MAX, &LAYOUT), u32::MAX);
}

#[test]
fn the_window_is_one_full_page() {
    let size = |apps| {
        let s = flyout_size(apps, &LAYOUT);
        (s.width, s.height)
    };
    // 12 + 3 * 64 + 2 * 8 + 12 wide; 12 + 36 + 3 * 84 + 2 * 8 + 12 tall.
    assert_eq!(size(1), (232, 328));
    assert_eq!(size(0), (232, 328));
    assert_eq!(size(9), (232, 328));
    // A second page adds the 32px dots row, nothing else.
    assert_eq!(size(10), (232, 360));
    assert_eq!(size(20), (232, 360));
}

#[test]
fn every_layout_option_sizes_the_window() {
    // (option, icon) -> (columns, rows, width, height) for 30 apps.
    let cases = [
        (FlyoutGrid::G3x3, 48, (3, 3, 232, 360)),
        (FlyoutGrid::G4x4, 48, (4, 4, 304, 452)),
        (FlyoutGrid::G5x5, 48, (5, 5, 376, 544)),
        (FlyoutGrid::G4x2, 48, (4, 2, 304, 268)),
        (FlyoutGrid::G3x2, 48, (3, 2, 232, 268)),
        (FlyoutGrid::G2x2, 48, (2, 2, 160, 268)),
        (FlyoutGrid::G3x3, 40, (3, 3, 208, 336)),
        (FlyoutGrid::G3x3, 64, (3, 3, 280, 408)),
    ];
    for (grid, icon, want) in cases {
        let layout = layout_for(grid, icon);
        let size = flyout_size(30, &layout);
        let got = (layout.columns, layout.rows, size.width, size.height);
        assert_eq!(got, want, "{grid:?} at {icon}px");
    }
    assert_eq!(layout_for(FlyoutGrid::G3x3, 48), LAYOUT);
    assert_eq!(FlyoutGrid::ALL.len(), 6);
}

#[test]
fn huge_values_do_not_overflow() {
    let huge = layout_for(FlyoutGrid::G5x5, u32::MAX);
    assert_eq!(flyout_size(usize::MAX, &huge).height, u32::MAX);
    assert_eq!(flyout_size(usize::MAX, &huge).width, u32::MAX);
}

#[test]
fn rows_shrink_to_fit_the_work_area() {
    let five = layout_for(FlyoutGrid::G5x5, 64);
    // Five rows of 100 plus chrome is 12 + 36 + 532 + 32 + 12 = 624 tall.
    assert_eq!(flyout_size(60, &five).height, 624);
    assert_eq!(fit_rows(five, 60, 2000, 1.0), five);
    // 1032 physical at 1.5 is 688 logical, minus 2 * 12: still fits.
    assert_eq!(fit_rows(five, 60, 1032, 1.5).rows, 5);
    // 600 logical minus 24 = 576 leaves room for four rows (516).
    assert_eq!(fit_rows(five, 60, 600, 1.0).rows, 4);
    // A tiny or broken work area still leaves one row.
    assert_eq!(fit_rows(five, 60, 10, 1.0).rows, 1);
    assert_eq!(fit_rows(five, 60, -5, f64::NAN).rows, 1);
    assert_eq!(fit_rows(five, 60, 10, 1.0).columns, 5);
}

#[test]
fn an_auto_hide_bottom_bar_shrinks_the_room_and_a_side_bar_does_not() {
    use crate::taskbar::placement::{usable_height, MonitorInfo, Rect};
    let rect = |left, top, right, bottom| Rect {
        left,
        top,
        right,
        bottom,
    };
    let screen = rect(0, 0, 3840, 2160);
    let auto_hide = |taskbar| MonitorInfo {
        monitor: screen,
        work: screen,
        scale: 1.5,
        taskbar: Some(taskbar),
    };
    let tall = layout_for(FlyoutGrid::G5x5, 64);
    let bottom = auto_hide(rect(0, 2088, 3840, 2160));
    assert_eq!(usable_height(&bottom), 2088);
    // floor(2088 / 2.0) - 2 * 12 = 1020 fits all five rows; at 4.0 only
    // floor(2088 / 4.0) - 24 = 498 is left: three rows (408).
    assert_eq!(fit_rows(tall, 99, usable_height(&bottom), 2.0).rows, 5);
    assert_eq!(fit_rows(tall, 99, usable_height(&bottom), 4.0).rows, 3);
    let side = auto_hide(rect(0, 0, 72, 2160));
    assert_eq!(usable_height(&side), 2160);
    // floor(2160 / 4.0) - 24 = 516: exactly four rows.
    assert_eq!(fit_rows(tall, 99, usable_height(&side), 4.0).rows, 4);
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
