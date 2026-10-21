//! Tests for [`super::orphans`]: temp folders and a fake AppUserModelID
//! reader (the file's text), so they run on any OS and never touch the
//! real Start Menu or pins.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::group_removal::Pins;
use super::orphans::{find_orphans, is_stray_temp, orphan_group, sweep_orphans, Orphans, Swept};

/// The fake reader: a `.lnk` here holds its AppUserModelID as text; an
/// empty file has none, and `unreadable` cannot be read.
fn read_aumid(path: &Path) -> Result<Option<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    match text.as_str() {
        "unreadable" => Err("not a shortcut".to_owned()),
        "" => Ok(None),
        aumid => Ok(Some(aumid.to_owned())),
    }
}

fn live() -> HashSet<&'static str> {
    ["live1", "live2"].into_iter().collect()
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// Grouper's folder with orphans, live shortcuts, temps and other files.
fn start_menu() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("Grouper");
    fs::create_dir_all(dir.join("Sub.lnk")).unwrap();
    for (name, text) in [
        ("AI (2).lnk", "com.grouper.group.gone1"),
        ("IDE's.LNK", "com.grouper.group.gone2"),
        ("Work.lnk", "com.grouper.group.live1"),
        ("Copied.lnk", "com.other.app"),
        ("Plain.lnk", ""),
        ("Broken.lnk", "unreadable"),
        ("Unsafe.lnk", "com.grouper.group.../x"),
        ("AI (2).e822bed3a97217a3.tmp", "half"),
        ("notes.tmp", "user file"),
        ("readme.txt", "com.grouper.group.gone3"),
    ] {
        fs::write(dir.join(name), text).unwrap();
    }
    (temp, dir)
}

#[test]
fn only_grouper_ids_of_deleted_groups_are_orphans() {
    let live = live();
    assert_eq!(
        orphan_group(Some("com.grouper.group.gone"), &live),
        Some("gone".to_owned())
    );
    assert_eq!(orphan_group(Some("com.grouper.group.live1"), &live), None);
    assert_eq!(orphan_group(Some("com.grouper.group."), &live), None);
    assert_eq!(orphan_group(Some("com.grouper.group.a\\b"), &live), None);
    assert_eq!(orphan_group(Some("com.other.gone"), &live), None);
    assert_eq!(orphan_group(None, &live), None);
}

#[test]
fn stray_temps_are_the_writers_names_only() {
    assert!(is_stray_temp("ai (2).e822bed3a97217a3.tmp"));
    assert!(is_stray_temp(".0123456789abcdef.tmp"));
    for name in [
        "notes.tmp",
        "a.0123456789abcde.tmp",
        "a.0123456789abcdeg.tmp",
        "a.lnk",
    ] {
        assert!(!is_stray_temp(name), "{name}");
    }
}

#[test]
fn finds_orphans_and_temps_and_nothing_else() {
    let (_temp, dir) = start_menu();
    let mut found = find_orphans(&dir, &live(), &read_aumid);
    found.shortcuts.sort();
    assert_eq!(
        found,
        Orphans {
            shortcuts: vec![
                (dir.join("AI (2).lnk"), "gone1".to_owned()),
                (dir.join("IDE's.LNK"), "gone2".to_owned()),
            ],
            temps: vec![dir.join("AI (2).e822bed3a97217a3.tmp")],
            listed: true,
        }
    );
    // A missing folder gives nothing, and counts as looked through; a
    // file in its place gives nothing and does not.
    let missing = find_orphans(&dir.join("missing"), &live(), &read_aumid);
    let listed = Orphans {
        listed: true,
        ..Orphans::default()
    };
    assert_eq!(missing, listed);
    let file = find_orphans(&dir.join("readme.txt"), &live(), &read_aumid);
    assert_eq!(file, Orphans::default());
}

