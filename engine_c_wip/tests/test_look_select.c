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

static const char *effect_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect) return NULL;
    for (int i = 0; i < effect->n_extra; i++)
        if (effect->extra_k[i] && !strcmp(effect->extra_k[i], key))
            return effect->extra_v[i];
    return NULL;
}

static void clear_player_cards(TestGame *tg, int pl)
{
    RbPlayer *player = &tg->state.p[pl];
    player->deck.n = 0;
    player->hand.n = 0;
    player->discard.n = 0;
    player->live.n = 0;
    player->success.n = 0;
}

static void test_decoded_compound_ownership(void)
{
    int card = rb_find_card_by_no("PL!HS-PR-002-PR");
    CHECK(card >= 0, "decoded look-and-select fixture resolves");
    if (card < 0) return;
    Ability ability;
    memset(&ability, 0, sizeof(ability));
    int decoded = rb_decode_card_ability((uint32_t)card, 0, &ability);
    CHECK(decoded, "look-and-select ability decodes");
    if (!decoded) return;
    AbilityEffect *effect = ability.effect;
    CHECK(effect && effect->action && !strcmp(effect->action, "look_and_select"),
          "decoded root is look_and_select");
    CHECK(effect && effect->look_action && effect->look_action->action &&
              !strcmp(effect->look_action->action, "look_at"),
          "look_action is retained as an owned compound field");
    CHECK(effect && effect->select_action && effect->select_action->action &&
              !strcmp(effect->select_action->action, "select_cards"),
          "select_action is retained as an owned compound field");
    CHECK(effect && effect->look_action && effect->look_action->count == 3,
          "decoded look_action preserves count");
    CHECK(effect && effect->select_action && effect->select_action->count == 1 &&
              effect->select_action->destination &&
              !strcmp(effect->select_action->destination, "hand"),
          "decoded select_action preserves destination and count");
    CHECK(effect && effect->n_child == 0,
          "structural branches are not flattened into generic children");
    AbilityEffect *clone = effect ? rb_effect_deep_clone(effect) : NULL;
    CHECK(clone && clone->look_action && clone->select_action,
          "deep clone retains both structural branches");
    CHECK(clone && clone->look_action && clone->look_action != effect->look_action &&
              clone->select_action && clone->select_action != effect->select_action,
          "deep clone owns independent structural branches");
    rb_effect_free(clone);
    rb_free_ability(&ability);
}

static void test_dynamic_look_count_decodes(void)
{
    int card = rb_find_card_by_no("PL!-bp5-001-P");
    int live = rb_find_card_by_no("PL!HS-bp1-019-L");
    CHECK(card >= 0 && live >= 0, "dynamic look-count fixtures resolve");
    if (card < 0 || live < 0) return;
    Ability ability;
    memset(&ability, 0, sizeof(ability));
    int decoded = rb_decode_card_ability((uint32_t)card, 0, &ability);
    CHECK(decoded && ability.effect && ability.effect->look_action,
          "dynamic look ability decodes");
    if (!decoded || !ability.effect || !ability.effect->look_action) {
        rb_free_ability(&ability);
        return;
    }
    AbilityEffect *look = ability.effect->look_action;
    const char *reference = effect_extra(look, "reference");
    CHECK(reference && !strcmp(reference, "total_live_score"),
          "dynamic_count reference is decoded into look_action");
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    test_add_to_live(&tg, live);
    Card live_card;
    memset(&live_card, 0, sizeof(live_card));
    CHECK(rb_decode_card_by_index((uint32_t)live, &live_card),
          "dynamic count live card decodes");
    int expected = (int)live_card.score + 2;
    rb_free_card(&live_card);
    CHECK_EQ(rb_effect_count(&tg.state, 0, live, look, 0), expected,
             "dynamic total_live_score overrides the placeholder count");
    rb_free_ability(&ability);
}

