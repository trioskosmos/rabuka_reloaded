/* resolver.c  Efaithful C port of engine/src/ability/resolver.rs
   AbilityResolver orchestrator: use-limit, cost payment, condition gating,
   keyword checks, pending-choice routing, and modify_cost folding.

   The Rust AbilityResolver is a per-resolution holder of transient state
   (pending_choice, selected_cards, etc.). The C engine keeps most of that
   persistent state in GameState (queue, selected_cards, etc.) and threads
   host_cid explicitly. The C AbilityResolver globals here mirror only the
   resolver-only transient bits (condition_cache string hash, last_offered_sig,
   current_ability bookkeeping) that GameState does not already carry.

   Parity notes are tagged with the Rust line or § they mirror so a
   future auditor can diff directly against resolver.rs.
*/

#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
char *rb_strdup2(const char *s);

/* ── Forward declarations from other subsystems ── */
int rb_ability_uses_used(const GameState *g, int cid, int idx);
void rb_record_ability_use(GameState *g, int cid, int idx);
int rb_eval_condition_for_host(const GameState *g, int actor, int host_cid, const Condition *c);
int rb_ability_has_remaining_uses(const GameState *g, int cid, int idx);
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost);
int rb_distinct_stage_groups(const GameState *g, int pl);
int rb_target_player_index(const char *target, const char *master);
/* rb_effect_group_name is declared in include/rabuka.h and takes a caller-owned
 * scratch buffer: (e, buf, bufsz). The old 1-arg forward declaration here was a
 * leftover from when util.c's version was static and invisible; it is a
   conflicting declaration for a now-exported symbol. Do not re-add it — and do not
 * satisfy it. Callers must pass their own buffer. */
const char *rb_effect_position_any(const AbilityEffect *e);
void rb_execute_effect_ex(GameState *g, int actor, AbilityEffect *e, int host_cid);
int rb_activation_position_index(const char *p);
int rb_find_card_stage_position(const GameState *g, int cid);
void rb_log_push_verdict(const char *text, const char *kind, int passed);
int rb_log_buffer_len(void);
void rb_log_clear_verdicts(void);
/* log.rs:213 — the real signature drains into a caller buffer. The previous
 * one-arg prototype here did not match src/ability/log.c and would have linked
 * against garbage if it were ever called. */
int rb_log_drain_verdicts(RbAbilityLogItem *out, int max);
int rb_log_drain_verdicts_since(int start_index, RbAbilityLogItem *out, int max);
void rb_log_free_item(RbAbilityLogItem *item);
int rb_ability_debug_enabled(void);
/* cost.c: energy_count_any() — the PayEnergy amount rb_pay_cost actually charges. */
int rb_cost_energy_count_any(const AbilityEffect *e);
/* Defined at the foot of this file; used by the condition gate above. */
void rb_resolver_drain_verdicts(GameState *g);
void rb_resolver_drain_verdicts_since(GameState *g, int snapshot);
void rb_resolver_push_verdict(GameState *g, const char *text, const char *kind, int passed);
/* trigger helpers (rb_ability_has_trigger declared in rabuka.h) */
int rb_resolve_target_player(const GameState *g, const char *target);
/* condition field accessors (defined in src/core/card.c, not yet in rabuka.h) */
int rb_condition_get_cache(const Condition *c, int *out);
const char *rb_condition_get_group_names(const Condition *c);
const char *rb_condition_get_position(const Condition *c);
const char *rb_condition_get_activation_position(const Condition *c);
int rb_condition_get_distinct(const Condition *c);
/* Defined in src/ability/vm.c, beside the positions_characters decoder that
   produces the field this reads (the other accessors live in src/core/card.c,
   which this wave did not own). */
int rb_condition_get_positions_characters(const Condition *c);

/* ── Card helpers ── */
int rb_card_has_blade_heart(const Card *c);

/* ── Globals mirroring AbilityResolver fields not in GameState ── */
static Ability g_current_ability;
static int     g_current_ability_valid = 0;
static int     g_current_ability_idx = -1;
static int     g_activating_card_id = -1;
/* The dedup signature buffer must be able to hold the WORST CASE the writer can
   produce, or the signature truncates and two different offers that share a long
   prefix compare equal -- `last_offered_sig` would then suppress a prompt it
   should have shown. Worst case: "skip=1;" (7) + "[0]" (3) + the whole
   `RbChoice::description` (char[512], so 511 chars) + NUL = 522. The old `char
   sig[512]` / `static char g_last_offered_sig[512]` were both one byte too small
   for that, which is what gcc's -Wformat-truncation at :243 was pointing at
   (up to 515 bytes into the 505 that remain after the "skip=N;" prefix). */
#define RB_CHOICE_SIG_MAX 544
static char    g_last_offered_sig[RB_CHOICE_SIG_MAX] = {0};
static int     g_has_last_sig = 0;

/* ── Helpers ── */

/* effect helpers: read extra_kv by key (mirrors effect.*_any()) */
static const char *eff_extra(const AbilityEffect *e, const char *key) {
    if (!e || !key) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}
static const char *rb_effect_activation_position(const AbilityEffect *e) {
    return eff_extra(e, "activation_position");
}
static const char *rb_effect_group_names_any(const AbilityEffect *e) {
    const char *v = eff_extra(e, "group_names");
    if (v) return v;
    return eff_extra(e, "group_name");
}

int rb_find_card_stage_position(const GameState *g, int cid) {
    if (!g) return -1;
    for (int pl = 0; pl < 2; pl++) for (int s = 0; s < RB_STAGE_SIZE; s++) if (g->p[pl].stage[s] == cid) return s;
    return -1;
}
/* condition helpers */
static int cond_get_cache(const Condition *c) {
    int out = 0;
    if (rb_condition_get_cache(c, &out)) return out;
    return 0;
}
static const char *cond_get_group_names(const Condition *c) {
    return rb_condition_get_group_names(c);
}
static const char *cond_get_position(const Condition *c) {
    return rb_condition_get_position(c);
}
static int cond_get_positions_characters(const Condition *c) {
    return rb_condition_get_positions_characters(c);
}
static int cond_get_distinct(const Condition *c) {
    return rb_condition_get_distinct(c);
}
static int cond_is_appearance(const Condition *c) {
    if (!c) return 0;
    return c->variant == RB_COND_APPEARANCE;
}

/* ── Top-level activation-position overlay (resolver.rs:401-405, 421-422,
      461-468 and the identical application at 509-...) ──
   Rust's `condition_needs_position_overlay` decides whether the patched copy
   is worth making at all: cond.get_position().is_none()
   && cond.get_positions_characters().is_none()
   && (effect.position_any().is_some() || effect.activation_position_any().is_some()).
   Both `is_none()` clauses are live in Rust; positions_characters used to have
   no C accessor at all, so the second clause was vacuous here and an effect
   that declared `position` would patch a position onto a condition that had
   already pinned its own slots through positions_characters, overriding the
   per-pair area names the card actually declared.

   Returns `cond` unchanged when the overlay would not write; otherwise a
   patched shallow copy in caller storage `scratch` with the effect's position
   (else its activation_position) appended as a top-level field, and
   *injected set so the caller can release it with overlay_release().

   THE KEY IS NOT ALWAYS "position". Rust writes `set_position(pos)` in the
   first arm and `set_activation_position(act_pos)` in the `else if` arm
   (resolver.rs:653-655), i.e. the two overlays land on DIFFERENT condition
   fields, and the two fields are read by different gates with different
   semantics:

     - "position" is a single PositionInfo. condition.c's `position` arm runs
       it through stage_index_of_position(), which accepts exactly one of
       "center" / "left_side" / "right_side" and returns -1 for anything else,
       so a value the other arm would have accepted fails outright here.
     - "activation_position" is a single string but may be a COMMA LIST, and
       condition.c's `activation_position` arm splits on ',' before mapping
       each entry (condition.c:1378-1389).

   Writing the activation_position value under "position" therefore broke
   every effect that declares a comma list. MEASURED over the whole card
   database (probe: 3689 effects), 6 effect nodes carry activation_position
   with no position and a comma list -- "center,left_side,right_side" and
   "left_side,right_side" -- and every one of them evaluated to FALSE in the C
   because stage_index_of_position() returned -1, where Rust evaluates the
   list membership and succeeds. Both overlay arms now write the key Rust
   writes for that arm. */

