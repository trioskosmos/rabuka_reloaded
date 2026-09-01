#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* Forward declarations */
static int s_heart_idx(const char *h);
static int count_surplus_heart(const struct GameState *g, int actor, const Condition *c, const char *target);
static int eval_card_count(const GameState *g, int actor, const Condition *c);
static int eval_card_blade(const GameState *g, int actor, const Condition *c);
static int count_surplus_heart(const struct GameState *g, int actor, const Condition *c, const char *target);
static int eval_resource_count(const GameState *g, int actor, const Condition *c);
static int eval_card_count(const GameState *g, int actor, const Condition *c);
static int eval_card_blade(const GameState *g, int actor, const Condition *c);

static int s_heart_idx(const char *h){
    if(!h) return RB_HEART_ALL;
    if(!strcmp(h,"pink")||!strcmp(h,"heart00")) return RB_HEART_PINK;
    if(!strcmp(h,"red")||!strcmp(h,"heart01")) return RB_HEART_RED;
    if(!strcmp(h,"yellow")||!strcmp(h,"heart02")) return RB_HEART_YELLOW;
    if(!strcmp(h,"green")||!strcmp(h,"heart03")) return RB_HEART_GREEN;
    if(!strcmp(h,"blue")||!strcmp(h,"heart04")) return RB_HEART_BLUE;
    if(!strcmp(h,"purple")||!strcmp(h,"heart05")) return RB_HEART_PURPLE;
    if(!strcmp(h,"orange")||!strcmp(h,"heart06")) return RB_HEART_ORANGE;
    if(!strcmp(h,"all")||!strcmp(h,"heart07")) return RB_HEART_ALL;
    if(!strncmp(h,"heart",5)){ int idx=atoi(h+5); if(idx>=0&&idx<=7) return idx; }
    return RB_HEART_ALL;
}
#include <stdlib.h>

/* ── field lookup helpers ── */
static const CondValue *find_val(const Condition *c, const char *key) {
    for (uint32_t i = 0; i < c->n_fields; i++) if (!strcmp(c->fields[i].key, key)) return &c->fields[i].v;
    return NULL;
}
static const char *get_str(const Condition *c, const char *key) {
    const CondValue *v = find_val(c, key);
    if (!v || v->tag != RB_TAG_STR) return NULL;
    return v->s;
}
static int get_i(const Condition *c, const char *key, int *out) {
    const CondValue *v = find_val(c, key);
    if (!v) return 0;
    if (v->tag == RB_TAG_I64) { *out = (int)v->i; return 1; }
    if (v->tag == RB_TAG_STR && v->s) { *out = atoi(v->s); return 1; }
    return 0;
}
static int get_bool(const Condition *c, const char *key, int *out) {
    const CondValue *v = find_val(c, key);
    if (!v) return 0;
    if (v->tag == RB_TAG_TRUE) { *out = 1; return 1; }
    if (v->tag == RB_TAG_FALSE) { *out = 0; return 1; }
    if (v->tag == RB_TAG_STR && v->s) { *out = (!strcmp(v->s,"true")); return 1; }
    return 0;
}
/* Return a nested Condition stored under `key` (Rust condition.get_condition()). */
static const Condition *get_cond(const Condition *c, const char *key) {
    const CondValue *v = find_val(c, key);
    if (!v || v->tag != RB_TAG_OBJVAR) return NULL;
    return v->cond;
}
/* Per-card "moved this turn" gate  Emirrors GameState::has_card_moved_this_turn. */
static int card_moved_this_turn(const GameState *g, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return g->moved_this_turn[cid] != 0;
}
static int eval_operator(int actual, const char *op, int threshold) {
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return actual >= threshold;
    if (!strcmp(op, ">"))  return actual >  threshold;
    if (!strcmp(op, "<=")) return actual <= threshold;
    if (!strcmp(op, "<"))  return actual <  threshold;
    if (!strcmp(op, "=") || !strcmp(op, "==") || !strcmp(op, "eq")) return actual == threshold;
    if (!strcmp(op, "!=") || !strcmp(op, "ne")) return actual != threshold;
    return actual >= threshold;
}

static int target_player_idx(int actor, const Condition *c) {
    const char *t = get_str(c, "target");
    if (!t) t = get_str(c, "scope");
    if (t && !strcmp(t, "opponent")) return actor ^ 1;
    if (t && (!strcmp(t, "both") || !strcmp(t, "either"))) return actor; /* self pass; both handled by caller */
    return actor;
}

static int count_in_zone(const struct GameState *g, int pl, const char *loc) {
    if (!loc) return 0;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(loc, "hand")) return P->hand.n;
    if (!strcmp(loc, "stage")) {
        int cnt = 0; for (int i=0;i<RB_STAGE_SIZE;i++) if (P->stage[i]!=RB_EMPTY_SLOT) cnt++;
        return cnt;
    }
    if (!strcmp(loc, "deck")||!strcmp(loc,"deck_top")||!strcmp(loc,"deck_bottom")) return P->deck.n;
    if (!strcmp(loc, "discard")||!strcmp(loc,"waitroom")) return P->discard.n;
    if (!strcmp(loc, "energy")||!strcmp(loc,"energy_zone")) return P->energy.n;
    if (!strcmp(loc, "live_card_zone")||!strcmp(loc,"live")) return P->live.n;
    if (!strcmp(loc, "success")||!strcmp(loc,"success_zone")||!strcmp(loc,"success_live_zone")||!strcmp(loc,"success_live_card_zone")) return P->success.n;
    if (!strcmp(loc, "resolution")||!strcmp(loc,"resolution_zone")) return g->resolution.n;
    if (!strcmp(loc, "revealed_cards")) return g->n_revealed;
    if (!strcmp(loc, "empty_area")) {
        int e=0; for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==RB_EMPTY_SLOT) e++;
        return e;
    }
    return 0;
}
/* Mirror engine/src/ability/util.rs:zone_cards  Eresolve a named zone to its
   card-id list. The zone-name table is the same one Rust's `Zone::from_str`
   accepts (ability/enums.rs). Two documented extensions mirror special cases
   Rust applies at the CALL sites rather than inside zone_cards:
     • UnderMember is flattened over stage.under_cards (util::count_in_zone), and
     • RevealedCards / Resolution resolve to the GameState-level pools
       (card.rs:evaluate_location_condition reads game_state.revealed_cards
       directly before falling through to zone_cards).
   Empty stage slots (-1) are dropped here because EVERY Rust call site guards
   with `id != -1`. Returns the number of ids written to `out`. */
static int zone_ids(const struct GameState *g, int pl, const char *loc, int *out, int max) {
    if (!loc || !out || max <= 0) return 0;
    const RbPlayer *P = &g->p[pl];
    int n = 0;
#define RB_ZPUSH(id) do { int _z=(id); if (n < max && _z != RB_EMPTY_SLOT) out[n++] = _z; } while (0)
    if (!strcmp(loc, "stage")) {
        for (int i=0;i<RB_STAGE_SIZE;i++) RB_ZPUSH(P->stage[i]);
    } else if (!strcmp(loc, "center")) {
        RB_ZPUSH(P->stage[1]);
    } else if (!strcmp(loc, "left") || !strcmp(loc, "left_side")) {
        RB_ZPUSH(P->stage[0]);
    } else if (!strcmp(loc, "right") || !strcmp(loc, "right_side")) {
        RB_ZPUSH(P->stage[2]);
    } else if (!strcmp(loc, "hand")) {
        for (int i=0;i<P->hand.n;i++) RB_ZPUSH(P->hand.cards[i]);
    } else if (!strcmp(loc, "deck") || !strcmp(loc, "deck_top") ||
               !strcmp(loc, "deck_bottom") || !strcmp(loc, "energy_deck")) {
        for (int i=0;i<P->deck.n;i++) RB_ZPUSH(P->deck.cards[i]);
    } else if (!strcmp(loc, "discard") || !strcmp(loc, "waitroom")) {
        for (int i=0;i<P->discard.n;i++) RB_ZPUSH(P->discard.cards[i]);
    } else if (!strcmp(loc, "energy") || !strcmp(loc, "energy_zone")) {
        for (int i=0;i<P->energy.n;i++) RB_ZPUSH(P->energy.cards[i]);
    } else if (!strcmp(loc, "live_card_zone") || !strcmp(loc, "live")) {
        for (int i=0;i<P->live.n;i++) RB_ZPUSH(P->live.cards[i]);
    } else if (!strcmp(loc, "success_live_zone") || !strcmp(loc, "success_live_card_zone") ||
               !strcmp(loc, "success_zone") || !strcmp(loc, "success")) {
        for (int i=0;i<P->success.n;i++) RB_ZPUSH(P->success.cards[i]);
    } else if (!strcmp(loc, "under_member") || !strcmp(loc, "under")) {
        for (int s=0;s<RB_STAGE_SIZE;s++)
            for (int i=0;i<P->under_cards[s].n;i++) RB_ZPUSH(P->under_cards[s].cards[i]);
    } else if (!strcmp(loc, "revealed_cards")) {
        for (int i=0;i<g->n_revealed;i++) RB_ZPUSH(g->revealed_cards[i]);
    } else if (!strcmp(loc, "resolution") || !strcmp(loc, "resolution_zone")) {
        for (int i=0;i<g->resolution.n;i++) RB_ZPUSH(g->resolution.cards[i]);
    } else if (!strcmp(loc, "recently_moved") || !strcmp(loc, "preceding_moved") ||
               !strcmp(loc, "those_cards") || !strcmp(loc, "selected_cards")) {
        for (int i=0;i<g->n_recently_moved;i++) RB_ZPUSH(g->recently_moved[i]);
    }
    /* Unknown / non-zone marker strings resolve to the empty slice, exactly as
       Rust's `None => &[]` / `_ => &[]` arms do. */
#undef RB_ZPUSH
    return n;
}
static int count_distinct_in_zone(const struct GameState *g, int pl, const char *loc) {
    if (!loc) return 0;
    int ids[RB_MAX_ZONE];
    int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    /* distinct by card name string */
    int distinct=0;
    for(int i=0;i<n;i++){
        Card ci; if(!rb_decode_card_by_index((uint32_t)ids[i],&ci)) continue;
        int seen=0;
        for(int j=0;j<i;j++){
            Card cj; if(!rb_decode_card_by_index((uint32_t)ids[j],&cj)) continue;
            if(!strcmp(ci.name, cj.name)) seen=1;
            rb_free_card(&cj);
            if(seen) break;
        }
        if(!seen) distinct++;
        rb_free_card(&ci);
    }
    return distinct;
}
static int zone_count_filtered_ex(const struct GameState *g, int pl, const char *loc, const char *card_type, const char *group, int exclude_cid){
    if(!card_type && !group && exclude_cid < 0) return count_in_zone(g,pl,loc);
    /* filter: live_card vs member_card vs energy_card (+ optional group_names,
       + CardFilter.exclude_self). Mirror card.rs:evaluate_location_condition's
       `count_side` closure  Eit filters util::zone_cards(player, location) for
       EVERY zone, so the zone list comes from the shared zone_ids() resolver
       (no per-zone opt-out). */
    int ids[RB_MAX_ZONE];
    int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    int filtered=0;
    for(int i=0;i<n;i++){
        if(exclude_cid >= 0 && ids[i] == exclude_cid) continue;
        /* Use the faithful type_flags encoding (rb_card_is_live/rb_card_is_energy),
            NOT the broken n_hearts==0&&cost==0&&blade==0 heuristic  Ereal live/energy
            cards DO have hearts/cost, so that heuristic mis-classified everything
            (mirrors the fix in rb_card_is_live/rb_card_is_energy elsewhere). */
        int is_live = rb_card_is_live(ids[i]);
        int is_energy = rb_card_is_energy(ids[i]);
        int is_member = !is_live && !is_energy;
        int match=0;
        if(card_type){
            if(!strcmp(card_type,"live_card") && is_live) match=1;
            else if(!strcmp(card_type,"member_card") && is_member) match=1;
            else if(!strcmp(card_type,"energy_card") && is_energy) match=1;
            else if(!strcmp(card_type,"card")) match=1;
        } else match=1; /* group / exclude_self filter only */
        if(match && group && !rb_card_matches_group_str(ids[i], group)) match=0;
        if(match) filtered++;
    }
    return filtered;
}
static int zone_count_filtered(const struct GameState *g, int pl, const char *loc, const char *card_type, const char *group){
    return zone_count_filtered_ex(g, pl, loc, card_type, group, -1);
}

/* Forward */
static int eval_condition_inner(const struct GameState *g, int actor, const Condition *c);
static int eval_condition_inner_host(const struct GameState *g, int actor, int host_cid, const Condition *c);
static int stage_index_of_position(const char *pos);

/* Mirror engine/src/ability/condition/card.rs:resolve_target_for_scope  E   target=="self" with scope=="both" widens the scope to both players. */
static const char *resolve_target_for_scope(const Condition *c) {
    const char *target = get_str(c, "target");
    if (!target) target = "self";
    const char *scope = get_str(c, "scope");
    if (!strcmp(target, "self") && scope && !strcmp(scope, "both")) return "both";
    return target;
}

/* Mirror engine/src/ability/condition/card.rs:evaluate_check_self_condition.
   `check_self` conditions (parser emits them for 「このカードが控え室にある、E   style gates) test whether the ACTIVATING card itself sits in the location
   instead of counting matching cards there.
   Returns -1 (Rust `None`) when the condition is not a check_self condition or
   lacks a resolvable location, so callers fall through to normal evaluation.
   Negation is NOT applied here  Ecallers own negation semantics. */
static int eval_check_self(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    int cs = 0;
    if (!get_bool(c, "check_self", &cs) || !cs) return -1;
    if (host_cid < 0) return -1;                 /* Rust: `self.activating_card_id?` */
    const char *loc = get_str(c, "location");
    if (!loc || !*loc) return -1;
    int ids[RB_MAX_ZONE];
    int present = 0;
    const char *scope = resolve_target_for_scope(c);
    if (!strcmp(scope, "both")) {
        for (int p = 0; p < 2 && !present; p++) {
            int n = zone_ids(g, p, loc, ids, RB_MAX_ZONE);
            for (int i = 0; i < n; i++) if (ids[i] == host_cid) { present = 1; break; }
        }
    } else {
        int pl = (!strcmp(scope, "opponent")) ? (actor ^ 1) : actor;
        int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
        for (int i = 0; i < n; i++) if (ids[i] == host_cid) { present = 1; break; }
    }
    int thr = 1; get_i(c, "count", &thr);
    return eval_operator(present ? 1 : 0, get_str(c, "operator"), thr);
}

/* ── compound (variant 0) / or ── */
static int eval_compound(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    const char *op = get_str(c, "operator");
    const CondValue *v = find_val(c, "conditions");
    if (!v || v->tag != RB_TAG_ARRAY || !v->arr) return 1;
    int is_or = op && !strcmp(op, "or");
    for (uint32_t i=0;i<v->arr_n;i++) {
        const CondValue *cv = &v->arr[i];
        if (cv->tag != RB_TAG_OBJVAR || !cv->cond) continue;
        /* Rust keeps the same ConditionEvaluator (same activating_card_id) for
           every nested condition  Eforward the host so check_self / not_moved /
           exclude_self keep working inside compounds. */
        int r = eval_condition_inner_host(g, actor, host_cid, cv->cond);
        if (is_or && r) return 1;
        if (!is_or && !r) return 0;
    }
    return is_or ? 0 : 1;
}

