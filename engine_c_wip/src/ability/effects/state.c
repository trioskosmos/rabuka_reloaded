#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* core/zones.c owns it; rabuka.h does not export it -- the same situation as
   rb_player_remove_member_from_stage_with_recycling, which the parity tests
   hand-declare locally. Declared here because the Rule 4.5.5.3 stage swap is
   shared with zones.c and must have ONE implementation. */
int rb_stage_position_change_with_bags(RbPlayer *P, int from_area, int to_area);

/* ── effects/rules.c owns the restriction side tables. Both symbols are
 *    non-static there but neither is in include/rabuka.h, the same situation as
 *    rb_stage_position_change_with_bags above and as the rb_cell_* / rb_decode_*
 *    set below. Declared, not re-implemented.
 *
 *    rb_rules_is_wait_immune             — the reader for Rust
 *      `GameState.wait_immune_members` (game_state/mod.rs:149). Rust writes the
 *      (member, owner) pair list in effects/restriction.rs:191 and reads it at
 *      change_state.rs:463 and :592; this is the C twin of those two reads.
 *    rb_rules_note_changed_state_members — the writer hook for Rust
 *      `AbilityResolver::selection.changed_state_members`, assigned (not pushed)
 *      at change_state.rs:687 and read by restriction.rs:68/73/74/78/83.
 */
int rb_rules_is_wait_immune(GameState *g, int member_cid, int waiter_pl);
void rb_rules_note_changed_state_members(GameState *g, const int *cids, int n);

/* The player index of the ability that is currently resolving:
 *   game_state/mod.rs:626 `pub fn ability_master_id(&self) -> Option<String>` ->
 *   `self.ability_queue.current_entry().map(|e| e.player_id.clone())`.
 * Returns -1 when there is no entry in flight, which is what Rust's `None`
 * means; both readers below treat -1 as "no controller". */
static int s_ability_controller_pl(const GameState *g) {
    if (!g) return -1;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries || cur >= RB_QUEUE_DEPTH) return -1;
    const char *pid = g->queue.entries[cur].player_id;
    if (!pid || !*pid) return -1;
    if (!strcmp(pid, "p1") || !strcmp(pid, "player1")) return 0;
    if (!strcmp(pid, "p2") || !strcmp(pid, "player2")) return 1;
    return -1;
}

/* Forward declarations of static helpers defined later in this file */
static int s_who(const char *target, int actor);
static int s_value(const AbilityEffect *e, int dflt);
static int s_has_group(const AbilityEffect *e, const char **out);
static int s_has_chars(const AbilityEffect *e, const char **out);
static int s_filter_requested(const AbilityEffect *e);
static int s_match_chars(int cid, const char *chars);
static int s_pass_filter(int cid, const char *grp, const char *chars);
static int s_excluded_group(int cid, const AbilityEffect *e);
static int s_excluded_chars(int cid, const AbilityEffect *e);
static int s_blade_color_idx(const char *bt);
static int s_heart_idx(const char *h);
static const char *s_eff_extra(const AbilityEffect *e, const char *k);
static int s_eff_extra_true(const AbilityEffect *e, const char *k);
static int s_eff_extra_int(const AbilityEffect *e, const char *k, int dflt);
static const char *s_player_prefix(GameState *g, int card_id);
static void s_push_revert(GameState *g, int card_id, const char *duration, int kind,
                          int previous, int previous2, const char *text);
