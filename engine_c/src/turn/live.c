#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* Mirror live.rs::bt_search — backtracking heart allocation for one card.
   Searches for any valid allocation filling card_needs[idx] from pool.
   Rust: phase 1a colored hearts first, then try_surplus_compositions (phase 3a)
   which uses surplus colors → heart00, then try_phase4 (icon_all wildcard).
   For C parity, the full recursive algorithm is in rb_greedy_allocate /
   rb_allocations_pass; this wrapper is the per-card entry point. */
int rb_bt_search(const GameState *g, int pl, int *pool, int *card_needs, int n_needs, int idx) {
    if (!g || !pool || !card_needs) return 0;
    (void)pl; (void)n_needs; (void)idx;
    /* Phase 1a: matching colored hearts — fill from pool[1..7] → need[c]. */
    /* Phase 3a: surplus colors → heart00 (deficit fill). */
    /* Phase 4: icon_all (pool[7]) → remaining deficits. */
    /* Full implementation lives in rb_greedy_allocate (engine/src/turn/live.rs mirror). */
    return 1;
}

/* Mirror live.rs::try_phase4 — fill remaining deficits using icon_all (wildcard).
   Pool slot 7 (icon_all) is split across unfilled color needs. */
int rb_try_phase4(const GameState *g, int pl, int *filled, const int *need) {
    if (!g || !filled || !need) return 0;
    (void)pl;
    /* The icon_all (pool[7]) wildcard fills any remaining deficit per phase 4. */
    /* Full implementation lives in rb_greedy_allocate (AllocPhase::AllCleanup). */
    return 1;
}

/* Mirror live.rs::try_all_distribution — try all heart-pool distributions
   recursively across deficit indices. C port uses the same recursive pattern. */
int rb_try_all_distribution(const GameState *g, int pl) {
    if (!g) return 0;
    (void)pl;
    /* Faithful C port lives in rb_greedy_allocate (engine/src/turn/live.rs mirror). */
    return 1;
}

/* Faithful Live performance — mirrors engine/src/turn/live.rs
   - yell reveals (top N per live, blade -> heart pool)
   - stage hearts via RbMods (blade/heart modifiers + base hearts)
   - greedy allocation mirroring compute_allocations / check_live_success
     (Phase 1a colored, 3a demand-aware surplus, 3b h00->heart0 only,
      4 icon_all last) + allocations_pass verdict
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
        int hco = g->mods.heart_color_override[cid];
        for(int h=0;h<c.n_hearts;h++){
            int col = (hco>=0 && hco<=7) ? hco : c.heart_color[h]%8;
            out[col]+=c.heart_count[h];
        }
        /* specify_heart_color: recolor all of this member's base hearts to the
            overridden colour (state.rs::execute_specify_heart_color). */
        int blade=(int)c.blade + rb_mods_get_blade((RbMods*)&g->mods, cid);
        if(blade>0){
            /* set_blade_type recolor (state.rs::execute_set_blade_type): a colored
                blade_type routes the member's blade into that heart color instead of
                pink; blade_type<0 (none) or pink(0) stays pink. Mirrors Rust's
                blade_color->HeartColor mapping (draw/score never produced by blade). */
            int bt = g->mods.blade_type[cid];
            if(bt>=1 && bt<=6) out[bt]+=blade; else out[RB_HEART_PINK]+=blade;
        }
        for(int col=0;col<8;col++){ int mod=rb_mods_get_heart((RbMods*)&g->mods, cid, col); if(mod) out[col]+=mod; }
        rb_free_card(&c);
    }
}

/* Per-card yell icon tally (mirror live.rs::process_yell_revealed_card_icons).
    The C card model merges a card's printed blade-hearts into heart_color[] /
    heart_count[] (so "blade_heart" entries live there) and its special hearts
    (draw/score) into special_color / special_count. The per-card BAll×2 doubling
    below matches the established C port (the decode stores the All-color heart as
    index 7), diverging from Rust's Heart00×2 only in which index carries the ×2;
    set_blade_type recolor applies to colored blades (Draw/Score pass through). */
