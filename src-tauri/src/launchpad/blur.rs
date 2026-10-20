//! Hiding the Launchpad on blur, but only on a real one.
//!
//! WebView2 can report the window as unfocused while it is still in front:
//! the first click into a freshly shown page moves keyboard focus from the
//! window into the WebView2 child, and that shows up as `Focused(false)`.
//! Hiding on that event made the first tile click (or the first swipe)
//! close the Launchpad without doing anything. So a blur only starts a
//! short watch, and the Launchpad hides once another window is really in
//! front (or right away if it already is when the watch checks).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Runtime};

use crate::util::spawn::spawn_logged;
use crate::win32::foreground::is_foreground;

/// How often the watch checks which window is in front.
const POLL: Duration = Duration::from_millis(75);

/// Set while a watch thread runs, so a burst of blurs starts only one.
static WATCHING: AtomicBool = AtomicBool::new(false);

/// What the watch does on one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// Already hidden (a launch, Escape, a toggle): stop watching.
    Stop,
    /// Still showing and still in front: the blur was not real yet.
    Wait,
    /// Showing, and another window is in front: hide.
    Hide,
}

/// Pure decision for one check.
pub fn check(visible: bool, in_front: bool) -> Check {
    match (visible, in_front) {
        (false, _) => Check::Stop,
        (true, true) => Check::Wait,
        (true, false) => Check::Hide,
    }
}

/// Handles `Focused(false)` on the Launchpad window `hwnd` (an integer, so
/// it can cross threads): watches until it is hidden or another window is
/// in front, then hides it.
pub fn on_blur<R: Runtime>(app: &AppHandle<R>, hwnd: isize) {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let watcher = app.clone();
    let started = spawn_logged("launchpad-blur", move || {
        loop {
            std::thread::sleep(POLL);
            let visible = super::window::launchpad_visible(&watcher);
            // Pointer-sized integer to pointer: the API's own handle type.
            let in_front = is_foreground(hwnd as *mut std::ffi::c_void);
            match check(visible, in_front) {
                Check::Stop => break,
                Check::Wait => {}
                Check::Hide => {
                    log::debug!("Launchpad lost the foreground; hiding");
                    super::window::hide_or_log(&watcher);
                    break;
                }
            }
        }
        WATCHING.store(false, Ordering::SeqCst);
        Ok(())
    });
    if let Err(error) = started {
        WATCHING.store(false, Ordering::SeqCst);
        log::warn!("Launchpad blur watch: {error}; hiding now");
        super::window::hide_or_log(app);
    }
}

#[cfg(test)]
mod tests {
    use super::{check, Check};

    #[test]
    fn hides_only_when_showing_and_not_in_front() {
        assert_eq!(check(false, false), Check::Stop);
        assert_eq!(check(false, true), Check::Stop);
        assert_eq!(check(true, true), Check::Wait);
        assert_eq!(check(true, false), Check::Hide);
    }
}
