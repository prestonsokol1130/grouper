//! Tests for unsafe-file handling: unreadable files, newer schemas, backup
//! writes, load-time rewrites, publish ordering and symlinks.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

use super::disk::{StatePaths, STATE_FILE};
use super::disk_write::write_state;
use super::model::{AppEntry, AppState, Group};
use super::store::Store;

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
            ..Group::default()
        }],
        ..AppState::default()
    }
}

fn no_corrupt_files(dir: &Path) -> bool {
    !fs::read_dir(dir)
        .unwrap()
        .any(|e| e.unwrap().file_name().to_string_lossy().contains("corrupt"))
}

#[test]
fn unreadable_main_errors_and_stays_put() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join(STATE_FILE);
    fs::create_dir(&main).unwrap();
    assert!(Store::load(dir.path()).is_err());
    assert!(main.is_dir());
}

#[test]
fn bad_json_with_unreadable_backup_errors_and_stays_put() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join(STATE_FILE);
    fs::write(&main, "garbage").unwrap();
    fs::create_dir(dir.path().join("app_state.json.bak")).unwrap();
    assert!(Store::load(dir.path()).is_err());
    assert_eq!(fs::read_to_string(&main).unwrap(), "garbage");
    assert!(no_corrupt_files(dir.path()));
}

#[test]
fn bad_json_without_backup_is_quarantined() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(STATE_FILE), "garbage").unwrap();
    let store = Store::load(dir.path()).unwrap();
    assert_eq!(store.snapshot(), AppState::default());
    assert!(!no_corrupt_files(dir.path()));
}

#[test]
fn newer_schema_is_never_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join(STATE_FILE);
    let text = r#"{"schemaVersion":2,"extra":true}"#;
    fs::write(&main, text).unwrap();
    let store = Store::load(dir.path()).unwrap();
    let change = store.update(|s| {
        s.groups.clear();
        Ok(())
    });
    assert!(change.is_err());
    let _ = store.save();
    let _ = store.shutdown();
    assert_eq!(fs::read_to_string(&main).unwrap(), text);
}

#[test]
fn newer_schema_that_does_not_fit_errors_and_stays_put() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join(STATE_FILE);
    let text = r#"{"schemaVersion":2,"launchpad":{"pages":[[{"kind":"widget"}]]}}"#;
    fs::write(&main, text).unwrap();
    assert!(Store::load(dir.path()).is_err());
    assert_eq!(fs::read_to_string(&main).unwrap(), text);
    assert!(no_corrupt_files(dir.path()));
}

#[test]
fn failed_backup_write_keeps_old_backup_and_main() {
    let dir = tempfile::tempdir().unwrap();
    let paths = StatePaths::new(dir.path());
    fs::write(&paths.main, "MAIN").unwrap();
    fs::write(&paths.backup, "OLD").unwrap();
    fs::create_dir(&paths.backup_temp).unwrap();
    assert!(write_state(&paths, &sample(), true).is_err());
    assert_eq!(fs::read_to_string(&paths.backup).unwrap(), "OLD");
    assert_eq!(fs::read_to_string(&paths.main).unwrap(), "MAIN");
    assert!(!paths.temp.exists());
}

#[test]
fn second_save_replaces_backup_with_previous_main() {
    let dir = tempfile::tempdir().unwrap();
    let paths = StatePaths::new(dir.path());
    write_state(&paths, &AppState::default(), true).unwrap();
    let first = fs::read_to_string(&paths.main).unwrap();
    write_state(&paths, &sample(), true).unwrap();
    assert_eq!(fs::read_to_string(&paths.backup).unwrap(), first);
    assert!(!paths.backup_temp.exists());
}

#[test]
fn load_rewrites_migrated_and_cleaned_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = sample();
    state.schema_version = 0;
    state.groups[0].app_ids.push("ghost".into());
    let main = dir.path().join(STATE_FILE);
    fs::write(&main, serde_json::to_string(&state).unwrap()).unwrap();
    Store::load(dir.path()).unwrap().shutdown().unwrap();
    let saved: AppState = serde_json::from_str(&fs::read_to_string(&main).unwrap()).unwrap();
    assert_eq!(saved.schema_version, 1);
    assert_eq!(saved.groups[0].app_ids, vec!["a1".to_string()]);
}

#[test]
fn publishes_arrive_in_commit_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::load(dir.path()).unwrap());
    let barrier = Arc::new(Barrier::new(2));
    let seen = Arc::new(Mutex::new(Vec::<AppState>::new()));
    let threads: Vec<_> = ["x", "y"]
        .into_iter()
        .map(|id| {
            let (store, barrier, seen) = (store.clone(), barrier.clone(), seen.clone());
            std::thread::spawn(move || {
                barrier.wait();
                let add = |s: &mut AppState| {
                    s.apps.push(AppEntry {
                        id: id.into(),
                        ..AppEntry::default()
                    });
                    Ok(())
                };
                let record = |s: &AppState| {
                    if s.apps.len() == 1 {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    seen.lock().unwrap().push(s.clone());
                };
                store.update_and_publish(add, record).unwrap();
            })
        })
        .collect();
    threads.into_iter().for_each(|t| t.join().unwrap());
    let seen = seen.lock().unwrap();
    let counts: Vec<usize> = seen.iter().map(|s| s.apps.len()).collect();
    assert_eq!(counts, vec![1, 2]);
    assert_eq!(seen[1].apps[0].id, seen[0].apps[0].id);
}

#[test]
fn set_group_apps_result_matches_committed_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::load(dir.path()).unwrap();
    store
        .update(|s| {
            *s = sample();
            Ok(())
        })
        .unwrap();
    let ids = vec!["a1".into(), "a1".into(), "ghost".into()];
    let (group, _) = store.update(|s| Ok(s.set_group_apps("g1", ids))).unwrap();
    let committed = store.snapshot().group_by_id("g1").cloned();
    assert_eq!(group, committed);
    assert_eq!(group.map(|g| g.app_ids), Some(vec!["a1".to_string()]));
}

/// Makes a symlink, or returns false where the OS does not allow it
/// (for example Windows without Developer Mode).
fn try_symlink(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(target, link);
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(target, link);
    made.is_ok()
}

#[test]
fn writes_never_follow_symlinks() {
    for link_name in ["app_state.json.tmp", "app_state.json.bak"] {
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("victim.txt");
        fs::write(&target, "SAFE").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let paths = StatePaths::new(dir.path());
        fs::write(&paths.main, "{}").unwrap();
        let link = dir.path().join(link_name);
        if !try_symlink(&target, &link) {
            eprintln!("skipping symlink test: cannot create symlinks here");
            return;
        }
        assert!(write_state(&paths, &sample(), true).is_err());
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "SAFE");
    }
}
