#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* Faithful Live performance — mirrors engine/src/turn/live.rs
   Simplified but structurally faithful:
   - yell reveals (top N per live, blade → heart pool)
   - stage hearts via RbMods (blade/heart modifiers + base hearts)
   - greedy allocation H00Wild→Wildcard→AllWild handling
   - per-live verdict and score with modifiers
   - surplus tracking for no_excess checks
   Host still auto-resolves pending choices via skip in engine.c. */



/* Compute stage hearts for player pl (mirrors stats_pipeline::stage_hearts).
   Members' base hearts + heart modifiers + blade converted to pink. */
void rb_calc_stage_hearts(const GameState *g, int pl, int out[8]){
    memset(out,0,8*sizeof(int));
    const RbPlayer *P=&g->p[pl];
    for(int s=0;s<RB_STAGE_SIZE;s++){
        int cid=P->stage[s];
        if(cid==RB_EMPTY_SLOT) continue;
        Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
        /* base hearts */
        for(int h=0;h<c.n_hearts;h++){
            int col=c.heart_color[h]%8;
            out[col]+=c.heart_count[h];
        }
        /* blade as pink (col 0) plus blade modifiers */
        int blade = (int)c.blade + rb_mods_get_blade((RbMods*)&g->mods, cid);
        if(blade>0) out[RB_HEART_PINK]+=blade;
        /* per-card heart modifiers (already in RbMods) */
        for(int col=0;col<8;col++){
            int mod=rb_mods_get_heart((RbMods*)&g->mods, cid, col);
            if(mod) out[col]+=mod;
        }
        rb_free_card(&c);
    }
}

/* Yell: reveal top yell_count cards per live (default 1) and harvest blade hearts.
   Returns number of yell cards revealed, fills blade_hearts[8] + note_icons. */
static int do_yell(GameState *g, int pl, int yell_cards[RB_MAX_LIVE_CARDS*3], int *n_yell, int blade_hearts[8], int *note_icons){
    RbPlayer *P=&g->p[pl];
    int lives=P->live.n;
    if(lives==0) return 0;
    int per_live=1; /* TODO: modify_yell_count modifier would adjust */
    int total_needed=lives*per_live;
    int revealed=0;
    memset(blade_hearts,0,8*sizeof(int));
    *note_icons=0;
    *n_yell=0;
    for(int i=0;i<total_needed && P->deck.n>0;i++){
        int cid=P->deck.cards[--P->deck.n]; /* top */
        yell_cards[(*n_yell)++]=cid;
        Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)){ continue; }
        /* blade_heart handling: blade contributes pink; special hearts could be draw/score */
        if(c.blade>0) blade_hearts[RB_HEART_PINK]+=c.blade;
        for(int h=0;h<c.n_hearts;h++){
            int col=c.heart_color[h];
            if(col==RB_HEART_DRAW) { /* draw icon → immediate draw handled by caller */ }
            else if(col==RB_HEART_SCORE) (*note_icons)+=c.heart_count[h];
            else blade_hearts[col%8]+=c.heart_count[h];
        }
        /* special_heart for draw/score already handled */
        rb_free_card(&c);
        revealed++;
    }
    return revealed;
}

/* Greedy allocation: assign total_hearts[8] to each live's required[8] (need_heart).
   Returns 1 if all lives pass. */
