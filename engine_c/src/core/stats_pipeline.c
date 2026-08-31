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

/* Stage hearts with copy/multiplier — mirrors Player::calculate_stage_hearts */
void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]){
    rb_calc_stage_hearts(g, pl, out);
    /* heart_copy: target copies source's base hearts */
    for(int s=0;s<RB_STAGE_SIZE;s++){
        int target=g->p[pl].stage[s];
        if(target==RB_EMPTY_SLOT) continue;
        int src=g->mods.heart_copy[target];
        if(src<=0 || src>=RB_MAX_CARD_IDS) continue;
        /* heart_copy REPLACES the target's own base hearts with the source's. */
        Card tc; if(rb_decode_card_by_index((uint32_t)target,&tc)){
            for(int h=0;h<tc.n_hearts;h++){
                int col=tc.heart_color[h]%8;
                out[col]-=tc.heart_count[h];
                if(out[col]<0) out[col]=0;
            }
            rb_free_card(&tc);
        }
        Card sc; if(rb_decode_card_by_index((uint32_t)src,&sc)){
            for(int h=0;h<sc.n_hearts;h++) out[sc.heart_color[h]%8]+=sc.heart_count[h];
            rb_free_card(&sc);
        }
    }
    /* heart_color_multiplier: one colour multiplied by the stored amount */
    for(int s=0;s<RB_STAGE_SIZE;s++){
        int cid=g->p[pl].stage[s];
        if(cid==RB_EMPTY_SLOT) continue;
        int mult_col=g->mods.heart_multiplier[cid];
        if(mult_col<0) continue;
        int amt=g->mods.heart_multiplier_amt[cid];
        if(amt<1) amt=2;
        out[mult_col%8]*=amt;
    }
}

/* ── Ported from stats_pipeline.rs (unmatched functions) ── */

/* member_original_hearts: compute a member's original hearts after
   copy/multiplier/override layers. Mirrors Rust member_original_hearts.
   out is a flat int[8] array of per-color heart counts. */
void rb_member_original_hearts(const RbMods *mods, int card_id, int out[8]){
    memset(out, 0, 8 * sizeof(int));
    if(card_id < 0 || card_id >= RB_MAX_CARD_IDS) return;

    /* 9.9.1.4: heart_override replaces the member's original hearts outright.
       The C field stores only the override color (Rust keeps (color, count);
       the count is not consumed by the portable core). So we recolor all base
       hearts to the override color. */
    int override_color = mods->heart_color_override[card_id];
    int src_id = mods->heart_copy[card_id];
    int use_id = (src_id > 0 && src_id < RB_MAX_CARD_IDS) ? src_id : card_id;

    Card c;
    if(rb_decode_card_by_index((uint32_t)use_id, &c)){
        for(int h = 0; h < c.n_hearts; h++){
            int col = (override_color >= 0 && override_color <= 7) ? override_color : (c.heart_color[h] % 8);
            out[col] += c.heart_count[h];
        }
        rb_free_card(&c);
    }

    /* Color multiplier collapses the whole multiset into one color. */
    int mult_col = mods->heart_multiplier[card_id];
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

    /* check_heart_requirement: total check + per-color with wildcards */
    int total_need = 0, total_prov = 0;
    for(int i = 0; i < 8; i++){
        if(eff[i] > 0) total_need += eff[i];
        if(provided && provided[i] > 0) total_prov += provided[i];
    }
    if(total_prov < total_need) return 0;

    int wildcard = (provided ? provided[0] : 0) + (provided ? provided[7] : 0);
    for(int c = 1; c < 7; c++){
        if(!provided) continue;
        int deficit = eff[c] - provided[c];
        if(deficit > 0){
            if(wildcard < deficit) return 0;
            wildcard -= deficit;
        }
    }
    return 1;
}
