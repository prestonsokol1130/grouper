//! Atomic, symlink-safe writes of `app_state.json` and its `.bak`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::disk::StatePaths;
use super::model::AppState;

/// Errors if `path` exists and is a symlink: writes must never follow a
/// link out of the app data folder.
pub(crate) fn reject_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(format!(
            "refusing to write through symlink {}",
            path.display()
        )),
        _ => Ok(()),
    }
}

/// Creates `path` as a brand-new file (`create_new`, so nothing existing is
/// opened or truncated), after removing a stale regular file left by a crash.
fn create_fresh(path: &Path) -> Result<File, String> {
    reject_symlink(path)?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("remove stale {}: {e}", path.display()))?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create {}: {e}", path.display()))
}

/// Writes `bytes` to a fresh file at `path`, then flushes and syncs it.
pub(crate) fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = create_fresh(path)?;
    file.write_all(bytes)
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    file.flush()
        .map_err(|e| format!("flush {}: {e}", path.display()))?;
    file.sync_all()
        .map_err(|e| format!("sync {}: {e}", path.display()))
}

/// Removes a temp file after a failed write; only logs if that fails too.
pub(crate) fn discard(path: &Path) {
    let is_file = fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file());
    if is_file {
        if let Err(error) = fs::remove_file(path) {
            log::warn!("could not remove {}: {error}", path.display());
        }
    }
}

/// Copies the current main file to `.bak` through `.bak.tmp` + rename, so a
/// failure never leaves a half-written backup.
fn backup_main(paths: &StatePaths) -> Result<(), String> {
    let bytes = fs::read(&paths.main).map_err(|e| format!("read {}: {e}", paths.main.display()))?;
    write_synced(&paths.backup_temp, &bytes)?;
    fs::rename(&paths.backup_temp, &paths.backup)
        .map_err(|e| format!("rename {} over backup: {e}", paths.backup_temp.display()))
}

/// Writes `state` atomically: pretty JSON to `.tmp` (flushed and synced),
/// back up the current main file to `.bak` (only when `backup_current`, so a
/// bad main file never replaces a good backup), then rename `.tmp` over main.
/// On any error the previous main and `.bak` bytes are left unchanged.
pub fn write_state(
    paths: &StatePaths,
    state: &AppState,
    backup_current: bool,
) -> Result<(), String> {
    for path in [&paths.main, &paths.backup, &paths.temp, &paths.backup_temp] {
        reject_symlink(path)?;
    }
    let json =
        serde_json::to_string_pretty(state).map_err(|e| format!("serialize app state: {e}"))?;
    let result = write_synced(&paths.temp, json.as_bytes()).and_then(|()| {
        if backup_current && paths.main.is_file() {
            backup_main(paths)?;
        }
        fs::rename(&paths.temp, &paths.main)
            .map_err(|e| format!("rename {} over main: {e}", paths.temp.display()))
    });
    if result.is_err() {
        discard(&paths.temp);
        discard(&paths.backup_temp);
    }
    result
}
