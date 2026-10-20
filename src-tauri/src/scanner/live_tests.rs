//! Live checks against this PC's Start menu. Ignored by default; run with
//! `cargo test --lib live_ -- --ignored --nocapture`. Set
//! `GROUPER_LIVE_OUT` to a folder to also save every found app's name and
//! the icons of the Applications-folder apps, raw and as cached, plus a
//! sample 2x2 group icon.

use std::path::Path;

use image::RgbaImage;

use super::scan;
use crate::icons::cache::{ensure_icon_with, ICON_SIZE};
use crate::icons::extract::extract_icon;
use crate::state::AppEntry;
use crate::taskbar::compose::compose_folder;
use crate::taskbar::style::FOLDER_STYLE;
use crate::win32::com::ComGuard;
use crate::win32::launch::target_exists;

#[test]
#[ignore = "reads this PC's Start menu"]
fn live_scan_report() {
    let outcome = scan().unwrap();
    let (folder, lnk): (Vec<_>, Vec<_>) = outcome
        .apps
        .iter()
        .partition(|app| app.apps_folder_id().is_some());
    println!("complete: {}", outcome.complete);
    println!("from shortcuts: {}", lnk.len());
    println!("from the Applications folder: {}", folder.len());
    println!("total: {}", outcome.apps.len());
    let _com = ComGuard::new().unwrap();
    for name in ["Claude", "ChatGPT"] {
        let app = find(&folder, name);
        println!("{name}: {} (id {})", app.target, app.id);
        assert!(target_exists(Path::new(&app.target)), "{name}");
    }
    assert!(!target_exists(Path::new(
        r"shell:AppsFolder\NoSuch.App_0000000000000!App"
    )));
    let Ok(out) = std::env::var("GROUPER_LIVE_OUT") else {
        return;
    };
    let out = Path::new(&out);
    let lines: Vec<_> = outcome
        .apps
        .iter()
        .map(|app| format!("{}\t{}", app.name, app.target))
        .collect();
    std::fs::write(out.join("found-apps.tsv"), lines.join("\n")).unwrap();
    let cache = out.join("icons");
    for app in &folder {
        let made = ensure_icon_with(app, &cache, &extract_icon).unwrap();
        println!("  icon {}: {made:?}", app.name);
    }
    let mut group = Vec::new();
    for name in ["Claude", "ChatGPT", "Calculator"] {
        let app = find(&folder, name);
        let raw = extract_icon(Path::new(&app.target), ICON_SIZE).unwrap();
        raw.save(out.join(format!("{name}-raw.png"))).unwrap();
        group.push(load(&cache.join(format!("{}.png", app.id))));
    }
    let desktop = lnk.iter().find(|app| app.name.contains("Chrome")).unwrap();
    group.push(extract_icon(Path::new(&desktop.target), ICON_SIZE).unwrap());
    let composed = compose_folder(&group, &FOLDER_STYLE);
    composed.save(out.join("group-2x2.png")).unwrap();
}

/// Starts Windows Calculator the way Grouper starts any Store app. Close
/// it afterwards.
#[test]
#[ignore = "opens Calculator"]
fn live_launch_calculator() {
    let calculator = AppEntry {
        name: "Calculator".into(),
        target: crate::state::apps_folder_target("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
        ..AppEntry::default()
    };
    crate::win32::launch::launch(&calculator).unwrap();
}

fn find<'a>(apps: &[&'a AppEntry], name: &str) -> &'a AppEntry {
    apps.iter().find(|app| app.name == name).unwrap()
}

fn load(path: &Path) -> RgbaImage {
    image::open(path).unwrap().into_rgba8()
}
