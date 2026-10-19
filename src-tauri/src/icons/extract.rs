//! Extracts an item's icon as RGBA pixels through the Windows Shell.

use std::path::Path;

use image::RgbaImage;

/// Extracts the icon of the Shell item at `path` (an `.exe`, a `.lnk` or any
/// other file) as a `size` x `size` image with straight (not premultiplied)
/// alpha.
///
/// The caller's thread must hold a live
/// [`ComGuard`](crate::win32::com::ComGuard) for the whole call; without one
/// this returns `Err` (COM is not initialised). Every COM interface and GDI
/// handle used here is released before it returns, so the guard may be
/// dropped as soon as this call is done.
///
/// The Shell is asked for the icon only (never a thumbnail) at `size`, with
/// `SIIGBF_BIGGERSIZEOK`; if that fails it is asked once more, allowing a
/// thumbnail. A result of another size is scaled with Lanczos3, keeping its
/// aspect ratio, and centred on a transparent canvas.
///
/// This can read the disk and be slow, so call it off the UI thread.
///
/// Returns `Err` if `size` is 0 or above
/// [`MAX_ICON_SIZE`](super::MAX_ICON_SIZE), if the item cannot be found, or
/// if the Shell gives no usable bitmap, including a fully blank one. On
/// non-Windows platforms it always returns `Err("unsupported platform")`.
pub fn extract_icon(path: &Path, size: u32) -> Result<RgbaImage, String> {
    if size == 0 || size > super::MAX_ICON_SIZE {
        return Err(format!(
            "icon size {size} is outside 1..={}",
            super::MAX_ICON_SIZE
        ));
    }
    imp::extract_icon(path, size)
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use image::RgbaImage;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF, SIIGBF_BIGGERSIZEOK,
        SIIGBF_ICONONLY,
    };

    use crate::icons::pixels::{fit_square, gdi_to_rgba};
    use crate::win32::gdi::{OwnedDC, OwnedHBitmap};

    /// Largest bitmap side accepted from the Shell. With
    /// `SIIGBF_BIGGERSIZEOK` it may return more than was asked for; this cap
    /// keeps a bad Shell extension from forcing a huge allocation.
    const MAX_BITMAP_SIDE: u32 = 4096;

    /// `size` is already checked to be in `1..=MAX_ICON_SIZE`.
    pub(super) fn extract_icon(path: &Path, size: u32) -> Result<RgbaImage, String> {
        let shown = path.display();
        let side = i32::try_from(size).map_err(|_| format!("icon size {size} is too large"))?;
        let bitmap = shell_bitmap(path, side).map_err(|e| format!("icon of {shown}: {e}"))?;
        let (width, height) = bitmap_size(&bitmap).map_err(|e| format!("icon of {shown}: {e}"))?;
        let mut buf =
            read_pixels(&bitmap, width, height).map_err(|e| format!("icon of {shown}: {e}"))?;
        // The pixels are copied out, so free the bitmap now.
        drop(bitmap);

        gdi_to_rgba(&mut buf).map_err(|e| format!("icon of {shown}: {e}"))?;
        let image = RgbaImage::from_raw(width, height, buf)
            .ok_or_else(|| format!("icon of {shown}: pixel buffer has the wrong length"))?;
        Ok(fit_square(image, size))
    }

    /// Asks the Shell for the item's icon, then (on failure) for any image.
    fn shell_bitmap(path: &Path, side: i32) -> Result<OwnedHBitmap, String> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide is NUL-terminated and outlives the call. COM is
        // initialised on this thread (the caller holds a ComGuard; without
        // one this returns CO_E_NOTINITIALIZED as Err).
        let created = unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) };
        let factory: IShellItemImageFactory =
            created.map_err(|e| format!("open Shell item: {e}"))?;
        let size = SIZE { cx: side, cy: side };
        get_image(&factory, size, SIIGBF_BIGGERSIZEOK | SIIGBF_ICONONLY).or_else(|icon_error| {
            log::debug!("icon-only GetImage failed ({icon_error}); retrying with thumbnails");
            get_image(&factory, size, SIIGBF_BIGGERSIZEOK)
                .map_err(|e| format!("GetImage failed: {icon_error}; then: {e}"))
        })
        // `factory` is released here, before returning.
    }

    /// One `GetImage` call. The bitmap is owned the moment it is returned.
    fn get_image(
        factory: &IShellItemImageFactory,
        size: SIZE,
        flags: SIIGBF,
    ) -> Result<OwnedHBitmap, String> {
        // SAFETY: factory is a live interface on this COM thread; on success
        // GetImage hands the caller a bitmap that the caller must delete.
        let raw = unsafe { factory.GetImage(size, flags) }.map_err(|e| e.to_string())?;
        // SAFETY: raw was just returned by GetImage, so this code owns it,
        // nothing else frees it, and it is not selected into any DC.
        unsafe { OwnedHBitmap::from_raw(raw) }
    }

    /// The bitmap's width and height, each checked to be in
    /// `1..=MAX_BITMAP_SIDE`.
    fn bitmap_size(bitmap: &OwnedHBitmap) -> Result<(u32, u32), String> {
        let mut info = BITMAP::default();
        // BITMAP is a small fixed-size struct, so its size fits an i32.
        let info_len = std::mem::size_of::<BITMAP>() as i32;
        // SAFETY: the bitmap handle is live, and info is a writable BITMAP
        // of exactly info_len bytes.
        let written = unsafe {
            GetObjectW(
                bitmap.get().into(),
                info_len,
                Some(std::ptr::from_mut(&mut info).cast()),
            )
        };
        if written == 0 {
            return Err("GetObjectW could not read the bitmap".to_owned());
        }
        let side =
            |value: i32| Some(value.unsigned_abs()).filter(|v| (1..=MAX_BITMAP_SIDE).contains(v));
        match (side(info.bmWidth), side(info.bmHeight)) {
            (Some(width), Some(height)) => Ok((width, height)),
            _ => Err(format!(
                "bitmap is {} x {}, outside 1..={MAX_BITMAP_SIDE}",
                info.bmWidth, info.bmHeight
            )),
        }
    }

    /// Copies the bitmap out as top-down 32-bit BGRA rows.
    fn read_pixels(bitmap: &OwnedHBitmap, width: u32, height: u32) -> Result<Vec<u8>, String> {
        // Both sides are at most MAX_BITMAP_SIDE (4096), so these conversions
        // and the buffer length cannot overflow.
        let (w, h) = (width as i32, height as i32);
        let mut buf = vec![0u8; width as usize * height as usize * 4];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative height asks for top-down rows.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // A memory DC that the bitmap is never selected into, as GetDIBits
        // requires.
        let dc = OwnedDC::memory()?;
        // SAFETY: buf holds exactly height rows of width 32-bit pixels (such
        // rows need no padding); info describes that buffer as a 32 bpp
        // BI_RGB DIB, which has no colour table to write past bmiColors; the
        // bitmap and DC are live, and the bitmap is not selected into a DC.
        let lines = unsafe {
            GetDIBits(
                dc.get(),
                bitmap.get(),
                0,
                height,
                Some(buf.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        if lines != h {
            return Err(format!("GetDIBits copied {lines} of {h} rows"));
        }
        Ok(buf)
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    use image::RgbaImage;

    pub(super) fn extract_icon(_path: &Path, _size: u32) -> Result<RgbaImage, String> {
        Err("unsupported platform".to_owned())
    }
}
