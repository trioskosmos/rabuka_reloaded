#include "rabuka.h"

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

static const char *effect_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect || !key) return NULL;
    for (int i = 0; i < effect->n_extra; i++) {
        if (effect->extra_k[i] && strcmp(effect->extra_k[i], key) == 0)
            return effect->extra_v[i];
    }
    return NULL;
}

static void translated_log(GameState *g, int actor, const char *text)
{
    if (!g) return;
    char line[256];
    snprintf(line, sizeof(line), "P%d %s", actor + 1, text ? text : "");
    rb_log_push_verdict(line, "rule_log", 1);
}

int rb_translated_execute_gain_ability_effect(GameState *g, int actor,
                                                AbilityEffect *effect, int host_cid)
{
    if (!g || !effect) return 0;
    if (effect->source && strcmp(effect->source, "stage") == 0 &&
        g->n_selected_cards == 0) {
        const char *group = effect_extra(effect, "group_names");
        if (group && *group) {
            int target_player = effect->target && !strcmp(effect->target, "opponent")
                                ? actor ^ 1 : actor;
            int candidate_positions[RB_STAGE_SIZE];
            int count = 0;
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int cid = g->p[target_player].stage[i];
                if (cid != RB_EMPTY_SLOT && rb_card_matches_group_str(cid, group)) {
                    candidate_positions[count] = i;
                    count++;
                }
            }
            if (count > 0) {
                int pick = effect->count > 0 ? effect->count : 1;
                if (pick > count) pick = count;
                rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage",
                               effect->card_type_field[0] ? effect->card_type_field : "member_card",
                               pick, 1, "gain_ability_targets");
                strncpy(g->queue.pending.filter_group, group,
                        sizeof(g->queue.pending.filter_group) - 1);
                g->queue.pending.n_filtered_indices = count;
                for (int i = 0; i < count; i++)
                    g->queue.pending.filtered_indices[i] = candidate_positions[i];
                strncpy(g->queue.pending.target_player_id,
                        effect->target ? effect->target : "self",
                        sizeof(g->queue.pending.target_player_id) - 1);
                rb_queue_pause_for_choice(g, &g->queue.pending);
                g->queue.resume_mode = 2;
                g->queue.resume_eff = effect;
                g->queue.deferred = effect;
                g->queue.resume_actor = actor;
                g->queue.resume_host = host_cid;
                return 1;
            }
        }
    }

    int targets[RB_MAX_RECENTLY_MOVED];
    int target_count = 0;
    if (g->n_selected_cards > 0) {
        for (int i = 0; i < g->n_selected_cards && target_count < RB_MAX_RECENTLY_MOVED; i++)
            targets[target_count++] = g->selected_cards[i];
    } else if (effect_extra(effect, "anaphora") &&
               !strcmp(effect_extra(effect, "anaphora"), "cost_waited") &&
               g->n_last_cost_waited_members > 0) {
        for (int i = 0; i < g->n_last_cost_waited_members && target_count < RB_MAX_RECENTLY_MOVED; i++)
            targets[target_count++] = g->last_cost_waited_members[i];
    } else {
        int activating = g->activating_card >= 0 ? g->activating_card : host_cid;
        if (activating >= 0) targets[target_count++] = activating;
    }

    const char *text = effect_extra(effect, "ability_gain");
    if (!text || !*text) text = effect->text;
    if (!text) text = "";
    for (int i = 0; i < target_count; i++) {
        g->n_selected_cards = 1;
        g->selected_cards[0] = targets[i];
        AbilityEffect copy = *effect;
        copy.n_child = 0;
        copy.primary_effect = NULL;
        copy.alternative_effect = NULL;
        copy.look_action = NULL;
        copy.select_action = NULL;
        copy.followup_action = NULL;
        copy.optional_action = NULL;
        copy.conditional_action = NULL;
        rb_gain_ability(g, actor, &copy);
    }
    g->n_selected_cards = 0;
    translated_log(g, actor, "[[log_gain_ability]]");
    return target_count > 0;
}

static int enqueue_matching_ability(GameState *g, int actor, int card_id,
                                   const char *trigger)
{
    int count = rb_card_num_abilities((uint32_t)card_id);
    int queued = 0;
    for (int i = 0; i < count; i++) {
        Ability ability;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ability)) continue;
        int match = !trigger || !*trigger ||
                    (ability.triggers && strstr(ability.triggers, trigger));
        if (match) {
            int index = rb_queue_make_entry(g, card_id, i);
            if (index >= 0) {
                snprintf(g->queue.entries[index].player_id,
                         sizeof(g->queue.entries[index].player_id), "%s",
                         actor == 0 ? "p1" : "p2");
                rb_queue_set_entry_trigger(g, index, trigger, NULL, 0);
                queued++;
            }
        }
        rb_free_ability(&ability);
        if (queued) break;
    }
    return queued;
}

