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

/// A two-image `.ico`: 16 px red and 64 px blue, built from two one-image
/// files.
fn two_size_ico() -> Vec<u8> {
    let small = ico_bytes(16, Rgba([255, 0, 0, 255]));
    let large = ico_bytes(64, Rgba([0, 0, 255, 255]));
    let (small_data, large_data) = (&small[22..], &large[22..]);
    let mut out = vec![0, 0, 1, 0, 2, 0];
    let mut offset = 6 + 2 * 16;
    for (file, data) in [(&small, small_data), (&large, large_data)] {
        out.extend_from_slice(&file[6..18]);
        out.extend_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
        offset += data.len();
    }
    out.extend_from_slice(small_data);
    out.extend_from_slice(large_data);
    out
}

#[test]
fn the_image_nearest_the_size_is_drawn() {
    let ico = two_size_ico();
    assert_eq!(
        decode_ico(&ico, 16).unwrap().get_pixel(8, 8),
        &Rgba([255, 0, 0, 255])
    );
    assert_eq!(
        decode_ico(&ico, 32).unwrap().get_pixel(16, 16),
        &Rgba([0, 0, 255, 255])
    );
    assert_eq!(
        decode_ico(&ico, 256).unwrap().get_pixel(128, 128),
        &Rgba([0, 0, 255, 255])
    );
}
