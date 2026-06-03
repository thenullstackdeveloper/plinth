//! Puertos de salida: **traits definidos por el consumidor**. La capa de
//! aplicación declara aquí lo que necesita del mundo; infra los implementa.
//! La dirección de la dependencia se invierte: infra → application, nunca al
//! revés.

use std::time::Duration;

use plinth_domain::{ActionCatalog, GameAction, KeyBinding, ScanCode, Template};

/// Valor de un evento de tecla a nivel de input: pulsada o soltada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyValue {
    Down,
    Up,
}

/// Un evento de input atómico que la capa de aplicación entrega a `InputSender`.
/// Es deliberadamente de bajo nivel: el *timing* (holds) lo orquesta el
/// `SequenceRunner` usando el `Clock`, no el adaptador, para que sea testeable
/// con un reloj falso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: ScanCode,
    pub value: KeyValue,
}

impl KeyEvent {
    pub fn down(code: ScanCode) -> Self {
        Self {
            code,
            value: KeyValue::Down,
        }
    }

    pub fn up(code: ScanCode) -> Self {
        Self {
            code,
            value: KeyValue::Up,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("error de la capa de input: {0}")]
pub struct InputError(pub String);

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("la acción `{0}` existe pero no tiene tecla asignada")]
    NotBound(GameAction),
    #[error("error del proveedor de keybinds: {0}")]
    Provider(String),
}

#[derive(Debug, thiserror::Error)]
#[error("error refrescando keybinds: {0}")]
pub struct ProviderError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("error cargando la plantilla: {0}")]
pub struct TemplateError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("error de configuración: {0}")]
pub struct ConfigError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("error de colocación de ventana: {0}")]
pub struct PlacementError(pub String);

/// Emite eventos de input al sistema. *(Peso arquitectónico real:
/// portabilidad por SO; la impl concreta depende del sistema operativo.)*
pub trait InputSender {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError>;
}

/// Resuelve acciones de juego a teclas leyendo la configuración real del
/// juego. *(Peso real: aquí vive TODO lo específico de SC. Juego #2 = nueva
/// impl; el núcleo no cambia.)*
pub trait GameKeybindProvider {
    fn resolve(&self, action: &GameAction) -> Result<KeyBinding, ResolveError>;
    fn refresh(&mut self) -> Result<(), ProviderError>;
    /// El catálogo de acciones conocidas. Alimenta la autoridad del dominio en
    /// el composition root.
    fn catalog(&self) -> ActionCatalog;
}

/// Carga una plantilla **ya validada** (forma + catálogo + invariantes). El
/// `serde`/parseo queda interno a infra: este puerto solo expone el agregado
/// de dominio.
pub trait TemplateSource {
    fn load(&self, reference: &str) -> Result<Template, TemplateError>;
}

/// Reloj inyectable: timing determinista en tests.
pub trait Clock {
    fn sleep(&self, dur: Duration);
}

/// Preferencias persistidas del usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    pub sidebar_side: plinth_domain::SidebarSide,
    pub last_template: Option<String>,
}

/// Almacén de preferencias. *(Ligero; colapsable a llamada directa de infra.)*
pub trait ConfigStore {
    fn load(&self) -> Result<Prefs, ConfigError>;
    fn save(&self, prefs: &Prefs) -> Result<(), ConfigError>;
}

/// Colocación de la ventana del panel. *Seam diseñado, no construido*: en
/// Linux es no-op (la ventana la gestionan reglas de Hyprland); en Win/Mac
/// será código real (futuro).
pub trait WindowPlacement {
    fn place(&self) -> Result<(), PlacementError>;
}
