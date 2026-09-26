#include "rabuka.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
    return rb_modifier_total(m->blade[cid]);
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
    int v = rb_modifier_total(m->heart[cid][color]);
    v += rb_modifier_total(m->heart[cid][RB_HEART_PINK]);
    return v;
}
void rb_mods_add_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->heart[cid][color].add = saturate_modifier((int64_t)m->heart[cid][color].add + delta);
}

/* ── need_heart ── */
int rb_mods_get_need_heart(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return 0;
    return rb_modifier_total(m->need_heart[cid][color]);
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
    return rb_modifier_total(m->score[cid]);
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
    return rb_modifier_total(m->cost[cid]);
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
void rb_mods_remove_need_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    if (m->need_heart[cid][color].add == 0 && m->need_heart[cid][color].set == 0) return;
    m->need_heart[cid][color].add = saturate_modifier((int64_t)m->need_heart[cid][color].add - delta);
}
void rb_mods_remove_score(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    if (m->score[cid].add == 0 && m->score[cid].set == 0) return;
    m->score[cid].add = saturate_modifier((int64_t)m->score[cid].add - delta);
}
void rb_mods_remove_cost(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    if (m->cost[cid].add == 0 && m->cost[cid].set == 0) return;
    /* Rust remove_cost_modifier clamps the subtract at 0 (no negative cost). */
    int64_t v = (int64_t)m->cost[cid].add - delta;
    if (v < 0) v = 0;
    m->cost[cid].add = saturate_modifier(v);
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

/* ── snapshot trace ring (mirror add_*_modifier_with_trace) ── */
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

void rb_mods_add_blade_with_trace(RbMods *m, int cid, int delta,
                                  int source_card_id, const char *ability_text) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    rb_mods_add_blade(m, cid, delta);
    rb_mods_trace_push(m, source_card_id, ability_text, RB_EFFECT_BLADE_BONUS, cid, -1, delta);
}

void rb_mods_add_heart_with_trace(RbMods *m, int cid, int color, int delta,
                                  int source_card_id, const char *ability_text) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    rb_mods_add_heart(m, cid, color, delta);
    rb_mods_trace_push(m, source_card_id, ability_text, RB_EFFECT_HEART_BONUS, cid, color, delta);
}

/* ───────────────────────────── total (game_modifiers.rs) ─────────────────────────────
    Mirror ModifierEntry::total — returns the combined set + additive value.
    set (absolute override) is the base; additive deltas stack on top.
    Mirrors ModifierEntry::total(self) -> i32 in game_modifiers.rs. */
int rb_modifier_total_entry(const RbModifierEntry *e) {
    if (!e) return 0;
    return (int)e->set + (int)e->add;
}

/* -- record_card_appearance -- */
void rb_record_card_appearance(GameState *g, int card_id, int source) {
    if (!g || card_id < 0) return;
    (void)source;
    int found = 0;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++) {
        if (g->cards_appeared_this_turn[i] == card_id) { found = 1; break; }
    }
    if (!found && g->n_cards_appeared_this_turn < 64) {
        g->cards_appeared_this_turn[g->n_cards_appeared_this_turn++] = card_id;
    }
    for (int i = 0; i < g->n_recently_appeared; i++) {
        if (g->recently_appeared[i] == card_id) return;
    }
    if (g->n_recently_appeared < RB_MAX_RECENTLY_MOVED) {
        g->recently_appeared[g->n_recently_appeared++] = card_id;
    }
}

/* -- has_card_appeared_this_turn -- */
int rb_has_card_appeared_this_turn(GameState *g, int card_id) {
    if (!g) return 0;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == card_id) return 1;
    return 0;
}

