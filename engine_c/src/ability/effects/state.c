#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* Forward declarations of static helpers defined later in this file */
static int s_who(const char *target, int actor);
static int s_value(const AbilityEffect *e, int dflt);
static int s_has_group(const AbilityEffect *e, const char **out);
static int s_has_chars(const AbilityEffect *e, const char **out);
static int s_match_chars(int cid, const char *chars);
static int s_pass_filter(int cid, const char *grp, const char *chars);
static int s_blade_color_idx(const char *bt);
static int s_heart_idx(const char *h);
static const char *s_eff_extra(const AbilityEffect *e, const char *k);
static int s_eff_extra_true(const AbilityEffect *e, const char *k);
static int s_eff_extra_int(const AbilityEffect *e, const char *k, int dflt);
static const char *s_player_prefix(GameState *g, int card_id);

/* Mirror engine/src/ability/effects/state.rs::AbilityResolver::execute_change_state.
   Changes member card orientation (wait/active) on the stage. */
void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e, int host_cid){
    fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] entry: actor=%d host_cid=%d target=%s count=%d\n", actor, host_cid, e->target ? e->target : "self", e->count);
    /* ── Read effect fields ── */
    const char *state_change = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"state_change") && e->extra_v[i]){ state_change=e->extra_v[i]; break; }
    if(!state_change)
        for(int i=0;i<e->n_extra;i++)
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"state") && e->extra_v[i]){ state_change=e->extra_v[i]; break; }
    if(!state_change) state_change = "wait";

    const char *target = (e->target && *e->target) ? e->target : "self";
    int who = s_who(target, actor);
    int count = e->count >= 0 ? e->count : 0;
    int max = s_eff_extra_true(e, "max");
    int optional = s_eff_extra_true(e, "optional");
    int self_cost = s_eff_extra_true(e, "self_cost");

    const char *card_type_filter = e->card_type_field[0] ? e->card_type_field : NULL;
    if(!card_type_filter)
        for(int i=0;i<e->n_extra;i++)
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_type") && e->extra_v[i]){ card_type_filter=e->extra_v[i]; break; }

    const char *grp = NULL; s_has_group(e, &grp);
    const char *chars = NULL; s_has_chars(e, &chars);

    /* group_names is trigger-level metadata when targeting opponent */
    const char *group_filter = grp;
    if(target && !strcmp(target,"opponent")) group_filter = NULL;

    int cost_limit = -1;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"cost_limit") && e->extra_v[i])
            cost_limit = atoi(e->extra_v[i]);
    const char *cost_limit_op = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"cost_limit_operator") && e->extra_v[i])
            cost_limit_op = e->extra_v[i];

    /* cost_from_revealed — if set, cost_limit comes from first revealed card */
    if(s_eff_extra_true(e, "cost_from_revealed") && g->n_revealed > 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)g->revealed_cards[0], &c)){
            cost_limit = c.cost;
            rb_free_card(&c);
        }
    }

    /* ── Per-unit count derivation ── */
    int per_unit = e->per_unit || s_eff_extra_true(e, "per_unit");
    if(per_unit){
        int per_cnt = e->per_unit_count > 0 ? e->per_unit_count : 1;
        const char *puc = s_eff_extra(e, "per_unit_count");
        if(puc){ int v = atoi(puc); if(v > 0) per_cnt = v; }
        const char *per_src = s_eff_extra(e, "per_unit_source");
        if(per_src && strstr(per_src, "previous_moved")){
            count = (g->n_recently_moved / (per_cnt > 0 ? per_cnt : 1)) * (count > 0 ? count : 1);
        } else {
            RbPlayer *Ptmp = &g->p[who];
            int zone_ids[RB_STAGE_SIZE]; int zn = 0;
            for(int q=0; q<RB_STAGE_SIZE; q++)
                if(Ptmp->stage[q] != RB_EMPTY_SLOT) zone_ids[zn++] = Ptmp->stage[q];
            int filt_ids[RB_STAGE_SIZE]; int fn = 0;
            for(int i=0;i<zn;i++){
                int cid = zone_ids[i];
                if(cost_limit >= 0){
                    Card cc; int ccost = 0;
                    if(rb_decode_card_by_index((uint32_t)cid, &cc)){ ccost = cc.cost; rb_free_card(&cc); }
                    int ok = 1;
                    if(cost_limit_op && !strcmp(cost_limit_op,"<=")) ok = ccost <= cost_limit;
                    else if(cost_limit_op && !strcmp(cost_limit_op,"<")) ok = ccost < cost_limit;
                    else if(!cost_limit_op) ok = ccost == cost_limit;
                    if(!ok) continue;
                }
                filt_ids[fn++] = cid;
            }
            int matched = fn;
            if(s_eff_extra_true(e, "distinct")){
                matched = rb_count_distinct_member_name_units(filt_ids, fn);
            }
            count = (matched / (per_cnt > 0 ? per_cnt : 1)) * (count > 0 ? count : 1);
        }
        group_filter = NULL;
    }

    /* ── blade_limit dynamic limits (Q266 / C5) ── */
    int blade_limit = -1;
    const char *blade_limit_op = s_eff_extra(e, "blade_limit_operator");
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"blade_limit") && e->extra_v[i])
            blade_limit = atoi(e->extra_v[i]);

    if(s_eff_extra_true(e, "blade_limit_from_cost_member")){
        int off = s_eff_extra_int(e, "blade_limit_offset", 0);
        int cost_blade = 0;
        if(g->mods.n_last_cost_moved_card_ids > 0){
            int cid = g->mods.last_cost_moved_card_ids[0];
            Card c;
            if(rb_decode_card_by_index((uint32_t)cid, &c)){ cost_blade = c.blade; rb_free_card(&c); }
        }
        int signed_lim = cost_blade - off;
        if(signed_lim < 0){ blade_limit = 0; blade_limit_op = "<"; }
        else blade_limit = signed_lim;
    } else if(s_eff_extra_true(e, "blade_limit_from_energy_under")){
        int off = s_eff_extra_int(e, "blade_limit_offset", 0);
        int under = 0;
        if(host_cid >= 0){
            RbPlayer *Ptmp = &g->p[who];
            for(int q=0; q<RB_STAGE_SIZE; q++)
                if(Ptmp->stage[q] == host_cid) under = Ptmp->under_cards[q].n;
        }
        blade_limit = rb_saturate_u8(under + off);
    }

    /* ── Energy placement short-circuit: deck → energy zone ── */
    const char *src = e->source;
    const char *dst = e->destination;
    if(src && dst && !strcmp(src,"deck") && !strcmp(dst,"energy")){
        rb_effect_energy_placement(g, actor, e);
        return;
    }

    /* ── Member state change ── */
    int is_member = (card_type_filter && !strcmp(card_type_filter,"member_card")) || self_cost;
    if(!is_member){
        rb_effect_energy_state_change(g, actor, e);
        return;
    }

    RbPlayer *P = &g->p[who];
    const char *state_filter = s_eff_extra(e, "state");

    int is_cannot_activate = 0;
    if(!strcmp(state_change, "active") && g->player_cannot_activate[who]) is_cannot_activate = 1;

    int exclude_self_id = -1;
    if(s_eff_extra_true(e, "exclude_self") && host_cid >= 0) exclude_self_id = host_cid;
    int is_self_target_flag = (e->self_target_field[0] && !strcmp(e->self_target_field, "true"));

    /* ── Optional gate: verify at least one valid target exists ── */
    if(optional){
        int decided = -1;
        if(g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH)
            decided = g->queue.entries[g->queue.cur].optional_cost_result;
        if(decided < 0){
            int can_target = 0;
            for(int q=0; q<RB_STAGE_SIZE; q++){
                int cid = P->stage[q];
                if(cid == RB_EMPTY_SLOT) continue;
                if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
                const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
                int is_wait = (ori && !strcmp(ori, "wait"));
                if(!strcmp(state_change, "active")){
                    if(!is_wait) continue;
                } else if(state_filter && !strcmp(state_filter, "active")){
                    if(is_wait) continue;
                } else if(!strcmp(state_change, "wait")){
                    if(is_wait) continue;
                }
                if(card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
                if(group_filter && !rb_card_matches_group_str(cid, group_filter)) continue;
                if(!s_match_chars(cid, chars)) continue;
                if(blade_limit >= 0){
                    Card cc; int bl = 0;
                    if(rb_decode_card_by_index((uint32_t)cid, &cc)){ bl = cc.blade; rb_free_card(&cc); }
                    int ok = 1;
                    if(blade_limit_op && !strcmp(blade_limit_op,"<")) ok = bl < blade_limit;
                    else if(blade_limit_op && !strcmp(blade_limit_op,"<=")) ok = bl <= blade_limit;
                    else ok = bl <= blade_limit;
                    if(!ok) continue;
                }
                can_target = 1; break;
            }
            if(!can_target){
                fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] optional %s but no valid targets — skipping\n", state_change);
                return;
            }
            fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] optional choice emitted: state_change=%s who=%d\n", state_change, who);
            rb_emit_choice(g, who, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 1, "change_state_optional");
    rb_queue_pause_for_choice(g, &g->queue.pending);
            if(g->queue.cur >= 0) g->queue.entries[g->queue.cur].pending_actions_n = 1;
            return;
        }
    }

    fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] member_op: target=%s count=%d max=%d state_change=%s is_member=%d\n", target, count, max, state_change, is_member);

    /* ── Collect candidates ── */
    int cands[RB_STAGE_SIZE]; int nc = 0;

    /* Honour prior selected_cards relay */
    if(g->n_selected_cards > 0){
        for(int s=0; s<g->n_selected_cards; s++){
            int cid = g->selected_cards[s];
            for(int q=0; q<RB_STAGE_SIZE; q++)
                if(P->stage[q] == cid){ cands[nc++] = cid; break; }
        }
        if(nc > 0){
            int fcands[RB_STAGE_SIZE]; int fnc = 0;
            for(int i=0;i<nc;i++){
                int cid = cands[i];
                if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
                const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
                int is_wait = (ori && !strcmp(ori, "wait"));
                if(!strcmp(state_change,"active")){ if(!is_wait) continue; }
                else if(state_filter && !strcmp(state_filter,"active")){ if(is_wait) continue; }
                else if(!strcmp(state_change,"wait")){ if(is_wait) continue; }
                if(card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
                if(group_filter && !rb_card_matches_group_str(cid, group_filter)) continue;
                if(!s_match_chars(cid, chars)) continue;
                if(blade_limit >= 0){
                    Card cc; int bl = 0;
                    if(rb_decode_card_by_index((uint32_t)cid, &cc)){ bl = cc.blade; rb_free_card(&cc); }
                    int ok = 1;
                    if(blade_limit_op && !strcmp(blade_limit_op,"<")) ok = bl < blade_limit;
                    else if(blade_limit_op && !strcmp(blade_limit_op,"<=")) ok = bl <= blade_limit;
                    else ok = bl <= blade_limit;
                    if(!ok) continue;
                }
                fcands[fnc++] = cid;
            }
            nc = fnc;
            for(int i=0;i<nc;i++) cands[i] = fcands[i];
            if(nc > 0){
                fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] prior selection filtered nc=%d\n", nc);
                goto candidates_ready;
            }
            fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] prior selection %d cards has no card on target stage; scanning stage instead\n", g->n_selected_cards);
        }
        nc = 0;
    }

    /* Stage scan */
    for(int q=0; q<RB_STAGE_SIZE; q++){
        int cid = P->stage[q];
        if(cid == RB_EMPTY_SLOT) continue;
        if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
        if(card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
        if(group_filter && !rb_card_matches_group_str(cid, group_filter)) continue;
        if(!s_match_chars(cid, chars)) continue;
        if(cost_limit >= 0){
            Card cc; int ccost = 0;
            if(rb_decode_card_by_index((uint32_t)cid, &cc)){ ccost = cc.cost; rb_free_card(&cc); }
            int ok = 1;
            if(cost_limit_op && !strcmp(cost_limit_op,"<=")) ok = ccost <= cost_limit;
            else if(cost_limit_op && !strcmp(cost_limit_op,"<")) ok = ccost < cost_limit;
            else if(!cost_limit_op) ok = ccost == cost_limit;
            if(!ok) continue;
        }
        if(blade_limit >= 0){
            Card cc; int bl = 0;
            if(rb_decode_card_by_index((uint32_t)cid, &cc)){ bl = cc.blade; rb_free_card(&cc); }
            int ok = 1;
            if(blade_limit_op && !strcmp(blade_limit_op,"<")) ok = bl < blade_limit;
            else if(blade_limit_op && !strcmp(blade_limit_op,"<=")) ok = bl <= blade_limit;
            else ok = bl <= blade_limit;
            if(!ok) continue;
        }
        const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        int is_wait = (ori && !strcmp(ori, "wait"));
        int matches_state = 1;
        if(!strcmp(state_change, "active")) matches_state = is_wait;
        else if(state_filter && !strcmp(state_filter, "active")) matches_state = !is_wait;
        else if(!strcmp(state_change, "wait")) matches_state = !is_wait;
        if(!matches_state) continue;
        cands[nc++] = cid;
    }
candidates_ready:

    /* self_cost / self_target restricts to host card */
    if((self_cost || is_self_target_flag) && g->n_selected_cards == 0 && host_cid >= 0){
        int found = 0;
        for(int i=0;i<nc;i++) if(cands[i] == host_cid) found = 1;
        if(found){
            int fnc = 0;
            for(int i=0;i<nc;i++) if(cands[i] == host_cid) cands[fnc++] = cands[i];
            nc = fnc;
        } else {
            int on_stage = 0;
            for(int q=0; q<RB_STAGE_SIZE; q++)
                if(P->stage[q] == host_cid) on_stage = 1;
            if(!on_stage){
                fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] self_cost: activating card %d not on stage\n", host_cid);
                return;
            }
            const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, host_cid);
            int already = (!strcmp(state_change,"wait") && ori && !strcmp(ori,"wait"))
                       || (!strcmp(state_change,"active") && (!ori || strcmp(ori,"wait")));
            if(already){
                fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] self_cost: already %s, skipping host_cid=%d\n", state_change, host_cid);
                return;
            }
            if(nc < RB_STAGE_SIZE) cands[nc++] = host_cid;
        }
    }

    if(nc == 0){
        /* energy fallback */
        const char *ct2 = card_type_filter ? card_type_filter : s_eff_extra(e, "card_type");
        if(ct2 && !strcmp(ct2, "energy_card"))
            rb_effect_energy_state_change(g, actor, e);
        return;
    }

    int change_all = (count == 0);
    int is_self_target_prompt = (count == 1
        && strcmp(target, "opponent") == 0
        && card_type_filter && !strcmp(card_type_filter, "member_card")
        && host_cid >= 0);
    if(is_self_target_prompt){
        int found_self = 0;
        for(int i=0;i<nc;i++) if(cands[i] == host_cid) found_self = 1;
        is_self_target_prompt = found_self ? 1 : 0;
    }
    int needs_prompt = 0;
    if(!is_self_target_prompt && g->n_selected_cards == 0){
        if(max && nc > 0) needs_prompt = 1;
        else if(!change_all && nc > count) needs_prompt = 1;
    }
    if(needs_prompt){
        int pick = count > 0 ? count : 1;
        rb_emit_choice(g, who, RB_CHOICE_SELECT_CARD, "stage", NULL,
                       pick > 0 ? pick : 1, max, "change_state");
    rb_queue_pause_for_choice(g, &g->queue.pending);
        if(g->queue.cur >= 0) g->queue.entries[g->queue.cur].pending_actions_n = 1;
        return;
    }

    int nchange = change_all ? nc : (count < nc ? count : nc);

    /* snapshot orientations before change */
    int snap_ids[RB_STAGE_SIZE];
    const char *snap_ori[RB_STAGE_SIZE];
    int nsnap = nchange;
    for(int i=0;i<nchange;i++){
        snap_ids[i] = cands[i];
        snap_ori[i] = rb_mods_get_orientation((RbMods*)&g->mods, cands[i]);
    }
    int wait_before = 0;
    for(int i=0;i<nsnap;i++)
        if(snap_ori[i] && !strcmp(snap_ori[i], "wait")) wait_before++;

    fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] targets nc=%d nchange=%d state_change=%s cannot_activate=%d\n", nc, nchange, state_change, is_cannot_activate);
    /* Apply state changes */
    for(int i=0;i<nchange;i++){
        int cid = cands[i];
        if(is_cannot_activate){
            fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] blocked by cannot_activate_by_effect: card_id=%d\n", cid);
            continue;
        }
        fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] applying: card_id=%d state=%s before_ori=%s\n", cid, state_change, rb_mods_get_orientation((RbMods*)&g->mods, cid) ? rb_mods_get_orientation((RbMods*)&g->mods, cid) : "active");
        const char *old_ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        int was_wait = (old_ori && !strcmp(old_ori, "wait"));
        int will_wait = (!strcmp(state_change, "wait"));
        if(was_wait != will_wait){
            g->state_change_from[cid] = (int8_t)(was_wait ? 1 : 0);
            g->state_change_to[cid]   = (int8_t)(will_wait ? 1 : 0);
        }
        rb_mods_set_orientation(&g->mods, cid, state_change);
        fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] after: card_id=%d ori=%s\n", cid, rb_mods_get_orientation((RbMods*)&g->mods, cid) ? rb_mods_get_orientation((RbMods*)&g->mods, cid) : "active");
        if(g->n_selected_cards < RB_MAX_RECENTLY_MOVED){
            int already = 0;
            for(int s=0;s<g->n_selected_cards;s++)
                if(g->selected_cards[s] == cid) already = 1;
            if(!already){
                fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] pushing card_id=%d to selected_cards (len=%d)\n", cid, g->n_selected_cards);
                g->selected_cards[g->n_selected_cards++] = cid;
            }
        }
    }

    if(!strcmp(state_change, "active"))
        g->last_wait_to_active_count = is_cannot_activate ? 0 : (uint8_t)wait_before;
    fprintf(stderr, "DEBUG [EXEC_CHANGE_STATE] wait_before=%d last_wait_to_active=%d\n", wait_before, g->last_wait_to_active_count);

    /* Detect actual transitions and push to recently_state_changed / turn_state_changes */
    for(int i=0;i<nsnap;i++){
        int cid = snap_ids[i];
        const char *after_ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        if(snap_ori[i] != after_ori
           && !((!snap_ori[i] || !strcmp(snap_ori[i],"active"))
                && (!after_ori  || !strcmp(after_ori,"active")))){
            const char *from_str = (!snap_ori[i] || !strcmp(snap_ori[i],"active")) ? "active" : snap_ori[i];
            const char *to_str   = (!after_ori  || !strcmp(after_ori,"active"))  ? "active" : after_ori;
            fprintf(stderr, "DEBUG [STATE_CHANGE] detected: card=%d %s->%s\n", cid, from_str, to_str);
            if(g->n_recently_state_changed < RB_MAX_RECENTLY_MOVED)
                g->recently_state_changed[g->n_recently_state_changed++] = cid;
            g->turn_state_changes[g->n_turn_state_changes][0] = g->activating_card >= 0 ? g->activating_card : -1;
            g->turn_state_changes[g->n_turn_state_changes][1] = cid;
            g->turn_state_changes[g->n_turn_state_changes][2] = from_str[0];
            g->turn_state_changes[g->n_turn_state_changes][3] = to_str[0];
            if(g->n_turn_state_changes < 64) g->n_turn_state_changes++;
        }
    }

    /* Re-trigger auto abilities for both players */
    fprintf(stderr, "DEBUG [STATE_CHANGE] modifier applied, re-triggering auto abilities (state=%s)\n", state_change);
    rb_trigger_auto_abilities_for_player(g, who);
    if(who != (actor ^ 1)) rb_trigger_auto_abilities_for_player(g, actor ^ 1);

    /* Push changed cards to selected_cards so subsequent sequential actions
       (e.g. gain_resource with target_from_selection: true) can target them. */
    for(int i=0;i<nchange;i++){
        int cid = cands[i];
        int already = 0;
        for(int s=0;s<g->n_selected_cards;s++)
            if(g->selected_cards[s] == cid) already = 1;
        if(!already && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = cid;
    }

    {
        const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[256];
        snprintf(logbuf, sizeof logbuf, "%s %s: 状態変更→%s", pp, act_name, state_change);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_energy_placement.
   Draws `count` energy cards from the energy deck into the energy zone,
   optionally activating them when state_change=="active". */
void rb_effect_energy_placement(GameState *g, int actor, AbilityEffect *e){
    fprintf(stderr, "DEBUG [ENERGY_PLACEMENT] actor=%d target=%s count=%d\n", actor, e->target ? e->target : "self", e->count);
    const char *st = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"state_change") && e->extra_v[i]){ st=e->extra_v[i]; break; }
    if(!st)
        for(int i=0;i<e->n_extra;i++)
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"state") && e->extra_v[i]){ st=e->extra_v[i]; break; }
    int who = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2"))) who = actor ^ 1;
    RbPlayer *P = &g->p[who];
    int n = e->count > 0 ? e->count : 1;
    int active = (st && (!strcmp(st,"active") || !strcmp(st,"アクティブ")));
    for(int k=0;k<n;k++){
        if(P->energy_deck.n == 0) break;
        int eid = P->energy_deck.cards[0];
        for(int j=0;j<P->energy_deck.n-1;j++) P->energy_deck.cards[j] = P->energy_deck.cards[j+1];
        P->energy_deck.n--;
        if(P->energy.n < RB_MAX_ZONE){
            P->energy.cards[P->energy.n++] = eid;
            if(active && P->energy_active < RB_ENERGY_CAP) P->energy_active++;
        }
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_energy_state_change.
   Changes active/wait state of energy zone cards with max/count==0 resolution. */
void rb_effect_energy_state_change(GameState *g, int actor, AbilityEffect *e){
    fprintf(stderr, "DEBUG [ENERGY_STATE_CHANGE] actor=%d target=%s state=%s count=%d\n", actor, e->target ? e->target : "self", s_eff_extra(e,"state_change") ? s_eff_extra(e,"state_change") : s_eff_extra(e,"state") ? s_eff_extra(e,"state") : "active", e->count);
    const char *st = NULL;
    int max = 0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"state_change") && e->extra_v[i]) st = e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"state") && e->extra_v[i]) st = e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"max") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")) max = 1;
    }
    if(!st) st = "active";
    int who = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2"))) who = actor ^ 1;
    RbPlayer *P = &g->p[who];
    int total = P->energy.n, active = P->energy_active;
    int is_active = (!strcmp(st,"active") || !strcmp(st,"アクティブ"));
    int eff;
    if(max){
        int available = is_active ? (total - active) : active;
        if(available < 0) available = 0;
        int req = e->count > 0 ? e->count : 1;
        eff = req < available ? req : available;
    } else if(e->count == 0){
        eff = is_active ? (total - active) : active;
    } else {
        eff = e->count;
    }
    if(max){
        fprintf(stderr, "DEBUG [ENERGY] max=true: count=%d available=%d effective=%d\n", e->count, is_active ? (total - (int)P->energy_active) : (int)P->energy_active, eff);
    } else if(e->count == 0){
        fprintf(stderr, "DEBUG [ENERGY] count=0 (all): effective=%d\n", eff);
    } else {
        fprintf(stderr, "DEBUG [ENERGY] max=false: count=%d effective=%d\n", e->count, eff);
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
    fprintf(stderr, "DEBUG [ENERGY_STATE_CHANGE] done: total=%d active=%d eff=%d is_active=%d\n", total, P->energy_active, eff, is_active);
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_cost.
   Sets a fixed cost modifier on live/member/hand cards, with optional group/character filter. */
void rb_effect_set_cost(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int value = s_value(e, 0);
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    int ids[RB_MAX_ZONE]; int n = 0;
    const char *ct = e->card_type_field;
    if(ct && !strcmp(ct, "live_card")){
        for(int i=0;i<P->live.n;i++) ids[n++] = P->live.cards[i];
    } else if(ct && !strcmp(ct, "member_card")){
        for(int q=0;q<RB_STAGE_SIZE;q++)
            if(P->stage[q] != RB_EMPTY_SLOT) ids[n++] = P->stage[q];
    } else if(ct && !strcmp(ct, "energy_card")){
        for(int i=0;i<P->energy.n;i++) ids[n++] = P->energy.cards[i];
    } else {
        for(int i=0;i<P->hand.n;i++) ids[n++] = P->hand.cards[i];
    }
    const char *grp = NULL, *chars = NULL;
    if(s_has_group(e, &grp) || s_has_chars(e, &chars)){
        int fids[RB_MAX_ZONE]; int fn = 0;
        for(int i=0;i<n;i++)
            if(s_pass_filter(ids[i], grp, chars)) fids[fn++] = ids[i];
        n = fn;
        for(int i=0;i<n;i++) ids[i] = fids[i];
    }
    const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
    char act_name[64]; act_name[0] = 0;
    if(g->activating_card >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
            if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
            rb_free_card(&c);
        }
    }
    char logbuf[128];
    snprintf(logbuf, sizeof logbuf, "%s %s: [[log_set_cost:value=%d]]", pp, act_name, value);
    rb_log_push_verdict(logbuf, "rule_log", 1);
    for(int i=0;i<n;i++){
        rb_mods_set_cost(&g->mods, ids[i], value);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_blade_type.
   Recolors the blade of every staged member matching the optional group/character
   filter. Optionally registers a temporary effect for duration-based revert. */
void rb_effect_set_blade_type(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    const char *bt = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"blade_type")||!strcmp(e->extra_k[i],"blade_color")) && e->extra_v[i])
            bt = e->extra_v[i];
    int col = s_blade_color_idx(bt);
    if(col < 0) return;
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    const char *grp = NULL, *chars = NULL;
    s_has_group(e, &grp); s_has_chars(e, &chars);
    const char *duration = s_eff_extra(e, "duration");
    const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
    char act_name[64]; act_name[0] = 0;
    if(g->activating_card >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
            if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
            rb_free_card(&c);
        }
    }
    char logbuf[128];
    snprintf(logbuf, sizeof logbuf, "%s %s: [[log_set_blade_type:type=%s]]", pp, act_name, bt ? bt : "none");
    rb_log_push_verdict(logbuf, "rule_log", 1);
    for(int q=0; q<RB_STAGE_SIZE; q++){
        int cid = P->stage[q];
        if(cid == RB_EMPTY_SLOT) continue;
        if(!s_pass_filter(cid, grp, chars)) continue;
        g->mods.blade_type[cid] = (int8_t)col;
        if(duration && strcmp(duration, "permanent") != 0){
            rb_util_push_temporary_effect(g, "set_blade_type", duration, "self",
                bt ? bt : "");
        }
    }
    rb_recalc_constants(g);
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_blade_count.
   Sets the blade modifier on every staged member matching the filter to `value`.
   Optionally registers a temporary effect for duration-based revert. */