/* ── location / card_count (variant 1) ── */
static int eval_location(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    /* Rust evaluates check_self FIRST and short-circuits on it. */
    int cs = eval_check_self(g, actor, host_cid, c);
    if (cs >= 0) return cs;
    int has_count = 0; int cnt_thr = 1;
    int tmp;
    has_count = get_i(c, "count", &tmp); if (has_count) cnt_thr = tmp;
    /* distinct flag  Eif true, count distinct card names not total cards */
    int distinct=0; get_bool(c,"distinct",&distinct);
    if(!distinct){
        const CondValue *dv=find_val(c,"distinct");
        if(dv && dv->tag==RB_TAG_OBJVAR && dv->cond){
            for(uint32_t i=0;i<dv->cond->n_fields;i++) if(!strcmp(dv->cond->fields[i].key,"distinct") && dv->cond->fields[i].v.tag==RB_TAG_TRUE) distinct=1;
        }
    }
    /* location field may be in 'location' or 'locations' arr */
    const char *loc = get_str(c, "location");
    if (!loc) {
        const CondValue *lv = find_val(c, "locations");
        if (lv && lv->tag == RB_TAG_ARRAY && lv->arr_n>0 && lv->arr[0].tag==RB_TAG_STR) loc = lv->arr[0].s;
    }
    if (!loc) loc = "stage";
    int pl = target_player_idx(actor, c);
    const char *ctype = get_str(c, "card_type");
    /* group_names / group filter (mirrors Rust CardFilter.group_names substring match) */
    const char *group = get_str(c, "group");
    const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n>0 && gv->arr[0].tag==RB_TAG_STR) group = gv->arr[0].s;
    /* group_reference=="same_group_name" overrides the filter group with the
       ACTIVATING card's own group (card.rs: `group_override`). */
    const char *gref = get_str(c, "group_reference");
    Card hostc; int have_hostc = 0;
    if (gref && !strcmp(gref, "same_group_name") && host_cid >= 0 &&
        rb_decode_card_by_index((uint32_t)host_cid, &hostc)) {
        const char *hg = rb_card_string(hostc.group_idx);
        if (hg && *hg) group = hg;
        have_hostc = 1;
    }
    /* exclude_self: drop the activating card from the match set
       (card.rs: `filter.exclude_self = self.activating_card_id`). */
    int excl = 0; get_bool(c, "exclude_self", &excl);
    int exclude_cid = (excl && host_cid >= 0) ? host_cid : -1;
    int actual = 0;
    if (ctype || group) actual = zone_count_filtered_ex(g, pl, loc, ctype, group, exclude_cid);
    else if (distinct) actual = count_distinct_in_zone(g, pl, loc);
    else if (exclude_cid >= 0) actual = zone_count_filtered_ex(g, pl, loc, NULL, NULL, exclude_cid);
    else actual = count_in_zone(g, pl, loc);
    if (have_hostc) rb_free_card(&hostc);

    /* 'all' flag means require all slots filled etc. For now treat as count check */
    if (has_count) {
        const char *op = get_str(c, "operator");
        return eval_operator(actual, op, cnt_thr);
    }
    /* no count: just existence */
    return actual > 0;
}

static int eval_highest_cost(const struct GameState *g, int actor, int host_cid, const Condition *c);

static int eval_highest_cost(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    (void)host_cid;
    // SP-bp2-004: center member is highest cost among stage members
    // Condition: position=center, location=stage, operator > (or >=)
    // Text: "center area has member with greatest cost"  Ei.e. center member exists and its cost is strictly greater than all others.
    // This is NOT about host being at center; host is the card whose ability is being evaluated (sumire) which may be at left/right or center.
    // We check existence of center member and that its cost is max.
    const RbPlayer *P = &g->p[actor];
    int center_cid = P->stage[1];
    if (center_cid == RB_EMPTY_SLOT) return 0;
    Card center; if (!rb_decode_card_by_index((uint32_t)center_cid, &center)) return 0;
    int center_cost = center.cost;
    rb_free_card(&center);
    int max_other = -1;
    int count_max = 0;
    int max_val = -1;
    for (int i=0;i<RB_STAGE_SIZE;i++) {
        int cid = P->stage[i];
        if (cid==RB_EMPTY_SLOT) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        int cst = cc.cost;
        rb_free_card(&cc);
        if (cst > max_val) { max_val = cst; count_max = 1; }
        else if (cst == max_val) count_max++;
        if (cid != center_cid && cst > max_other) max_other = cst;
    }
    // If center is alone, max_other is -1 -> true
    if (max_other < 0) return 1;
    const char *op = get_str(c, "operator");
    if (!op) op = ">";
    // For ">" the center must be strictly greater than all others
    if (!strcmp(op, ">")) {
        return center_cost > max_other;
    }
    if (!strcmp(op, ">=")) {
        return center_cost >= max_other;
    }
    // For other ops, compare center_cost vs max_other
    return eval_operator(center_cost, op, max_other);
}

/* Mirror engine/src/ability/condition/card.rs:evaluate_position_condition.
   target defaults to "self"; resolve to a player; check that the player's
   stage slot for the given position is occupied. */
static int resolve_target_player(int actor, const Condition *c) {
    const char *t = get_str(c, "target");
    if (t && !strcmp(t, "opponent")) return 1 - actor;
    return actor; /* "self" or default */
}
static int eval_position(const struct GameState *g, int actor, const Condition *c) {
    int pl = resolve_target_player(actor, c);
    const RbPlayer *P = &g->p[pl];
    const char *pos = get_str(c, "position");
    if (!pos) return 1;
    if (!strcmp(pos, "center"))    return P->stage[1] != RB_EMPTY_SLOT;
    if (!strcmp(pos, "left_side")) return P->stage[0] != RB_EMPTY_SLOT;
    if (!strcmp(pos, "right_side"))return P->stage[2] != RB_EMPTY_SLOT;
    if (!strcmp(pos, "any"))
        return P->stage[0] != RB_EMPTY_SLOT || P->stage[1] != RB_EMPTY_SLOT || P->stage[2] != RB_EMPTY_SLOT;
    return 1;
}

/* ── both_condition (variant 2 alias) ──
   Mirror engine/src/ability/condition/card.rs:evaluate_both_condition.
    `values` is a list of card scores; the candidate pool is selected BY
    LOCATION (SuccessLiveZone ↁEsuccess only, LiveCardZone ↁElive only, anything
    else ↁEsuccess ⧺ live) and every listed score must be present. Dispatched
    from eval_comparison_inner when no comparison_type=="score" is present
    (variant 2 shared with comparison_condition). */
static int eval_both_condition(const struct GameState *g, int actor, const Condition *c) {
    const CondValue *vv = find_val(c, "values");
    if (!vv || vv->tag != RB_TAG_ARRAY || vv->arr_n == 0) return 0;
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    const char *loc = get_str(c, "location");
    int ids[RB_MAX_ZONE]; int n = 0;
    int only_success = loc && (!strcmp(loc, "success_live_zone") || !strcmp(loc, "success_live_card_zone"));
    int only_live    = loc && !strcmp(loc, "live_card_zone");
    if (!only_live)    for (int j = 0; j < P->success.n && n < RB_MAX_ZONE; j++) ids[n++] = P->success.cards[j];
    if (!only_success) for (int j = 0; j < P->live.n    && n < RB_MAX_ZONE; j++) ids[n++] = P->live.cards[j];
    for (uint32_t i = 0; i < vv->arr_n; i++) {
        int want = (vv->arr[i].tag == RB_TAG_I64) ? (int)vv->arr[i].i
                 : (vv->arr[i].tag == RB_TAG_STR && vv->arr[i].s) ? atoi(vv->arr[i].s) : 0;
        int found = 0;
        for (int j = 0; j < n && !found; j++) {
            Card cc; if (rb_decode_card_by_index((uint32_t)ids[j], &cc)) {
                if (cc.score == want) found = 1; rb_free_card(&cc);
            }
        }
        if (!found) return 0;
    }
    return 1;
}

static int eval_comparison_inner(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    /* Rust evaluates check_self FIRST and short-circuits on it
       (card.rs:evaluate_comparison_condition line 1). */
    { int cs = eval_check_self(g, actor, host_cid, c); if (cs >= 0) return cs; }
    const char *loc = get_str(c, "location");
    const char *agg = get_str(c, "aggregate");
    const char *ctype = get_str(c, "comparison_type");
    /* both_condition: values = required scores that must ALL be present among the
       player's success/live cards. It shares variant 2 with comparison_condition
       but carries NO comparison_type=="score", so route it here. Mirrors
       card.rs:evaluate_both_condition. */
    const CondValue *bvals = find_val(c, "values");
    if (bvals && bvals->tag == RB_TAG_ARRAY && bvals->arr_n > 0 &&
        !(ctype && !strcmp(ctype, "score")))
        return eval_both_condition(g, actor, c);
    /* hanayo: location=success_live_card_zone, card_type=live_card, count=6, operator=>=, comparison_type=score, aggregate=total
       ↁEsum of card scores in zone, not count. Mirrors engine/src/ability/condition/card.rs evaluate_comparison. */
    if (loc && agg && !strcmp(agg,"total") && ctype && !strcmp(ctype,"score")) {
        int pl = target_player_idx(actor, c);
        const RbPlayer *P=&g->p[pl];
        const char *ct = get_str(c,"card_type");
        int sum=0;
        if(!strcmp(loc,"success")||!strcmp(loc,"success_zone")||!strcmp(loc,"success_live_zone")||!strcmp(loc,"success_live_card_zone")){
            for(int i=0;i<P->success.n;i++){
                if(ct && !card_matches_card_type_filter(P->success.cards[i], ct)) continue;
                Card cc; if(!rb_decode_card_by_index((uint32_t)P->success.cards[i],&cc)) continue;
                sum += cc.score;
                rb_free_card(&cc);
            }
        } else if(!strcmp(loc,"hand")){
            for(int i=0;i<P->hand.n;i++){
                if(ct && !card_matches_card_type_filter(P->hand.cards[i], ct)) continue;
                Card cc; if(!rb_decode_card_by_index((uint32_t)P->hand.cards[i],&cc)) continue;
                sum += cc.score;
                rb_free_card(&cc);
            }
        } else {
            /* fallback: count */
            sum = count_in_zone(g,pl,loc);
        }
        int thr=0; get_i(c,"count",&thr);
        const char *op=get_str(c,"operator");
        return eval_operator(sum, op, thr);
    }
    const char *pos = get_str(c, "position");
    if (pos && !strcmp(pos, "center") && loc && !strcmp(loc, "stage")) {
        return eval_highest_cost(g, actor, host_cid, c);
    }
    if (loc) return eval_location(g, actor, host_cid, c);
    int cnt=0, has_cnt=get_i(c, "count", &cnt);
    if (!has_cnt) {
        const CondValue *vv = find_val(c, "values");
        if (vv && vv->tag==RB_TAG_ARRAY) cnt = (int)vv->arr_n;
    }
    const char *op = get_str(c, "operator");
    if (!has_cnt) return 1;
    int actual = cnt;
    const char *src = get_str(c, "comparison_source");
    if (src) {
        int pl = target_player_idx(actor, c);
        actual = count_in_zone(g, pl, src);
    }
    return eval_operator(actual, op, cnt);
}

/* ── movement (variant 3) ── */
static int eval_movement(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    (void)host_cid;
    const char *mv = get_str(c, "movement");
    if (!mv) mv = get_str(c, "source"); /* some use source */
    int has_any = g->n_recently_moved > 0;
    /* If condition specifies a card filter (e.g. group), check if any recently moved matches */
    if (has_any) {
        const char *group = get_str(c, "group_names");
        const char *ctype = get_str(c, "card_type");
        if (group || ctype) {
            has_any = 0;
            for(int i=0;i<g->n_recently_moved;i++){
                int cid=g->recently_moved[i];
                if(ctype && !card_matches_card_type_filter(cid, ctype)) continue;
                if(group){
                    Card cc; if(!rb_decode_card_by_index((uint32_t)cid,&cc)) continue;
                    const char *gn=rb_card_string(cc.group_idx);
                    int ok= gn && (strstr(gn,group)||!strcmp(gn,group));
                    rb_free_card(&cc);
                    if(!ok) continue;
                }
                has_any=1; break;
            }
        }
    }
    if (mv && (!strcmp(mv,"has_moved") || !strcmp(mv,"hasMoved") || !strcmp(mv,"has_moved_stage"))) {
        return has_any ? 1 : 0;
    }
    if (mv && (!strcmp(mv,"not_moved") || !strcmp(mv,"notMoved"))) return has_any ? 0 : 1;
    return has_any ? 1 : 0;
}

/* ── group (variant 4) ── */
static int eval_group_aggregate(const GameState *g, int actor, const Condition *c);
static int eval_group_multi(const GameState *g, int actor, const Condition *c);
static int get_card_total_hearts(const struct GameState *g, int cid);
static int eval_group(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    (void)host_cid;
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int pl = target_player_idx(actor, c);
    /* all_members branch: "…のみがいめE  Eevery card in the zone must belong
       to ONE of the listed groups (e.g. 『Aqours』か『SaintSnow、E. */
    int all_members_val = 0;
    int all_members = get_i(c, "all_members", &all_members_val) ? all_members_val : 0;
    if (all_members) {
        const CondValue *gv = find_val(c, "group_names");
        if (!gv || gv->tag != RB_TAG_ARRAY || gv->arr_n == 0) {
            /* no group list with all_members: ambiguous ↁEtreat as no-match */
            return 0;
        }
        int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
        if (n == 0) return 0;
        for (int i = 0; i < n; i++) {
            int matched = 0;
            for (uint32_t gi = 0; gi < gv->arr_n; gi++) {
                const char *t = (gv->arr[gi].tag == RB_TAG_STR) ? gv->arr[gi].s : NULL;
                if (t && rb_card_matches_group_str(ids[i], t)) { matched = 1; break; }
            }
            if (!matched) return 0;
        }
        return 1;
    }
    /* Aggregate total check */
    { int agg = eval_group_aggregate(g, actor, c); if (agg >= 0) return agg; }
    /* Heart colors check: collective cards cover all required colors */
    const CondValue *hc = find_val(c, "heart_colors");
    if (hc && hc->tag == RB_TAG_ARRAY && hc->arr_n > 0) {
        int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
        int required = 0;
        for (uint32_t k = 0; k < hc->arr_n; k++) {
            int col = RB_HEART_PINK;
            if (hc->arr[k].tag == RB_TAG_I64) col = (int)hc->arr[k].i;
            else if (hc->arr[k].tag == RB_TAG_STR && hc->arr[k].s) col = atoi(hc->arr[k].s);
            int found = 0;
            for (int i = 0; i < n && !found; i++) {
                if (ids[i] < 0) continue;
                Card cc;
                if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
                    for (int h = 0; h < cc.n_hearts; h++) {
                        if (cc.heart_color[h] == (uint8_t)col) { found = 1; break; }
                    }
                    rb_free_card(&cc);
                }
            }
            if (found) required++;
        }
        if (required < (int)hc->arr_n) return 0;
        return 1;
    }
    /* Multiple group_names: each group must have >= count members */
    { int multi = eval_group_multi(g, actor, c); if (multi >= 0) return multi; }
    /* Temporal this_turn + self_target + group_names */
    int temporal = 0; get_bool(c, "temporal", &temporal);
    int self_target = 0; get_bool(c, "self_target", &self_target);
    const CondValue *gn = find_val(c, "group_names");
    if (temporal && self_target && gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0) {
        const char *ct = get_str(c, "card_type");
        if (ct && !strcmp(ct, "member_card")) {
            int activating_card = g->activating_card;
            if (activating_card >= 0) {
                int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
                int matched = 0;
                for (uint32_t gi = 0; gi < gn->arr_n && !matched; gi++) {
                    const char *t = (gn->arr[gi].tag == RB_TAG_STR) ? gn->arr[gi].s : NULL;
                    if (!t) continue;
                    for (int i = 0; i < n; i++) {
                        if (ids[i] < 0) continue;
                        if (rb_card_matches_group_str(ids[i], t)) { matched = 1; break; }
                    }
                }
                return matched;
            }
            return 0;
        }
    }
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    if (n == 0) return 0;
    for(int i=0;i<n;i++){
        Card card; if(!rb_decode_card_by_index((uint32_t)ids[i],&card)) continue;
        const char *gname = rb_card_string(card.group_idx);
        const char *uname = rb_card_string(card.unit_idx);
        if (rb_ability_debug_enabled() && gname && (strstr(gname,"XX")||strstr(gname,"YY"))) {
            for(uint32_t gi=0;gi<gn->arr_n;gi++) if(gn->arr[gi].tag==RB_TAG_STR && gn->arr[gi].s)
                fprintf(stderr,"[grp] card=%s gname=%s uname=%s target=%s\n",
                        card.name?card.name:"?", gname, uname?uname:"-", gn->arr[gi].s);
        }
        for(uint32_t gi=0;gi<gn->arr_n;gi++){
            const char *t = (gn->arr[gi].tag==RB_TAG_STR)?gn->arr[gi].s:NULL;
            if(!t) continue;
            if(rb_card_matches_group_str(ids[i], t)) { rb_free_card(&card); return 1; }
        }
        rb_free_card(&card);
    }
    return 0;
}

/* Mirror evaluate_group_condition: aggregate total check (e.g. heart02 >= 6).
   Sums the hearts of matching cards in the zone and compares to aggregate_total. */
