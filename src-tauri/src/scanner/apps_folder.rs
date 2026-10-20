//! Adds the apps that only the Applications folder (`shell:AppsFolder`)
//! knows about, such as Store apps, to the shortcut scan. Pure.

use std::collections::HashSet;

use super::filter::{self, Rejection};
use crate::state::ids::{app_id, apps_folder_target};
use crate::state::{AppEntry, AppSource};
use crate::win32::apps_folder::AppsFolderItem;

/// AppUserModelID prefix of Grouper's own group shortcuts.
const OWN_AUMID_PREFIX: &str = "com.grouper.";

/// What a `.lnk` the shortcut scan read is known by, for matching.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Known {
    pub name: String,
    pub target: String,
    pub aumid: Option<String>,
}

/// What [`folder_apps`] made of the Applications folder.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FolderApps {
    /// New entries, each with target `shell:AppsFolder\{parsing name}`.
    pub apps: Vec<AppEntry>,
    /// Items that are the same app as a scanned `.lnk`.
    pub duplicates: usize,
    /// Items that are not apps (see [`check`]).
    pub rejected: usize,
}

/// The Applications-folder items that are apps and that no scanned `.lnk`
/// already covers, as Start Menu entries. An item matches a `.lnk` by its
/// file target when it has one; otherwise by AppUserModelID, then by name.
/// The `.lnk` entry always wins, so the ids of saved apps never change.
pub fn folder_apps(
    known: &[Known],
    items: Vec<AppsFolderItem>,
    is_file: impl Fn(&str) -> bool,
) -> FolderApps {
    let targets: HashSet<String> = known.iter().map(|k| fold(&k.target)).collect();
    let aumids: HashSet<String> = known
        .iter()
        .filter_map(|k| k.aumid.as_deref())
        .map(fold)
        .collect();
    let names: HashSet<String> = known.iter().map(|k| normal_name(&k.name)).collect();
    let mut out = FolderApps::default();
    let mut seen = HashSet::new();
    for item in items {
        if let Err(reason) = check(&item, &is_file) {
            log::debug!("not an app: {} ({reason})", item.parsing_name);
            out.rejected += 1;
            continue;
        }
        let file = item.target_path.as_deref().filter(|t| is_file_path(t));
        let duplicate = match file {
            Some(path) => targets.contains(&fold(path)),
            None => {
                item.aumid
                    .as_deref()
                    .is_some_and(|id| aumids.contains(&fold(id)))
                    || names.contains(&normal_name(&item.name))
            }
        };
        if duplicate || !seen.insert(fold(&item.parsing_name)) {
            out.duplicates += 1;
            continue;
        }
        out.apps.push(entry(item));
    }
    out
}

/// Why an item is not an app: Grouper's own group shortcuts, web pages,
/// and whatever the shortcut [`filter`] rejects. An item with a file target
/// gets the full filter; one without (a Store app, a `steam:` game, a Shell
/// folder) only the name and web-page checks.
pub fn check(item: &AppsFolderItem, is_file: impl FnOnce(&str) -> bool) -> Result<(), Rejection> {
    let own = |id: &str| id.to_ascii_lowercase().starts_with(OWN_AUMID_PREFIX);
    if own(&item.parsing_name) || item.aumid.as_deref().is_some_and(own) {
        return Err(Rejection::OwnShortcut);
    }
    let target = item.target_path.as_deref().unwrap_or_default();
    if filter::is_web_link(target) || filter::is_web_link(&item.parsing_name) {
        return Err(Rejection::WebLink);
    }
    if is_file_path(target) {
        filter::check(&item.name, target, is_file)
    } else {
        filter::check_name(&item.name)
    }
}

/// True for a drive (`C:\…`) or network (`\\server\…`) path.
pub fn is_file_path(target: &str) -> bool {
    let bytes = target.as_bytes();
    target.starts_with(r"\\") || (bytes.len() > 2 && bytes[1] == b':' && bytes[2] == b'\\')
}

/// The Start Menu entry for `item`.
fn entry(item: AppsFolderItem) -> AppEntry {
    let target = apps_folder_target(&item.parsing_name);
    AppEntry {
        id: app_id(&target, ""),
        name: item.name,
        target,
        source: AppSource::StartMenu,
        ..AppEntry::default()
    }
}

/// Lowercased, for case-insensitive paths and AppUserModelIDs.
fn fold(text: &str) -> String {
    text.trim().to_lowercase()
}

/// Lowercased with runs of white space made one space.
fn normal_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
