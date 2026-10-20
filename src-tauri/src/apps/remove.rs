//! Removing apps: Manual apps are deleted, Start Menu apps are hidden.

use crate::state::{AppEntry, AppSource, AppState};

/// What [`remove_or_hide`] did to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// A Manual app: deleted, along with every reference in groups and on
    /// the Launchpad.
    Deleted,
    /// A Start Menu app: hidden instead, because the next scan would only
    /// add it back.
    Hidden,
}

/// Deletes Manual app `id`, or hides Start Menu app `id`.
pub fn remove_or_hide(state: &mut AppState, id: &str) -> Result<Removal, String> {
    let app = state
        .apps
        .iter_mut()
        .find(|app| app.id == id)
        .ok_or_else(|| format!("Unknown app: {id}"))?;
    match app.source {
        AppSource::StartMenu => {
            app.hidden = true;
            Ok(Removal::Hidden)
        }
        AppSource::Manual => {
            state.remove_app(id);
            Ok(Removal::Deleted)
        }
    }
}

/// The apps to show: every app the user has not hidden.
pub fn visible_apps(apps: &[AppEntry]) -> Vec<&AppEntry> {
    apps.iter().filter(|app| !app.hidden).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Group, LaunchpadItem};

    fn app(id: &str, source: AppSource) -> AppEntry {
        AppEntry {
            id: id.into(),
            name: id.to_uppercase(),
            source,
            ..AppEntry::default()
        }
    }

    fn state() -> AppState {
        let item = |id: &str| LaunchpadItem::App { app_id: id.into() };
        let mut state = AppState {
            apps: vec![app("m", AppSource::Manual), app("s", AppSource::StartMenu)],
            groups: vec![Group {
                id: "g".into(),
                name: "G".into(),
                app_ids: vec!["m".into(), "s".into()],
                icon_version: 0,
                ..Group::default()
            }],
            ..AppState::default()
        };
        state.launchpad.pages = vec![vec![item("m"), item("s")]];
        state
    }

    #[test]
    fn remove_app_drops_a_manual_app_from_groups_and_launchpad() {
        let mut state = state();
        assert_eq!(remove_or_hide(&mut state, "m"), Ok(Removal::Deleted));
        assert!(state.app_by_id("m").is_none());
        assert_eq!(state.groups[0].app_ids, vec!["s".to_owned()]);
        assert_eq!(state.groups[0].icon_version, 1);
        let only_s = vec![vec![LaunchpadItem::App { app_id: "s".into() }]];
        assert_eq!(state.launchpad.pages, only_s);
    }

    #[test]
    fn a_start_menu_app_is_hidden_not_deleted() {
        let mut state = state();
        let before = state.clone();
        assert_eq!(remove_or_hide(&mut state, "s"), Ok(Removal::Hidden));
        assert!(state.app_by_id("s").is_some_and(|app| app.hidden));
        assert_eq!(state.groups, before.groups);
        assert_eq!(state.launchpad, before.launchpad);
        let shown: Vec<&str> = visible_apps(&state.apps)
            .iter()
            .map(|app| app.id.as_str())
            .collect();
        assert_eq!(shown, vec!["m"]);
    }

    #[test]
    fn an_unknown_app_is_an_error() {
        let mut state = state();
        assert_eq!(
            remove_or_hide(&mut state, "nope"),
            Err("Unknown app: nope".into())
        );
    }
}
