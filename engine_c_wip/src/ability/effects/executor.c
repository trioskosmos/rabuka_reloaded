#include "rabuka.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char *action;
    int structural;
} ExecutorEntry;

static const ExecutorEntry executor_table[] = {
    {"draw_card", 0}, {"draw", 0}, {"draw_until_count", 0},
    {"move_cards", 0}, {"discard_card", 0}, {"gain_resource", 0},
    {"change_state", 0}, {"modify_score", 0}, {"gain_score", 0},
    {"modify_required_hearts", 0}, {"set_cost", 0}, {"set_cost_to_use", 0},
    {"set_blade_type", 0}, {"set_blade_count", 0}, {"set_heart_type", 0},
    {"activate_ability", 0}, {"invalidate_ability", 0},
    {"suppress_ability_trigger", 0}, {"gain_ability", 0},
    {"gain_ability_from_source", 0}, {"play_baton_touch", 0},
    {"reveal", 0}, {"reveal_per_group", 0}, {"select", 0},
    {"select_number", 0}, {"look_at", 0}, {"select_cards", 0},
    {"modify_required_hearts_global", 0}, {"modify_yell_count", 0},
    {"place_energy_under_member", 0}, {"activation_cost", 0},
    {"energy_placement", 0}, {"energy_state_change", 0},
    {"position_change", 0}, {"rotation", 0}, {"choice", 0},
    {"pay_energy", 0}, {"set_card_identity", 0}, {"discard_until_count", 0},
    {"restriction", 0}, {"re_yell", 0}, {"activation_restriction", 0},
    {"modify_limit", 0}, {"shuffle", 0}, {"custom", 0},
    {"modify_yell_source", 0}, {"specify_heart_color", 0},
    {"modify_required_hearts_success", 0}, {"all_blade_timing", 0},
    {"reveal_until_live_card", 0}, {"reveal_until_chosen_card", 0},
    {"choose_target_player", 0}, {"choose_required_hearts", 0},
    {"perform_yell", 0}, {"repeat_procedure", 1}, {"sequential", 1},
    {"conditional_alternative", 1}, {"look_and_select", 1},
    {"conditional_on_result", 1}, {"conditional_on_optional", 1},
    {"conditional_optional", 1}, {"compound_action", 1},
    {"opponent_action", 1}, {"action_by", 1}, {"sequential_cost", 1},
    {"choice_condition", 1}, {"energy_condition", 1}, {"do_nothing", 0}
};

static const ExecutorEntry *find_executor(const char *action)
{
    if (!action) return NULL;
    for (size_t i = 0; i < sizeof(executor_table) / sizeof(executor_table[0]); i++)
        if (strcmp(executor_table[i].action, action) == 0)
            return &executor_table[i];
    return NULL;
}

int rb_executor_has_executor(const char *action)
{
    return find_executor(action) != NULL;
}

int rb_executor_is_structural(const char *action)
{
    const ExecutorEntry *entry = find_executor(action);
    return entry && entry->structural;
}

static const char *effect_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect || !key) return NULL;
    for (int i = 0; i < effect->n_extra; i++)
        if (effect->extra_k[i] && strcmp(effect->extra_k[i], key) == 0)
            return effect->extra_v[i];
    return NULL;
}

static void push_effect_verdict(const AbilityEffect *effect)
{
    if (!effect || rb_executor_is_structural(effect->action)) return;
    char details[128];
    if (effect->count >= 0)
        snprintf(details, sizeof(details), "%s %d", effect->action, effect->count);
    else
        snprintf(details, sizeof(details), "%s", effect->action);
    rb_log_push_verdict_effect(effect->text ? effect->text : "",
                               effect->action ? effect->action : "", details);
}

static int execute_misc(GameState *g, int actor, const AbilityEffect *effect, int host_cid)
{
    int resolved = 0;
    return rb_execute_misc_effect_ex(g, actor, &g->p[actor], effect, host_cid, &resolved);
}

static int execute_modify_required_hearts_global(GameState *g, int actor,
                                                  const AbilityEffect *effect)
{
    const char *colors[8];
    int n_colors = 0;
    const char *csv = effect_extra(effect, "heart_colors");
    if (!csv) csv = effect_extra(effect, "heart_color");
    if (csv) {
        char buffer[256];
        snprintf(buffer, sizeof(buffer), "%s", csv);
        char *token = strtok(buffer, ",");
        while (token && n_colors < 8) {
            colors[n_colors++] = token;
            token = strtok(NULL, ",");
        }
    }
    int value = effect->count >= 0 ? effect->count : 1;
    const char *value_text = effect_extra(effect, "value");
    if (value_text) value = atoi(value_text);
    const char *operation = effect_extra(effect, "operation");
    if (!operation) operation = "increase";
    rb_execute_modify_required_hearts_standard(g, actor, operation, value,
                                                colors, n_colors,
                                                effect->target ? effect->target : "self",
                                                effect->text);
    return 1;
}

