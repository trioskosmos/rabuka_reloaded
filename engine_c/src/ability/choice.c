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
    /* Mirror choice.rs::finalize_choice: preserve initial looked-at if sequential pending,
       clear sub_choice, pay deferred, resume, and set continuation. */
    int is_looked_at = (self->pending_choice.zone == RB_ZONEID_LOOKED_AT);
    int has_pending_sequential = rb_queue_has_pending(&self->gs->queue);
    int is_initial_looked_at = is_looked_at && has_pending_sequential;
    int should_preserve = is_initial_looked_at && has_pending_sequential;
    int sub_choice = self->sub_choice_created;
    self->sub_choice_created = 0;
    /* Pay deferred costs via queue drain (stubbed here as clear+resume for parity) */
    if (!should_preserve && !sub_choice) {
        rb_clear_pending_choice(self->gs);
    }
    rb_resolver_resume_execution(self);
    int has_pending = rb_queue_has_pending(&self->gs->queue);
    int was_select_card = (self->pending_choice.zone == RB_ZONEID_SELECTED_CARDS); /* simplified */
    if (sub_choice) {
        /* immediate handled above */
    } else if (has_pending && was_select_card) {
        rb_clear_pending_choice(self->gs);
        rb_resolver_resume_pending_actions(self);
    } else if (has_pending) {
        rb_resolver_resume_pending_actions(self);
    } else {
        if (!sub_choice) rb_resolver_resume_pending_actions(self);
    }
    if (!sub_choice && !self->pending_choice.zone) {
        RbQueueEntry *e = &self->gs->queue.entries[self->gs->queue.cur];
        if (e && !e->cost_paid && !e->effect_started) e->cost_paid = 1;
    }
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

/* Move `cid` to `dst` for a choice handler — waitroom/discard use the waitroom
    bag, every other zone uses the generic zone placement helper. */
static void rb_choice_send_to_dst(GameState *g, int pl, int cid, const char *dst) {
    if (!dst || !strcmp(dst, "waitroom") || !strcmp(dst, "discard"))
        rb_waitroom_add(&g->p[pl], cid);
    else
        rb_place_card_in_zone(g, pl, cid, dst, -1);
}

int rb_resolver_build_reprompt(RbAbilityResolver *self, GameState *g) {
    (void)g;
    if (!self) return 0;
    /* Mirror choice.rs::build_reprompt — re-issue the pending select_cards
        prompt (used for "select N more" loops and reprompt-on-budget). */
    self->has_pending_reprompt = 1;
    self->has_pending_reprompt_choice = 1;
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.pending.kind = RB_CHOICE_SELECT_CARD;
    g->queue.pending.count = 1;
    g->queue.pending.allow_skip = 1;
    g->queue.has_pending = 1;
    g->queue.actor = self->actor;
    return 0;
}

/* Mirror Rust GameState::entry_cost() — the cost AbilityEffect of the ability
    currently being resolved (queue.entries[cur]). Copies the cost action into
    out_act (e.g. "discard"/"reveal"/"pay_energy") and returns 1 if a cost exists. */
static int rb_queue_current_cost_action(GameState *g, char *out_act, int outlen) {
    if (out_act && outlen > 0) out_act[0] = '\0';
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= RB_QUEUE_DEPTH) return 0;
    RbQueueEntry *e = &g->queue.entries[cur];
    if (e->card_id < 0) return 0;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)e->card_id, e->ability_idx, &ab)) return 0;
    int has = (ab.cost != NULL);
    if (has && out_act && outlen > 0) {
        const char *a = ab.cost->action ? ab.cost->action : "";
        strncpy(out_act, a, outlen - 1); out_act[outlen - 1] = '\0';
    }
    rb_free_ability(&ab);
    return has;
}

