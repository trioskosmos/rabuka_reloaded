#include "rabuka.h"
#include <string.h>
#include <stdio.h>

int rb_member_area_to_index(const char *area){
    if(!area) return -1;
    if(!strcmp(area,"left")||!strcmp(area,"left_side")||!strcmp(area,"LeftSide")) return 0;
    if(!strcmp(area,"center")||!strcmp(area,"Center")) return 1;
    if(!strcmp(area,"right")||!strcmp(area,"right_side")||!strcmp(area,"RightSide")) return 2;
    return -1;
}
const char *rb_member_area_to_str(int idx){
    if(idx==0) return "left";
    if(idx==1) return "center";
    if(idx==2) return "right";
    return "?";
}
int rb_member_area_front(int area){
    if(area==0) return 2;
    if(area==1) return 1;
    if(area==2) return 0;
    return -1;
}

/* Mirror zones.rs: invariant, total_blades, draw, draw_bottom, draw_multiple, refresh */
int rb_zone_invariant(const GameState *g) {
    /* Stage must have exactly STAGE_SIZE positions and under_cards mirror */
    if (!g) return 0;
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        if (P->stage[0] < -1 || P->stage[1] < -1 || P->stage[2] < -1) return 0;
    }
    return 1;
}
int rb_zone_total_blades(const GameState *g, int pl, int include_waited) {
    if (!g) return 0;
    const RbPlayer *P = &g->p[pl];
    int total = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = P->stage[s];
        if (cid == RB_EMPTY_SLOT) continue;
        if (!include_waited && P->stage_wait[s]) continue;
        Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int eff = (int)c.blade + rb_mods_get_blade((RbMods *)&g->mods, cid);
        rb_free_card(&c);
        if (eff < 0) eff = 0; if (eff > 255) eff = 255;
        total += eff;
    }
    return total;
}
int rb_zone_draw(GameState *g, int pl) {
    if (!g) return -1;
    RbBag *deck = &g->p[pl].deck;
    if (deck->n == 0) return -1;
    int cid = deck->cards[0];
    for (int i = 0; i < deck->n - 1; i++) deck->cards[i] = deck->cards[i+1];
    deck->n--;
    return cid;
}
int rb_zone_draw_bottom(GameState *g, int pl) {
    if (!g) return -1;
    RbBag *deck = &g->p[pl].deck;
    if (deck->n == 0) return -1;
    return deck->cards[--deck->n];
}
int rb_zone_draw_multiple(GameState *g, int pl, int n) {
    if (!g) return 0;
    int drawn = 0;
    for (int i = 0; i < n; i++) {
        if (rb_zone_draw(g, pl) < 0) break;
        drawn++;
    }
    return drawn;
}
int rb_zone_refresh(GameState *g, int pl) {
    if (!g) return 0;
    RbPlayer *P = &g->p[pl];
    if (P->discard.n == 0) return 0;
    rb_shuffle(P->discard.cards, P->discard.n);
    for (int i = 0; i < P->discard.n && P->deck.n < RB_MAX_ZONE; i++)
        P->deck.cards[P->deck.n++] = P->discard.cards[i];
    P->discard.n = 0;
    P->deck_refreshed_this_turn = 1;
    return 1;
}
int rb_zone_track_deployment(const GameState *g) { (void)g; return 0; }

/* Mirror MemberArea::to_tag (Rule 4.5.7 wire protocol: 1=left, 2=center, 3=right). */
uint8_t rb_member_area_to_tag(int idx){
    if(idx==0) return 1;
    if(idx==1) return 2;
    if(idx==2) return 3;
    return 0;
}

/* Mirror MemberArea::from_tag — inverse of rb_member_area_to_tag. */
int rb_member_area_from_tag(uint8_t tag){
    if(tag==1) return 0;
    if(tag==2) return 1;
    if(tag==3) return 2;
    return -1;
}

int rb_check_trigger_position(const char *triggers, int card_position){
    if(!triggers) return 1;
    if(strstr(triggers,"左サイド") && card_position!=0) return 0;
    if(strstr(triggers,"右サイド") && card_position!=2) return 0;
    if(strstr(triggers,"センター") && card_position!=1) return 0;
    return 1;
}