static int execute_repeat(GameState *g, int actor, AbilityEffect *effect, int host_cid)
{
    int count = effect->count >= 0 ? effect->count : 1;
    if (effect->repeat_limit > 0) count = effect->repeat_limit + 1;
    for (int r = 0; r < count; r++) {
        for (int i = 0; i < effect->n_child; i++) {
            rb_execute_effect_ex(g, actor, effect->child[i], host_cid);
            if (rb_has_pending_choice(g)) return 1;
        }
    }
    return 1;
}

int rb_executor_execute(GameState *g, int actor, AbilityEffect *effect, int host_cid)
{
    if (!g || !effect || !effect->action) return 0;
    if (!find_executor(effect->action)) {
        fprintf(stderr, "[EXECUTOR_MISS] action=%s\n", effect->action);
        return 0;
    }
    if (rb_ability_debug_enabled())
        fprintf(stderr, "[EXECUTOR] action=%s host=%d cond=%d\n",
                effect->action, host_cid, effect->has_condition);
    const char *action = effect->action;
    int result = 1;

    if (strcmp(action, "draw_card") == 0 || strcmp(action, "draw") == 0) {
        result = rb_effect_draw_card(g, actor, effect, host_cid);
    } else if (strcmp(action, "draw_until_count") == 0) {
        fprintf(stderr, "[EXECUTOR_DRAW_UNTIL] actor=%d ptr=%p\n", actor, (void *)effect);
        rb_effect_draw_until_count(g, actor, effect);
    } else if (strcmp(action, "move_cards") == 0) {
        rb_effect_move_cards(g, actor, effect);
    } else if (strcmp(action, "discard_card") == 0) {
        AbilityEffect moved = *effect;
        moved.source = effect->source ? effect->source : (char *)"hand";
        moved.destination = effect->destination ? effect->destination : (char *)"waitroom";
        rb_effect_move_cards(g, actor, &moved);
    } else if (strcmp(action, "select") == 0) {
        rb_effect_select_effect(g, actor, effect, host_cid);
    } else if (strcmp(action, "select_cards") == 0) {
        rb_effect_select_cards(g, actor, effect);
    } else if (strcmp(action, "look_and_select") == 0) {
        rb_effect_look_and_select(g, actor, effect);
    } else if (strcmp(action, "select_number") == 0) {
        rb_effect_select_number(g, actor, effect);
    } else if (strcmp(action, "look_at") == 0) {
        rb_effect_look_at(g, actor, effect);
    } else if (strcmp(action, "reveal") == 0) {
        rb_effect_reveal(g, actor, effect);
    } else if (strcmp(action, "reveal_per_group") == 0) {
        rb_effect_reveal_per_group(g, actor, effect);
    } else if (strcmp(action, "reveal_until_live_card") == 0) {
        rb_effect_reveal_until_live_card(g, actor, effect);
    } else if (strcmp(action, "reveal_until_chosen_card") == 0) {
        rb_effect_reveal_until_chosen_card(g, actor, effect);
    } else if (strcmp(action, "change_state") == 0) {
        rb_effect_change_state(g, actor, effect, host_cid);
    } else if (strcmp(action, "position_change") == 0 ||
               strcmp(action, "rotation") == 0) {
        result = execute_misc(g, actor, effect, host_cid);
    } else if (strcmp(action, "gain_resource") == 0 ||
               strcmp(action, "place_energy_under_member") == 0 ||
               strcmp(action, "play_baton_touch") == 0 ||
               strcmp(action, "pay_energy") == 0 ||
               strcmp(action, "discard_until_count") == 0 ||
               strcmp(action, "restriction") == 0 ||
               strcmp(action, "activation_restriction") == 0 ||
               strcmp(action, "choice") == 0 ||
               strcmp(action, "choose_target_player") == 0 ||
               strcmp(action, "choose_required_hearts") == 0 ||
               strcmp(action, "re_yell") == 0 ||
               strcmp(action, "perform_yell") == 0 ||
               strcmp(action, "shuffle") == 0 ||
               strcmp(action, "custom") == 0) {
        result = execute_misc(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_score") == 0 || strcmp(action, "gain_score") == 0) {
        if (host_cid < 0 && !effect->target && !effect->self_target_field[0]) {
            const char *operation = effect_extra(effect, "operation");
            int value = effect->count >= 0 ? effect->count : 1;
            const char *value_text = effect_extra(effect, "value");
            if (value_text) value = atoi(value_text);
            int delta = operation && !strcmp(operation, "remove") ? -value : value;
            g->p[actor].score += delta;
            result = 1;
        } else {
            result = rb_execute_modify_score(g, actor, effect);
        }
    } else if (strcmp(action, "modify_required_hearts") == 0 ||
               strcmp(action, "modify_required_hearts_success") == 0) {
        result = strcmp(action, "modify_required_hearts_success") == 0
                     ? rb_execute_modify_required_hearts_success(g, actor, effect)
                     : rb_execute_modify_required_hearts(g, actor, effect);
    } else if (strcmp(action, "modify_required_hearts_global") == 0) {
        result = execute_modify_required_hearts_global(g, actor, effect);
    } else if (strcmp(action, "set_cost") == 0) {
        rb_effect_set_cost(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_cost_to_use") == 0) {
        rb_effect_set_cost_to_use(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_cost") == 0) {
        rb_effect_modify_cost(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_yell_count") == 0) {
        result = rb_execute_modify_yell_count(g, actor, effect);
    } else if (strcmp(action, "set_blade_type") == 0) {
        rb_effect_set_blade_type(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_blade_count") == 0) {
        rb_effect_set_blade_count(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_heart_type") == 0) {
        rb_effect_set_heart_type(g, actor, effect, host_cid);
    } else if (strcmp(action, "specify_heart_color") == 0) {
        rb_effect_specify_heart_color(g, actor, effect, host_cid);
    } else if (strcmp(action, "activate_ability") == 0) {
        result = rb_translated_execute_activate_ability(g, actor, effect, host_cid);
    } else if (strcmp(action, "invalidate_ability") == 0) {
        result = rb_translated_execute_invalidate_ability(g, actor, effect);
    } else if (strcmp(action, "suppress_ability_trigger") == 0) {
        result = rb_translated_execute_suppress_ability_trigger(g, actor, effect);
    } else if (strcmp(action, "gain_ability") == 0) {
        result = rb_translated_execute_gain_ability_effect(g, actor, effect, host_cid);
    } else if (strcmp(action, "gain_ability_from_source") == 0) {
        result = rb_translated_execute_gain_ability_from_source(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_card_identity") == 0) {
        result = rb_translated_execute_set_card_identity_effect(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_limit") == 0) {
        result = rb_execute_modify_limit(g, actor, effect);
    } else if (strcmp(action, "reduce_live_card_set_limit") == 0) {
        rb_effect_reduce_live_card_set_limit(g, actor, effect, host_cid);
    } else if (strcmp(action, "all_blade_timing") == 0) {
        rb_effect_all_blade_timing(g, actor, effect, host_cid);
    } else if (strcmp(action, "energy_placement") == 0) {
        rb_effect_energy_placement(g, actor, effect);
    } else if (strcmp(action, "energy_state_change") == 0) {
        rb_effect_energy_state_change(g, actor, effect);
    } else if (strcmp(action, "sequential") == 0) {
        rb_compound_sequential(g, actor, effect, host_cid);
    } else if (strcmp(action, "conditional_alternative") == 0) {
        rb_compound_conditional_alternative(g, actor, effect, -1, host_cid);
    } else if (strcmp(action, "conditional_on_result") == 0) {
        rb_compound_conditional_on_result(g, actor, effect, host_cid);
    } else if (strcmp(action, "conditional_on_optional") == 0 ||
               strcmp(action, "conditional_optional") == 0) {
        rb_compound_conditional_on_optional(g, actor, effect, -1, host_cid);
    } else if (strcmp(action, "repeat_procedure") == 0) {
        result = execute_repeat(g, actor, effect, host_cid);
    } else if (strcmp(action, "compound_action") == 0) {
        if (effect->primary_effect)
            rb_execute_effect_ex(g, actor, effect->primary_effect, host_cid);
        else if (effect->n_child > 0)
            rb_execute_effect_ex(g, actor, effect->child[0], host_cid);
    } else if (strcmp(action, "opponent_action") == 0) {
        AbilityEffect routed = *effect;
        routed.target = (char *)"opponent";
        if (effect->primary_effect)
            routed.primary_effect = effect->primary_effect;
        rb_execute_effect_ex(g, actor, &routed, host_cid);
    } else if (strcmp(action, "action_by") == 0 ||
               strcmp(action, "choice_condition") == 0 ||
               strcmp(action, "energy_condition") == 0 ||
               strcmp(action, "modify_yell_source") == 0 ||
               strcmp(action, "do_nothing") == 0) {
        result = 1;
    } else {
        result = 0;
    }

    if (result) push_effect_verdict(effect);
    return result;
}

int rb_execute_effect_via_registry(GameState *g, int actor, AbilityEffect *effect,
                                  int host_cid)
{
    return rb_executor_execute(g, actor, effect, host_cid);
}
