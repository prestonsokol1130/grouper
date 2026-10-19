//! The persisted app state types. Pure data: no IO here.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Schema version written by this build. Bump it and extend
/// [`crate::state::migrate`] when the shape of `app_state.json` changes.
pub const CURRENT_SCHEMA: u32 = 1;

/// Where an app entry came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppSource {
    /// Found by scanning the Start Menu.
    #[default]
    StartMenu,
    /// Added by the user by hand.
    Manual,
}

/// One launchable app.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AppEntry {
    /// Stable id from [`crate::state::ids::app_id`].
    pub id: String,
    pub name: String,
    pub target: String,
    pub args: String,
    pub working_dir: String,
    pub source: AppSource,
    /// The `.lnk` this entry came from, if any.
    pub lnk_path: Option<String>,
    /// The target no longer exists on disk.
    pub missing: bool,
    /// Hidden by the user (Start Menu apps cannot be deleted).
    pub hidden: bool,
}

/// A group of apps: both a taskbar folder and a Launchpad folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Group {
    /// Short random url-safe id (`[A-Za-z0-9_-]`).
    pub id: String,
    pub name: String,
    pub app_ids: Vec<String>,
    /// Bumped whenever the group's composite icon must be rebuilt.
    pub icon_version: u32,
}

/// One cell on a Launchpad page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchpadItem {
    App {
        #[serde(rename = "appId")]
        app_id: String,
    },
    Folder {
        #[serde(rename = "groupId")]
        group_id: String,
    },
}

/// The Launchpad grid: a list of pages, each a list of items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchpadLayout {
    pub pages: Vec<Vec<LaunchpadItem>>,
}

/// User settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub hotkey: String,
    pub autostart: bool,
    /// `"system"`, `"light"` or `"dark"`.
    pub theme: String,
    pub first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+Space".into(),
            autostart: false,
            theme: "system".into(),
            first_run_done: false,
        }
    }
}

/// Everything Grouper persists in `app_state.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AppState {
    pub schema_version: u32,
    pub apps: Vec<AppEntry>,
    pub groups: Vec<Group>,
    pub launchpad: LaunchpadLayout,
    pub settings: Settings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA,
            apps: Vec::new(),
            groups: Vec::new(),
            launchpad: LaunchpadLayout::default(),
            settings: Settings::default(),
        }
    }
}
