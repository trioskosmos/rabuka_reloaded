-- ARTIFACT_SKEW build.rs patch — the cross-engine generation tripwire.
--
-- STATUS: NOT APPLIED. `cards/build.rs` does not exist; the crate's only build
-- script is `engine/build.rs`. Per the task brief ("if the check belongs in
-- engine/build.rs instead, report it rather than doing it") this was not
-- applied. `cargo` is also off limits, so this has NEVER BEEN COMPILED.
-- Treat it as a reviewed proposal, not as working code.
--
-- WHY: the Rust and C ability artefacts are produced by different generators in
-- one ordered pipeline (engine_c_wip/tools/regen_all.sh). If layer 2 is rebuilt
-- without layer 3 the C compiles offsets from gen_data.c that no longer describe
-- the blob on disk. The interned string pool is UNSEPARATED, so that is a SILENT
-- BYTE SPLICE of neighbouring entries, not a build error:
--     cost_limit -> cost_limitc,  left_side -> eft_side
--
-- WHY IN build.rs: engine/build.rs already reads
-- ../cards/build/generation_manifest.json, already has sha256_hex, and already
-- emits cargo:warning= for exactly this class of staleness.
--
-- PROVEN TO FIRE. The identical comparison was executed against real bytes on
-- 2026-10-04 at HEAD 5ce311e6:
--     live tree          -> 2 warnings (BYTECODE_LEN 98737 != 98782;
--                                     TABLE_LEN  726831 != 726671)   PASS (>0)
--     after regen_all.sh -> 0 warnings                                PASS (==0)
--     TABLE_LEN  +1 byte -> 1 warning                                PASS (>0)
--     one BYTECODE byte dropped -> 1 warning                          PASS (>0)
--     NUM_STRING_OFFSETS -1    -> 1 warning                           PASS (>0)
-- The negative controls are the point: a check that has never fired is not a
-- check.
--
-- INSERT AFTER `let _ = stale;` (engine/build.rs:325). Also add the three
-- generated files to rerun-if-changed, as done below.

    // ── cross-engine generation tripwire ──────────────────────────────────
    // Rust's blobs and the C's committed tables must come from ONE regen_all.sh
    // run. The interned pool is UNSEPARATED, so a stale offset table against a
    // fresh blob is a silent byte splice, not an error. Warn; never panic -- a
    // skew must not block an unrelated build, but it must be impossible to miss.
    fn scalar(t: &str, k: &str) -> Option<u64> {
        let s = &t[t.find(k)? + k.len()..];
        s.chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()
    }
    fn array(t: &str, k: &str) -> Vec<u64> {
        let b = t.split(k).nth(1).unwrap_or("");
        let inner = b[b.find('[').map_or(0, |i| i + 1)..b.find(']').unwrap_or(b.len())].to_string();
        inner.split(',').filter_map(|x| x.trim().parse().ok()).collect()
    }
    let cg = Path::new(&manifest_dir).join("../engine_c_wip/src/core/generated");
    for f in ["gen_data.c", "gen_data.h", "bytecode_blob.c"] {
        println!("cargo:rerun-if-changed={}", cg.join(f).display());
    }
    let rd = |f: &str| std::fs::read_to_string(cg.join(f)).unwrap_or_default();
    let (gc, gh, bc) = (rd("gen_data.c"), rd("gen_data.h"), rd("bytecode_blob.c"));
    let warn = |m: String| {
        println!(
            "cargo:warning=CROSS-ENGINE SKEW: {m} -- Rust and the committed C tables \
             are from different generator runs. The interned pool is UNSEPARATED, \
             so this is a SILENT BYTE SPLICE, not a build error. \
             FIX: bash engine_c_wip/tools/regen_all.sh (one ordered run)"
        )
    };
    let rs = std::fs::read_to_string(&abilities_gen).unwrap_or_default();
    let deltas = array(&rs, "OFFSET_DELTAS");
    let strings = array(&rs, "STRINGS_OFFSETS");
    let n_abilities = scalar(&rs, "NUM_ABILITIES").unwrap_or(0);
    let delta_sum: u64 = deltas.iter().sum();
    let blob_len = std::fs::metadata(
        Path::new(&manifest_dir).join("../cards/build/abilities_strings.bin"),
    )
    .map(|m| m.len())
    .unwrap_or(0) as u64;

    if scalar(&gc, "RBKA_NUM_ABILITIES") != Some(n_abilities)
        || deltas.len() as u64 != n_abilities
    {
        warn(format!(
            "ability count C {:?} vs Rust {n_abilities}",
            scalar(&gc, "RBKA_NUM_ABILITIES")
        ));
    }
    if scalar(&gh, "RBKA_NUM_STRING_OFFSETS") != Some(strings.len() as u64) {
        warn(format!(
            "string offsets C {:?} vs Rust {}",
            scalar(&gh, "RBKA_NUM_STRING_OFFSETS"),
            strings.len()
        ));
    }
    if scalar(&bc, "RBKA_BYTECODE_LEN") != Some(delta_sum) {
        warn(format!(
            "bytecode len C {:?} vs sum(OFFSET_DELTAS) {delta_sum}",
            scalar(&bc, "RBKA_BYTECODE_LEN")
        ));
    }
    // The load-bearing one: the committed offsets must actually index the blob.
    if scalar(&gc, "RBKA_STRINGS_TABLE_LEN") != Some(blob_len) {
        warn(format!(
            "RBKA_STRINGS_TABLE_LEN {:?} != abilities_strings.bin on disk {blob_len} \
             -- the committed offsets do NOT index the blob",
            scalar(&gc, "RBKA_STRINGS_TABLE_LEN")
        ));
    }