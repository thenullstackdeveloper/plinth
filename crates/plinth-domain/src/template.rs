//! The template: root of the first aggregate. Immutable once built.
//!
//! Smart constructors: an invalid `Template` is unconstructable. Business
//! invariants (non-empty pages, unique IDs, in-range `GoToPage` references) are
//! checked here, not in infra.

use crate::action::{Action, AppAction};
use crate::error::DomainError;
use crate::image::{ImageAsset, ImageRef};
use crate::key::{ComponentPosition, SchemaVersion, SidebarSide, VisualProps};
use crate::theme::{GridSize, PanelSize, Theme, ThemeOverrides, LAYOUT_TOKENS};

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
    /// The button's own colours, over the template's theme: only the tokens
    /// its `style` gives. Empty for a button that has none.
    pub style: ThemeOverrides,
    pub action: Action,
}

/// A page: a grid of components. It cannot be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    components: Vec<Component>,
    /// The page's own background, which replaces the template's default.
    background: Option<ImageRef>,
}

impl Page {
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// The same page with its own background, or none to take the
    /// template's default.
    pub fn with_background(mut self, background: Option<ImageRef>) -> Self {
        self.background = background;
        self
    }

    /// The rows and columns the page spans: the largest `row + row_span` and
    /// `col + col_span`, where its last buttons end.
    pub fn grid_size(&self) -> GridSize {
        let end = |pick: fn(&ComponentPosition) -> (u16, u16)| {
            self.components
                .iter()
                .map(|c| {
                    let (start, span) = pick(&c.position);
                    u32::from(start) + u32::from(span)
                })
                .max()
                .unwrap_or(0)
        };
        GridSize {
            rows: end(|p| (p.row, p.row_span)),
            cols: end(|p| (p.col, p.col_span)),
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
    /// The images the components point to, by `ImageRef`. None until
    /// `with_images` gives them.
    images: Vec<ImageAsset>,
    /// The background of every page that has none of its own.
    background: Option<ImageRef>,
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

        // No cell of a page under two buttons. The sidebar is a column
        // ordered by `row`, so it is not checked.
        for (index, page) in pages.iter().enumerate() {
            let components = page.components();
            for (later, b) in components.iter().enumerate() {
                if let Some(a) = components[..later]
                    .iter()
                    .find(|a| overlap(&a.position, &b.position))
                {
                    return Err(DomainError::CellsOverlap {
                        page: index,
                        a: a.id.as_str().to_string(),
                        b: b.id.as_str().to_string(),
                    });
                }
            }
        }

        // Where each look is allowed: the quit button has none of its own,
        // so no template can disguise it, and a sidebar button neither an
        // image, a style nor more than one cell.
        for component in pages
            .iter()
            .flat_map(|p| p.components())
            .chain(sidebar.components.iter())
        {
            check_looks(component)?;
        }
        for component in &sidebar.components {
            for (field, used) in [
                ("image", component.visual.image.is_some()),
                ("style", !component.style.is_empty()),
                ("row_span", component.position.row_span != 1),
                ("col_span", component.position.col_span != 1),
            ] {
                if used {
                    return Err(DomainError::NotAllowedInSidebar {
                        field,
                        component: component.id.as_str().to_string(),
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
            images: Vec::new(),
            background: None,
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

    /// The same template with the default background of its pages, or none.
    pub fn with_background(mut self, background: Option<ImageRef>) -> Self {
        self.background = background;
        self
    }

    /// The background page `index` shows: its own, or else the template's
    /// default. `None` for a page with neither, or past the last page.
    pub fn page_background(&self, index: usize) -> Option<ImageRef> {
        let page = self.pages.get(index)?;
        page.background.or(self.background)
    }

    /// The same template with the images its components and backgrounds
    /// point to. An `ImageRef` with no image in `images` is rejected.
    pub fn with_images(mut self, images: Vec<ImageAsset>) -> Result<Self, DomainError> {
        let refs = self
            .pages
            .iter()
            .flat_map(|p| p.components())
            .chain(self.sidebar.components.iter())
            .flat_map(|c| [c.visual.image, c.visual.icon])
            .chain(self.pages.iter().map(|p| p.background))
            .chain([self.background])
            .flatten();
        for image in refs {
            if image.0 >= images.len() {
                return Err(DomainError::UnknownImage(image.0));
            }
        }
        self.images = images;
        Ok(self)
    }

    /// The template's images, in the order `ImageRef` counts them.
    pub fn images(&self) -> &[ImageAsset] {
        &self.images
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
        Ok(Self {
            components,
            background: None,
        })
    }
}

/// A button shows an icon or an image, not both; the quit button shows
/// neither, no colours of its own, and always its label.
fn check_looks(component: &Component) -> Result<(), DomainError> {
    let visual = &component.visual;
    if matches!(component.action, Action::App(AppAction::Quit)) {
        for (field, used) in [
            ("icon", visual.icon.is_some()),
            ("image", visual.image.is_some()),
            ("style", !component.style.is_empty()),
            ("hide_label", visual.hide_label),
        ] {
            if used {
                return Err(DomainError::NotAllowedOnQuit { field });
            }
        }
    }
    if visual.icon.is_some() && visual.image.is_some() {
        return Err(DomainError::IconWithImage {
            component: component.id.as_str().to_string(),
        });
    }
    Ok(())
}

/// Whether two buttons cover at least one same cell: their rows and their
/// columns both intersect. Counted in `u32`, so no span wraps around.
fn overlap(a: &ComponentPosition, b: &ComponentPosition) -> bool {
    let meet = |start_a: u16, span_a: u16, start_b: u16, span_b: u16| {
        let (start_a, start_b) = (u32::from(start_a), u32::from(start_b));
        start_a < start_b + u32::from(span_b) && start_b < start_a + u32::from(span_a)
    };
    meet(a.row, a.row_span, b.row, b.row_span) && meet(a.col, a.col_span, b.col, b.col_span)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageRef;

    #[test]
    fn with_images_rejects_an_icon_with_no_image() {
        let button = |id: &str, icon: Option<ImageRef>| Component {
            id: ComponentId::new(id),
            position: ComponentPosition {
                row: 0,
                col: 0,
                row_span: 1,
                col_span: 1,
            },
            visual: VisualProps {
                icon,
                ..VisualProps::default()
            },
            style: ThemeOverrides::default(),
            action: Action::App(AppAction::NextPage),
        };
        let metadata = TemplateMetadata {
            game: "TestGame".into(),
            version: "1".into(),
            author: "dev".into(),
            schema_version: SchemaVersion("1".into()),
        };
        let page = Page::new(vec![button("btn", None)]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![button("lock", Some(ImageRef(0)))],
        };
        let template = Template::new(metadata, vec![page], sidebar).unwrap();

        assert_eq!(
            template.with_images(Vec::new()),
            Err(DomainError::UnknownImage(0))
        );
    }

    #[test]
    fn with_images_rejects_a_background_with_no_image() {
        let metadata = TemplateMetadata {
            game: "TestGame".into(),
            version: "1".into(),
            author: "dev".into(),
            schema_version: SchemaVersion("1".into()),
        };
        let page = || {
            Page::new(vec![Component {
                id: ComponentId::new("btn"),
                position: ComponentPosition {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                },
                visual: VisualProps::default(),
                style: ThemeOverrides::default(),
                action: Action::App(AppAction::NextPage),
            }])
            .unwrap()
        };
        let template = |page: Page| {
            let sidebar = Sidebar {
                default_side: SidebarSide::Right,
                components: Vec::new(),
            };
            Template::new(metadata.clone(), vec![page], sidebar).unwrap()
        };

        let default = template(page()).with_background(Some(ImageRef(0)));
        assert_eq!(
            default.with_images(Vec::new()),
            Err(DomainError::UnknownImage(0))
        );
        let own = template(page().with_background(Some(ImageRef(1))));
        assert_eq!(
            own.with_images(Vec::new()),
            Err(DomainError::UnknownImage(1))
        );
    }
}
