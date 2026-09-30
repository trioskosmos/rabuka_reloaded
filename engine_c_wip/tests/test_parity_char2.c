/* CLAIM (tr_char2): this agent owns tests/test_parity_char2.c and NO other
 * file. Rust sources translated here (and only here):
 *   engine/tests/test_modules/characterization/ability_golden_test.rs
 *   engine/tests/test_modules/characterization/describe_parity_test.rs
 *   engine/tests/test_modules/characterization/logging_test.rs
 *   engine/tests/test_modules/characterization/performance_snapshot_audit_test.rs
 *   engine/tests/test_modules/characterization/bytecode_deep_compare_test.rs
 *   engine/tests/test_modules/characterization/action_coverage_test.rs
 *   engine/tests/test_modules/characterization/corpus_smoke_test.rs
 *   engine/tests/test_modules/characterization/size_budget_test.rs
 *   engine/tests/test_modules/characterization/unique_abilities_test.rs
 * No src/ file, no other tests/ file and no Makefile is touched by me.
 *
 * ==========================================================================
 * CHARACTERIZATION CLUSTER PORT.  Read this before trusting a green.
 * ==========================================================================
 *
 * PROCESS MODEL. Every case runs in its own fork()ed child (the card-copy pool
 * is process-global and monotonic, and a case that segfaults must not take the
 * rest of the run with it). Counters live in one MAP_SHARED|MAP_ANONYMOUS
 * mapping so a child killed on SIGSEGV still reports the assertions and
 * failures it had already reached; the child zeroes its LOCALS before running
 * so it never double-counts the parent's accumulated totals, and the PARENT
 * accumulates across children.
 *
 * VACUITY. Every Rust test in this cluster is a CORPUS-WIDE or
 * MULTI-SNAPSHOT claim, so "no violations" is worthless without a population.
 * Every test below therefore asserts the SIZE OF THE SET IT EXAMINED:
 *   - ability_golden / action_coverage: the ability and card censuses are
 *     asserted exactly, and the distinct-action census is asserted exactly.
 *   - describe_parity census: the node count walked is asserted exactly.
 *   - logging boundedness: the push count is asserted to exceed the cap, and
 *     the observed length is asserted EQUAL to the cap (not merely <=).
 *   - every performance_snapshot_audit case asserts n_lives and the per-live
 *     requirement/fill totals it inspected, so a lives[] that never populated
 *     cannot read as "all entries passed".
 *
 * HARNESS NOTES.
 *  - load_card_db() falls back to ../cards/build: rb_load("src") is the in-tree
 *    layout, but tools/isolated_build.sh stages the blobs beside the copied
 *    tree, so an isolated out-of-tree build finds the database only there.
 *  - test_game_new() is called in every case: without it state.winner stays 0,
 *    which rb_advance_phase reads as "the game is already decided".
 *  - Every TestGame is `static`: sizeof(GameState) is ~781 KB.
 *  - test_card_copy_pool_reset() per case; the pool is global and monotonic.
 *  - Card identity is pinned with test_id_exact, never the forgiving
 *    test_id: rb_find_card_by_no falls back to a SUBSTRING search, so a
 *    mistyped print resolves to some other real card and the assertion becomes
 *    vacuously true about the wrong card.
 *  - test_get_heart_modifier() is a plain passthrough; heart reads go through
 *    rb_mods_get_heart with rb_parse_heart_color-derived indices, printed at
 *    startup from the engine's own parser.
 *  - Need-heart requirements are modified with rb_mods_add_need_heart /
 *    rb_mods_set_need_heart. stats_pipeline.c applies `set` first (only when
 *    non-zero) and then `add`, which is exactly Rust's ModifierEntry
 *    { set, additive } order in sp_effective_need_heart.
 *
 * WHAT IS NOT PORTABLE, AND WHY (each Rust file, precisely):
 *
 *  describe_parity_test.rs -- NOT PORTABLE. Its assertion is
 *      describe_effect_en(node) != node.text  and  describe_effect_ja(node)
 *      != node.text
 *  for every effect node carrying text. The C engine has NO describe layer at
 *  all: there is no describe_effect_en/describe_effect_ja anywhere in
 *  include/ or src/ (grep for "describe" in the C tree returns only two
 *  unrelated prose comments in condition.c and live.c). The fallback arm the
 *  test forbids therefore cannot be written and must not be faked. What IS
 *  portable -- and is what section B does -- is the POPULATION the Rust test
 *  walks: the same recursive node walk, counted and asserted, so whoever adds
 *  the describe templates inherits a pinned census and a pinned node count.
 *  Note also that the Rust `set` here has NO equivalent negative: a C test that
 *  only checked "every node has some non-empty rendering" would pass
 *  vacuously against the raw text, which is exactly the bug the Rust file
 *  exists to catch.
 *
 *  logging_test.rs -- 1 of 7 arms PORTED, 6 NOT PORTABLE. The C engine has no
 *  `state.structured_log`, no `rule_log`, no `LogCategory`, no `LogMetadata`
 *  and no push_choice_offered / push_choice_resolved / push_ability_resolution
 *  / push_trigger_evaluation emitters (grep for those symbols in the C tree
 *  returns nothing). The bounded-buffer arm maps onto the ONE bounded log the
 *  C port actually has: the ability VERDICT ring in src/ability/log.c, a fixed
 *  RB_LOG_BUF_MAX = 32 array. Section C ports the bound strictly. The other six
 *  arms -- resolved_ability_leaves_no_pending, choice_metadata_presence,
 *  choice_metadata_options_is_array, shared_log_never_leaks_private_zone_
 *  card_names, real_choice_flow_emits_choice_metadata,
 *  choice_resolved_metadata_is_trimmed, self_or_opponent_counts_two_options
 *  -- have no host object in C and are reported, not faked. See section C for
 *  the one further behavioural divergence found (retention direction).
 *
 *  bytecode_deep_compare_test.rs -- NOT PORTABLE as written. It needs
 *  serde_json plus the serde JSON-path decode system
 *  (#[cfg(all(feature = "bytecode_abilities", feature = "json_path_test"))]),
 *  builds a Rust `Ability` from cards/abilities.json, runs
 *  normalize_moved_types / normalize_cost_keys / populate_from_json, then
 *  deep-compares serialized forms field by field. The C engine has NO JSON
 *  parser and no serde path at all, so the second half of the comparison has
 *  no oracle. Its FIRST assertion -- ability_count() == json unique_abilities
 *  length -- IS expressible and is ported in section A. The string-fidelity
 *  concern the file is really about IS covered elsewhere: section E calls
 *  rb_verify_generated_tables() (src/ability/vm.c), the load-time per-string
 *  FNV-1a digest self-check over the compiled (chunk,start,len) offsets
 *  against the regenerated string table, and tests/test_decode_skew.c drives
 *  the corpus-wide plausibility walk over it.
 *
 *  corpus_smoke_test.rs -- ALREADY COVERED, not duplicated here. See section
 *  G note in the run summary.
 *  size_budget_test.rs -- ALREADY COVERED, not duplicated here. See section H.
 *  unique_abilities_test.rs -- 1 byte on disk, zero #[test], confirmed and
 *  skipped.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include "gen_data.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* ==========================================================================
 * shared counters + fork harness
 * ========================================================================== */
typedef struct {
    volatile int assertions;
    volatile int failures;
    volatile int signal;      /* fatal signal number, 0 if none  */
    volatile int exited_ok;   /* child reached the end of the case */
    char          name[96];   /* case in flight                  */
} SharedCounters;

static SharedCounters *g_shared;
static int g_assertions;      /* child-local; parent accumulates  */
static int g_failures;        /* child-local; parent accumulates  */
static int g_gaps;            /* harness gaps reported            */
static char g_case_name[96];

static void publish_counts(void)
{
    if (g_shared) {
        g_shared->assertions = g_assertions;
        g_shared->failures = g_failures;
    }
}

/* snprintf is not async-signal-safe, so the in-flight case name is copied by
 * hand. A child killed mid-assertion still names itself. */
static void fatal_handler(int sig)
{
    if (g_shared) {
        g_shared->signal = sig;
        g_shared->assertions = g_assertions;
        g_shared->failures = g_failures;
        size_t i = 0;
        for (; i + 1 < sizeof g_shared->name && g_case_name[i]; i++)
            g_shared->name[i] = g_case_name[i];
        g_shared->name[i] = 0;
    }
    _exit(0);
}

/* VARIADIC on purpose: a two-parameter CHECK(cond, "msg") silently splits an
 * adjacent string literal into extra macro arguments. */
#define CHECK(cond, ...) do {                                              \
    g_assertions++;                                                        \
    char _m[600];                                                          \
    snprintf(_m, sizeof _m, __VA_ARGS__);                                  \
    if (!(cond)) {                                                         \
        fprintf(stderr, "FAIL: %s\n", _m);                                 \
        g_failures++; publish_counts();                                    \
    } else {                                                               \
        printf("ok: %s\n", _m);                                            \
    }                                                                      \
} while (0)

