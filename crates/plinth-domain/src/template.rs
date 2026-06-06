//! The template: root of the first aggregate. Immutable once built.
//!
//! Smart constructors: an invalid `Template` is unconstructable. Business
//! invariants (non-empty pages, unique IDs, in-range `GoToPage` references) are
//! checked here, not in infra.

use crate::action::{Action, AppAction};
use crate::error::DomainError;
use crate::key::{ComponentPosition, SchemaVersion, SidebarSide, VisualProps};

/// Identifier of a component, unique within the template.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentId(String);

impl ComponentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Template traceability metadata (keybind drift across patches).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateMetadata {
    pub game: String,
    pub version: String,
    pub author: String,
    pub schema_version: SchemaVersion,
}

/// An actionable panel component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub id: ComponentId,
    pub position: ComponentPosition,
    pub visual: VisualProps,
    pub action: Action,
}

/// A page: a grid of components. It cannot be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    components: Vec<Component>,
}

impl Page {
    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

/// The sidebar: default side + its components (lock, navigation…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidebar {
    pub default_side: SidebarSide,
    pub components: Vec<Component>,
}

/// The full template. Immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    metadata: TemplateMetadata,
    pages: Vec<Page>,
    sidebar: Sidebar,
}

impl Template {
    /// Builds a template, validating all business invariants.
    pub fn new(
        metadata: TemplateMetadata,
        pages: Vec<Page>,
        sidebar: Sidebar,
    ) -> Result<Self, DomainError> {
        Self::from_parts(metadata, pages, sidebar)
    }

    fn from_parts(
        metadata: TemplateMetadata,
        pages: Vec<Page>,
        sidebar: Sidebar,
    ) -> Result<Self, DomainError> {
        if pages.is_empty() {
            return Err(DomainError::EmptyTemplate);
        }

        // Component IDs unique across the whole template (pages + sidebar).
        let mut seen = std::collections::BTreeSet::new();
        for component in pages
            .iter()
            .flat_map(|p| p.components())
            .chain(sidebar.components.iter())
        {
            if !seen.insert(component.id.clone()) {
                return Err(DomainError::DuplicateComponentId(
                    component.id.as_str().to_string(),
                ));
            }
        }

        // GoToPage references within range.
        let page_count = pages.len();
        for component in pages
            .iter()
            .flat_map(|p| p.components())
            .chain(sidebar.components.iter())
        {
            if let Action::App(AppAction::GoToPage(idx)) = &component.action {
                if *idx >= page_count {
                    return Err(DomainError::InvalidPageReference(*idx));
                }
            }
        }

        Ok(Self {
            metadata,
            pages,
            sidebar,
        })
    }

    pub fn metadata(&self) -> &TemplateMetadata {
        &self.metadata
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn sidebar(&self) -> &Sidebar {
        &self.sidebar
    }

    /// Finds a component by id on any page or in the sidebar.
    pub fn find_component(&self, id: &ComponentId) -> Option<&Component> {
        self.pages
            .iter()
            .flat_map(|p| p.components())
            .chain(self.sidebar.components.iter())
            .find(|c| &c.id == id)
    }
}

/// Page constructor with validation (non-empty).
impl Page {
    pub fn new(components: Vec<Component>) -> Result<Self, DomainError> {
        if components.is_empty() {
            // An empty page makes no sense; we reuse EmptyTemplate at the
            // template level, but a standalone empty page is rejected by its
            // own constructor.
            return Err(DomainError::EmptyTemplate);
        }
        Ok(Self { components })
    }
}