#[test]
fn sweep_unpins_before_deleting_and_only_unpins_pinned_copies() {
    let (temp, dir) = start_menu();
    let pinned = temp.path().join("TaskBar");
    fs::create_dir_all(&pinned).unwrap();
    fs::write(pinned.join("IDE's.lnk"), "com.grouper.group.gone9").unwrap();
    fs::write(pinned.join("Work.lnk"), "com.grouper.group.live2").unwrap();
    fs::write(pinned.join("Edge.lnk"), "").unwrap();

    let calls = RefCell::new(Vec::new());
    let record = |path: &Path| {
        assert!(path.is_file(), "{} went before its unpin", path.display());
        calls.borrow_mut().push(path.to_path_buf());
        Err("not pinned".to_owned())
    };
    let pins = Pins {
        taskbar_dir: Some(&pinned),
        unpin: &record,
    };
    let swept = sweep_orphans(Some(&dir), &live(), &read_aumid, &pins);
    assert_eq!(
        swept,
        Swept {
            shortcuts: 2,
            temps: 1,
            pins: 1,
            listed: true,
            // The fake unpin leaves the pinned copy, so its icon must stay.
            kept: vec!["gone9".to_owned()],
        }
    );
    let mut calls = calls.into_inner();
    calls.sort();
    let mut expected = vec![
        dir.join("AI (2).lnk"),
        dir.join("IDE's.LNK"),
        pinned.join("IDE's.lnk"),
    ];
    expected.sort();
    assert_eq!(calls, expected);
    let left = [
        "Broken.lnk",
        "Copied.lnk",
        "Plain.lnk",
        "Sub.lnk",
        "Unsafe.lnk",
        "Work.lnk",
        "notes.tmp",
        "readme.txt",
    ];
    assert_eq!(names(&dir), left);
    // Pinned copies are never deleted.
    assert_eq!(names(&pinned), ["Edge.lnk", "IDE's.lnk", "Work.lnk"]);
}

/// A Start Menu folder with one orphan and a pin folder with another,
/// swept with an unpin that fails (as for a group never pinned) and, when
/// `unpin_removes`, deletes the pinned copy as the Shell does.
fn sweep_two(unpin_removes: bool) -> Swept {
    let temp = tempfile::tempdir().unwrap();
    let (menu, pinned) = (temp.path().join("Grouper"), temp.path().join("TaskBar"));
    fs::create_dir_all(&menu).unwrap();
    fs::create_dir_all(&pinned).unwrap();
    fs::write(menu.join("Old.lnk"), "com.grouper.group.gone1").unwrap();
    fs::write(pinned.join("Older.lnk"), "com.grouper.group.gone2").unwrap();
    let unpin = |path: &Path| {
        if unpin_removes && path.starts_with(&pinned) {
            fs::remove_file(path).unwrap();
        }
        Err("not pinned".to_owned())
    };
    let pins = Pins {
        taskbar_dir: Some(&pinned),
        unpin: &unpin,
    };
    sweep_orphans(Some(&menu), &live(), &read_aumid, &pins)
}

#[test]
fn only_groups_with_a_file_left_keep_their_icons() {
    // The removed shortcut keeps nothing, though its unpin failed; the
    // pinned copy still on disk keeps its group.
    let left = sweep_two(false);
    assert!(left.listed);
    assert_eq!(left.kept, ["gone2"]);
    // Once the pinned copy is gone too, nothing is kept.
    let gone = sweep_two(true);
    assert!(gone.listed);
    assert!(gone.kept.is_empty(), "{gone:?}");
}

#[test]
fn a_folder_not_looked_through_is_reported() {
    let (_temp, dir) = start_menu();
    let never = |_: &Path| -> Result<(), String> { Ok(()) };
    let no_pins = Pins {
        taskbar_dir: None,
        unpin: &never,
    };
    assert!(!sweep_orphans(Some(&dir), &live(), &read_aumid, &no_pins).listed);
    let file = dir.join("readme.txt");
    let pins = Pins {
        taskbar_dir: Some(&file),
        unpin: &never,
    };
    assert!(!sweep_orphans(Some(&dir), &live(), &read_aumid, &pins).listed);
    assert!(!sweep_orphans(None, &live(), &read_aumid, &pins).listed);
}

#[cfg(windows)]
#[test]
fn a_linked_folder_is_never_swept() {
    let (temp, dir) = start_menu();
    let link = temp.path().join("Linked");
    if std::os::windows::fs::symlink_dir(&dir, &link).is_err() {
        return; // Creating symlinks needs Developer Mode or admin.
    }
    let never = |_: &Path| -> Result<(), String> { panic!("nothing to unpin") };
    let pins = Pins {
        taskbar_dir: None,
        unpin: &never,
    };
    let swept = sweep_orphans(Some(&link), &live(), &read_aumid, &pins);
    assert_eq!(swept, Swept::default());
    assert!(dir.join("AI (2).lnk").is_file());
}