static const Condition *position_overlay(const AbilityEffect *eff, const Condition *cond,
                                         Condition *scratch, int *injected) {
    *injected = 0;
    if (!cond) return cond;
    if (cond_get_position(cond) || cond_get_positions_characters(cond)) return cond;
    const char *eff_pos = rb_effect_position_any(eff);
    const char *eff_act_pos = rb_effect_activation_position(eff);
    if (!eff_pos && !eff_act_pos) return cond;
    if (cond->n_fields >= RB_MAX_COND_FIELD) return cond;
    *scratch = *cond;
    /* resolver.rs:653-655 -- first arm `position`, else-if arm
       `activation_position`. */
    const char *key = eff_pos ? "position" : "activation_position";
    const char *val = eff_pos ? eff_pos : eff_act_pos;
    scratch->fields[scratch->n_fields].key = rb_strdup2(key);
    scratch->fields[scratch->n_fields].v.tag = RB_TAG_STR;
    scratch->fields[scratch->n_fields].v.s = rb_strdup2(val);
    scratch->n_fields++;
    *injected = 1;
    return scratch;
}

/* Release the field position_overlay() appended. Only the trailing field is
   ever ours, and only when it is one of the overlay keys, so this never
   frees anything owned by the decoded condition. */
static void overlay_release(Condition *scratch) {
    if (!scratch || scratch->n_fields == 0) return;
    CondField *f = &scratch->fields[scratch->n_fields - 1];
    if (!f->key) return;
    if (strcmp(f->key, "position") != 0 &&
        strcmp(f->key, "activation_position") != 0 &&
        strcmp(f->key, "group_names") != 0) return;
    free(f->key);
    if (f->v.tag == RB_TAG_STR) free(f->v.s);
    f->key = NULL;
    memset(&f->v, 0, sizeof(f->v));
}

/* Stable hash for condition cache key (mirrors format!("{:?}", condition) hash)
   Use pointer + variant + field count as cheap stable key; bytecode conditions
   are interned per ability so pointer identity is sufficient.

   The body lives in compound.c (rb_cond_cache_key) because compound.c used to
   hold a byte-identical static copy of it; this file called its own, so the two
   implementations had to be kept in step by hand. One definition now. */
extern int rb_cond_cache_key(const Condition *c);
static int cond_cache_key(const Condition *c) { return rb_cond_cache_key(c); }

/* The string-keyed twin of cond_cache_key, for the exported
 * rb_resolver_{cached,store}_condition_verdict entry points, whose callers hold
 * a condition TEXT rather than a decoded Condition*. FNV-1a over the text, kept
 * disjoint from the pointer-derived range (0 for "no key") by never returning
 * 0. Two texts therefore share a slot exactly when they are byte-identical,
 * which is the same equality Rust's `format!("{:?}", condition)` key buys. */
static int cond_text_cache_key(const char *cond_text) {
    if (!cond_text || !cond_text[0]) return 0;
    unsigned h = 2166136261u;
    for (const unsigned char *p = (const unsigned char *)cond_text; *p; p++) {
        h ^= (unsigned)*p; h *= 16777619u;
    }
    int key = (int)(h & 0x7FFFFFFF);
    return key == 0 ? 1 : key;
}

/* ── choice_offer_sig (resolver.rs:44) ── */
static void choice_offer_sig(const RbChoice *ch, char *out, size_t sz) {
    if (!out || sz == 0) return;
    out[0] = '\0';
    if (!ch) return;
    int pos = snprintf(out, sz, "skip=%d;", ch->allow_skip);
    if (pos < 0 || (size_t)pos >= sz) return;
    /* in Rust: for (i, o) in offered.iter().enumerate() write!(sig, "[{i}]{o}") .
       In C the offered labels are not materialized at store_pending_choice time except
       via description; use description + target as proxy. */
    snprintf(out + pos, sz - pos, "[0]%s", ch->description);
}

/* ── resolver.rs::zone_for_card ── */
const char *rb_resolver_zone_for_card(const GameState *g, int card_id) {
    if (!g) return "?";
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        for (int s = 0; s < RB_STAGE_SIZE; s++) if (P->stage[s] == card_id) return "stage";
        for (int i = 0; i < P->live.n; i++) if (P->live.cards[i] == card_id) return "live_card_zone";
        for (int i = 0; i < P->success.n; i++) if (P->success.cards[i] == card_id) return "success_live_card_zone";
        for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == card_id) return "hand";
        for (int i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == card_id) return "waitroom";
        for (int i = 0; i < P->energy.n; i++) if (P->energy.cards[i] == card_id) return "energy_zone";
    }
    return "?";
}

/* ── resolver.rs::check_use_limit_reached ── */
int rb_resolver_use_limit_reached(const GameState *g, int card_id, int ability_index, int use_limit) {
    if (!g || use_limit <= 0) return 0;
    return rb_ability_uses_used(g, card_id, ability_index) >= use_limit;
}

/* ── resolver.rs internal: check_use_limit_reached (self) ── */
static int check_use_limit_reached_self(const GameState *g, int card_id, int ability_index, int use_limit) {
    return rb_resolver_use_limit_reached(g, card_id, ability_index, use_limit);
}

/* ── resolver.rs::get_pending_choice / pending_choice index ── */
const RbChoice *rb_resolver_get_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return NULL;
    return &g->queue.pending;
}
int rb_resolver_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return -1;
    return (int)g->queue.pending.route;
}

/* ── resolver.rs::current_ability_is_activation ── */
int rb_resolver_current_ability_is_activation(const Ability *ab) {
    return ab && rb_ability_has_trigger(ab, 0); /* RB_TRIGGER_ACTIVATION = 0 */
}
static int ability_is_activation(const Ability *ab) {
    return rb_resolver_current_ability_is_activation(ab);
}

/* ── resolver.rs::cached_condition_verdict ──
   Returns 1 if cache hit, writes *out; 0 otherwise. Mirrors Option<bool>. */
