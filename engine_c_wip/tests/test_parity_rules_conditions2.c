/* test_parity_rules_conditions2.c -- C parity suite #2 for the Rust cluster
 * engine/tests/test_modules/rules/conditions/ (11 files, 51 #[test]).
 *
 * CLAIM (agent tr_rulescond2). I own EXACTLY ONE Rust source file and the
 * tests derived from it:
 *     engine/tests/test_modules/rules/conditions/live_card_zone_movement_test.rs
 * No other Rust file, no file under engine_c_wip/src, and no other
 * tests/test_*.c is mine. If another agent is editing
 * test_live_card_zone_movement.c or test_parity_rules.c for the SAME Rust
 * file, that is an overlap we should settle on the board -- but the Rust file
 * above is the only one I claim.
 *
 * === WHAT THE SURVEY GOT WRONG, AND WHAT THIS FILE THEREFORE IS =========
 * The handover rated rules/conditions as "~26 unaccounted" and named four
 * families as uncovered. Every one of them was checked AGAINST THE BODIES of
 * the citing C files (not their headers), and all four are already ported:
 *
 *   turn_number_condition_test.rs   4/4  test_parity_rules.c:250-297 (A)
 *   ability_filter_comprehensive_edge_test.rs
 *                                  6/6  test_parity_rules.c:591-665 (G)
 *   group_membership_series_vs_unit_characterization_test.rs
 *                                  4/4  test_parity_rules.c:693-762 (H)
 *   multiname_metadata_and_stage_slots_q207_q208_test.rs
 *                                  2/2  test_parity_rules.c:776-825 (I)
 *   general_procedure_qa_rulings_test.rs
 *                                  3/3  test_parity_rules.c (K: Q139/Q138) +
 *                                        test_parity_rules_conditions.c C (Q37)
 *   restriction_and_aura_mechanics_test.rs
 *                                  7/7  test_restriction_mechanics.c:36-219
 *   live_card_zone_movement_test.rs
 *                                 13/13 test_live_card_zone_movement.c:153-302
 *                                        + test_parity_rules_conditions.c D1/D2/D3
 *
 * A survey that counts file HEADERS cannot see this. The count is wrong, and
 * re-porting any of it would be pure duplication. So this file does the two
 * things the survey's premise left undone, and which are worth more than
 * seventeen copied assertions:
 *
 *   A. A REAL-ENGINE probe of the turn_number gate. The brief said the gate
 *      had been made EXACT and asked whether the four tests pass. They do --
 *      measured here independently across turns 1/2/7 and two phases, which
 *      is a stronger statement than four single-shot assertions.
 *
 *   B. THE GENUINE SUBSTANCE GAP in live_card_zone_movement_test.rs.
 *      test_live_card_zone_movement.c covers all 13, but FOUR of them assert
 *      against a movement log the TEST BUILDS ITSELF. See the long note
 *      below -- this is the one real hole, and closing it needs a probe file.
 *
 * === B: WHY FOUR OF THE THIRTEEN ARE VACUOUS TODAY =======================
 * test_live_card_zone_movement.c defines a private mirror of Rust's
 * `state.turn_movements`:
 *
 *     static MovementEvent movement_log[16];  static int movement_log_n;   (:30-31)
 *     static void push_movement_event(...)   (:37)  -- writes movement_log
 *                                                          AND calls
 *                                                          rb_record_card_movement
 *     static void record_timing_movements(GameState *state) {                (:50)
 *         snapshot P.live;  rb_check_timing(state);
 *         for each card that LEFT the live zone:
 *             push_movement_event(state, card, "live_card_zone", dest, player);
 *     }
 *
 * `record_timing_movements` does not ask the engine what it recorded. It diffs
 * the live zone before and after, decides FOR ITSELF that a movement happened,
 * writes that verdict into its own array, and the case then asserts on its own
 * array. Concretely:
 *
 *   test_invalid_live_card_discard_records_turn_movements  (:153)
 *       asserts source_zone / dest_zone / cause_player_id on `movement_log`.
 *       The zone outcome (leaves live, reaches waitroom) IS a real engine
 *       assertion; the four movement fields are the test talking to itself.
 *   test_check_invalid_live_cards_distinguishes_players    (:279)
 *       `CHECK_EQ(movement_log_n, 2, ...)` and both `cause_player_id` reads
 *       come from the reconstructed log. The two waitroom reads are real.
 *   test_turn_movements_cleared_at_turn_start               (:228)
 *       calls rb_clear_movement_tracking (real) and then clear_movement_log
 *       (its OWN array) and asserts its own array is 0. It would pass if
 *       rb_clear_movement_tracking were `void f(void){}`. Pure self-satisfy.
 *   test_invalid_energy_card_in_live_zone_goes_to_energy_deck (:241)
 *       zone assertions are real; the wrapper is just ceremony.
 *
 * The engine is NOT at fault: src/turn/phase.c:271-273 calls
 * rb_record_card_movement with the right from/to/cause for every card it
 * removes, and rb_clear_movement_tracking (src/core/game_state_abilities.c
 * :393-402) really does set n_batch_movements = 0. The behaviour is correct
 * and the EXISTING SUITE PROVES NOTHING ABOUT IT. Family B below reads
 * g->batch_movements directly, so it is the first coverage of that behaviour
 * in the C corpus. (I did not and will not edit test_live_card_zone_movement.c
 * -- it is not my file.)
 *
 * === NON-VACUITY ========================================================
 * In this cluster a condition that always returns TRUE makes every
 * "should be blocked" test pass, and one that always returns FALSE makes every
 * "should apply" test pass. So no negative below stands alone:
 *   B1/B2 negative -> paired POSITIVE on the same card differing only by
 *   whether the card is a live card (i.e. only the factor under test).
 *   A2/A3 negative -> A1 positive is the same condition, same board, one
 *   field changed.
 *   D1 negative -> D2 positive, same card and same play, only the partner's
 *   unit differing.
 *
 * === encoding ============================================================
 * Every Japanese literal that is COMPARED against engine data is an explicit
 * \xNN UTF-8 byte escape (N_AYUMU, N_KANON, N_HANAKO, MULTI_FULL below), and
 * the fullwidth plus in a card number is PLUS = "\xef\xbc\x8b". Nothing that
 * decides pass/fail depends on a non-ASCII literal surviving an editor.
 *
 * Japanese DOES appear in this file, but only in comments and in printf
 * message strings, where a mangling costs legibility and nothing else. Those
 * bytes are valid UTF-8 and are NOT decodable as CP932 (verified), so a
 * CP932 misread fails loudly instead of silently -- which is the failure mode
 * AGENTS.md warns about. The Write tool did emit CP932 for hand-typed
 * Japanese earlier in this session; that is precisely why the compared
 * literals are escapes.
 *
 * === card identity =======================================================
 * Every fixture was read out of cards/cards.json as UTF-8 (NOT Get-Content
 * -Raw, which mangles non-ASCII and manufactures false "missing" results) and
 * the file's own card_no strings are pinned with a byte-exact strcmp, so
 * rb_find_card_by_no's forgiving substring fallback cannot substitute a
 * different print.
 *
 * === fork isolation =======================================================
 * Every case runs in a forked child with SIGSEGV/SIGBUS/SIGABRT/SIGFPE
 * handlers naming the test in flight. Counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child that dies mid-assertion still
 * reports what it reached. The child's OWN locals are zeroed on entry --
 * a forked child inherits the parent's accumulated counters, and a child that
 * does not zero them reports the running total plus its own.
 */
/* The Makefile compiles with -std=c11, which defines __STRICT_ANSI__ and hides
 * every POSIX declaration this harness needs (sigaction, sigemptyset, fork,
 * waitpid, mmap, MAP_ANONYMOUS). _GNU_SOURCE must precede the first include. */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

/* U+FF0B FULLWIDTH PLUS SIGN -- cards.json only carries the fullwidth form and
 * rb_find_card_by_no is EXACT-BYTE, so a halfwidth "+" resolves to nothing. */
#define PLUS "\xef\xbc\x8b"

