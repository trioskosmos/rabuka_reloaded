use crate::card::{Ability, Card};
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct CardLoader;

impl CardLoader {
    pub fn load_cards_from_file(path: &Path) -> Result<Vec<Card>, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Try parsing as array first
        let mut cards: Vec<Card> = match serde_json::from_str::<Vec<Card>>(&contents) {
            Ok(cards) => cards,
            Err(_) => {
                // If that fails, try parsing as object (map) and convert to array
                let card_map: HashMap<String, Card> = serde_json::from_str(&contents)
                    .map_err(|e| format!("Failed to parse JSON as object: {}", e))?;
                card_map.into_values().collect()
            }
        };
        
        // Load abilities from abilities.json
        let abilities_path = path.parent().unwrap().join("abilities.json");
        if let Ok(abilities_data) = Self::load_abilities_from_file(&abilities_path) {
            cards = Self::attach_abilities(cards, &abilities_data);
        }
        
        Ok(cards)
    }

    fn load_abilities_from_file(path: &Path) -> Result<serde_json::Value, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open abilities file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| format!("Failed to read abilities file: {}", e))?;
        
        let data: serde_json::Value = serde_json::from_str(&contents).map_err(|e| format!("Failed to parse abilities JSON: {}", e))?;
        
        Ok(data)
    }

    fn attach_abilities(mut cards: Vec<Card>, abilities_data: &serde_json::Value) -> Vec<Card> {
        // Map card numbers to their abilities
        let mut ability_map: HashMap<String, Vec<Ability>> = HashMap::new();

        if let Some(unique_abilities) = abilities_data.get("unique_abilities").and_then(|v| v.as_array()) {
            for ability_entry in unique_abilities {
                // The ability entry itself contains the ability data directly
                if let Ok(ability) = serde_json::from_value::<Ability>(ability_entry.clone()) {
                    if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                        for card_entry in card_list {
                            if let Some(card_str) = card_entry.as_str() {
                                // Parse card identifier like "PL!-sd1-005-SD | 星空 凛 (ab#0)"
                                // Extract just the card number part before the space
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map.entry(card_no.to_string()).or_insert_with(Vec::new).push(ability.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Attach abilities to cards
        for card in &mut cards {
            if let Some(card_abilities) = ability_map.get(&card.card_no) {
                card.abilities = card_abilities.clone();
            }
        }

        cards
    }
}
