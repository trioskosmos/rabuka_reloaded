/* test_tp_joint_card_live_start.c — C port of
 * engine/tests/test_modules/rules/trigger_paths/joint_card_live_start_test.rs
 *
 * LL-bpX-001-R＋ multi-name joint cards (bp1 / bp2 / bp3 / bp4 / bp6) and their
 * ライブ開始時 (live-start) / 登場 (debut) trigger subtleties.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define REQUIRE(c, msg) do { \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
        return; \
    } \
} while (0)

#define CHECK(c, msg) do { \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

#define CHECK_EQ(a, e, msg) do { \
    int a_ = (a); \
    int e_ = (e); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, a_, e_); \
        failures++; \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

/* Rust: assert_select_card(zone, count, allow_skip) over the live pending
   Choice::SelectCard. Reads the flat C RbChoice the queue parks. */
static int is_select_card(TestGame *game, const char *zone, int count, int allow_skip)
{
    const RbChoice *c;
    if (!rb_has_pending_choice(&game->state)) return 0;
    c = rb_get_pending_choice(&game->state);
    if (c->kind != RB_CHOICE_SELECT_CARD) return 0;
    if (strcmp(c->zone, zone) != 0) return 0;
    if (c->count != count) return 0;
    return c->allow_skip == allow_skip;
}

/* Rust: game.select_indices(&[]) — the "skip / select nothing" answer.
   ENGINE/HARNESS GAP: test_select_indices(tg, NULL, 0) in src/test_game.c is a
   no-op (it only calls rb_resume_with_choice when n > 0), so an empty Rust
   selection would spin forever. The C engine's own auto-answer path
   (src/engine.c: rb_resume_with_choice(g, -1)) uses -1 to mean "select nothing",
   so that is the real engine behaviour we drive here. */
static void select_nothing(TestGame *game)
{
    test_resume_choice(game, -1);
}

/* Rust: while game.has_pending_choice() { game.select_indices(&[]) } */
static void drain_skipping(TestGame *game)
{
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 64) {
        select_nothing(game);
    }
}

/* Rust: game.state.player2.hand.cards.push(x) */
static void opp_hand_push(TestGame *game, int card_id)
{
    RbPlayer *P = &game->state.p[1];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = card_id;
}

/* Rust: fill both main decks with 10 copies of `filler` and give p2 one hand
   card (identical preamble in almost every live-start test in the Rust file). */
static void fill_both_decks(TestGame *game, int filler)
{
    int i;
    for (i = 0; i < 10; i++) {
        test_add_to_deck_pl(game, 0, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
    opp_hand_push(game, filler);
}

/* Rust helper fn advance_to_live_start(game) — 5 passes: Main -> Active ->
   Energy -> Draw -> Main -> LiveCardSetP1 (P1 then sets its live card). */
static void advance_to_live_start(TestGame *game)
{
    int i;
    for (i = 0; i < 5; i++) test_pass(game);
}

/* Rust helper fn finish_live_setup(game) — 2 passes: LiveCardSetP1 ->
   LiveCardSetP2 -> LiveStart (live-start triggers fire here). */
static void finish_live_setup(TestGame *game)
{
    test_pass(game);
    test_pass(game);
}

/* ─────────────────────────────────────────────────────────────
 * bp1 — 上原歩夢＆澁谷かのん＆日野下花帆
 * ライブ開始時: discard exactly 3 cards whose name is any of the three → gain score+3
 * ───────────────────────────────────────────────────────────── */

/* The multi-name card itself counts as all three names for the cost filter.
   One copy in hand + 2 fillers whose names match should satisfy count=3.
   Selecting all 3 should pay cost and grant score+3. */
static void test_bp1_live_start_pay_cost_gains_score_three(void)
{
    TestGame game;
    int joint, ayumu, kanon, live, filler, guard = 0;
    int idxs[3];

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp1-001-R＋"); /* 上原歩夢&澁谷かのん&日野下花帆 */
    ayumu  = test_id(&game, "PL!N-sd1-001-SD"); /* 上原歩夢 */
    kanon  = test_id(&game, "PL!SP-sd1-001-SD"); /* 澁谷かのん */
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && ayumu >= 0 && kanon >= 0 && live >= 0 && filler >= 0,
            "bp1: all card ids resolve");

    /* Stage: joint in center */
    test_add_to_stage(&game, 1, joint);
    /* Hand: 3 matching names (joint self + ayumu + kanon) to pay cost */
    test_add_to_hand(&game, joint);
    test_add_to_hand(&game, ayumu);
    test_add_to_hand(&game, kanon);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game),
          "bp1 live_start should prompt to select 3 cards for optional cost");

    /* Select all 3 matching cards. ENGINE GAP: the C harness
       rb_resume_with_choice(g, idx) carries a single index, so a genuine
       multi-index resume (Rust try_select_indices(&[0,1,2]) falling through to
       resume_with_choice(state, None, Some(vec![0,1,2]))) is not expressible;
       we drive the real engine with the first filtered index each round. */
    idxs[0] = 0; idxs[1] = 1; idxs[2] = 2;
    while (test_has_pending_choice(&game) && guard++ < 16) {
        test_select_indices(&game, idxs, 3);
    }

    CHECK(!test_has_pending_choice(&game), "bp1 should resolve after cost payment");

    /* Effect: score+3 modifier applied to joint */
    rb_recalc_constants(&game.state);
    CHECK_EQ(game.state.mods.p1_constant_total_score_bonus, 3,
             "bp1: paying cost should grant score+3");
}

