//! `RuntimeState`: la **única entidad mutable** del dominio. Raíz del segundo
//! agregado. Página actual, candado, toggles.

use std::collections::HashMap;

use crate::error::DomainError;
use crate::key::SidebarSide;
use crate::template::ComponentId;

/// Dirección de navegación entre páginas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Next,
    Prev,
}

/// Estado mutable del panel en ejecución.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeState {
    current_page: usize,
    locked: bool,
    sidebar_side: SidebarSide,
    toggles: HashMap<ComponentId, bool>,
}

impl RuntimeState {
    pub fn new(sidebar_side: SidebarSide) -> Self {
        Self {
            current_page: 0,
            locked: false,
            sidebar_side,
            toggles: HashMap::new(),
        }
    }

    pub fn current_page(&self) -> usize {
        self.current_page
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn sidebar_side(&self) -> SidebarSide {
        self.sidebar_side
    }

    pub fn toggle_state(&self, id: &ComponentId) -> bool {
        self.toggles.get(id).copied().unwrap_or(false)
    }

    /// Navega una página. **Invariante:** rechazado con el candado activo, y
    /// no se sale del rango `[0, page_count)`.
    pub fn navigate(&mut self, dir: NavDirection, page_count: usize) -> Result<(), DomainError> {
        if self.locked {
            return Err(DomainError::LockEngaged);
        }
        let target = match dir {
            NavDirection::Next => self.current_page + 1,
            NavDirection::Prev => {
                if self.current_page == 0 {
                    return Err(DomainError::PageOutOfRange {
                        requested: 0,
                        max: page_count.saturating_sub(1),
                    });
                }
                self.current_page - 1
            }
        };
        self.go_to(target, page_count)
    }

    /// Salta a una página concreta. Misma invariante de candado y rango.
    pub fn go_to(&mut self, idx: usize, page_count: usize) -> Result<(), DomainError> {
        if self.locked {
            return Err(DomainError::LockEngaged);
        }
        if idx >= page_count {
            return Err(DomainError::PageOutOfRange {
                requested: idx,
                max: page_count.saturating_sub(1),
            });
        }
        self.current_page = idx;
        Ok(())
    }

    /// Alterna el candado. Nunca falla: bloquear/desbloquear siempre es legal.
    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
    }

    /// Cambia la sidebar de lado. Siempre legal.
    pub fn switch_sidebar(&mut self) {
        self.sidebar_side = self.sidebar_side.flip();
    }

    /// Alterna el estado de un toggle y devuelve el **nuevo** estado.
    pub fn flip_toggle(&mut self, id: &ComponentId) -> bool {
        let entry = self.toggles.entry(id.clone()).or_insert(false);
        *entry = !*entry;
        *entry
    }
}
