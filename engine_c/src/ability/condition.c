#include "rabuka.h"
#include <string.h>
#include <stdio.h>
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
    if (!strcmp(loc, "resolution")||!strcmp(loc,"resolution_zone")) return 0; /* not tracked */
    if (!strcmp(loc, "revealed_cards")) return 0;
    if (!strcmp(loc, "empty_area")) {
        int e=0; for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==RB_EMPTY_SLOT) e++;
        return e;
    }
    return 0;
}
static int count_distinct_in_zone(const struct GameState *g, int pl, const char *loc) {
    if (!loc) return 0;
    const RbPlayer *P = &g->p[pl];
    int ids[RB_MAX_ZONE]; int n=0;
    if (!strcmp(loc, "hand")){ for(int i=0;i<P->hand.n;i++) ids[n++]=P->hand.cards[i]; }
    else if (!strcmp(loc, "stage")){ for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]!=RB_EMPTY_SLOT) ids[n++]=P->stage[i]; }
    else if (!strcmp(loc, "deck")){ for(int i=0;i<P->deck.n;i++) ids[n++]=P->deck.cards[i]; }
    else if (!strcmp(loc, "discard")||!strcmp(loc,"waitroom")){ for(int i=0;i<P->discard.n;i++) ids[n++]=P->discard.cards[i]; }
    else return count_in_zone(g,pl,loc);
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
static int zone_count_filtered(const struct GameState *g, int pl, const char *loc, const char *card_type){
    int base = count_in_zone(g,pl,loc);
    if(!card_type) return base;
    /* filter: live_card vs member_card vs energy_card */
    const RbPlayer *P=&g->p[pl];
    int ids[RB_MAX_ZONE]; int n=0;
    if (!strcmp(loc, "hand")){ for(int i=0;i<P->hand.n;i++) ids[n++]=P->hand.cards[i]; }
    else if (!strcmp(loc, "stage")){ for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]!=RB_EMPTY_SLOT) ids[n++]=P->stage[i]; }
    else return base; /* other zones: no filter for now */
    int filtered=0;
    for(int i=0;i<n;i++){
        Card c; if(!rb_decode_card_by_index((uint32_t)ids[i],&c)) continue;
        int is_live = (c.n_hearts==0 && c.cost==0 && c.blade==0);
        int is_member = !is_live;
        int match=0;
        if(!strcmp(card_type,"live_card") && is_live) match=1;
        else if(!strcmp(card_type,"member_card") && is_member) match=1;
        else if(!strcmp(card_type,"energy_card")) match=0; /* no energy cards in hand/stage */
        else if(!strcmp(card_type,"card")) match=1;
        if(match) filtered++;
        rb_free_card(&c);
    }
    return filtered;
}

/* Forward */
static int eval_condition_inner(const struct GameState *g, int actor, const Condition *c);

/* ── compound (variant 0) / or ── */
static int eval_compound(const struct GameState *g, int actor, const Condition *c) {
    const char *op = get_str(c, "operator");
    const CondValue *v = find_val(c, "conditions");
    if (!v || v->tag != RB_TAG_ARRAY || !v->arr) return 1;
    int is_or = op && !strcmp(op, "or");
    for (uint32_t i=0;i<v->arr_n;i++) {
        const CondValue *cv = &v->arr[i];
        if (cv->tag != RB_TAG_OBJVAR || !cv->cond) continue;
        int r = eval_condition_inner(g, actor, cv->cond);
        if (is_or && r) return 1;
        if (!is_or && !r) return 0;
    }
    return is_or ? 0 : 1;
}

