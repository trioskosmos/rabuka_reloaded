/* effects/misc.c — faithful, complete C translation of
   engine/src/ability/effects/misc.rs.
   Covers: gain_resource, pay_energy, discard_until_count, restriction,
   re_yell, perform_yell, shuffle, reveal, reveal_until_chosen_card,
   play_baton_touch, place_energy_under_member, position_change, rotation,
   choice, gain_surplus_heart, custom, activation_restriction,
   choose_required_hearts, choose_target_player, pay_cost_all_discard,
   handle_both_targets, compute_valid_position_destinations,
   finalize_formation_change, position_change_with_destination. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── forward decls ── */
void rb_effect_gain_surplus_heart(GameState *g, int actor, const AbilityEffect *e);
void rb_effect_reveal_until_target(GameState *g, int actor, AbilityEffect *e);
void rb_gain_ability(GameState *g, int actor, AbilityEffect *e);
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e);
void rb_recalc_constants(GameState *g);
void rb_recalc_constants(GameState *g);
int  rb_pos_to_area(const char *pos);
int  rb_stage_position_index(const char *pos);
void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r);
void rb_log_push_verdict(const char *text, const char *kind, int passed);
int  rb_ability_debug_enabled(void);

/* ── shared helpers (private fns of misc.rs) ── */

static const char *eff_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}

static int extra_true(const AbilityEffect *e, const char *k){
    const char *v = eff_extra(e,k);
    return v && (!strcmp(v,"true") || !strcmp(v,"1"));
}

static int extra_int(const AbilityEffect *e, const char *k, int dflt){
    const char *v = eff_extra(e,k);
    if(!v || !*v) return dflt;
    return atoi(v);
}

#define RB_RES_OTHER 0
#define RB_RES_BLADE 1
#define RB_RES_HEART  2

static int resource_kind(const char *s){
    if(!s) return RB_RES_OTHER;
    if(!strcmp(s,"blade") || !strcmp(s,"ブレード")) return RB_RES_BLADE;
    if(!strcmp(s,"heart") || !strcmp(s,"ハート"))  return RB_RES_HEART;
    return RB_RES_OTHER;
}

/* Threaded by rb_execute_misc_effect_ex; -1 when unknown. */
static int s_activating_card = -1;

/* "both" resolves to self at this level (deferred to rb_misc_handle_both_targets). */
static int misc_target_player(int actor, const AbilityEffect *e){
    if(e->target && !strcmp(e->target,"opponent")) return actor ^ 1;
    return actor;
}

/* "P1"/"P2" for the activating card's owner (stage / live / hand), else active player. */
static const char *player_prefix(const GameState *g, int card_id){
    if(card_id >= 0){
        for(int pl=0; pl<2; pl++){
            const RbPlayer *P = &g->p[pl];
            for(int q=0;q<RB_STAGE_SIZE;q++)    if(P->stage[q]==card_id)         return pl==0?"P1":"P2";
            for(int i=0;i<P->live.n;i++)         if(P->live.cards[i]==card_id)    return pl==0?"P1":"P2";
            for(int i=0;i<P->hand.n;i++)         if(P->hand.cards[i]==card_id)    return pl==0?"P1":"P2";
        }
    }
    return g->active==0 ? "P1" : "P2";
}

/* "P1 <card name>: <label>" into the rule log. */
static void rule_log_activated(const GameState *g, int card_id, const char *label){
    char name[96]; name[0]=0;
    if(card_id >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)card_id, &c)){
            if(c.name){ strncpy(name, c.name, sizeof name-1); name[sizeof name-1]=0; }
            rb_free_card(&c);
        }
    }
    char line[256];
    snprintf(line, sizeof line, "%s %s: %s",
             player_prefix(g, card_id), name, label?label:"");
    rb_log_push_verdict(line, "rule_log", 1);
}

/* gs.prohibition_effects.push("<a>:<b>"). */
static void push_prohibition(GameState *g, const char *a, const char *b){
    if(g->n_prohibition >= 64) return;
    char *out = g->prohibition[g->n_prohibition];
    size_t cap = sizeof(g->prohibition[0]) - 1;
    size_t i = 0;
    for(const char *p=a?a:""; *p && i<cap; ) out[i++] = *p++;
    if(i < cap) out[i++]=':';
    for(const char *p=b?b:""; *p && i<cap; ) out[i++] = *p++;
    out[i]=0;
    g->n_prohibition++;
}

/* Recently-moved ring. */
static void record_movement(GameState *g, int cid){
    if(cid < 0) return;
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
        g->recently_moved[g->n_recently_moved++]=cid;
    else {
        for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++)
            g->recently_moved[i-1]=g->recently_moved[i];
        g->recently_moved[RB_MAX_RECENTLY_MOVED-1]=cid;
    }
}

/* Swap two stage positions (carries under_cards + stage_wait). */
static int stage_swap(GameState *g, int who, int from, int to){
    if(from<0||to<0||from>=RB_STAGE_SIZE||to>=RB_STAGE_SIZE||from==to) return 0;
    RbPlayer *P=&g->p[who];
    int a=P->stage[from], b=P->stage[to];
    int aw=P->stage_wait[from], bw=P->stage_wait[to];
    RbBag au=P->under_cards[from], bu=P->under_cards[to];
    P->stage[from]=b; P->stage_wait[from]=bw; P->under_cards[from]=bu;
    P->stage[to]=a;   P->stage_wait[to]=aw;   P->under_cards[to]=au;
    record_movement(g,a);
    if(b>=0) record_movement(g,b);
    return 1;
}

/* Duration string → RB_TEMP_* kind. */
static int effect_duration(const AbilityEffect *e){
    const char *d = eff_extra(e,"duration");
    if(!d || !strcmp(d,"permanent"))      return RB_TEMP_PERM;
    if(!strcmp(d,"live_end") || !strcmp(d,"during_live")) return RB_TEMP_LIVE_END;
    return RB_TEMP_TURN_END;
}
static int duration_is_temporary(const AbilityEffect *e){
    const char *d = eff_extra(e,"duration");
    return d && strcmp(d,"permanent")!=0;
}

/* Register revert data for a temporary grant. */
static void push_temporary_effect(GameState *g, int card_id, int dur,
                                  int blade, const int heart[8]){
    if(card_id < 0 || dur == RB_TEMP_PERM) return;
    if(g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) return;
    RbTempEffect te; memset(&te,0,sizeof te);
    te.card_id=card_id; te.dur=dur; te.blade=blade;
    if(heart) for(int i=0;i<8;i++) te.heart[i]=heart[i];
    g->temp_effects[g->n_temp_effects++]=te;
}

/* Grant `amount` blades to one card. */
static void grant_blade(GameState *g, int card_id, int amount){
    if(card_id < 0) return;
    rb_mods_add_blade(&g->mods, card_id, amount);
}

/* Grant heart-color distribution to one card; is_negative flips every entry's sign. */
static void grant_heart_distribution(GameState *g, int card_id,
                                     const int *colors, const int *counts,
                                     int n, int is_negative){
    if(card_id < 0) return;
    for(int i=0;i<n;i++){
        int amt = is_negative ? -counts[i] : counts[i];
        rb_mods_add_heart(&g->mods, card_id, colors[i], amt);
    }
}

/* Grant distribution and, for temporary durations, register revert data. */
static void apply_heart_to_card(GameState *g, int card_id,
                                const int *colors, const int *counts,
                                int n, int is_negative, int dur){
    grant_heart_distribution(g, card_id, colors, counts, n, is_negative);
    if(dur == RB_TEMP_PERM || card_id < 0) return;
    int heart[8]; memset(heart,0,sizeof heart);
    for(int i=0;i<n;i++){
        int c=colors[i];
        if(c>=0 && c<8) heart[c] += is_negative ? -counts[i] : counts[i];
    }
    push_temporary_effect(g, card_id, dur, 0, heart);
}

static int card_appeared_this_turn(const GameState *g, int cid){
    for(int i=0;i<g->n_cards_appeared_this_turn;i++)
        if(g->cards_appeared_this_turn[i]==cid) return 1;
    return 0;
}
static int is_selected_card(const GameState *g, int cid){
    for(int i=0;i<g->n_selected_cards;i++)
        if(g->selected_cards[i]==cid) return 1;
    return 0;
}

#define RB_MISC_MAX_TARGETS (RB_STAGE_SIZE + RB_MAX_RECENTLY_MOVED)

