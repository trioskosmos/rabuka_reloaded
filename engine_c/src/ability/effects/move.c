#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdio.h>

/* Full CardFilter for move_cards — mirrors engine/src/ability/move_cards.rs
   Handles card_type + group_names + card_names (name fragments) filtering.
   Relay pools (those_cards/recently_moved/looked_at) are still stubbed to
   hand for the portable core; full relay lands with look pools in next
   batch. This already makes ~200 of the 338 move_cards faithful where
   the filter was the only inaccuracy. */

/* Forward declarations for functions added during move.rs port */
void rb_move_prompt_card_selection(GameState *g, int actor, const char *zone,
                                    int count, int can_skip, AbilityEffect *e);
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max);
int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                       int count, int *out_ids, int max);
int rb_move_resolve_from_zone(GameState *g, int actor, const char *effective_source,
                               AbilityEffect *e, int use_p2, int count,
                               int *out_ids, int max);
int rb_move_resolve_from_recently_moved(GameState *g, int use_p2,
                                        const char *card_type_filter,
                                        const char *group_name,
                                        int *out_ids, int max);
int rb_move_resolve_from_energy_deck(GameState *g, int pl, int count, int *out_ids, int max);
int rb_move_resolve_from_stage(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                int count, int *out_ids, int max);
int rb_move_resolve_from_under_member(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                       int count, int *out_ids, int max);
int rb_move_resolve_from_deck_bottom(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, const char *card_type_filter,
                                      const char *group_name, int *out_ids, int max);
int rb_move_resolve_source_looked_at(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, int *out_ids, int max);
int rb_move_from_revealed(GameState *g, int actor, const int *indices, int n_indices,
                           int (*validate_card)(int), const char *dst,
                           int *out_ids, int max);
void rb_move_execute_selected_cards_from_zone(GameState *g, int actor, const char *zone,
                                              const int *indices, int n_indices,
                                              const char *card_type_filter,
                                              int cost_limit, const char *cost_limit_op,
                                              const char *group,
                                              const char **characters, int n_characters,
                                              const char *target_player_id);
void rb_move_handle_select_cards_looked_at(GameState *g, int actor, const int *indices,
                                            int n_indices, const char *ctx_destination,
                                            int ctx_discard_remaining);
void rb_move_handle_energy_zone_selection(GameState *g, int actor, const int *indices,
                                           int n_indices, int count,
                                           const char *destination,
                                           int (*validate_card)(int));
void rb_move_finalize_card_movement(GameState *g, int actor,
                                    const int *moved_cards, int n_moved,
                                    const char *destination, const char *source,
                                    const char *state_change, const char *target);
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id,
                                     const char *target, const char *source);
int rb_move_execute_stage_placement_choices(GameState *g, int actor,
                                            const int *card_ids, int n_ids,
                                            const char *src_zone,
                                            const char *dest,
                                            int vacated_area,
                                            const char *target,
                                            int *out_ids, int max);
void rb_move_handle_select_position(GameState *g, int actor, const char *position,
                                     int card_id, const char *target,
                                     const char *source_zone, const char *state_change);
int rb_move_place_card_with_stage_choice(GameState *g, int actor, int host_cid,
                                           const char *player_target, int card_id,
                                           const char *destination, int vacated_area,
                                           int is_max, int count, const char *state_change,
                                           int deck_position, const char *source_zone,
                                           int allow_occupied_stage, int under_self);

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

/* ═══════════════════════════════════════════════════════════════════════════
   The following 9 functions mirror the unmatched Rust ability methods listed
   in SIZE_AUDIT.md under src/ability/effects/move.c. Each is a faithful port
   of the corresponding method on AbilityResolver / MoveSourceContext in
   engine/src/ability/move_cards.rs.
   ═══════════════════════════════════════════════════════════════════════════ */

/* Mirror MoveSourceContext::player_mut — select the target player by use_p2. */
static RbPlayer *mc_player_mut(GameState *g, int use_p2) {
    return use_p2 ? &g->p[1] : &g->p[0];
}

/* Mirror AbilityResolver::resolve_cost_limit_reference — resolve a dynamic cost
    limit from a reference to a previously moved card (e.g. "previous_moved_card"
    + offset). Returns the resolved u8 cost, or -1 when no reference applies. */
int rb_move_resolve_cost_limit_reference(const GameState *g, const AbilityEffect *e) {
    if (!g || !e) return -1;
    const char *reference = cmf_extra(e, "cost_reference");
    if (!reference) {
        const char *cl = cmf_extra(e, "cost_limit");
        return cl ? atoi(cl) : -1;
    }
    const char *offset_str = cmf_extra(e, "cost_offset");
    int offset = offset_str ? atoi(offset_str) : 0;
    int referenced_id = -1;
    if (!strcmp(reference, "previous_moved_card")) {
        if (g->n_recently_moved > 0)
            referenced_id = g->recently_moved[g->n_recently_moved - 1];
    }
    if (referenced_id < 0) return -1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)referenced_id, &c)) return -1;
    int base_cost = (int)c.cost;
    rb_free_card(&c);
    int resolved = base_cost + offset;
    if (resolved < 0) resolved = 0;
    if (resolved > 255) resolved = 255;
    return resolved;
}

/* Mirror AbilityResolver::resolve_from_looked_at — drain the looked_at pool into
    the target player's waitroom and return the drained card ids (out_count set
    to the number returned). Returns 0 on success, -1 on error. */
