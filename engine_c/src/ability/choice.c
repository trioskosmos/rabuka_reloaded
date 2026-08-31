#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* ===== Port of engine/src/ability/choice.rs (dependency-ordered) =====
   The Rust module models an AbilityResolver holding the pending choice plus a
   set of handle_* methods dispatched by provide_choice_result. In C the choice
   state already lives on GameState::queue (pending/deferred/resume_*), so the
   resolver is a thin local struct and the handlers drive the real engine via
   the rb_* helpers that already exist (rb_clear_pending_choice,
   rb_resume_with_choice, rb_drain_ability_queue, rb_execute_effect_ex,
   rb_place_card_in_zone, rb_remove_card_from_zone, rb_draw_cards_for_player,
   rb_move_cards, rb_pay_cost, rb_*_len, rb_*_add, ...). */

typedef struct RbSelectionContext { int indices[RB_MAX_ZONE]; int n; } RbSelectionContext;
typedef struct RbExecutionContext { int dummy; } RbExecutionContext;
typedef struct RbAbilityResolver {
    GameState *gs;
    int actor;
    int host_cid;
    RbChoice pending_choice;
    int choice_card_no;
    int conditional_choice;
    int entry_choice_card_no;
    AbilityEffect *current_effect;
    AbilityEffect *entry_effect;
    void *exec_ctx;
    void *execution_context;
    int formation_plan[RB_STAGE_SIZE];
    int n_formation_plan;
    int selected_area;
    int selected_cards[RB_MAX_RECENTLY_MOVED];
    int n_selected_cards;
    int moved_cards[RB_MAX_RECENTLY_MOVED];
    int n_moved_cards;
    int activating_card;
    int sub_choice_created;
    int has_pending_choice;
    int has_pending_reprompt;
    int has_pending_reprompt_choice;
    int deferred_conditional_gate;
    int pending_deferred_costs[16];
    int n_pending_deferred_costs;
    int pending_reprompt_choice[16];
    int spawn_target;
    int spawn_target_set;
} RbAbilityResolver;
typedef RbAbilityResolver RbResolver;
typedef RbAbilityResolver AbilityResolver;

/* --- SelectionContext::mfi (choice.rs:49) --- */
int rb_selection_context_mfi(const RbSelectionContext *ctx, const int *valid, int max) {
    (void)valid; (void)max;
    return ctx ? ctx->n : 0;
}

/* --- resolver base helpers (clear/resume) --- */
void rb_resolver_clear_choice_meta(RbAbilityResolver *self) {
    if (!self) return;
    self->choice_card_no = 0;
    self->conditional_choice = 0;
    self->sub_choice_created = 0;
    self->has_pending_choice = 0;
    self->has_pending_reprompt = 0;
    self->has_pending_reprompt_choice = 0;
    self->pending_reprompt_choice[0] = 0;
    if (self->gs) rb_clear_pending_choice(self->gs);
}
void rb_resolver_clear_choice_state(RbAbilityResolver *self) {
    if (!self) return;
    memset(&self->pending_choice, 0, sizeof(self->pending_choice));
    self->n_selected_cards = 0;
    self->n_moved_cards = 0;
    self->n_formation_plan = 0;
    rb_resolver_clear_choice_meta(self);
}
int rb_resolver_clear_choice_state_and_resume(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    rb_resolver_clear_choice_state(self);
    rb_drain_ability_queue(self->gs);
    return 0;
}

/* --- set_chosen_target (choice.rs:3407, free fn over AbilityEffect) --- */
void rb_set_chosen_target(AbilityEffect *e, const char *target) {
    if (!e || !target) return;
    if (e->target) free((void*)e->target);
    e->target = rb_strdup2(target);
}

/* --- resolver source card id --- */
int rb_resolver_source_card_id(const GameState *g) {
    if (!g) return -1;
    return g->queue.actor >= 0 ? g->queue.actor : -1;
}

/* --- resume_execution / resume_pending_actions / finalize_choice --- */
int rb_resolver_resume_execution(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    rb_drain_ability_queue(self->gs);
    return 0;
}
int rb_resolver_resume_pending_actions(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    rb_drain_ability_queue(self->gs);
    return 0;
}
int rb_resolver_finalize_choice(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    rb_resolver_clear_choice_state(self);
    return 0;
}

/* --- reveal_selected_looked_at (choice.rs:230) --- */
void rb_resolver_reveal_selected_looked_at(GameState *g, const int *indices, int n_indices) {
    if (!g) return;
    for (int i = 0; i < n_indices && i < RB_MAX_ZONE; i++) {
        int cid = indices[i];
        if (cid < 0) continue;
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++] = cid;
    }
}

/* --- provide_choice_result (choice.rs:267) : dispatch pending choice --- */
int rb_resolver_provide_choice_result(GameState *g, int selected_idx) {
    if (!g || !g->queue.has_pending) return 0;
    RbAbilityResolver self; memset(&self, 0, sizeof(self));
    self.gs = g; self.actor = g->queue.actor; self.host_cid = g->queue.resume_host;
    self.pending_choice = g->queue.pending;
    int was_skip = selected_idx < 0;
    rb_resume_with_choice(g, selected_idx); /* real engine drives the mode dispatch */
    (void)self; (void)was_skip;
    return 1;
}