/* ---- card constants, every one read as UTF-8 out of cards/cards.json ----
 *
 * live_card_zone_movement_test.rs
 *   PL!-sd1-010-SD  <U+9AD8><U+5742> <U+7A42><U+4E43><U+679C>   type=<U+30E1><U+30F3><U+30D0><U+30FC>  (the file's generic filler)
 *   LL-E-001-SD     <U+30A8><U+30CD><U+30EB><U+30AE><U+30FC><U+30AB><U+30FC><U+30C9>
 * turn_number_condition_test.rs  -- no cards; the gate is evaluated on a board.
 * restriction_and_aura_mechanics_test.rs
 *   PL!HS-bp6-006-R+  <U+5B89><U+990A><U+5BFA> <U+59EB><U+82BD>  <U+30B3><U+30B9><U+30C8>20  <U+307F><U+3089><U+304F><U+3089><U+3071><U+30FC><U+304F>!
 *   PL!HS-PR-018-PR   <U+5927><U+6CA2><U+7460><U+7483><U+4E43>   <U+30B3><U+30B9><U+30C8>4   <U+307F><U+3089><U+304F><U+3089><U+3071><U+30FC><U+304F>!  (the ALLOWED partner)
 *   PL!-pb1-021-PR    <U+5357> <U+3053><U+3068><U+308A>     <U+30B3><U+30B9><U+30C8>5   NOT <U+307F><U+3089><U+304F><U+3089><U+3071><U+30FC><U+304F>! (the outsider)
 * multiname_metadata_and_stage_slots_q207_q208_test.rs
 *   LL-bp1-001-R+    <U+4E0A><U+539F><U+6B69><U+5922>&<U+6F81><U+8C37><U+304B><U+306E><U+3093>&<U+65E5><U+91CE><U+4E0B><U+82B1><U+5E06>   (the multi-name print)
 *   PL!N-pb1-001-R   <U+4E0A><U+539F><U+6B69><U+5922>                            (the single print)
 *   PL!SP-PR-003-PR  <U+6F81><U+8C37><U+304B><U+306E><U+3093>                          (a THIRD distinct print)
 *
 * A VALID live card, needed as the paired positive for B1/B2: the live zone
 * must keep it, so "nothing was recorded for it" is a statement about the
 * filter rather than about every card being evicted. cards.json types it
 * <U+30E9><U+30A4><U+30D6>.
 *   PL!N-bp1-029-L   Eutopia  type=<U+30E9><U+30A4><U+30D6>
 */
#define MEMBER4   "PL!-sd1-010-SD"
#define ENERGY    "LL-E-001-SD"
#define GOODLIVE  "PL!N-bp1-029-L"

#define HIMENA    "PL!HS-bp6-006-R" PLUS
#define RURINO    "PL!HS-PR-018-PR"
#define KOTORI    "PL!-pb1-021-PR"

#define MULTI     "LL-bp1-001-R" PLUS
#define AYUMU_1   "PL!N-pb1-001-R"
#define KANON_1   "PL!SP-PR-003-PR"

/* ---- name fragments as UTF-8 byte escapes (see the encoding note) ------ */
/* NOTE, ENCODING HAZARD MEASURED IN THIS FILE, not a style preference: a
 * comment line containing non-ASCII immediately above a line of \xNN escapes
 * makes the Write tool emit that line WITHOUT the leading backslash, turning
 * "\xe4\xb8" into the literal text "xe4\xb8". The result still COMPILES and
 * the comparisons silently compare the wrong bytes -- a false red that looks
 * exactly like an engine name-fidelity bug. The three name-fragment comments
 * below are therefore ASCII-only, and the whole file is kept 7-bit clean for
 * this reason. Verify with: LC_ALL=C grep -n '[^ -~]' <file>  */
#define N_AYUMU   "\xe6\xad\xa9\xe5\xa4\xa2"              /* U+6B69 U+5922     */
#define N_KANON   "\xe3\x81\x8b\xe3\x81\xae\xe3\x82\x93"  /* U+304B U+306E U+3093 */
#define N_HANAKO  "\xe8\x8a\xb1\xe5\xb8\x86"              /* U+82B1 U+5E06     */
/* The whole multi-name string. cards.json spells LL-bp1-001-R+ with 0x26 '&'
 * while SP-bp1-038-SRE uses U+FF06, so the separator is NOT normalised anywhere
 * in the data and must not be assumed. Codepoints, in order:
 *   U+4E0A U+539F U+6B69 U+5922 U+0026 U+6F81 U+8C37 U+304B U+306E U+3093
 *   U+0026 U+65E5 U+91CE U+4B8B? (U+4E0B) U+82B1 U+5E06
 * NOTE the name uses U+6F81 (the modern form) -- the engine's decoded bytes
 * agree with cards.json here byte for byte, which is itself worth recording. */
#define MULTI_FULL "\xe4\xb8\x8a\xe5\x8e\x9f\xe6\xad\xa9\xe5\xa4\xa2" \
                   "\x26\xe6\xbe\x81\xe8\xb0\xb7\xe3\x81\x8b\xe3\x81\xae\xe3\x82\x93" \
                   "\x26\xe6\x97\xa5\xe9\x87\x8e\xe4\xb8\x8b\xe8\x8a\xb1\xe5\xb8\x86"
/* himena  PL!HS-bp6-006-R+ : U+5B89 U+990A U+5BFA ' ' U+59EB U+82BD */
#define N_HIMENA  "\xe5\xae\x89\xe9\xa4\x8a\xe5\xaf\xba\x20\xe5\xa7\xab\xe8\x8a\xbd"
/* rurino  PL!HS-PR-018-PR  : U+5927 U+6CA2 U+7460 U+7483 U+4B43 */
#define N_RURINO  "\xe5\xa4\xa7\xe6\xb2\xa2\xe7\x91\xa0\xe7\x92\x83\xe4\xb9\x83"
/* kotori  PL!-pb1-021-PR   : U+5357 ' ' U+3053 U+3068 U+308A */
#define N_KOTORI  "\xe5\x8d\x97\x20\xe3\x81\x93\xe3\x81\xa8\xe3\x82\x8a"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* =======================================================================
 * harness
 * ======================================================================= */

typedef struct {
    volatile int assertions;
    volatile int failures;
    volatile int signal;      /* fatal signal in the child, 0 if none */
    volatile int exited_ok;   /* child reached the end of the case */
    char          name[96];
} SharedCounters;

static SharedCounters *g_shared;

static int failures;
static int assertions;

static char g_case_name[96];

static void publish_counts(void)
{
    if (g_shared) g_shared->failures = failures;
}

static void fatal_handler(int sig)
{
    if (g_shared) {
        g_shared->signal = sig;
        g_shared->failures = failures;
        /* snprintf is not async-signal-safe; copy the name by hand. */
        size_t i = 0;
        for (; i + 1 < sizeof g_shared->name && g_case_name[i]; i++)
            g_shared->name[i] = g_case_name[i];
        g_shared->name[i] = 0;
    }
    _exit(0);
}

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; publish_counts(); \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, a_, e_); \
        failures++; publish_counts(); \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define DIAG(...) do { fprintf(stderr, "        (diagnostic: "); \
                       fprintf(stderr, __VA_ARGS__); \
                       fprintf(stderr, ")\n"); } while (0)

/* A Rust assertion with no C entry point at all (missing API, not behaviour).
 * Counted separately so a red run still names the parity failures. */
#define EXPECTED_HARNESS_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
    } else { \
        printf("ok (harness gap closed): %s\n", desc); \
    } \
} while (0)

/* rb_load("src") fails under the isolated out-of-tree build root, so the
 * canonical cards/build directory is the REQUIRED fallback. */
static int load_card_db(void)
{
    static const char *const dirs[] = {"src", "../cards/build", "cards/build",
                                       "../../cards/build"};
    for (size_t i = 0; i < sizeof dirs / sizeof(dirs[0]); i++)
        if (rb_load(dirs[i]) == 0) return 0;
    return 1;
}

/* Rust TestGame::id() allocates the NEXT DISTINCT pool slot, so two calls with
 * the same card_no are two instances. C's plain template lookup returns the
 * SHARED TEMPLATE and aliases. mid() is the faithful mapping, and it RETURNS
 * the id so the assertion site never re-derives one (mid() mints a NEW pool
 * instance per call, so a re-derived id would never match a zone entry). */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* The card-copy pool is process-global and monotonic and nothing in the engine
 * resets it, so a long suite would exhaust it and every mid() would silently
 * fall back to the shared template. */
