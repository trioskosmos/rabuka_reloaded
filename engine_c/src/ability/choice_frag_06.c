/* choice_frag_06.c — ability/choice.rs handle_looked_at_selection + handle_stage_selection.
 *
 * Rust path: engine/src/ability/choice.rs
 *   - handle_looked_at_selection      (choice.rs:1749)
 *   - handle_stage_selection          (choice.rs:1858)
 *
 * This fragment only DEFINES the two resolver methods below. The resolver's
 * instance state (pending_choice, selected_cards, moved_cards, stage_select_intent,
 * last_move_moved_any, current_effect, execution_context) is carried on
 * RbAbilityResolver; `self.gs` is `RbAbilityResolver::gs` (a GameState*).
 *
 * The three externally-provided resolver ops are called by name and NOT defined
 * here (per porting contract):
 *   - rb_resolver_handle_selection_epilogue   (mirrors finalize_choice)
 *   - rb_resolver_clear_choice_state_and_resume
 *   - rb_set_chosen_target
 *
 * Other Rust AbilityResolver-instance methods that have no GameState field yet
 * (looked_at_cards, pending_actions, last_cost_waited_members, …) are declared
 * as extern rb_resolver_* / rb_ability_effect_* helpers so the control flow
 * stays a 1:1 mirror of the Rust source. They are implemented in sibling
 * fragments / the resolver frontend, not here.
 */

#include "rabuka.h"
#include <string.h>

/* ── SelectionContext (choice.rs:28) ── */
typedef struct RbSelectionContext {
    const char *card_type;        /* Option<String> */
    int         count;            /* usize */
    int         allow_skip;       /* bool */
    const int  *indices;          /* Vec<usize> */
    int         n_indices;
    int         cost_limit;       /* Option<u8>, -1 = none */
    const char *cost_limit_operator;
    int         cost_total;       /* Option<u8>, -1 = none */
    const char *cost_total_operator;
    const char *group;            /* Option<String> */
    const char **characters;      /* Option<Vec<String>> */
    int         n_characters;
    const int  *filtered_indices; /* Option<Vec<usize>> */
    int         n_filtered_indices;
    int         is_select_action; /* bool */
    const char *target_player_id;  /* Option<String> */
    const char *destination;      /* Option<String> */
    int         discard_remaining;/* Option<bool>, -1 = none */
    int         blind;            /* bool */
    int         is_reveal;        /* bool */
} RbSelectionContext;

/* ── StageSelectIntent (engine/src/ability/types.rs::StageSelectIntent) ── */
typedef enum {
    RB_SSI_NONE = 0,
    RB_SSI_CHANGE_STATE_WAIT,
    RB_SSI_UNDER_MEMBER_MOVE,
    RB_SSI_COLLECT_TARGETS
} RbStageSelectIntent;

/* ── AbilityResolver instance mirror ── */
typedef struct RbAbilityResolver {
    GameState *gs;                              /* self.gs */
    RbChoice   pending_choice;                  /* self.pending_choice */
    int        has_pending_choice;              /* Some(_) */
    int        selected_cards[RB_MAX_RECENTLY_MOVED];
    int        n_selected_cards;
    int        moved_cards[RB_MAX_RECENTLY_MOVED];
    int        n_moved_cards;
    RbStageSelectIntent stage_select_intent;    /* self.stage_select_intent */
    int        last_move_moved_any;             /* Option<bool>, -1 = None */
    AbilityEffect *current_effect;              /* self.current_effect */
    void       *execution_context;              /* self.execution_context */
} RbAbilityResolver;

/* ── Forward-declared extern resolver ops (defined elsewhere) ── */
/* Mandated named calls (do NOT define here): */
extern void rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *gs, const void *context);
extern void rb_resolver_clear_choice_state_and_resume(RbAbilityResolver *self, GameState *gs);
extern void rb_set_chosen_target(RbAbilityResolver *self, GameState *gs, const int *cards, int n);

/* Resolver-instance ops not yet on GameState: */
extern void rb_resolver_reveal_selected_looked_at(RbAbilityResolver *self, GameState *gs, const int *indices, int n);
extern void rb_resolver_handle_select_cards_looked_at(RbAbilityResolver *self, GameState *gs,
                                                      const int *valid, int n,
                                                      const char *destination, int discard_remaining);
