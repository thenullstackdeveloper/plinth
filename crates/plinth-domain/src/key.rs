//! Objetos de valor relacionados con teclas y secuencias.
//!
//! El dominio razona en términos de **scancodes físicos** (la tecla de la fila
//! superior izquierda, no la letra que produce): el compositor aplica el layout
//! por encima. Los juegos suelen vincular por tecla física, así que esto encaja
//! (ver `docs/plinth-fase0-analisis.md` §2). Aquí solo guardamos el número del
//! código; traducir el token de cada juego → `ScanCode` es trabajo de la capa
//! de infraestructura, no del dominio.

use std::time::Duration;

use crate::error::DomainError;

/// Un scancode físico (p. ej. la tecla "A" == 30). Newtype opaco: el dominio no
/// interpreta el valor, solo lo transporta hasta la capa de input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScanCode(pub u16);

impl ScanCode {
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Cómo se acciona una tecla dentro de una secuencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStepKind {
    /// Pulsar y soltar inmediatamente.
    Press,
    /// Mantener pulsada durante una duración y soltar.
    Hold(Duration),
    /// Alternar: el estado lo lleva el `RuntimeState`; la capa de aplicación
    /// decide si emite pulsación o no según el estado previo.
    Toggle,
}

/// Un paso de una secuencia: una tecla con su modo de accionamiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyStep {
    pub code: ScanCode,
    pub kind: KeyStepKind,
}

impl KeyStep {
    pub const fn press(code: ScanCode) -> Self {
        Self {
            code,
            kind: KeyStepKind::Press,
        }
    }

    pub const fn hold(code: ScanCode, dur: Duration) -> Self {
        Self {
            code,
            kind: KeyStepKind::Hold(dur),
        }
    }

    pub const fn toggle(code: ScanCode) -> Self {
        Self {
            code,
            kind: KeyStepKind::Toggle,
        }
    }
}

/// Una secuencia de teclas no vacía. Constructor inteligente: una secuencia
/// vacía es inconstruible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySequence(Vec<KeyStep>);

impl KeySequence {
    pub fn new(steps: Vec<KeyStep>) -> Result<Self, DomainError> {
        if steps.is_empty() {
            return Err(DomainError::EmptyKeySequence);
        }
        Ok(Self(steps))
    }

    /// Atajo para la secuencia de una sola pulsación (el caso más común).
    pub fn single_press(code: ScanCode) -> Self {
        Self(vec![KeyStep::press(code)])
    }

    pub fn steps(&self) -> &[KeyStep] {
        &self.0
    }
}

/// Una tecla resuelta a partir de la configuración real del juego: el
/// resultado de `GameKeybindProvider::resolve`. Es una `KeySequence` ya lista
/// para emitirse.
pub type KeyBinding = KeySequence;

/// Lado en el que vive la barra lateral. Para diestros/zurdos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSide {
    Left,
    Right,
}

impl SidebarSide {
    pub fn flip(self) -> Self {
        match self {
            SidebarSide::Left => SidebarSide::Right,
            SidebarSide::Right => SidebarSide::Left,
        }
    }
}

/// Versión del esquema de plantilla con el que se creó. Trazabilidad de la
/// deriva de keybinds entre parches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion(pub String);

/// Posición de un componente dentro de su página (rejilla lógica).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentPosition {
    pub row: u16,
    pub col: u16,
}

/// Propiedades puramente visuales de un componente (etiqueta, icono…). El
/// dominio no las interpreta; las transporta hacia la UI vía el DTO.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VisualProps {
    pub label: Option<String>,
    pub icon: Option<String>,
}
