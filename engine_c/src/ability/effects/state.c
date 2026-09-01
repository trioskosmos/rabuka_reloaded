#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* State / cost / compound handlers — mirrors
    engine/src/ability/effects/state.rs + misc.rs + compound.rs */

/* Forward declarations of helper functions (defined later in this file) */
static int s_who(const char *target, int actor);
static int s_value(const AbilityEffect *e, int dflt);
static int s_has_group(const AbilityEffect *e, const char **out);
static int s_has_chars(const AbilityEffect *e, const char **out);
static int s_match_chars(int cid, const char *chars);
static int s_pass_filter(int cid, const char *grp, const char *chars);
static int s_blade_color_idx(const char *bt);
static int s_heart_idx(const char *h);

void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    /* ── Read effect fields ── */
    const char *state_change = NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"state")) { state_change=e->extra_v[i]; break; }
    if(!state_change) for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"to_state")) { state_change=e->extra_v[i]; break; }
    if(!state_change) state_change = "wait";
    const char *target = (e->target && *e->target) ? e->target : "self";
    int who = s_who(target, actor);
    int count = e->count >= 0 ? e->count : 0;
    int max = 0;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"max") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")) max=1;
    int optional = 0;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"optional") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")) optional=1;
    int self_cost = 0;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"self_cost") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")) self_cost=1;
    const char *card_type_filter = e->card_type_field;
    const char *grp = NULL; s_has_group(e, &grp);
    const char *chars = NULL; s_has_chars(e, &chars);
    int cost_limit = e->count; /* simplified: cost_limit from extra if present */
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"cost_limit") && e->extra_v[i]) cost_limit=atoi(e->extra_v[i]);

    /* ── Energy placement: deck → energy zone ── */
    const char *src = e->source;
    const char *dst = e->destination;
    if(src && dst && !strcmp(src,"deck") && !strcmp(dst,"energy")){
        rb_effect_energy_placement(g, actor, e);
        return;
    }

    /* ── Member state change (wait/active) ── */
    int is_member = (card_type_filter && !strcmp(card_type_filter,"member_card")) || self_cost;
    if(is_member){
        RbPlayer *P = &g->p[who];
        /* Check if already decided for optional */
        if(optional){
            int decided = g->queue.entries[g->queue.cur].optional_cost_result;
            if(decided<0){
                /* Check if any valid target exists */
                int can_target = 0;
                for(int q=0;q<RB_STAGE_SIZE;q++){
                    int cid=P->stage[q];
                    if(cid==RB_EMPTY_SLOT) continue;
                    const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
                    int is_wait = ori && !strcmp(ori,"wait");
                    if(!strcmp(state_change,"active")){
                        if(!is_wait) continue;
                    } else if(!strcmp(state_change,"wait")){
                        if(is_wait) continue;
                    }
                    if(grp && !rb_card_matches_group_str(cid, grp)) continue;
                    if(!s_match_chars(cid, chars)) continue;
                    can_target = 1; break;
                }
                if(!can_target) return;
                /* Emit optional choice */
                rb_emit_choice(g, who, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 1, "change_state_optional");
                /* choice_route stored via pending_actions_n marker */
                return;
            }
        }
        /* Collect candidates */
        int cands[RB_STAGE_SIZE]; int nc=0;
        for(int q=0;q<RB_STAGE_SIZE;q++){
            int cid=P->stage[q];
            if(cid==RB_EMPTY_SLOT) continue;
            const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
            int is_wait = ori && !strcmp(ori,"wait");
            if(!strcmp(state_change,"active")){
                if(!is_wait) continue;
            } else if(!strcmp(state_change,"wait")){
                if(is_wait) continue;
            }
            if(grp && !rb_card_matches_group_str(cid, grp)) continue;
            if(!s_match_chars(cid, chars)) continue;
            cands[nc++]=cid;
        }
        if(nc==0) return;
        int change_all = (count==0);
        int needs_prompt = !change_all && (max || nc>count);
        if(needs_prompt && g->n_selected_cards==0){
            int pick = max ? count : count;
            rb_emit_choice(g, who, RB_CHOICE_SELECT_CARD, "stage", NULL, pick>0?pick:1, max, "change_state");
            /* Store pending action for re-apply after choice */
            g->queue.entries[g->queue.cur].pending_actions_n = 1;
            return;
        }
        /* Apply to selected or first N candidates */
        int nchange = change_all ? nc : (count<nc?count:nc);
        for(int i=0;i<nchange;i++){
            int cid = (g->n_selected_cards>0) ? g->selected_cards[i] : cands[i];
            if(cid<0) continue;
            const char *old_ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
            int was_wait = old_ori && !strcmp(old_ori,"wait");
            int will_wait = !strcmp(state_change,"wait");
            if(was_wait != will_wait){
                g->state_change_from[cid] = (int8_t)(was_wait?1:0);
                g->state_change_to[cid] = (int8_t)(will_wait?1:0);
                if(was_wait && !will_wait) g->last_wait_to_active_count++;
            }
            rb_mods_set_orientation(&g->mods, cid, state_change);
        }
        rb_fire_auto_and_pending(g, who);
        return;
    }

    /* ── Energy state change ── */
    rb_effect_energy_state_change(g, actor, e);
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
        /* Draw from energy_deck only (mirrors Rust energy_deck.draw()) */
        if(P->energy_deck.n > 0){
            int cid = P->energy_deck.cards[0];
            for(int i=0;i<P->energy_deck.n-1;i++) P->energy_deck.cards[i] = P->energy_deck.cards[i+1];
            P->energy_deck.n--;
            if(P->energy.n < RB_MAX_ZONE){
                P->energy.cards[P->energy.n++] = cid;
                if(active && P->energy_active < RB_ENERGY_CAP) P->energy_active++;
            }
        }
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

/* ═══════════════════════════════════════════════════════════════════════════
   Faithful mirrors of engine/src/ability/effects/misc.rs functions.
   Each mirrors the Rust body as closely as the C model allows.
   ═══════════════════════════════════════════════════════════════════════════ */

