#include "rabuka.h"
#include <string.h>

/* Stats pipeline — mirrors engine/src/core/stats_pipeline.rs
   Single source for stage heart computation: base hearts + heart_override
   + heart_modifiers + heart_copy × heart_color_multiplier. Used by
   rb_calc_stage_hearts (live.c) and constant re-eval. */

/* Effective need_heart for a live card: base need + need_heart_modifiers */
void rb_effective_need_heart(const GameState *g, int live_cid, int out[8]){
    Card c; if(!rb_decode_card_by_index((uint32_t)live_cid,&c)){ memset(out,0,8*sizeof(int)); return; }
    for(int i=0;i<8;i++) out[i]=0;
    for(int h=0;h<c.n_hearts;h++) out[c.heart_color[h]%8]+=c.heart_count[h];
    for(int col=0;col<8;col++){
        int mod=rb_mods_get_need_heart((RbMods*)&g->mods, live_cid, col);
        if(mod) out[col]=rb_saturate_u8(out[col]+mod);
    }
    rb_free_card(&c);
}

void rb_member_original_hearts(const RbMods *mods, int card_id, int out[8]);

/* Stage hearts — faithful port of stats_pipeline.rs::stage_hearts.
    For each stage slot: member_original_hearts (override/copy/multiplier
    layers, 9.9.1.1-9.9.1.4), then additive mods stack ON TOP (9.9.1.5),
    then fold into the pooled output. */
void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]){
    memset(out, 0, 8 * sizeof(int));
    if(!g || pl < 0 || pl >= 2) return;
    const RbMods *mods = &g->mods;
    for(int s = 0; s < RB_STAGE_SIZE; s++){
        int card_id = g->p[pl].stage[s];
        if(card_id == RB_EMPTY_SLOT) continue;
        int m[8];
        rb_member_original_hearts(mods, card_id, m);
        /* 9.9.1.5: additive modifiers stack ON TOP of the set/base value. */
        for(int col = 0; col < 8; col++){
            RbModifierEntry e = mods->heart[card_id][col];
            int delta = rb_modifier_total(e);
            if(delta == 0) continue;
            int new_val = rb_saturate_u8(m[col] + delta);
            m[col] = new_val > 0 ? new_val : 0;
        }
        for(int col = 0; col < 8; col++) out[col] += m[col];
    }
}

/* ── Ported from stats_pipeline.rs (unmatched functions) ── */

/* member_original_hearts: compute a member's original hearts after
   copy/multiplier/override layers. Mirrors Rust member_original_hearts.
   out is a flat int[8] array of per-color heart counts. */
