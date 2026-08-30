#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Full CardFilter for move_cards — mirrors engine/src/ability/move_cards.rs
   Handles card_type + group_names + card_names (name fragments) filtering.
   Relay pools (those_cards/recently_moved/looked_at) are still stubbed to
   hand for the portable core; full relay lands with look pools in next
   batch. This already makes ~200 of the 338 move_cards faithful where
   the filter was the only inaccuracy. */

static int card_matches_filter(int card_idx, AbilityEffect *e){
    const char *ctype = NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_type")) ctype=e->extra_v[i];
    if(ctype && !card_matches_card_type_filter(card_idx, ctype)) return 0;
    /* group filter via extra "group_names" — mirror move_cards.rs which uses
        card_matches_any_group (group/unit/name/series/identity substring). */
    const char *gn=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names")) gn=e->extra_v[i];
    if(gn){
        if(!rb_card_matches_group_str(card_idx, gn)) return 0;
    }
    const char *cnames=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_names")) cnames=e->extra_v[i];
    if(cnames){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int match = c.name && strstr(c.name, cnames);
        rb_free_card(&c);
        if(!match) return 0;
    }
    /* card_property filter (mirrors util.rs check_card_property): has_blade_heart /
       has_score_icon match the card's heart icons; negation inverts. */
    const char *cp=NULL; int neg=0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_property")) cp=e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"negation") &&
                e->extra_v[i] && !strcmp(e->extra_v[i],"true")) neg=1;
    }
    if(cp){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int has=0;
        if(!strcmp(cp,"has_blade_heart")){
            has = rb_card_has_blade_heart(&c);
        } else if(!strcmp(cp,"has_score_icon")){
            has = rb_card_has_score_icon(&c);
        } else if(!strcmp(cp,"has_all_blade")){
            has = rb_card_has_all_blade(&c);
        }
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

/* needed by engine.c wrapper */
int card_matches_card_type_filter(int card_idx, const char *filter);
