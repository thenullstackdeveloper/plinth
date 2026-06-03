//! El tipo `Action` y sus dos categorías, separadas por diseño.
//!
//! Regla de seguridad innegociable (ver `docs/plinth-contexto.md` §3): **no
//! existe una variante `Raw(String)`**. "Ejecutar comando arbitrario" es
//! *irrepresentable* en el sistema de tipos. Una plantilla maliciosa no puede
//! describir nada que no esté en este catálogo cerrado.

use std::fmt;

/// Identificador semántico opaco de una acción de juego (p. ej.
/// `"v_target_lock"`). **El dominio no lo interpreta**: es agnóstico al juego.
/// La legalidad de un `ActionId` la decide el [`crate::catalog::ActionCatalog`],
/// cuyo contenido específico de cada juego se inyecta desde infra.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(String);

impl ActionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Acciones de la propia app. Enum **cerrado**: viven dentro del motor y son
/// seguras por definición — nunca tocan el juego.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    NextPage,
    PrevPage,
    GoToPage(usize),
    ToggleLock,
    SwitchSidebarSide,
}

impl AppAction {
    /// Identificadores reservados con los que una plantilla referencia una
    /// acción de app. El catálogo los reconoce antes que a las de juego.
    /// `GoToPage` no aparece aquí porque lleva parámetro (se parsea aparte en
    /// el mapper de infra).
    pub fn from_reserved(raw: &str) -> Option<Self> {
        match raw {
            "app.next_page" => Some(AppAction::NextPage),
            "app.prev_page" => Some(AppAction::PrevPage),
            "app.toggle_lock" => Some(AppAction::ToggleLock),
            "app.switch_sidebar" => Some(AppAction::SwitchSidebarSide),
            _ => None,
        }
    }
}

/// Acción de juego: un identificador semántico que se resolverá a teclas vía
/// el `GameKeybindProvider`. El payload es opaco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAction(pub ActionId);

impl fmt::Display for GameAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Las dos únicas categorías de acción. La separación por tipo garantiza, por
/// diseño, que un botón de app nunca inyecta teclas y uno de juego nunca muta
/// estado (lo aplica `ExecuteAction` en la capa de aplicación).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    App(AppAction),
    Game(GameAction),
}
