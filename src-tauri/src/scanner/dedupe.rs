//! Collapses shortcuts that launch the same app into one entry. Pure.

use std::collections::HashMap;

use crate::state::ids::app_id;
use crate::state::{AppEntry, AppSource};

/// A shortcut that passed the filter, before de-duplication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Display name: the `.lnk` file stem.
    pub name: String,
    pub target: String,
    pub args: String,
    pub working_dir: String,
    pub lnk_path: String,
    /// Found under the current user's Start Menu (not the all-users one).
    pub per_user: bool,
}

/// True when `new` should replace `old` for the same app: a per-user entry
/// wins over an all-users one, then the shorter display name wins. On a
/// full tie the first one found is kept.
fn is_better(new: &Candidate, old: &Candidate) -> bool {
    if new.per_user != old.per_user {
        return new.per_user;
    }
    new.name.chars().count() < old.name.chars().count()
}

/// One Start Menu [`AppEntry`] per [`app_id`]`(target, args)`, in the order
/// each app was first found.
pub fn dedupe(candidates: Vec<Candidate>) -> Vec<AppEntry> {
    let mut order: Vec<(String, Candidate)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for candidate in candidates {
        let id = app_id(&candidate.target, &candidate.args);
        match index.get(&id) {
            Some(&at) => {
                if let Some((_, kept)) = order.get_mut(at) {
                    if is_better(&candidate, kept) {
                        *kept = candidate;
                    }
                }
            }
            None => {
                index.insert(id.clone(), order.len());
                order.push((id, candidate));
            }
        }
    }
    order
        .into_iter()
        .map(|(id, c)| AppEntry {
            id,
            name: c.name,
            target: c.target,
            args: c.args,
            working_dir: c.working_dir,
            source: AppSource::StartMenu,
            lnk_path: Some(c.lnk_path),
            missing: false,
            hidden: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, target: &str, args: &str, per_user: bool) -> Candidate {
        Candidate {
            name: name.into(),
            target: target.into(),
            args: args.into(),
            working_dir: String::new(),
            lnk_path: format!(r"C:\Start\{name}.lnk"),
            per_user,
        }
    }

    #[test]
    fn prefers_the_per_user_entry() {
        let apps = dedupe(vec![
            candidate("App", r"C:\app.exe", "", false),
            candidate("My Long App Name", r"C:\APP.EXE", "", true),
            candidate("A", r"c:\app.exe", "", false),
        ]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "My Long App Name");
        assert_eq!(apps[0].id, app_id(r"C:\app.exe", ""));
        assert_eq!(
            apps[0].lnk_path.as_deref(),
            Some(r"C:\Start\My Long App Name.lnk")
        );
    }

    #[test]
    fn then_prefers_the_shorter_name() {
        let apps = dedupe(vec![
            candidate("App Long", r"C:\app.exe", "", true),
            candidate("App", r"C:\app.exe", "", true),
            candidate("Ap2", r"C:\app.exe", "", true),
        ]);
        assert_eq!(apps.len(), 1);
        // "Ap2" ties with "App" on length; the first one found stays.
        assert_eq!(apps[0].name, "App");
    }

    #[test]
    fn different_args_are_different_apps() {
        let apps = dedupe(vec![
            candidate("App", r"C:\app.exe", "", false),
            candidate("App Safe Mode", r"C:\app.exe", "--safe", false),
            candidate("Other", r"C:\other.exe", "", false),
        ]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["App", "App Safe Mode", "Other"]);
        assert!(apps
            .iter()
            .all(|a| a.source == AppSource::StartMenu && !a.missing && !a.hidden));
    }

    #[test]
    fn empty_input_gives_nothing() {
        assert!(dedupe(Vec::new()).is_empty());
    }
}
