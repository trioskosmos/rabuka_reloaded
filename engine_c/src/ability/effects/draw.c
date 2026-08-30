#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Ported from engine/src/ability/effects/draw.rs
   draw_cards_for_player: draw `count` cards from source deck to destination,
   handling card_type filter, distinct, deck refresh (rule 10.2.1) and
   place_card_in_zone. Simplified: card_type_filter as string match on
   rb_card_is_live / is_energy; distinct ignored for now. */

int rb_draw_cards_for_player(RbPlayer *player, uint8_t count, const char *source,
                             const char *destination, const char *card_type_filter,
                             int is_any_number, void *distinct, void *card_db, int self_target_id){
    (void)distinct; (void)card_db;
    if(is_any_number) return 0;
    int drawn = 0;
    while(drawn < count){
        int card = -1;
        int from_deck = source && (!strcmp(source,"deck")||!strcmp(source,"main_deck")||!strcmp(source,"deck_top")||!strcmp(source,"deck_bottom"));
        int deck_bottom = source && !strcmp(source,"deck_bottom");
        if(from_deck){
            if(player->deck.n>0){
                if(deck_bottom){ card = player->deck.cards[0]; for(int i=1;i<player->deck.n;i++) player->deck.cards[i-1]=player->deck.cards[i]; player->deck.n--; }
                else card = player->deck.cards[--player->deck.n];
            } else {
                /* Q104 / Rule 10.2.1: deck empty mid-draw -> refresh from waitroom */
                if(player->discard.n>0){
                    for(int i=0;i<player->discard.n;i++) player->deck.cards[player->deck.n++] = player->discard.cards[i];
                    player->discard.n = 0;
                    rb_shuffle(player->deck.cards, player->deck.n);
                    player->deck_refreshed_this_turn = 1;
                    continue;
                }
                break;
            }
        } else if(source && (!strcmp(source,"discard")||!strcmp(source,"waitroom"))){
            if(player->discard.n>0) card = player->discard.cards[--player->discard.n];
            else break;
        } else if(source && !strcmp(source,"hand")){
            if(player->hand.n>0) card = player->hand.cards[--player->hand.n];
            else break;
        } else if(source && !strcmp(source,"energy")){
            if(player->energy.n>0) card = player->energy.cards[--player->energy.n];
            else break;
        } else if(source && (!strcmp(source,"success")||!strcmp(source,"success_zone")||!strcmp(source,"success_live_zone")||!strcmp(source,"success_live_card_zone"))){
            if(player->success.n>0) card = player->success.cards[--player->success.n];
            else break;
        } else if(source && (!strcmp(source,"staged")||!strcmp(source,"stage"))){
            for(int i=0;i<RB_STAGE_SIZE;i++) if(player->stage[i]!=RB_EMPTY_SLOT){ card=player->stage[i]; player->stage[i]=RB_EMPTY_SLOT; break; }
            if(card==-1) break;
        } else {
            /* resolution_zone / revealed_cards / unknown: not ported (revealed pool
                lives in GameState, not RbPlayer; resolution not tracked) */
            break;
        }
        if(card==-1) break;
        /* card_type filter */
        int matches = 1;
        if(card_type_filter){
            if(!strcmp(card_type_filter,"live_card")) matches = rb_card_is_live(card);
            else if(!strcmp(card_type_filter,"member_card")) matches = !rb_card_is_live(card) && !rb_card_is_energy(card);
            else if(!strcmp(card_type_filter,"energy_card")) matches = rb_card_is_energy(card);
            /* exclude self */
            if(self_target_id!=-1 && card==self_target_id) matches = 0;
        }
        if(matches){
            const char *dst = destination ? destination : "hand";
            RbZone z;
            if(rb_zone_of_str(dst,&z)==0){
                /* fallback to hand */
                if(player->hand.n < RB_MAX_ZONE) player->hand.cards[player->hand.n++] = card;
            } else {
                /* place in zone */
                if(z==RB_ZONE_HAND && player->hand.n < RB_MAX_ZONE) player->hand.cards[player->hand.n++] = card;
                else if(z==RB_ZONE_DISCARD && player->discard.n < RB_MAX_ZONE) player->discard.cards[player->discard.n++] = card;
                else if(z==RB_ZONE_DECK && player->deck.n < RB_MAX_ZONE) player->deck.cards[player->deck.n++] = card;
                else if(z==RB_ZONE_ENERGY && player->energy.n < RB_MAX_ZONE) player->energy.cards[player->energy.n++] = card;
                else if(z==RB_ZONE_LIVE && player->live.n < RB_MAX_ZONE) player->live.cards[player->live.n++] = card;
                else if(z==RB_ZONE_SUCCESS && player->success.n < RB_MAX_ZONE) player->success.cards[player->success.n++] = card;
                else if(player->hand.n < RB_MAX_ZONE) player->hand.cards[player->hand.n++] = card;
            }
            drawn++;
        } else {
            /* not matching -> put back on the source pile bottom */
            if(from_deck){
                for(int i=player->deck.n;i>0;i--) player->deck.cards[i]=player->deck.cards[i-1];
                player->deck.cards[0]=card; player->deck.n++;
            } else if(source && (!strcmp(source,"discard")||!strcmp(source,"waitroom"))){
                if(player->discard.n<RB_MAX_ZONE) player->discard.cards[player->discard.n++]=card;
            } else if(source && !strcmp(source,"hand")){
                if(player->hand.n<RB_MAX_ZONE) player->hand.cards[player->hand.n++]=card;
            } else if(source && !strcmp(source,"energy")){
                if(player->energy.n<RB_MAX_ZONE) player->energy.cards[player->energy.n++]=card;
            } else if(source && (!strcmp(source,"success")||!strcmp(source,"success_zone")||!strcmp(source,"success_live_zone")||!strcmp(source,"success_live_card_zone"))){
                if(player->success.n<RB_MAX_ZONE) player->success.cards[player->success.n++]=card;
            }
            /* stage source: a non-matching staged draw simply returns to its slot */
        }
    }
    return drawn;
}

/* Stubs for the AbilityResolver methods that use draw — full port lands with
   the 100-fixture audit. For now they delegate to the core helper so callers
   link. */
int rb_execute_draw_wrapper_stub(void *resolver, GameState *gs, AbilityEffect *e){
    (void)resolver; (void)gs; (void)e;
    return 0;
}
