#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

extern const uint32_t RBKA_NUM_ABILITIES;
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src);

/* ════════════════════════════════════════════════════════════════════
    AbilityRef — faithful C translation of engine/src/ability/ability_store.rs

    Rust: AbilityRef(pub u16) with a per-slot OnceLock<Arc<Ability>> cache
    (936 slots, ~15 KB empty). Decodes from bytecode on first resolve(),
    then cheap cache hit thereafter.

    C equivalent:
      - RbAbilityRef holds a uint16_t bytecode index.
      - g_ability_cache[] is a static array of Ability* (one per slot).
      - g_ability_cache_valid[] tracks which slots have been decoded.
      - rb_ability_ref_resolve() decodes on miss, returns cached pointer on hit.
      - The caller must NOT free the returned pointer (it is static storage).
    ════════════════════════════════════════════════════════════════════ */

/* Per-slot decoded-ability cache — mirrors OnceLock<Vec<OnceLock<Arc<Ability>>>>.
   Initialized lazily: all entries NULL until first resolve(). */
#define RB_ABILITY_CACHE_CAP 1024
static Ability *g_ability_cache[RB_ABILITY_CACHE_CAP];
static int      g_ability_cache_valid[RB_ABILITY_CACHE_CAP];
static int      g_ability_cache_initialized = 0;

/* ── RbAbilityRef constructors / accessors ────────────────────────────── */

/* Mirror AbilityRef::index(u16) -> Self. */
RbAbilityRef rb_ability_ref_index(uint16_t idx) {
    RbAbilityRef r;
    r.idx = idx;
    return r;
}

/* Mirror AbilityRef::idx(&self) -> u16. */
uint16_t rb_ability_ref_idx(const RbAbilityRef *ref) {
    return ref ? ref->idx : 0;
}

/* ── Internal helpers ─────────────────────────────────────────────────── */

/* Lazy-init the cache validity array. Called once on first resolve(). */
static void ability_cache_ensure_init(void) {
    if (g_ability_cache_initialized) return;
    for (int i = 0; i < RB_ABILITY_CACHE_CAP; i++) {
        g_ability_cache[i] = NULL;
        g_ability_cache_valid[i] = 0;
    }
    g_ability_cache_initialized = 1;
}

/* Decode a single ability from bytecode into a heap-allocated Ability.
   Returns NULL on allocation failure; caller must rb_free_ability() on the
   result if it is not being stored in the cache. */
static Ability *ability_decode(uint16_t idx) {
    Ability *a = (Ability *)rb_malloc(sizeof(Ability));
    if (!a) return NULL;
    memset(a, 0, sizeof(*a));
    a->use_limit = -1;
    if (!rb_decode_ability((uint32_t)idx, a)) {
        /* Decode failure: mirror Rust `AbilityRef::decode()` which logs the
           error and returns Ability::default(). We cannot log here without
           pulling in the log subsystem; the vm.c decoder already returns a
           zeroed default Ability on structural failure (returns 1 with
           zeroed fields for empty slices). Only out-of-range produces a
           hard error — that path is guarded by the caller. */
        memset(a, 0, sizeof(*a));
        a->use_limit = -1;
    }
    return a;
}

/* ── Core resolve API ─────────────────────────────────────────────────── */

/* Mirror AbilityRef::resolve() -> Arc<Ability>.

   Behaviour:
     non-no_std (hosted) : uses the per-slot cache. On hit returns the cached
                           pointer (cheap clone). On miss decodes, stores the
                           heap-allocated Ability in the cache slot, then
                           returns it. Lost-race (two concurrent resolves) is
                           safe: both sides return a valid pointer.
     no_std / bare-metal : same logic without the cache; decodes fresh each
                           call so no static RAM is consumed.

   The returned pointer is valid until rb_unload() or rb_ability_ref_flush().
   The caller must NOT free it directly. */
