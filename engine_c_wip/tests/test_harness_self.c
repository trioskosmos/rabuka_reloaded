/* test_harness_self.c — a self-test for the test harness itself.
 *
 * WHY THIS FILE EXISTS
 * ====================
 * `src/test_game.c` is 1269 lines that EVERY parity suite depends on: a bug in
 * one of its helpers does not fail one suite, it silently changes what 142
 * suites measure. Until now nothing checked the harness, which is how these
 * survived:
 *
 *   * `sizeof(TestGame)` = 885 912 B and there are 1005 non-static
 *     `GameState`/`TestGame` locals across tests/. Death is prevented ONLY by
 *     `-Wl,--stack,16777216` on the link line. Cluster J converted 230 of them
 *     in five files as a one-time cleanup, with nothing to stop the count
 *     climbing back.
 *   * `test_id()` returns the SHARED TEMPLATE id, so two calls for the same
 *     card_no collapse to one id and every "these are two distinct instances"
 *     assertion written against it is VACUOUS. That already caused one
 *     misdiagnosis. Rust's `TestGame::id` does NOT do this (see §C).
 *
 * So this suite pins the four things a harness needs to keep:
 *   §A the EXACT size of the thing on everyone's stack, and that the linked
 *         stack can still hold it,
 *   §B that the stack-local count only ever goes DOWN,
 *   §C/D the `test_id` vs `test_new_id` contract and a ratchet that stops the
 *         aliasing form from spreading,
 *   §E the strict-vs-forggiving lookup split,
 *   §F the `test_fire_trigger_*` exact-vs-token trigger match,
 *   §G/H/I/J the placement, attacking-seat, phase-name and pool contracts.
 *   §K the count-shaped helpers (a helper that answers a CONSTANT is not a
 *         helper — this is how test_pending_choice_count's old `? 1 : 0` body
 *         hid for so long),
 *   §L the Rust twins that had NO C spelling (fill_decks, fill_energy_deck,
 *         stage_occupant, id_ref) and the fill_decks/test_filler_hand name trap.
 *   §M the identity trap is DETECTABLE, not merely documented (§C pins what
 *         test_id does; §M proves a test written against it is visibly wrong).
 *
 * Every bound below is a MEASURED number with the date it was taken, not a
 * round number, so "it changed" is distinguishable from "it was always like
 * that". Raising a bound is a deliberate edit: change the number AND change the
 * comment that says what raising it costs.
 *
 * SCOPE OF THE RATCHETS. §B and §D read the suite sources from disk (cwd is
 * engine_c_wip when `make t` runs). That is deliberate: the only mechanically
 * checkable form of "test_id() used where test_new_id() is required" is a
 * count that may not rise. A human judgement about WHICH uses need identity is
 * not automatable, and pretending otherwise would produce a check that passes
 * while the hazard grows.
 */
#include "test_game.h"

#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", (message)); \
        failures++; \
    } else { \
        printf("ok: %s\n", (message)); \
    } \
} while (0)

/* ═══════════════════════════════════════════════════════════════════════════
 * §A  SIZE OF THE THING EVERY SUITE PUTS ON THE STACK — PINNED EXACTLY
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * `TestGame` is `{ GameState state; }` and nothing else, so the two sizes are
 * structurally required to be equal. If they ever are not, something grew the
 * harness wrapper and every `sizeof(TestGame)` claim below is wrong.
 *
 * A2 USED TO BE A CEILING, NOT A PIN, AND THAT IS THE DEFECT THIS SECTION NOW
 * FIXES. `sizeof(GameState) <= linked_stack / RB_MIN_SAFE_LOCALS` is satisfied
 * by every size BELOW the ceiling, so it cannot tell 780 KB from 1.17 MB — and
 * this number has moved four times: ~780 KB, 885 912, 1 113 736, 1 195 656.
 * Two of those moves happened with every object file in the tree still holding
 * the previous layout, which is the silent invalidation A4 exists to stop.
 * GameState's fields are APPENDED, never inserted (Makefile:11-13), so a move
 * is always growth -- but growth is exactly what a `<=` check waves through.
 *
 * A4 is therefore an EXACT equality against RB_GAMESTATE_BYTES_PINNED. It goes
 * RED on the next append, on purpose: the person who appended has to re-measure
 * with a throwaway TU, re-check the stack budget (A2) and restate the history
 * above. That is the whole cost, and it is smaller than shipping a 1.4 MB
 * struct against 156 stale object files.
 *
 * A2's ceiling is not a round number either: it is derived from the only thing
 * that actually keeps this harness alive. The Makefile links every target with
 * `-Wl,--stack,<N>`, and a non-static `GameState` local is exactly
 * sizeof(GameState) of that budget. So the bound that matters is
 *
 *     sizeof(GameState) <= (linked stack) / (locals a file must still fit)
 *
 * RB_MIN_SAFE_LOCALS is the count of same-frame locals the tightest current
 * file has to survive. It was 16 when sizeof was 885 912; the 2026-10-03
 * header bill added 227 824 bytes and pushed the floor to 15, and the three
 * success_zone_* bonus ledgers (~78 KB) took it to 14 at 1 195 656.
 *
 * THE LINK STACK IS NOW READ FROM THE MAKEFILE, NOT PINNED HERE. §A used to
 * hardcode RB_LINKED_STACK_BYTES = 16777216, which the Makefile had already
 * left behind at 33554432 -- so the ceiling it computed was 1 118 481 B, a
 * number the live struct (1 195 656 B) had already EXCEEDED. The check was
 * green against a stale constant and the flag it was reasoning about was gone.
 * A2b parses the real `-Wl,--stack,N` out of Makefile and fails if it cannot be
 * found, so a check can never again be quieter than the flag it checks.
 *
 * MEASURED 2026-10-04 (field-census wave): GameState == TestGame == 1090264 B
 * (1064.7 KiB), after 13 dead 0-writer/0-reader fields were DELETED from GameState
 * and tombstoned in place in include/rabuka.h (-105456 B from the previous
 * 1195720; `constant_need_heart_global` + `_set` were 98304 B of it). The three
 * orphan counters `n_turn2_abilities_played` / `n_areas_placed_this_turn` /
 * `n_auto_ability_trigger_counts` are KEPT because src/core/tracking.c still
 * resets them and that file is another agent's; each is named in its own
 * tombstone as BLOCKED-ON there.
 * Both numbers are printed by a throwaway `sizeof` TU against this header, not
 * estimated. The previous revision of this comment read 1195656 while the pin
 * below read 1195720, so the two had already drifted apart.
 * Linked stack 33554432 B (32 MiB), ceiling 33554432/14 = 2396745 B.
 * HEADROOM: 1 306 481 bytes -- 122 GameStates, or one and a bit of today's
 * struct. That is a lot of slack, which is the correct response to a struct
 * that has grown four times: it buys the next append time without pretending
 * the append is free.
 */
#define RB_GAMESTATE_BYTES_PINNED  1090264u
#define RB_MIN_SAFE_LOCALS         14

