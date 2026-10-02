#include "rabuka.h"
#include "gen_data.h"   /* RBKA_NUM_ABILITIES_C — the generated ability count */
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

extern const uint32_t RBKA_NUM_ABILITIES;
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src);

/* ════════════════════════════════════════════════════════════════════
    AbilityRef — faithful C translation of engine/src/ability/ability_store.rs

    Rust: AbilityRef(pub u16) with a per-slot OnceLock<Arc<Ability>> cache
    (one slot per generated ability; Rust's doc comment still quotes "936
    slots, ~15 KB empty", a count the corpus has since outgrown). Decodes from
    bytecode on first resolve(), then cheap cache hit thereafter.

    C equivalent:
      - RbAbilityRef holds a uint16_t bytecode index.
      - g_ability_cache[] is a static array of Ability* (one per slot).
      - g_ability_cache_valid[] tracks which slots have been decoded.
      - rb_ability_ref_resolve() decodes on miss, returns cached pointer on hit.
      - The caller must NOT free the returned pointer: the cache owns it and
        keeps it for the life of the process.

    LIFETIME — ONCE-ONLY, AND THAT IS RUST'S RULE, NOT AN OMISSION.
    Rust's cache is `static CACHE: OnceLock<Vec<OnceLock<Arc<Ability>>>>`
    (ability_store.rs:22-30): built on first use, never reset, and never even
    DROPPED, because a Rust `static` has no destructor — every Arc<Ability> in
    it is reclaimed by the OS at process exit and never by Rust code. The whole
    safety argument is therefore one sentence: the compiled-in tables do not
    change while the process runs, so a cached Ability can never be stale.
    This file reproduces that exactly: a slot is written once and never freed.
    There is deliberately no flush. See the note where one used to be.
    ════════════════════════════════════════════════════════════════════ */

/* Cache capacity is DERIVED from the generated ability count — never a literal.
   RBKA_NUM_ABILITIES_C is the macro tools/gen_from_rs.py writes into gen_data.h
   from the same generator run that writes src/core/generated/gen_data.c, so a
   corpus change (936 -> 985 happened during this work) moves the bound with it.
   The old `#define RB_ABILITY_CACHE_CAP 1024` was a hand-guessed round number,
   and the guess produced two DIFFERENT wrong answers for an out-of-range index
   (measured against this tree's 985 abilities by linking the same probe against
   the old and new object):
     idx 985..1023 -> cached a PHANTOM slot and returned a NON-NULL pointer to a
                      silently empty default Ability (use_limit=-1, full_text=NULL),
                      which then stayed cached for the life of the process. A
                      caller could not tell it apart from a real ability.
     idx >= 1024   -> decoded the ability, immediately freed it, and returned NULL.
   Both are now one answer: out of range means NULL, and nothing is cached.

   The array below is sized by the macro, so the bound is a compile-time
   constant and there is no separate hand-maintained number that can drift.
   There is deliberately no `+ 1` slack either: the cache is indexed by the SAME
   u16 ability index the bytecode decoder is, and an index the decoder rejects
   must not have a cache slot (see ability_index_in_range below), so a slot that
   no decoder call could ever reach would be pure dead capacity.

   GCC does NOT diagnose a cross-TU `extern const T[N]` whose size disagrees with
   its definition (measured on this toolchain: gcc 15.3.0, both TUs clean, link
   clean), so deriving the bound from the macro is necessary but not sufficient
   — it cannot, on its own, prove gen_data.h and gen_data.c are from the same
   generator run. ability_slot_count() below closes that gap at runtime by
   clamping to RBKA_NUM_ABILITIES, the count gen_data.c itself publishes and the
   one vm.c range-checks every decode against. */
#define RB_ABILITY_CACHE_SLOTS ((int)RBKA_NUM_ABILITIES_C)

/* Per-slot decoded-ability cache — mirrors OnceLock<Vec<OnceLock<Arc<Ability>>>>.
   Initialized lazily: all entries NULL until first resolve(). */
static Ability *g_ability_cache[RB_ABILITY_CACHE_SLOTS];
static int      g_ability_cache_valid[RB_ABILITY_CACHE_SLOTS];
static int      g_ability_cache_initialized = 0;

