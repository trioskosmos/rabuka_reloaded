/* effects/misc.c — miscellaneous ability-effect handlers.
   Mirror engine/src/ability/effects/misc.rs (execute_gain_resource,
   play_baton_touch, place_energy_under_member, position_change, rotation,
   choice, pay_energy, discard_until_count, restriction, re_yell,
   perform_yell, shuffle, ...).

   STUBS: each handler mirrors its Rust counterpart's signature and returns
   the permissive default. The dispatch rb_execute_misc_effect routes by
   effect name so callers (engine.c) can delegate unknown effect types here
   without touching the main switch. Fill handlers in one by one. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* Mirror misc.rs:execute_gain_surplus_heart — capture this player's live surplus
   (total_hearts − total_required) so it can be granted/lost as a resource. */
void rb_effect_gain_surplus_heart(GameState *g, int actor, const AbilityEffect *e);

/* ══════════════════ shared helpers (the private fns of misc.rs) ══════════════════ */

/* Read one of the effect's extra_kv fields. The C decoder flattens every scalar
   wire field into extra_k/extra_v, so Rust's `effect.foo_any()` is eff_extra("foo"). */
static const char *eff_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}
/* `effect.foo_any().unwrap_or(false)` */
static int extra_true(const AbilityEffect *e, const char *k){
    const char *v = eff_extra(e,k);
    return v && (!strcmp(v,"true") || !strcmp(v,"1"));
}
/* `effect.foo_any().unwrap_or(dflt)` for the numeric fields */
static int extra_int(const AbilityEffect *e, const char *k, int dflt){
    const char *v = eff_extra(e,k);
    if(!v || !*v) return dflt;
    return atoi(v);
}

/* ResourceKind (mirror misc.rs:ResourceKind::from_str) — card data spells the
   resource in EN or JA; normalize once instead of matching raw strings ad hoc. */
#define RB_RES_OTHER 0
#define RB_RES_BLADE 1
#define RB_RES_HEART 2
static int resource_kind(const char *s){
    if(!s) return RB_RES_OTHER;
    if(!strcmp(s,"blade") || !strcmp(s,"ブレード")) return RB_RES_BLADE;
    if(!strcmp(s,"heart") || !strcmp(s,"ハート")) return RB_RES_HEART;
    return RB_RES_OTHER;
}

/* Mirrors gs.activating_card — the card whose ability is resolving. Threaded in by
   rb_execute_misc_effect_ex; rb_execute_misc_effect leaves it -1 (unknown host). */
static int s_activating_card = -1;

/* `effect.target_name()` → player index ("both" resolves to self, as in Rust). */
static int misc_target_player(int actor, const AbilityEffect *e){
    if(e->target && !strcmp(e->target,"opponent")) return actor ^ 1;
    return actor;
}

/* Mirror misc.rs:player_prefix — "P1"/"P2" for the activating card's owner
   (stage / live zone / hand), else the active player. */
static const char *player_prefix(const GameState *g, int card_id){
    if(card_id >= 0){
        for(int pl=0; pl<2; pl++){
            const RbPlayer *P = &g->p[pl];
            for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==card_id) return pl==0?"P1":"P2";
            for(int i=0;i<P->live.n;i++) if(P->live.cards[i]==card_id) return pl==0?"P1":"P2";
            for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id) return pl==0?"P1":"P2";
        }
    }
    return g->active == 0 ? "P1" : "P2";
}

/* Mirror misc.rs:rule_log_activated — "P1 <card name>: <label>" into the rule log
   (the C rule log is the verdict buffer, gated by ABILITY_DEBUG). */
static void rule_log_activated(const GameState *g, int card_id, const char *label){
    char name[96]; name[0]=0;
    if(card_id >= 0){
        Card c;
        if(rb_decode_card_by_index((uint32_t)card_id, &c)){
            if(c.name){ strncpy(name, c.name, sizeof name - 1); name[sizeof name - 1]=0; }
            rb_free_card(&c);
        }
    }
    char line[256];
    snprintf(line, sizeof line, "%s %s: %s", player_prefix(g, card_id), name, label?label:"");
    rb_log_push_verdict(line, "rule_log", 1);
}

/* Mirror gs.prohibition_effects.push("<a>:<b>"). */
static void push_prohibition(GameState *g, const char *a, const char *b){
    if(g->n_prohibition >= 64) return;
    char *out = g->prohibition[g->n_prohibition];
    size_t cap = sizeof(g->prohibition[0]) - 1;
    size_t i = 0;
    for(const char *p = a?a:""; *p && i<cap; ) out[i++] = *p++;
    if(i < cap) out[i++] = ':';
    for(const char *p = b?b:""; *p && i<cap; ) out[i++] = *p++;
    out[i] = 0;
    g->n_prohibition++;
}

/* Mirror gs.record_card_movement — recently_moved is a ring of the latest moves. */
static void record_movement(GameState *g, int cid){
    if(cid < 0) return;
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
    else {
        for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++) g->recently_moved[i-1]=g->recently_moved[i];
        g->recently_moved[RB_MAX_RECENTLY_MOVED-1]=cid;
    }
}

/* Mirror stage.position_change(from,to): the two areas SWAP (the member standing
   at the destination moves to the source), carrying wait state + under_cards. */
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

/* util::push_temporary_effect — Duration string → RB_TEMP_* revert bucket. */
static int effect_duration(const AbilityEffect *e){
    const char *d = eff_extra(e,"duration");
    if(!d || !strcmp(d,"permanent")) return RB_TEMP_PERM;
    if(!strcmp(d,"live_end") || !strcmp(d,"during_live")) return RB_TEMP_LIVE_END;
    return RB_TEMP_TURN_END;
}
static int duration_is_temporary(const AbilityEffect *e){
    const char *d = eff_extra(e,"duration");
    return d && strcmp(d,"permanent") != 0;
}
/* Register the revert data for a temporary grant (Rust EffectData::SingleCard /
   MultiCard); rb_check_expired_effects undoes it when the duration expires. */
static void push_temporary_effect(GameState *g, int card_id, int dur, int blade, const int heart[8]){
    if(card_id < 0 || dur == RB_TEMP_PERM) return;
    if(g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) return;
    RbTempEffect te; memset(&te, 0, sizeof te);
    te.card_id = card_id; te.dur = dur; te.blade = blade;
    if(heart) for(int i=0;i<8;i++) te.heart[i]=heart[i];
    g->temp_effects[g->n_temp_effects++] = te;
}

/* Mirror misc.rs:grant_blade — grant `amount` blades to one card. */
static void grant_blade(GameState *g, int card_id, int amount){
    if(card_id < 0) return;
    rb_mods_add_blade(&g->mods, card_id, amount);
}

/* Mirror misc.rs:grant_heart_distribution — grant a heart-color distribution to
   one card; `is_negative` flips the sign of every entry so sign handling cannot
   drift apart between the heart-granting paths. */
