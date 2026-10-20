//! The first Launchpad layout: every visible app, sorted by name, in
//! pages of one grid each.

use tauri::{AppHandle, Emitter, Runtime};

use crate::commands::state_cmds::STATE_CHANGED_EVENT;
use crate::state::{AppEntry, LaunchpadItem, LaunchpadLayout, Store};

/// The default "Launchpad grid" (DESIGN.md section 8): 7 across, 5 down.
/// Settings has no grid field yet, so this is the grid.
pub const GRID_COLUMNS: usize = 7;
pub const GRID_ROWS: usize = 5;

/// Apps per page with the default grid.
pub const PER_PAGE: usize = GRID_COLUMNS * GRID_ROWS;

/// The apps not hidden by the user, sorted by name (ignoring case, then
/// by id so equal names keep a stable order), as App items in pages of
/// `per_page` (at least 1). No apps gives no pages.
pub fn default_layout(apps: &[AppEntry], per_page: usize) -> LaunchpadLayout {
    let mut visible: Vec<&AppEntry> = apps.iter().filter(|app| !app.hidden).collect();
    visible.sort_by_cached_key(|app| (app.name.to_lowercase(), app.id.clone()));
    let pages = visible
        .chunks(per_page.max(1))
        .map(|chunk| {
            chunk
                .iter()
                .map(|app| LaunchpadItem::App {
                    app_id: app.id.clone(),
                })
                .collect()
        })
        .collect();
    LaunchpadLayout { pages }
}

/// True when the layout has no items at all (never built, or emptied).
pub fn is_empty(layout: &LaunchpadLayout) -> bool {
    layout.pages.iter().all(Vec::is_empty)
}

/// Builds and saves the default layout when the current one is empty and
/// there are apps to show (the first scan may still be running). Every
/// window hears the new state. A failure is logged; the Launchpad still
/// opens.
pub fn ensure_default<R: Runtime>(app: &AppHandle<R>, store: &Store) {
    // Checked on a copy first: an update always marks the state for saving.
    let state = store.snapshot();
    if !is_empty(&state.launchpad) || state.apps.iter().all(|app| app.hidden) {
        return;
    }
    let built = store.update_and_publish(
        |state| {
            if !is_empty(&state.launchpad) {
                return Ok(false);
            }
            let layout = default_layout(&state.apps, PER_PAGE);
            if is_empty(&layout) {
                return Ok(false);
            }
            state.launchpad = layout;
            Ok(true)
        },
        |state| {
            if let Err(error) = app.emit(STATE_CHANGED_EVENT, state) {
                log::warn!("could not emit {STATE_CHANGED_EVENT}: {error}");
            }
        },
    );
    match built {
        Ok(true) => log::info!("built the default Launchpad layout"),
        Ok(false) => {}
        Err(error) => log::warn!("could not build the default Launchpad layout: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, name: &str) -> AppEntry {
        AppEntry {
            id: id.into(),
            name: name.into(),
            ..AppEntry::default()
        }
    }

    fn ids(layout: &LaunchpadLayout) -> Vec<Vec<String>> {
        let id = |item: &LaunchpadItem| match item {
            LaunchpadItem::App { app_id } => app_id.clone(),
            LaunchpadItem::Folder { group_id } => format!("folder:{group_id}"),
        };
        layout
            .pages
            .iter()
            .map(|page| page.iter().map(id).collect())
            .collect()
    }

    #[test]
    fn sorts_by_name_ignoring_case_and_skips_hidden_apps() {
        let mut hidden = app("h", "Aardvark");
        hidden.hidden = true;
        let mut missing = app("m", "mail");
        missing.missing = true;
        let apps = [
            app("z", "zoom"),
            app("b", "Bravo"),
            hidden,
            missing,
            app("a", "alpha"),
        ];
        let layout = default_layout(&apps, PER_PAGE);
        assert_eq!(ids(&layout), vec![vec!["a", "b", "m", "z"]]);
    }

    #[test]
    fn equal_names_keep_id_order() {
        let apps = [app("2", "Same"), app("1", "same")];
        assert_eq!(ids(&default_layout(&apps, 35)), vec![vec!["1", "2"]]);
    }

    #[test]
    fn pages_hold_per_page_apps_and_the_last_takes_the_rest() {
        let apps: Vec<AppEntry> = (0..80)
            .map(|i| app(&format!("{i:02}"), &format!("App {i:02}")))
            .collect();
        let layout = default_layout(&apps, PER_PAGE);
        let sizes: Vec<usize> = layout.pages.iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![35, 35, 10]);
        assert_eq!(ids(&layout)[1][0], "35");
        assert_eq!(ids(&layout)[2][9], "79");
    }

    #[test]
    fn exact_multiple_has_no_empty_last_page() {
        let apps: Vec<AppEntry> = (0..70).map(|i| app(&i.to_string(), "x")).collect();
        assert_eq!(default_layout(&apps, 35).pages.len(), 2);
    }

    #[test]
    fn no_apps_is_no_pages_and_zero_per_page_means_one() {
        assert!(default_layout(&[], 35).pages.is_empty());
        let apps = [app("a", "A"), app("b", "B")];
        assert_eq!(ids(&default_layout(&apps, 0)), vec![vec!["a"], vec!["b"]]);
    }

    #[test]
    fn empty_means_no_items_on_any_page() {
        assert!(is_empty(&LaunchpadLayout::default()));
        assert!(is_empty(&LaunchpadLayout {
            pages: vec![vec![], vec![]]
        }));
        let one = default_layout(&[app("a", "A")], 35);
        assert!(!is_empty(&one));
    }
}