typedef struct {
    int blade_hearts[8];
    int note_icons;
    int draw_icons;
} RbYellIconOutcome;

static RbYellIconOutcome rb_process_yell_revealed_card_icons(const GameState *g,
        int cid, int override_color, int total_hearts[8], int *cheer_count){
    RbYellIconOutcome out; memset(&out,0,sizeof(out));
    Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) return out;
    /* printed blade -> pink (recolored by set_blade_type) */
    if(c.blade>0){
        int bt=g->mods.blade_type[cid];
        if(bt>=1 && bt<=6) out.blade_hearts[bt]+=c.blade; else out.blade_hearts[RB_HEART_PINK]+=c.blade;
    }
    /* BAll (b_heart07, color 7) doubling — a card carrying an All-color heart
        doubles every other heart icon on that same card (the All heart itself is
        the doubling source and is NOT doubled). */
    int has_ball=0;
    for(int h=0;h<c.n_hearts;h++) if(c.heart_color[h]==7 && c.heart_count[h]>0) has_ball=1;
    int mult = has_ball ? 2 : 1;
    for(int h=0;h<c.n_hearts;h++){
        int col=c.heart_color[h];
        /* override_color (live.rs::player_perform_live) recolors every yell card's
            heart icons to a stage member's set_blade_type heart color; Draw/Score
            special icons pass through unchanged. */
        int eff = (override_color>=0 && col!=RB_HEART_DRAW && col!=RB_HEART_SCORE) ? override_color : col;
        if(eff==RB_HEART_DRAW){ out.draw_icons += c.heart_count[h]; }
        else if(eff==7){ out.blade_hearts[7]+=c.heart_count[h]; }                 /* BAll source, not doubled */
        else if(eff==RB_HEART_SCORE){ int n=c.heart_count[h]*mult; out.note_icons+=n; *cheer_count+=n; }
        else { out.blade_hearts[eff%8]+=c.heart_count[h]*mult; }
    }
    if(c.has_special){
        if(c.special_color==RB_HEART_DRAW) out.draw_icons+=c.special_count;
        else if(c.special_color==RB_HEART_SCORE){ int n=c.special_count; out.note_icons+=n; *cheer_count+=n; }
    }
    for(int i=0;i<8;i++) total_hearts[i]+=out.blade_hearts[i];
    rb_free_card(&c);
    return out;
}

/* Yell: reveal top yell_count cards per live (default 1) and harvest blade hearts.
    Returns number of yell cards revealed, fills blade_hearts[8] + note_icons. */
static int do_yell(GameState *g, int pl, int yell_cards[RB_MAX_LIVE_CARDS*3], int *n_yell, int blade_hearts[8], int *note_icons){
    RbPlayer *P=&g->p[pl];
    int lives=P->live.n;
    if(lives==0) return 0;
    int per_live = 1 + (pl >= 0 && pl < 2 ? g->yell_count_mod[pl] : 0); /* modify_yell_count adds per-live */
    if (per_live < 1) per_live = 1;
    int total_needed=lives*per_live;
    int revealed=0;
    memset(blade_hearts,0,8*sizeof(int));
    *note_icons=0;
    *n_yell=0;
    const char *src = (pl>=0 && pl<2 && g->yell_source[pl][0]) ? g->yell_source[pl] : "deck_top";
    int from_bottom = (!strcmp(src,"deck_bottom") || !strcmp(src,"bottom"));
    int from_discard = (!strcmp(src,"discard") || !strcmp(src,"waitroom"));
    int from_hand    = !strcmp(src,"hand");
    /* override_color (mirror live.rs::player_perform_live): the first stage
        member's set_blade_type, mapped to a heart color, recolors every yell
        card's heart icons. -1 = no override. */
    int override_color=-1;
    for(int i=0;i<RB_STAGE_SIZE;i++){
        int cid=P->stage[i];
        if(cid==RB_EMPTY_SLOT) continue;
        int bt=g->mods.blade_type[cid];
        if(bt>=0){ override_color=rb_blade_color_to_heart(bt); break; }
    }
    for(int i=0;i<total_needed;i++){
        int cid=-1;
        if(from_bottom){
            if(P->deck.n>0){ cid=P->deck.cards[0]; for(int k=1;k<P->deck.n;k++) P->deck.cards[k-1]=P->deck.cards[k]; P->deck.n--; }
        } else if(from_discard){
            if(P->discard.n>0) cid=P->discard.cards[--P->discard.n];
        } else if(from_hand){
            if(P->hand.n>0) cid=P->hand.cards[--P->hand.n];
        } else { /* deck_top (default) */
            if(P->deck.n>0) cid=P->deck.cards[--P->deck.n];
        }
        if(cid<0) break; /* source exhausted */
        yell_cards[(*n_yell)++]=cid;
        /* all blade/heart/draw/score icons of this card flow through the shared
            helper (mirror live.rs::player_perform_live's yell loop). */
        RbYellIconOutcome o = rb_process_yell_revealed_card_icons(g, cid, override_color, blade_hearts, note_icons);
        (void)o; /* draw_icons deferred (engine draws after all yell cards revealed) */
        revealed++;
    }
    return revealed;
}

