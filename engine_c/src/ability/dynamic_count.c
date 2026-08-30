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

int rb_resolve_dynamic_count(const struct GameState *g, int owner, int host_cid,
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
        else count = g->mods.last_cost_discard_count; /* Rust: mods.last_cost_discard_count */
    } else if (!strcmp(reference_text, "previous_draw")) {
        if (last_draw_count > 0) count = last_draw_count;
        else if (g->n_recently_moved > 0) count = g->n_recently_moved;
        else count = 0;
    } else if (!strcmp(reference_text, "revealed_cards") ||
                 !strcmp(reference_text, "previous_reveal")) {
        /* Mirror revealed_count: no separate cheer pool in the C port, so count the
            revealed (yell) cards that currently belong to the owner's zones. */
        const RbPlayer *P = &g->p[owner];
        for (int i = 0; i < g->n_revealed; i++) {
            int cid = g->revealed_cards[i];
            int in = 0;
            for (int k = 0; k < P->hand.n; k++) if (P->hand.cards[k] == cid) { in = 1; break; }
            if (!in) for (int k = 0; k < P->discard.n; k++) if (P->discard.cards[k] == cid) { in = 1; break; }
            if (!in) for (int k = 0; k < RB_STAGE_SIZE; k++) if (P->stage[k] == cid) { in = 1; break; }
            if (!in) for (int k = 0; k < RB_STAGE_SIZE; k++) if (P->under_cards[k].n) { in = 1; break; }
            if (!in) for (int k = 0; k < P->energy.n; k++) if (P->energy.cards[k] == cid) { in = 1; break; }
            if (!in) for (int k = 0; k < P->deck.n; k++) if (P->deck.cards[k] == cid) { in = 1; break; }
            if (!in) for (int k = 0; k < P->live.n; k++) if (P->live.cards[k] == cid) { in = 1; break; }
            if (!in) for (int k = 0; k < P->success.n; k++) if (P->success.cards[k] == cid) { in = 1; break; }
            if (in) count++;
        }
    } else if (!strcmp(reference_text, "unit_count")) {
        const RbPlayer *P = &g->p[owner];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) count++;
    } else if (!strcmp(reference_text, "energy_difference")) {
        int threshold = base_reference ? atoi(base_reference) : 0;
        const RbPlayer *P = &g->p[owner];
        int n = P->energy.n - threshold;
        count = n < 0 ? 0 : n;
    } else if (!strcmp(reference_text, "success_pile_count_difference")) {
        /* opponent's success pile minus owner's success pile */
        const RbPlayer *own = &g->p[owner];
        const RbPlayer *other = &g->p[1 - owner];
        int diff = other->success.n - own->success.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "these_waitroom_placed_count")) {
        if (g->n_recently_moved > 0) count = g->n_recently_moved;
        else count = n_moved;
    } else if (!strcmp(reference_text, "total_live_score")) {
        const RbPlayer *P = &g->p[owner];
        for (int i = 0; i < P->live.n; i++) {
            Card c; if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) { count += c.score; rb_free_card(&c); }
        }
    } else if (!strcmp(reference_text, "stage_member_count")) {
        const RbPlayer *P = &g->p[owner];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) count++;
    } else if (!strcmp(reference_text, "opponent_stage_member_count")) {
        const RbPlayer *P = &g->p[1 - owner];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) count++;
    } else if (!strcmp(reference_text, "opponent_waited_member_count")) {
        const RbPlayer *P = &g->p[1 - owner];
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT && P->stage_wait[i]) count++;
    } else if (!strcmp(reference_text, "waitroom_count_below_base")) {
        int threshold = base_reference ? atoi(base_reference) : 0;
        const RbPlayer *P = &g->p[owner];
        int diff = threshold - P->discard.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "energy_cards_under_this_member")) {
        /* Mirror Rust: count only the under-cards of the MEMBER whose ability is
            resolving (self.activating_card's stage slot), not every stage member. */
        const RbPlayer *P = &g->p[owner];
        if (host_cid >= 0) {
            for (int a = 0; a < RB_STAGE_SIZE; a++)
                if (P->stage[a] == host_cid) { count += P->under_cards[a].n; break; }
        } else {
            for (int a = 0; a < RB_STAGE_SIZE; a++) count += P->under_cards[a].n;
        }
    } else {
        if (count_type && !strcmp(count_type, "revealed_cards")) count = 0;
        else count = 0;
    }

    if (calculation && !strcmp(calculation, "add")) {
        count += calculation_value;
    }
    return count;
}

