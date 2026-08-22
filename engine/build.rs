use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cards_json = Path::new(&manifest_dir).join("../cards/cards.json");
    let abilities_json = Path::new(&manifest_dir).join("../cards/abilities.json");
    let abilities_gen = Path::new(&manifest_dir).join("src/ability/abilities_gen.rs");

    println!("cargo:rerun-if-changed={}", cards_json.display());
    println!("cargo:rerun-if-changed={}", abilities_json.display());
    println!("cargo:rerun-if-changed={}", abilities_gen.display());
    let build_bin_z = Path::new(&manifest_dir).join("../cards/build/abilities.bin.z");
    println!("cargo:rerun-if-changed={}", build_bin_z.display());

    // Binary blobs embedded via include_bytes! in the *_gen.rs modules.
    // include_bytes! itself does track these files for change detection, but
    // declaring them here makes the data -> rebuild contract explicit and
    // covers the case where a generator rewrites a gen file with identical
    // bytes but different bin content ordering.
    println!(
        "cargo:rerun-if-changed={}",
        Path::new(&manifest_dir).join("../cards/build/cards.bin").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        Path::new(&manifest_dir)
            .join("../cards/build/abilities_strings.bin")
            .display()
    );
    let decks_bin_dir = Path::new(&manifest_dir).join("baked/decks");
    if decks_bin_dir.is_dir() {
        println!("cargo:rerun-if-changed={}", decks_bin_dir.display());
    }

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

    // Report current bytecode size (the authoritative metric, not abilities.json)
    // Prefer the compressed size (what actually ships on host) if available
    let build_bin_z = Path::new(&manifest_dir).join("../cards/build/abilities.bin.z");
    let build_bin = Path::new(&manifest_dir).join("../cards/build/abilities.bin");
    if let Ok(meta) = std::fs::metadata(&build_bin_z) {
        println!("cargo:warning=bytecode size: {} bytes (compressed, from {})", meta.len(), build_bin_z.display());
        if let Ok(meta2) = std::fs::metadata(&build_bin) {
            println!("cargo:warning=bytecode uncompressed: {} bytes", meta2.len());
        }
    } else if let Ok(meta) = std::fs::metadata(&build_bin) {
        println!("cargo:warning=bytecode size: {} bytes (from {})", meta.len(), build_bin.display());
    } else if let Ok(src) = std::fs::read_to_string(&abilities_gen) {
        // Fallback: count 0x.. entries in BYTECODE array when the bin was cleaned
        let count = src.matches("0x").count();
        // Each 0x.. is one byte in the hex dump; rough estimate
        println!("cargo:warning=bytecode size: ~{} bytes (estimated from abilities_gen.rs)", count);
    }
}
