/* test_parity_per_card3.c -- C parity suite for five Rust clusters whose C
 * coverage boundary was re-established FROM THE BODIES of the existing suites
 * (a file header's coverage claim has been wrong repeatedly in this project,
 * so every "already covered" statement below is backed by a body citation).
 *
 * TAKEN HERE
 *   engine/tests/test_modules/effects/score/per_card/
 *     tokimeki_runners_all_heart_colors_live_success_score_test.rs      5 tests
 *     shizuku_aggregate_score_comparison_draw_test.rs                  4 tests
 *     strawberry_trapper_live_success_compound_score_test.rs           5 tests
 *     excess_heart_score_shift_never_below_zero_pl_n_bp5_010_r_test.rs  2 tests
 *   engine/tests/test_modules/effects/ability_mod/
 *     live_total_threshold_pl_pr_020_pr_test.rs                        1 test
 *   engine/tests/test_modules/effects/gain/blades/per_card/
 *     turn_scope_regression_test.rs                                     2 tests
 *     per_unit_discard_count_regression_test.rs                         1 test
 *   ---- 20 tests
 *
 * COVERAGE BOUNDARY, ESTABLISHED BY READING THE C BODIES (not their headers)
 *
 * ALREADY COVERED -- a peer's header says otherwise, and the header is wrong:
 *   score/per_card/solitude_rain_unique_heart_color_score_q67_test.rs (2 tests)
 *       COVERED, and MORE THOROUGHLY than the Rust, by
 *       test_parity_card_score.c::test_solitude_rain_unique_heart_color
 *       (body at test_parity_card_score.c:742-797). It pins 6 boards on
 *       PL!N-bp1-027-L: empty stage, a 2-colour member, an overlapping pair
 *       -> +3, a Nijigasaki member beside a non-Nijigasaki one, a heart01/03
 *       member, and two members with IDENTICAL colours. The Rust file has only
 *       the +3 and the 0 cases. test_parity_per_card.c:43 lists this file
 *       under "NOT COVERED" -- that header line is FALSE.
 *
 *   score/per_card/vitamin_summer_hand_count_comparison_live_score_test.rs (3)
 *       COVERED by test_parity_characterization_extra.c::
 *       test_l_vitamin_summer_live_success_hand_condition
 *       (body at :1803-1859, registered at :2089). It drives the full
 *       live flow and asserts the same "bonus cleared after the live" claim.
 *       The Rust file's extra assertion (performance_snapshots[0].lives[0]
 *       .score - .base_score == 1) is NOT expressible: RbLiveSnapshot
 *       (rabuka.h:1228-1240) has no base_score field, and the peer says so
 *       in a comment rather than substituting a number. That shape gap
 *       remains open and is NOT re-claimed here.
 *
 *   score/per_card/live_success_surplus_score_zero_floor_test.rs (8 tests)
 *       The 3 that are behavioural engine tests (not abilities.json
 *       introspection) are covered at the EFFECT level by
 *       test_parity_score_effects.c::test_live_total_floor_min_zero
 *       (body at :655-720), which pins add -> +1, remove -> 0, a second
 *       remove -> still 0, a synthesised floorless remove -> -2, and a
 *       score-1 live keeping -1 legal. The remaining 5 are pure
 *       abilities.json schema assertions (effect_constraint=="min:0",
 *       score_floor==0, actions.len()==2, the P/AR rarities) with no C
 *       counterpart worth building.
 *   score/per_card/excess_heart_score_shift_never_below_zero (2 tests)
 *       The mechanism is covered by the suite above at effect level, but the
 *       Rust drives it through a REAL LIVE on a REAL board, and that end-to-
 *       end path is what a whole-column no-op would hide. So both tests are
 *       ported here anyway, as the live-flow cross-check.
 *   score/per_card/takaramono_live_success_no_excess_heart_score_test.rs (2)
 *       PL!-bp3-025-L is referenced by test_parity_cost_and_effect.c:616,653;
 *       body checked, the cost/no-excess interaction is exercised there.
 *
 * NOT COVERED ANYWHERE, and NOT TAKEN -- named, with the reason:
 *   score/per_card/replacement_choose_order_on_live_event_test.rs (2)
 *       NOT PORTABLE. The Rust asserts on `game.state.turn_movements`, a
 *       per-event log of (moved_card_id, cause_player_id). The C GameState
 *       has no such array: `recently_moved[RB_MAX_RECENTLY_MOVED]` is a
 *       BATCH of at most 8 ids with no per-entry causer, and
 *       rb_record_card_movement (rabuka.h:2345) is the only writer. Porting
 *       would mean asserting on a different structure, i.e. faking the test.
 *       SKIPPED.
 *   score/per_card/neutral_live_success_test.rs (6)
 *       NOT PORTABLE. Needs game.generated_actions() / select_generated() /
 *       assert_pending_choice_type(), which have no C shim. SKIPPED (the same
 *       reason test_parity_per_card.c:49-51 gives).
 *   score/per_card/live_success_remaining_yell_live_score_q253_test.rs (4)
 *       SKIPPED -- budget. The yell + live-success ordering it pins is already
 *       driven end-to-end by test_parity_jidou_move_yell.c and
 *       test_parity_ability_watch.c; a fourth walkthrough is worth less than
 *       a fully-controlled cluster.
 *   score/per_card/aurora_flower_set_card_identity_group_match_test.rs (5)
 *       SKIPPED -- budget. AURORA FLOWER (PL!HS-bp5-018-L) set_card_identity is
 *       already covered by test_parity_card_score.c:588
 *       (test_aurora_distinct_name_and_cost) and the SetCardIdentity group term
 *       IS present in C util.c's rb_card_matches_group_str.
 *   score/per_card/energy_state_condition_active_energy_gate_test.rs (7)
 *       SKIPPED -- budget. The active-energy gate itself is covered by
 *       test_parity_card_score.c:1127/1142 (test_own_energy_ahead_live_success,
 *       test_opponent_energy_gated).
 *   blades/per_card/* (the other 9 files, ~31 tests), ALL of effects/deploy,
 *       effects/ability_mod's other 13 files, and integration/multi_card --
 *       NOT TAKEN. 60 files across five clusters is far beyond one agent; this
 *       file takes the 20 with the best value-to-effort, weighted hard toward
 *       score/per_card because PORT_TRACKER.md certifies that mechanic
 *       "Complete" and a failure there is the highest-value finding available.
 *
 * HARNESS LANDMINES HONOURED (all confirmed by peers; see each site)
 *   - load_card_db() falls back to "../cards/build": rb_load("src") alone
 *     fails under the isolated out-of-tree build.
 *   - sizeof(TestGame) is ~781 KB -> every TestGame here is `static`.
 *   - mid() == test_new_id(), never test_id(): several of these fixtures need
 *     two real instances of ONE print.
 *   - rb_card_string() takes a STRING-TABLE INDEX. card_no_of() uses
 *     rb_card_string(c.card_no_idx) and a ROTATING ring, because a single static
 *     buffer called 2-3 times in one printf prints the LAST value every time,
 *     which is a false-red generator. pin_id() is a byte-exact strcmp, because
 *     rb_card_no_eq is FUZZY and a peer found it accept PL!SP-PR-004-R against
 *     the real PL!SP-PR-004-PR.
 *   - test_get_heart_modifier() REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE. Every heart written or read here goes through
 *     rb_mods_add_heart / rb_calc_stage_hearts with an index derived from
 *     rb_parse_heart_color("heartNN"). main() PRINTS the derived indices.
 *   - test_set_live_card(tg, slot, card) is the P1 APPEND path (slot is
 *     ignored); test_set_live_card_for(tg, pl, card) is the seat-aware one.
 *   - test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) DESTROYS the yell
 *     window (one test_pass runs yell through victory and auto-answers every
 *     standing prompt with index 0), so the phase walk stops at
 *     RB_PHASE_PERFORMANCE and steps one phase at a time.
 *   - RbLiveSnapshot has no base_score, so the Rust `score - base_score` delta
 *     is not expressible (see vitamin above).
 *   - rb_card_copy_pool_reset() runs per case via fresh().
 *   - Card.ability is NOT populated by rb_decode_card_by_index; ability
 *     inspection goes through rb_card_num_abilities + rb_decode_card_ability.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *
 * VACUITY CONTROL. A per_card cluster is a grid of one card x one behaviour, so
 * a silent no-op makes a whole column read green. Every negative in this file is
 * therefore paired with a positive on the SAME board shape, and the pair is
 * reported together:
 *     A1 (missing colours -> no +1)  vs  A2 (all six -> +1)     PAIRS
 *     A4 (score 2 -> no recover)     vs  A5 (score 3 -> recover) PAIRS
 *     B1 (P1 wins -> draw 1)         vs  B2 (tied -> no draw)    PAIRS
 *     C2 (both conditions -> +2)     vs  C3/C4/C5 (each half broken) PAIRS
 *     D1/D2 (surplus -> cancel)      vs  the effect-level floor already
 *                                     covered in test_parity_score_effects.c
 *     E1 (live total 8 -> +1)        vs  E2 (live total 7 -> no +1) PAIRS
 *     F1 (facts survive the round)   vs  F2 (facts clear at rollover) PAIRS
 *     G1 (nothing discarded)         vs  "a card really was drawn" (in-test)
 * CHECK_PROMPTS additionally counts a prompt still ON SCREEN at the assertion
 * site, not only at answer time.
 *
 * ASCII-ONLY FILE, ON PURPOSE. The editor can emit CP932, which silently
 * corrupts Japanese literals so group/series filters stop matching. No Japanese
 * appears here at all: card numbers are ASCII, the two trigger tokens are the
 * engine's own RB_TSTR_LIVE_START / RB_TSTR_LIVE_SUCCESS macros rather than
 * literal Japanese, and the U+FF0B FULLWIDTH PLUS is spelled
 * "\xef\xbc\x8b" so a later edit cannot reintroduce the trap silently.
 *
 * No watchdog is achievable on this toolchain. run() is crash ISOLATION, not a
 * hang detector. Every drain is bounded by an explicit iteration cap.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as a byte escape. Unused by the cards below; kept
 * so a future edit does not reintroduce a raw fullwidth character. */
