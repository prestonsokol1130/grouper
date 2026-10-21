//! Test helper: records every file name that changes in one folder, the
//! way Explorer and the Start menu see it (`ReadDirectoryChangesW`).

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::{Duration, Instant};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

/// Written last into the watched folder; its event ends the recording.
pub const END_MARKER: &str = "zz-end.marker";

/// Watches `dir`, runs `work`, writes [`END_MARKER`] into `dir`, and
/// returns every file name reported for `dir` (marker included), in order.
pub fn names_seen_while(dir: &Path, work: impl FnOnce()) -> Vec<String> {
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: wide is a NUL-terminated path that outlives the call.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
    }
    .unwrap();
    // SAFETY: a plain manual-reset event with no name or attributes.
    let event = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }.unwrap();
    let mut buffer = vec![0_u32; 16 * 1024];
    let mut overlapped = OVERLAPPED {
        hEvent: event,
        ..OVERLAPPED::default()
    };
    arm(handle, &mut buffer, &mut overlapped);
    work();
    std::fs::write(dir.join(END_MARKER), b"end").unwrap();

    let mut names = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !names.iter().any(|name| name == END_MARKER) && Instant::now() < deadline {
        // SAFETY: event is a live handle owned here.
        if unsafe { WaitForSingleObject(event, 10_000) } != WAIT_OBJECT_0 {
            break;
        }
        let mut bytes = 0_u32;
        // SAFETY: overlapped belongs to the read armed on handle, which
        // has completed (its event is signalled).
        unsafe { GetOverlappedResult(handle, &overlapped, &mut bytes, false) }.unwrap();
        names.extend(parse(&buffer, bytes as usize));
        arm(handle, &mut buffer, &mut overlapped);
    }
    // SAFETY: cancel the read still armed and wait for it to finish, so
    // the kernel is done with buffer and overlapped before they drop.
    unsafe {
        let _ = CancelIoEx(handle, Some(&overlapped));
        let mut bytes = 0_u32;
        let _ = GetOverlappedResult(handle, &overlapped, &mut bytes, true);
        CloseHandle(event).unwrap();
        CloseHandle(handle).unwrap();
    }
    names
}

/// Starts an overlapped read of the next changes into `buffer`.
fn arm(handle: HANDLE, buffer: &mut [u32], overlapped: &mut OVERLAPPED) {
    let len = u32::try_from(std::mem::size_of_val(buffer)).unwrap();
    // SAFETY: buffer (DWORD aligned) and overlapped stay alive and unmoved
    // until the read completes or is cancelled in names_seen_while.
    unsafe {
        ReadDirectoryChangesW(
            handle,
            buffer.as_mut_ptr().cast(),
            len,
            false,
            FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE,
            None,
            Some(overlapped),
            None,
        )
    }
    .unwrap();
}

/// The names in a `FILE_NOTIFY_INFORMATION` list of `len` bytes.
fn parse(buffer: &[u32], len: usize) -> Vec<String> {
    let bytes: Vec<u8> = buffer.iter().flat_map(|word| word.to_le_bytes()).collect();
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut names = Vec::new();
    let mut at = 0;
    while at + 12 <= len {
        let (next, name_len) = (word(at), word(at + 8));
        let units: Vec<u16> = bytes[at + 12..at + 12 + name_len]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        names.push(String::from_utf16_lossy(&units));
        if next == 0 {
            break;
        }
        at += next;
    }
    names
}