void rb_member_original_hearts(const RbMods *mods, int card_id, int out[8]){
    memset(out, 0, 8 * sizeof(int));
    if(card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;

    /* 9.9.1.4: heart_override REPLACES originals outright with (color, count). */
    int override_color = -1;
    if(mods){
        override_color = mods->heart_color_override[card_id];
        if(override_color >= 0 && override_color <= 7){
            out[override_color] = mods->heart_override_count[card_id];
            return;
        }
    }

    int src_id = mods ? mods->heart_copy[card_id] : -1;
    int use_id = (src_id >= 0 && src_id < RB_MAX_CARD_IDS) ? src_id : card_id;

    Card c;
    if(rb_decode_card_by_index((uint32_t)use_id, &c)){
        for(int h = 0; h < c.num_base && h < c.n_hearts; h++){
            int color = c.heart_color[h];
            int col = color <= 6 ? color : (color == 10 ? 7 : 0);
            out[col] += c.heart_count[h];
        }
        rb_free_card(&c);
    }

    /* Color multiplier collapses the whole multiset into one color. */
    int mult_col = mods ? mods->heart_multiplier[card_id] : -1;
    if(mult_col >= 0 && mult_col <= 7){
        int total = 0;
        for(int i = 0; i < 8; i++) total += out[i];
        memset(out, 0, 8 * sizeof(int));
        out[mult_col] = total;
    }
}

/* apply_additive_heart_mods: 9.9.1.5 — additive modifiers stack ON TOP,
   saturating at 0/255. mods is an array of 8 RbModifierEntry (per color).
   Mirrors Rust apply_additive_heart_mods. */
void rb_apply_additive_heart_mods(int hearts[8], const RbModifierEntry *mods){
    for(int col = 0; col < 8; col++){
        int delta = rb_modifier_total(mods[col]);
        if(delta == 0) continue;
        int new_val = rb_saturate_u8(hearts[col] + delta);
        if(new_val > 0) hearts[col] = new_val; else hearts[col] = 0;
    }
}

/* effective_blade_parts: blade layering (9.9.1.4->.5). A non-zero SET
   replaces the printed blade; additive stacks either way.
   Returns (effective_base, additive_bonus) via pointers.
   Mirrors Rust effective_blade_parts. */
void rb_effective_blade_parts(RbModifierEntry entry, int printed_blade, int *base, int *additive){
    if(entry.set != 0){
        *base = rb_saturate_u8(rb_modifier_total(entry));
        *additive = 0;
    } else {
        *base = printed_blade;
        *additive = rb_saturate_u8(rb_modifier_total(entry));
    }
}

/* member_heart_detail: per-member heart detail (base_arr, bonus_arr).
   base = original hearts after copy/multiplier/override (no additives).
   bonus = additive contributions (positive deltas only).
   Mirrors Rust member_heart_detail. */
void rb_member_heart_detail(const RbMods *mods, int card_id, uint8_t base_arr[8], uint8_t bonus_arr[8]){
    int base[8];
    rb_member_original_hearts(mods, card_id, base);
    for(int i = 0; i < 8; i++) base_arr[i] = (uint8_t)base[i];

    memset(bonus_arr, 0, 8);
    if(card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;
    for(int col = 0; col < 8; col++){
        RbModifierEntry entry = mods->heart[card_id][col];
        int total = rb_modifier_total(entry);
        if(total > 0) bonus_arr[col] += (uint8_t)total;
    }
}

/* effective_blade: effective blade for a single card after modifiers.
   Mirrors Rust effective_blade. */
int rb_effective_blade(int card_id, RbModifierEntry entry){
    int printed = 0;
    Card c;
    if(rb_decode_card_by_index((uint32_t)card_id, &c)){
        printed = c.blade;
        rb_free_card(&c);
    }
    if(entry.set != 0){
        return rb_saturate_u8(rb_modifier_total(entry));
    } else {
        return rb_saturate_u8(printed + rb_modifier_total(entry));
    }
}

/* need_satisfied: check whether a live card's need is satisfied by a heart
   pool, using the canonical check_heart_requirement helper.
   Mirrors Rust need_satisfied. Returns 1 if satisfied, 0 otherwise. */
int rb_need_satisfied(const int base_need[8], const int provided[8], int card_id, const RbMods *mods){
    if(card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 1;

    int eff[8];
    const int zeros[8] = {0};
    int empty = 1;
    for(int i = 0; i < 8; i++){
        eff[i] = base_need ? base_need[i] : 0;
        if(eff[i] > 0) empty = 0;
    }
    if(empty) return 1;

    /* Q115/Q127: Set-to-X applies first (per-color), then additive stacks. */
    for(int col = 0; col < 8; col++){
        RbModifierEntry me = mods->need_heart[card_id][col];
        if(me.set != 0) eff[col] = me.set;
    }
    for(int col = 0; col < 8; col++){
        RbModifierEntry me = mods->need_heart[card_id][col];
        if(me.add != 0){
            int new_val = rb_saturate_u8(eff[col] + me.add);
            if(new_val > 0) eff[col] = new_val; else eff[col] = 0;
        }
    }

    empty = 1;
    for(int i = 0; i < 8; i++) if(eff[i] > 0){ empty = 0; break; }
    if(empty) return 1;

    return rb_check_heart_requirement(eff, provided ? provided : zeros);
}
