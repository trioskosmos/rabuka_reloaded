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

#define REQUIRE_EQ(a, b, msg) do { \
    int actual_ = (a); \
    int expected_ = (b); \
    if (actual_ != expected_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, actual_, expected_); \
        failures++; \
        return; \
    } \
} while (0)

static const char *orientation(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}

static void wait(TestGame *tg, int cid)
{
    rb_mods_set_orientation(&tg->state.mods, cid, "wait");
    tg->state.p[rb_owner_of_card(&tg->state, cid)].stage_wait[0] = 1;
}

static int normal_phase_seat(TestGame *tg)
{
    if (tg->state.phase == RB_PHASE_LIVE_SET ||
        tg->state.phase == RB_PHASE_PERFORMANCE ||
        tg->state.phase == RB_PHASE_VICTORY ||
        tg->state.phase == RB_PHASE_DONE) {
        return -1;
    }
    if (tg->state.active == tg->state.first_attacker) return 0;
    if (tg->state.active == tg->state.second_attacker) return 1;
    return -1;
}

static void pass(TestGame *tg)
{
    test_pass(tg);
    while (test_has_pending_choice(tg)) {
        test_select_indices(tg, NULL, 0);
    }
}

static int pass_until(TestGame *tg, int phase, int seat)
{
    int guard = 0;
    while (!(tg->state.phase == phase && normal_phase_seat(tg) == seat)) {
        guard++;
        if (guard > 24) return 0;
        pass(tg);
    }
    return 1;
}

static void fill_decks(TestGame *tg, int filler)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void fill_energy_deck(TestGame *tg, int player, int count)
{
    int energy = test_id(tg, "LL-E-001-SD");
    for (int i = 0; i < count; i++) {
        test_add_to_energy_deck(tg, player, energy);
    }
}

static int bag_contains(const RbBag *bag, int card)
{
    for (int i = 0; i < bag->n; i++) {
        if (bag->cards[i] == card) return 1;
    }
    return 0;
}

static int bag_equals_pair(const RbBag *bag, int first, int second)
{
    return bag->n == 2 && bag->cards[0] == first && bag->cards[1] == second;
}

static void test_active_phase_stands_only_the_turn_players_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    int p1_member = test_id(&tg, "PL!-sd1-010-SD");
    int p2_member = test_id(&tg, "PL!N-PR-008-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = p1_member;
    tg.state.p[1].stage[0] = p2_member;
    wait(&tg, p1_member);
    wait(&tg, p2_member);

    for (int seat = 0; seat < 2; seat++) {
        int e1 = test_id(&tg, "LL-E-001-SD");
        int e2 = test_id(&tg, "LL-E-001-SD");
        RbPlayer *player = &tg.state.p[seat];
        player->energy.cards[player->energy.n++] = e1;
        player->energy.cards[player->energy.n++] = e2;
        player->energy_active = 1;
    }

    fill_decks(&tg, filler);

    test_pass(&tg);
    REQUIRE_EQ(tg.state.phase, RB_PHASE_ACTIVE,
               "one pass reaches the second attacker's Active phase");
    REQUIRE_EQ(normal_phase_seat(&tg), 1,
               "after one pass from first normal phase the second attacker acts");

    test_pass(&tg);
    REQUIRE_EQ(tg.state.phase, RB_PHASE_ENERGY,
               "one more pass reaches the second attacker's Energy phase");

    const char *p2_orientation = orientation(&tg, p2_member);
    REQUIRE(p2_orientation == NULL || strcmp(p2_orientation, "wait") != 0,
            "7.4.1: the turn player's waited member stands");
    REQUIRE(orientation(&tg, p1_member) != NULL &&
                strcmp(orientation(&tg, p1_member), "wait") == 0,
            "the non-turn player's waited member must stay waited");
    REQUIRE_EQ(tg.state.p[1].energy_active, 2,
               "turn player's waited energy stands");
    REQUIRE_EQ(tg.state.p[0].energy_active, 1,
               "non-turn player's waited energy stays put");
}