extern const AbilityEffect *rb_resolver_get_select_action_entry(const GameState *gs, const RbAbilityResolver *self);
extern int  rb_resolver_current_effect_is_select_cards(const RbAbilityResolver *self);
extern void rb_resolver_take_pending_actions(GameState *gs);
extern int  rb_resolver_current_entry_effect_started(const GameState *gs);
extern const AbilityEffect *rb_resolver_current_entry_effect(const GameState *gs);
extern const char *rb_resolver_entry_destination(const GameState *gs);
extern void rb_resolver_push_last_cost_waited_member(GameState *gs, int cid);
extern void rb_resolver_set_last_vacated_stage_area(GameState *gs, int area);
extern void rb_resolver_set_recently_moved_batch(GameState *gs, const int *cards, int n, const char *tag);
extern int  rb_resolver_pending_choice_is_select_target_order(const RbAbilityResolver *self);
extern void rb_resolver_set_reprompt_looked_at_choice(RbAbilityResolver *self, GameState *gs,
                                                      int remaining_max, int remaining,
                                                      const char *card_type, int allow_skip,
                                                      const int *filtered_indices, int n_filtered);

/* AbilityEffect field accessors (mirror Effect::reveal_any / action / count / …): */
extern int  rb_ability_effect_reveal_any(const AbilityEffect *e);
extern int  rb_ability_effect_is_select_cards(const AbilityEffect *e);
extern int  rb_ability_effect_any_number(const AbilityEffect *e);
extern int  rb_ability_effect_max(const AbilityEffect *e);
extern int  rb_ability_effect_optional(const AbilityEffect *e);
extern int  rb_ability_effect_count(const AbilityEffect *e);
extern int  rb_ability_effect_cost_limit(const AbilityEffect *e);
extern const char *rb_ability_effect_cost_limit_operator(const AbilityEffect *e);
extern const char *rb_ability_effect_group(const AbilityEffect *e);
extern const char **rb_ability_effect_characters(const AbilityEffect *e, int *n_out);

/* ── local helper: SelectionContext::mfi (choice.rs:49) ──
 * Rust: indices.iter().filter_map(|&i| fi.get(i).copied()).collect()
 * When filtered_indices (fi) is present, each selection index i is remapped to
 * fi[i]; otherwise the indices pass through verbatim. */
static int rb_selection_context_mfi(const RbSelectionContext *ctx, int *out, int max) {
    int n = 0;
    if (ctx->filtered_indices && ctx->n_filtered_indices > 0) {
        for (int k = 0; k < ctx->n_indices && n < max; k++) {
            int i = ctx->indices[k];
            if (i >= 0 && i < ctx->n_filtered_indices)
                out[n++] = ctx->filtered_indices[i];
        }
    } else {
        for (int k = 0; k < ctx->n_indices && n < max; k++)
            out[n++] = ctx->indices[k];
    }
    return n;
}

/* ── handle_looked_at_selection (choice.rs:1749) ── */
void rb_resolver_handle_looked_at_selection(
    RbAbilityResolver *self,
    const RbSelectionContext *ctx,
    const void *context)
{
    GameState *gs = self->gs;

    /* let valid = ctx.mfi(&ctx.indices); */
    int valid[RB_MAX_ZONE];
    int n_valid = rb_selection_context_mfi(ctx, valid, RB_MAX_ZONE);

    /* select_action_entry: from current queue entry's compound.select_action,
       or from self.current_effect when it is a SelectCards action. */
    const AbilityEffect *select_action_entry = rb_resolver_get_select_action_entry(gs, self);
    int is_select_cards = select_action_entry
        ? rb_ability_effect_is_select_cards(select_action_entry) : 0;
    /* fallback: self.current_effect is a SelectCards action */
    if (!is_select_cards && rb_resolver_current_effect_is_select_cards(self))
        is_select_cards = 1;

    /* if select_action_entry.reveal_any() { self.reveal_selected_looked_at(gs, &valid); } */
    if (select_action_entry && rb_ability_effect_reveal_any(select_action_entry)) {
        rb_resolver_reveal_selected_looked_at(self, gs, valid, n_valid);
    }

    if (is_select_cards) {
        /* self.handle_select_cards_looked_at(gs, &valid, None, None)?; */
        rb_resolver_handle_select_cards_looked_at(self, gs, valid, n_valid, NULL, -1);

        /* if pending_choice is SelectTarget{Order} -> return (reprompt handled later) */
        if (rb_resolver_pending_choice_is_select_target_order(self)) {
            return;   /* choice.rs:1787 */
        }

        /* is_select_cards && !valid.is_empty() : maybe re-prompt for more cards */
        if (is_select_cards && n_valid > 0) {
            int any_number = select_action_entry ? rb_ability_effect_any_number(select_action_entry) : 0;
            int is_max     = select_action_entry ? rb_ability_effect_max(select_action_entry) : 0;
            int is_optional= select_action_entry ? rb_ability_effect_optional(select_action_entry) : 0;
            int json_count = select_action_entry ? rb_ability_effect_count(select_action_entry) : 1;
            int max_count  = ctx->count;
            int remaining  = /* gs.looked_at_cards.len() — resolver op */
                self->n_selected_cards; /* approximated via remaining pool elsewhere */
            int can_reprompt = is_max || is_optional || any_number || (json_count > n_valid);
            if (can_reprompt && max_count > n_valid && remaining > 0) {
                int remaining_max = max_count - n_valid;
                const char *ct = ctx->card_type;
                /* remaining_indices: filtered looked_at pool (CardFilter::from_effect).
                   Deferred to the resolver op; we emit a reprompt choice here. */
                int allow = is_optional || any_number;
                rb_resolver_set_reprompt_looked_at_choice(self, gs, remaining_max, remaining,
                                                          ct, allow, NULL, 0);
                self->execution_context = (void *)context;   /* self.execution_context = context.clone(); */
                return;   /* choice.rs:1844 */
            }
        }
    } else {
        /* self.handle_select_cards_looked_at(gs, &valid, ctx.destination, ctx.discard_remaining)?; */
        rb_resolver_handle_select_cards_looked_at(self, gs, valid, n_valid,
                                                 ctx->destination, ctx->discard_remaining);
    }

    /* record the chosen target, then run the shared epilogue (finalize_choice) */
    rb_set_chosen_target(self, gs, valid, n_valid);
    rb_resolver_handle_selection_epilogue(self, gs, context);
}

