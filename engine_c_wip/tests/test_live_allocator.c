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

static void clear_bag(RbBag *bag)
{
    bag->n = 0;
}

static void clear_game_cards(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        clear_bag(&tg->state.p[pl].deck);
        clear_bag(&tg->state.p[pl].hand);
        clear_bag(&tg->state.p[pl].discard);
        clear_bag(&tg->state.p[pl].live);
        clear_bag(&tg->state.p[pl].success);
        clear_bag(&tg->state.p[pl].energy);
        clear_bag(&tg->state.p[pl].energy_deck);
    }
}

static void add_deck_top(TestGame *tg, int pl, int card_id)
{
    RbPlayer *player = &tg->state.p[pl];
    if (player->deck.n >= RB_MAX_ZONE) return;
    for (int i = player->deck.n; i > 0; i--)
        player->deck.cards[i] = player->deck.cards[i - 1];
    player->deck.cards[0] = card_id;
    player->deck.n++;
}

static RbLiveSnapshot *latest_snapshot(TestGame *tg, int pl)
{
    for (int i = tg->state.n_snapshots - 1; i >= 0; i--)
        if (tg->state.snapshots[i].player == pl)
            return &tg->state.snapshots[i];
    return NULL;
}

static RbLiveSnapshot *perform_batch(TestGame *tg, int pl)
{
    tg->state.live_batch_mode = 1;
    rb_perform_live(&tg->state, pl);
    tg->state.live_batch_mode = 0;
    return latest_snapshot(tg, pl);
}

static void test_zero_blade_does_not_reveal(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_game_cards(&tg);
    int live = test_id(&tg, "PL!HS-bp1-019-L");
    int first = test_id(&tg, "PL!N-bp7-030-L");
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, first);
    int before = tg.state.p[0].deck.n;
    RbLiveSnapshot *snapshot = perform_batch(&tg, 0);
    CHECK(snapshot != NULL, "zero-blade performance creates a snapshot");
    if (snapshot) {
        CHECK_EQ(snapshot->n_yell_cards, 0, "zero effective blades reveals no card");
        CHECK_EQ(tg.state.p[0].deck.n, before, "zero-blade yell does not consume deck");
    }
}

static void test_b_heart07_doubles_and_restricts(void)
{
    for (int mode = 0; mode < 3; mode++) {
        TestGame tg;
        test_game_new(&tg);
        clear_game_cards(&tg);
        int live = test_id(&tg, mode == 1 ? "PL!-sd1-020-SD" : "PL!HS-bp1-019-L");
        int member = test_id(&tg, mode == 2 ? "PL!-bp4-012-N" : "PL!SP-pb1-014-PR");
        int bheart = test_id(&tg, "PL!N-bp7-030-L");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        test_add_to_stage(&tg, 1, member);
        test_add_to_live(&tg, live);
        for (int i = 0; i < 8; i++) add_deck_top(&tg, 0, filler);
        add_deck_top(&tg, 0, bheart);
        add_deck_top(&tg, 0, bheart);
        RbLiveSnapshot *snapshot = perform_batch(&tg, 0);
        CHECK(snapshot != NULL, "b_heart07 performance creates a snapshot");
        if (!snapshot) continue;
        CHECK_EQ(snapshot->n_yell_cards, 2, "two blades reveal two yell cards");
        CHECK_EQ(snapshot->yell_blade_hearts[0], 4, "each b_heart07 doubles into colorless hearts");
        CHECK_EQ(snapshot->total_hearts[0] >= 4, 1, "b_heart07 contributes colorless pool");
        if (mode == 0) {
            CHECK(snapshot->live_passed[0], "colorless hearts satisfy a heart00-only live");
        } else if (mode == 1) {
            CHECK(!snapshot->live_passed[0], "colorless hearts cannot fill colored requirements");
        } else {
            CHECK(snapshot->live_passed[0], "colored stage hearts plus colorless hearts pass");
        }
    }
}

static void test_insufficient_b_heart07_fails(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_game_cards(&tg);
    int live = test_id(&tg, "PL!HS-bp1-019-L");
    int member = test_id(&tg, "PL!-sd1-002-SD");
    int bheart = test_id(&tg, "PL!N-bp7-030-L");
    test_add_to_stage(&tg, 1, member);
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, bheart);
    RbLiveSnapshot *snapshot = perform_batch(&tg, 0);
    CHECK(snapshot != NULL, "one-blade performance creates a snapshot");
    if (snapshot) {
        CHECK_EQ(snapshot->total_hearts[0], 2, "one b_heart07 gives two colorless hearts");
        CHECK(!snapshot->live_passed[0], "two colorless hearts do not satisfy heart00 four");
    }
}