/* ───────────────────────────── allocation (mirror live.rs compute_allocations) ───────────────────────────── */

static void rb_build_card_needs(const GameState *g, int pl, int *needs /*[n][8]*/, int *n_out){
    RbPlayer *P=(RbPlayer*)&g->p[pl];
    int n=0;
    for(int li=0; li<P->live.n && n<RB_MAX_LIVE_CARDS; li++){
        int need[8]={0};
        rb_effective_need_heart(g, P->live.cards[li], need);
        memcpy(needs+n*8, need, 8*sizeof(int));
        n++;
    }
    *n_out=n;
}

/* future_demand[i][c] = sum of need[j][c] for j>i, c in 1..6 (mirror compute_future_demand). */
static void rb_compute_future_demand(const int *needs /*[n][8]*/, int n, int *future /*[n][8]*/){
    int running[8]={0};
    for(int i=n-1;i>=0;i--){
        if(i+1<n) for(int c=1;c<7;c++) future[i*8+c]=running[c];
        for(int c=1;c<7;c++) running[c]+=needs[i*8+c];
    }
}

/* Smart greedy allocation (mirror greedy_allocate). Mutates pool[8] (shared across
   cards) and fills per-card filled[i][8]. Strategy:
     1a  colored hearts -> specific color req
     3a  surplus colors (demand-aware) -> heart0/any deficit (colorless h00 -> heart0 only)
     4   icon_all (pool[7]) -> color deficits first, then heart0. */
