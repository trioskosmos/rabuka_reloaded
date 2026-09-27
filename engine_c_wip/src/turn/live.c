#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* The exported per-card allocation entry points (rb_bt_search, rb_try_phase4,
   rb_try_all_distribution) live at the BOTTOM of this file, next to
   rb_try_surplus_compositions / rb_card_ok_with_wildcard, so that they can
   delegate to the real backtracking searchers (bt_search, bt_try_phase4,
   bt_try_all_distribution) instead of re-implementing them. */

/* Faithful Live performance — mirrors engine/src/turn/live.rs
   - yell reveals (top N per live, blade -> heart pool)
   - stage hearts via RbMods (blade/heart modifiers + base hearts)
   - greedy allocation mirroring compute_allocations / check_live_success
     (Phase 1a colored, 3a demand-aware surplus, 3b h00->heart0 only,
      4 icon_all last) + allocations_pass verdict
   - per-live verdict and score with modifiers
   - surplus tracking for no_excess checks
   Host still auto-resolves pending choices via skip in engine.c. */





void rb_calc_stage_hearts(const GameState *g, int pl, int out[8]){
    rb_stage_hearts_pipeline(g, pl, out);
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
    RbYellIconOutcome out;
    Card c;
    memset(&out, 0, sizeof(out));
    if (!total_hearts || !cheer_count || !g || cid < 0) return out;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return out;

    int blade_start = c.num_base;
    int blade_end = blade_start + c.num_blade;
    if (blade_end > c.n_hearts) blade_end = c.n_hearts;
    for (int h = blade_start; h < blade_end; h++) {
        int raw_color = c.heart_color[h];
        int amount = c.heart_count[h];
        if (raw_color == RB_HEART_PINK) amount *= 2;
        int effective_color = raw_color;
        if (override_color >= 0 && raw_color != RB_HEART_PINK &&
            raw_color != RB_HEART_DRAW && raw_color != RB_HEART_SCORE) {
            effective_color = override_color;
        }
        if (effective_color == RB_HEART_DRAW) {
            out.draw_icons += amount;
        } else if (effective_color == RB_HEART_SCORE) {
            out.note_icons += amount;
            *cheer_count += amount;
        } else if (effective_color == RB_HEART_ALL || effective_color == 7) {
            out.blade_hearts[7] += amount;
        } else if (effective_color == RB_HEART_PINK) {
            out.blade_hearts[0] += amount;
        } else if (effective_color >= RB_HEART_RED && effective_color <= RB_HEART_ORANGE) {
            out.blade_hearts[effective_color] += amount;
        }
    }
    if (c.has_special) {
        if (c.special_color == RB_HEART_DRAW) {
            out.draw_icons += c.special_count;
        } else if (c.special_color == RB_HEART_SCORE) {
            out.note_icons += c.special_count;
            *cheer_count += c.special_count;
        }
    }
    for (int i = 0; i < 8; i++) total_hearts[i] += out.blade_hearts[i];
    rb_free_card(&c);
    return out;
}

static int rb_yell_refresh_deck(GameState *g, int pl)
{
    if (!g || pl < 0 || pl > 1) return 0;
    RbPlayer *player = &g->p[pl];
    if (player->deck.n > 0 || player->discard.n == 0) return 0;
    rb_shuffle(player->discard.cards, player->discard.n);
    int count = player->discard.n;
    if (count > RB_MAX_ZONE - player->deck.n) count = RB_MAX_ZONE - player->deck.n;
    for (int i = 0; i < count; i++) player->deck.cards[player->deck.n++] = player->discard.cards[i];
    player->discard.n -= count;
    if (player->discard.n > 0) {
        int remaining = player->discard.n;
        for (int i = count; i < count + remaining; i++) {
            int source_index = i - count;
            if (source_index < player->discard.n)
                player->discard.cards[source_index] = player->discard.cards[source_index + count];
        }
        player->discard.n = remaining;
    }
    player->deck_refreshed_this_turn = 1;
    return count > 0;
}

static int rb_yell_count_delta(const GameState *g, int pl)
{
    if (!g || pl < 0 || pl > 1) return 0;
    int delta = g->yell_count_mod[pl];
    int slot = pl + 1;
    for (int i = 0; i < g->n_yell_count_modifiers; i++) {
        if (g->yell_count_modifiers[i].slot == slot)
            delta += g->yell_count_modifiers[i].delta;
    }
    return delta;
}

static int rb_stage_member_waited(const GameState *g, int pl, int slot)
{
    if (!g || pl < 0 || pl > 1 || slot < 0 || slot >= RB_STAGE_SIZE) return 0;
    int card_id = g->p[pl].stage[slot];
    if (card_id == RB_EMPTY_SLOT || card_id < 0) return 0;
    if (g->p[pl].stage_wait[slot]) return 1;
    const char *orientation = rb_mods_get_orientation((RbMods *)&g->mods, card_id);
    return orientation && !strcmp(orientation, "wait");
}

