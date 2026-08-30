#include "rabuka.h"
#include <string.h>
#include <stdio.h>

int rb_has_pending_choice(const GameState *g) { return g ? g->queue.has_pending : 0; }
const RbChoice *rb_get_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return NULL;
    return &g->queue.pending;
}
void rb_clear_pending_choice(GameState *g) {
    if (!g) return;
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.has_pending = 0;
    g->queue.deferred = NULL;
    g->queue.resume_is_select = 0;
}
int rb_resume_with_choice(GameState *g, int selected_idx) {
    if (!g || !g->queue.has_pending) return 0;
    int actor = g->queue.actor;
    int mode = g->queue.resume_mode;
    int is_select = g->queue.resume_is_select;
    AbilityEffect *eff = g->queue.resume_eff;
    int host = g->queue.resume_host;
    /* Capture the deferred effect BEFORE clearing the queue (clearing nulls it). */
    AbilityEffect *def = g->queue.deferred;
    const AbilityEffect *cont = g->queue.resume_parent;
    int cont_from = g->queue.resume_child + 1;
    int was_skip = (selected_idx < 0);
    g->queue.choice_result = selected_idx;   /* record the player's pick (select_number etc.) */
    rb_clear_pending_choice(g);
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
    g->queue.auto_ability = 0;
    g->queue.state = RB_QUEUE_RESOLVING;   /* resuming / draining an ability */
    if (mode == 2) {                 /* select_cards → look.ts keep/drop */
        const char *dest = eff ? eff->destination : NULL;
        rb_look_resume(g, actor, selected_idx, dest, is_select);
    } else if (mode == 1) {          /* position_change destination selection */
        if (!was_skip && eff) {
            g->queue.resume_active = 1;
            g->queue.choice_result = selected_idx;
            rb_resume_position_change(g, actor, eff, host, selected_idx);
            g->queue.resume_active = 0;
        }
    } else if (mode == 3) {          /* auto-ability → execute deferred body */
        if (!was_skip && def) rb_execute_effect_ex(g, actor, def, host);
    } else if (mode == 4) {         /* optional draw gate (draw.rs execute_draw_wrapper) */
        if (!was_skip) {
            int n = 0;
            int t = g->queue.resume_draw_target;
            int self_id = g->queue.resume_draw_self_id;
            if (t == 2) { /* both */
                n += rb_draw_cards_for_player(&g->p[0], (uint8_t)g->queue.resume_draw_count,
                        g->queue.resume_draw_source, g->queue.resume_draw_dest,
                        g->queue.resume_draw_ctype, 0, NULL, NULL, -1);
                n += rb_draw_cards_for_player(&g->p[1], (uint8_t)g->queue.resume_draw_count,
                        g->queue.resume_draw_source, g->queue.resume_draw_dest,
                        g->queue.resume_draw_ctype, 0, NULL, NULL, -1);
            } else {
                n += rb_draw_cards_for_player(&g->p[t], (uint8_t)g->queue.resume_draw_count,
                        g->queue.resume_draw_source, g->queue.resume_draw_dest,
                        g->queue.resume_draw_ctype, 0, NULL, NULL, self_id);
            }
            g->last_draw_count = n;
        }
        /* continue any remaining sibling effects of the parent ability */
        if (cont) {
            for (int j = cont_from; j < cont->n_child; j++) {
                if (rb_has_pending_choice(g)) break;
                rb_execute_effect_ex(g, actor, cont->child[j], host);
            }
        }
    } else {                         /* default: optional-cost / generic deferred */
        if (!was_skip && def) {
            if (def->action && (!strcmp(def->action, "pay_energy") ||
                                !strcmp(def->action, "pay_cost") ||
                                !strcmp(def->action, "activation_cost")))
                rb_pay_cost(g, actor, def);
            else
                rb_execute_effect_ex(g, actor, def, host);
        }
        /* After paying an optional cost, continue the ability's remaining
            sibling effects (e.g. the gain_resource that follows the cost). */
        if (!was_skip && cont) {
            for (int j = cont_from; j < cont->n_child; j++) {
                if (rb_has_pending_choice(g)) break;
                rb_execute_effect_ex(g, actor, cont->child[j], host);
            }
        }
    }
    /* continue resolving any queued trigger/auto abilities */
    rb_drain_ability_queue(g);
    return 1;
}

/* internal: emit a choice that pauses execution. Called from engine.c handle_action. */
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target) {
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.pending.kind = kind;
    if (zone) strncpy(g->queue.pending.zone, zone, sizeof(g->queue.pending.zone)-1);
    if (card_type) strncpy(g->queue.pending.card_type, card_type, sizeof(g->queue.pending.card_type)-1);
    g->queue.pending.count = count > 0 ? count : 1;
    g->queue.pending.allow_skip = allow_skip;
    if (target) strncpy(g->queue.pending.target, target, sizeof(g->queue.pending.target)-1);
    g->queue.has_pending = 1;
    g->queue.actor = actor;
    g->queue.deferred = NULL;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;   /* QueueState FSM (ability_queue.rs) */
}
