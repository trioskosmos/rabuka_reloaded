#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>

#define P1_MEMBER "PL!-sd1-007-SD"
#define P1_LIVE "PL!-sd1-019-SD"
#define TRAPPER "PL!S-pb1-021-L"
#define P2_MEMBER "PL!S-bp2-001-R"

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

static void select_empty_choices(TestGame *tg)
{
    while (test_has_pending_choice(tg)) {
        test_resume_choice(tg, 0);
    }
}

static void set_opponent_live_card(TestGame *tg, int card_id)
{
    RbPlayer *player = &tg->state.p[1];
    player->live.cards[0] = card_id;
    if (player->live.n < 1) {
        player->live.n = 1;
    }
}

static void dump_snapshots(const TestGame *tg)
{
    for (int i = 0; i < tg->state.n_snapshots; i++) {
        const RbLiveSnapshot *snapshot = &tg->state.snapshots[i];
        fprintf(stderr, "player=%d lives=", snapshot->player);
        for (int j = 0; j < snapshot->n_lives; j++) {
            fprintf(stderr, "%s%d", j == 0 ? "" : ",", snapshot->lives[j]);
        }
        fprintf(stderr, "\n");
    }
}

static void p2_owned_trapper_scores_from_p1_success_in_real_round(void)
{
    TestGame tg;
    test_game_new(&tg);

    int p1_member = test_id(&tg, P1_MEMBER);
    int p1_live = test_id(&tg, P1_LIVE);
    int trapper = test_id(&tg, TRAPPER);
    int p2_member = test_id(&tg, P2_MEMBER);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int energy = test_id(&tg, "LL-E-001-SD");

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = p1_member;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[1] = p2_member;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;

    for (int i = 0; i < 12; i++) {
        tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = energy;
        tg.state.p[1].deck.cards[tg.state.p[1].deck.n++] = filler;
    }
    test_give_energy(&tg, 15);
    tg.state.p[0].hand.cards[tg.state.p[0].hand.n++] = p1_live;
    tg.state.p[1].hand.cards[tg.state.p[1].hand.n++] = trapper;
    rb_mods_add_heart(&tg.state.mods, p2_member, 5, 4);

    for (int i = 0; i < 5; i++) {
        test_pass(&tg);
        select_empty_choices(&tg);
    }
    test_set_live_card(&tg, 0, p1_live);

    test_pass(&tg);
    select_empty_choices(&tg);
    set_opponent_live_card(&tg, trapper);

    for (int i = 0; i < 8; i++) {
        select_empty_choices(&tg);
        if (tg.state.p1_live_won || tg.state.p2_live_won) {
            break;
        }
        test_pass(&tg);
    }
    select_empty_choices(&tg);

    int p1_won = tg.state.live_success[0];
    int p1_no_excess = tg.state.p1_live_success_no_excess;
    int p2_won = tg.state.live_success[1];
    CHECK(p1_won, "P1 must have succeeded (exact-fill live)");
    CHECK(p1_no_excess, "P1 succeeded WITHOUT surplus hearts (exact fill)");
    CHECK(p2_won, "P2 must have succeeded as well");

    for (int i = 0; i < 3; i++) {
        if (!test_has_pending_choice(&tg)) {
            test_pass(&tg);
        }
        select_empty_choices(&tg);
    }

    const RbLiveSnapshot *found = NULL;
    int found_live = -1;
    for (int i = 0; i < tg.state.n_snapshots && !found; i++) {
        for (int j = 0; j < tg.state.snapshots[i].n_lives; j++) {
            if (tg.state.snapshots[i].lives[j] == trapper) {
                found = &tg.state.snapshots[i];
                found_live = j;
                break;
            }
        }
    }
    if (!found) {
        fprintf(stderr, "FAIL: Trapper live missing from snapshots; have:\n");
        dump_snapshots(&tg);
        failures++;
        return;
    }

    Card card;
    if (!rb_decode_card_by_index((uint32_t)trapper, &card)) {
        CHECK(0, "Trapper card can be decoded for base score");
        return;
    }
    int base_score = (int)card.score;
    rb_free_card(&card);
    CHECK_EQ(found->live_score_detail[found_live] - base_score, 2,
             "P2-owned Trapper must gain +2 from P1's no-excess success");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    p2_owned_trapper_scores_from_p1_success_in_real_round();
    rb_unload();
    if (failures) return 1;
    printf("ALL OPPONENT LIVE SUCCESS FLOW CHECKS PASSED\n");
    return 0;
}
