#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* Portable trigger scan — mirrors engine/src/triggers.rs:canonical_trigger
   + engine/src/turn/triggers.rs . Wire trigger strings are Japanese:
   "登場" (Debut), "ライブ開始時", "ライブ成功時", "常時", "起動", "自動" */

int rb_trigger_is(const char *triggers, const char *needle) {
    if (!triggers || !needle) return 0;
    return strstr(triggers, needle) != NULL;
}

/* Queue all stage members' debut abilities for the player who just played.
     Scan ALL abilities for that card (cards can have debut+constant). Mirrors
     Rust Card.abilities:Vec<AbilityRef> via CARD_ABILITY_PAIRS. */
int rb_trigger_debut(GameState *g, int pl, int card_id) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    int queued = 0;
    for(int i=0;i<n;i++){
        Ability ab; if(!rb_decode_card_ability((uint32_t)card_id,i,&ab)) continue;
        if(ab.triggers && rb_trigger_is(ab.triggers, "登場")){
            if (!rb_use_limit_reached(&g->queue, card_id, i, ab.use_limit < 0 ? 99 : ab.use_limit, g->turn)) {
                rb_queue_push(&g->queue, card_id, i);
                rb_record_use(&g->queue, card_id, i, g->turn);
                queued = 1;
            }
        }
        rb_free_ability(&ab);
    }
    (void)pl;
    return queued;
}

int rb_trigger_live_start(GameState *g, int pl) {
    int queued=0;
    /* Live cards in the live-card zone — mirrors Rust's scan of
       player.live_card_zone.cards before the stage scan. Many LiveStart
       autos (e.g. sd1-022) live on the live card, not on a stage member. */
    for (int i = 0; i < g->p[pl].live.n; i++) {
        int cid = g->p[pl].live.cards[i];
        if (cid == RB_EMPTY_SLOT) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; if(!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            if (ab.triggers && rb_trigger_is(ab.triggers,"ライブ開始時")) {
                if (!rb_use_limit_reached(&g->queue, cid, ai, ab.use_limit<0?99:ab.use_limit, g->turn)) {
                    rb_queue_push(&g->queue, cid, ai);
                    rb_record_use(&g->queue, cid, ai, g->turn);
                    queued++;
                }
            }
            rb_free_ability(&ab);
        }
    }
    /* Stage members — mirrors Rust's scan of player.stage. */
    for(int s=0;s<RB_STAGE_SIZE;s++){
        int cid=g->p[pl].stage[s];
        if(cid==RB_EMPTY_SLOT) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for(int i=0;i<n;i++){
            Ability ab; if(!rb_decode_card_ability((uint32_t)cid,i,&ab)) continue;
            if(ab.triggers && rb_trigger_is(ab.triggers,"ライブ開始時")){
                if(!rb_use_limit_reached(&g->queue, cid, i, ab.use_limit<0?99:ab.use_limit, g->turn)){
                    rb_queue_push(&g->queue, cid, i);
                    rb_record_use(&g->queue, cid, i, g->turn);
                    queued++;
                }
            }
            rb_free_ability(&ab);
        }
    }
    return queued;
}

/* Gate: only fire LiveSuccess when the live actually satisfied its heart
   requirements this turn (mirrors engine/src/core/game_state/abilities.rs:
   should_trigger_live_success — the live must have passed the heart check).
   In the C pipeline `passed` is recorded into g->live_success[pl] before the
   trigger block runs, so gating on it is equivalent to Rust's need_heart scan. */
int rb_should_trigger_live_success(const GameState *g, int pl) {
    if (!g) return 0;
    if (pl < 0 || pl > 1) return 0;
    if (g->p[pl].live.n == 0) return 0;
    return g->live_success[pl] ? 1 : 0;
}

/* Queue every ライブ成功時 (LiveSuccess) ability on a single card, deduplicated
   by (card_id, ability_idx). Mirrors the per-card scan in
   turn/triggers.rs::trigger_live_success_abilities. */
