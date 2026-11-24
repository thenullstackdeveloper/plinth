//! The system tray icon. Its `Templates` submenu picks the template the panel
//! shows, and its `Quit` always closes the panel: it is the way out that
//! never depends on the panel window or on the lock.

use std::sync::Mutex;

use plinth_infra::template_dir::TemplateDir;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::commands::{AppPanel, PANEL_CHANGED_EVENT};

/// Id of the tray menu's quit item.
pub const TRAY_QUIT_ID: &str = "quit";

/// Prefix of a template item's id; the rest is the template's file name.
pub const TEMPLATE_ID_PREFIX: &str = "template:";

/// One template in the `Templates` submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateMenuEntry {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
}

/// The submenu entries for the templates in the folder. `name_loads` gets a
/// file name and says whether that template reads and loads; one that does
/// not is shown disabled as `(invalid)`. The active one is checked.
pub fn template_menu_entries(
    names: &[String],
    name_loads: impl Fn(&str) -> bool,
    active: Option<&str>,
) -> Vec<TemplateMenuEntry> {
    names
        .iter()
        .map(|name| {
            let loads = name_loads(name);
            let stem = name.strip_suffix(".yaml").unwrap_or(name);
            TemplateMenuEntry {
                id: format!("{TEMPLATE_ID_PREFIX}{name}"),
                label: if loads {
                    stem.to_string()
                } else {
                    format!("{stem} (invalid)")
                },
                enabled: loads,
                checked: active == Some(name.as_str()),
            }
        })
        .collect()
}

/// The template file name a menu id points to, if it is a template item.
pub fn template_name_from_menu_id(id: &str) -> Option<&str> {
    id.strip_prefix(TEMPLATE_ID_PREFIX)
}

pub fn build_tray<R: Runtime, M: Manager<R>>(
    manager: &M,
    entries: &[TemplateMenuEntry],
) -> tauri::Result<TrayIcon<R>> {
    let items = entries
        .iter()
        .map(|entry| {
            CheckMenuItem::with_id(
                manager,
                entry.id.as_str(),
                entry.label.as_str(),
                entry.enabled,
                entry.checked,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let item_refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = items
        .iter()
        .map(|item| item as &dyn tauri::menu::IsMenuItem<R>)
        .collect();
    let templates =
        Submenu::with_id_and_items(manager, "templates", "Templates", true, &item_refs)?;
    let quit = MenuItem::with_id(manager, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(manager, &[&templates, &quit])?;

    let mut builder = TrayIconBuilder::new().tooltip("Plinth").menu(&menu);
    if let Some(icon) = manager.app_handle().default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            if id == TRAY_QUIT_ID {
                app.exit(0);
            } else if let Some(name) = template_name_from_menu_id(id) {
                choose_template(app, name, &items);
            }
        })
        .build(manager)
}

/// Switches the panel to the template `name` from the folder, tells the UI to
/// redraw if it worked, and re-checks every item against the active template
/// (a click flips a check item on its own, whatever happened).
fn choose_template<R: Runtime>(app: &AppHandle<R>, name: &str, items: &[CheckMenuItem<R>]) {
    let dir = app.state::<TemplateDir>();
    let panel_state = app.state::<Mutex<AppPanel>>();
    let Ok(mut panel) = panel_state.lock() else {
        eprintln!("plinth: the panel state is poisoned");
        return;
    };
    let switched = dir.read(name).map_err(|e| e.to_string()).and_then(|yaml| {
        panel
            .switch_template(name, &yaml)
            .map_err(|e| e.to_string())
    });
    match switched {
        Ok(()) => {
            if let Err(e) = app.emit(PANEL_CHANGED_EVENT, ()) {
                eprintln!("plinth: could not tell the UI the panel changed: {e}");
            }
        }
        Err(e) => eprintln!("plinth: could not switch to `{name}`: {e}"),
    }
    let active = panel
        .template_name()
        .map(|n| format!("{TEMPLATE_ID_PREFIX}{n}"));
    for item in items {
        let checked = active.as_deref() == Some(item.id().as_ref());
        if let Err(e) = item.set_checked(checked) {
            eprintln!("plinth: could not update the templates menu: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn entries_mark_the_active_template() {
        let entries = template_menu_entries(
            &names(&["wow-default-keys.yaml", "wow-demo.yaml"]),
            |_| true,
            Some("wow-demo.yaml"),
        );
        let checked: Vec<bool> = entries.iter().map(|e| e.checked).collect();
        assert_eq!(checked, vec![false, true]);
        assert_eq!(entries[1].id, "template:wow-demo.yaml");
        assert!(entries.iter().all(|e| e.enabled));
    }

    #[test]
    fn entries_disable_invalid_templates() {
        let entries = template_menu_entries(
            &names(&["broken.yaml", "wow-demo.yaml"]),
            |name| name != "broken.yaml",
            None,
        );
        assert_eq!(
            entries[0],
            TemplateMenuEntry {
                id: "template:broken.yaml".into(),
                label: "broken (invalid)".into(),
                enabled: false,
                checked: false,
            }
        );
        assert!(entries[1].enabled);
    }

    #[test]
    fn entries_label_drops_the_extension() {
        let entries = template_menu_entries(&names(&["wow-default-keys.yaml"]), |_| true, None);
        assert_eq!(entries[0].label, "wow-default-keys");
    }

    #[test]
    fn menu_id_roundtrips_to_the_template_name() {
        assert_eq!(
            template_name_from_menu_id("template:wow-demo.yaml"),
            Some("wow-demo.yaml")
        );
        assert_eq!(template_name_from_menu_id(TRAY_QUIT_ID), None);
        assert_eq!(template_name_from_menu_id("templates"), None);
    }
}
