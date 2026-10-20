//! "Start with Windows" through `tauri-plugin-autostart`, which writes
//! Grouper to the current user's `Run` registry key. Windows then starts
//! it at sign-in with [`BACKGROUND_FLAG`], so no window opens.

use std::sync::Arc;

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

use crate::cli::BACKGROUND_FLAG;
use crate::state::Store;

/// The plugin, registering Grouper with `--background`.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_autostart::Builder::new()
        .arg(BACKGROUND_FLAG)
        .build()
}

/// What turns start-at-sign-in on and off: the plugin, or a fake in tests.
pub trait Launcher {
    fn enable(&self) -> Result<(), String>;
    fn disable(&self) -> Result<(), String>;
    fn is_enabled(&self) -> Result<bool, String>;
}

/// The plugin's registry-backed [`Launcher`].
pub struct PluginLauncher<'a, R: Runtime>(pub &'a AppHandle<R>);

impl<R: Runtime> Launcher for PluginLauncher<'_, R> {
    fn enable(&self) -> Result<(), String> {
        self.0.autolaunch().enable().map_err(|e| e.to_string())
    }

    fn disable(&self) -> Result<(), String> {
        self.0.autolaunch().disable().map_err(|e| e.to_string())
    }

    fn is_enabled(&self) -> Result<bool, String> {
        self.0.autolaunch().is_enabled().map_err(|e| e.to_string())
    }
}

/// Turns start-at-sign-in on or off and returns whether it is on now, as
/// Windows reports it.
pub fn set(launcher: &impl Launcher, enabled: bool) -> Result<bool, String> {
    if enabled {
        launcher.enable()?;
    } else {
        launcher.disable()?;
    }
    launcher.is_enabled()
}

/// At startup, makes `Settings::autostart` match the registry, so the
/// Settings toggle shows the truth if the entry was removed elsewhere.
/// Reads only; a failure is logged.
pub fn sync_setting(launcher: &impl Launcher, store: &Arc<Store>) {
    let enabled = match launcher.is_enabled() {
        Ok(enabled) => enabled,
        Err(error) => {
            log::warn!("could not read the start-with-Windows entry: {error}");
            return;
        }
    };
    if store.snapshot().settings.autostart == enabled {
        return;
    }
    log::info!("start with Windows is {enabled} in the registry; saving that");
    let saved = store.update(|state| {
        state.settings.autostart = enabled;
        Ok(())
    });
    if let Err(error) = saved {
        log::warn!("could not save the start-with-Windows setting: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;

    use super::{set, sync_setting, Launcher};
    use crate::state::Store;

    /// Pretends to be the Run key.
    #[derive(Default)]
    struct Fake {
        on: Cell<bool>,
        broken: bool,
    }

    impl Launcher for Fake {
        fn enable(&self) -> Result<(), String> {
            if self.broken {
                return Err("access denied".into());
            }
            self.on.set(true);
            Ok(())
        }

        fn disable(&self) -> Result<(), String> {
            self.on.set(false);
            Ok(())
        }

        fn is_enabled(&self) -> Result<bool, String> {
            Ok(self.on.get())
        }
    }

    #[test]
    fn set_turns_it_on_and_off_and_reports_the_result() {
        let fake = Fake::default();
        assert_eq!(set(&fake, true), Ok(true));
        assert!(fake.on.get());
        assert_eq!(set(&fake, false), Ok(false));
        assert!(!fake.on.get());
    }

    #[test]
    fn a_failed_enable_is_an_error() {
        let fake = Fake {
            broken: true,
            ..Fake::default()
        };
        assert_eq!(set(&fake, true), Err("access denied".into()));
    }

    #[test]
    fn startup_follows_the_registry() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::load(dir.path()).unwrap());
        let fake = Fake::default();
        fake.on.set(true);
        sync_setting(&fake, &store);
        assert!(store.snapshot().settings.autostart);
        fake.on.set(false);
        sync_setting(&fake, &store);
        assert!(!store.snapshot().settings.autostart);
    }
}