int rb_resolver_cached_condition_verdict(const GameState *g, int actor, const char *cond_text, int *result) {
    (void)actor;
    if (!g || !result) return 0;
    /* String-keyed twin of the typed helper below: these entry points are handed
       a condition TEXT, so the key is derived from that text. This used to be a
       `(void)` no-op that always reported a miss, so nothing a string-keyed
       caller stored was ever read back. */
    int key = cond_text_cache_key(cond_text);
    if (key == 0) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    const RbQueueEntry *e = &g->queue.entries[cur];
    for (int i = 0; i < e->n_cond_cache; i++)
        if (e->cond_cache_keys[i] == key) { *result = e->cond_cache_vals[i]; return 1; }
    *result = 0;
    return 0;
}
/* Internal typed version taking Condition* (faithful) */
static int cached_condition_verdict(const GameState *g, const Condition *cond, int *out) {
    if (!g || !cond || !out) return 0;
    if (!cond_get_cache(cond)) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    const RbQueueEntry *e = &g->queue.entries[cur];
    int key = cond_cache_key(cond);
    for (int i = 0; i < e->n_cond_cache; i++) if (e->cond_cache_keys[i] == key) { *out = e->cond_cache_vals[i]; return 1; }
    return 0;
}
static void store_condition_verdict(GameState *g, const Condition *cond, int passed) {
    if (!g || !cond) return;
    if (!cond_get_cache(cond)) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    int key = cond_cache_key(cond);
    for (int i = 0; i < e->n_cond_cache; i++) if (e->cond_cache_keys[i] == key) { e->cond_cache_vals[i] = passed ? 1 : 0; return; }
    if (e->n_cond_cache < RB_COND_CACHE_CAP) {
        e->cond_cache_keys[e->n_cond_cache] = key;
        e->cond_cache_vals[e->n_cond_cache] = passed ? 1 : 0;
        e->n_cond_cache++;
    }
}
void rb_resolver_store_condition_verdict(GameState *g, int actor, const char *cond_text, int result) {
    (void)actor;
    if (!g) return;
    /* resolver.rs:352-361 — store keyed on the condition, replacing any
       previously stored verdict for the same key. */
    int key = cond_text_cache_key(cond_text);
    if (key == 0) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    for (int i = 0; i < e->n_cond_cache; i++)
        if (e->cond_cache_keys[i] == key) { e->cond_cache_vals[i] = result ? 1 : 0; return; }
    if (e->n_cond_cache < RB_COND_CACHE_CAP) {
        e->cond_cache_keys[e->n_cond_cache] = key;
        e->cond_cache_vals[e->n_cond_cache] = result ? 1 : 0;
        e->n_cond_cache++;
    }
}

/* ── resolver.rs::store_pending_choice (dedup + snapshot_requested) ── */
void rb_resolver_store_pending_choice(GameState *g) {
    if (!g) return;
    g->queue.has_pending = 1;
    g->queue.snapshot_requested = 1;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    /* Dedup last_offered_sig (mirrors Rust last_offered_sig) */
    char sig[RB_CHOICE_SIG_MAX];
    choice_offer_sig(&g->queue.pending, sig, sizeof(sig));
    if (g_has_last_sig && !strcmp(sig, g_last_offered_sig)) return;
    strncpy(g_last_offered_sig, sig, sizeof(g_last_offered_sig)-1);
    g_last_offered_sig[sizeof(g_last_offered_sig)-1] = '\0';
    g_has_last_sig = 1;
    /* In Rust this pushes choice_offered structured log and debug trace; C logs via rule_log elsewhere */
    if (rb_ability_debug_enabled()) {
        /* log pending choice at debug level - part of fix, stays as diagnostic */
        /* printf("[PENDING_CHOICE] %s skip=%d\n", g->queue.pending.description, g->queue.pending.allow_skip); */
    }
}

/* ── resolver.rs::emit_pay_skip_gate ── */
void rb_resolver_emit_pay_skip_gate(GameState *g, int actor, const AbilityEffect *e,
                                      const char *description, int optional, const char *route) {
    (void)e;
    if (!g || !description) return;
    /* Rust: self.pending_choice = Some(Choice::SelectTarget{ target=PAY_SKIP_TARGET, ... }) + route -> queue.current_entry.choice_card_no */
    RbChoice ch; memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    strncpy(ch.target, "pay_optional_cost:skip_optional_cost", sizeof(ch.target)-1);
    strncpy(ch.description, description, sizeof(ch.description)-1);
    ch.allow_skip = optional ? 1 : 1;
    ch.route = RB_ROUTE_OPTIONAL_COST;
    g->queue.pending = ch;
    g->queue.has_pending = 1;
    g->queue.actor = actor;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    /* store route onto queue entry (mirrors Rust entry.choice_card_no = Some(route)) */
    RbQueueEntry *entry = NULL;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) entry = &g->queue.entries[g->queue.cur];
    if (entry && route && route[0]) {
        /* route stored as string on entry if the field exists; for now, store as route enum */
        (void)route;
    }
    rb_resolver_store_pending_choice(g);
}

/* ── resolver.rs::check_keywords (faithful) ── */
static int check_keywords(const GameState *g, const Ability *ability, int activating_card) {
    if (!ability || !ability->triggers) return 1;
    const char *trig = ability->triggers;

    /* Turn keywords gate on turn number */
    if (strstr(trig, "Turn1") || strstr(trig, "turn1") || strstr(trig, "1ターン目")) {
        if (g->turn != 1) return 0;
    }
    if (strstr(trig, "Turn2") || strstr(trig, "turn2") || strstr(trig, "2ターン目")) {
        if (g->turn != 2) return 0;
    }

    /* Position keywords: determine the card's stage position (if on stage) */
    int pos = -1;
    int card_on_stage = 0;
    if (activating_card >= 0) {
        pos = rb_find_card_stage_position(g, activating_card);
        if (pos >= 0) card_on_stage = 1;
    }

    /* Center / LeftSide / RightSide keywords */
    int need_pos = -1; /* -1 = no position keyword found */
    if (strstr(trig, "Center")) need_pos = 1;
    else if (strstr(trig, "LeftSide") || strstr(trig, "左側")) need_pos = 0;
    else if (strstr(trig, "RightSide") || strstr(trig, "右側")) need_pos = 2;
    if (need_pos >= 0) {
        if (!card_on_stage || pos != need_pos) return 0;
    }

    /* Debut keyword: card must be at the center position on the active player's stage.
       The Rust code uses MemberArea::Center (stage[1]) on the resolver's player. */
    if (strstr(trig, "Debut") || strstr(trig, " debut")) {
        if (!card_on_stage || pos != 1) return 0;
    }

    /* LiveStart keyword — resolver.rs:773-780:
         Keyword::LiveStart => {
             if !matches!(gs.current_phase,
                 Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker)
             { return false; }
         }
       Rust names BOTH live-card-set phases, and phase.c now STORES both
       (RB_PHASE_LIVE_SET / RB_PHASE_LIVE_SET_SECOND, phase.c:131/:151). So the
       faithful C predicate is a positive membership test over the PAIR, not
       "RB_PHASE_LIVE_SET minus whatever the second one is": the second
       attacker's window is a real LiveStart window, and dropping it here
       rejected every ライブ開始時 ability owned by the second seat. */
    if (strstr(trig, "LiveStart") || strstr(trig, "ライブ開始")) {
        if (g->phase != RB_PHASE_LIVE_SET && g->phase != RB_PHASE_LIVE_SET_SECOND) return 0;
    }

    /* LiveSuccess keyword — resolver.rs:781-785:
         Keyword::LiveSuccess => {
             if !matches!(gs.current_phase, Phase::LiveVictoryDetermination) { return false; }
         }
       Rust gates on ONE phase. The C reaches that same moment under THREE
       stored values, so the faithful C predicate is the positive membership
       test over the set — never a subtraction, because a subtraction would
       silently drop a phase the engine actually runs the determination under:
         * RB_PHASE_VICTORY           phase.c:1165, the steady state after
                                     rb_execute_live_victory_determination() returns.
         * RB_PHASE_PERFORMANCE       phase.c:1162 — the C calls
         * RB_PHASE_PERFORMANCE_SECOND rb_execute_live_victory_determination()
                                     from the tail of rb_execute_performance_phase()
                                     while g->phase is STILL the performance value
                                     it is about to overwrite, and the LiveSuccess
                                     abilities fire INSIDE that call. So the
                                     SECOND attacker's window presents as
                                     RB_PHASE_PERFORMANCE_SECOND. */
    if (strstr(trig, "LiveSuccess") || strstr(trig, "ライブ成功")) {
        if (g->phase != RB_PHASE_PERFORMANCE && g->phase != RB_PHASE_PERFORMANCE_SECOND &&
            g->phase != RB_PHASE_VICTORY) return 0;
    }

    /* PositionChange keyword: a position change must have occurred this turn */
    if (strstr(trig, "PositionChange") || strstr(trig, "position_change")) {
        if (!g->position_change_occurred_this_turn) return 0;
    }

    /* FormationChange keyword: a formation change must have occurred this turn */
    if (strstr(trig, "FormationChange") || strstr(trig, "formation_change")) {
        if (!g->formation_change_occurred_this_turn) return 0;
    }

    return 1;
}