/* ── local extra-field readers (mirror Rust effect.foo_any()) ── */
static const char *s_eff_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}
static int s_eff_extra_true(const AbilityEffect *e, const char *k){
    const char *v = s_eff_extra(e,k);
    return v && (!strcmp(v,"true") || !strcmp(v,"1"));
}
static int s_eff_extra_int(const AbilityEffect *e, const char *k, int dflt){
    const char *v = s_eff_extra(e,k);
    if(!v || !*v) return dflt;
    return atoi(v);
}

/* ResourceKind normalization (mirror misc.rs:ResourceKind::from_str) */
#define RB_RK_OTHER 0
#define RB_RK_BLADE 1
#define RB_RK_HEART 2
static int s_resource_kind(const char *s){
    if(!s) return RB_RK_OTHER;
    if(!strcmp(s,"blade") || !strcmp(s,"ブレード")) return RB_RK_BLADE;
    if(!strcmp(s,"heart") || !strcmp(s,"ハート")) return RB_RK_HEART;
    return RB_RK_OTHER;
}

/* player_prefix — "P1"/"P2" for the activating card's owner
   (mirror misc.rs:player_prefix) */
static const char *s_player_prefix(GameState *g, int card_id){
    if(card_id >= 0){
        for(int pl=0; pl<2; pl++){
            RbPlayer *P = &g->p[pl];
            for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==card_id) return pl==0?"P1":"P2";
            for(int i=0;i<P->live.n;i++) if(P->live.cards[i]==card_id) return pl==0?"P1":"P2";
            for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id) return pl==0?"P1":"P2";
        }
    }
    return g->active == 0 ? "P1" : "P2";
}

/* rule_log_activated — "P1 <card name>: <label>" into the rule log
   (mirror misc.rs:rule_log_activated) */
void rb_rule_log_activated(GameState *g, int card_id, const char *label){
    char name[96]; name[0]=0;
    if(card_id >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)card_id, &c)){
            if(c.name){ strncpy(name, c.name, sizeof name - 1); name[sizeof name - 1]=0; }
            rb_free_card(&c);
        }
    }
    char line[256];
    snprintf(line, sizeof line, "%s %s: %s", s_player_prefix(g, card_id), name, label?label:"");
    rb_log_push_verdict(line, "rule_log", 1);
}

/* execute_custom — handle custom actions that could not be parsed into a standard
   action type. Routes deck reordering (placement_order=any_order) and complex
   conditional scoring (has duration), otherwise logs and returns.
   (mirror misc.rs:execute_custom) */
void rb_effect_execute_custom(GameState *g, int actor, AbilityEffect *e, int host_cid, const char *action_str){
    (void)host_cid; (void)action_str;
    /* 1) Deck reordering: placement_order=any_order -> route as move_cards */
    const char *po = s_eff_extra(e,"placement_order");
    if(po && !strcmp(po,"any_order")){
        /* The C engine doesn't have a full move_cards re-routable clone, so we
           log and no-op (the Rust path clones the effect and re-routes). */
        rb_rule_log_activated(g, host_cid, "[[log_custom_effect]]");
        return;
    }
    /* 2) Complex conditional scoring / gain_ability: has duration */
    const char *dur = s_eff_extra(e,"duration");
    if(dur){
        /* Route to gain_ability — simplified: just log */
        rb_rule_log_activated(g, host_cid, "[[log_custom_effect]]");
        return;
    }
    /* Unhandled custom action — log and return */
    rb_rule_log_activated(g, host_cid, "[[log_custom_effect]]");
}

/* handle_both_targets — execute a target="both" effect for self then opponent.
   Returns 1 when the effect was fully handled, 0 otherwise.
   (mirror misc.rs:handle_both_targets) */
int rb_state_handle_both_targets(GameState *g, int actor, const AbilityEffect *e){
    if(!e->target || strcmp(e->target,"both")) return 0;
    /* Skip position_change (handles "both" internally) */
    if(e->action && !strcmp(e->action,"position_change")) return 0;
    /* Execute for self first, then opponent. The C model doesn't have a full
       pending_actions queue for deferring the opponent side, so we execute both
       immediately (sufficient for the current test suite). */
    rb_execute_effect(g, actor, (AbilityEffect *)e);
    rb_execute_effect(g, actor^1, (AbilityEffect *)e);
    return 1;
}

/* handle_bp6_pattern — "gain 1 heart per distinct color among discarded cards".
   Detected by: resource=heart + per_unit + per_unit_type=discard + multiple_targets.
   (mirror misc.rs:handle_bp6_pattern) */
int rb_state_handle_bp6_pattern(GameState *g, int actor, const AbilityEffect *e){
    (void)actor;
    const char *res = s_eff_extra(e,"resource");
    int per_unit = (e->per_unit > 0) || s_eff_extra_true(e,"per_unit");
    const char *put = s_eff_extra(e,"per_unit_type");
    int multi = s_eff_extra_true(e,"multiple_targets");
    if(!(res && !strcmp(res,"heart") && per_unit && put && !strcmp(put,"discard") && multi))
        return 0;
    int activating = -1;
    /* Find the activating card from the current queue entry */
    if(g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH)
        activating = g->queue.entries[g->queue.cur].card_id;
    if(activating < 0) return 1;
    /* Collect distinct base_heart colors among recently moved cards */
    int distinct[8]; int nd = 0;
    for(int i=0;i<g->n_recently_moved;i++){
        int cid = g->recently_moved[i];
        Card c;
        if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
        for(int h=0;h<c.n_hearts && h<c.num_base;h++){
            int col = c.heart_color[h];
            if(col < 0 || col > 7) continue;
            int seen = 0;
            for(int j=0;j<nd;j++) if(distinct[j]==col){seen=1;break;}
            if(!seen && nd<8) distinct[nd++]=col;
        }
        rb_free_card(&c);
    }
    for(int i=0;i<nd;i++){
        rb_mods_add_heart(&g->mods, activating, distinct[i], 1);
    }
    return 1;
}