#define GAP(...) do {                                                      \
    g_gaps++;                                                              \
    char _m[600];                                                          \
    snprintf(_m, sizeof _m, __VA_ARGS__);                                  \
    printf("HARNESS GAP: %s\n", _m);                                      \
    fflush(stdout);                                                        \
} while (0)

/* ==========================================================================
 * helpers
 * ========================================================================== */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rotating ring: one static buffer reused 2-3 times in a printf prints the
 * LAST card every time. */
static const char *no_of(int card_id)
{
    static char bufs[6][80];
    static int  next = 0;
    char *b = bufs[next];
    next = (next + 1) % 6;
    Card c;
    if (rb_card_get_card_by_id(card_id, &c) == 0 && c.card_no_idx)
        snprintf(b, 80, "%s", rb_card_string(c.card_no_idx));
    else
        snprintf(b, 80, "id%d", card_id);
    return b;
}

static void dump8(const char *label, const int *v)
{
    fprintf(stderr, "  %s = [%d,%d,%d,%d,%d,%d,%d,%d]\n", label,
            v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
    fflush(stderr);
}

static void clear_bag(RbBag *bag) { bag->n = 0; }

/* performance_snapshot_audit_test.rs:12 setup_game */
static void setup_game_with_decks(TestGame *tg, const int p1_stage[3], int filler)
{
    clear_bag(&tg->state.p[0].deck);
    clear_bag(&tg->state.p[0].hand);
    clear_bag(&tg->state.p[0].discard);
    clear_bag(&tg->state.p[0].success);
    clear_bag(&tg->state.p[0].energy_deck);
    clear_bag(&tg->state.p[1].deck);
    clear_bag(&tg->state.p[1].hand);
    clear_bag(&tg->state.p[1].discard);
    clear_bag(&tg->state.p[1].success);
    clear_bag(&tg->state.p[1].energy_deck);
    for (int i = 0; i < 40; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[0].stage[i] = p1_stage[i];
    tg->state.p[1].stage[0] = -1;
    tg->state.p[1].stage[1] = filler;
    tg->state.p[1].stage[2] = -1;
}

/* performance_snapshot_audit_test.rs:6 advance_to_live_card_set_p1 */
static void advance_to_live_card_set_p1(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust game.set_live_card(lc): take the card out of `pl`'s hand and APPEND it
 * to `pl`'s live zone. */
static int set_live_card_for(TestGame *tg, int pl, int card_id)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] != card_id) continue;
        int card = rb_hand_remove_card(P, i);
        return card >= 0 && rb_live_add_card(P, card) == 0;
    }
    return 0;
}

/* run_to_end. The C port runs the whole yell/allocation/verdict/victory
 * window inside RB_PHASE_PERFORMANCE, so this is one loop of passes instead
 * of Rust's two explicit advance_to_phase calls. The prompt contract is kept
 * strict: SelectCard is skipped, SelectAutoAbility answers "proceed", and
 * anything else is a hard failure rather than a blind skip. */
static int run_full_turn(TestGame *tg)
{
    for (int i = 0; i < 40; i++) {
        if (tg->state.phase == RB_PHASE_ACTIVE && !test_has_pending_choice(tg))
            return 1;
        const char *choice = test_pending_choice_type(tg);
        if (choice[0] == 0) {
            test_pass(tg);
        } else if (strcmp(choice, "SelectCard") == 0) {
            test_resume_choice(tg, -1);
        } else if (strcmp(choice, "SelectAutoAbility") == 0) {
            test_resume_choice(tg, 0);
        } else {
            fprintf(stderr, "FAIL: unexpected prompt during run_to_end "
                            "(expected SelectCard arrange or none), got %s\n", choice);
            g_failures++;
            publish_counts();
            return 0;
        }
    }
    fprintf(stderr, "FAIL: run_to_end did not return to Active\n");
    g_failures++;
    publish_counts();
    return 0;
}

/* The LAST snapshot recorded for `pl` (Rust uses .find(), i.e. the first;
 * with one performance window per seat in these fixtures the two agree --
 * asserted by n_snapshots below where it matters). */
static RbLiveSnapshot *snapshot_for(TestGame *tg, int pl)
{
    RbLiveSnapshot *found = NULL;
    for (int i = 0; i < tg->state.n_snapshots; i++)
        if (tg->state.snapshots[i].player == pl)
            found = &tg->state.snapshots[i];
    return found;
}

#define LIVE_DASH  "PL!-sd1-019-SD"  /* START:DASH!! needs h01=1 h03=1 h06=1 */
#define LIVE_ANY4  "PL!N-sd1-025-SD"  /* needs any 4 hearts                    */
#define MEMBER     "PL!-sd1-001-SD"  /* honoka, bh h01=1 h03=2 h06=1 blade=3  */
#define FILLER     "PL!-sd1-010-SD"  /* bh h01=1 h03=1, blade=1                */

/* ==========================================================================
 * #A  ability_golden_test.rs -- abilities.json shape guard
 *
 * ability_golden_test.rs:7-13
 *   let val = serde_json::from_slice(&bytes).expect("... parses");
 *   let obj = val.as_object().expect("abilities.json is object");
 *   assert_eq!(obj.len(), 10, ...);
 *
 * There is no serde_json in the C engine, so the test file carries a minimal
 * depth-tracking JSON scanner. It is deliberately STRICT: a naive colon count
 * would have produced 10 by luck on a file that is not an object at all, so
 * the scanner tracks {}/[] depth and string state and only counts a key that
 * opens at depth 1 inside the root object.
 *
 * The 10th key is `_warning` (verified against cards/abilities.json read as
 * UTF-8, not via Get-Content -Raw which mangles non-ASCII). The file was
 * regenerated 2026-09-30 with 985 unique_abilities, which is why the derived
 * count is pinned at 985 and NOT at the 936 quoted in the Rust comment --
 * that comment is stale, see the run summary.
 * ========================================================================== */
