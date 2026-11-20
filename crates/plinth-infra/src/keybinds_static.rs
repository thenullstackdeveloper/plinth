//! `GameKeybindProvider` backed by a fixed table: each action maps to a known
//! key sequence. It lets a game be driven through `execute_action` before its
//! real keybind configuration is read (WoW in phase 1).

use std::collections::HashMap;

use plinth_application::{GameKeybindProvider, ProviderError, ResolveError};
use plinth_domain::{ActionCatalog, ActionId, GameAction, KeyBinding, KeySequence, ScanCode};

/// World of Warcraft's default key for the world map: `M` (evdev `KEY_M`).
const WOW_KEY_M: ScanCode = ScanCode(50);

pub struct StaticKeybindProvider {
    bindings: HashMap<ActionId, KeySequence>,
}

impl StaticKeybindProvider {
    pub fn new(bindings: Vec<(ActionId, KeySequence)>) -> Self {
        Self {
            bindings: bindings.into_iter().collect(),
        }
    }

    /// The WoW actions available in phase 1.
    pub fn wow_defaults() -> Self {
        Self::new(vec![(
            ActionId::new("wow.world_map"),
            KeySequence::single_press(WOW_KEY_M),
        )])
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
    fn catalog_lists_the_table() {
        let catalog = StaticKeybindProvider::wow_defaults().catalog();
        assert!(catalog.contains(&ActionId::new("wow.world_map")));
        assert!(!catalog.contains(&ActionId::new("wow.nope")));
    }
}
