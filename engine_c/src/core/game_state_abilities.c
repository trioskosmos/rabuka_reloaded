/* game_state_abilities.c — auto-trigger engine, ability-use tracking, and
   temporary-effect expiry.
   Mirror engine/src/core/game_state/abilities.rs (ability_matches_trigger,
   record_ability_use, collect_constant_hand, collect_live_ability_modifiers,
   trigger_auto_abilities_for_player, process_pending_auto_abilities,
   check_expired_effects, apply_ability_effects).

   STUBS: the Rust engine drives auto-triggers (debut/on_live/on_resolve)
   and temporary effect expiry from here. The C port does neither yet —
   these signatures exist so the wiring can be added function-by-function.
   Defaults are permissive/no-op so the build stays green. */

#include "rabuka.h"
#include <string.h>

/* Mirror abilities.rs:ability_matches_trigger — does ability `ab` fire on
   `trigger`? Uses the decoded ability text trigger set. */
int rb_ability_matches_trigger(const Ability *ab, const char *trigger) {
    if (!ab || !trigger) return 0;
    return rb_trigger_is(ab->triggers, trigger);
}

/* Mirror abilities.rs:record_ability_use — mark `cid`'s `idx`-th ability as
   used this turn (for once-per-turn gating). The authoritative per-turn tracker
   is the ability queue's use table (mirrors Rust's turn_limited_abilities_used
   HashMap keyed by (card_id, ability_idx, turn)); delegate there so the saturating
   count is the same one consulted by rb_use_limit_reached. */
void rb_record_ability_use(GameState *g, int cid, int idx) {
    if (!g) return;
    rb_record_use(&g->queue, cid, idx, g->turn);
}

/* Apply one constant-modifier effect node to the per-card constant tables.
   The wire maps the resource kind via source/destination/action and the
   magnitude via count. Best-effort mapping (see PROGRESS §12). */
static void apply_constant_node(RbMods *m, int cid, const AbilityEffect *e) {
    if (!e) return;
    int delta = e->count != 0 ? e->count : 1;
    const char *kind = e->source ? e->source : (e->destination ? e->destination : e->action);
    if (!kind) return;
    if (strstr(kind, "score"))            rb_mods_add_score(m, cid, delta);
    else if (strstr(kind, "blade"))       rb_mods_add_blade(m, cid, delta);
    else if (strstr(kind, "heart"))       rb_mods_add_heart(m, cid, 0, delta);
    else if (strstr(kind, "need_heart"))  rb_mods_add_need_heart(m, cid, 0, delta);
    else if (strstr(kind, "cost"))        rb_mods_add_cost(m, cid, delta);
    /* recurse into sub-nodes (conditional constant abilities) */
    for (int i = 0; i < e->n_child; i++) apply_constant_node(m, cid, e->child[i]);
}

/* Mirror abilities.rs:collect_constant_hand — scan the actor's stage members,
   decode their triggerless/constant abilities, and apply their constant
   modifiers into g->mods. Returns the number of constant abilities found. */
int rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)out; (void)max;
    if (!g) return 0;
    int found = 0;
    const RbPlayer *P = &g->p[actor];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = P->stage[s];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            int is_constant = (ab.triggers == NULL || ab.triggers[0] == '\0' ||
                               strstr(ab.triggers, "constant") != NULL ||
                               strstr(ab.triggers, "continuous") != NULL);
            if (is_constant && ab.effect) {
                apply_constant_node((RbMods *)&g->mods, cid, ab.effect);
                found++;
            }
            rb_free_ability(&ab);
        }
    }
    return found;
}

/* Mirror abilities.rs:collect_live_ability_modifiers — gather temporary
   modifiers applied during a live. Returns 0 (not tracked yet). */
int rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)g; (void)actor; (void)out; (void)max;
    return 0;
}

/* Queue every ability on the cards `ids[0..n]` whose trigger matches `trigger`.
    Mirrors abilities.rs:trigger_auto_abilities_for_player_with_event's TAS scan:
    each stage (and live/success/hand/energy) card's auto abilities are enqueued
    once per turn (use_limit gating) and the just-resolved ability is skipped so
    an auto ability does not recursively re-trigger itself. Returns count queued. */
static int queue_zone_abilities(GameState *g, int actor, const int *ids, int n,
                                 const char *trigger) {
    int queued = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_ability_matches_trigger(&ab, trigger)) {
                int key = (cid << 16) | (a & 0xFFFF);
                int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
                if (key != g->just_completed_ability_key &&
                    !rb_use_limit_reached(&g->queue, cid, a, limit, g->turn)) {
                    rb_queue_push(&g->queue, cid, a);
                    rb_record_use(&g->queue, cid, a, g->turn);
                    queued++;
                }
            }
            rb_free_ability(&ab);
        }
    }
    return queued;
}

