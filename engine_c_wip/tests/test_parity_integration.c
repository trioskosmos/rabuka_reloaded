/* Parity suite for the Rust integration cluster
 *   engine/tests/test_modules/integration/
 *     full_round/  live_each_time_victory_road..., live_score_tie...,
 *                  real_card_phase_walkthrough..., rps_through_turn3...
 *     multi_card/  opponent_forced_position_change..., round5_seat_relative...,
 *                  sumire_wien_yell_no_blade_gain...
 *     parser_e2e/  parser_issues_e2e_test.rs, _part2_test.rs, _part3_test.rs
 *     per_card/    hard_tier_cards..., live_end_expiry_rollover...,
 *                  q203_q204_q218_eternalize...
 *   plus the engine-facing half of integration/pvp_room_test.rs
 * (the `pvp_player_can_act` web-server gate itself has no C port — see GAP-00.)
 *
 * These are the end-to-end tests: they drive whole turns, whole live rounds and
 * multi-card cross-player chains, so they are the strongest parity guard in the
 * repo. Each section below names the Rust file it mirrors.
 *
 * KNOWN-DIVERGENCE POLICY
 *   A divergence that was CONFIRMED against the Rust source is reported through
 *   EXPECTED_GAP(): it is counted in `gaps`, never in `failures`, and printed
 *   with a "GAP:" prefix so the worklist is greppable.  exit status is non-zero
 *   ONLY when a real (non-GAP) assertion fails.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int assertions;
static int gaps;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A confirmed C-vs-Rust divergence.  Counted separately from failures. */
#define EXPECTED_GAP(desc, condition) do { \
    assertions++; \
    if (!(condition)) { \
        printf("GAP: %s\n", desc); \
        gaps++; \
    } else { \
        printf("ok(gap-closed): %s\n", desc); \
    } \
} while (0)

/* ── helpers mirroring engine/tests/helpers/mod.rs ────────────────────── */

/* helpers::fill_decks — clear both decks and push `n` copies of `filler`. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

/* parser_e2e's LOCAL fill_decks (20 cards, id_ref, no clear). */
static void fill_decks20(TestGame *tg)
{
    int f = test_id(tg, "PL!-sd1-010-SD");
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        for (int i = 0; i < 20 && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = f;
    }
}

/* Main -> LiveCardSet(first attacker).  The C phase machine is
 * Main -> Active -> Energy -> Draw -> Main -> LiveSet, so 5 passes. */
static void advance_to_live_set_p1(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* LiveCardSet(second attacker) -> the ライブ開始時 window. */
static void advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* Answer every pending choice.  pick_first: 1 answers index 0 for anything that
 * is not a SelectAutoAbility (mirrors the parser_e2e drain loops), 0 answers
 * empty (the "skip everything" drain). */
static void drain(TestGame *tg, int pick_first)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const char *t = test_pending_choice_type(tg);
        if (!strcmp(t, "SelectAutoAbility")) {
            rb_resume_with_choice(&tg->state, 0);
        } else if (pick_first) {
            rb_resume_with_choice(&tg->state, 0);
        } else {
            rb_resume_with_choice(&tg->state, -1);
        }
    }
}

/* Skip every pending choice including auto-ability ones. */
static void drain_skip_all(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40)
        rb_resume_with_choice(&tg->state, -1);
}

/* Choose the first `n` indices at the current choice (multi-select). */
static void select_n(TestGame *tg, int n) __attribute__((unused));
static void select_n(TestGame *tg, int n)
{
    int idx[8];
    for (int i = 0; i < n && i < 8; i++) idx[i] = i;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* helpers::fire_trigger — queue the trigger for one seat and resolve it.
 * The C engine has no single-ability "fire" entry point, so this queues every
 * ability the seat owns for `trigger` (rb_queue_trigger_abilities) and drains,
 * which is a superset of Rust's trigger_auto_ability(card only). */
static void fire_trigger(TestGame *tg, int pl, int cid, const char *trigger)
{
    rb_queue_trigger_abilities(&tg->state, pl, trigger);
    tg->state.activating_card = cid;
    rb_drain_ability_queue(&tg->state);
}

/* Give a seat a fixed energy-zone total with an `active` prefix (round5
 * give_total_energy). */
static void give_total_energy(TestGame *tg, int pl, int active, int waited)
{
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if (eid < 0) eid = 0;
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < active + waited; i++)
        if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = eid;
    P->energy_active = active;
}

/* Set the live card for the second seat (test_set_live_card is P1-only). */
static void set_live_p2(TestGame *tg, int card_id)
{
    RbPlayer *P = &tg->state.p[1];
    for (int i = 0; i < P->hand.n; i++)
        if (P->hand.cards[i] == card_id) { rb_hand_remove_card(P, i); break; }
    int z = P->live.n;
    if (z >= RB_MAX_LIVE_CARDS) return;
    P->live.cards[z] = card_id;
    P->live.n = z + 1;
}

static int is_wait(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && !strcmp(o, "wait");
}

static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}

/* Distinct heart01..heart06 colours a card PRINTS (parser_e2e distinct_note_colors
 * / part3 printed_notes).  Computed from card data so a data change shows up as a
 * failing number, not as a tautology. */
static int distinct_note_colors(int card_id)
{
    Card c;
    int n = 0;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    for (int i = 0; i < c.n_hearts; i++) {
        if (c.heart_count[i] == 0) continue;
        int idx = rb_heart_color_index(c.heart_color[i]);
        if (idx >= 1 && idx <= 6) n++;
    }
    rb_free_card(&c);
    return n;
}

/* Count live cards in a hand. */
static int live_cards_in_hand(TestGame *tg, int pl)
{
    RbPlayer *P = &tg->state.p[pl];
    int n = 0;
    for (int i = 0; i < P->hand.n; i++)
        if (rb_card_is_live(P->hand.cards[i])) n++;
    return n;
}

/* ===================================================================== */
/* A. full_round/rps_through_turn3_main_phase_flow_test.rs                */
/*    e2e_full_game_rps_to_turn3                                         */
/*    The C phase machine (src/turn/phase.c:37 rb_advance_phase) is a      */
/*    linear Main->Active->Energy->Draw->Main->LiveSet walk with no RPS    */
/*    action, so the rock/paper/scissors half has no C counterpart.        */
/* ===================================================================== */

static void test_phase_walk_through_turn3(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);

    CHECK_EQ(tg.state.phase, RB_PHASE_MAIN, "walk: a fresh TestGame starts in Main");
    CHECK_EQ(tg.state.active, tg.state.first_attacker, "walk: the first attacker acts first");

    test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_ACTIVE, "walk: Main pass hands over to the second attacker");
    CHECK_EQ(tg.state.active, tg.state.second_attacker, "walk: the second attacker is now active");

    test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_ENERGY, "walk: Active -> Energy");
    test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_DRAW, "walk: Energy -> Draw");
    test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_MAIN, "walk: Draw -> Main (second attacker)");

    test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET, "walk: second attacker's Main -> LiveCardSet");
    CHECK_EQ(tg.state.active, tg.state.first_attacker,
             "walk: LiveCardSet opens with the first attacker");

    test_pass(&tg);
    CHECK_EQ(tg.state.active, tg.state.second_attacker,
             "walk: LiveCardSet second window is the second attacker");
    test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_PERFORMANCE,
             "walk: after the second LiveCardSet the performance window opens (ライブ開始時 fires here)");

    /* Run the live round out; the victory determination must roll the turn over. */
    int guard = 0;
    while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 24) {
        test_pass(&tg);
        drain_skip_all(&tg);
    }
    CHECK_EQ(tg.state.phase, RB_PHASE_ACTIVE,
             "walk: the live round rolls over into Active of the next turn");
    CHECK(tg.state.turn >= 2, "walk: the turn counter advanced past the first round");

    /* GAP-00: the RPS sub-flow of e2e_full_game_rps_to_turn3.
     * Rust drives TurnEngine::execute_main_phase_action(ActionType::RockChoice /
     * ScissorsChoice) and asserts the tie-reset, rps_winner and the
     * ChooseFirstAttacker -> MulliganFirstAttacker -> MulliganSecondAttacker ->
     * Active chain.  C decides RPS deterministically inside rb_game_init
     * (src/engine.c:1432-1440) and rb_execute_main_phase_action only dispatches
     * UseAbility (0) and Pass (1) (src/engine.c:1361-1376); RB_PHASE_RPS and
     * the two RB_PHASE_MULLIGAN_* slots are unreachable from any action. */
    EXPECTED_GAP("GAP-00 RPS + mulligan action chain is not ported to C "
                 "(engine/src/turn/phases.rs vs engine_c_wip/src/engine.c:1361)",
                 tg.state.phase == RB_PHASE_RPS);
}

/* ===================================================================== */
/* B. full_round/live_score_tie_live_zone_placement_across_rounds_test.rs */
/*    tie_with_two_card_live_zones_allows_selection                        */
/*    tie_at_two_successes_blocks_third_placement                         */
/*    solo_winner_third_success_ends_before_live_reentry                  */
/*    no_live_cards_at_all_no_winner_clean_rollover                       */
/* ===================================================================== */

static void mirrored_support_boards(TestGame *tg)
{
    int m1 = test_id(tg, "PL!-sd1-001-SD");
    test_add_to_stage(tg, 0, m1);
    test_add_to_stage(tg, 1, test_new_id(tg, "PL!-sd1-001-SD"));
    test_add_to_stage(tg, 2, test_new_id(tg, "PL!-sd1-001-SD"));
    test_set_opp_stage(tg, 0, test_new_id(tg, "PL!-sd1-001-SD"));
    test_set_opp_stage(tg, 1, test_new_id(tg, "PL!-sd1-001-SD"));
    test_set_opp_stage(tg, 2, test_new_id(tg, "PL!-sd1-001-SD"));
}

