//! Adding apps by hand from a program path (`.exe` or `.lnk`).

use std::path::{Path, PathBuf};

use crate::state::ids::app_id;
use crate::state::{AppEntry, AppSource, AppState, Store};
use crate::win32::com::ComGuard;
use crate::win32::lnk_read::{resolve_lnk, ResolvedLink};
use crate::win32::version_info::file_description;

/// Shortcut targets that are added as the program itself. Anything else
/// (an MSI icon file, a document, a folder) keeps the shortcut.
pub const PROGRAM_TARGETS: &[&str] = &["exe", "com", "bat", "cmd"];

/// The file name without its extension.
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Display name of a program: `describe(path)` (its version resource's
/// `FileDescription`) when that is not empty, else the file stem.
pub fn exe_name(path: &Path, describe: impl FnOnce(&Path) -> Option<String>) -> String {
    describe(path)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| file_stem(path))
}

/// A Manual entry with its id computed from `target` and `args`.
fn manual(
    name: String,
    target: String,
    args: String,
    dir: String,
    lnk: Option<String>,
) -> AppEntry {
    AppEntry {
        id: app_id(&target, &args),
        name,
        target,
        args,
        working_dir: dir,
        source: AppSource::Manual,
        lnk_path: lnk,
        missing: false,
        hidden: false,
    }
}

/// Lowercase extension of `path`, if any.
fn extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
}

/// True when the shortcut target `target` is a program file on disk.
fn is_program_file(target: &str) -> bool {
    let path = Path::new(target);
    !target.is_empty()
        && path.is_file()
        && extension(path).is_some_and(|ext| PROGRAM_TARGETS.contains(&ext.as_str()))
}

/// The entry for the shortcut at `lnk` (path text `text`) read as `link`.
///
/// Shortcuts break easily, so when the target is a program on disk the
/// program itself is added: its args and working folder come from the
/// shortcut, its name is the shortcut's name (the program's description if
/// that is empty), and `lnk_path` is `None`. Only a shortcut without such a
/// target (a Store app, an MSI advertised shortcut, a missing program) is
/// kept as a shortcut, with `lnk_path` set. If its target is also empty its
/// id comes from the shortcut path, so two such shortcuts never collide.
fn entry_from_lnk(
    lnk: &Path,
    text: &str,
    link: ResolvedLink,
    describe: impl FnOnce(&Path) -> Option<String>,
) -> AppEntry {
    let stem = file_stem(lnk);
    if is_program_file(&link.target) {
        let name = if stem.is_empty() {
            exe_name(Path::new(&link.target), describe)
        } else {
            stem
        };
        return manual(name, link.target, link.args, link.working_dir, None);
    }
    let key = if link.target.is_empty() {
        text
    } else {
        &link.target
    };
    let id = app_id(key, &link.args);
    let entry = manual(
        stem,
        link.target,
        link.args,
        link.working_dir,
        Some(text.to_owned()),
    );
    AppEntry { id, ..entry }
}

/// Builds a Manual entry for the program at `path`.
///
/// - `.lnk`: read with `read_lnk` (normally [`resolve_lnk`], which needs a
///   `ComGuard` on this thread), then see [`entry_from_lnk`]: a shortcut to
///   a program adds the program, any other shortcut is kept.
/// - `.exe`: named by [`exe_name`] with `describe`; its working folder is
///   the folder it is in, as when it is opened from Explorer.
///
/// `Err` for a relative path, a missing file, or any other kind of file.
pub fn entry_from_path(
    path: &Path,
    read_lnk: impl FnOnce(&Path) -> Result<ResolvedLink, String>,
    describe: impl FnOnce(&Path) -> Option<String>,
) -> Result<AppEntry, String> {
    let shown = path.display();
    if !path.is_absolute() {
        return Err(format!("Not a full path: {shown}"));
    }
    if !path.is_file() {
        return Err(format!("File not found: {shown}"));
    }
    let text = path
        .to_str()
        .ok_or_else(|| format!("Path is not valid Unicode: {shown}"))?;
    match extension(path).as_deref() {
        Some("lnk") => Ok(entry_from_lnk(path, text, read_lnk(path)?, describe)),
        Some("exe") => {
            let dir = path.parent().and_then(Path::to_str).unwrap_or_default();
            let name = exe_name(path, describe);
            Ok(manual(
                name,
                text.to_owned(),
                String::new(),
                dir.to_owned(),
                None,
            ))
        }
        _ => Err(format!("Not a program (.exe or .lnk): {shown}")),
    }
}

/// Adds `entry` to `state`. When an app with the same id is already there,
/// that app is returned instead, no longer missing or hidden: adding it on
/// purpose means the user wants to see it.
pub fn add_entry(state: &mut AppState, entry: AppEntry) -> AppEntry {
    if let Some(existing) = state.apps.iter_mut().find(|app| app.id == entry.id) {
        existing.missing = false;
        existing.hidden = false;
        return existing.clone();
    }
    state.apps.push(entry.clone());
    entry
}

/// Adds the programs at `paths` and returns their entries (existing ones
/// for duplicates), publishing the new state once. A path that cannot be
/// added is logged and skipped; `Err` only when none could be added.
///
/// Blocks: reads shortcuts on the calling thread, which gets its own
/// `ComGuard` here.
pub fn add_paths(
    store: &Store,
    paths: &[PathBuf],
    publish: impl FnOnce(&AppState),
) -> Result<Vec<AppEntry>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let _com = ComGuard::new()?;
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        match entry_from_path(path, resolve_lnk, file_description) {
            Ok(entry) => entries.push(entry),
            Err(error) => {
                log::warn!("could not add app {}: {error}", path.display());
                errors.push(error);
            }
        }
    }
    if entries.is_empty() {
        return Err(errors.join("; "));
    }
    store.update_and_publish(
        |state| {
            Ok(entries
                .into_iter()
                .map(|entry| add_entry(state, entry))
                .collect())
        },
        publish,
    )
}
