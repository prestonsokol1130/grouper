//! Which apps a group's folder icon shows, and when that changes. Pure: no
//! IO.

use super::compose::MAX_FOLDER_ICONS;
use crate::state::{AppState, Group};

/// The app ids whose icons a group's folder shows: the first four of the
/// group's apps that exist and are neither missing nor hidden. Empty for
/// an unknown group.
pub fn group_icon_inputs(state: &AppState, group_id: &str) -> Vec<String> {
    state
        .group_by_id(group_id)
        .map(|group| icon_inputs_of(state, group))
        .unwrap_or_default()
}

/// [`group_icon_inputs`] for `group` itself, which need not be in `state`
/// (only `state`'s apps are looked at).
fn icon_inputs_of(state: &AppState, group: &Group) -> Vec<String> {
    group
        .app_ids
        .iter()
        .filter(|id| {
            state
                .app_by_id(id)
                .is_some_and(|app| !app.missing && !app.hidden)
        })
        .take(MAX_FOLDER_ICONS)
        .cloned()
        .collect()
}

/// Whether going from `old_group` to `new_group` changes the group's
/// folder icon: true when the first four visible apps (see
/// [`group_icon_inputs`], judged by the apps in `state`) differ, including
/// their order. Apps after the fourth visible one, and hidden or missing
/// apps, never count.
pub fn needs_icon_rebuild(old_group: &Group, new_group: &Group, state: &AppState) -> bool {
    icon_inputs_of(state, old_group) != icon_inputs_of(state, new_group)
}

/// The ids of the groups in both `before` and `after` whose folder icon
/// shows different apps in `after` (see [`group_icon_inputs`]): a change
/// to the group's apps, or an app in it being hidden, shown again, found
/// missing, found again, added or removed. In `after`'s group order.
pub fn changed_icon_groups(before: &AppState, after: &AppState) -> Vec<String> {
    after
        .groups
        .iter()
        .filter(|group| {
            before
                .group_by_id(&group.id)
                .is_some_and(|old| icon_inputs_of(before, old) != icon_inputs_of(after, group))
        })
        .map(|group| group.id.clone())
        .collect()
}
