/* ════════════════════════════════════════════════════════════════════════
 * test_artifacts_consistent.c
 *
 * The build gate for the generated ability artifacts.
 *
 * WHY THIS SUITE IS SEPARATE FROM test_decode_skew.c
 * --------------------------------------------------
 * test_decode_skew.c is a deep audit: it walks every decoded ability, every
 * card-ability pair and every named key, and prints a long report. It is the
 * right place to investigate a skew, but it is a large suite that moves for
 * many unrelated reasons, so it is a poor thing to hang "are the generated
 * tables even self-consistent?" on.
 *
 * This suite answers exactly one question, in a few milliseconds:
 *
 *     do the COMMITTED offsets in gen_data.c still describe the string table
 *     that is actually on disk?
 *
 * That question was answered "no" twice. On 2026-09-30 and again on
 * 2026-10-01, cards/build/abilities_strings.bin was regenerated while the
 * committed RBKA_STRINGS_OFFSETS were left behind. Because the interned pool is
 * UNSEPARATED, that is not an out-of-range crash -- it silently returns a byte
 * splice of two neighbouring entries ("cost_limit" -> "cost_limitc",
 * "left_side" -> "eft_side", "resource_type" -> "ent_orderchoi"), which
 * corrupted 494 of 985 abilities on the second occurrence.
 *
 * WHY A SUITE IS THE RIGHT GATE HERE
 * ---------------------------------
 *   * cards/build/abilities_strings.bin is now TRACKED in git, so drift shows
 *     up in `git status`. But a build from a stale working tree, or from a
 *     tarball, or after a partial checkout, still links.
 *   * rb_verify_generated_tables() is called by the engine at load time, but
 *     only in the paths that choose to call it.
 *   * `make t T=artifacts_consistent` and `make run-all-tests` both pick this
 *     file up automatically (the Makefile globs tests/test_*.c), so the check
 *     is in the path every agent already runs, with no Makefile edit.
 *
 * FIX when this fails: bash tools/regen_all.sh
 * ════════════════════════════════════════════════════════════════════════ */

#include "rabuka.h"
#include "gen_data.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

extern const uint32_t RBKA_STRINGS_OFFSETS[];
extern const uint32_t RBKA_STRINGS_TABLE_LEN;
extern const uint32_t RBKA_NUM_ABILITIES;
/* Defined in src/core/generated/bytecode_blob.c and gen_data.c. rabuka.h
 * declares only RBKA_CARD_ABILITY_PAIRS, so these two are declared here. */
extern const unsigned char RBKA_BYTECODE[];
extern const uint32_t RBKA_BYTECODE_LEN;
extern const uint16_t RBKA_OFFSET_DELTAS[];
/* Emitted by tools/gen_from_rs.py. Not a macro, so it is absent from a
 * gen_data.c produced before this check existed -- hence the weak fallback
 * test below, which only asserts the value is not the unset sentinel. */
extern const uint32_t RBKA_STRINGS_TABLE_FNV;

extern int rb_verify_generated_tables(int *out_bad_strings, int *out_bad_keys,
                                      char *msg, unsigned long msg_sz);

static int g_fail = 0, g_checks = 0;

static void ck(int cond, const char *what) {
    g_checks++;
    if (cond) return;
    g_fail++;
    printf("  FAIL  %s\n", what);
}

/* FNV-1a 32, resumable. MUST match fnv1a() in tools/gen_from_rs.py and
 * rb_fnv1a32_resume() in src/ability/vm.c. A third implementation of one hash
 * is already one too many, but it exists so this suite recomputes the digest
 * itself instead of trusting the engine's own report. */
static uint32_t fnv1a32_resume(uint32_t h, const unsigned char *p, size_t n) {
    for (size_t i = 0; i < n; i++) { h ^= (uint32_t)p[i]; h *= 0x01000193u; }
    return h;
}

