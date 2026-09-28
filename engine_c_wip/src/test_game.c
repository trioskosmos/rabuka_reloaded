#include "test_game.h"
#include <string.h>
#include <stdio.h>

void test_game_new(TestGame *tg){
    memset(tg,0,sizeof(*tg));
    rb_look_reset_all();
    rb_mods_init(&tg->state.mods);
    tg->state.winner=-1; tg->state.turn=1;
    tg->state.phase=RB_PHASE_MAIN;
    tg->state.active=0; tg->state.first_attacker=0; tg->state.second_attacker=1;
    for(int p=0;p<2;p++) for(int i=0;i<RB_STAGE_SIZE;i++) tg->state.p[p].stage[i]=RB_EMPTY_SLOT;
    tg->state.cheer_check_base = -1;
    tg->state.baton_touch_replaced_member_cost = -1;
    tg->state.baton_touch_replaced_member_id = -1;
     tg->state.baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) tg->state.gained_card_ids[i] = -1;
}


int test_id(TestGame *tg, const char *card_no){
    (void)tg;
    return rb_find_card_by_no(card_no);
}
int test_new_id(TestGame *tg, const char *card_no){
    (void)tg;
    int template_id = rb_find_card_by_no(card_no);
    if (template_id < 0) return -1;
    return rb_create_card_copy(template_id);
}
void test_add_to_hand_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++]=card_id;
}
void test_add_to_hand(TestGame *tg, int card_id){ test_add_to_hand_for(tg, 0, card_id); }
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
void test_add_to_live(TestGame *tg, int card_id){ test_add_to_live_for(tg, 0, card_id); }
/* Seat-aware raw live placement (Rust live_card_zone.cards.push with no hand
   removal). Like test_set_live_card_for this APPENDS, and it appends to `pl`'s
   OWN live zone. The cap stays RB_MAX_ZONE (the bag capacity test_add_to_live
   has always used) so this shim's reachability does not change. */
void test_add_to_live_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1 || card_id < 0) return;
    RbPlayer *P=&tg->state.p[pl];
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
/* Add a card to the energy deck (mirrors Rust energy_deck.cards.push) */
void test_add_to_energy_deck(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++]=card_id;
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
    /* Mirror rb_energy_add_card (zones.c): the card is appended and the
       ACTIVE count rises with it. Only bump active when the card really
       landed, so energy_active <= energy.n stays true (the invariant
       rb_energy_activate_all relies on). */
    if(P->energy.n >= RB_MAX_ZONE) return;
    P->energy.cards[P->energy.n++]=card_id;
    if(P->energy_active < P->energy.n) P->energy_active++;
}
void test_set_energy_active(TestGame *tg, int pl, int n){
    if(pl<0||pl>1) return;
    tg->state.p[pl].energy_active = n;
}
void test_add_to_revealed(TestGame *tg, int card_id){
    if(tg->state.n_revealed < RB_MAX_REVEALED_CARDS)
        tg->state.revealed_cards[tg->state.n_revealed++]=card_id;
}
void test_give_opp_energy(TestGame *tg, int count){
    test_give_energy_for(tg, 1, count);
}
/* Find a live card with a specific score — mirrors Rust's db lookup.
   Returns the card index, or -1 when no live card carries that score. */
int test_find_live_by_score(TestGame *tg, int score){
    (void)tg;
    uint32_t n = rb_num_cards();
    for(uint32_t i = 0; i < n; i++){
        if(!rb_card_is_live((int)i)) continue;
        Card c;
        if(!rb_decode_card_by_index(i, &c)) continue;
        int match = (int)c.score == score;
        rb_free_card(&c);
        if(match) return (int)i;
    }
    return -1;
}

void test_give_energy(TestGame *tg, int count){ test_give_energy_for(tg, 0, count); }

/* Give `count` energy cards to seat `pl`, all Active.
   Mirror Rust TestGame::give_energy_for -> energy_zone.push_active. The C
   energy zone is an unordered bag plus a SEPARATE `energy_active` count (see
   rb_energy_add_card / rb_energy_activate_all in zones.c), so "push active"
   is: append the card, then raise the active count with it. A card that does
   not land never raises the count, so energy_active <= energy.n — the
   invariant rb_energy_activate_all relies on — always holds. */
