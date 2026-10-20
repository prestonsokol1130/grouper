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
    /// The user made a Start Menu shortcut, so Grouper keeps it in sync.
    // Its shortcut and icon follow the group, and are recreated if missing.
    pub has_shortcut: bool,
    /// Where that shortcut was last written.
    pub shortcut_path: Option<String>,
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

// Which screen corners open the Launchpad (DESIGN.md section 12); both on
// by default. (Plain comments: doc comments would grow the bindings.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct HotCorners {
    pub top_left: bool,
    pub top_right: bool,
}

impl Default for HotCorners {
    fn default() -> Self {
        Self {
            top_left: true,
            top_right: true,
        }
    }
}

/// The Launchpad shortcut before DESIGN.md picked Alt + `; nobody could
/// choose it (there was no setting), so a stored one is upgraded.
pub const LEGACY_HOTKEY: &str = "Ctrl+Alt+Space";

/// The default Launchpad shortcut (DESIGN.md section 12).
pub const DEFAULT_HOTKEY: &str = "Alt+`";

/// User settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // Changed only through `set_hotkey`, which registers it first.
    pub hotkey: String,
    pub hot_corners: HotCorners,
    pub autostart: bool,
    /// `"system"`, `"light"` or `"dark"`.
    pub theme: String,
    pub first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.into(),
            hot_corners: HotCorners::default(),
            autostart: false,
            theme: "system".into(),
            first_run_done: false,
        }
    }
}

/// A partial settings update: only the fields that are set are changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub hot_corners: Option<HotCorners>,
    pub autostart: Option<bool>,
    pub theme: Option<String>,
    pub first_run_done: Option<bool>,
}

impl Settings {
    /// Replaces the legacy default shortcut with [`DEFAULT_HOTKEY`].
    /// Returns true when it did.
    pub fn upgrade_legacy_hotkey(&mut self) -> bool {
        let legacy = self.hotkey == LEGACY_HOTKEY;
        if legacy {
            self.hotkey = DEFAULT_HOTKEY.into();
        }
        legacy
    }

    /// Applies the fields set in `patch`, keeping every other setting.
    pub fn apply(&mut self, patch: SettingsPatch) {
        if let Some(hot_corners) = patch.hot_corners {
            self.hot_corners = hot_corners;
        }
        if let Some(autostart) = patch.autostart {
            self.autostart = autostart;
        }
        if let Some(theme) = patch.theme {
            self.theme = theme;
        }
        if let Some(first_run_done) = patch.first_run_done {
            self.first_run_done = first_run_done;
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
