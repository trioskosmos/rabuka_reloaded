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

static void advance_to_live_card_set_p1(TestGame *tg) {
    for (int i = 0; i < 5; i++) test_pass(tg);
}

static void clear_bag(RbBag *bag) {
    bag->n = 0;
}

static void setup_game_with_decks(TestGame *tg, const int p1_stage[3], int filler) {
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

    for (int i = 0; i < 3; i++) tg->state.p[0].stage[i] = p1_stage[i];
    tg->state.p[1].stage[0] = -1;
    tg->state.p[1].stage[1] = filler;
    tg->state.p[1].stage[2] = -1;
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

static int run_full_turn(TestGame *tg) {
    for (int i = 0; i < 30; i++) {
        if (tg->state.phase == RB_PHASE_ACTIVE && !test_has_pending_choice(tg)) return 1;
        const char *choice = test_pending_choice_type(tg);
        if (strcmp(choice, "SelectCard") == 0) {
            rb_resume_with_choice(&tg->state, -1);
        } else if (strcmp(choice, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&tg->state, 0);
        } else if (choice[0] == 0) {
            test_pass(tg);
        } else {
            fprintf(stderr, "FAIL: unexpected prompt during performance turn (expected looked_at SelectCard or none), got %s\n", choice);
            failures++;
            return 0;
        }
    }
    fprintf(stderr, "FAIL: performance turn did not return to Active\n");
    failures++;
    return 0;
}

static RbLiveSnapshot *p1_snapshot(TestGame *tg) {
    for (int i = 0; i < tg->state.n_snapshots; i++) {
        if (tg->state.snapshots[i].player == 0) return &tg->state.snapshots[i];
    }
    return NULL;
}

static void test_zero_cards_on_stage_fails_live(void) {
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    int stage[3] = {-1, -1, -1};
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card(&tg, live), "live card enters the live card set");
    run_full_turn(&tg);

    CHECK_EQ(tg.state.p[0].success.n, 0,
             "Failed live card should NOT be in success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", live),
          "Failed live card should be moved to waitroom");

    RbLiveSnapshot *snapshot = p1_snapshot(&tg);
    if (snapshot) {
        CHECK(!snapshot->success, "Performance snapshot success should be false");
        CHECK_EQ(snapshot->total_score, 0, "Failed live total score should be 0");
    }
}

static void test_sufficient_hearts_passes_live(void) {
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    int member = test_id(&tg, "PL!-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    int stage[3] = {-1, member, -1};
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card(&tg, live), "live card enters the live card set");
    run_full_turn(&tg);

    CHECK_EQ(tg.state.p[0].success.n, 1,
             "Passed live card should be in success zone");
    CHECK(tg.state.p[0].success.n > 0 && tg.state.p[0].success.cards[0] == live,
          "Success zone card should match set live card");

    RbLiveSnapshot *snapshot = p1_snapshot(&tg);
    CHECK(snapshot != NULL, "P1 snapshot should exist");
    if (snapshot) {
        CHECK(snapshot->success, "Performance snapshot success should be true");
        CHECK(snapshot->total_score > 0, "Passed live score should be > 0");
        CHECK(snapshot->live_passed[0], "Live card passed field should be true");
    }
}

static void test_per_color_modifier_does_not_erase_base_requirements(void) {
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    int member = test_id(&tg, "PL!-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    int stage[3] = {-1, member, -1};
    setup_game_with_decks(&tg, stage, filler);
    test_add_to_hand(&tg, live);

    rb_mods_set_need_heart(&tg.state.mods, live, 1, 2);

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card(&tg, live), "live card enters the live card set");
    run_full_turn(&tg);

    CHECK_EQ(tg.state.p[0].success.n, 0,
             "Card requiring 2 h01 should fail when stage only provides 1 h01");

    RbLiveSnapshot *snapshot = p1_snapshot(&tg);
    CHECK(snapshot != NULL, "P1 snapshot should exist");
    if (snapshot) {
        CHECK_EQ(snapshot->live_required[0][1], 2,
                 "h01 required should be modified to 2");
        CHECK_EQ(snapshot->live_required[0][3], 1,
                 "h03 required should remain base 1");
        CHECK_EQ(snapshot->live_required[0][6], 1,
                 "h06 required should remain base 1");
    }
}

