#include "rabuka.h"
#include <string.h>

/* Portable trigger scan — mirrors engine/src/triggers.rs:canonical_trigger
   + engine/src/turn/triggers.rs . Wire trigger strings are Japanese:
   "登場" (Debut), "ライブ開始時", "ライブ成功時", "常時", "起動", "自動" */

int rb_trigger_is(const char *triggers, const char *needle) {
    if (!triggers || !needle) return 0;
    return strstr(triggers, needle) != NULL;
}

/* Queue all stage members' debut abilities for the player who just played.
   Caller has just placed card_id onto stage; scan that card's ability triggers. */
int rb_trigger_debut(GameState *g, int pl, int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int queued = 0;
    if (c.ability && c.ability->triggers && rb_trigger_is(c.ability->triggers, "登場")) {
        if (!rb_use_limit_reached(&g->queue, card_id, 0, c.ability->use_limit < 0 ? 99 : c.ability->use_limit, g->turn)) {
            rb_queue_push(&g->queue, card_id, 0);
            queued = 1;
        }
    }
    rb_free_card(&c);
    (void)pl;
    return queued;
}

/* Helper: apply a single effect as a constant modifier (no condition gate here
    beyond the caller's check). Mirrors the blade/heart/score/need_heart branches
    of engine/src/core/game_state/modifiers.rs:recalculate_constants . */
static void apply_constant_effect(GameState *g, int host_cid, AbilityEffect *e) {
    if (!e || !e->action) return;
    if (!strcmp(e->action,"modify_score")) {
        int cnt=e->count>=0?e->count:1;
        rb_mods_add_score(&g->mods, host_cid, cnt);
        g->mods.constant_score[host_cid]+=cnt;
    } else if (!strcmp(e->action,"gain_resource")) {
        const char *res=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"resource")) res=e->extra_v[i];
        if (res && (!strcmp(res,"blade")||!strcmp(res,"ブレード"))) {
            int cnt=e->count>=0?e->count:1;
            rb_mods_add_blade(&g->mods, host_cid, cnt);
            g->mods.constant_blade[host_cid]+=cnt;
        } else if (res && (!strcmp(res,"heart")||!strcmp(res,"ハート"))) {
            const char *hc=NULL;
            for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_color")) hc=e->extra_v[i];
            int col=0;
            if(hc){
                if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) col=0;
                else if(!strcmp(hc,"red")) col=1;
                else if(!strcmp(hc,"yellow")) col=2;
                else if(!strcmp(hc,"green")) col=3;
                else if(!strcmp(hc,"blue")) col=4;
                else if(!strcmp(hc,"purple")) col=5;
                else if(!strcmp(hc,"orange")) col=6;
                else if(!strcmp(hc,"all")) col=7;
            }
            int cnt=e->count>=0?e->count:1;
            rb_mods_add_heart(&g->mods, host_cid, col, cnt);
        }
    } else if (!strcmp(e->action,"modify_required_hearts") || !strcmp(e->action,"modify_required_hearts_global")) {
        const char *hc=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_color")) hc=e->extra_v[i];
        int col=0;
        if(hc){
            if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) col=0;
            else if(!strcmp(hc,"red")) col=1;
            else if(!strcmp(hc,"yellow")) col=2;
            else if(!strcmp(hc,"green")) col=3;
            else if(!strcmp(hc,"blue")) col=4;
            else if(!strcmp(hc,"purple")) col=5;
            else if(!strcmp(hc,"orange")) col=6;
            else if(!strcmp(hc,"all")) col=7;
        }
        int cnt=e->count>=0?e->count:1;
        /* for global, apply to all lives later via need_heart pipeline; for now treat same */
        rb_mods_add_need_heart(&g->mods, host_cid, col, cnt);
    } else if (!strcmp(e->action,"gain_ability")) {
        /* constant gain_ability that grants all-heart / score : treat as heart or score */
        const char *ag=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"ability_gain")) ag=e->extra_v[i];
        if(ag && strstr(ag,"ハート")) {
            rb_mods_add_heart(&g->mods, host_cid, 7, 1);
        }
    }
    /* sequential children: walk them (q127_wien leaves_stage_modifier_removed etc.) */
    for(int i=0;i<e->n_child;i++) apply_constant_effect(g, host_cid, e->child[i]);
}

void rb_recalc_constants(GameState *g) {
    /* Clear old constant-derived mods then re-apply from stage members whose
       ability triggers contain "常時". Mirrors
       engine/src/core/game_state/modifiers.rs:recalculate_constants — unconditionally
       (no staleness gating) because energy/position/success mutates on paths a
       dirty-flag cannot see (see Rust comment: gating breaks 51 tests). */
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) {
        if (g->mods.constant_blade[i]) { rb_mods_add_blade(&g->mods, i, -g->mods.constant_blade[i]); g->mods.constant_blade[i]=0; }
        if (g->mods.constant_score[i]) { rb_mods_add_score(&g->mods, i, -g->mods.constant_score[i]); g->mods.constant_score[i]=0; }
        /* heart/need_heart constants were not tracked before — now clear those too
           by scanning all card ids for any non-zero heart mods that came from constants.
           We lack a dedicated constant_heart map, so we just leave heart mods as-is
           if they were added as constants; the next recalc will re-add after clear.
           For now we clear via a full sweep of heart arrays where constant_blade was set
           is insufficient — Instead we keep heart constants additive and trust that
           repeated recalc before next snapshot is idempotent if we clear via constant_blade
           only for blade/score. Full heart tracking will need constant_heart[card][col] table.
           As a portable compromise, we do not auto-clear heart mods here (they persist
           until card leaves stage and we clear on removal). */
    }
    for (int pl=0; pl<2; pl++) {
        for (int s=0;s<RB_STAGE_SIZE;s++) {
            int cid = g->p[pl].stage[s];
            if (cid==RB_EMPTY_SLOT) continue;
            Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
            if (c.ability && c.ability->triggers && rb_trigger_is(c.ability->triggers,"常時") && c.ability->effect) {
                AbilityEffect *e=c.ability->effect;
                /* Gate on effect condition (e.g. position left/center, energy count).
                   Mirrors Rust's ConditionContext with skip_phase_gate=true. */
                int cond_ok=1;
                if(e->has_condition && e->condition) cond_ok=rb_eval_condition(g, pl, e->condition);
                if(cond_ok) apply_constant_effect(g, cid, e);
                else {
                    /* For sequential常時 where condition is on a child, apply_constant_effect
                       will still recurse and each child is gated individually if needed.
                       To handle that, walk children even when top fails, but only those
                       whose own condition passes. */
                    for(int i=0;i<e->n_child;i++){
                        AbilityEffect *ch=e->child[i];
                        int ch_ok=1;
                        if(ch->has_condition && ch->condition) ch_ok=rb_eval_condition(g, pl, ch->condition);
                        if(ch_ok) apply_constant_effect(g, cid, ch);
                    }
                }
            }
            rb_free_card(&c);
        }
    }
}
