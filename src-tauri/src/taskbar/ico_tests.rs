//! Checks the raw `ICONDIR` of the group `.ico` field by field against
//! what Windows' own icons use. For reference, the folder icon group 4 in
//! `shell32.dll` (Windows 11) has 8 entries, 256 (as PNG), 64, 48, 40, 32,
//! 24, 20 and 16, each with `bColorCount` 0, `wPlanes` 1 and `wBitCount`
//! 32; its 32 px frame is 4264 bytes (header, colour and AND mask).

use image::{Rgba, RgbaImage};

use super::ico::{encode_ico, ICO_SIZES};

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn sample_ico() -> Vec<u8> {
    let source = RgbaImage::from_fn(256, 256, |x, y| {
        let alpha = if x < 8 || y < 8 { 96 } else { 255 };
        Rgba([(x % 256) as u8, (y % 256) as u8, 120, alpha])
    });
    encode_ico(&source).unwrap()
}

#[test]
fn the_icon_directory_matches_windows_own_icons() {
    let bytes = sample_ico();
    assert_eq!(u16_at(&bytes, 0), 0, "reserved");
    assert_eq!(u16_at(&bytes, 2), 1, "type: icon");
    let count = usize::from(u16_at(&bytes, 4));
    assert_eq!(count, ICO_SIZES.len());

    let mut largest_first = ICO_SIZES.to_vec();
    largest_first.reverse();
    let mut next_offset = 6 + 16 * count;
    for (index, size) in largest_first.into_iter().enumerate() {
        let entry = &bytes[6 + 16 * index..6 + 16 * (index + 1)];
        let side = if size == 256 { 0 } else { size as u8 };
        assert_eq!(entry[0], side, "{size} px width byte");
        assert_eq!(entry[1], side, "{size} px height byte");
        assert_eq!(entry[2], 0, "{size} px bColorCount");
        assert_eq!(entry[3], 0, "{size} px reserved");
        assert_eq!(u16_at(entry, 4), 1, "{size} px wPlanes");
        assert_eq!(u16_at(entry, 6), 32, "{size} px wBitCount");
        let (len, offset) = (u32_at(entry, 8) as usize, u32_at(entry, 12) as usize);
        assert_eq!(offset, next_offset, "{size} px frames are packed in order");
        next_offset = offset + len;
        assert!(next_offset <= bytes.len(), "{size} px frame is in the file");
        check_frame(&bytes[offset..next_offset], size);
    }
    assert_eq!(next_offset, bytes.len(), "nothing after the last frame");
}

/// 256 is a PNG of that size; the rest are 32-bit DIBs with a full AND
/// mask (rows padded to 4 bytes), as `LoadImage` expects.
fn check_frame(frame: &[u8], size: u32) {
    if size == 256 {
        assert_eq!(frame[..8], PNG_SIGNATURE);
        assert_eq!(&frame[12..16], b"IHDR");
        let width = u32::from_be_bytes(frame[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(frame[20..24].try_into().unwrap());
        assert_eq!((width, height), (256, 256));
        return;
    }
    assert_eq!(u32_at(frame, 0), 40, "{size} px BITMAPINFOHEADER size");
    assert_eq!(u32_at(frame, 4), size, "{size} px biWidth");
    assert_eq!(u32_at(frame, 8), 2 * size, "{size} px biHeight (XOR + AND)");
    assert_eq!(u16_at(frame, 12), 1, "{size} px biPlanes");
    assert_eq!(u16_at(frame, 14), 32, "{size} px biBitCount");
    assert_eq!(u32_at(frame, 16), 0, "{size} px BI_RGB");
    let side = size as usize;
    let mask_row = side.div_ceil(8).div_ceil(4) * 4;
    assert_eq!(frame.len(), 40 + side * side * 4 + mask_row * side);
}

#[test]
fn a_32_px_frame_is_the_same_size_as_windows_own() {
    let bytes = sample_ico();
    let index = ICO_SIZES.len() - 1 - ICO_SIZES.iter().position(|&s| s == 32).unwrap();
    let entry = 6 + 16 * index;
    assert_eq!(bytes[entry], 32);
    assert_eq!(u32_at(&bytes, entry + 8), 4264);
}
