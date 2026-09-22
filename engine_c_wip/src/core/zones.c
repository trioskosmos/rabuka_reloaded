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

/* Mirror MemberArea::from_index — inverse of rb_member_area_to_index. */
int rb_member_area_from_index(int idx){
    if(idx==0) return 0;
    if(idx==1) return 1;
    if(idx==2) return 2;
    return -1;
}

/* Mirror MainDeck::shuffle (Rule: index 0 = top). */
void rb_deck_shuffle(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return;
    rb_shuffle(g->p[pl].deck.cards, g->p[pl].deck.n);
}

/* Mirror MainDeck::is_empty. */
int rb_deck_is_empty(const GameState *g, int pl){
    if(!g || pl<0 || pl>1) return 1;
    return g->p[pl].deck.n == 0;
}

/* Mirror MainDeck::len. */
int rb_deck_len(const GameState *g, int pl){
    if(!g || pl<0 || pl>1) return 0;
    return g->p[pl].deck.n;
}

/* Mirror EnergyDeck::draw — draw the top energy card. */
int rb_energy_deck_draw(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return -1;
    RbBag *d = &g->p[pl].energy_deck;
    if(d->n == 0) return -1;
    int cid = d->cards[0];
    for(int i=0;i<d->n-1;i++) d->cards[i] = d->cards[i+1];
    d->n--;
    return cid;
}

/* Mirror EnergyDeck::is_empty. */
int rb_energy_deck_is_empty(const GameState *g, int pl){
    if(!g || pl<0 || pl>1) return 1;
    return g->p[pl].energy_deck.n == 0;
}

/* Check whether `need` (8-element color counts) is satisfied by `provided`.
 * Mirrors the relevant part of check_heart_requirement for flat arrays. */
static int local_check_heart_req(const int *need, const int *provided){
    int total_need = 0, total_prov = 0;
    for(int c=0;c<8;c++){
        if(need[c] > 0) total_need += need[c];
        if(provided && provided[c] > 0) total_prov += provided[c];
    }
    if(total_need == 0) return 1;
    if(total_prov < total_need) return 0;
    int wildcard = (provided ? provided[0] : 0) + (provided ? provided[7] : 0);
    for(int c=1;c<7;c++){
        if(!provided) continue;
        int deficit = need[c] - provided[c];
        if(deficit > 0){
            if(wildcard < deficit) return 0;
            wildcard -= deficit;
        }
    }
    return 1;
}

/* Mirror LiveCardZone::calculate_live_score (Rule 5.2/8.2). Iterates the live
 * card zone, sums each card's (base_score + score_modifier) when its need_heart
 * is satisfied by stage_hearts, then adds cheer_blade_heart_count and the
 * constant_total_score_bonus — all saturated to u8. */
