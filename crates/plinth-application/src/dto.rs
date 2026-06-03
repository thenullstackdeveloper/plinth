//! DTOs de salida hacia la UI. Viven aquí, no en el dominio.
//!
//! **Regla del test de omisión del DTO:** `RenderState` describe *qué pintar*,
//! nunca *qué tecla se emite*. No contiene `ScanCode` ni `KeyBinding` ni
//! ningún detalle de resolución. Añadir un campo que filtre eso rompe el test
//! de `tests` y obliga a una decisión consciente.

use plinth_domain::SidebarSide;

/// Una acción tal como la ve la UI: solo su naturaleza, sin teclas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// Acción de app (navegar, lock, sidebar). La UI puede pintarla distinto.
    App,
    /// Acción de juego: la UI solo sabe que "hace algo en el juego".
    Game,
}

/// Un componente listo para pintar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentView {
    pub id: String,
    pub row: u16,
    pub col: u16,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub action_kind: ActionKind,
    /// Estado de toggle (si aplica). La UI lo usa para el resaltado.
    pub toggled: bool,
}

/// Una página lista para pintar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageView {
    pub components: Vec<ComponentView>,
}

/// El estado completo de render: lo único que la UI necesita.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderState {
    pub pages: Vec<PageView>,
    pub current_page: usize,
    pub sidebar: Vec<ComponentView>,
    pub sidebar_side: SidebarSide,
    pub locked: bool,
}