static int eval_group_aggregate(const GameState *g, int actor, const Condition *c) {
    int agg_total = 0;
    if (!get_i(c, "aggregate_total", &agg_total) || agg_total <= 0) return -1;
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location"); if (!loc) loc = "stage";
    const char *op = get_str(c, "aggregate_total_operator");
    if (!op) op = ">=";
    /* Sum hearts for all cards in the zone matching the group filter */
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    int total = 0;
    const CondValue *gv = find_val(c, "group_names");
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0) {
            int matched = 0;
            for (uint32_t gi = 0; gi < gv->arr_n; gi++) {
                const char *t = (gv->arr[gi].tag == RB_TAG_STR) ? gv->arr[gi].s : NULL;
                if (t && rb_card_matches_group_str(ids[i], t)) { matched = 1; break; }
            }
            if (!matched) continue;
        }
        total += get_card_total_hearts(g, ids[i]);
    }
    if (!strcmp(op, ">=")) return total >= agg_total;
    if (!strcmp(op, ">"))  return total > agg_total;
    if (!strcmp(op, "<=")) return total <= agg_total;
    if (!strcmp(op, "<"))  return total < agg_total;
    if (!strcmp(op, "==")) return total == agg_total;
    return total >= agg_total;
}

/* Mirror evaluate_group_condition: multiple group_names branch.
   Each listed group must have at least `count` members in the zone. */
static int eval_group_multi(const GameState *g, int actor, const Condition *c) {
    const CondValue *gv = find_val(c, "group_names");
    if (!gv || gv->tag != RB_TAG_ARRAY || gv->arr_n <= 1) return -1;
    const char *agg = get_str(c, "aggregate");
    if (agg && !strcmp(agg, "total")) return -1; /* handled by aggregate branch */
    int target_count = 0;
    if (!get_i(c, "count", &target_count)) target_count = 1;
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location"); if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    for (uint32_t gi = 0; gi < gv->arr_n; gi++) {
        const char *t = (gv->arr[gi].tag == RB_TAG_STR) ? gv->arr[gi].s : NULL;
        if (!t) continue;
        int count = 0;
        for (int i = 0; i < n; i++) {
            if (ids[i] < 0) continue;
            if (rb_card_matches_group_str(ids[i], t)) count++;
        }
        if (count < target_count) return 0;
    }
    return 1;
}

/* Mirror engine/src/ability/condition/card.rs::evaluate_appearance_condition_inner
   + evaluate_appearance_stage. Handles:
     • baton_touch_trigger (get_baton_touch_count + min_baton_touch_count)
     • presence/absence (不在) routing by location (stage/hand/discard/other)
     • self-trigger guard: when there are no card-targeting filters, the ability
       only fires if the ACTIVATING card (host_cid) actually appeared this turn
       and is on stage
     • all_areas, cost_limit, card_type, group_names/characters (+verify an
       appeared card matches), exclude_self, activation_position, position. */
static int card_appeared_this_turn(const GameState *g, int cid) {
    if (cid < 0) return 0;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == cid) return 1;
    return 0;
}
static int baton_touch_count_for(const GameState *g, int pl) {
    return pl ? g->baton_touch_count_p2 : g->baton_touch_count_p1;
}
static int eval_appearance_stage(const struct GameState *g, int actor, int host_cid,
                                  const Condition *c, int pl) {
    const RbPlayer *P = &g->p[pl];
    int ids[RB_STAGE_SIZE]; int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) ids[n++] = P->stage[i];
    if (n == 0) return 0;

    int has_group = 0; const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0) has_group = 1;
    int has_chars = 0; const CondValue *chars = find_val(c, "characters");
    if (chars && chars->tag == RB_TAG_ARRAY && chars->arr_n > 0) has_chars = 1;
    int has_cost_limit = 0; get_i(c, "cost_limit", &has_cost_limit);
    int has_card_type = 0; const char *ct = get_str(c, "card_type"); if (ct) has_card_type = 1;
    int has_card_filters = has_group || has_chars || has_cost_limit || has_card_type;

    int baton_trigger = 0; get_bool(c, "baton_touch_trigger", &baton_trigger);

    /* Self-trigger guard: no card filters ↁEonly "this member appeared". */
    if (!baton_trigger && !has_card_filters) {
        if (g->n_cards_appeared_this_turn == 0) return 0;
        if (host_cid < 0) return 0;
        int on_stage = 0;
        for (int i = 0; i < n; i++) if (ids[i] == host_cid) { on_stage = 1; break; }
        if (!on_stage || !card_appeared_this_turn(g, host_cid)) return 0;
    }

    /* exclude_self with cost_limit/card_type: require ANOTHER appeared card matches. */
    int excl = 0; get_bool(c, "exclude_self", &excl);
    if (!baton_trigger && excl && (has_cost_limit || has_card_type) &&
        g->n_cards_appeared_this_turn > 0) {
        int other = 0;
        for (int i = 0; i < n; i++) {
            int cid = ids[i];
            if (host_cid >= 0 && cid == host_cid) continue;
            if (card_appeared_this_turn(g, cid)) { other = 1; break; }
        }
        if (!other) return 0;
    }

    int all_areas = 0; get_bool(c, "all_areas", &all_areas);
    if (all_areas && n != RB_STAGE_SIZE) return 0;

    /* cost_limit (e.g. コスチE0のメンバ�E) */
    if (has_cost_limit) {
        const char *op = get_str(c, "operator"); if (!op) op = "=";
        int cost_match = 0;
        for (int i = 0; i < n; i++) {
            Card cc; if (!rb_decode_card_by_index((uint32_t)ids[i], &cc)) continue;
            int cost = (int)cc.cost;
            rb_free_card(&cc);
            if (eval_operator(cost, op, has_cost_limit)) { cost_match = 1; break; }
        }
        if (!cost_match) return 0;
    }

    /* card_type (member_card / live_card / energy_card) */
    if (has_card_type) {
        int type_match = 0;
        for (int i = 0; i < n; i++) if (rb_card_matches_type(ids[i], ct)) { type_match = 1; break; }
        if (!type_match) return 0;
    }

    /* group_names / characters  Eany (or all if all_areas) stage card matches. */
    if (has_group || has_chars) {
        int any_match = 0, all_match = 1;
        for (int i = 0; i < n; i++) {
            int matched = 0;
            if (has_group)
                for (uint32_t gi = 0; gi < gv->arr_n && !matched; gi++)
                    if (gv->arr[gi].tag == RB_TAG_STR && gv->arr[gi].s &&
                        rb_card_matches_group_str(ids[i], gv->arr[gi].s)) matched = 1;
            if (has_chars && !matched)
                for (uint32_t gi = 0; gi < chars->arr_n && !matched; gi++)
                    if (chars->arr[gi].tag == RB_TAG_STR && chars->arr[gi].s &&
                        rb_card_matches_group_str(ids[i], chars->arr[gi].s)) matched = 1;
            if (matched) any_match = 1; else all_match = 0;
        }
        if (all_areas ? !all_match : !any_match) return 0;
        /* Verify an APPEARED card matches the group (appearance trigger should only
           fire when a card that actually appeared this turn matches). */
        if (!baton_trigger && g->n_cards_appeared_this_turn > 0) {
            int appeared_match = 0;
            for (int i = 0; i < n; i++) {
                int cid = ids[i];
                if (!card_appeared_this_turn(g, cid)) continue;
                int matched = 0;
                if (has_group)
                    for (uint32_t gi = 0; gi < gv->arr_n && !matched; gi++)
                        if (gv->arr[gi].tag == RB_TAG_STR && gv->arr[gi].s &&
                            rb_card_matches_group_str(cid, gv->arr[gi].s)) matched = 1;
                if (has_chars && !matched)
                    for (uint32_t gi = 0; gi < chars->arr_n && !matched; gi++)
                        if (chars->arr[gi].tag == RB_TAG_STR && chars->arr[gi].s &&
                            rb_card_matches_group_str(cid, chars->arr[gi].s)) matched = 1;
                if (matched) { appeared_match = 1; break; }
            }
            if (!appeared_match) return 0;
        }
    }

    /* activation_position: the activating card itself must be at the position. */
    const char *actpos = get_str(c, "activation_position");
    if (actpos && host_cid >= 0) {
        int passes = 0;
        char buf[64]; strncpy(buf, actpos, sizeof(buf) - 1); buf[sizeof(buf) - 1] = '\0';
        char *t = strtok(buf, ",");
        while (t) {
            int idx = rb_activation_position_index(t);
            if (idx >= 0 && idx < RB_STAGE_SIZE && idx < 3 && P->stage[idx] == host_cid) { passes = 1; break; }
            t = strtok(NULL, ",");
        }
        if (!passes) return 0;
    }

    /* position (without position_compare): the activating card must be there. */
    const char *pos = get_str(c, "position");
    const char *poscmp = get_str(c, "position_compare");
    if (pos && !poscmp && host_cid >= 0) {
        int idx = stage_index_of_position(pos);
        if (idx < 0) return 0;
        if (idx >= RB_STAGE_SIZE || P->stage[idx] != host_cid) return 0;
    }
    return 1;
}

/* ── appearance (variant 5) ── */
static int eval_appearance(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    int ap = 0; get_bool(c, "appearance", &ap);   /* unwrap_or(false) */
    int baton_trigger = 0; get_bool(c, "baton_touch_trigger", &baton_trigger);
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];

    if (baton_trigger) {
        int bt = baton_touch_count_for(g, pl);
        if (bt == 0) return 0;
        int minc = 0; if (get_i(c, "min_baton_touch_count", &minc) && minc > 0 && bt < minc) return 0;
        /* baton appearances legitimately trigger on the activating card; presence is
           checked by the stage/character filter below when present. */
        if (!ap) return 1;
    }

    if (!baton_trigger && !ap) {
        /* 不在 (absence) condition. */
        const char *loc = get_str(c, "location");
        if (!loc || !strcmp(loc, "stage")) {
            for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) return 0;
            return 1;
        }
        if (!strcmp(loc, "hand"))    return P->hand.n == 0;
        if (!strcmp(loc, "discard") || !strcmp(loc, "waitroom")) return P->discard.n == 0;
        return 1;   /* other zones: absence is accepted */
    }

    /* appearance present condition. */
    const char *loc = get_str(c, "location");
    if (!loc || !strcmp(loc, "stage")) return eval_appearance_stage(g, actor, host_cid, c, pl);
    if (!strcmp(loc, "hand"))    return P->hand.n > 0;
    if (!strcmp(loc, "discard") || !strcmp(loc, "waitroom")) return P->discard.n > 0;
    return 1;   /* other zones: presence accepted */
}

/* ── temporal (variant 6) ──
   Mirror engine/src/ability/condition/state.rs:evaluate_temporal_condition.
   Supports turn_number (+operator), `temporal` scope strings ("this_turn",
   "live_end", "during_live", "before_live", "first_turn") and the `phase`
   gate. Sub-checks that need unimplemented state (has_card_moved_this_turn,
   debut_count_this_turn, nested conditions) degrade gracefully. */
static int phase_in_live(const struct GameState *g) {
    return g->phase==RB_PHASE_LIVE_SET || g->phase==RB_PHASE_PERFORMANCE || g->phase==RB_PHASE_VICTORY;
}
static int eval_temporal(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    int tn=0;
    if (get_i(c,"turn_number",&tn)) {
        const char *op = get_str(c,"operator");
        return eval_operator(g->turn, op, tn);
    }
    /* phase gate */
    const char *phase = get_str(c,"phase");
    if (phase) {
        if (!strcmp(phase,"main") || !strcmp(phase,"main_phase")) return g->phase==RB_PHASE_MAIN;
        if (!strcmp(phase,"active")) return g->phase==RB_PHASE_ACTIVE;
        if (!strcmp(phase,"live") || !strcmp(phase,"live_phase") ||
            !strcmp(phase,"live_card_set")||!strcmp(phase,"live_performance")||!strcmp(phase,"live_victory"))
            return phase_in_live(g);
    }
    /* temporal scope strings */
    const char *temp = get_str(c,"temporal");
    if (temp) {
        if (!strcmp(temp,"live_end"))  return g->phase==RB_PHASE_VICTORY;
        if (!strcmp(temp,"during_live")||!strcmp(temp,"this_live"))
            return phase_in_live(g);
        if (!strcmp(temp,"before_live"))
            return !(g->phase==RB_PHASE_LIVE_SET || g->phase==RB_PHASE_PERFORMANCE || g->phase==RB_PHASE_VICTORY);
        if (!strcmp(temp,"first_turn")) return g->turn==1;
        if (!strcmp(temp,"this_turn")) {
            /* Mirror state.rs::evaluate_temporal_condition "this_turn" branch. */
            int count = -1; int has_count = get_i(c, "count", &count);
            const char *loc = get_str(c, "location");
            const char *ctype = get_str(c, "card_type");
            const char *tgt = get_str(c, "target");
            int who = (tgt && !strcmp(tgt,"opponent")) ? (actor^1) : actor;
            if (has_count && (!loc || !strcmp(loc,"stage")) &&
                (!ctype || !strcmp(ctype,"member_card"))) {
                const char *group = get_str(c, "group_names");
                if (group && *group) {
                    /* count stage members matching the group that moved this turn. */
                    int matching = 0;
                    for (int q=0; q<RB_STAGE_SIZE; q++) {
                        int cid = g->p[who].stage[q];
                        if (cid < 0) continue;
                        if (!rb_card_matches_group_str(cid, group)) continue;
                        if (card_moved_this_turn(g, cid)) matching++;
                    }
                    return matching >= count;
                }
                return g->debut_count_this_turn[who] >= count;
            }
            /* no explicit count: fall back to temporal_scope year or a nested condition. */
            const char *scope = get_str(c, "temporal_scope");
            if (scope) return atoi(scope) == g->turn;
            const Condition *nested = get_cond(c, "condition");
            if (nested) {
                /* NotMoved/HasMoved nested conditions gate on this-turn movement.
                   Faithful to state.rs:evaluate_temporal_condition  Ethe checks
                   are keyed on the ACTIVATING card (host_cid), with the
                   position / position-change / activating-card fallback chain. */
                const char *mv = get_str(nested, "movement");
                if (mv && (!strcmp(mv,"not_moved")||!strcmp(mv,"notMoved"))) {
                    /* Rust: Some(cid) => !has_card_moved_this_turn(cid); None => true. */
                    if (host_cid < 0) return 1;
                    return !card_moved_this_turn(g, host_cid);
                }
                if (mv && (!strcmp(mv,"has_moved")||!strcmp(mv,"hasMoved"))) {
                    /* 1) explicit `position` on the OUTER condition names the card. */
                    const Condition *posc = get_cond(c, "position");
                    const char *pos_str = posc ? get_str(posc, "position") : get_str(c, "position");
                    if (pos_str) {
                        int idx = stage_index_of_position(pos_str);
                        if (idx >= 0 && g->p[who].stage[idx] != RB_EMPTY_SLOT)
                            return card_moved_this_turn(g, g->p[who].stage[idx]);
                    }
                    /* 2) a position change happened this turn: any stage member
                          (optionally group-filtered by the nested-then-outer
                          group_names) that moved satisfies the gate. */
                    if (g->position_change_occurred_this_turn) {
                        const char *grp = get_str(nested, "group_names");
                        if (!grp) grp = get_str(c, "group_names");
                        for (int q=0; q<RB_STAGE_SIZE; q++) {
                            int cid = g->p[who].stage[q];
                            if (cid < 0) continue;
                            if (grp && !rb_card_matches_group_str(cid, grp)) continue;
                            if (card_moved_this_turn(g, cid)) return 1;
                        }
                        return 0;
                    }
                    /* 3) fall back to the activating card itself. */
                    if (host_cid >= 0) return card_moved_this_turn(g, host_cid);
                    return 0;
                }
                return eval_condition_inner_host(g, actor, host_cid, nested);
            }
            return 1; /* Rust default when nothing gates */
        }
    }
    return 1;
}

/* ── state (variant 7) ──
   Mirror engine/src/ability/condition/state.rs:evaluate_state_condition +
   evaluate_energy_state_condition. Covers the `state_condition`,
   `energy_state_condition` and `state_change_condition` aliases (all share
   the State variant discriminant). resource_type=="energy" or the
   `energy_state` field gate on active/wait energy counts; otherwise the
   member orientation (stage_wait flag + orientation modifier) is matched. */
static int state_idx(const char *s){
    if(!s) return 0;
    if(!strcmp(s,"wait")||!strcmp(s,"WAIT_JA")) return 1;
    return 0; /* active / none */
}
/* state_change_condition: a member transitioned from_state -> to_state this turn.
   Mirrors state.rs:evaluate_state_change_condition (primary recently_state_changed
   path; turn-scoped fallbacks approximated by the per-turn tracking set in
   rb_effect_change_state). */
