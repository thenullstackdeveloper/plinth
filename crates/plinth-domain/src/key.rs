//! Value objects related to keys and sequences.
//!
//! The domain reasons in terms of **physical scancodes** (the key in the
//! top-left row, not the letter it produces): the compositor applies the layout
//! on top. Games usually bind by physical key, so this fits (see
//! `docs/input-linux.md`). Here we only store the code number; translating each
//! game's token → `ScanCode` is the infrastructure layer's job, not the
//! domain's.

use std::time::Duration;

use crate::error::DomainError;

/// A physical scancode (e.g. the "A" key == 30). An opaque newtype: the domain
/// does not interpret the value, it only carries it to the input layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScanCode(pub u16);

impl ScanCode {
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// How a key is actuated within a sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStepKind {
    /// Press and release immediately.
    Press,
    /// Hold down for a duration and release.
    Hold(Duration),
    /// Toggle: the state is held by `RuntimeState`; the application layer decides
    /// whether to emit a press based on the previous state.
    Toggle,
}

/// A step of a sequence: a key with its actuation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyStep {
    pub code: ScanCode,
    pub kind: KeyStepKind,
}

impl KeyStep {
    pub const fn press(code: ScanCode) -> Self {
        Self {
            code,
            kind: KeyStepKind::Press,
        }
    }

    pub const fn hold(code: ScanCode, dur: Duration) -> Self {
        Self {
            code,
            kind: KeyStepKind::Hold(dur),
        }
    }

    pub const fn toggle(code: ScanCode) -> Self {
        Self {
            code,
            kind: KeyStepKind::Toggle,
        }
    }
}

/// A non-empty key sequence. Smart constructor: an empty sequence is
/// unconstructable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySequence(Vec<KeyStep>);

impl KeySequence {
    pub fn new(steps: Vec<KeyStep>) -> Result<Self, DomainError> {
        if steps.is_empty() {
            return Err(DomainError::EmptyKeySequence);
        }
        Ok(Self(steps))
    }

    /// Shortcut for the single-press sequence (the most common case).
    pub fn single_press(code: ScanCode) -> Self {
        Self(vec![KeyStep::press(code)])
    }

    pub fn steps(&self) -> &[KeyStep] {
        &self.0
    }
}

/// A key resolved from the game's real configuration: the result of
/// `GameKeybindProvider::resolve`. It is a `KeySequence` already ready to emit.
pub type KeyBinding = KeySequence;

/// The side the sidebar lives on. For right-/left-handed use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSide {
    Left,
    Right,
}

impl SidebarSide {
    pub fn flip(self) -> Self {
        match self {
            SidebarSide::Left => SidebarSide::Right,
            SidebarSide::Right => SidebarSide::Left,
        }
    }
}

/// Version of the template schema the template was created with. Traceability of
/// keybind drift across patches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion(pub String);

/// Position of a component within its page (logical grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentPosition {
    pub row: u16,
    pub col: u16,
}

/// Purely visual properties of a component (label, icon…). The domain does not
/// interpret them; it carries them to the UI via the DTO.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VisualProps {
    pub label: Option<String>,
    pub icon: Option<String>,
}
