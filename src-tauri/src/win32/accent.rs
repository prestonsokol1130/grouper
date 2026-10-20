//! Reads the Windows accent colour through WinRT `UISettings` and reports
//! changes. On other platforms [`watch_accent`] returns
//! `Err("unsupported platform")`.

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// The accent for light backgrounds and for dark backgrounds.
pub type AccentPair = (Rgb, Rgb);

/// Keeps the accent change handler registered while alive.
pub struct AccentWatcher {
    #[cfg(windows)]
    inner: imp::Watcher,
}

/// Reads the accent now and calls `on_change` with a fresh read every time
/// the system colours change (on a system thread). `UISettings` is agile,
/// so this works from any thread with COM or WinRT initialised (Tauri's
/// main thread is).
pub fn watch_accent(
    on_change: impl Fn(Result<AccentPair, String>) + Send + 'static,
) -> Result<(AccentPair, AccentWatcher), String> {
    #[cfg(windows)]
    {
        let (pair, inner) = imp::watch(on_change)?;
        Ok((pair, AccentWatcher { inner }))
    }
    #[cfg(not(windows))]
    {
        let _ = on_change;
        Err("unsupported platform".to_owned())
    }
}

impl std::fmt::Debug for AccentWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccentWatcher")
    }
}

#[cfg(windows)]
impl Drop for AccentWatcher {
    fn drop(&mut self) {
        self.inner.stop();
    }
}

#[cfg(windows)]
mod imp {
    use windows::Foundation::TypedEventHandler;
    use windows::UI::ViewManagement::{UIColorType, UISettings};

    use super::{AccentPair, Rgb};
    use crate::win32::com::ComGuard;

    pub(super) struct Watcher {
        settings: UISettings,
        token: i64,
    }

    impl Watcher {
        pub(super) fn stop(&self) {
            if let Err(error) = self.settings.RemoveColorValuesChanged(self.token) {
                log::debug!("could not remove the accent handler: {error}");
            }
        }
    }

    fn read(settings: &UISettings) -> Result<AccentPair, String> {
        let get = |kind: UIColorType| {
            settings
                .GetColorValue(kind)
                .map(|c| Rgb(c.R, c.G, c.B))
                .map_err(|e| format!("GetColorValue: {e}"))
        };
        Ok((
            get(UIColorType::AccentDark1)?,
            get(UIColorType::AccentLight2)?,
        ))
    }

    pub(super) fn watch(
        on_change: impl Fn(Result<AccentPair, String>) + Send + 'static,
    ) -> Result<(AccentPair, Watcher), String> {
        let settings = UISettings::new().map_err(|e| format!("UISettings: {e}"))?;
        let pair = read(&settings)?;
        let handler = TypedEventHandler::<UISettings, windows::core::IInspectable>::new(
            move |sender, _args| {
                // The handler runs on a system thread, so COM is set up
                // here for the duration of this call.
                let _com = match ComGuard::new() {
                    Ok(guard) => guard,
                    Err(error) => {
                        log::warn!("accent change: no COM on this thread: {error}");
                        return Ok(());
                    }
                };
                let colors = match sender.as_ref() {
                    Some(settings) => read(settings),
                    None => Err("no UISettings sender".to_owned()),
                };
                on_change(colors);
                Ok(())
            },
        );
        let token = settings
            .ColorValuesChanged(&handler)
            .map_err(|e| format!("ColorValuesChanged: {e}"))?;
        Ok((pair, Watcher { settings, token }))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::watch_accent;
    use crate::win32::com::ComGuard;

    #[test]
    fn reads_the_live_accent() {
        let _com = ComGuard::new().unwrap();
        let (pair, watcher) = watch_accent(|_| {}).unwrap();
        // AccentDark1 is darker than AccentLight2 for every Windows accent.
        let sum = |c: super::Rgb| u32::from(c.0) + u32::from(c.1) + u32::from(c.2);
        assert!(sum(pair.0) < sum(pair.1), "{pair:?}");
        drop(watcher);
    }
}