static int queue_live_success_for_card(GameState *g, int pl, int cid) {
    int queued = 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab; if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && rb_trigger_is(ab.triggers, "ライブ成功時")) {
            int key = (cid << 16) | (i & 0xFFFF);
            if (key != g->just_completed_ability_key &&
                !rb_use_limit_reached(&g->queue, cid, i, ab.use_limit < 0 ? 99 : ab.use_limit, g->turn)) {
                rb_queue_push(&g->queue, cid, i);
                rb_record_use(&g->queue, cid, i, g->turn);
                queued++;
            }
        }
        rb_free_ability(&ab);
    }
    return queued;
}

/* Mirror rb_trigger_live_start for the ライブ成功時 (LiveSuccess) trigger.
   Queues abilities whose trigger matches, scanning BOTH the live-card zone
   (live cards carry their own LiveSuccess autos, e.g. live-card effects) and
   the staged members (mirrors turn/triggers.rs::trigger_live_success_abilities
   which iterates player.live_card_zone then player.stage). Gated by
   rb_should_trigger_live_success so it only fires on a successful live. */
int rb_trigger_live_success(GameState *g, int pl) {
    if (!rb_should_trigger_live_success(g, pl)) return 0;
    int queued = 0;
    for (int i = 0; i < g->p[pl].live.n; i++) {
        int cid = g->p[pl].live.cards[i];
        if (cid == RB_EMPTY_SLOT) continue;
        queued += queue_live_success_for_card(g, pl, cid);
    }
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = g->p[pl].stage[s];
        if (cid == RB_EMPTY_SLOT) continue;
        queued += queue_live_success_for_card(g, pl, cid);
    }
    return queued;
}

/* Mirror live.rs::drain_pending_live_success_choices. Re-entrantly drain the
   ability queue until it empties or a pending choice surfaces (the host
   resolver resumes and re-drains). Returns 1 if a pending choice was left
   unresolved (caller should yield to the host), 0 otherwise. */
int rb_drain_live_success_choices(GameState *g) {
    if (!g) return 0;
    int guard = 0;
    while (g->queue.n_entries > 0 && guard++ < 64) {
        rb_drain_ability_queue(g);
        if (rb_has_pending_choice(g)) return 1;
    }
    return 0;
}

/* Helper: apply a single effect as a constant modifier (no condition gate here
    beyond the caller's check). Mirrors the blade/heart/score/need_heart branches
    of engine/src/core/game_state/modifiers.rs:recalculate_constants . */
/* Mirror a stage position across the center line: left<->right, center stays. */
static const char *mirror_position(const char *pos) {
    if (!pos) return NULL;
    if (!strcmp(pos,"left_side"))  return "right_side";
    if (!strcmp(pos,"right_side")) return "left_side";
    return pos; /* center / other */
}
/* Return the card id occupying `pos` on player `pl`, or RB_EMPTY_SLOT. */
static int card_at_position(const GameState *g, int pl, const char *pos) {
    int idx = -1;
    if (!pos) return RB_EMPTY_SLOT;
    if (!strcmp(pos,"center")) idx = 1;
    else if (!strcmp(pos,"left_side")) idx = 0;
    else if (!strcmp(pos,"right_side")) idx = 2;
    else return RB_EMPTY_SLOT;
    return g->p[pl].stage[idx];
}

static int effect_is_live_end(AbilityEffect *e) {
    if (!e) return 0;
    for (int i=0;i<e->n_extra;i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i],"duration") && e->extra_v[i]) {
            const char *v = e->extra_v[i];
            if (strstr(v,"live") || strstr(v,"ライブ")) return 1;
        }
    }
    return 0;
}

/* acc != NULL means "record deltas into this temporary effect" (for Duration::LiveEnd
   debut effects). The same code path applies the modifier to the live modifier table. */