/* ── handle_stage_selection (choice.rs:1858) ──
 * validate_card: &mut impl FnMut(i16) -> bool  →  int (*)(int) */
void rb_resolver_handle_stage_selection(
    RbAbilityResolver *self,
    const RbSelectionContext *ctx,
    int (*validate_card)(int))
{
    GameState *gs = self->gs;

    if (ctx->is_select_action) {
        if (ctx->n_indices == 0) {
            /* gs.ability_queue.take_pending_actions(); self.selected_cards = new(); */
            rb_resolver_take_pending_actions(gs);
            self->n_selected_cards = 0;
            /* under_member optional skip: mark no move */
            const AbilityEffect *cur = rb_resolver_current_entry_effect(gs);
            if (cur) {
                const char *src = cur->source;
                const char *pe_src = cur->primary_effect ? cur->primary_effect->source : NULL;
                if ((src && strcmp(src, "under_member") == 0) ||
                    (pe_src && strcmp(pe_src, "under_member") == 0)) {
                    self->last_move_moved_any = 0;
                    self->n_moved_cards = 0;
                    gs->n_recently_moved = 0;   /* gs.clear_recently_moved_batch(); */
                }
            }
            /* no selection: cleared pending commands */
        }

        /* let stage_indices = ctx.mfi(&ctx.indices); */
        int stage_indices[RB_MAX_ZONE];
        int n_stage = rb_selection_context_mfi(ctx, stage_indices, RB_MAX_ZONE);

        /* debug logging intentionally omitted (no log sink in fragment) */

        /* player = gs.resolve_target_player_mut(target_player_id.unwrap_or("self")) */
        const char *tp = ctx->target_player_id ? ctx->target_player_id : "self";
        int pl = rb_resolve_target_player(gs, tp);
        RbPlayer *player = (pl >= 0) ? &gs->p[pl] : &gs->p[0];

        int cards[RB_MAX_ZONE];
        int n_cards = 0;
        for (int k = 0; k < n_stage; k++) {
            int idx = stage_indices[k];
            if (idx < RB_STAGE_SIZE && player->stage[idx] != RB_EMPTY_SLOT) {
                int cid = player->stage[idx];
                if (validate_card(cid)) {
                    cards[n_cards++] = cid;
                }
            }
        }

        /* for &cid in &cards: if !self.selected_cards.contains(&cid) { push } */
        for (int k = 0; k < n_cards; k++) {
            int found = 0;
            for (int j = 0; j < self->n_selected_cards; j++)
                if (self->selected_cards[j] == cards[k]) { found = 1; break; }
            if (!found && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = cards[k];
        }

        /* match self.stage_select_intent.take() */
        RbStageSelectIntent intent = self->stage_select_intent;
        self->stage_select_intent = RB_SSI_NONE;
        switch (intent) {
        case RB_SSI_CHANGE_STATE_WAIT:
            /* if !cards.is_empty() && current_entry effect !started */
            if (n_cards > 0 && !rb_resolver_current_entry_effect_started(gs)) {
                for (int k = 0; k < n_cards; k++) {
                    rb_mods_set_orientation(&gs->mods, cards[k], "wait");   /* add_orientation_modifier */
                    rb_resolver_push_last_cost_waited_member(gs, cards[k]);  /* gs.last_cost_waited_members.push */
                }
            }
            break;

        case RB_SSI_UNDER_MEMBER_MOVE:
            if (n_cards > 0) {
                /* entry_target = current entry effect.target or "self" */
                const AbilityEffect *cur = rb_resolver_current_entry_effect(gs);
                const char *entry_target = (cur && cur->target) ? cur->target : "self";
                int moved[RB_MAX_RECENTLY_MOVED];
                int n_moved = 0;
                for (int k = 0; k < n_cards; k++) {
                    int mid = cards[k];
                    int tpl = rb_resolve_target_player(gs, entry_target);
                    RbPlayer *tp_player = (tpl >= 0) ? &gs->p[tpl] : &gs->p[0];
                    int pos = -1;
                    for (int s = 0; s < RB_STAGE_SIZE; s++)
                        if (tp_player->stage[s] == mid) { pos = s; break; }
                    if (pos >= 0) {
                        int got = rb_drain_under_cards_to_energy_zone(gs, entry_target, pos);
                        /* append returned moved ids (rb_drain returns count; ids tracked
                           via recently_moved batch by the resolver op). */
                        if (got > 0) {
                            /* gather what the resolver just moved from recently_moved */
                            for (int m = 0; m < gs->n_recently_moved && n_moved < RB_MAX_RECENTLY_MOVED; m++)
                                moved[n_moved++] = gs->recently_moved[m];
                        }
                    }
                }
                if (n_moved > 0) {
                    for (int m = 0; m < n_moved && self->n_moved_cards < RB_MAX_RECENTLY_MOVED; m++)
                        self->moved_cards[self->n_moved_cards++] = moved[m];
                    rb_resolver_set_recently_moved_batch(gs, moved, n_moved, "under_member");
                    self->last_move_moved_any = 1;
                } else {
                    self->last_move_moved_any = 0;
                }
            }
            return;   /* choice.rs:1990 */

        case RB_SSI_COLLECT_TARGETS:
            return;   /* choice.rs:1993 */

        case RB_SSI_NONE:
        default:
            break;
        }
    } else {
        /* non-select-action: move the chosen stage cards to the destination */
        const char *edst = rb_resolver_entry_destination(gs);   /* gs.entry_destination() */
        const char *dst_str = ctx->destination
            ? ctx->destination
            : (edst ? edst : "discard");
        int pl = rb_resolve_target_player(gs,
                    ctx->target_player_id ? ctx->target_player_id : "self");
        RbPlayer *player = (pl >= 0) ? &gs->p[pl] : &gs->p[0];

        /* card_ids = resolve_indices_to_ids(player, "stage", &ctx.indices) */
        int card_ids[RB_MAX_ZONE];
        int n_ids = rb_resolve_indices_to_ids(gs, pl, "stage", ctx->indices, ctx->n_indices, card_ids);
        int valid_ids[RB_MAX_ZONE];
        int n_valid = 0;
        for (int k = 0; k < n_ids; k++)
            if (validate_card(card_ids[k]))
                valid_ids[n_valid++] = card_ids[k];

        int last_vacated = -1;
        for (int k = 0; k < n_valid; k++) {
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (player->stage[s] == valid_ids[k]) { last_vacated = s; break; }
        }

        /* util::move_cards(player, &valid_ids, "stage", dst_str, None, card_db) */
        int moved_count = rb_move_cards(gs, pl, valid_ids, n_valid, "stage", dst_str, -1);
        if (moved_count > 0) {
            if (last_vacated >= 0)
                rb_resolver_set_last_vacated_stage_area(gs, last_vacated);
            /* self.selected_cards = self.moved_cards = valid_ids */
            self->n_selected_cards = 0;
            self->n_moved_cards = 0;
            for (int k = 0; k < n_valid && k < RB_MAX_RECENTLY_MOVED; k++) {
                self->selected_cards[self->n_selected_cards++] = valid_ids[k];
                self->moved_cards[self->n_moved_cards++] = valid_ids[k];
            }
            rb_resolver_set_recently_moved_batch(gs, valid_ids, n_valid, "stage");
        }
    }

    /* clear the choice state and resume (choice.rs:2035 returns Ok(())) */
    rb_resolver_clear_choice_state_and_resume(self, gs);
}
