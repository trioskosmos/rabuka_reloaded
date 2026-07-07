// Pre-bake abilities directly into cards.json for the 3DS.
// Reads cards.json + abilities.json, builds the ability map,
// attaches abilities to each card entry, and writes a single merged
// compact cards.json to RomFS.  The 3DS then loads this via
// CardLoader::load_cards_from_strs, which handles both array and
// object-with-keys formats.
//
// Called by build_3ds.bat before the 3DS cross-compile.

use rabuka_engine::card_loader::CardLoader;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "engine_3ds/romfs".into());
    let cwd = std::env::current_dir().unwrap();
    let repo_root = if cwd.ends_with("engine_3ds") {
        cwd.parent().unwrap().to_owned()
    } else {
        cwd
    };

    // 1. Load cards.json + abilities.json
    let cards_path = repo_root.join("cards/cards.json");
    println!("Loading {}", cards_path.display());
    let cards_json = fs::read_to_string(&cards_path)
        .unwrap_or_else(|_| panic!("{} not found", cards_path.display()));

    let abilities_path = repo_root.join("cards/abilities.json");
    println!("Loading {}", abilities_path.display());
    let abilities_content = fs::read_to_string(&abilities_path)
        .unwrap_or_else(|_| panic!("{} not found", abilities_path.display()));

    // 2. Build ability map
    println!("Building abilities map...");
    let abilities_val: serde_json::Value =
        serde_json::from_str(&abilities_content).expect("Failed to parse abilities.json");
    let ability_map = CardLoader::build_abilities_map(&abilities_val);
    println!("  {} cards have abilities", ability_map.len());

    // 3. Parse the ORIGINAL JSON as a Value tree (preserving the exact format
    //    that the custom Deserializers expect).  Then inject abilities into
    //    each card entry.  This avoids breaking the roundtrip: the custom
    //    Deserializers for BladeHeart/BaseHeart expect the original format.
    println!("Merging abilities into cards...");
    let cards_val: serde_json::Value =
        serde_json::from_str(&cards_json).expect("Failed to parse cards.json");

    // Normalise to an array of objects (the original may be an object map)
    let mut entries: Vec<serde_json::Value> = if cards_val.is_array() {
        cards_val.as_array().unwrap().clone()
    } else if cards_val.is_object() {
        cards_val.as_object().unwrap().values().cloned().collect()
    } else {
        panic!("cards.json is neither array nor object")
    };

    let mut count_attached = 0usize;
    for entry in &mut entries {
        let card_no = match entry.get("card_no").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if let Some(abilities) = ability_map.get(&card_no) {
            let ab = serde_json::to_value(abilities).unwrap_or(serde_json::Value::Array(vec![]));
            entry
                .as_object_mut()
                .unwrap()
                .insert("abilities".into(), ab);
            count_attached += 1;
        }
    }
    println!("  Attached abilities to {} cards", count_attached);

    // 4. Write compact cards.json (no pretty-print — the 27 MB pretty-printed
    //    version was what caused the "freezes at parsing" on the 3DS ARM11).
    let out_path = repo_root.join(&out_dir).join("cards.json");
    let json_bytes =
        serde_json::to_vec(&serde_json::Value::Array(entries.clone())).expect("JSON serialization failed");
    fs::write(&out_path, &json_bytes).expect("Failed to write cards.json");
    println!("Wrote {} ({} bytes)", out_path.display(), json_bytes.len());

    // 5. Write cards.bin (MessagePack format). This is even smaller and avoids
    //    the massive CPU cost of parsing 17MB of JSON on the 3DS ARM11.
    let bin_out_path = repo_root.join(&out_dir).join("cards.bin");
    let bin_bytes = rmp_serde::to_vec(&serde_json::Value::Array(entries))
        .expect("MessagePack serialization failed");
    fs::write(&bin_out_path, &bin_bytes).expect("Failed to write cards.bin");
    println!("Wrote {} ({} bytes)", bin_out_path.display(), bin_bytes.len());
}
