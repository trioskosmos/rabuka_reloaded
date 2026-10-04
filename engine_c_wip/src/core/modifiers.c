#include "rabuka.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Defined in src/core/game_state_abilities.c; folds Rust
   GameState::fire_opponent_cause_watchers_for_move (abilities.rs:1049-1149).
   include/rabuka.h does not export it, so it is declared with the same extern
   pattern already used at src/ability/choice.c:6. */
extern void rb_fire_opponent_cause_watchers_for_move(GameState *g, int moved_card_id,
                                                      int causer_player);

static int16_t saturate_modifier(int64_t value) {
    if (value > INT16_MAX) return INT16_MAX;
    if (value < INT16_MIN) return INT16_MIN;
    return (int16_t)value;
}

void rb_mods_init(RbMods *m) {
    memset(m, 0, sizeof(*m));
    for(int i=0;i<RB_MAX_CARD_IDS;i++){ m->heart_copy[i]=-1; m->heart_multiplier[i]=-1; m->heart_multiplier_amt[i]=2; m->blade_type[i]=-1; m->heart_color_override[i]=-1; }
    m->n_trace = 0;
    m->p1_constant_total_score_bonus = 0;
    m->p2_constant_total_score_bonus = 0;
}

void rb_mods_clear_card(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    memset(&m->blade[cid], 0, sizeof(RbModifierEntry));
    for (int c = 0; c < 8; c++) {
        m->heart[cid][c].set = 0; m->heart[cid][c].add = 0;
        m->need_heart[cid][c].set = 0; m->need_heart[cid][c].add = 0;
    }
    m->score[cid].set = 0; m->score[cid].add = 0;
    m->cost[cid].set = 0; m->cost[cid].add = 0;
    m->orientation[cid] = 0;
    m->delayed_cannot_active[cid] = 0;
    m->constant_blade[cid] = 0;
    m->constant_score[cid] = 0;
    m->constant_cost[cid] = 0;
    for (int c = 0; c < 8; c++) m->constant_heart[cid][c] = 0;
    m->heart_copy[cid] = -1;
    m->heart_multiplier[cid] = -1;
    m->heart_multiplier_amt[cid] = 2;
    m->blade_type[cid] = -1;
    m->heart_color_override[cid] = -1;
}

/* ── blade ── */
int rb_mods_get_blade(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return rb_modifier_total_entry(&m->blade[cid]);
}
void rb_mods_add_blade(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->blade[cid].add = saturate_modifier((int64_t)m->blade[cid].add + delta);
}
void rb_mods_set_blade(RbMods *m, int cid, int v) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->blade[cid].set = saturate_modifier(v);
}

/* ── heart (color 0..7) ── */
int rb_mods_get_heart(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    if (color < 0 || color >= 8) return 0;
    /* Mirrors Rust GameModifiers::get_heart_modifier: Heart00 (index 0, the
       "colorless"/wildcard heart) is added to every color query. */
    int v = rb_modifier_total_entry(&m->heart[cid][color]);
    v += rb_modifier_total_entry(&m->heart[cid][RB_HEART_PINK]);
    return v;
}
void rb_mods_add_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->heart[cid][color].add = saturate_modifier((int64_t)m->heart[cid][color].add + delta);
}

/* ── need_heart ── */
int rb_mods_get_need_heart(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return 0;
    return rb_modifier_total_entry(&m->need_heart[cid][color]);
}
void rb_mods_add_need_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->need_heart[cid][color].add = saturate_modifier((int64_t)m->need_heart[cid][color].add + delta);
}
void rb_mods_set_need_heart(RbMods *m, int cid, int color, int value) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->need_heart[cid][color].set = saturate_modifier(value);
}

/* ── score ── */
int rb_mods_get_score(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return rb_modifier_total_entry(&m->score[cid]);
}
void rb_mods_add_score(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->score[cid].add = saturate_modifier((int64_t)m->score[cid].add + delta);
}
void rb_mods_set_score(RbMods *m, int cid, int value) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->score[cid].set = saturate_modifier(value);
}

/* ── cost ── */
int rb_mods_get_cost(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return rb_modifier_total_entry(&m->cost[cid]);
}
void rb_mods_add_cost(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->cost[cid].add = saturate_modifier((int64_t)m->cost[cid].add + delta);
}
void rb_mods_set_cost(RbMods *m, int cid, int value) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->cost[cid].set = saturate_modifier(value);
}

/* ── orientation ── */
const char *rb_mods_get_orientation(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return NULL;
    if (m->orientation[cid] == 1) return "active";
    if (m->orientation[cid] == 2) return "wait";
    return NULL;
}
void rb_mods_set_orientation(RbMods *m, int cid, const char *s) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || !s) return;
    if (!strcmp(s, "active")) m->orientation[cid] = 1;
    else if (!strcmp(s, "wait")) m->orientation[cid] = 2;
}

/* ── delayed cannot_active ── */
int rb_mods_is_delayed_cannot_active(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return m->delayed_cannot_active[cid] > 0;
}
void rb_mods_add_delayed_cannot_active(RbMods *m, int cid, uint8_t turns) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    uint8_t cur = m->delayed_cannot_active[cid];
    if (turns > cur) m->delayed_cannot_active[cid] = turns;
}
void rb_mods_tick_delayed_for(RbMods *m, const int *owned, int n_owned) {
    /* build a quick set of owned ids for this tick */
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        if (m->delayed_cannot_active[cid] == 0) continue;
        int is_owned = 0;
        for (int i = 0; i < n_owned; i++) if (owned[i] == cid) { is_owned = 1; break; }
        if (!is_owned) continue;
        uint8_t v = m->delayed_cannot_active[cid];
        if (v > 0) v--;
        m->delayed_cannot_active[cid] = v;
    }
}

/* ── set-override getters (mirror get_*_set_modifier / get_cost_modifier_set) ──
   A "set" override is an absolute value that replaces the base value entirely
   (Rust returns Some(set) only when set != 0). The C RbModifierEntry keeps the
   set field separately so the engine can read it directly. */
int rb_mods_get_blade_set(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return m->blade[cid].set;
}
void rb_mods_clear_blade_set(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->blade[cid].set = 0;
}
int rb_mods_get_score_set(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return m->score[cid].set;
}
void rb_mods_clear_score_set(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->score[cid].set = 0;
}
int rb_mods_get_cost_set(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    int v = m->cost[cid].set;
    return v != 0 ? v : 0;   /* Rust get_cost_modifier_set filters set==0 → None */
}
void rb_mods_clear_cost_set(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->cost[cid].set = 0;
}

/* ── remove (mirror remove_*_modifier: saturating subtract of a previously added delta) ── */
void rb_mods_remove_blade(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    if (m->blade[cid].add == 0 && m->blade[cid].set == 0) return;
    m->blade[cid].add = saturate_modifier((int64_t)m->blade[cid].add - delta);
}
void rb_mods_remove_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    if (m->heart[cid][color].add == 0 && m->heart[cid][color].set == 0) return;
    m->heart[cid][color].add = saturate_modifier((int64_t)m->heart[cid][color].add - delta);
}
/* E15 — DELETED. `rb_mods_remove_need_heart` had NO Rust twin and ZERO callers
   tree-wide (measured 2026-10-03: 0 hits in src/, include/ and tests/ for the
   identifier, and 0 hits for `remove_need_heart_modifier` in engine/src).
   game_modifiers.rs defines exactly four `remove_*_modifier` methods —
   remove_blade_modifier :332, remove_heart_modifier :423, remove_score_modifier
   :460, remove_cost_modifier :548. There is no `remove_need_heart_modifier`.

   Rust reverses a need-heart delta by adding its NEGATION, in the one place
   that undoes them — game_state/modifiers.rs:607-610, the global-need-heart
   withdrawal arm of commit_constant_results:

       for (card_id, color_str, delta) in &old_global_nh {
           let hc = crate::card::parse_heart_color(color_str);
           self.mods.add_need_heart_modifier(*card_id, hc, -*delta);
       }

   which is what turn/triggers.c:1303 and :1341 already do via
   rb_mods_add_need_heart(&mods, cid, color, -delta). The function was a fifth
   copy of the same arithmetic under a name Rust does not have, kept alive only
   by a declaration in include/rabuka.h.

   ⛔ BLOCKED-ON: include/rabuka.h:306 — delete the line
       void rb_mods_remove_need_heart(RbMods *m, int card_id, int color, int delta);
   rabuka.h is main's file. Until that line goes the header declares a function
   with no definition, which is a LINK ERROR for any caller rather than a
   warning; nothing calls it, so the tree still builds. */
void rb_mods_remove_score(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    if (m->score[cid].add == 0 && m->score[cid].set == 0) return;
    m->score[cid].add = saturate_modifier((int64_t)m->score[cid].add - delta);
}
void rb_mods_remove_cost(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    if (m->cost[cid].add == 0 && m->cost[cid].set == 0) return;
    /* game_modifiers.rs:548-555 — the subtract is NOT clamped at zero:

           pub fn remove_cost_modifier(&mut self, card_id: i16, delta: i16) {
               if let Some(entry) = self.cost_modifiers.get_mut(&card_id) {
                   entry.additive = entry.additive.saturating_sub(delta);
                   if entry.additive == 0 && entry.set == 0 {
                       self.cost_modifiers.remove(&card_id);
                   }
               }
           }

       and the method's own doc (game_modifiers.rs:535-547) says why, in as many
       words: "The result is deliberately NOT clamped at zero. add_cost_modifier
       does not clamp either, so a negative `additive` is a legitimate, reachable
       state — it is exactly what 「…コストは2減る」 means. Clamping here could
       produce a residue that no add path can build: a +6 increase alongside a -2
       reduction sums to +4, and undoing the +6 must leave the -2 showing, but
       `4 - 6 = -2` clamped to 0 destroyed a reduction the player was entitled to."

       The previous body here DID clamp ("Rust remove_cost_modifier clamps the
       subtract at 0"), which is the second half of that sentence taken as the
       rule. It was the only difference from the other three remove_*_modifier
       twins — blade, score and heart all subtract raw — so the one stat whose
       Rust doc explicitly forbids the clamp was the one that had it. */
    m->cost[cid].add = saturate_modifier((int64_t)m->cost[cid].add - delta);
}

