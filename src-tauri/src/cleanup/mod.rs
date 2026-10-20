//! `Grouper.exe --cleanup [--delete-data]`, run by the NSIS uninstaller
//! (`src-tauri/windows/hooks.nsh`) before it removes Grouper's files.
//!
//! It unpins and deletes the group shortcuts in Grouper's Start Menu
//! folder, then the folder, removes the "Start with Windows" entry, and with `--delete-data` also
//! deletes the app data folder. Every failure is logged and the process
//! always exits 0, so cleanup can never block an uninstall.

pub mod autostart_entry;
pub mod files;
pub mod log_file;

#[cfg(test)]
mod files_tests;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::win32::com::ComGuard;
use crate::win32::{known_folder, unpin};

/// The bundle identifier in `tauri.conf.json`: the app data folder's name.
pub const APP_IDENTIFIER: &str = "com.grouper.app";

/// Where cleanup works. Tests point these at temp folders.
pub struct Targets {
    /// The Start Menu "Programs" folder that holds `Grouper\`.
    pub programs_dir: Result<PathBuf, String>,
    /// `{app_data_dir}`: app_state.json and the icon caches.
    pub app_data_dir: Result<PathBuf, String>,
}

impl Targets {
    /// The current user's real folders.
    pub fn current_user() -> Self {
        Self {
            programs_dir: known_folder::programs(),
            app_data_dir: known_folder::roaming_app_data().map(|dir| dir.join(APP_IDENTIFIER)),
        }
    }
}

/// Runs every cleanup step, logging each result, and returns the errors.
/// A failed step never stops the next one.
pub fn run(
    targets: &Targets,
    registry: &impl autostart_entry::Registry,
    unpin: &dyn Fn(&Path) -> Result<(), String>,
    delete_data: bool,
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut fail = |step: &str, error: String| {
        log::error!("cleanup: {step}: {error}");
        errors.push(format!("{step}: {error}"));
    };
    match targets
        .programs_dir
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|dir| files::remove_start_menu_folder(dir, unpin))
    {
        Ok(outcome) => log::info!("cleanup: Start Menu folder: {outcome:?}"),
        Err(error) => fail("Start Menu folder", error),
    }
    match autostart_entry::remove(registry) {
        Ok(outcome) => log::info!("cleanup: start with Windows entry: {outcome:?}"),
        Err(error) => fail("start with Windows entry", error),
    }
    if !delete_data {
        log::info!("cleanup: keeping app data (Delete app data was not ticked)");
        return errors;
    }
    match targets
        .app_data_dir
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|dir| files::remove_app_data(dir, APP_IDENTIFIER))
    {
        Ok(outcome) => log::info!("cleanup: app data: {outcome:?}"),
        Err(error) => fail("app data", error),
    }
    errors
}

/// The `--cleanup` entry point: logs to `grouper.log`, cleans up the
/// current user's install and exits 0 whatever happened.
pub fn run_and_exit(delete_data: bool) -> ! {
    if let Ok(local) = known_folder::local_app_data() {
        log_file::install(&local.join(APP_IDENTIFIER).join("logs"));
    }
    log::info!("cleanup: starting (delete data: {delete_data})");
    // Unpinning goes through the Shell, which needs COM on this thread.
    let com = ComGuard::new();
    if let Err(error) = &com {
        log::warn!("cleanup: COM: {error}; pinned groups stay pinned");
    }
    let errors = run(
        &Targets::current_user(),
        &autostart_entry::CurrentUser,
        &unpin::unpin,
        delete_data,
    );
    drop(com);
    log::info!("cleanup: finished with {} error(s)", errors.len());
    log::logger().flush();
    std::process::exit(0)
}