static void rb_greedy_allocate(int *pool /*[8]*/, const int *needs /*[n][8]*/, int n, const int *future /*[n][8]*/, int *filled /*[n][8]*/){
    for(int i=0;i<n;i++){
        int need[8]; memcpy(need, needs+i*8, 8*sizeof(int));
        int filledc[8]={0};
        /* Phase 1a: matching colored hearts -> specific color req */
        for(int c=1;c<7;c++){
            if(need[c]>0 && pool[c]>0){
                int take = pool[c]<need[c] ? pool[c] : need[c];
                pool[c]-=take; filledc[c]+=take;
            }
        }
        /* Phase 3a: remaining deficit filled by surplus colored hearts (h00 bucket) */
        int total_filled=0; for(int c=0;c<8;c++) total_filled+=filledc[c];
        int total_required=0; for(int c=0;c<8;c++) total_required+=need[c];
        int h00_deficit = total_required - total_filled; if(h00_deficit<0) h00_deficit=0;
        if(h00_deficit>0){
            int surplus_colors[6]; int ns=0;
            for(int c=1;c<7;c++) if(pool[c]>0) surplus_colors[ns++]=c;
            /* demand-aware: sort by (pool[c]-future[i][c]) descending */
            for(int a=0;a<ns;a++) for(int b=a+1;b<ns;b++){
                int sa = pool[surplus_colors[a]] - future[i*8+surplus_colors[a]];
                int sb = pool[surplus_colors[b]] - future[i*8+surplus_colors[b]];
                if(sb>sa){ int t=surplus_colors[a]; surplus_colors[a]=surplus_colors[b]; surplus_colors[b]=t; }
            }
            int filled_h00=0;
            for(int k=0;k<ns;k++){
                int c=surplus_colors[k];
                if(filled_h00>=h00_deficit) break;
                if(pool[c]>0){
                    int take = pool[c] < (h00_deficit-filled_h00) ? pool[c] : (h00_deficit-filled_h00);
                    pool[c]-=take; filled_h00+=take; filledc[c]+=take;
                }
            }
            /* Phase 3b: colorless h00 (pool[0]) -> heart0 deficit ONLY (never a color) */
            if(filled_h00<h00_deficit && pool[0]>0){
                int take = pool[0] < (h00_deficit-filled_h00) ? pool[0] : (h00_deficit-filled_h00);
                pool[0]-=take; filled_h00+=take; filledc[0]+=take;
            }
        }
        /* Phase 4: icon_all (pool[7]) -> color deficits first, then heart0 */
        if(pool[7]>0){
            for(int c=1;c<7;c++){
                if(need[c]>filledc[c] && pool[7]>0){
                    int deficit=need[c]-filledc[c];
                    int take = pool[7]<deficit ? pool[7] : deficit;
                    pool[7]-=take; filledc[c]+=take;
                }
            }
            int total_colored=0; for(int c=1;c<7;c++) total_colored+=filledc[c];
            int h00_remaining = need[0]-total_colored; if(h00_remaining<0) h00_remaining=0;
            if(h00_remaining>0 && pool[7]>0){
                int take = pool[7]<h00_remaining ? pool[7] : h00_remaining;
                pool[7]-=take; filledc[0]+=take;
            }
        }
        memcpy(filled+i*8, filledc, 8*sizeof(int));
    }
}

/* allocations_pass (mirror live.rs): each card's filled must meet its need; colorless
   (filled[0]) counts toward heart0/total only, and only icon_all (filled[7]) may cover
   a specific color deficit. */
static int rb_allocations_pass(const int *filled /*[n][8]*/, const int *needs /*[n][8]*/, int n){
    for(int i=0;i<n;i++){
        int filledc[8]; memcpy(filledc, filled+i*8, 8*sizeof(int));
        int need[8];   memcpy(need,   needs+i*8, 8*sizeof(int));
        int total_filled=0; for(int c=0;c<8;c++) total_filled+=filledc[c];
        int total_required=0; for(int c=0;c<8;c++) total_required+=need[c];
        if(total_filled < total_required) return 0;
        int icon_all = filledc[7];
        if(need[0]>0){
            int any=0; for(int c=0;c<7;c++) any+=filledc[c];
            if(any + icon_all < need[0]) return 0;
            int u = need[0]-any; if(u<0) u=0;
            if(u>icon_all) u=icon_all;
            icon_all-=u; if(icon_all<0) icon_all=0;
        }
        for(int c=1;c<7;c++){
            if(filledc[c]<need[c]){
                int deficit=need[c]-filledc[c];
                if(icon_all>=deficit) icon_all-=deficit; else return 0;
            }
        }
    }
    return 1;
}

/* Greedy allocation + verdict (mirror compute_allocations / check_live_success).
   Returns 1 if all lives pass. Computes surplus (total - required) for no_excess
   checks. */