static void test_live_zone_two_card_selection(void)
{
    TestGame tg;
    test_game_new(&tg);
    mirrored_support_boards(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg.state.p[pl];
        P->deck.n = 0; P->hand.n = 0; P->discard.n = 0; P->success.n = 0;
        for (int i = 0; i < 40; i++) P->deck.cards[P->deck.n++] = filler;
    }
    int p1_a = test_id(&tg, "PL!-sd1-019-SD");
    int p2_a = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, p1_a);
    tg.state.p[1].hand.cards[tg.state.p[1].hand.n++] = p2_a;

    advance_to_live_set_p1(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET, "two-card live: reached LiveCardSet");
    test_set_live_card(&tg, 0, p1_a);
    set_live_p2(&tg, p2_a);
    /* A SECOND live card per side, as if granted by an effect. */
    tg.state.p[0].live.cards[tg.state.p[0].live.n++] = test_new_id(&tg, "PL!-sd1-019-SD");
    tg.state.p[1].live.cards[tg.state.p[1].live.n++] = test_new_id(&tg, "PL!-sd1-019-SD");
    CHECK_EQ(tg.state.p[0].live.n, 2, "two-card live: P1 holds two live cards");
    CHECK_EQ(tg.state.p[1].live.n, 2, "two-card live: P2 holds two live cards");

    int guard = 0;
    while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 24) {
        test_pass(&tg);
        drain(&tg, 1);
    }
    CHECK(tg.state.phase != RB_PHASE_PERFORMANCE && tg.state.phase != RB_PHASE_LIVE_SET,
          "two-card live: the flow crossed victory determination");
    CHECK_EQ(tg.state.p[0].success.n, 1, "two-card live: P1's tied live card reached the success zone");
    CHECK_EQ(tg.state.p[1].success.n, 1, "two-card live: P2's tied live card reached the success zone");
}

static void test_tie_at_two_successes_blocks_third(void)
{
    TestGame tg;
    test_game_new(&tg);
    mirrored_support_boards(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg.state.p[pl];
        P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
        for (int i = 0; i < 40; i++) P->deck.cards[P->deck.n++] = filler;
    }
    test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));

    int p1_live = test_id(&tg, "PL!-sd1-019-SD");
    int p2_live = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, p1_live);
    tg.state.p[1].hand.cards[tg.state.p[1].hand.n++] = p2_live;

    advance_to_live_set_p1(&tg);
    test_set_live_card(&tg, 0, p1_live);
    set_live_p2(&tg, p2_live);

    int guard = 0;
    while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 24) {
        test_pass(&tg);
        drain(&tg, 1);
    }
    CHECK_EQ(tg.state.p[0].live.n, 0, "two successes: P1's live zone is empty again");
    CHECK_EQ(tg.state.p[1].live.n, 0, "two successes: P2's live zone is empty again");
    CHECK_EQ(tg.state.p[0].success.n, 2, "two successes: P1 stays on two successes (no third placement)");
    CHECK_EQ(tg.state.p[1].success.n, 2, "two successes: P2 stays on two successes (no third placement)");
    CHECK_EQ(tg.state.winner, -1, "two successes on both sides is not a finished game");
}

static void test_no_live_cards_clean_rollover(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);

    int guard = 0;
    while (!(tg.state.phase == RB_PHASE_ACTIVE && tg.state.active == tg.state.first_attacker)
           && guard++ < 24) {
        test_pass(&tg);
        drain_skip_all(&tg);
    }
    CHECK_EQ(tg.state.p[0].success.n, 0,
             "rule 8.4.6.1: no live cards set -> no winner -> no success-zone placement (P1)");
    CHECK_EQ(tg.state.p[1].success.n, 0,
             "rule 8.4.6.1: no live cards set -> no winner -> no success-zone placement (P2)");
    CHECK(tg.state.turn >= 1, "rule 8.4.6.1: the round still rolled over");
}

/* ===================================================================== */
/* C. multi_card/opponent_forced_position_change_arms_opponent_area_move_ */
/*    watcher_test.rs                                                    */
/*    himeko_debut_repositions_opponent_member_and_koko_responds          */
/*    himeko_gate_blocked_no_reposition_no_koko_response                  */
/* ===================================================================== */

static void test_himeko_repositions_opponent_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    int rurino  = test_id(&tg, "PL!HS-PR-029-PR");   /* MiraKura member */
    int himeko  = test_id(&tg, "PL!HS-pb1-014-R");   /* 安養寺姫芽, 登場 */
    int koko    = test_id(&tg, "PL!SP-sd2-002-P");   /* 唐可可, 自動 area-move */
    int filler  = test_id(&tg, "PL!-sd1-010-SD");    /* mu's, breaks the gate */

    test_add_to_stage(&tg, 0, rurino);
    test_set_opp_stage(&tg, 2, koko);
    test_add_to_hand(&tg, himeko);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 15);

    test_play_to_stage(&tg, himeko, 1);
    CHECK_EQ(tg.state.p[1].stage[2], koko, "himeko: 可可 starts parked at P2-right");
    drain(&tg, 1);

    CHECK(tg.state.p[1].stage[2] != koko,
          "himeko: 姫芽's debut must displace 可可 off P2-right");
    EXPECTED_GAP("himeko: 可可 must land on P2-center (姫芽's front area)",
                 tg.state.p[1].stage[1] == koko);
    EXPECTED_GAP("himeko: 可可's opponent-caused area-move watcher grants heart06",
                 heart_of(&tg, koko, RB_HEART_PURPLE) == 1);
}

static void test_himeko_gate_blocked(void)
{
    TestGame tg;
    test_game_new(&tg);
    int himeko = test_id(&tg, "PL!HS-pb1-014-R");
    int koko   = test_id(&tg, "PL!SP-sd2-002-P");
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    test_add_to_stage(&tg, 0, outsider);
    test_set_opp_stage(&tg, 2, koko);
    test_add_to_hand(&tg, himeko);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 15);

    test_play_to_stage(&tg, himeko, 1);
    drain_skip_all(&tg);
    CHECK_EQ(tg.state.p[1].stage[2], koko,
             "himeko gate: a non-MiraKura stage member blocks the reposition, 可可 stays at P2-right");
    CHECK_EQ(heart_of(&tg, koko, RB_HEART_PURPLE), 0,
             "himeko gate: no movement happened, so 可可's watcher is silent");
}

/* ===================================================================== */
/* D. multi_card/round5_seat_relative_live_success_and_live_start_        */
/*    cross_player_test.rs — PL!S-bp6-022-L 近未来ハッピーエンド          */
/*    p2_owned_copy_compares_against_p1_energy                          */
/*    p2_owned_copy_no_bonus_when_p1_not_strictly_ahead                   */
/*    mirror_copies_each_evaluate_against_own_opponent                    */
/*    waited_energy_still_counts_for_comparison                          */
/* ===================================================================== */

static void test_seat_relative_energy_comparison(void)
{
    /* P2-owned copy, P1 ahead. */
    {
        TestGame tg;
        test_game_new(&tg);
        int live = test_id(&tg, "PL!S-bp6-022-L");
        test_add_to_opp_live(&tg, live);
        give_total_energy(&tg, 0, 3, 0);
        give_total_energy(&tg, 1, 1, 0);
        fire_trigger(&tg, 1, live, "ライブ成功時");
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, live), 1,
                 "round5: P2's copy scores +1 because opponent(P1)=3 > self(P2)=1");
    }
    /* P2-owned copy, P1 behind. */
    {
        TestGame tg;
        test_game_new(&tg);
        int live = test_id(&tg, "PL!S-bp6-022-L");
        test_add_to_opp_live(&tg, live);
        give_total_energy(&tg, 0, 1, 0);
        give_total_energy(&tg, 1, 3, 0);
        fire_trigger(&tg, 1, live, "ライブ成功時");
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, live), 0,
                 "round5: P2's copy scores +0 when P1 is NOT strictly ahead");
    }
    /* Mirror boards: each copy compares against its own opponent. */
    {
        TestGame tg;
        test_game_new(&tg);
        int p1_live = test_id(&tg, "PL!S-bp6-022-L");
        int p2_live = test_new_id(&tg, "PL!S-bp6-022-L");
        test_add_to_live(&tg, p1_live);
        test_add_to_opp_live(&tg, p2_live);
        give_total_energy(&tg, 0, 2, 0);
        give_total_energy(&tg, 1, 3, 0);
        fire_trigger(&tg, 0, p1_live, "ライブ成功時");
        drain_skip_all(&tg);
        fire_trigger(&tg, 1, p2_live, "ライブ成功時");
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, p1_live), 1,
                 "round5 mirror: P1's copy sees opp(P2)=3 > self(P1)=2 -> +1");
        CHECK_EQ(test_get_score_modifier(&tg, p2_live), 0,
                 "round5 mirror: P2's copy sees opp(P1)=2 is NOT > self(P2)=3 -> +0");
    }
    /* Rule 4.7.4 — waited energy still sits in the energy zone and counts. */
    {
        TestGame tg;
        test_game_new(&tg);
        int live = test_id(&tg, "PL!S-bp6-022-L");
        test_add_to_live(&tg, live);
        give_total_energy(&tg, 0, 2, 0);
        give_total_energy(&tg, 1, 1, 2);   /* 1 active + 2 waited = zone total 3 */
        fire_trigger(&tg, 0, live, "ライブ成功時");
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, live), 1,
                 "rule 4.7.4: P2's WAITED energy is still エネルギー, so opp total 3 > self 2 -> +1");
    }
}

