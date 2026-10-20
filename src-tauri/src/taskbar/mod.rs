//! Taskbar group folders: the composed folder image ([`compose`], drawn in
//! the [`style`] with [`squircle`] masks), its multi-size `.ico`
//! ([`ico`]), and the cached `.ico` file for a group ([`group_icon`]).
//!
//! Pure Rust with no Win32, so all of it runs and tests on any OS.

pub mod compose;
pub mod group_icon;
pub mod ico;
pub mod squircle;
pub mod style;

#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod group_icon_tests;