int rb_translated_execute_activate_ability(GameState *g, int actor,
                                             AbilityEffect *effect, int host_cid)
{
    if (!g || !effect) return 0;
    const char *source_card = effect_extra(effect, "source_card");
    const char *target_trigger = effect_extra(effect, "target_trigger");
    if (!target_trigger || !*target_trigger) {
        int target = g->activating_card >= 0 ? g->activating_card : host_cid;
        if (target >= 0) {
            AbilityEffect copy = *effect;
            copy.text = effect->text ? effect->text : (char *)"";
            rb_gain_ability(g, actor, &copy);
        }
        return 1;
    }
    int selected[RB_MAX_RECENTLY_MOVED];
    int selected_count = 0;
    if (source_card && !strcmp(source_card, "previous_selected")) {
        selected_count = g->n_selected_cards;
        if (selected_count > RB_MAX_RECENTLY_MOVED)
            selected_count = RB_MAX_RECENTLY_MOVED;
        memcpy(selected, g->selected_cards, sizeof(int) * (size_t)selected_count);
    } else if (source_card && !strcmp(source_card, "cost_card") &&
               g->n_recently_moved > 0) {
        selected[0] = g->recently_moved[g->n_recently_moved - 1];
        selected_count = 1;
    } else if (host_cid >= 0) {
        selected[0] = host_cid;
        selected_count = 1;
    }

    int queued = 0;
    for (int i = 0; i < selected_count; i++)
        queued += enqueue_matching_ability(g, actor, selected[i], target_trigger);
    if (queued) {
        translated_log(g, actor, "[[log_activated_ability]]");
    }
    return queued > 0;
}

int rb_translated_execute_invalidate_ability(GameState *g, int actor,
                                              AbilityEffect *effect)
{
    if (!g || !effect) return 0;
    const char *trigger = effect_extra(effect, "target_trigger");
    if (!trigger || !*trigger) return 0;
    const char *duration = effect_extra(effect, "duration");
    if (!duration) duration = "permanent";
    const char *target = effect->target ? effect->target : "self";
    int target_player = rb_resolve_target_player(g, target);
    if (target_player < 0 || target_player > 1) target_player = actor;
    const char *card_type = effect->card_type_field[0]
                            ? effect->card_type_field
                            : effect_extra(effect, "card_type");
    const char *group = effect_extra(effect, "group_names");

    if (effect->self_target_field[0] && !strcmp(effect->self_target_field, "true")) {
        int card_id = g->activating_card;
        return rb_try_add_ability_invalidation(g, card_id, trigger, duration);
    }

    int candidates[RB_STAGE_SIZE];
    int positions[RB_STAGE_SIZE];
    int count = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int card_id = g->p[target_player].stage[i];
        if (card_id == RB_EMPTY_SLOT) continue;
        if (card_type && !rb_card_matches_type(card_id, card_type)) continue;
        if (group && !rb_card_matches_group_str(card_id, group)) continue;
        if (!rb_card_has_ability_trigger_for(g, card_id, trigger)) continue;
        if (rb_ability_is_invalidated(g, card_id, trigger)) continue;
        candidates[count] = card_id;
        positions[count] = i;
        count++;
    }
    if (count == 0) return 0;

    int selected = -1;
    if (g->n_selected_cards > 0) {
        selected = g->selected_cards[0];
    } else if (count > 1) {
        int pick = effect->count > 0 ? effect->count : 1;
        if (pick > count) pick = count;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage",
                       card_type ? card_type : "member_card", pick,
                       effect->is_optional, "invalidate_ability");
        g->queue.pending.n_filtered_indices = count;
        for (int i = 0; i < count; i++)
            g->queue.pending.filtered_indices[i] = positions[i];
        strncpy(g->queue.pending.filter_group, group ? group : "",
                sizeof(g->queue.pending.filter_group) - 1);
        strncpy(g->queue.pending.target_player_id, target,
                sizeof(g->queue.pending.target_player_id) - 1);
        rb_queue_pause_for_choice(g, &g->queue.pending);
        g->queue.resume_mode = 2;
        g->queue.resume_eff = effect;
        g->queue.deferred = effect;
        g->queue.resume_actor = actor;
        g->queue.resume_host = g->activating_card;
        return 1;
    } else {
        selected = candidates[0];
    }

    int success = 0;
    for (int i = 0; i < count; i++) {
        if (candidates[i] == selected)
            success = rb_try_add_ability_invalidation(g, selected, trigger, duration);
    }
    g->n_selected_cards = 0;
    return success;
}

int rb_translated_execute_suppress_ability_trigger(GameState *g, int actor,
                                                    AbilityEffect *effect)
{
    if (!g || !effect) return 0;
    const char *trigger = effect_extra(effect, "suppressed_trigger");
    rb_suppress_ability_trigger(g, actor, effect, g->activating_card);
    char text[128];
    snprintf(text, sizeof(text), "[[log_suppress_ability:trigger=%s]]",
             trigger ? trigger : "unknown");
    translated_log(g, actor, text);
    return 1;
}