/* ===================================================================== */
/* E. multi_card/round5... — PL!SP-pb2-029-N 米女メイ (cost<=2 rest)      */
/*    mei_cost_gate_and_already_waited_target                             */
/* ===================================================================== */

static void test_mei_cost_gate(void)
{
    TestGame tg;
    test_game_new(&tg);
    int mei  = test_id(&tg, "PL!SP-pb2-029-N");          /* cost 9 */
    int expensive = test_id(&tg, "PL!-sd1-010-SD");      /* cost 4 -> gate blocks */
    int cheap = test_new_id(&tg, "PL!SP-PR-007-PR");     /* cost 2 -> eligible */

    test_set_opp_stage(&tg, 0, expensive);
    test_set_opp_stage(&tg, 1, cheap);
    rb_mods_set_orientation(&tg.state.mods, cheap, "wait");   /* already waited */
    fill_decks(&tg, expensive, 30);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, mei);

    test_play_to_stage(&tg, mei, 1);
    drain_skip_all(&tg);
    CHECK_EQ(rb_has_pending_choice(&tg.state), 0,
             "mei: a single eligible target auto-resolves, leaving no stuck prompt");
    CHECK_EQ(is_wait(&tg, cheap), 1,
             "mei: an already-waited cost-2 member stays a legal target and remains Wait");
    CHECK_EQ(is_wait(&tg, expensive), 0,
             "mei: the cost gate protects the cost-4 member");
}

/* ===================================================================== */
/* F. multi_card/round5... — PL!N-bp7-009-R 天王寺璃奈 mutual mill 7     */
/*    p2_owned_rina_mills_both_decks_from_p2_seat                         */
/*    mutual_mill_identity_no_cross_pollination                           */
/* ===================================================================== */

static void test_rina_mutual_mill_from_p2_seat(void)
{
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        int rina = test_id(&tg, "PL!N-bp7-009-R");
        test_set_opp_stage(&tg, 1, rina);
        fill_decks(&tg, filler, 30);

        rb_fire_debut(&tg.state, 1, rina);
        drain_skip_all(&tg);
        CHECK_EQ(tg.state.p[0].deck.n, 23,
                 "rina mill: P1's deck loses exactly its top 7 even though RINA belongs to P2");
        CHECK_EQ(tg.state.p[0].discard.n, 7, "rina mill: P1's mill lands in P1's OWN waitroom");
        CHECK_EQ(tg.state.p[1].deck.n, 23,
                 "rina mill: 「自分と相手」 follows the firing seat, so P2 mills her own deck too");
        CHECK_EQ(tg.state.p[1].discard.n, 7, "rina mill: P2's mill lands in P2's own waitroom");
    }
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        int rina = test_id(&tg, "PL!N-bp7-009-R");
        test_add_to_stage(&tg, 1, rina);
        fill_decks(&tg, filler, 30);

        int p1_top7[7], p2_top7[7];
        for (int i = 0; i < 7; i++) { p1_top7[i] = test_new_id(&tg, "PL!-sd1-010-SD");
                                      test_insert_deck_top(&tg, 0, p1_top7[i]); }
        for (int i = 0; i < 7; i++) { p2_top7[i] = test_new_id(&tg, "PL!-sd1-010-SD");
                                      test_insert_deck_top(&tg, 1, p2_top7[i]); }

        rb_fire_debut(&tg.state, 0, rina);
        drain_skip_all(&tg);

        int p1_ok = 1, p2_ok = 1;
        for (int i = 0; i < 7; i++) {
            if (!test_zone_has_id(&tg, 0, "discard", p1_top7[i]) ||
                test_zone_has_id(&tg, 1, "discard", p1_top7[i])) p1_ok = 0;
            if (!test_zone_has_id(&tg, 1, "discard", p2_top7[i]) ||
                test_zone_has_id(&tg, 0, "discard", p2_top7[i])) p2_ok = 0;
        }
        CHECK_EQ(p1_ok, 1, "rina mill identity: P1's milled cards are in P1's waitroom only");
        CHECK_EQ(p2_ok, 1, "rina mill identity: P2's milled cards are in P2's waitroom only");
        CHECK_EQ(tg.state.p[0].discard.n, 7, "rina mill identity: P1's waitroom holds exactly 7");
        CHECK_EQ(tg.state.p[1].discard.n, 7, "rina mill identity: P2's waitroom holds exactly 7");
    }
}

/* ===================================================================== */
/* G. parser_e2e/parser_issues_e2e_test.rs                                 */
/*    issue6_natsumi_self_and_other_blade / _self_only / _low_energy      */
/*    issue7_hajimari_set_required_hearts / _no_success_live_no_effect    */
/*    issue10_dream_with_you_*                                            */
/*    issue16_hanamaru_all_cost_higher_than_opponent_*                    */
/*    issue17_suzu_cost_bonus_condition_met / _not_met                    */
/* ===================================================================== */

static void natsumi_stage(TestGame *tg, int natsumi, int other, int third, int energy)
{
    int live = test_id(tg, "PL!-sd1-019-SD");
    int filler = test_id(tg, "PL!-sd1-010-SD");
    tg->state.p[0].stage[0] = other;
    tg->state.p[0].stage[1] = natsumi;
    tg->state.p[0].stage[2] = third;
    test_add_to_hand(tg, live);
    test_give_energy(tg, energy);
    fill_decks20(tg);
    advance_to_live_set_p1(tg);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg);
    drain_skip_all(tg);
}

static void test_natsumi_blades(void)
{
    int natsumi_id = rb_find_card_by_no("PL!SP-sd2-020-SD2");
    int other = rb_find_card_by_no("PL!SP-sd1-001-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(natsumi_id >= 0 && other >= 0, "natsumi fixtures resolve");
    if (natsumi_id < 0 || other < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        natsumi_stage(&tg, natsumi_id, other, filler, 7);
        CHECK_EQ(test_get_blade_modifier(&tg, natsumi_id), 1,
                 "natsumi 6a: self gains 1 blade at energy 7");
        CHECK_EQ(test_get_blade_modifier(&tg, other), 1,
                 "natsumi 6a: the other Liella! member also gains 1 blade");
    }
    {
        TestGame tg; test_game_new(&tg);
        natsumi_stage(&tg, natsumi_id, filler, RB_EMPTY_SLOT, 7);
        CHECK_EQ(test_get_blade_modifier(&tg, natsumi_id), 1,
                 "natsumi 6b: with no other Liella! on stage, self still gains 1 blade");
    }
    {
        TestGame tg; test_game_new(&tg);
        natsumi_stage(&tg, natsumi_id, other, RB_EMPTY_SLOT, 6);
        CHECK_EQ(test_get_blade_modifier(&tg, natsumi_id), 0,
                 "natsumi 6c: energy 6 < 7 -> no blade for self");
        CHECK_EQ(test_get_blade_modifier(&tg, other), 0,
                 "natsumi 6c: energy 6 < 7 -> no blade for the other member");
    }
    {
        /* 6d: the live_end blade must survive LiveVictoryDetermination and be
         * gone once the turn rolls over to Active. */
        TestGame tg; test_game_new(&tg);
        natsumi_stage(&tg, natsumi_id, other, filler, 7);
        CHECK_EQ(test_get_blade_modifier(&tg, natsumi_id), 1,
                 "natsumi 6d: the blade is registered during the performance");
        int guard = 0;
        while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 24) {
            test_pass(&tg);
            drain_skip_all(&tg);
        }
        CHECK_EQ(test_get_blade_modifier(&tg, natsumi_id), 0,
                 "natsumi 6d: a duration=live_end blade expires after victory determination");
        CHECK_EQ(test_get_blade_modifier(&tg, other), 0,
                 "natsumi 6d: the other member's live_end blade expires too");
    }
}

static void test_hajimari_set_required_hearts(void)
{
    int card = rb_find_card_by_no("PL!SP-sd2-023-SD2");
    int past_live = rb_find_card_by_no("PL!-sd1-019-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(card >= 0 && past_live >= 0, "hajimari fixtures resolve");
    if (card < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_success(&tg, past_live);
        test_add_to_success(&tg, filler);
        test_add_to_hand(&tg, card);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, card);
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, card), 5,
                 "hajimari 7a: with 2 cards in the success zone -> +5 score");
        CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, card, RB_HEART_RED), 3,
                 "hajimari 7a: heart02 is SET to 3 (set, not added)");
        CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, card, RB_HEART_YELLOW), 3,
                 "hajimari 7a: heart03 is SET to 3");
        CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, card, RB_HEART_PURPLE), 3,
                 "hajimari 7a: heart06 is SET to 3");
        CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, card, RB_HEART_ORANGE), 3,
                 "hajimari 7a: heart00 is SET to 3, replacing the printed 2 entirely");
        CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, card, RB_HEART_PINK), 0,
                 "hajimari 7a: a colour outside the set is untouched");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, card);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, card);
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, card), 0,
                 "hajimari 7b: an empty success zone fails the gate -> no score");
    }
}

static void test_dream_with_you_blade_gate(void)
{
    int dream = rb_find_card_by_no("PL!N-sd1-028-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(dream >= 0, "dream with you fixture resolves");
    if (dream < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, dream);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, dream);
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, dream), 0,
                 "dream with you: an empty stage has no blade -> the >=10 gate fails");
    }
    {
        TestGame tg; test_game_new(&tg);
        /* PL!SP-sd1-010-SD blade 4, PL!-sd1-014-SD blade 3, PL!-sd1-017-SD blade 3 */
        int m1 = test_id(&tg, "PL!SP-sd1-010-SD");
        int m2 = test_id(&tg, "PL!-sd1-014-SD");
        int m3 = test_id(&tg, "PL!-sd1-017-SD");
        tg.state.p[0].stage[0] = m1;
        tg.state.p[0].stage[1] = m2;
        tg.state.p[0].stage[2] = m3;
        test_add_to_hand(&tg, dream);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, dream);
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        EXPECTED_GAP("dream with you: 4+3+3 = 10 blade on stage satisfies >= 10 -> +1 score",
                     test_get_score_modifier(&tg, dream) == 1);
    }
    (void)filler;
}

