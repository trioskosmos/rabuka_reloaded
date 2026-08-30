#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Gain/invalidate ability — mirrors engine/src/ability/effects/ability_effects.rs.
   Tracks gained abilities as temporary score/blade/heart/need_heart modifiers
   with expiry on next recalc (full Duration handling lands with the 100-fixture
   harness). For the 900-ability count, surfacing the expiry is what flips
   ~20 of the gain_ability abilities from no-ops to faithful. */

typedef struct {
    int target; /* card_id */
    int score;  /* bonus */
    int blade;
    int heart;
    int need_heart;
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
    int score=0, blade=0, heart=0, need=0;
    for(int i=0;i<e->n_extra;i++){
        if(!e->extra_k[i]) continue;
        if(!strcmp(e->extra_k[i],"value"))      score=atoi(e->extra_v[i]);
        else if(!strcmp(e->extra_k[i],"blade"))  blade=atoi(e->extra_v[i]);
        else if(!strcmp(e->extra_k[i],"heart"))  heart=atoi(e->extra_v[i]);
        else if(!strcmp(e->extra_k[i],"need_heart")) need=atoi(e->extra_v[i]);
    }
    if(!score) score=e->count>=0?e->count:1;
    if(g_n < MAX_GAINED){
        Gained *gg=&g_gained[g_n++];
        gg->target=target; gg->score=score; gg->blade=blade; gg->heart=heart;
        gg->need_heart=need; gg->turns=2; /* live one full round */
        if(score) rb_mods_add_score(&g->mods, target, score);
        if(blade) rb_mods_add_blade(&g->mods, target, blade);
        if(heart) rb_mods_add_heart(&g->mods, target, 0, heart);
        if(need)  rb_mods_add_need_heart(&g->mods, target, 0, need);
    }
}

void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e){
    (void)e;
    /* Mirror ability_effects.rs::execute_invalidate_ability — revoke every gained
        ability owned by the targeted player (revert its score/blade/heart/need
        bonus, then drop). */
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    for(int i=g_n-1;i>=0;i--){
        int t=g_gained[i].target;
        if(rb_owner_of_card(g, t) == who){
            if(g_gained[i].score) rb_mods_add_score(&g->mods, t, -g_gained[i].score);
            if(g_gained[i].blade) rb_mods_add_blade(&g->mods, t, -g_gained[i].blade);
            if(g_gained[i].heart) rb_mods_add_heart(&g->mods, t, 0, -g_gained[i].heart);
            if(g_gained[i].need_heart) rb_mods_add_need_heart(&g->mods, t, 0, -g_gained[i].need_heart);
            for(int j=i;j<g_n-1;j++) g_gained[j]=g_gained[j+1];
            g_n--;
        }
    }
}

void rb_tick_gained(GameState *g){
    if(!g) return;
    for(int i=0;i<g_n;i++){
        if(--g_gained[i].turns<=0){
            /* Mirror TemporaryEffect expiry: revert the granted modifiers
                on the target card so the bonus does not leak past its duration. */
            int t=g_gained[i].target;
            if(g_gained[i].score) rb_mods_add_score(&g->mods, t, -g_gained[i].score);
            if(g_gained[i].blade) rb_mods_add_blade(&g->mods, t, -g_gained[i].blade);
            if(g_gained[i].heart) rb_mods_add_heart(&g->mods, t, 0, -g_gained[i].heart);
            if(g_gained[i].need_heart) rb_mods_add_need_heart(&g->mods, t, 0, -g_gained[i].need_heart);
            for(int j=i;j<g_n-1;j++) g_gained[j]=g_gained[j+1];
            g_n--; i--;
        }
    }
}

/* Mirror ability_effects.rs::execute_activate_ability. The common path is
   source_card=="previous_selected": fire the matching-trigger ability of every
   card in g->selected_cards (default trigger 登場/Debut). Fallback: fire the
   activating card's own ability effect. */
void rb_activate_ability_effect(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *source = NULL;
    const char *trigger = NULL;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"source_card")) source=e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"target_trigger")) trigger=e->extra_v[i];
    }
    if(!trigger && e->target && strstr(e->target,"登場")) trigger="登場";

    int src_ids[RB_MAX_RECENTLY_MOVED]; int ns=0;
    if(source && !strcmp(source,"previous_selected")){
        for(int i=0;i<g->n_selected_cards && ns<RB_MAX_RECENTLY_MOVED;i++)
            src_ids[ns++]=g->selected_cards[i];
    }
    for(int i=0;i<ns;i++){
        int cid=src_ids[i];
        Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
        AbilityEffect *fx = (c.ability && c.ability->effect) ? c.ability->effect : NULL;
        int match = fx && (!trigger || (c.ability->triggers && strstr(c.ability->triggers, trigger)));
        if(match) rb_execute_effect_ex(g, actor, fx, cid);
        rb_free_card(&c);
    }
    if(ns==0){
        /* Fallback: fire the activating card's own ability effect if present. */
        int cid = host_cid;
        if(cid < 0) for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]>=0){ cid=g->p[actor].stage[q]; break; }
        if(cid>=0){
            Card c; if(rb_decode_card_by_index((uint32_t)cid,&c)){
                if(c.ability && c.ability->effect) rb_execute_effect_ex(g, actor, c.ability->effect, cid);
                rb_free_card(&c);
            }
        }
    }
}

/* Mirror ability_effects.rs::execute_gain_ability_from_source. Copy the ability
   effect of a matching source card (found under the activating card) onto the
   activating card by executing that source's ability effect on the activating
   card. Bounded: first matching under-card with the requested group filter. */
void rb_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int cid = host_cid;
    if(cid < 0) for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]>=0){ cid=g->p[actor].stage[q]; break; }
    if(cid < 0) return;
    const char *grp=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names")) grp=e->extra_v[i];
    RbPlayer *P=&g->p[actor];
    int area=-1;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==cid){ area=q; break; }
    if(area<0) return;
    for(int u=0;u<P->under_cards[area].n;u++){
        int src=P->under_cards[area].cards[u];
        Card sc; if(!rb_decode_card_by_index((uint32_t)src,&sc)) continue;
        int ok=1;
        if(grp && !(sc.group_idx>=0 && rb_card_matches_group_str(src, grp))) ok=0;
        if(ok && sc.ability && sc.ability->effect)
            rb_execute_effect_ex(g, actor, sc.ability->effect, cid);
        rb_free_card(&sc);
    }
}