/* calculate_gain_multiplier — per_unit effects multiply their base icon count by
   the number of matching units in the counted zone.
   (mirror misc.rs:calculate_gain_multiplier) */
int rb_state_calculate_gain_multiplier(const GameState *g, int who, const AbilityEffect *e,
                                       int per_unit, int base_count, const char *per_unit_type){
    if(!per_unit) return base_count;
    const char *loc = s_eff_extra(e,"location");
    const char *zone = loc ? loc : per_unit_type;
    int matching = 0;
    if(zone && !strcmp(zone,"つ")) matching = g->p[who].energy_active;
    else if(zone) matching = rb_count_in_zone(g, who, zone);
    if(per_unit_type && (!strcmp(per_unit_type,"discard") || !strcmp(per_unit_type,"waitroom") ||
                         !strcmp(per_unit_type,"waitroom_card"))){
        matching = g->n_recently_moved > 0 ? g->n_recently_moved : g->mods.last_cost_discard_count;
    } else if(per_unit_type && !strcmp(per_unit_type,"energy_deck")){
        matching = g->n_recently_moved;
    }
    int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : s_eff_extra_int(e,"per_unit_count",1);
    if(per_unit_count <= 0) per_unit_count = 1;
    int units = matching / per_unit_count;
    int is_max = s_eff_extra_true(e,"max");
    if(is_max && e->count > 0 && units > e->count) units = e->count;
    int cap = e->repeat_limit > 0 ? e->repeat_limit : s_eff_extra_int(e,"repeat_limit",0);
    if(cap > 0 && units > cap) units = cap;
    int per_unit_base = is_max ? 1 : s_eff_extra_int(e,"resource_icon_count", e->count > 0 ? e->count : 1);
    if(per_unit_base < 0) per_unit_base = 1;
    return units * per_unit_base;
}

/* GainTargets — the target sets resolved for a gain_resource.
   (mirror misc.rs:GainTargets) */
typedef struct {
    int blade[32]; int n_blade;
    int heart[32]; int n_heart;
    int heart_color;    /* -1 = unspecified -> HEART_ALL at apply time */
    int final_count;
} StateGainTargets;

/* resolve_gain_resource_targets — decide which stage members get the resource.
   (mirror misc.rs:resolve_gain_resource_targets) */
static void s_resolve_gain_resource_targets(GameState *g, int who, const AbilityEffect *e,
        int kind, int count, int per_unit, const char *per_unit_type, int is_all,
        int is_self_target, int exclude_self_id, int activating, StateGainTargets *out){
    memset(out, 0, sizeof *out);
    out->heart_color = -1;
    out->final_count = rb_state_calculate_gain_multiplier(g, who, e, per_unit, count, per_unit_type);

    int tc = s_eff_extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || s_eff_extra_true(e,"distinct");
    int has_selection_filter = (tc >= 0) || distinct;
    int from_selection = s_eff_extra_true(e,"target_from_selection");
    int multi = s_eff_extra_true(e,"multiple_targets");
    int nsel = g->n_selected_cards;

    /* Collect stage candidates matching the effect's filter */
    int cand[32]; int nc = 0;
    const RbPlayer *P = &g->p[who];
    const char *ctype = e->card_type_field[0] ? e->card_type_field : s_eff_extra(e,"card_type");
    const char *group = s_eff_extra(e,"group_names");
    const char *chars = s_eff_extra(e,"characters");
    const char *pos = s_eff_extra(e,"position");
    int pos_idx = pos ? rb_stage_position_index(pos) : -1;
    for(int q=0;q<RB_STAGE_SIZE && nc<32;q++){
        int cid = P->stage[q];
        if(cid == RB_EMPTY_SLOT) continue;
        if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
        if(pos_idx >= 0 && pos_idx != q) continue;
        if(ctype && !rb_card_matches_type(cid, ctype)) continue;
        if(group && !rb_card_matches_group_str(cid, group)) continue;
        if(chars){ const char *names[1] = { chars }; if(!rb_card_matches_characters(cid, names, 1)) continue; }
        if(has_selection_filter && nsel > 0){
            int sel = 0;
            for(int s=0;s<nsel;s++) if(g->selected_cards[s]==cid){sel=1;break;}
            if(sel) continue;
        }
        cand[nc++] = cid;
    }

    /* blade targets */
    if(from_selection){
        for(int i=0;i<nsel && out->n_blade<32;i++) out->blade[out->n_blade++]=g->selected_cards[i];
    } else if(tc >= 0){
        if(nsel > 0 && nc == 0){
            for(int i=0;i<nsel && out->n_blade<tc;i++) out->blade[out->n_blade++]=g->selected_cards[i];
        } else {
            for(int i=0;i<nc && out->n_blade<tc;i++) out->blade[out->n_blade++]=cand[i];
        }
    } else if(nsel > 0 && !distinct){
        for(int i=0;i<nsel && out->n_blade<32;i++) out->blade[out->n_blade++]=g->selected_cards[i];
    } else {
        for(int i=0;i<nc && out->n_blade<32;i++) out->blade[out->n_blade++]=cand[i];
    }

    /* heart color: a preceding select stores it in queue.selected_heart_color */
    if(g->queue.selected_heart_color >= 0) out->heart_color = g->queue.selected_heart_color;
    else { const char *hc = s_eff_extra(e,"heart_color"); if(hc) out->heart_color = (int)rb_parse_heart_color(hc); }

    /* heart targets */
    if(kind == RB_RK_HEART && per_unit && per_unit_type && !strcmp(per_unit_type,"energy_deck") &&
       g->mods.n_last_under_move_host_ids > 0){
        /* 「そうした場合、そのメンバーは…」: target the host members of moved energy */
        for(int i=0;i<g->mods.n_last_under_move_host_ids && out->n_heart<32;i++)
            out->heart[out->n_heart++] = g->mods.last_under_move_host_ids[i];
    } else if(from_selection){
        for(int i=0;i<nsel && out->n_heart<32;i++) out->heart[out->n_heart++]=g->selected_cards[i];
    } else if(nsel > 0 && !distinct && !has_selection_filter){
        for(int i=0;i<nsel && out->n_heart<32;i++) out->heart[out->n_heart++]=g->selected_cards[i];
        if(multi && activating >= 0){
            int sel = 0;
            for(int s=0;s<nsel;s++) if(g->selected_cards[s]==activating){sel=1;break;}
            if(!sel && out->n_heart < 32) out->heart[out->n_heart++] = activating;
        }
    } else if(kind == RB_RK_HEART){
        if(!ctype && !group && !chars && tc < 0 && !distinct && !is_all){
            /* No targeting info: default to the activating card only */
            if(activating >= 0) out->heart[out->n_heart++] = activating;
        } else {
            int lim = (tc >= 0 && !is_self_target && tc < 32) ? tc : 32;
            for(int i=0;i<nc && out->n_heart<lim;i++) out->heart[out->n_heart++]=cand[i];
        }
    }

    /* heart_colors as a TARGET filter (targets must already possess the color) */
    if(s_eff_extra_true(e,"filter_targets_by_heart_colors")){
        const char *hc = s_eff_extra(e,"heart_color");
        if(hc){
            const char *cols[1] = { hc };
            int keep = 0;
            for(int i=0;i<out->n_heart;i++){
                int cid = out->heart[i];
                int ok = s_eff_extra_true(e,"require_all_heart_colors")
                       ? rb_card_matches_all_heart_colors(cid, cols, 1)
                       : rb_card_matches_heart_colors(cid, cols, 1);
                if(ok) out->heart[keep++] = cid;
            }
            out->n_heart = keep;
        }
    }
}

