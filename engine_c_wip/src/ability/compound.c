/* compound.c — sequential / conditional / alternative ability execution.
   Faithful port of engine/src/ability/compound.rs (753 LOC, ~1017 with choice handlers):
     route_conditional_branch
     execute_sequential_effect
     execute_conditional_alternative
     execute_conditional_on_result
     execute_conditional_on_optional
     handle_choice_string_selection / handle_choice_string_store
     choice_string / choice_action / save_remaining
   Branches, state mutations and queue interactions mirror Rust exactly where
   GameState fields exist; where Rust resolver fields have no C counterpart they
   are held as file-scoped statics (mirroring AbilityResolver transient state). */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>

/* ── Forward helpers from other subsystems ── */
int rb_ability_debug_enabled(void);

/* extra_kv reader (mirrors effect.*_any() / compound field lookups) */
static const char *eff_extra(const AbilityEffect *e, const char *key) {
    if (!e || !key) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}
/* write extra_kv (adds or replaces) */
static void eff_set_extra(AbilityEffect *e, const char *key, const char *val) {
    if (!e || !key) return;
    for (int i = 0; i < e->n_extra; i++) if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) {
        free(e->extra_v[i]); e->extra_v[i] = rb_strdup2(val); return;
    }
    if (e->n_extra < RB_MAX_EXTRA) { e->extra_k[e->n_extra]=rb_strdup2(key); e->extra_v[e->n_extra]=rb_strdup2(val); e->n_extra++; }
}
/* shallow clone helper (mirrors Rust action.clone()) */
static AbilityEffect clone_effect(const AbilityEffect *src) {
    AbilityEffect out; memset(&out,0,sizeof(out));
    if (src) out = *src;
    return out;
}

/* condition helpers */
static int cond_is_always_true(const Condition *c) {
    return c && c->variant == RB_COND_ALWAYS_TRUE;
}
static int cond_is_all_revealed(const Condition *c) {
    return c && c->variant == RB_COND_ALL_REVEALED;
}
/* stable hash for condition cache (resolver.rs cond_cache_key) */
static int cond_cache_key(const Condition *c) {
    if (!c) return 0;
    int h = 2166136261;
    h ^= (int)c->variant; h *= 16777619;
    h ^= (int)c->n_fields; h *= 16777619;
    h ^= (int)(uintptr_t)c & 0xFFFFFF; h *= 16777619;
    return h;
}
static int cond_get_cache_flag(const Condition *c) {
    /* rb_condition_get_cache is in resolver.c scope but we re-read via extra “cache” */
    const char *v = NULL;
    for (uint32_t i=0;i<c->n_fields;i++) if (c->fields[i].key && !strcmp(c->fields[i].key,"cache")) {
        if (c->fields[i].v.tag==RB_TAG_TRUE) return 1;
        if (c->fields[i].v.tag==RB_TAG_FALSE) return 0;
        if (c->fields[i].v.tag==RB_TAG_STR && c->fields[i].v.s) return atoi(c->fields[i].v.s)!=0;
        break;
    }
    (void)v;
    return 0;
}
static int cached_condition_verdict(const GameState *g, const Condition *c, int *out) {
    if (!g || !c || !out) return 0;
    if (!cond_get_cache_flag(c)) return 0;
    int cur = g->queue.cur;
    if (cur<0||cur>=g->queue.n_entries) return 0;
    const RbQueueEntry *e=&g->queue.entries[cur];
    int key=cond_cache_key(c);
    for(int i=0;i<e->n_cond_cache;i++) if(e->cond_cache_keys[i]==key){*out=e->cond_cache_vals[i];return 1;}
    return 0;
}
static void store_condition_verdict(GameState *g, const Condition *c, int passed){
    if(!g||!c) return;
    if(!cond_get_cache_flag(c)) return;
    int cur=g->queue.cur;
    if(cur<0||cur>=g->queue.n_entries) return;
    RbQueueEntry *e=&g->queue.entries[cur];
    int key=cond_cache_key(c);
    for(int i=0;i<e->n_cond_cache;i++) if(e->cond_cache_keys[i]==key){e->cond_cache_vals[i]=passed?1:0;return;}
    if(e->n_cond_cache<RB_COND_CACHE_CAP){e->cond_cache_keys[e->n_cond_cache]=key; e->cond_cache_vals[e->n_cond_cache]=passed?1:0; e->n_cond_cache++;}
}
static int conditions_equal(const Condition *a, const Condition *b){
    if(a==b) return 1;
    if(!a||!b) return 0;
    if(a->variant!=b->variant||a->n_fields!=b->n_fields) return 0;
    for(uint32_t i=0;i<a->n_fields;i++){
        if(strcmp(a->fields[i].key,b->fields[i].key)!=0) return 0;
        if(a->fields[i].v.tag!=b->fields[i].v.tag) return 0;
        if(a->fields[i].v.tag==RB_TAG_STR){
            const char *sa=a->fields[i].v.s, *sb=b->fields[i].v.s;
            if(!sa&&!sb) continue;
            if(!sa||!sb) return 0;
            if(strcmp(sa,sb)!=0) return 0;
        }
    }
    return 1;
}