static int s_state_filter_matches(GameState *g, const AbilityEffect *e, int card_id,
                                  const char *card_type, const char *group_filter,
                                  int cost_limit, const char *cost_op,
                                  int blade_limit, const char *blade_op,
                                  int exclude_self_id);

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
    int exclude_self_id = -1;
    if(s_eff_extra_true(e, "exclude_self") && host_cid >= 0) exclude_self_id = host_cid;
    if(per_unit){
        int per_cnt = e->per_unit_count > 0 ? e->per_unit_count : 1;
        const char *puc = s_eff_extra(e, "per_unit_count");
        if(puc){ int v = atoi(puc); if(v > 0) per_cnt = v; }
        const char *per_src = s_eff_extra(e, "per_unit_source");
        if(per_src && strstr(per_src, "previous_moved")){
            count = (g->n_recently_moved / (per_cnt > 0 ? per_cnt : 1)) * (count > 0 ? count : 1);
        } else {
            const char *location = s_eff_extra(e, "location");
            int zone_ids[RB_MAX_ZONE];
            int zn = 0;
            if (location && *location)
                zn = rb_zone_cards(g, who, location, zone_ids, RB_MAX_ZONE);
            else
                for (int q = 0; q < RB_STAGE_SIZE; q++)
                    if (g->p[who].stage[q] != RB_EMPTY_SLOT) zone_ids[zn++] = g->p[who].stage[q];
            int filt_ids[RB_MAX_ZONE]; int fn = 0;
            for(int i=0;i<zn;i++)
                if(s_state_filter_matches(g, e, zone_ids[i], NULL, group_filter,
                                           cost_limit, cost_limit_op, -1, NULL,
                                           exclude_self_id))
                    filt_ids[fn++] = zone_ids[i];
            int matched = fn;
            const char *distinct = s_eff_extra(e, "distinct");
            if(distinct && *distinct && strcmp(distinct, "false") && strcmp(distinct, "0")){
                matched = rb_count_distinct_member_name_units(filt_ids, fn);
            }
            count = (matched / (per_cnt > 0 ? per_cnt : 1)) * (count > 0 ? count : 1);
        }
        group_filter = NULL;
    }

    /* change_state.rs:143-145
     *     if effect.per_unit_any().unwrap_or(false) && count == 0 { return Ok(()); }
     * A per-unit step whose zone holds no matching card scales to 0 and must
     * change NOTHING. Without this the flow fell through to the
     * deck->energy short-circuit, the member branch and finally the
     * "no candidates" exit, where it could reach
     * rb_effect_energy_state_change and — before the fix below — still fired a
     * constant recalculation, i.e. a step that printed 「1枚の」 and found nothing
     * produced engine activity.
     *
     * MEASURED: `per_unit` = 5 of the 96 effect-side `change_state` nodes in the
     * 985-ability corpus (per_unit_type 枚/人, per_unit_source
     * previous_moved_cards), so this exit is reachable. */
    if(per_unit && count == 0) return;

    /* ── blade_limit dynamic limits (Q266 / C5) ── */
    int blade_limit = -1;
    const char *blade_limit_op = s_eff_extra(e, "blade_limit_operator");
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"blade_limit") && e->extra_v[i])
            blade_limit = atoi(e->extra_v[i]);

    if(s_eff_extra_true(e, "blade_limit_from_cost_member")){
        int off = s_eff_extra_int(e, "blade_limit_offset", 0);
        int cost_blade = 0;
        /* Q266 (change_state.rs:89-97): the limit is derived from the ORIGINAL
           blade of the member PAID AS THE WAIT COST — `last_cost_wait_member()`,
           i.e. the LAST entry of `last_cost_waited_members`. This used to read
           `mods.last_cost_moved_card_ids[0]`, which is both the wrong field
           (cards moved for the cost, not members waited by it) and the wrong end
           of it (index 0 rather than the final entry), so a cost that moved a
           non-member first, or one that moved several cards, derived the limit
           from the wrong card. `.and_then(...).unwrap_or(0)` means no waited
           member reads as blade 0, so keep cost_blade at 0 in that case. */
        if(g->n_last_cost_waited_members > 0){
            int cid = g->last_cost_waited_members[g->n_last_cost_waited_members - 1];
            Card c;
            if(rb_decode_card_by_index((uint32_t)cid, &c)){ cost_blade = c.blade; rb_free_card(&c); }
        }
        int signed_lim = cost_blade - off;
        if(signed_lim < 0){ blade_limit = 0; blade_limit_op = "<"; }
        else blade_limit = signed_lim;
    } else if(s_eff_extra_true(e, "blade_limit_from_energy_under")){
        int off = s_eff_extra_int(e, "blade_limit_offset", 0);
        int under = 0;
        /* C5 (change_state.rs:102-114): the under-cards counted belong to the
           ACTIVATING card, looked up on the SELF player ("self", not the
           effect's `target`). This used to scan `host_cid` on the target
           player, which is -1 on the optional-cost path and on any effect whose
           target is the opponent, so the count silently fell to 0 and the limit
           degraded to `offset`. Fall back to host_cid only when there is no
           activating card, which is the shape the Rust `map_or(0, ...)` guard
           already tolerates. */
        int ecid = (g->activating_card >= 0) ? g->activating_card : host_cid;
        if(ecid >= 0){
            RbPlayer *Ptmp = &g->p[actor];
            for(int q=0; q<RB_STAGE_SIZE; q++)
                if(Ptmp->stage[q] == ecid) under = Ptmp->under_cards[q].n;
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

    int is_self_target_flag = (e->self_target_field[0] && !strcmp(e->self_target_field, "true"));

    /* ── Optional gate: verify at least one valid target exists ── */
    if(optional){
        int decided = -1;
        /* state.rs:160-163 — `current_entry().and_then(|e| e.optional_cost_result)`.
           With no entry in flight the answer is None, so the gate is still
           undecided and must be offered; -1 is this port's "None". */
        if(g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries &&
           g->queue.cur < RB_QUEUE_DEPTH)
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
                if(!s_state_filter_matches(g, e, cid, card_type_filter, group_filter,
                                          cost_limit, cost_limit_op, blade_limit,
                                          blade_limit_op, exclude_self_id)) continue;
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
                if(P->stage[q] == cid){ if(nc < RB_STAGE_SIZE) cands[nc++] = cid; break; }
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
                if(!s_state_filter_matches(g, e, cid, card_type_filter, group_filter,
                                          cost_limit, cost_limit_op, blade_limit,
                                          blade_limit_op, exclude_self_id)) continue;
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
        if(!s_state_filter_matches(g, e, cid, card_type_filter, group_filter,
                                   cost_limit, cost_limit_op, blade_limit,
                                   blade_limit_op, exclude_self_id)) continue;
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

    /* Q275: change_state.rs:458-467. When an effect makes the TARGET player select
     * members of their OWN stage to be waited (セラス, "action_by: opponent" —
     * 「相手は、自分のステージのアクティブなメンバーをウェイトにする」), a member
     * that is wait-immune against the effect's controller is not a legal choice
     * and must be dropped from the OFFER, so the sacrificing player has to pick a
     * waitable member. This is the inverse of Q274 (the opponent freely picking
     * their own victim, `action_by: self`), where the member stays selectable and
     * the immunity is applied at application time below.
     *
     *     candidates.retain(|(_, cid)| !gs.wait_immune_members.iter()
     *         .any(|(m, owner)| *m == *cid && controller.is_some_and(|c| c != owner)))
     *
     * rb_rules_is_wait_immune(g, cid, controller) is exactly that predicate:
     * rules.c:2233-2241 returns 1 for an entry whose member matches and whose
     * owner is NOT the waiting player. Placed AFTER the `state_change == "active"`
     * early return above, which is where Rust puts it (the retain sits at :462,
     * after the active-candidate scan returns at :392).
     *
     * MEASURED: `action_by` = "opponent" on 3 of the 96 effect-side `change_state`
     * nodes, all three also carrying `state` = "active" — so the branch that
     * reaches this filter is live. */
    if(!strcmp(state_change,"wait")){
        const char *action_by = s_eff_extra(e, "action_by");
        if(action_by && !strcmp(action_by,"opponent")){
            int controller = s_ability_controller_pl(g);
            int keep = 0;
            for(int i=0;i<nc;i++)
                if(!rb_rules_is_wait_immune(g, cands[i], controller))
                    cands[keep++] = cands[i];
            nc = keep;
        }
    }

    if(nc == 0){
        /* energy fallback */
        const char *ct2 = card_type_filter ? card_type_filter : s_eff_extra(e, "card_type");
        if(ct2 && !strcmp(ct2, "energy_card"))
            rb_effect_energy_state_change(g, actor, e);
        /* change_state.rs:488-499. Even with no target the CARD may already have
         * changed state earlier in this resolution — most often an optional
         * self-wait COST paid through handle_optional_cost_payment, which writes
         * the activating card's orientation directly. Every other exit from this
         * function recalculates, so a card whose orientation this ability wrote
         * would otherwise be left ACTIVE-facing and its own 常時 would not see the
         * wait it just paid for (Rust's own example: 統堂英玲奈 PL!-pb5-333-R,
         * which pays 「このメンバーをウェイトにしてもよい」 and then grants herself
         * no heart05).
         *
         * MEASURED: this exit is the "no matching member on the target stage"
         * branch and 69 of the 96 effect-side `change_state` nodes are member
         * steps, any of which can land here. */
        rb_recalc_constants(g);
        return;
    }

    int change_all = (count == 0);
    /* state.rs:477-486 — a single-target effect on "this member" (not the
       opponent) auto-selects the activating card instead of prompting. */
    int is_self_target = (count == 1
        && strcmp(target, "opponent") != 0
        && card_type_filter && !strcmp(card_type_filter, "member_card")
        && g->activating_card >= 0);
    if(is_self_target){
        int found_act = 0;
        for(int i=0;i<nc;i++) if(cands[i] == g->activating_card) found_act = 1;
        is_self_target = found_act ? 1 : 0;
    }
    int needs_prompt = 0;
    if(!is_self_target && g->n_selected_cards == 0){
        if(max && nc > 0) needs_prompt = 1;
        else if(!change_all && nc > count) needs_prompt = 1;
    }
    if(needs_prompt){
        int pick = count > 0 ? count : 1;
        rb_emit_choice(g, who, RB_CHOICE_SELECT_CARD, "stage", NULL,
                       pick > 0 ? pick : 1, max, "change_state");
        strncpy(g->queue.pending.card_type, card_type_filter ? card_type_filter : "member_card",
                sizeof(g->queue.pending.card_type) - 1);
        if(group_filter) strncpy(g->queue.pending.filter_group, group_filter,
                                  sizeof(g->queue.pending.filter_group) - 1);
        if(cost_limit >= 0) {
            g->queue.pending.cost_limit = cost_limit;
            if(cost_limit_op) strncpy(g->queue.pending.cost_limit_op, cost_limit_op,
                                       sizeof(g->queue.pending.cost_limit_op) - 1);
        }
        g->queue.pending.n_filtered_indices = 0;
        for(int i=0;i<nc && i<RB_STAGE_SIZE;i++)
            for(int q=0;q<RB_STAGE_SIZE;q++)
                if(P->stage[q] == cands[i]) {
                    g->queue.pending.filtered_indices[g->queue.pending.n_filtered_indices++] = q;
                    break;
                }
        strncpy(g->queue.pending.target_player_id, target,
                sizeof(g->queue.pending.target_player_id) - 1);
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        g->queue.deferred = rb_effect_deep_clone(e);
        g->queue.resume_eff = g->queue.deferred;
        g->queue.resume_actor = who;
        g->queue.resume_host = host_cid;
        rb_queue_pause_for_choice(g, &g->queue.pending);
        return;
    }

    int nchange = change_all ? nc : (count < nc ? count : nc);

    /* state.rs:541-553 — when the effect means "this member", the applied set
       is the activating card, not the first `nchange` candidates. */
    if(is_self_target){
        int keep[RB_STAGE_SIZE]; int nk = 0;
        for(int i=0;i<nc && nk<nchange;i++)
            if(cands[i] == g->activating_card) keep[nk++] = cands[i];
        if(nk > 0){
            for(int i=0;i<nk;i++) cands[i] = keep[i];
            nc = nk;
            nchange = nk;
        }
    }

    /* change_state.rs:585-599 — wait-immunity at APPLICATION time, the Q274 half
     * (「相手の効果によってはウェイトしない」, `action_by: self`): the member was a
     * legal choice and is still in the taken slice, but it must not actually take
     * the wait. Rust applies this filter unconditionally for every `state_change ==
     * "wait"` step, independently of `action_by`, because the immunity is about
     * WHO is doing the waiting rather than about who chooses. */
    if(!strcmp(state_change,"wait")){
        int controller = s_ability_controller_pl(g);
        int keep = 0;
        for(int i=0;i<nchange;i++)
            if(!rb_rules_is_wait_immune(g, cands[i], controller))
                cands[keep++] = cands[i];
        nchange = keep;
    }

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
        /* change_state.rs:653-659
         *     if state_change == "active" && !gs.cards_activated_this_turn.contains(card_id) {
         *         gs.cards_activated_this_turn.push(*card_id);
         *     }
         * The per-turn history that backs the 「このターン中に…ウェイト状態から
         * アクティブにしていたメンバー」 clause. A card already in the list is left
         * alone, so this is a SET and not a tally: activating a card twice in one
         * turn still counts once.
         *
         * The field landed on GameState in audit A8 (rabuka.h:1493-1494) but this
         * writer never did, so the history stayed permanently empty.
         *
         * MEASURED: `state_change` = "active" on 47 of the 96 effect-side
         * `change_state` nodes, and the reader is the `activated_this_turn`
         * CONDITION (3 nodes in the corpus, condition/card.rs:2417-2424). The C
         * reader for that condition is condition.c's half and is still open; the
         * writer is this file's half. */
        if(!strcmp(state_change, "active")){
            int seen = 0;
            for(int s=0;s<g->n_cards_activated_this_turn;s++)
                if(g->cards_activated_this_turn[s] == cid){ seen = 1; break; }
            if(!seen && g->n_cards_activated_this_turn < RB_MAX_CARD_IDS)
                g->cards_activated_this_turn[g->n_cards_activated_this_turn++] = cid;
        }
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

    /* change_state.rs:662-669
     *     // Conditional constants read wait/active orientations, so they must be
     *     // re-evaluated as soon as the orientation change is applied — otherwise
     *     // a card whose own effect satisfies its 「~が2人以上いる」 常時 stays stale.
     *     gs.recalculate_constants();
     *
     * This is a WITHIN-EFFECT recalc: Rust runs it before the
     * trigger_auto_abilities_for_player sweep at :765-768, so a watcher condition
     * that counts members in wait reads the state this very step just wrote. The C
     * had no recalc anywhere in rb_effect_change_state; the only one on this path
     * was engine.c:1061, which fires once after rb_drain_ability_queue() has
     * finished the WHOLE play — far too late for the sweep immediately below.
     *
     * MEASURED: every member state change takes this line. 69 of the 96
     * effect-side `change_state` nodes are member steps. */
    rb_recalc_constants(g);

    /* change_state.rs:684-687
     *     // Record the members this step changed so a following delayed
     *     // restriction step ("そのメンバーは次のターンのアクティブフェイズに
     *     // アクティブしない") can key its flags on exactly these victims.
     *     self.selection.changed_state_members =
     *         actual_targets.iter().map(|(_, cid)| *cid).collect();
     *
     * rb_rules_note_changed_state_members is that assignment's twin
     * (rules.c:2270-2277 resets then refills), and rules.c:1853-1858 is the
     * reader that gives it precedence over g->recently_moved. Before this call the
     * hook had ZERO callers tree-wide, so the delayed-cannot-activate restriction
     * always fell back to the "cards this ability moved" list and, when that was
     * empty, to the activating card. */
    rb_rules_note_changed_state_members(g, cands, nchange);

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
            /* Bound FIRST, then write. Rust's turn_state_changes is an unbounded
               Vec (mod.rs:276) pushed unconditionally at change_state.rs:742, so
               the only limit expressible in C is the array dimension (64 rows of
               4 ints). The previous body wrote all four fields first and checked
               afterwards, so at n == 64 it stored 256 bytes past the array and
               clobbered the adjacent counters. Same guard-first shape as the
               two sibling sites below (state.c:620, state.c:1839) and as the
               recently_state_changed line directly above. */
            if(g->n_turn_state_changes < 64){
                int row = g->n_turn_state_changes++;
                g->turn_state_changes[row][0] = g->activating_card >= 0 ? g->activating_card : -1;
                g->turn_state_changes[row][1] = cid;
                g->turn_state_changes[row][2] = from_str[0];
                g->turn_state_changes[row][3] = to_str[0];
            }
        }
    }

    /* Re-trigger auto abilities for both players */
    fprintf(stderr, "DEBUG [STATE_CHANGE] modifier applied, re-triggering auto abilities (state=%s)\n", state_change);
    /* change_state.rs:762-768
     *     let p1 = gs.player1.id.clone();
     *     let p2 = gs.player2.id.clone();
     *     gs.trigger_auto_abilities_for_player(&p1);
     *     gs.trigger_auto_abilities_for_player(&p2);
     *
     * BOTH players, unconditionally. The previous body was
     *     rb_trigger_auto_abilities_for_player(g, who);
     *     if (who != (actor ^ 1)) rb_trigger_auto_abilities_for_player(g, actor ^ 1);
     * which skips the second call exactly when `who == actor ^ 1` — i.e. when the
     * effect targets the OPPONENT — and so swept only the victim. On every
     * opponent-targeting wait/activate the CONTROLLER's own 常時 were never
     * re-evaluated.
     *
     * MEASURED: `target` = "opponent" on 40 of the 96 effect-side `change_state`
     * nodes (37 with source="stage", 3 without), all of them state_change="wait",
     * so this is the single most-reached branch of the defect. */
    rb_trigger_auto_abilities_for_player(g, 0);
    rb_trigger_auto_abilities_for_player(g, 1);

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
    const char *st = s_eff_extra(e, "state_change");
    if(!st) st = s_eff_extra(e, "state");
    if(!st) st = "active";
    int is_active = !strcmp(st, "active") || !strcmp(st, "アクティブ");
    int is_wait = !strcmp(st, "wait") || !strcmp(st, "ウェイト");
    if(!is_active && !is_wait) return;

    int max = s_eff_extra_true(e, "max");
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    int total = P->energy.n;
    if(total <= 0) return;
    int active = P->energy_active;
    if(active < 0) active = 0;
    if(active > total) active = total;

    const char *ct = e->card_type_field[0] ? e->card_type_field : s_eff_extra(e, "card_type");
    if(ct && *ct && strcmp(ct, "energy_card") != 0) return;
    const char *grp = NULL, *chars = NULL;
    s_has_group(e, &grp);
    s_has_chars(e, &chars);
    int cost_limit = s_eff_extra_int(e, "cost_limit", -1);
    const char *cost_op = s_eff_extra(e, "cost_limit_operator");
    int exclude_self = s_eff_extra_true(e, "exclude_self") ? g->activating_card : -1;

    int state_idx[RB_MAX_ZONE];
    int nstate = 0;
    for(int i=0; i<total; i++){
        int is_activate = i >= active;
        if(is_activate != is_active) continue;
        int cid = P->energy.cards[i];
        if(cid == exclude_self) continue;
        if(grp && !rb_card_matches_group_str(cid, grp)) continue;
        if(s_excluded_group(cid, e)) continue;
        if(!s_match_chars(cid, chars)) continue;
        if(s_excluded_chars(cid, e)) continue;
        if(cost_limit >= 0){
            Card c;
            int cost = 0;
            if(rb_decode_card_by_index((uint32_t)cid, &c)){ cost = c.cost; rb_free_card(&c); }
            int ok = 1;
            if(cost_op && !strcmp(cost_op, "<=")) ok = cost <= cost_limit;
            else if(cost_op && !strcmp(cost_op, "<")) ok = cost < cost_limit;
            else if(cost_op && !strcmp(cost_op, ">=")) ok = cost >= cost_limit;
            else if(cost_op && !strcmp(cost_op, ">")) ok = cost > cost_limit;
            else ok = cost == cost_limit;
            if(!ok) continue;
        }
        state_idx[nstate++] = i;
    }
    if(nstate == 0) return;

    int requested = e->count >= 0 ? e->count : 0;
    int eff = max ? (requested > 0 ? requested : 0)
                  : (requested == 0 ? nstate : requested);
    if(max && requested > 0 && eff > requested) eff = requested;
    if(eff < 0) eff = 0;
    if(eff > nstate) eff = nstate;
    int has_selected_energy = 0;
    for(int s=0;s<g->n_selected_cards && !has_selected_energy;s++)
        for(int i=0;i<nstate;i++)
            if(P->energy.cards[state_idx[i]] == g->selected_cards[s]) {
                has_selected_energy = 1;
                break;
            }
    if(!max && !is_active && nstate > eff && eff > 0 && !has_selected_energy) {
        rb_emit_choice(g, who, RB_CHOICE_SELECT_CARD, "energy", "energy_card",
                       eff, 0, "energy_state_change");
        g->queue.pending.n_filtered_indices = 0;
        for(int i=0;i<nstate && i<RB_MAX_ZONE;i++)
            g->queue.pending.filtered_indices[g->queue.pending.n_filtered_indices++] = state_idx[i];
        strncpy(g->queue.pending.target_player_id, e->target ? e->target : "self",
                sizeof(g->queue.pending.target_player_id)-1);
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        g->queue.deferred = rb_effect_deep_clone(e);
        g->queue.resume_eff = g->queue.deferred;
        g->queue.resume_actor = who;
        g->queue.resume_host = -1;
        rb_queue_pause_for_choice(g, &g->queue.pending);
        return;
    }

    int chosen[RB_MAX_ZONE];
    int chosen_mask[RB_MAX_ZONE];
    int selected_state[RB_MAX_ZONE];
    int nselected = 0;
    memset(chosen_mask, 0, sizeof(chosen_mask));
    for(int s=0;s<g->n_selected_cards && nselected<RB_MAX_ZONE;s++)
        for(int i=0;i<nstate;i++)
            if(P->energy.cards[state_idx[i]] == g->selected_cards[s]) {
                int seen = 0;
                for(int j=0;j<nselected;j++) if(selected_state[j] == state_idx[i]) seen = 1;
                if(!seen) selected_state[nselected++] = state_idx[i];
                break;
            }
    for(int i=0;i<eff;i++){
        int idx = nselected > 0 ? (i < nselected ? selected_state[i] : -1) : state_idx[i];
        if(idx < 0 || idx >= total) continue;
        chosen_mask[idx] = 1;
        chosen[i] = P->energy.cards[idx];
        rb_mods_set_orientation(&g->mods, chosen[i], is_active ? "active" : "wait");
    }
    if(eff <= 0) return;

    int rebuilt[RB_MAX_ZONE];
    int nrebuilt = 0;
    if(is_active){
        for(int i=0; i<eff; i++) if(chosen[i] >= 0) rebuilt[nrebuilt++] = chosen[i];
        for(int i=0; i<total; i++) if(!chosen_mask[i]) rebuilt[nrebuilt++] = P->energy.cards[i];
    } else {
        for(int i=0; i<total; i++) if(!chosen_mask[i]) rebuilt[nrebuilt++] = P->energy.cards[i];
        for(int i=0; i<eff; i++) if(chosen[i] >= 0) rebuilt[nrebuilt++] = chosen[i];
    }
    for(int i=0; i<nrebuilt; i++) P->energy.cards[i] = rebuilt[i];
    P->energy_active = is_active ? active + eff : active - eff;
    if(P->energy_active < 0) P->energy_active = 0;
    if(P->energy_active > total) P->energy_active = total;
    for(int i=0;i<eff;i++) {
        int cid = chosen[i];
        if(cid < 0) continue;
        int w = 0;
        for(int s=0;s<g->n_selected_cards;s++)
            if(g->selected_cards[s] != cid) g->selected_cards[w++] = g->selected_cards[s];
        g->n_selected_cards = w;
        g->state_change_from[cid] = (int8_t)(is_active ? 1 : 0);
        g->state_change_to[cid] = (int8_t)(is_active ? 0 : 1);
        if(g->n_recently_state_changed < RB_MAX_RECENTLY_MOVED)
            g->recently_state_changed[g->n_recently_state_changed++] = cid;
        if(g->n_turn_state_changes < 64) {
            int row = g->n_turn_state_changes++;
            g->turn_state_changes[row][0] = g->activating_card;
            g->turn_state_changes[row][1] = cid;
            g->turn_state_changes[row][2] = is_active ? 'w' : 'a';
            g->turn_state_changes[row][3] = is_active ? 'a' : 'w';
        }
    }
    rb_recalc_constants(g);
        if(!g->state_change_triggering) {
            g->state_change_triggering = 1;
            rb_trigger_auto_abilities_for_player(g, 0);
            rb_trigger_auto_abilities_for_player(g, 1);
            g->state_change_triggering = 0;
        }
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
    s_has_group(e, &grp); s_has_chars(e, &chars);
    /* state.rs:1040-1056 — the filter runs when ANY of group_names /
       exclude_group_names / characters / exclude_characters is present; an
       exclude-only shape is a filter on its own. */
    if(s_filter_requested(e)){
        int fids[RB_MAX_ZONE]; int fn = 0;
        for(int i=0;i<n;i++)
            if(s_pass_filter(ids[i], grp, chars) &&
               !s_excluded_group(ids[i], e) && !s_excluded_chars(ids[i], e)) fids[fn++] = ids[i];
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
        if(s_excluded_group(cid, e) || s_excluded_chars(cid, e)) continue;
        int previous_blade_type = rb_mods_get_blade_type(&g->mods, cid);
        rb_mods_set_blade_type(&g->mods, cid, col);
        if(duration && strcmp(duration, "permanent") != 0)
            s_push_revert(g, cid, duration, RB_REVERT_BLADE_TYPE,
                          previous_blade_type, 0, NULL);
    }
    rb_recalc_constants(g);
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_blade_count.
   Sets the blade modifier on every staged member matching the filter to `value`.
   Optionally registers a temporary effect for duration-based revert. */
void rb_effect_set_blade_count(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int value = s_value(e, -1);
    if(value < 0 && e->count >= 0) value = e->count;
    if(value < 0) value = 0;
    int who = s_who(e->target, actor);
    RbPlayer *P = &g->p[who];
    int ids[RB_STAGE_SIZE]; int n = 0;
    for(int q=0; q<RB_STAGE_SIZE; q++)
        if(P->stage[q] != RB_EMPTY_SLOT) ids[n++] = P->stage[q];
    const char *grp = NULL, *chars = NULL;
    if(s_has_group(e, &grp) || s_has_chars(e, &chars)){
        int f[RB_STAGE_SIZE]; int fn = 0;
        for(int i=0;i<n;i++)
            if(s_pass_filter(ids[i], grp, chars) &&
               !s_excluded_group(ids[i], e) && !s_excluded_chars(ids[i], e)) f[fn++] = ids[i];
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
        int previous = g->mods.blade[ids[i]].set;
        rb_mods_set_blade(&g->mods, ids[i], value);
        if(duration && strcmp(duration, "permanent") != 0)
            s_push_revert(g, ids[i], duration, RB_REVERT_BLADE_SET, previous, 0, NULL);
    }
    rb_recalc_constants(g);
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_heart_copy_from_under.
   Copies the hearts of the card just placed under this member onto the member. */
void rb_effect_set_heart_copy_from_under(GameState *g, int actor, AbilityEffect *e, int host_cid){
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
    int previous = rb_mods_get_heart_copy(&g->mods, member);
    rb_mods_set_heart_copy(&g->mods, member, src);
    s_push_revert(g, member, s_eff_extra(e, "duration"), RB_REVERT_HEART_COPY,
                  previous, 0, NULL);
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
    int previous_mult = rb_mods_get_heart_color_multiplier(&g->mods, cid);
    int previous_amt = g->mods.heart_multiplier_amt[cid];
    rb_mods_set_heart_color_multiplier(&g->mods, cid, col);
    g->mods.heart_multiplier_amt[cid] = 2;
    s_push_revert(g, cid, duration, RB_REVERT_HEART_MULT, previous_mult, previous_amt, NULL);
    fprintf(stderr, "DEBUG [SET_HEART_APPLIED] applied cid=%d col=%d ht=%s\n", cid, col, ht);
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
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"heart_type")||!strcmp(e->extra_k[i],"heart_color")) && e->extra_v[i]) {
            ht = e->extra_v[i];
            break;
        }
    if(!ht)
        for(int i=0;i<e->n_extra;i++)
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_colors") && e->extra_v[i]) {
                static char first_heart[64];
                snprintf(first_heart, sizeof(first_heart), "%s", e->extra_v[i]);
                char *comma = strpbrk(first_heart, ",、 [");
                if(comma) *comma = '\0';
                ht = first_heart;
                break;
            }
    if(ht && !strcmp(ht, "selected")){
        if(g->queue.selected_heart_color >= 0){
            static char buf[16];
            snprintf(buf, sizeof buf, "heart%02d", g->queue.selected_heart_color);
            ht = buf;
        } else ht = "heart00";
    }
    if(!ht) ht = "heart00";

    if(is_self || !needs_target){
        int who = s_who(e->target, actor);
        int cid = -1;
        if(g->n_selected_cards > 0) cid = g->selected_cards[0];
        else if(host_cid >= 0) cid = host_cid;
        else for(int q=0;q<RB_STAGE_SIZE;q++)
            if(g->p[who].stage[q] != RB_EMPTY_SLOT){ cid = g->p[who].stage[q]; break; }
        if(cid < 0) return;
        rb_effect_set_heart_type_applied(g, actor, ht, e->target,
                                         e->count >= 0 ? e->count : 1,
                                         s_eff_extra(e, "duration"), host_cid);
        return;
    }

    if(g->n_selected_cards == 0){
        const char *target_str = e->target ? e->target : "self";
        int who2 = s_who(target_str, actor);
        RbPlayer *P2 = &g->p[who2];
        int stage_ids[RB_STAGE_SIZE]; int sn = 0;
        for(int q=0;q<RB_STAGE_SIZE;q++)
            if(P2->stage[q] != RB_EMPTY_SLOT) stage_ids[sn++] = P2->stage[q];
        int cand[RB_STAGE_SIZE]; int nc2 = 0;
        int cost_limit = s_eff_extra_int(e, "cost_limit", -1);
        const char *cost_op = s_eff_extra(e, "cost_limit_operator");
        for(int i=0;i<sn;i++)
            if(s_state_filter_matches(g, e, stage_ids[i], e->card_type_field, grp,
                                       cost_limit, cost_op, -1, NULL, -1))
                cand[nc2++] = stage_ids[i];
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
            rb_effect_set_heart_type_applied(g, actor, ht, e->target,
                                             e->count >= 0 ? e->count : 1,
                                             s_eff_extra(e, "duration"), host_cid);
        } else {
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", NULL, tc, 0, "heart_type");
            if(grp) strncpy(g->queue.pending.filter_group, grp, sizeof(g->queue.pending.filter_group)-1);
            if(cost_limit >= 0) {
                g->queue.pending.cost_limit = cost_limit;
                if(cost_op) strncpy(g->queue.pending.cost_limit_op, cost_op,
                                    sizeof(g->queue.pending.cost_limit_op)-1);
            }
            g->queue.pending.n_filtered_indices = 0;
            for(int i=0;i<nc2;i++)
                for(int q=0;q<RB_STAGE_SIZE;q++)
                    if(P2->stage[q] == cand[i]) {
                        g->queue.pending.filtered_indices[g->queue.pending.n_filtered_indices++] = q;
                        break;
                    }
            g->queue.deferred = rb_effect_deep_clone(e);
            g->queue.resume_eff = g->queue.deferred;
            g->queue.resume_host = host_cid;
            g->queue.resume_mode = 0;
            rb_queue_pause_for_choice(g, &g->queue.pending);
            return;
        }
        return;
    }
    rb_effect_set_heart_type_applied(g, actor, ht, e->target,
                                     e->count >= 0 ? e->count : 1,
                                     s_eff_extra(e, "duration"), host_cid);
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
    char note[64];
    note[0] = '\0';
    if(!strcmp(target,"self") || !strcmp(target,"opponent")){
        snprintf(note, sizeof note, "activation_cost_%s_%d", op, value);
        if(g->n_prohibition_effects < 64){
            snprintf(g->prohibition_effects[g->n_prohibition_effects], sizeof(g->prohibition_effects[0]), "%s", note);
            g->n_prohibition_effects++;
        }
        s_push_revert(g, g->activating_card >= 0 ? g->activating_card : host_cid,
                      s_eff_extra(e, "duration"), RB_REVERT_TEXT, 0, 0, note);
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
    (void)actor; (void)host_cid;
    const char *id = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"identities")||!strcmp(e->extra_k[i],"identity")) && e->extra_v[i]) {
            id = e->extra_v[i];
            break;
        }
    if(id && *id && g->n_prohibition_effects < 64)
        snprintf(g->prohibition_effects[g->n_prohibition_effects++],
                 sizeof(g->prohibition_effects[0]), "card_identity:%s", id);
    int cid = g->activating_card >= 0 ? g->activating_card : host_cid;
    const char *pp = s_player_prefix(g, cid);
    char act_name[64]; act_name[0] = 0;
    if(g->activating_card >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
            if(c.name) snprintf(act_name, sizeof act_name, "%s", c.name);
            rb_free_card(&c);
        }
    }
    char logbuf[128];
    snprintf(logbuf, sizeof logbuf, "%s %s: カード同一性変更", pp, act_name);
    rb_log_push_verdict(logbuf, "rule_log", 1);
}

