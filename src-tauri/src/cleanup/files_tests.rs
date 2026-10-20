//! Removing the Start Menu folder and the app data, against temp folders.

use std::cell::RefCell;
use std::fs;
use std::path::Path;

use super::files::{remove_app_data, remove_start_menu_folder, Outcome};
use super::tests::{not_pinned, touch};
use super::APP_IDENTIFIER;
#[test]
fn each_shortcut_is_unpinned_before_it_goes() {
    let programs = tempfile::tempdir().unwrap();
    let own = programs.path().join("Grouper");
    fs::create_dir(&own).unwrap();
    touch(&own.join("Work.lnk"));
    touch(&own.join("notes.txt"));
    let seen = RefCell::new(Vec::new());
    let record = |path: &Path| {
        assert!(path.exists(), "unpinned after delete");
        seen.borrow_mut().push(path.file_name().unwrap().to_owned());
        Ok(())
    };
    remove_start_menu_folder(programs.path(), &record).unwrap();
    assert_eq!(
        seen.into_inner(),
        vec![std::ffi::OsString::from("Work.lnk")]
    );
}

#[test]
fn start_menu_folder_and_its_shortcuts_go() {
    let programs = tempfile::tempdir().unwrap();
    let own = programs.path().join("Grouper");
    fs::create_dir(&own).unwrap();
    touch(&own.join("Work.lnk"));
    touch(&own.join("Games (2).LNK"));
    touch(&programs.path().join("Other.lnk"));
    assert_eq!(
        remove_start_menu_folder(programs.path(), &not_pinned),
        Ok(Outcome::Removed)
    );
    assert!(!own.exists());
    assert!(programs.path().join("Other.lnk").exists());
    assert_eq!(
        remove_start_menu_folder(programs.path(), &not_pinned),
        Ok(Outcome::NotFound)
    );
}

#[test]
fn start_menu_folder_with_other_things_is_kept() {
    let programs = tempfile::tempdir().unwrap();
    let own = programs.path().join("Grouper");
    fs::create_dir_all(own.join("sub")).unwrap();
    touch(&own.join("Work.lnk"));
    touch(&own.join("notes.txt"));
    let kept = vec!["notes.txt".to_owned(), "sub".to_owned()];
    assert_eq!(
        remove_start_menu_folder(programs.path(), &not_pinned),
        Ok(Outcome::Kept(kept))
    );
    assert!(!own.join("Work.lnk").exists());
    assert!(own.join("notes.txt").exists());
}

/// A directory symlink, or `None` when this account may not make one.
fn dir_link(target: &Path, link: &Path) -> Option<()> {
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(target, link);
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(target, link);
    made.ok()
}

#[test]
fn symlinked_folders_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = root.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    touch(&elsewhere.join("precious.lnk"));
    let programs = root.path().join("programs");
    fs::create_dir(&programs).unwrap();
    if dir_link(&elsewhere, &programs.join("Grouper")).is_none() {
        eprintln!("skipping: no symlink privilege");
        return;
    }
    assert!(remove_start_menu_folder(&programs, &not_pinned).is_err());
    let data_link = root.path().join(APP_IDENTIFIER);
    dir_link(&elsewhere, &data_link).unwrap();
    assert!(remove_app_data(&data_link, APP_IDENTIFIER).is_err());
    assert!(elsewhere.join("precious.lnk").exists());
}

#[test]
fn app_data_is_deleted_only_under_its_own_name() {
    let root = tempfile::tempdir().unwrap();
    let wrong = root.path().join("some.other.app");
    fs::create_dir(&wrong).unwrap();
    assert!(remove_app_data(&wrong, APP_IDENTIFIER).is_err());
    assert!(wrong.exists());
    let data = root.path().join(APP_IDENTIFIER);
    fs::create_dir_all(data.join("cache").join("groups")).unwrap();
    touch(&data.join("app_state.json"));
    assert_eq!(remove_app_data(&data, APP_IDENTIFIER), Ok(Outcome::Removed));
    assert!(!data.exists());
    assert_eq!(
        remove_app_data(&data, APP_IDENTIFIER),
        Ok(Outcome::NotFound)
    );
}