int main(void) {
    /* Load the blobs the same way every suite does, so this test sees the
     * same bytes the engine would. */
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0 &&
        rb_load("cards/build") != 0) {
        printf("FATAL: rb_load failed; no card/ability data\n");
        return 2;
    }

    printf("=== generated-artifact consistency gate ===\n");
    printf("abilities=%u interned_strings=%u table_bytes=%u\n",
           (unsigned)RBKA_NUM_ABILITIES,
           (unsigned)(RBKA_NUM_STRING_OFFSETS - 1),
           (unsigned)RBKA_STRINGS_TABLE_LEN);

    uint32_t n = RBKA_NUM_STRING_OFFSETS - 1;

    /* ── 1. the load-time self-check is the primary gate ────────────────── */
    int bad_strings = 0, bad_keys = 0;
    char msg[4096];
    int rc = rb_verify_generated_tables(&bad_strings, &bad_keys, msg, sizeof msg);
    fputs(msg, stdout);
    ck(rc == 0,
       "rb_verify_generated_tables() reports a consistent table");
    ck(bad_strings == 0, "zero defective interned strings");
    ck(bad_keys == 0, "zero defective key positions");

    /* ── 2. offsets are well formed ─────────────────────────────────────── */
    ck(RBKA_STRINGS_OFFSETS[0] == 0, "RBKA_STRINGS_OFFSETS[0] == 0");
    int mono = 1;
    for (uint32_t i = 0; i < n; i++)
        if (RBKA_STRINGS_OFFSETS[i] > RBKA_STRINGS_OFFSETS[i + 1]) { mono = 0; break; }
    ck(mono, "RBKA_STRINGS_OFFSETS is monotonic");
    ck(RBKA_STRINGS_OFFSETS[n] == RBKA_STRINGS_TABLE_LEN,
       "offset sentinel == generator-recorded table length");

    /* ── 3. recompute the whole-table digest HERE, not from the engine ──── */
    /* The pool is unseparated, so the table is exactly the concatenation of
     * string 0 .. string n-1. Hashing that concatenation catches drift that
     * preserves every individual string length, which is precisely what a
     * mid-pool splice does. */
    int spans_ok = 1;
    uint32_t h = 0x811C9DC5u;
    size_t total = 0;
    for (uint32_t i = 0; i < n; i++) {
        uint32_t a = RBKA_STRINGS_OFFSETS[i], b = RBKA_STRINGS_OFFSETS[i + 1];
        if (b > RBKA_STRINGS_TABLE_LEN || a > b) { spans_ok = 0; break; }
        const char *s = rb_get_string(i);
        size_t want = (size_t)(b - a);
        if (!s || strlen(s) != want) { spans_ok = 0; break; }
        /* resume, do not restart: the digest is over the whole byte run */
        h = fnv1a32_resume(h, (const unsigned char *)s, want);
        total += want;
    }
    ck(spans_ok, "every interned string's length matches its offset span");
    if (spans_ok) {
        ck(total == (size_t)RBKA_STRINGS_TABLE_LEN,
           "the interned spans sum to the recorded table length");
        /* 0 is the "not recorded" sentinel for a pre-digest gen_data.c. */
        ck(RBKA_STRINGS_TABLE_FNV != 0u,
           "gen_data.c records a whole-table digest (re-run tools/regen_all.sh "
           "if this fails; a gen_data.c from before the digest existed is a "
           "one-time regeneration, not a skew)");
        ck(h == RBKA_STRINGS_TABLE_FNV,
           "the string table on disk hashes to the value committed in gen_data.c");
    }

    /* ── 4. the named keys that skew corrupted, checked directly ───────── */
    /* These are the exact strings that decoded as splices under skew. They are
     * checked by identity so that a regression names a card-relevant string
     * rather than an index. */
    static const char *KEYS[] = {
        "cost_limit", "left_side", "invalidate_ability", "resource_type",
        "placement_order", "group_names", "temporal", "card_property",
        "operator", "activated_this_turn", "cost_limit_operator",
        "modify_required_hearts", "position_compare", "max_repeats",
        "conditional", "choice",
    };
    int missing = 0;
    for (size_t i = 0; i < sizeof KEYS / sizeof KEYS[0]; i++) {
        int found = 0;
        for (uint32_t k = 0; k < n; k++) {
            const char *s = rb_get_string(k);
            if (s && strcmp(s, KEYS[i]) == 0) { found = 1; break; }
        }
        if (!found) {
            missing++;
            printf("  FAIL  key \"%s\" absent from the pool (spliced under skew)\n",
                   KEYS[i]);
        }
    }
    g_checks++;
    if (missing) g_fail++;

    /* ── 5. the bytecode blob must still be the size the deltas claim ──── */
    /* A mismatch here means bytecode_blob.c and gen_data.c came from different
     * generator runs even when the string pool happens to agree.
     * RBKA_BYTECODE / RBKA_BYTECODE_LEN / RBKA_OFFSET_DELTAS are declared by
     * rabuka.h / gen_data.h already. */
    {
        size_t sum = 0;
        for (uint32_t i = 0; i < RBKA_NUM_ABILITIES; i++) sum += RBKA_OFFSET_DELTAS[i];
        ck(RBKA_BYTECODE_LEN == sum,
           "bytecode_blob.c length == sum(RBKA_OFFSET_DELTAS)");
    }

    printf("\n=== %d checks, %d failed ===\n", g_checks, g_fail);
    if (g_fail) {
        printf("RESULT: THE GENERATED ARTIFACTS ARE INCONSISTENT.\n"
               "A skewed engine links silently -- the interned pool is\n"
               "unseparated, so drift returns byte splices, not errors.\n"
               "FIX: bash tools/regen_all.sh\n"
               "     then commit cards/build/abilities_strings.bin,\n"
               "     src/core/generated/gen_data.c and bytecode_blob.c\n"
               "     TOGETHER, in one commit.\n");
        return 1;
    }
    printf("RESULT: consistent. %u interned strings, %u bytes, all digests match.\n",
           (unsigned)n, (unsigned)RBKA_STRINGS_TABLE_LEN);
    return 0;
}
