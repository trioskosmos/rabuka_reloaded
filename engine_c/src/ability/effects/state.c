#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

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
        /* A position change is an event that can trigger the player's 自動 (Auto)
            abilities (mirrors Rust choice.rs position handling →
            trigger_auto_abilities_for_movement_current). Queue them; the queue is
            drained once rb_resume_with_choice normalizes its state below. */
        rb_fire_auto_and_pending(g, who);
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
        rb_fire_auto_and_pending(g, who);
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
    /* Interactive swap just resolved — fire the player's 自動 (Auto) abilities
        (mirrors Rust choice.rs position handler →
        trigger_auto_abilities_for_movement_current). Queued; drained by
        rb_resume_with_choice once it normalizes its queue state. */
    rb_fire_auto_and_pending(g, actor);
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

        /* Mirror modifiers.rs::refresh_yell_sources: a deck_bottom source sets
           yell_from_bottom so the cheer check (tracking.rs) draws from the deck
           bottom (G8 — 恋になりたいAQUARIUM). */
void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e){
    /* Faithful mirror of engine/src/ability/effects/score.rs::execute_modify_required_hearts:
       apply add/set need_heart modifiers (operation: decrease/increase/set) to the
       target player's live cards, for each listed heart color, scaled by per_unit. */
    int value = e->count >= 0 ? e->count : 1;
    const char *op = "decrease"; int is_set = 0; int sign = -1;
    const char *grp = NULL; const char *loc = NULL; int per_unit = 0; int per_unit_count = 1;
    for (int i = 0; i < e->n_extra; i++) {
        if (!e->extra_k[i]) continue;
        if (!strcmp(e->extra_k[i], "operation") && e->extra_v[i]) {
            op = e->extra_v[i];
            if (!strcmp(op, "increase")) { sign = 1; is_set = 0; }
            else if (!strcmp(op, "set")) { sign = 1; is_set = 1; }
            else { sign = -1; is_set = 0; }
        } else if (!strcmp(e->extra_k[i], "group_names") || !strcmp(e->extra_k[i], "group_name")) {
            if (e->extra_v[i]) grp = e->extra_v[i];
        } else if (!strcmp(e->extra_k[i], "per_unit") && e->extra_v[i] && !strcmp(e->extra_v[i], "true")) {
            per_unit = 1;
        } else if (!strcmp(e->extra_k[i], "location")) {
            loc = e->extra_v[i];
        } else if (!strcmp(e->extra_k[i], "per_unit_count") && e->extra_v[i]) {
            per_unit_count = atoi(e->extra_v[i]);
        }
    }
    /* colors (default heart00) from heart_colors / heart_color (comma list) */
    int cols[8]; int nc = 0;
    for (int i = 0; i < e->n_extra && nc < 8; i++) {
        if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_colors") || !strcmp(e->extra_k[i], "heart_color")) && e->extra_v[i]) {
            cols[nc++] = heart_color_of((AbilityEffect*)e, 0);
            break;
        }
    }
    if (nc == 0) cols[nc++] = 0;
    RbPlayer *Pp = (e->target && !strcmp(e->target, "opponent")) ? &g->p[actor ^ 1] : &g->p[actor];
    if (per_unit) {
        int units = Pp->live.n;
        if (loc && (!strcmp(loc, "success_live_zone") || !strcmp(loc, "live_zone") || !strcmp(loc, "success_live_card_zone")))
            units = Pp->success.n;
        if (per_unit_count < 1) per_unit_count = 1;
        value = value * (units / per_unit_count);
    }
    int who = (e->target && !strcmp(e->target, "opponent")) ? actor ^ 1 : actor;
    RbPlayer *P = &g->p[who];
    for (int i = 0; i < P->live.n; i++) {
        int cid = P->live.cards[i];
        if (grp && !rb_card_matches_group_str(cid, grp)) continue;
        for (int c = 0; c < nc; c++) {
            if (is_set) rb_mods_set_need_heart(&g->mods, cid, cols[c], value);
            else       rb_mods_add_need_heart(&g->mods, cid, cols[c], value * sign);
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

/* ═══════════════════════════════════════════════════════════════════════════
   Faithful mirrors of engine/src/ability/effects/state.rs execute_* functions.
   Each mirrors the Rust body as closely as the C model allows. Helpers first.
   ═══════════════════════════════════════════════════════════════════════════ */

static int s_who(const char *target, int actor){
    if(target && (!strcmp(target,"opponent")||!strcmp(target,"p2"))) return actor^1;
    return actor;
}
static int s_value(const AbilityEffect *e, int dflt){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")){
        int v=atoi(e->extra_v[i]); return v;
    }
    return dflt;
}
static int s_has_group(const AbilityEffect *e, const char **out){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && (!strcmp(e->extra_k[i],"group_names")||!strcmp(e->extra_k[i],"group_name")) && e->extra_v[i]){ *out=e->extra_v[i]; return 1; }
    return 0;
}
static int s_has_chars(const AbilityEffect *e, const char **out){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"characters") && e->extra_v[i]){ *out=e->extra_v[i]; return 1; }
    return 0;
}
static int s_match_chars(int cid, const char *chars){
    if(!chars) return 1;
    char buf[256]; strncpy(buf, chars, 255); buf[255]=0;
    char *tok=strtok(buf, ",、 ");
    const char *arr[8]; int n=0;
    while(tok && n<8){ arr[n++]=tok; tok=strtok(NULL, ",、 "); }
    if(n==0) return 1;
    return rb_card_matches_characters(cid, arr, n);
}
/* Keep a candidate only if it passes the optional group/character filter. */
static int s_pass_filter(int cid, const char *grp, const char *chars){
    if(grp && !rb_card_matches_group_str(cid, grp)) return 0;
    if(!s_match_chars(cid, chars)) return 0;
    return 1;
}
static int s_blade_color_idx(const char *bt){
    if(!bt) return -1;
    if(!strcmp(bt,"pink")||!strcmp(bt,"heart00")) return 0;
    if(!strcmp(bt,"red")) return 1;
    if(!strcmp(bt,"yellow")) return 2;
    if(!strcmp(bt,"green")) return 3;
    if(!strcmp(bt,"blue")) return 4;
    if(!strcmp(bt,"purple")) return 5;
    if(!strcmp(bt,"orange")) return 6;
    if(!strcmp(bt,"all")) return 7;
    return -1;
}
static int s_heart_idx(const char *h){
    if(!h) return RB_HEART_ALL;
    if(!strcmp(h,"pink")||!strcmp(h,"heart00")) return RB_HEART_PINK;
    if(!strcmp(h,"red")||!strcmp(h,"heart01")) return RB_HEART_RED;
    if(!strcmp(h,"yellow")||!strcmp(h,"heart02")) return RB_HEART_YELLOW;
    if(!strcmp(h,"green")||!strcmp(h,"heart03")) return RB_HEART_GREEN;
    if(!strcmp(h,"blue")||!strcmp(h,"heart04")) return RB_HEART_BLUE;
    if(!strcmp(h,"purple")||!strcmp(h,"heart05")) return RB_HEART_PURPLE;
    if(!strcmp(h,"orange")||!strcmp(h,"heart06")) return RB_HEART_ORANGE;
    if(!strcmp(h,"all")||!strcmp(h,"heart07")) return RB_HEART_ALL;
    if(!strncmp(h,"heart",5)){ int idx=atoi(h+5); if(idx>=0&&idx<=7) return idx; }
    return RB_HEART_ALL;
}

