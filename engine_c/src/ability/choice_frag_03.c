/* engine_c/src/ability/choice_frag_03.c
 * Port of engine/src/ability/choice.rs (lines ~1059-1397):
 *   AbilityResolver::build_reprompt
 *   AbilityResolver::handle_hand_selection
 * Self-contained fragment; depends on rb_* helpers from rabuka.h and on the
 * resolver-level helpers that are declared extern below (defined elsewhere).
 *
 * Rust-path notes are kept brief, marking where the C port diverges from the
 * exact Rust field layout (the C GameState/RbChoice structs are narrower than
 * the Rust AbilityResolver/Choice types).
 */

#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* ── Forward/opaque types ──────────────────────────────────────────────── */
struct RbExecutionContext;  /* ExecutionContext — opaque (mirrors Rust &ExecutionContext) */

/* SelectionContext — mirrors engine/src/ability/choice.rs::SelectionContext
   (the filter snapshot carried through a select_cards sub-choice). */
typedef struct {
    int   indices[RB_MAX_ZONE];
    int   n_indices;
    char  card_type[32];
    int   cost_limit;
    char  cost_limit_operator[16];
    int   cost_total;            /* -1 = None */
    char  cost_total_operator[16];
    char  group[32];
    char  characters[8][32];     /* Vec<String> */
    int   n_characters;
    char  target_player_id[32];  /* Option<String> */
    int   count;
    int   allow_skip;
    int   blind;
    int   is_reveal;
} RbSelectionContext;

/* RbResolver — mirrors the parts of Rust AbilityResolver touched by these two
   methods. `gs` is the owning GameState* (task: self.gs -> GameState*). The
   C GameState already carries keep_shuffle_under_*; here we keep the resolver
   copies so the fragment compiles against a self-contained struct. */
typedef struct {
    GameState *gs;
    int  keep_shuffle_under_phase;
    int  keep_shuffle_under_count;
    int  keep_shuffle_under_snapshots[2][RB_MAX_HAND];
    int  keep_shuffle_under_snapshot_n[2];
    int  keep_shuffle_under_selected[RB_MAX_HAND];
    int  keep_shuffle_under_selected_n;
    int  selected_cards[RB_MAX_RECENTLY_MOVED];
    int  n_selected_cards;
    int  moved_cards[RB_MAX_RECENTLY_MOVED];
    int  n_moved_cards;
    RbChoice pending_choice;      /* Option<Choice> */
    struct { char target[32]; } spawn_context;
} RbResolver;

/* ── In-fragment forward prototypes ────────────────────────────────────── */
static int  rb_player_hand_cards(const GameState *g, const char *target, int *out, int max);
static void rb_resolver_store_pending_choice(RbResolver *self, GameState *gs);

/* Resolver-level helpers called by name (defined elsewhere — do NOT define here). */
extern void rb_resolver_handle_selection_epilogue(GameState *g, const struct RbExecutionContext *ctx);
extern void rb_resolver_clear_choice_state_and_resume(GameState *g);
extern void rb_set_chosen_target(GameState *g, int cid);
extern void rb_resolver_move_non_selected_hand_to_deck_bottom(GameState *g, const char *player,
                                                              const int *snapshot, int snap_n);
extern void rb_ability_queue_take_pending_actions(GameState *g);
extern int  rb_resolver_execute_selected_cards_from_zone(
              GameState *g, const char *zone, const int *idxs, int n,
              const char *card_type, int cost_limit, const char *cost_limit_op,
              int cost_total, const char *cost_total_op, const char *group,
              const char **characters, int n_char, const char *target_pid);

/* ── build_reprompt ──────────────────────────────────────────────────────
 * Rust:
 *   Choice::select_cards(zone, count, desc, skip)
 *     .description_ja(Some(desc_ja))
 *     .card_type(ctx.card_type.clone())
 *     .cost_limit(ctx.cost_limit, ctx.cost_limit_operator.clone())
 *     .cost_total(ct, cto)
 *     .group(ctx.group.clone())
 *     .characters(ctx.characters.clone())
 *     .filtered_indices(fi)
 *     .target_player_id(tpid)
 * Returns the built RbChoice (Rust ChoiceBuilder is folded into RbChoice).
 */