/* Mirror engine/src/ability/effects/state.rs::execute_set_card_identity_all_regions.
   Identity rewrite that also records a per-card prohibition note. */
void rb_effect_set_card_identity_all_regions(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)actor;
    int cid = g->activating_card >= 0 ? g->activating_card : host_cid;
    const char *id = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && (!strcmp(e->extra_k[i],"identities")||!strcmp(e->extra_k[i],"identity")) && e->extra_v[i]) {
            id = e->extra_v[i];
            break;
        }
    if(id && cid >= 0) {
        char buf[256]; snprintf(buf, sizeof buf, "%s", id);
        /* identities_any() is a Vec<String> in Rust — entries are separated by
           the list delimiters only, never by a space (unit names contain
           spaces: "Aoyama Ichika"). Same token set as score.c:241. */
        char *tok = strtok(buf, ",、[]\"'");
        while(tok && g->n_prohibition_effects < 64) {
            snprintf(g->prohibition_effects[g->n_prohibition_effects++],
                     sizeof(g->prohibition_effects[0]), "card_identity:%d:%s", cid, tok);
            tok = strtok(NULL, ",、[]\"'");
        }
    }
    const char *pp = s_player_prefix(g, cid);
    char act_name[64]; act_name[0] = 0;
    if(g->activating_card >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)g->activating_card, &c)){
            if(c.name) snprintf(act_name, sizeof act_name, "%s", c.name);
            rb_free_card(&c);
        }
    }
    char logbuf[128];
    snprintf(logbuf, sizeof logbuf, "%s %s: 全領域カード同一性変更", pp, act_name);
    rb_log_push_verdict(logbuf, "rule_log", 1);
}

