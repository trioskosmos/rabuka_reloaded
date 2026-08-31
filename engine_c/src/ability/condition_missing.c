/* condition_missing.c — port of remaining missing functions from
   engine/src/ability/condition.rs and condition/card.rs.
   Append these functions to condition.c (they share its static helpers). */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* ── comparison_default_count (condition.rs) ──
   Returns 1 if the condition specifies a location or card_type filter, else 0.
   Mirrors: if condition.get_location().is_some() || condition.get_card_type().is_some() */
int rb_comparison_default_count(const Condition *c) {
    const char *loc = get_str(c, "location");
    const char *ct = get_str(c, "card_type");
    return (loc && *loc) || (ct && *ct) ? 1 : 0;
}

/* ── stage_has_any_member (condition.rs) ──
   Returns true if any stage slot is occupied. */
int rb_stage_has_any_member(const struct GameState *g, int pl) {
    const RbPlayer *P = &g->p[pl];
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (P->stage[i] != RB_EMPTY_SLOT) return 1;
    return 0;
}

/* ── zone_len (condition/card.rs) ──
   Returns the "length" of a zone for comparison purposes. For stage this
   is total effective blade (not card count); for other zones it is the
   number of cards. Mirrors player.stage.total_blades(...) for Stage. */
static int zone_len(const struct GameState *g, int pl, const char *location) {
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(location, "stage")) {
        int total = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            int b = (int)cc.blade + rb_mods_get_blade((RbMods*)&g->mods, cid);
            rb_free_card(&cc);
            if (b > 0) total += b;
        }
        return rb_saturate_u8(total);
    }
    if (!strcmp(location, "hand")) return P->hand.n;
    if (!strcmp(location, "deck") || !strcmp(location, "deck_top") || !strcmp(location, "deck_bottom")) return P->deck.n;
    if (!strcmp(location, "discard") || !strcmp(location, "waitroom")) return P->discard.n;
    if (!strcmp(location, "energy") || !strcmp(location, "energy_zone")) return P->energy.n;
    if (!strcmp(location, "live_card_zone") || !strcmp(location, "live")) return P->live.n;
    if (!strcmp(location, "success") || !strcmp(location, "success_zone") ||
        !strcmp(location, "success_live_zone") || !strcmp(location, "success_live_card_zone")) return P->success.n;
    if (!strcmp(location, "revealed_cards")) return g->n_revealed;
    return 0;
}

/* ── count_distinct_in_cards (condition/card.rs) ──
   Count distinct names/costs/groups among cards matching the condition's filters.
   distinct_type: "cost" → distinct effective costs,
                  "group_name"→ distinct group names,
                  otherwise → distinct card names. */
static int count_distinct_in_cards(const struct GameState *g, const int *cards, int n,
                                    const Condition *cond, const char *card_type,
                                    const char *group) {
    /* Collect matching cards first */
    int matching[RB_MAX_ZONE];
    int nm = 0;
    for (int i = 0; i < n; i++) {
        int cid = cards[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (card_type && !card_matches_card_type_filter(cid, card_type)) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        matching[nm++] = cid;
    }
    /* Determine distinct type */
    const char *dtype = "card_name";
    const CondValue *dv = find_val(cond, "distinct");
    if (dv && dv->tag == RB_TAG_STR && dv->s) dtype = dv->s;
    else {
        const char *gref = get_str(cond, "group_reference");
        if (gref && !strcmp(gref, "different_group_names")) dtype = "group_name";
    }
    if (!strcmp(dtype, "cost")) {
        return count_distinct_cost(g, matching, nm, card_type);
    }
    if (!strcmp(dtype, "group_name")) {
        /* Count distinct non-empty group names */
        int ng = 0;
        for (int i = 0; i < nm; i++) {
            Card cc; if (!rb_decode_card_by_index((uint32_t)matching[i], &cc)) continue;
            const char *gn = rb_card_string(cc.group_idx);
            rb_free_card(&cc);
            if (gn && *gn) ng++;
        }
        return ng;
    }
    /* card_name distinct: count distinct card names */
    int nd = 0;
    for (int i = 0; i < nm; i++) {
        Card ci; if (!rb_decode_card_by_index((uint32_t)matching[i], &ci)) continue;
        int seen = 0;
        for (int j = 0; j < i; j++) {
            Card cj; if (!rb_decode_card_by_index((uint32_t)matching[j], &cj)) continue;
            if (!strcmp(ci.name, cj.name)) seen = 1;
            rb_free_card(&cj);
            if (seen) break;
        }
        if (!seen) nd++;
        rb_free_card(&ci);
    }
    return nd;
}

/* ── count_cards_with_filters (condition/card.rs) ──
   Count cards in a slice matching the condition's card_type / group / heart_colors /
   cost_limit / exclude_self filters, honoring original_value for blade/heart. */
static int count_cards_with_filters(const struct GameState *g, int actor, const Condition *c,
                                     const int *cards, int n,
                                     const char *card_type, const char *group,
                                     int cost_limit, const char *cost_op,
                                     int exclude_cid, int respect_original) {
    int count = 0;
    for (int i = 0; i < n; i++) {
        int cid = cards[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (exclude_cid >= 0 && cid == exclude_cid) continue;
        if (card_type && !card_matches_card_type_filter(cid, card_type)) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        if (cost_limit > 0) {
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            int cost = cc.cost;
            rb_free_card(&cc);
            if (!eval_operator(cost, cost_op, cost_limit)) continue;
        }
        if (respect_original) {
            int ov = 0; get_bool(c, "original_value", &ov);
            if (ov) {
                if (!check_original_blade_filter(g, actor, c, cid)) continue;
                if (!check_original_heart_filter(g, actor, c, cid)) continue;
            }
        }
        count++;
    }
    return count;
}

/* ── sum_group_hearts_in_stage (condition/card.rs) ──
   Sum base hearts of stage members matching the group filter. */
static int sum_group_hearts_in_stage(const struct GameState *g, int pl, const char *group) {
    const RbPlayer *P = &g->p[pl];
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        for (int h = 0; h < cc.n_hearts; h++) total += cc.heart_count[h];
        rb_free_card(&cc);
    }
    return total;
}

/* ── sum_group_filtered_zone (condition/card.rs) ──
   Sum a per-card value (hearts/score) for cards in a zone matching
   card_type and group filters. value_kind: 0 = base hearts, 1 = score */
static int sum_group_filtered_zone(const struct GameState *g, const int *cards, int n,
                                    const char *card_type, const char *group, int value_kind) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        int cid = cards[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (card_type && !card_matches_card_type_filter(cid, card_type)) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        if (value_kind == 0) {
            for (int h = 0; h < cc.n_hearts; h++) total += cc.heart_count[h];
        } else {
            total += cc.score;
        }
        rb_free_card(&cc);
    }
    return total;
}