void rb_mods_set_heart_override(RbMods *m, int cid, int color, int count) {
    if (!m || cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->heart_color_override[cid] = (int8_t)color;
    m->heart_override_count[cid] = rb_saturate_u8(count);
}
void rb_mods_remove_heart_override(RbMods *m, int cid) {
    if (!m || cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->heart_color_override[cid] = -1;
    m->heart_override_count[cid] = 0;
}
int rb_mods_get_heart_override(RbMods *m, int cid, int *out_count) {
    if (!m || cid < 0 || cid >= RB_MAX_CARD_IDS) return -1;
    if (m->heart_color_override[cid] < 0) return -1;
    if (out_count) *out_count = m->heart_override_count[cid];
    return m->heart_color_override[cid];
}

/* ── heart_copy (mirror set_heart_copy / get_heart_copy) ── */
void rb_mods_set_heart_copy(RbMods *m, int target_cid, int source_cid) {
    if (target_cid < 0 || target_cid >= RB_MAX_CARD_IDS) return;
    m->heart_copy[target_cid] = (int16_t)source_cid;
}
int rb_mods_get_heart_copy(RbMods *m, int target_cid) {
    if (target_cid < 0 || target_cid >= RB_MAX_CARD_IDS) return -1;
    return m->heart_copy[target_cid];
}

/* ── blade_type (mirror set_blade_type_modifier / clear_blade_type_modifier) ── */
void rb_mods_set_blade_type(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->blade_type[cid] = (int8_t)color;
}
void rb_mods_clear_blade_type(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->blade_type[cid] = -1;
}
int rb_mods_get_blade_type(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return -1;
    return m->blade_type[cid];
}

/* ── heart_color_multiplier (mirror heart_color_multiplier map) ── */
void rb_mods_set_heart_color_multiplier(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->heart_multiplier[cid] = (int8_t)color;
}
int rb_mods_get_heart_color_multiplier(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return -1;
    return m->heart_multiplier[cid];
}

/* ── snapshot trace ring (mirror GameState::ability_applications) ──
   The ring is the C fold of Rust's `GameState::ability_applications`
   (game_state/mod.rs:227, `SmallVec<[AbilityApplication; 4]>`), which every
   `*_with_trace` modifier helper is handed as `&mut` and every
   `record_ability_application` (mod.rs:1621) pushes into. See the note on
   rb_mods_add_blade_with_trace below for the two writers this file still does
   not have. */
int rb_mods_trace_len(const RbMods *m) { return m->n_trace; }

void rb_mods_trace_push(RbMods *m, int source_card_id, const char *ability_text,
                        int effect_type, int target_card_id, int heart_color, int amount) {
    if (m->n_trace >= RB_MODS_TRACE_CAP) {
        /* compact_state behaviour: drop the oldest entry when full. */
        memmove(&m->trace[0], &m->trace[1], (size_t)(RB_MODS_TRACE_CAP - 1) * sizeof(RbAbilityTraceEntry));
        m->n_trace = RB_MODS_TRACE_CAP - 1;
    }
    RbAbilityTraceEntry *e = &m->trace[m->n_trace++];
    e->source_card_id = (int16_t)source_card_id;
    e->target_card_id = (int16_t)target_card_id;
    e->amount = saturate_modifier(amount);
    e->effect_type = (int8_t)effect_type;
    e->heart_color = (int8_t)(heart_color < 0 ? -1 : heart_color);
    memset(e->ability_text, 0, RB_MODS_TRACE_TEXT);
    if (ability_text) {
        int n = 0;
        while (ability_text[n] && n < RB_MODS_TRACE_TEXT - 1) { e->ability_text[n] = ability_text[n]; n++; }
    }
}

/* ⛔ WIRE REQUIRED IN src/ability/effects/misc.c — NOT THIS FILE.
   E13. These two ARE faithful ports and they are the ring's only missing writers.

   ⚠ CORRECTION TO A MEASUREMENT THAT WAS WRONG. The reconciliation recorded
   "blade half wired: rb_mods_add_blade_with_trace def modifiers.c:395, called
   :371". Line 371 is INSIDE THIS COMMENT BLOCK — it is the illustrative
   `grant_blade` body the comment shows the reader how to write, not a call.
   Measured 2026-10-03 over the whole tree:
     rb_mods_add_blade_with_trace — 0 production callers (the only other hits are
       this file's comment at :318 and the definition at :395)
     rb_mods_add_heart_with_trace — 0 production callers; reachable only from
       tests/test_parity_per_card2.c:1631 / :1633, which use it to hand a
       fixture a labelled heart bonus
   So BOTH halves of E13 are unwired, not one.

   Rust call sites (all in ability/effects/resource.rs, all passing
   `&mut gs.ability_applications` as the trace and `gs.activating_card
   .unwrap_or(-1)` — NOT the target — as the source):

     :205  grant_heart_distribution  (resource.rs:192-214)
             gs.mods.add_heart_modifier_with_trace(
                 card_id, color, dist_amount,
                 &mut gs.ability_applications,
                 gs.activating_card.unwrap_or(-1), effect_text);
     :218  grant_blade              (resource.rs:217-225)
             gs.mods.add_blade_modifier_with_trace(
                 card_id, amount, &mut gs.ability_applications,
                 gs.activating_card.unwrap_or(-1), &effect.text);
     :326  BP6 distinct-colour heart per discarded card
             source = gs.activating_card.unwrap_or(-1), amount = 1
     :653  gain_heart per base-heart colour
             source = TARGET card (target_id), NOT activating_card
     :721  gain_heart(All)
             source = card_id, colour = HeartColor::All

   The C counterparts are misc.c:218 `grant_blade` and misc.c:224
   `grant_heart_distribution`, which call the UNTRACED
   rb_mods_add_blade / rb_mods_add_heart. Exact change, in misc.c:

     static void grant_blade(GameState *g, int card_id, int amount) {
         if(card_id < 0) return;
         rb_mods_add_blade_with_trace(&g->mods, card_id, amount,
                                      g->activating_card, effect_text);
     }

   ...and the same swap in grant_heart_distribution's loop. That requires
   THREADING THE ABILITY TEXT down: Rust's grant_blade takes
   `effect: &AbilityEffect` and grant_heart_distribution takes `effect_text:
   &str`, while the C versions take neither. Callers to widen are misc.c:551,
   :558, :562, :571, :792 (grant_blade) and :238 (grant_heart_distribution,
   itself fed by apply_heart_to_card, whose Rust twin
   resource.rs:228 `apply_heart_to_card` likewise carries the effect).

   The two :653 / :721 sites have no C counterpart function at all — they are
   inside gain_heart's per-colour loop, which the C does not have; that is a
   separate gap in misc.c, not a wiring problem here.

Consequence while unwired: `mods.trace` receives score/cost/need-heart
    applications (5 live rb_mods_trace_push sites in game_state_core.c,
    rules.c and score.c) but NO blade or heart application, so any consumer
    filtering on RB_EFFECT_BLADE_BONUS / RB_EFFECT_HEART_BONUS sees nothing.
    rb_mods_trace_len (below) is reachable and correct; it is only the blade and
    heart effect_types that are missing. That does NOT affect
    test_condition_evaluation.c's snapshot_has_score_source, which skips
    everything that is not a score effect_type.

    ⛔ BLOCKED-ON: src/ability/effects/misc.c — widen `grant_blade`
    (misc.c:218) and `grant_heart_distribution` (misc.c:224) to take the ability
    text, then call the two *_with_trace twins. That is F3, and F3 needs the
    five grant_blade call sites (misc.c:551, :558, :562, :571, :792) plus
    misc.c:238 (grant_heart_distribution, fed by apply_heart_to_card) widened.
    misc.c is cluster C's and cluster F's; it is not this file's. */
void rb_mods_add_blade_with_trace(RbMods *m, int cid, int delta,
                                  int source_card_id, const char *ability_text) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    rb_mods_add_blade(m, cid, delta);
    rb_mods_trace_push(m, source_card_id, ability_text, RB_EFFECT_BLADE_BONUS, cid, -1, delta);
}

/* Same story as rb_mods_add_blade_with_trace above: faithful, and reachable
   only from tests (test_parity_per_card2.c:1615/:1617 use it to hand a test
   fixture a heart bonus with an attribution label). Production wiring is the
   misc.c change described above. */
void rb_mods_add_heart_with_trace(RbMods *m, int cid, int color, int delta,
                                  int source_card_id, const char *ability_text) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    rb_mods_add_heart(m, cid, color, delta);
    rb_mods_trace_push(m, source_card_id, ability_text, RB_EFFECT_HEART_BONUS, cid, color, delta);
}

/* ───────────────────────────── total (game_modifiers.rs) ─────────────────────────────
    Mirror ModifierEntry::total (game_modifiers.rs:138-140) — the ONE definition
    of what a modifier entry means:

        #[inline]
        pub fn total(&self) -> i16 { self.set.saturating_add(self.additive) }

    `set` (absolute override) is the base; additive deltas stack on top.

    THE SINGLE LIVE DEFINITION. This function used to be dead: the six
    getters above called the `static inline rb_modifier_total` in
    include/rabuka.h:199 instead, so the C carried two definitions of one Rust
    method and the ported one had zero callers. They now all route here, which
    is Rust's shape (one `total`, called from everywhere). rabuka.h:199 is not
    this file's — core/stats_pipeline.c still calls the inline, and the two are
    arithmetically identical, so this is de-duplication, not a behaviour
    change. Saturating vs wrapping is likewise moot: both fields are i16 and
    both are summed in C `int`, which cannot overflow. */
int rb_modifier_total_entry(const RbModifierEntry *e) {
    if (!e) return 0;
    return (int)e->set + (int)e->add;
}

/* -- record_card_appearance --
   Mirror modifiers.rs:1377-1388 (GameState::record_card_appearance). Rust
   appends to three structures:
     cards_appeared_this_turn -> g->cards_appeared_this_turn[]  (deduped)
     recently_appeared_cards  -> g->recently_appeared[]         (deduped)
     card_appearance_source   -> g->card_appearance_source[]    (NOT deduped)
   The third list is pushed unconditionally and the reader takes the FIRST
   entry for a card, so a source is kept even for a card that already appeared
   this turn; that ordering is what makes the 控え室から登場 gate decidable.
   `source` is an RbZone value; a negative value stands for Rust's empty source
   string and records nothing. */
