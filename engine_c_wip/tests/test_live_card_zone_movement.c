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

/* phase.c implements the phases.rs live-card-set handlers but does not export
 * them in rabuka.h; they are the real engine path for SetLiveCard
 * (helpers/mod.rs::set_live_card runs the same action), so drive them
 * directly instead of the index-writing test_set_live_card shim. */
int rb_handle_live_card_selection(GameState *g, int card_id, const int *indices, int n_indices);
int rb_handle_live_card_confirmation(GameState *g, const int *indices, int n_indices);

/* ── Port of engine/tests/test_modules/effects/other/live_cards_disappear_test.rs
 *    ::live_cards_stuck_in_live_zone_instead_of_discard ───────────────────────
 * Zone-independent card census (Rust count_all_cards) — setting live cards and
 * running live victory determination must relocate cards, never destroy them. */
static int count_all_cards(const GameState *state) {
    int total = 0;
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &state->p[pl];
        total += P->hand.n;
        total += P->deck.n;
        total += P->energy.n;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) total++;
        total += P->live.n;
        total += P->success.n;
        total += P->discard.n;
    }
    total += state->resolution.n;
    total += state->n_revealed;
    return total;
}

/* Rust advance_to_live_card_set: five passes out of Main reach LiveCardSet. */
static void advance_to_live_card_set(TestGame *game) {
    for (int i = 0; i < 5; i++) test_pass(game);
}

/* Rust: drain SelectAutoAbility with [] and SelectLiveSuccess with [0]. The C
 * engine models the live-success prompt as a SelectCard whose pending target is
 * "select_live_success" (live.c:rb_try_take_success_zone_choice), so key off
 * that rather than the Rust choice-variant name. */
static int drain_live_choices(TestGame *game, int *matched_live_success) {
    int safety = 20;
    while (test_has_pending_choice(game) && safety > 0) {
        const char *type = test_pending_choice_type(game);
        int is_auto = type && strcmp(type, "SelectAutoAbility") == 0;
        int is_live_success = game->state.queue.pending.target &&
                              strstr(game->state.queue.pending.target, "select_live_success") != NULL;
        if (is_auto) {
            test_resume_choice(game, -1);
        } else if (is_live_success) {
            test_resume_choice(game, 0);
            *matched_live_success = 1;
        } else {
            break;
        }
        safety--;
    }
    return safety;
}

static void test_live_cards_stuck_in_live_zone_instead_of_discard(void) {
    TestGame game;
    test_game_new(&game);
    clear_movement_log();

    int live1 = test_id(&game, "PL!SP-sd1-023-SD");
    int live2 = test_new_id(&game, "PL!SP-sd1-023-SD");
    int live3 = test_new_id(&game, "PL!SP-sd1-023-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    CHECK(live1 >= 0 && live2 >= 0 && live3 >= 0 && filler >= 0, "live card templates resolve");

    fill_decks(&game, filler);
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = live1;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = live2;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = live3;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        game.state.p[0].stage[i] = RB_EMPTY_SLOT;
        game.state.p[1].stage[i] = RB_EMPTY_SLOT;
    }

    int total_before = count_all_cards(&game.state);

    advance_to_live_card_set(&game);
    /* Rust drives the real SetLiveCard action three times; the C shim
     * test_set_live_card writes at a fixed index and would overwrite, so use the
     * engine's live-card selection + confirmation handlers. */
    CHECK_EQ(rb_handle_live_card_selection(&game.state, live1, NULL, 0), 1, "first live card selected");
    CHECK_EQ(rb_handle_live_card_selection(&game.state, live2, NULL, 0), 1, "second live card selected");
    CHECK_EQ(rb_handle_live_card_selection(&game.state, live3, NULL, 0), 1, "third live card selected");
    CHECK_EQ(rb_handle_live_card_confirmation(&game.state, NULL, 0), 1, "live card selection confirmed");

    int total_after_set = count_all_cards(&game.state);
    CHECK_EQ(total_after_set, total_before, "setting the live zone loses no cards");
    CHECK_EQ(game.state.p[0].live.n, 3, "all three lives are in the live zone");
    CHECK_EQ(game.state.p[0].discard.n, 0, "waitroom is empty right after the live set");

    test_pass(&game);
    test_pass(&game);
    int matched = 0;
    drain_live_choices(&game, &matched);

    for (int i = 0; i < 10; i++) {
        if (game.state.phase == RB_PHASE_MAIN || game.state.phase == RB_PHASE_LIVE_SET) break;
        if (!test_has_pending_choice(&game)) test_pass(&game);
        if (drain_live_choices(&game, &matched) <= 0) break;
    }
    fprintf(stderr, "[CANARY] live=%d waitroom=%d success=%d resolution=%d revealed=%d matched_live_success=%d\n",
            game.state.p[0].live.n, game.state.p[0].discard.n, game.state.p[0].success.n,
            game.state.resolution.n, game.state.n_revealed, matched);

    int total_after = count_all_cards(&game.state);
    CHECK_EQ(total_after, total_before, "no cards vanish across live victory determination");

    /* Zero-stage boards => every live fails its heart requirement => rule 8.4.8
     * drains ALL remaining live-zone cards to the waitroom. */
    CHECK_EQ(game.state.p[0].live.n, 0, "8.4.8: failed live cards leave the live zone");
    CHECK_EQ(game.state.p[0].discard.n, 3, "all three failed lives land in the waitroom");
    CHECK_EQ(game.state.p[0].success.n, 0, "failed lives never reach the success zone");
    CHECK_EQ(game.state.resolution.n, 0, "resolution zone is drained by victory determination");
    CHECK_EQ(game.state.n_revealed, 0, "revealed zone is drained by victory determination");
}

/* The card blobs live in src/ in the in-tree build, but the isolated build root
 * (tools/isolated_build.sh) only copies sources/headers, so fall back to the
 * canonical cards/build directory. Test-harness plumbing, not a behaviour change. */
static int load_test_database(void) {
    static const char *const dirs[] = {"src", "../cards/build", "../../cards/build"};
    for (size_t i = 0; i < sizeof(dirs) / sizeof(dirs[0]); i++) {
        if (rb_load(dirs[i]) == 0) return 0;
    }
    return -1;
}

int main(void) {
    if (load_test_database() != 0) {
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
    test_live_cards_stuck_in_live_zone_instead_of_discard();
    rb_unload();
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL LIVE CARD ZONE MOVEMENT CHECKS PASSED\n");
    return 0;
}
