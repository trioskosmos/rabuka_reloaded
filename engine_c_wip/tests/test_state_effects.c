#include "rabuka.h"
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

int main(void) {
    if (rb_load("src") != 0) return 1;
    GameState g;
    uint32_t d0[] = {0, 1, 2, 3};
    uint32_t d1[] = {4, 5, 6, 7};
    rb_seed(7);
    rb_game_init(&g, d0, 4, d1, 4);
    int energy[3] = {-1, -1, -1};
    int nenergy = 0;
    for (uint32_t i = 0; i < rb_num_cards() && nenergy < 3; i++) {
        if (rb_card_is_energy((int)i)) energy[nenergy++] = (int)i;
    }
    CHECK(nenergy == 3, "energy fixtures resolve");
    if (nenergy != 3) return 1;
    RbPlayer *p = &g.p[0];
    p->energy.n = 0;
    p->energy.cards[p->energy.n++] = energy[0];
    p->energy.cards[p->energy.n++] = energy[1];
    p->energy.cards[p->energy.n++] = energy[2];
    p->energy_active = 1;
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"energy_state_change";
    e.target = (char *)"self";
    e.count = 1;
    e.card_type_field[0] = '\0';
    snprintf(e.card_type_field, sizeof(e.card_type_field), "energy_card");
    e.extra_k[0] = (char *)"state_change";
    e.extra_v[0] = (char *)"wait";
    e.n_extra = 1;
    rb_effect_energy_state_change(&g, 0, &e);
    CHECK(p->energy_active == 0, "energy wait changes only one active card");
    CHECK(strcmp(rb_mods_get_orientation(&g.mods, energy[0]), "wait") == 0,
          "energy wait records the changed card orientation");
    e.extra_v[0] = (char *)"active";
    rb_effect_energy_state_change(&g, 0, &e);
    CHECK(p->energy_active == 1, "energy activation changes only one waited card");
    CHECK(strcmp(rb_mods_get_orientation(&g.mods, energy[1]), "active") == 0,
          "energy activation records the changed card orientation");
    rb_unload();
    if (failures) return 1;
    printf("ALL STATE EFFECT CHECKS PASSED\n");
    return 0;
}
