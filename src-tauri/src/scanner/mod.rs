//! Installed-app discovery: finds the apps in the Start Menu.
//!
//! [`scan`] walks both Start Menu "Programs" folders ([`roots`], [`walk`]),
//! reads each shortcut, drops the ones that are not apps ([`filter`]),
//! collapses duplicates ([`dedupe`]), and [`merge::merge_scan`] folds the
//! result into the saved state.

pub mod dedupe;
pub mod filter;
pub mod merge;
pub mod roots;
pub mod walk;

#[cfg(test)]
mod filter_tests;

use std::path::Path;

use crate::state::AppEntry;
use crate::win32::com::ComGuard;
use crate::win32::lnk_read::resolve_lnk;
use dedupe::Candidate;

/// Name of the thread [`scan`] runs on.
const SCAN_THREAD: &str = "app-scan";

/// Scans the Start Menu and returns one entry per app found, with
/// `lnk_path` set. Blocks the caller until the scan is done.
///
/// The work runs on its own background thread, which holds its own
/// [`ComGuard`]. A shortcut that cannot be read is logged and skipped; it
/// never stops the scan. Returns `Err` only when no Start Menu folder can be
/// found or COM cannot start (always, on non-Windows platforms).
pub fn scan() -> Result<Vec<AppEntry>, String> {
    std::thread::Builder::new()
        .name(SCAN_THREAD.to_owned())
        .spawn(scan_on_this_thread)
        .map_err(|e| format!("could not spawn thread {SCAN_THREAD:?}: {e}"))?
        .join()
        .map_err(|_| format!("thread {SCAN_THREAD:?} panicked"))?
}

/// Counts for the summary log line.
#[derive(Default)]
struct Tally {
    shortcuts: usize,
    unreadable: usize,
    rejected: usize,
}

/// The scan itself. COM stays initialised until every shortcut is read.
fn scan_on_this_thread() -> Result<Vec<AppEntry>, String> {
    let _com = ComGuard::new()?;
    let mut tally = Tally::default();
    let mut candidates = Vec::new();
    for root in roots::scan_roots()? {
        for lnk in walk::find_shortcuts(&root.path) {
            tally.shortcuts += 1;
            match read_candidate(&lnk, root.per_user) {
                Ok(Some(candidate)) => candidates.push(candidate),
                Ok(None) => tally.rejected += 1,
                Err(error) => {
                    tally.unreadable += 1;
                    log::warn!("skipping shortcut {}: {error}", lnk.display());
                }
            }
        }
    }
    let apps = dedupe::dedupe(candidates);
    log::info!(
        "app scan: {} apps from {} shortcuts ({} rejected, {} unreadable)",
        apps.len(),
        tally.shortcuts,
        tally.rejected,
        tally.unreadable
    );
    Ok(apps)
}

/// Reads one shortcut. `Ok(None)` means it was read but is not an app (the
/// reason is logged at debug level).
fn read_candidate(lnk: &Path, per_user: bool) -> Result<Option<Candidate>, String> {
    let name = lnk
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let link = resolve_lnk(lnk)?;
    if let Err(reason) = filter::check(&name, &link.target, |t| Path::new(t).exists()) {
        log::debug!("not an app: {} ({reason})", lnk.display());
        return Ok(None);
    }
    Ok(Some(Candidate {
        name,
        target: link.target,
        args: link.args,
        working_dir: link.working_dir,
        lnk_path: lnk.to_string_lossy().into_owned(),
        per_user,
    }))
}

#[cfg(all(test, windows))]
mod tests {
    use super::scan;

    /// Every Windows install (CI included) has Start Menu apps.
    #[test]
    fn scan_finds_apps() {
        let apps = scan().unwrap();
        assert!(!apps.is_empty());
        for app in &apps {
            assert!(!app.target.is_empty(), "{app:?}");
            assert!(app.lnk_path.is_some(), "{app:?}");
            assert_eq!(app.id.len(), 16, "{app:?}");
        }
    }
}