/* ── count_for_player_target (condition/card.rs) ──
   Resolve the comparison count for a specific player + location + comparison_type.
   Mirrors the Rust closure used by get_count_for_target. */
static int count_for_player_target(const struct GameState *g, int actor, const Condition *c,
                                    const char *target) {
    int pl = target_player_idx(actor, c);
    if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
    const char *loc = get_str(c, "location");
    const char *ctype = get_str(c, "comparison_type");
    const char *res = get_str(c, "resource_type");
    const RbPlayer *P = &g->p[pl];
    if (ctype && !strcmp(ctype, "score")) {
        int total = 0;
        if (!strcmp(loc, "success") || !strcmp(loc, "success_zone") ||
            !strcmp(loc, "success_live_zone") || !strcmp(loc, "success_live_card_zone")) {
            for (int i = 0; i < P->success.n; i++) {
                Card cc; if (!rb_decode_card_by_index((uint32_t)P->success.cards[i], &cc)) continue;
                total += cc.score; rb_free_card(&cc);
            }
        } else if (!strcmp(loc, "live_card_zone") || !strcmp(loc, "live")) {
            for (int i = 0; i < P->live.n; i++) {
                Card cc; if (!rb_decode_card_by_index((uint32_t)P->live.cards[i], &cc)) continue;
                total += cc.score; rb_free_card(&cc);
            }
        } else if (!loc || !strcmp(loc, "")) {
            for (int i = 0; i < P->success.n; i++) {
                Card cc; if (!rb_decode_card_by_index((uint32_t)P->success.cards[i], &cc)) continue;
                total += cc.score; rb_free_card(&cc);
            }
            for (int i = 0; i < P->live.n; i++) {
                Card cc; if (!rb_decode_card_by_index((uint32_t)P->live.cards[i], &cc)) continue;
                total += cc.score; rb_free_card(&cc);
            }
        }
        return total;
    }
    if (ctype && !strcmp(ctype, "cost")) {
        int total = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            int base = (int)cc.cost;
            rb_free_card(&cc);
            total += rb_saturate_u8(base + rb_mods_get_cost((RbMods*)&g->mods, cid));
        }
        return total;
    }
    if (ctype && !strcmp(ctype, "energy")) {
        return P->energy.n;
    }
    if (res && !strcmp(res, "hand_count")) {
        return P->hand.n;
    }
    if (res && !strncmp(res, "heart", 5)) {
        /* resource_type starting with "heart" → sum base hearts of given color */
        char clean[32]; int j = 0;
        for (int i = 0; res[i] && j < 31; i++) if (res[i] != '_') clean[j++] = res[i];
        clean[j] = '\0';
        int color = s_heart_idx(clean);
        int total = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            if (color < cc.n_hearts) total += cc.heart_count[color];
            rb_free_card(&cc);
        }
        return total;
    }
    if (res && !strcmp(res, "surplus_heart")) {
        int member = 0, need = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            for (int h = 0; h < cc.n_hearts; h++) member += cc.heart_count[h];
            rb_free_card(&cc);
        }
        for (int i = 0; i < P->live.n; i++) {
            Card cc; if (!rb_decode_card_by_index((uint32_t)P->live.cards[i], &cc)) continue;
            for (int h = 0; h < cc.n_hearts; h++) need += cc.heart_count[h];
            rb_free_card(&cc);
        }
        for (int i = 0; i < P->success.n; i++) {
            Card cc; if (!rb_decode_card_by_index((uint32_t)P->success.cards[i], &cc)) continue;
            for (int h = 0; h < cc.n_hearts; h++) need += cc.heart_count[h];
            rb_free_card(&cc);
        }
        int diff = member - need;
        return diff < 0 ? 0 : diff;
    }
    if (res && !strcmp(res, "energy")) {
        return P->energy.n;
    }
    /* Default: zone_len (stage → total blade, others → card count) */
    return zone_len(g, pl, loc ? loc : "");
}

