#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static int drain_prompts(TestGame *tg) {
    int guard = 0;
    while (test_has_pending_choice(tg) && guard < 40) {
        const char *type = test_pending_choice_type(tg);
        if (strcmp(type, "SelectAutoAbility") == 0) {
            guard++;
            test_select_indices(tg, NULL, 0);
            continue;
        }
        const RbChoice *choice = rb_get_pending_choice(&tg->state);
        if (strcmp(type, "SelectCard") == 0 && choice &&
            !strcmp(choice->zone, "live_card_zone") &&
            !strcmp(choice->target, "select_live_success")) {
            guard++;
            test_resume_choice(tg, 0);
            continue;
        }
        return 0;
    }
    return !test_has_pending_choice(tg);
}

static void setup_bellas(TestGame *tg, int keep_second) {
    int bella = test_id(tg, "PL!N-bp3-027-L");
    int emma = test_id(tg, "PL!N-pb1-008-R");
    int ayumu = test_id(tg, "PL!N-PR-003-PR");
    int hasu = test_id(tg, "PL!HS-pb1-023-N");
    int filler = test_id(tg, "PL!-sd1-010-SD");
    int energy = test_id(tg, "LL-E-001-SD");

    for (int i = 0; i < 15; i++) {
        test_add_to_deck(tg, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    for (int i = 0; i < 3; i++) test_add_to_energy_deck(tg, 0, energy);

    tg->state.p[0].stage[0] = emma;
    tg->state.p[0].stage[1] = ayumu;
    tg->state.p[0].stage[2] = hasu;
    test_add_to_hand(tg, bella);
    if (keep_second) test_add_to_hand(tg, bella);
}

static int set_live_card(TestGame *tg, int card_id) {
    RbPlayer *player = &tg->state.p[0];
    for (int i = 0; i < player->hand.n; i++) {
        if (player->hand.cards[i] != card_id) continue;
        int card = rb_hand_remove_card(player, i);
        return card >= 0 && rb_live_add_card(player, card) == 0;
    }
    return 0;
}

static int drive_past_placement(TestGame *tg, int want) {
    for (int i = 0; i < 14; i++) {
        if (!test_has_pending_choice(tg)) {
            test_pass(tg);
            if (!drain_prompts(tg)) return 0;
            if (tg->state.p[0].energy.n >= want &&
                tg->state.phase == RB_PHASE_ACTIVE && tg->state.turn > 1) return 1;
        } else if (!drain_prompts(tg)) {
            return 0;
        }
    }
    return 0;
}

static void bella_q173_two_lives_succeed_both_trigger_waited_placement(void) {
    TestGame tg;
    test_game_new(&tg);
    setup_bellas(&tg, 1);

    for (int i = 0; i < 5; i++) test_pass(&tg);

    int h0 = tg.state.p[0].hand.cards[0];
    int h1 = tg.state.p[0].hand.cards[1];
    CHECK(tg.state.p[0].hand.n >= 2, "two Bella copies are in hand");
    CHECK(rb_card_no_eq(h0, "PL!N-bp3-027-L") &&
          rb_card_no_eq(h1, "PL!N-bp3-027-L"), "both hand entries are Bella");
    CHECK(set_live_card(&tg, h0), "first Bella enters the live set");
    CHECK(set_live_card(&tg, h1), "second Bella enters the live set");
    CHECK(drive_past_placement(&tg, 2), "driver reaches the post-rollover placement window");

    CHECK_EQ(tg.state.p[0].energy.n, 2, "both successful Bella lives place one energy each");
    CHECK_EQ(tg.state.p[0].energy_deck.n, 1, "3 seeded minus 2 placed leaves exactly 1 energy in deck");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0, "Q173 energy must be placed in WAIT state");
}

static void bella_q173_single_life_places_single_waited_energy(void) {
    TestGame tg;
    test_game_new(&tg);
    setup_bellas(&tg, 0);

    for (int i = 0; i < 5; i++) test_pass(&tg);

    int h0 = tg.state.p[0].hand.cards[0];
    CHECK(rb_card_no_eq(h0, "PL!N-bp3-027-L"), "single hand entry is Bella");
    CHECK(set_live_card(&tg, h0), "Bella enters the live set");
    CHECK(drive_past_placement(&tg, 1), "driver reaches the post-rollover placement window");

    CHECK_EQ(tg.state.p[0].energy.n, 1, "one successful life places exactly one energy");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0, "single placement is also WAITED");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    bella_q173_two_lives_succeed_both_trigger_waited_placement();
    bella_q173_single_life_places_single_waited_energy();
    rb_unload();
    if (failures) return 1;
    printf("ALL SURPLUS HEART CHECKS PASSED\n");
    return 0;
}