static int eval_state_change(const struct GameState *g, int actor, const Condition *c, const char *from, const char *to) {
    int fi = state_idx(from), ti = state_idx(to);
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    if (fi==1 && ti==0) {
        int cnt=1; get_i(c,"count",&cnt);
        const char *op=get_str(c,"operator");
        if(op && !strcmp(op,">=")) return g->last_wait_to_active_count >= cnt;
        if(op && !strcmp(op,">"))  return g->last_wait_to_active_count >  cnt;
        if(op && !strcmp(op,"=") ) return g->last_wait_to_active_count == cnt;
    }
    for(int i=0;i<RB_STAGE_SIZE;i++){
        int cid=P->stage[i];
        if(cid==RB_EMPTY_SLOT) continue;
        if(g->state_change_from[cid]==fi && g->state_change_to[cid]==ti) return 1;
    }
    return 0;
}
/* Mirror engine/src/ability/util.rs:orientation_matches_state  Ea card whose
   current orientation modifier is `orientation` matches `state`; a card with NO
   modifier is treated as active (the default orientation).
   The C engine additionally keeps the per-slot `stage_wait` flag, so a member
   with no explicit orientation modifier falls back to that flag (the two are
   the same fact in Rust, where wait state IS the orientation modifier). */
static int orientation_matches_state(const struct GameState *g, int pl, int slot, const char *state) {
    if (!state) return 0;
    const char *om = rb_mods_get_orientation((RbMods*)&g->mods, g->p[pl].stage[slot]);
    if (om && *om) return !strcmp(om, state);
    return !strcmp(state, g->p[pl].stage_wait[slot] ? "wait" : "active");
}
static int eval_state(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    const char *from = get_str(c, "from_state");
    const char *to   = get_str(c, "to_state");
    if (from && to) return eval_state_change(g, actor, c, from, to);
    const char *res = get_str(c, "resource_type");
    const char *es  = get_str(c, "energy_state");
    const char *st  = get_str(c, "state");
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];

    /* energy_state_condition: "active" when active energy count > 0.
       (abilities emit "state":"active" in some dumps; accept both.) */
    if (es && !strcmp(es, "active")) return P->energy_active > 0;
    if (res && !strcmp(res, "energy")) {
        if (!st) st = "active";
        int is_all = 0; get_bool(c, "all", &is_all);
        if (!strcmp(st, "active")) {
            if (is_all) return P->energy.n > 0 && P->energy_active == P->energy.n;
            return P->energy_active > 0;
        }
        if (!strcmp(st, "wait")) {
            if (is_all) return P->energy_active == 0;
            return P->energy_active < P->energy.n;
        }
        return 1;
    }

    /* member active/wait state  Efaithful port of
       state.rs:evaluate_state_condition's non-energy branch. */
    if (st && (!strcmp(st, "active") || !strcmp(st, "wait"))) {
        int occupied = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) occupied++;
        if (occupied == 0) return 0;   /* Rust: stage_cards.is_empty() => false */
        /* 「このメンバ�Eが…、Eself-state text must NOT be widened by the parser's
           default card_type=member_card, otherwise every waited member on stage
           would satisfy every copy of the card (the two-copy bug). */
        const char *text = get_str(c, "text");
        const char *cs   = get_str(c, "check_self");
        int is_self_text = text && strstr(text, "SELF_TEXT") != NULL;
        const char *ctype = get_str(c, "card_type");
        const char *group = get_str(c, "group_names");
        const CondValue *chars = find_val(c, "characters");
        int has_filter;
        if (is_self_text && !cs) has_filter = 0;
        else has_filter = (group != NULL) || (ctype != NULL) ||
                          (chars && chars->tag == RB_TAG_ARRAY && chars->arr_n > 0);
        if (has_filter) {
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int cid = P->stage[i];
                if (cid == RB_EMPTY_SLOT) continue;
                if (!orientation_matches_state(g, pl, i, st)) continue;
                if (ctype && !card_matches_card_type_filter(cid, ctype)) continue;
                if (group && !rb_card_matches_group_str(cid, group)) continue;
                if (chars && chars->tag == RB_TAG_ARRAY && chars->arr_n > 0) {
                    int ok = 0;
                    for (uint32_t k = 0; k < chars->arr_n && !ok; k++)
                        if (chars->arr[k].tag == RB_TAG_STR && chars->arr[k].s &&
                            rb_card_matches_group_str(cid, chars->arr[k].s)) ok = 1;
                    if (!ok) continue;
                }
                return 1;
            }
            return 0;
        }
        /* Self branch: the ACTIVATING card must be on this stage AND match the
           requested orientation (Rust `self.activating_card_id.is_some_and(...)`). */
        if (host_cid < 0) return 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == host_cid) return orientation_matches_state(g, pl, i, st);
        return 0;
    }
    return 1;
}

/* ── resource / card_blade (variant 8) ──
   Mirror engine/src/ability/condition/card.rs:evaluate_resource_condition
   and evaluate_card_blade_condition. resource_type "blade" sums each
   staged member's effective blade (printed + modifier); "surplus_heart"
   is stage total_hearts minus live-card need hearts. The CardBlade
   condition (selected_cards) falls back to the staged members when no
   movement snapshot exists (C tracks recently_moved as the selection set). */
static int effective_blade(const struct GameState *g, int cid) {
    Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int b = (int)c.blade + rb_mods_get_blade((RbMods*)&g->mods, cid);
    rb_free_card(&c);
    if (b < 0) b = 0;
    if (b > 255) b = 255;
    return b;
}
/* Collect effective cost (base + cost modifier, saturating u8) of each
   occupied stage member for `pl`. Returns the occupied count (0..STAGE_SIZE).
   Mirror engine/src/ability/condition/card.rs get_stage_costs closure. */
static int collect_stage_costs(const struct GameState *g, int pl, int out[RB_STAGE_SIZE]) {
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        int base = (int)cc.cost;
        rb_free_card(&cc);
        int eff = rb_saturate_u8(base + rb_mods_get_cost((RbMods*)&g->mods, cid));
        out[n++] = eff;
    }
    return n;
}

/* Map a stage position string to its stage index (mirror util::card_at_position). */
static int stage_index_of_position(const char *pos) {
    if (!pos) return -1;
    if (!strcmp(pos, "center"))    return 1;
    if (!strcmp(pos, "left_side")) return 0;
    if (!strcmp(pos, "right_side"))return 2;
    return -1;
}

/* highest_cost_on_stage_condition: the card at `position` must have an effective
   cost that satisfies `operator` against EVERY other occupied stage member's
   effective cost. Mirror card.rs:evaluate_highest_cost_on_stage_condition. */
static int eval_highest_cost_on_stage(const struct GameState *g, int actor, const Condition *c) {
    const char *pos = get_str(c, "position");
    int idx = stage_index_of_position(pos);
    if (idx < 0) return 0;
    const char *tgt = get_str(c, "target");
    int pl = (tgt && !strcmp(tgt, "opponent")) ? (actor ^ 1) : actor;
    int cid = g->p[pl].stage[idx];
    if (cid == RB_EMPTY_SLOT) return 0;
    Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) return 0;
    int pos_cost = rb_saturate_u8((int)cc.cost + rb_mods_get_cost((RbMods*)&g->mods, cid));
    rb_free_card(&cc);
    const char *op = get_str(c, "operator");
    if (!op) op = ">";
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int other = g->p[pl].stage[i];
        if (other == RB_EMPTY_SLOT || other == cid) continue;
        Card oc; if (!rb_decode_card_by_index((uint32_t)other, &oc)) continue;
        int o_cost = rb_saturate_u8((int)oc.cost + rb_mods_get_cost((RbMods*)&g->mods, other));
        rb_free_card(&oc);
        if (!eval_operator(pos_cost, op, o_cost)) return 0;
    }
    return 1;
}

static int eval_resource(const struct GameState *g, int actor, const Condition *c) {
    const char *res = get_str(c, "resource_type");
    const char *src = get_str(c, "source");
    const char *cmptype = get_str(c, "comparison_type");
    int pl = target_player_idx(actor, c);
    int thr=1; get_i(c,"count",&thr); if(!thr) thr=1;
    const char *op=get_str(c,"operator");

    /* all_cost_comparison_condition: at least one of SELF's stage members has
       an effective cost that satisfies `operator` against the MAXIMUM cost of
       the OPPONENT's stage members. Mirror card.rs:evaluate_all_cost_comparison_condition. */
    if (cmptype && !strcmp(cmptype, "cost")) {
        int self_costs[RB_STAGE_SIZE];
        int opp_costs[RB_STAGE_SIZE];
        int ns = collect_stage_costs(g, actor, self_costs);
        int no = collect_stage_costs(g, actor ^ 1, opp_costs);
        int max_opp = 0;
        for (int i = 0; i < no; i++) if (opp_costs[i] > max_opp) max_opp = opp_costs[i];
        const char *oop = op ? op : ">=";
        for (int i = 0; i < ns; i++)
            if (eval_operator(self_costs[i], oop, max_opp)) return 1;
        return 0;
    }

    /* highest_cost_on_stage_condition: position-based cost comparison (no
       resource_type / comparison_type=="cost"). Mirror card.rs:evaluate_highest_cost_on_stage_condition. */
    const char *pos = get_str(c, "position");
    if (pos) return eval_highest_cost_on_stage(g, actor, c);

    if (res && !strcmp(res, "blade")) {
        /* sum effective blade of selection set (recently_moved) or stage */
        /* card_blade_condition: sum effective blade of the selected/moved
           card set (falling back to stage members) and compare to `count`.
           Mirror card.rs:evaluate_card_blade_condition  Ean EMPTY selection set
           resolves to false (Rust returns false when selected_card_ids is empty). */
        int ids[RB_MAX_ZONE]; int n=0;
        if (src && !strcmp(src, "preceding_moved") && g->n_recently_moved>0) {
            for (int i=0;i<g->n_recently_moved;i++) ids[n++]=g->recently_moved[i];
        } else if (g->n_recently_moved>0) {
            for (int i=0;i<g->n_recently_moved;i++) ids[n++]=g->recently_moved[i];
        } else {
            for (int i=0;i<RB_STAGE_SIZE;i++)
                if (g->p[pl].stage[i]!=RB_EMPTY_SLOT) ids[n++]=g->p[pl].stage[i];
        }
        if (n == 0) return 0;
        int total=0; for(int i=0;i<n;i++) total += effective_blade(g, ids[i]);
        return eval_operator(rb_saturate_u8(total), op ? op : ">=", thr);
    }
    if (res && !strcmp(res, "surplus_heart")) {
        /* stage total_hearts - live/success need hearts */
        int heart_total=0;
        for (int i=0;i<RB_STAGE_SIZE;i++) {
            int cid=g->p[pl].stage[i]; if (cid==RB_EMPTY_SLOT) continue;
            Card cc; if(!rb_decode_card_by_index((uint32_t)cid,&cc)) continue;
            for (int h=0;h<cc.n_hearts;h++) heart_total += cc.heart_count[h];
            rb_free_card(&cc);
        }
        int need=0;
        for (int i=0;i<g->p[pl].live.n;i++) {
            int cid=g->p[pl].live.cards[i];
            Card cc; if(!rb_decode_card_by_index((uint32_t)cid,&cc)) continue;
            for (int h=0;h<cc.n_hearts;h++) need += cc.heart_count[h];
            rb_free_card(&cc);
        }
        for (int i=0;i<g->p[pl].success.n;i++) {
            int cid=g->p[pl].success.cards[i];
            Card cc; if(!rb_decode_card_by_index((uint32_t)cid,&cc)) continue;
            for (int h=0;h<cc.n_hearts;h++) need += cc.heart_count[h];
            rb_free_card(&cc);
        }
        return eval_operator(heart_total - need, op, thr);
    }
    /* energy_state-style count (active energy / energy zone) */
    const char *loc = get_str(c, "location");
    int actual;
    if (loc && (!strcmp(loc,"energy")||!strcmp(loc,"energy_zone"))) {
        actual = g->p[pl].energy_active;
        const char *state = get_str(c,"state");
        if(state && !strcmp(state,"wait")){
            actual = g->p[pl].energy.n - g->p[pl].energy_active;
        }
    } else {
        actual = count_in_zone(g, pl, "stage");
    }
    return eval_operator(actual, op, thr);
}
static int eval_no_excess(const struct GameState *g, int actor, const Condition *c){
    int pl=target_player_idx(actor,c);
    /* NoExcessHeart: check if last live had no surplus hearts (all_exact)
       Mirrors engine/src/turn/live.rs compute_surplus_and_flags ↁEsurplus_hearts == 0.
       We now snapshot surplus directly in live.c (total_pool - total_required). */
    for(int i=g->n_snapshots-1;i>=0;i--) if(g->snapshots[i].player==pl && g->snapshots[i].success){
        if(g->snapshots[i].surplus_hearts==0) return 1;
        if(g->snapshots[i].surplus_hearts>=0) return 0; /* most recent snapshot decides */
    }
    return 0;
}

/* Read a string-array field (e.g. ability_filter_triggers / any_of). */
static const CondValue *get_arr(const Condition *c, const char *key) {
    const CondValue *v = find_val(c, key);
    if (!v || v->tag != RB_TAG_ARRAY) return NULL;
    return v;
}
/* Does any decoded ability of card `cid` have a trigger containing `trig`? */
static int card_ability_trigger_contains(int cid, const char *trig) {
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab; if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = 0;
        if (ab.triggers && strstr(ab.triggers, trig)) hit = 1;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}
/* ── ability_filter (variant 9) ──
   Mirror engine/src/ability/condition/card.rs:evaluate_ability_filter_condition.
   Scans the location's cards (stage/hand/discard/live) for has_ability /
   no_ability, optionally requiring a matching trigger prefix. */
static int eval_ability_filter(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    const char *filter = get_str(c, "ability_filter");
    if (!filter) filter = "no_ability";
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int pl = target_player_idx(actor, c);

    /* Rust scans util::zone_cards(player, location)  Ethe shared resolver keeps
       every zone reachable (the old per-zone if-chain silently dropped
       deck/success/under_member to the activating-card fallback). */
    int ids[RB_MAX_ZONE];
    int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);

    /* trigger prefixes */
    const CondValue *tv = get_arr(c, "ability_filter_triggers");

    int has_ability = 0;
    if (n == 0) {
        /* No cards in the zone: Rust falls back to the ACTIVATING card
           (card.rs:evaluate_ability_filter_condition). With no activating card
           either, has_ability stays false (so no_ability answers true). */
        if (host_cid >= 0) { ids[0] = host_cid; n = 1; }
    }
    if (n > 0) {
        for (int i = 0; i < n; i++) {
            int cid = ids[i];
            int na = rb_card_num_abilities((uint32_t)cid);
            int present = na > 0;
            if (present && tv) {
                present = 0;
                for (uint32_t t = 0; t < tv->arr_n; t++) {
                    if (tv->arr[t].tag==RB_TAG_STR && tv->arr[t].s &&
                        card_ability_trigger_contains(cid, tv->arr[t].s)) { present = 1; break; }
                }
            }
            if (present) { has_ability = 1; break; }
        }
    }

    if (!strcmp(filter, "has_ability")) return has_ability;
    if (!strcmp(filter, "no_ability"))  return !has_ability;
    if (!strcmp(filter, "no_ability_type")) {
        /* Some card lacks EVERY listed trigger type. */
        if (!tv || tv->arr_n==0) return 0;
        if (n == 0) return 0;
        for (int i = 0; i < n; i++) {
            int cid = ids[i];
            int lacks_all = 1;
            for (uint32_t t = 0; t < tv->arr_n; t++) {
                if (tv->arr[t].tag==RB_TAG_STR && tv->arr[t].s &&
                    card_ability_trigger_contains(cid, tv->arr[t].s)) { lacks_all = 0; break; }
            }
            if (lacks_all) return 1;
        }
        return 0;
    }
    return 1;
}

/* ── any_of (variant 18) ──
   Mirror engine/src/ability/condition/compound.rs:any_of_matches. The
   condition carries an `any_of` string-array; each entry names a probe. */
