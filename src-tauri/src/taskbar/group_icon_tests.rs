//! Tests for [`super::group_icon`]. Pure Rust and temp folders: any OS.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::{Rgba, RgbaImage};

use super::group_icon::{
    build_group_icon, group_icon_inputs, group_icon_path, needs_icon_rebuild, remove_group_icons,
};
use super::ico::ICO_SIZES;
use crate::state::paths::{GROUPS_DIR, ICONS_DIR};
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

fn state(app_ids: &[&str], icon_version: u32) -> AppState {
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
            icon_version,
            ..Group::default()
        }],
        ..AppState::default()
    }
}

fn write_png(dir: &Path, app_id: &str, colour: [u8; 4]) {
    let icons = dir.join(ICONS_DIR);
    fs::create_dir_all(&icons).unwrap();
    RgbaImage::from_pixel(256, 256, Rgba(colour))
        .save(icons.join(format!("{app_id}.png")))
        .unwrap();
}

fn decode(path: &Path) -> ::ico::IconDir {
    ::ico::IconDir::read(Cursor::new(fs::read(path).unwrap())).unwrap()
}

#[test]
fn inputs_are_the_first_four_visible_apps() {
    let state = state(&["a", "b", "c", "zz", "d", "e", "f", "g"], 0);
    assert_eq!(group_icon_inputs(&state, "grp"), ["a", "d", "e", "f"]);
    assert!(group_icon_inputs(&state, "nope").is_empty());
}

#[test]
fn builds_every_size_at_the_versioned_path() {
    let dir = tempfile::tempdir().unwrap();
    write_png(dir.path(), "a", [220, 40, 40, 255]);
    let path = build_group_icon(&state(&["a"], 7), "grp", dir.path()).unwrap();
    let groups = dir.path().join(GROUPS_DIR);
    assert_eq!(path, group_icon_path(&groups, "grp", 7));
    assert!(path.ends_with("grp-v7.ico"));

    let ico = decode(&path);
    let mut sizes: Vec<u32> = ico.entries().iter().map(|e| e.width()).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, ICO_SIZES);
    // The single icon covers the middle of the folder, in the app's colour.
    let big = ico.entries().iter().find(|e| e.width() == 256).unwrap();
    let rgba = big.decode().unwrap();
    let middle = (128 * 256 + 128) * 4;
    assert_eq!(rgba.rgba_data()[middle..middle + 4], [220, 40, 40, 255]);
    // No temp files are left behind.
    let names: Vec<_> = fs::read_dir(&groups).unwrap().flatten().collect();
    assert_eq!(names.len(), 1);
}

#[test]
fn older_versions_of_the_group_are_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let groups = dir.path().join(GROUPS_DIR);
    fs::create_dir_all(&groups).unwrap();
    let kept_names = ["other-v1.ico", "grp-v1-v2.ico", "grp-vx.ico", "grp-v2.png"];
    for name in ["grp-v1.ico", "grp-v2.ico", "grp-v9.ico"]
        .into_iter()
        .chain(kept_names)
    {
        fs::write(groups.join(name), b"old").unwrap();
    }
    let path = build_group_icon(&state(&["a", "d"], 3), "grp", dir.path()).unwrap();

    let mut left: Vec<String> = fs::read_dir(&groups)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    let mut expected: Vec<String> = kept_names.iter().map(|&n| n.to_owned()).collect();
    expected.push("grp-v3.ico".to_owned());
    expected.sort();
    assert_eq!(left, expected);
    assert!(decode(&path).entries().len() == ICO_SIZES.len());
}

#[test]
fn missing_and_broken_app_icons_fall_back() {
    let dir = tempfile::tempdir().unwrap();
    // "a" has no PNG; "d" has a file that is not a PNG.
    let icons = dir.path().join(ICONS_DIR);
    fs::create_dir_all(&icons).unwrap();
    fs::write(icons.join("d.png"), b"not a png").unwrap();
    let path = build_group_icon(&state(&["a", "d", "e"], 1), "grp", dir.path()).unwrap();
    assert_eq!(decode(&path).entries().len(), ICO_SIZES.len());
}

