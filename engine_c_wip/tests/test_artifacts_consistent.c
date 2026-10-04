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

/* Result lines start at COLUMN 0 with the canonical `FAIL: ` prefix, not with
 * the indented `  FAIL  ` this suite used to print. tools/verdict_runner.c
 * only rewrites a prefix that begins the line, so the indented form was
 * invisible to every `^FAIL` count in the project: a build-gate failure in
 * this file would have shown up as a red suite but never as a failure line. A
 * gate whose failures nobody can count is how §6 below stayed invisible for two
 * generations. */
static void ck(int cond, const char *what) {
    g_checks++;
    if (cond) return;
    g_fail++;
    printf("FAIL: %s\n", what);
}

/* FNV-1a 32, resumable. MUST match fnv1a() in tools/gen_from_rs.py and
 * rb_fnv1a32_resume() in src/ability/vm.c. A third implementation of one hash
 * is already one too many, but it exists so this suite recomputes the digest
 * itself instead of trusting the engine's own report. */
static uint32_t fnv1a32_resume(uint32_t h, const unsigned char *p, size_t n) {
    for (size_t i = 0; i < n; i++) { h ^= (uint32_t)p[i]; h *= 0x01000193u; }
    return h;
}

/* ── the cross-generation readers (section 6) ──────────────────────────────
 * A whole-FILE digest, over bytes read straight off disk, not through the
 * interned pool. That distinction is the point: the pool is what a stale offset
 * table silently mis-slices, so hashing through it can only ever re-derive the
 * island. Hashing the file itself compares the two artefacts as bytes. */
static int fnv1a32_file(const char *path, uint32_t *out_h, unsigned long *out_n)
{
    FILE *f = fopen(path, "rb");
    if (!f) return 0;
    uint32_t h = 0x811C9DC5u;
    unsigned long total = 0;
    unsigned char buf[65536];
    size_t got;
    while ((got = fread(buf, 1, sizeof buf, f)) > 0) {
        h = fnv1a32_resume(h, buf, got);
        total += (unsigned long)got;
    }
    fclose(f);
    *out_h = h;
    *out_n = total;
    return 1;
}

/* generation_manifest.json, slurped once. Static and bounded rather than
 * heap- or stack-allocated: this tree already overflows a 1 MB stack on a
 * GameState local, and a manifest is a few KB of JSON that no honest run
 * approaches this cap. A truncated read is treated as no manifest at all. */
static char g_man[1 << 19];
static size_t g_man_len;

static int load_manifest(void)
{
    static const char *const PATHS[] = {
        "../cards/build/generation_manifest.json",
        "cards/build/generation_manifest.json",
        "../../cards/build/generation_manifest.json",
        NULL
    };
    if (g_man_len) return 1;
    for (const char *const *p = PATHS; *p; p++) {
        FILE *f = fopen(*p, "rb");
        if (!f) continue;
        size_t got = fread(g_man, 1, sizeof g_man - 1, f);
        fclose(f);
        g_man[got] = '\0';
        g_man_len = got;
        return 1;
    }
    return 0;
}

/* First unsigned integer following "key", searching only at or after `from`.
 * Callers pass the offset of "c_artifacts" so the search cannot wander into the
 * sibling `output` block, which carries its own interned_strings. Returns 0 for
 * "key absent or carries no number", and every caller in section 6 treats 0 as
 * a FAILURE -- a digest that was never recorded is not a digest that matched. */
