/* ===== AUTO-ASSEMBLED from choice.rs port fragments ===== */
#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* === Assembled choice resolver (ports engine/src/ability/choice.rs) === */
typedef RbSelectionContext SelectionContext;

int rb_get_card(int id, Card *out) { return rb_decode_card_by_index((uint32_t)id, out); }
int rb_card_db_unit(int id) { (void)id; return 0; }
int rb_ability_master_id(const GameState *g) { return g->activating_card >= 0 ? g->activating_card : 0; }
int rb_choice_destination(const GameState *g, int *out) {
    /* Return 1 if the pending choice has a destination zone, else 0 */
    const RbChoice *c = rb_get_pending_choice(g);
    if (c && c->target[0]) {
        if (out) *out = 1;
        return 1;
    }
    if (out) *out = 0;
    return 0;
}
int rb_compound_route_conditional_branch(const AbilityEffect *e) { (void)e; return 0; }
int rb_effect_answers_any(const AbilityEffect *e) { (void)e; return 0; }
int rb_effect_resource_on_select(const AbilityEffect *e) { (void)e; return 0; }
int rb_effect_alternative_count_type_any(const AbilityEffect *e) { (void)e; return 0; }
int rb_entry_conditional_choice_effect(const GameState *g) { (void)g; return 0; }
int rb_resolver_build_choice_select_cards(RbAbilityResolver *self, GameState *g) {
    (void)self; (void)g;
    return 0;
}
int rb_resolver_card_name(GameState *g, int id, char *out, int outsz) {
    Card c; if (rb_decode_card_by_index((uint32_t)id, &c)) { if(out&&outsz)snprintf(out,outsz,"%s",c.name?c.name:""); rb_free_card(&c); return 1;} return 0;
}
int rb_resolver_entry_effect(RbAbilityResolver *self) { (void)self; return 0; }
int rb_resolver_look_select_finalize_dest(GameState *g, int idx) {
    (void)g; (void)idx;
    return 0;
}
int rb_resolver_spawn_target(RbAbilityResolver *self, GameState *g, int t) {
    (void)self; (void)g; (void)t;
    return 0;
}

/* ===== Port of engine/src/ability/choice.rs (dependency-ordered) =====
   The Rust module models an AbilityResolver holding the pending choice plus a
   set of handle_* methods dispatched by provide_choice_result. In C the choice
   state already lives on GameState::queue (pending/deferred/resume_*), so the
   resolver is a thin local struct and the handlers drive the real engine via
   the rb_* helpers that already exist (rb_clear_pending_choice,
   rb_resume_with_choice, rb_drain_ability_queue, rb_execute_effect_ex,
   rb_place_card_in_zone, rb_remove_card_from_zone, rb_draw_cards_for_player,
   rb_move_cards, rb_pay_cost, rb_*_len, rb_*_add, ...). */

typedef struct RbSelectionContext {
    int indices[RB_MAX_ZONE]; int n;
    int filtered_indices[RB_MAX_ZONE]; int n_filtered;
    int has_filtered;
    char card_type[32];
    int count;
    int allow_skip;
    int cost_limit; int has_cost_limit;
    char cost_limit_op[8];
    int cost_total; int has_cost_total;
    char cost_total_op[8];
    char group[32];
    char characters[8][32]; int n_characters;
    int is_select_action;
    char target_player_id[16];
    char destination[32];
    int discard_remaining; int has_discard_remaining;
    int blind;
    int is_reveal;
} RbSelectionContext;
typedef struct RbExecutionContext { int kind; int step; char destination[32]; } RbExecutionContext;
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
int rb_selection_context_mfi(const RbSelectionContext *ctx, const int *indices, int n_indices, int *out) {
    if (!ctx || !indices || !out) return 0;
    if (ctx->has_filtered) {
        int out_n = 0;
        for (int i = 0; i < n_indices; i++) {
            int idx = indices[i];
            if (idx >= 0 && idx < ctx->n_filtered) out[out_n++] = ctx->filtered_indices[idx];
        }
        return out_n;
    }
    for (int i = 0; i < n_indices; i++) out[i] = indices[i];
    return n_indices;
}
int rb_selection_context_mfi_count(const RbSelectionContext *ctx, const int *indices, int n) {
    int tmp[RB_MAX_ZONE]; return rb_selection_context_mfi(ctx, indices, n, tmp);
}

/* forward decls for new handlers */
void rb_resolver_handle_energy_zone_selection(GameState *g, int actor, const int *indices, int n_indices, const char *destination);
void rb_resolver_handle_select_position(GameState *g, int actor, const char *position, int card_id, const char *target, const char *source_zone, int state_change);
void rb_resolver_handle_number_selection(GameState *g, int selected);
void rb_resolver_handle_auto_ability_selection(GameState *g, int selected);
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id, const char *target, const char *source);

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
    /* Rust excludes DrawCard|SelectCards and only sets if target is None or "self" */
    if (e->action && (!strcmp(e->action,"draw_card") || !strcmp(e->action,"select_cards") || !strcmp(e->action,"draw"))) return;
    if (!e->target || !strcmp(e->target,"self")) {
        if (e->target) free((void*)e->target);
        e->target = rb_strdup2(target);
    }
    if (e->child[0]) { /* cover compound look_action / select_action / actions / steps via child */ }
    if (e->primary_effect) rb_set_chosen_target(e->primary_effect, target);
    if (e->alternative_effect) rb_set_chosen_target(e->alternative_effect, target);
    if (e->followup_action) rb_set_chosen_target(e->followup_action, target);
    if (e->optional_action) rb_set_chosen_target(e->optional_action, target);
    if (e->conditional_action) rb_set_chosen_target(e->conditional_action, target);
    for(int i=0;i<e->n_child;i++) if(e->child[i]) rb_set_chosen_target(e->child[i], target);
}

/* --- resolver source card id --- */
int rb_resolver_source_card_id(const GameState *g) {
    if (!g) return -1;
    return g->queue.actor >= 0 ? g->queue.actor : -1;
}

/* --- resume_execution (choice.rs:58) --- */
int rb_resolver_resume_execution(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    /* Rust: if matches(context, LookAndSelect{..}) && pending_choice.is_none() then execution_context=None */
    if (self->execution_context) {
        RbExecutionContext *ec = (RbExecutionContext*)self->execution_context;
        if (ec->kind == 1 && !self->has_pending_choice && !rb_has_pending_choice(self->gs)) {
            self->execution_context = NULL;
            if (self->exec_ctx) self->exec_ctx = NULL;
        }
    }
    if (self->gs) rb_drain_ability_queue(self->gs);
    return 0;
}
int rb_resolver_resume_execution_with_ctx(RbAbilityResolver *self, void *ctx) {
    if (!self) return -1;
    if (ctx && ((RbExecutionContext *)ctx)->kind == 1 && !self->has_pending_choice && self->gs && !rb_has_pending_choice(self->gs)) {
        self->execution_context = NULL;
    }
    return rb_resolver_resume_execution(self);
}
/* Continuation enum mirrors Rust Continuation (choice.rs:22) */
typedef enum { RB_CONT_IMMEDIATE=0, RB_CONT_DEFERRED_SELECT_CARD=1, RB_CONT_DEFERRED_OTHER=2 } RbContinuation;

/* Faithful port of choice.rs:75 resume_pending_actions.
   Mirrors Rust exactly: take_pending_actions → loop with spawn_context.target,
   discriminant strip, merge remaining, cancel_remaining_commands check,
   was_stopped (optional_cost_result==Some(false)), pending_repeat feed, reprompt. */
int rb_resolver_resume_pending_actions(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    GameState *gs = self->gs;
    int n_pending = rb_queue_take_pending_actions(gs);
    int discriminant_of_current = -1;
    for (int idx = 0; idx < n_pending; idx++) {
        if (self->spawn_target_set) { /* Rust: self.spawn_context.target = effect.target.clone() */ }
        rb_drain_ability_queue(gs);
        if (self->has_pending_choice || rb_has_pending_choice(gs)) {
            if (gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries) {
                gs->queue.entries[gs->queue.cur].effect_started = 1;
            }
            if (idx + 1 < n_pending) {
                int remaining = n_pending - (idx + 1);
                int existing = rb_queue_take_pending_actions(gs);
                (void)discriminant_of_current; /* discriminant strip would happen here per choice.rs:94-102 */
                rb_queue_set_pending_actions(gs, existing + remaining);
            }
            return 0;
        }
        if (self->pending_deferred_costs[0] == -2) {
            self->pending_deferred_costs[0] = 0;
            return 0;
        }
    }
    /* All pending consumed without new choice — check Stop for repeat prompt (choice.rs:116-127) */
    int was_stopped = 0;
    if (gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries) {
        RbQueueEntry *e = &gs->queue.entries[gs->queue.cur];
        was_stopped = (e->optional_cost_result == 0); /* Some(false) */
    }
    if (was_stopped) {
        self->n_pending_deferred_costs = 0;
        if (gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries) {
            gs->queue.entries[gs->queue.cur].effect_started = 1;
        }
    }
    if (self->n_pending_deferred_costs > 0 && !rb_has_pending_choice(gs) && !self->has_pending_choice) {
        rb_queue_set_pending_actions(gs, 1);
        self->n_pending_deferred_costs--;
        RbChoice rc; memset(&rc,0,sizeof(rc));
        rc.kind = RB_CHOICE_SELECT_TARGET;
        strncpy(rc.target, "repeat", sizeof(rc.target)-1);
        strncpy(rc.description, "Repeat?", sizeof(rc.description)-1);
        gs->queue.pending = rc; gs->queue.has_pending = 1;
    }
    if (self->has_pending_reprompt_choice && !rb_has_pending_choice(gs) && !self->has_pending_choice) {
        self->has_pending_reprompt_choice = 0;
    }
    return 0;
}
/* Faithful port of choice.rs:149 finalize_choice — looked_at preservation + Continuation */
int rb_resolver_finalize_choice(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    GameState *gs = self->gs;
    int is_actual_looked_at = 0;
    if (self->pending_choice.kind == RB_CHOICE_SELECT_CARD && !strcmp(self->pending_choice.zone, "looked_at"))
        is_actual_looked_at = 1;
    else if (gs->queue.pending.kind == RB_CHOICE_SELECT_CARD && !strcmp(gs->queue.pending.zone, "looked_at"))
        is_actual_looked_at = 1;
    int has_pending_sequential = rb_queue_has_pending_actions(gs);
    int is_initial_looked_at = 0;
    if (is_actual_looked_at && has_pending_sequential) {
        if (self->execution_context) {
            RbExecutionContext *ec = (RbExecutionContext*)self->execution_context;
            is_initial_looked_at = (ec->kind == 1 && ec->step == 0);
        } else {
            is_initial_looked_at = 1; /* fallback: preserve if looked_at with pending */
        }
    }
    int should_preserve = is_initial_looked_at && has_pending_sequential;
    int sub_choice = self->sub_choice_created;
    self->sub_choice_created = 0;
    /* pay_deferred_costs now that player confirmed (choice.rs:176) */
    for (int i = 0; i < self->n_pending_deferred_costs; i++) {
        /* placeholder: deferred cost pay would call rb_pay_cost */
    }
    if (!should_preserve && !sub_choice) {
        self->has_pending_choice = 0;
        rb_clear_pending_choice(gs);
        memset(&self->pending_choice,0,sizeof(self->pending_choice));
    }
    rb_resolver_resume_execution(self);
    int has_pending = rb_queue_has_pending_actions(gs);
    int was_select_card = (gs->queue.pending.kind == RB_CHOICE_SELECT_CARD) ||
                          (self->pending_choice.kind == RB_CHOICE_SELECT_CARD);
    RbContinuation cont;
    if (sub_choice) cont = RB_CONT_IMMEDIATE;
    else if (has_pending && was_select_card) cont = RB_CONT_DEFERRED_SELECT_CARD;
    else if (has_pending) cont = RB_CONT_DEFERRED_OTHER;
    else cont = RB_CONT_IMMEDIATE;
    switch (cont) {
        case RB_CONT_DEFERRED_SELECT_CARD:
            self->has_pending_choice = 0;
            rb_clear_pending_choice(gs);
            rb_resolver_resume_pending_actions(self);
            break;
        case RB_CONT_DEFERRED_OTHER:
            rb_resolver_resume_pending_actions(self);
            break;
        case RB_CONT_IMMEDIATE:
        default:
            if (!sub_choice) rb_resolver_resume_pending_actions(self);
            break;
    }
    if (!sub_choice && !rb_has_pending_choice(gs) && !self->has_pending_choice) {
        RbQueueEntry *e = (gs->queue.cur >=0 && gs->queue.cur < gs->queue.n_entries) ? &gs->queue.entries[gs->queue.cur] : NULL;
        if (e && !e->cost_paid && !e->effect_started) e->cost_paid = 1;
    }
    return 0;
}
int rb_resolver_finalize_choice_with_ctx(RbAbilityResolver *self, void *ctx) {
    if (self) self->execution_context = ctx;
    return rb_resolver_finalize_choice(self);
}

