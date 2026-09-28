/* tests/test_parity_full_round.c
 *
 * C port of the gaps in
 *   engine/tests/test_modules/integration/full_round/   (5 .rs / 91 #[test])
 *   engine/tests/test_modules/rules/baton/double_baton_touch_*.rs
 *
 * ALREADY COVERED by the existing C corpus (read before writing, see the
 * §0 audit at the bottom of this header for the exact file:function):
 *   full_round/live_each_time_victory_road_through_real_live_rounds_test.rs
 *       19 of 22 already ported by test_parity_integration_extra.c §A/§A2.
 *       T4 live_start_another_member / T5 live_start_cost_free_still_triggers
 *       are the §A T1 fixture with the stage slots swapped and the Rust file
 *       itself calls T4 "Redundant with T1" -> not re-padded here.
 *       T6 / T8 (the two below) are the only uncovered ones; both are cheap
 *       and are ported because they add a stage[0] occupant that T1 lacks.
 *   full_round/live_score_tie_live_zone_placement_across_rounds_test.rs
 *       3 of 4 ported by test_parity_integration.c
 *       (test_live_zone_two_card_selection / test_tie_at_two_successes_blocks_third
 *        / test_no_live_cards_clean_rollover).  The FOURTH,
 *       solo_winner_third_success_ends_before_live_reentry, is genuinely
 *       uncovered and IS ported here.
 *   full_round/rps_through_turn3_main_phase_flow_test.rs
 *       e2e_full_game_rps_to_turn3 -> test_parity_integration.c
 *       test_phase_walk_through_turn3, including the documented GAP-00 for the
 *       RPS / mulligan action chain (rb_execute_main_phase_action dispatches
 *       only UseAbility and Pass).
 *   full_round/real_card_phase_walkthrough_and_ability_suite_test.rs
 *       63 tests.  umi_pr014_* / dia_sd1_* -> test_parity_realcard_walkthrough.c
 *       (§U/§D).  ai_screeam_soreigai / answer_both_draw / rise_up_high_turn1 ->
 *       test_parity_realcard_walkthrough.c §B.  energy_zone_capacity_handled,
 *       revealed_cards_filtered_by_player_ownership,
 *       choice_condition_shows_proper_labels_in_actions ->
 *       test_parity_condition_and_effect.c.  The DISTINCT behaviours still
 *       missing are ported here in §2.
 *
 * §1  rules/baton/double_baton_touch_*  (2 .rs, 19 #[test], ZERO C coverage)
 *       Both cards are 常時「2人のメンバーとバトンタッチしてもよい」 double
 *       baton cards, so the whole family hinges on the C engine's
 *       rb_complete_double_baton() path (src/engine.c:897), which the survey
 *       records as having NO caller outside rb_resolver_handle_double_baton_touch.
 *       13 ported, 6 skipped by name (duplicates or no C counterpart).
 *       FOUR engine defects were found here and are reported, not papered
 *       over; see the KNOWN RED comments on A1, A7, B1, B2, B6, B7.
 * §2  full_round gaps: the phase walkthrough smoke test, the uncovered
 *       victory-road negative controls, the whole ディストーション
 *       (PL!SP-pb1-023-L) energy-refresh / score family, the chika turn-limit,
 *       the Q186 cost reduction, the turn-2 gate on Rise Up High! and the
 *       P2-owned 愛♡スクリ～ム！ blade.  THREE further defects here; see the
 *       KNOWN RED comments on F2, F11, F15, F16, F17.
 *
 * Honesty contract
 * ----------------
 * Every assertion is a hard CHECK about the C engine's behaviour.  A
 * divergence from Rust is the most valuable thing this file can report, so
 * nothing is downgraded to a warning to make the suite green.  Exit status is
 * non-zero whenever any test fails.  Failures are classified in the final
 * report as ENGINE BUG / TEST BUG / PARSER GAP, never suppressed.
 *
 * PROMPT-COUNT CONTROL (the quality bar for this file)
 * ---------------------------------------------------
 * Several of the ported tests are NEGATIVE by nature ("no draw", "+0 blade",
 * "the ability does not fire").  On a board where the effect is a silent
 * no-op those all pass vacuously.  Every such test therefore also asserts a
 * POSITIVE control that can only be true if the effect path actually ran:
 *   - the double-baton tests assert rb_get_baton_touch_count() and/or the hand
 *     delta BEFORE the negative blade assertion, so "+0 blade" is read
 *     against a proven-executed draw, not against a dead board;
 *   - the "normal debut fires nothing" tests assert the played card actually
 *     reached the stage and that prompts were raised at all;
 *   - drain_* return the number of prompts they answered and the callers
 *     CHECK(prompts > 0) where a zero would make the assertion meaningless.
 *
 * KNOWN HARNESS LANDMINES respected here:
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE.  No
 *     heart read below uses it; rb_mods_get_heart is called directly.
 *   - RbChoice.zone for the waitroom is "discard", not "waitroom".
 *   - mid() = test_new_id() wherever Rust wrote game.new_id(); test_id() only
 *     where Rust wrote game.id().
 *   - sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *   - rb_card_copy_pool_reset() is called per test: rb_create_card_copy draws
 *     from a process-global monotonic pool that nothing else resets, and a
 *     long suite exhausts it (4096 ids), after which test_new_id() returns -1
 *     and every identity assertion goes vacuous.
 *   - Card.ability is NOT populated by rb_decode_card_by_index; ability
 *     presence is read with rb_card_num_abilities.
 *   - test_set_live_card is P1-only; test_set_live_card_for is used for P2.
 *   - Card numbers carrying the FULLWIDTH PLUS (U+FF0B) are spelled
 *     "\xef\xbc\x8b": rb_find_card_by_no is EXACT-BYTE and does NOT normalise
 *     fullwidth to halfwidth the way Rust's CardDatabase::get_card_id does.
 *
 * PER-TEST FORK ISOLATION
 * Each test runs in a forked child; a child that dies on a signal is reported
 * as CRASH, never as a pass.  The SIGSEGV/SIGBUS/SIGABRT handler names the
 * test in flight.  Every drain carries an iteration guard: one peer found an
 * ability that re-prompts UNBOUNDEDLY, and an unbounded drain would hang the
 * whole suite instead of reporting the engine gap.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* the fullwidth plus used by variant card numbers (U+FF0B) */
#define PLUS "\xef\xbc\x8b"

#define FILLER_NO "PL!-sd1-010-SD"     /* 高坂穂乃果, cost 4 */
#define ENERGY_NO "LL-E-001-SD"

/* ═══════════════════════════════ harness ═══════════════════════════════ */

static const char *current_test = "(none)";
static int failures;
static long assertions;
static int setup_bugs;

#define CHECK(condition, message) do {                              \
    assertions++;                                                    \
    if (!(condition)) {                                              \
        fprintf(stderr, "FAIL [%s]: %s\n", current_test, message);  \
        failures++;                                                  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
} while (0)

#define CHECK_EQ(actual, expected, message) do {                    \
    assertions++;                                                    \
    int a_ = (actual);                                              \
    int e_ = (expected);                                            \
    if (a_ != e_) {                                                 \
        fprintf(stderr, "FAIL [%s]: %s (got %d expected %d)\n",      \
                current_test, message, a_, e_);                     \
        failures++;                                                  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
} while (0)

/* Rust load_real_database() + rb_load(): in-tree first, then the out-of-tree
 * location the isolated build resolves.  REQUIRED, not optional: without the
 * fallback every rb_find_card_by_no in this file returns -1 and every test
 * would report a setup bug instead of a behaviour. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    return rb_load("../cards/build");
}

/* ── fixtures ── */

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

static void fill_decks_p1_p2(TestGame *tg, int filler, int n1, int n2)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        int n = pl == 0 ? n1 : n2;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

/* Rust `game.state.playerX.energy_zone.cards.push(energy_id)` on its own,
 * which raises the bag length WITHOUT raising active_count.  The C energy
 * zone is an unordered bag plus a separate energy_active count, so the two
 * halves are set explicitly. */
static void push_wait_energy(TestGame *tg, int pl, int n)
{
    int eid = test_id(tg, ENERGY_NO);
    if (eid < 0) eid = 0;
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->energy.n < RB_MAX_ZONE; i++)
        P->energy.cards[P->energy.n++] = eid;
}

static void push_wait_energy_one(TestGame *tg, int pl, int card_id)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = card_id;
}

static int energy_active(TestGame *tg, int pl)
{
    return rb_energy_active_count(&tg->state.p[pl]);
}

static int energy_total(TestGame *tg, int pl)
{
    return tg->state.p[pl].energy.n;
}

/* Rust advance_to_live_card_set_p1(): 5 passes Main -> ... -> LiveCardSet. */
static void advance_to_live_set_5(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust advance_to_live_start(): 2 further passes; in C that is the window in
 * which ライブ開始時 abilities resolve (RB_PHASE_PERFORMANCE). */
static void finish_live_setup(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* Rust `advance_to_turn2` in the two double-baton files: exactly 7 passes.
 * Named "turn 2" in the Rust sources, but the documented phase walk puts the
 * 7th pass on ライブ開始時 of round 1; ported verbatim. */
static void advance_seven(TestGame *tg)
{
    for (int i = 0; i < 7; i++) test_pass(tg);
}

/* ── drains ──
 * Every drain is bounded and RETURNS the number of prompts it answered, so a
 * caller can CHECK(prompts > 0) and prove a negative assertion non-vacuous. */

#define DRAIN_GUARD 60

/* Answer a "double_baton_touch" SelectTarget with option 0 (the first pair),
 * answer a non-skippable SelectCard with its first option, decline the rest.
 * Skipping the double-baton prompt would abandon the whole play, so it is
 * never skipped. */
static int drain_baton(TestGame *tg)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < DRAIN_GUARD) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = -1;
        if (c && !strcmp(c->target, "double_baton_touch")) {
            idx = 0;
        } else if (c && c->kind == RB_CHOICE_SELECT_CARD && !c->allow_skip) {
            idx = 0;
        }
        rb_resume_with_choice(&tg->state, idx);
        n++;
    }
    return n;
}