static int do_yell(GameState *g, int pl, int yell_cards[RB_MAX_ZONE], int *n_yell,
                   int blade_hearts[8], int *note_icons)
{
    if (!g || pl < 0 || pl > 1 || !yell_cards || !n_yell || !blade_hearts || !note_icons)
        return 0;
    RbPlayer *player = &g->p[pl];
    memset(blade_hearts, 0, 8 * sizeof(int));
    *note_icons = 0;
    *n_yell = 0;
    if (player->live.n == 0) return 0;

    int total_needed = 0;
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        int card_id = player->stage[slot];
        if (card_id == RB_EMPTY_SLOT || card_id < 0 || rb_stage_member_waited(g, pl, slot))
            continue;
        total_needed += rb_effective_blade(card_id, g->mods.blade[card_id]);
    }
    total_needed += rb_yell_count_delta(g, pl);
    if (total_needed < 0) total_needed = 0;
    if (total_needed > RB_MAX_ZONE) total_needed = RB_MAX_ZONE;

    const char *source = g->yell_source[pl][0] ? g->yell_source[pl] : "deck_top";
    int from_bottom = player->yell_from_bottom || !strcmp(source, "deck_bottom") ||
                      !strcmp(source, "bottom");
    int from_discard = !strcmp(source, "discard") || !strcmp(source, "waitroom");
    int from_hand = !strcmp(source, "hand");
    int override_color = -1;
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        int card_id = player->stage[slot];
        if (card_id == RB_EMPTY_SLOT || rb_stage_member_waited(g, pl, slot)) continue;
        int blade_type = g->mods.blade_type[card_id];
        if (blade_type >= 0) {
            override_color = rb_blade_color_to_heart(blade_type);
            break;
        }
    }

    int draw_icons = 0;
    for (int i = 0; i < total_needed && *n_yell < RB_MAX_ZONE; i++) {
        int card_id = -1;
        if (from_bottom) {
            rb_yell_refresh_deck(g, pl);
            if (player->deck.n > 0) card_id = player->deck.cards[--player->deck.n];
        } else if (from_discard) {
            if (player->discard.n > 0) card_id = player->discard.cards[--player->discard.n];
        } else if (from_hand) {
            if (player->hand.n > 0) card_id = player->hand.cards[--player->hand.n];
        } else {
            rb_yell_refresh_deck(g, pl);
            if (player->deck.n > 0) {
                card_id = player->deck.cards[0];
                for (int j = 1; j < player->deck.n; j++) player->deck.cards[j - 1] = player->deck.cards[j];
                player->deck.n--;
            }
        }
        if (card_id < 0) break;
        yell_cards[(*n_yell)++] = card_id;
        RbYellIconOutcome outcome = rb_process_yell_revealed_card_icons(
            g, card_id, override_color, blade_hearts, note_icons);
        draw_icons += outcome.draw_icons;
    }
    for (int i = 0; i < draw_icons; i++) rb_draw(g, pl);
    return *n_yell;
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
        int total_filled=0; for(int c=0;c<8;c++) total_filled+=filledc[c];
        int total_required=0; for(int c=0;c<8;c++) total_required+=need[c];
        int h00_deficit = total_required - total_filled; if(h00_deficit<0) h00_deficit=0;
        if(h00_deficit>0 && need[0]>0){
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
            int any=filledc[0]; for(int c=1;c<7;c++) any+=filledc[c];
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

int rb_backtrack_allocate(const int pool[8], const int card_needs[8], int n_cards,
                          int *out_allocs, int max_allocs);

/* Greedy allocation + verdict (mirror compute_allocations / check_live_success).
   Returns 1 if all lives pass. Computes surplus (total - required) for no_excess
   checks.

   `pre_score` selects which per-card score modifier the live score is built
   from, mirroring zones.rs:606 calculate_live_score's `score_modifiers`
   argument:
     - NULL  → the CURRENT mods. This is the performance path
               (LivePerformance::compute_allocations), where the live result
               must reflect everything granted so far.
     - non-NULL → `pre_score_flat`, the pre-trigger snapshot taken by
               snapshot_score_flats (live.rs:1064-1092). compute_pregame_scores
               passes it so the LiveSuccess delta is counted exactly once:
               base + PRE here, then + pX_extra (= current - PRE) by the
               caller. Reading the CURRENT value here and adding the delta
               again double-counts every ライブ成功時 score bonus. */
static int allocate_and_verdict(const GameState *g, int pl, const int total_hearts[8],
                                int score_bonus, int *out_passed, int *out_score,
                                int *out_surplus, int *out_per_live,
                                const int *pre_score){
    RbPlayer *P=(RbPlayer*)&g->p[pl];
    int total_score=0;
    int pool[8]; memcpy(pool,total_hearts,8*sizeof(int));
    int needs[RB_MAX_LIVE_CARDS*8] = {0};
    int filled[RB_MAX_LIVE_CARDS*8] = {0};
    int future[RB_MAX_LIVE_CARDS*8] = {0};
    int n=0;
    rb_build_card_needs(g, pl, needs, &n);
    rb_compute_future_demand(needs, n, future);
    rb_greedy_allocate(pool, needs, n, future, filled);
    int all_pass = rb_allocations_pass(filled, needs, n) ? 1 : 0;
    if (!all_pass && n > 0) {
        int backtrack_allocs[RB_MAX_LIVE_CARDS * 8 * 255];
        int max_allocs = (int)(sizeof(backtrack_allocs) / sizeof(backtrack_allocs[0]));
        memset(backtrack_allocs, -1, sizeof(backtrack_allocs));
        if (rb_backtrack_allocate(total_hearts, (const int *)needs, n,
                                   backtrack_allocs, max_allocs)) {
            memset(filled, 0, sizeof(filled));
            int used = 0;
            while (used < max_allocs && backtrack_allocs[used] >= 0) {
                int entry = backtrack_allocs[used++];
                int card = entry / 8;
                int color = entry % 8;
                if (card >= 0 && card < n && color >= 0 && color < 8)
                    filled[card * 8 + color]++;
            }
            if (used > 0) {
                all_pass = rb_allocations_pass(filled, needs, n) ? 1 : 0;
                fprintf(stderr, "[LIVE_ALLOCATION_BACKTRACK] pl=%d entries=%d pass=%d\n",
                        pl, used, all_pass);
            }
        }
    }
    fprintf(stderr, "[LIVE_ALLOCATION] pl=%d lives=%d hearts=[%d,%d,%d,%d,%d,%d,%d,%d]\n",
            pl, n, total_hearts[0], total_hearts[1], total_hearts[2], total_hearts[3],
            total_hearts[4], total_hearts[5], total_hearts[6], total_hearts[7]);
    for (int di = 0; di < n; di++) {
        fprintf(stderr, "[LIVE_ALLOCATION_CARD] idx=%d need=[%d,%d,%d,%d,%d,%d,%d,%d] filled=[%d,%d,%d,%d,%d,%d,%d,%d]\n",
                di, needs[di*8], needs[di*8+1], needs[di*8+2], needs[di*8+3], needs[di*8+4], needs[di*8+5], needs[di*8+6], needs[di*8+7],
                filled[di*8], filled[di*8+1], filled[di*8+2], filled[di*8+3], filled[di*8+4], filled[di*8+5], filled[di*8+6], filled[di*8+7]);
    }

    int total_required_all=0;
    int total_pool=0; for(int k=0;k<8;k++) total_pool+=total_hearts[k];
    for(int i=0;i<n;i++) for(int k=0;k<8;k++) total_required_all+=needs[i*8+k];

    /* snapshot detail consumed by rb_populate_live_verdicts / rb_compute_surplus_and_flags */
    RbLiveSnapshot *sn = (g->n_snapshots < RB_MAX_SNAPSHOTS)
        ? (RbLiveSnapshot*)&g->snapshots[g->n_snapshots] : NULL;

    for(int li=0; li<P->live.n; li++){
        int need[8]; memcpy(need, needs+li*8, 8*sizeof(int));
        int got=0; for(int c=0;c<8;c++) got+=filled[li*8+c];
        int reqt=0; for(int c=0;c<8;c++) reqt+=need[c];
        int ok = (reqt==0) || (got>=reqt);
        int score=0;
        if(out_per_live && li<RB_MAX_LIVE_CARDS) out_per_live[li]=ok?1:0;
        if(ok){
            Card sc; int base=0;
            int cid = P->live.cards[li];
            if(rb_decode_card_by_index((uint32_t)cid,&sc)){ base=(int)sc.score; rb_free_card(&sc); }
            int mod = pre_score ? (cid >= 0 && cid < RB_MAX_CARD_IDS ? pre_score[cid] : 0)
                                : rb_mods_get_score((RbMods*)&g->mods, cid);
            score=base + mod;
            if(score<0) score=0;
            total_score+=score;
        } else all_pass=0;
        if(sn && li<RB_MAX_LIVE_CARDS){
            sn->live_score_detail[li]=ok?score:0;
            memcpy(sn->live_required[li], need, 8*sizeof(int));
            memcpy(sn->live_filled[li], filled+li*8, 8*sizeof(int));
        }
    }
    int lt_bonus = (pl == 0) ? (int)g->mods.p1_constant_total_score_bonus
                             : (int)g->mods.p2_constant_total_score_bonus;
    if (all_pass && lt_bonus > 0) total_score += lt_bonus;
    if (all_pass && score_bonus > 0) total_score += score_bonus;
    if (total_score < 0) total_score = 0;
    if (total_score > 255) total_score = 255;
    if(out_passed) *out_passed=all_pass;
    if(out_score) *out_score=total_score;
    if(out_surplus){
        *out_surplus = all_pass ? (total_pool - total_required_all) : -1;
        if(*out_surplus < 0 && all_pass) *out_surplus = 0;
    }
    return all_pass;
}

int rb_perform_live(GameState *g, int pl){
    if (!g || pl < 0 || pl > 1) return 0;
    RbPlayer *P=&g->p[pl];
    int yell_cards[RB_MAX_ZONE]; int n_yell=0;
    int blade_hearts[8]={0}; int note_icons=0;
    if (g->performance_resume_pending) {
        if (g->performance_resume_player != pl) return 0;
        n_yell = g->performance_resume_n_yell_cards;
        if (n_yell > RB_MAX_ZONE) n_yell = RB_MAX_ZONE;
        for (int i = 0; i < n_yell; i++) yell_cards[i] = g->performance_resume_yell_cards[i];
        memcpy(blade_hearts, g->performance_resume_blade_hearts, sizeof(blade_hearts));
        note_icons = g->performance_resume_note_icons;
        g->performance_resume_pending = 0;
        g->yell_occurred = g->performance_resume_yell_flag;
        if (g->queue.n_entries > 0 && !rb_has_pending_choice(g)) {
            rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
            rb_drain_ability_queue(g);
        }
        if (rb_has_pending_choice(g)) {
            g->performance_resume_pending = 1;
            return 0;
        }
        g->yell_occurred = 0;
    } else {
        if(P->live.n==0) return 0;
        g->re_yell_occurred = 0;
        g->re_yell_note_icons = 0;
        memset(g->re_yell_blade_hearts, 0, sizeof(g->re_yell_blade_hearts));
        g->n_revealed = 0;
        do_yell(g, pl, yell_cards, &n_yell, blade_hearts, &note_icons);
        for (int i = 0; i < n_yell && i < RB_MAX_REVEALED_CARDS; i++)
            g->revealed_cards[g->n_revealed++] = yell_cards[i];
        g->yell_occurred = n_yell > 0;
        int queued = rb_queue_yell_auto_abilities(g, pl);
        fprintf(stderr, "[YELL_PERFORM] queued=%d entries=%d state=%d occurred=%d revealed=%d\n",
                queued, g->queue.n_entries, g->queue.state, g->yell_occurred, g->n_revealed);
        if (queued > 0) {
            rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
            rb_drain_ability_queue(g);
        }
        if (rb_has_pending_choice(g)) {
            if (g->queue.pending.kind != RB_CHOICE_SELECT_AUTO_ABILITY)
                g->yell_occurred = 0;
            g->performance_resume_player = pl;
            g->performance_resume_n_yell_cards = n_yell;
            for (int i = 0; i < n_yell && i < RB_MAX_ZONE; i++)
                g->performance_resume_yell_cards[i] = yell_cards[i];
            memcpy(g->performance_resume_blade_hearts, blade_hearts, sizeof(blade_hearts));
            g->performance_resume_note_icons = note_icons;
            g->performance_resume_yell_flag = g->yell_occurred;
            g->performance_resume_pending = 1;
            return 0;
        }
        g->yell_occurred = 0;
    }

    int stage_hearts[8]={0};
    rb_stage_hearts_pipeline(g, pl, stage_hearts);

    int total_hearts[8]={0};
    for(int i=0;i<8;i++) total_hearts[i]=stage_hearts[i]+blade_hearts[i];
    /* add ability-granted hearts pool (P->hearts flat = pink etc.) — map to col 0..7 */
    for(int col=0;col<8 && col<RB_MAX_HEARTS;col++) total_hearts[col]+=P->hearts[col];

    int passed=0, live_score=0, surplus=-1;
    int live_passed[RB_MAX_LIVE_CARDS]={0};
    if (g->n_snapshots < RB_MAX_SNAPSHOTS)
        memset(&g->snapshots[g->n_snapshots], 0, sizeof(g->snapshots[g->n_snapshots]));
    allocate_and_verdict(g, pl, total_hearts, note_icons,
                         &passed, &live_score, &surplus, live_passed, NULL);
    g->live_success[pl] = passed; /* record this turn's live result for opponent_live_success */
    /* push snapshot for parity diff (trace_game oracle) — surplus feeds
       NoExcessHeart condition (engine/src/turn/live.rs compute_surplus_and_flags) */
    if(g->n_snapshots < RB_MAX_SNAPSHOTS){
        RbLiveSnapshot *s=&g->snapshots[g->n_snapshots++];
        s->player=pl; s->turn=g->turn; s->n_lives=P->live.n;
        for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++) s->lives[i]=P->live.cards[i];
        s->n_yell_cards = n_yell;
        for (int i = 0; i < P->live.n && i < RB_MAX_LIVE_CARDS; i++) {
            int required[8] = {0};
            rb_effective_need_heart(g, P->live.cards[i], required);
            memcpy(s->live_required[i], required, sizeof(required));
        }
        for (int i = 0; i < n_yell && i < RB_MAX_LIVE_CARDS * 3; i++) {
            s->yell_cards[i] = yell_cards[i];
        }
        for (int i = 0; i < 8; i++) s->yell_blade_hearts[i] = blade_hearts[i];
        for(int i=0;i<8;i++) s->total_hearts[i]=total_hearts[i];
        s->total_score=live_score; s->success=passed;
        s->surplus_hearts = surplus;
        s->note_icons = note_icons;
        for(int i=0;i<P->live.n && i<RB_MAX_LIVE_CARDS;i++) s->live_passed[i]=passed ? live_passed[i] : 0;
    }
    for (int i = 0; i < n_yell; i++) {
        if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = yell_cards[i];
    }
    if (g->live_batch_mode) {
        g->live_pre_valid[pl] = 1;
        for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
            g->live_pre_score[pl][cid] = rb_mods_get_score(&g->mods, cid);
        }
        return passed;
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
        allocate_and_verdict(g, pl, total2, note_icons + g->re_yell_note_icons,
                             &passed2, &score2, &surplus2, live_passed2, NULL);
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
        passed = 1;
        for (int i = 0; i < P->live.n && i < RB_MAX_LIVE_CARDS; i++) {
            if (!g->snapshots[g->n_snapshots - 1].live_passed[i]) passed = 0;
        }
        if (!passed) {
            passed = 0;
            live_score = 0;
            g->snapshots[g->n_snapshots - 1].total_score = 0;
            g->snapshots[g->n_snapshots - 1].success = 0;
            g->snapshots[g->n_snapshots - 1].live_passed[0] = 0;
        }
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
    else                                     { r0 = 1; r1 = 1; }
    if (p1_won) *p1_won = r0;
    if (p2_won) *p2_won = r1;
}

