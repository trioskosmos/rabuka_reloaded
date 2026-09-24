#ifndef RABUKA_DECK_BUILDER_H
#define RABUKA_DECK_BUILDER_H

#include "rabuka.h"
#include "deck_parser.h"

typedef struct {
    int main_cards[RB_MAX_DECK];
    int main_count;
    int energy_cards[RB_MAX_ENERGY_CARDS];
    int energy_count;
} RbBuiltDeck;

enum {
    RB_DECK_BUILD_OK = 0,
    RB_DECK_BUILD_EINVAL = -1,
    RB_DECK_BUILD_EFORMAT = -2,
    RB_DECK_BUILD_EOVERFLOW = -3,
    RB_DECK_BUILD_ENOMEM = -4,
    RB_DECK_BUILD_EUNKNOWN = -5
};

int rb_build_deck_from_card_ids(const int *main_ids, size_t main_count,
                                const int *energy_ids, size_t energy_count,
                                RbBuiltDeck *out);
int rb_build_deck_from_card_numbers(const char *const *main_nos, size_t main_count,
                                    const char *const *energy_nos, size_t energy_count,
                                    RbBuiltDeck *out);
int rb_build_deck_from_list(const RbDeckList *deck, RbBuiltDeck *out);
int rb_init_game_from_built_decks(GameState *g, const RbBuiltDeck *deck0,
                                  const RbBuiltDeck *deck1);
void rb_built_deck_clear(RbBuiltDeck *deck);

#endif