#define FP "\xef\xbc\x8b"

/* ------------------------------------------------------------------ *
 * fixtures -- every card_no below was read out of cards/cards.json as
 * UTF-8 (NOT Get-Content -Raw, which mangles non-ASCII and produces false
 * "missing" results), and the printed values are asserted in the tests so a
 * fixture swap cannot pass silently.
 * ------------------------------------------------------------------ */
#define TOKIMEKI      "PL!N-bp5-026-L"   /* |score 2| need h03:2 h0:4  b_all:1 */
#define SHIZUKU_R     "PL!N-bp4-003-R"   /* |cost 4  | member A.ZU.NA     */
#define SHIZUKU_P     "PL!N-bp4-003-P"   /* |cost 4  | member A.ZU.NA     */
#define STRAWBERRY    "PL!S-pb1-021-L"   /* |score 1 | need h05:2 h0:2    */
#define MIFUNE_R      "PL!N-bp5-010-R"   /* |cost 13 | member R3BIRTH     */
#define MIFUNE_P      "PL!N-bp5-010-P"
#define MIFUNE_AR     "PL!N-bp5-010-AR"
#define HONOKA_PR     "PL!-PR-020-PR"    /* |cost 13 | member Printemps   */
#define CHIKA_SD      "PL!S-sd1-001-SD"   /* |base h02:3 h04:2 h05:2     */
#define RIKO_SD       "PL!S-sd1-011-SD"   /* |base h05:2                 */
#define HONOKA_SD     "PL!-sd1-001-SD"   /* |base h01:1 h03:2 h06:1     */
#define STARTDASH     "PL!-sd1-019-SD"   /* |score 1 | need h01,h03,h06:1*/
#define FILLER        "PL!-sd1-010-SD"
#define AYUMU_SD      "PL!N-sd1-001-SD"   /* |base h01:3 h02:1 h04:1     */
#define CHIKA_R       "PL!S-bp2-001-R"    /* |cost 9  | base h02,h04,h05  */
#define SING_SMILE    "PL!SP-bp1-027-L"   /* |score 6 | need h02,h03,h06:3 h0:5 */

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ------------------------------------------------------------------ *
 * reporting
 * ------------------------------------------------------------------ */
static int failures;
static int assertions;
static int setup_bugs;
static int gaps;
static int prompts;
static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB: every TestGame is a static (BSS) object. */
static TestGame a_tg, b_tg, c_tg, d_tg, e_tg, f_tg, g_tg, h_tg, i_tg, j_tg;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions before the fault: %d, failures: %d, prompts: %d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        assertions++;                                                     \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

/* A Rust assertion with no faithful C entry point. Recorded, never a pass and
 * never a failure -- it stands in for no real CHECK. */
#define EXPECTED_GAP(condition, ...) do {                                \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            printf("GAP: ");                                              \
            printf(__VA_ARGS__);                                          \
            printf("\n");                                                 \
            gaps++;                                                       \
        }                                                                 \
    } while (0)

/* VACUITY CONTROL: the board must have produced at least `n` real prompts
 * before a negative claim can be trusted. The currently-pending prompt is
 * counted HERE, because the test asserts while the prompt is still on screen. */
#define CHECK_PROMPTS(tg_arg, n, ...) do {                               \
        assertions++;                                                     \
        if (pending(tg_arg)) prompts++;                                   \
        if (prompts < (n)) {                                              \
            failures++;                                                   \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) were offered, need %d\n",\
                    prompts, (n));                                        \
        }                                                                 \
    } while (0)

/* ------------------------------------------------------------------ *
 * fixtures / helpers
 * ------------------------------------------------------------------ */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

/* rb_card_string() takes a STRING-TABLE INDEX, not a card id. ROTATING ring: a
 * single static buffer called two or three times inside one printf prints the
 * LAST value every time, which is a false-red generator a peer already hit. */
static const char *card_no_of(int cid)
{
    static char ring[8][96];
    static int k = 0;
    char *buf = ring[k]; k = (k + 1) & 7;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 96, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++; setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* AGENTS.md identity rule. rb_card_no_eq is FUZZY, so the pin is a byte-exact
 * strcmp of the decoded card_no against the requested literal. */
static int pin_id(int cid, const char *no)
{
    assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    setup_bugs++; failures++;
    fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

static int card_cost(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int card_score(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}

static int card_prints_trigger(int cid, const char *trig)
{
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig) != NULL;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* Diagnostic only. Prints the decoded heart table layout so an off-by-one from
 * folding a blade heart into a requirement is visible in the log. */
static void dump_heart_table(int cid, const char *no)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return;
    fprintf(stderr, "  [hearts %s] n=%d num_base=%d num_need=%d num_blade=%d :",
            no, c.n_hearts, c.num_base, c.num_need, c.num_blade);
    for (int i = 0; i < c.n_hearts; i++)
        fprintf(stderr, " (%d x%d)%s", c.heart_color[i], c.heart_count[i],
                i < c.num_blade ? "[blade]"
                    : (i < c.num_blade + c.num_need ? "[need]" : "[base]"));
    fprintf(stderr, "\n");
    rb_free_card(&c);
}

/* Sum of BASE hearts of one colour, read out of the decoded card. This is the
 * same quantity as the Rust `base_heart` map and it is what the stage-heart
 * pipeline rolls up. Derived from the MEASURED layout: a MEMBER is
 * n_hearts = num_base + num_blade with the blade block LAST, a LIVE is
 * num_blade + num_need with the blade block FIRST. Summing the whole table
 * folds a blade heart into the requirement and is off by one. */
static int printed_base_heart(int cid, int color)
{
    Card c; int total = 0;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    for (int i = 0; i < c.num_base && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == color) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

static int score_mod(TestGame *tg, int cid)
{
    return rb_mods_get_score(&tg->state.mods, cid);
}

static int live_total_bonus(TestGame *tg, int pl)
{
    return pl == 0 ? tg->state.mods.p1_constant_total_score_bonus
                   : tg->state.mods.p2_constant_total_score_bonus;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }

static int appeared_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_cards_appeared_this_turn; i++)
        if (tg->state.cards_appeared_this_turn[i] == cid) return 1;
    return 0;
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    tg->state.n_selected_cards = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_cards_appeared_this_turn = 0;
    tg->state.debut_count_this_turn[0] = 0;
    tg->state.debut_count_this_turn[1] = 0;
    tg->state.n_turn_state_changes = 0;
    tg->state.mods.p1_constant_total_score_bonus = 0;
    tg->state.mods.p2_constant_total_score_bonus = 0;
    rb_look_reset_all();
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

/* C's deck bag is TOP-INDEX-0, so an APPEND is Rust's cards.push. */
static void deck_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) { deck_append(tg, 0, filler); deck_append(tg, 1, filler); }
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static void note_prompt(TestGame *tg) { if (pending(tg)) prompts++; }

static void skip(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

/* Rust `drain_choices()` -- answer what cannot be declined, decline what can.
 * A mandatory prompt left parked mid-ability produces exactly the absence the
 * next assertion checks, so nothing fails; every drain here is CAPPED. */
static int drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) skip(tg);
    return guard;
}

