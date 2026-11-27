//! `plinth-domain` — the product's language, pure and game-agnostic.
//!
//! This crate depends on no infrastructure crate and no logic specific to any
//! particular game. It is the architecture's golden rule: if any of that appears
//! here, it **won't compile**, and the CI fitness test catches it.

pub mod action;
pub mod catalog;
pub mod error;
pub mod image;
pub mod key;
pub mod runtime;
pub mod template;
pub mod theme;

pub use action::{Action, ActionId, AppAction, GameAction};
pub use catalog::ActionCatalog;
pub use error::DomainError;
pub use image::{ImageAsset, ImageKind, ImageLimits, ImagePath, ImageRef, ImageUse};
pub use key::{
    ComponentPosition, KeyBinding, KeyChord, KeyMacro, KeySequence, KeyStep, KeyStepKind, ScanCode,
    SchemaVersion, SidebarSide, VisualProps,
};
pub use runtime::{NavDirection, RuntimeState};
pub use template::{Component, ComponentId, Page, Sidebar, Template, TemplateMetadata};
pub use theme::{
    GridSize, Hundredths, PanelSize, Srgb8, Theme, ThemeCatalog, TokenKind, TokenPath, TokenRule,
    TokenSpec, TokenValue, LAYOUT_TOKENS,
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

    fn image(path: &str) -> ImageAsset {
        ImageAsset {
            path: ImagePath::parse(path).unwrap(),
            kind: ImageKind::Png,
            width: 1,
            height: 1,
            bytes: vec![1u8, 2, 3].into(),
        }
    }

    #[test]
    fn with_images_rejects_a_reference_to_no_image() {
        let mut button = component("btn", Action::App(AppAction::NextPage));
        button.visual.image = Some(ImageRef(1));
        let page = Page::new(vec![button]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        let template = Template::new(meta(), vec![page], sidebar).unwrap();

        assert_eq!(
            template.clone().with_images(vec![image("a.png")]),
            Err(DomainError::UnknownImage(1))
        );
        let with = template
            .with_images(vec![image("a.png"), image("b.png")])
            .unwrap();
        let paths: Vec<&str> = with.images().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, vec!["a.png", "b.png"]);
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

    // --- Fitting the panel and one cell per button ---

    const PANEL: PanelSize = PanelSize {
        width: 900,
        height: 300,
    };

    fn at(id: &str, row: u16, col: u16) -> Component {
        Component {
            position: ComponentPosition { row, col },
            ..component(id, Action::App(AppAction::NextPage))
        }
    }

    /// The layout tokens with the defaults of `design/tokens.json`.
    fn layout_catalog_without(missing: &str) -> ThemeCatalog {
        let defaults = [
            ("size.touch.min", 44),
            ("size.button.min", 64),
            ("size.sidebar.width", 128),
            ("space.page.gap", 8),
            ("space.page.padding", 12),
            ("space.sidebar.gap", 8),
            ("space.sidebar.padding", 8),
        ];
        ThemeCatalog::new(
            defaults
                .into_iter()
                .filter(|(path, _)| *path != missing)
                .map(|(path, px)| TokenSpec {
                    path: TokenPath::new(path).unwrap(),
                    default: TokenValue::Px(px),
                    themeable: true,
                    rule: TokenRule::Any,
                })
                .collect(),
        )
        .unwrap()
    }

    /// A one-page template with the default layout tokens and `overrides`.
    fn laid_out(
        page: Vec<Component>,
        sidebar: Vec<Component>,
        overrides: &[(&str, u16)],
    ) -> Template {
        let theme = layout_catalog_without("")
            .resolve(
                overrides
                    .iter()
                    .map(|(path, px)| (path.to_string(), TokenValue::Px(*px)))
                    .collect(),
            )
            .unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: sidebar,
        };
        Template::new(meta(), vec![Page::new(page).unwrap()], sidebar)
            .unwrap()
            .with_theme(theme)
    }

    fn column(rows: u16) -> Vec<Component> {
        (0..rows).map(|r| at(&format!("r{r}"), r, 0)).collect()
    }

    fn row_of(cols: u16) -> Vec<Component> {
        (0..cols).map(|c| at(&format!("c{c}"), 0, c)).collect()
    }

    fn sidebar_of(n: usize) -> Vec<Component> {
        (0..n).map(|i| at(&format!("s{i}"), 0, 0)).collect()
    }

    #[test]
    fn grid_size_is_max_plus_one() {
        let page = Page::new(vec![at("a", 0, 0), at("b", 2, 1), at("c", 1, 4)]).unwrap();
        assert_eq!(page.grid_size(), GridSize { rows: 3, cols: 5 });
        let single = Page::new(vec![at("a", 0, 0)]).unwrap();
        assert_eq!(single.grid_size(), GridSize { rows: 1, cols: 1 });
    }

    #[test]
    fn fits_at_the_exact_height() {
        let tight = [
            ("size.button.min", 64),
            ("space.page.gap", 12),
            ("space.page.padding", 4),
        ];
        // 4·64 + 3·12 + 2·4 = 300.
        assert_eq!(laid_out(column(4), vec![], &tight).check_fit(PANEL), Ok(()));
        let over = [
            ("size.button.min", 64),
            ("space.page.gap", 12),
            ("space.page.padding", 5),
        ];
        assert_eq!(
            laid_out(column(4), vec![], &over).check_fit(PANEL),
            Err(DomainError::ThemeDoesNotFit {
                area: "page grid height",
                needed: 302,
                available: 300,
            })
        );
    }

    #[test]
    fn rejects_a_grid_too_wide() {
        // 11·64 + 10·8 + 2·12 = 808, and 900 − 128 = 772.
        let err = laid_out(row_of(11), vec![], &[])
            .check_fit(PANEL)
            .unwrap_err();
        assert_eq!(
            err,
            DomainError::ThemeDoesNotFit {
                area: "page grid width",
                needed: 808,
                available: 772,
            }
        );
        assert_eq!(
            err.to_string(),
            "the theme does not fit the panel: page grid width needs 808px but only 772px are available"
        );
        assert_eq!(laid_out(row_of(10), vec![], &[]).check_fit(PANEL), Ok(()));
    }

    #[test]
    fn a_huge_row_is_rejected() {
        // 60001·64 + 60000·8 + 2·12, in u32 and without a panic.
        let err = laid_out(vec![at("far", 60000, 0)], vec![], &[])
            .check_fit(PANEL)
            .unwrap_err();
        assert_eq!(
            err,
            DomainError::ThemeDoesNotFit {
                area: "page grid height",
                needed: 4_320_088,
                available: 300,
            }
        );
    }

    #[test]
    fn rejects_a_sidebar_too_tall() {
        // 6·44 + 5·8 + 2·8 = 320.
        assert_eq!(
            laid_out(row_of(1), sidebar_of(6), &[]).check_fit(PANEL),
            Err(DomainError::ThemeDoesNotFit {
                area: "sidebar height",
                needed: 320,
                available: 300,
            })
        );
    }

    #[test]
    fn fits_five_sidebar_buttons() {
        // 5·44 + 4·8 + 2·8 = 268.
        assert_eq!(
            laid_out(row_of(1), sidebar_of(5), &[]).check_fit(PANEL),
            Ok(())
        );
        // The same 268px do not fit a panel one pixel shorter.
        let short = PanelSize {
            width: 900,
            height: 267,
        };
        assert_eq!(
            laid_out(row_of(1), sidebar_of(5), &[]).check_fit(short),
            Err(DomainError::ThemeDoesNotFit {
                area: "sidebar height",
                needed: 268,
                available: 267,
            })
        );
    }

    #[test]
    fn an_empty_sidebar_fits() {
        // As in `wow-phase1.yaml`: no sidebar buttons, so no (n − 1) to underflow.
        assert_eq!(laid_out(row_of(1), vec![], &[]).check_fit(PANEL), Ok(()));
    }

    #[test]
    fn rejects_a_sidebar_too_narrow() {
        // Out of reach of the real catalog (96 − 2·16 ≥ 44), but the rule holds:
        // the column minus its padding must still take a finger.
        let narrow = [("size.sidebar.width", 59)];
        assert_eq!(
            laid_out(row_of(1), vec![], &narrow).check_fit(PANEL),
            Err(DomainError::ThemeDoesNotFit {
                area: "sidebar width",
                needed: 44,
                available: 43,
            })
        );
        let enough = [("size.sidebar.width", 60)];
        assert_eq!(
            laid_out(row_of(1), vec![], &enough).check_fit(PANEL),
            Ok(())
        );
    }

    #[test]
    fn check_fit_requires_the_layout_tokens() {
        for missing in LAYOUT_TOKENS {
            let theme = layout_catalog_without(missing).defaults();
            let template = one_page_template().with_theme(theme);
            assert_eq!(
                template.check_fit(PANEL),
                Err(DomainError::UnknownToken(missing.to_string())),
                "`{missing}` must be required"
            );
        }
        assert_eq!(
            one_page_template().check_fit(PANEL),
            Err(DomainError::UnknownToken(LAYOUT_TOKENS[0].to_string()))
        );
    }

    #[test]
    fn rejects_two_components_in_the_same_cell() {
        let first = Page::new(vec![at("a", 0, 0)]).unwrap();
        let second = Page::new(vec![at("b", 0, 0), at("c", 1, 2), at("d", 1, 2)]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        let err = Template::new(meta(), vec![first, second], sidebar).unwrap_err();
        assert_eq!(
            err,
            DomainError::DuplicatePosition {
                page: 1,
                row: 1,
                col: 2,
                first: "c".into(),
                second: "d".into(),
            }
        );
        assert_eq!(
            err.to_string(),
            "components `c` and `d` share the cell row 1, col 2 on page 1"
        );
    }

    #[test]
    fn sidebar_buttons_may_share_a_cell() {
        // The sidebar is a column ordered by `row`, not a grid.
        let page = Page::new(vec![at("a", 0, 0)]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![at("s0", 0, 0), at("s1", 0, 0)],
        };
        assert!(Template::new(meta(), vec![page], sidebar).is_ok());
    }

    #[test]
    fn duplicate_ids_are_checked_before_cells() {
        let page = Page::new(vec![at("dup", 0, 0), at("dup", 0, 0)]).unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        assert_eq!(
            Template::new(meta(), vec![page], sidebar).unwrap_err(),
            DomainError::DuplicateComponentId("dup".into())
        );
    }

    #[test]
    fn cells_are_checked_before_page_references() {
        let page = Page::new(vec![
            Component {
                position: ComponentPosition { row: 0, col: 0 },
                ..component("jump", Action::App(AppAction::GoToPage(5)))
            },
            at("b", 0, 0),
        ])
        .unwrap();
        let sidebar = Sidebar {
            default_side: SidebarSide::Right,
            components: vec![],
        };
        assert!(matches!(
            Template::new(meta(), vec![page], sidebar).unwrap_err(),
            DomainError::DuplicatePosition { .. }
        ));
    }
}
