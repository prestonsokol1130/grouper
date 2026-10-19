//! Windows-only tests for `extract_icon` and `icon_or_fallback` against real
//! files. The GDI leak test is in `tests/gdi_leaks.rs`, in its own process.

use std::path::Path;

use image::RgbaImage;

use super::extract::extract_icon;
use super::fallback::fallback_icon;
use super::{icon_or_fallback, MAX_ICON_SIZE};
use crate::win32::com::ComGuard;
use crate::win32::lnk_read_tests::write_lnk_to;

const NOTEPAD: &str = r"C:\Windows\System32\notepad.exe";

/// Asserts `icon` is `size` x `size` and not fully transparent.
fn assert_visible(icon: &RgbaImage, size: u32) {
    assert_eq!(icon.dimensions(), (size, size));
    let visible = icon.pixels().filter(|p| p[3] > 0).count();
    assert!(visible > 0, "every pixel is transparent");
}

#[test]
fn extracts_notepad_exe_at_256() {
    let _com = ComGuard::new().unwrap();
    let icon = extract_icon(Path::new(NOTEPAD), 256).unwrap();
    assert_visible(&icon, 256);
}

#[test]
fn extracts_a_shortcut_at_256() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let lnk = dir.path().join("Notepad.lnk");
    write_lnk_to(&lnk, NOTEPAD, "", "", None);
    let icon = extract_icon(&lnk, 256).unwrap();
    assert_visible(&icon, 256);
}

#[test]
fn odd_sizes_come_back_square() {
    let _com = ComGuard::new().unwrap();
    for size in [1, 100] {
        let icon = extract_icon(Path::new(NOTEPAD), size).unwrap();
        assert_visible(&icon, size);
    }
}

#[test]
fn missing_file_is_an_error() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let result = extract_icon(&dir.path().join("does-not-exist.exe"), 256);
    assert!(result.is_err(), "got an image for a missing file");
}

#[test]
fn sizes_out_of_range_are_errors() {
    let _com = ComGuard::new().unwrap();
    let notepad = Path::new(NOTEPAD);
    assert!(extract_icon(notepad, 0).is_err());
    assert!(extract_icon(notepad, MAX_ICON_SIZE + 1).is_err());
}

#[test]
fn failed_extraction_returns_the_fallback() {
    let _com = ComGuard::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let icon = icon_or_fallback(&dir.path().join("does-not-exist.exe"), 256);
    assert_eq!(icon, fallback_icon(256));
}

#[test]
fn successful_extraction_is_not_the_fallback() {
    let _com = ComGuard::new().unwrap();
    let icon = icon_or_fallback(Path::new(NOTEPAD), 256);
    assert_visible(&icon, 256);
    assert_ne!(icon, fallback_icon(256));
}