void test_give_energy_for(TestGame *tg, int pl, int count){
    if(!tg || pl<0 || pl>1) return;
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if(eid<0) eid=0;
    RbPlayer *P=&tg->state.p[pl];
    for(int i=0;i<count;i++){
        if(P->energy.n >= RB_MAX_ZONE) return;  /* bypasses the 7/12 cap, not the bag */
        P->energy.cards[P->energy.n++]=eid;
        P->energy_active++;
    }
}
int test_play_to_stage(TestGame *tg, int card_id, int area){
    return test_play_to_stage_for(tg, 0, card_id, area);
}
/* Mirror TestGame::try_play_to_stage_for(Side, card, area): make `pl` the
   attacking seat, then run the real main-phase play action. Returns 1 on
   success, 0 when the card is not in that seat's hand. */
int test_play_to_stage_for(TestGame *tg, int pl, int card_id, int area){
    if(!tg || pl<0 || pl>1) return 0;
    RbPlayer *P=&tg->state.p[pl];
    int idx=-1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id){ idx=i; break; }
    if(idx<0) return 0;
    return rb_play_member(&tg->state, pl, idx, area);
}
int test_try_play_to_stage(TestGame *tg, int card_id, int area){
    return test_play_to_stage(tg, card_id, area);
}
void test_recalc(TestGame *tg){ rb_recalc_constants(&tg->state); }
void test_clear_mods_for_card(TestGame *tg, int card_id){ rb_mods_clear_card(&tg->state.mods, card_id); }
void test_set_opp_stage(TestGame *tg, int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    tg->state.p[1].stage[area]=card_id;
    tg->state.p[1].stage_wait[area]=0;
}
void test_add_to_opp_live(TestGame *tg, int card_id){ test_add_to_live_for(tg, 1, card_id); }
void test_add_to_opp_success(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[1];
    if(P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++]=card_id;
}
void test_fire_debut(TestGame *tg, int card_id){ rb_fire_debut(&tg->state, 0, card_id); }
void test_expire_effects(TestGame *tg){ rb_check_expired_effects(&tg->state, 0); }
int test_activate_ability(TestGame *tg, int card_id){
    return test_activate_ability_for(tg, 0, card_id);
}
/* Mirror TestGame::activate_ability_for(Side, card): make `pl` the attacking
   seat, then run the real 起動 path. */
int test_activate_ability_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1) return 0;
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++)
        if (P->hand.cards[i] == card_id) return rb_activate_ability(&tg->state, pl, i);
    /* Rust activate_ability also fires a member already on stage — run the real
        multi-ability activate path (cost + 起動-triggered effect). */
    return rb_activate_card(&tg->state, pl, card_id);
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

/* Select indices from a pending choice — mirrors game.select_indices(&[..]).
   n == 0 is Rust's DECLINE (select_indices(&[])) and must go through the
   engine's skip path (rb_resume_with_choice(g, -1)); it is NOT a no-op.
   n > 1 is a genuine multi-index answer and must reach the engine intact
   (rb_resume_with_choice_indices). The previous revision used only indices[0]
   and skipped the resume entirely when n == 0, so "decline" left the prompt
   pending (turning the ubiquitous
       while (test_has_pending_choice(g)) test_select_indices(g, NULL, 0);
   drain loops into no-ops) and a multi-card pick could never complete. */