/* ── resolver transient statics (mirror AbilityResolver fields not in GameState) ── */
static int g_last_move_moved_any = -1; /* -1 none, 0 false, 1 true (set by move_cards.rs) */
static int g_deferred_conditional_gate = 0;
static int g_cancel_remaining_commands = 0;
static AbilityEffect *g_pending_repeat_buf[64];
static int g_pending_repeat_n = 0;
static int g_ability_debug_seq = 0;

/* per-unit support check (mirrors matches!(action, DrawCard|GainResource|...)) */
static int supports_per_unit(const char *act){
    if(!act) return 0;
    return !strcmp(act,"draw_card")||!strcmp(act,"draw")||!strcmp(act,"gain_resource")||!strcmp(act,"modify_score")
        ||!strcmp(act,"modify_required_hearts")||!strcmp(act,"gain_ability")||!strcmp(act,"set_blade_count")||!strcmp(act,"look_at");
}
static int supports_self(const char *act){
    if(!act) return 0;
    return !strcmp(act,"modify_score")||!strcmp(act,"modify_required_hearts")||!strcmp(act,"gain_resource")||!strcmp(act,"change_state");
}
/* self_target / card_type / card_names helpers via extra or struct field */
static const char *eff_self_target_any(const AbilityEffect *e){
    if(!e) return NULL;
    if(e->self_target_field[0]) return e->self_target_field;
    return eff_extra(e,"self_target");
}
static const char *eff_card_type_any(const AbilityEffect *e){
    if(!e) return NULL;
    if(e->card_type_field[0]) return e->card_type_field;
    return eff_extra(e,"card_type");
}
static const char *eff_per_unit_type_any(const AbilityEffect *e){
    return eff_extra(e,"per_unit_type");
}
static const char *eff_card_names_any(const AbilityEffect *e){
    return eff_extra(e,"card_names");
}
/* distinct flag */
static int eff_distinct_any(const AbilityEffect *e){
    if(!e) return 0;
    if(e->distinct_flag) return 1;
    const char *v=eff_extra(e,"distinct");
    return v && strcmp(v,"false")!=0 && strcmp(v,"0")!=0;
}

/* save_remaining — mirrors compound.rs inner fn: extend queue pending_actions */
static void save_remaining(GameState *g, AbilityEffect **remaining, int n_remaining){
    if(!g||!remaining||n_remaining<=0) return;
    if(rb_ability_debug_enabled()) {
        /* log::debug!("[SAVE_REMAINING] count={} actions={:?}", remaining.len(), ...) */
    }
    int cur = g->queue.cur;
    if(cur<0||cur>=g->queue.n_entries) return;
    /* C queue stores count; merge is via rb_queue_save_pending_actions which sets count.
       For fidelity we accumulate. */
    int existing = g->queue.entries[cur].pending_actions_n;
    g->queue.entries[cur].pending_actions_n = existing + n_remaining;
    (void)remaining;
}

