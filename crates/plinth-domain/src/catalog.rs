//! The action catalog: the domain's **legality authority**.
//!
//! The domain is game-agnostic, so it **does not hardcode** any ID from any
//! specific game. Instead, the set of legal game `ActionId`s is *injected* at
//! composition time (provided by the game adapter in infra via
//! `GameKeybindProvider::catalog`). What lives here is the *authority*:
//! the [`ActionCatalog::resolve`] method, the single gate through which a string
//! becomes an `Action`. An unknown action dies here and therefore **can never
//! reach `ExecuteAction`** (it is unconstructable).

use std::collections::BTreeSet;

use crate::action::{Action, ActionId, AppAction, GameAction};
use crate::error::DomainError;

/// Closed set of legal game actions. App actions are intrinsic (a closed enum)
/// and need not be listed here.
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

    /// Is `id` a known game action?
    pub fn contains(&self, id: &ActionId) -> bool {
        self.game_actions.contains(id)
    }

    pub fn game_actions(&self) -> impl Iterator<Item = &ActionId> {
        self.game_actions.iter()
    }

    /// Turns a raw string (from a template) into a legal `Action`.
    ///
    /// Resolution order:
    /// 1. Reserved app action (`app.*`) → `Action::App`.
    /// 2. `app.go_to_page:<n>` → `Action::App(GoToPage)`.
    /// 3. ID present in the game catalog → `Action::Game`.
    /// 4. Otherwise → [`DomainError::UnknownAction`].
    ///
    /// This is the security boundary: what doesn't fit isn't constructed.
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
