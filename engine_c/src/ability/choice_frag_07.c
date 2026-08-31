/* choice_frag_07.c — Port of engine/src/ability/choice.rs
   (filter_discard_by_budget, handle_discard_selection).

   Rust source paths referenced inline:
     choice.rs:2038  fn filter_discard_by_budget
     choice.rs:2074  fn handle_discard_selection

   Conventions:
     - self: AbilityResolver*  (self.gs -> GameState*)
     - Rust `ctx.mfi(&ctx.indices)` mapping is pre-applied: ctx->indices /
       ctx->n_indices already hold the mapped zone positions.
     - Rust `Option<u8>` cost_total is modeled as (has_cost_total, cost_total_val).
     - Helper resolvers (build_reprompt / store_pending_choice / finalize_choice /
       execute_selected_cards_from_zone / handle_selection_epilogue /
       clear_choice_state_and_resume) and rb_set_chosen_target are CALLED BY NAME
       and are NOT defined in this fragment. */

#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* ── Local mirror of the Rust AbilityResolver (engine/src/ability/types.rs) ── */
typedef struct AbilityResolver {
    GameState *gs;                                   /* self.gs */
    int  selected_cards[RB_MAX_RECENTLY_MOVED];      /* self.selected_cards: Vec<i16> */
    int  n_selected_cards;
    RbChoice pending_choice;                         /* self.pending_choice: Option<Choice> */
    int  has_pending_choice;
    RbChoice pending_reprompt_choice;                /* self.pending_reprompt_choice */
    int  has_pending_reprompt_choice;
    int  pending_filtered_idxs[RB_MAX_ZONE];         /* reprompt filtered-index pool */
    int  pending_n_filtered;
    int  pending_is_select_action;
    int  sub_choice_created;                         /* self.sub_choice_created */
} AbilityResolver;

/* ── Local mirror of SelectionContext (engine/src/ability/types.rs) ── */
typedef struct SelectionContext {
    int         is_select_action;        /* ctx.is_select_action */
    int         count;                   /* ctx.count */
    int         allow_skip;              /* ctx.allow_skip */
    const char *target_player_id;        /* ctx.target_player_id ("self"/"opponent"/NULL) */
    const int  *indices;                 /* ctx.indices (already mfi-mapped) */
    int         n_indices;
    int         has_cost_total;          /* ctx.cost_total: Option<u8> */
    uint8_t     cost_total_val;
    const char *cost_total_operator;     /* ctx.cost_total_operator ("<=" or NULL) */
    const char *card_type;               /* ctx.card_type */
    int         cost_limit;              /* ctx.cost_limit */
    const char *cost_limit_operator;     /* ctx.cost_limit_operator */
    const char *group;                   /* ctx.group */
    const char **characters;             /* ctx.characters */
    int         n_characters;
} SelectionContext;

/* ── Local mirror of ExecutionContext (engine/src/ability/types.rs) ── */
typedef struct ExecutionContext {
    int dummy; /* placeholder; real fields unused by this fragment */
} ExecutionContext;

/* ── Forward prototypes (defined in other fragments / choice.c) ── */
RbChoice rb_resolver_build_reprompt(
        const SelectionContext *ctx, const char *zone, int count,
        const char *desc, const char *desc_ja, int allow_skip,
        const int *filtered_idxs, int n_filtered, const char *target,
        int has_cost_total, uint8_t cost_total_val, const char *cost_total_operator,
        int is_select_action);
void     rb_resolver_store_pending_choice(AbilityResolver *self, GameState *gs);
const char *rb_resolver_finalize_choice(AbilityResolver *self, GameState *gs,
                                        const ExecutionContext *context);
const char *rb_resolver_execute_selected_cards_from_zone(
        AbilityResolver *self, GameState *gs, const char *zone,
        const int *mapped_indices, int n_mapped, const char *card_type,
        int cost_limit, const char *cost_limit_operator,
        int has_cost_total, uint8_t cost_total_val, const char *cost_total_operator,
        const char *group, const char **characters, int n_characters,
        const char *target_player_id);
