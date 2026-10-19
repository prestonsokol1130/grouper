//! Windows-only tests for `ComGuard` and `resolve_lnk` against real `.lnk`
//! files written through `IShellLinkW` and `IPersistFile::Save`.

use std::path::Path;

use windows::core::{Interface, PCWSTR};
use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

use super::com::ComGuard;
use super::lnk_read::{resolve_lnk, ResolvedLink};
use super::wide::to_wide;

const NOTEPAD: &str = r"C:\Windows\System32\notepad.exe";

/// Writes a shortcut at `lnk` to notepad with the given arguments, working
/// directory and icon. The caller must hold a `ComGuard`.
fn write_lnk(lnk: &Path, args: &str, working_dir: &str, icon: Option<(&str, i32)>) {
    write_lnk_to(lnk, NOTEPAD, args, working_dir, icon);
}

/// Like `write_lnk`, but with any target path.
fn write_lnk_to(
    lnk: &Path,
    target: &str,
    args: &str,
    working_dir: &str,
    icon: Option<(&str, i32)>,
) {
    // SAFETY: the caller holds a ComGuard, so COM is initialised on this
    // thread.
    let link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.unwrap();
    let notepad = to_wide(target);
    let args = to_wide(args);
    let working_dir = to_wide(working_dir);
    // SAFETY: SetPath, SetArguments and SetWorkingDirectory each get a
    // NUL-terminated wide buffer that outlives the call.
    unsafe {
        link.SetPath(PCWSTR(notepad.as_ptr())).unwrap();
        link.SetArguments(PCWSTR(args.as_ptr())).unwrap();
        link.SetWorkingDirectory(PCWSTR(working_dir.as_ptr()))
            .unwrap();
    }
    if let Some((icon_path, index)) = icon {
        let icon_path = to_wide(icon_path);
        // SAFETY: icon_path is NUL-terminated and outlives the call.
        unsafe { link.SetIconLocation(PCWSTR(icon_path.as_ptr()), index) }.unwrap();
    }
    let file: IPersistFile = link.cast().unwrap();
    let lnk_wide = to_wide(lnk.to_str().unwrap());
    // SAFETY: lnk_wide is NUL-terminated and outlives the call; fRemember
    // TRUE only makes the saved file the link's current file.
    unsafe { file.Save(PCWSTR(lnk_wide.as_ptr()), true) }.unwrap();
}

#[test]
fn resolves_a_saved_shortcut() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let lnk = dir.path().join("Notepad.lnk");
    write_lnk(
        &lnk,
        r#"--flag "C:\some file.txt""#,
        r"C:\Windows",
        Some((r"C:\Windows\System32\shell32.dll", 3)),
    );

    let resolved = resolve_lnk(&lnk).unwrap();
    assert!(
        resolved.target.eq_ignore_ascii_case(NOTEPAD),
        "target was {:?}",
        resolved.target
    );
    assert_eq!(
        resolved,
        ResolvedLink {
            target: resolved.target.clone(),
            args: r#"--flag "C:\some file.txt""#.to_owned(),
            working_dir: r"C:\Windows".to_owned(),
            icon_location: Some((r"C:\Windows\System32\shell32.dll".to_owned(), 3)),
        }
    );
}

#[test]
fn shortcut_without_icon_or_args_has_empty_fields() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let lnk = dir.path().join("Plain.lnk");
    write_lnk(&lnk, "", "", None);

    let resolved = resolve_lnk(&lnk).unwrap();
    assert!(resolved.target.eq_ignore_ascii_case(NOTEPAD));
    assert_eq!(resolved.args, "");
    assert_eq!(resolved.working_dir, "");
    assert_eq!(resolved.icon_location, None);
}

#[test]
fn target_environment_variables_are_expanded() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let lnk = dir.path().join("Env.lnk");
    write_lnk_to(&lnk, r"%SystemRoot%\System32\notepad.exe", "", "", None);

    let target = resolve_lnk(&lnk).unwrap().target;
    let root = std::env::var("SystemRoot").unwrap();
    assert!(!target.contains('%'), "target was {target:?}");
    assert!(
        target.eq_ignore_ascii_case(&format!(r"{root}\System32\notepad.exe")),
        "target was {target:?}"
    );
}

#[test]
fn missing_file_is_an_error() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_lnk(&dir.path().join("does-not-exist.lnk"));
    assert!(result.is_err(), "got {result:?}");
}

#[test]
fn non_shortcut_file_is_an_error() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("fake.lnk");
    std::fs::write(&fake, b"not a shortcut").unwrap();
    assert!(resolve_lnk(&fake).is_err());
}

#[test]
fn nested_guards_on_one_thread_work() {
    // A fresh thread, so the test harness thread's COM state cannot matter.
    std::thread::spawn(|| {
        let dir = tempfile::tempdir().unwrap();
        let lnk = dir.path().join("Nested.lnk");
        let outer = ComGuard::new().unwrap();
        assert!(outer.uninitializes_on_drop());
        {
            let inner = ComGuard::new().unwrap();
            // S_FALSE: already initialised, and still owes a CoUninitialize.
            assert!(inner.uninitializes_on_drop());
            write_lnk(&lnk, "inner", "", None);
            assert_eq!(resolve_lnk(&lnk).unwrap().args, "inner");
        }
        // Dropping the inner guard must leave COM usable for the outer one.
        assert_eq!(resolve_lnk(&lnk).unwrap().args, "inner");
        drop(outer);
        // Both inits were balanced, so a new guard starts cleanly again.
        let again = ComGuard::new().unwrap();
        assert!(again.uninitializes_on_drop());
        assert_eq!(resolve_lnk(&lnk).unwrap().args, "inner");
    })
    .join()
    .unwrap();
}