static void grant_heart_distribution(GameState *g, int card_id, const int *colors,
                                     const int *counts, int n, int is_negative){
    if(card_id < 0) return;
    for(int i=0;i<n;i++){
        int amt = is_negative ? -counts[i] : counts[i];
        rb_mods_add_heart(&g->mods, card_id, colors[i], amt);
    }
}

/* Mirror misc.rs:apply_heart_to_card — grant the distribution and, for temporary
   durations, record the revert data. */
static void apply_heart_to_card(GameState *g, int card_id, const int *colors, const int *counts,
                                int n, int is_negative, int dur){
    grant_heart_distribution(g, card_id, colors, counts, n, is_negative);
    if(dur == RB_TEMP_PERM || card_id < 0) return;
    int heart[8]; memset(heart, 0, sizeof heart);
    for(int i=0;i<n;i++){
        int c = colors[i];
        if(c >= 0 && c < 8) heart[c] += is_negative ? -counts[i] : counts[i];
    }
    push_temporary_effect(g, card_id, dur, 0, heart);
}

static int card_appeared_this_turn(const GameState *g, int cid){
    for(int i=0;i<g->n_cards_appeared_this_turn;i++) if(g->cards_appeared_this_turn[i]==cid) return 1;
    return 0;
}
static int is_selected_card(const GameState *g, int cid){
    for(int i=0;i<g->n_selected_cards;i++) if(g->selected_cards[i]==cid) return 1;
    return 0;
}

/* util::matching_ids_filtered over the target's stage: the effect's filter_subset
   (card_type / group_names / characters / position / timing_condition /
   exclude_self) plus the already-selected exclusion used by target_count/distinct. */
#define RB_MISC_MAX_TARGETS (RB_STAGE_SIZE + RB_MAX_RECENTLY_MOVED)
static int collect_stage_candidates(const GameState *g, int who, const AbilityEffect *e,
                                    int exclude_self_id, int exclude_selected,
                                    int *out, int max){
    const RbPlayer *P = &g->p[who];
    const char *ctype  = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    const char *group  = eff_extra(e,"group_names");
    const char *chars  = eff_extra(e,"characters");
    const char *pos    = eff_extra(e,"position");
    const char *timing = eff_extra(e,"timing_condition");
    int pos_idx = pos ? rb_stage_position_index(pos) : -1;
    int n = 0;
    for(int q=0;q<RB_STAGE_SIZE && n<max;q++){
        int cid = P->stage[q];
        if(cid == RB_EMPTY_SLOT) continue;
        if(exclude_self_id >= 0 && cid == exclude_self_id) continue;
        if(pos_idx >= 0 && pos_idx != q) continue;
        if(ctype && !rb_card_matches_type(cid, ctype)) continue;
        if(group && !rb_card_matches_group_str(cid, group)) continue;
        if(chars){ const char *names[1] = { chars }; if(!rb_card_matches_characters(cid, names, 1)) continue; }
        if(timing){
            if(!strcmp(timing,"appeared_this_turn")){ if(!card_appeared_this_turn(g, cid)) continue; }
            else if(!strcmp(timing,"moved_this_turn")){
                if(cid >= RB_MAX_CARD_IDS || !g->moved_this_turn[cid]) continue;
            }
        }
        if(exclude_selected && is_selected_card(g, cid)) continue;
        out[n++] = cid;
    }
    return n;
}

/* Mirror misc.rs:calculate_gain_multiplier — per_unit effects multiply their base
   icon count by the number of matching units in the counted zone. */
static int calculate_gain_multiplier(const GameState *g, int who, const AbilityEffect *e,
                                     int per_unit, int base_count, const char *per_unit_type){
    if(!per_unit) return base_count;
    /* An explicit `location` overrides the generic per_unit_type → zone mapping. */
    const char *loc  = eff_extra(e,"location");
    const char *zone = loc ? loc : per_unit_type;
    int matching = 0;
    /* "つ" counts energy (Rust last_cost_energy_count; the C model tracks the
        active energy count only). */
    if(zone && !strcmp(zone,"つ")) matching = g->p[who].energy_active;
    else if(zone) matching = rb_count_in_zone(g, who, zone);
    /* per_unit_type=discard/waitroom: always the tracked move/cost counts, never
        the whole waitroom (util::resolve_discard_per_unit_count). */
    if(per_unit_type && (!strcmp(per_unit_type,"discard") || !strcmp(per_unit_type,"waitroom") ||
                         !strcmp(per_unit_type,"waitroom_card"))){
        matching = g->n_recently_moved > 0 ? g->n_recently_moved : g->mods.last_cost_discard_count;
    } else if(per_unit_type && !strcmp(per_unit_type,"energy_deck")){
        /* energy-deck placements: only the cards moved by THIS effect count. */
        matching = g->n_recently_moved;
    }
    int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : extra_int(e,"per_unit_count",1);
    if(per_unit_count <= 0) per_unit_count = 1;
    int units = matching / per_unit_count;
    /* `max` caps the unit count at `count` and pins the per-unit base to 1. */
    int is_max = extra_true(e,"max");
    if(is_max && e->count > 0 && units > e->count) units = e->count;
    /* max_repeats (aliased repeat_limit) is the sole cap on some per_unit effects. */
    int cap = e->repeat_limit > 0 ? e->repeat_limit : extra_int(e,"repeat_limit",0);
    if(cap > 0 && units > cap) units = cap;
    int per_unit_base = is_max ? 1 : extra_int(e,"resource_icon_count", e->count > 0 ? e->count : 1);
    if(per_unit_base < 0) per_unit_base = 1;
    return units * per_unit_base;
}

/* Mirror misc.rs:GainTargets — the target sets resolved for a gain_resource. */
typedef struct {
    int blade[RB_MISC_MAX_TARGETS]; int n_blade;
    int heart[RB_MISC_MAX_TARGETS]; int n_heart;
    int heart_color;    /* -1 = unspecified → HEART_ALL at apply time */
    int final_count;
} GainTargets;

/* Mirror misc.rs:resolve_gain_resource_targets — decide which stage members get
   the resource and in what amount. */
