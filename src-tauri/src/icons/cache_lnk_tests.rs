//! Icon cache tests for apps with a shortcut (`lnk_path`).

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::time::{Duration, SystemTime};

use image::{Rgba, RgbaImage};

use super::cache::{ensure_icon_with, icon_path, Ensured};
use super::cache_tests::{app_with_target, pixel, set_mtime, Fake, BLUE, RED};

#[test]
fn the_lnk_is_preferred_over_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_target(dir.path(), SystemTime::now());
    let fake = Fake::new(Some(RED));
    fake.run(&app, dir.path()).unwrap();
    app.lnk_path = Some(r"C:\Start Menu\Test.lnk".into());
    set_mtime(&icon_path(dir.path(), &app.id), SystemTime::UNIX_EPOCH);
    fake.run(&app, dir.path()).unwrap();
    let calls = fake.calls.borrow();
    assert_eq!(calls[0], Path::new(&app.target));
    assert_eq!(calls[1], Path::new(r"C:\Start Menu\Test.lnk"));
    assert_eq!(calls.len(), 2);
}

#[test]
fn a_failed_lnk_falls_back_to_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_target(dir.path(), SystemTime::now());
    app.lnk_path = Some(r"C:\Start Menu\Deleted.lnk".into());
    let calls = RefCell::new(Vec::new());
    let extract = |path: &Path, size: u32| {
        calls.borrow_mut().push(path.to_path_buf());
        if path.extension().is_some_and(|ext| ext == "lnk") {
            Err("no such shortcut".to_owned())
        } else {
            Ok(RgbaImage::from_pixel(size, size, Rgba(BLUE)))
        }
    };
    let png = icon_path(dir.path(), &app.id);
    let ensured = ensure_icon_with(&app, dir.path(), &extract);
    assert_eq!(ensured, Ok(Ensured::Written(png.clone())));
    assert_eq!(pixel(&png), BLUE);
    assert_eq!(calls.borrow()[1], Path::new(&app.target));
}

#[test]
fn a_shortcut_newer_than_the_png_makes_it_stale() {
    let dir = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let mut app = app_with_target(dir.path(), now - Duration::from_secs(3600));
    let lnk = dir.path().join("Test.lnk");
    File::create(&lnk).unwrap().set_modified(now).unwrap();
    app.lnk_path = Some(lnk.display().to_string());
    let png = icon_path(dir.path(), &app.id);
    Fake::new(Some(RED)).run(&app, dir.path()).unwrap();
    // Target older than the PNG, shortcut newer than the PNG.
    set_mtime(&png, now - Duration::from_secs(60));

    let fake = Fake::new(Some(BLUE));
    assert_eq!(
        fake.run(&app, dir.path()),
        Ok(Ensured::Written(png.clone()))
    );
    assert_eq!(*fake.calls.borrow(), vec![lnk]);
    assert_eq!(pixel(&png), BLUE);

    // Once the PNG is newer than both, it is reused again.
    let again = Fake::new(Some(RED));
    assert_eq!(again.run(&app, dir.path()), Ok(Ensured::Fresh(png.clone())));
    assert!(again.calls.borrow().is_empty());
}
