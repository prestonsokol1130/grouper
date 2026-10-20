//! Tests for appending missing apps and groups to the Launchpad.

use crate::state::{AppEntry, AppState, Group, LaunchpadItem, LaunchpadLayout};

fn app(id: &str, name: &str) -> AppEntry {
    AppEntry {
        id: id.into(),
        name: name.into(),
        ..AppEntry::default()
    }
}

fn tile(id: &str) -> LaunchpadItem {
    LaunchpadItem::App { app_id: id.into() }
}

fn folder(id: &str) -> LaunchpadItem {
    LaunchpadItem::Folder {
        group_id: id.into(),
    }
}

fn group(id: &str, apps: &[&str]) -> Group {
    Group {
        id: id.into(),
        name: id.to_uppercase(),
        app_ids: apps.iter().map(|&a| a.to_owned()).collect(),
        ..Group::default()
    }
}

/// A 4 x 3 grid: 12 items per page.
fn state(apps: Vec<AppEntry>, pages: Vec<Vec<LaunchpadItem>>) -> AppState {
    let mut state = AppState {
        apps,
        launchpad: LaunchpadLayout { pages },
        ..AppState::default()
    };
    state.settings.grid_columns = 4;
    state.settings.grid_rows = 3;
    state
}

fn numbered(count: usize) -> Vec<AppEntry> {
    (0..count)
        .map(|i| app(&format!("a{i:02}"), &format!("App {i:02}")))
        .collect()
}

#[test]
fn a_new_app_goes_at_the_end_of_the_last_page() {
    let mut apps = numbered(3);
    apps.push(app("new", "Zed"));
    let mut state = state(
        apps,
        vec![vec![tile("a00"), tile("a01")], vec![tile("a02")]],
    );
    assert_eq!(state.place_missing_on_launchpad(), 1);
    assert_eq!(
        state.launchpad.pages,
        vec![
            vec![tile("a00"), tile("a01")],
            vec![tile("a02"), tile("new")]
        ]
    );
    assert_eq!(state.place_missing_on_launchpad(), 0, "idempotent");
}

#[test]
fn a_full_last_page_starts_a_new_one_and_nothing_moves() {
    let mut apps = numbered(12);
    apps.push(app("x", "b"));
    apps.push(app("y", "A"));
    let page: Vec<_> = (0..12).map(|i| tile(&format!("a{i:02}"))).collect();
    let mut state = state(apps, vec![page.clone()]);
    assert_eq!(state.place_missing_on_launchpad(), 2);
    assert_eq!(
        state.launchpad.pages,
        vec![page, vec![tile("y"), tile("x")]]
    );
}

#[test]
fn many_new_apps_fill_pages_of_per_page() {
    let mut state = state(numbered(30), vec![vec![tile("a00")]]);
    assert_eq!(state.place_missing_on_launchpad(), 29);
    let sizes: Vec<usize> = state.launchpad.pages.iter().map(Vec::len).collect();
    assert_eq!(sizes, vec![12, 12, 6]);
    assert_eq!(state.launchpad.pages[0][0], tile("a00"));
}

#[test]
fn hidden_apps_are_not_added_and_do_not_fill_a_page() {
    let mut apps = numbered(13);
    for app in apps.iter_mut().take(11) {
        app.hidden = true;
    }
    apps[12].hidden = false;
    let page: Vec<_> = (0..12).map(|i| tile(&format!("a{i:02}"))).collect();
    let mut state = state(apps, vec![page]);
    assert_eq!(state.place_missing_on_launchpad(), 1);
    assert_eq!(
        state.launchpad.pages.len(),
        1,
        "only one item on the page shows"
    );
    assert_eq!(state.launchpad.pages[0].last(), Some(&tile("a12")));
}

#[test]
fn a_new_group_is_added_as_a_folder_and_covers_its_apps() {
    let mut state = state(numbered(4), vec![vec![tile("a00")]]);
    state.groups.push(group("g", &["a01", "a02"]));
    state.groups.push(group("empty", &[]));
    assert_eq!(state.place_missing_on_launchpad(), 2);
    assert_eq!(
        state.launchpad.pages,
        vec![vec![tile("a00"), folder("g"), tile("a03")]]
    );
}

#[test]
fn apps_inside_a_shown_folder_are_not_added_again() {
    let mut state = state(numbered(3), vec![vec![tile("a00"), folder("g")]]);
    state.groups.push(group("g", &["a01", "a02"]));
    assert_eq!(state.place_missing_on_launchpad(), 0);
}

#[test]
fn an_empty_layout_is_left_for_the_first_open() {
    let mut state = state(numbered(3), vec![]);
    assert_eq!(state.place_missing_on_launchpad(), 0);
    assert!(state.launchpad.pages.is_empty());
    let mut state = state_with_empty_pages();
    assert_eq!(state.place_missing_on_launchpad(), 0);
}

fn state_with_empty_pages() -> AppState {
    state(numbered(2), vec![vec![], vec![]])
}

#[test]
fn a_group_of_only_hidden_apps_is_not_added() {
    let mut apps = numbered(2);
    apps[1].hidden = true;
    let mut state = state(apps, vec![vec![tile("a00")]]);
    state.groups.push(group("g", &["a01"]));
    assert_eq!(state.place_missing_on_launchpad(), 0);
}