/* ── location / card_count (variant 1) ── */
static int eval_location(const struct GameState *g, int actor, const Condition *c) {
    int has_count = 0; int cnt_thr = 1;
    int tmp;
    has_count = get_i(c, "count", &tmp); if (has_count) cnt_thr = tmp;
    /* distinct flag — if true, count distinct card names not total cards */
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
    int actual = 0;
    if (ctype) actual = zone_count_filtered(g, pl, loc, ctype);
    else if (distinct) actual = count_distinct_in_zone(g, pl, loc);
    else actual = count_in_zone(g, pl, loc);

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
    // Text: "center area has member with greatest cost" — i.e. center member exists and its cost is strictly greater than all others.
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
   `values` is a list of card scores; route to success/live cards and
   require that EVERY listed score is present. Not yet dispatched from
   eval_comparison_inner because the C bytecode merges both_condition into
   the same variant 2 as comparison_condition (no wire-type discriminator);
   kept for future routing once the envelope carries the condition type. */
static int __attribute__((unused)) eval_both_condition(const struct GameState *g, int actor, const Condition *c) {
    const CondValue *vv = find_val(c, "values");
    if (!vv || vv->tag != RB_TAG_ARRAY || vv->arr_n == 0) return 0;
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    for (uint32_t i = 0; i < vv->arr_n; i++) {
        int want = (vv->arr[i].tag == RB_TAG_I64) ? (int)vv->arr[i].i
                 : (vv->arr[i].tag == RB_TAG_STR && vv->arr[i].s) ? atoi(vv->arr[i].s) : 0;
        int found = 0;
        for (int j = 0; j < P->success.n && !found; j++) {
            Card cc; if (rb_decode_card_by_index((uint32_t)P->success.cards[j], &cc)) {
                if (cc.score == want) found = 1; rb_free_card(&cc);
            }
        }
        for (int j = 0; j < P->live.n && !found; j++) {
            Card cc; if (rb_decode_card_by_index((uint32_t)P->live.cards[j], &cc)) {
                if (cc.score == want) found = 1; rb_free_card(&cc);
            }
        }
        if (!found) return 0;
    }
    return 1;
}

static int eval_comparison_inner(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    const char *loc = get_str(c, "location");
    const char *agg = get_str(c, "aggregate");
    const char *ctype = get_str(c, "comparison_type");
    /* hanayo: location=success_live_card_zone, card_type=live_card, count=6, operator=>=, comparison_type=score, aggregate=total
       → sum of card scores in zone, not count. Mirrors engine/src/ability/condition/card.rs evaluate_comparison. */
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
    if (loc) return eval_location(g, actor, c);
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
static int eval_movement(const struct GameState *g, int actor, const Condition *c) {
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
static int eval_group(const struct GameState *g, int actor, const Condition *c) {
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int pl = target_player_idx(actor, c);
    const CondValue *gv = find_val(c, "group_names");
    if (!gv || gv->tag != RB_TAG_ARRAY || gv->arr_n==0) return count_in_zone(g, pl, loc)>0;
    /* Check if any card in zone has group matching any of group_names */
    const RbPlayer *P=&g->p[pl];
    int ids[RB_MAX_ZONE]; int n=0;
    if (!strcmp(loc,"stage")){ for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]!=RB_EMPTY_SLOT) ids[n++]=P->stage[i]; }
    else if(!strcmp(loc,"hand")){ for(int i=0;i<P->hand.n;i++) ids[n++]=P->hand.cards[i]; }
    else if(!strcmp(loc,"discard")||!strcmp(loc,"waitroom")){ for(int i=0;i<P->discard.n;i++) ids[n++]=P->discard.cards[i]; }
    else return count_in_zone(g,pl,loc)>0;
    for(int i=0;i<n;i++){
        Card card; if(!rb_decode_card_by_index((uint32_t)ids[i],&card)) continue;
        const char *gname = rb_card_string(card.group_idx);
        const char *uname = rb_card_string(card.unit_idx);
        if (gname && (strstr(gname,"μ")||strstr(gname,"ミ"))) {
            for(uint32_t gi=0;gi<gv->arr_n;gi++) if(gv->arr[gi].tag==RB_TAG_STR && gv->arr[gi].s)
                fprintf(stderr,"[grp] card=%s gname=%s uname=%s target=%s\n",
                        card.name?card.name:"?", gname, uname?uname:"-", gv->arr[gi].s);
        }
        for(uint32_t gi=0;gi<gv->arr_n;gi++){
            const char *t = (gv->arr[gi].tag==RB_TAG_STR)?gv->arr[gi].s:NULL;
            if(!t) continue;
            if(gname && (!strcmp(gname,t)||strstr(gname,t)||strstr(t,gname))) { rb_free_card(&card); return 1; }
            if(uname && (!strcmp(uname,t)||strstr(uname,t)||strstr(t,uname))) { rb_free_card(&card); return 1; }
        }
        rb_free_card(&card);
    }
    return 0;
}

/* ── appearance (variant 5) ── */
static int eval_appearance(const struct GameState *g, int actor, const Condition *c) {
    int ap=0; if (get_bool(c,"appearance",&ap) && !ap) return 0;
    const CondValue *chars = find_val(c, "characters");
    if (!chars || chars->tag!=RB_TAG_ARRAY || chars->arr_n==0) return 1;
    int pl = target_player_idx(actor, c);
    /* characters/group filter: check stage has at least one card matching any listed group/character */
    const RbPlayer *P=&g->p[pl];
    int ids[RB_MAX_ZONE]; int n=0;
    for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]!=RB_EMPTY_SLOT) ids[n++]=P->stage[i];
    if(n==0) return 0;
    for(int i=0;i<n;i++){
        Card card; if(!rb_decode_card_by_index((uint32_t)ids[i],&card)) continue;
        const char *gname = rb_card_string(card.group_idx);
        const char *uname = rb_card_string(card.unit_idx);
        int matched=0;
        for(uint32_t gi=0;gi<chars->arr_n;gi++) if(chars->arr[gi].tag==RB_TAG_STR && chars->arr[gi].s){
            if(gname && (!strcmp(gname,chars->arr[gi].s)||strstr(gname,chars->arr[gi].s))) matched=1;
            if(uname && (!strcmp(uname,chars->arr[gi].s)||strstr(uname,chars->arr[gi].s))) matched=1;
        }
        rb_free_card(&card);
        if(matched) return 1;
    }
    return 0;
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
static int eval_temporal(const struct GameState *g, int actor, const Condition *c) {
    (void)actor;
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
        /* this_turn with a count defaults to debut_count_this_turn which the
           C port does not track; fall back to true per Rust's no-nested default. */
        if (!strcmp(temp,"this_turn")) return 1;
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
static int eval_state(const struct GameState *g, int actor, const Condition *c) {
    const char *res = get_str(c, "resource_type");
    const char *es  = get_str(c, "energy_state");
    const char *st  = get_str(c, "state");
    if (!st) st = get_str(c, "from_state");
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
            if (is_all) return P->energy.n > 0 && P->energy_active == 0;
            return P->energy_active < P->energy.n;
        }
        return 1;
    }

    /* member active/wait state */
    if (st && (!strcmp(st, "active") || !strcmp(st, "wait"))) {
        int matching = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) {
            int is_wait = P->stage_wait[i];
            const char *cur = is_wait ? "wait" : "active";
            if (!strcmp(st, cur)) matching++;
            const char *om = rb_mods_get_orientation((RbMods*)&g->mods, P->stage[i]);
            if (om && !strcmp(om, st)) matching++;
        }
        const char *op = get_str(c, "operator");
        int cnt = 1; get_i(c, "count", &cnt);
        return eval_operator(matching, op, cnt);
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
           Mirror card.rs:evaluate_card_blade_condition — an EMPTY selection set
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
       Mirrors engine/src/turn/live.rs compute_surplus_and_flags → surplus_hearts == 0.
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
static int eval_ability_filter(const struct GameState *g, int actor, const Condition *c) {
    const char *filter = get_str(c, "ability_filter");
    if (!filter) filter = "no_ability";
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int pl = target_player_idx(actor, c);

    int ids[RB_MAX_ZONE]; int n=0;
    if (!strcmp(loc,"stage")) { for(int i=0;i<RB_STAGE_SIZE;i++) if(g->p[pl].stage[i]!=RB_EMPTY_SLOT) ids[n++]=g->p[pl].stage[i]; }
    else if (!strcmp(loc,"hand")) { for(int i=0;i<g->p[pl].hand.n;i++) ids[n++]=g->p[pl].hand.cards[i]; }
    else if (!strcmp(loc,"discard")||!strcmp(loc,"waitroom")) { for(int i=0;i<g->p[pl].discard.n;i++) ids[n++]=g->p[pl].discard.cards[i]; }
    else if (!strcmp(loc,"energy")) { for(int i=0;i<g->p[pl].energy.n;i++) ids[n++]=g->p[pl].energy.cards[i]; }
    else if (!strcmp(loc,"live")||!strcmp(loc,"live_card_zone")) { for(int i=0;i<g->p[pl].live.n;i++) ids[n++]=g->p[pl].live.cards[i]; }
    else { /* fall back to activating card (host_cid) */ }

    /* trigger prefixes */
    const CondValue *tv = get_arr(c, "ability_filter_triggers");

    int has_ability = 0;
    if (n == 0) {
        /* no zone cards: ability_filter resolves to the activating card; if
           none, mirror Rust (returns false for has_ability, true for no_ability). */
    } else {
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
            int hb = c.has_special; /* proxy: blade heart ≈ special/blade heart flag */
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
static int eval_choice(const struct GameState *g, int actor, const Condition *c) {
    /* Choice conditions are interactive; headless eval treats them as
       satisfiable. If a nested "condition" field is present, gate on it. */
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_OBJVAR && f->v.cond && f->key && !strcmp(f->key, "condition"))
            return rb_eval_condition(g, actor, f->v.cond);
    }
    return 1;
}