static void setup_yell_game(TestGame *tg) {
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

    int filler = test_id(tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 40; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    int sumire = test_id(tg, "PL!SP-bp2-015-N");
    int wien = test_id(tg, "PL!SP-bp2-021-N");
    int honoka = test_id(tg, "PL!-sd1-010-SD");
    tg->state.p[0].stage[0] = sumire;
    tg->state.p[0].stage[1] = wien;
    tg->state.p[0].stage[2] = honoka;
    tg->state.p[1].stage[0] = -1;
    tg->state.p[1].stage[1] = filler;
    tg->state.p[1].stage[2] = -1;
    test_add_to_hand(tg, test_id(tg, "PL!-sd1-019-SD"));
}

static void set_yell_top(TestGame *tg, const char *const *yell_no, int n_yell) {
    for (int i = n_yell - 1; i >= 0; i--)
        test_insert_deck_top(tg, 0, test_new_id(tg, yell_no[i]));
}

static void run_yell_case(const char *const *yell_no, int n_yell, int yell_bonus, int expected) {
    TestGame tg;
    test_game_new(&tg);
    setup_yell_game(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int wien = test_id(&tg, "PL!SP-bp2-021-N");
    int live = tg.state.p[0].hand.cards[0];
    tg.state.yell_count_mod[0] = yell_bonus;

    advance_to_live_card_set_p1(&tg);
    CHECK(set_live_card(&tg, live), "yell fixture live enters the live card set");
    test_pass(&tg);
    set_yell_top(&tg, yell_no, n_yell);
    test_pass(&tg);
    CHECK_EQ(test_get_heart_modifier(&tg, sumire, RB_HEART_ORANGE), 0,
             "yell condition does not fire before a yell");
    CHECK_EQ(test_get_heart_modifier(&tg, wien, RB_HEART_GREEN), 0,
             "yell condition does not fire before a yell for Wien");
    run_full_turn(&tg);

    CHECK_EQ(test_get_heart_modifier(&tg, sumire, RB_HEART_ORANGE), expected,
             "Sumire heart06 matches the yell contents");
    CHECK_EQ(test_get_heart_modifier(&tg, wien, RB_HEART_GREEN), expected,
             "Wien heart03 matches the yell contents");
}

static void test_yell_auto_abilities_use_revealed_cards(void) {
    const char *no_blade[] = {
        "PL!S-bp2-002-R",
        "PL!S-bp2-002-R",
        "PL!S-bp2-002-R"
    };
    run_yell_case(no_blade, 3, 0, 1);
}

static void test_yell_auto_abilities_block_on_blade(void) {
    const char *with_blade[] = {
        "PL!S-bp2-002-R",
        "PL!S-bp2-002-R",
        "PL!-pb1-014-R"
    };
    run_yell_case(with_blade, 3, 0, 0);
}

static void test_yell_auto_abilities_block_on_all_blade(void) {
    const char *with_all[] = {
        "PL!S-bp2-002-R",
        "PL!S-bp2-002-R",
        "PL!HS-PR-010-PR"
    };
    run_yell_case(with_all, 3, 0, 0);
}

static void test_yell_property_scan_beyond_recent_move_limit(void) {
    const char *beyond_recent[RB_MAX_RECENTLY_MOVED + 1];
    for (int i = 0; i < RB_MAX_RECENTLY_MOVED; i++)
        beyond_recent[i] = "PL!S-bp2-002-R";
    beyond_recent[RB_MAX_RECENTLY_MOVED] = "PL!-pb1-014-R";
    run_yell_case(beyond_recent, RB_MAX_RECENTLY_MOVED + 1,
                  RB_MAX_RECENTLY_MOVED - 2, 0);
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_zero_cards_on_stage_fails_live();
    test_sufficient_hearts_passes_live();
    test_per_color_modifier_does_not_erase_base_requirements();
    test_yell_auto_abilities_use_revealed_cards();
    test_yell_auto_abilities_block_on_blade();
    test_yell_auto_abilities_block_on_all_blade();
    test_yell_property_scan_beyond_recent_move_limit();

    rb_unload();
    if (failures) return 1;
    printf("ALL PERFORMANCE PHASE RULE CHECKS PASSED\n");
    return 0;
}
