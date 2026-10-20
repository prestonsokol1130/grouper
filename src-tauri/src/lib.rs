#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod appearance;
pub mod apps;
pub mod cli;
pub mod commands;
pub mod flyout;
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

use std::sync::Arc;

use tauri::{Manager, RunEvent};

/// Starts Grouper. Startup failures are logged and the process exits.
pub fn run() {
    logging::install_panic_hook();
    // Before the single-instance check: a second launch from a taskbar
    // click passes its foreground right on to the running instance.
    win32::flyout_window::allow_any_foreground();
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
    let commands = commands::specta_builder();
    let app = tauri::Builder::default()
        // Single instance must be registered first so a second launch exits
        // before any other plugin does work.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            router::route(app, cli::parse_args(&argv));
        }))
        .plugin(logging::plugin())
        .invoke_handler(commands.invoke_handler())
        .on_window_event(windows::handle_window_event)
        .setup(|app| {
            let handle = app.handle();
            log::info!("Grouper {} starting", env!("CARGO_PKG_VERSION"));
            match handle.path().app_log_dir() {
                Ok(dir) => log::info!("logging to {}", dir.display()),
                Err(error) => log::warn!("could not resolve log dir: {error}"),
            }
            let store = open_store(handle)?;
            app.manage(Arc::clone(&store));
            // Icons are cached in the background; a failure only leaves
            // them uncached.
            match commands::icon_cmds::start_icon_worker(handle) {
                Ok(worker) => {
                    app.manage(worker);
                    commands::icon_cmds::queue_icons(handle, store.snapshot().apps);
                }
                Err(error) => log::error!("could not start the icon worker: {error}"),
            }
            // Shortcuts are kept in sync in the background; at startup any
            // tracked shortcut whose .lnk or .ico is missing is rebuilt.
            commands::shortcut_worker::start_shortcut_sync(handle, &store);
            // A failed first scan is logged; the app still starts.
            if let Err(error) = commands::scan_cmds::scan_if_empty(handle, &store) {
                log::error!("could not start the first app scan: {error}");
            }
            tray::create(handle)?;
            flyout::window::init(handle);
            appearance::material::init(handle, appearance::material::BackgroundSetting::default());
            appearance::accent::init(handle);
            // args_os: std::env::args panics on non-Unicode arguments.
            let args: Vec<String> = std::env::args_os()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            router::route(handle, cli::parse_args(&args));
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|error| error.to_string())?;

    app.run(|app, event| match event {
        // Closing the last window must not end the daemon; only Quit
        // (app.exit, which carries an exit code) is allowed to.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        RunEvent::Exit => {
            commands::icon_cmds::stop_icon_worker(app);
            if let Some(worker) = app.try_state::<commands::shortcut_worker::ShortcutWorker>() {
                worker.stop();
            }
            if let Some(store) = app.try_state::<Arc<state::Store>>() {
                if let Err(error) = store.shutdown() {
                    log::error!("saving app state on exit failed: {error}");
                }
            }
        }
        _ => {}
    });
    Ok(())
}

/// Creates the app data folders, loads `app_state.json` and starts the
/// debounced saver.
fn open_store(app: &tauri::AppHandle) -> Result<Arc<state::Store>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?;
    state::paths::ensure_dirs(&dir)?;
    // A state file that cannot be loaded safely is left untouched on disk;
    // Grouper still starts, keeping changes in memory only.
    let store = Arc::new(
        state::Store::load(&dir).unwrap_or_else(|error| state::Store::in_memory(&dir, error)),
    );
    log::info!("app state at {}", store.path().display());
    store.start_flusher()?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    /// Checks that the compiled package name is `grouper`.
    #[test]
    fn package_name_is_grouper() {
        assert_eq!(env!("CARGO_PKG_NAME"), "grouper");
    }
}
