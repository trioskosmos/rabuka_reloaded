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

static void generated_card_number_normalization(void)
{
    int canonical = rb_find_card_by_no("PL!N-bp1-027-L");
    int normalized = rb_find_card_by_no("pl!n-bp1-027-l");
    char normalized_text[64];
    char equivalent[16];
    CHECK(canonical >= 0, "canonical card number resolves");
    CHECK(normalized >= 0, "normalized card number resolves");
    rb_card_normalize_no("ａｂ！－＊＃＋", normalized_text, sizeof(normalized_text));
    CHECK(!strcmp(normalized_text, "AB!-*#+"), "fullwidth card number normalization");
    CHECK(rb_card_equivalent_rarity("PR+", equivalent, sizeof(equivalent)), "promo rarity equivalence");
    CHECK(!strcmp(equivalent, "PR＋"), "promo rarity canonical form");
    CHECK(rb_card_get_card_id("pl!n-bp1-027-X") == canonical, "rarity fallback resolves base card");
}

static void generated_card_helpers(void)
{
    AbilityEffect effect = {0};
    RbCardFilter filter;
    effect.count = 3;
    effect.target = NULL;
    effect.source = NULL;
    effect.card_type_field[0] = 0;
    effect.extra_k[0] = (char *)"card_type";
    effect.extra_v[0] = (char *)"member_card";
    effect.extra_k[1] = (char *)"group_names";
    effect.extra_v[1] = (char *)"Aqours";
    effect.extra_k[2] = (char *)"cost_limit";
    effect.extra_v[2] = (char *)"5";
    effect.extra_k[3] = (char *)"cost_limit_operator";
    effect.extra_v[3] = (char *)"<=";
    effect.extra_k[4] = (char *)"exclude_self";
    effect.extra_v[4] = (char *)"true";
    effect.extra_k[5] = (char *)"card_names";
    effect.extra_v[5] = (char *)"[Aqours,μ's]";
    effect.extra_k[6] = (char *)"cost_values";
    effect.extra_v[6] = (char *)"[10,20]";
    effect.extra_k[7] = (char *)"distinct";
    effect.extra_v[7] = (char *)"card_name";
    effect.extra_k[8] = (char *)"blade_limit";
    effect.extra_v[8] = (char *)"4";
    effect.extra_k[9] = (char *)"blade_limit_operator";
    effect.extra_v[9] = (char *)">=";
    effect.n_extra = 10;
    CHECK(rb_card_filter_subset(&effect, &filter), "effect filter subset builds");
    CHECK(!strcmp(filter.card_type, "member_card"), "filter subset keeps card type");
    CHECK(filter.has_group && !strcmp(filter.group, "Aqours"), "filter subset keeps first group");
    CHECK(filter.has_cost_limit && filter.cost_limit == 5 && !strcmp(filter.cost_op, "<="), "filter subset keeps cost comparison");
    CHECK(filter.has_exclude_self && filter.exclude_self_id == -1, "filter subset keeps self exclusion");
    CHECK_EQ(filter.n_name_fragments, 2, "filter subset keeps card name fragments");
    CHECK(!strcmp(filter.name_fragments[0], "Aqours") && !strcmp(filter.name_fragments[1], "μ's"), "filter subset keeps fragment values");
    CHECK_EQ(filter.n_cost_values, 2, "filter subset keeps discrete cost values");
    CHECK(filter.cost_values[0] == 10 && filter.cost_values[1] == 20, "filter subset cost values match Rust OR semantics");
    CHECK_EQ(filter.distinct, 1, "filter subset keeps distinct mode");
    CHECK(filter.has_blade_limit && filter.blade_limit == 4 && !strcmp(filter.blade_op, ">="), "filter subset keeps blade count alternative");
    CHECK_EQ(rb_effect_count_or(&effect, 9), 3, "effect count uses explicit count");
    CHECK_EQ(rb_effect_count_or(NULL, 9), 9, "effect count uses default");
    CHECK_EQ(rb_effect_value_or_count(&effect, 9), 3, "effect value falls back to count");
    CHECK_EQ(rb_effect_value_or_count(NULL, 9), 9, "effect value uses default");
    CHECK(!strcmp(rb_effect_target_name(&effect), "self"), "effect target default");
    CHECK(!strcmp(rb_effect_source_or(&effect, "hand"), "hand"), "effect source default");
}

static void generated_card_property_heart_helpers(void)
{
    Card card = {0};
    card.num_base = 0;
    card.num_blade = 1;
    card.n_hearts = 1;
    card.heart_color[0] = RB_HEART_ALL;
    card.heart_count[0] = 1;
    CHECK(rb_card_has_blade_heart(&card), "card property accepts blade hearts");
    CHECK(rb_card_has_blade_heart_strict(&card), "strict blade-heart property accepts blade hearts");
    CHECK(rb_card_has_all_blade(&card), "card property detects all-blade heart");
    CHECK(!rb_card_has_score_icon(&card), "plain blade heart is not score icon");

    card.num_blade = 0;
    card.has_special = 1;
    card.special_count = 1;
    card.special_color = RB_HEART_SCORE;
    CHECK(rb_card_has_blade_heart(&card), "legacy blade-heart property accepts special heart");
    CHECK(!rb_card_has_blade_heart_strict(&card), "strict blade-heart property rejects special heart");
    CHECK(rb_card_has_score_icon(&card), "card property detects score icon");

    int need_empty[8] = {0};
    int provided_empty[8] = {0};
    int need_red[8] = {0, 1};
    int provided_colorless[8] = {1};
    int provided_all[8] = {0, 0, 0, 0, 0, 0, 0, 1};
    int provided_red[8] = {0, 1};
    CHECK(rb_check_heart_requirement(NULL, provided_empty) == 0, "heart check rejects missing requirement");
    CHECK(rb_check_heart_requirement(need_empty, provided_empty), "empty heart requirement is satisfied");
    CHECK(rb_check_heart_requirement(need_red, provided_red), "matching heart satisfies requirement");
    CHECK(rb_check_heart_requirement(need_red, provided_colorless), "heartless wildcard follows Rust allocation rule");
    CHECK(rb_check_heart_requirement(need_red, provided_all), "all-heart wildcard satisfies requirement");
    CHECK(!rb_check_heart_requirement(need_red, need_empty), "unfulfilled heart requirement is rejected");
}

