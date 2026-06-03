//! La plantilla: raíz del primer agregado. Inmutable una vez construida.
//!
//! Constructores inteligentes: un `Template` inválido es inconstruible. Las
//! invariantes de negocio (páginas no vacías, IDs únicos, referencias de
//! `GoToPage` en rango) se comprueban aquí, no en infra.

use crate::action::{Action, AppAction};
use crate::error::DomainError;
use crate::key::{ComponentPosition, SchemaVersion, SidebarSide, VisualProps};

/// Identificador de un componente, único dentro de la plantilla.
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

/// Metadatos de trazabilidad de la plantilla (deriva de keybinds entre parches).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateMetadata {
    pub game: String,
    pub version: String,
    pub author: String,
    pub schema_version: SchemaVersion,
}

/// Un componente accionable del panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub id: ComponentId,
    pub position: ComponentPosition,
    pub visual: VisualProps,
    pub action: Action,
}

/// Una página: una rejilla de componentes. No puede estar vacía.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    components: Vec<Component>,
}

impl Page {
    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

/// La barra lateral: lado por defecto + sus componentes (candado, navegación…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidebar {
    pub default_side: SidebarSide,
    pub components: Vec<Component>,
}

/// La plantilla completa. Inmutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    metadata: TemplateMetadata,
    pages: Vec<Page>,
    sidebar: Sidebar,
}

impl Template {
    /// Construye una plantilla validando todas las invariantes de negocio.
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

        // IDs de componente únicos en toda la plantilla (páginas + sidebar).
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

        // Referencias de GoToPage dentro de rango.
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

    /// Busca un componente por id en cualquier página o en la sidebar.
    pub fn find_component(&self, id: &ComponentId) -> Option<&Component> {
        self.pages
            .iter()
            .flat_map(|p| p.components())
            .chain(self.sidebar.components.iter())
            .find(|c| &c.id == id)
    }
}

/// Constructor de páginas con validación (no vacía).
impl Page {
    pub fn new(components: Vec<Component>) -> Result<Self, DomainError> {
        if components.is_empty() {
            // Una página vacía no tiene sentido; reutilizamos EmptyTemplate
            // a nivel de plantilla, pero una página suelta vacía la rechaza
            // su propio constructor.
            return Err(DomainError::EmptyTemplate);
        }
        Ok(Self { components })
    }
}
