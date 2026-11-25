//! `plinth-domain` — the product's language, pure and game-agnostic.
//!
//! This crate depends on no infrastructure crate and no logic specific to any
//! particular game. It is the architecture's golden rule: if any of that appears
//! here, it **won't compile**, and the CI fitness test catches it.

pub mod action;
pub mod catalog;
pub mod error;
pub mod key;
pub mod runtime;
pub mod template;
pub mod theme;

pub use action::{Action, ActionId, AppAction, GameAction};
pub use catalog::ActionCatalog;
pub use error::DomainError;
pub use key::{
    ComponentPosition, KeyBinding, KeyChord, KeyMacro, KeySequence, KeyStep, KeyStepKind, ScanCode,
    SchemaVersion, SidebarSide, VisualProps,
};
pub use runtime::{NavDirection, RuntimeState};
pub use template::{Component, ComponentId, Page, Sidebar, Template, TemplateMetadata};
pub use theme::{
    Hundredths, Srgb8, Theme, ThemeCatalog, TokenKind, TokenPath, TokenRule, TokenSpec, TokenValue,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn catalog() -> ActionCatalog {
        ActionCatalog::from_actions([ActionId::new("v_target_lock"), ActionId::new("v_eject")])
    }

    #[test]
    fn resolve_known_game_action() {
        let action = catalog().resolve("v_target_lock").unwrap();
        assert_eq!(
            action,
            Action::Game(GameAction(ActionId::new("v_target_lock")))
        );
    }

    #[test]
    fn resolve_app_action() {
        assert_eq!(
            catalog().resolve("app.toggle_lock").unwrap(),
            Action::App(AppAction::ToggleLock)
        );
        assert_eq!(
            catalog().resolve("app.go_to_page:3").unwrap(),
            Action::App(AppAction::GoToPage(3))
        );
    }

    #[test]
    fn quit_is_a_reserved_app_action() {
        assert_eq!(
            catalog().resolve("app.quit").unwrap(),
            Action::App(AppAction::Quit)
        );
    }

    #[test]
    fn resolve_unknown_action_dies() {
        let err = catalog().resolve("rm -rf /").unwrap_err();
        assert!(matches!(err, DomainError::UnknownAction(_)));
    }

    #[test]
    fn empty_key_sequence_is_unconstructable() {
        assert_eq!(
            KeySequence::new(vec![]).unwrap_err(),
            DomainError::EmptyKeySequence
        );
    }

    #[test]
    fn key_sequence_carries_steps() {
        let seq = KeySequence::new(vec![
            KeyStep::press(ScanCode::new(30)),
            KeyStep::hold(ScanCode::new(42), Duration::from_millis(50)),
        ])
        .unwrap();
        assert_eq!(seq.steps().len(), 2);
    }

    fn chords(n: usize) -> Vec<KeyChord> {
        (0..n)
            .map(|_| KeyChord::single(ScanCode::new(30)))
            .collect()
    }

    #[test]
    fn key_macro_rejects_empty() {
        assert_eq!(
            KeyMacro::new(vec![]).unwrap_err(),
            DomainError::EmptyKeySequence
        );
    }

    #[test]
    fn key_macro_rejects_seventeen_steps() {
        assert_eq!(
            KeyMacro::new(chords(17)).unwrap_err(),
            DomainError::MacroTooLong { len: 17, max: 16 }
        );
    }

    #[test]
    fn key_macro_accepts_sixteen_steps() {
        let keys = KeyMacro::new(chords(16)).unwrap();
        assert_eq!(keys.steps().len(), KeyMacro::MAX_STEPS);
    }

    #[test]
    fn chord_rejects_repeated_modifier() {
        let shift = ScanCode::new(42);
        assert_eq!(
            KeyChord::new(vec![shift, shift], ScanCode::new(2)).unwrap_err(),
            DomainError::InvalidChord { code: 42 }
        );
    }

    #[test]
    fn chord_rejects_key_equal_to_modifier() {
        let shift = ScanCode::new(42);
        assert_eq!(
            KeyChord::new(vec![shift], shift).unwrap_err(),
            DomainError::InvalidChord { code: 42 }
        );
    }

    #[test]
    fn chord_keeps_modifiers_in_order() {
        let chord =
            KeyChord::new(vec![ScanCode::new(29), ScanCode::new(42)], ScanCode::new(2)).unwrap();
        assert_eq!(chord.modifiers(), &[ScanCode::new(29), ScanCode::new(42)]);
        assert_eq!(chord.key(), ScanCode::new(2));
    }

    #[test]
    fn lock_blocks_navigation() {
        let mut state = RuntimeState::new(SidebarSide::Right);
        state.toggle_lock();
        assert!(state.is_locked());
        assert_eq!(
            state.navigate(NavDirection::Next, 3).unwrap_err(),
            DomainError::LockEngaged
        );
    }

    #[test]
    fn navigation_respects_bounds() {
        let mut state = RuntimeState::new(SidebarSide::Left);
        state.go_to(2, 3).unwrap();
        assert_eq!(state.current_page(), 2);
        assert!(matches!(
            state.go_to(3, 3).unwrap_err(),
            DomainError::PageOutOfRange {
                requested: 3,
                max: 2
            }
        ));
        // Prev from page 0 is also out of range.
        let mut at_zero = RuntimeState::new(SidebarSide::Left);
        assert!(matches!(
            at_zero.navigate(NavDirection::Prev, 3).unwrap_err(),
            DomainError::PageOutOfRange { .. }
        ));
    }

    #[test]
    fn toggle_flips_and_reports_new_state() {
        let mut state = RuntimeState::new(SidebarSide::Left);
        let id = ComponentId::new("shield_fore");
        assert!(!state.toggle_state(&id));
        assert!(state.flip_toggle(&id));
        assert!(state.toggle_state(&id));
        assert!(!state.flip_toggle(&id));
    }

    #[test]
    fn switch_sidebar_flips_side() {
        let mut state = RuntimeState::new(SidebarSide::Left);
        state.switch_sidebar();
        assert_eq!(state.sidebar_side(), SidebarSide::Right);
    }

    fn component(id: &str, action: Action) -> Component {
        Component {
            id: ComponentId::new(id),
            position: ComponentPosition { row: 0, col: 0 },
            visual: VisualProps::default(),
            action,
        }
    }

    fn meta() -> TemplateMetadata {
        TemplateMetadata {
            game: "TestGame".into(),
            version: "4.0".into(),
            author: "dev".into(),
            schema_version: SchemaVersion("1".into()),
        }
    }

    #[test]
    fn template_rejects_empty() {
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        assert_eq!(
            Template::new(meta(), vec![], sidebar).unwrap_err(),
            DomainError::EmptyTemplate
        );
    }

    #[test]
    fn template_rejects_duplicate_ids() {
        let page = Page::new(vec![
            component("dup", Action::App(AppAction::ToggleLock)),
            component("dup", Action::App(AppAction::NextPage)),
        ])
        .unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        assert!(matches!(
            Template::new(meta(), vec![page], sidebar).unwrap_err(),
            DomainError::DuplicateComponentId(_)
        ));
    }

    #[test]
    fn template_rejects_out_of_range_goto() {
        let page = Page::new(vec![component("jump", Action::App(AppAction::GoToPage(5)))]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        assert!(matches!(
            Template::new(meta(), vec![page], sidebar).unwrap_err(),
            DomainError::InvalidPageReference(5)
        ));
    }

    #[test]
    fn template_finds_components() {
        let page = Page::new(vec![component(
            "btn",
            Action::Game(GameAction(ActionId::new("v_eject"))),
        )])
        .unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![component("lock", Action::App(AppAction::ToggleLock))],
        };
        let template = Template::new(meta(), vec![page], sidebar).unwrap();
        assert!(template.find_component(&ComponentId::new("btn")).is_some());
        assert!(template.find_component(&ComponentId::new("lock")).is_some());
        assert!(template.find_component(&ComponentId::new("nope")).is_none());
    }

    fn one_page_template() -> Template {
        let page = Page::new(vec![component("btn", Action::App(AppAction::NextPage))]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        Template::new(meta(), vec![page], sidebar).unwrap()
    }

    #[test]
    fn template_new_has_an_empty_theme() {
        assert_eq!(one_page_template().theme().iter().count(), 0);
    }

    #[test]
    fn with_theme_carries_the_theme() {
        let catalog = ThemeCatalog::new(vec![TokenSpec {
            path: TokenPath::new("color.button.background").unwrap(),
            default: TokenValue::Color(Srgb8 {
                r: 38,
                g: 42,
                b: 59,
            }),
            themeable: true,
            rule: TokenRule::Any,
        }])
        .unwrap();
        let theme = catalog.defaults();
        let template = one_page_template().with_theme(theme.clone());
        assert_eq!(template.theme(), &theme);
        assert_eq!(template.theme().iter().count(), 1);
    }
}
