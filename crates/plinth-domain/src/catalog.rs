//! El catálogo de acciones: la **autoridad de legalidad** del dominio.
//!
//! El dominio es agnóstico al juego, así que **no hardcodea** ningún ID de
//! ningún juego concreto. En su lugar, el conjunto de `ActionId` legales de
//! juego se *inyecta* en tiempo de composición (lo provee el adaptador de juego
//! en infra vía `GameKeybindProvider::catalog`). Lo que vive aquí es la
//! *autoridad*:
//! el método [`ActionCatalog::resolve`], única puerta por la que una cadena se
//! convierte en `Action`. Una acción desconocida muere aquí y por tanto **no
//! puede llegar nunca a `ExecuteAction`** (es inconstruible).

use std::collections::BTreeSet;

use crate::action::{Action, ActionId, AppAction, GameAction};
use crate::error::DomainError;

/// Conjunto cerrado de acciones de juego legales. Las acciones de app son
/// intrínsecas (enum cerrado) y no necesitan figurar aquí.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionCatalog {
    game_actions: BTreeSet<ActionId>,
}

impl ActionCatalog {
    pub fn from_actions(actions: impl IntoIterator<Item = ActionId>) -> Self {
        Self {
            game_actions: actions.into_iter().collect(),
        }
    }

    /// ¿Es `id` una acción de juego conocida?
    pub fn contains(&self, id: &ActionId) -> bool {
        self.game_actions.contains(id)
    }

    pub fn game_actions(&self) -> impl Iterator<Item = &ActionId> {
        self.game_actions.iter()
    }

    /// Convierte una cadena cruda (de una plantilla) en un `Action` legal.
    ///
    /// Orden de resolución:
    /// 1. Acción de app reservada (`app.*`) → `Action::App`.
    /// 2. `app.go_to_page:<n>` → `Action::App(GoToPage)`.
    /// 3. ID presente en el catálogo de juego → `Action::Game`.
    /// 4. En otro caso → [`DomainError::UnknownAction`].
    ///
    /// Esta es la frontera de seguridad: lo que no encaja, no se construye.
    pub fn resolve(&self, raw: &str) -> Result<Action, DomainError> {
        if let Some(app) = AppAction::from_reserved(raw) {
            return Ok(Action::App(app));
        }
        if let Some(rest) = raw.strip_prefix("app.go_to_page:") {
            let n = rest
                .parse::<usize>()
                .map_err(|_| DomainError::UnknownAction(raw.to_string()))?;
            return Ok(Action::App(AppAction::GoToPage(n)));
        }
        let id = ActionId::new(raw);
        if self.contains(&id) {
            return Ok(Action::Game(GameAction(id)));
        }
        Err(DomainError::UnknownAction(raw.to_string()))
    }
}