int rb_resolver_handle_select_card(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g || !g->queue.has_pending) return -1;
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int was_skip = (idx < 0);
    const char *zone = g->queue.pending.zone[0] ? g->queue.pending.zone : "hand";
    const char *target = g->queue.pending.target[0] ? g->queue.pending.target : NULL;
    int allow_skip = g->queue.pending.allow_skip;

    int cur = g->queue.cur;
    int effect_started = (cur >= 0 && cur < RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
    char cost_act[48]; cost_act[0] = '\0';
    int has_cost = rb_queue_current_cost_action(g, cost_act, sizeof(cost_act));
    int is_reveal = (target && strstr(target, "reveal")) || (zone && strstr(zone, "reveal"));
    int is_cost_reveal = (!effect_started && has_cost && !strcmp(cost_act, "reveal"));

    /* Consume the deferred そうした場合 gate (mirrors Rust): a skipped answer drops
        the remaining actions of the current entry. */
    if (self->deferred_conditional_gate) {
        self->deferred_conditional_gate = 0;
        if (was_skip && cur >= 0) {
            /* C has no per-entry pending_actions list; the gate is consumed and the
                ability proceeds (Rust clears entry.pending_actions on skip). */
        }
    }

    /* Reveal selection: push the chosen cards to the revealed set; do not move. */
    if (is_reveal || is_cost_reveal) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
                    g->revealed_cards[g->n_revealed++] = cid;
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = cid;
                if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, target ? target : "reveal");
            }
        }
        return rb_resolver_clear_choice_state_and_resume(self);
    }

    /* Hand-cost payment (Rust Rule 9.4.2.3): the chosen hand cards are the cost;
        move them to the waitroom (discard) and record the cost result + movement
        so downstream cost-condition gates and auto-triggers see it. */
    if (!effect_started && has_cost && !strcmp(zone, "hand")) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, "hand", ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                rb_choice_send_to_dst(g, actor, cid, "waitroom");
                if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                    self->moved_cards[self->n_moved_cards++] = cid;
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = cid;
                if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
                    g->recently_moved[g->n_recently_moved++] = cid;
            }
            if (cur >= 0) g->queue.entries[cur].optional_cost_result = 1;
            g->mods.last_cost_discard_count = self->n_moved_cards;
            g->mods.n_last_cost_moved_card_ids = self->n_moved_cards;
            for (int i = 0; i < self->n_moved_cards && i < RB_MAX_RECENTLY_MOVED; i++)
                g->mods.last_cost_moved_card_ids[i] = self->moved_cards[i];
        } else {
            if (cur >= 0) g->queue.entries[cur].optional_cost_result = 0;
        }
        /* C emits one card per choice; a single pick satisfies the gate. */
        return rb_resolver_clear_choice_state_and_resume(self);
    }

    /* Plain select: record the chosen card as the target. The deferred effect
        (re-executed by the resume path) applies the actual zone move. */
    if (!was_skip) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
        if (idx >= 0 && idx < n) {
            int cid = ids[idx];
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = cid;
            if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, zone);
        }
    }
    (void)allow_skip;
    return rb_resolver_clear_choice_state_and_resume(self);
}

int rb_resolver_handle_hand_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    return rb_resolver_handle_select_card(self, g, selected);
}

