/* engine_c/src/ability/choice_frag_11.c
 *
 * Port fragment of engine/src/ability/choice.rs (~lines 2966-3107):
 *   - handle_primary_alternative  -> rb_resolver_handle_primary_alternative
 *   - handle_position_destination -> rb_resolver_handle_position_destination
 *
 * Rust path notes:
 *   - The Rust methods live on `AbilityResolver` (`self`). Per choice_frag_08.c
 *     the resolver is modelled as `RbResolver *rs`; the Rust `self.gs` is the
 *     `GameState *g` argument threaded through every rb_resolver_* call.
 *   - `apply_effect_modification` (choice.rs:2949) is inlined: clear choice
 *     state, mutate the current entry effect (`g->queue.resume_eff`), park it
 *     via `rb_resolver_set_pending_actions`, then resume through
 *     `rb_resolver_handle_selection_epilogue` (mirrors resume_pending_actions).
 *   - `core::mem::replace(&mut self.execution_context, None)` maps to reading
 *     `rs->execution_context` into a local and resetting `rs->execution_context.kind`
 *     to RB_EXEC_NONE.
 *   - Mandated external helpers (do NOT define here):
 *       rb_resolver_handle_selection_epilogue
 *       rb_resolver_clear_choice_state_and_resume
 *
 * C11, only depends on rabuka.h + sibling rb_resolver_* helpers (forward-declared).
 */

#include "rabuka.h"
#include <string.h>

/* ExecutionContext — mirrors engine/src/ability/types.rs::ExecutionContext. */
typedef enum {
    RB_EXEC_NONE = 0,
    RB_EXEC_SINGLE_EFFECT,
    RB_EXEC_LOOK_AND_SELECT,
    RB_EXEC_MOVE_CARDS_POSITION
} RbExecCtxKind;

typedef struct RbExecutionContext {
    RbExecCtxKind kind;
    int   card_id;            /* MoveCardsPosition: i16 card id */
    char  target[64];         /* MoveCardsPosition: String target */
    char  source_zone[32];    /* MoveCardsPosition: String source_zone */
} RbExecutionContext;

/* ── Resolver-local state (mirrors AbilityResolver; layout matches choice_frag_08.c
 *    plus the execution_context field used by handle_position_destination). ── */
typedef struct RbResolver {
    int           has_pending_choice;            /* self.pending_choice.is_some() */
    char          spawn_target[32];              /* self.spawn_context.target */
    AbilityEffect *current_effect;               /* self.current_effect */
    int           moved_cards[RB_MAX_RECENTLY_MOVED]; /* self.moved_cards */
    int           n_moved_cards;
    int           pending_reprompt;              /* self.pending_reprompt_choice.is_some() */
    char          reprompt_target[32];
    char          reprompt_desc[256];
    char          reprompt_desc_en[256];
    char          reprompt_desc_ja[256];
    int           reprompt_allow_skip;
    /* self.execution_context (choice.rs ExecutionContext) */
    RbExecutionContext execution_context;
} RbResolver;


/* ── Forward prototypes (in-fragment forward use + mandated external helpers) ── */

/* Mandated external helpers, defined in sibling translation units. */
int  rb_resolver_handle_selection_epilogue(RbResolver *rs, GameState *g,
                                           const RbExecutionContext *ctx);
int  rb_resolver_clear_choice_state_and_resume(GameState *g, RbResolver *rs);

/* Other self.* resolver methods / shared helpers (defined elsewhere). */
void rb_resolver_clear_choice_state(GameState *g, RbResolver *rs);
int  rb_resolver_set_pending_actions(GameState *g, const AbilityEffect *const *cmds, int n);
AbilityEffect *rb_effect_clone(const AbilityEffect *e);
void rb_effect_free(AbilityEffect *e);

/* ─────────────────────────────────────────────────────────────────────────
 * handle_primary_alternative (choice.rs:2966)
 *   Rust: self.apply_effect_modification(gs, |effect| { ... })
 * ───────────────────────────────────────────────────────────────────────── */
int rb_resolver_handle_primary_alternative(RbResolver *rs, GameState *g,
                                           const char *selected) {
    /* apply_effect_modification → clear_choice_state(gs) first. */
    rb_resolver_clear_choice_state(g, rs);

    /* let mut effect = gs.entry_effect().cloned() — the current effect node. */
    AbilityEffect *eff = g->queue.resume_eff;
    if (eff) {
        /* choice.rs:2972 — chosen = alternative_effect_any().or(primary_effect)
           for "1"/"alternative"/"secondary", else just primary_effect. */
        AbilityEffect *chosen = NULL;
        if (strcmp(selected, "1") == 0 ||
            strcmp(selected, "alternative") == 0 ||
            strcmp(selected, "secondary") == 0) {
            chosen = eff->alternative_effect ? eff->alternative_effect
                                              : eff->primary_effect;
        } else {
            chosen = eff->primary_effect;
        }
        /* choice.rs:2982 — *effect = sub_effect.clone() (replace entry effect). */
        if (chosen) {
            AbilityEffect *repl = rb_effect_clone(chosen);
            rb_effect_free(eff);
            g->queue.resume_eff = repl;
            eff = repl;
        }
    }

    /* choice.rs:2960 — gs.ability_queue.set_pending_actions(vec![effect]). */
    if (eff) {
        const AbilityEffect *cmds[1] = { eff };
        rb_resolver_set_pending_actions(g, cmds, 1);
    }

    /* choice.rs:2962 — self.resume_pending_actions(gs) via selection epilogue. */
    return rb_resolver_handle_selection_epilogue(rs, g, NULL);
}

