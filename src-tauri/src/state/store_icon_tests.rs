//! Tests that every committed change bumps a group's icon version exactly
//! when its folder icon changes, and tells the icon listener once.

use std::sync::{Arc, Mutex};

use super::model::{AppEntry, AppSource, AppState, Group};
use super::store::Store;
use crate::apps::add::add_entry;
use crate::apps::remove::remove_or_hide;

fn app(id: &str) -> AppEntry {
    AppEntry {
        id: id.into(),
        name: id.to_uppercase(),
        source: AppSource::StartMenu,
        ..AppEntry::default()
    }
}

/// A store whose group "g" holds apps a..e (a-d are its icon), with a
/// listener that records every call.
fn store() -> (tempfile::TempDir, Store, Arc<Mutex<Vec<Vec<String>>>>) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::load(dir.path()).unwrap();
    store
        .update(|state| {
            *state = AppState {
                apps: ["a", "b", "c", "d", "e"].into_iter().map(app).collect(),
                groups: vec![Group {
                    id: "g".into(),
                    name: "G".into(),
                    app_ids: ["a", "b", "c", "d", "e"].map(String::from).to_vec(),
                    has_shortcut: true,
                    ..Group::default()
                }],
                ..AppState::default()
            };
            Ok(())
        })
        .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&calls);
    store.set_icon_listener(Box::new(move |ids| seen.lock().unwrap().push(ids.to_vec())));
    (dir, store, calls)
}

fn version(store: &Store) -> u32 {
    store.snapshot().groups[0].icon_version
}

fn set_missing(state: &mut AppState, id: &str, missing: bool) -> Result<(), String> {
    let app = state.apps.iter_mut().find(|app| app.id == id);
    app.ok_or("no such app")?.missing = missing;
    Ok(())
}

#[test]
fn hiding_an_app_in_the_icon_bumps_once_and_tells_the_listener() {
    let (_dir, store, calls) = store();
    // Hiding the fifth app changes nothing.
    store
        .update(|state| remove_or_hide(state, "e").map(|_| ()))
        .unwrap();
    assert_eq!(version(&store), 0);
    assert!(calls.lock().unwrap().is_empty());
    store
        .update(|state| remove_or_hide(state, "b").map(|_| ()))
        .unwrap();
    assert_eq!(version(&store), 1);
    assert_eq!(*calls.lock().unwrap(), [vec!["g".to_owned()]]);
}

#[test]
fn missing_and_found_again_bump_only_inside_the_first_four() {
    let (_dir, store, calls) = store();
    store.update(|s| set_missing(s, "e", true)).unwrap();
    assert_eq!(version(&store), 0);
    store.update(|s| set_missing(s, "a", true)).unwrap();
    assert_eq!(version(&store), 1);
    // With "a" missing, "e" moves into the icon when it is found again.
    store.update(|s| set_missing(s, "e", false)).unwrap();
    assert_eq!(version(&store), 2);
    // Adding "a" again by hand clears its missing flag.
    store
        .update(|state| Ok(add_entry(state, app("a"))))
        .unwrap();
    assert_eq!(version(&store), 3);
    assert_eq!(calls.lock().unwrap().len(), 3);
}

#[test]
fn removing_an_app_bumps_once_and_only_when_it_was_shown() {
    let (_dir, store, calls) = store();
    store.update(|state| Ok(state.remove_app("e"))).unwrap();
    assert_eq!(version(&store), 0);
    assert!(calls.lock().unwrap().is_empty());
    store.update(|state| Ok(state.remove_app("a"))).unwrap();
    assert_eq!(version(&store), 1);
    assert_eq!(*calls.lock().unwrap(), [vec!["g".to_owned()]]);
}

#[test]
fn set_group_apps_bumps_once_and_other_changes_never_call() {
    let (_dir, store, calls) = store();
    let ids: Vec<String> = ["b", "a", "c", "d"].map(String::from).to_vec();
    store
        .update(|state| Ok(state.set_group_apps("g", ids)))
        .unwrap();
    assert_eq!(version(&store), 1);
    store
        .update(|state| {
            state.settings.theme = "dark".into();
            state.groups[0].name = "Renamed".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(version(&store), 1);
    assert_eq!(*calls.lock().unwrap(), [vec!["g".to_owned()]]);
}