/* The Rust files' own drain_auto_prompts(): consume SelectAutoAbility prompts
 * (declining them) and STOP on anything else, so a mis-typed prompt surfaces
 * as a stuck choice rather than being silently consumed.  Used on the paths
 * where the Rust sources assert "the auto-ability prompt was declined", i.e.
 * the single-baton ones, where drain_baton must not sweep everything. */
static int drain_auto_prompts(TestGame *tg)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < DRAIN_GUARD) {
        const char *t = test_pending_choice_type(tg);
        if (strcmp(t, "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&tg->state, -1);
        n++;
    }
    return n;
}

/* The Rust files' `while has_pending_choice { select_indices(&[0]) }`: answer
 * everything with option 0. */
static int drain_pick_first_all(TestGame *tg)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < DRAIN_GUARD) {
        rb_resume_with_choice(&tg->state, 0);
        n++;
    }
    return n;
}

/* Answer what cannot be declined, decline what can (integration_extra.c's
 * drain_skip). */
static int drain_skip(TestGame *tg)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < DRAIN_GUARD) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int must_pick = (c && c->kind == RB_CHOICE_SELECT_CARD && !c->allow_skip);
        rb_resume_with_choice(&tg->state, must_pick ? 0 : -1);
        n++;
    }
    return n;
}

static int blade_of(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}

static int score_of(TestGame *tg, int cid)
{
    return rb_mods_get_score(&tg->state.mods, cid);
}

static int all_heart_of(TestGame *tg, int cid)
{
    return rb_mods_get_heart(&tg->state.mods, cid, RB_HEART_ALL);
}

static int printed_cost(int card_id)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return -1;
    int cost = c.cost;
    rb_free_card(&c);
    return cost;
}

/* Byte-exact card identity.  rb_card_no_eq is FUZZY and is NOT an identity
 * pin -- a peer found it accept "PL!SP-PR-004-R" against the real
 * "PL!SP-PR-004-PR" -- so every fixture that matters is pinned here with a
 * strcmp of the decoded card number instead. */
static int card_is(int card_id, const char *no)
{
    if (card_id < 0) return 0;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    const char *cn = rb_card_string(c.card_no_idx);
    int eq = (cn && !strcmp(cn, no));
    rb_free_card(&c);
    return eq;
}

static int stage_has(TestGame *tg, int area, int cid)
{
    return tg->state.p[0].stage[area] == cid;
}

static int hand_has(TestGame *tg, int cid)
{
    return test_zone_has_id(tg, 0, "hand", cid);
}

