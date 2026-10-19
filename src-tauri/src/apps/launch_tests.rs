//! Tests for launching apps and groups with a fake launcher.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use super::launch::{launch_app_with, launch_group_with, LaunchSummary};
use crate::state::{AppEntry, AppState, Group, Store};

fn app(id: &str, target: &str) -> AppEntry {
    AppEntry {
        id: id.into(),
        name: format!("App {id}"),
        target: target.into(),
        ..AppEntry::default()
    }
}

/// A store holding apps `a` and `b` (whose targets exist), `gone` (whose
/// target does not) and group `g` with all three, in the order a, gone, b.
fn store(dir: &tempfile::TempDir) -> Store {
    let real = dir.path().join("real.exe");
    std::fs::write(&real, b"x").unwrap();
    let real = real.to_str().unwrap();
    let gone = dir.path().join("gone.exe");
    let store = Store::load(&dir.path().join("data")).unwrap();
    store
        .update(|state| {
            *state = AppState {
                apps: vec![
                    app("a", real),
                    app("gone", gone.to_str().unwrap()),
                    app("b", real),
                ],
                groups: vec![Group {
                    id: "g".into(),
                    name: "G".into(),
                    app_ids: vec!["a".into(), "gone".into(), "b".into()],
                    icon_version: 0,
                }],
                ..AppState::default()
            };
            Ok(())
        })
        .unwrap();
    store
}

#[test]
fn launch_group_reports_launched_and_failed_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let started = RefCell::new(Vec::new());
    let launcher = |entry: &AppEntry| {
        started.borrow_mut().push(entry.id.clone());
        if entry.id == "b" {
            Err("Could not start App b: denied".to_owned())
        } else {
            Ok(())
        }
    };
    let summary = launch_group_with(&store, "g", &launcher, &|_| {}, Duration::ZERO).unwrap();
    let expected = LaunchSummary {
        launched: 1,
        failed: vec!["App gone".into(), "App b".into()],
    };
    assert_eq!(summary, expected);
    // The missing app never reaches the launcher; the rest go in order.
    assert_eq!(*started.borrow(), vec!["a".to_owned(), "b".to_owned()]);
}

#[test]
fn launch_group_of_an_unknown_group_fails() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let result = launch_group_with(&store, "nope", &|_| Ok(()), &|_| {}, Duration::ZERO);
    assert_eq!(result, Err("Group not found: nope".into()));
}

#[test]
fn a_missing_path_marks_the_app_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let published = Cell::new(0);
    let publish = |state: &AppState| {
        assert!(state.app_by_id("gone").is_some_and(|app| app.missing));
        published.set(published.get() + 1);
    };
    let never = |_: &AppEntry| -> Result<(), String> { panic!("must not launch") };
    let result = launch_app_with(&store, "gone", &never, &publish);
    assert_eq!(result, Err("App not found: App gone".into()));
    assert_eq!(published.get(), 1);
    let state = store.snapshot();
    assert!(state.app_by_id("gone").is_some_and(|app| app.missing));
    assert!(state.app_by_id("a").is_some_and(|app| !app.missing));
    // Already missing: still an error, but nothing new to publish.
    assert!(launch_app_with(&store, "gone", &never, &publish).is_err());
    assert_eq!(published.get(), 1);
}

#[test]
fn an_existing_app_is_handed_to_the_launcher() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let called = Cell::new(false);
    let launcher = |entry: &AppEntry| {
        assert_eq!(entry.id, "a");
        called.set(true);
        Ok(())
    };
    assert_eq!(launch_app_with(&store, "a", &launcher, &|_| {}), Ok(()));
    assert!(called.get());
    let unknown = launch_app_with(&store, "zzz", &launcher, &|_| {});
    assert_eq!(unknown, Err("Unknown app: zzz".into()));
}
