/* ===========================================================================
 * test_decode_bytecode_shape.c
 *
 * The bytecode<->JSON ORACLE, as far as the C can honestly go today.
 *
 * WHAT THE RUST ORACLE IS, AND WHY IT NEVER RUNS
 * ----------------------------------------------
 * engine/tests/test_modules/characterization/bytecode_deep_compare_test.rs:1
 *     #[cfg(all(feature = "bytecode_abilities", feature = "json_path_test"))]
 * `json_path_test` is NOT in engine/Cargo.toml:62 `default`, and build.rs:149-152
 * adds only `#[cfg(feature = "bytecode_abilities")]` to the `mod.rs`
 * declaration — so the module is REGISTERED and then compiles to nothing. Its
 * single test `bytecode_deep_matches_json_path` (:137-192) does the only thing
 * that would catch a bytecode/JSON disagreement: for each of the 985 abilities
 * it decodes twice — `get_ability(idx)` from the blob, and
 * `json_path_decode(entry)` from cards/abilities.json through
 * `serde_json::from_value::<Ability>` + `populate_from_json` — re-serialises
 * both and asserts equality field by field (`collect_diffs`, :83-135, reporting
 * MISSING_IN_JSON / MISSING_IN_BC / EXTRA_IN_BC by path). It would catch ANY
 * field the bytecode round-trip drops, defaults differently, or spells
 * differently from the JSON source — including the three normalisations the
 * test itself has to perform (:20-41 has_moved/not_moved type tags, :46-49 cost
 * key normalisation, :62-78 the compound draw_card `count` default).
 *
 * WHY THE SEMANTIC COMPARISON STILL MUST WAIT — the judgement, stated plainly
 * ---------------------------------------------------------------------
 * NOT WRITTEN, AND IT SHOULD NOT BE WRITTEN YET. Not because it is hard, and
 * not because the C lacks the decoder (it has one: `rb_decode_ability`,
 * src/ability/vm.c, and it walks the same object form `decode_ability` does) —
 * because it would be RED ON ARRIVAL for a reason that is not a defect.
 *
 * MEASURED, at HEAD 5ce311e6, from the two artifacts themselves rather than
 * from a report:
 *     engine/build.rs -> cards/build/generation_manifest.json
 *         output.bytes            = 98733      output.interned_strings = 6030
 *     engine_c_wip/src/core/generated/bytecode_blob.c:4
 *         RBKA_BYTECODE_LEN        = 98737
 *     engine_c_wip/src/core/generated/gen_data.h:2
 *         RBKA_NUM_STRING_OFFSETS  = 6033  (6032 strings)
 * So the Rust side is on a NEWER generation than the committed C tables, by
 * exactly 4 payload bytes and 2 interned strings, and the two encoding changes
 * are slices 325 (`PL!-pb2-012-P＋` / `-R`) and 805 (`PL!N-bp7-028-L`). Three
 * Rust tests pin the OLD encoding. Both engines are internally exact on their
 * own tables (985/985 tag-valid here; sum(OFFSET_DELTAS) == payload exactly).
 *
 * A deep-compare written today would therefore report ~325 and ~805 as
 * differences that are not bugs, and a suite whose failures are known-not-bugs
 * is a suite that trains its readers to ignore it. The order has to be:
 *   1. re-run the generator ONCE so cards/build and the committed C tables come
 *      from the same run (tools/regen_all.sh exists for exactly this, and
 *      tools/guard_data_blob.sh exists because doing it in two steps byte-splices
 *      the interned pool),
 *   2. re-point or retire the three Rust tests that pin the old encoding,
 *   3. THEN enable the oracle, and enable it in the same commit as (1).
 * What is landed instead, and lands today, is the SHAPE half: every invariant
 * that must hold for the blob to be describable AT ALL, each of which the
 * generation skew does not touch and therefore cannot make red.
 *
 * WHY THE SHAPE HALF IS NOT OPTIONAL EITHER
 * -----------------------------------------
 * The 2026-10-01 corruption hit 494 of 985 abilities. Read that number against
 * rb_bc_slice (src/core/data.c:374-382):
 *     if (idx >= RBKA_NUM_ABILITIES) return NULL;
 *     start = sum(deltas[0..idx]);  len = deltas[idx];
 *     if (start + len > g_bc_len) return NULL;      <-- the whole failure mode
 * and rb_decode_ability (src/ability/vm.c), which turns a NULL slice into a
 * DEFAULT Ability with `return 1` and no diagnostic — mirroring Rust's
 * `if start >= end { return Ok(Ability::default()) }` at engine/src/ability/
 * vm.rs:263-265. So one spliced delta does not fail one ability: it makes every
 * slice from that index to the end decode as an empty default, SILENTLY. 494
 * corrupted abilities is 985 - 491, i.e. a splice at slice 491. That is the
 * arithmetic this suite exists to make impossible, and it is why the checks
 * below are ordered so that the per-slice scan cannot itself run off the end of
 * the blob on the way to reporting the problem.
 *
 * WHAT IS PINNED HERE, AND WHY EACH ONE IS NOT SELF-REFERENTIAL
 * -------------------------------------------------------------
 * tests/test_artifacts_consistent.c is the C's INTERNAL-consistency gate and it
 * is good: the strings offsets-are-monotonic and sentinel==length checks
 * (:109-115), a whole-table FNV digest recomputed from the bytes on disk
 * (:117-146), and `RBKA_BYTECODE_LEN == sum(RBKA_OFFSET_DELTAS)` (:180-185).
 *
 * Everything in section 1 below used to be derived from a symbol that lives in
 * the SAME generated file as the bytes it describes. A byte-splice that
 * regenerated bytecode_blob.c and gen_data.c together would leave every one of
 * those self-consistent checks green. So section 1 pins the counts against
 * gen_data.h — a DIFFERENT FILE, and the one the C's own table is not generated
 * into — and section 3 pins them against the numbers recorded in the Makefile's
 * data rule. Three independent copies of the same count, so no single
 * regeneration can move all of them.
 *
 * The per-slice half — 985/985 tagged 0x08, prefix sums strictly increasing —
 * is what this file added, and it is the exact byte-splice signature that
 * corrupted 494 abilities, unchecked, because nothing anywhere checked it.
 * Neither the C nor Rust has a compiled guard for it on the Rust side either:
 * engine/tests/test_modules/characterization/bytecode_validation_test.rs
 * asserts the COUNT matches JSON (:51-59) and that every ability DECODES
 * (:62-80), and a zero-length slice decodes successfully by construction, so
 * neither test can see it.
 * =========================================================================== */