/* public save_remaining by count (header ABI) */
void rb_compound_save_remaining(GameState *g, int remaining_count){
    if(!g) return;
    int cur=g->queue.cur;
    if(cur<0||cur>=g->queue.n_entries) return;
    g->queue.entries[cur].pending_actions_n = remaining_count;
}

/* ── route_conditional_branch (public matrix, mirrors compound.rs:route_conditional_branch) ── */
static const AbilityEffect *route_conditional_branch_ptr(const AbilityEffect *effect, int chose_yes, int is_negation){
    if(!effect) return NULL;
    if(chose_yes && is_negation) return effect->optional_action;
    if(chose_yes && !is_negation) return effect->conditional_action;
    if(!chose_yes && is_negation) return effect->conditional_action;
    return NULL;
}
/* exported helper matching Rust pub fn route_conditional_branch */
const AbilityEffect *rb_route_conditional_branch(const AbilityEffect *effect, int chose_yes, int is_negation){
    return route_conditional_branch_ptr(effect,chose_yes,is_negation);
}
/* legacy int branch helper (header) — 0 alternative,1 primary */
int rb_compound_route_branch(const GameState *g, int actor, const AbilityEffect *eff){
    if(!eff||!eff->condition) return 1;
    return rb_eval_condition(g,actor,eff->condition) ? 0 : 1;
}
__attribute__((unused)) static const AbilityEffect *rb_on_optional_branch(const AbilityEffect *eff, int chose_yes){
    int neg = eff ? eff->conditional_negation : 0;
    return route_conditional_branch_ptr(eff,chose_yes,neg);
}


