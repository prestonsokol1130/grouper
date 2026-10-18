#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod commands;
pub mod icons;
pub mod launchpad;
pub mod scanner;
pub mod state;
pub mod taskbar;
pub mod win32;

/// Starts Grouper. Startup failures are written to stderr and the process exits.
pub fn run() {
    if let Err(error) = start() {
        eprintln!("grouper: {error}");
        std::process::exit(1);
    }
}

/// Builds and runs the Tauri app, returning an error string if startup fails.
fn start() -> Result<(), String> {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    /// Checks that the compiled package name is `grouper`.
    #[test]
    fn package_name_is_grouper() {
        assert_eq!(env!("CARGO_PKG_NAME"), "grouper");
    }
}
