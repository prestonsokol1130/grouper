//! Cleanup against temp folders and a fake registry. The real Run key and
//! Start Menu are never touched.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::autostart_entry::{
    is_grouper_command, remove, Outcome as EntryOutcome, Registry, RUN_KEY, RUN_VALUE_NAME,
    STARTUP_APPROVED_KEY,
};

use super::{run, Targets, APP_IDENTIFIER};
use crate::taskbar::group_icon_files::local_group_icons_dir;

/// A registry in a map of `(key, name) -> data`.
#[derive(Default)]
struct FakeRegistry {
    values: RefCell<HashMap<(String, String), String>>,
    broken: bool,
}

impl FakeRegistry {
    fn with(entries: &[(&str, &str, &str)]) -> Self {
        let fake = Self::default();
        for (key, name, data) in entries {
            fake.values
                .borrow_mut()
                .insert((key.to_string(), name.to_string()), data.to_string());
        }
        fake
    }

    fn has(&self, key: &str, name: &str) -> bool {
        self.values
            .borrow()
            .contains_key(&(key.to_owned(), name.to_owned()))
    }
}

impl Registry for FakeRegistry {
    fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, String> {
        if self.broken {
            return Err("access denied".into());
        }
        let values = self.values.borrow();
        Ok(values.get(&(key.to_owned(), name.to_owned())).cloned())
    }

    fn delete_value(&self, key: &str, name: &str) -> Result<(), String> {
        self.values
            .borrow_mut()
            .remove(&(key.to_owned(), name.to_owned()));
        Ok(())
    }
}

const OURS: &str = r"C:\Users\p\AppData\Local\Grouper\Grouper.exe --background";

pub(super) fn touch(path: &Path) {
    fs::write(path, b"x").unwrap();
}

#[test]
fn recognises_grouper_commands_only() {
    assert!(is_grouper_command(OURS));
    assert!(is_grouper_command(r#""C:\Apps\grouper.exe" --background"#));
    assert!(is_grouper_command(r"D:\dev\target\debug\grouper.exe"));
    assert!(!is_grouper_command(r"C:\Other\NotGrouper.exe --background"));
    assert!(!is_grouper_command(r"C:\Grouper\other.exe"));
    assert!(!is_grouper_command(""));
}

#[test]
fn removes_our_run_value_and_its_task_manager_flag_only() {
    let fake = FakeRegistry::with(&[
        (RUN_KEY, RUN_VALUE_NAME, OURS),
        (STARTUP_APPROVED_KEY, RUN_VALUE_NAME, "flag"),
        (RUN_KEY, "OneDrive", r"C:\OneDrive.exe /background"),
        (STARTUP_APPROVED_KEY, "OneDrive", "flag"),
    ]);
    assert_eq!(remove(&fake), Ok(EntryOutcome::Removed));
    assert!(!fake.has(RUN_KEY, RUN_VALUE_NAME));
    assert!(!fake.has(STARTUP_APPROVED_KEY, RUN_VALUE_NAME));
    assert!(fake.has(RUN_KEY, "OneDrive"));
    assert!(fake.has(STARTUP_APPROVED_KEY, "OneDrive"));
    assert_eq!(remove(&fake), Ok(EntryOutcome::NotFound));
}

#[test]
fn leaves_a_same_named_value_for_another_program() {
    let other = r"C:\Other\Thing.exe";
    let fake = FakeRegistry::with(&[
        (RUN_KEY, RUN_VALUE_NAME, other),
        (STARTUP_APPROVED_KEY, RUN_VALUE_NAME, "flag"),
    ]);
    assert_eq!(remove(&fake), Ok(EntryOutcome::NotOurs(other.into())));
    assert!(fake.has(RUN_KEY, RUN_VALUE_NAME));
    assert!(fake.has(STARTUP_APPROVED_KEY, RUN_VALUE_NAME));
}

/// An unpin that fails, as the Shell does for a group that is not pinned.
pub(super) fn not_pinned(_shortcut: &Path) -> Result<(), String> {
    Err("not pinned".into())
}

/// A programs folder with `Grouper\Work.lnk`, an app data folder with
/// `app_state.json` and a group icon folder with one `.ico`, under one
/// temp root.
fn installed() -> (tempfile::TempDir, Targets) {
    let root = tempfile::tempdir().unwrap();
    let programs = root.path().join("Programs");
    fs::create_dir_all(programs.join("Grouper")).unwrap();
    touch(&programs.join("Grouper").join("Work.lnk"));
    let data = root.path().join(APP_IDENTIFIER);
    fs::create_dir_all(&data).unwrap();
    touch(&data.join("app_state.json"));
    let icons = local_group_icons_dir(&root.path().join("Local"));
    fs::create_dir_all(&icons).unwrap();
    touch(&icons.join("g1-v1.ico"));
    let targets = Targets {
        programs_dir: Ok(programs),
        app_data_dir: Ok(data),
        group_icons_dir: Ok(icons),
    };
    (root, targets)
}

#[test]
fn run_keeps_app_data_unless_asked() {
    let (_root, targets) = installed();
    let fake = FakeRegistry::with(&[(RUN_KEY, RUN_VALUE_NAME, OURS)]);
    assert!(run(&targets, &fake, &not_pinned, false).is_empty());
    let programs = targets.programs_dir.as_ref().unwrap();
    let data = targets.app_data_dir.as_ref().unwrap();
    assert!(!programs.join("Grouper").exists());
    assert!(!targets.group_icons_dir.as_ref().unwrap().exists());
    assert!(!fake.has(RUN_KEY, RUN_VALUE_NAME));
    assert!(data.join("app_state.json").exists());
    assert!(run(&targets, &fake, &not_pinned, true).is_empty());
    assert!(!data.exists());
}

#[test]
fn run_goes_on_after_a_failed_step() {
    let (_root, mut targets) = installed();
    targets.programs_dir = Err("no Start Menu".into());
    targets.group_icons_dir = Err("no Local AppData".into());
    let fake = FakeRegistry {
        broken: true,
        ..FakeRegistry::default()
    };
    let errors = run(&targets, &fake, &not_pinned, true);
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert!(!targets.app_data_dir.as_ref().unwrap().exists());
}

#[test]
fn identifier_matches_tauri_conf() {
    let conf = include_str!("../../tauri.conf.json");
    assert!(conf.contains(&format!("\"identifier\": \"{APP_IDENTIFIER}\"")));
    assert!(conf.contains("\"productName\": \"Grouper\""));
}