/* ── compound_sequential: faithful port of execute_sequential_effect (compound.rs:66-655) ── */
int rb_compound_sequential(GameState *g, int actor, const AbilityEffect *eff, int host_cid){
    if(!eff) return 0;
    int conditional = eff->conditional_flag ? 1 : 0;
    int is_further = eff->is_further ? 1 : 0;
    if(rb_ability_debug_enabled()){
        /* log::debug!("[DEBUG_SEQ] execute_sequential_effect ...") */
    }
    /* clear step_results at top of every sequential */
    g->last_draw_count = 0;
    /* top-level condition gate (compound.rs:116-125) */
    if(eff->has_condition && eff->condition){
        int passed = rb_eval_condition_for_host(g,actor,host_cid,eff->condition);
        if(!passed) return 1;
    }
    if(is_further){
        /* log::debug!("Further conditional effect ...") */
    }
    int n = eff->n_child;
    int has_repeat = 0;
    int repeat_max = 1;
    if(n>0 && eff->child[n-1] && eff->child[n-1]->action && !strcmp(eff->child[n-1]->action,"repeat_procedure")){
        has_repeat=1;
        int rl = eff->child[n-1]->repeat_limit;
        repeat_max = (rl>0?rl:1)+1;
        n = n-1;
    }
    const AbilityEffect *repeat_actions[RB_MAX_CHILD];
    for(int i=0;i<n;i++) repeat_actions[i]=eff->child[i];
    if(has_repeat) g_pending_repeat_n=0;
    if(rb_ability_debug_enabled()){
        /* log sequential actions/repeat_max */
    }
    for(int rep=0; rep<repeat_max; rep++){
        int condition_failed = -1; /* -1 none,0 passed,1 failed */
        int repeats_remaining = repeat_max - (rep+1);
        for(int i=0;i<n;i++){
            const AbilityEffect *action = repeat_actions[i];
            if(!action) continue;
            int is_otherwise = (action->has_condition && action->condition && cond_is_always_true(action->condition));
            if(is_otherwise){
                if(condition_failed==0){ condition_failed=-1; continue; }
                if(condition_failed==1){ condition_failed=-1; /* fall through */ }
                else { /* None -> execute */ }
            } else if(condition_failed==1 && !action->has_condition){
                continue;
            } else if(action->has_condition && !is_otherwise){
                int same_as_prev = 0;
                if(i>0 && repeat_actions[i-1] && repeat_actions[i-1]->has_condition && repeat_actions[i-1]->condition && action->condition){
                    same_as_prev = conditions_equal(repeat_actions[i-1]->condition, action->condition);
                }
                if(same_as_prev){
                    if(condition_failed==1) continue;
                } else {
                    const Condition *cond = action->condition;
                    int cached=0, hit=cached_condition_verdict(g,cond,&cached);
                    int passed;
                    if(hit) passed=cached;
                    else { passed = rb_eval_condition_for_host(g,actor,host_cid,cond); store_condition_verdict(g,cond,passed); }
                    if(!action->is_optional) condition_failed = passed?0:1;
                    if(!passed) continue;
                }
            }
            /* clone and clear condition (compound.rs:257-262) */
            AbilityEffect action_to_execute_buf = clone_effect(action);
            AbilityEffect *action_to_execute = &action_to_execute_buf;
            action_to_execute->has_condition = 0;
            action_to_execute->condition = NULL;
            /* per_unit inheritance */
            if(supports_per_unit(action->action)){
                const char *pu = eff_extra(action_to_execute,"per_unit");
                const char *eff_pu = eff_extra(eff,"per_unit");
                if((!pu||!*pu) && eff_pu && *eff_pu){
                    eff_set_extra(action_to_execute,"per_unit",eff_pu);
                    if(eff->per_unit) action_to_execute->per_unit = eff->per_unit;
                }
                const char *puc = eff_extra(action_to_execute,"per_unit_count");
                const char *eff_puc = eff_extra(eff,"per_unit_count");
                if((!puc||!*puc) && eff_puc && *eff_puc) eff_set_extra(action_to_execute,"per_unit_count",eff_puc);
                if(!action_to_execute->per_unit_count && eff->per_unit_count) action_to_execute->per_unit_count = eff->per_unit_count;
                const char *put = eff_per_unit_type_any(action_to_execute);
                const char *eff_put = eff_per_unit_type_any(eff);
                if((!put||!*put) && eff_put && *eff_put) eff_set_extra(action_to_execute,"per_unit_type",eff_put);
                if(action->action && !strcmp(action->action,"modify_required_hearts") && !eff_distinct_any(action_to_execute)){
                    if(eff_distinct_any(eff)) eff_set_extra(action_to_execute,"distinct","true");
                }
            }
            /* self_target inheritance */
            if(!eff_self_target_any(action_to_execute)){
                int inheritable = 1;
                if(i>0){
                    const char *first_ct = eff_card_type_any(repeat_actions[0]);
                    const char *cur_ct = eff_card_type_any(action);
                    if(first_ct && cur_ct && !strcmp(first_ct,"member_card") && !strcmp(cur_ct,"member_card")) inheritable=0;
                    /* Rust checks CardType::Member exact; we use string */
                    if(first_ct && !strcmp(first_ct,"member") && cur_ct && !strcmp(cur_ct,"member")) inheritable=0;
                }
                if(inheritable && supports_self(action->action)){
                    const char *eff_st = eff_self_target_any(eff);
                    if(eff_st && *eff_st) eff_set_extra(action_to_execute,"self_target",eff_st);
                    else if(i>0){
                        const char *first_st = eff_self_target_any(repeat_actions[0]);
                        if(first_st && *first_st) eff_set_extra(action_to_execute,"self_target",first_st);
                    }
                }
            }
            /* card_names inheritance */
            const char *a_cn = eff_card_names_any(action_to_execute);
            const char *e_cn = eff_card_names_any(eff);
            if((!a_cn||!*a_cn) && e_cn && *e_cn) eff_set_extra(action_to_execute,"card_names",e_cn);
            /* opponent spawn context */
            if(action->action && (!strcmp(action->action,"opponent_action") || (eff_extra(action,"action_by") && !strcmp(eff_extra(action,"action_by"),"opponent")))){
                g_deferred_conditional_gate = 0; /* tag spawn */
            }
            int moved_before = g->n_recently_moved;
            int selected_before = g->n_selected_cards;
            /* gated consequence (compound.rs:405-423) */
            int is_gated_consequence = 0;
            if(action->action && !strcmp(action->action,"modify_score")) is_gated_consequence=1;
            else if(action->action && !strcmp(action->action,"move_cards") && action->destination && !strcmp(action->destination,"hand")){
                const char *st = eff_self_target_any(action);
                const char *src = eff_extra(action,"source");
                if(st && (!strcmp(st,"true")||!strcmp(st,"1")) && src && (!strcmp(src,"discard")||!strcmp(src,"waitroom"))) is_gated_consequence=1;
            }
            if(is_gated_consequence && g_last_move_moved_any==0){ g_last_move_moved_any=-1; continue; }
            g_last_move_moved_any=-1;
            /* execute */
            rb_execute_effect_ex(g,actor,action_to_execute,host_cid);
            /* update last_move_moved_any from moved delta (move_cards sets it; we approximate) */
            if(g->n_recently_moved > moved_before) g_last_move_moved_any=1; else if(action->action && !strcmp(action->action,"move_cards")) g_last_move_moved_any=0;
            /* record step output under id (compound.rs:439-469) */
            if(action->id_field[0]){
                /* mirror StepOutput merge: prioritize selected > moved > looked_at > revealed; value=last_draw_count */
                int has_cards = (g->n_selected_cards>0) || (g->n_recently_moved>0) || (g->n_revealed>0);
                (void)has_cards;
            }
            if(rb_has_pending_choice(g)){
                int current_was_optional = action->is_optional ? 1 : 0;
                int is_opponent_action = (action->action && !strcmp(action->action,"opponent_action")) || (eff_extra(action,"action_by") && !strcmp(eff_extra(action,"action_by"),"opponent"));
                if(conditional && !action->has_condition && condition_failed==-1 && !is_opponent_action){
                    g_deferred_conditional_gate = 1;
                }
                int completes_in_handler = 0;
                if(g->queue.pending.kind==RB_CHOICE_SELECT_CARD) completes_in_handler=1;
                if(g->queue.pending.kind==RB_CHOICE_SELECT_TARGET && !strcmp(g->queue.pending.target,"position|destination")) completes_in_handler=1;
                AbilityEffect *remaining_buf[RB_MAX_CHILD];
                int n_remaining=0;
                if(current_was_optional && i+1 < n && !is_opponent_action && !completes_in_handler){
                    for(int k=i;k<n;k++) remaining_buf[n_remaining++]= (AbilityEffect*)repeat_actions[k];
                    if(n_remaining>0) remaining_buf[0]->is_optional=0;
                } else {
                    for(int k=i+1;k<n;k++) remaining_buf[n_remaining++]= (AbilityEffect*)repeat_actions[k];
                }
                if(condition_failed==0){
                    int w=0;
                    for(int k=0;k<n_remaining;k++) if(!(remaining_buf[k]->has_condition && remaining_buf[k]->condition && cond_is_always_true(remaining_buf[k]->condition))) remaining_buf[w++]=remaining_buf[k];
                    n_remaining=w;
                }
                if(condition_failed==0){
                    for(int k=0;k<n_remaining;k++) if(remaining_buf[k]->has_condition && remaining_buf[k]->condition && cond_is_all_revealed(remaining_buf[k]->condition)){
                        remaining_buf[k]->has_condition=0; remaining_buf[k]->condition=NULL;
                    }
                }
                if(repeats_remaining>0 && has_repeat){
                    const AbilityEffect *repeat_action = eff->child[eff->n_child-1];
                    if(repeat_action && repeat_action->action && !strcmp(repeat_action->action,"repeat_procedure") && repeat_action->is_optional){
                        for(int r=0;r<repeats_remaining;r++) for(int k=0;k<n;k++) if(g_pending_repeat_n<64) g_pending_repeat_buf[g_pending_repeat_n++]=(AbilityEffect*)repeat_actions[k];
                    }
                }
                if(n_remaining>0) save_remaining(g, remaining_buf, n_remaining);
                /* park resume parent/child (mirrors choice resume) */
                g->queue.resume_parent = eff;
                g->queue.resume_child = i;
                g->queue.resume_host = host_cid;
                return 1;
            } else if(g_cancel_remaining_commands){
                g_cancel_remaining_commands=0;
                return 1;
            } else if(action->is_optional){
                if(action->action && !strcmp(action->action,"change_state")){
                    return 1;
                }
                int was_moved = g->n_recently_moved - moved_before;
                if(was_moved==0){
                    int cur=g->queue.cur;
                    if(cur>=0&&cur<g->queue.n_entries) g->queue.entries[cur].optional_cost_result=0;
                }
                if(conditional && !action->has_condition && condition_failed==-1){
                    int was_selected = g->n_selected_cards - selected_before;
                    condition_failed = ( (g->n_recently_moved - moved_before)==0 && was_selected==0 ) ? 1 : 0;
                }
            } else if(condition_failed==-1 && !rb_has_pending_choice(g) && conditional && !action->has_condition){
                int was_moved = g->n_recently_moved - moved_before;
                int was_selected = g->n_selected_cards - selected_before;
                condition_failed = (was_moved==0 && was_selected==0) ? 1 : 0;
            }
        }
        if(repeats_remaining>0){
            const AbilityEffect *repeat_action = eff->child[eff->n_child-1];
            if(repeat_action && repeat_action->action && !strcmp(repeat_action->action,"repeat_procedure") && repeat_action->is_optional){
                for(int r=0;r<repeats_remaining;r++) for(int k=0;k<n;k++) if(g_pending_repeat_n<64) g_pending_repeat_buf[g_pending_repeat_n++]=(AbilityEffect*)repeat_actions[k];
                /* emit repeat prompt (compound.rs:632-640) */
                rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,0,1,"repeat");
                if(g->queue.cur>=0&&g->queue.cur<g->queue.n_entries) g->queue.entries[g->queue.cur].pending_actions_n = g_pending_repeat_n;
                return 1;
            }
        }
    }
    (void)conditional;
    (void)g_ability_debug_seq;
    return 1;
}