static void check_bag_n(const RbBag *bag, int want, const char *what)
{
    assertions++;
    if (bag->n == want) return;
    failures++;
    fprintf(stderr, "FAIL: %s (got %d expected %d):", what, bag->n, want);
    for (int i = 0; i < bag->n && i < 12; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
    fprintf(stderr, "\n");
}

static void check_snapshot_score(const TestGame *tg, int cid, int want_delta, const char *what)
{
    assertions++;
    const RbLiveSnapshot *s = NULL;
    for (int i = tg->state.n_snapshots - 1; i >= 0; i--)
        if (tg->state.snapshots[i].player == 0) { s = &tg->state.snapshots[i]; break; }
    if (!s) {
        failures++;
        fprintf(stderr, "FAIL: %s -- no P1 performance snapshot exists\n", what);
        return;
    }
    /* RbLiveSnapshot has NO base_score field (rabuka.h:1228-1240), so the Rust
     * `l.score - l.base_score` delta is not expressible. What IS available is
     * total_score, so the delta is taken against the ENGINE's decoded printed
     * score, which rule 8 requires be asserted too. */
    int base = card_score(cid);
    int got = s->total_score - base;
    if (got != want_delta) {
        failures++;
        fprintf(stderr, "FAIL: %s (snapshot total_score %d minus the engine's "
                        "decoded base score %d = %d, expected %d)\n",
                what, s->total_score, base, got, want_delta);
    }
}

/* Rust advance_to_live_card_set_p1: five passes out of Main. */
static int to_live_set(TestGame *tg)
{
    if (!test_advance_to_phase(tg, RB_PHASE_LIVE_SET)) {
        failures++; assertions++;
        fprintf(stderr, "FAIL: the phase walk never reached the live-card set\n");
        return 0;
    }
    drain_skip(tg, 16);
    return 1;
}

/* Rust advance_to_live_start: two passes past the live-card set, which is where
 * the live-start window opens. STOPS AT RB_PHASE_PERFORMANCE on purpose:
 * test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) destroys the yell window. */
static int to_live_start(TestGame *tg)
{
    if (!test_advance_to_phase(tg, RB_PHASE_PERFORMANCE)) {
        failures++; assertions++;
        fprintf(stderr, "FAIL: the phase walk never reached the performance phase\n");
        return 0;
    }
    drain_skip(tg, 16);
    return 1;
}

/* The full live: set the live card, reach live start, perform, fire the
 * LiveSuccess window, and clear the bonus at the end of the live. This is the
 * idiom test_parity_characterization_extra.c:1821-1835 already uses. */
static void run_full_live(TestGame *tg, int live)
{
    test_set_live_card(tg, 0, live);
    drain_skip(tg, 16);
    to_live_start(tg);
    rb_perform_live(&tg->state, 0);
    drain_skip(tg, 16);
    tg->state.live_success[0] = 1;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg, 24);
    test_advance_to_phase(tg, RB_PHASE_VICTORY);
    drain_skip(tg, 24);
}

/* ------------------------------------------------------------------ *
 * SECTION A
 *   score/per_card/tokimeki_runners_all_heart_colors_live_success_score
 *   PL!N-bp5-026-L "TOKIMEKI Runners"
 *
 *   ab#0 (live start): if the hearts held by the members on MY stage include
 *     heart01 through heart06 ALL of them, +1 to this card's score.
 *   ab#1 (live success): if this card's score is 3, add 1 "Nijigasaki" card
 *     from my waitroom to my hand.
 *
 *   The decoded ab#0 condition is a location_condition carrying
 *     target: "self", location: "stage", card_type: "member_card",
 *     heart_colors: [heart01..heart06], and NO count / operator key.
 * ------------------------------------------------------------------ */

/* The two stage members are supplied by the caller so A1 and A2 differ ONLY in
 * which heart colours the stage carries -- that is what makes the pair a real
 * control rather than two unrelated boards. */
static int tokimeki_live_start_board(TestGame *tg, int m_a, int m_b, int *tokimeki_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int tokimeki = mid(tg, TOKIMEKI);
    if (tokimeki < 0) return 0;
    if (!pin_id(tokimeki, TOKIMEKI)) return 0;
    /* Rule 8: a printed score must be pinned on the ENGINE's decoded value too,
     * because a cards.json / cards.bin disagreement would make Q232's
     * "the modifier is separate from the base" claim silently unfalsifiable. */
    CHECK_EQ(card_cost(tokimeki), 0, "tokimeki: the print has no member cost");
    CHECK_EQ(card_score(tokimeki), 2, "tokimeki: the ENGINE decodes the printed score 2");
    CHECK(card_prints_trigger(tokimeki, RB_TSTR_LIVE_START),
          "tokimeki: the print carries a live-start ability");

    int filler = mid(tg, FILLER);
    if (filler < 0) return 0;
    fill_decks(tg, filler, 20);
    test_add_to_hand(tg, tokimeki);

    set_stage(tg, 0, AREA_LEFT, m_a);
    set_stage(tg, 0, AREA_CENTER, m_b);
    set_stage(tg, 0, AREA_RIGHT, filler);
    *tokimeki_out = tokimeki;
    return 1;
}

/* Rust tokimeki_q216_missing_hearts_no_score -- the stage carries heart02/04/05
 * only, so heart01, heart03 and heart06 are all absent and the +1 must NOT
 * fire. THE PAIRED POSITIVE IS test_a2 below, on the same helper. */
static void test_a1_tokimeki_missing_hearts_no_score(void)
{
    TestGame *tg = &a_tg;
    int chika = mid(tg, CHIKA_SD), riko = mid(tg, RIKO_SD);
    if (chika < 0 || riko < 0) return;
    if (!pin_id(chika, CHIKA_SD) || !pin_id(riko, RIKO_SD)) return;

    /* Fixture premise, pinned against cards.json read as UTF-8:
     *   CHIKA_SD base_heart {heart02:3, heart04:2, heart05:2}
     *   RIKO_SD  base_heart {heart05:2}
     * Together {02,04,05}: heart01/03/06 absent.
     * NOTE: the Rust test's own comment claims RIKO_SD carries
     * "heart02:1, heart05:2". cards.json says heart05:2 ONLY. The conclusion
     * (heart01/03/06 still missing -> no +1) is unchanged, but the Rust
     * comment is wrong and is reported as a fixture-comment bug. */
    int h01 = rb_parse_heart_color("heart01");
    int h02 = rb_parse_heart_color("heart02");
    int h03 = rb_parse_heart_color("heart03");
    int h05 = rb_parse_heart_color("heart05");
    int h06 = rb_parse_heart_color("heart06");
    dump_heart_table(chika, CHIKA_SD);
    dump_heart_table(riko, RIKO_SD);
    CHECK(printed_base_heart(chika, h01) == 0 && printed_base_heart(riko, h01) == 0,
          "a1 premise: no stage member carries heart01");
    CHECK(printed_base_heart(chika, h03) == 0 && printed_base_heart(riko, h03) == 0,
          "a1 premise: no stage member carries heart03");
    CHECK(printed_base_heart(chika, h06) == 0 && printed_base_heart(riko, h06) == 0,
          "a1 premise: no stage member carries heart06");
    CHECK_EQ(printed_base_heart(chika, h02), 3, "a1 premise: CHIKA_SD carries heart02 x3");
    CHECK_EQ(printed_base_heart(chika, h05), 2, "a1 premise: CHIKA_SD carries heart05 x2");

    int tokimeki = -1;
    if (!tokimeki_live_start_board(tg, chika, riko, &tokimeki)) return;
    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, tokimeki);
    /* Rust asserts there is no prompt at live start; nothing to answer. */
    CHECK(!pending(tg), "a1: no prompt is offered when the live card is set");
    if (!to_live_start(tg)) return;

    CHECK_EQ(score_mod(tg, tokimeki), 0,
             "Q216: heart01/03/06 absent from the stage -> no +1 score");
}

/* Rust tokimeki_all_6_colors_collectively_score_plus_1 -- THE PAIRED POSITIVE.
 * Same helper, same card, same everything except the members now cover all six
 * colours. If A1 and A2 return the same value, the heart-colour gate is
 * provably not being read. */
