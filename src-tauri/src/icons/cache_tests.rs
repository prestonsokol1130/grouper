//! Tests for the icon cache, with a fake extractor so they run on any OS.

use std::cell::RefCell;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use image::{Rgba, RgbaImage};

use super::cache::{ensure_fallback, ensure_icon_with, fallback_path, icon_path, Ensured};
use super::cache::{Extractor, ICON_SIZE};
use crate::state::AppEntry;

/// A fake extractor that records every path it is asked for and returns a
/// solid image of `colour` (or `Err` when `colour` is None).
pub(super) struct Fake {
    pub(super) calls: RefCell<Vec<PathBuf>>,
    colour: Option<[u8; 4]>,
}

impl Fake {
    pub(super) fn new(colour: Option<[u8; 4]>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            colour,
        }
    }

    pub(super) fn run(&self, app: &AppEntry, dir: &Path) -> Result<Ensured, String> {
        let extract = |path: &Path, size: u32| {
            self.calls.borrow_mut().push(path.to_path_buf());
            self.colour
                .map(|c| RgbaImage::from_pixel(size, size, Rgba(c)))
                .ok_or_else(|| "no icon".to_owned())
        };
        let extract: Extractor<'_> = &extract;
        ensure_icon_with(app, dir, extract)
    }
}

/// An app whose target is a real file in `dir`, with mtime `target_at`.
pub(super) fn app_with_target(dir: &Path, target_at: SystemTime) -> AppEntry {
    let target = dir.join("app.exe");
    let file = File::create(&target).unwrap();
    file.set_modified(target_at).unwrap();
    AppEntry {
        id: "0123456789abcdef".into(),
        name: "Test".into(),
        target: target.display().to_string(),
        ..AppEntry::default()
    }
}

pub(super) fn set_mtime(path: &Path, at: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(at)
        .unwrap();
}

pub(super) fn pixel(path: &Path) -> [u8; 4] {
    let image = image::open(path).unwrap().into_rgba8();
    assert_eq!(image.dimensions(), (ICON_SIZE, ICON_SIZE));
    // The centre: written icons are normalised to a padded square.
    image.get_pixel(ICON_SIZE / 2, ICON_SIZE / 2).0
}

pub(super) const RED: [u8; 4] = [255, 0, 0, 255];
pub(super) const BLUE: [u8; 4] = [0, 0, 255, 255];

#[test]
fn icon_path_is_app_id_png_in_the_cache_dir() {
    let dir = Path::new("cache").join("icons");
    assert_eq!(icon_path(&dir, "abc"), dir.join("abc.png"));
    assert_eq!(fallback_path(&dir), dir.join("_fallback.png"));
}

#[test]
fn a_fresh_cache_file_is_reused() {
    let dir = tempfile::tempdir().unwrap();
    let old = SystemTime::now() - Duration::from_secs(3600);
    let app = app_with_target(dir.path(), old);
    let first = Fake::new(Some(RED));
    let png = icon_path(dir.path(), &app.id);
    assert_eq!(
        first.run(&app, dir.path()),
        Ok(Ensured::Written(png.clone()))
    );
    assert_eq!(pixel(&png), RED);
    // Normalised: the full-bleed fake gets the 5% padding.
    let written = image::open(&png).unwrap().into_rgba8();
    assert_eq!(written.get_pixel(2, 2)[3], 0);

    let second = Fake::new(Some(BLUE));
    assert_eq!(
        second.run(&app, dir.path()),
        Ok(Ensured::Fresh(png.clone()))
    );
    assert!(second.calls.borrow().is_empty());
    assert_eq!(pixel(&png), RED);
}

#[test]
fn a_stale_cache_file_is_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let app = app_with_target(dir.path(), now);
    let png = icon_path(dir.path(), &app.id);
    Fake::new(Some(RED)).run(&app, dir.path()).unwrap();
    set_mtime(&png, now - Duration::from_secs(60));

    let fake = Fake::new(Some(BLUE));
    assert_eq!(
        fake.run(&app, dir.path()),
        Ok(Ensured::Written(png.clone()))
    );
    assert_eq!(fake.calls.borrow().len(), 1);
    assert_eq!(pixel(&png), BLUE);
    // Only the PNG is left: the temp file was renamed away.
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn a_missing_target_reuses_the_cache_or_uses_the_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_target(dir.path(), SystemTime::now());
    let png = icon_path(dir.path(), &app.id);
    fs::remove_file(&app.target).unwrap();

    let fake = Fake::new(Some(RED));
    let fallback = fallback_path(dir.path());
    assert_eq!(
        fake.run(&app, dir.path()),
        Ok(Ensured::Fallback(fallback.clone()))
    );
    assert!(fallback.is_file());
    assert!(!png.exists());

    fs::write(&png, b"old").unwrap();
    assert_eq!(fake.run(&app, dir.path()), Ok(Ensured::Fresh(png)));
    assert!(fake.calls.borrow().is_empty());
}

#[test]
fn a_failed_extraction_uses_the_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_target(dir.path(), SystemTime::now());
    let fallback = fallback_path(dir.path());
    let ensured = Fake::new(None).run(&app, dir.path());
    assert_eq!(ensured, Ok(Ensured::Fallback(fallback.clone())));
    assert!(!icon_path(dir.path(), &app.id).exists());
    let decoded = image::open(&fallback).unwrap().into_rgba8();
    assert_eq!(decoded.dimensions(), (ICON_SIZE, ICON_SIZE));
}

#[test]
fn the_fallback_is_written_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = ensure_fallback(dir.path()).unwrap();
    set_mtime(&path, SystemTime::UNIX_EPOCH);
    assert_eq!(ensure_fallback(dir.path()).unwrap(), path);
    let mtime = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(mtime, SystemTime::UNIX_EPOCH);
}

#[test]
fn unsafe_app_ids_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_target(dir.path(), SystemTime::now());
    let fake = Fake::new(Some(RED));
    for id in ["", "..", r"..\evil", "a/b", "a.b"] {
        app.id = id.into();
        assert!(fake.run(&app, dir.path()).is_err(), "{id:?}");
    }
    assert!(fake.calls.borrow().is_empty());
}

#[cfg(windows)]
#[test]
fn a_symlinked_png_is_never_written_through() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let app = app_with_target(dir.path(), SystemTime::now());
    let victim = outside.path().join("victim.png");
    fs::write(&victim, b"keep").unwrap();
    let png = icon_path(dir.path(), &app.id);
    // Creating symlinks needs Developer Mode or admin; skip the check if not.
    if std::os::windows::fs::symlink_file(&victim, &png).is_err() {
        return;
    }
    assert!(Fake::new(Some(RED)).run(&app, dir.path()).is_err());
    assert_eq!(fs::read(&victim).unwrap(), b"keep");
}