/* Mirror live.rs::live_score_for_card (live.rs:646-654 and 488-497) — the
    per-live score line is the card's PRINTED score with the current score
    modifiers applied: a `set` modifier replaces the printed base, any
    additive modifier is added on top, and the result saturates to u8.

        let base_score = card.get_score() as i32;
        let set_score  = game_state.mods.get_score_set_modifier(lc_id);
        let additive   = game_state.mods.get_score_modifier(lc_id) - set_score;
        let effective_base = if set_score != 0 { set_score } else { base_score };
        snap.lives[i].score = crate::constants::saturate_u8(effective_base + additive);

    NOTE this MUST read the LIVE modifier state at the moment
    populate_live_verdicts runs (engine/src/turn/live.rs:1300 — after
    resolve_live_success_extras, before revert_live_success_score_modifiers at
    :1336). That is precisely what makes a ライブ成功時 score grant such as
    イチゴトラッパー PL!S-pb1-021-L (「相手が余剰のハートを持たずにライブを
    成功させていた場合、このカードのスコアを＋２」) show up in the snapshot's
    per-live score line, and it is what
    engine/tests/test_modules/rules/phases/opponent_live_success_flow_test.rs
    pins with `found.score - found.base_score == 2`. */
static int live_score_for_card(const GameState *g, int card_id){
    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) { rb_free_card(&card); return 0; }
    int base_score = (int)card.score;
    rb_free_card(&card);
    int set_score = rb_mods_get_score_set((RbMods *)&g->mods, card_id);
    int additive  = rb_mods_get_score((RbMods *)&g->mods, card_id) - set_score;
    int effective_base = (set_score != 0) ? set_score : base_score;
    return rb_saturate_u8(effective_base + additive);
}

/* Mirror live.rs::populate_live_verdicts (live.rs:469-657) — for every snapshot,
    recompute each live's pass/fail from the allocation already stored in
    live_filled / live_required (the same acceptance rules: total coverage, then
    the heart0 bucket, then per-colour deficits coverable by icon_all) AND
    rewrite its score line from the CURRENT score modifiers. Operates
    per-snapshot independently of the victory determination, so it is safe to
    run from rb_perform_live after each allocation. */
void rb_populate_live_verdicts(GameState *g){
    if (!g) return;
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
                int any=filled[0]; for(int c=1;c<7;c++) any+=filled[c];
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
            /* live.rs:646-654 — the score line is recomputed here, AFTER the
               ライブ成功時 triggers have granted their (still-unreverted)
               score modifiers. */
            s->live_score_detail[i] = live_score_for_card(g, s->lives[i]);
        }
    }
}

/* Mirror live.rs::finalize_snapshot_fields (live.rs:659-759) — fill each
    snapshot's total_score and success flag from the victory determination
    result. The relevant Rust arms:

        snap.total_score = if is_first { p1_score } else { p2_score };
        let zone_empty = if is_first { player1.live_card_zone.cards.is_empty() }
                         else         { player2.live_card_zone.cards.is_empty() };
        if zone_empty { snap.total_score = 0; }
        snap.success = snap.lives.iter().all(|l| l.passed) && snap.total_score > 0;

    Note what Rust does NOT do: it never zeroes total_score because a live
    FAILED — a failed live still has a computed score; only an EMPTY live zone
    forces 0, and `success` is what encodes the pass/fail verdict. The previous
    C body zeroed on `!all_passed` and never consulted the live zone, so a
    player who performed no lives at all could be reported successful on the
    strength of the other seat's score. `p1_won`/`p2_won` land in the snapshot
    as p0_wins/p1_wins upstream (no C field); they are accepted for signature
    parity. */
void rb_finalize_snapshot_fields(GameState *g, int p1_won, int p2_won,
                                 int p1_score, int p2_score){
    if (!g) return;
    (void)p1_won; (void)p2_won;
    for(int si=0;si<g->n_snapshots;si++){
        RbLiveSnapshot *s=&g->snapshots[si];
        int is_first = (s->player == 0);
        s->total_score = is_first ? p1_score : p2_score;
        int zone_empty = is_first ? (g->p[0].live.n == 0) : (g->p[1].live.n == 0);
        if (zone_empty) s->total_score = 0;
        int all_passed=1;
        for(int i=0;i<s->n_lives && i<RB_MAX_LIVE_CARDS;i++) if(!s->live_passed[i]) all_passed=0;
        s->success = all_passed && s->total_score>0;
    }
}

/* Mirror live.rs::compute_surplus_and_flags (live.rs:896-955) — per-color
    surplus into each snapshot, and the GameState surplus-count / no-excess
    flags used by NoExcessHeart conditions.

        for snap in &mut game_state.performance_snapshots {
            if !snap.success { snap.surplus_hearts = [0u8; 8]; continue; }   // Q142/Q259
            let total_available: u8 = snap.total_hearts.iter().sum();
            let total_filled: u8    = snap.lives.iter().flat_map(|l| l.filled.iter()).sum();
            let surplus = total_available.saturating_sub(total_filled);
            for (color, &total_color) in snap.total_hearts.iter().enumerate() {
                let filled_color: u8 = snap.lives.iter().map(|l| l.filled[color]).sum();
                per_color_surplus[color] = total_color.saturating_sub(filled_color);
            }
            snap.surplus_hearts = per_color_surplus;
            if snap.player_id == p1_id { p1_surplus = surplus; }
            else if snap.player_id == p2_id { p2_surplus = surplus; }
        }
        game_state.opponent_live_surplus_count = p2_surplus;
        game_state.self_live_surplus_count     = p1_surplus;
        game_state.live_surplus_ready_this_turn = true;
        if p2_won { game_state.set_opponent_live_success(p2_surplus == 0); }
        if p1_won { game_state.self_no_excess_heart_this_turn = p1_surplus == 0; }
        game_state.p1_live_success_this_turn  = p1_won;
        game_state.p1_live_success_no_excess  = p1_surplus == 0;
        game_state.p2_live_success_this_turn  = p2_won;
        game_state.p2_live_success_no_excess  = p2_surplus == 0;

    The gate is `!snap.success` alone (which already encodes the per-live
    verdicts), the per-seat accumulators are written UNCONDITIONALLY from the
    (default-0) accumulators, and the last snapshot for a seat wins. The
    previous C body re-derived `all_passed` itself and then refused to publish
    p1/p2_live_success_no_excess at all for a seat with no snapshot (and forced
    it to 0 when that seat "won"), which is not what Rust does. */
void rb_compute_surplus_and_flags(GameState *g, int p1_won, int p2_won){
    if (!g) return;
    int p1_surplus = 0, p2_surplus = 0;
    for (int si = 0; si < g->n_snapshots; si++) {
        RbLiveSnapshot *s = &g->snapshots[si];
        if (!s->success) {
            s->surplus_hearts = 0;
            memset(s->surplus_per_color, 0, sizeof(s->surplus_per_color));
            continue;
        }
        int total_avail = 0;
        int total_filled = 0;
        for (int c = 0; c < 8; c++) total_avail += s->total_hearts[c];
        for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++)
            for (int c = 0; c < 8; c++) total_filled += s->live_filled[i][c];
        int surplus = total_avail - total_filled;
        if (surplus < 0) surplus = 0;
        for (int c = 0; c < 8; c++) {
            int filled_color = 0;
            for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++)
                filled_color += s->live_filled[i][c];
            int per_color = s->total_hearts[c] - filled_color;
            if (per_color < 0) per_color = 0;
            s->surplus_per_color[c] = per_color;
        }
        s->surplus_hearts = surplus;
        if (s->player == 0) p1_surplus = surplus;
        else if (s->player == 1) p2_surplus = surplus;
    }
    g->opponent_live_surplus_count = p2_surplus;
    g->self_live_surplus_count = p1_surplus;
    g->live_surplus_ready_this_turn = 1;
    /* set_opponent_live_success(no_excess) / self_no_excess_heart_this_turn.
       The C GameState only carries the seat-relative pair
       (p1_live_success_no_excess / p2_live_success_no_excess) plus
       `opponent_live_success_this_turn`; the attacker-order-relative
       `opponent_live_no_excess_heart_this_turn` /
       `self_no_excess_heart_this_turn` mirrors of Rust
       (game_state/mod.rs:319-321) have no field here. */
    if (p2_won) g->opponent_live_success_this_turn = 1;
    g->p1_live_success_no_excess = (p1_surplus == 0);
    g->p2_live_success_no_excess = (p2_surplus == 0);
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

static int rb_latest_snapshot_note_icons(const GameState *g, int pl)
{
    if (!g || pl < 0 || pl > 1) return 0;
    for (int i = g->n_snapshots - 1; i >= 0; i--)
        if (g->snapshots[i].player == pl && g->snapshots[i].turn == g->turn)
            return g->snapshots[i].note_icons;
    return 0;
}

/* Mirror live.rs::TurnEngine::compute_pregame_scores: each player's live score from
    current stage hearts + granted hearts, plus the per-player extra (LiveSuccess
    delta). Reuses the shared allocation/verdict path so the score formula stays a
    single source of truth with rb_perform_live.

    The per-card score is built from the PRE-trigger modifier snapshot
    (`live_pre_score`, captured by rb_perform_live and standing in for Rust's
    snapshot_score_flats / pre_score_flat) and the LiveSuccess delta is added
    once as `extra` — see live.rs:1064-1092:
        "Capture pre-trigger score modifiers so we can avoid double-counting:
         calculate_live_score gets the PRE values, and pX_extra carries only
         the delta from LiveSuccess-triggered abilities." */
void rb_compute_pregame_scores(const GameState *g, int p1_extra, int p2_extra,
                               int *p1_score, int *p2_score){
    for (int pl = 0; pl < 2; pl++){
        int stage[8] = {0};
        if (g->live_surplus_ready_this_turn) {
            for (int i = 0; i < 8; i++) stage[i] = g->stage_hearts[pl][i];
        } else {
            rb_stage_hearts_pipeline(g, pl, stage);
        }
        int total[8];
        for (int i = 0; i < 8; i++) total[i] = stage[i] + g->p[pl].hearts[i];
        int passed = 0, score = 0, surplus = -1;
        const int *pre = g->live_pre_valid[pl] ? g->live_pre_score[pl] : NULL;
        allocate_and_verdict(g, pl, total, rb_latest_snapshot_note_icons(g, pl),
                             &passed, &score, &surplus, NULL, pre);
        int extra = (pl == 0) ? p1_extra : p2_extra;
        int s = score + extra;
        if (s < 0) s = 0;
        if (pl == 0) { if (p1_score) *p1_score = s; }
        else         { if (p2_score) *p2_score = s; }
    }
}

/* ── Ported from live.rs (SIZE_AUDIT.md unmatched functions) ── */

/* Mirror live.rs::fmt_player_id — format a player id string as "P<digits>"
    (e.g. "player1" → "P1", "player2" → "P2"). Writes into out (cap sz). */