void rb_resolver_handle_reveal_selection(RbAbilityResolver *self, GameState *g,
                                         const RbSelectionContext *ctx, const char *selected) {
    (void)ctx;
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, g->queue.pending.zone[0] ? g->queue.pending.zone : "hand",
                          ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_revealed_cards_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected) {
    (void)ctx;
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    if (idx >= 0 && idx < g->n_revealed) {
        int cid = g->revealed_cards[idx];
        for (int i = idx; i < g->n_revealed - 1; i++) g->revealed_cards[i] = g->revealed_cards[i + 1];
        g->n_revealed--;
        const char *dst = (g->queue.resume_eff && g->queue.resume_eff->destination)
                              ? g->queue.resume_eff->destination : "waitroom";
        rb_choice_send_to_dst(g, pl, cid, dst);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_success_live_zone_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "success", ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        const char *dst = (g->queue.resume_eff && g->queue.resume_eff->destination)
                              ? g->queue.resume_eff->destination : "waitroom";
        rb_move_card(g, pl, cid, "success", dst, -1);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_entry_cost_reveal(RbAbilityResolver *self, GameState *g, const char *selected) {
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "hand", ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_looked_at_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_looked_at_pool(pl, ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++] = cid;
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            self->selected_cards[self->n_selected_cards++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_stage_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "stage", ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        const char *dst = (g->queue.resume_eff && g->queue.resume_eff->destination)
                              ? g->queue.resume_eff->destination : "waitroom";
        rb_move_card(g, pl, cid, "stage", dst, idx);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

int rb_resolver_filter_discard_by_budget(RbAbilityResolver *self, GameState *g, int budget) {
    (void)budget;
    if (!self || !g) return 0;
    /* Mirror choice.rs::filter_discard_by_budget — return the indices of the
        waitroom cards that still fit the remaining cost budget. The C card model
        does not expose per-card printed cost cheaply, so we return the full
        waitroom set (the caller still enforces the budget at pay time). */
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, g->queue.actor, "waitroom", ids, RB_MAX_ZONE);
    int count = 0;
    for (int i = 0; i < n && count < (int)(sizeof(self->pending_deferred_costs)/sizeof(self->pending_deferred_costs[0])); i++) {
        self->pending_deferred_costs[count++] = ids[i];
    }
    self->n_pending_deferred_costs = count;
    return count;
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
    /* Mirror choice.rs::handle_selection_epilogue — the selection sequence is
        complete; resolve any stored target then resume the queue. */
    if (g && g->queue.resume_eff && self->spawn_target_set)
        rb_set_chosen_target(g->queue.resume_eff, g->queue.pending.target[0] ? g->queue.pending.target : "self");
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
    (void)choice_card_no;
    if (self && g) {
        /* Record the chosen stage area for the position change so the
            downstream rb_resume_position_change applies it. */
        self->selected_area = selected ? atoi(selected) : -1;
        if (self->n_formation_plan < RB_STAGE_SIZE)
            self->formation_plan[self->n_formation_plan++] = self->selected_area;
        g->queue.resume_child = self->selected_area;
    }
    return rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_apply_effect_modification(RbAbilityResolver *self, GameState *g,
                                           void (*modifier)(AbilityEffect *)) {
    (void)self; (void)g; (void)modifier;
}

void rb_resolver_handle_primary_alternative(RbAbilityResolver *self, GameState *g, const char *selected) {
    /* Mirror choice.rs::handle_primary_alternative — `selected` picks the
        primary effect (0) or the alternative effect (1); record it as the
        conditional branch the resolver will execute. */
    if (self) {
        int pick = selected ? atoi(selected) : 0;
        self->conditional_choice = (pick != 0) ? 1 : 0;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_position_destination(RbAbilityResolver *self, GameState *g, const char *selected) {
    /* Mirror choice.rs::handle_position_destination — apply the chosen stage
        area as the position-change destination through the real resolver. */
    if (g) {
        int idx = selected ? atoi(selected) : -1;
        rb_resume_position_change(g, g->queue.actor, g->queue.resume_eff,
                                   g->queue.resume_host, idx);
    }
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
    /* Mirror choice.rs::handle_choice_condition — the chosen branch (0 = the
        condition was false / skip, 1 = true) drives the conditional effect. */
    if (self) {
        int pick = selected ? atoi(selected) : 0;
        self->conditional_choice = (pick != 0) ? 1 : 0;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_heart_selection(RbAbilityResolver *self, GameState *g, int count,
                                         const char *const *colors, int n_colors) {
    (void)self;
    /* Mirror choice.rs::handle_heart_selection — apply up to `count` chosen
        heart colors. The C queue tracks a single selected color, so we apply the
        first chosen color (multi-color gains are handled at effect resolution). */
    if (g) {
        int apply = (count > 0 && count <= n_colors) ? count : n_colors;
        for (int i = 0; i < apply; i++) {
            if (colors[i]) {
                g->queue.selected_heart_color = (int)rb_parse_heart_color(colors[i]);
                break;
            }
        }
    }
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
/* Continue any remaining sibling effects of the parent ability after a choice
    resolves (mirrors Rust's parent-effect child continuation in provide_choice_result). */
static void rb_resolver_continue_siblings(GameState *g, int actor, int host,
                                          const AbilityEffect *cont, int cont_from) {
    if (!cont) return;
    for (int j = cont_from; j < cont->n_child; j++) {
        if (rb_has_pending_choice(g)) break;
        rb_execute_effect_ex(g, actor, cont->child[j], host);
    }
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
    int kind = g->queue.pending.kind;   /* captured BEFORE rb_clear_pending_choice */
    RbAbilityResolver self; memset(&self, 0, sizeof(self));
    self.gs = g; self.actor = actor; self.host_cid = host;
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
    } else {                         /* generic: route by choice kind to the
                                         resolver handlers (mirrors Rust
                                         provide_choice_result's match). Each
                                         handler applies the selection's mutation,
                                         then clears state and resumes the queue. */
        char selbuf[32];
        const char *selected = NULL;
        if (!was_skip) { snprintf(selbuf, sizeof(selbuf), "%d", selected_idx); selected = selbuf; }
        int is_cost = (def && def->action &&
                       (!strcmp(def->action, "pay_energy") ||
                        !strcmp(def->action, "pay_cost") ||
                        !strcmp(def->action, "activation_cost") ||
                        !strcmp(def->action, "pay_any_cost")));
        /* "discard your entire hand" optional cost (mirrors Rust
            handle_pay_cost_all_discard routing). */
        if (g->queue.pending.target && strstr(g->queue.pending.target, "pay_cost_all_discard")) {
            rb_handle_pay_cost_all_discard(g, actor, selected);
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            return 1;
        }
        switch (kind) {
        case RB_CHOICE_SELECT_CARD: {
            /* Faithful handle_select_card dispatch: reveal and hand-cost selections
                are applied directly by the handler (which moves cards / records the
                cost result). Plain selects still re-run the deferred effect, because
                in the C engine the deferred effect performs the actual zone move
                based on the chosen index — the engine models selection via
                deferred re-execution rather than the handler doing the move. */
            const char *pzone = g->queue.pending.zone[0] ? g->queue.pending.zone : "hand";
            const char *ptarget = g->queue.pending.target[0] ? g->queue.pending.target : NULL;
            int cur = g->queue.cur;
            int eff_started = (cur >= 0 && cur < RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
            char ca[48]; ca[0] = '\0';
            int hc = rb_queue_current_cost_action(g, ca, sizeof(ca));
            int rev = (ptarget && strstr(ptarget, "reveal")) || (pzone && strstr(pzone, "reveal"));
            int cost_hand = (!eff_started && hc && !strcmp(pzone, "hand"));
            rb_resolver_handle_select_card(&self, g, selected);
            if (!was_skip && def && !rev && !cost_hand) {
                if (is_cost) rb_pay_cost(g, actor, def);
                else         rb_execute_effect_ex(g, actor, def, host);
            }
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        }
        case RB_CHOICE_SELECT_TARGET:
            /* record the chosen target via the handler, then run the deferred
                effect that consumes it (C models target selection via deferral). */
            rb_resolver_handle_select_target(&self, g, NULL, selected);
            if (!was_skip && def) {
                if (def->action && (!strcmp(def->action, "pay_energy") ||
                                    !strcmp(def->action, "pay_cost") ||
                                    !strcmp(def->action, "activation_cost") ||
                                    !strcmp(def->action, "pay_any_cost")))
                    rb_pay_cost(g, actor, def);
                else
                    rb_execute_effect_ex(g, actor, def, host);
            }
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        case RB_CHOICE_SELECT_HEART_COLOR:
            /* record the picked heart color via the handler, then run the deferred
                gain_resource that consumes it. */
            rb_resolver_handle_heart_color_selection(&self, g, selected);
            if (!was_skip && def) {
                if (def->action && (!strcmp(def->action, "pay_energy") ||
                                    !strcmp(def->action, "pay_cost") ||
                                    !strcmp(def->action, "activation_cost") ||
                                    !strcmp(def->action, "pay_any_cost")))
                    rb_pay_cost(g, actor, def);
                else
                    rb_execute_effect_ex(g, actor, def, host);
            }
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        case RB_CHOICE_SELECT_NUMBER:
            rb_resolver_handle_draw_any_number(g, selected);
            break;
        default:
            if (!was_skip && def) {
                if (is_cost) rb_pay_cost(g, actor, def);
                else         rb_execute_effect_ex(g, actor, def, host);
            }
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        }
    }
    /* continue resolving any queued trigger/auto abilities. The mode switch above
        set state = RB_QUEUE_RESOLVING, which makes rb_drain_ability_queue a no-op
        (re-entrancy guard). Normalize the state so this top-level drain actually
        runs: if a choice is still pending we must yield to the host; otherwise we
        drop back to IDLE and drain the queued auto/trigger abilities now
        (mirrors Rust provide_choice_result → resume_execution → drain). */
    if (rb_has_pending_choice(g)) {
        g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    } else {
        g->queue.state = RB_QUEUE_IDLE;
        rb_drain_ability_queue(g);
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
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;   /* QueueState FSM (ability_queue.rs) */
}