void     rb_resolver_handle_selection_epilogue(AbilityResolver *self, GameState *gs,
                                               const ExecutionContext *context);
void     rb_resolver_clear_choice_state_and_resume(AbilityResolver *self, GameState *gs);
void     rb_set_chosen_target(GameState *gs, int target_card_id);

/* Forward prototype for the function defined later in this fragment. */
void rb_resolver_filter_discard_by_budget(
        AbilityResolver *self, GameState *gs, const SelectionContext *ctx,
        const int *waitroom_cards, int n_wr,
        int *out_has_budget, uint8_t *out_budget,
        int *out_idxs, int *out_n_idxs);

/* ── small fragment-local helpers ── */

/* Rust: self.selected_cards.contains(&cid) */
static int rb_resolver_selected_contains(const AbilityResolver *self, int cid) {
    for (int i = 0; i < self->n_selected_cards; i++)
        if (self->selected_cards[i] == cid) return 1;
    return 0;
}

/* Rust: gs.resolve_target_player_mut(&target) -> &mut Player.
   C has no _mut variant; rb_resolve_target_player returns the player index. */
static RbPlayer *rb_resolver_target_player_mut(GameState *gs, const char *target) {
    const char *t = target ? target : "self";
    int pl = rb_resolve_target_player(gs, t);
    if (pl < 0) pl = 0;
    return &gs->p[pl];
}

/* choice.rs:2038  fn filter_discard_by_budget
   Rust returns (Option<u8> remaining_budget, Vec<usize> all_idxs).
   C returns via out-params; out_idxs must hold at least n_wr entries. */
void rb_resolver_filter_discard_by_budget(
        AbilityResolver *self, GameState *gs, const SelectionContext *ctx,
        const int *waitroom_cards, int n_wr,
        int *out_has_budget, uint8_t *out_budget,
        int *out_idxs, int *out_n_idxs) {
    /* Rust: spent = sum of selected_cards' cost (Option<u8> summed, None => 0) */
    unsigned spent = 0;
    for (int i = 0; i < self->n_selected_cards; i++) {
        int cid = self->selected_cards[i];
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            /* Rust Option<u8> cost; C Card.cost is u8 (0 when absent) */
            spent += (unsigned)c.cost;
        }
    }
    /* Rust: remaining_budget = ctx.cost_total.map(|tb| tb.saturating_sub(spent)) */
    int has_budget = 0;
    uint8_t budget = 0;
    if (ctx->has_cost_total) {
        has_budget = 1;
        budget = rb_saturate_u8((int)ctx->cost_total_val - (int)spent); /* saturating_sub */
    }
    /* Rust: use_budget_filter = match cost_total_operator { Some(op)=>op=="<=", None=>cost_total.is_some() } */
    int use_budget_filter;
    if (ctx->cost_total_operator) {
        use_budget_filter = (strcmp(ctx->cost_total_operator, "<=") == 0);
    } else {
        use_budget_filter = ctx->has_cost_total;
    }
    int n = 0;
    if (use_budget_filter) {
        uint8_t rb = has_budget ? budget : 0;
        /* Rust: filter waitroom_cards where cost.unwrap_or(99) <= rb */
        for (int idx = 0; idx < n_wr; idx++) {
            int cid = waitroom_cards[idx];
            Card c;
            uint8_t cc = 99; /* unwrap_or(99) */
            if (rb_decode_card_by_index((uint32_t)cid, &c)) cc = c.cost;
            if (cc <= rb) out_idxs[n++] = idx;
        }
    } else {
        /* Rust: (0..waitroom_cards.len()).collect() */
        for (int idx = 0; idx < n_wr; idx++) out_idxs[idx] = idx;
        n = n_wr;
    }
    if (out_has_budget) *out_has_budget = has_budget;
    if (out_budget)     *out_budget = budget;
    if (out_n_idxs)     *out_n_idxs = n;
}