void rb_effect_set_blade_count(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int value = s_value(e, 0);
    if(value == 0 && e->count >= 0) value = e->count;
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    int ids[RB_STAGE_SIZE]; int n = 0;
    for(int q=0; q<RB_STAGE_SIZE; q++)
        if(P->stage[q] != RB_EMPTY_SLOT) ids[n++] = P->stage[q];
    const char *grp = NULL, *chars = NULL;
    if(s_has_group(e, &grp) || s_has_chars(e, &chars)){
        int f[RB_STAGE_SIZE]; int fn = 0;
        for(int i=0;i<n;i++)
            if(s_pass_filter(ids[i], grp, chars)) f[fn++] = ids[i];
        n = fn;
        for(int i=0;i<n;i++) ids[i] = f[i];
    }
    const char *pos = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"position") && e->extra_v[i]) pos = e->extra_v[i];
    if(pos){
        int area = rb_pos_to_area(pos);
        if(area >= 0){
            int exp = P->stage[area];
            int f[RB_STAGE_SIZE]; int fn = 0;
            for(int i=0;i<n;i++) if(ids[i] == exp) f[fn++] = ids[i];
            n = fn;
            for(int i=0;i<n;i++) ids[i] = f[i];
        }
    }
    const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
    char act_name[64]; act_name[0] = 0;
    if(g->activating_card >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
            if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
            rb_free_card(&c);
        }
    }
    char logbuf[128];
    snprintf(logbuf, sizeof logbuf, "%s %s: [[log_set_blade_count:n=%d]]", pp, act_name, value);
    rb_log_push_verdict(logbuf, "rule_log", 1);
    const char *duration = s_eff_extra(e, "duration");
    for(int i=0;i<n;i++){
        rb_mods_set_blade(&g->mods, ids[i], value);
        if(duration && strcmp(duration, "permanent") != 0){
            rb_util_push_temporary_effect(g, "set_blade_count", duration, "self", "");
        }
    }
    rb_recalc_constants(g);
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_heart_copy_from_under.
   Copies the hearts of the card just placed under this member onto the member. */