static char *read_whole_file(const char *path, size_t *out_len)
{
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long n = ftell(f);
    if (n <= 0) { fclose(f); return NULL; }
    rewind(f);
    char *buf = (char *)malloc((size_t)n + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t got = fread(buf, 1, (size_t)n, f);
    fclose(f);
    buf[got] = '\0';
    if (out_len) *out_len = got;
    return buf;
}

/* Depth-tracking scan. Returns the number of top-level keys, or -1 if the
 * document is not a single JSON object.
 *
 * A string at depth 1 counts as a KEY only if the first non-whitespace byte
 * after its closing quote is ':'. Without that lookahead the scanner also
 * counts top-level VALUE strings ("schema": "..." contributes the schema
 * string as well as the key), which inflates the tally -- it reported 17 for
 * a document that has 10 top-level keys.
 * Records the key NAMES so the test can prove the scanner resolved names
 * rather than counting colons. */
static int scan_top_level_keys(const char *text, char names[][32], int max_names)
{
    const char *p = text;
    int depth = 0, n_keys = 0;
    int in_str = 0, esc = 0, at_depth1_str = 0;
    const char *key_start = NULL;
    int first = 1;

    while (*p) {
        char c = *p;
        if (in_str) {
            if (esc) { esc = 0; }
            else if (c == '\\') { esc = 1; }
            else if (c == '"') {
                in_str = 0;
                if (at_depth1_str) {
                    const char *q = p + 1;
                    while (*q == ' ' || *q == '\t' || *q == '\n' || *q == '\r') q++;
                    if (*q == ':') {
                        if (n_keys < max_names && key_start) {
                            size_t len = (size_t)(p - key_start);
                            if (len > 31) len = 31;
                            memcpy(names[n_keys], key_start, len);
                            names[n_keys][len] = '\0';
                        }
                        n_keys++;
                    }
                }
                at_depth1_str = 0;
            }
            p++;
            continue;
        }
        if (c == '"') {
            in_str = 1; esc = 0;
            at_depth1_str = (depth == 1 && !first);
            if (at_depth1_str) key_start = p + 1;
            first = 0;
            p++;
            continue;
        }
        if (c == '{' || c == '[') {
            if (first) {
                if (c != '{') return -1;   /* root must be an object */
                first = 0;
            }
            depth++; p++; continue;
        }
        if (c == '}' || c == ']') { depth--; p++; continue; }
        first = 0;
        p++;
    }
    if (depth != 0) return -1;
    return n_keys;
}

/* The isolated build (tools/isolated_build.sh) copies only cards.bin and
 * abilities_strings.bin into $ROOT/cards/build; cards/abilities.json is NOT
 * staged, so under `bash tools/isolated_build.sh` this lookup misses and
 * section A reports a HARNESS GAP while the engine-side census still runs.
 * In the REAL harness (`make t T=parity_char2` and run-all-tests, cwd
 * engine_c_wip) ../cards/abilities.json resolves and every assertion below is
 * live. See the run summary for the one-line script fix. */
static const char *find_abilities_json(char *resolved, size_t resolved_sz)
{
    const char *env = getenv("RABUKA_ABILITIES_JSON");
    if (env && *env) {
        FILE *f = fopen(env, "rb");
        if (f) { fclose(f); snprintf(resolved, resolved_sz, "%s", env); return resolved; }
    }
    static const char *const cands[] = {
        "../cards/abilities.json",
        "cards/abilities.json",
        "../../cards/abilities.json",
        "./cards/abilities.json",
        "../../../cards/abilities.json",
        NULL
    };
    for (int i = 0; cands[i]; i++) {
        FILE *f = fopen(cands[i], "rb");
        if (f) { fclose(f); snprintf(resolved, resolved_sz, "%s", cands[i]); return resolved; }
    }
    /* Last resort: walk up from the cwd looking for the walker output. */
    char path[512];
    if (getcwd(path, sizeof path)) {
        for (int up = 0; up < 8; up++) {
            char cand[600];
            snprintf(cand, sizeof cand, "%s/cards/abilities.json", path);
            FILE *f = fopen(cand, "rb");
            if (f) { fclose(f); snprintf(resolved, resolved_sz, "%s", cand); return resolved; }
            char *slash = strrchr(path, '/');
            if (!slash || slash == path) break;
            *slash = '\0';
        }
    }
    return NULL;
}

static void test_a1_abilities_json_shape(void)
{
    static TestGame tg;   /* unused here but keeps the static-locals idiom */
    (void)tg;

    char path[1024];
    const char *p = find_abilities_json(path, sizeof path);
    size_t len = 0;
    char *text = p ? read_whole_file(p, &len) : NULL;

    if (!text) {
        GAP("#A abilities.json unreachable (searched $RABUKA_ABILITIES_JSON, "
            "six relative candidates and eight parents of the cwd). The "
            "isolated build does not stage it; add "
            "`cp -f \"$REPO/cards/abilities.json\" \"$ROOT/cards/\"` to "
            "tools/isolated_build.sh. The engine-side census in this same "
            "test STILL runs below, so the section is not vacuous.");
    } else {
        CHECK(len > 1500000,
              "#A abilities.json was read whole (got %zu bytes; the walker "
              "output is ~1.82 MB, so a short read would silently under-count "
              "keys)", len);

        char names[16][32];
        int n = scan_top_level_keys(text, names, 16);
        CHECK(n >= 0,
              "#A abilities.json is a single balanced JSON object (scanner "
              "returned %d; -1 means the root was not '{' or depth did not "
              "return to 0)", n);
        if (n >= 0) {
            CHECK(n == 10,
                  "#A abilities.json top-level key count == 10 (got %d: the "
                  "walker output shape changed -- re-golden deliberately)",
                  n);
            /* Anti-vacuity: prove the scanner resolved NAMES, not colons. */
            int saw_schema = 0, saw_unique = 0, saw_stats = 0, saw_warning = 0;
            for (int i = 0; i < n && i < 16; i++) {
                if (strcmp(names[i], "schema") == 0) saw_schema = 1;
                if (strcmp(names[i], "unique_abilities") == 0) saw_unique = 1;
                if (strcmp(names[i], "statistics") == 0) saw_stats = 1;
                if (strcmp(names[i], "_warning") == 0) saw_warning = 1;
            }
            CHECK(saw_schema && saw_unique && saw_stats && saw_warning,
                  "#A the 10 keys resolved by NAME include schema, statistics, "
                  "unique_abilities and _warning (so the count is real, not a "
                  "colon tally)");
            /* The walker element census: one "full_text" per unique ability. */
            int abilities = 0;
            const char *q = text;
            while ((q = strstr(q, "\"full_text\":")) != NULL) { abilities++; q += 12; }
            CHECK(abilities == 985,
                  "#A cards/abilities.json holds 985 unique_abilities entries "
                  "(counted %d; \"full_text\" appears once per entry)",
                  abilities);
            CHECK((uint32_t)abilities == rb_num_abilities(),
                  "#A the engine's rb_num_abilities() (%u) equals the "
                  "walker's unique_abilities count (%d) -- the compiled "
                  "tables and the walker output are the same corpus",
                  rb_num_abilities(), abilities);
        }
        free(text);
    }

    /* Engine-side census, always available. This is the C half of
     * bytecode_deep_compare_test.rs:140 assert_eq!(ability_count(),
     * json_abilities.len()) with the JSON side substituted by the golden
     * 985 measured from cards/abilities.json statistics.unique_abilities. */
    CHECK(rb_num_cards() == 2639,
          "#A the compiled card database holds 2639 records "
          "(rb_num_cards() == %u)", rb_num_cards());
    CHECK(rb_num_abilities() == 985,
          "#A the compiled ability table holds 985 entries "
          "(rb_num_abilities() == %u)", rb_num_abilities());

    /* Every ability index decodes -- the loadable half of the deep compare. */
    uint32_t na = rb_num_abilities();
    int decoded = 0;
    for (uint32_t i = 0; i < na; i++) {
        Ability a;
        if (rb_decode_ability(i, &a)) { decoded++; rb_free_ability(&a); }
    }
    CHECK(decoded == (int)na,
          "#A all %u compiled abilities decode (decoded %d) -- zero failures",
          na, decoded);
}

/* ==========================================================================
 * #B  describe_parity_test.rs -- NOT PORTABLE; census only
 *
 * The Rust assertion needs describe_effect_en / describe_effect_ja, neither of
 * which exists in the C engine. This section walks the SAME node population
 * the Rust test walks and pins its size, so a future describe implementation
 * inherits both the census and the node count. The walker is a superset of
 * Rust's collect_nodes: it also descends options[], gained_effect,
 * resource_on_select, opponent_action and alternative_effect.
 * ========================================================================== */
typedef struct {
    long nodes;
    long nodes_with_text;
} NodeCensus;

static void census_walk_effect(const AbilityEffect *e, NodeCensus *out, int depth)
{
    if (!e || depth > 24) return;
    out->nodes++;
    if (e->text && *e->text) out->nodes_with_text++;
    census_walk_effect(e->primary_effect, out, depth + 1);
    census_walk_effect(e->alternative_effect, out, depth + 1);
    census_walk_effect(e->look_action, out, depth + 1);
    census_walk_effect(e->select_action, out, depth + 1);
    census_walk_effect(e->followup_action, out, depth + 1);
    census_walk_effect(e->optional_action, out, depth + 1);
    census_walk_effect(e->conditional_action, out, depth + 1);
    census_walk_effect(e->gained_effect, out, depth + 1);
    census_walk_effect(e->resource_on_select, out, depth + 1);
    census_walk_effect(e->opponent_action, out, depth + 1);
    for (int i = 0; i < e->n_child; i++) census_walk_effect(e->child[i], out, depth + 1);
    for (int i = 0; i < e->n_options; i++) census_walk_effect(e->options[i], out, depth + 1);
}

static void test_b1_describe_node_census(void)
{
    uint32_t na = rb_num_abilities();
    NodeCensus c; c.nodes = 0; c.nodes_with_text = 0;
    int abilities_with_effect = 0;

    for (uint32_t i = 0; i < na; i++) {
        Ability a;
        if (!rb_decode_ability(i, &a)) continue;
        if (a.effect) { abilities_with_effect++; census_walk_effect(a.effect, &c, 0); }
        rb_free_ability(&a);
    }

    printf("info: #B describe-census: %d/%u abilities carry an effect tree; "
           "%ld effect nodes, %ld with text\n",
           abilities_with_effect, na, c.nodes, c.nodes_with_text);
    fflush(stdout);

    /* Rust SKIPS the effect-less abilities, not fails on them:
     *   let Some(ref effect) = ability.effect else { continue; };
     * so the pinned population is 983, not 985. */
    CHECK(abilities_with_effect == 983,
          "#B 983 of the 985 compiled abilities carry an effect tree (got %d). "
          "The 2 effect-less ones are skipped by Rust's `let Some(ref effect) "
          "= ability.effect else { continue }` -- they are not a failure, so "
          "this pins the population instead of asserting 985",
          abilities_with_effect);
    CHECK(c.nodes == 1879,
          "#B the effect-node walk visits 1879 nodes (got %ld). This is the "
          "golden census of the population describe_parity_test.rs walks; a "
          "change here means the walker or the corpus changed -- re-golden "
          "deliberately", c.nodes);
    CHECK(c.nodes_with_text == 1875,
          "#B 1875 of the 1879 effect nodes carry text (got %ld). These are "
          "exactly the nodes describe_parity_test.rs requires a template for "
          "-- it skips any node whose `node.text` is empty", c.nodes_with_text);
    CHECK(c.nodes_with_text > 1000,
          "#B anti-vacuity floor: the text-bearing node count is large enough "
          "that the census cannot have walked an empty or truncated tree "
          "(%ld)", c.nodes_with_text);

    GAP("#B describe_parity_test.rs's real assertion (describe_effect_en/ja "
        "must not equal node.text) is NOT PORTABLE: the C engine has no "
        "describe layer. No describe_effect_en / describe_effect_ja exists in "
        "include/ or src/, so the fallback arm the Rust file forbids cannot "
        "be written. Writing a C stand-in that only checked `every node has "
        "some rendering` would pass vacuously against the raw text -- which is "
        "the exact bug the Rust file exists to catch.");
}

/* ==========================================================================
 * #C  logging_test.rs -- the ONE ported arm
 *
 * Rust buffers_are_bounded (logging_test.rs) asserts two bounded buffers cap
 * and drop their OLDEST entries. The C port has exactly one bounded log: the
 * ability verdict ring in src/ability/log.c, a fixed RB_LOG_BUF_MAX = 32
 * static array behind the rb_log_set_enabled gate.
 * ========================================================================== */
#define LOG_CAP 32   /* RB_LOG_BUF_MAX, src/ability/log.c */

static void test_c1_verdict_ring_is_bounded(void)
{
    rb_log_set_enabled(1);
    rb_log_clear_verdicts();

    /* Push far past the cap. */
    const int pushes = 400;
    for (int i = 0; i < pushes; i++) {
        char line[32];
        snprintf(line, sizeof line, "line_%d", i);
        rb_log_push_verdict(line, "hand_count", 1);
    }
    int len = rb_log_buffer_len();

    printf("info: #C verdict ring: %d pushes -> len %d (cap %d)\n",
           pushes, len, LOG_CAP);
    fflush(stdout);

    CHECK(len <= LOG_CAP,
          "#C the ability verdict ring never exceeds its cap: %d pushes gave "
          "len %d against a cap of %d", pushes, len, LOG_CAP);
    /* NOT "<= cap": the ring must have actually FILLED, otherwise a ring that
     * recorded nothing would satisfy the assertion above. */
    CHECK(len == LOG_CAP,
          "#C the ring really saturated -- len must EQUAL the cap %d, not "
          "merely be under it (len %d after %d pushes proves the bound was "
          "reached rather than never approached)", LOG_CAP, len, pushes);

    rb_log_clear_verdicts();
    CHECK(rb_log_buffer_len() == 0,
          "#C rb_log_clear_verdicts empties the ring (len %d)",
          rb_log_buffer_len());
    rb_log_set_enabled(0);
}

/* ==========================================================================
 * #D  performance_snapshot_audit_test.rs
 * ========================================================================== */

/* TEST 1: sufficient hearts -> card PASS */
static void test_d1_sufficient_hearts_passes(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int member = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "#D T1 fixtures resolve EXACTLY (live=%d member=%d filler=%d; "
          "test_id would have papered over a mistyped print)", live, member, filler);
    if (live < 0 || member < 0 || filler < 0) return;

    int stage[3] = { -1, member, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T1 the live card enters P1's live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T1 P1 has a performance snapshot (n_snapshots=%d)",
          tg.state.n_snapshots);
    if (!s) return;

    /* Non-vacuity: the lives entry this case inspects must EXIST and carry a
     * real requirement, otherwise "passed" reads true over nothing. */
    CHECK(s->n_lives == 1, "#D T1 the snapshot holds exactly one lives entry (got %d)", s->n_lives);
    if (s->n_lives < 1) return;
    int req_total = 0, filled_total = 0;
    for (int c = 0; c < 8; c++) { req_total += s->live_required[0][c]; filled_total += s->live_filled[0][c]; }
    dump8("T1 total_hearts", s->total_hearts);
    dump8("T1 live[0].required", s->live_required[0]);
    dump8("T1 live[0].filled", s->live_filled[0]);
    CHECK(req_total >= 3,
          "#D T1 live[0] carries a real requirement (required_total=%d; the "
          "live needs h01=1 h03=1 h06=1) -- a 0 here would make 'passed' "
          "vacuous", req_total);

    CHECK(s->live_passed[0] != 0,
          "#D T1 card PASSES: 7 hearts >= 3 needed (filled_total=%d)", filled_total);
    CHECK(s->total_score > 0, "#D T1 total_score > 0 on pass (got %d)", s->total_score);
    CHECK(s->success != 0, "#D T1 success on pass");

    GAP("#D T1 base_score_total / card_bonus_total are dumped, not asserted, "
        "in the Rust file and have no field on RbLiveSnapshot in C.");
}