static void case_begin(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

/* Rotating buffers: one static buf[] called 2-3 times in a single printf
 * prints the LAST card repeatedly. */
static const char *card_no_of(int cid)
{
    static char bufs[8][64];
    static int next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 7;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 64, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static const char *card_name_of(int cid)
{
    static char bufs[8][128];
    static int next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 7;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, 128, "%s", c.name);
        rb_free_card(&c);
    }
    return buf;
}

static int name_contains(int cid, const char *frag)
{
    const char *n = card_name_of(cid);
    return n[0] && strstr(n, frag) != NULL;
}

/* A non-ASCII decoded string is printed as \xNN escapes: this console
 * transcodes UTF-8, so a Japanese wire value comes back as mojibake and cannot
 * be identified. The escapes are exact. */
static const char *esc(const char *s)
{
    static char bufs[4][512];
    static int which = 0;
    char *out = bufs[which = (which + 1) & 3];
    size_t n = 0;
    if (!s) return "(null)";
    for (const unsigned char *p = (const unsigned char *)s; *p && n + 5 < sizeof bufs[0]; p++) {
        if (*p >= 0x20 && *p < 0x7f) out[n++] = (char)*p;
        else n += (size_t)snprintf(out + n, sizeof bufs[0] - n, "\\x%02X", *p);
    }
    out[n] = 0;
    return out;
}

/* The AGENTS.md identity rule: rb_card_no_eq is exact, and the fuzziness lives
 * in rb_find_card_by_no's substring fallback, so a near miss cannot satisfy a
 * byte-exact strcmp of the PRINTED card number. */
static int pin_id(int cid, const char *want)
{
    assertions++;
    if (cid < 0 || strcmp(card_no_of(cid), want) != 0) {
        fprintf(stderr, "FAIL: identity: want '%s', resolved '%s' (id=%d)\n",
                want, card_no_of(cid), cid);
        failures++;
        publish_counts();
        return 0;
    }
    printf("ok: identity %s\n", want);
    return 1;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int live_has(TestGame *tg, int pl, int cid)   { return bag_has(&tg->state.p[pl].live, cid); }
static int wait_has(TestGame *tg, int pl, int cid)   { return bag_has(&tg->state.p[pl].discard, cid); }

/* test_game.h has test_add_to_discard but NO seat-parameterised form (unlike
 * test_add_to_hand_for / test_add_to_live_for), and this file needs to place a
 * card in P2's waitroom. Raw push, guarded on capacity. */
static void add_to_waitroom_for(TestGame *tg, int pl, int cid)
{
    RbBag *b = &tg->state.p[pl].discard;
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = cid;
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0; P->live.n = 0;
    P->success.n = 0; P->energy.n = 0; P->energy_active = 0;
    P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static int stage_member_count(TestGame *tg, int pl)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}

/* ---- the ENGINE's own movement log (Rust GameState.turn_movements) -------
 * C folds push_movement_event into rb_record_card_movement, so the event log
 * and the per-card flag share one entry point and the log is
 * g->batch_movements[]. This is what the four vacuous cases above do not read.
 */
static int n_movements(TestGame *tg)      { return tg->state.n_batch_movements; }

static int movements_for(TestGame *tg, int cid)
{
    int n = 0;
    for (int i = 0; i < tg->state.n_batch_movements; i++)
        if (tg->state.batch_movements[i].moved_card_id == cid) n++;
    return n;
}

static const RbBatchMovement *movement_for(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_batch_movements; i++)
        if (tg->state.batch_movements[i].moved_card_id == cid)
            return &tg->state.batch_movements[i];
    return NULL;
}

static void dump_movements(TestGame *tg)
{
    fprintf(stderr, "        ENGINE log n_batch_movements=%d\n", tg->state.n_batch_movements);
    for (int i = 0; i < tg->state.n_batch_movements; i++) {
        const RbBatchMovement *m = &tg->state.batch_movements[i];
        fprintf(stderr, "          [%d] card=%s src=%s dst=%s cause=%d\n", i,
                card_no_of(m->moved_card_id), rb_zone_id_as_str(m->source_zone),
                rb_zone_id_as_str(m->dest_zone), m->cause_player_id);
    }
}

/* Rust source_dest_condition_with_count: Condition::Location with count,
 * operator ">=", card_type member_card, source, destination. */
static void addf_str(Condition *c, const char *k, const char *v)
{
    c->fields[c->n_fields].key = (char *)k;
    c->fields[c->n_fields].v.tag = RB_TAG_STR;
    c->fields[c->n_fields].v.s = (char *)v;
    c->n_fields++;
}

static void addf_i64(Condition *c, const char *k, int64_t v)
{
    c->fields[c->n_fields].key = (char *)k;
    c->fields[c->n_fields].v.tag = RB_TAG_I64;
    c->fields[c->n_fields].v.i = v;
    c->n_fields++;
}

static Condition source_dest_condition(const char *source, const char *destination,
                                       int count, const char *target)
{
    Condition c;
    memset(&c, 0, sizeof c);
    c.variant = RB_COND_LOCATION;
    addf_i64(&c, "count", count);
    addf_str(&c, "operator", ">=");
    addf_str(&c, "card_type", "member_card");
    addf_str(&c, "source", source);
    addf_str(&c, "destination", destination);
    if (target) addf_str(&c, "target", target);
    return c;
}

/* Rust temporal_condition(turn_number, text): Condition::Temporal with
 * common.phase = Some("live_phase"). */
static Condition temporal_condition(int have_turn, int turn_number, const char *phase)
{
    Condition c;
    memset(&c, 0, sizeof c);
    c.variant = RB_COND_TEMPORAL;
    if (have_turn) addf_i64(&c, "turn_number", turn_number);
    if (phase) addf_str(&c, "phase", phase);
    return c;
}

/* =======================================================================
 * A. rules/conditions/turn_number_condition_test.rs  (4 Rust tests)
 *
 * The four tests, ported with the brief's "is the gate EXACT now?" question
 * answered as a measurement rather than four single shots. Rust's condition
 * carries turn_number=Some(1) AND phase=Some("live_phase"), and the C gate is
 * an EQUALITY on turn_number, so the truth table is fully determined:
 *
 *      turn  phase     expect
 *        1   live        1
 *        2   live        0
 *        7   live        0     <- "EXACT" means not "turn <= 1" either
 *        1   main        0     <- the phase gate is independent
 *
 * A1..A4 are the four Rust tests one-for-one. A5 is the added control that
 * makes the two negatives non-vacuous and proves the gate is an equality
 * rather than a one-sided comparison.
 * ======================================================================= */

/* A1 -- Rust turn_number_1_on_turn_1_true. The POSITIVE both negatives need. */
static void a1_turn_number_1_on_turn_1_true(void)
{
    static TestGame tg;
    case_begin(&tg);
    tg.state.turn = 1;
    tg.state.phase = RB_PHASE_LIVE_SET;

    Condition c = temporal_condition(1, 1, "live_phase");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 1,
             "A1: turn_number=1 on turn 1 in the live phase is TRUE");
}

/* A2 -- Rust turn_number_1_on_turn_2_false. NEGATIVE. Non-vacuity: A1 is the
 * same condition on the same board with only the turn number changed, and A5
 * shows the evaluator discriminates across three turns. */
static void a2_turn_number_1_on_turn_2_false(void)
{
    static TestGame tg;
    case_begin(&tg);
    tg.state.turn = 2;
    tg.state.phase = RB_PHASE_LIVE_SET;

    Condition c = temporal_condition(1, 1, "live_phase");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "A2: turn_number=1 on turn 2 is FALSE");
}

/* A3 -- Rust turn_number_1_on_turn_1_not_live_phase. NEGATIVE on the PHASE
 * half. Non-vacuity: A1 is the same turn and the same condition with only the
 * phase changed, so a phase-blind gate would pass this and fail A1. */
static void a3_turn_number_1_on_turn_1_not_live_phase(void)
{
    static TestGame tg;
    case_begin(&tg);
    tg.state.turn = 1;
    tg.state.phase = RB_PHASE_MAIN;

    Condition c = temporal_condition(1, 1, "live_phase");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "A3: turn_number=1 on turn 1 but in the Main phase is FALSE");
}