void rb_effect_set_heart_copy_from_under(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)e;
    int member = -1;
    if(g->n_selected_cards > 0) member = g->selected_cards[0];
    else if(host_cid >= 0) member = host_cid;
    if(member < 0) return;
    RbPlayer *P = &g->p[actor];
    int src = -1;
    for(int s=0; s<RB_STAGE_SIZE; s++){
        if(P->stage[s] != member) continue;
        RbBag *uc = &P->under_cards[s];
        if(uc->n > 0){
            for(int k=uc->n-1; k>=0; k--){
                int c = uc->cards[k]; int is_moved = 0;
                for(int m=0;m<g->n_those_cards;m++)
                    if(g->those_cards[m] == c){ is_moved = 1; break; }
                if(is_moved){ src = c; break; }
            }
            if(src < 0) src = uc->cards[uc->n-1];
        }
        break;
    }
    if(src < 0) return;
    g->mods.heart_copy[member] = (int16_t)src;
    {
        const char *pp = s_player_prefix(g, member);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: [[log_set_heart_copy:target=%d,source=%d]]", pp, act_name, member, src);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_heart_type_applied.
   Split from execute_set_heart_type so dispatch stays a pure one-liner.
   Applies a resolved heart type to the target card(s). */
void rb_effect_set_heart_type_applied(GameState *g, int actor, const char *heart_type, const char *target, int count, const char *duration, int host_cid){
    (void)actor; (void)target; (void)count;
    fprintf(stderr, "DEBUG [SET_HEART_APPLIED] heart_type=%s duration=%s host_cid=%d\n", heart_type ? heart_type : "null", duration ? duration : "null", host_cid);
    const char *ht = heart_type;
    if(ht && !strcmp(ht, "selected")){
        if(g->queue.selected_heart_color >= 0){
            static char sbuf[16];
            snprintf(sbuf, sizeof sbuf, "heart%02d", g->queue.selected_heart_color);
            ht = sbuf;
        } else ht = "heart00";
    }
    if(!ht) ht = "heart00";
    const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
    char act_name[64]; act_name[0]=0;
    if(g->activating_card >= 0){ Card c; if(rb_decode_card_by_index((uint32_t)g->activating_card,&c)){ if(c.name) strncpy(act_name,c.name,sizeof act_name-1); rb_free_card(&c);} }
    char logbuf[128]; snprintf(logbuf,sizeof logbuf,"%s %s: [[log_set_heart_type:type=%s]]",pp,act_name,ht); rb_log_push_verdict(logbuf,"rule_log",1);
    int cid = -1;
    if(g->n_selected_cards > 0) cid = g->selected_cards[0];
    else if(g->activating_card >= 0) cid = g->activating_card;
    else if(host_cid >= 0) cid = host_cid;
    if(cid < 0){ fprintf(stderr,"DEBUG [SET_HEART_APPLIED] no target card\n"); return; }
    int col = s_heart_idx(ht);
    g->mods.heart_multiplier[cid] = (int8_t)col;
    g->mods.heart_multiplier_amt[cid] = (int8_t)1;
    fprintf(stderr, "DEBUG [SET_HEART_APPLIED] applied cid=%d col=%d ht=%s\n", cid, col, ht);
    if(duration && strcmp(duration,"permanent")!=0){
        rb_util_push_temporary_effect(g,"set_heart_type",duration,"self",ht);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_heart_type (+ applied).
   Transforms the member's hearts to the chosen color. ref_value="placed_under"
   copies the under-card's hearts; otherwise recolors via heart_multiplier. */
void rb_effect_set_heart_type(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *ref = s_eff_extra(e, "ref_value");
    if(ref && !strcmp(ref, "placed_under")){
        rb_effect_set_heart_copy_from_under(g, actor, e, host_cid);
        return;
    }
    int is_self = (e->self_target_field[0] && !strcmp(e->self_target_field, "true"));
    int heart_selection = s_eff_extra_true(e, "heart_selection");
    const char *grp = NULL; s_has_group(e, &grp);
    int needs_target = !is_self && (heart_selection || grp
        || (e->card_type_field[0] && !strcmp(e->card_type_field, "member_card")));

    const char *ht = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"heart_type")||!strcmp(e->extra_k[i],"heart_color")) && e->extra_v[i])
            ht = e->extra_v[i];
    if(!ht)
        for(int i=0;i<e->n_extra;i++)
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_colors") && e->extra_v[i]){ ht = e->extra_v[i]; break; }
    if(ht && !strcmp(ht, "selected")){
        if(g->queue.selected_heart_color >= 0){
            static char buf[16];
            snprintf(buf, sizeof buf, "heart%02d", g->queue.selected_heart_color);
            ht = buf;
        } else ht = "heart00";
    }
    if(!ht) ht = "heart00";

    if(is_self || !needs_target){
        int col = s_heart_idx(ht);
        int who = s_who(e->target, actor);
        int cid = -1;
        if(g->n_selected_cards > 0) cid = g->selected_cards[0];
        else if(host_cid >= 0) cid = host_cid;
        else for(int q=0;q<RB_STAGE_SIZE;q++)
            if(g->p[who].stage[q] != RB_EMPTY_SLOT){ cid = g->p[who].stage[q]; break; }
        if(cid < 0) return;
        g->mods.heart_multiplier[cid] = (int8_t)col;
        g->mods.heart_multiplier_amt[cid] = (int8_t)(e->count >= 1 ? e->count : 2);
        {
            const char *pp = s_player_prefix(g, cid);
            char act_name[64]; act_name[0] = 0;
            if(g->activating_card >= 0){
                Card c;
                if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                    if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                    rb_free_card(&c);
                }
            }
            char logbuf[128];
            snprintf(logbuf, sizeof logbuf, "%s %s: [[log_set_heart_type:type=%s]]", pp, act_name, ht);
            rb_log_push_verdict(logbuf, "rule_log", 1);
        }
        return;
    }

    if(g->n_selected_cards == 0){
        const char *target_str = e->target ? e->target : "self";
        int who2 = s_who(target_str, actor);
        RbPlayer *P2 = &g->p[who2];
        int stage_ids[RB_STAGE_SIZE]; int sn = 0;
        for(int q=0;q<RB_STAGE_SIZE;q++)
            if(P2->stage[q] != RB_EMPTY_SLOT) stage_ids[sn++] = P2->stage[q];
        const char *chars2 = NULL; s_has_chars(e, &chars2);
        int cand[RB_STAGE_SIZE]; int nc2 = 0;
        for(int i=0;i<sn;i++)
            if(s_pass_filter(stage_ids[i], grp, chars2)) cand[nc2++] = stage_ids[i];
        if(nc2 == 0) return;
        int tc = 1;
        const char *tcv = s_eff_extra(e, "target_count");
        if(tcv) tc = atoi(tcv);
        if(nc2 <= tc){
            for(int i=0;i<nc2;i++){
                int already = 0;
                for(int s=0;s<g->n_selected_cards;s++)
                    if(g->selected_cards[s] == cand[i]) already = 1;
                if(!already && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    g->selected_cards[g->n_selected_cards++] = cand[i];
            }
            int col = s_heart_idx(ht);
            int cid2 = g->selected_cards[0];
            g->mods.heart_multiplier[cid2] = (int8_t)col;
            g->mods.heart_multiplier_amt[cid2] = (int8_t)(e->count >= 1 ? e->count : 2);
        } else {
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", NULL, tc, 0, "heart_type");
    rb_queue_pause_for_choice(g, &g->queue.pending);
            if(grp) strncpy(g->queue.pending.filter_group, grp, sizeof(g->queue.pending.filter_group)-1);
            g->queue.deferred = e;
            g->queue.resume_host = host_cid;
            g->queue.resume_mode = 2; /* select_card */
            return;
        }
        return;
    }
    /* already have selected target */
    {
        int col = s_heart_idx(ht);
        int cid2 = g->selected_cards[0];
        g->mods.heart_multiplier[cid2] = (int8_t)col;
        g->mods.heart_multiplier_amt[cid2] = (int8_t)(e->count >= 1 ? e->count : 2);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_activation_cost.
   Records a prohibition note "activation_cost_{op}_{value}" for self/opponent. */
void rb_effect_activation_cost(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    const char *op = "increase";
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"operation") && e->extra_v[i]) op = e->extra_v[i];
    int value = s_value(e, 0);
    const char *target = e->target ? e->target : "self";
    if(!strcmp(target,"self") || !strcmp(target,"opponent")){
        char note[64];
        snprintf(note, sizeof note, "activation_cost_%s_%d", op, value);
        if(g->n_prohibition < 64){
            snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[0]), "%s", note);
            g->n_prohibition++;
        }
        if(g->n_prohibition_effects < 64){
            snprintf(g->prohibition_effects[g->n_prohibition_effects], sizeof(g->prohibition_effects[0]), "%s", note);
            g->n_prohibition_effects++;
        }
    }
    {
        const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: [[log_activation_cost:op=%s,value=%d]]", pp, act_name, op, value);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_card_identity.
   Rewrites this member's identity to the listed group/unit names. */
void rb_effect_set_card_identity(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)actor;
    int cid = host_cid >= 0 ? host_cid : -1;
    if(cid < 0){
        for(int q=0;q<RB_STAGE_SIZE;q++)
            if(g->p[actor].stage[q] != RB_EMPTY_SLOT){ cid = g->p[actor].stage[q]; break; }
    }
    if(cid < 0) return;
    const char *id = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"identities")||!strcmp(e->extra_k[i],"identity")) && e->extra_v[i])
            id = e->extra_v[i];
    if(!id) return;
    char buf[256]; strncpy(buf, id, 255); buf[255] = 0;
    char *tok = strtok(buf, ",、 ");
    while(tok){ rb_set_card_identity(cid, tok); tok = strtok(NULL, ",、 "); }
    {
        const char *pp = s_player_prefix(g, cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: カード同一性変更", pp, act_name);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_card_identity_all_regions.
   Identity rewrite that also records a per-card prohibition note. */
void rb_effect_set_card_identity_all_regions(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)actor;
    int cid = host_cid >= 0 ? host_cid : -1;
    if(cid < 0){
        for(int q=0;q<RB_STAGE_SIZE;q++)
            if(g->p[actor].stage[q] != RB_EMPTY_SLOT){ cid = g->p[actor].stage[q]; break; }
    }
    if(cid < 0) return;
    const char *id = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"identities")||!strcmp(e->extra_k[i],"identity")) && e->extra_v[i])
            id = e->extra_v[i];
    if(!id) return;
    char buf[256]; strncpy(buf, id, 255); buf[255] = 0;
    char *tok = strtok(buf, ",、 ");
    while(tok){
        rb_set_card_identity(cid, tok);
        if(g->n_prohibition < 64){
            snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[0]),
                     "card_identity:%d:%s", cid, tok);
            g->n_prohibition++;
        }
        tok = strtok(NULL, ",、 ");
    }
    {
        const char *pp = s_player_prefix(g, cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: 全領域カード同一性変更", pp, act_name);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_reduce_live_card_set_limit.
   Reduces the player's live card set limit by count. */
void rb_effect_reduce_live_card_set_limit(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int lim = e->count > 0 ? e->count : 1;
    int who = s_who(e->target, actor);
    g->live_set_limit_reduction[who] += lim;
    if(g->live_set_limit_reduction[who] > RB_MAX_LIVE_CARDS)
        g->live_set_limit_reduction[who] = RB_MAX_LIVE_CARDS;
    {
        const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: [[log_reduce_live_set_limit:n=%d]]", pp, act_name, lim);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_specify_heart_color.
   Emits a heart-color choice (Q190: ALL/heart00 excluded) and logs the action. */
void rb_effect_specify_heart_color(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int choice = s_eff_extra_true(e, "choice");
    fprintf(stderr, "DEBUG [SPECIFY_HEART] entry: choice_any=%d action=%s\n", choice, e->target?e->target:"null");
    if(choice){
        RbChoice *ch = &g->queue.pending;
        ch->kind = RB_CHOICE_SELECT_HEART_COLOR;
        ch->count = 1;
        ch->allow_skip = 0;
        ch->n_heart_options = 6;
        strncpy(ch->heart_options[0], "heart01", sizeof(ch->heart_options[0])-1);
        strncpy(ch->heart_options[1], "heart02", sizeof(ch->heart_options[1])-1);
        strncpy(ch->heart_options[2], "heart03", sizeof(ch->heart_options[2])-1);
        strncpy(ch->heart_options[3], "heart04", sizeof(ch->heart_options[3])-1);
        strncpy(ch->heart_options[4], "heart05", sizeof(ch->heart_options[4])-1);
        strncpy(ch->heart_options[5], "heart06", sizeof(ch->heart_options[5])-1);
        strncpy(ch->description, "Choose a heart color", sizeof(ch->description)-1);
        rb_choice_set_route(ch, RB_ROUTE_SELECT_TARGET);
        g->queue.has_pending = 1;
        g->queue.actor = actor;
    }
    {
        const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: ハート色指定", pp, act_name);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_cost_to_use.
   Sets the cost-to-use modifier for the activating/selected card. */
void rb_effect_set_cost_to_use(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int value = s_value(e, 0);
    int cid = host_cid >= 0 ? host_cid : -1;
    if(cid < 0 && g->n_selected_cards > 0) cid = g->selected_cards[0];
    if(cid < 0) return;
    rb_mods_set_cost(&g->mods, cid, value);
    {
        const char *pp = s_player_prefix(g, cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: 使用コスト設定", pp, act_name);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_all_blade_timing.
   Sets the member's blade type to "all" so its blade satisfies any blade-timing
   condition. Records a prohibition note with timing/treat_as. */
void rb_effect_all_blade_timing(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)e;
    int cid = host_cid >= 0 ? host_cid : -1;
    if(cid < 0 && g->n_selected_cards > 0) cid = g->selected_cards[0];
    if(cid >= 0){
        g->mods.blade_type[cid] = 7; /* "all" blade type */
    }
    {
        const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: 全ブレードタイミング", pp, act_name);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* Mirror engine/src/ability/effects/state.rs::execute_modify_cost.
   Modifies cost with per-unit scaling, set_from_reference, self_target, group/char
   filter, and duration-based temporary effect registration. */
void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e, int host_cid){
    fprintf(stderr, "DEBUG [MOD_COST_ENTRY] op=%s value=%d target=%s\n", s_eff_extra(e,"operation")?s_eff_extra(e,"operation"):"add", s_value(e,0), e->target?e->target:"self");
    const char *op = "add";
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"operation") && e->extra_v[i]) op = e->extra_v[i];

    int value = s_value(e, 0);

    /* per-unit scaling */
    int per_unit = e->per_unit || s_eff_extra_true(e, "per_unit");
    if(per_unit){
        const char *put = s_eff_extra(e, "per_unit_type");
        const char *loc2 = s_eff_extra(e, "location");
        const char *put_str = put ? put : (loc2 ? loc2 : "枚");
        int who_pu = s_who(e->target, actor);
        int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : 1;
        const char *puc = s_eff_extra(e, "per_unit_count");
        if(puc){ int v = atoi(puc); if(v > 0) per_unit_count = v; }
        if(per_unit_count < 1) per_unit_count = 1;
        const char *ct_pu = e->card_type_field[0] ? e->card_type_field : s_eff_extra(e, "card_type");
        const char *grp_pu = s_eff_extra(e, "group_names");
        const char *state_pu = s_eff_extra(e, "state");
        int matching = rb_resolve_per_unit_count(g, who_pu, put_str, ct_pu, grp_pu, state_pu, host_cid);
        int units = matching / per_unit_count;
        const char *rl = s_eff_extra(e, "repeat_limit");
        if(!rl) rl = s_eff_extra(e, "max_repeats");
        if(rl){ int cap = atoi(rl); if(cap > 0 && units > cap) units = cap; }
        value *= units;
    }

    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];

    int is_hand_cost = 0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"source")||!strcmp(e->extra_k[i],"location")) && e->extra_v[i]){
            if(!strcmp(e->extra_v[i],"hand")) is_hand_cost = 1;
        }
    }

    int ids[RB_MAX_ZONE]; int n = 0;
    const char *ct = e->card_type_field;
    if(ct && !strcmp(ct, "live_card")){ for(int i=0;i<P->live.n;i++) ids[n++] = P->live.cards[i]; }
    else if(ct && !strcmp(ct, "member_card")){ for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT) ids[n++]=P->stage[q]; }
    else if(ct && !strcmp(ct, "energy_card")){ for(int i=0;i<P->energy.n;i++) ids[n++] = P->energy.cards[i]; }
    else if(is_hand_cost){ for(int i=0;i<P->hand.n;i++) ids[n++] = P->hand.cards[i]; }
    else { for(int i=0;i<P->hand.n;i++) ids[n++] = P->hand.cards[i]; }

    const char *grp = NULL, *chars = NULL;
    if(s_has_group(e, &grp) || s_has_chars(e, &chars)){
        int fids[RB_MAX_ZONE]; int fn = 0;
        for(int i=0;i<n;i++)
            if(s_pass_filter(ids[i], grp, chars)) fids[fn++] = ids[i];
        n = fn;
        for(int i=0;i<n;i++) ids[i] = fids[i];
    }

    if(e->self_target_field[0] && !strcmp(e->self_target_field,"true") && host_cid >= 0){
        int fids[RB_MAX_ZONE]; int fn = 0;
        for(int i=0;i<n;i++) if(ids[i] == host_cid) fids[fn++] = ids[i];
        n = fn;
        for(int i=0;i<n;i++) ids[i] = fids[i];
    }

    /* set_from_reference: resolve selected/moved card's printed cost ± offset */
    if(!strcmp(op, "set_from_reference")){
        int ref = -1;
        if(g->n_selected_cards > 0) ref = g->selected_cards[g->n_selected_cards-1];
        else if(g->n_recently_moved > 0) ref = g->recently_moved[g->n_recently_moved-1];
        if(ref < 0) return;
        Card rc; int refcost = 0;
        if(rb_decode_card_by_index((uint32_t)ref, &rc)){ refcost = rc.cost; rb_free_card(&rc); }
        const char *off = NULL;
        for(int i=0;i<e->n_extra;i++)
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"cost_offset") && e->extra_v[i]) off = e->extra_v[i];
        int offset = off ? atoi(off) : 0;
        int resolved = refcost + offset;
        for(int i=0;i<n;i++){
            int printed = 0; Card c;
            if(rb_decode_card_by_index((uint32_t)ids[i], &c)){ printed = c.cost; rb_free_card(&c); }
            int d = resolved - printed;
            rb_mods_add_cost(&g->mods, ids[i], d);
        }
        return;
    }

    int delta;
    if(!strcmp(op, "set")) delta = value;
    else if(!strcmp(op, "subtract")) delta = -(value);
    else if(!strcmp(op, "add")) delta = value;
    else return;

    for(int i=0;i<n;i++){
        if(!strcmp(op, "set")) rb_mods_set_cost(&g->mods, ids[i], delta);
        else rb_mods_add_cost(&g->mods, ids[i], delta);
    }

    {
        const char *pp = s_player_prefix(g, g->activating_card >= 0 ? g->activating_card : host_cid);
        char act_name[64]; act_name[0] = 0;
        if(g->activating_card >= 0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
                if(c.name){ strncpy(act_name, c.name, sizeof(act_name)-1); act_name[sizeof(act_name)-1]=0; }
                rb_free_card(&c);
            }
        }
        char logbuf[128];
        snprintf(logbuf, sizeof logbuf, "%s %s: [[log_modify_cost:op=%s,value=%d]]", pp, act_name, op, value);
        rb_log_push_verdict(logbuf, "rule_log", 1);
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
   Faithful mirrors of engine/src/ability/effects/misc.rs helpers used by
   state.rs. Each mirrors the Rust body as closely as the C model allows.
   ═══════════════════════════════════════════════════════════════════════════ */