_Static_assert(RBKA_NUM_ABILITIES_C > 0,
               "generated ability count is zero — gen_data.h is stale");
_Static_assert(RBKA_NUM_ABILITIES_C <= 0xFFFFu,
               "generated ability count exceeds the u16 range RbAbilityRef.idx can address");

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

/* NOT PORTED ON PURPOSE — do not "add the missing function".
 *
 * AbilityRef::peek() (ability_store.rs:64-85) returns Option<&Ability> for
 * callers that only want to READ an already-decoded ability. It exists purely
 * to avoid `Arc::clone`, which is an atomic read-modify-write on a cache line
 * shared by every worker thread, and the doc comment says so outright: "This
 * costs nothing" against ~50 .resolve() sites, many inside per-card loops,
 * spending a few hundred atomic RMWs per action just to keep a `use_limit: u8`
 * alive for one read. It is a LIFETIME optimisation with no semantic content —
 * peek() returns None before a slot has been decoded, so any caller that needs a
 * value still has to call resolve(). There is no C counterpart here and one must
 * not be written: the C hands out a borrowed `const Ability *` that resolve()
 * already owns, so "clone an Arc" is literally a pointer copy and there is
 * nothing left to optimise. An rb_ability_ref_peek() would be a second spelling
 * of resolve() that can return NULL for a reason resolve() never does
 * (not-yet-decoded), i.e. a new way for the two entry points to disagree — the
 * exact defect this file just spent a change removing.
 *
 * DERIVES: Rust's AbilityRef derives Debug, Clone, Copy, PartialEq, Eq and Hash
 * (ability_store.rs:18), all of which are over the u16 INDEX ONLY — never over
 * a decoded Ability, and never over a cache slot's contents. The C matches:
 * RbAbilityRef is a bare uint16_t, so == on two refs compares indexes exactly as
 * Rust's PartialEq does, and rb_ability_ref_idx() is the field accessor Rust's
 * generated Copy hands out by value. Two C divergences worth naming rather than
 * papering over: (1) rb_ability_ref_idx(NULL) returns 0, where Rust has no
 * null-reference concept at all and would be UB, so a NULL ref silently compares
 * equal to index 0; (2) nothing in this file compares an Ability's CONTENTS, so
 * there is no place where a C equality test could silently differ from Rust's
 * index-only equality. The functions deleted below (flush / flush_slot) were the
 * only flush-adjacent code here, and neither performed any comparison at all —
 * they compared an index against a bound, never an Ability against an Ability. */

/* ── Internal helpers ─────────────────────────────────────────────────── */

/* The number of ability indices this cache may hold: the generated count
   (gen_data.h) clamped DOWN to the count gen_data.c publishes. The clamp can
   only ever shrink the bound, so the result is always <= the array size and
   therefore always a safe index limit.

   Computed on demand rather than cached, so the bound is correct for any caller
   regardless of whether ability_cache_ensure_init() has run yet. */
static uint32_t ability_slot_count(void) {
    uint32_t slots = (uint32_t)RB_ABILITY_CACHE_SLOTS;
    if (RBKA_NUM_ABILITIES < slots) slots = RBKA_NUM_ABILITIES;
    return slots;
}

/* THE range check — one predicate, used by resolve(), decode() and
   cache_size(), for the cache slot index AND the bytecode decode index alike.

   Both entry points take the same u16 ability index into the same
   [0, slot_count) domain, so bounding them with one predicate is what stops
   them drifting apart again: previously rb_ability_ref_decode() rejected
   idx >= RBKA_NUM_ABILITIES while rb_ability_ref_resolve() did not, so the two
   ways into one concept disagreed about what an out-of-range index means.
   Measured on this tree at 985 abilities: the old pair returned rc=0 from
   decode(985) and a NON-NULL empty Ability from resolve(985). Callers now get
   NULL from resolve() and 0 plus a zeroed Ability from decode(), both before the
   cache array or the decoder is touched.

   Rust refuses the same index differently: `decoded_slots()[self.0]` (vm
   resolve, ability_store.rs:48) PANICS on an out-of-range u16, because the Vec
   has exactly NUM_ABILITIES elements. C has no unwinding, and the declared C
   contract is a pointer that is NULL on failure (rabuka.h:2801), so a NULL
   return is the available translation of that panic: both refuse the index and
   neither decodes it. */
