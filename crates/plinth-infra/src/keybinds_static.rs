//! `GameKeybindProvider` backed by a fixed table: each action maps to a known
//! key sequence. It lets a game be driven through `execute_action` before its
//! real keybind configuration is read (WoW in phase 1).

use std::collections::HashMap;

use plinth_application::{GameKeybindProvider, ProviderError, ResolveError};
use plinth_domain::{ActionCatalog, ActionId, GameAction, KeyBinding, KeySequence, ScanCode};

// World of Warcraft's default keys (evdev codes).
/// World map: `M`.
const WOW_KEY_M: ScanCode = ScanCode(50);
/// Character panel: `C`.
const WOW_KEY_C: ScanCode = ScanCode(46);
/// Bags: `B`.
const WOW_KEY_B: ScanCode = ScanCode(48);
/// Spellbook: `P`.
const WOW_KEY_P: ScanCode = ScanCode(25);
/// Quest log: `L`.
const WOW_KEY_L: ScanCode = ScanCode(38);
/// Talents: `N`.
const WOW_KEY_N: ScanCode = ScanCode(49);

pub struct StaticKeybindProvider {
    bindings: HashMap<ActionId, KeySequence>,
}

impl StaticKeybindProvider {
    pub fn new(bindings: Vec<(ActionId, KeySequence)>) -> Self {
        Self {
            bindings: bindings.into_iter().collect(),
        }
    }

    /// The WoW actions of the demo panel, bound to their default keys.
    pub fn wow_defaults() -> Self {
        Self::new(
            [
                ("wow.world_map", WOW_KEY_M),
                ("wow.character", WOW_KEY_C),
                ("wow.bags", WOW_KEY_B),
                ("wow.spellbook", WOW_KEY_P),
                ("wow.quest_log", WOW_KEY_L),
                ("wow.talents", WOW_KEY_N),
            ]
            .into_iter()
            .map(|(id, key)| (ActionId::new(id), KeySequence::single_press(key)))
            .collect(),
        )
    }
}

impl GameKeybindProvider for StaticKeybindProvider {
    fn resolve(&self, action: &GameAction) -> Result<KeyBinding, ResolveError> {
        self.bindings
            .get(&action.0)
            .cloned()
            .ok_or_else(|| ResolveError::NotBound(action.clone()))
    }

    fn refresh(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn catalog(&self) -> ActionCatalog {
        ActionCatalog::from_actions(self.bindings.keys().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: &str) -> GameAction {
        GameAction(ActionId::new(id))
    }

    #[test]
    fn wow_defaults_resolves_world_map_to_m() {
        let provider = StaticKeybindProvider::wow_defaults();
        assert_eq!(
            provider.resolve(&action("wow.world_map")).unwrap(),
            KeySequence::single_press(ScanCode(50))
        );
    }

    #[test]
    fn unknown_action_is_not_bound() {
        let provider = StaticKeybindProvider::wow_defaults();
        assert!(matches!(
            provider.resolve(&action("wow.nope")),
            Err(ResolveError::NotBound(_))
        ));
    }

    #[test]
    fn wow_defaults_resolve_the_six_demo_keys() {
        let provider = StaticKeybindProvider::wow_defaults();
        for (id, code) in [
            ("wow.world_map", 50),
            ("wow.character", 46),
            ("wow.bags", 48),
            ("wow.spellbook", 25),
            ("wow.quest_log", 38),
            ("wow.talents", 49),
        ] {
            assert_eq!(
                provider.resolve(&action(id)).unwrap(),
                KeySequence::single_press(ScanCode(code)),
                "{id}"
            );
        }
    }

    #[test]
    fn catalog_lists_the_table() {
        let catalog = StaticKeybindProvider::wow_defaults().catalog();
        assert!(catalog.contains(&ActionId::new("wow.world_map")));
        assert!(!catalog.contains(&ActionId::new("wow.nope")));
    }
}
