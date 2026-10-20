//! Keeping the Launchpad complete: an app or group added after the layout
//! was first built (a rescan, "Add other program", a new group) gets a
//! slot at the end. Pure: no IO.

use std::collections::HashSet;

use super::model::{AppState, LaunchpadItem};

impl AppState {
    /// Appends every visible item missing from a built Launchpad layout,
    /// at the end of the last page, starting a new page whenever that page
    /// already shows `settings.per_page()` items. Existing items never move.
    ///
    /// Missing means: a group with at least one visible app and no folder
    /// on any page (added in group order), then a visible app with no tile
    /// that is not in any group shown as a folder (added by name, ignoring
    /// case, then id, like the first layout). Hidden apps neither count
    /// towards a full page nor get added. An empty layout is left empty:
    /// the first Launchpad open builds it sorted. Returns how many items
    /// were added.
    pub fn place_missing_on_launchpad(&mut self) -> usize {
        let pages = &self.launchpad.pages;
        if pages.iter().all(Vec::is_empty) {
            return 0;
        }
        let visible: HashSet<&str> = self
            .apps
            .iter()
            .filter(|app| !app.hidden)
            .map(|app| app.id.as_str())
            .collect();
        let mut tiles = HashSet::new();
        let mut folders = HashSet::new();
        for item in pages.iter().flatten() {
            match item {
                LaunchpadItem::App { app_id } => tiles.insert(app_id.as_str()),
                LaunchpadItem::Folder { group_id } => folders.insert(group_id.as_str()),
            };
        }
        let mut added = Vec::new();
        let mut in_folders: HashSet<&str> = HashSet::new();
        for group in &self.groups {
            let shown = folders.contains(group.id.as_str());
            let has_visible = group.app_ids.iter().any(|id| visible.contains(id.as_str()));
            if !shown && has_visible {
                added.push(LaunchpadItem::Folder {
                    group_id: group.id.clone(),
                });
            }
            if shown || has_visible {
                in_folders.extend(group.app_ids.iter().map(String::as_str));
            }
        }
        let mut apps: Vec<_> = self
            .apps
            .iter()
            .filter(|app| !app.hidden)
            .filter(|app| !tiles.contains(app.id.as_str()) && !in_folders.contains(app.id.as_str()))
            .collect();
        apps.sort_by_cached_key(|app| (app.name.to_lowercase(), app.id.clone()));
        added.extend(apps.into_iter().map(|app| LaunchpadItem::App {
            app_id: app.id.clone(),
        }));
        let count = added.len();
        if count > 0 {
            let per_page = self.settings.per_page().max(1);
            append(
                &mut self.launchpad.pages,
                added,
                per_page,
                |item| match item {
                    LaunchpadItem::App { app_id } => visible.contains(app_id.as_str()),
                    LaunchpadItem::Folder { .. } => true,
                },
            );
        }
        count
    }
}

/// Pushes `items` onto the end of `pages`, opening a new page whenever the
/// last one has `per_page` items that `shows`.
fn append(
    pages: &mut Vec<Vec<LaunchpadItem>>,
    items: Vec<LaunchpadItem>,
    per_page: usize,
    shows: impl Fn(&LaunchpadItem) -> bool,
) {
    for item in items {
        let full = pages
            .last()
            .is_none_or(|page| page.iter().filter(|item| shows(item)).count() >= per_page);
        if full {
            pages.push(Vec::new());
        }
        if let Some(page) = pages.last_mut() {
            page.push(item);
        }
    }
}

#[cfg(test)]
#[path = "layout_fill_tests.rs"]
mod tests;
