//! Colocación de ventana.
//!
//! En Linux/Hyprland es **no-op a propósito**: la posición, el "siempre
//! encima" y el "sin foco" del panel los gestionan reglas del compositor
//! (`windowrulev2`), no el código de la app, porque Tauri no controla la
//! ventana bajo Wayland (ver `docs/plinth-fase0-analisis.md` §3 y
//! `docs/hyprland-rules.md`). En Windows/macOS este *seam* alojará código real
//! con APIs nativas de ventana (futuro).

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
        // No-op: la colocación vive en las reglas de Hyprland.
        Ok(())
    }
}