RbChoice rb_resolver_build_reprompt(
    const RbResolver *self,
    const RbSelectionContext *ctx,
    const char *zone,
    int count,
    const char *desc,
    const char *desc_ja,
    int skip,
    const int *fi, int fi_n,
    const char *tpid,
    int ct,
    const char *cto)
{
    (void)self; (void)desc_ja; (void)fi; (void)fi_n; (void)ct; (void)cto;
    RbChoice c;
    memset(&c, 0, sizeof(c));
    c.kind = RB_CHOICE_SELECT_CARD;          /* Choice::select_cards */
    if (zone) strncpy(c.zone, zone, sizeof(c.zone) - 1);
    c.count = count > 0 ? count : 1;
    c.allow_skip = skip;
    if (desc) strncpy(c.description, desc, sizeof(c.description) - 1);
    if (ctx && ctx->card_type[0])
        strncpy(c.card_type, ctx->card_type, sizeof(c.card_type) - 1);
    if (ctx && ctx->group[0])
        strncpy(c.filter_group, ctx->group, sizeof(c.filter_group) - 1);
    /* cost_limit / cost_total have no RbChoice field in the C port — tracked by
       the caller via ctx; .filtered_indices(fi) is likewise not stored here. */
    const char *t = tpid;
    if (!t && ctx) t = ctx->target_player_id;
    if (t) strncpy(c.target, t, sizeof(c.target) - 1);  /* .target_player_id(tpid) */
    return c;
}

/* ── handle_hand_selection ───────────────────────────────────────────────
 * Rust: AbilityResolver::handle_hand_selection — resolves a SELECT_CARD choice
 * for the hand zone, possibly re-prompting for the remaining count, recording
 * the kept cards, or running the C6 keep-N-shuffle-rest flow.
 */