void rb_record_card_appearance(GameState *g, int card_id, int source) {
    if (!g || card_id < 0) return;
    int found = 0;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++) {
        if (g->cards_appeared_this_turn[i] == card_id) { found = 1; break; }
    }
    if (!found && g->n_cards_appeared_this_turn < 64) {
        g->cards_appeared_this_turn[g->n_cards_appeared_this_turn++] = card_id;
    }
    int in_recent = 0;
    for (int i = 0; i < g->n_recently_appeared; i++) {
        if (g->recently_appeared[i] == card_id) { in_recent = 1; break; }
    }
    if (!in_recent && g->n_recently_appeared < RB_MAX_RECENTLY_MOVED) {
        g->recently_appeared[g->n_recently_appeared++] = card_id;
    }
    if (source >= 0 && g->n_card_appearance_source <
                       (int)(sizeof(g->card_appearance_source_card) /
                             sizeof(g->card_appearance_source_card[0]))) {
        g->card_appearance_source_card[g->n_card_appearance_source] = card_id;
        g->card_appearance_source_zone[g->n_card_appearance_source] = (uint8_t)source;
        g->n_card_appearance_source++;
    }
}

/* -- get_card_appearance_source --
   Mirror modifiers.rs:1394-1399 (GameState::get_card_appearance_source):
   `find(|(k, _)| k == &card_id).map(|(_, v)| v.as_str())` — the FIRST entry
   wins and a card with no entry yields None. */
int rb_get_card_appearance_source(const GameState *g, int card_id, RbZone *out) {
    if (!g || card_id < 0) return 0;
    for (int i = 0; i < g->n_card_appearance_source; i++) {
        if (g->card_appearance_source_card[i] == card_id) {
            if (out) *out = (RbZone)g->card_appearance_source_zone[i];
            return 1;
        }
    }
    return 0;
}

/* RbZone (the movement-side enum) -> RbZoneId (the compact wire enum), so a
   recorded debut zone can be fed to the same source-string resolver the rest
   of the engine uses. */
static RbZoneId appearance_source_zone_id(RbZone z) {
    switch (z) {
        case RB_ZONE_HAND:       return RB_ZONEID_HAND;
        case RB_ZONE_DECK:       return RB_ZONEID_DECK;
        case RB_ZONE_STAGE:      return RB_ZONEID_STAGE;
        case RB_ZONE_DISCARD:    return RB_ZONEID_DISCARD;
        case RB_ZONE_ENERGY:     return RB_ZONEID_ENERGY;
        case RB_ZONE_LIVE:       return RB_ZONEID_LIVE_CARD_ZONE;
        case RB_ZONE_SUCCESS:    return RB_ZONEID_SUCCESS_LIVE_ZONE;
        case RB_ZONE_RESOLUTION: return RB_ZONEID_RESOLUTION;
    }
    return RB_ZONEID_UNKNOWN;
}

/* RbZoneId (the compact wire enum) -> RbZone (the movement-side enum): the
   INVERSE of appearance_source_zone_id, and the only such inverse in the tree.
   Exported because the effect-driven placement sites need it.

   Why it is needed: Rust records the debut source as a WIRE STRING taken
   straight off the effect —
     ability/effects/move/mod.rs:2537-2538
       `let source = self.in_flight.spawn_context.source.as_deref().unwrap_or("")`
   — and hands that string to record_card_appearance, which stores it verbatim
   (modifiers.rs:1638-1641). The C port stores an RbZone instead, so an
   effect-driven placement that only has a zone NAME must be translated here
   before it reaches rb_record_card_appearance.

   Rust's "no source" case is that EMPTY string, guarded by
   `if !source.is_empty()`. The C spells the same thing as a NEGATIVE RbZone
   (rb_record_card_appearance's `source >= 0` guard, modifiers.c:340), so
   every zone id with no RbZone counterpart — and RB_ZONEID_UNKNOWN itself —
   returns -1 here. That is the WHOLE rule now: this function refuses every
   wire zone that RbZone cannot name, instead of folding it onto a neighbour.

   ── Why "refuse" and not "fold" ────────────────────────────────────────────
   The three still-collapsing rows this used to carry were all silent
   MIS-PLACEMENTS, not simplifications, because the fold is lossy in BOTH
   directions once it is read back. rb_get_card_appearance_source hands an
   RbZone to rb_appearance_source_matches, which converts it back to a
   RbZoneId (appearance_source_zone_id, above) and asks
   rb_zone_matches_source — a faithful port of Rust's
   `ZoneId::matches_source` (core/types.rs:914-922), which is EXACT for
   everything except the documented deck / discard widenings. A folded row
   therefore produces a wrong answer for the specific source AND for the
   zone it was folded onto, where Rust's verbatim string comparison
   (condition/card.rs:3000 `get_card_appearance_source(cid) == Some(expected)`)
   produces the opposite answer in each case:

     energy_deck -> RB_ZONE_ENERGY
       the energy DECK is the card POOL (RbPlayer::energy_deck, drawn from by
       rb_energy_deck_draw); the energy ZONE is where energy cards are PLAYED
       (RbPlayer::energy, filled by rb_energy_add_card). Folding the pool
       onto the zone puts an energy-deck card in the wrong bag with no error.
       Read back, a 「登場元がエネルギゾーン」 gate matches a deck debut (Rust:
       no) and a 「登場元がエネルギーデッキ」 gate never can (Rust: yes).
     deck_top / deck_bottom -> RB_ZONE_DECK
       RB_ZONE_DECK is one bag, so the top and the bottom become
       indistinguishable. Read back, 「登場元が山札」 matches a bottom-of-deck
       debut and 「登場元が山札の下」 never does — both inverted from Rust,
       whose matches_source is deliberately asymmetric (a generic `deck`
       matches DECK/DECK_TOP/DECK_BOTTOM, but `deck_top`/`deck_bottom` match
       only themselves).
     deck_top_or_bottom -> RB_ZONE_DECK
       a SELECTOR wire value, not a zone at all.
     success_zone -> RB_ZONE_SUCCESS
       Rust has two distinct variants (enums.rs:102 `Zone::SuccessZone`,
       :104 `Zone::SuccessLiveZone`) and RbZone has one bag for both, so the
       fold makes a success_zone debut answer as success_live_zone.

   engine.c's rb_zone_of_str already applies this same discipline from the
   other direction — it derives its shared rows from rb_zone_id_from_str and
   returns 0 (unresolvable) for RB_ZONEID_DECK_BOTTOM and RB_ZONEID_ENERGY_DECK
   rather than collapsing them (engine.c:137-138). The two tables now agree.

   KEPT, because they are ALIASES and not folds: energy_zone and
   success_live_zone both decode to the SAME RbZoneId as their canonical
   spelling (data.c:918-919's alias table, mirroring Rust
   `ZoneId::from_str`'s `"energy" | "energy_zone"` and
   `"success_live_zone" | "success_live_card_zone"` arms), and waitroom and
   discard are one RbBag on both sides (RbPlayer::discard IS the 控え室;
   `ZoneId::matches_source` folds them in both directions, types.rs:919-920).

   NOTE ON RB_ZONEID_DECK_TOP: engine.c maps it to RB_ZONE_DECK on purpose,
   because that caller's job is the top INSERT (util.rs:2426-2436
   DeckTop inserts at `idx`, default 0 = top) and RB_ZONE_DECK is the
   caller's top insert. This function's job is different — it names the
   zone a card debuted FROM, where the top is a distinguishable answer — so
   it refuses. The same split is why RB_ZONEID_UNDER_MEMBER /
   SAME_AREA / EMPTY_AREA fall to `default: -1` here while engine.c routes
   them to RB_ZONE_STAGE for placement.

   ⛔ ZERO CALLERS, AND IT IS NOT DECLARED IN include/rabuka.h. Both facts are
   real as of this edit: a repo-wide search of engine_c_wip finds the
   identifier only at its own definition below. It is not a phantom — it is
   the ONE honest converter for exactly the call it was written for, and that
   call does not exist yet:

     Rust  ability/effects/move/mod.rs:2582-2583, inside fire_debut_side_effects
             (the "every stage placement goes through here" choke point, :2579):
                 let source = self.in_flight.spawn_context.source.as_deref().unwrap_or("");
                 gs.record_card_appearance(card_id, source);

     C     ability/effects/move.c:2141-2153, rb_move_fire_debut_side_effects —
           the same choke point, called from move.c:2028, :2135, :2213, :2546.
           It DOES NOT call rb_record_card_appearance at all. It hand-rolls
           half of it (:2145-2146, an UNDEDUPED push onto
           g->cards_appeared_this_turn) and then discards the source outright
           with `(void)source;` at :2147. So an effect-driven stage placement
           records NO source at all, and a 「登場元が…」 gate can never be
           answered by one.

   The change, in move.c (NOT this file) — this REPLACES the two hand-rolled
   lines at :2145-2146 and the `(void)source;` at :2147:

       rb_record_card_appearance(g, card_id,
           source && *source
               ? rb_zone_id_to_movement_zone(rb_zone_id_from_str(source))
               : -1);

   `-1` is rb_record_card_appearance's spelling of Rust's empty source string
   (the `source >= 0` guard below), and rb_zone_of_str must NOT be used here:
   it folds DECK_TOP onto RB_ZONE_DECK and UNDER_MEMBER/SAME_AREA onto
   RB_ZONE_STAGE, which is right for a placement destination and wrong for a
   debut source, per the note above. That change also needs the prototype
   added to include/rabuka.h — also not this file's. */
int rb_zone_id_to_movement_zone(RbZoneId zid) {
    switch (zid) {
        case RB_ZONEID_HAND:             return RB_ZONE_HAND;
        case RB_ZONEID_DECK:             return RB_ZONE_DECK;
        case RB_ZONEID_STAGE:            return RB_ZONE_STAGE;
        case RB_ZONEID_DISCARD:
        case RB_ZONEID_WAITROOM:         return RB_ZONE_DISCARD;
        case RB_ZONEID_ENERGY:
        case RB_ZONEID_ENERGY_ZONE:      return RB_ZONE_ENERGY;
        case RB_ZONEID_LIVE_CARD_ZONE:   return RB_ZONE_LIVE;
        case RB_ZONEID_SUCCESS_LIVE_ZONE: return RB_ZONE_SUCCESS;
        case RB_ZONEID_RESOLUTION:       return RB_ZONE_RESOLUTION;
        /* Every zone id below names a bag or a selector that RbZone does not
           have a partner for. See the note above; -1 is the honest failure and
           makes rb_record_card_appearance record no source at all. */
        default:                         return -1;
    }
}

