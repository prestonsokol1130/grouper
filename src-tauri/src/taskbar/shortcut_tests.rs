//! Pure tests for group shortcut names (sanitising and collisions) and for
//! picking a path that never replaces another group's shortcut.

use std::path::Path;

use super::shortcut::{sanitise_name, shortcut_file_name, shortcut_name_parts, MAX_NAME_CHARS};
use super::shortcut_write::{choose_path, find_path, Slot};
use crate::state::Group;

fn group(id: &str, name: &str) -> Group {
    Group {
        id: id.to_owned(),
        name: name.to_owned(),
        app_ids: Vec::new(),
        icon_version: 0,
    }
}

#[test]
fn plain_names_are_kept() {
    assert_eq!(sanitise_name("Work", "g1"), "Work");
    assert_eq!(
        sanitise_name("Dev Tools & Games", "g1"),
        "Dev Tools & Games"
    );
    assert_eq!(sanitise_name("Zoë's 📁 apps", "g1"), "Zoë's 📁 apps");
    assert_eq!(sanitise_name("v1.2 tools", "g1"), "v1.2 tools");
}

#[test]
fn invalid_characters_are_removed() {
    assert_eq!(sanitise_name(r#"a<b>c:d"e/f\g|h?i*j"#, "g1"), "abcdefghij");
    assert_eq!(sanitise_name("tab\there\nnew", "g1"), "tabherenew");
}

#[test]
fn trailing_dots_and_spaces_are_stripped() {
    assert_eq!(sanitise_name("Work. . ..", "g1"), "Work");
    assert_eq!(sanitise_name("Work?.", "g1"), "Work");
    // Leading ones are allowed in Windows file names and kept.
    assert_eq!(sanitise_name(".hidden", "g1"), ".hidden");
}

#[test]
fn nothing_left_falls_back_to_the_group_id() {
    for name in ["", "   ", "...", "<>:\"/\\|?*", ". . ?"] {
        assert_eq!(sanitise_name(name, "g1"), "g1", "{name:?}");
    }
}

#[test]
fn reserved_device_names_fall_back_to_the_group_id() {
    for name in ["CON", "con", "Nul", "COM1", "lpt9", "AUX.txt", "PRN ."] {
        assert_eq!(sanitise_name(name, "g1"), "g1", "{name:?}");
    }
    // Only exact device names are reserved.
    assert_eq!(sanitise_name("Console", "g1"), "Console");
    assert_eq!(sanitise_name("COM10", "g1"), "COM10");
}

#[test]
fn long_names_are_cut_without_a_trailing_space() {
    let long = "a".repeat(MAX_NAME_CHARS + 50);
    assert_eq!(sanitise_name(&long, "g1").chars().count(), MAX_NAME_CHARS);
    let spaced = format!("{} b", "a".repeat(MAX_NAME_CHARS - 1));
    assert_eq!(sanitise_name(&spaced, "g1"), "a".repeat(MAX_NAME_CHARS - 1));
}

#[test]
fn distinct_names_need_no_suffix() {
    let groups = [group("g1", "Work"), group("g2", "Play")];
    assert_eq!(
        shortcut_file_name(&groups, "g1").as_deref(),
        Some("Work.lnk")
    );
    assert_eq!(
        shortcut_file_name(&groups, "g2").as_deref(),
        Some("Play.lnk")
    );
}

#[test]
fn colliding_names_get_numbered_suffixes_in_group_order() {
    let groups = [
        group("g1", "Work"),
        group("g2", "work"),
        group("g3", "Work?"),
        group("g4", "Play"),
    ];
    let names: Vec<_> = ["g1", "g2", "g3", "g4"]
        .iter()
        .map(|id| shortcut_file_name(&groups, id))
        .collect();
    assert_eq!(
        names,
        [
            Some("Work.lnk".to_owned()),
            Some("work (2).lnk".to_owned()),
            Some("Work (3).lnk".to_owned()),
            Some("Play.lnk".to_owned()),
        ]
    );
}

#[test]
fn a_suffix_never_takes_another_groups_name() {
    let groups = [
        group("g1", "Work"),
        group("g2", "Work (2)"),
        group("g3", "Work"),
    ];
    assert_eq!(
        shortcut_file_name(&groups, "g2").as_deref(),
        Some("Work (2).lnk")
    );
    assert_eq!(
        shortcut_file_name(&groups, "g3").as_deref(),
        Some("Work (3).lnk")
    );
}

#[test]
fn a_fallback_id_collides_like_any_other_name() {
    let groups = [group("g1", "g2"), group("g2", "???")];
    assert_eq!(shortcut_file_name(&groups, "g1").as_deref(), Some("g2.lnk"));
    assert_eq!(
        shortcut_file_name(&groups, "g2").as_deref(),
        Some("g2 (2).lnk")
    );
}

#[test]
fn unknown_group_has_no_shortcut() {
    let groups = [group("g1", "Work"), group("g2", "Work")];
    assert_eq!(shortcut_file_name(&groups, "nope"), None);
    assert_eq!(shortcut_name_parts(&groups, "nope"), None);
    assert_eq!(
        shortcut_name_parts(&groups, "g2"),
        Some(("Work".to_owned(), 2))
    );
}

/// A slot function over a fixed map of file names; anything else is
/// missing.
fn slots<'a>(map: &'a [(&'a str, Slot)]) -> impl FnMut(&Path) -> Slot + 'a {
    move |path| {
        let name = path.file_name().unwrap().to_str().unwrap();
        map.iter()
            .find(|(file, _)| *file == name)
            .map_or(Slot::Missing, |(_, slot)| *slot)
    }
}

#[test]
fn the_writer_skips_only_foreign_files() {
    let dir = Path::new("menu");
    let choose = |map: &[(&str, Slot)], start| choose_path(dir, "Beta", start, slots(map));
    assert_eq!(choose(&[], 1), Ok(dir.join("Beta.lnk")));
    assert_eq!(
        choose(&[("Beta.lnk", Slot::Ours)], 1),
        Ok(dir.join("Beta.lnk"))
    );
    assert_eq!(
        choose(&[("Beta.lnk", Slot::Unclaimed)], 1),
        Ok(dir.join("Beta.lnk"))
    );
    let taken = [("Beta.lnk", Slot::Foreign), ("Beta (2).lnk", Slot::Foreign)];
    assert_eq!(choose(&taken, 1), Ok(dir.join("Beta (3).lnk")));
    assert_eq!(
        choose(&[("Beta (2).lnk", Slot::Foreign)], 2),
        Ok(dir.join("Beta (3).lnk"))
    );
    let mut all_foreign = |_: &Path| Slot::Foreign;
    assert!(choose_path(dir, "Beta", 1, &mut all_foreign).is_err());
}

#[test]
fn finding_skips_other_groups_and_stops_at_a_gap() {
    let dir = Path::new("menu");
    let find = |map: &[(&str, Slot)]| find_path(dir, "Beta", 1, slots(map));
    assert_eq!(find(&[]), None);
    assert_eq!(
        find(&[("Beta.lnk", Slot::Ours)]),
        Some(dir.join("Beta.lnk"))
    );
    let skipped = [
        ("Beta.lnk", Slot::Foreign),
        ("Beta (2).lnk", Slot::Unclaimed),
        ("Beta (3).lnk", Slot::Ours),
    ];
    assert_eq!(find(&skipped), Some(dir.join("Beta (3).lnk")));
    let gap = [("Beta.lnk", Slot::Foreign), ("Beta (3).lnk", Slot::Ours)];
    assert_eq!(find(&gap), None);
}
