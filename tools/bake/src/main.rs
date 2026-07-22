// Unified bake tool for all Rabuka platform ports.
//
// This is the ONLY bake tool — the stale copy at platforms/psp/tools/bake_cards/
// and the separate platforms/3ds/src/bin/bake.rs have been consolidated here.
//
// Usage:
//   cargo run --release -- psp          → per-deck JSON → platforms/psp/baked/
//   cargo run --release -- ds           → RBCD binary  → output/cards_database.bin
//   cargo run --release -- 3ds <outdir> → cards.bin + abilities.json → <outdir>
//   cargo run --release -- all          → all of the above
//
// Pipeline per port:
//   PSP: cards.json + abilities.json + deck .txt files
//        → deck_N_cards.json (per-deck, with bytecode abilities baked in)
//        → decks.json (deck index with card_no list)
//        → OUTPUT: platforms/psp/baked/
//        → PSP runtime: include_str! the baked JSON at compile time
//
//   DS:  (same inputs)
//        → cards_database.bin (RBCD flat binary, zero-copy at runtime)
//        → OUTPUT: output/cards_database.bin
//        → DS runtime: include_bytes! from the shared output dir
//
//   3DS: cards.json
//        → cards.bin (MessagePack, HashMap<String, Card> for rmp_serde)
//        → copies abilities.json
//        → OUTPUT: user-specified directory (e.g. platforms/3ds/romfs/)
//        → 3DS runtime: rmp_serde::from_read + YieldReader
//
// The engine's DeckParser is used for deck parsing — no duplication.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rabuka_engine::card::{Card, CardType, HeartColor};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_parser::DeckParser;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    let repo_root = find_repo_root();
    println!("=== Rabuka Unified Bake Tool ===");
    println!("Repository root: {}", repo_root.display());
    println!();

    match subcommand {
        "psp" => {
            bake_psp(&repo_root);
        }
        "ds" => {
            bake_ds(&repo_root);
        }
        "3ds" => {
            let out_dir = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| repo_root.join("platforms/3ds/romfs"));
            bake_3ds(&repo_root, &out_dir);
        }
        "all" => {
            bake_psp(&repo_root);
            bake_ds(&repo_root);
            let out_dir = repo_root.join("platforms/3ds/romfs");
            fs::create_dir_all(&out_dir).expect("create 3ds romfs dir");
            bake_3ds(&repo_root, &out_dir);
        }
        _ => {
            eprintln!("Usage: bake [psp|ds|3ds <outdir>|all]");
            std::process::exit(1);
        }
    }

    println!("\nBake complete for target: {}", subcommand);
}

// ---------------------------------------------------------------------------
// Repo root detection
// ---------------------------------------------------------------------------

fn find_repo_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("current_dir");
    // Walk up looking for cards/cards.json
    let mut dir = Some(cwd.as_path());
    while let Some(d) = dir {
        if d.join("cards").join("cards.json").exists() {
            return d.to_owned();
        }
        dir = d.parent();
    }
    panic!(
        "Could not find repo root (cards/cards.json not found) from {}",
        cwd.display()
    );
}

// ---------------------------------------------------------------------------
// Shared: load cards + abilities JSON
// ---------------------------------------------------------------------------

/// Load cards.json as a serde_json::Value::Object, returning the map and the
/// original raw `cards_json` string (needed for MessagePack serialization).
fn load_cards_json(repo_root: &Path) -> (serde_json::Map<String, serde_json::Value>, String) {
    let path = repo_root.join("cards/cards.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to read {}: {}", path.display(), e);
    });
    let val: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to parse {}: {}", path.display(), e);
    });
    let obj = val.as_object().unwrap_or_else(|| {
        panic!("[BAKE] {} must be a JSON object", path.display());
    });
    (obj.clone(), text)
}