/* util::matching_ids_filtered over the target's stage. */
static int collect_stage_candidates(const GameState *g, int who,
                                    const AbilityEffect *e,
                                    int exclude_self_id, int exclude_selected,
                                    int *out, int max){
    const RbPlayer *P=&g->p[who];
    const char *ctype  = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    const char *group  = eff_extra(e,"group_names");
    const char *chars  = eff_extra(e,"characters");
    const char *pos    = eff_extra(e,"position");
    const char *timing = eff_extra(e,"timing_condition");
    int pos_idx = pos ? rb_stage_position_index(pos) : -1;
    int n=0;
    for(int q=0;q<RB_STAGE_SIZE && n<max;q++){
        int cid=P->stage[q];
        if(cid==RB_EMPTY_SLOT) continue;
        if(exclude_self_id>=0 && cid==exclude_self_id) continue;
        if(pos_idx>=0 && pos_idx!=q) continue;
        if(ctype && !rb_card_matches_type(cid,ctype)) continue;
        if(group && !rb_card_matches_group_str(cid,group)) continue;
        if(chars){
            const char *names[1]={chars};
            if(!rb_card_matches_characters(cid,names,1)) continue;
        }
        if(timing){
            if(!strcmp(timing,"appeared_this_turn")){
                if(!card_appeared_this_turn(g,cid)) continue;
            } else if(!strcmp(timing,"moved_this_turn")){
                if(cid>=RB_MAX_CARD_IDS || !g->moved_this_turn[cid]) continue;
            }
        }
        if(exclude_selected && is_selected_card(g,cid)) continue;
        out[n++]=cid;
    }
    return n;
}

/* Per-unit multiplier: matching_count / per_unit_count * per_unit_base. */
static int calculate_gain_multiplier(const GameState *g, int who,
                                     const AbilityEffect *e,
                                     int per_unit, int base_count,
                                     const char *per_unit_type){
    if(!per_unit) return base_count;
    const char *loc  = eff_extra(e,"location");
    const char *zone = loc ? loc : per_unit_type;
    int matching=0;
    if(zone && !strcmp(zone,"つ")) matching = g->p[who].energy_active;
    else if(zone) matching = rb_count_in_zone(g, who, zone);
    if(per_unit_type &&
       (!strcmp(per_unit_type,"discard") || !strcmp(per_unit_type,"waitroom") ||
        !strcmp(per_unit_type,"waitroom_card"))){
        matching = g->n_recently_moved > 0 ? g->n_recently_moved : g->mods.last_cost_discard_count;
    } else if(per_unit_type && !strcmp(per_unit_type,"energy_deck")){
        matching = g->n_recently_moved;
    }
    int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : extra_int(e,"per_unit_count",1);
    if(per_unit_count <= 0) per_unit_count = 1;
    int units = matching / per_unit_count;
    int is_max = extra_true(e,"max");
    if(is_max && e->count>0 && units > e->count) units = e->count;
    int cap = e->repeat_limit > 0 ? e->repeat_limit : extra_int(e,"repeat_limit",0);
    if(cap>0 && units>cap) units = cap;
    int per_unit_base = is_max ? 1
        : extra_int(e,"resource_icon_count", e->count>0 ? e->count : 1);
    if(per_unit_base < 0) per_unit_base = 1;
    return units * per_unit_base;
}

/* Target sets for gain_resource. */
typedef struct {
    int  blade[RB_MISC_MAX_TARGETS];
    int  n_blade;
    int  heart[RB_MISC_MAX_TARGETS];
    int  n_heart;
    int  heart_color;   /* -1 = unspecified → RB_HEART_ALL at apply time */
    int  final_count;
} GainTargets;

/* Resolve which stage members receive the resource. */
static void resolve_gain_resource_targets(GameState *g, int who,
        const AbilityEffect *e, int kind, int count,
        int per_unit, const char *per_unit_type,
        int is_all, int is_self_target, int exclude_self_id,
        int activating, GainTargets *out){
    memset(out,0,sizeof *out);
    out->heart_color = -1;
    out->final_count = calculate_gain_multiplier(g,who,e,per_unit,count,per_unit_type);

    int tc       = extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || extra_true(e,"distinct");
    int has_sel  = (tc>=0) || distinct;
    int from_sel = extra_true(e,"target_from_selection");
    int multi    = extra_true(e,"multiple_targets");
    int nsel     = g->n_selected_cards;

    int cand[RB_MISC_MAX_TARGETS];
    int nc = collect_stage_candidates(g,who,e,exclude_self_id,
                                      has_sel && nsel>0, cand, RB_MISC_MAX_TARGETS);

    /* blade targets */
    if(from_sel){
        for(int i=0;i<nsel && out->n_blade<RB_MISC_MAX_TARGETS;i++)
            out->blade[out->n_blade++]=g->selected_cards[i];
    } else if(tc>=0){
        if(nsel>0 && nc==0)
            for(int i=0;i<nsel && out->n_blade<tc;i++)
                out->blade[out->n_blade++]=g->selected_cards[i];
        else
            for(int i=0;i<nc && out->n_blade<tc;i++)
                out->blade[out->n_blade++]=cand[i];
    } else if(nsel>0 && !distinct){
        for(int i=0;i<nsel && out->n_blade<RB_MISC_MAX_TARGETS;i++)
            out->blade[out->n_blade++]=g->selected_cards[i];
    } else {
        for(int i=0;i<nc && out->n_blade<RB_MISC_MAX_TARGETS;i++)
            out->blade[out->n_blade++]=cand[i];
    }

    /* heart color */
    if(g->queue.selected_heart_color >= 0)
        out->heart_color = g->queue.selected_heart_color;
    else {
        const char *hc = eff_extra(e,"heart_color");
        if(hc) out->heart_color = (int)rb_parse_heart_color(hc);
    }

    /* heart targets */
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    const char *group = eff_extra(e,"group_names");
    const char *chars = eff_extra(e,"characters");
    if(kind==RB_RES_HEART && per_unit && per_unit_type &&
       !strcmp(per_unit_type,"energy_deck") &&
       g->mods.n_last_under_move_host_ids > 0){
        for(int i=0;i<g->mods.n_last_under_move_host_ids && out->n_heart<RB_MISC_MAX_TARGETS;i++)
            out->heart[out->n_heart++]=g->mods.last_under_move_host_ids[i];
    } else if(from_sel){
        for(int i=0;i<nsel && out->n_heart<RB_MISC_MAX_TARGETS;i++)
            out->heart[out->n_heart++]=g->selected_cards[i];
    } else if(nsel>0 && !distinct && !has_sel){
        for(int i=0;i<nsel && out->n_heart<RB_MISC_MAX_TARGETS;i++)
            out->heart[out->n_heart++]=g->selected_cards[i];
        if(multi && activating>=0 && !is_selected_card(g,activating) &&
           out->n_heart < RB_MISC_MAX_TARGETS)
            out->heart[out->n_heart++]=activating;
    } else if(kind==RB_RES_HEART){
        if(!ctype && !group && !chars && tc<0 && !distinct && !is_all){
            if(activating>=0) out->heart[out->n_heart++]=activating;
        } else {
            int lim = (tc>=0 && !is_self_target && tc<RB_MISC_MAX_TARGETS) ? tc : RB_MISC_MAX_TARGETS;
            for(int i=0;i<nc && out->n_heart<lim;i++)
                out->heart[out->n_heart++]=cand[i];
        }
    }

    /* filter_targets_by_heart_colors */
    if(extra_true(e,"filter_targets_by_heart_colors")){
        const char *hc = eff_extra(e,"heart_color");
        if(hc){
            const char *cols[1]={hc};
            int keep=0;
            for(int i=0;i<out->n_heart;i++){
                int cid=out->heart[i];
                int ok = extra_true(e,"require_all_heart_colors")
                       ? rb_card_matches_all_heart_colors(cid,cols,1)
                       : rb_card_matches_heart_colors(cid,cols,1);
                if(ok) out->heart[keep++]=cid;
            }
            out->n_heart=keep;
        }
    }
}

/* Guard: the effect that emitted a target-selection choice. On re-entry the
   guard clears and the grant proceeds. */
static const AbilityEffect *s_target_choice_effect = NULL;

/* Create a target-selection choice when target_count > matching candidates.
   Returns 1 when a choice was created (caller must stop processing). */
