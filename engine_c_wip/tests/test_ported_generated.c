#include "rabuka.h"
#include "deck_parser.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
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

static void drain_choices(TestGame *game)
{
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 256)
        test_resume_choice(game, -1);
}

static void generated_opponent_live_success(void)
{
    TestGame game;
    test_game_new(&game);

    int p1_member = test_id(&game, "PL!-sd1-007-SD");
    int p1_live = test_id(&game, "PL!-sd1-019-SD");
    int trapper = test_id(&game, "PL!S-pb1-021-L");
    int p2_member = test_id(&game, "PL!S-bp2-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int energy = test_id(&game, "LL-E-001-SD");
    CHECK(rb_card_no_eq(trapper, "PL!S-pb1-021-L"), "generated Trapper identity");

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = p1_member;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = p2_member;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;

    for (int i = 0; i < 12; i++) {
        game.state.p[0].deck.cards[game.state.p[0].deck.n++] = energy;
        game.state.p[1].deck.cards[game.state.p[1].deck.n++] = filler;
    }
    test_give_energy(&game, 15);
    test_add_to_hand(&game, p1_live);
    game.state.p[1].hand.cards[game.state.p[1].hand.n++] = trapper;
    rb_mods_add_heart(&game.state.mods, p2_member, 5, 4);

    for (int i = 0; i < 5; i++) {
        test_pass(&game);
        drain_choices(&game);
    }
    test_set_live_card(&game, 0, p1_live);
    test_pass(&game);
    drain_choices(&game);
    game.state.p[1].live.cards[0] = trapper;
    game.state.p[1].live.n = 1;

    for (int i = 0; i < 8; i++) {
        drain_choices(&game);
        if (game.state.p1_live_won || game.state.p2_live_won)
            break;
        test_pass(&game);
    }
    drain_choices(&game);

    CHECK(game.state.live_success[0], "generated P1 live succeeds");
    CHECK(game.state.p1_live_success_no_excess, "generated P1 live has no excess");
    CHECK(game.state.live_success[1], "generated P2 live succeeds");

    const RbLiveSnapshot *found = NULL;
    int found_live = -1;
    for (int i = 0; i < game.state.n_snapshots && !found; i++) {
        for (int j = 0; j < game.state.snapshots[i].n_lives; j++) {
            if (game.state.snapshots[i].lives[j] == trapper) {
                found = &game.state.snapshots[i];
                found_live = j;
                break;
            }
        }
    }
    CHECK(found != NULL, "generated Trapper snapshot exists");
    if (found) {
        Card card;
        CHECK(rb_decode_card_by_index((uint32_t)trapper, &card), "generated Trapper decodes");
        int base_score = (int)card.score;
        rb_free_card(&card);
        CHECK_EQ(found->live_score_detail[found_live] - base_score, 2,
                 "generated opponent live success grants +2");
    }
}

static void generated_cross_player_position(void)
{
    TestGame game;
    test_game_new(&game);

    int rurino = test_id(&game, "PL!HS-PR-029-PR");
    int himeko = test_id(&game, "PL!HS-pb1-014-R");
    int koko = test_id(&game, "PL!SP-sd2-002-P");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[0] = rurino;
    game.state.p[0].stage[1] = RB_EMPTY_SLOT;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = RB_EMPTY_SLOT;
    game.state.p[1].stage[2] = koko;
    test_add_to_hand(&game, himeko);
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&game, 0, filler);
        test_add_to_deck_pl(&game, 1, filler);
    }
    test_give_energy(&game, 15);

    test_play_to_stage(&game, himeko, 1);
    CHECK(test_has_pending_choice(&game), "generated opponent position prompt appears");
    test_resume_choice(&game, 0);
    drain_choices(&game);

    CHECK(game.state.p[1].stage[2] != koko, "generated opponent member leaves right area");
    CHECK_EQ(game.state.p[1].stage[1], koko, "generated opponent member enters front area");
    CHECK_EQ(test_get_heart_modifier(&game, koko, RB_HEART_ORANGE), 1,
             "generated opponent movement watcher fires");
}