#[test]
fn empty_group_builds_an_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = build_group_icon(&state(&[], 0), "grp", dir.path()).unwrap();
    assert!(path.is_file());
}

#[test]
fn unknown_or_unsafe_groups_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(&["a"], 0);
    assert!(build_group_icon(&state, "nope", dir.path()).is_err());
    assert!(build_group_icon(&state, "../grp", dir.path()).is_err());
    assert!(build_group_icon(&state, "", dir.path()).is_err());
    assert!(!dir.path().join(GROUPS_DIR).exists());
}

/// Group "grp" from [`state`] with its apps set to `app_ids`.
fn with_apps(state: &AppState, app_ids: &[&str]) -> Group {
    Group {
        app_ids: app_ids.iter().map(|&id| id.to_owned()).collect(),
        ..state.groups[0].clone()
    }
}

#[test]
fn rebuild_when_the_first_four_visible_apps_change() {
    // "b" is missing and "c" hidden; the rest are visible.
    let state = state(&[], 0);
    let old = with_apps(&state, &["a", "d", "e", "f", "g"]);
    assert!(!needs_icon_rebuild(&old, &old.clone(), &state));
    // A new app inside the first four.
    let added = with_apps(&state, &["g", "a", "d", "e"]);
    assert!(needs_icon_rebuild(&old, &added, &state));
    // An app gone from the first four.
    let removed = with_apps(&state, &["a", "d", "e"]);
    assert!(needs_icon_rebuild(&old, &removed, &state));
    // An order change inside the first four counts.
    let reordered = with_apps(&state, &["d", "a", "e", "f", "g"]);
    assert!(needs_icon_rebuild(&old, &reordered, &state));
}

#[test]
fn no_rebuild_for_changes_after_the_fourth_visible_app() {
    let state = state(&[], 0);
    let four = with_apps(&state, &["a", "d", "e", "f"]);
    let appended = with_apps(&state, &["a", "d", "e", "f", "g"]);
    assert!(!needs_icon_rebuild(&four, &appended, &state));
    // Reordering and removing after the fourth visible app.
    let tail = with_apps(&state, &["a", "d", "e", "f", "g", "zz"]);
    let tail_changed = with_apps(&state, &["a", "d", "e", "f", "zz"]);
    assert!(!needs_icon_rebuild(&tail, &tail_changed, &state));
}

#[test]
fn hidden_missing_and_unknown_apps_are_skipped() {
    let state = state(&[], 0);
    let old = with_apps(&state, &["a", "d"]);
    // Adding a missing ("b"), hidden ("c") or unknown ("zz") app changes
    // nothing.
    let padded = with_apps(&state, &["b", "a", "c", "d", "zz"]);
    assert!(!needs_icon_rebuild(&old, &padded, &state));
    // A hidden app does not take one of the four slots: "f" is still the
    // fourth visible app, and "g" still out of the icon.
    let five = with_apps(&state, &["a", "d", "e", "f", "g"]);
    let hidden_inside = with_apps(&state, &["a", "c", "d", "e", "f", "g"]);
    assert!(!needs_icon_rebuild(&five, &hidden_inside, &state));
}

#[test]
fn every_version_of_a_deleted_group_is_removed() {
    let dir = tempfile::tempdir().unwrap();
    let groups = dir.path().join(GROUPS_DIR);
    fs::create_dir_all(&groups).unwrap();
    let kept_names = ["other-v1.ico", "grp-vx.ico", "grp-v2.png", "grp2-v1.ico"];
    for name in ["grp-v0.ico", "grp-v1.ico", "grp-v12.ico"]
        .into_iter()
        .chain(kept_names)
    {
        fs::write(groups.join(name), b"old").unwrap();
    }
    remove_group_icons(dir.path(), "grp");
    // An unsafe id or a missing folder is a logged no-op.
    remove_group_icons(dir.path(), "../grp");
    remove_group_icons(&dir.path().join("nowhere"), "grp");

    let mut left: Vec<String> = fs::read_dir(&groups)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    let mut expected: Vec<String> = kept_names.iter().map(|&n| n.to_owned()).collect();
    expected.sort();
    assert_eq!(left, expected);
}