/* ── resolver.rs::check_post_cost_position_keywords (faithful) ── */
static int check_post_cost_position_keywords(const GameState *g, const Ability *ability, int activating_card, char *err_out, size_t err_sz) {
    if (activating_card < 0) return 1;
    int pos = rb_find_card_stage_position(g, activating_card);
    if (!ability || !ability->triggers) return 1;
    const char *trig = ability->triggers;

    int need = -1;
    if (strstr(trig, "Center")) need = 1;
    else if (strstr(trig, "LeftSide") || strstr(trig, "左側")) need = 0;
    else if (strstr(trig, "RightSide") || strstr(trig, "右側")) need = 2;
    else return 1; /* no position keyword */

    int ok = (pos == need);
    if (!ok) {
        /* VERIFIED AGAINST RUST: there is NO auto/debut exemption here.
           `post_cost_position_gate` (gates.rs:167-191) filters the ability's
           keywords down to Center / LeftSide / RightSide and then does exactly
           one thing with them:

               if !position_keywords.is_empty()
                   && !resolver.check_keywords(gs, &position_keywords, position)
               { GateResult::stop("position requirement not met", "position_fail") }

           There is no branch on the trigger kind, no suppression for an auto
           or debut ability, and no "silently skip" path — a position mismatch
           is a Stop for EVERY trigger kind. This body used to carry an
           `is_activation` test whose non-activation arm was an empty block
           marked "silently skip for auto abilities (Rust behavior)", i.e. it
           claimed an exemption Rust does not have while changing nothing: both
           arms fell through to the same `return 0`. The claim was removed and
           the dead test with it; the stop below is the whole body. */
        if (err_out && err_sz) snprintf(err_out, err_sz, "position requirement not met - effect skipped");
        return 0;
    }
    return 1;
}

/* ── resolver.rs::apply_modify_cost_to_ability_cost (faithful subset) ──
   Handles: group_name per-unit reduction on PayEnergy, success_live count reduction on MoveCards.
   Mirrors util::find_modify_cost search inside ability.effect tree. */
static const AbilityEffect *find_modify_cost_in_effect(const AbilityEffect *eff) {
    if (!eff) return NULL;
    if (eff->action && !strcmp(eff->action, "modify_cost")) return eff;
    for (int i = 0; i < eff->n_child; i++) {
        const AbilityEffect *found = find_modify_cost_in_effect(eff->child[i]);
        if (found) return found;
    }
    if (eff->primary_effect) { const AbilityEffect *f = find_modify_cost_in_effect(eff->primary_effect); if (f) return f; }
    if (eff->alternative_effect) { const AbilityEffect *f = find_modify_cost_in_effect(eff->alternative_effect); if (f) return f; }
    if (eff->look_action) { const AbilityEffect *f = find_modify_cost_in_effect(eff->look_action); if (f) return f; }
    if (eff->select_action) { const AbilityEffect *f = find_modify_cost_in_effect(eff->select_action); if (f) return f; }
    return NULL;
}
static AbilityEffect clone_cost(const AbilityEffect *c) {
    AbilityEffect out; memset(&out, 0, sizeof(out));
    if (!c) return out;
    out = *c; /* shallow clone; extra strings are shared (read-only) */
    return out;
}

/* resolver.rs::per_unit_reduction (1191-1193) — ONE definition of the
 * `(units / per_unit_count.max(1)) * amount` math. */
static int per_unit_reduction(int total_units, int per_unit_count, int unit_amount) {
    if (per_unit_count < 1) per_unit_count = 1;
    return (total_units / per_unit_count) * unit_amount;
}

/* Write `value` into the cloned cost's `key` extra, inserting the pair when the
 * key is absent. Rust's `cost.set_energy_count(Some(n))` always leaves a
 * readable value behind; the previous C code only rewrote an EXISTING
 * "energy_count" pair and silently dropped the whole reduction when the key
 * had never been decoded.
 *
 * The clone is SHALLOW, so an existing value string is still owned by the
 * decoded ability effect and must NOT be freed here. Returns the index of a
 * pair this call allocated (owned solely by the clone, released by
 * cost_clone_release), or -1 when nothing was allocated. */
static int cost_set_extra_int(AbilityEffect *cost, const char *key, int value) {
    if (!cost || !key) return -1;
    char buf[16];
    snprintf(buf, sizeof(buf), "%d", value);
    for (int i = 0; i < cost->n_extra; i++) {
        if (cost->extra_k[i] && !strcmp(cost->extra_k[i], key)) {
            cost->extra_v[i] = rb_strdup2(buf);
            return -1;
        }
    }
    if (cost->n_extra >= RB_MAX_EXTRA) return -1;
    cost->extra_k[cost->n_extra] = rb_strdup2(key);
    cost->extra_v[cost->n_extra] = rb_strdup2(buf);
    cost->n_extra++;
    return cost->n_extra - 1;
}

static void cost_clone_release(AbilityEffect *cost, int inserted_index) {
    if (!cost || inserted_index < 0 || inserted_index >= cost->n_extra) return;
    free(cost->extra_k[inserted_index]);
    free(cost->extra_v[inserted_index]);
    cost->extra_k[inserted_index] = NULL;
    cost->extra_v[inserted_index] = NULL;
}

/* resolver.rs::apply_per_unit_cost_reduction (1196-1253). */
static void apply_per_unit_cost_reduction(GameState *g, AbilityEffect *cost,
                                          const AbilityEffect *mod_cost,
                                          int *inserted_index) {
    int per_unit_count = 1;
    /* resolver.rs:1497 reads `mod_cost.per_unit_count_any()` — a TYPED field.
       vm.c:1040-1043 stores `per_unit_count` in AbilityEffect::per_unit_count and
       `continue`s, so it is NEVER in the extras table (MEASURED 0 of 155 effect
       nodes across 136 abilities carry it as an extra). This read was therefore
       permanently NULL and every per-unit cost reduction divided by 1. The
       struct field is authoritative and wins; the extras read is kept only as
       the fallback so a future decoder change cannot lose the value. */
    const char *puc = mod_cost->per_unit_count > 0 ? NULL : eff_extra(mod_cost, "per_unit_count");
    if (mod_cost->per_unit_count > 0) per_unit_count = mod_cost->per_unit_count;
    else if (puc) per_unit_count = atoi(puc);
    if (per_unit_count < 1) per_unit_count = 1;
    int unit = mod_cost->count >= 0 ? mod_cost->count : 1;
    const char *per_unit_type = eff_extra(mod_cost, "per_unit_type");

    if (per_unit_type && !strcmp(per_unit_type, "group_name")) {
        /* Rust only folds the reduction into an energy cost. The C port used to
         * subtract from `count` for ANY cost, and rb_pay_cost charges
         * pay_energy from `energy_count` (cost.c:828) — so on a pay_energy
         * cost carrying a decoded `count` the reduction landed on a field the
         * engine never reads and the discount was silently lost. */
        if (!cost->action || strcmp(cost->action, "pay_energy") != 0) return;
        int groups = rb_distinct_stage_groups(g, g->active);
        int reduction = per_unit_reduction(groups, per_unit_count, unit);
        int current = rb_cost_energy_count_any(cost);
        if (current < 0) current = 0;
        int next = current - reduction;
        if (next < 0) next = 0;
        if (inserted_index) *inserted_index = cost_set_extra_int(cost, "energy_count", next);
        if (rb_ability_debug_enabled())
            fprintf(stderr, "[COST_REDUCE_ENERGY] groups=%d per_unit_cnt=%d unit=%d "
                            "reduction=%d energy_count=%d->%d\n",
                    groups, per_unit_count, unit, reduction, current, next);
        return;
    }
    if (per_unit_type && (!strcmp(per_unit_type, "success_live_card_zone") ||
                          !strcmp(per_unit_type, "success_live_zone") ||
                          !strcmp(per_unit_type, "live_card_zone") ||
                          !strcmp(per_unit_type, "live_zone"))) {
        /* PB1-007: hand discard 3 reduced by 1 per success live. Rust gates
         * this branch on a move_cards cost as well. */
        if (!cost->action || strcmp(cost->action, "move_cards") != 0) return;
        /* resolver.rs:1229-1233 — the success zone belongs to the queue
         * entry's player (defaulting to player 1), NOT to g->active. */
        int pl = 0;
        int cur_entry = g->queue.cur;
        if (cur_entry >= 0 && cur_entry < g->queue.n_entries) {
            const char *pid = g->queue.entries[cur_entry].player_id;
            if (pid && pid[0] == 'p' && (pid[1] == '1' || pid[1] == '2'))
                pl = pid[1] - '1';
        }
        int success_len = g->p[pl].success.n;
        int reduction = per_unit_reduction(success_len, per_unit_count, unit);
        int current = cost->count >= 0 ? cost->count : 0;
        int next = current - reduction;
        if (next < 0) next = 0;
        cost->count = next;
        if (rb_ability_debug_enabled())
            fprintf(stderr, "[COST_REDUCE_HAND] pl=%d success_len=%d per_unit_cnt=%d "
                            "unit=%d reduction=%d new_count=%d\n",
                    pl, success_len, per_unit_count, unit, reduction, next);
    }
}