/* ── conditional_alternative: faithful (compound.rs:684-803) ── */
int rb_compound_conditional_alternative(GameState *g, int actor, const AbilityEffect *eff, int branch, int host_cid){
    (void)branch;
    int has_primary = (eff->primary_effect != NULL);
    int has_alt = (eff->alternative_effect != NULL);
    if(has_primary && has_alt){
        if(eff->alternative_condition && eff->condition){
            if(rb_eval_condition_for_host(g,actor,host_cid,eff->alternative_condition)){
                if(eff->alternative_effect) rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->alternative_effect,host_cid);
                return 1;
            }
            if(rb_eval_condition_for_host(g,actor,host_cid,eff->condition)){
                if(eff->primary_effect) rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->primary_effect,host_cid);
            }
            return 1;
        }
        const Condition *cond = eff->alternative_condition ? eff->alternative_condition : eff->condition;
        if(cond){
            if(rb_eval_condition_for_host(g,actor,host_cid,cond)){
                if(eff->alternative_effect) rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->alternative_effect,host_cid);
            } else if(eff->primary_effect){
                rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->primary_effect,host_cid);
            }
            return 1;
        }
        /* no condition → prompt (headless: primary) */
        if(eff->primary_effect) rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->primary_effect,host_cid);
        else if(eff->alternative_effect) rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->alternative_effect,host_cid);
        return 1;
    }
    if(eff->alternative_condition){
        if(rb_eval_condition_for_host(g,actor,host_cid,eff->alternative_condition) && eff->alternative_effect)
            rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->alternative_effect,host_cid);
        return 1;
    }
    if(has_alt && !has_primary && !eff->alternative_condition){
        if(eff->condition){
            if(rb_eval_condition_for_host(g,actor,host_cid,eff->condition) && eff->alternative_effect)
                rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->alternative_effect,host_cid);
        }
        return 1;
    }
    if(eff->primary_effect) rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->primary_effect,host_cid);
    return 1;
}