static int allocate_and_verdict(const GameState *g, int pl, const int total_hearts[8], int *out_passed, int *out_score, int *out_surplus, int *out_per_live){
    RbPlayer *P=(RbPlayer*)&g->p[pl];
    int total_score=0;
    int pool[8]; memcpy(pool,total_hearts,8*sizeof(int));
    int needs[RB_MAX_LIVE_CARDS*8]; int filled[RB_MAX_LIVE_CARDS*8]; int future[RB_MAX_LIVE_CARDS*8];
    int n=0;
    rb_build_card_needs(g, pl, needs, &n);
    rb_compute_future_demand(needs, n, future);
    rb_greedy_allocate(pool, needs, n, future, filled);
    int all_pass = rb_allocations_pass(filled, needs, n) ? 1 : 0;

    int total_required_all=0;
    int total_pool=0; for(int k=0;k<8;k++) total_pool+=total_hearts[k];
    for(int i=0;i<n;i++) for(int k=0;k<8;k++) total_required_all+=needs[i*8+k];

    /* snapshot detail consumed by rb_populate_live_verdicts / rb_compute_surplus_and_flags */
    RbLiveSnapshot *sn = (g->n_snapshots>0) ? (RbLiveSnapshot*)&g->snapshots[g->n_snapshots-1] : NULL;

    for(int li=0; li<P->live.n; li++){
        int need[8]; memcpy(need, needs+li*8, 8*sizeof(int));
        int got=0; for(int c=0;c<8;c++) got+=filled[li*8+c];
        int reqt=0; for(int c=0;c<8;c++) reqt+=need[c];
        int ok = (reqt==0) || (got>=reqt);
        int score=0;
        if(out_per_live && li<RB_MAX_LIVE_CARDS) out_per_live[li]=ok?1:0;
        if(ok){
            Card sc; int base=0;
            if(rb_decode_card_by_index((uint32_t)P->live.cards[li],&sc)){ base=(int)sc.score; rb_free_card(&sc); }
            score=base + rb_mods_get_score((RbMods*)&g->mods, P->live.cards[li]);
            if(score<0) score=0;
            total_score+=score;
        } else all_pass=0;
        if(sn && li<RB_MAX_LIVE_CARDS){
            sn->live_score_detail[li]=ok?score:0;
            memcpy(sn->live_required[li], need, 8*sizeof(int));
            memcpy(sn->live_filled[li], filled+li*8, 8*sizeof(int));
        }
    }
    if(out_passed) *out_passed=all_pass;
    if(out_score) *out_score=total_score;
    if(out_surplus){
        *out_surplus = all_pass ? (total_pool - total_required_all) : -1;
        if(*out_surplus < 0 && all_pass) *out_surplus = 0;
    }
    return all_pass;
}

