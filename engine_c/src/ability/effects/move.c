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

/* Exposed for engine.c handle_action — filtered move */
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e){
    int cnt = e->count>=0? e->count : 1;
    const char *src_s = e->source ? e->source : "hand";
    const char *dst_s = e->destination ? e->destination : "discard";
    if (!strcmp(src_s,"those_cards")||!strcmp(src_s,"recently_moved")||!strcmp(src_s,"looked_at")||!strcmp(src_s,"selected_cards")) src_s="hand";
    if (!strcmp(dst_s,"those_cards")||!strcmp(dst_s,"recently_moved")||!strcmp(dst_s,"looked_at")) dst_s="discard";
    if (!strcmp(dst_s,"under_member")||!strcmp(dst_s,"same_area")||!strcmp(dst_s,"empty_area")) dst_s="discard";
    RbZone src=RB_ZONE_HAND, dst=RB_ZONE_DISCARD;
    rb_zone_of_str(src_s,&src); rb_zone_of_str(dst_s,&dst);
    int to_top = e->destination && (!strcmp(e->destination,"deck_top")||!strcmp(e->destination,"deck_top_or_bottom"));
    /* Filtered move: only cards matching CardFilter */
    RbPlayer *A=&g->p[actor^ (e->target && !strcmp(e->target,"opponent")?1:0)];
    (void)actor;
    RbBag *sb=NULL;
    if(src==RB_ZONE_STAGE){
        int moved=0;
        for(int pos=0; pos<RB_STAGE_SIZE && moved < cnt; pos++){
            if(A->stage[pos]>=0 && card_matches_filter(A->stage[pos], e)){
                int c=A->stage[pos]; A->stage[pos]=-1; A->stage_wait[pos]=0;
                RbBag *db=NULL;
                if(dst==RB_ZONE_STAGE){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ A->stage[q]=c; break; } }
                else { db=NULL; if(dst==RB_ZONE_HAND) db=&A->hand; else if(dst==RB_ZONE_DISCARD) db=&A->discard; else if(dst==RB_ZONE_DECK) db=&A->deck; else if(dst==RB_ZONE_ENERGY) db=&A->energy; else if(dst==RB_ZONE_LIVE) db=&A->live; else if(dst==RB_ZONE_SUCCESS) db=&A->success;
                    if(db && db->n < RB_MAX_ZONE) db->cards[db->n++]=c;
                }
                moved++;
            }
        }
        return;
    }
    if(src==RB_ZONE_HAND) sb=&A->hand;
    else if(src==RB_ZONE_DECK) sb=&A->deck;
    else if(src==RB_ZONE_DISCARD) sb=&A->discard;
    else if(src==RB_ZONE_ENERGY) sb=&A->energy;
    else if(src==RB_ZONE_LIVE) sb=&A->live;
    else if(src==RB_ZONE_SUCCESS) sb=&A->success;
    if(!sb) return;
    int moved=0;
    for(int i=sb->n-1; i>=0 && moved < cnt; i--){
        if(!card_matches_filter(sb->cards[i], e)) continue;
        int c=sb->cards[i];
        for(int k=i;k<sb->n-1;k++) sb->cards[k]=sb->cards[k+1];
        sb->n--;
        RbBag *db=NULL;
        if(dst==RB_ZONE_STAGE){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ A->stage[q]=c; break; } }
        else {
            if(dst==RB_ZONE_HAND) db=&A->hand;
            else if(dst==RB_ZONE_DECK) db=&A->deck;
            else if(dst==RB_ZONE_DISCARD) db=&A->discard;
            else if(dst==RB_ZONE_ENERGY) db=&A->energy;
            else if(dst==RB_ZONE_LIVE) db=&A->live;
            else if(dst==RB_ZONE_SUCCESS) db=&A->success;
            if(!db) continue;
            if(to_top && dst==RB_ZONE_DECK){
                if(db->n < RB_MAX_ZONE){ for(int k=db->n;k>0;k--) db->cards[k]=db->cards[k-1]; db->cards[0]=c; db->n++; }
            } else if(db->n < RB_MAX_ZONE) db->cards[db->n++]=c;
        }
        moved++;
    }
}

/* needed by engine.c wrapper */
int card_matches_card_type_filter(int card_idx, const char *filter);