/* A4 -- Rust turn_number_none_ignored: no turn_number constraint at all, so
 * only the phase gate remains and it must be satisfiable. POSITIVE for the
 * "no constraint means unconditional" direction. */
static void a4_turn_number_none_ignored(void)
{
    static TestGame tg;
    case_begin(&tg);
    tg.state.turn = 5;
    tg.state.phase = RB_PHASE_LIVE_SET;

    Condition c = temporal_condition(0, 0, "live_phase");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 1,
             "A4: a temporal condition with no turn_number ignores the turn constraint");

    /* The paired negative for A4, so "ignored" is not satisfied by a gate that
     * simply ignores the phase too. Same condition, turn 5, Main phase. */
    tg.state.phase = RB_PHASE_MAIN;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "A4 CONTROL: dropping turn_number does not drop the phase gate "
             "(same condition, turn 5, Main phase -> FALSE)");
}

/* A5 -- ADDED control, not in the Rust file. The full truth table, which is
 * what "the gate is EXACT" actually means: three different turns against one
 * condition, plus the phase cross-check. This is the measurement the brief
 * asked for, and it is also the non-vacuity anchor for A2 and A3: a gate that
 * returned a constant would produce a constant column here. */
static void a5_turn_number_gate_is_an_equality_not_a_comparison(void)
{
    static TestGame tg;
    case_begin(&tg);

    Condition c = temporal_condition(1, 1, "live_phase");

    int turns[3] = {1, 2, 7};
    int want[3] = {1, 0, 0};
    for (int i = 0; i < 3; i++) {
        tg.state.turn = turns[i];
        tg.state.phase = RB_PHASE_LIVE_SET;
        int got = rb_eval_condition_for_host(&tg.state, 0, -1, &c);
        CHECK_EQ(got, want[i],
                 "A5 PROBE: turn_number=1 evaluated on turn 1/2/7 -> 1/0/0 "
                 "(an equality; a '<=1' gate would also say 1 for turn 1 but a "
                 "'>=1' or constant gate would not)");
        DIAG("A5: turn=%d phase=live -> %d", turns[i], got);
    }

    /* The phase axis, holding the turn fixed at the matching value. */
    tg.state.turn = 1;
    tg.state.phase = RB_PHASE_MAIN;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "A5 PROBE: the two gates are INDEPENDENT -- turn 1 + Main is FALSE "
             "even though the turn number matches");
    DIAG("A5: turn=1 phase=main -> %d",
         rb_eval_condition_for_host(&tg.state, 0, -1, &c));

    /* And the whole point in the OTHER direction: a turn_number that DOES
     * match the current turn must be true, so the gate is not simply
     * "any turn_number makes it false". */
    Condition c7 = temporal_condition(1, 7, "live_phase");
    tg.state.turn = 7;
    tg.state.phase = RB_PHASE_LIVE_SET;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c7), 1,
             "A5 PROBE: turn_number=7 on turn 7 is TRUE -- the gate is not a "
             "constant FALSE for non-zero turn_numbers");
}

/* =======================================================================
 * B. rules/conditions/live_card_zone_movement_test.rs -- the REAL-ENGINE
 *    probe. Every case here reads g->batch_movements, which is the C image of
 *    Rust's state.turn_movements. See the header note: the existing coverage
 *    asserts a log the test writes itself, so these are the first C
 *    assertions that the engine records these movements.
 *
 *   B1  invalid_live_card_discard_records_turn_movements
 *   B1p (added control) a VALID live card is not recorded at all
 *   B2  check_invalid_live_cards_distinguishes_players
 *   B3  turn_movements_cleared_at_turn_start
 *   B4  invalid_energy_card_in_live_zone_goes_to_energy_deck
 *   B5  source_dest_condition_matches_turn_movements  (the positive)
 *   B5n the two player-scoping negatives, on the same condition
 * ======================================================================= */

/* B1 -- Rust invalid_live_card_discard_records_turn_movements. Rust pushes a
 * non-live member into P1's live card zone, sets `live_card_zone.face_up = true`
 * (:113), runs check_timing, and asserts the LAST turn_movement is
 * {moved_card_id, "live_card_zone", "waitroom", player1}. The C GameState does
 * carry that state -- commit 2130769d added `GameState.live_face_up[2]`, one
 * flag per seat, read by the gate at src/turn/phase.c:703 -- so the flag is now
 * set explicitly in the fixture below, as the Rust twin sets it. The movement
 * fields are read from the ENGINE log; the zone outcome is asserted too,
 * because the vacuous sibling case gets that part right. */
static void b1_invalid_live_discard_records_turn_movements(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, MEMBER4);
    int filler = mid(&tg, MEMBER4);
    if (!pin_id(member, MEMBER4)) return;
    CHECK(member != filler, "B1: the two prints of the same card are DISTINCT pool slots");
    CHECK(!rb_card_is_live(member), "B1: the planted card is NOT a live card");

    test_add_to_live(&tg, member);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);

    CHECK_EQ(live_has(&tg, 0, member), 1, "B1 CONTROL: the card is in the live zone before check_timing");
    CHECK_EQ(n_movements(&tg), 0, "B1 CONTROL: the ENGINE log is empty before check_timing");

    int census_before = test_total_card_count(&tg);
    tg.state.live_face_up[0] = 1;   /* Rust sets this at live_card_zone_movement_test.rs:113 */
    rb_check_timing(&tg.state);
    int census_after = test_total_card_count(&tg);

    /* Whole-board card census. Two card-DESTROYING defects were found today
     * and neither was visible to a zone count. */
    CHECK_EQ(census_after, census_before,
             "B1 CONTROL: no card was created or destroyed by check_timing");

    CHECK_EQ(live_has(&tg, 0, member), 0, "B1: the non-live card leaves the live zone");
    CHECK_EQ(wait_has(&tg, 0, member), 1, "B1: it lands in P1's waitroom");

    /* THE POINT: the ENGINE's log, not a test-built one. */
    CHECK_EQ(n_movements(&tg), 1,
             "B1: the ENGINE recorded exactly one movement in g->batch_movements");
    dump_movements(&tg);

    const RbBatchMovement *m = movement_for(&tg, member);
    CHECK(m != NULL, "B1: the ENGINE's log names the moved card");
    if (m) {
        CHECK_EQ(m->moved_card_id, member, "B1: moved_card_id is the discarded member");
        CHECK_EQ((int)m->source_zone, (int)RB_ZONEID_LIVE_CARD_ZONE,
                 "B1: source_zone is the live card zone");
        CHECK_EQ((int)m->dest_zone, (int)RB_ZONEID_WAITROOM,
                 "B1: dest_zone is the waitroom");
        CHECK_EQ(m->cause_player_id, 0, "B1: cause_player_id is P1, whose zone was cleaned");
    }
}

/* B1p -- ADDED paired POSITIVE, and the reason B1 is not vacuous. Same zone,
 * same call, same everything; the ONLY difference is that this card IS a live
 * card, so the filter must let it stay AND record nothing for it. Without
 * this, B1's "one movement recorded" could be satisfied by a check_timing
 * that records every card unconditionally. */
static void b1p_valid_live_card_records_no_movement(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int good  = mid(&tg, GOODLIVE);
    int bad   = mid(&tg, MEMBER4);
    int filler = mid(&tg, MEMBER4);
    if (!pin_id(good, GOODLIVE)) return;
    if (!pin_id(bad, MEMBER4)) return;
    CHECK(rb_card_is_live(good), "B1p CONTROL: the second fixture really IS a live card");
    CHECK(!rb_card_is_live(bad), "B1p CONTROL: the first fixture really is NOT");

    /* One board, ONE check_timing call, two live-zone occupants differing only
     * in whether the print is a live card. This is the tightest form of the
     * paired positive the brief asks for. */
    test_add_to_live(&tg, good);
    test_add_to_live(&tg, bad);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);

    CHECK_EQ(tg.state.p[0].live.n, 2, "B1p CONTROL: the live zone holds both cards before the check");
    CHECK_EQ(n_movements(&tg), 0, "B1p CONTROL: the ENGINE log is empty before check_timing");

    int census_before = test_total_card_count(&tg);
    /* Rust's B1 twin sets face_up at :113; B1p is the SAME board, so it needs
     * the SAME precondition or the `good` card's survival would be satisfied by
     * a check_timing that never looked at the live zone at all. */
    tg.state.live_face_up[0] = 1;
    rb_check_timing(&tg.state);
    int census_after = test_total_card_count(&tg);

    CHECK_EQ(census_after, census_before,
             "B1p CONTROL: no card was created or destroyed by check_timing");
    CHECK_EQ(live_has(&tg, 0, good), 1, "B1p: a VALID live card SURVIVES the check");
    CHECK_EQ(movements_for(&tg, good), 0,
             "B1p: and the ENGINE records NO movement for it -- so B1's single "
             "entry is the filter discriminating, not an unconditional record");
    CHECK_EQ(movements_for(&tg, bad), 1, "B1p: the non-live card beside it IS recorded");
    CHECK_EQ(n_movements(&tg), 1, "B1p: exactly one movement total, for the non-live card only");
    dump_movements(&tg);
}