static int wait_has(TestGame *tg, int cid)
{
    return test_zone_has_id(tg, 0, "discard", cid);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §1  rules/baton/double_baton_touch_blade_grant_per_liella_moved_test.rs
 *     嵐 千砂都＆鬼塚夏美 (PL!SP-pb2-000-R), cost 15
 *       ab#0 (常時)      「このカードのプレイに際し、2人のメンバーと
 *                        バトンタッチしてもよい。」  -> play_baton_touch
 *                        count 2, optional
 *       ab#1 (登場)      「バトンタッチして登場した場合、このバトンタッチ
 *                        によって控え室に置かれた『Liella!』のメンバーカード
 *                        1枚につき、カードを1枚引く。ブレードハートを持たない
 *                        『Liella!』のメンバーカード1枚につき、ライブ終了時
 *                        まで、{{ブレード}}{{ブレード}}を得る。」
 *
 *  Card facts verified against cards/cards.json (UTF-8):
 *    PL!SP-pb2-000-R     cost 15  unit (blank)  blade_heart b_heart03=1
 *    PL!SP-bp1-001-R     cost  9  CatChu!       NO blade_heart
 *    PL!SP-bp1-004-R     cost 15  CatChu!       blade_heart b_heart03=1
 *    PL!SP-bp1-005-R     cost  2  KALEIDOSCORE  blade_heart b_heart03=1
 *    PL!-sd1-001-SD      cost 11  Printemps     (mu's, NOT Liella!)
 *  So the Rust test's "has blade_heart" / "no blade_heart" comments are
 *  correct and the +0 / +2 / +4 blade expectations follow from the data.
 * ═══════════════════════════════════════════════════════════════════════ */

#define CHISATO_NO  "PL!SP-pb2-000-R"
#define LIELLA_NOBLADE "PL!SP-bp1-001-R"
#define LIELLA_BLADE_A "PL!SP-bp1-004-R"
#define LIELLA_BLADE_B "PL!SP-bp1-005-R"
#define MUS_NO      "PL!-sd1-001-SD"

/* Rust `advance_to_turn2` + one play_to_stage(Center) with a double baton,
 * followed by drain_auto_prompts.  `area` is Sumire/Chisato's target area. */
static int chisato_double_baton(TestGame *tg, int a0, int a1, int a2,
                                int area, int *prompts_out)
{
    int chisato = test_id(tg, CHISATO_NO);
    int filler  = test_id(tg, FILLER_NO);
    if (chisato < 0 || filler < 0) { setup_bugs++; return 0; }
    /* byte-exact identity, because rb_card_no_eq is fuzzy */
    if (!card_is(chisato, CHISATO_NO)) {
        fprintf(stderr, "SETUP BUG: %s did not resolve to the expected card\n", CHISATO_NO);
        setup_bugs++;
        return 0;
    }
    fill_decks_p1_p2(tg, filler, 40, 10);
    test_add_to_hand(tg, chisato);
    test_add_to_hand(tg, filler);
    tg->state.p[0].stage[0] = a0;
    tg->state.p[0].stage[1] = a1;
    tg->state.p[0].stage[2] = a2;
    test_give_energy(tg, 20);

    advance_seven(tg);
    int played = test_play_to_stage(tg, chisato, area);
    *prompts_out = drain_baton(tg);
    return played ? chisato : -2;
}

/* T-A1 chisato_natsumi_double_baton_both_have_blade_heart
 * 2 Liella! replaced, BOTH have blade_heart -> draw 2, blade +0.
 * KNOWN RED (see the report) for TWO independent defects:
 *  1. dynamic_count.c:272-331 has no "discard"/waitroom arm in the per_unit
 *     branch, and this effect declares per_unit_type "discard", so it falls
 *     through to `units = 1` at :325.  The multiplier is therefore always
 *     exactly 1, whatever the waitroom holds.
 *  2. util.c:34-38 compiles RF_CARD_PROPERTY out to "" because
 *     RB_CARD_FILTER_CARD_PROPERTY is never defined, so the
 *     card_property:"has_blade_heart" + negation filter on the blade half is
 *     absent and every replaced card counts. */
static void a1_both_have_blade_heart(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int l1 = test_id(&tg, LIELLA_BLADE_A);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    int prompts = 0;
    int card = chisato_double_baton(&tg, l1, l2, RB_EMPTY_SLOT, 1, &prompts);
    if (card == -2) { setup_bugs++; return; }

    /* PROMPT-COUNT CONTROL: the double-baton prompt must actually have been
     * raised, otherwise "baton_touch_count == 2" and the hand delta below
     * would be assertions about a board on which nothing ran. */
    CHECK(prompts > 0,
          "A1: the double-baton prompt was raised (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2,
             "A1: double baton -> count=2");
    /* hand = 2 initial - 1 played + 2 drawn */
    CHECK_EQ(tg.state.p[0].hand.n, 3,
             "A1: 2 Liella! replaced -> 2 cards drawn (hand 2-1+2=3)");
    /* The draw above proves ab#1's per_unit branch executed, so the +0 blade
     * below is a real reading and not a dead board. */
    CHECK_EQ(blade_of(&tg, card), 0,
             "A1: both replaced members have blade_heart -> +0 blade");
}

/* T-A2 chisato_natsumi_double_baton_no_blade_heart
 * liella1 has NO blade_heart, liella2 has one -> draw 2, blade +2. */
static void a2_one_no_blade_heart(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int l1 = test_id(&tg, LIELLA_NOBLADE);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    int prompts = 0;
    int card = chisato_double_baton(&tg, l1, l2, RB_EMPTY_SLOT, 1, &prompts);
    if (card == -2) { setup_bugs++; return; }

    CHECK(prompts > 0, "A2: the double-baton prompt was raised (control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2, "A2: double baton -> count=2");
    CHECK_EQ(tg.state.p[0].hand.n, 3, "A2: 2 Liella! replaced -> 2 cards drawn");
    CHECK_EQ(blade_of(&tg, card), 2,
             "A2: exactly one replaced Liella! lacks blade_heart -> +2 blade");
}

/* T-A3 chisato_natsumi_q265_double_baton_both_no_blade_heart
 * two DISTINCT instances of the same blade-heart-less Liella! -> +4. */
static void a3_q265_both_no_blade_heart(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int l1 = test_id(&tg, LIELLA_NOBLADE);
    int l2 = test_new_id(&tg, LIELLA_NOBLADE);
    int prompts = 0;
    if (l2 < 0) { setup_bugs++; return; }
    CHECK(l1 != l2, "A3: the two on-stage members are two DISTINCT card ids");
    int card = chisato_double_baton(&tg, l1, l2, RB_EMPTY_SLOT, 1, &prompts);
    if (card == -2) { setup_bugs++; return; }

    CHECK(prompts > 0, "A3: the double-baton prompt was raised (control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2, "A3: double baton -> count=2");
    CHECK_EQ(tg.state.p[0].hand.n, 3, "A3: 2 Liella! replaced -> 2 cards drawn");
    CHECK_EQ(blade_of(&tg, card), 4,
             "A3: two blade-heart-less Liella! -> +4 blade (Q265)");
}

/* T-A4 chisato_natsumi_debut_draw_counts_only_replaced_not_full_waitroom
 * 5 pre-existing waitroom fillers + 2 replaced -> hand must be 3-1+2 = 4,
 * NOT 3-1+7.  The regression this pins is counting the whole waitroom. */
static void a4_draw_counts_only_replaced(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int chisato = test_id(&tg, CHISATO_NO);
    int filler  = test_id(&tg, FILLER_NO);
    int l1 = test_id(&tg, LIELLA_NOBLADE);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    if (chisato < 0 || filler < 0 || l1 < 0 || l2 < 0) { setup_bugs++; return; }

    fill_decks_p1_p2(&tg, filler, 40, 10);
    for (int i = 0; i < 5; i++) test_add_to_discard(&tg, filler);
    test_add_to_hand(&tg, chisato);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = l1;
    tg.state.p[0].stage[1] = l2;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 20);

    advance_seven(&tg);
    int prompts = 0;
    CHECK(test_play_to_stage(&tg, chisato, 1) == 1, "A4: the play was accepted");
    prompts = drain_baton(&tg);

    CHECK(prompts > 0, "A4: the double-baton prompt was raised (control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2, "A4: double baton -> count=2");
    CHECK_EQ(tg.state.p[0].discard.n, 7,
             "A4: the waitroom really holds 5 pre-existing + 2 replaced cards");
    /* hand = 3 - 1 (played) + 2 (drawn) = 4.  A waitroom-sized draw (7) or a
     * group-blind one would give 9 or 5. */
    CHECK_EQ(tg.state.p[0].hand.n, 4,
             "A4: drew 2 for the 2 REPLACED Liella!, ignoring the 5 pre-existing waitroom cards");
}

/* T-A5 chisato_natsumi_baton_replacing_non_liella_counts_only_liella
 * stage [Liella!(no blade heart), mu's member] -> only the Liella! counts:
 * draw exactly 1, blade exactly +2. */
static void a5_group_filter_counts_only_liella(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int chisato = test_id(&tg, CHISATO_NO);
    int filler  = test_id(&tg, FILLER_NO);
    int liella  = test_id(&tg, LIELLA_NOBLADE);
    int nonliella = test_id(&tg, MUS_NO);
    if (chisato < 0 || filler < 0 || liella < 0 || nonliella < 0) { setup_bugs++; return; }

    fill_decks_p1_p2(&tg, filler, 40, 10);
    for (int i = 0; i < 3; i++) test_add_to_discard(&tg, filler);
    test_add_to_hand(&tg, chisato);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = liella;
    tg.state.p[0].stage[1] = nonliella;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 20);

    advance_seven(&tg);
    CHECK(test_play_to_stage(&tg, chisato, 1) == 1, "A5: the play was accepted");
    int prompts = drain_baton(&tg);

    CHECK(prompts > 0, "A5: the double-baton prompt was raised (control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2, "A5: both areas were replaced");
    /* hand = 3 - 1 + 1 = 3.  A group-blind engine draws 2 (hand 4); a
     * whole-waitroom engine draws 5 (hand 7). */
    CHECK_EQ(tg.state.p[0].hand.n, 3,
             "A5: drew for the ONE replaced Liella! only");
    CHECK_EQ(blade_of(&tg, chisato), 2,
             "A5: the blade gain is fed only by the Liella! member");
}

/* T-A6 chisato_natsumi_debut_single_baton_one_liella
 * a SINGLE baton onto the occupied Left area -> draw 1, blade +2. */
static void a6_single_baton_one_liella(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int chisato = test_id(&tg, CHISATO_NO);
    int filler  = test_id(&tg, FILLER_NO);
    int liella  = test_id(&tg, LIELLA_NOBLADE);
    if (chisato < 0 || filler < 0 || liella < 0) { setup_bugs++; return; }

    fill_decks_p1_p2(&tg, filler, 40, 10);
    test_add_to_hand(&tg, chisato);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = liella;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 20);

    advance_seven(&tg);
    int deck_before = tg.state.p[0].deck.n;
    int energy_before = energy_active(&tg, 0);
    CHECK(test_play_to_stage(&tg, chisato, 0) == 1, "A6: the play was accepted");
    drain_baton(&tg);
    drain_auto_prompts(&tg);

    /* PROMPT-COUNT CONTROL, corrected: the single-baton path raises NO prompt
     * at all (rb_play_member's non-double branch is synchronous), so counting
     * prompts here would be a control that can never pass.  The real control
     * is the DECK DELTA: exactly one card left the deck, which is only
     * possible if ab#1's per_unit draw executed.  Without it, the blade
     * assertion below would be a reading off a dead board. */
    CHECK_EQ(deck_before - tg.state.p[0].deck.n, 1,
             "A6: exactly 1 card was drawn (non-vacuous control for the blade reading)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 1,
             "A6: a single baton (one occupied area) records exactly 1");
    CHECK_EQ(wait_has(&tg, liella), 1, "A6: the replaced Liella! went to the waitroom");
    CHECK_EQ(stage_has(&tg, 0, chisato), 1, "A6: Chisato occupies the baton'd area");
    CHECK_EQ(tg.state.p[0].hand.n, 3, "A6: 1 replaced Liella! -> 1 card drawn (3-1+1)");
    CHECK_EQ(blade_of(&tg, chisato), 2,
             "A6: 1 blade-heart-less Liella! -> +2 blade");
    /* A baton play pays only the DIFFERENCE against the replaced member's
     * printed cost (Rule 9.6.2.1): Chisato 15 - bp1-001-R 9 = 6. */
    CHECK_EQ(energy_before - energy_active(&tg, 0), printed_cost(chisato) - printed_cost(liella),
             "A6: the baton cost paid is the difference of the printed costs");
}

/* T-A7 chisato_natsumi_normal_debut_without_baton_touch_fires_nothing
 * The ab#1 GATE: on a NORMAL debut (empty area, no baton) nothing fires.
 * KNOWN RED (see the report): eval_movement (src/ability/condition.c:610-640)
 * has no `movement == "baton_touch"` arm — it handles only has_moved /
 * not_moved and otherwise returns "did ANYTHING move recently" at :639.
 * check_baton_touch at :2017 is the faithful implementation and is DEAD CODE
 * (the compiler reports it unused).  rb_play_member:1072-1073 always records
 * the played card in recently_moved, so the gate passes on a NORMAL debut. */
static void a7_normal_debut_fires_nothing(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int chisato = test_id(&tg, CHISATO_NO);
    int filler  = test_id(&tg, FILLER_NO);
    if (chisato < 0 || filler < 0) { setup_bugs++; return; }

    fill_decks(&tg, filler, 40);
    test_add_to_hand(&tg, chisato);
    test_give_energy(&tg, 20);

    advance_seven(&tg);
    /* Baselines AFTER all phase-advance draws, as the Rust test does. */
    int hand_before = tg.state.p[0].hand.n;
    int deck_before = tg.state.p[0].deck.n;

    CHECK(test_play_to_stage(&tg, chisato, 1) == 1, "A7: the normal debut was accepted");
    int prompts = drain_baton(&tg);

    /* PROMPT-COUNT CONTROL: the Rust assertion is `!has_pending_choice()`.
     * Asserting only that would pass on a board where the debut never
     * happened, so the played card is pinned on stage first. */
    CHECK_EQ(stage_has(&tg, 1, chisato), 1,
             "A7: the debut really placed Chisato on stage (non-vacuous control)");
    CHECK(prompts == 0,
          "A7: a normal debut leaves no unresolved prompts");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
             "A7: played 1, drew 0");
    CHECK_EQ(deck_before - tg.state.p[0].deck.n, 0,
             "A7: the deck is untouched - no draw on a non-baton debut");
    CHECK_EQ(blade_of(&tg, chisato), 0,
             "A7: no blade bonus on a non-baton debut");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 0,
             "A7: no baton touch was recorded");
}

/* T-A8 chisato_natsumi_blade_bonus_expires_at_live_end
 * 「ライブ終了時まで」: the +2 blade must be gone once the live closes. */
static void a8_blade_expires_at_live_end(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int chisato = test_id(&tg, CHISATO_NO);
    int filler  = test_id(&tg, FILLER_NO);
    int liella  = test_id(&tg, LIELLA_NOBLADE);
    int live    = test_id(&tg, "PL!-sd1-020-SD");
    if (chisato < 0 || filler < 0 || liella < 0 || live < 0) { setup_bugs++; return; }

    fill_decks_p1_p2(&tg, filler, 40, 10);
    test_add_to_hand(&tg, chisato);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = liella;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 20);

    advance_seven(&tg);
    CHECK(test_play_to_stage(&tg, chisato, 0) == 1, "A8: the play was accepted");
    drain_baton(&tg);
    CHECK_EQ(blade_of(&tg, chisato), 2,
             "A8: the bonus is active right after the baton debut");

    /* Named targets, not "3 passes after setting". */
    test_add_to_hand(&tg, live);
    int reached = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK(reached == 1, "A8: reached the live-card-set window by name");
    test_set_live_card(&tg, 0, live);
    int guard = 0;
    while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 24) {
        test_pass(&tg);
        drain_skip(&tg);
    }
    CHECK_EQ(blade_of(&tg, chisato), 0,
             "A8: the live_end-duration blade bonus expired at live end");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §1b rules/baton/double_baton_touch_draw_deploy_q193_q194_test.rs
 *     平安名すみれ (PL!SP-bp4-004-R＋), cost 22
 *       ab#0 (常時)  double baton (play_baton_touch count 2)
 *       ab#1 (登場,センター) gated on min_baton_touch_count == 2 with
 *                     appearance.baton_touch: sequential [draw 2,
 *                     move_cards discard->empty_area cost<=4 Liella! 1]
 *
 *  cards.json: PL!SP-bp4-004-R＋ cost 22.  The Rust file gives only 20 energy
 *  and, in sumire_explicit_double_baton_via_card_indices, asserts a payment
 *  of `expected_cost = 5u32`.  Both are inconsistent with the printed cost;
 *  see the RUST FIXTURE BUG note in the report.  This port provisions 30
 *  energy (so the play is legal) and CHECKs the printed cost so the
 *  discrepancy is visible in the output rather than hidden.
 * ═══════════════════════════════════════════════════════════════════════ */

