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
    if (pos && !strcmp(pos, "center") && loc && !strcmp(loc, "stage") && host_cid >= 0) {
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
        for(uint32_t gi=0;gi<gv->arr_n;gi++) if(gv->arr[gi].tag==RB_TAG_STR && gv->arr[gi].s && gname && !strcmp(gname, gv->arr[gi].s)){
            rb_free_card(&card); return 1;
        }
        /* also substring match for normalized group */
        for(uint32_t gi=0;gi<gv->arr_n;gi++) if(gv->arr[gi].tag==RB_TAG_STR && gv->arr[gi].s && gname && strstr(gname, gv->arr[gi].s)){
            rb_free_card(&card); return 1;
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

/* ── temporal (variant 6) ── */
static int eval_temporal(const struct GameState *g, int actor, const Condition *c) {
    (void)actor;
    int tn=0;
    if (get_i(c,"turn_number",&tn)) {
        const char *op = get_str(c,"operator");
        return eval_operator(g->turn, op, tn);
    }
    const char *phase = get_str(c,"phase");
    if (phase) {
        /* crude phase gate: check current phase string */
        if (!strcmp(phase,"main") || !strcmp(phase,"main_phase")) return g->phase==RB_PHASE_MAIN;
        if (!strcmp(phase,"active")) return g->phase==RB_PHASE_ACTIVE;
        if (!strcmp(phase,"live") || !strcmp(phase,"live_phase")) return g->phase==RB_PHASE_LIVE_SET || g->phase==RB_PHASE_PERFORMANCE;
    }
    return 1;
}

/* ── state (variant 7) ── */
static int eval_state(const struct GameState *g, int actor, const Condition *c) {
    const char *st = get_str(c,"state");
    if (!st) st = get_str(c,"from_state");
    if (!st) return 1;
    int pl = target_player_idx(actor, c);
    const RbPlayer *P = &g->p[pl];
    int matching=0;
    for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]!=RB_EMPTY_SLOT){
        int is_wait = P->stage_wait[i];
        const char *cur = is_wait? "wait":"active";
        if(!strcmp(st,cur)) matching++;
        /* also check orientation mods */
        const char *om = rb_mods_get_orientation((RbMods*)&g->mods, P->stage[i]);
        if(om && !strcmp(om,st)) matching++;
    }
    const char *op=get_str(c,"operator"); int cnt=1; get_i(c,"count",&cnt);
    return eval_operator(matching, op, cnt);
}

/* ── resource / card_blade (variant 8) + energy_state (variant ~11) ── */
static int eval_resource(const struct GameState *g, int actor, const Condition *c) {
    int pl = target_player_idx(actor, c);
    int thr=1; get_i(c,"count",&thr); if(!thr) thr=1;
    const char *op=get_str(c,"operator");
    /* energy_state_condition checks active energy or energy_zone count.
       Distinguish by location field if present. */
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

/* ── ability_filter (variant 9) ── */
static int eval_ability_filter(const struct GameState *g, int actor, const Condition *c) {
    (void)g;(void)actor;(void)c; return 1;
}
/* ── score_threshold (10) ── */
static int eval_score(const struct GameState *g, int actor, const Condition *c) {
    int thr=1; get_i(c,"count",&thr);
    const char *op=get_str(c,"operator");
    int pl=target_player_idx(actor,c);
    return eval_operator(g->p[pl].score, op, thr);
}
/* ── choice/position etc ── */
static int eval_choice(const struct GameState *g, int actor, const Condition *c){ (void)g;(void)actor;(void)c; return 1; }
static int eval_complex(const struct GameState *g, int actor, const Condition *c){ (void)g;(void)actor;(void)c; return 1; }


static int eval_condition_inner_host(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    if (!c) return 1;
    int negation=0; get_bool(c,"negation",&negation);
    int r=1;
    switch (c->variant) {
        case 0:  r = eval_compound(g, actor, c); break;
        case 1:  r = eval_location(g, actor, c); break;
        case 2:  r = eval_comparison_inner(g, actor, host_cid, c); break;
        case 3:  r = eval_movement(g, actor, c); break;
        case 4:  r = eval_group(g, actor, c); break;
        case 5:  r = eval_appearance(g, actor, c); break;
        case 6:  r = eval_temporal(g, actor, c); break;
        case 7:  r = eval_state(g, actor, c); break;
        case 8:  r = eval_resource(g, actor, c); break;
        case 9:  r = eval_ability_filter(g, actor, c); break;
        case 10: r = eval_score(g, actor, c); break;
        case 11: r = eval_choice(g, actor, c); break;
        case 12: r = eval_complex(g, actor, c); break;
        case 13: /* PositionCond */ r = eval_location(g, actor, c); break;
        case 14: /* OpponentChoice */ r = 1; break;
        case 15: /* OpponentLiveSuccess */ r = 0; break;
        case 16: /* NoExcessHeart */ r = eval_no_excess(g, actor, c); break;
        case 17: /* AlwaysTrue */ r = 1; break;
        case 18: /* AnyOf */ r = 1; break;
        case 19: /* AllRevealed */ r = 1; break;
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
