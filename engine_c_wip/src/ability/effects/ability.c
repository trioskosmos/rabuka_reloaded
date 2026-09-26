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

/* Upsert a scalar extra on a live effect. vm.c's effect_set_extra is static
   there, so mirror it here for the bindings this file installs (e.g. the
   per-recipient `target_card` binding). `value` must outlive the effect. */
static void s_set_extra(AbilityEffect *e, const char *key, const char *value) {
    if (!e || !key || !value) return;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) {
            e->extra_v[i] = (char *)value;
            return;
        }
    }
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)key;
    e->extra_v[e->n_extra] = (char *)value;
    e->n_extra++;
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

void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e){
    if (!g) return;
    int who = actor;
    if (e && e->target && !strcmp(e->target, "opponent")) who = actor ^ 1;
    int synthetic_changed = 0;
    for (int slot = 0; slot < 64; slot++) {
        int card_id = g->gained_card_ids[slot];
        if (card_id < 0 || rb_owner_of_card(g, card_id) != who) continue;
        while (rb_card_num_gained_abilities(g, card_id) > 0) {
            rb_remove_gained_ability(g, card_id, 0);
            synthetic_changed = 1;
        }
    }
    int temp_write = 0;
    for (int i = 0; i < g->n_temp_effects; i++) {
        RbTempEffect *te = &g->temp_effects[i];
        if (rb_owner_of_card(g, te->card_id) == who) {
            rb_mods_add_blade(&g->mods, te->card_id, -te->blade);
            rb_mods_add_score(&g->mods, te->card_id, -te->score);
            for (int c = 0; c < 8; c++) {
                rb_mods_add_heart(&g->mods, te->card_id, c, -te->heart[c]);
                rb_mods_add_need_heart(&g->mods, te->card_id, c, -te->need_heart[c]);
            }
        } else {
            g->temp_effects[temp_write++] = *te;
        }
    }
    g->n_temp_effects = temp_write;
    if (synthetic_changed) rb_recalc_constants(g);
}

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

/* Mirror ability_effects.rs:141-227 execute_activate_ability.

   "previous_selected" (emitted by the parser for 「そのカード／それらが持つ…能力を発動させる」)
   fires EVERY selected card's ability whose trigger matches target_trigger.
   With no target_trigger the fallback stores the ability text on the
   activating card instead of firing a debut. */
static void s_execute_activate_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *source_card = gain_extra(e, "source_card");
    const char *trigger = gain_extra(e, "target_trigger");
    const char *ability_text = gain_extra(e, "ability_text");

    /* Which cards' abilities are fired. */
    int card_ids[RB_MAX_RECENTLY_MOVED];
    int n_ids = 0;
    if (source_card && !strcmp(source_card, "previous_selected")) {
        for (int i = 0; i < g->n_selected_cards && n_ids < RB_MAX_RECENTLY_MOVED; i++)
            card_ids[n_ids++] = g->selected_cards[i];
    } else if (source_card && !strcmp(source_card, "cost_card")) {
        if (g->n_recently_moved > 0)
            card_ids[n_ids++] = g->recently_moved[g->n_recently_moved - 1];
    }

    if (trigger && *trigger) {
        /* Rust compares the FIRST '/'-separated component of the ability's
           trigger string; rb_card_has_ability_trigger_for applies the same
           trigger_text_matches normalisation. */
        for (int i = 0; i < n_ids; i++) {
            int cid = card_ids[i];
            if (cid < 0) continue;
            if (!rb_card_has_ability_trigger_for(g, cid, trigger)) continue;
            /* Fire through the normal ability queue so the fired ability's own
               cost is paid before its effect resolves (Q273). */
            rb_trigger_debut(g, actor, cid);
        }
        return;
    }

    /* Fallback: store the gained ability string on the activating card.
       Rust pushes the string into GameState::gained_abilities
       (HashMap<i16, Vec<String>>); the C port has no string map, so the
       equivalent is a synthetic trigger-carrying gained Ability on the same
       card — the same store gained_card_abilities uses, so every consumer
       that clears/reads gained abilities sees it. */
    if (g->activating_card >= 0) {
        char buf[256];
        const char *text = ability_text ? ability_text : "";
        if (trigger && *trigger)
            snprintf(buf, sizeof(buf), "%s_trigger:%s", text, trigger);
        else
            snprintf(buf, sizeof(buf), "%s", text);
        Ability gained;
        memset(&gained, 0, sizeof(gained));
        gained.use_limit = -1;
        gained.full_text = rb_strdup2(buf);
        gained.triggerless_text = rb_strdup2(buf);
        gained.triggers = (trigger && *trigger) ? rb_strdup2(trigger) : NULL;
        if (rb_register_gained_ability(g, g->activating_card, &gained) < 0)
            rb_free_ability(&gained);
    }
}