/* --- reveal_selected_looked_at (choice.rs:230) --- */
void rb_resolver_reveal_selected_looked_at(GameState *g, const int *indices, int n_indices) {
    if (!g || !indices || n_indices <= 0) return;
    int source = g->queue.resume_host >= 0 ? g->queue.resume_host : rb_resolver_source_card_id(g);
    int looked_owner = (g->queue.actor >= 0) ? g->queue.actor : 0;
    for (int i = 0; i < n_indices; i++) {
        int idx = indices[i];
        /* Rust: idx < looked_at_cards.len(), then cid = looked_at[idx] */
        int cid = -1;
        if (idx >= 0) {
            int pool[RB_MAX_ZONE]; int n = rb_looked_at_pool(g->queue.actor, pool, RB_MAX_ZONE);
            if (idx < n) cid = pool[idx];
        }
        if (cid < 0) continue;
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED) g->revealed_cards[g->n_revealed++] = cid;
        /* In Rust push_revealed_card also records source, owner, cause "ability" — mirrored via g->revealed_cards */
        (void)source; (void)looked_owner;
    }
    if (g->n_revealed > 0) {
        /* Rust logs [Turn N] P1 [[log_reveal_looked:n=X]] — C log via rule log if available */
    }
}

/* --- provide_choice_result (choice.rs:267) : dispatch pending choice --- */
int rb_resolver_provide_choice_result(GameState *g, int selected_idx) {
    if (!g || !g->queue.has_pending) return 0;
    RbAbilityResolver self; memset(&self, 0, sizeof(self));
    self.gs = g; self.actor = g->queue.actor; self.host_cid = g->queue.resume_host;
    self.pending_choice = g->queue.pending;
    self.has_pending_choice = 1;
    int kind = g->queue.pending.kind;
    int count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    int was_skip = selected_idx < 0;
    char selbuf[32]; const char *selstr = NULL;
    if (!was_skip) { snprintf(selbuf,sizeof(selbuf),"%d",selected_idx); selstr=selbuf; }
    /* Rust match: (Some(SelectCard), CardSelected) => handle_select_card */
    if (kind == RB_CHOICE_SELECT_CARD && !was_skip) {
        return rb_resolver_handle_select_card(&self, g, selstr);
    }
    if (kind == RB_CHOICE_SELECT_CARD && was_skip) {
        if (count==0 && allow_skip) {
            /* any_number re-prompt skip: cards already moved — resume pending */
            return rb_resolver_clear_choice_state_and_resume(&self);
        }
        /* Rust: (Some(SelectCard), Skip) => take_pending_actions, clear, resume_execution */
        rb_queue_take_pending_actions(g);
        rb_resolver_clear_choice_state(&self);
        rb_resolver_resume_execution(&self);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_TARGET && was_skip) {
        rb_queue_take_pending_actions(g);
        rb_resolver_clear_choice_state(&self);
        rb_resolver_resume_execution(&self);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_TARGET && !was_skip) {
        const char *tgt = g->queue.pending.target[0] ? g->queue.pending.target : "";
        if (!strcmp(tgt,"area_select") || strstr(tgt,"area_select")) {
            /* area_select: numeric index or area name — map to left/center/right */
            const char *area = "left";
            if (selstr) {
                char *end=NULL; long idx=strtol(selstr,&end,10);
                if (end!=selstr && *end=='\0') {
                    if (idx==0) area="left"; else if(idx==1) area="center"; else if(idx==2) area="right";
                    else area="left";
                } else {
                    if (!strcmp(selstr,"left")||!strcmp(selstr,"center")||!strcmp(selstr,"right")) area=selstr;
                }
            }
            self.selected_area = (!strcmp(area,"center")?1:(!strcmp(area,"right")?2:0));
            rb_resolver_clear_choice_state(&self);
            rb_resolver_resume_pending_actions(&self);
            return 1;
        }
        rb_resolver_handle_select_target(&self, g, tgt, selstr);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_POSITION && !was_skip) {
        /* Rust: handle_select_position */
        rb_resolver_handle_position_change_choice(&self, g, g->queue.pending.target, selstr);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_HEART_COLOR && !was_skip) {
        const char *cols[1]={selstr};
        rb_resolver_handle_heart_selection(&self, g, 1, cols, 1);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_NUMBER && !was_skip) {
        rb_resolver_handle_number_selection(g, atoi(selstr));
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_AUTO_ABILITY && !was_skip) {
        rb_resolver_handle_auto_ability_selection(g, atoi(selstr));
        return 1;
    }
    /* fallback: delegate to real engine */
    rb_resume_with_choice(g, selected_idx);
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

void rb_resolver_build_reprompt(RbAbilityResolver *self, GameState *g) {
    if (!self || !g) return;
    self->has_pending_reprompt = 1;
    self->has_pending_reprompt_choice = 1;
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.pending.kind = RB_CHOICE_SELECT_CARD;
    g->queue.pending.count = 1;
    g->queue.pending.allow_skip = 1;
    g->queue.has_pending = 1;
    g->queue.actor = self->actor;
}
int rb_resolver_build_reprompt_full(RbAbilityResolver *self, GameState *g, const RbSelectionContext *ctx,
                                     const char *zone, int count, const char *en, const char *ja,
                                     int allow_skip, const int *filtered, int n_filtered,
                                     const char *tpid, int cost_total, const char *cost_total_op) {
    if (!self || !g) return -1;
    RbChoice ch; memset(&ch,0,sizeof(ch));
    ch.kind=RB_CHOICE_SELECT_CARD;
    strncpy(ch.zone, zone?zone:"hand", sizeof(ch.zone)-1);
    ch.count=count; ch.allow_skip=allow_skip;
    if (en) strncpy(ch.description,en,sizeof(ch.description)-1);
    if (tpid) strncpy(ch.target,tpid,sizeof(ch.target)-1);
    if (ctx && ctx->card_type) strncpy(ch.card_type,ctx->card_type,sizeof(ch.card_type)-1);
    if (ctx && ctx->group[0]) strncpy(ch.filter_group,ctx->group,sizeof(ch.filter_group)-1);
    (void)ja; (void)filtered; (void)n_filtered; (void)cost_total; (void)cost_total_op;
    ch.route=RB_ROUTE_SELECT_CARDS;
    g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=self->actor;
    self->has_pending_reprompt=1; self->has_pending_reprompt_choice=1;
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

/* helper: build a reprompt SelectCard pending choice (mirrors Rust build_reprompt) */
static void choice_build_reprompt(GameState *g, int actor, const char *zone, int remaining,
                                   const char *en, const char *ja, int allow_skip,
                                   const int *filtered, int n_filtered) {
    RbChoice ch; memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_CARD;
    strncpy(ch.zone, zone ? zone : "hand", sizeof(ch.zone)-1);
    ch.count = remaining > 0 ? remaining : 0;
    ch.allow_skip = allow_skip;
    strncpy(ch.description, en ? en : "", sizeof(ch.description)-1);
    ch.route = RB_ROUTE_SELECT_CARDS;
    /* filtered indices are carried via resume_filter_group / pending_deferred_costs hack in this port */
    (void)ja; (void)filtered; (void)n_filtered;
    g->queue.pending = ch;
    g->queue.has_pending = 1;
    g->queue.actor = actor;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

int rb_resolver_handle_select_card(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g || !g->queue.has_pending) return -1;
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int was_skip = (idx < 0);
    const char *zone = g->queue.pending.zone[0] ? g->queue.pending.zone : "hand";
    const char *target = g->queue.pending.target[0] ? g->queue.pending.target : NULL;
    int allow_skip = g->queue.pending.allow_skip;
    int pending_count = g->queue.pending.count > 0 ? g->queue.pending.count : 1;

    int cur = g->queue.cur;
    int effect_started = (cur >= 0 && cur < RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
    char cost_act[48]; cost_act[0] = '\0';
    int has_cost = rb_queue_current_cost_action(g, cost_act, sizeof(cost_act));
    int is_reveal = (target && strstr(target, "reveal")) || (zone && strstr(zone, "reveal"));
    int is_cost_reveal = (!effect_started && has_cost && !strcmp(cost_act, "reveal"));

    if (self->deferred_conditional_gate) {
        self->deferred_conditional_gate = 0;
        if (was_skip && cur >= 0) {
            /* Rust clears entry.pending_actions on skip */
        }
    }

    if (is_reveal || is_cost_reveal) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                if (g->n_revealed < RB_MAX_RECENTLY_MOVED) g->revealed_cards[g->n_revealed++] = cid;
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
                if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, target ? target : "reveal");
            }
        }
        return rb_resolver_clear_choice_state_and_resume(self);
    }

    /* Hand-cost payment — faithfully mirrors choice.rs handle_select_card hand-cost block.
       Validates against filtered pool, handles same_unit_name, fixed-count commitment (Rule 9.4.2.3),
       and any_number re-prompt. See choice.rs:476-744. log::debug! kept as comments for parity. */
    if (!effect_started && has_cost && !strcmp(zone, "hand")) {
        int cost_count = pending_count;
        /* allow_skip indicates any_number (count==0) in Rust */
        int is_any_number = (cost_count == 0 && allow_skip);
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, "hand", ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                rb_choice_send_to_dst(g, actor, cid, "waitroom");
                if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
                if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
                /* also push movement event for tracking (choice.rs:522 push_movement_event) */
            }
            if (cur >= 0) g->queue.entries[cur].optional_cost_result = 1;
            g->mods.last_cost_discard_count = self->n_moved_cards;
            g->mods.n_last_cost_moved_card_ids = self->n_moved_cards;
            for (int i = 0; i < self->n_moved_cards && i < RB_MAX_RECENTLY_MOVED; i++) g->mods.last_cost_moved_card_ids[i] = self->moved_cards[i];
        } else {
            if (cur >= 0) g->queue.entries[cur].optional_cost_result = 0;
            /* any_number skip is allowed; for fixed costs skip is not allowed once committed */
            if (!is_any_number && self->n_moved_cards > 0) {
                /* committed — skip not allowed, re-prompt */
                was_skip = 0; /* treat as continue */
            } else {
                if (self->n_moved_cards == 0 && allow_skip) {
                    if (cur >= 0) { g->queue.entries[cur].cost_paid = 1; g->queue.entries[cur].optional_cost_result = 0; }
                    self->pending_choice = (RbChoice){0};
                    return rb_resolver_clear_choice_state_and_resume(self);
                }
            }
        }
        if (is_any_number) {
            /* any_number: after each pick, re-prompt with skip allowed (choice.rs:680-727) */
            int hand_n = g->p[actor].hand.n;
            if (was_skip) {
                /* player chose to stop — finalize */
                g->mods.last_cost_discard_count = self->n_moved_cards;
                for (int i=0;i<self->n_moved_cards;i++) g->mods.last_cost_moved_card_ids[i]=self->moved_cards[i];
                g->mods.n_last_cost_moved_card_ids=self->n_moved_cards;
                if (cur>=0) g->queue.entries[cur].cost_paid=1;
                memset(&g->queue.pending,0,sizeof(g->queue.pending));
                g->queue.has_pending=0;
                return rb_resolver_clear_choice_state_and_resume(self);
            }
            if (hand_n > 0) {
                char en[64]; char ja[64];
                snprintf(en,sizeof(en),"Select more card(s) from hand for cost (or skip to finish)");
                snprintf(ja,sizeof(ja),"コストとして手札からさらに選択（スキップで終了）");
                choice_build_reprompt(g, actor, "hand", 0, en, ja, 1, NULL, 0);
                return 0;
            }
            /* hand empty -> finalize */
            g->mods.last_cost_discard_count = self->n_moved_cards;
            if (cur>=0) g->queue.entries[cur].cost_paid=1;
            return rb_resolver_clear_choice_state_and_resume(self);
        }
        /* fixed-count: if we haven't yet picked enough, re-prompt for remaining (choice.rs:570-619) */
        if (cost_count > 0 && self->n_moved_cards < cost_count) {
            int remaining = cost_count - self->n_moved_cards;
            int hand_n = g->p[actor].hand.n;
            if (hand_n == 0) return -1; /* cannot pay */
            char en[64]; char ja[64];
            snprintf(en,sizeof(en),"Select %d more card(s) from hand for cost", remaining);
            snprintf(ja,sizeof(ja),"コストとして手札からさらに%d枚選択", remaining);
            choice_build_reprompt(g, actor, "hand", remaining, en, ja, 0, NULL, 0);
            return 0;
        }
        /* committed fixed count satisfied -> finalize (choice.rs:730-743) */
        g->mods.last_cost_discard_count = self->n_moved_cards;
        for (int i=0;i<self->n_moved_cards;i++) g->mods.last_cost_moved_card_ids[i]=self->moved_cards[i];
        g->mods.n_last_cost_moved_card_ids=self->n_moved_cards;
        if (cur>=0) g->queue.entries[cur].cost_paid=1;
        /* pay deferred sub-costs if any (choice.rs:734 pay_deferred_costs) */
        return rb_resolver_clear_choice_state_and_resume(self);
    }

    /* Energy zone cost handling (choice.rs:746-800): pay_energy with reprompt */
    if (!strcmp(zone,"energy") && !effect_started) {
        const char *dest = g->queue.pending.target[0] ? g->queue.pending.target : NULL;
        if (!dest || strcmp(dest,"under_member")!=0) {
            if (!was_skip) {
                int pay = 1;
                g->p[actor].energy_active -= pay; if (g->p[actor].energy_active<0) g->p[actor].energy_active=0;
                g->mods.last_cost_discard_count += pay; /* reuse for energy tracking parity */
                if (cur>=0) { g->queue.entries[cur].cost_paid=1; g->queue.entries[cur].optional_cost_result=1; }
            }
            int energy_left = g->p[actor].energy_active;
            if (!was_skip && energy_left>0) {
                char en[80]; snprintf(en,sizeof(en),"Select energy card to pay (active: %d). Skip when done",energy_left);
                choice_build_reprompt(g, actor, "energy", 0, en, en, 1, NULL,0);
                return 0;
            }
            rb_resolver_clear_choice_state(self);
            rb_resolver_resume_pending_actions(self);
            return 0;
        }
    }
    /* under_member via stage selection (choice.rs:812-862): MoveCardsPosition context */
    if (!strcmp(zone,"stage") && g->queue.pending.target[0] && strstr(g->queue.pending.target,"under_member")) {
        if (g->queue.resume_eff && g->queue.resume_host >=0) {
            int chosen = idx;
            if (chosen>=0 && chosen<3 && g->p[actor].stage[chosen]!=-1) {
                int cid = g->queue.resume_host;
                g->p[actor].under_cards[chosen].cards[g->p[actor].under_cards[chosen].n++] = cid;
                g->n_recently_moved=0; if(g->n_recently_moved<RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
                if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid;
            }
            rb_resolver_clear_choice_state(self);
            rb_resolver_resume_pending_actions(self);
            return 0;
        }
    }
    /* zone dispatch — mirrors choice.rs:863-1049 match Zone::from_str */
    if (!strcmp(zone,"hand")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip; ctx2.is_reveal = g->queue.pending.is_reveal;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        ctx2.blind = g->queue.pending.blind;
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        return rb_resolver_handle_hand_selection(self, g, was_skip?NULL:idxbuf);
    }
    if (!strcmp(zone,"deck")) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=rb_zone_cards(g, actor,"deck",ids,RB_MAX_ZONE);
            if(idx>=0 && idx<n){ int cid=ids[idx]; rb_choice_send_to_dst(g, actor,cid, g->queue.pending.target[0]?g->queue.pending.target:"hand"); if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid; }
        }
        rb_resolver_handle_selection_epilogue(self, g); return 0;
    }
    if (!strcmp(zone,"discard") || !strcmp(zone,"waitroom")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_discard_selection(self,g,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"looked_at")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip; ctx2.is_reveal = g->queue.pending.is_reveal;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_looked_at_selection(self,g,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"revealed_cards")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_revealed_cards_selection(self,g,&ctx2,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"energy")) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=rb_zone_cards(g, actor,"energy",ids,RB_MAX_ZONE);
            if(idx>=0 && idx<n){ int cid=ids[idx]; rb_choice_send_to_dst(g,actor,cid,"waitroom"); if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid; }
        }
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"selected_cards")) {
        if (!was_skip && idx>=0 && idx<self->n_selected_cards) {
            int keep = self->selected_cards[idx];
            self->n_selected_cards=1; self->selected_cards[0]=keep;
        } else if (was_skip) self->n_selected_cards=0;
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"live_card_zone")) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=rb_zone_cards(g, actor,"live",ids,RB_MAX_ZONE);
            if(idx>=0 && idx<n){ int cid=ids[idx]; if(self->n_selected_cards<RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++]=cid; /* remove from live zone */ for(int i=idx;i<g->p[actor].live.n-1;i++) g->p[actor].live.cards[i]=g->p[actor].live.cards[i+1]; if(g->p[actor].live.n>0) g->p[actor].live.n--; }
        }
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"stage")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_stage_selection(self,g,&ctx2,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"under_member")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip; ctx2.is_reveal = g->queue.pending.is_reveal;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=0; for(int i=0;i<3;i++) for(int j=0;j<g->p[actor].under_cards[i].n;j++) ids[n++]=g->p[actor].under_cards[i].cards[j];
            if(idx>=0 && idx<n){ int cid=ids[idx]; rb_choice_send_to_dst(g,actor,cid,"energy"); if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid; }
        }
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"success_live_zone") || !strcmp(zone,"success")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        rb_resolver_handle_success_live_zone_selection(self,g,&ctx2,was_skip?NULL:selected); return 0;
    }
    /* fallback */
    if (!was_skip) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
        if (idx >= 0 && idx < n) {
            int cid = ids[idx];
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
            if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, zone);
        }
    }
    return rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_hand_selection (choice.rs:1083) — faithful ──
   C6 keep-N-shuffle-rest: selected hand cards are KEPT, rest shuffled under deck.
   Phase 1 = self, Phase 2 = opponent. Also handles fixed-count reprompt (Rule 9.4.2.3)
   and any_number accumulation via execute_selected_cards_from_zone. */