/* ── count_surplus_heart (condition/card.rs) ──
   Compute surplus hearts for the target player, with support for
   heart_colors filtering and the live-snapshot fast path. */
static int count_surplus_heart(const struct GameState *g, int actor, const Condition *c,
                                const char *target) {
    int delta = 0; get_bool(c, "delta", &delta);
    if (delta) {
        int pl = target_player_idx(actor, c);
        if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
        return g->mods.last_surplus_loss_count[pl];
    }
    int pl = target_player_idx(actor, c);
    if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
    const RbPlayer *P = &g->p[pl];
    /* If a live snapshot was recorded this turn, prefer it */
    if (g->live_surplus_ready_this_turn) {
        const CondValue *hv = find_val(c, "heart_colors");
        if (hv && hv->tag == RB_TAG_ARRAY && hv->arr_n > 0) {
            /* Per-color surplus from snapshot */
            int total = 0;
            for (uint32_t k = 0; k < hv->arr_n; k++) {
                int col = RB_HEART_ALL;
                if (hv->arr[k].tag == RB_TAG_I64) col = (int)hv->arr[k].i;
                else if (hv->arr[k].tag == RB_TAG_STR && hv->arr[k].s) col = atoi(hv->arr[k].s);
                if (col >= 0 && col < 8 && g->n_snapshots > 0) {
                    /* Find most recent snapshot for this player */
                    for (int s = g->n_snapshots - 1; s >= 0; s--) {
                        if (g->snapshots[s].player == pl) {
                            total += g->snapshots[s].surplus_per_color[col];
                            break;
                        }
                    }
                }
            }
            return total;
        }
        if (pl == 0) return g->self_live_surplus_count;
        return g->opponent_live_surplus_count;
    }
    /* Fallback: compute from current state */
    const CondValue *hv = find_val(c, "heart_colors");
    if (hv && hv->tag == RB_TAG_ARRAY && hv->arr_n > 0) {
        int total = 0;
        for (uint32_t k = 0; k < hv->arr_n; k++) {
            int col = RB_HEART_ALL;
            if (hv->arr[k].tag == RB_TAG_I64) col = (int)hv->arr[k].i;
            else if (hv->arr[k].tag == RB_TAG_STR && hv->arr[k].s) col = atoi(hv->arr[k].s);
            int member = 0, need = 0;
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int cid = P->stage[i];
                if (cid == RB_EMPTY_SLOT) continue;
                Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
                if (col < cc.n_hearts) member += cc.heart_count[col];
                rb_free_card(&cc);
            }
            for (int i = 0; i < P->live.n; i++) {
                Card cc; if (!rb_decode_card_by_index((uint32_t)P->live.cards[i], &cc)) continue;
                if (col < cc.n_hearts) need += cc.heart_count[col];
                rb_free_card(&cc);
            }
            for (int i = 0; i < P->success.n; i++) {
                Card cc; if (!rb_decode_card_by_index((uint32_t)P->success.cards[i], &cc)) continue;
                if (col < cc.n_hearts) need += cc.heart_count[col];
                rb_free_card(&cc);
            }
            int diff = member - need;
            if (diff > 0) total += diff;
        }
        return total;
    }
    int member = 0, need = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        for (int h = 0; h < cc.n_hearts; h++) member += cc.heart_count[h];
        rb_free_card(&cc);
    }
    for (int i = 0; i < P->live.n; i++) {
        Card cc; if (!rb_decode_card_by_index((uint32_t)P->live.cards[i], &cc)) continue;
        for (int h = 0; h < cc.n_hearts; h++) need += cc.heart_count[h];
        rb_free_card(&cc);
    }
    for (int i = 0; i < P->success.n; i++) {
        Card cc; if (!rb_decode_card_by_index((uint32_t)P->success.cards[i], &cc)) continue;
        for (int h = 0; h < cc.n_hearts; h++) need += cc.heart_count[h];
        rb_free_card(&cc);
    }
    int diff = member - need;
    return diff < 0 ? 0 : diff;
}