int rb_move_resolve_from_looked_at(GameState *g, int use_p2, int *out_ids, int max, int *out_count) {
    if (!g || !out_ids || !out_count) return -1;
    *out_count = 0;
    RbPlayer *P = mc_player_mut(g, use_p2);
    int ids[RB_MAX_RECENTLY_MOVED];
    int n = rb_looked_at_pool(use_p2 ? 1 : 0, ids, RB_MAX_RECENTLY_MOVED);
    for (int i = 0; i < n && *out_count < max; i++) {
        out_ids[(*out_count)++] = ids[i];
        if (P->discard.n < RB_MAX_ZONE)
            P->discard.cards[P->discard.n++] = ids[i];
    }
    return 0;
}

/* Mirror AbilityResolver::optional_gate_source — true when the source zone's
    optional 「〜してもよい」 moves go through the shared pay/skip gate. */
int rb_move_optional_gate_source(const char *zone_str) {
    if (!zone_str) return 0;
    return (!strcmp(zone_str, "deck") || !strcmp(zone_str, "deck_top") ||
            !strcmp(zone_str, "deck_bottom") || !strcmp(zone_str, "energy_deck") ||
            !strcmp(zone_str, "energy")) ? 1 : 0;
}

/* Mirror AbilityResolver::gate_optional_source — per-zone prompts for the
    shared optional-move gate. Emits a pay/skip choice and returns 1 when the
    caller must yield to the player, 0 when execution should proceed. */
int rb_move_gate_optional_source(GameState *g, int actor, const AbilityEffect *e,
                                  const char *zone_str, int count) {
    if (!g || !e || !zone_str) return 0;
    if (!e->is_optional) return 0;
    if (!rb_move_optional_gate_source(zone_str)) return 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, count > 0 ? count : 1, 1, "pay_optional_cost");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
    return 1;
}

/* Mirror AbilityResolver::resolve_from_deck — draw up to `count` cards from the
    deck top that match the card_type / group filter, refreshing from the
    waitroom when the deck runs dry mid-draw (Rule 10.2.1). Returns the number
    of cards placed in out_ids. */
int rb_move_resolve_from_deck(GameState *g, int actor, const AbilityEffect *e,
                               int use_p2, int count,
                               const char *card_type_filter, const char *group_name,
                               int *out_ids, int max) {
    if (!g || count <= 0 || !out_ids) return 0;
    if (rb_move_gate_optional_source(g, actor, e, "deck", count)) return 0;
    RbPlayer *P = mc_player_mut(g, use_p2);
    int drawn = 0;
    int attempts = 0;
    int remaining = count;
    int cap = count + P->deck.n + 10;
    while (remaining > 0 && attempts < cap) {
        if (P->deck.n == 0) {
            if (P->discard.n > 0) {
                rb_shuffle(P->discard.cards, P->discard.n);
                for (int k = 0; k < P->discard.n && P->deck.n < RB_MAX_ZONE; k++)
                    P->deck.cards[P->deck.n++] = P->discard.cards[k];
                P->discard.n = 0;
                P->deck_refreshed_this_turn = 1;
            } else break;
        }
        if (P->deck.n == 0) break;
        int cid = P->deck.cards[--P->deck.n];
        attempts++;
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) {
            if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
            continue;
        }
        if (group_name && !rb_card_matches_group_str(cid, group_name)) {
            if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
            continue;
        }
        if (drawn < max) out_ids[drawn++] = cid;
        remaining--;
    }
    return drawn;
}

/* Mirror AbilityResolver::take_cards_from_standard_zone — core card-selection
   logic for hand/discard/live/success zones. Resolves the selection outcome
   (Exact/Prompt/Skip) and either takes cards directly or prompts via
   rb_move_prompt_card_selection. Returns count taken (writes to out_ids), or
   -1 if a choice was prompted (caller yields). */
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    int cards[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone_name, cards, RB_MAX_ZONE);
    int idxs[RB_MAX_ZONE];
    int mn = rb_get_selection_indices(cards, n, ctype, gn,
                                       e->self_target_field[0] && !strcmp(e->self_target_field, "true"), -1,
                                       idxs, RB_MAX_ZONE);
    int outcome = rb_classify_selection(idxs, mn, count, is_all);
    if (outcome == 1 && can_skip && mn > 0) {
        if (is_all) {
            rb_zone_remove_at_indices(g, pl, zone_name, idxs, mn);
            for (int i = 0; i < mn && i < max; i++) out_ids[i] = cards[idxs[i]];
            return mn;
        }
        rb_move_prompt_card_selection(g, actor, zone_name, mn, can_skip, e);
        return -1;
    }
    if (outcome == 1) {
        rb_zone_remove_at_indices(g, pl, zone_name, idxs, mn);
        for (int i = 0; i < mn && i < max; i++) out_ids[i] = cards[idxs[i]];
        return mn;
    }
    if (outcome == 2) {
        rb_move_prompt_card_selection(g, actor, zone_name, count, can_skip, e);
        return -1;
    }
    if (can_skip && mn > 0) {
        rb_move_prompt_card_selection(g, actor, zone_name, mn, can_skip, e);
        return -1;
    }
    return 0;
}

/* Mirror AbilityResolver::resolve_from_zone — main zone-dispatch router.
   Routes to the zone-specific resolver based on effective_source.
   Returns count of cards resolved (writes to out_ids), or -1 if prompted. */