/* TEST 2: two live cards, second fails -> Rule 8.3.16 */
static void test_d2_two_cards_second_fails(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live_a = test_id_exact(&tg, LIVE_ANY4);
    int live_b = test_new_id(&tg, LIVE_ANY4);
    int member = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live_a >= 0 && live_b >= 0 && member >= 0 && filler >= 0,
          "#D T2 fixtures resolve EXACTLY (a=%d b=%d member=%d filler=%d)",
          live_a, live_b, member, filler);
    if (live_a < 0 || live_b < 0 || member < 0 || filler < 0) return;
    CHECK(live_a != live_b,
          "#D T2 live_b is a DISTINCT card-copy instance, not the same id "
          "(a=%d b=%d); a shared id would make lives[] read one card twice",
          live_a, live_b);

    int stage[3] = { -1, member, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live_a);
    test_add_to_hand(&tg, live_b);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live_a), "#D T2 first live card enters the live zone");
    CHECK(set_live_card_for(&tg, 0, live_b), "#D T2 second live card enters the live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T2 P1 has a performance snapshot");
    if (!s) return;

    CHECK(s->n_lives == 2,
          "#D T2 the snapshot holds BOTH live cards (n_lives=%d; a 1 here would "
          "make lives[1] read out of bounds or as zero)", s->n_lives);
    if (s->n_lives < 2) return;

    for (int i = 0; i < 2; i++) {
        int req = 0, fill = 0;
        for (int c = 0; c < 8; c++) { req += s->live_required[i][c]; fill += s->live_filled[i][c]; }
        dump8("T2 live[i].required", s->live_required[i]);
        dump8("T2 live[i].filled", s->live_filled[i]);
        printf("info: #D T2 live[%d]: passed=%d required_total=%d filled_total=%d\n",
               i, s->live_passed[i], req, fill);
    }
    CHECK(!s->live_passed[1],
          "#D T2 second card FAILS: only 3 hearts left, needs 4");
    CHECK(!s->success,
          "#D T2 success=false when ANY card fails (Rule 8.3.16)");
}

/* TEST 3: allocations match filled -- NOT EXPRESSIBLE as written.
 * RbLiveSnapshot has live_filled but NO breakdown.allocations ledger, so the
 * Rust cross-check (fold the per-colour allocations for target 0 and compare
 * to lc.filled) has nothing to fold. The C-expressible restatement is the
 * conservation law the ledger exists to establish: the greedy allocator
 * mutates a SHARED pool, so no heart may be allocated twice. */
static void test_d3_allocation_conservation(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_ANY4);
    int member = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "#D T3 fixtures resolve EXACTLY (live=%d member=%d filler=%d)",
          live, member, filler);
    if (live < 0 || member < 0 || filler < 0) return;

    int stage[3] = { -1, member, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);
    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T3 the live card enters P1's live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T3 P1 has a performance snapshot");
    if (!s) return;
    CHECK(s->n_lives == 1, "#D T3 the snapshot holds one lives entry (got %d)", s->n_lives);
    if (s->n_lives < 1) return;

    /* Conservation: per colour, the sum over live cards of live_filled must
     * not exceed the pool, and no single card may be filled beyond the pool.
     * A double-allocated heart shows up here as a sum above total_hearts. */
    int sum_ok = 1, card_ok = 1;
    for (int c = 0; c < 8; c++) {
        int sum = 0;
        for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++)
            sum += s->live_filled[i][c];
        if (sum > s->total_hearts[c]) sum_ok = 0;
        for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++)
            if (s->live_filled[i][c] > s->total_hearts[c]) card_ok = 0;
    }
    dump8("T3 total_hearts", s->total_hearts);
    dump8("T3 live[0].filled", s->live_filled[0]);
    CHECK(sum_ok,
          "#D T3 allocation conserves the pool: for every colour the sum of "
          "live_filled over all live cards is <= total_hearts (the greedy "
          "allocator mutates a SHARED pool, so a heart allocated twice is the "
          "bug this catches)");
    CHECK(card_ok,
          "#D T3 no single live card is filled beyond the whole pool");

    /* Non-vacuity: the pool must actually have held hearts, or conservation
     * over an all-zero pool is trivially true. */
    int pool_total = 0;
    for (int c = 0; c < 8; c++) pool_total += s->total_hearts[c];
    CHECK(pool_total > 0,
          "#D T3 the pool was NON-EMPTY (total_hearts sum=%d); conservation "
          "over an empty pool proves nothing", pool_total);
    int fill_total = 0;
    for (int c = 0; c < 8; c++) fill_total += s->live_filled[0][c];
    CHECK(fill_total > 0,
          "#D T3 the live card actually drew hearts (live[0].filled sum=%d); "
          "an all-zero fill would make the conservation check vacuous",
          fill_total);

    GAP("#D T3 audit_allocations_match_filled's exact form is NOT EXPRESSIBLE: "
        "RbLiveSnapshot has no breakdown.allocations list (no allocations, no "
        "target_idx, no per-allocation colour/amount/phase), so the Rust fold "
        "over `perf.breakdown.allocations` has no counterpart. The "
        "conservation law asserted above is the C-expressible restatement of "
        "what that ledger exists to prove.");
}

