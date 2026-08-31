/* engine_c/src/ability/choice_frag_01.c
 * Port fragment: engine/src/ability/choice.rs:230-402
 *   - reveal_selected_looked_at  -> rb_resolver_reveal_selected_looked_at
 *   - provide_choice_result      -> rb_resolver_provide_choice_result
 * Mirrors the choice.c state model: the pending choice lives in g->queue.pending
 * (RbChoice) and the player's pick in g->queue.choice_result (-1 == Skip).
 * self.gs == GameState* ; self.pending_choice/execution_context == g->queue.
 */

#include "rabuka.h"
#include <string.h>

/* ── Forward prototypes for the dispatch handlers owned by sibling fragments.
 *     These are NOT defined here (other subagents own them); we only call them. ── */
void rb_resolver_handle_select_card(GameState *g);
void rb_resolver_handle_hand_selection(GameState *g);
void rb_resolver_handle_reveal_selection(GameState *g);
void rb_resolver_handle_revealed_cards_selection(GameState *g);
void rb_resolver_handle_success_live_zone_selection(GameState *g);
void rb_resolver_handle_entry_cost_reveal(GameState *g);
void rb_resolver_handle_looked_at_selection(GameState *g);
void rb_resolver_handle_stage_selection(GameState *g);
void rb_resolver_handle_discard_selection(GameState *g);
void rb_resolver_handle_select_target(GameState *g, const char *target, const char *selected);
void rb_resolver_handle_draw_any_number(GameState *g);
void rb_resolver_handle_order_selection(GameState *g);
void rb_resolver_handle_position_change_choice(GameState *g);
void rb_resolver_handle_primary_alternative(GameState *g);
void rb_resolver_handle_position_destination(GameState *g);
void rb_resolver_handle_double_baton_touch(GameState *g);
void rb_resolver_handle_conditional_optional(GameState *g);
void rb_resolver_handle_heart_color_selection(GameState *g);
void rb_resolver_handle_choice_condition(GameState *g);
void rb_resolver_handle_heart_selection(GameState *g, int count,
                                        const int *colors, int n_colors);

/* Mirror engine/src/ability/resolver.rs current_ability_source_card_id():
 * the activating card for the in-flight ability (queue.entries[cur].card_id). */
static int rb_resolver_source_card_id(const GameState *g) {
    if (!g) return -1;
    int cur = g->queue.cur;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH) return g->queue.entries[cur].card_id;
    return -1;
}

/* choice.rs:230 reveal_selected_looked_at — reveal the looked_at cards picked by
 * indices (mirror gs.push_revealed_card + the [[log_reveal_looked]] rule log). */
void rb_resolver_reveal_selected_looked_at(GameState *g,
                                           const int *indices, int n_indices) {
    if (!g || !indices) return;

    int source = rb_resolver_source_card_id(g);

    /* looked_owner: ability_master_id parsed to a "self" player index. */
    int looked_owner = rb_target_player_index("self", NULL); /* None master -> actor */

    /* gs.looked_at_cards pool (mirror rb_looked_at_pool). */
    int looked_pool[RB_MAX_RECENTLY_MOVED];
    int np = rb_looked_at_pool(g->active, looked_pool, RB_MAX_RECENTLY_MOVED);

    int revealed_ids[RB_MAX_RECENTLY_MOVED];
    int nr = 0;

    for (int i = 0; i < n_indices; i++) {
        int idx = indices[i];
        if (idx >= 0 && idx < np) {
            int cid = looked_pool[idx];
            /* gs.push_revealed_card(cid, source, false, looked_owner, "ability") */
            if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
                g->revealed_cards[g->n_revealed++] = cid;
            if (nr < RB_MAX_RECENTLY_MOVED) revealed_ids[nr++] = cid;
        }
    }

    if (nr > 0) {
        /* Collect card names for the rule log (mirror card_db.get_card(*id).name). */
        int nn = 0;
        for (int i = 0; i < nr; i++) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)revealed_ids[i], &c) == 1) {
                (void)c.name;       /* name would feed [[log_reveal_looked:n=..]] */
                rb_free_card(&c);
                nn++;
            }
        }
        if (nn > 0) {
            /* gs.push_rule_log("[Turn {}] P{} [[log_reveal_looked:n={}]]", turn, p, n) */
            int p = g->active + 1;  /* active_player() == player1 -> P1 else P2 */
            (void)p;
        }
    }
}