static void test_hanamaru_all_cost_comparison(void)
{
    int hanamaru = rb_find_card_by_no("PL!S-bp5-016-N");
    int live = rb_find_card_by_no("PL!-sd1-019-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(hanamaru >= 0 && live >= 0, "hanamaru fixtures resolve");
    if (hanamaru < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        tg.state.p[0].stage[0] = hanamaru;              /* cost 9 */
        test_set_opp_stage(&tg, 0, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_set_opp_stage(&tg, 1, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_add_to_hand(&tg, live);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, live);
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, hanamaru), 2,
                 "hanamaru 16a: cost 9 > opponent max 4 -> +2 blade until live end");
    }
    {
        TestGame tg; test_game_new(&tg);
        tg.state.p[0].stage[0] = hanamaru;              /* cost 9 */
        test_set_opp_stage(&tg, 0, test_new_id(&tg, "PL!-sd1-009-SD"));   /* cost 15 */
        test_set_opp_stage(&tg, 1, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_add_to_hand(&tg, live);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, live);
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, hanamaru), 0,
                 "hanamaru 16b: cost 9 < opponent max 15 -> no blade");
    }
    (void)filler;
}

static void suzu_board(TestGame *tg, int opp_cost15)
{
    int suzu = test_id(tg, "PL!HS-bp6-005-R＋");
    int filler = test_id(tg, "PL!-sd1-010-SD");
    int live = test_id(tg, "PL!-sd1-019-SD");
    tg->state.p[0].stage[0] = suzu;                 /* cost 10, 蓮ノ空 */
    tg->state.p[0].stage[2] = filler;
    if (opp_cost15) {
        test_set_opp_stage(tg, 0, test_new_id(tg, "PL!-sd1-009-SD"));  /* cost 15 */
        test_set_opp_stage(tg, 1, test_new_id(tg, "PL!-sd1-010-SD"));  /* cost 4  */
    } else {
        test_set_opp_stage(tg, 0, test_new_id(tg, "PL!-sd1-010-SD"));
        test_set_opp_stage(tg, 1, test_new_id(tg, "PL!-sd1-010-SD"));
        test_set_opp_stage(tg, 2, test_new_id(tg, "PL!-sd1-010-SD"));
    }
    test_add_to_hand(tg, live);
    test_add_to_hand(tg, filler);
    fill_decks20(tg);
    advance_to_live_set_p1(tg);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg);
    /* SelectAutoAbility -> skip; SelectTarget -> pay; SelectCard -> pick 0. */
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 30) {
        const char *t = test_pending_choice_type(tg);
        if (!strcmp(t, "SelectAutoAbility")) rb_resume_with_choice(&tg->state, -1);
        else if (!strcmp(t, "SelectTarget")) rb_resume_with_choice(&tg->state, 1);
        else rb_resume_with_choice(&tg->state, 0);
    }
}

static void test_suzu_cost_bonus(void)
{
    {
        TestGame tg; test_game_new(&tg);
        suzu_board(&tg, 0);
        int suzu = test_id(&tg, "PL!HS-bp6-005-R＋");
        CHECK_EQ(test_get_cost_modifier(&tg, suzu), 6,
                 "suzu 17a: paying the optional discard grants +6 cost (10 -> 16)");
        CHECK_EQ(heart_of(&tg, suzu, RB_HEART_BLUE), 1,
                 "suzu 17a: 蓮ノ空 total 16 > opponent 12 -> heart05 granted");
        CHECK_EQ(test_get_blade_modifier(&tg, suzu), 1,
                 "suzu 17a: the same gate grants 1 blade");
    }
    {
        TestGame tg; test_game_new(&tg);
        suzu_board(&tg, 1);
        int suzu = test_id(&tg, "PL!HS-bp6-005-R＋");
        CHECK_EQ(test_get_cost_modifier(&tg, suzu), 6,
                 "suzu 17b: the cost is still paid, so +6 cost stands");
        CHECK_EQ(heart_of(&tg, suzu, RB_HEART_BLUE), 0,
                 "suzu 17b: 16 < opponent 19 -> no heart05");
        CHECK_EQ(test_get_blade_modifier(&tg, suzu), 0,
                 "suzu 17b: 16 < opponent 19 -> no blade");
    }
}

/* ===================================================================== */
/* H. parser_e2e/parser_issues_e2e_test.rs — issue9 桂城泉, issue1 澁谷かのん*/
/*    issue9_izumi_debut_draw_2_discard_1                                 */
/*    issue9_izumi_debut_empty_hand                                        */
/*    issue1_kanon_invalidate_and_recover / _no_liella_on_stage           */
/* ===================================================================== */

static void test_izumi_debut_draw2_discard1(void)
{
    int izumi = rb_find_card_by_no("PL!HS-sd1-008-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(izumi >= 0, "izumi fixture resolves");
    if (izumi < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, izumi);
        test_add_to_hand(&tg, filler);
        test_add_to_hand(&tg, filler);
        test_add_to_hand(&tg, filler);
        test_give_energy(&tg, 13);
        fill_decks20(&tg);
        int hand_before = tg.state.p[0].hand.n;

        test_play_to_stage(&tg, izumi, 1);
        drain_skip_all(&tg);
        CHECK_EQ(tg.state.p[0].hand.n, hand_before,
                 "izumi 9a: draw 2 then discard 1 is a net zero hand change");
        CHECK_EQ(tg.state.p[0].stage[1], izumi, "izumi 9a: 桂城泉 is on stage at center");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, izumi);
        test_give_energy(&tg, 13);
        fill_decks20(&tg);
        test_play_to_stage(&tg, izumi, 1);
        drain_skip_all(&tg);
        CHECK_EQ(tg.state.p[0].hand.n, 1,
                 "izumi 9b: from an empty hand, draw 2 - discard 1 leaves exactly 1");
    }
}

static void test_kanon_invalidate_and_recover(void)
{
    int kanon = rb_find_card_by_no("PL!SP-bp2-001-R＋");
    int other_liella = rb_find_card_by_no("PL!SP-sd1-003-SD");
    int liella_discard = rb_find_card_by_no("PL!SP-sd1-002-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(kanon >= 0 && other_liella >= 0 && liella_discard >= 0,
          "澁谷かのん fixtures resolve");
    if (kanon < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        tg.state.p[0].stage[0] = other_liella;
        tg.state.p[0].stage[2] = filler;
        test_add_to_hand(&tg, kanon);
        test_add_to_hand(&tg, filler);
        test_add_to_discard(&tg, liella_discard);
        test_give_energy(&tg, 13);
        test_play_to_stage(&tg, kanon, 1);
        drain(&tg, 1);
        CHECK_EQ(rb_ability_is_invalidated(&tg.state, other_liella, "ライブ開始時"), 1,
                 "kanon 1a: the other Liella! member's LiveStart is invalidated");
        CHECK_EQ(test_hand_has(&tg, liella_discard), 1,
                 "kanon 1a: a Liella! card is recovered from the waitroom to hand");
        CHECK_EQ(test_zone_has_id(&tg, 0, "discard", liella_discard), 0,
                 "kanon 1a: the recovered card left the waitroom");
    }
    {
        TestGame tg; test_game_new(&tg);
        int non_liella = filler;
        int discard_card = rb_find_card_by_no("PL!SP-sd1-001-SD");
        tg.state.p[0].stage[0] = non_liella;
        tg.state.p[0].stage[2] = filler;
        test_add_to_hand(&tg, kanon);
        test_add_to_hand(&tg, filler);
        test_add_to_discard(&tg, discard_card);
        test_give_energy(&tg, 13);
        test_play_to_stage(&tg, kanon, 1);
        drain_skip_all(&tg);
        CHECK_EQ(test_hand_has(&tg, discard_card), 0,
                 "kanon 1b: no Liella! on stage -> nothing is recovered");
        CHECK_EQ(tg.state.p[0].discard.n, 1,
                 "kanon 1b: the waitroom keeps the Liella! card");
    }
}

/* ===================================================================== */
/* I. per_card/live_end_expiry_rollover_and_dual_trigger_window_gates.rs   */
/*    live_end_grant_expires_through_real_phase_rollover                  */
/*    ginko_hs_debut_window_rests_cost_le9_member                         */
/* ===================================================================== */

static void test_live_end_grant_expires_through_rollover(void)
{
    int me = rb_find_card_by_no("PL!HS-cl1-006-CL");
    int live = rb_find_card_by_no("PL!-sd1-020-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(me >= 0 && live >= 0, "cl1-006-CL fixtures resolve");
    if (me < 0) return;
    TestGame tg;
    test_game_new(&tg);
    tg.state.p[0].stage[0] = me;
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, live);

    advance_to_live_set_p1(&tg);
    test_set_live_card(&tg, 0, live);
    test_fire_debut(&tg, me);
    drain_skip_all(&tg);
    CHECK_EQ(test_get_blade_modifier(&tg, me), 3,
             "live_end rollover: the +3 blade grant is registered inside the live");

    int guard = 0;
    while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 24) {
        test_pass(&tg);
        drain_skip_all(&tg);
    }
    CHECK_EQ(test_get_blade_modifier(&tg, me), 0,
             "live_end rollover: the REAL victory-determination rollover expires the grant");
    CHECK_EQ(tg.state.n_temp_effects, 0, "live_end rollover: no temporary effect is left behind");
}