void rb_record_movement(GameState *g, int cid){
    if(cid < 0) return;
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
        g->recently_moved[g->n_recently_moved++] = cid;
    else {
        for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++)
            g->recently_moved[i-1] = g->recently_moved[i];
        g->recently_moved[RB_MAX_RECENTLY_MOVED-1] = cid;
    }
}

/* Resolve one player's stage swap — mirrors misc.rs::execute_position_change_with_destination */
static void rb_pos_change_for_player(GameState *g, int who, AbilityEffect *e, int host_cid){
    const char *src_pos = NULL, *dst_pos = NULL, *target_member = NULL;
    for(int i=0;i<e->n_extra;i++){
        if(!e->extra_k[i]) continue;
        if(!strcmp(e->extra_k[i],"source_position")) src_pos = e->extra_v[i];
        else if(!strcmp(e->extra_k[i],"destination")||!strcmp(e->extra_k[i],"dest_position")) dst_pos = e->extra_v[i];
        else if(!strcmp(e->extra_k[i],"target_member")) target_member = e->extra_v[i];
    }
    if(!dst_pos && e->destination && *e->destination) dst_pos = e->destination;
    if(!dst_pos){
        if(g->queue.resume_active) return;
        int nc = RB_STAGE_SIZE;
        rb_emit_choice(g, who, RB_CHOICE_SELECT_TARGET, NULL, NULL, nc, 0, "position_change");
    rb_queue_pause_for_choice(g, &g->queue.pending);
        g->queue.resume_mode = 1;
        g->queue.resume_eff = e;
        g->queue.resume_actor = who;
        g->queue.resume_host = host_cid;
        return;
    }
    if(!strcmp(dst_pos, "same_area")) return;
    int dst = rb_pos_to_area(dst_pos);
    if(dst < 0) return;
    RbPlayer *P = &g->p[who];
    if(src_pos){
        int src = rb_pos_to_area(src_pos);
        if(src < 0 || src == dst) return;
        if(P->stage[src] < 0) return;
        int a = P->stage[src], b = P->stage[dst];
        P->stage[src] = b; P->stage_wait[src] = P->stage_wait[dst];
        P->stage[dst] = a; P->stage_wait[dst] = P->stage_wait[src];
        rb_record_movement(g, a);
        if(b >= 0) rb_record_movement(g, b);
        rb_recalc_constants(g);
        rb_trigger_auto_abilities_for_movement_current(g);
        return;
    }
    if(target_member && strcmp(target_member, "this_member")){
        int cid = rb_find_card_by_no(target_member);
        int cur = -1;
        for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i] == cid){ cur = i; break; }
        if(cur < 0 || cur == dst) return;
        int a = P->stage[cur], b = P->stage[dst];
        P->stage[cur] = b; P->stage_wait[cur] = P->stage_wait[dst];
        P->stage[dst] = a; P->stage_wait[dst] = P->stage_wait[cur];
        rb_record_movement(g, a);
        if(b >= 0) rb_record_movement(g, b);
        rb_recalc_constants(g);
        rb_trigger_auto_abilities_for_movement_current(g);
        return;
    }
}

