//! Windows Shell and COM helpers.
//!
//! Win32 calls in this module stay behind `#[cfg(windows)]`. On other
//! platforms the same API compiles, and fallible calls return
//! `Err("unsupported platform")`. The exception is [`gdi`], which wraps
//! Win32 handle types and so exists on Windows only.

pub mod com;
#[cfg(windows)]
pub mod gdi;
pub mod lnk_read;
pub mod wide;

#[cfg(all(test, windows))]
pub(crate) mod lnk_read_tests;
