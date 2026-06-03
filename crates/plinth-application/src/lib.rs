//! `plinth-application` — casos de uso y **definición de puertos**.
//!
//! Depende solo de `plinth-domain`. Aquí no entra ningún crate de
//! infraestructura ni lógica de ningún juego concreto: el fitness test de CI
//! lo verifica comprobando que no se importan esos crates.

pub mod dto;
pub mod ports;
pub mod usecases;

#[cfg(feature = "testing")]
pub mod testing;

pub use dto::{ActionKind, ComponentView, PageView, RenderState};
pub use ports::{
    Clock, ConfigError, ConfigStore, GameKeybindProvider, InputError, InputSender, KeyEvent,
    KeyValue, PlacementError, Prefs, ProviderError, ResolveError, TemplateError, TemplateSource,
    WindowPlacement,
};
pub use usecases::{
    execute_action, go_to_page, load_template, navigate, query_render_state, refresh_keybinds,
    switch_sidebar_side, toggle_lock, AppError,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use plinth_domain::{
        Action, ActionCatalog, ActionId, AppAction, Component, ComponentId, ComponentPosition,
        DomainError, GameAction, KeySequence, KeyStep, Page, RuntimeState, ScanCode, SchemaVersion,
        Sidebar, SidebarSide, Template, TemplateMetadata, VisualProps,
    };
    use std::time::Duration;

    const KEY_T: ScanCode = ScanCode(20); // KEY_T
    const KEY_SHIFT: ScanCode = ScanCode(42); // KEY_LEFTSHIFT

    fn meta() -> TemplateMetadata {
        TemplateMetadata {
            game: "TestGame".into(),
            version: "4.0".into(),
            author: "dev".into(),
            schema_version: SchemaVersion("1".into()),
        }
    }

    fn comp(id: &str, action: Action) -> Component {
        Component {
            id: ComponentId::new(id),
            position: ComponentPosition { row: 0, col: 0 },
            visual: VisualProps {
                label: Some(id.to_string()),
                icon: None,
            },
            action,
        }
    }

    fn target_lock() -> GameAction {
        GameAction(ActionId::new("v_target_lock"))
    }

    /// Plantilla de 2 páginas: pág0 con un botón de juego y un toggle de juego;
    /// pág1 con un botón de app (next/lock). Sidebar con lock.
    fn template() -> Template {
        let page0 = Page::new(vec![
            comp("lock_target", Action::Game(target_lock())),
            comp(
                "shield",
                Action::Game(GameAction(ActionId::new("v_toggle_shield"))),
            ),
        ])
        .unwrap();
        let page1 = Page::new(vec![comp("next", Action::App(AppAction::NextPage))]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![comp("lock", Action::App(AppAction::ToggleLock))],
        };
        Template::new(meta(), vec![page0, page1], sidebar).unwrap()
    }

    fn provider() -> FakeKeybindProvider {
        let catalog = ActionCatalog::from_actions([
            ActionId::new("v_target_lock"),
            ActionId::new("v_toggle_shield"),
            ActionId::new("v_unbound"),
        ]);
        let bindings = vec![
            (
                target_lock(),
                KeySequence::new(vec![
                    KeyStep::hold(KEY_SHIFT, Duration::from_millis(80)),
                    KeyStep::press(KEY_T),
                ])
                .unwrap(),
            ),
            (
                GameAction(ActionId::new("v_toggle_shield")),
                KeySequence::new(vec![KeyStep::toggle(KEY_T)]).unwrap(),
            ),
        ];
        FakeKeybindProvider::new(bindings, catalog)
    }

    #[test]
    fn game_button_emits_exact_sequence_with_timing() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        let provider = provider();

        execute_action(
            &ComponentId::new("lock_target"),
            &template,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap();

        // Hold(shift): down, [sleep], up ; luego Press(t): down, up.
        assert_eq!(
            input.recorded(),
            vec![
                KeyEvent::down(KEY_SHIFT),
                KeyEvent::up(KEY_SHIFT),
                KeyEvent::down(KEY_T),
                KeyEvent::up(KEY_T),
            ]
        );
        assert_eq!(clock.recorded_sleeps(), vec![Duration::from_millis(80)]);
    }

    #[test]
    fn toggle_step_flips_runtime_and_emits_press() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        let provider = provider();

        let shield = ComponentId::new("shield");
        execute_action(&shield, &template, &mut state, &input, &provider, &clock).unwrap();

        assert!(state.toggle_state(&shield));
        assert_eq!(
            input.recorded(),
            vec![KeyEvent::down(KEY_T), KeyEvent::up(KEY_T)]
        );
    }

    #[test]
    fn action_not_bound_path() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        // Catálogo conoce v_unbound pero no hay binding para él.
        let provider = provider();

        // Construimos una plantilla con un botón que apunta a v_unbound.
        let page = Page::new(vec![comp(
            "eject",
            Action::Game(GameAction(ActionId::new("v_unbound"))),
        )])
        .unwrap();
        let tmpl = Template::new(
            meta(),
            vec![page],
            Sidebar {
                default_side: SidebarSide::Right,
                components: vec![],
            },
        )
        .unwrap();
        let _ = template; // silencia unused en este caso

        let err = execute_action(
            &ComponentId::new("eject"),
            &tmpl,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            AppError::Domain(DomainError::ActionNotBound(_))
        ));
        assert!(input.recorded().is_empty());
    }

    #[test]
    fn app_button_never_emits_input() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        let provider = provider();

        execute_action(
            &ComponentId::new("next"),
            &template,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap();

        assert_eq!(state.current_page(), 1);
        assert!(
            input.recorded().is_empty(),
            "un botón de app nunca inyecta teclas"
        );
    }

    #[test]
    fn navigation_blocked_when_locked() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        let provider = provider();

        // Activa el candado vía el botón de la sidebar.
        execute_action(
            &ComponentId::new("lock"),
            &template,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap();
        assert!(state.is_locked());

        let err = execute_action(
            &ComponentId::new("next"),
            &template,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Domain(DomainError::LockEngaged)));
    }

    #[test]
    fn go_to_page_out_of_range() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let err = go_to_page(&mut state, &template, 9).unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::PageOutOfRange { .. })
        ));
    }

    #[test]
    fn unknown_component_errors() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Right);
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        let provider = provider();
        let err = execute_action(
            &ComponentId::new("ghost"),
            &template,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::UnknownComponent(_))
        ));
    }

    #[test]
    fn refresh_keybinds_delegates_to_provider() {
        let mut provider = provider();
        refresh_keybinds(&mut provider).unwrap();
        assert_eq!(*provider.refresh_calls.borrow(), 1);
    }

    #[test]
    fn render_state_projection_and_dto_omission() {
        let template = template();
        let mut state = RuntimeState::new(SidebarSide::Left);
        // Marca un toggle para verificar que llega al DTO.
        let input = FakeInputSender::new();
        let clock = FakeClock::new();
        let provider = provider();
        execute_action(
            &ComponentId::new("shield"),
            &template,
            &mut state,
            &input,
            &provider,
            &clock,
        )
        .unwrap();

        let render = query_render_state(&template, &state);
        assert_eq!(render.pages.len(), 2);
        assert_eq!(render.sidebar_side, SidebarSide::Left);
        let shield = render.pages[0]
            .components
            .iter()
            .find(|c| c.id == "shield")
            .unwrap();
        assert!(shield.toggled);
        assert_eq!(shield.action_kind, ActionKind::Game);

        // --- Test de omisión del DTO ---
        // `RenderState` describe qué pintar, nunca qué tecla se emite. Esta
        // comprobación estructural (vía la representación Debug) falla si algún
        // día se filtra un código de tecla al DTO, forzando una decisión
        // consciente.
        let dump = format!("{render:?}");
        assert!(
            !dump.contains("ScanCode") && !dump.contains("KeyEvent") && !dump.contains("KeyStep"),
            "el RenderState no debe exponer códigos de tecla resueltos: {dump}"
        );
    }
}