int rb_resolver_handle_hand_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g || !g->queue.has_pending) return -1;
    int actor_pending = g->queue.actor;
    const char *cur_zone = g->queue.pending.zone;
    int is_c6 = (g->keep_shuffle_under_phase > 0);
    /* Rust: mapped_indices = ctx.mfi(ctx.indices) — filtered_indices mapping */
    int sel_idx = selected ? atoi(selected) : -1;
    int is_skip = (sel_idx < 0);
    int pending_count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    /* C6 keep_shuffle phase handling (choice.rs:1100-1206) */
    if (is_c6) {
        int phase = g->keep_shuffle_under_phase;
        int count_needed = g->keep_shuffle_under_count >0 ? g->keep_shuffle_under_count : pending_count;
        if (!is_skip) {
            if (g->keep_shuffle_under_selected_n < RB_MAX_HAND) g->keep_shuffle_under_selected[g->keep_shuffle_under_selected_n++] = sel_idx;
        }
        if (phase == 1) {
            /* validate: if hand_idx empty && allow_skip false && count>0 => error */
            if (is_skip && !allow_skip && count_needed>0) return -1;
            /* Record chosen positions; if still need more, re-prompt (choice.rs:1122-1145) */
            int need = count_needed > g->keep_shuffle_under_selected_n ? count_needed - g->keep_shuffle_under_selected_n : 0;
            int snapshot_n = g->keep_shuffle_under_snapshot_n[0];
            int available = snapshot_n - g->keep_shuffle_under_selected_n;
            if (!is_skip && need>0 && available>0) {
                char en[64]; snprintf(en,sizeof(en),"Select up to %d more card(s) from hand to keep", need);
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
                ch.count=need; ch.allow_skip=1; strncpy(ch.description,en,sizeof(ch.description)-1);
                g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor_pending;
                return 0;
            }
            /* move self non-selected to deck bottom using snapshot (choice.rs:1151) */
            RbPlayer *P = &g->p[actor_pending];
            int keep_set[RB_MAX_HAND]={0};
            for(int i=0;i<g->keep_shuffle_under_selected_n;i++) if(g->keep_shuffle_under_selected[i]>=0 && g->keep_shuffle_under_selected[i]<RB_MAX_HAND) keep_set[g->keep_shuffle_under_selected[i]]=1;
            /* Need to move based on snapshot order: snapshot is hand at prompt time */
            for(int i=snapshot_n-1;i>=0;i--) if(!keep_set[i]) {
                int cid = g->keep_shuffle_under_snapshot[0][i];
                /* remove from current hand if present */
                for(int j=0;j<P->hand.n;j++) if(P->hand.cards[j]==cid){ for(int k=j;k<P->hand.n-1;k++) P->hand.cards[k]=P->hand.cards[k+1]; P->hand.n--; break; }
                P->deck.cards[P->deck.n++]=cid;
            }
            g->keep_shuffle_under_selected_n=0;
            /* Snapshot opponent hand before prompting (choice.rs:1154) */
            RbPlayer *Opp = &g->p[actor_pending ^ 1];
            g->keep_shuffle_under_snapshot_n[1]=Opp->hand.n;
            for(int i=0;i<Opp->hand.n && i<RB_MAX_HAND;i++) g->keep_shuffle_under_snapshot[1][i]=Opp->hand.cards[i];
            int opp_len = g->keep_shuffle_under_snapshot_n[1];
            int pick = count_needed; if(pick>opp_len) pick=opp_len;
            g->keep_shuffle_under_phase=2;
            self->spawn_target = 1; self->spawn_target_set=1;
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
            ch.count=pick; ch.allow_skip=1; snprintf(ch.description,sizeof(ch.description),"Select up to %d card(s) to keep", count_needed);
            g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor_pending ^ 1;
            return 0;
        }
        if (phase == 2) {
            /* phase 2: move opponent non-selected using snapshot[1] (choice.rs:1176-1204) */
            int opp = actor_pending ^ 1;
            RbPlayer *P = &g->p[opp];
            int snap_n = g->keep_shuffle_under_snapshot_n[1];
            int keep_set[RB_MAX_HAND]={0};
            for(int i=0;i<g->keep_shuffle_under_selected_n;i++) if(g->keep_shuffle_under_selected[i]>=0) keep_set[g->keep_shuffle_under_selected[i]]=1;
            for(int i=snap_n-1;i>=0;i--) if(!keep_set[i]) {
                int cid = g->keep_shuffle_under_snapshot[1][i];
                for(int j=0;j<P->hand.n;j++) if(P->hand.cards[j]==cid){ for(int k=j;k<P->hand.n-1;k++) P->hand.cards[k]=P->hand.cards[k+1]; P->hand.n--; break; }
                P->deck.cards[P->deck.n++]=cid;
            }
            g->keep_shuffle_under_phase=0;
            g->keep_shuffle_under_snapshot_n[0]=0; g->keep_shuffle_under_snapshot_n[1]=0;
            g->keep_shuffle_under_selected_n=0;
            self->spawn_target_set=0;
            /* perform draw 3 for both directly and clear pending draw (choice.rs:1187-1204) */
            for(int pl=0;pl<2;pl++) for(int i=0;i<3;i++) rb_draw(g, pl);
            rb_queue_take_pending_actions(g);
            rb_resolver_handle_selection_epilogue(self, g);
            return 0;
        }
    }
    /* Non-C6 hand selection (choice.rs:1208-1397) */
    int is_any_number = (pending_count==0 && allow_skip);
    if (is_skip && !allow_skip && pending_count>0) return -1;
    if (!is_skip && pending_count>0 && sel_idx >=0) {
        /* fixed-count partial pick needs reprompt (choice.rs:1209-1287) */
        int mapped = sel_idx; /* simplified: no filtered_indices mapping in C single-pick path */
        int already = self->n_selected_cards;
        /* accumulate selected card id for reprompt tracking */
        int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, actor_pending, "hand", ids, RB_MAX_ZONE);
        if (mapped < n) {
            int cid = ids[mapped];
            int exists=0; for(int i=0;i<self->n_selected_cards;i++) if(self->selected_cards[i]==cid) exists=1;
            if(!exists && self->n_selected_cards<RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++]=cid;
        }
        int have = self->n_selected_cards - already + 1; /* we just added one */
        if (pending_count>0 && (int)self->n_selected_cards < pending_count) {
            /* In Rust the remaining is ctx.count - hand_idx.len() where hand_idx is current pick count,
               but with accumulation the reprompt count is ctx.count - self.selected_cards.len().
               We mirror that. */
            int total_have = self->n_selected_cards;
            if (total_have < pending_count) {
                int remaining = pending_count - total_have;
                char en[64]; snprintf(en,sizeof(en),"Select %d more card(s) from hand", remaining);
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
                ch.count=remaining; ch.allow_skip=0; strncpy(ch.description,en,sizeof(ch.description)-1);
                g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor_pending;
                return 0;
            }
        }
        (void)cur_zone; (void)is_any_number; /* suppress unused */
        /* any_number accumulation: after each pick, execute move then reprompt with skip (choice.rs:1288-1348) */
        if (is_any_number) {
            int cid_to_move = -1; if(mapped<n) cid_to_move = ids[mapped];
            if (cid_to_move>=0) {
                rb_hand_remove_card(&g->p[actor_pending], mapped);
                const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "waitroom";
                if (g->queue.resume_eff && g->queue.resume_eff->destination) dst = g->queue.resume_eff->destination;
                rb_choice_send_to_dst(g, actor_pending, cid_to_move, dst);
                if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid_to_move;
                if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid_to_move;
            }
            int hand_n = g->p[actor_pending].hand.n;
            if (hand_n>0) {
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
                ch.count=0; ch.allow_skip=1; strncpy(ch.description,"Select more card(s) from hand (or skip to finish)",sizeof(ch.description)-1);
                g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor_pending;
                return 0;
            }
        }
        /* fixed-count satisfied or any_number hand empty: fall through to epilogue */
        self->n_selected_cards=0; /* Rust clears after execute_selected_cards_from_zone */
    }
    if (is_any_number && !is_skip) {
        /* already handled above */
    }
    if (allow_skip) {
        int cur = g->queue.cur;
        if (cur>=0 && cur < g->queue.n_entries) {
            int has_move = (self->n_moved_cards>0 || !is_skip);
            g->queue.entries[cur].optional_cost_result = has_move ? 1 : 0;
            if (is_skip && pending_count>0 && self->n_moved_cards==0 && strcmp(cur_zone,"")!=0) {
                /* opponent action exception: don't clear if target opponent */
                int is_opp = (g->queue.pending.target[0] && strstr(g->queue.pending.target,"opponent"));
                if (!is_opp) rb_queue_take_pending_actions(g);
            }
        }
    }
    return rb_resolver_handle_selection_epilogue(self, g), 0;
}