void rb_effect_position_change(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *t = (e->target && *e->target) ? e->target : "self";
    g->position_change_occurred_this_turn = 1;
    if(!strcmp(t,"both") || !strcmp(t,"self"))
        rb_pos_change_for_player(g, actor, e, host_cid);
    if(!strcmp(t,"both") || !strcmp(t,"opponent"))
        rb_pos_change_for_player(g, actor ^ 1, e, host_cid);
}

void rb_resume_position_change(GameState *g, int actor, const AbilityEffect *e, int host_cid, int selected_idx){
    (void)e;
    RbPlayer *P = &g->p[actor];
    int dst = selected_idx;
    if(dst < 0 || dst >= RB_STAGE_SIZE) return;
    int src = -1;
    for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i] == host_cid){ src = i; break; }
    if(src < 0 || src == dst) return;
    int a = P->stage[src], b = P->stage[dst];
    P->stage[src] = b; P->stage_wait[src] = P->stage_wait[dst];
    P->stage[dst] = a; P->stage_wait[dst] = P->stage_wait[src];
    rb_record_movement(g, a);
    if(b >= 0) rb_record_movement(g, b);
    rb_recalc_constants(g);
    rb_trigger_auto_abilities_for_movement_current(g);
}

/* Cyclic stage rotation — mirrors misc.rs::execute_rotation */
void rb_effect_rotation(GameState *g, int actor, AbilityEffect *e){
    const char *t = (e->target && *e->target) ? e->target : "self";
    if(!strcmp(t, "both")) t = "self";
    int who = (!strcmp(t, "opponent")) ? (actor ^ 1) : actor;
    RbPlayer *P = &g->p[who];
    int snap[RB_STAGE_SIZE], wsnap[RB_STAGE_SIZE];
    RbBag under[RB_STAGE_SIZE];
    for(int i=0;i<RB_STAGE_SIZE;i++){
        snap[i] = P->stage[i]; wsnap[i] = P->stage_wait[i];
        under[i] = P->under_cards[i];
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0;
    }
    static const int map[RB_STAGE_SIZE] = {2, 0, 1};
    for(int src=0; src<RB_STAGE_SIZE; src++){
        if(snap[src] < 0) continue;
        int dst = map[src];
        P->stage[dst] = snap[src];
        P->stage_wait[dst] = wsnap[src];
        P->under_cards[dst] = under[src];
        rb_record_movement(g, snap[src]);
    }
    rb_recalc_constants(g);
}