static void test_yell_order_and_waited_state(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_game_cards(&tg);
    int live = test_id(&tg, "PL!HS-bp1-019-L");
    int member = test_id(&tg, "PL!-sd1-002-SD");
    int first = test_id(&tg, "PL!N-bp7-030-L");
    int second = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_stage(&tg, 1, member);
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, second);
    add_deck_top(&tg, 0, first);
    RbLiveSnapshot *ordered = perform_batch(&tg, 0);
    CHECK(ordered != NULL, "deck-top yell creates a snapshot");
    if (ordered) CHECK_EQ(ordered->yell_cards[0], first, "deck-top yell draws index zero first");

    clear_game_cards(&tg);
    test_add_to_stage(&tg, 1, member);
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, first);
    tg.state.p[0].stage_wait[1] = 1;
    RbLiveSnapshot *waited = perform_batch(&tg, 0);
    CHECK(waited != NULL, "waited-member performance creates a snapshot");
    if (waited) CHECK_EQ(waited->n_yell_cards, 0, "waited member contributes no yell");
}

static void test_structured_yell_modifier(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_game_cards(&tg);
    int live = test_id(&tg, "PL!HS-bp1-019-L");
    int member = test_id(&tg, "PL!-sd1-002-SD");
    int card = test_id(&tg, "PL!N-bp7-030-L");
    test_add_to_stage(&tg, 1, member);
    tg.state.p[0].stage_wait[1] = 1;
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, card);
    rb_add_yell_count_modifier(&tg.state, 1, 1);
    RbLiveSnapshot *snapshot = perform_batch(&tg, 0);
    CHECK(snapshot != NULL, "structured yell modifier creates a snapshot");
    if (snapshot) CHECK_EQ(snapshot->n_yell_cards, 1, "structured yell modifier adds one reveal");
}

static void test_backtracking_and_wildcards(void)
{
    int pool[8] = {0, 0, 0, 0, 0, 0, 0, 2};
    int needs[8] = {0, 1, 0, 0, 0, 0, 0, 0};
    int allocations[64];
    for (int i = 0; i < 64; i++) allocations[i] = -1;
    CHECK(rb_backtrack_allocate(pool, needs, 1, allocations, 64),
          "backtracking finds an all-heart allocation");
    int filled[8] = {0};
    for (int i = 0; i < 64 && allocations[i] >= 0; i++) {
        int entry = allocations[i];
        filled[entry % 8]++;
    }
    CHECK(rb_card_ok_with_wildcard(filled, needs), "all-heart allocation passes wildcard check");
    int colored_missing[8] = {0, 1, 0, 0, 0, 0, 0, 0};
    int colorless_only[8] = {2, 0, 0, 0, 0, 0, 0, 0};
    CHECK(!rb_card_ok_with_wildcard(colorless_only, colored_missing),
          "colorless hearts cannot satisfy a colored requirement");
    int all_only[8] = {0, 0, 0, 0, 0, 0, 0, 1};
    CHECK(rb_card_ok_with_wildcard(all_only, colored_missing),
          "all hearts satisfy a colored requirement");
}

static void test_score_icon_yell_increments_live_score(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_game_cards(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    int member = test_id(&tg, "PL!-sd1-001-SD");
    int revealed = test_id(&tg, "PL!-sd1-019-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_stage(&tg, 1, member);
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, filler);
    add_deck_top(&tg, 0, revealed);
    RbLiveSnapshot *snapshot = perform_batch(&tg, 0);
    CHECK(snapshot != NULL, "score-icon performance creates a snapshot");
    if (snapshot) {
        CHECK(snapshot->n_yell_cards >= 1, "score-icon card is revealed by yell");
        CHECK_EQ(snapshot->note_icons, 1, "score icon is counted once");
        CHECK_EQ(snapshot->total_score, 2, "score icon adds one to live score");
    }
}

static void test_batch_yell_cards_are_conserved(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_game_cards(&tg);
    int live = test_id(&tg, "PL!HS-bp1-019-L");
    int member = test_id(&tg, "PL!-sd1-002-SD");
    int yelled = test_id(&tg, "PL!N-bp7-030-L");
    test_add_to_stage(&tg, 1, member);
    test_add_to_live(&tg, live);
    add_deck_top(&tg, 0, yelled);
    RbLiveSnapshot *snapshot = perform_batch(&tg, 0);
    CHECK(snapshot != NULL, "batch yell creates a snapshot");
    if (snapshot) {
        CHECK_EQ(tg.state.p[0].discard.n, 1, "batch yell card is moved to waitroom");
        CHECK_EQ(tg.state.p[0].live.n, 1, "batch mode leaves live card in live zone");
    }
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_zero_blade_does_not_reveal();
    test_b_heart07_doubles_and_restricts();
    test_insufficient_b_heart07_fails();
    test_yell_order_and_waited_state();
    test_structured_yell_modifier();
    test_backtracking_and_wildcards();
    test_score_icon_yell_increments_live_score();
    test_batch_yell_cards_are_conserved();
    rb_unload();
    if (failures) return 1;
    printf("ALL LIVE ALLOCATION CHECKS PASSED\n");
    return 0;
}
