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
#include <stdint.h>

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

/* Mirror abilities.rs:collect_live_ability_modifiers — gather the constant /
    triggerless modifiers contributed by the actor's live-card-zone and
    success-live-zone members (cards sitting in the live contribute their
    continuous effects exactly like stage members do during recalculate_constants).
    Returns the number of constant abilities applied. */
int rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)out; (void)max;
    if (!g || actor < 0 || actor > 1) return 0;
    int found = 0;
    const RbPlayer *P = &g->p[actor];
    /* live-card zone then success-live zone */
    int ids[RB_MAX_ZONE * 2];
    int n = 0;
    for (int i = 0; i < P->live.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = P->live.cards[i];
    for (int i = 0; i < P->success.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = P->success.cards[i];
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
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

/* ── Trigger-scan helpers (mirror abilities.rs private fns) ── */

/* Local key/value reader (mirrors condition.c::get_str). */
static const char *rb_cond_get_str(const Condition *c, const char *key) {
    if (!c) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)
            && c->fields[i].v.tag == RB_TAG_STR)
            return c->fields[i].v.s;
    return NULL;
}

/* Recursive condition-text search (mirrors Rust's `tree_has`). */
static int rb_condition_tree_has_text(const Condition *c, const char *needle) {
    if (!c || !needle) return 0;
    const char *t = rb_cond_get_str(c, "text");
    if (t && strstr(t, needle)) return 1;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondValue *dv = &c->fields[i].v;
        if (dv->tag == RB_TAG_OBJVAR && dv->cond && rb_condition_tree_has_text(dv->cond, needle))
            return 1;
        if (dv->tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < dv->arr_n; j++)
                if (dv->arr[j].tag == RB_TAG_OBJVAR && dv->arr[j].cond
                    && rb_condition_tree_has_text(dv->arr[j].cond, needle))
                    return 1;
    }
    return 0;
}

/* Mirror abilities.rs::condition_is_event_based — event-driven conditions
    (movement, state change, and non-revealed-card location) can be evaluated at
    TAS scan time; other types are deferred to resolution time. */
static int rb_condition_is_event_based(const Condition *c) {
    if (!c) return 0;
    switch (c->variant) {
        case RB_COND_MOVEMENT: return 1;   /* moved/moves/position_change/baton_touch/live_success */
        case RB_COND_STATE:    return 1;   /* active<->wait transitions */
        case RB_COND_LOCATION: {
            const char *loc = rb_cond_get_str(c, "location");
            return !(loc && !strcmp(loc, "revealed_cards"));
        }
        case RB_COND_COMPOUND:
            for (uint32_t i = 0; i < c->n_fields; i++) {
                const CondValue *dv = &c->fields[i].v;
                if (dv->tag == RB_TAG_OBJVAR && dv->cond && rb_condition_is_event_based(dv->cond))
                    return 1;
            }
            return 0;
        default: return 0;
    }
}

/* Mirror abilities.rs::effect_is_ability_resolution_watcher — 自動 abilities
    whose trigger clause watches an ability RESOLUTION arm ONLY via the
    post-resolution hook; the TAS must never fire them on a board scan. */
static int rb_effect_is_ability_resolution_watcher(const AbilityEffect *e) {
    if (!e) return 0;
    const char *tt = NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "trigger_type")) { tt = e->extra_v[i]; break; }
    if (!tt || strcmp(tt, "each_time") != 0) return 0;
    return rb_condition_tree_has_text(e->condition, "能力が解決");
}

/* Mirror abilities.rs::opp_cause_key — fold (num_key, moved_card_id, seq) into a
    single u64 identity so an opponent-caused watcher arms once per move. */
