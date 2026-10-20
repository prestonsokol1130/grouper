//! Showing and hiding the flyout window. Every failure is logged; none is
//! fatal, and the window is only ever hidden, never destroyed.

use std::sync::Arc;
use std::time::Instant;

use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, Runtime,
    WebviewWindow, WindowEvent,
};

use super::hide::hide_or_log;
use super::icon_size::flyout_icon_px;
use super::layout::{fit_rows, flyout_size, layout_for, to_physical};
use super::origin::origin;
use super::toggle::Decision;
use super::{FlyoutShow, FlyoutState, FLYOUT_LABEL, FLYOUT_SHOW_EVENT};
use crate::state::{Group, Store};
use crate::taskbar::placement::{place, taskbar_edge, usable_height};
use crate::win32::flyout_window::{apply_flyout_styles, bring_to_foreground, round_corners};
use crate::win32::monitor::current_monitor_info;
use crate::win32::taskbar_icons::taskbar_icons;

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
    let Some(state) = app.try_state::<FlyoutState>() else {
        log::error!("flyout requested before it was set up");
        return;
    };
    let Some(group) = find_group(app, group_id) else {
        log::warn!("flyout requested for unknown group {group_id:?}");
        return;
    };
    // The guard is dropped at the end of the statement, before any window call.
    let decision = state.toggle().request(group_id, started);
    match decision {
        Decision::Hide => {
            log::info!("flyout for {group_id:?} toggled off");
            hide_or_log(app);
        }
        Decision::StayHidden => {
            log::info!("flyout for {group_id:?} toggled off (already hidden by the click)");
        }
        Decision::Show => {
            // Marked shown first so a blur during the show still hides it.
            state.toggle().shown(group_id);
            match show(app, &state, &group) {
                Ok(()) => {
                    let us = started.elapsed().as_micros();
                    log::info!("flyout_show_us={us} group={group_id}");
                }
                Err(error) => {
                    // Never leave a visible topmost window behind a failure.
                    state.toggle().hidden();
                    hide_or_log(app);
                    log::warn!("could not show the flyout for {group_id:?}: {error}");
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

fn find_group<R: Runtime>(app: &AppHandle<R>, group_id: &str) -> Option<Group> {
    let store = app.try_state::<Arc<Store>>()?;
    store
        .snapshot()
        .groups
        .into_iter()
        .find(|group| group.id == group_id)
}

/// Sizes and places the flyout for `group` on the monitor under the
/// cursor, sends it the group's data, then shows and focuses it.
fn show<R: Runtime>(app: &AppHandle<R>, state: &FlyoutState, group: &Group) -> Result<(), String> {
    let window = flyout_window(app)?;
    let (cursor, monitor) = current_monitor_info()?;
    let settings = app
        .try_state::<Arc<Store>>()
        .map(|s| s.snapshot().settings)
        .unwrap_or_default();
    // Read on every show, so a taskbar or monitor change shows next time.
    let icon_px = flyout_icon_px(settings.flyout_icon_size, taskbar_icons, monitor.scale);
    let apps = group.app_ids.len();
    let layout = layout_for(settings.flyout_layout, icon_px);
    let layout = fit_rows(layout, apps, usable_height(&monitor), monitor.scale);
    let size = flyout_size(apps, &layout);
    let logical = (f64::from(size.width), f64::from(size.height));
    let (x, y) = place(cursor, logical, &monitor, f64::from(layout.margin));
    let edge = taskbar_edge(&monitor);
    let payload = FlyoutShow {
        group_id: group.id.clone(),
        apps: group.app_ids.clone(),
        size,
        origin: origin(edge, cursor, (x, y), size, monitor.scale),
        columns: layout.columns,
        rows: layout.rows,
        icon_px,
    };
    state.set_last(payload.clone());
    // Before showing, so the warm React tree swaps to this group's data.
    let target = EventTarget::webview_window(FLYOUT_LABEL);
    if let Err(error) = app.emit_to(target, FLYOUT_SHOW_EVENT, &payload) {
        log::warn!("could not emit {FLYOUT_SHOW_EVENT}: {error}");
    }
    let physical = PhysicalSize::new(
        to_physical(size.width, monitor.scale),
        to_physical(size.height, monitor.scale),
    );
    let position = PhysicalPosition::new(x, y);
    // Moving onto a monitor with another DPI rescales the window, so the
    // size is set after the move and the position once more after that.
    window
        .set_position(position)
        .map_err(|e| format!("set_position: {e}"))?;
    window
        .set_size(physical)
        .map_err(|e| format!("set_size: {e}"))?;
    window
        .set_position(position)
        .map_err(|e| format!("set_position: {e}"))?;
    window.show().map_err(|e| format!("show: {e}"))?;
    // A flyout that cannot take focus would never blur, so never hide.
    focus(&window)?;
    Ok(())
}

/// Moves keyboard focus to the flyout so a click elsewhere blurs it.
/// `Err` only when both `SetForegroundWindow` and `set_focus` failed.
fn focus<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
    // Allowed by the AllowSetForegroundWindow every Grouper launch makes.
    let foreground = raw_hwnd(window).and_then(bring_to_foreground);
    if let Err(error) = &foreground {
        log::warn!("flyout foreground: {error}");
    }
    let set_focus = window.set_focus().map_err(|e| e.to_string());
    if let Err(error) = &set_focus {
        log::warn!("flyout set_focus: {error}");
    }
    either_focused(foreground, set_focus)
}

/// Focus moved if either attempt succeeded; otherwise both errors.
fn either_focused(
    foreground: Result<(), String>,
    set_focus: Result<(), String>,
) -> Result<(), String> {
    match (foreground, set_focus) {
        (Err(fg), Err(sf)) => Err(format!("focus failed: foreground: {fg}; set_focus: {sf}")),
        _ => Ok(()),
    }
}

pub(super) fn flyout_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, String> {
    app.get_webview_window(FLYOUT_LABEL)
        .ok_or_else(|| format!("flyout window {FLYOUT_LABEL:?} not found"))
}

#[cfg(windows)]
fn raw_hwnd<R: Runtime>(window: &WebviewWindow<R>) -> Result<*mut std::ffi::c_void, String> {
    window
        .hwnd()
        .map(|hwnd| hwnd.0)
        .map_err(|e| format!("flyout hwnd: {e}"))
}

#[cfg(not(windows))]
fn raw_hwnd<R: Runtime>(_window: &WebviewWindow<R>) -> Result<*mut std::ffi::c_void, String> {
    Err("unsupported platform".to_owned())
}

#[cfg(test)]
mod tests {
    use super::either_focused;

    fn err() -> Result<(), String> {
        Err("no".to_owned())
    }

    #[test]
    fn focus_succeeds_if_either_attempt_does() {
        assert_eq!(either_focused(Ok(()), Ok(())), Ok(()));
        assert_eq!(either_focused(err(), Ok(())), Ok(()));
        assert_eq!(either_focused(Ok(()), err()), Ok(()));
    }

    #[test]
    fn focus_fails_only_if_both_do() {
        let error = either_focused(Err("fg".into()), Err("sf".into())).unwrap_err();
        assert!(error.contains("fg") && error.contains("sf"), "{error}");
    }
}
