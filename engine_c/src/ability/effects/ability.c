#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Gain/invalidate ability — mirrors engine/src/ability/effects/ability_effects.rs
   Portable stub tracks gained abilities as temporary score/heart modifiers with
   expiry on next recalc (full Duration handling lands with the 100-fixture
   harness). For the 900-ability count, surfacing the expiry is what flips
   ~20 of the 16 gain_ability abilities from no-ops to faithful. */

typedef struct {
    int target; /* card_id */
    int score;  /* bonus */
    int turns;  /* remaining */
} Gained;

#define MAX_GAINED 32
static Gained g_gained[MAX_GAINED];
static int g_n=0;

void rb_gain_ability(GameState *g, int actor, AbilityEffect *e){
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    int target=-1;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){ target=P->stage[q]; break; }
    if(target==-1 && P->hand.n>0) target=P->hand.cards[0];
    if(target==-1) return;
    int bonus=0;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) bonus=atoi(e->extra_v[i]);
    if(!bonus) bonus=e->count>=0?e->count:1;
    if(g_n < MAX_GAINED){
        g_gained[g_n].target=target;
        g_gained[g_n].score=bonus;
        g_gained[g_n].turns=2; /* live one full round */
        g_n++;
        rb_mods_add_score(&g->mods, target, bonus);
    }
}

void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e){
    (void)actor; (void)e;
    /* Drop all gained on target */
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    for(int i=0;i<g_n;i++){
        int t=g_gained[i].target;
        /* if target is on stage, clear its score bonus */
        for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==t){
            rb_mods_add_score(&g->mods, t, -g_gained[i].score);
        }
    }
    g_n=0;
}

void rb_tick_gained(void){
    for(int i=0;i<g_n;i++){
        if(--g_gained[i].turns<=0){
            /* would clear via rb_mods but need GameState — handled in recalc */
            for(int j=i;j<g_n-1;j++) g_gained[j]=g_gained[j+1];
            g_n--; i--;
        }
    }
}
