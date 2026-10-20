//! Decides which Start Menu shortcuts are real apps. Pure: the only outside
//! check (is the target a file) is passed in by the caller.

use std::fmt;
use std::path::Path;

/// Words and phrases that mark a shortcut as not an app (uninstallers,
/// installers, documentation, web links). Matched case-insensitively as
/// whole words against the shortcut name and the target's file name.
pub const REJECT_WORDS: &[&str] = &[
    "uninstall",
    "uninst",
    "unins000",
    "remove",
    "setup",
    "read me",
    "readme",
    "help",
    "documentation",
    "manual",
    "license",
    "release notes",
    "what's new",
    "website",
    "support",
];

/// Target extensions that are documents or web links, not programs.
pub const REJECT_EXTENSIONS: &[&str] = &[".url", ".html", ".htm", ".txt", ".pdf", ".chm"];

/// Why a shortcut was not kept as an app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// The shortcut has no file-system target (for example a Store app).
    EmptyTarget,
    /// The shortcut name contains this reject word.
    NameWord(&'static str),
    /// The target's file name contains this reject word.
    TargetWord(&'static str),
    /// The target ends with this document or web-link extension.
    Extension(&'static str),
    /// The target is not an existing file (missing, or a folder).
    NotAFile,
    /// The target is a web page (`http:` or `https:`).
    WebLink,
    /// One of Grouper's own group shortcuts.
    OwnShortcut,
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTarget => write!(f, "no file-system target"),
            Self::NameWord(word) => write!(f, "name contains {word:?}"),
            Self::TargetWord(word) => write!(f, "target file name contains {word:?}"),
            Self::Extension(ext) => write!(f, "target is a {ext} file"),
            Self::NotAFile => write!(f, "target is not a file"),
            Self::WebLink => write!(f, "target is a web page"),
            Self::OwnShortcut => write!(f, "a Grouper group shortcut"),
        }
    }
}

/// Checks one shortcut with display name `name` and target path `target`.
/// `target_is_file` (normally [`is_file`]) is only called when every other
/// check has passed.
pub fn check(
    name: &str,
    target: &str,
    target_is_file: impl FnOnce(&str) -> bool,
) -> Result<(), Rejection> {
    let target = target.trim();
    if target.is_empty() {
        return Err(Rejection::EmptyTarget);
    }
    if let Some(word) = find_word(name) {
        return Err(Rejection::NameWord(word));
    }
    if let Some(word) = find_word(file_name(target)) {
        return Err(Rejection::TargetWord(word));
    }
    let lower = target.to_lowercase();
    if let Some(ext) = REJECT_EXTENSIONS.iter().find(|ext| lower.ends_with(*ext)) {
        return Err(Rejection::Extension(ext));
    }
    if !target_is_file(target) {
        return Err(Rejection::NotAFile);
    }
    Ok(())
}

/// Checks only the display name, for an app with no file target (a Store
/// app, a `steam:` game, a Shell folder).
pub fn check_name(name: &str) -> Result<(), Rejection> {
    find_word(name).map_or(Ok(()), |word| Err(Rejection::NameWord(word)))
}

/// True for an `http:` or `https:` address (any letter case).
pub fn is_web_link(target: &str) -> bool {
    let lower = target.trim().to_ascii_lowercase();
    lower.starts_with("http:") || lower.starts_with("https:")
}

/// True when `target` is an existing file on disk (not a folder), the
/// predicate the scanner passes to [`check`]. A network (UNC) path such as
/// `\\server\share\app.exe` or `//server/share/app.exe` is trusted without
/// being checked, so a scan never waits on the network.
pub fn is_file(target: &str) -> bool {
    if target.starts_with(r"\\") || target.starts_with("//") {
        return true;
    }
    Path::new(target).is_file()
}

/// The last component of a Windows or Unix style path.
fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// The first reject word or phrase found in `text` as whole words.
fn find_word(text: &str) -> Option<&'static str> {
    let haystack = words(text);
    REJECT_WORDS.iter().copied().find(|phrase| {
        let phrase = words(phrase);
        !phrase.is_empty()
            && haystack
                .windows(phrase.len())
                .any(|window| window == phrase)
    })
}

/// `text` lowercased and split into words on anything that is not a letter
/// or digit, so "What's New" becomes `["what", "s", "new"]`.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}
