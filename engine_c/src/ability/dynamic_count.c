/* dynamic_count.c — single source of truth for resolving a DynamicCount
   reference into a count.
   Mirror engine/src/ability/dynamic_count.rs:GameState::resolve_dynamic_count.

   Both the constant-path (recalculate_constants) and the ability-execution
   path (AbilityResolver) call this one method, so dynamic_count semantics
   live in exactly one place instead of being duplicated per caller.

   The transient resolver context (which cards moved / were selected / how
   many were drawn in the current step) is passed in because the constant
   path has no AbilityResolver. Callers that don't have that context pass
   empty slices / 0.

   Some reference arms depend on GameState fields the C port does not yet
   track (revealed_cards pool, per-area under_cards, last_cost_discard_count,
   cheer_revealed_cards); those return a documented best-effort value. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

int rb_resolve_dynamic_count(const struct GameState *g,
                             const char *reference,
                             const char *base_reference,
                             const char *count_type,
                             const char *calculation,
                             int calculation_value,
                             int owner_on_p1,
                             const int *moved, int n_moved,
                             const int *selected, int n_selected,
                             int last_draw_count)
{
    const char *reference_text = reference ? reference : base_reference;

    int count = 0;
    if (!reference_text) {
        /* fall through to count_type default */
    } else if (!strcmp(reference_text, "selected_card_score")) {
        if (n_selected > 0) {
            int cid = selected[0];
            Card c; if (rb_decode_card_by_index((uint32_t)cid, &c)) { count = c.score; rb_free_card(&c); }
        }
    } else if (!strcmp(reference_text, "previous_moved_cards") ||
               !strcmp(reference_text, "previous_move")) {
        if (n_moved > 0) count = n_moved;
        else if (g->n_recently_moved > 0) count = g->n_recently_moved;
        else count = 0; /* Rust: mods.last_cost_discard_count (not tracked) */
    } else if (!strcmp(reference_text, "previous_draw")) {
        if (last_draw_count > 0) count = last_draw_count;
        else if (g->n_recently_moved > 0) count = g->n_recently_moved;
        else count = 0;
    } else if (!strcmp(reference_text, "revealed_cards") ||
               !strcmp(reference_text, "previous_reveal")) {
        count = 0; /* revealed pool not tracked in C GameState (cheer_revealed_cards) */
    } else if (!strcmp(reference_text, "unit_count")) {
        const RbPlayer *P = &g->p[g->active];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) count++;
    } else if (!strcmp(reference_text, "energy_difference")) {
        int threshold = base_reference ? atoi(base_reference) : 0;
        const RbPlayer *P = &g->p[g->active];
        int n = P->energy.n - threshold;
        count = n < 0 ? 0 : n;
    } else if (!strcmp(reference_text, "success_pile_count_difference")) {
        /* opponent's success pile minus owner's success pile */
        const RbPlayer *own = owner_on_p1 ? &g->p[0] : &g->p[1];
        const RbPlayer *other = owner_on_p1 ? &g->p[1] : &g->p[0];
        int diff = other->success.n - own->success.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "these_waitroom_placed_count")) {
        if (g->n_recently_moved > 0) count = g->n_recently_moved;
        else count = n_moved;
    } else if (!strcmp(reference_text, "total_live_score")) {
        const RbPlayer *P = &g->p[g->active];
        for (int i = 0; i < P->live.n; i++) {
            Card c; if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) { count += c.score; rb_free_card(&c); }
        }
    } else if (!strcmp(reference_text, "stage_member_count")) {
        const RbPlayer *P = &g->p[g->active];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) count++;
    } else if (!strcmp(reference_text, "opponent_stage_member_count")) {
        const RbPlayer *P = &g->p[1 - g->active];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) count++;
    } else if (!strcmp(reference_text, "opponent_waited_member_count")) {
        const RbPlayer *P = &g->p[1 - g->active];
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT && P->stage_wait[i]) count++;
    } else if (!strcmp(reference_text, "waitroom_count_below_base")) {
        int threshold = base_reference ? atoi(base_reference) : 0;
        const RbPlayer *P = &g->p[g->active];
        int diff = threshold - P->discard.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "energy_cards_under_this_member")) {
        count = 0; /* per-area under_cards not tracked in C RbPlayer */
    } else {
        if (count_type && !strcmp(count_type, "revealed_cards")) count = 0;
        else count = 0;
    }

    if (calculation && !strcmp(calculation, "add")) {
        count += calculation_value;
    }
    return count;
}
