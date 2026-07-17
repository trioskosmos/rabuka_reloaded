use std::collections::HashSet;
use std::fs;

fn extract_card_no(line: &str) -> &str {
    line.split(" x ").next().unwrap_or(line).trim()
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

    let out = repo_root.join("engine_psp/baked");
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

    // Read decks
    let decks_path = repo_root.join("web_ui/decks");
    let mut deck_entries: Vec<serde_json::Value> = Vec::new();
    let mut needed_nos: HashSet<String> = HashSet::new();

    if let Ok(entries) = fs::read_dir(&decks_path) {
        for entry in entries.flatten() {
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
                    let no = extract_card_no(line);
                    let qty: usize = line
                        .split(" x ")
                        .nth(1)
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(1);
                    for _ in 0..qty {
                        card_nos.push(no.to_string());
                    }
                }
                for cn in &card_nos {
                    needed_nos.insert(cn.clone());
                }
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
                deck_entries.push(serde_json::Value::Object(deck_entry));
            }
        }
    }

    // Write cards (only those in decks)
    let cards_array: Vec<serde_json::Value> = cards_obj
        .iter()
        .filter(|(key, _)| needed_nos.contains(*key))
        .map(|(_, v)| v.clone())
        .collect();
    let cards_str = serde_json::to_string(&cards_array).unwrap();
    fs::write(out.join("cards.json"), &cards_str).unwrap();
    println!(
        "cards.json: {} cards, {} bytes",
        cards_array.len(),
        cards_str.len()
    );

    // Write decks
    let decks_str = serde_json::to_string(&deck_entries).unwrap();
    fs::write(out.join("decks.json"), &decks_str).unwrap();
    println!(
        "decks.json: {} decks, {} bytes",
        deck_entries.len(),
        decks_str.len()
    );

    // Write abilities (pass full JSON, CardLoader::attach_abilities handles it)
    let abilities_path = repo_root.join("cards/abilities.json");
    if let Ok(abilities_json) = fs::read_to_string(&abilities_path) {
        fs::write(out.join("abilities.json"), &abilities_json).unwrap();
        println!("abilities.json: {} bytes", abilities_json.len());
    } else {
        println!("WARNING: abilities.json not found, abilities disabled");
        fs::write(out.join("abilities.json"), "{}").unwrap();
    }
}
