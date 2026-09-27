/* test_parity_zones_position.c
 *
 * C parity suite for the Rust position/area-move test cluster:
 *   engine/tests/test_modules/effects/position/area_move/
 *   engine/tests/test_modules/effects/position/per_card/
 *
 * Two layers:
 *   1. Unit parity for the position/zone surface that src/core/zones.c owns
 *      (engine/src/core/zones.rs: MemberArea, check_trigger_position,
 *      check_effect_position, Stage area accessors, position_change /
 *      formation_change, recycle_under_cards, can_place_card).
 *   2. End-to-end parity for the position_change / formation_change shapes the
 *      Rust card tests exercise (mill_three, chisato rotation, himeno front
 *      area, wien center rotation, live-start formation, tomari energy return).
 *
 * This file is deliberately pure ASCII: the two Japanese trigger keywords are
 * spelled as explicit UTF-8 byte escapes so the file survives any tool that
 * rewrites sources in a legacy code page.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s  [%s:%d]\n", message, __FILE__, __LINE__); \
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
        fprintf(stderr, "FAIL: %s (got %d expected %d)  [%s:%d]\n", message, \
                actual_value, expected_value, __FILE__, __LINE__); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* zones.c owns these but they are not yet surfaced in rabuka.h; declare them
 * locally the same way src/turn/triggers.c does. */
int rb_check_effect_position(const char *effect_pos, int card_position);
int rb_check_trigger_position(const char *triggers, int card_position);
int rb_member_area_front(int area);
int rb_member_area_from_index(int idx);
int rb_stage_get_area(const int stage[RB_STAGE_SIZE], int area);
void rb_stage_set_area(int stage[RB_STAGE_SIZE], int area, int card_id);
int rb_stage_position_change(int stage[RB_STAGE_SIZE], int from_area, int to_area);

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* Japanese trigger keywords scanned by zones.rs::check_trigger_position. */
#define TRIG_LEFT   "\xE5\xB7\xA6\xE3\x82\xB5\xE3\x82\xA4\xE3\x83\x89"                    /* left side   */
#define TRIG_RIGHT  "\xE5\x8F\xB3\xE3\x82\xB5\xE3\x82\xA4\xE3\x83\x89"                    /* right side  */
#define TRIG_CENTER "\xE3\x83\x9B\xE3\x83\xB3\xE3\x82\xBF\xE3\x83\xBC"                    /* center      */
#define TRIG_BOTH   TRIG_LEFT "\xE3\x81\xA8" TRIG_RIGHT                                   /* left and right */
#define TRIG_LIVE   "live_success"

/* Mirrors Rust GameState::has_card_moved_this_turn. */
static int moved_this_turn(const TestGame *tg, int card_id)
{
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 0;
    return tg->state.moved_this_turn[card_id];
}

/* ---- helpers -------------------------------------------------------- */

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    P->hand.n = 0;
    P->discard.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) test_add_to_deck(tg, filler);
    for (int i = 0; i < n / 2; i++) test_add_to_deck_pl(tg, 1, filler);
}

static void set_p2_stage(TestGame *tg, int left, int center, int right)
{
    tg->state.p[1].stage[AREA_LEFT] = left;
    tg->state.p[1].stage[AREA_CENTER] = center;
    tg->state.p[1].stage[AREA_RIGHT] = right;
}

/* Answer the pending choice with option `idx` (the first one that appears) and
 * drain the follow-up auto-ability choices, like the Rust test helpers. */
static void answer(TestGame *tg, int idx, int max_rounds)
{
    for (int guard = 0; guard < max_rounds; guard++) {
        if (!rb_has_pending_choice(&tg->state)) return;
        rb_resume_with_choice(&tg->state, idx);
    }
}

static void drain_all(TestGame *tg, int max_rounds)
{
    for (int guard = 0; guard < max_rounds; guard++) {
        if (!rb_has_pending_choice(&tg->state)) return;
        rb_resume_with_choice(&tg->state, 0);
    }
}

/* ---- 1. MemberArea label / index / tag surface (zones.rs:64-152) ------ */

static void test_member_area_labels(void)
{
    CHECK_EQ(rb_member_area_to_index("left"), AREA_LEFT,
             "MemberArea::from_str(\"left\") maps to index 0");
    CHECK_EQ(rb_member_area_to_index("center"), AREA_CENTER,
             "MemberArea::from_str(\"center\") maps to index 1");
    CHECK_EQ(rb_member_area_to_index("right"), AREA_RIGHT,
             "MemberArea::from_str(\"right\") maps to index 2");
    CHECK_EQ(rb_member_area_to_index("upstairs"), -1,
             "MemberArea::from_str rejects an unknown area");
    CHECK_EQ(rb_member_area_to_index(NULL), -1,
             "MemberArea::from_str rejects a missing area");

    CHECK_EQ(rb_member_area_from_index(AREA_LEFT), AREA_LEFT,
             "from_index(0) is LeftSide");
    CHECK_EQ(rb_member_area_from_index(AREA_CENTER), AREA_CENTER,
             "from_index(1) is Center");
    CHECK_EQ(rb_member_area_from_index(AREA_RIGHT), AREA_RIGHT,
             "from_index(2) is RightSide");
    CHECK_EQ(rb_member_area_from_index(3), -1,
             "from_index rejects an out-of-range slot");
    CHECK_EQ(rb_member_area_from_index(-1), -1,
             "from_index rejects a negative slot");

    CHECK(!strcmp(rb_member_area_to_str(AREA_LEFT), "left"),
          "as_str(LeftSide) is \"left\"");
    CHECK(!strcmp(rb_member_area_to_str(AREA_CENTER), "center"),
          "as_str(Center) is \"center\"");
    CHECK(!strcmp(rb_member_area_to_str(AREA_RIGHT), "right"),
          "as_str(RightSide) is \"right\"");
    CHECK(!strcmp(rb_member_area_to_str(7), "?"),
          "as_str of an unknown slot is the placeholder");

    /* Rule 4.5.7: left faces the opponent's right, right faces their left. */
    CHECK_EQ(rb_member_area_front(AREA_LEFT), AREA_RIGHT,
             "front_area(LeftSide) is RightSide");
    CHECK_EQ(rb_member_area_front(AREA_CENTER), AREA_CENTER,
             "front_area(Center) is Center");
    CHECK_EQ(rb_member_area_front(AREA_RIGHT), AREA_LEFT,
             "front_area(RightSide) is LeftSide");
    CHECK_EQ(rb_member_area_front(-1), -1,
             "front_area rejects an unknown area");

    CHECK_EQ(rb_member_area_to_tag(AREA_LEFT), 1, "to_tag(LeftSide) is 1");
    CHECK_EQ(rb_member_area_to_tag(AREA_CENTER), 2, "to_tag(Center) is 2");
    CHECK_EQ(rb_member_area_to_tag(AREA_RIGHT), 3, "to_tag(RightSide) is 3");
    CHECK_EQ(rb_member_area_to_tag(9), 0, "to_tag of an unknown slot is 0");
    CHECK_EQ(rb_member_area_from_tag(1), AREA_LEFT, "from_tag(1) is LeftSide");
    CHECK_EQ(rb_member_area_from_tag(2), AREA_CENTER, "from_tag(2) is Center");
    CHECK_EQ(rb_member_area_from_tag(3), AREA_RIGHT, "from_tag(3) is RightSide");
    CHECK_EQ(rb_member_area_from_tag(0), -1, "from_tag(0) is rejected");
    CHECK_EQ(rb_member_area_from_tag(4), -1, "from_tag(4) is rejected");
}