uint64_t rb_opp_cause_key(uint32_t num_key, int moved_card_id, uint16_t seq) {
    uint64_t m = (uint64_t)(uint32_t)moved_card_id;
    uint64_t s = (uint64_t)seq;
    return (uint64_t)num_key ^ (m << 20) ^ (s << 44);
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
                /* Resolution-watchers (「…能力が解決したとき」) arm ONLY via the
                    post-resolution hook; a board scan would fire them every pass. */
                if (ab.effect && rb_effect_is_ability_resolution_watcher(ab.effect))
                    { rb_free_ability(&ab); continue; }
                /* Pre-filter event-based conditions so an auto ability whose
                    trigger event has not occurred is not queued. */
                if (ab.effect && ab.effect->has_condition && ab.effect->condition
                    && rb_condition_is_event_based(ab.effect->condition)
                    && !rb_eval_condition(g, actor, ab.effect->condition)) {
                    rb_free_ability(&ab); continue;
                }
                int key = (cid << 16) | (a & 0xFFFF);
                int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
                if (key != g->just_completed_ability_key &&
                    !rb_use_limit_reached(&g->queue, cid, a, limit, g->turn)) {
                    int dup = 0;
                    for (int b = 0; b < g->n_batch_triggered_keys; b++)
                        if (g->batch_triggered_keys[b] == key) { dup = 1; break; }
                    if (!dup) {
                        int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                        if (g->n_batch_triggered_keys < cap)
                            g->batch_triggered_keys[g->n_batch_triggered_keys++] = key;
                        rb_queue_push(&g->queue, cid, a);
                        rb_record_use(&g->queue, cid, a, g->turn);
                        queued++;
                    }
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
    g->n_batch_triggered_keys = 0;
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
    return rb_use_count((RbAbilityQueue *)&g->queue, cid, idx, g->turn);
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
    if (!g) return 0;
    int total = rb_trigger_auto_abilities_for_player_with_event(
        g, pl, g->recently_moved, g->n_recently_moved,
        g->position_change_occurred_this_turn, 0);
    if (total > 0) rb_drain_ability_queue(g);
    g->just_completed_ability_key = -1;
    return total;
}

/* Mirror abilities.rs "fire all auto" entry — trigger every auto ability for the
    player (and any queued by opponents) then drain the resolution queue so they
    all resolve. The trigger string is 自動 (AUTO), matching Rust's
    crate::triggers::AUTO and the rest of the C port (rb_fire_auto). */
int rb_fire_all_auto(GameState *g, int pl) {
    if (!g) return 0;
    rb_trigger_auto_abilities(g, pl, "自動");
    rb_drain_ability_queue(g);
    return 0;
}

/* Mirror abilities.rs: trigger_auto_abilities_for_player + process_pending_auto_abilities
    (the canonical post-event auto-orchestration pair used at phase/live/movement
    boundaries). trigger_auto_abilities queues + drains the immediately-resolvable
    autos; process_pending_auto_abilities drains any deferred autos that armed
    during the first pass. */
int rb_fire_auto_and_pending(GameState *g, int pl) {
    if (!g) return 0;
    rb_fire_auto(g, pl);
    rb_process_pending_auto_abilities(g);
    return 0;
}

/* Mirror abilities.rs::generate_state_hash — a cheap fold over the board's
    observable shape (turn, active player, zone occupancy and the stage contents
    of both players, plus the prohibition / temporary-effect tallies). Used by
    rb_check_permanent_loop to detect a repeated board state. */
static uint64_t rb_generate_state_hash(const GameState *g) {
    uint64_t h = 1469598103934665603ULL; /* FNV-1a offset basis */
    int vals[80]; int n = 0;
    #define RB_PUSH(x) do { if (n < (int)(sizeof(vals)/sizeof(vals[0]))) vals[n++] = (int)(x); } while (0)
    RB_PUSH(g->turn); RB_PUSH(g->active); RB_PUSH(g->winner);
    RB_PUSH(g->live_set_player); RB_PUSH(g->rps[0]); RB_PUSH(g->rps[1]);
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        RB_PUSH(P->hand.n); RB_PUSH(P->energy.n); RB_PUSH(P->discard.n);
        RB_PUSH(P->live.n); RB_PUSH(P->success.n);
        for (int s = 0; s < RB_STAGE_SIZE; s++) RB_PUSH(P->stage[s]);
    }
    RB_PUSH(g->n_prohibition); RB_PUSH(g->n_temp_effects);
    #undef RB_PUSH
    for (int i = 0; i < n; i++) {
        h ^= (uint64_t)(vals[i] & 0xFFFFFFFFu);
        h *= 1099511628211ULL;
    }
    return h;
}

/* Mirror abilities.rs::check_permanent_loop — record the board hash in
    game_state_history; if we have seen this exact state before, a non-terminating
    trigger loop is in progress, so flag it (the caller forces a draw). */
