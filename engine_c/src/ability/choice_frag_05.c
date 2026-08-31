/* engine_c/src/ability/choice_frag_05.c
 *
 * Port of engine/src/ability/choice.rs fragments:
 *   - handle_success_live_zone_selection  (Rust ~lines 1603-1649)
 *   - handle_entry_cost_reveal            (Rust ~lines 1651-1733)
 *
 * Rust-path summary:
 *   - `self` is the AbilityResolver; in this C port the resolver's `gs`
 *     field is passed directly as `GameState *gs` (self.gs -> GameState*).
 *   - Resolver-only state (moved_cards / recently_moved batch, choice-state
 *     clearing, pending-action resume) is funneled through rb_* helpers and
 *     the resolver methods called by name:
 *       rb_resolver_handle_selection_epilogue
 *       rb_resolver_clear_choice_state_and_resume
 *       rb_set_chosen_target
 *   - util::resolve_indices_to_ids / move_cards / card_matches_* map to the
 *     rb_* helpers declared in rabuka.h.
 */

#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* ── Local context types (mirror engine/src/ability/types.rs) ── */
typedef struct RbEntryCost {
    const char *card_type;        /* card_type_any(), nullable */
    const char **characters;      /* characters_any(), nullable */
    int         n_characters;
    const char **group_names;     /* group_names_any(), nullable */
    int         n_group_names;
    int         cost_limit;       /* cost_limit_any(); -1 = None */
    const char *cost_limit_op;    /* comparison operator for cost_limit */
    int         count;            /* Option<usize>; -1 = None (default 1) */
} RbEntryCost;

typedef struct RbSelectionContext {
    const int *indices;           /* chosen positions (hand/deck/zone) */
    int        n_indices;
    const char *destination;      /* nullable; destination zone override */
    const char *target_player_id; /* nullable; defaults to "self" */
    /* mfi(indices) -> mapped ids (mirror SelectionContext::mfi) */
    int (*mfi)(const int *indices, int n_indices, int *out);
} RbSelectionContext;

typedef struct RbExecutionContext {
    int actor;
    int host_cid;
} RbExecutionContext;

/* ── Forward prototypes for resolver-level / engine helpers used here ──
   (declared, not defined: three are resolver methods per the task brief) */
void rb_resolver_handle_selection_epilogue(GameState *g);
void rb_resolver_clear_choice_state_and_resume(GameState *g);
void rb_set_chosen_target(GameState *g, int card_id);

int  rb_entry_destination(const GameState *g, char *out, size_t n);
int  rb_entry_cost(const GameState *g, RbEntryCost *out);
int  rb_current_ability_source_card_id(const GameState *g);
void rb_set_recently_moved_cards(GameState *g, const int *ids, int n);
void rb_push_rule_log(GameState *g, const char *msg);
const char *rb_card_name(int card_id);
void rb_push_revealed_card(GameState *g, int card_id, int source, int hidden,
                           int owner, const char *kind);
void rb_push_revealed_cost_card(GameState *g, int card_id, int source, int hidden,
                                int owner, const char *kind);
AbilityEffect *rb_ability_queue_take_pending_actions(GameState *g, int *n_out);
void rb_ability_queue_set_pending_actions(GameState *g, AbilityEffect *arr, int n);

/* Default mfi: pass indices through unchanged when the context provides none. */
static int rb_selection_context_mfi_passthrough(const int *indices, int n_indices,
                                                int *out) {
    int n = n_indices < RB_MAX_ZONE ? n_indices : RB_MAX_ZONE;
    for (int i = 0; i < n; i++) out[i] = indices[i];
    return n;
}

/* ════════════════════════════════════════════════════════════════════════
 * rb_resolver_handle_success_live_zone_selection
 *   Rust: AbilityResolver::handle_success_live_zone_selection
 *   (choice.rs ~1603-1649)
 * ════════════════════════════════════════════════════════════════════════ */