static void resolve_gain_resource_targets(GameState *g, int who, const AbilityEffect *e,
        int kind, int count, int per_unit, const char *per_unit_type, int is_all,
        int is_self_target, int exclude_self_id, int activating, GainTargets *out){
    memset(out, 0, sizeof *out);
    out->heart_color = -1;
    out->final_count = calculate_gain_multiplier(g, who, e, per_unit, count, per_unit_type);

    int tc       = extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || extra_true(e,"distinct");
    int has_selection_filter = (tc >= 0) || distinct;
    int from_selection = extra_true(e,"target_from_selection");
    int multi = extra_true(e,"multiple_targets");
    int nsel = g->n_selected_cards;

    int cand[RB_MISC_MAX_TARGETS];
    int nc = collect_stage_candidates(g, who, e, exclude_self_id,
                                     has_selection_filter && nsel > 0,
                                     cand, RB_MISC_MAX_TARGETS);

    /* ── blade targets ── */
    if(from_selection){
        for(int i=0;i<nsel && out->n_blade<RB_MISC_MAX_TARGETS;i++) out->blade[out->n_blade++]=g->selected_cards[i];
    } else if(tc >= 0){
        /* Pure-selection shapes with no filter match inherit the preceding step's
            selection; otherwise the effect's own filter describes fresh targets. */
        if(nsel > 0 && nc == 0){
            for(int i=0;i<nsel && out->n_blade<tc;i++) out->blade[out->n_blade++]=g->selected_cards[i];
        } else {
            for(int i=0;i<nc && out->n_blade<tc;i++) out->blade[out->n_blade++]=cand[i];
        }
    } else if(nsel > 0 && !distinct){
        for(int i=0;i<nsel && out->n_blade<RB_MISC_MAX_TARGETS;i++) out->blade[out->n_blade++]=g->selected_cards[i];
    } else {
        for(int i=0;i<nc && out->n_blade<RB_MISC_MAX_TARGETS;i++) out->blade[out->n_blade++]=cand[i];
    }

    /* ── heart color: a preceding select stores it in queue.selected_heart_color
        (Rust conditional_choice = Str(color)); else the effect's own field. ── */
    if(g->queue.selected_heart_color >= 0) out->heart_color = g->queue.selected_heart_color;
    else { const char *hc = eff_extra(e,"heart_color"); if(hc) out->heart_color = (int)rb_parse_heart_color(hc); }

    /* ── heart targets ── */
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    const char *group = eff_extra(e,"group_names");
    const char *chars = eff_extra(e,"characters");
    if(kind == RB_RES_HEART && per_unit && per_unit_type && !strcmp(per_unit_type,"energy_deck") &&
       g->mods.n_last_under_move_host_ids > 0){
        /* 「そうした場合、そのメンバーは…」: a per-unit heart gain that follows an
            under_member→energy_deck move targets THE HOST MEMBER(S) of the moved
            energy, never the whole card_type-filter match set. */
        for(int i=0;i<g->mods.n_last_under_move_host_ids && out->n_heart<RB_MISC_MAX_TARGETS;i++)
            out->heart[out->n_heart++] = g->mods.last_under_move_host_ids[i];
    } else if(from_selection){
        for(int i=0;i<nsel && out->n_heart<RB_MISC_MAX_TARGETS;i++) out->heart[out->n_heart++]=g->selected_cards[i];
    } else if(nsel > 0 && !distinct && !has_selection_filter){
        for(int i=0;i<nsel && out->n_heart<RB_MISC_MAX_TARGETS;i++) out->heart[out->n_heart++]=g->selected_cards[i];
        if(multi && activating >= 0 && !is_selected_card(g, activating) &&
           out->n_heart < RB_MISC_MAX_TARGETS)
            out->heart[out->n_heart++] = activating;
    } else if(kind == RB_RES_HEART){
        if(!ctype && !group && !chars && tc < 0 && !distinct && !is_all){
            /* No targeting info: default to the activating card only, so heart
                never leaks to every stage member. */
            if(activating >= 0) out->heart[out->n_heart++] = activating;
        } else {
            int lim = (tc >= 0 && !is_self_target && tc < RB_MISC_MAX_TARGETS) ? tc : RB_MISC_MAX_TARGETS;
            for(int i=0;i<nc && out->n_heart<lim;i++) out->heart[out->n_heart++]=cand[i];
        }
    }

    /* heart_colors as a TARGET filter (targets must already possess the color). */
    if(extra_true(e,"filter_targets_by_heart_colors")){
        const char *hc = eff_extra(e,"heart_color");
        if(hc){
            const char *cols[1] = { hc };
            int keep = 0;
            for(int i=0;i<out->n_heart;i++){
                int cid = out->heart[i];
                int ok = extra_true(e,"require_all_heart_colors")
                       ? rb_card_matches_all_heart_colors(cid, cols, 1)
                       : rb_card_matches_heart_colors(cid, cols, 1);
                if(ok) out->heart[keep++] = cid;
            }
            out->n_heart = keep;
        }
    }
}

/* The effect that already emitted its target-selection choice. On re-entry (after
   the host answered) the guard clears and the grant proceeds against the
   truncated candidate list — mirrors Rust parking a target_count-cleared copy of
   the effect in the queue's pending actions. */
static const AbilityEffect *s_target_choice_effect;

/* Mirror misc.rs:try_create_target_selection_choice — when target_count is set and
   more members qualify than it allows, prompt the player. Returns 1 when a choice
   was created (the caller must stop processing this effect now). */