/* ---- 2. check_trigger_position (zones.rs:154-173) --------------------- */

static void test_check_trigger_position(void)
{
    CHECK(rb_check_trigger_position(NULL, AREA_LEFT),
          "Q143: a missing trigger string constrains nothing");
    CHECK(rb_check_trigger_position("", AREA_RIGHT),
          "an empty trigger string constrains nothing");
    CHECK(rb_check_trigger_position(TRIG_LEFT, AREA_LEFT),
          "left-side trigger is satisfied at LeftSide");
    CHECK(!rb_check_trigger_position(TRIG_LEFT, AREA_CENTER),
          "left-side trigger rejects Center");
    CHECK(rb_check_trigger_position(TRIG_RIGHT, AREA_RIGHT),
          "right-side trigger is satisfied at RightSide");
    CHECK(!rb_check_trigger_position(TRIG_RIGHT, AREA_LEFT),
          "right-side trigger rejects LeftSide");
    CHECK(rb_check_trigger_position(TRIG_CENTER, AREA_CENTER),
          "center trigger is satisfied at Center");
    CHECK(!rb_check_trigger_position(TRIG_CENTER, AREA_RIGHT),
          "center trigger rejects RightSide");
    /* Both side requirements present: zones.rs:163-171 checks each
     * independently, so no area can satisfy them. */
    CHECK(!rb_check_trigger_position(TRIG_BOTH, AREA_LEFT),
          "a trigger naming both sides is unsatisfiable at LeftSide");
    CHECK(!rb_check_trigger_position(TRIG_BOTH, AREA_RIGHT),
          "a trigger naming both sides is unsatisfiable at RightSide");
    CHECK(rb_check_trigger_position(TRIG_LIVE, AREA_CENTER),
          "a non-position trigger constrains nothing");
}

/* ---- 3. check_effect_position (zones.rs:175-206) ---------------------- */

static void test_check_effect_position_single(void)
{
    CHECK(rb_check_effect_position(NULL, AREA_CENTER),
          "a missing activation_position constrains nothing");
    CHECK(rb_check_effect_position("center", AREA_CENTER),
          "activation_position \"center\" matches Center");
    CHECK(!rb_check_effect_position("center", AREA_LEFT),
          "activation_position \"center\" rejects LeftSide");
    CHECK(rb_check_effect_position("left", AREA_LEFT),
          "activation_position \"left\" matches LeftSide");
    CHECK(!rb_check_effect_position("left", AREA_RIGHT),
          "activation_position \"left\" rejects RightSide");
    CHECK(rb_check_effect_position("right", AREA_RIGHT),
          "activation_position \"right\" matches RightSide");
    CHECK(!rb_check_effect_position("right", AREA_CENTER),
          "activation_position \"right\" rejects Center");
    CHECK(rb_check_effect_position("left_side", AREA_LEFT),
          "activation_position \"left_side\" matches LeftSide");
    CHECK(!rb_check_effect_position("left_side", AREA_CENTER),
          "activation_position \"left_side\" rejects Center");
    CHECK(rb_check_effect_position("right_side", AREA_RIGHT),
          "activation_position \"right_side\" matches RightSide");
    CHECK(!rb_check_effect_position("right_side", AREA_LEFT),
          "activation_position \"right_side\" rejects LeftSide");
    /* zones.rs:198-205: an unrecognised token is not a position restriction. */
    CHECK(rb_check_effect_position("anywhere", AREA_LEFT),
          "an unrecognised activation_position constrains nothing at LeftSide");
    CHECK(rb_check_effect_position("anywhere", AREA_CENTER),
          "an unrecognised activation_position constrains nothing at Center");
    CHECK(rb_check_effect_position("anywhere", AREA_RIGHT),
          "an unrecognised activation_position constrains nothing at RightSide");
}