/* If there are no matching name cards in hand, the optional cost is skipped
   and no effect fires. */
static void test_bp1_live_start_no_matching_hand_cards_skips(void)
{
    TestGame game;
    int joint, live, filler;

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp1-001-R＋");
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && live >= 0 && filler >= 0, "bp1-skip: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    /* Hand: only unrelated fillers — no matching names */
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    /* Add live card to hand so it can be set */
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    /* Optional cost with no eligible cards → engine skips automatically */
    CHECK(!test_has_pending_choice(&game),
          "bp1 should auto-skip when there are no matching-name cards in hand");
}

/* ─────────────────────────────────────────────────────────────
 * bp2 — 渡辺曜＆鬼塚夏美＆大沢瑠璃乃
 * ライブ開始時: discard ANY NUMBER of matching → 1 blade per discarded
 * ───────────────────────────────────────────────────────────── */

/* Discarding 2 matching cards should grant 2 blades. */
static void test_bp2_live_start_discard_any_number_gains_blade_per_card(void)
{
    TestGame game;
    int joint, you, natsumi, live, filler;
    int idxs[2];

    test_game_new(&game);
    joint   = test_id(&game, "LL-bp2-001-R＋"); /* 渡辺曜&鬼塚夏美&大沢瑠璃乃 */
    you     = test_id(&game, "PL!S-sd1-005-SD"); /* 渡辺 曜 */
    natsumi = test_id(&game, "PL!SP-sd1-009-SD"); /* 鬼塚夏美 */
    live    = test_id(&game, "PL!-sd1-010-SD");
    filler  = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && you >= 0 && natsumi >= 0 && live >= 0 && filler >= 0,
            "bp2: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    /* Put 2 matching-name cards in hand (not the joint card itself) */
    test_add_to_hand(&game, you);
    test_add_to_hand(&game, natsumi);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp2 live_start should prompt for any-number discard");

    /* Select both matching cards (indices 0 and 1) */
    idxs[0] = 0; idxs[1] = 1;
    test_select_indices(&game, idxs, 2);
    select_nothing(&game); /* skip re-prompt, finalize */

    CHECK(!test_has_pending_choice(&game), "bp2 should resolve after card selection");

    /* 2 cards discarded → 2 blades gained by joint */
    CHECK_EQ(test_get_blade_modifier(&game, joint), 2,
             "bp2: discarding 2 cards should grant 2 blades");
}