/* choice.rs:2074  fn handle_discard_selection
   Rust returns Result<(), String>; C returns const char* (NULL = Ok, else error). */
const char *rb_resolver_handle_discard_selection(
        AbilityResolver *self, GameState *gs,
        const SelectionContext *ctx, const ExecutionContext *context,
        int (*validate_card)(int)) {
    (void)validate_card; /* Rust _validate_card unused on the discard path */

    /* Rust: let mapped_indices = ctx.mfi(&ctx.indices); */
    const int *mapped_indices = ctx->indices;
    int n_mapped = ctx->n_indices;

    const char *target = ctx->target_player_id ? ctx->target_player_id : "self";

    /* Rust: clear stale resolver choice state and resume the queue before handling. */
    rb_resolver_clear_choice_state_and_resume(self, gs);

    if (ctx->is_select_action) {
        /* Rust: player = gs.resolve_target_player_mut(&target) */
        RbPlayer *player = rb_resolver_target_player_mut(gs, target);
        /* Rust: collect chosen waitroom cards into self.selected_cards */
        for (int k = 0; k < n_mapped; k++) {
            int i = mapped_indices[k];
            if (i >= 0 && i < player->discard.n) {
                int cid = player->discard.cards[i];
                if (!rb_resolver_selected_contains(self, cid)) {
                    if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) {
                        self->selected_cards[self->n_selected_cards++] = cid;
                        /* record the chosen target (called by name, do not define) */
                        rb_set_chosen_target(gs, cid);
                    }
                }
            }
        }
        int selected_so_far = self->n_selected_cards;
        /* Rust: if ctx.count > 0 && !mapped_indices.is_empty() && selected_so_far < ctx.count */
        if (ctx->count > 0 && n_mapped > 0 && selected_so_far < ctx->count) {
            int remaining = ctx->count - selected_so_far;
            /* Rust: waitroom_cards = player.waitroom.cards.to_vec() */
            int wr[RB_MAX_ZONE]; int n_wr = player->discard.n;
            for (int i = 0; i < n_wr; i++) wr[i] = player->discard.cards[i];
            /* Rust: filtered_idxs = indices whose card is NOT already selected */
            int filtered_idxs[RB_MAX_ZONE]; int nf = 0;
            for (int i = 0; i < n_wr; i++) {
                if (!rb_resolver_selected_contains(self, wr[i])) filtered_idxs[nf++] = i;
            }
            int *fi = (nf > 0) ? filtered_idxs : NULL;
            int n_fi = nf;
            char desc[256], desc_ja[256];
            snprintf(desc, sizeof(desc),
                     "Select %d more card(s) from discard from %d remaining",
                     remaining, nf);
            snprintf(desc_ja, sizeof(desc_ja),
                     "控え室から残り%d枚中さらに%d枚選択", nf, remaining);
            /* Rust: build_reprompt(...).is_select_action(true).build() */
            RbChoice ch = rb_resolver_build_reprompt(
                    ctx, "discard", remaining, desc, desc_ja, 0,
                    fi, n_fi, target,
                    ctx->has_cost_total, ctx->cost_total_val, ctx->cost_total_operator,
                    1 /* is_select_action */);
            self->pending_choice = ch;
            self->has_pending_choice = 1;
            self->pending_is_select_action = 1;
            self->pending_n_filtered = n_fi;
            if (n_fi > 0) memcpy(self->pending_filtered_idxs, fi, sizeof(int) * n_fi);
            rb_resolver_store_pending_choice(self, gs);
            return NULL; /* Rust: return Ok(()) */
        }
        /* Rust: return self.finalize_choice(gs, &context); */
        return rb_resolver_finalize_choice(self, gs, context);
    }

    /* ── else: non-select_action path ── */

    /* Rust: empty pick on a MANDATORY selection must re-offer the same choice. */
    if (n_mapped == 0 && !ctx->allow_skip) {
        char desc[256], desc_ja[256];
        snprintf(desc, sizeof(desc), "Select %d card(s) from the waiting room", ctx->count);
        snprintf(desc_ja, sizeof(desc_ja), "控え室から%d枚選択", ctx->count);
        RbChoice ch = rb_resolver_build_reprompt(
                ctx, "discard", ctx->count, desc, desc_ja, 0,
                NULL, 0, target,
                ctx->has_cost_total, ctx->cost_total_val, ctx->cost_total_operator,
                0);
        self->pending_choice = ch;
        self->has_pending_choice = 1;
        self->pending_is_select_action = 0;
        self->pending_n_filtered = 0;
        rb_resolver_store_pending_choice(self, gs);
        return NULL;
    }

    /* Rust: partial pick — fewer than ctx.count selected. */
    if (n_mapped > 0 && ctx->count > 0 && n_mapped < ctx->count) {
        RbPlayer *player = rb_resolver_target_player_mut(gs, target);
        int wr[RB_MAX_ZONE]; int n_wr = player->discard.n;
        for (int i = 0; i < n_wr; i++) wr[i] = player->discard.cards[i];

        /* Rust: filtered_idxs from previously selected cards (if any). */
        int filtered_idxs[RB_MAX_ZONE]; int nf = 0;
        if (self->n_selected_cards > 0) {
            for (int idx = 0; idx < n_wr; idx++) {
                int cid = wr[idx];
                if (rb_resolver_selected_contains(self, cid)) {
                    int dup = 0;
                    for (int j = 0; j < nf; j++) if (filtered_idxs[j] == idx) dup = 1;
                    if (!dup) filtered_idxs[nf++] = idx;
                }
            }
        }
        /* Rust: current_card_ids = mapped waitroom cards; push into selected_cards. */
        for (int k = 0; k < n_mapped; k++) {
            int i = mapped_indices[k];
            if (i >= 0 && i < n_wr) {
                int cid = wr[i];
                if (!rb_resolver_selected_contains(self, cid)) {
                    if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                        self->selected_cards[self->n_selected_cards++] = cid;
                }
            }
        }
        /* Rust: self.execute_selected_cards_from_zone(...) */
        const char *err = rb_resolver_execute_selected_cards_from_zone(
                self, gs, "discard", mapped_indices, n_mapped,
                ctx->card_type, ctx->cost_limit, ctx->cost_limit_operator,
                ctx->has_cost_total, ctx->cost_total_val, ctx->cost_total_operator,
                ctx->group, ctx->characters, ctx->n_characters,
                ctx->target_player_id);
        if (err) return err;

        if (self->sub_choice_created) {
            self->sub_choice_created = 0;
            int remaining = ctx->count - n_mapped;
            if (remaining > 0) {
                int wr2[RB_MAX_ZONE]; int n_wr2 = player->discard.n;
                for (int i = 0; i < n_wr2; i++) wr2[i] = player->discard.cards[i];
                int has_budget = 0; uint8_t budget = 0;
                int all_idxs[RB_MAX_ZONE]; int n_all = 0;
                rb_resolver_filter_discard_by_budget(self, gs, ctx, wr2, n_wr2,
                                                    &has_budget, &budget, all_idxs, &n_all);
                if (n_all > 0 || ctx->allow_skip) {
                    char desc[256], desc_ja[256];
                    snprintf(desc, sizeof(desc),
                             "Select %d more card(s) from discard%s",
                             remaining, ctx->allow_skip ? " (or skip to finish)" : "");
                    snprintf(desc_ja, sizeof(desc_ja),
                             "控え室からさらに%d枚選択%s", remaining,
                             ctx->allow_skip ? "（スキップで終了）" : "");
                    int *fi = (n_all > 0) ? all_idxs : NULL;
                    int n_fi = n_all;
                    RbChoice reprompt = rb_resolver_build_reprompt(
                            ctx, "discard", remaining, desc, desc_ja, ctx->allow_skip,
                            fi, n_fi, target,
                            has_budget, budget, "<=", 0);
                    /* Rust: pending = gs.ability_queue.take_pending_actions();
                       self.pending_reprompt_choice = Some(reprompt);
                       gs.ability_queue.set_pending_actions(pending);
                       (C keeps pending_reprompt_choice separate, no queue clobber.) */
                    self->pending_reprompt_choice = reprompt;
                    self->has_pending_reprompt_choice = 1;
                }
            }
            return NULL;
        }

        /* Rust: not a sub_choice — re-offer the remaining picks. */
        int wr3[RB_MAX_ZONE]; int n_wr3 = player->discard.n;
        for (int i = 0; i < n_wr3; i++) wr3[i] = player->discard.cards[i];
        int has_budget = 0; uint8_t budget = 0;
        int all_idxs[RB_MAX_ZONE]; int n_all = 0;
        rb_resolver_filter_discard_by_budget(self, gs, ctx, wr3, n_wr3,
                                            &has_budget, &budget, all_idxs, &n_all);
        int remaining = ctx->count - n_mapped;
        char desc[256], desc_ja[256];
        snprintf(desc, sizeof(desc),
                 "Select %d more card(s) from discard%s",
                 remaining, ctx->allow_skip ? " (or skip to finish)" : "");
        snprintf(desc_ja, sizeof(desc_ja),
                 "控え室からさらに%d枚選択%s", remaining,
                 ctx->allow_skip ? "（スキップで終了）" : "");
        int *fi = (n_all > 0) ? all_idxs : NULL;
        int n_fi = n_all;
        RbChoice ch = rb_resolver_build_reprompt(
                ctx, "discard", remaining, desc, desc_ja, ctx->allow_skip,
                fi, n_fi, target,
                has_budget, budget, "<=", 0);
        self->pending_choice = ch;
        self->has_pending_choice = 1;
        self->pending_is_select_action = 0;
        self->pending_n_filtered = n_fi;
        if (n_fi > 0) memcpy(self->pending_filtered_idxs, fi, sizeof(int) * n_fi);
        rb_resolver_store_pending_choice(self, gs);
        return NULL;
    }

    /* ── final branch: selection complete ── */
    RbPlayer *player = rb_resolver_target_player_mut(gs, target);
    int all_idxs[RB_MAX_ZONE]; int n_all = 0;
    for (int k = 0; k < n_mapped; k++) all_idxs[n_all++] = mapped_indices[k];

    /* Rust: prev_ids = self.selected_cards.clone(); if !prev_ids.is_empty() { merge } */
    if (self->n_selected_cards > 0) {
        int wr[RB_MAX_ZONE]; int n_wr = player->discard.n;
        for (int i = 0; i < n_wr; i++) wr[i] = player->discard.cards[i];
        for (int idx = 0; idx < n_wr; idx++) {
            int cid = wr[idx];
            if (rb_resolver_selected_contains(self, cid)) {
                int dup = 0;
                for (int j = 0; j < n_all; j++) if (all_idxs[j] == idx) dup = 1;
                if (!dup && n_all < RB_MAX_ZONE) all_idxs[n_all++] = idx;
            }
        }
        self->n_selected_cards = 0; /* Rust: self.selected_cards.clear(); */
    }

    const char *err = rb_resolver_execute_selected_cards_from_zone(
            self, gs, "discard", all_idxs, n_all,
            ctx->card_type, ctx->cost_limit, ctx->cost_limit_operator,
            ctx->has_cost_total, ctx->cost_total_val, ctx->cost_total_operator,
            ctx->group, ctx->characters, ctx->n_characters,
            ctx->target_player_id);
    if (err) return err;

    if (self->sub_choice_created) {
        rb_resolver_store_pending_choice(self, gs);
    }

    /* Rust: self.handle_selection_epilogue(gs, context) */
    rb_resolver_handle_selection_epilogue(self, gs, context);
    /* Rust: choice state cleared + queue resumed. */
    rb_resolver_clear_choice_state_and_resume(self, gs);
    return NULL;
}