void rb_activate_ability_effect(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    s_execute_activate_ability(g, actor, e);
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

/* Mirror ability_effects.rs::execute_set_card_identity_effect. When the
   all_regions flag is set, route through the all-regions variant (which also
   records per-card prohibition notes); otherwise apply the identity rewrite
   for this region only. */
void rb_set_card_identity_effect(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int all_regions=0;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"all_regions") && e->extra_v[i] && !strcmp(e->extra_v[i],"true"))
            all_regions=1;
    if(all_regions) rb_effect_set_card_identity_all_regions(g, actor, e, host_cid);
    else            rb_effect_set_card_identity(g, actor, e, host_cid);
}

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

/* -- execute_gain_ability_effect (ability_effects.rs:13-121) --
   「…を得る」. Resolves the recipient list, then registers the gained ability
   once per recipient. Two selection sources precede the activating-card
   fallback:
     * stage_member_targeting — 「自分のステージにいる『X』のメンバーN人まで…を得る」:
       prompt for the members, then re-apply this effect with the picks in
       selected_cards.
     * anaphora "cost_waited" — 「これによってウェイト状態になったメンバー」:
       bind to the members THIS ability's cost waited, not to the source. */
void rb_execute_gain_ability_effect(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;

    const char *card_type = e->card_type_field[0] ? e->card_type_field
                                                  : gain_extra(e, "card_type");
    const char *source = e->source ? e->source : gain_extra(e, "source");
    const char *grp = gain_extra(e, "group_names");

    int stage_member_targeting = source && !strcmp(source, "stage") &&
                                 card_type && !strcmp(card_type, "member_card") &&
                                 grp && *grp;

    if (stage_member_targeting && g->n_selected_cards == 0) {
        const char *target = rb_effect_target_name(e);
        RbPlayer *P = &g->p[rb_resolve_target_player(g, target)];
        int candidates[RB_STAGE_SIZE];
        int n_cand = 0;
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            int cid = P->stage[q];
            if (cid == RB_EMPTY_SLOT) continue;
            if (!rb_card_matches_group_str(cid, grp)) continue;
            candidates[n_cand++] = q;
        }
        if (n_cand > 0) {
            int pick = rb_effect_count_or(e, 1);
            if (pick < 1) pick = 1;
            if (pick > n_cand) pick = n_cand;
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", card_type,
                           pick, 1, "gain_ability");
            strncpy(g->queue.pending.filter_group, grp,
                    sizeof(g->queue.pending.filter_group) - 1);
            strncpy(g->queue.pending.target_player_id, target,
                    sizeof(g->queue.pending.target_player_id) - 1);
            g->queue.pending.n_filtered_indices = 0;
            for (int i = 0; i < n_cand; i++)
                g->queue.pending.filtered_indices[g->queue.pending.n_filtered_indices++] =
                    candidates[i];
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
            /* Re-apply THIS effect after the choice (Rust:
               ability_queue.set_pending_actions(vec![effect.clone()])). */
            g->queue.deferred = rb_effect_deep_clone(e);
            g->queue.resume_eff = g->queue.deferred;
            g->queue.resume_actor = actor;
            g->queue.resume_host = g->activating_card;
            rb_queue_pause_for_choice(g, &g->queue.pending);
            return;
        }
    }

    int targets[RB_STAGE_SIZE];
    int n_targets = 0;
    if (g->n_selected_cards > 0) {
        for (int i = 0; i < g->n_selected_cards && n_targets < RB_STAGE_SIZE; i++)
            targets[n_targets++] = g->selected_cards[i];
        g->n_selected_cards = 0;
    } else {
        const char *anaphora = gain_extra(e, "anaphora");
        if (anaphora && !strcmp(anaphora, "cost_waited") &&
            g->n_last_cost_waited_members > 0) {
            for (int i = 0; i < g->n_last_cost_waited_members; i++)
                targets[n_targets++] = g->last_cost_waited_members[i];
        } else if (g->activating_card >= 0) {
            targets[n_targets++] = g->activating_card;
        }
    }

    for (int i = 0; i < n_targets; i++) {
        int target_card = targets[i];
        if (target_card < 0) continue;
        /* Bind the recipient for this iteration; rb_gain_ability honours the
           target_card binding before falling back to the activating card. */
        char binding[16];
        snprintf(binding, sizeof(binding), "%d", target_card);
        s_set_extra(e, "target_card", binding);
        rb_gain_ability(g, actor, e);
    }
}

/* -- execute_set_card_identity_effect (ability_effects.rs:123-137) --
   all_regions routes to the all-regions rewrite, otherwise the single-region
   rewrite. Both live in state.c; this is the action-level dispatch. */
void rb_execute_set_card_identity_effect(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    rb_set_card_identity_effect(g, actor, e, g->activating_card);
}

/* -- execute_activate_ability (ability_effects.rs:141-227) -- */
void rb_execute_activate_ability(GameState *g, int actor, AbilityEffect *e) {
    s_execute_activate_ability(g, actor, e);
}

