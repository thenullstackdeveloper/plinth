//! Smoke test de toda la arquitectura, **CI-safe** (sin `/dev/uinput`, sin SC,
//! sin root). Es el análogo del "slice vertical" del roadmap sin el device
//! físico: carga una plantilla real (`template_fs`) y un export real de SC
//! (`keybinds_sc`), construye el catálogo desde el proveedor, y ejercita
//! `ExecuteAction` con un `FakeInputSender`/`FakeClock`. Verifica que tocar un
//! botón de juego emite la tecla que dicta el keybind, recorriendo todas las
//! capas y los dos puertos pesados (salvo el adaptador uinput físico).

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
    // 1. Proveedor de keybinds desde el export real de SC.
    let provider =
        ScKeybindProvider::from_export_file(fixture("exported.xml")).expect("parsear export de SC");

    // 2. El catálogo del dominio se alimenta del proveedor (composition root).
    let catalog = plinth_application::GameKeybindProvider::catalog(&provider);

    // 3. Cargador de plantillas con ese catálogo como autoridad de legalidad.
    let source = FsTemplateSource::new(catalog).expect("compilar schema");
    let template = {
        use plinth_application::TemplateSource;
        source
            .load(fixture("template.yaml").to_str().unwrap())
            .expect("cargar plantilla")
    };

    // 4. Estado + puertos falsos.
    let mut state = RuntimeState::new(SidebarSide::Right);
    let input = FakeInputSender::new();
    let clock = FakeClock::new();

    // 5. Tocar el botón "eject" (kb1_y → KEY_Y == 21).
    execute_action(
        &ComponentId::new("eject"),
        &template,
        &mut state,
        &input,
        &provider,
        &clock,
    )
    .expect("ejecutar acción de juego");

    let key_y = ScanCode::new(21);
    assert_eq!(
        input.recorded(),
        vec![KeyEvent::down(key_y), KeyEvent::up(key_y)],
        "tocar 'eject' debe emitir la tecla del keybind (Y)"
    );

    // 6. El DTO de render proyecta las dos páginas y la sidebar.
    let render = query_render_state(&template, &state);
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

    // Avanza a la página 1 manipulando el estado, luego "back" (app.prev_page).
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
    assert!(
        input.recorded().is_empty(),
        "un botón de app no emite teclas"
    );
}