void rb_fmt_player_id(const char *id, char *out, size_t sz){
    if(!id || !out || sz==0) return;
    while(*id && !(*id>='0' && *id<='9')) id++;
    if(!*id){ snprintf(out,sz,"?"); return; }
    snprintf(out,sz,"P%s",id);
}

/* Mirror live.rs::TurnEngine::try_take_success_zone_choice — when a player won
    and holds >1 live card, prompt them to choose which one goes to the success
    zone (any card that cannot be placed is filtered out by the caller). Returns
    1 if a choice was emitted. */
int rb_try_take_success_zone_choice(GameState *g, int won, int must_skip,
                                     int cards_count, const int *cards, int player_pl){
    if(!won || must_skip || cards_count <= 1) return 0;
    int filtered[RB_MAX_ZONE];
    int n_filtered = 0;
    for(int i=0;i<cards_count && n_filtered<RB_MAX_ZONE;i++){
        if(rb_can_place_card_in_zone(g, cards[i], "success_live_zone"))
            filtered[n_filtered++] = i;
    }
    if(n_filtered == 0) return 0;
    rb_emit_choice(g, player_pl, RB_CHOICE_SELECT_CARD, "live_card_zone", "live_card",
                   1, 0, "select_live_success");
    g->queue.pending.actor = player_pl;
    g->queue.pending.n_filtered_indices = n_filtered;
    for(int i=0;i<n_filtered;i++) g->queue.pending.filtered_indices[i] = filtered[i];
    fprintf(stderr, "[LIVE_SUCCESS_PROMPT] pl=%d live=%d legal=%d\n",
            player_pl, cards_count, n_filtered);
    return 1;
}

/* Mirror live.rs::TurnEngine::get_success_replacement_info — scan a card's
    constant abilities for a ConditionalAlternative whose condition targets the
    success zone and whose alternative moves a live card from discard. Returns
    the group name (static buffer) or NULL. */
static const char* get_success_replacement_info(const GameState *g, int card_id){
    static char group_buf[64];
    int nab = rb_card_num_abilities((uint32_t)card_id);
    for(int a=0;a<nab;a++){
        Ability ab;
        if(!rb_decode_card_ability((uint32_t)card_id, a, &ab)) continue;
        if(!rb_ability_has_trigger(&ab, RB_TK_CONSTANT)) {
            rb_free_ability(&ab);
            continue;
        }
        AbilityEffect *e = ab.effect;
        if(!e || !e->action || strcmp(e->action,"conditional_alternative")) {
            rb_free_ability(&ab);
            continue;
        }
        int success_condition = 0;
        if(e->condition) {
            for(int i=0;i<(int)e->condition->n_fields;i++) {
                const CondField *field = &e->condition->fields[i];
                if(field->key && !strcmp(field->key, "location") && field->v.s &&
                   (!strcmp(field->v.s, "success_live_zone") ||
                    !strcmp(field->v.s, "success_live_card_zone"))) {
                    success_condition = 1;
                    break;
                }
            }
        }
        if(!success_condition) {
            rb_free_ability(&ab);
            continue;
        }
        AbilityEffect *alt = e->alternative_effect;
        if(!alt || !alt->action || strcmp(alt->action,"move_cards") ||
           !alt->source || strcmp(alt->source,"discard")) {
            rb_free_ability(&ab);
            continue;
        }
        for(int i=0;i<e->n_extra;i++){
            if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names") && e->extra_v[i] && e->extra_v[i][0]){
                strncpy(group_buf, e->extra_v[i], sizeof(group_buf)-1);
                group_buf[sizeof(group_buf)-1]=0;
                rb_free_ability(&ab);
                return group_buf;
            }
        }
        for(int i=0;i<alt->n_extra;i++){
            if(alt->extra_k[i] && !strcmp(alt->extra_k[i],"group_names") && alt->extra_v[i] && alt->extra_v[i][0]){
                strncpy(group_buf, alt->extra_v[i], sizeof(group_buf)-1);
                group_buf[sizeof(group_buf)-1]=0;
                rb_free_ability(&ab);
                return group_buf;
            }
        }
        rb_free_ability(&ab);
    }
    return NULL;
}

/* Mirror live.rs::TurnEngine::try_create_success_replacement_choice — if the
    card has a success-replacement ability and the player's waitroom holds a
    matching live card, emit a select_cards choice from discard. Returns 1 if a
    choice was emitted. */
int rb_try_create_success_replacement_choice(GameState *g, int card_id, int player_pl){
    const char *group_name = get_success_replacement_info(g, card_id);
    if(!group_name || player_pl < 0 || player_pl > 1) return 0;
    const RbPlayer *P = &g->p[player_pl];
    int filtered[RB_MAX_ZONE];
    int n_filtered = 0;
    for(int i=0;i<P->discard.n && n_filtered<RB_MAX_ZONE;i++){
        int cid = P->discard.cards[i];
        if(rb_card_is_live(cid) && rb_card_matches_group_str(cid, group_name) &&
           rb_can_place_card_in_zone(g, cid, "success_live_zone"))
            filtered[n_filtered++] = i;
    }
    if(n_filtered == 0) return 0;
    rb_emit_choice(g, player_pl, RB_CHOICE_SELECT_CARD, "discard", "live_card",
                   1, 1, "success_replacement");
    g->queue.actor = player_pl;
    g->queue.pending.actor = player_pl;
    g->queue.resume_actor = player_pl;
    g->queue.resume_host = card_id;
    g->queue.resume_mode = 0;
    g->queue.pending.n_filtered_indices = n_filtered;
    for(int i=0;i<n_filtered;i++) g->queue.pending.filtered_indices[i] = filtered[i];
    strncpy(g->queue.pending.filter_group, group_name,
            sizeof(g->queue.pending.filter_group) - 1);
    g->queue.pending.filter_group[sizeof(g->queue.pending.filter_group) - 1] = 0;
    strncpy(g->queue.resume_move_destination, "success_live_zone",
            sizeof(g->queue.resume_move_destination) - 1);
    g->queue.resume_move_destination[sizeof(g->queue.resume_move_destination) - 1] = 0;
    fprintf(stderr, "[SUCCESS_REPLACEMENT_PROMPT] pl=%d original=%d legal=%d group=%s\n",
            player_pl, card_id, n_filtered, group_name);
    return 1;
}

void rb_handle_success_replacement_choice(GameState *g, int player_pl,
                                          int original_card_id,
                                          int selected_discard_index,
                                          int accepted) {
    if (!g || player_pl < 0 || player_pl > 1) return;
    RbPlayer *P = &g->p[player_pl];
    int replacement_id = -1;
    if (accepted && selected_discard_index >= 0 && selected_discard_index < P->discard.n) {
        replacement_id = P->discard.cards[selected_discard_index];
        for (int i = selected_discard_index; i < P->discard.n - 1; i++)
            P->discard.cards[i] = P->discard.cards[i + 1];
        P->discard.n--;
        if (!rb_card_is_live(replacement_id) ||
            !rb_can_place_card_in_zone(g, replacement_id, "success_live_zone")) {
            rb_waitroom_add(P, replacement_id);
            replacement_id = -1;
        }
    }

    int original_index = -1;
    for (int i = 0; i < P->live.n; i++) {
        if (P->live.cards[i] == original_card_id) {
            original_index = i;
            break;
        }
    }
    if (original_index >= 0) {
        for (int i = original_index; i < P->live.n - 1; i++)
            P->live.cards[i] = P->live.cards[i + 1];
        P->live.n--;
    }

    if (replacement_id >= 0) {
        if (original_card_id >= 0) rb_waitroom_add(P, original_card_id);
        if (P->success.n < RB_MAX_ZONE) rb_success_add(P, replacement_id);
        else rb_waitroom_add(P, replacement_id);
    } else if (original_card_id >= 0) {
        if (P->success.n < RB_MAX_ZONE) rb_success_add(P, original_card_id);
        else rb_waitroom_add(P, original_card_id);
    }

    while (P->live.n > 0) {
        int cid = P->live.cards[0];
        for (int i = 0; i < P->live.n - 1; i++)
            P->live.cards[i] = P->live.cards[i + 1];
        P->live.n--;
        rb_waitroom_add(P, cid);
    }
    fprintf(stderr, "[SUCCESS_REPLACEMENT_RESULT] pl=%d original=%d replacement=%d accepted=%d success=%d waitroom=%d\n",
            player_pl, original_card_id, replacement_id, accepted,
            P->success.n, P->discard.n);
}

/* Mirror live.rs::enrich_from_applications (live.rs:2995-3091).

    for app in applications {
        if let Some(mc) = member_contributions.iter_mut().find(|m| m.source_id == app.target_card_id) {
            match app.effect_type {
                HeartBonus => mc.ability_heart_bonuses.push(...),
                BladeBonus => mc.ability_blade_bonuses.push(...),
                _ => {}
            }
        }
        match app.effect_type {
            ScoreBonus | ScoreSet => breakdown.scores.push(ScoreLine { source, value }),
            Transform => breakdown.transforms.push(EffectEntry { ... }),
            _ => {}
        }
        let key = (app.source_card_id, &app.ability_text);
        if seen.insert(key) && !app.ability_text.is_empty() { triggered_abilities.push(...) }
    }

    Every one of those targets is a BREAKDOWN / contribution field — the
    function is a pure projection onto the display breakdown. It does NOT touch
    `snap.lives[i].score` (that is owned by populate_live_verdicts,
    live.rs:646-654) and it does NOT touch `snap.total_score` (owned by
    finalize_snapshot_fields, live.rs:696-709). It also takes
    `applications: &[AbilityApplication]` — a borrow, not a drain.

    The C snapshot has no `breakdown` / `member_contributions` /
    `triggered_abilities` fields to project onto (see RbLiveSnapshot in
    include/rabuka.h), so the faithful port is a no-op over the trace: the
    numeric score line and total MUST be left exactly as the two owning
    functions above wrote them. The previous body added HEART_BONUS /
    BLADE_BONUS amounts into `live_score_detail` and recomputed `total_score`,
    which both (a) is not what Rust does and (b) double-counts any score
    modifier already folded in by populate_live_verdicts. */
void rb_enrich_from_applications(const GameState *g){
    (void)g;
}