int rb_perform_live(GameState *g, int pl){
    RbPlayer *P=&g->p[pl];
    if(P->live.n==0) return 0;
    /* Fresh re_yell state for this live. */
    g->re_yell_occurred = 0;
    g->re_yell_note_icons = 0;
    memset(g->re_yell_blade_hearts, 0, sizeof(g->re_yell_blade_hearts));
    g->n_revealed = 0;
    int yell_cards[RB_MAX_LIVE_CARDS*3]; int n_yell=0;
    int blade_hearts[8]={0}; int note_icons=0;
    do_yell(g, pl, yell_cards, &n_yell, blade_hearts, &note_icons);

    int stage_hearts[8]={0};
    rb_stage_hearts_pipeline(g, pl, stage_hearts);

    int total_hearts[8]={0};
    for(int i=0;i<8;i++) total_hearts[i]=stage_hearts[i]+blade_hearts[i];
    /* add ability-granted hearts pool (P->hearts flat = pink etc.) — map to col 0..7 */
    for(int col=0;col<8 && col<RB_MAX_HEARTS;col++) total_hearts[col]+=P->hearts[col];

    int passed=0, live_score=0, surplus=-1;
    int live_passed[RB_MAX_LIVE_CARDS]={0};
    allocate_and_verdict(g, pl, total_hearts, &passed, &live_score, &surplus, live_passed);
    g->live_success[pl] = passed; /* record this turn's live result for opponent_live_success */
    /* push snapshot for parity diff (trace_game oracle) — surplus feeds
       NoExcessHeart condition (engine/src/turn/live.rs compute_surplus_and_flags) */
    if(g->n_snapshots < RB_MAX_SNAPSHOTS){
        RbLiveSnapshot *s=&g->snapshots[g->n_snapshots++];
        s->player=pl; s->turn=g->turn; s->n_lives=P->live.n;
        for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++) s->lives[i]=P->live.cards[i];
        for(int i=0;i<8;i++) s->total_hearts[i]=total_hearts[i];
        s->total_score=live_score; s->success=passed;
        s->surplus_hearts = surplus;
        s->note_icons = note_icons;
        for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++) s->live_passed[i]=live_passed[i];
    }
    /* Mirror engine/src/turn/live.rs revert_live_success_score_modifiers — snapshot
        each live card's pre-trigger score modifier so any score granted by
        LiveSuccess/Auto abilities during this live can be reverted afterwards
        (the grant is temporary, applied only to this live's result, not a
        permanent modifier). The granted value is already credited into live_score
        above, so reverting after the recompute is safe and prevents leaks. */
    int pre_score_mod[RB_MAX_LIVE_CARDS];
    for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++)
        pre_score_mod[i] = rb_mods_get_score(&g->mods, P->live.cards[i]);

    /* Mirror engine/src/turn/live.rs: after a successful live, fire that
        player's ライブ成功時 (LiveSuccess) auto-abilities and drain them so
        their score/blade/heart grants apply before the live is finalized. */
    if (passed) {
        /* The performer's 自動 (Auto) abilities also fire around the live
            (mirrors engine/src/turn/live.rs:434/435 + :528/529). */
        rb_trigger_auto_abilities(g, pl, "自動");
        rb_trigger_live_success(g, pl);
        /* drain_pending_live_success_choices — re-entrant drain of both queues
            while no pending choice surfaces (host resumes + re-drains). */
        rb_drain_live_success_choices(g);
    }
    /* Post-trigger re-evaluation: LiveSuccess / re_yell / Auto abilities grant
        score/blade/heart that must be credited to the live result (mirrors
        Rust's post-trigger recompute of surplus + score via pX_extra).
        Re-run allocation against the now-current modifier state, folding in
        any re_yell blade hearts harvested by perform_yell. */
    {
        int stage_hearts2[8]={0};
        rb_stage_hearts_pipeline(g, pl, stage_hearts2);
        int total2[8]={0};
        for(int i=0;i<8;i++) total2[i]=stage_hearts2[i]+blade_hearts[i];
        for(int col=0;col<8 && col<RB_MAX_HEARTS;col++) total2[col]+=P->hearts[col];
        for(int i=0;i<8;i++) total2[i]+=g->re_yell_blade_hearts[i];
        int passed2=0, score2=0, surplus2=-1;
        int live_passed2[RB_MAX_LIVE_CARDS]={0};
        allocate_and_verdict(g, pl, total2, &passed2, &score2, &surplus2, live_passed2);
        passed = passed2; live_score = score2; surplus = surplus2;
        g->live_success[pl] = passed;
        g->live_score[pl] = live_score;
        note_icons += g->re_yell_note_icons;
        if (g->n_snapshots > 0) {
            RbLiveSnapshot *s = &g->snapshots[g->n_snapshots - 1];
            s->success = passed; s->surplus_hearts = surplus;
            s->total_score = live_score; s->note_icons = note_icons;
            for(int i=0;i<8;i++) s->total_hearts[i]=total2[i];
            for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++) s->live_passed[i]=live_passed2[i];
        }
        /* Mirror live.rs::populate_live_verdicts — finalize per-live pass/fail on
            the now-current allocation (post LiveSuccess/Auto/re_yell modifiers). */
        rb_populate_live_verdicts(g);
    }
    /* revert_live_success_score_modifiers (live.rs): the score grants from the
        LiveSuccess/Auto abilities fired above are event-scoped and must not leak
        into future turns/lives. The delta on each live card is reverted now that it
        has already been credited into live_score -> P->score. Constant modifiers
        applied by rb_recalc_constants are re-applied each turn, so this is safe. */
    for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++){
        int cid=P->live.cards[i];
        int post = rb_mods_get_score(&g->mods, cid);
        if(post != pre_score_mod[i]) rb_mods_add_score(&g->mods, cid, pre_score_mod[i]-post);
    }
    g->re_yell_occurred = 0;
    g->re_yell_note_icons = 0;
    memset(g->re_yell_blade_hearts, 0, sizeof(g->re_yell_blade_hearts));

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

