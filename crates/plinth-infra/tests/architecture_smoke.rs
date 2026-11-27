//! Whole-architecture smoke test, **CI-safe** (no `/dev/uinput`, no SC, no root).
//! It is the analogue of the roadmap's "vertical slice" without the physical
//! device: it loads a real template (`template_fs`) and a real SC export
//! (`keybinds_sc`), builds the catalog from the provider, and exercises
//! `ExecuteAction` with a `FakeInputSender`/`FakeClock`. It verifies that tapping
//! a game button emits the key the keybind dictates, traversing all layers and
//! both heavyweight ports (except the physical uinput adapter).

use std::path::PathBuf;

use plinth_application::testing::{FakeClock, FakeInputSender};
use plinth_application::{execute_action, query_render_state, KeyEvent};
use plinth_domain::{ComponentId, RuntimeState, ScanCode, SidebarSide};
use plinth_infra::keybinds_sc::ScKeybindProvider;
use plinth_infra::template_fs::FsTemplateSource;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn vertical_slice_with_fakes() {
    // 1. Keybind provider from the real SC export.
    let provider =
        ScKeybindProvider::from_export_file(fixture("exported.xml")).expect("parse SC export");

    // 2. The domain catalog is fed by the provider (composition root).
    let catalog = plinth_application::GameKeybindProvider::catalog(&provider);

    // 3. Template loader with that catalog as the legality authority.
    let source = FsTemplateSource::new(catalog).expect("compile schema");
    let template = {
        use plinth_application::TemplateSource;
        source
            .load(fixture("template.yaml").to_str().unwrap())
            .expect("load template")
    };

    // 4. State + fake ports.
    let mut state = RuntimeState::new(SidebarSide::Right);
    let input = FakeInputSender::new();
    let clock = FakeClock::new();

    // 5. Tap the "eject" button (kb1_y → KEY_Y == 21).
    execute_action(
        &ComponentId::new("eject"),
        &template,
        &mut state,
        &input,
        &provider,
        &clock,
    )
    .expect("execute game action");

    let key_y = ScanCode::new(21);
    assert_eq!(
        input.recorded(),
        vec![KeyEvent::down(key_y), KeyEvent::up(key_y)],
        "tapping 'eject' must emit the keybind's key (Y)"
    );

    // 6. The render DTO projects the two pages and the sidebar.
    let render = query_render_state(&template, &state, 1);
    assert_eq!(render.pages.len(), 2);
    assert_eq!(render.sidebar.len(), 1);
}

#[test]
fn app_button_navigates_without_input() {
    let provider = ScKeybindProvider::from_export_file(fixture("exported.xml")).unwrap();
    let catalog = plinth_application::GameKeybindProvider::catalog(&provider);
    let source = FsTemplateSource::new(catalog).unwrap();
    let template = {
        use plinth_application::TemplateSource;
        source
            .load(fixture("template.yaml").to_str().unwrap())
            .unwrap()
    };

    let mut state = RuntimeState::new(SidebarSide::Right);
    let input = FakeInputSender::new();
    let clock = FakeClock::new();

    // Advance to page 1 by manipulating state, then "back" (app.prev_page).
    plinth_application::go_to_page(&mut state, &template, 1).unwrap();
    execute_action(
        &ComponentId::new("back"),
        &template,
        &mut state,
        &input,
        &provider,
        &clock,
    )
    .unwrap();

    assert_eq!(state.current_page(), 0);
    assert!(input.recorded().is_empty(), "an app button emits no keys");
}
