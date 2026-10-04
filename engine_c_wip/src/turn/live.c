#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* Forward: rb_perform_live calls this at the point Rust's phases.rs:583 calls
   enrich_from_applications (after check_live_success, at the snapshot push);
   the definition sits far below, beside the other display-projection helpers. */
void rb_enrich_from_applications(GameState *g);

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





/* rb_calc_stage_hearts is a C-ONLY alias, not a live.rs item: Rust spells the
   same thing `Player::calculate_stage_hearts` (engine/src/core/player.rs:516),
   which delegates to `stats_pipeline::stage_hearts` (stats_pipeline.rs:93).
   The C counterpart of THAT is rb_stage_hearts_pipeline
   (core/stats_pipeline.c:269), and it is the one production code uses — this
   file calls it at :512 and :588, phase.c owns the rest. rb_calc_stage_hearts
   exists only because tests call it by the Rust method's name
   (tests/replay.c:82, test_parity_blade_under.c:1864/:1890,
   test_parity_hearts_live_start.c, test_parity_required_hearts.c,
   test_parity_rules_conditions.c, test_parity_full_round3.c,
   test_parity_hearts_constants.c, test_parity_compound_per_card.c,
   test_parity_characterization_extra.c). It is a one-line forward, kept. */
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
/* Mirror live.rs::YellIconOutcome (live.rs:61-65). Kept as a file-local type
   because the exported entry point below flattens it into out-params: a public
   function cannot expose a file-local struct to another translation unit. */
typedef struct {
    int blade_hearts[8];
    int note_icons;
    int draw_icons;
} RbYellIconOutcome;

/* EXPORTED (was `static`). engine/src/ability/effects/reyell.rs:160 is the
   second production caller of Rust's `pub fn process_yell_revealed_card_icons`
   (the first is live.rs:1997, inside process_yell_zone_cards), and its C port
   src/ability/effects/rules.c:1997 carries a byte-for-byte duplicate of this
   body under the name `r_yell_icons`, with the note "live.c owns a `static`
   copy; this local mirror keeps rules.c self-contained until live.c exports it
   (CROSS-FILE REQUEST 4)". This is that export. The signature is deliberately
   FLAT (out-params, int return) so rules.c can call it with nothing but an
   `extern` declaration; the return value is 1 when the card decoded, 0 when it
   did not (Rust: `if let Some(card) = card_db.get_card(*card_id)` — the whole
   body is skipped for an unknown id). Once rules.c drops its copy and adds the
   extern, the three hand-rolled variants of this decode (here, r_yell_icons,
   and phase.c:1369 ptc_yell_had_draw_icon) become one.

   CROSS-FILE REQUEST to the owner of src/ability/effects/rules.c: add
       extern int rb_process_yell_revealed_card_icons(const GameState *, int, int,
           int total_hearts[8], int *cheer_count, int blade_hearts_out[8],
           int *note_icons_out, int *draw_icons_out);
   and delete r_yell_icons (rules.c:1992-2031) plus its call at rules.c:2129.
   Adding it to include/rabuka.h is the header owner's call, not this file's. */