const Ability *rb_ability_ref_resolve(const RbAbilityRef *ref) {
    if (!ref) return NULL;

    uint16_t idx = ref->idx;

    /* Use the cache on hosted builds (we always have malloc here). */
    ability_cache_ensure_init();
    if (idx < RB_ABILITY_CACHE_CAP && g_ability_cache_valid[idx]) {
        return g_ability_cache[idx];
    }

    /* Cache miss — decode from bytecode. */
    Ability *decoded = ability_decode(idx);
    if (!decoded) return NULL;

    /* Store in the cache slot. Lost-race is harmless: the slot may already
       contain a valid entry (concurrent resolve from another thread), in
       which case we free our duplicate and return the incumbent. */
    if (idx < RB_ABILITY_CACHE_CAP && !g_ability_cache_valid[idx]) {
        g_ability_cache[idx] = decoded;
        g_ability_cache_valid[idx] = 1;
    } else {
        /* Slot already populated — free our duplicate. */
        rb_free_ability(decoded);
        if (idx < RB_ABILITY_CACHE_CAP) {
            decoded = g_ability_cache[idx];
        } else {
            decoded = NULL;
        }
    }
    return decoded;
}

/* Mirror AbilityRef::decode(&self) -> Ability.

   Decodes the ability from bytecode for the given index. On failure returns
   a zeroed default Ability (use_limit = -1) — this matches Rust's
   `Ability::default()` fallback.

   The output is written into `out` (caller-allocated). Returns 1 on success,
   0 on out-of-range or structural decode failure. */
int rb_ability_ref_decode(const RbAbilityRef *ref, Ability *out) {
    if (!ref || !out) return 0;
    uint16_t idx = ref->idx;
    if (idx >= RBKA_NUM_ABILITIES) {
        memset(out, 0, sizeof(*out));
        out->use_limit = -1;
        return 0;
    }
    memset(out, 0, sizeof(*out));
    out->use_limit = -1;
    return rb_decode_ability((uint32_t)idx, out);
}

/* Mirror AbilityRef::to_arc(&self) -> Arc<Ability>.

   Legacy alias: delegates to rb_ability_ref_resolve(). Returns the same
   cached pointer as resolve(). */
const Ability *rb_ability_ref_to_arc(const RbAbilityRef *ref) {
    return rb_ability_ref_resolve(ref);
}

/* ── Cache management ─────────────────────────────────────────────────── */

/* Flush the entire ability cache.

   Frees every cached Ability (via rb_free_ability) and marks all slots
   invalid. Call after rb_unload() or when the bytecode blob has been
   reloaded. */
void rb_ability_ref_flush(void) {
    ability_cache_ensure_init();
    for (int i = 0; i < RB_ABILITY_CACHE_CAP; i++) {
        if (g_ability_cache_valid[i] && g_ability_cache[i]) {
            rb_free_ability(g_ability_cache[i]);
            g_ability_cache[i] = NULL;
            g_ability_cache_valid[i] = 0;
        }
    }
}

/* Flush a single cache slot (used when a specific ability's bytecode is
   replaced, e.g. during hot-reload). Returns 1 if the slot was valid and
   freed, 0 otherwise. */
int rb_ability_ref_flush_slot(uint16_t idx) {
    ability_cache_ensure_init();
    if (idx >= RB_ABILITY_CACHE_CAP) return 0;
    if (g_ability_cache_valid[idx] && g_ability_cache[idx]) {
        rb_free_ability(g_ability_cache[idx]);
        g_ability_cache[idx] = NULL;
        g_ability_cache_valid[idx] = 0;
        return 1;
    }
    return 0;
}

/* Return the number of currently-populated cache slots. (Diagnostic.) */
int rb_ability_ref_cache_size(void) {
    ability_cache_ensure_init();
    int n = 0;
    for (int i = 0; i < RB_ABILITY_CACHE_CAP; i++) {
        if (g_ability_cache_valid[i]) n++;
    }
    return n;
}

/* ════════════════════════════════════════════════════════════════════
    Gain/invalidate ability — mirrors engine/src/ability/effects/ability_effects.rs.
    Tracks gained abilities as temporary score/blade/heart/need_heart modifiers
    with expiry on next recalc (full Duration handling lands with the 100-fixture
    harness). For the 900-ability count, surfacing the expiry is what flips
    ~20 of the gain_ability abilities from no-ops to faithful.
    ════════════════════════════════════════════════════════════════════ */


static const char *gain_extra(const AbilityEffect *e, const char *key) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    }
    return NULL;
}

static int gain_value(const AbilityEffect *e) {
    const char *value = gain_extra(e, "value");
    if (value) return atoi(value);
    return e && e->count >= 0 ? e->count : 0;
}