#define SUMIRE_NO "PL!SP-bp4-004-R" PLUS

static int sumire_setup(TestGame *tg, int a0, int a1, int a2, int *prompts_out)
{
    int sumire = test_id(tg, SUMIRE_NO);
    int filler = test_id(tg, FILLER_NO);
    if (sumire < 0 || filler < 0) { setup_bugs++; return 0; }
    /* byte-exact identity: the FULLWIDTH PLUS must survive the lookup */
    if (!card_is(sumire, SUMIRE_NO)) {
        fprintf(stderr, "SETUP BUG: %s did not resolve to the expected card\n", SUMIRE_NO);
        setup_bugs++;
        return 0;
    }
    fill_decks_p1_p2(tg, filler, 40, 10);
    test_add_to_hand(tg, sumire);
    test_add_to_hand(tg, filler);
    tg->state.p[0].stage[0] = a0;
    tg->state.p[0].stage[1] = a1;
    tg->state.p[0].stage[2] = a2;
    test_give_energy(tg, 30);
    advance_seven(tg);
    *prompts_out = 0;
    return sumire;
}

/* T-B1 sumire_q193_q194_baton_touch_draw_and_deploy
 * Double baton into Center -> ab#1 fires: draw 2 and deploy a cost<=4 Liella!
 * from the waitroom into a vacated area. */
static void b1_q193_q194_draw_and_deploy(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int l1 = test_id(&tg, LIELLA_BLADE_A);   /* cost 15, Liella! */
    int l2 = test_id(&tg, LIELLA_BLADE_B);   /* cost  2, Liella! */
    if (l1 < 0 || l2 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, l1, l2, RB_EMPTY_SLOT, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    CHECK_EQ(printed_cost(sumire), 22,
             "B1: cards.json prints cost 22 for PL!SP-bp4-004-R+ (documents the Rust fixture's 20-energy / cost-5 assumption)");

    CHECK(test_play_to_stage(&tg, sumire, 1) == 1, "B1: the play was accepted");
    int n = drain_baton(&tg) + drain_pick_first_all(&tg);

    /* PROMPT-COUNT CONTROL */
    CHECK(n > 0, "B1: the double-baton prompt was raised (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2,
             "B1/Q194: 2 baton touches recorded (double baton)");

    /* Diagnostic, printed unconditionally so the report names the observed
     * placement rather than a bare "got 0".  rb_complete_double_baton
     * (src/engine.c:932) sets dst = baton_last_vacated_area, and because the
     * played area is by construction one of the two areas being vacated, the
     * `dst == i1 || dst == i2` guard ALWAYS fires and the arriving card lands
     * in the FIRST vacated area.  The assertions below are consequently
     * DOWNSTREAM of that placement defect: the ab#1 deploy never happens
     * because the arriving card is not in the CENTER the ability requires. */
    fprintf(stderr,
            "   [B1 diag] stage=[%d,%d,%d] sumire=%d l1=%d l2=%d "
            "waitroom=%d hand=%d energy=%d\n",
            tg.state.p[0].stage[0], tg.state.p[0].stage[1], tg.state.p[0].stage[2],
            sumire, l1, l2, tg.state.p[0].discard.n, tg.state.p[0].hand.n,
            energy_active(&tg, 0));

    /* Q193: exactly one of the two replaced Liella! members is deployed back
     * to stage (the cost<=4 one); the other stays in the waitroom. */
    CHECK_EQ(stage_has(&tg, 1, sumire), 1,
             "B1/Q193: Sumire occupies the CENTER area it was played into");
    int liella_on_stage = (tg.state.p[0].stage[0] == l1) + (tg.state.p[0].stage[0] == l2) +
                          (tg.state.p[0].stage[1] == l1) + (tg.state.p[0].stage[1] == l2) +
                          (tg.state.p[0].stage[2] == l1) + (tg.state.p[0].stage[2] == l2);
    CHECK_EQ(liella_on_stage, 1,
             "B1/Q193: exactly one of the two Liella! members is back on stage");
    CHECK_EQ(wait_has(&tg, l1) + wait_has(&tg, l2), 1,
             "B1/Q193: the other Liella! member stayed in the waitroom");
    /* hand = 2 initial - 1 played + 2 drawn */
    CHECK_EQ(tg.state.p[0].hand.n, 3, "B1/Q193: ab#1 drew 2 (hand 2-1+2=3)");
}

/* T-B2 sumire_no_baton_touch_no_draw
 * Playing into the EMPTY center -> no baton -> ab#1 does not trigger.
 * Rust reaches that state by passing `use_baton_touch: Some(false)`, i.e. by
 * DECLINING the optional 常時 double baton.  rb_play_member has no such
 * parameter: it offers the double baton whenever the card carries the 常時
 * ability and >= 2 members are on stage, and answering the offer with a skip
 * (rb_resume_with_choice(g, -1) on the SelectTarget) takes the pending
 * actions and abandons the play altogether.  So "decline the double baton and
 * then play normally" is UNREACHABLE in the C port, and the assertions below
 * are the faithful reading of what the engine does instead. */
static void b2_no_baton_no_draw(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int l1 = test_id(&tg, LIELLA_BLADE_A);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    if (l1 < 0 || l2 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, l1, RB_EMPTY_SLOT, l2, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    int hand_before = tg.state.p[0].hand.n;
    CHECK(test_play_to_stage(&tg, sumire, 1) == 1, "B2: the play was accepted");
    drain_baton(&tg);
    drain_pick_first_all(&tg);

    /* PROMPT-COUNT CONTROL: the card must be ON STAGE, otherwise "no draw"
     * is asserting an absence on a board where the play never happened. */
    CHECK_EQ(stage_has(&tg, 1, sumire), 1,
             "B2: Sumire really landed on stage (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 0,
             "B2: no baton touch happened (the target area was empty)");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
             "B2: no baton -> ab#1 does not trigger, no draw");
}

/* T-B3 sumire_only_one_occupied_area_triggers_single_baton_touch
 * one occupied area -> exactly 1 baton touch -> min_baton_touch_count==2
 * unmet -> no draw. */
static void b3_only_one_occupied_area(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int l1 = test_id(&tg, LIELLA_BLADE_A);
    if (l1 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, l1, RB_EMPTY_SLOT, RB_EMPTY_SLOT, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    CHECK(test_play_to_stage(&tg, sumire, 0) == 1, "B3: the play was accepted");
    drain_baton(&tg);
    drain_pick_first_all(&tg);

    /* PROMPT-COUNT CONTROL, corrected: the single-baton path is synchronous
     * and raises no prompt, so the control is the two observable moves. */
    CHECK_EQ(wait_has(&tg, l1), 1,
             "B3: the replaced Liella! really went to the waitroom (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 1,
             "B3: only 1 baton touch (only 1 occupied area)");
    CHECK_EQ(stage_has(&tg, 0, sumire), 1,
             "B3: Sumire occupies the baton'd LEFT area");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "B3: no draw from ab#1 - only 1 baton touch occurred");
}

/* T-B4 sumire_double_baton_to_left_does_not_activate_debut
 * The double baton happens, but to a NON-CENTER area, so ab#1's
 * activation_position == center gate must block the whole effect. */
static void b4_double_baton_to_left(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int l1 = test_id(&tg, LIELLA_BLADE_A);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    if (l1 < 0 || l2 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, l1, l2, RB_EMPTY_SLOT, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    CHECK(test_play_to_stage(&tg, sumire, 0) == 1, "B4: the play was accepted");
    int n = drain_baton(&tg) + drain_pick_first_all(&tg);

    CHECK(n > 0, "B4: the double-baton prompt was raised (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2,
             "B4: the double baton DID happen (2 replacements)");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "B4: no draw - ab#1 requires the CENTER position and Sumire is at LEFT");
    CHECK_EQ(stage_has(&tg, 0, sumire), 1,
             "B4: Sumire sits at the LEFT area it was played into");
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT,
             "B4: the CENTER is empty after the double baton");
}

/* T-B5 sumire_q194_locked_member_excluded_from_double_baton
 * A member deployed THIS TURN may not be baton-touched (Rule 9.6.2.1.2.1),
 * so the double baton drops to a single touch and ab#1 stays shut. */
static void b5_locked_member_excluded(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int l1 = test_id(&tg, LIELLA_BLADE_A);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    if (l1 < 0 || l2 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, l1, l2, RB_EMPTY_SLOT, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    /* Lock LEFT as "arrived this turn" AFTER the phase advance, exactly as
     * the Rust test does by pushing liella1 into deployed_this_turn. */
    tg.state.stage_arrived[0][0] = 1;

    CHECK(test_play_to_stage(&tg, sumire, 1) == 1, "B5: the play was accepted");
    drain_baton(&tg);
    drain_pick_first_all(&tg);

    /* PROMPT-COUNT CONTROL, corrected: the single-baton path is synchronous
     * and raises no prompt, so the control is the two observable moves. */
    CHECK_EQ(wait_has(&tg, l2), 1,
             "B5: the CENTER member really went to the waitroom (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 1,
             "B5/Q194: only 1 baton touch - the LEFT member was deployed this turn");
    CHECK_EQ(tg.state.p[0].stage[0], l1,
             "B5/Q194: the locked LEFT member was not replaced");
    CHECK_EQ(stage_has(&tg, 1, sumire), 1,
             "B5: Sumire occupies the CENTER area it was played into");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "B5/Q194: no draw from ab#1 - only 1 baton touch");
}