/* -- appearance_source_matches --
   Rust compares the recorded source STRING with the condition's
   `appearance_source` string verbatim
   (condition/card.rs:3000: get_card_appearance_source(cid) == Some(expected)).
   The C port records the RbZone instead, so the comparison goes through
   rb_zone_matches_source — the same resolver the rest of the engine uses to
   turn a wire source string into a zone — which also folds the
   discard/waitroom spellings the Rust wire uses interchangeably. */
int rb_appearance_source_matches(const GameState *g, int card_id, const char *expected_source) {
    RbZone z;
    if (!expected_source || !*expected_source) return 0;
    if (!rb_get_card_appearance_source(g, card_id, &z)) return 0;
    return rb_zone_matches_source(appearance_source_zone_id(z), expected_source);
}

/* -- has_card_appeared_this_turn -- */
int rb_has_card_appeared_this_turn(GameState *g, int card_id) {
    if (!g) return 0;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == card_id) return 1;
    return 0;
}

/* -- clear_card_appearance_tracking --
   Mirror modifiers.rs:1401-1404 (GameState::clear_card_appearance_tracking):
   Rust clears BOTH cards_appeared_this_turn and card_appearance_source, so a
   debut recorded in an earlier turn cannot satisfy a 控え室から登場 gate
   afterwards. `recently_appeared` is deliberately NOT cleared — that matches
   Rust, which leaves recently_appeared_cards alone here. */
void rb_clear_card_appearance_tracking(GameState *g) {
    if (!g) return;
    g->n_cards_appeared_this_turn = 0;
    g->n_card_appearance_source = 0;
}

/* -- record_baton_touch -- */
void rb_record_baton_touch(GameState *g, int pl, int arriving_card_id) {
    if (!g || pl < 0 || pl >= 2) return;
    if (pl == 0) g->baton_touch_count_p1++;
    else g->baton_touch_count_p2++;
    if (arriving_card_id >= 0 &&
        g->n_baton_touch_arriving_card_ids < 16) {
        g->baton_touch_arriving_card_ids[
            g->n_baton_touch_arriving_card_ids++] = arriving_card_id;
    }
}

/* -- get_baton_touch_count -- */
int rb_get_baton_touch_count(const GameState *g, int pl) {
    if (!g || pl < 0 || pl >= 2) return 0;
    return pl == 0 ? g->baton_touch_count_p1 : g->baton_touch_count_p2;
}

/* -- clear_baton_touch_tracking --
   Mirror modifiers.rs:1425-1433 (GameState::clear_baton_touch_tracking): the
   FULL turn-scoped reset. Reached from rb_reset_keyword_tracking
   (tracking.rs:28-34) on entry to each player's Active phase. The narrower
   PLAY-scoped half is rb_clear_play_scoped_baton_touch (modifiers.rs:1442-1447,
   called at phases.rs:1154) and deliberately does NOT drop the counts or the
   arriving-id list. */
void rb_clear_baton_touch_tracking(GameState *g) {
    if (!g) return;
    g->baton_touch_count_p1 = 0;
    g->baton_touch_count_p2 = 0;
    g->n_baton_touch_arriving_card_ids = 0;
    g->baton_touch_zero_cost = 0;
    g->baton_touch_replaced_member_cost = -1;
    g->baton_touch_replaced_member_id = -1;
    g->baton_touch_arriving_card_id = -1;
}

/* -- record_card_movement -- */
void rb_record_card_movement(GameState *g, int card_id, int from_zone, int to_zone, int causer, int target) {
    if (!g || card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;
    g->moved_this_turn[card_id] = 1;
    /* Rust builds ONE `MovementEvent` and clones it into batch_movements,
       turn_movements and (for an area move) turn_area_movements
       (modifiers.rs:1918-1956). One local, three readers — so the area-move
       arm below cannot be handed a stale tail entry when batch_movements is
       already at its 16-slot cap. */
    RbBatchMovement event;
    event.moved_card_id = card_id;
    event.source_zone = from_zone;
    event.dest_zone = to_zone;
    event.cause_player_id = causer;
    /* E16 WRITER. `cause_card_id` (rabuka.h:1440) had ZERO writers tree-wide, so
       the struct above — a stack local — carried WHATEVER WAS IN THAT STACK SLOT
       into g->batch_movements and g->turn_area_movements. That is an
       uninitialised read, not a missing feature, and it is the reason
       `eval_group`'s temporal this_turn arm could not be ported: Rust
       (condition/card.rs:1646-1654) tests
           m.cause_card_id.is_some_and(|cause_cid| groups.iter().any(|g|
               card_matches_group_str(card_db, cause_cid, Some(g))))
       on the turn_area_movements entry, and the C had no value to test.

       The value is `self.activating_card`, exactly as Rust's two push sites pass
       it: modifiers.rs:1843/1851 (record_stage_position_swap) and modifiers.rs
       :1943 (push_movement_event's own cause_card_id argument, which
       record_card_movement is the C fold of). C convention for "no activating
       card" is a negative value (8 call sites test `activating_card >= 0`).

       NOT a regression risk: the field had no reader either, so nothing could
       observe the old garbage. */
    event.cause_card_id = g->activating_card;
    event.effect_only = target;
    if (g->n_batch_movements < (int)(sizeof(g->batch_movements) / sizeof(g->batch_movements[0])))
        g->batch_movements[g->n_batch_movements++] = event;
    if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) {
        g->recently_moved[g->n_recently_moved++] = card_id;
    } else {
        memmove(g->recently_moved, g->recently_moved + 1,
                (RB_MAX_RECENTLY_MOVED - 1) * sizeof(g->recently_moved[0]));
        g->recently_moved[RB_MAX_RECENTLY_MOVED - 1] = card_id;
    }
    /* modifiers.rs:1934-1972 — the stage->stage (area) move arm, which the C
       fold used to drop on the floor, so the turn-level position flag and the
       opponent-cause watcher were only ever set by whichever call site
       remembered to do so by hand. Rust order: recently_moved is appended
       first, then the area-move arm runs, which is why the hook sits here and
       not at the top of the function.

       Rust guards the hook on `caused_by_opponent` (the moved card's owner
       differs from event.cause_player_id); that same test lives INSIDE the
       hook (game_state_abilities.c:1592-1593, abilities.rs:1054-1063, owner ==
       causer_player returns silently), so calling it unconditionally is
       behaviourally identical. */
    if (from_zone == RB_ZONEID_STAGE && to_zone == RB_ZONEID_STAGE) {
        g->position_change_occurred_this_turn = 1;
        /* modifiers.rs:1956 — `self.turn_area_movements.push(event.clone())`,
           the turn-level AREA-ONLY ordered log, immediately before the
           position flag at :1957 and the watcher hook at :1966-1971. The field
           now exists (rabuka.h:1436) and this is its ONLY writer; before this
           line it was declared and never written, which is also why the
           clear in rb_clear_card_movement_tracking was vacuous.

           The event is the same one already appended to batch_movements at the
           top of this function — Rust clones the same `event` into
           batch_movements, turn_movements and turn_area_movements — so the two
           C entries are built from identical fields.

           BOUNDED, unlike Rust's growable SmallVec: the C slot is
           RB_MAX_RECENTLY_MOVED (8). On overflow the OLDEST is dropped, the
           same sliding window g->recently_moved uses six lines above, because
           the only reader shape Rust supports here is
           `turn_area_movements.iter().rev().find(..)`
           (hooked_by_foreign_cause, abilities.rs:506-545) — a scan BACKWARD
           from the newest — so the newest 8 are the ones that answer.

           The reader itself is not this file's: game_state_abilities.c:2258
           still scans g->batch_movements for the same predicate. Swapping it
           to g->turn_area_movements is the exact Rust lookup and is noted in
           that file's comment. Behaviour is identical either way (batch_movements
           is a superset, filtered to stage->stage by the reader), so writing
           this field changes nothing observable today. */
        if (g->n_turn_area_movements < RB_MAX_RECENTLY_MOVED) {
            g->turn_area_movements[g->n_turn_area_movements++] = event;
        } else {
            memmove(g->turn_area_movements, g->turn_area_movements + 1,
                    (RB_MAX_RECENTLY_MOVED - 1) * sizeof(g->turn_area_movements[0]));
            g->turn_area_movements[RB_MAX_RECENTLY_MOVED - 1] = event;
        }
        rb_fire_opponent_cause_watchers_for_move(g, card_id, causer);
    }
}