void rb_resolver_handle_success_live_zone_selection(
        GameState *gs,
        const RbSelectionContext *ctx,
        int (*validate_card)(int)) {
    /* Rust: let mapped = ctx.mfi(&ctx.indices); */
    int mapped[RB_MAX_ZONE];
    int n_mapped = 0;
    if (ctx && ctx->indices) {
        if (ctx->mfi)
            n_mapped = ctx->mfi(ctx->indices, ctx->n_indices, mapped);
        else
            n_mapped = rb_selection_context_mfi_passthrough(ctx->indices,
                                                            ctx->n_indices, mapped);
    }

    /* Rust: edst = gs.entry_destination(); dst_str = destination.or(edst)
     *       .unwrap_or(Zone::Discard); */
    char edst[32];
    int has_edst = rb_entry_destination(gs, edst, sizeof edst);
    const char *dst_str = "discard";
    if (ctx && ctx->destination && ctx->destination[0])
        dst_str = ctx->destination;
    else if (has_edst)
        dst_str = edst;

    /* Rust: let target = ctx.target_player_id.unwrap_or("self"); */
    const char *target = (ctx && ctx->target_player_id && ctx->target_player_id[0])
                             ? ctx->target_player_id : "self";
    int pl = rb_resolve_target_player(gs, target);

    /* Rust: resolve_indices_to_ids(player, SuccessLiveZone, &mapped) */
    int card_ids[RB_MAX_ZONE];
    int n_ids = rb_resolve_indices_to_ids(gs, pl, "success_live_zone",
                                          mapped, n_mapped, card_ids);

    /* Rust: valid_ids = card_ids.filter(validate_card) */
    int valid_ids[RB_MAX_ZONE];
    int n_valid = 0;
    for (int i = 0; i < n_ids; i++) {
        if (validate_card && validate_card(card_ids[i]))
            valid_ids[n_valid++] = card_ids[i];
    }

    if (n_valid > 0) {
        /* Rust: move_cards(player, valid_ids, SuccessLiveZone, dst_str, None, db) */
        int mc = rb_move_cards(gs, pl, valid_ids, n_valid,
                               "success_live_zone", dst_str, -1);
        if (mc > 0) {
            /* Rust: self.moved_cards = valid_ids;
             *       gs.set_recently_moved_batch(valid_ids, Some(SuccessLiveZone)); */
            rb_set_recently_moved_cards(gs, valid_ids, n_valid);
            for (int i = 0; i < n_valid; i++)
                rb_set_chosen_target(gs, valid_ids[i]);
        }
    }

    /* Rust: self.clear_choice_state(gs) */
    rb_resolver_clear_choice_state_and_resume(gs);

    /* Rust: pending = gs.ability_queue.take_pending_actions();
     *       filtered = pending.filter(|c| c.source != Some(SuccessLiveZone));
     *       gs.ability_queue.set_pending_actions(filtered); */
    int n_pend = 0;
    AbilityEffect *pending = rb_ability_queue_take_pending_actions(gs, &n_pend);
    if (pending && n_pend > 0) {
        int k = 0;
        for (int i = 0; i < n_pend; i++) {
            if (pending[i].source &&
                strcmp(pending[i].source, "success_live_zone") == 0)
                continue; /* drop actions sourced from the success live zone */
            pending[k++] = pending[i];
        }
        rb_ability_queue_set_pending_actions(gs, pending, k);
    }

    /* Rust: self.resume_pending_actions(gs) */
    rb_resolver_handle_selection_epilogue(gs);
}

/* ════════════════════════════════════════════════════════════════════════
 * rb_resolver_handle_entry_cost_reveal
 *   Rust: AbilityResolver::handle_entry_cost_reveal
 *   (choice.rs ~1651-1733)
 * ════════════════════════════════════════════════════════════════════════ */
