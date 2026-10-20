//! Whether a `--flyout <id>` request shows or hides the flyout: a small
//! pure state machine, no Tauri.
//!
//! Clicking a pinned group's taskbar button while its flyout is open first
//! moves focus to the taskbar, so the flyout hides on blur, and only then
//! does the shell's `Grouper.exe --flyout <id>` arrive. Without care, that
//! second click would show the flyout again. So a blur remembers which
//! group it hid and when, and a request for that same group within
//! [`BLUR_TOGGLE_WINDOW`] is treated as the toggle-off it was meant to be.

use std::time::{Duration, Instant};

/// How long after a blur-hide a request for the same group still counts as
/// the second click of a toggle.
pub const BLUR_TOGGLE_WINDOW: Duration = Duration::from_millis(300);

/// What to do with a flyout request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Show the flyout for the group (switching groups if another is open).
    Show,
    /// The group's flyout is open: hide it.
    Hide,
    /// The group's flyout was just hidden by the click's blur: leave it
    /// hidden.
    StayHidden,
}

/// The flyout's visibility as far as toggling cares.
#[derive(Debug, Clone, Default)]
pub struct Toggle {
    /// The group whose flyout is showing, if any.
    shown: Option<String>,
    /// The group last hidden by a blur, and when.
    blurred: Option<(String, Instant)>,
}

impl Toggle {
    /// Decides what a request for `group_id` at `now` does, and records
    /// the result as if it were carried out. After [`Decision::Show`] the
    /// caller must report the outcome with [`Toggle::shown`] or
    /// [`Toggle::hidden`].
    pub fn request(&mut self, group_id: &str, now: Instant) -> Decision {
        if self.shown.as_deref() == Some(group_id) {
            self.hidden();
            return Decision::Hide;
        }
        if let Some((blurred, at)) = self.blurred.take() {
            // A clock that went backwards counts as "just now".
            let since = now.saturating_duration_since(at);
            if blurred == group_id && since <= BLUR_TOGGLE_WINDOW {
                return Decision::StayHidden;
            }
        }
        Decision::Show
    }

    /// The flyout for `group_id` is now showing.
    pub fn shown(&mut self, group_id: &str) {
        self.shown = Some(group_id.to_owned());
        self.blurred = None;
    }

    /// The flyout was hidden on purpose (Escape, a launch, a toggle or a
    /// failed show); a later request always shows it again.
    pub fn hidden(&mut self) {
        self.shown = None;
        self.blurred = None;
    }

    /// The flyout lost focus at `now`. Returns true if it was showing and
    /// so must be hidden. A blur while already hidden (for example the one
    /// that follows a deliberate hide) changes nothing.
    pub fn blurred(&mut self, now: Instant) -> bool {
        match self.shown.take() {
            Some(group_id) => {
                self.blurred = Some((group_id, now));
                true
            }
            None => false,
        }
    }

    /// The group whose flyout is showing, if any.
    pub fn visible_group(&self) -> Option<&str> {
        self.shown.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    fn shown(group: &str) -> Toggle {
        let mut t = Toggle::default();
        t.shown(group);
        t
    }

    #[test]
    fn first_request_shows() {
        let mut t = Toggle::default();
        assert_eq!(t.request("a", Instant::now()), Decision::Show);
        assert_eq!(t.visible_group(), None, "not shown until reported");
    }

    #[test]
    fn second_request_for_the_open_group_hides() {
        let mut t = shown("a");
        assert_eq!(t.request("a", Instant::now()), Decision::Hide);
        assert_eq!(t.visible_group(), None);
        assert_eq!(t.request("a", Instant::now()), Decision::Show);
    }

    #[test]
    fn request_for_another_group_switches() {
        let mut t = shown("a");
        assert_eq!(t.request("b", Instant::now()), Decision::Show);
        t.shown("b");
        assert_eq!(t.visible_group(), Some("b"));
    }

    #[test]
    fn click_on_the_taskbar_button_toggles_off_after_blur() {
        let start = Instant::now();
        let mut t = shown("a");
        assert!(t.blurred(start));
        assert_eq!(t.request("a", start + 120 * MS), Decision::StayHidden);
        assert_eq!(t.visible_group(), None);
        // The third click opens it again.
        assert_eq!(t.request("a", start + 900 * MS), Decision::Show);
    }

    #[test]
    fn blur_window_edges() {
        let start = Instant::now();
        let mut t = shown("a");
        t.blurred(start);
        assert_eq!(
            t.request("a", start + BLUR_TOGGLE_WINDOW),
            Decision::StayHidden
        );
        let mut t = shown("a");
        t.blurred(start);
        assert_eq!(
            t.request("a", start + BLUR_TOGGLE_WINDOW + MS),
            Decision::Show
        );
    }

    #[test]
    fn blur_memory_is_used_once() {
        let start = Instant::now();
        let mut t = shown("a");
        t.blurred(start);
        assert_eq!(t.request("a", start + MS), Decision::StayHidden);
        assert_eq!(t.request("a", start + 2 * MS), Decision::Show);
    }

    #[test]
    fn blur_then_another_group_shows_it() {
        let start = Instant::now();
        let mut t = shown("a");
        t.blurred(start);
        assert_eq!(t.request("b", start + 50 * MS), Decision::Show);
        // The memory of "a" is gone, so "a" opens normally afterwards.
        assert_eq!(t.request("a", start + 60 * MS), Decision::Show);
    }

    #[test]
    fn blur_while_hidden_changes_nothing() {
        let start = Instant::now();
        let mut t = Toggle::default();
        assert!(!t.blurred(start));
        let mut t = shown("a");
        t.hidden();
        // The blur that follows a deliberate hide must not arm a toggle.
        assert!(!t.blurred(start));
        assert_eq!(t.request("a", start + MS), Decision::Show);
    }

    #[test]
    fn clock_going_backwards_counts_as_just_now() {
        let start = Instant::now() + 10 * MS;
        let mut t = shown("a");
        t.blurred(start);
        assert_eq!(t.request("a", start - 5 * MS), Decision::StayHidden);
    }

    #[test]
    fn showing_clears_blur_memory() {
        let start = Instant::now();
        let mut t = shown("a");
        t.blurred(start);
        t.shown("b");
        t.blurred(start + MS);
        assert_eq!(t.request("a", start + 2 * MS), Decision::Show);
    }
}