/* TEST 4: additive requirement modifier stacks on base */
static void test_d4_additive_modifier_stacks(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int member = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "#D T4 fixtures resolve EXACTLY (live=%d member=%d filler=%d)",
          live, member, filler);
    if (live < 0 || member < 0 || filler < 0) return;

    int stage[3] = { -1, member, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    /* Additive +2 on the h03 requirement (set stays 0, as in Rust). */
    rb_mods_add_need_heart(&tg.state.mods, live, rb_parse_heart_color("heart03"), 2);
    CHECK(rb_mods_get_need_heart(&tg.state.mods, live, rb_parse_heart_color("heart03")) == 2,
          "#D T4 the need_heart modifier was staged as additive-only (total %d)",
          rb_mods_get_need_heart(&tg.state.mods, live, rb_parse_heart_color("heart03")));

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T4 the live card enters P1's live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T4 P1 has a performance snapshot");
    if (!s) return;
    CHECK(s->n_lives == 1, "#D T4 the snapshot holds one lives entry (got %d)", s->n_lives);
    if (s->n_lives < 1) return;

    dump8("T4 live[0].required", s->live_required[0]);
    dump8("T4 live[0].filled", s->live_filled[0]);
    CHECK(s->live_required[0][3] == 3,
          "#D T4 h03 required should be base(1) + additive(2) = 3 (got %d)",
          s->live_required[0][3]);
    CHECK(s->live_required[0][1] == 1,
          "#D T4 h01 required unchanged from base (got %d)", s->live_required[0][1]);
    CHECK(s->live_required[0][6] == 1,
          "#D T4 h06 required unchanged from base (got %d)", s->live_required[0][6]);
    CHECK(s->live_passed[0] != 0,
          "#D T4 card PASSES: h03 filled >= h03_req(3) and the others met");
}

/* TEST 5: additive modifier can make the requirement impossible */
static void test_d5_additive_modifier_causes_failure(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int member = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "#D T5 fixtures resolve EXACTLY (live=%d member=%d filler=%d)",
          live, member, filler);
    if (live < 0 || member < 0 || filler < 0) return;

    int stage[3] = { -1, member, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    rb_mods_add_need_heart(&tg.state.mods, live, rb_parse_heart_color("heart01"), 5);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T5 the live card enters P1's live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T5 P1 has a performance snapshot");
    if (!s) return;
    CHECK(s->n_lives == 1, "#D T5 the snapshot holds one lives entry (got %d)", s->n_lives);
    if (s->n_lives < 1) return;

    dump8("T5 live[0].required", s->live_required[0]);
    CHECK(s->live_required[0][1] == 6,
          "#D T5 h01 required should be base(1) + additive(5) = 6 (got %d)",
          s->live_required[0][1]);
    CHECK(s->live_required[0][3] == 1,
          "#D T5 h03 required unchanged (got %d)", s->live_required[0][3]);
    CHECK(s->live_required[0][6] == 1,
          "#D T5 h06 required unchanged (got %d)", s->live_required[0][6]);
    CHECK(!s->live_passed[0],
          "#D T5 card FAILS: h01 required=6, only 1 available");
    CHECK(!s->success, "#D T5 success=false when the card fails");
}

/* TEST 6: additive -1 zeroes an unsatisfiable colour requirement */
static void test_d6_set_zero_removes_requirement(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && filler >= 0,
          "#D T6 fixtures resolve EXACTLY (live=%d filler=%d)", live, filler);
    if (live < 0 || filler < 0) return;

    /* Stage: the filler supplies h01 and h03 but NO h06. */
    int stage[3] = { -1, filler, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    /* Rust uses ModifierEntry { set: 0, additive: -1 }. In C, set is a no-op
     * when zero (stats_pipeline.c:63 applies `set` only when entry.set != 0),
     * so the faithful spelling is the additive arm alone. */
    rb_mods_add_need_heart(&tg.state.mods, live, rb_parse_heart_color("heart06"), -1);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T6 the live card enters P1's live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T6 P1 has a performance snapshot");
    if (!s) return;
    CHECK(s->n_lives == 1, "#D T6 the snapshot holds one lives entry (got %d)", s->n_lives);
    if (s->n_lives < 1) return;

    dump8("T6 live[0].required", s->live_required[0]);
    CHECK(s->live_required[0][1] == 1, "#D T6 h01 still required (got %d)", s->live_required[0][1]);
    CHECK(s->live_required[0][3] == 1, "#D T6 h03 still required (got %d)", s->live_required[0][3]);
    CHECK(s->live_required[0][6] == 0,
          "#D T6 h06 requirement should be 0 after additive -1 (got %d)",
          s->live_required[0][6]);
    CHECK(s->live_passed[0] != 0,
          "#D T6 card PASSES with the h06 requirement removed");
    CHECK(s->success != 0, "#D T6 success=true when all requirements are met");
}

/* TEST 7: a MEMBER in the live zone yields no live performance entry.
 *
 * KNOWN RED, DELIBERATELY -- this section's snapshot-existence check fails and
 * stays strict. Rust performance_snapshot_audit_test.rs:463-467 does
 *     game.state.performance_snapshots.iter().find(|s| s.player_id == "p1")
 *         .expect("...")
 * and only then asserts perf.lives.is_empty(). Rust ALWAYS has that snapshot:
 * engine/src/turn/phases.rs:621-632 calls build_snapshot() and
 * push_performance_snapshot() unconditionally for the performer, and
 * build_snapshot (live.rs:3133-3152) iterates perf.live_card_ids -- captured
 * BEFORE the heart check cleared the zone -- so the snapshot exists with an
 * EMPTY lives vec.
 * C does not: src/turn/live.c:446 in rb_perform_live has
 *     if (P->live.n == 0) return 0;
 * so when check_timing has already dropped the invalid member there is no
 * snapshot at all. Consequence beyond the test: g->snapshots is the input to
 * rb_populate_live_verdicts (live.c:685) and the trace parity diff, so a
 * missing snapshot silently skips all downstream snapshot processing.
 * The rest of this section's assertions (lives empty, no success, zero score)
 * are reachable only if the snapshot exists, so they stay guarded. */

static void test_d7_member_as_live_is_invalid(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int member_as_live = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(member_as_live >= 0 && filler >= 0,
          "#D T7 fixtures resolve EXACTLY (member=%d filler=%d)", member_as_live, filler);
    if (member_as_live < 0 || filler < 0) return;
    CHECK(rb_card_is_member(member_as_live) != 0,
          "#D T7 the invalid live-zone card really is a MEMBER card (%s)",
          no_of(member_as_live));

    int stage[3] = { -1, -1, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, member_as_live);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, member_as_live),
          "#D T7 the member is placed into P1's live zone");
    run_full_turn(&tg);

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T7 P1 STILL has a performance snapshot (the window runs; "
                     "the member is dropped as an invalid live card)");
    if (!s) return;

    CHECK(s->n_lives == 0,
          "#D T7 a member removed as an invalid live card must NOT produce a "
          "lives entry (n_lives=%d)", s->n_lives);
    CHECK(!s->success, "#D T7 no valid live performed -> no success");
    CHECK(s->total_score == 0, "#D T7 no valid live -> zero score (got %d)", s->total_score);
}

/* Run to Active with BOTH seats having set a live card. Mirrors the shared
 * "run to Active" loop of Rust TEST 8 / TEST 10. */
static void run_both_seats_to_active(TestGame *tg)
{
    for (int i = 0; i < 40; i++) {
        if (tg->state.phase == RB_PHASE_ACTIVE && !test_has_pending_choice(tg)) return;
        const char *choice = test_pending_choice_type(tg);
        if (choice[0] == 0) {
            test_pass(tg);
        } else if (strcmp(choice, "SelectCard") == 0) {
            test_resume_choice(tg, -1);
        } else if (strcmp(choice, "SelectAutoAbility") == 0) {
            test_resume_choice(tg, 0);
        } else {
            fprintf(stderr, "FAIL: unexpected prompt running to Active "
                            "(expected SelectCard arrange or none), got %s\n", choice);
            g_failures++;
            publish_counts();
            return;
        }
    }
    fprintf(stderr, "FAIL: two-seat run did not return to Active\n");
    g_failures++;
    publish_counts();
}

