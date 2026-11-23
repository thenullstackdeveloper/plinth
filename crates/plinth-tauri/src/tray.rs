//! The system tray icon. Its `Quit` always closes the panel: it is the way out
//! that never depends on the panel window or on the lock.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{Manager, Runtime};

/// Id of the tray menu's quit item.
pub const TRAY_QUIT_ID: &str = "quit";

pub fn build_tray<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<TrayIcon<R>> {
    let quit = MenuItem::with_id(manager, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(manager, &[&quit])?;
    let mut builder = TrayIconBuilder::new().tooltip("Plinth").menu(&menu);
    if let Some(icon) = manager.app_handle().default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .on_menu_event(|app, event| {
            if event.id().as_ref() == TRAY_QUIT_ID {
                app.exit(0);
            }
        })
        .build(manager)
}