static void test_check_effect_position_comma_list(void)
{
    CHECK(rb_check_effect_position("left_side,right_side", AREA_LEFT),
          "comma list \"left_side,right_side\" matches LeftSide");
    CHECK(!rb_check_effect_position("left_side,right_side", AREA_CENTER),
          "comma list \"left_side,right_side\" rejects Center");
    CHECK(rb_check_effect_position("left_side,right_side", AREA_RIGHT),
          "comma list \"left_side,right_side\" matches RightSide");
    /* zones.rs:185 trims each comma-separated token. */
    CHECK(rb_check_effect_position(" left , right ", AREA_RIGHT),
          "comma list trims surrounding whitespace before matching");
    CHECK(!rb_check_effect_position(" left , right ", AREA_CENTER),
          "whitespace-trimmed comma list still rejects Center");
    CHECK(rb_check_effect_position("left,center", AREA_CENTER),
          "comma list \"left,center\" matches Center");
    CHECK(!rb_check_effect_position("left,center", AREA_RIGHT),
          "comma list \"left,center\" rejects RightSide");
    /* A comma forces the list path: unrecognised tokens never match. */
    CHECK(!rb_check_effect_position("up,down", AREA_CENTER),
          "an all-unknown comma list matches nothing");
}

/* ---- 4. Stage area accessors (zones.rs:246-276) ----------------------- */

static void test_stage_area_accessors(void)
{
    int stage[RB_STAGE_SIZE] = { 11, RB_EMPTY_SLOT, 13 };
    CHECK_EQ(rb_stage_get_area(stage, AREA_LEFT), 11,
             "get_area returns the member in LeftSide");
    CHECK_EQ(rb_stage_get_area(stage, AREA_CENTER), RB_EMPTY_SLOT,
             "get_area yields None for an empty area");
    CHECK_EQ(rb_stage_get_area(stage, AREA_RIGHT), 13,
             "get_area returns the member in RightSide");
    CHECK_EQ(rb_stage_get_area(stage, 3), RB_EMPTY_SLOT,
             "get_area yields None for an out-of-range area");
    CHECK_EQ(rb_stage_get_area(stage, -1), RB_EMPTY_SLOT,
             "get_area yields None for a negative area");

    rb_stage_set_area(stage, AREA_CENTER, 12);
    CHECK_EQ(stage[AREA_CENTER], 12, "set_area fills the requested slot");
    rb_stage_set_area(stage, 3, 99);
    rb_stage_set_area(stage, -1, 99);
    CHECK_EQ(stage[AREA_LEFT], 11, "set_area ignores an out-of-range area");
    CHECK_EQ(stage[AREA_CENTER], 12, "set_area ignores a negative area");
}

/* ---- 5. Stage::position_change (zones.rs:367-414) --------------------- */

static void test_stage_position_change(void)
{
    int stage[RB_STAGE_SIZE] = { 11, 12, RB_EMPTY_SLOT };

    /* Empty destination: a plain move. */
    CHECK_EQ(rb_stage_position_change(stage, AREA_CENTER, AREA_RIGHT), 12,
             "position_change into an empty area moves the member");
    CHECK_EQ(stage[AREA_CENTER], RB_EMPTY_SLOT,
             "position_change empties the source area");
    CHECK_EQ(stage[AREA_RIGHT], 12, "position_change fills the destination");

    /* Occupied destination: a swap (Rule 11.10.2). */
    stage[AREA_CENTER] = 21;
    CHECK_EQ(rb_stage_position_change(stage, AREA_CENTER, AREA_LEFT), 21,
             "position_change into an occupied area returns the moving member");
    CHECK_EQ(stage[AREA_LEFT], 21, "the moving member lands on the destination");
    CHECK_EQ(stage[AREA_CENTER], 11, "the displaced member takes the source slot");

    /* Errors. */
    stage[AREA_CENTER] = RB_EMPTY_SLOT;
    CHECK_EQ(rb_stage_position_change(stage, AREA_CENTER, AREA_LEFT), -1,
             "position_change from an empty area is refused");
    CHECK_EQ(stage[AREA_LEFT], 21,
             "a refused position_change leaves the board alone");
    CHECK_EQ(rb_stage_position_change(stage, AREA_RIGHT, AREA_RIGHT), -1,
             "position_change to the same area is refused");
    CHECK_EQ(rb_stage_position_change(stage, -1, AREA_LEFT), -1,
             "position_change from a negative area is refused");
    CHECK_EQ(rb_stage_position_change(stage, AREA_LEFT, 3), -1,
             "position_change to an out-of-range area is refused");
    CHECK_EQ(stage[AREA_LEFT], 21, "out-of-range position_change is a no-op");
}

/* ---- 6. Stage::formation_change (zones.rs:416-434) + under-card travel */

static void test_stage_formation_change_rotation(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    RbPlayer *P = &tg.state.p[0];
    int a = test_new_id(&tg, "PL!-sd1-010-SD");
    int b = test_new_id(&tg, "PL!-sd1-013-SD");
    int c = test_new_id(&tg, "PL!-sd1-014-SD");
    if (a < 0 || b < 0 || c < 0) {
        CHECK(0, "formation rotation fixture cards resolve");
        return;
    }
    P->stage[AREA_LEFT] = a;
    P->stage[AREA_CENTER] = b;
    P->stage[AREA_RIGHT] = c;

    /* live_start_subunit_threshold_formation_test: right, center, left. */
    const int from[3] = { AREA_LEFT, AREA_CENTER, AREA_RIGHT };
    const int to[3] = { AREA_RIGHT, AREA_CENTER, AREA_LEFT };
    CHECK_EQ(rb_stage_formation_change(&tg.state, 0, from, to, 3), 0,
             "formation_change accepts a full three-member rotation");
    CHECK_EQ(P->stage[AREA_LEFT], a, "formation rotation keeps a on the left");
    CHECK_EQ(P->stage[AREA_CENTER], c, "formation rotation puts c in the center");
    CHECK_EQ(P->stage[AREA_RIGHT], b, "formation rotation puts b on the right");
}

