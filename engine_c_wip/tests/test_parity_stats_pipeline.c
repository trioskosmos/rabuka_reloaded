/* Parity suite: stats_pipeline (member stat resolution) + the Rust
 * effects/score/{card_score,per_card} test cluster.
 *
 * Rust sources mirrored here:
 *   engine/src/core/stats_pipeline.rs                                  (unit shapes)
 *   engine/tests/helpers/mod.rs:114 fire_trigger                      (trigger shim)
 *   engine/tests/test_modules/effects/score/card_score/*              (per-card scores)
 *   engine/tests/test_modules/effects/score/per_card/*                (per-card scores)
 *
 * Two groups of assertions:
 *   A. Direct unit assertions on the exported stats_pipeline API
 *      (member_original_hearts / apply_additive_heart_mods /
 *       effective_blade_parts / member_heart_detail / effective_blade /
 *       effective_need_heart / need_satisfied / stage_hearts_pipeline).
 *   B. End-to-end per-card score recalculation, driven either by firing the
 *      card's live-start / live-success ability directly (mirroring the Rust
 *      fire_trigger helper) or by inspecting the modifiers the pipeline left.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* Trigger wire tokens (engine/src/ability/enums.rs ability trigger strings). */
#define TRIG_LIVE_START  "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE8\xB5\xB7\xE5\xA7\x8B\xE6\x99\x82"      /* ライブ開始時 */
#define TRIG_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"      /* ライブ成功時 */

/* ── fire_trigger shim (engine/tests/helpers/mod.rs:114) ───────────────
 * rb_trigger_auto_ability exists in src/core/game_state_abilities.c but has
 * no prototype in include/rabuka.h, so declare it locally exactly as
 * defined there. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static void par_drain_choices(TestGame *game) {
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 32) {
        rb_resume_with_choice(&game->state, -1);
    }
}

/* Mirror Rust fire_trigger/fire_trigger_nth: find the `nth` ability whose
 * `triggers` string equals `trig`, build "<card_no>_<full_text>", queue it
 * for the live-start/live-success trigger and process the queue. */
static int par_fire_trigger(TestGame *game, int cid, const char *trig,
                            const char *trigger_type, int nth) {
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof(card_no), "%s", rb_card_string(card.card_no_idx));
    char ability_id[512];
    int found = 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab && !found; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            if (nth == 0) {
                snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
                         ab.full_text ? ab.full_text : "");
                found = 1;
            } else {
                nth--;
            }
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
    if (!found) return 0;
    rb_trigger_auto_ability(&game->state, ability_id, trigger_type, 0, card_no,
                            cid, NULL, 0, -1);
    game->state.activating_card = cid;
    rb_process_pending_auto_abilities(&game->state);
    par_drain_choices(game);
    return 1;
}

static int par_fire_live_start(TestGame *game, int cid) {
    return par_fire_trigger(game, cid, TRIG_LIVE_START, TRIG_LIVE_START, 0);
}

static int par_fire_live_success(TestGame *game, int cid) {
    return par_fire_trigger(game, cid, TRIG_LIVE_SUCCESS, TRIG_LIVE_SUCCESS, 0);
}

