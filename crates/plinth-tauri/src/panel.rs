//! The panel's state and behaviour, free of Tauri: a template, its runtime
//! state and the ports it dispatches through. The Tauri commands only translate
//! to and from this type, so everything here is testable with the fakes of
//! `plinth-application`.

use plinth_application::{
    execute_action, navigate, query_render_state, ActionKind, ActionOutcome, AppError, Clock,
    ComponentView, ConfigStore, GameKeybindProvider, InputSender, Prefs,
};
use plinth_domain::{
    Action, AppAction, ComponentId, DomainError, NavDirection, RuntimeState, SidebarSide, Template,
};
use plinth_infra::template_fs::FsTemplateSource;
use serde::{Deserialize, Serialize};

/// The phase 1 template: one page with the WoW world map button.
pub const WOW_PHASE1_TEMPLATE: &str = include_str!("../templates/wow-phase1.yaml");

/// The demo template the app loads: three pages of default WoW keys.
pub const WOW_DEMO_TEMPLATE: &str = include_str!("../templates/wow-demo.yaml");

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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageViewDto {
    pub components: Vec<ComponentDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDto {
    pub id: String,
    pub label: Option<String>,
    /// `"game"` or `"app"`.
    pub action_kind: &'static str,
    /// The app action a component triggers (`"toggle_lock"`, …), or `None`
    /// for a game action. Lets the UI tell the lock button apart.
    pub app_action: Option<&'static str>,
}

pub struct Panel<I, P, C, S>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
    S: ConfigStore,
{
    template: Template,
    state: RuntimeState,
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
        template_yaml: &str,
        provider: P,
        input: I,
        clock: C,
        config: S,
    ) -> Result<Self, PanelError> {
        let source = FsTemplateSource::new(provider.catalog())
            .map_err(|e| PanelError::Template(e.to_string()))?;
        let template = source
            .load_str(template_yaml)
            .map_err(|e| PanelError::Template(e.to_string()))?;
        // Saved preferences win over the template; none yet means first launch.
        let side = config
            .load()
            .map(|prefs| prefs.sidebar_side)
            .unwrap_or(template.sidebar().default_side);
        let state = RuntimeState::new(side);
        Ok(Self {
            template,
            state,
            provider,
            input,
            clock,
            config,
        })
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
            self.remember_side();
        }
        Ok(outcome)
    }

    /// Saves the sidebar side. A failure only means the side is not remembered
    /// on the next launch, so it is logged and the panel keeps working.
    fn remember_side(&self) {
        let prefs = Prefs {
            sidebar_side: self.state.sidebar_side(),
            last_template: None,
        };
        if let Err(e) = self.config.save(&prefs) {
            eprintln!("plinth: could not save the sidebar side: {e}");
        }
    }

    pub fn navigate(&mut self, direction: NavDirectionDto) -> Result<(), PanelError> {
        let direction = match direction {
            NavDirectionDto::Next => NavDirection::Next,
            NavDirectionDto::Prev => NavDirection::Prev,
        };
        navigate(&mut self.state, &self.template, direction).map_err(map_app_error)
    }

    pub fn render(&self) -> RenderView {
        let state = query_render_state(&self.template, &self.state);
        let dto = |view: ComponentView| self.component_dto(view);
        RenderView {
            pages: state
                .pages
                .into_iter()
                .map(|page| PageViewDto {
                    components: page.components.into_iter().map(dto).collect(),
                })
                .collect(),
            current_page: state.current_page,
            locked: state.locked,
            sidebar: state.sidebar.into_iter().map(dto).collect(),
            sidebar_side: match state.sidebar_side {
                SidebarSide::Left => "left",
                SidebarSide::Right => "right",
            },
        }
    }

    pub fn input(&self) -> &I {
        &self.input
    }

    pub fn config(&self) -> &S {
        &self.config
    }

    fn component_dto(&self, view: ComponentView) -> ComponentDto {
        let app_action = self
            .template
            .find_component(&ComponentId::new(view.id.as_str()))
            .and_then(|component| match &component.action {
                Action::App(app) => Some(app_action_name(app)),
                Action::Game(_) => None,
            });
        ComponentDto {
            id: view.id,
            label: view.label,
            action_kind: match view.action_kind {
                ActionKind::Game => "game",
                ActionKind::App => "app",
            },
            app_action,
        }
    }
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
        Panel::new(
            template,
            StaticKeybindProvider::wow_defaults(),
            FakeInputSender::new(),
            FakeClock::new(),
            config,
        )
        .expect("the template loads")
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
}