static void test_stage_formation_change_rejections(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    RbPlayer *P = &tg.state.p[0];
    int a = test_new_id(&tg, "PL!-sd1-010-SD");
    int b = test_new_id(&tg, "PL!-sd1-013-SD");
    if (a < 0 || b < 0) {
        CHECK(0, "formation rejection fixture cards resolve");
        return;
    }
    P->stage[AREA_LEFT] = a;
    P->stage[AREA_CENTER] = b;
    P->stage[AREA_RIGHT] = RB_EMPTY_SLOT;

    /* Rule 11.11.2: two members may not target the same area. */
    const int dup_from[2] = { AREA_LEFT, AREA_CENTER };
    const int dup_to[2] = { AREA_RIGHT, AREA_RIGHT };
    CHECK_EQ(rb_stage_formation_change(&tg.state, 0, dup_from, dup_to, 2), -1,
             "formation_change refuses two members targeting one area");
    CHECK_EQ(P->stage[AREA_LEFT], a, "a rejected formation leaves the source area");
    CHECK_EQ(P->stage[AREA_CENTER], b,
             "a rejected formation leaves the second member");
    CHECK_EQ(P->stage[AREA_RIGHT], RB_EMPTY_SLOT,
             "a rejected formation fills nothing");

    const int same_from[1] = { AREA_LEFT };
    const int same_to[1] = { AREA_LEFT };
    CHECK_EQ(rb_stage_formation_change(&tg.state, 0, same_from, same_to, 1), -1,
             "formation_change refuses a member staying in its own area");

    const int empty_from[1] = { AREA_RIGHT };
    const int empty_to[1] = { AREA_LEFT };
    CHECK_EQ(rb_stage_formation_change(&tg.state, 0, empty_from, empty_to, 1), -1,
             "formation_change refuses an empty source area");

    const int oob_from[1] = { AREA_LEFT };
    const int oob_to[1] = { 5 };
    CHECK_EQ(rb_stage_formation_change(&tg.state, 0, oob_from, oob_to, 1), -1,
             "formation_change refuses an out-of-range destination");
}

static void test_formation_change_carries_under_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    RbPlayer *P = &tg.state.p[0];
    int left = test_new_id(&tg, "PL!-sd1-010-SD");
    int right = test_new_id(&tg, "PL!-sd1-013-SD");
    int under_left = test_new_id(&tg, "PL!-sd1-014-SD");
    int energy_left = test_id(&tg, "LL-E-001-SD");
    int under_right = test_new_id(&tg, "PL!-sd1-002-SD");
    if (left < 0 || right < 0 || under_left < 0 || under_right < 0) {
        CHECK(0, "under-card travel fixture cards resolve");
        return;
    }
    P->stage[AREA_LEFT] = left;
    P->stage[AREA_RIGHT] = right;
    test_place_under(&tg, 0, AREA_LEFT, under_left);
    test_place_under(&tg, 0, AREA_LEFT, energy_left);
    test_place_under(&tg, 0, AREA_RIGHT, under_right);

    const int from[2] = { AREA_LEFT, AREA_RIGHT };
    const int to[2] = { AREA_RIGHT, AREA_LEFT };
    CHECK_EQ(rb_stage_formation_change(&tg.state, 0, from, to, 2), 0,
             "formation_change swaps the two occupied side areas");
    /* Rule 4.5.5.3: under-cards travel with their member. */
    CHECK_EQ(P->under_cards[AREA_LEFT].n, 1,
             "the right-side member's under-card moved to LeftSide");
    CHECK_EQ(P->under_cards[AREA_LEFT].cards[0], under_right,
             "LeftSide keeps the under-card that came with its new member");
    CHECK_EQ(P->under_cards[AREA_RIGHT].n, 2,
             "the left-side member's two under-cards moved to RightSide");
    CHECK_EQ(P->under_cards[AREA_RIGHT].cards[0], under_left,
             "RightSide keeps the first under-card in order");
    CHECK_EQ(P->under_cards[AREA_RIGHT].cards[1], energy_left,
             "RightSide keeps the energy under-card in order");
}

/* ---- 7. recycle_under_cards + can_place_card (zones.rs:301-365,478-486) */

static void test_recycle_under_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int member_a = test_new_id(&tg, "PL!-sd1-010-SD");
    int member_b = test_new_id(&tg, "PL!-sd1-013-SD");
    int energy = test_id(&tg, "LL-E-001-SD");
    if (member_a < 0 || member_b < 0) {
        CHECK(0, "recycle fixture cards resolve");
        return;
    }
    test_place_under(&tg, 0, AREA_CENTER, member_a);
    test_place_under(&tg, 0, AREA_CENTER, energy);
    test_place_under(&tg, 0, AREA_CENTER, member_b);

    int wait[RB_MAX_ZONE], energy_out[RB_MAX_ZONE];
    int n_wait = -1, n_energy = -1;
    int moved = rb_stage_recycle_under_cards(&tg.state, 0, AREA_CENTER, wait,
                                             &n_wait, energy_out, &n_energy,
                                             RB_MAX_ZONE);
    CHECK_EQ(moved, 3, "recycle_under_cards reports every under-card moved");
    CHECK_EQ(n_wait, 2, "member under-cards route to the waitroom");
    CHECK_EQ(n_energy, 1, "energy under-cards route to the energy deck");
    CHECK(wait[0] == member_a && wait[1] == member_b,
          "waitroom under-cards keep their relative order");
    CHECK_EQ(energy_out[0], energy,
             "the energy under-card is the energy-deck card");
    CHECK_EQ(tg.state.p[0].under_cards[AREA_CENTER].n, 0,
             "recycle_under_cards empties the source stack");

    int wait2[RB_MAX_ZONE], energy2[RB_MAX_ZONE];
    int nw2 = 0, ne2 = 0;
    CHECK_EQ(rb_stage_recycle_under_cards(&tg.state, 0, AREA_CENTER, wait2,
                                          &nw2, energy2, &ne2, RB_MAX_ZONE), 0,
             "recycling an empty stack moves nothing");
}

