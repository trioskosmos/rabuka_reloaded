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

static int base_heart02(const TestGame *game, int card_id) {
    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return 0;
    int count = 0;
    for (int i = 0; i < card.num_base && i < card.n_hearts; i++) {
        if (card.heart_color[i] == RB_HEART_YELLOW) count += card.heart_count[i];
    }
    rb_free_card(&card);
    return count;
}

static void fill_both_decks(TestGame *game, int filler) {
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(game, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static void drain_auto_choices(TestGame *game) {
    while (test_has_pending_choice(game)) {
        if (strcmp(test_pending_choice_type(game), "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&game->state, -1);
    }
}

static int set_live_card(TestGame *game, int card_id) {
    RbPlayer *player = &game->state.p[game->state.active];
    for (int i = 0; i < player->hand.n; i++) {
        if (player->hand.cards[i] != card_id) continue;
        int card = rb_hand_remove_card(player, i);
        return card >= 0 && rb_live_add_card(player, card) == 0;
    }
    return 0;
}

static int snapshot_has_score_source(const TestGame *game) {
    for (int i = 0; i < game->state.n_snapshots; i++) {
        const RbLiveSnapshot *snapshot = &game->state.snapshots[i];
        if (snapshot->total_score <= 0) continue;
        for (int k = 0; k < rb_mods_trace_len(&game->state.mods); k++) {
            const RbAbilityTraceEntry *entry = &game->state.mods.trace[k];
            if (entry->effect_type != RB_EFFECT_SCORE_BONUS &&
                entry->effect_type != RB_EFFECT_SCORE_SET) continue;
            for (int live = 0; live < snapshot->n_lives; live++) {
                if (snapshot->lives[live] == entry->target_card_id) return 1;
            }
        }
    }
    return 0;
}

static void opponent_cost_self_higher_scores_in_snapshot(void) {
    TestGame game;
    test_game_new(&game);

    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int self_center = test_id(&game, "PL!SP-pb1-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = filler;
    game.state.p[0].stage[1] = self_center;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].stage[0] = filler;
    game.state.p[1].stage[1] = test_id(&game, "PL!-sd1-010-SD");
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);

    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, nonfiction);
    test_pass(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    test_pass(&game);

    CHECK(snapshot_has_score_source(&game),
          "Self cost=11 > Opponent cost=3 → snapshot should contain +1 score");
}

static void opponent_cost_self_lower_no_score_in_snapshot(void) {
    TestGame game;
    test_game_new(&game);

    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int opp_center = test_id(&game, "PL!SP-pb1-001-R");

    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = filler;
    game.state.p[0].stage[1] = filler;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].stage[0] = filler;
    game.state.p[1].stage[1] = opp_center;
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);

    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, nonfiction);
    test_pass(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    test_pass(&game);

    CHECK(!snapshot_has_score_source(&game),
          "Self cost=3 < Opponent cost=11 → snapshot should NOT contain score line");
}

static void left_side_heart_meets_threshold_gains_blade(void) {
    TestGame game;
    test_game_new(&game);

    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int left_high = test_id(&game, "PL!SP-pb1-001-R");
    int center_high = test_new_id(&game, "PL!SP-pb1-001-R");
    int right_high = test_new_id(&game, "PL!SP-pb1-001-R");
    CHECK_EQ(base_heart02(&game, left_high), 4, "left_high heart02 is 4");
    CHECK_EQ(base_heart02(&game, center_high), 4, "center_high heart02 is 4");
    CHECK_EQ(base_heart02(&game, right_high), 4, "right_high heart02 is 4");

    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = left_high;
    game.state.p[0].stage[1] = center_high;
    game.state.p[0].stage[2] = right_high;
    fill_both_decks(&game, left_high);

    int left_blade_before = test_get_blade_modifier(&game, left_high);
    int center_blade_before = test_get_blade_modifier(&game, center_high);
    int right_blade_before = test_get_blade_modifier(&game, right_high);

    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, nonfiction);
    test_pass(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    test_pass(&game);

    CHECK_EQ(test_get_blade_modifier(&game, left_high) - left_blade_before, 2,
             "Left_side heart02=4 >= 3 → should gain +2 blade");
    CHECK_EQ(test_get_blade_modifier(&game, center_high), center_blade_before,
             "Center member must NOT gain blade — only left_side qualifies");
    CHECK_EQ(test_get_blade_modifier(&game, right_high), right_blade_before,
             "Right member must NOT gain blade — only left_side qualifies");
}

static void left_side_heart_below_ignores_center_high(void) {
    TestGame game;
    test_game_new(&game);

    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int left_low = test_id(&game, "PL!SP-bp2-006-R＋");
    int center_high = test_id(&game, "PL!SP-pb1-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    CHECK_EQ(base_heart02(&game, left_low), 1, "left_low heart02 is 1");
    CHECK_EQ(base_heart02(&game, center_high), 4, "center_high heart02 is 4");

    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = left_low;
    game.state.p[0].stage[1] = center_high;
    game.state.p[0].stage[2] = filler;
    fill_both_decks(&game, filler);

    int blade_before = test_get_blade_modifier(&game, left_low);

    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, nonfiction);
    test_pass(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    test_pass(&game);

    CHECK_EQ(test_get_blade_modifier(&game, left_low), blade_before,
             "Left_side heart02=1 < 3, total stage=5 → position filter must reject");
}

static void blade_goes_to_stage_member_not_live_card(void) {
    TestGame game;
    test_game_new(&game);

    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int left_high = test_id(&game, "PL!SP-pb1-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int deck_filler = test_new_id(&game, "PL!-sd1-010-SD");

    CHECK_EQ(base_heart02(&game, left_high), 4, "left_high heart02 is 4");

    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = left_high;
    game.state.p[0].stage[1] = filler;
    game.state.p[0].stage[2] = filler;
    fill_both_decks(&game, deck_filler);

    int live_blade_before = test_get_blade_modifier(&game, nonfiction);
    int member_blade_before = test_get_blade_modifier(&game, left_high);

    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, nonfiction);
    test_pass(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    test_pass(&game);

    CHECK_EQ(test_get_blade_modifier(&game, nonfiction), live_blade_before,
             "Live card must NOT receive blade from position effect");
    CHECK_EQ(test_get_blade_modifier(&game, left_high) - member_blade_before, 2,
             "Left_side stage member must receive exactly +2 blade");
}

static void both_players_get_own_live_start_effects(void) {
    TestGame game;
    test_game_new(&game);

    int p1_card = test_id(&game, "PL!SP-bp4-024-L");
    int p2_card = test_new_id(&game, "PL!SP-bp4-024-L");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int p_left = test_id(&game, "PL!SP-pb1-001-R");
    int p_center = test_new_id(&game, "PL!SP-pb1-001-R");
    int p2_left = test_new_id(&game, "PL!SP-pb1-001-R");
    int p2_center = test_new_id(&game, "PL!SP-pb1-001-R");

    test_add_to_hand(&game, p1_card);
    game.state.p[0].stage[0] = p_left;
    game.state.p[0].stage[1] = p_center;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].hand.cards[game.state.p[1].hand.n++] = p2_card;
    game.state.p[1].stage[0] = p2_left;
    game.state.p[1].stage[1] = p2_center;
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);

    int p1_blade_before = test_get_blade_modifier(&game, p_left);
    int p2_blade_before = test_get_blade_modifier(&game, p2_left);

    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, p1_card);
    test_pass(&game);
    game.state.active = 1;
    set_live_card(&game, p2_card);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game);
    drain_auto_choices(&game);

    CHECK_EQ(test_get_blade_modifier(&game, p_left) - p1_blade_before, 2,
             "P1 left_side should gain +2 blade from its own LiveStart");
    CHECK_EQ(test_get_blade_modifier(&game, p2_left) - p2_blade_before, 2,
             "P2 left_side should gain +2 blade from its own LiveStart");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    opponent_cost_self_higher_scores_in_snapshot();
    opponent_cost_self_lower_no_score_in_snapshot();
    left_side_heart_meets_threshold_gains_blade();
    left_side_heart_below_ignores_center_high();
    blade_goes_to_stage_member_not_live_card();
    both_players_get_own_live_start_effects();
    rb_unload();
    if (failures) return 1;
    printf("ALL CONDITION EVALUATION CHECKS PASSED\n");
    return 0;
}
