#[cfg(not(feature = "no_std"))]
use std::fs::File;
#[cfg(not(feature = "no_std"))]
use std::io::Read;
#[cfg(not(feature = "no_std"))]
use std::path::Path;

#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::string::String;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

use crate::ability::abilities_gen::{CARD_ABILITY_PAIRS, STRINGS};
use crate::ability::ability_store::AbilityRef;
use crate::card::Card;
use crate::HashMap;

pub struct CardLoader;

impl CardLoader {
    #[cfg(not(feature = "no_std"))]
    pub fn load_cards_from_file(path: &Path) -> Result<Vec<Card>, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        Self::load_cards_from_strs(&contents)
    }

    /// Load cards from serde_json str — internally calls attach_abilities().
    /// This is the single entry point that all ports should use.
    pub fn load_cards_from_strs(cards_json: &str) -> Result<Vec<Card>, String> {
        let mut cards: Vec<Card> = match serde_json::from_str::<Vec<Card>>(cards_json) {
            Ok(cards) => cards,
            Err(e1) => {
                let card_map: HashMap<String, Card> = serde_json::from_str(cards_json)
                    .map_err(|e| format!("Vec: {}; Object: {}", e1, e))?;
                card_map.into_values().collect()
            }
        };
        Self::attach_abilities(&mut cards);
        Ok(cards)
    }

    /// Load cards from MessagePack bytes — internally calls attach_abilities().
    /// Used by the 3DS port (cards.bin is MessagePack format).
    /// Requires the `rmp-serde` feature on the engine crate.
    #[cfg(feature = "rmp-serde")]
    pub fn load_cards_from_msgpack(bytes: &[u8]) -> Result<Vec<Card>, String> {
        let map: crate::HashMap<String, Card> = rmp_serde::from_read(bytes)
            .map_err(|e| format!("MessagePack deserialization failed: {}", e))?;
        let mut cards: Vec<Card> = map.into_values().collect();
        Self::attach_abilities(&mut cards);
        Ok(cards)
    }

    /// Attach ability references to all cards using the embedded CARD_ABILITY_PAIRS.
    /// No abilities are decoded here — just u16 indices stored on each card.
    /// Abilities are decoded lazily on first access via AbilityRef::deref().
    pub fn attach_abilities(cards: &mut [Card]) {
        let ability_map = Self::build_abilities_map_shared();
        for card in cards.iter_mut() {
            if let Some(card_abilities) = ability_map.get(card.card_no.as_ref()) {
                card.abilities = card_abilities.to_vec();
            }
        }
    }

    /// Build a map of card_no → Vec<AbilityRef> from the embedded CARD_ABILITY_PAIRS
    /// constant. No abilities.json parsing needed — the mapping is compiled into the
    /// binary as a flat array of (string_index, ability_index) pairs.
    ///
    /// This eliminates ~500KB temporary peak from serde_json::Value DOM creation.
    pub fn build_abilities_map_shared() -> HashMap<String, Vec<AbilityRef>> {
        let mut map: HashMap<String, Vec<AbilityRef>> = HashMap::default();
        let mut i = 0;
        while i + 1 < CARD_ABILITY_PAIRS.len() {
            let str_idx = CARD_ABILITY_PAIRS[i] as usize;
            let ability_idx = CARD_ABILITY_PAIRS[i + 1];
            if str_idx < STRINGS.len() {
                let card_no = STRINGS[str_idx];
                map.entry(card_no.to_string())
                    .or_default()
                    .push(AbilityRef::index(ability_idx));
            }
            i += 2;
        }
        map
    }

    /// Build the ability map for tests that need it directly.
    pub fn build_abilities_map_shared_for_tests() -> HashMap<String, Vec<AbilityRef>> {
        Self::build_abilities_map_shared()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_map_is_non_empty() {
        let map = CardLoader::build_abilities_map_shared_for_tests();
        assert!(
            !map.is_empty(),
            "ability map should contain at least one card"
        );
    }

    #[test]
    fn ability_refs_all_have_valid_indices() {
        let map = CardLoader::build_abilities_map_shared_for_tests();
        for (card_no, refs) in map.iter() {
            for r in refs {
                let idx = r.idx();
                assert!(
                    idx < crate::ability::abilities_gen::NUM_ABILITIES as u16,
                    "card {} has ability index {} >= {}",
                    card_no,
                    idx,
                    crate::ability::abilities_gen::NUM_ABILITIES
                );
            }
        }
    }
}
