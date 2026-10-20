//! The group flyout: the warm, never-destroyed `flyout` window that
//! `Grouper.exe --flyout <group_id>` shows above a pinned group's taskbar
//! button.
//!
//! [`layout`] sizes it, [`icon_size`] picks its icon size, [`origin`] finds
//! where it animates from and [`toggle`] decides whether a request shows or
//! hides it; all are pure. [`window`] does the Tauri and Win32 work and [`hide`]
//! hides it.

pub mod hide;
pub mod icon_size;
pub mod layout;
pub mod origin;
pub mod toggle;
pub mod window;

use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use specta::Type;

use layout::FlyoutSize;
use origin::FlyoutOrigin;
use toggle::Toggle;

/// The flyout window's label in `tauri.conf.json`.
pub const FLYOUT_LABEL: &str = "flyout";

/// Sent to the flyout window, before it is shown, with what to display.
pub const FLYOUT_SHOW_EVENT: &str = "flyout:show";

/// Sent to the flyout window, with no payload, whenever it is hidden.
pub const FLYOUT_HIDE_EVENT: &str = "flyout:hide";

/// What the flyout shows: the [`FLYOUT_SHOW_EVENT`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FlyoutShow {
    pub group_id: String,
    // The group's app ids, in order.
    pub apps: Vec<String>,
    // The window size in logical pixels, and where the open animation
    // starts. (Plain comments: the bindings repeat this struct inline.)
    pub size: FlyoutSize,
    pub origin: FlyoutOrigin,
    // One page's grid and the icon side, all in logical pixels.
    pub columns: u32,
    pub rows: u32,
    pub icon_px: u32,
}

/// Flyout bookkeeping, held in Tauri's managed state.
#[derive(Debug, Default)]
pub struct FlyoutState {
    toggle: Mutex<Toggle>,
    /// The last payload sent, for a flyout page that loads after the event.
    last: Mutex<Option<FlyoutShow>>,
}

impl FlyoutState {
    /// The toggle state machine. Never hold the guard across a window call:
    /// window calls can deliver focus events, whose handler locks it too.
    fn toggle(&self) -> MutexGuard<'_, Toggle> {
        // A panic while locked cannot leave Toggle half-updated in a way
        // that matters, so a poisoned lock is simply reused.
        self.toggle.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn set_last(&self, payload: FlyoutShow) {
        *self.last.lock().unwrap_or_else(|p| p.into_inner()) = Some(payload);
    }

    /// The token of the visible show ([`Toggle::token`]), if any.
    pub fn visible_token(&self) -> Option<u64> {
        self.toggle().token()
    }

    /// The payload of the most recent show, if any.
    pub fn last(&self) -> Option<FlyoutShow> {
        self.last.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}
