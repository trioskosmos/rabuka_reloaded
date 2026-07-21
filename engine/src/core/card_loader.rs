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

use crate::ability::ability_store::AbilityRef;
use crate::card::Card;
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::boxed::Box;
use serde_json;

pub struct CardLoader;

impl CardLoader {
    #[cfg(not(feature = "no_std"))]
    pub fn load_cards_from_file(path: &Path) -> Result<Vec<Card>, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let abilities_path = path.parent().unwrap().join("abilities.json");
        let abilities_contents = std::fs::read_to_string(&abilities_path).ok();

        Self::load_cards_from_strs(&contents, abilities_contents.as_deref())
    }

    pub fn load_cards_from_strs(
        cards_json: &str,
        abilities_json: Option<&str>,
    ) -> Result<Vec<Card>, String> {
        let mut cards: Vec<Card> = match serde_json::from_str::<Vec<Card>>(cards_json) {
            Ok(cards) => cards,
            Err(e1) => {
                let card_map: HashMap<String, Card> = serde_json::from_str(cards_json)
                    .map_err(|e| format!("Vec: {}; Object: {}", e1, e))?;
                card_map.into_values().collect()
            }
        };

        if let Some(abilities_str) = abilities_json {
            if let Ok(abilities_data) = Self::load_abilities_from_str(abilities_str) {
                cards = Self::attach_abilities(cards, &abilities_data);
            }
        }

        Ok(cards)
    }

    #[cfg(not(feature = "no_std"))]
    #[allow(dead_code)]
    fn load_abilities_from_file(path: &Path) -> Result<serde_json::Value, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open abilities file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read abilities file: {}", e))?;
        Self::load_abilities_from_str(&contents)
    }

    pub fn load_abilities_from_str(contents: &str) -> Result<serde_json::Value, String> {
        let data: serde_json::Value = serde_json::from_str(contents)
            .map_err(|e| format!("Failed to parse abilities JSON: {}", e))?;
        Ok(data)
    }

    pub fn attach_abilities(mut cards: Vec<Card>, abilities_data: &serde_json::Value) -> Vec<Card> {
        let ability_map = Self::build_abilities_map_shared(abilities_data);
        for card in &mut cards {
            if let Some(card_abilities) = ability_map.get(card.card_no.as_ref()) {
                card.abilities = card_abilities.to_vec();
            }
        }
        cards
    }

    /// Build a map of card_no → Vec<AbilityRef> by storing bytecode indices.
    ///
    /// Abilities are NOT decoded here. Each `AbilityRef` stores a `u16`
    /// bytecode index. The ability is decoded on first access via
    /// `AbilityRef::resolve()`, which caches the result in a global
    /// `HashMap<u16, Arc<Ability>>`.
    ///
    /// This eliminates ~2.8MB of decoded structs from RAM at load time.
    /// Only abilities actually triggered in a game are decoded (~30-45
    /// out of 800), saving ~2.68MB.
    pub fn build_abilities_map_shared(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<AbilityRef>> {
        let mut ability_map: HashMap<String, Vec<AbilityRef>> = HashMap::default();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for (idx, ability_entry) in unique_abilities.iter().enumerate() {
                if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                    for card_entry in card_list {
                        if let Some(card_str) = card_entry.as_str() {
                            if let Some(card_no) = card_str.split(" | ").next() {
                                ability_map
                                    .entry(card_no.to_string())
                                    .or_default()
                                    .push(AbilityRef::index(idx as u16));
                            }
                        }
                    }
                }
            }
        }
        ability_map
    }
}