int rb_process_yell_revealed_card_icons(const GameState *g,
        int cid, int override_color, int total_hearts[8], int *cheer_count,
        int blade_hearts_out[8], int *note_icons_out, int *draw_icons_out){
    RbYellIconOutcome out;
    Card c;
    memset(&out, 0, sizeof(out));
    if (blade_hearts_out) memset(blade_hearts_out, 0, 8 * sizeof(int));
    if (note_icons_out) *note_icons_out = 0;
    if (draw_icons_out) *draw_icons_out = 0;
    if (!total_hearts || !cheer_count || !g || cid < 0) return 0;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;

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
    if (blade_hearts_out) memcpy(blade_hearts_out, out.blade_hearts, 8 * sizeof(int));
    if (note_icons_out) *note_icons_out = out.note_icons;
    if (draw_icons_out) *draw_icons_out = out.draw_icons;
    return 1;
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
        int yb[8], ynote = 0, ydraw = 0;
        rb_process_yell_revealed_card_icons(g, card_id, override_color, blade_hearts,
                                            note_icons, yb, &ynote, &ydraw);
        draw_icons += ydraw;
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

/* ── build_member_contributions (faithful port of live.rs:1844-1915) ───────────
   Rust builds one MemberContribution per OCCUPIED stage slot and returns the
   Vec, which `player_perform_live` stores in `LivePerformanceData.member_contributions`
   (live.rs:2197). That Vec is the ONLY production consumer of two functions in
   engine/src/core/stats_pipeline.rs:

       live.rs:1872  member_heart_detail   -> C rb_member_heart_detail
       live.rs:1884  effective_blade_parts  -> C rb_effective_blade_parts

   Both C twins are faithful and were UNREACHABLE from src/ for exactly that
   reason (they were exercisable only from tests/test_parity_stats_pipeline.c and
   tests/test_parity_characterization_extra.c:1099). Calling this at the Rust
   call site is what makes them reachable from production code again.

   (`need_satisfied` — the third function stats_pipeline.c blocked on — is NOT
   reached from here, and cannot be: live.rs:1844-1915 never calls it, and
   stats_pipeline.rs:258 has ZERO callers in engine/src AND in engine/tests. It
   is dead in Rust too.)

   WHERE THE RESULT GOES — RESOLVED 2026-10-03 (checklist A18). The header now
   carries `RbLiveSnapshot::member_contributions[2][RB_STAGE_SIZE]` +
   `n_member_contributions[2]` (rabuka.h:1348-1349), so the data lands on the
   snapshot the way Rust puts it on PerformanceSnapshot.member_contributions.
   The two file-statics below are kept ONLY as the per-window scratch the builder
   fills: rb_enrich_from_applications copies them onto the snapshot at the
   phases.rs:583 position (inside rb_perform_live, at the push) and computes
   `transform_delta` there. Before A18 the vector had nowhere to go and the
   header's own note on RbMemberContribution said `card_no` is "owned by the
   snapshot" — that is what parked it here. */
static RbMemberContribution s_member_contrib[2][RB_STAGE_SIZE];
static int s_n_member_contrib[2];

/* live.rs:1886-1889 — `is_wait` is `orientation_modifiers[cid] == Wait`. The C
   carries orientation as a string on the mods table (same source live.c's
   rb_stage_member_waited reads at :130). */
static int live_member_is_wait(const GameState *g, int cid) {
    const char *o = rb_mods_get_orientation((RbMods *)&g->mods, cid);
    return (o && !strcmp(o, "wait")) ? 1 : 0;
}

static int live_build_member_contributions(const GameState *g, int pl,
                                            RbMemberContribution *out, int max_out) {
    if (!g || pl < 0 || pl > 1 || !out || max_out <= 0) return 0;
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (n >= max_out) break;
        int cid = g->p[pl].stage[i];
        if (cid == RB_EMPTY_SLOT) continue;          /* live.rs:1857-1859 */
        if (cid < 0 || cid >= RB_MAX_CARD_IDS) continue;

        RbMemberContribution mc;
        memset(&mc, 0, sizeof(mc));
        mc.source_id = cid;
        mc.slot = (uint8_t)i;                        /* live.rs:1893 */

        /* live.rs:1860-1870 — draw_icons counts only the HeartColor::Draw entries
           of the card's special_heart; live.rs:1881 — printed_blade is the base
           the blade split is layered onto. Both come out of one card decode. */
        int printed_blade = 0;
        Card card;
        if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &card)) {
            if (card.has_special && card.special_color == RB_HEART_DRAW)
                mc.draw_icons = (uint8_t)card.special_count;
            printed_blade = (int)card.blade;
            const char *no = rb_card_string(card.card_no_idx);
            mc.card_no = no ? no : "";
            rb_free_card(&card);
        } else {
            mc.card_no = "";                         /* ArcStr::default() */
        }

        /* live.rs:1872-1879 — heart pipeline: base vs bonus, one definition. */
        rb_member_heart_detail(&g->mods, cid, mc.base_hearts, mc.bonus_hearts);

        /* live.rs:1882-1884 — blade pipeline: effective base + additive bonus.
           `blade_modifiers.get(&cid).copied().unwrap_or_default()` is the
           mods table's zero-initialized entry for an unmodified card. */
        int base_blades = 0, bonus_blades = 0;
        rb_effective_blade_parts(g->mods.blade[cid], printed_blade,
                                 &base_blades, &bonus_blades);
        mc.base_blades = (uint8_t)base_blades;
        mc.bonus_blades = (uint8_t)bonus_blades;

        mc.is_wait = live_member_is_wait(g, cid);   /* live.rs:1886-1889 */
        /* base_notes / bonus_notes / transform_delta are hard-zero at
           construction (live.rs:1897-1898, :1907) and only written later, at
           live.rs:3035 (enrich_from_applications) and live.rs:783-800
           (finalize_snapshot_fields). memset above already did that. */

        if (rb_ability_debug_enabled())
            fprintf(stderr,
                    "[MEMBER_CONTRIB] cid=%d slot=%d base_hearts=[%u,%u,%u,%u,%u,%u,%u,%u] "
                    "bonus_hearts=[%u,%u,%u,%u,%u,%u,%u,%u] base_blades=%d bonus_blades=%d\n",
                    cid, i,
                    mc.base_hearts[0], mc.base_hearts[1], mc.base_hearts[2], mc.base_hearts[3],
                    mc.base_hearts[4], mc.base_hearts[5], mc.base_hearts[6], mc.base_hearts[7],
                    mc.bonus_hearts[0], mc.bonus_hearts[1], mc.bonus_hearts[2], mc.bonus_hearts[3],
                    mc.bonus_hearts[4], mc.bonus_hearts[5], mc.bonus_hearts[6], mc.bonus_hearts[7],
                    base_blades, bonus_blades);
        out[n++] = mc;
    }
    return n;
}

/* ── I2 BLOCKED-ON: discard_live_on_cannot_live (live.rs:1830-1839) ─────────
   WHY `cannot_live` NEVER REACHED THIS FILE — the full chain, so the next
   reader does not have to re-derive it:

     * WRITER exists. src/ability/effects/rules.c:1876-1878 sets
       `rb_rules_state(g)->cannot_live_players[pl] = 1` from the `cannot_live`
       arm of r_apply_restriction — the twin of Rust's restriction.rs:100-105
       (and modifiers.rs:1097-1100 for the constant path).
     * READER exists, with ZERO callers anywhere in src or tests:
       `rb_rules_cannot_live_player` (rules.c:2268). That zero is the whole bug:
       phases.rs:351 `cannot_live_players.contains(&player_id)` had no C
       counterpart, so rb_perform_live had no gate and a cannot_live seat
       performed normally.
     * THE MISSING THIRD PIECE IS THE CLEAR, and it is what makes this
       BLOCKED-ON rather than a two-line fix. Rust clears `cannot_live_players`
       at every turn rollover (phases.rs:280). The C copy lives in the
       file-static `RbRulesState` (rules.c:72) whose accessor `rb_rules_state` is
       `static`, so neither this file nor src/turn/phase.c can clear it. Wiring
       the reader WITHOUT the clear turns a correct gate into a permanent one:
       `cannot_live_players[pl]` would stay 1 for the rest of the match and the
       seat could never perform a live again.
     * `rb_rules_forget` (rules.c:140) is not a substitute — it is 0-callers too
       and drops EVERY side table for the state (delayed_prohibition,
       wait_immune, conditional_choice, moved_cards, …), none of which Rust
       clears at the rollover.

   EXACT SYMBOL NEEDED (src/ability/effects/rules.c, not this file's):

   one exported definition plus its prototype — the declaration text is quoted
   ONCE, at src/turn/phase.c, and deliberately not repeated here. Until then the
   blocker has no declaration anywhere in the tree, so a future call site fails
   at LINK rather than silently resolving to nothing.

   clearing `RbRulesState::cannot_live_players[2]` and, in the same frame,
   `cannot_activate_members[2]` (Rust phases.rs:279-280 — both are per-turn sets
   cleared together). It must be called from rb_advance_phase's RB_PHASE_VICTORY
   rollover at src/turn/phase.c:427, next to the player_cannot_activate and
   baton_touch_used clears that are already there.

   ⚠ THIS FILE USED TO SPELL THAT PROTOTYPE OUT, inside a comment, and so did
   src/turn/phase.c. Two declarations, no definition anywhere in the tree, no
   caller — while checklist item I2 claimed `[x]`. A caller added later would
   have hit a LINK ERROR with nothing in the build to catch it, because a
   comment is not a declaration. The single tombstone for this blocker now lives
   at src/turn/phase.c in the RB_PHASE_VICTORY rollover; the definition and its
   prototype must arrive together, in rules.c, in one edit.

   Then this file's body is exactly the shape below, in rb_perform_live, after
   build_member_contributions (live.rs:2053-2070 — the bail-out sits between the
   member-contribution build and the empty-live-zone return, so the contributions
   are still built):

       if (rb_rules_cannot_live_player(g, pl)) {
           s_n_member_contrib[pl] = live_build_member_contributions(...);
           for (int i = 0; i < P->live.n; i++) rb_waitroom_add(P, P->live.cards[i]);
           P->live.n = 0;
           return 0;
       }

   Nothing is stubbed in here on purpose: a `#if 0` copy of a live Rust branch
   is dead code that reads like coverage, so the diagnosis is left in prose and
   the branch is left absent until it can be correct.
   See PARITY_CHECKLIST.md I2. */

