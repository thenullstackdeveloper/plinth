//! The panel's state and behaviour, free of Tauri: a template, its runtime
//! state and the ports it dispatches through. The Tauri commands only translate
//! to and from this type, so everything here is testable with the fakes of
//! `plinth-application`.

use plinth_application::{
    execute_action, query_render_state, ActionKind, Clock, ComponentView, GameKeybindProvider,
    InputSender,
};
use plinth_domain::{ComponentId, RuntimeState, Template};
use plinth_infra::template_fs::FsTemplateSource;
use serde::Serialize;

/// The phase 1 template: one page with the WoW world map button.
pub const WOW_PHASE1_TEMPLATE: &str = include_str!("../templates/wow-phase1.yaml");

#[derive(Debug, thiserror::Error)]
pub enum PanelError {
    #[error("template: {0}")]
    Template(String),
    #[error("action: {0}")]
    Action(String),
}

/// What the UI renders. Serializable here because the core DTOs cannot depend
/// on `serde` (see `scripts/fitness.sh`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderView {
    pub pages: Vec<PageViewDto>,
    pub current_page: usize,
    pub locked: bool,
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
}

pub struct Panel<I, P, C>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
{
    template: Template,
    state: RuntimeState,
    provider: P,
    input: I,
    clock: C,
}

impl<I, P, C> Panel<I, P, C>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
{
    pub fn new(template_yaml: &str, provider: P, input: I, clock: C) -> Result<Self, PanelError> {
        let source = FsTemplateSource::new(provider.catalog())
            .map_err(|e| PanelError::Template(e.to_string()))?;
        let template = source
            .load_str(template_yaml)
            .map_err(|e| PanelError::Template(e.to_string()))?;
        let state = RuntimeState::new(template.sidebar().default_side);
        Ok(Self {
            template,
            state,
            provider,
            input,
            clock,
        })
    }

    pub fn press(&mut self, component_id: &str) -> Result<(), PanelError> {
        execute_action(
            &ComponentId::new(component_id),
            &self.template,
            &mut self.state,
            &self.input,
            &self.provider,
            &self.clock,
        )
        .map_err(|e| PanelError::Action(e.to_string()))
    }

    pub fn render(&self) -> RenderView {
        let state = query_render_state(&self.template, &self.state);
        RenderView {
            pages: state
                .pages
                .into_iter()
                .map(|page| PageViewDto {
                    components: page.components.into_iter().map(component_dto).collect(),
                })
                .collect(),
            current_page: state.current_page,
            locked: state.locked,
        }
    }

    pub fn input(&self) -> &I {
        &self.input
    }
}

fn component_dto(view: ComponentView) -> ComponentDto {
    ComponentDto {
        id: view.id,
        label: view.label,
        action_kind: match view.action_kind {
            ActionKind::Game => "game",
            ActionKind::App => "app",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_application::testing::{FakeClock, FakeInputSender};
    use plinth_application::KeyEvent;
    use plinth_domain::ScanCode;
    use plinth_infra::keybinds_static::StaticKeybindProvider;

    fn panel() -> Panel<FakeInputSender, StaticKeybindProvider, FakeClock> {
        Panel::new(
            WOW_PHASE1_TEMPLATE,
            StaticKeybindProvider::wow_defaults(),
            FakeInputSender::new(),
            FakeClock::new(),
        )
        .expect("the phase 1 template loads")
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
}
