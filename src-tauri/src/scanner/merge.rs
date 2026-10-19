//! Folds a fresh scan into the saved app list. Pure.

use std::collections::{HashMap, HashSet};

use super::ScanOutcome;
use crate::state::{AppEntry, AppSource, AppState};

/// What [`merge_scan`] changed, for the log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeSummary {
    pub added: usize,
    pub updated: usize,
    pub missing: usize,
}

/// Merges the apps of `scan` into `state.apps`:
///
/// - an app with a new id is added;
/// - a Start Menu app found again gets the scanned name, target, args,
///   working dir and `.lnk` path, and is no longer missing (its `hidden`
///   flag is kept);
/// - a Start Menu app that was not found is marked `missing`, not deleted,
///   because groups and the Launchpad may still use it. This only happens
///   when the scan was complete, and never for an app whose `.lnk` still
///   exists but could not be read;
/// - Manual apps are never changed, even when a scan finds the same id.
pub fn merge_scan(state: &mut AppState, scan: ScanOutcome) -> MergeSummary {
    let mut summary = MergeSummary::default();
    if scan.complete {
        summary.missing = mark_missing(state, &scan);
    }
    let index: HashMap<String, usize> = state
        .apps
        .iter()
        .enumerate()
        .map(|(at, app)| (app.id.clone(), at))
        .collect();
    for new in scan.apps {
        match index.get(&new.id).and_then(|&at| state.apps.get_mut(at)) {
            Some(old) if old.source == AppSource::Manual => {}
            Some(old) => {
                if refresh(old, new) {
                    summary.updated += 1;
                }
            }
            None => {
                state.apps.push(AppEntry {
                    source: AppSource::StartMenu,
                    missing: false,
                    ..new
                });
                summary.added += 1;
            }
        }
    }
    summary
}

/// Marks Start Menu apps that `scan` did not see as missing; returns how
/// many newly became missing.
fn mark_missing(state: &mut AppState, scan: &ScanOutcome) -> usize {
    let found: HashSet<&str> = scan.apps.iter().map(|app| app.id.as_str()).collect();
    let unreadable: HashSet<String> = scan.unreadable.iter().map(|p| p.to_lowercase()).collect();
    let mut count = 0;
    for app in &mut state.apps {
        let lnk_unreadable = app
            .lnk_path
            .as_ref()
            .is_some_and(|lnk| unreadable.contains(&lnk.to_lowercase()));
        if app.source == AppSource::StartMenu
            && !app.missing
            && !lnk_unreadable
            && !found.contains(app.id.as_str())
        {
            app.missing = true;
            count += 1;
        }
    }
    count
}

