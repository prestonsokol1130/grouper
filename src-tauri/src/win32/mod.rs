//! Windows Shell and COM helpers.
//!
//! Win32 calls in this module stay behind `#[cfg(windows)]`. On other
//! platforms the same API compiles, and fallible calls return
//! `Err("unsupported platform")`. The exception is [`gdi`], which wraps
//! Win32 handle types and so exists on Windows only.

pub mod accent;
pub mod apps_folder;
pub mod balloon;
pub mod com;
pub mod cursor;
pub mod env;
pub mod file_dialog;
pub mod flyout_window;
pub mod foreground;
#[cfg(windows)]
pub mod gdi;
pub mod known_folder;
pub mod launch;
pub mod lnk_aumid;
pub mod lnk_read;
pub mod lnk_write;
pub mod material;
pub mod monitor;
pub mod reveal;
pub mod shell_notify;
pub mod unpin;
pub mod version_info;
pub mod wide;

#[cfg(all(test, windows))]
pub(crate) mod lnk_read_tests;
#[cfg(all(test, windows))]
mod lnk_write_tests;
