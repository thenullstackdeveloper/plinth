//! The template: root of the first aggregate. Immutable once built.
//!
//! Smart constructors: an invalid `Template` is unconstructable. Business
//! invariants (non-empty pages, unique IDs, in-range `GoToPage` references) are
//! checked here, not in infra.

use crate::action::{Action, AppAction};
use crate::error::DomainError;
use crate::key::{ComponentPosition, SchemaVersion, SidebarSide, VisualProps};
use crate::theme::{GridSize, PanelSize, Theme, LAYOUT_TOKENS};

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

    /// The rows and columns the page spans: its largest `row` and `col`, plus one.
    pub fn grid_size(&self) -> GridSize {
        let span = |pick: fn(&Component) -> u16| {
            self.components
                .iter()
                .map(|c| u32::from(pick(c)) + 1)
                .max()
                .unwrap_or(0)
        };
        GridSize {
            rows: span(|c| c.position.row),
            cols: span(|c| c.position.col),
        }
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
    /// The resolved theme. Empty until `with_theme` gives it one.
    theme: Theme,
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

        // One cell per button on a page. The sidebar is a column ordered by
        // `row`, so it is not checked.
        for (index, page) in pages.iter().enumerate() {
            let mut cells = std::collections::BTreeMap::new();
            for component in page.components() {
                let cell = (component.position.row, component.position.col);
                if let Some(first) = cells.insert(cell, &component.id) {
                    return Err(DomainError::DuplicatePosition {
                        page: index,
                        row: cell.0,
                        col: cell.1,
                        first: first.as_str().to_string(),
                        second: component.id.as_str().to_string(),
                    });
                }
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
            theme: Theme::default(),
        })
    }

    /// The same template with its resolved theme.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Whether every page's grid and the sidebar fit `panel` with the theme's
    /// layout tokens. A template that does not fit is rejected, never cropped.
    pub fn check_fit(&self, panel: PanelSize) -> Result<(), DomainError> {
        let mut px = [0u32; LAYOUT_TOKENS.len()];
        for (value, path) in px.iter_mut().zip(LAYOUT_TOKENS) {
            *value = self
                .theme
                .px(path)
                .map(u32::from)
                .ok_or_else(|| DomainError::UnknownToken(path.to_string()))?;
        }
        let [touch, button, sidebar_width, gap, pad, sidebar_gap, sidebar_pad] = px;
        let (width, height) = (u32::from(panel.width), u32::from(panel.height));

        for page in &self.pages {
            let grid = page.grid_size();
            fits(
                "page grid height",
                span(grid.rows, button, gap, pad),
                height,
            )?;
            fits(
                "page grid width",
                span(grid.cols, button, gap, pad),
                width.saturating_sub(sidebar_width),
            )?;
        }
        let buttons = u32::try_from(self.sidebar.components.len()).unwrap_or(u32::MAX);
        if buttons > 0 {
            fits(
                "sidebar height",
                span(buttons, touch, sidebar_gap, sidebar_pad),
                height,
            )?;
        }
        fits(
            "sidebar width",
            touch,
            sidebar_width.saturating_sub(sidebar_pad.saturating_mul(2)),
        )
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

/// The room `count` buttons of `size` need in a line: the buttons, the gaps
/// between them and the padding on both ends. `count` is at least 1.
fn span(count: u32, size: u32, gap: u32, pad: u32) -> u32 {
    count
        .saturating_mul(size)
        .saturating_add(count.saturating_sub(1).saturating_mul(gap))
        .saturating_add(pad.saturating_mul(2))
}

fn fits(area: &'static str, needed: u32, available: u32) -> Result<(), DomainError> {
    if needed <= available {
        Ok(())
    } else {
        Err(DomainError::ThemeDoesNotFit {
            area,
            needed,
            available,
        })
    }
}