static void test_a2_tokimeki_all6_colors_score_plus1(void)
{
    TestGame *tg = &b_tg;
    int chika = mid(tg, CHIKA_SD), honoka = mid(tg, HONOKA_SD);
    if (chika < 0 || honoka < 0) return;
    if (!pin_id(chika, CHIKA_SD) || !pin_id(honoka, HONOKA_SD)) return;
    /* Fixture premise, pinned against cards.json:
     *   CHIKA_SD  {heart02:3, heart04:2, heart05:2}
     *   HONOKA_SD {heart01:1, heart03:2, heart06:1}
     * Together: all six colours. */
    int h01 = rb_parse_heart_color("heart01");
    int h03 = rb_parse_heart_color("heart03");
    int h06 = rb_parse_heart_color("heart06");
    dump_heart_table(honoka, HONOKA_SD);
    CHECK_EQ(printed_base_heart(honoka, h01), 1, "a2 premise: HONOKA_SD carries heart01 x1");
    CHECK_EQ(printed_base_heart(honoka, h03), 2, "a2 premise: HONOKA_SD carries heart03 x2");
    CHECK_EQ(printed_base_heart(honoka, h06), 1, "a2 premise: HONOKA_SD carries heart06 x1");

    int tokimeki = -1;
    if (!tokimeki_live_start_board(tg, chika, honoka, &tokimeki)) return;
    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, tokimeki);
    if (!to_live_start(tg)) return;

    CHECK_EQ(score_mod(tg, tokimeki), 1,
             "Q216: all six colours collectively on stage -> +1 score");
}

/* Rust tokimeki_q232_modifier_separate_from_base_score -- the +1 is a modifier,
 * not a rewrite of the printed score. The score is re-read from the decoded
 * card AFTER the live, so a modifier that mutated Card.score is visible here. */
static void test_a3_tokimeki_modifier_separate_from_base(void)
{
    TestGame *tg = &c_tg;
    int chika = mid(tg, CHIKA_SD), honoka = mid(tg, HONOKA_SD);
    if (chika < 0 || honoka < 0) return;
    if (!pin_id(chika, CHIKA_SD) || !pin_id(honoka, HONOKA_SD)) return;
    int tokimeki = -1;
    if (!tokimeki_live_start_board(tg, chika, honoka, &tokimeki)) return;
    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, tokimeki);
    if (!to_live_start(tg)) return;

    CHECK_EQ(score_mod(tg, tokimeki), 1, "Q232: the +1 lands as a modifier");
    CHECK_EQ(card_score(tokimeki), 2,
             "Q232: the printed/encoded base score is still 2, untouched by the +1");
}

/* Rust setup_tokimeki_live_success + tokimeki_live_success_score_three_
 * recovers_nijigasaki_card. The window is forced open explicitly, because the
 * paired negative below is exactly the absence this one measures: a CLOSED
 * window would satisfy that negative vacuously. */
static int tokimeki_live_success_board(TestGame *tg, int score_modifier,
                                       int *live_out, int *recoverable_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int live = mid(tg, TOKIMEKI);
    int recoverable = mid(tg, SHIZUKU_R);   /* a Nijigasaki (A.ZU.NA) member */
    if (live < 0 || recoverable < 0) return 0;
    if (!pin_id(live, TOKIMEKI) || !pin_id(recoverable, SHIZUKU_R)) return 0;
    CHECK_EQ(card_cost(recoverable), 4, "tokimeki fixture: the recoverable card costs 4");
    CHECK_EQ(card_score(recoverable), 0, "tokimeki fixture: the recoverable card scores 0");

    int filler = mid(tg, FILLER);
    if (filler < 0) return 0;
    fill_decks(tg, filler, 20);
    test_add_to_hand(tg, filler);

    /* Wild hearts plus the zoned live card, exactly as the Rust helper builds
     * stage_hearts {heart03: 2, heart00: 4}. */
    int h03 = rb_parse_heart_color("heart03");
    int h00 = rb_parse_heart_color("heart00");
    tg->state.stage_hearts[0][h03] = 2;
    tg->state.stage_hearts[0][h00] = 4;

    test_add_to_live(tg, live);
    test_add_to_discard(tg, recoverable);
    rb_mods_set_score(&tg->state.mods, live, score_modifier);
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.active = 0;
    tg->state.live_success[0] = 1;

    /* THE vacuity control: the live-success window must be OPEN, or both the
     * positive below and the negative in a5 measure nothing. */
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "tokimeki live-success: the live-success window must be OPEN");

    *live_out = live;
    *recoverable_out = recoverable;
    return 1;
}

static void test_a4_tokimeki_score3_recovers_nijigasaki(void)
{
    TestGame *tg = &d_tg;
    int live = -1, recoverable = -1;
    if (!tokimeki_live_success_board(tg, 1, &live, &recoverable)) return;

    tg->state.activating_card = live;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg, 16);

    CHECK(hand_has(tg, recoverable),
          "tokimeki live-success: score 3 -> the Nijigasaki waitroom card reaches hand");
    CHECK(!wait_has(tg, recoverable),
          "tokimeki live-success: the recovered card leaves the waitroom");
}

/* Rust tokimeki_live_success_score_two_does_not_recover_card -- THE PAIRED
 * NEGATIVE. Identical board to a4 with the score modifier one lower. */
static void test_a5_tokimeki_score2_does_not_recover(void)
{
    TestGame *tg = &e_tg;
    int live = -1, recoverable = -1;
    if (!tokimeki_live_success_board(tg, 0, &live, &recoverable)) return;

    tg->state.activating_card = live;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg, 16);

    CHECK(!hand_has(tg, recoverable),
          "tokimeki live-success: score 2 -> the waitroom card stays out of hand");
    CHECK(wait_has(tg, recoverable),
          "tokimeki live-success: the waitroom card is untouched at score 2");
}

/* ------------------------------------------------------------------ *
 * SECTION B
 *   score/per_card/shizuku_aggregate_score_comparison_draw_test.rs
 *   PL!N-bp4-003-R / -P "Sakurazaki Shizuku"
 *   live success: if MY total live score is HIGHER than the opponent's,
 *   draw 1 card.
 *
 *   The decoded condition is a comparison_condition carrying
 *     comparison_target: "opponent", operator: ">",
 *     comparison_type: "score", aggregate: "total"
 *   with NO `position` key.
 * ------------------------------------------------------------------ */
static int shizuku_setup(TestGame *tg, const char *card_no, int p2_live, int *shizuku_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int shizuku = mid(tg, card_no);
    if (shizuku < 0) return 0;
    if (!pin_id(shizuku, card_no)) return 0;
    CHECK_EQ(card_cost(shizuku), 4, "shizuku: the print's cost");
    CHECK(card_prints_trigger(shizuku, RB_TSTR_LIVE_SUCCESS),
          "shizuku: the print carries a live-success ability");

    int filler = mid(tg, FILLER);
    if (filler < 0) return 0;
    fill_decks(tg, filler, 30);
    test_give_energy(tg, 5);

    /* Shizuku sits on P1's own stage, flanked by two 13-cost starter members,
     * exactly as the Rust fixture does. */
    int m1 = mid(tg, HONOKA_SD), m2 = mid(tg, HONOKA_SD);
    if (m1 < 0 || m2 < 0) return 0;
    set_stage(tg, 0, AREA_LEFT, m1);
    set_stage(tg, 0, AREA_CENTER, shizuku);
    set_stage(tg, 0, AREA_RIGHT, m2);

    int live = mid(tg, STARTDASH);
    if (live < 0) return 0;
    CHECK_EQ(card_score(live), 1, "shizuku: the live card prints score 1");
    test_add_to_hand(tg, live);

    if (p2_live) {
        int pm1 = mid(tg, HONOKA_SD), pm2 = mid(tg, HONOKA_SD), pm3 = mid(tg, HONOKA_SD);
        int p2card = mid(tg, STARTDASH);
        if (pm1 < 0 || pm2 < 0 || pm3 < 0 || p2card < 0) return 0;
        set_stage(tg, 1, AREA_LEFT, pm1);
        set_stage(tg, 1, AREA_CENTER, pm2);
        set_stage(tg, 1, AREA_RIGHT, pm3);
        test_add_to_hand_for(tg, 1, p2card);
    } else {
        tg->state.p[1].hand.n = 0;   /* P2 cannot set a live card -> P1 auto-wins */
    }
    *shizuku_out = shizuku;
    return 1;
}

/* The live card is the only card the Rust fixture puts in P1's hand, so the
 * caller can read it back by index. */