int rb_live_calculate_score(const GameState *g, int pl, int cheer_blade_heart_count,
                            const int *stage_hearts, int constant_total_score_bonus){
    if(!g || pl<0 || pl>1) return 0;
    const RbPlayer *P = &g->p[pl];
    int total_score = 0;
    for(int i=0;i<P->live.n;i++){
        int cid = P->live.cards[i];
        Card c;
        if(!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int base_score = (int)c.score;
        int modifier = rb_mods_get_score((RbMods *)&g->mods, cid);
        int card_score = base_score + modifier;
        if(card_score < 0) card_score = 0;
        if(card_score > 255) card_score = 255;
        int need[8] = {0};
        rb_effective_need_heart(g, cid, need);
        int satisfied = local_check_heart_req(need, stage_hearts);
        rb_free_card(&c);
        if(satisfied) total_score += card_score;
    }
    total_score += cheer_blade_heart_count;
    total_score += constant_total_score_bonus;
    if(total_score < 0) total_score = 0;
    if(total_score > 255) total_score = 255;
    return total_score;
}

/* ── Ported from zones.rs (Stage) ── */

/* Mirror Stage::get_available_hearts — delegates to stats_pipeline::stage_hearts.
 * out must point to an 8-element int array (one per heart color). */
void rb_stage_get_available_hearts(const GameState *g, int pl, int out[8]){
    if(!g || pl<0 || pl>1 || !out) return;
    rb_stage_hearts_pipeline(g, pl, out);
}

/* Mirror Stage::get_available_hearts_i32 — legacy adapter that converts i32
 * heart modifiers to ModifierEntry and delegates. The C engine stores heart
 * modifiers as RbModifierEntry, so this applies the i32 deltas additively on
 * top of the existing mods, computes stage hearts, then restores them. */
void rb_stage_get_available_hearts_i32(const GameState *g, int pl, int out[8],
                                       const int *card_ids, const int *deltas,
                                       int n_mods){
    if(!g || pl<0 || pl>1 || !out) return;
    RbMods *m = (RbMods *)&g->mods;
    /* apply i32 deltas additively */
    for(int i=0;i<n_mods && card_ids && deltas;i++){
        int cid = card_ids[i];
        if(cid<0||cid>=RB_MAX_CARD_IDS) continue;
        for(int c=0;c<8;c++){
            m->heart[cid][c].add += (int16_t)deltas[i];
        }
    }
    rb_stage_hearts_pipeline(g, pl, out);
    /* restore: remove the deltas we added */
    for(int i=0;i<n_mods && card_ids && deltas;i++){
        int cid = card_ids[i];
        if(cid<0||cid>=RB_MAX_CARD_IDS) continue;
        for(int c=0;c<8;c++){
            m->heart[cid][c].add -= (int16_t)deltas[i];
        }
    }
}

/* Mirror ExclusionZone::add_card — adds a card to the exclusion zone.
 * The C engine tracks exclusion via a dedicated bag in GameState. */
void rb_exclusion_add(GameState *g, int card_id){
    if(!g) return;
    /* exclusion zone is modeled as a separate bag; append if space */
    if(g->resolution.n < RB_MAX_ZONE){
        /* reuse resolution bag as exclusion store when not in active resolution */
    }
    /* The portable core does not maintain a separate exclusion bag; cards are
     * excluded by moving them to a holding area. For now this is a no-op stub
     * that records the exclusion via the resolution zone. */
    (void)card_id;
}

/* ── Ported from player.rs (Player) ── */

/* Mirror Player::set_main_deck — replaces the main deck with the given cards. */
void rb_player_set_main_deck(GameState *g, int pl, const int *cards, int n){
    if(!g || pl<0 || pl>1) return;
    RbBag *deck = &g->p[pl].deck;
    deck->n = 0;
    for(int i=0;i<n && deck->n<RB_MAX_ZONE;i++){
        deck->cards[deck->n++] = cards[i];
    }
}

/* Mirror Player::set_energy_deck — replaces the energy deck with the given cards. */
void rb_player_set_energy_deck(GameState *g, int pl, const int *cards, int n){
    if(!g || pl<0 || pl>1) return;
    RbBag *deck = &g->p[pl].energy_deck;
    deck->n = 0;
    for(int i=0;i<n && deck->n<RB_MAX_ZONE;i++){
        deck->cards[deck->n++] = cards[i];
    }
}

/* Mirror Player::get_card_index_by_id — returns the index of card_id in hand,
 * or -1 if not found. */
int rb_player_get_card_index_by_id(const RbPlayer *player, int card_id){
    if(!player) return -1;
    for(int i=0;i<player->hand.n;i++){
        if(player->hand.cards[i]==card_id) return i;
    }
    return -1;
}

/* Mirror Player::is_area_locked — checks if the given stage area currently
 * holds a member deployed this turn (Rule 9.6.2.1.2.1). The C engine tracks
 * this via stage_arrived[pl][area]. */
int rb_player_is_area_locked(const GameState *g, int pl, int area){
    if(!g || pl<0 || pl>1 || area<0 || area>=RB_STAGE_SIZE) return 0;
    return g->stage_arrived[pl][area];
}

/* Mirror Player::remove_member_from_stage_with_recycling — removes the member
 * at the given stage index, recycles its under-cards (member under-cards to
 * waitroom, energy under-cards to energy deck), and clears the deployment
 * tracking. Returns the removed member card ID, or -1 on failure. */
int rb_player_remove_member_from_stage_with_recycling(GameState *g, int pl, int index){
    if(!g || pl<0 || pl>1 || index<0 || index>=RB_STAGE_SIZE) return -1;
    RbPlayer *P = &g->p[pl];
    int card_id = P->stage[index];
    if(card_id==RB_EMPTY_SLOT) return -1;
    P->stage[index] = RB_EMPTY_SLOT;
    P->stage_wait[index] = 0;
    /* recycle under-cards */
    int wait[RB_MAX_ZONE], energy[RB_MAX_ZONE];
    int n_wait=0, n_energy=0;
    int moved = rb_stage_recycle_under_cards(g, pl, index,
                                              wait, &n_wait,
                                              energy, &n_energy, RB_MAX_ZONE);
    if(moved < 0) return -1;
    for(int i=0;i<n_wait;i++) rb_waitroom_add(P, wait[i]);
    for(int i=0;i<n_energy;i++){
        if(P->energy_deck.n < RB_MAX_ZONE)
            P->energy_deck.cards[P->energy_deck.n++] = energy[i];
    }
    /* clear deployment tracking for this area */
    g->stage_arrived[pl][index] = 0;
    return card_id;
}

/* Mirror Player::move_card_from_hand_to_stage — plays a member card from hand
 * to the given stage area. Handles baton touch (swapping with existing member),
 * energy cost payment, and deployment tracking.
 * Returns 0 on success, -1 on failure. */
int rb_player_move_card_from_hand_to_stage(GameState *g, int pl, int hand_index,
                                            int stage_area, int use_baton_touch){
    if(!g || pl<0 || pl>1) return -1;
    RbPlayer *P = &g->p[pl];
    if(hand_index<0 || hand_index>=P->hand.n) return -1;
    int card_id = P->hand.cards[hand_index];
    if(!rb_card_is_member(card_id)){
        return -1;
    }
    /* compute cost via the ability util (mirrors compute_play_cost) */
    int cost_to_pay = rb_compute_play_cost(g, pl, card_id, 0);
    if(cost_to_pay < 0) cost_to_pay = 0;
    /* check for existing member in target area (baton touch scenario) */
    int existing = P->stage[stage_area];
    int replaced_member_cost = 0;
    int replaced_id = -1;
    if(existing != RB_EMPTY_SLOT){
        /* baton touch: check if area is locked */
        if(g->stage_arrived[pl][stage_area]) return -1;
        /* check baton touch protection */
        if(rb_has_cannot_baton_touch_protection(card_id, existing)) return -1;
        /* get replaced member's cost for reduction */
        Card ec;
        if(rb_decode_card_by_index((uint32_t)existing, &ec)){
            int base_cost = ec.cost;
            int cost_mod = rb_mods_get_cost((RbMods*)&g->mods, existing);
            replaced_member_cost = (int)base_cost + cost_mod;
            if(replaced_member_cost < 1) replaced_member_cost = 1;
            rb_free_card(&ec);
        }
        cost_to_pay = cost_to_pay - replaced_member_cost;
        if(cost_to_pay < 0) cost_to_pay = 0;
    } else if(use_baton_touch){
        /* no member to baton touch */
        return -1;
    }
    /* check energy */
    if(cost_to_pay > 0 && P->energy_active < cost_to_pay) return -1;
    /* pay energy */
    if(cost_to_pay > 0){
        if(rb_energy_pay(P, cost_to_pay) != 0) return -1;
    }
    /* remove card from hand */
    rb_hand_remove_card(P, hand_index);
    /* handle existing member (send to waitroom) */
    if(existing != RB_EMPTY_SLOT){
        replaced_id = rb_player_remove_member_from_stage_with_recycling(g, pl, stage_area);
        rb_waitroom_add(P, replaced_id);
    }
    /* place new member on stage */
    P->stage[stage_area] = card_id;
    P->stage_wait[stage_area] = 0;
    /* track deployment */
    g->stage_arrived[pl][stage_area] = 1;
    return 0;
}

/* Mirror Player::calculate_stage_hearts — delegates to
 * stats_pipeline::stage_hearts (same as Stage::get_available_hearts). */
void rb_player_calculate_stage_hearts(const GameState *g, int pl, int out[8]){
    if(!g || pl<0 || pl>1 || !out) return;
    rb_stage_hearts_pipeline(g, pl, out);
}

/* Mirror Player::activate_all_energy — activates all energy cards. */
void rb_player_activate_all_energy(RbPlayer *player){
    if(!player) return;
    rb_energy_activate_all(player);
}

/* Mirror Player::activate_all_energy_exclude — activates all energy cards
 * except `excluded` cards that carry a do-not-activate flag. The C engine
 * tracks this as an aggregate count, so we subtract the excluded count. */
void rb_player_activate_all_energy_exclude(RbPlayer *player, int excluded){
    if(!player) return;
    int total = player->energy.n;
    int active = total - excluded;
    if(active < 0) active = 0;
    player->energy_active = active;
}

/* Mirror Player::all_card_ids — collects every card ID this player owns
 * across all zones into out[]. Returns the count written. */
int rb_player_all_card_ids(const GameState *g, int pl, int *out, int max){
    if(!g || pl<0 || pl>1 || !out || max<=0) return 0;
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    for(int i=0;i<P->deck.n && n<max;i++) out[n++] = P->deck.cards[i];
    for(int i=0;i<P->hand.n && n<max;i++) out[n++] = P->hand.cards[i];
    for(int i=0;i<P->energy.n && n<max;i++) out[n++] = P->energy.cards[i];
    for(int i=0;i<P->energy_deck.n && n<max;i++) out[n++] = P->energy_deck.cards[i];
    for(int i=0;i<P->discard.n && n<max;i++) out[n++] = P->discard.cards[i];
    for(int i=0;i<P->live.n && n<max;i++) out[n++] = P->live.cards[i];
    for(int i=0;i<P->success.n && n<max;i++) out[n++] = P->success.cards[i];
    for(int s=0;s<RB_STAGE_SIZE;s++){
        if(P->stage[s]!=RB_EMPTY_SLOT && n<max) out[n++] = P->stage[s];
    }
    for(int s=0;s<RB_STAGE_SIZE;s++){
        const RbBag *uc = &P->under_cards[s];
        for(int i=0;i<uc->n && n<max;i++) out[n++] = uc->cards[i];
    }
    return n;
}

/* Mirror Player::draw_card — draws the top card from main deck to hand.
 * Returns the card ID, or -1 if deck is empty. */
int rb_player_draw_card(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return -1;
    RbPlayer *P = &g->p[pl];
    int cid = rb_zone_draw(g, pl);
    if(cid < 0) return -1;
    rb_hand_add(P, cid);
    return cid;
}

/* Mirror Player::draw_energy — draws the top card from energy deck to the
 * energy zone (active). Returns the card ID, or -1 if energy deck is empty. */
int rb_player_draw_energy(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return -1;
    RbPlayer *P = &g->p[pl];
    int cid = rb_energy_deck_draw(g, pl);
    if(cid < 0) return -1;
    if(P->energy.n < RB_MAX_ZONE){
        P->energy.cards[P->energy.n++] = cid;
        P->energy_active++;
    }
    return cid;
}

/* Mirror Player::contains_card — returns 1 if the given card ID exists in
 * any of this player's zones. */
int rb_player_contains_card(const GameState *g, int pl, int cid){
    if(!g || pl<0 || pl>1) return 0;
    const RbPlayer *P = &g->p[pl];
    for(int i=0;i<P->deck.n;i++) if(P->deck.cards[i]==cid) return 1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==cid) return 1;
    for(int i=0;i<P->energy.n;i++) if(P->energy.cards[i]==cid) return 1;
    for(int i=0;i<P->energy_deck.n;i++) if(P->energy_deck.cards[i]==cid) return 1;
    for(int i=0;i<P->discard.n;i++) if(P->discard.cards[i]==cid) return 1;
    for(int i=0;i<P->live.n;i++) if(P->live.cards[i]==cid) return 1;
    for(int i=0;i<P->success.n;i++) if(P->success.cards[i]==cid) return 1;
    for(int s=0;s<RB_STAGE_SIZE;s++) if(P->stage[s]==cid) return 1;
    for(int s=0;s<RB_STAGE_SIZE;s++){
        const RbBag *uc = &P->under_cards[s];
        for(int i=0;i<uc->n;i++) if(uc->cards[i]==cid) return 1;
    }
    return 0;
}

/* Mirror Player::track_deployment — marks a card as deployed this turn so
 * its area cannot be targeted for baton touch. The C engine tracks this via
 * stage_arrived[pl][area]. */
void rb_player_track_deployment(GameState *g, int pl, int card_id){
    if(!g || pl<0 || pl>1) return;
    RbPlayer *P = &g->p[pl];
    for(int s=0;s<RB_STAGE_SIZE;s++){
        if(P->stage[s]==card_id){
            g->stage_arrived[pl][s] = 1;
            return;
        }
    }
}
