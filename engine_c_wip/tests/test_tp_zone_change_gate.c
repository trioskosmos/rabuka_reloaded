/* Port of engine/tests/test_modules/rules/trigger_paths/zone_change_gate_test.rs
 *
 * Zone-change trigger-gate coverage. Abilities whose printed text fires only
 * on a SPECIFIC zone transition 「…から…に置かれたとき」. Positive controls plus
 * wrong-source, wrong-group and wrong-player negatives for each family.
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
    } else printf("ok: %s\n", msg); \
} while (0)

#define CHECK_EQ(a, e, msg) do { \
    int a_ = (a), e_ = (e); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, a_, e_); \
        failures++; \
    } else printf("ok: %s\n", msg); \
} while (0)

#define CHECK_EQ_STR(actual, expected, msg) do { \
    const char *actual_ = (actual); \
    const char *expected_ = (expected); \
    if (strcmp(actual_, expected_) != 0) { \
        fprintf(stderr, "FAIL: %s (got %s expected %s)\n", msg, actual_, expected_); \
        failures++; \
    } else printf("ok: %s\n", msg); \
} while (0)

/* 桜内梨子 ab#0: 『Aqours』のライブカードが自分のライブカード置き場から
 * 控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。 */
#define RIKO               "PL!S-bp6-002-R＋"
#define AQOURS_LIVE        "PL!S-bp2-019-L" /* WATER BLUE NEW WORLD */
#define MUSE_LIVE          "PL!-sd1-019-SD" /* μ's live — wrong-group negative */
#define FILLER             "PL!-sd1-010-SD"

/* 天王寺璃奈 ab#0 (ライブ開始時): このターン、ブレードハートを持たない
 * メンバーが自分のライブカード置き場から控え室に置かれている場合 →
 * draw 1 + heart03/05/06 until live end. */
#define REINA              "PL!N-pb1-009-R"
#define NO_BLADE_MEMBER    "PL!-sd1-001-SD" /* 穂乃果: blade_heart=None */
#define BLADE_MEMBER       "PL!-sd1-008-SD" /* 花陽: blade_heart={'b_heart03':1} */

/* 宮下愛 ab#0 (自動) — baton touch payoff, conditions describe the NEWCOMER. */
#define MIYAMIYA           "PL!N-bp5-005-R＋"
#define NEWCOMER_COST15    "PL!N-bp7-003-R＋"
#define NEWCOMER_COST13    "PL!N-bp4-007-R＋"
#define NEWCOMER_BLADE_HEART "PL!N-bp4-009-R"

/* Rust `fill_decks`: clear both main decks and push 30 filler into each. */
static void fill_decks(TestGame *g, int filler)
{
    for (int p = 0; p < 2; p++) g->state.p[p].deck.n = 0;
    for (int p = 0; p < 2; p++)
        for (int i = 0; i < 30; i++) test_add_to_deck_pl(g, p, filler);
}

/* Move `card` live_card_zone→waitroom as a real effect-caused zone change
 * and arm the movement batch exactly like the engine does
 * (mirrors the Rust helper `move_live_to_waitroom`). `owner_side` 1=p1 2=p2. */
static int move_live_to_waitroom(TestGame *g, int card, int owner_side, int causer)
{
    int pl = (owner_side == 1) ? 0 : 1;
    RbPlayer *P = &g->state.p[pl];
    int found = 0;
    for (int i = 0; i < P->live.n; i++) {
        if (P->live.cards[i] == card) {
            for (int j = i; j + 1 < P->live.n; j++) P->live.cards[j] = P->live.cards[j + 1];
            P->live.n--;
            found = 1;
            break;
        }
    }
    if (!found) {
        fprintf(stderr, "FAIL: test setup bug: card must start in the live card zone\n");
        failures++;
        return 0;
    }
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card;
    rb_record_card_movement(&g->state, card, RB_ZONEID_LIVE_CARD_ZONE,
                            RB_ZONEID_DISCARD, causer, 1);
    return 1;
}