/* Mirror abilities.rs:trigger_auto_abilities_for_player — enqueue all
    auto-trigger abilities of `actor` matching `trigger` across the actor's
    stage, success-live, live, hand and energy zones (plus the batch of cards
    recently moved this resolution). Returns count queued. */
int rb_queue_trigger_abilities(GameState *g, int pl, const char *trigger) {
    if (!g || !trigger) return 0;
    int total = 0;
    const RbPlayer *P = &g->p[pl];
    total += queue_zone_abilities(g, pl, P->stage, RB_STAGE_SIZE, trigger);
    total += queue_zone_abilities(g, pl, P->success.cards, P->success.n, trigger);
    total += queue_zone_abilities(g, pl, P->live.cards, P->live.n, trigger);
    total += queue_zone_abilities(g, pl, P->hand.cards, P->hand.n, trigger);
    total += queue_zone_abilities(g, pl, P->energy.cards, P->energy.n, trigger);
    total += queue_zone_abilities(g, pl, g->recently_moved, g->n_recently_moved, trigger);
    return total;
}

/* Mirror abilities.rs:trigger_auto_abilities_for_player + process_pending — queue
    then drain. Returns count fired. */
int rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger) {
    int queued = rb_queue_trigger_abilities(g, actor, trigger);
    if (queued > 0) rb_drain_ability_queue(g);
    /* Mirror Rust: just_completed_ability_key is cleared after the TAS scan so a
        later scan in the same resolution does not keep skipping this ability. */
    g->just_completed_ability_key = -1;
    return queued;
}

/* Convenience: fire a player's 自動 (Auto) trigger abilities. */
int rb_fire_auto(GameState *g, int pl) {
    return rb_trigger_auto_abilities(g, pl, "自動");
}

/* Event→trigger recording (mirrors Rust's movement-event → auto-trigger queue).
    The test harness records discrete events via push_movement_event(...) and then
    calls trigger_auto_abilities_for_player, which should fire ONLY the abilities
    whose trigger matches a recorded event — not every auto ability unconditionally
    (that would break "should-not-trigger" tests). */
static const struct { int bit; const char *trig; } RB_EV[] = {
    { 1, "エネルギー置いた時" }, { 2, "移動時" }, { 4, "応援時" },
    { 8, "公開時" }, { 16, "覚醒時" }, { 32, "レスト時" },
    { 64, "バトンタッチ時" }, { 128, "除外時" }, { 256, "ターン開始時" },
    { 512, "ドロー時" }, { 1024, "自動" }, { 0, NULL }
};
void rb_record_event(GameState *g, int pl, const char *trig) {
    if (!g || pl < 0 || pl > 1) return;
    for (int i = 0; RB_EV[i].trig; i++)
        if (!strcmp(RB_EV[i].trig, trig)) { g->auto_event_mask[pl] |= RB_EV[i].bit; return; }
}
int rb_fire_recorded_auto(GameState *g, int pl) {
    if (!g || pl < 0 || pl > 1) return 0;
    int total = 0, mask = g->auto_event_mask[pl];
    for (int i = 0; RB_EV[i].trig; i++)
        if (mask & RB_EV[i].bit) total += rb_trigger_auto_abilities(g, pl, RB_EV[i].trig);
    g->auto_event_mask[pl] = 0;
    return total;
}

/* Mirror abilities.rs:process_pending_auto_abilities — drain the queue of
    deferred auto-triggers. Returns count processed. */
int rb_process_pending_auto_abilities(GameState *g) {
    if (!g) return 0;
    return rb_drain_ability_queue(g);
}

/* Mirror abilities.rs:check_expired_effects — expire temporary effects whose
    duration has elapsed (end of turn / end of live). Definition lives in
    turn/triggers.c (shared with the live/turn pipeline); declared in rabuka.h. */

/* Mirror abilities.rs:apply_ability_effects — run the persistent / on-trigger
   effects of an ability. The engine's rb_execute_effect already handles the
   action dispatch; here we drive the decoded effect tree via the compound
   sequential runner. */
int rb_apply_ability_effects(GameState *g, int actor, const Ability *ab, int host_cid) {
    if (!g || !ab || !ab->effect) return 0;
    rb_compound_sequential(g, actor, ab->effect, host_cid);
    return 1;
}

/* ── GameState ability helpers (mirror game_state/abilities.rs methods) ── */

/* Mirror abilities.rs::opponent_id — the index of the other player. */
int rb_opponent_id(int pl) { return pl ? 0 : 1; }

/* Mirror abilities.rs::distinct_stage_groups — count of distinct canonical
   groups among `pl`'s stage members (multi-name cards contribute every group). */
int rb_distinct_stage_groups(const GameState *g, int pl) {
    static const char *CANON[5] = {"μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空"};
    const RbPlayer *P = &g->p[pl];
    int count = 0;
    for (int gi = 0; gi < 5; gi++) {
        int matched = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int cid = P->stage[s];
            if (cid != RB_EMPTY_SLOT && rb_card_matches_group_str(cid, CANON[gi])) {
                matched = 1; break;
            }
        }
        if (matched) count++;
    }
    return count;
}