static int try_create_target_selection_choice(GameState *g, int actor, const AbilityEffect *e,
        int kind, int who, int is_self_target, int per_unit, int exclude_self_id){
    int tc = extra_int(e,"target_count",-1);
    int distinct = e->distinct_flag || extra_true(e,"distinct");
    if(tc < 0 || is_self_target || per_unit || kind == RB_RES_OTHER) return 0;
    if(!(g->n_selected_cards == 0 || distinct)) return 0;
    if(s_target_choice_effect == e){ s_target_choice_effect = NULL; return 0; }
    int cand[RB_MISC_MAX_TARGETS];
    int nc = collect_stage_candidates(g, who, e, exclude_self_id, g->n_selected_cards > 0,
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
    g->queue.resume_host = s_activating_card;
    s_target_choice_effect = e;
    return 1;
}

/* Mirror misc.rs:apply_blade_resource — apply blade modifiers to the resolved
   targets (and register the revert data for temporary durations). */
static void apply_blade_resource(GameState *g, const AbilityEffect *e, int kind, int who,
        const int *targets, int n_targets, int activating, int is_all, int dur,
        int final_count, int blades_to_add){
    if(kind != RB_RES_BLADE) return;
    const char *ctype  = e->card_type_field[0] ? e->card_type_field : eff_extra(e,"card_type");
    const char *group  = eff_extra(e,"group_names");
    const char *chars  = eff_extra(e,"characters");
    const char *pos    = eff_extra(e,"position");
    const char *timing = eff_extra(e,"timing_condition");
    if(n_targets == 0){
        if(is_all && !group && !ctype && !chars && !timing && !pos){
            RbPlayer *P = &g->p[who];
            for(int q=0;q<RB_STAGE_SIZE;q++){
                int cid = P->stage[q];
                if(cid == RB_EMPTY_SLOT) continue;
                grant_blade(g, cid, blades_to_add);
                push_temporary_effect(g, cid, dur, blades_to_add, NULL);
            }
        } else if(pos){
            int idx = rb_stage_position_index(pos);
            int cid = (idx >= 0) ? g->p[who].stage[idx] : RB_EMPTY_SLOT;
            if(cid != RB_EMPTY_SLOT){
                grant_blade(g, cid, blades_to_add);
                push_temporary_effect(g, cid, dur, blades_to_add, NULL);
            }
        } else if(extra_int(e,"target_count",-1) < 0 && !extra_true(e,"exclude_self")){
            /* Fallback-to-self is for plain "this member gains N" shapes only;
                「ほかのメンバーは…」 must never land on the activating card. */
            grant_blade(g, activating, blades_to_add);
            push_temporary_effect(g, activating, dur, blades_to_add, NULL);
        }
        return;
    }
    /* Pure sequential select→gain_resource applies to ALL selected cards. */
    int lim = n_targets;
    if(!(g->n_selected_cards > 0 && !e->source) && !is_all && final_count < n_targets)
        lim = final_count;
    for(int i=0;i<lim;i++){
        grant_blade(g, targets[i], blades_to_add);
        push_temporary_effect(g, targets[i], dur, blades_to_add, NULL);
    }
}

/* Mirror misc.rs:apply_heart_resource — apply heart modifiers to the resolved
   targets (and register the revert data for temporary durations). */
static void apply_heart_resource(GameState *g, const AbilityEffect *e, int kind, int who,
        const int *targets, int n_targets, int activating, int is_self_target, int is_all,
        int dur, int is_negative, const int *colors, const int *counts, int n_dist,
        int final_count){
    if(kind != RB_RES_HEART) return;
    const char *pos = eff_extra(e,"position");
    int tc = extra_int(e,"target_count",-1);
    if(n_targets == 0){
        if(pos){
            int idx = rb_stage_position_index(pos);
            int cid = (idx >= 0) ? g->p[who].stage[idx] : RB_EMPTY_SLOT;
            if(cid != RB_EMPTY_SLOT) apply_heart_to_card(g, cid, colors, counts, n_dist, is_negative, dur);
        } else if(tc < 0 && (!eff_extra(e,"exclude_self") ||
                             (e->target && !strcmp(e->target,"self")))){
            apply_heart_to_card(g, activating, colors, counts, n_dist, is_negative, dur);
        }
        return;
    }
    if(is_self_target){
        apply_heart_to_card(g, activating, colors, counts, n_dist, is_negative, dur);
        return;
    }
    int lim = (is_all || extra_true(e,"multiple_targets")) ? n_targets
            : (final_count < n_targets ? final_count : n_targets);
    for(int i=0;i<lim;i++)
        apply_heart_to_card(g, targets[i], colors, counts, n_dist, is_negative, dur);
}

/* Mirror misc.rs:gain_heart_colors_from_selected_card — gain 1 heart of each color
   that the previously-selected card has (its base_heart), for every matching member
   on the target's stage. */
static int h_gain_heart_colors_from_selected_card(GameState *g, int actor, const AbilityEffect *e) {
    int who = misc_target_player(actor, e);
    RbPlayer *P = &g->p[who];
    const char *chars = eff_extra(e, "characters");
    int dur = duration_is_temporary(e) ? effect_duration(e) : RB_TEMP_PERM;
    if (g->n_selected_cards == 0) return 1;
    int selected = g->selected_cards[0];
    Card sc;
    if (!rb_decode_card_by_index((uint32_t)selected, &sc)) return 1;
    for (int h = 0; h < sc.num_base && h < sc.n_hearts; h++) {
        int color = sc.heart_color[h];
        if (color < 0 || color > 7) continue;
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            int tid = P->stage[q];
            if (tid == RB_EMPTY_SLOT) continue;
            if (chars) { const char *names[1] = { chars };
                         if (!rb_card_matches_characters(tid, names, 1)) continue; }
            int c1[1] = { color }, n1[1] = { 1 };
            apply_heart_to_card(g, tid, c1, n1, 1, 0, dur);
        }
    }
    rb_free_card(&sc);
    return 1;
}

/* Mirror misc.rs:gain_heart_all_type — heart_type="all" grants `count` all-color
   hearts to the triggering member (C: the activating card), the member at the
   effect's position, or the target's first staged member. */
static int h_gain_heart_all_type(GameState *g, int actor, const AbilityEffect *e) {
    int who = misc_target_player(actor, e);
    RbPlayer *P = &g->p[who];
    const char *pos = eff_extra(e, "position");
    int card_id = -1;
    if (pos) {
        int idx = rb_stage_position_index(pos);
        if (idx >= 0 && P->stage[idx] != RB_EMPTY_SLOT) card_id = P->stage[idx];
    } else if (e->target && *e->target) {
        for (int q = 0; q < RB_STAGE_SIZE; q++)
            if (P->stage[q] != RB_EMPTY_SLOT) { card_id = P->stage[q]; break; }
    }
    if (card_id < 0) card_id = s_activating_card;
    if (card_id < 0) return 1;
    int amount = e->count > 0 ? e->count : 1;
    int c1[1] = { RB_HEART_ALL }, n1[1] = { amount };
    apply_heart_to_card(g, card_id, c1, n1, 1, 0,
                        duration_is_temporary(e) ? effect_duration(e) : RB_TEMP_PERM);
    return 1;
}

/* Mirror misc.rs:handle_bp6_pattern — "gain 1 heart of each distinct color among
    discarded cards". Detected by: resource=heart + per_unit + per_unit_type=discard
    + multiple_targets. For every distinct base_heart color present in the recently
    moved (discarded) cards, grant 1 heart of that color to the activating card. */
static int h_bp6_pattern(GameState *g, int actor, const AbilityEffect *e) {
    (void)actor;
    const char *res = eff_extra(e, "resource");
    int per_unit = (e->per_unit > 0) || extra_true(e, "per_unit");
    const char *put = eff_extra(e, "per_unit_type");
    int multi = extra_true(e, "multiple_targets");
    if (!(res && !strcmp(res, "heart") && per_unit && put && !strcmp(put, "discard") && multi))
        return 0;
    int activating = s_activating_card;
    if (activating < 0) return 1;
    /* Collect the distinct base_heart colors among the recently moved cards. */
    int distinct[8]; int nd = 0;
    for (int i = 0; i < g->n_recently_moved; i++) {
        int cid = g->recently_moved[i];
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        for (int h = 0; h < c.n_hearts && h < c.num_base; h++) {
            int col = c.heart_color[h];
            if (col < 0 || col > 7) continue;
            int seen = 0;
            for (int j = 0; j < nd; j++) if (distinct[j] == col) { seen = 1; break; }
            if (!seen && nd < 8) distinct[nd++] = col;
        }
        rb_free_card(&c);
    }
    int dur = duration_is_temporary(e) ? effect_duration(e) : RB_TEMP_PERM;
    for (int i = 0; i < nd; i++) {
        int amt = 1;
        apply_heart_to_card(g, activating, &distinct[i], &amt, 1, 0, dur);
    }
    return 1;
}

