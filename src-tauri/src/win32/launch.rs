//! Starts an app through the Shell, the way Explorer would.

use std::path::Path;

use crate::state::AppEntry;

/// Name of the short-lived thread each launch runs on.
const LAUNCH_THREAD: &str = "app-launch";

/// What [`launch`] hands to the Shell for one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchTarget {
    /// Open the `.lnk` itself, so Windows applies the shortcut's own
    /// arguments, working folder and AppUserModelID.
    Shortcut(String),
    /// Run the target directly with the entry's arguments and working folder.
    Program {
        file: String,
        args: String,
        working_dir: String,
    },
}

/// Picks how to start `entry`: its `.lnk` when one is set and `exists`,
/// else its target when that `exists`. `None` when neither is there.
pub fn launch_target(entry: &AppEntry, exists: impl Fn(&Path) -> bool) -> Option<LaunchTarget> {
    let lnk = entry.lnk_path.as_deref().filter(|lnk| !lnk.is_empty());
    if let Some(lnk) = lnk.filter(|lnk| exists(Path::new(lnk))) {
        return Some(LaunchTarget::Shortcut(lnk.to_owned()));
    }
    if !entry.target.is_empty() && exists(Path::new(&entry.target)) {
        return Some(LaunchTarget::Program {
            file: entry.target.clone(),
            args: entry.args.clone(),
            working_dir: entry.working_dir.clone(),
        });
    }
    None
}

/// The error for an app with neither its `.lnk` nor its target on disk.
pub fn not_found(name: &str) -> String {
    format!("App not found: {name}")
}

/// Starts `entry` with `ShellExecuteExW` (verb `open`) on a short-lived
/// thread that holds its own `ComGuard`, and waits until the Shell has
/// started it. Returns `Err("App not found: <name>")` when neither the
/// `.lnk` nor the target exists, and `Err` when the Shell fails. On
/// non-Windows platforms the Shell step returns `Err("unsupported platform")`.
pub fn launch(entry: &AppEntry) -> Result<(), String> {
    let target = launch_target(entry, Path::exists).ok_or_else(|| not_found(&entry.name))?;
    let name = entry.name.clone();
    std::thread::Builder::new()
        .name(LAUNCH_THREAD.to_owned())
        .spawn(move || imp::shell_open(&target))
        .map_err(|e| format!("could not spawn thread {LAUNCH_THREAD:?}: {e}"))?
        .join()
        .map_err(|_| format!("thread {LAUNCH_THREAD:?} panicked"))?
        .map_err(|error| format!("Could not start {name}: {error}"))
}

#[cfg(windows)]
mod imp {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL,
    };

    use super::LaunchTarget;
    use crate::win32::com::ComGuard;
    use crate::win32::wide::to_wide;

    /// Opens `target` with the Shell. Runs on the launch thread.
    pub(super) fn shell_open(target: &LaunchTarget) -> Result<(), String> {
        let _com = ComGuard::new()?;
        // Lets the started app take the foreground. It fails when Grouper
        // is not in the foreground itself, which only means the app may
        // open behind the current window.
        // SAFETY: takes no pointers; ASFW_ANY is a documented value.
        if let Err(error) = unsafe { AllowSetForegroundWindow(ASFW_ANY) } {
            log::debug!("AllowSetForegroundWindow failed: {error}");
        }
        let (file, args, dir) = match target {
            LaunchTarget::Shortcut(lnk) => (lnk.as_str(), "", ""),
            LaunchTarget::Program {
                file,
                args,
                working_dir,
            } => (file.as_str(), args.as_str(), working_dir.as_str()),
        };
        // Every buffer below is NUL-terminated and lives until this
        // function returns, after ShellExecuteExW is done with it.
        let verb = to_wide("open");
        let file = to_wide(file);
        let args = to_wide(args);
        let dir = to_wide(dir);
        let mut info = SHELLEXECUTEINFOW {
            // The struct is about a hundred bytes, so the cast is exact.
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: optional(&args),
            lpDirectory: optional(&dir),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        // SAFETY: info is fully initialised with its cbSize set, and every
        // string it points to is NUL-terminated and outlives the call.
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| e.message())
    }

    /// NULL for an empty string (the Shell then uses its default), else a
    /// pointer to `wide`, which must stay alive while the pointer is used.
    fn optional(wide: &[u16]) -> PCWSTR {
        if wide.len() <= 1 {
            PCWSTR::null()
        } else {
            PCWSTR(wide.as_ptr())
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::LaunchTarget;

    pub(super) fn shell_open(_target: &LaunchTarget) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn entry(lnk: Option<&str>) -> AppEntry {
        AppEntry {
            name: "Tool".into(),
            target: r"C:\tool.exe".into(),
            args: "-x".into(),
            working_dir: r"C:\work".into(),
            lnk_path: lnk.map(str::to_owned),
            ..AppEntry::default()
        }
    }

    #[test]
    fn prefers_the_shortcut_when_it_exists() {
        let picked = launch_target(&entry(Some(r"C:\tool.lnk")), |_| true);
        assert_eq!(picked, Some(LaunchTarget::Shortcut(r"C:\tool.lnk".into())));
    }

    #[test]
    fn falls_back_to_the_target_with_args_and_dir() {
        let lnk_gone = |path: &Path| !path.to_string_lossy().ends_with(".lnk");
        let expected = Some(LaunchTarget::Program {
            file: r"C:\tool.exe".into(),
            args: "-x".into(),
            working_dir: r"C:\work".into(),
        });
        assert_eq!(
            launch_target(&entry(Some(r"C:\tool.lnk")), lnk_gone),
            expected
        );
        assert_eq!(launch_target(&entry(Some("")), |_| true), expected);
        assert_eq!(launch_target(&entry(None), |_| true), expected);
    }

    #[test]
    fn nothing_on_disk_is_not_found() {
        assert_eq!(launch_target(&entry(Some(r"C:\tool.lnk")), |_| false), None);
        let missing = AppEntry {
            name: "Gone".into(),
            target: r"C:\does\not\exist\gone.exe".into(),
            ..AppEntry::default()
        };
        assert_eq!(launch(&missing), Err("App not found: Gone".into()));
    }
}
