#include "deck_builder.h"
#include <stdlib.h>
#include <string.h>

static int rb_valid_template_id(int card_id)
{
    return card_id >= 0 && rb_card_record((uint32_t)card_id) != NULL;
}

static int rb_default_energy_template(void)
{
    int card_id = rb_find_card_by_no("LL-E-001-SD");
    if (card_id >= 0) return card_id;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        if (rb_card_is_energy((int)i)) return (int)i;
    }
    return -1;
}

int rb_build_deck_from_card_ids(const int *main_ids, size_t main_count,
                                const int *energy_ids, size_t energy_count,
                                RbBuiltDeck *out)
{
    if (!out) return RB_DECK_BUILD_EINVAL;
    memset(out, 0, sizeof(*out));
    if ((main_count && !main_ids) || (energy_count && !energy_ids))
        return RB_DECK_BUILD_EINVAL;
    if (main_count > SIZE_MAX / sizeof(int) || energy_count > SIZE_MAX / sizeof(int))
        return RB_DECK_BUILD_EOVERFLOW;

    int main_templates[RB_MAX_DECK];
    int energy_templates[RB_MAX_ENERGY_CARDS];
    int main_n = 0;
    int energy_n = 0;
    for (size_t i = 0; i < main_count; i++) {
        int card_id = main_ids[i];
        if (!rb_valid_template_id(card_id)) return RB_DECK_BUILD_EUNKNOWN;
        if (rb_card_is_energy(card_id)) {
            if (energy_n >= RB_MAX_ENERGY_CARDS) return RB_DECK_BUILD_EOVERFLOW;
            energy_templates[energy_n++] = card_id;
        } else {
            if (main_n >= RB_MAX_DECK) return RB_DECK_BUILD_EOVERFLOW;
            main_templates[main_n++] = card_id;
        }
    }
    for (size_t i = 0; i < energy_count; i++) {
        int card_id = energy_ids[i];
        if (!rb_valid_template_id(card_id)) return RB_DECK_BUILD_EUNKNOWN;
        if (!rb_card_is_energy(card_id)) return RB_DECK_BUILD_EFORMAT;
        if (energy_n >= RB_MAX_ENERGY_CARDS) return RB_DECK_BUILD_EOVERFLOW;
        energy_templates[energy_n++] = card_id;
    }

    if (energy_n == 0) {
        int default_energy = rb_default_energy_template();
        if (default_energy < 0) return RB_DECK_BUILD_EFORMAT;
        energy_templates[energy_n++] = default_energy;
    }
    while (energy_n < RB_MAX_ENERGY_CARDS)
        energy_templates[energy_n++] = energy_templates[0];

    for (int i = 0; i < main_n; i++) {
        int copy_id = rb_create_card_copy(main_templates[i]);
        if (copy_id < 0) {
            rb_built_deck_clear(out);
            return RB_DECK_BUILD_ENOMEM;
        }
        out->main_cards[out->main_count++] = copy_id;
    }
    for (int i = 0; i < energy_n; i++) {
        int copy_id = rb_create_card_copy(energy_templates[i]);
        if (copy_id < 0) {
            rb_built_deck_clear(out);
            return RB_DECK_BUILD_ENOMEM;
        }
        out->energy_cards[out->energy_count++] = copy_id;
    }
    return RB_DECK_BUILD_OK;
}

int rb_build_deck_from_card_numbers(const char *const *main_nos, size_t main_count,
                                    const char *const *energy_nos, size_t energy_count,
                                    RbBuiltDeck *out)
{
    if (!out) return RB_DECK_BUILD_EINVAL;
    memset(out, 0, sizeof(*out));
    if ((main_count && !main_nos) || (energy_count && !energy_nos))
        return RB_DECK_BUILD_EINVAL;
    if (main_count > SIZE_MAX / sizeof(int) || energy_count > SIZE_MAX / sizeof(int))
        return RB_DECK_BUILD_EOVERFLOW;
    int *main_ids = main_count ? (int *)malloc(main_count * sizeof(int)) : NULL;
    int *energy_ids = energy_count ? (int *)malloc(energy_count * sizeof(int)) : NULL;
    if ((main_count && !main_ids) || (energy_count && !energy_ids)) {
        free(main_ids);
        free(energy_ids);
        return RB_DECK_BUILD_ENOMEM;
    }
    for (size_t i = 0; i < main_count; i++) {
        main_ids[i] = rb_card_get_card_id(main_nos[i]);
        if (main_ids[i] < 0) {
            free(main_ids);
            free(energy_ids);
            return RB_DECK_BUILD_EUNKNOWN;
        }
    }
    for (size_t i = 0; i < energy_count; i++) {
        energy_ids[i] = rb_card_get_card_id(energy_nos[i]);
        if (energy_ids[i] < 0) {
            free(main_ids);
            free(energy_ids);
            return RB_DECK_BUILD_EUNKNOWN;
        }
    }
    int result = rb_build_deck_from_card_ids(main_ids, main_count,
                                             energy_ids, energy_count, out);
    free(main_ids);
    free(energy_ids);
    return result;
}

int rb_build_deck_from_list(const RbDeckList *deck, RbBuiltDeck *out)
{
    if (!deck || !out) return RB_DECK_BUILD_EINVAL;
    size_t total = 0;
    for (size_t i = 0; i < deck->count; i++) {
        if (deck->entries[i].quantity == 0) return RB_DECK_BUILD_EFORMAT;
        if (total > SIZE_MAX - deck->entries[i].quantity)
            return RB_DECK_BUILD_EOVERFLOW;
        total += deck->entries[i].quantity;
    }
    if (total == 0) return RB_DECK_BUILD_EFORMAT;
    int *ids = (int *)malloc(total * sizeof(int));
    if (!ids) return RB_DECK_BUILD_ENOMEM;
    size_t n = 0;
    for (size_t i = 0; i < deck->count; i++) {
        int template_id = rb_card_get_card_id(deck->entries[i].card_no);
        if (template_id < 0) {
            free(ids);
            return RB_DECK_BUILD_EUNKNOWN;
        }
        for (uint8_t copy = 0; copy < deck->entries[i].quantity; copy++)
            ids[n++] = template_id;
    }
    int result = rb_build_deck_from_card_ids(ids, n, NULL, 0, out);
    free(ids);
    return result;
}

int rb_init_game_from_built_decks(GameState *g, const RbBuiltDeck *deck0,
                                  const RbBuiltDeck *deck1)
{
    if (!g || !deck0 || !deck1) return -1;
    if (deck0->main_count < 0 || deck0->main_count > RB_MAX_DECK ||
        deck1->main_count < 0 || deck1->main_count > RB_MAX_DECK ||
        deck0->energy_count <= 0 || deck0->energy_count > RB_MAX_ENERGY_CARDS ||
        deck1->energy_count <= 0 || deck1->energy_count > RB_MAX_ENERGY_CARDS)
        return -1;
    int result = rb_game_init(g,
                              (const uint32_t *)deck0->main_cards, deck0->main_count,
                              (const uint32_t *)deck1->main_cards, deck1->main_count);
    if (result != 0) return result;
    rb_player_set_energy_deck(g, 0, deck0->energy_cards, deck0->energy_count);
    rb_player_set_energy_deck(g, 1, deck1->energy_cards, deck1->energy_count);
    rb_setup_initial_energy(g);
    return 0;
}

void rb_built_deck_clear(RbBuiltDeck *deck)
{
    if (deck) memset(deck, 0, sizeof(*deck));
}
