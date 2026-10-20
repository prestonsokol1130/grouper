//! Id generation for apps and groups.

use sha2::{Digest, Sha256};

/// Characters allowed in a group id.
const GROUP_ID_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
/// Length of a generated group id.
pub(crate) const GROUP_ID_LEN: usize = 10;

/// Stable app id: the first 16 hex chars of
/// `sha256(lowercase(target) + "|" + args)`. Case-insensitive on the target.
pub fn app_id(target: &str, args: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(target.to_lowercase().as_bytes());
    hasher.update(b"|");
    hasher.update(args.as_bytes());
    hasher
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A new random group id made of `[A-Za-z0-9_-]` that is not in `taken`.
pub fn new_group_id(taken: &[&str]) -> Result<String, String> {
    // 64^10 ids: a clash is very unlikely, but retry a few times anyway.
    for _ in 0..8 {
        let mut bytes = [0_u8; GROUP_ID_LEN];
        getrandom::fill(&mut bytes).map_err(|e| format!("random group id: {e}"))?;
        let id: String = bytes
            .iter()
            .map(|b| char::from(GROUP_ID_ALPHABET[usize::from(b % 64)]))
            .collect();
        if !taken.contains(&id.as_str()) {
            return Ok(id);
        }
    }
    Err("could not generate a unique group id".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_is_stable_and_case_insensitive() {
        let a = app_id(r"C:\Windows\notepad.exe", "");
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, app_id(r"c:\windows\NOTEPAD.EXE", ""));
        assert_eq!(a, app_id(r"C:\Windows\notepad.exe", ""));
        assert_ne!(a, app_id(r"C:\Windows\notepad.exe", "file.txt"));
    }

    #[test]
    fn group_id_is_url_safe_and_unique() {
        let first = new_group_id(&[]).unwrap();
        assert_eq!(first.len(), GROUP_ID_LEN);
        assert!(first
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        let second = new_group_id(&[first.as_str()]).unwrap();
        assert_ne!(first, second);
    }
}
