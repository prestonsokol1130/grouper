//! Showing and hiding the Launchpad window. Every failure is logged; none
//! is fatal, and the window is only ever hidden, never destroyed.

use std::sync::Arc;
use std::time::Instant;

use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, Runtime,
    WebviewWindow, WindowEvent,
};

use super::layout::ensure_default;
use super::origin::launchpad_origin;
use super::{
    LaunchpadShow, LaunchpadState, Opener, LAUNCHPAD_HIDE_EVENT, LAUNCHPAD_LABEL,
    LAUNCHPAD_SHOW_EVENT,
};
use crate::hotkey;
use crate::state::Store;
use crate::taskbar::placement::MonitorInfo;
use crate::win32::flyout_window::{apply_flyout_styles, bring_to_foreground};
use crate::win32::monitor::current_monitor_info;
use crate::win32::window_rect::set_window_rect;

/// Manages the Launchpad's state. Call at startup before any window
/// exists.
pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(LaunchpadState::default());
}

/// Sets up the Launchpad window at startup, after [`manage`]: the
/// flyout's tool-window styles (no taskbar button, topmost) and
/// hide-on-blur.
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(LAUNCHPAD_LABEL) else {
        log::error!("window {LAUNCHPAD_LABEL:?} not found; the Launchpad will not open");
        return;
    };
    if let Err(error) = raw_hwnd(&window).and_then(apply_flyout_styles) {
        log::error!("could not style the Launchpad window: {error}");
    }
    let handle = app.clone();
    // A plain integer, so the blur watch can take it to its thread.
    let hwnd = raw_hwnd(&window).map(|hwnd| hwnd as isize);
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            if launchpad_visible(&handle) {
                match hwnd {
                    Ok(hwnd) => super::blur::on_blur(&handle, hwnd),
                    Err(_) => hide_or_log(&handle),
                }
            }
        }
    });
}

/// Shows the Launchpad on the monitor under the cursor, or hides it when
/// it is already showing.
pub fn toggle<R: Runtime>(app: &AppHandle<R>, opener: Opener) {
    if launchpad_visible(app) {
        log::info!("Launchpad toggled off ({opener:?})");
        hide_or_log(app);
        return;
    }
    let started = Instant::now();
    match show(app, opener, started) {
        Ok(steps) => {
            let us = started.elapsed().as_micros();
            log::info!("launchpad_show_us={us} opener={opener:?} {steps}");
        }
        Err(error) => {
            // Never leave a fullscreen topmost window behind a failure.
            log::warn!("could not show the Launchpad: {error}");
            hide_or_log(app);
        }
    }
}

pub(super) fn launchpad_visible<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<LaunchpadState>()
        .is_some_and(|state| state.visible())
}

/// Covers the monitor under the cursor, tells the page where to animate
/// from, then shows and focuses the window. Returns how long after
/// `started` the window was placed and visible, for the log.
fn show<R: Runtime>(
    app: &AppHandle<R>,
    opener: Opener,
    started: Instant,
) -> Result<String, String> {
    let window = launchpad_window(app)?;
    if let Some(store) = app.try_state::<Arc<Store>>() {
        ensure_default(app, &store);
    }
    let (_, monitor) = current_monitor_info()?;
    let rect = monitor.monitor;
    let payload = LaunchpadShow {
        origin: launchpad_origin(opener, rect, monitor.scale),
    };
    let target = EventTarget::webview_window(LAUNCHPAD_LABEL);
    if let Err(error) = app.emit_to(target, LAUNCHPAD_SHOW_EVENT, payload) {
        log::warn!("could not emit {LAUNCHPAD_SHOW_EVENT}: {error}");
    }
    let position = PhysicalPosition::new(rect.left, rect.top);
    let width = u32::try_from(rect.right.saturating_sub(rect.left)).unwrap_or(1);
    let height = u32::try_from(rect.bottom.saturating_sub(rect.top)).unwrap_or(1);
    cover(&window, position, PhysicalSize::new(width, height), monitor)?;
    let placed = started.elapsed().as_micros();
    // Marked visible first so a blur during the show still hides it.
    if let Some(state) = app.try_state::<LaunchpadState>() {
        state.set_visible(true);
    }
    window.show().map_err(|e| format!("show: {e}"))?;
    let visible = started.elapsed().as_micros();
    focus(&window);
    hotkey::notify_pending(app, LAUNCHPAD_LABEL);
    Ok(format!("placed_us={placed} visible_us={visible}"))
}