/* Rust `while g.has_pending_choice() { g.select_indices(&[0]); }` — the C
 * harness's test_select_indices ignores empty selections, so answer every
 * outstanding prompt with its first option (same index 0). */
static void drain_choices(TestGame *g)
{
    int guard = 0;
    while (test_has_pending_choice(g) && guard++ < 200) test_resume_choice(g, 0);
}

/* Printed cost of a card (Rust: card_database.get_card(id).unwrap().cost). */
static int card_cost(int card_id)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return -1;
    int cost = (int)c.cost;
    rb_free_card(&c);
    return cost;
}

/* ---------------------------------------------------------------- Riko --- */

static void riko_aqours_live_leaving_live_zone_offers_deck_placement(void)
{
    TestGame game;
    test_game_new(&game);

    int riko   = test_id(&game, RIKO);
    int live   = test_id(&game, AQOURS_LIVE);
    int filler = test_id(&game, FILLER);
    REQUIRE(riko >= 0 && live >= 0 && filler >= 0, "card ids resolve");

    test_add_to_stage(&game, 1, riko);
    test_add_to_live(&game, live);
    fill_decks(&game, filler);
    int deck_before = test_deck_len(&game);

    REQUIRE(move_live_to_waitroom(&game, live, 1, 0), "live card moved out of the live zone");

    rb_trigger_auto_abilities_for_player(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);

    /* Optional deck-top/bottom offer must appear. */
    CHECK(test_has_pending_choice(&game), "Riko should be offered the deck-top/bottom placement");
    drain_choices(&game);

    /* The live card went to the deck (top or bottom) — either way out of the waitroom. */
    CHECK(test_zone_has_id(&game, 0, "deck", live),
          "Aqours live should be placed into the deck");
    CHECK_EQ(test_deck_len(&game), deck_before + 1,
             "deck grew by exactly the placed live card");
    CHECK(!test_zone_has_id(&game, 0, "waitroom", live),
          "live card must not stay in the waitroom after the placement");
}

static void riko_ignores_non_aqours_live_leaving_live_zone(void)
{
    TestGame game;
    test_game_new(&game);

    int riko     = test_id(&game, RIKO);
    int muse_live = test_id(&game, MUSE_LIVE);
    int filler   = test_id(&game, FILLER);
    REQUIRE(riko >= 0 && muse_live >= 0 && filler >= 0, "card ids resolve");

    test_add_to_stage(&game, 1, riko);
    test_add_to_live(&game, muse_live);
    fill_decks(&game, filler);

    REQUIRE(move_live_to_waitroom(&game, muse_live, 1, 0),
            "muse live moved out of the live zone");

    rb_trigger_auto_abilities_for_player(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);

    CHECK(!test_has_pending_choice(&game),
          "muse live leaving the zone must not offer Riko's placement");
    CHECK(test_zone_has_id(&game, 0, "waitroom", muse_live),
          "the muse live stays in the waitroom");
}

static void riko_responds_only_to_own_side_live_zone(void)
{
    TestGame game;
    test_game_new(&game);

    int riko_p1 = test_id(&game, RIKO);
    int riko_p2 = test_new_id(&game, RIKO);
    int live_p2 = test_new_id(&game, AQOURS_LIVE);
    int filler  = test_id(&game, FILLER);
    REQUIRE(riko_p1 >= 0 && riko_p2 >= 0 && live_p2 >= 0 && filler >= 0, "card ids resolve");

    test_add_to_stage(&game, 1, riko_p1);
    test_set_opp_stage(&game, 1, riko_p2);
    test_add_to_opp_live(&game, live_p2);
    fill_decks(&game, filler);

    REQUIRE(move_live_to_waitroom(&game, live_p2, 2, 1),
            "p2 live card moved out of p2's live zone");

    /* Scan ONLY P1: her Riko must stay silent about P2's zone. */
    rb_trigger_auto_abilities_for_player(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);
    CHECK(!test_has_pending_choice(&game),
          "P1's Riko must not react to P2's live-zone change");

    /* Scanning P2 arms HIS copy. */
    rb_trigger_auto_abilities_for_player(&game.state, 1);
    rb_process_pending_auto_abilities(&game.state);
    CHECK(test_has_pending_choice(&game),
          "P2's own Riko should react to his live leaving");
    drain_choices(&game);
    CHECK(test_zone_has_id(&game, 1, "deck", live_p2),
          "P2's Aqours live ends up in P2's deck");
}