int rb_check_effect_position(const char *effect_pos, int card_position){
    if(!effect_pos) return 1;
    if(strchr(effect_pos,',')){
        char buf[64]; strncpy(buf, effect_pos, 63); buf[63]=0;
        char *tok = strtok(buf, ",");
        while(tok){
            while(*tok==' ') tok++;
            char *end = tok+strlen(tok)-1; while(end>tok && *end==' ') *end--=0;
            if((!strcmp(tok,"center")||!strcmp(tok,"中央")) && card_position==1) return 1;
            if((!strcmp(tok,"left")||!strcmp(tok,"左")||!strcmp(tok,"左側")||!strcmp(tok,"left_side")) && card_position==0) return 1;
            if((!strcmp(tok,"right")||!strcmp(tok,"右")||!strcmp(tok,"右側")||!strcmp(tok,"right_side")) && card_position==2) return 1;
            tok = strtok(NULL, ",");
        }
        return 0;
    }
    if((!strcmp(effect_pos,"center")||!strcmp(effect_pos,"中央")) && card_position==1) return 1;
    if((!strcmp(effect_pos,"left")||!strcmp(effect_pos,"左")||!strcmp(effect_pos,"左側")||!strcmp(effect_pos,"left_side")) && card_position==0) return 1;
    if((!strcmp(effect_pos,"right")||!strcmp(effect_pos,"右")||!strcmp(effect_pos,"右側")||!strcmp(effect_pos,"right_side")) && card_position==2) return 1;
    if(!strcmp(effect_pos,"center")||!strcmp(effect_pos,"left")||!strcmp(effect_pos,"right")||
       !strcmp(effect_pos,"左")||!strcmp(effect_pos,"右")||!strcmp(effect_pos,"中央")||
       !strcmp(effect_pos,"左側")||!strcmp(effect_pos,"右側")||!strcmp(effect_pos,"left_side")||!strcmp(effect_pos,"right_side"))
        return 0;
    return 1;
}

int rb_stage_get_area(const int stage[RB_STAGE_SIZE], int area){
    if(area<0||area>=RB_STAGE_SIZE) return RB_EMPTY_SLOT;
    return stage[area];
}
void rb_stage_set_area(int stage[RB_STAGE_SIZE], int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    stage[area]=card_id;
}
int rb_stage_position_change(int stage[RB_STAGE_SIZE], int from_area, int to_area){
    if(from_area==to_area) return -1;
    if(from_area<0||from_area>=RB_STAGE_SIZE||to_area<0||to_area>=RB_STAGE_SIZE) return -1;
    int card_id = stage[from_area];
    if(card_id==RB_EMPTY_SLOT) return -1;
    int dest = stage[to_area];
    if(dest!=RB_EMPTY_SLOT){
        stage[from_area]=dest;
        stage[to_area]=card_id;
    } else {
        stage[to_area]=card_id;
        stage[from_area]=RB_EMPTY_SLOT;
    }
    return card_id;
}

int rb_stage_first_empty(const int stage[RB_STAGE_SIZE]){
    for(int i=0;i<RB_STAGE_SIZE;i++) if(stage[i]==RB_EMPTY_SLOT) return i;
    return -1;
}

/* ── Stage under-card helpers (mirror Stage::place_under_card /
   get_under_cards / under_cards_with_hosts / recycle_under_cards) ── */

void rb_stage_place_under_card(RbPlayer *player, int area, int card_id){
    if(!player || area<0 || area>=RB_STAGE_SIZE) return;
    RbBag *b = &player->under_cards[area];
    if(b->n < RB_MAX_ZONE) b->cards[b->n++] = card_id;
}

int rb_stage_get_under_cards(const RbPlayer *player, int area, int *out, int max){
    if(!player || area<0 || area>=RB_STAGE_SIZE || !out) return 0;
    const RbBag *b = &player->under_cards[area];
    int n = 0;
    for(int i=0;i<b->n && n<max;i++) out[n++] = b->cards[i];
    return n;
}

/* Mirror Stage::under_cards_with_hosts — emit (under_card, host_member) pairs.
   out_under[k] = tucked card id, out_host[k] = the stage member above it. */
int rb_stage_under_cards_with_hosts(const RbPlayer *player, int *out_under, int *out_host, int max){
    if(!player || !out_under || !out_host) return 0;
    int n = 0;
    for(int a=0;a<RB_STAGE_SIZE && n<max;a++){
        int host = player->stage[a];
        if(host==RB_EMPTY_SLOT) continue;
        const RbBag *b = &player->under_cards[a];
        for(int i=0;i<b->n && n<max;i++){
            out_under[n] = b->cards[i];
            out_host[n] = host;
            n++;
        }
    }
    return n;
}