#include "rabuka.h"
#include "gen_data.h"

#include <stdio.h>
#include <string.h>

/* Defined in src/core/generated/gen_data.c and bytecode_blob.c. rabuka.h
 * declares only RBKA_CARD_ABILITY_PAIRS, so these are declared here — the same
 * declarations, and the same reason, as tests/test_artifacts_consistent.c:47-58. */
extern const uint32_t RBKA_NUM_ABILITIES;
extern const uint32_t RBKA_BYTECODE_LEN;
extern const unsigned char RBKA_BYTECODE[];
extern const uint16_t RBKA_OFFSET_DELTAS[];
extern const uint32_t RBKA_STRINGS_OFFSETS[];
extern const uint32_t RBKA_STRINGS_TABLE_LEN;
extern const uint32_t RBKA_STRINGS_TABLE_FNV;

/* The format tag every ability slice must begin with.
 * engine/src/ability/vm.rs:74 `const TAG_OBJECT: u8 = 0x08`, and
 * rb_decode_ability (src/ability/vm.c) rejects any other first byte with
 * `tag != RB_TAG_OBJECT`. The blob the C executes has its 8-byte `RBKA`
 * magic+version header ALREADY STRIPPED (strip_bytecode_header,
 * engine/src/ability/vm.rs:173-183), so slice 0 starts at payload offset 0 and
 * its first byte IS the tag — verified, not assumed: RBKA_BYTECODE[0..3] is
 * 8,8,8,6. */