static void test_energy_phase_draws_one_and_empty_deck_skips(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    fill_energy_deck(&tg, 0, 1);
    int seeded = tg.state.p[0].energy_deck.cards[0];
    fill_decks(&tg, filler);

    test_pass(&tg);
    test_pass(&tg);
    REQUIRE_EQ(normal_phase_seat(&tg), 1,
               "the second attacker reaches the Energy phase");
    REQUIRE_EQ(tg.state.p[1].energy_deck.n, 0,
               "precondition: their energy deck is empty");
    REQUIRE_EQ(tg.state.p[1].energy.n, 0,
               "empty energy deck places nothing and does not panic");

    REQUIRE(pass_until(&tg, RB_PHASE_ENERGY, 0),
            "the first attacker's Energy phase is reached");
    test_pass(&tg);

    REQUIRE_EQ(tg.state.p[0].energy.n, 1,
               "7.5.2: the turn player's Energy phase moves her top deck card");
    REQUIRE(bag_contains(&tg.state.p[0].energy, seeded),
            "the seeded card was moved in top-of-deck order");
}

static void test_draw_phase_on_empty_main_deck_refreshes_then_draws(void)
{
    TestGame tg;
    test_game_new(&tg);

    tg.state.p[0].deck.n = 0;
    tg.state.p[0].discard.n = 0;
    for (int i = 0; i < 3; i++) {
        int card = test_id(&tg, "PL!-sd1-010-SD");
        tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = card;
    }
    int f = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&tg, 1, f);
    }

    REQUIRE(pass_until(&tg, RB_PHASE_ENERGY, 0),
            "the first attacker's Energy phase is reached before the draw");
    int hand_before = tg.state.p[0].hand.n;

    REQUIRE(pass_until(&tg, RB_PHASE_MAIN, 0),
            "the first attacker's Main phase is reached after the draw");
    REQUIRE_EQ(tg.state.p[0].hand.n, hand_before + 1,
               "7.6.2: exactly one card drawn despite the empty deck");
    REQUIRE_EQ(tg.state.p[0].deck.n, 2,
               "3 waitroom cards shuffled in and 1 drawn out");
}

static void test_live_card_set_refill_draws_placed_count(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live1 = test_id(&tg, "PL!-sd1-020-SD");
    int live2 = test_id(&tg, "PL!-sd1-020-SD");

    fill_decks(&tg, filler);
    test_add_to_hand(&tg, live1);

    int guard = 0;
    while (tg.state.phase != RB_PHASE_LIVE_SET && guard < 12) {
        guard++;
        pass(&tg);
    }

    test_set_live_card(&tg, 0, live1);
    if (tg.state.p[0].live.n < RB_MAX_ZONE) {
        tg.state.p[0].live.cards[tg.state.p[0].live.n++] = live2;
    }

    int top_b = test_id(&tg, "PL!-sd1-010-SD");
    int top_a = test_id(&tg, "PL!-sd1-010-SD");
    test_insert_deck_top(&tg, 0, top_b);
    test_insert_deck_top(&tg, 0, top_a);

    int p2_hand_before = tg.state.p[1].hand.n;

    test_pass(&tg);

    REQUIRE(bag_equals_pair(&tg.state.p[0].hand, top_a, top_b),
            "8.2.2: refill equals placed count 2 and uses deck top order");

    int p2_hand_at_refill = tg.state.p[1].hand.n;
    REQUIRE_EQ(p2_hand_before, p2_hand_at_refill,
               "the second attacker's boundary refills zero cards");
    test_pass(&tg);
    REQUIRE_EQ(tg.state.p[1].hand.n, p2_hand_at_refill,
               "placed 0 live cards means 0 refill draws for the second attacker");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_active_phase_stands_only_the_turn_players_cards();
    test_energy_phase_draws_one_and_empty_deck_skips();
    test_draw_phase_on_empty_main_deck_refreshes_then_draws();
    test_live_card_set_refill_draws_placed_count();

    rb_unload();
    if (failures) return 1;
    printf("ALL PHASE MACHINE RULE CHECKS PASSED\n");
    return 0;
}