/* Mirror state.rs::execute_set_cost — value from "value" extra; zone chosen by
   card_type (live_card / member_card / energy_card / default hand); optional
   group / character filter. */
void rb_effect_set_cost(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int value = s_value(e, 0);
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    int ids[RB_MAX_ZONE]; int n=0;
    const char *ct = e->card_type_field;
    if(ct && !strcmp(ct,"live_card")) { for(int i=0;i<P->live.n;i++) ids[n++]=P->live.cards[i]; }
    else if(ct && !strcmp(ct,"member_card")) { for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT) ids[n++]=P->stage[q]; }
    else if(ct && !strcmp(ct,"energy_card")) { for(int i=0;i<P->energy.n;i++) ids[n++]=P->energy.cards[i]; }
    else { for(int i=0;i<P->hand.n;i++) ids[n++]=P->hand.cards[i]; }
    const char *grp=NULL,*chars=NULL;
    if(s_has_group(e,&grp)||s_has_chars(e,&chars)){
        int fids[RB_MAX_ZONE]; int fn=0;
        for(int i=0;i<n;i++) if(s_pass_filter(ids[i], grp, chars)) fids[fn++]=ids[i];
        n=fn; for(int i=0;i<n;i++) ids[i]=fids[i];
    }
    for(int i=0;i<n;i++) rb_mods_set_cost(&g->mods, ids[i], value);
}

/* Mirror state.rs::execute_modify_cost — operation add/subtract/set (default add),
   zone by card_type / source=hand, optional group/character filter, self_target
   restriction to the activating card (host_cid), and the "set_from_reference"
   family (resolve a previously selected/moved card's printed cost ± offset as an
   additive delta). */
