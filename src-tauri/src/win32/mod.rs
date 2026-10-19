//! Windows Shell and COM helpers.
//!
//! Win32 calls in this module stay behind `#[cfg(windows)]`. On other
//! platforms the same API compiles, and fallible calls return
//! `Err("unsupported platform")`.

pub mod com;
pub mod lnk_read;
pub mod wide;

#[cfg(all(test, windows))]
mod lnk_read_tests;