/* ─────────────────────────────────────────────────────────────────────────
 * handle_position_destination (choice.rs:2988)
 * ───────────────────────────────────────────────────────────────────────── */
int rb_resolver_handle_position_destination(RbResolver *rs, GameState *g,
                                            const char *selected) {
    /* choice.rs:2996 — let ctx = core::mem::replace(&mut self.execution_context, None) */
    RbExecutionContext ctx = rs->execution_context;
    rs->execution_context.kind = RB_EXEC_NONE;

    if (ctx.kind == RB_EXEC_MOVE_CARDS_POSITION) {
        /* choice.rs:3006 — compute use_limit key BEFORE mutable player borrow. */
        int use_limit_cid = -1;
        int use_limit_idx = 0;
        int want_record_use = 0;
        if (strcmp(selected, "skip") != 0) {
            /* choice.rs:3008 — gs.ability_queue.current_entry() → entries[cur] */
            RbQueueEntry *entry = &g->queue.entries[g->queue.cur];
            /* Rust: entry.ability.effect.is_some_and(|e| e.optional) — the C
               entry carries no Ability; mirror via the resolving effect's flag. */
            AbilityEffect *cur = g->queue.resume_eff;
            int is_optional = (cur && cur->is_optional) ? 1 : 0;
            /* Rust: is_optional && entry.ability.use_limit.is_some() → record. */
            if (is_optional) {
                int cid = entry->card_id;
                if (cid == 0) cid = g->queue.resume_host; /* entry.card_id.or(gs.activating_card) */
                if (cid != 0) {
                    use_limit_cid = cid;
                    use_limit_idx = entry->ability_idx;
                    want_record_use = 1;
                }
            }
        }

        /* choice.rs:3021 — let player = gs.resolve_target_player_mut(&target) */
        int pl = rb_resolve_target_player(g, ctx.target);

        /* choice.rs:3024 — destination == "skip": card stays in waitroom, no-op. */
        if (strcmp(selected, "skip") == 0) {
            if (rb_ability_debug_enabled()) { /* [DECK_DIAG] skip — card stays in waitroom */ }
            /* choice.rs:3029 — mark optional as skipped (RbQueueEntry has no
               optional_cost_result in C; documented mirror, then clear+resume). */
            rb_resolver_clear_choice_state_and_resume(g, rs);
            return 0;
        }

        if (rb_ability_debug_enabled()) { /* [DECK_DIAG] handle_position_destination ctx=MoveCardsPosition */ }

        /* choice.rs:3040 — remove the card from its source zone first.
           Discard/Waitroom/those_cards all live in the player's waitroom bag. */
        if (strcmp(ctx.source_zone, "hand") == 0) {
            rb_remove_card_from_zone(g, pl, ctx.card_id, "hand");
        } else {
            rb_remove_card_from_zone(g, pl, ctx.card_id, "waitroom");
        }

        /* choice.rs:3072 — place_card_in_zone(player, card_id, destination, ...) */
        rb_place_card_in_zone(g, pl, ctx.card_id, selected, -1);

        if (rb_ability_debug_enabled()) { /* [DECK_DIAG] recorded use_limit for optional effect */ }

        /* choice.rs:3090 — record use_limit after the player borrow is done. */
        if (want_record_use) {
            rb_record_ability_use(g, use_limit_cid, use_limit_idx);
        }

        /* choice.rs:3096 — clear_choice_state; resume_pending_actions. */
        rb_resolver_clear_choice_state_and_resume(g, rs);
        return 0;
    }

    /* choice.rs:3099 — fallback: non-card-specific position choice (stage position).
       Reuses apply_effect_modification: set effect.destination, then resume. */
    rb_resolver_clear_choice_state(g, rs);
    AbilityEffect *eff = g->queue.resume_eff;
    if (eff) {
        if (eff->destination) rb_free(eff->destination);
        eff->destination = rb_strdup2(selected);   /* effect.destination = Some(Zone::from_source_str(selected)) */
        const AbilityEffect *cmds[1] = { eff };
        rb_resolver_set_pending_actions(g, cmds, 1);
    }
    return rb_resolver_handle_selection_epilogue(rs, g, NULL);
}