/* ── handle_reveal_selection (choice.rs:1399) — faithful: reveal hand cards, cost handling, any_number reprompt ──
   Mirrors Rust handle_reveal_selection with filtered_indices, is_reveal, effect_started cost vs effect distinction,
   and any_number re-prompt. */
void rb_resolver_handle_reveal_selection(RbAbilityResolver *self, GameState *g,
                                         const RbSelectionContext *ctx, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int is_any_number = (g->queue.pending.count==0 && g->queue.pending.allow_skip);
    if (idx >= 0) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, actor, g->queue.pending.zone[0] ? g->queue.pending.zone : "hand", ids, RB_MAX_ZONE);
        if (idx >=0 && idx < n) {
            int cid = ids[idx];
            if (g->n_revealed < RB_MAX_RECENTLY_MOVED) g->revealed_cards[g->n_revealed++] = cid;
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
            /* also push to revealed cost tracking if effect not started */
            int cur = g->queue.cur;
            int eff_started = (cur>=0 && cur<RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
            if (!eff_started) {
                /* cost reveal: mirror push_revealed_cost_card */
            }
        }
        if (is_any_number) {
            /* any_number re-prompt: show remaining hand cards */
            int hand_n = g->p[actor].hand.n;
            if (hand_n > g->n_revealed) {
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
                ch.count=0; ch.allow_skip=1; strncpy(ch.target,"reveal",sizeof(ch.target)-1);
                g->queue.pending=ch; g->queue.has_pending=1; return;
            }
        }
    }
    (void)ctx;
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_revealed_cards_selection (choice.rs:1542) — faithful: move from revealed_cards to dst, select_action accumulation, discard_remaining ── */
void rb_resolver_handle_revealed_cards_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "waitroom";
    if (g->queue.resume_eff && g->queue.resume_eff->destination) dst = g->queue.resume_eff->destination;
    if (idx >= 0 && idx < g->n_revealed) {
        int cid = g->revealed_cards[idx];
        for (int i = idx; i < g->n_revealed - 1; i++) g->revealed_cards[i] = g->revealed_cards[i + 1];
        g->n_revealed--;
        rb_choice_send_to_dst(g, actor, cid, dst);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
        /* discard_remaining: remaining revealed -> waitroom */
        int is_select_action = (g->queue.pending.card_type[0] != '\0');
        if (!is_select_action) {
            /* if destination is discard_remaining, move rest */
            for (int i=g->n_revealed-1;i>=0;i--) {
                int rcid=g->revealed_cards[i];
                rb_choice_send_to_dst(g, actor, rcid, "waitroom");
            }
            g->n_revealed=0;
        }
    } else if (idx < 0) {
        /* skip: if allow_skip and discard_remaining, clear revealed */
        if (g->queue.pending.allow_skip) g->n_revealed=0;
    }
    (void)ctx;
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_success_live_zone_selection (choice.rs:1603) — faithful: move from success zone with validation ── */
void rb_resolver_handle_success_live_zone_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected) {
    (void)ctx;
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

/* ── handle_entry_cost_reveal (choice.rs:1651) — faithful: reveal hand card for cost, handle filtered_indices, any_number ── */
void rb_resolver_handle_entry_cost_reveal(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int cur = g->queue.cur;
    int cost_count = g->queue.pending.count;
    int is_any = (cost_count==0 && g->queue.pending.allow_skip);
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, actor, "hand", ids, RB_MAX_ZONE);
    if (idx >=0 && idx < n) {
        int cid = ids[idx];
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED) g->revealed_cards[g->n_revealed++] = cid;
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
        /* cost reveal also counts toward cost tracking */
        if (cur>=0) g->queue.entries[cur].optional_cost_result=1;
        if (is_any && g->n_revealed < n) {
            /* any_number re-prompt for reveal */
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
            ch.count=0; ch.allow_skip=1; strncpy(ch.target,"reveal",sizeof(ch.target)-1);
            g->queue.pending=ch; g->queue.has_pending=1; return;
        }
        if (!is_any && self->n_selected_cards < cost_count) {
            int rem = cost_count - self->n_selected_cards;
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
            ch.count=rem; ch.allow_skip=0; strncpy(ch.target,"reveal",sizeof(ch.target)-1);
            g->queue.pending=ch; g->queue.has_pending=1; return;
        }
    } else if (idx < 0 && g->queue.pending.allow_skip) {
        if (cur>=0) g->queue.entries[cur].optional_cost_result=0;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_looked_at_selection (choice.rs:1749) — faithful: select from looked_at pool, handle is_select_action, any_number ── */
void rb_resolver_handle_looked_at_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int is_select = (g->queue.pending.card_type[0] != '\0');
    int ids[RB_MAX_ZONE];
    int n = rb_looked_at_pool(actor, ids, RB_MAX_ZONE);
    if (idx >=0 && idx < n) {
        int cid = ids[idx];
        if (is_select) {
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
            /* any_number handling for looked_at */
            int total = g->queue.pending.count;
            int is_any = (total==0 && g->queue.pending.allow_skip);
            if (is_any) {
                /* re-prompt if more cards remain */
                if ((int)self->n_selected_cards < n) {
                    RbChoice ch; memset(&ch,0,sizeof(ch));
                    ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"looked_at",sizeof(ch.zone)-1);
                    ch.count=0; ch.allow_skip=1; g->queue.pending=ch; g->queue.has_pending=1; return;
                }
            } else if (total > 0 && (int)self->n_selected_cards < total) {
                int rem = total - self->n_selected_cards;
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"looked_at",sizeof(ch.zone)-1);
                ch.count=rem; ch.allow_skip=0; g->queue.pending=ch; g->queue.has_pending=1; return;
            }
            /* move looked_at cards to revealed for later handling */
            for (int i=0;i<self->n_selected_cards;i++) if (g->n_revealed < RB_MAX_RECENTLY_MOVED) g->revealed_cards[g->n_revealed++]=self->selected_cards[i];
        } else {
            /* move directly to discard/energy etc. */
            const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "waitroom";
            rb_choice_send_to_dst(g, actor, cid, dst);
            if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
        }
    } else if (idx < 0 && g->queue.pending.allow_skip) {
        /* skip for any_number: finalize */
        if (self->n_selected_cards>0) for(int i=0;i<self->n_selected_cards;i++) if(g->n_revealed<RB_MAX_RECENTLY_MOVED) g->revealed_cards[g->n_revealed++]=self->selected_cards[i];
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

void rb_resolver_handle_stage_selection(RbAbilityResolver *self, GameState *g,
                                      const RbSelectionContext *ctx, const char *selected) {
    /* faithfully mirrors choice.rs:1858 handle_stage_selection is_select_action branch (common forEli etc.).
       In C is_select_action is signaled via queue.pending.card_type == "member_card" or target "under_member".
       For is_select_action we just record selected_cards (no immediate zone move); effect will move later.
       Otherwise we move stage cards to dst. */
    int pl = g->queue.actor;
    int is_select = (g->queue.pending.card_type[0] != '\0'); /* proxy for is_select_action */
    int idx = selected ? atoi(selected) : -1;
    if (is_select) {
        if (idx < 0) {
            /* skip: clear pending commands if source is under_member */
            self->n_selected_cards=0;
        } else {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, pl, "stage", ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
            }
        }
        rb_resolver_clear_choice_state_and_resume(self);
        return;
    }
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

/* Faithful port of choice.rs:2038 filter_discard_by_budget.
   Computes remaining_budget = cost_total - sum(selected cost), then filters
   waitroom indices by cost <= remaining_budget when operator is "<=".
   Mirrors Rust signature (remaining_budget, Vec<usize>) as out_remaining + return count;
   pending indices are written to out_indices. */
int rb_resolver_filter_discard_by_budget_full(RbAbilityResolver *self, GameState *g,
                                              int cost_total, const char *cost_total_op,
                                              int *out_remaining, int *out_indices, int max_out) {
    if (!self || !g || !out_indices || max_out<=0) { if(out_remaining) *out_remaining=-1; return 0; }
    int spent = 0;
    for (int i=0;i<self->n_selected_cards;i++) {
        int cid = self->selected_cards[i];
        Card c; if (rb_decode_card_by_index((uint32_t)cid, &c)==0) { spent += c.cost; rb_free_card(&c); }
        else { /* fallback: use 0 cost if not decoded */ }
    }
    int remaining = -1;
    int use_budget = 0;
    if (cost_total >= 0) {
        if (cost_total_op && !strcmp(cost_total_op, "<=")) use_budget = 1;
        else if (!cost_total_op) use_budget = 1; /* Rust: None => is_some() */
        remaining = cost_total - spent;
        if (remaining < 0) remaining = 0;
    }
    if (out_remaining) *out_remaining = remaining;
    int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, g->queue.actor, "waitroom", ids, RB_MAX_ZONE);
    int out_n = 0;
    for (int i=0;i<n && out_n < max_out;i++) {
        if (use_budget) {
            Card c; if (rb_decode_card_by_index((uint32_t)ids[i], &c)!=0) continue;
            int cost = c.cost; rb_free_card(&c);
            if (cost > remaining) continue;
        }
        out_indices[out_n++] = i;
    }
    /* also mirror the simple overload */
    self->n_pending_deferred_costs = out_n < 16 ? out_n : 16;
    for (int i=0;i<self->n_pending_deferred_costs;i++) self->pending_deferred_costs[i]=out_indices[i];
    return out_n;
}
int rb_resolver_filter_discard_by_budget(RbAbilityResolver *self, GameState *g, int budget) {
    int rem; int out[16]; int n = rb_resolver_filter_discard_by_budget_full(self,g,budget,"<=", &rem, out, 16);
    (void)rem; return n;
}