static void test_multi_select_and_followup(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    int cards[4];
    for (int i = 0; i < 4; i++) {
        cards[i] = test_new_id(&tg, "PL!-sd1-010-SD");
        if (cards[i] < 0) return;
        test_add_to_deck(&tg, cards[i]);
    }
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    look.target = "self";
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 2;
    select.extra_k[0] = "discard_remaining";
    select.extra_v[0] = "true";
    select.n_extra = 1;
    AbilityEffect followup = {0};
    followup.action = "modify_score";
    followup.count = 3;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    parent.followup_action = &followup;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(rb_has_pending_choice(&tg.state), "structural look-and-select emits one choice");
    const RbChoice *choice = rb_get_pending_choice(&tg.state);
    CHECK(choice && choice->kind == RB_CHOICE_SELECT_CARD && choice->count == 2,
          "select_action controls the offered count");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "look_action drains exactly its count from deck top");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 3, "looked-at pool retains looked count");
    CHECK(pool[0] == cards[0] && pool[1] == cards[1] && pool[2] == cards[2],
          "looked-at pool preserves deck-top order");
    int indices[2] = {0, 2};
    rb_resume_with_choice_indices(&tg.state, indices, 2);
    CHECK(tg.state.p[0].hand.n == 2 && tg.state.p[0].hand.cards[0] == cards[0] &&
              tg.state.p[0].hand.cards[1] == cards[2],
          "multi-selection moves selected cards in choice order");
    CHECK(tg.state.p[0].discard.n == 1 && tg.state.p[0].discard.cards[0] == cards[1],
          "discard_remaining sends only unselected cards to waitroom");
    CHECK(tg.state.p[0].deck.n == 1 && tg.state.p[0].deck.cards[0] == cards[3],
          "unrevealed deck tail remains untouched");
    CHECK(tg.state.n_selected_cards == 2 && tg.state.selected_cards[0] == cards[0] &&
              tg.state.selected_cards[1] == cards[2],
          "multi-selection records every chosen card");
    CHECK_EQ(tg.state.p[0].score, 3, "look-and-select followup runs after finalization");
    CHECK_EQ(tg.state.p[0].hand.n + tg.state.p[0].deck.n + tg.state.p[0].discard.n, 4,
             "multi-selection conserves every physical card");
}

static void test_type_filtered_selection(void)
{
    int member_template = rb_find_card_by_no("PL!-sd1-010-SD");
    int live_template = rb_find_card_by_no("PL!N-bp1-027-L");
    CHECK(member_template >= 0 && live_template >= 0, "type filter fixtures resolve");
    if (member_template < 0 || live_template < 0) return;
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck";
    look.count = 4;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    strcpy(select.card_type_field, "member_card");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    int member_a = rb_create_card_copy(member_template);
    int member_b = rb_create_card_copy(member_template);
    int live_a = rb_create_card_copy(live_template);
    int live_b = rb_create_card_copy(live_template);
    if (member_a < 0 || member_b < 0 || live_a < 0 || live_b < 0) return;
    test_add_to_deck(&tg, live_a);
    test_add_to_deck(&tg, member_a);
    test_add_to_deck(&tg, live_b);
    test_add_to_deck(&tg, member_b);
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    const RbChoice *choice = rb_get_pending_choice(&tg.state);
    CHECK(choice && choice->n_filtered_indices == 2,
          "type filter exposes only matching looked-at indices");
    CHECK(choice && choice->filtered_indices[0] == 1 && choice->filtered_indices[1] == 3,
          "filtered indices retain original looked-at positions");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(tg.state.p[0].hand.n == 1 && tg.state.p[0].hand.cards[0] == member_a,
          "filtered choice index maps to the matching pool card");
    CHECK(tg.state.p[0].discard.n == 3,
          "filtered look-and-select discards the complete remainder");
}

static void test_no_match_runs_followup(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    int first = test_new_id(&tg, "PL!-sd1-010-SD");
    int second = test_new_id(&tg, "PL!-sd1-002-SD");
    if (first < 0 || second < 0) return;
    test_add_to_deck(&tg, first);
    test_add_to_deck(&tg, second);
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck";
    look.count = 2;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    strcpy(select.card_type_field, "live_card");
    AbilityEffect followup = {0};
    followup.action = "modify_score";
    followup.count = 4;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    parent.followup_action = &followup;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(!rb_has_pending_choice(&tg.state), "no matching looked-at card emits no impossible choice");
    CHECK(tg.state.p[0].hand.n == 0 && tg.state.p[0].deck.n == 0 &&
              tg.state.p[0].discard.n == 2,
          "no-match path discards the looked-at pool");
    CHECK_EQ(tg.state.p[0].score, 4, "no-match path executes the followup action");
}

static void test_refresh_and_optional_skip(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    int cards[3];
    cards[0] = test_new_id(&tg, "PL!-sd1-010-SD");
    cards[1] = test_new_id(&tg, "PL!-sd1-002-SD");
    cards[2] = test_new_id(&tg, "PL!-sd1-003-SD");
    if (cards[0] < 0 || cards[1] < 0 || cards[2] < 0) return;
    test_add_to_deck(&tg, cards[0]);
    test_add_to_discard(&tg, cards[1]);
    test_add_to_discard(&tg, cards[2]);
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    select.is_optional = 1;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(rb_has_pending_choice(&tg.state), "short look refresh still offers selection");
    CHECK(tg.state.p[0].deck.n == 0 && tg.state.p[0].discard.n == 0,
          "short look refreshes waitroom before completing the count");
    rb_resume_with_choice(&tg.state, -1);
    CHECK(tg.state.p[0].deck.n == 3 && tg.state.p[0].discard.n == 0 &&
              tg.state.p[0].hand.n == 0,
          "optional skip without remainder directive restores looked-at origin");
}

