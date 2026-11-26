//! `plinth-tauri` — composition root. Wires the concrete adapters into the
//! panel, manages it as Tauri state and opens the panel window.

use std::sync::Mutex;

use plinth_application::ConfigStore;
use plinth_infra::clock::SystemClock;
use plinth_infra::config_fs::FsConfigStore;
use plinth_infra::keybinds_static::StaticKeybindProvider;
use plinth_infra::template_dir::TemplateDir;
use tauri::Manager;

pub mod commands;
#[cfg(test)]
mod packaging;
pub mod panel;
pub mod templates;
pub mod tray;
pub mod window;

use panel::Panel;
use templates::{loads_with_wow_catalog, pick_startup_template, BUNDLED_TEMPLATES};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = TemplateDir::at_default_location()?;
            if let Err(e) = dir.seed(BUNDLED_TEMPLATES) {
                eprintln!("plinth: could not seed the templates folder: {e}");
            }
            let config = FsConfigStore::at_default_location()?;
            let last = config.load().ok().and_then(|prefs| prefs.last_template);
            let startup = pick_startup_template(&dir, last.as_deref(), loads_with_wow_catalog);
            let panel: commands::AppPanel = Panel::new(
                &startup.yaml,
                StaticKeybindProvider::wow_defaults(),
                commands::platform_sender(),
                SystemClock::new(),
                config,
            )?
            .named(startup.name);
            let names = dir.list().unwrap_or_else(|e| {
                eprintln!("plinth: could not list the templates folder: {e}");
                Vec::new()
            });
            let entries = tray::template_menu_entries(
                &names,
                |name| {
                    dir.read(name)
                        .map(|yaml| loads_with_wow_catalog(&yaml))
                        .unwrap_or(false)
                },
                panel.template_name(),
            );
            app.manage(Mutex::new(panel));
            app.manage(dir);
            window::build_panel_window(app)?;
            tray::build_tray(app, &entries)?;
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