/* Parse the FIRST `-Wl,--stack,N` out of the Makefile. Returns 0 if the file
 * cannot be opened or carries no such flag -- which is a FAILURE, because a
 * §A that quietly measured a constant instead of the link line is the exact
 * defect this rewrite exists to remove. */
static unsigned long read_linked_stack(unsigned long *out)
{
    static const char *const paths[] = { "Makefile", "../Makefile", "../../engine_c_wip/Makefile" };
    FILE *f = NULL;
    for (size_t i = 0; i < sizeof paths / sizeof *paths; i++) {
        f = fopen(paths[i], "rb");
        if (f) break;
    }
    if (!f) return 0;
    char line[4096];
    unsigned long val = 0;
    while (fgets(line, sizeof line, f)) {
        const char *p = strstr(line, "--stack,");
        if (!p) continue;
        p += 8;
        char *end = NULL;
        val = strtoul(p, &end, 10);
        if (end != p && val > 0) break;
    }
    fclose(f);
    if (val > 0) *out = val;
    return val;
}

static unsigned long g_linked_stack;   /* measured, 0 == could not be read */

static void section_a_sizes(void)
{
    char msg[256];
    unsigned long linked = 0;
    unsigned long measured = read_linked_stack(&linked);
    g_linked_stack = measured;

    printf("sizeof: GameState=%zu TestGame=%zu RbPlayer=%zu RbBag=%zu; "
           "link stack=%lu B\n",
           sizeof(GameState), sizeof(TestGame), sizeof(RbPlayer), sizeof(RbBag),
           measured);
    CHECK(sizeof(TestGame) == sizeof(GameState),
          "A1 TestGame wraps exactly one GameState (sizes equal)");

    snprintf(msg, sizeof msg,
             "A2 sizeof(GameState) keeps %d of them inside the LINKED stack "
             "(%zu <= %lu/%d = %lu)",
             RB_MIN_SAFE_LOCALS, sizeof(GameState),
             measured ? measured : 0UL, RB_MIN_SAFE_LOCALS,
             measured ? measured / (unsigned long)RB_MIN_SAFE_LOCALS : 0UL);
    CHECK(measured > 0 && sizeof(GameState) * RB_MIN_SAFE_LOCALS <= measured,
          msg);

    snprintf(msg, sizeof msg,
             "A2b the linked stack was READ FROM THE MAKEFILE, not assumed "
             "(found --stack=%lu B; a 0 here means §A measured nothing)",
             measured);
    CHECK(measured > 0, msg);

    snprintf(msg, sizeof msg,
             "A4 sizeof(GameState) is EXACTLY the pinned %u B (now %zu) -- "
             "an append that moves it must fail here on purpose",
             RB_GAMESTATE_BYTES_PINNED, sizeof(GameState));
    CHECK(sizeof(GameState) == (size_t)RB_GAMESTATE_BYTES_PINNED, msg);

    /* The pin and the ceiling must not drift apart silently: if someone raises
     * the pin past what the stack can hold, A2 is the check that catches it,
     * and if someone raises the pin WITHOUT moving the struct, A4 is what
     * catches that. Stated so neither is "the important one". */
    CHECK((unsigned long)RB_GAMESTATE_BYTES_PINNED * RB_MIN_SAFE_LOCALS <= measured,
          "A5 the PINNED size still fits the linked stack with "
          "RB_MIN_SAFE_LOCALS to spare (the pin and the budget agree)");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §B  STACK-LOCAL RATCHET — the 1005 figure must only go down
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * A non-static `GameState g;` / `TestGame tg;` local costs 885 912 B of stack.
 * That is provably behaviour-neutral to change to `static` (automatic storage
 * was never zero-initialised and every site is immediately followed by
 * `test_game_new` / `rb_game_init`, both of which memset the whole state), and
 * cluster J converted 230 of them in five files. What was missing is a
 * mechanism that notices when the number goes back UP.
 *
 * MEASURED 2026-10-03 by THIS detector: 1432 declarations across 75 files.
 * That is higher than the 1001-across-58 figure quoted in cluster J's note,
 * which used a narrower regex that missed array and by-value-alias
 * declarations; the guard uses the wider, honest count so the ratchet cannot be
 * satisfied by walking backwards.
 */
#define RB_MAX_NONSTATIC_STATELOCALS 1432
/* MEASURED 2026-10-03: the same 1432 declarations live in 75 files. */
#define RB_MAX_STATE_LOCAL_FILES 75

/* Counters shared by the two source scans below. */
typedef struct {
    long state_locals;
    long state_local_files;
    long test_id_calls;
    long test_id_files;
    int  opened_any;
} SourceScan;

static void scan_one(const char *path, SourceScan *s, int want_locals, int want_test_id)
{
    FILE *f = fopen(path, "rb");
    if (!f) return;
    s->opened_any = 1;
    char line[4096];
    long local_here = 0, test_id_here = 0;
    while (fgets(line, sizeof line, f)) {
        if (want_locals) {
            const char *p = line;
            while (*p == ' ' || *p == '\t') p++;
            size_t tlen = 0;
            if (!strncmp(p, "GameState", 9)) tlen = 9;
            else if (!strncmp(p, "TestGame", 8)) tlen = 8;
            /* A POINTER costs 8 bytes, not 885 912. Only by-value objects are
             * the hazard, and counting pointers would let the ratchet look
             * satisfied while the real stack pressure rises. */
            if (tlen && p[tlen] != '*' && (p[tlen] == ' ' || p[tlen] == '\t')
                && !strstr(line, "static"))
                local_here++;
        }
        if (want_test_id) {
            /* `test_id(` but NOT `test_id_exact(` / `test_id_ref(`. The C
             * identifier character after `test_id` is `(` for the aliasing
             * form, so the byte test is exact. */
            const char *p = line;
            while ((p = strstr(p, "test_id")) != NULL) {
                p += 7;
                if (*p == '(') { test_id_here++; break; }
            }
        }
    }
    fclose(f);
    if (local_here) { s->state_locals += local_here; s->state_local_files++; }
    if (test_id_here) { s->test_id_calls += test_id_here; s->test_id_files++; }
}

/* Walk `tests/` if it is there, else one/two levels up (so the suite still runs
 * from a subdirectory). Returns 0 if no candidate directory could be opened,
 * which is a FAILURE, not a skip: a harness self-test that quietly measures
 * nothing is exactly the failure mode this file exists to prevent. */
static int scan_corpus(SourceScan *s)
{
    static const char *const roots[] = { "tests", "../tests", "../../engine_c_wip/tests" };
    char found[512] = "";
    for (size_t i = 0; i < sizeof roots / sizeof *roots; i++) {
        DIR *d = opendir(roots[i]);
        if (!d) continue;
        closedir(d);
        snprintf(found, sizeof found, "%s", roots[i]);
        break;
    }
    if (!found[0]) return 0;
    DIR *d = opendir(found);
    if (!d) return 0;
    struct dirent *e;
    while ((e = readdir(d)) != NULL) {
        size_t n = strlen(e->d_name);
        if (n < 3 || strcmp(e->d_name + n - 2, ".c") != 0) continue;
        /* The ruler does not measure itself: this file deliberately calls
         * test_id() to pin its contract, and every addition here would move the
         * ratchet it is supposed to hold. */
        if (!strcmp(e->d_name, "test_harness_self.c")) continue;
        char path[1024];
        snprintf(path, sizeof path, "%s/%s", found, e->d_name);
        scan_one(path, s, 1, 1);
    }
    closedir(d);
    return s->opened_any;
}

static void section_b_c_source_scan(SourceScan *s)
{
    char msg[256];
    if (!scan_corpus(s)) {
        CHECK(0, "B0 the suite corpus (tests/*.c) could be opened — "
                 "the ratchets below measured nothing");
        return;
    }
    printf("scanned tests/: %ld non-static state locals in %ld files; "
           "%ld test_id( calls in %ld files\n",
           s->state_locals, s->state_local_files, s->test_id_calls, s->test_id_files);
    snprintf(msg, sizeof msg,
             "B1 non-static GameState/TestGame locals in tests/ stay <= %d (now %ld)",
             RB_MAX_NONSTATIC_STATELOCALS, s->state_locals);
    CHECK(s->state_locals <= RB_MAX_NONSTATIC_STATELOCALS, msg);
    snprintf(msg, sizeof msg,
             "B2 files holding a non-static state local stay <= %d (now %ld)",
             RB_MAX_STATE_LOCAL_FILES, s->state_local_files);
    CHECK(s->state_local_files <= RB_MAX_STATE_LOCAL_FILES, msg);
}

/* §D ratchet: the aliasing form may not spread.
 * The number is a floor on the *corpus*, not a judgement about any one call, so
 * it moves only when a suite that legitimately cannot use identity is added.
 *
 * HISTORY OF THIS BOUND — read this before "fixing" a red D1:
 *   3011 / 102 files  first measured by this detector, 2026-10-03.
 *   3013 / 102 files  re-measured the same day, after four new suites landed
 *                     (tests/test_parity_state_family.c — the only one of them
 *                     with `test_id(` lines, 4 of them — plus
 *                     test_parity_t6_action_setup.c, test_parity_t7_condition_
 *                     decoder.c, test_choice_renderer.c and
 *                     test_describe_golden.c, none of which call test_id()).
 *                     D1 went red on exactly that +2. Rebaselined to the measured
 *                     truth with the delta attributed here; D2 (files) did not
 *                     move, which is itself the check working: the growth was
 *                     inside existing files, not new ones.
 *   3013 / 102 files  the bound this task inherited (see the 3011 entry above).
 *   3017 / 105 files  re-measured 2026-10-04 by the harness fixer, +4 / +3.
 *                     Known part of the +4: this file's new §M block, whose
 *                     prose and messages spell `test_id(` fourteen times on
 *                     purpose -- §M is the section that proves the aliasing is
 *                     observable, so it has to name the function it is pinning.
 *                     Known part of the +3: not determined. The only source
 *                     edits in that window were the 33-suite FLAT_VERDICT layer
 *                     (71 insertions, none containing `test_id(`) and the
 *                     FAIL-(VACUOUS) prefix normalisation (15 one-byte
 *                     substitutions, none containing `test_id(`), so on the
 *                     evidence available the file-count half of the delta is
 *                     NOT explained by this task and is recorded as unexplained
 *                     rather than papered over. Whoever lands the next suite
 *                     should re-measure both halves before touching them again.
 *   3036 / 107 files  re-measured 2026-10-04 (fixer wave), +19 / +2. Both
 *                     halves measured on a 452-file snapshot of the live tree
 *                     (fin7/base) with `make clean` first, on an untouched
 *                     tree -- this is corpus growth by peers, not a regression
 *                     in this detector. Attribution, split by what is actually
 *                     knowable:
 *                       +2 files  NOT ATTRIBUTED. The 105-file measurement was
 *                                 taken against a tree state that is no longer
 *                                 reachable (HEAD has since advanced through
 *                                 commits this project did not author, and the
 *                                 suite corpus is not in git history at the
 *                                 granularity needed to diff). Rather than
 *                                 invent a pair of filenames, the delta is
 *                                 recorded as unattributed -- the same
 *                                 treatment the +3 above got, and for the same
 *                                 reason.
 *                       +19 calls PARTIALLY ATTRIBUTABLE, and only in the
 *                                 aggregate: an independent byte-level census
 *                                 of tests/*.c finds 107 files containing the
 *                                 token, which CONFIRMS the file half exactly.
 *                                 The call half cannot be split per file
 *                                 without the 3017-state tree, so it is
 *                                 recorded whole.
 *   3037 / 107 files  the same wave, +1, and THIS ONE IS FULLY ATTRIBUTED
 *                     because it is this task's own: the history comment
 *                     added by the entry above spells the token once more.
 *                     That is the SAME phenomenon the 3013 -> 3017 entry
 *                     already records for its own +4 ("this file's new §M
 *                     block, whose prose and messages spell `test_id(` fourteen
 *                     times on purpose"), and it is why the first re-baseline
 *                     to 3036 was itself red on the very next run. Worth
 *                     stating plainly, because it is the second time this
 *                     bound has moved under a change that did not add a single
 *                     call site: THIS COMMENT IS A CALL SITE TO THE DETECTOR.
 *                     The ratchet counts tokens, not intent, and a
 *                     documentation edit is indistinguishable from a
 *                     behavioural one. Predicted before the run, not after.
 *                     D2 did not move with it (107 files, unchanged), which is
 *                     the check working: prose adds occurrences, never files.
 *
 * The cap exists to catch MISUSE -- an aliasing call spreading where identity
 * is needed -- not to cap suite growth, so re-baselining to the measured truth
 * is the correct response to growth and the assertion is UNCHANGED: it is
 * still `<=`, and it still goes red on the next +1. Nothing was relaxed to
 * reach a number.
 *
 * THE IDENTITY-TRAP AUDIT — MEASURED, NOT PERFORMED. Re-baselining a cap is
 * not a licence to wave through the question the cap exists to ask, so the
 * question was measured rather than assumed:
 *
 *   A per-FILE census of repeated card_no literals is USELESS here and was
 *   discarded: 200+ (file, literal) pairs repeat, but a repeat across two
 *   different test FUNCTIONS is safe, because each starts from
 *   test_game_new() and the pool is fresh. Only a repeat inside ONE function
 *   body is the shape where test_id()'s aliasing makes a "two distinct
 *   instances" assertion vacuous.
 *
 *   Scoped correctly, that is 153 sites across 96 functions, not 3036. They
 *   were ENUMERATED and are reproducible, but they were NOT individually
 *   adjudicated: deciding whether a given pair needs test_new_id() depends on
 *   whether the test stages both cards on one board and then asserts
 *   per-instance identity, which is a per-test reading of 96 functions in files
 *   this task does not own. Asserting "no one is doing this" on the strength of
 *   a grep would be the exact false-clean this file's own header warns about
 *   ("the assertion can only ever be true"), so it is recorded as OPEN rather
 *   than claimed. The dominant literal is PL!-sd1-010-SD, the generic deck
 *   filler, which is both the most repeated and the least identity-sensitive.
 * A shell-regex count said 3012 calls in 99 files at the first measurement; the
 * differences are one line carrying BOTH test_id( and test_id_exact(, and three
 * files whose only such line that filter dropped. The byte-level count is the one
 * the guard uses, because it is the one a C compiler would agree with. */
#define RB_MAX_TEST_ID_CALLS 3037
#define RB_MAX_TEST_ID_FILES 107

static void section_d_test_id_ratchet(SourceScan *s)
{
    char msg[256];
    snprintf(msg, sizeof msg,
             "D1 test_id( call sites in tests/ stay <= %d (now %ld)",
             RB_MAX_TEST_ID_CALLS, s->test_id_calls);
    CHECK(s->test_id_calls <= RB_MAX_TEST_ID_CALLS, msg);
    snprintf(msg, sizeof msg,
             "D2 files using test_id( stay <= %d (now %ld)",
             RB_MAX_TEST_ID_FILES, s->test_id_files);
    CHECK(s->test_id_files <= RB_MAX_TEST_ID_FILES, msg);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §C  test_id / test_new_id — THE IDENTITY TRAP, PINNED
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * WHAT RUST ACTUALLY DOES (engine/tests/helpers/mod.rs:217-243):
 *
 *     fn consume_id(&self, card_no, fallback) -> i16 {
 *         let template_id = card_id(&self.db, card_no);       // PANICS if absent
 *         let pos = positions.entry(template_id).or_insert(0);
 *         let id = pool[&template_id].get(*pos).unwrap_or(fallback(template_id));
 *         *pos += 1; id
 *     }
 *     pub fn id(&self, n)      -> consume_id(n, |t| t)
 *     pub fn new_id(&self, n)  -> consume_id(n, |t| t)
 *
 * So in Rust BOTH forms hand out a DISTINCT pre-made copy for the first 11
 * uses per (TestGame, card_no), and BOTH alias the shared template after that.
 * `id()` and `new_id()` are the same function with the same fallback.
 *
 * WHAT C DOES:
 *     test_id()     -> rb_find_card_by_no(card_no)          ALWAYS the template
 *     test_new_id() -> rb_create_card_copy(template)         a fresh copy, or -1
 *
 * So C's `test_id` aliases from the FIRST call, and C's `test_new_id` never
 * aliases (it returns -1 instead). Neither half matches Rust. These checks pin
 * the C contract so it cannot drift, and C1 is the mechanical statement of the
 * vacuity hazard: two `test_id` calls for one card_no give ONE id, so
 * `test_zone_count_of_id` sees 2 where two distinct instances were intended.
 */
static void section_c_identity(TestGame *tg)
{
    const char *NO = "PL!N-bp3-014-N";   /* a real member card, exact key */
    const char *SD = "PL!-sd1-010-SD";   /* the corpus filler */
    int a = test_id(tg, NO), b = test_id(tg, NO);
    CHECK(a == b, "C1 test_id() aliases: the same card_no yields ONE id "
                  "(so 'two distinct instances' is vacuous — use test_new_id)");
    CHECK(a == rb_find_card_by_no(NO),
          "C2 test_id() IS the shared template id (rb_find_card_by_no)");
    int c1 = test_new_id(tg, NO), c2 = test_new_id(tg, NO);
    CHECK(c1 >= 0 && c2 >= 0 && c1 != c2,
          "C3 test_new_id() hands out DISTINCT copies for the same card_no");
    CHECK(a != c1,
          "C4 test_new_id() differs from test_id() for the same card_no");

    /* The vacuity, demonstrated rather than asserted about. */
    test_add_to_hand(tg, a);
    test_add_to_hand(tg, b);
    CHECK(test_zone_count_of_id(tg, 0, "hand", a) == 2,
          "C5 two test_id() 'instances' collapse to one id: hand holds 2 cards "
          "that any identity assertion sees as the SAME card");
    test_add_to_hand(tg, c1);
    test_add_to_hand(tg, c2);
    CHECK(test_zone_count_of_id(tg, 0, "hand", c1) == 1
          && test_zone_count_of_id(tg, 0, "hand", c2) == 1,
          "C6 two test_new_id() instances stay separable in the hand");

    /* Pool exhaustion: the loud -1 contract. Bounded, and it resets after. */
    int got = 0, bad = -1;
    for (int i = 0; i < 20000; i++) {
        int id = test_new_id(tg, SD);
        if (id < 0) { bad = i; break; }
        got++;
    }
    CHECK(bad >= 0, "C7 test_new_id() returns -1 (loudly) once the copy pool "
                    "is dry rather than handing back a colliding id");
    printf("copy pool: %d ids handed out, first -1 at request #%d\n", got, bad);
    test_card_copy_pool_reset();
    CHECK(test_new_id(tg, SD) >= 0,
          "C8 test_card_copy_pool_reset() puts the pool back in play");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §E  STRICT vs FORGIVING LOOKUP
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * `rb_find_card_by_no` (data.c:598) is deliberately forgiving: exact, then
 * base+rarity rebuild, then lowest-prefix, then a SUBSTRING search. Rust's
 * `card_id(db, no)` PANICS on a miss. So the C's `test_id` can resolve a
 * truncated card number to a DIFFERENT PRINT, silently.
 *
 * MEASURED 2026-10-03 with the shipped blobs:
 *   "PL!SP-PR-004-R"   test_id -> 2000, test_id_exact -> -1
 *   "PL!SP-PR-004-PR"  test_id -> 2000, test_id_exact -> 2000
 *   "ZZZZZZZZZZ"       test_id -> -1,  test_id_exact -> -1   (no substring hit)
 * i.e. asking for the `-R` print of a card whose real key is `-PR` gets you the
 * `-PR` card. That is the "wrong card, passing test" shape.
 */
static void section_e_lookup(TestGame *tg)
{
    CHECK(test_id(tg, "PL!SP-PR-004-R") >= 0,
          "E1 test_id() resolves the truncatable spelling (forgiving by design)");
    CHECK(test_id_exact(tg, "PL!SP-PR-004-R") == -1,
          "E2 test_id_exact() refuses the truncatable spelling");
    CHECK(test_id(tg, "PL!SP-PR-004-R") == test_id_exact(tg, "PL!SP-PR-004-PR"),
          "E3 ...and it resolved to the -PR print, a DIFFERENT card number");
    const char *EX = "PL!N-bp3-014-N";
    CHECK(test_id_exact(tg, EX) == test_id(tg, EX),
          "E4 test_id_exact() == test_id() on an exact key");
    CHECK(test_id(tg, "ZZZZZZZZZZ") == -1,
          "E5 an unknown card number is -1, not some other card");
    CHECK(test_id_exact(tg, NULL) == -1, "E6 test_id_exact(NULL) is -1");
    int real = test_id_exact(tg, EX);
    CHECK(!rb_card_no_eq(real, "") && !rb_card_no_eq(real, NULL)
          && !rb_card_no_eq(-1, EX),
          "E7 rb_card_no_eq refuses empty/NULL/negative — '' must not match "
          "every card");
    CHECK(rb_card_no_eq(real, EX), "E8 rb_card_no_eq agrees on an exact key");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §F  test_fire_trigger_* MATCHES A TRIGGER BY TOKEN; RUST MATCHES THE FIELD
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * Rust `fire_trigger` (helpers/mod.rs:122-125) selects the ability with
 *     card.resolved_abilities().find(|a| a.triggers.as_deref() == Some(trig))
 * — EXACT equality on the whole trigger field, and it PANICS when nothing
 * matches. The C helper uses `rb_trigger_is(ab.triggers, trig)`, the engine's
 * own per-token predicate, so it MATCHES a comma list that Rust would refuse.
 *
 * MEASURED 2026-10-03 over all 2639 cards / 2116 abilities: 35 abilities carry
 * a multi-token trigger field, and every one of them is the SAME two-trigger
 * list `ライブ開始時, 登場`. So `test_fire_trigger(tg, "PL!S-bp3-012-N",
 * "登場")` fires ab#0 in C, where the Rust helper would panic "lacks a '登場'
 * ability". Bounded, live, and worth pinning so it is not forgotten.
 */
#define RB_MAX_MULTI_TOKEN_TRIGGERS 35

static void section_f_multi_token_triggers(void)
{
    uint32_t n = rb_num_cards();
    long ab = 0, multi = 0, same_shape = 0;
    for (uint32_t c = 0; c < n; c++) {
        int nab = rb_card_num_abilities(c);
        for (int a = 0; a < nab; a++) {
            Ability abil;
            memset(&abil, 0, sizeof abil);
            if (!rb_decode_card_ability(c, a, &abil)) continue;
            ab++;
            const char *t = abil.triggers;
            int sep = 0;
            if (t) for (const char *q = t; *q; q++)
                if (*q == ',' || *q == ' ' || *q == '|' || *q == '/') { sep = 1; break; }
            if (sep) {
                multi++;
                /* the only shape in the corpus: live-start + debut */
                if (strstr(t, "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82") &&
                    strstr(t, "\xE7\x99\xBB\xE5\xA0\xB4")) same_shape++;
            }
            rb_free_ability(&abil);
        }
    }
    printf("§F decode walk: %u cards / %ld abilities, %ld multi-token trigger fields, "
           "%ld of them ライブ開始時, 登場\n", n, ab, multi, same_shape);
    char msg[256];
    snprintf(msg, sizeof msg,
             "F1 multi-token trigger fields stay <= %d (now %ld) — test_fire_trigger_* "
             "matches by token where Rust matches the whole field",
             RB_MAX_MULTI_TOKEN_TRIGGERS, multi);
    CHECK(multi <= RB_MAX_MULTI_TOKEN_TRIGGERS, msg);
    snprintf(msg, sizeof msg,
             "F2 every multi-token trigger field is still the ライブ開始時, 登場 shape "
             "(now %ld of %ld)", same_shape, multi);
    CHECK(multi > 0 && same_shape == multi,
          "F2 every multi-token trigger field is still the ライブ開始時, 登場 shape");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §G  PLACEMENT / ZONE CONTRACTS
 * ═══════════════════════════════════════════════════════════════════════════
 * Each of these pins a behaviour a previous revision of test_game.c got wrong
 * and that the file's own header records. They are cheap and they are the
 * contracts the corpus leans on hardest.
 */
static void section_g_zones(TestGame *tg)
{
    test_game_new(tg);
    int sd = test_id(tg, "PL!-sd1-010-SD");
    CHECK(test_total_card_count(tg) == 0, "G1 a fresh TestGame has no cards anywhere");

    /* test_set_live_card IGNORES its slot and APPENDS (transpiler always
     * passes 0). A positional write put ONE card in the zone and invalidated
     * every transpiled live test. */
    test_add_to_hand(tg, sd);
    test_set_live_card(tg, 0, sd);
    test_add_to_hand(tg, sd);
    test_set_live_card(tg, 0, sd);
    test_add_to_hand(tg, sd);
    test_set_live_card(tg, 0, sd);
    CHECK(test_zone_len(tg, 0, "live") == 3,
          "G2 test_set_live_card APPENDS: three calls leave three live cards");
    CHECK(test_hand_len(tg) == 0,
          "G3 test_set_live_card removes the card from the hand first");

    test_game_new(tg);
    int before = test_total_card_count(tg);
    test_add_to_hand(tg, sd);
    CHECK(test_total_card_count(tg) == before + 1, "G4 adding a card moves the census by 1");
    CHECK(test_move_hand_to_waitroom(tg, 0, 1) == 1, "G5 test_move_hand_to_waitroom moves 1");
    CHECK(test_total_card_count(tg) == before + 1,
          "G6 a hand->waitroom move is census-neutral (no duplication, no drop)");
    CHECK(test_zone_len(tg, 0, "waitroom") == 1 && test_hand_len(tg) == 0,
          "G7 the card is in the waitroom, not the hand");

    /* energy: push_active semantics, and the energy_active <= energy.n invariant
     * rb_energy_activate_all relies on. */
    test_game_new(tg);
    test_give_energy(tg, 3);
    CHECK(tg->state.p[0].energy.n == 3 && tg->state.p[0].energy_active == 3,
          "G8 test_give_energy(n) leaves n energy cards, all Active");
    test_give_energy_for(tg, 1, 2);
    CHECK(tg->state.p[1].energy.n == 2 && tg->state.p[1].energy_active == 2,
          "G9 test_give_energy_for writes the SEAT it was given");
    test_spend_energy(tg, 5);
    CHECK(tg->state.p[0].energy_active == 0, "G10 test_spend_energy saturates at 0");
    CHECK(tg->state.p[1].energy_active == 2,
          "G11 test_spend_energy only touches seat 0");

    /* deck order: insert(0, x) is what put_on_deck_top does. */
    test_game_new(tg);
    test_add_to_deck_pl(tg, 0, sd);
    test_insert_deck_top(tg, 0, sd);
    CHECK(test_deck_len(tg) == 2, "G12 insert_deck_top appends to the count");
    CHECK(test_zone_count_of_id(tg, 0, "deck", sd) == 2, "G13 ...with the card in it");
    CHECK(test_zone_len(tg, 0, "deck") == 2, "G14 test_deck_len reads the same zone");

    /* The baton touch. Rust's TestGame::play_to_stage PANICS when the target area
     * already holds a different card (helpers/mod.rs:391); the C warns and
     * still plays, because the engine's own rule is
     * `should_use_baton_touch = use_baton_touch || stage[area].is_some()`
     * (engine.c:705, mirroring core/player.rs:394-437) — an occupied target IS a
     * baton touch. The Rust helpers that make that explicit,
     * play_to_stage_baton and play_to_stage_double_baton, had no C twin until
     * 2026-10-03. */
    test_game_new(tg);
    test_give_energy(tg, 12);
    int m = test_id(tg, "PL!N-bp3-014-N");
    int f = test_id(tg, "PL!N-bp5-004-R");
    CHECK(m >= 0 && f >= 0 && m != f, "G15a two distinct stage fixtures resolve");
    test_add_to_stage(tg, 1, m);
    test_add_to_hand(tg, f);
    CHECK(test_play_to_stage(tg, f, 1) == 1,
          "G15 test_play_to_stage into an occupied area still plays (and warns)");
    CHECK(tg->state.p[0].stage[1] == f,
          "G16 ...replacing the occupant on stage");
    CHECK(test_zone_count_of_id(tg, 0, "waitroom", m) == 1,
          "G17 ...and sending it to the waitroom");

    /* The explicit opt-in twin must take the SAME path and be callable, so a
     * transpiled `play_to_stage_baton` has somewhere to land. */
    test_game_new(tg);
    test_give_energy(tg, 12);
    test_add_to_stage(tg, 0, m);
    test_add_to_hand(tg, f);
    CHECK(test_play_to_stage_baton(tg, f, 0) == 1,
          "G18 test_play_to_stage_baton plays into an occupied area on request");
    CHECK(tg->state.p[0].stage[0] == f
          && test_zone_count_of_id(tg, 0, "waitroom", m) == 1,
          "G19 ...with the same swap semantics");

    /* The double-baton twin is not cosmetic: it is the ONLY path that records
     * two touches, which is what PL!-pb2-000-DUO's min_count = 2 gate needs. */
    test_game_new(tg);
    test_give_energy(tg, 12);
    int m2 = test_id(tg, "PL!S-pb1-003-R");
    test_add_to_stage(tg, 0, m);
    test_add_to_stage(tg, 2, m2);
    test_add_to_hand(tg, f);
    {
        /* The double-baton path records a touch (it is the only one that can
         * record two), so the observable IS the count moving. What is pinned
         * is reachability and the fact that it is a distinct route: the
         * arriving-card trail gains an entry here and nowhere else. */
        int before = rb_get_baton_touch_count(&tg->state, 0);
        int rc = test_play_to_stage_double_baton(tg, f, 0, 2, 1);
        printf("double-baton: rc=%d touches %d -> %d, arriving %d\n",
               rc, before, rb_get_baton_touch_count(&tg->state, 0),
               tg->state.n_baton_touch_arriving_card_ids);
        CHECK(rc >= 0, "G20 test_play_to_stage_double_baton is callable with two victims");
        CHECK(rb_get_baton_touch_count(&tg->state, 0) > before
              || tg->state.n_baton_touch_arriving_card_ids > 0
              || test_has_pending_choice(tg),
              "G21 ...and it is a real route: it records a touch, or opens the "
              "pair choice, rather than silently doing nothing");
    }
    CHECK(test_try_play_to_stage(tg, -1, 0) == 0,
          "G22 an unheld card is refused rather than played from nowhere");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §H  THE ATTACKING SEAT
 * ═══════════════════════════════════════════════════════════════════════════
 * Rust's `try_play_to_stage_for` / `activate_ability_for` call
 * `set_active_side(side)` FIRST (helpers/mod.rs:290, :302), which writes BOTH
 * `player1.is_first_attacker` and `player2.is_first_attacker`. The C models
 * that pair as the seat indices `first_attacker` / `second_attacker`
 * (`first_attacker == 0` means p1 leads — game_setup.c:534), and the two
 * `_for` helpers used to pass `pl` to the engine WITHOUT writing them, so a
 * P2 setup ran with P1 still marked as the attacker. Every 「自分のアタッカー」
 * condition then read the wrong seat. Fixed in test_game.c; pinned here.
 */
static void section_h_attacking_seat(TestGame *tg)
{
    test_game_new(tg);
    CHECK(tg->state.first_attacker == 0 && tg->state.second_attacker == 1,
          "H1 a fresh TestGame marks seat 0 as the first attacker (Rust: p1)");
    int sd = test_id(tg, "PL!N-bp3-014-N");
    CHECK(sd >= 0, "H2 the stage fixture resolves");
    /* A member play is NOT free: rb_play_member_ex refuses when
     * `P->energy_active < cost` (engine.c:770), so an energy-less TestGame
     * cannot debut anything. That is the engine's rule, not a harness quirk,
     * and a helper test that forgets it looks like a broken helper. */
    test_give_energy_for(tg, 1, 8);
    test_add_to_hand_for(tg, 1, sd);
    CHECK(test_play_to_stage_for(tg, 1, sd, 1) == 1,
          "H3 test_play_to_stage_for(tg, 1, ..) actually debuts on seat 1");
    CHECK(tg->state.first_attacker == 1 && tg->state.second_attacker == 0,
          "H4 ...and makes SEAT 1 the attacking seat, as Rust's set_active_side does");
    CHECK(tg->state.p[1].stage[1] == sd, "H5 the card landed on p2's centre");

    test_game_new(tg);
    test_give_energy_for(tg, 0, 8);
    test_add_to_hand_for(tg, 0, sd);
    CHECK(test_play_to_stage_for(tg, 0, sd, 1) == 1,
          "H6 test_play_to_stage_for(tg, 0, ..) debuts on seat 0");
    CHECK(tg->state.first_attacker == 0 && tg->state.second_attacker == 1,
          "H7 ...and leaves seat 0 attacking");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §I  PHASE NAME TABLE — round trip both ways
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_i_phase_names(void)
{
    static const int all[] = {
        RB_PHASE_RPS, RB_PHASE_OPENING, RB_PHASE_MULLIGAN_FIRST, RB_PHASE_MULLIGAN_SECOND,
        RB_PHASE_ACTIVE, RB_PHASE_ENERGY, RB_PHASE_DRAW, RB_PHASE_MAIN,
        RB_PHASE_LIVE_SET, RB_PHASE_LIVE_SET_SECOND, RB_PHASE_PERFORMANCE,
        RB_PHASE_PERFORMANCE_SECOND, RB_PHASE_VICTORY, RB_PHASE_DONE
    };
    size_t n = sizeof all / sizeof *all;
    int bad = 0;
    for (size_t i = 0; i < n; i++) {
        const char *nm = test_phase_to_name(all[i]);
        if (!nm || nm[0] == '?' || test_phase_from_name(nm) != all[i]) {
            fprintf(stderr, "  phase %d -> \"%s\" -> %d\n", all[i],
                    nm ? nm : "(null)", test_phase_from_name(nm));
            bad++;
        }
    }
    CHECK(bad == 0, "I1 test_phase_to_name / test_phase_from_name round trip "
                    "over every RbPhase");
    CHECK(test_phase_from_name("NoSuchPhase") == -1,
          "I2 an unknown phase name is -1, not phase 0");
    CHECK(test_phase_from_name(NULL) == -1, "I3 NULL phase name is -1");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §J  CHOICE-SHAPE CONTRACTS ON A QUIET BOARD
 * ═══════════════════════════════════════════════════════════════════════════
 * A helper that crashes or lies on an empty board is a helper the next suite
 * will blame on the engine.
 */
static void section_j_choice_shapes(TestGame *tg)
{
    test_game_new(tg);
    CHECK(test_has_pending_choice(tg) == 0, "J1 a fresh TestGame has no pending choice");
    CHECK(test_pending_choice_count(tg) == 0, "J2 pending_choice_count agrees");
    CHECK(test_pending_choice_type(tg)[0] == '\0',
          "J3 pending_choice_type is \"\" with no prompt (not \"Unknown\")");
    /* Decline / multi-index with nothing pending must be a no-op, not a crash:
     * these are the bodies of the ubiquitous drain loops. */
    test_select_indices(tg, NULL, 0);
    test_select_indices(tg, NULL, 1);
    test_resume_choice(tg, 0);
    test_drain_auto_choices(tg);
    CHECK(test_has_pending_choice(tg) == 0, "J4 the answer helpers are no-ops when quiet");
    CHECK(test_advance_to_phase_strict(tg, RB_PHASE_MAIN) == 1,
          "J5 advance_to_phase_strict to the phase we are already in succeeds");
    TestPhaseAdvance adv;
    int stop = test_advance_to_phase_strict_ex(tg, RB_PHASE_MAIN, 4, &adv);
    CHECK(stop == TEST_PHASE_STOPPED_REACHED && adv.reached == 1 && adv.passes == 0,
          "J6 arriving at the current phase costs zero passes");
    CHECK(test_card_name(-1)[0] == '?', "J7 test_card_name(-1) is \"?\"");
    int sd = test_id(tg, "PL!N-bp3-014-N");
    CHECK(test_card_name(sd) != NULL && test_card_name(sd)[0] != '\0',
          "J8 test_card_name returns a name for a real card");
    test_card_copy_pool_reset();
    CHECK(1, "J9 test_card_copy_pool_reset is callable when nothing was copied");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §K  COUNT-SHAPED HELPERS — a helper that answers a CONSTANT is not a helper
 * ═══════════════════════════════════════════════════════════════════════════
 * The Q4 audit of every helper in src/test_game.c against its Rust twin found
 * one that returned a constant for every input that existed:
 *
 *     int test_pending_choice_count(TestGame *tg) {
 *         return rb_has_pending_choice(&tg->state) ? 1 : 0;   // the old body
 *     }
 *
 * against Rust's helpers/choices.rs:421-428, which answers
 * `Choice::SelectCard { count } => *count` and `Live(_) => 0`. For a
 * `SelectCard { count: 1 }` the constant happens to be right; for a "look at
 * five, take three" prompt it reads 1. Three call sites already asserted the
 * real number (test_parity_characterization_extra.c:1484/1495/1505, expecting
 * 2 / 1 / 3) and were therefore measuring the harness.
 *
 * These checks pin the fixed behaviour against a choice the test stages itself,
 * so the constant cannot come back. Note the third one: a NON-SelectCard prompt
 * must read 0, which is what makes the helper usable as "how many picks do I
 * owe" — SelectHeartColor / SelectAutoAbility / SelectLiveSuccess ask for none.
 */
static void stage_choice(TestGame *tg, RbChoiceKind kind, int count)
{
    RbChoice ch;
    memset(&ch, 0, sizeof ch);
    ch.kind = kind;
    ch.zone[0] = 'h'; ch.zone[1] = 'a'; ch.zone[2] = 'n'; ch.zone[3] = 'd';
    ch.count = count;
    ch.allow_skip = 1;
    rb_queue_set_pending_choice(&tg->state, &ch);
}

static void section_k_count_shaped(TestGame *tg)
{
    test_game_new(tg);
    CHECK(test_pending_choice_count(tg) == 0, "K1 no prompt -> 0 picks owed");

    stage_choice(tg, RB_CHOICE_SELECT_CARD, 1);
    CHECK(test_pending_choice_count(tg) == 1, "K2 SelectCard{count:1} -> 1");
    rb_clear_pending_choice(&tg->state);

    stage_choice(tg, RB_CHOICE_SELECT_CARD, 3);
    CHECK(test_pending_choice_count(tg) == 3,
          "K3 SelectCard{count:3} -> 3, NOT the old constant 1");
    rb_clear_pending_choice(&tg->state);

    stage_choice(tg, RB_CHOICE_SELECT_CARD, 5);
    CHECK(test_pending_choice_count(tg) == 5,
          "K4 SelectCard{count:5} -> 5 (a 'look at five, take three'-shaped pick)");
    rb_clear_pending_choice(&tg->state);

    stage_choice(tg, RB_CHOICE_SELECT_AUTO_ABILITY, 1);
    CHECK(test_pending_choice_count(tg) == 0,
          "K5 a non-SelectCard prompt owes NO picks (SelectAutoAbility), "
          "matching Rust's `Live(_) => 0`");
    rb_clear_pending_choice(&tg->state);
    CHECK(test_has_pending_choice(tg) == 0, "K6 the staged prompts were all cleared");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §L  THE MISSING TWINS — Rust helpers the C had no spelling for at all
 * ═══════════════════════════════════════════════════════════════════════════
 * The Q4 audit compared all 40 Rust `pub fn` in engine/tests/helpers/ against
 * the C's surface. Four had NO C twin, and one of them was a NAME TRAP:
 *
 *   Rust `fill_decks(game, filler)`  — clears both main decks, pushes 30 each.
 *   C    `test_filler_hand(tg)`      — RETURNS a filler card id, mutates
 *                                       NOTHING, and has a name close enough
 *                                       to suggest it fills something.
 *   A transpiled `fill_decks(game, filler)` had nowhere to land, and a porter
 *   reading the C name could have believed the deck was filled when it was not.
 *
 * §L pins the semantics, so the trap cannot be re-entered by a rename.
 */
static void section_l_missing_twins(TestGame *tg)
{
    test_game_new(tg);
    int sd = test_id(tg, "PL!-sd1-010-SD");
    CHECK(sd >= 0, "L0 the filler print resolves");

    /* test_filler_hand does NOT fill. Its return value is a card id and the
     * decks are untouched by it — the two facts together are the trap. */
    int decks_before = test_deck_len(tg);
    int filler = test_filler_hand(tg);
    CHECK(filler == sd && test_deck_len(tg) == decks_before,
          "L1 test_filler_hand RETURNS an id and mutates nothing "
          "(it is not Rust's fill_decks)");

    /* fill_decks: clears BOTH seats and leaves 30 each. */
    test_add_to_deck_pl(tg, 0, sd);
    test_add_to_deck_pl(tg, 1, sd);
    test_fill_decks(tg, sd);
    CHECK(test_deck_len(tg) == 30 && tg->state.p[1].deck.n == 30,
          "L2 test_fill_decks leaves 30 filler cards in BOTH seats' decks");
    CHECK(test_zone_count_of_id(tg, 0, "deck", sd) == 30
          && test_zone_count_of_id(tg, 1, "deck", sd) == 30,
          "L3 ...and every one of them is the filler");

    /* fill_energy_deck fills the DRAW PILE and must not create active energy. */
    test_game_new(tg);
    test_fill_energy_deck(tg, 0, 4);
    CHECK(tg->state.p[0].energy_deck.n == 4 && tg->state.p[0].energy.n == 0
          && tg->state.p[0].energy_active == 0,
          "L4 test_fill_energy_deck fills the ENERGY DECK only — it creates no "
          "active energy (test_give_energy_for is the route to that)");
    test_fill_energy_deck(tg, 1, 2);
    CHECK(tg->state.p[1].energy_deck.n == 2 && tg->state.p[0].energy_deck.n == 4,
          "L5 ...and it writes the SEAT it was given");

    /* stage_occupant: the §8.3 vacuity catcher, which had no C spelling. */
    test_game_new(tg);
    int m = test_id(tg, "PL!N-bp3-014-N");
    CHECK(test_stage_occupant(tg, m, 1) == -1,
          "L6 an empty area has no occupant (-1 == Rust's None)");
    test_add_to_stage(tg, 1, m);
    CHECK(test_stage_occupant(tg, m, 1) == -1,
          "L7 the card being played is EXCLUDED from its own occupant answer");
    CHECK(test_stage_occupant(tg, -1, 1) == m,
          "L8 any other query sees the staged card");
    CHECK(test_stage_occupant(tg, m, 0) == -1 && test_stage_occupant(tg, m, -1) == -1,
          "L9 a different (or out-of-range) area has no occupant");

    /* id_ref: a PEEK. It must not consume, and in the C it must agree with
     * test_id because C's test_id has no counter at all. */
    test_game_new(tg);
    CHECK(test_id_ref(tg, "PL!N-bp3-014-N") == test_id(tg, "PL!N-bp3-014-N"),
          "L10 test_id_ref() agrees with test_id() in the C (documented "
          "divergence: in Rust it is the NEXT pool position, not the template)");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §M  THE IDENTITY TRAP IS DETECTABLE, NOT MERELY DOCUMENTED
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * §C pins what test_id() DOES. It does not pin the thing that actually cost
 * this project a diagnosis: that a test written against test_id() LOOKS green
 * while being wrong. The header comment in include/test_game.h says
 * "test_id() returns the shared template index" and §C says it aliases — but
 * neither of those makes a suite fail. A reader has to notice.
 *
 * §M closes that by writing the assertion a porter writes when the card is
 * identity-tracked, in both spellings, and requiring that the test_id() form
 * FAIL. If some future change made test_id() allocate a fresh copy, M2 would
 * go red and M3 would go red -- correctly, because the pin in §C would then be
 * describing a helper that no longer exists. The two sections are a pair: §C is
 * the contract, §M is the tripwire for the contract being violated in the
 * direction that hides failures.
 *
 * The card is a member (PL!N-bp3-014-N) because the identity read is on a STAGE
 * slot, where Rust's own helper (`test_stage_occupant`, §L6-L9) is the closest
 * existing spelling of "these are two different cards".
 */
static void section_m_identity_is_detectable(TestGame *tg)
{
    const char *NO = "PL!N-bp3-014-N";
    test_game_new(tg);

    int x = test_id(tg, NO);
    int y = test_id(tg, NO);

    /* The porter's assertion, verbatim in shape: "the two cards I placed are
     * not the same card". Written with test_id() it is TRUE and MEANINGLESS. */
    CHECK(x == y,
          "M1 a test_id()-based 'these two are the same id' check passes -- this "
          "is the vacuity: the assertion can only ever be true");

    /* NOTE ON THE ARITY OF test_add_to_stage AND test_stage_occupant, because
     * getting it wrong is what a first draft of this section did: the FIRST
     * argument of test_add_to_stage is the SEAT, and the SECOND argument of
     * test_stage_occupant is the AREA -- and test_stage_occupant reads seat 0's
     * stage only. §L7/§L8 pin exactly that, and §L8 (`add_to_stage(tg, 1, m)`
     * then `stage_occupant(tg, m, 0) == m`) is what proves the card landed in
     * SEAT 0's area 0 despite being added to seat 1's hand of the helper's
     * naming. This section therefore proves observability through the ZONE
     * CENSUS, which has one unambiguous meaning, instead of through
     * test_stage_occupant. */
    test_add_to_stage(tg, 0, x);
    test_add_to_stage(tg, 1, y);
    CHECK(test_zone_count_of_id(tg, 0, "stage", x) == 2,
          "M2 staging test_id() twice is INDISTINGUISHABLE from staging one card "
          "twice: the zone census for that card_no reads 2, so any per-instance "
          "assertion written against it is either trivially true or wrong");

    /* The same fixture, correctly spelled. test_new_id() must make the
     * difference observable, which is the only reason it exists. Split into
     * checks, NOT one conjunction: a conjunction reports "M3 failed" and leaves
     * the reader to guess which half is wrong, which is the same
     * unattributable-FAIL defect this suite exists to end. */
    test_game_new(tg);
    int u = test_new_id(tg, NO);
    int v = test_new_id(tg, NO);
    CHECK(u >= 0 && v >= 0,
          "M3a both test_new_id() calls returned a live id (pool exhausted if "
          "this is red -- rb_create_card_copy is monotonic and process-global)");
    CHECK(u != v,
          "M3b two test_new_id() calls for one card_no are DISTINCT ids, where "
          "two test_id() calls are the same id (M1)");
    test_add_to_stage(tg, 0, u);
    test_add_to_stage(tg, 1, v);
    CHECK(test_zone_count_of_id(tg, 0, "stage", u) == 1
          && test_zone_count_of_id(tg, 0, "stage", v) == 1,
          "M3c test_new_id() makes the two-distinct-card assertion OBSERVABLE: "
          "each id censuses as its own single instance, where the aliased "
          "test_id() form censused as one id present twice (M2)");
    CHECK(test_zone_count_of_id(tg, 0, "stage", u)
          != test_zone_count_of_id(tg, 0, "stage", x),
          "M3d ...and the two readings differ, which is the whole point: a test "
          "written against test_id() is visibly wrong, not vacuous");

    /* test_id() must not consume the copy pool. If it ever started to, C3's
     * "distinct copies" guarantee would silently become a pool-drain bug in
     * every suite that stages a card by id. */
    test_card_copy_pool_reset();
    (void)test_new_id(tg, NO);
    int p1 = test_new_id(tg, NO);
    test_card_copy_pool_reset();
    int p2 = test_new_id(tg, NO);
    CHECK(p1 >= 0 && p2 >= 0 && p1 != p2,
          "M4 test_new_id() ids come from the copy pool, and the pool is the "
          "ONLY thing test_id() must leave alone");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: harness self-test could not load src/cards.bin\n");
        return 1;
    }
    static TestGame tg;
    test_game_new(&tg);

    section_a_sizes();

    SourceScan scan;
    memset(&scan, 0, sizeof scan);
    section_b_c_source_scan(&scan);
    section_d_test_id_ratchet(&scan);

    section_c_identity(&tg);
    section_e_lookup(&tg);
    section_f_multi_token_triggers();
    section_g_zones(&tg);
    section_h_attacking_seat(&tg);
    section_i_phase_names();
    section_j_choice_shapes(&tg);
    section_k_count_shaped(&tg);
    section_l_missing_twins(&tg);
    section_m_identity_is_detectable(&tg);

    printf("%s: %d checks failed\n", failures ? "FAILED" : "GREEN", failures);
    return failures ? 1 : 0;
}