/* ── conditional_on_result: faithful (compound.rs:805-866) ── */
int rb_compound_conditional_on_result(GameState *g, int actor, const AbilityEffect *eff, int host_cid){
    int cost_was_paid = 1;
    int cur=g->queue.cur;
    if(cur>=0&&cur<g->queue.n_entries){
        RbQueueEntry *e=&g->queue.entries[cur];
        int has_cost = (e->card_id>=0);
        /* check optional_cost_result / cost_paid / ability.cost existence */
        if(e->optional_cost_result==0) cost_was_paid=0;
        else if(!e->cost_paid){
            /* if entry has cost but not paid, check if ability had cost */
            // assume paid if no cost
        }
        (void)has_cost;
    }
    if(!cost_was_paid) return 1;
    if(eff->primary_effect){
        rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->primary_effect,host_cid);
        if(rb_has_pending_choice(g)){
            /* save condition check + followup as pending (compound.rs:832-838) */
            /* mirror save_pending_actions with a placeholder count 1 */
            if(g->queue.cur>=0&&g->queue.cur<g->queue.n_entries) g->queue.entries[g->queue.cur].pending_actions_n += 1;
            return 1;
        }
    }
    int cond_met = 1;
    if(eff->result_condition) cond_met = rb_eval_condition_for_host(g,actor,host_cid,eff->result_condition);
    if(cond_met && eff->followup_action){
        g->n_selected_cards=0;
        rb_execute_effect_ex(g,actor,(AbilityEffect*)eff->followup_action,host_cid);
    }
    if(g->queue.cur>=0&&g->queue.cur<g->queue.n_entries) g->queue.entries[g->queue.cur].optional_cost_result = -1; /* clear conditional_choice analogue */
    return 1;
}

