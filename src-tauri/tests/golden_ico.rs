//! Golden tests for the folder `.ico`.
//!
//! Four solid-colour icons (and, for the empty folder, none) are composed
//! with the default folder style and encoded, and every decoded frame is
//! compared with `tests/golden/folder_4.ico` (`folder_0.ico`), pixel by
//! pixel within a small tolerance (not byte for byte, so encoder details do
//! not matter).
//!
//! After an intended change to the look (for example `FOLDER_STYLE`),
//! regenerate the golden from `src-tauri` and commit it:
//!
//! - bash: `UPDATE_GOLDEN=1 cargo test --test golden_ico`
//! - PowerShell: `$env:UPDATE_GOLDEN=1; cargo test --test golden_ico; Remove-Item Env:UPDATE_GOLDEN`

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use grouper_lib::taskbar::compose::compose_folder;
use grouper_lib::taskbar::ico::{encode_ico, ICO_SIZES};
use grouper_lib::taskbar::style::FOLDER_STYLE;
use ico::IconDir;
use image::{Rgba, RgbaImage};

/// Largest allowed difference per premultiplied channel (0..=255).
const TOLERANCE: u8 = 2;

fn golden_path(count: usize) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/folder_{count}.ico"))
}

fn current_ico(count: usize) -> Vec<u8> {
    let icons: Vec<RgbaImage> = [
        [220, 40, 40, 255],
        [40, 180, 70, 255],
        [40, 90, 220, 255],
        [240, 200, 30, 255],
    ]
    .into_iter()
    .map(|colour| RgbaImage::from_pixel(256, 256, Rgba(colour)))
    .collect();
    encode_ico(&compose_folder(&icons[..count], &FOLDER_STYLE)).unwrap()
}

/// Each frame's side and premultiplied RGBA bytes, smallest first.
fn frames(bytes: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let dir = IconDir::read(Cursor::new(bytes)).unwrap();
    let mut frames: Vec<(u32, Vec<u8>)> = dir
        .entries()
        .iter()
        .map(|entry| {
            let mut rgba = entry.decode().unwrap().into_rgba_data();
            for px in rgba.as_chunks_mut::<4>().0 {
                let alpha = u16::from(px[3]);
                for channel in &mut px[..3] {
                    *channel = u8::try_from(u16::from(*channel) * alpha / 255).unwrap();
                }
            }
            (entry.width(), rgba)
        })
        .collect();
    frames.sort_by_key(|(size, _)| *size);
    frames
}

#[test]
fn folder_4_matches_the_golden() {
    assert_matches_golden(4);
}

#[test]
fn empty_folder_matches_the_golden() {
    assert_matches_golden(0);
}

fn assert_matches_golden(count: usize) {
    let current = current_ico(count);
    let path = golden_path(count);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &current).unwrap();
        println!("wrote {}", path.display());
    }
    let golden = fs::read(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} (UPDATE_GOLDEN=1 writes it)", path.display()));

    let (current, golden) = (frames(&current), frames(&golden));
    let sizes: Vec<u32> = golden.iter().map(|(size, _)| *size).collect();
    assert_eq!(sizes, ICO_SIZES);
    assert_eq!(current.len(), golden.len());
    for ((size, now), (_, then)) in current.iter().zip(&golden) {
        assert_eq!(now.len(), then.len(), "{size} px frame size");
        let worst = now.iter().zip(then).map(|(a, b)| a.abs_diff(*b)).max();
        assert!(
            worst.unwrap_or(0) <= TOLERANCE,
            "{size} px frame differs by up to {worst:?}; if the change is intended, \
             regenerate with UPDATE_GOLDEN=1"
        );
    }
}
