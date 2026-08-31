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
    tg->state.cheer_check_base = -1;
    tg->state.baton_touch_replaced_member_cost = -1;
    tg->state.baton_touch_replaced_member_id = -1;
    tg->state.baton_touch_arriving_card_id = -1;
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
void test_place_under(TestGame *tg, int pl, int area, int card_id){
    /* Mirror stage.place_under_card(area, card): tuck `card_id` under the
        member occupying `area` of player `pl` (0=p1, 1=p2). */
    if(pl<0||pl>1||area<0||area>=RB_STAGE_SIZE) return;
    RbPlayer *P=&tg->state.p[pl];
    RbBag *u=&P->under_cards[area];
    if(u->n < RB_MAX_ZONE) u->cards[u->n++]=card_id;
}
void test_add_to_success(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++]=card_id;
}
void test_add_to_live(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->live.n < RB_MAX_ZONE) P->live.cards[P->live.n++]=card_id;
}
void test_add_to_deck(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++]=card_id;
}
void test_add_to_deck_pl(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++]=card_id;
}
/* Prepend a card to the top of player pl's deck (mirrors Rust
    main_deck.cards.insert(0, card)). Shifts existing cards down by one; if the
    deck is full the new card is dropped (matching a saturated RbBag). */
void test_insert_deck_top(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->deck.n >= RB_MAX_ZONE) return;
    for (int i = P->deck.n; i > 0; i--) P->deck.cards[i] = P->deck.cards[i-1];
    P->deck.cards[0] = card_id;
    P->deck.n++;
}
void test_add_to_energy(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++]=card_id;
    if(P->energy_active < RB_MAX_ZONE) P->energy_active++;
}
void test_set_energy_active(TestGame *tg, int pl, int n){
    if(pl<0||pl>1) return;
    tg->state.p[pl].energy_active = n;
}
void test_add_to_revealed(TestGame *tg, int card_id){
    if(tg->state.n_revealed < RB_MAX_RECENTLY_MOVED)
        tg->state.revealed_cards[tg->state.n_revealed++]=card_id;
}
void test_give_energy(TestGame *tg, int count){
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if(eid<0) eid=0;
    RbPlayer *P=&tg->state.p[0];
    for(int i=0;i<count;i++){
        if(P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++]=eid;
        P->energy_active++;   /* test helper: bypass the 7/12 in-game cap */
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
int test_try_play_to_stage(TestGame *tg, int card_id, int area){
    return test_play_to_stage(tg, card_id, area);
}
void test_recalc(TestGame *tg){ rb_recalc_constants(&tg->state); }
void test_clear_mods_for_card(TestGame *tg, int card_id){ rb_mods_clear_card(&tg->state.mods, card_id); }
void test_give_opp_energy(TestGame *tg, int count){
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if(eid<0) eid=0;
    RbPlayer *P=&tg->state.p[1];
    for(int i=0;i<count;i++){
        if(P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++]=eid;
        if(P->energy_active < RB_MAX_ENERGY_CARDS) P->energy_active++;
    }
}
void test_set_opp_stage(TestGame *tg, int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    tg->state.p[1].stage[area]=card_id;
    tg->state.p[1].stage_wait[area]=0;
}
void test_add_to_opp_live(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[1];
    if(P->live.n < RB_MAX_ZONE) P->live.cards[P->live.n++]=card_id;
}
void test_add_to_opp_success(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[1];
    if(P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++]=card_id;
}
void test_fire_debut(TestGame *tg, int card_id){ rb_fire_debut(&tg->state, 0, card_id); }
void test_expire_effects(TestGame *tg){ rb_check_expired_effects(&tg->state, 0); }
int test_activate_ability(TestGame *tg, int card_id){
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < P->hand.n; i++)
        if (P->hand.cards[i] == card_id) return rb_activate_ability(&tg->state, 0, i);
    /* Rust activate_ability also fires a member already on stage — run the real
        multi-ability activate path (cost + 起動-triggered effect). */
    return rb_activate_card(&tg->state, 0, card_id);
}
void test_spend_energy(TestGame *tg, int n){
    RbPlayer *P=&tg->state.p[0];
    P->energy_active -= n;
    if(P->energy_active < 0) P->energy_active = 0;
}
void test_drain_auto_choices(TestGame *tg){
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 1000) {
        if (tg->state.queue.resume_mode == 3 || tg->state.queue.auto_ability)
            rb_resume_with_choice(&tg->state, 0);  /* proceed with auto ability */
        else
            break;  /* only auto-ability prompts are drainable here */
    }
}
/* Answer a single interactive (position/target) pending choice by index — mirrors
   the test's select_position_option / accept_position_swap calls. */
