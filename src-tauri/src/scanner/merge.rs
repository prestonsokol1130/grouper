//! Folds a fresh scan into the saved app list. Pure.

use std::collections::{HashMap, HashSet};

use crate::state::{AppEntry, AppSource, AppState};

/// What [`merge_scan`] changed, for the log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeSummary {
    pub added: usize,
    pub updated: usize,
    pub missing: usize,
}

/// Merges `scanned` Start Menu apps into `state.apps`:
///
/// - an app with a new id is added;
/// - a Start Menu app found again gets the scanned name, target, args,
///   working dir and `.lnk` path, and is no longer missing (its `hidden`
///   flag is kept);
/// - a Start Menu app that was not found is marked `missing`, not deleted,
///   because groups and the Launchpad may still use it;
/// - Manual apps are never changed, even when a scan finds the same id.
pub fn merge_scan(state: &mut AppState, scanned: Vec<AppEntry>) -> MergeSummary {
    let mut summary = MergeSummary::default();
    let found: HashSet<String> = scanned.iter().map(|app| app.id.clone()).collect();
    for app in &mut state.apps {
        if app.source == AppSource::StartMenu && !found.contains(&app.id) && !app.missing {
            app.missing = true;
            summary.missing += 1;
        }
    }
    let index: HashMap<String, usize> = state
        .apps
        .iter()
        .enumerate()
        .map(|(at, app)| (app.id.clone(), at))
        .collect();
    for new in scanned {
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

    #[test]
    fn adds_new_apps() {
        let mut state = AppState::default();
        let summary = merge_scan(
            &mut state,
            vec![app("A", r"C:\a.exe", AppSource::StartMenu)],
        );
        assert_eq!(summary.added, 1);
        assert_eq!(state.apps.len(), 1);
        assert_eq!(state.apps[0].name, "A");
        assert_eq!(state.apps[0].source, AppSource::StartMenu);
    }

    #[test]
    fn updates_name_and_target_and_keeps_hidden() {
        let mut state = AppState::default();
        let mut old = app("Old Name", r"C:\App\a.exe", AppSource::StartMenu);
        old.hidden = true;
        old.missing = true;
        state.apps.push(old);
        let new = app("New Name", r"c:\app\A.EXE", AppSource::StartMenu);
        let summary = merge_scan(&mut state, vec![new]);
        assert_eq!(
            summary,
            MergeSummary {
                updated: 1,
                ..MergeSummary::default()
            }
        );
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
        state.apps.push(app("A", r"C:\a.exe", AppSource::StartMenu));
        let summary = merge_scan(
            &mut state,
            vec![app("A", r"C:\a.exe", AppSource::StartMenu)],
        );
        assert_eq!(summary, MergeSummary::default());
    }

    #[test]
    fn vanished_start_menu_apps_become_missing() {
        let mut state = AppState::default();
        state
            .apps
            .push(app("Gone", r"C:\gone.exe", AppSource::StartMenu));
        state
            .apps
            .push(app("Kept", r"C:\kept.exe", AppSource::StartMenu));
        let summary = merge_scan(
            &mut state,
            vec![app("Kept", r"C:\kept.exe", AppSource::StartMenu)],
        );
        assert_eq!(summary.missing, 1);
        assert_eq!(state.apps.len(), 2);
        assert!(state.apps[0].missing);
        assert!(!state.apps[1].missing);
        // Already missing: not counted again.
        let again = merge_scan(
            &mut state,
            vec![app("Kept", r"C:\kept.exe", AppSource::StartMenu)],
        );
        assert_eq!(again.missing, 0);
    }

    #[test]
    fn manual_apps_are_never_touched() {
        let mut state = AppState::default();
        let manual = app("Mine", r"C:\mine.exe", AppSource::Manual);
        state.apps.push(manual.clone());
        let summary = merge_scan(
            &mut state,
            vec![app("Scanned", r"C:\mine.exe", AppSource::StartMenu)],
        );
        assert_eq!(summary, MergeSummary::default());
        assert_eq!(state.apps, vec![manual.clone()]);
        // Not found by a scan either: still not missing.
        merge_scan(&mut state, Vec::new());
        assert_eq!(state.apps, vec![manual]);
    }
}