int rb_perform_live(GameState *g, int pl){
    if (!g || pl < 0 || pl > 1) return 0;
    RbPlayer *P=&g->p[pl];
    /* live.rs:2073 — `Self::build_member_contributions(...)` is the first thing
       player_perform_live does after the `cannot_live` bail-out, i.e. BEFORE the
       empty-live-zone return at live.rs:2086. Placing the call here reproduces
       that order exactly. */
    s_n_member_contrib[pl] = live_build_member_contributions(g, pl,
                                                             s_member_contrib[pl],
                                                             RB_STAGE_SIZE);
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
        /* phases.rs:583 — enrich_from_applications runs after check_live_success
           and BEFORE build_snapshot / push_performance_snapshot, which is exactly
           here: the allocation verdict is in, the snapshot slot is taken, and the
           ライブ成功時 abilities have NOT fired yet (they run in
           rb_execute_live_victory_determination). The C pushes the snapshot
           inline where Rust calls build_snapshot, so the C's call site is the
           push, not a separate later line. */
        rb_enrich_from_applications(g);
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
            else rb_waitroom_add(P, cid);
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

/* Mirror live.rs::determine_winners (live.rs:442-503) — decide which player(s)
   placed a live this turn. A player "won" iff they passed ALL their live cards'
   heart checks (g->live_success[pl], which rb_perform_live / rb_populate_live_
   verdicts / rb_record_pretrigger_live_results all write). If both passed, the
   higher live score wins; on a score tie BOTH win (both place to success),
   which is the Rust tie rule that feeds
   move_live_to_success_and_handle_wins / the first-attacker rollover.

   ONE RUST TERM HAS NO C COUNTERPART, and it is deliberately NOT synthesised.
   Rust (live.rs:449-464):

       let p1_has = !game_state.player1.live_card_zone.cards.is_empty();
       let p1_all = p1_has && <latest snapshot: !lives.is_empty() && all passed>;

   The `p1_has` conjunct cannot be read as `g->p[0].live.n > 0` in this port,
   because the C drains the live zone at a DIFFERENT point than Rust does:
   rb_perform_live moves the live cards to success/discard itself at :605-624
   on the direct (non-phase-machine) path, whereas Rust does that only inside
   move_live_to_success_and_handle_wins, which runs at the very END of
   execute_live_victory_determination. Adding the guard would therefore be
   correct on the `live_batch_mode` path (where rb_perform_live returns early
   at :520-526 and leaves the zone intact) and would make BOTH seats lose on
   the direct path. `g->live_success[pl]` is reset per turn by
   src/core/tracking.c:56-57, which is what stands in for the `p1_has` guard
   Rust gets from the zone contents. Left as-is deliberately. */
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
    parity.

    THE THREE REMAINING WRITES OF THIS FUNCTION ARE NOW TOMBSTONED, NOT
    BLOCKED-ON. Rust writes, after the arms above:

        snap.lives[i].spare          = spare;                       // live.rs:741
        snap.base_score_total       = Σ passed l.score;            // live.rs:771
        snap.card_bonus_total       = Σ passed (l.score - l.base_score); // :777
        mc.transform_delta[i]       = bonus_hearts[i] - ability_per_color[i]; // :798

    `transform_delta` IS landed — it is an RbMemberContribution field
    (rabuka.h:1283) and rb_enrich_from_applications above computes it from the
    ability-application trace, exactly as live.rs:783-800 does.

    ⛔ The other three were DELETED from RbLiveSnapshot on 2026-10-04 (field
    census), and this paragraph is the reason that was the right call rather than a
    loss. The old text here asked the header for
        int base_score_total; int card_bonus_total; int live_spare[RB_MAX_LIVE_CARDS][8];
    on the grounds that "with those three fields the bodies are three lines each".
    The premise was never checked, and it is false: in RUST all three are
    WRITE-ONLY. `base_score_total` / `card_bonus_total` (types.rs:498 / :500) are
    written at live.rs:771 / :777 and read NOWHERE in engine/src — not by the
    condition evaluator, not by the live-success check, not by the display
    projection; the only readers anywhere are the characterisation test
    tests/test_modules/characterization/performance_snapshot_audit_test.rs:82/144.
    `LiveCardResult.spare` (types.rs:512) is written at live.rs:733-741 and
    initialised empty at live.rs:3153; `spare` appears nowhere else in engine/src
    outside unrelated bot-local variables. So the C bodies would have written three
    values that nothing on either side can observe — three lines each, producing
    nothing, which is precisely the "producer for something nobody consumes" defect
    the census set out to remove. The `AllocPhase` difficulty described below is
    real but it was difficulty in service of a dead value.

    MEASURED: all three are WRITE-ONLY in Rust. A repo-wide search of
    engine/src finds `spare` at types.rs:512 (declaration), live.rs:741 (this
    write) and live.rs:3153 (build_snapshot's EMPTY_H8 initialiser) — no reader.
    `base_score_total` / `card_bonus_total` likewise appear only at types.rs:498/500,
    live.rs:771/777 and live.rs:3180-3181. So landing them changes no engine
    behaviour; they are display fields for the web/JSON surface. */
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

/* Mirror live.rs::compute_snapshot_surplus (live.rs:1119-1167) — the PRE-TRIGGER
    surplus pass. This is a DIFFERENT computation from rb_compute_surplus_and_flags
    above, not a duplicate of it, and the difference is load-bearing:

      * compute_surplus_and_flags runs at live.rs:1406, AFTER every ライブ成功時
        ability has resolved, and GATES on `!snap.success` (Q142/Q259).
      * compute_snapshot_surplus runs at live.rs:1206, from INSIDE
        resolve_live_success_extras, immediately BEFORE the first ライブ成功時
        block, has NO success gate, and computes `required` from the PRINTED
        need_heart rather than from the modified requirement.

    Its doc comment names the consumer: "Feeds color-filtered surplus
    conditions (e.g. La Bella Patria heart04 >= 1, Q174)" — i.e.
    ConditionCard::count_surplus_heart (card.rs:4200-4245) reads
    `snap.surplus_hearts[color]` while `live_surplus_ready_this_turn` is true, and
    that reader runs DURING the ライブ成功時 abilities. Without this function the
    C's per-colour array still held the previous live's numbers (or zero) at
    exactly the moment Rust's holds the correct ones.

    Two details carried across verbatim:
      * `filled_per_color` sums `alloc.color` RAW — there is no H00Wild /
        AllCleanup remap here (unlike finalize_snapshot_fields). The C's
        `live_filled[i][c]` is likewise the raw colour bucket the allocator took
        from, so the two agree by construction (see I4).
      * the surplus COUNT (the u8 written to self/opponent_live_surplus_count)
        uses the printed requirement; only the per-colour array uses the
        allocation. Both are reproduced separately below.

    `snap.surplus_hearts = per_color` writes Rust's [u8;8]; the C folds that into
    `surplus_per_color` and keeps the scalar `surplus_hearts` for
    rb_compute_surplus_and_flags' `surplus` total, so ONLY surplus_per_color is
    touched here — the scalar is not this function's to write. */
static void rb_compute_snapshot_surplus(GameState *g) {
    if (!g) return;
    for (int si = 0; si < g->n_snapshots; si++) {
        RbLiveSnapshot *s = &g->snapshots[si];
        int pl = s->player;
        if (pl < 0 || pl > 1) continue;

        int total_hearts = 0;
        for (int c = 0; c < 8; c++) total_hearts += s->total_hearts[c];

        /* live.rs:1133-1140 — `.filter_map(get_card).filter_map(|c| c.need_heart
           .as_ref()).flat_map(|nh| nh.hearts.values()).sum()`, i.e. the PRINTED
           need of every live-zone card, with no need_heart_modifiers layering
           (that is sp_effective_need_heart / rb_effective_need_heart, a
           different function). `need_heart` decodes as the `num_need` slice of
           the card's heart list, which is what sp_card_need_hearts reads before
           it applies the modifiers. */
        const RbPlayer *P = &g->p[pl];
        int required = 0;
        for (int i = 0; i < P->live.n; i++) {
            Card c;
            if (!rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) continue;
            int start = c.num_base + c.num_blade;
            for (int k = 0; k < c.num_need && start + k < c.n_hearts; k++)
                required += c.heart_count[start + k];
            rb_free_card(&c);
        }
        int surplus = total_hearts - required;
        if (surplus < 0) surplus = 0;              /* saturating_sub */
        if (pl == 1) g->opponent_live_surplus_count = surplus;
        else          g->self_live_surplus_count = surplus;

        int filled_per_color[8] = {0};
        for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++)
            for (int c = 0; c < 8; c++) filled_per_color[c] += s->live_filled[i][c];
        for (int c = 0; c < 8; c++) {
            int per_color = s->total_hearts[c] - filled_per_color[c];
            if (per_color < 0) per_color = 0;      /* saturating_sub */
            s->surplus_per_color[c] = per_color;
        }
    }
    g->live_surplus_ready_this_turn = 1;           /* live.rs:1166 */
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

/* Mirror live.rs::log_performance_summary (live.rs:1258-1289) — the debug-only
   per-snapshot rule-log line:

       for snap in &game_state.performance_snapshots {
           let player = fmt_player_id(&snap.player_id);
           let mut live_details = String::new();
           for (i, live) in snap.lives.iter().enumerate() {
               let live_result = if live.passed { "PASS" } else { "FAIL" };
               if i > 0 { live_details.push_str(", "); }
               ... format_args!("live score+{} → {}", live.score, live_result)
           }
           let perf_result = if snap.success { "PASS" } else { "FAIL" };
           let detail_str = if live_details.is_empty() { String::new() }
                            else { format!(" [{}]", live_details) };
           GameState::push_rule_log_to(&mut game_state.rule_log, format!(
               "[Turn {}] {} [[log_performance:score={},result={}]]{}",
               snap.turn, player, snap.total_score, perf_result, detail_str,
           ));
       }

   Gated on ABILITY_DEBUG, which the C carries as rb_ability_debug_enabled()
   (ability/debug.c:8) and which is FALSE by default, so this is a no-op for
   every suite that does not opt in. That is the faithful reading of Rust's
   `if !ABILITY_DEBUG.load(..) { return; }` — it is a diagnostic, not a verdict.

   This is the ONLY caller of rb_fmt_player_id (live.rs:3185-3197, whose only
   Rust call site is live.rs:1263 inside this function), so wiring it is what
   makes that exported twin reachable from production code. The C player id is a
   seat index rather than Rust's "player1"/"player2" string; rb_fmt_player_id
   takes that seat index as its digits, so 0 → "P0" / 1 → "P1", which keeps the
   line's "P<n>" shape and the turn/score/result fields exactly as Rust emits
   them.

   The sink is rb_log_push_verdict(line, "rule_log", 1) — the same C rule-log
   ring rb_gsc_push_rule_log feeds (core/game_state_core.c:712-714), which is what
   GameState::push_rule_log_to is in this port. */
static void live_log_performance_summary(GameState *g) {
    if (!rb_ability_debug_enabled()) return;          /* live.rs:1259-1261 */
    for (int si = 0; si < g->n_snapshots; si++) {
        RbLiveSnapshot *s = &g->snapshots[si];
        char player[16];
        char seat_no[8];
        snprintf(seat_no, sizeof(seat_no), "%d", s->player);
        rb_fmt_player_id(seat_no, player, sizeof(player));
        char details[512];
        int off = 0;
        details[0] = 0;
        for (int i = 0; i < s->n_lives && i < RB_MAX_LIVE_CARDS; i++) {
            const char *res = s->live_passed[i] ? "PASS" : "FAIL";
            int n = snprintf(details + off, sizeof(details) - (size_t)off,
                             "%slive score+%d → %s", i > 0 ? ", " : "",
                             s->live_score_detail[i], res);
            if (n < 0) break;
            off += n;
            if ((size_t)off >= sizeof(details) - 1) break;
        }
        const char *perf_result = s->success ? "PASS" : "FAIL";
        char detail[530];
        if (details[0]) snprintf(detail, sizeof(detail), " [%s]", details);
        else detail[0] = 0;
        char line[768];
        snprintf(line, sizeof(line),
                 "[Turn %d] %s [[log_performance:score=%d,result=%s]]%s",
                 s->turn, player, s->total_score, perf_result, detail);
        rb_log_push_verdict(line, "rule_log", 1);
    }
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

/* Mirror live.rs::enrich_from_applications (live.rs:3032-3100), restricted to the
    half whose target fields the C now HAS.

    Rust's body has four effects:
      1. mc.ability_heart_bonuses / ability_blade_bonuses.push(...) for the member
         matching app.target_card_id           (live.rs:3043-3066)
      2. breakdown.scores.push / breakdown.transforms.push   (:3067-3084)
      3. triggered_abilities.push(...) for each new (source_card_id,
         ability_text) pair                        (:3085-3098)
      4. (not here) finalize_snapshot_fields later derives
         mc.transform_delta[i] = bonus_hearts[i].saturating_sub(ability_per_color[i])
         from the vectors effect 1 built          (live.rs:783-800)

    (2) and (3) are pure DISPLAY projections onto PerformanceSnapshot.breakdown /
    .triggered_abilities, and RbLiveSnapshot still has neither field, so those two
    remain unwritten: there is nowhere to put them, and the header owner has not
    added them. See the BLOCKED-ON note in PARITY_CHECKLIST.md I1.

    Effects (1)+(4) ARE landable and are landed here, because their output field
    already exists: `RbMemberContribution.transform_delta` (rabuka.h:1283) and
    `RbLiveSnapshot.member_contributions` (rabuka.h:1348, A18). The C has no
    `AbilityBonus` vector to push into — the header deliberately does not carry
    one — so effect (1) is not materialised as a list; it is CONSUMED directly
    into the per-colour accumulator that effect (4) needs, which is the same
    arithmetic on the same data. `sourrce`/`ability_text` (the AbilityBonus
    payload Rust stores) are debug-only labels gated on ABILITY_DEBUG
    (bonus_source, live.rs:3116-3127) and have no reader, so they are not
    reconstructed.

    The applications themselves are the C's trace ring
    (`RbMods.trace` / `n_trace`, rabuka.h:264-265), which is the fold of Rust's
    `GameState::ability_applications` and is what
    rb_gsc_record_ability_application (game_state_core.c:787) pushes into. The
    ring is read as a plain array: modifiers.c exports only
    rb_mods_trace_len, and that file is not this one's to edit.

    The per-performer SPLIT that phases.rs:556-582 performs before calling
    enrich_from_applications is not needed for effects (1)+(4): every application
    that names one of this performer's stage members as its target already
    belongs to that member, so filtering by member id selects the same
    applications the split would. That split exists so the OTHER player's
    snapshot is not enriched from the first performer's applications, and a
    member card id is on exactly one stage, so `source_id == target_card_id`
    cannot cross seats.

    Signature changed from `const GameState *` to `GameState *`: the port writes
    the snapshot. No prototype existed in any header (the only mentions were in
    comments), so nothing else could break. */
void rb_enrich_from_applications(GameState *g) {
    if (!g) return;
    for (int pl = 0; pl < 2; pl++) {
        int n = s_n_member_contrib[pl];
        if (n < 0) n = 0;
        if (n > RB_STAGE_SIZE) n = RB_STAGE_SIZE;
        for (int i = 0; i < g->n_snapshots; i++) {
            RbLiveSnapshot *s = &g->snapshots[i];
            if (s->player != pl) continue;
            s->n_member_contributions[pl] = n;
            for (int m = 0; m < n; m++) s->member_contributions[pl][m] = s_member_contrib[pl][m];
            /* live.rs:783-800 — for each member, ability_per_color[c] is the sum
               of every HeartBonus application's amount for that member's colour,
               then transform_delta[c] = bonus_hearts[c].saturating_sub(that). */
            for (int m = 0; m < n; m++) {
                RbMemberContribution *mc = &s->member_contributions[pl][m];
                int ability_per_color[8] = {0,0,0,0,0,0,0,0};
                for (int e = 0; e < g->mods.n_trace; e++) {
                    const RbAbilityTraceEntry *te = &g->mods.trace[e];
                    if (te->target_card_id != mc->source_id) continue;
                    if (te->effect_type != RB_EFFECT_HEART_BONUS) continue;
                    if (te->heart_color < 0 || te->heart_color >= 8) continue;
                    ability_per_color[te->heart_color] += (te->amount < 0) ? -te->amount : te->amount;
                }
                for (int c = 0; c < 8; c++) {
                    int bh = mc->bonus_hearts[c];
                    int apc = ability_per_color[c];
                    int d = bh - apc;
                    if (d < 0) d = 0;              /* saturating_sub */
                    if (d > 255) d = 255;
                    mc->transform_delta[c] = (uint8_t)d;
                }
            }
            break;                                 /* Rust takes one snapshot per seat per turn */
        }
    }
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

/* Mirror live.rs::record_pretrigger_live_results (live.rs:213-384) — BUG#5
   fix: each seat's ライブ成功時 outcome is recorded BEFORE the trigger block
   runs, so a condition like Strawberry Trapper's
   「相手が余剰のハートを持たずにライブを成功させていた場合」 sees real state.

   The `no_excess` term is faithful: Rust (live.rs:317-320) is
   `snap.total_hearts[c] >= ft && snap.total_hearts[c] - ft == 0` for every
   colour, i.e. exact equality, which is what the loop below tests.

   TWO RUST TERMS ARE ABSENT BY STRUCTURE, not by oversight:
     * `won = all_passed && pre_score > 0` (live.rs:336). The score half needs
       `calculate_live_score` over the PRE-trigger modifier flat map, which this
       port represents as `g->live_pre_score[pl]` — and that map is not populated
       yet when this runs (rb_perform_live captures it at :520-525, but that is
       skipped on the `live_batch_mode` path's early return... it is in fact
       captured before this point; the real gap is that the C has no
       `seat_cheer_count` / `seat_score_bonus` accessors to feed the second
       argument). Omitting it makes `won` equal `all_passed`. For the
       no_excess flag, which is what this function is FOR, the two agree: a seat
       with no excess hearts but total score 0 has no surplus only if it filled
       nothing, and `live_passed[]` is already false in that case. For
       `g->live_success[pl]` — which rb_determine_live_winners also reads — Rust
       does NOT use the score term either (it re-derives all-passed from the
       snapshot), so the winner decision is unaffected.
     * the `snap.lives.is_empty() || seat.live_card_zone.cards.is_empty() =>
       continue` skip (live.rs:240-242). The C iterates snapshots, and every
       snapshot this port pushes was pushed by rb_perform_live AFTER a non-empty
       live-zone check, so `s->n_lives > 0` already implies Rust's condition on
       every path that reaches here. */
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

/* Mirror live.rs::drain_pending_live_success_choices (live.rs:387-407):

       game_state.process_pending_auto_abilities(p1_id);
       if game_state.has_pending_choice() { return true; }
       game_state.process_pending_auto_abilities(p2_id);
       if game_state.has_pending_choice() { return true; }
       false

   The C collapses both seat passes into one queue drain and leaves the
   mid-function pending check to the caller, which does perform it —
   rb_execute_live_victory_determination does
   `rb_drain_pending_live_success_choices(g); if (rb_has_pending_choice(g)) return;`
   (the C spelling of Rust's `if Self::drain_pending_live_success_choices(..) {
   return; }` at live.rs:1354-1356). The seat-by-seat early return INSIDE the
   helper is not reproduced because the C queue is not seat-partitioned:
   rb_drain_ability_queue (ability/ability_queue.c:1315) resolves whatever is
   queued and stops on a pending choice itself, whereas Rust's per-player_id
   passes are the ones that keep a p2 entry from being resolved during p1's
   pass — a difference the per-seat block in rb_resolve_live_success_side
   (which calls rb_process_player_abilities(g, pl), the seat-scoped variant)
   already handles. */
void rb_drain_pending_live_success_choices(GameState *g) {
    if (!g) return;
    rb_drain_ability_queue(g);
}

/* Mirror live.rs::side_score_extra (live.rs:1170-1190):

       let mut total = 0i32;
       for &cid in live_zone {
           let current = game_state.mods.score_modifiers.get(&cid).map_or(0, |e| e.total());
           total += i32::from(current.saturating_sub(pre_score_flat.get(&cid).copied().unwrap_or(0)));
       }
       crate::constants::saturate_u8(total)

   `g->live_pre_score[pl]` IS the C's `pre_score_flat` (captured by
   rb_perform_live from rb_mods_get_score, standing in for
   snapshot_score_flats / live.rs:1089-1117), and rb_score_delta_since is the
   faithful exported twin of that same loop — it was UNREACHABLE because this
   function re-implemented it inline. Delegating makes the twin the single
   definition, exactly as Rust has one. The `current` side is the live modifier
   total per card id (mods.score[cid].total(), i.e. rb_mods_get_score); only
   the ids actually in this seat's live zone are ever read back, so leaving the
   rest of the array zero matches Rust's `map_or(0, ..)` for a missing entry. */
static int s_score_current[RB_MAX_CARD_IDS];

static int live_score_extra_for_side(const GameState *g, int pl) {
    if (!g->live_pre_valid[pl]) return 0;
    for (int i = 0; i < g->p[pl].live.n; i++) {
        int cid = g->p[pl].live.cards[i];
        if (cid < 0 || cid >= RB_MAX_CARD_IDS) continue;
        s_score_current[cid] = rb_mods_get_score((RbMods *)&g->mods, cid);
    }
    int total = rb_score_delta_since(s_score_current, g->live_pre_score[pl],
                                     g->p[pl].live.cards, g->p[pl].live.n);
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

/* ── I1: `check_live_success` and `build_snapshot` — DELETED 2026-10-03 ────────
   Both C names below used to exist as ZERO-CALLER bodies that were explicitly
   NOT the Rust ports, and both are now gone rather than kept as placeholders.
   They are deleted, not stubbed, because a live Rust function's name sitting in
   the C tree attached to a wrong body is a trap: the next reader either calls it
   (wrong verdict / blanked snapshot fields) or assumes the port is missing. There
   was no prototype in include/rabuka.h and no caller in src/ or tests/ for either
   name, so deletion is complete.

   WHERE THEIR PORTS ACTUALLY ARE:

     check_live_success (live.rs:2879, called at phases.rs:492)
         → src/turn/phase.c:1424 `rb_perform_live(g, performer)`, which FUSES the
           phases.rs:373-527 span (player_perform_live + check_live_success) and
           the phases.rs:621-641 span (build_snapshot + push_performance_snapshot).
           Every branch of Rust's check_live_success runs there: the recomputed
           heart pool (allocate_and_verdict's `total_hearts`), the recomputed
           allocation and backtrack fallback, the per-card acceptance verdict
           (rb_populate_live_verdicts), the failed-live -> waitroom drain at the
           tail of rb_perform_live, and `player.last_resolution_cards` (mirrored
           by g->revealed_cards). The old C body summed `card.num_need` against
           the flat `P->hearts[]` pool and ignored per-colour acceptance,
           icon_all, the modifier layers and both drains — and calling it would
           have double-drained.

     build_snapshot (live.rs:3130, called at phases.rs:621)
         → the INLINE push at the end of rb_perform_live (lives, per-live
           required, yell_cards, yell_blade_hearts, total_hearts, total_score,
           success, surplus, note_icons, live_passed), which is then revised in
           place by the post-trigger re-evaluation. The old C body was a narrower
           restatement — it copied `g->p[pl].hearts` into total_hearts (Rust uses
           perf.total_hearts) and left yell_cards, n_yell_cards, live_required,
           live_passed and surplus at zero, so calling it INSTEAD of the inline
           push would have blanked four fields the C pipeline reads
           (rb_populate_live_verdicts reads live_required/live_filled;
           rb_compute_surplus_and_flags reads success/total_hearts/live_filled).

     enrich_from_applications (live.rs:3034, called at phases.rs:583)
         → NOT deleted. Its body is landed and wired at the snapshot push; see its
           definition above. Two of its four effects (breakdown.scores,
           breakdown.transforms, triggered_abilities) remain unwritable because
           RbLiveSnapshot has no such fields. */

/* Mirror live.rs::handle_live_success_choice (live.rs:1788-1828) — handle the
   result of a live success zone card selection:

       let card_id = player.live_card_zone.cards.remove(selected_index);
       player.success_live_card_zone.cards.push(card_id);
       let remaining = core::mem::take(&mut player.live_card_zone.cards);
       player.waitroom.cards.extend(remaining);
       let success_count = player.success_live_card_zone.cards.len();
       Self::check_victory_condition(game_state);

   The drain is UNCONDITIONAL (outside any win guard) — same rule as
   process_player_live_result — and the drain only reaches the waitroom on a
   non-skipped live, which the `can_place` early return already encodes.
   check_victory_condition is the load-bearing tail: without it a player who
   reaches the victory threshold through the SUCCESS-ZONE CHOICE (a multi-live
   seat picking which live to place) would never have the win recorded, because
   rb_check_timing — the other caller in this port, turn/phase.c:586 — does not
   run again inside a live window. */
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
    /* live.rs:1814 */
    rb_check_victory_condition(g);
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
        /* live.rs:1202-1206 — `if !live_success_triggered_this_turn { set it;
           compute_snapshot_surplus(game_state, player2_id); }`, i.e. exactly once
           per victory determination, BEFORE the first ライブ成功時 block. The C
           guard is `live_victory_stage == 0`, which IS this determination's first
           entry: the field is only reset to 0 after move_to_success_and_update_
           attacker completes (below) and on the resume-from-stage-3 entry. */
        rb_compute_snapshot_surplus(g);
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
    /* live.rs:1415 — `Self::log_performance_summary(game_state)` sits between
       compute_surplus_and_flags and move_to_success_and_update_attacker. */
    live_log_performance_summary(g);
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
   color = entry%8.

   ── I4: IS THIS ENCODING LOSSLESS AGAINST EVERY CONSUMER? MEASURED ──────────
   Rust's `Allocation` (live.rs:34-55, types.rs:686-698) carries ten fields:
   target_idx, target_name, source_type, source_name, source_slot, wildcard,
   color, amount, is_bonus, phase. A repo-wide search of engine/src finds the
   `allocations` vector read at exactly FOUR places, all in live.rs:

     live.rs:254   record_pretrigger_live_results — target_idx, color, amount
     live.rs:548   populate_live_verdicts         — target_idx, color, amount
     live.rs:720   finalize_snapshot_fields       — target_idx, PHASE, amount
     live.rs:1153  compute_snapshot_surplus       — color, amount

   So the seven fields with no consumer anywhere in the engine are
   target_name / source_type / source_name / source_slot / wildcard / is_bonus
   (plus `phase` outside the one consumer below). Dropping them is lossless.

   The first three consumers read `alloc.color` RAW. `card*8+color` is injective
   over (card_index ∈ [0, RB_MAX_LIVE_CARDS), color ∈ [0,8)) and amount is the
   repetition count, so the encoded stream reproduces (target_idx, color, amount)
   exactly for all three. rb_populate_live_verdicts and rb_compute_snapshot_surplus
   consume `live_filled[i][c]`, which is that same raw bucket. LOSSLESS.

   THE ONE LOSSY SPOT IS live.rs:720. finalize_snapshot_fields needs `phase` to
   remap each allocation into a SOURCE bucket:

       H00Wild | Wildcard                     -> 0
       AllWild | CAll | AllCleanup             -> 7
       _                                       -> alloc.color

   Greedy and backtracking only ever CONSTRUCT four phases — Colored (:2329,
   :2549), ColoredSurplus (:2369, :2631), H00 (:2389, :2693) and AllCleanup
   (:2411, :2433, :2811) — so H00Wild / Wildcard / AllWild / CAll are dead
   variants and the table reduces to: AllCleanup -> 7, H00 -> 0, Colored and
   ColoredSurplus -> alloc.color. The C cannot reproduce that, because
   `live_filled[i][c]` is the raw bucket and Phase 4 records an icon_all fill of
   colour c as bucket c, indistinguishable from a Phase-1a native heart of that
   colour. That is where the encoding is lossy, and it is lossy ONLY for
   `spare` (live.rs:741), which is itself write-only in Rust (no reader
   anywhere in engine/src). Hence it stays blocked on the `live_spare` header
   field named on rb_finalize_snapshot_fields above. */
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
           /* live.rs:2717 — `if Self::card_ok_with_wildcard(filled, need)`. The
              exported C twin is called here (not the file-local bt_card_ok) so
              that rb_card_ok_with_wildcard — declared in include/rabuka.h:2036 —
              is reachable from production code; it is a null-checking delegate
              to bt_card_ok, so the verdict is identical. */
           if (rb_card_ok_with_wildcard(new_filled, need)) {
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
          /* live.rs:2777 — same exported twin as in bt_try_phase4 above. */
          if (rb_card_ok_with_wildcard(filled, need)) {
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
   Delegates to bt_try_surplus for the actual enumeration.

   ⚠ ZERO CALLERS, and not deletable. Rust's `try_surplus_compositions`
   (live.rs:2594) IS live — bt_search calls it at live.rs:2572 — and its
   faithful C counterpart is the file-local bt_try_surplus above, which
   rb_backtrack_allocate -> bt_search reaches. This exported alias adds no
   behaviour: it is the same call with max_allocs pinned to 64, so it cannot
   stand in for the threaded buffer bt_search carries. Kept unreferenced as the
   named counterpart. */
int rb_try_surplus_compositions(int *pool, const int card_needs[8], int n_cards,
                                 int idx, const int *colors, int n_colors,
                                 int remaining, int color_idx,
                                 int *allocs, int *filled) {
     if (!pool || !card_needs || !colors || !allocs || !filled) return 0;
     int n_allocs = 0;
     return bt_try_surplus(pool, card_needs, n_cards, idx, allocs, &n_allocs,
                            64, colors, n_colors, remaining, color_idx, filled);
}

/* Mirror live.rs::card_ok_with_wildcard (live.rs:2846-2871) — check whether a
   single card's heart requirements are satisfied given its filled array.
   Implements canonical acceptance rules: total coverage, heart0 bucket,
   per-color deficits.

   REACHABLE: called from the two faithful backtracking verdicts —
   bt_try_phase4 (live.rs:2717) and bt_try_all_distribution (live.rs:2777).
   Rust's call sites are exactly those two. */
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

   ⚠ ZERO CALLERS, and NOT the C counterpart of the Rust item. Two corrections
   to what this comment used to claim:
     * this symbol is NOT declared in include/rabuka.h — nothing outside this
       translation unit has ever seen it, and a repo-wide search finds no other
       reference to the name at all;
     * the signature is not "the exported C signature declared in
       include/rabuka.h" for a Rust signature that already exists — Rust's
       `fn bt_search(pool, card_needs, idx, allocs)` takes only 4 arguments and
       this one takes 6, two of which (`g`, `pl`) are pure noise.
   The faithful counterpart is the file-local `bt_search` above, which
   rb_backtrack_allocate drives and which allocate_and_verdict calls whenever
   the greedy allocation fails. Rust's `bt_search` (live.rs:2517) is live for the
   same reason, so nothing is deleted here — this is a redundant entry point,
   not a dead Rust item. */
int rb_bt_search(const GameState *g, int pl, int *pool, int *card_needs,
                 int n_needs, int idx) {
    (void)g; (void)pl;
    if (!pool || !card_needs || n_needs <= 0) return 0;
    if (idx < 0 || idx >= n_needs) return 0;
    int scratch[64];
    int n_allocs = 0;
    return bt_search(pool, card_needs, n_needs, idx, scratch, &n_allocs, 64);
}

/* Mirror live.rs::try_phase4 (live.rs:2660-2750) — after the Phase 3a surplus
   choices, apply Phase 3b (COLORLESS heart00 pool -> the card's remaining
   heart00/total deficit only, never a specific colour) and then Phase 4
   (icon_all, pool slot 7, -> the still-unmet per-colour deficits, then the
   heart00 bucket). Returns 1 once the card's requirement array is satisfied.

   ⚠ ZERO CALLERS, NOT DECLARED IN include/rabuka.h, AND NOT FAITHFUL — the body
   below is a single-card approximation, and it must not be wired. Rust's
   `try_phase4` receives the whole `pool`, the whole `card_needs` slice and the
   allocation out-param, then (a) enumerates icon_all splits through
   try_all_distribution and (b) recurses into bt_search for the NEXT card
   (live.rs:2718 / :2731 / :2778). This signature carries `g`, `pl`, `filled` and
   `need` and NOTHING else: no pool, no remaining cards, no deficit list, no
   recursion — so the two structural steps above are inexpressible here. The
   faithful body is the file-local bt_try_phase4, reached from
   rb_backtrack_allocate -> bt_search -> bt_try_surplus. Rust's `try_phase4`
   (live.rs:2660) is live via try_surplus_compositions, so this duplicate stays
   in the tree unreferenced. A faithful exported form needs a header change
   (the real parameter list), which is the header owner's call. */
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

/* live.rs::try_all_distribution (live.rs:2754-2840) enumerates every way the
   remaining icon_all hearts can be split across a card's deficit buckets. Every
   one of its parameters is data (the live pool, the already-filled array, the
   deficit colour/amount lists, the remaining icon_all count, the deficit cursor)
   and NONE of them is present in this signature — only (game, pl). There is
   therefore no faithful body that can be written against it; the real port is
   the file-local bt_try_all_distribution above, which
   rb_backtrack_allocate -> bt_search -> bt_try_surplus -> bt_try_phase4 drives.

   ⚠ TWO CORRECTIONS to what this comment used to claim:
     * this symbol is NOT declared in include/rabuka.h, contrary to the old
       wording — nothing outside this file has ever referenced it, and a
       repo-wide search finds no other mention of the name;
     * Rust's `try_all_distribution` (live.rs:2754) IS live — bt_try_phase4 calls
       it at live.rs:2731 — so this is a redundant entry point, NOT a dead Rust
       item, and it is deliberately not deleted.

   Rather than return a hard-coded 1 (which would assert "a distribution was
   found" without having looked at anything), report that no distribution was
   produced. */
int rb_try_all_distribution(const GameState *g, int pl) {
    if (!g) return 0;
    (void)pl;
    return 0;
}
