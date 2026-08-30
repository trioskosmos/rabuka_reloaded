#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Ported from engine/src/ability/effects/score.rs
   Simplified: handles the common operation="add" with value and target.
   Full per-unit / group / duration logic lands with the 100-fixture audit. */

int rb_execute_modify_score(GameState *gs, int actor, AbilityEffect *e){
    if(!gs || !e) return -1;
    const char *op = "add";
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"operation")) op=e->extra_v[i];
    int value = 0;
    if(e->count>=0) value = e->count;
    else {
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) value = atoi(e->extra_v[i]);
    }
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if(target && (!strcmp(target,"opponent")||!strcmp(target,"p2"))) pl = actor^1;

    /* resolve target card(s): for now apply to the activating card or all stage */
    int cids[RB_STAGE_SIZE]; int n=0;
    if(!strcmp(target,"self") || !strcmp(target,"target")){
        /* host card is the activating card; caller should have set gs->queue.resume_host etc.
           Fallback: apply to stage members */
        for(int i=0;i<RB_STAGE_SIZE;i++) if(gs->p[pl].stage[i]!=RB_EMPTY_SLOT) cids[n++]=gs->p[pl].stage[i];
        if(n==0 && gs->queue.resume_host>=0) cids[n++]=gs->queue.resume_host;
    } else {
        for(int i=0;i<RB_STAGE_SIZE;i++) if(gs->p[pl].stage[i]!=RB_EMPTY_SLOT) cids[n++]=gs->p[pl].stage[i];
    }
    for(int i=0;i<n;i++){
        if(!strcmp(op,"add")||!strcmp(op,"increase")){
            rb_mods_add_score(&gs->mods, cids[i], (int16_t)value);
        } else if(!strcmp(op,"set")){
            rb_mods_set_score(&gs->mods, cids[i], (int16_t)value);
        } else {
            rb_mods_add_score(&gs->mods, cids[i], (int16_t)value);
        }
    }
    return 0;
}