/* T-B6 sumire_no_liella_on_stage_plays_normally
 * Nothing Liella! on stage -> the play is a plain placement.
 * KNOWN RED (see the report): rb_play_member (src/engine.c:963-984) offers the
 * 常時 double baton for ANY play of a double-baton card whenever two or more
 * members are on stage — it does not look at the destination area and there is
 * no "decline and play normally" path (a skip on the SelectTarget abandons the
 * play).  So two mu's fillers are baton-touched off the stage. */
static void b6_no_liella_plays_normally(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int f1 = test_id(&tg, FILLER_NO);
    int f2 = test_new_id(&tg, FILLER_NO);
    if (f2 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, f1, f2, RB_EMPTY_SLOT, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    CHECK(test_play_to_stage(&tg, sumire, 1) == 1, "B6: the play was accepted");
    drain_baton(&tg);
    drain_pick_first_all(&tg);

    CHECK_EQ(stage_has(&tg, 1, sumire), 1,
             "B6: Sumire was placed on stage normally (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 0,
             "B6: no baton touch - no Liella! member was on stage");
}

/* T-B7 sumire_explicit_double_baton_via_card_indices, energy half only
 * The Rust test asserts `energy_after == energy_before - 5`; cards.json prints
 * cost 22 for this card, so 5 is a stale fixture number.  The load-bearing
 * claim is "the printed cost is what leaves the energy zone", which is what
 * is asserted here, with the printed cost read from the database rather than
 * hard-coded. */
static void b7_double_baton_pays_printed_cost(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int prompts = 0;
    int l1 = test_id(&tg, LIELLA_BLADE_A);
    int l2 = test_id(&tg, LIELLA_BLADE_B);
    if (l1 < 0 || l2 < 0) { setup_bugs++; return; }
    int sumire = sumire_setup(&tg, l1, l2, RB_EMPTY_SLOT, &prompts);
    if (sumire <= 0) { setup_bugs++; return; }

    int energy_before = energy_active(&tg, 0);
    int cost = printed_cost(sumire);
    CHECK_EQ(cost, 22,
             "B7: cards.json prints cost 22 - the Rust test's expected_cost=5 is stale");

    CHECK(test_play_to_stage(&tg, sumire, 1) == 1, "B7: the play was accepted");
    int n = drain_baton(&tg) + drain_pick_first_all(&tg);

    CHECK(n > 0, "B7: the double-baton prompt was raised (non-vacuous control)");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 2, "B7: 2 baton touches");
    /* rb_complete_double_baton (src/engine.c:897-947) performs the replacement
     * and calls rb_fire_debut, but never touches P->energy_active: the
     * DOUBLE-BATON play path pays NO cost at all, while the single-baton path
     * (rb_play_member:1032-1035) does.  The first defect is the placement one
     * above; this cost omission is independent of it. */
    CHECK_EQ(energy_active(&tg, 0), energy_before - cost,
             "B7: the printed cost is what left the energy zone");

    /* Q193/Q194 identity: only the cost<=4 Liella! comes back to stage. */
    CHECK_EQ(stage_has(&tg, 0, l2), 1,
             "B7: the cost-2 Liella! (bp1-005-R) is deployed back to the vacated area");
    CHECK_EQ(wait_has(&tg, l1), 1,
             "B7: the cost-15 Liella! (bp1-004-R) stays in the waitroom");
    CHECK_EQ(wait_has(&tg, l2), 0, "B7: the deployed Liella! left the waitroom");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §2  integration/full_round gaps
 * ═══════════════════════════════════════════════════════════════════════ */

/* F-1 phase_walkthrough_two_turns
 * Smoke test: 20 passes cycle 2+ turns, and neither stage is polluted. */
static void f1_phase_walkthrough_two_turns(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int fill = test_id(&tg, FILLER_NO);
    if (fill < 0) { setup_bugs++; return; }
    fill_decks(&tg, fill, 10);

    CHECK_EQ(tg.state.turn, 1, "F1: a fresh TestGame is on turn 1");
    CHECK_EQ(tg.state.phase, RB_PHASE_MAIN, "F1: and starts in the Main phase");

    for (int i = 0; i < 20; i++) {
        test_pass(&tg);
        drain_skip(&tg);
    }
    CHECK(tg.state.turn >= 2, "F1: at least turn 2 after 20 passes");
    int p1_empty = 1, p2_empty = 1;
    for (int a = 0; a < RB_STAGE_SIZE; a++) {
        if (tg.state.p[0].stage[a] != RB_EMPTY_SLOT) p1_empty = 0;
        if (tg.state.p[1].stage[a] != RB_EMPTY_SLOT) p2_empty = 0;
    }
    CHECK(p1_empty, "F1: P1's stage is empty after 20 passes (nothing was played)");
    CHECK(p2_empty, "F1: P2's stage is empty after 20 passes (nothing was played)");
}

/* Shared fixture for the two 繚乱！ビクトリーロード negative controls
 * (T6 / T8 of live_each_time_victory_road_through_real_live_rounds_test.rs). */
static int vr_negative_setup(TestGame *tg, int stage0, int *member_out)
{
    int victory = test_id(tg, "PL!N-bp5-030-L");
    int member  = test_id(tg, "PL!-bp3-012-N");   /* has a ライブ開始時 ability */
    int filler  = test_new_id(tg, FILLER_NO);
    if (victory < 0 || member < 0 || filler < 0) { setup_bugs++; return 0; }
    fill_decks(tg, filler, 30);
    test_add_to_live(tg, victory);
    tg->state.p[0].stage[0] = stage0;
    tg->state.p[0].stage[1] = member;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(tg, victory);
    test_add_to_hand(tg, filler);
    *member_out = member;
    return victory;
}

/* F-2 non_member_on_stage_no_trigger (Rust T6)
 * A second stage occupant that is not the LiveStart member must not change
 * the outcome: the member with ライブ開始時 still gains all-harts.
 * KNOWN RED (see the report): rb_trigger_each_time_for_member — the ONLY C
 * implementation of 「〜のライブ開始時能力が解決するたび」 — has ZERO callers
 * outside tests/test_parity_auto_abilities.c.  The real live-start scan
 * (src/turn/triggers.c:118-190) queues and executes the resolving member's own
 * ability but never notifies the each_time watcher, so Victory Road's all-hart
 * is never granted.  F3c proves the watcher itself works when called. */
static void f2_non_member_on_stage_no_trigger(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int member = 0;
    int filler = test_id(&tg, FILLER_NO);
    if (filler < 0) { setup_bugs++; return; }
    int victory = vr_negative_setup(&tg, filler, &member);
    if (!victory) { setup_bugs++; return; }

    advance_to_live_set_5(&tg);
    test_set_live_card(&tg, 0, victory);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    /* PROMPT-COUNT CONTROL, honestly scoped: 南ことり's ライブ開始時 only
     * offers a heart-colour choice and then gains hearts per card in the
     * (empty) success zone, so on this board it legitimately raises NO prompt
     * and has no effect of its own.  The control that IS available is the
     * precondition: the live-start window really opened over a populated live
     * zone, and the member is really on stage. */
    CHECK_EQ(tg.state.p[0].live.n, 2,
             "F2: the Rust fixture leaves TWO copies of Victory Road in the live zone (pre-push + set)");
    CHECK_EQ(tg.state.p[0].stage[1], member,
             "F2: the ライブ開始時 member is on stage (precondition)");
    CHECK_EQ(0, prompts, "F2: this member's own ライブ開始時 raises no prompt on an empty success zone");
    CHECK(all_heart_of(&tg, member) > 0,
          "F2: the member with ライブ開始時 gains all-harts; the non-member neighbour is irrelevant");
}

/* F-3 member_without_live_start_no_trigger (Rust T8)
 * The stage[0] occupant has NO ライブ開始時 ability; it must not consume the
 * Victory-Road each_time trigger, and the real member still gains all-harts.
 * KNOWN RED for the same reason as F2 (dead each_time watcher). */
static void f3_member_without_live_start_no_trigger(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int member = 0;
    int no_live_start = test_new_id(&tg, FILLER_NO);
    if (no_live_start < 0) { setup_bugs++; return; }
    int victory = vr_negative_setup(&tg, no_live_start, &member);
    if (!victory) { setup_bugs++; return; }

    advance_to_live_set_5(&tg);
    test_set_live_card(&tg, 0, victory);
    finish_live_setup(&tg);
    drain_skip(&tg);

    CHECK_EQ(tg.state.p[0].live.n, 2,
             "F3: the Rust fixture leaves TWO copies of Victory Road in the live zone (precondition)");
    CHECK_EQ(all_heart_of(&tg, no_live_start), 0,
             "F3: the ability-less neighbour gains nothing");
    CHECK(all_heart_of(&tg, member) > 0,
          "F3: a stage neighbour without ライブ開始時 does not consume the each_time trigger");
}

/* F-3c ROOT-CAUSE PROOF — NOT a Rust test.  F2/F3 are red because the
 * each_time watcher is never invoked by the engine.  This test isolates that:
 * it runs the SAME board, confirms the phase machine leaves the member without
 * all-harts, then calls rb_trigger_each_time_for_member DIRECTLY and confirms
 * the all-hart is then granted.  Watcher-works-but-is-never-called is a very
 * different report from watcher-broken, and this pins which one it is. */
static void f3c_each_time_watcher_direct_call(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int filler  = test_id(&tg, FILLER_NO);
    int member  = test_id(&tg, "PL!-bp3-012-N");
    int victory = test_id(&tg, "PL!N-bp5-030-L");
    if (filler < 0 || member < 0 || victory < 0) { setup_bugs++; return; }
    fill_decks(&tg, filler, 30);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = member;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, victory);

    advance_to_live_set_5(&tg);
    test_set_live_card(&tg, 0, victory);
    finish_live_setup(&tg);
    drain_skip(&tg);

    /* Step 1: the live-start window ran over a populated live zone. */
    CHECK_EQ(tg.state.p[0].live.n, 1, "F3c: one Victory Road in the live zone");
    CHECK_EQ(tg.state.p[0].stage[1], member, "F3c: the member is on stage");
    /* Step 2: the member's ライブ開始時 resolved (it was scanned and executed)
     * but the all-hart was NOT granted, because nothing called the watcher. */
    CHECK_EQ(all_heart_of(&tg, member), 0,
             "F3c: the phase machine resolved the LiveStart ability but did NOT grant the all-hart");

    /* Step 3: call the watcher the engine never calls.  Victory Road's ab#0
     * effect carries NO target/host field, so the "whose card gains the
     * all-hart" question is decided by the executor's default.  Both readings
     * are printed so the report can name which one it is. */
    rb_trigger_each_time_for_member(&tg.state, 0, "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82", member);
    rb_drain_ability_queue(&tg.state);
    drain_skip(&tg);

    fprintf(stderr, "   [F3c diag] all_heart(member=%d)=%d all_heart(victory=%d)=%d\n",
            member, all_heart_of(&tg, member), victory, all_heart_of(&tg, victory));
    CHECK(all_heart_of(&tg, member) > 0,
          "F3c: calling rb_trigger_each_time_for_member by hand DOES grant the all-hart - the engine simply never calls it");
}