/* Zone equivalence with live↔success interchange (mirrors Rust Zone comparisons). */
static int zone_eq(const char *a, const char *b) {
    if (!a || !b) return 0;
    int a_live = (!strcmp(a, "live") || !strcmp(a, "live_card_zone"));
    int b_live = (!strcmp(b, "live") || !strcmp(b, "live_card_zone"));
    int a_succ = (!strcmp(a, "success") || !strcmp(a, "success_live_zone") ||
                  !strcmp(a, "success_live_card_zone"));
    int b_succ = (!strcmp(b, "success") || !strcmp(b, "success_live_zone") ||
                  !strcmp(b, "success_live_card_zone"));
    if (a_live && b_live) return 1;
    if (a_succ && b_succ) return 1;
    if (a_live && b_succ) return 1;
    if (a_succ && b_live) return 1;
    return !strcmp(a, b);
}

/* Mirror abilities.rs::_prohibition_destination_blocks — does a
   "restriction:cannot_place:ZONE" prohibition block placement in `zone`? */
static int prohibition_blocks_zone(const char *p, const char *zone) {
    const char *prefix = "restriction:cannot_place:";
    size_t plen = strlen(prefix);
    if (strncmp(p, prefix, plen) != 0) return 0;
    const char *block = p + plen;
    return zone_eq(block, zone);
}

/* Mirror abilities.rs::can_place_card_in_zone — reject if a constant
   "cannot_place" restriction or a dynamic prohibition_effects entry blocks it. */
int rb_can_place_card_in_zone(const GameState *g, int cid, const char *zone) {
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int is_constant = (ab.triggers == NULL || ab.triggers[0] == '\0' ||
                           strstr(ab.triggers, "constant") != NULL ||
                           strstr(ab.triggers, "continuous") != NULL);
        if (is_constant && ab.effect) {
            const AbilityEffect *e = ab.effect;
            const char *act = e->action ? e->action : "";
            const char *rtype = NULL;
            for (int i = 0; i < e->n_extra; i++)
                if (e->extra_k[i] && !strcmp(e->extra_k[i], "restriction_type"))
                    rtype = e->extra_v[i];
            const char *dest = NULL;
            for (int i = 0; i < e->n_extra; i++)
                if (e->extra_k[i] && (!strcmp(e->extra_k[i], "destination") ||
                                      !strcmp(e->extra_k[i], "restricted_destination")))
                    dest = e->extra_v[i];
            if (!strcmp(act, "restriction") && rtype && !strcmp(rtype, "cannot_place") &&
                dest && zone_eq(dest, zone)) {
                rb_free_ability(&ab);
                return 0;
            }
        }
        rb_free_ability(&ab);
    }
    for (int i = 0; i < g->n_prohibition; i++)
        if (prohibition_blocks_zone(g->prohibition[i], zone)) return 0;
    return 1;
}

/* Mirror abilities.rs::clear_movement_tracking — drop the recently-moved batch. */
void rb_clear_movement_tracking(GameState *g) {
    if (!g) return;
    g->n_recently_moved = 0;
}

/* Mirror abilities.rs::process_with_completed_key — set the completed key, drain
   pending auto abilities, then clear the key. */
void rb_process_with_completed_key(GameState *g, int key) {
    if (!g) return;
    g->just_completed_ability_key = key;
    rb_drain_ability_queue(g);
    g->just_completed_ability_key = -1;
}

/* Mirror abilities.rs::ability_uses_used — uses recorded this turn for an ability. */
int rb_ability_uses_used(const GameState *g, int cid, int idx) {
    if (!g) return 0;
    return rb_use_count(&g->queue, cid, idx, g->turn);
}

/* Mirror abilities.rs::ability_has_remaining_uses — single source of truth for the
   once-per-turn gate. Abilities without a limit are always allowed. */
int rb_ability_has_remaining_uses(const GameState *g, int cid, int idx) {
    if (!g) return 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    if (idx < 0 || idx >= nab) return 0;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) return 1;
    int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
    int used = rb_ability_uses_used(g, cid, idx);
    rb_free_ability(&ab);
    return used < limit;
}

/* Mirror abilities.rs::trigger_auto_abilities_for_movement — fire 移動時 autos. */
int rb_trigger_auto_abilities_for_movement(GameState *g, int pl) {
    return rb_trigger_auto_abilities(g, pl, "移動時");
}

/* Mirror abilities.rs::resolve_target_player — map a target string to a player
   index (C has no per-player id strings; defaults master to player1). */
int rb_resolve_target_player(const GameState *g, const char *target) {
    (void)g;
    return rb_target_player_index(target, NULL);
}
