//! Frontend commands for the group flyout window.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::launch_cmds::launch_app;
use crate::flyout::{hide, FlyoutShow, FlyoutState};
use crate::state::Store;

/// Hides the flyout (Escape); `Err` when Windows could not hide it.
#[tauri::command]
#[specta::specta]
pub fn hide_flyout(app: AppHandle) -> Result<(), String> {
    hide::hide(&app)
}

/// The last `flyout:show` payload (`null` before any), for a late page.
#[tauri::command]
#[specta::specta]
pub fn get_flyout(state: State<'_, FlyoutState>) -> Option<FlyoutShow> {
    state.last()
}

/// Launches app `app_id`, then hides the flyout; on failure it stays open.
#[tauri::command]
#[specta::specta]
pub async fn launch_from_flyout(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    flyout: State<'_, FlyoutState>,
    app_id: String,
) -> Result<(), String> {
    // visible_token locks and releases the toggle; nothing is held across
    // the await.
    let before = flyout.visible_token();
    let launched = launch_app(app.clone(), store, app_id).await;
    if hide_after_launch(&launched, before, flyout.visible_token()) {
        // The launch already succeeded, so a failed hide is only logged.
        if let Err(error) = hide::hide(&app) {
            log::warn!("{error}");
        }
    }
    launched
}

/// Whether a finished launch hides the flyout: only when it succeeded and
/// the show it was started from is still the visible one, so a late
/// launch never closes a newer flyout or clears its blur memory.
fn hide_after_launch(launched: &Result<(), String>, before: Option<u64>, now: Option<u64>) -> bool {
    launched.is_ok() && before.is_some() && before == now
}

#[cfg(test)]
mod tests {
    use super::hide_after_launch;

    #[test]
    fn hides_only_after_a_success_from_the_same_visible_show() {
        let ok = Ok(());
        assert!(hide_after_launch(&ok, Some(3), Some(3)));
        // A newer show, a blur or hide, or no show at all: leave it.
        assert!(!hide_after_launch(&ok, Some(3), Some(4)));
        assert!(!hide_after_launch(&ok, Some(3), None));
        assert!(!hide_after_launch(&ok, None, None));
    }

    #[test]
    fn a_failed_launch_never_hides() {
        let failed = Err("App not found: X".to_owned());
        assert!(!hide_after_launch(&failed, Some(3), Some(3)));
    }
}