/* Complex condition (variant 12) — Rust ANDs a nested `cause` (and `effect`)
   condition. The decoder stores nested conditions as RB_TAG_OBJVAR CondValues,
   so we evaluate every nested sub-condition with AND (OR when the field is an
   array keyed "or"/"any_of"). Mirrors state.rs:evaluate_complex_condition. */
static int eval_complex(const struct GameState *g, int actor, const Condition *c) {
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_OBJVAR && f->v.cond) {
            if (!rb_eval_condition(g, actor, f->v.cond)) return 0;
        } else if (f->v.tag == RB_TAG_ARRAY) {
            int combine_or = (f->key && (!strcmp(f->key, "or") ||
                                         !strcmp(f->key, "any_of") ||
                                         !strcmp(f->key, "any")));
            int any = 0, all = 1;
            for (uint32_t j = 0; j < f->v.arr_n; j++) {
                CondValue *e = &f->v.arr[j];
                if (e->tag == RB_TAG_OBJVAR && e->cond) {
                    if (rb_eval_condition(g, actor, e->cond)) any = 1; else all = 0;
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
    { const char *dp=get_str(c,"position"), *dl=get_str(c,"location"), *dc=get_str(c,"comparison_type"),
        *dt=get_str(c,"target"), *dcs=get_str(c,"check_self");
      if(dp&&!strcmp(dp,"center")) fprintf(stderr,"[cond] v=%d pos=center loc=%s cmptype=%s target=%s checkself=%s\n",
          c->variant, dl?dl:"-", dc?dc:"-", dt?dt:"-", dcs?dcs:"-"); }
    int r=1;
    switch ((RbConditionVariant)c->variant) {
        case RB_COND_COMPOUND:            r = eval_compound(g, actor, c); break;
        case RB_COND_LOCATION:            r = eval_location(g, actor, c); break;
        case RB_COND_COMPARISON:          r = eval_comparison_inner(g, actor, host_cid, c); break;
        case RB_COND_MOVEMENT:            r = eval_movement(g, actor, c); break;
        case RB_COND_GROUP:               r = eval_group(g, actor, c); break;
        case RB_COND_APPEARANCE:          r = eval_appearance(g, actor, c); break;
        case RB_COND_TEMPORAL:            r = eval_temporal(g, actor, c); break;
        case RB_COND_STATE:               r = eval_state(g, actor, c); break;
        case RB_COND_RESOURCE:            r = eval_resource(g, actor, c); break;
        case RB_COND_ABILITY_FILTER:      r = eval_ability_filter(g, actor, c); break;
        case RB_COND_SCORE_THRESHOLD:     r = eval_score(g, actor, c); break;
        case RB_COND_CHOICE:              r = eval_choice(g, actor, c); break;
        case RB_COND_COMPLEX:             r = eval_complex(g, actor, c); break;
        case RB_COND_POSITION:            r = eval_position(g, actor, c); break;
        case RB_COND_OPPONENT_CHOICE:
            /* Mirror state.rs:evaluate_opponent_choice_condition — true unless the
               opponent declined. Headless has no opponent-decline state, so assume
               the opponent accepted (gs.opponent_choice_declined == false). Negation
               is applied by rb_eval_condition's top-level wrapper, so return raw. */
            r = 1;
            break;
        case RB_COND_OPPONENT_LIVE_SUCCESS:
            /* Mirror state.rs:evaluate_opponent_live_success_condition — true only if
               the owner's opponent won their live this turn. Headless tracks no
               live-success flag yet, so return false (no live => not succeeded). */
            r = 0;
            break;
        case RB_COND_NO_EXCESS_HEART:     r = eval_no_excess(g, actor, c); break;
        case RB_COND_ALWAYS_TRUE:         r = 1; break;
        case RB_COND_ANY_OF:              r = eval_any_of(g, actor, c); break;
        case RB_COND_ALL_REVEALED:
            /* Mirror condition.rs:evaluate_all_revealed_match_heart_color — true if
               >= count revealed cards match the heart color. Headless has no
               revealed_cards list, so matching == 0 => false for any threshold. */
            r = 0;
            break;
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