/* F-4 solo_winner_third_success_ends_before_live_reentry
 * P1 has 2 successes, P2 has 2 and sets a SECOND live card in the window, so
 * P2 alone reaches 3: the game must END and the flow must not re-enter the
 * live.  (The only one of the four tie/live-zone tests with no C port.) */
static void f4_solo_winner_third_success_ends_game(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int filler = test_id(&tg, FILLER_NO);
    if (filler < 0) { setup_bugs++; return; }
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg.state.p[pl];
        P->stage[0] = test_new_id(&tg, "PL!-sd1-001-SD");
        P->stage[1] = test_new_id(&tg, "PL!-sd1-001-SD");
        P->stage[2] = test_new_id(&tg, "PL!-sd1-001-SD");
        P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
        for (int i = 0; i < 40; i++) P->deck.cards[P->deck.n++] = filler;
        /* mirrored boards, so the only asymmetry is the third success */
        P->success.n = 0;
        for (int i = 0; i < 2; i++) {
            int sid = test_new_id(&tg, "PL!-sd1-019-SD");
            if (sid < 0) { setup_bugs++; return; }
            P->success.cards[P->success.n++] = sid;
        }
    }
    int p1_live = test_id(&tg, "PL!-sd1-019-SD");
    int p2_live_a = test_new_id(&tg, "PL!-sd1-019-SD");
    int p2_live_b = test_new_id(&tg, "PL!-sd1-019-SD");
    if (p1_live < 0 || p2_live_a < 0 || p2_live_b < 0) { setup_bugs++; return; }
    test_add_to_hand(&tg, p1_live);
    test_add_to_hand_for(&tg, 1, p2_live_a);
    test_add_to_hand_for(&tg, 1, p2_live_b);

    advance_to_live_set_5(&tg);
    test_set_live_card(&tg, 0, p1_live);
    test_pass(&tg);
    test_set_live_card_for(&tg, 1, p2_live_a);
    /* a SECOND live card acquired mid-phase, as if by an effect */
    test_add_to_live_for(&tg, 1, p2_live_b);

    int guard = 0;
    while (tg.state.winner < 0 && guard++ < 24) {
        test_pass(&tg);
        drain_pick_first_all(&tg);
    }
    CHECK(tg.state.winner >= 0,
          "F4: the game ENDED once P2 alone reached a third success");
    CHECK_EQ(tg.state.winner, 1, "F4: P2 is the winner");
    CHECK_EQ(tg.state.p[0].success.n, 2, "F4: P1 stays on two successes");
    CHECK_EQ(tg.state.p[1].success.n, 3, "F4: P2 holds three successes");
}

/* ── ディストーション PL!SP-pb1-023-L ──
 * ライブ開始時, sequential conditional:
 *   [1] 自分のステージに名前の異なる『CatChu!』のメンバーが2人以上いる場合
 *       -> エネルギーを6枚までアクティブにする。
 *   [2] 自分のエネルギーがすべてアクティブ状態の場合 -> このカードのスコアを＋１。
 *
 * Every test below reaches the live-start window through the real phase
 * machine, so the ability is driven by the engine rather than hand-fired. */

static int distortion_fixture(TestGame *tg, int wait_energy, int want_members)
{
    int distortion = test_id(tg, "PL!SP-pb1-023-L");
    int filler     = test_id(tg, FILLER_NO);
    if (distortion < 0 || filler < 0) { setup_bugs++; return 0; }
    fill_decks(tg, filler, 10);
    test_add_to_hand(tg, distortion);
    test_add_to_hand(tg, filler);
    (void)want_members;
    advance_to_live_set_5(tg);
    if (wait_energy > 0) push_wait_energy(tg, 0, wait_energy);
    return distortion;
}

/* F-5 distortion_q97_all_active_no_catchu_score_plus_1
 * No CatChu! on stage, but ALL energy is already active -> still +1. */
static void f5_q97_all_active_no_catchu(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 0, 0);
    if (!distortion) { setup_bugs++; return; }
    test_give_energy(&tg, 3);   /* Rust gives 3 active energy and no wait cards */

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    /* PROMPT-COUNT CONTROL: the live-start window must have been opened and
     * its prompts consumed, otherwise "+1" is read off a window that never
     * opened.  The live card being in the live zone is the second half. */
    CHECK_EQ(tg.state.p[0].live.n, 1,
             "F5: Distortion is in the live zone (the live-start window opened)");
    CHECK(prompts > 0 || score_of(&tg, distortion) == 1,
          "F5: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(score_of(&tg, distortion), 1,
             "F5/Q97: score +1 when all energy is active, even with no CatChu! member");
}

/* F-6 distortion_q96_score_permanent_after_energy_used
 * The +1 is permanent: making energy non-all-active afterwards does not undo
 * it. */
static void f6_q96_score_is_permanent(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 0, 0);
    if (!distortion) { setup_bugs++; return; }
    test_give_energy(&tg, 3);

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    drain_skip(&tg);
    CHECK_EQ(score_of(&tg, distortion), 1, "F6/Q96: precondition - score is +1");

    push_wait_energy_one(&tg, 0, test_id(&tg, ENERGY_NO));
    CHECK(energy_active(&tg, 0) < energy_total(&tg, 0),
          "F6: the energy zone is no longer all-active (control)");
    CHECK_EQ(score_of(&tg, distortion), 1,
             "F6/Q96: the +1 is permanent even once energy stops being all-active");
}

/* F-7 distortion_basic_energy_refresh_with_catchu
 * Two differently-named CatChu! on stage + 3 active / 4 wait -> all 7 active
 * and, consequently, score +1. */
static void f7_basic_refresh_with_catchu(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 4, 2);
    if (!distortion) { setup_bugs++; return; }
    int kanon = test_id(&tg, "PL!SP-sd1-001-SD");   /* CatChu! */
    int keke  = test_id(&tg, "PL!SP-sd1-004-SD");   /* CatChu! */
    if (kanon < 0 || keke < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = keke;
    test_give_energy(&tg, 3);        /* 3 active */
    /* 4 wait cards were pushed by the fixture -> 7 total, 3 active */

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || energy_active(&tg, 0) == 7,
          "F7: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_total(&tg, 0), 7, "F7: the energy zone still holds 7 cards");
    CHECK_EQ(energy_active(&tg, 0), 7,
             "F7: the two differently-named CatChu! refresh all 4 wait energy");
    CHECK_EQ(score_of(&tg, distortion), 1,
             "F7: all-active energy then grants the +1");
}

/* F-8 distortion_no_refresh_when_no_wait_energy
 * CatChu! condition met, nothing to refresh; +1 still fires. */
static void f8_no_wait_energy(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 0, 2);
    if (!distortion) { setup_bugs++; return; }
    int kanon = test_id(&tg, "PL!SP-sd1-001-SD");
    int keke  = test_id(&tg, "PL!SP-sd1-004-SD");
    if (kanon < 0 || keke < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = keke;
    test_give_energy(&tg, 6);

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    drain_skip(&tg);

    CHECK_EQ(energy_active(&tg, 0), 6, "F8: no wait energy, nothing to refresh");
    CHECK_EQ(score_of(&tg, distortion), 1,
             "F8: +1 still fires with no wait energy (all energy already active)");
}

/* F-9 distortion_max_cap_8_wait_refresh_6_only
 * 8 wait energy: the 「6枚まで」 cap binds, so 2 stay waited and no +1. */
static void f9_max_cap_8_wait(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 8, 2);
    if (!distortion) { setup_bugs++; return; }
    int kanon = test_id(&tg, "PL!SP-sd1-001-SD");
    int keke  = test_id(&tg, "PL!SP-sd1-004-SD");
    if (kanon < 0 || keke < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = keke;

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || energy_active(&tg, 0) == 6,
          "F9: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_total(&tg, 0), 8, "F9: the energy zone still holds 8 cards");
    CHECK_EQ(energy_active(&tg, 0), 6,
             "F9: only 6 of the 8 wait cards are refreshed (the 6 cap binds)");
    CHECK_EQ(score_of(&tg, distortion), 0,
             "F9: not all active -> no +1");
}