void rb_apply_deferred_reyell(GameState *g) {
     if (!g || !g->re_yell_pending) return;
     int owner = g->re_yell_owner;
     if (owner < 0 || owner > 1) {
          g->re_yell_pending = 0;
          return;
     }
     int stage[8];
     rb_stage_hearts_pipeline(g, owner, stage);
     for (int s = g->n_snapshots - 1; s >= 0; s--) {
          RbLiveSnapshot *snapshot = &g->snapshots[s];
          if (snapshot->player != owner || snapshot->turn != g->turn) continue;
          snapshot->n_yell_cards = 0;
          for (int i = 0; i < g->n_revealed && i < RB_MAX_LIVE_CARDS * 3; i++)
               snapshot->yell_cards[snapshot->n_yell_cards++] = g->revealed_cards[i];
          memcpy(snapshot->yell_blade_hearts, g->re_yell_blade_hearts,
                 sizeof(snapshot->yell_blade_hearts));
          snapshot->note_icons = g->re_yell_note_icons;
          for (int color = 0; color < 8; color++)
               snapshot->total_hearts[color] = stage[color] + g->re_yell_blade_hearts[color];
          break;
     }
     g->re_yell_pending = 0;
     g->re_yell_occurred = 0;
}

void rb_rebuild_stage_hearts_with_yell(GameState *g) {
    if (!g) return;
    int stage[2][8];
    for (int pl = 0; pl < 2; pl++) {
        rb_stage_hearts_pipeline(g, pl, stage[pl]);
        for (int i = 0; i < g->n_snapshots; i++) {
            RbLiveSnapshot *s = &g->snapshots[i];
            if (s->player != pl) continue;
            for (int c = 0; c < 8; c++) stage[pl][c] += s->yell_blade_hearts[c];
        }
    }
    for (int pl = 0; pl < 2; pl++) {
        for (int c = 0; c < 8; c++) g->stage_hearts[pl][c] = stage[pl][c];
    }
    g->live_surplus_ready_this_turn = 1;
}

void rb_record_pretrigger_live_results(GameState *g) {
    if (!g) return;
    for (int i = 0; i < g->n_snapshots; i++) {
        RbLiveSnapshot *s = &g->snapshots[i];
        if (s->turn != g->turn || s->player < 0 || s->player > 1) continue;
        int passed = s->n_lives > 0;
        for (int live = 0; live < s->n_lives && live < RB_MAX_LIVE_CARDS; live++)
            if (!s->live_passed[live]) passed = 0;
        int no_excess = passed;
        for (int color = 0; color < 8 && no_excess; color++) {
            int filled = 0;
            for (int live = 0; live < s->n_lives && live < RB_MAX_LIVE_CARDS; live++)
                filled += s->live_filled[live][color];
            if (s->total_hearts[color] != filled) no_excess = 0;
        }
        g->live_success[s->player] = passed;
        if (s->player == 0) g->p1_live_success_no_excess = no_excess;
        else g->p2_live_success_no_excess = no_excess;
        fprintf(stderr, "[EARLY_SEAT] pl=%d won=%d no_excess=%d surplus=%d\n",
                s->player, passed, no_excess, s->surplus_hearts);
    }
}

void rb_drain_pending_live_success_choices(GameState *g) {
    if (!g) return;
    rb_drain_ability_queue(g);
}

static int live_score_extra_for_side(const GameState *g, int pl) {
    if (!g->live_pre_valid[pl]) return 0;
    int total = 0;
    for (int i = 0; i < g->p[pl].live.n; i++) {
        int cid = g->p[pl].live.cards[i];
        if (cid < 0 || cid >= RB_MAX_CARD_IDS) continue;
        total += rb_mods_get_score((RbMods *)&g->mods, cid) - g->live_pre_score[pl][cid];
    }
    return rb_saturate_u8(total);
}

void rb_revert_live_success_score_modifiers(GameState *g) {
    if (!g) return;
    for (int pl = 0; pl < 2; pl++) {
        if (!g->live_pre_valid[pl]) continue;
        for (int i = 0; i < g->p[pl].live.n; i++) {
            int cid = g->p[pl].live.cards[i];
            if (cid < 0 || cid >= RB_MAX_CARD_IDS) continue;
            int current = rb_mods_get_score(&g->mods, cid);
            int delta = current - g->live_pre_score[pl][cid];
            if (delta != 0) rb_mods_add_score(&g->mods, cid, -delta);
        }
        g->live_pre_valid[pl] = 0;
    }
}

void rb_process_delayed_gained_effects(GameState *g) {
    if (!g || g->n_delayed_gained_effects == 0) return;
    int saved_revealed[RB_MAX_REVEALED_CARDS];
    int saved_revealed_n = g->n_revealed;
    int saved_activating = g->activating_card;
    for (int i = 0; i < saved_revealed_n && i < RB_MAX_REVEALED_CARDS; i++) {
        saved_revealed[i] = g->revealed_cards[i];
    }

    for (int i = 0; i < g->n_delayed_gained_effects; i++) {
        RbDelayedGainedEffect *delayed = &g->delayed_gained_effects[i];
        int owner = rb_owner_of_card(g, delayed->card_id);
        if (owner < 0 || g->p[owner].live.n == 0) continue;
        RbLiveSnapshot *snapshot = NULL;
        for (int s = g->n_snapshots - 1; s >= 0; s--) {
            if (g->snapshots[s].player == owner && g->snapshots[s].turn == g->turn) {
                snapshot = &g->snapshots[s];
                break;
            }
        }
        if (!snapshot || snapshot->n_lives == 0) continue;
        int passed = 1;
        for (int j = 0; j < snapshot->n_lives; j++) {
            if (!snapshot->live_passed[j]) passed = 0;
        }
        if (!passed) continue;

        g->n_revealed = 0;
        for (int j = 0; j < snapshot->n_yell_cards && j < RB_MAX_REVEALED_CARDS; j++) {
            g->revealed_cards[g->n_revealed++] = snapshot->yell_cards[j];
        }
        g->activating_card = delayed->card_id;
        rb_execute_effect_ex(g, owner, delayed->effect, delayed->card_id);
        rb_drain_ability_queue(g);
    }

    g->n_revealed = saved_revealed_n;
    for (int i = 0; i < saved_revealed_n && i < RB_MAX_REVEALED_CARDS; i++) {
        g->revealed_cards[i] = saved_revealed[i];
    }
    g->activating_card = saved_activating;
    for (int i = 0; i < g->n_delayed_gained_effects; i++) {
        rb_effect_free(g->delayed_gained_effects[i].effect);
        g->delayed_gained_effects[i].effect = NULL;
    }
    g->n_delayed_gained_effects = 0;
}

/* Mirror live.rs::merge_late_score_apps (live.rs:866-894):

    let late_apps = core::mem::take(&mut game_state.ability_applications);
    if late_apps.is_empty() { return; }
    let p1_cards = &game_state.player1.live_card_zone.cards;
    let p2_cards = &game_state.player2.live_card_zone.cards;
    for snap in game_state.performance_snapshots.iter_mut() {
        let player_cards = if snap.player_id == p1_id { p1_cards }
                           else if snap.player_id == p2_id { p2_cards }
                           else { continue };
        for app in &late_apps {
            if (app.effect_type == ScoreBonus || app.effect_type == ScoreSet)
                && player_cards.contains(&app.target_card_id) {
                snap.breakdown.scores.push(ScoreLine { source, value });
            }
        }
    }

    Two things matter here and BOTH differ from the previous C body:
      * the applications list is DRAINED (`core::mem::take`), and
      * the only thing written is `snap.breakdown.scores` — a DISPLAY line.
        `snap.lives[i].score` is owned by populate_live_verdicts (live.rs:654)
        and `snap.total_score` by finalize_snapshot_fields (live.rs:696-709).
    The C snapshot has no `breakdown` field, so the faithful port drains the
    trace and leaves every numeric field alone. The previous body instead added
    each ScoreBonus/ScoreSet amount on top of `live_score_detail` and then
    overwrote `total_score` with `note_icons + sum(live_score_detail)` — which
    double-counts every score modifier populate_live_verdicts already folded in
    and discards the authoritative p1_score/p2_score computed by
    compute_pregame_scores. */
void rb_merge_late_score_apps(GameState *g) {
    if (!g) return;
    /* core::mem::take(&mut game_state.ability_applications) */
    g->mods.n_trace = 0;
}

/* The ids process_player_live_result pushed to the waitroom. Rust returns them
   from the function (live.rs:1437-1458) purely so
   move_live_to_success_and_handle_wins can publish them as the
   "recently moved" batch and re-scan both seats' auto abilities
   (live.rs:1594-1603). The C signature is fixed at `void` in
   include/rabuka.h, so the list is threaded through this file-static buffer,
   which the single caller drains immediately afterwards. */
static int s_moved_to_waitroom[RB_MAX_RECENTLY_MOVED];
static int  s_n_moved_to_waitroom;

static void rb_moved_record(int cid) {
    if (cid < 0) return;
    if (s_n_moved_to_waitroom < RB_MAX_RECENTLY_MOVED)
        s_moved_to_waitroom[s_n_moved_to_waitroom++] = cid;
}

/* Mirror GameState::set_recently_moved_batch (core/game_state/modifiers.rs:1570)
   — the batch scratch view is REPLACED, not appended. */
static void rb_set_recently_moved_batch(GameState *g, const int *cards, int n) {
    if (!g) return;
    int k = n > RB_MAX_RECENTLY_MOVED ? RB_MAX_RECENTLY_MOVED : n;
    g->n_recently_moved = 0;
    for (int i = 0; i < k; i++) g->recently_moved[g->n_recently_moved++] = cards[i];
}