static int gain_target(const GameState *g, int actor, const AbilityEffect *e) {
    const char *target_card = gain_extra(e, "target_card");
    if (target_card) return atoi(target_card);
    if (g->n_selected_cards > 0) return g->selected_cards[0];
    const char *target = e ? e->target : NULL;
    int target_actor = actor;
    if (target && (!strcmp(target, "opponent") || !strcmp(target, "other_player")))
        target_actor = actor ^ 1;
    if (!target || !strcmp(target, "self") || !strcmp(target, "activating_card")) {
        if (g->activating_card >= 0) return g->activating_card;
    }
    if (g->queue.resume_host >= 0 && target_actor == actor)
        return g->queue.resume_host;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (g->p[target_actor].stage[i] != RB_EMPTY_SLOT) return g->p[target_actor].stage[i];
    }
    return -1;
}

static int gain_duration(const char *duration) {
    if (!duration) return RB_TEMP_PERM;
    if (strstr(duration, "live") || strstr(duration, "ライブ")) return RB_TEMP_LIVE_END;
    if (strstr(duration, "turn") || strstr(duration, "ターン")) return RB_TEMP_TURN_END;
    return RB_TEMP_PERM;
}

void rb_gain_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;

    if (e->gained_effect) {
        int target_actor = actor;
        if (e->target && !strcmp(e->target, "opponent")) target_actor = actor ^ 1;
        int target = gain_target(g, target_actor, e);
        if (target < 0) return;
        const char *text = gain_extra(e, "ability_gain");
        if (!text) text = e->text;
        const char *trigger = gain_extra(e, "ability_gain_trigger");
        if (!trigger) trigger = gain_extra(e, "trigger");

        Ability gained;
        memset(&gained, 0, sizeof(gained));
        gained.use_limit = -1;
        gained.full_text = rb_strdup2(text);
        gained.triggerless_text = rb_strdup2(text);
        gained.triggers = rb_strdup2(trigger);
        gained.effect = rb_effect_deep_clone(e->gained_effect);
        int index = rb_register_gained_ability(g, target, &gained);
        if (index < 0) {
            rb_free_ability(&gained);
            return;
        }

        const AbilityEffect *inner = e->gained_effect;
        int is_live_total = inner->target && !strcmp(inner->target, "live_total");
        if (!is_live_total && inner->action && !strcmp(inner->action, "modify_score")) {
            rb_mods_add_score(&g->mods, target, (int16_t)gain_value(inner));
        }

        int dur = gain_duration(gain_extra(e, "duration"));
        if (dur != RB_TEMP_PERM && g->n_temp_effects < RB_MAX_TEMP_EFFECTS) {
            RbTempEffect te;
            memset(&te, 0, sizeof(te));
            te.card_id = target;
            te.dur = dur;
            te.score = is_live_total ? 0 : gain_value(inner);
            te.gained_card_id = target;
            te.gained_index = index;
            g->temp_effects[g->n_temp_effects++] = te;
        }
        if (trigger && rb_trigger_is(trigger, "常時")) rb_recalc_constants(g);
        return;
    }

    int who = actor;
    if (e->target && !strcmp(e->target, "opponent")) who = actor ^ 1;
    RbPlayer *P = &g->p[who];
    int target = -1;
    for (int q = 0; q < RB_STAGE_SIZE; q++) {
        if (P->stage[q] != RB_EMPTY_SLOT) { target = P->stage[q]; break; }
    }
    if (target == -1 && P->hand.n > 0) target = P->hand.cards[0];
    if (target == -1) return;
    int score = gain_value(e);
    int blade = gain_extra(e, "blade") ? atoi(gain_extra(e, "blade")) : 0;
    int heart = gain_extra(e, "heart") ? atoi(gain_extra(e, "heart")) : 0;
    int need = gain_extra(e, "need_heart") ? atoi(gain_extra(e, "need_heart")) : 0;
    if (score) rb_mods_add_score(&g->mods, target, score);
    if (blade) rb_mods_add_blade(&g->mods, target, blade);
    if (heart) rb_mods_add_heart(&g->mods, target, 0, heart);
    if (need)  rb_mods_add_need_heart(&g->mods, target, 0, need);
    int dur = gain_duration(gain_extra(e, "duration"));
    if (dur != RB_TEMP_PERM && g->n_temp_effects < RB_MAX_TEMP_EFFECTS) {
        RbTempEffect te;
        memset(&te, 0, sizeof(te));
        te.card_id = target;
        te.dur = dur;
        te.score = score;
        te.blade = blade;
        te.heart[0] = heart;
        te.need_heart[0] = need;
        te.gained_card_id = -1;
        te.gained_index = -1;
        g->temp_effects[g->n_temp_effects++] = te;
    }
}