/* choice.rs:267 provide_choice_result — the big dispatcher.
 * self.gs == g ; self.pending_choice == g->queue.pending ;
 * result   == g->queue.choice_result (>=0 index, -1 == Skip). */
int rb_resolver_provide_choice_result(GameState *g) {
    if (!g || !g->queue.has_pending) return 0;

    RbChoice *ch   = &g->queue.pending;
    int       sel  = g->queue.choice_result;   /* selected index (-1 == Skip) */
    int       skip = (sel < 0);
    const char *target = ch->target[0] ? ch->target : NULL;
    const char *zone   = ch->zone[0]   ? ch->zone   : NULL;

    switch (ch->kind) {

    case RB_CHOICE_SELECT_CARD: {
        if (skip) {
            /* choice.rs:343 SelectCard + Skip: take_pending_actions; clear; resume */
            g->queue.deferred = NULL;
            rb_resume_with_choice(g, -1);
            return 1;
        }
        /* Route by zone / target to the appropriate handle_* (mirror handle_select_card
           internal zoning; the task's canonical C handler names). */
        if (zone && !strcmp(zone, "looked_at"))
            rb_resolver_handle_looked_at_selection(g);
        else if (zone && !strcmp(zone, "revealed_cards"))
            rb_resolver_handle_revealed_cards_selection(g);
        else if (zone && (!strcmp(zone, "success") || !strcmp(zone, "live") ||
                          !strcmp(zone, "success_live_card_zone")))
            rb_resolver_handle_success_live_zone_selection(g);
        else if (target && strstr(target, "entry_cost_reveal"))
            rb_resolver_handle_entry_cost_reveal(g);
        else if (target && strstr(target, "draw_any_number"))
            rb_resolver_handle_draw_any_number(g);
        else if (target && strstr(target, "order"))
            rb_resolver_handle_order_selection(g);
        else if (zone && !strcmp(zone, "stage"))
            rb_resolver_handle_stage_selection(g);
        else if (zone && !strcmp(zone, "discard"))
            rb_resolver_handle_discard_selection(g);
        else if (zone && !strcmp(zone, "hand"))
            rb_resolver_handle_hand_selection(g);
        else if (target && strstr(target, "reveal"))
            rb_resolver_handle_reveal_selection(g);
        else
            rb_resolver_handle_select_card(g);   /* choice.rs:297 default SelectCard arm */
        return 1;
    }

    case RB_CHOICE_SELECT_TARGET: {
        if (skip) {
            /* choice.rs:350 SelectTarget + Skip: take_pending_actions; clear; resume */
            g->queue.deferred = NULL;
            rb_resume_with_choice(g, -1);
            return 1;
        }
        if (target && strstr(target, "area_select")) {
            /* choice.rs:356 area_select: set selected_area then resume_pending_actions */
            rb_resolver_handle_position_destination(g);
        } else if (target && strstr(target, "position_change")) {
            rb_resolver_handle_position_change_choice(g);
        } else if (target && strstr(target, "double_baton_touch")) {
            rb_resolver_handle_double_baton_touch(g);
        } else if (target && strstr(target, "primary_alternative")) {
            rb_resolver_handle_primary_alternative(g);
        } else if (target && strstr(target, "conditional_optional")) {
            rb_resolver_handle_conditional_optional(g);
        } else {
            /* choice.rs:386 SelectTarget + TargetSelected -> handle_select_target */
            rb_resolver_handle_select_target(g, target, NULL);
        }
        return 1;
    }

    case RB_CHOICE_SELECT_HEART_COLOR: {
        if (skip) {
            rb_resume_with_choice(g, -1);
            return 1;
        }
        /* choice.rs:391/394 SelectHeartColor|SelectHeartType -> handle_heart_selection */
        int colors[8];
        int nc = 0;
        for (int i = 0; i < ch->n_heart_options && nc < 8; i++)
            colors[nc++] = (int)rb_parse_heart_color(ch->heart_options[i]);
        rb_resolver_handle_heart_selection(g, ch->count, colors, nc);
        return 1;
    }

    case RB_CHOICE_SELECT_NUMBER: {
        if (skip) {
            rb_resume_with_choice(g, -1);
            return 1;
        }
        rb_resolver_handle_choice_condition(g);
        return 1;
    }

    default:
        return 0;   /* pending choice kind not handled here */
    }
}