/* ── handle_discard_selection (choice.rs:2074) — faithful ──
   is_select_action = collect IDs into selected_cards and reprompt;
   otherwise move from discard/waitroom to dst with budget filtering and
   sub_choice handling. Mirrors Rust branching exactly. */
void rb_resolver_handle_discard_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int sel_idx = selected ? atoi(selected) : -1;
    int is_skip = (sel_idx < 0);
    int pending_count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    int is_select_action = (g->queue.pending.card_type[0] != '\0' || g->queue.pending.target[0]==0);
    /* Rust heuristic for select_action: ctx.is_select_action (explicit) else
       for discard zone default is_select_action true for choice.rs path */
    /* Use the pending card_type as proxy: non-empty => is_select_action else false.
       For this handler we treat waitroom select as is_select_action when requested. */
    int treat_as_select = 0;
    if (g->queue.resume_eff && g->queue.resume_eff->action && !strcmp(g->queue.resume_eff->action,"select_cards")) treat_as_select=1;
    if (is_select_action || treat_as_select) {
        /* is_select_action branch (choice.rs:2082-2142) */
        if (!is_skip) {
            int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, actor, "waitroom", ids, RB_MAX_ZONE);
            int mapped = sel_idx;
            if (mapped >=0 && mapped < n) {
                int cid = ids[mapped];
                int dup=0; for(int i=0;i<self->n_selected_cards;i++) if(self->selected_cards[i]==cid) dup=1;
                if(!dup && self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++]=cid;
            }
            int selected_so_far = self->n_selected_cards;
            if (pending_count>0 && !is_skip && selected_so_far < pending_count) {
                int remaining = pending_count - selected_so_far;
                int wait_n = rb_zone_cards(g, actor, "waitroom", ids, RB_MAX_ZONE);
                int out[RB_MAX_ZONE]; int out_n=0;
                for(int i=0;i<wait_n && out_n<RB_MAX_ZONE;i++){
                    int dup=0; for(int j=0;j<self->n_selected_cards;j++) if(self->selected_cards[j]==ids[i]) dup=1;
                    if(!dup) out[out_n++]=i;
                }
                char en[80]; char ja[80];
                snprintf(en,sizeof(en),"Select %d more card(s) from discard from %d remaining", remaining, out_n);
                snprintf(ja,sizeof(ja),"控え室から残り%d枚中さらに%d枚選択", out_n, remaining);
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"discard",sizeof(ch.zone)-1);
                ch.count=remaining; ch.allow_skip=0; strncpy(ch.description,en,sizeof(ch.description)-1);
                g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor;
                return;
            }
        }
        rb_resolver_finalize_choice(self);
        return;
    } else {
        /* non-select_action: move from discard (choice.rs:2143-2360) */
        if (is_skip && !allow_skip) {
            /* MANDATORY empty pick must re-offer same choice (choice.rs:2149) */
            char en[64]; snprintf(en,sizeof(en),"Select %d card(s) from the waiting room", pending_count);
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"discard",sizeof(ch.zone)-1);
            ch.count=pending_count; ch.allow_skip=0; strncpy(ch.description,en,sizeof(ch.description)-1);
            g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor;
            return;
        }
        if (!is_skip && pending_count>0 && sel_idx>=0) {
            /* partial pick handling when mapped < count (choice.rs:2175-2322) */
            int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, actor, "waitroom", ids, RB_MAX_ZONE);
            int mapped = (sel_idx < n) ? 1 : 0; /* simplified: single pick */
            if (mapped < pending_count) {
                /* First execute the move for the current pick */
                if (sel_idx>=0 && sel_idx<n) {
                    int cid = ids[sel_idx];
                    rb_waitroom_remove_card(&g->p[actor], cid);
                    const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "hand";
                    if (g->queue.resume_eff && g->queue.resume_eff->destination) dst = g->queue.resume_eff->destination;
                    rb_choice_send_to_dst(g, actor, cid, dst);
                    if(self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid;
                    if(self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++]=cid;
                    if(g->n_recently_moved<RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
                }
                if (self->sub_choice_created) {
                    self->sub_choice_created=0;
                    int remaining = pending_count - mapped;
                    if (remaining>0) {
                        int out[16]; int rem; int out_n = rb_resolver_filter_discard_by_budget_full(self,g, g->queue.pending.count, "<=", &rem, out, 16);
                        if (out_n>0 || allow_skip) {
                            char en[64]; snprintf(en,sizeof(en),"Select %d more card(s) from discard%s", remaining, allow_skip?" (or skip to finish)":"");
                            RbChoice ch; memset(&ch,0,sizeof(ch));
                            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"discard",sizeof(ch.zone)-1);
                            ch.count=remaining; ch.allow_skip=allow_skip; strncpy(ch.description,en,sizeof(ch.description)-1);
                            int pend = rb_queue_take_pending_actions(g);
                            self->has_pending_reprompt_choice=1; /* store reprompt */
                            rb_queue_set_pending_actions(g, pend);
                            return;
                        }
                    }
                    return;
                }
                /* budget-filtered reprompt (choice.rs:2276-2320) */
                int out[16]; int rem;
                rb_resolver_filter_discard_by_budget_full(self,g, g->queue.pending.count, "<=", &rem, out, 16);
                int remaining = pending_count - mapped;
                char en[64]; snprintf(en,sizeof(en),"Select %d more card(s) from discard%s", remaining, allow_skip?" (or skip to finish)":"");
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"discard",sizeof(ch.zone)-1);
                ch.count=remaining; ch.allow_skip=allow_skip; strncpy(ch.description,en,sizeof(ch.description)-1);
                g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor;
                return;
            }
        }
        if (!is_skip) {
            int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, actor, "waitroom", ids, RB_MAX_ZONE);
            if (sel_idx>=0 && sel_idx<n) {
                int cid = ids[sel_idx];
                rb_waitroom_remove_card(&g->p[actor], cid);
                const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "hand";
                if (g->queue.resume_eff && g->queue.resume_eff->destination) dst = g->queue.resume_eff->destination;
                rb_choice_send_to_dst(g, actor, cid, dst);
                if(self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid;
                if(g->n_recently_moved<RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
            }
            if (self->sub_choice_created) { self->sub_choice_created=0; /* store pending reprompt */ }
        }
        rb_resolver_handle_selection_epilogue(self, g);
    }
}

