/// Desktop pre-bake tool for 3DS ability loading.
///
/// Reads `cards/abilities.json`, runs the full ability-building pipeline
/// on desktop, then writes a compact deduplicated `abilities_map.json`.
///
/// Format: { "abilities": [<Ability>, ...], "cards": {"card_no": [idx, ...]} }
///
/// Each ability is stored ONCE and cards reference it by index — this avoids
/// the 14MB expansion that happens when duplicating ability data per card.
/// Result is typically ~1-2MB instead of 14MB, fast enough for 3DS ARM11.
use rabuka_engine::card::Ability;
use rabuka_engine::core::card_loader::CardLoader;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

#[derive(Serialize)]
struct AbilitiesMapFile {
    abilities: Vec<Ability>,
    cards: HashMap<String, Vec<usize>>,
}

fn main() {
    let abilities_path = "cards/abilities.json";
    let out_path = "engine_3ds/romfs/abilities_map.json";

    let abilities_json = fs::read_to_string(abilities_path)
        .unwrap_or_else(|e| panic!("Could not read {}: {}", abilities_path, e));

    let data = CardLoader::load_abilities_from_str(&abilities_json)
        .unwrap_or_else(|e| panic!("Could not parse abilities.json: {}", e));

    // Build deduplicated index: each unique_ability entry is stored once.
    // Cards reference abilities by index into the abilities array.
    let mut abilities: Vec<Ability> = Vec::new();
    let mut cards: HashMap<String, Vec<usize>> = HashMap::new();

    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
        for ability_entry in unique_abilities {
            // Apply the same trigger_condition merging as the engine does
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

            let mut ability: Ability = match serde_json::from_value(entry) {
                Ok(a) => a,
                Err(_) => continue,
            };

            // Fix draw actions missing count
            if let Some(ref mut effect) = ability.effect {
                if let Some(ref actions) = effect.compound.actions.clone() {
                    let fixed: Vec<_> = actions
                        .iter()
                        .map(|a| {
                            let mut fa = a.clone();
                            if (fa.action == "draw" || fa.action == "draw_card")
                                && fa.count.is_none()
                                && fa.dynamic_count.is_none()
                            {
                                fa.count = Some(1);
                            }
                            fa
                        })
                        .collect();
                    effect.compound.actions = Some(fixed);
                }
            }

            // Strip display-only text — not needed for gameplay
            ability.full_text = String::new();
            ability.triggerless_text = String::new();

            // Store this ability once and record its index
            let idx = abilities.len();
            abilities.push(ability);

            // Map each card to this ability's index
            if let Some(card_list) = ability_entry.get("cards").and_then(|v| v.as_array()) {
                for card_entry in card_list {
                    if let Some(card_str) = card_entry.as_str() {
                        if let Some(card_no) = card_str.split(" | ").next() {
                            cards.entry(card_no.to_string()).or_default().push(idx);
                        }
                    }
                }
            }
        }
    }

    let file = AbilitiesMapFile { abilities, cards };

    println!(
        "Built map: {} unique abilities, {} cards with abilities",
        file.abilities.len(),
        file.cards.len()
    );

    let out_bin = "engine_3ds/romfs/abilities_map.bin";
    let bin_data = rmp_serde::to_vec(&file)
        .unwrap_or_else(|e| panic!("Could not serialize map with MessagePack: {}", e));

    fs::write(out_bin, &bin_data)
        .unwrap_or_else(|e| panic!("Could not write {}: {}", out_bin, e));

    println!("Written {} bytes to {}", bin_data.len(), out_bin);
}