static int p1_live_in_hand(TestGame *tg)
{
    if (tg->state.p[0].hand.n < 1) return -1;
    return tg->state.p[0].hand.cards[0];
}

/* Rust shizuku_higher_score_draws_card. Ported VERBATIM, including its
 * bookkeeping -- and the bookkeeping is itself a fixture weakness worth
 * recording: the Rust records hand_before BEFORE the five setup passes, so the
 * "+1" it asserts is the DRAW PHASE's own card PLUS any live-success draw.
 * That makes the Rust positive satisfiable without the ability ever running.
 * The hand count is asserted exactly as Rust states it, and CHECK_PROMPTS
 * additionally demands that the flow really produced prompts. */
static void test_b1_shizuku_higher_score_draws(void)
{
    TestGame *tg = &f_tg;
    int shizuku = -1;
    if (!shizuku_setup(tg, SHIZUKU_R, 0, &shizuku)) return;
    int live = p1_live_in_hand(tg);
    if (live < 0) { EXPECTED_GAP(0, "b1: P1 has no live card in hand"); return; }

    int hand_before = tg->state.p[0].hand.n;
    run_full_live(tg, live);
    check_bag_n(&tg->state.p[0].hand, hand_before + 1,
                "Higher live score -> draw 1 card (Rust: hand == hand_before + 1)");

    CHECK_PROMPTS(tg, 1,
                  "shizuku: the live flow must actually offer prompts, otherwise "
                  "hand_before+1 is satisfied by the Draw phase alone");
}

/* Rust shizuku_tied_score_no_draw -- THE PAIRED NEGATIVE: identical board with
 * P2 fielding the same score-1 live card, so the comparison is a tie. */
static void test_b2_shizuku_tied_score_no_draw(void)
{
    TestGame *tg = &g_tg;
    int shizuku = -1;
    if (!shizuku_setup(tg, SHIZUKU_R, 1, &shizuku)) return;
    int live = p1_live_in_hand(tg);
    if (live < 0) { EXPECTED_GAP(0, "b2: P1 has no live card in hand"); return; }

    int hand_before = tg->state.p[0].hand.n;
    run_full_live(tg, live);
    check_bag_n(&tg->state.p[0].hand, hand_before,
                "Tied live score -> no draw (Rust: hand == hand_before)");
    /* Paired control: the same flow MUST have produced prompts, otherwise this
     * 'no draw' passes because nothing happened at all. */
    CHECK_PROMPTS(tg, 1,
                  "shizuku: the tied-score flow must offer prompts too, otherwise "
                  "the no-draw result is vacuous");
}

static void test_b3_shizuku_p2_no_live_card_p1_wins(void)
{
    TestGame *tg = &h_tg;
    int shizuku = -1;
    if (!shizuku_setup(tg, SHIZUKU_R, 0, &shizuku)) return;
    int live = p1_live_in_hand(tg);
    if (live < 0) { EXPECTED_GAP(0, "b3: P1 has no live card in hand"); return; }
    int hand_before = tg->state.p[0].hand.n;
    run_full_live(tg, live);
    check_bag_n(&tg->state.p[0].hand, hand_before + 1,
                "P2 has no live card -> P1 wins -> draws 1");
}

static void test_b4_shizuku_p_rarity_same_as_r(void)
{
    TestGame *tg = &i_tg;
    int shizuku = -1;
    if (!shizuku_setup(tg, SHIZUKU_P, 0, &shizuku)) return;
    int live = p1_live_in_hand(tg);
    if (live < 0) { EXPECTED_GAP(0, "b4: P1 has no live card in hand"); return; }
    int hand_before = tg->state.p[0].hand.n;
    run_full_live(tg, live);
    check_bag_n(&tg->state.p[0].hand, hand_before + 1,
                "P rarity: the same draw-on-win behaviour as the R print");
}

/* ------------------------------------------------------------------ *
 * SECTION C
 *   score/per_card/strawberry_trapper_live_success_compound_score_test.rs
 *   PL!S-pb1-021-L "Strawberry Trapper"
 *   live success: +2 to this card's score if the hearts held by the "Aqours"
 *   members on my stage hold 4 or more heart05 IN TOTAL, and this turn the
 *   opponent succeeded at a live with NO excess hearts.
 *
 *   ab#0 is a compound(and) of
 *     (A) group_condition: group_names ["Aqours"], location stage,
 *         heart_colors ["heart05"], card_type member_card, count 4,
 *         operator >=, aggregate total
 *     (B) temporal_condition(this_turn, opponent_live_success{no_excess_heart})
 *   The effect is modify_score(+2, self_target).
 * ------------------------------------------------------------------ */
static int strawberry_setup(TestGame *tg, int stage_member, int with_heart_mod,
                            int p2_success, int no_excess, int *straw_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int straw = mid(tg, STRAWBERRY);
    if (straw < 0) return 0;
    if (!pin_id(straw, STRAWBERRY)) return 0;
    CHECK_EQ(card_score(straw), 1, "strawberry: the print's score is 1");
    CHECK(card_prints_trigger(straw, RB_TSTR_LIVE_SUCCESS),
          "strawberry: the print carries a live-success ability");

    int filler = mid(tg, FILLER);
    if (filler < 0) return 0;
    fill_decks(tg, filler, 20);
    test_add_to_hand(tg, straw);
    set_stage(tg, 0, AREA_CENTER, stage_member);

    /* Condition (B): the opponent won THIS turn, with / without excess hearts.
     * The C twin of Rust's p2_live_success_this_turn + p2_live_success_no_excess
     * is live_success[1] plus p2_live_success_no_excess, which is exactly what
     * rb_set_opponent_live_success writes (game_state_abilities.c:910). Both
     * are set so the test cannot depend on g->active being 0. */
    tg->state.live_success[1] = p2_success ? 1 : 0;
    tg->state.p2_live_success_no_excess = no_excess;
    if (p2_success) rb_set_opponent_live_success(&tg->state, no_excess);

    if (with_heart_mod) {
        /* Rust mods.add_heart_modifier(chika, Heart05, 4). Derived through
         * rb_parse_heart_color, NEVER the literal 5: test_get_heart_modifier
         * remaps a requested colour of 5 onto RB_HEART_ORANGE. */
        int h05 = rb_parse_heart_color("heart05");
        rb_mods_add_heart(&tg->state.mods, stage_member, h05, 4);
        CHECK_EQ(rb_mods_get_heart(&tg->state.mods, stage_member, h05), 4,
                 "strawberry fixture: heart05 +4 is staged on the centre member");
    }
    *straw_out = straw;
    return 1;
}

static void strawberry_fire(TestGame *tg, int straw)
{
    rb_perform_live(&tg->state, 0);
    drain_skip(tg, 16);
    /* Q36: live success resolves during the victory determination, and the
     * bonus is CLEARED once the live ends. */
    tg->state.live_success[0] = 1;
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.activating_card = straw;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg, 24);
}

/* Rust strawberry_q36_only_fires_in_live_victory_determination. The Rust test
 * makes no behavioural claim of its own -- it walks to the LiveVictoryDetermination
 * phase and asserts nothing about the card -- so the meaningful translation is
 * the PHASE PIN it takes on the way plus the "no prompt at live start" claim. */
static void test_c1_strawberry_q36_timing(void)
{
    TestGame *tg = &a_tg;
    int chika = mid(tg, CHIKA_SD);
    if (chika < 0) return;
    int straw = -1;
    if (!strawberry_setup(tg, chika, 0, 0, 0, &straw)) return;

    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, straw);
    CHECK(!pending(tg), "q36: no prompt is expected at live start");
    if (!to_live_start(tg)) return;

    CHECK_EQ(tg->state.phase, RB_PHASE_PERFORMANCE,
             "q36: two passes past the live-card set land in the performance phase");
    /* The Rust assertion "the opponent's live success is false before
     * LiveVictoryDetermination" is a fixture precondition this board satisfies
     * by construction. */
    CHECK_EQ(tg->state.live_success[1], 0,
             "q36 precondition: the opponent's live success is not set yet");
    CHECK_EQ(score_mod(tg, straw), 0,
             "q36: nothing has been applied before the victory determination");
}

/* Rust strawberry_q132_first_attacker_evaluated -- BOTH halves of the compound
 * satisfied, so the +2 must land in the performance snapshot. THE POSITIVE that
 * the three negatives below are paired against. */