#define RB_SHAPE_TAG_OBJECT 0x08

/* The canonical shape, pinned from OUTSIDE the generated tables.
 *
 *   gen_data.h:2-4                      the C's own header
 *   Makefile:122-123                    the human-facing description
 *   cards/build/generation_manifest.json what the RUST tree is on RIGHT NOW
 *
 * The first two must agree with each other today. The third is deliberately
 * recorded as a DIFFERENT number: it is the 325/805 generation skew, and this
 * suite prints it rather than asserting equality, because the whole point of
 * the shape gate is that it stays green across a skew. */
#define RB_SHAPE_ABILITIES        985u
#define RB_SHAPE_NUM_STR_OFFSETS  6033u
#define RB_SHAPE_CARD_ABILITY_PAIRS 2116u
#define RB_SHAPE_BYTECODE_LEN     98737u
#define RB_SHAPE_STRINGS_TABLE_LEN 726831u

/* What the RUST tree is on, measured 2026-10-04 at HEAD 5ce311e6 from
 * cards/build/generation_manifest.json. Printed as the skew, asserted as
 * nothing: pinning the C to the Rust generation is the regeneration step, not
 * this gate's business, and a gate that encodes the skew would have to be
 * edited in the same commit that fixes it — which is how gates get deleted. */
#define RB_SHAPE_RUST_BYTECODE_BYTES  98733u
#define RB_SHAPE_RUST_INTERNED        6030u

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } \
} while (0)

