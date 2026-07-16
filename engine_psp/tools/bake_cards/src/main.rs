use std::fs;
use std::path::PathBuf;

fn main() {
    // Determine repo root: if CWD is engine_psp/tools/bake_cards, go up 3 levels
    let cwd = std::env::current_dir().unwrap();
    let repo_root = if cwd.ends_with("bake_cards") {
        // running from tools/bake_cards/
        cwd.parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned()
    } else if cwd.ends_with("tools") {
        // running from tools/
        cwd.parent().unwrap().to_owned()
    } else if cwd.ends_with("engine_psp") {
        // running from engine_psp/
        cwd.parent().unwrap().to_owned()
    } else {
        // assume we're at repo root
        cwd
    };

    let out = repo_root.join("engine_psp/baked");
    fs::create_dir_all(&out).ok();

    // Bake cards
    let cards_path = repo_root.join("cards/cards.json");
    let cards_json = fs::read_to_string(&cards_path)
        .unwrap_or_else(|_| panic!("{} not found", cards_path.display()));
    let cards_out = out.join("cards.json");
    fs::write(&cards_out, &cards_json).expect("Failed to write cards.json");
    println!("Wrote {} ({} bytes)", cards_out.display(), cards_json.len());

    // Bake decks
    let decks_path = repo_root.join("web_ui/decks");
    let decks_out = out.join("decks.json");
    let mut deck_entries: Vec<serde_json::Value> = Vec::new();

    if let Ok(entries) = fs::read_dir(&decks_path) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "txt") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let name = entry
                        .path()
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let card_nos: Vec<&str> = content
                        .lines()
                        .map(|l| l.trim())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    let mut entry = serde_json::Map::new();
                    entry.insert("name".into(), serde_json::Value::String(name));
                    entry.insert(
                        "cards".into(),
                        serde_json::Value::Array(
                            card_nos
                                .iter()
                                .map(|s| serde_json::Value::String(s.to_string()))
                                .collect(),
                        ),
                    );
                    deck_entries.push(serde_json::Value::Object(entry));
                }
            }
        }
    }

    let deck_json = serde_json::to_string(&deck_entries).expect("Failed to serialize decks");
    fs::write(&decks_out, &deck_json).expect("Failed to write decks.json");
    println!(
        "Wrote {} ({} decks, {} bytes)",
        decks_out.display(),
        deck_entries.len(),
        deck_json.len()
    );
}
