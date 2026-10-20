//! The fullscreen Launchpad: the warm, never-destroyed `launchpad` window
//! that the shortcut, a hot corner, the tray and `--launchpad` toggle.
//!
//! [`layout`] builds the first layout and [`origin`] finds where the open
//! animation starts; both are pure. [`window`] does the Tauri and Win32
//! work.

pub mod blur;
pub mod layout;
pub mod origin;
pub mod window;

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::flyout::origin::FlyoutOrigin;
use crate::hot_corners::dwell::Corner;

/// The Launchpad window's label in `tauri.conf.json`.
pub const LAUNCHPAD_LABEL: &str = "launchpad";

/// Sent to the Launchpad window, just before it is shown.
pub const LAUNCHPAD_SHOW_EVENT: &str = "launchpad:show";

/// Sent to the Launchpad window, with no payload, whenever it is hidden.
pub const LAUNCHPAD_HIDE_EVENT: &str = "launchpad:hide";

/// What opened the Launchpad, which decides where it animates from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opener {
    /// The shortcut, the tray menu or `--launchpad`: the screen's centre
    /// bottom.
    Shortcut,
    /// A hot corner: that corner.
    Corner(Corner),
}

/// The [`LAUNCHPAD_SHOW_EVENT`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchpadShow {
    /// Where the open animation starts, in logical pixels from the
    /// window's top-left (the same shape as the flyout's).
    pub origin: FlyoutOrigin,
}

/// Whether the Launchpad is showing, held in Tauri's managed state.
#[derive(Debug, Default)]
pub struct LaunchpadState {
    visible: AtomicBool,
}

impl LaunchpadState {
    /// True while the Launchpad is showing.
    pub fn visible(&self) -> bool {
        self.visible.load(Ordering::SeqCst)
    }

    /// Records whether it is showing; returns what it was before.
    fn set_visible(&self, visible: bool) -> bool {
        self.visible.swap(visible, Ordering::SeqCst)
    }
}
