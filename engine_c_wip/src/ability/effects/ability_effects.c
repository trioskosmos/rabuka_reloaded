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
    } else if (host_cid >= 0) {
        targets[target_count++] = host_cid;
    } else {
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (g->p[actor].stage[i] != RB_EMPTY_SLOT) {
                targets[target_count++] = g->p[actor].stage[i];
                break;
            }
        }
    }
    const char *text = effect_extra(effect, "ability_gain");
    if (!text || !*text) text = effect->text;
    if (!text) text = "";
    for (int i = 0; i < target_count; i++) {
        AbilityEffect copy = *effect;
        copy.n_child = 0;
        copy.primary_effect = NULL;
        copy.alternative_effect = NULL;
        copy.followup_action = NULL;
        copy.optional_action = NULL;
        copy.conditional_action = NULL;
        rb_gain_ability(g, actor, &copy);
        if (g->n_selected_cards > 0) {
            int found = 0;
            for (int j = 0; j < g->n_selected_cards; j++)
                if (g->selected_cards[j] == targets[i]) found = 1;
            if (!found) break;
        }
    }
    translated_log(g, actor, "[[log_gain_ability]]");
    return 1;
}

int rb_translated_execute_activate_ability(GameState *g, int actor,
                                             AbilityEffect *effect, int host_cid)
{
    if (!g || !effect) return 0;
    rb_activate_ability_effect(g, actor, effect, host_cid);
    return 1;
}

int rb_translated_execute_invalidate_ability(GameState *g, int actor,
                                              AbilityEffect *effect)
{
    if (!g || !effect) return 0;
    rb_invalidate_ability(g, actor, effect);
    return 1;
}

int rb_translated_execute_suppress_ability_trigger(GameState *g, int actor,
                                                    AbilityEffect *effect)
{
    if (!g || !effect) return 0;
    const char *trigger = effect_extra(effect, "suppressed_trigger");
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

int rb_translated_execute_gain_ability_from_source(GameState *g, int actor,
                                                    AbilityEffect *effect,
                                                    int host_cid)
{
    if (!g || !effect) return 0;
    rb_gain_ability_from_source(g, actor, effect, host_cid);
    return 1;
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