static int h_gain_resource(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs:execute_gain_resource — special heart shapes first, then the
       normalized resource kind drives target resolution + blade/heart application.
       Energy has no per-card modifier in the C model, so it advances the zone size
       AND the active count together (capped at RB_ENERGY_CAP). */
    const char *res = eff_extra(e, "resource");
    if (res && !strcmp(res, "heart") && extra_true(e, "heart_colors_from_selected_card"))
        return h_gain_heart_colors_from_selected_card(g, actor, e);
    if (res && !strcmp(res, "heart") && rb_is_all_heart_type(e))
        return h_gain_heart_all_type(g, actor, e);
    if (h_bp6_pattern(g, actor, e)) return 1;
    if (res && !strcmp(res, "surplus_heart")) { rb_effect_gain_surplus_heart(g, actor, e); return 1; }

    int kind = resource_kind(res);
    int who  = misc_target_player(actor, e);
    RbPlayer *P = &g->p[who];
    if (kind == RB_RES_OTHER) {
        int n = e->count > 0 ? e->count : 1;
        P->energy.n += n;
        if (P->energy.n > RB_ENERGY_CAP) P->energy.n = RB_ENERGY_CAP;
        P->energy_active += n;
        if (P->energy_active > RB_ENERGY_CAP) P->energy_active = RB_ENERGY_CAP;
        return 1;
    }

    int count = extra_int(e, "resource_icon_count", e->count > 0 ? e->count : 1);
    int per_unit = (e->per_unit > 0) || extra_true(e, "per_unit");
    const char *per_unit_type = eff_extra(e, "per_unit_type");
    int is_self_target = e->self_target_field[0] && !strcmp(e->self_target_field, "true");
    int activating = s_activating_card;
    int exclude_self_id = extra_true(e, "exclude_self") ? activating : -1;
    const char *sign = eff_extra(e, "sign");
    int is_negative = sign && !strcmp(sign, "negative");
    int dur = duration_is_temporary(e) ? effect_duration(e) : RB_TEMP_PERM;
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e, "card_type");
    int tc = extra_int(e, "target_count", -1);
    int distinct = e->distinct_flag || extra_true(e, "distinct");
    int player_target = e->target && (!strcmp(e->target, "self") || !strcmp(e->target, "opponent"));
    int is_member_ct = ctype && !strcmp(ctype, "member_card");
    /* is_all: explicit `all`, or the unbounded "自分のステージにいるメンバーは" shapes. */
    int is_all = extra_true(e, "all")
              || (!e->source && is_member_ct && player_target && !is_self_target &&
                  !eff_extra(e, "exclude_self") && tc < 0)
              || (is_member_ct && player_target && tc < 0 && !distinct);

    if (try_create_target_selection_choice(g, actor, e, kind, who, is_self_target,
                                          per_unit, exclude_self_id))
        return 1;

    GainTargets t;
    resolve_gain_resource_targets(g, who, e, kind, count, per_unit, per_unit_type,
                                 is_all, is_self_target, exclude_self_id, activating, &t);

    /* Store the picked ids when target_count/distinct is set so the next sequential
       action can exclude them (mirrors self.selected_cards bookkeeping). */
    if (tc >= 0 || distinct) {
        const int *picked = (kind == RB_RES_BLADE) ? t.blade : t.heart;
        int n_picked = (kind == RB_RES_BLADE) ? t.n_blade : t.n_heart;
        for (int i = 0; i < n_picked; i++)
            if (!is_selected_card(g, picked[i]) && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                g->selected_cards[g->n_selected_cards++] = picked[i];
    }

    int final_count = t.final_count;
    int blades_to_add = is_negative ? -final_count : final_count;
    /* Heart distribution. Rust splits `count` across every listed color; the C
       decoder keeps only the first heart_colors entry, so the distribution has a
       single entry (heart_gain_per_entry of 1 color == count). */
    int colors[1] = { t.heart_color >= 0 ? t.heart_color : RB_HEART_ALL };
    int counts[1] = { final_count };

    if (is_self_target && activating >= 0) {
        int on_stage = 0;
        for (int q = 0; q < RB_STAGE_SIZE; q++) if (P->stage[q] == activating) on_stage = 1;
        if (!on_stage) return 0;  /* Rust: Err("activating card not on stage") */
        if (kind == RB_RES_BLADE) {
            grant_blade(g, activating, blades_to_add);
            push_temporary_effect(g, activating, dur, blades_to_add, NULL);
        } else {
            apply_heart_to_card(g, activating, colors, counts, 1, is_negative, dur);
        }
        g->queue.selected_heart_color = -1;   /* consumed by this grant */
        rule_log_activated(g, activating, "[[log_gain_resource]]");
        return 1;
    }

    apply_blade_resource(g, e, kind, who, t.blade, t.n_blade, activating, is_all, dur,
                          final_count, blades_to_add);

    /* group_reference="same_group_name": restrict heart targets to members whose
        group matches the cost-discarded card's group (mirrors misc.rs:1199). */
    if (extra_true(e, "group_reference")) {
        const char *gref = eff_extra(e, "group_reference");
        if (gref && !strcmp(gref, "same_group_name") && g->n_recently_moved > 0) {
            char ref_group[64]; ref_group[0] = 0;
            Card rc;
            if (rb_decode_card_by_index((uint32_t)g->recently_moved[0], &rc)) {
                const char *gs = rc.group_idx ? rb_card_string(rc.group_idx) : NULL;
                if (gs) strncpy(ref_group, gs, sizeof ref_group - 1);
                rb_free_card(&rc);
            }
            if (ref_group[0]) {
                int keep = 0;
                for (int i = 0; i < t.n_heart; i++)
                    if (rb_card_matches_group_str(t.heart[i], ref_group))
                        t.heart[keep++] = t.heart[i];
                t.n_heart = keep;
            }
        }
    }

    apply_heart_resource(g, e, kind, who, t.heart, t.n_heart, activating, is_self_target,
                         is_all, dur, is_negative, colors, counts, 1, final_count);
    g->queue.selected_heart_color = -1;       /* consumed by this grant */
    if (dur == RB_TEMP_PERM) rb_recalc_constants(g);
    rule_log_activated(g, activating, "[[log_gain_resource]]");
    return 1;
}
static int h_pay_energy(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs pay_energy — spend `count` active energy. */
    int n = e->count > 0 ? e->count : 1;
    RbPlayer *P = &g->p[actor];
    P->energy_active -= n;
    if (P->energy_active < 0) P->energy_active = 0;
    return 1;
}
/* Mirror cost.rs::handle_pay_cost_all_discard — the "may discard your whole hand"
    cost: move every card in the target player's hand into the waitroom (C's discard
    pile). Cost moves are player actions, not effects, so we do NOT mark them in
    recently_moved / moved_this_turn (mirrors Rust push_movement_event(...false)). */