static void generated_effect_convenience_getters(void)
{
    AbilityEffect effect = {0};
    AbilityEffect option = {0};
    const AbilityEffect *options[1];
    effect.destination = (char *)"stage";
    effect.n_options = 1;
    effect.options[0] = &option;
    effect.repeat_limit = 3;
    effect.n_extra = 7;
    effect.extra_k[0] = (char *)"exclude_heart_colors";
    effect.extra_v[0] = (char *)"heart01,heart02";
    effect.extra_k[1] = (char *)"heart_colors";
    effect.extra_v[1] = (char *)"heart03,heart04";
    effect.extra_k[2] = (char *)"per_unit_heart_colors";
    effect.extra_v[2] = (char *)"heart05";
    effect.extra_k[3] = (char *)"under_self";
    effect.extra_v[3] = (char *)"true";
    effect.extra_k[4] = (char *)"action_by";
    effect.extra_v[4] = (char *)"opponent";
    effect.extra_k[5] = (char *)"picker";
    effect.extra_v[5] = (char *)"self";
    effect.extra_k[6] = (char *)"non_stackable";
    effect.extra_v[6] = (char *)"true";

    CHECK(!strcmp(rb_effect_exclude_heart_colors_any(&effect), "heart01,heart02"), "exclude-heart getter");
    CHECK(!strcmp(rb_effect_heart_colors_any(&effect), "heart03,heart04"), "heart-color getter");
    CHECK(!strcmp(rb_effect_per_unit_heart_colors_any(&effect), "heart05"), "per-unit heart-color getter");
    CHECK_EQ(rb_effect_options_any(&effect, options, 1), 1, "options getter returns option count");
    CHECK(options[0] == &option, "options getter returns option pointer");
    CHECK_EQ(rb_effect_repeat_limit_any(&effect), 3, "repeat-limit getter");
    CHECK(!strcmp(rb_effect_destination_any(&effect), "stage"), "destination getter");
    CHECK(rb_effect_is_under_self(&effect), "under-self convenience getter");
    CHECK(!strcmp(rb_effect_action_by_any(&effect), "opponent"), "action-by-any getter");
    CHECK(!strcmp(rb_effect_action_by(&effect), "opponent"), "action-by getter");
    CHECK(!strcmp(rb_effect_picker_any(&effect), "self"), "picker getter");
    int non_stackable = 0;
    CHECK(rb_effect_non_stackable_any(&effect, &non_stackable) && non_stackable, "non-stackable getter");
    effect.n_extra = 0;
    effect.destination = NULL;
    CHECK(rb_effect_destination_any(&effect) == NULL, "destination getter reports missing field");
    CHECK_EQ(rb_effect_repeat_limit_any(&effect), 3, "repeat-limit getter keeps decoded field");
    CHECK_EQ(rb_effect_repeat_limit_any(NULL), -1, "repeat-limit getter reports missing effect");
}

static void generated_card_database_helpers(void)
{
    int template_id = rb_find_card_by_no("PL!N-bp1-027-L");
    Card template = {0};
    Card copy = {0};
    int copy_id = rb_card_database_create_copy(template_id);
    CHECK(template_id >= 0 && rb_card_get_card_by_id(template_id, &template), "database get-by-id");
    CHECK(copy_id > template_id && rb_card_get_card_by_id(copy_id, &copy), "database create-copy");
    CHECK(copy_id != template_id && !strcmp(template.name, copy.name), "database copy preserves card identity");
    rb_free_card(&template);
    rb_free_card(&copy);

    Card by_no = {0};
    CHECK(rb_card_get_card_by_no("pl!n-bp1-027-l", &by_no), "database get-by-number");
    if (by_no.name) rb_free_card(&by_no);

    int names_id = -1;
    char names[256] = {0};
    for (uint32_t i = 0; i < rb_num_cards() && names_id < 0; i++) {
        Card candidate = {0};
        if (!rb_decode_card_by_index(i, &candidate)) continue;
        if (candidate.name && (strchr(candidate.name, '&') || strstr(candidate.name, "＆"))) {
            names_id = (int)i;
            int n = rb_card_get_card_names((int)i, names, sizeof(names));
            CHECK(n >= 2, "database multi-name helper returns names");
        }
        rb_free_card(&candidate);
    }
    CHECK(names_id >= 0, "database contains a multi-name card");
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
    generated_card_number_normalization();
    generated_card_helpers();
    generated_card_property_heart_helpers();
    generated_effect_convenience_getters();
    generated_card_database_helpers();
    rb_unload();
    if (failures)
        return 1;
    printf("ALL GENERATED CHECKS PASSED\n");
    printf("generated: card helper checks\n");
    return 0;
}
