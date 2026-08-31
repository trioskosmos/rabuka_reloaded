/* engine_c/src/ability/choice_frag_09.c
 *
 * Port fragment of engine/src/ability/choice.rs (~lines 2585-2650):
 *   - handle_draw_any_number   -> rb_resolver_handle_draw_any_number
 *   - handle_order_selection   -> rb_resolver_handle_order_selection
 *
 * Rust path notes:
 *   - Both methods live on `AbilityResolver` (`self`). `self.gs` becomes a
 *     `GameState*` here. The Rust `gs.entry_effect()` maps to the resolver's
 *     current effect node `g->queue.resume_eff` (no separate entry stack in C).
 *   - Resolver-local fields with no C struct (execution_context, spawn_context,
 *     looked_at_cards) are mirrored with module-scope statics, following the
 *     convention established in choice_frag_14.c (one resolver per process).
 *   - `self.clear_choice_state(gs)` is replaced by the two external resume
 *     helpers called by name: rb_resolver_handle_selection_epilogue then
 *     rb_resolver_clear_choice_state_and_resume (defined in choice_frag_14.c).
 *   - draw_cards_for_player falls back to rb_draw_cards_for_player; the Rust
 *     card_database clone has no C equivalent (pass NULL for the card_db arg).
 */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* ── Resolver-local state (mirrors AbilityResolver fields with no C struct) ── */

/* looked_at_cards — Rust GameState::looked_at_cards (Vec<i16> of card ids). */
#define RB_MAX_LOOKED 256
static int  g_looked_at_cards[RB_MAX_LOOKED];
static int  g_looked_at_n = 0;

/* spawn_context.target — Rust AbilityResolver::spawn_context.target (String). */
static char g_spawn_context_target[64] = "";   /* empty => unset */

/* execution_context — Rust AbilityResolver::execution_context (ExecutionContext).
 *   step discriminator for LookAndSelect::Finalize (carries a `destination`). */
typedef enum {
    RB_EXEC_NONE = 0,
    RB_EXEC_LOOK_AND_SELECT
} RbExecContextKind;
static RbExecContextKind g_execution_context = RB_EXEC_NONE;
static int               g_look_step_is_finalize = 0;
static char              g_look_step_destination[32] = "";  /* Finalize.destination */

/* External helpers owned by other translation units — prototypes only (not defined here). */
extern void rb_resolver_handle_selection_epilogue(GameState *g);
extern void rb_resolver_clear_choice_state_and_resume(GameState *g);

/* Prototypes for the functions defined in this fragment. */
static void   rb_push_deck_top(RbPlayer *P, int card_id);
static void   rb_push_deck_top_helper_looked(int card);
void          rb_resolver_handle_draw_any_number(GameState *g, const char *selected);
void          rb_resolver_handle_order_selection(GameState *g, const char *selected);

/* rb_push_deck_top — mirror player.main_deck.cards.insert(0, card_id):
 *   unshift a card id to the front of the player's deck bag. */
static void rb_push_deck_top(RbPlayer *P, int card_id) {
    if (!P || P->deck.n >= RB_MAX_ZONE) return;
    for (int i = P->deck.n; i > 0; i--)
        P->deck.cards[i] = P->deck.cards[i - 1];
    P->deck.cards[0] = card_id;
    P->deck.n++;
}

/* rb_resolver_handle_draw_any_number — Rust: handle_draw_any_number
 *   (ability/choice.rs). selected is the chosen draw count string. */
void rb_resolver_handle_draw_any_number(GameState *g, const char *selected) {
    if (!g || !selected) return;

    /* Rust: let Ok(count) = selected.parse::<usize>() else { warn; return Err };
     * reject a non-numeric selection (first token must be a digit). */
    const char *p = selected;
    while (*p == ' ' || *p == '\t') p++;
    if (*p < '0' || *p > '9') {
        /* Rust: log::warn!("[DRAW_ANY] non-numeric count ...; rejecting selection") */
        return;   /* selection rejected; leave choice state untouched */
    }
    size_t count = (size_t)strtoul(p, NULL, 10);

    /* Rust: if let Some(effect) = gs.entry_effect().cloned() { ... } */
    AbilityEffect *eff = g->queue.resume_eff;
    if (eff) {
        /* source_any().unwrap_or(Zone::Deck.to_str()) */
        const char *source = eff->source ? eff->source : "deck";
        /* destination.map(|d| d.to_str()).unwrap_or(Zone::Hand.to_str()) */
        const char *destination = eff->destination ? eff->destination : "hand";
        /* card_type_any().map(|ct| ct.as_card_str()) */
        const char *card_type = eff->card_type_field && eff->card_type_field[0]
                                  ? eff->card_type_field : NULL;
        /* target.as_deref().unwrap_or("self") */
        const char *target = eff->target ? eff->target : "self";

        /* Rust: let player = gs.resolve_target_player_mut(target); */
        int pl = rb_resolve_target_player(g, target);
        RbPlayer *player = (pl >= 0) ? &g->p[pl] : &g->p[g->active];

        if (count > 0) {
            /* Rust: draw_cards_for_player(player, count as u8, source, destination,
             *         card_type, false, None, &card_db, None)
             * C card_db (Rust clone) is unused -> pass NULL. */
            rb_draw_cards_for_player(player, (uint8_t)count, source, destination,
                                     card_type, 0, NULL, NULL, -1);
        }
    }

    /* Rust: self.clear_choice_state(gs); Ok(())
     * -> finalize selection bookkeeping, then clear + resume pending actions. */
    rb_resolver_handle_selection_epilogue(g);
    rb_resolver_clear_choice_state_and_resume(g);
}