void rb_resolver_handle_entry_cost_reveal(
        GameState *gs,
        const RbSelectionContext *ctx,
        const RbExecutionContext *context) {
    (void)context; /* ExecutionContext carried for parity; finalize_choice below */

    /* Rust: let Some(cost) = gs.entry_cost().cloned() else { clear; Err } */
    RbEntryCost cost;
    if (!rb_entry_cost(gs, &cost)) {
        /* Rust: log::error!(...); self.clear_choice_state(gs);
         *       return Err("entry cost reveal without entry cost"); */
        rb_resolver_clear_choice_state_and_resume(gs);
        return;
    }

    /* Rust: let player = gs.resolve_target_player("self"); */
    int pl = rb_resolve_target_player(gs, "self");
    RbPlayer *player = &gs->p[pl];

    /* Rust: card_ids = indices.filter_map(|idx| if idx < hand.len {
     *            cid = hand[idx]; if passes { Some(cid) } else { None } }) */
    int card_ids[RB_MAX_ZONE];
    int n_ids = 0;
    for (int j = 0; j < ctx->n_indices; j++) {
        int idx = ctx->indices[j];
        if (idx >= 0 && idx < player->hand.n) {
            int cid = player->hand.cards[idx];
            int passes = 1;

            /* Rust: card_matches_type(cid, cost.card_type_any()) */
            if (passes && cost.card_type &&
                !rb_card_matches_type(cid, cost.card_type))
                passes = 0;

            /* Rust: card_matches_characters(cid, cost.characters_any()) */
            if (passes && cost.characters && cost.n_characters > 0 &&
                !rb_card_matches_characters(cid, cost.characters, cost.n_characters))
                passes = 0;

            /* Rust: match cost.group_names_any() {
             *         Some(groups) => groups.any(|g| card_matches_group_str(...)),
             *         None => true } */
            if (passes && cost.group_names && cost.n_group_names > 0) {
                int any = 0;
                for (int gi = 0; gi < cost.n_group_names; gi++) {
                    if (rb_card_matches_group_str(cid, cost.group_names[gi])) {
                        any = 1;
                        break;
                    }
                }
                if (!any) passes = 0;
            }

            /* Rust: card_matches_cost_limit(cid, cost.cost_limit_any()) */
            if (passes && cost.cost_limit >= 0 &&
                !rb_card_matches_cost_limit(cid, cost.cost_limit,
                    cost.cost_limit_op ? cost.cost_limit_op : ">="))
                passes = 0;

            if (passes)
                card_ids[n_ids++] = cid;
        }
    }

    /* Rust: let count = cost.count.unwrap_or(1);
     *       if card_ids.len() < count { Err("Not enough valid cards...") } */
    int count = (cost.count >= 0) ? cost.count : 1;
    if (n_ids < count)
        return;

    if (n_ids > 0) {
        /* Rust: names = card_ids.map(|id| db.get_card(id).name).collect() */
        char names_buf[512];
        names_buf[0] = '\0';
        for (int i = 0; i < n_ids; i++) {
            const char *nm = rb_card_name(card_ids[i]);
            if (nm) {
                size_t cur = strlen(names_buf);
                if (cur)
                    strncat(names_buf, ", ", sizeof names_buf - cur - 1);
                strncat(names_buf, nm, sizeof names_buf - strlen(names_buf) - 1);
            }
        }

        /* Rust: player_num = if self==player1 {1} else {2}; (self -> index 0/1) */
        int turn = gs->turn;
        int player_num = pl + 1;
        if (names_buf[0]) {
            char msg[640];
            snprintf(msg, sizeof msg,
                     "[Turn %d] P%d [[log_reveal_cost]]: %s",
                     turn, player_num, names_buf);
            rb_push_rule_log(gs, msg);
        }
    }

    /* Rust: cost_source = gs.current_ability_source_card_id();
     *       cost_owner = if self==player1 {Some(0)} else {Some(1)}; */
    int cost_source = rb_current_ability_source_card_id(gs);
    int cost_owner = pl;

    /* Rust: for card_id in card_ids {
     *           gs.push_revealed_card(...); gs.push_revealed_cost_card(...); } */
    for (int i = 0; i < n_ids; i++) {
        rb_push_revealed_card(gs, card_ids[i], cost_source, 0, cost_owner, "cost");
        rb_push_revealed_cost_card(gs, card_ids[i], cost_source, 0, cost_owner, "cost");
        rb_set_chosen_target(gs, card_ids[i]);
    }

    /* Rust: self.finalize_choice(gs, context) */
    rb_resolver_clear_choice_state_and_resume(gs);
}