static int try_create_target_selection_choice(GameState *g, int actor,
        const AbilityEffect *e, int kind, int who,
        int is_self_target, int per_unit, int exclude_self_id){
    int tc = extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || extra_true(e,"distinct");
    if(tc<0 || is_self_target || per_unit || kind==RB_RES_OTHER) return 0;
    if(!(g->n_selected_cards==0 || distinct)) return 0;
    if(s_target_choice_effect==e){ s_target_choice_effect=NULL; return 0; }
    int cand[RB_MISC_MAX_TARGETS];
    int nc = collect_stage_candidates(g,who,e,exclude_self_id,
                                      g->n_selected_cards>0,
                                      cand, RB_MISC_MAX_TARGETS);
    if(nc <= tc) return 0;
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", ctype, tc, 0, NULL);
    const char *group = eff_extra(e,"group_names");
    if(group) strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group)-1);
    g->queue.pending.filter_heart = -1;
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    g->queue.deferred = (AbilityEffect *)e;
    g->queue.resume_mode = 0;
    g->queue.resume_actor = actor;
    g->queue.resume_host  = s_activating_card;
    s_target_choice_effect = e;
    return 1;
}

/* Apply blade modifiers to resolved targets; register revert data if temporary. */
static void apply_blade_resource(GameState *g, const AbilityEffect *e, int kind,
        int who, const int *targets, int n_targets,
        int activating, int is_all, int dur, int final_count, int blades_to_add){
    if(kind != RB_RES_BLADE) return;
    const char *ctype  = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    const char *group  = eff_extra(e,"group_names");
    const char *chars  = eff_extra(e,"characters");
    const char *pos    = eff_extra(e,"position");
    const char *timing = eff_extra(e,"timing_condition");
    if(n_targets==0){
        if(is_all && !group && !ctype && !chars && !timing && !pos){
            RbPlayer *P=&g->p[who];
            for(int q=0;q<RB_STAGE_SIZE;q++){
                int cid=P->stage[q];
                if(cid==RB_EMPTY_SLOT) continue;
                grant_blade(g,cid,blades_to_add);
                push_temporary_effect(g,cid,dur,blades_to_add,NULL);
            }
        } else if(pos){
            int idx=rb_stage_position_index(pos);
            int cid=(idx>=0)?g->p[who].stage[idx]:RB_EMPTY_SLOT;
            if(cid!=RB_EMPTY_SLOT){
                grant_blade(g,cid,blades_to_add);
                push_temporary_effect(g,cid,dur,blades_to_add,NULL);
            }
        } else if(extra_int(e,"target_count",-1)<0 && !extra_true(e,"exclude_self")){
            grant_blade(g,activating,blades_to_add);
            push_temporary_effect(g,activating,dur,blades_to_add,NULL);
        }
        return;
    }
    int lim = n_targets;
    if(!(g->n_selected_cards>0 && !e->source) && !is_all && final_count<n_targets)
        lim = final_count;
    for(int i=0;i<lim;i++){
        grant_blade(g,targets[i],blades_to_add);
        push_temporary_effect(g,targets[i],dur,blades_to_add,NULL);
    }
}

/* Apply heart modifiers to resolved targets; return effect_data for temp cleanup. */
static void apply_heart_resource(GameState *g, const AbilityEffect *e, int kind,
        int who, const int *targets, int n_targets,
        int activating, int is_self_target, int is_all,
        int dur, int is_negative,
        const int *colors, const int *counts, int n_dist, int final_count){
    if(kind != RB_RES_HEART) return;
    const char *pos = eff_extra(e,"position");
    int tc = extra_int(e,"target_count",-1);
    if(n_targets==0){
        if(pos){
            int idx=rb_stage_position_index(pos);
            int cid=(idx>=0)?g->p[who].stage[idx]:RB_EMPTY_SLOT;
            if(cid!=RB_EMPTY_SLOT)
                apply_heart_to_card(g,cid,colors,counts,n_dist,is_negative,dur);
        } else if(tc<0 && (!eff_extra(e,"exclude_self") ||
                           (e->target && !strcmp(e->target,"self")))){
            apply_heart_to_card(g,activating,colors,counts,n_dist,is_negative,dur);
        }
        return;
    }
    if(is_self_target){
        apply_heart_to_card(g,activating,colors,counts,n_dist,is_negative,dur);
        return;
    }
    int lim = (is_all || extra_true(e,"multiple_targets")) ? n_targets
            : (final_count < n_targets ? final_count : n_targets);
    for(int i=0;i<lim;i++)
        apply_heart_to_card(g,targets[i],colors,counts,n_dist,is_negative,dur);
}

/* ─────────────────────── h_gain_heart_colors_from_selected_card ───────────────── */
static int h_gain_heart_colors_from_selected_card(GameState *g, int actor,
                                                   const AbilityEffect *e){
    int who = misc_target_player(actor,e);
    RbPlayer *P=&g->p[who];
    int dur = duration_is_temporary(e) ? effect_duration(e) : RB_TEMP_PERM;
    if(g->n_selected_cards==0) return 1;
    int selected = g->selected_cards[0];
    Card sc;
    if(!rb_decode_card_by_index((uint32_t)selected,&sc)) return 1;
    for(int h=0; h<sc.n_hearts && h<sc.num_base; h++){
        int color = sc.heart_color[h];
        if(color<0 || color>7) continue;
        for(int q=0;q<RB_STAGE_SIZE;q++){
            int tid=P->stage[q];
            if(tid==RB_EMPTY_SLOT) continue;
            int c1[1]={color}, n1[1]={1};
            apply_heart_to_card(g,tid,c1,n1,1,0,dur);
        }
    }
    rb_free_card(&sc);
    return 1;
}

/* ─────────────────────── h_gain_heart_all_type ───────────────────────────────── */
static int h_gain_heart_all_type(GameState *g, int actor, const AbilityEffect *e){
    int who = misc_target_player(actor,e);
    RbPlayer *P=&g->p[who];
    const char *pos = eff_extra(e,"position");
    int card_id=-1;
    if(pos){
        int idx=rb_stage_position_index(pos);
        if(idx>=0 && P->stage[idx]!=RB_EMPTY_SLOT) card_id=P->stage[idx];
    } else if(e->target && *e->target){
        for(int q=0;q<RB_STAGE_SIZE;q++)
            if(P->stage[q]!=RB_EMPTY_SLOT){ card_id=P->stage[q]; break; }
    }
    if(card_id<0) card_id=s_activating_card;
    if(card_id<0) return 1;
    int amount = e->count>0 ? e->count : 1;
    int c1[1]={RB_HEART_ALL}, n1[1]={amount};
    apply_heart_to_card(g,card_id,c1,n1,1,0,
                        duration_is_temporary(e)?effect_duration(e):RB_TEMP_PERM);
    return 1;
}

/* ─────────────────────── h_bp6_pattern ──────────────────────────────────────── */
static int h_bp6_pattern(GameState *g, int actor, const AbilityEffect *e){
    (void)actor;
    const char *res  = eff_extra(e,"resource");
    int per_unit     = (e->per_unit>0) || extra_true(e,"per_unit");
    const char *put  = eff_extra(e,"per_unit_type");
    int multi        = extra_true(e,"multiple_targets");
    if(!(res && !strcmp(res,"heart") && per_unit && put && !strcmp(put,"discard") && multi))
        return 0;
    int activating = s_activating_card;
    if(activating<0) return 1;
    int distinct[8]; int nd=0;
    for(int i=0;i<g->n_recently_moved;i++){
        int cid=g->recently_moved[i];
        Card c;
        if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
        for(int h=0; h<c.n_hearts && h<c.num_base; h++){
            int col=c.heart_color[h];
            if(col<0 || col>7) continue;
            int seen=0;
            for(int j=0;j<nd;j++) if(distinct[j]==col){seen=1;break;}
            if(!seen && nd<8) distinct[nd++]=col;
        }
        rb_free_card(&c);
    }
    int dur = duration_is_temporary(e)?effect_duration(e):RB_TEMP_PERM;
    for(int i=0;i<nd;i++){
        int amt=1;
        apply_heart_to_card(g,activating,&distinct[i],&amt,1,0,dur);
    }
    return 1;
}