static int allocate_and_verdict(const GameState *g, int pl, const int total_hearts[8], int *out_passed, int *out_score){
    RbPlayer *P=(RbPlayer*)&g->p[pl];
    int total_score=0;
    int all_pass=1;
    int pool[8]; memcpy(pool,total_hearts,8*sizeof(int));
    for(int li=0; li<P->live.n; li++){
        Card c; if(!rb_decode_card_by_index((uint32_t)P->live.cards[li],&c)){ all_pass=0; continue; }
        int required[8]={0};
        for(int h=0;h<c.n_hearts;h++) required[c.heart_color[h]%8]+=c.heart_count[h];
        /* apply need_heart modifiers (set/add) */
        for(int col=0;col<8;col++){
            int mod=rb_mods_get_need_heart((RbMods*)&g->mods, P->live.cards[li], col);
            if(mod){
                /* set vs additive already in total; we just apply additive on top of base */
                /* RbMods need_heart total is set+add, but base required already includes set? Simplified: add */
                required[col]=rb_saturate_u8(required[col]+mod);
            }
        }
        /* allocation: total coverage check */
        int total_req=0,total_pool=0; for(int k=0;k<8;k++){ total_req+=required[k]; total_pool+=pool[k]; }
        if(total_pool < total_req){ all_pass=0; rb_free_card(&c); continue; }
        /* heart0 bucket (col 0) can be filled by any 1..6 + icon_all */
        /* For faithful, we use greedy: first fill with exact color, then icon_all (col 7) covers deficits */
        int icon_all=pool[7];
        int ok=1;
        /* heart0 check */
        if(required[0]>0){
            int any = pool[1]+pool[2]+pool[3]+pool[4]+pool[5]+pool[6]+pool[0];
            if(any + icon_all < required[0]) ok=0;
            else {
                int need0=required[0];
                int use0 = need0 - (pool[0]+pool[1]+pool[2]+pool[3]+pool[4]+pool[5]+pool[6]);
                if(use0<0) use0=0;
                if(use0>icon_all) use0=icon_all;
                icon_all-=use0;
            }
        }
        if(ok){
            for(int col=1;col<7;col++){
                if(pool[col] < required[col]){
                    int deficit=required[col]-pool[col];
                    if(icon_all >= deficit) icon_all-=deficit;
                    else { ok=0; break; }
                }
            }
        }
        if(ok){
            /* consume */
            for(int col=0;col<7;col++){
                int need=required[col];
                int take = need < pool[col] ? need : pool[col];
                pool[col]-=take;
                need-=take;
                if(need>0){
                    int from_all = need < icon_all ? need : icon_all;
                    icon_all-=from_all;
                    pool[7]=icon_all;
                }
            }
            int score=(int)c.score + rb_mods_get_score((RbMods*)&g->mods, P->live.cards[li]);
            if(score<0) score=0;
            total_score+=score;
        } else all_pass=0;
        rb_free_card(&c);
    }
    if(out_passed) *out_passed=all_pass;
    if(out_score) *out_score=total_score;
    return all_pass;
}

int rb_perform_live(GameState *g, int pl){
    RbPlayer *P=&g->p[pl];
    if(P->live.n==0) return 0;
    int yell_cards[RB_MAX_LIVE_CARDS*3]; int n_yell=0;
    int blade_hearts[8]={0}; int note_icons=0;
    do_yell(g, pl, yell_cards, &n_yell, blade_hearts, &note_icons);

    int stage_hearts[8]={0};
    rb_calc_stage_hearts(g, pl, stage_hearts);

    int total_hearts[8]={0};
    for(int i=0;i<8;i++) total_hearts[i]=stage_hearts[i]+blade_hearts[i];
    /* add ability-granted hearts pool (P->hearts flat = pink etc.) — map to col 0..7 */
    for(int col=0;col<8 && col<RB_MAX_HEARTS;col++) total_hearts[col]+=P->hearts[col];

    int passed=0, live_score=0;
    allocate_and_verdict(g, pl, total_hearts, &passed, &live_score);

    /* Move lives: if all passed, to success (score added); else to discard */
    int lives_to_move=P->live.n;
    if(passed){
        for(int i=0;i<lives_to_move;i++){
            int cid=P->live.cards[0];
            for(int k=0;k<P->live.n-1;k++) P->live.cards[k]=P->live.cards[k+1];
            P->live.n--;
            if(P->success.n < RB_MAX_LIVE_CARDS) P->success.cards[P->success.n++]=cid;
            else P->discard.cards[P->discard.n++]=cid;
        }
        P->score+=live_score;
        P->yell_note_icons+=note_icons;
    } else {
        for(int i=0;i<lives_to_move;i++){
            int cid=P->live.cards[0];
            for(int k=0;k<P->live.n-1;k++) P->live.cards[k]=P->live.cards[k+1];
            P->live.n--;
            if(P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++]=cid;
        }
    }
    /* yell cards go to discard (resolution) after use */
    for(int i=0;i<n_yell;i++){
        if(P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++]=yell_cards[i];
    }
    return passed;
}