/* --------------------------------------------------------------- Reina --- */

static void reina_fires_after_non_blade_member_left_live_zone(void)
{
    TestGame game;
    test_game_new(&game);

    int reina  = test_id(&game, REINA);
    int hanayo = test_id(&game, NO_BLADE_MEMBER);
    int filler = test_id(&game, FILLER);
    REQUIRE(reina >= 0 && hanayo >= 0 && filler >= 0, "card ids resolve");

    test_add_to_stage(&game, 1, reina);
    test_add_to_live(&game, hanayo);
    fill_decks(&game, filler);

    REQUIRE(move_live_to_waitroom(&game, hanayo, 1, 0),
            "non-blade member moved out of the live zone");

    int hand_before = test_hand_len(&game);

    /* Rust fire_trigger(reina, AbilityTrigger::LiveStart, "ライブ開始時")
     * — the C equivalent is the real live-start scan + queue drain. */
    rb_trigger_live_start(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);
    drain_choices(&game);

    CHECK_EQ(test_hand_len(&game), hand_before + 1, "draw 1 from the LiveStart ability");
    /* Rust HeartColor::Heart03/05/06 are the 0-based indices 2/4/5; the C
     * harness maps index 5 to RB_HEART_ORANGE internally (see
     * test_get_heart_modifier in src/test_game.c). */
    CHECK_EQ(test_get_heart_modifier(&game, reina, 2), 1,
             "heart03 granted until live end");
    CHECK_EQ(test_get_heart_modifier(&game, reina, 4), 1,
             "heart05 granted until live end");
    CHECK_EQ(test_get_heart_modifier(&game, reina, 5), 1,
             "heart06 granted until live end");
}

static void reina_silent_when_blade_heart_member_left_live_zone(void)
{
    TestGame game;
    test_game_new(&game);

    int reina  = test_id(&game, REINA);
    int honoka = test_id(&game, BLADE_MEMBER); /* blade=3 */
    int filler = test_id(&game, FILLER);
    REQUIRE(reina >= 0 && honoka >= 0 && filler >= 0, "card ids resolve");

    test_add_to_stage(&game, 1, reina);
    test_add_to_live(&game, honoka);
    fill_decks(&game, filler);

    REQUIRE(move_live_to_waitroom(&game, honoka, 1, 0),
            "blade-heart member moved out of the live zone");

    int hand_before = test_hand_len(&game);

    rb_trigger_live_start(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);
    drain_choices(&game);

    CHECK_EQ(test_hand_len(&game), hand_before, "blade-heart departure must NOT draw");
    CHECK_EQ(test_get_heart_modifier(&game, reina, 2), 0,
             "blade-heart departure must NOT grant hearts");
}

/* ------------------------------------------------------------- Miyamiya --- */

/* Shared setup for the three baton-touch payoffs: 愛 on the center slot, the
 * newcomer in hand, 25 energy of which 5 are left waiting. */
static int miyamiya_setup(TestGame *game, int ai, int newcomer, int filler)
{
    test_add_to_stage(game, 1, ai);
    fill_decks(game, filler);
    test_add_to_hand(game, newcomer);
    test_give_energy(game, 25);
    /* Leave 5 in wait state so the ability's "activate 2" has cards to flip. */
    test_set_energy_active(game, 0, 25 - 5);
    return test_hand_len(game);
}