/* -- clear_card_appearance_tracking -- */
void rb_clear_card_appearance_tracking(GameState *g) {
    if (!g) return;
    g->n_cards_appeared_this_turn = 0;
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

/* -- clear_baton_touch_tracking -- */
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
    if (g->n_batch_movements < (int)(sizeof(g->batch_movements) / sizeof(g->batch_movements[0]))) {
        RbBatchMovement *movement = &g->batch_movements[g->n_batch_movements++];
        movement->moved_card_id = card_id;
        movement->source_zone = from_zone;
        movement->dest_zone = to_zone;
        movement->cause_player_id = causer;
        movement->effect_only = target;
    }
    if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) {
        g->recently_moved[g->n_recently_moved++] = card_id;
    } else {
        memmove(g->recently_moved, g->recently_moved + 1,
                (RB_MAX_RECENTLY_MOVED - 1) * sizeof(g->recently_moved[0]));
        g->recently_moved[RB_MAX_RECENTLY_MOVED - 1] = card_id;
    }
}

/* -- clear_card_movement_tracking -- */
void rb_clear_card_movement_tracking(GameState *g) {
    if (!g) return;
    memset(g->moved_this_turn, 0, sizeof(g->moved_this_turn));
    g->n_cards_appeared_this_turn = 0;
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

/* -- clear_revealed_cards -- */
void rb_clear_revealed_cards(GameState *g) {
    if (!g) return;
    g->n_revealed = 0;
}

/* ══════════════════════ constant cost modifiers ══════════════════════
   Port of modifiers.rs::recalculate_constant_cost_modifiers +
   recalculate_constant_cost_modifiers_with_ids
   (engine/src/core/game_state/modifiers.rs:1109-1330).

   Every 常時 ModifyCost effect on a stage or hand card is re-evaluated into
   two expected maps and then DIFFED against what was committed last time:
     · additive  (operation "add" / "subtract")  -> mods.cost[cid].add
     · absolute  (operation "set")               -> mods.cost[cid].set
   The previous body ("Simplified: reset and re-apply") re-applied
   mods.constant_cost[cid] through rb_mods_set_cost, which (a) turned every
   additive delta into an absolute set-override and (b) never dropped a bonus
   whose condition had stopped passing — stale costs accumulated forever. */

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
   2-D under-card structure, everything else defers to zone_cards. */
static int cost_per_unit_count(const GameState *g, int pl, const AbilityEffect *e) {
    const char *per_unit_loc = cost_extra(e, "per_unit_location");
    const char *loc = cost_extra(e, "location");
    const char *count_zone = (per_unit_loc && *per_unit_loc) ? per_unit_loc
                             : ((loc && *loc) ? loc : "hand");
    const char *group = cost_extra(e, "group_names");
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
   stage nor hand cannot be the target of any 常時 ModifyCost, so a `set`
   override still sitting on it is provably stale. */
static int cost_is_constant_host(const GameState *g, int cid) {
    for (int pl = 0; pl < 2; pl++) {
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == cid) return 1;
        for (int h = 0; h < g->p[pl].hand.n; h++)
            if (g->p[pl].hand.cards[h] == cid) return 1;
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
                            int per_unit_count = cost_extra_int(e, "per_unit_count", 1);
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

    /* Additive pass — remove what is no longer expected, add what is.
       mods.constant_cost[cid] holds the last committed additive total, the
       same role as Rust's mods.constant_cost_bonuses. */
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        int old_add = g->mods.constant_cost[cid];
        int new_add = expected_add[cid];
        if (old_add == new_add) continue;
        if (old_add) rb_mods_remove_cost(&g->mods, cid, old_add);
        if (new_add) rb_mods_add_cost(&g->mods, cid, new_add);
        g->mods.constant_cost[cid] = (int16_t)new_add;
    }

    /* Absolute pass. Rust keeps a dedicated mods.constant_cost_set_bonuses map;
       RbMods has no such field, so a stale set is only dropped when the card
       can no longer be a constant-cost host — never clobbering a set written
       by a non-constant path (execute_set_cost etc.). */
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        if (expected_set[cid]) {
            rb_mods_set_cost(&g->mods, cid, expected_set[cid]);
        } else if (g->mods.cost[cid].set != 0 && !cost_is_constant_host(g, cid)) {
            rb_mods_clear_cost_set(&g->mods, cid);
        }
    }
}

/* -- on_cards_left_zones -- */
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