/* ─────────────────────── h_gain_resource ────────────────────────────────────── */
static int h_gain_resource(GameState *g, int actor, const AbilityEffect *e){
    /* Special heart shapes first */
    const char *res = eff_extra(e,"resource");
    if(res && !strcmp(res,"heart") && extra_true(e,"heart_colors_from_selected_card"))
        return h_gain_heart_colors_from_selected_card(g,actor,e);
    if(res && !strcmp(res,"heart") && rb_is_all_heart_type(e))
        return h_gain_heart_all_type(g,actor,e);
    if(h_bp6_pattern(g,actor,e)) return 1;
    if(res && !strcmp(res,"surplus_heart")){
        rb_effect_gain_surplus_heart(g,actor,e);
        return 1;
    }

    int kind = resource_kind(res);
    int who  = misc_target_player(actor,e);
    RbPlayer *P=&g->p[who];

    if(kind==RB_RES_OTHER){
        int n = e->count>0 ? e->count : 1;
        P->energy.n += n;
        if(P->energy.n > RB_ENERGY_CAP) P->energy.n = RB_ENERGY_CAP;
        P->energy_active += n;
        if(P->energy_active > RB_ENERGY_CAP) P->energy_active = RB_ENERGY_CAP;
        return 1;
    }

    int count       = extra_int(e,"resource_icon_count", e->count>0 ? e->count : 1);
    int per_unit    = (e->per_unit>0) || extra_true(e,"per_unit");
    const char *per_unit_type = eff_extra(e,"per_unit_type");
    int is_self_target = e->self_target_field[0] && !strcmp(e->self_target_field,"true");
    int activating  = s_activating_card;
    int exclude_self_id = extra_true(e,"exclude_self") ? activating : -1;
    const char *sign = eff_extra(e,"sign");
    int is_negative = sign && !strcmp(sign,"negative");
    int dur = duration_is_temporary(e) ? effect_duration(e) : RB_TEMP_PERM;
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    int tc = extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || extra_true(e,"distinct");
    int player_target = e->target && (!strcmp(e->target,"self")||!strcmp(e->target,"opponent"));
    int is_member_ct = ctype && !strcmp(ctype,"member_card");
    int is_all = extra_true(e,"all")
              || (!e->source && is_member_ct && player_target && !is_self_target
                  && !eff_extra(e,"exclude_self") && tc<0)
              || (is_member_ct && player_target && tc<0 && !distinct);
    fprintf(stderr, "DEBUG [GAIN_RESOURCE] kind=%d is_all=%d per_unit=%d tc=%d distinct=%d group=%s ctype=%s who=%d actor=%d\n",
        kind, is_all, per_unit, tc, distinct, eff_extra(e,"group_names")?eff_extra(e,"group_names"):"none",
        ctype?ctype:"none", who, actor);

    if(try_create_target_selection_choice(g,actor,e,kind,who,is_self_target,
                                          per_unit,exclude_self_id))
        return 1;

    GainTargets t;
    resolve_gain_resource_targets(g,who,e,kind,count,per_unit,per_unit_type,
                                  is_all,is_self_target,exclude_self_id,
                                  activating,&t);

    /* Store picked ids when target_count/distinct is set. */
    if(tc>=0 || distinct){
        const int *picked = (kind==RB_RES_BLADE)?t.blade:t.heart;
        int np = (kind==RB_RES_BLADE)?t.n_blade:t.n_heart;
        for(int i=0;i<np;i++)
            if(!is_selected_card(g,picked[i]) && g->n_selected_cards<RB_MAX_RECENTLY_MOVED)
                g->selected_cards[g->n_selected_cards++]=picked[i];
    }

    int final_count    = t.final_count;
    int blades_to_add  = is_negative ? -final_count : final_count;
    int colors[1]      = { t.heart_color>=0 ? t.heart_color : RB_HEART_ALL };
    int counts[1]      = { final_count };

    /* is_self_target: activating card only */
    if(is_self_target && activating>=0){
        int on_stage=0;
        for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==activating){on_stage=1;break;}
        if(!on_stage) return 0;
        if(kind==RB_RES_BLADE){
            grant_blade(g,activating,blades_to_add);
            push_temporary_effect(g,activating,dur,blades_to_add,NULL);
        } else {
            apply_heart_to_card(g,activating,colors,counts,1,is_negative,dur);
        }
        g->queue.selected_heart_color=-1;
        rule_log_activated(g,activating,"[[log_gain_resource]]");
        return 1;
    }

    /* group_reference="same_group_name": restrict heart targets */
    if(extra_true(e,"group_reference")){
        const char *gref=eff_extra(e,"group_reference");
        if(gref && !strcmp(gref,"same_group_name") && g->n_recently_moved>0){
            char ref_group[64]; ref_group[0]=0;
            Card rc;
            if(rb_decode_card_by_index((uint32_t)g->recently_moved[0],&rc)){
                const char *gs=rc.group_idx ? rb_card_string(rc.group_idx) : NULL;
                if(gs) strncpy(ref_group,gs,sizeof ref_group-1);
                rb_free_card(&rc);
            }
            if(ref_group[0]){
                int keep=0;
                for(int i=0;i<t.n_heart;i++)
                    if(rb_card_matches_group_str(t.heart[i],ref_group))
                        t.heart[keep++]=t.heart[i];
                t.n_heart=keep;
            }
        }
    }

    apply_blade_resource(g,e,kind,who,t.blade,t.n_blade,activating,is_all,dur,
                          final_count,blades_to_add);
    fprintf(stderr, "DEBUG [GAIN_RESOURCE] applied blade: targets=%d final=%d blades=%d is_all=%d\n",
            t.n_blade, final_count, blades_to_add, is_all);

    apply_heart_resource(g,e,kind,who,t.heart,t.n_heart,activating,is_self_target,
                         is_all,dur,is_negative,colors,counts,1,final_count);

    g->queue.selected_heart_color=-1;
    if(dur==RB_TEMP_PERM) rb_recalc_constants(g);
    rule_log_activated(g,activating,"[[log_gain_resource]]");
    return 1;
}

/* ─────────────────────── h_pay_energy ───────────────────────────────────────── */
static int h_pay_energy(GameState *g, int actor, const AbilityEffect *e){
    int count = e->count;
    const char *dyn = eff_extra(e,"dynamic_count");
    if(dyn) count = rb_effect_count(g,actor,s_activating_card,e,g->last_draw_count);
    if(count<=0) count = e->count>0 ? e->count : 1;
    int ec = extra_int(e,"energy_count",-1);
    if(ec>=0) count=ec;

    if(extra_true(e,"optional") || e->is_optional){
        RbPlayer *P=&g->p[actor];
        if(rb_energy_active_count(P) < count){
            /* Insufficient energy: skip payment, cancel remaining */
            return 1;
        }
        rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,count,1,"pay_optional_cost");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
        return 1;
    }
    if(count>0){
        RbPlayer *P=&g->p[actor];
        rb_energy_pay(P,count);
        rb_recalc_constants(g);
    }
    if(s_activating_card>=0){
        char label[64];
        snprintf(label,sizeof label,"%dエネルギー支払",count);
        rule_log_activated(g,s_activating_card,label);
    }
    return 1;
}

/* ─────────────────────── h_discard_until_count ──────────────────────────────── */
static int h_discard_until_count(GameState *g, int actor, const AbilityEffect *e){
    int target = extra_int(e,"target_count", e->count>=0 ? e->count : 0);
    const char *tgt = e->target ? e->target : "self";
    int who = (!strcmp(tgt,"opponent")) ? actor^1 : actor;
    RbPlayer *P=&g->p[who];
    int cur = P->hand.n;
    if(cur <= target) return 1;
    int to_discard = cur - target;
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    rb_emit_choice(g,who,RB_CHOICE_SELECT_CARD,"hand",ctype,to_discard,0,"hand");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    if(s_activating_card>=0)
        rule_log_activated(g,s_activating_card,"[[log_discard_until]]");
    return 1;
}

/* ─────────────────────── h_restriction ──────────────────────────────────────── */
static int h_restriction(GameState *g, int actor, const AbilityEffect *e){
    const char *rtype = eff_extra(e,"restriction_type");
    const char *rdest = eff_extra(e,"restricted_destination");
    if(!rtype) rtype = eff_extra(e,"type");
    if(!rdest && e->destination) rdest = e->destination;
    int delayed = extra_true(e,"delayed");

    push_prohibition(g, rtype?rtype:"unknown", rdest?rdest:"");

    if(s_activating_card>=0)
        rule_log_activated(g,s_activating_card,"[[log_restriction]]");

    int is_cannot_active = rtype && (!strcmp(rtype,"cannot_activate_by_effect")
                              || !strcmp(rtype,"cannot_active")
                              || !strcmp(rtype,"cannot_activate"));
    if(is_cannot_active){
        int tgt = actor;
        if(e->target && !strcmp(e->target,"opponent")) tgt = actor^1;
        if(delayed){
            int keyed=0;
            for(int i=0;i<g->n_recently_moved;i++){
                int cid=g->recently_moved[i];
                if(cid>=0){ rb_mods_add_delayed_cannot_active(&g->mods,cid,1); keyed=1; }
            }
            if(!keyed && s_activating_card>=0)
                rb_mods_add_delayed_cannot_active(&g->mods,s_activating_card,1);
            for(int i=0;i<g->n_recently_moved && g->n_cannot_active_cards<RB_MAX_ZONE;i++)
                g->cannot_active_cards[g->n_cannot_active_cards++]=g->recently_moved[i];
            if(g->n_cannot_active_cards==0)
                for(int q=0;q<RB_STAGE_SIZE;q++)
                    if(g->p[tgt].stage[q]>=0 && g->n_cannot_active_cards<RB_MAX_ZONE)
                        g->cannot_active_cards[g->n_cannot_active_cards++]=g->p[tgt].stage[q];
        } else {
            g->player_cannot_activate[tgt] = 1;
        }
    }
    if(rtype && !strcmp(rtype,"cannot_live"))
        push_prohibition(g,"cannot_live", e->target?e->target:"self");
    if(rtype && !strcmp(rtype,"cannot_wait_by_effect"))
        push_prohibition(g,"cannot_wait_by_effect",
                         eff_extra(e,"group_names")?eff_extra(e,"group_names"):"");
    return 1;
}