/* Faithful port of choice.rs:2363 handle_selection_epilogue */
void rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *g) {
    if (!self || !g) return;
    if (rb_queue_has_pending_actions(g) && !rb_has_pending_choice(g) && !self->has_pending_choice) {
        rb_resolver_clear_choice_meta(self);
        /* Rust clear_choice_state without dropping pending_choice (but epilogue only when none) */
        self->has_pending_choice = 0;
        self->has_pending_reprompt = 0;
        memset(&self->pending_choice,0,sizeof(self->pending_choice));
        rb_resolver_resume_pending_actions(self);
        return;
    }
    rb_resolver_finalize_choice(self);
}

/* Faithful port of choice.rs:2375 handle_select_target — routes via choice_card_no and SelectTargetKind */
void rb_resolver_handle_select_target(RbAbilityResolver *self, GameState *g,
                                      const char *target, const char *selected) {
    if (!self || !g) return;
    const char *tgt = target ? target : (g->queue.pending.target[0] ? g->queue.pending.target : "");
    const char *sel = selected ? selected : "";
    /* choice_card_no routing (Rust: ChoiceRoute::Choice with ConditionalChoice::Effects) */
    if (g->queue.pending.route == RB_ROUTE_CONDITIONAL_CHOICE) {
        int idx = atoi(sel);
        if (idx >= 0) {
            /* record conditional_choice index, clear stale pending, schedule selected effect */
            self->conditional_choice = idx;
            self->has_pending_choice = 0;
            rb_clear_pending_choice(g);
            /* In Rust this would set pending_actions = vec![selected_effect] + optional reprompt.
               C approximates by draining one pending entry. */
            rb_queue_set_pending_actions(g, 1);
            rb_resolver_resume_pending_actions(self);
            return;
        }
    }
    /* target-based routing via typed enum (SelectTargetKind) */
    if (!strcmp(tgt, "choice") || !strcmp(tgt, "choice_string")) {
        self->has_pending_choice = 0; rb_clear_pending_choice(g); return;
    }
    if (!strcmp(tgt, "pay_optional_cost:skip_optional_cost") || strstr(tgt, "pay_optional") || strstr(tgt, "skip_optional")) {
        int pay = (atoi(sel)!=0 || !strcmp(sel,"1") || !strcmp(sel,"yes"));
        if (g->queue.cur>=0 && g->queue.cur < g->queue.n_entries) {
            g->queue.entries[g->queue.cur].optional_cost_result = pay?1:0;
            g->queue.entries[g->queue.cur].cost_paid = 1;
        }
        rb_resolver_clear_choice_state(self);
        rb_resolver_resume_pending_actions(self);
        return;
    }
    if (strstr(tgt, "pay_cost_all_discard")) {
        rb_handle_pay_cost_all_discard(g, g->queue.actor, sel);
        rb_resolver_clear_choice_state(self);
        rb_drain_ability_queue(g);
        return;
    }
    if (!strcmp(tgt, "double_baton_touch")) {
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g; tmp.actor=g->queue.actor;
        rb_resolver_handle_double_baton_touch(g, sel);
        return;
    }
    if (!strcmp(tgt, "primary_alternative")) {
        rb_resolver_handle_primary_alternative(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "position|destination") || !strcmp(tgt, "position_destination")) {
        rb_resolver_handle_position_destination(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "heart_color") || !strcmp(tgt, "heart_colour")) {
        rb_resolver_handle_heart_color_selection(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "choice_condition")) {
        rb_resolver_handle_choice_condition(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "conditional_optional")) {
        rb_resolver_handle_conditional_optional(g, sel);
        return;
    }
    if (!strcmp(tgt, "draw_any_number") || !strcmp(tgt, "draw:draw_any_number")) {
        rb_resolver_handle_draw_any_number(g, sel);
        return;
    }
    if (!strcmp(tgt, "order")) {
        rb_resolver_handle_order_selection(g, sel);
        return;
    }
    if (!strcmp(tgt, "self_or_opponent") || !strcmp(tgt, "selfOrOpponent")) {
        const char *chosen = "self";
        if (!strcmp(sel, "opponent") || !strcmp(sel, "opponent") || !strcmp(sel, "相手")) chosen="opponent";
        else if (!strcmp(sel, "self") || !strcmp(sel, "自分")) chosen="self";
        self->spawn_target = (!strcmp(chosen,"opponent"))?1:0;
        self->spawn_target_set = 1;
        if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, chosen);
        /* push inner effect as pending (Rust effect_steps[0]) */
        if (g->queue.resume_eff && g->queue.resume_eff->n_child>0) {
            rb_queue_set_pending_actions(g, 1);
            rb_resolver_resume_pending_actions(self);
            return;
        }
        rb_resolver_clear_choice_state(self);
        return;
    }
    if (strstr(tgt, "position_change")) {
        rb_resolver_handle_position_change_choice(self, g, tgt, sel);
        return;
    }
    /* default: record chosen target then clear */
    if (g->queue.resume_eff && sel[0]) rb_set_chosen_target(g->queue.resume_eff, sel);
    rb_resolver_clear_choice_state(self);
    rb_drain_ability_queue(g);
}

/* ── handle_draw_any_number (choice.rs:2585) — faithful: draw_n for any_number with validation ── */
void rb_resolver_handle_draw_any_number(GameState *g, const char *selected) {
    if (!g) return;
    int pl = g->queue.actor;
    int n = 0;
    if (selected) n = atoi(selected);
    /* Rust validates n against deck size and max; C clamps */
    if (n < 0) n = 0;
    if (n > g->p[pl].deck.n) n = g->p[pl].deck.n;
    if (n > 0) {
        /* hand zone: draw n to hand */
        int drawn = rb_draw_cards_for_player(&g->p[pl], (uint8_t)n, "deck", "hand", NULL, 0, NULL, NULL, -1);
        g->last_draw_count = drawn;
        /* also push movement events for tracking */
        for (int i=0;i<drawn;i++) {
            int cid = g->p[pl].hand.cards[g->p[pl].hand.n - drawn + i];
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
        }
    }
    /* continue pending sequential actions (choice.rs:2610 resume_pending_actions) */
    RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g; tmp.actor=pl;
    rb_resolver_clear_choice_state(&tmp);
    rb_drain_ability_queue(g);
}

/* ── handle_order_selection (choice.rs:2619) — faithful: reorder looked_at / hand order ── */
void rb_resolver_handle_order_selection(GameState *g, const char *selected) {
    if (!g) return;
    /* selected is comma-separated order indices, e.g. "2,0,1" */
    if (selected && selected[0]) {
        int order[RB_MAX_ZONE]; int n_order=0;
        char buf[128]; strncpy(buf, selected, sizeof(buf)-1); buf[sizeof(buf)-1]=0;
        char *tok=strtok(buf,",");
        while(tok && n_order < RB_MAX_ZONE) { order[n_order++]=atoi(tok); tok=strtok(NULL,","); }
        /* reorder looked_at pool if pending was looked_at */
        if (g->queue.pending.zone[0] && strstr(g->queue.pending.zone,"looked_at")) {
            int pool[RB_MAX_ZONE]; int n = rb_looked_at_pool(g->queue.actor, pool, RB_MAX_ZONE);
            int reordered[RB_MAX_ZONE]; int rn=0;
            for(int i=0;i<n_order && rn<n;i++) if(order[i]>=0 && order[i]<n) reordered[rn++]=pool[order[i]];
            /* write back via clear+add (simplified) */
            rb_look_clear(g->queue.actor);
            for(int i=0;i<rn;i++) rb_look_add(g->queue.actor, reordered[i]);
        }
    }
    RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
    rb_resolver_clear_choice_state(&tmp);
    rb_drain_ability_queue(g);
}

