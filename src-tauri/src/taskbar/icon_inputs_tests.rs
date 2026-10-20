//! Tests for [`super::icon_inputs`]: which apps a folder icon shows and
//! when that changes. Pure: any OS.

use super::icon_inputs::{changed_icon_groups, group_icon_inputs, needs_icon_rebuild};
use crate::state::{AppEntry, AppState, Group};

fn app(id: &str, missing: bool, hidden: bool) -> AppEntry {
    AppEntry {
        id: id.to_owned(),
        name: id.to_owned(),
        missing,
        hidden,
        ..AppEntry::default()
    }
}

/// "b" is missing and "c" hidden; the other apps are visible.
fn state(app_ids: &[&str]) -> AppState {
    AppState {
        apps: vec![
            app("a", false, false),
            app("b", true, false),
            app("c", false, true),
            app("d", false, false),
            app("e", false, false),
            app("f", false, false),
            app("g", false, false),
        ],
        groups: vec![Group {
            id: "grp".to_owned(),
            name: "Group".to_owned(),
            app_ids: app_ids.iter().map(|&id| id.to_owned()).collect(),
            ..Group::default()
        }],
        ..AppState::default()
    }
}

/// Group "grp" from [`state`] with its apps set to `app_ids`.
fn with_apps(state: &AppState, app_ids: &[&str]) -> Group {
    Group {
        app_ids: app_ids.iter().map(|&id| id.to_owned()).collect(),
        ..state.groups[0].clone()
    }
}

fn set_flags(state: &mut AppState, id: &str, missing: bool, hidden: bool) {
    let app = state.apps.iter_mut().find(|app| app.id == id).unwrap();
    app.missing = missing;
    app.hidden = hidden;
}

#[test]
fn inputs_are_the_first_four_visible_apps() {
    let state = state(&["a", "b", "c", "zz", "d", "e", "f", "g"]);
    assert_eq!(group_icon_inputs(&state, "grp"), ["a", "d", "e", "f"]);
    assert!(group_icon_inputs(&state, "nope").is_empty());
}

#[test]
fn rebuild_when_the_first_four_visible_apps_change() {
    let state = state(&[]);
    let old = with_apps(&state, &["a", "d", "e", "f", "g"]);
    assert!(!needs_icon_rebuild(&old, &old.clone(), &state));
    let added = with_apps(&state, &["g", "a", "d", "e"]);
    assert!(needs_icon_rebuild(&old, &added, &state));
    let removed = with_apps(&state, &["a", "d", "e"]);
    assert!(needs_icon_rebuild(&old, &removed, &state));
    // An order change inside the first four counts.
    let reordered = with_apps(&state, &["d", "a", "e", "f", "g"]);
    assert!(needs_icon_rebuild(&old, &reordered, &state));
}

#[test]
fn no_rebuild_for_changes_after_the_fourth_visible_app() {
    let state = state(&[]);
    let four = with_apps(&state, &["a", "d", "e", "f"]);
    let appended = with_apps(&state, &["a", "d", "e", "f", "g"]);
    assert!(!needs_icon_rebuild(&four, &appended, &state));
    let tail = with_apps(&state, &["a", "d", "e", "f", "g", "zz"]);
    let tail_changed = with_apps(&state, &["a", "d", "e", "f", "zz"]);
    assert!(!needs_icon_rebuild(&tail, &tail_changed, &state));
}

#[test]
fn hidden_missing_and_unknown_apps_are_skipped() {
    let state = state(&[]);
    let old = with_apps(&state, &["a", "d"]);
    let padded = with_apps(&state, &["b", "a", "c", "d", "zz"]);
    assert!(!needs_icon_rebuild(&old, &padded, &state));
    // A hidden app does not take one of the four slots.
    let five = with_apps(&state, &["a", "d", "e", "f", "g"]);
    let hidden_inside = with_apps(&state, &["a", "c", "d", "e", "f", "g"]);
    assert!(!needs_icon_rebuild(&five, &hidden_inside, &state));
}

#[test]
fn hiding_or_losing_an_app_changes_the_icon_only_inside_the_first_four() {
    let before = state(&["a", "d", "e", "f", "g"]);
    for (missing, hidden) in [(false, true), (true, false)] {
        let mut after = before.clone();
        set_flags(&mut after, "d", missing, hidden);
        assert_eq!(changed_icon_groups(&before, &after), ["grp"]);
        // "g" is fifth: not shown, so the icon is unchanged.
        let mut after = before.clone();
        set_flags(&mut after, "g", missing, hidden);
        assert!(changed_icon_groups(&before, &after).is_empty());
    }
}

#[test]
fn an_app_shown_again_changes_the_icon_only_when_it_moves_into_the_first_four() {
    // "b" (missing) and "c" (hidden) sit inside the group's first four.
    let before = state(&["a", "b", "c", "d", "e", "f"]);
    let mut after = before.clone();
    set_flags(&mut after, "c", false, false);
    assert_eq!(changed_icon_groups(&before, &after), ["grp"]);
    // Behind four visible apps they are never shown.
    let before = state(&["a", "d", "e", "f", "b", "c"]);
    let mut after = before.clone();
    set_flags(&mut after, "b", false, false);
    set_flags(&mut after, "c", false, false);
    assert!(changed_icon_groups(&before, &after).is_empty());
}

#[test]
fn removed_apps_and_new_or_deleted_groups() {
    let before = state(&["a", "d", "e", "f", "g"]);
    let mut after = before.clone();
    after.apps.retain(|app| app.id != "g");
    assert!(changed_icon_groups(&before, &after).is_empty());
    after.apps.retain(|app| app.id != "a");
    assert_eq!(changed_icon_groups(&before, &after), ["grp"]);
    // Only groups in both states are reported.
    let mut renamed_id = before.clone();
    renamed_id.groups[0].id = "new".to_owned();
    renamed_id.groups[0].app_ids.clear();
    assert!(changed_icon_groups(&before, &renamed_id).is_empty());
}
