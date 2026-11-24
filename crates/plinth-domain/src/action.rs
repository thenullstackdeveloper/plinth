//! The `Action` type and its three categories, separated by design.
//!
//! Non-negotiable security rule (see `docs/architecture.md`, security boundary):
//! **there is no `Raw(String)` variant**. "Run an arbitrary command" is
//! *unrepresentable* in the type system. A malicious template cannot describe
//! anything outside this closed catalog.

use std::fmt;

use crate::key::KeyMacro;

/// Opaque semantic identifier of a game action (e.g. `"v_target_lock"`).
/// **The domain does not interpret it**: it is game-agnostic. The legality of an
/// `ActionId` is decided by [`crate::catalog::ActionCatalog`], whose
/// game-specific contents are injected from infra.
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

/// The app's own actions. A **closed** enum: they live inside the engine and are
/// safe by definition — they never touch the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    NextPage,
    PrevPage,
    GoToPage(usize),
    ToggleLock,
    SwitchSidebarSide,
    /// Close the panel. The engine only reports it; the host closes the app.
    Quit,
}

impl AppAction {
    /// Reserved identifiers a template uses to reference an app action. The
    /// catalog recognizes them before game actions. `GoToPage` is not listed
    /// here because it carries a parameter (parsed separately in the infra
    /// mapper).
    pub fn from_reserved(raw: &str) -> Option<Self> {
        match raw {
            "app.next_page" => Some(AppAction::NextPage),
            "app.prev_page" => Some(AppAction::PrevPage),
            "app.toggle_lock" => Some(AppAction::ToggleLock),
            "app.switch_sidebar" => Some(AppAction::SwitchSidebarSide),
            "app.quit" => Some(AppAction::Quit),
            _ => None,
        }
    }
}

/// Game action: a semantic identifier that will be resolved to keys via the
/// `GameKeybindProvider`. The payload is opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAction(pub ActionId);

impl fmt::Display for GameAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The only three action categories. The split by type guarantees, by design,
/// that an app button never injects keys and a key-sending button never
/// mutates state (enforced by `ExecuteAction` in the application layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    App(AppAction),
    /// A semantic game action, resolved to keys by the game's provider.
    Game(GameAction),
    /// Literal keys written in the template (`shift+1`, `[c, b]`): sent as
    /// they are, without a catalog or a provider. Still nothing but key codes.
    Keys(KeyMacro),
}