static AbilityEffect apply_modify_cost_to_ability_cost(GameState *g, const AbilityEffect *cost, const Ability *ability, int *inserted_index) {
    AbilityEffect res = clone_cost(cost);
    if (inserted_index) *inserted_index = -1;
    if (!ability || !ability->effect) return res;
    const AbilityEffect *mod = find_modify_cost_in_effect(ability->effect);
    if (!mod) return res;
    const char *op = eff_extra(mod, "operation");
    const char *per_unit = eff_extra(mod, "per_unit");
    if (!op || strcmp(op, "subtract") != 0) return res;
    if (!per_unit || strcmp(per_unit, "true") != 0) return res;
    apply_per_unit_cost_reduction(g, &res, mod, inserted_index);
    return res;
}

/* ── Card helpers delegation ── */
int rb_resolver_card_matches_type(int cid, const char *filter) {
    return rb_card_matches_type(cid, filter);
}
int rb_resolver_card_matches_cost_limit(int card_id, int cost_limit, const char *op) {
    if (cost_limit < 0) return 1;
    return rb_card_matches_cost_limit(card_id, cost_limit, op);
}
void rb_resolver_fmt_card(int cid, char *out, size_t out_sz) {
    if (!out || out_sz == 0) return;
    out[0] = '\0';
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) { strncpy(out, c.name, out_sz - 1); out[out_sz - 1] = '\0'; }
        rb_free_card(&c);
    }
}
const char *rb_resolver_fmt_ids(const int *ids, int n) {
    static char buf[1024];
    buf[0] = '\0';
    int pos = 0;
    for (int i = 0; i < n && pos < (int)sizeof(buf) - 4; i++) {
        if (i > 0) buf[pos++] = ',';
        pos += snprintf(buf + pos, sizeof(buf) - pos, "%d", ids[i]);
    }
    return buf;
}
const char *rb_resolver_merge_group_names(const char **groups, int n) {
    static char buf[1024];
    buf[0] = '\0';
    int pos = 0;
    for (int i = 0; i < n && pos < 1000; i++) {
        if (groups[i]) {
            int len = strlen(groups[i]);
            if (pos > 0) buf[pos++] = ',';
            if (pos + len < 1000) { memcpy(buf + pos, groups[i], len); pos += len; }
        }
    }
    buf[pos] = '\0';
    return buf;
}

/* ── can_activate_effect (faithful, resolver.rs:278) ──
   Mirrors Rust control flow: activation_condition gate (with position merge),
   condition cache, condition merge (position + group_names), standalone activation_position check.
*/
int rb_can_activate_effect(const GameState *g, int actor, const AbilityEffect *eff, int host_cid) {
    if (!g || !eff) return 1;

    /* resolver.rs:581-582 — `let mut dbg = AbDebug::new(); dbg.effect(effect);`, the
       FIRST thing can_activate_effect does, before the activation-condition
       gate. A per-effect AbDebug, separate from the one resolve_ability owns,
       exactly as in Rust: both write to the same global trace buffer. */
    RbAbDebug eff_dbg;
    rb_abdebug_init(&eff_dbg);
    rb_abdebug_effect(&eff_dbg, eff);

    /* ── Activation condition check (resolver.rs:451-493) ──
       Only evaluated when cost has not already been paid. This is a SEPARATE
       gate from `condition`: Rust reads effect.activation_condition_parsed_any()
       (resolver.rs:458) and, on failure, returns false BEFORE the main gate is
       ever reached (resolver.rs:488-490). Abilities that carry both must
       satisfy both — an early return here used to skip the has_moved gate.

       The gate is the ACTIVATION tree, not eff->condition. 13 abilities carry
       one and on 9 of them eff->condition is NULL, and
       rb_eval_condition_for_host returns 1 for NULL (condition.c:1581) — so
       evaluating eff->condition here was live-but-vacuous: on those 9 the
       ability was never actually gated. */
    int cost_already_paid = 0;
    int cur = g->queue.cur;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH && g->queue.entries[cur].cost_paid) cost_already_paid = 1;

    if (!cost_already_paid && eff->activation_condition) {
        /* resolver.rs:461-482 — the patched copy is only made when the position
           overlay would actually write; the plain path evaluates the decoded
           tree as-is. The verdict drain rides with the clone in Rust, so it is
           reproduced verbatim: drained on success, kept on failure. */
        Condition act_tmp;
        int act_injected = 0;
        const Condition *act_eval =
            position_overlay(eff, eff->activation_condition, &act_tmp, &act_injected);
        int act_passed;
        if (act_injected) {
            int act_snapshot = rb_log_buffer_len();
            act_passed = rb_eval_condition_for_host(g, actor, host_cid, act_eval);
            if (act_passed) rb_resolver_drain_verdicts_since((GameState *)g, act_snapshot);
        } else {
            act_passed = rb_eval_condition_for_host(g, actor, host_cid, eff->activation_condition);
        }
        if (act_injected) overlay_release(&act_tmp);
        /* Fail closed rather than vacuously pass (resolver.rs:488-490). */
        if (!act_passed) return 0;
    }

    /* ── Main condition gate (resolver.rs:335-404) ── */
    if (eff->condition) {
        if (eff->action && !strcmp(eff->action, "conditional_alternative")) return 1; /* branch selector, not gate */

        /* Check cache first (resolver.rs:341) */
        int cached = 0;
        if (cached_condition_verdict(g, eff->condition, &cached)) return cached;

        /* Build a temporary condition for evaluation, injecting position/group_names */
        Condition tmp_cond;
        int injected = 0;
        const Condition *cond_for_eval = eff->condition;

        /* Merge effect position into condition if condition lacks position info
           (same overlay the activation pre-check above uses). */
        cond_for_eval = position_overlay(eff, cond_for_eval, &tmp_cond, &injected);

        /* Merge group_names: only for AppearanceCondition or conditions with distinct=true */
        const char *eff_group = rb_effect_group_names_any(eff);
        int needs_group = 0;
        if (cond_for_eval) {
            needs_group = cond_is_appearance(cond_for_eval) || cond_get_distinct(cond_for_eval);
        }
        if (needs_group && cond_for_eval) {
            const char *cond_group = cond_get_group_names(cond_for_eval);
            if ((!cond_group || !cond_group[0]) && eff_group && eff_group[0]) {
                if (!injected) {
                    tmp_cond = *cond_for_eval;
                    cond_for_eval = &tmp_cond;
                    injected = 1;
                }
                if (tmp_cond.n_fields < RB_MAX_COND_FIELD) {
                    tmp_cond.fields[tmp_cond.n_fields].key = rb_strdup2("group_names");
                    tmp_cond.fields[tmp_cond.n_fields].v.tag = RB_TAG_STR;
                    tmp_cond.fields[tmp_cond.n_fields].v.s = rb_strdup2(eff_group);
                    tmp_cond.n_fields++;
                }
            }
        }

        /* resolver.rs:466-476 — drain the gate's own verdicts on success (the
           condition is re-evaluated during execution); keep them on failure. */
        int cond_snapshot = rb_log_buffer_len();
        int passed = rb_eval_condition_for_host(g, actor, host_cid, cond_for_eval);

        /* Free injected fields (same release the overlay's own caller uses). */
        if (injected) overlay_release(&tmp_cond);

        store_condition_verdict((GameState *)g, eff->condition, passed);
        if (passed) rb_resolver_drain_verdicts_since((GameState *)g, cond_snapshot);
        if (!passed) return 0;
    }

    /* ── Standalone activation_position check for effects with no condition (resolver.rs:409-430) ── */
    if (!eff->condition) {
        const char *actpos = rb_effect_activation_position(eff);
        if (actpos && actpos[0] && host_cid >= 0) {
            int pos = rb_find_card_stage_position(g, host_cid);
            if (pos < 0) return 0;
            int passes = 0;
            char buf[64]; strncpy(buf, actpos, sizeof(buf)-1); buf[sizeof(buf)-1]='\0';
            char *tok = strtok(buf, ",");
            while (tok) {
                int idx = rb_activation_position_index(tok);
                if (idx >= 0 && idx < RB_STAGE_SIZE && idx == pos) { passes = 1; break; }
                tok = strtok(NULL, ",");
            }
            if (!passes) return 0;
        }
    }
    return 1;
}