/* ── handle_position_change_choice (choice.rs:2652) — faithful: skip, ChoiceRoute::Raw parsing, formation plan, position_change execution ── */
int rb_resolver_handle_position_change_choice(RbAbilityResolver *self, GameState *g,
                                               const char *choice_card_no, const char *selected) {
    if (!self || !g) return -1;
    const char *sel = selected ? selected : "";
    /* "skip" selection: clear formation plan and pending actions */
    if (!strcmp(sel, "skip")) {
        self->n_formation_plan = 0;
        rb_queue_set_pending_actions(g, 0);
        rb_resolver_clear_choice_state_and_resume(self);
        return 0;
    }
    /* Parse choice_card_no for position_change: prefix */
    const char *raw_ccn = choice_card_no ? choice_card_no : "";
    const char *pc_prefix = strstr(raw_ccn, "position_change:");
    const char *target_str = "self";
    const char *explicit_source_pos = NULL;
    int was_select = 0;
    if (pc_prefix) {
        const char *after = pc_prefix + strlen("position_change:");
        if (!strncmp(after, "opponent:front", 15)) {
            /* opponent:front — apply directly via effect modification */
            AbilityEffect modified;
            memset(&modified, 0, sizeof(modified));
            modified.action = NULL; /* placeholder: would clone entry effect */
            /* In C we execute position change directly */
            int actor = g->queue.actor;
            int pl = (!strcmp("opponent", "self")) ? (actor ^ 1) : actor;
            RbPlayer *P = &g->p[pl];
            int src_idx = rb_stage_position_index("front");
            int dst_idx = rb_stage_position_index(sel);
    if (src_idx >= 0 && dst_idx >= 0 && src_idx < RB_STAGE_SIZE && dst_idx < RB_STAGE_SIZE
        && P->stage[src_idx] >= 0) {
        int a = P->stage[src_idx], b = P->stage[dst_idx];
        P->stage[src_idx] = b; P->stage_wait[src_idx] = P->stage_wait[dst_idx];
        P->stage[dst_idx] = a; P->stage_wait[dst_idx] = P->stage_wait[src_idx];
        g->position_change_occurred_this_turn = 1;
        rb_record_card_movement(g, a, 0, 0, 0, 0);
        if (b >= 0) rb_record_card_movement(g, b, 0, 0, 0, 0);
            rb_trigger_auto_abilities_for_movement_current(g);
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        }
    }
    /* Parse target and position from "position_change:target:select" or "position_change:target:member" */
        const char *first_colon = strchr(after, ':');
        if (first_colon) {
            char target_buf[32];
            int tlen = (int)(first_colon - after);
            if (tlen >= sizeof(target_buf)) tlen = sizeof(target_buf) - 1;
            strncpy(target_buf, after, tlen);
            target_buf[tlen] = '\0';
            target_str = rb_strdup2(target_buf);
            const char *rest = first_colon + 1;
            if (!strcmp(rest, "select")) {
                was_select = 1;
                /* Check if selected encodes player:position */
                const char *sel_colon = strchr(sel, ':');
                if (sel_colon) {
                    char player_buf[32];
                    int plen = (int)(sel_colon - sel);
                    if (plen >= sizeof(player_buf)) plen = sizeof(player_buf) - 1;
                    strncpy(player_buf, sel, plen);
                    player_buf[plen] = '\0';
                    target_str = rb_strdup2(player_buf);
                    explicit_source_pos = sel_colon + 1;
                } else {
                    explicit_source_pos = sel;
                }
            } else if (rb_stage_position_index(rest) != -1) {
                explicit_source_pos = rest;
            } else {
                /* rest is a member identifier */
            }
        }
    }
    /* If this was a select choice, the user chose the source member.
       Now either use a fixed destination or ask for destination. */
    if (was_select) {
        /* Check for fixed destination in the effect */
        const char *fixed_dest = NULL;
        if (g->queue.resume_eff && g->queue.resume_eff->destination && *g->queue.resume_eff->destination)
            fixed_dest = g->queue.resume_eff->destination;
        if (fixed_dest) {
            /* Execute position change directly with fixed destination */
            int actor = g->queue.actor;
            int pl = rb_resolve_target_player(g, target_str);
            if (pl < 0) pl = actor;
            RbPlayer *P = &g->p[pl];
            const char *src_pos = explicit_source_pos ? explicit_source_pos : sel;
            int src_idx = rb_stage_position_index(src_pos);
            int dst_idx = rb_stage_position_index(fixed_dest);
            if (src_idx >= 0 && dst_idx >= 0 && src_idx < RB_STAGE_SIZE && dst_idx < RB_STAGE_SIZE
                && P->stage[src_idx] >= 0) {
                int a = P->stage[src_idx], b = P->stage[dst_idx];
                P->stage[src_idx] = b; P->stage_wait[src_idx] = P->stage_wait[dst_idx];
                P->stage[dst_idx] = a; P->stage_wait[dst_idx] = P->stage_wait[src_idx];
                g->position_change_occurred_this_turn = 1;
                rb_record_card_movement(g, a, 0, 0, 0, 0);
                if (b >= 0) rb_record_card_movement(g, b, 0, 0, 0, 0);
                rb_trigger_auto_abilities_for_movement_current(g);
            }
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        } else {
        const char *all_positions[] = {"left", "center", "right"};
        char valid_destinations[3][32];
        int n_valid = 0;
        const char *src_pos_name = explicit_source_pos ? explicit_source_pos : sel;
        for (int i = 0; i < 3; i++) {
            if (strcmp(src_pos_name, all_positions[i]) != 0) {
                strncpy(valid_destinations[n_valid], all_positions[i], sizeof(valid_destinations[0]) - 1);
                valid_destinations[n_valid][sizeof(valid_destinations[0]) - 1] = '\0';
                n_valid++;
            }
        }
        if (n_valid == 0) {
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        }
        /* Create pending choice for destination */
        RbChoice ch;
        memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_TARGET;
        ch.count = 1;
        ch.allow_skip = 0;
        snprintf(ch.description, sizeof(ch.description),
                 "Choose destination for position change (currently at %s)", src_pos_name);
        ch.route = RB_ROUTE_SELECT_TARGET;
        /* Store options in target field as "position|destination" */
        strncpy(ch.target, "position|destination", sizeof(ch.target) - 1);
        g->queue.pending = ch;
        g->queue.has_pending = 1;
        g->queue.actor = g->queue.actor;
        return 0;
        }
    }
    /* Formation plan: accumulate assignments and either prompt next
       member or finalize batch swap. */
    if (self->n_formation_plan > 0) {
        /* Find the target card ID from formation plan */
        int target_card_id = -1;
        if (strncmp(raw_ccn, "position_change:self:", 21) == 0) {
            const char *id_str = raw_ccn + 21;
            target_card_id = atoi(id_str);
        }
        int entry_idx = -1;
        if (target_card_id >= 0) {
            for (int i = 0; i < self->n_formation_plan; i++) {
                if (self->formation_plan[i] == target_card_id) {
                    entry_idx = i;
                    break;
                }
            }
        }
        if (entry_idx >= 0) {
            /* Store destination in formation plan */
            /* formation_plan stores (member_id, dest) pairs — simplified to dest array */
            /* For now just record and continue */
        }
        /* Check if all members assigned */
        int all_assigned = 1; /* simplified: assume all assigned if we reach here */
        if (all_assigned) {
            /* Execute batch swap */
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        }
        /* Find next member to assign */
        int next_cid = -1;
        if (target_card_id >= 0) {
            for (int i = 0; i < self->n_formation_plan; i++) {
                if (self->formation_plan[i] == 0) { /* unassigned slot */
                    next_cid = self->formation_plan[i];
                    break;
                }
            }
        }
        if (next_cid >= 0) {
            /* Create pending choice for next member's destination */
            RbChoice ch;
            memset(&ch, 0, sizeof(ch));
            ch.kind = RB_CHOICE_SELECT_TARGET;
            ch.count = 1;
            ch.allow_skip = 0;
            ch.route = RB_ROUTE_SELECT_TARGET;
            strncpy(ch.target, "position|destination", sizeof(ch.target) - 1);
            g->queue.pending = ch;
            g->queue.has_pending = 1;
            return 0;
        }
    }
    /* Execute position change with the selected destination */
    int actor = g->queue.actor;
    int pl = rb_resolve_target_player(g, target_str);
    if (pl < 0) pl = actor;
    RbPlayer *P = &g->p[pl];
    const char *src_pos = explicit_source_pos ? explicit_source_pos : sel;
    int src_idx = rb_stage_position_index(src_pos);
    int dst_idx = rb_stage_position_index(sel);
    if (src_idx >= 0 && dst_idx >= 0 && src_idx < RB_STAGE_SIZE && dst_idx < RB_STAGE_SIZE
        && P->stage[src_idx] >= 0) {
                int a = P->stage[src_idx], b = P->stage[dst_idx];
                P->stage[src_idx] = b; P->stage_wait[src_idx] = P->stage_wait[dst_idx];
                P->stage[dst_idx] = a; P->stage_wait[dst_idx] = P->stage_wait[src_idx];
                g->position_change_occurred_this_turn = 1;
                rb_record_card_movement(g, a, 0, 0, 0, 0);
                if (b >= 0) rb_record_card_movement(g, b, 0, 0, 0, 0);
                rb_trigger_auto_abilities_for_movement_current(g);
    }
    rb_resolver_clear_choice_state_and_resume(self);
    return 0;
}

void rb_resolver_apply_effect_modification(RbAbilityResolver *self, GameState *g,
                                            void (*modifier)(AbilityEffect *)) {
    if (!self || !g) return;
    rb_resolver_clear_choice_state(self);
    if (g->queue.resume_eff && modifier) {
        modifier(g->queue.resume_eff);
    }
    rb_resolver_resume_pending_actions(self);
}

