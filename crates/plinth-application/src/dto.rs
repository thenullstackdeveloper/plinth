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
    pub label: Option<String>,
    pub icon: Option<String>,
    pub action_kind: ActionKind,
    /// Toggle state (if applicable). The UI uses it for highlighting.
    pub toggled: bool,
}

/// A page ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageView {
    pub components: Vec<ComponentView>,
    /// Grid rows: the largest component `row` plus one.
    pub rows: u32,
    /// Grid columns: the largest component `col` plus one.
    pub cols: u32,
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