static void test_stage_can_place_card(void)
{
    TestGame tg;
    test_game_new(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    int live = test_new_id(&tg, "PL!N-bp1-027-L");
    if (member < 0 || live < 0) {
        CHECK(0, "can_place_card fixture cards resolve");
        return;
    }
    CHECK(rb_stage_can_place_card(&tg.state, 0, member),
          "a member card may be placed on the stage");
    CHECK(!rb_stage_can_place_card(&tg.state, 0, live),
          "a live card may not be placed on the stage (Rule 8.2.2)");
    CHECK(!rb_stage_can_place_card(&tg.state, 0, -1),
          "an unknown card id places nowhere");
    CHECK(!rb_stage_can_place_card(&tg.state, 0, 1 << 20),
          "an out-of-range card id places nowhere");
}

/* ---- 8. End-to-end: PL!SP-bp5-006-R position_change ------------------ */

static void test_mill_three_deck_four(void)
{
    /* mill_three_self_position_change_q104_q234_test::q234_deck_4 */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int kinako = test_new_id(&tg, "PL!SP-bp5-006-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    int center_member = test_new_id(&tg, "PL!-sd1-013-SD");
    if (kinako < 0 || filler < 0 || center_member < 0) {
        CHECK(0, "mill_three deck-4 fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_LEFT] = kinako;
    tg.state.p[0].stage[AREA_CENTER] = center_member;
    for (int i = 0; i < 4; i++) test_add_to_deck(&tg, filler);

    test_activate_ability(&tg, kinako);
    CHECK_EQ(tg.state.p[0].deck.n, 1,
             "Q234: the top three deck cards pay the activation cost");
    CHECK(rb_has_pending_choice(&tg.state),
          "position_change offers a destination choice after the cost");
    {
        const RbChoice *dbg = rb_get_pending_choice(&tg.state);
        fprintf(stderr, "[DBG mill3] kind=%d route=%d zone=%s target=%s cnt=%d nfid=%d fids=[",
                dbg ? (int)dbg->kind : -1, dbg ? (int)dbg->route : -1,
                dbg ? dbg->zone : "", dbg ? dbg->target : "",
                dbg ? dbg->count : -1, dbg ? dbg->n_filtered_indices : -1);
        if (dbg) for (int i = 0; i < dbg->n_filtered_indices; i++) fprintf(stderr, "%d,", dbg->filtered_indices[i]);
        fprintf(stderr, "] desc=%s host=%d actor=%d resume_mode=%d resume_eff=%p\n", dbg ? dbg->description : "",
                tg.state.queue.resume_host, tg.state.queue.actor, tg.state.queue.resume_mode,
                (void *)tg.state.queue.resume_eff);
    }
    /* Source is left, so the destination options are [center, right]. */
    rb_resume_with_choice(&tg.state, 1);
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], RB_EMPTY_SLOT,
             "LeftSide empties after the position change");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], kinako,
             "Q234: the member reaches RightSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], center_member,
             "the center member is not disturbed by a move into an empty area");
}

static void test_mill_three_deck_two_no_waitroom(void)
{
    /* mill_three_self_position_change_q104_q234_test::q234_deck_2_without_waitroom */
    const char *rarities[3] = { "PL!SP-bp5-006-AR", "PL!SP-bp5-006-P",
                                "PL!SP-bp5-006-R" };
    for (int r = 0; r < 3; r++) {
        TestGame tg;
        test_game_new(&tg);
        clear_p1(&tg);
        int kinako = test_new_id(&tg, rarities[r]);
        int filler = test_new_id(&tg, "PL!-sd1-010-SD");
        if (kinako < 0 || filler < 0) {
            CHECK(0, "mill_three deck-2 fixtures resolve");
            return;
        }
        tg.state.p[0].stage[AREA_LEFT] = kinako;
        test_add_to_deck(&tg, filler);
        test_add_to_deck(&tg, filler);
        test_activate_ability(&tg, kinako);
        CHECK(!rb_has_pending_choice(&tg.state),
              "a two-card deck with an empty waitroom cannot pay the cost");
        CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], kinako,
                 "the unpayable activation leaves the member in place");
        CHECK_EQ(tg.state.p[0].deck.n, 2, "the unpayable activation draws nothing");
    }
}

static void test_mill_three_deck_and_waitroom_empty(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int kinako = test_new_id(&tg, "PL!SP-bp5-006-R");
    if (kinako < 0) {
        CHECK(0, "mill_three empty-deck fixture resolves");
        return;
    }
    tg.state.p[0].stage[AREA_CENTER] = kinako;
    test_activate_ability(&tg, kinako);
    CHECK(!rb_has_pending_choice(&tg.state),
          "no position choice when both deck and waitroom are empty");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], kinako,
             "the member remains at the original position when the cost fails");
}

/* ---- 9. End-to-end: formation via a live-start position_change -------- */