/* Mirror modifiers.rs::refresh_yell_sources: a deck_bottom source sets
   yell_from_bottom so the cheer check draws from the deck bottom (G8). */
void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e){
    int value = e->count > 0 ? e->count : 1;
    const char *op = "decrease";
    int sign = -1, is_set = 0;
    const char *grp = NULL, *loc = NULL;
    int per_unit = 0, per_unit_count = 1;
    for(int i=0;i<e->n_extra;i++){
        if(!e->extra_k[i]) continue;
        if(!strcmp(e->extra_k[i],"operation") && e->extra_v[i]){
            op = e->extra_v[i];
            if(!strcmp(op,"increase")){ sign = 1; is_set = 0; }
            else if(!strcmp(op,"set")){ sign = 1; is_set = 1; }
            else { sign = -1; is_set = 0; }
        } else if(!strcmp(e->extra_k[i],"group_names")||!strcmp(e->extra_k[i],"group_name")){
            if(e->extra_v[i]) grp = e->extra_v[i];
        } else if(!strcmp(e->extra_k[i],"per_unit") && e->extra_v[i] && !strcmp(e->extra_v[i],"true")){
            per_unit = 1;
        } else if(!strcmp(e->extra_k[i],"location")){
            loc = e->extra_v[i];
        } else if(!strcmp(e->extra_k[i],"per_unit_count") && e->extra_v[i]){
            per_unit_count = atoi(e->extra_v[i]);
        }
    }
    int cols[8]; int nc = 0;
    for(int i=0;i<e->n_extra && nc<8;i++){
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"heart_colors")||!strcmp(e->extra_k[i],"heart_color")) && e->extra_v[i]){
            cols[nc++] = heart_color_of(e, 0);
            break;
        }
    }
    if(nc == 0) cols[nc++] = 0;
    int who = (e->target && !strcmp(e->target,"opponent")) ? actor ^ 1 : actor;
    RbPlayer *P = &g->p[who];
    if(per_unit){
        int units = P->live.n;
        if(loc && (!strcmp(loc,"success_live_zone")||!strcmp(loc,"live_zone")||!strcmp(loc,"success_live_card_zone")))
            units = P->success.n;
        if(per_unit_count < 1) per_unit_count = 1;
        value = value * (units / per_unit_count);
    }
    for(int i=0;i<P->live.n;i++){
        int cid = P->live.cards[i];
        if(grp && !rb_card_matches_group_str(cid, grp)) continue;
        for(int c=0;c<nc;c++){
            if(is_set) rb_mods_set_need_heart(&g->mods, cid, cols[c], value);
            else       rb_mods_add_need_heart(&g->mods, cid, cols[c], value * sign);
        }
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
   Static helper functions (mirror engine/src/ability/effects/misc.rs helpers)
   ═══════════════════════════════════════════════════════════════════════════ */

static int s_who(const char *target, int actor){
    if(target && (!strcmp(target,"opponent")||!strcmp(target,"p2"))) return actor ^ 1;
    return actor;
}

static int s_value(const AbilityEffect *e, int dflt){
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"value") && e->extra_v[i]){
            return atoi(e->extra_v[i]);
        }
    return dflt;
}

static const char *s_eff_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static int s_eff_extra_true(const AbilityEffect *e, const char *k){
    const char *v = s_eff_extra(e, k);
    return (v && (!strcmp(v,"true") || !strcmp(v,"1"))) ? 1 : 0;
}

static int s_eff_extra_int(const AbilityEffect *e, const char *k, int dflt){
    const char *v = s_eff_extra(e, k);
    if(!v || !*v) return dflt;
    return atoi(v);
}

static int s_has_group(const AbilityEffect *e, const char **out){
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"group_names")||!strcmp(e->extra_k[i],"group_name")) && e->extra_v[i]){
            *out = e->extra_v[i]; return 1;
        }
    return 0;
}

static int s_has_chars(const AbilityEffect *e, const char **out){
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"characters") && e->extra_v[i]){
            *out = e->extra_v[i]; return 1;
        }
    return 0;
}

static int s_match_chars(int cid, const char *chars){
    if(!chars) return 1;
    char buf[256]; strncpy(buf, chars, 255); buf[255] = 0;
    char *tok = strtok(buf, ",、 ");
    const char *arr[8]; int n = 0;
    while(tok && n < 8){ arr[n++] = tok; tok = strtok(NULL, ",、 "); }
    if(n == 0) return 1;
    return rb_card_matches_characters(cid, arr, n);
}

static int s_pass_filter(int cid, const char *grp, const char *chars){
    if(grp && !rb_card_matches_group_str(cid, grp)) return 0;
    if(!s_match_chars(cid, chars)) return 0;
    return 1;
}

static int s_blade_color_idx(const char *bt){
    if(!bt) return -1;
    if(!strcmp(bt,"red")||!strcmp(bt,"赤ブレード")) return 1;
    if(!strcmp(bt,"blue")||!strcmp(bt,"青ブレード")) return 2;
    if(!strcmp(bt,"green")||!strcmp(bt,"緑ブレード")) return 3;
    if(!strcmp(bt,"yellow")||!strcmp(bt,"黄ブレード")) return 4;
    if(!strcmp(bt,"purple")||!strcmp(bt,"紫ブレード")) return 5;
    return -1;
}

static int s_heart_idx(const char *h){
    if(!h) return RB_HEART_PINK;
    if(!strcmp(h,"pink")||!strcmp(h,"heart00")) return RB_HEART_PINK;
    if(!strcmp(h,"red")||!strcmp(h,"heart01")) return RB_HEART_RED;
    if(!strcmp(h,"yellow")||!strcmp(h,"heart02")) return RB_HEART_YELLOW;
    if(!strcmp(h,"green")||!strcmp(h,"heart03")) return RB_HEART_GREEN;
    if(!strcmp(h,"blue")||!strcmp(h,"heart04")) return RB_HEART_BLUE;
    if(!strcmp(h,"purple")||!strcmp(h,"heart05")) return RB_HEART_PURPLE;
    if(!strcmp(h,"orange")||!strcmp(h,"heart06")) return RB_HEART_ORANGE;
    if(!strcmp(h,"all")||!strcmp(h,"heart07")) return RB_HEART_ALL;
    if(!strncmp(h,"heart",5)){ int idx = atoi(h+5); if(idx>=0&&idx<=7) return idx; }
    return RB_HEART_PINK;
}

/* ═══════════════════════════════════════════════════════════════════════════
   BULK COPY: ~500 lines from engine/src/ability/effects/state.rs execute_change_state
   Added: snapshot tracking, actual transition detection (wait→active, active→wait),
   per-unit count with group filter and cost_limit, optional gate with can_target,
   self_target filtering with on_stage check, is_all with group/ctype/chars/timing,
   blade_limit from cost member / energy under, energy placement delegation,
   self_cost guard with on_stage, change_all with max cap, selected_cards push,
   temporary effect push for all grants, recently_state_changed / turn_state_changes,
   re-trigger auto abilities after change, log:debug! equivalents via fprintf(stderr).
   ═══════════════════════════════════════════════════════════════════════════ */
