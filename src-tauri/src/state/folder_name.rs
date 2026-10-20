//! The name a new Launchpad folder gets. Pure.

use std::collections::HashSet;

/// The name used when the apps share no word.
pub const DEFAULT_FOLDER_NAME: &str = "Folder";

/// The most words a suggested name keeps.
const MAX_NAME_WORDS: usize = 3;

/// Words that are never a name on their own (they may sit inside one, as
/// in "Age of Empires").
const FILLER: &[&str] = &[
    "a", "an", "and", "app", "apps", "by", "for", "in", "of", "on", "the", "to", "with", "x64",
    "x86",
];

/// The words of `name`: runs of letters and digits.
fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// A word that can carry a name: two or more characters, not only digits
/// (a version or year) and not filler.
fn significant(word: &str) -> bool {
    let lower = word.to_lowercase();
    word.chars().count() >= 2
        && !word.chars().all(|c| c.is_numeric())
        && !FILLER.contains(&lower.as_str())
}

/// Suggests a name for a folder made of apps called `names`.
///
/// The name is the first run of consecutive words in the first app's name
/// that every other name also has (ignoring case), at most
/// [`MAX_NAME_WORDS`] long, starting and ending on a significant word,
/// with the first name's spelling. "Microsoft Word" and "Microsoft Excel"
/// give "Microsoft"; "Visual Studio Code" and "Visual Studio 2022" give
/// "Visual Studio". Fewer than two names, or no shared significant word,
/// give [`DEFAULT_FOLDER_NAME`].
pub fn suggest_folder_name<S: AsRef<str>>(names: &[S]) -> String {
    let Some((first, others)) = names.split_first() else {
        return DEFAULT_FOLDER_NAME.into();
    };
    if others.is_empty() {
        return DEFAULT_FOLDER_NAME.into();
    }
    let others: Vec<HashSet<String>> = others
        .iter()
        .map(|name| words(name.as_ref()).map(str::to_lowercase).collect())
        .collect();
    let shared = |word: &str| {
        let lower = word.to_lowercase();
        others.iter().all(|set| set.contains(&lower))
    };
    let mut run: Vec<&str> = Vec::new();
    for word in words(first.as_ref()) {
        let starts = run.is_empty() && significant(word);
        if shared(word) && (starts || !run.is_empty()) && run.len() < MAX_NAME_WORDS {
            run.push(word);
        } else if !run.is_empty() {
            break;
        }
    }
    while run.last().is_some_and(|word| !significant(word)) {
        run.pop();
    }
    if run.is_empty() {
        DEFAULT_FOLDER_NAME.into()
    } else {
        run.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(names: &[&str]) -> String {
        suggest_folder_name(names)
    }

    #[test]
    fn a_shared_word_names_the_folder() {
        assert_eq!(name(&["Microsoft Word", "Microsoft Excel"]), "Microsoft");
        assert_eq!(name(&["Google Chrome", "Chrome Remote Desktop"]), "Chrome");
        assert_eq!(name(&["Word", "Excel", "Microsoft"]), "Folder");
    }

    #[test]
    fn consecutive_shared_words_stay_together() {
        assert_eq!(
            name(&["Visual Studio Code", "Visual Studio 2022"]),
            "Visual Studio"
        );
        assert_eq!(
            name(&["Age of Empires II", "Age of Empires III"]),
            "Age of Empires"
        );
        assert_eq!(
            name(&["Adobe Photoshop 2024", "Adobe Illustrator 2024"]),
            "Adobe"
        );
    }

    #[test]
    fn case_and_punctuation_are_ignored_and_the_first_spelling_kept() {
        assert_eq!(name(&["Steam (Valve)", "steam-link"]), "Steam");
        assert_eq!(name(&["Notepad++", "Notepad"]), "Notepad");
    }

    #[test]
    fn filler_numbers_and_single_letters_never_name_a_folder() {
        assert_eq!(name(&["The App 2", "The App 2"]), "Folder");
        assert_eq!(name(&["Tool for X", "Kit for X"]), "Folder");
        assert_eq!(name(&["Game of Life", "Book of Life"]), "Life");
    }

    #[test]
    fn the_name_is_capped_and_needs_two_apps() {
        assert_eq!(
            name(&["One Two Three Four", "One Two Three Four"]),
            "One Two Three"
        );
        assert_eq!(name(&["Microsoft Word"]), "Folder");
        assert_eq!(name(&[]), "Folder");
        assert_eq!(name(&["", ""]), "Folder");
    }
}