/* ─────────────────────── h_re_yell ──────────────────────────────────────────── */
static int h_re_yell(GameState *g, int actor, const AbilityEffect *e){
    int lose_blade_hearts = extra_true(e,"lose_blade_hearts");
    if(lose_blade_hearts){
        RbPlayer *P=&g->p[actor];
        for(int i=0;i<RB_STAGE_SIZE;i++){
            int cid=P->stage[i];
            if(cid!=RB_EMPTY_SLOT) rb_mods_clear_card(&g->mods,cid);
        }
    }
    g->re_yell_occurred = 1;
    g->n_revealed = 0;
    push_prohibition(g,"re_yell","");
    if(s_activating_card>=0)
        rule_log_activated(g,s_activating_card,"[[log_re_yell]]");
    return 1;
}

/* ─────────────────────── h_perform_yell ─────────────────────────────────────── */
static int h_perform_yell(GameState *g, int actor, const AbilityEffect *e){
    (void)e;
    RbPlayer *P=&g->p[actor];
    for(int i=0;i<P->live.n;i++){
        Card c;
        if(rb_decode_card_by_index((uint32_t)P->live.cards[i],&c)){
            int blade=(int)c.blade + rb_mods_get_blade(&g->mods,P->live.cards[i]);
            if(blade>0) g->re_yell_blade_hearts[RB_HEART_PINK] += blade;
            rb_free_card(&c);
        }
    }
    g->re_yell_occurred = 1;
    return 1;
}

/* ─────────────────────── h_shuffle ──────────────────────────────────────────── */
static int h_shuffle(GameState *g, int actor, const AbilityEffect *e){
    const char *zone = e->target && *e->target ? e->target : "deck";
    RbBag *b=NULL;
    if(!strcmp(zone,"hand"))    b=&g->p[actor].hand;
    else if(!strcmp(zone,"deck"))   b=&g->p[actor].deck;
    else if(!strcmp(zone,"energy")) b=&g->p[actor].energy;
    else if(!strcmp(zone,"discard")) b=&g->p[actor].discard;
    if(!b || b->n<2) return 1;
    for(int i=b->n-1;i>0;i--){
        int j=rand()%(i+1);
        int t=b->cards[i]; b->cards[i]=b->cards[j]; b->cards[j]=t;
    }
    if(s_activating_card>=0)
        rule_log_activated(g,s_activating_card,"[[log_shuffle]]");
    return 1;
}

/* ─────────────────────── h_reveal (execute_reveal_effect) ───────────────────── */
static int h_reveal(GameState *g, int actor, const AbilityEffect *e){
    /* multiple_targets_any + source=deck_top → reveal_until_chosen_card path */
    if(extra_true(e,"multiple_targets") && e->source && !strcmp(e->source,"deck_top")){
        if(e->is_optional){
            rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,1,"pay_optional_cost");
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
            return 1;
        }
        /* Delegate to reveal_until_target (reads card_type / cost_limit from extras) */
        rb_effect_reveal_until_target(g, actor, (AbilityEffect *)e);
        return 1;
    }
    /* Simple reveal: count cards from source zone into revealed pool */
    int n = e->count>0 ? e->count : extra_int(e,"count",1);
    if(n<=0) n=1;
    RbPlayer *P=&g->p[actor];
    const char *src = e->source ? e->source : "deck";
    RbBag *src_bag = NULL;
    if(!strcmp(src,"deck"))            src_bag=&P->deck;
    else if(!strcmp(src,"hand"))       src_bag=&P->hand;
    else if(!strcmp(src,"energy"))     src_bag=&P->energy;
    else if(!strcmp(src,"discard"))    src_bag=&P->discard;
    if(!src_bag) return 1;
    for(int i=0;i<n && src_bag->n>0 && g->n_revealed<RB_MAX_RECENTLY_MOVED;i++){
        int cid=src_bag->cards[--src_bag->n];
        g->revealed_cards[g->n_revealed++]=cid;
    }
    return 1;
}

/* ─────────────────────── h_reveal_until_chosen_card ─────────────────────────── */
static int h_reveal_until_chosen_card(GameState *g, int actor, const AbilityEffect *e){
    rb_effect_reveal_until_chosen_card(g, actor, (AbilityEffect *)e);
    return 1;
}

/* ─────────────────────── h_activation_restriction ───────────────────────────── */
static int h_activation_restriction(GameState *g, int actor, const AbilityEffect *e){
    return h_restriction(g,actor,e);
}

/* ─────────────────────── h_choose_required_hearts ───────────────────────────── */
static int h_choose_required_hearts(GameState *g, int actor, const AbilityEffect *e){
    if(g->queue.resume_active) return 1;
    int cnt = e->count>=1 ? e->count : 1;
    rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,cnt,
                   e->is_optional?1:0,"choose_required_hearts");
    g->queue.resume_mode = 0;
    g->queue.resume_host = s_activating_card;
    g->queue.resume_actor = actor;
    return 1;
}

/* ─────────────────────── h_choose_target_player ─────────────────────────────── */
static int h_choose_target_player(GameState *g, int actor, const AbilityEffect *e){
    if(g->queue.resume_active) return 1;
    rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                   e->is_optional?1:0,"self_or_opponent");
    g->queue.resume_mode = 0;
    g->queue.resume_host = s_activating_card;
    g->queue.resume_actor = actor;
    return 1;
}

/* ─────────────────────── h_custom (execute_custom) ──────────────────────────── */
static int h_custom(GameState *g, int actor, const AbilityEffect *e){
    /* 1) Deck reordering: placement_order=any_order → route as move_cards */
    (void)actor;
    if(extra_true(e,"placement_order") ||
       (eff_extra(e,"placement_order") && !strcmp(eff_extra(e,"placement_order"),"any_order"))){
        AbilityEffect routed;
        memset(&routed,0,sizeof routed);
        routed.action = "move_cards";
        routed.source      = e->source      ? e->source      : (char *)"looked_at";
        routed.destination = e->destination ? e->destination : (char *)"deck_top";
        routed.is_optional = e->is_optional;
        for(int i=0;i<e->n_extra && i<RB_MAX_EXTRA;i++){
            routed.extra_k[i]=e->extra_k[i];
            routed.extra_v[i]=e->extra_v[i];
        }
        routed.n_extra = e->n_extra;
        rb_effect_move_cards(g, actor, &routed);
        return 1;
    }

    /* 2) Complex conditional scoring / gain_ability: has duration */
    if(eff_extra(e,"duration")){
        rb_gain_ability(g, actor, (AbilityEffect *)e);
        return 1;
    }

    /* Unhandled custom action: log */
    {
        const char *pp = player_prefix(g, s_activating_card);
        char act_name[96]; act_name[0]=0;
        if(s_activating_card>=0){
            Card c;
            if(rb_decode_card_by_index((uint32_t)s_activating_card,&c)){
                if(c.name){strncpy(act_name,c.name,sizeof act_name-1);act_name[sizeof act_name-1]=0;}
                rb_free_card(&c);
            }
        }
        char line[256];
        snprintf(line,sizeof line,"%s %s: [[log_custom_effect]]",pp,act_name);
        rb_log_push_verdict(line,"rule_log",1);
    }
    return 1;
}