static void setup_bare_both_seats(TestGame *tg, const int p1_stage[3],
                                  const int p2_stage[3], int filler)
{
    clear_bag(&tg->state.p[0].deck);
    clear_bag(&tg->state.p[0].hand);
    clear_bag(&tg->state.p[0].discard);
    clear_bag(&tg->state.p[0].success);
    clear_bag(&tg->state.p[0].energy_deck);
    clear_bag(&tg->state.p[1].deck);
    clear_bag(&tg->state.p[1].hand);
    clear_bag(&tg->state.p[1].discard);
    clear_bag(&tg->state.p[1].success);
    clear_bag(&tg->state.p[1].energy_deck);
    for (int i = 0; i < 40; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        tg->state.p[0].stage[i] = p1_stage[i];
        tg->state.p[1].stage[i] = p2_stage[i];
    }
}

/* TEST 8: both seats fail -> neither wins, neither gets a success-zone card */
static void test_d8_both_players_fail_no_winner(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int live2 = test_new_id(&tg, LIVE_DASH);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && live2 >= 0 && filler >= 0,
          "#D T8 fixtures resolve EXACTLY (live=%d live2=%d filler=%d)",
          live, live2, filler);
    if (live < 0 || live2 < 0 || filler < 0) return;
    CHECK(live != live2,
          "#D T8 live2 is a DISTINCT card-copy instance (live=%d live2=%d)",
          live, live2);

    int p1_stage[3] = { -1, -1, -1 };
    int p2_stage[3] = { -1, -1, -1 };
    setup_bare_both_seats(&tg, p1_stage, p2_stage, filler);
    test_add_to_hand_for(&tg, 0, live);
    test_add_to_hand_for(&tg, 1, live2);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T8 P1 sets its live card");
    test_pass(&tg);   /* LiveCardSet first attacker -> second attacker */
    CHECK(set_live_card_for(&tg, 1, live2), "#D T8 P2 sets its live card");
    run_both_seats_to_active(&tg);

    CHECK(test_zone_len(&tg, 0, "success") == 0,
          "#D T8 P1 failed live should NOT get a success zone card (got %d)",
          test_zone_len(&tg, 0, "success"));
    CHECK(test_zone_len(&tg, 1, "success") == 0,
          "#D T8 P2 failed live should NOT get a success zone card (got %d)",
          test_zone_len(&tg, 1, "success"));

    RbLiveSnapshot *s1 = snapshot_for(&tg, 0);
    RbLiveSnapshot *s2 = snapshot_for(&tg, 1);
    CHECK(s1 != NULL, "#D T8 P1 must have a performance snapshot");
    CHECK(s2 != NULL, "#D T8 P2 must have a performance snapshot");
    if (s1) CHECK(!s1->success, "#D T8 P1 success should be false");
    if (s2) CHECK(!s2->success, "#D T8 P2 success should be false");
    if (s1) {
        CHECK(s1->n_lives == 1, "#D T8 P1 snapshot holds one lives entry (got %d)", s1->n_lives);
        if (s1->n_lives >= 1) {
            int req = 0;
            for (int c = 0; c < 8; c++) req += s1->live_required[0][c];
            CHECK(req > 0,
                  "#D T8 P1 live[0] carries a real requirement (total %d); a 0 "
                  "would make the failure meaningless", req);
            CHECK(!s1->live_passed[0],
                  "#D T8 P1 live[0] fails with an empty stage");
        }
    }
}

/* TEST 9: THE CORE REGRESSION -- zero stage, live card with requirements */
static void test_d9_zero_stage_live_card_must_fail(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && filler >= 0,
          "#D T9 fixtures resolve EXACTLY (live=%d filler=%d)", live, filler);
    if (live < 0 || filler < 0) return;

    int stage[3] = { -1, -1, -1 };
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);
    int census_before = test_total_card_count(&tg);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T9 the live card enters P1's live zone");
    run_full_turn(&tg);

    CHECK(!test_zone_has_id(&tg, 0, "success", live),
          "#D T9 the card must NOT land in the success zone despite zero "
          "stage hearts");
    CHECK(test_zone_has_id(&tg, 0, "discard", live),
          "#D T9 the card must be in the waitroom after failing requirements");
    CHECK(test_total_card_count(&tg) == census_before,
          "#D T9 the zone census is conserved across the failed live "
          "(before=%d after=%d)", census_before, test_total_card_count(&tg));

    RbLiveSnapshot *s = snapshot_for(&tg, 0);
    CHECK(s != NULL, "#D T9 P1 must have a snapshot");
    if (!s) return;

    CHECK(!s->success,
          "#D T9 snapshot.success must be false with zero stage hearts");
    CHECK(s->total_score == 0,
          "#D T9 total_score must be 0 on a failed live (got %d)", s->total_score);

    /* Rust iterates perf.lives; here the loop must have run at least once. */
    CHECK(s->n_lives >= 1,
          "#D T9 the snapshot holds at least one lives entry (got %d); with "
          "zero entries the per-entry assertions below would be vacuous",
          s->n_lives);
    for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++) {
        int req_total = 0, filled_total = 0;
        for (int c = 0; c < 8; c++) {
            req_total += s->live_required[i][c];
            filled_total += s->live_filled[i][c];
        }
        dump8("T9 live[i].required", s->live_required[i]);
        dump8("T9 live[i].filled", s->live_filled[i]);
        CHECK(!s->live_passed[i],
              "#D T9 live[%d] must not pass with zero hearts", i);
        CHECK(req_total > 0,
              "#D T9 live[%d] required_total=%d must be > 0 -- requirements "
              "were not loaded from the card record", i, req_total);
        CHECK(filled_total == 0,
              "#D T9 live[%d] filled_total must be 0 with zero stage hearts "
              "(got %d)", i, filled_total);
    }
}

/* TEST 10: P1 passes, P2 fails */
static void test_d10_p1_passes_p2_fails(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, LIVE_DASH);
    int live2 = test_new_id(&tg, LIVE_DASH);
    int member = test_id_exact(&tg, MEMBER);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(live >= 0 && live2 >= 0 && member >= 0 && filler >= 0,
          "#D T10 fixtures resolve EXACTLY (live=%d live2=%d member=%d filler=%d)",
          live, live2, member, filler);
    if (live < 0 || live2 < 0 || member < 0 || filler < 0) return;
    CHECK(live != live2, "#D T10 live2 is a DISTINCT card-copy instance (%d vs %d)",
          live, live2);

    int p1_stage[3] = { -1, member, -1 };
    int p2_stage[3] = { -1, -1, -1 };
    setup_bare_both_seats(&tg, p1_stage, p2_stage, filler);
    test_add_to_hand_for(&tg, 0, live);
    test_add_to_hand_for(&tg, 1, live2);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card_for(&tg, 0, live), "#D T10 P1 sets its live card");
    test_pass(&tg);
    CHECK(set_live_card_for(&tg, 1, live2), "#D T10 P2 sets its live card");
    run_both_seats_to_active(&tg);

    CHECK(test_zone_len(&tg, 0, "success") == 1,
          "#D T10 P1 (requirements met) must have exactly one success-zone "
          "card (got %d)", test_zone_len(&tg, 0, "success"));
    CHECK(test_zone_len(&tg, 1, "success") == 0,
          "#D T10 P2 (requirements failed) must have NO success-zone card "
          "(got %d)", test_zone_len(&tg, 1, "success"));
    CHECK(test_zone_has_id(&tg, 1, "discard", live2),
          "#D T10 P2's failed live card must be in its waitroom");
    CHECK(!test_zone_has_id(&tg, 1, "success", live2),
          "#D T10 P2's failed live card must NOT be anywhere in its success zone");

    RbLiveSnapshot *s1 = snapshot_for(&tg, 0);
    RbLiveSnapshot *s2 = snapshot_for(&tg, 1);
    CHECK(s1 != NULL, "#D T10 P1 snapshot exists");
    CHECK(s2 != NULL, "#D T10 P2 snapshot exists");
    if (s1) {
        CHECK(s1->n_lives == 1, "#D T10 P1 snapshot holds one lives entry (got %d)", s1->n_lives);
        if (s1->n_lives >= 1)
            CHECK(s1->live_passed[0] != 0,
                  "#D T10 P1's live card passes with the member on stage");
    }
    if (s2) {
        CHECK(s2->n_lives >= 1, "#D T10 P2 snapshot holds a lives entry (got %d)", s2->n_lives);
        if (s2->n_lives >= 1)
            CHECK(!s2->live_passed[0],
                  "#D T10 P2's live card fails with an empty stage");
    }
}