static void apply_constant_effect(GameState *g, int pl, int host_cid, AbilityEffect *e, RbTempEffect *acc) {
    if (!e || !e->action) return;
    /* Position-targeted modifiers (ruby front / love_wing_bell center): the
       effect grants its resource to the member at a given stage position rather
       than the host. Mirrors engine/src/core/game_state/modifiers.rs constant
       application where the effect's `position`/`activation_position` select the
       recipient. */
    int tgt_cid = host_cid;
    int tgt_pl  = pl;
    const char *pos = NULL, *act_pos = NULL;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"position")) pos=e->extra_v[i];
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"activation_position")) act_pos=e->extra_v[i];
    }
    if (pos) {
        if (act_pos) {            /* front mechanic: affects OPPONENT at mirrored pos */
            tgt_pl = 1 - pl;
            pos = mirror_position(act_pos);
        } else {                  /* same-player position (e.g. own center) */
            tgt_pl = pl;
        }
        int c = card_at_position(g, tgt_pl, pos);
        if (c != RB_EMPTY_SLOT) tgt_cid = c;
    }
    if (!strcmp(e->action,"modify_cost")) {
        int cnt = e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")){
            int v = atoi(e->extra_v[i]); if(v) cnt = v;
        }
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"sign") && e->extra_v[i] && !strcmp(e->extra_v[i],"negative")) cnt = -cnt;
        rb_mods_add_cost(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_cost[tgt_cid]+=cnt;
        if(acc) acc->cost+=cnt;
    } else if (!strcmp(e->action,"modify_score")) {
        int cnt=e->count>=0?e->count:1;
        rb_mods_add_score(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_score[tgt_cid]+=cnt;
        if(acc) acc->score+=cnt;
    } else if (!strcmp(e->action,"gain_blade") || !strcmp(e->action,"add_blade") ||
               !strcmp(e->action,"gain_blade_heart") || !strcmp(e->action,"set_blade_count") ||
               !strcmp(e->action,"modify_blade")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"sign") && e->extra_v[i] && !strcmp(e->extra_v[i],"negative")) cnt = -cnt;
        rb_mods_add_blade(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_blade[tgt_cid]+=cnt;
        if(acc) acc->blade+=cnt;
    } else if (!strcmp(e->action,"gain_resource")) {
        const char *res=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"resource")) res=e->extra_v[i];
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"sign") && e->extra_v[i] && !strcmp(e->extra_v[i],"negative")) cnt = -cnt;
        if (res && (!strcmp(res,"blade")||!strcmp(res,"ブレード"))) {
            rb_mods_add_blade(&g->mods, tgt_cid, cnt);
            if(!acc) g->mods.constant_blade[tgt_cid]+=cnt;
            if(acc) acc->blade+=cnt;
        } else if (res && (!strcmp(res,"heart")||!strcmp(res,"ハート"))) {
            const char *hc=NULL;
            for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_color")) hc=e->extra_v[i];
            int col=0;
            if(hc){
                if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")||!strcmp(hc,"heart0")) col=0;
                else if(!strcmp(hc,"red")||!strcmp(hc,"heart01")||!strcmp(hc,"heart1")) col=1;
                else if(!strcmp(hc,"yellow")||!strcmp(hc,"heart02")||!strcmp(hc,"heart2")) col=2;
                else if(!strcmp(hc,"green")||!strcmp(hc,"heart03")||!strcmp(hc,"heart3")) col=3;
                else if(!strcmp(hc,"blue")||!strcmp(hc,"heart04")||!strcmp(hc,"heart4")) col=4;
                else if(!strcmp(hc,"purple")||!strcmp(hc,"heart05")||!strcmp(hc,"heart5")) col=5;
                else if(!strcmp(hc,"orange")||!strcmp(hc,"heart06")||!strcmp(hc,"heart6")) col=6;
                else if(!strcmp(hc,"all")||!strcmp(hc,"heart07")||!strcmp(hc,"b_all")) col=7;
            }
            rb_mods_add_heart(&g->mods, tgt_cid, col, cnt);
            if(!acc) g->mods.constant_heart[tgt_cid][col]+=cnt;
            if(acc) acc->heart[col]+=cnt;
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
        rb_mods_add_need_heart(&g->mods, tgt_cid, col, cnt);
        if(!acc) g->mods.constant_need_heart[tgt_cid][col]+=cnt;
        if(acc) acc->need_heart[col]+=cnt;
    } else if (!strcmp(e->action,"set_cost") || !strcmp(e->action,"set_cost_to_use")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) cnt=atoi(e->extra_v[i]);
        rb_mods_add_cost(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_cost[tgt_cid]+=cnt;
        if(acc) acc->cost+=cnt;
    } else if (!strcmp(e->action,"set_score")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) cnt=atoi(e->extra_v[i]);
        rb_mods_add_score(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_score[tgt_cid]+=cnt;
        if(acc) acc->score+=cnt;
    } else if (!strcmp(e->action,"set_blade_count")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) cnt=atoi(e->extra_v[i]);
        rb_mods_add_blade(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_blade[tgt_cid]+=cnt;
        if(acc) acc->blade+=cnt;
    } else if (!strcmp(e->action,"set_blade_type")) {
        /* Constant blade-color recolor — mirrors engine.c:set_blade_type field write.
            Re-applied idempotently each recalc (no additive tracking needed). */
        const char *bc=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && (!strcmp(e->extra_k[i],"blade_color")||!strcmp(e->extra_k[i],"blade_type"))) bc=e->extra_v[i];
        int col=-1;
        if(bc){
            if(!strcmp(bc,"pink")||!strcmp(bc,"heart00")) col=0;
            else if(!strcmp(bc,"red")) col=1;
            else if(!strcmp(bc,"yellow")) col=2;
            else if(!strcmp(bc,"green")) col=3;
            else if(!strcmp(bc,"blue")) col=4;
            else if(!strcmp(bc,"purple")) col=5;
            else if(!strcmp(bc,"orange")) col=6;
            else if(!strcmp(bc,"all")) col=7;
        }
        g->mods.blade_type[tgt_cid]=(int8_t)col;
    } else if (!strcmp(e->action,"set_heart_type")) {
        /* Constant heart recolor — mirrors engine.c:set_heart_type field write. */
        const char *ref=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"ref_value")) ref=e->extra_v[i];
        if(ref && !strcmp(ref,"placed_under")){
            for(int s=0;s<RB_STAGE_SIZE;s++){ if(g->p[tgt_pl].stage[s]==tgt_cid && g->p[tgt_pl].under_cards[s].n>0){ g->mods.heart_copy[tgt_cid]=g->p[tgt_pl].under_cards[s].cards[0]; break; } }
        } else {
            int hcol=7;
            for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_color")){
                const char *hc=e->extra_v[i];
                if(!hc) continue;
                else if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) hcol=0;
                else if(!strcmp(hc,"red")) hcol=1;
                else if(!strcmp(hc,"yellow")) hcol=2;
                else if(!strcmp(hc,"green")) hcol=3;
                else if(!strcmp(hc,"blue")) hcol=4;
                else if(!strcmp(hc,"purple")) hcol=5;
                else if(!strcmp(hc,"orange")) hcol=6;
                else if(!strcmp(hc,"all")) hcol=7;
            }
            g->mods.heart_multiplier[tgt_cid]=(int8_t)hcol;
        }
    } else if (!strcmp(e->action,"gain_ability")) {
        const char *ag=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"ability_gain")) ag=e->extra_v[i];
        if(ag && strstr(ag,"ハート")) {
            rb_mods_add_heart(&g->mods, tgt_cid, 7, 1);
            if(!acc) g->mods.constant_heart[tgt_cid][7]+=1;
            if(acc) acc->heart[7]+=1;
        }
    }
    /* sequential children: walk them (q127_wien leaves_stage_modifier_removed etc.) */
    for(int i=0;i<e->n_child;i++) apply_constant_effect(g, pl, host_cid, e->child[i], acc);
}