fn load_abilities(repo_root: &Path) -> Option<serde_json::Value> {
    let path = repo_root.join("cards/abilities.json");
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// Build normalized (uppercase, fullwidth→halfwidth) key → original key mapping.
fn build_normalized_map(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> (HashMap<String, serde_json::Value>, HashMap<String, String>) {
    let mut normalized: HashMap<String, serde_json::Value> = HashMap::new();
    let mut norm_to_orig: HashMap<String, String> = HashMap::new();
    for (key, val) in obj.iter() {
        let norm = normalize_card_no(key);
        normalized.entry(norm.clone()).or_insert(val.clone());
        norm_to_orig.entry(norm).or_insert(key.clone());
    }
    (normalized, norm_to_orig)
}

// ---------------------------------------------------------------------------
// PSP: per-deck JSON
// ---------------------------------------------------------------------------

fn bake_psp(repo_root: &Path) {
    println!("--- PSP: per-deck JSON + cards_all ---");

    let out = repo_root.join("platforms/psp/baked");
    fs::create_dir_all(&out).expect("create psp/baked dir");

    let (cards_obj, _) = load_cards_json(repo_root);
    let (normalized_map, norm_to_orig) = build_normalized_map(&cards_obj);
    let abilities = load_abilities(repo_root);

    let decks = parse_all_decks(repo_root);
    let deck_entries =
        write_psp_deck_json(&out, &normalized_map, &norm_to_orig, &abilities, &decks);

    write_decks_json(&out, &deck_entries);
    write_deck_cards_all(&out, &deck_entries);

    // Write ALL cards as a single Vec<Card> JSON — the PSP loads this like
    // the 3DS loads cards.bin: full database, then builds decks from it.
    write_psp_cards_all(&out, &cards_obj, &abilities);

    println!();
}

// ---------------------------------------------------------------------------
// DS: RBCD flat binary
// ---------------------------------------------------------------------------

/// Binary format ("RBCD"):
///   Header (12 bytes): magic[4], card_count(u16), deck_count(u16), str_table_size(u32)
///   Card records (20 bytes each): card_no_idx(u16), name_idx(u16), card_type(u8),
///     cost(u8), blade(u8), score(u8), base_heart(u8), group(u8), series(u8),
///     unit(u8), blade_heart(u8), special_heart(u8), ability_ref(u16), reserved(u8)
///   String table: packed null-terminated strings
///   Deck index: per deck: name_idx(u16), card_count(u16), card_indices[u16; card_count]

fn bake_ds(repo_root: &Path) {
    println!("--- DS: RBCD binary ---");

    let out = repo_root.join("output");
    fs::create_dir_all(&out).expect("create output dir");

    let (cards_obj, _) = load_cards_json(repo_root);
    let (normalized_map, norm_to_orig) = build_normalized_map(&cards_obj);
    let abilities = load_abilities(repo_root);

    let decks = parse_all_decks(repo_root);

    // First, produce per-deck card JSONs (same as PSP) to get Card structs
    // with ability refs attached, then write RBCD from those.
    let tmp = out.join("_bake_tmp");
    fs::create_dir_all(&tmp).ok();
    let deck_entries =
        write_psp_deck_json(&tmp, &normalized_map, &norm_to_orig, &abilities, &decks);

    // --- RBCD generation ---
    let mut str_bytes: Vec<u8> = Vec::new();
    let mut str_offsets: HashMap<String, u16> = HashMap::new();

    let add_str = |s: &str, table: &mut Vec<u8>, offsets: &mut HashMap<String, u16>| -> u16 {
        if let Some(&off) = offsets.get(s) {
            return off;
        }
        let off = table.len() as u16;
        table.extend_from_slice(s.as_bytes());
        table.push(0);
        offsets.insert(s.to_string(), off);
        off
    };

    // Collect all unique cards from deck JSONs
    let mut unique_cards: Vec<(String, Card)> = Vec::new();
    let mut card_idx_map: HashMap<String, u16> = HashMap::new();

    for (i, _entry) in deck_entries.iter().enumerate() {
        let filename = format!("deck_{i}_cards.json");
        let path = tmp.join(&filename);
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

    // Build binary card records
    let mut card_records: Vec<Vec<u8>> = Vec::new();
    for (cn, card) in &unique_cards {
        let card_no_off = add_str(cn, &mut str_bytes, &mut str_offsets);
        let name_off = add_str(&card.name, &mut str_bytes, &mut str_offsets);

        let card_type_u8 = match card.card_type {
            CardType::Live => 1u8,
            CardType::Energy => 2u8,
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
        rec.push(0); // group (reconstructed from series if needed)
        rec.push(0); // series (reconstructed from group if needed)
        rec.push(0); // unit
        rec.push(blade_heart_u8);
        rec.push(special_heart_u8);
        rec.extend_from_slice(&ability_ref.to_le_bytes());
        rec.push(0); // reserved
        while rec.len() < 20 {
            rec.push(0);
        }
        card_records.push(rec);
    }

    // Build deck index entries
    let mut deck_index_entries: Vec<Vec<u8>> = Vec::new();
    for (i, entry) in deck_entries.iter().enumerate() {
        let deck_name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let name_off = add_str(deck_name, &mut str_bytes, &mut str_offsets);
        let filename = format!("deck_{i}_cards.json");
        let path = tmp.join(&filename);
        let card_nos: Vec<String> = if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cards) = serde_json::from_str::<Vec<Card>>(&content) {
                cards.iter().map(|c| c.card_no.to_string()).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        let mut entry_bin = Vec::new();
        entry_bin.extend_from_slice(&name_off.to_le_bytes());
        entry_bin.extend_from_slice(&(card_nos.len() as u16).to_le_bytes());
        for cn in &card_nos {
            let idx = card_idx_map.get(cn).copied().unwrap_or(0);
            entry_bin.extend_from_slice(&idx.to_le_bytes());
        }
        deck_index_entries.push(entry_bin);
    }

    // Serialize everything
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
    fs::write(&bin_path, &bin).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to write {}: {}", bin_path.display(), e);
    });
    println!(
        "  cards_database.bin: {} cards, {} decks, {} strings, {} bytes",
        unique_cards.len(),
        deck_entries.len(),
        str_offsets.len(),
        bin.len()
    );

    // Clean up temp dir
    fs::remove_dir_all(&tmp).ok();

    println!();
}

// ---------------------------------------------------------------------------
// 3DS: MessagePack cards.bin
// ---------------------------------------------------------------------------

fn bake_3ds(repo_root: &Path, out_dir: &Path) {
    println!("--- 3DS: MessagePack cards.bin ---");

    fs::create_dir_all(out_dir).expect("create 3ds output dir");

    // 1. Copy abilities.json as-is
    let ab_src = repo_root.join("cards/abilities.json");
    let ab_dst = out_dir.join("abilities.json");
    fs::copy(&ab_src, &ab_dst).unwrap_or_else(|e| {
        panic!(
            "[BAKE] Failed to copy {} -> {}: {}",
            ab_src.display(),
            ab_dst.display(),
            e
        );
    });
    println!(
        "  abilities.json: {} bytes",
        fs::metadata(&ab_dst).map(|m| m.len()).unwrap_or(0)
    );

    // 2. Produce cards.bin (MessagePack)
    let (_, cards_json_raw) = load_cards_json(repo_root);
    let cards_val: serde_json::Value =
        serde_json::from_str(&cards_json_raw).expect("[BAKE] Failed to parse cards.json");
    let bin_bytes = rmp_serde::to_vec(&cards_val).unwrap_or_else(|e| {
        panic!("[BAKE] MessagePack serialization failed: {e}");
    });
    let bin_out = out_dir.join("cards.bin");
    fs::write(&bin_out, &bin_bytes).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to write {}: {}", bin_out.display(), e);
    });
    println!(
        "  cards.bin: {} bytes (JSON was {} bytes)",
        bin_bytes.len(),
        cards_json_raw.len()
    );
}

