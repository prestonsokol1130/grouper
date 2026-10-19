//! Persistent app state: the model, pure helpers, and the on-disk store.

pub mod disk;
pub mod ids;
pub mod migrate;
pub mod model;
pub mod ops;
pub mod paths;
pub mod store;

#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod store_tests;

pub use model::{
    AppEntry, AppSource, AppState, Group, LaunchpadItem, LaunchpadLayout, Settings, CURRENT_SCHEMA,
};
pub use store::Store;