int rb_move_resolve_from_zone(GameState *g, int actor, const char *effective_source,
                               AbilityEffect *e, int use_p2, int count,
                               int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *dst = e->destination ? e->destination : "";

    if (!strcmp(effective_source, "deck") || !strcmp(effective_source, "deck_top"))
        return rb_move_resolve_from_deck(g, actor, e, use_p2, count, ctype, gn, out_ids, max);
    if (!strcmp(effective_source, "deck_bottom"))
        return rb_move_resolve_from_deck_bottom(g, actor, e, use_p2, count, ctype, gn, out_ids, max);
    if (!strcmp(effective_source, "energy_deck"))
        return rb_move_resolve_from_energy_deck(g, pl, count, out_ids, max);
    if (!strcmp(effective_source, "stage"))
        return rb_move_resolve_from_stage(g, actor, e, use_p2, count, out_ids, max);
    if (!strcmp(effective_source, "energy")) {
        if (!strcmp(dst, "energy_deck")) {
            int n = 0;
            while (n < count && P->energy.n > 0) {
                int card = P->energy.cards[--P->energy.n];
                P->energy_active = P->energy_active > 0 ? P->energy_active - 1 : 0;
                if (n < max) out_ids[n++] = card;
            }
            return n;
        }
        return rb_move_take_cards_from_standard_zone(g, actor, "energy", e, count, 0, 1, out_ids, max);
    }
    if (!strcmp(effective_source, "hand") || !strcmp(effective_source, "discard") ||
        !strcmp(effective_source, "live_card_zone") || !strcmp(effective_source, "success_live_zone"))
        return rb_move_take_cards_from_standard_zone(g, actor, effective_source, e, count, 0, 1, out_ids, max);
    if (!strcmp(effective_source, "looked_at"))
        return rb_move_resolve_source_looked_at(g, actor, e, use_p2, count, out_ids, max);
    if (!strcmp(effective_source, "selected_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_selected_cards && n < max; i++)
            out_ids[n++] = g->selected_cards[i];
        return n;
    }
    if (!strcmp(effective_source, "revealed_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_revealed && n < max; i++) {
            if (ctype && !rb_card_matches_type(g->revealed_cards[i], ctype)) continue;
            if (gn && !rb_card_matches_group_str(g->revealed_cards[i], gn)) continue;
            out_ids[n++] = g->revealed_cards[i];
        }
        return n;
    }
    if (!strcmp(effective_source, "under_member"))
        return rb_move_resolve_from_under_member(g, actor, e, use_p2, count, out_ids, max);
    return 0;
}

/* Ring-buffer record_card_movement — appends cid to g->recently_moved. */
static void mc_record_movement(GameState *g, int cid){
    if(cid < 0) return;
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
    else { for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++) g->recently_moved[i-1]=g->recently_moved[i]; g->recently_moved[RB_MAX_RECENTLY_MOVED-1]=cid; }
}

/* Mirror move_cards.rs::move_from_revealed — move cards from revealed_cards to
   dst. Removes from physical zones (waitroom, deck) to prevent duplication. */
int rb_move_from_revealed(GameState *g, int actor, const int *indices, int n_indices,
                          int (*validate_card)(int), const char *dst,
                          int *out_ids, int max) {
    (void)actor;
    if (!g || !indices || n_indices <= 0 || !out_ids || max <= 0) return 0;
    int sorted[RB_MAX_RECENTLY_MOVED];
    int ns = n_indices < RB_MAX_RECENTLY_MOVED ? n_indices : RB_MAX_RECENTLY_MOVED;
    for (int i = 0; i < ns; i++) sorted[i] = indices[i];
    for (int i = 0; i < ns - 1; i++)
        for (int j = i + 1; j < ns; j++)
            if (sorted[j] > sorted[i]) { int t = sorted[i]; sorted[i] = sorted[j]; sorted[j] = t; }
    int pl = g->active;
    int nm = 0;
    for (int k = 0; k < ns; k++) {
        int idx = sorted[k];
        if (idx < 0 || idx >= g->n_revealed) continue;
        int cid = g->revealed_cards[idx];
        for (int j = idx; j < g->n_revealed - 1; j++)
            g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        if (validate_card && !validate_card(cid)) continue;
        for (int p = 0; p < 2; p++) {
            RbPlayer *P = &g->p[p];
            int found = 0;
            for (int i = 0; i < P->discard.n; i++) {
                if (P->discard.cards[i] == cid) {
                    for (int j = i; j < P->discard.n - 1; j++) P->discard.cards[j] = P->discard.cards[j + 1];
                    P->discard.n--; found = 1; break;
                }
            }
            if (found) break;
            for (int i = 0; i < P->deck.n; i++) {
                if (P->deck.cards[i] == cid) {
                    for (int j = i; j < P->deck.n - 1; j++) P->deck.cards[j] = P->deck.cards[j + 1];
                    P->deck.n--; found = 1; break;
                }
            }
            if (found) break;
        }
        rb_place_card_in_zone(g, pl, cid, dst, -1);
        if (nm < max) out_ids[nm++] = cid;
    }
    return nm;
}

/* Mirror move_cards.rs::finalize_card_movement — post-move side effects. */
void rb_move_finalize_card_movement(GameState *g, int actor,
                                    const int *moved_cards, int n_moved,
                                    const char *destination, const char *source,
                                    const char *state_change, const char *target) {
    if (!g || !moved_cards || n_moved <= 0) return;
    (void)source;
    for (int i = 0; i < n_moved; i++)
        rb_mods_clear_card(&g->mods, moved_cards[i]);
    if (state_change && *state_change) {
        if (!strcmp(state_change, "wait")) {
            for (int i = 0; i < n_moved; i++)
                rb_mods_set_orientation(&g->mods, moved_cards[i], "wait");
        } else if (!strcmp(state_change, "active")) {
            for (int i = 0; i < n_moved; i++)
                rb_mods_set_orientation(&g->mods, moved_cards[i], "active");
            if (destination && !strcmp(destination, "energy")) {
                int pl = actor;
                if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
                int sum = (int)g->p[pl].energy_active + n_moved;
                g->p[pl].energy_active = sum > 32767 ? 32767 : sum;
            }
        }
    }
    for (int i = 0; i < n_moved; i++)
        mc_record_movement(g, moved_cards[i]);
    rb_recalc_constants(g);
    if (destination && !strcmp(destination, "stage")) {
        for (int i = 0; i < n_moved; i++)
            rb_move_fire_debut_side_effects(g, actor, moved_cards[i],
                                            target ? target : "self", NULL);
    }
}