int rb_effect_pay_cost_all_discard(GameState *g, int actor, const AbilityEffect *e) {
    int who = actor;
    if (e->target && (!strcmp(e->target, "opponent") || !strcmp(e->target, "p2")))
        who = actor ^ 1;
    RbPlayer *P = &g->p[who];
    while (P->hand.n > 0) {
        int cid = P->hand.cards[--P->hand.n];
        rb_waitroom_add(P, cid);
    }
    return 1;
}
static int h_discard_until_count(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs discard_until_count — discard from hand until hand
       size reaches `count`. */
    int target = e->count > 0 ? e->count : 0;
    RbPlayer *P = &g->p[actor];
    while (P->hand.n > target && P->hand.n > 0) {
        int card = P->hand.cards[--P->hand.n]; /* drop from end */
        if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card;
    }
    return 1;
}
static int h_restriction(GameState *g, int actor, const AbilityEffect *e) {
    const char *rtype = eff_extra(e, "restriction_type");
    const char *rdest = eff_extra(e, "restricted_destination");
    if(!rtype) rtype = eff_extra(e, "type");
    if(!rdest && e->destination) rdest = e->destination;
    int delayed = 0;
    const char *dstr = eff_extra(e, "delayed");
    if(dstr && (!strcmp(dstr,"true")||!strcmp(dstr,"1"))) delayed = 1;

    /* Record the prohibition note (mirrors gs.prohibition_effects). */
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

    /* cannot_activate / cannot_active → block ability activation. */
    int is_cannot = rtype && (!strcmp(rtype,"cannot_activate_by_effect") ||
                             !strcmp(rtype,"cannot_active") || !strcmp(rtype,"cannot_activate"));
    if(is_cannot){
        int tgt = actor;
        if(e->target && !strcmp(e->target,"opponent")) tgt = actor^1;
        if(delayed){
            /* Key the ban on the cards this ability just moved, else the target
               player's staged members (next-turn-only activation lockout). */
            for(int i=0;i<g->n_recently_moved && g->n_cannot_active_cards<RB_MAX_ZONE;i++)
                g->cannot_active_cards[g->n_cannot_active_cards++]=g->recently_moved[i];
            for(int q=0;q<RB_STAGE_SIZE;q++)
                if(g->p[tgt].stage[q]>=0 && g->n_cannot_active_cards<RB_MAX_ZONE)
                    g->cannot_active_cards[g->n_cannot_active_cards++]=g->p[tgt].stage[q];
        } else {
            g->player_cannot_activate[tgt] = 1;
        }
    }
    return 1;
}
static int h_choice(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs / ability/choice.rs `execute_choice` — present a choice to
       the host (headless auto-skips via the resolver). Emit a SELECT_TARGET
       pending choice with the effect's count as the option count and its
       is_optional flag as the allow-skip bit, so the resume path mirrors the
       dedicated "choice" verb in engine.c. */
    if (g->queue.resume_active) return 1;   /* already resolving; don't re-emit */
    int cnt = e->count >= 0 ? e->count : 1;
    int allow = e->is_optional ? 1 : 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, cnt, allow, "choice");
    g->queue.resume_mode = 0;
    return 1;
}
static int h_position_change(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs position_change — move a member from a source area to a
       destination area on the actor's stage. Source defaults to center,
       destination from e->destination (else e->target). */
    RbPlayer *P = &g->p[actor];
    int src = 1, dst = 1;
    if (e->source && *e->source) src = rb_pos_to_area(e->source);
    const char *dest = e->destination && *e->destination ? e->destination : e->target;
    if (dest && *dest) dst = rb_pos_to_area(dest);
    if (src < 0 || src >= RB_STAGE_SIZE) src = 1;
    if (dst < 0 || dst >= RB_STAGE_SIZE) dst = 1;
    if (src == dst) return 1;
    if (P->stage[src] < 0) return 0;        /* nothing to move */
    if (P->stage[dst] >= 0) return 0;        /* destination occupied */
    int card = P->stage[src];
    P->stage[src] = -1; P->stage_wait[src] = 0;
    P->stage[dst] = card; P->stage_wait[dst] = 0;
    return 1;
}
static int h_rotation(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs rotation — flip the orientation (active<->wait) of the
       targeted member. Target area defaults to center. */
    RbPlayer *P = &g->p[actor];
    int area = 1; /* center */
    if (e->target && *e->target) area = rb_pos_to_area(e->target);
    if (area < 0 || area >= RB_STAGE_SIZE) area = 1;
    if (P->stage[area] >= 0) { P->stage_wait[area] = !P->stage_wait[area]; return 1; }
    return 0;
}
static int h_place_energy_under_member(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs place_energy_under_member — tuck `count` energy cards
       under a stage member (under_cards[area]). They leave the energy zone. */
    RbPlayer *P = &g->p[actor];
    int area = 1; /* center */
    const char *dest = e->destination && *e->destination ? e->destination : e->target;
    if (dest && *dest) area = rb_pos_to_area(dest);
    if (area < 0 || area >= RB_STAGE_SIZE) area = 1;
    if (P->stage[area] < 0) return 0; /* no member to tuck under */
    int n = e->count > 0 ? e->count : 1;
    int moved = 0;
    while (moved < n && P->energy.n > 0) {
        int cid = P->energy.cards[--P->energy.n];
        if (P->under_cards[area].n < RB_MAX_ZONE)
            P->under_cards[area].cards[P->under_cards[area].n++] = cid;
        moved++;
    }
    return 1;
}
static int h_play_baton_touch(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs::execute_play_baton_touch — baton-touch redirect gate.
        Faithful: once a baton touch has been performed during the current
        play action it is a no-op; otherwise (single) record the allowance as
        a prohibition note, or (double, count>1) emit a pair-selection choice. */
    int count = e->count >= 1 ? e->count : 1;
    int who = actor;
    if (e->target && (!strcmp(e->target, "opponent") || !strcmp(e->target, "p2"))) who = actor ^ 1;
    int bt_count = (who == 0) ? g->baton_touch_count_p1 : g->baton_touch_count_p2;
    if (bt_count > 0) return 1;            /* already done this play action */

    if (count > 1) {
        /* Double baton: choose 2 occupied stage areas, excluding members deployed
            this turn (baton arrival-ban, Rule 9.6.2.1.2.1; stage_arrived set on
            deploy in engine.c:812). The resume path decodes the selected pair index. */
        if (g->queue.resume_active) return 1;   /* already resolving */
        RbPlayer *P = &g->p[who];
        int occupied[RB_STAGE_SIZE]; int no = 0;
        /* Rust filters out members deployed this turn (baton arrival-ban, Rule
            9.6.2.1.2.1); stage_arrived is set on deploy in engine.c:812. */
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT && !g->stage_arrived[who][i]) occupied[no++] = i;
        if (no < 2) return 1;          /* not enough occupied positions */
        int pairs = no * (no - 1) / 2;
        rb_emit_choice(g, who, RB_CHOICE_SELECT_TARGET, NULL, NULL, pairs, 1, "double_baton_touch");
        g->queue.resume_mode = 1;
        g->queue.resume_eff = (AbilityEffect *)e;
        g->queue.resume_actor = who;
        g->queue.resume_host = -1;
        return 1;
    }
    /* Single baton: record "baton_touch_allowed:<count>" (mirrors
        gs.prohibition_effects.push) so downstream play is permitted. */
    if (g->n_prohibition < 64) {
        char *b = g->prohibition[g->n_prohibition]; int bi = 0;
        const char *s = "baton_touch_allowed:";
        for (const char *p = s; *p && bi < 46; ) b[bi++] = *p++;
        if (count >= 10) b[bi++] = (char)('0' + (count / 10));
        b[bi++] = (char)('0' + (count % 10));
        b[bi] = 0; g->n_prohibition++;
    }
    return 1;
}
static int h_re_yell(GameState *g, int actor, const AbilityEffect *e) {
    (void)actor; (void)e;
    /* Mirror misc.rs re_yell — re-run the live yell pool. Signals live.c's
       two-pass rebuild (g->re_yell_occurred) so hearts harvested by
       perform_yell are re-applied to the success check. */
    g->re_yell_occurred = 1;
    return 1;
}
static int h_perform_yell(GameState *g, int actor, const AbilityEffect *e) {
    (void)e;
    /* Mirror misc.rs perform_yell — finalize the current yell, harvesting the
       yelled member's blade into the live pool. The yelled cards are the
       actor's currently-staged live cards; sum their effective blade into the
       re_yell harvest that live.c's two-pass rebuild re-applies. */
    RbPlayer *P = &g->p[actor];
    for (int i = 0; i < P->live.n; i++) {
        Card c; if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) {
            int blade = (int)c.blade + rb_mods_get_blade(&g->mods, P->live.cards[i]);
            if (blade > 0) g->re_yell_blade_hearts[RB_HEART_PINK] += blade;
            rb_free_card(&c);
        }
    }
    g->re_yell_occurred = 1;
    return 1;
}
/* Mirror misc.rs:execute_gain_surplus_heart — capture this player's live surplus
   (total_hearts − total_required) so it can be granted/lost as a resource. The
   Rust path computes it from the latest performance snapshot; the C snapshot's
   `surplus_hearts` already holds total_pool − total_required. */
