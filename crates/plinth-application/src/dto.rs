//! Output DTOs toward the UI. They live here, not in the domain.
//!
//! **DTO-omission test rule:** `RenderState` describes *what to render*, never
//! *which key is emitted*. It contains no `ScanCode`, no `KeyBinding`, no
//! resolution detail. Adding a field that leaks that breaks the `tests` test and
//! forces a conscious decision.

use plinth_domain::SidebarSide;

/// An action as the UI sees it: only its nature, no keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// App action (navigate, lock, sidebar). The UI may render it differently.
    App,
    /// Game action: the UI only knows it "does something in the game".
    Game,
    /// Literal keys from the template. Like `Game`, the UI never sees which.
    Keys,
}

/// A component ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentView {
    pub id: String,
    pub row: u16,
    pub col: u16,
    /// The rows and columns it covers from `row` and `col`; 1 and 1 for a
    /// button of one cell, as every sidebar button is.
    pub row_span: u16,
    pub col_span: u16,
    pub label: Option<String>,
    /// The icon next to the label, as an id of the template's images like
    /// `image_id`.
    pub icon_id: Option<String>,
    /// The image that fills the button, as `t<generation>i<index>`: an id of
    /// the template's images, never their bytes.
    pub image_id: Option<String>,
    /// The label is hidden from view; it stays the button's name.
    pub hide_label: bool,
    /// The button's own colours as CSS custom properties: only those its
    /// `style` gives, so the rest keep the theme. Empty when it has none.
    pub style: Vec<ThemeVar>,
    pub action_kind: ActionKind,
    /// Toggle state (if applicable). The UI uses it for highlighting.
    pub toggled: bool,
}

/// A page ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageView {
    pub components: Vec<ComponentView>,
    /// Grid rows: the largest component `row + row_span`.
    pub rows: u32,
    /// Grid columns: the largest component `col + col_span`.
    pub cols: u32,
    /// The background the page shows, its own or the template's default, as
    /// an id of the template's images like `image_id`.
    pub background_id: Option<String>,
}

/// One design token as a CSS custom property: `--plinth-color-button-background`
/// and `#262a3b`. Presentation only, so it may cross to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeVar {
    pub name: String,
    pub value: String,
}

/// The full render state: everything the UI needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderState {
    pub pages: Vec<PageView>,
    pub current_page: usize,
    pub sidebar: Vec<ComponentView>,
    pub sidebar_side: SidebarSide,
    pub locked: bool,
    /// The template's resolved theme, every token of the catalog.
    pub theme: Vec<ThemeVar>,
    /// How long the change of page takes, from `motion.page-transition`.
    pub page_transition_ms: u16,
}
