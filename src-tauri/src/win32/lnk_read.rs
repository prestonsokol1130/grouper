//! Reads the fields of a Windows shortcut (`.lnk`) file.

use std::path::Path;

/// The fields read from a `.lnk` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLink {
    /// Target path with environment variables expanded. Empty when the
    /// shortcut has no file-system target (for example a shortcut that only
    /// stores a Shell item ID list, such as many Store-app shortcuts).
    pub target: String,
    /// Command-line arguments, as stored. Empty when there are none.
    pub args: String,
    /// Working directory, as stored (not environment-expanded). Empty when
    /// there is none.
    pub working_dir: String,
    /// Icon file and index, as stored (the path is not environment-expanded).
    /// `None` when the shortcut sets no icon of its own.
    pub icon_location: Option<(String, i32)>,
}

/// Reads the shortcut at `path` without resolving or repairing it.
///
/// The caller's thread must hold a live [`ComGuard`](super::com::ComGuard)
/// for the whole call; without one this returns `Err` (COM is not
/// initialised). Every COM interface used here is released before it
/// returns, so the guard may be dropped as soon as this call is done.
///
/// `IShellLink::Resolve` is deliberately never called: it can search the
/// disk and show UI. The target may therefore point to a file that no longer
/// exists; callers check that themselves.
///
/// Returns `Err` if the file is missing, unreadable or not a shortcut. On
/// non-Windows platforms it always returns `Err("unsupported platform")`.
pub fn resolve_lnk(path: &Path) -> Result<ResolvedLink, String> {
    imp::resolve_lnk(path)
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::System::Com::{
        CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
    };
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

    use super::ResolvedLink;
    use crate::win32::wide::{from_wide, to_wide};

    /// Characters in each string buffer handed to `IShellLinkW`.
    const BUF_LEN: usize = MAX_PATH as usize * 4;

    pub(super) fn resolve_lnk(path: &Path) -> Result<ResolvedLink, String> {
        let shown = path.display();
        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: COM is initialised on this thread (the caller holds a
        // ComGuard, otherwise this returns CO_E_NOTINITIALIZED as Err).
        let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| format!("create ShellLink: {e}"))?;
        let file: IPersistFile = link
            .cast()
            .map_err(|e| format!("query IPersistFile: {e}"))?;
        // SAFETY: wide_path is NUL-terminated and outlives the call.
        unsafe { file.Load(PCWSTR(wide_path.as_ptr()), STGM_READ) }
            .map_err(|e| format!("load {shown}: {e}"))?;

        let raw_target = read_string(|buf| {
            // SAFETY: buf is a valid, writable buffer; no find data wanted.
            // SLGP_RAWPATH is a small positive flag, so the cast is exact.
            unsafe { link.GetPath(buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32) }
        })
        .map_err(|e| format!("read target of {shown}: {e}"))?;
        // SAFETY: buf is a valid, writable buffer.
        let args = read_string(|buf| unsafe { link.GetArguments(buf) })
            .map_err(|e| format!("read arguments of {shown}: {e}"))?;
        // SAFETY: buf is a valid, writable buffer.
        let working_dir = read_string(|buf| unsafe { link.GetWorkingDirectory(buf) })
            .map_err(|e| format!("read working directory of {shown}: {e}"))?;
        let mut icon_index = 0i32;
        // SAFETY: buf is a valid, writable buffer; icon_index is a live i32.
        let icon_path = read_string(|buf| unsafe { link.GetIconLocation(buf, &mut icon_index) })
            .map_err(|e| format!("read icon location of {shown}: {e}"))?;

        // `file` and `link` are released here, before returning, so no COM
        // object outlives the caller's ComGuard.
        Ok(ResolvedLink {
            target: expand_env(&raw_target),
            args,
            working_dir,
            icon_location: (!icon_path.is_empty()).then_some((icon_path, icon_index)),
        })
    }

    /// Runs `fill` on a zeroed `BUF_LEN` buffer and decodes the result.
    fn read_string(
        fill: impl FnOnce(&mut [u16]) -> windows::core::Result<()>,
    ) -> windows::core::Result<String> {
        let mut buf = vec![0u16; BUF_LEN];
        fill(&mut buf)?;
        Ok(from_wide(&buf))
    }

    /// Expands `%VARIABLES%` in `raw`. Falls back to `raw` unchanged if the
    /// expansion fails, so a bad variable never loses the path.
    fn expand_env(raw: &str) -> String {
        if !raw.contains('%') {
            return raw.to_owned();
        }
        let source = to_wide(raw);
        let mut buf = vec![0u16; BUF_LEN];
        for _ in 0..2 {
            // SAFETY: source is NUL-terminated; buf is valid and writable.
            let needed =
                unsafe { ExpandEnvironmentStringsW(PCWSTR(source.as_ptr()), Some(&mut buf)) };
            let needed = needed as usize;
            if needed == 0 {
                break;
            }
            if needed <= buf.len() {
                return from_wide(&buf);
            }
            // Too small: `needed` is the full size including the NUL.
            buf = vec![0u16; needed];
        }
        log::warn!("could not expand environment variables in {raw:?}");
        raw.to_owned()
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    use super::ResolvedLink;

    pub(super) fn resolve_lnk(_path: &Path) -> Result<ResolvedLink, String> {
        Err("unsupported platform".to_owned())
    }
}