/* -- record_cost_paid -- STILL NOT WRITTEN HERE, but the BLOCKER IS GONE.
   The port is game_state/mod.rs:404-414 (GameState::record_cost_paid), the ONLY
   writer of cards_paid_cost_this_turn:

       if let Some(cid) = self.ability_queue.current_entry().and_then(|e| e.card_id) {
           if !self.cards_paid_cost_this_turn.contains(&cid) {
               self.cards_paid_cost_this_turn.push(cid);
           }
       }

   A SET, not a tally: the clause it feeds is 「このメンバーが、自分のカードの
   能力のコストを支払うため」 (PL!-pb2-021-N 南ことり), which means "paid at
   least once this turn", so two payments by the same member must still read
   as one card. Mod.rs:397-398 states that explicitly. The 5 Rust call sites
   are cost/handlers.rs:1389,:1477 and ability/choice.rs:734,:763,:948.

   ⛔ THE OLD "BLOCKED" NOTE HERE WAS STALE AND IS NOW DEAD. It claimed the two
   members sat inside RbLiveSnapshot and that `g->cards_paid_cost_this_turn`
   "does not compile". Both were true when written and both are false now: a
   peer has since RELOCATED them onto GameState itself (rabuka.h:1418-1419,
   with the move documented at :1406-1417), so the field compiles and the
   clear in rb_clear_card_movement_tracking below is now LANDED.

   What is still missing is the WRITER and the READER, and both are outside
   this file:
     · writer — all 5 Rust call sites are in ability/cost/handlers.rs and
       ability/choice.rs, neither of which is this file's;
     · reader — ability/condition/card/comparison.rs:60-70 has no C branch at
       all, so condition.c never tests the paid_cost_this_turn flag and the
       clause falls through to the numeric path (comparison.rs:50-55 names the
       symptom: there was nothing to compare against, so it read as 'cost >= 0'
       and was permanently satisfied). condition.c is not this file's either.

   The body, verbatim, for whoever owns those call sites (the two-step
   current_entry read is rb_gsc_current_ability_source_card_id's,
   game_state_core.c:491-496):

       void rb_record_cost_paid(GameState *g) {
           if (!g) return;
           int cur = rb_queue_current_entry(g);
           if (cur < 0 || cur >= g->queue.n_entries) return;
           int cid = g->queue.entries[cur].card_id;
           if (cid < 0) return;
           for (int i = 0; i < g->n_cards_paid_cost_this_turn; i++)
               if (g->cards_paid_cost_this_turn[i] == cid) return;
           if (g->n_cards_paid_cost_this_turn < (int)(sizeof(g->cards_paid_cost_this_turn)
                                                     / sizeof(g->cards_paid_cost_this_turn[0])))
               g->cards_paid_cost_this_turn[g->n_cards_paid_cost_this_turn++] = cid;
       }

   It is left as a note rather than compiled because a definition with no
   caller and no prototype in include/rabuka.h (also not this file's) is
   exactly the "exists by name is not parity" shape PROGRESS.md warns about;
   the one caller is the thing that has to be written first. */

/* -- clear_card_movement_tracking --
   Mirror game_state/modifiers.rs:1994-2019
   (GameState::clear_card_movement_tracking). The old cite here said
   1920-1942, which is push_movement_event, a different function.

   Rust clears EIGHT turn-scoped structures. Five were already here; THREE
   LANDED with this edit, because their fields now exist; ONE still has no
   field.

     :1995 cards_moved_this_turn   -> g->moved_this_turn[]        HELD
     :1996 turn_movements          -> g->batch_movements[]        HELD (the C
                                      folds push_movement_event into
                                      rb_record_card_movement, so the event
                                      log and the per-card flag share one
                                      entry point)
     :1997 cards_appeared_this_turn -> g->cards_appeared_this_turn[] HELD
     :2001 cards_activated_this_turn -> STILL NO C FIELD. Requested shape is
                                      in the handover report; nothing writes it
                                      either, so this is a whole missing
                                      feature, not a missing clear line.
     :2004 cards_paid_cost_this_turn -> LANDED. The field was RELOCATED onto
                                      GameState (rabuka.h:1418-1419) by a peer,
                                      so the premise of the old note here — that
                                      the two members were stranded inside
                                      RbLiveSnapshot and `g->cards_paid_cost_
                                      this_turn` did not compile — is dead.
                                      The WRITER is still missing (all 5 Rust
                                      call sites are in ability/cost/handlers.rs
                                      and ability/choice.rs); see the
                                      record_cost_paid note above.
     :2014 card_appearance_source  -> g->card_appearance_source[]   HELD
     :2015 turn_area_movements     -> LANDED, field at rabuka.h:1436-1437,
                                      and this edit also added its only
                                      WRITER (rb_record_card_movement's
                                      stage->stage arm, Rust modifiers.rs:1956).
     :2018 mods.opp_cause_fired_keys -> LANDED, field at rabuka.h:270-271
                                      (`opp_cause_fired_keys[16]` +
                                      `n_opp_cause_fired_keys`, documented
                                      there as a per-MOVE scope that
                                      g->batch_triggered_keys cannot
                                      express). HONEST CAVEAT: the clear is
                                      correct but INERT today, because the
                                      live dedupe set is not this field — it
                                      is a file-static side table
                                      `s_opp_cause_fired[]` in
                                      game_state_abilities.c:2081-2114, which
                                      reproduces the same turn-scoped lifetime
                                      by keying every entry on
                                      (GameState*, g->turn). The 16-slot
                                      field is still unwritten. Retiring the
                                      side table in favour of the field is
                                      game_state_abilities.c's change, not
                                      this file's.

   NOT cleared, matching Rust: g->recently_moved / recently_appeared.
   Call site in Rust: engine/src/turn/phases.rs:309, inside the victory /
   turn-rollover arm of advance_phase, immediately after turn_number += 1. */
void rb_clear_card_movement_tracking(GameState *g) {
    if (!g) return;
    memset(g->moved_this_turn, 0, sizeof(g->moved_this_turn));
    g->n_batch_movements = 0;
    g->n_cards_appeared_this_turn = 0;
    /* modifiers.rs:2014 — `self.card_appearance_source.clear();`

       card_appearance_source is the SECOND HALF of the 「このターン中に登場した場合」
       condition and it MUST turn over with cards_appeared_this_turn. The
       condition reads the pair together (ability/condition/card.rs compares
       has_card_appeared_this_turn against get_card_appearance_source), and
       rb_get_card_appearance_source returns the FIRST matching entry — so
       without this, a source recorded on turn N stays answerable on turn
       N+1, N+2, ... forever, and a genuinely different later debut could never
       match because the stale turn-N entry is the one found. This is Rust's
       own reasoning, verbatim, at the clear site. */
    g->n_card_appearance_source = 0;
    /* modifiers.rs:2024-2027 — the three landed clears. Rust's own comments
       say why each is turn-scoped rather than cosmetic:
         :2024 `self.cards_activated_this_turn.clear();`
              「このターン中に…」 is a per-turn history; left uncleared, an
              activation from a previous turn keeps satisfying PL!-pb2-010-P＋
              and PL!-pb2-040-L forever. (NO C FIELD — see the table above.)
         :2027 `self.cards_paid_cost_this_turn.clear();`
              Same turnover for PL!-pb2-021-N: 「コストを支払う」 is a per-turn
              event, and a cost paid on a previous turn must not keep firing
              her 自動.
         :2038 `self.turn_area_movements.clear();`
         :2041 `self.mods.opp_cause_fired_keys.clear();`
              Opponent-cause watcher dedupe is turn-scoped: a given move can
              arm a marked watcher once, and the set resets with the movement
              data. */
    g->n_cards_paid_cost_this_turn = 0;
    g->n_turn_area_movements = 0;
    g->mods.n_opp_cause_fired_keys = 0;
}

/* -- remove_revealed_card --
   Mirrors modifiers.rs:1606-1613 — remove the FIRST matching entry only.
   The previous body stripped every occurrence, so a card id revealed twice
   (two copies of the same template in the yell) lost both entries instead of
   one. */
void rb_remove_revealed_card(GameState *g, int card_id) {
    if (!g) return;
    for (int i = 0; i < g->n_revealed; i++) {
        if (g->revealed_cards[i] != card_id) continue;
        for (int j = i; j + 1 < g->n_revealed; j++) g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        return;
    }
}

/* -- record_stage_position_swap -- E9
   Mirror modifiers.rs:1799-1857 (GameState::record_stage_position_swap). This is
   the SINGLE choke point Rust's three position-change branches each wrote out in
   full (effects/position.rs:1022 / :1061 / :1102 — an explicit `source_position`,
   a named `target_member`, and the `this_member` case), and the doc comment at
   modifiers.rs:1780-1798 says the sequencing is load-bearing.

   ⛔ NOT A DISPLAY/REPLAY GAP. A peer measured E9 that way; that is wrong and
   the readers are in the GAMEPLAY condition evaluator, not a UI:

     engine/src/ability/condition/state.rs:871   passes
             &self.game_state.position_change_events
     engine/src/ability/condition/state.rs:879   ordering comment (batch-scoped)
     engine/src/ability/condition/state.rs:929   source list for a state condition
     engine/src/ability/condition/state.rs:938   .position_change_events
     engine/src/ability/condition/state.rs:1162  .position_change_events

   game/display.rs:1858/1861 is the only DISPLAY reader, and it is a fifth.
   So every 「このターン、このメンバーがエリアを移動した」 / position-change state
   condition in the corpus reads an EMPTY vector in the C.

   In the C, g->position_change_events[] (rabuka.h:1534) had ZERO writers — its
   only mention was the reset at core/game_state_abilities.c:589 — so this
   function is the missing writer.

   The three steps, in Rust's order:
     1. the two PositionChangeEvents, BEFORE push_movement_event, because that
        call consumes the recently-moved snapshot and arms the opponent-caused
        watcher trigger (modifiers.rs:1788-1790);
     2. the source is recorded unconditionally, the target only when it is a real
        card (modifiers.rs:1791-1793) — the source slot is checked non-empty by
        the caller while the target slot may legitimately be empty;
     3. push_movement_event for the source, and again for the target when real. */
static void mods_push_movement_event(GameState *g, int moved_card_id,
                                      int src_zone, int dst_zone, int effect_only) {
    if (!g || moved_card_id < 0 || moved_card_id >= RB_MAX_CARD_IDS) return;
    /* Rust's mover_pid is `ability_queue.current_entry().map(|e| e.player_id)`
       (modifiers.rs:1812-1816). rb_record_card_movement's `causer` IS that seat
       — the C's fold of push_movement_event — so the position change inherits the
       same seat the two Rust push sites pass as `mover_pid`. */
    rb_record_card_movement(g, moved_card_id, src_zone, dst_zone,
                            rb_queue_current_entry(g), effect_only);
}