void test_select_indices(TestGame *tg, const int *indices, int n){
    if (!tg || !rb_has_pending_choice(&tg->state)) return;
    if (n > 0 && indices) rb_resume_with_choice_indices(&tg->state, indices, n);
    else                     rb_resume_with_choice(&tg->state, -1);
}
int test_has_pending_choice(TestGame *tg){ return rb_has_pending_choice(&tg->state); }
int test_pending_choice_count(TestGame *tg){ return rb_has_pending_choice(&tg->state) ? 1 : 0; }
void test_set_live_card(TestGame *tg, int slot, int card_id){
    /* Live-card placement — mirror Rust TestGame::set_live_card(card_id).
       The live zone is an ORDERED bag: the card leaves the hand and is
       APPENDED to live.cards[live.n++], which is what live_card_zone.cards
       .push does in Rust.

       The `slot` parameter is IGNORED. It exists only because
       tools/port_one.py emits test_set_live_card(&game, 0, id) for the
       one-argument Rust call, and every call site in the transpiled corpus
       passes 0. The previous revision wrote live.cards[slot] at a FIXED index
       and only grew n to slot+1, so three consecutive calls put ONE card in
       the zone and silently invalidated every transpiled live test (canary
       live_cards_stuck_in_live_zone_instead_of_discard: "got 1 expected 3").
       Use test_insert_live_card_at for genuine positional placement. */
    (void)slot;
    test_set_live_card_for(tg, 0, card_id);
}
/* Seat-aware live placement — mirror TestGame::set_live_card_for(Side, card).
   The live zone it writes is `pl`'s OWN, so a P2-owned live card really lands
   in p2's live_card_zone (which is what a P2-triggered ability scans) instead
   of silently landing in P1's zone, where the fixture still looks plausible
   and measures the wrong thing. Still the APPEND path; never a fixed index. */
void test_set_live_card_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1 || card_id < 0) return;
    RbPlayer *P=&tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] == card_id) {
            rb_hand_remove_card(P, i);
            break;
        }
    }
    if (P->live.n >= RB_MAX_LIVE_CARDS) return;   /* the live zone holds at most 3 */
    P->live.cards[P->live.n++] = card_id;
}
void test_insert_live_card_at(TestGame *tg, int slot, int card_id){
    test_insert_live_card_at_for(tg, 0, slot, card_id);
}
/* Explicit positional live-zone placement: shift the tail right and place the
   card at `slot`. Test-only; prefer the append path (test_set_live_card) so a
   test never has to reason about slot reuse.

   Contract, chosen so a live zone can never contain a hole:
     slot == n          append at the tail;
     slot  < n          ordered insert, tail shifts right, and if the 3-card
                        zone is already full the LAST card falls off the end;
     slot  > n          refused -- the live zone is an ordered bag, and a gap
                        there would be read either as a phantom card (the bare
                        slot still holds whatever the zone had, 0 on a fresh
                        TestGame, which IS a real card index) or as end-of-zone
                        by the zone accessors. Neither is a card state the
                        engine can produce, so the placement is refused
                        instead of fabricating one. */
void test_insert_live_card_at_for(TestGame *tg, int pl, int slot, int card_id){
    if(!tg || pl<0 || pl>1 || card_id < 0) return;
    if(slot<0 || slot>=RB_MAX_LIVE_CARDS) return;
    RbPlayer *P=&tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] == card_id) {
            rb_hand_remove_card(P, i);
            break;
        }
    }
    if (slot > P->live.n) return;            /* would leave a gap: refuse */
    if (slot == P->live.n) {
        P->live.n = slot + 1;
    } else {
        int n = P->live.n;
        if (n >= RB_MAX_LIVE_CARDS) {
            /* Full: shift the tail right and let the last card fall off. The
               previous revision clamped to CAP-1 and then wrote cards[slot]
               over a live card, silently destroying it. */
            for (int i = RB_MAX_LIVE_CARDS - 1; i > slot; i--)
                P->live.cards[i] = P->live.cards[i-1];
        } else {
            for (int i = n; i > slot; i--) P->live.cards[i] = P->live.cards[i-1];
            P->live.n = n + 1;
        }
    }
    P->live.cards[slot] = card_id;
}
/* Debug-only card-name accessor. rb_decode_card_by_index allocates a Card
   whose `name` is heap-owned, so the previous revision either leaked the whole
   Card on every call (it never freed) or, once freed, handed back a dangling
   pointer. Copy the name into a small rotating set of static buffers: no
   leak, and the returned pointer stays valid for the last 8 calls. */