static void miyamiya_baton_touch_cost15_newcomer_full_payoff(void)
{
    TestGame game;
    test_game_new(&game);

    int ai        = test_id(&game, MIYAMIYA);
    int newcomer  = test_id(&game, NEWCOMER_COST15);
    int filler    = test_id(&game, FILLER);
    REQUIRE(ai >= 0 && newcomer >= 0 && filler >= 0, "card ids resolve");

    int hand_before = miyamiya_setup(&game, ai, newcomer, filler);
    int active_before = game.state.p[0].energy_active;

    /* Playing onto the occupied center slot performs the baton touch. */
    test_play_to_stage(&game, newcomer, 1);
    drain_choices(&game);

    int ai_cost = card_cost(ai);
    int nc_cost = card_cost(newcomer);
    REQUIRE(ai_cost >= 0 && nc_cost >= 0, "printed costs decode");
    CHECK_EQ(game.state.p[0].energy_active, active_before - (nc_cost - ai_cost) + 2,
             "paid the net after baton discount, then activated 2 wait energy");
    CHECK_EQ(test_hand_len(&game), hand_before - 1 + 1,
             "newcomer left hand (-1) and the ability drew 1 (+1)");
}

static void miyamiya_baton_touch_cost13_newcomer_energy_only(void)
{
    TestGame game;
    test_game_new(&game);

    int ai        = test_id(&game, MIYAMIYA);
    int newcomer  = test_id(&game, NEWCOMER_COST13);
    int filler    = test_id(&game, FILLER);
    REQUIRE(ai >= 0 && newcomer >= 0 && filler >= 0, "card ids resolve");

    int hand_before = miyamiya_setup(&game, ai, newcomer, filler);
    int active_before = game.state.p[0].energy_active;

    test_play_to_stage(&game, newcomer, 1);
    drain_choices(&game);

    int ai_cost = card_cost(ai);
    int nc_cost = card_cost(newcomer);
    REQUIRE(ai_cost >= 0 && nc_cost >= 0, "printed costs decode");
    CHECK_EQ(game.state.p[0].energy_active, active_before - (nc_cost - ai_cost) + 2,
             "cost-13 newcomer still clears the >=10 gate: activate 2 energy");
    CHECK_EQ(test_hand_len(&game), hand_before - 1,
             "below cost 15: NO draw (newcomer just left hand)");
}

static void miyamiya_baton_touch_blade_heart_newcomer_nothing(void)
{
    TestGame game;
    test_game_new(&game);

    int ai        = test_id(&game, MIYAMIYA);
    int newcomer  = test_id(&game, NEWCOMER_BLADE_HEART);
    int filler    = test_id(&game, FILLER);
    REQUIRE(ai >= 0 && newcomer >= 0 && filler >= 0, "card ids resolve");

    int hand_before = miyamiya_setup(&game, ai, newcomer, filler);
    int active_before = game.state.p[0].energy_active;

    test_play_to_stage(&game, newcomer, 1);
    drain_choices(&game);

    int ai_cost = card_cost(ai);
    int nc_cost = card_cost(newcomer);
    REQUIRE(ai_cost >= 0 && nc_cost >= 0, "printed costs decode");
    CHECK_EQ(game.state.p[0].energy_active, active_before - (nc_cost - ai_cost),
             "blade-heart newcomer must NOT activate energy — only the net cost was paid");
    CHECK_EQ(test_hand_len(&game), hand_before - 1,
             "no draw either — only the normal play happened");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    riko_aqours_live_leaving_live_zone_offers_deck_placement();
    riko_ignores_non_aqours_live_leaving_live_zone();
    riko_responds_only_to_own_side_live_zone();
    reina_fires_after_non_blade_member_left_live_zone();
    reina_silent_when_blade_heart_member_left_live_zone();
    miyamiya_baton_touch_cost15_newcomer_full_payoff();
    miyamiya_baton_touch_cost13_newcomer_energy_only();
    miyamiya_baton_touch_blade_heart_newcomer_nothing();

    rb_unload();
    if (failures) return 1;
    printf("ALL TRIGGER PATHS ZONE CHANGE GATE CHECKS PASSED\n");
    return 0;
}
