//! Table tests for the shortcut filter.

use super::filter::{check, is_file, Rejection};

const APP: &str = r"C:\Program Files\App\app.exe";

/// One row: shortcut name, target, and the expected outcome.
type Case = (&'static str, &'static str, Result<(), Rejection>);

fn cases() -> Vec<Case> {
    use Rejection::*;
    vec![
        // Kept: real apps.
        ("Notepad", r"C:\Windows\System32\notepad.exe", Ok(())),
        ("Visual Studio Code", APP, Ok(())),
        (
            "Computer Management",
            r"C:\Windows\System32\compmgmt.msc",
            Ok(()),
        ),
        ("Services", r"%windir%\system32\services.MSC", Ok(())),
        // Whole words only: these contain a reject word inside a longer one.
        ("SupportAssist", APP, Ok(())),
        ("Helper Tool", APP, Ok(())),
        ("Setupper", APP, Ok(())),
        ("Manually Run", APP, Ok(())),
        ("Remover Pro", APP, Ok(())),
        ("Licenses Manager", APP, Ok(())),
        ("Whats New", APP, Ok(())),
        ("Readmeter", APP, Ok(())),
        ("App", r"C:\Apps\helpdesk.exe", Ok(())),
        // Rejected by the shortcut name, in any letter case.
        ("Uninstall App", APP, Err(NameWord("uninstall"))),
        ("UNINSTALL", APP, Err(NameWord("uninstall"))),
        ("App Uninst", APP, Err(NameWord("uninst"))),
        ("unins000", APP, Err(NameWord("unins000"))),
        ("Remove App", APP, Err(NameWord("remove"))),
        ("App Setup", APP, Err(NameWord("setup"))),
        ("Read Me", APP, Err(NameWord("read me"))),
        ("README", APP, Err(NameWord("readme"))),
        ("App Help", APP, Err(NameWord("help"))),
        ("Documentation", APP, Err(NameWord("documentation"))),
        ("User Manual", APP, Err(NameWord("manual"))),
        ("License", APP, Err(NameWord("license"))),
        ("Release Notes", APP, Err(NameWord("release notes"))),
        ("What's New", APP, Err(NameWord("what's new"))),
        ("What\u{2019}s New in App", APP, Err(NameWord("what's new"))),
        ("App Website", APP, Err(NameWord("website"))),
        ("Help and Support", APP, Err(NameWord("help"))),
        ("Contact Support", APP, Err(NameWord("support"))),
        ("App-Setup (x64)", APP, Err(NameWord("setup"))),
        // Rejected by the target's file name.
        ("App", r"C:\Apps\unins000.exe", Err(TargetWord("unins000"))),
        (
            "App",
            r"C:\Apps\Uninstall.exe",
            Err(TargetWord("uninstall")),
        ),
        ("App", r"C:\Apps\uninst.exe", Err(TargetWord("uninst"))),
        ("App", r"C:\Apps\setup.exe", Err(TargetWord("setup"))),
        ("App", r"C:\Apps\readme.md", Err(TargetWord("readme"))),
        ("App", r"C:\Apps\help\app.exe", Ok(())),
        // Rejected by the target's extension.
        ("App Site", r"C:\Apps\site.url", Err(Extension(".url"))),
        ("Guide", r"C:\Apps\guide.HTML", Err(Extension(".html"))),
        ("Guide", r"C:\Apps\guide.htm", Err(Extension(".htm"))),
        ("Notes", r"C:\Apps\notes.txt", Err(Extension(".txt"))),
        ("Book", r"C:\Apps\book.pdf", Err(Extension(".pdf"))),
        ("Reference", r"C:\Apps\ref.chm", Err(Extension(".chm"))),
        // Rejected for an empty target.
        ("Calculator", "", Err(EmptyTarget)),
        ("Calculator", "   ", Err(EmptyTarget)),
    ]
}

#[test]
fn filter_table() {
    let cases = cases();
    assert!(cases.len() >= 30);
    for (name, target, expected) in cases {
        assert_eq!(
            check(name, target, |_| true),
            expected,
            "name {name:?}, target {target:?}"
        );
    }
}

#[test]
fn only_existing_files_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("App");
    std::fs::create_dir(&folder).unwrap();
    let exe = folder.join("app.exe");
    std::fs::write(&exe, b"").unwrap();
    let folder = folder.to_str().unwrap();
    let exe = exe.to_str().unwrap();
    let absent = dir.path().join("gone.exe");
    let absent = absent.to_str().unwrap();
    assert_eq!(check("App", exe, is_file), Ok(()));
    // A shortcut to an install folder is not an app.
    assert_eq!(check("App", folder, is_file), Err(Rejection::NotAFile));
    assert_eq!(check("App", absent, is_file), Err(Rejection::NotAFile));
}

#[test]
fn network_targets_are_kept_without_touching_the_disk() {
    // "server" does not exist; a real lookup would fail or hang, so these
    // passing shows the file system was never asked.
    assert!(is_file(r"\\server\share\app.exe"));
    assert!(is_file("//server/share/app.exe"));
    assert_eq!(check("App", r"\\server\share\app.exe", is_file), Ok(()));
    assert_eq!(check("App", "//server/share/app.exe", is_file), Ok(()));
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("gone.exe");
    assert!(!is_file(absent.to_str().unwrap()));
}

#[test]
fn file_check_runs_last_and_gets_the_target() {
    let mut called = false;
    let outcome = check("Uninstall", APP, |_| {
        called = true;
        true
    });
    assert_eq!(outcome, Err(Rejection::NameWord("uninstall")));
    assert!(!called);
    let mut seen = String::new();
    assert_eq!(
        check("App", APP, |t| {
            seen = t.to_owned();
            true
        }),
        Ok(())
    );
    assert_eq!(seen, APP);
}

#[test]
fn rejections_explain_themselves() {
    assert_eq!(
        Rejection::NameWord("setup").to_string(),
        "name contains \"setup\""
    );
    assert_eq!(
        Rejection::Extension(".url").to_string(),
        "target is a .url file"
    );
    assert_eq!(Rejection::NotAFile.to_string(), "target is not a file");
}