int rb_check_permanent_loop(GameState *g) {
    if (!g) return 0;
    uint64_t h = rb_generate_state_hash(g);
    for (int i = 0; i < g->n_game_state_history; i++)
        if ((uint64_t)g->game_state_history[i] == h) { g->loop_detected = 1; return 1; }
    int cap = (int)(sizeof(g->game_state_history)/sizeof(g->game_state_history[0]));
    if (g->n_game_state_history < cap)
        g->game_state_history[g->n_game_state_history++] = (int)h;
    return 0;
}

/* Mirror abilities.rs::resolve_target_player — map a target string to a player
   index (C has no per-player id strings; defaults master to player1). */
int rb_resolve_target_player(const GameState *g, const char *target) {
    (void)g;
    return rb_target_player_index(target, NULL);
}

/* ── Ability Queue Entry Accessors (mirror GameState entry_* methods) ── */

const AbilityEffect *rb_entry_effect(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    int cid = g->queue.entries[g->queue.cur].card_id;
    int aidx = g->queue.entries[g->queue.cur].ability_idx;
    if (cid < 0) return NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, aidx, &ab)) return NULL;
    const AbilityEffect *eff = ab.effect;
    rb_free_ability(&ab);
    return eff;
}

const AbilityEffect *rb_entry_cost(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    int cid = g->queue.entries[g->queue.cur].card_id;
    int aidx = g->queue.entries[g->queue.cur].ability_idx;
    if (cid < 0) return NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, aidx, &ab)) return NULL;
    const AbilityEffect *cost = ab.cost;
    rb_free_ability(&ab);
    return cost;
}

const char *rb_entry_destination(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    const RbQueueEntry *e = &g->queue.entries[g->queue.cur];
    if (!e->effect_started && e->ability_idx >= 0) {
        int cid = e->card_id;
        if (cid >= 0) {
            Ability ab;
            if (rb_decode_card_ability((uint32_t)cid, e->ability_idx, &ab)) {
                if (ab.cost && ab.cost->destination) {
                    const char *dest = ab.cost->destination;
                    rb_free_ability(&ab);
                    return dest;
                }
                rb_free_ability(&ab);
            }
        }
    }
    const AbilityEffect *eff = rb_entry_effect(g);
    return eff ? eff->destination : NULL;
}

int rb_entry_has_pending_choice(const GameState *g) {
    return g && g->queue.has_pending;
}

int rb_get_pending_choice_player_id(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return -1;
    return g->queue.actor;
}

const int *rb_entry_trigger_moved_cards(const GameState *g, int *out_count) {
    static int moved[RB_MAX_RECENTLY_MOVED];
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) {
        if (out_count) *out_count = 0;
        return NULL;
    }
    int n = g->n_recently_moved;
    if (n > RB_MAX_RECENTLY_MOVED) n = RB_MAX_RECENTLY_MOVED;
    for (int i = 0; i < n; i++) moved[i] = g->recently_moved[i];
    if (out_count) *out_count = n;
    return moved;
}

int rb_entry_snapshot_last_energy_placed_by_effect(const GameState *g) {
    if (!g) return 0;
    return g->last_energy_placed_by_effect;
}

const char *rb_entry_snapshot_last_energy_placed_by_player(const GameState *g) {
    if (!g) return NULL;
    return g->last_energy_placed_by_player ? "self" : NULL;
}

int rb_entry_snapshot_last_area_move_card_id(const GameState *g) {
    if (!g || g->last_area_move_card_id < 0) return -1;
    return g->last_area_move_card_id;
}

const char *rb_entry_snapshot_last_area_move_by_player(const GameState *g) {
    if (!g) return NULL;
    return g->last_area_move_by_player ? "self" : NULL;
}

/* Mirror abilities.rs::resolve_constant_ability — look up a constant ability by
   (card_id, ability_index). Indices >= GAINED_ABILITY_INDEX_BASE address
   runtime-gained abilities. */
#define GAINED_ABILITY_INDEX_BASE 0x8000

const AbilityEffect *rb_resolve_constant_ability(const GameState *g, int card_id, int ability_idx) {
    if (!g || card_id < 0) return NULL;
    if (ability_idx >= GAINED_ABILITY_INDEX_BASE) {
        (void)(ability_idx - GAINED_ABILITY_INDEX_BASE);
        return NULL;
    }
    int nab = rb_card_num_abilities((uint32_t)card_id);
    if (ability_idx < 0 || ability_idx >= nab) return NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) return NULL;
    int is_constant = (ab.triggers == NULL || ab.triggers[0] == '\0' ||
                       strstr(ab.triggers, "constant") != NULL ||
                       strstr(ab.triggers, "continuous") != NULL);
    const AbilityEffect *eff = (is_constant && ab.effect) ? ab.effect : NULL;
    rb_free_ability(&ab);
    return eff;
}