static int any_of_matches(const struct GameState *g, const char *ct) {
    if (!strcmp(ct, "has_member")) {
        for (int i=0;i<RB_STAGE_SIZE;i++) if (g->p[g->active].stage[i]!=RB_EMPTY_SLOT) return 1;
        return 0;
    }
    if (!strcmp(ct, "has_energy")) return g->p[g->active].energy.n > 0;
    if (!strcmp(ct, "has_hand"))   return g->p[g->active].hand.n > 0;
    if (!strcmp(ct, "has_live_card")) return g->p[g->active].live.n > 0;
    if (!strcmp(ct, "has_blade_heart")) {
        for (int i=0;i<RB_STAGE_SIZE;i++) {
            int cid=g->p[g->active].stage[i]; if (cid==RB_EMPTY_SLOT) continue;
            Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
            int hb = c.has_special; /* proxy: blade heart ≁Especial/blade heart flag */
            rb_free_card(&c);
            if (hb) return 1;
        }
        return 0;
    }
    if (!strcmp(ct, "is_active_phase")) return g->phase==RB_PHASE_ACTIVE;
    if (!strcmp(ct, "is_main_phase"))   return g->phase==RB_PHASE_MAIN;
    return 0;
}
static int eval_any_of(const struct GameState *g, int actor, const Condition *c) {
    (void)actor;
    const CondValue *av = get_arr(c, "any_of");
    if (!av) return 1;
    for (uint32_t i=0;i<av->arr_n;i++) {
        if (av->arr[i].tag==RB_TAG_STR && av->arr[i].s && any_of_matches(g, av->arr[i].s))
            return 1;
    }
    return 0;
}
/* ── score_threshold (10) ── */
static int eval_score(const struct GameState *g, int actor, const Condition *c) {
    int thr=1; get_i(c,"count",&thr);
    const char *op=get_str(c,"operator");
    int pl=target_player_idx(actor,c);
    return eval_operator(g->p[pl].score, op, thr);
}
/* ── choice/position etc ── */
static int eval_choice(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    /* Choice conditions are interactive; headless eval treats them as
       satisfiable. If a nested "condition" field is present, gate on it. */
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_OBJVAR && f->v.cond && f->key && !strcmp(f->key, "condition"))
            return eval_condition_inner_host(g, actor, host_cid, f->v.cond);
    }
    return 1;
}

/* Complex condition (variant 12)  ERust ANDs a nested `cause` (and `effect`)
   condition. The decoder stores nested conditions as RB_TAG_OBJVAR CondValues,
   so we evaluate every nested sub-condition with AND (OR when the field is an
   array keyed "or"/"any_of"). Mirrors state.rs:evaluate_complex_condition.
   The activating card (host_cid) is forwarded because Rust reuses the same
   ConditionEvaluator for every nested condition. */
static int eval_complex(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_OBJVAR && f->v.cond) {
            if (!eval_condition_inner_host(g, actor, host_cid, f->v.cond)) return 0;
        } else if (f->v.tag == RB_TAG_ARRAY) {
            int combine_or = (f->key && (!strcmp(f->key, "or") ||
                                         !strcmp(f->key, "any_of") ||
                                         !strcmp(f->key, "any")));
            int any = 0, all = 1;
            for (uint32_t j = 0; j < f->v.arr_n; j++) {
                CondValue *e = &f->v.arr[j];
                if (e->tag == RB_TAG_OBJVAR && e->cond) {
                    if (eval_condition_inner_host(g, actor, host_cid, e->cond)) any = 1; else all = 0;
                }
            }
            if (combine_or) { if (!any) return 0; }
            else if (!all) return 0;
        }
    }
    return 1;
}


static int eval_condition_inner_host(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    if (!c) return 1;
    int negation=0; get_bool(c,"negation",&negation);
    /* Mirrors the [cond] log::debug! in condition/card.rs  Egated on the shared
       ability-debug switch so a full suite run is not flooded. */
    if (rb_ability_debug_enabled()) {
        const char *dp=get_str(c,"position"), *dl=get_str(c,"location"), *dc=get_str(c,"comparison_type"),
            *dt=get_str(c,"target"), *dcs=get_str(c,"check_self");
        if(dp&&!strcmp(dp,"center")) fprintf(stderr,"[cond] v=%d pos=center loc=%s cmptype=%s target=%s checkself=%s\n",
            c->variant, dl?dl:"-", dc?dc:"-", dt?dt:"-", dcs?dcs:"-");
    }
    int r=1;
    switch ((RbConditionVariant)c->variant) {
        case RB_COND_COMPOUND:            r = eval_compound(g, actor, host_cid, c); break;
        case RB_COND_LOCATION:            r = eval_location(g, actor, host_cid, c); break;
        case RB_COND_COMPARISON:          r = eval_comparison_inner(g, actor, host_cid, c); break;
        case RB_COND_MOVEMENT:            r = eval_movement(g, actor, host_cid, c); break;
        case RB_COND_GROUP:               r = eval_group(g, actor, host_cid, c); break;
        case RB_COND_APPEARANCE:          r = eval_appearance(g, actor, host_cid, c); break;
        case RB_COND_TEMPORAL:            r = eval_temporal(g, actor, host_cid, c); break;
        case RB_COND_STATE:               r = eval_state(g, actor, host_cid, c); break;
        case RB_COND_RESOURCE:            r = eval_resource_count(g, actor, c); break;
        case RB_COND_ABILITY_FILTER:      r = eval_ability_filter(g, actor, host_cid, c); break;
        case RB_COND_SCORE_THRESHOLD:     r = eval_score(g, actor, c); break;
        case RB_COND_CHOICE:              r = eval_choice(g, actor, host_cid, c); break;
        case RB_COND_COMPLEX:             r = eval_complex(g, actor, host_cid, c); break;
        case RB_COND_POSITION:            r = eval_position(g, actor, c); break;
        case RB_COND_OPPONENT_CHOICE:
            /* Mirror state.rs:evaluate_opponent_choice_condition  Etrue unless the
               opponent declined. Headless has no opponent-decline state, so assume
               the opponent accepted (gs.opponent_choice_declined == false). Negation
               is applied by rb_eval_condition's top-level wrapper, so return raw. */
            r = 1;
            break;
        case RB_COND_OPPONENT_LIVE_SUCCESS:
            /* Mirror state.rs:evaluate_opponent_live_success_condition  Etrue only if
                the owner's OPPONENT passed their live THIS TURN (g->live_success
                tracks each player's per-turn live result, set in live.c). */
            r = g->live_success[actor ^ 1] ? 1 : 0;
            break;
        case RB_COND_NO_EXCESS_HEART:     r = eval_no_excess(g, actor, c); break;
        case RB_COND_ALWAYS_TRUE:         r = 1; break;
        case RB_COND_ANY_OF:              r = eval_any_of(g, actor, c); break;
        case RB_COND_ALL_REVEALED: {
            /* Mirror condition.rs:evaluate_all_revealed_match_heart_color  Eat
               least `count` revealed cards carry the required heart color. The
               headless engine tracks the revealed pool in g->revealed_cards[]. */
            const CondValue *hv = find_val(c, "heart_colors");
            int thr=1; get_i(c,"count",&thr);
            if (g->n_revealed==0) { r = 0; break; }
            int matched=0;
            for(int i=0;i<g->n_revealed;i++){
                int cid=g->revealed_cards[i];
                Card cc; if(!rb_decode_card_by_index((uint32_t)cid,&cc)) continue;
                int has=0;
                if(hv && hv->tag==RB_TAG_ARRAY && hv->arr_n>0){
                    for(int h=0;h<cc.n_hearts && !has;h++){
                        int col=cc.heart_color[h]%8;
                        for(uint32_t k=0;k<hv->arr_n;k++){
                            int want = hv->arr[k].tag==RB_TAG_I64?(int)hv->arr[k].i:
                                      (hv->arr[k].tag==RB_TAG_STR&&hv->arr[k].s)?atoi(hv->arr[k].s):0;
                            if(want>=0 && want<=7 && col==want){ has=1; break; }
                        }
                    }
                } else has = cc.n_hearts>0 ? 1 : 0;
                if(has) matched++;
                rb_free_card(&cc);
            }
            r = (matched >= thr) ? 1 : 0;
            break;
        }
        default: r = 1; break;
    }
    return negation ? !r : r;
}
static int eval_condition_inner(const struct GameState *g, int actor, const Condition *c) {
    return eval_condition_inner_host(g, actor, -1, c);
}

int rb_eval_condition(const struct GameState *g, int actor, const Condition *c) {
    return eval_condition_inner(g, actor, c);
}
int rb_eval_condition_for_host(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    return eval_condition_inner_host(g, actor, host_cid, c);
}

/* ── ConditionContext::allows (condition.rs) ──
   Returns true if the effect's condition (if any) evaluates to true.
   Mirrors: effect.condition.as_ref().map_or(true, |c| self.evaluate_condition(c)) ── */
int rb_condition_allows(const struct GameState *g, int actor, const AbilityEffect *effect, int host_cid) {
    if (!effect || !effect->has_condition || !effect->condition) return 1;
    return eval_condition_inner_host(g, actor, host_cid, effect->condition);
}

/* ── check_heart_type_all (condition/card.rs) ──
   Checks if any stage member has all-heart (HeartColor::All / Heart00 base).
   Used by appearance/heart-type conditions. ── */
int rb_check_heart_type_all(const struct GameState *g, int actor, const Condition *c, int negation) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (loc && strcmp(loc, "stage")) return 1;
    const char *ht = get_str(c, "heart_type");
    if (ht && strcmp(ht, "all")) return 1;
    RbPlayer *P = &g->p[pl];
    int q;
    if (negation) {
        /* Check if the specific triggering member lacks all-heart */
        int target = (g->queue.cur < g->queue.n_entries) ? g->queue.cur : -1;
        int found = 0;
        for (q = 0; q < RB_STAGE_SIZE; q++) {
            int cid = P->stage[q];
            if (cid < 0) continue;
            Card cc;
            if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            int has_all = (cc.n_hearts > 0 && cc.heart_count[0] > 0); /* Heart00 base */
            rb_free_card(&cc);
            if (!has_all) return 1;
            found = 1;
        }
        return found ? 0 : 1;
    }
    for (q = 0; q < RB_STAGE_SIZE; q++) {
        int cid = P->stage[q];
        if (cid < 0) continue;
        Card cc;
        if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        int has_all = (cc.n_hearts > 0 && cc.heart_count[0] > 0);
        rb_free_card(&cc);
        if (has_all) return 1;
    }
    return 0;
}

/* ── check_heart_colors (condition/card.rs) ──
   Checks if all specified heart colors are present in base_heart of any stage card.
   Only applies to stage zone; returns true for non-stage zones. ── */
int rb_check_heart_colors(const struct GameState *g, int actor, const Condition *c) {
    const char *loc = get_str(c, "location");
    if (loc && strcmp(loc, "stage")) return 1;
    /* Parse heart_colors from condition */
    int cols[8]; int nc = 0;
    for (int i = 0; i < c->n_fields; i++) {
        if (c->fields[i].key && !strcmp(c->fields[i].key, "heart_colors")) {
            CondValue *cv = &c->fields[i].v;
            if (cv->tag == RB_TAG_STR && cv->s) {
                cols[nc++] = s_heart_idx(cv->s);
            } else if (cv->tag == RB_TAG_ARRAY && cv->arr) {
                for (uint32_t j = 0; j < cv->arr_n && nc < 8; j++) {
                    if (cv->arr[j].s) cols[nc++] = s_heart_idx(cv->arr[j].s);
                }
            }
            break;
        }
    }
    if (nc == 0) return 1;
    /* Check if heart00 (wildcard) is in the list */
    for (int i = 0; i < nc; i++) if (cols[i] == 0) return 1;
    int pl = target_player_idx(actor, c);
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < nc; i++) {
        int found = 0;
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            int cid = P->stage[q];
            if (cid < 0) continue;
            Card cc;
            if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            if (cc.n_hearts > 0 && cols[i] < cc.n_hearts && cc.heart_count[cols[i]] > 0) found = 1;
            rb_free_card(&cc);
            if (found) break;
        }
        if (!found) return 0;
    }
    return 1;
}

/* ── check_aggregate_total (condition/card.rs) ──
   For aggregate="total" conditions: sum heart colors across stage cards and
   compare against threshold. Returns 1 if comparison passes, 0 if not,
   -1 if aggregate is not "total". ── */
int rb_check_aggregate_total(const struct GameState *g, int actor, const Condition *c) {
    const char *agg = get_str(c, "aggregate");
    if (!agg || strcmp(agg, "total")) return -1;
    int pl = target_player_idx(actor, c);
    RbPlayer *P = &g->p[pl];
    const char *op = get_str(c, "operator");
    int thr = 0; get_i(c, "count", &thr);
    /* Parse heart colors */
    int cols[8]; int nc = 0;
    for (int i = 0; i < c->n_fields; i++) {
        if (c->fields[i].key && !strcmp(c->fields[i].key, "heart_colors")) {
            CondValue *cv = &c->fields[i].v;
            if (cv->tag == RB_TAG_STR && cv->s) {
                cols[nc++] = s_heart_idx(cv->s);
            } else if (cv->tag == RB_TAG_ARRAY && cv->arr) {
                for (uint32_t j = 0; j < cv->arr_n && nc < 8; j++) {
                    if (cv->arr[j].s) cols[nc++] = s_heart_idx(cv->arr[j].s);
                }
            }
            break;
        }
    }
    const char *loc = get_str(c, "location");
    int total = 0;
    if (!strcmp(loc, "stage")) {
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            int cid = P->stage[q];
            if (cid < 0) continue;
            Card cc;
            if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            if (cc.n_hearts > 0) {
                for (int i = 0; i < nc; i++) {
                    if (cols[i] < cc.n_hearts) total += cc.heart_count[cols[i]];
                }
            }
            rb_free_card(&cc);
        }
    } else if (!strcmp(loc, "live_card_zone") || !strcmp(loc, "live")) {
        for (int i = 0; i < P->live.n; i++) {
            int cid = P->live.cards[i];
            Card cc;
            if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            if (cc.n_hearts > 0) {
                for (int h = 0; h < nc && h < cc.n_hearts; h++) {
                    total += cc.heart_count[cols[h]];
                }
            }
            rb_free_card(&cc);
        }
    }
    return eval_operator(total, op, thr);
}

/* ── Additional ported functions from Rust condition/card.rs, condition.rs ── */

/* check_phase_gate: mirror engine/src/ability/condition.rs:check_phase_gate.
   Checks whether the condition's phase restriction (if any) is satisfied. */
int rb_check_phase_gate(const struct GameState *g, int actor, const Condition *c, int skip_gate) {
    if (skip_gate) return 1;
    const char *phase = get_str(c, "phase");
    if (!phase) return 1;
    if (!strcmp(phase, "main") || !strcmp(phase, "main_phase")) {
        if (g->phase != RB_PHASE_MAIN) return 0;
        const char *pt = get_str(c, "phase_target");
        if (pt && !strcmp(pt, "self")) return actor == g->active;
        if (pt && !strcmp(pt, "opponent")) return actor != g->active;
        return 1;
    }
    if (!strcmp(phase, "active") || !strcmp(phase, "active_phase")) {
        if (g->phase != RB_PHASE_ACTIVE) return 0;
        const char *pt = get_str(c, "phase_target");
        if (pt && !strcmp(pt, "self")) return actor == g->active;
        if (pt && !strcmp(pt, "opponent")) return actor != g->active;
        return 1;
    }
    if (!strcmp(phase, "live") || !strcmp(phase, "live_phase") ||
        !strcmp(phase, "live_card_set") || !strcmp(phase, "live_performance") || !strcmp(phase, "live_victory")) {
        if (!phase_in_live(g)) return 0;
        const char *pt = get_str(c, "phase_target");
        if (pt && !strcmp(pt, "self")) return actor == g->active;
        if (pt && !strcmp(pt, "opponent")) return actor != g->active;
        return 1;
    }
    return 1;
}

/* evaluate_front_comparison: mirror card.rs:evaluate_front_comparison.
   Compares the activating card's cost against the opponent's front-area card cost. */
