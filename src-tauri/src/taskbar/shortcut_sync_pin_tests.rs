//! Windows-only test, split from `shortcut_sync_tests` for the file size
//! limit: a copy of a group's shortcut outside the Start Menu folder (as an
//! old taskbar pin is) is never touched, and the legacy `.ico` it shows is
//! kept until the group is deleted.

use std::fs;
use std::path::{Path, PathBuf};

use super::shortcut_sync::{
    remove_group_files, repair_group_shortcut, update_group_shortcut, Change, Repair,
};
use super::shortcut_write::write_shortcut_with_icon;
use crate::state::paths::GROUPS_DIR;
use crate::state::{AppState, Group};
use crate::win32::com::ComGuard;
use crate::win32::lnk_aumid::read_lnk_aumid;
use crate::win32::lnk_read::resolve_lnk;

fn exe() -> PathBuf {
    std::env::current_exe().unwrap()
}

fn icon_of(lnk: &Path) -> String {
    resolve_lnk(lnk).unwrap().icon_location.unwrap().0
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
fn an_old_pin_copy_and_its_legacy_icon_are_left_alone_until_delete() {
    let _com = ComGuard::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (cache, menu, pinned) = (
        temp.path().join("cache"),
        temp.path().join("menu"),
        temp.path().join("pinned"),
    );
    fs::create_dir_all(&menu).unwrap();
    fs::create_dir_all(&pinned).unwrap();
    let mut state = AppState {
        groups: vec![Group {
            id: "ga".to_owned(),
            name: "Alpha".to_owned(),
            has_shortcut: true,
            ..Group::default()
        }],
        ..AppState::default()
    };
    // A shortcut from the versioned scheme, and a pinned copy of it.
    let groups = cache.join(GROUPS_DIR);
    fs::create_dir_all(&groups).unwrap();
    let legacy = groups.join("ga-v1.ico");
    fs::write(&legacy, b"old").unwrap();
    let lnk = write_shortcut_with_icon(&state, "ga", &legacy, &menu, &exe()).unwrap();
    let pin = pinned.join("Alpha.lnk");
    fs::copy(&lnk, &pin).unwrap();
    let pin_bytes = fs::read(&pin).unwrap();
    let aumid = read_lnk_aumid(&pin).unwrap();
    assert_eq!(aumid.as_deref(), Some("com.grouper.group.ga"));

    let repaired = repair_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    assert_eq!(repaired, Repair::Rebuilt(lnk.clone()));
    state.groups[0].icon_version = 1;
    let icon = Change {
        renamed: false,
        icon: true,
    };
    update_group_shortcut(&state, "ga", icon, &cache, &menu, &exe()).unwrap();

    let stable = groups.join("ga.ico");
    assert_eq!(icon_of(&lnk), stable.to_str().unwrap());
    assert_eq!(names(&pinned), ["Alpha.lnk"]);
    assert_eq!(icon_of(&pin), legacy.to_str().unwrap());
    assert_eq!(fs::read(&pin).unwrap(), pin_bytes);
    assert_eq!(read_lnk_aumid(&pin).unwrap(), aumid);
    assert!(legacy.is_file());
    assert_eq!(names(&groups), ["ga-v1.ico", "ga.ico"]);

    // Deleting the group removes the legacy file too; the copy stays.
    remove_group_files("ga", &cache, Some(&menu));
    assert!(names(&menu).is_empty());
    assert!(names(&groups).is_empty());
    assert_eq!(fs::read(&pin).unwrap(), pin_bytes);
}
