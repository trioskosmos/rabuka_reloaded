#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Ported faithfully from engine/src/ability/effects/score.rs:execute_modify_score.
   Mirrors the Rust resolver: operation (add/remove/set), target player
   (self/opponent/both/live_total), card_type filter (member_card → stage only,
   else stage+live+success), group filter, self_target, heart_colors filter,
   per_unit multiplication (value × (matching_count / per_unit_count), capped by
   repeat_limit), and a floor ("min:0" / score_floor / 未満にはならない) that
   skips a delta that would drive a card's score modifier negative. */

static const char *sc_extra(const AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int sc_extra_int(const AbilityEffect *e, const char *k) {
    const char *v = sc_extra(e, k); return v ? atoi(v) : 0;
}

/* Does decoded card `c` carry a heart of color `col` (0..7)? Mirrors the
   heart_colors CardFilter match used by the Rust target_filter. The C Card
   stores hearts as (heart_color[i], heart_count[i]) pairs (n_hearts of them). */
static int card_has_heart_color(int cid, int col) {
    Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int r = 0;
    for (int i = 0; i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == col && c.heart_count[i] > 0) { r = 1; break; }
    rb_free_card(&c);
    return r;
}

static int heart_color_matches(int cid, const char *hc) {
    if (!hc) return 1;
    /* comma/space separated list; OR semantics. */
    char buf[64]; strncpy(buf, hc, sizeof(buf) - 1); buf[sizeof(buf) - 1] = 0;
    char *tok = strtok(buf, ",、 ");
    int ok = 0;
    while (tok) {
        int col = (int)rb_parse_heart_color(tok);
        if (card_has_heart_color(cid, col)) ok = 1;
        tok = strtok(NULL, ",、 ");
    }
    return ok;
}

/* Collect candidate card ids for `pl` into out (cap max). When card_type is
   "member_card" only the stage is scanned; otherwise stage + live + success
   (mirrors Rust candidate_ids: stage ++ live_card_zone ++ success_live_zone). */
static int collect_candidates(const GameState *g, int pl, const char *card_type,
                              int self_target, int *out, int max) {
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    if (card_type && !strcmp(card_type, "member_card")) {
        for (int i = 0; i < RB_STAGE_SIZE && n < max; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) out[n++] = P->stage[i];
        return n;
    }
    for (int i = 0; i < P->live.n && n < max; i++) out[n++] = P->live.cards[i];
    for (int i = 0; i < P->success.n && n < max; i++) out[n++] = P->success.cards[i];
    for (int i = 0; i < RB_STAGE_SIZE && n < max; i++)
        if (P->stage[i] != RB_EMPTY_SLOT) out[n++] = P->stage[i];
    if (self_target) {
        int act = g->queue.resume_host >= 0 ? g->queue.resume_host : -1;
        if (act >= 0) {
            int seen = 0;
            for (int i = 0; i < n; i++) if (out[i] == act) { seen = 1; break; }
            if (!seen && n < max) out[n++] = act;
        }
    }
    return n;
}

int rb_execute_modify_score(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *op = sc_extra(e, "operation"); if (!op) op = "add";
    int value = e->count >= 0 ? e->count : sc_extra_int(e, "value");

    const char *target = e->target ? e->target : "self";
    int is_live_total = target && !strcmp(target, "live_total");
    const char *resolved_target = is_live_total ? "self" : target;

    const char *card_type = sc_extra(e, "card_type");
    const char *group = sc_extra(e, "group"); if (!group) group = sc_extra(e, "group_names");
    const char *heart_colors = sc_extra(e, "heart_colors");
    const char *location = sc_extra(e, "location");
    int per_unit = sc_extra(e, "per_unit") && (!strcmp(sc_extra(e, "per_unit"), "true") || !strcmp(sc_extra(e, "per_unit"), "1"));
    int per_unit_count = sc_extra_int(e, "per_unit_count"); if (per_unit_count <= 0) per_unit_count = 1;
    int self_target = (!strcmp(e->self_target_field, "true")
                        || (sc_extra(e, "self_target") && !strcmp(sc_extra(e, "self_target"), "true"))) ? 1 : 0;
    int has_floor = sc_extra(e, "score_floor") != NULL
                    || (sc_extra(e, "effect_constraint") && !strcmp(sc_extra(e, "effect_constraint"), "min:0"))
                    || (e->text && strstr(e->text, "未満にはならない"));

    /* Resolve the set of players to apply to. */
    int pls[2]; int npl = 0;
    if (resolved_target && !strcmp(resolved_target, "both")) { pls[0] = 0; pls[1] = 1; npl = 2; }
    else if (resolved_target && (!strcmp(resolved_target, "opponent") || !strcmp(resolved_target, "p2")))
        { pls[0] = actor ^ 1; npl = 1; }
    else { pls[0] = actor; npl = 1; }

    for (int pi = 0; pi < npl; pi++) {
        int pl = pls[pi];
        int cand[RB_MAX_ZONE];
        int ncand = collect_candidates(gs, pl, card_type, self_target, cand, RB_MAX_ZONE);

        /* Filter to the real recipients (group / heart_colors / self_target). */
        int recv[RB_MAX_ZONE]; int nr = 0;
        for (int i = 0; i < ncand && nr < RB_MAX_ZONE; i++) {
            int cid = cand[i];
            if (group && !rb_card_matches_group_str(cid, group)) continue;
            if (heart_colors && !heart_color_matches(cid, heart_colors)) continue;
            if (self_target) {
                int act = gs->queue.resume_host >= 0 ? gs->queue.resume_host : -1;
                if (cid != act) continue;
            }
            recv[nr++] = cid;
        }

        /* Per-unit: value × number of matching units / per_unit_count
           (mirrors Rust value * effective_units, capped by repeat_limit). */
        int final_value = value;
        if (per_unit) {
            int units = nr / per_unit_count;
            if (e->repeat_limit > 0 && units > e->repeat_limit) units = e->repeat_limit;
            final_value = value * units;
        }

        int applied = 0;
        for (int i = 0; i < nr; i++) {
            int cid = recv[i];
            int delta = !strcmp(op, "add") ? final_value
                      : !strcmp(op, "remove") ? -final_value
                      : !strcmp(op, "set") ? final_value : final_value;
            if (has_floor) {
                int cur = rb_mods_get_score(&gs->mods, cid);
                if (cur + delta < 0) continue; /* floor at 0 */
            }
            if (!strcmp(op, "set")) rb_mods_set_score(&gs->mods, cid, (int16_t)delta);
            else rb_mods_add_score(&gs->mods, cid, (int16_t)delta);
            applied++;
        }
        (void)location;
    }
    return 0;
}

/* ── Shared helpers for the remaining score.rs ports ── */

/* Count of a single heart color on a card (mirrors base_heart.hearts.get(hc)). */
static int sc_card_heart_count(int cid, int col) {
    Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int r = 0;
    for (int i = 0; i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == col) r += c.heart_count[i];
    rb_free_card(&c);
    return r;
}

/* Parse a comma/space/、 separated heart-color string into color indices (0..7). */
static int sc_parse_colors(const char *s, int *out, int max) {
    if (!s || !*s) return 0;
    char buf[256]; strncpy(buf, s, sizeof(buf) - 1); buf[sizeof(buf) - 1] = 0;
    char *tok = strtok(buf, ",、 ");
    int n = 0;
    while (tok && n < max) {
        out[n++] = rb_heart_index(rb_parse_heart_color(tok));
        tok = strtok(NULL, ",、 ");
    }
    return n;
}

/* True when every heart color present on the card is listed in exc (mirrors
   Rust base_heart.hearts.keys().all(|hc| exclude_heart_colors.contains(hc)). */
static int sc_card_all_hearts_excluded(const Card *c, const int *exc, int nexc) {
    if (c->n_hearts == 0) return 0;
    for (int i = 0; i < c->n_hearts; i++) {
        int col = (int)c->heart_color[i];
        int found = 0;
        for (int k = 0; k < nexc; k++) if (exc[k] == col) { found = 1; break; }
        if (!found) return 0;
    }
    return 1;
}

static int sc_card_moved_this_turn(const GameState *g, int cid) {
    return (cid >= 0 && cid < RB_MAX_CARD_IDS) ? (g->moved_this_turn[cid] != 0) : 0;
}
static int sc_card_appeared_this_turn(const GameState *g, int cid) {
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == cid) return 1;
    return 0;
}

