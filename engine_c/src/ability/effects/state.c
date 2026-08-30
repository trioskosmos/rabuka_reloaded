#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* State / cost / compound handlers — mirrors
   engine/src/ability/effects/state.rs + misc.rs + compound.rs */

void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e){
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

    /* Mirror misc.rs::execute_change_state — "both" flips orientation on BOTH
        players' stages; otherwise self (or opponent when target=="opponent"). */
    int players[2]; int np=0;
    if(e->target && !strcmp(e->target,"both")){ players[np++]=actor; players[np++]=actor^1; }
    else if(e->target && !strcmp(e->target,"opponent")){ players[np++]=actor^1; }
    else { players[np++]=actor; }

    for(int pk=0; pk<np; pk++){
        RbPlayer *P=&g->p[players[pk]];
        for(int q=0;q<RB_STAGE_SIZE;q++){
            if(P->stage[q]==RB_EMPTY_SLOT) continue;
            if(!all && apply_pos>=0 && apply_pos!=q) continue;
            int ocid = P->stage[q];
            const char *old_ori = rb_mods_get_orientation((RbMods*)&g->mods, ocid);
            int was_wait = old_ori && !strcmp(old_ori, "wait");
            int will_wait = (!strcmp(st, "wait")) ? 1 : 0;
            if (was_wait != will_wait) {
                /* record the transition for state_change_condition */
                g->state_change_from[ocid] = (int8_t)(was_wait ? 1 : 0);
                g->state_change_to[ocid]   = (int8_t)(will_wait ? 1 : 0);
                if (was_wait && !will_wait) g->last_wait_to_active_count++;
            }
            P->stage_wait[q] = will_wait;
            /* "rest" sets the rest orientation; orientation mod stores the string verbatim */
            rb_mods_set_orientation(&g->mods, ocid, st);
            if(!all && apply_pos<0) break; /* first-member-only default */
        }
    }
}

static void rb_record_movement(GameState *g, int cid){
    if(cid<0) return;
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED){
        g->recently_moved[g->n_recently_moved++]=cid;
    } else {
        for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++) g->recently_moved[i-1]=g->recently_moved[i];
        g->recently_moved[RB_MAX_RECENTLY_MOVED-1]=cid;
    }
}

/* Resolve one player's stage swap. Rust's position_change(from,to) SWAPS the two
   stage slots (the member at the destination moves to the source). Mirrors
   misc.rs::execute_position_change_with_destination core. */
static void rb_pos_change_for_player(GameState *g, int who, AbilityEffect *e, int host_cid){
    const char *src_pos=NULL, *dst_pos=NULL, *target_member=NULL;
    for(int i=0;i<e->n_extra;i++){
        if(!e->extra_k[i]) continue;
        if(!strcmp(e->extra_k[i],"source_position")) src_pos=e->extra_v[i];
        else if(!strcmp(e->extra_k[i],"destination")||!strcmp(e->extra_k[i],"dest_position")) dst_pos=e->extra_v[i];
        else if(!strcmp(e->extra_k[i],"target_member")) target_member=e->extra_v[i];
    }
    if(!dst_pos && e->destination && *e->destination) dst_pos=e->destination;
    if(!dst_pos){
        /* No predetermined destination → interactive: ask the host to pick a
           stage area to swap with (candidates listed left→center→right so the
           transpiler's "left"→index 0 mapping holds). */
        if(g->queue.resume_active) return;   /* already resolving; don't re-emit */
        int cands[RB_STAGE_SIZE]; int nc=0;
        for(int i=0;i<RB_STAGE_SIZE;i++) cands[nc++]=i;
        rb_emit_choice(g, who, RB_CHOICE_SELECT_TARGET, NULL, NULL, nc, 0, "position_change");
        g->queue.resume_mode = 1; g->queue.resume_eff = e;
        g->queue.resume_actor = who; g->queue.resume_host = host_cid;
        return;
    }
    if(!strcmp(dst_pos,"same_area")) return;
    int dst=rb_pos_to_area(dst_pos);
    if(dst<0) return;
    RbPlayer *P=&g->p[who];

    if(src_pos){
        int src=rb_pos_to_area(src_pos);
        if(src<0||src==dst) return;
        if(P->stage[src]<0) return;      /* no member at source → nothing to move */
        int a=P->stage[src], b=P->stage[dst];
        P->stage[src]=b; P->stage_wait[src]=P->stage_wait[dst];
        P->stage[dst]=a; P->stage_wait[dst]=P->stage_wait[src];
        rb_record_movement(g,a);
        if(b>=0) rb_record_movement(g,b);
        rb_recalc_constants(g);
        return;
    }
    if(target_member && strcmp(target_member,"this_member")){
        int cid=rb_find_card_by_no(target_member);
        int cur=-1;
        for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==cid){cur=i;break;}
        if(cur<0||cur==dst) return;
        int a=P->stage[cur], b=P->stage[dst];
        P->stage[cur]=b; P->stage_wait[cur]=P->stage_wait[dst];
        P->stage[dst]=a; P->stage_wait[dst]=P->stage_wait[cur];
        rb_record_movement(g,a);
        if(b>=0) rb_record_movement(g,b);
        rb_recalc_constants(g);
        return;
    }
    /* No source/target_member: interactive (per-member choice) — emitted above. */
}