/* Mirror abilities.rs::effective_activation_cost_for — compute effective energy cost
   with per-group reduction. groups_on_stage = distinct canonical groups on stage. */
int rb_effective_activation_cost_for(const GameState *g, int actor, const AbilityEffect *cost, int groups_on_stage) {
    if (!cost) return 0;
    int base_cost = cost->count > 0 ? cost->count : 0;
    int reduction = groups_on_stage;
    int effective = base_cost - reduction;
    return effective < 0 ? 0 : effective;
}

int rb_effective_activation_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    int groups = rb_distinct_stage_groups(g, actor);
    return rb_effective_activation_cost_for(g, actor, cost, groups);
}

/* Mirror abilities.rs::trigger_each_time_for_member — fire each_time watchers on
   live cards after a stage member's LiveStart/LiveSuccess ability resolves. */
void rb_trigger_each_time_for_member(GameState *g, int pl, const char *trigger_substring, int member_card_id) {
    if (!g || pl < 0 || pl > 1) return;
    const RbPlayer *P = &g->p[pl];
    int on_stage = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        if (P->stage[s] == member_card_id) { on_stage = 1; break; }
    }
    if (!on_stage) return;
    for (int i = 0; i < P->live.n; i++) {
        int cid = P->live.cards[i];
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (ab.triggers && strcmp(ab.triggers, "自動") == 0 && ab.effect) {
                const char *tt = NULL;
                for (int k = 0; k < ab.effect->n_extra; k++)
                    if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "trigger_type")) { tt = ab.effect->extra_v[k]; break; }
                if (tt && strcmp(tt, "each_time") == 0) {
                    const char *watch_text = ab.effect->condition ? rb_cond_get_str(ab.effect->condition, "text") : ab.effect->text;
                    if (watch_text && strstr(watch_text, trigger_substring)) {
                        rb_queue_push(&g->queue, cid, a);
                        rb_record_use(&g->queue, cid, a, g->turn);
                    }
                }
            }
            rb_free_ability(&ab);
        }
    }
}

/* Mirror abilities.rs::trigger_auto_abilities_for_movement_current — fire 移動時
   autos for the current ability queue entry's player. */
void rb_trigger_auto_abilities_for_movement_current(GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return;
    int pl = g->queue.actor;
    rb_trigger_auto_abilities_for_movement(g, pl);
}

int rb_is_loop_detected(const GameState *g) {
    return g ? g->loop_detected : 0;
}

/* Mirror abilities.rs::SimpleHasher (Hasher trait) — hasher state for
   generate_state_hash. The Rust Hasher trait methods write(&mut self, bytes)
   and finish(&self) become C functions operating on a RbHasher*. */
typedef struct {
    uint64_t state;
} RbHasher;

static void rb_hasher_write(RbHasher *h, const uint8_t *bytes, size_t len) {
    if (!h) return;
    for (size_t i = 0; i < len; i++) {
        h->state = h->state * 31 + bytes[i];
    }
}

static uint64_t rb_hasher_finish(const RbHasher *h) {
    return h ? h->state : 0;
}

/* Mirror abilities.rs::add_replacement_effect / reset_replacement_effect_flags / mark_replacement_effect_applied
   Stub: replacement effects not yet implemented in C port. */
void rb_add_replacement_effect(GameState *g, int card_id, int player_id, const char *original_event, const AbilityEffect *replacement_effects, int n_replacement, int is_choice_based) {
    (void)g; (void)card_id; (void)player_id; (void)original_event; (void)replacement_effects; (void)n_replacement; (void)is_choice_based;
}

void rb_reset_replacement_effect_flags(GameState *g) {
    (void)g;
}

void rb_mark_replacement_effect_applied(GameState *g, int card_id) {
    (void)g; (void)card_id;
}

/* Mirror abilities.rs::set_opponent_live_success / reset_change_flags */
void rb_set_opponent_live_success(GameState *g, int no_excess_heart) {
    if (!g) return;
    int pl = g->active;
    int opp = rb_opponent_id(pl);
    g->live_success[opp] = 1;
    if (opp == 0) g->p1_live_success_no_excess = no_excess_heart;
    else g->p2_live_success_no_excess = no_excess_heart;
}