#define TEST_NAME_SLOTS 8
#define TEST_NAME_LEN   256
static char g_name_buf[TEST_NAME_SLOTS][TEST_NAME_LEN];
static int  g_name_next;
const char *test_card_name(int card_id){
    if(card_id < 0) return "?";
    Card c;
    if(!rb_decode_card_by_index((uint32_t)card_id,&c)) return "?";
    char *slot = g_name_buf[g_name_next];
    g_name_next = (g_name_next + 1) % TEST_NAME_SLOTS;
    if(c.name){
        strncpy(slot, c.name, TEST_NAME_LEN - 1);
        slot[TEST_NAME_LEN - 1] = '\0';
    } else {
        slot[0] = '?'; slot[1] = '\0';
    }
    rb_free_card(&c);
    return slot;
}
int test_stage_has(TestGame *tg, int area, int card_id){
    if(!tg || area<0 || area>=RB_STAGE_SIZE) return 0;
    return tg->state.p[0].stage[area]==card_id;
}
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
int test_get_heart_modifier(TestGame *tg, int cid, int color){
    if (color == 5) color = RB_HEART_ORANGE;
    int value = rb_mods_get_heart(&tg->state.mods, cid, color);
    fprintf(stderr, "[TEST_HEART_READ] cid=%d color=%d value=%d\n", cid, color, value);
    return value;
}
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
static RbBag *zone_bag(TestGame *tg, int pl, const char *zone){
    /* `pl` is validated HERE so no caller can index p[] out of range. */
    if(!tg || !zone || pl<0 || pl>1) return NULL;
    RbPlayer *P=&tg->state.p[pl];
    if(!strcmp(zone,"hand")) return &P->hand;
    if(!strcmp(zone,"deck")||!strcmp(zone,"main_deck")) return &P->deck;
    if(!strcmp(zone,"discard")||!strcmp(zone,"waitroom")) return &P->discard;
    if(!strcmp(zone,"live")||!strcmp(zone,"live_card_zone")) return &P->live;
    if(!strcmp(zone,"success")||!strcmp(zone,"success_live_card_zone")) return &P->success;
    if(!strcmp(zone,"energy")||!strcmp(zone,"energy_zone")) return &P->energy;
    if(!strcmp(zone,"energy_deck")) return &P->energy_deck;
    return NULL;
}
/* The i-th card of `zone` for seat `pl`, or -1 when out of range / unknown
   zone. One accessor, so test_zone_len / test_zone_ids /
   test_zone_count_of_id / test_zone_has_id / test_zone_has_card_no can never
   disagree about what a zone contains. `stage` skips empty slots; `under<0..2>`
   addresses the per-member under-card bag. */
static int zone_card_at(TestGame *tg, int pl, const char *zone, int i){
    if(!tg || !zone || pl<0 || pl>1 || i<0) return -1;
    RbPlayer *P=&tg->state.p[pl];
    if(!strcmp(zone,"stage")){
        int seen=0;
        for(int a=0;a<RB_STAGE_SIZE;a++){
            if(P->stage[a]==RB_EMPTY_SLOT) continue;
            if(seen++==i) return P->stage[a];
        }
        return -1;
    }
    if(!strncmp(zone,"under",5) && zone[5]){
        int a=0; const char *d=zone+5;
        while(*d>='0' && *d<='9'){ a=a*10+(*d-'0'); d++; }
        if(*d || a<0 || a>=RB_STAGE_SIZE) return -1;
        RbBag *b=&P->under_cards[a];
        return i<b->n ? b->cards[i] : -1;
    }
    RbBag *b=zone_bag(tg,pl,zone);
    if(!b || i>=b->n) return -1;
    return b->cards[i];
}
int test_zone_len(TestGame *tg, int pl, const char *zone){
    int n=0;
    while(n<RB_MAX_ZONE && zone_card_at(tg,pl,zone,n)>=0) n++;
    return n;
}
int test_zone_ids(TestGame *tg, int pl, const char *zone, int *out, int max){
    if(!out || max<=0) return 0;
    int n=0;
    while(n<max){
        int c=zone_card_at(tg,pl,zone,n);
        if(c<0) break;
        out[n++]=c;
    }
    return n;
}
int test_zone_count_of_id(TestGame *tg, int pl, const char *zone, int id){
    int n=0;
    for(int i=0;i<RB_MAX_ZONE;i++){
        int c=zone_card_at(tg,pl,zone,i);
        if(c<0) break;
        if(c==id) n++;
    }
    return n;
}
int test_total_card_count(TestGame *tg){
    /* Zone-independent census: every card slot of BOTH seats. A move between
       zones must leave this unchanged; a drain that duplicates or drops a card
       changes it. */
    static const char *const zones[] = {
        "hand","main_deck","waitroom","live","success","energy","energy_deck"
    };
    if(!tg) return 0;
    int total=0;
    for(int pl=0;pl<2;pl++){
        total += test_zone_len(tg,pl,"stage");
        for(int a=0;a<RB_STAGE_SIZE;a++){
            char z[16];
            snprintf(z,sizeof(z),"under%d",a);
            total += test_zone_len(tg,pl,z);
        }
        for(size_t i=0;i<sizeof(zones)/sizeof(zones[0]);i++)
            total += test_zone_len(tg,pl,zones[i]);
    }
    return total;
}
int rb_card_no_eq(int card_id, const char *no){
    if(!no || card_id<0) return 0;
    Card c;
    if(!rb_decode_card_by_index((uint32_t)card_id,&c)) return 0;
    const char *cn = rb_card_string(c.card_no_idx);
    int eq = (cn && strcmp(cn, no)==0);
    rb_free_card(&c);   /* the decoded Card is heap-owned */
    return eq;
}
int test_zone_has_card_no(TestGame *tg, int pl, const char *zone, const char *no){
    if(!no) return 0;
    for(int i=0;i<RB_MAX_ZONE;i++){
        int c=zone_card_at(tg,pl,zone,i);
        if(c<0) return 0;
        if(rb_card_no_eq(c,no)) return 1;
    }
    return 0;
}
int test_zone_has_id(TestGame *tg, int pl, const char *zone, int id){
    for(int i=0;i<RB_MAX_ZONE;i++){
        int c=zone_card_at(tg,pl,zone,i);
        if(c<0) return 0;
        if(c==id) return 1;
    }
    return 0;
}