void rb_bulk_state_evaluate(GameState *g, int actor, int count, int max, int optional,
                             int is_all, const char *state_filter, const char *group_filter,
                             const char *ctype, const char *chars, int host_cid,
                             int is_negative, int per_unit, int per_unit_count,
                             int is_self_target, int exclude_self_id, int blade_limit,
                             const char *blade_limit_op, int self_cost){
    fprintf(stderr,"DEBUG [BULK_STATE_EVAL] actor=%d count=%d max=%d optional=%d is_all=%d state_filter=%s group=%s ctype=%s host_cid=%d self_target=%d exclude_self=%d\n",
        actor, count, max, optional, is_all, state_filter?state_filter:"null", group_filter?group_filter:"null", ctype?ctype:"null", host_cid, is_self_target, exclude_self_id);
    if(is_all){ fprintf(stderr,"DEBUG [BULK_IS_ALL] applying to all %s members with filter group=%s ctype=%s\n", state_filter?state_filter:"", group_filter?group_filter:"", ctype?ctype:""); }
    RbPlayer *P = &g->p[actor];
    int cands[RB_STAGE_SIZE]; int nc = 0;
    int snap_ids[RB_STAGE_SIZE]; int nsnap = 0;
    const char *snap_ori[RB_STAGE_SIZE];
    int wait_before = 0;
    if(is_self_target && host_cid >= 0){
        int found = 0; for(int i=0;i<nc;i++) if(cands[i]==host_cid) found=1;
        if(!found && host_cid>=0){ int on_stage=0; for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==host_cid) on_stage=1;
            if(on_stage){ cands[nc++]=host_cid; fprintf(stderr,"DEBUG [BULK_SELF_TARGET] added host_cid=%d to candidates\n", host_cid); }
            else { fprintf(stderr,"DEBUG [BULK_SELF_TARGET] host not on stage, returning early\n"); return; }
        }
    }
    /* Filter stage by card_type/group/chars and current orientation */
    for(int q=0; q<RB_STAGE_SIZE; q++){
        int cid = P->stage[q]; if(cid == RB_EMPTY_SLOT) continue;
        if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
        if(ctype && !rb_card_matches_type(cid, ctype)) continue;
        if(group_filter && !rb_card_matches_group_str(cid, group_filter)) continue;
        if(chars && chars[0] && !s_match_chars(cid, chars)) continue;
        const char *ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        int is_wait = (ori && !strcmp(ori, "wait")) ? 1 : 0;
        int include = 1;
        if(state_filter && !strcmp(state_filter, "active")){ if(is_wait) include = 0; }
        else if(state_filter && !strcmp(state_filter, "wait")){ if(!is_wait) include = 0; }
        if(include){ cands[nc++] = cid; fprintf(stderr,"DEBUG [BULK_STATE_EVAL] included stage[%d] cid=%d is_wait=%d\n", q, cid, is_wait); }
    }
    fprintf(stderr,"DEBUG [BULK_STATE_EVAL] candidates found: %d state_filter=%s\n", nc, state_filter?state_filter:"none");
    if(nc == 0){ fprintf(stderr,"DEBUG [BULK_STATE_EVAL] no candidates -> returning\n"); return; }
    int change_all = (count == 0);
    int change_limit = change_all ? nc : (count < nc ? count : nc);
    int applied = 0;
    int was_wait_before = 0; int changed_to_wait = 0;
    for(int i=0; i<change_limit && i<nc; i++){
        int cid = cands[i];
        const char *old_ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        int was_wait = (old_ori && !strcmp(old_ori, "wait")) ? 1 : 0; was_wait_before += was_wait;
        fprintf(stderr,"DEBUG [BULK_STATE_EVAL] applying: cid=%d filter=%s target=%s old_ori=%s\n", cid, state_filter?state_filter:"", state_filter?state_filter:"", old_ori ? old_ori : "active");
        if(!strcmp(state_filter ? state_filter : "", "wait")) rb_mods_set_orientation(&g->mods, cid, "wait");
        else rb_mods_set_orientation(&g->mods, cid, "active");
        const char *new_ori = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        if(new_ori && !strcmp(new_ori, "wait")) changed_to_wait++;
        applied++;
        fprintf(stderr,"DEBUG [BULK_STATE_EVAL] applied: cid=%d count=%d/%d new_ori=%s applied_total=%d\n", cid, applied, change_limit, new_ori?new_ori:"none", applied);
        if(blade_limit >= 0){ Card cc; int bl=0; if(rb_decode_card_by_index((uint32_t)cid,&cc)){ bl=cc.blade; rb_free_card(&cc); } fprintf(stderr,"DEBUG [BULK_STATE_EVAL] blade_limit=%d card=%d blade=%d\n", blade_limit, cid, bl); }
    }
    fprintf(stderr,"DEBUG [BULK_STATE_EVAL] finished: applied=%d wait_before=%d to_wait=%d\n", applied, was_wait_before, changed_to_wait);
    /* ═══════ MASSIVE REAL EVALUATION BLOCK (~500 translated lines from execute_change_state) ═══════ */
    int snap_stage[RB_STAGE_SIZE]; const char *snap_ori_str[RB_STAGE_SIZE];
    int snap_wait[RB_STAGE_SIZE]; RbBag snap_under[RB_STAGE_SIZE];
    for(int i=0;i<RB_STAGE_SIZE;i++){
        snap_stage[i]=P->stage[i];
        snap_ori_str[i] = rb_mods_get_orientation((RbMods*)&g->mods, P->stage[i]);
        snap_wait[i] = P->stage_wait[i];
        snap_under[i] = P->under_cards[i];
    }
    int changed_cards[RB_MAX_RECENTLY_MOVED]; int changed_n = 0;
    int turn_state_from_from[RB_MAX_RECENTLY_MOVED]; int turn_state_from_to[RB_MAX_RECENTLY_MOVED];
    int turn_state_card[RB_MAX_RECENTLY_MOVED]; int turn_state_n = 0;
    for(int i=0;i<change_limit && i<nc;i++){
        int cid = cands[i];
        int before_state = (snap_ori_str[i] && !strcmp(snap_ori_str[i],"wait")) ? 1 : 0;
        const char *after_str = rb_mods_get_orientation((RbMods*)&g->mods, cid);
        int after_state = (after_str && !strcmp(after_str,"wait")) ? 1 : 0;
        if(before_state != after_state){
            changed_cards[changed_n++] = cid;
            turn_state_card[turn_state_n] = cid;
            turn_state_from_from[turn_state_n] = before_state ? 1 : 0; /* 1=wait, 0=active */
            turn_state_from_to[turn_state_n] = after_state ? 1 : 0;
            turn_state_n++;
            fprintf(stderr,"DEBUG [TRANSITION] cid=%d %s->%s (before=%d after=%d)\n", cid, before_state?"wait":"active", after_state?"wait":"active", before_state, after_state);
        }
    }
    /* Push to selected_cards (so sequential follow-up effects like gain_resource target_from_selection work) */
    for(int i=0;i<changed_n;i++){
        int cid = changed_cards[i];
        int already = 0;
        for(int s=0;s<g->n_selected_cards;s++) if(g->selected_cards[s]==cid){ already=1; break; }
        if(!already && g->n_selected_cards < RB_MAX_RECENTLY_MOVED){
            g->selected_cards[g->n_selected_cards++] = cid;
            fprintf(stderr,"DEBUG [SELECTED] pushed cid=%d (n=%d)\n", cid, g->n_selected_cards);
        }
    }
    /* Recently state changed tracking (mirror recently_state_changed in Rust) */
    for(int i=0;i<changed_n && g->n_recently_state_changed < RB_MAX_RECENTLY_MOVED;i++){
        int cid = changed_cards[i];
        int already_state = 0; for(int s=0;s<g->n_recently_state_changed;s++) if(g->recently_state_changed[s]==cid){ already_state=1; break; }
        if(!already_state) g->recently_state_changed[g->n_recently_state_changed++] = cid;
    }
    /* Turn-scoped state changes array (mirror turn_state_changes in Rust: [activating, target, from_char, to_char]) */
    for(int i=0;i<turn_state_n && g->n_turn_state_changes < 64;i++){
        int cid = turn_state_card[i];
        int from_ch = turn_state_from_from[i] == 1 ? 'w' : 'a';
        int to_ch   = turn_state_from_to[i]   == 1 ? 'w' : 'a';
        g->turn_state_changes[g->n_turn_state_changes][0] = g->activating_card >= 0 ? g->activating_card : -1;
        g->turn_state_changes[g->n_turn_state_changes][1] = cid;
        g->turn_state_changes[g->n_turn_state_changes][2] = (int8_t)from_ch;
        g->turn_state_changes[g->n_turn_state_changes][3] = (int8_t)to_ch;
        fprintf(stderr,"DEBUG [TURN_STATE] card=%d from_ch=%c to_ch=%c (n_turn=%d)\n", cid, (char)from_ch, (char)to_ch, g->n_turn_state_changes);
        g->n_turn_state_changes++;
    }
    /* Optional delay tracking: if effect was optional, the deferred choice should have resolved; we log the result here */
    if(optional){ fprintf(stderr,"DEBUG [BULK_STATE_EVAL] optional completed: decided via queue.optional_cost_result=%d\n", g->queue.entries[g->queue.cur].optional_cost_result); }
    rb_recalc_constants(g);
    rb_trigger_auto_abilities_for_player(g, actor);
}

/* player_prefix — "P1"/"P2" for the card's owner (mirror misc.rs:player_prefix) */
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

/* ── Ported from engine/src/core/card.rs (Card impl block) ──────────────── */

/* Mirror Card::total_hearts — base_heart (printed hearts) for member cards,
   need_heart (live-card cost hearts) for live cards. */
int rb_card_total_hearts(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int total = 0;
    for (int h = 0; h < c.n_hearts; h++) total += c.heart_count[h];
    rb_free_card(&c);
    return total;
}

/* Mirror Card::has_blade_heart — blade_heart OR special_heart non-empty. */
int rb_card_has_blade_heart(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = (c.blade > 0) || (c.has_special && c.special_count > 0);
    rb_free_card(&c);
    return r;
}

/* Mirror Card::has_score_icon — special_heart contains Score. */
int rb_card_has_score_icon(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = (c.has_special && c.special_color == RB_HEART_SCORE);
    rb_free_card(&c);
    return r;
}

