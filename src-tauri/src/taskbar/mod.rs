//! Taskbar group folders: the composed folder image ([`compose`], drawn in
//! the [`style`] with [`squircle`] masks), its multi-size `.ico`
//! ([`ico`]), the cached `.ico` file for a group ([`group_icon`]), and the
//! group's Start Menu shortcut that carries it ([`shortcut`],
//! [`shortcut_write`]), kept in step with the group by [`shortcut_sync`].
//!
//! All but [`shortcut_write`] and [`shortcut_sync`] is pure Rust with no
//! Win32, so it runs and tests on any OS. Those two read and write
//! shortcuts through the Win32 wrappers, which return
//! `Err("unsupported platform")` elsewhere.

pub mod compose;
pub mod group_icon;
pub mod ico;
pub mod icon_inputs;
pub mod shortcut;
pub mod shortcut_sync;
pub mod shortcut_write;
pub mod squircle;
pub mod style;

#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod group_icon_tests;
#[cfg(test)]
mod icon_inputs_tests;
#[cfg(all(test, windows))]
mod shortcut_sync_pin_tests;
#[cfg(all(test, windows))]
mod shortcut_sync_tests;
#[cfg(test)]
mod shortcut_tests;
