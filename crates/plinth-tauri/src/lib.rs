//! `plinth-tauri` — composition root. Wires the concrete adapters into the
//! panel, manages it as Tauri state and opens the panel window.

use std::sync::Mutex;

use plinth_infra::clock::SystemClock;
use plinth_infra::config_fs::FsConfigStore;
use plinth_infra::keybinds_static::StaticKeybindProvider;
use tauri::Manager;

pub mod commands;
pub mod panel;
pub mod tray;
pub mod window;

use panel::{Panel, WOW_DEFAULT_KEYS_TEMPLATE};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let panel: commands::AppPanel = Panel::new(
                WOW_DEFAULT_KEYS_TEMPLATE,
                StaticKeybindProvider::wow_defaults(),
                commands::platform_sender(),
                SystemClock::new(),
                FsConfigStore::at_default_location()?,
            )?;
            app.manage(Mutex::new(panel));
            window::build_panel_window(app)?;
            tray::build_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::render_state,
            commands::press,
            commands::navigate
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Plinth panel");
}
