//! Writes sample folder images for a human to look at. Not shipped.
//!
//! Run from `src-tauri`: `cargo run --example compositor_samples`. Writes
//! to `target/compositor-samples/`:
//!
//! - `folder_0.png` .. `folder_4.png`: folders of 0 to 4 solid-colour icons
//!   in the default style, at 256 x 256.
//! - `golden_16.png`, `golden_32.png`: the 16 and 32 px frames decoded from
//!   `tests/golden/folder_4.ico`.
//! - `empty_256.png`, `empty_48.png`, `empty_24.png`, `empty_16.png`: the
//!   empty folder's `.ico` frames, as the taskbar gets them.

use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use grouper_lib::taskbar::compose::compose_folder;
use grouper_lib::taskbar::ico::encode_ico;
use grouper_lib::taskbar::style::FOLDER_STYLE;
use ico::IconDir;
use image::{Rgba, RgbaImage};

const COLOURS: [[u8; 4]; 4] = [
    [220, 40, 40, 255],
    [40, 180, 70, 255],
    [40, 90, 220, 255],
    [240, 200, 30, 255],
];

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("target").join("compositor-samples");
    fs::create_dir_all(&out)?;

    let icons: Vec<RgbaImage> = COLOURS
        .iter()
        .map(|&colour| RgbaImage::from_pixel(256, 256, Rgba(colour)))
        .collect();
    for count in 0..=icons.len() {
        let path = out.join(format!("folder_{count}.png"));
        compose_folder(&icons[..count], &FOLDER_STYLE).save(&path)?;
        println!("{}", path.display());
    }

    let golden = fs::read(root.join("tests").join("golden").join("folder_4.ico"))?;
    write_frames(&golden, &[16, 32], "golden", &out)?;
    let empty = encode_ico(&compose_folder(&[], &FOLDER_STYLE))?;
    write_frames(&empty, &[16, 24, 48, 256], "empty", &out)
}

/// Saves the `sizes` frames of the `.ico` in `bytes` as `{prefix}_{size}.png`.
fn write_frames(
    bytes: &[u8],
    sizes: &[u32],
    prefix: &str,
    out: &Path,
) -> Result<(), Box<dyn Error>> {
    let dir = IconDir::read(Cursor::new(bytes))?;
    for entry in dir.entries() {
        let size = entry.width();
        if sizes.contains(&size) {
            let frame = entry.decode()?;
            let image = RgbaImage::from_raw(size, size, frame.into_rgba_data())
                .ok_or("frame has the wrong number of pixels")?;
            let path = out.join(format!("{prefix}_{size}.png"));
            image.save(&path)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