/* Mirror move_cards.rs::fire_debut_side_effects — canonical debut processing. */
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id,
                                     const char *target, const char *source) {
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    if (card_id >= 0 && g->n_cards_appeared_this_turn < 64)
        g->cards_appeared_this_turn[g->n_cards_appeared_this_turn++] = card_id;
    (void)source;
    if (pl >= 0 && pl < 2)
        g->debut_count_this_turn[pl]++;
    rb_trigger_debut(g, pl, card_id);
    rb_trigger_auto_abilities(g, pl, "自動");
    rb_process_pending_auto_abilities(g);
}

/* Mirror AbilityResolver::handle_select_position — handle the player's answer
   to a SelectPosition choice. Places the card at the chosen stage slot. */
void rb_move_handle_select_position(GameState *g, int actor, const char *position,
                                     int card_id, const char *target,
                                     const char *source_zone, const char *state_change) {
    if (!g) return;
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];
    int pos_idx = -1;
    if (!strcmp(position, "left_side")) pos_idx = 0;
    else if (!strcmp(position, "center")) pos_idx = 1;
    else if (!strcmp(position, "right_side")) pos_idx = 2;
    int should_lock = (!source_zone || strcmp(source_zone, "stage") != 0);
    int placed = 0;
    if (pos_idx >= 0 && pos_idx < RB_STAGE_SIZE) {
        if (P->stage[pos_idx] == RB_EMPTY_SLOT) {
            P->stage[pos_idx] = card_id;
            if (should_lock) g->stage_arrived[pl][pos_idx] = 1;
            placed = 1;
        } else if (P->stage[pos_idx] != RB_EMPTY_SLOT) {
            rb_waitroom_add(P, P->stage[pos_idx]);
            P->stage[pos_idx] = card_id;
            if (should_lock) g->stage_arrived[pl][pos_idx] = 1;
            placed = 1;
        }
    }
    if (!placed) rb_hand_add(P, card_id);
    rb_mods_clear_card(&g->mods, card_id);
    mc_record_movement(g, card_id);
    if (state_change && !strcmp(state_change, "wait"))
        rb_mods_set_orientation(&g->mods, card_id, "wait");
    rb_move_fire_debut_side_effects(g, actor, card_id, target ? target : "self", NULL);
    rb_clear_pending_choice(g);
}

/* Mirror AbilityResolver::execute_stage_placement_choices — execute stage
   placement for multiple card IDs. For each card: removes from source zone,
   calls rb_move_place_card_with_stage_choice. If a sub-choice is created,
   defers remaining cards. Fires debut side effects on immediate placements.
   Returns count of cards placed (writes to out_ids). */
int rb_move_execute_stage_placement_choices(GameState *g, int actor,
                                            const int *card_ids, int n_ids,
                                            const char *src_zone,
                                            const char *dest,
                                            int vacated_area,
                                            const char *target,
                                            int *out_ids, int max) {
    if (!g || !card_ids || n_ids <= 0 || !out_ids) return 0;
    int nm = 0;
    for (int pos = 0; pos < n_ids; pos++) {
        int card_id = card_ids[pos];
        int pl = actor;
        if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
        rb_remove_card_from_zone(g, pl, card_id, src_zone);
        int placed = rb_move_place_card_with_stage_choice(g, actor, -1, target,
                                                           card_id, dest, vacated_area,
                                                           0, 1, NULL, -1, src_zone,
                                                           0, 0);
        if (placed == 1) {
            out_ids[nm++] = card_id;
        } else if (placed == 0) {
            out_ids[nm++] = card_id;
            rb_move_fire_debut_side_effects(g, actor, card_id, target ? target : "self", NULL);
        }
    }
    return nm;
}

/* Mirror AbilityResolver::place_energy_under_member_selected — place tapped
   energy cards under the activating member. Falls back to moved_cards/center/
   left/right when activating card not on stage. If target slot empty, energy
   goes to energy deck. */
void rb_move_place_energy_under_member_selected(GameState *g, int actor,
                                               const int *cids, int n_cids) {
    if (!g || !cids || n_cids <= 0) return;
    int activating = g->queue.resume_host;
    int pl = actor;
    RbPlayer *P = &g->p[pl];
    int target_index = -1;
    if (activating >= 0) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == activating && P->stage[i] != RB_EMPTY_SLOT)
                { target_index = i; break; }
    }
    if (target_index < 0) {
        for (int i = g->n_recently_moved - 1; i >= 0; i--) {
            for (int j = 0; j < RB_STAGE_SIZE; j++)
                if (P->stage[j] == g->recently_moved[i] && P->stage[j] != RB_EMPTY_SLOT)
                    { target_index = j; break; }
            if (target_index >= 0) break;
        }
    }
    if (target_index < 0) {
        if (P->stage[1] != RB_EMPTY_SLOT) target_index = 1;
        else if (P->stage[0] != RB_EMPTY_SLOT) target_index = 0;
        else if (P->stage[2] != RB_EMPTY_SLOT) target_index = 2;
    }
    if (target_index < 0 || P->stage[target_index] == RB_EMPTY_SLOT) {
        for (int i = 0; i < n_cids; i++)
            if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cids[i];
        return;
    }
    int area = rb_pos_to_area(NULL);
    if (target_index == 0) area = 0;
    else if (target_index == 1) area = 1;
    else area = 2;
    for (int i = 0; i < n_cids; i++)
        rb_stage_place_under_card(P, area, cids[i]);
}

/* Mirror AbilityResolver::execute_move_cards_both — handle target="both" by
   processing opponent first, then self. */