/* ─────────────────────── h_play_baton_touch ─────────────────────────────────── */
static int h_play_baton_touch(GameState *g, int actor, const AbilityEffect *e){
    int count = e->count>=1 ? e->count : 1;
    int who = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2"))) who=actor^1;

    int bt_count = (who==0) ? g->baton_touch_count_p1 : g->baton_touch_count_p2;
    if(bt_count > 0) return 1;

    if(count > 1){
        /* Double baton: choose 2 occupied stage areas, excluding members deployed
           this turn (Rule 9.6.2.1.2.1). */
        RbPlayer *P=&g->p[who];
        int no=0;
        for(int i=0;i<RB_STAGE_SIZE;i++)
            if(P->stage[i]!=RB_EMPTY_SLOT && !g->stage_arrived[who][i]) no++;
        if(no<2) return 1;
        int pairs = no*(no-1)/2;
        rb_emit_choice(g,who,RB_CHOICE_SELECT_TARGET,NULL,NULL,pairs,1,"double_baton_touch");
        g->queue.resume_mode = 1;
        g->queue.resume_eff  = (AbilityEffect *)e;
        g->queue.resume_actor = who;
        g->queue.resume_host  = -1;
        return 1;
    }
    /* Single baton: record "baton_touch_allowed:<count>" */
    if(g->n_prohibition < 64){
        char *b = g->prohibition[g->n_prohibition]; int bi=0;
        const char *s="baton_touch_allowed:";
        for(const char *p=s;*p && bi<46;) b[bi++]=*p++;
        if(count>=10) b[bi++]=(char)('0'+(count/10));
        b[bi++]=(char)('0'+(count%10));
        b[bi]=0; g->n_prohibition++;
    }
    return 1;
}

/* ─────────────────────── h_place_energy_under_member ────────────────────────── */
static int h_place_energy_under_member(GameState *g, int actor, const AbilityEffect *e){
    int count = e->count>0 ? e->count : 1;
    const char *dyn = eff_extra(e,"dynamic_count");
    if(dyn) count = rb_effect_count(g,actor,s_activating_card,e,0);
    int any_number = extra_true(e,"any_number");
    const char *source = e->source ? e->source : eff_extra(e,"source");
    const char *dest   = e->destination;
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who = actor^1;
    RbPlayer *P=&g->p[who];

    /* wants_wait_energy_to_zone: source under_member + destination energy */
    int wants_wait_to_zone = source && !strcmp(source,"under_member")
                           && dest && !strcmp(dest,"energy");
    int canonical_to_zone = source && !strcmp(source,"energy_deck")
                          && dest && !strcmp(dest,"energy")
                          && eff_extra(e,"state_change")
                          && !strcmp(eff_extra(e,"state_change"),"wait");
    if(wants_wait_to_zone || canonical_to_zone){
        for(int i=0;i<count;i++){
            if(P->energy_deck.n==0) break;
            int cid=P->energy_deck.cards[--P->energy_deck.n];
            if(P->energy.n<RB_MAX_ZONE) P->energy.cards[P->energy.n++]=cid;
        }
        return 1;
    }
    /* under_member → empty_area */
    if(source && !strcmp(source,"under_member") && dest && !strcmp(dest,"empty_area")){
        int has_empty=0;
        for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==RB_EMPTY_SLOT) has_empty=1;
        if(!has_empty) return 1;
        int need = count>0 ? count : 1;
        rb_emit_choice(g,who,RB_CHOICE_SELECT_CARD,"under_member",
                       eff_extra(e,"card_type"),need,
                       e->is_optional?1:0,"under_member");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        return 1;
    }
    /* energy_deck → under_member */
    if(source && !strcmp(source,"energy_deck")){
        if(e->is_optional || extra_true(e,"optional")){
            rb_emit_choice(g,who,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,1,"pay_optional_cost");
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
            return 1;
        }
        const char *grp = eff_extra(e,"group_names");
        int idx=-1;
        for(int i=0;i<RB_STAGE_SIZE;i++)
            if(P->stage[i]!=RB_EMPTY_SLOT){
                if(grp && !rb_card_matches_group_str(P->stage[i],grp)) continue;
                if(s_activating_card>=0 && P->stage[i]==s_activating_card){idx=i;break;}
                if(idx<0) idx=i;
            }
        if(idx<0) idx=1;
        for(int i=0;i<count;i++){
            if(P->energy_deck.n==0) break;
            int cid=P->energy_deck.cards[--P->energy_deck.n];
            rb_stage_place_under_card(P, idx, cid);
        }
        rb_recalc_constants(g);
        return 1;
    }
    /* under_member (general) */
    if(source && !strcmp(source,"under_member")){
        int need = any_number ? 0 : count;
        rb_emit_choice(g,who,RB_CHOICE_SELECT_CARD,"under_member",
                       eff_extra(e,"card_type"),need,
                       e->is_optional?1:0,"under_member");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        return 1;
    }
    /* energy_zone → under_member (default) */
    if(P->energy.n==0) return 1;
    {
        int area=1;
        if(dest && *dest) area=rb_pos_to_area(dest);
        if(area<0||area>=RB_STAGE_SIZE) area=1;
        if(P->stage[area]==RB_EMPTY_SLOT) return 0;
        rb_emit_choice(g,who,RB_CHOICE_SELECT_CARD,"energy","energy_card",
                       count,e->is_optional?1:0,"under_member");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        return 1;
    }
}

/* ─────────────────────── h_execute_position_change ─────────────────────────── */
static int h_rotation(GameState *g, int actor, const AbilityEffect *e);
static int h_execute_position_change(GameState *g, int actor, const AbilityEffect *e){
    int target_is_both = e->target && !strcmp(e->target,"both");
    const char *trigger_type  = eff_extra(e,"trigger_type");
    const char *target        = e->target ? e->target : "self";
    const char *target_member = eff_extra(e,"target_member");
    if(!target_member) target_member = "this_member";
    const char *pos_param = eff_extra(e,"position")
                         ? eff_extra(e,"position")
                         : eff_extra(e,"source_position");

    RbPlayer *P=&g->p[actor];
    if(s_activating_card>=0)
        rule_log_activated(g,s_activating_card,"[[log_position_change]]");

    /* each_time watcher */
    if(trigger_type && !strcmp(trigger_type,"each_time")){
        int tm = g->queue.resume_host>=0 ? g->queue.resume_host : s_activating_card;
        int cur=-1;
        for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==tm){cur=i;break;}
        if(cur>=0){
            rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                           e->is_optional?1:0,"position|destination");
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
            return 1;
        }
        return 1;
    }
    /* destination already specified */
    const char *dest = e->destination;
    if(!dest) dest = eff_extra(e,"destination");
    if(dest && *dest){
        if(!strcmp(dest,"front") && !strcmp(target,"opponent")){
            rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                           e->is_optional?1:0,"position|destination");
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
            return 1;
        }
        return rb_position_change_with_destination(g,actor,e,dest,s_activating_card);
    }
    /* target_member == select */
    if(target_member && !strcmp(target_member,"select")){
        rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                       e->is_optional?1:0,"position|destination");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
        return 1;
    }
    /* both target */
    if(target_is_both){
        rb_emit_choice(g,actor^1,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                       e->is_optional?1:0,"position|destination");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
        return 1;
    }
    /* multiple_targets + position → rotation */
    if(extra_true(e,"multiple_targets") && pos_param)
        return h_rotation(g,actor,e);
    /* multiple_targets formation (this_member multiple) */
    if(extra_true(e,"multiple_targets") && !strcmp(target_member,"this_member")){
        rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                       e->is_optional?1:0,"position|destination");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
        return 1;
    }
    /* source-position branch */
    if(pos_param && *pos_param){
        int src=rb_stage_position_index(pos_param);
        if(src>=0 && P->stage[src]!=RB_EMPTY_SLOT){
            rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                           e->is_optional?1:0,"position|destination");
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
            return 1;
        }
        return 1;
    }
    /* generic this_member move of activating card */
    {
        int from=-1;
        for(int i=0;i<RB_STAGE_SIZE;i++) if(P->stage[i]==s_activating_card){from=i;break;}
        if(from<0){
            rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                           e->is_optional?1:0,"position|destination");
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
            return 1;
        }
        rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,1,
                       e->is_optional?1:0,"position|destination");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
        return 1;
    }
}

/* ─────────────────────── h_rotation ─────────────────────────────────────────── */
static int h_rotation(GameState *g, int actor, const AbilityEffect *e){
    (void)e;
    /* Normalize "both" to self */
    int tgt = actor;
    RbPlayer *P=&g->p[tgt];

    /* Snapshot current stage */
    int snap_cards[RB_STAGE_SIZE];
    RbBag snap_under[RB_STAGE_SIZE];
    int snap_wait[RB_STAGE_SIZE];
    for(int i=0;i<RB_STAGE_SIZE;i++){
        snap_cards[i]=P->stage[i];
        snap_under[i]=P->under_cards[i];
        snap_wait[i]=P->stage_wait[i];
    }

    /* Rotation: left(0)→right(2), center(1)→left(0), right(2)→center(1) */
    const int rot_map[3]={2,0,1};
    for(int i=0;i<RB_STAGE_SIZE;i++){
        P->stage[i]=RB_EMPTY_SLOT;
        P->under_cards[i].n=0;
        P->stage_wait[i]=0;
    }
    for(int src=0;src<RB_STAGE_SIZE;src++){
        int cid=snap_cards[src];
        if(cid==RB_EMPTY_SLOT) continue;
        int dst=rot_map[src];
        P->stage[dst]=cid;
        P->under_cards[dst]=snap_under[src];
        P->stage_wait[dst]=snap_wait[src];
        record_movement(g,cid);
    }
    g->position_change_occurred_this_turn=1;
    g->formation_change_occurred_this_turn=1;
    rb_recalc_constants(g);
    if(s_activating_card>=0)
        rule_log_activated(g,s_activating_card,"[[log_rotation]]");
    return 1;
}

