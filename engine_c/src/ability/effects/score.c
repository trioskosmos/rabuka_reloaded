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