void rb_move_execute_move_cards_both(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    rb_effect_move_cards(g, actor, e);
}

/* Mirror AbilityResolver::prompt_deck_top_or_bottom — emit a SelectTarget
    choice that lets the player place `card_id` on the deck top or bottom.
    The choice is routed through RB_ROUTE_SELECT_TARGET so resume can place
    the card at the chosen position. */
void rb_move_prompt_deck_top_or_bottom(GameState *g, int actor, int card_id,
                                        const char *target, const char *source_zone,
                                        int allow_skip) {
    if (!g) return;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, allow_skip,
                   "deck_top_or_bottom");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
    /* Stash the card id and target in the queue entry so resume can place it. */
    if (g->queue.cur < RB_QUEUE_DEPTH) {
        g->queue.entries[g->queue.cur].card_id = card_id;
    }
    g->queue.resume_mode = 1;
    g->queue.resume_actor = actor;
    g->queue.resume_host = card_id;
    if (target && *target)
        strncpy(g->queue.pending.target, target, sizeof(g->queue.pending.target) - 1);
    if (source_zone && *source_zone)
        strncpy(g->queue.resume_draw_source, source_zone, sizeof(g->queue.resume_draw_source) - 1);
}

/* Mirror AbilityResolver::resolve_from_recently_moved — filter the recently_moved
   pool by card_type / group, remove each from its zone, run zone-exit cleanup.
   Returns the count of cards resolved (writes ids into out_ids, cap max). */
int rb_move_resolve_from_recently_moved(GameState *g, int use_p2,
                                        const char *card_type_filter,
                                        const char *group_name,
                                        int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : 0;
    RbPlayer *P = &g->p[pl];
    int n = 0;
    for (int i = 0; i < g->n_recently_moved && n < max; i++) {
        int cid = g->recently_moved[i];
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
        if (group_name && !rb_card_matches_group_str(cid, group_name)) continue;
        out_ids[n++] = cid;
    }
    for (int i = 0; i < n; i++) {
        int last_vacated = -1;
        remove_card_from_any_zone(P, &last_vacated, out_ids[i]);
        if (last_vacated >= 0)
            g->baton_last_vacated_area[pl] = last_vacated;
    }
    if (n > 0) rb_recalc_constants(g);
    return n;
}

/* Mirror describe::zone_label_inner(zone, false) — English display label for a
   zone wire name. Faithful port of the Rust match table (describe.rs:47-68). */
static const char *move_zone_label(const char *zone) {
    if (!zone) return "unknown";
    if (!strcmp(zone, "hand")) return "hand";
    if (!strcmp(zone, "discard")) return "the waiting room";
    if (!strcmp(zone, "deck")) return "deck";
    if (!strcmp(zone, "deck_top")) return "top of deck";
    if (!strcmp(zone, "deck_bottom")) return "bottom of deck";
    if (!strcmp(zone, "stage")) return "stage";
    if (!strcmp(zone, "energy")) return "energy";
    if (!strcmp(zone, "energy_deck")) return "energy deck";
    if (!strcmp(zone, "energy_zone")) return "energy zone";
    if (!strcmp(zone, "waitroom")) return "wait room";
    if (!strcmp(zone, "success_zone")) return "success zone";
    if (!strcmp(zone, "live_card_zone")) return "live card zone";
    if (!strcmp(zone, "under_member")) return "under this member";
    if (!strcmp(zone, "revealed_cards")) return "revealed cards";
    if (!strcmp(zone, "those_cards")) return "those cards";
    if (!strcmp(zone, "all_selected")) return "selected cards";
    return zone;
}

/* Mirror AbilityResolver::prompt_card_selection — emit a SELECT_CARD choice
    with zone, count, filters, and descriptions. */
void rb_move_prompt_card_selection(GameState *g, int actor, const char *zone,
                                    int count, int can_skip, AbilityEffect *e) {
    const char *zlabel = move_zone_label(zone);
    const char *any_num_str = cmf_extra(e, "any_number");
    int any_number = any_num_str && (!strcmp(any_num_str, "true") || !strcmp(any_num_str, "1"));
    char desc[128];
    if (any_number)
        snprintf(desc, sizeof(desc), "Select any number of card(s) from %s", zlabel);
    else
        snprintf(desc, sizeof(desc), "Select %d card(s) from %s", count, zlabel);
    const char *card_type = cmf_extra(e, "card_type");
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, card_type, count, can_skip, NULL);
    rb_choice_set_description(&g->queue.pending, desc);
    const char *group = cmf_extra(e, "group_names");
    if (group)
        strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group) - 1);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
}

/* Mirror AbilityResolver::place_card_with_stage_choice — place `card_id` at
   `destination`. When destination is Stage with multiple available slots,
   emit a SelectPosition choice and return 1 (caller yields). Returns 0 on
   immediate placement, -1 on failure. */
