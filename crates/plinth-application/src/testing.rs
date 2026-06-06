//! Reusable fakes for the application and infra tests. Behind the `testing`
//! feature so they are not compiled into production builds.

use std::cell::RefCell;
use std::time::Duration;

use plinth_domain::{ActionCatalog, GameAction, KeyBinding, SidebarSide, Template};

use crate::ports::{
    Clock, ConfigError, ConfigStore, GameKeybindProvider, InputError, InputSender, KeyEvent,
    PlacementError, Prefs, ProviderError, ResolveError, TemplateError, TemplateSource,
    WindowPlacement,
};

/// An `InputSender` that records emitted events in order, without touching the OS.
#[derive(Default)]
pub struct FakeInputSender {
    pub events: RefCell<Vec<KeyEvent>>,
}

impl FakeInputSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recorded(&self) -> Vec<KeyEvent> {
        self.events.borrow().clone()
    }
}

impl InputSender for FakeInputSender {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError> {
        self.events.borrow_mut().extend_from_slice(events);
        Ok(())
    }
}

/// A `GameKeybindProvider` with a canned table and a fixed catalog.
pub struct FakeKeybindProvider {
    bindings: Vec<(GameAction, KeyBinding)>,
    catalog: ActionCatalog,
    pub refresh_calls: RefCell<u32>,
}

impl FakeKeybindProvider {
    pub fn new(bindings: Vec<(GameAction, KeyBinding)>, catalog: ActionCatalog) -> Self {
        Self {
            bindings,
            catalog,
            refresh_calls: RefCell::new(0),
        }
    }
}

impl GameKeybindProvider for FakeKeybindProvider {
    fn resolve(&self, action: &GameAction) -> Result<KeyBinding, ResolveError> {
        self.bindings
            .iter()
            .find(|(a, _)| a == action)
            .map(|(_, b)| b.clone())
            .ok_or_else(|| ResolveError::NotBound(action.clone()))
    }

    fn refresh(&mut self) -> Result<(), ProviderError> {
        *self.refresh_calls.borrow_mut() += 1;
        Ok(())
    }

    fn catalog(&self) -> ActionCatalog {
        self.catalog.clone()
    }
}

/// A `Clock` that records `sleep` durations without actually waiting.
#[derive(Default)]
pub struct FakeClock {
    pub sleeps: RefCell<Vec<Duration>>,
}

impl FakeClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recorded_sleeps(&self) -> Vec<Duration> {
        self.sleeps.borrow().clone()
    }
}

impl Clock for FakeClock {
    fn sleep(&self, dur: Duration) {
        self.sleeps.borrow_mut().push(dur);
    }
}

/// A `TemplateSource` that returns an in-memory template.
pub struct InMemoryTemplateSource {
    template: Template,
}

impl InMemoryTemplateSource {
    pub fn new(template: Template) -> Self {
        Self { template }
    }
}

impl TemplateSource for InMemoryTemplateSource {
    fn load(&self, _reference: &str) -> Result<Template, TemplateError> {
        Ok(self.template.clone())
    }
}

/// An in-memory `ConfigStore`.
pub struct InMemoryConfigStore {
    prefs: RefCell<Prefs>,
}

impl InMemoryConfigStore {
    pub fn new(prefs: Prefs) -> Self {
        Self {
            prefs: RefCell::new(prefs),
        }
    }
}

impl Default for InMemoryConfigStore {
    fn default() -> Self {
        Self::new(Prefs {
            sidebar_side: SidebarSide::Right,
            last_template: None,
        })
    }
}

impl ConfigStore for InMemoryConfigStore {
    fn load(&self) -> Result<Prefs, ConfigError> {
        Ok(self.prefs.borrow().clone())
    }

    fn save(&self, prefs: &Prefs) -> Result<(), ConfigError> {
        *self.prefs.borrow_mut() = prefs.clone();
        Ok(())
    }
}

/// A no-op `WindowPlacement` (like the real Linux adapter).
#[derive(Default)]
pub struct NoopWindowPlacement;

impl WindowPlacement for NoopWindowPlacement {
    fn place(&self) -> Result<(), PlacementError> {
        Ok(())
    }
}
