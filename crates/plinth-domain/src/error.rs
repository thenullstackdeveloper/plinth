//! Domain errors. They speak the product's language, never infrastructure's
//! (no file paths, no input-layer types, etc.).

use crate::action::ActionId;
use crate::theme::TokenKind;

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

    /// A token name that is not 2 to 4 dotted `[a-z][a-z0-9-]*` segments.
    #[error("invalid theme token path `{0}`")]
    InvalidTokenPath(String),

    /// The theme catalog defines the same token twice.
    #[error("theme token `{0}` is defined twice")]
    DuplicateToken(String),

    /// A theme names a token the catalog does not have.
    #[error("unknown theme token `{0}`")]
    UnknownToken(String),

    /// A theme gives a token a value of the wrong kind.
    #[error("theme token `{token}` expects {expected}")]
    TokenKindMismatch { token: String, expected: TokenKind },

    /// A theme changes a token the catalog keeps fixed.
    #[error("theme token `{0}` is fixed and cannot be changed by a template")]
    TokenNotThemeable(String),

    /// A value outside the bounds the catalog allows for its token.
    #[error("theme token `{token}`: {value} is outside the allowed range {min} to {max}")]
    TokenOutOfRange {
        token: String,
        value: String,
        min: String,
        max: String,
    },

    /// A font family list that is empty or holds a name that is not plain
    /// letters, digits, spaces and dashes.
    #[error("theme token `{0}` needs a non-empty list of plain font names")]
    InvalidFontFamily(String),
}