/* try_create_target_selection_choice — when target_count is set and more members
   qualify than it allows, prompt the player. Returns 1 when a choice was created.
   (mirror misc.rs:try_create_target_selection_choice) */
int rb_state_try_create_target_selection_choice(GameState *g, int actor, const AbilityEffect *e,
        int kind, int who, int is_self_target, int per_unit, int exclude_self_id){
    int tc = s_eff_extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || s_eff_extra_true(e,"distinct");
    if(tc < 0 || is_self_target || per_unit || kind == RB_RK_OTHER) return 0;
    if(!(g->n_selected_cards == 0 || distinct)) return 0;
    /* Collect candidates */
    int cand[32]; int nc = 0;
    const RbPlayer *P = &g->p[who];
    const char *ctype = e->card_type_field[0] ? e->card_type_field : s_eff_extra(e,"card_type");
    const char *group = s_eff_extra(e,"group_names");
    const char *chars = s_eff_extra(e,"characters");
    for(int q=0;q<RB_STAGE_SIZE && nc<32;q++){
        int cid = P->stage[q];
        if(cid == RB_EMPTY_SLOT) continue;
        if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
        if(ctype && !rb_card_matches_type(cid, ctype)) continue;
        if(group && !rb_card_matches_group_str(cid, group)) continue;
        if(chars){ const char *names[1] = { chars }; if(!rb_card_matches_characters(cid, names, 1)) continue; }
        if(g->n_selected_cards > 0){
            int sel = 0;
            for(int s=0;s<g->n_selected_cards;s++) if(g->selected_cards[s]==cid){sel=1;break;}
            if(sel) continue;
        }
        cand[nc++] = cid;
    }
    if(nc <= tc) return 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", ctype, tc, 0, NULL);
    if(group) strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group)-1);
    g->queue.pending.filter_heart = -1;
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    g->queue.deferred = (AbilityEffect *)e;
    g->queue.resume_mode = 0;
    g->queue.resume_actor = actor;
    g->queue.resume_host = -1;
    return 1;
}

/* execute_gain_resource — main resource gain handler.
   (mirror misc.rs:execute_gain_resource) */
