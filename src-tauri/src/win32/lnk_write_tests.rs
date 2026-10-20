//! Windows-only tests for `write_group_lnk` and `write_group_shortcut`:
//! write real shortcuts, read them back with `resolve_lnk`, and read the
//! AppUserModelID back through `IPropertyStore::GetValue`.

use std::path::{Path, PathBuf};

use windows::core::{Interface, PCWSTR};
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::{
    CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

use super::com::ComGuard;
use super::lnk_read::resolve_lnk;
use super::lnk_write::{write_group_lnk, LnkSpec};
use super::wide::{from_wide, to_wide};
use crate::state::{AppState, Group};
use crate::taskbar::shortcut::write_group_shortcut;

/// The AppUserModelID (which must be a `VT_LPWSTR`) and the description
/// stored in the shortcut at `lnk`. The caller must hold a `ComGuard`.
fn read_aumid_and_description(lnk: &Path) -> (String, String) {
    // SAFETY: the caller holds a ComGuard, so COM is initialised here.
    let link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.unwrap();
    let file: IPersistFile = link.cast().unwrap();
    let wide = to_wide(lnk.to_str().unwrap());
    // SAFETY: wide is NUL-terminated and outlives the call.
    unsafe { file.Load(PCWSTR(wide.as_ptr()), STGM_READ) }.unwrap();
    let store: IPropertyStore = link.cast().unwrap();
    // SAFETY: the key is a live constant; the returned PROPVARIANT is owned
    // here and cleared (PropVariantClear) by its Drop at the end of scope.
    let value = unsafe { store.GetValue(&PKEY_AppUserModel_ID) }.unwrap();
    assert_eq!(value.vt(), VT_LPWSTR);
    // SAFETY: vt is VT_LPWSTR (checked above), so pwszVal is the live,
    // NUL-terminated string the PROPVARIANT owns.
    let aumid = unsafe { value.Anonymous.Anonymous.Anonymous.pwszVal.to_string() }.unwrap();
    let mut buf = vec![0_u16; 1024];
    // SAFETY: buf is a valid, writable buffer.
    unsafe { link.GetDescription(&mut buf) }.unwrap();
    (aumid, from_wide(&buf))
}

/// This test binary: a real file whose path has spaces on Preston's PC.
fn exe() -> PathBuf {
    std::env::current_exe().unwrap()
}

fn spec(dir: &Path, args: &str) -> LnkSpec {
    LnkSpec {
        lnk_path: dir.join("My Group.lnk"),
        exe_path: exe(),
        args: args.to_owned(),
        icon_path: dir.join("icons dir").join("grp-v3.ico"),
        description: "My Group".to_owned(),
        aumid: "com.grouper.group.aB3_-x".to_owned(),
    }
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn writes_a_shortcut_that_reads_back() {
    let _com = ComGuard::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("Start Menu with spaces");
    std::fs::create_dir_all(&dir).unwrap();
    let spec = spec(&dir, "--flyout aB3_-x");
    write_group_lnk(&spec).unwrap();

    let resolved = resolve_lnk(&spec.lnk_path).unwrap();
    let exe = exe();
    assert!(
        resolved.target.eq_ignore_ascii_case(exe.to_str().unwrap()),
        "target was {:?}",
        resolved.target
    );
    assert_eq!(resolved.args, "--flyout aB3_-x");
    assert!(resolved
        .working_dir
        .eq_ignore_ascii_case(exe.parent().unwrap().to_str().unwrap()));
    let icon = spec.icon_path.to_str().unwrap().to_owned();
    assert_eq!(resolved.icon_location, Some((icon, 0)));

    let (aumid, description) = read_aumid_and_description(&spec.lnk_path);
    assert_eq!(aumid, spec.aumid);
    assert_eq!(description, "My Group");
    // Only the shortcut is left: the temp file was renamed over it.
    assert_eq!(file_names(&dir), ["My Group.lnk"]);
}

#[test]
fn rewriting_replaces_the_shortcut() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_group_lnk(&spec(dir.path(), "--flyout old")).unwrap();
    let spec = spec(dir.path(), "--flyout new");
    write_group_lnk(&spec).unwrap();
    assert_eq!(resolve_lnk(&spec.lnk_path).unwrap().args, "--flyout new");
    assert_eq!(file_names(dir.path()), ["My Group.lnk"]);
}

#[test]
fn a_failed_write_leaves_no_files() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut spec = spec(&dir.path().join("missing folder"), "--flyout x");
    spec.aumid = "com.grouper.group.x".to_owned();
    assert!(write_group_lnk(&spec).is_err());
    assert!(file_names(dir.path()).is_empty());
}

#[test]
fn group_shortcut_has_the_flyout_route_icon_and_aumid() {
    let _com = ComGuard::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("app data").join("cache");
    let start_menu = temp.path().join("Programs").join("Grouper");
    std::fs::create_dir_all(&start_menu).unwrap();
    let state = AppState {
        groups: vec![Group {
            id: "aB3_-x".to_owned(),
            name: "Work: Tools?".to_owned(),
            app_ids: Vec::new(),
            icon_version: 7,
        }],
        ..AppState::default()
    };

    let lnk = write_group_shortcut(&state, "aB3_-x", &cache, &start_menu, &exe()).unwrap();
    assert_eq!(lnk, start_menu.join("Work Tools.lnk"));
    let resolved = resolve_lnk(&lnk).unwrap();
    assert_eq!(resolved.args, "--flyout aB3_-x");
    let icon = cache.join("groups").join("aB3_-x-v7.ico");
    assert!(icon.is_file());
    assert_eq!(
        resolved.icon_location,
        Some((icon.to_str().unwrap().to_owned(), 0))
    );
    let (aumid, description) = read_aumid_and_description(&lnk);
    assert_eq!(aumid, "com.grouper.group.aB3_-x");
    assert_eq!(description, "Work: Tools?");

    let unknown = write_group_shortcut(&state, "nope", &cache, &start_menu, &exe());
    assert_eq!(unknown, Err("Group not found: nope".to_owned()));
}