static int ability_index_in_range(uint16_t idx) {
    return (uint32_t)idx < ability_slot_count();
}

/* Lazy-init the cache validity array. Called once on first resolve().
   Also the one place the two generated ability counts are compared. */
static void ability_cache_ensure_init(void) {
    if (g_ability_cache_initialized) return;
    for (int i = 0; i < RB_ABILITY_CACHE_SLOTS; i++) {
        g_ability_cache[i] = NULL;
        g_ability_cache_valid[i] = 0;
    }
    if (RBKA_NUM_ABILITIES != (uint32_t)RBKA_NUM_ABILITIES_C)
        fprintf(stderr,
                "FATAL: ability-count skew — gen_data.h says %u, "
                "generated/gen_data.c says %u; the ability cache is bounded to "
                "%u slots. These files come from different generator runs: "
                "FIX: bash tools/regen_all.sh\n",
                (unsigned)RBKA_NUM_ABILITIES_C, (unsigned)RBKA_NUM_ABILITIES,
                (unsigned)ability_slot_count());
    g_ability_cache_initialized = 1;
}

/* Decode a single ability from bytecode into a heap-allocated Ability.
   Precondition: ability_index_in_range(idx) — the caller owns the check, and
   there is exactly one of those checks in this file.

   Returns NULL only on allocation failure. A decode failure is NOT an error
   here: mirror Rust `AbilityRef::decode()`, which logs the error and returns
   Ability::default() so the game keeps running (ability_store.rs:87-95). We
   cannot log from here without pulling in the log subsystem, and vm.c's
   decoder already substitutes a zeroed default Ability for structural failure;
   only an out-of-range index is a hard error, and that is now rejected before
   this point. */
static Ability *ability_decode(uint16_t idx) {
    Ability *a = (Ability *)rb_malloc(sizeof(Ability));
    if (!a) return NULL;
    memset(a, 0, sizeof(*a));
    a->use_limit = -1;
    if (!rb_decode_ability((uint32_t)idx, a)) {
        memset(a, 0, sizeof(*a));
        a->use_limit = -1;
    }
    return a;
}

/* ── Core resolve API ─────────────────────────────────────────────────── */

/* Mirror AbilityRef::resolve() -> Arc<Ability>.

   Behaviour:
     cache hit  : returns the cached pointer (Rust's Arc::clone is free in C,
                  because a raw pointer is already shared, not owned).
     cache miss : decodes once and stores the heap-allocated Ability in the
                  slot. Lost-race (two concurrent resolves) is safe: both
                  sides return a valid pointer.

   Returns NULL for a NULL ref, for an out-of-range index (the same guard
   rb_ability_ref_decode() applies — see ability_index_in_range), or on
   allocation failure. Rust panics on the out-of-range case instead; see that
   comment for why a NULL return is the C translation.

   The returned pointer is owned by the cache and is valid for the rest of the
   process. The caller must NOT free it, and must not assume it stays valid
   across a bytecode reload — there is no flush, by design (see the LIFETIME
   note at the top of this file). */
const Ability *rb_ability_ref_resolve(const RbAbilityRef *ref) {
    if (!ref) return NULL;

    uint16_t idx = ref->idx;

    ability_cache_ensure_init();

    /* THE guard. Because it runs before the array is indexed, every index that
       reaches the cache below is inside it: ability_index_in_range() is bounded
       by ability_slot_count(), which is clamped to the array size. That is what
       makes the old `idx < RB_ABILITY_CACHE_CAP` tests dead code, and it kills
       the decode-then-free-then-return-NULL arm they sat next to. */
    if (!ability_index_in_range(idx)) return NULL;

    if (g_ability_cache_valid[idx]) return g_ability_cache[idx];

    /* Cache miss — decode from bytecode. */
    Ability *decoded = ability_decode(idx);
    if (!decoded) return NULL;

    /* Store in the cache slot. Lost-race is harmless: the slot may already
       contain a valid entry (concurrent resolve from another thread), in
       which case we drop our duplicate and return the incumbent. rb_free_ability
       frees the DECODED CONTENTS but not the struct itself (vm.c:1277), so the
       struct needs rb_free to match the rb_malloc in ability_decode. */
    if (!g_ability_cache_valid[idx]) {
        g_ability_cache[idx] = decoded;
        g_ability_cache_valid[idx] = 1;
    } else {
        rb_free_ability(decoded);
        rb_free(decoded);
        decoded = g_ability_cache[idx];
    }
    return decoded;
}