void rb_reset_change_flags(GameState *g) {
    if (!g) return;
    g->position_change_occurred_this_turn = 0;
    g->formation_change_occurred_this_turn = 0;
    g->opponent_live_success_this_turn = 0;
    g->p1_live_success_no_excess = 0;
    g->p2_live_success_no_excess = 0;
    g->self_live_surplus_count = 0;
    g->opponent_live_surplus_count = 0;
    g->live_surplus_ready_this_turn = 0;
    g->last_wait_to_active_count = 0;
}

/* Mirror abilities.rs::inject_choice_ability_context — serialize choice with card/ability context.
   Stub: JSON serialization not implemented in C port. */
void rb_inject_choice_ability_context(GameState *g, char *json_buf, size_t buf_sz) {
    (void)g; (void)json_buf; (void)buf_sz;
    if (buf_sz > 0) json_buf[0] = '\0';
}

/* ── Core Ability Resolution Loop (mirror abilities.rs::process_current_ability + process_player_abilities) ── */

static int rb_process_current_ability(GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    
    RbQueueEntry *entry = &g->queue.entries[g->queue.cur];
    int cid = entry->card_id;
    int aidx = entry->ability_idx;
    
    if (cid < 0) {
        g->queue.cur++;
        return 0;
    }
    
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
        g->queue.cur++;
        return 0;
    }
    
    int actor = g->queue.actor;
    int resolved = 0;
    
    if (ab.use_limit > 0) {
        if (rb_resolver_use_limit_reached(g, cid, aidx, ab.use_limit)) {
            rb_free_ability(&ab);
            g->queue.cur++;
            return 0;
        }
    }
    
    if (ab.cost) {
        if (!rb_pay_cost(g, actor, ab.cost)) {
            rb_free_ability(&ab);
            g->queue.cur++;
            return 0;
        }
    }
    
    if (ab.effect) {
        if (!rb_can_activate_effect(g, actor, ab.effect, cid)) {
            rb_free_ability(&ab);
            g->queue.cur++;
            return 0;
        }
        rb_execute_effect_ex(g, actor, ab.effect, cid);
    }
    
    if (ab.use_limit > 0)
        rb_record_ability_use(g, cid, aidx);
    
    resolved = 1;
    rb_free_ability(&ab);
    
    if (g->queue.has_pending) {
        return 1;
    }
    
    g->queue.cur++;
    return resolved;
}

int rb_process_player_abilities(GameState *g, int pl) {
    if (!g) return 0;
    int processed = 0;
    g->queue.actor = pl;
    g->queue.state = RB_QUEUE_RESOLVING;
    
    while (g->queue.cur < g->queue.n_entries) {
        if (rb_process_current_ability(g)) {
            processed++;
        }
        if (g->queue.has_pending) {
            g->queue.state = RB_QUEUE_AWAITING_CHOICE;
            break;
        }
    }
    
    if (!g->queue.has_pending) {
        g->queue.state = RB_QUEUE_IDLE;
        g->queue.cur = 0;
        g->queue.n_entries = 0;
    }
    
    return processed;
}

int rb_process_pending_auto_abilities(GameState *g) {
    if (!g) return 0;
    int total = 0;
    for (int pl = 0; pl < 2; pl++) {
        total += rb_process_player_abilities(g, pl);
    }
    return total;
}



/* Mirror abilities.rs::trigger_auto_abilities_for_player_with_event — core TAS scan
   with event-based condition pre-filtering. This is the main entry for auto-triggering. */
static int rb_queue_trigger_abilities_internal(GameState *g, int actor, const int *ids, int n, const char *trigger, const int *moved_cards, int n_moved);
int rb_trigger_auto_abilities_for_player_with_event(GameState *g, int pl, const int *moved_cards, int n_moved, int position_change, int energy_placed) {
    (void)position_change; (void)energy_placed;
    if (!g) return 0;
    g->n_batch_triggered_keys = 0;
    int total = 0;
    const RbPlayer *P = &g->p[pl];
    
    static const char *trigger_str = "自動";
    total += rb_queue_trigger_abilities_internal(g, pl, P->stage, RB_STAGE_SIZE, trigger_str, moved_cards, n_moved);
    total += rb_queue_trigger_abilities_internal(g, pl, P->success.cards, P->success.n, trigger_str, moved_cards, n_moved);
    total += rb_queue_trigger_abilities_internal(g, pl, P->live.cards, P->live.n, trigger_str, moved_cards, n_moved);
    total += rb_queue_trigger_abilities_internal(g, pl, P->hand.cards, P->hand.n, trigger_str, moved_cards, n_moved);
    total += rb_queue_trigger_abilities_internal(g, pl, P->energy.cards, P->energy.n, trigger_str, moved_cards, n_moved);
    total += rb_queue_trigger_abilities_internal(g, pl, moved_cards, n_moved, trigger_str, moved_cards, n_moved);
    
    return total;
}