/* Fire a card's 登場 (Debut) abilities by queuing them and draining the queue,
   exactly mirroring Rust's trigger_debut → process_pending_auto_abilities path.
   This executes the ability's full effect tree (move_cards / gain_resource /
   modify_cost / etc.) instead of only the constant modifiers. */
void rb_fire_debut(GameState *g, int pl, int card_id) {
    rb_trigger_debut(g, pl, card_id);
    if (g->queue.n_entries > 0)
        rb_drain_ability_queue(g);
}

/* Revert temporary effects whose Duration matches `which`:
     0 = all, 1 = live_end/during_live (called at live-phase end),
     2 = until_end_of_turn/first_turn (called at turn rollover).
   Reverted entries are compacted out of the array so other durations survive. */
void rb_check_expired_effects(GameState *g, int which) {
    int w = g->n_temp_effects;
    int j = 0;
    for(int i=0;i<w;i++){
        RbTempEffect *te=&g->temp_effects[i];
        int expire = (which==0) || (which==1 && te->dur==RB_TEMP_LIVE_END) ||
                     (which==2 && te->dur==RB_TEMP_TURN_END);
        if(expire){
            /* grants are credited to the effective modifier only (not the
               constant_* tracking that rb_recalc_constants owns), so revert by
               subtracting the effective modifier here. */
            rb_mods_add_blade(&g->mods, te->card_id, -te->blade);
            rb_mods_add_score(&g->mods, te->card_id, -te->score);
            rb_mods_add_cost(&g->mods, te->card_id, -te->cost);
            for(int c=0;c<8;c++){
                rb_mods_add_heart(&g->mods, te->card_id, c, -te->heart[c]);
                rb_mods_add_need_heart(&g->mods, te->card_id, c, -te->need_heart[c]);
            }
        } else {
            g->temp_effects[j++] = g->temp_effects[i]; /* keep */
        }
    }
    g->n_temp_effects = j;
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
        if (g->mods.constant_cost[i])  { rb_mods_add_cost(&g->mods, i, -g->mods.constant_cost[i]);  g->mods.constant_cost[i]=0; }
        for (int c = 0; c < 8; c++) {
            if (g->mods.constant_heart[i][c])      { rb_mods_add_heart(&g->mods, i, c, -g->mods.constant_heart[i][c]);      g->mods.constant_heart[i][c]=0; }
            if (g->mods.constant_need_heart[i][c]) { rb_mods_add_need_heart(&g->mods, i, c, -g->mods.constant_need_heart[i][c]); g->mods.constant_need_heart[i][c]=0; }
        }
    }
    for (int pl=0; pl<2; pl++) {
        /* Constant abilities can be owned by cards anywhere the player controls:
           on stage, in the success live-card zone, or the live-card zone. Rust's
           recalculate_constants scans all of these (e.g. Love wing bell lives in
           the success zone and buffs the center member). */
        int zone_cids[RB_STAGE_SIZE + RB_MAX_LIVE_CARDS*2];
        int zn = 0;
        for (int s=0;s<RB_STAGE_SIZE;s++) if (g->p[pl].stage[s]!=RB_EMPTY_SLOT) zone_cids[zn++]=g->p[pl].stage[s];
        for (int s=0;s<g->p[pl].success.n;s++) if (g->p[pl].success.cards[s]!=RB_EMPTY_SLOT) zone_cids[zn++]=g->p[pl].success.cards[s];
        for (int s=0;s<g->p[pl].live.n;s++) if (g->p[pl].live.cards[s]!=RB_EMPTY_SLOT) zone_cids[zn++]=g->p[pl].live.cards[s];
        for (int z=0; z<zn; z++) {
            int cid = zone_cids[z];
            int n = rb_card_num_abilities((uint32_t)cid);
            for(int ai=0; ai<n; ai++){
                Ability ab; if(!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
                if (ab.triggers && rb_trigger_is(ab.triggers,"常時") && ab.effect) {
                    AbilityEffect *e=ab.effect;
                    int cond_ok=1;
                    if(e->has_condition && e->condition) cond_ok=rb_eval_condition_for_host(g, pl, cid, e->condition);
                    if(cid==1923||cid==2500||cid==2406||cid==2412) fprintf(stderr,"[recalc] cid=%d pl=%d cond_ok=%d act=%s\n",cid,pl,cond_ok,e->action?e->action:"-");
                    if(cond_ok) apply_constant_effect(g, pl, cid, e, NULL);
                    else {
                        for(int i=0;i<e->n_child;i++){
                            AbilityEffect *ch=e->child[i];
                            int ch_ok=1;
                            if(ch->has_condition && ch->condition) ch_ok=rb_eval_condition_for_host(g, pl, cid, ch->condition);
                            if(ch_ok) apply_constant_effect(g, pl, cid, ch, NULL);
                        }
                    }
                }
                rb_free_ability(&ab);
            }
        }
    }
    rb_refresh_yell_sources(g);
}
