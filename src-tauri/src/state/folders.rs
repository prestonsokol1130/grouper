//! Launchpad folders made and edited by drag and drop. Pure: no IO.
//!
//! A Launchpad folder is a [`Group`] shown as `LaunchpadItem::Folder`, so
//! these change groups and the layout together, in one state change.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::folder_name::suggest_folder_name;
use super::model::{AppState, Group, LaunchpadItem};

/// A stored Launchpad slot: position `index` on page `page`.
// u32: the bindings refuse 64-bit ints. A page one past the last adds a
// page; an index past the end of a page puts the item at its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct LaunchpadSlot {
    pub page: u32,
    pub index: u32,
}

/// Makes a new group id that is not in the given list.
pub type NewGroupId<'a> = &'a dyn Fn(&[&str]) -> Result<String, String>;

impl AppState {
    /// Where `item` is stored on the Launchpad, as (page, index).
    fn find_item(&self, item: &LaunchpadItem) -> Option<(usize, usize)> {
        self.launchpad
            .pages
            .iter()
            .enumerate()
            .find_map(|(page, items)| {
                items
                    .iter()
                    .position(|other| other == item)
                    .map(|index| (page, index))
            })
    }

    /// Takes every App item for `app_id` off the Launchpad; a page that
    /// this empties is removed.
    fn remove_app_items(&mut self, app_id: &str) {
        let mut emptied = HashSet::new();
        for (index, page) in self.launchpad.pages.iter_mut().enumerate() {
            let before = page.len();
            page.retain(|item| !matches!(item, LaunchpadItem::App { app_id: id } if id == app_id));
            if page.is_empty() && before > 0 {
                emptied.insert(index);
            }
        }
        let mut index = 0;
        self.launchpad.pages.retain(|_| {
            index += 1;
            !emptied.contains(&(index - 1))
        });
    }

    /// Drops app `dragged_id` onto `target` on the Launchpad, in one change.
    ///
    /// Onto another app: a new group named by [`suggest_folder_name`] holds
    /// the target then the dragged app, its folder takes the target's
    /// place and the dragged app's tile goes. Onto a folder: the app is
    /// added to the end of that group (once; an app already in it is not
    /// repeated) and its tile goes. Returns the folder's group id. An
    /// unknown app or group, an app dropped onto itself or a target app
    /// that is not on the Launchpad is an `Err` and changes nothing.
    pub fn merge_on_launchpad(
        &mut self,
        dragged_id: &str,
        target: &LaunchpadItem,
        new_id: NewGroupId<'_>,
    ) -> Result<String, String> {
        let dragged = self
            .app_by_id(dragged_id)
            .ok_or_else(|| format!("App not found: {dragged_id}"))?;
        let dragged_name = dragged.name.clone();
        match target {
            LaunchpadItem::App { app_id } => {
                if app_id == dragged_id {
                    return Err("An app can't be dropped onto itself".into());
                }
                let target_app = self
                    .app_by_id(app_id)
                    .ok_or_else(|| format!("App not found: {app_id}"))?;
                let name = suggest_folder_name(&[target_app.name.as_str(), &dragged_name]);
                let (page, index) = self
                    .find_item(target)
                    .ok_or_else(|| format!("App not on the Launchpad: {app_id}"))?;
                let taken: Vec<&str> = self.groups.iter().map(|g| g.id.as_str()).collect();
                let group = Group {
                    id: new_id(&taken)?,
                    name,
                    app_ids: vec![app_id.clone(), dragged_id.to_owned()],
                    ..Group::default()
                };
                let slot = self
                    .launchpad
                    .pages
                    .get_mut(page)
                    .and_then(|items| items.get_mut(index))
                    .ok_or("Launchpad slot vanished")?;
                *slot = LaunchpadItem::Folder {
                    group_id: group.id.clone(),
                };
                let id = group.id.clone();
                self.groups.push(group);
                self.remove_app_items(dragged_id);
                Ok(id)
            }
            LaunchpadItem::Folder { group_id } => {
                let group = self
                    .group_by_id_mut(group_id)
                    .ok_or_else(|| format!("Group not found: {group_id}"))?;
                if !group.app_ids.iter().any(|id| id == dragged_id) {
                    group.app_ids.push(dragged_id.to_owned());
                }
                self.remove_app_items(dragged_id);
                Ok(group_id.clone())
            }
        }
    }

