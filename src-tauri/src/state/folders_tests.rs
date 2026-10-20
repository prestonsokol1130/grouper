//! Tests for Launchpad folders made and edited by drag and drop.

use std::sync::{Arc, Mutex};

use super::folders::LaunchpadSlot;
use super::model::{AppEntry, AppState, Group, LaunchpadItem};
use super::store::Store;

fn app(id: &str, name: &str) -> AppEntry {
    AppEntry {
        id: id.into(),
        name: name.into(),
        ..AppEntry::default()
    }
}

fn a(id: &str) -> LaunchpadItem {
    LaunchpadItem::App { app_id: id.into() }
}

fn f(id: &str) -> LaunchpadItem {
    LaunchpadItem::Folder {
        group_id: id.into(),
    }
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|id| (*id).to_string()).collect()
}

fn fixed_id(_: &[&str]) -> Result<String, String> {
    Ok("new".into())
}

/// Word and Excel (Microsoft), Notes and Paint loose on page one, folder
/// "g" (apps x, y) and Calc on page two.
fn state() -> AppState {
    AppState {
        apps: vec![
            app("w", "Microsoft Word"),
            app("e", "Microsoft Excel"),
            app("n", "Notes"),
            app("p", "Paint"),
            app("x", "X Tool"),
            app("y", "Y Tool"),
            app("c", "Calc"),
        ],
        groups: vec![Group {
            id: "g".into(),
            name: "Tools".into(),
            app_ids: ids(&["x", "y"]),
            ..Group::default()
        }],
        launchpad: super::model::LaunchpadLayout {
            pages: vec![vec![a("w"), a("n"), a("e"), a("p")], vec![f("g"), a("c")]],
        },
        ..AppState::default()
    }
}

fn slot(page: u32, index: u32) -> LaunchpadSlot {
    LaunchpadSlot { page, index }
}

#[test]
fn app_onto_app_makes_a_named_folder_in_the_targets_place() {
    let mut s = state();
    let id = s.merge_on_launchpad("e", &a("w"), &fixed_id).unwrap();
    assert_eq!(id, "new");
    let group = s.group_by_id("new").unwrap();
    assert_eq!(group.name, "Microsoft");
    assert_eq!(group.app_ids, ids(&["w", "e"]));
    assert!(!group.has_shortcut);
    assert_eq!(s.launchpad.pages[0], [f("new"), a("n"), a("p")]);
    // Without a shared word the folder is called "Folder".
    let mut s = state();
    s.merge_on_launchpad("p", &a("n"), &fixed_id).unwrap();
    assert_eq!(s.group_by_id("new").unwrap().name, "Folder");
}

#[test]
fn app_onto_folder_appends_it_and_removes_its_tile() {
    let mut s = state();
    let id = s.merge_on_launchpad("c", &f("g"), &fixed_id).unwrap();
    assert_eq!(id, "g");
    assert_eq!(s.group_by_id("g").unwrap().app_ids, ids(&["x", "y", "c"]));
    assert_eq!(s.launchpad.pages[1], [f("g")]);
}

#[test]
fn the_last_tile_of_a_page_takes_the_page_with_it() {
    let mut s = state();
    s.launchpad.pages.push(vec![a("x")]);
    s.merge_on_launchpad("x", &a("n"), &fixed_id).unwrap();
    assert_eq!(s.launchpad.pages.len(), 2);
}

#[test]
fn an_app_dropped_onto_itself_changes_nothing() {
    let mut s = state();
    let before = s.clone();
    assert!(s.merge_on_launchpad("w", &a("w"), &fixed_id).is_err());
    assert_eq!(s, before);
}

#[test]
fn an_app_already_in_the_folder_is_not_repeated() {
    let mut s = state();
    s.launchpad.pages[1].push(a("x"));
    s.merge_on_launchpad("x", &f("g"), &fixed_id).unwrap();
    assert_eq!(s.group_by_id("g").unwrap().app_ids, ids(&["x", "y"]));
    assert_eq!(s.launchpad.pages[1], [f("g"), a("c")]);
}

#[test]
fn unknown_apps_groups_and_targets_are_errors_that_change_nothing() {
    let failing = |_: &[&str]| -> Result<String, String> { Err("no id".into()) };
    let cases: [(&str, LaunchpadItem); 5] = [
        ("zz", a("w")),
        ("w", a("zz")),
        ("w", f("zz")),
        ("w", a("x")), // in a folder, not on the grid
        ("n", a("p")), // the id maker fails
    ];
    for (dragged, target) in cases {
        let mut s = state();
        let before = s.clone();
        assert!(s.merge_on_launchpad(dragged, &target, &failing).is_err());
        assert_eq!(s, before, "{dragged} onto {target:?}");
    }
}

#[test]
fn taking_an_app_out_keeps_a_bigger_folder() {
    let mut s = state();
    s.group_by_id_mut("g").unwrap().app_ids.push("c".into());
    s.launchpad.pages[1].pop();
    let deleted = s.take_out_of_folder("g", "x", slot(0, 1), 35).unwrap();
    assert!(!deleted);
    assert_eq!(s.group_by_id("g").unwrap().app_ids, ids(&["y", "c"]));
    assert_eq!(
        s.launchpad.pages[0],
        [a("w"), a("x"), a("n"), a("e"), a("p")]
    );
}

#[test]
fn a_folder_left_with_one_app_dissolves_into_it() {
    let mut s = state();
    let deleted = s.take_out_of_folder("g", "x", slot(0, 99), 35).unwrap();
    assert!(deleted);
    assert!(s.group_by_id("g").is_none());
    assert_eq!(s.launchpad.pages[0].last(), Some(&a("x")));
    // The remaining app takes the folder's place.
    assert_eq!(s.launchpad.pages[1], [a("y"), a("c")]);
}

#[test]
fn a_folder_left_empty_is_deleted() {
    let mut s = state();
    s.group_by_id_mut("g").unwrap().app_ids = ids(&["x"]);
    assert!(s.take_out_of_folder("g", "x", slot(2, 0), 35).unwrap());
    assert!(s.group_by_id("g").is_none());
    assert_eq!(
        s.launchpad.pages,
        [s.launchpad.pages[0].clone(), vec![a("c")], vec![a("x")]]
    );
}

#[test]
fn a_full_page_spills_onto_the_next() {
    let mut s = state();
    let deleted = s.take_out_of_folder("g", "x", slot(0, 0), 4).unwrap();
    assert!(deleted);
    assert_eq!(s.launchpad.pages[0], [a("x"), a("w"), a("n"), a("e")]);
    assert_eq!(s.launchpad.pages[1], [a("p"), a("y"), a("c")]);
}

#[test]
fn an_app_already_on_the_grid_moves_rather_than_repeats() {
    let mut s = state();
    s.launchpad.pages[0].insert(0, a("x"));
    s.take_out_of_folder("g", "x", slot(0, 3), 35).unwrap();
    assert_eq!(
        s.launchpad.pages[0],
        [a("w"), a("n"), a("x"), a("e"), a("p")]
    );
}

#[test]
fn taking_out_an_app_not_in_the_folder_is_an_error() {
    let mut s = state();
    let before = s.clone();
    assert!(s.take_out_of_folder("g", "w", slot(0, 0), 35).is_err());
    assert!(s.take_out_of_folder("zz", "x", slot(0, 0), 35).is_err());
    assert_eq!(s, before);
}

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
