//! Reading `app_state.json` (with `.bak` fallback and corrupt-file quarantine).
//! Writing lives in [`super::disk_write`].

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::model::{AppState, CURRENT_SCHEMA};

pub const STATE_FILE: &str = "app_state.json";
const BACKUP_FILE: &str = "app_state.json.bak";
const TEMP_FILE: &str = "app_state.json.tmp";
const BACKUP_TEMP_FILE: &str = "app_state.json.bak.tmp";

/// The file paths used for one state directory.
#[derive(Debug, Clone)]
pub struct StatePaths {
    pub main: PathBuf,
    pub backup: PathBuf,
    pub temp: PathBuf,
    pub backup_temp: PathBuf,
}

impl StatePaths {
    pub fn new(dir: &Path) -> Self {
        Self {
            main: dir.join(STATE_FILE),
            backup: dir.join(BACKUP_FILE),
            temp: dir.join(TEMP_FILE),
            backup_temp: dir.join(BACKUP_TEMP_FILE),
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
    /// The main file was bad JSON; the backup parsed.
    Backup(AppState),
    /// Both were unusable; the bad main file was moved aside.
    Corrupt,
    /// The main file was written by a newer build. It parsed, but it must
    /// never be rewritten by this build (fields it does not know are lost).
    Newer(AppState),
}

/// What reading one file gave: its text, nothing, or an IO error.
enum Read {
    Text(String),
    Missing,
}

/// Reads `path`; `NotFound` is [`Read::Missing`], any other error is `Err`.
fn read_text(path: &Path) -> Result<Read, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Read::Text(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Read::Missing),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// The `schemaVersion` in `text` if it is a JSON object that has one.
fn raw_schema_version(text: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("schemaVersion")?
        .as_u64()
}

/// Loads the state from disk.
///
/// - Missing main file: [`Loaded::Fresh`].
/// - Main file that cannot be read: `Err`, and the file is left in place.
/// - Main file from a newer schema: [`Loaded::Newer`] if it parses, else
///   `Err`; it is never moved or rewritten.
/// - Bad JSON: use `.bak` if it parses. If `.bak` cannot be read (other than
///   missing), `Err` and nothing moves. Otherwise the bad main file is moved
///   to `app_state.corrupt-<unix>.json` and [`Loaded::Corrupt`] is returned.
pub fn read_state(paths: &StatePaths) -> Result<Loaded, String> {
    let text = match read_text(&paths.main)? {
        Read::Missing => return Ok(Loaded::Fresh),
        Read::Text(text) => text,
    };
    if let Some(version) = raw_schema_version(&text).filter(|v| *v > u64::from(CURRENT_SCHEMA)) {
        return serde_json::from_str(&text).map(Loaded::Newer).map_err(|e| {
            format!("app_state.json is schema v{version} and does not fit this build: {e}")
        });
    }
    let parse_error = match serde_json::from_str(&text) {
        Ok(state) => return Ok(Loaded::Main(state)),
        Err(error) => error,
    };
    log::warn!("app state is corrupt, trying backup: {parse_error}");
    if let Read::Text(backup) = read_text(&paths.backup)? {
        match serde_json::from_str(&backup) {
            Ok(state) => {
                log::warn!("loaded app state from {}", paths.backup.display());
                return Ok(Loaded::Backup(state));
            }
            Err(error) => log::warn!("backup unusable too: {error}"),
        }
    }
    quarantine(&paths.main).map(|()| Loaded::Corrupt)
}

/// Moves a corrupt main file to `app_state.corrupt-<unix>.json`.
fn quarantine(main: &Path) -> Result<(), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let aside = main.with_file_name(format!("app_state.corrupt-{stamp}.json"));
    fs::rename(main, &aside)
        .map_err(|e| format!("could not move corrupt {} aside: {e}", main.display()))?;
    log::error!("moved corrupt app state to {}", aside.display());
    Ok(())
}
