#include "rabuka.h"
#include <stdio.h>
#include <string.h>

static int sp_heart_index(uint8_t color) {
    if (color <= 6) return color;
    if (color == 10) return 7;
    return 0;
}

static int sp_any_hearts(const int hearts[8]) {
    for (int i = 0; i < 8; i++) {
        if (hearts[i] > 0) return 1;
    }
    return 0;
}

static void sp_debug_hearts(const char *tag, int card_id, const int hearts[8]) {
    if (!rb_ability_debug_enabled()) return;
    fprintf(stderr,
            "[%s] card=%d hearts=[%d,%d,%d,%d,%d,%d,%d,%d]\n",
            tag, card_id, hearts[0], hearts[1], hearts[2], hearts[3],
            hearts[4], hearts[5], hearts[6], hearts[7]);
}

static void sp_card_base_hearts(int card_id, int out[8]) {
    memset(out, 0, 8 * sizeof(int));
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;

    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return;
    for (int i = 0; i < card.num_base && i < card.n_hearts; i++) {
        out[sp_heart_index(card.heart_color[i])] += card.heart_count[i];
    }
    rb_free_card(&card);
}

static int sp_card_need_hearts(int card_id, int out[8]) {
    memset(out, 0, 8 * sizeof(int));
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 0;

    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return 0;
    int start = card.num_base + card.num_blade;
    for (int i = start; i < start + card.num_need && i < card.n_hearts; i++) {
        out[sp_heart_index(card.heart_color[i])] += card.heart_count[i];
    }
    int has_need = card.num_need != 0;
    if (rb_ability_debug_enabled()) {
        fprintf(stderr, "[NEED_DECODE] card=%d base=%d blade=%d need=%d n_hearts=%d out=%d,%d,%d,%d,%d,%d,%d,%d\n",
                card_id, card.num_base, card.num_blade, card.num_need, card.n_hearts,
                out[0], out[1], out[2], out[3], out[4], out[5], out[6], out[7]);
    }
    rb_free_card(&card);
    return has_need;
}

/* The Q115/Q127 layering, isolated: per-colour SET first, then additive
   stacking on top. Rust has this inline inside `effective_need_heart`
   (stats_pipeline.rs:221-251) and `need_satisfied` reaches it by CALLING
   `effective_need_heart` rather than repeating the two loops
   (stats_pipeline.rs:264). This helper is that single definition; both C
   entry points below go through it so the two cannot drift apart the way the
   hand-rolled copies did. `mods` == NULL / card out of range is Rust's
   `need_heart_modifiers.get(&card_id)` returning None. */