/* Mirror AbilityRef::decode(&self) -> Ability.

   Decodes the ability from bytecode for the given index. On failure returns
   a zeroed default Ability (use_limit = -1) — this matches Rust's
   `Ability::default()` fallback.

   The output is written into `out` (caller-allocated). Returns 1 on success,
   0 on out-of-range or structural decode failure. The range check is the same
   predicate resolve() uses, so the two entry points cannot disagree about
   which indices exist. Unlike resolve() this does NOT consult the cache:
   Rust's decode() is also cache-free and hands back an owned Ability, which is
   what lets the caller write it into caller-owned storage. */
int rb_ability_ref_decode(const RbAbilityRef *ref, Ability *out) {
    if (!ref || !out) return 0;
    uint16_t idx = ref->idx;
    ability_cache_ensure_init();
    if (!ability_index_in_range(idx)) {
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

/* DELETED 2026-10-02: rb_ability_ref_flush and rb_ability_ref_flush_slot.

   Rust's ability_store.rs has NO flush. Its cache is
   `static CACHE: OnceLock<Vec<OnceLock<Arc<Ability>>>>` (ability_store.rs:24):
   populated by get_or_init, never reset, never dropped — a Rust `static` has
   no destructor, so the cached Arc<Ability>s live until the OS reclaims them at
   process exit. Rust's entire safety argument is one invariant: the compiled-in
   tables do not change while the process runs.

   These two functions were C inventions, and the only thing in this file that
   could turn the cache into a DANGLING-POINTER SOURCE. rb_ability_ref_resolve()
   hands out a pointer the cache owns and the caller must not free; a flush then
   rb_free_ability()'d that exact object, so every outstanding holder was left
   pointing at freed memory with no compile-time warning and no way to find the
   holders — the cache does not track them. Under RB_NO_MALLOC it is worse:
   rb_free is a no-op and rb_alloc_reset() rewinds a bump arena
   (src/core/alloc.c:6-20), so "freeing" a slot frees every allocation made
   after it, including live GameState data.

   Options weighed:
     (b) keep them and fix the lifetime — impossible without an owning return
         type. Rust gets safety from Arc; the C contract is a bare
         `const Ability *` that callers are told not to free, so the cache has no
         idea who holds what. Retrofitting a refcount would change the return
         type of a public, header-declared API and every future caller, to
         preserve a capability Rust deliberately does not offer.
     (c) keep them and document — rejected: a comment claiming safety on a
         function whose entire effect is to invalidate pointers other code holds
         is exactly the live foot-gun the audit asked to remove.
     (a) delete — chosen. The once-only lifetime is now stated in one place (the
         LIFETIME note at the top of this file) instead of being contradicted by
         two functions. rb_ability_ref_cache_size() below survives because it is
         read-only and cannot dangle anything.

   Zero callers tree-wide (src/ and tests/, by name) and rb_unload() never called
   them despite rabuka.h telling callers to — they were inert by luck, not by
   design. include/rabuka.h:2804-2805 declares them and is the header owner's to
   drop. */

/* Return the number of currently-populated cache slots. (Diagnostic.)
   Read-only: it cannot invalidate a pointer handed out by resolve(). Has no
   Rust twin — Rust exposes no cache-size accessor — so it is a C-only
   diagnostic, kept because it is inert. */
int rb_ability_ref_cache_size(void) {
    ability_cache_ensure_init();
    int n = 0;
    for (uint32_t i = 0; i < ability_slot_count(); i++) {
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
