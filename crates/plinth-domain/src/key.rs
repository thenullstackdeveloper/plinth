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
use crate::image::ImageRef;

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

/// A literal key with the modifiers held around it (`shift+1`): one step of a
/// [`KeyMacro`]. The domain does not know which codes are modifiers; it only
/// guarantees that no code appears twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChord {
    modifiers: Vec<ScanCode>,
    key: ScanCode,
}

impl KeyChord {
    /// Builds a chord. A modifier repeated, or equal to the key, is rejected.
    pub fn new(modifiers: Vec<ScanCode>, key: ScanCode) -> Result<Self, DomainError> {
        let mut seen = std::collections::HashSet::new();
        for code in modifiers.iter().chain(std::iter::once(&key)) {
            if !seen.insert(*code) {
                return Err(DomainError::InvalidChord { code: code.get() });
            }
        }
        Ok(Self { modifiers, key })
    }

    /// A bare key, without modifiers.
    pub fn single(key: ScanCode) -> Self {
        Self {
            modifiers: Vec::new(),
            key,
        }
    }

    /// The modifiers, in the order they are pressed.
    pub fn modifiers(&self) -> &[ScanCode] {
        &self.modifiers
    }

    pub fn key(&self) -> ScanCode {
        self.key
    }
}

/// The literal keys a template binds to a button: a non-empty, bounded list
/// of chords fired one after the other on a single tap. It never resolves
/// through a game provider, and it holds nothing but key codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMacro(Vec<KeyChord>);

impl KeyMacro {
    /// The longest macro a single tap may fire.
    pub const MAX_STEPS: usize = 16;

    pub fn new(steps: Vec<KeyChord>) -> Result<Self, DomainError> {
        if steps.is_empty() {
            return Err(DomainError::EmptyKeySequence);
        }
        if steps.len() > Self::MAX_STEPS {
            return Err(DomainError::MacroTooLong {
                len: steps.len(),
                max: Self::MAX_STEPS,
            });
        }
        Ok(Self(steps))
    }

    pub fn steps(&self) -> &[KeyChord] {
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

/// The highest minor of template format 1 this Plinth reads. Every minor
/// only adds optional fields, so a template of an older minor still loads.
pub const SUPPORTED_MINOR: u16 = 0;

/// The template format a template was written for. Once parsed it holds the
/// version as `1.<minor>`: `"1"` and `"1.0"` are the same version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion(pub String);

impl SchemaVersion {
    /// `1` or `1.<minor>`, with the minor written without leading zeros and
    /// in at most 4 digits, and no later than `SUPPORTED_MINOR`.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let unsupported = || DomainError::UnsupportedFormat(raw.to_string());
        let minor = match raw.strip_prefix('1') {
            Some("") => 0,
            Some(rest) => {
                let digits = rest.strip_prefix('.').ok_or_else(unsupported)?;
                let plain = digits == "0"
                    || (matches!(digits.len(), 1..=4)
                        && !digits.starts_with('0')
                        && digits.bytes().all(|b| b.is_ascii_digit()));
                if !plain {
                    return Err(unsupported());
                }
                digits.parse::<u16>().map_err(|_| unsupported())?
            }
            None => return Err(unsupported()),
        };
        if minor > SUPPORTED_MINOR {
            return Err(DomainError::NeedsNewerPlinth { minor });
        }
        Ok(Self(format!("1.{minor}")))
    }
}

/// Position of a component within its page (logical grid): its first cell,
/// and how many rows and columns it covers from there (1 for a button of one
/// cell). In the sidebar both spans are 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentPosition {
    pub row: u16,
    pub col: u16,
    pub row_span: u16,
    pub col_span: u16,
}

/// Purely visual properties of a component (label, icon, image…). The domain
/// does not interpret them; it carries them to the UI via the DTO. Where each
/// one may be used is a rule of `Template`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VisualProps {
    pub label: Option<String>,
    /// A small image next to the label, or alone with `hide_label`.
    pub icon: Option<ImageRef>,
    /// An image that fills the face of the button.
    pub image: Option<ImageRef>,
    /// The label is hidden from view but stays the button's accessible name.
    pub hide_label: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_1_and_1_0_are_the_same() {
        assert_eq!(SchemaVersion::parse("1"), SchemaVersion::parse("1.0"));
        assert_eq!(
            SchemaVersion::parse("1"),
            Ok(SchemaVersion("1.0".to_string()))
        );
    }

    #[test]
    fn leading_zeros_and_long_minors_are_rejected() {
        for raw in ["1.01", "1.00", "1.12345", "1.", "1.0.0", "01", "1.a"] {
            assert_eq!(
                SchemaVersion::parse(raw),
                Err(DomainError::UnsupportedFormat(raw.to_string())),
                "{raw}"
            );
        }
    }
}
