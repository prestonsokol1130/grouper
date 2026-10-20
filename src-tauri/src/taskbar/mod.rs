//! Taskbar group folders: the composed folder image ([`compose`], drawn in
//! the [`style`] with [`squircle`] masks), its multi-size `.ico`
//! ([`ico`]), the cached `.ico` file for a group ([`group_icon`]), and the
//! group's Start Menu shortcut that carries it ([`shortcut`],
//! [`shortcut_write`]).
//!
//! All but [`shortcut_write`] is pure Rust with no Win32, so it runs and
//! tests on any OS. [`shortcut_write`] reads and writes shortcuts through
//! the Win32 wrappers, which return `Err("unsupported platform")`
//! elsewhere.

pub mod compose;
pub mod group_icon;
pub mod ico;
pub mod shortcut;
pub mod shortcut_write;
pub mod squircle;
pub mod style;

#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod group_icon_tests;
#[cfg(test)]
mod shortcut_tests;