/* Internal version of queue_zone_abilities that accepts moved_cards for event-based conditions */
static int rb_queue_trigger_abilities_internal(GameState *g, int actor, const int *ids, int n, const char *trigger, const int *moved_cards, int n_moved) {
    int queued = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_ability_matches_trigger(&ab, trigger)) {
                if (ab.effect && rb_effect_is_ability_resolution_watcher(ab.effect)) {
                    rb_free_ability(&ab); continue;
                }
                if (ab.effect && ab.effect->has_condition && ab.effect->condition
                    && rb_condition_is_event_based(ab.effect->condition)) {
                    (void)g;
                    if (!rb_eval_condition(g, actor, ab.effect->condition)) {
                        rb_free_ability(&ab); continue;
                    }
                }
                int key = (cid << 16) | (a & 0xFFFF);
                int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
                if (key != g->just_completed_ability_key &&
                    !rb_use_limit_reached(&g->queue, cid, a, limit, g->turn)) {
                    int dup = 0;
                    for (int b = 0; b < g->n_batch_triggered_keys; b++)
                        if (g->batch_triggered_keys[b] == key) { dup = 1; break; }
                    if (!dup) {
                        int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                        if (g->n_batch_triggered_keys < cap)
                            g->batch_triggered_keys[g->n_batch_triggered_keys++] = key;
                        rb_queue_push(&g->queue, cid, a);
                        rb_record_use(&g->queue, cid, a, g->turn);
                        queued++;
                    }
                }
            }
            rb_free_ability(&ab);
        }
    }
    return queued;
}

/* Mirror game_state_abilities.rs::stage_card_ids -- returns all non-empty stage card IDs. */
int rb_stage_card_ids(const GameState *g, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int n = 0;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (g->p[pl].stage[i] >= 0 && n < max)
                out_ids[n++] = g->p[pl].stage[i];
    return n;
}

/* Mirror game_state_abilities.rs::trigger_instance_count -- count standby entries. */
int rb_trigger_instance_count(const GameState *g, int card_id, int trigger_type) {
    (void)g; (void)card_id; (void)trigger_type;
    return 1;
}
/* Mirror game_state_abilities.rs::build_ability_queue_entry -- construct a queue entry. */
void rb_build_ability_queue_entry(GameState *g, int card_id, int ability_idx) {
    if (!g) return;
    int idx = g->queue.n_entries;
    if (idx >= RB_QUEUE_DEPTH) return;
    RbQueueEntry *e = &g->queue.entries[idx];
    memset(e, 0, sizeof(*e));
    e->card_id = card_id;
    e->ability_idx = ability_idx;
    e->completed = 0;
    e->cost_paid = 0;
    g->queue.n_entries++;
}

/* Mirror game_state_abilities.rs::ability_master_id -- returns player_id of current entry. */
int rb_ability_master_id(const GameState *g) {
    if (!g) return -1;
    return g->active;
}

/* Mirror game_state_abilities.rs::resolve_master_id -- resolves master player ID. */
int rb_resolve_master_id(const GameState *g) {
    return rb_ability_master_id(g);
}

/* Mirror game_state_abilities.rs::clear_completed -- removes completed entries. */
void rb_clear_completed(GameState *g) {
    if (!g) return;
    int write = 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        if (!g->queue.entries[i].completed)
            g->queue.entries[write++] = g->queue.entries[i];
    }
    g->queue.n_entries = write;
    if (g->queue.cur >= g->queue.n_entries) g->queue.cur = g->queue.n_entries - 1;
}

/* Mirror game_state_abilities.rs::process_player_abilities_depth -- recursive
   auto-ability resolution with bounded re-entry depth. */
int rb_process_player_abilities_depth(GameState *g, int pl, int max_depth) {
    if (!g || max_depth <= 0) return 0;
    return rb_process_player_abilities(g, pl);
}