void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *op="add";
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"operation") && e->extra_v[i]) op=e->extra_v[i];
    int value = s_value(e, 0);
    int who = s_who(e->target, actor);
    /* modify_yell_count / modify_yell_source are per-player yell modifiers. */
    if(e->action && !strcmp(e->action,"modify_yell_count")){
        g->yell_count_mod[who] += (e->count>=0?e->count:1);
        return;
    } else if(e->action && !strcmp(e->action,"modify_yell_source")){
        const char *src = NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"source")) { src=e->extra_v[i]; break; }
        if(!src) src = e->source;
        if(!src) src = "deck_top";
        strncpy(g->yell_source[who], src, sizeof(g->yell_source[who])-1);
        g->yell_source[who][sizeof(g->yell_source[who])-1] = '\0';
        g->p[who].yell_from_bottom = (strcmp(src, "deck_bottom") == 0);
        return;
    }
    RbPlayer *P = &g->p[who];
    int is_hand = 0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"source")||!strcmp(e->extra_k[i],"location")) && e->extra_v[i] && !strcmp(e->extra_v[i],"hand")) is_hand=1;
    }
    int ids[RB_MAX_ZONE]; int n=0;
    const char *ct = e->card_type_field;
    if(is_hand) { for(int i=0;i<P->hand.n;i++) ids[n++]=P->hand.cards[i]; }
    else if(ct && !strcmp(ct,"live_card")) { for(int i=0;i<P->live.n;i++) ids[n++]=P->live.cards[i]; }
    else if(ct && !strcmp(ct,"member_card")) { for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT) ids[n++]=P->stage[q]; }
    else if(ct && !strcmp(ct,"energy_card")) { for(int i=0;i<P->energy.n;i++) ids[n++]=P->energy.cards[i]; }
    else { for(int i=0;i<P->hand.n;i++) ids[n++]=P->hand.cards[i]; }
    const char *grp=NULL,*chars=NULL;
    if(s_has_group(e,&grp)||s_has_chars(e,&chars)){
        int fids[RB_MAX_ZONE]; int fn=0;
        for(int i=0;i<n;i++) if(s_pass_filter(ids[i], grp, chars)) fids[fn++]=ids[i];
        n=fn; for(int i=0;i<n;i++) ids[i]=fids[i];
    }
    if(e->self_target_field[0]=='t' && host_cid>=0){
        int fids[RB_MAX_ZONE]; int fn=0;
        for(int i=0;i<n;i++) if(ids[i]==host_cid) fids[fn++]=ids[i];
        n=fn; for(int i=0;i<n;i++) ids[i]=fids[i];
    }
    if(!strcmp(op,"set_from_reference")){
        int ref=-1;
        if(g->n_selected_cards>0) ref=g->selected_cards[g->n_selected_cards-1];
        else if(g->n_those_cards>0) ref=g->those_cards[g->n_those_cards-1];
        else if(g->n_recently_moved>0) ref=g->recently_moved[g->n_recently_moved-1];
        if(ref<0) return;
        Card rc; int refcost=0;
        if(rb_decode_card_by_index((uint32_t)ref,&rc)){ refcost = rc.cost; rb_free_card(&rc); }
        const char *off=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i]&&!strcmp(e->extra_k[i],"cost_offset")&&e->extra_v[i]) off=e->extra_v[i];
        int offset = off?atoi(off):0;
        int resolved = refcost + offset;
        for(int i=0;i<n;i++){
            int printed=0; Card c;
            if(rb_decode_card_by_index((uint32_t)ids[i],&c)){ printed=c.cost; rb_free_card(&c); }
            int d = resolved - printed;
            rb_mods_add_cost(&g->mods, ids[i], d);
        }
        return;
    }
    int delta;
    if(!strcmp(op,"set")) delta=value;
    else if(!strcmp(op,"subtract")) delta=-value;
    else if(!strcmp(op,"add")) delta=value;
    else return;
    for(int i=0;i<n;i++){
        if(!strcmp(op,"set")) rb_mods_set_cost(&g->mods, ids[i], delta);
        else rb_mods_add_cost(&g->mods, ids[i], delta);
    }
}

/* Mirror state.rs::execute_set_blade_type — recolor the blade of every staged
   member matching the optional group/character filter to the given BladeColor. */