/// Copies the scanned fields of `new` into `old`; true if anything changed.
fn refresh(old: &mut AppEntry, new: AppEntry) -> bool {
    let next = AppEntry {
        id: old.id.clone(),
        source: old.source,
        hidden: old.hidden,
        missing: false,
        ..new
    };
    if *old == next {
        return false;
    }
    *old = next;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ids::app_id;

    fn app(name: &str, target: &str, source: AppSource) -> AppEntry {
        AppEntry {
            id: app_id(target, ""),
            name: name.into(),
            target: target.into(),
            source,
            lnk_path: Some(format!(r"C:\Start\{name}.lnk")),
            ..AppEntry::default()
        }
    }

    fn start(name: &str, target: &str) -> AppEntry {
        app(name, target, AppSource::StartMenu)
    }

    fn full(apps: Vec<AppEntry>) -> ScanOutcome {
        ScanOutcome {
            apps,
            complete: true,
            unreadable: Vec::new(),
        }
    }

    fn partial(apps: Vec<AppEntry>) -> ScanOutcome {
        ScanOutcome {
            complete: false,
            ..full(apps)
        }
    }

    #[test]
    fn adds_new_apps() {
        let mut state = AppState::default();
        let summary = merge_scan(&mut state, full(vec![start("A", r"C:\a.exe")]));
        assert_eq!(summary.added, 1);
        assert_eq!(state.apps.len(), 1);
        assert_eq!(state.apps[0].name, "A");
        assert_eq!(state.apps[0].source, AppSource::StartMenu);
    }

    #[test]
    fn updates_name_and_target_and_keeps_hidden() {
        let mut state = AppState::default();
        let mut old = start("Old Name", r"C:\App\a.exe");
        old.hidden = true;
        old.missing = true;
        state.apps.push(old);
        let summary = merge_scan(&mut state, full(vec![start("New Name", r"c:\app\A.EXE")]));
        let updated_one = MergeSummary {
            updated: 1,
            ..MergeSummary::default()
        };
        assert_eq!(summary, updated_one);
        let merged = &state.apps[0];
        assert_eq!(merged.name, "New Name");
        assert_eq!(merged.target, r"c:\app\A.EXE");
        assert_eq!(merged.lnk_path.as_deref(), Some(r"C:\Start\New Name.lnk"));
        assert!(merged.hidden);
        assert!(!merged.missing);
    }

    #[test]
    fn unchanged_app_is_not_counted() {
        let mut state = AppState::default();
        state.apps.push(start("A", r"C:\a.exe"));
        let summary = merge_scan(&mut state, full(vec![start("A", r"C:\a.exe")]));
        assert_eq!(summary, MergeSummary::default());
    }

    #[test]
    fn complete_scan_marks_vanished_apps_missing() {
        let mut state = AppState::default();
        state.apps.push(start("Gone", r"C:\gone.exe"));
        state.apps.push(start("Kept", r"C:\kept.exe"));
        let summary = merge_scan(&mut state, full(vec![start("Kept", r"C:\kept.exe")]));
        assert_eq!(summary.missing, 1);
        assert_eq!(state.apps.len(), 2);
        assert!(state.apps[0].missing);
        assert!(!state.apps[1].missing);
        // Already missing: not counted again.
        let again = merge_scan(&mut state, full(vec![start("Kept", r"C:\kept.exe")]));
        assert_eq!(again.missing, 0);
    }

    #[test]
    fn partial_scan_adds_and_updates_but_marks_nothing_missing() {
        let mut state = AppState::default();
        state.apps.push(start("Unseen", r"C:\unseen.exe"));
        state.apps.push(start("Old Name", r"C:\kept.exe"));
        let scan = partial(vec![
            start("New Name", r"C:\kept.exe"),
            start("Fresh", r"C:\fresh.exe"),
        ]);
        let summary = merge_scan(&mut state, scan);
        let expected = MergeSummary {
            added: 1,
            updated: 1,
            missing: 0,
        };
        assert_eq!(summary, expected);
        assert_eq!(state.apps.len(), 3);
        assert!(state.apps.iter().all(|app| !app.missing));
        assert_eq!(state.apps[1].name, "New Name");
        assert_eq!(state.apps[2].name, "Fresh");
    }

    #[test]
    fn unreadable_shortcut_keeps_its_app() {
        let mut state = AppState::default();
        state.apps.push(start("Locked", r"C:\locked.exe"));
        state.apps.push(start("Gone", r"C:\gone.exe"));
        let scan = ScanOutcome {
            unreadable: vec![r"c:\start\LOCKED.lnk".into()],
            ..full(Vec::new())
        };
        let summary = merge_scan(&mut state, scan);
        assert_eq!(summary.missing, 1);
        assert!(!state.apps[0].missing);
        assert!(state.apps[1].missing);
    }

    #[test]
    fn manual_apps_are_never_touched() {
        let mut state = AppState::default();
        let manual = app("Mine", r"C:\mine.exe", AppSource::Manual);
        state.apps.push(manual.clone());
        let summary = merge_scan(&mut state, full(vec![start("Scanned", r"C:\mine.exe")]));
        assert_eq!(summary, MergeSummary::default());
        assert_eq!(state.apps, vec![manual.clone()]);
        // Not found by a complete scan either: still not missing.
        merge_scan(&mut state, full(Vec::new()));
        assert_eq!(state.apps, vec![manual]);
    }
}
