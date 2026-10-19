//! Reading and atomically writing `app_state.json` (plus `.bak` / `.tmp`).

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::model::AppState;

pub const STATE_FILE: &str = "app_state.json";
const BACKUP_FILE: &str = "app_state.json.bak";
const TEMP_FILE: &str = "app_state.json.tmp";

/// The three file paths used for one state directory.
#[derive(Debug, Clone)]
pub struct StatePaths {
    pub main: PathBuf,
    pub backup: PathBuf,
    pub temp: PathBuf,
}

impl StatePaths {
    pub fn new(dir: &Path) -> Self {
        Self {
            main: dir.join(STATE_FILE),
            backup: dir.join(BACKUP_FILE),
            temp: dir.join(TEMP_FILE),
        }
    }
}

/// What [`read_state`] found on disk.
#[derive(Debug)]
pub enum Loaded {
    /// No main file yet: a first run.
    Fresh,
    /// The main file parsed.
    Main(AppState),
    /// The main file was bad; the backup parsed.
    Backup(AppState),
    /// Both were unusable; the bad main file was moved aside.
    Corrupt,
}

/// Parses one state file.
fn parse_file(path: &Path) -> Result<AppState, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Loads the state: main file, else `.bak`, else moves the bad main file to
/// `app_state.corrupt-<unix>.json` and reports [`Loaded::Corrupt`].
pub fn read_state(paths: &StatePaths) -> Loaded {
    if !paths.main.exists() {
        return Loaded::Fresh;
    }
    let main_error = match parse_file(&paths.main) {
        Ok(state) => return Loaded::Main(state),
        Err(error) => error,
    };
    log::warn!("app state unreadable, trying backup: {main_error}");
    match parse_file(&paths.backup) {
        Ok(state) => {
            log::warn!("loaded app state from {}", paths.backup.display());
            return Loaded::Backup(state);
        }
        Err(error) => log::warn!("backup unusable too: {error}"),
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let aside = paths
        .main
        .with_file_name(format!("app_state.corrupt-{stamp}.json"));
    match fs::rename(&paths.main, &aside) {
        Ok(()) => log::error!("moved corrupt app state to {}", aside.display()),
        Err(error) => log::error!("could not move corrupt app state aside: {error}"),
    }
    Loaded::Corrupt
}

/// Writes `state` atomically: pretty JSON to `.tmp`, flush + fsync, copy the
/// current main file to `.bak` (only when `backup_current` is true, so a bad
/// main file never overwrites a good backup), then rename `.tmp` over main.
pub fn write_state(
    paths: &StatePaths,
    state: &AppState,
    backup_current: bool,
) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(state).map_err(|e| format!("serialize app state: {e}"))?;
    let result = write_temp(&paths.temp, json.as_bytes()).and_then(|()| {
        if backup_current && paths.main.exists() {
            fs::copy(&paths.main, &paths.backup)
                .map_err(|e| format!("copy {} to backup: {e}", paths.main.display()))?;
        }
        fs::rename(&paths.temp, &paths.main)
            .map_err(|e| format!("rename {} over main: {e}", paths.temp.display()))
    });
    if result.is_err() && paths.temp.exists() {
        if let Err(error) = fs::remove_file(&paths.temp) {
            log::warn!("could not remove {}: {error}", paths.temp.display());
        }
    }
    result
}

/// Creates `path`, writes `bytes`, flushes and syncs it to disk.
fn write_temp(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    file.flush()
        .map_err(|e| format!("flush {}: {e}", path.display()))?;
    file.sync_all()
        .map_err(|e| format!("sync {}: {e}", path.display()))
}
