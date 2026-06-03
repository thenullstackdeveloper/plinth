//! Casos de uso = puertos de entrada. Funciones finas que orquestan el dominio
//! y los puertos de salida. La capa de UI (en una fase posterior) invoca estos
//! puntos de entrada; nunca toca los puertos ni el dominio directamente.

use plinth_domain::{
    Action, AppAction, Component, ComponentId, DomainError, GameAction, KeySequence, KeyStepKind,
    NavDirection, RuntimeState, Template,
};

use crate::dto::{ActionKind, ComponentView, PageView, RenderState};
use crate::ports::{
    Clock, GameKeybindProvider, InputSender, KeyEvent, ProviderError, ResolveError, TemplateError,
    TemplateSource,
};

/// El error unificado de la capa de aplicación.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error(transparent)]
    Input(#[from] crate::ports::InputError),
    #[error(transparent)]
    Template(#[from] TemplateError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

/// LoadTemplate — carga una plantilla ya validada desde un `TemplateSource`.
pub fn load_template<S: TemplateSource>(source: &S, reference: &str) -> Result<Template, AppError> {
    Ok(source.load(reference)?)
}

/// RefreshKeybinds — relee la configuración real de keybinds del juego.
pub fn refresh_keybinds<P: GameKeybindProvider>(provider: &mut P) -> Result<(), AppError> {
    Ok(provider.refresh()?)
}

/// Navigate — navega una página (respeta el candado y los límites).
pub fn navigate(
    state: &mut RuntimeState,
    template: &Template,
    dir: NavDirection,
) -> Result<(), AppError> {
    state.navigate(dir, template.page_count())?;
    Ok(())
}

/// GoToPage — salta a una página concreta.
pub fn go_to_page(
    state: &mut RuntimeState,
    template: &Template,
    idx: usize,
) -> Result<(), AppError> {
    state.go_to(idx, template.page_count())?;
    Ok(())
}

/// ToggleLock — alterna el candado.
pub fn toggle_lock(state: &mut RuntimeState) {
    state.toggle_lock();
}

/// SwitchSidebarSide — cambia la barra lateral de lado.
pub fn switch_sidebar_side(state: &mut RuntimeState) {
    state.switch_sidebar();
}

/// ExecuteAction — el corazón. Despacha la acción de un componente.
///
/// La separación por tipo es la garantía de seguridad: una `AppAction` solo
/// muta el `RuntimeState` (nunca inyecta teclas); una `GameAction` solo emite
/// teclas (nunca toca la navegación). Un botón no puede hacer ambas cosas.
pub fn execute_action<I, P, C>(
    component_id: &ComponentId,
    template: &Template,
    state: &mut RuntimeState,
    input: &I,
    provider: &P,
    clock: &C,
) -> Result<(), AppError>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
{
    let component = template
        .find_component(component_id)
        .ok_or_else(|| DomainError::UnknownComponent(component_id.as_str().to_string()))?;

    match &component.action {
        Action::App(app) => {
            apply_app_action(app, template, state)?;
            Ok(())
        }
        Action::Game(game) => {
            let binding = resolve_binding(provider, game)?;
            run_sequence(&binding, &component.id, state, input, clock)
        }
    }
}

fn apply_app_action(
    app: &AppAction,
    template: &Template,
    state: &mut RuntimeState,
) -> Result<(), DomainError> {
    match app {
        AppAction::NextPage => state.navigate(NavDirection::Next, template.page_count()),
        AppAction::PrevPage => state.navigate(NavDirection::Prev, template.page_count()),
        AppAction::GoToPage(idx) => state.go_to(*idx, template.page_count()),
        AppAction::ToggleLock => {
            state.toggle_lock();
            Ok(())
        }
        AppAction::SwitchSidebarSide => {
            state.switch_sidebar();
            Ok(())
        }
    }
}

fn resolve_binding<P: GameKeybindProvider>(
    provider: &P,
    game: &GameAction,
) -> Result<KeySequence, AppError> {
    provider.resolve(game).map_err(|e| match e {
        ResolveError::NotBound(g) => AppError::Domain(DomainError::ActionNotBound(g.0)),
        other => AppError::Resolve(other),
    })
}

/// Expande una `KeySequence` en eventos de input intercalando `Clock::sleep`
/// en los pasos `Hold`. Los pasos `Toggle` consultan/escriben el estado de
/// toggle del componente en el `RuntimeState`.
fn run_sequence<I: InputSender, C: Clock>(
    sequence: &KeySequence,
    component_id: &ComponentId,
    state: &mut RuntimeState,
    input: &I,
    clock: &C,
) -> Result<(), AppError> {
    for step in sequence.steps() {
        match step.kind {
            KeyStepKind::Press => {
                input.emit(&[KeyEvent::down(step.code), KeyEvent::up(step.code)])?;
            }
            KeyStepKind::Hold(dur) => {
                input.emit(&[KeyEvent::down(step.code)])?;
                clock.sleep(dur);
                input.emit(&[KeyEvent::up(step.code)])?;
            }
            KeyStepKind::Toggle => {
                // Una pulsación que voltea el estado en el juego; reflejamos el
                // estado percibido en el runtime para que la UI lo resalte.
                state.flip_toggle(component_id);
                input.emit(&[KeyEvent::down(step.code), KeyEvent::up(step.code)])?;
            }
        }
    }
    Ok(())
}

/// QueryRenderState — proyecta el estado actual a un DTO para la UI.
pub fn query_render_state(template: &Template, state: &RuntimeState) -> RenderState {
    let to_view = |c: &Component| ComponentView {
        id: c.id.as_str().to_string(),
        row: c.position.row,
        col: c.position.col,
        label: c.visual.label.clone(),
        icon: c.visual.icon.clone(),
        action_kind: match c.action {
            Action::App(_) => ActionKind::App,
            Action::Game(_) => ActionKind::Game,
        },
        toggled: state.toggle_state(&c.id),
    };

    let pages = template
        .pages()
        .iter()
        .map(|p| PageView {
            components: p.components().iter().map(to_view).collect(),
        })
        .collect();

    let sidebar = template.sidebar().components.iter().map(to_view).collect();

    RenderState {
        pages,
        current_page: state.current_page(),
        sidebar,
        sidebar_side: state.sidebar_side(),
        locked: state.is_locked(),
    }
}