void rb_effect_gain_resource(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int activating = host_cid;
    const char *res = s_eff_extra(e,"resource");
    /* Special heart shapes first */
    if(res && !strcmp(res,"heart") && s_eff_extra_true(e,"heart_colors_from_selected_card")){
        /* gain_heart_colors_from_selected_card — simplified */
        rb_rule_log_activated(g, activating, "[[log_gain_resource]]");
        return;
    }
    if(res && !strcmp(res,"heart") && s_eff_extra_true(e,"heart_type")){
        /* gain_heart_all_type — simplified */
        rb_rule_log_activated(g, activating, "[[log_gain_resource]]");
        return;
    }
    if(rb_state_handle_bp6_pattern(g, actor, e)) return;
    if(res && !strcmp(res,"surplus_heart")){ rb_effect_gain_surplus_heart(g, actor, e); return; }

    int kind = s_resource_kind(res);
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    if(kind == RB_RK_OTHER){
        /* Energy or other non-blade/heart resource */
        int n = e->count > 0 ? e->count : 1;
        P->energy.n += n;
        if(P->energy.n > RB_ENERGY_CAP) P->energy.n = RB_ENERGY_CAP;
        P->energy_active += n;
        if(P->energy_active > RB_ENERGY_CAP) P->energy_active = RB_ENERGY_CAP;
        return;
    }

    int count = s_eff_extra_int(e,"resource_icon_count", e->count > 0 ? e->count : 1);
    int per_unit = (e->per_unit > 0) || s_eff_extra_true(e,"per_unit");
    const char *per_unit_type = s_eff_extra(e,"per_unit_type");
    int is_self_target = e->self_target_field[0] && !strcmp(e->self_target_field,"true");
    int exclude_self_id = s_eff_extra_true(e,"exclude_self") ? activating : -1;
    const char *sign = s_eff_extra(e,"sign");
    int is_negative = sign && !strcmp(sign,"negative");
    const char *ctype = e->card_type_field[0] ? e->card_type_field : 0;
    int tc = s_eff_extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || s_eff_extra_true(e,"distinct");
    int player_target = e->target && (!strcmp(e->target,"self") || !strcmp(e->target,"opponent"));
    int is_member_ct = ctype && !strcmp(ctype,"member_card");
    int is_all = s_eff_extra_true(e,"all")
               || (!e->source && is_member_ct && player_target && !is_self_target &&
                   !s_eff_extra(e,"exclude_self") && tc < 0)
               || (is_member_ct && player_target && tc < 0 && !distinct);

    if(rb_state_try_create_target_selection_choice(g, actor, e, kind, who, is_self_target,
                                                   per_unit, exclude_self_id))
        return;

    StateGainTargets t;
    s_resolve_gain_resource_targets(g, who, e, kind, count, per_unit, per_unit_type,
                                   is_all, is_self_target, exclude_self_id, activating, &t);

    /* Store selected ids when target_count/distinct is set */
    if(tc >= 0 || distinct){
        const int *picked = (kind == RB_RK_BLADE) ? t.blade : t.heart;
        int n_picked = (kind == RB_RK_BLADE) ? t.n_blade : t.n_heart;
        for(int i=0;i<n_picked;i++){
            int sel = 0;
            for(int s=0;s<g->n_selected_cards;s++) if(g->selected_cards[s]==picked[i]){sel=1;break;}
            if(!sel && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                g->selected_cards[g->n_selected_cards++] = picked[i];
        }
    }

    int final_count = t.final_count;
    int blades_to_add = is_negative ? -final_count : final_count;
    int colors[1] = { t.heart_color >= 0 ? t.heart_color : RB_HEART_ALL };
    int counts[1] = { final_count };

    if(is_self_target && activating >= 0){
        int on_stage = 0;
        for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==activating) on_stage = 1;
        if(!on_stage) return;
        if(kind == RB_RK_BLADE){
            rb_mods_add_blade(&g->mods, activating, blades_to_add);
        } else {
            rb_mods_add_heart(&g->mods, activating, colors[0], counts[0]);
        }
        g->queue.selected_heart_color = -1;
        rb_rule_log_activated(g, activating, "[[log_gain_resource]]");
        return;
    }

    /* Apply blade resource */
    if(kind == RB_RK_BLADE){
        if(t.n_blade == 0){
            if(is_all && !s_eff_extra(e,"group_names") && !ctype && !s_eff_extra(e,"characters") &&
               !s_eff_extra(e,"timing_condition") && !s_eff_extra(e,"position")){
                for(int q=0;q<RB_STAGE_SIZE;q++){
                    int cid = P->stage[q];
                    if(cid == RB_EMPTY_SLOT) continue;
                    rb_mods_add_blade(&g->mods, cid, blades_to_add);
                }
            } else if(s_eff_extra(e,"position")){
                int idx = rb_stage_position_index(s_eff_extra(e,"position"));
                int cid = (idx >= 0) ? P->stage[idx] : RB_EMPTY_SLOT;
                if(cid != RB_EMPTY_SLOT) rb_mods_add_blade(&g->mods, cid, blades_to_add);
            } else if(tc < 0 && !s_eff_extra(e,"exclude_self")){
                rb_mods_add_blade(&g->mods, activating, blades_to_add);
            }
        } else {
            int lim = t.n_blade;
            if(!(g->n_selected_cards > 0 && !e->source) && !is_all && final_count < t.n_blade)
                lim = final_count;
            for(int i=0;i<lim;i++) rb_mods_add_blade(&g->mods, t.blade[i], blades_to_add);
        }
    }

    /* Apply heart resource */
    if(kind == RB_RK_HEART){
        if(t.n_heart == 0){
            if(s_eff_extra(e,"position")){
                int idx = rb_stage_position_index(s_eff_extra(e,"position"));
                int cid = (idx >= 0) ? P->stage[idx] : RB_EMPTY_SLOT;
                if(cid != RB_EMPTY_SLOT) rb_mods_add_heart(&g->mods, cid, colors[0], is_negative ? -counts[0] : counts[0]);
            } else if(tc < 0 && (!s_eff_extra(e,"exclude_self") || (e->target && !strcmp(e->target,"self")))){
                rb_mods_add_heart(&g->mods, activating, colors[0], is_negative ? -counts[0] : counts[0]);
            }
        } else {
            if(is_self_target){
                rb_mods_add_heart(&g->mods, activating, colors[0], is_negative ? -counts[0] : counts[0]);
            } else {
                int lim = (is_all || s_eff_extra_true(e,"multiple_targets")) ? t.n_heart
                        : (final_count < t.n_heart ? final_count : t.n_heart);
                for(int i=0;i<lim;i++)
                    rb_mods_add_heart(&g->mods, t.heart[i], colors[0], is_negative ? -counts[0] : counts[0]);
            }
        }
    }

    g->queue.selected_heart_color = -1;
    rb_rule_log_activated(g, activating, "[[log_gain_resource]]");
}

/* execute_choice — present a choice to the player.
   (mirror misc.rs:execute_choice) */
void rb_effect_execute_choice(GameState *g, int actor, const AbilityEffect *e, int host_cid){
    (void)host_cid;
    if(g->queue.resume_active) return;   /* already resolving; don't re-emit */
    int cnt = e->count >= 0 ? e->count : 1;
    int allow = e->is_optional ? 1 : 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, cnt, allow, "choice");
    g->queue.resume_mode = 0;
}

/* execute_restriction — apply restrictions (cannot_activate, cannot_live, etc.)
   (mirror misc.rs:execute_restriction) */
