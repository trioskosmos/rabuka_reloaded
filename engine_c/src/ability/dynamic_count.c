/* dynamic_count.c — single source of truth for resolving a DynamicCount
   reference into a count.
   Mirror engine/src/ability/dynamic_count.rs:GameState::resolve_dynamic_count.

   Both the constant-path (recalculate_constants) and the ability-execution
   path (AbilityResolver) call this one method, so dynamic_count semantics
   live in exactly one place instead of being duplicated per caller.

   The transient resolver context (which cards moved / were selected / how
   many were drawn in the current step) is passed in because the constant
   path has no AbilityResolver. Callers that don't have that context pass
   empty slices / 0. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Forward declaration */
static int rb_revealed_count(const struct GameState *g, int owner);

/* ── owner resolution helpers ──────────────────────────────────────────── */

/* Determine which player is "self" for a DynamicCount arm that uses
   resolve_target_player("self"). The C port collapses the concept to
   the player whose turn it is (g->active), equivalent to Rust's
   resolve_target_player("self") for normal play. */
static int rb_dc_self_player(const struct GameState *g)
{
    return (g && g->active >= 0) ? g->active : 0;
}

/* Determine which player is "opponent". */
static int rb_dc_opponent_player(const struct GameState *g, int self_pl)
{
    (void)g;
    return 1 - self_pl;
}

/* Determine the effective owner player from owner_card. Mirrors Rust:
   let own_is_p1 = match owner_card {
       Some(cid) => self.player1.stage.stage.contains(&cid),
       None => true
   };
   If owner_card < 0 we fall back to owner_on_p1. */
static int rb_dc_owner_from_card(const struct GameState *g, int owner_card, int owner_on_p1)
{
    if (owner_card >= 0) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (g->p[0].stage[i] == owner_card) return 0;
            if (g->p[1].stage[i] == owner_card) return 1;
        }
    }
    return owner_on_p1 ? 1 : 0;
}

/* ── stage member count helper ─────────────────────────────────────────── */

static int rb_stage_member_count(const RbPlayer *P)
{
    int c = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (P->stage[i] != RB_EMPTY_SLOT) c++;
    return c;
}

/* ── main resolver ─────────────────────────────────────────────────────── */

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
    (void)moved;
    const char *reference_text = reference ? reference : base_reference;

    int count = 0;

    if (!reference_text) {
        /* fall through to count_type default */
    } else if (!strcmp(reference_text, "selected_card_score")) {
        if (n_selected > 0) {
            int cid = selected[0];
            Card c;
            if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                count = c.score;
                rb_free_card(&c);
            }
        }
    } else if (!strcmp(reference_text, "previous_moved_cards") ||
               !strcmp(reference_text, "previous_move")) {
        if (n_moved > 0)
            count = n_moved;
        else if (g->n_recently_moved > 0)
            count = g->n_recently_moved;
        else
            count = g->mods.last_cost_discard_count;
    } else if (!strcmp(reference_text, "previous_draw")) {
        if (last_draw_count > 0)
            count = last_draw_count;
        else if (g->n_recently_moved > 0)
            count = g->n_recently_moved;
        else
            count = 0;
    } else if (!strcmp(reference_text, "revealed_cards") ||
               !strcmp(reference_text, "previous_reveal")) {
        count = rb_revealed_count(g, owner);
    } else if (!strcmp(reference_text, "unit_count")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        count = rb_stage_member_count(P);
    } else if (!strcmp(reference_text, "energy_difference")) {
        int threshold = 0;
        if (base_reference) {
            char *end = NULL;
            long v = strtol(base_reference, &end, 10);
            if (end != base_reference && *end == '\0' && v >= 0 && v <= 255)
                threshold = (int)v;
        }
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        int n = P->energy.n - threshold;
        count = n < 0 ? 0 : n;
    } else if (!strcmp(reference_text, "success_pile_count_difference")) {
        int own_pl   = rb_dc_owner_from_card(g, owner_on_p1 ? -1 : owner, owner_on_p1);
        int other_pl = 1 - own_pl;
        const RbPlayer *own   = &g->p[own_pl];
        const RbPlayer *other = &g->p[other_pl];
        int diff = other->success.n - own->success.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "these_waitroom_placed_count")) {
        if (g->n_recently_moved > 0)
            count = g->n_recently_moved;
        else
            count = n_moved;
    } else if (!strcmp(reference_text, "total_live_score")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        for (int i = 0; i < P->live.n; i++) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) {
                count += c.score;
                rb_free_card(&c);
            }
        }
    } else if (!strcmp(reference_text, "stage_member_count")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        count = rb_stage_member_count(P);
    } else if (!strcmp(reference_text, "opponent_stage_member_count")) {
        int self_pl  = rb_dc_self_player(g);
        int opp_pl   = rb_dc_opponent_player(g, self_pl);
        const RbPlayer *P = &g->p[opp_pl];
        count = rb_stage_member_count(P);
    } else if (!strcmp(reference_text, "opponent_waited_member_count")) {
        int self_pl  = rb_dc_self_player(g);
        int opp_pl   = rb_dc_opponent_player(g, self_pl);
        const RbPlayer *P = &g->p[opp_pl];
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT && P->stage_wait[i])
                count++;
    } else if (!strcmp(reference_text, "waitroom_count_below_base")) {
        int threshold = 0;
        if (base_reference) {
            char *end = NULL;
            long v = strtol(base_reference, &end, 10);
            if (end != base_reference && *end == '\0' && v >= 0 && v <= 255)
                threshold = (int)v;
        }
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        int diff = threshold - P->discard.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "energy_cards_under_this_member")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        if (host_cid >= 0) {
            for (int a = 0; a < RB_STAGE_SIZE; a++) {
                if (P->stage[a] == host_cid) {
                    count = P->under_cards[a].n;
                    break;
                }
            }
        } else {
            for (int a = 0; a < RB_STAGE_SIZE; a++)
                count += P->under_cards[a].n;
        }
    } else {
        if (count_type && !strcmp(count_type, "revealed_cards"))
            count = 0;
        else
            count = 0;
    }

    if (calculation && !strcmp(calculation, "add")) {
        count += calculation_value;
    }
    return count;
}

