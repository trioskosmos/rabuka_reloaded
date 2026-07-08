// Pre-bake cards for the 3DS.
// Produces cards.bin (MessagePack of original cards.json) and
// copies abilities.json — the 3DS loads cards.bin via rmp_serde
// (fast, supports YieldReader for watchdog) and attaches abilities
// from abilities.json just like the web server.
//
// RAM: sizeof(Ability) = 20 KB (19968 bytes).  1727 instances = 33 MB.
// cards.bin is MessagePack (2100 KB vs 3038 KB JSON — 33% smaller).
// See rabuka_3ds.rs header for full numbers.
//
// Called by build_3ds.bat before the 3DS cross-compile.

use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .map(Into::into)
        .unwrap_or_else(|| "engine_3ds/romfs".into());
    let cwd = std::env::current_dir().unwrap();
    let repo_root = if cwd.ends_with("engine_3ds") {
        cwd.parent().unwrap().to_owned()
    } else {
        cwd
    };
    let out = repo_root.join(&out_dir);
    fs::create_dir_all(&out).ok();

    // 1. Copy abilities.json as-is
    let ab_src = repo_root.join("cards/abilities.json");
    let ab_dst = out.join("abilities.json");
    println!("Copying {} -> {}", ab_src.display(), ab_dst.display());
    fs::copy(&ab_src, &ab_dst).unwrap_or_else(|e| panic!("copy: {}", e));
    println!("  {} bytes", fs::metadata(&ab_dst).unwrap().len());

    // 2. Produce cards.bin (MessagePack) from original cards.json
    //    This preserves the exact HashMap format that Card::Deserialize expects.
    let cards_path = repo_root.join("cards/cards.json");
    println!("Loading {}", cards_path.display());
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
}