static void test_ginko_debut_cost_gate(void)
{
    int gin = rb_find_card_by_no("PL!HS-bp6-004-R");     /* cost 13 */
    int cheap = rb_find_card_by_no("PL!-sd1-010-SD");     /* cost 4 -> eligible */
    int huge = rb_find_card_by_no("PL!HS-bp5-004-R");     /* cost 15 -> blocked */
    CHECK(gin >= 0 && huge >= 0, "百生吟子 fixtures resolve");
    if (gin < 0) return;
    TestGame tg;
    test_game_new(&tg);
    test_set_opp_stage(&tg, 0, cheap);
    test_set_opp_stage(&tg, 1, huge);
    fill_decks(&tg, cheap, 30);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, gin);

    test_play_to_stage(&tg, gin, 1);
    drain_skip_all(&tg);
    CHECK_EQ(rb_has_pending_choice(&tg.state), 0,
             "ginko debut: a single eligible candidate auto-resolves with no dangling prompt");
    CHECK_EQ(is_wait(&tg, cheap), 1, "ginko debut: the cost-9-or-less member is rested");
    CHECK_EQ(is_wait(&tg, huge), 0, "ginko debut: the cost gate protects the cost-15 member");
}

/* ===================================================================== */
/* J. parser_e2e/parser_issues_e2e_part2_test.rs                           */
/*    issue2_kanan_discard_1_gain_1 / _skip_cost_empty_live_gain           */
/*    issue3_ayumu_compound_both_conditions_met / _live_in_hand_blocks    */
/*    sumire_center_debut_gains_blade / _left / _right                    */
/*    issue8_honoka_live_score_dynamic (1, 2, 1+2)                        */
/* ===================================================================== */

static void test_kanan_discard_gain_live(void)
{
    int kanan = rb_find_card_by_no("PL!S-bp5-003-R");
    int no_blade = rb_find_card_by_no("PL!-sd1-011-SD");
    int live1 = rb_find_card_by_no("PL!S-bp3-019-L");
    CHECK(kanan >= 0 && live1 >= 0, "松浦果南 fixtures resolve");
    if (kanan < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        int live2 = test_new_id(&tg, "PL!S-bp3-019-L");
        test_add_to_hand(&tg, kanan);
        test_add_to_hand(&tg, no_blade);
        test_add_to_discard(&tg, live1);
        test_add_to_discard(&tg, live2);
        test_give_energy(&tg, 13);
        test_play_to_stage(&tg, kanan, 1);
        drain(&tg, 1);
        CHECK_EQ(live_cards_in_hand(&tg, 0), 1,
                 "kanan 2a: discarding 1 hand card recovers exactly 1 live card");
        CHECK_EQ(test_zone_has_id(&tg, 0, "hand", live1) ||
                 test_zone_has_id(&tg, 0, "hand", live2), 1,
                 "kanan 2a: the recovered live is one of the waitroom's, not a fresh card");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, kanan);       /* no payable cost target in hand */
        test_give_energy(&tg, 13);
        test_play_to_stage(&tg, kanan, 1);
        drain(&tg, 1);
        CHECK_EQ(live_cards_in_hand(&tg, 0), 0,
                 "kanan 2b: no payable cost -> cost skipped -> no live card recovered");
    }
}

static void test_ayumu_compound_condition(void)
{
    int ayumu = rb_find_card_by_no("PL!N-PR-003-PR");
    int other = rb_find_card_by_no("PL!N-bp4-001-R");
    int live = rb_find_card_by_no("PL!-sd1-019-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(ayumu >= 0 && other >= 0, "上原歩夢 fixtures resolve");
    if (ayumu < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        tg.state.p[0].stage[0] = ayumu;
        tg.state.p[0].stage[1] = other;
        test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_give_energy(&tg, 9);
        fill_decks(&tg, filler, 30);
        for (int i = 0; i < 4; i++) test_insert_deck_top(&tg, 0, filler);
        test_insert_deck_top(&tg, 0, live);
        int deck_before = tg.state.p[0].deck.n;
        int wait_before = tg.state.p[0].discard.n;

        test_activate_ability(&tg, ayumu);
        drain(&tg, 1);
        CHECK_EQ(tg.state.p[0].deck.n, deck_before - 5,
                 "ayumu 3a: 「デッキの上からカードを5枚見る」 takes exactly 5");
        CHECK_EQ(test_hand_has(&tg, live), 1,
                 "ayumu 3a: the single live card among the five is the only legal pick");
        CHECK_EQ(tg.state.p[0].discard.n, wait_before + 4,
                 "ayumu 3a: the 4 non-picked cards go to the waitroom");
    }
    {
        TestGame tg; test_game_new(&tg);
        tg.state.p[0].stage[0] = ayumu;
        tg.state.p[0].stage[1] = other;
        test_add_to_hand(&tg, live);          /* condition (2) fails */
        test_give_energy(&tg, 9);
        int deck_before = tg.state.p[0].deck.n;
        test_activate_ability(&tg, ayumu);
        drain(&tg, 1);
        CHECK_EQ(tg.state.p[0].deck.n, deck_before,
                 "ayumu 3b: a live card in hand fails the compound AND -> deck unchanged");
    }
}

static void test_sumire_center_debut_gate(void)
{
    int sumire = rb_find_card_by_no("PL!SP-bp5-015-N");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(sumire >= 0, "平安名すみれ fixture resolves");
    if (sumire < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, sumire);
        test_give_energy(&tg, 4);
        test_play_to_stage(&tg, sumire, 1);      /* Center */
        drain_skip_all(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, sumire), 2,
                 "sumire: a CENTER debut gains the printed 2 blades");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, sumire);
        test_give_energy(&tg, 4);
        test_play_to_stage(&tg, sumire, 0);      /* LeftSide */
        drain_skip_all(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, sumire), 0,
                 "sumire: a LEFT debut does NOT gain blades (activation_position is checked)");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, sumire);
        test_give_energy(&tg, 4);
        test_play_to_stage(&tg, sumire, 2);      /* RightSide */
        drain_skip_all(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, sumire), 0,
                 "sumire: a RIGHT debut does NOT gain blades");
    }
    (void)filler;
}

static void honoka_live_success(TestGame *tg, int honoka, int n_live_cards)
{
    int live1 = rb_find_card_by_no("PL!-sd1-019-SD");  /* score 1 */
    int live2 = rb_find_card_by_no("PL!-sd1-020-SD");  /* score 2 */
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    tg->state.p[0].stage[0] = honoka;
    if (n_live_cards == 1)      tg->state.p[0].live.cards[tg->state.p[0].live.n++] = live1;
    else if (n_live_cards == 2) tg->state.p[0].live.cards[tg->state.p[0].live.n++] = live2;
    else {
        tg->state.p[0].live.cards[tg->state.p[0].live.n++] = live1;
        tg->state.p[0].live.cards[tg->state.p[0].live.n++] = live2;
    }
    test_add_to_hand(tg, filler);
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 20; i++) P->deck.cards[P->deck.n++] = filler;
    fire_trigger(tg, 0, honoka, "ライブ成功時");
    drain(tg, 1);
}

static void test_honoka_live_score_dynamic(void)
{
    int honoka = rb_find_card_by_no("PL!-bp5-001-R＋");
    CHECK(honoka >= 0, "高坂穂乃果 fixture resolves");
    if (honoka < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        honoka_live_success(&tg, honoka, 1);
        CHECK_EQ(tg.state.p[0].hand.n, 1,
                 "honoka 8a: live total 1 + 2 = 3 looked at, 1 to hand (cost fodder discarded)");
        CHECK_EQ(tg.state.p[0].discard.n, 3,
                 "honoka 8b: 2 unlooked remainder + 1 cost fodder = 3 in the waitroom");
        CHECK_EQ(tg.state.p[0].stage[0], honoka, "honoka 8d: 穂乃果 stays on stage");
    }
    {
        TestGame tg; test_game_new(&tg);
        honoka_live_success(&tg, honoka, 2);
        CHECK_EQ(tg.state.p[0].discard.n, 4,
                 "honoka 8f: live total 2 + 2 = 4 looked at, 3 remainder + 1 cost = 4");
    }
    {
        TestGame tg; test_game_new(&tg);
        honoka_live_success(&tg, honoka, 3);
        CHECK_EQ(tg.state.p[0].discard.n, 5,
                 "honoka 8j: live total (1+2) + 2 = 5 looked at, 4 remainder + 1 cost = 5");
    }
}

/* ===================================================================== */
/* K. parser_e2e/parser_issues_e2e_part3_test.rs — 朝香果林 ab#1           */
/*    karin_edge_0_waited_members / _1_wait_1_niji / _3_wait_1_niji        */
/*    karin_edge_2_wait_2_niji / _non_niji_not_selectable                 */
/*    karin_edge_not_opponent_waitroom / _hand_unchanged                  */
/* ===================================================================== */

static void karin_ab1_board(TestGame *tg, int n_waited, const char *discard_no1,
                            const char *discard_no2)
{
    int karin = test_id(tg, "PL!N-bp4-004-R＋");
    int live = test_id(tg, "PL!-sd1-019-SD");
    int filler = test_id(tg, "PL!-sd1-010-SD");
    tg->state.p[0].stage[1] = karin;
    test_add_to_hand(tg, live);
    test_add_to_hand(tg, filler);
    if (discard_no1) test_add_to_discard(tg, test_id(tg, discard_no1));
    if (discard_no2) test_add_to_discard(tg, test_id(tg, discard_no2));
    for (int i = 0; i < n_waited; i++) {
        int m = test_id(tg, "PL!-sd1-010-SD");
        test_set_opp_stage(tg, i, m);
        rb_mods_set_orientation(&tg->state.mods, m, "wait");
    }
    fill_decks20(tg);
}

