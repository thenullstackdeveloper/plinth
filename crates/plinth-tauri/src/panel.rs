//! The panel's state and behaviour, free of Tauri: a template, its runtime
//! state and the ports it dispatches through. The Tauri commands only translate
//! to and from this type, so everything here is testable with the fakes of
//! `plinth-application`.

use std::collections::BTreeMap;

use plinth_application::usecases::image_id;
use plinth_application::{
    execute_action, navigate, query_render_state, ActionKind, ActionOutcome, AppError, Clock,
    ComponentView, ConfigStore, GameKeybindProvider, InputSender, Prefs,
};
use plinth_domain::{
    Action, AppAction, ComponentId, DomainError, ImageRef, NavDirection, PanelSize, RuntimeState,
    SidebarSide, Template,
};
use plinth_infra::template_dir::TemplatePackage;
use plinth_infra::template_fs::FsTemplateSource;
use serde::{Deserialize, Serialize};

use crate::data_uri;

/// The phase 1 template: one page with the WoW world map button.
pub const WOW_PHASE1_TEMPLATE: &str = include_str!("../templates/wow-phase1.yaml");

/// The phase 2 demo: three pages of semantic WoW actions.
pub const WOW_DEMO_TEMPLATE: &str = include_str!("../templates/wow-demo.yaml");

/// The template the app starts with: WoW's default keys as literal keys.
pub const WOW_DEFAULT_KEYS_TEMPLATE: &str = include_str!("../templates/wow-default-keys.yaml");

/// The size of the panel window. Every template must fit it.
pub const PANEL_SIZE: PanelSize = PanelSize {
    width: 900,
    height: 300,
};

/// A page change requested by the UI (a swipe).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NavDirectionDto {
    Next,
    Prev,
}

#[derive(Debug, thiserror::Error)]
pub enum PanelError {
    #[error("template: {0}")]
    Template(String),
    #[error("action: {0}")]
    Action(String),
    /// The lock or a page edge refused the request. Not an error for the user.
    #[error("blocked: {0}")]
    Blocked(String),
}

fn map_app_error(e: AppError) -> PanelError {
    match e {
        AppError::Domain(DomainError::LockEngaged | DomainError::PageOutOfRange { .. }) => {
            PanelError::Blocked(e.to_string())
        }
        other => PanelError::Action(other.to_string()),
    }
}

/// What the UI renders. Serializable here because the core DTOs cannot depend
/// on `serde` (see `scripts/fitness.sh`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderView {
    pub pages: Vec<PageViewDto>,
    pub current_page: usize,
    pub locked: bool,
    pub sidebar: Vec<ComponentDto>,
    /// `"left"` or `"right"`.
    pub sidebar_side: &'static str,
    /// CSS custom property → value, for every design token; an object in JSON.
    pub theme: BTreeMap<String, String>,
    /// How long the change of page takes, in milliseconds.
    pub page_transition_ms: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageViewDto {
    pub components: Vec<ComponentDto>,
    pub rows: u32,
    pub cols: u32,
    /// The page's background, its own or the template's default: a key of
    /// `template_images`. Left out when the page has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDto {
    pub id: String,
    /// The component's cell, zero-based; the UI places it at `row + 1`.
    pub row: u16,
    pub col: u16,
    pub label: Option<String>,
    /// `"game"`, `"app"` or `"keys"`.
    pub action_kind: &'static str,
    /// The app action a component triggers (`"toggle_lock"`, …), or `None`
    /// for a game action. Lets the UI tell the lock button apart.
    pub app_action: Option<&'static str>,
    /// The image that fills the button: a key of `template_images`, never
    /// the image itself. Left out when the button has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    /// The icon next to the label, a key of `template_images` too. Left out
    /// when the button has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_id: Option<String>,
    /// The label is hidden from view but stays the button's name. Left out
    /// when it is shown.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hide_label: bool,
}

pub struct Panel<I, P, C, S>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
    S: ConfigStore,
{
    template: Template,
    /// The template on screen, by file name; `None` for the embedded one.
    template_name: Option<String>,
    /// The template the user last chose, as saved in the prefs. It only
    /// changes on a switch, so a fallback at startup never overwrites it.
    last_template: Option<String>,
    state: RuntimeState,
    /// How many templates the panel has loaded, the first one included. It
    /// is part of every image id, so an id never outlives its template.
    generation: u64,
    /// Image id → `data:` URI of the template on screen, encoded once when it
    /// loads.
    images: BTreeMap<String, String>,
    provider: P,
    input: I,
    clock: C,
    config: S,
}

