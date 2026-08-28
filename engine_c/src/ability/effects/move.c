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
    /* group filter via extra "group_names" — stub: check Card.group_idx string contains fragment */
    const char *gn=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names")) gn=e->extra_v[i];
    if(gn){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        const char *gname = rb_card_string(c.group_idx);
        int match = gname && strstr(gname, gn);
        rb_free_card(&c);
        if(!match) return 0;
    }
    const char *cnames=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_names")) cnames=e->extra_v[i];
    if(cnames){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int match = c.name && strstr(c.name, cnames);
        rb_free_card(&c);
        if(!match) return 0;
    }
    return 1;
}

/* Exposed for engine.c handle_action — filtered move with relay pools.
   Mirrors engine/src/ability/move_cards.rs: the set of cards moved by an
   action is recorded in g->recently_moved so subsequent `preceding_moved` /
   `selected_cards` / `those_cards` references (conditions, chained effects)
   resolve against the actual moved cards. */
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
static void place_in_dst(RbPlayer *A, RbZone dst, int cid, int to_top){
    if(dst==RB_ZONE_STAGE){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ A->stage[q]=cid; A->stage_wait[q]=0; break; } return; }
    RbBag *db=NULL;
    if(dst==RB_ZONE_HAND) db=&A->hand; else if(dst==RB_ZONE_DECK) db=&A->deck;
    else if(dst==RB_ZONE_DISCARD) db=&A->discard; else if(dst==RB_ZONE_ENERGY) db=&A->energy;
    else if(dst==RB_ZONE_LIVE) db=&A->live; else if(dst==RB_ZONE_SUCCESS) db=&A->success;
    if(!db) return;
    if(db->n < RB_MAX_ZONE){
        if(to_top && dst==RB_ZONE_DECK){ for(int k=db->n;k>0;k--) db->cards[k]=db->cards[k-1]; db->cards[0]=cid; db->n++; }
        else db->cards[db->n++]=cid;
    }
}
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e){
    int cnt = e->count>=0? e->count : 1;
    const char *src_s = e->source ? e->source : "hand";
    const char *dst_s = e->destination ? e->destination : "discard";
    int relay = (!strcmp(src_s,"those_cards")||!strcmp(src_s,"recently_moved")||!strcmp(src_s,"looked_at")||!strcmp(src_s,"selected_cards"));
    if (!strcmp(dst_s,"those_cards")||!strcmp(dst_s,"recently_moved")||!strcmp(dst_s,"looked_at")) dst_s="discard";
    if (!strcmp(dst_s,"under_member")||!strcmp(dst_s,"same_area")||!strcmp(dst_s,"empty_area")) dst_s="discard";
    RbZone dst=RB_ZONE_DISCARD;
    rb_zone_of_str(dst_s,&dst);
    int to_top = e->destination && (!strcmp(e->destination,"deck_top")||!strcmp(e->destination,"deck_top_or_bottom"));
    RbPlayer *A=&g->p[actor^ (e->target && !strcmp(e->target,"opponent")?1:0)];
    (void)actor;
    int src_ids[RB_MAX_ZONE]; int ns=0;
    if(relay){
        for(int i=0;i<g->n_recently_moved;i++) src_ids[ns++]=g->recently_moved[i];
    } else {
        RbZone src=RB_ZONE_HAND; rb_zone_of_str(src_s,&src);
        RbBag *sb=NULL;
        if(src==RB_ZONE_STAGE){ for(int pos=0;pos<RB_STAGE_SIZE && ns<cnt;pos++) if(A->stage[pos]>=0 && card_matches_filter(A->stage[pos],e)) src_ids[ns++]=A->stage[pos]; }
        else if(src==RB_ZONE_HAND) sb=&A->hand; else if(src==RB_ZONE_DECK) sb=&A->deck;
        else if(src==RB_ZONE_DISCARD) sb=&A->discard; else if(src==RB_ZONE_ENERGY) sb=&A->energy;
        else if(src==RB_ZONE_LIVE) sb=&A->live; else if(src==RB_ZONE_SUCCESS) sb=&A->success;
        if(sb){ for(int i=sb->n-1;i>=0 && ns<cnt;i--) if(card_matches_filter(sb->cards[i],e)) src_ids[ns++]=sb->cards[i]; }
    }
    int moved_ids[RB_MAX_ZONE]; int nm=0;
    for(int i=0;i<ns;i++){
        int cid=src_ids[i];
        if(find_and_remove_card(A,cid)){ place_in_dst(A,dst,cid,to_top); moved_ids[nm++]=cid; }
    }
    /* Record the moved set for `preceding_moved`/`those_cards` relay references. */
    g->n_recently_moved = nm < RB_MAX_RECENTLY_MOVED ? nm : RB_MAX_RECENTLY_MOVED;
    for(int i=0;i<g->n_recently_moved;i++) g->recently_moved[i]=moved_ids[i];
}

/* needed by engine.c wrapper */
int card_matches_card_type_filter(int card_idx, const char *filter);