/* ==========================================================================
 * #E  bytecode_deep_compare_test.rs -- load-time guard only
 *
 * The deep compare itself needs serde_json + the serde JSON-path decode system
 * and has no C counterpart. What IS expressible is the load-time string-table
 * self-check: rb_verify_generated_tables() (src/ability/vm.c) walks every
 * interned string position in the compiled offsets, recomputes the FNV-1a
 * digest and compares it to the digest the generator recorded. This is the
 * direct defence against the skew that made 458 of 985 abilities decode at
 * least one string wrong: the (chunk,start,len) triples in bytecode_blob.c
 * index a string TABLE that is gitignored and regenerated, and the pool is
 * UNSEPARATED, so skew used to surface as byte splices rather than a failure.
 * tests/test_decode_skew.c drives the full corpus-wide plausibility walk on
 * top of this; this section only proves the guard itself is armed.
 * ========================================================================== */
static void test_e1_generated_table_self_check(void)
{
    int bad_strings = 0, bad_keys = 0;
    char msg[4096];
    int rc = rb_verify_generated_tables(&bad_strings, &bad_keys, msg, sizeof msg);
    fputs(msg, stdout);
    fflush(stdout);

    unsigned n_strings = RBKA_NUM_STRING_OFFSETS - 1;
    printf("info: #E interned string pool: %u entries, %u abilities, "
           "%u card-ability pairs\n",
           n_strings, (unsigned)RBKA_NUM_ABILITIES_C,
           (unsigned)RBKA_NUM_CARD_ABILITY_PAIRS);
    fflush(stdout);

    CHECK(n_strings > 5000,
          "#E the interned string pool is NON-EMPTY (%u entries); a "
          "self-check over an empty pool would pass without proving anything",
          n_strings);
    CHECK(RBKA_NUM_ABILITIES_C == 985,
          "#E the generated ability table header pins 985 abilities "
          "(got %u)", (unsigned)RBKA_NUM_ABILITIES_C);
    CHECK(rb_num_abilities() == RBKA_NUM_ABILITIES_C,
          "#E the engine's loaded ability count matches the generated header "
          "(%u vs %u)", rb_num_abilities(), (unsigned)RBKA_NUM_ABILITIES_C);
    CHECK(RBKA_NUM_CARD_ABILITY_PAIRS == 2116,
          "#E the card-ability pair table pins 2116 pairs, which is "
          "abilities.json statistics.total_abilities (got %u)",
          (unsigned)RBKA_NUM_CARD_ABILITY_PAIRS);
    CHECK(rc == 0,
          "#E rb_verify_generated_tables() reports a consistent table (rc=%d)",
          rc);
    CHECK(bad_strings == 0,
          "#E zero defective interned strings (got %d) -- this is the skew "
          "that made 46.5%% of abilities decode a spliced string", bad_strings);
    CHECK(bad_keys == 0, "#E zero defective key positions (got %d)", bad_keys);

    GAP("#E bytecode_deep_compare_test.rs's serde-vs-bytecode Ability "
        "deep-compare is NOT EXPRESSIBLE in C: the engine has no JSON parser "
        "and no serde path, so `populate_from_json` / the serialized-form "
        "comparison has no oracle. Its first assertion "
        "(ability_count() == json unique_abilities length) IS ported, in #A. "
        "The string-fidelity subject matter is covered by this guard plus "
        "tests/test_decode_skew.c.");
}

/* ==========================================================================
 * #F  action_coverage_test.rs
 *
 * Rust fires every action type on a member card and requires no
 * "No abilities"/"unknown action"/"not implemented" error. The C engine has
 * exactly that question and answers it with a table lookup:
 * rb_executor_has_executor(action) (src/ability/effects/executor.c). This is
 * the same claim -- every action the corpus actually uses has a dispatch arm
 * -- expressed as a census instead of a per-action execution, because the
 * behavioural half (stage + activate + drain every card) is already run for
 * the WHOLE corpus by tests/test_parity_characterization_extra.c section D.
 *
 * The Rust collector walks eff.action plus compound.actions only. This walk is
 * a superset: it also descends options[] and every named compound slot.
 *
 * KNOWN RED, DELIBERATELY. This section FAILS on exactly one action string:
 * `replacement_appear_wait`. That is a real C-engine gap and the assertion
 * stays strict. Evidence:
 *   - cards/abilities.json carries "action": "replacement_appear_wait" on the
 *     constant (triggers = jidou) ability of PL!-pb2-002-PP and
 *     PL!-pb2-002-R (Ayanagi Erii): count=4, card_type=member_card, position=center,
 *     original_value=true, heart_total_limit=4, heart_total_operator="<=",
 *     replacement="appear_wait".
 *   - Rust HAS it: ActionType::ReplacementAppearWait mapping to
 *     "replacement_appear_wait" (engine/src/ability/enums.rs:331,439), a
 *     describe template (describe.rs:191), the predicate
 *     "replacement_appear_wait" (core/card.rs:1310), and the phase hook
 *     game_state.replacement_appear_wait_applies(card_id, area)
 *     (engine/src/turn/phases.rs:1281).
 *   - C HAS none of them: no arm in rb_action_type_from_str
 *     (src/ability/enums.c:36), no RB_ACTION_* variant, no executor entry.
 * Rust's action_coverage_test.rs would return Err("unknown action") for this
 * card and fail; C silently drops the effect. The same string is reported as a
 * non-fatal EXPECTED_GAP by test_parity_characterization.c's
 * test_action_coverage_control.
 * ======================================================================== */
#define MAX_ACTIONS 512

static int collect_action_census(int seen[MAX_ACTIONS][64], char names[][64],
                                 int n_seen, const AbilityEffect *e, int depth)
{
    if (!e || depth > 24) return n_seen;
    const char *a = e->action;
    if (a && *a) {
        int found = 0;
        for (int i = 0; i < n_seen; i++)
            if (strcmp(names[i], a) == 0) { found = 1; break; }
        if (!found && n_seen < MAX_ACTIONS) {
            snprintf(names[n_seen], 64, "%s", a);
            seen[n_seen][0] = 1;
            n_seen++;
        }
    }
    n_seen = collect_action_census(seen, names, n_seen, e->primary_effect, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->alternative_effect, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->look_action, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->select_action, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->followup_action, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->optional_action, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->conditional_action, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->gained_effect, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->resource_on_select, depth + 1);
    n_seen = collect_action_census(seen, names, n_seen, e->opponent_action, depth + 1);
    for (int i = 0; i < e->n_child; i++)
        n_seen = collect_action_census(seen, names, n_seen, e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++)
        n_seen = collect_action_census(seen, names, n_seen, e->options[i], depth + 1);
    return n_seen;
}

static void test_f1_every_corpus_action_has_an_executor(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);

    static int seen[MAX_ACTIONS][64];
    static char names[MAX_ACTIONS][64];
    static char example[MAX_ACTIONS][40];   /* first card_no using the action */
    int n_seen = 0;
    long member_cards = 0, member_cards_with_abilities = 0;

    uint32_t nc = rb_num_cards();
    for (uint32_t ci = 0; ci < nc; ci++) {
        const unsigned char *r = rb_card_record(ci);
        if (!r) continue;
        const char *no = rb_card_string((uint16_t)(r[0] | (r[1] << 8)));
        if (!no || !*no) continue;
        int cid = test_id_exact(&tg, no);
        if (cid < 0) continue;
        if (!rb_card_is_member(cid)) continue;
        member_cards++;

        int na = rb_card_num_abilities(ci);
        if (na <= 0) continue;
        member_cards_with_abilities++;
        for (int i = 0; i < na; i++) {
            Ability a;
            if (!rb_decode_card_ability(ci, i, &a)) continue;
            /* seed the census with this card's number so a gap report names
             * the cards that need a dispatch arm, not just the action */
            char before[40];
            snprintf(before, sizeof before, "%s", no);
            int n_before = n_seen;
            n_seen = collect_action_census(seen, names, n_seen, a.effect, 0);
            for (int k = n_before; k < n_seen; k++) snprintf(example[k], 40, "%s", before);
            rb_free_ability(&a);
        }
    }

    printf("info: #F action coverage: %ld member cards, %ld with abilities, "
           "%d distinct actions\n",
           member_cards, member_cards_with_abilities, n_seen);
    fflush(stdout);

    /* Population assertions FIRST -- without them the executor check below
     * could pass over an empty set. */
    CHECK(member_cards == 1574,
          "#F the walk examined 1574 member cards (got %ld). This is the "
          "population action_coverage_test.rs filters to "
          "(`db.card_type() == CardType::Member`); the corpus holds 2198 "
          "unit records, the rest being non-member types", member_cards);
    CHECK(member_cards_with_abilities == 1385,
          "#F 1385 member cards carry at least one ability edge (got %ld)",
          member_cards_with_abilities);
    CHECK(n_seen == 40,
          "#F the corpus uses 40 distinct action strings on member-card "
          "abilities (got %d). Rust collects the same set at "
          "action_coverage_test.rs:38-50; a change means the corpus changed",
          n_seen);
    CHECK(n_seen >= 30,
          "#F anti-vacuity floor: the distinct-action census is large enough "
          "that it cannot be a truncated walk (%d)", n_seen);
    CHECK(n_seen < MAX_ACTIONS,
          "#F the distinct-action census FIT (%d < %d); hitting the cap would "
          "mean the action set is unbounded and the census is incomplete",
          n_seen, MAX_ACTIONS);

    int no_executor = 0, unknown_type = 0;
    for (int i = 0; i < n_seen; i++) {
        if (!rb_executor_has_executor(names[i])) {
            fprintf(stderr, "  #F NO EXECUTOR: %-28s first seen on card %s\n",
                    names[i], example[i]);
            no_executor++;
        }
        if (rb_action_type_from_str(names[i]) < 0) {
            fprintf(stderr, "  #F UNKNOWN ACTION TYPE: %-28s first seen on card %s\n",
                    names[i], example[i]);
            unknown_type++;
        }
    }
    fflush(stderr);
    CHECK(no_executor == 0,
          "#F every one of the %d action strings the corpus uses has a "
          "dispatch arm in the executor table (%d have none; see the "
          "NO EXECUTOR lines above for the card numbers)", n_seen, no_executor);
    CHECK(unknown_type == 0,
          "#F every one of the %d corpus action strings maps to an ActionType "
          "(%d do not; rb_action_type_from_str returns -1 and the effect is "
          "silently dropped)", n_seen, unknown_type);
}