/* rb_resolver_handle_order_selection — Rust: handle_order_selection
 *   (ability/choice.rs). Reorders cards looked at (LookAndSelect Finalize)
 *   back onto the top of the owner's deck. */
void rb_resolver_handle_order_selection(GameState *g, const char *selected) {
    if (!g) return;

    /* Rust: let ctx = self.execution_context.clone();
     *       if let ExecutionContext::LookAndSelect { step } = ctx {
     *         if let LookAndSelectStep::Finalize { destination, .. } = step {
     *           if Zone::from_str(&destination) == Some(Zone::Deck) { ... } } } */
    if (g_execution_context == RB_EXEC_LOOK_AND_SELECT && g_look_step_is_finalize &&
        strcmp(g_look_step_destination, "deck") == 0) {

        /* Rust: if let Ok(idx) = selected.parse::<usize>() {
         *         if idx < gs.looked_at_cards.len() {
         *           let card = gs.looked_at_cards.remove(idx);
         *           gs.looked_at_cards.insert(0, card); } } */
        if (selected) {
            const char *q = selected;
            while (*q == ' ' || *q == '\t') q++;
            if (*q >= '0' && *q <= '9') {
                size_t idx = (size_t)strtoul(q, NULL, 10);
                if (idx < (size_t)g_looked_at_n) {
                    int card = g_looked_at_cards[idx];
                    for (size_t i = idx; i + 1 < (size_t)g_looked_at_n; i++)
                        g_looked_at_cards[i] = g_looked_at_cards[i + 1];
                    g_looked_at_n--;
                    rb_push_deck_top_helper_looked(card);  /* insert(0, card) on looked_at */
                }
            }
        }

        /* Rust: let card_ids: Vec<i16> = gs.looked_at_cards.iter().rev().copied().collect(); */
        int card_ids[RB_MAX_LOOKED];
        int n_ids = 0;
        for (int i = g_looked_at_n - 1; i >= 0 && n_ids < RB_MAX_LOOKED; i--)
            card_ids[n_ids++] = g_looked_at_cards[i];

        /* Rust: target = self.spawn_context.target.clone()
         *         .or_else(|| gs.entry_effect().and_then(|e| e.target.clone()))
         *         .unwrap_or_else(|| "self") */
        const char *target = NULL;
        if (g_spawn_context_target[0]) {
            target = g_spawn_context_target;
        } else if (g->queue.resume_eff && g->queue.resume_eff->target) {
            target = g->queue.resume_eff->target;
        } else {
            target = "self";
        }

        /* Rust: let player = gs.resolve_target_player_mut(&target); */
        int pl = rb_resolve_target_player(g, target);
        RbPlayer *player = (pl >= 0) ? &g->p[pl] : &g->p[g->active];

        /* Rust: for card_id in card_ids { player.main_deck.cards.insert(0, card_id); }
         *       gs.looked_at_cards.clear(); */
        for (int i = 0; i < n_ids; i++)
            rb_push_deck_top(player, card_ids[i]);
        g_looked_at_n = 0;
    }

    /* Rust: self.clear_choice_state(gs); Ok(()) */
    rb_resolver_handle_selection_epilogue(g);
    rb_resolver_clear_choice_state_and_resume(g);
}

/* rb_push_deck_top_helper_looked — internal: unshift `card` onto looked_at_cards[0]
 *   (mirrors gs.looked_at_cards.insert(0, card) inside handle_order_selection). */
static void rb_push_deck_top_helper_looked(int card) {
    if (g_looked_at_n >= RB_MAX_LOOKED) return;
    for (int i = g_looked_at_n; i > 0; i--)
        g_looked_at_cards[i] = g_looked_at_cards[i - 1];
    g_looked_at_cards[0] = card;
    g_looked_at_n++;
}