    /// Takes app `app_id` out of folder `group_id` and puts its tile at
    /// stored slot `to` (any other tile of it goes). A full page pushes
    /// its last tiles onto the next page, so no page shows more than
    /// `per_page`. A folder left with one app is dissolved: its group is
    /// deleted ([`AppState::remove_group`]) and that app's tile takes its
    /// place; a folder left empty is deleted. Returns true when the group
    /// was deleted, so its taskbar shortcut must go too. An unknown group,
    /// or an app not in it, is an `Err`.
    pub fn take_out_of_folder(
        &mut self,
        group_id: &str,
        app_id: &str,
        to: LaunchpadSlot,
        per_page: usize,
    ) -> Result<bool, String> {
        let group = self
            .group_by_id_mut(group_id)
            .ok_or_else(|| format!("Group not found: {group_id}"))?;
        let before = group.app_ids.len();
        group.app_ids.retain(|id| id != app_id);
        if group.app_ids.len() == before {
            return Err(format!("App {app_id} is not in group {group_id}"));
        }
        let left = group.app_ids.len();
        self.place_app(app_id, to, per_page);
        let dissolve = left <= 1;
        if dissolve {
            self.remove_group(group_id);
        }
        Ok(dissolve)
    }

    /// Puts an App item for `app_id` at `to`, removing its other tiles,
    /// then moves overflow onto the following pages.
    fn place_app(&mut self, app_id: &str, to: LaunchpadSlot, per_page: usize) {
        let item = LaunchpadItem::App {
            app_id: app_id.to_owned(),
        };
        let pages = &mut self.launchpad.pages;
        let page = usize::try_from(to.page).map_or(pages.len(), |page| page.min(pages.len()));
        let mut index = usize::try_from(to.index).unwrap_or(usize::MAX);
        // Another tile of the app before `to` on its page shifts it left.
        for (at, items) in pages.iter_mut().enumerate() {
            let mut position = 0;
            items.retain(|other| {
                let keep = other != &item;
                if !keep && at == page && position < index {
                    index -= 1;
                }
                position += 1;
                keep
            });
        }
        if page == pages.len() {
            pages.push(Vec::new());
        }
        if let Some(items) = pages.get_mut(page) {
            items.insert(index.min(items.len()), item);
        }
        self.spill_from(page, per_page);
    }

    /// From page `start` on, moves the last items of any page showing more
    /// than `per_page` onto the start of the next page. Hidden apps do not
    /// count and ride along with the item before them.
    fn spill_from(&mut self, start: usize, per_page: usize) {
        let size = per_page.max(1);
        let shown: HashSet<&str> = self
            .apps
            .iter()
            .filter(|app| !app.hidden)
            .map(|app| app.id.as_str())
            .collect();
        let counts = |item: &LaunchpadItem| match item {
            LaunchpadItem::App { app_id } => shown.contains(app_id.as_str()),
            LaunchpadItem::Folder { .. } => true,
        };
        let pages = &mut self.launchpad.pages;
        let mut page = start;
        while let Some(items) = pages.get_mut(page) {
            let mut spill = Vec::new();
            while items.iter().filter(|item| counts(item)).count() > size {
                let Some(last) = items.pop() else { break };
                spill.insert(0, last);
                while items.last().is_some_and(|tail| !counts(tail)) {
                    if let Some(tail) = items.pop() {
                        spill.insert(0, tail);
                    }
                }
            }
            if spill.is_empty() {
                break;
            }
            if page + 1 == pages.len() {
                pages.push(Vec::new());
            }
            if let Some(next) = pages.get_mut(page + 1) {
                next.splice(0..0, spill);
            }
            page += 1;
        }
    }
}