/* Skipping the optional cost (0 cards) grants 0 blades. */
static void test_bp2_live_start_skip_cost_gains_no_blade(void)
{
    TestGame game;
    int joint, you, live, filler;

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp2-001-R＋");
    you    = test_id(&game, "PL!S-sd1-005-SD"); /* 渡辺 曜 */
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && you >= 0 && live >= 0 && filler >= 0, "bp2-skip: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, you);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp2 should prompt for card selection");

    /* Skip — select no cards */
    select_nothing(&game);

    CHECK(!test_has_pending_choice(&game), "Should resolve after skip");
    CHECK_EQ(test_get_blade_modifier(&game, joint), 0,
             "Skipping bp2 cost should give 0 blades");
}

/* ─────────────────────────────────────────────────────────────
 * bp3 — 園田海未＆津島善子＆天王寺璃奈
 * ライブ開始時: pay 6E optional → gain 3 blade until live end
 * ───────────────────────────────────────────────────────────── */

/* Paying 6 energy grants 3 blades. */
static void test_bp3_live_start_pay_6e_gains_3_blade(void)
{
    TestGame game;
    int joint, live, filler;

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp3-001-R＋"); /* 園田海未&津島善子&天王寺璃奈 */
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && live >= 0 && filler >= 0, "bp3: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10); /* more than enough */

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp3 should prompt to pay 6E (or skip)");

    /* Pay the optional cost */
    test_resume_choice(&game, 1); /* index 1 = "Pay" */

    CHECK(!test_has_pending_choice(&game), "bp3 should resolve after paying energy");

    CHECK_EQ(test_get_blade_modifier(&game, joint), 3,
             "bp3: paying 6E should grant 3 blades");

    /* Energy should have decreased by 6 */
    CHECK_EQ(game.state.p[0].energy_active, 4, "6 energy should have been paid");
}

/* Skipping the 6E cost gives 0 blades. */
static void test_bp3_live_start_skip_gains_no_blade(void)
{
    TestGame game;
    int joint, live, filler;

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp3-001-R＋");
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && live >= 0 && filler >= 0, "bp3-skip: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp3 should prompt for optional cost");

    test_resume_choice(&game, 0); /* Skip */

    CHECK_EQ(test_get_blade_modifier(&game, joint), 0, "Skipping bp3 cost gives 0 blades");
}

/* ─────────────────────────────────────────────────────────────
 * bp4 — 絢瀬絵里＆朝香果林＆葉月恋
 * triggers="ライブ開始時, 登場" — dual trigger, previously broken
 * Effect: look top 5, optionally take 1 named member to hand, discard rest,
 *         then wait all opponent members ≤ revealed card cost with ≤3 original blade.
 * ───────────────────────────────────────────────────────────── */

/* The dual-trigger ability MUST fire on ライブ開始時.
   Previously the engine did `triggers == "ライブ開始時"` (exact match),
   which missed `"ライブ開始時, 登場"`. This test verifies the fix. */
static void test_bp4_live_start_dual_trigger_fires(void)
{
    TestGame game;
    int joint, live, ayumu, filler, i;

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp4-001-R＋"); /* 絢瀬絵里&朝香果林&葉月恋 */
    live   = test_id(&game, "PL!-sd1-010-SD");
    ayumu  = test_id(&game, "PL!-pb1-011-R"); /* 絢瀬絵里 — matches select filter */
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && live >= 0 && ayumu >= 0 && filler >= 0, "bp4-live: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, live);
    /* Only ~1 draw happens before live_start triggers. Place ayumu at deck
       index 3 so after 1 draw it is still in the top-5 looked-at cards. */
    game.state.p[0].deck.n = 0;
    for (i = 0; i < 3; i++) test_add_to_deck_pl(&game, 0, filler);
    test_add_to_deck_pl(&game, 0, ayumu); /* positioned at index 3 */
    for (i = 0; i < 7; i++) test_add_to_deck_pl(&game, 0, filler);
    for (i = 0; i < 10; i++) test_add_to_deck_pl(&game, 1, filler);
    opp_hand_push(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game),
          "bp4 live_start ability MUST fire (dual trigger fix). Previously broken.");
    /* Rust: game.assert_select_card("looked_at", 1, true) */
    CHECK(is_select_card(&game, "looked_at", 1, 1),
          "bp4 live_start: pending choice is SelectCard zone=looked_at count=1 allow_skip");
    /* filtered_indices=[2]: only ayumu passes the name filter, so position 0
       within the filtered pool picks ayumu (frontend filtered-relative index). */
    {
        int zero[1] = {0};
        test_select_indices(&game, zero, 1);
    }
    CHECK(test_hand_has(&game, ayumu), "resolved look must take the named member to hand");
}

