//! Hot corners (DESIGN.md section 12): resting the cursor in the top-left
//! or top-right corner of any monitor opens the Launchpad.
//!
//! One background thread polls the cursor every [`POLL`] (a few cheap
//! Win32 reads, no hooks) and feeds [`dwell::Dwell`]. Which corners are on
//! is kept in [`CornerFlags`], updated whenever the setting changes.

pub mod dwell;

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Runtime};

use crate::launchpad::{window, Opener};
use crate::state::HotCorners;
use crate::util::spawn::spawn_logged;
use crate::win32::cursor::cursor_monitor;
use dwell::{corner_at, Dwell};

/// How often the cursor is sampled.
pub const POLL: Duration = Duration::from_millis(50);

const TOP_LEFT: u8 = 1;
const TOP_RIGHT: u8 = 2;

/// The enabled corners, shared with the watcher thread, and its stop flag.
#[derive(Debug, Default)]
pub struct CornerFlags {
    bits: AtomicU8,
    stop: AtomicBool,
}

impl CornerFlags {
    /// Turns corners on or off to match `corners`.
    pub fn set(&self, corners: HotCorners) {
        let bits = u8::from(corners.top_left) * TOP_LEFT + u8::from(corners.top_right) * TOP_RIGHT;
        self.bits.store(bits, Ordering::Relaxed);
    }

    /// The corners that are on.
    pub fn get(&self) -> HotCorners {
        let bits = self.bits.load(Ordering::Relaxed);
        HotCorners {
            top_left: bits & TOP_LEFT != 0,
            top_right: bits & TOP_RIGHT != 0,
        }
    }

    /// Ends the watcher thread at its next sample.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Starts the watcher with `corners` enabled. A failure to start is
/// logged; Grouper runs on without hot corners.
pub fn start<R: Runtime>(app: &AppHandle<R>, corners: HotCorners) {
    let flags = Arc::new(CornerFlags::default());
    flags.set(corners);
    app.manage(Arc::clone(&flags));
    let handle = app.clone();
    let started = spawn_logged("hot-corners", move || {
        watch(&handle, &flags);
        Ok(())
    });
    match started {
        Ok(_) => log::info!("hot corners on: {corners:?}"),
        Err(error) => log::error!("hot corners are off: {error}"),
    }
}

/// Follows a changed "Hot corners" setting.
pub fn update<R: Runtime>(app: &AppHandle<R>, corners: HotCorners) {
    if let Some(flags) = app.try_state::<Arc<CornerFlags>>() {
        flags.set(corners);
    }
}

/// Stops the watcher (on exit).
pub fn stop<R: Runtime>(app: &AppHandle<R>) {
    if let Some(flags) = app.try_state::<Arc<CornerFlags>>() {
        flags.stop();
    }
}

/// The polling loop. Read errors are logged once until a read works again.
fn watch<R: Runtime>(app: &AppHandle<R>, flags: &CornerFlags) {
    let mut dwell = Dwell::default();
    let mut failing = false;
    while !flags.stop.load(Ordering::Relaxed) {
        std::thread::sleep(POLL);
        let enabled = flags.get();
        if !enabled.top_left && !enabled.top_right {
            dwell.reset();
            continue;
        }
        let hit = match cursor_monitor() {
            Ok((cursor, monitor)) => {
                failing = false;
                corner_at(cursor, monitor, enabled).map(|corner| (corner, monitor))
            }
            Err(error) => {
                if !failing {
                    log::debug!("hot corners: cursor unreadable: {error}");
                }
                failing = true;
                None
            }
        };
        if let Some(corner) = dwell.step(Instant::now(), hit) {
            log::info!("hot corner {corner:?} triggered");
            let handle = app.clone();
            let run = app.run_on_main_thread(move || {
                window::toggle(&handle, Opener::Corner(corner));
            });
            if let Err(error) = run {
                log::warn!("hot corner: could not reach the main thread: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_round_trip_every_combination() {
        let flags = CornerFlags::default();
        for top_left in [false, true] {
            for top_right in [false, true] {
                let corners = HotCorners {
                    top_left,
                    top_right,
                };
                flags.set(corners);
                assert_eq!(flags.get(), corners);
            }
        }
    }
}