static void test_live_start_two_subunit_members_reform(void)
{
    /* live_start_subunit_threshold_formation_test::two_subunit_members_can_reform */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int live = test_new_id(&tg, "PL!SP-pb2-050-L");
    int sub_a = test_new_id(&tg, "PL!SP-bp1-014-N");
    int sub_b = test_new_id(&tg, "PL!SP-bp1-014-N");
    int other = test_new_id(&tg, "PL!-sd1-010-SD");
    if (live < 0 || sub_a < 0 || sub_b < 0 || other < 0) {
        CHECK(0, "live-start formation fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_LEFT] = sub_a;
    tg.state.p[0].stage[AREA_CENTER] = sub_b;
    tg.state.p[0].stage[AREA_RIGHT] = other;
    fill_decks(&tg, other, 20);
    test_add_to_live(&tg, live);

    rb_trigger_live_start(&tg.state, 0);
    CHECK(rb_has_pending_choice(&tg.state),
          "two subunit members unlock the live-start formation choice");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET,
          "live-start formation asks for a SelectTarget destination");
    drain_all(&tg, 8);
    CHECK(!rb_has_pending_choice(&tg.state),
          "the live-start formation resolves after every destination is chosen");
    CHECK(tg.state.p[1].stage[AREA_LEFT] == RB_EMPTY_SLOT &&
              tg.state.p[1].stage[AREA_CENTER] == RB_EMPTY_SLOT &&
              tg.state.p[1].stage[AREA_RIGHT] == RB_EMPTY_SLOT,
          "the opponent stage is untouched by our formation change");
}

static void test_live_start_one_subunit_member_no_formation(void)
{
    /* live_start_subunit_threshold_formation_test::one_subunit_member */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int live = test_new_id(&tg, "PL!SP-pb2-050-L");
    int sub_a = test_new_id(&tg, "PL!SP-bp1-014-N");
    int other = test_new_id(&tg, "PL!-sd1-010-SD");
    if (live < 0 || sub_a < 0 || other < 0) {
        CHECK(0, "live-start threshold fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_LEFT] = sub_a;
    tg.state.p[0].stage[AREA_CENTER] = other;
    tg.state.p[0].stage[AREA_RIGHT] = other;
    fill_decks(&tg, other, 20);
    test_add_to_live(&tg, live);

    int before_left = tg.state.p[0].stage[AREA_LEFT];
    int before_center = tg.state.p[0].stage[AREA_CENTER];
    int before_right = tg.state.p[0].stage[AREA_RIGHT];
    rb_trigger_live_start(&tg.state, 0);
    CHECK(!rb_has_pending_choice(&tg.state),
          "a single subunit member does not unlock formation");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], before_left,
             "the threshold case leaves LeftSide alone");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], before_center,
             "the threshold case leaves Center alone");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], before_right,
             "the threshold case leaves RightSide alone");
}

/* ---- 10. End-to-end: PL!HS-pb1-014-R front-area debut ----------------- */

static void test_himeno_front_area_shapes(void)
{
    /* Himeno on center: front area is center, so the chosen opponent member
     * lands in opponent Center. */
    TestGame tg;
    int himeno, opp_left, opp_center, opp_right;
    test_game_new(&tg);
    clear_p1(&tg);
    himeno = test_new_id(&tg, "PL!HS-pb1-014-R");
    opp_left = test_new_id(&tg, "PL!-sd1-010-SD");
    opp_center = test_new_id(&tg, "PL!-sd1-013-SD");
    opp_right = test_new_id(&tg, "PL!-sd1-014-SD");
    if (himeno < 0 || opp_left < 0 || opp_center < 0 || opp_right < 0) {
        CHECK(0, "himeno fixtures resolve");
        return;
    }
    set_p2_stage(&tg, opp_left, opp_center, opp_right);
    test_add_to_hand(&tg, himeno);
    test_give_energy(&tg, 9);
    test_play_to_stage(&tg, himeno, AREA_CENTER);
    /* Options are the opponent's areas in [left, center, right] order. */
    answer(&tg, 0, 6);
    CHECK_EQ(tg.state.p[1].stage[AREA_CENTER], opp_left,
             "front of center is center: the selected LeftSide member lands in Center");

    /* Himeno on left: front area is the mirrored RightSide. */
    test_game_new(&tg);
    clear_p1(&tg);
    himeno = test_new_id(&tg, "PL!HS-pb1-014-R");
    opp_center = test_new_id(&tg, "PL!-sd1-013-SD");
    opp_right = test_new_id(&tg, "PL!-sd1-014-SD");
    if (himeno < 0 || opp_center < 0 || opp_right < 0) {
        CHECK(0, "himeno left-side fixtures resolve");
        return;
    }
    set_p2_stage(&tg, RB_EMPTY_SLOT, opp_center, opp_right);
    test_add_to_hand(&tg, himeno);
    test_give_energy(&tg, 9);
    test_play_to_stage(&tg, himeno, AREA_LEFT);
    answer(&tg, 0, 6);
    CHECK_EQ(tg.state.p[1].stage[AREA_RIGHT], opp_center,
             "front of left is the mirrored RightSide (Rule 4.5.7)");

    /* Himeno on right: front area is the mirrored LeftSide. */
    test_game_new(&tg);
    clear_p1(&tg);
    himeno = test_new_id(&tg, "PL!HS-pb1-014-R");
    opp_left = test_new_id(&tg, "PL!-sd1-010-SD");
    opp_center = test_new_id(&tg, "PL!-sd1-013-SD");
    if (himeno < 0 || opp_left < 0 || opp_center < 0) {
        CHECK(0, "himeno right-side fixtures resolve");
        return;
    }
    set_p2_stage(&tg, opp_left, opp_center, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, himeno);
    test_give_energy(&tg, 9);
    test_play_to_stage(&tg, himeno, AREA_RIGHT);
    answer(&tg, 0, 6);
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], opp_center,
             "front of right is the mirrored LeftSide (Rule 4.5.7)");
}

static void test_himeno_front_no_opponent_members(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int himeno = test_new_id(&tg, "PL!HS-pb1-014-R");
    if (himeno < 0) {
        CHECK(0, "himeno empty-opponent fixture resolves");
        return;
    }
    set_p2_stage(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, himeno);
    test_give_energy(&tg, 9);
    test_play_to_stage(&tg, himeno, AREA_CENTER);
    CHECK(!rb_has_pending_choice(&tg.state),
          "an empty opponent stage offers no front-area target");
}

/* ---- 11. End-to-end: PL!SP-bp5-010-R center rotation ------------------ */

