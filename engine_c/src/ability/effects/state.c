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
    /* which members: explicit position, "all", or first occupied. */
    const char *pos=NULL;
    int all=0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"position")) pos=e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"all") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")) all=1;
    }
    if(e->target && !pos && !strcmp(e->target,"all")) all=1;
    int apply_pos = pos ? rb_pos_to_area(pos) : -1;
    for(int q=0;q<RB_STAGE_SIZE;q++){
        if(P->stage[q]==RB_EMPTY_SLOT) continue;
        if(!all && apply_pos>=0 && apply_pos!=q) continue;
        if(!all && apply_pos<0 && q!=(RB_STAGE_SIZE==3?1:0)) {
            /* no explicit target: act on the first member only (break after) */
        }
        P->stage_wait[q]=(!strcmp(st,"wait"))?1:0;
        /* "rest" sets the rest orientation; orientation mod stores the string verbatim */
        rb_mods_set_orientation(&g->mods, P->stage[q], st);
        if(!all && apply_pos<0) break; /* first-member-only default */
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
    /* Faithful target: apply to every staged member + hand-visible costs.
       Rust's cost_modifiers are per-card and constant abilities recalc via
       recalc_constants; here we mirror by applying to all owned members so
       later draws also see the modifier via the card_id entry. The old
       first-staged-only path was a P0/P1 coverage hole (modify_cost appeared
       to work but only for one member). */
    int any=0;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){
        int cid=P->stage[q];
        if(e->action && (!strcmp(e->action,"set_cost")||!strcmp(e->action,"set_cost_to_use")))
            rb_mods_set_cost(&g->mods, cid, cnt);
        else
            rb_mods_add_cost(&g->mods, cid, cnt);
        any=1;
    }
    if(!any){
        for(int i=0;i<P->hand.n;i++){
            int cid=P->hand.cards[i];
            if(e->action && (!strcmp(e->action,"set_cost")||!strcmp(e->action,"set_cost_to_use")))
                rb_mods_set_cost(&g->mods, cid, cnt);
            else
                rb_mods_add_cost(&g->mods, cid, cnt);
        }
        for(int i=0;i<P->deck.n;i++){
            int cid=P->deck.cards[i];
            if(e->action && (!strcmp(e->action,"set_cost")||!strcmp(e->action,"set_cost_to_use")))
                rb_mods_set_cost(&g->mods, cid, cnt);
            else
                rb_mods_add_cost(&g->mods, cid, cnt);
        }
    }
    /* modify_yell_count / modify_yell_source are per-player yell modifiers.
       Store as a cost-like entry on a synthetic id (0) and let live.c's
       do_yell read them — minimal portable wire until a full yell mods table lands. */
    if(e->action && !strcmp(e->action,"modify_yell_count")){
        /* Mirror misc.rs modify_yell_count — add `count` to the per-live yell
           card count; live.c do_yell reads g->yell_count_mod[pl]. */
        g->yell_count_mod[who] += cnt;
    } else if(e->action && !strcmp(e->action,"modify_yell_source")){
        /* modify_yell_source changes which cards are yelled; headless yell uses
           the deck top regardless of source, so this is a documented no-op. */
        (void)cnt;
    }
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
