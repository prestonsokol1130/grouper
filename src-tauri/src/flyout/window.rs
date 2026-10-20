//! Showing and hiding the flyout window. Every failure is logged; none is
//! fatal, and the window is only ever hidden, never destroyed.

use std::sync::Arc;
use std::time::Instant;

use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, Runtime,
    WebviewWindow, WindowEvent,
};

use super::hide::hide_or_log;
use super::layout::{fit_height, flyout_size, layout_for, to_physical};
use super::origin::origin;
use super::present::{focus, place};
use super::toggle::Decision;
use super::{FlyoutShow, FlyoutState, FLYOUT_LABEL, FLYOUT_SHOW_EVENT};
use crate::state::{Group, Store};
use crate::taskbar::placement::{place as place_on, taskbar_edge, usable_height};
use crate::win32::flyout_window::{apply_flyout_styles, round_corners};
use crate::win32::monitor::current_monitor_info;

/// Manages the flyout's state. Call at startup before any window exists,
/// so the page's first `get_flyout` always finds it.
pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(FlyoutState::default());
}

/// Sets up the flyout window at startup, after [`manage`]: its
/// tool-window styles and hide-on-blur. Call before the first [`request`].
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(FLYOUT_LABEL) else {
        log::error!("flyout window {FLYOUT_LABEL:?} not found; flyouts will not open");
        return;
    };
    match raw_hwnd(&window).and_then(apply_flyout_styles) {
        Ok(()) => {}
        Err(error) => log::error!("could not style the flyout window: {error}"),
    }
    // Windows 10 has no rounded corners; the page's own border still shows.
    match raw_hwnd(&window).and_then(round_corners) {
        Ok(()) => log::info!("flyout corners rounded"),
        Err(error) => log::info!("flyout corners not rounded: {error}"),
    }
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            on_blur(&handle);
        }
    });
}

/// Handles `--flyout <group_id>`: shows the group's flyout over the
/// taskbar, or hides it when this is the second click of a toggle.
pub fn request<R: Runtime>(app: &AppHandle<R>, group_id: &str) {
    let started = Instant::now();
    let Some(group) = find_group(app, group_id) else {
        log::warn!("flyout requested for unknown group {group_id:?}");
        return;
    };
    request_group(app, &group, started);
}

/// [`request`] for `group`, timed from `started`. Returns the microseconds
/// it took to show the flyout, or `None` when it did not show it.
pub(super) fn request_group<R: Runtime>(
    app: &AppHandle<R>,
    group: &Group,
    started: Instant,
) -> Option<u128> {
    let Some(state) = app.try_state::<FlyoutState>() else {
        log::error!("flyout requested before it was set up");
        return None;
    };
    let group_id = group.id.as_str();
    // The guard is dropped at the end of the statement, before any window call.
    let decision = state.toggle().request(group_id, started);
    match decision {
        Decision::Hide => {
            log::info!("flyout for {group_id:?} toggled off");
            hide_or_log(app);
            None
        }
        Decision::StayHidden => {
            log::info!("flyout for {group_id:?} toggled off (already hidden by the click)");
            None
        }
        Decision::Show => {
            // Marked shown first so a blur during the show still hides it.
            state.toggle().shown(group_id);
            match show(app, &state, group, started) {
                Ok(steps) => {
                    let us = started.elapsed().as_micros();
                    log::info!("flyout_show_us={us} group={group_id} {steps}");
                    Some(us)
                }
                Err(error) => {
                    // Never leave a visible topmost window behind a failure.
                    state.toggle().hidden();
                    hide_or_log(app);
                    log::warn!("could not show the flyout for {group_id:?}: {error}");
                    None
                }
            }
        }
    }
}

/// The flyout lost focus: hide it, remembering the group for the toggle.
fn on_blur<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<FlyoutState>() else {
        return;
    };
    let must_hide = state.toggle().blurred(Instant::now());
    if must_hide {
        log::debug!("flyout lost focus; hiding");
        hide_or_log(app);
    }
}

/// Group `group_id`, read without copying the rest of the state.
pub(super) fn find_group<R: Runtime>(app: &AppHandle<R>, group_id: &str) -> Option<Group> {
    let store = app.try_state::<Arc<Store>>()?;
    store.read(|state| state.group_by_id(group_id).cloned())
}

/// Sizes and places the flyout for `group` on the monitor under the
/// cursor, sends it the group's data, then shows and focuses it. Returns
/// how long after `started` each step ended, for the log.
fn show<R: Runtime>(
    app: &AppHandle<R>,
    state: &FlyoutState,
    group: &Group,
    started: Instant,
) -> Result<String, String> {
    let window = flyout_window(app)?;
    let (cursor, monitor) = current_monitor_info()?;
    let icon = app
        .try_state::<Arc<Store>>()
        .map(|s| s.read(|state| state.settings.icon_size));
    let layout = layout_for(icon.unwrap_or_default());
    let size = flyout_size(group.app_ids.len(), &layout);
    let size = fit_height(size, usable_height(&monitor), monitor.scale, &layout);
    let logical = (f64::from(size.width), f64::from(size.height));
    let (x, y) = place_on(cursor, logical, &monitor, f64::from(layout.margin));
    let edge = taskbar_edge(&monitor);
    let payload = FlyoutShow {
        group_id: group.id.clone(),
        apps: group.app_ids.clone(),
        size,
        origin: origin(edge, cursor, (x, y), size, monitor.scale),
    };
    state.set_last(payload.clone());
    // Before showing, so the warm React tree swaps to this group's data.
    let target = EventTarget::webview_window(FLYOUT_LABEL);
    if let Err(error) = app.emit_to(target, FLYOUT_SHOW_EVENT, &payload) {
        log::warn!("could not emit {FLYOUT_SHOW_EVENT}: {error}");
    }
    let emitted = started.elapsed().as_micros();
    let physical = PhysicalSize::new(
        to_physical(size.width, monitor.scale),
        to_physical(size.height, monitor.scale),
    );
    place(
        &window,
        PhysicalPosition::new(x, y),
        physical,
        monitor.scale,
    )?;
    let placed = started.elapsed().as_micros();
    window.show().map_err(|e| format!("show: {e}"))?;
    let visible = started.elapsed().as_micros();
    // A flyout that cannot take focus would never blur, so never hide.
    focus(&window)?;
    Ok(format!(
        "emitted_us={emitted} placed_us={placed} visible_us={visible}"
    ))
}

pub(super) fn flyout_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, String> {
    app.get_webview_window(FLYOUT_LABEL)
        .ok_or_else(|| format!("flyout window {FLYOUT_LABEL:?} not found"))
}

#[cfg(windows)]
pub(super) fn raw_hwnd<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<*mut std::ffi::c_void, String> {
    window
        .hwnd()
        .map(|hwnd| hwnd.0)
        .map_err(|e| format!("flyout hwnd: {e}"))
}

#[cfg(not(windows))]
pub(super) fn raw_hwnd<R: Runtime>(
    _window: &WebviewWindow<R>,
) -> Result<*mut std::ffi::c_void, String> {
    Err("unsupported platform".to_owned())
}
