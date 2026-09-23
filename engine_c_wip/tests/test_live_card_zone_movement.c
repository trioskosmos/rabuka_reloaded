#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s\n", message); failures++; } \
    else printf("ok: %s\n", message); \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_ = (actual); \
    int expected_ = (expected); \
    if (actual_ != expected_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_, expected_); \
        failures++; \
    } else printf("ok: %s\n", message); \
} while (0)

typedef struct {
    int moved_card_id;
    const char *source_zone;
    const char *dest_zone;
    int cause_player_id;
} MovementEvent;

static MovementEvent movement_log[16];
static int movement_log_n;

static void clear_movement_log(void) {
    movement_log_n = 0;
}

static void push_movement_event(GameState *state, int card_id, const char *source,
                                const char *destination, int cause_player) {
    if (movement_log_n < (int)(sizeof(movement_log) / sizeof(movement_log[0]))) {
        movement_log[movement_log_n].moved_card_id = card_id;
        movement_log[movement_log_n].source_zone = source;
        movement_log[movement_log_n].dest_zone = destination;
        movement_log[movement_log_n].cause_player_id = cause_player;
        movement_log_n++;
    }
    rb_record_card_movement(state, card_id, rb_zone_id_from_str(source),
                            rb_zone_id_from_str(destination), cause_player, 0);
}

static void record_timing_movements(GameState *state) {
    int old_live[2][RB_MAX_LIVE_CARDS];
    int old_live_n[2];
    old_live_n[0] = state->p[0].live.n;
    old_live_n[1] = state->p[1].live.n;
    memcpy(old_live[0], state->p[0].live.cards, (size_t)old_live_n[0] * sizeof(old_live[0][0]));
    memcpy(old_live[1], state->p[1].live.cards, (size_t)old_live_n[1] * sizeof(old_live[1][0]));
    rb_check_timing(state);
    for (int player = 0; player < 2; player++) {
        for (int i = 0; i < old_live_n[player]; i++) {
            int card = old_live[player][i];
            int still_live = 0;
            for (int j = 0; j < state->p[player].live.n; j++) {
                if (state->p[player].live.cards[j] == card) {
                    still_live = 1;
                    break;
                }
            }
            if (!still_live) {
                const char *destination = rb_card_is_energy(card) ? "energy_deck" : "waitroom";
                push_movement_event(state, card, "live_card_zone", destination, player);
            }
        }
    }
}

static void fill_decks(TestGame *game, int filler) {
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(game, 0, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static Condition source_dest_condition_with_count(const char *source, const char *destination,
                                                   int count) {
    Condition condition;
    memset(&condition, 0, sizeof(condition));
    condition.variant = RB_COND_LOCATION;
    condition.fields[condition.n_fields].key = "count";
    condition.fields[condition.n_fields].v.tag = RB_TAG_I64;
    condition.fields[condition.n_fields].v.i = count;
    condition.n_fields++;
    condition.fields[condition.n_fields].key = "operator";
    condition.fields[condition.n_fields].v.tag = RB_TAG_STR;
    condition.fields[condition.n_fields].v.s = ">=";
    condition.n_fields++;
    condition.fields[condition.n_fields].key = "card_type";
    condition.fields[condition.n_fields].v.tag = RB_TAG_STR;
    condition.fields[condition.n_fields].v.s = "member_card";
    condition.n_fields++;
    condition.fields[condition.n_fields].key = "source";
    condition.fields[condition.n_fields].v.tag = RB_TAG_STR;
    condition.fields[condition.n_fields].v.s = source;
    condition.n_fields++;
    condition.fields[condition.n_fields].key = "destination";
    condition.fields[condition.n_fields].v.tag = RB_TAG_STR;
    condition.fields[condition.n_fields].v.s = destination;
    condition.n_fields++;
    return condition;
}

static Condition source_dest_condition(const char *source, const char *destination) {
    return source_dest_condition_with_count(source, destination, 1);
}

static Condition source_dest_condition_targeted(const char *source, const char *destination) {
    Condition condition = source_dest_condition(source, destination);
    condition.fields[condition.n_fields].key = "target";
    condition.fields[condition.n_fields].v.tag = RB_TAG_STR;
    condition.fields[condition.n_fields].v.s = "opponent";
    condition.n_fields++;
    return condition;
}

static void add_to_discard_player(TestGame *game, int player, int card_id) {
    RbBag *discard = &game->state.p[player].discard;
    if (discard->n < RB_MAX_ZONE) discard->cards[discard->n++] = card_id;
}

static int zone_has_id(const RbBag *bag, int card_id) {
    for (int i = 0; i < bag->n; i++) {
        if (bag->cards[i] == card_id) return 1;
    }
    return 0;
}

static int movement_count_for(int card_id) {
    int count = 0;
    for (int i = 0; i < movement_log_n; i++) {
        if (movement_log[i].moved_card_id == card_id) count++;
    }
    return count;
}

static MovementEvent *movement_for(int card_id) {
    for (int i = 0; i < movement_log_n; i++) {
        if (movement_log[i].moved_card_id == card_id) return &movement_log[i];
    }
    return NULL;
}

static void test_invalid_live_card_discard_records_turn_movements(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    game.state.p[0].live.cards[game.state.p[0].live.n++] = member;
    game.state.p[0].stage[0] = member;
    fill_decks(&game, member);

    CHECK(zone_has_id(&game.state.p[0].live, member), "invalid live card precondition");
    CHECK_EQ(movement_log_n, 0, "no movement recorded before timing check");
    record_timing_movements(&game.state);

    CHECK(!zone_has_id(&game.state.p[0].live, member), "invalid member leaves live zone");
    CHECK(zone_has_id(&game.state.p[0].discard, member), "invalid member moves to waitroom");
    CHECK(movement_log_n > 0, "invalid live movement is recorded");
    if (movement_log_n > 0) {
        MovementEvent *movement = movement_for(member);
        CHECK(movement != NULL, "invalid live movement identifies the card");
        if (movement) {
            CHECK_EQ(movement->moved_card_id, member, "invalid live movement card id");
            CHECK(strcmp(movement->source_zone, "live_card_zone") == 0, "invalid live movement source");
            CHECK(strcmp(movement->dest_zone, "waitroom") == 0, "invalid live movement destination");
            CHECK_EQ(movement->cause_player_id, 0, "invalid live movement cause player");
        }
    }
}

static void test_source_dest_condition_matches_turn_movements(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "live_card_zone", "waitroom", 0);
    test_add_to_discard(&game, member);
    Condition condition = source_dest_condition("live_card_zone", "discard");
    CHECK(rb_eval_condition(&game.state, 0, &condition), "source and destination condition matches movement");
}

static void test_source_dest_condition_wrong_source_fails(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "stage", "waitroom", 0);
    test_add_to_discard(&game, member);
    Condition condition = source_dest_condition("live_card_zone", "discard");
    CHECK(!rb_eval_condition(&game.state, 0, &condition), "wrong movement source fails condition");
}

