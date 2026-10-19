//! Tests for adding apps by hand.

use std::path::{Path, PathBuf};

use super::add::{add_entry, entry_from_path, exe_name};
use crate::state::ids::app_id;
use crate::state::{AppSource, AppState};
use crate::win32::lnk_read::ResolvedLink;
use crate::win32::version_info::file_description;

fn no_lnk(_: &Path) -> Result<ResolvedLink, String> {
    panic!("read_lnk must not be called")
}

fn temp_file(dir: &tempfile::TempDir, name: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, b"not a real program").unwrap();
    path
}

#[test]
fn version_info_name_falls_back_to_the_file_stem() {
    let dir = tempfile::tempdir().unwrap();
    let exe = temp_file(&dir, "My Tool.exe");
    assert_eq!(exe_name(&exe, file_description), "My Tool");
    assert_eq!(exe_name(&exe, |_| Some("  ".into())), "My Tool");
    assert_eq!(exe_name(&exe, |_| Some(" Tool Pro ".into())), "Tool Pro");
}

#[test]
fn an_exe_becomes_a_manual_entry() {
    let dir = tempfile::tempdir().unwrap();
    let exe = temp_file(&dir, "Thing.EXE");
    let entry = entry_from_path(&exe, no_lnk, |_| None).unwrap();
    let target = exe.to_str().unwrap();
    assert_eq!(entry.id, app_id(target, ""));
    assert_eq!(entry.name, "Thing");
    assert_eq!(entry.target, target);
    assert_eq!(entry.working_dir, dir.path().to_str().unwrap());
    assert_eq!(entry.source, AppSource::Manual);
    assert_eq!(entry.lnk_path, None);
}

#[test]
fn a_shortcut_keeps_its_lnk_path() {
    let dir = tempfile::tempdir().unwrap();
    let lnk = temp_file(&dir, "Editor.lnk");
    let read = |_: &Path| {
        Ok(ResolvedLink {
            target: r"C:\Apps\editor.exe".into(),
            args: "--new".into(),
            working_dir: r"C:\Apps".into(),
            icon_location: None,
        })
    };
    let entry = entry_from_path(&lnk, read, |_| None).unwrap();
    assert_eq!(entry.id, app_id(r"C:\Apps\editor.exe", "--new"));
    assert_eq!(entry.name, "Editor");
    assert_eq!(entry.args, "--new");
    assert_eq!(entry.working_dir, r"C:\Apps");
    assert_eq!(entry.lnk_path.as_deref(), lnk.to_str());
    assert_eq!(entry.source, AppSource::Manual);
}

#[test]
fn bad_paths_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let text = temp_file(&dir, "notes.txt");
    assert!(entry_from_path(&text, no_lnk, |_| None).is_err());
    let absent = dir.path().join("absent.exe");
    assert!(entry_from_path(&absent, no_lnk, |_| None).is_err());
    assert!(entry_from_path(Path::new("relative.exe"), no_lnk, |_| None).is_err());
    let lnk = temp_file(&dir, "Store App.lnk");
    let no_target = |_: &Path| {
        Ok(ResolvedLink {
            target: String::new(),
            args: String::new(),
            working_dir: String::new(),
            icon_location: None,
        })
    };
    assert!(entry_from_path(&lnk, no_target, |_| None).is_err());
}

#[test]
fn duplicate_add_returns_the_existing_entry() {
    let dir = tempfile::tempdir().unwrap();
    let exe = temp_file(&dir, "Dup.exe");
    let mut state = AppState::default();
    let first = add_entry(&mut state, entry_from_path(&exe, no_lnk, |_| None).unwrap());
    state.apps[0].name = "Renamed".into();
    state.apps[0].missing = true;
    let again = entry_from_path(&exe, no_lnk, |_| Some("Other".into())).unwrap();
    let second = add_entry(&mut state, again);
    assert_eq!(state.apps.len(), 1);
    assert_eq!(second.id, first.id);
    assert_eq!(second.name, "Renamed");
    assert!(!second.missing);
    assert!(!state.apps[0].missing);
}
