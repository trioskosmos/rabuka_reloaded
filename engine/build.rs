use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cards_json = Path::new(&manifest_dir).join("../cards/cards.json");
    let abilities_json = Path::new(&manifest_dir).join("../cards/abilities.json");
    let abilities_gen = Path::new(&manifest_dir).join("src/ability/abilities_gen.rs");

    println!("cargo:rerun-if-changed={}", cards_json.display());
    println!("cargo:rerun-if-changed={}", abilities_json.display());
    println!("cargo:rerun-if-changed={}", abilities_gen.display());

    // Staleness check: abilities.json should be newer than cards.json
    if let (Ok(cards_meta), Ok(abilities_meta)) = (std::fs::metadata(&cards_json), std::fs::metadata(&abilities_json)) {
        if let (Ok(cards_time), Ok(abilities_time)) = (cards_meta.modified(), abilities_meta.modified()) {
            if abilities_time < cards_time {
                println!("cargo:warning=abilities.json is older than cards.json – run `python cards/ability_extraction/extract_card_abilities.py` from cards/ to regenerate");
            }
        }
    }

    // Also check abilities_gen.rs staleness vs abilities.json
    if let (Ok(abilities_meta), Ok(gen_meta)) = (std::fs::metadata(&abilities_json), std::fs::metadata(&abilities_gen)) {
        if let (Ok(abilities_time), Ok(gen_time)) = (abilities_meta.modified(), gen_meta.modified()) {
            if gen_time < abilities_time {
                println!("cargo:warning=abilities_gen.rs is older than abilities.json – run `python cards/compile_abilities.py`");
            }
        }
    }
}