/* Faithful port of execute_modify_required_hearts. Handles per_unit (both the
   per_unit_heart_colors total-icon mode and the default per-card location
   count, with distinct-name and timing-condition filters), the success-zone
   live+success card pool, group/original-value filters, and per-color
   set/add of need_heart modifiers. */
int rb_execute_modify_required_hearts(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "decrease";
    int value = e->count >= 0 ? e->count : sc_extra_int(e, "value");
    const char *target = e->target ? e->target : "self";
    int per_unit = sc_extra(e, "per_unit") && (!strcmp(sc_extra(e, "per_unit"), "true") || !strcmp(sc_extra(e, "per_unit"), "1"));
    int per_unit_count = sc_extra_int(e, "per_unit_count"); if (per_unit_count <= 0) per_unit_count = 1;
    const char *group = sc_extra(e, "group"); if (!group) group = sc_extra(e, "group_names");
    const char *location = sc_extra(e, "location");
    const char *timing_condition = sc_extra(e, "timing_condition");
    int original_value = sc_extra(e, "original_value") && (!strcmp(sc_extra(e, "original_value"), "true") || !strcmp(sc_extra(e, "original_value"), "1"));
    int original_count = sc_extra_int(e, "original_count");
    const char *original_operator = sc_extra(e, "original_operator");
    int exclude_self = sc_extra(e, "exclude_self") && (!strcmp(sc_extra(e, "exclude_self"), "true") || !strcmp(sc_extra(e, "exclude_self"), "1"));
    int self_target = (!strcmp(e->self_target_field, "true") || (sc_extra(e, "self_target") && !strcmp(sc_extra(e, "self_target"), "true"))) ? 1 : 0;
    int exc_colors[8]; int n_exc = sc_parse_colors(sc_extra(e, "exclude_heart_colors"), exc_colors, 8);
    int max_flag = sc_extra(e, "max") && (!strcmp(sc_extra(e, "max"), "true") || !strcmp(sc_extra(e, "max"), "1"));
    int repeat_limit = e->repeat_limit;
    int pu_colors[8]; int n_pu = sc_parse_colors(sc_extra(e, "per_unit_heart_colors"), pu_colors, 8);
    int is_distinct = (e->distinct_flag != 0);
    int act = gs->queue.resume_host >= 0 ? gs->queue.resume_host : -1;

    int pl = rb_target_player_index(target, actor == 0 ? "p1" : "p2");
    if (pl < 0) pl = actor;
    const RbPlayer *P = &gs->p[pl];

    /* ── per_unit: recompute value from matching units ── */
    if (per_unit) {
        if (n_pu > 0) {
            /* Count total heart icons of the given colors across matching stage members. */
            int total = 0;
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int cid = P->stage[i];
                if (cid == RB_EMPTY_SLOT) continue;
                if (exclude_self && cid == act) continue;
                if (group && !rb_card_matches_group_str(cid, group)) continue;
                for (int j = 0; j < n_pu; j++) total += sc_card_heart_count(cid, pu_colors[j]);
            }
            int per_unit_base = max_flag ? 1 : value;
            int units = total / (per_unit_count > 0 ? per_unit_count : 1);
            if (repeat_limit > 0 && units > repeat_limit) units = repeat_limit;
            value = per_unit_base * units;
        } else {
            /* Default per-unit: count cards in the specified location. */
            int zone_cards[RB_MAX_ZONE]; int nz = 0;
            if (location && (!strcmp(location, "success_live_zone") || !strcmp(location, "success_live_card_zone"))) {
                for (int i = 0; i < P->success.n; i++) zone_cards[nz++] = P->success.cards[i];
            } else if (location && (!strcmp(location, "live_card_zone") || !strcmp(location, "live_zone"))) {
                for (int i = 0; i < P->live.n; i++) zone_cards[nz++] = P->live.cards[i];
            } else {
                for (int i = 0; i < RB_STAGE_SIZE; i++)
                    if (P->stage[i] != RB_EMPTY_SLOT) zone_cards[nz++] = P->stage[i];
            }
            const char *seen[32]; int nseen = 0;
            int count = 0;
            for (int i = 0; i < nz; i++) {
                int cid = zone_cards[i];
                if (exclude_self && cid == act) continue;
                if (group && !rb_card_matches_group_str(cid, group)) continue;
                Card c; int have = rb_decode_card_by_index((uint32_t)cid, &c);
                if (have) {
                    if (is_distinct) {
                        int dup = 0;
                        for (int k = 0; k < nseen; k++) if (seen[k] && c.name && !strcmp(seen[k], c.name)) { dup = 1; break; }
                        if (dup) { rb_free_card(&c); continue; }
                        if (nseen < 32) seen[nseen++] = rb_strdup2(c.name);
                    }
                    if (n_exc > 0 && sc_card_all_hearts_excluded(&c, exc_colors, n_exc)) { rb_free_card(&c); continue; }
                    rb_free_card(&c);
                }
                if (timing_condition && !strcmp(timing_condition, "appeared_or_moved_this_turn")) {
                    if (!sc_card_moved_this_turn(gs, cid) && !sc_card_appeared_this_turn(gs, cid)) continue;
                }
                count++;
            }
            for (int k = 0; k < nseen; k++) rb_free((void *)seen[k]);
            int per_unit_base = max_flag ? 1 : value;
            int units = count / (per_unit_count > 0 ? per_unit_count : 1);
            if (repeat_limit > 0 && units > repeat_limit) units = repeat_limit;
            value = per_unit_base * units;
        }
    }

    /* ── Build candidate card ids (live, or live+success when self-target
       activating card sits in success_live_card_zone). ── */
    int in_success = 0;
    if (act >= 0) for (int i = 0; i < P->success.n; i++) if (P->success.cards[i] == act) { in_success = 1; break; }
    int card_ids[RB_MAX_ZONE]; int nids = 0;
    if (self_target && in_success) {
        for (int i = 0; i < P->live.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->live.cards[i];
        for (int i = 0; i < P->success.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->success.cards[i];
    } else {
        for (int i = 0; i < P->live.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->live.cards[i];
    }

    /* Filter by self_target / group / original score. */
    int recv[RB_MAX_ZONE]; int nr = 0;
    for (int i = 0; i < nids && nr < RB_MAX_ZONE; i++) {
        int cid = card_ids[i];
        if (self_target) { if (act < 0 || cid != act) continue; }
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        if (original_value) {
            Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
            int score = (int)c.score; rb_free_card(&c);
            if (original_operator) {
                int met = 1;
                if      (!strcmp(original_operator, ">=")) met = score >= original_count;
                else if (!strcmp(original_operator, "<=")) met = score <= original_count;
                else if (!strcmp(original_operator, ">"))  met = score >  original_count;
                else if (!strcmp(original_operator, "<"))  met = score <  original_count;
                else if (!strcmp(original_operator, "==")) met = score == original_count;
                else if (!strcmp(original_operator, "!=")) met = score != original_count;
                if (!met) continue;
            } else if (score != original_count) continue;
        }
        recv[nr++] = cid;
    }

    /* Resolve colors (default heart00) and apply per-color modifiers. */
    int colors[8]; int ncol = sc_parse_colors(sc_extra(e, "heart_colors"), colors, 8);
    if (ncol == 0) { colors[0] = 0; ncol = 1; }
    int per_color_value = value;
    for (int ci = 0; ci < ncol; ci++) {
        int color = colors[ci];
        for (int i = 0; i < nr; i++) {
            int cid = recv[i];
            int delta;
            if (!strcmp(operation, "decrease")) delta = -value;
            else if (!strcmp(operation, "increase")) delta = value;
            else if (!strcmp(operation, "set")) delta = per_color_value;
            else continue;
            if (!strcmp(operation, "set"))
                rb_mods_set_need_heart(&gs->mods, cid, color, (int16_t)per_color_value);
            else
                rb_mods_add_need_heart(&gs->mods, cid, color, delta);
        }
    }
    return 0;
}

/* Faithful port of execute_modify_required_hearts_standard. */
void rb_execute_modify_required_hearts_standard(GameState *gs, int actor,
        const char *operation, int value, const char **heart_colors, int n_colors,
        const char *target, const char *effect_text) {
    if (!gs) return;
    int colors[8]; int ncol = 0;
    if (!heart_colors || n_colors <= 0) { colors[0] = 0; ncol = 1; }
    else { for (int i = 0; i < n_colors && ncol < 8; i++) colors[ncol++] = rb_heart_index(rb_parse_heart_color(heart_colors[i])); }

    int pl = rb_target_player_index(target, actor == 0 ? "p1" : "p2");
    if (pl < 0) pl = actor;
    const RbPlayer *P = &gs->p[pl];

    for (int ci = 0; ci < ncol; ci++) {
        int color = colors[ci];
        for (int i = 0; i < P->live.n; i++) {
            int cid = P->live.cards[i];
            int modifier = !strcmp(operation, "increase") ? value
                         : !strcmp(operation, "decrease") ? -value : 0;
            rb_mods_add_need_heart(&gs->mods, cid, color, (int16_t)modifier);
        }
    }
    (void)effect_text;
}

/* Faithful port of execute_modify_yell_count. */
int rb_execute_modify_yell_count(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "subtract";
    int count = e->count >= 0 ? e->count : sc_extra_int(e, "count");
    int slot = (actor == 1) ? 2 : 1;
    if (!strcmp(operation, "add")) rb_add_yell_count_modifier(gs, (uint8_t)slot, (int32_t)count);
    else if (!strcmp(operation, "subtract")) rb_add_yell_count_modifier(gs, (uint8_t)slot, -(int32_t)count);
    return 0;
}

/* Faithful port of execute_modify_limit. */
int rb_execute_modify_limit(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "decrease";
    int count = e->count >= 0 ? e->count : sc_extra_int(e, "count");
    if (gs->n_prohibition < 64) {
        char *b = gs->prohibition[gs->n_prohibition];
        if (!strcmp(operation, "decrease")) snprintf(b, 48, "limit_decrease:%d", count);
        else if (!strcmp(operation, "increase")) snprintf(b, 48, "limit_increase:%d", count);
        else b[0] = 0;
        gs->n_prohibition++;
    }
    (void)actor;
    return 0;
}

/* Faithful port of execute_modify_required_hearts_success. */
int rb_execute_modify_required_hearts_success(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "increase";
    int value = e->count >= 0 ? e->count : sc_extra_int(e, "value");
    const char *target = e->target ? e->target : "self";
    const char *card_type = sc_extra(e, "card_type");
    const char *heart_colors = sc_extra(e, "heart_colors");

    int pl = rb_target_player_index(target, actor == 0 ? "p1" : "p2");
    if (pl < 0) pl = actor;
    RbPlayer *P = &gs->p[pl];

    int card_ids[RB_MAX_ZONE]; int nids = 0;
    if (card_type && !strcmp(card_type, "live_card")) {
        for (int i = 0; i < P->success.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->success.cards[i];
    }

    int delta = !strcmp(operation, "increase") ? value
              : !strcmp(operation, "decrease") ? -value : 0;
    if (delta == 0) return 0;

    int colors[8]; int ncol = sc_parse_colors(heart_colors, colors, 8);
    if (ncol == 0) { for (int i = 0; i < 7; i++) colors[i] = i; ncol = 7; }

    for (int i = 0; i < nids; i++) {
        int cid = card_ids[i];
        for (int ci = 0; ci < ncol; ci++) {
            rb_mods_add_need_heart(&gs->mods, cid, colors[ci], (int16_t)delta);
        }
    }
    return 0;
}