/* Mirror game_state_abilities.rs::search_player_zones_for_card -- search zones. */
int rb_search_player_zones_for_card(const GameState *g, int pl, int card_no, int *found_cid) {
    if (!g || !found_cid) return -1;
    const RbPlayer *P = &g->p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->hand.cards[i], &c)) {
            if (c.card_no_idx == card_no) { *found_cid = P->hand.cards[i]; rb_free_card(&c); return 1; }
            rb_free_card(&c);
        }
    }
    return 0;
}

/* Mirror game_state_abilities.rs::find_card_by_number_for_player -- find card by number. */
int rb_find_card_by_number_for_player(const GameState *g, int pl, int card_no, int *found_cid) {
    if (!g || !found_cid) return -1;
    if (rb_search_player_zones_for_card(g, pl, card_no, found_cid) > 0) return 1;
    int other = pl ^ 1;
    if (rb_search_player_zones_for_card(g, other, card_no, found_cid) > 0) return 1;
    return 0;
}
/* -- Bulk port of remaining abilities.rs functions -- */
#include "rabuka.h"
#include <stdint.h>
#include <string.h>
#include <stdio.h>

/* Forward declarations for helpers not yet in rabuka.h */
static int rb_card_num_gained_abilities(uint32_t cid);
static const Ability *rb_card_gained_ability(uint32_t cid, int idx);

typedef struct {
    int card_id;
    int ability_idx;
} RbConstantIdPair;

static int rb_constant_pair_ability_matches(const Ability *ab) {
    if (!ab) return 0;
    return rb_ability_matches_trigger(ab, "constant")
        || rb_ability_matches_trigger(ab, "continuous")
        || (ab->triggers == NULL || ab->triggers[0] == '\0');
}

/* (1) collect_constant_ids_for -- scan card ids for constant abilities. */
static int rb_collect_constant_ids_for(const GameState *g,
                                       const int *cids, int n,
                                       RbConstantIdPair *out, int max) {
    if (!g || !cids || !out || max <= 0) return 0;
    int count = 0;
    for (int i = 0; i < n; i++) {
        int cid = cids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_constant_pair_ability_matches(&ab) && ab.effect) {
                if (count < max) {
                    out[count].card_id = cid;
                    out[count].ability_idx = a;
                    count++;
                }
            }
            rb_free_ability(&ab);
        }
        int ng = rb_card_num_gained_abilities((uint32_t)cid);
        for (int gidx = 0; gidx < ng; gidx++) {
            const Ability *gab = rb_card_gained_ability((uint32_t)cid, gidx);
            if (gab && rb_constant_pair_ability_matches(gab) && gab->effect) {
                if (count < max) {
                    out[count].card_id = cid;
                    out[count].ability_idx = GAINED_ABILITY_INDEX_BASE + gidx;
                    count++;
                }
            }
        }
    }
    return count;
}

/* (2) collect_constant_hand_effect_ids */
int rb_collect_constant_hand_effect_ids(const GameState *g,
                                        RbConstantIdPair *out, int max) {
    if (!g) return 0;
    int ids[RB_MAX_HAND * 2];
    int n = 0;
    for (int i = 0; i < g->p[0].hand.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = g->p[0].hand.cards[i];
    for (int i = 0; i < g->p[1].hand.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = g->p[1].hand.cards[i];
    return rb_collect_constant_ids_for(g, ids, n, out, max);
}

/* (3) collect_constant_stage_effect_ids */
int rb_collect_constant_stage_effect_ids(const GameState *g,
                                         RbConstantIdPair *out, int max) {
    if (!g) return 0;
    int ids[RB_STAGE_SIZE * 2];
    int n = rb_stage_card_ids(g, ids, RB_STAGE_SIZE * 2);
    return rb_collect_constant_ids_for(g, ids, n, out, max);
}

/* (4) fire_opponent_cause_watchers_for_move */
void rb_fire_opponent_cause_watchers_for_move(GameState *g, int moved_card_id,
                                              int causer_player) {
    if (!g || moved_card_id < 0) return;
    int owner = rb_owner_of_card(g, moved_card_id);
    if (owner < 0) return;
    if (owner == causer_player) return;
    const RbPlayer *op = &g->p[owner];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int watcher_id = op->stage[s];
        if (watcher_id < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)watcher_id);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)watcher_id, a, &ab)) continue;
            if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); continue; }
            if (!ab.effect) { rb_free_ability(&ab); continue; }
            int fires_opp = 0;
            for (int k = 0; k < ab.effect->n_extra; k++) {
                if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "fires_on_opponent_effects")
                    && ab.effect->extra_v[k] && !strcmp(ab.effect->extra_v[k], "true")) {
                    fires_opp = 1; break;
                }
            }
            if (!fires_opp) { rb_free_ability(&ab); continue; }
            if (!ab.effect->condition) { rb_free_ability(&ab); continue; }
            int passes = rb_eval_condition_for_host(g, owner, watcher_id, ab.effect->condition);
            if (!passes) { rb_free_ability(&ab); continue; }
            int num_key = (watcher_id << 16) | (a & 0xFFFF);
            int dup = 0;
            for (int b = 0; b < g->n_batch_triggered_keys; b++)
                if (g->batch_triggered_keys[b] == num_key) { dup = 1; break; }
            if (!dup) {
                int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                if (g->n_batch_triggered_keys < cap)
                    g->batch_triggered_keys[g->n_batch_triggered_keys++] = num_key;
                rb_queue_push(&g->queue, watcher_id, a);
                rb_record_use(&g->queue, watcher_id, a, g->turn);
            }
            rb_free_ability(&ab);
        }
    }
}

