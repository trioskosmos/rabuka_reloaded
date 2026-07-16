use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .map(Into::into)
        .unwrap_or_else(|| "engine_psp/baked".into());
    let cwd = std::env::current_dir().unwrap();
    let repo_root = if cwd.ends_with("engine_psp") {
        cwd.parent().unwrap().to_owned()
    } else {
        cwd
    };
    let out = repo_root.join(&out_dir);
    fs::create_dir_all(&out).ok();

    let cards_path = repo_root.join("cards/cards.json");
    let cards_json = fs::read_to_string(&cards_path)
        .unwrap_or_else(|_| panic!("{} not found", cards_path.display()));
    let cards_val: serde_json::Value =
        serde_json::from_str(&cards_json).expect("Failed to parse cards.json");
    let bin_bytes = rmp_serde::to_vec(&cards_val).expect("MessagePack serialization failed");
    let bin_out = out.join("cards.bin");
    fs::write(&bin_out, &bin_bytes).expect("Failed to write cards.bin");
    println!(
        "Wrote {} ({} bytes, JSON was {} bytes)",
        bin_out.display(),
        bin_bytes.len(),
        cards_json.len()
    );

    let decks_path = repo_root.join("web_ui/decks");
    let decks_out = out.join("decks.bin");
    let mut deck_list = serde_json::Map::new();
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
                    deck_list.insert(name, serde_json::Value::String(content));
                }
            }
        }
    }
    let deck_bytes = rmp_serde::to_vec(&deck_list).expect("MessagePack serialization failed");
    fs::write(&decks_out, &deck_bytes).expect("Failed to write decks.bin");
    println!(
        "Wrote {} ({} decks, {} bytes)",
        decks_out.display(),
        deck_list.len(),
        deck_bytes.len()
    );
}
