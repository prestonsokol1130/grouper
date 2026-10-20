//! Installed-app discovery: finds every app Start lists under "All apps".
//!
//! [`scan`] walks both Start Menu "Programs" folders ([`roots`], [`walk`]),
//! reads each shortcut, drops the ones that are not apps ([`filter`]),
//! collapses duplicates ([`dedupe`]), then adds the Applications-folder
//! apps no shortcut covers, such as Store apps ([`apps_folder`]).
//! [`merge::merge_scan`] folds the result into the saved state.

pub mod apps_folder;
pub mod dedupe;
pub mod filter;
pub mod merge;
pub mod roots;
pub mod walk;

#[cfg(test)]
mod apps_folder_tests;
#[cfg(test)]
mod filter_tests;
#[cfg(all(test, windows))]
mod live_tests;

use std::path::Path;

use crate::state::AppEntry;
use crate::win32::apps_folder::list_apps_folder;
use crate::win32::com::ComGuard;
use crate::win32::lnk_aumid::read_lnk_aumid;
use crate::win32::lnk_read::resolve_lnk;
use apps_folder::Known;
use dedupe::Candidate;

/// Name of the thread [`scan`] runs on.
const SCAN_THREAD: &str = "app-scan";

/// The result of one [`scan`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanOutcome {
    /// One entry per app found: `lnk_path` is set for a shortcut, and the
    /// target is `shell:AppsFolder\{id}` for an Applications-folder app.
    pub apps: Vec<AppEntry>,
    /// True only when both Start Menu folders were found and read in full,
    /// every shortcut in them could be read, and so could the Applications
    /// folder. A partial scan must never mark saved apps as missing.
    pub complete: bool,
    /// Shortcuts that exist but could not be read. Saved apps that came from
    /// one of these are not marked missing.
    pub unreadable: Vec<String>,
}

/// Scans the Start Menu. Blocks the caller until the scan is done.
///
/// The work runs on its own background thread, which holds its own
/// [`ComGuard`]. A shortcut that cannot be read is logged and skipped; it
/// never stops the scan. Returns `Err` only when no Start Menu folder can be
/// found or COM cannot start (always, on non-Windows platforms).
pub fn scan() -> Result<ScanOutcome, String> {
    std::thread::Builder::new()
        .name(SCAN_THREAD.to_owned())
        .spawn(scan_on_this_thread)
        .map_err(|e| format!("could not spawn thread {SCAN_THREAD:?}: {e}"))?
        .join()
        .map_err(|_| format!("thread {SCAN_THREAD:?} panicked"))?
}

/// The scan itself. COM stays initialised until every shortcut is read.
fn scan_on_this_thread() -> Result<ScanOutcome, String> {
    let _com = ComGuard::new()?;
    let roots = roots::scan_roots()?;
    let roots_found = roots.len();
    let mut walks_complete = true;
    let (mut shortcuts, mut rejected) = (0_usize, 0_usize);
    let mut candidates = Vec::new();
    let mut known = Vec::new();
    let mut unreadable = Vec::new();
    for root in roots {
        let walked = walk::find_shortcuts(&root.path);
        walks_complete &= walked.complete;
        for lnk in walked.shortcuts {
            shortcuts += 1;
            match read_candidate(&lnk, root.per_user) {
                Ok(Some((candidate, aumid))) => {
                    known.push(Known {
                        name: candidate.name.clone(),
                        target: candidate.target.clone(),
                        aumid,
                    });
                    candidates.push(candidate);
                }
                Ok(None) => rejected += 1,
                Err(error) => {
                    log::warn!("skipping shortcut {}: {error}", lnk.display());
                    unreadable.push(lnk.to_string_lossy().into_owned());
                }
            }
        }
    }
    let mut apps = dedupe::dedupe(candidates);
    log::info!(
        "app scan: {} apps from {shortcuts} shortcuts ({rejected} rejected, {} unreadable)",
        apps.len(),
        unreadable.len()
    );
    let folder_read = match list_apps_folder() {
        Ok(items) => {
            let total = items.len();
            let found = apps_folder::folder_apps(&known, items, filter::is_file);
            log::info!(
                "app scan: {} more apps from {total} Applications folder items \
                 ({} already found, {} rejected)",
                found.apps.len(),
                found.duplicates,
                found.rejected
            );
            apps.extend(found.apps);
            true
        }
        Err(error) => {
            log::warn!("could not read the Applications folder: {error}");
            false
        }
    };
    let complete = folder_read && is_complete(roots_found, walks_complete, unreadable.len());
    if !complete {
        log::warn!("app scan was incomplete; no saved app will be marked missing");
    }
    Ok(ScanOutcome {
        apps,
        complete,
        unreadable,
    })
}

/// A scan is complete only when every Start Menu folder was found, every
/// walk read its whole tree, and no shortcut failed to read.
fn is_complete(roots_found: usize, walks_complete: bool, unreadable: usize) -> bool {
    roots_found == roots::ROOT_COUNT && walks_complete && unreadable == 0
}

/// Reads one shortcut and its AppUserModelID. `Ok(None)` means it was read
/// but is not an app (the reason is logged at debug level).
fn read_candidate(
    lnk: &Path,
    per_user: bool,
) -> Result<Option<(Candidate, Option<String>)>, String> {
    let name = lnk
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let link = resolve_lnk(lnk)?;
    if let Err(reason) = filter::check(&name, &link.target, filter::is_file) {
        log::debug!("not an app: {} ({reason})", lnk.display());
        return Ok(None);
    }
    // Only used to match Applications-folder items, so a failure is no loss.
    let aumid = read_lnk_aumid(lnk).ok().flatten();
    let candidate = Candidate {
        name,
        target: link.target,
        args: link.args,
        working_dir: link.working_dir,
        lnk_path: lnk.to_string_lossy().into_owned(),
        per_user,
    };
    Ok(Some((candidate, aumid)))
}

#[cfg(test)]
mod tests {
    use super::{is_complete, roots::ROOT_COUNT};

    #[test]
    fn only_a_full_clean_scan_is_complete() {
        assert!(is_complete(ROOT_COUNT, true, 0));
        // One Start Menu folder could not be found.
        assert!(!is_complete(ROOT_COUNT - 1, true, 0));
        // Part of a folder tree could not be read.
        assert!(!is_complete(ROOT_COUNT, false, 0));
        // One shortcut could not be read.
        assert!(!is_complete(ROOT_COUNT, true, 1));
    }

    /// Every Windows install (CI included) has Start Menu apps.
    #[cfg(windows)]
    #[test]
    fn scan_finds_apps() {
        use super::scan;

        let outcome = scan().unwrap();
        assert!(!outcome.apps.is_empty());
        for app in &outcome.apps {
            assert!(!app.target.is_empty(), "{app:?}");
            assert!(
                app.lnk_path.is_some() || app.apps_folder_id().is_some(),
                "{app:?}"
            );
            assert_eq!(app.id.len(), 16, "{app:?}");
        }
    }
}