static void test_wien_both_centers_empty(void)
{
    /* wien_bp5_test::both_centers_empty_no_moves */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int wien = test_new_id(&tg, "PL!SP-bp5-010-R");
    int p1_right = test_new_id(&tg, "PL!-sd1-010-SD");
    int p2_right = test_new_id(&tg, "PL!-sd1-013-SD");
    if (wien < 0 || p1_right < 0 || p2_right < 0) {
        CHECK(0, "wien empty-center fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_RIGHT] = p1_right;
    set_p2_stage(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, p2_right);
    test_add_to_hand(&tg, wien);
    test_give_energy(&tg, 13);
    test_play_to_stage(&tg, wien, AREA_LEFT);
    CHECK(!rb_has_pending_choice(&tg.state),
          "an empty center on both sides means no position change at all");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], wien, "wien stays at LeftSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT, "P1 Center stays empty");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], p1_right, "P1 RightSide keeps its member");
    CHECK_EQ(tg.state.p[1].stage[AREA_RIGHT], p2_right, "P2 RightSide keeps its member");
}

static void test_wien_opponent_center_only(void)
{
    /* wien_bp5_test::q223_p1_center_empty_opponent_chooses */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int wien = test_new_id(&tg, "PL!SP-bp5-010-R");
    int p1_right = test_new_id(&tg, "PL!-sd1-010-SD");
    int p2_center = test_new_id(&tg, "PL!-sd1-013-SD");
    if (wien < 0 || p1_right < 0 || p2_center < 0) {
        CHECK(0, "wien opponent-center fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_RIGHT] = p1_right;
    set_p2_stage(&tg, RB_EMPTY_SLOT, p2_center, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, wien);
    test_give_energy(&tg, 13);
    test_play_to_stage(&tg, wien, AREA_LEFT);
    CHECK(rb_has_pending_choice(&tg.state),
          "Q223: the opponent owns the destination choice for their center member");
    /* Source is center, so the options are [left, right]. */
    rb_resume_with_choice(&tg.state, 0);
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], p2_center,
             "the opponent's center member moves to the chosen LeftSide");
    CHECK_EQ(tg.state.p[1].stage[AREA_CENTER], RB_EMPTY_SLOT, "P2 Center empties");
    CHECK(!rb_has_pending_choice(&tg.state),
          "an empty P1 Center means there is no self-side choice");
}

static void test_wien_opponent_swap_on_occupied_destination(void)
{
    /* wien_bp5_test::q223_opponent_swap_when_destination_occupied */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int wien = test_new_id(&tg, "PL!SP-bp5-010-R");
    int p1_right = test_new_id(&tg, "PL!-sd1-010-SD");
    int p2_center = test_new_id(&tg, "PL!-sd1-013-SD");
    int p2_left = test_new_id(&tg, "PL!-sd1-014-SD");
    if (wien < 0 || p1_right < 0 || p2_center < 0 || p2_left < 0) {
        CHECK(0, "wien swap fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_RIGHT] = p1_right;
    set_p2_stage(&tg, p2_left, p2_center, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, wien);
    test_give_energy(&tg, 13);
    test_play_to_stage(&tg, wien, AREA_LEFT);
    CHECK(rb_has_pending_choice(&tg.state), "the opponent gets the destination choice");
    rb_resume_with_choice(&tg.state, 0); /* choose Left (occupied) */
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], p2_center,
             "choosing an occupied area swaps the two opponent members");
    CHECK_EQ(tg.state.p[1].stage[AREA_CENTER], p2_left,
             "the displaced opponent member takes the vacated Center");
}

/* ---- 12. End-to-end: PL!SP-bp7-022-N cost + position change ----------- */

static void test_tomari_activation_returns_energy(void)
{
    /* energy_return_cost_position_change_test::with_energy */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int tomari = test_new_id(&tg, "PL!SP-bp7-022-N");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    if (tomari < 0 || filler < 0) {
        CHECK(0, "tomari fixtures resolve");
        return;
    }
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = tomari;
    test_give_energy(&tg, 5);

    int deck_before = tg.state.p[0].energy_deck.n;
    int zone_before = tg.state.p[0].energy.n;
    test_activate_ability(&tg, tomari);
    drain_all(&tg, 10);

    CHECK_EQ(tg.state.p[0].energy_deck.n, deck_before + 1,
             "one energy is returned to the energy deck");
    CHECK_EQ(tg.state.p[0].energy.n, zone_before - 1,
             "the energy zone loses the energy that paid the cost");
    CHECK(tg.state.p[0].stage[AREA_LEFT] != tomari &&
              tg.state.p[0].stage[AREA_CENTER] != tomari,
          "the member no longer sits in the center after the position change");
}

static void test_tomari_activation_without_energy_refused(void)
{
    /* energy_return_cost_position_change_test::without_energy */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int tomari = test_new_id(&tg, "PL!SP-bp7-022-N");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    if (tomari < 0 || filler < 0) {
        CHECK(0, "tomari zero-energy fixtures resolve");
        return;
    }
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = tomari;
    /* No energy at all: the mandatory {E} cost cannot be paid. */
    test_activate_ability(&tg, tomari);
    drain_all(&tg, 4);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], tomari,
             "an unpayable mandatory cost leaves the member on stage");
    CHECK(!rb_has_pending_choice(&tg.state),
          "an unpayable mandatory cost creates no position choice");
}

/* ---- 13. End-to-end: PL!SP-pb1-003-R two-stage rotation --------------- */