void rb_effect_position_change(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *t = (e->target && *e->target) ? e->target : "self";
    /* Mirror GameState::position_change_occurred_this_turn — set the per-player flag
        so temporal conditions ("このターンに配置が変化している") can gate on it. */
    g->position_change_occurred_this_turn = 1; /* scalar per-game flag (mirrors GameState) */
    if(!strcmp(t,"both") || !strcmp(t,"self"))  rb_pos_change_for_player(g, actor, e, host_cid);
    if(!strcmp(t,"both") || !strcmp(t,"opponent")) rb_pos_change_for_player(g, actor^1, e, host_cid);
}

void rb_resume_position_change(GameState *g, int actor, const AbilityEffect *e, int host_cid, int selected_idx){
    (void)e;
    RbPlayer *P=&g->p[actor];
    int dst = selected_idx;             /* candidate index → stage area (0/1/2) */
    if(dst<0 || dst>=RB_STAGE_SIZE) return;
    int src=-1;
    for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==host_cid){src=i;break;}
    if(src<0 || src==dst) return;
    int a=P->stage[src], b=P->stage[dst];
    P->stage[src]=b; P->stage_wait[src]=P->stage_wait[dst];
    P->stage[dst]=a; P->stage_wait[dst]=P->stage_wait[src];
    rb_record_movement(g,a);
    if(b>=0) rb_record_movement(g,b);
    rb_recalc_constants(g);
}

/* Cyclic stage rotation — mirrors misc.rs::execute_rotation (rotation_map=[2,0,1]:
   left(0)->right(2), center(1)->left(0), right(2)->center(1)). Deterministic. */
void rb_effect_rotation(GameState *g, int actor, AbilityEffect *e){
    const char *t = (e->target && *e->target) ? e->target : "self";
    if(!strcmp(t,"both")) t = "self";       /* Rust: "both" resolves to self only */
    int who = (!strcmp(t,"opponent")) ? (actor^1) : actor;
    RbPlayer *P=&g->p[who];
    int snap[RB_STAGE_SIZE], wait[RB_STAGE_SIZE];
    RbBag under[RB_STAGE_SIZE];
    for(int i=0;i<RB_STAGE_SIZE;i++){
        snap[i]=P->stage[i]; wait[i]=P->stage_wait[i];
        under[i]=P->under_cards[i];
        P->stage[i]=RB_EMPTY_SLOT; P->stage_wait[i]=0;
    }
    static const int map[RB_STAGE_SIZE] = {2,0,1};
    for(int src=0;src<RB_STAGE_SIZE;src++){
        if(snap[src]<0) continue;
        int dst=map[src];
        P->stage[dst]=snap[src];
        P->stage_wait[dst]=wait[src];
        P->under_cards[dst]=under[src];
        rb_record_movement(g, snap[src]);
    }
    rb_recalc_constants(g);
}