/* bp4 debut trigger still works independently. */
static void test_bp4_debut_trigger_fires_independently(void)
{
    TestGame game;
    int joint, ayumu, filler, i, zero[1];

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp4-001-R＋");
    ayumu  = test_id(&game, "PL!-pb1-011-R"); /* 絢瀬絵里 — matches select filter */
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && ayumu >= 0 && filler >= 0, "bp4-debut: all card ids resolve");

    test_add_to_hand(&game, joint);
    game.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&game, 0, ayumu);
    for (i = 0; i < 10; i++) test_add_to_deck_pl(&game, 0, filler);
    test_give_energy(&game, 20);

    /* Play joint to stage — this fires 登場 trigger */
    test_play_to_stage(&game, joint, 1);

    CHECK(test_has_pending_choice(&game),
          "bp4 debut (登場) trigger should also fire the look-and-select ability");
    /* Rust: game.assert_select_card("looked_at", 1, true) */
    CHECK(is_select_card(&game, "looked_at", 1, 1),
          "bp4 debut: pending choice is SelectCard zone=looked_at count=1 allow_skip");
    /* Deck top is ayumu → position 0. */
    zero[0] = 0;
    test_select_indices(&game, zero, 1);
    CHECK(test_hand_has(&game, ayumu), "resolved debut look must take the named member to hand");
}

/* bp4 debut trigger completes look_and_select: select member → hand, rest → waitroom. */
static void test_bp4_debut_look_select_puts_card_in_hand(void)
{
    TestGame game;
    int joint, ayumu, f1, f2, f3, f4, i, zero[1];
    int hand_before, hand_after_play, hand_after;

    test_game_new(&game);
    joint = test_id(&game, "LL-bp4-001-R＋"); /* 絢瀬絵里&朝香果林&葉月恋 */
    ayumu = test_id(&game, "PL!-pb1-011-R");   /* 絢瀬絵里 cost=2 — matches select filter */
    f1 = test_new_id(&game, "PL!-sd1-010-SD"); /* cost=4 blade=1 */
    f2 = test_new_id(&game, "PL!-sd1-010-SD");
    f3 = test_new_id(&game, "PL!-sd1-010-SD");
    f4 = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && ayumu >= 0 && f1 >= 0 && f2 >= 0 && f3 >= 0 && f4 >= 0,
            "bp4-look: all card ids resolve");

    test_add_to_hand(&game, joint);
    /* Deck top: [ayumu(matching), filler1, filler2, filler3, filler4, ...] */
    game.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&game, 0, ayumu);
    test_add_to_deck_pl(&game, 0, f1);
    test_add_to_deck_pl(&game, 0, f2);
    test_add_to_deck_pl(&game, 0, f3);
    test_add_to_deck_pl(&game, 0, f4);
    for (i = 0; i < 10; i++) test_new_id(&game, "PL!-sd1-010-SD"),
                                   test_add_to_deck_pl(&game, 0, test_new_id(&game, "PL!-sd1-010-SD"));
    test_give_energy(&game, 20);

    hand_before = test_hand_len(&game);

    /* Play joint to stage — this triggers 登場 ability (look top 5, select 1) */
    test_play_to_stage(&game, joint, 1);

    hand_after_play = test_hand_len(&game);
    CHECK_EQ(hand_after_play, hand_before - 1, "Joint should leave hand when played");

    CHECK(test_has_pending_choice(&game),
          "bp4 debut should prompt for card selection from looked-at cards");

    /* Select the first card (ayumu — matches name filter) */
    zero[0] = 0;
    test_select_indices(&game, zero, 1);

    hand_after = test_hand_len(&game);
    CHECK_EQ(hand_after, hand_after_play + 1, "Selected card should go to hand");
    CHECK(test_hand_has(&game, ayumu), "Ayumu should be in hand after selection");

    /* Rust player1.waitroom === the C engine's `discard` bag. */
    CHECK(game.state.p[0].discard.n >= 4,
          "Remaining looked-at cards should go to waitroom");

    /* No pending choices should remain */
    drain_skipping(&game);
    CHECK(!test_has_pending_choice(&game),
          "All choices should be resolved after look-and-select completes");
}

