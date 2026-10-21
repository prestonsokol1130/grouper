//! Command-line parsing for launches and single-instance forwarding.

/// What a launch (or a forwarded second launch) asks Grouper to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchCommand {
    /// Open the flyout for one taskbar group.
    Flyout { group_id: String },
    /// Open the fullscreen Launchpad.
    Launchpad,
    /// Open the Manager window.
    Manager,
    /// Windows sign-in (`--background`): no window, not even the welcome.
    Background,
    /// The uninstaller (`--cleanup`, optionally `--delete-data`); handled
    /// before the app starts, see [`crate::cleanup`].
    Cleanup { delete_data: bool },
    /// Hidden `--bench flyout`: show and hide the flyout 100 times, timed.
    BenchFlyout,
    /// Nothing to do; stay in the tray.
    None,
}

/// Parses process arguments. `args[0]` is the executable and is skipped.
///
/// The first recognised flag wins and unknown flags are ignored. `--flyout`
/// needs a following id made only of `[A-Za-z0-9_-]` (it may start with
/// `--`) that is not itself one of the known flags; anything else yields
/// [`LaunchCommand::None`] and a log warning.
pub fn parse_args(args: &[String]) -> LaunchCommand {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--flyout" => return parse_flyout(rest.next()),
            "--launchpad" => return LaunchCommand::Launchpad,
            "--manager" => return LaunchCommand::Manager,
            BACKGROUND_FLAG => return LaunchCommand::Background,
            CLEANUP_FLAG => {
                let delete_data = rest.any(|arg| arg == DELETE_DATA_FLAG);
                return LaunchCommand::Cleanup { delete_data };
            }
            // Only the exact value counts; anything else does nothing.
            BENCH_FLAG if rest.next().is_some_and(|value| value == "flyout") => {
                return LaunchCommand::BenchFlyout;
            }
            BENCH_FLAG => return LaunchCommand::None,
            _ => {}
        }
    }
    LaunchCommand::None
}

/// The flag the Windows sign-in (Run key) launch passes.
pub const BACKGROUND_FLAG: &str = "--background";

/// The flag the uninstaller passes. A later [`DELETE_DATA_FLAG`] also
/// deletes the app data.
pub const CLEANUP_FLAG: &str = "--cleanup";

/// With [`CLEANUP_FLAG`]: the user ticked "Delete the application data".
pub const DELETE_DATA_FLAG: &str = "--delete-data";

/// The hidden measuring flag; see [`LaunchCommand::BenchFlyout`].
const BENCH_FLAG: &str = "--bench";

/// The flags [`parse_args`] knows; one after `--flyout` is a missing id.
const KNOWN_FLAGS: [&str; 6] = [
    "--flyout",
    "--launchpad",
    "--manager",
    BACKGROUND_FLAG,
    CLEANUP_FLAG,
    BENCH_FLAG,
];

/// Validates the id that follows `--flyout`. Group ids may start with `--`
/// (`-` is in their alphabet), so only the exact known flags are refused.
fn parse_flyout(value: Option<&String>) -> LaunchCommand {
    match value {
        None => {
            log::warn!("--flyout needs a group id; ignoring");
            LaunchCommand::None
        }
        Some(id) if KNOWN_FLAGS.contains(&id.as_str()) => {
            log::warn!("--flyout needs a group id but got flag {id:?}; ignoring");
            LaunchCommand::None
        }
        Some(id) if is_valid_group_id(id) => LaunchCommand::Flyout {
            group_id: id.clone(),
        },
        Some(id) => {
            log::warn!("--flyout group id {id:?} has invalid characters; ignoring");
            LaunchCommand::None
        }
    }
}