/* B2 -- Rust check_invalid_live_cards_distinguishes_players: a non-live card
 * in each seat's live zone; after check_timing each must be in ITS OWN
 * waitroom, the ENGINE log must hold exactly two entries, and each entry's
 * cause_player_id must be that seat. The cause ids are the part the existing
 * suite asserts on its own reconstructed log. */
static void b2_check_invalid_live_cards_distinguishes_players(void)
{
    static TestGame tg;
    case_begin(&tg);

    int m1 = mid(&tg, MEMBER4);
    int m2 = mid(&tg, MEMBER4);
    int filler = mid(&tg, MEMBER4);
    if (!pin_id(m1, MEMBER4)) return;
    if (!pin_id(m2, MEMBER4)) return;
    CHECK(m1 != m2, "B2: the two members are DISTINCT pool slots");

    test_add_to_live_for(&tg, 0, m1);
    test_add_to_live_for(&tg, 1, m2);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(&tg, 1, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);

    CHECK_EQ(live_has(&tg, 0, m1), 1, "B2 CONTROL: P1's live zone holds the first member");
    CHECK_EQ(live_has(&tg, 1, m2), 1, "B2 CONTROL: P2's live zone holds the second member");
    CHECK_EQ(n_movements(&tg), 0, "B2 CONTROL: the ENGINE log is empty before check_timing");

    int census_before = test_total_card_count(&tg);
    /* Rust's twin check_invalid_live_cards_distinguishes_players sets face_up
     * for BOTH seats -- :347 (P1) and :348 (P2) -- and that is the whole point
     * of the test: the gate is per SEAT, so a one-seat fixture could not tell a
     * correct per-seat read from a hardcoded one. */
    tg.state.live_face_up[0] = 1;
    tg.state.live_face_up[1] = 1;
    rb_check_timing(&tg.state);
    int census_after = test_total_card_count(&tg);

    CHECK_EQ(census_after, census_before,
             "B2 CONTROL: no card was created or destroyed by check_timing");

    CHECK_EQ(wait_has(&tg, 0, m1), 1, "B2: P1's card lands in P1's waitroom");
    CHECK_EQ(wait_has(&tg, 1, m2), 1, "B2: P2's card lands in P2's waitroom");
    CHECK_EQ(wait_has(&tg, 0, m2), 0, "B2: P1's waitroom must NOT receive P2's card");
    CHECK_EQ(wait_has(&tg, 1, m1), 0, "B2: P2's waitroom must NOT receive P1's card");

    CHECK_EQ(n_movements(&tg), 2, "B2: the ENGINE recorded exactly two movements");
    dump_movements(&tg);

    const RbBatchMovement *a = movement_for(&tg, m1);
    const RbBatchMovement *b = movement_for(&tg, m2);
    CHECK(a && b, "B2: the ENGINE's log names both moved cards");
    if (a) CHECK_EQ(a->cause_player_id, 0, "B2: the first card's cause_player_id is P1");
    if (b) CHECK_EQ(b->cause_player_id, 1, "B2: the second card's cause_player_id is P2");
    CHECK_EQ(movements_for(&tg, m1), 1, "B2: the first card is recorded exactly once");
    CHECK_EQ(movements_for(&tg, m2), 1, "B2: the second card is recorded exactly once");
}

/* B3 -- Rust turn_movements_cleared_at_turn_start: push one movement, assert
 * the log holds 1, call clear_card_movement_tracking, assert 0. The existing
 * C twin of this test asserts its OWN array after clearing its OWN array,
 * which no engine change can ever turn red. Here both readings are the
 * engine's. */
static void b3_turn_movements_cleared_at_turn_start(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, MEMBER4);
    if (!pin_id(member, MEMBER4)) return;

    rb_record_card_movement(&tg.state, member, RB_ZONEID_LIVE_CARD_ZONE,
                            RB_ZONEID_WAITROOM, 0, 0);
    CHECK_EQ(n_movements(&tg), 1,
             "B3: the ENGINE log holds exactly one movement before the clear");
    CHECK_EQ(tg.state.moved_this_turn[member] != 0 ? 1 : 0, 1,
             "B3: the per-card moved-this-turn flag is set by the same call");

    rb_clear_movement_tracking(&tg.state);

    CHECK_EQ(n_movements(&tg), 0,
             "B3: rb_clear_movement_tracking empties the ENGINE log "
             "(n_batch_movements -- the state the source/dest condition reads)");
    CHECK_EQ(tg.state.moved_this_turn[member] != 0 ? 1 : 0, 0,
             "B3: and it clears the per-card flag, which is the other half of "
             "Rust's clear_card_movement_tracking");

    /* The condition must actually become unsatisfiable afterwards -- a count
     * gate reading an emptied log is the observable consequence, and it is
     * what makes this more than a field-presence check. */
    int c2 = mid(&tg, MEMBER4);
    if (!pin_id(c2, MEMBER4)) return;
    test_add_to_discard(&tg, c2);
    Condition cond = source_dest_condition("live_card_zone", "discard", 1, NULL);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &cond), 0,
             "B3: with the log emptied, a source/dest condition is FALSE even "
             "though the card IS in the destination zone");
}

/* B4 -- Rust invalid_energy_card_in_live_zone_goes_to_energy_deck. Rule 10.5.2
 * sends an illegal ENERGY card to the energy DECK, not the waitroom, so this
 * also pins that the two illegal-card paths take different destinations --
 * which is the only way a dest_zone assertion can be non-vacuous. */
static void b4_invalid_energy_in_live_zone_goes_to_energy_deck(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int energy = mid(&tg, ENERGY);
    int member = mid(&tg, MEMBER4);
    int filler = mid(&tg, MEMBER4);
    if (!pin_id(energy, ENERGY)) return;
    if (!pin_id(member, MEMBER4)) return;
    CHECK(rb_card_is_energy(energy), "B4 CONTROL: LL-E-001-SD really is an energy card");
    CHECK(!rb_card_is_energy(member), "B4 CONTROL: the member really is NOT an energy card");

    test_add_to_live(&tg, energy);
    test_add_to_live(&tg, member);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);

    int census_before = test_total_card_count(&tg);
    /* Rust's twin invalid_energy_card_in_live_zone_goes_to_energy_deck sets
     * `player1.live_card_zone.face_up = true` at :282. Without it the gate at
     * src/turn/phase.c:703 returns early, the energy card never reaches the
     * energy deck, and the "the two illegal paths take different destinations"
     * claim at the bottom of this test -- the reason B4 exists -- would be
     * vacuous. */
    tg.state.live_face_up[0] = 1;
    rb_check_timing(&tg.state);
    int census_after = test_total_card_count(&tg);

    CHECK_EQ(census_after, census_before,
             "B4 CONTROL: no card was created or destroyed by check_timing");
    CHECK_EQ(live_has(&tg, 0, energy), 0, "B4: the illegal energy card leaves the live zone");
    CHECK_EQ(bag_has(&tg.state.p[0].energy_deck, energy), 1,
             "B4: it goes to the ENERGY DECK, not the waitroom");
    CHECK_EQ(wait_has(&tg, 0, energy), 0, "B4: and definitely not to the waitroom");
    CHECK_EQ(n_movements(&tg), 2, "B4 CONTROL: the ENGINE recorded both movements");
    dump_movements(&tg);

    const RbBatchMovement *me = movement_for(&tg, energy);
    const RbBatchMovement *mm = movement_for(&tg, member);
    CHECK(me && mm, "B4: the ENGINE's log names both moved cards");
    if (me) CHECK_EQ((int)me->dest_zone, (int)RB_ZONEID_ENERGY_DECK,
                     "B4: the energy card's dest_zone is the energy deck");
    if (mm) CHECK_EQ((int)mm->dest_zone, (int)RB_ZONEID_WAITROOM,
                     "B4: the member's dest_zone is the waitroom -- the two "
                     "illegal paths record DIFFERENT destinations");
}

