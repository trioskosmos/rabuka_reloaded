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

static int find_cards(int type, int wanted, int *out) {
    int n = 0;
    for (uint32_t i = 0; i < rb_num_cards() && n < wanted; i++) {
        if (type == 0 && rb_card_is_member((int)i)) out[n++] = (int)i;
        if (type == 1 && rb_card_is_energy((int)i)) out[n++] = (int)i;
    }
    return n;
}

static int has_text(const GameState *g, const char *text) {
    for (int i = 0; i < g->n_prohibition_effects; i++)
        if (!strcmp(g->prohibition_effects[i], text)) return 1;
    return 0;
}

int main(void) {
    if (rb_load("src") != 0) return 1;
    GameState g;
    uint32_t d0[] = {0, 1, 2, 3};
    uint32_t d1[] = {4, 5, 6, 7};
    rb_seed(7);
    rb_game_init(&g, d0, 4, d1, 4);
    int members[3];
    int energy[3];
    int nm = find_cards(0, 3, members);
    int ne = find_cards(1, 3, energy);
    CHECK(nm == 3, "member fixtures resolve");
    CHECK(ne == 3, "energy fixtures resolve");
    if (nm != 3 || ne != 3) return 1;
    RbPlayer *p = &g.p[0];
    for (int i = 0; i < 3; i++) p->stage[i] = members[i];
    g.activating_card = members[0];

    p->energy.n = 0;
    for (int i = 0; i < 3; i++) p->energy.cards[p->energy.n++] = energy[i];
    p->energy_active = 1;
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"energy_state_change";
    e.target = (char *)"self";
    e.count = 1;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "energy_card");
    e.extra_k[0] = (char *)"state_change";
    e.extra_v[0] = (char *)"wait";
    e.n_extra = 1;
    rb_effect_energy_state_change(&g, 0, &e);
    CHECK(p->energy_active == 0, "energy wait changes only one active card");
    CHECK(strcmp(rb_mods_get_orientation(&g.mods, energy[0]), "wait") == 0,
          "energy wait records card orientation");
    CHECK(g.state_change_from[energy[0]] == 0 && g.state_change_to[energy[0]] == 1,
          "energy wait records an active-to-wait transition");
    CHECK(g.n_recently_state_changed == 1, "energy wait feeds state-change tracking");
    e.extra_v[0] = (char *)"active";
    rb_effect_energy_state_change(&g, 0, &e);
    CHECK(p->energy_active == 1, "energy activation changes only one waited card");
    CHECK(strcmp(rb_mods_get_orientation(&g.mods, energy[1]), "active") == 0,
          "energy activation records card orientation");
    CHECK(g.state_change_from[energy[1]] == 1 && g.state_change_to[energy[1]] == 0,
          "energy activation records a wait-to-active transition");

    p->energy.n = 0;
    p->energy.cards[p->energy.n++] = energy[0];
    p->energy.cards[p->energy.n++] = energy[1];
    p->energy_active = 2;
    memset(g.mods.orientation, 0, sizeof(g.mods.orientation));
    memset(&e, 0, sizeof(e));
    e.action = (char *)"energy_state_change";
    e.target = (char *)"self";
    e.count = 1;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "energy_card");
    e.extra_k[0] = (char *)"state_change";
    e.extra_v[0] = (char *)"wait";
    e.n_extra = 1;
    rb_effect_energy_state_change(&g, 0, &e);
    CHECK(g.queue.has_pending && !strcmp(g.queue.pending.target, "energy_state_change"),
          "ambiguous energy deactivation prompts instead of auto-picking");
    CHECK(g.queue.pending.n_filtered_indices == 2,
          "energy state prompt exposes only state-matching cards");
    g.n_selected_cards = 0;
    rb_resume_with_choice(&g, 0);
    CHECK(p->energy_active == 1 && !strcmp(rb_mods_get_orientation(&g.mods, energy[0]), "wait"),
          "energy state prompt applies the selected card only");

    p->energy.n = 0;
    for (int i = 0; i < 3; i++) p->energy.cards[p->energy.n++] = energy[i];
    p->energy_active = 3;
    memset(g.mods.orientation, 0, sizeof(g.mods.orientation));
    g.n_selected_cards = 0;
    e.count = 2;
    rb_effect_energy_state_change(&g, 0, &e);
    CHECK(g.queue.pending.count == 2,
          "multi-card energy state change retains its requested count");
    rb_resume_with_choice(&g, 0);
    CHECK(g.queue.has_pending && g.queue.pending.count == 1,
          "energy state fixed-count selection reprompts for the remainder");
    rb_effect_free(g.queue.deferred);
    g.queue.deferred = NULL;
    g.queue.has_pending = 0;
    g.n_selected_cards = 0;
    g.selected_cards[g.n_selected_cards++] = energy[0];
    g.selected_cards[g.n_selected_cards++] = energy[1];
    g.state_change_triggering = 1;
    rb_effect_energy_state_change(&g, 0, &e);
    g.state_change_triggering = 0;
    CHECK(p->energy_active == 1 &&
          !strcmp(rb_mods_get_orientation(&g.mods, energy[0]), "wait") &&
          !strcmp(rb_mods_get_orientation(&g.mods, energy[1]), "wait") &&
          !strcmp(rb_mods_get_orientation(&g.mods, energy[2]), "active"),
          "multi-card energy state change applies only chosen cards");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_blade_type";
    e.target = (char *)"self";
    e.card_type_field[0] = '\0';
    e.extra_k[0] = (char *)"blade_type";
    e.extra_v[0] = (char *)"red";
    e.extra_k[1] = (char *)"duration";
    e.extra_v[1] = (char *)"live_end";
    e.n_extra = 2;
    rb_effect_set_blade_type(&g, 0, &e, members[0]);
    CHECK(rb_mods_get_blade_type(&g.mods, members[0]) == 1,
          "blade type recolors matching members");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(rb_mods_get_blade_type(&g.mods, members[0]) == 1,
          "live-scoped blade type survives turn-end expiry");
    rb_check_expired_effects(&g, RB_TEMP_LIVE_END);
    CHECK(rb_mods_get_blade_type(&g.mods, members[0]) == -1,
          "live-scoped blade type expires at live end");

    g.mods.blade[members[0]].set = 3;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_blade_count";
    e.target = (char *)"self";
    e.count = 1;
    e.extra_k[0] = (char *)"value";
    e.extra_v[0] = (char *)"5";
    e.extra_k[1] = (char *)"duration";
    e.extra_v[1] = (char *)"this_turn";
    e.n_extra = 2;
    rb_effect_set_blade_count(&g, 0, &e, members[0]);
    CHECK(g.mods.blade[members[0]].set == 5, "set blade count applies its exact value");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(g.mods.blade[members[0]].set == 3,
          "turn-scoped blade count restores the prior set value");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_heart_type";
    e.target = (char *)"self";
    e.self_target_field[0] = '\0';
    snprintf(e.self_target_field, sizeof(e.self_target_field), "true");
    e.extra_k[0] = (char *)"heart_type";
    e.extra_v[0] = (char *)"red";
    e.extra_k[1] = (char *)"duration";
    e.extra_v[1] = (char *)"this_turn";
    e.n_extra = 2;
    rb_effect_set_heart_type(&g, 0, &e, members[0]);
    CHECK(rb_mods_get_heart_color_multiplier(&g.mods, members[0]) == RB_HEART_RED,
          "set heart type transforms the selected member");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(rb_mods_get_heart_color_multiplier(&g.mods, members[0]) == -1,
          "turn-scoped heart type expires cleanly");

    p->under_cards[0].n = 1;
    p->under_cards[0].cards[0] = members[1];
    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_heart_type";
    e.target = (char *)"self";
    e.extra_k[0] = (char *)"ref_value";
    e.extra_v[0] = (char *)"placed_under";
    e.extra_k[1] = (char *)"duration";
    e.extra_v[1] = (char *)"this_turn";
    e.n_extra = 2;
    rb_effect_set_heart_type(&g, 0, &e, members[0]);
    CHECK(rb_mods_get_heart_copy(&g.mods, members[0]) == members[1],
          "heart type copies the most recently placed under-card");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(rb_mods_get_heart_copy(&g.mods, members[0]) == -1,
          "temporary heart copy expires cleanly");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"activation_cost";
    e.target = (char *)"self";
    e.extra_k[0] = (char *)"operation";
    e.extra_v[0] = (char *)"increase";
    e.extra_k[1] = (char *)"value";
    e.extra_v[1] = (char *)"2";
    e.extra_k[2] = (char *)"duration";
    e.extra_v[2] = (char *)"this_turn";
    e.n_extra = 3;
    rb_effect_activation_cost(&g, 0, &e, members[0]);
    CHECK(has_text(&g, "activation_cost_increase_2"),
          "activation cost records a prohibition effect");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(!has_text(&g, "activation_cost_increase_2"),
          "activation cost prohibition expires with its duration");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_card_identity";
    e.target = (char *)"self";
    e.extra_k[0] = (char *)"all_regions";
    e.extra_v[0] = (char *)"true";
    e.extra_k[1] = (char *)"identities";
    e.extra_v[1] = (char *)"ZZZ Identity";
    e.n_extra = 2;
    rb_effect_set_card_identity_all_regions(&g, 0, &e, members[0]);
    CHECK(g.n_prohibition_effects > 0 &&
          strstr(g.prohibition_effects[g.n_prohibition_effects - 1], "card_identity:") != NULL,
          "all-regions identity records a card-scoped prohibition");
    CHECK(!rb_card_matches_identity_str(members[0], "ZZZ Identity"),
          "identity bookkeeping does not fabricate a permanent identity override");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_cost";
    e.target = (char *)"self";
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    e.extra_k[0] = (char *)"operation";
    e.extra_v[0] = (char *)"set";
    e.extra_k[1] = (char *)"value";
    e.extra_v[1] = (char *)"6";
    e.extra_k[2] = (char *)"duration";
    e.extra_v[2] = (char *)"this_turn";
    e.n_extra = 3;
    rb_effect_modify_cost(&g, 0, &e, members[0]);
    CHECK(g.mods.cost[members[0]].set == 6, "modify cost applies set operation");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(g.mods.cost[members[0]].set == 0,
          "temporary cost set restores the prior set value");

    for (int i = 0; i < 3; i++) rb_mods_set_orientation(&g.mods, members[i], "active");
    Card selected_card;
    rb_decode_card_by_index((uint32_t)members[0], &selected_card);
    char exact_name[128];
    snprintf(exact_name, sizeof(exact_name), "%s", selected_card.name ? selected_card.name : "");
    rb_free_card(&selected_card);
    memset(&e, 0, sizeof(e));
    e.action = (char *)"change_state";
    e.target = (char *)"self";
    e.count = 1;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    e.extra_k[0] = (char *)"state_change";
    e.extra_v[0] = (char *)"wait";
    e.extra_k[1] = (char *)"card_names";
    e.extra_v[1] = exact_name;
    e.n_extra = 2;
    g.n_selected_cards = 0;
    rb_effect_change_state(&g, 0, &e, members[0]);
    CHECK(!strcmp(rb_mods_get_orientation(&g.mods, members[0]), "wait") &&
          !strcmp(rb_mods_get_orientation(&g.mods, members[1]), "active") &&
          !strcmp(rb_mods_get_orientation(&g.mods, members[2]), "active"),
          "member state change honors card-name target filtering");

    for (int i = 0; i < 3; i++) rb_mods_set_orientation(&g.mods, members[i], "active");
    char cost_values[] = "255";
    memset(&e, 0, sizeof(e));
    e.action = (char *)"change_state";
    e.target = (char *)"self";
    e.count = 0;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    e.extra_k[0] = (char *)"state_change";
    e.extra_v[0] = (char *)"wait";
    e.extra_k[1] = (char *)"cost_values";
    e.extra_v[1] = cost_values;
    e.n_extra = 2;
    g.n_selected_cards = 0;
    rb_effect_change_state(&g, 0, &e, members[0]);
    CHECK(!strcmp(rb_mods_get_orientation(&g.mods, members[0]), "active") &&
          !strcmp(rb_mods_get_orientation(&g.mods, members[1]), "active") &&
          !strcmp(rb_mods_get_orientation(&g.mods, members[2]), "active"),
          "cost-value filter excludes nonmatching members");

    rb_unload();
    if (failures) return 1;
    printf("ALL STATE EFFECT CHECKS PASSED\n");
    return 0;
}