int rb_move_place_card_with_stage_choice(
    GameState *g, int actor, int host_cid, const char *player_target,
    int card_id, const char *destination, int vacated_area,
    int is_max, int count, const char *state_change,
    int deck_position, const char *source_zone,
    int allow_occupied_stage, int under_self) {
    int pl = actor;
    if (player_target && *player_target) {
        int t = rb_resolve_target_player(g, player_target);
        if (t >= 0) pl = t;
    }
    RbPlayer *P = &g->p[pl];
    if (!strcmp(destination, "empty_area") || !strcmp(destination, "stage")) {
        int empty_slots[RB_STAGE_SIZE], n_empty = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == RB_EMPTY_SLOT) empty_slots[n_empty++] = i;
        if (is_max && n_empty < count) return -1;
        int avail[RB_STAGE_SIZE], n_avail = 0;
        if (allow_occupied_stage) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] == RB_EMPTY_SLOT || !g->stage_arrived[pl][i])
                    avail[n_avail++] = i;
        } else {
            for (int i = 0; i < n_empty; i++) avail[i] = empty_slots[i];
            n_avail = n_empty;
        }
        if (n_avail == 0) return -1;
        if (n_avail > 1) {
            if (vacated_area >= 0 && vacated_area < RB_STAGE_SIZE && P->stage[vacated_area] == RB_EMPTY_SLOT) {
                P->stage[vacated_area] = card_id;
                if (strcmp(source_zone, "stage") != 0) g->stage_arrived[pl][vacated_area] = 1;
                return 0;
            }
            char pos_str[128]; pos_str[0] = '\0';
            for (int i = 0; i < n_avail; i++) {
                if (i > 0) strcat(pos_str, ",");
                switch (avail[i]) {
                    case 0: strcat(pos_str, "left_side"); break;
                    case 1: strcat(pos_str, "center"); break;
                    default: strcat(pos_str, "right_side"); break;
                }
            }
            char card_name[64]; card_name[0] = '\0';
            Card c;
            if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
                if (c.name) strncpy(card_name, c.name, sizeof(card_name) - 1);
                rb_free_card(&c);
            }
            if (card_name[0] == '\0') strcpy(card_name, "card");
            char desc[128];
            snprintf(desc, sizeof(desc), "Choose position for %s", card_name);
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 0, pos_str);
            rb_choice_set_description(&g->queue.pending, desc);
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
            g->queue.resume_mode = 1;
            g->queue.resume_actor = actor;
            g->queue.resume_host = card_id;
            if (state_change && *state_change)
                strncpy(g->queue.resume_draw_dest, state_change, sizeof(g->queue.resume_draw_dest) - 1);
            if (source_zone && *source_zone)
                strncpy(g->queue.resume_draw_source, source_zone, sizeof(g->queue.resume_draw_source) - 1);
            return 1;
        } else {
            int slot = avail[0];
            if (P->stage[slot] != RB_EMPTY_SLOT) rb_waitroom_add(P, P->stage[slot]);
            P->stage[slot] = card_id;
            if (strcmp(source_zone, "stage") != 0) g->stage_arrived[pl][slot] = 1;
            return 0;
        }
    }
    int pos_to_use = -1;
    if (!strcmp(destination, "under_member")) {
        int from_self_displacement = (!strcmp(source_zone, "stage") || source_zone[0] == '\0');
        int n_members = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n_members++;
        if (!from_self_displacement && !under_self && n_members > 1 && vacated_area < 0) {
            char card_name[64]; card_name[0] = '\0';
            Card c;
            if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
                if (c.name) strncpy(card_name, c.name, sizeof(card_name) - 1);
                rb_free_card(&c);
            }
            if (card_name[0] == '\0') strcpy(card_name, "card");
            char desc[128];
            snprintf(desc, sizeof(desc), "Choose a member to place %s under", card_name);
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", NULL, 1, 0, NULL);
            rb_choice_set_description(&g->queue.pending, desc);
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
            g->queue.resume_mode = 1;
            g->queue.resume_actor = actor;
            g->queue.resume_host = card_id;
            if (player_target && *player_target)
                strncpy(g->queue.pending.target, player_target, sizeof(g->queue.pending.target) - 1);
            if (state_change && *state_change)
                strncpy(g->queue.resume_draw_dest, state_change, sizeof(g->queue.resume_draw_dest) - 1);
            if (source_zone && *source_zone)
                strncpy(g->queue.resume_draw_source, source_zone, sizeof(g->queue.resume_draw_source) - 1);
            return 1;
        }
        pos_to_use = -1;
        if (host_cid >= 0) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] == host_cid) { pos_to_use = i; break; }
        }
        if (pos_to_use < 0 && vacated_area >= 0) pos_to_use = vacated_area;
        if (pos_to_use < 0) {
            for (int i = g->n_recently_moved - 1; i >= 0; i--) {
                for (int j = 0; j < RB_STAGE_SIZE; j++)
                    if (P->stage[j] == g->recently_moved[i]) { pos_to_use = j; break; }
                if (pos_to_use >= 0) break;
            }
        }
    } else if (!strcmp(destination, "deck") || !strcmp(destination, "deck_top")) {
        pos_to_use = (deck_position >= 0) ? deck_position : vacated_area;
    } else {
        pos_to_use = vacated_area;
    }
    if (!strcmp(destination, "deck") || !strcmp(destination, "deck_top")) {
        int idx = pos_to_use >= 0 ? pos_to_use : 0;
        if (idx > P->deck.n) idx = P->deck.n;
        if (P->deck.n < RB_MAX_ZONE) {
            for (int k = P->deck.n; k > idx; k--) P->deck.cards[k] = P->deck.cards[k - 1];
            P->deck.cards[idx] = card_id;
            P->deck.n++;
        }
    } else if (!strcmp(destination, "deck_bottom")) {
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = card_id;
    } else if (!strcmp(destination, "under_member")) {
        if (pos_to_use >= 0 && pos_to_use < RB_STAGE_SIZE)
            rb_stage_place_under_card(P, pos_to_use, card_id);
    } else {
        rb_place_card_in_zone(g, pl, card_id, destination, pos_to_use);
    }
    return 0;
}

/* Mirror AbilityResolver::resolve_from_energy_deck — draw up to `count` cards
   from the energy deck. */
int rb_move_resolve_from_energy_deck(GameState *g, int pl, int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    RbPlayer *P = &g->p[pl];
    int n = 0;
    while (n < count && P->energy_deck.n > 0) {
        int card = P->energy_deck.cards[0];
        for (int i = 0; i < P->energy_deck.n - 1; i++) P->energy_deck.cards[i] = P->energy_deck.cards[i + 1];
        P->energy_deck.n--;
        if (n < max) out_ids[n++] = card;
    }
    return n;
}