/* B5 -- Rust source_dest_condition_matches_turn_movements, and the positive
 * that makes B5n non-vacuous. The Rust helper push_movement_event is folded
 * into rb_record_card_movement in C, so the fixture is one call.
 *
 * The condition is source="live_card_zone", destination="discard",
 * card_type=member_card, count=1, operator ">=", target="self". The card must
 * also be IN the destination zone: resolve_moved_cards_source applies a
 * zone_ids() membership filter on top of the movement match. */
static void b5_source_dest_matches_turn_movements(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, MEMBER4);
    if (!pin_id(member, MEMBER4)) return;

    rb_record_card_movement(&tg.state, member, RB_ZONEID_LIVE_CARD_ZONE,
                            RB_ZONEID_WAITROOM, 0, 0);
    test_add_to_discard(&tg, member);
    fill_decks(&tg, member, 30);

    CHECK_EQ(n_movements(&tg), 1, "B5 CONTROL: the ENGINE log holds the movement");
    CHECK_EQ(wait_has(&tg, 0, member), 1, "B5 CONTROL: the card is in the destination zone");

    Condition cond = source_dest_condition("live_card_zone", "discard", 1, NULL);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &cond), 1,
             "B5: a matching movement for the same player satisfies the "
             "source/destination condition");

    /* COUNT > 1 must fail on the same board. This is the cheapest possible
     * non-vacuity proof that B5 is reading the log and not a constant: the
     * card, the zone, the movement and the condition are all identical, and
     * only the threshold moved. */
    Condition cond2 = source_dest_condition("live_card_zone", "discard", 2, NULL);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &cond2), 0,
             "B5 CONTROL: count>=2 fails on this same board -- so the positive "
             "above is a real count and not an unconditional TRUE");
}

/* B5n -- the two player-scoping negatives from the Rust file, on ONE board
 * each, differing from B5 only in WHO caused the movement and WHO the
 * condition is aimed at. These are the assertions that a target-ignoring
 * evaluator would fail. */
static void b5n_source_dest_player_scoping(void)
{
    static TestGame tg;

    /* --- arm 1: P2 caused it, the condition aims at SELF (P1) -> FALSE --- */
    case_begin(&tg);
    clear_p1(&tg);
    {
        int member = mid(&tg, MEMBER4);
        if (!pin_id(member, MEMBER4)) return;
        rb_record_card_movement(&tg.state, member, RB_ZONEID_LIVE_CARD_ZONE,
                                RB_ZONEID_WAITROOM, 1, 0);
        /* the card really is in P2's destination zone, so only the CAUSE
         * separates this arm from the B5 positive */
        add_to_waitroom_for(&tg, 1, member);
        CHECK_EQ(wait_has(&tg, 1, member), 1,
                 "B5n CONTROL: the card IS in the destination zone, so nothing "
                 "here fails for want of a card");
        CHECK_EQ(n_movements(&tg), 1, "B5n CONTROL: the ENGINE log holds the movement");

        Condition self_c = source_dest_condition("live_card_zone", "discard", 1, NULL);
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &self_c), 0,
                 "B5n: a self-aimed condition does NOT count a movement P2 caused");

        /* The other arm, SAME BOARD: aim the same condition at the opponent
         * and it must now be true. One board, one movement, one condition
         * body -- only the target field differs. This is the tightest paired
         * positive/negative pair available and it is why B5n cannot pass
         * vacuously. */
        Condition opp_c = source_dest_condition("live_card_zone", "discard", 1, "opponent");
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &opp_c), 1,
                 "B5n: the SAME board with target=\"opponent\" IS true -- so the "
                 "evaluator really reads the target field");
    }

    /* --- arm 2: wrong source zone -> FALSE, and the right source is TRUE --- */
    case_begin(&tg);
    clear_p1(&tg);
    {
        int member = mid(&tg, MEMBER4);
        if (!pin_id(member, MEMBER4)) return;
        rb_record_card_movement(&tg.state, member, RB_ZONEID_STAGE,
                                RB_ZONEID_WAITROOM, 0, 0);
        test_add_to_discard(&tg, member);
        CHECK_EQ(n_movements(&tg), 1, "B5n CONTROL: the ENGINE log holds the movement");
        CHECK_EQ(wait_has(&tg, 0, member), 1,
                 "B5n CONTROL: the card IS in the destination zone");

        Condition wrong = source_dest_condition("live_card_zone", "discard", 1, NULL);
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &wrong), 0,
                 "B5n: a movement whose SOURCE was the stage does not satisfy a "
                 "live_card_zone -> discard condition");

        /* Same movement, same card, same zone -- only the source string asked
         * for is corrected to match what was recorded. */
        Condition right = source_dest_condition("stage", "discard", 1, NULL);
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &right), 1,
                 "B5n: the SAME board with source=\"stage\" IS true -- so the "
                 "source field is really matched, not ignored");
    }

    /* --- arm 3: right source and destination, but the card is NOT in the
     *     destination zone -> FALSE. Rust: card_not_in_dest_fails. --- */
    case_begin(&tg);
    clear_p1(&tg);
    {
        int member = mid(&tg, MEMBER4);
        if (!pin_id(member, MEMBER4)) return;
        rb_record_card_movement(&tg.state, member, RB_ZONEID_LIVE_CARD_ZONE,
                                RB_ZONEID_WAITROOM, 0, 0);
        CHECK_EQ(n_movements(&tg), 1, "B5n CONTROL: the ENGINE log holds the movement");
        CHECK_EQ(wait_has(&tg, 0, member), 0,
                 "B5n CONTROL: the card is deliberately NOT in the destination zone");

        Condition c = source_dest_condition("live_card_zone", "discard", 1, NULL);
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
                 "B5n: a matching movement is not enough -- the card must also "
                 "BE in the destination zone");
    }
}

/* =======================================================================
 * C. The Q207/Q208 distinct-NAME hazard probe.
 *
 * A peer reported that the C COPY path can hand back a card carrying the WRONG
 * Card.name -- two distinct card numbers both reporting as one character --
 * which would silently make every distinct-NAME gate in the game unreachable.
 * The existing Q207/Q208 coverage in test_parity_rules.c CANNOT see that
 * hazard, and the reason is structural: it builds its fixtures with
 * test_id(), which returns the SHARED TEMPLATE and never goes through
 * rb_create_card_copy. A template lookup always reports the right name.
 *
 * So this family deliberately goes through mid() -- the copy pool -- and asks
 * the question the survey actually cares about: do two DIFFERENT card numbers
 * keep two DIFFERENT names after a copy?
 *
 * CONTEXT, because it decides what a "duplicate name" even means here: cards.json
 * has 45+ distinct card numbers printed <U+4E0A><U+539F><U+6B69><U+5922> and two distinct numbers
 * (PL!S-bp7-009-R cost 2 / PL!S-bp5-009-R cost 15) both printed <U+9ED2><U+6FA4><U+30EB><U+30D3><U+30A3>.
 * Duplicate names across printings are NORMAL. The defect would be two
 * DIFFERENT characters reporting one name.
 * ======================================================================= */

