#include "rabuka.h"
#include <string.h>

void rb_mods_init(RbMods *m) {
    memset(m, 0, sizeof(*m));
    for(int i=0;i<RB_MAX_CARD_IDS;i++){ m->heart_copy[i]=-1; m->heart_multiplier[i]=-1; m->heart_multiplier_amt[i]=2; m->blade_type[i]=-1; m->heart_color_override[i]=-1; }
    m->n_trace = 0;
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
    for (int c = 0; c < 8; c++) { m->constant_heart[cid][c] = 0; m->constant_need_heart[cid][c] = 0; }
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
    m->blade[cid].add = rb_saturate_i16((int)m->blade[cid].add + delta);
}
void rb_mods_set_blade(RbMods *m, int cid, int v) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->blade[cid].set = rb_saturate_i16(v);
}

/* ── heart (color 0..7) ── */
int rb_mods_get_heart(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    if (color < 0 || color >= 8) return 0;
    /* Mirrors Rust GameModifiers::get_heart_modifier: Heart00 (index 0, the
       "colorless"/wildcard heart) is added to every color query. */
    int v = rb_modifier_total(m->heart[cid][color]);
    if (color != RB_HEART_PINK) v += rb_modifier_total(m->heart[cid][RB_HEART_PINK]);
    return v;
}
void rb_mods_add_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->heart[cid][color].add = rb_saturate_i16((int)m->heart[cid][color].add + delta);
}

/* ── need_heart ── */
int rb_mods_get_need_heart(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return 0;
    return rb_modifier_total(m->need_heart[cid][color]);
}
void rb_mods_add_need_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->need_heart[cid][color].add = rb_saturate_i16((int)m->need_heart[cid][color].add + delta);
}
void rb_mods_set_need_heart(RbMods *m, int cid, int color, int value) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->need_heart[cid][color].set = rb_saturate_i16(value);
}

/* ── score ── */
int rb_mods_get_score(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return rb_modifier_total(m->score[cid]);
}
void rb_mods_add_score(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->score[cid].add = rb_saturate_i16((int)m->score[cid].add + delta);
}
void rb_mods_set_score(RbMods *m, int cid, int value) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->score[cid].set = rb_saturate_i16(value);
}

/* ── cost ── */
int rb_mods_get_cost(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return rb_modifier_total(m->cost[cid]);
}
void rb_mods_add_cost(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    /* cost additive never goes negative (Rust saturating_sub 0) */
    int v = (int)m->cost[cid].add + delta;
    if (v < 0) v = 0;
    m->cost[cid].add = rb_saturate_i16(v);
}
void rb_mods_set_cost(RbMods *m, int cid, int value) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->cost[cid].set = rb_saturate_i16(value);
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
    m->blade[cid].add = rb_saturate_i16((int)m->blade[cid].add - delta);
}
void rb_mods_remove_heart(RbMods *m, int cid, int color, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || color < 0 || color >= 8) return;
    m->heart[cid][color].add = rb_saturate_i16((int)m->heart[cid][color].add - delta);
}
void rb_mods_remove_score(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->score[cid].add = rb_saturate_i16((int)m->score[cid].add - delta);
}
void rb_mods_remove_cost(RbMods *m, int cid, int delta) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    /* Rust remove_cost_modifier clamps the subtract at 0 (no negative cost). */
    int v = (int)m->cost[cid].add - delta;
    if (v < 0) v = 0;
    m->cost[cid].add = rb_saturate_i16(v);
}

/* ── heart_override (mirror set_heart_override / remove_heart_override) ──
   The C field stores only the override heart color (Rust keeps (color, count);
   the count is not consumed by the portable core). -1 means "no override". */
void rb_mods_set_heart_override(RbMods *m, int cid, int color) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->heart_color_override[cid] = (int8_t)color;
}
void rb_mods_remove_heart_override(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return;
    m->heart_color_override[cid] = -1;
}
int rb_mods_get_heart_override(RbMods *m, int cid) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return -1;
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
    e->amount = (int16_t)amount;
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
    if (g->n_cards_appeared_this_turn < 64) {
        g->cards_appeared_this_turn[g->n_cards_appeared_this_turn++] = card_id;
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
void rb_record_baton_touch(GameState *g, int pl) {
    if (!g) return;
    g->baton_touch_used[pl] = 1;
}

/* -- get_baton_touch_count -- */
int rb_get_baton_touch_count(const GameState *g, int pl) {
    return g ? g->baton_touch_used[pl] : 0;
}

/* -- clear_baton_touch_tracking -- */
void rb_clear_baton_touch_tracking(GameState *g) {
    if (!g) return;
    g->baton_touch_used[0] = 0;
    g->baton_touch_used[1] = 0;
}

/* -- record_card_movement -- */
void rb_record_card_movement(GameState *g, int card_id, int from_zone, int to_zone, int causer, int target) {
    if (!g || card_id < 0) return;
    if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
        g->recently_moved[g->n_recently_moved++] = card_id;
}

/* -- clear_card_movement_tracking -- */
void rb_clear_card_movement_tracking(GameState *g) {
    if (!g) return;
    g->n_recently_moved = 0;
}

/* -- remove_revealed_card -- */
void rb_remove_revealed_card(GameState *g, int card_id) {
    if (!g) return;
    for (int i = 0; i < g->n_revealed; i++) {
        if (g->revealed_cards[i] == card_id) {
            for (int j = i; j < g->n_revealed - 1; j++)
                g->revealed_cards[j] = g->revealed_cards[j + 1];
            g->n_revealed--;
            return;
        }
    }
}

/* -- clear_revealed_cards -- */
void rb_clear_revealed_cards(GameState *g) {
    if (!g) return;
    g->n_revealed = 0;
}

/* -- recalculate_constant_cost_modifiers -- */
void rb_recalculate_constant_cost_modifiers(GameState *g) {
    if (!g) return;
    /* Simplified: reset and re-apply constant cost modifiers */
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) {
        if (g->mods.constant_cost[i]) {
            rb_mods_set_cost(&g->mods, i, g->mods.constant_cost[i]);
        }
    }
}

/* -- on_cards_left_zones -- */
void rb_on_cards_left_zones(GameState *g, int card_id) {
    if (!g || card_id < 0) return;
    /* Simplified: clear gained abilities when card leaves zone */
    (void)g; (void)card_id;
}