//! Domain errors. They speak the product's language, never infrastructure's
//! (no file paths, no input-layer types, etc.).

use crate::action::ActionId;

/// Everything that can go wrong inside the game-agnostic core.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// An action string belongs neither to the game catalog nor to the closed
    /// set of app actions. An `Action` cannot be constructed.
    #[error("unknown action: `{0}` (not in the catalog)")]
    UnknownAction(String),

    /// The game action exists in the catalog but the user has not bound any key
    /// to it. Distinct from `UnknownAction`.
    #[error("action `{0}` has no key bound to it")]
    ActionNotBound(ActionId),

    /// Attempt to go to a page out of range.
    #[error("page out of range: requested {requested}, max {max}")]
    PageOutOfRange { requested: usize, max: usize },

    /// Navigation rejected because the lock is engaged.
    #[error("lock engaged: navigation blocked")]
    LockEngaged,

    /// A template with no pages is not representable.
    #[error("the template has no pages")]
    EmptyTemplate,

    /// Two components share an identifier within the template.
    #[error("duplicate component identifier: `{0}`")]
    DuplicateComponentId(String),

    /// An empty key sequence makes no sense.
    #[error("empty key sequence")]
    EmptyKeySequence,

    /// A literal macro longer than a single tap may fire.
    #[error("macro too long: {len} steps, max {max}")]
    MacroTooLong { len: usize, max: usize },

    /// A key code appears twice in one chord (`shift+shift+1`).
    #[error("key {code} appears twice in a chord")]
    InvalidChord { code: u16 },

    /// A component that does not exist in the template was referenced.
    #[error("unknown component: `{0}`")]
    UnknownComponent(String),

    /// A `GoToPage` action points to an index the template does not have.
    #[error("invalid page reference: {0}")]
    InvalidPageReference(usize),
}
