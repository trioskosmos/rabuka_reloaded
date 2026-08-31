#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Full CardFilter for move_cards — mirrors engine/src/ability/move_cards.rs
   Handles card_type + group_names + card_names (name fragments) filtering.
   Relay pools (those_cards/recently_moved/looked_at) are still stubbed to
   hand for the portable core; full relay lands with look pools in next
   batch. This already makes ~200 of the 338 move_cards faithful where
   the filter was the only inaccuracy. */

/* Mirrors engine/src/ability/util.rs CardFilter::matches — evaluate a card
    against EVERY filter field present on the effect (card_type, group,
    name fragments, cost limit/values, original blade, heart color, characters,
    and ability property). Extra fields are decoded verbatim by vm.c so all of
    these keys are available here. */
static const char *cmf_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}
static int cmf_cmp(const char *op, int a, int b){
    if(!op||!*op) return a==b;
    if(!strcmp(op,">=")) return a>=b;
    if(!strcmp(op,"<=")) return a<=b;
    if(!strcmp(op,">"))  return a>b;
    if(!strcmp(op,"<"))  return a<b;
    if(!strcmp(op,"==")||!strcmp(op,"=")) return a==b;
    if(!strcmp(op,"!=")) return a!=b;
    return a==b;
}
static int cmf_has_heart(const Card *c, int hc){
    if(hc<0||hc>7) return 0;
    for(int k=0;k<c->n_hearts;k++) if(c->heart_color[k]==(uint8_t)hc) return 1;
    return 0;
}
static int card_matches_filter(int card_idx, AbilityEffect *e){
    const char *ctype = cmf_extra(e,"card_type");
    if(ctype && !card_matches_card_type_filter(card_idx, ctype)) return 0;
    const char *gn = cmf_extra(e,"group_names");
    if(gn && !rb_card_matches_group_str(card_idx, gn)) return 0;
    const char *cnames = cmf_extra(e,"card_names");
    if(cnames){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int m = c.name && strstr(c.name, cnames); rb_free_card(&c);
        if(!m) return 0;
    }
    const char *nf = cmf_extra(e,"name_fragments");
    if(nf){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int m = c.name && strstr(c.name, nf); rb_free_card(&c);
        if(!m) return 0;
    }
    /* cost: cost_limit(operator) / cost_values / cost_limit_min(>=) / cost_limit_max(<=) / cost_total(op) */
    const char *cl = cmf_extra(e,"cost_limit");
    if(cl){
        int v=atoi(cl); const char *op=cmf_extra(e,"cost_operator");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(op,(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *cv = cmf_extra(e,"cost_values");
    if(cv){
        int v=atoi(cv); Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=(c.cost==v)||(c.score==v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *clmin = cmf_extra(e,"cost_limit_min");
    if(clmin){
        int v=atoi(clmin); Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(">=",(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *clmax = cmf_extra(e,"cost_limit_max");
    if(clmax){
        int v=atoi(clmax); Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp("<=",(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *ct = cmf_extra(e,"cost_total");
    if(ct){
        int v=atoi(ct); const char *op=cmf_extra(e,"cost_total_operator");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(op,(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    /* original (printed) blade base — mirrors util.rs original_blade_limit */
    const char *ob = cmf_extra(e,"original_blade_limit");
    if(ob){
        int v=atoi(ob); const char *op=cmf_extra(e,"original_blade_operator");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(op,(int)c.blade,v); rb_free_card(&c); if(!ok) return 0;
    }
    /* heart color — mirrors util.rs check_heart_colors */
    const char *hc = cmf_extra(e,"heart_color");
    if(hc){
        int col = (int)rb_parse_heart_color(hc);
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok = (col==(int)RB_HEART_ANY && c.n_hearts>0) || cmf_has_heart(&c, col);
        rb_free_card(&c); if(!ok) return 0;
    }
    /* characters / exclude_characters (group/unit/name fragments) */
    const char *chars = cmf_extra(e,"characters");
    if(chars && !rb_card_matches_group_str(card_idx, chars)) return 0;
    const char *exch = cmf_extra(e,"exclude_characters");
    if(exch && rb_card_matches_group_str(card_idx, exch)) return 0;
    /* card_property filter (mirrors util.rs check_card_property) — negation inverts */
    const char *cp = cmf_extra(e,"card_property");
    if(cp){
        const char *ng = cmf_extra(e,"negation");
        int neg = ng && !strcmp(ng,"true");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int has=0;
        if(!strcmp(cp,"has_blade_heart"))      has = rb_card_has_blade_heart(&c);
        else if(!strcmp(cp,"has_score_icon"))  has = rb_card_has_score_icon(&c);
        else if(!strcmp(cp,"has_all_blade"))   has = rb_card_has_all_blade(&c);
        if(neg) has=!has;
        rb_free_card(&c);
        if(!has) return 0;
    }
    return 1;
}

/* Exposed for engine.c handle_action — filtered move with relay pools.
   Mirrors engine/src/ability/move_cards.rs: the set of cards moved by an
   action is recorded in g->recently_moved so subsequent `preceding_moved` /
   `selected_cards` / `those_cards` references (conditions, chained effects)
   resolve against the actual moved cards. */
static int stage_area_of(RbPlayer *P, int cid){
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==cid) return q;
    return -1;
}
static int find_and_remove_card(RbPlayer *P, int cid){
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==cid){ P->stage[q]=-1; P->stage_wait[q]=0; return 1; }
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==cid){ for(int k=i;k<P->hand.n-1;k++) P->hand.cards[k]=P->hand.cards[k+1]; P->hand.n--; return 1; }
    for(int i=0;i<P->deck.n;i++) if(P->deck.cards[i]==cid){ for(int k=i;k<P->deck.n-1;k++) P->deck.cards[k]=P->deck.cards[k+1]; P->deck.n--; return 1; }
    for(int i=0;i<P->discard.n;i++) if(P->discard.cards[i]==cid){ for(int k=i;k<P->discard.n-1;k++) P->discard.cards[k]=P->discard.cards[k+1]; P->discard.n--; return 1; }
    for(int i=0;i<P->live.n;i++) if(P->live.cards[i]==cid){ for(int k=i;k<P->live.n-1;k++) P->live.cards[k]=P->live.cards[k+1]; P->live.n--; return 1; }
    for(int i=0;i<P->success.n;i++) if(P->success.cards[i]==cid){ for(int k=i;k<P->success.n-1;k++) P->success.cards[k]=P->success.cards[k+1]; P->success.n--; return 1; }
    for(int i=0;i<P->energy.n;i++) if(P->energy.cards[i]==cid){ for(int k=i;k<P->energy.n-1;k++) P->energy.cards[k]=P->energy.cards[k+1]; P->energy.n--; return 1; }
    return 0;
}
/* Place a card into a zone, or onto/under a stage area. dst_area: -1 = first
   empty stage slot, 0..2 = specific area. under=1 places beneath the member.
   dst_stage=1 routes to stage placement even when dst RbZone isn't RB_ZONE_STAGE. */
static void place_in_dst(RbPlayer *A, RbZone dst, int cid, int to_top, int to_bottom,
                         int dst_area, int under, int dst_stage){
    if(dst_stage || dst==RB_ZONE_STAGE){
        int area = dst_area;
        if(area<0){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ area=q; break; } }
        if(area<0 || area>=RB_STAGE_SIZE) return;
        if(under){ if(A->under_cards[area].n < RB_MAX_ZONE) A->under_cards[area].cards[A->under_cards[area].n++]=cid; }
        else { A->stage[area]=cid; A->stage_wait[area]=0; }
        return;
    }
    RbBag *db=NULL;
    if(dst==RB_ZONE_HAND) db=&A->hand; else if(dst==RB_ZONE_DECK) db=&A->deck;
    else if(dst==RB_ZONE_DISCARD) db=&A->discard; else if(dst==RB_ZONE_ENERGY) db=&A->energy;
    else if(dst==RB_ZONE_LIVE) db=&A->live; else if(dst==RB_ZONE_SUCCESS) db=&A->success;
    if(!db) return;
    if(db->n < RB_MAX_ZONE){
        if(to_bottom && dst==RB_ZONE_DECK){ db->cards[db->n++]=cid; }
        else if(to_top && dst==RB_ZONE_DECK){ for(int k=db->n;k>0;k--) db->cards[k]=db->cards[k-1]; db->cards[0]=cid; db->n++; }
        else db->cards[db->n++]=cid;
    }
}
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e){
    int drain_all = (e->count < 0);   /* count=-1 mirrors drain-all semantics */
    int cnt = drain_all ? 0x7fffffff : (e->count>=0? e->count : 1);
    const char *src_s = e->source ? e->source : "hand";
    const char *dst_s = e->destination ? e->destination : "discard";
    int relay = (!strcmp(src_s,"those_cards")||!strcmp(src_s,"recently_moved")||!strcmp(src_s,"looked_at")||!strcmp(src_s,"selected_cards"));

    /* ── Destination resolution (computed once; shared by both players) ── */
    RbZone dst=RB_ZONE_DISCARD;
    int dst_stage=0, dst_area=-1, dst_under=0;
    int to_top = e->destination && (!strcmp(e->destination,"deck_top")||!strcmp(e->destination,"deck_top_or_bottom"));
    int to_bottom = e->destination && !strcmp(e->destination,"deck_bottom");
    if(!strcmp(dst_s,"stage")||!strcmp(dst_s,"empty_area")){ dst_stage=1; dst_area=-1; }
    else if(!strcmp(dst_s,"same_area")){ dst_stage=1; dst_area=-2; } /* -2 = same area the card came from */
    else if(!strcmp(dst_s,"under_member")){ dst_stage=1; dst_area=-3; dst_under=1; } /* -3 = under source area / first staged */
    else if(!strcmp(dst_s,"those_cards")||!strcmp(dst_s,"recently_moved")||!strcmp(dst_s,"looked_at")){ dst=RB_ZONE_DISCARD; }
    else rb_zone_of_str(dst_s,&dst);

    /* ── Target players (Rule: "both" applies to self AND opponent) ── */
    int players[2]; int np=0;
    if (e->target && !strcmp(e->target,"both")) { players[np++]=actor; players[np++]=actor^1; }
    else if (e->target && !strcmp(e->target,"opponent")) { players[np++]=actor^1; }
    else { players[np++]=actor; }

    int moved_ids[RB_MAX_ZONE]; int nm=0;
    for(int pk=0; pk<np; pk++){
        RbPlayer *A=&g->p[players[pk]];
        int is_deck = (!relay && !strcmp(src_s,"deck"));

        /* ── Source collection (deck source may be refilled by refresh) ── */
        int src_ids[RB_MAX_ZONE]; int src_area[RB_MAX_ZONE]; int ns=0;
        if(!strcmp(src_s,"looked_at")||!strcmp(src_s,"looked_at_remaining")){
            ns = rb_looked_at_pool(actor, src_ids, RB_MAX_ZONE);
            for(int i=0;i<ns;i++) src_area[i]=-1;
        } else if(relay){
            if(!strcmp(src_s,"selected_cards")){
                for(int i=0;i<g->n_selected_cards && ns<cnt;i++){ src_ids[ns]=g->selected_cards[i]; src_area[ns]=-1; ns++; }
            } else if(!strcmp(src_s,"those_cards")){
                /* Rust `those_cards` relay: the cards moved by the immediately
                    preceding move_cards action (recorded below). */
                for(int i=0;i<g->n_those_cards && ns<cnt;i++){ src_ids[ns]=g->those_cards[i]; src_area[ns]=-1; ns++; }
            } else {
                for(int i=0;i<g->n_recently_moved && ns<cnt;i++){ src_ids[ns]=g->recently_moved[i]; src_area[ns]=-1; ns++; }
            }
        } else {
            RbZone src=RB_ZONE_HAND; rb_zone_of_str(src_s,&src);
            if(src==RB_ZONE_STAGE){ for(int pos=0;pos<RB_STAGE_SIZE && ns<cnt;pos++) if(A->stage[pos]>=0 && card_matches_filter(A->stage[pos],e)){ src_ids[ns]=A->stage[pos]; src_area[ns]=pos; ns++; } }
            else { RbBag *sb=NULL;
                if(src==RB_ZONE_HAND) sb=&A->hand; else if(src==RB_ZONE_DECK) sb=&A->deck;
                else if(src==RB_ZONE_DISCARD) sb=&A->discard; else if(src==RB_ZONE_ENERGY) sb=&A->energy;
                else if(src==RB_ZONE_LIVE) sb=&A->live; else if(src==RB_ZONE_SUCCESS) sb=&A->success;
                if(sb){ for(int i=sb->n-1;i>=0 && ns<cnt;i--) if(card_matches_filter(sb->cards[i],e)){ src_ids[ns]=sb->cards[i]; src_area[ns]=-1; ns++; } }
            }
        }

        /* ── Move (deck source refreshes mid-mill per Rule 10.2.2.1) ── */
        int moved=0;
        for(int i=0;i<ns;i++){
            int cid=src_ids[i];
            if(!find_and_remove_card(A,cid)) continue;
            if(dst_stage){
                int area=dst_area;
                if(area==-2) area=src_area[i];
                if(area==-3){ area=src_area[i]; dst_under=1; }
                if(area<0){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ area=q; break; } }
                place_in_dst(A,dst,cid,to_top,to_bottom,area,dst_under,1);
            } else {
                place_in_dst(A,dst,cid,to_top,to_bottom,-1,0,0);
            }
            moved_ids[nm++]=cid; moved++;
        }
        /* Deck refill via refresh when the mill ran the deck dry mid-effect. */
        if(is_deck && moved < cnt){
            while(moved < cnt){
                if(A->deck.n==0){
                    if(A->discard.n>0){ /* Rule 10.2.2.1: shuffle waitroom under deck */
                        rb_shuffle(A->discard.cards, A->discard.n);
                        for(int k=0;k<A->discard.n;k++) A->deck.cards[A->deck.n++]=A->discard.cards[k];
                        A->discard.n=0;
                        A->deck_refreshed_this_turn=1;
                    } else break;
                }
                int cid=A->deck.cards[--A->deck.n];
                if(dst_stage){
                    int area=dst_area;
                    if(area<0){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ area=q; break; } }
                    place_in_dst(A,dst,cid,to_top,to_bottom,area,dst_under,1);
                } else {
                    place_in_dst(A,dst,cid,to_top,to_bottom,-1,0,0);
                }
                moved_ids[nm++]=cid; moved++;
            }
        }
    }
    /* Record the moved set for `preceding_moved`/`those_cards` relay references.
        `those_cards` holds exactly the cards this move_cards just moved, so the
        next move_cards with source="those_cards" resolves against them (Rust
        `those_cards` relay). `recently_moved` is the broader batch pool. */
    g->n_recently_moved = nm < RB_MAX_RECENTLY_MOVED ? nm : RB_MAX_RECENTLY_MOVED;
    for(int i=0;i<g->n_recently_moved;i++) g->recently_moved[i]=moved_ids[i];
    g->n_those_cards = nm < RB_MAX_RECENTLY_MOVED ? nm : RB_MAX_RECENTLY_MOVED;
    for(int i=0;i<g->n_those_cards;i++) g->those_cards[i]=moved_ids[i];
    /* Mirror GameState::has_card_moved_this_turn — mark every card this move just
        moved so temporal/movement conditions ("このターンに移動している") can gate on it. */
    for(int i=0;i<nm;i++) g->moved_this_turn[moved_ids[i]] = 1;
}

/* Mirror move_cards.rs::move_from_under_member — pull the cards at the given
    global under_member indices out from beneath stage members and place them
    into dst. validate(card_id) must return nonzero for the card to be moved
    (NULL = accept all). Returns the count moved, or -1 on a missing/invalid
    index. Host stage member ids are recorded in g->mods.last_under_move_host_ids
    so a following gain step can target them specifically. */
int rb_move_from_under_member(GameState *g, int actor, const int *indices, int n_indices,
                              int (*validate)(int), const char *dst, const char *target) {
    if (!g) return -1;
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];

    /* Stage the (area, card_id) pairs to move (mirrors cards_to_move: Vec<(usize,i16)>). */
    int host_ids[4]; int nh = 0;
    for (int k = 0; k < n_indices; k++) {
        int idx = indices ? indices[k] : -1;
        if (idx < 0) return -1;
        int global = 0, found = 0, si = -1, cid = -1;
        for (si = 0; si < RB_STAGE_SIZE; si++) {
            int len = P->under_cards[si].n;
            if (idx < global + len) {
                cid = P->under_cards[si].cards[idx - global];
                if (validate && !validate(cid)) return -1; /* type filter mismatch */
                found = 1;
                break;
            }
            global += len;
        }
        if (!found) return -1; /* index not found in under_member */
        /* Remove the card from under_cards[si]. */
        for (int j = idx - global; j < P->under_cards[si].n - 1; j++)
            P->under_cards[si].cards[j] = P->under_cards[si].cards[j + 1];
        P->under_cards[si].n--;
        rb_place_card_in_zone(g, pl, cid, dst ? dst : "discard", -1);
        /* Record the hosting stage member (mirrors last_under_move_host_ids). */
        int host = P->stage[si];
        if (host >= 0) {
            int dup = 0;
            for (int h = 0; h < nh; h++) if (host_ids[h] == host) { dup = 1; break; }
            if (!dup && nh < 4) host_ids[nh++] = host;
        }
    }

    /* Persist host ids into the mods record (mirrors gs.mods.last_under_move_host_ids). */
    g->mods.n_last_under_move_host_ids = 0;
    for (int h = 0; h < nh; h++) {
        int dup = 0;
        for (int i = 0; i < g->mods.n_last_under_move_host_ids; i++)
            if (g->mods.last_under_move_host_ids[i] == host_ids[h]) { dup = 1; break; }
        if (!dup && g->mods.n_last_under_move_host_ids < 4)
            g->mods.last_under_move_host_ids[g->mods.n_last_under_move_host_ids++] = (int16_t)host_ids[h];
    }
    rb_recalc_constants(g);
    return n_indices;
}

/* Mirror move_cards.rs::drain_under_cards_to_energy_zone — pull every card
    tucked under the given stage member and route it to the energy zone (if it
    is an energy card, marked wait) or the waitroom otherwise. Returns the
    number of cards moved. */
int rb_drain_under_cards_to_energy_zone(GameState *g, const char *target, int stage_idx) {
    if (!g || stage_idx < 0 || stage_idx >= RB_STAGE_SIZE) return 0;
    int pl = 0;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];
    int n = P->under_cards[stage_idx].n;
    int moved = 0;
    for (int i = 0; i < n; i++) {
        int cid = P->under_cards[stage_idx].cards[i];
        if (rb_card_is_energy(cid)) {
            if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
            rb_mods_set_orientation(&g->mods, cid, "wait");
        } else {
            if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
        }
        moved++;
    }
    P->under_cards[stage_idx].n = 0;
    rb_recalc_constants(g);
    return moved;
}

/* Mirror move_cards.rs::remove_card_from_any_zone — remove a single card from
    whichever zone currently holds it. When it leaves the stage the vacated area
    index is written back to *last_vacated (mirrors rule 9.6.2.1.2.1 tracking). */
static void remove_card_from_any_zone(RbPlayer *P, int *last_vacated, int cid) {
    int i;
    for (i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == cid) {
        for (int k = i; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
        P->hand.n--; return;
    }
    for (i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == cid) {
        for (int k = i; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
        P->discard.n--; return;
    }
    for (i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == cid) {
        P->stage[i] = -1; P->stage_wait[i] = 0;
        if (last_vacated) *last_vacated = i;
        return;
    }
    for (i = 0; i < P->energy.n; i++) if (P->energy.cards[i] == cid) {
        for (int k = i; k < P->energy.n - 1; k++) P->energy.cards[k] = P->energy.cards[k + 1];
        P->energy.n--; return;
    }
    for (i = 0; i < P->live.n; i++) if (P->live.cards[i] == cid) {
        for (int k = i; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
        P->live.n--; return;
    }
}

/* needed by engine.c wrapper */
int card_matches_card_type_filter(int card_idx, const char *filter);

/* Mirror move_cards.rs::execute_selected_energy_zone_cards — for each energy-zone
    card at the given indices, clear its modifiers and flip it to "wait" orientation,
    and decrement the player's active energy count by the number of marked cards. */
void rb_effect_selected_energy_zone_cards(GameState *g, int actor, const int *indices, int n_indices) {
    if (!g || n_indices <= 0) return;
    int pl = actor;
    RbPlayer *P = &g->p[pl];
    int to_mark[RB_MAX_ZONE]; int nm = 0;
    for (int i = 0; i < n_indices && nm < RB_MAX_ZONE; i++) {
        int idx = indices[i];
        if (idx >= 0 && idx < P->energy.n) to_mark[nm++] = P->energy.cards[idx];
    }
    /* active_energy_count -= marked count (saturating at 0). */
    int sub = nm < 32768 ? nm : 32767;
    P->energy_active = P->energy_active > sub ? P->energy_active - sub : 0;
    for (int i = 0; i < nm; i++) {
        rb_mods_clear_card(&g->mods, to_mark[i]);
        rb_mods_set_orientation(&g->mods, to_mark[i], "wait");
    }
}
