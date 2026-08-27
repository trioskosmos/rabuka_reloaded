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

/* Queue constant abilities — no queue push needed, but recalc modifiers.
   For portability we just re-apply constant heart/blade/score via mods. */
void rb_recalc_constants(GameState *g) {
    /* Clear old constant-derived mods (tracked in RbMods.constant_*) then
       re-apply from stage members whose ability triggers contain "常時".
       Simplified: no per-card positioning filter, just stage scan. */
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) {
        if (g->mods.constant_blade[i]) { rb_mods_add_blade(&g->mods, i, -g->mods.constant_blade[i]); g->mods.constant_blade[i]=0; }
        if (g->mods.constant_score[i]) { rb_mods_add_score(&g->mods, i, -g->mods.constant_score[i]); g->mods.constant_score[i]=0; }
    }
    for (int pl=0; pl<2; pl++) {
        for (int s=0;s<RB_STAGE_SIZE;s++) {
            int cid = g->p[pl].stage[s];
            if (cid==RB_EMPTY_SLOT) continue;
            Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
            if (c.ability && c.ability->triggers && rb_trigger_is(c.ability->triggers,"常時") && c.ability->effect) {
                /* Very small subset: modify_score / gain_resource blade as constant */
                AbilityEffect *e=c.ability->effect;
                if (e->action && !strcmp(e->action,"modify_score")) {
                    int cnt=e->count>=0?e->count:1;
                    rb_mods_add_score(&g->mods, cid, cnt);
                    g->mods.constant_score[cid]+=cnt;
                } else if (e->action && !strcmp(e->action,"gain_resource")) {
                    /* treat as blade bonus for host */
                    int cnt=e->count>=0?e->count:1;
                    rb_mods_add_blade(&g->mods, cid, cnt);
                    g->mods.constant_blade[cid]+=cnt;
                }
            }
            rb_free_card(&c);
        }
    }
}
