//! Reading a shortcut's own `.ico` file.

use std::fs;
use std::io::Cursor;

use image::{ImageFormat, Rgba, RgbaImage};

use super::ico_file::{decode_ico, ico_path, read_ico};

/// A one-image `.ico` of `side` x `side` pixels, all `color`.
fn ico_bytes(side: u32, color: Rgba<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    RgbaImage::from_pixel(side, side, color)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Ico)
        .unwrap();
    bytes
}

#[test]
fn an_ico_is_scaled_to_the_asked_size() {
    let image = decode_ico(&ico_bytes(32, Rgba([200, 30, 30, 255])), 256).unwrap();
    assert_eq!(image.dimensions(), (256, 256));
    assert_eq!(image.get_pixel(128, 128), &Rgba([200, 30, 30, 255]));
}

#[test]
fn a_blank_or_broken_ico_is_refused() {
    assert!(decode_ico(&ico_bytes(16, Rgba([0, 0, 0, 0])), 64).is_err());
    assert!(decode_ico(b"not an icon", 64).is_err());
}

#[test]
fn only_an_existing_ico_at_index_0_is_used() {
    let dir = tempfile::tempdir().unwrap();
    let ico = dir.path().join("App.ico");
    fs::write(&ico, ico_bytes(48, Rgba([1, 2, 3, 255]))).unwrap();
    let raw = ico.to_string_lossy().into_owned();
    assert_eq!(ico_path(Some((raw.clone(), 0))), Some(ico.clone()));
    assert_eq!(ico_path(Some((raw, 1))), None);
    let exe = dir.path().join("chrome_proxy.exe");
    fs::write(&exe, b"MZ").unwrap();
    assert_eq!(
        ico_path(Some((exe.to_string_lossy().into_owned(), 0))),
        None
    );
    let gone = dir.path().join("gone.ico").to_string_lossy().into_owned();
    assert_eq!(ico_path(Some((gone, 0))), None);
    assert_eq!(ico_path(None), None);
    assert_eq!(read_ico(&ico, 96).unwrap().dimensions(), (96, 96));
}