/* Mirror AbilityResolver::resolve_from_stage — resolve source="stage" cards.
   Takes member cards from the stage matching the filter. */
int rb_move_resolve_from_stage(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    int n = 0;
    for (int area = 0; area < RB_STAGE_SIZE && n < count; area++) {
        int cid = P->stage[area];
        if (cid < 0) continue;
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        out_ids[n++] = cid;
        P->stage[area] = RB_EMPTY_SLOT;
        P->stage_wait[area] = 0;
        g->baton_last_vacated_area[pl] = area;
    }
    return n;
}

/* Mirror AbilityResolver::resolve_from_under_member — resolve source="under_member"
   by draining energy cards from under stage members. */
int rb_move_resolve_from_under_member(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                       int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    return rb_drain_under_cards_to_energy_zone(g, "energy_deck", -1);
}

/* Mirror AbilityResolver::resolve_from_deck_bottom — draw from the bottom of the deck. */
int rb_move_resolve_from_deck_bottom(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, const char *card_type_filter,
                                      const char *group_name, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    int n = 0;
    while (n < count && P->deck.n > 0) {
        int card = P->deck.cards[P->deck.n - 1];
        P->deck.n--;
        if (card_type_filter && !rb_card_matches_type(card, card_type_filter)) continue;
        if (group_name && !rb_card_matches_group_str(card, group_name)) continue;
        if (n < max) out_ids[n++] = card;
    }
    return n;
}

/* Mirror AbilityResolver::resolve_source_looked_at — filter the looked_at pool. */
int rb_move_resolve_source_looked_at(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : 0;
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    int pool[RB_MAX_ZONE], pn = rb_looked_at_pool(pl, pool, RB_MAX_ZONE);
    int n = 0;
    for (int i = 0; i < pn && n < count; i++) {
        int cid = pool[i];
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        out_ids[n++] = cid;
        rb_look_remove(pl, cid);
    }
    return n;
}

/* ═══════════════════════════════════════════════════════════════════════════
   The following 3 functions mirror the unmatched Rust ability methods for
   select_cards answer handling and energy zone selection.
   ═══════════════════════════════════════════════════════════════════════════ */

/* Mirror AbilityResolver::execute_selected_cards_from_zone — main handler for
   the select_cards answer. Moves the selected cards from their source zone
   to the destination, applying filters and stage placement as needed. */
void rb_move_execute_selected_cards_from_zone(
    GameState *g, int actor, const char *zone, const int *indices, int n_indices,
    const char *card_type_filter, int cost_limit, const char *cost_limit_op,
    const char *group, const char **characters, int n_characters,
    const char *target_player_id) {
    if (!g || !zone || !indices || n_indices <= 0) return;

    /* Resolve destination from entry effect, falling back to discard. */
    const char *destination = rb_entry_destination(g);
    if (!destination) destination = "discard";

    /* Resolve target player. */
    const char *target = target_player_id ? target_player_id : "self";
    int pl = rb_resolve_target_player(g, target);
    if (pl < 0) pl = actor;

    /* Filter indices to only include cards that match required filters. */
    int filtered[RB_MAX_RECENTLY_MOVED];
    int nf = 0;
    int cards[RB_MAX_ZONE];
    int nc = rb_zone_cards(g, pl, zone, cards, RB_MAX_ZONE);
    for (int i = 0; i < n_indices && nf < RB_MAX_RECENTLY_MOVED; i++) {
        int idx = indices[i];
        if (idx < 0 || idx >= nc) continue;
        int cid = cards[idx];
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
        if (cost_limit >= 0 && !rb_card_matches_cost_limit(cid, cost_limit, cost_limit_op ? cost_limit_op : "<=")) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        if (characters && n_characters > 0 && !rb_card_matches_characters(cid, characters, n_characters)) continue;
        filtered[nf++] = idx;
    }

    /* Resolve indices to card IDs. */
    int card_ids[RB_MAX_RECENTLY_MOVED];
    int n_ids = rb_resolve_indices_to_ids(g, pl, zone, filtered, nf, card_ids);
    if (n_ids <= 0) return;

    /* Determine if destination is stage. */
    int dest_is_stage = !strcmp(destination, "stage") || !strcmp(destination, "empty_area") || !strcmp(destination, "same_area");
    int dest_is_deck_top_or_bottom = !strcmp(destination, "deck_top_or_bottom");

    int moved[RB_MAX_RECENTLY_MOVED];
    int nm = 0;

    if (dest_is_stage) {
        /* Route through stage placement. */
        int out_ids[RB_MAX_RECENTLY_MOVED];
        nm = rb_move_execute_stage_placement_choices(g, actor, card_ids, n_ids,
                                                      zone, destination, -1,
                                                      target, out_ids, RB_MAX_RECENTLY_MOVED);
    } else if (dest_is_deck_top_or_bottom) {
        /* Prompt for deck top or bottom placement. */
        if (n_ids > 0) {
            rb_move_prompt_deck_top_or_bottom(g, actor, card_ids[0], target, zone, 0);
            return;
        }
    } else {
        /* Standard zone-to-zone move. */
        /* Remove cards from source zone. */
        rb_zone_remove_at_indices(g, pl, zone, filtered, nf);
        /* Place cards in destination. */
        for (int i = 0; i < n_ids; i++) {
            rb_place_card_in_zone(g, pl, card_ids[i], destination, -1);
            moved[nm++] = card_ids[i];
        }
    }

    /* Post-move: clear mods, record in selected_cards, apply state_change. */
    for (int i = 0; i < nm; i++) {
        rb_mods_clear_card(&g->mods, moved[i]);
        if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = moved[i];
        mc_record_movement(g, moved[i]);
    }
    rb_recalc_constants(g);
    rb_clear_pending_choice(g);
}