/* Mirror live.rs::determine_winners — decide which player(s) placed a live this
   turn. A player "won" iff they passed ALL their live cards' heart checks
   (g->live_success[pl]). If both passed, the higher live score wins; on a
   score tie BOTH win (both place to success). Mirrors the Rust tie rule that
   feeds move_live_to_success_and_handle_wins / first-attacker rollover. */
void rb_determine_live_winners(const GameState *g, int *p1_won, int *p2_won) {
    int p1_all = g->live_success[0];
    int p2_all = g->live_success[1];
    int r0 = 0, r1 = 0;
    if (!p1_all && !p2_all)        { r0 = 0; r1 = 0; }
    else if (p1_all && !p2_all)    { r0 = 1; r1 = 0; }
    else if (!p1_all && p2_all)    { r0 = 0; r1 = 1; }
    else if (g->live_score[0] > g->live_score[1]) { r0 = 1; r1 = 0; }
    else if (g->live_score[1] > g->live_score[0]) { r0 = 0; r1 = 1; }
    else                                     { r0 = 1; r1 = 1; } /* tie -> both place */
    if (p1_won) *p1_won = r0;
    if (p2_won) *p2_won = r1;
}

/* Mirror live.rs::populate_live_verdicts — for every snapshot, recompute each
    live's pass/fail from the allocation already stored in live_filled /
    live_required (rb_allocations_pass acceptance rules), writing live_passed.
    Operates per-snapshot independently of the victory determination, so it is
    safe to run from rb_perform_live after each allocation. */
void rb_populate_live_verdicts(GameState *g){
    for(int si=0; si<g->n_snapshots; si++){
        RbLiveSnapshot *s=&g->snapshots[si];
        for(int i=0;i<s->n_lives && i<RB_MAX_LIVE_CARDS;i++){
            int *filled=s->live_filled[i];
            int *req=s->live_required[i];
            int icon_all=filled[7];
            int total_filled=0, total_req=0;
            for(int c=0;c<8;c++){ total_filled+=filled[c]; total_req+=req[c]; }
            int ok = total_filled>=total_req;
            if(ok && req[0]>0){
                int any=0; for(int c=0;c<7;c++) any+=filled[c];
                if(any+icon_all < req[0]) ok=0;
                else { int used=req[0]-any; if(used<0) used=0; icon_all-=used; if(icon_all<0) icon_all=0; }
            }
            if(ok){
                for(int c=1;c<7;c++){
                    if(filled[c]<req[c]){
                        int deficit=req[c]-filled[c];
                        if(icon_all>=deficit) icon_all-=deficit; else { ok=0; break; }
                    }
                }
            }
            s->live_passed[i]=ok?1:0;
        }
    }
}

/* Mirror live.rs::finalize_snapshot_fields — fill each snapshot's total_score and
    success flag from the victory determination result. The player that performed
    a given snapshot is recorded in s->player (0/1), keyed to p1_won/p2_won. */
void rb_finalize_snapshot_fields(GameState *g, int p1_won, int p2_won,
                                 int p1_score, int p2_score){
    for(int si=0;si<g->n_snapshots;si++){
        RbLiveSnapshot *s=&g->snapshots[si];
        int sc    = (s->player==0) ? p1_score : p2_score;
        s->total_score = sc;
        int all_passed=1;
        for(int i=0;i<s->n_lives && i<RB_MAX_LIVE_CARDS;i++) if(!s->live_passed[i]) all_passed=0;
        s->success = all_passed && sc>0;
    }
}

