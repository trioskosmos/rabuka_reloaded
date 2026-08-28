#include "rabuka.h"
#include <string.h>

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
}
int rb_resume_with_choice(GameState *g, int selected_idx) {
    if (!g || !g->queue.has_pending) return 0;
    /* selected_idx==-1 means skip (may-pay gate declined). For the portable stub,
       the deferred effect (stashed at emit time) is dropped on skip, executed on
       pick. Optional costs (pay_optional_cost:skip) are paid on pick rather than
       executed as effects (mirrors cost.rs handle_optional_cost_payment). */
    AbilityEffect *def = g->queue.deferred;
    int was_skip = (selected_idx < 0);
    int actor = g->queue.actor;
    rb_clear_pending_choice(g);
    if (!was_skip && def) {
        /* An optional energy/cost gate defers the cost effect; paying it deducts
           the energy. Any other deferred effect is resumed as an effect tree. */
        if (def->action && (!strcmp(def->action, "pay_energy") ||
                            !strcmp(def->action, "pay_cost") ||
                            !strcmp(def->action, "activation_cost")))
            rb_pay_cost(g, actor, def);
        else
            rb_execute_effect(g, actor, def);
    }
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
}
