//! Tests for [`super`], the flyout size math.

use super::*;

#[test]
fn columns_are_up_to_three() {
    let expected = [(0, 3), (1, 1), (2, 2), (3, 3), (4, 3), (5, 3), (100, 3)];
    for (apps, cols) in expected {
        assert_eq!(columns(apps, &LAYOUT), cols, "{apps} apps");
    }
}

#[test]
fn rows_grow_downward() {
    let expected = [(0, 1), (1, 1), (3, 1), (4, 2), (6, 2), (7, 3), (17, 6)];
    for (apps, r) in expected {
        assert_eq!(rows(apps, &LAYOUT), r, "{apps} apps");
    }
}

#[test]
fn size_is_header_grid_and_padding() {
    let size = |apps| {
        let s = flyout_size(apps, &LAYOUT);
        (s.width, s.height)
    };
    // 12 + 64 + 12 wide; 12 + 36 + 84 + 12 tall.
    assert_eq!(size(1), (88, 144));
    assert_eq!(size(2), (160, 144));
    assert_eq!(size(3), (232, 144));
    // Two rows: 84 + 8 + 84.
    assert_eq!(size(5), (232, 236));
    assert_eq!(size(9), (232, 328));
    assert_eq!(size(0), (232, 144));
}

#[test]
fn icon_sizes_change_the_cell() {
    assert_eq!(layout_for(IconSize::Medium), LAYOUT);
    let large = layout_for(IconSize::Large);
    assert_eq!((large.cell_width, large.cell_height), (80, 100));
    assert_eq!(
        flyout_size(3, &layout_for(IconSize::Small)).width,
        3 * 56 + 16 + 24
    );
}

#[test]
fn other_layouts_are_respected() {
    let four = FlyoutLayout {
        max_columns: 4,
        ..LAYOUT
    };
    assert_eq!(columns(16, &four), 4);
    assert_eq!(rows(16, &four), 4);
    let zero = FlyoutLayout {
        max_columns: 0,
        ..LAYOUT
    };
    assert_eq!(columns(5, &zero), 1);
    assert_eq!(rows(5, &zero), 5);
}

#[test]
fn huge_counts_do_not_overflow() {
    assert_eq!(columns(usize::MAX, &LAYOUT), 3);
    assert!(rows(usize::MAX, &LAYOUT) > 1);
    assert_eq!(flyout_size(usize::MAX, &LAYOUT).height, u32::MAX);
}

#[test]
fn tall_flyouts_are_capped_to_the_work_area() {
    let tall = flyout_size(30, &LAYOUT);
    // 1032 physical at 1.5 is 688 logical, minus 2 * 12.
    assert_eq!(fit_height(tall, 1032, 1.5, &LAYOUT).height, 664);
    assert_eq!(fit_height(tall, 1032, 1.5, &LAYOUT).width, tall.width);
    let short = flyout_size(5, &LAYOUT);
    assert_eq!(fit_height(short, 1032, 1.0, &LAYOUT), short);
    // A tiny or broken work area still leaves one row.
    assert_eq!(fit_height(tall, 10, 1.0, &LAYOUT).height, 144);
    assert_eq!(fit_height(tall, -5, f64::NAN, &LAYOUT).height, 144);
    assert_eq!(fit_height(tall, i32::MAX, 1.0, &LAYOUT), tall);
}

#[test]
fn an_auto_hide_bottom_bar_shrinks_the_cap_and_a_side_bar_does_not() {
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
    let tall = flyout_size(100, &LAYOUT);
    let bottom = auto_hide(rect(0, 2088, 3840, 2160));
    assert_eq!(usable_height(&bottom), 2088);
    // floor(2088 / 1.5) - 2 * 12.
    let capped = fit_height(tall, usable_height(&bottom), 1.5, &LAYOUT);
    assert_eq!(capped.height, 1392 - 24);
    let side = auto_hide(rect(0, 0, 72, 2160));
    assert_eq!(usable_height(&side), 2160);
    let capped = fit_height(tall, usable_height(&side), 1.5, &LAYOUT);
    assert_eq!(capped.height, 1440 - 24);
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
