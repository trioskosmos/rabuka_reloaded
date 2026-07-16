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
    let _out_path = "engine_3ds/romfs/abilities_map.json";

    let abilities_json = fs::read_to_string(abilities_path)
        .unwrap_or_else(|e| panic!("Could not read {}: {}", abilities_path, e));

    let data = CardLoader::load_abilities_from_str(&abilities_json)
        .unwrap_or_else(|e| panic!("Could not parse abilities.json: {}", e));

    // Use the shared pipeline to build a card_no → abilities map.
    // This handles trigger_condition merging, EffectKind population,
    // and draw-count fixing — no need to reimplement any of that here.
    let ability_map = CardLoader::build_abilities_map(&data);

    // Build deduplicated index: each ability is stored once in `abilities`,
    // and `cards` maps card_no to indices into that vec.
    let mut abilities: Vec<Ability> = Vec::new();
    let mut cards: HashMap<String, Vec<usize>> = HashMap::default();

    for (card_no, card_abilities) in &ability_map {
        for ability in card_abilities {
            let idx = abilities.iter().position(|a| a == ability);
            let idx = match idx {
                Some(i) => i,
                None => {
                    let i = abilities.len();
                    abilities.push(ability.clone());
                    i
                }
            };
            cards.entry(card_no.clone()).or_default().push(idx);
        }
    }

    // Strip display-only text — not needed for gameplay
    for ability in &mut abilities {
        ability.full_text = String::new();
        ability.triggerless_text = String::new();
    }

    let file = AbilitiesMapFile { abilities, cards };

    println!(
        "Built map: {} unique abilities, {} cards with abilities",
        file.abilities.len(),
        file.cards.len()
    );

    let out_bin = "engine_3ds/romfs/abilities_map.bin";
    let bin_data = rmp_serde::to_vec_named(&file)
        .unwrap_or_else(|e| panic!("Could not serialize map with MessagePack: {}", e));
    fs::write(out_bin, &bin_data).unwrap_or_else(|e| panic!("Could not write {}: {}", out_bin, e));
    println!("Written {} bytes to {}", bin_data.len(), out_bin);

    let out_json = "engine_3ds/romfs/abilities_index.json";
    let json_data = serde_json::to_vec(&file)
        .unwrap_or_else(|e| panic!("Could not serialize map with JSON: {}", e));
    fs::write(out_json, &json_data)
        .unwrap_or_else(|e| panic!("Could not write {}: {}", out_json, e));
    println!("Written {} bytes to {}", json_data.len(), out_json);
}