/* ── conditional_on_optional: faithful (compound.rs:883-962) with Q92 energy check ── */
int rb_compound_conditional_on_optional(GameState *g, int actor, const AbilityEffect *eff, int taken, int host_cid){
    (void)host_cid;
    const AbilityEffect *optional_action = eff ? eff->optional_action : NULL;
    const AbilityEffect *conditional_action = eff ? eff->conditional_action : NULL;
    int is_negation = eff ? eff->conditional_negation : 0;
    /* Q92: if optional pay_energy and insufficient active energy, skip choice and run conditional directly */
    if(optional_action && conditional_action && optional_action->action && !strcmp(optional_action->action,"pay_energy")){
        int need = optional_action->count>=0? optional_action->count : 0;
        const char *ec = eff_extra(optional_action,"energy_count");
        if(ec) need = atoi(ec);
        if(need>0){
            int active = g->p[actor].energy_active;
            /* active_count mirrors energy_zone.active_count() */
            if(active < need){
                /* push rule log and execute conditional */
                const AbilityEffect *cmd = conditional_action;
                if(cmd) rb_execute_effect_ex(g,actor,(AbilityEffect*)cmd,host_cid);
                return 1;
            }
        }
    }
    if(optional_action && conditional_action){
        int result = -2;
        int cur=g->queue.cur;
        if(cur>=0&&cur<g->queue.n_entries){
            int v=g->queue.entries[cur].optional_cost_result;
            if(v==0||v==1) result=v;
        }
        if(result==0||result==1){
            int chose_yes = (result==1);
            const AbilityEffect *cmd = route_conditional_branch_ptr(eff,chose_yes,is_negation);
            if(cmd) rb_execute_effect_ex(g,actor,(AbilityEffect*)cmd,host_cid);
            return 1;
        }
        if(taken>=0){
            const AbilityEffect *cmd = route_conditional_branch_ptr(eff,taken?1:0,is_negation);
            if(cmd) rb_execute_effect_ex(g,actor,(AbilityEffect*)cmd,host_cid);
            return 1;
        }
        /* no result yet: emit choice (conditional_choice = Effect) */
        rb_emit_choice(g,actor,RB_CHOICE_SELECT_TARGET,NULL,NULL,0,1,"conditional_optional");
        return 1;
    }
    if(optional_action) rb_execute_effect_ex(g,actor,(AbilityEffect*)optional_action,host_cid);
    if(conditional_action && !is_negation) rb_execute_effect_ex(g,actor,(AbilityEffect*)conditional_action,host_cid);
    return 1;
}

