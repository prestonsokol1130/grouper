//! Expands `%VARIABLES%` in paths read from shortcuts and app entries.

/// Expands `%VARIABLES%` in `raw` with `ExpandEnvironmentStringsW`. Falls
/// back to `raw` unchanged (with a warning) if the expansion fails, so a bad
/// variable never loses the path. Unknown variables are left as they are,
/// and a string without `%` is returned as is. On non-Windows platforms it
/// returns `raw` unchanged.
pub fn expand_env(raw: &str) -> String {
    if !raw.contains('%') {
        return raw.to_owned();
    }
    imp::expand_env(raw)
}

#[cfg(windows)]
mod imp {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

    use crate::win32::wide::{from_wide, to_wide};

    /// Characters in the first output buffer; grown once if too small.
    const BUF_LEN: usize = MAX_PATH as usize * 4;

    pub(super) fn expand_env(raw: &str) -> String {
        let wide = to_wide(raw);
        let mut buf = vec![0u16; BUF_LEN];
        let source = PCWSTR(wide.as_ptr());
        for _ in 0..2 {
            // SAFETY: source points into wide, which is NUL-terminated and
            // outlives the call; buf is valid and writable for its length.
            let needed = unsafe { ExpandEnvironmentStringsW(source, Some(&mut buf)) };
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
    pub(super) fn expand_env(raw: &str) -> String {
        raw.to_owned()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::expand_env;

    #[test]
    fn expands_known_variables_and_keeps_the_rest() {
        let root = std::env::var("SystemRoot").unwrap();
        assert_eq!(
            expand_env(r"%SystemRoot%\System32"),
            format!(r"{root}\System32")
        );
        let unknown = r"%NOT_A_REAL_GROUPER_VAR%\x";
        assert_eq!(expand_env(unknown), unknown);
        assert_eq!(expand_env(""), "");
    }
}