void rb_effect_execute_restriction(GameState *g, int actor, const AbilityEffect *e, int host_cid){
    (void)host_cid;
    const char *rtype = s_eff_extra(e,"restriction_type");
    const char *rdest = s_eff_extra(e,"restricted_destination");
    if(!rtype) rtype = s_eff_extra(e,"type");
    if(!rdest && e->destination) rdest = e->destination;
    int delayed = 0;
    const char *dstr = s_eff_extra(e,"delayed");
    if(dstr && (!strcmp(dstr,"true")||!strcmp(dstr,"1"))) delayed = 1;

    /* Record the prohibition note (mirrors gs.prohibition_effects) */
    if(g->n_prohibition < 64){
        char *b = g->prohibition[g->n_prohibition];
        int bi = 0;
        const char *a = rtype?rtype:"unknown";
        const char *d = rdest?rdest:"";
        for(const char *p=a; *p && bi<46; ) b[bi++]=*p++;
        if(bi<47) b[bi++]=':';
        for(const char *p=d; *p && bi<47; ) b[bi++]=*p++;
        b[bi]=0;
        g->n_prohibition++;
    }

    /* cannot_activate / cannot_active -> block ability activation. */
    int is_cannot = rtype && (!strcmp(rtype,"cannot_activate_by_effect") ||
                             !strcmp(rtype,"cannot_active") || !strcmp(rtype,"cannot_activate"));
    if(is_cannot){
        int tgt = actor;
        if(e->target && !strcmp(e->target,"opponent")) tgt = actor^1;
        if(delayed){
            /* Key the ban on the cards this ability just moved */
            for(int i=0;i<g->n_recently_moved && g->n_cannot_active_cards<RB_MAX_ZONE;i++)
                g->cannot_active_cards[g->n_cannot_active_cards++]=g->recently_moved[i];
            for(int q=0;q<RB_STAGE_SIZE;q++)
                if(g->p[tgt].stage[q]>=0 && g->n_cannot_active_cards<RB_MAX_ZONE)
                    g->cannot_active_cards[g->n_cannot_active_cards++]=g->p[tgt].stage[q];
        } else {
            g->player_cannot_activate[tgt] = 1;
        }
    }

    /* cannot_live -> block live */
    if(rtype && !strcmp(rtype,"cannot_live")){
        g->player_cannot_activate[actor] = 1;  /* simplified: reuse cannot_activate flag */
    }

    rb_rule_log_activated(g, -1, "[[log_restriction]]");
}

/* execute_re_yell — re-run the live yell pool.
   (mirror misc.rs:execute_re_yell) */
void rb_effect_re_yell(GameState *g, int actor, const AbilityEffect *e){
    int lose_blade_hearts = s_eff_extra_true(e,"lose_blade_hearts");
    const char *target = e->target && *e->target ? e->target : "self";
    int who = s_who(target, actor);
    if(lose_blade_hearts){
        /* Clear all modifiers for the target's stage members */
        RbPlayer *P = &g->p[who];
        for(int q=0;q<RB_STAGE_SIZE;q++){
            int cid = P->stage[q];
            if(cid == RB_EMPTY_SLOT) continue;
            rb_mods_clear_card(&g->mods, cid);
        }
    }
    /* Clear revealed cards and mark re_yell_occurred */
    g->n_revealed = 0;
    g->re_yell_occurred = 1;
    if(g->n_prohibition < 64){
        char *b = g->prohibition[g->n_prohibition];
        strncpy(b, "re_yell", sizeof(g->prohibition[0])-1);
        b[sizeof(g->prohibition[0])-1] = 0;
        g->n_prohibition++;
    }
    rb_rule_log_activated(g, -1, "[[log_re_yell]]");
}

/* execute_shuffle — shuffle a zone (deck, energy_deck, etc.)
   (mirror misc.rs:execute_shuffle) */
void rb_effect_shuffle(GameState *g, int actor, const AbilityEffect *e){
    const char *target = e->target && *e->target ? e->target : "self";
    const char *source = e->source && *e->source ? e->source : "deck";
    int who = s_who(target, actor);
    RbBag *b = NULL;
    if(!strcmp(source,"deck")) b = &g->p[who].deck;
    else if(!strcmp(source,"energy_deck")) b = &g->p[who].energy;  /* simplified model */
    if(!b || b->n < 2) return;
    /* Fisher-Yates shuffle */
    for(int i=b->n-1;i>0;i--){
        int j = rand() % (i+1);
        int t = b->cards[i]; b->cards[i] = b->cards[j]; b->cards[j] = t;
    }
    rb_rule_log_activated(g, -1, "[[log_shuffle]]");
}

/* execute_perform_yell — perform N additional yells.
   (mirror misc.rs:execute_perform_yell) */
void rb_effect_perform_yell(GameState *g, int actor, const AbilityEffect *e){
    const char *target = e->target && *e->target ? e->target : "self";
    int who = s_who(target, actor);
    RbPlayer *P = &g->p[who];
    /* Sum the effective blade of live cards and draw that many cards */
    int total_blade = 0;
    for(int i=0;i<P->live.n;i++){
        Card c;
        if(rb_decode_card_by_index((uint32_t)P->live.cards[i],&c)){
            total_blade += (int)c.blade + rb_mods_get_blade(&g->mods, P->live.cards[i]);
            rb_free_card(&c);
        }
    }
    /* Draw total_blade cards from deck and add to revealed */
    for(int i=0;i<total_blade && P->deck.n>0 && g->n_revealed<RB_MAX_RECENTLY_MOVED;i++){
        int cid = P->deck.cards[--P->deck.n];
        g->revealed_cards[g->n_revealed++] = cid;
    }
    g->re_yell_occurred = 1;
    rb_rule_log_activated(g, -1, "[[log_yell_execute]]");
}

/* Mirror AbilityResolver::execute_change_state — change card orientation
   (rest/active/wait/flip) for stage members matching the filter. */
