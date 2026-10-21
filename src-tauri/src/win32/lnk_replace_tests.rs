//! Windows-only tests for rewriting a shortcut that another process
//! holds open without `FILE_SHARE_DELETE`, as Explorer does while it reads
//! a shortcut's icon: the write waits for it, and never leaves a temp file
//! in the shortcut's folder or the staging folder.

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::com::ComGuard;
use super::lnk_read::resolve_lnk;
use super::lnk_write::{write_group_lnk, LnkSpec};

/// `FILE_SHARE_READ` only.
const SHARE_READ: u32 = 1;

fn spec(menu: &Path, stage: &Path, icon: &str) -> LnkSpec {
    LnkSpec {
        lnk_path: menu.join("Held.lnk"),
        exe_path: std::env::current_exe().unwrap(),
        args: "--flyout heldTest01".to_owned(),
        icon_path: stage.join(icon),
        description: "Held".to_owned(),
        aumid: "com.grouper.group.heldTest01".to_owned(),
        stage_dir: stage.to_path_buf(),
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn folders() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let (menu, stage) = (temp.path().join("menu"), temp.path().join("stage"));
    fs::create_dir_all(&menu).unwrap();
    fs::create_dir_all(&stage).unwrap();
    (temp, menu, stage)
}

fn icon_of(lnk: &Path) -> String {
    resolve_lnk(lnk).unwrap().icon_location.unwrap().0
}

#[test]
fn a_rewrite_waits_for_a_reader_to_let_go() {
    let _com = ComGuard::new().unwrap();
    let (_temp, menu, stage) = folders();
    write_group_lnk(&spec(&menu, &stage, "g-v1.ico")).unwrap();
    let lnk = menu.join("Held.lnk");
    let held = OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ)
        .open(&lnk)
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(held);
    });
    let next = spec(&menu, &stage, "g-v2.ico");
    write_group_lnk(&next).unwrap();
    release.join().unwrap();
    assert_eq!(icon_of(&lnk), next.icon_path.to_str().unwrap());
    assert_eq!(names(&menu), ["Held.lnk"]);
    assert!(names(&stage).is_empty());
}

#[test]
fn a_reader_that_never_lets_go_fails_the_write_and_leaves_no_temp() {
    let _com = ComGuard::new().unwrap();
    let (_temp, menu, stage) = folders();
    let first = spec(&menu, &stage, "g-v1.ico");
    write_group_lnk(&first).unwrap();
    let lnk = menu.join("Held.lnk");
    let before = fs::read(&lnk).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ)
        .open(&lnk)
        .unwrap();
    let error = write_group_lnk(&spec(&menu, &stage, "g-v2.ico")).unwrap_err();
    assert!(error.contains("Held.lnk"), "{error}");
    drop(held);
    // Same volume, so no second try in the shortcut's own folder: no
    // temp file ever appears next to the shortcut.
    assert_eq!(names(&menu), ["Held.lnk"]);
    assert!(names(&stage).is_empty());
    assert_eq!(fs::read(&lnk).unwrap(), before);
}
