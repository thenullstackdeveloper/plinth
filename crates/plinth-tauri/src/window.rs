//! The panel window. Its one hard requirement: clicking it must never take
//! keyboard focus from the game. On Windows `focusable(false)` becomes
//! `WS_EX_NOACTIVATE`, and `focused(false)` shows it without activating it.

use tauri::{Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub struct PanelWindowSpec {
    pub label: &'static str,
    pub title: &'static str,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub decorations: bool,
    pub always_on_top: bool,
    pub focusable: bool,
    pub focused: bool,
    pub skip_taskbar: bool,
}

pub fn panel_window_spec() -> PanelWindowSpec {
    PanelWindowSpec {
        label: "main",
        // Same title as the Phase 0 spike window.
        title: "Plinth",
        x: 100.0,
        y: 100.0,
        // Room for the sidebar and a page; provisional until the touchscreen.
        width: 900.0,
        height: 300.0,
        decorations: false,
        always_on_top: true,
        focusable: false,
        focused: false,
        skip_taskbar: true,
    }
}

/// Creates the panel window, applying every field of [`panel_window_spec`].
pub fn build_panel_window<R: Runtime, M: Manager<R>>(
    manager: &M,
) -> tauri::Result<WebviewWindow<R>> {
    let spec = panel_window_spec();
    WebviewWindowBuilder::new(manager, spec.label, WebviewUrl::App("index.html".into()))
        .title(spec.title)
        .position(spec.x, spec.y)
        .inner_size(spec.width, spec.height)
        .decorations(spec.decorations)
        .always_on_top(spec.always_on_top)
        .focusable(spec.focusable)
        .focused(spec.focused)
        .skip_taskbar(spec.skip_taskbar)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_window_spec_never_takes_focus() {
        let spec = panel_window_spec();
        assert!(!spec.focusable);
        assert!(!spec.focused);
        assert!(spec.always_on_top);
        assert!(!spec.decorations);
        assert!(spec.skip_taskbar);
        assert_eq!(spec.label, "main");
        assert_eq!(spec.title, "Plinth");
    }

    #[test]
    fn panel_window_spec_fits_sidebar_and_page() {
        let spec = panel_window_spec();
        assert_eq!(spec.width, 900.0);
        assert_eq!(spec.height, 300.0);
        assert_eq!(spec.x, 100.0);
        assert_eq!(spec.y, 100.0);
    }
}
