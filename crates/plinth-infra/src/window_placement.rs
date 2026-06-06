//! Window placement.
//!
//! On Linux/Hyprland it is a **deliberate no-op**: the panel's position,
//! "always-on-top", and "no-focus" are handled by compositor rules
//! (`windowrulev2`), not the app's code, because Tauri does not control the
//! window under Wayland (see `docs/input-linux.md` and `docs/hyprland-rules.md`).
//! On Windows/macOS this *seam* will host real code using native window APIs
//! (future).

use plinth_application::{PlacementError, WindowPlacement};

#[derive(Default)]
pub struct LinuxWindowPlacement;

impl LinuxWindowPlacement {
    pub fn new() -> Self {
        Self
    }
}

impl WindowPlacement for LinuxWindowPlacement {
    fn place(&self) -> Result<(), PlacementError> {
        // No-op: placement lives in the Hyprland rules.
        Ok(())
    }
}