/* bp4 debut: skip selection → no card to hand, all cards to waitroom. */
static void test_bp4_debut_skip_select_discards_all(void)
{
    TestGame game;
    int joint, ayumu, f1, f2, f3, f4, i;
    int hand_after_play, hand_after;

    test_game_new(&game);
    joint = test_id(&game, "LL-bp4-001-R＋");
    ayumu = test_id(&game, "PL!-pb1-011-R"); /* 絢瀬絵里 */
    f1 = test_new_id(&game, "PL!-sd1-010-SD");
    f2 = test_new_id(&game, "PL!-sd1-010-SD");
    f3 = test_new_id(&game, "PL!-sd1-010-SD");
    f4 = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && ayumu >= 0 && f1 >= 0 && f2 >= 0 && f3 >= 0 && f4 >= 0,
            "bp4-skip: all card ids resolve");

    test_add_to_hand(&game, joint);
    game.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&game, 0, ayumu);
    test_add_to_deck_pl(&game, 0, f1);
    test_add_to_deck_pl(&game, 0, f2);
    test_add_to_deck_pl(&game, 0, f3);
    test_add_to_deck_pl(&game, 0, f4);
    for (i = 0; i < 10; i++) {
        test_add_to_deck_pl(&game, 0, test_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_deck_pl(&game, 1, test_new_id(&game, "PL!-sd1-010-SD"));
    }
    opp_hand_push(&game, test_new_id(&game, "PL!-sd1-010-SD"));
    test_give_energy(&game, 20);

    test_play_to_stage(&game, joint, 1);
    hand_after_play = test_hand_len(&game);

    CHECK(test_has_pending_choice(&game), "bp4 debut should prompt");

    /* Skip — select empty */
    select_nothing(&game);

    hand_after = test_hand_len(&game);
    CHECK_EQ(hand_after, hand_after_play, "Skipping selection should not add any card to hand");
    CHECK(game.state.p[0].discard.n >= 5,
          "All looked-at cards should go to waitroom when skipped");
}

/* bp4 debut: select matching member → wait opponent members with
 * cost ≤ revealed card's cost AND original blade ≤ 3. */