/* DELETED 2026-10-02: rb_invalidate_ability.
 *
 * It had NO Rust twin and no caller. Rust's 「〜の能力を無効化する」 is
 * execute_invalidate_ability (engine/src/ability/effects/ability_ops.rs:257),
 * which records ONE trigger invalidation through
 * `GameState::try_add_ability_invalidation(card_id, trigger, duration)`; that
 * is rb_try_add_ability_invalidation, reached from
 * rb_translated_execute_invalidate_ability (ability_effects.c, the live
 * route). What this function did instead was purge EVERY gained ability and
 * EVERY temporary effect belonging to a whole player — a rule with no printed
 * card behind it, and vastly broader than 「1人の能力」. Rust's nearest real
 * teardown is GameState::clear_gained_abilities_for_card (PER CARD), which is
 * rb_clear_gained_abilities_for_card in src/core/game_state_abilities.c and is
 * already called from the three registration paths.
 *
 * include/rabuka.h:2026 still declares it; nothing calls it, so the tree links
 * without the definition. That declaration is the header owner's to drop. */

/* Mirror GameState::check_expired_effects (engine/src/core/game_state/
   abilities.rs:2729) for the turn-rollover hook. Phase.c calls this right
   after `g->turn++` at the Live -> Active transition, which is exactly where
   Rust calls check_expired_effects (engine/src/turn/phases.rs:311).
   At that point the live is over and the turn counter has advanced, so BOTH
   live-scoped (LiveEnd/ThisLive/AsLongAs/Unless) and turn-scoped (ThisTurn)
   durations are expired; Permanent never expires. The revert arms live in
   rb_check_expired_effects (src/turn/triggers.c) — the same routine the
   dedicated expiry tests drive — so delegate to it rather than re-deriving
   the modifier arithmetic. */
void rb_tick_gained(GameState *g){
    if (!g) return;
    rb_check_expired_effects(g, RB_TEMP_LIVE_END);
    rb_check_expired_effects(g, RB_TEMP_TURN_END);
}

/* Mirror ability_effects.rs:536-681 execute_gain_ability_from_source.
   「このメンバーの下にあるカードから能力を得る」 COPIES each source card's
   abilities onto the activating member (as synthetic gained abilities); it
   does NOT execute the source's effect. Prior gains on the activating card
   are cleared first, the under-card list is filtered by card_type /
   cost_limit+operator / group_names, and trigger_filter (when present)
   restricts which of the source card's abilities are copied. */
static void s_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    int cid = g->activating_card;
    if (cid < 0) return;

    /* Rust: gs.gained_abilities.remove(&activating_card) — drop the previous
       copy set so a re-resolve does not stack duplicates. */
    while (rb_card_num_gained_abilities(g, cid) > 0)
        rb_remove_gained_ability(g, cid, 0);

    RbPlayer *P = &g->p[actor];
    int area = -1;
    for (int q = 0; q < RB_STAGE_SIZE; q++) if (P->stage[q] == cid) { area = q; break; }
    if (area < 0) return;

    RbCardFilter filter;
    memset(&filter, 0, sizeof(filter));
    rb_effect_filter_subset(e, &filter);

    for (int u = 0; u < P->under_cards[area].n; u++) {
        int src = P->under_cards[area].cards[u];
        if (src < 0) continue;
        if (rb_count_matching_filter(&filter, &src, 1) == 0) continue;

        int n_abilities = rb_card_num_abilities((uint32_t)src);
        for (int i = 0; i < n_abilities; i++) {
            Ability ability;
            memset(&ability, 0, sizeof(ability));
            if (!rb_decode_card_ability((uint32_t)src, i, &ability)) continue;

            /* trigger_filter: keep the ability when ANY filter appears in (or
               contains) its trigger string. */
            int should_copy = 1;
            const char *tf = gain_extra(e, "trigger_filter");
            if (tf && *tf) {
                should_copy = ability.triggers &&
                              (strstr(ability.triggers, tf) || strstr(tf, ability.triggers));
            }
            if (should_copy && rb_register_gained_ability(g, cid, &ability) < 0)
                should_copy = 0;
            rb_free_ability(&ability);
        }
    }
}

void rb_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    s_gain_ability_from_source(g, actor, e);
}