void rb_effect_set_blade_type(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    const char *bt=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && (!strcmp(e->extra_k[i],"blade_type")||!strcmp(e->extra_k[i],"blade_color")) && e->extra_v[i]) bt=e->extra_v[i];
    int col = s_blade_color_idx(bt);
    if(col<0) return;
    int who = s_who(e->target, actor);
    RbPlayer *P=&g->p[who];
    const char *grp=NULL,*chars=NULL; s_has_group(e,&grp); s_has_chars(e,&chars);
    for(int q=0;q<RB_STAGE_SIZE;q++){
        int cid=P->stage[q];
        if(cid==RB_EMPTY_SLOT) continue;
        if(!s_pass_filter(cid, grp, chars)) continue;
        g->mods.blade_type[cid]=(int8_t)col;
    }
    rb_recalc_constants(g);
}

/* Mirror state.rs::execute_set_blade_count — set the blade modifier of every
   staged member (filtered by group/character/position) to `value`. */
void rb_effect_set_blade_count(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int value = s_value(e, 0);
    if(value==0) value = (e->count>=0?e->count:0);
    int who = s_who(e->target, actor);
    RbPlayer *P=&g->p[who];
    int ids[RB_STAGE_SIZE]; int n=0;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT) ids[n++]=P->stage[q];
    const char *grp=NULL,*chars=NULL;
    if(s_has_group(e,&grp)||s_has_chars(e,&chars)){
        int f[RB_STAGE_SIZE]; int fn=0;
        for(int i=0;i<n;i++) if(s_pass_filter(ids[i],grp,chars)) f[fn++]=ids[i];
        n=fn; for(int i=0;i<n;i++) ids[i]=f[i];
    }
    const char *pos=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i]&&!strcmp(e->extra_k[i],"position")&&e->extra_v[i]) pos=e->extra_v[i];
    if(pos){ int area=rb_pos_to_area(pos); if(area>=0){ int exp=P->stage[area]; int f[RB_STAGE_SIZE]; int fn=0; for(int i=0;i<n;i++) if(ids[i]==exp) f[fn++]=ids[i]; n=fn; for(int i=0;i<n;i++) ids[i]=f[i]; } }
    for(int i=0;i<n;i++) rb_mods_set_blade(&g->mods, ids[i], value);
    rb_recalc_constants(g);
}

/* Mirror state.rs::execute_set_heart_copy_from_under — copy the hearts of the
   card just placed under this member onto the member. */
void rb_effect_set_heart_copy_from_under(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)e;
    int member=-1;
    if(g->n_selected_cards>0) member=g->selected_cards[0];
    else if(host_cid>=0) member=host_cid;
    if(member<0) return;
    RbPlayer *P=&g->p[actor];
    int src=-1;
    for(int s=0;s<RB_STAGE_SIZE;s++) if(P->stage[s]==member){
        RbBag *uc=&P->under_cards[s];
        if(uc->n>0){
            for(int k=uc->n-1;k>=0;k--){
                int c=uc->cards[k]; int is_moved=0;
                for(int m=0;m<g->n_those_cards;m++) if(g->those_cards[m]==c){is_moved=1;break;}
                if(is_moved){ src=c; break; }
            }
            if(src<0) src=uc->cards[uc->n-1];
        }
        break;
    }
    if(src<0) return;
    rb_mods_set_heart_copy(&g->mods, member, src);
}

/* Mirror state.rs::execute_set_heart_type (+ set_heart_type_applied). ref_value
   "placed_under" copies the under-card's hearts; otherwise the member's hearts
   are recolored (transform) to the chosen heart color via heart_multiplier. */
void rb_effect_set_heart_type(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *ref=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i]&&!strcmp(e->extra_k[i],"ref_value")&&e->extra_v[i]) ref=e->extra_v[i];
    if(ref && !strcmp(ref,"placed_under")){ rb_effect_set_heart_copy_from_under(g, actor, e, host_cid); return; }
    const char *ht=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && (!strcmp(e->extra_k[i],"heart_type")||!strcmp(e->extra_k[i],"heart_color")) && e->extra_v[i]) ht=e->extra_v[i];
    if(!ht) for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_colors") && e->extra_v[i]) ht=e->extra_v[i];
    if(!ht || !strcmp(ht,"selected")) ht="heart00";
    int col = s_heart_idx(ht);
    int who = s_who(e->target, actor);
    int cid=-1;
    if(g->n_selected_cards>0) cid=g->selected_cards[0];
    else if(host_cid>=0) cid=host_cid;
    else for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[who].stage[q]!=RB_EMPTY_SLOT){ cid=g->p[who].stage[q]; break; }
    if(cid<0) return;
    g->mods.heart_multiplier[cid]=(int8_t)col;
    g->mods.heart_multiplier_amt[cid]=(int8_t)(e->count>=1?e->count:2);
}

