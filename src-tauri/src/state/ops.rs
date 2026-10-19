//! Pure helpers on [`AppState`]. None of these touch disk or panic.

use std::collections::HashSet;

use super::model::{AppEntry, AppState, Group, LaunchpadItem};

impl AppState {
    /// The app with `id`, if any.
    pub fn app_by_id(&self, id: &str) -> Option<&AppEntry> {
        self.apps.iter().find(|app| app.id == id)
    }

    /// The group with `id`, if any.
    pub fn group_by_id(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|group| group.id == id)
    }

    /// Mutable access to the group with `id`, if any.
    pub fn group_by_id_mut(&mut self, id: &str) -> Option<&mut Group> {
        self.groups.iter_mut().find(|group| group.id == id)
    }

    /// Removes an app and every reference to it (groups and Launchpad).
    pub fn remove_app(&mut self, id: &str) -> Option<AppEntry> {
        let index = self.apps.iter().position(|app| app.id == id)?;
        let removed = self.apps.remove(index);
        for group in &mut self.groups {
            group.app_ids.retain(|app_id| app_id != id);
        }
        for page in &mut self.launchpad.pages {
            page.retain(|item| !matches!(item, LaunchpadItem::App { app_id } if app_id == id));
        }
        Some(removed)
    }

    /// Removes a group. On the Launchpad its folder is replaced, in place, by
    /// App items for its apps (skipping apps already shown elsewhere).
    pub fn remove_group(&mut self, id: &str) -> Option<Group> {
        let index = self.groups.iter().position(|group| group.id == id)?;
        let removed = self.groups.remove(index);
        let mut shown: HashSet<String> = self
            .launchpad
            .pages
            .iter()
            .flatten()
            .filter_map(|item| match item {
                LaunchpadItem::App { app_id } => Some(app_id.clone()),
                LaunchpadItem::Folder { .. } => None,
            })
            .collect();
        for page in &mut self.launchpad.pages {
            let mut next = Vec::with_capacity(page.len());
            for item in page.drain(..) {
                match item {
                    LaunchpadItem::Folder { group_id } if group_id == id => {
                        for app_id in &removed.app_ids {
                            if shown.insert(app_id.clone()) {
                                next.push(LaunchpadItem::App {
                                    app_id: app_id.clone(),
                                });
                            }
                        }
                    }
                    other => next.push(other),
                }
            }
            *page = next;
        }
        Some(removed)
    }

    /// Drops dangling and duplicate ids from groups and the Launchpad.
    /// Returns how many references were removed.
    pub fn validate(&mut self) -> usize {
        let app_ids: HashSet<String> = self.apps.iter().map(|app| app.id.clone()).collect();
        let group_ids: HashSet<String> = self.groups.iter().map(|g| g.id.clone()).collect();
        let mut removed = 0;
        for group in &mut self.groups {
            let mut seen = HashSet::new();
            let before = group.app_ids.len();
            group
                .app_ids
                .retain(|id| app_ids.contains(id) && seen.insert(id.clone()));
            removed += before - group.app_ids.len();
        }
        let mut seen_items = HashSet::new();
        for page in &mut self.launchpad.pages {
            let before = page.len();
            page.retain(|item| {
                let known = match item {
                    LaunchpadItem::App { app_id } => app_ids.contains(app_id),
                    LaunchpadItem::Folder { group_id } => group_ids.contains(group_id),
                };
                known && seen_items.insert(item.clone())
            });
            removed += before - page.len();
        }
        removed
    }
}
