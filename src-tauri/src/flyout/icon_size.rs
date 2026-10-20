//! The flyout's icon size: fixed (Small, Medium, Large) or matched to the
//! taskbar's own button icons. Pure math; the Windows reads are in
//! [`crate::win32::taskbar_icons`].
//!
//! "Match taskbar" mapping (DESIGN.md section 7): the Windows 11 taskbar
//! draws its button icons 24 logical px square by default, 16 with small
//! taskbar buttons and 32 with the (Windows 10 era) large taskbar. A grid
//! of 24px icons reads as tiny, so the flyout draws them at
//! [`TASKBAR_SCALE`] times that, kept between the Small and Large flyout
//! sizes: small 16 -> 40, default 24 -> 48, large 32 -> 64. The result is
//! snapped so it is a whole number of physical pixels on the monitor under
//! the cursor, the way Explorer picks its own icon bitmaps.

use crate::state::settings::FlyoutIconSize;

/// The taskbar's button icon size setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskbarIcons {
    /// `TaskbarSi` 0, or Windows 10's `TaskbarSmallIcons` 1.
    Small,
    /// No setting, or `TaskbarSi` 1.
    #[default]
    Default,
    /// `TaskbarSi` 2.
    Large,
}

impl TaskbarIcons {
    /// From the `Explorer\Advanced` registry values (either may be
    /// missing). `TaskbarSi` (Windows 11) wins over `TaskbarSmallIcons`.
    pub fn from_registry(taskbar_si: Option<u32>, small_icons: Option<u32>) -> Self {
        match (taskbar_si, small_icons) {
            (Some(0), _) => Self::Small,
            (Some(2), _) => Self::Large,
            (Some(_), _) => Self::Default,
            (None, Some(1)) => Self::Small,
            (None, _) => Self::Default,
        }
    }

    /// The taskbar's icon side in logical px.
    pub fn logical_px(self) -> u32 {
        match self {
            Self::Small => 16,
            Self::Default => 24,
            Self::Large => 32,
        }
    }
}

/// Flyout icons are this many times the taskbar's.
pub const TASKBAR_SCALE: f64 = 2.0;

/// The smallest and largest matched size: the Small and Large flyout icons.
pub const MATCHED_RANGE: (u32, u32) = (40, 64);

/// The flyout icon for a taskbar with `icons` on a monitor at `scale`, in
/// logical px (see the file comment).
pub fn matched_px(icons: TaskbarIcons, scale: f64) -> u32 {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let wanted = f64::from(icons.logical_px()) * TASKBAR_SCALE;
    let (low, high) = MATCHED_RANGE;
    let clamped = wanted.clamp(f64::from(low), f64::from(high));
    // Whole physical pixels, then back to logical, rounded.
    let physical = (clamped * scale).round();
    let logical = (physical / scale).round();
    // `as` saturates; the clamp keeps it in range anyway.
    (logical as u32).clamp(low, high)
}

/// The flyout icon side in px for the setting: a fixed size, or (Match
/// taskbar) `taskbar` read now, so a change shows on the next open.
pub fn flyout_icon_px(
    setting: FlyoutIconSize,
    taskbar: impl FnOnce() -> TaskbarIcons,
    scale: f64,
) -> u32 {
    setting
        .fixed_px()
        .unwrap_or_else(|| matched_px(taskbar(), scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_values_pick_the_taskbar_size() {
        let read = TaskbarIcons::from_registry;
        assert_eq!(read(None, None), TaskbarIcons::Default);
        assert_eq!(read(Some(0), None), TaskbarIcons::Small);
        assert_eq!(read(Some(1), Some(1)), TaskbarIcons::Default);
        assert_eq!(read(Some(2), None), TaskbarIcons::Large);
        assert_eq!(read(Some(7), None), TaskbarIcons::Default);
        assert_eq!(read(None, Some(1)), TaskbarIcons::Small);
        assert_eq!(read(None, Some(0)), TaskbarIcons::Default);
    }

    #[test]
    fn matched_sizes_are_twice_the_taskbar_within_small_and_large() {
        let px = |icons| matched_px(icons, 1.0);
        assert_eq!(px(TaskbarIcons::Small), 40);
        assert_eq!(px(TaskbarIcons::Default), 48);
        assert_eq!(px(TaskbarIcons::Large), 64);
    }

    #[test]
    fn matched_sizes_hold_at_every_scale() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 3.0, 3.5] {
            assert_eq!(matched_px(TaskbarIcons::Default, scale), 48, "{scale}");
        }
        assert_eq!(matched_px(TaskbarIcons::Default, f64::NAN), 48);
        assert_eq!(matched_px(TaskbarIcons::Large, -1.0), 64);
    }

    #[test]
    fn fixed_settings_never_read_the_taskbar() {
        let never = || -> TaskbarIcons { panic!("read the taskbar") };
        assert_eq!(flyout_icon_px(FlyoutIconSize::Small, never, 3.0), 40);
        assert_eq!(flyout_icon_px(FlyoutIconSize::Medium, never, 3.0), 48);
        assert_eq!(flyout_icon_px(FlyoutIconSize::Large, never, 3.0), 64);
        let small = || TaskbarIcons::Small;
        assert_eq!(flyout_icon_px(FlyoutIconSize::MatchTaskbar, small, 3.0), 40);
    }
}
