//! Tests that folder changes made through the store bump icon versions
//! and tell the icon listener, so taskbar icons are rebuilt.

use std::sync::{Arc, Mutex};

use super::folders_tests::{a, f, fixed_id, slot, state};
use super::store::Store;

/// A store holding [`state`], with group "g" given a shortcut, and a
/// listener that records every icon change it hears.
fn store() -> (tempfile::TempDir, Store, Arc<Mutex<Vec<Vec<String>>>>) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::load(dir.path()).unwrap();
    store
        .update(|s| {
            *s = state();
            s.groups[0].has_shortcut = true;
            Ok(())
        })
        .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&calls);
    store.set_icon_listener(Box::new(move |ids| seen.lock().unwrap().push(ids.to_vec())));
    (dir, store, calls)
}

#[test]
fn merging_into_a_folder_bumps_its_icon_and_tells_the_shortcut_worker() {
    let (_dir, store, calls) = store();
    store
        .update(|s| s.merge_on_launchpad("c", &f("g"), &fixed_id))
        .unwrap();
    assert_eq!(store.snapshot().group_by_id("g").unwrap().icon_version, 1);
    assert_eq!(*calls.lock().unwrap(), [vec!["g".to_owned()]]);
    // The fourth app joins the icon; a fifth does not.
    for id in ["w", "e"] {
        store
            .update(|s| s.merge_on_launchpad(id, &f("g"), &fixed_id))
            .unwrap();
    }
    assert_eq!(store.snapshot().group_by_id("g").unwrap().icon_version, 2);
    assert_eq!(calls.lock().unwrap().len(), 2);
}

#[test]
fn taking_an_app_out_of_a_kept_folder_bumps_its_icon() {
    let (_dir, store, calls) = store();
    store
        .update(|s| s.merge_on_launchpad("c", &f("g"), &fixed_id))
        .unwrap();
    store
        .update(|s| s.take_out_of_folder("g", "x", slot(0, 0), 35))
        .unwrap();
    assert_eq!(store.snapshot().group_by_id("g").unwrap().icon_version, 2);
    assert_eq!(calls.lock().unwrap().len(), 2);
}

#[test]
fn a_failed_merge_is_rolled_back_in_the_store() {
    let (_dir, store, calls) = store();
    let before = store.snapshot();
    assert!(store
        .update(|s| s.merge_on_launchpad("w", &a("w"), &fixed_id))
        .is_err());
    assert_eq!(store.snapshot(), before);
    assert!(calls.lock().unwrap().is_empty());
}