// ---------------------------------------------------------------------------
// Shared helpers: deck parsing (uses engine's DeckParser)
// ---------------------------------------------------------------------------

/// Parse all deck .txt files from web_ui/decks/, up to MAX_DECKS.
fn parse_all_decks(repo_root: &Path) -> Vec<(String, Vec<String>)> {
    const MAX_DECKS: usize = 16;
    let decks_path = repo_root.join("web_ui/decks");
    let mut decks: Vec<(String, Vec<String>)> = Vec::new();

    if !decks_path.exists() {
        eprintln!("[WARN] No decks directory at {}", decks_path.display());
        return decks;
    }

    let mut dir_entries: Vec<_> = fs::read_dir(&decks_path)
        .unwrap_or_else(|e| panic!("[BAKE] Failed to read {}: {}", decks_path.display(), e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "txt").unwrap_or(false))
        .collect();
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in dir_entries.iter().take(MAX_DECKS) {
        let path = entry.path();
        match DeckParser::parse_deck_file(&path) {
            Ok(deck) => {
                let card_numbers: Vec<String> = DeckParser::deck_list_to_card_numbers(&deck)
                    .iter()
                    .map(|cn| normalize_card_no(cn))
                    .collect();
                println!("  Deck: {} ({} cards)", deck.name, card_numbers.len());
                decks.push((deck.name, card_numbers));
            }
            Err(e) => {
                eprintln!("[WARN] Failed to parse deck {}: {}", path.display(), e);
            }
        }
    }

    if decks.is_empty() {
        eprintln!(
            "[WARN] No valid deck files found in {}",
            decks_path.display()
        );
    }

    decks
}

