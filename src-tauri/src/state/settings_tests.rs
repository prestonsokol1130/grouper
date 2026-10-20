//! Tests for the user settings: defaults, old files, patches and the
//! round trip through `app_state.json`.

use super::model::AppState;
use super::settings::*;
use super::store::Store;

#[test]
fn defaults_follow_design_md() {
    let settings = Settings::default();
    assert_eq!(settings.hotkey, "Alt+`");
    assert_eq!(settings.accent, "windows");
    assert_eq!(settings.background, BackgroundSetting::Mixed);
    assert_eq!(settings.folder_shape, FolderShape::Square);
    assert_eq!(settings.icon_size, IconSize::Medium);
    assert_eq!((settings.grid_columns, settings.grid_rows), (7, 5));
    assert_eq!(settings.animation, AnimationStyle::Genie);
    assert_eq!(settings.per_page(), 35);
    assert_eq!(settings.flyout_layout, FlyoutGrid::G3x3);
    assert_eq!(settings.flyout_icon_size, FlyoutIconSize::MatchTaskbar);
    assert!(!settings.show_page_arrows && !settings.always_show_focus);
}

#[test]
fn an_old_settings_object_loads_with_new_defaults() {
    let json = r#"{"hotkey":"Shift+Alt+L","hotCorners":{"topLeft":false},
        "autostart":true,"theme":"system","firstRunDone":true}"#;
    let settings: Settings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.hotkey, "Shift+Alt+L");
    assert!(settings.autostart && settings.first_run_done);
    assert!(!settings.hot_corners.top_left);
    assert_eq!(settings.icon_size, IconSize::Medium);
    assert_eq!(settings.grid_columns, 7);
    assert_eq!(settings.accent, "windows");
    // The flyout settings came later: an older file gets their defaults.
    assert_eq!(settings.flyout_layout, FlyoutGrid::G3x3);
    assert_eq!(settings.flyout_icon_size, FlyoutIconSize::MatchTaskbar);
    assert!(!settings.show_page_arrows && !settings.always_show_focus);
}

#[test]
fn enum_values_use_the_frontend_names() {
    let json = serde_json::to_value(Settings {
        background: BackgroundSetting::Acrylic,
        folder_shape: FolderShape::Squircle,
        icon_size: IconSize::Large,
        animation: AnimationStyle::Smooth,
        ..Settings::default()
    })
    .unwrap();
    assert_eq!(json["background"], "acrylic");
    assert_eq!(json["folderShape"], "squircle");
    assert_eq!(json["iconSize"], "large");
    assert_eq!(json["animation"], "smooth");
    assert_eq!(json["gridColumns"], 7);
    assert_eq!(json["flyoutLayout"], "3x3");
    assert_eq!(json["flyoutIconSize"], "matchTaskbar");
    assert_eq!(json["showPageArrows"], false);
    assert_eq!(json["alwaysShowFocus"], false);
    let names: Vec<_> = FlyoutGrid::ALL
        .iter()
        .map(|grid| serde_json::to_value(grid).unwrap())
        .collect();
    assert_eq!(names, ["3x3", "4x4", "5x5", "4x2", "3x2", "2x2"]);
}

#[test]
fn patches_change_only_their_fields_and_clamp_the_grid() {
    let mut settings = Settings::default();
    let patch: SettingsPatch =
        serde_json::from_str(r#"{"gridColumns": 99, "gridRows": 0, "iconSize": "small"}"#).unwrap();
    settings.apply(patch);
    assert_eq!((settings.grid_columns, settings.grid_rows), (10, 3));
    assert_eq!(settings.icon_size, IconSize::Small);
    assert_eq!(settings.hotkey, "Alt+`");
    assert_eq!(settings.per_page(), 30);
    let patch: SettingsPatch =
        serde_json::from_str(r#"{"flyoutLayout": "5x5", "showPageArrows": true}"#).unwrap();
    settings.apply(patch);
    assert_eq!(settings.flyout_layout, FlyoutGrid::G5x5);
    assert!(settings.show_page_arrows && !settings.always_show_focus);
    assert_eq!(settings.flyout_icon_size, FlyoutIconSize::MatchTaskbar);
}

#[test]
fn only_windows_or_hex_accents_are_taken() {
    let mut settings = Settings::default();
    for (accent, kept) in [
        ("#12AbEf", "#12AbEf"),
        ("red", "#12AbEf"),
        ("#12345", "#12AbEf"),
        ("windows", "windows"),
    ] {
        settings.apply(SettingsPatch {
            accent: Some(accent.into()),
            ..SettingsPatch::default()
        });
        assert_eq!(settings.accent, kept, "{accent}");
    }
}

#[test]
fn icon_sizes_match_design_md() {
    let sizes: Vec<u32> = [IconSize::Small, IconSize::Medium, IconSize::Large]
        .iter()
        .map(|size| size.launchpad_px())
        .collect();
    assert_eq!(sizes, [56, 72, 96]);
    let flyout: Vec<Option<u32>> = [
        FlyoutIconSize::MatchTaskbar,
        FlyoutIconSize::Small,
        FlyoutIconSize::Medium,
        FlyoutIconSize::Large,
    ]
    .iter()
    .map(|size| size.fixed_px())
    .collect();
    assert_eq!(flyout, [None, Some(40), Some(48), Some(64)]);
}

#[test]
fn settings_round_trip_through_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::load(dir.path()).unwrap();
    store
        .update(|state: &mut AppState| {
            state.settings.apply(SettingsPatch {
                first_run_done: Some(true),
                accent: Some("#FF8800".into()),
                background: Some(BackgroundSetting::Solid),
                folder_shape: Some(FolderShape::Squircle),
                icon_size: Some(IconSize::Large),
                grid_columns: Some(8),
                grid_rows: Some(6),
                animation: Some(AnimationStyle::Scale),
                flyout_layout: Some(FlyoutGrid::G4x2),
                flyout_icon_size: Some(FlyoutIconSize::Large),
                show_page_arrows: Some(true),
                always_show_focus: Some(true),
                ..SettingsPatch::default()
            });
            Ok(())
        })
        .unwrap();
    store.save().unwrap();
    let saved = store.snapshot().settings;
    let reloaded = Store::load(dir.path()).unwrap().snapshot().settings;
    assert_eq!(reloaded, saved);
    assert!(reloaded.first_run_done);
    assert_eq!(reloaded.per_page(), 48);
    assert_eq!(reloaded.flyout_layout, FlyoutGrid::G4x2);
    assert_eq!(reloaded.flyout_icon_size, FlyoutIconSize::Large);
    assert!(reloaded.show_page_arrows && reloaded.always_show_focus);
}
