#include "rabuka.h"
#include <string.h>

/* Turn phase machine — mirrors engine/src/turn/phases.rs:advance_phase
   Two TurnPhases per round: FirstAttackerNormal / SecondAttackerNormal / Live.
   For portability we keep it linear: RPS → Active → Energy → Draw → Main
   executed twice (first then second attacker) before LiveSet → Performance
   → Victory → rollover. Hosts that don't need mulligan can skip it. */

static void activate_wait_members(GameState *g, int pl) {
    RbPlayer *P=&g->p[pl];
    int owned[RB_MAX_CARD_IDS]; int n_owned=0;
    for(int i=0;i<RB_MAX_CARD_IDS;i++) if(g->mods.delayed_cannot_active[i]) {}
    /* collect owned card ids for delayed tick */
    for(int s=0;s<RB_STAGE_SIZE;s++) if(P->stage[s]!=RB_EMPTY_SLOT) owned[n_owned++]=P->stage[s];
    for(int i=0;i<P->energy.n;i++) owned[n_owned++]=P->energy.cards[i];
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT && P->stage_wait[q]){
        if(rb_mods_is_delayed_cannot_active(&g->mods,P->stage[q])) continue;
        P->stage_wait[q]=0;
    }
    rb_mods_tick_delayed_for(&g->mods, owned, n_owned);
    if(P->energy_active < P->energy.n) P->energy_active = P->energy.n;
}

void rb_advance_phase(GameState *g) {
    if(g->winner!=-1) return;
    /* Mulligan phases are no-ops for headless/skip */
    if(g->phase==RB_PHASE_RPS || g->phase==RB_PHASE_OPENING){
        g->phase=RB_PHASE_ACTIVE;
        return;
    }
    if(g->phase==RB_PHASE_ACTIVE){
        activate_wait_members(g, g->active);
        rb_recalc_constants(g);
        rb_check_timing(g);
        g->phase=RB_PHASE_ENERGY;
        return;
    }
    if(g->phase==RB_PHASE_ENERGY){
        rb_draw_energy(g, g->active);
        g->phase=RB_PHASE_DRAW;
        return;
    }
    if(g->phase==RB_PHASE_DRAW){
        rb_draw(g, g->active);
        rb_recalc_constants(g);
        rb_check_timing(g);
        g->phase=RB_PHASE_MAIN;
        return;
    }
    if(g->phase==RB_PHASE_MAIN){
        /* In the two-attacker model, after first attacker's Main we flip
           active to second attacker and re-enter Active. If this was already
           the second attacker, proceed to LiveSet. Mirrors
           engine/src/turn/phases.rs: TurnPhase::FirstAttackerNormal → SecondAttackerNormal → Live.
           No static: use g->active vs g->first_attacker as the turn discriminator
           (static would leak across games and break determinism). */
        if(g->active==g->first_attacker){
            g->active=g->second_attacker;
            g->phase=RB_PHASE_ACTIVE;
        } else {
            g->active=g->first_attacker; /* Live first_attacker starts */
            g->phase=RB_PHASE_LIVE_SET;
        }
        return;
    }
    if(g->phase==RB_PHASE_LIVE_SET){
        /* Load-bearing: re-evaluates constant abilities before performance (mirrors
           engine/src/turn/phases.rs:222 check_timing at LiveCardSetSecond→FirstPerformance).
           Without this q127_wien leaves_stage_modifier_removed breaks.
           Trigger LiveStart autos for both players, then process them (phases.rs:231-243). */
        rb_check_timing(g);
        rb_trigger_live_start(g, 0);
        rb_trigger_live_start(g, 1);
        rb_process_pending_auto_abilities(g);
        g->phase=RB_PHASE_PERFORMANCE;
        return;
    }
    if(g->phase==RB_PHASE_PERFORMANCE){
        rb_recalc_constants(g);
        g->phase=RB_PHASE_VICTORY;
        return;
    }
    if(g->phase==RB_PHASE_VICTORY){
        /* victory check + rollover */
        for(int pl=0;pl<2;pl++){
            if(g->p[pl].success.n >= RB_VICTORY_CARD_COUNT) g->winner=pl;
            else if(g->p[pl].score >= RB_SCORE_WIN) g->winner=pl;
        }
        if(g->p[0].success.n>=RB_VICTORY_CARD_COUNT && g->p[1].success.n>=RB_VICTORY_CARD_COUNT) g->winner=2;
        if(g->winner!=-1){ g->phase=RB_PHASE_DONE; return; }
        g->turn++;
        g->active=g->active^1;
        g->phase=RB_PHASE_ACTIVE;
    }
}