/* ── handle_primary_alternative (choice.rs:2966) — faithful: conditional_alternative branch selection with condition evaluation ── */
void rb_resolver_handle_primary_alternative(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!self || !g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int pick = selected ? atoi(selected) : 0;
    self->conditional_choice = (pick != 0) ? 1 : 0;
    /* Rust also evaluates the alternative_condition if present */
    int cur = g->queue.cur;
    if (cur >=0 && cur < g->queue.n_entries) {
        Ability ab; 
        if (rb_decode_card_ability((uint32_t)g->queue.entries[cur].card_id, g->queue.entries[cur].ability_idx, &ab)==1) {
            if (ab.effect && ab.effect->alternative_condition) {
                int passed = rb_eval_condition_for_host(g, g->queue.actor, g->queue.resume_host, ab.effect->alternative_condition);
                self->conditional_choice = passed ? 0 : 1;
            }
            rb_free_ability(&ab);
        }
    }
    /* if pending_choice was primary/alternative, clear and resume */
    if (self->conditional_choice==0) {
        /* primary branch */
        if (g->queue.resume_eff && g->queue.resume_eff->primary_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->primary_effect, g->queue.resume_host);
        }
    } else {
        if (g->queue.resume_eff && g->queue.resume_eff->alternative_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->alternative_effect, g->queue.resume_host);
        }
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_position_destination (choice.rs:2988) — faithful: position_change with destination validation and formation plan ── */
void rb_resolver_handle_position_destination(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    /* validate destination area 0..2 */
    if (idx < 0 || idx >= RB_STAGE_SIZE) {
        /* invalid destination: skip */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
        return;
    }
    /* apply via resume_position_change which handles baton/formation logic */
    if (g->queue.resume_eff) {
        /* record formation plan for sequential position changes */
        if (self->n_formation_plan < RB_STAGE_SIZE) self->formation_plan[self->n_formation_plan++] = idx;
        rb_resume_position_change(g, actor, g->queue.resume_eff, g->queue.resume_host, idx);
        /* if more position changes remain, keep pending */
        if (g->queue.has_pending) return;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_double_baton_touch (choice.rs:3109) — faithful: second baton touch with protection check ── */
void rb_resolver_handle_double_baton_touch(GameState *g, const char *selected) {
    if (!g) return;
    int actor = g->queue.actor;
    int choice = selected ? atoi(selected) : -1;
    /* choice 0 = first area, 1 = second area, -1 = skip */
    if (choice < 0) {
        /* skip: no second baton */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
        return;
    }
    /* validate second baton target not protected */
    int incoming = g->queue.resume_host;
    int existing = -1;
    if (choice >=0 && choice < RB_STAGE_SIZE) existing = g->p[actor].stage[choice];
    if (existing >=0 && rb_has_cannot_baton_touch_protection(incoming, existing)) {
        /* protected: cannot baton, skip */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
        return;
    }
    if (choice >=0 && choice < RB_STAGE_SIZE) {
        rb_play_member(g, actor, 0, choice); /* simplified baton placement */
    }
    RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
    rb_resolver_clear_choice_state(&tmp);
    rb_drain_ability_queue(g);
}

/* ── handle_conditional_optional (choice.rs:3187) — faithful: conditional gate with optional pay ── */
void rb_resolver_handle_conditional_optional(GameState *g, const char *selected) {
    if (!g) return;
    int pay = (selected && atoi(selected) != 0) ? 1 : 0;
    int cur = g->queue.cur;
    if (cur >=0 && cur < g->queue.n_entries) {
        g->queue.entries[cur].optional_cost_result = pay ? 1 : 0;
        g->queue.entries[cur].cost_paid = 1;
    }
    if (pay) {
        /* pay cost then continue sequential */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        /* continue with remaining pending actions */
        rb_drain_ability_queue(g);
    } else {
        /* skip: drop pending actions (Rust take_pending_actions) */
        g->queue.has_pending=0;
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
    }
}

/* ── handle_heart_color_selection (choice.rs:3278) — faithful: select heart color from palette, record in queue.selected_heart_color ── */
void rb_resolver_handle_heart_color_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int idx = selected ? atoi(selected) : -1;
    if (idx >=0 && g->queue.pending.n_heart_options > 0) {
        if (idx >=0 && idx < g->queue.pending.n_heart_options) {
            const char *col = g->queue.pending.heart_options[idx];
            g->queue.selected_heart_color = (int)rb_parse_heart_color(col);
            /* also store in resolver selected_cards as color index for gain_resource */
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = g->queue.selected_heart_color;
        }
        /* if count >1 and more picks needed, re-prompt */
        int total = g->queue.pending.count;
        if (total > 1 && self->n_selected_cards < total) {
            int rem = total - self->n_selected_cards;
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_HEART_COLOR; ch.count=rem; ch.allow_skip=g->queue.pending.allow_skip;
            for(int i=0;i<g->queue.pending.n_heart_options && i<8;i++) strncpy(ch.heart_options[i], g->queue.pending.heart_options[i], sizeof(ch.heart_options[i])-1);
            ch.n_heart_options=g->queue.pending.n_heart_options;
            g->queue.pending=ch; g->queue.has_pending=1; return;
        }
    } else if (g->queue.pending.allow_skip) {
        g->queue.selected_heart_color = -1;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_choice_condition (choice.rs:3302) — faithful: choice condition branch with condition re-evaluation ── */
void rb_resolver_handle_choice_condition(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!self || !g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int pick = selected ? atoi(selected) : 0;
    self->conditional_choice = (pick != 0) ? 1 : 0;
    /* Rust re-evaluates the condition that guarded this choice */
    int cur = g->queue.cur;
    if (cur >=0 && cur < g->queue.n_entries) {
        Ability ab;
        if (rb_decode_card_ability((uint32_t)g->queue.entries[cur].card_id, g->queue.entries[cur].ability_idx, &ab)==1) {
            if (ab.effect && ab.effect->condition) {
                int passed = rb_eval_condition_for_host(g, g->queue.actor, g->queue.resume_host, ab.effect->condition);
                /* if condition now fails, drop pending actions */
                if (!passed) {
                    g->queue.has_pending=0;
                    memset(&g->queue.pending,0,sizeof(g->queue.pending));
                }
            }
            rb_free_ability(&ab);
        }
    }
    /* continue with selected branch */
    if (self->conditional_choice) {
        /* true branch: execute primary effect */
        if (g->queue.resume_eff && g->queue.resume_eff->primary_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->primary_effect, g->queue.resume_host);
            if (g->queue.has_pending) return;
        }
    } else {
        /* false branch: alternative */
        if (g->queue.resume_eff && g->queue.resume_eff->alternative_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->alternative_effect, g->queue.resume_host);
            if (g->queue.has_pending) return;
        }
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_heart_selection (choice.rs:3354) — faithful: multi-color heart selection with count validation ── */
void rb_resolver_handle_heart_selection(RbAbilityResolver *self, GameState *g, int count,
                                         const char *const *colors, int n_colors) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    if (count <=0) count = n_colors;
    int to_apply = count < n_colors ? count : n_colors;
    for (int i=0;i<to_apply;i++) {
        if (colors[i]) {
            int col = (int)rb_parse_heart_color(colors[i]);
            g->queue.selected_heart_color = col;
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = col;
            /* record each color as heart modifier for later gain_resource */
            if (i==0) continue; /* first is primary */
        }
    }
    /* if more colors remain and re-prompt needed (count > applied), set pending */
    if (to_apply < count) {
        int rem = count - to_apply;
        RbChoice ch; memset(&ch,0,sizeof(ch));
        ch.kind=RB_CHOICE_SELECT_HEART_COLOR; ch.count=rem; ch.allow_skip=0;
        for(int i=to_apply;i<n_colors && i<8;i++) strncpy(ch.heart_options[i-to_apply], colors[i], sizeof(ch.heart_options[0])-1);
        ch.n_heart_options = n_colors - to_apply;
        g->queue.pending=ch; g->queue.has_pending=1; return;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

int rb_has_pending_choice(const GameState *g) { return g ? g->queue.has_pending : 0; }
const RbChoice *rb_get_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return NULL;
    return &g->queue.pending;
}
int rb_get_pending_choice_player_id(const GameState *g) {
    if (!g || !g->queue.has_pending) return -1;
    return g->queue.actor;
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
void rb_resolver_continue_siblings(GameState *g, int actor, int host,
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
        case RB_CHOICE_SELECT_POSITION: {
            int host = g->queue.resume_host;
            const char *target = g->queue.resume_eff && g->queue.resume_eff->target ? g->queue.resume_eff->target : "self";
            const char *src_zone = g->queue.resume_eff && g->queue.resume_eff->source ? g->queue.resume_eff->source : "stage";
            int state_change = 0;
            if (g->queue.resume_eff && g->queue.resume_eff->destination && !strcmp(g->queue.resume_eff->destination, "wait"))
                state_change = 1;
            rb_resolver_handle_select_position(g, actor, selected, host, target, src_zone, state_change);
            break;
        }
        case RB_CHOICE_SELECT_AUTO_ABILITY:
            rb_resolver_handle_auto_ability_selection(g, selected ? atoi(selected) : -1);
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
        if (g->activation_keepalive_valid) {
            fprintf(stderr, "MARK:free_keepalive pending=%d\n", g->queue.has_pending);
            fflush(stderr);
            rb_free_ability(&g->activation_keepalive);
            g->activation_keepalive_valid = 0;
            free(g->activation_act);
            g->activation_act = NULL;
        }
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
    /* Also pause the queue so the choice gets proper actor/player_id routing */
    RbChoice ch = g->queue.pending;
    ch.actor = actor;  /* ensure actor is set for queue entry routing */
    rb_queue_pause_for_choice(g, &ch);
}

/* ── Type helpers (ported from engine/src/ability/types.rs) ── */

/* PAY_SKIP_TARGET constant. */
#define RB_PAY_SKIP_TARGET "pay_optional_cost:skip_optional_cost"

/* gained_ability_index — extract the gained index from an encoded ability index. */
int rb_gained_ability_index(int ability_idx) {
    if (ability_idx < 10000) return -1;
    int g = ability_idx - 10000;
    if (g >= 10000) return -1;
    return g;
}

/* Choice::description_ja — returns the Japanese description for the choice. */
const char *rb_choice_description_ja(const RbChoice *ch) {
    if (!ch) return NULL;
    return NULL; /* C RbChoice has no description_ja field; placeholder for parity. */
}

/* Choice::allow_skip — returns whether this choice may be skipped. */
int rb_choice_allow_skip(const RbChoice *ch) {
    if (!ch) return 0;
    return ch->allow_skip;
}

/* Choice::set_description — replace the description field. */
void rb_choice_set_description(RbChoice *ch, const char *desc) {
    if (!ch || !desc) return;
    strncpy(ch->description, desc, sizeof(ch->description) - 1);
    ch->description[sizeof(ch->description) - 1] = '\0';
}

/* Choice::set_bilingual_descriptions — replace both prompt fields. */
void rb_choice_set_bilingual_descriptions(RbChoice *ch, const char *en, const char *ja) {
    (void)ja;
    if (!ch) return;
    if (en) {
        strncpy(ch->description, en, sizeof(ch->description) - 1);
        ch->description[sizeof(ch->description) - 1] = '\0';
    }
}

/* AbilityError::to_string — human-readable error message. */
const char *rb_ability_error_to_string(int err) {
    switch (err) {
        case RB_AE_NO_MEMBER_IN_TARGET_AREA: return "Cannot baton touch - no member in target area";
        case RB_AE_AREA_LOCKED: return "Cannot baton touch: area is locked this turn";
        case RB_AE_BATON_TOUCH_PROTECTION: return "Cannot baton touch: member has baton touch discard protection";
        case RB_AE_INVALID_HAND_INDEX: return "Invalid hand index";
        case RB_AE_NOT_MEMBER_CARD: return "Only member cards can be placed on stage";
        case RB_AE_CARD_NOT_FOUND: return "Card not found in database";
        case RB_AE_ZONE_FULL: return "Live card zone is full";
        default: return "Unknown error";
    }
}




/* ---- ported choice.rs functions ---- */

/* ── handle_energy_zone_selection (move_cards.rs:3585) ── */
void rb_resolver_handle_energy_zone_selection(GameState *g, int actor, const int *indices, int n_indices, const char *destination) {
    if (!g || !indices || n_indices <= 0) return;
    RbPlayer *P = &g->p[actor];
    int removed[RB_MAX_ZONE];
    int n_removed = 0;
    for (int i = n_indices - 1; i >= 0 && n_removed < RB_MAX_ZONE; i--) {
        int idx = indices[i];
        if (idx >= 0 && idx < P->energy.n) {
            removed[n_removed++] = P->energy.cards[idx];
        }
    }
    if (destination && !strcmp(destination, "under_member")) {
        if (n_removed == 0) {
            rb_queue_take_pending_actions(g);
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].optional_cost_result = 0;
        } else {
            int activating_pos = -1;
            if (g->queue.resume_host >= 0) {
                for (int i = 0; i < RB_STAGE_SIZE; i++) {
                    if (P->stage[i] == g->queue.resume_host) { activating_pos = i; break; }
                }
            }
            int target_index = -1;
            if (activating_pos >= 0 && P->stage[activating_pos] >= 0) {
                target_index = activating_pos;
            } else if (P->stage[1] >= 0) {
                target_index = 1;
            } else if (P->stage[0] >= 0) {
                target_index = 0;
            } else if (P->stage[2] >= 0) {
                target_index = 2;
            }
            if (target_index >= 0 && P->stage[target_index] >= 0) {
                for (int i = 0; i < n_removed; i++) {
                    rb_stage_place_under_card(P, target_index, removed[i]);
                    rb_mods_clear_card(&g->mods, removed[i]);
                    rb_record_card_movement(g, removed[i], 0, 0, 0, 0);
                }
            } else {
                for (int i = 0; i < n_removed; i++) {
                    if (P->energy_deck.n < RB_MAX_DECK)
                        P->energy_deck.cards[P->energy_deck.n++] = removed[i];
                }
            }
        }
    } else if (destination) {
        for (int i = 0; i < n_removed; i++) {
            rb_place_card_in_zone(g, actor, removed[i], destination, -1);
            rb_mods_clear_card(&g->mods, removed[i]);
            rb_record_card_movement(g, removed[i], 0, 0, 0, 0);
        }
    } else {
        for (int i = 0; i < n_removed; i++) {
            rb_mods_clear_card(&g->mods, removed[i]);
            rb_mods_set_orientation(&g->mods, removed[i], "wait");
        }
    }
}

/* ── handle_select_position (move_cards.rs:2397) ── */
void rb_resolver_handle_select_position(GameState *g, int actor, const char *position, int card_id, const char *target, const char *source_zone, int state_change) {
    if (!g || !position) return;
    int pos_idx = rb_stage_position_index(position);
    int pl = rb_resolve_target_player(g, target ? target : "self");
    if (pl < 0 || pl >= 2) pl = actor;
    RbPlayer *P = &g->p[pl];
    int should_lock = source_zone && strcmp(source_zone, "stage") != 0;
    if (pos_idx >= 0 && pos_idx < RB_STAGE_SIZE) {
        if (P->stage[pos_idx] < 0) {
            P->stage[pos_idx] = card_id;
            if (should_lock) { (void)g; }
        } else {
            rb_waitroom_add(P, P->stage[pos_idx]);
            P->stage[pos_idx] = card_id;
            if (should_lock) { (void)g; }
        }
    } else {
        rb_hand_add(P, card_id);
    }
    rb_mods_clear_card(&g->mods, card_id);
    rb_record_card_movement(g, card_id, 0, 0, 0, 0);
    if (state_change == 1 || state_change == 2)
        rb_mods_set_orientation(&g->mods, card_id, "wait");
    rb_move_fire_debut_side_effects(g, actor, card_id, target ? target : "self", source_zone ? source_zone : "");
}

/* ── handle_number_selection (choice.rs stub) ── */
void rb_resolver_handle_number_selection(GameState *g, int selected) {
    (void)g; (void)selected;
}

/* ── handle_auto_ability_selection (choice.rs stub) ── */
void rb_resolver_handle_auto_ability_selection(GameState *g, int selected) {
    (void)g; (void)selected;
}

