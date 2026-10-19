#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod cli;
pub mod commands;
pub mod icons;
pub mod launchpad;
pub mod logging;
pub mod router;
pub mod scanner;
pub mod state;
pub mod taskbar;
pub mod tray;
pub mod util;
pub mod win32;
pub mod windows;

use tauri::{Manager, RunEvent};

/// Starts Grouper. Startup failures are logged and the process exits.
pub fn run() {
    logging::install_panic_hook();
    if let Err(error) = start() {
        // stderr too: a failure before the log plugin starts has no logger.
        eprintln!("grouper: startup failed: {error}");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        log::error!("grouper: startup failed: {error}");
        std::process::exit(1);
    }
}

/// Builds and runs the Tauri app, returning an error string if startup fails.
fn start() -> Result<(), String> {
    let app = tauri::Builder::default()
        // Single instance must be registered first so a second launch exits
        // before any other plugin does work.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            router::route(app, cli::parse_args(&argv));
        }))
        .plugin(logging::plugin())
        .on_window_event(windows::handle_window_event)
        .setup(|app| {
            let handle = app.handle();
            log::info!("Grouper {} starting", env!("CARGO_PKG_VERSION"));
            match handle.path().app_log_dir() {
                Ok(dir) => log::info!("logging to {}", dir.display()),
                Err(error) => log::warn!("could not resolve log dir: {error}"),
            }
            tray::create(handle)?;
            // args_os: std::env::args panics on non-Unicode arguments.
            let args: Vec<String> = std::env::args_os()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            router::route(handle, cli::parse_args(&args));
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|error| error.to_string())?;

    app.run(|_app, event| {
        // Closing the last window must not end the daemon; only Quit
        // (app.exit, which carries an exit code) is allowed to.
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit();
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Checks that the compiled package name is `grouper`.
    #[test]
    fn package_name_is_grouper() {
        assert_eq!(env!("CARGO_PKG_NAME"), "grouper");
    }
}