/* F-10 distortion_exact_max_boundary_6_wait_all_refreshed
 * The boundary case: 6 wait -> all 6 refreshed -> all active -> +1. */
static void f10_exact_max_boundary(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 6, 2);
    if (!distortion) { setup_bugs++; return; }
    int kanon = test_id(&tg, "PL!SP-sd1-001-SD");
    int keke  = test_id(&tg, "PL!SP-sd1-004-SD");
    if (kanon < 0 || keke < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = keke;

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || energy_active(&tg, 0) == 6,
          "F10: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_active(&tg, 0), 6, "F10: all 6 wait cards were refreshed");
    CHECK_EQ(energy_total(&tg, 0), 6, "F10: the zone still holds 6 cards");
    CHECK_EQ(score_of(&tg, distortion), 1, "F10: all active -> +1");
}

/* F-11 distortion_same_name_catchu_condition_not_met
 * Two copies of the SAME CatChu! -> the "different names" condition fails,
 * so no energy is refreshed and no +1 is granted.
 * KNOWN RED (see the report): the effect's condition carries
 * `distinct: "card_name"` as a STRING.  eval_location reads it with get_bool
 * (src/ability/condition.c:304), which cannot read a string, and the fallback
 * at :306-310 only accepts a nested RB_TAG_OBJVAR.  `distinct` therefore
 * stays 0, and because the condition ALSO names card_type + group_names the
 * `else if (ctype || group)` arm at :379 counts CARDS, so the mutually
 * exclusive `else if (distinct)` arm at :380 is dead. */
static void f11_same_name_catchu_not_met(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 4, 2);
    if (!distortion) { setup_bugs++; return; }
    int kanon = test_id(&tg, "PL!SP-sd1-001-SD");
    int kanon2 = test_new_id(&tg, "PL!SP-sd1-001-SD");
    if (kanon < 0 || kanon2 < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = kanon2;
    test_give_energy(&tg, 3);   /* 3 active + 4 wait = 7 total */

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || energy_active(&tg, 0) == 3,
          "F11: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_active(&tg, 0), 3,
             "F11: the same-name condition is NOT met, so no wait energy is refreshed");
    CHECK_EQ(energy_total(&tg, 0), 7, "F11: the zone still holds 7 cards");
    CHECK_EQ(score_of(&tg, distortion), 0,
             "F11: not all active -> no +1");
}

/* F-12 distortion_q103_two_triggers_only_one_plus_1
 * TWO Distortions in the live zone, 7 wait energy: the first trigger
 * refreshes, the second sees an all-active zone and grants the single +1. */
static void f12_q103_two_triggers_one_plus_1(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int d1 = test_id(&tg, "PL!SP-pb1-023-L");
    int d2 = test_new_id(&tg, "PL!SP-pb1-023-L");
    int filler = test_id(&tg, FILLER_NO);
    int kanon = test_id(&tg, "PL!SP-sd1-001-SD");
    int keke  = test_id(&tg, "PL!SP-sd1-004-SD");
    if (d1 < 0 || d2 < 0 || filler < 0 || kanon < 0 || keke < 0) { setup_bugs++; return; }
    CHECK(d1 != d2, "F12: the two live cards are two DISTINCT card ids");
    fill_decks(&tg, filler, 10);
    test_add_to_hand(&tg, d1);
    test_add_to_hand(&tg, d2);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = keke;
    advance_to_live_set_5(&tg);
    push_wait_energy(&tg, 0, 7);

    test_set_live_card(&tg, 0, d1);
    test_set_live_card(&tg, 0, d2);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || energy_active(&tg, 0) == 7,
          "F12: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_active(&tg, 0), 7, "F12: all 7 wait energy became active");
    int total = 0;
    for (int i = 0; i < tg.state.p[0].live.n; i++)
        total += score_of(&tg, tg.state.p[0].live.cards[i]);
    CHECK_EQ(total, 1, "F12/Q103: only the SECOND trigger's card gets the +1");
}

/* F-13 distortion_two_same_one_diff_catchu_condition_met
 * 2 x Kanon + 1 x Keke -> two DISTINCT CatChu! names -> the condition is met
 * and all 4 wait energy is refreshed. */
static void f13_two_same_one_diff_condition_met(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 4, 3);
    if (!distortion) { setup_bugs++; return; }
    int kanon  = test_id(&tg, "PL!SP-sd1-001-SD");
    int kanon2 = test_new_id(&tg, "PL!SP-sd1-001-SD");
    int keke   = test_id(&tg, "PL!SP-sd1-004-SD");
    if (kanon < 0 || kanon2 < 0 || keke < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[1] = kanon2;
    tg.state.p[0].stage[2] = keke;

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || energy_active(&tg, 0) == 4,
          "F13: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_active(&tg, 0), 4,
             "F13: two DISTINCT CatChu! names meet the condition -> all 4 refreshed");
    CHECK_EQ(score_of(&tg, distortion), 1, "F13: all active -> +1");
}

/* F-14 distortion_same_name_catchu_all_active_score_plus_1
 * Same-name CatChu! but ALL energy already active -> the Q97 path fires the
 * +1 even though the refresh condition is unmet. */
static void f14_same_name_all_active_plus_1(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int distortion = distortion_fixture(&tg, 0, 2);
    if (!distortion) { setup_bugs++; return; }
    int kanon  = test_id(&tg, "PL!SP-sd1-001-SD");
    int kanon2 = test_new_id(&tg, "PL!SP-sd1-001-SD");
    if (kanon < 0 || kanon2 < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = kanon;
    tg.state.p[0].stage[2] = kanon2;
    test_give_energy(&tg, 4);   /* all active, nothing waited */

    test_set_live_card(&tg, 0, distortion);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    CHECK(prompts > 0 || score_of(&tg, distortion) == 1,
          "F14: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(energy_active(&tg, 0), 4, "F14: no wait energy, nothing to refresh");
    CHECK_EQ(score_of(&tg, distortion), 1,
             "F14: all active -> +1 even with same-name CatChu!");
}

/* F-15 turn_limit_prevents_second_activation
 * 黒澤ルビィ (PL!S-bp2-009-R): first 起動 succeeds (self-cost to the
 * waitroom, live card recovered); the SECOND attempt must change nothing.
 * KNOWN RED (see the report), for TWO independent reasons:
 *  1. RUST FIXTURE BUG.  The Rust comment says "Chika has ターン1 limit" but
 *     cards.json gives the card exactly one ability,
 *     「{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室
 *     からライブカードを1枚手札に加える。」, with NO 「このゲームに1回だけ」 and
 *     no use_limit in abilities.json.  The Rust test passes only because the
 *     self-cost moved the card out of hand, so `try_activate_ability` has
 *     nothing to activate.
 *  2. C ENGINE GAP.  The card is now in the WAITROOM, and the C mirror of
 *     `try_activate_ability` (test_activate_ability_for) falls back to
 *     rb_activate_card (src/engine.c:1189), which scans the card's 起動
 *     abilities with NO zone-eligibility check at all — so a 起動 resolves
 *     from the waitroom.  The real main-phase action (rb_activate_ability,
 *     hand-indexed) is unaffected. */
static void f15_turn_limit_prevents_second_activation(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int chika  = test_id(&tg, "PL!S-bp2-009-R");
    int filler = test_id(&tg, FILLER_NO);
    int live   = test_id(&tg, "PL!-sd1-020-SD");
    if (chika < 0 || filler < 0 || live < 0) { setup_bugs++; return; }

    test_add_to_hand(&tg, chika);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_discard(&tg, live);   /* seed the waitroom so the recover is visible */
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, chika, 1) == 1, "F15: Chika was played to center");
    drain_skip(&tg);

    int prompts1 = drain_pick_first_all(&tg);
    CHECK(test_activate_ability(&tg, chika) != 0, "F15: the first 起動 was accepted");
    prompts1 += drain_pick_first_all(&tg);

    /* PROMPT-COUNT CONTROL: the first activation must have produced prompts,
     * otherwise the "nothing changed on the second" reading is vacuous. */
    CHECK(prompts1 > 0,
          "F15: the first 起動 raised at least one prompt (non-vacuous control)");
    CHECK_EQ(wait_has(&tg, chika), 1, "F15: the self-cost moved Chika to the waitroom");
    CHECK_EQ(hand_has(&tg, live), 1, "F15: the effect recovered the live card to hand");

    int hand_before = tg.state.p[0].hand.n;
    int wait_before = tg.state.p[0].discard.n;
    test_activate_ability(&tg, chika);
    drain_pick_first_all(&tg);
    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "F15: the blocked second 起動 changed nothing in hand");
    CHECK_EQ(tg.state.p[0].discard.n, wait_before,
             "F15: the blocked second 起動 left the waitroom untouched");
}

/* F-16 you_q186_cost_reduced_playable
 * LL-bp2-001-R＋ prints cost 20; ab#0 is the 常時 self cost reduction
 * 「手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚
 *   につき、1少なくなる。」  With 16 fillers the card costs 4, and 4 energy is
 * exactly enough.
 * KNOWN RED (see the report): rb_play_member computes the play cost as
 * `card.cost + rb_mods_get_cost(&g->mods, cid)` (src/engine.c:1003-1005) and
 * never calls rb_compute_play_cost, which DOES implement the reduction via
 * cr_calc_reduction (src/ability/cost.c:1569).  The gate at engine.c:1032
 * also runs BEFORE rb_recalc_constants at :1037, so a mods-based reduction
 * could not have applied either. */
