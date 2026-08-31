/* engine_c/src/ability/choice_frag_04.c
 * Port of engine/src/ability/choice.rs::handle_reveal_selection /
 * handle_revealed_cards_selection (Rust lines ~1399-1601).
 *
 * Rust-path notes are kept inline. `self` (AbilityResolver) -> RbAbilityResolver*.
 * `self.gs` (GameState) -> the GameState* passed alongside `self`.
 * The three resolver helpers named below are declared (not defined) here because
 * they live in the larger resolver translation. */

#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* ── Resolver-local types (mirror engine/src/ability/types.rs + choice.rs) ── */

/* SelectionContext — engine/src/ability/choice.rs:28 struct SelectionContext. */
typedef struct RbSelectionContext {
    char    *card_type;          /* Option<String> */
    int      count;              /* usize (0 = arbitrary) */
    int      allow_skip;         /* bool */
    int     *indices;            /* Vec<usize> */
    int      n_indices;
    int      cost_limit;         /* Option<u8>, -1 = none */
    char    *cost_limit_operator;
    int      cost_total;         /* Option<u8>, -1 = none */
    char    *cost_total_operator;
    char    *group;              /* Option<String> */
    char   **characters;         /* Option<Vec<String>> */
    int      n_characters;
    int     *filtered_indices;   /* Option<Vec<usize>> */
    int      n_filtered_indices;
    int      is_select_action;   /* bool */
    char    *target_player_id;   /* Option<String> */
    char    *destination;        /* Option<String> */
    int      discard_remaining;  /* Option<bool>, -1 none / 0 / 1 */
    int      blind;              /* bool */
    int      is_reveal;          /* bool */
} RbSelectionContext;

/* AbilityResolver — engine/src/ability/resolver.rs (minus fields unused here). */
typedef struct RbAbilityResolver {
    int       actor;             /* resolving player (self.actor) */
    int16_t   selected_cards[RB_MAX_RECENTLY_MOVED]; /* Vec<i16> */
    int       n_selected_cards;
    RbChoice  pending_choice;    /* Option<Choice> */
    int       has_pending_choice;
    int16_t   moved_cards[RB_MAX_RECENTLY_MOVED];    /* Vec<i16> */
    int       n_moved_cards;
    AbilityEffect *current_effect;   /* Option<&AbilityEffect> */
    int       sub_choice_created;
} RbAbilityResolver;

/* ── Forward-declared external helpers (rb_* engine API, defined elsewhere) ── */
int  rb_current_ability_source_card_id(const GameState *g);
const char *rb_ability_master_id(const GameState *g);
int  rb_effect_resource_on_select_any(const AbilityEffect *e);
int  rb_effect_discard_remaining_any(const AbilityEffect *e);
void rb_ability_queue_take_pending_actions(GameState *g);

/* ── The three resolver helpers named in the port spec (do NOT define here) ── */
int  rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *gs);
int  rb_resolver_clear_choice_state_and_resume(RbAbilityResolver *self, GameState *gs);
void rb_set_chosen_target(AbilityEffect *effect, const char *target);

/* ── Static resolver-local helpers (rb_* helpers used within this fragment) ── */

/* choice.rs:49 SelectionContext::mfi — map ctx.indices through filtered_indices. */
static int rb_resolver_mfi(const RbSelectionContext *ctx,
                           const int *indices, int n_indices,
                           int *out, int max_out) {
    int n = 0;
    if (ctx->filtered_indices) {
        for (int i = 0; i < n_indices && n < max_out; i++) {
            int idx = indices[i];
            if (idx >= 0 && idx < ctx->n_filtered_indices)
                out[n++] = ctx->filtered_indices[idx];
        }
    } else {
        for (int i = 0; i < n_indices && n < max_out; i++)
            out[n++] = indices[i];
    }
    return n;
}

/* store_pending_choice — mirrors self.store_pending_choice(gs): publish the
   resolver's pending_choice onto the ability queue so the host can answer. */