impl<I, P, C, S> Panel<I, P, C, S>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
    S: ConfigStore,
{
    pub fn new(
        package: &TemplatePackage,
        provider: P,
        input: I,
        clock: C,
        config: S,
    ) -> Result<Self, PanelError> {
        let source = FsTemplateSource::new(provider.catalog())
            .map_err(|e| PanelError::Template(e.to_string()))?;
        let template = source
            .load_package(package)
            .map_err(|e| PanelError::Template(e.to_string()))?;
        template
            .check_fit(PANEL_SIZE)
            .map_err(|e| PanelError::Template(e.to_string()))?;
        // Saved preferences win over the template; none yet means first launch.
        let saved = config.load().ok();
        let side = saved
            .as_ref()
            .map(|prefs| prefs.sidebar_side)
            .unwrap_or(template.sidebar().default_side);
        let last_template = saved.and_then(|prefs| prefs.last_template);
        let state = RuntimeState::new(side);
        let generation = 1;
        let images = encode_images(&template, generation);
        Ok(Self {
            template,
            template_name: None,
            last_template,
            state,
            generation,
            images,
            provider,
            input,
            clock,
            config,
        })
    }

    /// Names the template the panel was opened with (its file name).
    pub fn named(mut self, name: Option<String>) -> Self {
        self.template_name = name;
        self
    }

    /// The file name of the template on screen, if it came from the folder.
    pub fn template_name(&self) -> Option<&str> {
        self.template_name.as_deref()
    }

    /// Replaces the template with `package`, chosen by the user as `name`.
    /// On success the panel starts over on page 0, unlocked, on the same
    /// side, with new image ids, and the choice is saved; a template that
    /// does not load or does not fit the panel changes nothing.
    pub fn switch_template(
        &mut self,
        name: &str,
        package: &TemplatePackage,
    ) -> Result<(), PanelError> {
        let source = FsTemplateSource::new(self.provider.catalog())
            .map_err(|e| PanelError::Template(e.to_string()))?;
        let template = source
            .load_package(package)
            .map_err(|e| PanelError::Template(format!("`{name}`: {e}")))?;
        template
            .check_fit(PANEL_SIZE)
            .map_err(|e| PanelError::Template(format!("`{name}`: {e}")))?;
        self.generation += 1;
        self.images = encode_images(&template, self.generation);
        self.template = template;
        self.state = RuntimeState::new(self.state.sidebar_side());
        self.template_name = Some(name.to_string());
        self.last_template = Some(name.to_string());
        self.save_prefs();
        Ok(())
    }

    pub fn press(&mut self, component_id: &str) -> Result<ActionOutcome, PanelError> {
        let side_before = self.state.sidebar_side();
        let outcome = execute_action(
            &ComponentId::new(component_id),
            &self.template,
            &mut self.state,
            &self.input,
            &self.provider,
            &self.clock,
        )
        .map_err(map_app_error)?;
        if self.state.sidebar_side() != side_before {
            self.save_prefs();
        }
        Ok(outcome)
    }

    /// Saves the sidebar side and the template the user chose. A failure only
    /// means they are not remembered on the next launch, so it is logged and
    /// the panel keeps working.
    fn save_prefs(&self) {
        let prefs = Prefs {
            sidebar_side: self.state.sidebar_side(),
            last_template: self.last_template.clone(),
        };
        if let Err(e) = self.config.save(&prefs) {
            eprintln!("plinth: could not save the preferences: {e}");
        }
    }

    pub fn navigate(&mut self, direction: NavDirectionDto) -> Result<(), PanelError> {
        let direction = match direction {
            NavDirectionDto::Next => NavDirection::Next,
            NavDirectionDto::Prev => NavDirection::Prev,
        };
        navigate(&mut self.state, &self.template, direction).map_err(map_app_error)
    }

    /// Image id → `data:` URI of every image of the template on screen. The
    /// UI asks for it once per template; `render` only carries the ids.
    pub fn template_images(&self) -> BTreeMap<String, String> {
        self.images.clone()
    }

    pub fn render(&self) -> RenderView {
        let state = query_render_state(&self.template, &self.state, self.generation);
        let dto = |view: ComponentView| self.component_dto(view);
        RenderView {
            pages: state
                .pages
                .into_iter()
                .map(|page| PageViewDto {
                    components: page.components.into_iter().map(dto).collect(),
                    rows: page.rows,
                    cols: page.cols,
                    background_id: page.background_id,
                })
                .collect(),
            current_page: state.current_page,
            locked: state.locked,
            sidebar: state.sidebar.into_iter().map(dto).collect(),
            sidebar_side: match state.sidebar_side {
                SidebarSide::Left => "left",
                SidebarSide::Right => "right",
            },
            theme: state
                .theme
                .into_iter()
                .map(|var| (var.name, var.value))
                .collect(),
            page_transition_ms: state.page_transition_ms,
        }
    }

    pub fn input(&self) -> &I {
        &self.input
    }

    pub fn config(&self) -> &S {
        &self.config
    }

    pub fn clock(&self) -> &C {
        &self.clock
    }

    fn component_dto(&self, view: ComponentView) -> ComponentDto {
        let app_action = self
            .template
            .find_component(&ComponentId::new(view.id.as_str()))
            .and_then(|component| match &component.action {
                Action::App(app) => Some(app_action_name(app)),
                Action::Game(_) | Action::Keys(_) => None,
            });
        ComponentDto {
            id: view.id,
            row: view.row,
            col: view.col,
            label: view.label,
            action_kind: match view.action_kind {
                ActionKind::Game => "game",
                ActionKind::App => "app",
                ActionKind::Keys => "keys",
            },
            app_action,
            image_id: view.image_id,
            icon_id: view.icon_id,
            hide_label: view.hide_label,
        }
    }
}