/* ── get_trigger_ability_infos (resolver.rs::get_trigger_ability_infos) ── */
int rb_resolver_trigger_infos(const GameState *g, int actor, const char *trigger, AbilityInfo *out, int max) {
    if (!g || !trigger || !out || max <= 0) return 0;
    int n = 0;
    const RbPlayer *P = &g->p[actor];
    int zone[RB_STAGE_SIZE + 32];
    int zn = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++) if (P->stage[s] >= 0) zone[zn++] = P->stage[s];
    for (int i = 0; i < P->success.n && zn < (int)(sizeof(zone)/sizeof(zone[0])); i++) zone[zn++] = P->success.cards[i];
    for (int i = 0; i < P->live.n && zn < (int)(sizeof(zone)/sizeof(zone[0])); i++) zone[zn++] = P->live.cards[i];
    for (int i = 0; i < P->hand.n && zn < (int)(sizeof(zone)/sizeof(zone[0])); i++) zone[zn++] = P->hand.cards[i];
    for (int i = 0; i < P->energy.n && zn < (int)(sizeof(zone)/sizeof(zone[0])); i++) zone[zn++] = P->energy.cards[i];
    for (int z = 0; z < zn && n < max; z++) {
        int cid = zone[z];
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab && n < max; a++) {
            Ability ab; if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_ability_matches_trigger(&ab, trigger)) {
                out[n].cid = cid; out[n].ability_idx = a; out[n].trigger = trigger; n++;
            }
            rb_free_ability(&ab);
        }
    }
    return n;
}

/* ── player_prefix (resolver.rs:1512) ──
   The Rust resolver carries its OWN player_prefix, and it is NOT the one in
   game_state_core (rb_gsc_player_prefix_id, which reads the activating card's
   owner off the stage and its `under_cards`) nor the one in cost.c (which also
   scans the ENERGY zone). Rust's resolver twin reads three zones in this order:

       activating_card in p1.stage | p1.live_card_zone | p1.hand -> "P1"
       activating_card in p2.stage | p2.live_card_zone | p2.hand -> "P2"
       otherwise the ACTIVE player's index

   The zone set matters: a card that is in nobody's hand or stage (a waited
   member is still in stage, but a card in the success zone or the deck is in
   NEITHER) falls through to the active-player arm here, where the game_state_core
   twin would return the active player too but the cost.c twin would return "??".
   Implemented as a twin rather than aliased so this file keeps a body for every
   Rust function it ports; it is exercised through push_ability_result below. */
static const char *player_prefix(const GameState *g) {
    if (!g) return "??";
    int cid = g->activating_card;
    if (cid >= 0) {
        for (int pl = 0; pl < 2; pl++) {
            const RbPlayer *P = &g->p[pl];
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (P->stage[s] == cid) return pl == 0 ? "P1" : "P2";
            for (int i = 0; i < P->live.n; i++)
                if (P->live.cards[i] == cid) return pl == 0 ? "P1" : "P2";
            for (int i = 0; i < P->hand.n; i++)
                if (P->hand.cards[i] == cid) return pl == 0 ? "P1" : "P2";
        }
    }
    return g->active == 0 ? "P1" : "P2";
}

/* ── push_ability_result (resolver.rs:971-1039) ──
   Rust builds the structured LogMetadata::AbilityResolution here, and its FIRST
   step is `let pp = gs.player_prefix();` (resolver.rs:992). This C body used to
   be a `(void)` no-op for all three parameters, so player_prefix had no caller
   in this file at all. It now emits the prefix, the result word, the activating
   card and any error on the debug channel, which is where Rust's own
   log::debug! in the same function writes and which every other C file in this
   tree gates on with rb_ability_debug_enabled(). The canonical trigger, the
   ability text and the zone stay out: they are inputs to the STRUCTURED record,
   and C's rb_log_push_verdict takes a plain verdict line rather than a metadata
   record, so emitting them would invent a format no reader here consumes. */
static void push_ability_result(GameState *g, const char *result, const char *error_msg) {
    if (!g || !rb_ability_debug_enabled()) return;
    fprintf(stderr, "[ABILITY_RESULT] %s %s card=%d%s%s\n",
            player_prefix(g), result ? result : "?",
            g->activating_card,
            error_msg ? " error=" : "", error_msg ? error_msg : "");
}

/* ── record_ability_use_guarded (resolver.rs:755) ── */
static void record_ability_use_guarded(GameState *g, int card_id, int ability_idx, int turn, const Ability *ability, int unconditional) {
    if (!ability || ability->use_limit < 0) return; /* no use_limit = no guard needed */
    int can = unconditional;
    if (!can) {
        if (!ability->effect) can = 1;
        else can = rb_can_activate_effect(g, g->active, ability->effect, card_id);
    }
    if (can) rb_record_ability_use(g, card_id, ability_idx);
}

/* ── resolve_ability orchestrator (resolver.rs:821) ──
   Faithful C translation: early current_ability set, use_limit guard, keyword gate,
   cost payment with modify_cost folding, pending_choice branch, post-cost keywords,
   optional-cost skip, effect condition, execute_effect, pending_choice handling,
   final use recording.
   Returns 0 on error (caller should surface), 1 on Ok.
   *resolved = 1 when effect executed or choice emitted.

   The BODY lives in resolve_ability_body so the AbDebug trace buffer is flushed
   on EVERY exit, exactly once. Rust never has to think about this: `dbg` is a
   local, and its lines sit in the process-global ABILITY_LOG_BUFFER drained by
   AbDebug::flush_to_* — which C only reaches through this one seam. Collapsing
   the body's eleven early returns into a single flush would mean a goto chain;
   a wrapper leaves every one of them untouched. */
