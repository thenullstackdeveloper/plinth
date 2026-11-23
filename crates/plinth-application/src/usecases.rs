//! Use cases = inbound ports. Thin functions that orchestrate the domain and the
//! outbound ports. The UI layer (in a later phase) calls these entry points; it
//! never touches the ports or the domain directly.

use plinth_domain::{
    Action, AppAction, Component, ComponentId, DomainError, GameAction, KeySequence, KeyStepKind,
    NavDirection, RuntimeState, Template,
};

use crate::dto::{ActionKind, ComponentView, PageView, RenderState};
use crate::ports::{
    Clock, GameKeybindProvider, InputSender, KeyEvent, ProviderError, ResolveError, TemplateError,
    TemplateSource,
};

/// The unified error of the application layer.
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

/// What the host must do after an action. The engine never closes anything:
/// it reports a quit and the host decides how to end the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOutcome {
    Continue,
    Quit,
}

/// LoadTemplate — loads an already-validated template from a `TemplateSource`.
pub fn load_template<S: TemplateSource>(source: &S, reference: &str) -> Result<Template, AppError> {
    Ok(source.load(reference)?)
}

/// RefreshKeybinds — re-reads the game's real keybind configuration.
pub fn refresh_keybinds<P: GameKeybindProvider>(provider: &mut P) -> Result<(), AppError> {
    Ok(provider.refresh()?)
}

/// Navigate — navigates one page (respects the lock and the bounds).
pub fn navigate(
    state: &mut RuntimeState,
    template: &Template,
    dir: NavDirection,
) -> Result<(), AppError> {
    state.navigate(dir, template.page_count())?;
    Ok(())
}

/// GoToPage — jumps to a specific page.
pub fn go_to_page(
    state: &mut RuntimeState,
    template: &Template,
    idx: usize,
) -> Result<(), AppError> {
    state.go_to(idx, template.page_count())?;
    Ok(())
}

/// ToggleLock — toggles the lock.
pub fn toggle_lock(state: &mut RuntimeState) {
    state.toggle_lock();
}

/// SwitchSidebarSide — switches the sidebar side.
pub fn switch_sidebar_side(state: &mut RuntimeState) {
    state.switch_sidebar();
}

/// ExecuteAction — the heart. Dispatches a component's action.
///
/// The split by type is the security guarantee: an `AppAction` only mutates the
/// `RuntimeState` (never injects keys); a `GameAction` only emits keys (never
/// touches navigation). A button cannot do both.
pub fn execute_action<I, P, C>(
    component_id: &ComponentId,
    template: &Template,
    state: &mut RuntimeState,
    input: &I,
    provider: &P,
    clock: &C,
) -> Result<ActionOutcome, AppError>
where
    I: InputSender,
    P: GameKeybindProvider,
    C: Clock,
{
    let component = template
        .find_component(component_id)
        .ok_or_else(|| DomainError::UnknownComponent(component_id.as_str().to_string()))?;

    match &component.action {
        // Quitting is guarded by the lock, like page changes, so a tap in the
        // middle of a fight cannot close the panel.
        Action::App(AppAction::Quit) if state.is_locked() => Err(DomainError::LockEngaged.into()),
        Action::App(AppAction::Quit) => Ok(ActionOutcome::Quit),
        Action::App(app) => {
            apply_app_action(app, template, state)?;
            Ok(ActionOutcome::Continue)
        }
        Action::Game(game) => {
            let binding = resolve_binding(provider, game)?;
            run_sequence(&binding, &component.id, state, input, clock)?;
            Ok(ActionOutcome::Continue)
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
        // Handled by `execute_action` before reaching here.
        AppAction::Quit => Ok(()),
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

/// Expands a `KeySequence` into input events, interleaving `Clock::sleep` on
/// `Hold` steps. `Toggle` steps read/write the component's toggle state in the
/// `RuntimeState`.
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
                // A press that flips the state in the game; we mirror the
                // perceived state in the runtime so the UI can highlight it.
                state.flip_toggle(component_id);
                input.emit(&[KeyEvent::down(step.code), KeyEvent::up(step.code)])?;
            }
        }
    }
    Ok(())
}

/// QueryRenderState — projects the current state into a DTO for the UI.
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