/// The images of `template` as `data:` URIs, by the ids `render` gives them.
fn encode_images(template: &Template, generation: u64) -> BTreeMap<String, String> {
    template
        .images()
        .iter()
        .enumerate()
        .map(|(index, asset)| (image_id(generation, ImageRef(index)), data_uri::of(asset)))
        .collect()
}

fn app_action_name(action: &AppAction) -> &'static str {
    match action {
        AppAction::NextPage => "next_page",
        AppAction::PrevPage => "prev_page",
        AppAction::GoToPage(_) => "go_to_page",
        AppAction::ToggleLock => "toggle_lock",
        AppAction::SwitchSidebarSide => "switch_sidebar",
        AppAction::Quit => "quit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_application::testing::{FakeClock, FakeInputSender, InMemoryConfigStore};
    use plinth_application::{ConfigError, KeyEvent};
    use plinth_domain::ScanCode;
    use plinth_infra::keybinds_static::StaticKeybindProvider;

    type TestPanel<S> = Panel<FakeInputSender, StaticKeybindProvider, FakeClock, S>;

    fn panel() -> TestPanel<InMemoryConfigStore> {
        panel_with(WOW_PHASE1_TEMPLATE)
    }

    fn panel_with(template: &str) -> TestPanel<InMemoryConfigStore> {
        panel_with_config(template, InMemoryConfigStore::default())
    }

    fn panel_with_config<S: ConfigStore>(template: &str, config: S) -> TestPanel<S> {
        panel_of(&TemplatePackage::single_file(template), config)
    }

    fn panel_of<S: ConfigStore>(package: &TemplatePackage, config: S) -> TestPanel<S> {
        Panel::new(
            package,
            StaticKeybindProvider::wow_defaults(),
            FakeInputSender::new(),
            FakeClock::new(),
            config,
        )
        .expect("the template loads")
    }

    fn single(yaml: &str) -> TemplatePackage {
        TemplatePackage::single_file(yaml)
    }

    /// A store that can neither read nor write: a first launch with a
    /// read-only config directory.
    struct FailingConfigStore;

    impl ConfigStore for FailingConfigStore {
        fn load(&self) -> Result<Prefs, ConfigError> {
            Err(ConfigError("no prefs".into()))
        }

        fn save(&self, _prefs: &Prefs) -> Result<(), ConfigError> {
            Err(ConfigError("read-only".into()))
        }
    }

    #[test]
    fn panel_press_world_map_emits_m() {
        let mut panel = panel();
        panel.press("world_map").unwrap();
        assert_eq!(
            panel.input().recorded(),
            vec![KeyEvent::down(ScanCode(50)), KeyEvent::up(ScanCode(50))]
        );
    }

    #[test]
    fn press_unknown_component_is_an_error() {
        let mut panel = panel();
        assert!(matches!(panel.press("nope"), Err(PanelError::Action(_))));
        assert!(panel.input().recorded().is_empty());
    }

    #[test]
    fn render_lists_the_world_map_button() {
        let view = panel().render();
        assert_eq!(view.current_page, 0);
        assert_eq!(view.pages.len(), 1);
        let components = &view.pages[0].components;
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].id, "world_map");
        assert_eq!(components[0].label.as_deref(), Some("World map"));
        assert_eq!(components[0].action_kind, "game");
    }

    #[test]
    fn render_view_serializes_in_camel_case() {
        let json = serde_json::to_value(panel().render()).unwrap();
        assert!(json.get("currentPage").is_some());
        assert!(json["pages"][0]["components"][0]
            .get("actionKind")
            .is_some());
    }

    #[test]
    fn demo_template_has_three_pages() {
        let view = panel_with(WOW_DEMO_TEMPLATE).render();
        assert_eq!(view.pages.len(), 3);
        let ids: Vec<&str> = view.pages[0]
            .components
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(ids, vec!["world_map", "character"]);
    }

    #[test]
    fn navigate_next_moves_to_page_1() {
        let mut panel = panel_with(WOW_DEMO_TEMPLATE);
        panel.navigate(NavDirectionDto::Next).unwrap();
        assert_eq!(panel.render().current_page, 1);
    }

    #[test]
    fn navigate_past_the_last_page_is_blocked() {
        let mut panel = panel_with(WOW_DEMO_TEMPLATE);
        panel.navigate(NavDirectionDto::Next).unwrap();
        panel.navigate(NavDirectionDto::Next).unwrap();
        assert!(matches!(
            panel.navigate(NavDirectionDto::Next),
            Err(PanelError::Blocked(_))
        ));
        assert_eq!(panel.render().current_page, 2);
    }

    #[test]
    fn render_includes_the_sidebar() {
        let view = panel_with(WOW_DEMO_TEMPLATE).render();
        let ids: Vec<&str> = view.sidebar.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"lock"));
        assert!(ids.contains(&"switch_side"));
        assert_eq!(view.sidebar_side, "right");
        let lock = view.sidebar.iter().find(|c| c.id == "lock").unwrap();
        assert_eq!(lock.app_action, Some("toggle_lock"));
    }

    #[test]
    fn switch_side_moves_the_sidebar_and_saves_it() {
        let mut panel = panel_with(WOW_DEMO_TEMPLATE);
        panel.press("switch_side").unwrap();
        assert_eq!(panel.render().sidebar_side, "left");
        assert_eq!(
            panel.config().load().unwrap().sidebar_side,
            SidebarSide::Left
        );
    }

    #[test]
    fn saved_side_wins_over_the_template() {
        let config = InMemoryConfigStore::new(Prefs {
            sidebar_side: SidebarSide::Left,
            last_template: None,
        });
        let panel = panel_with_config(WOW_DEMO_TEMPLATE, config);
        assert_eq!(panel.render().sidebar_side, "left");
    }

    #[test]
    fn missing_prefs_fall_back_to_the_template() {
        let panel = panel_with_config(WOW_DEMO_TEMPLATE, FailingConfigStore);
        assert_eq!(panel.render().sidebar_side, "right");
    }

    #[test]
    fn failed_save_keeps_the_panel_working() {
        let mut panel = panel_with_config(WOW_DEMO_TEMPLATE, FailingConfigStore);
        assert!(panel.press("switch_side").is_ok());
        assert_eq!(panel.render().sidebar_side, "left");
    }

    #[test]
    fn navigate_is_blocked_when_locked() {
        let mut panel = panel_with(WOW_DEMO_TEMPLATE);
        panel.press("lock").unwrap();
        assert!(matches!(
            panel.navigate(NavDirectionDto::Next),
            Err(PanelError::Blocked(_))
        ));
        assert_eq!(panel.render().current_page, 0);
    }

    #[test]
    fn quit_press_reports_quit() {
        let mut panel = panel_with(WOW_DEMO_TEMPLATE);
        assert_eq!(panel.press("quit").unwrap(), ActionOutcome::Quit);
    }

    #[test]
    fn quit_press_is_blocked_when_locked() {
        let mut panel = panel_with(WOW_DEMO_TEMPLATE);
        panel.press("lock").unwrap();
        assert!(matches!(panel.press("quit"), Err(PanelError::Blocked(_))));
    }
    // --- The default-keys template (literal keys) ---

    #[test]
    fn default_keys_template_has_three_pages() {
        let view = panel_with(WOW_DEFAULT_KEYS_TEMPLATE).render();
        let sizes: Vec<usize> = view.pages.iter().map(|p| p.components.len()).collect();
        assert_eq!(sizes, vec![6, 6, 3]);
        assert_eq!(view.pages[2].components[2].id, "char_and_bags");
    }

    #[test]
    fn pressing_char_and_bags_emits_c_then_b() {
        let mut panel = panel_with(WOW_DEFAULT_KEYS_TEMPLATE);
        panel.press("char_and_bags").unwrap();
        assert_eq!(
            panel.input().recorded(),
            vec![
                KeyEvent::down(ScanCode(46)),
                KeyEvent::up(ScanCode(46)),
                KeyEvent::down(ScanCode(48)),
                KeyEvent::up(ScanCode(48)),
            ]
        );
        assert_eq!(
            panel.clock().recorded_sleeps(),
            vec![std::time::Duration::from_millis(30)]
        );
    }

    #[test]
    fn pressing_bar_page_2_sends_shift_2() {
        let mut panel = panel_with(WOW_DEFAULT_KEYS_TEMPLATE);
        panel.press("bar_page_2").unwrap();
        assert_eq!(
            panel.input().recorded(),
            vec![
                KeyEvent::down(ScanCode(42)),
                KeyEvent::down(ScanCode(3)),
                KeyEvent::up(ScanCode(3)),
                KeyEvent::up(ScanCode(42)),
            ]
        );
    }

    #[test]
    fn render_carries_grid_positions() {
        let view = panel_with(WOW_DEFAULT_KEYS_TEMPLATE).render();
        let sizes: Vec<(u32, u32)> = view.pages.iter().map(|p| (p.rows, p.cols)).collect();
        assert_eq!(sizes, vec![(2, 3), (2, 3), (1, 3)]);
        let cells: Vec<(&str, u16, u16)> = view.pages[0]
            .components
            .iter()
            .map(|c| (c.id.as_str(), c.row, c.col))
            .collect();
        assert_eq!(
            cells,
            vec![
                ("action_1", 0, 0),
                ("action_2", 0, 1),
                ("action_3", 0, 2),
                ("action_4", 1, 0),
                ("action_5", 1, 1),
                ("action_6", 1, 2),
            ]
        );
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["pages"][0]["rows"], 2);
        assert_eq!(json["pages"][0]["components"][4]["col"], 1);
    }

    #[test]
    fn keys_component_renders_as_keys() {
        let view = panel_with(WOW_DEFAULT_KEYS_TEMPLATE).render();
        let first = &view.pages[0].components[0];
        assert_eq!(first.id, "action_1");
        assert_eq!(first.action_kind, "keys");
        assert_eq!(first.app_action, None);
    }
    // --- Switching templates and remembering the chosen one ---

    fn prefs(side: SidebarSide, last: Option<&str>) -> Prefs {
        Prefs {
            sidebar_side: side,
            last_template: last.map(str::to_string),
        }
    }

    #[test]
    fn switch_template_resets_page_and_lock_and_keeps_the_side() {
        let mut panel = panel_with(WOW_DEFAULT_KEYS_TEMPLATE);
        panel.press("switch_side").unwrap();
        panel.navigate(NavDirectionDto::Next).unwrap();
        panel.press("lock").unwrap();

        panel
            .switch_template("wow-demo.yaml", &single(WOW_DEMO_TEMPLATE))
            .unwrap();

        let view = panel.render();
        assert_eq!(view.current_page, 0);
        assert!(!view.locked);
        assert_eq!(view.sidebar_side, "left");
        assert_eq!(view.pages[0].components[0].action_kind, "game");
        assert_eq!(panel.template_name(), Some("wow-demo.yaml"));
    }

    #[test]
    fn switch_template_saves_last_template() {
        let mut panel = panel_with(WOW_DEFAULT_KEYS_TEMPLATE);
        panel
            .switch_template("wow-demo.yaml", &single(WOW_DEMO_TEMPLATE))
            .unwrap();
        assert_eq!(
            panel.config().load().unwrap(),
            prefs(SidebarSide::Right, Some("wow-demo.yaml"))
        );
    }

    #[test]
    fn switch_to_an_invalid_template_keeps_the_current_one() {
        let mut panel =
            panel_with(WOW_DEFAULT_KEYS_TEMPLATE).named(Some("wow-default-keys.yaml".into()));
        panel.navigate(NavDirectionDto::Next).unwrap();

        let result = panel.switch_template("broken.yaml", &single("pages: nope"));

        assert!(matches!(result, Err(PanelError::Template(_))));
        assert_eq!(panel.template_name(), Some("wow-default-keys.yaml"));
        assert_eq!(panel.render().current_page, 1);
        assert_eq!(panel.config().load().unwrap().last_template, None);
    }

    #[test]
    fn switching_side_keeps_the_chosen_template() {
        let config = InMemoryConfigStore::new(prefs(SidebarSide::Right, Some("mine.yaml")));
        let mut panel = panel_with_config(WOW_DEFAULT_KEYS_TEMPLATE, config)
            .named(Some("wow-default-keys.yaml".into()));
        panel.press("switch_side").unwrap();
        assert_eq!(
            panel.config().load().unwrap(),
            prefs(SidebarSide::Left, Some("mine.yaml"))
        );
    }

    #[test]
    fn a_failed_save_does_not_undo_the_switch() {
        let mut panel = panel_with_config(WOW_DEFAULT_KEYS_TEMPLATE, FailingConfigStore);
        assert!(panel
            .switch_template("wow-demo.yaml", &single(WOW_DEMO_TEMPLATE))
            .is_ok());
        assert_eq!(panel.template_name(), Some("wow-demo.yaml"));
    }
    // --- The theme, against the values the UI tests use ---

    /// The resolved default theme the UI tests, stories and fake backend use.
    const DEFAULT_THEME_FIXTURE: &str =
        include_str!("../../../ui/src/testing/fixtures/default-theme.json");

    /// The theme of a template that overrides `color.button.background`.
    const SLICE_THEME_FIXTURE: &str =
        include_str!("../../../ui/src/testing/fixtures/slice-theme.json");

    fn theme_json(panel: &TestPanel<InMemoryConfigStore>) -> serde_json::Value {
        serde_json::to_value(panel.render()).unwrap()["theme"].clone()
    }

    fn fixture(src: &str) -> serde_json::Value {
        serde_json::from_str(src).unwrap()
    }

    #[test]
    fn render_view_carries_the_default_theme_as_the_ui_fixture() {
        assert_eq!(
            theme_json(&panel_with(WOW_DEFAULT_KEYS_TEMPLATE)),
            fixture(DEFAULT_THEME_FIXTURE)
        );
    }

    #[test]
    fn theme_fixtures_resolve_all_25_tokens() {
        let default = fixture(DEFAULT_THEME_FIXTURE);
        let slice = fixture(SLICE_THEME_FIXTURE);
        let (default, slice) = (default.as_object().unwrap(), slice.as_object().unwrap());
        assert_eq!(default.len(), 25);
        // The slice theme is the default with one override, and nothing else.
        let mut expected = default.clone();
        expected.insert("--plinth-color-button-background".into(), "#1e1e2e".into());
        assert_eq!(slice, &expected);
    }

    #[test]
    fn themed_template_renders_the_ui_slice_fixture() {
        let themed =
            format!("{WOW_PHASE1_TEMPLATE}theme:\n  color.button.background: \"#1e1e2e\"\n");
        assert_eq!(
            theme_json(&panel_with(&themed)),
            fixture(SLICE_THEME_FIXTURE)
        );
    }

    #[test]
    fn render_view_carries_the_page_transition_in_ms() {
        let json = |panel: &TestPanel<InMemoryConfigStore>| {
            serde_json::to_value(panel.render()).unwrap()["pageTransitionMs"].clone()
        };
        assert_eq!(json(&panel_with(WOW_PHASE1_TEMPLATE)), 200);
        let themed = format!("{WOW_PHASE1_TEMPLATE}theme:\n  motion.page-transition: \"150ms\"\n");
        assert_eq!(json(&panel_with(&themed)), 150);
    }

    // --- Fitting the panel window ---

    /// The values the UI tests use for the largest theme that still fits.
    const MAX_THEME_FIXTURE: &str = include_str!("../../../ui/src/testing/fixtures/max-theme.json");

    /// Every size and space token at its maximum, as a template `theme` block.
    const MAX_THEME_BLOCK: &str = "theme:
  size.button.min: \"96px\"
  space.page.gap: \"16px\"
  space.page.padding: \"24px\"
  size.sidebar.width: \"240px\"
  space.sidebar.gap: \"16px\"
  space.sidebar.padding: \"16px\"
  size.button.border-width: \"4px\"
  radius.button: \"24px\"
  font.size.label: \"28px\"
  font.weight.label: 900
  font.line-height.label: 1.6
  motion.page-transition: \"400ms\"
  motion.press-feedback: \"200ms\"
