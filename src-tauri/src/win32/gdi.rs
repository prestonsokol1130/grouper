//! Owned GDI handles, each freed exactly once when its owner is dropped.
//!
//! Windows only: these types wrap Win32 handles, so the module is compiled
//! on Windows alone.

use std::marker::PhantomData;

use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, ReleaseDC, HBITMAP, HDC,
};

/// A GDI bitmap owned by this value. Dropping it calls `DeleteObject` once.
///
/// The bitmap must not be selected into a device context when this value
/// drops: `DeleteObject` then fails (the failure is logged) and the bitmap
/// leaks.
#[derive(Debug)]
pub struct OwnedHBitmap(HBITMAP);

impl OwnedHBitmap {
    /// Takes ownership of `bitmap`. Returns `Err` for a null or invalid
    /// handle, which has nothing to free.
    ///
    /// # Safety
    ///
    /// `bitmap` must be a valid bitmap handle that the caller owns, and
    /// nothing else may free it: from this call on, only the returned value
    /// deletes it.
    pub unsafe fn from_raw(bitmap: HBITMAP) -> Result<Self, String> {
        if bitmap.is_invalid() {
            return Err("got a null bitmap handle".to_owned());
        }
        Ok(Self(bitmap))
    }

    /// The raw handle, to pass to GDI calls. It stays owned by `self`: never
    /// free it, and never use it after `self` is dropped.
    pub fn get(&self) -> HBITMAP {
        self.0
    }
}

impl Drop for OwnedHBitmap {
    fn drop(&mut self) {
        // SAFETY: self.0 is a valid bitmap owned by this value alone (the
        // from_raw contract), and drop runs once, so it is deleted once.
        let deleted = unsafe { DeleteObject(self.0.into()) };
        if !deleted.as_bool() {
            log::warn!(
                "DeleteObject failed for bitmap {:?}; it may still be selected into a DC",
                self.0 .0
            );
        }
    }
}

/// How an [`OwnedDC`] was obtained, which decides how it is freed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DcKind {
    /// From `CreateCompatibleDC`; freed with `DeleteDC`.
    Memory,
    /// From `GetDC(None)`; freed with `ReleaseDC(None, ..)`.
    Screen,
}

/// A device context owned by this value, freed the way it was obtained:
/// `DeleteDC` for a memory DC ([`OwnedDC::memory`]) and `ReleaseDC` for the
/// screen DC ([`OwnedDC::screen`]).
///
/// It is neither `Send` nor `Sync`, because a DC from `GetDC` must be
/// released on the thread that got it:
///
/// ```compile_fail,E0277
/// fn assert_send<T: Send>() {}
/// assert_send::<grouper_lib::win32::gdi::OwnedDC>();
/// ```
#[derive(Debug)]
pub struct OwnedDC {
    hdc: HDC,
    kind: DcKind,
    /// A raw pointer is neither `Send` nor `Sync`, so neither is the DC.
    _not_send: PhantomData<*const ()>,
}

impl OwnedDC {
    /// A new memory DC compatible with the screen (`CreateCompatibleDC`),
    /// deleted with `DeleteDC` on drop.
    pub fn memory() -> Result<Self, String> {
        // SAFETY: CreateCompatibleDC(None) takes no pointers; the new DC is
        // owned by the returned value and deleted once, in drop.
        let hdc = unsafe { CreateCompatibleDC(None) };
        Self::own(hdc, DcKind::Memory, "CreateCompatibleDC")
    }

    /// The screen DC (`GetDC(None)`), released with `ReleaseDC(None, ..)` on
    /// drop, on this same thread.
    pub fn screen() -> Result<Self, String> {
        // SAFETY: GetDC(None) takes no pointers; the DC is released once, in
        // drop, on this thread (the value cannot leave it).
        let hdc = unsafe { GetDC(None) };
        Self::own(hdc, DcKind::Screen, "GetDC")
    }

    fn own(hdc: HDC, kind: DcKind, call: &str) -> Result<Self, String> {
        if hdc.is_invalid() {
            return Err(format!("{call} returned no device context"));
        }
        Ok(Self {
            hdc,
            kind,
            _not_send: PhantomData,
        })
    }

    /// The raw handle, to pass to GDI calls. It stays owned by `self`: never
    /// free it, and never use it after `self` is dropped.
    pub fn get(&self) -> HDC {
        self.hdc
    }
}

impl Drop for OwnedDC {
    fn drop(&mut self) {
        let freed = match self.kind {
            // SAFETY: hdc came from CreateCompatibleDC and is deleted only
            // here, once.
            DcKind::Memory => unsafe { DeleteDC(self.hdc).as_bool() },
            // SAFETY: hdc came from GetDC(None) on this thread (the value is
            // not Send) and is released only here, once.
            DcKind::Screen => unsafe { ReleaseDC(None, self.hdc) == 1 },
        };
        if !freed {
            log::warn!("freeing {:?} DC {:?} failed", self.kind, self.hdc.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::Graphics::Gdi::HBITMAP;

    use super::{DcKind, OwnedDC, OwnedHBitmap};

    #[test]
    fn null_bitmap_is_rejected() {
        // SAFETY: a null handle owns nothing; from_raw must refuse it.
        let result = unsafe { OwnedHBitmap::from_raw(HBITMAP(std::ptr::null_mut())) };
        assert!(result.is_err());
    }

    #[test]
    fn each_dc_kind_is_recorded() {
        assert_eq!(OwnedDC::memory().unwrap().kind, DcKind::Memory);
        assert_eq!(OwnedDC::screen().unwrap().kind, DcKind::Screen);
    }
}