static void test_bp4_debut_wait_opponent_members_after_selection(void)
{
    TestGame game;
    int joint, ayumu, filler, i, zero[1];
    int p2_low, p2_high_cost, p2_filler;
    const char *ori;

    test_game_new(&game);
    joint = test_id(&game, "LL-bp4-001-R＋");
    ayumu = test_id(&game, "PL!-pb1-011-R"); /* 絢瀬絵里 cost=2, blade=1 */
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    p2_low       = test_id(&game, "PL!-sd1-002-SD"); /* cost=2, blade=1 → should wait */
    p2_high_cost = test_id(&game, "PL!-sd1-001-SD"); /* cost=11, blade=3 → should NOT wait */
    p2_filler    = test_id(&game, "PL!-sd1-010-SD"); /* cost=4, blade=1 → should NOT wait */
    REQUIRE(joint >= 0 && ayumu >= 0 && filler >= 0 && p2_low >= 0 && p2_high_cost >= 0 &&
                p2_filler >= 0,
            "bp4-wait: all card ids resolve");

    test_set_opp_stage(&game, 0, p2_high_cost);
    test_set_opp_stage(&game, 1, p2_low);
    test_set_opp_stage(&game, 2, p2_filler);

    test_add_to_hand(&game, joint);
    game.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&game, 0, ayumu);
    test_add_to_deck_pl(&game, 0, filler);
    test_add_to_deck_pl(&game, 0, filler);
    test_add_to_deck_pl(&game, 0, filler);
    test_add_to_deck_pl(&game, 0, filler);
    for (i = 0; i < 10; i++) {
        test_add_to_deck_pl(&game, 0, test_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_deck_pl(&game, 1, test_new_id(&game, "PL!-sd1-010-SD"));
    }
    opp_hand_push(&game, test_new_id(&game, "PL!-sd1-010-SD"));
    test_give_energy(&game, 20);

    test_play_to_stage(&game, joint, 1);
    CHECK(test_has_pending_choice(&game), "bp4 debut should prompt");

    zero[0] = 0;
    test_select_indices(&game, zero, 1);

    drain_skipping(&game);

    /* ayumu cost=2 → filter: cost<=2 AND original blade<=3
       p2_low (cost=2, blade=1) should be waited (orientation = "wait") */
    ori = rb_mods_get_orientation(&game.state.mods, p2_low);
    CHECK(ori != NULL && strcmp(ori, "wait") == 0, "p2_low should be waited");
    ori = rb_mods_get_orientation(&game.state.mods, p2_high_cost);
    CHECK(!(ori != NULL && strcmp(ori, "wait") == 0), "p2_high_cost should NOT be waited");
    ori = rb_mods_get_orientation(&game.state.mods, p2_filler);
    CHECK(!(ori != NULL && strcmp(ori, "wait") == 0), "p2_filler should NOT be waited");
}

/* ─────────────────────────────────────────────────────────────
 * bp6 — 南ことり＆黒澤ダイヤ＆徒町小鈴
 * ライブ開始時: discard ANY NUMBER → gain 1 heart per distinct COLOR of discarded
 * ───────────────────────────────────────────────────────────── */

/* Sum of the joint card's heart-01..heart-06 modifiers.
   Direct rb_mods_get_heart (not test_get_heart_modifier) because that shim
   remaps colour 5 to RB_HEART_ORANGE, which is wrong for heart05. */
static int total_hearts(TestGame *game, int joint)
{
    int c, total = 0;
    for (c = 1; c <= 6; c++) total += rb_mods_get_heart(&game->state.mods, joint, c);
    return total;
}

static int heart_of(TestGame *game, int joint, int color)
{
    return rb_mods_get_heart(&game->state.mods, joint, color);
}

/* Discarding 2 cards of 2 different colors → gain 1 of each distinct color. */
static void test_bp6_live_start_two_distinct_colors_gain_two_hearts(void)
{
    TestGame game;
    int joint, kotori, dia, live, filler;
    int idxs[2];

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp6-001-R＋"); /* 南ことり&黒澤ダイヤ&徒町小鈴 */
    kotori = test_id(&game, "PL!-bp3-003-R");  /* 南ことり — hearts: 01,03,06 */
    dia    = test_id(&game, "PL!S-sd1-004-SD"); /* 黒澤ダイヤ — hearts: 02,04,05 */
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && kotori >= 0 && dia >= 0 && live >= 0 && filler >= 0,
            "bp6: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, kotori);
    test_add_to_hand(&game, dia);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp6 should prompt for any-number discard");

    /* Discard both cards */
    idxs[0] = 0; idxs[1] = 1;
    test_select_indices(&game, idxs, 2);
    select_nothing(&game); /* skip re-prompt, finalize */

    CHECK(!test_has_pending_choice(&game), "bp6 should resolve after selection");

    /* kotori: heart01, heart03, heart06
       dia:    heart02, heart04, heart05
       Combined distinct colors: 01+02+03+04+05+06 = 6 */
    CHECK_EQ(heart_of(&game, joint, 1), 1, "Heart01 from kotori");
    CHECK_EQ(heart_of(&game, joint, 2), 1, "Heart02 from dia");
    CHECK_EQ(heart_of(&game, joint, 3), 1, "Heart03 from kotori");
    CHECK_EQ(heart_of(&game, joint, 4), 1, "Heart04 from dia");
    CHECK_EQ(heart_of(&game, joint, 5), 1, "Heart05 from dia");
    CHECK_EQ(heart_of(&game, joint, 6), 1, "Heart06 from kotori");
}