/* ── cost / selection zone moves (see the header for the scope note) ── */
static int bag_take_id(RbBag *b, int id){
    for(int i=0;i<b->n;i++){
        if(b->cards[i]!=id) continue;
        for(int k=i;k<b->n-1;k++) b->cards[k]=b->cards[k+1];
        b->n--;
        return 1;
    }
    return 0;
}
int test_move_hand_to_waitroom(TestGame *tg, int pl, int n){
    if(!tg || pl<0 || pl>1) return 0;
    RbPlayer *P=&tg->state.p[pl];
    int moved=0;
    for(int i=0;i<n && P->hand.n>0;i++){
        int id=P->hand.cards[0];
        rb_hand_remove_card(P,0);
        rb_waitroom_add(P,id);
        moved++;
    }
    return moved;
}
int test_move_ids_to_waitroom(TestGame *tg, int pl, const int *ids, int n){
    if(!tg || !ids || n<=0 || pl<0 || pl>1) return 0;
    RbPlayer *P=&tg->state.p[pl];
    int moved=0;
    for(int i=0;i<n;i++){
        int id=ids[i];
        if(id<0) continue;
        /* hand, then live, then success: the zones a cost / selection pick can
           legitimately come from. */
        if(bag_take_id(&P->hand,id) || bag_take_id(&P->live,id) || bag_take_id(&P->success,id)){
            rb_waitroom_add(P,id);
            moved++;
        }
    }
    return moved;
}

/* Mirror TestGame::advance_to_phase: step the turn until `target` is current.
   A prompt raised by the arriving step is deliberately NOT answered here — the
   caller is usually there to inspect it. Returns 1 on arrival, 0 on timeout. */
int test_advance_to_phase(TestGame *tg, int target){
    if(!tg) return 0;
    for(int i=0;i<16;i++){
        if(tg->state.phase==target) return 1;
        test_pass(tg);
        if(tg->state.phase==target) return 1;
        int guard=0;
        while(rb_has_pending_choice(&tg->state) && guard++<64)
            rb_resume_with_choice(&tg->state, 0);
    }
    return 0;
}

int test_deck_len(TestGame *tg){ return tg->state.p[0].deck.n; }
int test_hand_len(TestGame *tg){ return tg->state.p[0].hand.n; }

