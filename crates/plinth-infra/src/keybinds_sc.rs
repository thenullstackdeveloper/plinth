//! Star Citizen keybind adapter. **The only module that names SC.**
//!
//! Works from the **exportable XML** (`pp_rebindkeys export all <name>` →
//! `layout_<name>_exported.xml`, plain text), and optionally from the user
//! profile's `actionmaps.xml`. This retires the adapter's biggest risk: parsing
//! the binary `Data.p4k` (defaults) is **out of scope** for the MVP.
//!
//! **Known limitation (MVP):** only **single keyboard key** bindings
//! (`kb1_<key>`) are mapped. Modifier chords (`kb1_lshift+t`) and joystick/mouse
//! entries (`js1_*`, `mo1_*`) are registered in the catalog (the action is
//! "known") but produce no emittable binding → resolution returns `NotBound`.
//! The domain's `KeySequence` model does not express simultaneous presses;
//! supporting chords is a later improvement.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use plinth_application::{GameKeybindProvider, ProviderError, ResolveError};
use plinth_domain::{ActionCatalog, ActionId, GameAction, KeyBinding, KeySequence, ScanCode};
use serde::Deserialize;

/// Parse error of the SC XML.
#[derive(Debug, thiserror::Error)]
pub enum ScParseError {
    #[error("error reading the keybinds XML: {0}")]
    Io(String),
    #[error("malformed keybinds XML: {0}")]
    Xml(String),
}

// --- Structure of the exported SC XML -----------------------------------

#[derive(Debug, Deserialize)]
struct ActionMapsDoc {
    #[serde(rename = "actionmap", default)]
    actionmaps: Vec<ActionMapNode>,
}

#[derive(Debug, Deserialize)]
struct ActionMapNode {
    #[serde(rename = "action", default)]
    actions: Vec<ActionNode>,
}

#[derive(Debug, Deserialize)]
struct ActionNode {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "rebind", default)]
    rebinds: Vec<RebindNode>,
}

#[derive(Debug, Deserialize)]
struct RebindNode {
    #[serde(rename = "@input")]
    input: String,
}

/// Result of parsing: the catalog of known actions + the map of those that have
/// an emittable keyboard binding.
pub struct ParsedKeybinds {
    pub catalog: ActionCatalog,
    pub bindings: BTreeMap<ActionId, KeyBinding>,
}

/// Parses the contents of an exported SC XML.
pub fn parse_export(xml: &str) -> Result<ParsedKeybinds, ScParseError> {
    let doc: ActionMapsDoc =
        quick_xml::de::from_str(xml).map_err(|e| ScParseError::Xml(e.to_string()))?;

    let mut catalog_ids: BTreeSet<ActionId> = BTreeSet::new();
    let mut bindings: BTreeMap<ActionId, KeyBinding> = BTreeMap::new();

    for map in &doc.actionmaps {
        for action in &map.actions {
            let id = ActionId::new(&action.name);
            catalog_ids.insert(id.clone());

            // Take the first rebind we know how to represent.
            for rebind in &action.rebinds {
                if let Some(seq) = input_to_sequence(&rebind.input) {
                    bindings.entry(id.clone()).or_insert(seq);
                    break;
                }
            }
        }
    }

    Ok(ParsedKeybinds {
        catalog: ActionCatalog::from_actions(catalog_ids),
        bindings,
    })
}

/// Converts SC's `input` attribute into an emittable `KeySequence`, or `None` if
/// it is not a single keyboard key we know how to map.
fn input_to_sequence(input: &str) -> Option<KeySequence> {
    let token = input.strip_prefix("kb1_")?; // keyboard only
    if token.is_empty() || token.contains('+') {
        return None; // modifier chords: not supported in the MVP
    }
    let code = token_to_keycode(token)?;
    Some(KeySequence::single_press(ScanCode::new(code)))
}

/// SC token → evdev key code table (constants from
/// `linux/input-event-codes.h`, stable at the kernel level).
fn token_to_keycode(token: &str) -> Option<u16> {
    let code = match token {
        // Letters
        "a" => 30,
        "b" => 48,
        "c" => 46,
        "d" => 32,
        "e" => 18,
        "f" => 33,
        "g" => 34,
        "h" => 35,
        "i" => 23,
        "j" => 36,
        "k" => 37,
        "l" => 38,
        "m" => 50,
        "n" => 49,
        "o" => 24,
        "p" => 25,
        "q" => 16,
        "r" => 19,
        "s" => 31,
        "t" => 20,
        "u" => 22,
        "v" => 47,
        "w" => 17,
        "x" => 45,
        "y" => 21,
        "z" => 44,
        // Top-row digits
        "0" => 11,
        "1" => 2,
        "2" => 3,
        "3" => 4,
        "4" => 5,
        "5" => 6,
        "6" => 7,
        "7" => 8,
        "8" => 9,
        "9" => 10,
        // Control / spacing keys
        "escape" => 1,
        "minus" => 12,
        "equals" => 13,
        "backspace" => 14,
        "tab" => 15,
        "lbracket" => 26,
        "rbracket" => 27,
        "enter" => 28,
        "lctrl" => 29,
        "semicolon" => 39,
        "apostrophe" => 40,
        "grave" => 41,
        "lshift" => 42,
        "backslash" => 43,
        "comma" => 51,
        "period" => 52,
        "slash" => 53,
        "rshift" => 54,
        "lalt" => 56,
        "space" => 57,
        "capslock" => 58,
        "ralt" => 100,
        "rctrl" => 97,
        // Function
        "f1" => 59,
        "f2" => 60,
        "f3" => 61,
        "f4" => 62,
        "f5" => 63,
        "f6" => 64,
        "f7" => 65,
        "f8" => 66,
        "f9" => 67,
        "f10" => 68,
        "f11" => 87,
        "f12" => 88,
        // Navigation block
        "home" => 102,
        "up" => 103,
        "pgup" => 104,
        "left" => 105,
        "right" => 106,
        "end" => 107,
        "down" => 108,
        "pgdn" => 109,
        "insert" => 110,
        "delete" => 111,
        // Numpad
        "np_0" => 82,
        "np_1" => 79,
        "np_2" => 80,
        "np_3" => 81,
        "np_4" => 75,
        "np_5" => 76,
        "np_6" => 77,
        "np_7" => 71,
        "np_8" => 72,
        "np_9" => 73,
        "np_enter" => 96,
        "np_add" => 78,
        "np_subtract" => 74,
        "np_multiply" => 55,
        "np_divide" => 98,
        "np_period" => 83,
        _ => return None,
    };
    Some(code)
}