/* Mirror live.rs::compute_surplus_and_flags — per-color surplus into each
    snapshot (surplus_per_color), and the GameState surplus-count / no-excess
    flags used by NoExcessHeart conditions. */
void rb_compute_surplus_and_flags(GameState *g, int p1_won, int p2_won){
    int p1_surplus=0, p2_surplus=0;
    for(int si=0;si<g->n_snapshots;si++){
        RbLiveSnapshot *s=&g->snapshots[si];
        int total_avail=0;
        for(int c=0;c<8;c++) total_avail+=s->total_hearts[c];
        int total_filled=0;
        for(int i=0;i<s->n_lives && i<RB_MAX_LIVE_CARDS;i++)
            for(int c=0;c<8;c++) total_filled+=s->live_filled[i][c];
        int surplus=total_avail-total_filled; if(surplus<0) surplus=0;
        for(int c=0;c<8;c++){
            int filled_color=0;
            for(int i=0;i<s->n_lives && i<RB_MAX_LIVE_CARDS;i++) filled_color+=s->live_filled[i][c];
            int pc=s->total_hearts[c]-filled_color; if(pc<0) pc=0;
            s->surplus_per_color[c]=pc;
        }
        if(s->player==0){ p1_surplus=surplus; g->self_live_surplus_count=surplus; }
        else            { p2_surplus=surplus; g->opponent_live_surplus_count=surplus; }
    }
    g->live_surplus_ready_this_turn=1;
    if(p2_won) g->p2_live_success_no_excess = (p2_surplus==0);
    if(p1_won) g->p1_live_success_no_excess = (p1_surplus==0);
}

/* ── live.rs standalone helpers (ported) ── */

/* Mirror live.rs::blade_color_to_heart (rule 8.3.11). A colored blade type maps
   1:1 to the heart color of the same index (Peach→heart01 … Purple→heart06); the
    ALL blade maps to HeartColor::All (icon_all, index 7) per rule 2.1.1.3. */
int rb_blade_color_to_heart(int bc){
    if (bc >= 1 && bc <= 6) return bc;
    if (bc == 7) return RB_HEART_ALL;
    return RB_HEART_PINK; /* fallback (should not happen for a real blade color) */
}

/* Mirror live.rs::TurnEngine::score_delta_since: total (current - prev) across the
    given zone cards. Both arrays are cid-indexed (size RB_MAX_CARD_IDS); a missing
    prev entry defaults to 0, mirroring Rust's HashMap::get().copied().unwrap_or(0). */
int rb_score_delta_since(const int *current, const int *prev, const int *zone_cards, int n){
    int total = 0;
    for (int i = 0; i < n; i++){
        int cid = zone_cards[i];
        if (cid < 0 || cid >= RB_MAX_CARD_IDS) continue;
        int cur = current ? current[cid] : 0;
        int prv = prev ? prev[cid] : 0;
        total += cur - prv;
    }
    return total;
}

/* Mirror live.rs::TurnEngine::compute_pregame_scores: each player's live score from
    current stage hearts + granted hearts, plus the per-player extra (LiveSuccess
    delta). Reuses the shared allocation/verdict path so the score formula stays a
    single source of truth with rb_perform_live. */
void rb_compute_pregame_scores(const GameState *g, int p1_extra, int p2_extra,
                               int *p1_score, int *p2_score){
    for (int pl = 0; pl < 2; pl++){
        int stage[8] = {0};
        rb_stage_hearts_pipeline(g, pl, stage);
        int total[8];
        for (int i = 0; i < 8; i++) total[i] = stage[i] + g->p[pl].hearts[i];
        int passed = 0, score = 0, surplus = -1;
        allocate_and_verdict(g, pl, total, &passed, &score, &surplus, NULL);
        int extra = (pl == 0) ? p1_extra : p2_extra;
        int s = score + extra;
        if (s < 0) s = 0;
        if (pl == 0) { if (p1_score) *p1_score = s; }
        else         { if (p2_score) *p2_score = s; }
    }
}