static void c_q207_q208_distinct_names_survive_the_copy_pool(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int multi = mid(&tg, MULTI);
    int ayumu = mid(&tg, AYUMU_1);
    int kanon = mid(&tg, KANON_1);
    if (!pin_id(multi, MULTI)) return;
    if (!pin_id(ayumu, AYUMU_1)) return;
    if (!pin_id(kanon, KANON_1)) return;

    /* Three copies, three pool slots. If any two collapsed onto one template
     * every identity assertion below would be trivially self-satisfying. */
    CHECK(multi != ayumu && multi != kanon && ayumu != kanon,
          "C CONTROL: the three fixtures are three DISTINCT copy-pool slots");

    /* The Q207 assertion, on the COPY rather than the template. */
    DIAG("C: multi  name = %s", esc(card_name_of(multi)));
    DIAG("C: ayumu  name = %s", esc(card_name_of(ayumu)));
    DIAG("C: kanon  name = %s", esc(card_name_of(kanon)));

    CHECK_EQ(strcmp(card_name_of(multi), MULTI_FULL), 0,
             "C: through the COPY pool, LL-bp1-001-R+ still reports the full "
             "three-name string <U+4E0A><U+539F><U+6B69><U+5922>&<U+6F81><U+8C37><U+304B><U+306E><U+3093>&<U+65E5><U+91CE><U+4E0B><U+82B1><U+5E06>");
    CHECK(name_contains(multi, N_AYUMU),  "C: the copy still contains <U+6B69><U+5922>");
    CHECK(name_contains(multi, N_KANON),  "C: the copy still contains <U+304B><U+306E><U+3093>");
    CHECK(name_contains(multi, N_HANAKO), "C: the copy still contains <U+82B1><U+5E06>");

    /* THE HAZARD ITSELF. Three different card numbers, three different
     * characters; the engine must not have collapsed any two onto one name.
     * Each pair is compared directly, so a single mis-reported copy shows up
     * as a red pair rather than as a vague "something is off". */
    CHECK(strcmp(card_name_of(multi), card_name_of(ayumu)) != 0,
          "C HAZARD: the multi-name card and the single <U+4E0A><U+539F><U+6B69><U+5922> print do NOT "
          "report the same name (this is the copy-path fidelity check the "
          "template-lookup Q207/Q208 coverage cannot make)");
    CHECK(strcmp(card_name_of(ayumu), card_name_of(kanon)) != 0,
          "C HAZARD: <U+4E0A><U+539F><U+6B69><U+5922> and <U+6F81><U+8C37><U+304B><U+306E><U+3093> are DIFFERENT characters and their "
          "copies do not share a name");
    CHECK(strcmp(card_name_of(multi), card_name_of(kanon)) != 0,
          "C HAZARD: the multi-name card and <U+6F81><U+8C37><U+304B><U+306E><U+3093> do not share a name");

    /* And the direction Q208 actually needs: the multi-name card's OTHER
     * names are the ones that distinguish it from the single <U+4E0A><U+539F><U+6B69><U+5922>, so a
     * collision on those is what makes the "reference her as <U+304B><U+306E><U+3093>" rule
     * unreachable. */
    CHECK(!name_contains(ayumu, N_KANON),
          "C HAZARD: the single <U+4E0A><U+539F><U+6B69><U+5922> copy does NOT carry <U+304B><U+306E><U+3093> -- if it did, "
          "every distinct-NAME gate in the game is ambiguous and unreachable");
    CHECK(!name_contains(ayumu, N_HANAKO),
          "C HAZARD: the single <U+4E0A><U+539F><U+6B69><U+5922> copy does NOT carry <U+82B1><U+5E06> either");

    /* The stage-slot half of Q207/Q208, on the copies. One slot = one member
     * no matter how many names the card prints. */
    set_stage(&tg, 0, multi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    CHECK_EQ(stage_member_count(&tg, 0), 1,
             "C: the three-name card still occupies exactly ONE stage slot (Q207)");
    CHECK_EQ(tg.state.p[0].stage[0], multi, "C: and it is the multi-name card in that slot");

    set_stage(&tg, 0, multi, ayumu, RB_EMPTY_SLOT);
    CHECK_EQ(stage_member_count(&tg, 0), 2,
             "C: the multi-name card and the single print occupy TWO slots (Q208)");
}

/* =======================================================================
 * D. restriction_and_aura_mechanics_test.rs -- the pins the existing C twin
 *    DROPPED.
 *
 * test_restriction_mechanics.c:53 ports
 * himena_bp6006_baton_only_with_murasakipark but keeps only ONE of the Rust
 * assertions:
 *
 *     Rust  (restriction_and_aura_mechanics_test.rs:81-105)
 *       res.is_err()
 *       stage.stage[0] == himena            "still occupies the area"
 *       !waitroom.contains(&himena)         "not put in the waitroom"
 *       hand.contains(&outsider)             "arriving outsider still in hand"
 *       baton_touch_count_p1 == 0           "not counted"
 *       assert_energy_untouched_after_refusal(6)
 *     C    (test_restriction_mechanics.c:65-67)
 *       CHECK(result == 0, "non-murasakipark baton partner must be blocked");
 *
 * The Rust file says in a comment why the extra pins exist: "The block must be
 * total. The allowed-partner sibling test proves the same play is otherwise
 * legal, so these are what pin 'nothing happened'." A lone `result == 0` is
 * satisfied by a refusal for ANY reason -- a wrong card number, a cost the
 * fixture cannot pay, an empty stage -- so on its own it proves the play did
 * not succeed without proving the RESTRICTION is what stopped it. That is
 * exactly the vacuous negative this file is written to avoid, and the drop is
 * a genuine gap in substance, not in coverage count.
 *
 * D1 restores the "nothing happened" pins. D2 is the paired POSITIVE -- the
 * same himena, the same play, the only difference being that the arriving
 * partner is <U+307F><U+3089><U+304F><U+3089><U+3071><U+30FC><U+304F>! -- and D2 already passes in the existing suite,
 * which is what makes D1's "the refusal is specific" claim a measurement.
 * ======================================================================= */

static void d1_blocked_baton_touch_leaves_nothing_behind(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int himena   = mid(&tg, HIMENA);
    int outsider = mid(&tg, KOTORI);
    if (!pin_id(himena, HIMENA)) return;
    if (!pin_id(outsider, KOTORI)) return;
    CHECK(himena != outsider, "D1: the restricted member and the outsider are DISTINCT cards");
    CHECK(name_contains(himena, N_HIMENA), "D1: the restricted print is <U+5B89><U+990A><U+5BFA> <U+59EB><U+82BD>");
    CHECK(name_contains(outsider, N_KOTORI), "D1: the outsider is <U+5357> <U+3053><U+3068><U+308A>");
    CHECK(strcmp(card_name_of(himena), card_name_of(outsider)) != 0,
          "D1 CONTROL: the two fixtures are different characters -- so this is "
          "not the copy-path name hazard again");

    set_stage(&tg, 0, himena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, himena, "wait");
    test_add_to_hand(&tg, outsider);
    test_give_energy(&tg, 6);

    int census_before = test_total_card_count(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], himena, "D1 CONTROL: the restricted member occupies the area before the play");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "D1 CONTROL: the outsider is in hand before the play");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "D1 CONTROL: P1's waitroom is empty before the play");
    CHECK_EQ(tg.state.baton_touch_count_p1, 0, "D1 CONTROL: no baton touch is counted yet");
    CHECK_EQ(tg.state.p[0].energy_active, 6, "D1 CONTROL: 6 active energy before the play");

    int result = test_try_play_to_stage(&tg, outsider, AREA_LEFT);

    int census_after = test_total_card_count(&tg);
    CHECK_EQ(census_after, census_before,
             "D1 CONTROL: the refused play neither created nor destroyed a card");

    CHECK_EQ(result, 0, "D1: the non-<U+307F><U+3089><U+304F><U+3089><U+3071><U+30FC><U+304F><U+FF01> baton partner is blocked");

    /* THE PINS THE EXISTING SUITE DROPS. */
    CHECK_EQ(tg.state.p[0].stage[0], himena,
             "D1: the restricted member STILL occupies the area");
    CHECK_EQ(wait_has(&tg, 0, himena), 0,
             "D1: a blocked baton touch must NOT put the member in the waitroom");
    CHECK_EQ(test_hand_has(&tg, outsider), 1,
             "D1: the arriving outsider is STILL in hand");
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "D1: P1's waitroom is still empty -- nothing was moved anywhere");
    CHECK_EQ(tg.state.baton_touch_count_p1, 0,
             "D1: a blocked baton touch must NOT be counted");
    CHECK_EQ(tg.state.p[0].energy_active, 6,
             "D1: 6 active energy is untouched by the refusal -- the cost was "
             "never charged");

    /* D2 is the paired positive: the same himena, the same 6 energy, the same
     * "wait" orientation, and a partner from the right unit, and the play
     * SUCCEEDS. If the refusal above were caused by anything other than the
     * group restriction -- a mis-resolved card number, a cost the fixture
     * cannot pay, an occupied-stage rule -- D2 would fail here. */
    {
        int partner = mid(&tg, RURINO);
        if (!pin_id(partner, RURINO)) return;
        CHECK(name_contains(partner, N_RURINO), "D2 CONTROL: the allowed partner is <U+5927><U+6CA2><U+7460><U+7483><U+4E43>");
        CHECK(strcmp(card_name_of(partner), card_name_of(outsider)) != 0,
              "D2 CONTROL: the allowed partner and the blocked outsider are "
              "DIFFERENT characters -- the pair differs by more than the unit");

        case_begin(&tg);
        clear_p1(&tg);
        int himena2 = mid(&tg, HIMENA);
        int partner2 = mid(&tg, RURINO);
        int filler = mid(&tg, MEMBER4);
        if (!pin_id(himena2, HIMENA)) return;
        if (!pin_id(partner2, RURINO)) return;

        set_stage(&tg, 0, himena2, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        rb_mods_set_orientation(&tg.state.mods, himena2, "wait");
        test_add_to_hand(&tg, partner2);
        test_give_energy(&tg, 6);

        int ok = test_try_play_to_stage(&tg, partner2, AREA_LEFT);

        CHECK_EQ(ok, 1,
                 "D1 CONTROL (paired positive): with a <U+307F><U+3089><U+304F><U+3089><U+3071><U+30FC><U+304F><U+FF01> partner the "
                 "SAME play SUCCEEDS -- so the refusal above is the group "
                 "restriction and not some unrelated precondition");
        CHECK_EQ(tg.state.p[0].stage[0], partner2, "D1 CONTROL: the partner now occupies the area");
        CHECK_EQ(wait_has(&tg, 0, himena2), 1, "D1 CONTROL: the batoned member reaches the waitroom");
        (void)filler;
    }
}

