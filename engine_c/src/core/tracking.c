#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdint.h>

/* Helpers ported from modifiers.rs / abilities.rs — minimal stubs that mirror
   the Rust clear/reset helpers so tracking.rs can call them. */

static void rb_clear_auto_ability_trigger_tracking(GameState *g){
    g->n_auto_ability_trigger_counts = 0;
}

static void rb_reset_change_flags(GameState *g){
    g->position_change_occurred_this_turn = 0;
    g->formation_change_occurred_this_turn = 0;
    g->opponent_live_success_this_turn = 0;
}

static void rb_reset_loop_detection(GameState *g){
    g->n_game_state_history = 0;
    g->loop_detected = 0;
}

static void rb_clear_area_placement_tracking(GameState *g){
    g->n_areas_placed_this_turn = 0;
}

void rb_reset_keyword_tracking(GameState *g){
    g->n_turn1_abilities_played = 0;
    g->n_turn2_abilities_played = 0;
    g->player1_cheer_blade_heart_count = 0;
    g->player2_cheer_blade_heart_count = 0;
    g->p[0].deck_refreshed_this_turn = 0;
    g->p[1].deck_refreshed_this_turn = 0;
    g->last_resolution_cards_p1.n = 0;
    g->last_resolution_cards_p2.n = 0;
    rb_clear_auto_ability_trigger_tracking(g);
    rb_reset_change_flags(g);
    g->cheer_check_completed = 0;
    rb_reset_loop_detection(g);
    g->baton_touch_count_p1 = 0;
    g->baton_touch_count_p2 = 0;
    g->n_baton_touch_arriving_card_ids = 0;
    g->baton_touch_zero_cost = 0;
    g->baton_touch_replaced_member_cost = -1;
    g->baton_touch_replaced_member_id = -1;
    g->baton_touch_arriving_card_id = -1;
    rb_clear_area_placement_tracking(g);
}

void rb_add_yell_count_modifier(GameState *g, uint8_t player_slot, int32_t delta){
    if(g->n_yell_count_modifiers >= 32) return;
    g->yell_count_modifiers[g->n_yell_count_modifiers].slot = player_slot;
    g->yell_count_modifiers[g->n_yell_count_modifiers].delta = delta;
    g->n_yell_count_modifiers++;
}

uint8_t rb_effective_cheer_checks_required(const GameState *g, const char *player_id, uint8_t base){
    uint8_t slot = 2;
    if(player_id && (!strcmp(player_id, "p1") || !strcmp(player_id, "1"))) slot = 1;
    int eff_base = (g->cheer_check_base >= 0) ? g->cheer_check_base : (int)base;
    int sum = 0;
    for(int i=0;i<g->n_yell_count_modifiers;i++){
        if(g->yell_count_modifiers[i].slot == slot) sum += g->yell_count_modifiers[i].delta;
    }
    int total = eff_base + sum;
    if(total < 0) total = 0;
    if(total > 255) total = 255;
    return (uint8_t)total;
}

int rb_perform_cheer_check(GameState *g, const char *player_id, uint8_t blade_count){
    if(g->cheer_check_base < 0){
        g->cheer_check_base = blade_count;
    }
    g->cheer_checks_required = rb_effective_cheer_checks_required(g, player_id, blade_count);

    int pl = 0;
    if(player_id && !strcmp(player_id, "p2")) pl = 1;
    /* mirror Rust: pick player by id; we use p1/p2 string. */
    RbPlayer *player = &g->p[pl];
    int from_bottom = 0; /* TODO: player.yell_from_bottom not ported; assume top */

    for(int i=0;i<blade_count;i++){
        if(player->deck.n==0 && player->discard.n>0){
            /* refresh from waitroom when deck runs out mid-draw — rule 10.2.1 */
            rb_player_refresh(g, pl);
        }
        int card_id = -1;
        if(from_bottom){
            /* draw_bottom not yet ported; fallback to top */
            if(player->deck.n>0) card_id = player->deck.cards[--player->deck.n];
        } else {
            if(player->deck.n>0) card_id = player->deck.cards[--player->deck.n];
        }
        if(card_id != -1){
            if(g->resolution.n < RB_MAX_ZONE) g->resolution.cards[g->resolution.n++] = card_id;
            g->cheer_checks_done++;
        }
    }
    if(g->cheer_checks_done >= g->cheer_checks_required){
        g->cheer_check_completed = 1;
    }
    return 0;
}

int rb_check_required_hearts(const GameState *g){
    if(g->cheer_checks_done < g->cheer_checks_required){
        return -1;
    }
    return 0;
}

int rb_is_action_prohibited(const GameState *g, const char *action){
    if(!action) return 0;
    for(int i=0;i<g->n_prohibition;i++){
        if(strstr(g->prohibition[i], action)) return 1;
    }
    return 0;
}
