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

static void fill_decks(TestGame *game, int filler) {
    game->state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(game, 0, filler);
    }
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(game, 1, filler);
    }
}

static void advance_to_live_card_set(TestGame *game) {
    for (int i = 0; i < 5; i++) {
        test_pass(game);
    }
}

static void drain_choices(TestGame *game) {
    while (test_has_pending_choice(game)) {
        test_select_indices(game, NULL, 0);
    }
}

static void test_bp3_005_debut_activates_all_waited_members(void) {
    TestGame game;
    test_game_new(&game);

    int card = test_id(&game, "PL!-bp3-005-R");
    int mate_a = test_id(&game, "PL!-sd1-010-SD");
    int mate_b = test_new_id(&game, "PL!-sd1-010-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    test_give_energy(&game, 20);
    fill_decks(&game, filler);

    test_add_to_stage(&game, 0, mate_a);
    test_add_to_stage(&game, 1, mate_b);
    rb_mods_set_orientation(&game.state.mods, mate_a, "wait");
    rb_mods_set_orientation(&game.state.mods, mate_b, "wait");

    test_add_to_hand(&game, card);
    test_play_to_stage(&game, card, 2);
    drain_choices(&game);

    const char *mate_a_orientation = rb_mods_get_orientation(&game.state.mods, mate_a);
    CHECK(mate_a_orientation && strcmp(mate_a_orientation, "active") == 0,
          "waited teammate A should be activated by the debut");
    const char *mate_b_orientation = rb_mods_get_orientation(&game.state.mods, mate_b);
    CHECK(mate_b_orientation && strcmp(mate_b_orientation, "active") == 0,
          "waited teammate B should be activated by the debut");
}

static void test_combo_bp3_005_mass_activate_readies_self_waited_ability_member(void) {
    TestGame game;
    test_game_new(&game);

    int mass = test_id(&game, "PL!-bp3-005-R");
    int kanata = test_id(&game, "PL!N-pb1-006-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    test_give_energy(&game, 25);
    fill_decks(&game, filler);

    test_add_to_hand(&game, kanata);
    test_play_to_stage(&game, kanata, 0);
    drain_choices(&game);

    int energy_before = game.state.p[0].energy_active;
    test_activate_ability(&game, kanata);
    drain_choices(&game);
    const char *kanata_orientation = rb_mods_get_orientation(&game.state.mods, kanata);
    CHECK(kanata_orientation && strcmp(kanata_orientation, "wait") == 0,
          "cost: kanata should be waited");
    CHECK(game.state.p[0].energy_active == energy_before + 1,
          "effect: one energy activated");

    test_add_to_hand(&game, mass);
    test_play_to_stage(&game, mass, 1);
    drain_choices(&game);

    kanata_orientation = rb_mods_get_orientation(&game.state.mods, kanata);
    CHECK(kanata_orientation && strcmp(kanata_orientation, "active") == 0,
          "bp3-005 debut should re-activate the self-waited kanata");
    int e2 = game.state.p[0].energy_active;
    test_activate_ability(&game, kanata);
    drain_choices(&game);
    CHECK(game.state.p[0].energy_active == e2 + 1,
          "reactivated kanata should be able to activate again");
}

static void test_bp3_001_live_start_activates_one_chosen_member(void) {
    TestGame game;
    test_game_new(&game);

    int card = test_id(&game, "PL!-bp3-001-R");
    int sleeper = test_id(&game, "PL!-sd1-010-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int live_card = test_id(&game, "PL!-sd1-020-SD");

    test_give_energy(&game, 15);
    fill_decks(&game, filler);

    test_add_to_stage(&game, 0, sleeper);
    rb_mods_set_orientation(&game.state.mods, sleeper, "wait");

    test_add_to_hand(&game, card);
    test_play_to_stage(&game, card, 1);
    drain_choices(&game);

    test_add_to_hand(&game, live_card);
    advance_to_live_card_set(&game);
    test_set_live_card(&game, 0, live_card);

    int hand_before = game.state.p[0].hand.n;
    int p1_zone = game.state.p[0].live.n;
    test_pass(&game);
    test_pass(&game);

    int safety = 0;
    while (test_has_pending_choice(&game) && safety < 30) {
        safety++;
        const char *choice_type = test_pending_choice_type(&game);
        if (strcmp(choice_type, "SelectAutoAbility") == 0) {
            int selected[] = {0};
            test_select_indices(&game, selected, 1);
        } else if (strcmp(choice_type, "SelectCard") == 0) {
            const RbChoice *choice = rb_get_pending_choice(&game.state);
            if (!choice || strcmp(choice->zone, "stage") != 0 || !choice->allow_skip) {
                if (choice) {
                    fprintf(stderr, "expected skippable stage SelectCard, got kind=%d zone=%s allow_skip=%d description=%s\n",
                            choice->kind, choice->zone, choice->allow_skip, choice->description);
                } else {
                    fprintf(stderr, "expected skippable stage SelectCard, got no choice\n");
                }
                failures++;
            }
            int selected[] = {0};
            test_select_indices(&game, selected, 1);
        } else {
            test_select_indices(&game, NULL, 0);
        }
    }

    const char *sleeper_orientation = rb_mods_get_orientation(&game.state.mods, sleeper);
    CHECK(sleeper_orientation && strcmp(sleeper_orientation, "active") == 0,
          "live-start should activate the chosen waited member");
    CHECK(game.state.p[0].hand.n == hand_before + p1_zone,
          "activate-one draws nothing beyond the live-zone refill");
}

static void test_bp3_001_live_start_can_skip_activation(void) {
    TestGame game;
    test_game_new(&game);

    int card = test_id(&game, "PL!-bp3-001-R");
    int sleeper = test_id(&game, "PL!-sd1-010-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int live_card = test_id(&game, "PL!-sd1-020-SD");

    test_give_energy(&game, 15);
    fill_decks(&game, filler);

    test_add_to_stage(&game, 0, sleeper);
    rb_mods_set_orientation(&game.state.mods, sleeper, "wait");

    test_add_to_hand(&game, card);
    test_play_to_stage(&game, card, 1);
    drain_choices(&game);

    test_add_to_hand(&game, live_card);
    advance_to_live_card_set(&game);
    test_set_live_card(&game, 0, live_card);
    test_pass(&game);
    test_pass(&game);

    int safety = 0;
    while (test_has_pending_choice(&game) && safety < 30) {
        safety++;
        const char *choice_type = test_pending_choice_type(&game);
        if (strcmp(choice_type, "SelectAutoAbility") == 0) {
            int selected[] = {0};
            test_select_indices(&game, selected, 1);
        } else {
            test_select_indices(&game, NULL, 0);
        }
    }

    const char *sleeper_orientation = rb_mods_get_orientation(&game.state.mods, sleeper);
    CHECK(sleeper_orientation && strcmp(sleeper_orientation, "wait") == 0,
          "declined activation: member must stay waited");
}

static int discard_has(TestGame *game, int card_id)
{
    for (int i = 0; i < game->state.p[0].discard.n; i++)
        if (game->state.p[0].discard.cards[i] == card_id) return 1;
    return 0;
}

static void test_kinako_baton_recovery(void)
{
    TestGame game;
    test_game_new(&game);
    int kinako = test_id(&game, "PL!SP-bp2-006-P");
    int liela = test_id(&game, "PL!SP-pb1-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    CHECK(kinako >= 0 && liela >= 0 && filler >= 0, "Kinako recovery fixtures resolve");
    if (kinako < 0 || liela < 0 || filler < 0) return;
    test_give_energy(&game, 25);
    fill_decks(&game, filler);
    test_add_to_stage(&game, 1, liela);
    test_add_to_hand(&game, kinako);
    test_play_to_stage(&game, kinako, 1);
    drain_choices(&game);
    CHECK(test_stage_has(&game, 1, kinako), "Kinako occupies the baton-touched area");
    CHECK(test_hand_has(&game, liela), "Kinako recovers a displaced Liella! member");
    CHECK(!discard_has(&game, liela), "recovered Liella! member is not left in waitroom");
}

static void test_suppress_ability_trigger_is_persistent(void) {
    GameState state;
    memset(&state, 0, sizeof(state));
    state.p[0].stage[0] = 1;
    AbilityEffect effect;
    memset(&effect, 0, sizeof(effect));
    effect.n_extra = 1;
    effect.extra_k[0] = (char *)"suppressed_trigger";
    effect.extra_v[0] = (char *)"live_start";

    rb_suppress_ability_trigger(&state, 0, &effect, 1);
    CHECK(rb_is_trigger_suppressed(&state, 0, "live_start"),
          "executed suppression persists in game state");
    rb_suppress_ability_trigger(&state, 0, &effect, 1);
    CHECK(state.n_prohibition == 1, "repeated suppression registration is deduplicated");
    CHECK(!rb_is_trigger_suppressed(&state, 1, "live_start"),
          "suppression remains player-specific");
    CHECK(!rb_is_trigger_suppressed(&state, 0, "live_success"),
          "suppression does not affect unrelated triggers");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_bp3_005_debut_activates_all_waited_members();
    test_combo_bp3_005_mass_activate_readies_self_waited_ability_member();
    test_bp3_001_live_start_activates_one_chosen_member();
    test_bp3_001_live_start_can_skip_activation();
    test_kinako_baton_recovery();
    test_suppress_ability_trigger_is_persistent();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL MEMBER ACTIVATION DEBUT AND LIVE START CHECKS PASSED\n");
    return 0;
}
