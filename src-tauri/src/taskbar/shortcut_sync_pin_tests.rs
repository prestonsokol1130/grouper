//! Windows-only tests for deleting a group ([`super::group_removal`]):
//! real shortcuts and `.ico` files in temp folders, with a recording
//! unpin (and once the Shell's own, on a temp shortcut nobody pinned).

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use super::group_removal::{remove_group_files, Pins};
use super::shortcut_sync::{update_group_shortcut, Change};
use super::shortcut_write::write_group_shortcut;
use crate::state::paths::GROUPS_DIR;
use crate::state::{AppState, Group};
use crate::win32::com::ComGuard;
use crate::win32::lnk_read::resolve_lnk;
use crate::win32::unpin::unpin;

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

fn group(id: &str, name: &str) -> Group {
    Group {
        id: id.to_owned(),
        name: name.to_owned(),
        has_shortcut: true,
        ..Group::default()
    }
}

#[test]
fn delete_unpins_first_then_removes_the_shortcut_and_every_icon_version() {
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
        groups: vec![group("ga", "Alpha"), group("gb", "Beta")],
        ..AppState::default()
    };
    let lnk = write_group_shortcut(&state, "ga", &cache, &menu, &exe()).unwrap();
    write_group_shortcut(&state, "gb", &cache, &menu, &exe()).unwrap();
    // A taskbar pin of the first version, then two icon changes.
    let pin = pinned.join("Alpha.lnk");
    fs::copy(&lnk, &pin).unwrap();
    let pin_bytes = fs::read(&pin).unwrap();
    let icon = Change {
        renamed: false,
        icon: true,
    };
    for version in 1..=2 {
        state.groups[0].icon_version = version;
        update_group_shortcut(&state, "ga", icon, &cache, &menu, &exe()).unwrap();
    }
    let groups = cache.join(GROUPS_DIR);
    let all = ["ga-v1.ico", "ga-v2.ico", "ga-v3.ico", "gb-v1.ico"];
    assert_eq!(names(&groups), all);
    assert_eq!(icon_of(&pin), groups.join("ga-v1.ico").to_str().unwrap());

    // Each unpin sees the shortcut and every icon still in place.
    let calls = RefCell::new(Vec::new());
    let record = |path: &Path| {
        assert!(lnk.is_file(), "the shortcut went before the unpin");
        assert_eq!(names(&groups), all, "an icon went before the unpin");
        calls.borrow_mut().push(path.to_path_buf());
        Err("not pinned".to_owned())
    };
    let pins = Pins {
        taskbar_dir: Some(&pinned),
        unpin: &record,
    };
    remove_group_files("ga", &cache, Some(&menu), &pins);

    assert_eq!(calls.into_inner(), [lnk, pin.clone()]);
    assert_eq!(names(&menu), ["Beta.lnk"]);
    assert_eq!(names(&groups), ["gb-v1.ico"]);
    // The taskbar's own copy is never edited or deleted.
    assert_eq!(fs::read(&pin).unwrap(), pin_bytes);
}

#[test]
fn missing_folders_still_remove_the_icons() {
    let temp = tempfile::tempdir().unwrap();
    let (cache, menu) = (temp.path().join("cache"), temp.path().join("menu"));
    let groups = cache.join(GROUPS_DIR);
    fs::create_dir_all(&groups).unwrap();
    for name in ["gb.ico", "gb-v2.ico", "gbb-v1.ico"] {
        fs::write(groups.join(name), b"x").unwrap();
    }
    let never = |_: &Path| -> Result<(), String> { panic!("nothing to unpin") };
    let pins = Pins {
        taskbar_dir: Some(&temp.path().join("no pins")),
        unpin: &never,
    };
    remove_group_files("gb", &cache, None, &pins);
    remove_group_files("gb", &cache, Some(&menu.join("missing")), &pins);
    assert_eq!(names(&groups), ["gbb-v1.ico"]);
}

#[test]
fn the_shells_unpin_is_called_on_a_temp_shortcut_and_its_failure_tolerated() {
    let _com = ComGuard::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (cache, menu) = (temp.path().join("cache"), temp.path().join("menu"));
    fs::create_dir_all(&menu).unwrap();
    // A unique id, so no real pin can match it.
    let id = "zzUnpinT3st";
    let state = AppState {
        groups: vec![group(id, "Grouper unpin test")],
        ..AppState::default()
    };
    write_group_shortcut(&state, id, &cache, &menu, &exe()).unwrap();
    let calls = RefCell::new(0);
    let shell = |path: &Path| {
        *calls.borrow_mut() += 1;
        // Not pinned: Ok or Err are both fine; it must not stop the delete.
        let _ = unpin(path);
        Err("ignored".to_owned())
    };
    let pins = Pins {
        taskbar_dir: None,
        unpin: &shell,
    };
    remove_group_files(id, &cache, Some(&menu), &pins);
    assert_eq!(calls.into_inner(), 1);
    assert!(names(&menu).is_empty());
    assert!(names(&cache.join(GROUPS_DIR)).is_empty());
}