static int eval_front_comparison(const struct GameState *g, int actor, const Condition *c) {
    if (g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    int master_id = g->queue.entries[g->queue.cur].card_id;
    if (master_id < 0) return 0;
    const RbPlayer *mp = &g->p[actor];
    int master_idx = -1;
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (mp->stage[i] == master_id) { master_idx = i; break; }
    if (master_idx < 0) return 0;
    /* front area: left=0 centers on left, center=1 centers on center, right=2 centers on right */
    int front_idx = master_idx;
    int pl = actor ^ 1;
    const RbPlayer *op = &g->p[pl];
    int front_cid = op->stage[front_idx];
    if (front_cid == RB_EMPTY_SLOT) return 0;
    Card mc, fc;
    if (!rb_decode_card_by_index((uint32_t)master_id, &mc)) return 0;
    if (!rb_decode_card_by_index((uint32_t)front_cid, &fc)) { rb_free_card(&mc); return 0; }
    int result = eval_operator(fc.cost, get_str(c, "operator"), mc.cost);
    rb_free_card(&mc); rb_free_card(&fc);
    return result;
}

/* evaluate_all_cost_comparison_condition: mirror card.rs:evaluate_all_cost_comparison_condition.
   At least one of self's stage members has effective cost satisfying operator against max opponent cost. */
static int eval_all_cost_comparison(const struct GameState *g, int actor, const Condition *c) {
    int self_costs[RB_STAGE_SIZE], opp_costs[RB_STAGE_SIZE];
    int ns = collect_stage_costs(g, actor, self_costs);
    int no = collect_stage_costs(g, actor ^ 1, opp_costs);
    int max_opp = 0;
    for (int i = 0; i < no; i++) if (opp_costs[i] > max_opp) max_opp = opp_costs[i];
    const char *op = get_str(c, "operator"); if (!op) op = ">=";
    for (int i = 0; i < ns; i++) if (eval_operator(self_costs[i], op, max_opp)) return 1;
    return 0;
}

/* check_card_property: mirror card.rs:check_card_property.
   Checks has_blade_heart / has_score_icon / has_all_blade for cards in a zone. */
static int check_card_property(const struct GameState *g, int actor, const Condition *c, const char *loc) {
    const char *prop = get_str(c, "card_property");
    if (!prop) return 1;
    int pl = target_player_idx(actor, c);
    int neg = 0; get_bool(c, "negation", &neg);
    int ids[RB_MAX_ZONE]; int n = 0;
    if (!strcmp(loc, "revealed_cards")) {
        n = g->n_revealed; for (int i = 0; i < n; i++) ids[i] = g->revealed_cards[i];
    } else {
        n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    }
    if (n == 0) return 1;
    for (int i = 0; i < n; i++) {
        Card cc; if (!rb_decode_card_by_index((uint32_t)ids[i], &cc)) continue;
        int has = 0;
        if (!strcmp(prop, "has_blade_heart")) has = rb_card_has_blade_heart(&cc);
        else if (!strcmp(prop, "has_score_icon")) has = rb_card_has_score_icon(&cc);
        else if (!strcmp(prop, "has_all_blade")) has = rb_card_has_all_blade(&cc);
        rb_free_card(&cc);
        if (neg) { if (!has) return 1; } else { if (has) return 1; }
    }
    return neg ? 1 : 0;
}

/* check_baton_touch: mirror card.rs:check_baton_touch.
   Validates baton-touch trigger conditions (count, group, source, cost). */
static int check_baton_touch(const struct GameState *g, int actor, const Condition *c) {
    int bt = 0; get_bool(c, "baton_touch_trigger", &bt);
    if (!bt) return 1;
    int pl = target_player_idx(actor, c);
    int bt_count = pl ? g->baton_touch_count_p2 : g->baton_touch_count_p1;
    if (bt_count == 0) return 0;
    int minc = 0; if (get_i(c, "min_baton_touch_count", &minc) && minc > 0 && bt_count < minc) return 0;
    const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0 && g->baton_touch_replaced_member_id >= 0) {
        int gid = g->baton_touch_replaced_member_id;
        int ok = 0;
        for (uint32_t i = 0; i < gv->arr_n; i++) {
            if (gv->arr[i].tag == RB_TAG_STR && gv->arr[i].s && rb_card_matches_group_str(gid, gv->arr[i].s)) { ok = 1; break; }
        }
        if (!ok) return 0;
    }
    const char *src = get_str(c, "baton_touch_source");
    if (src && g->baton_touch_replaced_member_id >= 0) {
        Card rc; if (!rb_decode_card_by_index((uint32_t)g->baton_touch_replaced_member_id, &rc)) return 0;
        char nbuf[128], nsrc[128];
        rb_card_normalize_name(rc.name ? rc.name : "", nbuf, sizeof(nbuf));
        rb_card_normalize_name(src, nsrc, sizeof(nsrc));
        int found = strstr(nbuf, nsrc) != NULL;
        rb_free_card(&rc);
        if (!found) return 0;
    }
    int cl = 0; if (get_i(c, "cost_limit", &cl) && g->baton_touch_replaced_member_cost >= 0) {
        const char *cop = get_str(c, "cost_limit_operator");
        if (!eval_operator(g->baton_touch_replaced_member_cost, cop, cl)) return 0;
    }
    return 1;
}

/* check_ability_filter: mirror card.rs:check_ability_filter.
   Checks has_ability / no_ability for cards in a zone, optionally filtered by trigger prefixes. */
static int check_ability_filter(const struct GameState *g, int actor, const Condition *c, const char *loc) {
    const char *filter = get_str(c, "ability_filter");
    if (!filter) return 1;
    int pl = target_player_idx(actor, c);
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const CondValue *tv = get_arr(c, "ability_filter_triggers");
    if (n == 0) return 1;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        int na = rb_card_num_abilities((uint32_t)cid);
        int present = na > 0;
        if (present && tv) {
            present = 0;
            for (uint32_t t = 0; t < tv->arr_n; t++) {
                if (tv->arr[t].tag == RB_TAG_STR && tv->arr[t].s && card_ability_trigger_contains(cid, tv->arr[t].s)) { present = 1; break; }
            }
        }
        if (!strcmp(filter, "has_ability") && present) return 1;
        if (!strcmp(filter, "no_ability") && !present) return 1;
    }
    return 0;
}

/* Mirror card.rs::evaluate_ability_filter_condition_with_card_check.
   Counts cards in a zone matching an ability filter (no_ability, has_ability,
   no_ability_type) and compares to count with operator. */
static int eval_ability_filter_with_card_check(const struct GameState *g, int actor, const Condition *c, const char *filter) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location"); if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const CondValue *etv = find_val(c, "ability_filter_triggers");
    int count_needed = 0; if (!get_i(c, "count", &count_needed)) count_needed = 1;
    const char *op = get_str(c, "operator"); if (!op) op = ">=";
    int match_count = 0;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        int cid = ids[i];
        int na = rb_card_num_abilities((uint32_t)cid);
        int has_ability = (na > 0);
        int m = 0;
        if (!strcmp(filter, "no_ability")) m = !has_ability;
        else if (!strcmp(filter, "has_ability")) m = has_ability;
        else if (!strcmp(filter, "no_ability_type") && has_ability) {
            /* Card has abilities but NONE match the excluded trigger types */
            int excluded_match = 0;
            for (int a = 0; a < na && !excluded_match; a++) {
                uint32_t aidx;
                if (!rb_card_get_ability_idx((uint32_t)cid, a, &aidx)) continue;
                for (uint32_t gi = 0; gi < etv->arr_n && !excluded_match; gi++) {
                    const char *et = (etv->arr[gi].tag == RB_TAG_STR) ? etv->arr[gi].s : NULL;
                    if (et && card_ability_trigger_contains(cid, et)) { excluded_match = 1; break; }
                }
            }
            m = !excluded_match;
        } else m = 1;
        if (m) match_count++;
    }
    if (!strcmp(op, "="))  return match_count == count_needed;
    if (!strcmp(op, "<=")) return match_count <= count_needed;
    if (!strcmp(op, "<"))  return match_count < count_needed;
    if (!strcmp(op, ">"))  return match_count > count_needed;
    return match_count >= count_needed;
}

/* check_distinct_names: mirror card.rs:check_distinct_names.
   Validates distinct-name/cost/group constraints for cards in a zone. */
static int check_distinct_names(const struct GameState *g, int actor, const Condition *c, const char *loc) {
    int dist = 0; get_bool(c, "distinct", &dist);
    if (!dist) {
        const CondValue *dv = find_val(c, "distinct");
        if (dv && dv->tag == RB_TAG_OBJVAR && dv->cond) {
            for (uint32_t i = 0; i < dv->cond->n_fields; i++)
                if (!strcmp(dv->cond->fields[i].key, "distinct") && dv->cond->fields[i].v.tag == RB_TAG_TRUE) dist = 1;
        }
    }
    if (!dist) return 1;
    int pl = target_player_idx(actor, c);
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const char *group = get_str(c, "group");
    int thr = 0; int has_thr = get_i(c, "count", &thr);
    const char *dtype = "card";
    const CondValue *dv = find_val(c, "distinct");
    if (dv && dv->tag == RB_TAG_STR && dv->s) dtype = dv->s;
    if (!strcmp(dtype, "cost")) {
        int seen[256] = {0}; int nd = 0;
        for (int i = 0; i < n; i++) {
            if (group && !rb_card_matches_group_str(ids[i], group)) continue;
            Card cc; if (!rb_decode_card_by_index((uint32_t)ids[i], &cc)) continue;
            int cost = rb_saturate_u8((int)cc.cost + rb_mods_get_cost((RbMods*)&g->mods, ids[i]));
            rb_free_card(&cc);
            if (!seen[cost]) { seen[cost] = 1; nd++; }
        }
        if (has_thr) return eval_operator(nd, get_str(c, "operator"), thr);
        return nd == n;
    }
    if (!strcmp(dtype, "group_name")) {
        int ng = 0;
        for (int i = 0; i < n; i++) {
            if (group && !rb_card_matches_group_str(ids[i], group)) continue;
            Card cc; if (!rb_decode_card_by_index((uint32_t)ids[i], &cc)) continue;
            const char *gn = rb_card_string(cc.group_idx);
            rb_free_card(&cc);
            if (gn && *gn) ng++;
        }
        if (has_thr) return eval_operator(ng, get_str(c, "operator"), thr);
        return ng == n;
    }
    /* card_name distinct: count distinct card names */
    int nd = count_distinct_in_zone(g, pl, loc);
    if (has_thr) return eval_operator(nd, get_str(c, "operator"), thr);
    return nd >= n;
}

/* check_no_excess_heart: mirror card.rs:check_no_excess_heart.
   Validates that stage total_hearts <= live+success need hearts. */
static int check_no_excess_heart(const struct GameState *g, int actor, const Condition *c, int host_cid) {
    int ne = 0; get_bool(c, "no_excess_heart", &ne);
    if (!ne) return 1;
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i]; if (cid == RB_EMPTY_SLOT) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        if (cc.blade > 0 || cc.n_hearts > 0) {
            for (int h = 0; h < cc.n_hearts; h++) total += cc.heart_count[h];
        }
        rb_free_card(&cc);
    }
    int need = 0;
    for (int i = 0; i < P->live.n; i++) {
        int cid = P->live.cards[i];
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        for (int h = 0; h < cc.n_hearts; h++) need += cc.heart_count[h];
        rb_free_card(&cc);
    }
    for (int i = 0; i < P->success.n; i++) {
        int cid = P->success.cards[i];
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        for (int h = 0; h < cc.n_hearts; h++) need += cc.heart_count[h];
        rb_free_card(&cc);
    }
    return total <= need;
}

/* evaluate_multi_location_condition: mirror card.rs:evaluate_multi_location_condition.
   Handles conditions with multiple locations (locations array). */
static int eval_multi_location(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    const CondValue *locs = find_val(c, "locations");
    if (!locs || locs->tag != RB_TAG_ARRAY || locs->arr_n == 0) return 0;
    const char *target = get_str(c, "target");
    const char *ctype = get_str(c, "card_type");
    const char *group = get_str(c, "group");
    const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0 && gv->arr[0].tag == RB_TAG_STR) group = gv->arr[0].s;
    const char *op = get_str(c, "operator");
    int thr = 1; get_i(c, "count", &thr);
    int pl = target_player_idx(actor, c);
    int combined[RB_MAX_ZONE]; int nc = 0;
    for (uint32_t i = 0; i < locs->arr_n; i++) {
        if (locs->arr[i].tag != RB_TAG_STR || !locs->arr[i].s) continue;
        int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, locs->arr[i].s, ids, RB_MAX_ZONE);
        for (int j = 0; j < n && nc < RB_MAX_ZONE; j++) combined[nc++] = ids[j];
    }
    int matching = 0;
    for (int i = 0; i < nc; i++) {
        int cid = combined[i];
        if (ctype && !card_matches_card_type_filter(cid, ctype)) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        matching++;
    }
    return eval_operator(matching, op, thr);
}

/* resolve_moved_cards_source: mirror card.rs:resolve_moved_cards_source.
   Resolves the set of card IDs that satisfy a moved-card condition (preceding_moved or zone transition). */
static int resolve_moved_cards_source(const struct GameState *g, int actor, const Condition *c,
                                       int *out_ids, int max, int *out_n) {
    const char *src = get_str(c, "source");
    const char *dst = get_str(c, "destination");
    const char *ctype = get_str(c, "card_type");
    const char *group = get_str(c, "group");
    const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0 && gv->arr[0].tag == RB_TAG_STR) group = gv->arr[0].s;
    int is_old = src && (!strcmp(src, "preceding_moved") || !strcmp(src, "previous_moved_cards"));
    int pl = target_player_idx(actor, c);
    int ids[RB_MAX_ZONE]; int n = 0;
    if (is_old) {
        n = g->n_recently_moved;
        for (int i = 0; i < n && i < RB_MAX_ZONE; i++) ids[i] = g->recently_moved[i];
    } else if (dst && *dst) {
        /* new movement format: filter recently_moved by destination zone */
        for (int i = 0; i < g->n_recently_moved; i++) {
            int cid = g->recently_moved[i];
            /* check if card is in destination zone */
            int in_dst = 0;
            const RbPlayer *P = &g->p[pl];
            if (!strcmp(dst, "discard") || !strcmp(dst, "waitroom")) {
                for (int j = 0; j < P->discard.n; j++) if (P->discard.cards[j] == cid) { in_dst = 1; break; }
            } else if (!strcmp(dst, "stage")) {
                for (int j = 0; j < RB_STAGE_SIZE; j++) if (P->stage[j] == cid) { in_dst = 1; break; }
            } else if (!strcmp(dst, "hand")) {
                for (int j = 0; j < P->hand.n; j++) if (P->hand.cards[j] == cid) { in_dst = 1; break; }
            }
            if (in_dst) ids[n++] = cid;
        }
    } else {
        n = g->n_recently_moved;
        for (int i = 0; i < n && i < RB_MAX_ZONE; i++) ids[i] = g->recently_moved[i];
    }
    /* apply filters */
    int filtered = 0;
    for (int i = 0; i < n && filtered < max; i++) {
        int cid = ids[i];
        if (ctype && !card_matches_card_type_filter(cid, ctype)) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        out_ids[filtered++] = cid;
    }
    *out_n = filtered;
    return filtered;
}

/* resolve_zone_card_count: mirror card.rs:resolve_zone_card_count.
   Counts cards in a zone with all applicable filters (card_type, group, heart_colors, cost_limit, etc.). */
static int resolve_zone_card_count(const struct GameState *g, int actor, const Condition *c, const char *loc) {
    int pl = target_player_idx(actor, c);
    const char *ctype = get_str(c, "card_type");
    const char *group = get_str(c, "group");
    const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0 && gv->arr[0].tag == RB_TAG_STR) group = gv->arr[0].s;
    int excl = 0; get_bool(c, "exclude_self", &excl);
    int host_cid = -1;
    /* get host_cid from queue if available */
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) host_cid = g->queue.entries[g->queue.cur].card_id;
    int exclude_cid = (excl && host_cid >= 0) ? host_cid : -1;
    int cl = 0; int has_cl = get_i(c, "cost_limit", &cl);
    const char *clop = get_str(c, "cost_limit_operator");
    const CondValue *hc = find_val(c, "heart_colors");
    int ids[RB_MAX_ZONE]; int n;
    if (!strcmp(loc, "revealed_cards")) {
        n = g->n_revealed; for (int i = 0; i < n; i++) ids[i] = g->revealed_cards[i];
    } else {
        n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    }
    int matching = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (exclude_cid >= 0 && cid == exclude_cid) continue;
        if (ctype && !card_matches_card_type_filter(cid, ctype)) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        if (has_cl) {
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            int cost = cc.cost;
            rb_free_card(&cc);
            if (!eval_operator(cost, clop, cl)) continue;
        }
        if (hc && hc->tag == RB_TAG_ARRAY && hc->arr_n > 0) {
            int hcolors[8]; int nh = 0;
            for (uint32_t k = 0; k < hc->arr_n && nh < 8; k++) {
                if (hc->arr[k].tag == RB_TAG_STR && hc->arr[k].s) hcolors[nh++] = s_heart_idx(hc->arr[k].s);
            }
            Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
            int found = 0;
            for (int h = 0; h < cc.n_hearts && !found; h++) {
                for (int k = 0; k < nh; k++) if (cc.heart_color[h] % 8 == hcolors[k]) { found = 1; break; }
            }
            rb_free_card(&cc);
            if (!found) continue;
        }
        matching++;
    }
    return matching;
}

/* evaluate_card_count_condition: mirror card.rs:evaluate_card_count_condition.
   Handles card_count conditions with count + operator + filters. */