static unsigned long long man_u64(size_t from, const char *key)
{
    char needle[128];
    int nl = snprintf(needle, sizeof needle, "\"%s\"", key);
    if (nl <= 0 || (size_t)nl >= sizeof needle) return 0;
    if (from >= g_man_len) return 0;
    const char *p = strstr(g_man + from, needle);
    if (!p) return 0;
    p += nl;
    while (*p && (*p < '0' || *p > '9')) p++;
    if (!*p) return 0;
    unsigned long long v = 0;
    while (*p >= '0' && *p <= '9') {
        v = v * 10u + (unsigned long long)(*p - '0');
        p++;
    }
    return v;
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
            printf("FAIL: key \"%s\" absent from the pool (spliced under skew)\n",
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

    /* ── 6. THE CROSS-GENERATION GATE ─────────────────────────────────────
     *
     * EVERYTHING ABOVE IS AN ISLAND, AND THAT IS THE WHOLE DEFECT.
     *
     * Sections 1-5 compare the compiled-in tables (gen_data.c) against the blob
     * `rb_load("src")` happened to open. On 2026-10-04 that pair was the STALE
     * generation -- src/abilities_strings.bin at 726831 bytes, matching a stale
     * gen_data.c at 726831 -- so all of it passed and this suite printed "all
     * digests match" while the tree carried two generations-old card data. The
     * pool is UNSEPARATED, so a stale offsets table against a fresh blob is a
     * silent byte splice and every internal check is happy with it.
     *
     * A self-consistency check can NEVER detect this class, because the two
     * halves of the check move together. So this section reaches outside the
     * island to the two things that are generated by the OTHER pipeline and
     * committed independently:
     *
     *   (a) cards/build/abilities_strings.bin -- the blob the C tables were
     *       MEANT to be generated from. Compared by LENGTH and by whole-file
     *       FNV-1a, the same digest gen_data.c records, so this detects the skew
     *       directly rather than by proxy.
     *   (b) cards/build/generation_manifest.json's `c_artifacts` block -- the
     *       cross-engine provenance record. Compared against the values actually
     *       compiled into this binary, so the record cannot rot unnoticed.
     *
     * "Not recorded" is a FAILURE in both cases, never a pass. An absent
     * `c_artifacts` block means nobody recorded this generation, and a
     * generation nobody recorded is exactly how the 2026-10-04 skew shipped.
     */
    {
        static const char *const BLOB[] = {
            "../cards/build/abilities_strings.bin",
            "cards/build/abilities_strings.bin",
            "../../cards/build/abilities_strings.bin",
            NULL
        };
        uint32_t canon_h = 0;
        unsigned long canon_n = 0;
        int have_canon = 0;
        for (const char *const *p = BLOB; *p; p++)
            if (fnv1a32_file(*p, &canon_h, &canon_n)) { have_canon = 1; break; }

        ck(have_canon,
           "the canonical cards/build/abilities_strings.bin is reachable from "
           "here; without it NO cross-generation check is possible");
        if (have_canon) {
            printf("canonical blob: %lu bytes, FNV-1a 32 = %u\n", canon_n,
                   (unsigned)canon_h);
            printf("committed C   : %u bytes, FNV-1a 32 = %u\n",
                   (unsigned)RBKA_STRINGS_TABLE_LEN, (unsigned)RBKA_STRINGS_TABLE_FNV);
            ck((unsigned long)RBKA_STRINGS_TABLE_LEN == canon_n,
               "RBKA_STRINGS_TABLE_LEN == the canonical blob's length "
               "(the committed offsets DO index the canonical blob)");
            ck(RBKA_STRINGS_TABLE_FNV == canon_h,
               "RBKA_STRINGS_TABLE_FNV == the canonical blob's whole-file digest");
        }

        ck(load_manifest(),
           "cards/build/generation_manifest.json is reachable from here");
        if (g_man_len) {
            const char *ca = strstr(g_man, "\"c_artifacts\"");
            ck(ca != NULL,
               "generation_manifest.json records a c_artifacts block for the C "
               "generation (absent == cannot cross-check, never == agrees; if it "
               "vanished, cards/build_lib.py dropped it on a rebuild)");
            if (ca) {
                size_t off = (size_t)(ca - g_man);
                unsigned long long r;
                char what[320];

                r = man_u64(off, "strings_table_len");
                snprintf(what, sizeof what,
                         "c_artifacts.strings_table_len %llu == compiled "
                         "RBKA_STRINGS_TABLE_LEN %u", r,
                         (unsigned)RBKA_STRINGS_TABLE_LEN);
                ck(r == (unsigned long long)RBKA_STRINGS_TABLE_LEN, what);

                r = man_u64(off, "bytecode_len");
                snprintf(what, sizeof what,
                         "c_artifacts.bytecode_len %llu == compiled "
                         "RBKA_BYTECODE_LEN %u", r, (unsigned)RBKA_BYTECODE_LEN);
                ck(r == (unsigned long long)RBKA_BYTECODE_LEN, what);

                r = man_u64(off, "num_string_offsets");
                snprintf(what, sizeof what,
                         "c_artifacts.num_string_offsets %llu == compiled "
                         "RBKA_NUM_STRING_OFFSETS %d", r, RBKA_NUM_STRING_OFFSETS);
                ck(r == (unsigned long long)RBKA_NUM_STRING_OFFSETS, what);

                r = man_u64(off, "num_abilities");
                snprintf(what, sizeof what,
                         "c_artifacts.num_abilities %llu == compiled "
                         "RBKA_NUM_ABILITIES %u", r, (unsigned)RBKA_NUM_ABILITIES);
                ck(r == (unsigned long long)RBKA_NUM_ABILITIES, what);

                r = man_u64(off, "strings_table_fnv1a32");
                snprintf(what, sizeof what,
                         "c_artifacts.strings_table_fnv1a32 %llu == compiled "
                         "RBKA_STRINGS_TABLE_FNV %u", r,
                         (unsigned)RBKA_STRINGS_TABLE_FNV);
                ck(r == (unsigned long long)RBKA_STRINGS_TABLE_FNV, what);

                /* The one-line statement of the whole skew, and the check that
                 * makes it a build gate rather than a report: the C generation
                 * and the canonical generation are the SAME generation. */
                r = man_u64(off, "strings_table_len");
                snprintf(what, sizeof what,
                         "c_artifacts.strings_table_len %llu == the canonical "
                         "blob's length %lu -- the C tables and cards/build are "
                         "from ONE regen_all.sh run", r, canon_n);
                ck(have_canon && r == (unsigned long long)canon_n, what);
            }
        }
    }

    printf("\n=== %d checks, %d failed ===\n", g_checks, g_fail);
    if (g_fail) {
        printf("RESULT: THE GENERATED ARTIFACTS ARE INCONSISTENT.\n"
               "A skewed engine links silently -- the interned pool is\n"
               "unseparated, so drift returns byte splices, not errors.\n"
               "If only the CROSS-GENERATION checks above failed, the C tables\n"
               "and the blob they index are each internally fine but come from\n"
               "different generator runs -- the 2026-10-04 state exactly.\n"
               "FIX: bash tools/regen_all.sh\n"
               "     then commit cards/build/abilities_strings.bin,\n"
               "     src/core/generated/gen_data.c and bytecode_blob.c\n"
               "     TOGETHER, in one commit, and update the c_artifacts\n"
               "     block in cards/build/generation_manifest.json to match.\n");
        return 1;
    }
    printf("RESULT: consistent. %u interned strings, %u bytes, all digests match.\n",
           (unsigned)n, (unsigned)RBKA_STRINGS_TABLE_LEN);
    return 0;
}
