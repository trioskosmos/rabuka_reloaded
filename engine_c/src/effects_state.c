#include "rabuka.h"
#include <string.h>

/* State / cost / compound handlers — mirrors
   engine/src/ability/effects/state.rs + misc.rs + compound.rs */

void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e){
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    const char *st=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"state")) st=e->extra_v[i];
    if(!st) for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"to_state")) st=e->extra_v[i];
    if(!st) st="wait";
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){
        P->stage_wait[q]=(!strcmp(st,"wait"))?1:0;
        rb_mods_set_orientation(&g->mods, P->stage[q], st);
        break;
    }
}

void rb_effect_position_change(GameState *g, int actor, AbilityEffect *e){
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    (void)e;
    /* Formation swap batch: simple left<->right as portable core */
    if(P->stage[0]!=RB_EMPTY_SLOT && P->stage[2]!=RB_EMPTY_SLOT){
        int tmp=P->stage[0]; P->stage[0]=P->stage[2]; P->stage[2]=tmp;
        int tmpw=P->stage_wait[0]; P->stage_wait[0]=P->stage_wait[2]; P->stage_wait[2]=tmpw;
    }
}

void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e){
    int cnt=e->count>=0?e->count:1;
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    int target=-1;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){ target=P->stage[q]; break; }
    if(target==-1 && P->hand.n>0) target=P->hand.cards[0];
    if(target==-1) return;
    if(e->action && (!strcmp(e->action,"set_cost")||!strcmp(e->action,"set_cost_to_use")))
        rb_mods_set_cost(&g->mods, target, cnt);
    else
        rb_mods_add_cost(&g->mods, target, cnt);
}

void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e){
    int cnt=e->count>=0?e->count:1;
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    int col=0;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_color")){
        const char *hc=e->extra_v[i];
        if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) col=0;
        else if(!strcmp(hc,"red")) col=1;
        else if(!strcmp(hc,"yellow")) col=2;
        else if(!strcmp(hc,"green")) col=3;
        else if(!strcmp(hc,"blue")) col=4;
        else if(!strcmp(hc,"purple")) col=5;
        else if(!strcmp(hc,"orange")) col=6;
        else if(!strcmp(hc,"all")) col=7;
    }
    int target=-1;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){ target=P->stage[q]; break; }
    if(target==-1 && P->hand.n>0) target=P->hand.cards[0];
    if(target!=-1) rb_mods_add_need_heart(&g->mods, target, col%8, cnt);
}
