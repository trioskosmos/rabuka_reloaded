//! Dump EN/JA descriptions for every baked ability (audit F3).
//!
//! Writes `../test_output/describe_dump.json`: one entry per ability with the
//! original full_text plus its template-rendered EN/JA descriptions. The
//! Python side (`cards/describe_fidelity_report.py`) normalizes and diffs
//! them to rank abilities whose structure least matches their Japanese text.
//!
//! Non-gating diagnostic: run via `cargo run --bin describe_dump`.

use rabuka_engine::ability::ability_store::AbilityRef;
use rabuka_engine::ability::abilities_gen::NUM_ABILITIES;
use rabuka_engine::ability::describe::{describe_effect_en, describe_effect_ja};
use std::io::Write;

fn main() {
    let mut out = String::from("[\n");
    for idx in 0..NUM_ABILITIES {
        let ability = AbilityRef(idx as u16).resolve();
        let (en, ja) = match ability.effect.as_ref() {
            Some(effect) => (describe_effect_en(effect), describe_effect_ja(effect)),
            None => (String::new(), String::new()),
        };
        let entry = serde_json::json!({
            "index": idx,
            "full_text": ability.full_text,
            "describe_en": en,
            "describe_ja": ja,
        });
        out.push_str(&serde_json::to_string(&entry).unwrap());
        out.push_str(",\n");
    }
    // Drop trailing comma
    if out.ends_with(",\n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
    out.push_str("]\n");

    let path = std::path::Path::new("../test_output/describe_dump.json");
    let mut f = std::fs::File::create(path).expect("create dump file");
    f.write_all(out.as_bytes()).expect("write dump");
    println!("wrote {} abilities to {}", NUM_ABILITIES, path.display());
}