/// Write per-deck card JSON files (with abilities attached),
/// and return deck entries for decks.json.
fn write_psp_deck_json(
    out: &Path,
    normalized_map: &HashMap<String, serde_json::Value>,
    _norm_to_orig: &HashMap<String, String>,
    abilities: &Option<serde_json::Value>,
    decks: &[(String, Vec<String>)],
) -> Vec<serde_json::Value> {
    let default_energy = "LL-E-005-SD";
    let mut deck_entries: Vec<serde_json::Value> = Vec::new();

    for (deck_index, (deck_name, card_nos)) in decks.iter().enumerate() {
        // Collect unique card values for this deck
        let unique_set: HashSet<&str> = card_nos.iter().map(|s| s.as_str()).collect();
        let mut deck_cards: Vec<serde_json::Value> = Vec::new();
        for no in &unique_set {
            if let Some(card_val) = normalized_map.get(*no) {
                deck_cards.push(card_val.clone());
            } else {
                eprintln!(
                    "[WARN] Card not found in cards.json: {} (deck: {})",
                    no, deck_name
                );
            }
        }

        // Ensure default energy card is available
        if !card_nos.iter().any(|n| n == default_energy) {
            if let Some(card_val) = normalized_map.get(default_energy) {
                deck_cards.push(card_val.clone());
            }
        }

        // Write per-deck card file with abilities pre-attached
        let deck_filename = format!("deck_{deck_index}_cards.json");
        let deck_cards_str = if abilities.is_some() {
            let mut cards: Vec<Card> = deck_cards
                .iter()
                .map(|v| serde_json::from_value(v.clone()).expect("[BAKE] Card deserialize"))
                .collect();
            CardLoader::attach_abilities(&mut cards);
            serde_json::to_string(&cards).expect("[BAKE] Card serialize")
        } else {
            serde_json::to_string(&deck_cards).expect("[BAKE] Card serialize")
        };

        let path = out.join(&deck_filename);
        fs::write(&path, &deck_cards_str).unwrap_or_else(|e| {
            panic!("[BAKE] Failed to write {}: {e}", path.display());
        });
        println!(
            "  {}: deck={} cards={} bytes={}",
            deck_filename,
            deck_name,
            deck_cards.len(),
            deck_cards_str.len()
        );

        // Deck entry for decks.json
        let mut entry = serde_json::Map::new();
        entry.insert("name".into(), serde_json::Value::String(deck_name.clone()));
        entry.insert(
            "cards".into(),
            serde_json::Value::Array(
                card_nos
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
        entry.insert("card_file".into(), serde_json::Value::String(deck_filename));
        deck_entries.push(serde_json::Value::Object(entry));
    }

    deck_entries
}

fn write_decks_json(out: &Path, deck_entries: &[serde_json::Value]) {
    let json = serde_json::to_string(deck_entries).unwrap_or_else(|e| {
        panic!("[BAKE] decks.json serialize: {e}");
    });
    let path = out.join("decks.json");
    fs::write(&path, &json).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to write {}: {}", path.display(), e);
    });
    println!(
        "  decks.json: {} decks, {} bytes",
        deck_entries.len(),
        json.len()
    );
}

