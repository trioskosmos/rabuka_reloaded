#include "test_game.h"
#include <string.h>
#include <stdio.h>

void test_game_new(TestGame *tg){
    memset(tg,0,sizeof(*tg));
    rb_mods_init(&tg->state.mods);
    tg->state.winner=-1; tg->state.turn=1;
    tg->state.phase=RB_PHASE_MAIN;
    tg->state.active=0; tg->state.first_attacker=0; tg->state.second_attacker=1;
    for(int p=0;p<2;p++) for(int i=0;i<RB_STAGE_SIZE;i++) tg->state.p[p].stage[i]=RB_EMPTY_SLOT;
}

int test_id(TestGame *tg, const char *card_no){
    (void)tg;
    return rb_find_card_by_no(card_no);
}
void test_add_to_hand(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++]=card_id;
}
void test_add_to_discard(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++]=card_id;
}
void test_add_to_stage(TestGame *tg, int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    tg->state.p[0].stage[area]=card_id;
    tg->state.p[0].stage_wait[area]=0;
}
void test_add_to_success(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++]=card_id;
}
void test_give_energy(TestGame *tg, int count){
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if(eid<0) eid=0;
    RbPlayer *P=&tg->state.p[0];
    for(int i=0;i<count;i++){
        if(P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++]=eid;
        if(P->energy_active < RB_MAX_ENERGY_CARDS) P->energy_active++;
    }
}
int test_play_to_stage(TestGame *tg, int card_id, int area){
    /* find card in hand */
    RbPlayer *P=&tg->state.p[0];
    int idx=-1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id){ idx=i; break; }
    if(idx<0) return 0;
    return rb_play_member(&tg->state, 0, idx, area);
}
void test_recalc(TestGame *tg){ rb_recalc_constants(&tg->state); }
const char *test_card_name(int card_id){
    Card c; if(!rb_decode_card_by_index((uint32_t)card_id,&c)) return "?";
    const char *n=c.name; /* borrowed */
    /* Note: caller must not free; for debug only immediate use */
    return n ? n : "?";
}
int test_stage_has(TestGame *tg, int area, int card_id){ return tg->state.p[0].stage[area]==card_id; }
int test_hand_has(TestGame *tg, int card_id){
    for(int i=0;i<tg->state.p[0].hand.n;i++) if(tg->state.p[0].hand.cards[i]==card_id) return 1;
    return 0;
}
int test_success_count(TestGame *tg){ return tg->state.p[0].success.n; }
void test_print_board(TestGame *tg){ rb_print_state(&tg->state); }