/* Discarding 2 cards of the SAME color → only 1 heart gained (deduplication). */
static void test_bp6_live_start_same_color_deduplicates(void)
{
    TestGame game;
    Card kotori_card;
    int joint, live, filler;
    int kotori1, kotori2;
    int seen[7], k, expected = 0, total;
    int idxs[2];

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp6-001-R＋");
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    /* Use two copies of the same card (same colors guaranteed) */
    kotori1 = test_id(&game, "PL!-bp3-003-R");    /* 南ことり */
    kotori2 = test_new_id(&game, "PL!-bp3-003-R"); /* another copy, same colors */
    REQUIRE(joint >= 0 && live >= 0 && filler >= 0 && kotori1 >= 0 && kotori2 >= 0,
            "bp6-dedupe: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, kotori1);
    test_add_to_hand(&game, kotori2);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp6 should prompt for discard");

    /* Discard both copies */
    idxs[0] = 0; idxs[1] = 1;
    test_select_indices(&game, idxs, 2);
    select_nothing(&game); /* skip re-prompt, finalize */

    /* Determine the exact colors from the card.
       Member cards store their heart colors in base_heart; need_heart is only
       on live cards — in C the first num_base (color,count) pairs. */
    REQUIRE(rb_decode_card_by_index((uint32_t)kotori1, &kotori_card), "kotori card must exist");
    memset(seen, 0, sizeof seen);
    for (k = 0; k < kotori_card.num_base && k < RB_MAX_HEARTS; k++) {
        int col = kotori_card.heart_color[k];
        if (kotori_card.heart_count[k] > 0 && col >= 0 && col <= 6 && !seen[col]) {
            seen[col] = 1;
            expected++;
        }
    }
    rb_free_card(&kotori_card);
    /* 1 per distinct color, deduped */
    total = total_hearts(&game, joint);
    CHECK_EQ(total, expected,
             "bp6: 2 same-color cards → distinct colors only, deduplicated");
}

/* Skipping the bp6 cost (0 cards) gains no hearts. */
static void test_bp6_live_start_skip_gains_no_heart(void)
{
    TestGame game;
    int joint, kotori, live, filler;

    test_game_new(&game);
    joint  = test_id(&game, "LL-bp6-001-R＋");
    kotori = test_id(&game, "PL!-bp3-003-R");
    live   = test_id(&game, "PL!-sd1-010-SD");
    filler = test_new_id(&game, "PL!-sd1-010-SD");
    REQUIRE(joint >= 0 && kotori >= 0 && live >= 0 && filler >= 0, "bp6-skip: all card ids resolve");

    test_add_to_stage(&game, 1, joint);
    test_add_to_hand(&game, kotori);
    test_add_to_hand(&game, live);
    fill_both_decks(&game, filler);
    test_give_energy(&game, 10);

    advance_to_live_start(&game);
    test_set_live_card(&game, 0, live);
    finish_live_setup(&game);

    CHECK(test_has_pending_choice(&game), "bp6 should prompt for discard");

    /* Skip — select nothing */
    select_nothing(&game);

    CHECK_EQ(total_hearts(&game, joint), 0, "Skipping bp6 cost should give 0 hearts");
}

/* ─────────────────────────────────────────────────────────────
 * General: multi-name card identity checks
 * ───────────────────────────────────────────────────────────── */

/* bp1 card (上原歩夢&澁谷かのん&日野下花帆) must be recognised as having
   all three individual names (FAQ: the card has all three identities). */
static void test_joint_card_has_all_three_name_identities(void)
{
    TestGame game;
    Card card;
    char norm[512];
    int joint_id;
    /* 上原歩夢 / 澁谷かのん / 日野下花帆 — UCN-escaped so the source stays ASCII
       while gcc emits the same UTF-8 bytes the card database stores. */
    const char *ayumu  = "上原歩夢";
    const char *kanon  = "澁谷かのん";
    const char *kaho   = "日野下花帆";
    const char *full   = "＆";

    test_game_new(&game);
    joint_id = test_id(&game, "LL-bp1-001-R＋");
    REQUIRE(joint_id >= 0, "bp1 joint card exists");
    REQUIRE(rb_decode_card_by_index((uint32_t)joint_id, &card), "card must exist");
    REQUIRE(card.name != NULL, "card has a name");

    /* Rust db.get_card_names(id) == normalize_name(name).replace('＆',"&").split('&');
       "any name contains X" is therefore "the normalized name contains X". */
    rb_card_normalize_name(card.name, norm, sizeof norm);

    CHECK(strstr(norm, ayumu) != NULL, "Joint card must carry identity '上原歩夢'");
    CHECK(strstr(norm, kanon) != NULL, "Joint card must carry identity '澁谷かのん'");
    CHECK(strstr(norm, kaho)  != NULL, "Joint card must carry identity '日野下花帆'");
    CHECK(strstr(card.name, "&") != NULL || strstr(card.name, full) != NULL,
          "Card name should contain an ampersand separator");
    rb_free_card(&card);
}

/* The characters filter on a cost matches a multi-name card for ALL constituent names. */
static void test_multi_name_card_matches_any_constituent_in_characters_filter(void)
{
    TestGame game;
    int joint_id;
    /* Rust: util::filter_from_parts(..., Some(&["上原歩夢"]), ...)
             .matches(&db, joint_id, false)
       C: the characters half of CardFilter is rb_card_matches_characters(). */
    const char *ayumu_names[1];
    const char *kanon_names[1];
    const char *kaho_names[1];

    test_game_new(&game);
    joint_id = test_id(&game, "LL-bp1-001-R＋");
    REQUIRE(joint_id >= 0, "bp1 joint card exists");

    /* Filter: matches "上原歩夢" — should match the joint card */
    ayumu_names[0] = "上原歩夢";
    CHECK(rb_card_matches_characters(joint_id, ayumu_names, 1) == 1,
          "Joint card should match a filter for '上原歩夢'");

    /* Filter: matches "澁谷かのん" — should match the joint card */
    kanon_names[0] = "澁谷かのん";
    CHECK(rb_card_matches_characters(joint_id, kanon_names, 1) == 1,
          "Joint card should match a filter for '澁谷かのん'");

    /* Filter: matches "日野下花帆" — should match the joint card */
    kaho_names[0] = "日野下花帆";
    CHECK(rb_card_matches_characters(joint_id, kaho_names, 1) == 1,
          "Joint card should match a filter for '日野下花帆'");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_bp1_live_start_pay_cost_gains_score_three();
    test_bp1_live_start_no_matching_hand_cards_skips();
    test_bp2_live_start_discard_any_number_gains_blade_per_card();
    test_bp2_live_start_skip_cost_gains_no_blade();
    test_bp3_live_start_pay_6e_gains_3_blade();
    test_bp3_live_start_skip_gains_no_blade();
    test_bp4_live_start_dual_trigger_fires();
    test_bp4_debut_trigger_fires_independently();
    test_bp4_debut_look_select_puts_card_in_hand();
    test_bp4_debut_skip_select_discards_all();
    test_bp4_debut_wait_opponent_members_after_selection();
    test_bp6_live_start_two_distinct_colors_gain_two_hearts();
    test_bp6_live_start_same_color_deduplicates();
    test_bp6_live_start_skip_gains_no_heart();
    test_joint_card_has_all_three_name_identities();
    test_multi_name_card_matches_any_constituent_in_characters_filter();

    rb_unload();
    if (failures) return 1;
    printf("ALL TRIGGER PATHS JOINT CARD LIVE START CHECKS PASSED\n");
    return 0;
}