int rb_translated_execute_set_card_identity_effect(GameState *g, int actor,
                                                    AbilityEffect *effect,
                                                    int host_cid)
{
    if (!g || !effect) return 0;
    rb_effect_set_card_identity(g, actor, effect, host_cid);
    return 1;
}

static int trigger_filter_matches(const Ability *ability, const char *filter)
{
    if (!filter || !*filter) return 1;
    char buffer[256];
    snprintf(buffer, sizeof(buffer), "%s", filter);
    char *token = strtok(buffer, ",");
    while (token) {
        if (ability->triggers &&
            (strstr(ability->triggers, token) || strstr(token, ability->triggers))) {
            return 1;
        }
        token = strtok(NULL, ",");
    }
    return 0;
}

static int copy_source_ability(GameState *g, int target_card, const Ability *source)
{
    if (!g || !source) return 0;
    Ability copy;
    memset(&copy, 0, sizeof(copy));
    copy.full_text = rb_strdup2(source->full_text ? source->full_text : "");
    copy.triggerless_text = rb_strdup2(source->triggerless_text ? source->triggerless_text : "");
    copy.triggers = rb_strdup2(source->triggers ? source->triggers : "");
    copy.use_limit = source->use_limit;
    copy.is_null = source->is_null;
    copy.cost = source->cost ? rb_effect_deep_clone(source->cost) : NULL;
    copy.effect = source->effect ? rb_effect_deep_clone(source->effect) : NULL;
    int index = rb_register_gained_ability(g, target_card, &copy);
    if (index < 0) {
        rb_free_ability(&copy);
        return 0;
    }
    return 1;
}

int rb_translated_execute_gain_ability_from_source(GameState *g, int actor,
                                                    AbilityEffect *effect,
                                                    int host_cid)
{
    if (!g || !effect) return 0;
    int target_card = g->activating_card >= 0 ? g->activating_card : host_cid;
    if (target_card < 0) return 0;
    while (rb_card_num_gained_abilities(g, target_card) > 0)
        rb_remove_gained_ability(g, target_card, 0);

    int owner = rb_owner_of_card(g, target_card);
    if (owner < 0) owner = actor;
    int area = -1;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (g->p[owner].stage[i] == target_card) {
            area = i;
            break;
        }
    }
    if (area < 0) return 0;

    const char *card_type = effect->card_type_field[0]
                            ? effect->card_type_field
                            : effect_extra(effect, "card_type");
    const char *cost_limit_text = effect_extra(effect, "cost_limit");
    int cost_limit = cost_limit_text ? atoi(cost_limit_text) : -1;
    const char *cost_op = effect_extra(effect, "cost_limit_operator");
    const char *group = effect_extra(effect, "group_names");
    const char *trigger_filter = effect_extra(effect, "trigger_filter");
    int copied = 0;
    RbBag *under = &g->p[owner].under_cards[area];
    for (int i = 0; i < under->n; i++) {
        int source_card = under->cards[i];
        if (source_card < 0) continue;
        if (card_type && !rb_card_matches_type(source_card, card_type)) continue;
        if (cost_limit >= 0 && !rb_card_matches_cost_limit(source_card, cost_limit, cost_op)) continue;
        if (group && !rb_card_matches_group_str(source_card, group)) continue;
        int count = rb_card_num_abilities((uint32_t)source_card);
        for (int a = 0; a < count; a++) {
            Ability source;
            if (!rb_decode_card_ability((uint32_t)source_card, a, &source)) continue;
            if (trigger_filter_matches(&source, trigger_filter))
                copied += copy_source_ability(g, target_card, &source);
            rb_free_ability(&source);
        }
    }
    if (copied) {
        char line[128];
        snprintf(line, sizeof(line), "[[log_gain_ability_from_source:%d]]", copied);
        translated_log(g, actor, line);
        rb_recalc_constants(g);
    }
    return copied > 0;
}

int rb_translated_execute_custom_effect(GameState *g, int actor,
                                         AbilityEffect *effect,
                                         const char *action_str)
{
    if (!g || !effect) return 0;
    const char *placement = effect_extra(effect, "placement_order");
    if (placement && strcmp(placement, "any_order") == 0) {
        AbilityEffect routed = *effect;
        routed.action = (char *)"move_cards";
        if (!routed.source) routed.source = (char *)"looked_at";
        if (!routed.destination) routed.destination = (char *)"deck_top";
        rb_effect_move_cards(g, actor, &routed);
        return 1;
    }
    const char *duration = effect_extra(effect, "duration");
    if (duration && *duration) {
        rb_gain_ability(g, actor, effect);
        return 1;
    }
    char text[128];
    snprintf(text, sizeof(text), "[[log_custom_effect:%s]]",
             action_str ? action_str : "custom");
    translated_log(g, actor, text);
    return 1;
}
