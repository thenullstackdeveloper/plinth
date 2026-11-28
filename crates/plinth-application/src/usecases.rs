//! Use cases = inbound ports. Thin functions that orchestrate the domain and the
//! outbound ports. The UI layer (in a later phase) calls these entry points; it
//! never touches the ports or the domain directly.

use std::time::Duration;

use plinth_domain::{
    Action, AppAction, Component, ComponentId, DomainError, GameAction, ImageRef, KeyMacro,
    KeySequence, KeyStepKind, NavDirection, RuntimeState, Template,
};

use crate::dto::{ActionKind, ComponentView, PageView, RenderState};
use crate::ports::{
    Clock, GameKeybindProvider, InputSender, KeyEvent, ProviderError, ResolveError, TemplateError,
    TemplateSource,
};
use crate::theme_css::{override_vars, theme_vars};

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

/// The pause between two steps of a literal macro, so the game reads each one.
/// None before the first step or after the last.
pub const MACRO_STEP_GAP: Duration = Duration::from_millis(30);

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
        Action::Keys(keys) => {
            run_macro(keys, input, clock)?;
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

/// Fires a literal macro: each chord in one batch (modifiers down in order,
/// key down, key up, modifiers up in reverse), with [`MACRO_STEP_GAP`]
/// between chords. It never reads or writes the runtime state.
fn run_macro<I: InputSender, C: Clock>(
    keys: &KeyMacro,
    input: &I,
    clock: &C,
) -> Result<(), AppError> {
    for (i, chord) in keys.steps().iter().enumerate() {
        if i > 0 {
            clock.sleep(MACRO_STEP_GAP);
        }
        let mut events: Vec<KeyEvent> = chord
            .modifiers()
            .iter()
            .map(|&code| KeyEvent::down(code))
            .collect();
        events.push(KeyEvent::down(chord.key()));
        events.push(KeyEvent::up(chord.key()));
        events.extend(
            chord
                .modifiers()
                .iter()
                .rev()
                .map(|&code| KeyEvent::up(code)),
        );
        input.emit(&events)?;
    }
    Ok(())
}

/// The id of a template's image for the UI: `t<generation>i<index>`. The
/// host raises `generation` on every template it loads, so an id never
/// points to an image of the template before.
pub fn image_id(generation: u64, image: ImageRef) -> String {
    format!("t{generation}i{}", image.0)
}

/// QueryRenderState — projects the current state into a DTO for the UI.
/// `generation` is the host's count of loaded templates, for the image ids.
pub fn query_render_state(
    template: &Template,
    state: &RuntimeState,
    generation: u64,
) -> RenderState {
    let to_view = |c: &Component| ComponentView {
        id: c.id.as_str().to_string(),
        row: c.position.row,
        col: c.position.col,
        row_span: c.position.row_span,
        col_span: c.position.col_span,
        label: c.visual.label.clone(),
        icon_id: c.visual.icon.map(|icon| image_id(generation, icon)),
        image_id: c.visual.image.map(|image| image_id(generation, image)),
        hide_label: c.visual.hide_label,
        style: override_vars(&c.style),
        action_kind: match c.action {
            Action::App(_) => ActionKind::App,
            Action::Game(_) => ActionKind::Game,
            Action::Keys(_) => ActionKind::Keys,
        },
        toggled: state.toggle_state(&c.id),
    };

    let pages = template
        .pages()
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let grid = p.grid_size();
            PageView {
                components: p.components().iter().map(to_view).collect(),
                rows: grid.rows,
                cols: grid.cols,
                background_id: template
                    .page_background(index)
                    .map(|background| image_id(generation, background)),
            }
        })
        .collect();

    // The sidebar is one column, shown top to bottom by `row` (then `col`),
    // whatever order the template lists its buttons in. The sort is stable.
    let mut sidebar: Vec<ComponentView> =
        template.sidebar().components.iter().map(to_view).collect();
    sidebar.sort_by_key(|c| (c.row, c.col));

    RenderState {
        pages,
        current_page: state.current_page(),
        sidebar,
        sidebar_side: state.sidebar_side(),
        locked: state.is_locked(),
        theme: theme_vars(template.theme()),
        page_transition_ms: template.theme().ms("motion.page-transition").unwrap_or(0),
    }
}