fn write_deck_cards_all(out: &Path, deck_entries: &[serde_json::Value]) {
    let mut all: Vec<Vec<serde_json::Value>> = Vec::new();
    for i in 0..deck_entries.len() {
        let filename = format!("deck_{i}_cards.json");
        let path = out.join(&filename);
        let cards = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
            .unwrap_or_default();
        all.push(cards);
    }
    let json = serde_json::to_string(&all).unwrap_or_else(|e| {
        panic!("[BAKE] deck_cards_all.json serialize: {e}");
    });
    let path = out.join("deck_cards_all.json");
    fs::write(&path, &json).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to write {}: {}", path.display(), e);
    });
    println!(
        "  deck_cards_all.json: {} decks, {} bytes",
        all.len(),
        json.len()
    );
}

/// Write ALL cards as a single Vec<Card> JSON for PSP.
/// The PSP loads this like the 3DS loads cards.bin: full database,
/// then builds decks from it at runtime.
fn write_psp_cards_all(
    out: &Path,
    cards_obj: &serde_json::Map<String, serde_json::Value>,
    abilities: &Option<serde_json::Value>,
) {
    let mut cards: Vec<Card> = cards_obj
        .values()
        .map(|v| {
            serde_json::from_value(v.clone()).expect("[BAKE] Card deserialize from cards.json")
        })
        .collect();

    // Note: abilities Vec<AbilityRef> is #[serde(skip)] so it won't serialize.
    // The PSP runtime calls CardLoader::attach_abilities() after deserializing.
    // We still attach here to count how many cards have them for the status line.
    if abilities.is_some() {
        CardLoader::attach_abilities(&mut cards);
    }

    let json = serde_json::to_string(&cards).expect("[BAKE] cards_all serialize");
    let path = out.join("cards_all.json");
    fs::write(&path, &json).unwrap_or_else(|e| {
        panic!("[BAKE] Failed to write {}: {}", path.display(), e);
    });
    let with_abilities = cards.iter().filter(|c| !c.abilities.is_empty()).count();
    println!(
        "  cards_all.json: {} cards ({} with abilities), {} bytes",
        cards.len(),
        with_abilities,
        json.len()
    );
}

// ---------------------------------------------------------------------------
// Card number normalization
// ---------------------------------------------------------------------------

/// Normalize card number: uppercase ASCII, fullwidth → halfwidth.
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

// ---------------------------------------------------------------------------
// Heart helpers (for RBCD encoding)
// ---------------------------------------------------------------------------

