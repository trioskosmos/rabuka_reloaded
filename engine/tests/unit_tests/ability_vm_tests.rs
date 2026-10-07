// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::ability::abilities_gen::*;
use rabuka_engine::ability::vm::*;


    /// `abilities_gen.rs` and `abilities.bin.z` are two generated artifacts that
    /// must agree byte-for-byte: the first carries the per-ability lengths, the
    /// second carries the bytes those lengths index into. `get_ability` turns
    /// them straight into a slice range, so a mismatch is an out-of-range slice
    /// panic in the middle of resolving a card rather than a decode error.
    ///
    /// `build.rs` only prints a `cargo:warning` when the two drift, and
    /// `strip_bytecode_header` asserts the blob's VERSION — so the hazard was
    /// known and covered on one axis while the length axis went unchecked. This
    /// is that check, at the point where it is cheap to report.
    ///
    /// Regenerate both with `python cards/compile_abilities.py`.
    #[test]
    fn offset_table_agrees_with_the_bytecode_blob() {
        let entries = OFFSET_DELTAS.len();
        assert_eq!(
            entries,
            NUM_ABILITIES,
            "OFFSET_DELTAS has {entries} entries but NUM_ABILITIES is {NUM_ABILITIES}; \
             `get_ability` indexes OFFSET_DELTAS[idx] for every idx < NUM_ABILITIES",
        );

        let blob = get_decompressed_bytecode();
        let described: usize = OFFSET_DELTAS.iter().map(|&d| usize::from(d)).sum();
        assert!(
            described > 0,
            "the offset table describes no bytes at all, so no ability could decode",
        );
        assert!(
            described <= blob.len(),
            "OFFSET_DELTAS describes {described} bytes but the blob holds {} — \
             abilities_gen.rs is out of sync with abilities.bin.z, and \
             `get_ability` would slice past the end of the blob",
            blob.len()
        );
    }

    /// The same invariant from the other side: every individual slice the table
    /// produces has to be in bounds, not merely the total. A single oversized
    /// delta in the middle of the table passes a total-only check when another
    /// entry is short, and that is precisely the ability whose resolution dies.
    #[test]
    fn every_offset_slice_is_within_the_blob() {
        let blob = get_decompressed_bytecode();
        let mut start = 0usize;
        for (idx, &delta) in OFFSET_DELTAS.iter().enumerate() {
            let end = start + usize::from(delta);
            assert!(
                end <= blob.len(),
                "ability {idx} claims bytes {start}..{end} but the blob is {} bytes",
                blob.len()
            );
            start = end;
        }
    }
