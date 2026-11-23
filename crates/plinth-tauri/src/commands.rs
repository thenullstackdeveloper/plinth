//! Tauri commands: thin input adapters. They lock the managed panel, call it
//! and translate errors to strings. No logic lives here.

use std::sync::Mutex;

use plinth_infra::clock::SystemClock;
use plinth_infra::keybinds_static::StaticKeybindProvider;
use tauri::State;

use crate::panel::{NavDirectionDto, Panel, PanelError, RenderView};

/// The input adapter of the running platform. Only Windows sends real keys;
/// elsewhere the panel logs them.
#[cfg(target_os = "windows")]
pub type Sender = plinth_infra::input_windows::WindowsInputSender;
#[cfg(not(target_os = "windows"))]
pub type Sender = plinth_infra::input_log::LogInputSender<std::io::Stderr>;

#[cfg(target_os = "windows")]
pub fn platform_sender() -> Sender {
    plinth_infra::input_windows::WindowsInputSender::new()
}

#[cfg(not(target_os = "windows"))]
pub fn platform_sender() -> Sender {
    plinth_infra::input_log::stderr()
}

/// The panel as the app manages it. The same type is used in `manage` and in
/// every `State`.
pub type AppPanel = Panel<Sender, StaticKeybindProvider, SystemClock>;

#[tauri::command]
pub fn render_state(panel: State<'_, Mutex<AppPanel>>) -> Result<RenderView, String> {
    let panel = panel.lock().map_err(|e| e.to_string())?;
    Ok(panel.render())
}

/// Answers with the view after the press. A press the lock or a page edge
/// refused is not an error for the UI: it gets the unchanged view.
#[tauri::command]
pub async fn press(
    component_id: String,
    panel: State<'_, Mutex<AppPanel>>,
) -> Result<RenderView, String> {
    let mut panel = panel.lock().map_err(|e| e.to_string())?;
    match panel.press(&component_id) {
        Ok(()) | Err(PanelError::Blocked(_)) => Ok(panel.render()),
        Err(e) => Err(e.to_string()),
    }
}

/// A swipe. Same answer as [`press`].
#[tauri::command]
pub async fn navigate(
    direction: NavDirectionDto,
    panel: State<'_, Mutex<AppPanel>>,
) -> Result<RenderView, String> {
    let mut panel = panel.lock().map_err(|e| e.to_string())?;
    match panel.navigate(direction) {
        Ok(()) | Err(PanelError::Blocked(_)) => Ok(panel.render()),
        Err(e) => Err(e.to_string()),
    }
}