/* Mirror state.rs::execute_set_card_identity — rewrite this member's identity to
   the listed group/unit names so it counts as them in group/name matching. */
void rb_effect_set_card_identity(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int cid=-1;
    if(host_cid>=0) cid=host_cid;
    else for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]!=RB_EMPTY_SLOT){ cid=g->p[actor].stage[q]; break; }
    if(cid<0) return;
    const char *id=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i]&&(!strcmp(e->extra_k[i],"identities")||!strcmp(e->extra_k[i],"identity"))&&e->extra_v[i]) id=e->extra_v[i];
    if(!id) return;
    char buf[256]; strncpy(buf,id,255); buf[255]=0;
    char *tok=strtok(buf, ",、 ");
    while(tok){ rb_set_card_identity(cid, tok); tok=strtok(NULL, ",、 "); }
}

/* Mirror state.rs::execute_set_card_identity_all_regions — identity rewrite that
   also records a per-card prohibition note "card_identity:{cid}:{identity}". */
void rb_effect_set_card_identity_all_regions(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int cid=-1;
    if(host_cid>=0) cid=host_cid;
    else for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]!=RB_EMPTY_SLOT){ cid=g->p[actor].stage[q]; break; }
    if(cid<0) return;
    const char *id=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i]&&(!strcmp(e->extra_k[i],"identities")||!strcmp(e->extra_k[i],"identity"))&&e->extra_v[i]) id=e->extra_v[i];
    if(!id) return;
    char buf[256]; strncpy(buf,id,255); buf[255]=0;
    char *tok=strtok(buf, ",、 ");
    while(tok){
        rb_set_card_identity(cid, tok);
        if(g->n_prohibition < 64){
            snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[g->n_prohibition]), "card_identity:%d:%s", cid, tok);
            g->n_prohibition++;
        }
        tok=strtok(NULL, ",、 ");
    }
}

/* Mirror state.rs::execute_reduce_live_card_set_limit — reduce the player's live
   card set limit by `count`. */
void rb_effect_reduce_live_card_set_limit(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int lim = e->count>=0?e->count:1;
    int who = s_who(e->target, actor);
    g->live_set_limit_reduction[who]+=lim;
    if(g->live_set_limit_reduction[who]>RB_MAX_LIVE_CARDS) g->live_set_limit_reduction[who]=RB_MAX_LIVE_CARDS;
}

/* Mirror state.rs::execute_specify_heart_color — set a persistent per-card heart
   color override (target member's base hearts counted as `col`). */
void rb_effect_specify_heart_color(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int col = heart_color_of(e, RB_HEART_PINK);
    if(col<0||col>7) col=RB_HEART_PINK;
    int who = s_who(e->target, actor);
    RbPlayer *P=&g->p[who];
    if(host_cid>=0) g->mods.heart_color_override[host_cid]=(int8_t)col;
    else for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT) g->mods.heart_color_override[P->stage[q]]=(int8_t)col;
    rb_recalc_constants(g);
}

/* Mirror state.rs::execute_set_cost_to_use — set this member's cost-to-use to
   `value` (applies to the activating/selected card). */
void rb_effect_set_cost_to_use(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int value=s_value(e,0);
    int cid = host_cid>=0?host_cid:-1;
    if(cid<0 && g->n_selected_cards>0) cid=g->selected_cards[0];
    if(cid<0) return;
    rb_mods_set_cost(&g->mods, cid, value);
}

/* Mirror state.rs::execute_all_blade_timing — set the member's blade type to
   "all" so its blade satisfies any blade-timing condition. */
void rb_effect_all_blade_timing(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)e;
    int cid = host_cid>=0?host_cid:-1;
    if(cid<0 && g->n_selected_cards>0) cid=g->selected_cards[0];
    if(cid<0) for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]!=RB_EMPTY_SLOT){cid=g->p[actor].stage[q];break;}
    if(cid<0) return;
    g->mods.blade_type[cid]=7;
}

/* Mirror state.rs::execute_activation_cost — record a prohibition note
   "activation_cost_{op}_{value}" for self/opponent targets. */
void rb_effect_activation_cost(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    const char *op="increase";
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i]&&!strcmp(e->extra_k[i],"operation")&&e->extra_v[i]) op=e->extra_v[i];
    int value=s_value(e,0);
    const char *target = e->target?e->target:"self";
    if(!strcmp(target,"self")||!strcmp(target,"opponent")){
        char note[64]; snprintf(note,sizeof(note),"activation_cost_%s_%d",op,value);
        if(g->n_prohibition < 64){ snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[g->n_prohibition]), "%s", note); g->n_prohibition++; }
    }
}
