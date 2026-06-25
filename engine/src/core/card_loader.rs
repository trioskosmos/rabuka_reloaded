use crate::card::{Ability, Card};
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::string::String;
use std::vec::Vec;

pub struct CardLoader;

impl CardLoader {
    pub fn load_cards_from_file(path: &Path) -> Result<Vec<Card>, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let abilities_path = path.parent().unwrap().join("abilities.json");
        let abilities_contents = std::fs::read_to_string(&abilities_path).ok();

        Self::load_cards_from_strs(&contents, abilities_contents.as_deref())
    }

    /// Parse cards from embedded string content (no file I/O at runtime).
    pub fn load_cards_from_strs(
        cards_json: &str,
        abilities_json: Option<&str>,
    ) -> Result<Vec<Card>, String> {
        // Try parsing as array first
        let mut cards: Vec<Card> = match serde_json::from_str::<Vec<Card>>(cards_json) {
            Ok(cards) => cards,
            Err(_) => {
                // If that fails, try parsing as object (map) and convert to array
                let card_map: HashMap<String, Card> = serde_json::from_str(cards_json)
                    .map_err(|e| format!("Failed to parse JSON as object: {}", e))?;
                card_map.into_values().collect()
            }
        };

        // Load abilities if provided
        if let Some(abilities_str) = abilities_json {
            if let Ok(abilities_data) = Self::load_abilities_from_str(abilities_str) {
                cards = Self::attach_abilities(cards, &abilities_data);
            }
        }

        Ok(cards)
    }

    fn load_abilities_from_file(path: &Path) -> Result<serde_json::Value, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open abilities file: {}", e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read abilities file: {}", e))?;
        Self::load_abilities_from_str(&contents)
    }

    fn load_abilities_from_str(contents: &str) -> Result<serde_json::Value, String> {
        let data: serde_json::Value = serde_json::from_str(contents)
            .map_err(|e| format!("Failed to parse abilities JSON: {}", e))?;
        Ok(data)
    }

    fn attach_abilities(mut cards: Vec<Card>, abilities_data: &serde_json::Value) -> Vec<Card> {
        // Map card numbers to their abilities
        let mut ability_map: HashMap<String, Vec<Ability>> = HashMap::new();
        let mut _total_abilities_mapped = 0;

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            // println!("Loading {} unique abilities from abilities.json", unique_abilities.len());
            for ability_entry in unique_abilities {
                // Merge trigger_condition into condition BEFORE deserialization,
                // so the Rust struct only ever sees `condition`.  This avoids
                // serde alias-vs-flatten conflicts and unifies the two fields.
                let mut entry = ability_entry.clone();
                if let Some(obj) = entry.as_object_mut() {
                    if let Some(tc_val) = obj.remove("trigger_condition") {
                        match obj.get_mut("condition") {
                            Some(cond_val) => {
                                // Both exist → wrap in compound AND
                                let mut merged = serde_json::Map::new();
                                merged.insert("type".into(), "compound".into());
                                merged.insert("operator".into(), "and".into());
                                merged.insert(
                                    "conditions".into(),
                                    serde_json::Value::Array(vec![cond_val.take(), tc_val]),
                                );
                                obj.insert("condition".into(), serde_json::Value::Object(merged));
                            }
                            None => {
                                // Only trigger_condition exists → use as condition
                                obj.insert("condition".into(), tc_val);
                            }
                        }
                    }
                }
                // Try to deserialize the ability - #[serde(default)] handles missing fields
                if let Ok(mut ability) = serde_json::from_value::<Ability>(entry) {
                    // Fix nested actions - rebuild the actions array with count set
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref actions) = effect.compound.actions.clone() {
                            let fixed_actions: Vec<crate::card::AbilityEffect> = actions
                                .iter()
                                .map(|action| {
                                    let mut fixed_action = action.clone();
                                    if (fixed_action.action == "draw"
                                        || fixed_action.action == "draw_card")
                                        && fixed_action.count.is_none()
                                        && fixed_action.dynamic_count.is_none()
                                    {
                                        fixed_action.count = Some(1);
                                    }
                                    fixed_action
                                })
                                .collect();
                            effect.compound.actions = Some(fixed_actions);
                        }
                    }

                    if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                        for card_entry in card_list {
                            if let Some(card_str) = card_entry.as_str() {
                                // Parse card identifier like "PL!-sd1-005-SD | 星空 凛 (ab#0)"
                                // Extract just the card number part before the space
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map
                                        .entry(card_no.to_string())
                                        .or_default()
                                        .push(ability.clone());
                                    _total_abilities_mapped += 1;
                                }
                            }
                        }
                    }
                } else {
                    // Log deserialization error for debugging
                    log::debug!(
                        "Failed to deserialize ability entry: {}",
                        serde_json::to_string_pretty(ability_entry).unwrap_or_default()
                    );
                    if let Err(e) = serde_json::from_value::<Ability>(ability_entry.clone()) {
                        log::debug!("Deserialization error: {}", e);
                    }
                }
            }
            // println!("Mapped {} total abilities to cards", total_abilities_mapped);
        }

        // Attach abilities to cards
        let _cards_with_abilities = 0;
        for card in &mut cards {
            if let Some(card_abilities) = ability_map.get(&card.card_no) {
                card.abilities = card_abilities.clone();
            }
        }
        // println!("Attached abilities to {} cards", cards_with_abilities);

        cards
    }
}
