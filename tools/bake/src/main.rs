use std::collections::HashSet;
use std::fs;

use rabuka_engine::card::Card;
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
    let args: Vec<String> = std::env::args().collect();
    let repo_root = if args.len() > 1 {
        std::path::PathBuf::from(&args[1])
    } else {
        let cwd = std::env::current_dir().unwrap();
        if cwd.ends_with("tools/bake") || cwd.ends_with("tools\\bake") {
            cwd.parent().unwrap().parent().unwrap().to_owned()
        } else {
            cwd
        }
    };

    let out = repo_root.join("platforms/psp/baked");
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
                let deck_cards_str = if abilities_value.is_some() {
                    // Parse into Card structs, attach bytecode ability indices, re-serialize
                    let mut cards: Vec<Card> = deck_cards
                        .iter()
                        .map(|v| serde_json::from_value(v.clone()).unwrap())
                        .collect();
                    CardLoader::attach_abilities(&mut cards);
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

    // Ability map is no longer needed — abilities are baked as bytecode indices
    // into each card via CardLoader::attach_abilities().

    // === Produce flat binary card database for DS/embedded platforms ===
    // Binary format ("RBCD"):
    //   Header (12 bytes): magic[4], card_count(u16), deck_count(u16), str_table_size(u32)
    //   Card records (20 bytes each): card_no_idx(u16), name_idx(u16), card_type(u8),
    //     cost(u8), blade(u8), score(u8), base_heart(u8), group(u8), series(u8),
    //     unit(u8), blade_heart(u8), special_heart(u8), ability_ref(u16), reserved(u8)
    //   String table: packed null-terminated strings
    //   Deck index: per deck: name_idx(u16), card_count(u16), card_indices[u16; card_count]
    use std::collections::HashMap;

    let mut str_bytes: Vec<u8> = Vec::new();
    let mut str_offsets: HashMap<String, u16> = HashMap::new();

    let mut add_str = |s: &str, table: &mut Vec<u8>, offsets: &mut HashMap<String, u16>| -> u16 {
        if let Some(&off) = offsets.get(s) {
            return off;
        }
        let off = table.len() as u16;
        table.extend_from_slice(s.as_bytes());
        table.push(0);
        offsets.insert(s.to_string(), off);
        off
    };

    // Collect all unique cards across all decks with their parsed values
    let mut unique_cards: Vec<(String, Card)> = Vec::new();
    let mut card_idx_map: HashMap<String, u16> = HashMap::new();

    for i in 0..deck_entries.len() {
        let filename = format!("deck_{i}_cards.json");
        let path = out.join(&filename);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cards) = serde_json::from_str::<Vec<Card>>(&content) {
                for card in cards {
                    let cn = card.card_no.to_string();
                    if !card_idx_map.contains_key(&cn) {
                        let idx = unique_cards.len() as u16;
                        card_idx_map.insert(cn.clone(), idx);
                        unique_cards.push((cn, card));
                    }
                }
            }
        }
    }

    // Build binary card records (this populates str_bytes with card strings)
    fn heart_color_to_u8(c: rabuka_engine::card::HeartColor) -> u8 {
        use rabuka_engine::card::HeartColor;
        match c {
            HeartColor::Heart01 => 1,
            HeartColor::Heart02 => 2,
            HeartColor::Heart03 => 3,
            HeartColor::Heart04 => 4,
            HeartColor::Heart05 => 5,
            _ => 0,
        }
    }
    fn first_heart_u8(map: &rabuka_engine::card::HeartMap) -> u8 {
        map.keys()
            .next()
            .map(|c| heart_color_to_u8(*c))
            .unwrap_or(0)
    }

    let mut card_records: Vec<Vec<u8>> = Vec::new();
    for (cn, card) in &unique_cards {
        let card_no_off = add_str(cn, &mut str_bytes, &mut str_offsets);
        let name_off = add_str(&card.name, &mut str_bytes, &mut str_offsets);
        let card_type_u8 = match card.card_type {
            rabuka_engine::card::CardType::Live => 1u8,
            rabuka_engine::card::CardType::Energy => 2u8,
            _ => 0u8,
        };
        let base_heart_u8 = card
            .base_heart
            .as_ref()
            .map(|bh| first_heart_u8(&bh.hearts))
            .unwrap_or(0);
        let blade_heart_u8 = card
            .blade_heart
            .as_ref()
            .map(|bh| first_heart_u8(&bh.hearts))
            .unwrap_or(0);
        let special_heart_u8 = card
            .special_heart
            .as_ref()
            .map(|sh| first_heart_u8(&sh.hearts))
            .unwrap_or(0);
        let ability_ref = card.abilities.first().map(|ar| ar.idx()).unwrap_or(0);

        let mut rec = Vec::with_capacity(20);
        rec.extend_from_slice(&card_no_off.to_le_bytes());
        rec.extend_from_slice(&name_off.to_le_bytes());
        rec.push(card_type_u8);
        rec.push(card.cost.unwrap_or(0) as u8);
        rec.push(card.blade as u8);
        rec.push(card.score.unwrap_or(0) as u8);
        rec.push(base_heart_u8);
        rec.push(0); // group (not in binary, reconstructed from series if needed)
        rec.push(0); // series (not in binary, reconstructed from group if needed)
        rec.push(0); // unit (not in binary)
        rec.push(blade_heart_u8);
        rec.push(special_heart_u8);
        rec.extend_from_slice(&ability_ref.to_le_bytes());
        rec.push(0); // reserved
        card_records.push(rec);
    }

    // Build deck index entries (this populates str_bytes with deck names)
    let mut deck_index_entries: Vec<Vec<u8>> = Vec::new();
    for i in 0..deck_entries.len() {
        let deck_name = deck_entries[i]
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let name_off = add_str(deck_name, &mut str_bytes, &mut str_offsets);
        let filename = format!("deck_{i}_cards.json");
        let path = out.join(&filename);
        let card_nos: Vec<String> = if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cards) = serde_json::from_str::<Vec<Card>>(&content) {
                cards.iter().map(|c| c.card_no.to_string()).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        let mut entry = Vec::new();
        entry.extend_from_slice(&name_off.to_le_bytes());
        entry.extend_from_slice(&(card_nos.len() as u16).to_le_bytes());
        for cn in &card_nos {
            let idx = card_idx_map.get(cn).copied().unwrap_or(0);
            entry.extend_from_slice(&idx.to_le_bytes());
        }
        deck_index_entries.push(entry);
    }

    // Now serialize everything: header + card records + string table + deck index
    let mut bin: Vec<u8> = Vec::new();
    bin.extend_from_slice(b"RBCD");
    bin.extend_from_slice(&(unique_cards.len() as u16).to_le_bytes());
    bin.extend_from_slice(&(deck_entries.len() as u16).to_le_bytes());
    bin.extend_from_slice(&(str_bytes.len() as u32).to_le_bytes());

    for rec in &card_records {
        bin.extend_from_slice(rec);
    }

    bin.extend_from_slice(&str_bytes);

    for entry in &deck_index_entries {
        bin.extend_from_slice(entry);
    }

    let bin_path = out.join("cards_database.bin");
    fs::write(&bin_path, &bin).unwrap();
    println!(
        "cards_database.bin: {} cards, {} decks, {} strings, {} bytes",
        unique_cards.len(),
        deck_entries.len(),
        str_offsets.len(),
        bin.len()
    );
}