void rb_record_stage_position_swap(GameState *g, int source_id, int source_index,
                                   int target_id, int target_index) {
    if (!g) return;
    /* Rust does NOT set activating_card here — it READS it (modifiers.rs:1823 /
       :1834 / :1843 / :1851) and takes the mover seat from the queue's current
       entry (modifiers.rs:1812-1816). The caller publishes activating_card, the
       same way rb_recalc_constants publishes it around its own per-card apply
       (triggers.c:1380-1381). */

    /* 1. the two events (modifiers.rs:1817-1838). effect_only is TRUE on both:
       modifiers.rs:1825 / :1835 pass `effect_only: true` literally. */
    int n_events = g->n_position_change_events;
    int cap_events = (int)(sizeof(g->position_change_events) /
                           sizeof(g->position_change_events[0]));
    if (source_id >= 0) {
        if (n_events < cap_events) {
            RbPositionChangeEvent *ev = &g->position_change_events[n_events++];
            ev->card_id   = source_id;
            ev->position  = target_index;
        }
    }
    if (target_id >= 0) {
        if (n_events < cap_events) {
            RbPositionChangeEvent *ev = &g->position_change_events[n_events++];
            ev->card_id   = target_id;
            ev->position  = source_index;
        }
    }
    g->n_position_change_events = n_events;

    /* 2. + 3. the movement events (modifiers.rs:1839-1856). Both are
       stage -> stage with effect_only = true (modifiers.rs:1845 / :1853). */
    mods_push_movement_event(g, source_id, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 1);
    if (target_id >= 0)
        mods_push_movement_event(g, target_id, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 1);
}

/* -- count_as_multiple_weight -- E10
   Mirror modifiers.rs:227-301 (GameState::count_as_multiple_weight). Returns how
   many cards `counted` is worth: 1 normally, `count_as` (>= 1) when the card
   carries a 常時 `count_as_multiple` whose own condition holds and whose
   `counted_by` list accepts the counting card.

   REACHABLE, MEASURED: exactly ONE `count_as_multiple` effect node in the 985
   abilities (count_as=2, counted_by="lily white", self_target=true,
   duration=as_long_as, plus one condition), on a member card — PL!-pb2-041-L
   春情ロマンティック. Census tool tools/census_e.c, over 2116 card-ability pairs /
   4925 effect nodes. Without this function that card is counted as 1 by
   card.rs:2124 / modifiers.rs:341.

   The `counting_card` resolution order (modifiers.rs:232-241) is: the explicit
   argument, else the queue's current entry card, else g->activating_card, and a
   `-1` anywhere in that chain falls through to the next; no card at all means 1. */
static int mods_effect_count_as(const AbilityEffect *e) {
    if (!e) return 0;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "count_as")) return atoi(e->extra_v[i]);
    return 0;
}

/* -- E10/E11/E12 shared helpers --------------------------------------------
   `card_database.get_card(id)` returns None for an id that is not a key, which
   is what both E10 (modifiers.rs:245-248) and E12 (modifiers.rs:361-363) test.
   In the C the card index IS the card id, so "is a key" is "does it decode". */
static int mods_card_exists(int card_id) {
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 0;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    rb_free_card(&c);
    return 1;
}

/* GameState::owner_of_card (modifiers.rs:1663-1680), used by E10's condition
   evaluation (modifiers.rs:267-268) as the `self` player of the rule's own
   condition. Rust checks SIX zones per seat, in this order, and returns the
   first seat that holds it — so a card id present in both seats resolves to
   seat 0, which the C must reproduce rather than "fix". */
static int mods_owner_of_card(const GameState *g, int card_id) {
    if (!g) return -1;
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == card_id) return pl;
        for (int i = 0; i < P->hand.n; i++)    if (P->hand.cards[i] == card_id) return pl;
        for (int i = 0; i < P->live.n; i++)    if (P->live.cards[i] == card_id) return pl;
        for (int i = 0; i < P->energy.n; i++)  if (P->energy.cards[i] == card_id) return pl;
        for (int i = 0; i < P->success.n; i++) if (P->success.cards[i] == card_id) return pl;
        for (int i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == card_id) return pl;
    }
    return -1;
}

unsigned rb_mods_count_as_multiple_weight(const GameState *g, int counted, int counting_card) {
    if (!g || counted < 0) return 1;
    /* modifiers.rs:232-244. */
    if (counting_card <= 0) {
        counting_card = -1;
        int cur = rb_queue_current_entry(g);
        if (cur >= 0 && cur < g->queue.n_entries) {
            int cid = g->queue.entries[cur].card_id;
            if (cid > 0) counting_card = cid;
        }
    }
    if (counting_card <= 0 && g->activating_card > 0) counting_card = g->activating_card;
    if (counting_card <= 0) return 1;               /* modifiers.rs:242-244 */
    if (!mods_card_exists(counted)) return 1;       /* modifiers.rs:245-248 */

    int n = rb_card_num_abilities((uint32_t)counted);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)counted, ai, &ab)) continue;
        /* modifiers.rs:249-255 — the effect must live on a 常時 ability. */
        int constant = ab.triggers && strstr(ab.triggers, "\xe5\xb8\xb8\xe6\x99\x82") != NULL;
        const AbilityEffect *e = constant ? ab.effect : NULL;
        if (!e || !e->action || strcmp(e->action, "count_as_multiple")) {
            rb_free_ability(&ab);
            continue;
        }
        /* modifiers.rs:265-276 — the rule's OWN condition, evaluated with the
           COUNTED card's owner as `self`. Evaluated through the engine's own
           entry point, which is the twin of card.rs's ConditionContext. */
        int cond_met = !e->has_condition || !e->condition ||
                       rb_eval_condition_for_host(g, mods_owner_of_card(g, counted),
                                                  counted, e->condition);
        if (!cond_met) { rb_free_ability(&ab); continue; }

        /* modifiers.rs:282-297 — `counted_by` is the list of groups whose effects
           SEE the weight; an empty list means any effect sees it. The extra
           arrives comma-joined, so it is split per token exactly as the corpus
           stores it. */
        const char *seen_by = NULL;
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "counted_by")) seen_by = e->extra_v[i];
        if (seen_by && *seen_by) {
            const char *p = seen_by;
            int ok = 0;
            for (;;) {
                const char *comma = strchr(p, ',');
                size_t len = comma ? (size_t)(comma - p) : strlen(p);
                if (len > 0 && len < 128) {
                    char tok[128];
                    memcpy(tok, p, len); tok[len] = '\0';
                    if (rb_card_matches_group_str(counting_card, tok)) { ok = 1; break; }
                }
                if (!comma) break;
                p = comma + 1;
            }
            if (!ok) { rb_free_ability(&ab); continue; }
        }
        /* modifiers.rs:298 — `count_as_any().unwrap_or(2).max(1)`. */
        int weight = mods_effect_count_as(e);
        if (weight <= 0) weight = 2;
        rb_free_ability(&ab);
        return (unsigned)weight;
    }
    return 1;                                        /* modifiers.rs:300 */
}

/* -- weighted_zone_count -- E11
   Mirror modifiers.rs:308-343 (GameState::weighted_zone_count): count a zone's
   cards, honouring each card's count_as_multiple weight, filtered by
   card_type and group_names exactly as the UNWEIGHTED path does.

   REACHABLE, MEASURED: the single corpus `count_as_multiple` node above is the
   whole of it — the weight is only ever consulted from this one card, so
   landing E11 without E10 would have been dead code in one direction and a
   silent wrong count in the other.

   ⚠ AN EMPTY card_type MEANS "any" (modifiers.rs:317-318). Passing Some("")
   through the matcher matches nothing and counts zero cards, which is the
   comment's exact warning; the C's rb_card_matches_type returns 1 for an
   unrecognised filter, so the empty string is filtered out HERE rather than
   handed over. */
unsigned rb_mods_weighted_zone_count(const GameState *g, const int *cards, int n_cards,
                                     const char *card_type, const char *group_names,
                                     int counting_card) {
    if (!g || !cards || n_cards <= 0) return 0;
    const char *ct = (card_type && *card_type) ? card_type : NULL;
    char grp[256];
    const char *groups = NULL;
    if (group_names && *group_names) {
        size_t len = strlen(group_names);
        if (len < sizeof(grp)) { memcpy(grp, group_names, len + 1); groups = grp; }
    }
    unsigned total = 0;
    for (int i = 0; i < n_cards; i++) {
        int cid = cards[i];
        if (cid == RB_EMPTY_SLOT) continue;          /* modifiers.rs:323-324 */
        if (ct && !rb_card_matches_type(cid, ct)) continue;
        if (groups && *groups) {
            /* modifiers.rs:330-339 — `.any()` over the WHOLE comma-joined list,
               and an empty list means no group restriction at all. */
            const char *p = groups;
            int ok = 0;
            for (;;) {
                const char *comma = strchr(p, ',');
                size_t len = comma ? (size_t)(comma - p) : strlen(p);
                if (len > 0 && len < 128) {
                    char tok[128];
                    memcpy(tok, p, len); tok[len] = '\0';
                    if (rb_card_matches_group_str(cid, tok)) { ok = 1; break; }
                }
                if (!comma) break;
                p = comma + 1;
            }
            if (!ok) continue;
        }
        total += rb_mods_count_as_multiple_weight(g, cid, counting_card);
    }
    return total;
}

/* -- replacement_appear_wait_applies -- E12
   Mirror modifiers.rs:360-451 (GameState::replacement_appear_wait_applies):
   does an on-stage 常時 `replacement_appear_wait` rule hold `incoming` in WAIT as
   it appears in area `area_idx`?

   REACHABLE, MEASURED: TWO such effect nodes in the 985 abilities, both with
   position=center, heart_total_limit=4, heart_total_operator="<=",
   card_type=member_card, original_value=true, replacement=appear_wait
   (PL!-pb2-002-PP 絢瀬絵里 and its twin). Rust calls it from
   turn/phases.rs:1281 at the moment of placement.

   Every filter below is a Rust `continue`, so the function returns on the FIRST
   rule that survives all of them — which is why the loop order (both seats, then
   the three stage slots) is the corpus order and not a set. */
