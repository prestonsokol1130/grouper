//! Windows-only tests for [`super::shortcut_sync`]: real shortcuts and
//! `.ico` files in temp folders.

use std::fs;
use std::path::{Path, PathBuf};

use super::shortcut_sync::{
    remove_group_files, repair_group_shortcut, update_group_shortcut, Change, Repair,
};
use super::shortcut_write::write_group_shortcut;
use crate::state::paths::GROUPS_DIR;
use crate::state::{AppState, Group};
use crate::win32::com::ComGuard;
use crate::win32::lnk_aumid::read_lnk_aumid;
use crate::win32::lnk_read::resolve_lnk;

const RENAME: Change = Change {
    renamed: true,
    icon: false,
};
const ICON: Change = Change {
    renamed: false,
    icon: true,
};

fn exe() -> PathBuf {
    std::env::current_exe().unwrap()
}

fn named(id: &str, name: &str) -> Group {
    Group {
        id: id.to_owned(),
        name: name.to_owned(),
        has_shortcut: true,
        ..Group::default()
    }
}

/// A temp folder with `cache` and `menu` (the Start Menu folder) inside.
fn folders() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let (cache, menu) = (temp.path().join("cache"), temp.path().join("menu"));
    fs::create_dir_all(&menu).unwrap();
    (temp, cache, menu)
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn aumid(lnk: &Path) -> Option<String> {
    read_lnk_aumid(lnk).unwrap()
}

fn icon_of(lnk: &Path) -> String {
    resolve_lnk(lnk).unwrap().icon_location.unwrap().0
}

#[test]
fn a_rename_replaces_only_the_groups_own_shortcut() {
    let _com = ComGuard::new().unwrap();
    let (_temp, cache, menu) = folders();
    let mut state = AppState {
        groups: vec![named("ga", "Alpha"), named("gb", "Beta")],
        ..AppState::default()
    };
    write_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    let beta = write_group_shortcut(&state, "gb", &cache, &menu, &exe()).unwrap();
    fs::write(menu.join("Junk.lnk"), b"not a shortcut").unwrap();
    let icon = cache.join(GROUPS_DIR).join("ga-v0.ico");
    let icon_bytes = fs::read(&icon).unwrap();

    state.groups[0].name = "Gamma".to_owned();
    let lnk = update_group_shortcut(&state, "ga", RENAME, &cache, &menu, &exe()).unwrap();
    assert_eq!(lnk, menu.join("Gamma.lnk"));
    assert_eq!(names(&menu), ["Beta.lnk", "Gamma.lnk", "Junk.lnk"]);
    assert_eq!(aumid(&lnk).as_deref(), Some("com.grouper.group.ga"));
    assert_eq!(aumid(&beta).as_deref(), Some("com.grouper.group.gb"));
    assert_eq!(fs::read(menu.join("Junk.lnk")).unwrap(), b"not a shortcut");
    // The icon is reused as it was, not rebuilt.
    assert_eq!(icon_of(&lnk), icon.to_str().unwrap());
    assert_eq!(fs::read(&icon).unwrap(), icon_bytes);

    // A change of letter case renames the file too.
    state.groups[0].name = "GAMMA".to_owned();
    update_group_shortcut(&state, "ga", RENAME, &cache, &menu, &exe()).unwrap();
    assert_eq!(names(&menu), ["Beta.lnk", "GAMMA.lnk", "Junk.lnk"]);

    // Renamed onto another group's name, it never replaces that shortcut.
    state.groups[0].name = "Beta".to_owned();
    let moved = update_group_shortcut(&state, "ga", RENAME, &cache, &menu, &exe()).unwrap();
    assert_eq!(moved, menu.join("Beta (2).lnk"));
    assert_eq!(aumid(&beta).as_deref(), Some("com.grouper.group.gb"));
    assert_eq!(names(&menu), ["Beta (2).lnk", "Beta.lnk", "Junk.lnk"]);
}

#[test]
fn an_icon_change_builds_the_next_version_and_points_the_shortcut_at_it() {
    let _com = ComGuard::new().unwrap();
    let (_temp, cache, menu) = folders();
    let mut state = AppState {
        groups: vec![Group {
            icon_version: 1,
            ..named("ga", "Alpha")
        }],
        ..AppState::default()
    };
    let first = write_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    let groups = cache.join(GROUPS_DIR);
    assert_eq!(names(&groups), ["ga-v1.ico"]);

    state.groups[0].icon_version = 2;
    let lnk = update_group_shortcut(&state, "ga", ICON, &cache, &menu, &exe()).unwrap();
    assert_eq!(lnk, first);
    assert_eq!(names(&groups), ["ga-v2.ico"]);
    assert_eq!(icon_of(&lnk), groups.join("ga-v2.ico").to_str().unwrap());
    assert_eq!(aumid(&lnk).as_deref(), Some("com.grouper.group.ga"));
}

#[test]
fn repair_rebuilds_a_missing_shortcut_or_icon() {
    let _com = ComGuard::new().unwrap();
    let (_temp, cache, menu) = folders();
    let state = AppState {
        groups: vec![named("ga", "Alpha")],
        ..AppState::default()
    };
    let lnk = write_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    let repair = || repair_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    assert_eq!(repair(), Repair::Intact(lnk.clone()));

    fs::remove_file(&lnk).unwrap();
    assert_eq!(repair(), Repair::Rebuilt(lnk.clone()));
    assert_eq!(aumid(&lnk).as_deref(), Some("com.grouper.group.ga"));

    let icon = cache.join(GROUPS_DIR).join("ga-v0.ico");
    fs::remove_file(&icon).unwrap();
    assert_eq!(repair(), Repair::Rebuilt(lnk.clone()));
    assert!(icon.is_file());
    assert_eq!(repair(), Repair::Intact(lnk));
}

#[test]
fn deleting_a_group_removes_only_its_own_files() {
    let _com = ComGuard::new().unwrap();
    let (_temp, cache, menu) = folders();
    let state = AppState {
        groups: vec![named("ga", "Alpha"), named("gb", "Beta")],
        ..AppState::default()
    };
    write_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    write_group_shortcut(&state, "gb", &cache, &menu, &exe()).unwrap();
    let groups = cache.join(GROUPS_DIR);
    fs::write(groups.join("ga-v7.ico"), b"old").unwrap();

    remove_group_files("ga", &cache, Some(&menu));
    assert_eq!(names(&menu), ["Beta.lnk"]);
    assert_eq!(names(&groups), ["gb-v0.ico"]);

    // Without a Start Menu folder only the icons go; a missing folder is
    // not an error.
    remove_group_files("gb", &cache, None);
    remove_group_files("gb", &cache, Some(&menu.join("missing")));
    assert_eq!(names(&menu), ["Beta.lnk"]);
    assert!(names(&groups).is_empty());
}