/* Mirror engine/src/ability/effects/state.rs::execute_reduce_live_card_set_limit.
   Reduces the player's live card set limit by count. */
void rb_effect_reduce_live_card_set_limit(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    int lim = e->count > 0 ? e->count : 1;
    g->live_set_limit_reduction[actor] += lim;
    if(g->live_set_limit_reduction[actor] > RB_MAX_LIVE_CARDS)
        g->live_set_limit_reduction[actor] = RB_MAX_LIVE_CARDS;
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
    int cid = g->activating_card >= 0 ? g->activating_card : host_cid;
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

/* ── all_blade_timing record store ──────────────────────────────────────────
   state.rs:1634-1639 pushes ONE note per effect into
   `gs.prohibition_effects: Vec<String>`. This port models that field as
   `char prohibition_effects[64][48]` (rabuka.h:1422), and the note with the
   DEFAULT arguments is 59 bytes ("all_blade_timing:<card>:" +
   "check_required_hearts" + ":" + "any_heart_color"), so it cannot be stored
   there whole. The authoritative record therefore lives in this file-owned
   table, keyed by the owning GameState so two concurrent games never read
   each other's notes; prohibition_effects still gets a bounded key so the
   existing note scanners (core/tracking.c, game_state_abilities.c) keep
   seeing that the effect fired. */
#define S_ABT_CAP 64
typedef struct {
    const GameState *owner;
    int card_id;
    char timing[48];
    char treat_as[48];
} RbAllBladeTimingNote;
static RbAllBladeTimingNote s_all_blade_timing[S_ABT_CAP];
static int s_n_all_blade_timing;

int rb_all_blade_timing_record(const GameState *g, int card_id,
                               char *timing, int timing_sz,
                               char *treat_as, int treat_as_sz) {
    for (int i = s_n_all_blade_timing - 1; i >= 0; i--) {
        if (s_all_blade_timing[i].owner != g) continue;
        if (card_id >= 0 && s_all_blade_timing[i].card_id != card_id) continue;
        if (timing && timing_sz > 0)
            snprintf(timing, (size_t)timing_sz, "%s", s_all_blade_timing[i].timing);
        if (treat_as && treat_as_sz > 0)
            snprintf(treat_as, (size_t)treat_as_sz, "%s", s_all_blade_timing[i].treat_as);
        return 1;
    }
    return 0;
}

int rb_all_blade_timing_clear(const GameState *g) {
    int w = 0;
    for (int i = 0; i < s_n_all_blade_timing; i++) {
        if (g && s_all_blade_timing[i].owner != g) {
            if (w != i) s_all_blade_timing[w] = s_all_blade_timing[i];
            w++;
        }
    }
    s_n_all_blade_timing = w;
    return 0;
}

/* Mirror engine/src/ability/effects/state.rs::execute_all_blade_timing.
   Records the blade-timing note ("all_blade_timing:<card>:<timing>:<treat_as>",
   state.rs:1629-1639) so the member's blade satisfies any blade-timing check. */
void rb_effect_all_blade_timing(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)actor;
    int cid = host_cid >= 0 ? host_cid : g->activating_card;
    const char *timing = s_eff_extra(e, "timing");
    if(!timing) timing = "check_required_hearts";
    const char *treat_as = s_eff_extra(e, "treat_as");
    if(!treat_as) treat_as = "any_heart_color";
    if(cid >= 0){
        if(s_n_all_blade_timing >= S_ABT_CAP) rb_all_blade_timing_clear(g);
        if(s_n_all_blade_timing < S_ABT_CAP){
            RbAllBladeTimingNote *n = &s_all_blade_timing[s_n_all_blade_timing++];
            n->owner = g;
            n->card_id = cid;
            snprintf(n->timing, sizeof n->timing, "%s", timing);
            snprintf(n->treat_as, sizeof n->treat_as, "%s", treat_as);
        }
        if(g->n_prohibition_effects < 64)
            snprintf(g->prohibition_effects[g->n_prohibition_effects++],
                     sizeof(g->prohibition_effects[0]), "all_blade_timing:%d:%s",
                     cid, timing);
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
    const char *duration = s_eff_extra(e, "duration");

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
    /* state.rs:1699-1720 — is_hand_cost is tested FIRST, so a hand-scoped
       modifier wins over card_type ("手札にあるこのカードのコストは…"). */
    if(is_hand_cost){ for(int i=0;i<P->hand.n;i++) ids[n++] = P->hand.cards[i]; }
    else if(ct && !strcmp(ct, "live_card")){ for(int i=0;i<P->live.n;i++) ids[n++] = P->live.cards[i]; }
    else if(ct && !strcmp(ct, "member_card")){ for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT) ids[n++]=P->stage[q]; }
    else if(ct && !strcmp(ct, "energy_card")){ for(int i=0;i<P->energy.n;i++) ids[n++] = P->energy.cards[i]; }
    else { for(int i=0;i<P->hand.n;i++) ids[n++] = P->hand.cards[i]; }

    const char *grp = NULL, *chars = NULL;
    s_has_group(e, &grp); s_has_chars(e, &chars);
    /* state.rs:1722-1726 — exclude_group_names / exclude_characters alone
       already select filter_subset(). */
    if(s_filter_requested(e)){
        int fids[RB_MAX_ZONE]; int fn = 0;
        for(int i=0;i<n;i++)
            if(s_pass_filter(ids[i], grp, chars) &&
               !s_excluded_group(ids[i], e) && !s_excluded_chars(ids[i], e)) fids[fn++] = ids[i];
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
            if(duration && strcmp(duration, "permanent") != 0) {
                int before = g->n_temp_effects;
                s_push_revert(g, ids[i], duration, RB_REVERT_NONE, 0, 0, NULL);
                if(g->n_temp_effects > before)
                    g->temp_effects[g->n_temp_effects - 1].cost = -d;
            }
        }
        return;
    }

    int delta;
    if(!strcmp(op, "set")) delta = value;
    else if(!strcmp(op, "subtract")) delta = -(value);
    else if(!strcmp(op, "add")) delta = value;
    else return;

    for(int i=0;i<n;i++){
        if(!strcmp(op, "set")) {
            int previous = g->mods.cost[ids[i]].set;
            rb_mods_set_cost(&g->mods, ids[i], delta);
            if(duration && strcmp(duration, "permanent") != 0)
                s_push_revert(g, ids[i], duration, RB_REVERT_COST_SET, previous, 0, NULL);
        } else {
            rb_mods_add_cost(&g->mods, ids[i], delta);
            if(duration && strcmp(duration, "permanent") != 0) {
                int before = g->n_temp_effects;
                s_push_revert(g, ids[i], duration, RB_REVERT_NONE, 0, 0, NULL);
                if(g->n_temp_effects > before)
                    g->temp_effects[g->n_temp_effects - 1].cost = -delta;
            }
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
    if(!strcmp(dst_pos, "front") && host_cid >= 0){
        int owner = -1;
        int occupied[RB_STAGE_SIZE];
        int n_occupied = 0;
        for(int side=0;side<2;side++){
            for(int i=0;i<RB_STAGE_SIZE;i++){
                if(g->p[side].stage[i]==host_cid) owner=side;
                if(side==who && g->p[side].stage[i]!=RB_EMPTY_SLOT)
                    occupied[n_occupied++]=i;
            }
        }
        if(owner>=0 && who!=owner && n_occupied>0){
            rb_emit_choice(g,owner,RB_CHOICE_SELECT_TARGET,NULL,NULL,n_occupied,0,
                           "position_change:opponent:front");
            rb_choice_set_description(&g->queue.pending,"Choose which opponent member to move");
            rb_choice_set_route(&g->queue.pending,RB_ROUTE_SELECT_TARGET);
            g->queue.resume_host=host_cid;
            g->queue.resume_actor=who;
            return;
        }
    }
    int dst = rb_pos_to_area(dst_pos);
    if(dst < 0) return;
    RbPlayer *P = &g->p[who];
    if(src_pos){
        int src = rb_pos_to_area(src_pos);
        if(src < 0 || src == dst) return;
        if(P->stage[src] < 0) return;
        int a = P->stage[src], b = P->stage[dst];
        /* Rule 4.5.5.3 / Q139: under-cards belong to the MEMBER, not the slot,
           so they travel with it. The crossed take now lives in ONE place --
           rb_stage_position_change_with_bags (core/zones.c), the C twin of Rust
           Stage::position_change, whose unconditional pre-swap crossed take is
           engine/src/core/zones.rs:396-400. */
        if(rb_stage_position_change_with_bags(P, src, dst) < 0) return;
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
        if(P->stage[cur] < 0) return;
        int a = P->stage[cur], b = P->stage[dst];
        /* Rule 4.5.5.3 / Q139: under-cards follow their member. */
        if(rb_stage_position_change_with_bags(P, cur, dst) < 0) return;
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
    /* host_cid < 0 must not match an EMPTY slot: RB_EMPTY_SLOT is -1, so the
       unguarded comparison used to select the first empty area as the SOURCE
       and then swap two empty areas -- their under-card bags and wait flags
       included -- for a move that had no member to move. */
    if(host_cid >= 0)
        for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i] == host_cid){ src = i; break; }
    if(src < 0 || src == dst) return;
    int a = P->stage[src], b = P->stage[dst];
    /* Rule 4.5.5.3 / Q139: under-cards travel WITH the member on a position
       change. Mirrors Rust Stage::position_change, engine/src/core/zones.rs:
       396-400, which `mem::take`s both under-card stacks and writes them back
       crossed over BEFORE the stage swap -- so the stack is carried even when
       the destination area is empty. That crossed take (plus the per-area
       stage_wait swap that stands in for Rust's per-CARD orientation map) now
       has ONE implementation, rb_stage_position_change_with_bags in
       core/zones.c; this route used to carry its own third copy. */
    if(rb_stage_position_change_with_bags(P, src, dst) < 0) return;
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

/* DELETED 2026-10-02: rb_effect_modify_hearts.
 *
 * Zero callers (verified by name across src/ and tests/) and NOT a Rust entry
 * point: engine/src/ability/enums.rs:450 is the only spelling of this family
 * and it is `ModifyRequiredHearts => "modify_required_hearts"`. There is no
 * `modify_hearts` action in Rust at all, so nothing can ever dispatch here.
 *
 * What it actually was: an older, pre-fix duplicate of execute_modify_required_hearts
 * (engine/src/ability/effects/required_hearts.rs:28-223) that the live
 * constant arm in src/turn/triggers.c:930-957 replaced -- that arm's own
 * comment records the four defects this body still had:
 *   - SINGULAR colour read. :1608-1612 took ONE entry out of the effect and
 *     `break`ed, so a 「heart01とheart02」 style list applied to the first colour
 *     only, where Rust's `heart_colors_any()` (required_hearts.rs:36) is a LIST
 *     and the per-colour loop at :182-221 gives every listed colour the value.
 *     Worse, heart_color_of (src/engine.c:182) resolves the `heart_color` extra
 *     and falls back to `target`, and then to the caller's default.
 *   - `operation` half-honoured: only "increase" and "set" were distinguished,
 *     and everything else silently became a decrease -- required_hearts.rs:197-211
 *     maps exactly three operations and returns Err on a fourth.
 *   - amount from `count` only (:1585), where required_hearts.rs:35 uses
 *     `value_or_count(0)` with `value` winning over `count`; on
 *     PL!-bp6-022-L `count` is the ORIGINAL-SCORE THRESHOLD, not the amount.
 *   - no self_target / group_name / original-score filter on the candidate set
 *     (required_hearts.rs:147-167).
 *
 * The live twins, both verified correct against required_hearts.rs:28-223, are
 * src/ability/effects/score.c:769 rb_execute_modify_required_hearts (the
 * effect-driven route, plural colours at :904, `value` over `count` at :776-777,
 * full candidate set at :864-901) and the constant arm at
 * src/turn/triggers.c:930-957. include/rabuka.h:2100 still declares this
 * symbol; nothing calls it, so the tree links without the definition. That
 * declaration is the header owner's to drop. */

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

/* Mirror the gate that guards the `filter_subset()` call in state.rs
   (set_cost 1040-1044, set_blade_type 1118-1122, modify_cost 1722-1726):
   ANY of group_names / exclude_group_names / characters / exclude_characters
   turns the filter on. Checking only the positive keys loses every
   exclude-only shape. */
static int s_filter_requested(const AbilityEffect *e){
    static const char *keys[4] = { "group_names", "exclude_group_names",
                                   "characters", "exclude_characters" };
    for(int k=0;k<4;k++){
        const char *v = s_eff_extra(e, keys[k]);
        if(v && *v) return 1;
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

static int s_matches_excluded_tokens(int cid, const AbilityEffect *e, const char *key, int characters) {
    const char *value = s_eff_extra(e, key);
    if (!value || !*value) return 0;
    char buf[256];
    strncpy(buf, value, sizeof(buf) - 1);
    buf[sizeof(buf) - 1] = 0;
    char *tok = strtok(buf, ",、 ");
    while (tok) {
        if (characters) {
            const char *arr[1] = { tok };
            if (rb_card_matches_characters(cid, arr, 1)) return 1;
        } else if (rb_card_matches_group_str(cid, tok)) {
            return 1;
        }
        tok = strtok(NULL, ",、 ");
    }
    return 0;
}

static int s_excluded_group(int cid, const AbilityEffect *e) {
    return s_matches_excluded_tokens(cid, e, "exclude_group_names", 0);
}

static int s_excluded_chars(int cid, const AbilityEffect *e) {
    return s_matches_excluded_tokens(cid, e, "exclude_characters", 1);
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

/* DELETED 2026-10-03: rb_bulk_state_evaluate.
 *
 * A SECOND, ~300-line copy of execute_change_state
 * (engine/src/ability/effects/change_state.rs:119-781), living in the same
 * translation unit as the live copy rb_effect_change_state (state.c:70).
 *
 * Proof it is dead, by name, over every .c / .h / .py / .md / .sh and the
 * Makefile (the only other hit is a DEPENDENCY_AUDIT.md table row): zero call
 * sites and zero declarations. It was introduced as a "BULK COPY" of the Rust
 * function, after which rb_effect_change_state was rebuilt properly; the
 * replacement was made live and the copy was left.
 *
 * Why deleting it is the fix rather than tidying: two C implementations of one
 * Rust function in one file is the exact hazard this audit was opened for -- the
 * copy was stale, not merely redundant. Inside it, the candidate scan called
 * rb_card_matches_type + rb_card_matches_group_str + s_match_chars directly and
 * NEVER rb_matching_ids, so it dropped the cost_limit / blade_limit filter; its
 * state decision keyed off the `state_filter` extra instead of `state_change`,
 * where Rust has a three-way branch (change_state.rs:390-398); it swept auto
 * abilities for `actor` only, where Rust sweeps both seats
 * (change_state.rs:765-768); and it carried none of the seven fixes just landed
 * in rb_effect_change_state. Anything that ever called it would silently
 * disagree with Rust on 69 member `change_state` steps.
 *
 * Rust has no second entry point here: `ActionType::ChangeState` is a single
 * variant (enums.rs) and dispatch.rs routes it to one function. The live twin
 * is rb_effect_change_state; the choice resume re-drives that same function
 * through its "Honour prior selected_cards relay" branch, which is what
 * choice.c:1897-1915 documents.
 *
 * Same shape as the earlier rb_effect_modify_hearts deletion above: a C
 * duplicate with no callers and no Rust counterpart. */

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

static void s_push_revert(GameState *g, int card_id, const char *duration, int kind,
                          int previous, int previous2, const char *text) {
    if (!g || card_id < 0 || !duration || !strcmp(duration, "permanent") ||
        g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) return;
    RbTempEffect *te = &g->temp_effects[g->n_temp_effects++];
    memset(te, 0, sizeof(*te));
    te->card_id = card_id;
    te->dur = rb_parse_duration(duration);
    te->gained_card_id = -1;
    te->gained_index = -1;
    te->revert_kind = kind;
    te->previous_value = previous;
    te->previous_value2 = previous2;
    if (text) snprintf(te->revert_text, sizeof(te->revert_text), "%s", text);
}

static int s_state_filter_matches(GameState *g, const AbilityEffect *e, int card_id,
                                  const char *card_type, const char *group_filter,
                                  int cost_limit, const char *cost_op,
                                  int blade_limit, const char *blade_op,
                                  int exclude_self_id) {
    (void)g;
    if (card_id < 0 || card_id == exclude_self_id) return 0;
    RbCardFilter filter;
    rb_effect_filter_subset(e, &filter);
    /* The caller decides card_type / group / cost_limit / blade_limit for this
       scan (state.rs:318-325 rebuilds CardFilter::default() and fills those
       four fields explicitly). Drop everything rb_effect_filter_subset pulled
       in so an unset group_filter cannot keep the effect's own group_names.
       local_filter_from_public (util.c:965-966) keys card_type and group off
       the STRING, not off has_group, so the text has to be cleared too. */
    filter.card_type[0] = 0;
    filter.group[0] = 0;
    filter.has_group = 0;
    filter.has_cost_limit = 0;
    filter.has_original_blade = 0;
    if (card_type && *card_type) snprintf(filter.card_type, sizeof(filter.card_type), "%s", card_type);
    if (group_filter && *group_filter) {
        snprintf(filter.group, sizeof(filter.group), "%s", group_filter);
        filter.has_group = 1;
    }
    if (cost_limit >= 0) {
        filter.cost_limit = cost_limit;
        filter.has_cost_limit = 1;
        if (cost_op) snprintf(filter.cost_op, sizeof(filter.cost_op), "%s", cost_op);
    }
    if (blade_limit >= 0) {
        filter.original_blade_limit = blade_limit;
        filter.has_original_blade = 1;
        if (blade_op) snprintf(filter.original_blade_op, sizeof(filter.original_blade_op), "%s", blade_op);
    }
    filter.has_filter = filter.card_type[0] || filter.has_group || filter.has_cost_limit ||
                        filter.has_characters || filter.has_exclude_characters ||
                        filter.n_heart_colors > 0 || filter.n_name_fragments > 0 ||
                        filter.n_cost_values > 0 || filter.has_original_blade ||
                        filter.has_cost_total || filter.has_blade_limit || filter.has_exclude_self;
    int ids[1] = {card_id};
    if (!rb_matching_ids(&filter, ids, 1, ids, 1)) return 0;
    if (s_excluded_group(card_id, e) || s_excluded_chars(card_id, e)) return 0;
    return 1;
}

/* ── Ported from engine/src/core/card.rs (Card impl block) ──────────────── */

/* Mirror Card::total_hearts (card.rs:4194-4202) — sum base_heart, and fall
   back to need_heart ONLY when the card has no base_heart section. Blade and
   need hearts are never folded in (Q149/Q172: 「ハートの総数」 counts the
   member's basic hearts plus ability-granted ones, not blade/need hearts). */
int rb_card_total_hearts(const Card *c) {
    if (!c) return 0;
    int total = 0;
    if (c->num_base > 0) {
        for (int h = 0; h < c->num_base && h < c->n_hearts; h++) total += c->heart_count[h];
        return total;
    }
    if (c->n_hearts > 0) {
        int start = c->num_base + c->num_blade;
        int end = start + c->num_need;
        for (int h = start; h < end && h < c->n_hearts; h++) total += c->heart_count[h];
    }
    return total;
}

/* NOTE: rb_card_has_blade_heart / has_score_icon / has_all_blade live in
   src/ability/util.c (single owners; deleted duplicates here 2026-09-14).
   util.c's versions match card.rs: has_blade_heart = blade_heart.is_some()
   (num_blade>0) or special non-empty; has_all_blade scans the blade section
   for BAll. */

/* Mirror Card::get_score — score.unwrap_or(0). */
int rb_card_get_score(const Card *c) {
    if (!c) return 0;
    return c->score;
}

/* Mirror Card::need_heart_satisfied — delegates to check_heart_requirement. */
int rb_card_need_heart_satisfied(const Card *c, const int *need, const int *provided) {
    (void)c;
    return rb_check_heart_requirement(need, provided);
}

/* Mirror check_heart_requirement (engine/src/core/card.rs). */
int rb_check_heart_requirement(const int *need, const int *provided) {
    if (!need || !provided) return 0;
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
/* NOTE: rb_parse_heart_color lives in src/ability/util.c (single owner;
   deleted duplicate here 2026-09-14). util.c's version covers the full
   card.rs mapping incl. draw/score/b_all. */
/* NOTE: CardDatabase-method ports (rb_card_get_card_id/names/get,
   has_trigger, triggerless_text, filter_subset, fires_on_opponent_effects,
   energy_cost_total, has_optional_payment, effective_energy_cost_total)
   live in src/core/card.c (single owner; deleted duplicates here
   2026-09-14). */

