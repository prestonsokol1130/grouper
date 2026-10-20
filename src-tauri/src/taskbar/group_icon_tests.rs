//! Tests for [`super::group_icon`]. Pure Rust and temp folders: any OS.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::{Rgba, RgbaImage};

use super::group_icon::{build_group_icon, group_icon_path, remove_group_icons};
use super::ico::ICO_SIZES;
use crate::state::paths::{GROUPS_DIR, ICONS_DIR};
use crate::state::{AppEntry, AppState, Group};

fn app(id: &str) -> AppEntry {
    AppEntry {
        id: id.to_owned(),
        name: id.to_owned(),
        ..AppEntry::default()
    }
}

fn state(app_ids: &[&str]) -> AppState {
    AppState {
        apps: ["a", "d", "e"].into_iter().map(app).collect(),
        groups: vec![Group {
            id: "grp".to_owned(),
            name: "Group".to_owned(),
            app_ids: app_ids.iter().map(|&id| id.to_owned()).collect(),
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

/// The colour in the middle of the 256 px image: the single app's icon.
fn middle_colour(path: &Path) -> [u8; 4] {
    let ico = decode(path);
    let big = ico.entries().iter().find(|e| e.width() == 256).unwrap();
    let rgba = big.decode().unwrap();
    let middle = (128 * 256 + 128) * 4;
    rgba.rgba_data()[middle..middle + 4].try_into().unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn builds_every_size_at_the_stable_path() {
    let dir = tempfile::tempdir().unwrap();
    write_png(dir.path(), "a", [220, 40, 40, 255]);
    let mut state = state(&["a"]);
    state.groups[0].icon_version = 7;
    let path = build_group_icon(&state, "grp", dir.path()).unwrap();
    let groups = dir.path().join(GROUPS_DIR);
    assert_eq!(path, group_icon_path(&groups, "grp"));
    assert!(path.ends_with("grp.ico"));

    let mut sizes: Vec<u32> = decode(&path).entries().iter().map(|e| e.width()).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, ICO_SIZES);
    // The single icon covers the middle of the folder, in the app's colour.
    assert_eq!(middle_colour(&path), [220, 40, 40, 255]);
    // No temp files are left behind.
    assert_eq!(names(&groups), ["grp.ico"]);
}

#[test]
fn a_rebuild_overwrites_the_same_file_and_deletes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let groups = dir.path().join(GROUPS_DIR);
    fs::create_dir_all(&groups).unwrap();
    for name in ["grp-v1.ico", "other.ico"] {
        fs::write(groups.join(name), b"old").unwrap();
    }
    write_png(dir.path(), "a", [220, 40, 40, 255]);
    write_png(dir.path(), "d", [40, 40, 220, 255]);
    let first = build_group_icon(&state(&["a"]), "grp", dir.path()).unwrap();
    let second = build_group_icon(&state(&["d"]), "grp", dir.path()).unwrap();
    assert_eq!(first, second);
    assert_eq!(middle_colour(&second), [40, 40, 220, 255]);
    assert_eq!(names(&groups), ["grp-v1.ico", "grp.ico", "other.ico"]);
}

#[test]
fn missing_and_broken_app_icons_fall_back() {
    let dir = tempfile::tempdir().unwrap();
    // "a" has no PNG; "d" has a file that is not a PNG.
    let icons = dir.path().join(ICONS_DIR);
    fs::create_dir_all(&icons).unwrap();
    fs::write(icons.join("d.png"), b"not a png").unwrap();
    let path = build_group_icon(&state(&["a", "d", "e"]), "grp", dir.path()).unwrap();
    assert_eq!(decode(&path).entries().len(), ICO_SIZES.len());
}

#[test]
fn empty_group_builds_an_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = build_group_icon(&state(&[]), "grp", dir.path()).unwrap();
    assert!(path.is_file());
}

#[test]
fn unknown_or_unsafe_groups_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(&["a"]);
    assert!(build_group_icon(&state, "nope", dir.path()).is_err());
    assert!(build_group_icon(&state, "../grp", dir.path()).is_err());
    assert!(build_group_icon(&state, "", dir.path()).is_err());
    assert!(!dir.path().join(GROUPS_DIR).exists());
}

/// A `groups` folder holding `grp`'s current and legacy icons plus files
/// that are not `grp`'s icons.
fn groups_folder() -> (tempfile::TempDir, std::path::PathBuf, [&'static str; 5]) {
    let dir = tempfile::tempdir().unwrap();
    let groups = dir.path().join(GROUPS_DIR);
    fs::create_dir_all(&groups).unwrap();
    let others = [
        "other-v1.ico",
        "grp-vx.ico",
        "grp-v2.png",
        "grp2-v1.ico",
        "grp2.ico",
    ];
    for name in ["grp.ico", "grp-v0.ico", "grp-v1.ico", "grp-v12.ico"]
        .into_iter()
        .chain(others)
    {
        fs::write(groups.join(name), b"old").unwrap();
    }
    (dir, groups, others)
}

#[test]
fn every_icon_of_a_deleted_group_is_removed() {
    let (dir, groups, others) = groups_folder();
    // An unsafe id or a missing folder is a logged no-op.
    remove_group_icons(dir.path(), "../grp");
    remove_group_icons(&dir.path().join("nowhere"), "grp");
    assert_eq!(names(&groups).len(), others.len() + 4);
    remove_group_icons(dir.path(), "grp");
    let mut expected: Vec<String> = others.iter().map(|&n| n.to_owned()).collect();
    expected.sort();
    assert_eq!(names(&groups), expected);
}