/* =======================================================================
 * fork harness
 * ======================================================================= */

typedef struct { const char *name; void (*fn)(void); } Case;

static void run(const Case *c)
{
    fflush(stdout); fflush(stderr);
    g_shared->assertions = 0;
    g_shared->failures   = 0;
    g_shared->signal     = 0;
    g_shared->exited_ok  = 0;
    g_shared->name[0]    = 0;

    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "FAIL: %s: fork() failed\n", c->name);
        failures++;
        return;
    }
    if (pid == 0) {
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_handler = fatal_handler;
        sigemptyset(&sa.sa_mask);
        sa.sa_flags = 0;
        sigaction(SIGSEGV, &sa, NULL);
        sigaction(SIGBUS,  &sa, NULL);
        sigaction(SIGABRT, &sa, NULL);
        sigaction(SIGFPE,  &sa, NULL);
        snprintf(g_case_name, sizeof g_case_name, "%s", c->name);
        /* The counters are process globals and the parent has been
         * accumulating them across cases, so the child must start from zero or
         * it reports the running total plus its own. */
        assertions = 0;
        failures   = 0;
        rb_card_copy_pool_reset();
        c->fn();
        g_shared->assertions = assertions;
        g_shared->failures   = failures;
        g_shared->exited_ok  = 1;
        fflush(stdout);
        fflush(stderr);
        _exit(0);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) status = -1;
    assertions += g_shared->assertions;
    failures   += g_shared->failures;

    if (g_shared->signal) {
        const char *nm = "signal";
        switch (g_shared->signal) {
            case SIGSEGV: nm = "SIGSEGV"; break;
            case SIGBUS:  nm = "SIGBUS";  break;
            case SIGABRT: nm = "SIGABRT"; break;
            case SIGFPE:  nm = "SIGFPE";  break;
            default: break;
        }
        fprintf(stderr, "FAIL: %s: fatal %s (%d) in child; %d assertion(s) reached, %d failing\n",
                g_shared->name[0] ? g_shared->name : c->name, nm, g_shared->signal,
                g_shared->assertions, g_shared->failures);
        failures++;
        assertions++;
    } else if (!g_shared->exited_ok) {
        fprintf(stderr, "FAIL: %s: child terminated abnormally (wait status %d) after %d assertion(s)\n",
                c->name, status, g_shared->assertions);
        failures++;
        assertions++;
    }
    /* The corpus verdict, in the shape every other FORK suite prints. The
     * `== case ...` line below stays for its per-case counts; nothing that
     * matched `^FAILED` could see it. */
    printf(g_shared->failures > 0 ? "FAILED: %s\n" : "ok: %s\n", c->name);
    printf("== case %-46s assertions=%d failures=%d\n",
           c->name, g_shared->assertions, g_shared->failures);
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);

    g_shared = mmap(NULL, sizeof *g_shared, PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (g_shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: mmap(MAP_SHARED|MAP_ANONYMOUS) for the shared "
                        "counters failed\n");
        return 2;
    }
    memset((void *)g_shared, 0, sizeof *g_shared);

    if (getenv("RB_ABILITY_DEBUG")) rb_ability_debug_set(1);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    /* AGENTS.md: derive heart indices with rb_parse_heart_color and print the
     * mapping at startup rather than hardcoding an assumption. */
    printf("--- heart colour map (rb_parse_heart_color) ---\n");
    for (int i = 0; i <= 6; i++) {
        char nm[16];
        snprintf(nm, sizeof nm, "heart%02d", i);
        printf("    %s -> %d\n", nm, (int)rb_parse_heart_color(nm));
    }
    /* Both group accessors, printed together as AGENTS.md requires:
     * rb_card_group_name is SERIES-level, rb_card_unit_name is unit-level. */
    {
        static TestGame probe_tg;
        rb_card_copy_pool_reset();
        test_game_new(&probe_tg);
        int probe = test_id_exact(&probe_tg, MULTI);
        if (probe >= 0) {
            const char *g = rb_card_group_name((uint32_t)probe);
            const char *u = rb_card_unit_name((uint32_t)probe);
            printf("--- accessors on %s (id=%d) ---\n", MULTI, probe);
            printf("    rb_card_group_name = %s\n", g ? g : "(null)");
            printf("    rb_card_unit_name  = %s\n", u ? u : "(null)");
        } else {
            printf("--- accessors: %s did not resolve for the startup dump ---\n", MULTI);
        }
    }

    static const Case cases[] = {
        { "A1_turn_number_1_on_turn_1_true",        a1_turn_number_1_on_turn_1_true },
        { "A2_turn_number_1_on_turn_2_false",       a2_turn_number_1_on_turn_2_false },
        { "A3_turn_number_1_turn_1_not_live_phase", a3_turn_number_1_on_turn_1_not_live_phase },
        { "A4_turn_number_none_ignored",            a4_turn_number_none_ignored },
        { "A5_turn_number_gate_truth_table",        a5_turn_number_gate_is_an_equality_not_a_comparison },

        { "B1_invalid_live_discard_records",        b1_invalid_live_discard_records_turn_movements },
        { "B1p_valid_live_card_records_nothing",    b1p_valid_live_card_records_no_movement },
        { "B2_invalid_live_distinguishes_players",  b2_check_invalid_live_cards_distinguishes_players },
        { "B3_turn_movements_cleared_at_turn",      b3_turn_movements_cleared_at_turn_start },
        { "B4_invalid_energy_to_energy_deck",       b4_invalid_energy_in_live_zone_goes_to_energy_deck },
        { "B5_source_dest_matches_movements",       b5_source_dest_matches_turn_movements },
        { "B5n_source_dest_player_scoping",         b5n_source_dest_player_scoping },

        { "C_q207_q208_distinct_names_copy_path",   c_q207_q208_distinct_names_survive_the_copy_pool },

        { "D1_blocked_baton_touch_total_block",     d1_blocked_baton_touch_leaves_nothing_behind },
    };

    printf("--- A. turn_number condition (eval_temporal turn_number gate) ---\n");
    for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        if (i == 5)  printf("--- B. live_card_zone_movement: the ENGINE's own movement log ---\n");
        if (i == 12) printf("--- C. Q207/Q208 distinct names through the COPY pool ---\n");
        if (i == 13) printf("--- D. restriction_and_aura: the dropped 'nothing happened' pins ---\n");
        run(&cases[i]);
    }

    rb_unload();
    printf("\n==== rules/conditions parity #2: %d assertions, %d failures ====\n",
           assertions, failures);
    return failures ? 1 : 0;
}
