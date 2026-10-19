//! Tests for loading and saving the store on disk.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use super::disk::STATE_FILE;
use super::model::{AppEntry, AppState, Group};
use super::store::{Store, SAVE_DEBOUNCE};

fn sample() -> AppState {
    AppState {
        apps: vec![AppEntry {
            id: "a1".into(),
            name: "Notepad".into(),
            ..AppEntry::default()
        }],
        groups: vec![Group {
            id: "g1".into(),
            name: "Tools".into(),
            app_ids: vec!["a1".into()],
            icon_version: 0,
        }],
        ..AppState::default()
    }
}

fn save_sample(dir: &std::path::Path) {
    let store = Store::load(dir).unwrap();
    store
        .update(|state| {
            *state = sample();
            Ok(())
        })
        .unwrap();
    store.save().unwrap();
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn save_then_load_round_trips_and_leaves_no_tmp() {
    let dir = tempfile::tempdir().unwrap();
    save_sample(dir.path());
    assert_eq!(Store::load(dir.path()).unwrap().snapshot(), sample());
    assert!(!names(dir.path()).iter().any(|n| n.ends_with(".tmp")));
}

#[test]
fn second_save_writes_a_backup() {
    let dir = tempfile::tempdir().unwrap();
    save_sample(dir.path());
    save_sample(dir.path());
    assert_eq!(
        names(dir.path()),
        vec![
            "app_state.json".to_string(),
            "app_state.json.bak".to_string()
        ]
    );
}

#[test]
fn corrupt_main_falls_back_to_backup() {
    let dir = tempfile::tempdir().unwrap();
    save_sample(dir.path());
    save_sample(dir.path());
    fs::write(dir.path().join(STATE_FILE), "{ not json").unwrap();
    let store = Store::load(dir.path()).unwrap();
    assert_eq!(store.snapshot(), sample());
    // Saving must not copy the corrupt main file over the good backup.
    store.save().unwrap();
    let backup = fs::read_to_string(dir.path().join("app_state.json.bak")).unwrap();
    assert!(backup.contains("Notepad"));
}

#[test]
fn both_corrupt_gives_defaults_and_keeps_corrupt_copy() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(STATE_FILE), "garbage").unwrap();
    fs::write(dir.path().join("app_state.json.bak"), "also garbage").unwrap();
    let store = Store::load(dir.path()).unwrap();
    assert_eq!(store.snapshot(), AppState::default());
    let files = names(dir.path());
    assert!(!files.contains(&STATE_FILE.to_string()));
    let corrupt: Vec<_> = files
        .iter()
        .filter(|n| n.starts_with("app_state.corrupt-") && n.ends_with(".json"))
        .collect();
    assert_eq!(corrupt.len(), 1);
}

#[test]
fn missing_dir_is_created_with_defaults() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("nested").join("com.grouper.app");
    let store = Store::load(&dir).unwrap();
    assert!(dir.is_dir());
    assert_eq!(store.snapshot(), AppState::default());
    // A first run writes the defaults out so the file exists.
    store.shutdown().unwrap();
    assert!(dir.join(STATE_FILE).is_file());
}

#[test]
fn clean_load_does_not_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    save_sample(dir.path());
    let store = Store::load(dir.path()).unwrap();
    store.shutdown().unwrap();
    assert!(!dir.path().join("app_state.json.bak").exists());
}

#[test]
fn load_validates_dangling_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = sample();
    state.groups[0].app_ids.push("ghost".into());
    fs::write(
        dir.path().join(STATE_FILE),
        serde_json::to_string(&state).unwrap(),
    )
    .unwrap();
    let mut expected = sample();
    expected.groups[0].icon_version = 1;
    assert_eq!(Store::load(dir.path()).unwrap().snapshot(), expected);
}

#[test]
fn failed_update_is_rolled_back() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::load(dir.path()).unwrap();
    let result: Result<((), AppState), String> = store.update(|state| {
        state.apps.clear();
        Err("no".into())
    });
    assert!(result.is_err());
    assert_eq!(store.snapshot(), AppState::default());
}

#[test]
fn flusher_saves_after_debounce_and_on_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::load(dir.path()).unwrap());
    store.start_flusher().unwrap();
    store
        .update(|s| {
            *s = sample();
            Ok(())
        })
        .unwrap();
    std::thread::sleep(SAVE_DEBOUNCE + Duration::from_millis(500));
    assert!(store.path().exists());
    store
        .update(|s| {
            s.groups.clear();
            Ok(())
        })
        .unwrap();
    store.shutdown().unwrap();
    let saved = Store::load(dir.path()).unwrap().snapshot();
    assert!(saved.groups.is_empty());
}