/* Mirror Stage::recycle_under_cards (Rule 10.5.3-10.5.4): energy under-cards
   route to the energy deck, member under-cards to the waitroom. out_wait /
   out_energy are filled (each capped at max); *n_wait / *n_energy receive the
   counts. Returns total moved (-1 on bad args). */
int rb_stage_recycle_under_cards(GameState *g, int pl, int area,
                                 int *out_wait, int *n_wait,
                                 int *out_energy, int *n_energy, int max){
    if(!g || pl<0 || pl>1 || area<0 || area>=RB_STAGE_SIZE || !out_wait ||
       !n_wait || !out_energy || !n_energy) return -1;
    RbBag *b = &g->p[pl].under_cards[area];
    *n_wait = 0; *n_energy = 0;
    for(int i=0;i<b->n;i++){
        int cid = b->cards[i];
        if(rb_card_is_energy(cid)){
            if(*n_energy < max) out_energy[(*n_energy)++] = cid;
        } else {
            if(*n_wait < max) out_wait[(*n_wait)++] = cid;
        }
    }
    b->n = 0; /* mem::take */
    return *n_wait + *n_energy;
}

/* Mirror Stage::can_place_card (Rule 8.2.2): only non-live cards may occupy the
   stage. Unknown card ids place nowhere. */
int rb_stage_can_place_card(const GameState *g, int pl, int card_id){
    (void)g; (void)pl;
    if(!rb_card_record((uint32_t)card_id)) return 0;
    return rb_card_is_live(card_id) ? 0 : 1;
}

/* Mirror Stage::formation_change (Rule 11.11): apply a list of (from,to) area
   moves as one permutation; fails if two members target the same area or a
   source is empty. Under-cards travel with their member (Rule 4.5.5.3). */
int rb_stage_formation_change(GameState *g, int pl,
                              const int *from_areas, const int *to_areas, int n){
    if(!g || pl<0 || pl>1 || !from_areas || !to_areas) return -1;
    int *stage = g->p[pl].stage;
    RbBag *uc = g->p[pl].under_cards;
    for(int i=0;i<n;i++){
        if(from_areas[i]<0 || from_areas[i]>=RB_STAGE_SIZE) return -1;
        if(to_areas[i]<0 || to_areas[i]>=RB_STAGE_SIZE) return -1;
        for(int j=i+1;j<n;j++) if(to_areas[i]==to_areas[j]) return -1;
    }
    for(int i=0;i<n;i++){
        int f = from_areas[i], t = to_areas[i];
        if(f==t) return -1;
        int card_id = stage[f];
        if(card_id==RB_EMPTY_SLOT) return -1;
        RbBag tmp = uc[f]; uc[f] = uc[t]; uc[t] = tmp; /* swap under-cards */
        int dest = stage[t];
        if(dest!=RB_EMPTY_SLOT){
            stage[f] = dest; stage[t] = card_id;
        } else {
            stage[t] = card_id; stage[f] = RB_EMPTY_SLOT;
        }
    }
    return 0;
}

/* ── EnergyZone helpers (mirror EnergyZone; backed by RbPlayer::energy /
   energy_active) ── */

int rb_energy_can_place_card(const RbPlayer *player, int card_id){
    (void)player;
    if(!rb_card_record((uint32_t)card_id)) return 0;
    return rb_card_is_energy(card_id) ? 1 : 0;
}

int rb_energy_add_card(RbPlayer *player, int card_id){
    if(!player) return -1;
    if(player->energy.n >= RB_MAX_ZONE) return -1;
    player->energy.cards[player->energy.n++] = card_id;
    player->energy_active++; /* new energy starts Active (Rule 7.4) */
    return 0;
}

int rb_energy_pay(RbPlayer *player, int amount){
    if(!player || amount<0) return -1;
    if(player->energy_active >= amount){
        player->energy_active -= amount; /* Rule 5.9 */
        return 0;
    }
    return -1;
}

void rb_energy_activate_all(RbPlayer *player){
    if(!player) return;
    player->energy_active = player->energy.n;
}

int rb_energy_active_count(const RbPlayer *player){
    return player ? player->energy_active : 0;
}

/* Mirror EnergyZone::set_active_count (Rule 7.4 energy activation state). */
void rb_energy_set_active_count(RbPlayer *player, int count){
    if(!player) return;
    if(count < 0) count = 0;
    player->energy_active = count;
}

/* Mirror EnergyZone::add_active — saturating increment of active energy. */
void rb_energy_add_active(RbPlayer *player, int delta){
    if(!player || delta <= 0) return;
    player->energy_active += delta;
    if(player->energy_active < 0) player->energy_active = 0;
}

