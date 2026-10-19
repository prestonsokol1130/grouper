//! Tests for adding apps by hand.

use std::path::{Path, PathBuf};

use super::add::{add_entry, entry_from_path, exe_name};
use super::remove::visible_apps;
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

fn link(target: &str, args: &str, dir: &str) -> ResolvedLink {
    ResolvedLink {
        target: target.into(),
        args: args.into(),
        working_dir: dir.into(),
        icon_location: None,
    }
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
fn a_shortcut_to_a_program_adds_the_program() {
    let dir = tempfile::tempdir().unwrap();
    let exe = temp_file(&dir, "editor.exe");
    let target = exe.to_str().unwrap();
    let lnk = temp_file(&dir, "My Editor.lnk");
    let read = |_: &Path| Ok(link(target, "--new", r"C:\Work"));
    let entry = entry_from_path(&lnk, read, |_| panic!("the name is the stem")).unwrap();
    assert_eq!(entry.id, app_id(target, "--new"));
    assert_eq!(entry.name, "My Editor");
    assert_eq!(entry.target, target);
    assert_eq!(entry.args, "--new");
    assert_eq!(entry.working_dir, r"C:\Work");
    assert_eq!(entry.lnk_path, None);
    assert_eq!(entry.source, AppSource::Manual);
}

#[test]
fn a_shortcut_without_a_usable_target_keeps_its_lnk_path() {
    let dir = tempfile::tempdir().unwrap();
    let lnk = temp_file(&dir, "Editor.lnk");
    // The target program is gone.
    let gone = r"C:\No\Such\editor.exe";
    let read = |_: &Path| Ok(link(gone, "--new", r"C:\Apps"));
    let entry = entry_from_path(&lnk, read, |_| None).unwrap();
    assert_eq!(entry.id, app_id(gone, "--new"));
    assert_eq!(entry.name, "Editor");
    assert_eq!(entry.target, gone);
    assert_eq!(entry.args, "--new");
    assert_eq!(entry.lnk_path.as_deref(), lnk.to_str());
    // The target exists but is not a program (an MSI advertised shortcut).
    let icon = temp_file(&dir, "app.ico");
    let icon = icon.to_str().unwrap();
    let entry = entry_from_path(&lnk, |_| Ok(link(icon, "", "")), |_| None).unwrap();
    assert_eq!(entry.target, icon);
    assert_eq!(entry.lnk_path.as_deref(), lnk.to_str());
}

#[test]
fn shortcuts_without_a_target_get_distinct_ids_from_their_paths() {
    let dir = tempfile::tempdir().unwrap();
    let first = temp_file(&dir, "Store App.lnk");
    let second = temp_file(&dir, "Other Store App.lnk");
    let a = entry_from_path(&first, |_| Ok(link("", "", "")), |_| None).unwrap();
    let b = entry_from_path(&second, |_| Ok(link("", "", "")), |_| None).unwrap();
    assert_eq!(a.id, app_id(first.to_str().unwrap(), ""));
    assert_ne!(a.id, b.id);
    assert_ne!(a.id, app_id("", ""));
    assert_eq!(a.lnk_path.as_deref(), first.to_str());
    assert_eq!(a.name, "Store App");
}

#[test]
fn bad_paths_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let text = temp_file(&dir, "notes.txt");
    assert!(entry_from_path(&text, no_lnk, |_| None).is_err());
    let absent = dir.path().join("absent.exe");
    assert!(entry_from_path(&absent, no_lnk, |_| None).is_err());
    assert!(entry_from_path(Path::new("relative.exe"), no_lnk, |_| None).is_err());
    let lnk = temp_file(&dir, "Broken.lnk");
    let unreadable = |_: &Path| Err("not a shortcut".to_owned());
    assert!(entry_from_path(&lnk, unreadable, |_| None).is_err());
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

#[test]
fn re_adding_a_hidden_app_shows_it_again() {
    let dir = tempfile::tempdir().unwrap();
    let exe = temp_file(&dir, "Hidden.exe");
    let mut state = AppState::default();
    add_entry(&mut state, entry_from_path(&exe, no_lnk, |_| None).unwrap());
    state.apps[0].hidden = true;
    assert!(visible_apps(&state.apps).is_empty());
    let again = add_entry(&mut state, entry_from_path(&exe, no_lnk, |_| None).unwrap());
    assert!(!again.hidden);
    assert_eq!(visible_apps(&state.apps).len(), 1);
}
