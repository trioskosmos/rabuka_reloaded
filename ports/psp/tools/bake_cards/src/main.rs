use std::collections::HashSet;
use std::fs;

use rabuka_engine::card::{Ability, Card};
use rabuka_engine::card_loader::CardLoader;

/// Normalize card number: uppercase ASCII, fullwidth → halfwidth.
/// Mirrors CardDatabase::normalize_card_no in the engine.
fn normalize_card_no(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            'a'..='z' => result.push((ch as u8 - b'a' + b'A') as char),
            'ａ'..='ｚ' => result.push((ch as u32 - 'ａ' as u32 + 'A' as u32) as u8 as char),
            '＋' => result.push('+'),
            '！' => result.push('!'),
            '－' => result.push('-'),
            '＊' => result.push('*'),
            '＃' => result.push('#'),
            _ => result.push(ch),
        }
    }
    result
}

fn extract_card_no_and_qty(line: &str) -> (&str, usize) {
    let parts: Vec<&str> = line.split(" x ").collect();
    if parts.len() == 2 {
        let a = parts[0].trim();
        let b = parts[1].trim();
        // Try QTY x CARD_NO first, then CARD_NO x QTY
        if let Ok(qty) = a.parse::<usize>() {
            (b, qty)
        } else if let Ok(qty) = b.parse::<usize>() {
            (a, qty)
        } else {
            (line.trim(), 1)
        }
    } else {
        (line.trim(), 1)
    }
}