";

    /// The default-keys template with six sidebar buttons: 6·44 + 5·8 + 2·8 =
    /// 320px, taller than the panel.
    fn too_tall_template() -> String {
        let extra: String = (3..6)
            .map(|row| {
                format!(
                    "    - {{ id: extra_{row}, position: {{ row: {row}, col: 0 }}, action: \"app.next_page\", label: \"Extra\" }}\n"
                )
            })
            .collect();
        format!("{WOW_DEFAULT_KEYS_TEMPLATE}{extra}")
    }

    #[test]
    fn panel_new_rejects_a_template_that_does_not_fit() {
        let result = Panel::new(
            &single(&too_tall_template()),
            StaticKeybindProvider::wow_defaults(),
            FakeInputSender::new(),
            FakeClock::new(),
            InMemoryConfigStore::default(),
        );
        match result {
            Err(PanelError::Template(message)) => assert_eq!(
                message,
                "the theme does not fit the panel: sidebar height needs 320px but only 300px are available"
            ),
            Err(other) => panic!("expected a template error, got {other}"),
            Ok(_) => panic!("a template that does not fit must not open"),
        }
    }

    #[test]
    fn switch_to_a_template_that_does_not_fit_keeps_the_current_one() {
        let mut panel =
            panel_with(WOW_DEFAULT_KEYS_TEMPLATE).named(Some("wow-default-keys.yaml".into()));
        panel.navigate(NavDirectionDto::Next).unwrap();

        let result = panel.switch_template("tall.yaml", &single(&too_tall_template()));

        match result {
            Err(PanelError::Template(message)) => assert_eq!(
                message,
                "`tall.yaml`: the theme does not fit the panel: sidebar height needs 320px but only 300px are available"
            ),
            other => panic!("expected a template error, got {other:?}"),
        }
        assert_eq!(panel.template_name(), Some("wow-default-keys.yaml"));
        assert_eq!(panel.render().current_page, 1);
        assert_eq!(panel.render().sidebar.len(), 3);
        assert_eq!(panel.config().load().unwrap().last_template, None);
    }

    #[test]
    fn max_theme_renders_the_ui_fixture() {
        let themed = format!("{WOW_DEFAULT_KEYS_TEMPLATE}{MAX_THEME_BLOCK}");
        assert_eq!(theme_json(&panel_with(&themed)), fixture(MAX_THEME_FIXTURE));
    }

    #[test]
    fn max_theme_puts_every_overridden_token_at_its_maximum() {
        let catalog = plinth_infra::design_tokens::embedded_catalog().unwrap();
        let (default, max) = (fixture(DEFAULT_THEME_FIXTURE), fixture(MAX_THEME_FIXTURE));
        let mut changed = 0;
        for spec in catalog.specs() {
            let name = plinth_application::theme_css::css_variable(&spec.path);
            if default[&name] == max[&name] {
                continue;
            }
            changed += 1;
            let plinth_domain::TokenRule::Range { max: bound, .. } = spec.rule else {
                panic!("`{}` changed but has no range", spec.path);
            };
            let mut at_max = spec.default.clone();
            match &mut at_max {
                plinth_domain::TokenValue::Px(n)
                | plinth_domain::TokenValue::Ms(n)
                | plinth_domain::TokenValue::FontWeight(n) => *n = bound,
                plinth_domain::TokenValue::Number(h) => h.0 = bound,
                _ => panic!("`{}` is not a ranged kind", spec.path),
            }
            assert_eq!(max[&name], at_max.to_string(), "`{}`", spec.path);
        }
        assert_eq!(changed, 13);
    }

    // --- Images from a template folder ---

    use plinth_infra::template_dir::TemplateAssets;
    use std::path::{Path, PathBuf};

    /// The part of a PNG the loader reads: the signature and an `IHDR` of
    /// `width` × `height`, then `IEND`. The CRCs are not checked, so they
    /// are left at zero; `tag` makes two images differ in their bytes.
    fn png_header(width: u32, height: u32, tag: u8) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, tag]);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(b"IEND");
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    /// A template folder of its own under the temp dir, with `files`.
    fn image_folder(test: &str, files: &[(&str, Vec<u8>)]) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("plinth-panel-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for (path, bytes) in files {
            let file = folder.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        folder
    }

    /// One page with a button per `(id, image)`, side by side.
    fn image_template(buttons: &[(&str, &str)]) -> String {
        let components: String = buttons
            .iter()
            .enumerate()
            .map(|(col, (id, image))| {
                format!(
                    "      - {{ id: {id}, position: {{ row: 0, col: {col} }}, keys: \"m\", label: \"{id}\", image: \"{image}\" }}\n"
                )
            })
            .collect();
        format!(
            "schema_version: \"1\"\nmetadata: {{ game: WoW, version: \"1\", author: dev }}\npages:\n  - components:\n{components}      - {{ id: plain, position: {{ row: 1, col: 0 }}, keys: \"m\" }}\nsidebar: {{ default_side: right, components: [] }}\n"
        )
    }

    fn folder_package(folder: &Path, yaml: String) -> TemplatePackage {
        TemplatePackage {
            yaml,
            assets: TemplateAssets::Folder(folder.to_path_buf()),
        }
    }

    fn image_ids(view: &RenderView) -> Vec<Option<&str>> {
        view.pages[0]
            .components
            .iter()
            .map(|c| c.image_id.as_deref())
            .collect()
    }

    #[test]
    fn the_same_file_twice_is_one_image() {
        let folder = image_folder("same-file", &[("img/a.png", png_header(2, 2, 1))]);
        let yaml = image_template(&[("one", "img/a.png"), ("two", "img/a.png")]);
        let panel = panel_of(
            &folder_package(&folder, yaml),
            InMemoryConfigStore::default(),
        );

        assert_eq!(
            image_ids(&panel.render()),
            vec![Some("t1i0"), Some("t1i0"), None]
        );
        let images = panel.template_images();
        assert_eq!(images.keys().collect::<Vec<_>>(), vec!["t1i0"]);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn render_view_carries_ids_not_data() {
        let folder = image_folder("ids-not-data", &[("a.png", png_header(2, 2, 1))]);
        let yaml = image_template(&[("one", "a.png")]);
        let panel = panel_of(
            &folder_package(&folder, yaml),
            InMemoryConfigStore::default(),
        );

        let json = serde_json::to_value(panel.render()).unwrap();
        assert_eq!(json["pages"][0]["components"][0]["imageId"], "t1i0");
        // A button without an image has no `imageId` at all.
        assert!(json["pages"][0]["components"][1].get("imageId").is_none());
        assert!(!json.to_string().contains("data:"), "{json}");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn template_images_maps_each_id_to_its_data_uri() {
        let (a, b) = (png_header(2, 2, 1), png_header(3, 1, 2));
        let folder = image_folder(
            "data-uris",
            &[("a.png", a.clone()), ("img/b.png", b.clone())],
        );
        let package = folder_package(
            &folder,
            image_template(&[("one", "a.png"), ("two", "img/b.png")]),
        );
        let mut panel = panel_of(&package, InMemoryConfigStore::default());

        let uri = |bytes: &[u8]| format!("data:image/png;base64,{}", data_uri::base64(bytes));
        let expected: BTreeMap<String, String> =
            [("t1i0".to_string(), uri(&a)), ("t1i1".to_string(), uri(&b))].into();
        assert_eq!(panel.template_images(), expected);
        assert_eq!(
            image_ids(&panel.render()),
            vec![Some("t1i0"), Some("t1i1"), None]
        );

        // Another template is another generation: no id of the first one is
        // left, even for the same files.
        panel.switch_template("t/", &package).unwrap();
        let ids: Vec<String> = panel.template_images().into_keys().collect();
        assert_eq!(ids, vec!["t2i0", "t2i1"]);
        assert_eq!(
            image_ids(&panel.render()),
            vec![Some("t2i0"), Some("t2i1"), None]
        );
        // A template without images has none.
        panel
            .switch_template("wow-demo.yaml", &single(WOW_DEMO_TEMPLATE))
            .unwrap();
        assert!(panel.template_images().is_empty());
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_view_carries_icon_ids_and_hidden_labels() {
        let folder = image_folder(
            "icons",
            &[
                ("icon.png", png_header(2, 2, 1)),
                ("face.png", png_header(3, 1, 2)),
            ],
        );
        let yaml = "schema_version: \"1\"\nmetadata: { game: WoW, version: \"1\", author: dev }\npages:\n  - components:\n      - { id: fire, position: { row: 0, col: 0 }, keys: \"m\", label: \"Fire\", icon: \"icon.png\", hide_label: true }\n      - { id: face, position: { row: 0, col: 1 }, keys: \"m\", image: \"face.png\" }\nsidebar:\n  default_side: right\n  components:\n    - { id: lock, position: { row: 0, col: 0 }, action: \"app.toggle_lock\", icon: \"icon.png\" }\n";
        let panel = panel_of(
            &folder_package(&folder, yaml.to_string()),
            InMemoryConfigStore::default(),
        );

        let json = serde_json::to_value(panel.render()).unwrap();
        let fire = &json["pages"][0]["components"][0];
        assert_eq!(fire["iconId"], "t1i0");
        assert_eq!(fire["hideLabel"], true);
        assert!(fire.get("imageId").is_none(), "{fire}");
        // A shown label and no icon are left out.
        let face = &json["pages"][0]["components"][1];
        assert_eq!(face["imageId"], "t1i1");
        assert!(face.get("iconId").is_none(), "{face}");
        assert!(face.get("hideLabel").is_none(), "{face}");
        // The sidebar takes the icon too, by the same id.
        let lock = &json["sidebar"][0];
        assert_eq!(lock["iconId"], "t1i0");
        assert!(lock.get("hideLabel").is_none(), "{lock}");
        assert_eq!(
            panel.template_images().into_keys().collect::<Vec<_>>(),
            vec!["t1i0", "t1i1"]
        );
        assert!(!json.to_string().contains("data:"), "{json}");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_view_carries_each_page_background_id() {
        let folder = image_folder(
            "backgrounds",
            &[
                ("root.png", png_header(4, 4, 1)),
                ("own.png", png_header(5, 5, 2)),
            ],
        );
        let yaml = "schema_version: \"1\"\nmetadata: { game: WoW, version: \"1\", author: dev }\nbackground: { image: \"root.png\" }\npages:\n  - components:\n      - { id: a, position: { row: 0, col: 0 }, keys: \"m\", image: \"own.png\" }\n  - background: { image: \"own.png\" }\n    components:\n      - { id: b, position: { row: 0, col: 0 }, keys: \"m\" }\nsidebar: { default_side: right, components: [] }\n";
        let panel = panel_of(
            &folder_package(&folder, yaml.to_string()),
            InMemoryConfigStore::default(),
        );

        let json = serde_json::to_value(panel.render()).unwrap();
        // The first page takes the default; the second, its own, by the
        // same id as the button image of the same file.
        assert_eq!(json["pages"][0]["backgroundId"], "t1i0");
        assert_eq!(json["pages"][1]["backgroundId"], "t1i1");
        assert_eq!(json["pages"][0]["components"][0]["imageId"], "t1i1");
        assert_eq!(
            panel.template_images().into_keys().collect::<Vec<_>>(),
            vec!["t1i0", "t1i1"]
        );
        assert!(!json.to_string().contains("data:"), "{json}");
        // A page with no background has no `backgroundId` at all.
        let plain = serde_json::to_value(panel_with(WOW_DEMO_TEMPLATE).render()).unwrap();
        assert!(plain["pages"][0].get("backgroundId").is_none(), "{plain}");
        let _ = std::fs::remove_dir_all(folder);
    }
}
