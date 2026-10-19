//! The app list beyond scanning: launching apps and groups, adding apps by
//! hand, and removing or hiding them.

pub mod add;
pub mod launch;
pub mod remove;

#[cfg(test)]
mod add_tests;
#[cfg(test)]
mod launch_tests;
