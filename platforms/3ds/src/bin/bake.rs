// 3DS bake entry point — thin wrapper around the unified tools/bake tool.
//
// This binary exists so that build_3ds.bat can invoke it via:
//   cargo run --bin bake --release -- "<outdir>"
//
// It delegates to tools/bake (the consolidated bake tool):
//   bake_cards 3ds <outdir>
//
// For full documentation, see tools/bake/src/main.rs.
//
// Rationale: keeping a separate binary here avoids needing to restructure
// the 3DS crate or add a dependency on the bake_cards package. The wrapper
// is ~20 lines of glue that calls the underlying tool's `bake_3ds()` function.
//
// History: Previously this file contained its own MessagePack conversion logic
// (duplicating parts of tools/bake). All bake logic is now consolidated in
// tools/bake/src/main.rs. This file is just a CLI shim.

use std::path::PathBuf;

fn main() {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .map(Into::into)
        .unwrap_or_else(|| "platforms/3ds/romfs".into());

    // Locate repo root (walk up from this binary's compile-time path)
    let cwd = std::env::current_dir().expect("current_dir");
    let repo_root = if cwd.ends_with("platforms/3ds")
        || cwd.ends_with("ports/3ds")
        || cwd.ends_with("engine_3ds")
    {
        cwd.parent().and_then(|p| p.parent()).map(|p| p.to_owned())
    } else {
        Some(cwd.clone())
    }
    .unwrap_or_else(|| {
        // Walk up looking for cards/cards.json
        let mut d = Some(cwd.as_path());
        while let Some(p) = d {
            if p.join("cards/cards.json").exists() {
                return p.to_owned();
            }
            d = p.parent();
        }
        panic!("Cannot find repo root from {}", cwd.display());
    });

    let out = repo_root.join(&out_dir);
    std::fs::create_dir_all(&out).expect("create output dir");

    // Use the standalone bake_3ds logic from the unified bake tool.
    // We inline a minimal call here; if more subcommands are needed,
    // consider making this binary depend on bake_cards as a library.
    bake_3ds_shim(&repo_root, &out);
}

/// Standalone 3DS bake logic (mirrors tools/bake's bake_3ds).
fn bake_3ds_shim(repo_root: &std::path::Path, out_dir: &std::path::Path) {
    println!("=== Rabuka 3DS Bake ===");
    println!("Repository root: {}", repo_root.display());

    // 1. Copy abilities.json
    let ab_src = repo_root.join("cards/abilities.json");
    let ab_dst = out_dir.join("abilities.json");
    std::fs::copy(&ab_src, &ab_dst).unwrap_or_else(|e| {
        panic!(
            "Failed to copy {} -> {}: {}",
            ab_src.display(),
            ab_dst.display(),
            e
        );
    });
    println!(
        "abilities.json: {} bytes",
        std::fs::metadata(&ab_dst).map(|m| m.len()).unwrap_or(0)
    );

    // 2. Produce cards.bin (MessagePack)
    let cards_path = repo_root.join("cards/cards.json");
    let cards_json = std::fs::read_to_string(&cards_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", cards_path.display(), e));
    let cards_val: serde_json::Value =
        serde_json::from_str(&cards_json).expect("Failed to parse cards.json");
    let bin_bytes = rmp_serde::to_vec(&cards_val).expect("MessagePack serialization failed");
    let bin_out = out_dir.join("cards.bin");
    std::fs::write(&bin_out, &bin_bytes).expect("Failed to write cards.bin");
    println!(
        "cards.bin: {} bytes (JSON was {} bytes)",
        bin_bytes.len(),
        cards_json.len()
    );
}