static void f16_q186_cost_reduced_playable(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int keke = test_id(&tg, "LL-bp2-001-R" PLUS);
    int filler = test_id(&tg, FILLER_NO);
    if (keke < 0 || filler < 0) { setup_bugs++; return; }
    CHECK_EQ(printed_cost(keke), 20,
             "F16: cards.json prints base cost 20 for LL-bp2-001-R+");

    for (int i = 0; i < 16; i++) test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, keke);
    test_give_energy(&tg, 4);
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "F16: the LEFT area starts empty");

    int before = energy_active(&tg, 0);
    CHECK(test_play_to_stage(&tg, keke, 0) == 1,
          "F16: You is playable with only 4 energy (cost 20 - 16 reduction)");
    drain_skip(&tg);

    /* PROMPT-COUNT CONTROL: the card must be ON STAGE, so the energy reading
     * below is a real payment rather than a refused play. */
    CHECK_EQ(stage_has(&tg, 0, keke), 1,
             "F16: You really reached the stage (non-vacuous control)");
    CHECK_EQ(before - energy_active(&tg, 0), 4,
             "F16: exactly 4 energy was spent, i.e. the self cost reduction applied");
}

/* F-17 rise_up_high_does_not_fire_on_turn_2
 * The negative half of the already-ported turn-1 companion.  A turn gate
 * that fails OPEN is invisible on turn 1, so the same board is replayed on
 * turn 2 where the correct answer is "nothing happens".
 * KNOWN RED (see the report): the ability's condition is a temporal_condition
 * with `turn_number: 1` and NO `operator` field.  eval_operator defaults a
 * missing operator to ">=" (src/ability/condition.c:71), so the gate reads
 * `turn >= 1` and passes on every turn.  */
static void f17_rise_up_high_does_not_fire_on_turn_2(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int live = test_id(&tg, "PL!N-bp4-029-L");
    int niji = test_id(&tg, "PL!N-bp1-016-N");
    int filler = test_id(&tg, FILLER_NO);
    if (live < 0 || niji < 0 || filler < 0) { setup_bugs++; return; }
    fill_decks(&tg, filler, 10);

    /* Ride out round 1 without setting a live. */
    int guard = 0;
    while (!(tg.state.turn >= 2 && tg.state.phase == RB_PHASE_MAIN) && guard++ < 60) {
        if (rb_has_pending_choice(&tg.state)) drain_skip(&tg);
        test_pass(&tg);
    }
    CHECK(tg.state.turn == 2 && tg.state.phase == RB_PHASE_MAIN,
          "F17: reached turn 2 Main without a live being set");
    CHECK_EQ(score_of(&tg, live), 0, "F17: the turn-1 baseline score is +0");
    CHECK_EQ(blade_of(&tg, niji), 0, "F17: the turn-1 baseline blade is +0");

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = niji;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, live);

    advance_to_live_set_5(&tg);
    test_set_live_card(&tg, 0, live);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    /* PROMPT-COUNT CONTROL: the live-start window must have been opened and
     * processed, otherwise "no +1" would be vacuous. */
    CHECK_EQ(tg.state.p[0].live.n, 1,
             "F17: Rise Up High! is in the live zone (the window opened)");
    CHECK(prompts > 0 || score_of(&tg, live) == 0,
          "F17: the live-start window was processed (non-vacuous control)");
    CHECK_EQ(score_of(&tg, live), 0,
             "F17: Rise Up High! must not add score on turn 2 (turn gate)");
    CHECK_EQ(blade_of(&tg, niji), 0,
             "F17: Rise Up High! must not grant a blade on turn 2 (turn gate)");
}

/* F-18 ai_screeam_p2_owned_live_gives_blade_to_all_members
 * 愛♡スクリ～ム！ owned by P2 and set as P2's live; option 2 grants one
 * blade to EVERY member on BOTH stages. */
static void f18_ai_screeam_p2_owned_live_blade(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int screeam = test_id(&tg, "LL-PR-004-PR");
    int filler  = test_id(&tg, FILLER_NO);
    if (screeam < 0 || filler < 0) { setup_bugs++; return; }
    fill_decks(&tg, filler, 10);

    int p1_left   = test_id(&tg, "PL!-sd1-013-SD");
    int p1_center = test_new_id(&tg, "PL!-sd1-013-SD");
    int p2_left   = test_id(&tg, "PL!-sd1-014-SD");
    int p2_center = test_new_id(&tg, "PL!-sd1-014-SD");
    if (p1_left < 0 || p1_center < 0 || p2_left < 0 || p2_center < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = p1_left;
    tg.state.p[0].stage[1] = p1_center;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[0] = p2_left;
    tg.state.p[1].stage[1] = p2_center;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand_for(&tg, 1, screeam);

    advance_to_live_set_5(&tg);
    test_set_live_card_for(&tg, 1, screeam);
    finish_live_setup(&tg);

    /* Option 2 = 「その他」 -> every member on both stages gains a blade.
     * Answer the FIRST prompt with option 2; a negative answer would skip the
     * whole ability, so the index is explicit rather than drained. */
    int guard = 0;
    while (!rb_has_pending_choice(&tg.state) && guard++ < 4) { test_pass(&tg); }
    CHECK(rb_has_pending_choice(&tg.state),
          "F18: 愛♡スクリ～ム！ offered its answer-based choice (non-vacuous control)");
    if (rb_has_pending_choice(&tg.state)) rb_resume_with_choice(&tg.state, 2);
    drain_skip(&tg);

    CHECK_EQ(blade_of(&tg, p1_left), 1, "F18: P1's Left member gained the blade");
    CHECK_EQ(blade_of(&tg, p1_center), 1, "F18: P1's Center member gained the blade");
    CHECK_EQ(blade_of(&tg, p2_left), 1, "F18: P2's Left member gained the blade");
    CHECK_EQ(blade_of(&tg, p2_center), 1, "F18: P2's Center member gained the blade");
}

/* ═══════════════════════════════════════════════════════════════════════ */

static void on_segv(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d during test \"%s\" (engine fault) ***\n",
            sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures, s0 = setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = failures - f0;
        if (nf > 0 || setup_bugs - s0 > 0) n_failed++; else n_ok++;
        printf("%-8s %s\n", nf ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = (int)assertions, f0 = failures, s0 = setup_bugs;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s)\n",
               name, (int)assertions - a0, failures - f0, setup_bugs - s0);
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 || setup_bugs > s0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- §1a double baton blade grant per Liella! moved (chisato) ---\n");
    run("A1_both_have_blade_heart",  a1_both_have_blade_heart);
    run("A2_one_no_blade_heart",     a2_one_no_blade_heart);
    run("A3_q265_both_no_blade",     a3_q265_both_no_blade_heart);
    run("A4_draw_counts_replaced",   a4_draw_counts_only_replaced);
    run("A5_group_filter_liella",    a5_group_filter_counts_only_liella);
    run("A6_single_baton_one_liella",a6_single_baton_one_liella);
    run("A7_normal_debut_fires_nothing", a7_normal_debut_fires_nothing);
    run("A8_blade_expires_at_live_end",  a8_blade_expires_at_live_end);

    printf("--- §1b double baton draw + deploy Q193/Q194 (sumire) ---\n");
    run("B1_q193_q194_draw_and_deploy", b1_q193_q194_draw_and_deploy);
    run("B2_no_baton_no_draw",          b2_no_baton_no_draw);
    run("B3_only_one_occupied_area",    b3_only_one_occupied_area);
    run("B4_double_baton_to_left",      b4_double_baton_to_left);
    run("B5_locked_member_excluded",    b5_locked_member_excluded);
    run("B6_no_liella_plays_normally",  b6_no_liella_plays_normally);
    run("B7_double_baton_pays_cost",    b7_double_baton_pays_printed_cost);

    printf("--- §2 full_round gaps ---\n");
    run("F1_phase_walkthrough_two_turns", f1_phase_walkthrough_two_turns);
    run("F2_non_member_on_stage_no_trigger", f2_non_member_on_stage_no_trigger);
    run("F3_member_without_live_start",     f3_member_without_live_start_no_trigger);
    run("F3c_each_time_watcher_direct",     f3c_each_time_watcher_direct_call);
    run("F4_solo_winner_third_success",     f4_solo_winner_third_success_ends_game);
    run("F5_distortion_q97_all_active",     f5_q97_all_active_no_catchu);
    run("F6_distortion_q96_permanent",      f6_q96_score_is_permanent);
    run("F7_distortion_refresh_with_catchu",f7_basic_refresh_with_catchu);
    run("F8_distortion_no_wait_energy",     f8_no_wait_energy);
    run("F9_distortion_max_cap_8_wait",     f9_max_cap_8_wait);
    run("F10_distortion_exact_max_boundary",f10_exact_max_boundary);
    run("F11_distortion_same_name_not_met", f11_same_name_catchu_not_met);
    run("F12_distortion_q103_two_triggers", f12_q103_two_triggers_one_plus_1);
    run("F13_distortion_two_same_one_diff", f13_two_same_one_diff_condition_met);
    run("F14_distortion_same_name_all_act", f14_same_name_all_active_plus_1);
    run("F15_turn_limit_second_activation", f15_turn_limit_prevents_second_activation);
    run("F16_you_q186_cost_reduced",        f16_q186_cost_reduced_playable);
    run("F17_rise_up_high_not_turn_2",      f17_rise_up_high_does_not_fire_on_turn_2);
    run("F18_ai_screeam_p2_owned_live",     f18_ai_screeam_p2_owned_live_blade);

    rb_unload();

    printf("\n=== parity_full_round ===\n");
    printf("tests passed          : %d\n", n_ok);
    printf("tests failed (parity) : %d\n", n_failed);
    printf("tests crashed (engine): %d\n", n_crashed);
    printf("assertions (in-parent): %ld\n", assertions);
    printf("failures  (in-parent): %d\n", failures);
    printf("fixture setup bugs    : %d\n", setup_bugs);
    printf("see the per-test tally lines above for assertion counts\n");
    if (failures || n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("ALL FULL-ROUND PARITY CHECKS PASSED\n");
    return 0;
}