/* ── effect-level count resolution ── */
static const char *dc_extra(const AbilityEffect *e, const char *key) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}
static int dc_extra_int(const AbilityEffect *e, const char *key) {
    const char *v = dc_extra(e, key);
    return v ? atoi(v) : 0;
}

/* Resolve an effect's repeat/draw count: return the static `count` if set,
   otherwise pull the DynamicCount parameters the decoder stored as extra_kv
   and feed them to rb_resolve_dynamic_count. Falls back to 1 when no dynamic
   parameters are present (preserves prior default). */
int rb_effect_count(const struct GameState *g, int actor, int host_cid, const AbilityEffect *e,
                      int last_draw_count) {
    if (!e) return 0;
    /* per_unit scaling (mirrors misc.rs calculate_gain_multiplier /
        resolve_per_unit_count): the base count is multiplied by the number of
        units at `location` (e.g. one heart per success-live-zone card). Checked
        BEFORE the e->count early-return because the base count is 1-per-unit. */
    const char *per_unit = dc_extra(e, "per_unit");
    if (per_unit && !strcmp(per_unit, "true")) {
        const char *loc = dc_extra(e, "location");
        int units = 1;
        if (loc) {
            if (!strcmp(loc, "success_live_zone") || !strcmp(loc, "success") ||
                !strcmp(loc, "live")) {
                /* the player's live-card zone (the live being performed) */
                int pl = (host_cid >= 0) ? rb_owner_of_card((GameState*)g, host_cid) : actor;
                if (pl < 0) pl = actor;
                units = g->p[pl].live.n;
            } else if (!strcmp(loc, "hand")) {
                units = g->p[actor].hand.n;
            } else if (!strcmp(loc, "stage")) {
                int c = 0; for (int s = 0; s < RB_STAGE_SIZE; s++)
                    if (g->p[actor].stage[s] != RB_EMPTY_SLOT) c++;
                units = c;
            } else if (!strcmp(loc, "deck")) {
                units = g->p[actor].deck.n;
            } else if (!strcmp(loc, "success_zone")) {
                units = g->p[actor].success.n;
            }
        }
        if (units < 0) units = 0;
        int base = (e->count >= 0) ? e->count : 1;
        if (base < 0) base = 1;
        return base * units;
    }
    if (e->count >= 0) return e->count;
    const char *reference = dc_extra(e, "reference");
    const char *base_reference = dc_extra(e, "base_reference");
    const char *count_type = dc_extra(e, "count_type");
    if (!reference && !base_reference && !count_type) return 1; /* unresolved → default */
    const char *calculation = dc_extra(e, "calculation");
    int calc_value = dc_extra_int(e, "calculation_value");
    const char *on_p1 = dc_extra(e, "owner_on_p1");
    int owner_on_p1 = (on_p1 && !strcmp(on_p1, "true")) ? 1 : 0;
    int moved = dc_extra_int(e, "moved");
    int selected = dc_extra_int(e, "selected");
    return rb_resolve_dynamic_count(g, actor, host_cid, reference, base_reference, count_type, calculation,
                                    calc_value, owner_on_p1,
                                    &moved, moved > 0 ? 1 : 0,
                                    &selected, selected > 0 ? 1 : 0,
                                    last_draw_count);
}
