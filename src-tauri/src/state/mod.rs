//! Persistent app state: the model, pure helpers, and the on-disk store.

pub mod disk;
pub mod disk_write;
pub mod ids;
pub mod migrate;
pub mod model;
pub mod ops;
pub mod paths;
pub mod reorder;
pub mod store;

#[cfg(test)]
mod disk_tests;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod store_icon_tests;
#[cfg(test)]
mod store_tests;

pub use model::{
    AppEntry, AppSource, AppState, Group, HotCorners, LaunchpadItem, LaunchpadLayout, Settings,
    SettingsPatch, CURRENT_SCHEMA,
};
pub use store::Store;
