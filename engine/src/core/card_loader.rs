use crate::card::{Ability, AbilityEffect, Card};
use crate::Arc;
use crate::HashMap;
use serde_json;
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
                card.abilities = card_abilities.clone();
            }
        }
        cards
    }

    fn build_abilities_map_inner(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Ability>> {
        let mut ability_map: HashMap<String, Vec<Ability>> = HashMap::new();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for ability_entry in unique_abilities {
                let entry = ability_entry.clone();
                let effect_entry = entry.get("effect").cloned();

                if let Ok(mut ability) = serde_json::from_value::<Ability>(entry) {
                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref json_effect) = effect_entry {
                            effect.populate_from_json(json_effect);
                        }
                    }

                    if let Some(ref mut effect) = ability.effect {
                        if let Some(ref actions) = effect.compound.actions.clone() {
                            let fixed_actions: Vec<Box<AbilityEffect>> = actions
                                .iter()
                                .map(|action| {
                                    let mut fixed_action = action.clone();
                                    if (fixed_action.action == "draw"
                                        || fixed_action.action == "draw_card")
                                        && fixed_action.count.is_none()
                                        && fixed_action.dynamic_count_any().is_none()
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

    pub fn build_abilities_map_shared(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Arc<Ability>>> {
        let inner = Self::build_abilities_map_inner(abilities_data);
        inner
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|a| Arc::new(a)).collect()))
            .collect()
    }

    pub fn build_abilities_map(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Ability>> {
        Self::build_abilities_map_inner(abilities_data)
    }

    pub fn apply_abilities_index(
        mut cards: Vec<Card>,
        abilities_index: &[Ability],
        card_index: &HashMap<String, Vec<usize>>,
    ) -> Vec<Card> {
        let shared_pool: Vec<Arc<Ability>> = abilities_index
            .iter()
            .map(|a| Arc::new(a.clone()))
            .collect();
        for card in &mut cards {
            if let Some(indices) = card_index.get(card.card_no.as_ref()) {
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
