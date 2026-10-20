//! Tests for [`super::group_icon`]. Pure Rust and temp folders: any OS.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::{Rgba, RgbaImage};

use super::group_icon::{build_group_icon, group_icon_inputs, group_icon_path};
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
