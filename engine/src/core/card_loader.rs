use crate::card::{Ability, Card};
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::string::String;
use std::sync::Arc;
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
            Err(e1) => {
                // If that fails, try parsing as object (map) and convert to array
                let card_map: HashMap<String, Card> = serde_json::from_str(cards_json)
                    .map_err(|e| format!("Vec: {}; Object: {}", e1, e))?;
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
            if let Some(card_abilities) = ability_map.get(&card.card_no) {
                card.abilities = card_abilities.clone();
            }
        }
        cards
    }

    /// Build a card_no -> Vec<Arc<Ability>> map from the parsed abilities JSON Value.
    pub fn build_abilities_map_shared(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Arc<Ability>>> {
        let mut ability_map: HashMap<String, Vec<Arc<Ability>>> = HashMap::new();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for ability_entry in unique_abilities {
                let mut entry = ability_entry.clone();
                if let Some(obj) = entry.as_object_mut() {
                    if let Some(tc_val) = obj.remove("trigger_condition") {
                        match obj.get_mut("condition") {
                            Some(cond_val) => {
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
                                obj.insert("condition".into(), tc_val);
                            }
                        }
                    }
                }
                if let Ok(mut ability) = serde_json::from_value::<Ability>(entry) {
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

                    let shared = Arc::new(ability);

                    if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                        for card_entry in card_list {
                            if let Some(card_str) = card_entry.as_str() {
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map
                                        .entry(card_no.to_string())
                                        .or_default()
                                        .push(Arc::clone(&shared));
                                }
                            }
                        }
                    }
                }
            }
        }
        ability_map
    }

    /// Build a card_no → Vec<Ability> map from the parsed abilities JSON Value.
    /// Exposed so the desktop gen_abilities_map tool can pre-bake it for 3DS.
    pub fn build_abilities_map(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Ability>> {
        let mut ability_map: HashMap<String, Vec<Ability>> = HashMap::new();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for ability_entry in unique_abilities {
                let mut entry = ability_entry.clone();
                if let Some(obj) = entry.as_object_mut() {
                    if let Some(tc_val) = obj.remove("trigger_condition") {
                        match obj.get_mut("condition") {
                            Some(cond_val) => {
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
                                obj.insert("condition".into(), tc_val);
                            }
                        }
                    }
                }
                if let Ok(mut ability) = serde_json::from_value::<Ability>(entry) {
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
                                if let Some(card_no) = card_str.split(" | ").next() {
                                    ability_map
                                        .entry(card_no.to_string())
                                        .or_default()
                                        .push(ability.clone());
                                }
                            }
                        }
                    }
                } else {
                    log::debug!(
                        "Failed to deserialize ability entry: {}",
                        serde_json::to_string_pretty(ability_entry).unwrap_or_default()
                    );
                }
            }
        }
        ability_map
    }

    /// Apply a pre-baked deduplicated abilities index to a card list.
    /// abilities_index: flat list of unique Ability objects.
    /// card_index: card_no → indices into abilities_index.
    /// Used on 3DS with the gen_abilities_map pre-baked compact format.
    pub fn apply_abilities_index(
        mut cards: Vec<Card>,
        abilities_index: &[Ability],
        card_index: &HashMap<String, Vec<usize>>,
    ) -> Vec<Card> {
        // Build shared Arc<Ability> pool from the index to avoid per-card clones
        let shared_pool: Vec<Arc<Ability>> = abilities_index
            .iter()
            .map(|a| Arc::new(a.clone()))
            .collect();
        for card in &mut cards {
            if let Some(indices) = card_index.get(&card.card_no) {
                card.abilities = indices
                    .iter()
                    .filter_map(|&i| shared_pool.get(i))
                    .map(|a| Arc::clone(a))
                    .collect();
            }
        }
        cards
    }
}