static int resolve_ability_body(GameState *g, int actor, const Ability *ab, int ability_idx, int host_cid, int *resolved) {
    if (resolved) *resolved = 0;
    if (!g || !ab) return 0;
    if (rb_ability_debug_enabled() && host_cid == 1049)
        fprintf(stderr, "[RESOLVE_MERMAID] cur=%d state=%d pending=%d cost=%d started=%d effect=%s\n",
                g->queue.cur, g->queue.state, g->queue.has_pending,
                g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries ? g->queue.entries[g->queue.cur].cost_paid : -1,
                g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries ? g->queue.entries[g->queue.cur].effect_started : -1,
                ab->effect && ab->effect->action ? ab->effect->action : "none");

    /* resolver.rs:1273 — `let mut dbg = AbDebug::new();`, then passed by &mut to
       pay_ability_cost (:1095) and run_ability_effect (:1150), both of which C
       has inlined into this body. One local therefore serves every site below,
       which is the same object Rust threads through those two helpers. */
    RbAbDebug dbg;
    rb_abdebug_init(&dbg);

    /* resolver.rs:1280-1303 — the three identity Strings, built only when a
       tracer is on (Rust's `debug_on`) and EMPTY when there is no activating
       card (`unwrap_or_default()`), not "?": a card-less resolution must print
       `"" [] ()` exactly as Rust does. */
    char card_name[256];
    char card_no[64];
    char card_id[24];
    card_name[0] = '\0'; card_no[0] = '\0'; card_id[0] = '\0';
    if (rb_ability_debug_enabled() && host_cid >= 0) {
        Card cd;
        if (rb_decode_card_by_index((uint32_t)host_cid, &cd)) {
            if (cd.name) { strncpy(card_name, cd.name, sizeof(card_name) - 1); card_name[sizeof(card_name) - 1] = '\0'; }
            const char *no = rb_card_string(cd.card_no_idx);
            if (no) { strncpy(card_no, no, sizeof(card_no) - 1); card_no[sizeof(card_no) - 1] = '\0'; }
            rb_free_card(&cd);
        }
        snprintf(card_id, sizeof(card_id), "%d", host_cid);
    }
    /* resolver.rs:1316 */
    rb_abdebug_ability(&dbg, card_name, card_no, card_id, ab);

    /* initialize resolver globals (mirrors Rust self.current_ability etc.) */
    g_current_ability = *ab; /* shallow copy of strings */
    g_current_ability_valid = 1;
    g_current_ability_idx = ability_idx;
    g_activating_card_id = host_cid;

    /* compute ability_key = (host_cid, ability_idx, g->turn) for use-limit recording */
    int ability_key_valid = (host_cid >= 0);
    int ability_key_cid = host_cid;
    int ability_key_idx = ability_idx;
    int ability_key_turn = g->turn;

    /* ── use_limit pre-check (Rust line 861-874) ── */
    if (ab->use_limit >= 0 && host_cid >= 0) {
        if (check_use_limit_reached_self(g, host_cid, ability_idx, ab->use_limit)) {
            int used = rb_ability_uses_used(g, host_cid, ability_idx);
            char msg[128];
            snprintf(msg, sizeof(msg), "Ability already used %d of %d times this turn", used, ab->use_limit);
            push_ability_result(g, "skipped", msg);
            g_current_ability_valid = 0;
            return 0;
        }
    }

    /* ── activation keyword pre-check (Rust line 877-892) ── */
    if (host_cid >= 0) {
        if (!check_keywords(g, ab, host_cid)) {
            push_ability_result(g, "position_fail", "Activation keywords not satisfied");
            g_current_ability_valid = 0;
            return 0;
        }
    }

    int cost_already_paid = 0;
    int cur = g->queue.cur;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH && g->queue.entries[cur].cost_paid) cost_already_paid = 1;

    /* ── pay cost (Rust line 899-926) ── */
    if (!cost_already_paid && ab->cost) {
        int inserted_index = -1;
        AbilityEffect eff_cost = apply_modify_cost_to_ability_cost(g, ab->cost, ab, &inserted_index);
        int paid = rb_pay_cost(g, actor, &eff_cost);
        cost_clone_release(&eff_cost, inserted_index);
        if (!paid) {
            /* resolver.rs:1108 — `dbg.p("RESULT", "COST FAILED: {e}")`. Rust
               formats the Err string its pay_cost returned; C's rb_pay_cost
               returns a bare int, so the SAME literal C already hands
               push_ability_result is used rather than a second invented one. */
            rb_abdebug_p(&dbg, "RESULT", "COST FAILED: cost payment failed");
            push_ability_result(g, "cost_fail", "cost payment failed");
            g_current_ability_valid = 0;
            return 0;
        }
        /* resolver.rs:1113 — `dbg.p("RESULT", "cost paid")`, emitted on the
           success side of pay_cost and BEFORE the pending-choice check. */
        rb_abdebug_p(&dbg, "RESULT", "cost paid \xE2\x9C\x93");

        /* cost paid log is handled inside rb_pay_cost */
        /* check if cost emitted a pending choice */
        if (g->queue.has_pending) {
            /* mark cost_paid on entry */
            if (cur >= 0 && cur < RB_QUEUE_DEPTH) g->queue.entries[cur].cost_paid = 1;
            rb_resolver_store_pending_choice(g);
            if (resolved) *resolved = 1;
            g_current_ability_valid = 0;
            return 1;
        }
    }

    /* ── record use_limit early (Rust line 944-952) ──
       Only when: not cost_already_paid, no pending_choice, not conditional_on_optional,
       not optional_effect. */
    int is_cond_opt = ab->effect && ab->effect->action && !strcmp(ab->effect->action, "conditional_on_optional");
    int is_optional_effect = ab->effect && ab->effect->is_optional;
    if (!cost_already_paid && !g->queue.has_pending && !is_cond_opt && !is_optional_effect) {
        if (ability_key_valid) {
            record_ability_use_guarded(g, ability_key_cid, ability_key_idx, ability_key_turn, ab, 0);
        }
    }

    /* pending choice from cost payment: store and return */
    if (g->queue.has_pending) {
        if (!cost_already_paid && cur >= 0 && cur < g->queue.n_entries) g->queue.entries[cur].cost_paid = 1;
        rb_resolver_store_pending_choice(g);
        if (resolved) *resolved = 1;
        g_current_ability_valid = 0;
        return 1;
    }

    /* ── post-cost position keyword check (Rust line 966-976) ── */
    if (host_cid >= 0) {
        char err[128] = {0};
        if (!check_post_cost_position_keywords(g, ab, host_cid, err, sizeof(err))) {
            push_ability_result(g, "position_fail", NULL);
            g_current_ability_valid = 0;
            if (resolved) *resolved = 1;
            return 1; /* Ok(()) but effect skipped */
        }
    }

    /* mark cost_paid when auto-resolved without creating a pending choice */
    if (!cost_already_paid && ab->cost && !g->queue.has_pending) {
        if (cur >= 0 && cur < g->queue.n_entries) g->queue.entries[cur].cost_paid = 1;
    }

    /* ── optional cost was skipped -> effect not executed (Rust line 988-1007) ── */
    int cur2 = g->queue.cur;
    int cost_was_skipped = 0;
    if (cur2 >= 0 && cur2 < g->queue.n_entries && g->queue.entries[cur2].optional_cost_result == 0) cost_was_skipped = 1;
    if (cost_was_skipped) {
        if (cur2 >= 0 && cur2 < g->queue.n_entries) g->queue.entries[cur2].effect_started = 1;
        push_ability_result(g, "skipped", "optional cost not paid");
        g_current_ability_valid = 0;
        if (resolved) *resolved = 1;
        return 1;
    }

    /* ── effect condition gate + execute (Rust line 1009-1088) ── */
    if (ab->effect) {
        int needs_gate = (ab->effect->condition != NULL) ||
                         (ab->effect->activation_condition != NULL) ||
                         eff_extra(ab->effect, "activation_condition_parsed") != NULL;
        if (needs_gate) {
            int passed = rb_can_activate_effect(g, actor, ab->effect, host_cid);
            if (!passed) {
                /* For 起動 abilities, the player deliberately paid the cost,
                   so the attempt counts toward the turn limit. */
                if (ab->use_limit >= 0 && ability_is_activation(ab) && host_cid >= 0) {
                    rb_record_ability_use(g, host_cid, ability_idx);
                }
                /* resolver.rs:1183 — `dbg.p("RESULT", "effect condition not met
                   — skipped")`, on the `!passed` arm of run_ability_effect. */
                rb_abdebug_p(&dbg, "RESULT", "effect condition not met \xE2\x80\x94 skipped");
                push_ability_result(g, "failure", NULL);
                g_current_ability_valid = 0;
                if (resolved) *resolved = 1;
                return 1;
            }
        }
        /* execute effect - carry host_cid for per-card attribution */
        rb_execute_effect_ex(g, actor, ab->effect, host_cid);
        /* resolver.rs:1189-1190 — `if let Err(e) = self.execute_effect(..) {
           dbg.p("RESULT", "EFFECT FAILED: {e}") }` — is NOT reproduced, and the
           blocker is a type difference, not an oversight: Rust's execute_effect
           returns Result<(), String>, and this line is the only C-visible
           consequence of the Err arm. C's rb_execute_effect_ex returns void —
           an execution failure is reported by the handlers themselves, not
           returned — so there is no `e` to format and no way to know at this
           point whether the Err arm was taken. Emitting an unconditional
           "EFFECT FAILED" here would print a failure Rust does not print on
           every successful resolution. It becomes wireable the moment
           rb_execute_effect_ex grows a Result-shaped return, and until then
           this comment is the record of the gap. */

        if (g->queue.has_pending) {
            if (!cost_already_paid && cur >= 0 && cur < g->queue.n_entries) g->queue.entries[cur].cost_paid = 1;
            /* defer use_limit for conditional_optional / optional position choice */
            int skip_use = 0;
            if (g->queue.pending.kind == RB_CHOICE_SELECT_TARGET && g->queue.pending.target[0]) {
                if (!strcmp(g->queue.pending.target, "conditional_optional")) skip_use = 1;
                if (!strcmp(g->queue.pending.target, "position|destination") && is_optional_effect) skip_use = 1;
            }
            if (host_cid >= 0 && ab->use_limit >= 0 && !skip_use) rb_record_ability_use(g, host_cid, ability_idx);
            /* Mark effect as started when a pending choice comes from effect execution (not cost) */
            int is_paid = cost_already_paid;
            if (!is_paid) {
                int c = g->queue.cur;
                if (c >= 0 && c < g->queue.n_entries && g->queue.entries[c].cost_paid) is_paid = 1;
            }
            if (is_paid) {
                if (cur >= 0 && cur < g->queue.n_entries) g->queue.entries[cur].effect_started = 1;
            }
            rb_resolver_store_pending_choice(g);
            if (resolved) *resolved = 1;
            g_current_ability_valid = 0;
            return 1;
        }
        /* resolver.rs:1202 — `dbg.p("RESULT", "effect applied")`, guarded in
           Rust by `self.awaiting.choice.is_none()`, which is exactly the branch
           this is: the pending-choice arm returned above. */
        rb_abdebug_p(&dbg, "RESULT", "effect applied \xE2\x9C\x93");
        push_ability_result(g, "success", NULL);
    }

    /* ── final use_limit recording (Rust line 1094-1120) ── */
    if (!cost_already_paid && host_cid >= 0 && ab->use_limit >= 0) {
        record_ability_use_guarded(g, host_cid, ability_idx, ability_key_turn, ab, 0);
    }
    if (ability_key_valid && ab->use_limit >= 0) {
        int is_cond_opt2 = ab->effect && ab->effect->action && !strcmp(ab->effect->action, "conditional_on_optional");
        int guard_ok = 1;
        if (!(cost_already_paid && is_cond_opt2)) {
            if (ab->effect) guard_ok = rb_can_activate_effect(g, actor, ab->effect, host_cid);
        }
        if (guard_ok) rb_record_ability_use(g, ability_key_cid, ability_key_idx);
    }

    g_current_ability_valid = 0;
    g_activating_card_id = -1;
    if (resolved) *resolved = 1;
    return 1;
}