static void test_chisato_rotates_both_stages(void)
{
    /* chisato_pb1_003_both_stages_rotation_test */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int chisato = test_new_id(&tg, "PL!SP-pb1-003-R");
    int syncrie_a = test_new_id(&tg, "PL!SP-pb1-014-N");
    int syncrie_b = test_new_id(&tg, "PL!SP-pb1-019-N");
    int opp_left = test_new_id(&tg, "PL!-sd1-010-SD");
    int opp_center = test_new_id(&tg, "PL!-sd1-013-SD");
    int opp_right = test_new_id(&tg, "PL!-sd1-014-SD");
    if (chisato < 0 || syncrie_a < 0 || syncrie_b < 0 ||
        opp_left < 0 || opp_center < 0 || opp_right < 0) {
        CHECK(0, "chisato rotation fixtures resolve");
        return;
    }
    tg.state.p[0].stage[AREA_LEFT] = syncrie_a;
    tg.state.p[0].stage[AREA_CENTER] = syncrie_b;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    set_p2_stage(&tg, opp_left, opp_center, opp_right);
    test_add_to_hand(&tg, chisato);
    test_give_energy(&tg, 9);
    test_play_to_stage(&tg, chisato, AREA_RIGHT);

    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], syncrie_b,
             "chisato rotation puts the former Center member on the left");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], chisato, "chisato lands in the center");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], syncrie_a,
             "chisato rotation sends the former LeftSide member to the right");
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], opp_center,
             "the opponent center member rotates to LeftSide");
    CHECK_EQ(tg.state.p[1].stage[AREA_CENTER], opp_right,
             "the opponent right-side member rotates to Center");
    CHECK_EQ(tg.state.p[1].stage[AREA_RIGHT], opp_left,
             "the opponent left-side member rotates to RightSide");
    CHECK(!rb_has_pending_choice(&tg.state),
          "chisato's automatic rotation asks nothing");
}

/* ---- 14. End-to-end: PL!S-bp5-111-R self position change ------------- */

static void test_seira_position_change_swaps_and_records(void)
{
    /* self_control_position_change_test::position_change_triggered_grant_blade */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int seira_a = test_new_id(&tg, "PL!S-bp5-111-R");
    int seira_b = test_new_id(&tg, "PL!S-bp5-111-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    if (seira_a < 0 || seira_b < 0 || filler < 0) {
        CHECK(0, "seira fixtures resolve");
        return;
    }
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT] = filler;
    tg.state.p[0].stage[AREA_CENTER] = seira_a;
    tg.state.p[0].stage[AREA_RIGHT] = seira_b;

    test_activate_ability(&tg, seira_a);
    CHECK(rb_has_pending_choice(&tg.state),
          "activating the position_change ability offers a destination choice");
    /* Source is center, so the options are [left, right]; pick right. */
    rb_resume_with_choice(&tg.state, 1);
    drain_all(&tg, 6);

    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], seira_b,
             "the displaced member takes the vacated center");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], seira_a,
             "the moving member lands on the right after the position change");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], filler,
             "the left-side filler is not disturbed");
    CHECK(moved_this_turn(&tg, seira_a),
          "the moving member is recorded as having moved this turn");
    CHECK(moved_this_turn(&tg, seira_b),
          "the swapped member is recorded as having moved this turn");
    CHECK(!moved_this_turn(&tg, filler),
          "an untouched member is not recorded as having moved");
    CHECK(tg.state.position_change_occurred_this_turn,
          "position_change_occurred_this_turn is set by the swap");
}

/* ---- 15. End-to-end: PL!SP-bp2-003-R jidou after a swap -------------- */

static void test_jidou_fires_after_position_change(void)
{
    /* position_change_triggers_jidou_move_test */
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int kinako = test_new_id(&tg, "PL!SP-bp5-006-R");
    int chisato = test_new_id(&tg, "PL!SP-bp2-003-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    int energy = test_id(&tg, "LL-E-001-SD");
    if (kinako < 0 || chisato < 0 || filler < 0) {
        CHECK(0, "jidou fixtures resolve");
        return;
    }
    test_add_to_hand(&tg, kinako);
    test_add_to_hand(&tg, chisato);
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < 10; i++) test_add_to_energy_deck(&tg, 0, energy);
    test_give_energy(&tg, 20);

    CHECK_EQ(test_try_play_to_stage(&tg, chisato, AREA_LEFT), 1,
             "the jidou member is deployed to LeftSide");
    CHECK_EQ(test_try_play_to_stage(&tg, kinako, AREA_RIGHT), 1,
             "the position_change member is deployed to RightSide");
    int energy_before = tg.state.p[0].energy.n;

    test_activate_ability(&tg, kinako);
    CHECK(rb_has_pending_choice(&tg.state),
          "the top-three cost resolves into a position destination choice");
    /* Source is right, so the options are [left, center]; pick left. */
    rb_resume_with_choice(&tg.state, 0);
    drain_all(&tg, 8);

    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], kinako, "the moving member lands on the left");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], chisato,
             "the swapped member lands on the right");
    CHECK(moved_this_turn(&tg, kinako), "the moving member is recorded as moved");
    CHECK(moved_this_turn(&tg, chisato), "the swapped member is recorded as moved");
    CHECK_EQ(tg.state.p[0].energy.n, energy_before + 1,
             "the moved member's jidou ability places exactly one energy card");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_member_area_labels();
    test_check_trigger_position();
    test_check_effect_position_single();
    test_check_effect_position_comma_list();
    test_stage_area_accessors();
    test_stage_position_change();
    test_stage_formation_change_rotation();
    test_stage_formation_change_rejections();
    test_formation_change_carries_under_cards();
    test_recycle_under_cards();
    test_stage_can_place_card();
    test_mill_three_deck_four();
    test_mill_three_deck_two_no_waitroom();
    test_mill_three_deck_and_waitroom_empty();
    test_live_start_two_subunit_members_reform();
    test_live_start_one_subunit_member_no_formation();
    test_himeno_front_area_shapes();
    test_himeno_front_no_opponent_members();
    test_wien_both_centers_empty();
    test_wien_opponent_center_only();
    test_wien_opponent_swap_on_occupied_destination();
    test_tomari_activation_returns_energy();
    test_tomari_activation_without_energy_refused();
    test_chisato_rotates_both_stages();
    test_seira_position_change_swaps_and_records();
    test_jidou_fires_after_position_change();
    rb_unload();
    printf("\nparity_zones_position: %d assertions, %d failures\n", assertions, failures);
    if (failures) return 1;
    printf("ALL PARITY ZONES POSITION CHECKS PASSED\n");
    return 0;
}