/* Mirror EnergyZone::sub_active — saturating decrement of active energy. */
void rb_energy_sub_active(RbPlayer *player, int delta){
    if(!player || delta <= 0) return;
    player->energy_active -= delta;
    if(player->energy_active < 0) player->energy_active = 0;
}

/* ── LiveCardZone helpers (mirror LiveCardZone; backed by RbPlayer::live) ── */

int rb_live_can_place_card(const RbPlayer *player, int card_id){
    if (!player) return 0;
    if (card_id < 0) return 0;
    /* Rule 8.2: any card from hand may enter the live card zone, subject to the
        live-zone capacity (RB_MAX_LIVE_CARDS). */
    if (player->live.n >= RB_MAX_LIVE_CARDS) return 0;
    return 1;
}

int rb_live_add_card(RbPlayer *player, int card_id){
    if(!player) return -1;
    if(player->live.n >= RB_MAX_LIVE_CARDS) return -1;
    player->live.cards[player->live.n++] = card_id;
    return 0;
}

int rb_live_clear(RbPlayer *player, int *out, int max){
    if(!player || !out) return 0;
    int n = 0;
    for(int i=0;i<player->live.n && n<max;i++) out[n++] = player->live.cards[i];
    player->live.n = 0;
    return n;
}

int rb_live_len(const RbPlayer *player){
    return player ? player->live.n : 0;
}

/* ── Hand helpers (mirror Hand; backed by RbPlayer::hand) ── */

void rb_hand_add(RbPlayer *player, int card_id){
    if(!player) return;
    if(player->hand.n < RB_MAX_HAND) player->hand.cards[player->hand.n++] = card_id;
}

int rb_hand_remove_card(RbPlayer *player, int index){
    if(!player || index<0 || index>=player->hand.n) return -1;
    int c = player->hand.cards[index];
    for(int k=index;k<player->hand.n-1;k++) player->hand.cards[k] = player->hand.cards[k+1];
    player->hand.n--;
    return c;
}

int rb_hand_len(const RbPlayer *player){
    return player ? player->hand.n : 0;
}

int rb_hand_is_empty(const RbPlayer *player){
    return player ? player->hand.n==0 : 1;
}

/* ── Waitroom helpers (mirror Waitroom; backed by RbPlayer::discard) ── */

void rb_waitroom_add(RbPlayer *player, int card_id){
    if(!player) return;
    if(player->discard.n < RB_MAX_ZONE) player->discard.cards[player->discard.n++] = card_id;
}

int rb_waitroom_take_all(RbPlayer *player, int *out, int max){
    if(!player || !out) return 0;
    int n = 0;
    for(int i=0;i<player->discard.n && n<max;i++) out[n++] = player->discard.cards[i];
    player->discard.n = 0;
    return n;
}

void rb_waitroom_shuffle(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return;
    rb_shuffle(g->p[pl].discard.cards, g->p[pl].discard.n);
}

int rb_waitroom_len(const RbPlayer *player){
    return player ? player->discard.n : 0;
}

void rb_waitroom_remove_card(RbPlayer *player, int card_id){
    if(!player) return;
    int w = 0;
    for(int i=0;i<player->discard.n;i++)
        if(player->discard.cards[i]!=card_id) player->discard.cards[w++] = player->discard.cards[i];
    player->discard.n = w;
}

/* ── SuccessLiveCardZone helpers (mirror SuccessLiveCardZone; RbPlayer::success) ── */

void rb_success_add(RbPlayer *player, int card_id){
    if(!player) return;
    if(player->success.n < RB_MAX_LIVE_CARDS) player->success.cards[player->success.n++] = card_id;
}

int rb_success_len(const RbPlayer *player){
    return player ? player->success.n : 0;
}

/* ── ResolutionZone helpers (mirror ResolutionZone; GameState::resolution) ── */

void rb_resolution_add(GameState *g, int card_id){
    if(!g) return;
    if(g->resolution.n < RB_MAX_ZONE) g->resolution.cards[g->resolution.n++] = card_id;
}

int rb_resolution_clear(GameState *g, int *out, int max){
    if(!g || !out) return 0;
    int n = 0;
    for(int i=0;i<g->resolution.n && n<max;i++) out[n++] = g->resolution.cards[i];
    g->resolution.n = 0;
    return n;
}

int rb_resolution_len(const GameState *g){
    return g ? g->resolution.n : 0;
}
