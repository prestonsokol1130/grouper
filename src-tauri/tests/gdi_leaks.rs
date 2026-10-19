//! GDI handle-leak checks for icon extraction and the owned GDI wrappers.
//!
//! `GetGuiResources` counts GDI objects for the whole process, so these
//! tests live in their own test binary (no other test shares the process)
//! and take a shared lock (they never overlap one another).
#![cfg(windows)]

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use grouper_lib::icons::extract::extract_icon;
use grouper_lib::win32::com::ComGuard;
use grouper_lib::win32::gdi::{OwnedDC, OwnedHBitmap};
use windows::Win32::Graphics::Gdi::CreateCompatibleBitmap;
use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};

const NOTEPAD: &str = r"C:\Windows\System32\notepad.exe";

static GDI_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    GDI_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// GDI objects currently held by this process.
fn gdi_objects() -> i64 {
    // SAFETY: GetCurrentProcess returns a pseudo-handle to this process
    // that needs no closing; GetGuiResources only reads a count.
    i64::from(unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) })
}

#[test]
fn extract_icon_does_not_leak_gdi_objects() {
    let _lock = lock();
    let _com = ComGuard::new().unwrap();
    let notepad = Path::new(NOTEPAD);
    // Warm up: the first extraction may make the Shell create objects that
    // it keeps for the life of the process.
    extract_icon(notepad, 256).unwrap();

    let start = gdi_objects();
    for _ in 0..500 {
        let icon = extract_icon(notepad, 256).unwrap();
        assert_eq!(icon.dimensions(), (256, 256));
    }
    let end = gdi_objects();
    let delta = end - start;
    println!("GDI objects over 500 extract_icon calls: start {start}, end {end}, delta {delta}");
    assert!(delta < 10, "GDI objects grew by {delta} ({start} -> {end})");
}

#[test]
fn owned_handles_are_freed_on_drop() {
    let _lock = lock();
    let start = gdi_objects();
    {
        let dc = OwnedDC::memory().unwrap();
        // SAFETY: dc is a live memory DC; the new bitmap belongs to the
        // caller.
        let raw = unsafe { CreateCompatibleBitmap(dc.get(), 16, 16) };
        // SAFETY: raw was just created here, nothing else frees it, and it
        // is never selected into a DC.
        let bitmap = unsafe { OwnedHBitmap::from_raw(raw) }.unwrap();
        let held = gdi_objects();
        // The count must see both handles, or this test proves nothing.
        assert!(held >= start + 2, "held {held}, start {start}");
        drop(bitmap);
        drop(dc);
    }
    for _ in 0..200 {
        drop(OwnedDC::memory().unwrap());
        drop(OwnedDC::screen().unwrap());
    }
    let end = gdi_objects();
    println!("GDI objects around owned handles: start {start}, end {end}");
    assert_eq!(end, start, "owned GDI handles leaked");
}