static void generated_draw_until_count(void)
{
    TestGame game;
    test_game_new(&game);

    int card = test_id(&game, "PL!N-PR-028-PR");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, card);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 30);
    for (int i = 0; i < 10; i++)
        test_add_to_deck(&game, filler);

    test_play_to_stage(&game, card, 1);
    CHECK(test_has_pending_choice(&game), "generated optional discard prompt appears");
    rb_resume_with_choice(&game.state, 0);
    rb_drain_ability_queue(&game.state);
    if (test_has_pending_choice(&game)) {
        rb_resume_with_choice(&game.state, 1);
        rb_drain_ability_queue(&game.state);
    }
    drain_choices(&game);
    CHECK_EQ(game.state.p[0].hand.n, 5, "generated draw-until-count fills hand");
}

static void generated_target_selection(void)
{
    TestGame game;
    test_game_new(&game);

    int member_a = test_id(&game, "PL!N-PR-003-PR");
    int member_b = test_id(&game, "PL!N-PR-005-PR");
    int phoenix = test_id(&game, "PL!N-pb1-038-L");
    int stellar = test_id(&game, "PL!N-pb1-039-L");
    game.state.p[0].stage[0] = member_b;
    game.state.p[0].stage[1] = member_a;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_live(&game, phoenix);
    test_add_to_live(&game, stellar);
    game.state.stage_hearts[0][0] = 7;
    game.state.stage_hearts[0][1] = 2;
    game.state.stage_hearts[0][5] = 6;
    game.state.stage_hearts[0][7] = 10;
    game.state.phase = RB_PHASE_LIVE_SET;

    rb_trigger_live_start(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);
    while (test_has_pending_choice(&game)) {
        test_resume_choice(&game, 0);
        rb_process_pending_auto_abilities(&game.state);
    }

    int a = test_get_heart_modifier(&game, member_a, RB_HEART_PURPLE);
    int b = test_get_heart_modifier(&game, member_b, RB_HEART_PURPLE);
    CHECK_EQ(a + b, 4, "generated target selection grants total +4");
    CHECK(a == 4 || b == 4, "generated target selection grants one full +4");
}

static void generated_deck_parser(void)
{
    const char *content = "3 x PL!N-bp1-026-L\nPL!N-bp1-029-L x 2\n// comment\nPL!N-bp3-001-SEC";
    char **cards = NULL;
    size_t count = 0;
    char normalized[64];
    CHECK_EQ(rb_parse_deck_content(content, &cards, &count), 0,
             "generated deck parser accepts quantities");
    CHECK_EQ(count, 6, "generated deck parser expands quantities");
    if (count == 6) {
        CHECK(!strcmp(cards[0], "PL!N-bp1-026-L"), "generated prefix quantity order");
        CHECK(!strcmp(cards[3], "PL!N-bp1-029-L"), "generated suffix quantity order");
        CHECK(!strcmp(cards[5], "PL!N-bp3-001-SEC"), "generated bare card order");
    }
    for (size_t i = 0; i < count; i++)
        free(cards[i]);
    free(cards);
    CHECK_EQ(rb_normalize_card_no(" TEST-001-SD+ ", normalized, sizeof(normalized)), 0,
             "generated card normalization succeeds");
    CHECK(!strcmp(normalized, "TEST-001-SD"), "generated card normalization strips suffix");
}

static void generated_decode_all_cards(void)
{
    uint32_t card_count = 0;
    uint32_t ability_count = 0;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        Card card;
        if (!rb_decode_card_by_index(i, &card))
            continue;
        card_count++;
        int n = rb_card_num_abilities(i);
        for (int a = 0; a < n; a++) {
            Ability ability;
            if (rb_decode_card_ability(i, (uint32_t)a, &ability)) {
                ability_count++;
                rb_free_ability(&ability);
            }
        }
        rb_free_card(&card);
    }
    CHECK(card_count > 0, "generated card database decodes cards");
    CHECK(ability_count > 0, "generated card database decodes abilities");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    generated_opponent_live_success();
    generated_cross_player_position();
    generated_draw_until_count();
    generated_target_selection();
    generated_deck_parser();
    generated_decode_all_cards();
    rb_unload();
    if (failures)
        return 1;
    printf("ALL GENERATED CHECKS PASSED\n");
    printf("generated: 6 checks\n");
    return 0;
}