static void rb_resolver_store_pending_choice(RbAbilityResolver *self, GameState *gs) {
    if (!gs) return;
    gs->queue.pending = self->pending_choice;
    gs->queue.has_pending = self->has_pending_choice ? 1 : 0;
    gs->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

/* push_revealed_card — mirrors gs.push_revealed_card(cid, source, false, owner, "ability"). */
static void rb_resolver_push_revealed_card(GameState *gs, int cid, int source,
                                           int is_cost, int owner, const char *reason) {
    (void)source; (void)is_cost; (void)owner; (void)reason;
    if (!gs) return;
    if (gs->n_revealed < RB_MAX_RECENTLY_MOVED)
        gs->revealed_cards[gs->n_revealed++] = cid;
}

/* push_revealed_cost_card — mirrors gs.push_revealed_cost_card(cid, source, false, owner, "cost"). */
static void rb_resolver_push_revealed_cost_card(GameState *gs, int cid, int source,
                                                int is_cost, int owner, const char *reason) {
    (void)source; (void)is_cost; (void)owner; (void)reason;
    if (!gs) return;
    if (gs->n_revealed < RB_MAX_RECENTLY_MOVED)
        gs->revealed_cards[gs->n_revealed++] = cid;
}

/* push_rule_log — mirrors gs.push_rule_log(format!(...)). No C log field; stub. */
static void rb_resolver_push_rule_log(GameState *gs, const char *fmt, ...) {
    (void)gs; (void)fmt;
}

/* move_from_revealed — mirrors self.move_from_revealed(gs, &mapped, validate_card, dst).
   mapped[] are indices into gs->revealed_cards. Returns moved card ids in out_moved. */
static int rb_resolver_move_from_revealed(RbAbilityResolver *self, GameState *gs,
                                          const int *mapped, int n_mapped,
                                          int (*validate_card)(int),
                                          const char *dst_str,
                                          int16_t *out_moved, int max_moved) {
    (void)self;
    if (!gs) return 0;
    int pl = rb_resolve_target_player(gs, "self");
    int n_moved = 0;
    /* Mark which revealed slots are consumed. */
    int consumed[RB_MAX_RECENTLY_MOVED] = {0};
    for (int i = 0; i < n_mapped; i++) {
        int slot = mapped[i];
        if (slot < 0 || slot >= gs->n_revealed) continue;
        int cid = gs->revealed_cards[slot];
        int ok = validate_card ? validate_card(cid) : 1;
        if (!ok) continue;
        if (pl >= 0 && dst_str)
            rb_place_card_in_zone(gs, pl, cid, dst_str, 0);
        if (n_moved < max_moved) out_moved[n_moved++] = (int16_t)cid;
        consumed[slot] = 1;
    }
    /* Compact the revealed_cards array, dropping consumed slots. */
    int w = 0;
    for (int r = 0; r < gs->n_revealed; r++) {
        if (!consumed[r]) gs->revealed_cards[w++] = gs->revealed_cards[r];
    }
    gs->n_revealed = w;
    return n_moved;
}

/* ── handle_reveal_selection (choice.rs:1399) ── */
int rb_resolver_handle_reveal_selection(RbAbilityResolver *self, GameState *gs,
                                        const RbSelectionContext *ctx) {
    /* let effect_started = gs.ability_queue.current_entry().is_some_and(|e| e.effect_started) */
    int effect_started = 0;
    if (gs->queue.n_entries > 0 && gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries)
        effect_started = gs->queue.entries[gs->queue.cur].effect_started;

    /* let target = ctx.target_player_id.clone().unwrap_or_else(|| "self".to_string().into()) */
    const char *target = (ctx->target_player_id && *ctx->target_player_id) ? ctx->target_player_id : "self";

    /* hand_positions: derive from filtered_indices (mfi semantics) */
    int hand_positions[RB_MAX_HAND];
    int n_hand_positions = rb_resolver_mfi(ctx, ctx->indices, ctx->n_indices,
                                           hand_positions, RB_MAX_HAND);

    /* If a bounded selection is still short, accumulate what we have and re-prompt. */
    if (n_hand_positions > 0 && ctx->count > 0 && n_hand_positions < ctx->count) {
        for (int i = 0; i < n_hand_positions; i++) {
            int hp = hand_positions[i];
            int found = 0;
            for (int j = 0; j < self->n_selected_cards; j++)
                if (self->selected_cards[j] == (int16_t)hp) { found = 1; break; }
            if (!found && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = (int16_t)hp;
        }
        int remaining = ctx->count - n_hand_positions;
        /* Choice::select_cards(Zone::Hand, remaining, "Select N more ...", false) */
        RbChoice ch;
        memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_CARD;
        strncpy(ch.zone, "hand", sizeof(ch.zone) - 1);
        ch.count = remaining;
        snprintf(ch.description, sizeof(ch.description) - 1,
                 "Select %d more card(s) from hand%s", remaining,
                 ctx->blind ? " (blind)" : "");
        if (ctx->card_type) strncpy(ch.card_type, ctx->card_type, sizeof(ch.card_type) - 1);
        strncpy(ch.target, target, sizeof(ch.target) - 1);
        ch.allow_skip = ctx->allow_skip;
        ch.route = RB_ROUTE_SELECT_CARDS;
        self->pending_choice = ch;
        self->has_pending_choice = 1;
        /* .filtered_indices(selected_cards.iter().map(|&i| i as usize)) */
        rb_resolver_store_pending_choice(self, gs);
        return 0; /* Ok(()) */
    }

    /* all_indices = selected_cards ++ hand_positions (dedup) */
    int all_indices[RB_MAX_HAND];
    int n_all = 0;
    for (int i = 0; i < self->n_selected_cards; i++) all_indices[n_all++] = (int)self->selected_cards[i];
    self->n_selected_cards = 0;
    for (int i = 0; i < n_hand_positions; i++) {
        int hp = hand_positions[i];
        int found = 0;
        for (int j = 0; j < n_all; j++) if (all_indices[j] == hp) { found = 1; break; }
        if (!found && n_all < RB_MAX_HAND) all_indices[n_all++] = hp;
    }

    /* ids_to_reveal = if count == 0 { &hand_positions } else { &all_indices } */
    const int *ids_to_reveal = (ctx->count == 0) ? hand_positions : all_indices;
    int n_ids = (ctx->count == 0) ? n_hand_positions : n_all;

    /* resolve_indices_to_ids(player, Zone::Hand, ids_to_reveal) */
    int revealed_card_ids[RB_MAX_HAND];
    int n_revealed_ids = 0;
    int pl = rb_resolve_target_player(gs, target);
    if (pl >= 0)
        n_revealed_ids = rb_resolve_indices_to_ids(gs, pl, "hand", ids_to_reveal, n_ids, revealed_card_ids);

    /* gs.push_revealed_card(...) for each */
    int source = rb_current_ability_source_card_id(gs);
    int owner = rb_target_player_index(target, rb_ability_master_id(gs));
    for (int i = 0; i < n_revealed_ids; i++)
        rb_resolver_push_revealed_card(gs, revealed_card_ids[i], source, 0, owner, "ability");

    /* rule log: [Turn N] <label> [[log_reveal_hand:n=K]] */
    if (n_revealed_ids > 0) {
        const char *player_label = rb_target_player_label(target, rb_ability_master_id(gs));
        rb_resolver_push_rule_log(gs, "[Turn %d] %s [[log_reveal_hand:n=%d]]",
                                  gs->turn, player_label, n_revealed_ids);
    }

    /* if !effect_started { push_revealed_cost_card(...) } */
    if (!effect_started) {
        int cost_source = rb_current_ability_source_card_id(gs);
        int cost_owner = rb_target_player_index(target, rb_ability_master_id(gs));
        for (int i = 0; i < n_revealed_ids; i++)
            rb_resolver_push_revealed_cost_card(gs, revealed_card_ids[i], cost_source, 0, cost_owner, "cost");
    }

    /* Optional multi-step reveal (count==0 && allow_skip && !effect_started && !all_indices.is_empty()) */
    if (ctx->count == 0 && ctx->allow_skip && !effect_started && n_all > 0) {
        self->n_selected_cards = 0;
        for (int i = 0; i < n_all; i++)
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = (int16_t)all_indices[i];

        int hand_len = (pl >= 0) ? gs->p[pl].hand.n : 0;
        int remaining_indices[RB_MAX_HAND];
        int n_remaining = 0;
        for (int i = 0; i < hand_len; i++) {
            int in = 0;
            for (int j = 0; j < n_all; j++) if (all_indices[j] == i) { in = 1; break; }
            if (!in && n_remaining < RB_MAX_HAND) remaining_indices[n_remaining++] = i;
        }
        if (n_remaining > 0) {
            RbChoice ch;
            memset(&ch, 0, sizeof(ch));
            ch.kind = RB_CHOICE_SELECT_CARD;
            strncpy(ch.zone, "hand", sizeof(ch.zone) - 1);
            ch.count = 0;
            strncpy(ch.description, "Select more cards to reveal from hand (or skip to finish)",
                    sizeof(ch.description) - 1);
            if (ctx->card_type) strncpy(ch.card_type, ctx->card_type, sizeof(ch.card_type) - 1);
            strncpy(ch.target, target, sizeof(ch.target) - 1);
            ch.allow_skip = 1;
            ch.route = RB_ROUTE_SELECT_CARDS;
            self->pending_choice = ch;
            self->has_pending_choice = 1;
            rb_resolver_store_pending_choice(self, gs);
            return 0; /* Ok(()) */
        }
    }

    /* self.clear_choice_state(gs); self.resume_pending_actions(gs) */
    return rb_resolver_clear_choice_state_and_resume(self, gs);
}

/* ── handle_revealed_cards_selection (choice.rs:1542) ── */
int rb_resolver_handle_revealed_cards_selection(RbAbilityResolver *self, GameState *gs,
                                                const RbSelectionContext *ctx,
                                                int (*validate_card)(int),
                                                const char *dst_str) {
    /* let mapped = ctx.mfi(&ctx.indices) */
    int mapped[RB_MAX_HAND];
    int n_mapped = rb_resolver_mfi(ctx, ctx->indices, ctx->n_indices, mapped, RB_MAX_HAND);

    /* let moved = self.move_from_revealed(gs, &mapped, validate_card, dst_str) */
    int16_t moved[RB_MAX_RECENTLY_MOVED];
    int n_moved = rb_resolver_move_from_revealed(self, gs, mapped, n_mapped,
                                                 validate_card, dst_str, moved, RB_MAX_RECENTLY_MOVED);

    /* if ctx.is_select_action { keep moved into self.selected_cards } */
    if (ctx->is_select_action) {
        for (int i = 0; i < n_moved; i++) {
            int found = 0;
            for (int j = 0; j < self->n_selected_cards; j++)
                if (self->selected_cards[j] == moved[i]) { found = 1; break; }
            if (!found && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = moved[i];
        }
    }

    /* self.moved_cards.extend(moved) */
    for (int i = 0; i < n_moved; i++) {
        int found = 0;
        for (int j = 0; j < self->n_moved_cards; j++)
            if (self->moved_cards[j] == moved[i]) { found = 1; break; }
        if (!found && self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
            self->moved_cards[self->n_moved_cards++] = moved[i];
    }

    /* resource_on_select — grants resource automatically on select. */
    const AbilityEffect *res = NULL;
    if (self->current_effect)
        res = (const AbilityEffect *)(intptr_t)rb_effect_resource_on_select_any(self->current_effect);
    if (res) {
        rb_set_chosen_target((AbilityEffect *)res,
                             (ctx->target_player_id && *ctx->target_player_id) ? ctx->target_player_id : "self");
        rb_execute_effect(gs, self->actor, (AbilityEffect *)res);
    }

    /* discard_remaining_any — dump leftover revealed cards into the waitroom. */
    int discard_remaining = self->current_effect ? rb_effect_discard_remaining_any(self->current_effect) : 0;
    if (discard_remaining) {
        int remaining[RB_MAX_RECENTLY_MOVED];
        int n_remaining = 0;
        for (int i = 0; i < gs->n_revealed; i++) {
            int in_moved = 0;
            for (int j = 0; j < n_moved; j++) if (moved[j] == (int16_t)gs->revealed_cards[i]) { in_moved = 1; break; }
            if (!in_moved && n_remaining < RB_MAX_RECENTLY_MOVED) remaining[n_remaining++] = gs->revealed_cards[i];
        }
        int pl = rb_resolve_target_player(gs, "self");
        if (pl >= 0) {
            RbPlayer *p = &gs->p[pl];
            for (int i = 0; i < n_remaining; i++) rb_waitroom_add(p, remaining[i]);
        }
        gs->n_revealed = 0;
    }

    /* Skip of an optional revealed_cards selection clears pending sequential actions. */
    int is_opponent_action = (ctx->target_player_id && strcmp(ctx->target_player_id, "opponent") == 0);
    if (ctx->n_indices == 0 && ctx->allow_skip && ctx->count > 0 &&
        !is_opponent_action && n_moved == 0) {
        rb_ability_queue_take_pending_actions(gs);
    }

    /* selection epilogue */
    return rb_resolver_handle_selection_epilogue(self, gs);
}