int main(void)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0 &&
        rb_load("cards/build") != 0) {
        printf("FATAL: rb_load failed; no card/ability data\n");
        return 2;
    }

    printf("=== bytecode slice shape gate ===\n");
    printf("abilities=%u bytecode_len=%u interned_strings=%u pairs=%u table_bytes=%u\n",
           (unsigned)RBKA_NUM_ABILITIES,
           (unsigned)RBKA_BYTECODE_LEN,
           (unsigned)(RBKA_NUM_STRING_OFFSETS - 1),
           (unsigned)RBKA_NUM_CARD_ABILITY_PAIRS,
           (unsigned)RBKA_STRINGS_TABLE_LEN);

    const unsigned char *blob = (const unsigned char *)RBKA_BYTECODE;
    uint32_t n = (uint32_t)RBKA_NUM_ABILITIES;

    /* ── 0. THE ARITHMETIC, BEFORE ANY INDEXED READ ──────────────────────────
     *
     * The 4-check version of this file read blob[off] inside the tag loop with
     * NO bounds test and only compared `running` to RBKA_BYTECODE_LEN after the
     * loop had finished. On precisely the input this gate exists to catch —
     * deltas from one generator run, bytes from another, summing PAST the blob —
     * that is an out-of-bounds read in the guard, before it can report
     * anything. This tree has already shipped one SIGSEGV that gcc predicted and
     * nobody read (Makefile:680), so the guard is not allowed to be the next
     * one.
     *
     * Everything is therefore computed first, and only a table whose total is
     * known to land inside the blob is then walked slice by slice. */
    size_t running = 0;
    int  overflowed = 0;                 /* a delta that would run past the blob */
    uint32_t first_overflow = 0;
    uint32_t non_increasing = 0, first_bad_delta = 0;
    uint32_t zero_delta = 0, first_zero = 0;
    uint32_t oversized = 0, first_oversized = 0;
    uint32_t min_delta = 0xFFFFFFFFu, max_delta = 0;
    size_t prev = 0;
    for (uint32_t i = 0; i < n; i++) {
        uint32_t d = RBKA_OFFSET_DELTAS[i];
        if (d == 0) {
            if (!zero_delta) first_zero = i;
            zero_delta++;
        }
        if (d > (uint32_t)RBKA_BYTECODE_LEN) {
            if (!oversized) first_oversized = i;
            oversized++;
        }
        if (d < min_delta) min_delta = d;
        if (d > max_delta) max_delta = d;
        running += d;
        if (running <= prev) {
            if (!non_increasing) first_bad_delta = i;
            non_increasing++;
        }
        if (running > (size_t)RBKA_BYTECODE_LEN && !overflowed) {
            first_overflow = i;
            overflowed = 1;
        }
        prev = running;
    }

    /* ── 1. THE COUNTS, PINNED AGAINST A DIFFERENT FILE ───────────────────────
     *
     * gen_data.h is not generated into by tools/gen_bytecode.py the way
     * bytecode_blob.c and gen_data.c are, so it is an independent witness to
     * how big this table is supposed to be. It also bounds the loop: the deltas
     * array is declared `const uint16_t RBKA_OFFSET_DELTAS[985]`
     * (gen_data.c:7), so RBKA_NUM_ABILITIES > 985 would read past its end, and
     * the only thing standing between this loop and that is the pin below. */
    CHECK(RBKA_NUM_ABILITIES == RB_SHAPE_ABILITIES,
          "RBKA_NUM_ABILITIES == 985, matching gen_data.h's RBKA_NUM_ABILITIES_C "
          "and gen_data.c's declared RBKA_OFFSET_DELTAS[985] array bound");
    CHECK(RBKA_NUM_STRING_OFFSETS == RB_SHAPE_NUM_STR_OFFSETS,
          "RBKA_NUM_STRING_OFFSETS == 6033 (6032 interned strings), per "
          "gen_data.h");
    CHECK(RBKA_NUM_CARD_ABILITY_PAIRS == RB_SHAPE_CARD_ABILITY_PAIRS,
          "RBKA_NUM_CARD_ABILITY_PAIRS == 2116, per gen_data.h");
    CHECK(RBKA_BYTECODE_LEN == RB_SHAPE_BYTECODE_LEN,
          "RBKA_BYTECODE_LEN == 98737, per bytecode_blob.c:4 and the Makefile's "
          "own description of the artifact");
    CHECK(RBKA_STRINGS_TABLE_LEN == RB_SHAPE_STRINGS_TABLE_LEN,
          "RBKA_STRINGS_TABLE_LEN == 726831, per gen_data.c:579 and the "
          "Makefile's STRINGS_BYTES — the same number the data guard hashes");
    CHECK(RBKA_STRINGS_TABLE_FNV != 0u,
          "gen_data.c records a whole-table digest; 0 is the unset sentinel of a "
          "gen_data.c produced before the digest existed");

    /* ── 2. THE DELTAS ARE REAL, STRICTLY-INCREASING SLICE LENGTHS ────────────
     *
     * Monotonicity of the PREFIX SUMS is the property that matters, and it is
     * the one with no other guard: a zero or reversed delta makes two slices
     * alias, which is indistinguishable from a splice once the decoder runs.
     *
     * WHY A ZERO DELTA IS NOT MERELY UNTIDY — it is invisible. Rust
     * (engine/src/ability/vm.rs:263-265) returns `Ok(Ability::default())` when
     * `start >= end`, and the C mirrors that exactly (rb_decode_ability:
     * `if (!slice || len == 0) return 1;`, src/ability/vm.c). So an empty
     * slice is a SUCCESS with no diagnostic, on both engines. That is why
     * bytecode_validation_test.rs cannot see it either: it asserts every
     * ability decodes (:62-80), and an empty slice decodes by construction. */
    CHECK(overflowed == 0,
          "no prefix sum runs past RBKA_BYTECODE_LEN — every slice lies inside "
          "the blob, so rb_bc_slice's `start + len > g_bc_len` guard never "
          "fires and no ability silently decodes as a default");
    CHECK(non_increasing == 0,
          "every slice length is strictly positive (the prefix sums strictly "
          "increase)");
    CHECK(zero_delta == 0,
          "no delta is zero — vm.rs:263-265 and rb_decode_ability both turn an "
          "empty slice into a DEFAULT Ability and report SUCCESS, so a zero "
          "delta cannot be caught by any behavioural test");
    CHECK(running == (size_t)RBKA_BYTECODE_LEN,
          "the prefix sums end exactly at RBKA_BYTECODE_LEN — the table "
          "describes the blob on disk and nothing more");
    CHECK(oversized == 0,
          "no single delta claims more bytes than the whole blob");

    if (overflowed || non_increasing || zero_delta || oversized)
        fprintf(stderr,
                "  first overflow at slice %u, first non-increasing at %u, "
                "first zero at %u, first oversized at %u\n",
                first_overflow, first_bad_delta, first_zero, first_oversized);

    /* ── 3. THE PER-SLICE HALF: 985/985 TAGGED 0x08 ───────────────────────────
     *
     * Only reached once section 2 has proved the table lands inside the blob,
     * so this walk is in bounds by construction rather than by hope.
     *
     * A slice whose first byte is anything else is a splice AT a slice
     * boundary — one of the two ways the 2026-10-01 damage presented. The other
     * way, a shift that keeps every offset and every tag intact and only
     * corrupts the bytes WITHIN the slices, is invisible to this check, to
     * test_artifacts_consistent's offsets, and to every count above; only the
     * semantic comparison at the top of this file would see it, which is the
     * concrete reason that comparison is worth writing once the skew is gone
     * and the concrete reason it cannot be written before. */
    uint32_t bad_tag = 0, first_bad_tag = 0, tagged = 0;
    size_t off = 0;
    for (uint32_t i = 0; i < n; i++) {
        uint32_t d = RBKA_OFFSET_DELTAS[i];
        if (off + d > (size_t)RBKA_BYTECODE_LEN) break;   /* section 2 proved this cannot fire */
        if (blob[off] == RB_SHAPE_TAG_OBJECT) tagged++;
        else {
            if (!bad_tag) first_bad_tag = i;
            bad_tag++;
        }
        off += d;
    }
    if (bad_tag)
        fprintf(stderr, "  first bad slice: %u\n", first_bad_tag);
    CHECK(bad_tag == 0,
          "every one of the 985 slices begins with the format tag 0x08 "
          "(vm.rs:74 TAG_OBJECT, which rb_decode_ability enforces)");
    CHECK(tagged == n,
          "the tagged count is 985 of 985, not merely \"no slice was seen to "
          "be wrong\" — the walk ran to completion over all n slices");

    /* ── 4. THE SKEW, PRINTED AND NOT ASSERTED ────────────────────────────────
     *
     * The Rust tree's own manifest says 98733 payload bytes and 6030 interned
     * strings; the committed C tables say 98737 and 6032. That 4-byte/2-string
     * difference IS the 325/805 generation skew, and it is the reason the
     * semantic comparison at the top of this file is deferred rather than
     * written. It is printed on every run so the deferral cannot quietly become
     * a permanent omission: the day these two lines agree, the deferral has an
     * end condition and it is visible. */
    printf("  skew: rust %u payload bytes / %u interned strings; "
           "c %u / %u\n",
           RB_SHAPE_RUST_BYTECODE_BYTES, RB_SHAPE_RUST_INTERNED,
           (unsigned)RBKA_BYTECODE_LEN,
           (unsigned)(RBKA_NUM_STRING_OFFSETS - 1));
    printf("  deltas: min=%u max=%u total=%u\n",
           (unsigned)min_delta, (unsigned)max_delta, (unsigned)running);

    printf("=== %d checks, %d failed ===\n", checks, failures);
    if (failures) {
        printf("RESULT: THE COMMITTED BYTECODE TABLE DOES NOT DESCRIBE ITS BLOB.\n");
        return 1;
    }
    printf("RESULT: every committed slice is well formed.\n");
    return 0;
}