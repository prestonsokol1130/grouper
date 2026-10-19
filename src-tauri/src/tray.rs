//! The system tray icon and its menu.

use tauri::menu::{MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::windows::show_window;

const OPEN_MANAGER: &str = "open-manager";
const OPEN_LAUNCHPAD: &str = "open-launchpad";
const QUIT: &str = "quit";

/// Creates the tray icon with its menu. Left-click opens the Manager.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let manager = MenuItemBuilder::with_id(OPEN_MANAGER, "Open Manager")
        .build(app)
        .map_err(|e| format!("tray menu item: {e}"))?;
    let launchpad = MenuItemBuilder::with_id(OPEN_LAUNCHPAD, "Open Launchpad")
        .build(app)
        .map_err(|e| format!("tray menu item: {e}"))?;
    let quit = MenuItemBuilder::with_id(QUIT, "Quit Grouper")
        .build(app)
        .map_err(|e| format!("tray menu item: {e}"))?;
    let menu = MenuBuilder::new(app)
        .items(&[&manager, &launchpad])
        .separator()
        .item(&quit)
        .build()
        .map_err(|e| format!("tray menu: {e}"))?;

    let mut builder = TrayIconBuilder::with_id("grouper")
        .tooltip("Grouper")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_event);
    match app.default_window_icon() {
        Some(icon) => builder = builder.icon(icon.clone()),
        None => log::warn!("no app icon found; the tray icon may be blank"),
    }
    builder
        .build(app)
        .map(|_| ())
        .map_err(|e| format!("tray icon: {e}"))
}

/// Handles clicks on the tray menu items.
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    // show_window logs its own failures.
    match event.id().as_ref() {
        OPEN_MANAGER => {
            let _ = show_window(app, "manager");
        }
        OPEN_LAUNCHPAD => {
            let _ = show_window(app, "launchpad");
        }
        QUIT => {
            log::info!("quit requested from tray");
            app.exit(0);
        }
        other => log::debug!("unhandled tray menu item {other:?}"),
    }
}

/// Opens the Manager when the tray icon is left-clicked.
fn handle_tray_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let _ = show_window(tray.app_handle(), "manager");
    }
}