static void test_opponent_pool_ownership(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    clear_player_cards(&tg, 1);
    int card = test_new_id(&tg, "PL!-sd1-010-SD");
    if (card < 0) return;
    test_add_to_deck_pl(&tg, 1, card);
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck";
    look.count = 1;
    look.target = "opponent";
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(tg.state.queue.resume_look_owner == 1,
          "opponent look records the pool owner across the choice pause");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(tg.state.p[1].hand.n == 1 && tg.state.p[1].hand.cards[0] == card &&
              tg.state.p[0].hand.n == 0,
          "opponent looked-at selection resolves into the opponent hand");
}

static void test_deck_bottom_order(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    int cards[3];
    cards[0] = test_new_id(&tg, "PL!-sd1-010-SD");
    cards[1] = test_new_id(&tg, "PL!-sd1-002-SD");
    cards[2] = test_new_id(&tg, "PL!-sd1-003-SD");
    if (cards[0] < 0 || cards[1] < 0 || cards[2] < 0) return;
    test_add_to_deck(&tg, cards[0]);
    test_add_to_deck(&tg, cards[1]);
    test_add_to_deck(&tg, cards[2]);
    AbilityEffect effect = {0};
    effect.action = "look_at";
    effect.source = "deck_bottom";
    effect.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &effect, -1);
    int pool[8];
    int n = rb_looked_at_pool(0, pool, 8);
    CHECK_EQ(n, 2, "deck-bottom look takes two cards");
    CHECK(n == 2 && pool[0] == cards[1] && pool[1] == cards[2],
          "deck-bottom look preserves bottom-up order");
    CHECK(tg.state.p[0].deck.n == 1 && tg.state.p[0].deck.cards[0] == cards[0],
          "deck-bottom look leaves the deck top in place");
    rb_resume_with_choice(&tg.state, -1);
}

static void test_reveal_order_and_until_refresh(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    int first = test_new_id(&tg, "PL!-sd1-010-SD");
    int second = test_new_id(&tg, "PL!-sd1-002-SD");
    int third = test_new_id(&tg, "PL!-sd1-003-SD");
    int live = test_new_id(&tg, "PL!N-bp1-027-L");
    if (first < 0 || second < 0 || third < 0 || live < 0) return;
    test_add_to_deck(&tg, first);
    test_add_to_deck(&tg, second);
    test_add_to_deck(&tg, third);
    AbilityEffect reveal = {0};
    reveal.action = "reveal_per_group";
    reveal.source = "deck";
    reveal.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &reveal, -1);
    CHECK(tg.state.n_revealed == 2 && tg.state.revealed_cards[0] == first &&
              tg.state.revealed_cards[1] == second,
          "reveal_per_group peeks from deck index zero");
    CHECK(tg.state.p[0].deck.n == 3,
          "reveal_per_group does not drain the deck");
    tg.state.n_revealed = 0;
    reveal.action = "reveal";
    rb_execute_effect_ex(&tg.state, 0, &reveal, -1);
    CHECK(tg.state.n_revealed == 2 && tg.state.revealed_cards[0] == first &&
              tg.state.revealed_cards[1] == second,
          "reveal peeks from deck index zero");
    tg.state.p[0].deck.n = 0;
    test_add_to_deck(&tg, first);
    test_add_to_discard(&tg, live);
    tg.state.n_revealed = 0;
    reveal.action = "reveal_until_live_card";
    rb_execute_effect_ex(&tg.state, 0, &reveal, -1);
    int pool[8];
    int n = rb_looked_at_pool(0, pool, 8);
    CHECK(n == 2 && pool[0] == first && pool[1] == live,
          "reveal_until refreshes and retains every revealed card");
    CHECK(tg.state.p[0].deck.n == 0 && tg.state.p[0].discard.n == 0,
          "reveal_until refresh consumes the previous waitroom");
    CHECK(tg.state.n_revealed == 2,
          "reveal_until records each revealed card");
    CHECK(!rb_has_pending_choice(&tg.state),
          "reveal_until only populates looked-at for its structural parent");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_decoded_compound_ownership();
    test_dynamic_look_count_decodes();
    test_multi_select_and_followup();
    test_type_filtered_selection();
    test_no_match_runs_followup();
    test_refresh_and_optional_skip();
    test_opponent_pool_ownership();
    test_deck_bottom_order();
    test_reveal_order_and_until_refresh();
    rb_unload();
    if (failures) return 1;
    printf("ALL LOOK SELECT CHECKS PASSED\n");
    return 0;
}
