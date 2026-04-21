use crate::card::{Ability, Card};
use std::fs::File;
use std::io::Read;
use std::string::String;
use std::vec::Vec;
use std::path::Path;
use std::collections::HashMap;
use serde_json;

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
        let mut total_abilities_mapped = 0;

        if let Some(unique_abilities) = abilities_data.get("unique_abilities").and_then(|v| v.as_array()) {
            println!("Loading {} unique abilities from abilities.json", unique_abilities.len());
            for ability_entry in unique_abilities {
                // Try to deserialize the ability directly - #[serde(default)] will handle missing fields
                if let Ok(mut ability) = serde_json::from_value::<Ability>(ability_entry.clone()) {
                    // If effect action field is empty, try to infer it from the effect structure
                    if let Some(ref mut effect) = ability.effect {
                        if effect.action.is_empty() {
                            if let Some(effect_json) = ability_entry.get("effect") {
                                let text = effect_json.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                
                                // Check if it has source/destination which indicates move_cards
                                if effect_json.get("source").is_some() && effect_json.get("destination").is_some() {
                                    effect.action = "move_cards".to_string();
                                }
                                // Otherwise check if it has an actions array
                                else if let Some(actions) = effect_json.get("actions").and_then(|a| a.as_array()) {
                                    if !actions.is_empty() {
                                        effect.action = "sequential".to_string();
                                    }
                                }
                                // If still empty, try to infer from text
                                else if !text.is_empty() {
                                    // Check for common action keywords
                                    if text.contains("手札に加える") || text.contains("手札に加え") {
                                        effect.action = "move_cards".to_string();
                                        if effect.source.is_none() {
                                            // Try to infer source from text
                                            if text.contains("控え室から") {
                                                effect.source = Some("discard".to_string());
                                            } else if text.contains("デッキから") {
                                                effect.source = Some("deck".to_string());
                                            }
                                        }
                                        if effect.destination.is_none() {
                                            effect.destination = Some("hand".to_string());
                                        }
                                    } else if text.contains("ドロー") {
                                        effect.action = "draw".to_string();
                                    } else if text.contains("ライブ") {
                                        effect.action = "live".to_string();
                                    }
                                }
                            }
                        }
                    }
                    if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                        for card_entry in card_list {
                            if let Some(card_str) = card_entry.as_str() {
                                // Parse card identifier like "PL!-sd1-005-SD | 星空 凛 (ab#0)"
                                // Extract just the card number part before the space
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map.entry(card_no.to_string()).or_insert_with(Vec::new).push(ability.clone());
                                    total_abilities_mapped += 1;
                                }
                            }
                        }
                    }
                } else {
                    // Log deserialization error for debugging
                    eprintln!("Failed to deserialize ability entry: {}", serde_json::to_string_pretty(ability_entry).unwrap_or_default());
                    if let Err(e) = serde_json::from_value::<Ability>(ability_entry.clone()) {
                        eprintln!("Deserialization error: {}", e);
                    }
                }
            }
            println!("Mapped {} total abilities to cards", total_abilities_mapped);
        }

        // Attach abilities to cards
        let mut cards_with_abilities = 0;
        for card in &mut cards {
            if let Some(card_abilities) = ability_map.get(&card.card_no) {
                card.abilities = card_abilities.clone();
                cards_with_abilities += 1;
            }
        }
        println!("Attached abilities to {} cards", cards_with_abilities);

        cards
    }
}