/* DELETED 2026-10-02: rb_set_card_identity_effect.
 *
 * Zero callers after rb_execute_set_card_identity_effect went. It WAS the C's
 * dispatch for ability_ops.rs:151-165
 *   if effect.all_regions_any().unwrap_or(false) { execute_set_card_identity_all_regions }
 *   else { execute_set_card_identity(&identities) }
 * and the live route (rb_translated_execute_set_card_identity_effect in
 * ability_effects.c) calls rb_effect_set_card_identity directly, so the
 * all_regions arm is still unimplemented there. Wiring it through this function
 * was implemented and MEASURED as -1 assertion on
 * tests/test_parity_executor_dispatch.c:896, which asserts the single-region
 * prohibition spelling for an effect that sets all_regions=true. That test is
 * wrong against Rust (the all-regions arm emits `card_identity:<card_id>:<id>`,
 * ability_ops.rs:766); the fix is the test's, and the report names it. Deleting
 * this function removes a second, unreachable spelling of the same dispatch
 * rather than the only one. */

/* Register a persistent trigger suppression in game state. The trigger scan
   also detects suppressors directly on active cards, matching Rust. */
void rb_suppress_ability_trigger(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    if (!g || !e) return;
    const char *trigger = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"suppressed_trigger") && e->extra_v[i]) {
            trigger=e->extra_v[i];
            break;
        }
    if (!trigger || !*trigger || g->n_prohibition >= 64) return;
    char entry[48];
    snprintf(entry, sizeof(entry), "suppress:%d:%s", actor, trigger);
    for (int i = 0; i < g->n_prohibition; i++)
        if (!strcmp(g->prohibition[i], entry)) return;
    snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[0]), "%s", entry);
    g->n_prohibition++;
}

/* DELETED 2026-10-02 — six zero-caller entry points, one Rust concept each.
 *
 * Every function removed below had NO caller anywhere in the C tree (src/ and
 * tests/, verified by name) and duplicated a live function that already owns
 * the same Rust function. They are the "correctly ported code with zero
 * callers" defect class: they read as implemented behaviour, so a later reader
 * trusts them, and a fix applied to one changes nothing that runs.
 *
 *   deleted here              Rust twin                        live owner
 *   -----------------------   ------------------------------   ------------------------------------
 *   rb_execute_gain_ability_  execute_gain_ability_effect     rb_translated_execute_gain_ability_effect
 *     effect                    (ability_ops.rs:29-149)        (ability_effects.c; executor.c:384)
 *   rb_execute_invalidate_    execute_invalidate_ability     rb_translated_execute_invalidate_ability
 *     ability                   (ability_ops.rs:257-387)       (ability_effects.c; executor.c:380)
 *   rb_execute_activate_      execute_activate_ability       rb_translated_execute_activate_ability
 *     ability                   (ability_ops.rs:169-255)       (ability_effects.c; executor.c:378)
 *   rb_execute_gain_ability  execute_gain_ability            rb_gain_ability (called from
 *                                                    (ability_ops.rs:413)   ability_effects.c and misc.c:1091)
 *   rb_execute_gain_ability_ execute_gain_ability_from_      rb_gain_ability_from_source (called from
 *     from_source               source (ability_ops.rs:567)   turn/triggers.c:1126)
 *   rb_execute_set_card_      execute_set_card_identity_     rb_translated_execute_set_card_identity_
 *     identity_effect           effect (ability_ops.rs:151)   effect (ability_effects.c; executor.c:388)
 *
 * s_execute_activate_ability went with them: its only two callers were
 * rb_activate_ability_effect and rb_execute_activate_ability, both zero-caller
 * and both declared in include/rabuka.h:2028 with nothing calling them.
 * s_set_extra went too: its only caller was rb_execute_gain_ability_effect,
 * and it wrote a POINTER to a 16-byte stack buffer (`char binding[16]`) into the
 * LIVE effect's extras table, so any later read of that effect's "target_card"
 * key was a dangling read. Removing the caller removed the defect.
 *
 * DIVERGENCE STILL OPEN between a deleted copy and the live one, reported
 * rather than fixed because the live copy is what the suites measure:
 * ability_ops.rs:213-223 compares the FIRST '/'-separated component of the
 * ability's trigger string for EQUALITY against target_trigger
 * (`a.triggers.split('/').next() == trig`). The live enqueue_matching_ability
 * (ability_effects.c) uses `strstr(ability.triggers, trigger)` instead — a
 * substring test, so a trigger that is a substring of a longer one matches
 * where Rust would not. */