/* ── effect-level count resolution ────────────────────────────────────── */

/* Look up an extra_kv pair by key in an AbilityEffect. */
static const char *dc_extra(const AbilityEffect *e, const char *key)
{
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key))
            return e->extra_v[i];
    return NULL;
}

/* Parse an extra_kv value as a base-10 integer; returns 0 on missing/invalid. */
static int dc_extra_int(const AbilityEffect *e, const char *key)
{
    const char *v = dc_extra(e, key);
    if (!v) return 0;
    char *end = NULL;
    long val = strtol(v, &end, 10);
    if (end == v || *end != '\0') return 0;
    return (int)val;
}

/* Resolve an effect's repeat/draw count: return the static `count` if set,
   otherwise pull the DynamicCount parameters the decoder stored as extra_kv
   and feed them to rb_resolve_dynamic_count. Falls back to 1 when no dynamic
   parameters are present (preserves prior default). */
int rb_effect_count(const struct GameState *g, int actor, int host_cid, const AbilityEffect *e,
                    int last_draw_count)
{
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
                int pl = (host_cid >= 0) ? rb_owner_of_card((GameState *)g, host_cid) : actor;
                if (pl < 0) pl = actor;
                units = g->p[pl].live.n;
            } else if (!strcmp(loc, "hand")) {
                units = g->p[actor].hand.n;
            } else if (!strcmp(loc, "stage")) {
                int c = 0;
                for (int s = 0; s < RB_STAGE_SIZE; s++)
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

    const char *reference      = dc_extra(e, "reference");
    const char *base_reference = dc_extra(e, "base_reference");
    const char *count_type     = dc_extra(e, "count_type");
    if (!reference && !base_reference && !count_type) return 1;

    const char *calculation    = dc_extra(e, "calculation");
    int         calc_value     = dc_extra_int(e, "calculation_value");
    const char *on_p1          = dc_extra(e, "owner_on_p1");
    int         owner_on_p1    = (on_p1 && !strcmp(on_p1, "true")) ? 1 : 0;
    int         moved          = dc_extra_int(e, "moved");
    int         selected       = dc_extra_int(e, "selected");

    return rb_resolve_dynamic_count(g, actor, host_cid,
                                    reference, base_reference, count_type,
                                    calculation, calc_value, owner_on_p1,
                                    &moved, moved > 0 ? 1 : 0,
                                    &selected, selected > 0 ? 1 : 0,
                                    last_draw_count);
}

/* ── revealed_count: mirror GameState::revealed_count ──
   Number of cards in the revealed (yell) pool belonging to `owner`.

   Rust order (engine/src/ability/dynamic_count.rs):
     1. If cheer_revealed_cards() is non-empty, return its length.
     2. Otherwise filter g.revealed_cards to those belonging to owner's
        zones: hand / waitroom / stage / under_cards / energy_zone /
        main_deck / energy_deck / live_card_zone / success_live_card_zone /
        resolution_zone.

   The C port does not maintain a separate cheer pool, so step 1 always
   falls through to the zone-filtering step. */
int rb_revealed_count(const struct GameState *g, int owner)
{
    const RbPlayer *P = &g->p[owner];
    int count = 0;

    for (int i = 0; i < g->n_revealed; i++) {
        int cid = g->revealed_cards[i];
        int in  = 0;

        /* hand */
        for (int k = 0; k < P->hand.n && !in; k++)
            if (P->hand.cards[k] == cid) { in = 1; break; }

        /* waitroom (discard bag) */
        for (int k = 0; k < P->discard.n && !in; k++)
            if (P->discard.cards[k] == cid) { in = 1; break; }

        /* stage */
        for (int k = 0; k < RB_STAGE_SIZE && !in; k++)
            if (P->stage[k] == cid) { in = 1; break; }

        /* under_cards per stage area */
        for (int a = 0; a < RB_STAGE_SIZE && !in; a++) {
            for (int j = 0; j < P->under_cards[a].n; j++)
                if (P->under_cards[a].cards[j] == cid) { in = 1; break; }
            if (in) break;
        }

        /* energy zone */
        for (int k = 0; k < P->energy.n && !in; k++)
            if (P->energy.cards[k] == cid) { in = 1; break; }

        /* main deck */
        for (int k = 0; k < P->deck.n && !in; k++)
            if (P->deck.cards[k] == cid) { in = 1; break; }

        /* energy deck */
        for (int k = 0; k < P->energy_deck.n && !in; k++)
            if (P->energy_deck.cards[k] == cid) { in = 1; break; }

        /* live card zone */
        for (int k = 0; k < P->live.n && !in; k++)
            if (P->live.cards[k] == cid) { in = 1; break; }

        /* success live card zone */
        for (int k = 0; k < P->success.n && !in; k++)
            if (P->success.cards[k] == cid) { in = 1; break; }

        /* resolution zone (global, not per-player) */
        for (int k = 0; k < g->resolution.n && !in; k++)
            if (g->resolution.cards[k] == cid) { in = 1; break; }

        if (in) count++;
    }

    return count;
}