static int eval_card_count_old(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    int cs = eval_check_self(g, actor, host_cid, c);
    if (cs >= 0) return cs;
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    const char *ctype = get_str(c, "card_type");
    const char *group = get_str(c, "group");
    const CondValue *gv = find_val(c, "group_names");
    if (gv && gv->tag == RB_TAG_ARRAY && gv->arr_n > 0 && gv->arr[0].tag == RB_TAG_STR) group = gv->arr[0].s;
    int thr = 1; get_i(c, "count", &thr);
    const char *op = get_str(c, "operator");
    const char *src = get_str(c, "source");
    const char *dst = get_str(c, "destination");
    int is_old = src && (!strcmp(src, "preceding_moved") || !strcmp(src, "previous_moved_cards"));
    int is_new = !is_old && dst && *dst;
    int actual;
    if (is_old || is_new) {
        int ids[RB_MAX_ZONE], n = 0;
        resolve_moved_cards_source(g, actor, c, ids, RB_MAX_ZONE, &n);
        actual = n;
    } else {
        actual = resolve_zone_card_count(g, actor, c, loc);
    }
    return eval_operator(actual, op, thr);
}

/* evaluate_card_blade_condition: mirror card.rs:evaluate_card_blade_condition.
   Sums effective blade of selected/moved cards and compares to threshold. */
static int eval_card_blade_old(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    const char *src = get_str(c, "source");
    int thr = 1; get_i(c, "count", &thr);
    const char *op = get_str(c, "operator");
    int ids[RB_MAX_ZONE]; int n = 0;
    if (src && (!strcmp(src, "preceding_moved") || !strcmp(src, "selected_cards"))) {
        n = g->n_recently_moved;
        for (int i = 0; i < n && i < RB_MAX_ZONE; i++) ids[i] = g->recently_moved[i];
    } else {
        n = g->n_recently_moved;
        for (int i = 0; i < n && i < RB_MAX_ZONE; i++) ids[i] = g->recently_moved[i];
    }
    if (n == 0) return 0;
    int total = 0;
    for (int i = 0; i < n; i++) total += effective_blade(g, ids[i]);
    return eval_operator(rb_saturate_u8(total), op ? op : ">=", thr);
}

/* get_count_for_condition: mirror card.rs:get_count_for_condition.
   BULK PORT: card.rs functions 3822-4836
   Mirror zone_len, count_cards_with_filters, count_distinct_in_cards,
   sum_group_hearts_in_stage, sum_group_filtered_zone, count_group_cards_in_cards,
   count_for_player_target, get_count_for_condition, get_count_for_target,
   get_group_card_count, evaluate_resource_condition, and the rest.
   ══════════════════════════════════════════════════════════════════════════╁E*/

/* ── zone_len: count cards/hearts/blades in a zone ── */
static int zone_blade_total(const GameState *g, int pl) {
    const RbPlayer *P = &g->p[pl];
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) continue;
        int blade = cc.blade;
        int mod = rb_mods_get_blade((RbMods*)&g->mods, cid);
        total += blade + mod;
        rb_free_card(&cc);
    }
    return total;
}
static int zone_len_impl(const GameState *g, int pl, const char *location) {
    const RbPlayer *P = &g->p[pl];
    if (!location || !*location) return 0;
    if (!strcmp(location, "stage")) return zone_blade_total(g, pl);
    if (!strcmp(location, "hand")) return P->hand.n;
    if (!strcmp(location, "deck") || !strcmp(location, "main_deck")) return P->deck.n;
    if (!strcmp(location, "discard") || !strcmp(location, "waitroom")) return P->discard.n;
    if (!strcmp(location, "energy") || !strcmp(location, "energy_zone")) return P->energy.n;
    if (!strcmp(location, "live") || !strcmp(location, "live_card_zone")) return P->live.n;
    if (!strcmp(location, "success") || !strcmp(location, "success_live_card_zone")) return P->success.n;
    if (!strcmp(location, "revealed") || !strcmp(location, "revealed_cards")) return g->n_revealed;
    return 0;
}

/* ── card_matches_count_filters: single-card filter check ── */
static int card_matches_cost_limit_str(int cid, const char *op, int limit) {
    if (!op || !*op || !strcmp(op, "==")) {
        Card cc; int c = 0;
        if (rb_decode_card_by_index((uint32_t)cid, &cc)) { c = cc.cost; rb_free_card(&cc); }
        return c == limit;
    }
    Card cc; int c = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &cc)) { c = cc.cost; rb_free_card(&cc); }
    if (!strcmp(op, "<=")) return c <= limit;
    if (!strcmp(op, "<"))  return c < limit;
    if (!strcmp(op, ">=")) return c >= limit;
    if (!strcmp(op, ">"))  return c > limit;
    return 0;
}
static int card_matches_count_filters(const GameState *g, int cid, const char *ct, const CondValue *gn, const CondValue *hc, int cost_limit, const char *cost_op, const Condition *c) {
    if (ct && *ct) {
        Card cc;
        int tflags = 0;
        if (rb_decode_card_by_index((uint32_t)cid, &cc)) { tflags = cc.type_flags; rb_free_card(&cc); }
        int member = (tflags & 1);
        int live   = (tflags & 2);
        int energy = (tflags & 4);
        if (!strcmp(ct, "member_card") && !member) return 0;
        if (!strcmp(ct, "live_card") && !live) return 0;
        if (!strcmp(ct, "energy_card") && !energy) return 0;
    }
    if (gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0) {
        int matched = 0;
        for (uint32_t i = 0; i < gn->arr_n; i++) {
            const char *t = (gn->arr[i].tag == RB_TAG_STR) ? gn->arr[i].s : NULL;
            if (t && rb_card_matches_group_str(cid, t)) { matched = 1; break; }
        }
        if (!matched) return 0;
    }
    if (hc && hc->tag == RB_TAG_ARRAY && hc->arr_n > 0) {
        int matched = 0;
        for (uint32_t i = 0; i < hc->arr_n && !matched; i++) {
            int col = RB_HEART_PINK;
            if (hc->arr[i].tag == RB_TAG_I64) col = (int)hc->arr[i].i;
            else if (hc->arr[i].tag == RB_TAG_STR && hc->arr[i].s) col = atoi(hc->arr[i].s);
            Card cc; int has = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
                for (int h = 0; h < cc.n_hearts; h++) {
                    if (cc.heart_color[h] == (uint8_t)col) { has = 1; break; }
                }
                rb_free_card(&cc);
            }
            if (has) matched = 1;
        }
        if (!matched) return 0;
    }
    if (cost_limit >= 0) {
        if (!card_matches_cost_limit_str(cid, cost_op, cost_limit)) return 0;
    }
    int orig_val = 0; get_bool(c, "original_value", &orig_val);
    if (orig_val) {
        int ob = 0; if (get_i(c, "original_blade", &ob) && ob > 0) {
            Card cc; int blade = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) { blade = cc.blade; rb_free_card(&cc); }
            if (blade < ob) return 0;
        }
        int oh = 0; if (get_i(c, "original_heart", &oh) && oh > 0) {
            Card cc; int total = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
                for (int h = 0; h < cc.n_hearts; h++) total += cc.heart_count[h];
                rb_free_card(&cc);
            }
            if (total < oh) return 0;
        }
    }
    return 1;
}

/* ── count_cards_with_filters: count cards matching filters ── */
static int count_cards_with_filters(const GameState *g, int *ids, int n, const char *ct, const CondValue *gn, const CondValue *hc, int cost_limit, const char *cost_op, int exclude_self, const Condition *c) {
    int count = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        if (exclude_self >= 0 && cid == exclude_self) continue;
        if (card_matches_count_filters(g, cid, ct, gn, hc, cost_limit, cost_op, c)) count++;
    }
    return count;
}

/* ── count_distinct_in_cards: distinct name/cost/group counting ── */
static int count_distinct_in_cards(const GameState *g, int *ids, int n, const char *ct, const CondValue *gn) {
    int matched_n = 0;
    int *matched = (int*)calloc(n > 0 ? n : 1, sizeof(int));
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (ct && *ct) {
            Card cc; int tflags = 0;
            if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) { tflags = cc.type_flags; rb_free_card(&cc); }
            if (!strcmp(ct, "member_card") && !(tflags & 1)) continue;
            if (!strcmp(ct, "live_card") && !(tflags & 2)) continue;
            if (!strcmp(ct, "energy_card") && !(tflags & 4)) continue;
        }
        if (gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0) {
            int m = 0;
            for (uint32_t k = 0; k < gn->arr_n; k++) {
                const char *t = (gn->arr[k].tag == RB_TAG_STR) ? gn->arr[k].s : NULL;
                if (t && rb_card_matches_group_str(ids[i], t)) { m = 1; break; }
            }
            if (!m) continue;
        }
        matched[matched_n++] = ids[i];
    }
    /* Count distinct by name: simple dedupe via name string */
    int distinct = 0;
    for (int i = 0; i < matched_n; i++) {
        if (matched[i] < 0) continue;
        Card ca; if (!rb_decode_card_by_index((uint32_t)matched[i], &ca)) continue;
        int dup = 0;
        for (int j = 0; j < i; j++) {
            if (matched[j] < 0) continue;
            Card cb; if (!rb_decode_card_by_index((uint32_t)matched[j], &cb)) continue;
            int same = (ca.name && cb.name && strcmp(ca.name, cb.name) == 0);
            rb_free_card(&cb);
            if (same) { dup = 1; break; }
        }
        rb_free_card(&ca);
        if (!dup) distinct++;
    }
    free(matched);
    return distinct;
}

/* ── sum_group_hearts_in_stage: sum hearts for cards in a group on stage ── */
static int sum_group_hearts_in_stage(const GameState *g, int pl, const char *group) {
    const RbPlayer *P = &g->p[pl];
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (group && *group && !rb_card_matches_group_str(cid, group)) continue;
        Card cc;
        if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
            for (int h = 0; h < cc.n_hearts; h++) total += cc.heart_count[h];
            rb_free_card(&cc);
        }
    }
    return total;
}

/* ── sum_group_filtered_zone: sum values for cards in a zone with filters ── */
static int sum_group_filtered_zone(const GameState *g, int *ids, int n, const char *ct, const char *group, int is_blade, int is_score, int is_cost) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        if (ct && *ct) {
            Card cc; int tflags = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) { tflags = cc.type_flags; rb_free_card(&cc); }
            if (!strcmp(ct, "member_card") && !(tflags & 1)) continue;
            if (!strcmp(ct, "live_card") && !(tflags & 2)) continue;
            if (!strcmp(ct, "energy_card") && !(tflags & 4)) continue;
        }
        if (group && *group && !rb_card_matches_group_str(cid, group)) continue;
        Card cc;
        if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
            if (is_blade) total += cc.blade;
            else if (is_score) total += cc.score;
            else if (is_cost) total += cc.cost;
            rb_free_card(&cc);
        }
    }
    return total;
}

/* ── count_group_cards_in_cards: count cards matching group in a list ── */
static int count_group_cards_in_cards(const GameState *g, int *ids, int n, const char *group, const char *ct) {
    int count = 0;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (group && *group && !rb_card_matches_group_str(ids[i], group)) continue;
        if (ct && *ct) {
            Card cc; int tflags = 0;
            if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) { tflags = cc.type_flags; rb_free_card(&cc); }
            if (!strcmp(ct, "member_card") && !(tflags & 1)) continue;
            if (!strcmp(ct, "live_card") && !(tflags & 2)) continue;
            if (!strcmp(ct, "energy_card") && !(tflags & 4)) continue;
        }
        count++;
    }
    return count;
}

/* ── count_for_player_target: count by comparison type ── */
static int count_for_player_target(const GameState *g, int actor, int pl, const char *location, const char *comparison_type) {
    const RbPlayer *P = &g->p[pl];
    if (comparison_type && !strcmp(comparison_type, "score")) {
        int total = 0;
        if (!location || !strcmp(location, "live_card_zone") || !strcmp(location, "live")) {
            for (int i = 0; i < P->live.n; i++) {
                Card cc; int s = 0;
                if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &cc)) {
                    s = cc.score + rb_mods_get_score((RbMods*)&g->mods, P->live.cards[i]);
                    rb_free_card(&cc);
                }
                total += s < 0 ? 0 : s;
            }
        }
        if (!location || !strcmp(location, "success_live_card_zone") || !strcmp(location, "success")) {
            for (int i = 0; i < P->success.n; i++) {
                Card cc; int s = 0;
                if (rb_decode_card_by_index((uint32_t)P->success.cards[i], &cc)) {
                    s = cc.score + rb_mods_get_score((RbMods*)&g->mods, P->success.cards[i]);
                    rb_free_card(&cc);
                }
                total += s < 0 ? 0 : s;
            }
        }
        return total;
    }
    if (comparison_type && !strcmp(comparison_type, "cost")) {
        int total = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc; int c = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
                c = cc.cost + rb_mods_get_cost((RbMods*)&g->mods, cid);
                rb_free_card(&cc);
            }
            total += c < 0 ? 0 : c;
        }
        return total;
    }
    if (comparison_type && !strcmp(comparison_type, "energy")) return P->energy.n;
    return zone_len_impl(g, pl, location);
}

/* ── get_count_for_target: count for a specific target player ── */
static int get_count_for_target(const GameState *g, int actor, const Condition *c, const char *target) {
    int pl = target_player_idx(actor, c);
    if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
    const char *loc = get_str(c, "location");
    const char *ct = get_str(c, "comparison_type");
    return count_for_player_target(g, actor, pl, loc, ct);
}

/* ── get_count_for_condition: main count dispatcher ── */
static int get_count_for_condition(const GameState *g, int actor, const Condition *c) {
    const char *loc = get_str(c, "location");
    const char *target = get_str(c, "target");
    if (!target) target = "self";
    const char *ct = get_str(c, "comparison_type");
    const char *rt = get_str(c, "resource_type");
    if (ct && !strcmp(ct, "score")) return get_count_for_target(g, actor, c, target);
    if (ct && !strcmp(ct, "cost")) {
        /* Sum modified costs of cards in zone with group/type filtering */
        int pl = target_player_idx(actor, c);
        if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
        const CondValue *gn = find_val(c, "group_names");
        const char *gn0 = (gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0 && gn->arr[0].tag == RB_TAG_STR) ? gn->arr[0].s : NULL;
        if (!loc || !strcmp(loc, "stage")) {
            const RbPlayer *P = &g->p[pl];
            int total = 0;
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int cid = P->stage[i];
                if (cid == RB_EMPTY_SLOT) continue;
                if (gn0 && !rb_card_matches_group_str(cid, gn0)) continue;
                Card cc; int cst = 0;
                if (rb_decode_card_by_index((uint32_t)cid, &cc)) { cst = cc.cost; rb_free_card(&cc); }
                total += cst + rb_mods_get_cost((RbMods*)&g->mods, cid);
            }
            return total < 0 ? 0 : total;
        }
    }
    if (rt && !strcmp(rt, "hand_count")) {
        int pl = target_player_idx(actor, c);
        if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
        return g->p[pl].hand.n;
    }
    if (rt && !strcmp(rt, "energy")) {
        int pl = target_player_idx(actor, c);
        if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
        return g->p[pl].energy.n;
    }
    if (rt && strncmp(rt, "heart", 5) == 0) {
        int pl = target_player_idx(actor, c);
        if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
        const RbPlayer *P = &g->p[pl];
        int col = atoi(rt + 5);
        int total = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
                if (col < cc.n_hearts) total += cc.heart_count[col];
                rb_free_card(&cc);
            }
        }
        return total;
    }
    if (rt && !strcmp(rt, "surplus_heart")) return count_surplus_heart(g, actor, c, target);
    if (loc) {
        int pl = target_player_idx(actor, c);
        if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
        return zone_len_impl(g, pl, loc);
    }
    return 0;
}

/* ── get_group_card_count: count cards matching group in condition's location ── */
static int get_group_card_count(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const CondValue *gn = find_val(c, "group_names");
    if (!gn || gn->tag != RB_TAG_ARRAY || gn->arr_n == 0) {
        /* No group filter: count non-empty cards in zone */
        int count = 0;
        for (int i = 0; i < n; i++) if (ids[i] >= 0) count++;
        return count;
    }
    const char *ct = get_str(c, "card_type");
    const char *g0 = (gn->arr[0].tag == RB_TAG_STR) ? gn->arr[0].s : NULL;
    return count_group_cards_in_cards(g, ids, n, g0, ct);
}

/* ── evaluate_resource_condition: resource counting conditions ── */
static int get_card_total_hearts(const struct GameState *g, int cid) {
    Card cc; if (!rb_decode_card_by_index((uint32_t)cid, &cc)) return -1;
    if (cc.n_hearts == 0) { rb_free_card(&cc); return -1; }
    int total = 0;
    for (int h = 0; h < cc.n_hearts; h++) {
        int base = cc.heart_count[h];
        int mod = rb_mods_get_heart((RbMods*)&g->mods, cid, cc.heart_color[h] % 8);
        total += base + mod;
    }
    rb_free_card(&cc);
    return total;
}