fn main() {
    let cwd = std::env::current_dir().unwrap();
    let repo_root = if cwd.ends_with("bake_cards") {
        cwd.parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned()
    } else {
        cwd
    };

    let out = repo_root.join("ports/psp/baked");
    fs::create_dir_all(&out).ok();

    // Load all cards as object
    let cards_path = repo_root.join("cards/cards.json");
    let cards_json = fs::read_to_string(&cards_path)
        .unwrap_or_else(|_| panic!("{} not found", cards_path.display()));
    let cards_map: serde_json::Value =
        serde_json::from_str(&cards_json).expect("Failed to parse cards JSON");
    let cards_obj = match &cards_map {
        serde_json::Value::Object(m) => m,
        _ => panic!("cards.json must be a JSON object"),
    };

    // Build a normalized (uppercased) key → card value map for case-insensitive lookups,
    // and normalized key → original key mapping for ability map tracking.
    let mut normalized_map: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    let mut norm_to_orig: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (key, val) in cards_obj.iter() {
        let norm = normalize_card_no(key);
        normalized_map.entry(norm.clone()).or_insert(val.clone());
        norm_to_orig.entry(norm).or_insert(key.clone());
    }

    // Load abilities.json for pre-attaching to cards
    let abilities_path = repo_root.join("cards/abilities.json");
    let abilities_value: Option<serde_json::Value> = fs::read_to_string(&abilities_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    // Read decks
    let decks_path = repo_root.join("web_ui/decks");
    let mut deck_entries: Vec<serde_json::Value> = Vec::new();
    let mut all_needed: HashSet<String> = HashSet::new();
    const MAX_DECKS: usize = 16;

    if let Ok(entries) = fs::read_dir(&decks_path) {
        let mut deck_index = 0usize;
        for entry in entries.flatten() {
            if deck_index >= MAX_DECKS {
                break;
            }
            if entry.path().extension().map_or(false, |e| e == "txt") {
                let name = entry
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let content = fs::read_to_string(entry.path()).unwrap_or_default();
                let mut card_nos: Vec<String> = Vec::new();
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let (no, qty) = extract_card_no_and_qty(line);
                    let normalized = normalize_card_no(no);
                    for _ in 0..qty {
                        card_nos.push(normalized.clone());
                    }
                }

                // Collect unique card numbers for this deck
                let unique_nos: HashSet<&str> = card_nos.iter().map(|s| s.as_str()).collect();
                let mut deck_cards: Vec<serde_json::Value> = Vec::new();
                for no in &unique_nos {
                    if let Some(card_val) = normalized_map.get(*no) {
                        deck_cards.push(card_val.clone());
                        // Track original cards.json key for ability map matching
                        if let Some(orig_key) = norm_to_orig.get(*no) {
                            all_needed.insert(orig_key.clone());
                        }
                    }
                }

                // Ensure default energy card is always available
                let default_energy = "LL-E-005-SD";
                if !card_nos.iter().any(|n| n == default_energy) {
                    if let Some(card_val) = normalized_map.get(default_energy) {
                        deck_cards.push(card_val.clone());
                        if let Some(orig_key) = norm_to_orig.get(default_energy) {
                            all_needed.insert(orig_key.clone());
                        }
                        for _ in 0..12 {
                            card_nos.push(default_energy.to_string());
                        }
                    }
                }

                // Write per-deck card file with abilities pre-attached
                let deck_filename = format!("deck_{deck_index}_cards.json");
                let deck_cards_str = if let Some(ref abil_val) = abilities_value {
                    // Parse into Card structs, attach abilities, re-serialize
                    let cards: Vec<Card> = deck_cards
                        .iter()
                        .map(|v| serde_json::from_value(v.clone()).unwrap())
                        .collect();
                    let cards: Vec<Card> = CardLoader::attach_abilities(cards, abil_val);
                    serde_json::to_string(&cards).unwrap()
                } else {
                    serde_json::to_string(&deck_cards).unwrap()
                };
                fs::write(out.join(&deck_filename), &deck_cards_str).unwrap();
                println!(
                    "{}: deck={} cards={} bytes={}",
                    deck_filename,
                    name,
                    deck_cards.len(),
                    deck_cards_str.len()
                );

                // Deck entry for decks.json
                let mut deck_entry = serde_json::Map::new();
                deck_entry.insert("name".into(), serde_json::Value::String(name));
                deck_entry.insert(
                    "cards".into(),
                    serde_json::Value::Array(
                        card_nos
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
                deck_entry.insert("card_file".into(), serde_json::Value::String(deck_filename));
                deck_entries.push(serde_json::Value::Object(deck_entry));

                deck_index += 1;
            }
        }
    }

    // Write decks.json
    let decks_str = serde_json::to_string(&deck_entries).unwrap();
    fs::write(out.join("decks.json"), &decks_str).unwrap();
    println!(
        "decks.json: {} decks, {} bytes",
        deck_entries.len(),
        decks_str.len()
    );

    // Write deck_cards_all.json (array of card arrays, one per deck)
    let mut all_cards: Vec<Vec<serde_json::Value>> = Vec::new();
    for i in 0..deck_entries.len() {
        let filename = format!("deck_{i}_cards.json");
        let path = out.join(&filename);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cards) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                all_cards.push(cards);
            } else {
                all_cards.push(Vec::new());
            }
        } else {
            all_cards.push(Vec::new());
        }
    }
    let all_str = serde_json::to_string(&all_cards).unwrap();
    fs::write(out.join("deck_cards_all.json"), &all_str).unwrap();
    println!(
        "deck_cards_all.json: {} decks, {} bytes",
        all_cards.len(),
        all_str.len()
    );

    // Pre-bake ability map for ALL deck-referenced cards
    let abilities_path = repo_root.join("cards/abilities.json");
    if let Ok(abilities_json) = fs::read_to_string(&abilities_path) {
        if let Ok(abilities_value) = serde_json::from_str::<serde_json::Value>(&abilities_json) {
            let full_map: std::collections::HashMap<String, Vec<Ability>> =
                CardLoader::build_abilities_map(&abilities_value);
            let filtered: std::collections::HashMap<String, Vec<Ability>> = full_map
                .into_iter()
                .filter(|(k, _)| all_needed.contains(k))
                .collect();
            let map_str = serde_json::to_string(&filtered).unwrap();
            fs::write(out.join("ability_map.json"), &map_str).unwrap();
            println!(
                "ability_map.json: {} cards, {} bytes",
                filtered.len(),
                map_str.len()
            );
        } else {
            println!("WARNING: failed to parse abilities.json");
            fs::write(out.join("ability_map.json"), "{}").unwrap();
        }
    } else {
        println!("WARNING: abilities.json not found, abilities disabled");
        fs::write(out.join("ability_map.json"), "{}").unwrap();
    }
}