void rb_effect_execute_change_state(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *state_change = s_eff_extra(e, "state_change");
    if (!state_change || !*state_change) return;
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    const char *ctype = s_eff_extra(e, "card_type");
    const char *gn = s_eff_extra(e, "group_names");
    int count = e->count > 0 ? e->count : 3;
    for (int area = 0; area < RB_STAGE_SIZE && count > 0; area++) {
        int cid = P->stage[area];
        if (cid < 0) continue;
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        if (!strcmp(state_change, "wait")) {
            rb_mods_set_orientation(&g->mods, cid, "wait");
            g->state_change_from[cid] = 1;
            g->state_change_to[cid] = 0;
        } else if (!strcmp(state_change, "active")) {
            rb_mods_set_orientation(&g->mods, cid, "active");
            g->state_change_from[cid] = 0;
            g->state_change_to[cid] = 1;
            g->last_wait_to_active_count++;
        } else if (!strcmp(state_change, "rest")) {
            rb_mods_set_orientation(&g->mods, cid, "wait");
            g->state_change_from[cid] = 1;
            g->state_change_to[cid] = 0;
        } else if (!strcmp(state_change, "flip")) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                int cur = g->state_change_to[cid];
                if (cur) { rb_mods_set_orientation(&g->mods, cid, "wait"); g->state_change_to[cid] = 0; }
                else { rb_mods_set_orientation(&g->mods, cid, "active"); g->state_change_to[cid] = 1; }
                rb_free_card(&c);
            }
        }
        if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = cid;
        count--;
    }
    rb_trigger_auto_abilities(g, 0, "状態変更時");
    rb_trigger_auto_abilities(g, 1, "状態変更時");
    rb_recalc_constants(g);
}

/* Mirror AbilityResolver::execute_energy_placement — draw energy from
   energy_deck and place in energy zone. */
void rb_effect_execute_energy_placement(GameState *g, int actor,
                                        AbilityEffect *e, int count) {
    if (!g || count <= 0) return;
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    const char *state_change = s_eff_extra(e, "state_change");
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < count; i++) {
        if (P->energy_deck.n == 0) break;
        int energy_id = P->energy_deck.cards[0];
        for (int j = 0; j < P->energy_deck.n - 1; j++)
            P->energy_deck.cards[j] = P->energy_deck.cards[j + 1];
        P->energy_deck.n--;
        if (P->energy.n < RB_MAX_ZONE) {
            P->energy.cards[P->energy.n++] = energy_id;
            if (state_change && !strcmp(state_change, "active"))
                P->energy_active = (P->energy_active + 1) > RB_ENERGY_CAP ? RB_ENERGY_CAP : P->energy_active + 1;
        }
        rb_record_movement(g, energy_id);
    }
}

/* Mirror AbilityResolver::execute_energy_state_change — change state of
   energy zone cards (wait/active). */
void rb_effect_execute_energy_state_change(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *state_change = s_eff_extra(e, "state_change");
    if (!state_change || !*state_change) return;
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    int count = e->count > 0 ? e->count : P->energy.n;
    if (!strcmp(state_change, "wait")) {
        for (int i = 0; i < count && i < P->energy.n; i++)
            P->energy_active = P->energy_active > 0 ? P->energy_active - 1 : 0;
    } else if (!strcmp(state_change, "active")) {
        for (int i = 0; i < count && i < P->energy.n; i++)
            P->energy_active = (P->energy_active + 1) > RB_ENERGY_CAP ? RB_ENERGY_CAP : P->energy_active + 1;
    } else if (!strcmp(state_change, "flip")) {
        int cur = P->energy_active;
        P->energy_active = (P->energy.n - cur) > RB_ENERGY_CAP ? RB_ENERGY_CAP : (P->energy.n - cur);
    }
    rb_recalc_constants(g);
}

/* Mirror AbilityResolver::execute_set_heart_type_applied — apply a resolved
   heart type to the target card. */
void rb_effect_execute_set_heart_type_applied(GameState *g, int actor,
                                               AbilityEffect *e) {
    if (!g) return;
    const char *heart_type = s_eff_extra(e, "heart_type");
    if (!heart_type || !*heart_type) heart_type = "heart00";
    int card_id = (g->n_selected_cards > 0) ? g->selected_cards[0] : g->queue.resume_host;
    if (card_id < 0) return;
    int color = 0;
    if (strcmp(heart_type, "heart01") == 0) color = 1;
    else if (strcmp(heart_type, "heart02") == 0) color = 2;
    else if (strcmp(heart_type, "heart03") == 0) color = 3;
    else if (strcmp(heart_type, "heart04") == 0) color = 4;
    else if (strcmp(heart_type, "heart05") == 0) color = 5;
    else if (strcmp(heart_type, "heart06") == 0) color = 6;
    else if (strcmp(heart_type, "heart07") == 0) color = 7;
    rb_mods_set_heart_color_multiplier(&g->mods, card_id, color);
    rb_rule_log_activated(g, card_id, "[[log_set_heart_type]]");
}

/* Mirror AbilityResolver::execute_set_cost — set cost modifier for cards. */
void rb_effect_execute_set_cost(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    int value = 0;
    const char *val_str = s_eff_extra(e, "value");
    if (val_str && *val_str) value = atoi(val_str);
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    const char *ctype = s_eff_extra(e, "card_type");
    int card_ids[RB_MAX_ZONE];
    int n = 0;
    if (ctype && !strcmp(ctype, "live")) {
        for (int i = 0; i < P->live.n; i++) card_ids[n++] = P->live.cards[i];
    } else if (ctype && !strcmp(ctype, "member")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] >= 0) card_ids[n++] = P->stage[i];
    } else {
        for (int i = 0; i < P->hand.n; i++) card_ids[n++] = P->hand.cards[i];
    }
    for (int i = 0; i < n; i++)
        rb_mods_set_cost(&g->mods, card_ids[i], value);
}

/* Mirror AbilityResolver::execute_set_blade_type — set blade type for stage members. */
void rb_effect_execute_set_blade_type(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *blade_type = s_eff_extra(e, "blade_type");
    if (!blade_type || !*blade_type) return;
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    int color = 0;
    if (!strcmp(blade_type, "red") || strcmp(blade_type, "赤ブレード") == 0) color = 1;
    else if (!strcmp(blade_type, "blue") || strcmp(blade_type, "青ブレード") == 0) color = 2;
    else if (!strcmp(blade_type, "green") || strcmp(blade_type, "緑ブレード") == 0) color = 3;
    else if (!strcmp(blade_type, "yellow") || strcmp(blade_type, "黄ブレード") == 0) color = 4;
    else if (!strcmp(blade_type, "purple") || strcmp(blade_type, "紫ブレード") == 0) color = 5;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (P->stage[i] < 0) continue;
        rb_mods_set_blade_type(&g->mods, P->stage[i], color);
    }
}