/* Mirror AbilityResolver::handle_select_cards_looked_at — handle the player's
   answer to a looked_at card selection. Moves selected cards to destination,
   routes remaining cards to discard or deck bottom. */
void rb_move_handle_select_cards_looked_at(
    GameState *g, int actor, const int *indices, int n_indices,
    const char *ctx_destination, int ctx_discard_remaining) {
    if (!g || !indices || n_indices <= 0) return;

    int pl = actor;

    /* Resolve destination. */
    const char *destination = ctx_destination ? ctx_destination : "hand";
    int discard_remaining = ctx_discard_remaining >= 0 ? ctx_discard_remaining : 1;

    /* Get the looked_at pool. */
    int looked_at[RB_MAX_ZONE];
    int n_looked = rb_looked_at_pool(pl, looked_at, RB_MAX_ZONE);
    if (n_looked <= 0) return;

    /* Sort indices in reverse order for safe removal. */
    int sorted_idx[RB_MAX_RECENTLY_MOVED];
    int ns = n_indices < RB_MAX_RECENTLY_MOVED ? n_indices : RB_MAX_RECENTLY_MOVED;
    for (int i = 0; i < ns; i++) sorted_idx[i] = indices[i];
    /* Simple reverse sort (bubble for small arrays). */
    for (int i = 0; i < ns - 1; i++)
        for (int j = i + 1; j < ns; j++)
            if (sorted_idx[j] > sorted_idx[i]) { int t = sorted_idx[i]; sorted_idx[i] = sorted_idx[j]; sorted_idx[j] = t; }

    /* Remove selected cards from looked_at pool (in reverse order). */
    int selected[RB_MAX_RECENTLY_MOVED];
    int nsel = 0;
    for (int i = 0; i < ns; i++) {
        int idx = sorted_idx[i];
        if (idx >= 0 && idx < n_looked) {
            selected[nsel++] = looked_at[idx];
            rb_look_remove(pl, looked_at[idx]);
        }
    }

    /* Remaining cards in looked_at pool. */
    int remaining[RB_MAX_RECENTLY_MOVED];
    int nrem = rb_looked_at_pool(pl, remaining, RB_MAX_RECENTLY_MOVED);

    /* Place selected cards in destination. */
    for (int i = 0; i < nsel; i++) {
        rb_place_card_in_zone(g, pl, selected[i], destination, -1);
        if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = selected[i];
        mc_record_movement(g, selected[i]);
    }

    /* Route remaining cards. */
    const char *rem_dest;
    if (discard_remaining) {
        rem_dest = "discard";
    } else {
        rem_dest = "deck_bottom";
    }
    for (int i = 0; i < nrem; i++) {
        rb_place_card_in_zone(g, pl, remaining[i], rem_dest, -1);
    }

    /* Clear the looked_at pool. */
    for (int i = 0; i < nrem; i++)
        rb_look_remove(pl, remaining[i]);
    rb_clear_pending_choice(g);
    rb_recalc_constants(g);
}

/* Mirror AbilityResolver::handle_energy_zone_selection — handle the player's
   answer to an energy zone selection. Either moves selected energy cards to
   a destination (e.g. under_member) or marks them as wait. */
void rb_move_handle_energy_zone_selection(
    GameState *g, int actor, const int *indices, int n_indices,
    int count, const char *destination, int (*validate_card)(int)) {
    if (!g || !indices || n_indices <= 0) return;

    int pl = actor;
    RbPlayer *P = &g->p[pl];

    if (destination && *destination) {
        /* Remove selected cards from energy zone (reverse order for safe removal). */
        int sorted_idx[RB_MAX_RECENTLY_MOVED];
        int ns = n_indices < RB_MAX_RECENTLY_MOVED ? n_indices : RB_MAX_RECENTLY_MOVED;
        for (int i = 0; i < ns; i++) sorted_idx[i] = indices[i];
        for (int i = 0; i < ns - 1; i++)
            for (int j = i + 1; j < ns; j++)
                if (sorted_idx[j] > sorted_idx[i]) { int t = sorted_idx[i]; sorted_idx[i] = sorted_idx[j]; sorted_idx[j] = t; }

        int cids[RB_MAX_RECENTLY_MOVED];
        int nc = 0;
        for (int i = 0; i < ns; i++) {
            int idx = sorted_idx[i];
            if (idx >= 0 && idx < P->energy.n) {
                int cid = P->energy.cards[idx];
                if (validate_card && !validate_card(cid)) continue;
                cids[nc++] = cid;
                for (int k = idx; k < P->energy.n - 1; k++)
                    P->energy.cards[k] = P->energy.cards[k + 1];
                P->energy.n--;
            }
        }

        /* Decrement active energy count. */
        P->energy_active = P->energy_active > nc ? P->energy_active - nc : 0;

        if (!strcmp(destination, "under_member")) {
            if (nc > 0) {
                rb_move_place_energy_under_member_selected(g, actor, cids, nc);
            }
        } else {
            for (int i = 0; i < nc; i++) {
                rb_place_card_in_zone(g, pl, cids[i], destination, -1);
            }
        }

        /* Clear mods and record movement. */
        for (int i = 0; i < nc; i++) {
            rb_mods_clear_card(&g->mods, cids[i]);
            mc_record_movement(g, cids[i]);
        }
    } else {
        /* No destination: mark energy cards as wait. */
        rb_effect_selected_energy_zone_cards(g, actor, indices, n_indices);
    }
    rb_clear_pending_choice(g);
}