static int mods_printed_base_heart_total(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int total = 0;
    /* modifiers.rs:367-371 — `card.base_heart.hearts.values().fold(0, saturating_add)`,
       i.e. the PRINTED base-heart slice only. rb_card_total_hearts is the WRONG
       total here (it falls back to need_heart). */
    for (int i = 0; i < c.num_base && i < c.n_hearts; i++) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

int rb_mods_replacement_appear_wait_applies(const GameState *g, int incoming, int area_idx) {
    if (!g || incoming < 0 || !mods_card_exists(incoming)) return 0;
    if (!rb_card_is_member(incoming)) return 0;      /* modifiers.rs:364-366 */
    int printed = mods_printed_base_heart_total(incoming);

    for (int pl = 0; pl < 2; pl++) {
        for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
            int stage_id = g->p[pl].stage[slot];
            if (stage_id == RB_EMPTY_SLOT) continue;
            /* modifiers.rs:386-388 — a member is never in front of itself. */
            if (stage_id == incoming) continue;
            if (!mods_card_exists(stage_id)) continue;
            int n = rb_card_num_abilities((uint32_t)stage_id);
            for (int ai = 0; ai < n; ai++) {
                Ability ab;
                memset(&ab, 0, sizeof(ab));
                if (!rb_decode_card_ability((uint32_t)stage_id, ai, &ab)) continue;
                int constant = ab.triggers && strstr(ab.triggers, "\xe5\xb8\xb8\xe6\x99\x82") != NULL;
                const AbilityEffect *e = constant ? ab.effect : NULL;
                if (!e || !e->action ||
                    strcmp(e->action, "replacement_appear_wait")) {
                    rb_free_ability(&ab);
                    continue;
                }
                /* modifiers.rs:409-412 — a waited HOST's rule does not apply. */
                const char *orient = rb_mods_get_orientation((RbMods *)&g->mods, stage_id);
                if (orient && !strcmp(orient, "wait")) { rb_free_ability(&ab); continue; }
                /* modifiers.rs:413-426 — the rule governs the AREA the incoming
                   member is entering, so it is compared against `area_idx` and
                   NOT against the host's own slot. */
                const char *pos = NULL;
                for (int i = 0; i < e->n_extra; i++)
                    if (e->extra_k[i] && !strcmp(e->extra_k[i], "position")) pos = e->extra_v[i];
                if (pos) {
                    int want = rb_stage_position_index(pos);
                    if (want < 0) { rb_free_ability(&ab); continue; }
                    if (want != area_idx) { rb_free_ability(&ab); continue; }
                }
                /* modifiers.rs:427-431 — card_type must be Member when present. */
                const char *ect = NULL;
                for (int i = 0; i < e->n_extra; i++)
                    if (e->extra_k[i] && !strcmp(e->extra_k[i], "card_type")) ect = e->extra_v[i];
                if (ect && strcmp(ect, "member_card")) { rb_free_ability(&ab); continue; }

                const char *limit_s = NULL, *op_s = NULL, *blade_s = NULL;
                for (int i = 0; i < e->n_extra; i++) {
                    if (!e->extra_k[i]) continue;
                    if (!strcmp(e->extra_k[i], "heart_total_limit"))    limit_s = e->extra_v[i];
                    if (!strcmp(e->extra_k[i], "heart_total_operator")) op_s    = e->extra_v[i];
                    if (!strcmp(e->extra_k[i], "blade_limit"))          blade_s = e->extra_v[i];
                }
                if (limit_s && *limit_s) {
                    /* modifiers.rs:432-441 — default operator "<=", NOT the
                       compare_counts default (util.rs:1907 `.or(Some("<="))`). */
                    if (!op_s) op_s = "<=";
                    if (!rb_compare_counts(op_s, printed, atoi(limit_s))) {
                        rb_free_ability(&ab);
                        continue;
                    }
                } else if (blade_s) {
                    /* modifiers.rs:442-445 — a blade-bounded variant is not
                       modelled in Rust, and guessing one would be worse than
                       declining, so this is the same decline. */
                    rb_free_ability(&ab);
                    continue;
                }
                rb_free_ability(&ab);
                return 1;                            /* modifiers.rs:446 */
            }
        }
    }
    return 0;                                        /* modifiers.rs:450 */
}

/* -- clear_revealed_cards --
   Mirror modifiers.rs:1620-1623 (GameState::clear_revealed_cards).
   Rust clears revealed_cards AND revealed_card_meta; RbMods/GameState has no
   revealed_card_meta field (see field-addition request in the report).
   Call site in Rust: engine/src/turn/phases.rs:278, the first statement of the
   victory-determination arm of advance_phase — the same arm that calls
   rb_clear_card_movement_tracking at :310. */
void rb_clear_revealed_cards(GameState *g) {
    if (!g) return;
    g->n_revealed = 0;
}

/* ══════════════════════ constant cost modifiers ══════════════════════
   Port of game_state/modifiers.rs::recalculate_constant_cost_modifiers
   (engine/src/core/game_state/modifiers.rs:1433-1437, the public wrapper that
   collects the stage + hand constant ids) and of its one real body,
   recalculate_constant_cost_modifiers_with_ids (:1439-1654).

   Every 常時 ModifyCost effect on a stage or hand card is re-evaluated into
   two expected maps and then DIFFED against what was committed last time:
     · additive  (operation "add" / "subtract")  -> mods.cost[cid].add
     · absolute  (operation "set")               -> mods.cost[cid].set
   The previous body ("Simplified: reset and re-apply") re-applied
   mods.constant_cost[cid] through rb_mods_set_cost, which (a) turned every
   additive delta into an absolute set-override and (b) never dropped a bonus
   whose condition had stopped passing — stale costs accumulated forever.

   ONE CALL SITE, and that is the whole design. Rust calls this from exactly
   one place — the tail of recalculate_constants, at :1392, AFTER
   commit_constant_results (:1375) and BEFORE
   evaluate_success_zone_constant_modifiers (:1398) — and the C mirrors that
   placement exactly, at the tail of rb_recalc_constants
   (turn/triggers.c:1533). Two Rust facts make a second call site wrong
   rather than merely redundant:
     · commit_constant_results takes NO cost argument. Its parameters are
       exp_blade / exp_score / exp_heart / exp_prohibition /
       exp_global_need_heart / the three source lists / the two per-seat
       score bonuses (game_state/modifiers.rs:518-533, called at :1375-1386)
       — cost is not among them, so the main constant scan has no way to
       commit one.
     · recalculate_constants' `match effect.action` has no ModifyCost arm. It
       covers GainResource (:792), ModifyScore (:1028), Restriction (:1060),
       GainAbility (:1118), GainAbilityFromSource (:1202),
       ModifyRequiredHeartsGlobal (:1209) and Sequential (:1250) and nothing
       else — the only `ActionType::ModifyCost` test in this whole region is
       :1472, INSIDE recalculate_constant_cost_modifiers_with_ids itself — so
       no per-printed-effect cost arm exists to remove. Cost is owned
       end-to-end by the one diff-and-commit pass below. */

/* Typed extra getters (mirror AbilityEffect::{operation,value,location,…}_any) */
static const char *cost_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int cost_extra_int(const AbilityEffect *e, const char *k, int fallback) {
    const char *v = cost_extra(e, k);
    if (!v || !*v) return fallback;
    return atoi(v);
}
static int cost_extra_flag(const AbilityEffect *e, const char *k) {
    const char *v = cost_extra(e, k);
    return v && !strcmp(v, "true");
}
static int csv_len(const char *csv) {
    if (!csv || !*csv) return 0;
    int n = 1;
    for (const char *p = csv; *p; p++) if (*p == ',') n++;
    return n;
}

/* LL-bp7-001's 「手札3枚捨てて10」 is a pre-play choice, not a passive constant
   (modifiers.rs:1159-1169): set 10 + location hand + 3 characters + optional. */
static int cost_is_ll_bp7_play_cost(const AbilityEffect *e) {
    const char *op = cost_extra(e, "operation");
    if (!op || strcmp(op, "set") != 0) return 0;
    if (cost_extra_int(e, "value", 0) != 10) return 0;
    const char *loc = cost_extra(e, "location");
    if (!loc || strcmp(loc, "hand") != 0) return 0;
    if (!cost_extra_flag(e, "optional")) return 0;
    return csv_len(cost_extra(e, "characters")) == 3;
}

/* per_unit divisor (modifiers.rs:1194-1259). per_unit_location overrides the
   counting zone; "stage"+group filters by group, "under_member" flattens the
   2-D under-card structure, everything else defers to zone_cards.

   modifiers.rs:1216-1224 filters the stage through `effect.group_name()`, and
   core/card.rs:2462-2465 defines `group_name()` as `group_names.first()`. The
   extra therefore arrives comma-joined for a multi-entry list ("Aqours,SaintSnow",
   24 decoded effects), and the whole string was compared as ONE exact token, so
   the stage arm counted nothing. Split to the first entry — which is a no-op for
   the 813 single-entry filters. */
static const char *cost_group_name_first(const char *group_names, char *buf, size_t bufsz) {
    if (!group_names || !*group_names) return NULL;
    const char *comma = strchr(group_names, ',');
    size_t len = comma ? (size_t)(comma - group_names) : strlen(group_names);
    if (!buf || bufsz == 0) return group_names;
    if (len >= bufsz) len = bufsz - 1;
    memcpy(buf, group_names, len);
    buf[len] = '\0';
    return buf;
}

static int cost_per_unit_count(const GameState *g, int pl, const AbilityEffect *e) {
    const char *per_unit_loc = cost_extra(e, "per_unit_location");
    const char *loc = cost_extra(e, "location");
    const char *count_zone = (per_unit_loc && *per_unit_loc) ? per_unit_loc
                             : ((loc && *loc) ? loc : "hand");
    const char *group_csv = cost_extra(e, "group_names");
    char group_buf[128];
    const char *group = cost_group_name_first(group_csv, group_buf, sizeof group_buf);
    if (group && *group && !strcmp(count_zone, "stage")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int id = g->p[pl].stage[s];
            if (id == RB_EMPTY_SLOT) continue;
            if (rb_card_matches_group_str(id, group)) n++;
        }
        return n;
    }
    if (!strcmp(count_zone, "under_member")) {
        int under[RB_STAGE_SIZE * 16];
        int host[RB_STAGE_SIZE * 16];
        int m = rb_stage_under_cards_with_hosts(&g->p[pl], under, host,
                                                 RB_STAGE_SIZE * 16);
        if (!group || !*group) return m;
        int n = 0;
        for (int i = 0; i < m; i++)
            if (rb_card_matches_group_str(under[i], group)) n++;
        return n;
    }
    int ids[RB_MAX_ZONE];
    return rb_zone_cards(g, pl, count_zone, ids, RB_MAX_ZONE);
}

/* Is this card a constant-cost host at all? A card that is in neither player's
   stage, hand nor energy zone cannot be the target of any 常時 ModifyCost, so a
   `set` override still sitting on it is provably stale. Membership set mirrors
   the p1_memberships / p2_memberships of modifiers.rs:1120-1137, which also
   chain in energy_zone. */