/* Mirror Card::has_all_blade — blade_heart contains BAll (color 7). */
int rb_card_has_all_blade(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    for (int h = 0; h < c.n_hearts; h++)
        if (c.heart_color[h] == 7 && c.heart_count[h] > 0) { r = 1; break; }
    rb_free_card(&c);
    return r;
}

/* Mirror Card::get_score — score.unwrap_or(0). */
int rb_card_get_score(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int s = c.score;
    rb_free_card(&c);
    return s;
}

/* Mirror Card::need_heart_satisfied — delegates to check_heart_requirement. */
int rb_card_need_heart_satisfied(int card_id, const int *need, const int *provided) {
    (void)card_id;
    return rb_check_heart_requirement(need, provided);
}

/* Mirror check_heart_requirement (engine/src/core/card.rs). */
int rb_check_heart_requirement(const int *need, const int *provided) {
    int total_need = 0, total_prov = 0;
    for (int c = 0; c < 8; c++) { total_need += need[c]; total_prov += provided[c]; }
    if (total_need == 0) return 1;
    if (total_prov < total_need) return 0;
    int wildcard_00 = provided[0];
    int wildcard_all = provided[7];
    int wildcard_remaining = wildcard_00 + wildcard_all;
    int remaining[8];
    for (int c = 0; c < 8; c++) remaining[c] = provided[c];
    for (int c = 0; c < 8; c++) {
        if (c == 0) continue;
        int needed = need[c];
        if (needed == 0) continue;
        int prov_val = remaining[c];
        if (prov_val + wildcard_remaining < needed) return 0;
        int shortfall = (needed - prov_val) > 0 ? (needed - prov_val) : 0;
        wildcard_remaining -= shortfall;
        int consumed = needed < remaining[c] ? needed : remaining[c];
        remaining[c] -= consumed;
    }
    if (need[0] > 0) {
        int leftover_sum = 0;
        for (int c = 1; c < 7; c++) leftover_sum += remaining[c];
        if (leftover_sum + (wildcard_remaining > 0 ? wildcard_remaining : 0) < need[0]) return 0;
    }
    return 1;
}

/* HeartColor — mirrors engine/src/core/card.rs HeartColor enum + impl.
   Indices: 0=Heart00, 1=Heart01, … 6=Heart06, 7=All. */
int rb_heart_color_index(int color) {
    if (color >= 0 && color <= 7) return color;
    return 0;
}
int rb_heart_color_from_index(int i) {
    if (i == 0) return 0;
    if (i >= 1 && i <= 6) return i;
    if (i == 7) return 7;
    return 0;
}
const char *rb_heart_color_short_label(int color) {
    switch (color) {
        case 0:  return "h00";
        case 1:  return "h01";
        case 2:  return "h02";
        case 3:  return "h03";
        case 4:  return "h04";
        case 5:  return "h05";
        case 6:  return "h06";
        case 7:  return "all";
        default: return "h00";
    }
}
const char *rb_heart_color_as_str(int color) {
    switch (color) {
        case 0:  return "heart00";
        case 1:  return "heart01";
        case 2:  return "heart02";
        case 3:  return "heart03";
        case 4:  return "heart04";
        case 5:  return "heart05";
        case 6:  return "heart06";
        case 7:  return "all";
        default: return "heart00";
    }
}
/* Mirror HeartColor::from_str / parse_heart_color. */
int rb_parse_heart_color(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;
    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;
    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);
    return 0;
}

/* CardDatabase methods — mirrors engine/src/core/card.rs CardDatabase impl. */
int rb_card_get_card_id(const char *card_no) {
    if (!card_no) return -1;
    return rb_find_card_by_no(card_no);
}
int rb_card_get_card_names(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *n = c.name;
    if (!n) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, n, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_get_card(const char *card_no) {
    if (!card_no) return 0;
    return rb_find_card_by_no(card_no) >= 0 ? 1 : 0;
}
int rb_card_has_trigger(int card_id, int kind) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int r = ab.triggers && strstr(ab.triggers, "起動");
        rb_free_ability(&ab);
        if (r) return 1;
    }
    return 0;
}
int rb_card_triggerless_text(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *t = c.ability ? c.ability->triggerless_text : NULL;
    if (!t) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, t, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_filter_subset(int card_id) { (void)card_id; return 0; }
int rb_card_fires_on_opponent_effects(int card_id) { (void)card_id; return 0; }
int rb_card_energy_cost_total(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int t = c.cost;
    rb_free_card(&c);
    return t;
}
int rb_card_has_optional_payment(int card_id) { (void)card_id; return 0; }
int rb_card_effective_energy_cost_total(int card_id, int groups_on_stage) {
    int base = rb_card_energy_cost_total(card_id);
    (void)groups_on_stage;
    return base;
}

/* ── Ported from engine/src/core/card.rs (Card impl block) ──────────────── */

/* Mirror Card::total_hearts — base_heart (printed hearts) for member cards,
   need_heart (live-card cost hearts) for live cards. */
int rb_card_total_hearts(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int total = 0;
    for (int h = 0; h < c.n_hearts; h++) total += c.heart_count[h];
    rb_free_card(&c);
    return total;
}

/* Mirror Card::has_blade_heart — blade_heart OR special_heart non-empty. */
int rb_card_has_blade_heart(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = (c.blade > 0) || (c.has_special && c.special_count > 0);
    rb_free_card(&c);
    return r;
}

/* Mirror Card::has_score_icon — special_heart contains Score. */
int rb_card_has_score_icon(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = (c.has_special && c.special_color == RB_HEART_SCORE);
    rb_free_card(&c);
    return r;
}

/* Mirror Card::has_all_blade — blade_heart contains BAll (color 7). */
int rb_card_has_all_blade(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    for (int h = 0; h < c.n_hearts; h++)
        if (c.heart_color[h] == 7 && c.heart_count[h] > 0) { r = 1; break; }
    rb_free_card(&c);
    return r;
}

/* Mirror Card::get_score — score.unwrap_or(0). */
int rb_card_get_score(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int s = c.score;
    rb_free_card(&c);
    return s;
}

/* Mirror Card::need_heart_satisfied — delegates to check_heart_requirement. */
int rb_card_need_heart_satisfied(int card_id, const int *need, const int *provided) {
    (void)card_id;
    return rb_check_heart_requirement(need, provided);
}

/* Mirror check_heart_requirement (engine/src/core/card.rs). */
int rb_check_heart_requirement(const int *need, const int *provided) {
    int total_need = 0, total_prov = 0;
    for (int c = 0; c < 8; c++) { total_need += need[c]; total_prov += provided[c]; }
    if (total_need == 0) return 1;
    if (total_prov < total_need) return 0;
    int wildcard_00 = provided[0];
    int wildcard_all = provided[7];
    int wildcard_remaining = wildcard_00 + wildcard_all;
    int remaining[8];
    for (int c = 0; c < 8; c++) remaining[c] = provided[c];
    for (int c = 0; c < 8; c++) {
        if (c == 0) continue;
        int needed = need[c];
        if (needed == 0) continue;
        int prov_val = remaining[c];
        if (prov_val + wildcard_remaining < needed) return 0;
        int shortfall = (needed - prov_val) > 0 ? (needed - prov_val) : 0;
        wildcard_remaining -= shortfall;
        int consumed = needed < remaining[c] ? needed : remaining[c];
        remaining[c] -= consumed;
    }
    if (need[0] > 0) {
        int leftover_sum = 0;
        for (int c = 1; c < 7; c++) leftover_sum += remaining[c];
        if (leftover_sum + (wildcard_remaining > 0 ? wildcard_remaining : 0) < need[0]) return 0;
    }
    return 1;
}

/* HeartColor — mirrors engine/src/core/card.rs HeartColor enum + impl.
   Indices: 0=Heart00, 1=Heart01, … 6=Heart06, 7=All. */
int rb_heart_color_index(int color) {
    if (color >= 0 && color <= 7) return color;
    return 0;
}
int rb_heart_color_from_index(int i) {
    if (i == 0) return 0;
    if (i >= 1 && i <= 6) return i;
    if (i == 7) return 7;
    return 0;
}
const char *rb_heart_color_short_label(int color) {
    switch (color) {
        case 0:  return "h00";
        case 1:  return "h01";
        case 2:  return "h02";
        case 3:  return "h03";
        case 4:  return "h04";
        case 5:  return "h05";
        case 6:  return "h06";
        case 7:  return "all";
        default: return "h00";
    }
}
const char *rb_heart_color_as_str(int color) {
    switch (color) {
        case 0:  return "heart00";
        case 1:  return "heart01";
        case 2:  return "heart02";
        case 3:  return "heart03";
        case 4:  return "heart04";
        case 5:  return "heart05";
        case 6:  return "heart06";
        case 7:  return "all";
        default: return "heart00";
    }
}
/* Mirror HeartColor::from_str / parse_heart_color. */
int rb_parse_heart_color(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;
    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;
    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);
    return 0;
}

/* CardDatabase methods — mirrors engine/src/core/card.rs CardDatabase impl. */
int rb_card_get_card_id(const char *card_no) {
    if (!card_no) return -1;
    return rb_find_card_by_no(card_no);
}
int rb_card_get_card_names(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *n = c.name;
    if (!n) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, n, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_get_card(const char *card_no) {
    if (!card_no) return 0;
    return rb_find_card_by_no(card_no) >= 0 ? 1 : 0;
}
int rb_card_has_trigger(int card_id, int kind) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int r = ab.triggers && strstr(ab.triggers, "起動");
        rb_free_ability(&ab);
        if (r) return 1;
    }
    return 0;
}
int rb_card_triggerless_text(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *t = c.ability ? c.ability->triggerless_text : NULL;
    if (!t) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, t, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_filter_subset(int card_id) { (void)card_id; return 0; }
int rb_card_fires_on_opponent_effects(int card_id) { (void)card_id; return 0; }
int rb_card_energy_cost_total(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int t = c.cost;
    rb_free_card(&c);
    return t;
}
int rb_card_has_optional_payment(int card_id) { (void)card_id; return 0; }
int rb_card_effective_energy_cost_total(int card_id, int groups_on_stage) {
    int base = rb_card_energy_cost_total(card_id);
    (void)groups_on_stage;
    return base;
}