void rb_effect_gain_surplus_heart(GameState *g, int actor, const AbilityEffect *e) {
    if (!g || !e) return;
    int pl = actor;
    if (e->target && (!strcmp(e->target, "opponent") || !strcmp(e->target, "p2"))) pl = actor ^ 1;

    /* Most recent snapshot for this player (mirrors performance_snapshots.find). */
    int surplus = 0;
    for (int i = g->n_snapshots - 1; i >= 0; i--) {
        if (g->snapshots[i].player == pl) {
            int s = g->snapshots[i].surplus_hearts;
            surplus = (s >= 0) ? s : 0;
            break;
        }
    }
    g->last_surplus_loss_count[pl] = surplus;
    /* Rust resets self/opponent_live_surplus_count when sign==negative && is_all;
       the C engine has no separate per-player live surplus counter, so the value
       is captured above and reused by subsequent gain_resource/score effects. */
}

static int h_shuffle(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs shuffle — Fisher-Yates shuffle of the named zone
       (default: deck). */
    (void)actor;
    const char *zone = e->target && *e->target ? e->target : "deck";
    RbBag *b = NULL;
    if (!strcmp(zone, "hand")) b = &g->p[actor].hand;
    else if (!strcmp(zone, "deck")) b = &g->p[actor].deck;
    else if (!strcmp(zone, "energy")) b = &g->p[actor].energy;
    else if (!strcmp(zone, "discard")) b = &g->p[actor].discard;
    if (!b || b->n < 2) return 1;
    for (int i = b->n - 1; i > 0; i--) {
        int j = rand() % (i + 1);
        int t = b->cards[i]; b->cards[i] = b->cards[j]; b->cards[j] = t;
    }
    return 1;
}

/* Mirror misc.rs:execute_reveal_effect — reveal `count` cards from the top of the
   player's deck into the revealed pool (g->revealed_cards / g->n_revealed). */
static int h_reveal(GameState *g, int actor, const AbilityEffect *e) {
    int n = e->count > 0 ? e->count : 1;
    RbPlayer *P = &g->p[actor];
    for (int i = 0; i < n && P->deck.n > 0 && g->n_revealed < RB_MAX_RECENTLY_MOVED; i++) {
        int cid = P->deck.cards[--P->deck.n];
        g->revealed_cards[g->n_revealed++] = cid;
    }
    return 1;
}

/* Mirror misc.rs:execute_reveal_until_chosen_card — reveal from the deck until a
   card matching `card_type`/`group_names`/`characters` is found (or the deck runs
   out), leaving the revealed pool populated for a later selection. */
static int h_reveal_until_chosen_card(GameState *g, int actor, const AbilityEffect *e) {
    RbPlayer *P = &g->p[actor];
    const char *ctype = e->card_type_field[0] ? e->card_type_field : eff_extra(e, "card_type");
    const char *group = eff_extra(e, "group_names");
    const char *chars = eff_extra(e, "characters");
    int limit = e->count > 0 ? e->count : RB_MAX_RECENTLY_MOVED;
    int revealed = 0;
    while (P->deck.n > 0 && revealed < limit && g->n_revealed < RB_MAX_RECENTLY_MOVED) {
        int cid = P->deck.cards[--P->deck.n];
        g->revealed_cards[g->n_revealed++] = cid;
        revealed++;
        int ok = 1;
        if (ctype && !rb_card_matches_type(cid, ctype)) ok = 0;
        if (ok && group && !rb_card_matches_group_str(cid, group)) ok = 0;
        if (ok && chars) { const char *names[1] = { chars }; if (!rb_card_matches_characters(cid, names, 1)) ok = 0; }
        if (ok) break; /* found the chosen card */
    }
    return 1;
}

/* Mirror misc.rs:execute_activation_restriction — activation lockout keyed by the
   restriction_type, mirroring h_restriction's cannot_activate path. */
static int h_activation_restriction(GameState *g, int actor, const AbilityEffect *e) {
    return h_restriction(g, actor, e);
}

/* Mirror misc.rs:execute_choose_required_hearts — prompt the host to choose the
   required-heart colors (conditional heart choice). The selection is recorded in
   queue.selected_heart_color to be consumed by a subsequent gain_resource. */
static int h_choose_required_hearts(GameState *g, int actor, const AbilityEffect *e) {
    if (g->queue.resume_active) return 1;
    int cnt = e->count >= 1 ? e->count : 1;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, cnt, e->is_optional ? 1 : 0,
                   "choose_required_hearts");
    g->queue.resume_mode = 0;
    g->queue.resume_host = s_activating_card;
    g->queue.resume_actor = actor;
    return 1;
}

/* Mirror misc.rs:execute_choose_target_player — let the host pick self/opponent,
   mirroring engine.c's "choose_target_player" verb. */
static int h_choose_target_player(GameState *g, int actor, const AbilityEffect *e) {
    if (g->queue.resume_active) return 1;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, e->is_optional ? 1 : 0,
                   "self_or_opponent");
    g->queue.resume_mode = 0;
    g->queue.resume_host = s_activating_card;
    g->queue.resume_actor = actor;
    return 1;
}

/* Mirror misc.rs:execute_custom — engine-specific hook. Unrecognized custom
    types are permissive no-ops (matching Rust's behaviour); we still record the
    custom type for traceability instead of discarding the parameters. */
static int h_custom(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor;
    const char *ct = eff_extra(e, "custom_type");
    if (ct) rb_log_push_verdict(ct, "custom", 1);
    return 1;
}

