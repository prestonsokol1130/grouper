//! Tests for the startup icon cache clean-up, on temp folders.

use std::fs;
use std::path::Path;

use super::{collect, Collected};
use crate::state::paths::{ensure_dirs, group_cache_dir, icon_cache_dir};
use crate::state::{AppEntry, AppState, Group};

const KEPT_APP: &str = "0123456789abcdef";
const GONE_APP: &str = "fedcba9876543210";
const KEPT_GROUP: &str = "Kept_grp-1";
const GONE_GROUP: &str = "Gone_grp-2";

fn state() -> AppState {
    AppState {
        apps: vec![AppEntry {
            id: KEPT_APP.into(),
            ..AppEntry::default()
        }],
        groups: vec![Group {
            id: KEPT_GROUP.into(),
            ..Group::default()
        }],
        ..AppState::default()
    }
}

fn touch(dir: &Path, name: &str) {
    fs::write(dir.join(name), b"x").unwrap();
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn removes_only_stale_files_that_match_grouper_names() {
    let data = tempfile::tempdir().unwrap();
    ensure_dirs(data.path()).unwrap();
    let icons = icon_cache_dir(data.path());
    let groups = group_cache_dir(data.path());
    for name in [
        &format!("{KEPT_APP}.png"),
        &format!("{GONE_APP}.png"),
        "_fallback.png",
        "ABCDEF0000000000.png",
        "notes.txt",
        "abc.png",
        &format!("{GONE_APP}.png.0011223344556677.tmp"),
    ] {
        touch(&icons, name);
    }
    for name in [
        &format!("{KEPT_GROUP}.ico"),
        &format!("{KEPT_GROUP}-v3.ico"),
        &format!("{GONE_GROUP}.ico"),
        &format!("{GONE_GROUP}-v12.ico"),
        "some file.ico",
        "notes.ico",
        "Gone_grp-2-v.ico",
        "readme.md",
    ] {
        touch(&groups, name);
    }
    fs::create_dir(icons.join("aaaaaaaaaaaaaaaa.png")).unwrap();

    let collected = collect(data.path(), &state());

    assert_eq!(collected, Collected { pngs: 1, icos: 2 });
    let mut kept_icons = vec![
        format!("{KEPT_APP}.png"),
        "ABCDEF0000000000.png".into(),
        "_fallback.png".into(),
        "abc.png".into(),
        format!("{GONE_APP}.png.0011223344556677.tmp"),
        "aaaaaaaaaaaaaaaa.png".into(),
        "notes.txt".into(),
    ];
    kept_icons.sort();
    assert_eq!(names(&icons), kept_icons);
    let mut kept_groups = vec![
        format!("{KEPT_GROUP}-v3.ico"),
        format!("{KEPT_GROUP}.ico"),
        "Gone_grp-2-v.ico".into(),
        "notes.ico".into(),
        "readme.md".into(),
        "some file.ico".into(),
    ];
    kept_groups.sort();
    assert_eq!(names(&groups), kept_groups);
}

#[test]
fn a_group_id_that_looks_versioned_keeps_its_icon() {
    let data = tempfile::tempdir().unwrap();
    ensure_dirs(data.path()).unwrap();
    let groups = group_cache_dir(data.path());
    let mut state = state();
    state.groups.push(Group {
        id: "abcdef-v12".into(),
        ..Group::default()
    });
    state.groups.push(Group {
        id: "Kept_grp-1-v4".into(),
        ..Group::default()
    });
    touch(&groups, "abcdef-v12.ico");
    touch(&groups, "Kept_grp-1-v4.ico");
    assert_eq!(collect(data.path(), &state).icos, 0);
    assert_eq!(
        names(&groups),
        vec!["Kept_grp-1-v4.ico".to_owned(), "abcdef-v12.ico".to_owned()]
    );
}

#[test]
fn missing_cache_folders_are_fine() {
    let data = tempfile::tempdir().unwrap();
    assert_eq!(collect(data.path(), &state()), Collected::default());
    assert!(!icon_cache_dir(data.path()).exists());
}

#[test]
fn an_empty_state_cleans_every_grouper_file() {
    let data = tempfile::tempdir().unwrap();
    ensure_dirs(data.path()).unwrap();
    touch(&icon_cache_dir(data.path()), &format!("{KEPT_APP}.png"));
    touch(&group_cache_dir(data.path()), &format!("{KEPT_GROUP}.ico"));
    let collected = collect(data.path(), &AppState::default());
    assert_eq!(collected, Collected { pngs: 1, icos: 1 });
}

/// Symlinks need Developer Mode or admin on Windows; without them the
/// symlink tests are skipped (and say so).
#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
}

#[cfg(unix)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[test]
fn never_deletes_through_a_symlinked_file() {
    let data = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    ensure_dirs(data.path()).unwrap();
    let victim = outside.path().join("victim.png");
    touch(outside.path(), "victim.png");
    let link = icon_cache_dir(data.path()).join(format!("{GONE_APP}.png"));
    if !link_file(&victim, &link) {
        eprintln!("skipped: cannot create symlinks here");
        return;
    }
    assert_eq!(collect(data.path(), &state()).pngs, 0);
    assert!(victim.is_file());
    assert!(fs::symlink_metadata(&link).is_ok());
}

#[test]
fn never_cleans_a_symlinked_cache_folder() {
    let data = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(group_cache_dir(data.path())).unwrap();
    touch(outside.path(), &format!("{GONE_APP}.png"));
    if !link_dir(outside.path(), &icon_cache_dir(data.path())) {
        eprintln!("skipped: cannot create symlinks here");
        return;
    }
    assert_eq!(collect(data.path(), &state()).pngs, 0);
    assert!(outside.path().join(format!("{GONE_APP}.png")).is_file());
}
