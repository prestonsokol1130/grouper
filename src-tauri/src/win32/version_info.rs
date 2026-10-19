//! Reads the description from a program's version resource.

use std::path::Path;

/// The `FileDescription` of the program at `path`, read from the string
/// table of the first language in its version resource that has a
/// non-empty one. `None` when the file has no version resource or no
/// description (and always on non-Windows platforms).
pub fn file_description(path: &Path) -> Option<String> {
    imp::file_description(path)
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };

    use crate::win32::wide::{from_wide, to_wide};

    pub(super) fn file_description(path: &Path) -> Option<String> {
        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide_path is NUL-terminated and outlives the call; the
        // optional handle out-parameter is not wanted.
        let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(wide_path.as_ptr()), None) };
        if size == 0 {
            return None;
        }
        // u32 words keep the block 4-byte aligned, like the version
        // structures inside it, and hold at least `size` bytes.
        let mut block = vec![0_u32; (size as usize).div_ceil(4)];
        // SAFETY: block has at least `size` writable bytes and outlives the
        // call; wide_path is NUL-terminated and outlives the call.
        unsafe {
            GetFileVersionInfoW(
                PCWSTR(wide_path.as_ptr()),
                None,
                size,
                block.as_mut_ptr().cast(),
            )
        }
        .ok()?;
        translations(&block)
            .into_iter()
            .find_map(|(lang, codepage)| {
                let key = format!(r"\StringFileInfo\{lang:04x}{codepage:04x}\FileDescription");
                let text = query_string(&block, &key)?;
                let text = text.trim();
                (!text.is_empty()).then(|| text.to_owned())
            })
    }

    /// Looks up `sub_block` in `block`; returns the value's address and the
    /// length `VerQueryValueW` reported (bytes or characters, by value kind).
    fn query(block: &[u32], sub_block: &str) -> Option<(*const c_void, usize)> {
        let key = to_wide(sub_block);
        let mut value: *mut c_void = std::ptr::null_mut();
        let mut len = 0_u32;
        // SAFETY: block is the buffer GetFileVersionInfoW filled and stays
        // alive; key is NUL-terminated and outlives the call; value and len
        // are live out-parameters.
        let found = unsafe {
            VerQueryValueW(
                block.as_ptr().cast(),
                PCWSTR(key.as_ptr()),
                &mut value,
                &mut len,
            )
        };
        (found.as_bool() && !value.is_null() && len > 0)
            .then_some((value.cast_const(), len as usize))
    }

    /// True when `bytes` bytes from `value` all lie inside `block`.
    fn within(block: &[u32], value: *const c_void, bytes: usize) -> bool {
        let start = block.as_ptr().addr();
        let end = start + std::mem::size_of_val(block);
        let at = value.addr();
        at >= start && at.checked_add(bytes).is_some_and(|stop| stop <= end)
    }

    /// The (language, code page) pairs listed in `\VarFileInfo\Translation`.
    fn translations(block: &[u32]) -> Vec<(u16, u16)> {
        let Some((value, bytes)) = query(block, r"\VarFileInfo\Translation") else {
            return Vec::new();
        };
        let count = bytes / 4;
        if !within(block, value, count * 4) {
            return Vec::new();
        }
        let pairs = value.cast::<[u16; 2]>();
        (0..count)
            .map(|at| {
                // SAFETY: within() checked that all `count` pairs lie inside
                // block; read_unaligned has no alignment requirement.
                let [lang, codepage] = unsafe { pairs.add(at).read_unaligned() };
                (lang, codepage)
            })
            .collect()
    }

    /// The string value at `sub_block`, if any.
    fn query_string(block: &[u32], sub_block: &str) -> Option<String> {
        let (value, chars) = query(block, sub_block)?;
        if !within(block, value, chars * 2) {
            return None;
        }
        let units = value.cast::<u16>();
        let text: Vec<u16> = (0..chars)
            // SAFETY: within() checked that all `chars` units lie inside
            // block; read_unaligned has no alignment requirement.
            .map(|at| unsafe { units.add(at).read_unaligned() })
            .collect();
        Some(from_wide(&text))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub(super) fn file_description(_path: &Path) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::file_description;

    #[test]
    fn a_file_without_resources_has_no_description() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("NoResources.exe");
        std::fs::write(&path, b"not a real program").unwrap();
        assert_eq!(file_description(&path), None);
        assert_eq!(file_description(&dir.path().join("absent.exe")), None);
    }

    /// Every Windows install has cmd.exe with a version resource.
    #[cfg(windows)]
    #[test]
    fn a_system_program_has_a_description() {
        let windir = std::env::var("SystemRoot").unwrap();
        let cmd = std::path::Path::new(&windir).join(r"System32\cmd.exe");
        let description = file_description(&cmd).unwrap();
        assert!(!description.is_empty());
    }
}