static void test_source_dest_condition_card_not_in_dest_fails(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "live_card_zone", "waitroom", 0);
    Condition condition = source_dest_condition("live_card_zone", "discard");
    CHECK(!rb_eval_condition(&game.state, 0, &condition), "card absent from destination fails condition");
}

static void test_source_dest_condition_needs_multiple_fails(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "live_card_zone", "waitroom", 0);
    test_add_to_discard(&game, member);
    Condition condition = source_dest_condition_with_count("live_card_zone", "discard", 2);
    CHECK(!rb_eval_condition(&game.state, 0, &condition), "one movement does not satisfy count two");
}

static void test_turn_movements_cleared_at_turn_start(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "live_card_zone", "waitroom", 0);
    CHECK_EQ(movement_log_n, 1, "movement exists before turn tracking is cleared");
    rb_clear_movement_tracking(&game.state);
    clear_movement_log();
    CHECK_EQ(movement_log_n, 0, "turn movement tracking is cleared at turn start");
}

static void test_invalid_energy_card_in_live_zone_goes_to_energy_deck(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int energy = test_id(&game, "LL-E-001-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    game.state.p[0].live.cards[game.state.p[0].live.n++] = energy;
    game.state.p[0].stage[0] = filler;
    fill_decks(&game, filler);
    record_timing_movements(&game.state);
    CHECK(!zone_has_id(&game.state.p[0].live, energy), "invalid energy leaves live zone");
    CHECK(zone_has_id(&game.state.p[0].energy_deck, energy), "invalid energy moves to energy deck");
}

static void test_source_dest_condition_excludes_other_player(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "live_card_zone", "waitroom", 1);
    add_to_discard_player(&game, 1, member);
    Condition condition = source_dest_condition("live_card_zone", "discard");
    CHECK(!rb_eval_condition(&game.state, 0, &condition), "self condition excludes opponent movement");
}

static void test_source_dest_condition_opponent_target_includes_other_player(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member = test_id(&game, "PL!-sd1-010-SD");
    fill_decks(&game, member);
    push_movement_event(&game.state, member, "live_card_zone", "waitroom", 1);
    add_to_discard_player(&game, 1, member);
    Condition condition = source_dest_condition_targeted("live_card_zone", "discard");
    CHECK(rb_eval_condition(&game.state, 0, &condition), "opponent target includes opponent movement");
}

static void test_check_invalid_live_cards_distinguishes_players(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();
    int member1 = test_id(&game, "PL!-sd1-010-SD");
    int member2 = test_new_id(&game, "PL!-sd1-010-SD");
    int filler = member1;
    game.state.p[0].live.cards[game.state.p[0].live.n++] = member1;
    game.state.p[1].live.cards[game.state.p[1].live.n++] = member2;
    game.state.p[0].stage[0] = filler;
    game.state.p[1].stage[0] = filler;
    fill_decks(&game, filler);
    record_timing_movements(&game.state);

    CHECK(zone_has_id(&game.state.p[0].discard, member1), "player 1 invalid card moves to player 1 waitroom");
    CHECK(zone_has_id(&game.state.p[1].discard, member2), "player 2 invalid card moves to player 2 waitroom");
    CHECK_EQ(movement_log_n, 2, "two invalid live movements are recorded");
    CHECK_EQ(movement_count_for(member1), 1, "player 1 movement occurs once");
    CHECK_EQ(movement_count_for(member2), 1, "player 2 movement occurs once");
    MovementEvent *p1_moves = movement_for(member1);
    MovementEvent *p2_moves = movement_for(member2);
    CHECK(p1_moves != NULL && p1_moves->cause_player_id == 0, "player 1 movement cause id");
    CHECK(p2_moves != NULL && p2_moves->cause_player_id == 1, "player 2 movement cause id");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_invalid_live_card_discard_records_turn_movements();
    test_source_dest_condition_matches_turn_movements();
    test_source_dest_condition_wrong_source_fails();
    test_source_dest_condition_card_not_in_dest_fails();
    test_source_dest_condition_needs_multiple_fails();
    test_turn_movements_cleared_at_turn_start();
    test_invalid_energy_card_in_live_zone_goes_to_energy_deck();
    test_source_dest_condition_excludes_other_player();
    test_source_dest_condition_opponent_target_includes_other_player();
    test_check_invalid_live_cards_distinguishes_players();
    rb_unload();
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL LIVE CARD ZONE MOVEMENT CHECKS PASSED\n");
    return 0;
}