/* ==========================================================================
 * main
 * ========================================================================== */
static int n_ok, n_failed, n_crashed, n_gaps_total;
static int n_assertions_total, n_failures_total;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    memset(g_shared, 0, sizeof *g_shared);

    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "FAIL: fork() failed for %s -- running in process\n", name);
        snprintf(g_case_name, sizeof g_case_name, "%s", name);
        g_assertions = 0;
        g_failures = 0;
        fn();
        if (g_failures) n_failed++; else n_ok++;
        n_gaps_total += g_gaps;
        printf("%-8s %s  [%d assertions]\n", g_failures ? "FAILED" : "ok", name, g_assertions);
        return;
    }
    if (pid == 0) {
        /* The child must not inherit the parent's accumulating tallies. */
        g_assertions = 0;
        g_failures = 0;
        g_gaps = 0;
        snprintf(g_case_name, sizeof g_case_name, "%s", name);
        fn();
        if (g_shared) {
            g_shared->assertions = g_assertions;
            g_shared->failures = g_failures;
            g_shared->exited_ok = 1;
            size_t i = 0;
            for (; i + 1 < sizeof g_shared->name && g_case_name[i]; i++)
                g_shared->name[i] = g_case_name[i];
            g_shared->name[i] = 0;
        }
        printf("        %-46s %4d assertion(s), %d failure(s), %d gap(s)\n",
               name, g_assertions, g_failures, g_gaps);
        fflush(stdout);
        fflush(stderr);
        _Exit(0);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) status = -1;

    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault); it had reached "
               "%d assertion(s) with %d failure(s)\n",
               "CRASH", g_shared->name[0] ? g_shared->name : name,
               WTERMSIG(status), g_shared->assertions, g_shared->failures);
    } else if (g_shared->signal) {
        n_crashed++;
        printf("%-8s %s  <-- fatal signal %d inside the case\n",
               "CRASH", g_shared->name[0] ? g_shared->name : name, g_shared->signal);
    } else if (g_shared->failures > 0) {
        n_failed++;
        printf("%-8s %s  (%d assertion(s), %d failure(s), %d gap(s))\n",
               "FAILED", name, g_shared->assertions, g_shared->failures, g_shared->exited_ok ? 0 : -1);
    } else {
        n_ok++;
        printf("%-8s %s  (%d assertion(s))\n", "ok", name, g_shared->assertions);
    }
    n_assertions_total += g_shared->assertions;
    n_failures_total += g_shared->failures;
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, fatal_handler);
    signal(SIGBUS,  fatal_handler);
    signal(SIGABRT, fatal_handler);

    g_shared = mmap(NULL, sizeof *g_shared, PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (g_shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: mmap(MAP_SHARED|MAP_ANONYMOUS) for the shared "
                        "counter block failed\n");
        return 1;
    }
    memset(g_shared, 0, sizeof *g_shared);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    /* Heart mapping is derived from the ENGINE's own parser, not hardcoded:
     * 0=heart00 .. 6=heart06, 7=all (state.c rb_heart_color_index). */
    {
        static const char *const probe[8] = {
            "heart00","heart01","heart02","heart03",
            "heart04","heart05","heart06","heart07"
        };
        printf("heart colour map (rb_parse_heart_color):");
        for (int i = 0; i < 8; i++)
            printf("  %d=%s", i, probe[i]);
        printf("\n");
        printf("resolved:");
        for (int i = 0; i < 8; i++)
            printf("  %s->%d", probe[i], (int)rb_parse_heart_color(probe[i]));
        printf("\n");
        fflush(stdout);
    }

    printf("corpus census: cards=%u abilities=%u interned_strings=%u\n\n",
           rb_num_cards(), rb_num_abilities(),
           (unsigned)(RBKA_NUM_STRING_OFFSETS - 1));

    printf("--- A  ability_golden_test.rs (re-implemented; the tests/ported copy "
           "is built by NOTHING) ---\n");
    run("a1_abilities_json_shape", test_a1_abilities_json_shape);

    printf("--- B  describe_parity_test.rs (NOT PORTABLE; node census only) ---\n");
    run("b1_describe_node_census", test_b1_describe_node_census);

    printf("--- C  logging_test.rs (bounded-buffer arm only) ---\n");
    run("c1_verdict_ring_is_bounded", test_c1_verdict_ring_is_bounded);

    printf("--- D  performance_snapshot_audit_test.rs ---\n");
    run("d1_sufficient_hearts_passes",   test_d1_sufficient_hearts_passes);
    run("d2_two_cards_second_fails",     test_d2_two_cards_second_fails);
    run("d3_allocation_conservation",    test_d3_allocation_conservation);
    run("d4_additive_modifier_stacks",   test_d4_additive_modifier_stacks);
    run("d5_additive_modifier_fails",    test_d5_additive_modifier_causes_failure);
    run("d6_set_zero_removes_req",      test_d6_set_zero_removes_requirement);
    run("d7_member_as_live_invalid",    test_d7_member_as_live_is_invalid);
    run("d8_both_players_fail",         test_d8_both_players_fail_no_winner);
    run("d9_zero_stage_must_fail",      test_d9_zero_stage_live_card_must_fail);
    run("d10_p1_passes_p2_fails",       test_d10_p1_passes_p2_fails);

    printf("--- E  bytecode_deep_compare_test.rs (load-time guard arm) ---\n");
    run("e1_generated_table_self_check", test_e1_generated_table_self_check);

    printf("--- F  action_coverage_test.rs (executor coverage census) ---\n");
    run("f1_every_corpus_action_has_executor", test_f1_every_corpus_action_has_an_executor);

    printf("\n================ SUMMARY (test_parity_char2) ================\n");
    printf("cases: %d ok, %d FAILED, %d CRASHED\n", n_ok, n_failed, n_crashed);
    printf("assertions: %d, failures: %d\n", n_assertions_total, n_failures_total);
    printf("NOT RE-PORTED HERE (already covered by a peer's file):\n");
    printf("  corpus_smoke_test.rs  -> tests/test_parity_characterization_extra.c "
           "section D, which stages + activates + drains EVERY card and "
           "asserts smoked > 2000.\n");
    printf("  size_budget_test.rs   -> tests/test_parity_characterization_extra.c "
           "section J (RB_QUEUE_DEPTH == 16, an idle queue owns no resolver, "
           "sizeof(RbQueueEntry) < 4096). The Rust spelling of the test -- "
           "size_of::<AbilityQueueEntry>() < size_of::<AbilityResolver>() -- is "
           "NOT EXPRESSIBLE in C because RbAbilityResolver is an incomplete "
           "type in rabuka.h, so sizeof() on it does not compile.\n");
    printf("  unique_abilities_test.rs -> 1 byte on disk, zero #[test]; skipped.\n");
    printf("NOT PORTABLE AT ALL: describe_parity (no describe layer), "
           "logging_test's structured_log arms (no structured_log/rule_log/"
           "LogMetadata in C), bytecode_deep_compare's serde deep-compare "
           "(no JSON parser in C), snapshot breakdown.allocations (no such "
           "field on RbLiveSnapshot).\n");
    printf("==============================================================\n");
    fflush(stdout);

    rb_unload();
    munmap(g_shared, sizeof *g_shared);
    return (n_failed > 0 || n_crashed > 0) ? 1 : 0;
}
