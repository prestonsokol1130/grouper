//! Tests for the state model and its pure helpers.

use super::ids::app_id;
use super::model::*;

fn app(id: &str) -> AppEntry {
    AppEntry {
        id: id.into(),
        name: format!("App {id}"),
        target: format!(r"C:\Apps\{id}.exe"),
        ..AppEntry::default()
    }
}

fn app_item(id: &str) -> LaunchpadItem {
    LaunchpadItem::App { app_id: id.into() }
}

fn folder_item(id: &str) -> LaunchpadItem {
    LaunchpadItem::Folder {
        group_id: id.into(),
    }
}

fn populated() -> AppState {
    AppState {
        apps: vec![
            app("a"),
            app("b"),
            AppEntry {
                source: AppSource::Manual,
                lnk_path: Some(r"C:\x.lnk".into()),
                missing: true,
                hidden: true,
                args: "--flag".into(),
                working_dir: r"C:\Apps".into(),
                ..app("c")
            },
        ],
        groups: vec![Group {
            id: "g1".into(),
            name: "Work".into(),
            app_ids: vec!["a".into(), "b".into()],
            icon_version: 3,
            ..Group::default()
        }],
        launchpad: LaunchpadLayout {
            pages: vec![vec![folder_item("g1"), app_item("c")], vec![app_item("b")]],
        },
        settings: Settings {
            autostart: true,
            ..Settings::default()
        },
        ..AppState::default()
    }
}

#[test]
fn populated_state_round_trips() {
    let state = populated();
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("\"kind\":\"folder\""));
    assert!(json.contains("\"groupId\":\"g1\""));
    assert!(json.contains("\"schemaVersion\":1"));
    assert!(json.contains("\"source\":\"manual\""));
    let back: AppState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, state);
}

#[test]
fn empty_object_gives_defaults() {
    let state: AppState = serde_json::from_str("{}").unwrap();
    assert_eq!(state, AppState::default());
    assert_eq!(state.schema_version, CURRENT_SCHEMA);
    assert_eq!(state.settings.hotkey, "Ctrl+Alt+Space");
    assert_eq!(state.settings.theme, "system");
    assert!(!state.settings.autostart && !state.settings.first_run_done);
    let partial: AppEntry = serde_json::from_str(r#"{"id":"x"}"#).unwrap();
    assert_eq!(partial.source, AppSource::StartMenu);
    assert_eq!(partial.lnk_path, None);
}

#[test]
fn lookups() {
    let state = populated();
    assert_eq!(state.app_by_id("b").map(|a| a.name.as_str()), Some("App b"));
    assert!(state.app_by_id("zzz").is_none());
    assert_eq!(state.group_by_id("g1").map(|g| g.icon_version), Some(3));
    assert!(state.group_by_id("nope").is_none());
}

#[test]
fn remove_app_clears_every_reference() {
    let mut state = populated();
    assert_eq!(state.remove_app("b").map(|a| a.id), Some("b".into()));
    assert!(state.app_by_id("b").is_none());
    assert_eq!(state.groups[0].app_ids, vec!["a".to_string()]);
    assert_eq!(state.launchpad.pages[1], Vec::<LaunchpadItem>::new());
    assert!(state.remove_app("b").is_none());
}

#[test]
fn remove_group_unfolds_its_apps_in_place() {
    let mut state = populated();
    assert_eq!(
        state.remove_group("g1").map(|g| g.name),
        Some("Work".into())
    );
    assert!(state.groups.is_empty());
    // "b" is already on page 2, so only "a" replaces the folder.
    assert_eq!(state.launchpad.pages[0], vec![app_item("a"), app_item("c")]);
    assert_eq!(state.launchpad.pages[1], vec![app_item("b")]);
    assert!(state.remove_group("g1").is_none());
}

#[test]
fn validate_drops_dangling_and_duplicate_ids() {
    let mut state = populated();
    state.groups[0].app_ids.push("ghost".into());
    state.groups[0].app_ids.push("a".into());
    state.launchpad.pages[0].push(app_item("ghost"));
    state.launchpad.pages[0].push(folder_item("no-group"));
    state.launchpad.pages[1].push(app_item("c"));
    assert_eq!(state.validate(), 5);
    // Group g1 lost apps, so its icon is due for a rebuild.
    let mut expected = populated();
    expected.groups[0].icon_version += 1;
    assert_eq!(state, expected);
    assert_eq!(state.validate(), 0);
    assert_eq!(
        state.groups[0].icon_version,
        expected.groups[0].icon_version
    );
}

#[test]
fn remove_app_bumps_only_groups_that_held_it() {
    let mut state = populated();
    state.groups.push(Group {
        id: "g2".into(),
        name: "Other".into(),
        app_ids: vec!["c".into()],
        icon_version: 7,
        ..Group::default()
    });
    state.remove_app("a");
    assert_eq!(state.group_by_id("g1").map(|g| g.icon_version), Some(4));
    assert_eq!(state.group_by_id("g2").map(|g| g.icon_version), Some(7));
}

#[test]
fn set_group_apps_drops_unknown_and_repeated_ids() {
    let mut state = populated();
    let group = state
        .set_group_apps(
            "g1",
            vec!["c".into(), "ghost".into(), "c".into(), "a".into()],
        )
        .unwrap();
    assert_eq!(group.app_ids, vec!["c".to_string(), "a".to_string()]);
    assert_eq!(group.icon_version, 4);
    assert!(state.set_group_apps("nope", Vec::new()).is_none());
}

#[test]
fn set_group_apps_bumps_the_icon_version_only_when_the_icon_changes() {
    let mut state = populated();
    // "c" is missing and hidden, so the folder still shows "a" and "b".
    let group = state
        .set_group_apps("g1", vec!["a".into(), "c".into(), "b".into()])
        .unwrap();
    assert_eq!(group.icon_version, 3);
    let group = state
        .set_group_apps("g1", vec!["b".into(), "a".into()])
        .unwrap();
    assert_eq!(group.icon_version, 4);
    assert_eq!(state.group_by_id("g1"), Some(&group));
}

#[test]
fn app_id_matches_entry_ids() {
    let id = app_id(r"C:\Program Files\App\App.EXE", "-x");
    assert_eq!(id, app_id(r"c:\program files\app\app.exe", "-x"));
}

#[test]
fn settings_patch_changes_only_given_fields() {
    let mut settings = Settings {
        hotkey: "Alt+Space".into(),
        autostart: true,
        ..Settings::default()
    };
    let patch: SettingsPatch = serde_json::from_str(r#"{"theme": "dark"}"#).unwrap();
    settings.apply(patch);
    assert_eq!(settings.theme, "dark");
    assert_eq!(settings.hotkey, "Alt+Space");
    assert!(settings.autostart);
    assert!(!settings.first_run_done);
}
