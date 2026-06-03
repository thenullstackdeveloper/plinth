//! Adaptador de keybinds de Star Citizen. **Único módulo que nombra SC.**
//!
//! Parte del **XML exportable** (`pp_rebindkeys export all <nombre>` →
//! `layout_<nombre>_exported.xml`, texto plano), y opcionalmente del
//! `actionmaps.xml` del perfil del usuario. Esto desactiva el mayor riesgo del
//! adaptador: parsear el `Data.p4k` binario (defaults) queda **fuera de
//! alcance** del MVP.
//!
//! **Limitación conocida (MVP):** solo se mapean bindings de **teclado de una
//! sola tecla** (`kb1_<tecla>`). Los acordes con modificadores (`kb1_lshift+t`)
//! y las entradas de joystick/ratón (`js1_*`, `mo1_*`) se registran en el
//! catálogo (la acción es "conocida") pero no producen binding emitible → la
//! resolución devuelve `NotBound`. El modelo de `KeySequence` del dominio no
//! expresa pulsaciones simultáneas; soportar acordes es una mejora posterior.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use plinth_application::{GameKeybindProvider, ProviderError, ResolveError};
use plinth_domain::{ActionCatalog, ActionId, GameAction, KeyBinding, KeySequence, ScanCode};
use serde::Deserialize;

/// Error de parseo del XML de SC.
#[derive(Debug, thiserror::Error)]
pub enum ScParseError {
    #[error("error leyendo el XML de keybinds: {0}")]
    Io(String),
    #[error("XML de keybinds malformado: {0}")]
    Xml(String),
}

// --- Estructura del XML exportado de SC ---------------------------------

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

/// Resultado del parseo: el catálogo de acciones conocidas + el mapa de las
/// que tienen un binding de teclado emitible.
pub struct ParsedKeybinds {
    pub catalog: ActionCatalog,
    pub bindings: BTreeMap<ActionId, KeyBinding>,
}

/// Parsea el contenido de un XML exportado de SC.
pub fn parse_export(xml: &str) -> Result<ParsedKeybinds, ScParseError> {
    let doc: ActionMapsDoc =
        quick_xml::de::from_str(xml).map_err(|e| ScParseError::Xml(e.to_string()))?;

    let mut catalog_ids: BTreeSet<ActionId> = BTreeSet::new();
    let mut bindings: BTreeMap<ActionId, KeyBinding> = BTreeMap::new();

    for map in &doc.actionmaps {
        for action in &map.actions {
            let id = ActionId::new(&action.name);
            catalog_ids.insert(id.clone());

            // Toma el primer rebind que sepamos representar.
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

/// Convierte el atributo `input` de SC a una `KeySequence` emitible, o `None`
/// si no es una tecla de teclado única que sepamos mapear.
fn input_to_sequence(input: &str) -> Option<KeySequence> {
    let token = input.strip_prefix("kb1_")?; // solo teclado
    if token.is_empty() || token.contains('+') {
        return None; // acordes con modificadores: no soportados en MVP
    }
    let code = token_to_keycode(token)?;
    Some(KeySequence::single_press(ScanCode::new(code)))
}

/// Tabla token-de-SC → código de tecla de evdev (constantes de
/// `linux/input-event-codes.h`, estables a nivel de kernel).
fn token_to_keycode(token: &str) -> Option<u16> {
    let code = match token {
        // Letras
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
        // Dígitos de la fila superior
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
        // Teclas de control / espaciado
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
        // Función
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
        // Bloque de navegación
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
        // Teclado numérico
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

// --- El proveedor --------------------------------------------------------

/// `GameKeybindProvider` para Star Citizen. Mantiene el origen para poder
/// `refresh()` releyendo el fichero exportado tras un cambio de keybinds.
pub struct ScKeybindProvider {
    parsed: ParsedKeybinds,
    source_path: Option<PathBuf>,
}

impl ScKeybindProvider {
    /// Construye el proveedor desde el contenido de un export (sin fichero;
    /// `refresh()` será no-op).
    pub fn from_export_str(xml: &str) -> Result<Self, ScParseError> {
        Ok(Self {
            parsed: parse_export(xml)?,
            source_path: None,
        })
    }

    /// Construye desde un fichero exportado; `refresh()` lo relee.
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
        // El acorde y el binding de joystick están en el catálogo...
        assert!(parsed.catalog.contains(&ActionId::new("v_target_cycle")));
        assert!(parsed.catalog.contains(&ActionId::new("v_flightready")));
        // ...pero no tienen binding emitible.
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