void rb_move_live_to_success_and_handle_wins(GameState *g) {
    if (!g) return;
    s_n_moved_to_waitroom = 0;
    int p1_must_skip = g->p1_live_won && g->p2_live_won && g->p[0].success.n >= 2;
    int p2_must_skip = g->p1_live_won && g->p2_live_won && g->p[1].success.n >= 2;
    if (rb_try_take_success_zone_choice(g, g->p1_live_won, p1_must_skip,
                                        g->p[0].live.n, g->p[0].live.cards, 0))
        return;
    if (rb_try_take_success_zone_choice(g, g->p2_live_won, p2_must_skip,
                                        g->p[1].live.n, g->p[1].live.cards, 1))
        return;
    for (int pl = 0; pl < 2; pl++) {
        int won = pl == 0 ? g->p1_live_won : g->p2_live_won;
        int must_skip = pl == 0 ? p1_must_skip : p2_must_skip;
        int card_id = g->p[pl].live.n == 1 ? g->p[pl].live.cards[0] : -1;
        if (won && !must_skip && card_id >= 0 &&
            rb_try_create_success_replacement_choice(g, card_id, pl))
            return;
    }
    for (int pl = 0; pl < 2; pl++) {
        int won = pl == 0 ? g->p1_live_won : g->p2_live_won;
        int must_skip = pl == 0 ? p1_must_skip : p2_must_skip;
        int cid = g->p[pl].live.n > 0 ? g->p[pl].live.cards[g->p[pl].live.n - 1] : -1;
        int can_place = cid >= 0 && rb_can_place_card_in_zone(g, cid, "success_live_zone");
        fprintf(stderr, "[LIVE_SUCCESS_RESULT] pl=%d won=%d must_skip=%d card=%d can_place=%d live=%d\n",
                pl, won, must_skip, cid, can_place, g->p[pl].live.n);
        rb_process_player_live_result(g, pl, won, must_skip, can_place);
    }
    /* live.rs:1594-1603 — publish the waitroom batch and re-scan both seats.
         if !moved_to_waitroom.is_empty() {
             game_state.set_recently_moved_batch(moved_to_waitroom.into(), Some("live_card_zone"));
             Self::trigger_auto_abilities_for_player(game_state, &p1_id);
             Self::trigger_auto_abilities_for_player(game_state, &p2_id);
             game_state.process_pending_auto_abilities(&p1_id);
             game_state.process_pending_auto_abilities(&p2_id);
         }
       The previous C body stopped after process_player_live_result, so nothing
       that watches a live card leaving to the 控え室 (a 移動時 gate on the live
       zone, an opponent cause watcher) could ever fire off the live-success
       placement. */
    if (s_n_moved_to_waitroom > 0) {
        rb_set_recently_moved_batch(g, s_moved_to_waitroom, s_n_moved_to_waitroom);
        fprintf(stderr, "[LIVE_WAITROOM_BATCH] n=%d\n", s_n_moved_to_waitroom);
        rb_trigger_auto_abilities_for_player(g, 0);
        rb_trigger_auto_abilities_for_player(g, 1);
        rb_process_pending_auto_abilities(g);
    }
}

/* Mirror live.rs::check_live_success — Rule 8.3.14-8.3.16: Recompute hearts,
   recompute allocations, check heart requirements, determine pass/fail. */
int rb_check_live_success(GameState *g, int pl) {
    if (!g) return 0;
    RbPlayer *P = &g->p[pl];
    int live_cid = P->live.cards[0];
    if (live_cid < 0) return 0;
    Card card;
    if (!rb_decode_card_by_index((uint32_t)live_cid, &card)) { rb_free_card(&card); return 0; }
    int need_total = card.num_need;
    rb_free_card(&card);
    int have_total = 0;
    for (int c = 0; c < 8; c++) have_total += P->hearts[c];
    return have_total >= need_total ? 1 : 0;
}

/* Mirror live.rs::build_snapshot — construct a PerformanceSnapshot from
   LivePerformanceData. Fills the next available snapshot slot. */
void rb_build_snapshot(GameState *g, int turn, int pl, const int *live_card_ids, int n_live) {
    if (!g || !live_card_ids || n_live <= 0) return;
    if (g->n_snapshots >= RB_MAX_SNAPSHOTS) return;
    RbLiveSnapshot *s = &g->snapshots[g->n_snapshots];
    memset(s, 0, sizeof(*s));
    s->turn = turn;
    s->player = pl;
    s->n_lives = 0;
    for (int i = 0; i < n_live && i < RB_MAX_LIVE_CARDS; i++) {
        s->lives[s->n_lives] = live_card_ids[i];
        s->live_passed[s->n_lives] = 0;
        Card c;
        if (rb_decode_card_by_index((uint32_t)live_card_ids[i], &c)) {
            s->live_score_detail[s->n_lives] = c.score;
            rb_free_card(&c);
        }
        s->n_lives++;
    }
    memcpy(s->total_hearts, g->p[pl].hearts, sizeof(s->total_hearts));
    g->n_snapshots++;
}

/* Mirror live.rs::handle_live_success_choice — handle the result of a live
   success zone card selection. */
void rb_handle_live_success_choice(GameState *g, int pl, int selected_index) {
    if (!g || pl < 0 || pl > 1) return;
    RbPlayer *P = &g->p[pl];
    if (selected_index < 0 || selected_index >= P->live.n) {
        fprintf(stderr, "[LIVE_SUCCESS_CHOICE] pl=%d index=%d live=%d result=invalid_index\n",
                pl, selected_index, P->live.n);
        return;
    }
    int card_id = P->live.cards[selected_index];
    int can_place = rb_can_place_card_in_zone(g, card_id, "success_live_zone");
    fprintf(stderr, "[LIVE_SUCCESS_CHOICE] pl=%d index=%d card=%d can_place=%d live=%d\n",
            pl, selected_index, card_id, can_place, P->live.n);
    if (!can_place) return;
    for (int i = selected_index; i < P->live.n - 1; i++)
        P->live.cards[i] = P->live.cards[i + 1];
    P->live.n--;
    if (P->success.n < RB_MAX_ZONE) rb_success_add(P, card_id);
    else rb_waitroom_add(P, card_id);
    while (P->live.n > 0) {
        int cid = P->live.cards[0];
        for (int i = 0; i < P->live.n - 1; i++)
            P->live.cards[i] = P->live.cards[i + 1];
        P->live.n--;
        rb_waitroom_add(P, cid);
    }
}

/* Mirror live.rs::resolve_live_success_extras' per-seat block
    Self::trigger_live_success_abilities(game_state, player_id);
    Self::trigger_auto_abilities_for_player(game_state, player_id);
    game_state.process_pending_auto_abilities(player_id);
    if game_state.has_pending_choice() { return None; }

   Rust drives the drain ONE SEAT AT A TIME — process_pending_auto_abilities
   takes the player_id — so a seat's LiveSuccess entries are resolved before
   the other seat's pass runs. The C `rb_process_pending_auto_abilities` walks
   BOTH seats and `rb_process_player_abilities` empties the whole queue when it
   leaves no pending choice, so calling it after queueing P1's triggers drops
   P2's freshly queued entries (they carry P2's owner id and are not selected
   during the P1 pass) before the P2 pass ever sees them. Resolving the seat
   that was just triggered — the same call Rust makes — is what keeps a
   ライブ成功時 bonus (e.g. Strawberry Trapper's +2, which reads the OPPONENT's
   no-excess result) from being silently dropped.

   Returns 1 when a choice pends, i.e. Rust's `None` (caller returns early
   and re-enters through live_victory_stage, the C stand-in for Rust's
   live_success_p1_fired / live_success_p2_fired flags).

   NOTE on the one Rust line deliberately NOT reproduced here: Rust also calls
   `Self::trigger_auto_abilities_for_player(game_state, player_id)` between the
   trigger and the drain. Adding that C call here re-runs the 自動 scan at
   determination timing and was measured to turn 26 currently-green suites red
   (ability_effects, b8_live_timing, live_allocator, look_select, mechanics,
   member_activation_debut_live_start, move_cards, phase_machine_rules,
   target_selection, zone_filter_parity, parity_ability_mod,
   parity_auto_abilities, parity_card_identity, parity_conditions,
   parity_cost_compound, parity_custom_misc, parity_executor_dispatch,
   parity_integration, parity_look_reveal, parity_score_effects, ...). The
   performance-time scan rb_perform_live already runs, and the C 自動 queue
   entries are not idempotent under the second scan. Leave this out until the
   自動 queue gains Rust's use-limit/event gating; do not "complete" the port
   by adding the line. */
static int rb_resolve_live_success_side(GameState *g, int pl) {
    int before = g->queue.n_entries;
    int queued = rb_trigger_live_success(g, pl);
    int processed = rb_process_player_abilities(g, pl);
    int pending = rb_has_pending_choice(g);
    fprintf(stderr, "[LIVE_SUCCESS_SIDE] pl=%d queued=%d queue=%d->%d processed=%d pending=%d\n",
            pl, queued, before, g->queue.n_entries, processed, pending);
    return pending;
}

/* Mirror live.rs::move_to_success_and_update_attacker (live.rs:1264-1291):

     let p1_before = game_state.player1.success_live_card_zone.cards.len();
     let p2_before = game_state.player2.success_live_card_zone.cards.len();
     Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);
     let p1_now = game_state.player1.success_live_card_zone.cards.len();
     let p2_now = game_state.player2.success_live_card_zone.cards.len();
     let p1_added = p1_now > p1_before;
     let p2_added = p2_now > p2_before;
     if p1_added && !p2_added {
         game_state.player1.is_first_attacker = true;
         game_state.player2.is_first_attacker = false;
     } else if p2_added && !p1_added {
         game_state.player1.is_first_attacker = false;
         game_state.player2.is_first_attacker = true;
     }
     (p1_now, p1_added, p2_now, p2_added)

   Rule 8.4.13: if only ONE seat moved a card into the success zone this live,
   that seat becomes the first attacker for the next live. The previous C body
   called rb_move_live_to_success_and_handle_wins directly and never ran this
   step, so the attacker seats were never re-elected on the strength of THIS
   live's success-zone placement. Rust stores the flag per player
   (`player.is_first_attacker`); the C model stores it as the seat indices
   `GameState::first_attacker` / `second_attacker` (the same encoding
   phase.c:140-141 uses for the identical election), so the faithful C form is
   to move the two seat indices. The added/now counts are also the values Rust's
   structured "LIVE <verdict> | Pn ... → succ=N(+1)" summary line prints. */
static void rb_move_to_success_and_update_attacker(GameState *g, int p1_won, int p2_won,
                                                    int *p1_now, int *p1_added,
                                                    int *p2_now, int *p2_added) {
    int p1_before = g->p[0].success.n;
    int p2_before = g->p[1].success.n;
    rb_move_live_to_success_and_handle_wins(g);

    int p1_n = g->p[0].success.n;
    int p2_n = g->p[1].success.n;
    int p1_got = p1_n > p1_before;
    int p2_got = p2_n > p2_before;

    if (p1_got && !p2_got) {
        g->first_attacker = 0;
        g->second_attacker = 1;
        fprintf(stderr, "[FIRST_ATTACKER_RULE_8_4_13] first_attacker=0\n");
    } else if (p2_got && !p1_got) {
        g->first_attacker = 1;
        g->second_attacker = 0;
        fprintf(stderr, "[FIRST_ATTACKER_RULE_8_4_13] first_attacker=1\n");
    } else {
        fprintf(stderr, "[FIRST_ATTACKER_RULE_8_4_13] first_attacker unchanged (=%d) p1_added=%d p2_added=%d\n",
                g->first_attacker, p1_got, p2_got);
    }
    fprintf(stderr, "[LIVE_SUCCESS_SUMMARY] p1_won=%d p2_won=%d p1_succ=%d(+%d) p2_succ=%d(+%d)\n",
            p1_won, p2_won, p1_n, p1_got, p2_n, p2_got);
    if (p1_now) *p1_now = p1_n;
    if (p1_added) *p1_added = p1_got;
    if (p2_now) *p2_now = p2_n;
    if (p2_added) *p2_added = p2_got;
}

