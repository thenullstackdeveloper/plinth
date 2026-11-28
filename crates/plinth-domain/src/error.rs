//! Domain errors. They speak the product's language, never infrastructure's
//! (no input-layer types, no paths of the user's disk). The only paths they
//! name are those a template writes, relative to its own folder.

use crate::action::ActionId;
use crate::image::ImageUse;
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

    /// Two components of the same page cover at least one same cell of its
    /// grid; `a` is the one the page lists first.
    #[error("components `{a}` and `{b}` share a cell on page {page}")]
    CellsOverlap { page: usize, a: String, b: String },

    /// The grid or the sidebar needs more room than the panel has.
    #[error(
        "the theme does not fit the panel: {area} needs {needed}px but only {available}px are available"
    )]
    ThemeDoesNotFit {
        area: &'static str,
        needed: u32,
        available: u32,
    },

    /// An image path a template may not use: see `ImagePath`.
    #[error("`{path}` is not a valid image path: {reason}")]
    InvalidImagePath { path: String, reason: &'static str },

    /// An image larger than its use allows, by its header or its size.
    #[error("`{path}` is too large for {use_}: {measured} > {limit}")]
    ImageTooLarge {
        path: String,
        use_: ImageUse,
        measured: String,
        limit: String,
    },

    /// A template whose distinct image files are more than 64, or more than
    /// 16 MiB between them. `files` and `bytes` are what it reached.
    #[error("the template's images exceed 64 files or 16 MiB")]
    TemplateAssetsTooLarge { files: usize, bytes: u64 },

    /// A component points to an image the template does not have.
    #[error("image {0} is not one of the template's images")]
    UnknownImage(usize),

    /// A button with both an `icon` and an `image`: it shows one or the
    /// other.
    #[error("`{component}` has both an icon and an image; a button shows one or the other")]
    IconWithImage { component: String },

    /// The quit button with a look of its own (`icon`, `image`, `style`,
    /// `hide_label`), which could disguise it.
    #[error("the quit button cannot have `{field}`, so it always looks the same")]
    NotAllowedOnQuit { field: &'static str },

    /// A button's `style` naming a token other than its four colours
    /// (`STYLE_TOKENS`).
    #[error(
        "theme token `{token}` cannot be set on one button; its `style` only takes `color.button.background`, `color.button.text`, `color.button.border` and `color.button.pressed-background`"
    )]
    StyleTokenNotAllowed { token: String },

    /// A sidebar button with something only a page button may have.
    #[error("`{component}` is in the sidebar, which does not allow `{field}`")]
    NotAllowedInSidebar {
        field: &'static str,
        component: String,
    },
}
