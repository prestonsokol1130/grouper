//! S6-03 scenario tests: things users do to Grouper's files and apps while
//! it runs, and sizes far past the usual.

use std::time::{Duration, Instant};

use crate::flyout::layout::{fit_height, flyout_size, LAYOUT};
use crate::launchpad::layout::{default_layout, PER_PAGE};
use crate::scanner::dedupe::{dedupe, Candidate};
use crate::scanner::merge::merge_scan;
use crate::scanner::ScanOutcome;
use crate::state::folder_name::suggest_folder_name;
use crate::state::{AppEntry, AppState, Group, LaunchpadItem, Store};
use crate::taskbar::icon_inputs::group_icon_inputs;
use crate::taskbar::placement::{place, MonitorInfo, Rect};
use crate::taskbar::shortcut::{sanitise_name, MAX_NAME_CHARS};

fn candidate(i: usize, name: &str) -> Candidate {
    Candidate {
        name: name.to_owned(),
        target: format!(r"C:\Program Files\App {i}\app{i}.exe"),
        args: String::new(),
        working_dir: String::new(),
        lnk_path: format!(r"C:\Start Menu\Programs\{name}.lnk"),
        per_user: false,
    }
}

fn scan(apps: Vec<AppEntry>) -> ScanOutcome {
    ScanOutcome {
        apps,
        complete: true,
        unreadable: Vec::new(),
    }
}

#[test]
fn a_renamed_start_menu_shortcut_keeps_its_id_and_takes_the_new_name() {
    let mut state = AppState::default();
    merge_scan(&mut state, scan(dedupe(vec![candidate(1, "Old Name")])));
    let id = state.apps[0].id.clone();
    state.groups.push(Group {
        id: "g".into(),
        app_ids: vec![id.clone()],
        ..Group::default()
    });
    let summary = merge_scan(&mut state, scan(dedupe(vec![candidate(1, "New Name")])));
    assert_eq!((summary.added, summary.updated, summary.missing), (0, 1, 0));
    assert_eq!(state.apps.len(), 1);
    assert_eq!(state.apps[0].id, id);
    assert_eq!(state.apps[0].name, "New Name");
    assert!(state.apps[0]
        .lnk_path
        .as_deref()
        .unwrap()
        .ends_with("New Name.lnk"));
    assert!(!state.apps[0].missing);
    assert_eq!(state.groups[0].app_ids, vec![id]);
}

#[test]
fn two_thousand_start_menu_entries_scan_merge_and_lay_out_quickly() {
    let started = Instant::now();
    // 2,000 shortcuts, of which 100 point at an app already seen.
    let candidates: Vec<Candidate> = (0..2_000)
        .map(|i| candidate(i % 1_900, &format!("App {i:04}")))
        .collect();
    let apps = dedupe(candidates);
    assert_eq!(apps.len(), 1_900);
    let mut state = AppState::default();
    assert_eq!(merge_scan(&mut state, scan(apps.clone())).added, 1_900);
    assert_eq!(merge_scan(&mut state, scan(apps)).added, 0);
    state.launchpad = default_layout(&state.apps, PER_PAGE);
    assert_eq!(state.launchpad.pages.len(), 1_900_usize.div_ceil(PER_PAGE));
    state.apps.push(AppEntry {
        id: "0000000000000new".into(),
        name: "Brand New".into(),
        ..AppEntry::default()
    });
    assert_eq!(state.validate(), 0);
    assert_eq!(state.place_missing_on_launchpad(), 1);
    let last = state.launchpad.pages.last().unwrap();
    assert_eq!(
        last.last(),
        Some(&LaunchpadItem::App {
            app_id: "0000000000000new".into()
        })
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_group_of_fifty_apps_fits_the_screen_and_its_icon_shows_four() {
    let apps: Vec<AppEntry> = (0..50)
        .map(|i| AppEntry {
            id: format!("{i:016x}"),
            name: format!("App {i}"),
            ..AppEntry::default()
        })
        .collect();
    let mut state = AppState {
        groups: vec![Group {
            id: "fifty".into(),
            ..Group::default()
        }],
        apps,
        ..AppState::default()
    };
    let ids: Vec<String> = state.apps.iter().map(|app| app.id.clone()).collect();
    let group = state.set_group_apps("fifty", ids).unwrap();
    assert_eq!(group.app_ids.len(), 50);
    assert_eq!(group_icon_inputs(&state, "fifty").len(), 4);
    // 50 apps in 3 columns would be 17 rows; the flyout is capped to a
    // 1040 px work area at 100% and scrolls inside.
    let size = fit_height(flyout_size(50, &LAYOUT), 1_040, 1.0, &LAYOUT);
    assert!(size.height <= 1_040 - 2 * LAYOUT.margin, "{size:?}");
    assert!(flyout_size(50, &LAYOUT).height > size.height);
}

#[test]
fn very_long_names_make_short_file_names_and_folder_names() {
    let long = "A".repeat(2_000);
    let file = sanitise_name(&long, "abc");
    assert_eq!(file.chars().count(), MAX_NAME_CHARS);
    let emoji = "\u{1F600}".repeat(500);
    assert!(sanitise_name(&emoji, "abc").chars().count() <= MAX_NAME_CHARS);
    let shared = format!("{long} Tools");
    let name = suggest_folder_name(&[shared.as_str(), "Other Tools"]);
    assert_eq!(name, "Tools");
    let apps = [AppEntry {
        id: "a".into(),
        name: long,
        ..AppEntry::default()
    }];
    assert_eq!(default_layout(&apps, PER_PAGE).pages.len(), 1);
}

#[test]
fn a_deleted_state_file_or_folder_is_recreated_by_the_next_save() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("com.grouper.app");
    let store = Store::load(&dir).unwrap();
    let rename = |name: &'static str| {
        store
            .update(move |state| {
                state.settings.grid_columns = name.len().try_into().unwrap();
                Ok(())
            })
            .unwrap();
    };
    rename("seven..");
    store.save().unwrap();
    std::fs::remove_file(store.path()).unwrap();
    rename("eight...");
    store.save().unwrap();
    let saved = std::fs::read_to_string(store.path()).unwrap();
    assert!(saved.contains("\"gridColumns\": 8"), "{saved}");
    std::fs::remove_dir_all(&dir).unwrap();
    rename("nine.....");
    store.save().unwrap();
    assert!(store.path().is_file());
    assert_eq!(
        Store::load(&dir).unwrap().snapshot().settings.grid_columns,
        9
    );
}

#[test]
fn a_flyout_placed_from_a_vanished_monitor_lands_on_the_one_that_is_left() {
    // After unplugging, the cursor can still report a spot on a monitor
    // that is gone; MonitorFromPoint then gives the nearest one left.
    let monitor = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };
    let primary = MonitorInfo {
        monitor,
        work: Rect {
            bottom: 1032,
            ..monitor
        },
        scale: 1.0,
        taskbar: None,
    };
    for cursor in [
        (5_000, 500),
        (-3_000, -900),
        (i32::MAX, i32::MIN),
        (1_000, 4_000),
    ] {
        let (x, y) = place(cursor, (232.0, 300.0), &primary, 12.0);
        assert!((12..=1920 - 12 - 232).contains(&x), "{cursor:?} -> x {x}");
        assert!((12..=1032 - 12 - 300).contains(&y), "{cursor:?} -> y {y}");
    }
}