static int cost_is_constant_host(const GameState *g, int cid) {
    for (int pl = 0; pl < 2; pl++) {
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == cid) return 1;
        for (int h = 0; h < g->p[pl].hand.n; h++)
            if (g->p[pl].hand.cards[h] == cid) return 1;
        for (int h = 0; h < g->p[pl].energy.n; h++)
            if (g->p[pl].energy.cards[h] == cid) return 1;
    }
    return 0;
}

void rb_recalculate_constant_cost_modifiers(GameState *g) {
    if (!g) return;

    int16_t expected_add[RB_MAX_CARD_IDS];
    int16_t expected_set[RB_MAX_CARD_IDS];
    memset(expected_add, 0, sizeof(expected_add));
    memset(expected_set, 0, sizeof(expected_set));

    for (int pl = 0; pl < 2; pl++) {
        for (int slot = 0; slot < RB_STAGE_SIZE + RB_MAX_HAND; slot++) {
            int cid;
            if (slot < RB_STAGE_SIZE) {
                cid = g->p[pl].stage[slot];
            } else {
                int h = slot - RB_STAGE_SIZE;
                if (h >= g->p[pl].hand.n) break;
                cid = g->p[pl].hand.cards[h];
            }
            if (cid < 0) continue;

            int n_abilities = rb_card_num_abilities((uint32_t)cid);
            for (int ai = 0; ai < n_abilities; ai++) {
                Ability ab;
                memset(&ab, 0, sizeof(ab));
                if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
                const AbilityEffect *e = ab.effect;
                int usable = e && e->action && !strcmp(e->action, "modify_cost") &&
                             rb_ability_matches_trigger(&ab, "常時") &&
                             !cost_is_ll_bp7_play_cost(e);
                if (usable) {
                    /* Rust evaluates effect.condition only here — unlike
                       recalculate_constants it does NOT gate on
                       activation_position (modifiers.rs:1187-1190). */
                    int cond_met = !e->has_condition || !e->condition ||
                                   rb_eval_condition_for_host(g, pl, cid, e->condition);
                    if (cond_met) {
                        int value = cost_extra_int(e, "value", 0);
                        if (cost_extra_flag(e, "per_unit")) {
                            int count = cost_per_unit_count(g, pl, e);
                            if (cost_extra_flag(e, "exclude_self") && count > 0) count--;
                            /* modifiers.rs:1569 `effect.per_unit_count_any().unwrap_or(1)`.
                             * `per_unit_count_any()` is `filter_u8_getter!`
                             * (card.rs:1800-1805) — a TYPED FIELD read, and
                             * vm.c:1040 fills only that field (never the
                             * extras), so this extra-only read was always 0 and
                             * every 「N枚につき」 constant cost ran as
                             * 「1枚につき」. */
                            int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : 1;
                            const char *puc = cost_extra(e, "per_unit_count");
                            if (e->per_unit_count <= 0 && puc && *puc) per_unit_count = atoi(puc);
                            if (per_unit_count < 1) per_unit_count = 1;
                            value = (count / per_unit_count) * value;
                        }
                        const char *op = cost_extra(e, "operation");
                        if (!op || !*op) op = "add";
                        if (!strcmp(op, "add")) {
                            expected_add[cid] = saturate_modifier((int64_t)expected_add[cid] + value);
                        } else if (!strcmp(op, "subtract")) {
                            expected_add[cid] = saturate_modifier((int64_t)expected_add[cid] - value);
                        } else if (!strcmp(op, "set")) {
                            expected_set[cid] = saturate_modifier(value);
                        }
                    }
                }
                rb_free_ability(&ab);
            }
        }
    }

    /* Additive pass — the exact diff-and-commit of Rust modifiers.rs:1633-1642:

           let old_bonuses = core::mem::take(&mut self.mods.constant_cost_bonuses);
           if old_bonuses != expected {
               for (cid, old) in &old_bonuses { self.mods.remove_cost_modifier(*cid, *old); }
               for (&cid, &new_val) in &expected { self.mods.add_cost_modifier(cid, new_val); }
           }
           self.mods.constant_cost_bonuses = expected;

       g->mods.constant_cost[cid] is this file's role for
       `constant_cost_bonuses` (game_modifiers.rs:187, HashMap<i16,i16>) — a
       per-card ledger of the last committed additive total, NOT the modifier
       itself. Two things depend on keeping the two apart:
         · the ADDITIVE withdrawal must be sized by what THIS pass wrote last
           time, not by mods.cost[cid].add as a whole (other writers share
           that accumulator);
         · rb_recalc_constants' own prologue (triggers.c:1338-1346) zeroes
           this ledger and undoes its deltas before it gets here, because
           Rust's main constant scan never touches cost at all. So on that
           path old_add is already 0 and the withdrawal below is a no-op; the
           net total is identical either way. Do not "simplify" this into a
           blind re-apply — that is what the previous body did, and it is why
           stale costs accumulated forever.
       NOTE for the triggers.c owner (not this file): that prologue withdraws
       with rb_mods_add_cost(-old), a saturating ADD, where Rust's
       remove_cost_modifier (game_modifiers.rs:535-542) is a saturating
       SUBTRACT clamped at 0. Identical whenever the ledger and the
       accumulator agree; they can diverge by a non-constant writer that
       added a negative delta (triggers.c:1303), and then Rust floors at 0
       while the C can go negative. */
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        int old_add = g->mods.constant_cost[cid];
        int new_add = expected_add[cid];
        if (old_add == new_add) continue;
        if (old_add) rb_mods_remove_cost(&g->mods, cid, old_add);
        if (new_add) rb_mods_add_cost(&g->mods, cid, new_add);
        g->mods.constant_cost[cid] = (int16_t)new_add;
    }

    /* Absolute pass. Rust modifiers.rs:1634 + :1643-1650:

           let old_sets = core::mem::take(&mut self.mods.constant_cost_set_bonuses);
           if old_sets != expected_set {
               for cid in old_sets.keys() { self.mods.remove_cost_modifier_set(*cid); }
               for (&cid, &new_val) in &expected_set { self.mods.set_cost_modifier(cid, new_val); }
           }
           self.mods.constant_cost_set_bonuses = expected_set;

       The withdrawal arm iterates `old_sets` — the cards a CONSTANT set
       bonus was last written for — and nothing else, so it is safe to clear
       exactly those without ever touching a set written by a non-constant
       path (execute_set_cost / effects/cost.rs). That is the whole reason
       Rust keeps a SECOND ledger here and RbMods keeps only one.

       KNOWN RESIDUAL, and the field that closes it. RbMods has no
       constant_cost_set_bonuses (Rust: game_modifiers.rs:188-189,
       `pub constant_cost_set_bonuses: HashMap<i16, i16>` — separate from
       constant_cost_bonuses at :187), so there is no record of WHICH cards
       this pass set, and a stale absolute cost can only be dropped by the
       host-membership proxy below. Concretely: a 「手札にあるこのメンバーカードの
       コストはNになる」 whose condition stops passing keeps overriding the
       card's cost to N for as long as the card is in a hand / stage / energy
       zone, because cost_is_constant_host is still true. Rust drops it on the
       very next pass.

       Field shape requested (add beside RbMods::constant_cost, rabuka.h:~250,
       and next to constant_score/constant_heart):
           int16_t constant_cost_set[RB_MAX_CARD_IDS];
       i16 per card id, exactly mirroring the additive ledger above and the
       Rust HashMap's i16 value; 0 means "this pass set nothing for that card",
       which is why the additive loop needs no separate presence flag either.
       Then the loop collapses onto Rust's shape:
           for cid: old = mods.constant_cost_set[cid];
                     new = expected_set[cid];
                     if (old == new) continue;
                     if (old) rb_mods_clear_cost_set(&mods, cid);
                     if (new) rb_mods_set_cost(&mods, cid, new);
                     mods.constant_cost_set[cid] = new;
       DO NOT clear the new field in rb_mods_clear_card: Rust's
       clear_all_for_card (game_modifiers.rs:611-628) removes
       constant_blade/cost/score/heart_bonuses but NOT
       constant_cost_set_bonuses, so the ledger entry outliving a zone exit is
       Rust's behaviour, not an omission — and it self-heals on the next
       recalculate_constants, which rewrites the whole map.

       Until that field exists, the proxy below is the honest stand-in: it
       never clobbers a non-constant set, and it does drop the set once the
       card stops being a constant-cost host at all. */
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        if (expected_set[cid]) {
            rb_mods_set_cost(&g->mods, cid, expected_set[cid]);
        } else if (g->mods.cost[cid].set != 0 && !cost_is_constant_host(g, cid)) {
            rb_mods_clear_cost_set(&g->mods, cid);
        }
    }
}

/* -- on_cards_left_zones --
   Mirror modifiers.rs:1673-1685 (GameState::on_cards_left_zones): the single
   choke point for rule 4.1.4 zone-exit cleanup (a card that changes zones is a
   NEW card — all runtime state resets). Rust takes a slice and loops; the C
   port takes one card id per call and the zone-exit paths must call it per
   card (Rust call sites: move_cards.rs:339, :1370, :2278, :2548).

   Rust clears clear_all_for_card + clear_gained_abilities_for_card; the latter
   drops FOUR maps (modifiers.rs:1666-1671). The C port clears the two it can
   hold: the modifier tables and g->gained_card_abilities[]. constant_gained_
   abilities, gained_abilities and gained_ability_sources have no C field — see
   the field-addition request in the report. */
void rb_on_cards_left_zones(GameState *g, int card_id) {
    if (!g || card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;
    rb_mods_clear_card(&g->mods, card_id);
    int gained_changed = 0;
    for (size_t i = 0; i < sizeof(g->gained_card_ids) / sizeof(g->gained_card_ids[0]); i++) {
        if (g->gained_card_ids[i] != card_id) continue;
        for (int j = 0; j < g->gained_card_n[i]; j++) {
            rb_free_ability(&g->gained_card_abilities[i][j]);
        }
        memset(g->gained_card_abilities[i], 0, sizeof(g->gained_card_abilities[i]));
        g->gained_card_n[i] = 0;
        g->gained_card_ids[i] = -1;
        gained_changed = 1;
    }
    if (gained_changed) rb_recalc_constants(g);
}
