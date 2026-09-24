#include "deck_builder.h"
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

static int same_template(int copy_id, int template_id)
{
    Card copy_card;
    Card template_card;
    memset(&copy_card, 0, sizeof(copy_card));
    memset(&template_card, 0, sizeof(template_card));
    int same = 0;
    if (rb_decode_card_by_index((uint32_t)copy_id, &copy_card) &&
        rb_decode_card_by_index((uint32_t)template_id, &template_card)) {
        same = !strcmp(rb_card_string(copy_card.card_no_idx),
                       rb_card_string(template_card.card_no_idx));
    }
    rb_free_card(&copy_card);
    rb_free_card(&template_card);
    return same;
}

static void test_builds_unique_copies_and_fills_energy(void)
{
    int member = rb_find_card_by_no("PL!-sd1-010-SD");
    int live = rb_find_card_by_no("PL!HS-bp1-019-L");
    int energy = rb_find_card_by_no("LL-E-001-SD");
    int ids[3] = {member, energy, live};
    RbBuiltDeck deck;
    CHECK_EQ(rb_build_deck_from_card_ids(ids, 3, NULL, 0, &deck), 0,
             "deck builder accepts mixed main input");
    CHECK_EQ(deck.main_count, 2, "energy cards are removed from main deck");
    CHECK_EQ(deck.energy_count, RB_MAX_ENERGY_CARDS, "energy deck is filled to capacity");
    CHECK(deck.main_cards[0] != member && deck.main_cards[1] != live,
          "main deck entries receive copy ids");
    CHECK(same_template(deck.main_cards[0], member), "member copy resolves to member card");
    CHECK(same_template(deck.main_cards[1], live), "live copy resolves to live card");
    for (int i = 0; i < deck.energy_count; i++)
        CHECK(rb_card_is_energy(deck.energy_cards[i]), "every energy entry is classified as energy");
    CHECK(deck.main_cards[0] != deck.main_cards[1], "duplicate physical cards use distinct copies");
    RbMods mods;
    rb_mods_init(&mods);
    rb_mods_add_score(&mods, deck.main_cards[0], 3);
    CHECK_EQ(rb_mods_get_score(&mods, deck.main_cards[0]), 3, "copy modifier applies to selected copy");
    CHECK_EQ(rb_mods_get_score(&mods, deck.main_cards[1]), 0, "copy modifier does not leak to sibling");
}

static void test_rejects_invalid_and_non_energy_inputs(void)
{
    int member = rb_find_card_by_no("PL!-sd1-010-SD");
    int energy = rb_find_card_by_no("LL-E-001-SD");
    RbBuiltDeck deck;
    memset(&deck, 0xA5, sizeof(deck));
    int ids[61];
    for (int i = 0; i < 61; i++) ids[i] = member;
    CHECK_EQ(rb_build_deck_from_card_ids(ids, 61, NULL, 0, &deck),
             RB_DECK_BUILD_EOVERFLOW, "main deck overflow is rejected");
    CHECK_EQ(deck.main_count, 0, "overflow leaves output empty");
    int invalid_energy[1] = {member};
    CHECK_EQ(rb_build_deck_from_card_ids(NULL, 0, invalid_energy, 1, &deck),
             RB_DECK_BUILD_EFORMAT, "non-energy energy input is rejected");
    CHECK_EQ(deck.energy_count, 0, "invalid energy input leaves output empty");
    int unknown = -7;
    int unknown_ids[1] = {unknown};
    CHECK_EQ(rb_build_deck_from_card_ids(unknown_ids, 1, NULL, 0, &deck),
             RB_DECK_BUILD_EUNKNOWN, "unknown card id is rejected");
    int energy_ids[1] = {energy};
    CHECK_EQ(rb_build_deck_from_card_ids(NULL, 0, energy_ids, 1, &deck), 0,
             "energy-only input uses default main-empty deck");
    CHECK_EQ(deck.energy_count, RB_MAX_ENERGY_CARDS, "energy-only deck still fills energy capacity");
}

static void test_builds_from_deck_list_and_initializes_game(void)
{
    char member_no[] = "PL!-sd1-010-SD";
    char energy_no[] = "LL-E-001-SD";
    RbDeckEntry entries[2] = {
        {member_no, 2},
        {energy_no, 1}
    };
    RbDeckList list;
    memset(&list, 0, sizeof(list));
    list.name = (char *)"builder fixture";
    list.entries = entries;
    list.count = 2;
    RbBuiltDeck deck0;
    RbBuiltDeck deck1;
    CHECK_EQ(rb_build_deck_from_list(&list, &deck0), 0, "deck list expands quantities");
    CHECK_EQ(deck0.main_count, 2, "deck list quantity creates two main cards");
    CHECK_EQ(deck0.energy_count, RB_MAX_ENERGY_CARDS, "deck list energy is filled");
    CHECK_EQ(rb_build_deck_from_list(&list, &deck1), 0, "second deck list builds independently");
    GameState game;
    CHECK_EQ(rb_init_game_from_built_decks(&game, &deck0, &deck1), 0,
             "built decks initialize a game");
    CHECK_EQ(game.p[0].energy_deck.n, RB_MAX_ENERGY_CARDS - 3,
             "opening energy draw leaves nine energy cards");
    CHECK_EQ(game.p[0].energy.n, 3, "opening setup places three energy cards");
    CHECK_EQ(game.p[0].energy_active, 3, "opening energy cards are active");
}

static void test_builds_from_card_numbers(void)
{
    const char *main_nos[2] = {"PL!-sd1-010-SD", "PL!HS-bp1-019-L"};
    const char *energy_nos[1] = {"LL-E-001-SD"};
    RbBuiltDeck deck;
    CHECK_EQ(rb_build_deck_from_card_numbers(main_nos, 2, energy_nos, 1, &deck), 0,
             "deck builder resolves card numbers");
    CHECK_EQ(deck.main_count, 2, "resolved main card count is preserved");
    CHECK_EQ(deck.energy_count, RB_MAX_ENERGY_CARDS, "resolved energy count is filled");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    rb_card_copy_pool_reset();
    test_builds_unique_copies_and_fills_energy();
    test_rejects_invalid_and_non_energy_inputs();
    test_builds_from_deck_list_and_initializes_game();
    test_builds_from_card_numbers();
    rb_unload();
    if (failures) return 1;
    printf("ALL DECK BUILDER CHECKS PASSED\n");
    return 0;
}