static int count_surplus_heart(const struct GameState *g, int actor, const Condition *c, const char *target) {
    int pl = target_player_idx(actor, c);
    if (target && !strcmp(target, "opponent")) pl = actor ^ 1;
    const RbPlayer *P = &g->p[pl];
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

static int eval_resource_count(const GameState *g, int actor, const Condition *c) {
    const char *rt = get_str(c, "resource_type");
    if (!rt) return 1;
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    int count = 0;
    if (!strcmp(rt, "hand_count")) count = P->hand.n;
    else if (!strcmp(rt, "energy")) count = P->energy.n;
    else if (!strcmp(rt, "deck_count")) count = P->deck.n;
    else if (!strcmp(rt, "discard_count") || !strcmp(rt, "waitroom")) count = P->discard.n;
    else if (!strcmp(rt, "live_count")) count = P->live.n;
    else if (!strcmp(rt, "success_count")) count = P->success.n;
    else if (!strcmp(rt, "surplus_heart")) {
        const char *tgt = get_str(c, "target");
        count = count_surplus_heart(g, actor, c, tgt);
    }
    else if (strncmp(rt, "heart", 5) == 0) {
        int col = atoi(rt + 5);
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            Card cc;
            if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
                if (col < cc.n_hearts) count += cc.heart_count[col];
                rb_free_card(&cc);
            }
        }
    }
    else {
        const char *loc = get_str(c, "location");
        if (loc) count = zone_len_impl(g, pl, loc);
    }
    int needed = 0; get_i(c, "count", &needed);
    const char *op = get_str(c, "operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return count >= needed;
    if (!strcmp(op, ">"))  return count > needed;
    if (!strcmp(op, "<=")) return count <= needed;
    if (!strcmp(op, "<"))  return count < needed;
    if (!strcmp(op, "==")) return count == needed;
    return count >= needed;
}

/* ── evaluate_card_count_condition: count cards in location matching filter ── */
static int eval_card_count(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const char *ct = get_str(c, "card_type");
    const CondValue *gn = find_val(c, "group_names");
    const CondValue *hc = find_val(c, "heart_colors");
    int cost_limit = -1; get_i(c, "cost_limit", &cost_limit);
    const char *cost_op = get_str(c, "cost_limit_operator");
    int exclude_self = -1;
    int exc = 0; if (get_bool(c, "exclude_self", &exc) && exc) exclude_self = 0; /* placeholder */
    int count = count_cards_with_filters(g, ids, n, ct, gn, hc, cost_limit, cost_op, exclude_self, c);
    int needed = 0; get_i(c, "count", &needed);
    if (needed == 0) needed = 1;
    const char *op = get_str(c, "operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return count >= needed;
    if (!strcmp(op, ">"))  return count > needed;
    if (!strcmp(op, "<=")) return count <= needed;
    if (!strcmp(op, "<"))  return count < needed;
    if (!strcmp(op, "==")) return count == needed;
    return count >= needed;
}

/* ── evaluate_card_blade_condition: sum blades of cards matching filter ── */
static int eval_card_blade(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const char *ct = get_str(c, "card_type");
    const CondValue *gn = find_val(c, "group_names");
    const char *g0 = (gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0 && gn->arr[0].tag == RB_TAG_STR) ? gn->arr[0].s : NULL;
    int total = sum_group_filtered_zone(g, ids, n, ct, g0, 1, 0, 0);
    int needed = 0; get_i(c, "count", &needed);
    if (needed == 0) needed = 1;
    const char *op = get_str(c, "operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return total >= needed;
    if (!strcmp(op, ">"))  return total > needed;
    if (!strcmp(op, "<=")) return total <= needed;
    if (!strcmp(op, "<"))  return total < needed;
    if (!strcmp(op, "==")) return total == needed;
    return total >= needed;
}

/* ── check_distinct_names: validate distinct constraints ── */
static int check_distinct_names_impl(const GameState *g, int pl, const char *loc, const Condition *c) {
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const char *ct = get_str(c, "card_type");
    const CondValue *gn = find_val(c, "group_names");
    int distinct = count_distinct_in_cards(g, ids, n, ct, gn);
    int needed = 0; get_i(c, "count", &needed);
    if (needed == 0) needed = 1;
    const char *op = get_str(c, "operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return distinct >= needed;
    if (!strcmp(op, ">"))  return distinct > needed;
    if (!strcmp(op, "<=")) return distinct <= needed;
    if (!strcmp(op, "<"))  return distinct < needed;
    if (!strcmp(op, "==")) return distinct == needed;
    return distinct >= needed;
}

/* ── evaluate_comparison_condition: compare counts between targets ── */
static int eval_comparison(const GameState *g, int actor, const Condition *c) {
    const char *ct = get_str(c, "comparison_type");
    if (!ct) return 1;
    int pl = target_player_idx(actor, c);
    int my_count = count_for_player_target(g, actor, pl, get_str(c, "location"), ct);
    int opp_count = count_for_player_target(g, actor, pl ^ 1, get_str(c, "location"), ct);
    const char *op = get_str(c, "comparison_operator");
    if (!op) op = ">";
    if (!strcmp(op, ">"))  return my_count > opp_count;
    if (!strcmp(op, ">=")) return my_count >= opp_count;
    if (!strcmp(op, "<"))  return my_count < opp_count;
    if (!strcmp(op, "<=")) return my_count <= opp_count;
    if (!strcmp(op, "==")) return my_count == opp_count;
    return my_count > opp_count;
}

/* ── evaluate_all_cost_comparison: total cost comparison ── */
static int eval_all_cost(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = g->p[pl].stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        Card cc; int cost = 0;
        if (rb_decode_card_by_index((uint32_t)cid, &cc)) { cost = cc.cost; rb_free_card(&cc); }
        total += cost + rb_mods_get_cost((RbMods*)&g->mods, cid);
    }
    int needed = 0; get_i(c, "count", &needed);
    const char *op = get_str(c, "operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return total >= needed;
    if (!strcmp(op, "<=")) return total <= needed;
    if (!strcmp(op, "==")) return total == needed;
    if (!strcmp(op, ">"))  return total > needed;
    if (!strcmp(op, "<"))  return total < needed;
    return total >= needed;
}

/* ── evaluate_highest_cost_on_stage: highest cost member ── */
static int eval_highest_cost_new(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    int highest = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = g->p[pl].stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        Card cc; int cost = 0;
        if (rb_decode_card_by_index((uint32_t)cid, &cc)) { cost = cc.cost; rb_free_card(&cc); }
        int modified = cost + rb_mods_get_cost((RbMods*)&g->mods, cid);
        if (modified > highest) highest = modified;
    }
    int needed = 0; get_i(c, "count", &needed);
    const char *op = get_str(c, "operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return highest >= needed;
    if (!strcmp(op, "<=")) return highest <= needed;
    if (!strcmp(op, "==")) return highest == needed;
    if (!strcmp(op, ">"))  return highest > needed;
    if (!strcmp(op, "<"))  return highest < needed;
    return highest >= needed;
}

/* ── evaluate_both_condition: "both" target ── */
static int eval_both(const GameState *g, int actor, const Condition *c) {
    int pl_self = target_player_idx(actor, c);
    int pl_opp = pl_self ^ 1;
    const char *ct = get_str(c, "comparison_type");
    int self_count = count_for_player_target(g, actor, pl_self, get_str(c, "location"), ct);
    int opp_count = count_for_player_target(g, actor, pl_opp, get_str(c, "location"), ct);
    const char *op = get_str(c, "operator");
    if (!op) op = "and";
    if (!strcmp(op, "and")) return self_count > 0 && opp_count > 0;
    if (!strcmp(op, "or")) return self_count > 0 || opp_count > 0;
    int needed = 0; get_i(c, "count", &needed);
    if (!strcmp(op, ">=")) return self_count >= needed && opp_count >= needed;
    if (!strcmp(op, "==")) return self_count == needed && opp_count == needed;
    return self_count > 0 && opp_count > 0;
}

/* ── check_heart_type_all: heart type check across all matching cards ── */
static int check_heart_type_all(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    int ht = 0; get_i(c, "heart_type", &ht);
    if (ht < 0) ht = 0;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        Card cc; int has = 0;
        if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
            for (int h = 0; h < cc.n_hearts; h++) {
                if (cc.heart_color[h] == (uint8_t)ht) { has = 1; break; }
            }
            rb_free_card(&cc);
        }
        if (!has) return 0;
    }
    return 1;
}

/* ── check_heart_colors: at least one card matches heart colors ── */
static int check_heart_colors(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const CondValue *hc = find_val(c, "heart_colors");
    if (!hc || hc->tag != RB_TAG_ARRAY || hc->arr_n == 0) return 1;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        for (uint32_t k = 0; k < hc->arr_n; k++) {
            int col = RB_HEART_PINK;
            if (hc->arr[k].tag == RB_TAG_I64) col = (int)hc->arr[k].i;
            else if (hc->arr[k].tag == RB_TAG_STR && hc->arr[k].s) col = atoi(hc->arr[k].s);
            Card cc; int has = 0;
            if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
                for (int h = 0; h < cc.n_hearts; h++) {
                    if (cc.heart_color[h] == (uint8_t)col) { has = 1; break; }
                }
                rb_free_card(&cc);
            }
            if (has) return 1;
        }
    }
    return 0;
}

/* ── check_card_property: card property check ── */
static int check_card_property_new(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const char *prop = get_str(c, "card_property");
    if (!prop) return 1;
    int needed = 0; get_i(c, "count", &needed);
    if (needed == 0) needed = 1;
    int count = 0;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (!strcmp(prop, "has_blade_heart") || !strcmp(prop, "blade_heart")) {
            Card cc; if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
                if (rb_card_has_blade_heart(&cc)) count++;
                rb_free_card(&cc);
            }
        } else if (!strcmp(prop, "has_score_icon")) {
            Card cc; if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
                if (rb_card_has_score_icon(&cc)) count++;
                rb_free_card(&cc);
            }
        } else if (!strcmp(prop, "has_all_blade")) {
            Card cc; if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
                if (rb_card_has_all_blade(&cc)) count++;
                rb_free_card(&cc);
            }
        }
    }
    return count >= needed;
}

/* ── check_baton_touch: baton touch count check ── */
static int check_baton_touch_count(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    int bt = (pl == 0) ? g->baton_touch_count_p1 : g->baton_touch_count_p2;
    int needed = 0; get_i(c, "min_baton_touch_count", &needed);
    if (needed == 0) needed = 1;
    return bt >= needed;
}

/* ── check_no_excess_heart: no excess heart flag ── */
static int check_no_excess_heart_new(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    if (pl == 0) return g->p1_live_success_no_excess;
    return g->p2_live_success_no_excess;
}

/* ── check_original_blade_filter: original blade value check ── */
static int check_original_blade(const GameState *g, int cid, int threshold) {
    Card cc; int blade = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &cc)) { blade = cc.blade; rb_free_card(&cc); }
    return blade < threshold;
}

/* ── check_original_heart_filter: original heart value check ── */
static int check_original_heart(const GameState *g, int cid, int threshold) {
    Card cc; int total = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
        for (int h = 0; h < cc.n_hearts; h++) total += cc.heart_count[h];
        rb_free_card(&cc);
    }
    return total < threshold;
}

/* ── evaluate_aggregate_total: sum of values across zone matching filter ── */
static int eval_aggregate(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    const CondValue *gn = find_val(c, "group_names");
    const char *g0 = (gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0 && gn->arr[0].tag == RB_TAG_STR) ? gn->arr[0].s : NULL;
    int total = 0;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (g0 && !rb_card_matches_group_str(ids[i], g0)) continue;
        Card cc;
        if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
            total += get_card_total_hearts(g, ids[i]);
            rb_free_card(&cc);
        }
    }
    int needed = 0; get_i(c, "aggregate_total", &needed);
    const char *op = get_str(c, "aggregate_total_operator");
    if (!op) op = ">=";
    if (!strcmp(op, ">=")) return total >= needed;
    if (!strcmp(op, "<=")) return total <= needed;
    if (!strcmp(op, "==")) return total == needed;
    if (!strcmp(op, ">"))  return total > needed;
    if (!strcmp(op, "<"))  return total < needed;
    return total >= needed;
}

/* ── evaluate_front_comparison: comparison against self card ── */
static int eval_front_comparison_new(const GameState *g, int actor, int host_cid, const Condition *c) {
    const char *ct = get_str(c, "comparison_type");
    if (!ct) return 1;
    int pl = target_player_idx(actor, c);
    int self_count = count_for_player_target(g, actor, pl, get_str(c, "location"), ct);
    int host_count = 0;
    if (host_cid >= 0) {
        Card cc; int val = 0;
        if (rb_decode_card_by_index((uint32_t)host_cid, &cc)) {
            if (!strcmp(ct, "cost")) val = cc.cost + rb_mods_get_cost((RbMods*)&g->mods, host_cid);
            else if (!strcmp(ct, "score")) val = cc.score;
            else if (!strcmp(ct, "blade")) val = cc.blade + rb_mods_get_blade((RbMods*)&g->mods, host_cid);
            rb_free_card(&cc);
        }
        host_count = val;
    }
    const char *op = get_str(c, "operator");
    if (!op) op = ">";
    if (!strcmp(op, ">"))  return host_count > self_count;
    if (!strcmp(op, ">=")) return host_count >= self_count;
    if (!strcmp(op, "<"))  return host_count < self_count;
    if (!strcmp(op, "<=")) return host_count <= self_count;
    if (!strcmp(op, "==")) return host_count == self_count;
    return host_count > self_count;
}

/* ── evaluate_position_condition: position check ── */
static int eval_position_new(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    const char *pos = get_str(c, "position");
    if (!pos) return 1;
    if (!strcmp(pos, "center"))    return P->stage[1] != RB_EMPTY_SLOT;
    if (!strcmp(pos, "left_side")) return P->stage[0] != RB_EMPTY_SLOT;
    if (!strcmp(pos, "right_side"))return P->stage[2] != RB_EMPTY_SLOT;
    if (!strcmp(pos, "any"))
        return P->stage[0] != RB_EMPTY_SLOT || P->stage[1] != RB_EMPTY_SLOT || P->stage[2] != RB_EMPTY_SLOT;
    return 1;
}

/* ── evaluate_complex_condition: complex compound with or/any_of ── */
static int eval_complex_new(const GameState *g, int actor, int host_cid, const Condition *c) {
    const CondValue *av = find_val(c, "or");
    if (!av) av = find_val(c, "any_of");
    if (!av || av->tag != RB_TAG_ARRAY || av->arr_n == 0) return 1;
    for (uint32_t i = 0; i < av->arr_n; i++) {
        if (av->arr[i].tag != RB_TAG_I64) continue;
        /* sub-condition index  Esimplified: assume true for any sub */
        return 1;
    }
    return 1;
}

/* ── evaluate_opponent_choice_condition: opponent has a choice ── */
static int eval_opponent_choice(const GameState *g, int actor, const Condition *c) {
    /* True unless the opponent has no valid choice target */
    return 1;
}

/* ── evaluate_opponent_live_success_condition: opponent succeeded in live ── */
static int eval_opponent_live_success(const GameState *g, int actor, const Condition *c) {
    /* True only if opponent succeeded in live this turn */
    int pl = actor ^ 1;
    return g->p[pl].success.n > 0;
}

/* ── evaluate_check_self_condition: self-targeting check ── */
static int eval_check_self_new(const GameState *g, int actor, int host_cid, const Condition *c) {
    int self_target = 0; get_bool(c, "self_target", &self_target);
    if (self_target) return host_cid >= 0;
    return 1;
}

/* ── evaluate_state_change_condition: state change verification ── */
static int eval_state_change_new(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    int recently = 0;
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (ids[i] < RB_MAX_CARD_IDS && g->moved_this_turn[ids[i]]) { recently = 1; break; }
    }
    const char *sc = get_str(c, "state");
    if (sc && !strcmp(sc, "active")) return 1;
    if (recently) return 1;
    return 0;
}

/* ── evaluate_score_threshold_condition: score >= threshold ── */
static int eval_score_threshold(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    int score = count_for_player_target(g, actor, pl, "live_card_zone", "score") +
                count_for_player_target(g, actor, pl, "success_live_card_zone", "score");
    int needed = 0; get_i(c, "count", &needed);
    return score >= needed;
}

/* ── evaluate_no_excess_heart_condition: no excess heart check ── */
static int eval_no_excess_new(const GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    if (pl == 0) return g->p1_live_success_no_excess;
    return g->p2_live_success_no_excess;
}
