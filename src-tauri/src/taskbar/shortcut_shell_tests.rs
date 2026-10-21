//! Windows-only tests of what the Shell sees when a group shortcut is
//! created: the Start menu folder only ever holds the finished shortcut,
//! and the Shell shows the group's folder icon for it straight away.
//!
//! Each test uses its own temp folders (a stand-in Start menu folder and
//! icon cache) with unique names, so no icon the Shell cached in an earlier
//! run can be reused.

use std::path::Path;

use image::RgbaImage;

use super::shortcut_sync::{update_group_shortcut, Change};
use super::shortcut_write::write_group_shortcut;
use crate::icons::extract::extract_icon;
use crate::state::{AppState, Group};
use crate::win32::com::ComGuard;
use crate::win32::dir_watch_tests::{names_seen_while, END_MARKER};
use crate::win32::lnk_write::{write_group_lnk, LnkSpec};
use crate::win32::shell_notify::notify_shortcut_changed;

/// A group with an id and name unique to this run.
fn unique_group() -> (AppState, String, String) {
    let mut nonce = [0_u8; 6];
    getrandom::fill(&mut nonce).unwrap();
    let id: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("Probe {id}");
    let state = AppState {
        groups: vec![Group {
            id: id.clone(),
            name: name.clone(),
            has_shortcut: true,
            ..Group::default()
        }],
        ..AppState::default()
    };
    (state, id, name)
}

/// Mean difference per channel byte between two same-size images.
fn mean_diff(a: &RgbaImage, b: &RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions());
    let total: u64 = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    total as f64 / a.as_raw().len() as f64
}

#[test]
fn the_start_menu_folder_only_ever_sees_the_finished_shortcut() {
    let _com = ComGuard::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (cache, menu) = (temp.path().join("cache"), temp.path().join("menu"));
    std::fs::create_dir_all(&menu).unwrap();
    let (state, id, name) = unique_group();
    let exe = std::env::current_exe().unwrap();
    let icon_change = Change {
        renamed: false,
        icon: true,
    };

    let seen = names_seen_while(&menu, || {
        // Create, then rewrite over the existing file (an icon change).
        write_group_shortcut(&state, &id, &cache, &menu, &exe).unwrap();
        update_group_shortcut(&state, &id, icon_change, &cache, &menu, &exe).unwrap();
    });

    // Explorer and the Start menu read every shortcut that appears in the
    // folder. A temp shortcut renamed a moment later gets the blank
    // document icon cached for it, so none may ever appear.
    let wanted = format!("{name}.lnk");
    let others: Vec<&String> = seen
        .iter()
        .filter(|seen| **seen != wanted && *seen != END_MARKER)
        .collect();
    assert!(seen.contains(&wanted), "never saw {wanted}: {seen:?}");
    assert!(others.is_empty(), "other names in the folder: {others:?}");
    // The staged temp files are gone from the icon cache too.
    // The icon change wrote a second version and kept the first.
    let mut left: Vec<String> = std::fs::read_dir(cache.join("groups"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left, [format!("{id}-v1.ico"), format!("{id}-v2.ico")]);
}

/// A shortcut whose icon file cannot be read: the Shell shows the blank
/// document icon for it, as it did for Preston's new group.
fn blank_lnk(dir: &Path, exe: &Path) -> std::path::PathBuf {
    let empty = dir.join("empty.ico");
    std::fs::write(&empty, b"").unwrap();
    let lnk = dir.join("Blank.lnk");
    write_group_lnk(&LnkSpec {
        lnk_path: lnk.clone(),
        exe_path: exe.to_path_buf(),
        args: String::new(),
        icon_path: empty,
        description: "Blank".to_owned(),
        aumid: "com.grouper.test.blank".to_owned(),
        stage_dir: dir.to_path_buf(),
    })
    .unwrap();
    lnk
}

#[test]
fn a_new_shortcut_shows_the_folder_icon_at_once() {
    let _com = ComGuard::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (cache, menu) = (temp.path().join("cache"), temp.path().join("menu"));
    std::fs::create_dir_all(&menu).unwrap();
    let (state, id, _) = unique_group();
    let exe = std::env::current_exe().unwrap();

    let lnk = write_group_shortcut(&state, &id, &cache, &menu, &exe).unwrap();
    let ico = cache.join("groups").join(format!("{id}-v1.ico"));
    notify_shortcut_changed(&lnk, &ico);
    let blank = blank_lnk(temp.path(), &exe);

    for size in [16, 32, 48, 256] {
        // Asked for straight after the write, as Explorer would.
        let shown = extract_icon(&lnk, size).unwrap();
        let blank = extract_icon(&blank, size).unwrap();
        let own = extract_icon(&ico, size).unwrap();
        let from_blank = mean_diff(&shown, &blank);
        let from_own = mean_diff(&shown, &own);
        assert!(from_blank > 20.0, "{size} px looks blank ({from_blank:.1})");
        assert!(from_own < 1.0, "{size} px is not the .ico ({from_own:.1})");
    }
}