void test_resume_choice(TestGame *tg, int idx){
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}
int test_has_pending_choice(TestGame *tg){ return rb_has_pending_choice(&tg->state); }
int test_pending_choice_count(TestGame *tg){ return rb_has_pending_choice(&tg->state) ? 1 : 0; }
void test_set_live_card(TestGame *tg, int zone, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(zone<0||zone>=RB_MAX_LIVE_CARDS) return;
    P->live.cards[zone]=card_id;
    if(zone+1 > P->live.n) P->live.n = zone+1;
}
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

/* Advance the current player's phase, mirroring Rust TestGame::pass() which
   feeds execute_main_phase_action(ActionType::Pass). */
void test_pass(TestGame *tg){ rb_advance_phase(&tg->state); }

const char *test_pending_choice_type(TestGame *tg){
    if(!rb_has_pending_choice(&tg->state)) return "";
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    switch(c->kind){
        case RB_CHOICE_SELECT_CARD:        return "SelectCard";
        case RB_CHOICE_SELECT_TARGET:      return "SelectTarget";
        case RB_CHOICE_SELECT_HEART_COLOR: return "SelectHeartColor";
        case RB_CHOICE_SELECT_NUMBER:      return "SelectNumber";
        case RB_CHOICE_SELECT_POSITION:    return "SelectPosition";
        case RB_CHOICE_SELECT_AUTO_ABILITY: return "SelectAutoAbility";
        default:                           return "Unknown";
    }
}

int test_get_blade_modifier(TestGame *tg, int cid){ return rb_mods_get_blade(&tg->state.mods, cid); }
int test_get_score_modifier(TestGame *tg, int cid){ return rb_mods_get_score(&tg->state.mods, cid); }
int test_get_cost_modifier(TestGame *tg, int cid){ return rb_mods_get_cost(&tg->state.mods, cid); }
int test_get_heart_modifier(TestGame *tg, int cid, int color){ return rb_mods_get_heart(&tg->state.mods, cid, color); }
void test_answer_play_cost_choice(TestGame *tg, int accept){
    if (tg->state.ptc_active) {
        rb_complete_play_with_cost(&tg->state, 0, accept);
        return;
    }
    /* No paused alt-cost play: fall back to the generic choice resume. */
    rb_resume_with_choice(&tg->state, accept ? 1 : 0);
}
/* Default filler card id (mirrors per-module `fn filler_hand(game)` helpers that
   return a common SD filler). Used only where the helper call appears inline. */
int test_filler_hand(TestGame *tg){ return rb_find_card_by_no("PL!-sd1-010-SD"); }

/* Collection-predicate helpers mirroring the Rust tests' ubiquitous
   `zone.cards.iter().any(|c| c.card_no == "X")` / `.contains(&id)` patterns,
   which a line-based transpiler cannot emit directly. */
static int zone_bag(TestGame *tg, int pl, const char *zone, RbBag **out){
    if(!strcmp(zone,"hand")) *out=&tg->state.p[pl].hand;
    else if(!strcmp(zone,"deck")||!strcmp(zone,"main_deck")) *out=&tg->state.p[pl].deck;
    else if(!strcmp(zone,"discard")||!strcmp(zone,"waitroom")) *out=&tg->state.p[pl].discard;
    else if(!strcmp(zone,"live")) *out=&tg->state.p[pl].live;
    else if(!strcmp(zone,"success")) *out=&tg->state.p[pl].success;
    else if(!strcmp(zone,"energy")||!strcmp(zone,"energy_zone")) *out=&tg->state.p[pl].energy;
    else *out=NULL;
    return *out!=NULL;
}
int rb_card_no_eq(int card_id, const char *no){
    Card c; if(!rb_decode_card_by_index((uint32_t)card_id,&c)) return 0;
    const char *cn = rb_card_string(c.card_no_idx);
    return cn && strcmp(cn, no)==0;
}
int test_zone_has_card_no(TestGame *tg, int pl, const char *zone, const char *no){
    RbBag *b=NULL;
    if(!strcmp(zone,"stage")){
        for(int i=0;i<RB_STAGE_SIZE;i++)
            if(tg->state.p[pl].stage[i]!=RB_EMPTY_SLOT && rb_card_no_eq(tg->state.p[pl].stage[i],no)) return 1;
        return 0;
    }
    if(!zone_bag(tg,pl,zone,&b)) return 0;
    for(int i=0;i<b->n;i++) if(rb_card_no_eq(b->cards[i],no)) return 1;
    return 0;
}
int test_zone_has_id(TestGame *tg, int pl, const char *zone, int id){
    RbBag *b=NULL;
    if(!strcmp(zone,"stage")){
        for(int i=0;i<RB_STAGE_SIZE;i++)
            if(tg->state.p[pl].stage[i]!=RB_EMPTY_SLOT && tg->state.p[pl].stage[i]==id) return 1;
        return 0;
    }
    if(!zone_bag(tg,pl,zone,&b)) return 0;
    for(int i=0;i<b->n;i++) if(b->cards[i]==id) return 1;
    return 0;
}