/// True when `id` is non-empty and only uses ASCII letters, digits, `_` or `-`.
fn is_valid_group_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("Grouper.exe")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn valid_flyout() {
        assert_eq!(
            parse_args(&args(&["--flyout", "test-1_A"])),
            LaunchCommand::Flyout {
                group_id: "test-1_A".into()
            }
        );
    }

    #[test]
    fn flyout_missing_id() {
        assert_eq!(parse_args(&args(&["--flyout"])), LaunchCommand::None);
    }

    #[test]
    fn flyout_empty_id() {
        assert_eq!(parse_args(&args(&["--flyout", ""])), LaunchCommand::None);
    }

    #[test]
    fn flyout_followed_by_flag_is_missing_id() {
        assert_eq!(
            parse_args(&args(&["--flyout", "--manager"])),
            LaunchCommand::None
        );
        assert_eq!(
            parse_args(&args(&["--flyout", "--launchpad"])),
            LaunchCommand::None
        );
        assert_eq!(
            parse_args(&args(&["--flyout", "--flyout"])),
            LaunchCommand::None
        );
    }

    #[test]
    fn flyout_id_may_start_with_dashes() {
        assert_eq!(
            parse_args(&args(&["--flyout", "--ab_cdE12"])),
            LaunchCommand::Flyout {
                group_id: "--ab_cdE12".into()
            }
        );
    }

    #[test]
    fn flyout_bad_characters() {
        for bad in ["a b", "a/b", "../x", "é", "id;rm"] {
            assert_eq!(
                parse_args(&args(&["--flyout", bad])),
                LaunchCommand::None,
                "{bad}"
            );
        }
    }

    #[test]
    fn multiple_flags_first_wins() {
        assert_eq!(
            parse_args(&args(&["--launchpad", "--manager"])),
            LaunchCommand::Launchpad
        );
        assert_eq!(
            parse_args(&args(&["--manager", "--flyout", "g1"])),
            LaunchCommand::Manager
        );
        assert_eq!(
            parse_args(&args(&["--flyout", "g1", "--launchpad"])),
            LaunchCommand::Flyout {
                group_id: "g1".into()
            }
        );
    }

    #[test]
    fn unknown_flags_ignored() {
        assert_eq!(
            parse_args(&args(&["--verbose", "x", "--manager"])),
            LaunchCommand::Manager
        );
    }

    #[test]
    fn background_launch() {
        assert_eq!(
            parse_args(&args(&["--background"])),
            LaunchCommand::Background
        );
        assert_eq!(
            parse_args(&args(&["--flyout", "--background"])),
            LaunchCommand::None
        );
    }

    #[test]
    fn cleanup_with_and_without_data() {
        let cleanup = |delete_data| LaunchCommand::Cleanup { delete_data };
        assert_eq!(parse_args(&args(&["--cleanup"])), cleanup(false));
        let with_data = args(&["--cleanup", "--delete-data"]);
        assert_eq!(parse_args(&with_data), cleanup(true));
        let data_first = args(&["--delete-data", "--cleanup"]);
        assert_eq!(parse_args(&data_first), cleanup(false));
        let as_id = args(&["--flyout", "--cleanup"]);
        assert_eq!(parse_args(&as_id), LaunchCommand::None);
    }

    #[test]
    fn bench_needs_the_exact_value() {
        assert_eq!(
            parse_args(&args(&["--bench", "flyout"])),
            LaunchCommand::BenchFlyout
        );
        for bad in [&["--bench"][..], &["--bench", "Flyout"], &["--bench", "x"]] {
            assert_eq!(parse_args(&args(bad)), LaunchCommand::None, "{bad:?}");
        }
        assert_eq!(
            parse_args(&args(&["--flyout", "--bench"])),
            LaunchCommand::None
        );
    }

    #[test]
    fn no_args() {
        assert_eq!(parse_args(&args(&[])), LaunchCommand::None);
        assert_eq!(parse_args(&[]), LaunchCommand::None);
    }

    #[test]
    fn argv0_is_skipped() {
        let argv = vec!["--manager".to_string()];
        assert_eq!(parse_args(&argv), LaunchCommand::None);
    }
}