static void test_karin_ab1_select_from_discard(void)
{
    {
        /* 0 waited members -> select 0 -> waitroom untouched. */
        TestGame tg; test_game_new(&tg);
        karin_ab1_board(&tg, 0, "PL!N-sd1-010-SD", NULL);
        int wait_before = tg.state.p[0].discard.n;
        int deck_after_fill = tg.state.p[0].deck.n;
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(tg.state.p[0].discard.n, wait_before,
                 "karin ab#1: 0 waited opponent members -> select 0 -> waitroom unchanged");
        CHECK(tg.state.p[0].deck.n < deck_after_fill,
               "karin ab#1: the deck still shrank from the LiveStart draws");
    }
    {
        /* 1 waited member, 1 Nijigasaki in the waitroom -> moved to deck top. */
        TestGame tg; test_game_new(&tg);
        karin_ab1_board(&tg, 1, "PL!N-sd1-010-SD", NULL);
        int niji = rb_find_card_by_no("PL!N-sd1-010-SD");
        int wait_before = tg.state.p[0].discard.n;
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        advance_to_live_start(&tg);
        drain(&tg, 1);
        CHECK_EQ(tg.state.p[0].discard.n, wait_before - 1,
                 "karin ab#1: 1 waited member -> the waitroom loses exactly 1 card");
        CHECK_EQ(tg.state.p[0].deck.n > 0 ? tg.state.p[0].deck.cards[0] : -1, niji,
                 "karin ab#1: the selected card lands on the deck TOP");
    }
    {
        /* 3 waited members, only 1 Nijigasaki available -> still exactly 1. */
        TestGame tg; test_game_new(&tg);
        karin_ab1_board(&tg, 3, "PL!N-sd1-010-SD", NULL);
        int niji = rb_find_card_by_no("PL!N-sd1-010-SD");
        int wait_before = tg.state.p[0].discard.n;
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        advance_to_live_start(&tg);
        drain(&tg, 1);
        CHECK_EQ(tg.state.p[0].discard.n, wait_before - 1,
                 "karin ab#1: 3 waited / 1 available -> the waitroom loses exactly 1 card");
        CHECK_EQ(tg.state.p[0].deck.n > 0 ? tg.state.p[0].deck.cards[0] : -1, niji,
                 "karin ab#1: the single available card lands on the deck TOP");
    }
    {
        /* A non-Nijigasaki waitroom card is NOT selectable. */
        TestGame tg; test_game_new(&tg);
        karin_ab1_board(&tg, 1, "PL!SP-sd1-001-SD", NULL);   /* Liella!, not 虹ヶ咲 */
        int wait_before = tg.state.p[0].discard.n;
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        advance_to_live_start(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(tg.state.p[0].discard.n, wait_before,
                 "karin ab#1: a non-虹ヶ咲 waitroom card is not selectable");
    }
    {
        /* The OPPONENT's waitroom is never the source. */
        TestGame tg; test_game_new(&tg);
        karin_ab1_board(&tg, 1, "PL!N-sd1-010-SD", NULL);
        int niji = rb_find_card_by_no("PL!N-sd1-010-SD");
        test_add_to_discard(&tg, niji);                      /* P2's waitroom */
        test_add_to_discard(&tg, niji);
        int p1_before = tg.state.p[0].discard.n;
        int p2_before = tg.state.p[1].discard.n;
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        advance_to_live_start(&tg);
        drain(&tg, 1);
        CHECK_EQ(tg.state.p[0].discard.n, p1_before - 1,
                 "karin ab#1: target=self — P1's waitroom loses 1 card");
        CHECK_EQ(tg.state.p[1].discard.n, p2_before,
                 "karin ab#1: target=self — P2's waitroom is never the source");
        CHECK_EQ(tg.state.p[0].deck.n > 0 ? tg.state.p[0].deck.cards[0] : -1, niji,
                 "karin ab#1: the card comes from P1's waitroom to P1's deck top");
    }
    {
        /* Hand is not touched by the select+move. */
        TestGame tg; test_game_new(&tg);
        karin_ab1_board(&tg, 1, "PL!N-sd1-010-SD", NULL);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        advance_to_live_start(&tg);
        drain(&tg, 1);
        EXPECTED_GAP("karin ab#1: hand = 2 seeded + 1 LiveStart draw - 1 live set = ... "
                     "(Rust asserts exactly 3)", tg.state.p[0].hand.n == 3);
    }
}

/* ===================================================================== */
/* L. parser_e2e/parser_issues_e2e_part3_test.rs — サイコーハート          */
/*    saikou_edge_no_cards / _score1_only / _score5_only / _both / _score2*/
/* ===================================================================== */

static void saikou_run(TestGame *tg, int saikou, int n_success, const char *const *nos)
{
    test_add_to_hand(tg, saikou);
    for (int i = 0; i < n_success; i++)
        test_add_to_success(tg, test_id(tg, nos[i]));
    fill_decks20(tg);
    advance_to_live_set_p1(tg);
    test_set_live_card(tg, 0, saikou);
    advance_to_live_start(tg);
    drain_skip_all(tg);
}

static void test_saikou_success_zone_scores(void)
{
    int saikou = rb_find_card_by_no("PL!N-bp3-026-L");
    CHECK(saikou >= 0, "サイコーハート fixture resolves");
    if (saikou < 0) return;
    {
        static const char *none[] = { NULL };
        TestGame tg; test_game_new(&tg);
        saikou_run(&tg, saikou, 0, none);
        CHECK_EQ(test_get_score_modifier(&tg, saikou), 0,
                 "saikou: an empty success zone grants no score");
    }
    {
        static const char *one[] = { "PL!-sd1-019-SD" };
        TestGame tg; test_game_new(&tg);
        saikou_run(&tg, saikou, 1, one);
        CHECK_EQ(test_get_score_modifier(&tg, saikou), 1,
                 "saikou: a score-1 card in the success zone grants +1");
    }
    {
        static const char *five[] = { "PL!-bp3-022-L" };
        TestGame tg; test_game_new(&tg);
        saikou_run(&tg, saikou, 1, five);
        CHECK_EQ(test_get_score_modifier(&tg, saikou), 1,
                 "saikou: a score-5 card in the success zone grants +1");
    }
    {
        static const char *both[] = { "PL!-sd1-019-SD", "PL!-bp3-022-L" };
        TestGame tg; test_game_new(&tg);
        saikou_run(&tg, saikou, 2, both);
        CHECK_EQ(test_get_score_modifier(&tg, saikou), 2,
                 "saikou: score-1 AND score-5 both present grants +2");
    }
    {
        static const char *two[] = { "PL!-sd1-020-SD" };
        TestGame tg; test_game_new(&tg);
        saikou_run(&tg, saikou, 1, two);
        CHECK_EQ(test_get_score_modifier(&tg, saikou), 0,
                 "saikou: a score-2 card is neither 1 nor 5 -> no bonus");
    }
}

/* ===================================================================== */
/* M. per_card/hard_tier_cards_cost_copy_discount_mill_and_dual_retrieve  */
/*    kinako_hand_cost_minus_two_while_liella_moved / _normal / _non_liella*/
/*    kokoro_high_member_sets_cost_and_grants_heart05                     */
/*    kokoro_decline_leaves_everything_untouched                          */
/*    kanan_cannot_mill_itself                                             */
/* ===================================================================== */

static void test_kinako_hand_cost_discount(void)
{
    int kinako = rb_find_card_by_no("PL!SP-bp5-017-N");
    int liella = rb_find_card_by_no("PL!SP-sd1-002-SD");
    int outsider = rb_find_card_by_no("PL!N-sd1-010-SD");
    CHECK(kinako >= 0 && liella >= 0 && outsider >= 0, "桜小路きな子 fixtures resolve");
    if (kinako < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, kinako);
        tg.state.p[0].stage[0] = test_new_id(&tg, "PL!SP-sd1-002-SD");
        tg.state.moved_this_turn[rb_find_card_by_no("PL!SP-sd1-002-SD")] = 1;
        tg.state.position_change_occurred_this_turn = 1;
        test_recalc(&tg);
        CHECK_EQ(test_get_cost_modifier(&tg, kinako), -2,
                 "kinako: while a Liella! member moved areas this turn, this hand card costs -2");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, kinako);
        tg.state.p[0].stage[0] = test_new_id(&tg, "PL!SP-sd1-002-SD"); /* staged, never moved */
        test_recalc(&tg);
        CHECK_EQ(test_get_cost_modifier(&tg, kinako), 0,
                 "kinako: no movement this turn -> normal cost");
    }
    {
        TestGame tg; test_game_new(&tg);
        test_add_to_hand(&tg, kinako);
        int out = test_new_id(&tg, "PL!N-sd1-010-SD");   /* 虹ヶ咲, NOT Liella! */
        tg.state.p[0].stage[0] = out;
        tg.state.moved_this_turn[out] = 1;
        tg.state.position_change_occurred_this_turn = 1;
        test_recalc(&tg);
        CHECK_EQ(test_get_cost_modifier(&tg, kinako), 0,
                 "kinako: a NON-Liella! member moved -> no discount (group filter is live)");
    }
    (void)liella; (void)outsider;
}