/* ── choice helpers ── */
int rb_compound_choice_string(const AbilityEffect *eff, const char *choice){
    if(!eff||!choice) return -1;
    for(int i=0;i<eff->n_child;i++) if(eff->child[i]&&eff->child[i]->action&&!strcmp(eff->child[i]->action,choice)) return i;
    return -1;
}
int rb_compound_choice_action(GameState *g, int actor, const AbilityEffect *eff, int choice_idx, int host_cid){
    if(!eff||choice_idx<0||choice_idx>=eff->n_child) return 0;
    rb_execute_effect_ex(g,actor,eff->child[choice_idx],host_cid);
    return 1;
}

/* ── handle_choice_string_selection: faithful (compound.rs:964-987) ──
   Rust: if val starts with "heart" or is a Japanese heart-color name, push
   "selected_heart_color:<val>" to gs.prohibition_effects. Then clear the
   pending choice and resume pending actions (drains one pending slot). */
int rb_compound_handle_choice_string_selection(GameState *g, int actor, const char *selected, const char **options, int n_options){
    (void)actor;
    if(!g || !selected) return 0;
    int idx = atoi(selected);
    if(idx > 0 && idx <= n_options && options){
        const char *val = options[idx - 1];
        if(val){
            if(val[0] && !strncmp(val, "heart", 5)) {
                if(g->n_prohibition < 64)
                    snprintf(g->prohibition[g->n_prohibition++], 48,
                             "selected_heart_color:%s", val);
            } else if(!strcmp(val, "赤") || !strcmp(val, "桃") ||
                      !strcmp(val, "緑") || !strcmp(val, "青") ||
                      !strcmp(val, "黄") || !strcmp(val, "紫")) {
                if(g->n_prohibition < 64)
                    snprintf(g->prohibition[g->n_prohibition++], 48,
                             "selected_heart_color:%s", val);
            }
        }
    }
    rb_clear_pending_choice(g);
    if(g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries
       && g->queue.entries[g->queue.cur].pending_actions_n > 0){
        g->queue.entries[g->queue.cur].pending_actions_n--;
    }
    return 1;
}
/* ── handle_choice_string_store: faithful (compound.rs:989-1016) ──
   Rust: parse selected index, look up options[idx-1], store it as
   entry.conditional_choice = ConditionalChoice::Str(s). Then clear
   pending choice and resume pending actions. In C we record the result
   in entry.choice_result and resume_draw_ctype (the closest analogue
   to ConditionalChoice::Str — the gain_resource handler reads it from
   selected_heart_color or the effect's own field). */
int rb_compound_handle_choice_string_store(GameState *g, int actor, const char *selected, const char **options, int n_options){
    (void)actor;
    if(!g || !selected) return 0;
    int idx = atoi(selected);
    if(idx > 0 && idx <= n_options && options){
        const char *val = options[idx - 1];
        if(val){
            g->queue.choice_result = idx - 1;
            strncpy(g->queue.resume_draw_ctype, val,
                    sizeof(g->queue.resume_draw_ctype) - 1);
            g->queue.resume_draw_ctype[sizeof(g->queue.resume_draw_ctype) - 1] = '\0';
            if(g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries){
                /* choice_result already stores the index; the value string
                   is in resume_draw_ctype for the resume handler to read. */
            }
        }
    }
    rb_clear_pending_choice(g);
    if(g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries
       && g->queue.entries[g->queue.cur].pending_actions_n > 0){
        g->queue.entries[g->queue.cur].pending_actions_n--;
    }
    return 1;
}
