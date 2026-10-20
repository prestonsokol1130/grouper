//! User settings (DESIGN.md section 9). Pure data: no IO here.
//!
//! Every field has a serde default, so an `app_state.json` written by an
//! older build loads with the new settings at their defaults. Field and
//! type comments are plain `//` comments on purpose: doc comments would
//! grow the generated bindings, which have a line budget.

use serde::{Deserialize, Serialize};
use specta::Type;

// Which screen corners open the Launchpad (DESIGN.md section 12); both on
// by default.
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

// "Window background" (DESIGN.md section 4). Mixed is the per-window
// default; the others override every window at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundSetting {
    #[default]
    Mixed,
    Mica,
    Acrylic,
    Solid,
}

// "Folder shape" (DESIGN.md section 5), for folder tiles and the taskbar
// `.ico`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FolderShape {
    #[default]
    Square,
    Squircle,
}

// "Icon size" (DESIGN.md section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl IconSize {
    /// The Launchpad icon side in px.
    pub fn launchpad_px(self) -> u32 {
        match self {
            Self::Small => 56,
            Self::Medium => 72,
            Self::Large => 96,
        }
    }

    /// The flyout icon side in px.
    pub fn flyout_px(self) -> u32 {
        match self {
            Self::Small => 40,
            Self::Medium => 48,
            Self::Large => 64,
        }
    }
}

// "Animation style" (DESIGN.md section 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AnimationStyle {
    #[default]
    Genie,
    Scale,
    Smooth,
}

/// The Launchpad shortcut before DESIGN.md picked Alt + `; nobody could
/// choose it (there was no setting), so a stored one is upgraded.
pub const LEGACY_HOTKEY: &str = "Ctrl+Alt+Space";

/// The default Launchpad shortcut (DESIGN.md section 12).
pub const DEFAULT_HOTKEY: &str = "Alt+`";

/// The "Accent color" value that means "Use Windows accent".
pub const WINDOWS_ACCENT: &str = "windows";

/// The Launchpad grid limits, columns then rows (DESIGN.md default 7 x 5).
pub const GRID_COLUMNS: (u32, u32) = (4, 10);
pub const GRID_ROWS: (u32, u32) = (3, 8);

/// User settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // Changed only through `set_hotkey`, which registers it first.
    pub hotkey: String,
    pub hot_corners: HotCorners,
    // Changed only through `set_autostart`, which writes the Run key.
    pub autostart: bool,
    // Unused: DESIGN.md says light/dark follows Windows.
    pub theme: String,
    pub first_run_done: bool,
    // "windows" or a custom `#RRGGBB`.
    pub accent: String,
    pub background: BackgroundSetting,
    pub folder_shape: FolderShape,
    pub icon_size: IconSize,
    pub grid_columns: u32,
    pub grid_rows: u32,
    pub animation: AnimationStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.into(),
            hot_corners: HotCorners::default(),
            autostart: false,
            theme: "system".into(),
            first_run_done: false,
            accent: WINDOWS_ACCENT.into(),
            background: BackgroundSetting::default(),
            folder_shape: FolderShape::default(),
            icon_size: IconSize::default(),
            grid_columns: 7,
            grid_rows: 5,
            animation: AnimationStyle::default(),
        }
    }
}

/// A partial settings update: only the fields that are set are changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub hot_corners: Option<HotCorners>,
    pub theme: Option<String>,
    pub first_run_done: Option<bool>,
    pub accent: Option<String>,
    pub background: Option<BackgroundSetting>,
    pub folder_shape: Option<FolderShape>,
    pub icon_size: Option<IconSize>,
    pub grid_columns: Option<u32>,
    pub grid_rows: Option<u32>,
    pub animation: Option<AnimationStyle>,
}

/// True for `"windows"` or a `#RRGGBB` colour.
pub fn is_valid_accent(accent: &str) -> bool {
    accent == WINDOWS_ACCENT
        || accent
            .strip_prefix('#')
            .is_some_and(|hex| hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
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

    /// Applies the fields set in `patch`, keeping every other setting. A
    /// grid size is clamped to its limits; an accent that is neither
    /// `"windows"` nor `#RRGGBB` is ignored.
    pub fn apply(&mut self, patch: SettingsPatch) {
        set(&mut self.hot_corners, patch.hot_corners);
        set(&mut self.theme, patch.theme);
        set(&mut self.first_run_done, patch.first_run_done);
        set(
            &mut self.accent,
            patch.accent.filter(|a| is_valid_accent(a)),
        );
        set(&mut self.background, patch.background);
        set(&mut self.folder_shape, patch.folder_shape);
        set(&mut self.icon_size, patch.icon_size);
        let (low, high) = GRID_COLUMNS;
        set(
            &mut self.grid_columns,
            patch.grid_columns.map(|c| c.clamp(low, high)),
        );
        let (low, high) = GRID_ROWS;
        set(
            &mut self.grid_rows,
            patch.grid_rows.map(|r| r.clamp(low, high)),
        );
        set(&mut self.animation, patch.animation);
    }

    /// Items one stored Launchpad page holds: the grid setting, clamped.
    pub fn per_page(&self) -> usize {
        let columns = self.grid_columns.clamp(GRID_COLUMNS.0, GRID_COLUMNS.1);
        let rows = self.grid_rows.clamp(GRID_ROWS.0, GRID_ROWS.1);
        usize::try_from(columns * rows).unwrap_or(35)
    }
}

fn set<T>(field: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *field = value;
    }
}