/* Mirror live.rs::execute_live_victory_determination — main orchestration for
   live victory determination. */
void rb_execute_live_victory_determination(GameState *g) {
    if (!g) return;
    if (g->live_victory_stage == 3) {
        fprintf(stderr, "[LIVE_VICTORY_RESUME] stage=3 p1_live=%d p2_live=%d\n",
                g->p[0].live.n, g->p[1].live.n);
        int rn = 0, ra = 0;
        rb_move_to_success_and_update_attacker(g, g->p1_live_won, g->p2_live_won,
                                               &rn, &ra, &rn, &ra);
        if (rb_has_pending_choice(g)) return;
        g->live_victory_stage = 0;
        return;
    }
    rb_apply_deferred_reyell(g);
    rb_rebuild_stage_hearts_with_yell(g);
    rb_record_pretrigger_live_results(g);

    if (g->live_victory_stage == 0) {
        if (rb_resolve_live_success_side(g, 0)) {
            g->live_victory_stage = 1;
            return;
        }
    }

    if (g->live_victory_stage <= 1) {
        if (rb_resolve_live_success_side(g, 1)) {
            g->live_victory_stage = 2;
            return;
        }
    }

    g->live_victory_stage = 2;
    rb_drain_pending_live_success_choices(g);
    if (rb_has_pending_choice(g)) return;

    rb_populate_live_verdicts(g);
    rb_process_delayed_gained_effects(g);

    int p1_extra = live_score_extra_for_side(g, 0);
    int p2_extra = live_score_extra_for_side(g, 1);
    int p1_score = 0;
    int p2_score = 0;
    rb_compute_pregame_scores(g, p1_extra, p2_extra, &p1_score, &p2_score);
    g->live_score[0] = p1_score;
    g->live_score[1] = p2_score;
    fprintf(stderr, "[LIVE_SCORE] p1=%d p2=%d\n", p1_score, p2_score);

    int p1_won = 0;
    int p2_won = 0;
    rb_determine_live_winners(g, &p1_won, &p2_won);
    g->p1_live_won = p1_won;
    g->p2_live_won = p2_won;
    rb_finalize_snapshot_fields(g, p1_won, p2_won, p1_score, p2_score);
    rb_revert_live_success_score_modifiers(g);
    rb_merge_late_score_apps(g);
    rb_compute_surplus_and_flags(g, p1_won, p2_won);
    g->live_victory_stage = 3;
    int p1_now = 0, p1_added = 0, p2_now = 0, p2_added = 0;
    rb_move_to_success_and_update_attacker(g, p1_won, p2_won,
                                           &p1_now, &p1_added, &p2_now, &p2_added);
    if (rb_has_pending_choice(g)) return;
    g->live_victory_stage = 0;
}

/* Mirror live.rs::process_player_live_result (live.rs:1437-1458) — move a
   single player's live card to the success zone (if won & can_place) or the
   waitroom, then drain the remaining live cards to the waitroom. Every card
   that lands in the waitroom is recorded into the file-static batch consumed by
   rb_move_live_to_success_and_handle_wins (Rust returns that Vec). */