/* ── queue-entry seat introspection ──────────────────────────────────────
   A Rust test asserts things like
       assert_eq!(entry.choice_player_id.as_deref(), Some("p2"));
   whose C stand-in is `strcmp(game.queue.entries[i].choice_player_id, "p2")`.
   Two suites hand-rolled exactly that compare, in two different shapes, and
   neither could report the EMPTY-string case cleanly (an empty token compared
   equal to "p1" only by accident of never being asserted at all). These
   accessors give the comparison one typed spelling:

     - test_queue_entry_seat / test_queue_entry_choice_seat  -> 0 (p1), 1 (p2),
       or -1 for "empty or unrecognised". -1 is a DISTINCT answer from 0, so a
       test can assert the defect an un-stamped live-start entry causes.
     - the *_owned_by_seat predicates for the plain "is it this seat?" question.
     - the *_player_id readers for a test that wants the raw string.

   The long form is normalised here exactly as Rust's
   build_ability_queue_entry does (engine/src/core/game_state/abilities.rs
   :240-246: "player1" -> "p1", "player2" -> "p2"), so a test never has to
   re-implement that mapping. Layout of RbQueueEntry is untouched. */
const char *test_seat_id(int pl){
    if (pl == 0) return "p1";
    if (pl == 1) return "p2";
    return "";
}
/* Map a queue-entry player token onto a seat index. Returns -1 for NULL, "" and
   anything unrecognised, so "unstamped" and "seat 0" are never confused. */
static int seat_of_token(const char *tok){
    if(!tok) return -1;
    if(!strcmp(tok,"p1")||!strcmp(tok,"player1")) return 0;
    if(!strcmp(tok,"p2")||!strcmp(tok,"player2")) return 1;
    return -1;
}
int test_queue_n_entries(TestGame *tg){ return tg ? tg->state.queue.n_entries : 0; }
int test_queue_entry_seat(TestGame *tg, int idx){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries) return -1;
    return seat_of_token(tg->state.queue.entries[idx].player_id);
}
int test_queue_entry_choice_seat(TestGame *tg, int idx){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries) return -1;
    return seat_of_token(tg->state.queue.entries[idx].choice_player_id);
}
/* Copy the entry's owner token into `buf`. Returns 1 when the index addresses a
   real entry, 0 otherwise; `buf` is always NUL-terminated and cleared first, so
   a failed read yields "" rather than a stale value. */
static int queue_entry_id_copy(TestGame *tg, int idx, const char *field,
                               char *buf, size_t buf_len){
    if(!buf || buf_len==0) return 0;
    buf[0]='\0';
    if(!tg || !field || idx<0 || idx>=tg->state.queue.n_entries) return 0;
    snprintf(buf, buf_len, "%s", field);
    return 1;
}
int test_queue_entry_player_id(TestGame *tg, int idx, char *buf, size_t buf_len){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries){
        if(buf && buf_len) buf[0]='\0';
        return 0;
    }
    return queue_entry_id_copy(tg, idx, tg->state.queue.entries[idx].player_id, buf, buf_len);
}
int test_queue_entry_choice_player_id(TestGame *tg, int idx, char *buf, size_t buf_len){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries){
        if(buf && buf_len) buf[0]='\0';
        return 0;
    }
    return queue_entry_id_copy(tg, idx, tg->state.queue.entries[idx].choice_player_id, buf, buf_len);
}
int test_queue_entry_owned_by_seat(TestGame *tg, int idx, int pl){
    if(pl<0 || pl>1) return 0;
    return test_queue_entry_seat(tg, idx) == pl;
}
int test_queue_entry_choice_owned_by_seat(TestGame *tg, int idx, int pl){
    if(pl<0 || pl>1) return 0;
    return test_queue_entry_choice_seat(tg, idx) == pl;
}

/* ── distinct-name count ──────────────────────────────────────────────────
   rb_max_distinct_names is defined in src/ability/util.c but is NOT declared in
   include/rabuka.h, so a test that wanted Rust's max_distinct_names had to
   settle for the declared sibling rb_count_distinct_member_name_units and
   measure something else. The header is not this file's to edit, so declare
   the symbol here and expose it through the test header instead. */
extern int rb_max_distinct_names(const int *cards, int n);
int test_max_distinct_names(const int *cards, int n){
    if(!cards || n<=0) return 0;
    return rb_max_distinct_names(cards, n);
}