// --- The provider --------------------------------------------------------

/// `GameKeybindProvider` for Star Citizen. Keeps the source so it can `refresh()`
/// by re-reading the exported file after a keybind change.
pub struct ScKeybindProvider {
    parsed: ParsedKeybinds,
    source_path: Option<PathBuf>,
}

impl ScKeybindProvider {
    /// Builds the provider from the contents of an export (no file; `refresh()`
    /// will be a no-op).
    pub fn from_export_str(xml: &str) -> Result<Self, ScParseError> {
        Ok(Self {
            parsed: parse_export(xml)?,
            source_path: None,
        })
    }

    /// Builds from an exported file; `refresh()` re-reads it.
    pub fn from_export_file(path: PathBuf) -> Result<Self, ScParseError> {
        let xml = std::fs::read_to_string(&path).map_err(|e| ScParseError::Io(e.to_string()))?;
        Ok(Self {
            parsed: parse_export(&xml)?,
            source_path: Some(path),
        })
    }
}

impl GameKeybindProvider for ScKeybindProvider {
    fn resolve(&self, action: &GameAction) -> Result<KeyBinding, ResolveError> {
        self.parsed
            .bindings
            .get(&action.0)
            .cloned()
            .ok_or_else(|| ResolveError::NotBound(action.clone()))
    }

    fn refresh(&mut self) -> Result<(), ProviderError> {
        if let Some(path) = &self.source_path {
            let xml = std::fs::read_to_string(path).map_err(|e| ProviderError(e.to_string()))?;
            self.parsed = parse_export(&xml).map_err(|e| ProviderError(e.to_string()))?;
        }
        Ok(())
    }

    fn catalog(&self) -> ActionCatalog {
        self.parsed.catalog.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ActionMaps version="1">
  <actionmap name="spaceship_targeting">
    <action name="v_target_lock_selected">
      <rebind input="kb1_t"/>
    </action>
    <action name="v_target_cycle">
      <rebind input="kb1_lshift+t"/>
    </action>
  </actionmap>
  <actionmap name="spaceship_general">
    <action name="v_eject">
      <rebind input="kb1_np_enter"/>
    </action>
    <action name="v_flightready">
      <rebind input="js1_button5"/>
    </action>
  </actionmap>
</ActionMaps>"#;

    #[test]
    fn parses_single_key_bindings() {
        let parsed = parse_export(SAMPLE).unwrap();
        // KEY_T == 20
        let seq = parsed
            .bindings
            .get(&ActionId::new("v_target_lock_selected"))
            .unwrap();
        assert_eq!(seq.steps()[0].code, ScanCode::new(20));
        // np_enter == 96
        let eject = parsed.bindings.get(&ActionId::new("v_eject")).unwrap();
        assert_eq!(eject.steps()[0].code, ScanCode::new(96));
    }

    #[test]
    fn catalog_includes_all_actions_even_unrepresentable() {
        let parsed = parse_export(SAMPLE).unwrap();
        // The chord and the joystick binding are in the catalog...
        assert!(parsed.catalog.contains(&ActionId::new("v_target_cycle")));
        assert!(parsed.catalog.contains(&ActionId::new("v_flightready")));
        // ...but they have no emittable binding.
        assert!(!parsed
            .bindings
            .contains_key(&ActionId::new("v_target_cycle")));
        assert!(!parsed
            .bindings
            .contains_key(&ActionId::new("v_flightready")));
    }

    #[test]
    fn provider_resolve_and_not_bound() {
        let provider = ScKeybindProvider::from_export_str(SAMPLE).unwrap();
        assert!(provider
            .resolve(&GameAction(ActionId::new("v_eject")))
            .is_ok());
        let err = provider
            .resolve(&GameAction(ActionId::new("v_target_cycle")))
            .unwrap_err();
        assert!(matches!(err, ResolveError::NotBound(_)));
    }
}