static void test_c2_strawberry_both_conditions_plus2(void)
{
    TestGame *tg = &b_tg;
    int chika = mid(tg, CHIKA_SD);
    if (chika < 0) return;
    int straw = -1;
    if (!strawberry_setup(tg, chika, 1, 1, 1, &straw)) return;

    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, straw);
    if (!to_live_start(tg)) return;
    strawberry_fire(tg, straw);

    CHECK_EQ(score_mod(tg, straw), 0,
             "q132: the live-success score bonus is cleared after the live");
    check_snapshot_score(tg, straw, 2,
                         "q132: both compound conditions met -> the +2 is in the "
                         "final performance score");
}

/* Rust strawberry_q142_excess_heart_prevents_score -- condition (B) fails on
 * the no_excess_heart half. */
static void test_c3_strawberry_excess_heart_blocks(void)
{
    TestGame *tg = &c_tg;
    int chika = mid(tg, CHIKA_SD);
    if (chika < 0) return;
    int straw = -1;
    if (!strawberry_setup(tg, chika, 1, 1, 0, &straw)) return;

    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, straw);
    if (!to_live_start(tg)) return;
    strawberry_fire(tg, straw);

    /* The Rust test only asserts when the live actually succeeded, so the
     * translation asserts the same thing AND records which branch ran. */
    if (tg->state.p[0].success.n > 0) {
        int target = tg->state.p[0].success.cards[0];
        CHECK_EQ(score_mod(tg, target), 0,
                 "q142: excess hearts block the temporal half -> no score bonus");
        check_snapshot_score(tg, target, 0,
                             "q142: excess hearts -> no +2 in the final score");
    } else {
        EXPECTED_GAP(0,
                     "q142: the live card did not reach the success zone, so the "
                     "Rust test's guarded assertion never runs. Recorded as a gap, "
                     "never as a pass.");
    }
}

/* Rust strawberry_q142_wrong_group_prevents_score -- condition (A) fails: the
 * centre member is not an Aqours member. HONOKA_SD (Printemps) is the same
 * fixture the Rust file uses, and its base_heart is
 * {heart01:1, heart03:2, heart06:1} -- it carries NO heart05 at all, so BOTH
 * halves of (A) fail (wrong group AND zero heart05). That makes this a weaker
 * negative than the Rust intended; the stronger single-variable negative is the
 * +2 in c2, where the member is Chika with a staged heart05 x4. */
static void test_c4_strawberry_wrong_group_blocks(void)
{
    TestGame *tg = &d_tg;
    int other = mid(tg, HONOKA_SD);
    if (other < 0) return;
    if (!pin_id(other, HONOKA_SD)) return;
    int straw = -1;
    if (!strawberry_setup(tg, other, 0, 1, 1, &straw)) return;

    int h05 = rb_parse_heart_color("heart05");
    CHECK_EQ(printed_base_heart(other, h05), 0,
             "c4 premise: the non-Aqours centre member carries no heart05 at all");

    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, straw);
    if (!to_live_start(tg)) return;
    strawberry_fire(tg, straw);

    if (tg->state.p[0].success.n > 0) {
        int target = tg->state.p[0].success.cards[0];
        check_snapshot_score(tg, target, 0,
                             "q142: a non-Aqours centre member -> no +2 in the "
                             "final score");
    } else {
        EXPECTED_GAP(0,
                     "q142 wrong-group: the live card did not reach the success "
                     "zone (heart requirements unmet with the filler stage), so "
                     "the Rust assertion never runs. Gap, not a pass.");
    }
}

/* Rust strawberry_opponent_didnt_win_no_score -- condition (B) fails on the
 * opponent_live_success half. */
static void test_c5_strawberry_opponent_did_not_win(void)
{
    TestGame *tg = &e_tg;
    int chika = mid(tg, CHIKA_SD);
    if (chika < 0) return;
    int straw = -1;
    if (!strawberry_setup(tg, chika, 1, 0, 0, &straw)) return;

    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, straw);
    if (!to_live_start(tg)) return;
    strawberry_fire(tg, straw);

    int target = -1;
    if (tg->state.p[0].success.n > 0) target = tg->state.p[0].success.cards[0];
    else if (tg->state.p[0].live.n > 0) target = tg->state.p[0].live.cards[0];
    if (target < 0) {
        EXPECTED_GAP(0,
                     "opponent_didnt_win: neither live zone holds the card, so the "
                     "Rust test's guarded assertion never runs. Gap, not a pass.");
        return;
    }
    CHECK_EQ(score_mod(tg, target), 0,
             "opponent did not win -> the compound temporal half fails -> no bonus");
    check_snapshot_score(tg, target, 0,
                         "opponent did not win -> no +2 in the final score");
}

/* ------------------------------------------------------------------ *
 * SECTION D
 *   score/per_card/excess_heart_score_shift_never_below_zero_pl_n_bp5_010_r
 *   PL!N-bp5-010-R / -P / -AR "Mifune Shizuko"
 *   live success: no excess hearts -> live total +1 ; 2 or more excess hearts ->
 *   live total -1, floored at 0. Sequential, with effect_constraint min:0 on
 *   both children.
 *
 *   The effect-level floor is already covered by
 *   test_parity_score_effects.c::test_live_total_floor_min_zero (:655-720). What
 *   is ported here is the END-TO-END live path, because that is the layer a
 *   whole-column no-op hides.
 * ------------------------------------------------------------------ */
static int mifune_board(TestGame *tg, const char *card_no, int *mifune_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int mifune = mid(tg, card_no);
    if (mifune < 0) return 0;
    if (!pin_id(mifune, card_no)) return 0;
    CHECK_EQ(card_cost(mifune), 13, "mifune: the print's cost");
    CHECK(card_prints_trigger(mifune, RB_TSTR_LIVE_SUCCESS),
          "mifune: the print carries a live-success ability");

    int filler  = mid(tg, FILLER);
    int filler2 = mid(tg, FILLER);
    int ayumu   = mid(tg, AYUMU_SD);
    int ayumu2  = mid(tg, AYUMU_SD);
    int ayumu3  = mid(tg, AYUMU_SD);
    if (filler < 0 || filler2 < 0 || ayumu < 0 || ayumu2 < 0 || ayumu3 < 0) return 0;
    if (!pin_id(ayumu, AYUMU_SD)) return 0;
    CHECK_EQ(printed_base_heart(ayumu, rb_parse_heart_color("heart01")), 3,
             "mifune fixture: AYUMU_SD carries heart01 x3");

    /* Three 13-cost members on stage means the live's hearts go unmet, exactly
     * as the Rust fixture does; P1's deck is full of members so hearts ARE in
     * surplus, which is the branch under test. */
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 40; i++) { deck_append(tg, 0, ayumu); deck_append(tg, 1, filler); }

    set_stage(tg, 0, AREA_LEFT, ayumu);
    set_stage(tg, 0, AREA_CENTER, ayumu2);
    set_stage(tg, 0, AREA_RIGHT, ayumu3);
    test_add_to_hand(tg, mifune);
    test_add_to_hand(tg, filler2);
    *mifune_out = mifune;
    return 1;
}

/* Rust mifune_q231_excess_heart_2_score_cancels_to_0 -- 2 or more surplus
 * hearts, so the +1 (no surplus) and the -1 (2+ surplus) branches are mutually
 * exclusive and the net is 0. */
static void test_d1_mifune_q231_surplus_cancels(void)
{
    TestGame *tg = &f_tg;
    int mifune = -1;
    if (!mifune_board(tg, MIFUNE_R, &mifune)) return;

    /* The live-success window needs a live card in the zone to scan. */
    int live = mid(tg, SING_SMILE);
    if (live < 0) return;
    test_add_to_live(tg, live);

    /* Wild hearts so the requirement is trivially met and the surplus is
     * unambiguously positive. */
    int st[8];
    tg->state.stage_hearts[0][rb_parse_heart_color("heart00")] = 40;
    rb_calc_stage_hearts(&tg->state, 0, st);
    CHECK(st[rb_parse_heart_color("heart00")] == 40,
          "d1 premise: the stage really does hold 40 wild hearts");

    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    tg->state.activating_card = mifune;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "mifune: the live-success window must be OPEN");

    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg, 16);

    CHECK_EQ(score_mod(tg, mifune), 0,
             "Q231: with 2+ surplus hearts the +1 and the -1 cancel to 0");
}

/* Rust mifune_q231_excess_heart_2_score_cancels_to_0_all_rarities -- the same
 * board on the AR, P and R prints. */
