//! Outbound ports: **consumer-defined traits**. The application layer declares
//! here what it needs from the world; infra implements them. The dependency
//! direction is inverted: infra → application, never the other way around.

use std::time::Duration;

use plinth_domain::{ActionCatalog, GameAction, KeyBinding, ScanCode, Template};

/// Value of a key event at the input level: pressed or released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyValue {
    Down,
    Up,
}

/// An atomic input event the application layer hands to `InputSender`. It is
/// deliberately low-level: timing (holds) is orchestrated by the
/// `SequenceRunner` using the `Clock`, not by the adapter, so it is testable
/// with a fake clock.
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
#[error("input layer error: {0}")]
pub struct InputError(pub String);

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("action `{0}` exists but has no key bound to it")]
    NotBound(GameAction),
    #[error("keybind provider error: {0}")]
    Provider(String),
}

#[derive(Debug, thiserror::Error)]
#[error("error refreshing keybinds: {0}")]
pub struct ProviderError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("error loading the template: {0}")]
pub struct TemplateError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("configuration error: {0}")]
pub struct ConfigError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("window placement error: {0}")]
pub struct PlacementError(pub String);

/// Emits input events to the system. *(Real architectural weight: per-OS
/// portability; the concrete impl depends on the operating system.)*
pub trait InputSender {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError>;
}

/// Resolves game actions to keys by reading the game's real configuration.
/// *(Real weight: everything SC-specific lives here. Game #2 = a new impl; the
/// core does not change.)*
pub trait GameKeybindProvider {
    fn resolve(&self, action: &GameAction) -> Result<KeyBinding, ResolveError>;
    fn refresh(&mut self) -> Result<(), ProviderError>;
    /// The catalog of known actions. Feeds the domain's authority at the
    /// composition root.
    fn catalog(&self) -> ActionCatalog;
}

/// Loads an **already-validated** template (shape + catalog + invariants). The
/// `serde`/parsing stays internal to infra: this port only exposes the domain
/// aggregate.
pub trait TemplateSource {
    fn load(&self, reference: &str) -> Result<Template, TemplateError>;
}

/// Injectable clock: deterministic timing in tests.
pub trait Clock {
    fn sleep(&self, dur: Duration);
}

/// Persisted user preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    pub sidebar_side: plinth_domain::SidebarSide,
    pub last_template: Option<String>,
}

/// Preferences store. *(Lightweight; collapsible to a direct infra call.)*
pub trait ConfigStore {
    fn load(&self) -> Result<Prefs, ConfigError>;
    fn save(&self, prefs: &Prefs) -> Result<(), ConfigError>;
}

/// Placement of the panel window. *A designed seam, not built out*: on Linux it
/// is a no-op (the window is managed by Hyprland rules); on Win/Mac it will be
/// real code (future).
pub trait WindowPlacement {
    fn place(&self) -> Result<(), PlacementError>;
}
