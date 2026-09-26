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

/* Determine the effective owner player from owner_card. Mirrors Rust
   (engine/src/ability/dynamic_count.rs:76-79):
       let own_is_p1 = match owner_card {
           Some(cid) => self.player1.stage.stage.contains(&cid),
           None => true,
       };
   i.e. the owner is whichever player's stage holds the activating card; with
   no owner card the owner is the acting player (player1 in the default C
   model), which is what owner_on_p1 == 0 expresses. */
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
        int own_pl   = rb_dc_owner_from_card(g, host_cid, owner_on_p1);
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
        /* Rust (dynamic_count.rs:136-144): the activating card's stage
           *index* is used, and when the card is not on the stage the index
           falls back to 1 (center) — it does NOT widen to every member. */
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        int area = -1;
        if (host_cid >= 0) {
            for (int a = 0; a < RB_STAGE_SIZE; a++)
                if (P->stage[a] == host_cid) { area = a; break; }
        }
        if (area < 0) area = 1;
        if (area >= 0 && area < RB_STAGE_SIZE)
            count = P->under_cards[area].n;
    } else {
        /* Rust (dynamic_count.rs:145-148): an unknown reference falls through
           to count_type, where "revealed_cards" is the only non-zero arm. */
        if (count_type && !strcmp(count_type, "revealed_cards"))
            count = rb_revealed_count(g, owner);
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
    const char *reference = dc_extra(e, "reference");
    const char *base_reference = dc_extra(e, "base_reference");
    const char *count_type = dc_extra(e, "count_type");
    int has_dynamic = reference || base_reference || count_type;
    int dynamic_count = -1;
    if (has_dynamic) {
        const char *calculation = dc_extra(e, "calculation");
        int calc_value = dc_extra_int(e, "calculation_value");
        const char *on_p1 = dc_extra(e, "owner_on_p1");
        int owner_on_p1 = (on_p1 && !strcmp(on_p1, "true")) ? 1 : 0;
        int moved = dc_extra_int(e, "moved");
        int selected = dc_extra_int(e, "selected");
        dynamic_count = rb_resolve_dynamic_count(
            g, actor, host_cid, reference, base_reference, count_type,
            calculation, calc_value, owner_on_p1,
            &moved, moved > 0 ? 1 : 0, &selected, selected > 0 ? 1 : 0,
            last_draw_count);
    }
    const char *per_unit = dc_extra(e, "per_unit");
    if (per_unit && !strcmp(per_unit, "true")) {
        const char *loc = dc_extra(e, "location");
        if (!loc) loc = dc_extra(e, "per_unit_type");
        /* Rust resolves the per-unit multiplier through
           util.rs::resolve_per_unit_count, which counts the zone through the
           effect's CardFilter (card_type / group) rather than counting the
           raw zone length. Mirror that; the raw length is the unfiltered case. */
        const char *ct  = dc_extra(e, "card_type");
        const char *grp = dc_extra(e, "group");
        int units = 0;
        if (!loc || !strcmp(loc, "hand")) {
            units = rb_count_matching(g->p[actor].hand.cards, g->p[actor].hand.n, ct, grp);
        } else if (!strcmp(loc, "stage") || !strcmp(loc, "member") ||
                   !strcmp(loc, "members") || !strcmp(loc, "人")) {
            int ids[RB_MAX_ZONE];
            int n = 0;
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (g->p[actor].stage[s] != RB_EMPTY_SLOT && n < RB_MAX_ZONE)
                    ids[n++] = g->p[actor].stage[s];
            units = rb_count_matching(ids, n, ct, grp);
        } else if (!strcmp(loc, "success_live_zone") || !strcmp(loc, "success") ||
                   !strcmp(loc, "live") || !strcmp(loc, "success_zone") ||
                   !strcmp(loc, "live_card_zone") || !strcmp(loc, "success_live_card_zone")) {
            int pl = (host_cid >= 0) ? rb_owner_of_card((GameState *)g, host_cid) : actor;
            if (pl < 0) pl = actor;
            int ids[RB_MAX_ZONE];
            int n = 0;
            if (!strcmp(loc, "success_live_zone") || !strcmp(loc, "success") ||
                !strcmp(loc, "success_zone") || !strcmp(loc, "success_live_card_zone"))
                for (int k = 0; k < g->p[pl].success.n; k++) ids[n++] = g->p[pl].success.cards[k];
            else
                for (int k = 0; k < g->p[pl].live.n; k++) ids[n++] = g->p[pl].live.cards[k];
            units = rb_count_matching(ids, n, ct, grp);
        } else if (!strcmp(loc, "energy") || !strcmp(loc, "energy_zone")) {
            units = rb_count_matching(g->p[actor].energy.cards, g->p[actor].energy.n, ct, grp);
        } else if (!strcmp(loc, "under_member") || !strcmp(loc, "under") ||
                   !strcmp(loc, "下")) {
            int ids[RB_MAX_ZONE];
            int n = 0;
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                for (int k = 0; k < g->p[actor].under_cards[s].n && n < RB_MAX_ZONE; k++)
                    ids[n++] = g->p[actor].under_cards[s].cards[k];
            units = rb_count_matching(ids, n, ct, grp);
        } else if (!strcmp(loc, "deck")) {
            /* No per-unit counter in Rust covers the main deck; keep the raw
               length so the count does not silently collapse to 1. */
            units = g->p[actor].deck.n;
        } else {
            units = 1;
        }
        if (units < 0) units = 0;
        int base = dynamic_count >= 0 ? dynamic_count : (e->count >= 0 ? e->count : 1);
        if (base < 0) base = 1;
        return base * units;
    }
    if (dynamic_count >= 0) return dynamic_count;
    return e->count >= 0 ? e->count : 1;
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
