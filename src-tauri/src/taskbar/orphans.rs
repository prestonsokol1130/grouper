//! Startup clean-up of shortcuts left behind by deleted groups.
//!
//! A deleted group's files are removed by a background job. When Grouper
//! quits or is closed before that job runs (or the job fails), its shortcut
//! stays in the Start Menu, pointing at an `.ico` the cache clean-up later
//! deletes, so it shows a blank page. At startup this finds such shortcuts
//! by their AppUserModelID, unpins them, and deletes them.
//!
//! Safe by construction: only Grouper's own Start Menu folder is changed,
//! never through a symlink or junction. Only regular `.lnk` files whose
//! AppUserModelID is `com.grouper.group.<id>` for an id that is not a
//! current group are deleted, plus `*.{16 hex}.tmp` files the shortcut
//! writer stages there. Everything else is left alone. In the taskbar's
//! pinned folder the same shortcuts are only unpinned, never touched.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::group_removal::{unpin_each, Pins};
use super::shortcut::AUMID_PREFIX;
use crate::cleanup::files::is_link;
use crate::icons::cache::check_id;

/// Reads a shortcut's AppUserModelID (`read_lnk_aumid` in the app).
pub type ReadAumid<'a> = &'a dyn Fn(&Path) -> Result<Option<String>, String>;

/// What [`find_orphans`] found in one folder.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Orphans {
    /// Shortcuts of deleted groups, with the group id.
    pub shortcuts: Vec<(PathBuf, String)>,
    /// Temp files the shortcut writer left behind.
    pub temps: Vec<PathBuf>,
    /// Whether the folder was really looked through: true when it is
    /// missing (nothing can be left in it) or was listed, false when it is
    /// a link, not a folder, or could not be listed.
    pub listed: bool,
}

/// The deleted group a shortcut with AppUserModelID `aumid` belongs to:
/// `Some(id)` for `com.grouper.group.<id>` when `id` is a safe id and not
/// in `live`; `None` for anything else.
pub fn orphan_group(aumid: Option<&str>, live: &HashSet<&str>) -> Option<String> {
    let id = aumid?.strip_prefix(AUMID_PREFIX)?;
    (check_id(id).is_ok() && !live.contains(id)).then(|| id.to_owned())
}

/// True for `{stem}.{16 lowercase hex}.tmp`, the shortcut writer's temp
/// file name.
pub fn is_stray_temp(name: &str) -> bool {
    name.strip_suffix(".tmp")
        .and_then(|rest| rest.rsplit_once('.'))
        .is_some_and(|(_, nonce)| {
            nonce.len() == 16
                && nonce
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

/// The orphaned shortcuts and stray temp files directly in `dir`. A
/// missing folder, or one that is a link, gives nothing (see
/// [`Orphans::listed`]). Links and folders inside are skipped; so is a
/// shortcut whose ID cannot be read.
pub fn find_orphans(dir: &Path, live: &HashSet<&str>, read_aumid: ReadAumid<'_>) -> Orphans {
    let mut found = Orphans::default();
    match fs::symlink_metadata(dir) {
        Ok(meta) if meta.is_dir() && !is_link(&meta) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            found.listed = true;
            return found;
        }
        _ => return found,
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    found.listed = true;
    for entry in entries.flatten() {
        let path = entry.path();
        let plain = fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && !is_link(&m));
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !plain {
            continue;
        }
        if name.ends_with(".lnk") {
            match read_aumid(&path) {
                Ok(aumid) => {
                    if let Some(id) = orphan_group(aumid.as_deref(), live) {
                        found.shortcuts.push((path, id));
                    }
                }
                Err(error) => log::debug!("skipping {}: {error}", path.display()),
            }
        } else if is_stray_temp(&name) {
            found.temps.push(path);
        }
    }
    found
}

/// What [`sweep_orphans`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Swept {
    /// Orphaned shortcuts deleted from Grouper's folder.
    pub shortcuts: usize,
    /// Stray temp files deleted from it.
    pub temps: usize,
    /// Pinned copies of deleted groups' shortcuts that were unpinned.
    pub pins: usize,
    /// Both folders were found and looked through ([`Orphans::listed`]).
    pub listed: bool,
    /// Deleted groups (sorted ids) that still have a shortcut or a pinned
    /// copy on disk after the sweep: their `.ico` files must stay.
    pub kept: Vec<String>,
}

/// Unpins and deletes the orphaned shortcuts in Grouper's Start Menu
/// folder `dir`, deletes its stray temp files, and unpins the deleted
/// groups' copies in `pins.taskbar_dir` (without touching those files).
/// The caller holds a `ComGuard` when `read_aumid` needs one. Failures are
/// logged and the rest still run; what is still on disk afterwards is
/// reported in [`Swept::kept`]. A failed unpin alone keeps nothing: the
/// Shell fails it for every group that was never pinned.
pub fn sweep_orphans(
    dir: Option<&Path>,
    live: &HashSet<&str>,
    read_aumid: ReadAumid<'_>,
    pins: &Pins<'_>,
) -> Swept {
    let mut swept = Swept {
        listed: true,
        ..Swept::default()
    };
    match dir {
        None => swept.listed = false,
        Some(dir) => {
            let found = find_orphans(dir, live, read_aumid);
            swept.listed &= found.listed;
            for (path, id) in found.shortcuts {
                unpin_each(std::slice::from_ref(&path), pins);
                if remove(&path) {
                    log::info!("removed deleted group {id}'s shortcut {}", path.display());
                    swept.shortcuts += 1;
                }
                if path.exists() {
                    swept.kept.push(id);
                }
            }
            swept.temps = found.temps.iter().filter(|path| remove(path)).count();
        }
    }
    match pins.taskbar_dir {
        None => swept.listed = false,
        Some(taskbar) => {
            let found = find_orphans(taskbar, live, read_aumid);
            swept.listed &= found.listed;
            let pinned: Vec<PathBuf> = found
                .shortcuts
                .iter()
                .map(|(path, _)| path.clone())
                .collect();
            unpin_each(&pinned, pins);
            swept.pins = pinned.len();
            let left = found
                .shortcuts
                .into_iter()
                .filter(|(path, _)| path.exists());
            swept.kept.extend(left.map(|(_, id)| id));
        }
    }
    swept.kept.sort();
    swept.kept.dedup();
    swept
}

/// Deletes `path`; a failure is logged and gives false.
fn remove(path: &Path) -> bool {
    fs::remove_file(path)
        .map_err(|error| log::warn!("could not remove {}: {error}", path.display()))
        .is_ok()
}