/* ─────────────────────── h_choice (execute_choice) ──────────────────────────── */
static int h_choice(GameState *g, int actor, const AbilityEffect *e){
    /* If already resolving (resume path), skip re-emit */
    if(g->queue.resume_active) return 1;

    const char *choice_maker = eff_extra(e,"choice_maker");
    int who = actor;
    if(choice_maker && !strcmp(choice_maker,"opponent")) who = actor^1;

    if(g->queue.has_pending) return 1;
    if(g->queue.selected_heart_color>=0) return 1;

    int cnt   = e->count>=0 ? e->count : 1;
    int allow = e->is_optional ? 1 : 0;

    /* Propagate parent choice's group_names to each child option (unless options
       define their own groups). */
    const char *group_names = eff_extra(e,"group_names");
    if(group_names && *group_names){
        /* This is a selection-filter group_names: it narrows which cards can be
           chosen. The C choice model doesn't have per-option group propagation
           (that's handled in Rust via ConditionalChoice::Effects); we record the
           group in the pending choice's filter_group. */
        strncpy(g->queue.pending.filter_group, group_names,
                sizeof(g->queue.pending.filter_group)-1);
    }

    rb_emit_choice(g, who, RB_CHOICE_SELECT_TARGET, NULL, NULL, cnt, allow, "choice");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
    g->queue.resume_mode  = 0;
    g->queue.resume_actor = who;
    return 1;
}

/* ─────────────────────── rb_effect_pay_cost_all_discard ─────────────────────── */
int rb_effect_pay_cost_all_discard(GameState *g, int actor, const AbilityEffect *e){
    int who = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2")))
        who = actor^1;
    RbPlayer *P=&g->p[who];
    while(P->hand.n>0){
        int cid=P->hand.cards[--P->hand.n];
        rb_waitroom_add(P,cid);
    }
    return 1;
}

/* ─────────────────────── rb_effect_gain_surplus_heart ───────────────────────── */
void rb_effect_gain_surplus_heart(GameState *g, int actor, const AbilityEffect *e){
    if(!g||!e) return;
    int pl = actor;
    if(e->target && (!strcmp(e->target,"opponent")||!strcmp(e->target,"p2")))
        pl = actor^1;
    const char *sign = eff_extra(e,"sign");
    int is_all = extra_true(e,"all");
    const char *dur = eff_extra(e,"duration");
    int is_temp = dur && strcmp(dur,"permanent")!=0;

    if(!is_all){
        const char *ctype = e->card_type_field[0]?e->card_type_field:eff_extra(e,"card_type");
        int is_member = ctype && !strcmp(ctype,"member_card");
        int tc = extra_int(e,"target_count",-1);
        int target_is_self = !e->target || !strcmp(e->target,"self");
        if(is_member && (target_is_self||!strcmp(e->target?e->target:"","opponent")) && tc<0)
            is_all=1;
    }
    int surplus=0;
    for(int i=g->n_snapshots-1;i>=0;i--){
        if(g->snapshots[i].player==pl){
            int s=g->snapshots[i].surplus_hearts;
            surplus = (s>=0)?s:0;
            break;
        }
    }
    if(sign && !strcmp(sign,"negative") && is_all)
        g->last_surplus_loss_count[pl]=surplus;
    if(is_temp){
        /* Store for revert on expiry via prohibition note */
        char buf[32];
        snprintf(buf,sizeof buf,"%d",surplus);
        push_prohibition(g,"surplus_heart_temp",buf);
    }
}

/* ─────────────────────── compute_valid_position_destinations ────────────────── */
int rb_misc_position_destinations(const GameState *g, int actor,
                                   const AbilityEffect *e,
                                   int host_cid, const RbFormationSlot *plan,
                                   int n_plan, int *out_areas, int max){
    const RbPlayer *P=&g->p[actor];
    const char *exclude_pos = eff_extra(e,"exclude_position");
    const char *group       = eff_extra(e,"group_names");
    int exclude_self = extra_true(e,"exclude_self");
    int is_formation = extra_true(e,"multiple_targets");
    int n=0;
    int planned[3]={0,0,0};
    for(int i=0;i<n_plan;i++)
        if(plan[i].dest_area>=0 && plan[i].dest_area<3) planned[plan[i].dest_area]=1;
    for(int q=0;q<RB_STAGE_SIZE && n<max;q++){
        const char *pos_name = q==0?"left":q==1?"center":"right";
        if(planned[q]) continue;
        if(exclude_pos && !strcmp(pos_name,exclude_pos)) continue;
        if(exclude_self && P->stage[q]==host_cid) continue;
        int cid=P->stage[q];
        if(group){
            if(cid==RB_EMPTY_SLOT){
                if(!is_formation) continue;
            } else {
                if(!rb_card_matches_group_str(cid,group)) continue;
            }
        }
        out_areas[n++]=q;
    }
    return n;
}

/* ─────────────────────── finalize_formation_change ──────────────────────────── */
int rb_misc_finalize_formation_change(GameState *g, int actor,
                                      const RbFormationSlot *plan, int n_plan){
    if(n_plan==0) return 0;
    RbPlayer *P=&g->p[actor];
    int old_stage[RB_STAGE_SIZE];
    RbBag   old_under[RB_STAGE_SIZE];
    for(int q=0;q<RB_STAGE_SIZE;q++){
        old_stage[q]=P->stage[q];
        old_under[q]=P->under_cards[q];
    }
    int new_stage[RB_STAGE_SIZE];
    RbBag   new_under[RB_STAGE_SIZE];
    for(int q=0;q<RB_STAGE_SIZE;q++){
        new_stage[q]=RB_EMPTY_SLOT;
        new_under[q].n=0;
    }
    int moved=0;
    /* Phase 1: planned cards to dest */
    for(int i=0;i<n_plan;i++){
        int mid=plan[i].member_id, dst=plan[i].dest_area;
        if(mid<0||dst<0||dst>=RB_STAGE_SIZE) continue;
        int from=-1;
        for(int q=0;q<RB_STAGE_SIZE;q++) if(old_stage[q]==mid){from=q;break;}
        if(from<0) continue;
        if(from==dst) continue;
        new_stage[dst]=mid;
        new_under[dst]=old_under[from];
        moved++;
        record_movement(g,mid);
    }
    /* Phase 2: evicted / stay-in-place */
    for(int i=0;i<n_plan;i++){
        int mid=plan[i].member_id, dst=plan[i].dest_area;
        if(mid<0||dst<0||dst>=RB_STAGE_SIZE) continue;
        int from=-1;
        for(int q=0;q<RB_STAGE_SIZE;q++) if(old_stage[q]==mid){from=q;break;}
        if(from<0) continue;
        if(new_stage[from]!=RB_EMPTY_SLOT) continue;
        if(from==dst){
            new_stage[from]=mid; new_under[from]=old_under[from];
        } else {
            int evicted=old_stage[dst];
            int evicted_is_planned=0;
            for(int j=0;j<n_plan;j++) if(plan[j].member_id==evicted){evicted_is_planned=1;break;}
            if(evicted!=RB_EMPTY_SLOT && evicted!=mid && !evicted_is_planned
               && new_stage[from]==RB_EMPTY_SLOT){
                new_stage[from]=evicted;
                new_under[from]=old_under[dst];
                record_movement(g,evicted);
                moved++;
            }
        }
    }
    /* Phase 3: unplanned keep */
    for(int q=0;q<RB_STAGE_SIZE;q++){
        int cid=old_stage[q];
        if(cid==RB_EMPTY_SLOT) continue;
        int is_planned=0;
        for(int i=0;i<n_plan;i++) if(plan[i].member_id==cid){is_planned=1;break;}
        if(!is_planned && new_stage[q]==RB_EMPTY_SLOT){
            new_stage[q]=cid; new_under[q]=old_under[q];
        }
    }
    for(int q=0;q<RB_STAGE_SIZE;q++){
        P->stage[q]=new_stage[q];
        P->under_cards[q]=new_under[q];
    }
    g->position_change_occurred_this_turn=1;
    rb_recalc_constants(g);
    return moved;
}