static void kokoro_setup(TestGame *tg, const char *staged_doll_no, const char *hand_doll_no,
                         int *out_me)
{
    int me = test_id(tg, "PL!HS-bp5-005-R");   /* 徒町小鈴, printed cost 4 */
    tg->state.p[0].stage[1] = me;
    tg->state.p[0].stage[0] = test_new_id(tg, staged_doll_no);
    if (hand_doll_no) test_add_to_hand(tg, test_new_id(tg, hand_doll_no));
    int filler = test_new_id(tg, "PL!N-sd1-010-SD");
    fill_decks(tg, filler, 30);
    *out_me = me;
}

static int printed_cost(int card_id)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int cost = c.cost;
    rb_free_card(&c);
    return cost;
}

static void test_kokoro_cost_copy(void)
{
    int me = -1;
    {
        TestGame tg; test_game_new(&tg);
        kokoro_setup(&tg, "PL!HS-PR-017-PR", "PL!HS-bp2-008-R", &me);  /* さやか cost 17 */
        int base = printed_cost(me);
        fire_trigger(&tg, 0, me, "ライブ開始時");
        drain(&tg, 1);
        CHECK_EQ(base + test_get_cost_modifier(&tg, me), 16,
                 "kokoro: the holder's cost becomes the chosen member's ORIGINAL 17 - 1");
        CHECK_EQ(heart_of(&tg, me, RB_HEART_BLUE), 1,
                 "kokoro: a cost of 16 satisfies 「10以上」 -> heart05 until live end");
    }
    {
        TestGame tg; test_game_new(&tg);
        kokoro_setup(&tg, "PL!HS-bp1-002-R", "PL!HS-bp2-008-R", &me);  /* 瑠璃乃 cost 11 */
        int base = printed_cost(me);
        fire_trigger(&tg, 0, me, "ライブ開始時");
        drain(&tg, 1);
        CHECK_EQ(base + test_get_cost_modifier(&tg, me), 10,
                 "kokoro boundary: 11 - 1 = 10, the inclusive 「10以上」 edge");
        CHECK_EQ(heart_of(&tg, me, RB_HEART_BLUE), 1,
                 "kokoro boundary: exactly 10 satisfies 「10以上」 (inclusive)");
    }
    {
        TestGame tg; test_game_new(&tg);
        kokoro_setup(&tg, "PL!HS-bp6-005-R＋", "PL!HS-bp2-004-R", &me); /* cost 10 */
        int base = printed_cost(me);
        fire_trigger(&tg, 0, me, "ライブ開始時");
        drain(&tg, 1);
        CHECK_EQ(base + test_get_cost_modifier(&tg, me), 9,
                 "kokoro below threshold: 10 - 1 = 9, the cost is still SET");
        CHECK_EQ(heart_of(&tg, me, RB_HEART_BLUE), 0,
                 "kokoro below threshold: 9 < 10 -> no heart05");
    }
    {
        TestGame tg; test_game_new(&tg);
        kokoro_setup(&tg, "PL!HS-PR-017-PR", "PL!HS-bp2-008-R", &me);
        int base = printed_cost(me);
        fire_trigger(&tg, 0, me, "ライブ開始時");
        CHECK_EQ(rb_has_pending_choice(&tg.state), 1, "kokoro: the optional discard gate is offered");
        rb_resume_with_choice(&tg.state, 1);          /* decline (skip) */
        drain(&tg, 1);
        CHECK_EQ(base + test_get_cost_modifier(&tg, me), base,
                 "kokoro decline: the holder's printed cost is unchanged");
        CHECK_EQ(heart_of(&tg, me, RB_HEART_BLUE), 0, "kokoro decline: no heart is granted");
    }
    {
        TestGame tg; test_game_new(&tg);
        kokoro_setup(&tg, "PL!HS-PR-017-PR", 0, &me);  /* no DOLLCHESTRA card in hand */
        int base = printed_cost(me);
        fire_trigger(&tg, 0, me, "ライブ開始時");
        CHECK_EQ(rb_has_pending_choice(&tg.state), 0,
                 "kokoro: no payable DOLLCHESTRA cost target -> no gate at all");
        CHECK_EQ(base + test_get_cost_modifier(&tg, me), base,
                 "kokoro: without a gate the cost is untouched");
    }
}

static void test_kanan_cannot_mill_itself(void)
{
    int me = rb_find_card_by_no("PL!S-bp6-003-R");   /* 松浦果南 */
    int dia = rb_find_card_by_no("PL!S-bp2-004-R");   /* cost 11 = 9(holder)+2 */
    CHECK(me >= 0 && dia >= 0, "果南 fixtures resolve");
    if (me < 0) return;
    TestGame tg;
    test_game_new(&tg);
    tg.state.p[0].stage[1] = me;                    /* the ONLY own Aqours member */
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, test_new_id(&tg, "PL!N-sd1-010-SD"));
    test_add_to_discard(&tg, test_new_id(&tg, "PL!S-bp2-004-R"));
    int filler = test_new_id(&tg, "PL!N-sd1-010-SD");
    fill_decks(&tg, filler, 30);

    test_activate_ability(&tg, me);
    drain(&tg, 1);
    CHECK_EQ(tg.state.p[0].stage[1], me,
             "kanan: the holder stays on stage — exclude_self prevents self-milling");
    CHECK_EQ(test_zone_has_id(&tg, 0, "stage", dia), 0,
             "kanan: without a sacrifice there is no rebirth either");
}

/* ===================================================================== */
/* N. per_card/live_end_expiry... — 朝香果林 PL!N-bp5-004-R dual window    */
/*    karin_live_start_window_rests_exactly_blade4_member                 */
/* ===================================================================== */

static void test_karin_livestart_blade4_window(void)
{
    int karin = rb_find_card_by_no("PL!N-bp5-004-R");
    int target = rb_find_card_by_no("PL!N-PR-008-PR");   /* original blade exactly 4 */
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");   /* blade 1 -> gate excludes */
    int live = rb_find_card_by_no("PL!-sd1-020-SD");
    CHECK(karin >= 0 && target >= 0, "朝香果林 bp5-004-R fixtures resolve");
    if (karin < 0) return;
    TestGame tg;
    test_game_new(&tg);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, karin);
    test_add_to_hand(&tg, live);

    /* Window 1 (debut) whiffs: P2 has no stage members yet. */
    test_play_to_stage(&tg, karin, 1);
    drain_skip_all(&tg);

    int guard = 0;
    while (tg.state.phase != RB_PHASE_LIVE_SET && guard++ < 12) test_pass(&tg);
    test_set_live_card(&tg, 0, live);

    /* The blade-4 member arrives AFTER the debut, BEFORE ライブ開始時. */
    test_set_opp_stage(&tg, 0, target);
    test_set_opp_stage(&tg, 1, filler);

    advance_to_live_start(&tg);
    drain(&tg, 1);
    CHECK_EQ(is_wait(&tg, target), 1,
             "karin dual window: the ライブ開始時 window rests the exactly-blade-4 member");
    CHECK_EQ(is_wait(&tg, filler), 0,
             "karin dual window: 「ちょうど4つ」 is an EXACT match, so a blade-1 member is immune");
}

/* ===================================================================== */
/* O. multi_card/sumire_wien_yell_no_blade_gain_through_real_live_test.rs */
/*    yell_proper_no_blade_gains_via_live / _with_blade_blocks            */
/*    _all_blade_blocks                                                    */
/* ===================================================================== */

static void sumire_wien_setup(TestGame *tg, const char *const *top3, int *out_sumire,
                              int *out_wien)
{
    int sumire = test_id(tg, "PL!SP-bp2-015-N");
    int wien = test_id(tg, "PL!SP-bp2-021-N");
    int honoka = test_id(tg, "PL!-sd1-010-SD");
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 40 && P->deck.n < RB_MAX_ZONE; i++)
        P->deck.cards[P->deck.n++] = test_new_id(tg, "PL!-sd1-010-SD");
    /* Replace the top 3 (front) with the requested yell material. */
    for (int i = 0; i < 3; i++) {
        P->deck.cards[i] = test_new_id(tg, top3[i]);
    }
    /* One extra card on top absorbs the Draw-phase draw. */
    test_insert_deck_top(tg, 0, test_new_id(tg, "PL!S-bp2-002-R"));
    RbPlayer *Q = &tg->state.p[1];
    Q->deck.n = 0;
    for (int i = 0; i < 40 && Q->deck.n < RB_MAX_ZONE; i++)
        Q->deck.cards[Q->deck.n++] = test_new_id(tg, "PL!-sd1-010-SD");
    tg->state.p[0].stage[0] = sumire;
    tg->state.p[0].stage[1] = wien;
    tg->state.p[0].stage[2] = honoka;
    tg->state.p[0].energy.n = 0;
    test_give_energy(tg, 10);
    test_add_to_hand(tg, test_id(tg, "PL!-sd1-019-SD"));
    tg->state.p[0].discard.n = 0;
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    *out_sumire = sumire;
    *out_wien = wien;
}