static void par_fill_decks(TestGame *game, int filler) {
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 40; i++) {
        test_add_to_deck(game, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static int par_hearts_total(const int hearts[8]) {
    int total = 0;
    for (int i = 0; i < 8; i++) total += hearts[i];
    return total;
}

/* ??????????????????????????????????????????????????????????????????
   A. stats_pipeline.rs unit shapes
   ?????????????????????????????????????????????????????????????????? */

/* stats_pipeline.rs:22-57 member_original_hearts ? printed base, then the
 * copy / multiplier / override layering. */
static void a_member_original_hearts_layers(void) {
    TestGame game;
    test_game_new(&game);

    int big = test_id(&game, "PL!SP-pb2-005-R");     /* {02:3, 03:3, 06:3} */
    int other = test_id(&game, "PL!SP-sd1-002-SD");  /* 3 printed hearts   */

    int base[8];
    rb_member_original_hearts(&game.state.mods, big, base);
    CHECK_EQ(base[2], 3, "member_original_hearts base: heart02 printed = 3");
    CHECK_EQ(base[3], 3, "member_original_hearts base: heart03 printed = 3");
    CHECK_EQ(base[6], 3, "member_original_hearts base: heart06 printed = 3");
    CHECK_EQ(par_hearts_total(base), 9,
             "member_original_hearts base totals 9 printed hearts");

    int other_base[8];
    rb_member_original_hearts(&game.state.mods, other, other_base);
    int other_total = par_hearts_total(other_base);

    /* 9.9.1.3 heart_copy: originals become the referenced card's hearts. */
    rb_mods_set_heart_copy(&game.state.mods, big, other);
    int copied[8];
    rb_member_original_hearts(&game.state.mods, big, copied);
    CHECK_EQ(par_hearts_total(copied), other_total,
             "heart_copy replaces the base with the source card's hearts");
    CHECK_EQ(memcmp(copied, other_base, sizeof(copied)), 0,
             "heart_copy yields the source card's exact heart multiset");
    rb_mods_clear_card(&game.state.mods, big);

    /* The whole multiset collapses into one colour. */
    rb_mods_set_heart_color_multiplier(&game.state.mods, big, RB_HEART_PURPLE);
    int collapsed[8];
    rb_member_original_hearts(&game.state.mods, big, collapsed);
    CHECK_EQ(collapsed[RB_HEART_PURPLE], 9,
             "heart multiplier collapses all 9 hearts into one colour");
    CHECK_EQ(par_hearts_total(collapsed) - collapsed[RB_HEART_PURPLE], 0,
             "heart multiplier clears every other colour");
    rb_mods_clear_card(&game.state.mods, big);

    /* 9.9.1.4 heart_override replaces layers 1-3 outright. */
    rb_mods_set_heart_override(&game.state.mods, big, RB_HEART_GREEN, 4);
    int overridden[8];
    rb_member_original_hearts(&game.state.mods, big, overridden);
    CHECK_EQ(overridden[RB_HEART_GREEN], 4,
             "heart_override sets the printed heart count");
    CHECK_EQ(par_hearts_total(overridden) - overridden[RB_HEART_GREEN], 0,
             "heart_override replaces the original hearts outright");
}

/* stats_pipeline.rs:60-73 apply_additive_heart_mods (9.9.1.5). */
static void a_apply_additive_heart_mods(void) {
    RbMods mods;
    rb_mods_init(&mods);

    int hearts[8] = {0, 2, 2, 1, 0, 0, 0, 0};
    RbModifierEntry entries[8];
    memset(entries, 0, sizeof(entries));
    entries[2].add = 3;    /* 2 -> 5 */
    entries[1].add = -4;   /* 2 -> 0, entry removed */
    entries[3].add = -9;   /* saturates to 0, entry removed */

    rb_apply_additive_heart_mods(hearts, entries);
    CHECK_EQ(hearts[2], 5, "additive heart mod stacks on top of the base");
    CHECK_EQ(hearts[1], 0, "negative additive saturates at 0 (entry removed)");
    CHECK_EQ(hearts[3], 0, "additive below zero saturates at 0 (entry removed)");

    int noop[8] = {0, 0, 4, 0, 0, 0, 0, 0};
    RbModifierEntry clean[8];
    memset(clean, 0, sizeof(clean));
    rb_apply_additive_heart_mods(noop, clean);
    CHECK_EQ(noop[2], 4, "zero-delta additive mods leave the hearts untouched");
    (void)mods;
}

/* stats_pipeline.rs:78-90 / 187-201 blade layering. */
static void a_effective_blade(void) {
    TestGame game;
    test_game_new(&game);

    int blader = test_id(&game, "PL!S-PR-014-PR");   /* printed blade 6 */
    Card card;
    CHECK(rb_decode_card_by_index((uint32_t)blader, &card), "blade fixture decodes");
    int printed = card.blade;
    rb_free_card(&card);
    CHECK_EQ(printed, 6, "PL!S-PR-014-PR printed blade is 6");

    RbModifierEntry add = {0, 0};
    add.add = 3;
    int base = 0, additive = 0;
    rb_effective_blade_parts(add, printed, &base, &additive);
    CHECK_EQ(base, printed, "additive-only blade keeps the printed base");
    CHECK_EQ(additive, 3, "additive-only blade reports the bonus separately");
    CHECK_EQ(rb_effective_blade(blader, add), printed + 3,
             "effective_blade folds additive on top of the printed blade");

    RbModifierEntry set = {0, 0};
    set.set = 4;
    rb_effective_blade_parts(set, printed, &base, &additive);
    CHECK_EQ(base, 4, "non-zero SET replaces the printed blade base");
    CHECK_EQ(additive, 0, "non-zero SET leaves no additive bonus");
    CHECK_EQ(rb_effective_blade(blader, set), 4,
             "effective_blade uses the SET value when one is present");

    RbModifierEntry both = {5, 2};
    CHECK_EQ(rb_effective_blade(blader, both), 7,
             "SET + additive total is the effective blade");
}

/* stats_pipeline.rs:143-183 member_heart_detail ? base vs positive-only
 * bonus array. */
static void a_member_heart_detail(void) {
    TestGame game;
    test_game_new(&game);

    int member = test_id(&game, "PL!SP-pb2-005-R");
    uint8_t base[8], bonus[8];
    rb_member_heart_detail(&game.state.mods, member, base, bonus);
    CHECK_EQ(base[2], 3, "heart detail base keeps printed heart02 = 3");
    CHECK_EQ(bonus[2], 0, "heart detail bonus is empty with no modifiers");

    rb_mods_add_heart(&game.state.mods, member, 2, 4);
    rb_mods_add_heart(&game.state.mods, member, 3, -3);
    rb_member_heart_detail(&game.state.mods, member, base, bonus);
    CHECK_EQ(base[2], 3, "heart detail base excludes additive modifiers");
    CHECK_EQ(bonus[2], 4, "heart detail bonus carries the positive delta");
    CHECK_EQ(bonus[3], 0, "heart detail bonus ignores negative deltas");
}

/* stats_pipeline.rs:94-137 stage_hearts. */
static void a_stage_hearts_pipeline(void) {
    TestGame game;
    test_game_new(&game);

    int big = test_id(&game, "PL!SP-pb2-005-R");      /* 9 printed hearts */
    int small = test_new_id(&game, "PL!SP-bp4-004-P"); /* 8 printed hearts */
    game.state.p[0].stage[0] = big;
    game.state.p[0].stage[1] = small;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;

    int out[8];
    rb_stage_hearts_pipeline(&game.state, 0, out);
    CHECK_EQ(out[2], 6, "stage hearts sum heart02 across the two members (3+3)");
    CHECK_EQ(out[3], 6, "stage hearts sum heart03 across the two members (3+3)");
    CHECK_EQ(par_hearts_total(out), 17, "stage hearts total 9 + 8 = 17");

    /* 9.9.1.5: additives stack on top of whatever 1-4 produced. */
    rb_mods_add_heart(&game.state.mods, big, 2, 2);
    rb_stage_hearts_pipeline(&game.state, 0, out);
    CHECK_EQ(out[2], 8, "stage hearts add the member's additive heart modifier");
    CHECK_EQ(par_hearts_total(out), 19, "stage heart total follows the additive");
}

/* stats_pipeline.rs:205-254 effective_need_heart + 259-280 need_satisfied. */
static void a_effective_need_heart_and_satisfied(void) {
    TestGame game;
    test_game_new(&game);

    int live = test_id(&game, "PL!S-bp1-002-L");
    int out[8];
    rb_effective_need_heart(&game.state, live, out);
    int printed_total = par_hearts_total(out);
    CHECK(printed_total > 0, "fixture live card has a printed need");

    /* No modifiers -> printed need is returned unchanged. */
    int printed[8];
    memcpy(printed, out, sizeof(printed));

    /* Q115/Q127: per-colour SET applies first, additive stacks after. */
    rb_mods_set_need_heart(&game.state.mods, live, 2, 1);
    rb_effective_need_heart(&game.state, live, out);
    CHECK_EQ(out[2], 1, "need_heart SET replaces the printed count for that colour");
    rb_mods_add_need_heart(&game.state.mods, live, 2, 2);
    rb_effective_need_heart(&game.state, live, out);
    CHECK_EQ(out[2], 3, "need_heart additive stacks on top of the SET value");
    rb_mods_clear_card(&game.state.mods, live);
    rb_effective_need_heart(&game.state, live, out);
    CHECK_EQ(memcmp(out, printed, sizeof(out)), 0,
             "cleared need_heart mods restore the printed need");

    /* need_satisfied delegates to check_heart_requirement. */
    int none[8] = {0, 0, 0, 0, 0, 0, 0, 0};
    CHECK(rb_need_satisfied(none, none, live, &game.state.mods),
          "an empty need is always satisfied");
    CHECK(!rb_need_satisfied(printed, none, live, &game.state.mods),
          "a printed need with no hearts provided is not satisfied");

    int provided[8] = {0, 0, 0, 0, 0, 0, 0, 0};
    for (int c = 0; c < 8; c++) provided[c] = printed[c];
    CHECK(rb_need_satisfied(printed, provided, live, &game.state.mods),
          "an exactly matching heart pool satisfies the need");

    /* A wildcard (heart00) can cover an arbitrary colour shortfall. */
    int wildcard[8] = {0, 0, 0, 0, 0, 0, 0, 0};
    wildcard[0] = printed_total;
    CHECK(rb_need_satisfied(printed, wildcard, live, &game.state.mods),
          "a heart00 wildcard pool satisfies an arbitrary printed need");

    /* A need_heart modifier that empties the requirement short-circuits to true. */
    rb_mods_set_need_heart(&game.state.mods, live, 3, 1);
    rb_mods_add_need_heart(&game.state.mods, live, 3, -1);
    CHECK(rb_need_satisfied(printed, none, live, &game.state.mods),
          "a need reduced to nothing by modifiers is satisfied");
    rb_mods_clear_card(&game.state.mods, live);
}

/* ??????????????????????????????????????????????????????????????????
   B. effects/score cluster
   ?????????????????????????????????????????????????????????????????? */

/* card_score/aqours_heart04_threshold_score_pl_s_pb1_020_l_test.rs */
static void b_pb1_020_l_heart04_total_threshold(void) {
    TestGame game;
    test_game_new(&game);
    int live = test_id(&game, "PL!S-pb1-020-L");
    test_add_to_live(&game, live);
    int a1 = test_id(&game, "PL!S-bp5-007-R");
    int a2 = test_new_id(&game, "PL!S-bp5-007-R");
    game.state.p[0].stage[0] = a1;
    game.state.p[0].stage[1] = a2;
    CHECK(par_fire_live_start(&game, live), "PL!S-pb1-020-L has a live-start ability");
    CHECK_EQ(test_get_score_modifier(&game, live), 2,
             "combined printed heart04 = 10 -> score +2");

    TestGame game2;
    test_game_new(&game2);
    int live2 = test_id(&game2, "PL!S-pb1-020-L");
    test_add_to_live(&game2, live2);
    game2.state.p[0].stage[0] = test_id(&game2, "PL!S-bp5-007-R");
    CHECK(par_fire_live_start(&game2, live2), "PL!S-pb1-020-L re-fires on a one-member stage");
    CHECK_EQ(test_get_score_modifier(&game2, live2), 0,
             "combined printed heart04 = 5 -> no score");
}

/* card_score/energy_threshold_score_pl_sp_sd1_026_sd_srl_test.rs */
static void b_sd1_026_sd_energy_threshold(void) {
    const char *variants[2] = {"PL!SP-sd1-026-SD", "PL!SP-sd1-026-SRL"};
    for (int v = 0; v < 2; v++) {
        TestGame game;
        test_game_new(&game);
        int live = test_id(&game, variants[v]);
        test_add_to_live(&game, live);
        test_give_energy(&game, 9);
        CHECK(par_fire_live_start(&game, live), "energy-threshold live start ability found");
        CHECK_EQ(test_get_score_modifier(&game, live), 1, "9 energy -> score +1");
    }

    TestGame game;
    test_game_new(&game);
    int live = test_id(&game, "PL!SP-sd1-026-SD");
    test_add_to_live(&game, live);
    test_give_energy(&game, 8);
    CHECK(par_fire_live_start(&game, live), "8-energy variant re-fires the same ability");
    CHECK_EQ(test_get_score_modifier(&game, live), 0, "8 energy -> no score");
}

/* card_score/liella_heart_total_score_test.rs */
static void b_bp5_026_l_group_heart_total(void) {
    TestGame game;
    test_game_new(&game);
    int filler = test_new_id(&game, "PL!-sd1-010-SD");
    par_fill_decks(&game, filler);
    int live = test_id(&game, "PL!SP-bp5-026-L");
    test_add_to_live(&game, live);
    int big1 = test_id(&game, "PL!SP-pb2-005-R");    /* 9 hearts */
    int big2 = test_id(&game, "PL!SP-bp4-004-P");    /* 8 hearts */
    game.state.p[0].stage[0] = big1;
    game.state.p[0].stage[1] = big2;
    game.state.p[0].stage[2] = test_new_id(&game, "PL!-sd1-010-SD"); /* mu's, excluded */
    CHECK(par_fire_live_start(&game, live), "PL!SP-bp5-026-L has a live-start ability");
    CHECK_EQ(test_get_score_modifier(&game, live), 1,
             "two Liella! members' base hearts total 17 >= 11 -> score +1");

    TestGame low;
    test_game_new(&low);
    par_fill_decks(&low, filler);
    int live2 = test_id(&low, "PL!SP-bp5-026-L");
    test_add_to_live(&low, live2);
    low.state.p[0].stage[0] = test_id(&low, "PL!SP-pb2-036-N");   /* 2 hearts */
    low.state.p[0].stage[1] = test_id(&low, "PL!SP-pb2-037-N");   /* 2 hearts */
    low.state.p[0].stage[2] = test_new_id(&low, "PL!-sd1-010-SD");
    CHECK(par_fire_live_start(&low, live2), "PL!SP-bp5-026-L re-fires on the low stage");
    CHECK_EQ(test_get_score_modifier(&low, live2), 0, "total 4 < 11 -> no bonus");
}

/* card_score/distinct_waitroom_lives_pl_n_bp4_028_l_test.rs */
static void b_bp4_028_l_distinct_waitroom_lives(void) {
    static const char *niji[6] = {
        "PL!N-bp1-025-L", "PL!N-bp1-026-L", "PL!N-bp1-027-L",
        "PL!N-bp1-028-L", "PL!N-bp1-029-L", "PL!N-sd1-025-SD"
    };

    TestGame four;
    test_game_new(&four);
    int swc = test_id(&four, "PL!N-bp4-028-L");
    test_add_to_live(&four, swc);
    for (int i = 0; i < 4; i++) test_add_to_discard(&four, test_id(&four, niji[i]));
    CHECK(par_fire_live_start(&four, swc), "PL!N-bp4-028-L has a live-start ability");
    CHECK_EQ(test_get_score_modifier(&four, swc), 1,
             "4 distinct-name Nijigasaki lives -> +1");

    TestGame six;
    test_game_new(&six);
    int swc6 = test_id(&six, "PL!N-bp4-028-L");
    test_add_to_live(&six, swc6);
    for (int i = 0; i < 6; i++) test_add_to_discard(&six, test_id(&six, niji[i]));
    CHECK(par_fire_live_start(&six, swc6), "PL!N-bp4-028-L re-fires on a 6-live waitroom");
    CHECK_EQ(test_get_score_modifier(&six, swc6), 2,
             "6 distinct-name Nijigasaki lives -> +2 instead");

    TestGame dup;
    test_game_new(&dup);
    int swc_dup = test_id(&dup, "PL!N-bp4-028-L");
    test_add_to_live(&dup, swc_dup);
    for (int i = 0; i < 6; i++) test_add_to_discard(&dup, test_new_id(&dup, niji[0]));
    CHECK(par_fire_live_start(&dup, swc_dup), "PL!N-bp4-028-L re-fires on duplicate names");
    CHECK_EQ(test_get_score_modifier(&dup, swc_dup), 0,
             "6 same-name lives are not 4 cards with distinct names -> nothing");

    TestGame three;
    test_game_new(&three);
    int swc3 = test_id(&three, "PL!N-bp4-028-L");
    test_add_to_live(&three, swc3);
    for (int i = 0; i < 3; i++) test_add_to_discard(&three, test_id(&three, niji[i]));
    CHECK(par_fire_live_start(&three, swc3), "PL!N-bp4-028-L re-fires on a 3-live waitroom");
    CHECK_EQ(test_get_score_modifier(&three, swc3), 0, "3 distinct < 4 -> nothing");
}

/* card_score/opponent_energy_gated_score_test.rs */
static void b_bp6_022_l_opponent_energy_gate(void) {
    TestGame game;
    test_game_new(&game);
    int live = test_id(&game, "PL!S-bp6-022-L");
    test_add_to_live(&game, live);
    test_give_energy(&game, 1);
    for (int i = 0; i < 3; i++) test_give_opp_energy(&game, 1);
    CHECK(par_fire_live_success(&game, live), "PL!S-bp6-022-L has a live-success ability");
    CHECK_EQ(test_get_score_modifier(&game, live), 1,
             "opponent 3 > self 1 -> score +1");

    TestGame tie;
    test_game_new(&tie);
    int live2 = test_id(&tie, "PL!S-bp6-022-L");
    test_add_to_live(&tie, live2);
    test_give_energy(&tie, 2);
    for (int i = 0; i < 2; i++) test_give_opp_energy(&tie, 1);
    CHECK(par_fire_live_success(&tie, live2), "PL!S-bp6-022-L re-fires on an energy tie");
    CHECK_EQ(test_get_score_modifier(&tie, live2), 0,
             "opponent 2 vs self 2 is not more -> no score");
}

/* card_score/revealed_distinct_liella_score_test.rs */
static void b_bp4_026_l_revealed_distinct_liella(void) {
    static const char *cards[5] = {
        "PL!SP-pb1-001-PR", "PL!SP-bp1-004-PR", "PL!SP-bp1-016-PR",
        "PL!SP-bp1-018-PR", "PL!SP-PR-017-PR"
    };

    TestGame game;
    test_game_new(&game);
    int live = test_id(&game, "PL!SP-bp4-026-L");
    test_add_to_live(&game, live);
    for (int i = 0; i < 5; i++) {
        int id = test_id(&game, cards[i]);
        game.state.revealed_cards[game.state.n_revealed++] = id;
    }
    CHECK(par_fire_live_success(&game, live), "PL!SP-bp4-026-L has a live-success ability");
    CHECK_EQ(test_get_score_modifier(&game, live), 1,
             "5 distinct Liella! members revealed -> score +1");

    TestGame four;
    test_game_new(&four);
    int live4 = test_id(&four, "PL!SP-bp4-026-L");
    test_add_to_live(&four, live4);
    for (int i = 0; i < 4; i++) {
        int id = test_id(&four, cards[i]);
        four.state.revealed_cards[four.state.n_revealed++] = id;
    }
    CHECK(par_fire_live_success(&four, live4), "PL!SP-bp4-026-L re-fires on 4 revealed");
    CHECK_EQ(test_get_score_modifier(&four, live4), 0,
             "only 4 distinct Liella! members -> no score");
}

/* card_score/live_success_group_heart_gated_live_score_test.rs
 * Rust drives TurnEngine::trigger_live_success_abilities; the C gate lives
 * behind rb_trigger_live_success, so the same ability is fired directly. */
static void b_bp7_022_l_revealed_group_hearts(void) {
    TestGame game;
    test_game_new(&game);
    int live = test_id(&game, "PL!S-bp7-022-L");
    test_add_to_live(&game, live);
    game.state.stage_hearts[0][2] = 3;  /* heart02 */
    game.state.stage_hearts[0][4] = 3;  /* heart04 */
    game.state.stage_hearts[0][5] = 3;  /* heart05 */
    game.state.stage_hearts[0][0] = 10; /* heart00 */
    game.state.revealed_cards[game.state.n_revealed++] =
        test_new_id(&game, "PL!S-sd1-001-SD");   /* Aqours member */
    game.state.phase = RB_PHASE_VICTORY;
    CHECK(par_fire_live_success(&game, live), "PL!S-bp7-022-L has a live-success ability");
    CHECK_EQ(test_get_score_modifier(&game, live), 1,
             "Aqours hearts in the yell -> score +1");

    TestGame other;
    test_game_new(&other);
    int live2 = test_id(&other, "PL!S-bp7-022-L");
    test_add_to_live(&other, live2);
    other.state.stage_hearts[0][2] = 3;
    other.state.stage_hearts[0][4] = 3;
    other.state.stage_hearts[0][5] = 3;
    other.state.stage_hearts[0][0] = 10;
    other.state.revealed_cards[other.state.n_revealed++] =
        test_new_id(&other, "PL!N-PR-026-PR");
    other.state.revealed_cards[other.state.n_revealed++] =
        test_new_id(&other, "PL!N-bp1-021-N");
    other.state.phase = RB_PHASE_VICTORY;
    CHECK(par_fire_live_success(&other, live2), "PL!S-bp7-022-L re-fires on non-Aqours reveals");
    CHECK_EQ(test_get_score_modifier(&other, live2), 0,
             "non-Aqours hearts in the yell -> no score");
}

/* card_score/live_start_total_blade_threshold_live_score_q116_test.rs */
static void b_sd1_028_sd_total_blade_threshold(void) {
    TestGame game;
    test_game_new(&game);
    int f = test_id(&game, "PL!-sd1-010-SD");
    par_fill_decks(&game, f);
    int dream = test_id(&game, "PL!N-sd1-028-SD");
    game.state.p[0].stage[0] = test_id(&game, "PL!S-PR-014-PR");  /* blade 6 */
    game.state.p[0].stage[1] = test_new_id(&game, "PL!S-PR-014-PR");
    game.state.p[0].stage[2] = test_new_id(&game, "PL!-sd1-010-SD");
    test_add_to_hand(&game, dream);
    CHECK(par_fire_live_start(&game, dream), "PL!N-sd1-028-SD has a live-start ability");
    CHECK_EQ(test_get_score_modifier(&game, dream), 1, "Blade >= 10 -> score +1");

    TestGame low;
    test_game_new(&low);
    par_fill_decks(&low, f);
    int dream2 = test_id(&low, "PL!N-sd1-028-SD");
    low.state.p[0].stage[0] = test_id(&low, "PL!S-PR-014-PR");
    low.state.p[0].stage[1] = test_id(&low, "PL!-sd1-002-SD");   /* blade 1 */
    low.state.p[0].stage[2] = test_new_id(&low, "PL!-sd1-010-SD");
    test_add_to_hand(&low, dream2);
    CHECK(par_fire_live_start(&low, dream2), "PL!N-sd1-028-SD re-fires below the threshold");
    CHECK_EQ(test_get_score_modifier(&low, dream2), 0, "Blade < 10 -> no score");
}

/* per_card/solitude_rain_unique_heart_color_score_q67_test.rs */
static void b_bp1_027_l_unique_heart_colors(void) {
    TestGame game;
    test_game_new(&game);
    int filler = test_id(&game, "PL!-sd1-010-SD");
    par_fill_decks(&game, filler);
    int solitude = test_id(&game, "PL!N-bp1-027-L");
    game.state.p[0].stage[0] = test_id(&game, "PL!N-sd1-001-SD"); /* heart01+02+04 */
    game.state.p[0].stage[1] = test_new_id(&game, "PL!-sd1-010-SD");
    game.state.p[0].stage[2] = test_new_id(&game, "PL!-sd1-010-SD");
    test_add_to_hand(&game, solitude);
    CHECK(par_fire_live_start(&game, solitude), "PL!N-bp1-027-L has a live-start ability");
    CHECK_EQ(test_get_score_modifier(&game, solitude), 3,
             "3 unique heart colours across Nijigasaki members -> score +3");

    TestGame other;
    test_game_new(&other);
    par_fill_decks(&other, filler);
    int solitude2 = test_id(&other, "PL!N-bp1-027-L");
    other.state.p[0].stage[0] = test_id(&other, "PL!-sd1-002-SD"); /* mu's */
    other.state.p[0].stage[1] = test_new_id(&other, "PL!-sd1-010-SD");
    other.state.p[0].stage[2] = test_new_id(&other, "PL!-sd1-010-SD");
    test_add_to_hand(&other, solitude2);
    CHECK(par_fire_live_start(&other, solitude2), "PL!N-bp1-027-L re-fires with a mu's stage");
    CHECK_EQ(test_get_score_modifier(&other, solitude2), 0,
             "non-Nijigasaki members contribute no score");
}

/* card_score/live_start_group_member_four_heart_score_test.rs */
static void b_pb2_045_l_four_heart_liella_member(void) {
    TestGame game;
    test_game_new(&game);
    int filler = test_id(&game, "PL!-sd1-010-SD");
    par_fill_decks(&game, filler);
    int lover = test_id(&game, "PL!SP-pb2-045-L");
    game.state.p[0].stage[0] = test_id(&game, "PL!SP-sd1-002-SD");   /* 3 hearts */
    game.state.p[0].stage[1] = RB_EMPTY_SLOT;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, lover);
    CHECK(par_fire_live_start(&game, lover), "PL!SP-pb2-045-L has a live-start ability");
    CHECK_EQ(test_get_score_modifier(&game, lover), 0,
             "no Liella! member with 4+ hearts -> no score");

    TestGame qualified;
    test_game_new(&qualified);
    par_fill_decks(&qualified, filler);
    int lover2 = test_id(&qualified, "PL!SP-pb2-045-L");
    qualified.state.p[0].stage[0] = test_id(&qualified, "PL!SP-sd1-001-SD"); /* 4 hearts */
    qualified.state.p[0].stage[1] = test_id(&qualified, "PL!-sd1-010-SD");   /* mu's */
    qualified.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&qualified, lover2);
    CHECK(par_fire_live_start(&qualified, lover2), "PL!SP-pb2-045-L re-fires with a qualified member");
    CHECK_EQ(test_get_score_modifier(&qualified, lover2), 1,
             "only the Liella! member with 4+ hearts counts -> +1");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    if (getenv("PARITY_DEBUG")) {
        static const char *dbg[8] = {
            "PL!S-pb1-020-L", "PL!SP-sd1-026-SD", "PL!SP-bp5-026-L",
            "PL!S-bp6-022-L", "PL!SP-bp4-026-L", "PL!S-bp7-022-L",
            "PL!N-sd1-028-SD", "PL!N-bp1-027-L"
        };
        for (int d = 0; d < 8; d++) {
            int cid = rb_find_card_by_no(dbg[d]);
            printf("CARD %s id=%d\n", dbg[d], cid);
            if (cid < 0) continue;
            int nab = rb_card_num_abilities((uint32_t)cid);
            for (int a = 0; a < nab; a++) {
                Ability ab;
                if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
                printf("  ab[%d] triggers=[%s] full=[%s]\n", a,
                       ab.triggers ? ab.triggers : "(null)",
                       ab.full_text ? ab.full_text : "(null)");
                rb_free_ability(&ab);
            }
        }
        /* find a live card with a printed need */
        for (int i = 0; i < 400; i++) {
            Card c;
            if (!rb_decode_card_by_index((uint32_t)i, &c)) continue;
            int start = c.num_base + c.num_blade;
            if (c.num_need > 0) {
                printf("NEEDCARD %d %s need=%d base=%d blade=%d\n", i,
                       rb_card_string(c.card_no_idx), c.num_need, c.num_base, c.num_blade);
                rb_free_card(&c);
                if (i > 40) break;
            } else {
                rb_free_card(&c);
            }
        }
        return 0;
    }

    a_member_original_hearts_layers();
    a_apply_additive_heart_mods();
    a_effective_blade();
    a_member_heart_detail();
    a_stage_hearts_pipeline();
    a_effective_need_heart_and_satisfied();

    b_pb1_020_l_heart04_total_threshold();
    b_sd1_026_sd_energy_threshold();
    b_bp5_026_l_group_heart_total();
    b_bp4_028_l_distinct_waitroom_lives();
    b_bp6_022_l_opponent_energy_gate();
    b_bp4_026_l_revealed_distinct_liella();
    b_bp7_022_l_revealed_group_hearts();
    b_sd1_028_sd_total_blade_threshold();
    b_bp1_027_l_unique_heart_colors();
    b_pb2_045_l_four_heart_liella_member();

    rb_unload();
    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures) return 1;
    printf("ALL PARITY STATS PIPELINE CHECKS PASSED\n");
    return 0;
}
