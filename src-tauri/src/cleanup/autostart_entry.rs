//! The "Start with Windows" entry that `tauri-plugin-autostart` writes,
//! found and removed by its exact value name.
//!
//! The plugin (through the `auto-launch` crate) writes the value
//! [`RUN_VALUE_NAME`] under `HKCU\...\CurrentVersion\Run`, holding
//! `<path to Grouper.exe> --background`, and the same name under
//! `HKCU\...\Explorer\StartupApproved\Run` (the Task Manager toggle). Its
//! own `disable` only removes the first. Cleanup removes both, and only
//! when the `Run` value really points at a `Grouper.exe`.

/// The plugin's value name, passed to it explicitly in
/// [`crate::autostart::plugin`] so the two never drift apart.
pub const RUN_VALUE_NAME: &str = "Grouper";

/// `HKCU` subkey holding the sign-in commands.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// `HKCU` subkey holding Task Manager's enabled/disabled flag per entry.
pub const STARTUP_APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// The current user's registry, or a fake in tests. A missing key or value
/// is `Ok(None)` when reading and `Ok(())` when deleting.
pub trait Registry {
    /// The string value `name` under `HKCU\{key}`.
    fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, String>;
    /// Deletes value `name` under `HKCU\{key}`.
    fn delete_value(&self, key: &str, name: &str) -> Result<(), String>;
}

/// What [`remove`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Grouper's entry was there and is gone.
    Removed,
    /// There was no entry (start with Windows was off).
    NotFound,
    /// A value called `Grouper` runs something else; it was left alone.
    NotOurs(String),
}

/// True when Run-key command `data` starts `Grouper.exe`, quoted or not.
pub fn is_grouper_command(data: &str) -> bool {
    let lower = data.trim().trim_start_matches('"').to_lowercase();
    let exe = lower.split(".exe").next().unwrap_or_default();
    lower.contains(".exe")
        && (exe == "grouper" || exe.ends_with("\\grouper") || exe.ends_with("/grouper"))
}

/// Removes Grouper's sign-in entry and its Task Manager flag. A value of
/// the same name that runs another program is left as it is, and so is
/// its flag.
pub fn remove(registry: &impl Registry) -> Result<Outcome, String> {
    let outcome = match registry.read_string(RUN_KEY, RUN_VALUE_NAME)? {
        None => Outcome::NotFound,
        Some(data) if !is_grouper_command(&data) => return Ok(Outcome::NotOurs(data)),
        Some(_) => {
            registry.delete_value(RUN_KEY, RUN_VALUE_NAME)?;
            Outcome::Removed
        }
    };
    registry.delete_value(STARTUP_APPROVED_KEY, RUN_VALUE_NAME)?;
    Ok(outcome)
}

/// The real `HKEY_CURRENT_USER`.
pub struct CurrentUser;

#[cfg(windows)]
impl Registry for CurrentUser {
    fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, String> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};

        let (key_w, name_w) = (to_wide(key), to_wide(name));
        let mut bytes = 0_u32;
        // SAFETY: both strings are NUL-terminated and outlive the call; a
        // NULL data pointer asks only for the size, written to `bytes`.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key_w.as_ptr()),
                PCWSTR(name_w.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                None,
                Some(&mut bytes),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status != ERROR_SUCCESS {
            return Err(format!("read {key}\\{name}: {status:?}"));
        }
        let mut buffer = vec![0_u16; (bytes as usize).div_ceil(2) + 1];
        let mut size = u32::try_from(buffer.len() * 2).map_err(|e| e.to_string())?;
        // SAFETY: `buffer` holds `size` writable bytes and outlives the
        // call; RegGetValueW writes at most that many, NUL included.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key_w.as_ptr()),
                PCWSTR(name_w.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        match status {
            ERROR_SUCCESS => Ok(Some(crate::win32::wide::from_wide(&buffer))),
            ERROR_FILE_NOT_FOUND => Ok(None),
            other => Err(format!("read {key}\\{name}: {other:?}")),
        }
    }

    fn delete_value(&self, key: &str, name: &str) -> Result<(), String> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
        use windows::Win32::System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER};

        let (key_w, name_w) = (to_wide(key), to_wide(name));
        // SAFETY: both strings are NUL-terminated and outlive the call,
        // which deletes one named value and nothing else.
        let status = unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key_w.as_ptr()),
                PCWSTR(name_w.as_ptr()),
            )
        };
        match status {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            other => Err(format!("delete {key}\\{name}: {other:?}")),
        }
    }
}

#[cfg(windows)]
use crate::win32::wide::to_wide;

#[cfg(not(windows))]
impl Registry for CurrentUser {
    fn read_string(&self, _key: &str, _name: &str) -> Result<Option<String>, String> {
        Err("unsupported platform".to_owned())
    }

    fn delete_value(&self, _key: &str, _name: &str) -> Result<(), String> {
        Err("unsupported platform".to_owned())
    }
}