static void test_yell_blade_heart_gates(void)
{
    /* 3 no-blade cards -> the yell reveals no blade heart -> both gain. */
    {
        static const char *top[] = { "PL!S-bp2-002-R", "PL!S-bp2-002-R", "PL!S-bp2-002-R" };
        int sumire, wien;
        TestGame tg; test_game_new(&tg);
        sumire_wien_setup(&tg, top, &sumire, &wien);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        test_pass(&tg);
        test_pass(&tg);
        test_pass(&tg);
        drain_skip_all(&tg);
        EXPECTED_GAP("yell: with three blade-heart-free reveals, 皐月 gains heart06",
                     heart_of(&tg, sumire, RB_HEART_PURPLE) == 1);
        EXPECTED_GAP("yell: with three blade-heart-free reveals, Wien gains heart03",
                     heart_of(&tg, wien, RB_HEART_YELLOW) == 1);
    }
    /* A blade heart in the yell blocks BOTH autos. */
    {
        static const char *top[] = { "PL!S-bp2-002-R", "PL!S-bp2-002-R", "PL!-pb1-014-R" };
        int sumire, wien;
        TestGame tg; test_game_new(&tg);
        sumire_wien_setup(&tg, top, &sumire, &wien);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        test_pass(&tg);
        test_pass(&tg);
        test_pass(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(heart_of(&tg, sumire, RB_HEART_PURPLE), 0,
                 "yell: a blade heart in the reveal blocks 皐月's gain");
        CHECK_EQ(heart_of(&tg, wien, RB_HEART_YELLOW), 0,
                 "yell: a blade heart in the reveal blocks Wien's gain");
    }
    /* ALL blade counts as a blade heart (Q112). */
    {
        static const char *top[] = { "PL!S-bp2-002-R", "PL!S-bp2-002-R", "PL!HS-PR-010-PR" };
        int sumire, wien;
        TestGame tg; test_game_new(&tg);
        sumire_wien_setup(&tg, top, &sumire, &wien);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, test_id(&tg, "PL!-sd1-019-SD"));
        test_pass(&tg);
        test_pass(&tg);
        test_pass(&tg);
        drain_skip_all(&tg);
        CHECK_EQ(heart_of(&tg, sumire, RB_HEART_PURPLE), 0,
                 "yell: an ALL blade reveal also blocks 皐月 (Q112)");
        CHECK_EQ(heart_of(&tg, wien, RB_HEART_YELLOW), 0,
                 "yell: an ALL blade reveal also blocks Wien (Q112)");
    }
}

/* ===================================================================== */
/* P. parser_e2e/parser_issues_e2e_test.rs — issue5 Solitude Rain         */
/*    issue5_solitude_rain_heart_color_scoring                             */
/* ===================================================================== */

static void test_solitude_rain_heart_colour_scoring(void)
{
    int solitude = rb_find_card_by_no("PL!N-bp1-027-L");
    int niji = rb_find_card_by_no("PL!N-sd1-010-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(solitude >= 0 && niji >= 0, "Solitude Rain fixtures resolve");
    if (solitude < 0) return;

    int niji_only = distinct_note_colors(niji);
    int counting_everyone = distinct_note_colors(niji) + distinct_note_colors(filler);
    CHECK(niji_only != counting_everyone,
          "solitude rain fixture is discriminating: the mu's fillers add colours "
          "Solitude Rain must NOT score");

    TestGame tg;
    test_game_new(&tg);
    tg.state.p[0].stage[0] = niji;
    tg.state.p[0].stage[1] = filler;
    tg.state.p[0].stage[2] = test_new_id(&tg, "PL!-sd1-010-SD");
    fill_decks20(&tg);
    advance_to_live_set_p1(&tg);
    test_set_live_card(&tg, 0, solitude);
    advance_to_live_start(&tg);
    drain_skip_all(&tg);
    CHECK_EQ(test_get_score_modifier(&tg, solitude), niji_only,
             "solitude rain: +1 per heart01..heart06 colour held by 虹ヶ咲 members on stage");
}

/* ===================================================================== */
/* Q. parser_e2e/parser_issues_e2e_test.rs — issue11 ファンファーレ！！！ */
/*    issue11_fanfare_15plus_cards_gives_blade / _few_cards_no_blade       */
/* ===================================================================== */

static const char *const MIRAKURA[] = {
    "PL!HS-PR-021-PR", "PL!HS-bp5-003-R＋", "PL!HS-bp5-003-P", "PL!HS-bp5-003-AR",
    "PL!HS-bp5-003-SEC", "PL!HS-PR-021-RM", "PL!HS-pb1-019-N", "PL!HS-bp6-011-R",
    "PL!HS-bp6-014-R", "PL!HS-PR-006-PR", "PL!HS-PR-018-PR", "PL!HS-bp1-009-R",
    "PL!HS-bp1-009-P", "PL!HS-bp1-014-N", "PL!HS-bp1-015-N", "PL!HS-bp2-014-N",
    "PL!HS-bp5-014-N", "PL!HS-PR-018-RM"
};

static void test_fanfare_miracluck_blades(void)
{
    int live = rb_find_card_by_no("PL!HS-bp6-031-L");
    int himeno = rb_find_card_by_no("PL!HS-bp6-014-R");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(live >= 0 && himeno >= 0, "ファンファーレ fixtures resolve");
    if (live < 0) return;
    {
        TestGame tg; test_game_new(&tg);
        for (int i = 0; i < 15; i++)
            test_add_to_discard(&tg, test_new_id(&tg, MIRAKURA[i % 18]));
        test_add_to_discard(&tg, filler);            /* one non-MiraKura */
        tg.state.p[0].stage[0] = himeno;
        tg.state.p[0].stage[1] = filler;
        tg.state.p[0].stage[2] = test_new_id(&tg, "PL!-sd1-010-SD");
        test_add_to_hand(&tg, live);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, live);
        advance_to_live_start(&tg);
        drain(&tg, 1);
        EXPECTED_GAP("fanfare 11a: 15+ みらくらぱーく！ cards moved -> 3 blade on 姫芽",
                     test_get_blade_modifier(&tg, himeno) == 3);
    }
    {
        TestGame tg; test_game_new(&tg);
        for (int i = 0; i < 3; i++)
            test_add_to_discard(&tg, test_new_id(&tg, MIRAKURA[i % 18]));
        tg.state.p[0].stage[0] = himeno;
        tg.state.p[0].stage[1] = filler;
        tg.state.p[0].stage[2] = test_new_id(&tg, "PL!-sd1-010-SD");
        test_add_to_hand(&tg, live);
        fill_decks20(&tg);
        advance_to_live_set_p1(&tg);
        test_set_live_card(&tg, 0, live);
        advance_to_live_start(&tg);
        drain(&tg, 1);
        CHECK_EQ(test_get_blade_modifier(&tg, himeno), 0,
                 "fanfare 11b: fewer than 15 みらくらぱーく！ cards -> 0 blade");
    }
}

/* ===================================================================== */
/* R. integration/pvp_room_test.rs — the engine-facing half                */
/*    both_players_multiple_live_start_abilities_get_correct_choice_      */
/*    routing                                                              */
/*    (the pvp_player_can_act web gate itself has no C port)               */
/* ===================================================================== */

static void test_live_start_choice_routing_both_seats(void)
{
    int shizuku = rb_find_card_by_no("PL!N-bp3-015-N");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    CHECK(shizuku >= 0, "PL!N-bp3-015-N fixture resolves");
    if (shizuku < 0) return;
    TestGame tg;
    test_game_new(&tg);
    int p1[3], p2[3];
    for (int i = 0; i < 3; i++) { p1[i] = test_id(&tg, "PL!N-bp3-015-N");
                                  tg.state.p[0].stage[i] = p1[i]; }
    for (int i = 0; i < 3; i++) { p2[i] = test_id(&tg, "PL!N-bp3-015-N");
                                  tg.state.p[1].stage[i] = p2[i]; }
    fill_decks(&tg, filler, 30);

    /* Advance into the ライブ開始時 window BY NAME rather than a blind pass count. */
    int guard = 0;
    while (tg.state.phase != RB_PHASE_PERFORMANCE && guard++ < 16) {
        test_pass(&tg);
        drain(&tg, 1);
    }
    CHECK_EQ(tg.state.phase, RB_PHASE_PERFORMANCE,
             "routing: the ライブ開始時 window is reached by name");
    CHECK_EQ(rb_has_pending_choice(&tg.state), 1,
             "routing: LiveStart with 3 abilities per side leaves a pending choice");

    /* Drain both seats' LiveStart choices. */
    int heart_prompts = 0;
    guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 40) {
        if (!strcmp(test_pending_choice_type(&tg), "SelectHeartColor")) heart_prompts++;
        rb_resume_with_choice(&tg.state, 0);
    }
    CHECK(heart_prompts >= 3,
          "routing: P1's three ライブ開始時 abilities each raise their own heart-colour prompt");
}

/* ===================================================================== */

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_phase_walk_through_turn3();
    test_live_zone_two_card_selection();
    test_tie_at_two_successes_blocks_third();
    test_no_live_cards_clean_rollover();
    test_himeko_repositions_opponent_member();
    test_himeko_gate_blocked();
    test_seat_relative_energy_comparison();
    test_mei_cost_gate();
    test_rina_mutual_mill_from_p2_seat();
    test_natsumi_blades();
    test_hajimari_set_required_hearts();
    test_dream_with_you_blade_gate();
    test_hanamaru_all_cost_comparison();
    test_suzu_cost_bonus();
    test_izumi_debut_draw2_discard1();
    test_kanon_invalidate_and_recover();
    test_live_end_grant_expires_through_rollover();
    test_ginko_debut_cost_gate();
    test_kanan_discard_gain_live();
    test_ayumu_compound_condition();
    test_sumire_center_debut_gate();
    test_honoka_live_score_dynamic();
    test_karin_ab1_select_from_discard();
    test_saikou_success_zone_scores();
    test_kinako_hand_cost_discount();
    test_kokoro_cost_copy();
    test_kanan_cannot_mill_itself();
    test_karin_livestart_blade4_window();
    test_yell_blade_heart_gates();
    test_solitude_rain_heart_colour_scoring();
    test_fanfare_miracluck_blades();
    test_live_start_choice_routing_both_seats();
    rb_unload();
    printf("\n%d assertions, %d failures, %d known gaps\n",
           assertions, failures, gaps);
    if (failures) return 1;
    printf("ALL INTEGRATION PARITY CHECKS PASSED (%d known C/Rust gaps)\n", gaps);
    return 0;
}