void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e){
    int cnt=e->count>=0?e->count:1;
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    /* set_cost/set_cost_to_use carry the amount in a "value" extra when the
       wire encodes it that way; prefer it over the bare count. */
    int val=cnt;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")){
        int v=atoi(e->extra_v[i]); if(v) val=v;
    }
    int is_set = e->action && (!strcmp(e->action,"set_cost")||!strcmp(e->action,"set_cost_to_use"));
    /* Faithful target: apply to every staged member + hand-visible costs.
        Rust's cost_modifiers are per-card and constant abilities recalc via
        recalc_constants; here we mirror by applying to all owned members so
        later draws also see the modifier via the card_id entry. The old
        first-staged-only path was a P0/P1 coverage hole (modify_cost appeared
        to work but only for one member). */
    int any=0;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){
        int cid=P->stage[q];
        if(is_set) rb_mods_set_cost(&g->mods, cid, val);
        else       rb_mods_add_cost(&g->mods, cid, cnt);
        any=1;
    }
    if(!any){
        for(int i=0;i<P->hand.n;i++){
            int cid=P->hand.cards[i];
            if(is_set) rb_mods_set_cost(&g->mods, cid, val);
            else       rb_mods_add_cost(&g->mods, cid, cnt);
        }
        for(int i=0;i<P->deck.n;i++){
            int cid=P->deck.cards[i];
            if(is_set) rb_mods_set_cost(&g->mods, cid, val);
            else       rb_mods_add_cost(&g->mods, cid, cnt);
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
        /* Mirror misc.rs:execute_modify_yell_source — record the per-player yell
            source override (e.g. deck_bottom / discard / hand). live.c do_yell
            consults g->yell_source[pl] when drawing the revealed yell cards. */
        const char *src = NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"source")) { src=e->extra_v[i]; break; }
        if(!src) src = e->source;
        if(!src) src = "deck_top";
        strncpy(g->yell_source[who], src, sizeof(g->yell_source[who])-1);
        g->yell_source[who][sizeof(g->yell_source[who])-1] = '\0';
        /* Mirror modifiers.rs::refresh_yell_sources: a deck_bottom source sets
           yell_from_bottom so the cheer check (tracking.rs) draws from the deck
           bottom (G8 — 恋になりたいAQUARIUM). */
        g->p[who].yell_from_bottom = (strcmp(src, "deck_bottom") == 0);
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
    /* Mirror score.rs::execute_modify_required_hearts — required hearts attach to
        the player's LIVE cards (the cards actually performed); fall back to all
        stage members when no live cards are set yet. Previously only the first
        staged/hand card was modified (under-counted multi-live effects). */
    int applied = 0;
    for(int i=0;i<P->live.n;i++){ rb_mods_add_need_heart(&g->mods, P->live.cards[i], col%8, cnt); applied=1; }
    if(!applied){
        for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){
            rb_mods_add_need_heart(&g->mods, P->stage[q], col%8, cnt);
        }
    }
}

/* Mirror state.rs::execute_energy_placement — draw `count` energy from the
   energy deck into the energy zone, activating them when state_change=="active".
   The C energy model tracks energy as a single count (zone size + active count),
   so this simply grows the zone and (optionally) the active count. */
void rb_effect_energy_placement(GameState *g, int actor, AbilityEffect *e){
    const char *st = NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"state")) st=e->extra_v[i];
    int who = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2"))) who = actor^1;
    RbPlayer *P = &g->p[who];
    int n = e->count >= 0 ? e->count : 1;
    int active = st && (!strcmp(st,"active")||!strcmp(st,"アクティブ"));
    for(int k=0;k<n;k++){
        if(P->energy.n < RB_ENERGY_CAP) P->energy.n++;
        if(active && P->energy_active < RB_ENERGY_CAP) P->energy_active++;
    }
}

/* Mirror state.rs::execute_energy_state_change — change active/wait state of
   `count` energy cards. "active" activates (energy_active += effective);
   "wait" deactivates (energy_active -= effective). effective follows the Rust
   max/count==0 logic against the C active/total counts. */
void rb_effect_energy_state_change(GameState *g, int actor, AbilityEffect *e){
    const char *st = NULL;
    int max = 0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"state")) st=e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"max") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")) max=1;
    }
    if(!st) st = "active";
    int who = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2"))) who = actor^1;
    RbPlayer *P = &g->p[who];
    int total = P->energy.n, active = P->energy_active;
    int is_active = (!strcmp(st,"active")||!strcmp(st,"アクティブ"));
    int eff;
    if(max){
        int available = is_active ? (total - active) : active;
        if(available < 0) available = 0;
        int req = e->count > 0 ? e->count : 1;
        eff = req < available ? req : available;
    } else if(e->count == 0){
        eff = is_active ? (total - active) : active;   /* all of the opposite state */
    } else {
        eff = e->count;
    }
    if(is_active){
        active += eff;
        if(active > total) active = total;
        if(active > RB_ENERGY_CAP) active = RB_ENERGY_CAP;
        P->energy_active = active;
    } else {
        active -= eff;
        if(active < 0) active = 0;
        P->energy_active = active;
    }
}