static void sp_apply_need_mods(const RbMods *mods, int card_id, int out[8]) {
    if (!mods || card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;

    for (int color = 0; color < 8; color++) {
        RbModifierEntry entry = mods->need_heart[card_id][color];
        if (entry.set != 0) out[color] = rb_saturate_u8(entry.set);
    }
    for (int color = 0; color < 8; color++) {
        RbModifierEntry entry = mods->need_heart[card_id][color];
        if (entry.add != 0) out[color] = rb_saturate_u8(out[color] + entry.add);
    }
}

static int sp_effective_need_heart(const GameState *g, int card_id, int out[8]) {
    if (!sp_card_need_hearts(card_id, out)) return 0;
    if (!g) return 1;
    /* Rust: `if base.hearts.is_empty() { return Some(base.clone()); }`
       (stats_pipeline.rs:210-212) — an EMPTY printed need is returned
       untouched and the per-colour modifiers are never applied. Without this
       arm a card whose need decodes to all-zero buckets but still carries
       `num_need != 0` would be handed to need_heart_modifiers and come back
       demanding hearts Rust never asks for. */
    if (!sp_any_hearts(out)) return 1;

    sp_apply_need_mods(&g->mods, card_id, out);
    return 1;
}

void rb_member_original_hearts(const RbMods *mods, int card_id, int out[8]) {
    memset(out, 0, 8 * sizeof(int));
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;

    /* Rust keys both maps by HeartColor and only narrows to a bucket when a
       CONSUMER asks for one (stats_pipeline.rs:30-34 set-layer, :51-55
       multiplier, :158-162 member_heart_detail). The C port has to pick the
       bucket here, so it has to pick the one HeartColor::index() would:
       heart00..06 -> 0..6, "all" -> 7, and b_all / draw / score -> 0
       (card.rs heart_color_table!). The old `< 8` guard DROPPED a set-to-N of
       any out-of-band colour and fell through to the printed base hearts
       instead, which is not what Rust does. */
    int override_color = mods ? mods->heart_color_override[card_id] : -1;
    if (override_color >= 0) {
        out[sp_heart_index((uint8_t)override_color)] += mods->heart_override_count[card_id];
        return;
    }

    int source_id = mods && mods->heart_copy[card_id] >= 0
                  ? mods->heart_copy[card_id]
                  : card_id;
    sp_card_base_hearts(source_id, out);

    if (mods && mods->heart_multiplier[card_id] >= 0) {
        int color = sp_heart_index((uint8_t)mods->heart_multiplier[card_id]);
        int total = 0;
        for (int i = 0; i < 8; i++) total += out[i];
        memset(out, 0, 8 * sizeof(int));
        out[color] = total;
    }
}

void rb_apply_additive_heart_mods(int hearts[8], const RbModifierEntry *mods) {
    if (!mods) return;
    for (int color = 0; color < 8; color++) {
        int delta = rb_modifier_total(mods[color]);
        if (delta == 0) continue;
        int value = rb_saturate_u8(hearts[color] + delta);
        hearts[color] = value > 0 ? value : 0;
    }
}

void rb_effective_blade_parts(RbModifierEntry entry, int printed_blade,
                              int *base, int *additive) {
    if (!base || !additive) return;
    if (entry.set != 0) {
        *base = rb_saturate_u8(rb_modifier_total(entry));
        *additive = 0;
        return;
    }
    *base = printed_blade;
    *additive = rb_saturate_u8(rb_modifier_total(entry));
}

void rb_member_heart_detail(const RbMods *mods, int card_id,
                            uint8_t base_arr[8], uint8_t bonus_arr[8]) {
    if (!base_arr || !bonus_arr) return;

    int base[8];
    rb_member_original_hearts(mods, card_id, base);
    for (int i = 0; i < 8; i++) {
        base_arr[i] = (uint8_t)rb_saturate_u8(base[i]);
    }

    memset(bonus_arr, 0, 8 * sizeof(uint8_t));
    if (!mods || card_id < 0 || card_id >= RB_MAX_CARD_IDS) {
        sp_debug_hearts("HEART_DETAIL_BASE", card_id, base);
        return;
    }
    for (int color = 0; color < 8; color++) {
        int total = rb_modifier_total(mods->heart[card_id][color]);
        if (total > 0) {
            bonus_arr[color] = (uint8_t)rb_saturate_u8(bonus_arr[color] + total);
        }
    }
    sp_debug_hearts("HEART_DETAIL_BASE", card_id, base);
}

int rb_effective_blade(int card_id, RbModifierEntry entry) {
    int printed = 0;
    Card card;
    if (card_id >= 0 && card_id < RB_MAX_CARD_IDS &&
        rb_decode_card_by_index((uint32_t)card_id, &card)) {
        printed = card.blade;
        rb_free_card(&card);
    }
    if (entry.set != 0) return rb_saturate_u8(rb_modifier_total(entry));
    return rb_saturate_u8(printed + rb_modifier_total(entry));
}

void rb_effective_need_heart(const GameState *g, int live_cid, int out[8]) {
    if (!out) return;
    sp_effective_need_heart(g, live_cid, out);
    sp_debug_hearts("NEED_HEART_PIPELINE", live_cid, out);
}

int rb_need_satisfied(const int base_need[8], const int provided[8],
                      int card_id, const RbMods *mods) {
    if (!base_need) return 1;

    int effective[8];
    int zeros[8] = {0};
    if (!provided) provided = zeros;
    for (int i = 0; i < 8; i++) effective[i] = base_need[i];
    /* Rust: `effective_need_heart` returns the base UNTOUCHED when it is
       empty, then `need_satisfied` sees an empty `eff` and answers true
       (stats_pipeline.rs:210-212 + :266). That is the arm below. */
    if (!sp_any_hearts(effective)) return 1;

    sp_apply_need_mods(mods, card_id, effective);
    if (!sp_any_hearts(effective)) return 1;

    int result = rb_check_heart_requirement(effective, provided);
    if (rb_ability_debug_enabled()) {
        fprintf(stderr, "[NEED_CHECK] card=%d need=%d,%d,%d,%d,%d,%d,%d,%d -> %s\n",
                card_id, effective[0], effective[1], effective[2], effective[3],
                effective[4], effective[5], effective[6], effective[7],
                result ? "PASS" : "FAIL");
    }
    return result;
}

void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]) {
    if (!out) return;
    memset(out, 0, 8 * sizeof(int));
    if (!g || pl < 0 || pl > 1) return;

    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        int card_id = g->p[pl].stage[slot];
        if (card_id == RB_EMPTY_SLOT || card_id < 0 || card_id >= RB_MAX_CARD_IDS) continue;

        int hearts[8];
        rb_member_original_hearts(&g->mods, card_id, hearts);
        rb_apply_additive_heart_mods(hearts, g->mods.heart[card_id]);
        sp_debug_hearts("HEART_PIPELINE", card_id, hearts);
        for (int color = 0; color < 8; color++) out[color] += hearts[color];
    }
}