/* -- execute_invalidate_ability (ability_effects.rs:229-354) --
   「〜の能力を無効化する」. Only ライブ開始時 / ライブ成功時 are supported;
   any other target_trigger is an unsupported-effect error in Rust, which the
   executor records as a failed last_action_result without mutating state
   (here: no-op). self_target invalidates the activating card directly;
   otherwise one eligible stage member is chosen and invalidated. */
void rb_execute_invalidate_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *target_trigger = gain_extra(e, "target_trigger");
    if (!target_trigger) return;
    if (strcmp(target_trigger, "ライブ開始時") != 0 &&
        strcmp(target_trigger, "ライブ成功時") != 0) return;

    /* Duration: util::parse_duration() rejects unknown codes; an absent
       duration means Permanent. */
    const char *duration_code = gain_extra(e, "duration");
    const char *duration = "permanent";
    if (duration_code && *duration_code) {
        if (strstr(duration_code, "live") || strstr(duration_code, "ライブ"))
            duration = "live_end";
        else if (strstr(duration_code, "turn") || strstr(duration_code, "ターン"))
            duration = "until_end_of_turn";
        else if (strstr(duration_code, "permanent") || strstr(duration_code, "永久"))
            duration = "permanent";
        else
            return; /* unsupported duration code */
    }

    if (e->self_target_field[0] && !strcmp(e->self_target_field, "true")) {
        if (g->activating_card < 0) return;
        rb_try_add_ability_invalidation(g, g->activating_card, target_trigger, duration);
        return;
    }

    const char *target = e->target && *e->target ? e->target : "self";
    RbPlayer *P = &g->p[rb_resolve_target_player(g, target)];

    RbCardFilter filter;
    memset(&filter, 0, sizeof(filter));
    rb_effect_filter_subset(e, &filter);

    int stage_ids[RB_STAGE_SIZE];
    int n_stage = 0;
    for (int q = 0; q < RB_STAGE_SIZE; q++)
        if (P->stage[q] != RB_EMPTY_SLOT) stage_ids[n_stage++] = P->stage[q];

    /* Valid = the effect filter matches AND the card really has that trigger
       AND the trigger is not already invalidated. */
    int matched[RB_STAGE_SIZE];
    int n_matched = rb_matching_ids(&filter, stage_ids, n_stage, matched, RB_STAGE_SIZE);

    int valid[RB_STAGE_SIZE];
    int n_valid = 0;
    for (int i = 0; i < n_matched; i++) {
        if (!rb_card_has_ability_trigger_for(g, matched[i], target_trigger)) continue;
        if (rb_ability_is_invalidated(g, matched[i], target_trigger)) continue;
        valid[n_valid++] = matched[i];
    }
    if (n_valid == 0) return;

    if (g->n_selected_cards == 0) {
        int optional = e->is_optional;
        const char *opt = gain_extra(e, "optional");
        if (opt && !strcmp(opt, "true")) optional = 1;

        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage",
                       e->card_type_field[0] ? e->card_type_field : NULL,
                       1, optional, "invalidate_ability");
        if (filter.has_group) {
            size_t glen = strlen(filter.group);
            if (glen >= sizeof(g->queue.pending.filter_group))
                glen = sizeof(g->queue.pending.filter_group) - 1;
            memcpy(g->queue.pending.filter_group, filter.group, glen);
            g->queue.pending.filter_group[glen] = 0;
        }
        strncpy(g->queue.pending.target_player_id, target,
                sizeof(g->queue.pending.target_player_id) - 1);
        g->queue.pending.n_filtered_indices = 0;
        for (int i = 0; i < n_valid; i++)
            for (int q = 0; q < RB_STAGE_SIZE; q++)
                if (P->stage[q] == valid[i]) {
                    g->queue.pending.filtered_indices[g->queue.pending.n_filtered_indices++] = q;
                    break;
                }
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        g->queue.deferred = rb_effect_deep_clone(e);
        g->queue.resume_eff = g->queue.deferred;
        g->queue.resume_actor = actor;
        g->queue.resume_host = g->activating_card;
        rb_queue_pause_for_choice(g, &g->queue.pending);
        return;
    }

    int selected = g->selected_cards[0];
    g->n_selected_cards = 0;
    for (int i = 0; i < n_valid; i++) {
        if (valid[i] == selected) {
            rb_try_add_ability_invalidation(g, selected, target_trigger, duration);
            break;
        }
    }
}

/* -- execute_gain_ability (ability_effects.rs:379-534) --
   The registration half of 「…を得る」. The recipient is bound by
   execute_gain_ability_effect (target_card extra); rb_gain_ability performs
   the registration, the immediate per-card score application and the
   duration bookkeeping that util::push_temporary_effect records. */
void rb_execute_gain_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    rb_gain_ability(g, actor, e);
}

/* -- execute_gain_ability_from_source (ability_effects.rs:536-681) -- */
void rb_execute_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e) {
    s_gain_ability_from_source(g, actor, e);
}
