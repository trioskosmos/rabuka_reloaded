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
use crate::card::{Ability, Card};
use crate::Arc;
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
                card.abilities = card_abilities.clone();
            }
        }
        cards
    }

    /// Build a map of card_no → Vec<AbilityRef> by decoding abilities from bytecode.
    pub fn build_abilities_map_shared(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<AbilityRef>> {
        let inner = Self::build_abilities_map_inner(abilities_data);
        inner
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|a| AbilityRef(Arc::new(a))).collect()))
            .collect()
    }

    /// Decode ALL abilities from bytecode via `vm::get_ability(idx)`.
    ///
    /// # EAGER DECODE — NOT LAZY
    /// This function iterates all 800 unique abilities and decodes each one
    /// into a full `Ability` struct at load time. Each ability is ~3.5KB.
    /// Total: ~2.8MB of decoded Ability structs in RAM.
    ///
    /// This is called once from `attach_abilities` during CardLoader::load(),
    /// which runs inside a `OnceLock` — so it executes exactly once per process.
    ///
    /// # TODO: Lazy decode for 150KB target
    /// Instead of decoding all 800 abilities here, store only the bytecode
    /// index (u16) on each card. Decode on-demand when ability is first
    /// triggered. Architecture:
    ///
    /// ```text
    /// fn build_abilities_map_lazy(abilities_data) -> HashMap<String, Vec<AbilityRef>> {
    ///     // AbilityRef = AbilityRef(u16) — 2 bytes, no decode
    ///     for (idx, entry) in unique_abilities.iter().enumerate() {
    ///         for card_no in entry["cards"] {
    ///             map[card_no].push(AbilityRef::index(idx as u16));
    ///         }
    ///     }
    ///     map
    /// }
    /// ```
    ///
    /// Then in triggers.rs / abilities.rs, when an ability is first accessed:
    /// ```text
    /// let ability = resolver.resolve(*ability_ref);  // decode + cache
    /// ```
    ///
    /// This eliminates ~2.8MB from RAM. Only ~30-45 abilities are actually
    /// triggered in a typical game, so resident decode is ~120KB instead of
    /// ~2.8MB.
    ///
    /// See MEMORY_REFACTOR.md P1.7 for the full plan and 52 call sites.
    fn build_abilities_map_inner(
        abilities_data: &serde_json::Value,
    ) -> HashMap<String, Vec<Ability>> {
        let mut ability_map: HashMap<String, Vec<Ability>> = HashMap::default();

        if let Some(unique_abilities) = abilities_data
            .get("unique_abilities")
            .and_then(|v| v.as_array())
        {
            for (_idx, ability_entry) in unique_abilities.iter().enumerate() {
                let ability = crate::ability::vm::get_ability(_idx);

                if let Some(ability) = ability {
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
                }
            }
        }
        ability_map
    }
}
