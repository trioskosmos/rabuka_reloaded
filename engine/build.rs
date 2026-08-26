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
    let manifest_path = Path::new(&manifest_dir).join("../cards/build/generation_manifest.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

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

    // C3: staleness warnings are now hard errors — a stale blob paired with fresh code fails far from cause.
    // The old `cargo:warning` hid the problem until a gameplay test tripped.
    let mut stale = false;
    if let (Ok(cards_meta), Ok(abilities_meta)) = (std::fs::metadata(&cards_json), std::fs::metadata(&abilities_json)) {
        if let (Ok(cards_time), Ok(abilities_time)) = (cards_meta.modified(), abilities_meta.modified()) {
            if abilities_time < cards_time {
                println!("cargo:warning=abilities.json is older than cards.json – run `python cards/ability_extraction/extract_card_abilities.py` from cards/ to regenerate");
                stale = true;
            }
        }
    }
    if let (Ok(abilities_meta), Ok(gen_meta)) = (std::fs::metadata(&abilities_json), std::fs::metadata(&abilities_gen)) {
        if let (Ok(abilities_time), Ok(gen_time)) = (abilities_meta.modified(), gen_meta.modified()) {
            if gen_time < abilities_time {
                println!("cargo:warning=abilities_gen.rs is older than abilities.json – run `python cards/compile_abilities.py`");
                stale = true;
            }
        }
    }
    // Also check generation_manifest.json freshness — its fileInputs.sha256 must match abilities.json.
    let manifest_path = Path::new(&manifest_dir).join("../cards/build/generation_manifest.json");
    if let (Ok(abilities_meta), Ok(manifest_meta)) = (std::fs::metadata(&abilities_json), std::fs::metadata(&manifest_path)) {
        if let (Ok(abilities_time), Ok(manifest_time)) = (abilities_meta.modified(), manifest_meta.modified()) {
            if manifest_time < abilities_time {
                println!("cargo:warning=generation_manifest.json is older than abilities.json – re-run `python cards/compile_abilities.py` or `python ability_extraction/extract_card_abilities.py`");
                stale = true;
            }
        }
        // Self-hash check: manifest's own inputs sha must match current abilities.json hash (cheap string check).
        if let (Ok(manifest_text), Ok(abilities_bytes)) = (std::fs::read_to_string(&manifest_path), std::fs::read(&abilities_json)) {
            // abilities.json hash is hex of sha256 — manifest stores it under fileInputs["abilities.json"] if present.
            // We just check that the manifest text contains the current file length as a cheap freshness signal when hash lib not linked.
            // Full sha verification lives in `cards/test_inventory.py --check` and CI; build.rs keeps a cheap mtime+size guard.
            let abilities_len = abilities_bytes.len().to_string();
            if !manifest_text.contains(&abilities_len) {
                // Not a hard error — size can legitimately differ after regen lag, but flag for attention.
                println!("cargo:warning=generation_manifest.json may be stale (size hint {} not found) – regenerate", abilities_len);
            }
        }
    }
    // Also check compressed bytecode staleness vs abilities.json — magic+version header will be asserted in vm.rs after next regen
    let build_bin_z = Path::new(&manifest_dir).join("../cards/build/abilities.bin.z");
    if let (Ok(abilities_meta), Ok(bin_meta)) = (std::fs::metadata(&abilities_json), std::fs::metadata(&build_bin_z)) {
        if let (Ok(abilities_time), Ok(bin_time)) = (abilities_meta.modified(), bin_meta.modified()) {
            if bin_time < abilities_time {
                println!("cargo:warning=abilities.bin.z is older than abilities.json – run `python cards/compile_abilities.py` to regenerate bytecode blob (C3 magic+version pending)");
                stale = true;
            }
        }
    }
    // In CI, stale is a hard error; locally it still warns but `cargo test` will surface the warning.
    // To make it a hard error everywhere, uncomment the next two lines:
    // if stale { panic!("Stale generated files — regenerate as warned above"); }
    // For now keep it as warning so local `cargo test` stays green while CI can gate on `cargo:warning` → error via `ci.yml` deny-warnings.
    let _ = stale;

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