/// Moves and sizes the window to exactly `position` and `size`. When it
/// already has the monitor's DPI, one `SetWindowPos` does it (none when it
/// is already there). Crossing onto a monitor with another DPI makes
/// Windows rescale the window (sometimes only once the resize lands it
/// there), so then it is placed again until it sticks, at most three times.
fn cover<R: Runtime>(
    window: &WebviewWindow<R>,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    monitor: MonitorInfo,
) -> Result<(), String> {
    if window.scale_factor().ok() == Some(monitor.scale) {
        let placed = raw_hwnd(window).and_then(|hwnd| set_window_rect(hwnd, monitor.monitor));
        match placed {
            Ok(_) => return Ok(()),
            Err(error) => log::debug!("Launchpad one-step placement: {error}"),
        }
    }
    for _ in 0..3 {
        window
            .set_position(position)
            .map_err(|e| format!("set_position: {e}"))?;
        window
            .set_size(size)
            .map_err(|e| format!("set_size: {e}"))?;
        window
            .set_position(position)
            .map_err(|e| format!("set_position: {e}"))?;
        let placed = window.outer_position().ok() == Some(position);
        if placed && window.outer_size().ok() == Some(size) {
            return Ok(());
        }
    }
    log::warn!("Launchpad did not settle at {position:?} {size:?}");
    Ok(())
}

/// Moves keyboard focus to the Launchpad, so Escape works and a click
/// elsewhere hides it. Failures are logged: a click on the fullscreen
/// window still focuses it.
fn focus<R: Runtime>(window: &WebviewWindow<R>) {
    if let Err(error) = raw_hwnd(window).and_then(bring_to_foreground) {
        log::info!("Launchpad foreground: {error}");
    }
    if let Err(error) = window.set_focus() {
        log::warn!("Launchpad set_focus: {error}");
    }
}

/// Hides the Launchpad (Escape, a click on the background, a launch, a
/// toggle or a blur) and tells the page. `Err` when the native hide
/// failed; it then stays marked as showing.
pub fn hide<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.try_state::<LaunchpadState>();
    // Marked hidden first, so the blur the hide causes changes nothing.
    let before = state.as_ref().map(|state| state.set_visible(false));
    let hidden = launchpad_window(app).and_then(|window| {
        window
            .hide()
            .map_err(|e| format!("could not hide the Launchpad: {e}"))
    });
    if let Err(error) = hidden {
        if let (Some(state), Some(before)) = (&state, before) {
            state.set_visible(before);
        }
        return Err(error);
    }
    let target = EventTarget::webview_window(LAUNCHPAD_LABEL);
    if let Err(error) = app.emit_to(target, LAUNCHPAD_HIDE_EVENT, ()) {
        log::warn!("could not emit {LAUNCHPAD_HIDE_EVENT}: {error}");
    }
    Ok(())
}

pub(super) fn hide_or_log<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = hide(app) {
        log::warn!("{error}");
    }
}

fn launchpad_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, String> {
    app.get_webview_window(LAUNCHPAD_LABEL)
        .ok_or_else(|| format!("window {LAUNCHPAD_LABEL:?} not found"))
}

#[cfg(windows)]
fn raw_hwnd<R: Runtime>(window: &WebviewWindow<R>) -> Result<*mut std::ffi::c_void, String> {
    window
        .hwnd()
        .map(|hwnd| hwnd.0)
        .map_err(|e| format!("Launchpad hwnd: {e}"))
}

#[cfg(not(windows))]
fn raw_hwnd<R: Runtime>(_window: &WebviewWindow<R>) -> Result<*mut std::ffi::c_void, String> {
    Err("unsupported platform".to_owned())
}
