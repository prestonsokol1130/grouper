//! Finds the `.lnk` files under one Start Menu folder.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

/// How many folder levels below a root are searched.
pub const MAX_DEPTH: usize = 6;

/// Grouper's own Start Menu folder, holding the group shortcuts it creates.
/// It is never scanned, so those shortcuts never show up as apps.
pub const OWN_FOLDER: &str = "Grouper";

/// What [`find_shortcuts`] found under one root.
#[derive(Debug)]
pub struct Walked {
    pub shortcuts: Vec<PathBuf>,
    /// False when part of the tree (or the root itself) could not be read,
    /// so `shortcuts` may be missing some.
    pub complete: bool,
}

/// Every `.lnk` file under `root`, at most [`MAX_DEPTH`] levels deep, sorted
/// by path. Symbolic links are not followed (or returned), and the
/// [`OWN_FOLDER`] directly under `root` is skipped. A folder that cannot be
/// read is logged and skipped, and marks the result incomplete.
pub fn find_shortcuts(root: &Path) -> Walked {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(MAX_DEPTH)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !is_own_folder(entry));
    let mut found = Walked {
        shortcuts: Vec::new(),
        complete: true,
    };
    for entry in walker {
        match entry {
            Ok(entry) if entry.file_type().is_file() && is_lnk(entry.path()) => {
                found.shortcuts.push(entry.into_path());
            }
            Ok(_) => {}
            Err(error) => {
                found.complete = false;
                log::warn!("could not read part of {}: {error}", root.display());
            }
        }
    }
    found
}

/// True for Grouper's own folder directly under the root (any letter case).
fn is_own_folder(entry: &DirEntry) -> bool {
    entry.depth() == 1
        && entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.eq_ignore_ascii_case(OWN_FOLDER))
}

/// True when `path` has a `.lnk` extension (any letter case).
fn is_lnk(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn collects_lnk_files_and_skips_own_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(&root.join("Notepad.lnk"));
        touch(&root.join("Vendor").join("App.LNK"));
        touch(&root.join("Vendor").join("readme.txt"));
        touch(&root.join("Grouper").join("Group.lnk"));
        touch(&root.join("grouper").join("Other.lnk"));
        touch(&root.join("Vendor").join("Grouper").join("Nested.lnk"));
        let found = find_shortcuts(root);
        assert!(found.complete);
        let names: Vec<_> = found
            .shortcuts
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_path_buf())
            .collect();
        assert!(names.contains(&PathBuf::from("Notepad.lnk")));
        assert!(names.contains(&Path::new("Vendor").join("App.LNK")));
        // Only our own folder directly under the root is skipped.
        assert!(names.contains(&Path::new("Vendor").join("Grouper").join("Nested.lnk")));
        assert!(!names.iter().any(|p| p.starts_with("Grouper")));
        assert!(!names.iter().any(|p| p.starts_with("grouper")));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn stops_at_max_depth() {
        let dir = tempfile::tempdir().unwrap();
        let mut deep = dir.path().to_path_buf();
        for level in 1..=MAX_DEPTH {
            touch(&deep.join(format!("level{level}.lnk")));
            deep.push(format!("d{level}"));
        }
        touch(&deep.join("too-deep.lnk"));
        let found = find_shortcuts(dir.path()).shortcuts;
        assert_eq!(found.len(), MAX_DEPTH);
        assert!(!found.iter().any(|p| p.ends_with("too-deep.lnk")));
    }

    #[test]
    fn missing_root_is_empty_and_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let found = find_shortcuts(&dir.path().join("absent"));
        assert!(found.shortcuts.is_empty());
        assert!(!found.complete);
    }

    #[test]
    fn empty_root_is_complete() {
        let dir = tempfile::tempdir().unwrap();
        let found = find_shortcuts(dir.path());
        assert!(found.shortcuts.is_empty());
        assert!(found.complete);
    }
}