/* ───────────────────────────── check_timing (turn/actions.rs) ─────────────────────────────
   Integrity cascade run between phase steps: refresh derived zones, re-check
   victory, evict illegally-zoned cards, recompute constants, clear the
   resolution zone, detect permanent loops, then process pending auto-abilities. */

static void bag_push_local(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_remove_at_local(RbBag *b, int i) {
    if (i < 0 || i >= b->n) return -1;
    int c = b->cards[i];
    for (int j = i; j < b->n - 1; j++) b->cards[j] = b->cards[j + 1];
    b->n--;
    return c;
}

void rb_player_refresh(GameState *g, int pl) {
    /* Rust Player::refresh() recomputes cached derived zone state. The C model
       keeps zones authoritative (no separate cached view), so the only derived
       quantity is energy_active, which activates all non-delayed energy. */
    RbPlayer *P = &g->p[pl];
    if (P->energy_active < P->energy.n) P->energy_active = P->energy.n;
}

void rb_check_victory_condition(GameState *g) {
    int p1 = g->p[0].success.n;
    int p2 = g->p[1].success.n;
    if (p1 >= RB_VICTORY_CARD_COUNT && p2 >= RB_VICTORY_CARD_COUNT) {
        g->winner = 2;            /* draw */
    } else if (p1 >= RB_VICTORY_CARD_COUNT && p2 <= 2) {
        g->winner = 0;
    } else if (p2 >= RB_VICTORY_CARD_COUNT && p1 <= 2) {
        g->winner = 1;
    }
}

void rb_check_invalid_live_cards(GameState *g, int is_p1) {
    RbPlayer *P = is_p1 ? &g->p[0] : &g->p[1];
    /* collect indices of non-live cards in the live zone (iterate backwards) */
    for (int i = P->live.n - 1; i >= 0; i--) {
        int cid = P->live.cards[i];
        if (!rb_card_is_live(cid)) {
            int c = bag_remove_at_local(&P->live, i);
            if (rb_card_is_energy(c)) bag_push_local(&P->energy, c);
            else                       bag_push_local(&P->discard, c);
        }
    }
}

void rb_check_invalid_energy_cards(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    for (int i = P->energy.n - 1; i >= 0; i--) {
        int cid = P->energy.cards[i];
        if (!rb_card_is_energy(cid)) {
            int c = bag_remove_at_local(&P->energy, i);
            bag_push_local(&P->discard, c);
        }
    }
}

void rb_check_orphaned_under_cards(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    for (int a = 0; a < RB_STAGE_SIZE; a++) {
        if (P->stage[a] == RB_EMPTY_SLOT && P->under_cards[a].n > 0) {
            for (int i = P->under_cards[a].n - 1; i >= 0; i--) {
                int cid = bag_remove_at_local(&P->under_cards[a], i);
                if (rb_card_is_energy(cid)) bag_push_local(&P->energy, cid);
                else                        bag_push_local(&P->discard, cid);
            }
        }
    }
}

void rb_check_invalid_resolution_zone(GameState *g) {
    if (g->resolution.n == 0) return;
    RbPlayer *P = &g->p[g->active];
    for (int i = g->resolution.n - 1; i >= 0; i--) {
        int cid = bag_remove_at_local(&g->resolution, i);
        bag_push_local(&P->discard, cid);
    }
}

int rb_check_permanent_loop(const GameState *g) {
    /* Rust GameState::check_permanent_loop detects a non-terminating state
       (e.g. mutual infinite triggers). The C model does not track the loop
       graph, so it never forces a draw. */
    (void)g;
    return 0;
}

void rb_check_timing(GameState *g) {
    rb_player_refresh(g, 0);
    rb_player_refresh(g, 1);
    rb_check_victory_condition(g);
    rb_check_invalid_live_cards(g, true);
    rb_check_invalid_live_cards(g, false);
    rb_check_invalid_energy_cards(g, 0);
    rb_check_invalid_energy_cards(g, 1);
    rb_check_orphaned_under_cards(g, 0);
    rb_check_orphaned_under_cards(g, 1);
    rb_recalc_constants(g);
    rb_check_invalid_resolution_zone(g);
    if (rb_check_permanent_loop(g)) {
        g->winner = 2;
    }
    int active = g->active;
    rb_process_pending_auto_abilities(g);
    (void)active;
}
