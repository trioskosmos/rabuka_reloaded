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
    rb_free_card(&card);
    return has_need;
}

static int sp_effective_need_heart(const GameState *g, int card_id, int out[8]) {
    if (!sp_card_need_hearts(card_id, out)) return 0;
    if (!g || card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 1;

    for (int color = 0; color < 8; color++) {
        RbModifierEntry entry = g->mods.need_heart[card_id][color];
        if (entry.set != 0) out[color] = (uint8_t)entry.set;
    }
    for (int color = 0; color < 8; color++) {
        RbModifierEntry entry = g->mods.need_heart[card_id][color];
        if (entry.add != 0) out[color] = rb_saturate_u8(out[color] + entry.add);
    }
    return 1;
}

void rb_member_original_hearts(const RbMods *mods, int card_id, int out[8]) {
    memset(out, 0, 8 * sizeof(int));
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;

    int override_color = mods ? mods->heart_color_override[card_id] : -1;
    if (override_color >= 0 && override_color < 8) {
        out[override_color] += mods->heart_override_count[card_id];
        return;
    }

    int source_id = mods && mods->heart_copy[card_id] >= 0
                  ? mods->heart_copy[card_id]
                  : card_id;
    sp_card_base_hearts(source_id, out);

    if (mods && mods->heart_multiplier[card_id] >= 0) {
        int color = mods->heart_multiplier[card_id];
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
    if (!sp_any_hearts(effective)) return 1;
    if (!mods || card_id < 0 || card_id >= RB_MAX_CARD_IDS) {
        return rb_check_heart_requirement(effective, provided);
    }

    for (int color = 0; color < 8; color++) {
        RbModifierEntry entry = mods->need_heart[card_id][color];
        if (entry.set != 0) effective[color] = (uint8_t)entry.set;
    }
    for (int color = 0; color < 8; color++) {
        RbModifierEntry entry = mods->need_heart[card_id][color];
        if (entry.add != 0) {
            effective[color] = rb_saturate_u8(effective[color] + entry.add);
        }
    }
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