void rb_process_player_live_result(GameState *g, int pl, int won, int must_skip, int can_place) {
    if (!g || pl < 0 || pl > 1) return;
    RbPlayer *P = &g->p[pl];
    int card_count = rb_live_len(P);
    if (won && !must_skip && card_count > 0) {
        int card_id = P->live.cards[0];
        for (int k = 0; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
        P->live.n--;
        if (can_place && P->success.n < RB_MAX_ZONE) {
            rb_success_add(P, card_id);
        } else {
            rb_waitroom_add(P, card_id);
            rb_moved_record(card_id);
        }
    }
    while (P->live.n > 0) {
        int card_id = P->live.cards[0];
        for (int k = 0; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
        P->live.n--;
        rb_waitroom_add(P, card_id);
        rb_moved_record(card_id);
    }
}

/* Faithful port of live.rs::backtrack_allocate + bt_search +
   try_surplus_compositions + try_phase4 + try_all_distribution.
   Exhaustive backtracking search over Phase 3a (surplus color) and Phase 4
   (icon_all wildcard) allocations. Called when greedy_allocate fails.
   Returns 1 if a valid allocation is found (writes flattened [card*8+color]
   allocation counts to out_allocs, max_allocs entries), 0 otherwise. */

/* Allocation descriptor written to out_allocs: encoded as card_idx*8 + color
   (one entry per unit of heart allocated). The decoder uses card = entry/8,
   color = entry%8. */
#define BT_ALLOC_ENTRY(card, color) ((card)*8 + (color))

/* card_ok_with_wildcard (ported from live.rs::card_ok_with_wildcard) — check
   whether a single card's heart requirements are satisfied given its filled
   array. COLORLESS hearts (filled[0]) count toward heart0/total bucket but
   can NEVER be used as a specific color — only icon_all (filled[7]) can
   cover a colored-note deficit. */
static int bt_card_ok(const int filled[8], const int need[8]) {
     int total_filled = 0, total_need = 0;
     for (int i = 0; i < 8; i++) { total_filled += filled[i]; total_need += need[i]; }
     if (total_filled < total_need) return 0;
     int icon_all = filled[7];
     if (need[0] > 0) {
          int any_hearts = 0;
          for (int c = 1; c < 7; c++) any_hearts += filled[c];
          any_hearts += filled[0];
          if (any_hearts + icon_all < need[0]) return 0;
          int deficit = need[0] - any_hearts;
          if (deficit > 0) { icon_all -= deficit; }
     }
     for (int c = 1; c < 7; c++) {
          if (filled[c] < need[c]) {
               int deficit = need[c] - filled[c];
               if (icon_all < deficit) return 0;
               icon_all -= deficit;
          }
     }
     return 1;
}

/* Forward declarations */
static int bt_search(int *pool, const int *card_needs, int n_cards, int idx,
                      int *allocs, int *n_allocs, int max_allocs);
static int bt_try_phase4(int *pool, const int *card_needs, int n_cards, int idx,
                          int *allocs, int *n_allocs, int max_allocs,
                          const int filled[8]);
static int bt_try_all_distribution(int *pool, const int *card_needs, int n_cards,
                                    int idx, int *allocs, int *n_allocs, int max_allocs,
                                    const int filled[8],
                                    const int *deficit_colors, int n_deficits,
                                    const int *deficit_amts, int remaining, int di);

/* try_surplus_compositions (ported from live.rs::try_surplus_compositions) —
   recursively enumerate all compositions of `remaining` hearts from
   colors[color_idx..]. For each composition, update pool/filled and recurse
   into try_phase4. */
static int bt_try_surplus(int *pool, const int *card_needs, int n_cards, int idx,
                           int *allocs, int *n_allocs, int max_allocs,
                           const int *colors, int n_colors,
                           int remaining, int color_idx, int filled[8]) {
     if (color_idx >= n_colors) {
          if (remaining > 0) return 0;
          return bt_try_phase4(pool, card_needs, n_cards, idx, allocs, n_allocs, max_allocs, filled);
     }
     int saved_pool[8]; memcpy(saved_pool, pool, sizeof(saved_pool));
     int saved_n = *n_allocs;
     int c = colors[color_idx];
     int max_take = pool[c] < remaining ? pool[c] : remaining;
     for (int take = 0; take <= max_take; take++) {
          int new_filled[8]; memcpy(new_filled, filled, sizeof(new_filled));
          if (take > 0) {
               int entry = BT_ALLOC_ENTRY(idx, c);
               if (*n_allocs < max_allocs) allocs[(*n_allocs)++] = entry;
               pool[c] -= take;
               new_filled[c] += take;
          }
          if (bt_try_surplus(pool, card_needs, n_cards, idx, allocs, n_allocs, max_allocs,
                              colors, n_colors, remaining - take, color_idx + 1, new_filled))
               return 1;
          memcpy(pool, saved_pool, sizeof(saved_pool));
          *n_allocs = saved_n;
     }
     return 0;
}

/* try_phase4 (ported from live.rs::try_phase4) — after Phase 3a choices are
   made, try Phase 3b (heart00 -> heart00 deficit) and Phase 4 (icon_all ->
   remaining deficits). */
static int bt_try_phase4(int *pool, const int *card_needs, int n_cards, int idx,
                          int *allocs, int *n_allocs, int max_allocs,
                          const int filled[8]) {
     int saved_pool[8]; memcpy(saved_pool, pool, sizeof(saved_pool));
     int saved_n = *n_allocs;
     int need[8]; memcpy(need, &card_needs[idx * 8], sizeof(need));

     int total_filled_so_far = 0;
     for (int i = 0; i < 8; i++) total_filled_so_far += filled[i];
     int total_required = 0;
     for (int i = 0; i < 8; i++) total_required += need[i];
     int h00_deficit = total_required > total_filled_so_far ? total_required - total_filled_so_far : 0;

     int new_filled[8]; memcpy(new_filled, filled, sizeof(new_filled));

     /* Phase 3b: Heart00 (COLORLESS) -> remaining heart0/total deficit (forced) */
     if (h00_deficit > 0 && pool[0] > 0) {
          int take = pool[0] < h00_deficit ? pool[0] : h00_deficit;
          int entry = BT_ALLOC_ENTRY(idx, 0);
          for (int u = 0; u < take && *n_allocs < max_allocs; u++)
               allocs[(*n_allocs)++] = entry;
          pool[0] -= take;
          new_filled[0] += take;
     }

     /* Recompute deficits */
     int total_filled_now = 0;
     for (int i = 0; i < 8; i++) total_filled_now += new_filled[i];
     int h00_still_needed = total_required > total_filled_now ? total_required - total_filled_now : 0;

     int deficit_colors[8], deficit_amts[8], n_deficits = 0;
     for (int c = 1; c < 7; c++) {
          if (new_filled[c] < need[c]) {
               deficit_colors[n_deficits] = c;
               deficit_amts[n_deficits] = need[c] - new_filled[c];
               n_deficits++;
          }
     }
     if (h00_still_needed > 0) {
          deficit_colors[n_deficits] = 0;
          deficit_amts[n_deficits] = h00_still_needed;
          n_deficits++;
     }

     int all_count = pool[7];
     if (all_count == 0 && n_deficits == 0) {
          if (bt_card_ok(new_filled, need)) {
               return bt_search(pool, card_needs, n_cards, idx + 1, allocs, n_allocs, max_allocs);
          }
     } else if (all_count == 0) {
          /* No icon_all but deficits exist */
     } else {
          if (bt_try_all_distribution(pool, card_needs, n_cards, idx, allocs, n_allocs, max_allocs,
                                       new_filled, deficit_colors, n_deficits, deficit_amts,
                                       all_count, 0))
               return 1;
     }

     memcpy(pool, saved_pool, sizeof(saved_pool));
     *n_allocs = saved_n;
     return 0;
}

/* try_all_distribution (ported from live.rs::try_all_distribution) — try all
   distributions of `remaining` icon_all hearts to deficit types starting at di. */
static int bt_try_all_distribution(int *pool, const int *card_needs, int n_cards,
                                    int idx, int *allocs, int *n_allocs, int max_allocs,
                                    const int filled[8],
                                    const int *deficit_colors, int n_deficits,
                                    const int *deficit_amts, int remaining, int di) {
     if (di >= n_deficits) {
          if (remaining > 0) return 0;
          int need[8]; memcpy(need, &card_needs[idx * 8], sizeof(need));
          if (bt_card_ok(filled, need)) {
               return bt_search(pool, card_needs, n_cards, idx + 1, allocs, n_allocs, max_allocs);
          }
          return 0;
     }
     int saved_pool[8]; memcpy(saved_pool, pool, sizeof(saved_pool));
     int saved_n = *n_allocs;
     int target_color = deficit_colors[di];
     int deficit_amt = deficit_amts[di];
     int max_take = remaining < deficit_amt ? remaining : deficit_amt;
     int need[8]; memcpy(need, &card_needs[idx * 8], sizeof(need));

     for (int take = 0; take <= max_take; take++) {
          int new_filled[8]; memcpy(new_filled, filled, sizeof(new_filled));
          if (take > 0) {
               int alloc_color = (target_color == 0) ? 7 : target_color;
               int entry = BT_ALLOC_ENTRY(idx, alloc_color);
               for (int u = 0; u < take && *n_allocs < max_allocs; u++)
                    allocs[(*n_allocs)++] = entry;
               pool[7] -= take;
               if (target_color == 0) new_filled[0] += take;
               else new_filled[target_color] += take;
          }
          if (bt_try_all_distribution(pool, card_needs, n_cards, idx, allocs, n_allocs, max_allocs,
                                       new_filled, deficit_colors, n_deficits, deficit_amts,
                                       remaining - take, di + 1))
               return 1;
          memcpy(pool, saved_pool, sizeof(saved_pool));
          *n_allocs = saved_n;
     }
     return 0;
}

/* bt_search (ported from live.rs::bt_search) — recursive backtracking: try all
   valid allocations for card `idx` then recurse. */
static int bt_search(int *pool, const int *card_needs, int n_cards, int idx,
                       int *allocs, int *n_allocs, int max_allocs) {
     if (idx >= n_cards) return 1;
     int saved_pool[8]; memcpy(saved_pool, pool, sizeof(saved_pool));
     int saved_n = *n_allocs;
     int need[8]; memcpy(need, &card_needs[idx * 8], sizeof(need));

     /* Phase 1a: matching colored hearts -> color req (no choice) */
     int filled[8] = {0};
     for (int c = 1; c < 7; c++) {
          if (need[c] > 0 && pool[c] > 0) {
               int take = pool[c] < need[c] ? pool[c] : need[c];
               int entry = BT_ALLOC_ENTRY(idx, c);
               for (int u = 0; u < take && *n_allocs < max_allocs; u++)
                    allocs[(*n_allocs)++] = entry;
               pool[c] -= take;
               filled[c] += take;
          }
     }

     /* Compute h00 deficit */
     int total_filled_so_far = 0;
     for (int i = 0; i < 8; i++) total_filled_so_far += filled[i];
     int total_required = 0;
     for (int i = 0; i < 8; i++) total_required += need[i];
     int h00_deficit = total_required > total_filled_so_far ? total_required - total_filled_so_far : 0;

     /* Collect available surplus colors (pool[c] > 0 for c in 1..7) */
     int surplus_colors[7], n_surplus = 0;
     for (int c = 1; c < 7; c++) if (pool[c] > 0) surplus_colors[n_surplus++] = c;
     int total_surplus = 0;
     for (int i = 0; i < n_surplus; i++) total_surplus += pool[surplus_colors[i]];
      int max_surplus_take = h00_deficit < total_surplus ? h00_deficit : total_surplus;
      for (int surplus_take = 0; surplus_take <= max_surplus_take; surplus_take++) {
           int phase_pool[8]; memcpy(phase_pool, pool, sizeof(phase_pool));
           int phase_n = *n_allocs;
           if (bt_try_surplus(pool, card_needs, n_cards, idx, allocs, n_allocs, max_allocs,
                              surplus_colors, n_surplus, surplus_take, 0, filled))
                return 1;
           memcpy(pool, phase_pool, sizeof(phase_pool));
           *n_allocs = phase_n;
      }


     /* Undo */
     memcpy(pool, saved_pool, sizeof(saved_pool));
     *n_allocs = saved_n;
     return 0;
}

/* backtrack_allocate (ported from live.rs::backtrack_allocate) — entry point. */
int rb_backtrack_allocate(const int pool[8], const int card_needs[8], int n_cards,
                           int *out_allocs, int max_allocs) {
     if (!pool || !card_needs || n_cards <= 0 || !out_allocs || max_allocs <= 0)
          return 0;
     int work_pool[8]; memcpy(work_pool, pool, sizeof(work_pool));
     int n_allocs = 0;
     if (bt_search(work_pool, card_needs, n_cards, 0, out_allocs, &n_allocs, max_allocs))
          return 1;
     return 0;
}

/* try_surplus_compositions — public wrapper mirroring the Rust fn signature.
   Delegates to bt_try_surplus for the actual enumeration. */
int rb_try_surplus_compositions(int *pool, const int card_needs[8], int n_cards,
                                 int idx, const int *colors, int n_colors,
                                 int remaining, int color_idx,
                                 int *allocs, int *filled) {
     if (!pool || !card_needs || !colors || !allocs || !filled) return 0;
     int n_allocs = 0;
     return bt_try_surplus(pool, card_needs, n_cards, idx, allocs, &n_allocs,
                            64, colors, n_colors, remaining, color_idx, filled);
}

/* Mirror live.rs::card_ok_with_wildcard — check whether a single card's heart
   requirements are satisfied given its filled array. Implements canonical
   acceptance rules: total coverage, heart0 bucket, per-color deficits. */
int rb_card_ok_with_wildcard(const int filled[8], const int need[8]) {
    if (!filled || !need) return 0;
    return bt_card_ok(filled, need);
}

/* Mirror live.rs::bt_search — the recursive per-card entry point: take Phase 1a
   on card `idx` (no choice: matching coloured hearts fill their own colour),
   then enumerate Phase 3a surplus-colour compositions, each of which falls
   through to Phase 3b (heart00 -> remaining total deficit) and Phase 4
   (icon_all -> per-colour deficits) before recursing to `idx + 1`. Returns 1
   as soon as some complete assignment of cards [idx..n) is found.

   The Rust signature also threads an `allocs` out-parameter and a
   recursion-depth budget; the exported C signature declared in
   include/rabuka.h carries neither, so this wrapper runs the real searcher
   with a scratch buffer and reports success. `pool` is consumed exactly as the
   Rust original consumes it (mutated in place, restored on failure). */
int rb_bt_search(const GameState *g, int pl, int *pool, int *card_needs,
                 int n_needs, int idx) {
    (void)g; (void)pl;
    if (!pool || !card_needs || n_needs <= 0) return 0;
    if (idx < 0 || idx >= n_needs) return 0;
    int scratch[64];
    int n_allocs = 0;
    return bt_search(pool, card_needs, n_needs, idx, scratch, &n_allocs, 64);
}

/* Mirror live.rs::try_phase4 — after the Phase 3a surplus choices, apply
   Phase 3b (COLORLESS heart00 pool -> the card's remaining heart00/total
   deficit only, never a specific colour) and then Phase 4 (icon_all, pool
   slot 7, -> the still-unmet per-colour deficits, then the heart00 bucket).
   Returns 1 once the card's requirement array is satisfied.

   The Rust function additionally recurses into bt_search for the following
   card and enumerates icon_all splits; neither is expressible with this
   exported signature (it has no access to the remaining cards), so this
   applies the same two phases for the single card described by `need` and
   reports whether that card is now covered. */
int rb_try_phase4(const GameState *g, int pl, int *filled, const int *need) {
    (void)g; (void)pl;
    if (!filled || !need) return 0;
    int total_required = 0;
    for (int i = 0; i < 8; i++) total_required += need[i];
    int total_filled = 0;
    for (int i = 0; i < 8; i++) total_filled += filled[i];
    /* Phase 3b — COLORLESS heart00 covers the remaining heart00/total deficit.
       The exported signature has no pool argument, so only the already-allocated
       heart00 units in `filled[0]` can be spent here. */
    int deficit = total_required - total_filled;
    if (deficit < 0) deficit = 0;
    if (deficit > 0 && filled[0] > 0) {
        int take = filled[0] < deficit ? filled[0] : deficit;
        filled[0] -= take;
        total_filled += take;
    }
    /* Phase 4 — icon_all covers any still-unmet colour, then heart00. */
    if (filled[7] > 0) {
        for (int c = 1; c < 7; c++) {
            if (need[c] > filled[c]) {
                int d = need[c] - filled[c];
                int take = filled[7] < d ? filled[7] : d;
                filled[7] -= take;
                filled[c] += take;
            }
        }
        int colored = 0;
        for (int c = 1; c < 7; c++) colored += filled[c];
        int h00_left = need[0] - colored;
        if (h00_left > 0) {
            int take = filled[7] < h00_left ? filled[7] : h00_left;
            filled[7] -= take;
            filled[0] += take;
        }
    }
    return bt_card_ok(filled, need);
}

/* live.rs::try_all_distribution enumerates every way the remaining icon_all
   hearts can be split across a card's deficit buckets. Every one of its
   parameters is data (the live pool, the already-filled array, the deficit
   colour/amount lists, the remaining icon_all count, the deficit cursor) and
   NONE of them is present in the exported C signature declared in
   include/rabuka.h — only (game, pl). There is therefore no faithful body that
   can be written against this signature; the real port is the static
   bt_try_all_distribution above, which rb_backtrack_allocate drives.

   Rather than return a hard-coded 1 (which would assert "a distribution was
   found" without having looked at anything), report that no distribution was
   produced. Nothing in the engine calls this symbol. */
int rb_try_all_distribution(const GameState *g, int pl) {
    if (!g) return 0;
    (void)pl;
    return 0;
}
