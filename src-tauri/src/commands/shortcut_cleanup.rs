//! The shortcut worker's clean-up jobs: removing a deleted group's files,
//! and the startup sweep for files earlier runs left behind. Both unpin
//! before they delete, and delete `.ico` files last.

use std::collections::HashSet;
use std::path::Path;

use crate::icons::gc;
use crate::state::AppState;
use crate::taskbar::group_removal::{remove_group_files, taskbar_pins_dir, Pins};
use crate::taskbar::orphans::sweep_orphans;
use crate::taskbar::shortcut::shortcut_dir_path;
use crate::win32::lnk_aumid::read_lnk_aumid;
use crate::win32::unpin::unpin;

/// Runs `work` with the real taskbar pin folder (when it can be found)
/// and the Shell's unpin.
fn with_pins<T>(work: impl FnOnce(&Pins<'_>) -> T) -> T {
    let taskbar = taskbar_pins_dir()
        .map_err(|error| log::warn!("no taskbar pin folder: {error}"))
        .ok();
    work(&Pins {
        taskbar_dir: taskbar.as_deref(),
        unpin: &unpin,
    })
}

/// Grouper's Start Menu folder, or `None` (logged) when it is unknown.
fn start_menu_dir() -> Option<std::path::PathBuf> {
    shortcut_dir_path()
        .map_err(|error| log::warn!("no Start Menu folder to clean: {error}"))
        .ok()
}

/// Unpins and removes deleted group `group_id`'s shortcut, then its `.ico`
/// files. Runs on the worker thread, under its `ComGuard`.
pub(super) fn remove_deleted_group(group_id: &str, cache_dir: &Path) {
    let dir = start_menu_dir();
    with_pins(|pins| remove_group_files(group_id, cache_dir, dir.as_deref(), pins));
}

/// At startup: unpins and deletes shortcuts of groups that no longer
/// exist (their removal never ran), deletes stray temp files, then the
/// deleted groups' `.ico` files. The `.ico` files stay while the Start
/// Menu or taskbar pin folder cannot be found, because a leftover shortcut
/// or pin there may still show one. Skipped when `state` has no groups: that
/// is also what an unreadable `app_state.json` loads as, and then nothing
/// may be taken for deleted. Runs on the worker thread, under its
/// `ComGuard`.
pub(super) fn sweep_at_startup(state: &AppState, app_data_dir: &Path) {
    if state.groups.is_empty() {
        return;
    }
    let live: HashSet<&str> = state.groups.iter().map(|g| g.id.as_str()).collect();
    let dir = start_menu_dir();
    let (swept, looked_everywhere) = with_pins(|pins| {
        let swept = sweep_orphans(dir.as_deref(), &live, &read_lnk_aumid, pins);
        (swept, dir.is_some() && pins.taskbar_dir.is_some())
    });
    if swept != Default::default() {
        log::info!(
            "startup clean-up: {} orphaned shortcut(s), {} temp file(s), {} pin(s)",
            swept.shortcuts,
            swept.temps,
            swept.pins
        );
    }
    if !looked_everywhere {
        log::warn!("keeping deleted groups' .ico files until both folders can be checked");
        return;
    }
    let icos = gc::collect_icos(app_data_dir, state);
    log::info!("group icon clean-up removed {icos} .ico file(s)");
}
