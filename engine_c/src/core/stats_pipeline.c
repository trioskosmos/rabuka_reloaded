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
    /* heart_color_multiplier: one colour multiplied */
    for(int s=0;s<RB_STAGE_SIZE;s++){
        int cid=g->p[pl].stage[s];
        if(cid==RB_EMPTY_SLOT) continue;
        int mult_col=g->mods.heart_multiplier[cid];
        if(mult_col<0) continue;
        /* stub: double that colour */
        out[mult_col%8]*=2;
    }
}