static void test_d2_mifune_all_rarities_cancel(void)
{
    static const char *const RARITIES[3] = { MIFUNE_AR, MIFUNE_P, MIFUNE_R };
    for (int r = 0; r < 3; r++) {
        TestGame *tg = (r == 0) ? &g_tg : (r == 1) ? &h_tg : &i_tg;
        int mifune = -1;
        if (!mifune_board(tg, RARITIES[r], &mifune)) continue;

        int live = mid(tg, SING_SMILE);
        if (live < 0) continue;
        test_add_to_live(tg, live);
        tg->state.stage_hearts[0][rb_parse_heart_color("heart00")] = 40;

        tg->state.phase = RB_PHASE_VICTORY;
        tg->state.live_success[0] = 1;
        tg->state.activating_card = mifune;
        CHECK(rb_should_trigger_live_success(&tg->state, 0),
              "%s: the live-success window must be OPEN", RARITIES[r]);

        rb_trigger_live_success(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
        drain_skip(tg, 16);

        CHECK_EQ(score_mod(tg, mifune), 0,
                 "%s: Q231 +1 and -1 must cancel to 0", RARITIES[r]);
    }
}

/* ------------------------------------------------------------------ *
 * SECTION E
 *   effects/ability_mod/live_total_threshold_pl_pr_020_pr_test.rs
 *   PL!-PR-020-PR "Honoka", at live start:
 *   "if the total score of the live cards in MY live card zone is 8 or more,
 *    gain the constant ability [live total score +1]".
 *   (Transcribed in English on purpose -- this file is ASCII-only, see the
 *    header. The authoritative text is abilities.json, quoted structurally
 *    below.)
 *
 *   The decoded condition is a comparison_condition carrying
 *     target: "self", location: "live_card_zone", card_type: "live_card",
 *     count: 8, operator: ">=", comparison_type: "score", aggregate: "total",
 *     position: "center", activation_position: "center"
 *   and the effect is gain_ability with gained_effect modify_score +1
 *   targeted at live_total.
 *
 *   The C suite gains a PAIRED NEGATIVE the Rust file lacks: the same board
 *   with a live total of 7 instead of 8, which must NOT grant the +1. Without
 *   it a permanently-on constant would read green.
 * ------------------------------------------------------------------ */
static int honoka_board(TestGame *tg, int n_startdash, int *honoka_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int honoka = mid(tg, HONOKA_PR);
    if (honoka < 0) return 0;
    if (!pin_id(honoka, HONOKA_PR)) return 0;
    CHECK_EQ(card_cost(honoka), 13, "honoka: the print's cost");
    CHECK(card_prints_trigger(honoka, RB_TSTR_LIVE_START),
          "honoka: the print carries a live-start ability");
    set_stage(tg, 0, AREA_CENTER, honoka);

    int smile = mid(tg, SING_SMILE);   /* printed score 6 */
    if (smile < 0) return 0;
    CHECK_EQ(card_score(smile), 6, "honoka fixture: SING_SMILE prints score 6");
    test_add_to_live(tg, smile);

    int total = card_score(smile);
    for (int i = 0; i < n_startdash; i++) {
        int sd = mid(tg, STARTDASH);   /* printed score 1 */
        if (sd < 0) return 0;
        if (i == 0) CHECK_EQ(card_score(sd), 1, "honoka fixture: START:DASH!! prints score 1");
        test_add_to_live(tg, sd);
        total += card_score(sd);
    }

    /* Wild hearts so the live requirement is met and the window opens. */
    tg->state.stage_hearts[0][rb_parse_heart_color("heart00")] = 20;

    CHECK_EQ(tg->state.p[0].live.n, n_startdash + 1,
             "honoka fixture: every live card landed in the live zone");
    printf("  [honoka] live zone printed total = %d over %d cards\n",
           total, tg->state.p[0].live.n);
    *honoka_out = honoka;
    return 1;
}

static void honoka_fire(TestGame *tg, int honoka)
{
    tg->state.activating_card = honoka;
    rb_trigger_live_start(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg, 16);
}

/* Rust pl_pr_020_pr_live_total_eight_grants_constant_score_ability:
 * 6 + 1 + 1 = 8 live total -> +1 live total. THE POSITIVE. */
static void test_e1_honoka_live_total_eight_grants(void)
{
    TestGame *tg = &a_tg;
    int honoka = -1;
    if (!honoka_board(tg, 2, &honoka)) return;
    CHECK_EQ(tg->state.p[0].live.n, 3, "e1: three live cards, total 6+1+1 = 8");
    honoka_fire(tg, honoka);
    CHECK_EQ(live_total_bonus(tg, 0), 1,
             "live total >= 8 -> gained the constant live-total +1");
}

/* PAIRED NEGATIVE (added here; the Rust file has no counterpart). One fewer
 * START:DASH!!, so the live total is 7 and the gate must not open. */
static void test_e2_honoka_live_total_seven_does_not_grant(void)
{
    TestGame *tg = &b_tg;
    int honoka = -1;
    if (!honoka_board(tg, 1, &honoka)) return;
    CHECK_EQ(tg->state.p[0].live.n, 2, "e2: two live cards, total 6+1 = 7");
    honoka_fire(tg, honoka);
    CHECK_EQ(live_total_bonus(tg, 0), 0,
             "live total 7 < 8 -> the constant must NOT be granted");
}

/* ------------------------------------------------------------------ *
 * SECTION F
 *   effects/gain/blades/per_card/turn_scope_regression_test.rs
 *   Round-scope lifetime of the "this turn" tracking family.
 *   reset_keyword_tracking() used to run on EVERY Active-phase entry, twice
 *   per round, truncating first-attacker-window facts before any live-start /
 *   live-success condition could read them. Seventeen abilities of that shape
 *   depend on the facts surviving both players' normal phases. These tests pin
 *   the LIFETIME directly, which is what makes a future clear-site regression
 *   fail HERE with a named cause instead of as 17 silent card misbehaviours.
 * ------------------------------------------------------------------ */
static int turn_scope_setup(TestGame *tg, int *member_out, int *live_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int member = mid(tg, CHIKA_R);    /* cost 9 */
    int filler = mid(tg, FILLER);
    int live   = mid(tg, STARTDASH);
    if (member < 0 || filler < 0 || live < 0) return 0;
    if (!pin_id(member, CHIKA_R) || !pin_id(live, STARTDASH)) return 0;
    CHECK_EQ(card_cost(member), 9, "turn_scope fixture: CHIKA_R costs 9");

    fill_decks(tg, filler, 30);
    test_add_to_hand(tg, member);
    test_add_to_hand(tg, live);
    test_give_energy(tg, 30);
    *member_out = member;
    *live_out = live;
    return 1;
}

/* The play-cost prompt is answered HERE, inside this helper, so it is counted
 * here too -- otherwise a test whose only prompt is the debut cost reports zero
 * and its CHECK_PROMPTS control is a false negative. */
static int play_chika(TestGame *tg, int member)
{
    int r = test_play_to_stage(tg, member, AREA_CENTER);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        note_prompt(tg);
        test_answer_play_cost_choice(tg, 0);
        if (tg->state.p[0].stage[AREA_CENTER] == member) break;
        r = test_play_to_stage(tg, member, AREA_CENTER);
    }
    return r;
}

/* Rust first_attacker_window_facts_survive_second_active_phase_and_live_start */
static void test_f1_first_attacker_facts_survive(void)
{
    TestGame *tg = &c_tg;
    int member = -1, live = -1;
    if (!turn_scope_setup(tg, &member, &live)) return;

    /* FIRST ATTACKER'S WINDOW: debut a member from hand. */
    play_chika(tg, member);
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], member,
             "f1: the member is at centre after the debut");
    /* sanity: the debut records the appearance and bumps the counter */
    CHECK(appeared_has(tg, member), "f1 sanity: the debut records the appearance");
    CHECK_EQ(tg->state.debut_count_this_turn[0], 1,
             "f1 sanity: the debut counter incremented");

    /* Cross into the SECOND ATTACKER'S normal phase -- the historical
     * truncation point. */
    if (!to_live_set(tg)) return;

    CHECK(appeared_has(tg, member),
          "f1: the appearance must survive the second attacker's Active phase "
          "(\"this turn\" spans the whole round)");
    CHECK_EQ(tg->state.debut_count_this_turn[0], 1,
             "f1: the debut counter must survive the second attacker's Active phase");

    /* And still be readable when the live-start window opens. */
    test_set_live_card(tg, 0, live);
    if (!to_live_start(tg)) return;
    drain_skip(tg, 16);

    CHECK(appeared_has(tg, member),
          "f1: the appearance must still be visible at live start");
}