/* Mirror AbilityResolver::execute_set_heart_type — set heart type for cards.
   Handles self-target (activating_card) and member-target with selection. */
void rb_effect_execute_set_heart_type(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *heart_type = s_eff_extra(e, "heart_type");
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    int is_self = (e->self_target_field[0] && !strcmp(e->self_target_field, "true"));
    int color = 0;
    if (heart_type) {
        if (strcmp(heart_type, "heart01") == 0) color = 1;
        else if (strcmp(heart_type, "heart02") == 0) color = 2;
        else if (strcmp(heart_type, "heart03") == 0) color = 3;
        else if (strcmp(heart_type, "heart04") == 0) color = 4;
        else if (strcmp(heart_type, "heart05") == 0) color = 5;
        else if (strcmp(heart_type, "heart06") == 0) color = 6;
        else if (strcmp(heart_type, "heart07") == 0) color = 7;
    }
    if (is_self) {
        int card_id = g->queue.resume_host;
        if (card_id >= 0)
            rb_mods_set_heart_color_multiplier(&g->mods, card_id, color);
    } else {
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (P->stage[i] < 0) continue;
            rb_mods_set_heart_color_multiplier(&g->mods, P->stage[i], color);
        }
    }
}

/* Mirror AbilityResolver::execute_set_heart_copy_from_under — copy hearts from
   the card placed under a member onto the member itself. */
void rb_effect_execute_set_heart_copy_from_under(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    int member_card = (g->n_selected_cards > 0) ? g->selected_cards[0] : g->queue.resume_host;
    if (member_card < 0) return;
    rb_mods_set_heart_copy(&g->mods, member_card, g->queue.resume_host);
    rb_recalc_constants(g);
}

/* Mirror AbilityResolver::execute_activation_cost — modify activation cost. */
void rb_effect_execute_activation_cost(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *operation = s_eff_extra(e, "operation");
    if (!operation || !*operation) operation = "increase";
    int value = 0;
    const char *val_str = s_eff_extra(e, "value");
    if (val_str && *val_str) value = atoi(val_str);
    const char *target = e->target ? e->target : "self";
    if (!strcmp(target, "self") || !strcmp(target, "opponent")) {
        if (g->n_prohibition < 64) {
            snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[g->n_prohibition]),
                     "activation_cost_%s_%d", operation, value);
            g->n_prohibition++;
        }
    }
}

/* Mirror AbilityResolver::execute_set_blade_count — set blade count for stage members. */
void rb_effect_execute_set_blade_count(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    int value = e->count > 0 ? e->count : 0;
    const char *val_str = s_eff_extra(e, "value");
    if (val_str && *val_str) value = atoi(val_str);
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (P->stage[i] < 0) continue;
        rb_mods_set_blade(&g->mods, P->stage[i], value);
    }
}

/* Mirror AbilityResolver::execute_specify_heart_color — present heart color
   selection choice to the player (Q190). */
void rb_effect_execute_specify_heart_color(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *opts = "heart01,heart02,heart03,heart04,heart05,heart06";
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 0, opts);
    rb_choice_set_description(&g->queue.pending, "Choose a heart color");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
}

/* Mirror AbilityResolver::execute_set_card_identity_all_regions — set card
   identity across all regions. */
void rb_effect_execute_set_card_identity_all_regions(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *identities = s_eff_extra(e, "identities");
    if (identities && *identities && g->n_prohibition < 64) {
        snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[g->n_prohibition]),
                 "card_identity_all:%s", identities);
        g->n_prohibition++;
    }
}

/* Mirror AbilityResolver::execute_set_cost_to_use — set cost to use for abilities. */
void rb_effect_execute_set_cost_to_use(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    int value = 0;
    const char *val_str = s_eff_extra(e, "value");
    if (val_str && *val_str) value = atoi(val_str);
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    int card_id = (g->n_selected_cards > 0) ? g->selected_cards[0] : g->queue.resume_host;
    if (card_id >= 0)
        rb_mods_set_cost(&g->mods, card_id, value);
}

/* Mirror AbilityResolver::execute_all_blade_timing — handle all-blade timing. */
void rb_effect_execute_all_blade_timing(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (P->stage[i] < 0) continue;
        rb_mods_set_blade(&g->mods, P->stage[i], 1);
    }
}

/* Mirror AbilityResolver::execute_modify_cost — modify cost of cards. */
void rb_effect_execute_modify_cost(GameState *g, int actor, AbilityEffect *e) {
    if (!g) return;
    const char *operation = s_eff_extra(e, "operation");
    if (!operation || !*operation) operation = "increase";
    int value = 0;
    const char *val_str = s_eff_extra(e, "value");
    if (val_str && *val_str) value = atoi(val_str);
    const char *target = e->target ? e->target : "self";
    int pl = actor;
    if (!strcmp(target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    int sign = !strcmp(operation, "decrease") ? -1 : 1;
    int card_ids[RB_MAX_ZONE];
    int n = 0;
    const char *ctype = s_eff_extra(e, "card_type");
    if (ctype && !strcmp(ctype, "live")) {
        for (int i = 0; i < P->live.n; i++) card_ids[n++] = P->live.cards[i];
    } else if (ctype && !strcmp(ctype, "member")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] >= 0) card_ids[n++] = P->stage[i];
    } else {
        for (int i = 0; i < P->hand.n; i++) card_ids[n++] = P->hand.cards[i];
    }
    for (int i = 0; i < n; i++)
        rb_mods_set_cost(&g->mods, card_ids[i], sign * value);
}
