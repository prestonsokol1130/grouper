//! Pure helpers on [`AppState`]. None of these touch disk or panic.

use std::collections::HashSet;

use super::model::{AppEntry, AppState, Group, LaunchpadItem};
use crate::taskbar::icon_inputs::{changed_icon_groups, needs_icon_rebuild};

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
    /// Bumps the icon version of each group whose folder icon showed it.
    pub fn remove_app(&mut self, id: &str) -> Option<AppEntry> {
        let index = self.apps.iter().position(|app| app.id == id)?;
        let before = self.clone();
        let removed = self.apps.remove(index);
        for group in &mut self.groups {
            group.app_ids.retain(|app_id| app_id != id);
        }
        self.bump_icon_versions(&before);
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

    /// Sets group `id`'s apps, dropping unknown and repeated ids (first one
    /// wins). Its icon version is bumped only when the folder icon changes
    /// (see [`needs_icon_rebuild`]). Returns the updated group.
    pub fn set_group_apps(&mut self, id: &str, app_ids: Vec<String>) -> Option<Group> {
        let mut seen = HashSet::new();
        let known: Vec<String> = app_ids
            .into_iter()
            .filter(|app_id| self.app_by_id(app_id).is_some() && seen.insert(app_id.clone()))
            .collect();
        let old = self.group_by_id(id)?;
        let mut new = Group {
            app_ids: known,
            ..old.clone()
        };
        if needs_icon_rebuild(old, &new, self) {
            new.icon_version = new.icon_version.wrapping_add(1);
        }
        let group = self.group_by_id_mut(id)?;
        *group = new;
        Some(group.clone())
    }

    /// Bumps the icon version of every group whose folder icon shows other
    /// apps than in `before` (see [`changed_icon_groups`]), unless its
    /// version was already changed since `before`. Returns the ids of all
    /// groups whose icon version now differs from `before`.
    pub fn bump_icon_versions(&mut self, before: &AppState) -> Vec<String> {
        for id in changed_icon_groups(before, self) {
            let old = before.group_by_id(&id).map(|group| group.icon_version);
            if let Some(group) = self.group_by_id_mut(&id) {
                if old == Some(group.icon_version) {
                    group.icon_version = group.icon_version.wrapping_add(1);
                }
            }
        }
        self.groups
            .iter()
            .filter(|group| {
                before
                    .group_by_id(&group.id)
                    .is_some_and(|old| old.icon_version != group.icon_version)
            })
            .map(|group| group.id.clone())
            .collect()
    }

    /// Drops dangling and duplicate ids from groups and the Launchpad, and
    /// bumps the icon version of each group whose folder icon changed (a
    /// repeated id among its first four). Returns how many references were
    /// removed.
    pub fn validate(&mut self) -> usize {
        let app_ids: HashSet<String> = self.apps.iter().map(|app| app.id.clone()).collect();
        let group_ids: HashSet<String> = self.groups.iter().map(|g| g.id.clone()).collect();
        let mut removed = 0;
        let mut shrunk = Vec::new();
        for (index, group) in self.groups.iter_mut().enumerate() {
            let mut seen = HashSet::new();
            let before = group.app_ids.clone();
            group
                .app_ids
                .retain(|id| app_ids.contains(id) && seen.insert(id.clone()));
            if group.app_ids.len() < before.len() {
                removed += before.len() - group.app_ids.len();
                shrunk.push((index, before));
            }
        }
        for (index, old_ids) in shrunk {
            let changed = self.groups.get(index).is_some_and(|group| {
                let old = Group {
                    app_ids: old_ids,
                    ..group.clone()
                };
                needs_icon_rebuild(&old, group, self)
            });
            if let Some(group) = self.groups.get_mut(index).filter(|_| changed) {
                group.icon_version = group.icon_version.wrapping_add(1);
            }
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
