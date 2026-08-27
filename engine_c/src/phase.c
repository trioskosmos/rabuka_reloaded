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
        g->phase=RB_PHASE_MAIN;
        return;
    }
    if(g->phase==RB_PHASE_MAIN){
        /* In the two-attacker model, after first attacker's Main we flip
           active to second attacker and re-enter Active. If this was already
           the second attacker, proceed to LiveSet. */
        static int main_count=0;
        main_count++;
        if(g->active==g->first_attacker){
            g->active=g->second_attacker;
            g->phase=RB_PHASE_ACTIVE;
        } else {
            g->active=g->first_attacker; /* Live first_attacker starts */
            g->phase=RB_PHASE_LIVE_SET;
        }
        if(main_count>=2) main_count=0;
        return;
    }
    if(g->phase==RB_PHASE_LIVE_SET){
        g->phase=RB_PHASE_PERFORMANCE;
        return;
    }
    if(g->phase==RB_PHASE_PERFORMANCE){
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