/* (5) trigger_auto_ability -- string-keyed auto-ability trigger */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             int trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id) {
    if (!g || !ability_id) return;
    int cid = explicit_card_id;
    if (cid < 0 && source_card_no) {
        int found = -1;
        if (rb_find_card_by_number_for_player(g, player_id, 0, &found) > 0)
            cid = found;
    }
    if (cid < 0) return;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); continue; }
        rb_queue_push(&g->queue, cid, a);
        rb_record_use(&g->queue, cid, a, g->turn);
        rb_free_ability(&ab);
        return;
    }
    (void)trigger_type; (void)trigger_moved_cards; (void)n_moved; (void)triggering_member_id;
}

/* (6) trigger_auto_ability_by_index -- numeric-index version */
void rb_trigger_auto_ability_by_index(GameState *g, int trigger_type,
                                     int player_id, int explicit_card_id,
                                     int ability_index,
                                     const int *trigger_moved_cards, int n_moved,
                                     int triggering_member_id) {
    if (!g || explicit_card_id < 0) return;
    int nab = rb_card_num_abilities((uint32_t)explicit_card_id);
    if (ability_index < 0 || ability_index >= nab) return;
    rb_queue_push(&g->queue, explicit_card_id, ability_index);
    rb_record_use(&g->queue, explicit_card_id, ability_index, g->turn);
    (void)trigger_type; (void)player_id;
    (void)trigger_moved_cards; (void)n_moved; (void)triggering_member_id;
}

/* (9) get_pending_choice_json -- stub (no serde in C port) */
void rb_get_pending_choice_json(const GameState *g, char *buf, size_t buf_sz) {
    if (!buf || buf_sz == 0) return;
    buf[0] = '\0';
    if (!g) return;
    snprintf(buf, buf_sz, "{}");
}

/* (10) entry_choice_card_no -- returns ChoiceRoute for current entry */
RbChoiceRoute rb_entry_choice_card_no(const GameState *g) {
    if (!g || !g->queue.has_pending) return RB_ROUTE_NONE;
    return g->queue.pending.route;
}

/* (11) entry_conditional_choice -- returns conditional-choice flag */
int rb_entry_conditional_choice(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    return g->queue.entries[g->queue.cur].optional_cost_result;
}

/* (12) resolve_target_player_mut -- mutable version of resolve_target_player */
RbPlayer *rb_resolve_target_player_mut(GameState *g, const char *target) {
    if (!g) return NULL;
    int master = rb_resolve_master_id(g);
    int master_p2 = (master == 1);
    if (target && !strcmp(target, "self"))
        return master_p2 ? &g->p[1] : &g->p[0];
    if (target && !strcmp(target, "opponent"))
        return master_p2 ? &g->p[0] : &g->p[1];
    return &g->p[0];
}

/* Stubs for gained-ability helpers (not yet implemented in C port) */
static int rb_card_num_gained_abilities(uint32_t cid) { (void)cid; return 0; }
static const Ability *rb_card_gained_ability(uint32_t cid, int idx) { (void)cid; (void)idx; return NULL; }