/* Same dispatch as rb_execute_misc_effect but threads the resolving card id
   (Rust `activating_card`) into s_activating_card so per-card handlers resolve. */
int rb_execute_misc_effect_ex(GameState *g, int actor, const RbPlayer *self,
                              const AbilityEffect *e, int host_cid, int *resolved) {
    s_activating_card = host_cid;
    int r = rb_execute_misc_effect(g, actor, self, e, resolved);
    s_activating_card = -1;
    return r;
}

/* Mirror misc.rs::handle_both_targets — run a target="both" effect for self then
   opponent. Returns 1 when the effect was fully handled ("both" target), else 0. */
int rb_misc_handle_both_targets(GameState *g, int actor, const AbilityEffect *e) {
    if (!e || !e->target || strcmp(e->target, "both")) return 0;
    s_activating_card = -1;
    rb_execute_misc_effect(g, actor,   &g->p[actor],   e, NULL);
    rb_execute_misc_effect(g, actor^1, &g->p[actor^1], e, NULL);
    return 1;
}

/* Mirror misc.rs::compute_valid_position_destinations — write the currently-empty
   stage area indices into out_areas. */
int rb_misc_position_destinations(const GameState *g, int actor, const AbilityEffect *e,
                                  int host_cid, const RbFormationSlot *plan, int n_plan,
                                  int *out_areas, int max) {
    (void)e; (void)host_cid; (void)plan; (void)n_plan;
    int n = 0;
    const RbPlayer *P = &g->p[actor];
    for (int q = 0; q < RB_STAGE_SIZE && n < max; q++)
        if (P->stage[q] == RB_EMPTY_SLOT) out_areas[n++] = q;
    return n;
}

/* Mirror misc.rs::finalize_formation_change — apply every planned move as one
   atomic stage permutation (members listed in `plan` move to dest_area; members
   not in the plan keep their current area). Returns the number of movers. */
int rb_misc_finalize_formation_change(GameState *g, int actor,
                                      const RbFormationSlot *plan, int n_plan) {
    RbPlayer *P = &g->p[actor];
    int new_stage[RB_STAGE_SIZE], new_wait[RB_STAGE_SIZE];
    for (int q = 0; q < RB_STAGE_SIZE; q++) { new_stage[q] = RB_EMPTY_SLOT; new_wait[q] = 0; }
    /* Keep members not explicitly re-planned at their current area. */
    for (int q = 0; q < RB_STAGE_SIZE; q++) {
        int cid = P->stage[q];
        if (cid == RB_EMPTY_SLOT) continue;
        int in_plan = 0;
        for (int i = 0; i < n_plan; i++) if (plan[i].member_id == cid) { in_plan = 1; break; }
        if (!in_plan) { new_stage[q] = cid; new_wait[q] = P->stage_wait[q]; }
    }
    int moved = 0;
    for (int i = 0; i < n_plan; i++) {
        int cid = plan[i].member_id, dst = plan[i].dest_area;
        if (dst < 0 || dst >= RB_STAGE_SIZE) continue;
        if (new_stage[dst] != cid) moved++;
        new_stage[dst] = cid; new_wait[dst] = P->stage_wait[dst];
        record_movement(g, cid);
    }
    for (int q = 0; q < RB_STAGE_SIZE; q++) { P->stage[q] = new_stage[q]; P->stage_wait[q] = new_wait[q]; }
    return moved;
}

/* Mirror misc.rs::execute_position_change_with_destination — move the effect's
   member (host_cid) to `destination`. "same_area" is a no-op; "front" mirrors the
   area per Rule 4.5.7 (opposite side of the stage). */
int rb_position_change_with_destination(GameState *g, int actor, const AbilityEffect *e,
                                        const char *destination, int host_cid) {
    (void)e;
    RbPlayer *P = &g->p[actor];
    if (!destination || !strcmp(destination, "same_area")) return 1;
    int src = -1;
    if (host_cid >= 0) for (int q = 0; q < RB_STAGE_SIZE; q++) if (P->stage[q] == host_cid) src = q;
    int dst;
    if (!strcmp(destination, "front")) dst = (src >= 0) ? (RB_STAGE_SIZE - 1 - src) : 1;
    else dst = rb_stage_position_index(destination);
    if (src < 0 || dst < 0 || dst >= RB_STAGE_SIZE || src == dst) return 1;
    if (P->stage[dst] != RB_EMPTY_SLOT) return 0; /* destination occupied */
    int cid = P->stage[src];
    P->stage[src] = RB_EMPTY_SLOT; P->stage_wait[src] = 0;
    P->stage[dst] = cid; P->stage_wait[dst] = 0;
    return 1;
}

/* Dispatch an "misc" effect by its name string. Returns 1 on handled. */
int rb_execute_misc_effect(GameState *g, int actor, const RbPlayer *self,
                           const AbilityEffect *e, int *resolved) {
    if (!e) return 0;
    const char *name = e->action;
    int r = 1;
    if (name) {
        if      (!strcmp(name, "gain_resource"))          r = h_gain_resource(g, actor, e);
        else if (!strcmp(name, "pay_energy"))             r = h_pay_energy(g, actor, e);
        else if (!strcmp(name, "discard_until_count"))     r = h_discard_until_count(g, actor, e);
        else if (!strcmp(name, "restriction"))            r = h_restriction(g, actor, e);
        else if (!strcmp(name, "choice"))                 r = h_choice(g, actor, e);
        else if (!strcmp(name, "position_change"))        r = h_position_change(g, actor, e);
        else if (!strcmp(name, "rotation"))               r = h_rotation(g, actor, e);
        else if (!strcmp(name, "place_energy_under_member")) r = h_place_energy_under_member(g, actor, e);
        else if (!strcmp(name, "play_baton_touch"))       r = h_play_baton_touch(g, actor, e);
        else if (!strcmp(name, "re_yell"))                r = h_re_yell(g, actor, e);
        else if (!strcmp(name, "perform_yell"))           r = h_perform_yell(g, actor, e);
        else if (!strcmp(name, "shuffle"))                r = h_shuffle(g, actor, e);
        else if (!strcmp(name, "reveal"))                 r = h_reveal(g, actor, e);
        else if (!strcmp(name, "reveal_until_chosen_card")) r = h_reveal_until_chosen_card(g, actor, e);
        else if (!strcmp(name, "activation_restriction")) r = h_activation_restriction(g, actor, e);
        else if (!strcmp(name, "choose_required_hearts")) r = h_choose_required_hearts(g, actor, e);
        else if (!strcmp(name, "choose_target_player"))   r = h_choose_target_player(g, actor, e);
        else if (!strcmp(name, "custom"))                 r = h_custom(g, actor, e);
        else if (!strcmp(name, "pay_cost_all:discard_all")) r = rb_effect_pay_cost_all_discard(g, actor, e);
        else r = 0; /* unknown misc effect */
    }
    (void)self;
    if (resolved) *resolved = r;
    return r;
}