int rb_resolver_handle_hand_selection(
    RbResolver *self,
    GameState *gs,
    const RbSelectionContext *ctx,
    const struct RbExecutionContext *context,
    int (*validate_card)(int))
{
    self->gs = gs;  /* self.gs -> GameState* */

    /* mapped_indices = ctx.mfi(&ctx.indices)  (mfi maps queue indices -> local) */
    int mapped[RB_MAX_ZONE];
    int mapped_n = 0;
    for (int i = 0; i < ctx->n_indices && i < RB_MAX_ZONE; i++)
        mapped[mapped_n++] = ctx->indices[i];

    /* Rust: empty + !allow_skip + count>0 => Err(...) */
    int *hand_idx = mapped;        /* Rust: hand_idx = &mapped_indices */
    int hand_idx_n = mapped_n;
    if (mapped_n == 0 && !ctx->allow_skip && ctx->count > 0) {
        return -1;  /* "No cards selected from hand for required selection" */
    }

    /* C6 keep-N-shuffle-rest: selected hand cards are KEPT (not moved); the
       handler later shuffles the non-selected under the deck. */
    if (self->keep_shuffle_under_phase > 0) {
        const char *target = ctx->target_player_id[0] ? ctx->target_player_id : "self";
        int hand_cards[RB_MAX_HAND];
        int hand_n = rb_player_hand_cards(gs, target, hand_cards, RB_MAX_HAND);

        /* Record chosen hand POSITIONS to keep. */
        for (int i = 0; i < hand_idx_n; i++) {
            int idx = hand_idx[i];
            int found = 0;
            for (int j = 0; j < self->keep_shuffle_under_selected_n; j++)
                if (self->keep_shuffle_under_selected[j] == idx) { found = 1; break; }
            if (!found && self->keep_shuffle_under_selected_n < RB_MAX_HAND)
                self->keep_shuffle_under_selected[self->keep_shuffle_under_selected_n++] = idx;
        }

        int count = self->keep_shuffle_under_count;
        int available[RB_MAX_HAND];
        int avail_n = 0;
        for (int i = 0; i < hand_n; i++) {
            int in = 0;
            for (int j = 0; j < self->keep_shuffle_under_selected_n; j++)
                if (self->keep_shuffle_under_selected[j] == i) { in = 1; break; }
            if (!in) available[avail_n++] = i;
        }

        /* Need more kept cards from self — re-prompt. */
        if (hand_idx_n < count && hand_idx_n > 0 && avail_n > 0) {
            int remaining = count - hand_idx_n;  /* saturating_sub(clone..min(count)) */
            char desc[128];
            snprintf(desc, sizeof(desc),
                     "Select up to %d more card(s) from hand to keep", remaining);
            RbChoice c = rb_resolver_build_reprompt(
                self, ctx, "hand", remaining, desc,
                "手札からさらに選ぶ（スキップで終了）", 1,
                available, avail_n, target, ctx->cost_total,
                ctx->cost_total_operator[0] ? ctx->cost_total_operator : NULL);
            self->pending_choice = c;
            rb_resolver_store_pending_choice(self, gs);
            return 0;
        }

        /* Phase 1 done: move self's non-selected under, snapshot opponent, reprompt. */
        if (self->keep_shuffle_under_phase == 1) {
            int snap0[RB_MAX_HAND];
            int snap0_n = self->keep_shuffle_under_snapshot_n[0];
            for (int i = 0; i < snap0_n; i++)
                snap0[i] = self->keep_shuffle_under_snapshots[0][i];
            rb_resolver_move_non_selected_hand_to_deck_bottom(gs, "self", snap0, snap0_n);
            self->keep_shuffle_under_selected_n = 0;

            int opp_hand[RB_MAX_HAND];
            int opp_n = rb_player_hand_cards(gs, "opponent", opp_hand, RB_MAX_HAND);
            int s1_n = opp_n < RB_MAX_HAND ? opp_n : RB_MAX_HAND;
            for (int i = 0; i < s1_n; i++)
                self->keep_shuffle_under_snapshots[1][i] = opp_hand[i];
            self->keep_shuffle_under_snapshot_n[1] = s1_n;

            int pick = (count < opp_n) ? count : opp_n;  /* (count).min(opp_hand_len) */
            RbChoice c;
            memset(&c, 0, sizeof(c));
            c.kind = RB_CHOICE_SELECT_CARD;
            strncpy(c.zone, "hand", sizeof(c.zone) - 1);
            c.count = pick > 0 ? pick : 1;
            c.allow_skip = 1;
            strncpy(c.target, "opponent", sizeof(c.target) - 1);
            snprintf(c.description, sizeof(c.description),
                     "Select up to %d card(s) to keep", count);

            self->keep_shuffle_under_phase = 2;
            strncpy(self->spawn_context.target, "opponent",
                    sizeof(self->spawn_context.target) - 1);
            self->pending_choice = c;
            rb_resolver_store_pending_choice(self, gs);
            return 0;
        }

        /* Phase 2: move opponent's non-selected under, draw 3 for both, clear. */
        if (self->keep_shuffle_under_phase == 2) {
            int snap1[RB_MAX_HAND];
            int snap1_n = self->keep_shuffle_under_snapshot_n[1];
            for (int i = 0; i < snap1_n; i++)
                snap1[i] = self->keep_shuffle_under_snapshots[1][i];
            rb_resolver_move_non_selected_hand_to_deck_bottom(gs, "opponent", snap1, snap1_n);

            self->keep_shuffle_under_phase = 0;
            self->keep_shuffle_under_snapshot_n[0] = 0;
            self->keep_shuffle_under_snapshot_n[1] = 0;
            self->keep_shuffle_under_selected_n = 0;
            self->spawn_context.target[0] = 0;

            /* Sequential second action "draw 3 for both" performed directly to
               avoid the corrupted [select,draw] pending restarting keep_shuffle. */
            rb_draw_cards_for_player(&gs->p[0], 3, "deck", "hand", NULL, 0, NULL, NULL, -1);
            rb_draw_cards_for_player(&gs->p[1], 3, "deck", "hand", NULL, 0, NULL, NULL, -1);
            rb_ability_queue_take_pending_actions(gs);  /* gs.ability_queue.take_pending_actions() */
        }
        rb_resolver_handle_selection_epilogue(gs, context);  /* self.handle_selection_epilogue(gs, context) */
        return 0;
    }

    /* ── Non-keep-shuffle hand selection ── */
    if (hand_idx_n > 0 || ctx->allow_skip) {
        /* Need more cards to reach ctx.count. */
        if (hand_idx_n > 0 && ctx->count > 0 && hand_idx_n < ctx->count) {
            const char *target = ctx->target_player_id[0] ? ctx->target_player_id : "self";
            int hand_cards[RB_MAX_HAND];
            int hand_n = rb_player_hand_cards(gs, target, hand_cards, RB_MAX_HAND);

            int all_hand_idxs[RB_MAX_ZONE];
            int all_n = hand_idx_n;
            for (int i = 0; i < hand_idx_n; i++) all_hand_idxs[i] = hand_idx[i];

            /* Fold already-selected card ids back into the candidate positions. */
            for (int i = 0; i < self->n_selected_cards; i++) {
                int cid = self->selected_cards[i];
                for (int h = 0; h < hand_n; h++) {
                    if (hand_cards[h] == cid) {
                        int present = 0;
                        for (int k = 0; k < all_n; k++)
                            if (all_hand_idxs[k] == h) { present = 1; break; }
                        if (!present && all_n < RB_MAX_ZONE) all_hand_idxs[all_n++] = h;
                    }
                }
            }

            int new_ids[RB_MAX_HAND];
            int new_n = 0;
            for (int i = 0; i < all_n; i++) {
                int idx = all_hand_idxs[i];
                if (idx >= 0 && idx < hand_n) new_ids[new_n++] = hand_cards[idx];
            }
            for (int i = 0; i < new_n; i++) {
                int present = 0;
                for (int k = 0; k < self->n_selected_cards; k++)
                    if (self->selected_cards[k] == new_ids[i]) { present = 1; break; }
                if (!present && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = new_ids[i];
                rb_set_chosen_target(gs, new_ids[i]);  /* record chosen target id */
            }

            int remaining = ctx->count - hand_idx_n;
            int available[RB_MAX_HAND];
            int avail_n = 0;
            for (int i = 0; i < hand_n; i++) {
                int present = 0;
                for (int k = 0; k < all_n; k++)
                    if (all_hand_idxs[k] == i) { present = 1; break; }
                if (!present) available[avail_n++] = i;
            }
            int *fi = avail_n > 0 ? available : NULL;

            char desc[128];
            snprintf(desc, sizeof(desc), "Select %d more card(s) from hand%s",
                     remaining, ctx->blind ? " (blind)" : "");
            char desc_ja[64];
            snprintf(desc_ja, sizeof(desc_ja), "手札からさらに%d枚選択%s",
                     remaining, ctx->blind ? "（控えに選択）" : "");

            RbChoice c = rb_resolver_build_reprompt(
                self, ctx, "hand", remaining, desc, desc_ja, 0,
                fi, avail_n, target, ctx->cost_total,
                ctx->cost_total_operator[0] ? ctx->cost_total_operator : NULL);
            /* .blind(ctx.blind) / .is_reveal(ctx.is_reveal): RbChoice has no such
               fields in the C port — dropped with this note. */
            self->pending_choice = c;
            rb_resolver_store_pending_choice(self, gs);
            return 0;
        }

        /* count == 0 && allow_skip: execute the chosen, then re-prompt to keep going. */
        if (hand_idx_n > 0 && ctx->count == 0 && ctx->allow_skip) {
            const char *target = ctx->target_player_id[0] ? ctx->target_player_id : "self";
            int old_hand[RB_MAX_HAND];
            int old_n = rb_player_hand_cards(gs, target, old_hand, RB_MAX_HAND);

            int moved_ids[RB_MAX_HAND];
            int moved_n = 0;
            for (int i = 0; i < hand_idx_n; i++) {
                int idx = hand_idx[i];
                if (idx >= 0 && idx < old_n) moved_ids[moved_n++] = old_hand[idx];
            }

            rb_resolver_execute_selected_cards_from_zone(
                gs, "hand", hand_idx, hand_idx_n,
                ctx->card_type[0] ? ctx->card_type : NULL,
                ctx->cost_limit, ctx->cost_limit_operator[0] ? ctx->cost_limit_operator : NULL,
                ctx->cost_total, ctx->cost_total_operator[0] ? ctx->cost_total_operator : NULL,
                ctx->group[0] ? ctx->group : NULL, NULL, 0, target);

            for (int i = 0; i < moved_n; i++) {
                int present = 0;
                for (int k = 0; k < self->n_selected_cards; k++)
                    if (self->selected_cards[k] == moved_ids[i]) { present = 1; break; }
                if (!present && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = moved_ids[i];
                rb_set_chosen_target(gs, moved_ids[i]);
            }

            int hand_cards[RB_MAX_HAND];
            int hand_n = rb_player_hand_cards(gs, target, hand_cards, RB_MAX_HAND);
            int include_idxs[RB_MAX_HAND];
            int inc_n = 0;
            for (int i = 0; i < hand_n; i++)
                if (validate_card && validate_card(hand_cards[i])) include_idxs[inc_n++] = i;
            int *fi = inc_n > 0 ? include_idxs : NULL;

            RbChoice c = rb_resolver_build_reprompt(
                self, ctx, "hand", 0,
                "Select more card(s) from hand (or skip to finish)",
                "手札からさらに選択（スキップで終了）", 1,
                fi, inc_n, target, ctx->cost_total,
                ctx->cost_total_operator[0] ? ctx->cost_total_operator : NULL);
            self->pending_choice = c;
            rb_resolver_store_pending_choice(self, gs);
            return 0;
        }

        /* Otherwise execute all selected cards now. */
        int all_idxs[RB_MAX_ZONE];
        int all_n = hand_idx_n;
        for (int i = 0; i < hand_idx_n; i++) all_idxs[i] = hand_idx[i];

        if (self->n_selected_cards > 0) {
            const char *target = ctx->target_player_id[0] ? ctx->target_player_id : "self";
            int hand_cards[RB_MAX_HAND];
            int hand_n = rb_player_hand_cards(gs, target, hand_cards, RB_MAX_HAND);
            for (int h = 0; h < hand_n; h++) {
                int cid = hand_cards[h];
                int in = 0;
                for (int k = 0; k < self->n_selected_cards; k++)
                    if (self->selected_cards[k] == cid) { in = 1; break; }
                if (in) {
                    int present = 0;
                    for (int j = 0; j < all_n; j++)
                        if (all_idxs[j] == h) { present = 1; break; }
                    if (!present && all_n < RB_MAX_ZONE) all_idxs[all_n++] = h;
                }
            }
        }

        rb_resolver_execute_selected_cards_from_zone(
            gs, "hand", all_idxs, all_n,
            ctx->card_type[0] ? ctx->card_type : NULL,
            ctx->cost_limit, ctx->cost_limit_operator[0] ? ctx->cost_limit_operator : NULL,
            ctx->cost_total, ctx->cost_total_operator[0] ? ctx->cost_total_operator : NULL,
            ctx->group[0] ? ctx->group : NULL, NULL, 0,
            ctx->target_player_id[0] ? ctx->target_player_id : NULL);
        self->n_selected_cards = 0;  /* self.selected_cards.clear() */
    }

    /* Optional-cost bookkeeping / skip of empty required selection. */
    if (ctx->allow_skip) {
        /* Rust: entry.optional_cost_result = Some(...). RbQueueEntry in the C
           port has no such field — omitted (noted for parity). */
        int is_opponent = ctx->target_player_id[0] &&
                          strcmp(ctx->target_player_id, "opponent") == 0;
        if (hand_idx_n == 0 && ctx->count > 0 && !is_opponent && self->n_moved_cards == 0) {
            rb_ability_queue_take_pending_actions(gs);  /* mapped to clear+resume */
            rb_resolver_clear_choice_state_and_resume(gs);
        }
    }

    rb_resolver_handle_selection_epilogue(gs, context);  /* self.handle_selection_epilogue(gs, context) */
    return 0;
}

/* ── static helpers ───────────────────────────────────────────────────── */

/* Mirror AbilityResolver::store_pending_choice — copy the resolved RbChoice into
   the GameState ability queue so rb_resume_with_choice can answer it. */
static void rb_resolver_store_pending_choice(RbResolver *self, GameState *gs) {
    gs->queue.pending = self->pending_choice;     /* RbChoice copy */
    gs->queue.has_pending = 1;
    /* actor resolves from the choice's target player id. */
    gs->queue.actor = (strcmp(self->pending_choice.target, "opponent") == 0) ? 1 : 0;
    gs->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

/* Read a target player's current hand card ids into out. */
static int rb_player_hand_cards(const GameState *g, const char *target, int *out, int max) {
    int pl = rb_resolve_target_player(g, target);
    if (pl < 0 || pl > 1) return 0;
    int n = g->p[pl].hand.n;
    if (n > max) n = max;
    for (int i = 0; i < n; i++) out[i] = g->p[pl].hand.cards[i];
    return n;
}