/* ─────────────────────── rb_position_change_with_destination ────────────────── */
int rb_position_change_with_destination(GameState *g, int actor,
                                         const AbilityEffect *e,
                                         const char *destination, int host_cid){
    if(!destination || !strcmp(destination,"same_area")) return 1;
    int tgt = actor;
    if(e->target && !strcmp(e->target,"opponent")) tgt=actor;
    else if(e->target && !strcmp(e->target,"both"))  tgt=actor;
    RbPlayer *P=&g->p[tgt];

    /* exclude_position check */
    const char *exclude_pos = eff_extra(e,"exclude_position");
    if(exclude_pos && !strcmp(destination,exclude_pos)) return 0;

    /* front resolution */
    char front_buf[16];
    const char *dest_use = destination;
    if(!strcmp(destination,"front")){
        int src_front=-1;
        if(host_cid>=0)
            for(int q=0;q<RB_STAGE_SIZE;q++)
                if(g->p[actor].stage[q]==host_cid){src_front=q;break;}
        int front_idx = src_front>=0 ? (RB_STAGE_SIZE-1-src_front) : 0;
        const char *names[3]={"left","center","right"};
        dest_use = names[front_idx];
        strncpy(front_buf,dest_use,sizeof front_buf-1);
    }
    int dst=rb_stage_position_index(dest_use);
    if(dst<0||dst>=RB_STAGE_SIZE) return 0;

    const char *source_pos = eff_extra(e,"source_position")
                          ? eff_extra(e,"source_position")
                          : eff_extra(e,"position");
    if(source_pos && *source_pos){
        int src=rb_stage_position_index(source_pos);
        if(src<0||src>=RB_STAGE_SIZE) return 0;
        if(P->stage[src]==RB_EMPTY_SLOT) return 1;
        if(src==dst) return 1;
        stage_swap(g,tgt,src,dst);
        g->position_change_occurred_this_turn=1;
        rb_recalc_constants(g);
        return 1;
    }
    const char *target_member = eff_extra(e,"target_member");
    if(target_member && strcmp(target_member,"this_member")!=0){
        /* card_no branch */
        int cur=-1;
        for(int i=0;i<RB_STAGE_SIZE;i++){
            int cid=P->stage[i];
            if(cid==RB_EMPTY_SLOT) continue;
            Card c;
            if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
            int match = c.card_no_idx && rb_card_string(c.card_no_idx)
                     && !strcmp(rb_card_string(c.card_no_idx),target_member);
            rb_free_card(&c);
            if(match){cur=i;break;}
        }
        if(cur<0) return 1;
        if(cur==dst) return 1;
        stage_swap(g,tgt,cur,dst);
        g->position_change_occurred_this_turn=1;
        rb_recalc_constants(g);
        return 1;
    }
    /* this_member branch */
    int src=-1;
    if(host_cid>=0)
        for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==host_cid){src=q;break;}
    if(src<0) return 0;
    if(src==dst) return 1;
    stage_swap(g,tgt,src,dst);
    g->position_change_occurred_this_turn=1;
    rb_recalc_constants(g);
    return 1;
}

/* ─────────────────────── handle_both_targets ────────────────────────────────── */
int rb_misc_handle_both_targets(GameState *g, int actor, const AbilityEffect *e){
    if(!e || !e->target || strcmp(e->target,"both")) return 0;
    s_activating_card = -1;
    rb_execute_misc_effect(g, actor,   &g->p[actor],   e, NULL);
    rb_execute_misc_effect(g, actor^1, &g->p[actor^1], e, NULL);
    return 1;
}

/* ─────────────────────── rb_execute_misc_effect_ex ──────────────────────────── */
int rb_execute_misc_effect_ex(GameState *g, int actor, const RbPlayer *self,
                               const AbilityEffect *e, int host_cid, int *resolved){
    s_activating_card = host_cid;
    int r = rb_execute_misc_effect(g, actor, self, e, resolved);
    s_activating_card = -1;
    return r;
}

/* ─────────────────────── rb_effect_place_energy_under_member_non_optional ────── */
int rb_effect_place_energy_under_member_non_optional(GameState *g, int actor,
                                                      const AbilityEffect *e){
    if(!g||!e) return 0;
    RbPlayer *P=&g->p[actor];
    int area=1;
    const char *dest=e->destination && *e->destination ? e->destination : e->target;
    if(dest && *dest) area=rb_pos_to_area(dest);
    if(area<0||area>=RB_STAGE_SIZE) area=1;
    if(P->stage[area]<0) return 0;
    const char *source=e->source;
    int n=e->count>0 ? e->count : 1;
    if(source && !strcmp(source,"energy_deck")){
        int moved=0;
        while(moved<n && P->energy_deck.n>0){
            int cid=P->energy_deck.cards[--P->energy_deck.n];
            if(P->under_cards[area].n<RB_MAX_ZONE)
                P->under_cards[area].cards[P->under_cards[area].n++]=cid;
            moved++;
        }
        rb_recalc_constants(g);
        return 1;
    }
    if(source && (!strcmp(source,"under_member")||!strcmp(source,"energy_deck"))
       && e->destination && !strcmp(e->destination,"energy_zone")){
        int moved=0;
        while(moved<n && P->energy_deck.n>0){
            int cid=P->energy_deck.cards[--P->energy_deck.n];
            if(P->energy.n<RB_MAX_ZONE)
                P->energy.cards[P->energy.n++]=cid;
            moved++;
        }
        return 1;
    }
    int moved=0;
    while(moved<n && P->energy.n>0){
        int cid=P->energy.cards[--P->energy.n];
        if(P->under_cards[area].n<RB_MAX_ZONE)
            P->under_cards[area].cards[P->under_cards[area].n++]=cid;
        moved++;
    }
    rb_recalc_constants(g);
    return 1;
}

/* ─────────────────────── card_name helper ───────────────────────────────────── */
static const char *card_name(int card_id, char *buf, size_t cap){
    if(card_id>=0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)card_id,&c)){
            if(c.name && c.name[0]){
                strncpy(buf,c.name,cap-1);
                buf[cap-1]=0;
                rb_free_card(&c);
                return buf;
            }
            rb_free_card(&c);
        }
    }
    snprintf(buf,cap,"Card#%d",card_id);
    return buf;
}

/* ─────────────────────── dispatch ───────────────────────────────────────────── */
int rb_execute_misc_effect(GameState *g, int actor, const RbPlayer *self,
                            const AbilityEffect *e, int *resolved){
    if(!e) return 0;
    const char *name = e->action;
    int r=1;
    if(name){
        if      (!strcmp(name,"gain_resource"))              r=h_gain_resource(g,actor,e);
        else if (!strcmp(name,"pay_energy"))                 r=h_pay_energy(g,actor,e);
        else if (!strcmp(name,"discard_until_count"))         r=h_discard_until_count(g,actor,e);
        else if (!strcmp(name,"restriction"))                r=h_restriction(g,actor,e);
        else if (!strcmp(name,"choice"))                     r=h_choice(g,actor,e);
        else if (!strcmp(name,"position_change"))            r=h_execute_position_change(g,actor,e);
        else if (!strcmp(name,"rotation"))                   r=h_rotation(g,actor,e);
        else if (!strcmp(name,"place_energy_under_member"))  r=h_place_energy_under_member(g,actor,e);
        else if (!strcmp(name,"play_baton_touch"))           r=h_play_baton_touch(g,actor,e);
        else if (!strcmp(name,"re_yell"))                    r=h_re_yell(g,actor,e);
        else if (!strcmp(name,"perform_yell"))               r=h_perform_yell(g,actor,e);
        else if (!strcmp(name,"shuffle"))                    r=h_shuffle(g,actor,e);
        else if (!strcmp(name,"reveal"))                     r=h_reveal(g,actor,e);
        else if (!strcmp(name,"reveal_until_chosen_card"))   r=h_reveal_until_chosen_card(g,actor,e);
        else if (!strcmp(name,"activation_restriction"))     r=h_activation_restriction(g,actor,e);
        else if (!strcmp(name,"choose_required_hearts"))     r=h_choose_required_hearts(g,actor,e);
        else if (!strcmp(name,"choose_target_player"))       r=h_choose_target_player(g,actor,e);
        else if (!strcmp(name,"custom"))                     r=h_custom(g,actor,e);
        else if (!strcmp(name,"gain_surplus_heart"))         { rb_effect_gain_surplus_heart(g,actor,e); r=1; }
        else if (!strcmp(name,"pay_cost_all:discard_all"))   r=rb_effect_pay_cost_all_discard(g,actor,e);
        else r=0;
    }
    (void)self;
    if(resolved) *resolved=r;
    return r;
}
