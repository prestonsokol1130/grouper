//! Conversions between Rust strings and Win32 wide (UTF-16) strings.

/// Encodes `s` as UTF-16 with a trailing NUL, ready to pass as a `PCWSTR`.
///
/// Win32 reads up to the first NUL, so an interior NUL in `s` cuts the
/// string short there. Keep the returned vector alive for as long as the
/// pointer is in use.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Decodes a UTF-16 buffer filled by Win32, stopping at the first NUL (or
/// the end of the slice if there is none). Invalid UTF-16, such as a lone
/// surrogate, becomes U+FFFD instead of failing.
pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&unit| unit == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(test)]
mod tests {
    use super::{from_wide, to_wide};

    #[test]
    fn to_wide_appends_one_nul() {
        assert_eq!(to_wide("ab"), vec![u16::from(b'a'), u16::from(b'b'), 0]);
        assert_eq!(to_wide(""), vec![0]);
    }

    #[test]
    fn round_trips_non_ascii() {
        let text = r"C:\Users\Zoë\Ünïcode 📁";
        assert_eq!(from_wide(&to_wide(text)), text);
    }

    #[test]
    fn from_wide_stops_at_first_nul() {
        let buf = [u16::from(b'h'), u16::from(b'i'), 0, u16::from(b'x'), 0];
        assert_eq!(from_wide(&buf), "hi");
    }

    #[test]
    fn from_wide_without_nul_reads_whole_slice() {
        let buf = [u16::from(b'o'), u16::from(b'k')];
        assert_eq!(from_wide(&buf), "ok");
        assert_eq!(from_wide(&[]), "");
    }

    #[test]
    fn from_wide_is_lossy_on_lone_surrogate() {
        let buf = [u16::from(b'a'), 0xD800, u16::from(b'b'), 0];
        assert_eq!(from_wide(&buf), "a\u{FFFD}b");
    }
}
