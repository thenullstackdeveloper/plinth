//! `RuntimeState`: the **single mutable entity** of the domain. Root of the
//! second aggregate. Current page, lock, toggles.

use std::collections::HashMap;

use crate::error::DomainError;
use crate::key::SidebarSide;
use crate::template::ComponentId;

/// Navigation direction between pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Next,
    Prev,
}

/// Mutable state of the running panel.
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

    /// Navigates one page. **Invariant:** rejected while the lock is engaged, and
    /// never leaves the range `[0, page_count)`.
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

    /// Jumps to a specific page. Same lock and range invariant.
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

    /// Toggles the lock. Never fails: locking/unlocking is always legal.
    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
    }

    /// Switches the sidebar side. Always legal.
    pub fn switch_sidebar(&mut self) {
        self.sidebar_side = self.sidebar_side.flip();
    }

    /// Flips a toggle's state and returns the **new** state.
    pub fn flip_toggle(&mut self, id: &ComponentId) -> bool {
        let entry = self.toggles.entry(id.clone()).or_insert(false);
        *entry = !*entry;
        *entry
    }
}
