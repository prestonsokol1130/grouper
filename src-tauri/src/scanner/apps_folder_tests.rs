//! Tests for merging Applications-folder items into the shortcut scan.

use super::apps_folder::{check, folder_apps, is_file_path, Known};
use super::filter::Rejection;
use super::merge::merge_scan;
use super::ScanOutcome;
use crate::state::ids::{app_id, apps_folder_target};
use crate::state::{AppEntry, AppSource, AppState};
use crate::win32::apps_folder::AppsFolderItem;

const CLAUDE: &str = "Claude_pzs8sxrjxfjjc!Claude";
const CHATGPT: &str = "OpenAI.Codex_2p2nqsd0c76g0!App";

/// A Store app: an AppUserModelID and no file target.
fn store(name: &str, aumid: &str) -> AppsFolderItem {
    AppsFolderItem {
        name: name.into(),
        parsing_name: aumid.into(),
        aumid: Some(aumid.into()),
        target_path: None,
    }
}

/// A desktop shortcut's item, which points at a file.
fn desktop(name: &str, parsing_name: &str, target: &str) -> AppsFolderItem {
    AppsFolderItem {
        target_path: Some(target.into()),
        ..store(name, parsing_name)
    }
}

fn known(name: &str, target: &str, aumid: Option<&str>) -> Known {
    Known {
        name: name.into(),
        target: target.into(),
        aumid: aumid.map(str::to_owned),
    }
}

fn names(apps: &[AppEntry]) -> Vec<&str> {
    apps.iter().map(|app| app.name.as_str()).collect()
}

#[test]
fn store_apps_become_new_entries_with_stable_ids() {
    let found = folder_apps(
        &[],
        vec![store("Claude", CLAUDE), store("ChatGPT", CHATGPT)],
        |_| true,
    );
    assert_eq!(names(&found.apps), ["Claude", "ChatGPT"]);
    let claude = &found.apps[0];
    assert_eq!(claude.target, apps_folder_target(CLAUDE));
    assert_eq!(claude.id, app_id(&apps_folder_target(CLAUDE), ""));
    assert_eq!(claude.apps_folder_id(), Some(CLAUDE));
    assert_eq!(claude.source, AppSource::StartMenu);
    assert_eq!(claude.lnk_path, None);
    // The same AppUserModelID gives the same id on every scan.
    let again = folder_apps(&[], vec![store("Claude (renamed)", CLAUDE)], |_| true);
    assert_eq!(again.apps[0].id, claude.id);
}

#[test]
fn shortcut_apps_win_by_target_path() {
    let lnk = known("Chrome", r"C:\Program Files\Google\Chrome\chrome.exe", None);
    let item = desktop(
        "Google Chrome",
        "Chrome",
        r"c:\program files\google\chrome\CHROME.EXE",
    );
    let found = folder_apps(&[lnk], vec![item], |_| true);
    assert!(found.apps.is_empty());
    assert_eq!(found.duplicates, 1);
}

#[test]
fn then_by_aumid_then_by_name_for_items_without_a_file() {
    let lnks = [
        known("Some Shortcut", "", Some("Vendor.App_123!App")),
        known("Slack", r"C:\Slack\slack.exe", None),
    ];
    let items = vec![
        store("Vendor App", "Vendor.App_123!App"),
        store("  slack ", "91750D7E.Slack_8she8kybcnzg4!Slack"),
        store("Claude", CLAUDE),
    ];
    let found = folder_apps(&lnks, items, |_| true);
    assert_eq!(names(&found.apps), ["Claude"]);
    assert_eq!(found.duplicates, 2);
}

#[test]
fn a_desktop_item_is_not_matched_by_name() {
    // Same name, different program: both are kept.
    let lnk = known("Python", r"C:\Python312\python.exe", None);
    let item = desktop(
        "Python",
        r"{GUID}\Python314\python.exe",
        r"C:\Python314\python.exe",
    );
    let found = folder_apps(&[lnk], vec![item], |_| true);
    assert_eq!(names(&found.apps), ["Python"]);
    assert_eq!(
        found.apps[0].target,
        r"shell:AppsFolder\{GUID}\Python314\python.exe"
    );
}

#[test]
fn rejects_what_is_not_an_app() {
    let items = vec![
        store("Uninstall Tool", "Vendor.Tool_1!Uninstall"),
        desktop(
            "Node.js website",
            "https://nodejs.org/",
            "https://nodejs.org/",
        ),
        desktop("Help", r"{G}\app.chm", r"C:\App\app.chm"),
        desktop("Docs", r"{G}\docs.url", r"C:\App\docs.url"),
        desktop("Gone", r"{G}\gone.exe", r"C:\App\gone.exe"),
        store("AI", "com.grouper.group.abc123"),
        desktop(
            "Cyberpunk 2077",
            "steam://rungameid/1091500",
            "steam://rungameid/1091500",
        ),
        desktop(
            "File Explorer",
            "Microsoft.Windows.Explorer",
            "::{52205FD8}",
        ),
    ];
    let found = folder_apps(&[], items, |path| !path.contains("gone"));
    assert_eq!(names(&found.apps), ["Cyberpunk 2077", "File Explorer"]);
    assert_eq!(found.rejected, 6);
}

#[test]
fn check_gives_the_reason() {
    let web = desktop("Site", "http://x.test/", "http://x.test/");
    assert_eq!(check(&web, |_| true), Err(Rejection::WebLink));
    let own = store("AI", "COM.GROUPER.group.x");
    assert_eq!(check(&own, |_| true), Err(Rejection::OwnShortcut));
    let readme = store("Read Me", "Vendor.App_1!Readme");
    assert_eq!(
        check(&readme, |_| true),
        Err(Rejection::NameWord("read me"))
    );
    assert_eq!(check(&store("Claude", CLAUDE), |_| false), Ok(()));
}

#[test]
fn the_same_item_twice_is_kept_once() {
    let items = vec![store("Claude", CLAUDE), store("Claude", CLAUDE)];
    let found = folder_apps(&[], items, |_| true);
    assert_eq!(found.apps.len(), 1);
    assert_eq!(found.duplicates, 1);
}

#[test]
fn saved_store_apps_keep_their_id_and_go_missing_when_uninstalled() {
    let full = |apps| ScanOutcome {
        apps,
        complete: true,
        unreadable: Vec::new(),
    };
    let mut state = AppState::default();
    let first = folder_apps(
        &[],
        vec![store("Claude", CLAUDE), store("ChatGPT", CHATGPT)],
        |_| true,
    );
    let ids: Vec<_> = first.apps.iter().map(|app| app.id.clone()).collect();
    assert_eq!(merge_scan(&mut state, full(first.apps)).added, 2);
    let again = folder_apps(&[], vec![store("Claude", CLAUDE)], |_| true);
    let summary = merge_scan(&mut state, full(again.apps));
    assert_eq!((summary.added, summary.missing), (0, 1));
    let saved: Vec<_> = state.apps.iter().map(|app| app.id.clone()).collect();
    assert_eq!(saved, ids);
    assert!(!state.apps[0].missing && state.apps[1].missing);
}

#[test]
fn file_paths_are_drives_and_shares_only() {
    assert!(is_file_path(r"C:\a.exe"));
    assert!(is_file_path(r"\\server\share\a.exe"));
    assert!(!is_file_path("steam://rungameid/1"));
    assert!(!is_file_path("::{52205FD8}"));
    assert!(!is_file_path(CLAUDE));
}