/* Rust round_scoped_trackers_clear_at_turn_rollover -- THE OTHER SIDE OF THE
 * CONTRACT. Without it, "never cleared" in f1 would also pass and leak state
 * across rounds. PAIRED POSITIVE for f1. */
static void test_f2_round_scoped_trackers_clear_at_rollover(void)
{
    TestGame *tg = &d_tg;
    int member = -1, live = -1;
    if (!turn_scope_setup(tg, &member, &live)) return;

    play_chika(tg, member);
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], member, "f2: the member is at centre");
    CHECK(tg->state.n_cards_appeared_this_turn > 0,
          "f2 sanity: something was recorded before the rollover");

    /* Drive through the live into the next round. Steps ONE phase at a time:
     * test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) runs yell through
     * victory and auto-answers every standing prompt with index 0. */
    if (!to_live_set(tg)) return;
    test_set_live_card(tg, 0, live);
    int reached = 0;
    for (int i = 0; i < 10; i++) {
        test_pass(tg);
        drain_skip(tg, 8);
        if (tg->state.turn >= 2 || tg->state.phase == RB_PHASE_DONE) { reached = 1; break; }
    }
    if (!reached) {
        EXPECTED_GAP(0,
                     "f2: the round never rolled over, so the clear-at-rollover "
                     "claim could not be measured. Gap, not a pass.");
        return;
    }

    printf("  [f2] turn=%d phase=%d\n", tg->state.turn, tg->state.phase);
    CHECK(tg->state.turn >= 2, "f2 sanity: the turn rolled over");
    CHECK_EQ(tg->state.n_cards_appeared_this_turn, 0,
             "f2: appearances clear at the turn rollover");
    CHECK_EQ(tg->state.debut_count_this_turn[0], 0,
             "f2: debut counters clear at the turn rollover");
    CHECK_EQ(tg->state.n_turn_state_changes, 0,
             "f2: activation history clears at the turn rollover");
}

/* ------------------------------------------------------------------ *
 * SECTION G
 *   effects/gain/blades/per_card/per_unit_discard_count_regression_test.rs
 *   A Draw phase must not discard cards, whatever a per_unit ability on the
 *   stage would multiply.
 * ------------------------------------------------------------------ */
static void test_g1_draw_phase_discards_nothing(void)
{
    TestGame *tg = &e_tg;
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int filler = mid(tg, FILLER);
    if (filler < 0) return;
    if (!pin_id(filler, FILLER)) return;
    int filler2 = mid(tg, FILLER);
    if (filler2 < 0) return;
    if (filler == filler2) {
        setup_bugs++; failures++; assertions++;
        fprintf(stderr, "SETUP BUG: the two stage members alias one template slot\n");
        return;
    }
    set_stage(tg, 0, AREA_LEFT, filler);
    set_stage(tg, 0, AREA_CENTER, filler2);
    tg->state.p[0].hand.n = 0;
    for (int i = 0; i < 30; i++) deck_append(tg, 0, filler);

    int discard_before = tg->state.p[0].discard.n;
    int deck_before = tg->state.p[0].deck.n;
    int hand_before = tg->state.p[0].hand.n;

    /* TurnEngine::advance_phase -- the C twin is one phase step. */
    if (!test_advance_to_phase(tg, RB_PHASE_DRAW)) {
        EXPECTED_GAP(0,
                     "g1: the phase walk never reached the Draw phase, so the "
                     "claim could not be measured. Gap, not a pass.");
        return;
    }

    CHECK_EQ(tg->state.p[0].discard.n, discard_before,
             "the Draw phase must not discard cards");
    /* The positive half: the Draw phase must actually have drawn. A "nothing
     * was discarded" pass on a board where nothing happened at all is exactly
     * the whole-column-no-op failure this file exists to catch. */
    CHECK(tg->state.p[0].hand.n > hand_before ||
          tg->state.p[0].deck.n != deck_before,
          "the Draw phase must have actually drawn a card, otherwise the "
          "'nothing was discarded' assertion above is vacuous");
}

/* ------------------------------------------------------------------ *
 * fork-per-test reporting
 * ------------------------------------------------------------------ */
#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [in-process fallback]\n", failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps, p0 = prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d gap(s), %d prompt(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0), (int)(prompts - p0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    /* Print the heart-colour index mapping at startup. test_get_heart_modifier
     * REMAPS a requested colour of 5 onto RB_HEART_ORANGE, so every heart index
     * used in this file is derived from rb_parse_heart_color; this block is the
     * evidence that the derivation is the one the engine actually uses. */
    printf("--- heart colour mapping (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 8; i++)
        printf("    idx %d -> %s\n", i, rb_heart_color_as_str(i));
    printf("    parse heart00=%d heart01=%d heart03=%d heart05=%d heart06=%d\n",
           rb_parse_heart_color("heart00"), rb_parse_heart_color("heart01"),
           rb_parse_heart_color("heart03"), rb_parse_heart_color("heart05"),
           rb_parse_heart_color("heart06"));

    printf("--- A score/per_card/tokimeki_runners_all_heart_colors (PL!N-bp5-026-L) ---\n");
    run("a1_tokimeki_missing_no_score",  test_a1_tokimeki_missing_hearts_no_score);
    run("a2_tokimeki_all6_plus1",        test_a2_tokimeki_all6_colors_score_plus1);
    run("a3_tokimeki_modifier_separate", test_a3_tokimeki_modifier_separate_from_base);
    run("a4_tokimeki_score3_recovers",   test_a4_tokimeki_score3_recovers_nijigasaki);
    run("a5_tokimeki_score2_no_recover", test_a5_tokimeki_score2_does_not_recover);

    printf("--- B score/per_card/shizuku_aggregate_score_comparison_draw (PL!N-bp4-003-R/-P) ---\n");
    run("b1_shizuku_higher_draws",       test_b1_shizuku_higher_score_draws);
    run("b2_shizuku_tied_no_draw",       test_b2_shizuku_tied_score_no_draw);
    run("b3_shizuku_p2_no_live_draws",   test_b3_shizuku_p2_no_live_card_p1_wins);
    run("b4_shizuku_p_rarity",           test_b4_shizuku_p_rarity_same_as_r);

    printf("--- C score/per_card/strawberry_trapper_live_success_compound (PL!S-pb1-021-L) ---\n");
    run("c1_strawberry_q36_timing",      test_c1_strawberry_q36_timing);
    run("c2_strawberry_both_plus2",      test_c2_strawberry_both_conditions_plus2);
    run("c3_strawberry_excess_blocks",   test_c3_strawberry_excess_heart_blocks);
    run("c4_strawberry_wrong_group",     test_c4_strawberry_wrong_group_blocks);
    run("c5_strawberry_opp_no_win",      test_c5_strawberry_opponent_did_not_win);

    printf("--- D score/per_card/excess_heart_score_shift_never_below_zero (PL!N-bp5-010-R/-P/-AR) ---\n");
    run("d1_mifune_q231_cancels",        test_d1_mifune_q231_surplus_cancels);
    run("d2_mifune_all_rarities",        test_d2_mifune_all_rarities_cancel);

    printf("--- E effects/ability_mod/live_total_threshold (PL!-PR-020-PR) ---\n");
    run("e1_honoka_live_total_8",        test_e1_honoka_live_total_eight_grants);
    run("e2_honoka_live_total_7",        test_e2_honoka_live_total_seven_does_not_grant);

    printf("--- F blades/per_card/turn_scope_regression (PL!S-bp2-001-R) ---\n");
    run("f1_facts_survive",              test_f1_first_attacker_facts_survive);
    run("f2_clear_at_rollover",          test_f2_round_scoped_trackers_clear_at_rollover);

    printf("--- G blades/per_card/per_unit_discard_count_regression ---\n");
    run("g1_draw_discards_nothing",      test_g1_draw_phase_discards_nothing);

    rb_unload();

    printf("\n==== parity_per_card3 ====\n");
    printf("tests run              : %d\n", n_ok + n_failed + n_crashed);
    printf("tests passed           : %d\n", n_ok);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps          : %d\n", gaps);
    printf("fixture setup bugs     : %d\n", setup_bugs);
    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_crashed);
        return 1;
    }
    if (n_failed) {
        fprintf(stderr,
                "PARITY PER CARD 3: %d failing test(s) - every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, a Rust fixture\n"
                "bug, or a SETUP BUG naming a print the engine cannot resolve.\n"
                "Not a bug in this file.\n", n_failed);
        return n_failed > 125 ? 125 : n_failed;
    }
    printf("ALL PARITY_PER_CARD3 CHECKS PASSED\n");
    return 0;
}