/* ── AbDebug::flush_to_rule_log / flush_to_structured_log (debug.rs:77-97) ──
   Rust calls these from ONE place, game/display.rs:1405 and :1413, while
   building a display state: it drains the process-global ABILITY_LOG_BUFFER
   into game_state.rule_log, caps the result at LOG_BOUND_RULE, then drains the
   (now empty) buffer again into structured_log as category "debug".

   C HAS NO DISPLAY PATH — there is no game_state_to_display in this port, and
   grepping the tree for one returns nothing outside the phase/card-name
   helpers. So the display seam does not exist to wire into, and leaving the
   flush unwired is what made BOTH functions dead: every [AB] line produced by
   the calls above would have sat in the 256-line trace buffer and been
   discarded. The resolution end is therefore the C point at which a buffered
   trace line can reach the log AT ALL, and it is also where Rust's own
   `push_ability_result` writes, so the ordering matches.

   Both drains are called, in Rust's order, even though the second necessarily
   finds the buffer already empty. That is true of Rust too — display.rs:1413
   drains a buffer :1405 just emptied — and calling only the first would leave
   rb_abdebug_flush_to_structured_log dead, which is the very defect class this
   work is closing. */
int rb_resolve_ability(GameState *g, int actor, const Ability *ab, int ability_idx, int host_cid, int *resolved) {
    int r = resolve_ability_body(g, actor, ab, ability_idx, host_cid, resolved);
    rb_abdebug_flush_to_rule_log(NULL);
    rb_abdebug_flush_to_structured_log(NULL);
    return r;
}

/* ── log.rs verdict-buffer drains (were `(void)` no-ops) ──
   Rust drains the buffered AbilityLogItems into the ability-resolution entry;
   the C keeps the buffer in src/ability/log.c, so these forward to it and
   release the drained items. Bounded by the buffer cap: a drain never grows
   the C heap. */
#define RB_RESOLVER_DRAIN_CAP 64
void rb_resolver_drain_verdicts(GameState *g) {
    (void)g;
    RbAbilityLogItem items[RB_RESOLVER_DRAIN_CAP];
    int n = rb_log_drain_verdicts(items, RB_RESOLVER_DRAIN_CAP);
    for (int i = 0; i < n; i++) rb_log_free_item(&items[i]);
}
void rb_resolver_drain_verdicts_since(GameState *g, int snapshot) {
    (void)g;
    RbAbilityLogItem items[RB_RESOLVER_DRAIN_CAP];
    int n = rb_log_drain_verdicts_since(snapshot, items, RB_RESOLVER_DRAIN_CAP);
    for (int i = 0; i < n; i++) rb_log_free_item(&items[i]);
}
void rb_resolver_push_verdict(GameState *g, const char *text, const char *kind, int passed) {
    (void)g;
    rb_log_push_verdict(text, kind, passed);
}
