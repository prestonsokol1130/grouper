//! Dispatches a parsed [`LaunchCommand`] to the right window.

use tauri::{AppHandle, Emitter, Runtime};

use crate::cli::LaunchCommand;
use crate::windows::show_window;

/// Event emitted when a flyout is requested. Placeholder until S2-06.
pub const FLYOUT_REQUESTED_EVENT: &str = "grouper://flyout-requested";

/// Runs `cmd`. Used for the first launch and for forwarded second launches.
pub fn route<R: Runtime>(app: &AppHandle<R>, cmd: LaunchCommand) {
    log::info!("routing launch command {cmd:?}");
    match cmd {
        LaunchCommand::Flyout { group_id } => {
            if let Err(error) = app.emit(FLYOUT_REQUESTED_EVENT, &group_id) {
                log::warn!("could not emit flyout request for {group_id:?}: {error}");
            }
        }
        // show_window logs its own failures.
        LaunchCommand::Launchpad => {
            let _ = show_window(app, "launchpad");
        }
        LaunchCommand::Manager => {
            let _ = show_window(app, "manager");
        }
        LaunchCommand::None => {}
    }
}
