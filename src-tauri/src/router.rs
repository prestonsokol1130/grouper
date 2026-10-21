//! Dispatches a parsed [`LaunchCommand`] to the right window.

use tauri::{AppHandle, Runtime};

use crate::cli::LaunchCommand;
use crate::flyout;
use crate::launchpad::{self, Opener};
use crate::windows::show_window;

/// Runs `cmd`. Used for the first launch and for forwarded second launches.
pub fn route<R: Runtime>(app: &AppHandle<R>, cmd: LaunchCommand) {
    log::info!("routing launch command {cmd:?}");
    match cmd {
        // The flyout logs its own outcome, including the time to show.
        LaunchCommand::Flyout { group_id } => flyout::window::request(app, &group_id),
        LaunchCommand::Launchpad => launchpad::window::toggle(app, Opener::Shortcut),
        // show_window logs its own failures.
        LaunchCommand::Manager => {
            let _ = show_window(app, "manager");
        }
        LaunchCommand::BenchFlyout => flyout::bench::start(app),
        // Cleanup runs before the app starts (see lib::run), never here.
        LaunchCommand::Background | LaunchCommand::Cleanup { .. } | LaunchCommand::None => {}
    }
}