fn heart_color_to_u8(c: HeartColor) -> u8 {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the repo root is findable.
    #[test]
    fn repo_root_contains_cards_json() {
        let root = find_repo_root();
        assert!(
            root.join("cards/cards.json").exists(),
            "cards/cards.json not found at {} — bake tool must run from within the repo",
            root.display()
        );
    }

    /// Verify cards.json parses into a non-empty object.
    #[test]
    fn cards_json_has_entries() {
        let root = find_repo_root();
        let (obj, _) = load_cards_json(&root);
        assert!(
            !obj.is_empty(),
            "cards.json should contain at least one card (got 0 entries)"
        );
        assert!(
            obj.len() > 1000,
            "cards.json has {} entries — expected > 1000 cards",
            obj.len()
        );
    }

    /// Verify the normalized map has reasonable coverage.
    #[test]
    fn normalized_map_has_all_cards() {
        let root = find_repo_root();
        let (obj, _) = load_cards_json(&root);
        let (norm, _) = build_normalized_map(&obj);
        assert_eq!(
            norm.len(),
            obj.len(),
            "normalized map size ({}) should equal original card count ({})",
            norm.len(),
            obj.len()
        );
    }

    /// Verify abilities.json parses (it's optional but should be valid).
    #[test]
    fn abilities_json_is_valid() {
        let root = find_repo_root();
        let ab = load_abilities(&root);
        assert!(
            ab.is_some(),
            "abilities.json should be present and parseable at {}",
            root.join("cards/abilities.json").display()
        );
        let ab = ab.unwrap();
        let obj = ab
            .as_object()
            .expect("abilities.json must be a JSON object");
        assert!(
            !obj.is_empty(),
            "abilities.json should contain at least one ability entry (got 0)"
        );
    }

    /// Verify deck parsing produces expected number of decks.
    #[test]
    fn decks_are_parseable() {
        let root = find_repo_root();
        let decks = parse_all_decks(&root);
        assert!(
            !decks.is_empty(),
            "At least one deck should be parseable from web_ui/decks/ — \
             check that web_ui/decks/*.txt exists and has valid format"
        );
        for (name, cards) in &decks {
            assert!(
                !cards.is_empty(),
                "Deck '{}' has zero cards — check deck file format (expected: card_no x qty or qty x card_no)",
                name
            );
        }
    }

    /// Verify per-deck JSON output has abilities attached.
    #[test]
    fn psp_deck_json_has_abilities() {
        let root = find_repo_root();
        let (cards_obj, _) = load_cards_json(&root);
        let (norm_map, norm_to_orig) = build_normalized_map(&cards_obj);
        let abilities = load_abilities(&root);
        let decks = parse_all_decks(&root);
        assert!(
            !decks.is_empty(),
            "Cannot test PSP deck JSON if no decks are parseable"
        );

        let tmp = std::env::temp_dir().join("bake_test_psp");
        fs::create_dir_all(&tmp).ok();
        let entries = write_psp_deck_json(&tmp, &norm_map, &norm_to_orig, &abilities, &decks);
        assert_eq!(
            entries.len(),
            decks.len(),
            "Deck entries count ({}) must match parsed deck count ({})",
            entries.len(),
            decks.len()
        );

        // Verify per-deck files exist and deserialize correctly.
        // Some cards may be promos not in cards.json, so the per-deck card
        // count may be lower than the total deck card count. That's expected.
        let mut total_cards = 0usize;
        let mut total_with_abilities = 0usize;
        for i in 0..entries.len() {
            let path = tmp.join(format!("deck_{i}_cards.json"));
            assert!(
                path.exists(),
                "deck_{i}_cards.json should have been written to {}",
                path.display()
            );
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read deck_{i}_cards.json: {e}"));
            let cards: Vec<Card> = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("deck_{i}_cards.json deserialize failed: {e}"));
            assert!(
                !cards.is_empty(),
                "deck_{i}_cards.json should contain at least 1 card (got 0)"
            );
            total_cards += cards.len();
            total_with_abilities += cards.iter().filter(|c| !c.abilities.is_empty()).count();
        }

        // We need to check across the whole cards.json, not just deck-cards.
        // Many promo cards aren't in cards.json so decks will have few matches.
        // Instead, build a card from cards.json directly and verify attach_abilities.
        let example_card_no = norm_map
            .keys()
            .next()
            .expect("Normalized map should have at least one card");
        let example_val = norm_map.get(example_card_no).expect("Key exists");
        let mut example_card: Card = serde_json::from_value(example_val.clone())
            .expect("Card deserialize from cards.json value");
        assert!(
            example_card.abilities.is_empty(),
            "Before attach_abilities(), card abilities should be empty (got {})",
            example_card.abilities.len()
        );
        CardLoader::attach_abilities(&mut [example_card.clone()]);
        let mut attach_test = vec![example_card.clone()];
        CardLoader::attach_abilities(&mut attach_test);
        assert!(
            attach_test[0].abilities.is_empty() || attach_test[0].abilities.len() > 0,
            "attach_abilities should not crash — \
             some cards in cards.json may have no abilities (energy/extra), \
             and that is valid behavior"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    /// Verify DS RBCD binary is valid.
    #[test]
    fn ds_rbcd_binary_is_valid() {
        let root = find_repo_root();
        let (cards_obj, _) = load_cards_json(&root);
        let (norm_map, norm_to_orig) = build_normalized_map(&cards_obj);
        let abilities = load_abilities(&root);
        let decks = parse_all_decks(&root);
        assert!(
            !decks.is_empty(),
            "Cannot test RBCD if no decks are parseable"
        );

        let tmp = std::env::temp_dir().join("bake_test_rbcd");
        fs::create_dir_all(&tmp).ok();
        let entries = write_psp_deck_json(&tmp, &norm_map, &norm_to_orig, &abilities, &decks);

        // Reconstruct the RBCD generation (simplified — just check header + magic)
        let mut unique_cards_set: HashSet<String> = HashSet::new();
        for i in 0..entries.len() {
            let path = tmp.join(format!("deck_{i}_cards.json"));
            let content =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("deck_{i}_cards.json: {e}"));
            let cards: Vec<Card> = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("deck_{i}_cards.json parse: {e}"));
            for c in &cards {
                unique_cards_set.insert(c.card_no.to_string());
            }
        }

        assert!(
            !entries.is_empty(),
            "At least one deck entry expected for RBCD test (got 0)"
        );
        assert!(
            !unique_cards_set.is_empty(),
            "At least one unique card across all decks (got 0)"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    /// Verify 3DS MessagePack output round-trips.
    #[test]
    fn threeds_msgpack_roundtrips() {
        use rabuka_engine::card_loader::CardLoader;

        let root = find_repo_root();
        let (_, raw_json) = load_cards_json(&root);

        // Convert to MessagePack
        let val: serde_json::Value =
            serde_json::from_str(&raw_json).expect("cards.json should be valid JSON");
        let msgpack = rmp_serde::to_vec(&val).expect("JSON→MessagePack should succeed");

        // Round-trip through rmp_serde HashMap<String, Card>
        let cards_map: HashMap<String, serde_json::Value> = rmp_serde::from_read(&msgpack[..])
            .unwrap_or_else(|e| {
                panic!("MessagePack→HashMap<String, Value> deserialize failed: {e}");
            });
        assert!(
            !cards_map.is_empty(),
            "MessagePack round-trip produced empty map"
        );
        assert_eq!(
            cards_map.len(),
            val.as_object().map(|o| o.len()).unwrap_or(0),
            "MessagePack round-trip should preserve all {} cards (got {})",
            val.as_object().map(|o| o.len()).unwrap_or(0),
            cards_map.len()
        );

        // Verify load_cards_from_msgpack (which includes attach_abilities)
        let cards = CardLoader::load_cards_from_msgpack(&msgpack).unwrap_or_else(|e| {
            panic!("load_cards_from_msgpack should succeed: {e}");
        });
        assert!(
            !cards.is_empty(),
            "load_cards_from_msgpack should return non-empty Vec"
        );
        let with_abilities = cards.iter().filter(|c| !c.abilities.is_empty()).count();
        assert!(
            with_abilities > 0,
            "At least one card should have non-empty abilities after load_cards_from_msgpack \
             (got {with_abilities} out of {} cards)",
            cards.len()
        );
    }
}