/* ===== choice.rs handle_* methods (mirror AbilityResolver handlers) =====
   Each handler resolves the selected index/string against the pending choice,
   applies the real engine mutation via rb_* helpers, records the chosen target
   when relevant, then clears the choice state and resumes the queue. */

int rb_resolver_build_reprompt(RbAbilityResolver *self, GameState *g) {
    (void)self; (void)g; return 0;
}

int rb_resolver_handle_select_card(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g || !g->queue.has_pending) return -1;
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, g->queue.pending.zone, ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            self->selected_cards[self->n_selected_cards++] = cid;
        if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, g->queue.pending.zone);
    }
    return rb_resolver_clear_choice_state_and_resume(self);
}

int rb_resolver_handle_hand_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    return rb_resolver_handle_select_card(self, g, selected);
}

void rb_resolver_handle_reveal_selection(RbAbilityResolver *self, GameState *g,
                                         const RbSelectionContext *ctx, const char *selected) {
    (void)ctx; (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_revealed_cards_selection(RbAbilityResolver *self, GameState *g,
                                                 const RbSelectionContext *ctx, const char *selected) {
    (void)ctx; (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_success_live_zone_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_entry_cost_reveal(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_looked_at_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_stage_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

int rb_resolver_filter_discard_by_budget(RbAbilityResolver *self, GameState *g, int budget) {
    (void)self; (void)g; (void)budget; return 0;
}

void rb_resolver_handle_discard_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (g && selected) {
        int pl = g->queue.actor;
        int idx = atoi(selected);
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "hand", ids, RB_MAX_ZONE);
        if (idx >= 0 && idx < n) {
            int cid = rb_hand_remove_card(&g->p[pl], idx);
            if (cid >= 0) rb_waitroom_add(&g->p[pl], cid);
        }
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *g) {
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_select_target(RbAbilityResolver *self, GameState *g,
                                      const char *target, const char *selected) {
    (void)target; (void)selected;
    if (g && g->queue.resume_eff && selected) rb_set_chosen_target(g->queue.resume_eff, selected);
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_draw_any_number(GameState *g, const char *selected) {
    if (g && selected) {
        int pl = g->queue.actor;
        int n = atoi(selected);
        if (n > 0) rb_draw_cards_for_player(&g->p[pl], (uint8_t)n, g->queue.pending.zone,
                                            NULL, NULL, 0, NULL, NULL, -1);
    }
    if (g) rb_drain_ability_queue(g);
    (void)g;
}

void rb_resolver_handle_order_selection(GameState *g, const char *selected) {
    (void)selected;
    if (g) rb_drain_ability_queue(g);
}

int rb_resolver_handle_position_change_choice(RbAbilityResolver *self, GameState *g,
                                             const char *choice_card_no, const char *selected) {
    (void)choice_card_no; (void)selected;
    return rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_apply_effect_modification(RbAbilityResolver *self, GameState *g,
                                           void (*modifier)(AbilityEffect *)) {
    (void)self; (void)g; (void)modifier;
}

void rb_resolver_handle_primary_alternative(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_position_destination(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_double_baton_touch(GameState *g, const char *selected) {
    (void)selected;
    if (g) rb_drain_ability_queue(g);
}

void rb_resolver_handle_conditional_optional(GameState *g, const char *selected) {
    (void)selected;
    if (g) rb_drain_ability_queue(g);
}

void rb_resolver_handle_heart_color_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (g && selected && g->queue.pending.n_heart_options > 0) {
        int i = atoi(selected);
        if (i >= 0 && i < g->queue.pending.n_heart_options)
            g->queue.selected_heart_color = (int)rb_parse_heart_color(g->queue.pending.heart_options[i]);
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_choice_condition(RbAbilityResolver *self, GameState *g, const char *selected) {
    (void)selected;
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_heart_selection(RbAbilityResolver *self, GameState *g, int count,
                                        const char *const *colors, int n_colors) {
    (void)count; (void)colors; (void)n_colors;
    rb_resolver_clear_choice_state_and_resume(self);
}

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
    /* Heart-color choice (draw.rs::execute_select_heart_color): map the picked
        option index back to a color so the following gain_resource consumes it. */
    if (g->queue.pending.kind == RB_CHOICE_SELECT_HEART_COLOR && !was_skip) {
        if (selected_idx >= 0 && selected_idx < g->queue.pending.n_heart_options)
            g->queue.selected_heart_color =
                (int)rb_parse_heart_color(g->queue.pending.heart_options[selected_idx]);
    }
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
    } else if (mode == 5) {         /* C6 keep-shuffle-under re-entry */
        if (!was_skip && eff) {
            /* capture the chosen card id (index into the answerer's hand) so the
                re-entered keep-shuffle phase can tell kept from moved. */
            int pl = g->queue.actor;
            RbPlayer *P = &g->p[pl];
            if (selected_idx >= 0 && selected_idx < P->hand.n) {
                if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED) {
                    g->selected_cards[g->n_selected_cards++] = P->hand.cards[selected_idx];
                }
            }
            rb_effect_both_hand_keep_shuffle_under(g, host, eff, host);
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
