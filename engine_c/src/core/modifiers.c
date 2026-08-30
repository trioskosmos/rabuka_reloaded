#include "rabuka.h"
#include <string.h>

void rb_mods_init(RbMods *m) {
    memset(m, 0, sizeof(*m));
    for(int i=0;i<RB_MAX_CARD_IDS;i++){ m->heart_copy[i]=-1; m->heart_multiplier[i]=-1; m->heart_multiplier_amt[i]=2; m->blade_type[i]=-1; m->heart_color_override[i]=-1; }
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
    return rb_modifier_total(m->heart[cid][color]);
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
