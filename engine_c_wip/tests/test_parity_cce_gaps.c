/* test_parity_cce_gaps.c — C parity suite for the UNCOVERED part of the Rust
 * cluster
 *   engine/tests/test_modules/effects/compound/cost_and_effect/
 *
 * Scope. test_parity_cost_and_effect.c and test_parity_condition_and_effect.c
 * already cover the compound cost/effect engine SURFACE of this cluster. What
 * was still uncovered is the PER-CARD BEHAVIOUR of a further sixteen Rust
 * files. This file ports those, one block per Rust file, citing the governing
 * .rs above each block:
 *
 *   A. nahone_live_start_same_name_test.rs            (21 tests)
 *   B. live_start_optional_live_discard_chosen_heart_q190_test.rs   (4)
 *   C. live_start_optional_unit_discard_draw_unit_cost_gain_test.rs (2)
 *   D. live_start_optional_bottom_mill_named_recovery_source_test.rs(1)
 *   E. discard_score_energy_live_recovery_edges_test.rs             (3)
 *   F. live_start_optional_unit_position_change_heart_blade_test.rs  (4)
 *   G. live_start_discard_reactivate_other_member_both_heart04_test.rs (5)
 *   H. live_start_energy_under_member_draw_all_blades_q157_q158_q184 (5)
 *   I. self_discard_same_name_same_area_deploy_energy_under_q157_q184 (4)
 *   J. live_start_keep_or_discard_live_then_reveal_to_gain_blade_test  (8)
 *   K. live_success_success_count_scales_cost_pl_pb1_007_test.rs      (8)
 *
 * HARNESS NOTES (all measured / confirmed by peers, all live here):
 *   - rb_load("src") FAILS under the isolated out-of-tree build, so
 *     load_card_db() falls back to ../cards/build (and cards/build).
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE. This
 *     file NEVER calls it. Every colour is read through rb_mods_get_heart /
 *     rb_mods_get_need_heart and every colour INDEX is derived from
 *     rb_parse_heart_color("heartNN"); the mapping is printed at startup from
 *     the engine's own rb_heart_color_as_str().
 *   - test_id() is the SHARED TEMPLATE and aliases; the Rust helper's id() and
 *     new_id() BOTH hand out a distinct pool slot, so every fixture here goes
 *     through test_new_id (mid()). Two copies of one print must be two
 *     distinct ids or every same-name / per-area assertion is vacuous.
 *   - rb_card_copy_pool_reset() runs per case (fresh()): the pool is a
 *     process-global monotonic allocator nothing else resets.
 *   - rb_card_no_eq() is FUZZY (it normalises U+FF0B and tolerates a missing
 *     plus). Identity is pinned by a byte-exact strcmp of
 *     rb_card_string(c.card_no_idx) instead (pin_id()).
 *   - rb_card_string() takes a STRING-TABLE INDEX, not a card id.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - Card.ability is NOT populated by rb_decode_card_by_index; abilities go
 *     through rb_card_num_abilities + rb_decode_card_ability.
 *   - All TestGame objects are static: sizeof(GameState) is ~781 KB.
 *   - C's deck bag is TOP-INDEX-0, so a Rust cards.push() is a plain append.
 *   - test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) destroys the yell
 *     window, so every walk here steps test_pass() one phase at a time and
 *     STOPS on a standing prompt rather than answering it blindly.
 *
 * VACUITY CONTROL. Every negative claim in this file (a buff that must NOT
 * appear, a prompt that must NOT be raised) is paired either with
 * CHECK_PROMPTS(n) - counting the prompt still ON SCREEN at the assertion
 * site - or with a positive control on the same board, so a green negative is
 * evidence and not the silence of a no-op.
 *
 * There is deliberately NO watchdog: not achievable on this toolchain. run()
 * is crash ISOLATION. Every drain is bounded by an explicit cap.
 *
 * This file is a TEST file. It never edits engine code, and no assertion is
 * weakened to make the suite green: when the engine is wrong the check stays
 * red and the failure line names the root cause.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as a byte escape (never as a literal: the
 * editor can emit CP932 and silently corrupt it). */
#define FP "\xef\xbc\x8b"

/* ------------------------------------------------------------------ *
 * fixtures
 * ------------------------------------------------------------------ */
/* A. 百生 吟子 (Nahone) — ライブ開始時 optional hand-discard, then the
 *    same-name member gains heart04 + blade until live end. */
#define NAHONE_RP  "PL!HS-bp2-007-R" FP     /* 百生 吟子, member, cost 11 */
#define NAHONE_P   "PL!HS-bp2-007-P"        /* 百生 吟子, same ability   */
#define NAHONE_PP  "PL!HS-bp2-007-P" FP     /* 百生 吟子, same ability   */
#define NAHONE_SEC "PL!HS-bp2-007-SEC"      /* 百生 吟子, same ability   */
#define NAHONE_PR  "PL!HS-PR-007-PR"        /* 百生 吟子, PR print c9    */
#define HONOKA     "PL!-sd1-010-SD"          /* 高坂 穂乃果, filler c4    */
#define LIVE_M     "PL!-sd1-020-SD"          /* きっと青春が聞こえる s2  */
#define LIVE_1     "PL!-sd1-019-SD"          /* START:DASH!! s1           */
#define LIVE_SOME  "PL!-sd1-021-SD"          /* これからのSomeday         */
/* B. ミア・テイラー Q190 */
#define MIA        "PL!N-bp4-011-R" FP
/* C. 村野さやか DOLLCHESTRA cost modifier */
#define SAYAKA     "PL!HS-bp6-010-R"         /* 村野さやか, DOLLCHESTRA  */
#define KOTSU      "PL!HS-PR-008-PR"         /* 徒町 小鈴, DOLLCHESTRA   */
#define TSUYURI    "PL!HS-bp1-013-N"         /* 夕霧綴理, DOLLCHESTRA    */
#define KOTO       "PL!HS-bp1-012-PR"        /* 乙宗 梢, スリーズブーケ  */
/* D. 小原鞠莉 bottom mill */
#define MARI       "PL!S-bp7-008-R"          /* 小原鞠莉                  */
#define YOSHIKO    "PL!S-bp7-006-R"          /* 津島善子 (top marker)     */
#define KANON      "PL!SP-sd1-001-SD"        /* 澁谷かのん   (bottom 3rd) */
#define ANZE       "PL!SP-sd1-003-SD"        /* 嵐 千砂都   (bottom 2nd)  */
#define SUMIRE_SD  "PL!SP-sd1-004-SD"        /* 平安名すみれ (bottom-most) */
/* E. 桜坂しずく Q214 */
#define SHIZUKU_R  "PL!N-bp5-003-R"
#define SHIZUKU_P  "PL!N-bp5-003-P"
#define SHIZUKU_AR "PL!N-bp5-003-AR"
/* F. 安養寺姫芽 live-start position change */
#define HIMENO6    "PL!HS-pb1-006-R"         /* 安養寺姫芽 c11            */
#define HIMENO_SD  "PL!HS-sd1-014-SD"        /* 安養寺 姫芽 c9  mirakura  */
#define HIMENO_SD2 "PL!HS-sd1-006-SD"        /* 安養寺 姫芽 c15 mirakura  */
/* G. エマ・ヴェルデ live-start reactivation */
#define EMMA       "PL!N-bp3-008-R" FP
#define NIJI_A     "PL!N-sd1-010-SD"
#define NIJI_B     "PL!N-sd1-011-SD"
/* H. 上原歩夢 Q157/Q158/Q184 */
#define AYUMU      "PL!N-bp3-001-R" FP
#define ENERGY_C   "LL-E-001-SD"
#define HONOKA_13  "PL!-sd1-013-SD"
/* I. 優木せつ菜 self-discard same-name deploy */
#define SETSUNA    "PL!N-bp3-007-R"
#define SETSUNA_PR "PL!N-PR-009-PR"          /* 優木せつ菜, c2             */
/* J. ツナガルコネクト keep-or-discard */
#define TSUNAGARU  "PL!N-bp3-028-L"
#define NAGI       "PL!N-bp1-001-R"          /* 上原歩夢, Nijigasaki       */
#define MUS_LIVE   "PL!-bp3-019-L"
/* K. 東條 希 PL!-pb1-007-R success-scaled cost */
#define CHIKA_PB1  "PL!-pb1-007-R"
#define LILY_PB3   "PL!-bp3-014-N"
#define SUCC_LIVE  "PL!N-bp1-025-L"
#define LIELLA_LV  "PL!S-bp2-024-L"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ------------------------------------------------------------------ *
 * reporting
 * ------------------------------------------------------------------ */
static int failures;
static int assertions;
static int setup_bugs;
static int gaps;
static int prompts;
static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB: every TestGame is a static (BSS) object. */
static TestGame t_a, t_b, t_c, t_d, t_e, t_f, t_g, t_h, t_i, t_j, t_k,
              t_l, t_m, t_n, t_o, t_p, t_q, t_r, t_s, t_u, t_v, t_w, t_x,
              t_y, t_z;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures: %d, "
            "prompts: %d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        assertions++;                                                     \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

/* A Rust assertion with no reachable C entry point. Counted, never a pass and
 * never a failure - it stands in for no real CHECK. */
#define EXPECTED_GAP(condition, ...) do {                                \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            printf("GAP: ");                                              \
            printf(__VA_ARGS__);                                          \
            printf("\n");                                                 \
            gaps++;                                                       \
        }                                                                 \
    } while (0)

/* VACUITY CONTROL: the board must have produced at least `n` real prompts
 * before a negative claim can be trusted. The currently-pending prompt is
 * counted HERE, at the assertion site, because counting only at answer time
 * would report 0 for a prompt that is plainly open. */
#define CHECK_PROMPTS(tg_arg, n, ...) do {                                 \
        assertions++;                                                     \
        if (pending(tg_arg)) prompts++;                                   \
        if (prompts < (n)) {                                              \
            failures++;                                                   \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) were offered, need %d\n",\
                    prompts, (n));                                        \
        }                                                                 \
    } while (0)

/* ------------------------------------------------------------------ *
 * database / fixture helpers
 * ------------------------------------------------------------------ */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* A fresh game with the process-global copy pool reset, so a long suite cannot
 * exhaust it and silently fall back to the shared template. */
static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

/* ROTATING buffer. A single shared static buffer called 2-3 times inside one
 * printf prints the LAST value three times - a proven false-red generator. */
static const char *card_no_of(int cid)
{
    static char ring[8][96];
    static int  next;
    char *buf = ring[next];
    next = (next + 1) % 8;
    buf[0] = 0;
    Card c;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 96, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static const char *card_name_of(int cid)
{
    static char ring[8][96];
    static int  next;
    char *buf = ring[next];
    next = (next + 1) % 8;
    buf[0] = 0;
    Card c;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, 96, "%s", c.name);
        rb_free_card(&c);
    }
    return buf;
}

/* Rust `game.id("X")` and `game.new_id("X")` BOTH hand out a fresh pool slot,
 * so the C twin is always test_new_id. test_id is the SHARED TEMPLATE and
 * would make two "copies" of a print the same object. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++;
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* AGENTS.md identity rule: pin with a BYTE-EXACT strcmp of the decoded
 * card_no, because rb_card_no_eq / rb_find_card_by_no are fuzzy. */
static int pin_id(int cid, const char *no)
{
    assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    setup_bugs++;
    failures++;
    fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' "
                    "(id=%d)\n", no, got, cid);
    return 0;
}

/* Resolve a fixture, pin its identity, return 0 on a setup bug. */
static int fx(TestGame *tg, int *out, const char *no)
{
    *out = mid(tg, no);
    if (*out < 0) return 0;
    return pin_id(*out, no);
}

static int card_cost(int cid)
{
    Card c;
    int v = -1;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int card_score(int cid)
{
    Card c;
    int v = -1;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}

static int card_is_live(int cid)
{
    return cid >= 0 && rb_card_is_live(cid);
}

/* ---- modifier readers. NEVER test_get_heart_modifier (colour remap). ---- */
static int mh(TestGame *tg, int cid, int color)
{ return rb_mods_get_heart(&tg->state.mods, cid, color); }

static int mh_at(TestGame *tg, int cid, const char *label)
{
    int idx = (int)rb_parse_heart_color(label);
    return rb_mods_get_heart(&tg->state.mods, cid, idx);
}

static int blade_of(TestGame *tg, int cid)
{ return rb_mods_get_blade(&tg->state.mods, cid); }

static int cost_of(TestGame *tg, int cid)
{ return rb_mods_get_cost(&tg->state.mods, cid); }

static int score_mod_of(TestGame *tg, int cid)
{ return rb_mods_get_score(&tg->state.mods, cid); }

static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o != NULL && strcmp(o, "wait") == 0;
}

/* The colour mapping the engine actually uses, printed at startup so a reader
 * never has to guess which integer heart04 is. */
static void print_heart_mapping(void)
{
    static const char *const names[7] = {
        "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"
    };
    printf("heart index map (from rb_heart_color_as_str):");
    for (int i = 0; i < 7; i++)
        printf(" %d->%s", i, rb_heart_color_as_str((int)rb_parse_heart_color(names[i])));
    printf("\n");
    printf("heart04 index = %d, heart02 index = %d, heart05 index = %d\n",
           (int)rb_parse_heart_color("heart04"),
           (int)rb_parse_heart_color("heart02"),
           (int)rb_parse_heart_color("heart05"));
}

/* ---- zone helpers ---- */
static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].deck, cid); }
static int live_has(TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].live, cid); }
static int energy_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].energy, cid); }

static int hand_n(TestGame *tg)   { return tg->state.p[0].hand.n; }
static int wait_n(TestGame *tg)   { return tg->state.p[0].discard.n; }
static int deck_n(TestGame *tg)   { return tg->state.p[0].deck.n; }
static int energy_n(TestGame *tg) { return tg->state.p[0].energy.n; }
static int under_n(TestGame *tg, int area) { return tg->state.p[0].under_cards[area].n; }
static int under_has(TestGame *tg, int area, int cid)
{ return bag_has(&tg->state.p[0].under_cards[area], cid); }

static int stage_at(TestGame *tg, int area) { return tg->state.p[0].stage[area]; }
static int p2_stage_at(TestGame *tg, int area) { return tg->state.p[1].stage[area]; }

static int stage_has(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

static void set_stage3(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

/* C's deck bag is TOP-INDEX-0, so a Rust cards.push() is a plain append. */
static void deck_append_n(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = cid;
}

static void set_hand(TestGame *tg, const int *ids, int n)
{
    RbPlayer *P = &tg->state.p[0];
    P->hand.n = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) P->hand.cards[P->hand.n++] = ids[i];
}

static void add_wait(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static void add_live(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    if (P->live.n < RB_MAX_ZONE) P->live.cards[P->live.n++] = cid;
}

static void add_success(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    if (P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++] = cid;
}

static void add_energy(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
}

static void clear_decks(TestGame *tg)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
}

static void bare_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_deck.n = 0;
        p->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            p->stage[i] = RB_EMPTY_SLOT;
            p->stage_wait[i] = 0;
            p->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_selected_cards = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_recently_moved = 0;
    tg->state.yell_occurred = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    tg->state.queue.deferred = NULL;
    tg->state.queue.resume_eff = NULL;
    tg->state.activation_keepalive_valid = 0;
    rb_clear_pending_choice(&tg->state);
}

/* Rust fill_decks: `n` filler cards onto BOTH main decks (index 0 = top). */
static void fill_decks(TestGame *tg, int filler, int n)
{
    deck_append_n(tg, 0, filler, n);
    deck_append_n(tg, 1, filler, n);
}

/* ---- choice plumbing ---- */

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

static const RbChoice *pending_ch(TestGame *tg)
{
    return test_has_pending_choice(tg) ? rb_get_pending_choice(&tg->state) : NULL;
}

static const char *prompt_shape(TestGame *tg)
{
    static char buf[256];
    const RbChoice *c = pending_ch(tg);
    if (!c) { snprintf(buf, sizeof buf, "(no pending choice)"); return buf; }
    snprintf(buf, sizeof buf,
             "kind=%d zone=\"%s\" target=\"%s\" count=%d allow_skip=%d "
             "n_filtered=%d desc=\"%s\"",
             (int)c->kind, c->zone, c->target, c->count, c->allow_skip,
             c->n_filtered_indices, c->description);
    return buf;
}

static void pump(TestGame *tg)
{
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
    if (test_has_pending_choice(tg)) return;
    if (tg->state.queue.has_pending || tg->state.queue.deferred) return;
    if (tg->state.activation_keepalive_valid) {
        rb_free_ability(&tg->state.activation_keepalive);
        tg->state.activation_keepalive_valid = 0;
    }
}

/* Rust select_indices(&[idx]) */
static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    prompts++;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
    pump(tg);
}

static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg) || n <= 0) return;
    prompts++;
    rb_resume_with_choice_indices(&tg->state, idx, n);
    pump(tg);
}

/* Rust select_indices(&[]) - the EMPTY selection, which is how Rust declines a
 * SelectCard. Deliberately NOT a Skip: a SelectCard SKIP is routed elsewhere
 * and never reaches a 「そうした場合」 gate. */
static void pick_empty(TestGame *tg)
{
    if (!pending(tg)) return;
    prompts++;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
    pump(tg);
}

/* Rust select_option(i) - the option index of a SelectTarget /
 * SelectHeartColor / SelectAutoAbility prompt. */
static void select_option(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    prompts++;
    rb_resume_with_choice(&tg->state, idx);
    pump(tg);
}

static void note_prompt(TestGame *tg) { if (pending(tg)) prompts++; }

/* Rust `drain_choices`: DECLINE everything that can be declined, answer what
 * cannot. A mandatory prompt left parked mid-ability produces exactly the
 * absence the following assertion checks, so nothing would ever fail. */
static void drain_choices(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (guard++ < guard_max) {
        if (test_has_pending_choice(tg)) {
            const RbChoice *c = rb_get_pending_choice(&tg->state);
            int skippable = c ? rb_choice_allow_skip(c) : 1;
            prompts++;
            if (skippable) pick_empty(tg);
            else           select_option(tg, 0);
            rb_drain_ability_queue(&tg->state);
            continue;
        }
        if (tg->state.queue.has_pending) { rb_drain_ability_queue(&tg->state); continue; }
        rb_queue_resume_pending_actions(&tg->state);
        rb_process_pending_auto_abilities(&tg->state);
        if (test_has_pending_choice(tg) || tg->state.queue.has_pending) continue;
        break;
    }
    rb_drain_ability_queue(&tg->state);
}

/* Rust `while has_pending_choice() { select_indices(&[0]) }` */
static void drain_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (guard++ < guard_max) {
        if (test_has_pending_choice(tg)) { pick(tg, 0); rb_drain_ability_queue(&tg->state); continue; }
        if (tg->state.queue.has_pending) { rb_drain_ability_queue(&tg->state); continue; }
        rb_queue_resume_pending_actions(&tg->state);
        rb_process_pending_auto_abilities(&tg->state);
        if (test_has_pending_choice(tg) || tg->state.queue.has_pending) continue;
        break;
    }
    rb_drain_ability_queue(&tg->state);
}

/* Rust `drain_auto_abilities`: answer a standing SelectAutoAbility with
 * option 0 and stop at anything else. */
static void drain_auto_abilities(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (!c || c->kind != RB_CHOICE_SELECT_AUTO_ABILITY) return;
        select_option(tg, 0);
    }
}

/* ---- live-start trigger firing ---- */

/* helpers/mod.rs:114 fire_trigger: build the "<card_no>_<full_text>" ability id
 * for the first ability printing `trig`, queue exactly that ability, publish
 * activating_card, process the queue. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

#define TRIG_LIVE_START "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe9\x96\x8b\xe5\xa7\x8b\xe6\x99\x82"

static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof card_no, "%s", rb_card_string(card.card_no_idx));
    rb_free_card(&card);

    char ability_id[512];
    int found = 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab && !found; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            snprintf(ability_id, sizeof ability_id, "%s_%s", card_no,
                     ab.full_text ? ab.full_text : "");
            found = 1;
        }
        rb_free_ability(&ab);
    }
    if (!found) return 0;
    rb_trigger_auto_ability(&tg->state, ability_id, trigger_type, 0, card_no,
                             cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static int fire_live_start(TestGame *tg, int cid)
{
    return fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START);
}

/* ---- phase walk ---- */

/* Rust advance_to_live_card_set_p1: five blind passes reach the live-card set
 * window. Each pass is followed by a bounded skip drain so a standing prompt
 * never silently gets answered with the wrong index. */
static void advance_to_live_set(TestGame *tg, int n_pass)
{
    for (int i = 0; i < n_pass; i++) {
        drain_choices(tg, 8);
        test_pass(tg);
    }
}

/* Rust finish_live_setup / advance_to_live_start: two more passes reach the
 * ライブ開始時 point. Deliberately NOT test_advance_to_phase, which answers
 * every standing prompt with index 0 and destroys the yell window. */
static void advance_to_live_start(TestGame *tg, int n_pass)
{
    for (int i = 0; i < n_pass; i++) {
        drain_choices(tg, 8);
        test_pass(tg);
    }
}

/* ------------------------------------------------------------------ *
 * diagnostic dump: RB_CCE_DUMP=<card_no>[,...] prints every decoded ability
 * of the named prints, so a red check can be classified as a PARSER gap (the
 * tree decodes wrong) versus an ENGINE gap (it decodes right and evaluates
 * wrong) without guessing. Purely diagnostic; no assertion reads it.
 * ------------------------------------------------------------------ */

static void dump_tree(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("action=%s src=%s dst=%s tgt=%s count=%d opt=%d",
           e->action ? e->action : "(null)",
           e->source ? e->source : "-", e->destination ? e->destination : "-",
           e->target ? e->target : "-", e->count, e->is_optional);
    for (int i = 0; i < e->n_extra; i++)
        printf(" [%s=%s]", e->extra_k[i] ? e->extra_k[i] : "?",
               e->extra_v[i] ? e->extra_v[i] : "?");
    if (e->condition) printf(" (cond v=%d)", (int)e->condition->variant);
    printf("\n");
    for (int i = 0; i < e->n_child; i++) dump_tree(e->child[i], depth + 1);
    if (e->optional_action)     { for (int i=0;i<depth;i++) fputs("  ", stdout); printf("OPTIONAL->\n");    dump_tree(e->optional_action, depth+1); }
    if (e->conditional_action)  { for (int i=0;i<depth;i++) fputs("  ", stdout); printf("CONDITIONAL->\n"); dump_tree(e->conditional_action, depth+1); }
    if (e->followup_action)     { for (int i=0;i<depth;i++) fputs("  ", stdout); printf("FOLLOWUP->\n");    dump_tree(e->followup_action, depth+1); }
    if (e->select_action)       { for (int i=0;i<depth;i++) fputs("  ", stdout); printf("SELECT->\n");      dump_tree(e->select_action, depth+1); }
    if (e->look_action)         { for (int i=0;i<depth;i++) fputs("  ", stdout); printf("LOOK->\n");        dump_tree(e->look_action, depth+1); }
}

static void dump_ability(const char *no, int idx)
{
    int id = rb_find_card_by_no(no);
    if (id < 0) { printf("DUMP %s ab#%d: NO SUCH CARD\n", no, idx); return; }
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)id, (uint32_t)idx, &ab)) {
        printf("DUMP %s ab#%d: DECODE FAILED\n", no, idx);
        rb_free_ability(&ab);
        return;
    }
    printf("DUMP %s (idx %d) ab#%d trig=%s\n  COST:\n", no, id, idx,
           ab.triggers ? ab.triggers : "-");
    dump_tree(ab.cost, 2);
    printf("  EFFECT:\n");
    dump_tree(ab.effect, 2);
    rb_free_ability(&ab);
}

static void dump_requested(void)
{
    const char *env = getenv("RB_CCE_DUMP");
    if (!env || !*env) return;
    char buf[512];
    snprintf(buf, sizeof buf, "%s", env);
    /* Hand-rolled comma split: strtok_r is not declared under -std=c11 here. */
    char *cursor = buf;
    while (cursor && *cursor) {
        char *comma = strchr(cursor, ',');
        if (comma) *comma = '\0';
        if (*cursor)
            for (int i = 0; i < 5; i++) dump_ability(cursor, i);
        cursor = comma ? comma + 1 : NULL;
    }
}

/* ================================================================== *
 * A. nahone_live_start_same_name_test.rs
 *
 * 百生 吟子 PL!HS-bp2-007-{R+,P,P+,SEC} ab#1
 *   ライブ開始時: 手札を1枚控え室に置いてもよい：これにより控え室に置いたカードが
 *   メンバーカードの場合、控え室に置いたカードと同じ名前を持つメンバー1人は、
 *   ライブ終了時まで、heart04 + ブレードを得る。
 *
 * abilities.json (cards/abilities.json, the shipped compile input) for this
 * ability:
 *   cost  = { type: move_cards, source: hand, destination: discard,
 *             count: 1, optional: true }
 *   effect= { action: sequential, same_name: true, heart_colors: [heart04],
 *             condition: { type: location_condition, location: discard,
 *                           card_type: member_card, target: self },
 *             actions: [ gain_resource blade 1 same_name target_count 1
 *                        live_end,
 *                        gain_resource heart heart04 1 same_name
 *                        target_count 1 live_end ] }
 *
 * cards.json evidence for the fixtures (read as UTF-8):
 *   PL!HS-bp2-007-R＋ / -P / -P＋ / -SEC : name 百生 吟子, メンバー, cost 11
 *   PL!HS-PR-007-PR                      : name 百生 吟子, メンバー, cost 9
 *   PL!-sd1-020-SD                       : name きっと青春が聞こえる, ライブ
 *
 * The SAME-NAME test is also the reason the identity premise is checked first:
 * a copy that carries the WRONG Card.name would make every distinct-NAME gate
 * in the game unreachable and every test below would pass or fail for the
 * wrong reason.
 * ================================================================== */

/* The Rust preamble shared by 15 of the 21 tests:
 *   stage = [-1, nahone, -1]  (or a permutation)
 *   hand  = [nahone_hand, live];  deck filled with 10 filler each;  10 energy
 *   5 passes -> set the live card -> clear p1 deck -> 2 passes
 * The ability is then reached by the PHASE MACHINE, so if the walk lands
 * somewhere else the test says so rather than silently passing. */
static int nahone_walk_to_live_start(TestGame *tg, int nahone_stage, int area,
                                     int nahone_hand, int live, int filler,
                                     int extra_hand_first)
{
    (void)extra_hand_first;
    set_stage3(tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    if (area >= 0 && area < RB_STAGE_SIZE) tg->state.p[0].stage[area] = nahone_stage;
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);

    advance_to_live_set(tg, 5);
    int hand_ids[3];
    int n = 0;
    hand_ids[n++] = nahone_hand;
    hand_ids[n++] = live;
    set_hand(tg, hand_ids, n);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);
    return tg->state.p[0].stage[area] == nahone_stage;
}

/* Read the two modifiers the Rust get_heart04 / get_blade helpers read. */
static int nahone_h04(TestGame *tg, int cid) { return mh_at(tg, cid, "heart04"); }
static int nahone_blade(TestGame *tg, int cid) { return blade_of(tg, cid); }

/* A1. Rust discard_member_gives_heart04_and_blade_to_same_name */
static void a1_discard_member_gives_heart04_and_blade(void)
{
    TestGame *tg = &t_a;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    /* The identity premise, stated as an assertion: the activating card and the
     * discarded card really are two prints of the SAME 百生 吟子. */
    CHECK_EQ(0, strcmp(card_name_of(nahone), card_name_of(nahone_hand)),
             "a1: the stage print and the hand print share the name 百生 吟子");
    CHECK(nahone != nahone_hand,
          "a1: the two Nahone instances are DISTINCT pool slots");
    CHECK_EQ(0, strcmp(card_name_of(nahone), "\xe7\x99\xbe\xe7\x94\x9f \xe5\x94\x84\xe5\xad\x90"),
             "a1: cards.json names both prints 百生 吟子 (not a decode artefact)");

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1,
        "a1: the optional 手札を1枚控え室に置く cost must be OFFERED (the "
        "Rust assert!(has_pending_choice) is the premise of every "
        "positive assertion below)");
    CHECK(pending(tg), "a1: the optional hand-discard cost prompts at ライブ開始時");
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(c->kind == RB_CHOICE_SELECT_CARD,
              "a1: the cost is a SelectCard (got %s)", prompt_shape(tg));
        CHECK(c->zone[0] && !strcmp(c->zone, "hand"),
              "a1: the cost selects out of the HAND (got zone \"%s\")", c->zone);
        CHECK_EQ(1, c->count, "a1: the cost asks for exactly 1 card");
        CHECK(rb_choice_allow_skip(c),
              "a1: the cost is OPTIONAL (「〜てもよい」) so skipping is offered");
        pick(tg, 0);
    }

    drain_choices(tg, 24);

    CHECK(wait_has(tg, nahone_hand),
          "a1: the discarded member reached the WAITROOM (got %s)",
          card_no_of(nahone_hand));
    CHECK_EQ(1, nahone_h04(tg, nahone),
             "a1: the same-name member gains +1 heart04 until live end");
    CHECK(nahone_blade(tg, nahone) >= 1,
          "a1: the same-name member gains >= 1 blade (got %d)",
          nahone_blade(tg, nahone));
}

/* A2. Rust skip_cost_no_effect */
static void a2_skip_cost_no_effect(void)
{
    TestGame *tg = &t_b;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1,
        "a2: the cost prompt must be ON SCREEN for a decline to mean anything; "
        "a green 'no buff' on a board where the ability never ran is vacuous");
    CHECK(pending(tg), "a2: the optional cost prompts");
    if (pending(tg)) pick_empty(tg);   /* Rust select_indices(&[]) */
    drain_choices(tg, 24);

    CHECK(hand_has(tg, nahone_hand),
          "a2: declining leaves the card in HAND (got waitroom n=%d)", wait_n(tg));
    CHECK_EQ(0, nahone_h04(tg, nahone), "a2: no heart04 when the cost is declined");
    CHECK_EQ(0, nahone_blade(tg, nahone), "a2: no blade when the cost is declined");

    /* POSITIVE CONTROL on the identical board: paying the very same cost DOES
     * produce the buff, so the two greens above are evidence and not silence. */
    {
        TestGame *p = &t_c;
        fresh(p);
        int n2, h2, l2, f2;
        if (!fx(p, &n2, NAHONE_RP) || !fx(p, &h2, NAHONE_PR) ||
            !fx(p, &l2, LIVE_M) || !fx(p, &f2, HONOKA)) return;
        nahone_walk_to_live_start(p, n2, AREA_CENTER, h2, l2, f2, 0);
        CHECK_PROMPTS(p, 1, "a2c: the positive control must also prompt");
        if (pending(p)) pick(p, 0);
        drain_choices(p, 24);
        CHECK_EQ(1, nahone_h04(p, n2),
                 "a2c: POSITIVE CONTROL - paying the same cost grants heart04");
    }
}

/* A3. Rust discard_non_member_no_buff */
static void a3_discard_non_member_no_buff(void)
{
    TestGame *tg = &t_d;
    fresh(tg);
    int nahone, live_discard, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &live_discard, LIVE_M) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    CHECK(card_is_live(live_discard),
          "a3: the discarded card really is a LIVE card (not a member)");
    CHECK(strcmp(card_name_of(live_discard), card_name_of(nahone)) != 0,
          "a3: the discarded live card does NOT share Nahone's name");

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, live_discard, live, filler, 0);

    CHECK_PROMPTS(tg, 1,
        "a3: the cost must prompt; a 'no buff' on a silent board is vacuous");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK(wait_has(tg, live_discard),
          "a3: the discarded live card reached the waitroom");
    CHECK_EQ(0, nahone_h04(tg, nahone),
             "a3: discarding a NON-member fails the location_condition, so no heart04");
    CHECK_EQ(0, nahone_blade(tg, nahone),
             "a3: discarding a NON-member grants no blade");
}

/* A4. Rust empty_hand_skips */
static void a4_empty_hand_skips(void)
{
    TestGame *tg = &t_e;
    fresh(tg);
    int nahone, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &live, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;

    set_stage3(tg, 0, RB_EMPTY_SLOT, nahone, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[1] = { live };
    set_hand(tg, hand_ids, 1);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);

    CHECK(!pending(tg),
          "a4: with only the live card in hand the optional cost is "
          "unpayable and must AUTO-SKIP with no prompt at all");
    CHECK_EQ(0, nahone_h04(tg, nahone), "a4: no heart04");
    CHECK_EQ(0, nahone_blade(tg, nahone), "a4: no blade");
}

/* A5. Rust discard_member_different_name_than_activating
 * The discarded card IS the same print here (Rust reuses the same card_no for
 * the "other member"); the test's own comment says it verifies the filter uses
 * the DISCARDED card's name. Ported with the same fixture, and the identity is
 * pinned so a future edit cannot quietly turn it into A1. */
static void a5_discard_member_same_print_as_activating(void)
{
    TestGame *tg = &t_f;
    fresh(tg);
    int nahone, other_member, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &other_member, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    CHECK(nahone != other_member,
          "a5: the activating card and the discarded card are distinct instances");
    CHECK_EQ(0, strcmp(card_name_of(nahone), card_name_of(other_member)),
             "a5: the Rust fixture stages the SAME print, so the two are the "
             "same name (documented in the Rust comment)");

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, other_member, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a5: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK(wait_has(tg, other_member), "a5: the discarded member reached the waitroom");
    CHECK_EQ(1, nahone_h04(tg, nahone),
             "a5: the activating member is the only same-name member and is buffed");
    CHECK(nahone_blade(tg, nahone) >= 1, "a5: ... and gains >= 1 blade");
}

/* A6. Rust discard_same_name_as_activating_card - same board as A1 with the
 * intent spelled out: the activating card itself is the same-name target. */
static void a6_activating_card_is_the_same_name_target(void)
{
    TestGame *tg = &t_g;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a6: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK_EQ(1, nahone_h04(tg, nahone),
             "a6: the activating card is itself a same-name member, so it is buffed");
    CHECK(nahone_blade(tg, nahone) >= 1, "a6: ... and gains >= 1 blade");
}

/* A7. Rust q30_multiple_same_name_only_one_buffed */
static void a7_q30_two_same_name_only_one_buffed(void)
{
    TestGame *tg = &t_h;
    fresh(tg);
    int nahone1, nahone2, nahone_hand, live, filler;
    if (!fx(tg, &nahone1, NAHONE_RP) || !fx(tg, &nahone2, NAHONE_RP) ||
        !fx(tg, &nahone_hand, NAHONE_PR) || !fx(tg, &live, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(nahone1 != nahone2,
          "a7: the two stage slots are DISTINCT instances of one print "
          "(aliasing would make this assertion vacuous)");

    set_stage3(tg, 0, nahone1, nahone2, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[2] = { nahone_hand, live };
    set_hand(tg, hand_ids, 2);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);

    CHECK_PROMPTS(tg, 1, "a7: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_first(tg, 24);

    int h1 = nahone_h04(tg, nahone1), h2 = nahone_h04(tg, nahone2);
    CHECK_EQ(1, h1 + h2,
             "a7: Q30 - exactly ONE member gets +1 heart04 (got %d + %d)", h1, h2);
    int b1 = nahone_blade(tg, nahone1), b2 = nahone_blade(tg, nahone2);
    CHECK((b1 >= 1 && b2 == 0) || (b1 == 0 && b2 >= 1),
          "a7: exactly ONE member gets the blade (got %d / %d)", b1, b2);
}

/* A8. Rust same_name_target_excludes_different_name_and_opponent_instances.
 * The most load-bearing test in the file: it pins the FILTERED STAGE choice
 * (zone / filtered_indices / target_player_id) as well as the outcome. */
static void a8_same_name_filter_shape_and_scope(void)
{
    TestGame *tg = &t_i;
    fresh(tg);
    int nahone1, nahone2, different_name, opponent_copy, nahone_hand, live, filler;
    if (!fx(tg, &nahone1, NAHONE_RP) || !fx(tg, &nahone2, NAHONE_RP) ||
        !fx(tg, &different_name, HONOKA) || !fx(tg, &opponent_copy, NAHONE_RP) ||
        !fx(tg, &nahone_hand, NAHONE_PR) || !fx(tg, &live, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(nahone1 != nahone2 && nahone1 != opponent_copy && nahone2 != opponent_copy,
          "a8: the three Nahone slots are DISTINCT instances");
    CHECK(strcmp(card_name_of(different_name), card_name_of(nahone1)) != 0,
          "a8: the third stage member carries a DIFFERENT name (高坂 穂乃果)");
    CHECK_EQ(0, strcmp(card_name_of(opponent_copy), card_name_of(nahone1)),
             "a8: the opponent's copy carries the SAME name (so only target="
             "\"self\" can exclude it)");

    set_stage3(tg, 0, nahone1, different_name, nahone2);
    set_stage3(tg, 1, opponent_copy, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[2] = { nahone_hand, live };
    set_hand(tg, hand_ids, 2);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);

    /* Rust: select_indices(&[0]) for the hand cost, then a bounded loop that
     * answers the hand prompt with 0 and the filtered STAGE prompt with 1. */
    int guard = 0, saw_stage_choice = 0, saw_hand_choice = 0;
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->zone[0] && !strcmp(c->zone, "hand")) saw_hand_choice = 1;
        pick(tg, 0);
    }
    while (pending(tg) && guard++ < 8) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->zone[0] && !strcmp(c->zone, "hand")) { saw_hand_choice = 1; pick(tg, 0); continue; }
        saw_stage_choice = 1;
        CHECK(c != NULL, "a8: a stage choice was expected (guard %d)", guard);
        if (!c) break;
        CHECK(c->kind == RB_CHOICE_SELECT_CARD,
              "a8: the same-name target pick is a SelectCard (got %s)", prompt_shape(tg));
        CHECK(c->zone[0] && !strcmp(c->zone, "stage"),
              "a8: the same-name target is selected from the STAGE (got zone \"%s\")",
              c->zone);
        CHECK(!c->target_player_id[0] || !strcmp(c->target_player_id, "self"),
              "a8: the target player is SELF (got \"%s\")", c->target_player_id);
        /* Rust asserts filtered_indices == [0, 2]: the two own same-name stage
         * members, with the differently-named third one filtered OUT. */
        CHECK_EQ(2, c->n_filtered_indices,
                 "a8: exactly 2 filtered stage candidates (the two own "
                 "same-name members; got %d)", c->n_filtered_indices);
        if (c->n_filtered_indices == 2) {
            CHECK_EQ(0, c->filtered_indices[0], "a8: filtered[0] is area 0 (nahone1)");
            CHECK_EQ(2, c->filtered_indices[1], "a8: filtered[1] is area 2 (nahone2)");
        }
        pick(tg, 1);   /* Rust select_indices(&[1]) */
    }
    CHECK(saw_hand_choice,
          "a8: the optional hand-discard cost prompt was seen (a same-name "
          "stage pick cannot exist without it)");
    CHECK(saw_stage_choice,
          "a8: a same-name STAGE choice was actually offered");
    drain_choices(tg, 16);

    CHECK_EQ(0, nahone_h04(tg, nahone1), "a8: the UNselected member gets no heart04");
    CHECK_EQ(1, nahone_h04(tg, nahone2), "a8: the selected member gets +1 heart04");
    CHECK_EQ(0, nahone_h04(tg, different_name),
             "a8: the differently-named member gets NO heart04");
    CHECK_EQ(0, nahone_h04(tg, opponent_copy),
             "a8: the OPPONENT's same-name instance gets NO heart04");
    CHECK_EQ(0, nahone_blade(tg, nahone1), "a8: unselected member gets no blade");
    CHECK_EQ(1, nahone_blade(tg, nahone2), "a8: selected member gets +1 blade");
    CHECK_EQ(0, nahone_blade(tg, different_name), "a8: different name gets no blade");
    CHECK_EQ(0, nahone_blade(tg, opponent_copy), "a8: opponent copy gets no blade");
    CHECK(!pending(tg), "a8: the ability resolved cleanly (got %s)", prompt_shape(tg));
}

/* A9. Rust discarded_card_goes_to_waitroom */
static void a9_discarded_card_goes_to_waitroom(void)
{
    TestGame *tg = &t_j;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a9: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK(!hand_has(tg, nahone_hand), "a9: the card left the hand");
    CHECK(wait_has(tg, nahone_hand), "a9: the card is in the WAITROOM");
}

/* A10. Rust heart04_only_no_other_colors */
static void a10_heart04_only_no_other_colors(void)
{
    TestGame *tg = &t_k;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a10: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK_EQ(1, nahone_h04(tg, nahone), "a10: +1 heart04 is granted");
    /* heart04 is the only colour; index derived from the label, never from a
     * RB_HEART_* constant (test_get_heart_modifier remaps colour 5). */
    static const char *const others[6] = {
        "heart00", "heart01", "heart02", "heart03", "heart05", "heart06"
    };
    for (int i = 0; i < 6; i++)
        CHECK_EQ(0, mh_at(tg, nahone, others[i]),
                 "a10: no %s is granted alongside heart04", others[i]);
}

/* A11/A12/A13. The P / P+ / SEC prints carry the SAME ability. */
static void a11_variant_works(const char *no, const char *tag)
{
    TestGame *tg = (strcmp(tag, "p") == 0) ? &t_l :
                   (strcmp(tag, "pp") == 0) ? &t_m : &t_n;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, no) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    CHECK_EQ(0, strcmp(card_name_of(nahone), card_name_of(nahone_hand)),
             "a11/%s: the variant shares the discarded card's name", tag);

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a11/%s: the cost must prompt", tag);
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK_EQ(1, nahone_h04(tg, nahone), "a11/%s: the variant grants heart04", tag);
    CHECK(nahone_blade(tg, nahone) >= 1, "a11/%s: the variant grants >= 1 blade", tag);
}

static void a11_p(void)     { a11_variant_works(NAHONE_P,   "p"); }
static void a11_p_plus(void){ a11_variant_works(NAHONE_PP,  "pp"); }
static void a11_sec(void)   { a11_variant_works(NAHONE_SEC, "sec"); }

/* A14. Rust discard_from_non_contiguous_hand */
static void a14_discard_from_non_contiguous_hand(void)
{
    TestGame *tg = &t_o;
    fresh(tg);
    int nahone, filler, live, nahone_hand;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &nahone_hand, NAHONE_PR)) return;

    set_stage3(tg, 0, RB_EMPTY_SLOT, nahone, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[3] = { filler, live, nahone_hand };
    set_hand(tg, hand_ids, 3);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);
    /* the live card was lifted out of hand, so the member is the LAST entry */
    CHECK_EQ(1, hand_n(tg), "a14: the hand holds exactly the non-live member");
    CHECK(hand_has(tg, nahone_hand), "a14: hand index 0 is the member to discard");

    CHECK_PROMPTS(tg, 1, "a14: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK(wait_has(tg, nahone_hand),
          "a14: the member chosen from a non-contiguous hand reached the waitroom");
    CHECK_EQ(1, nahone_h04(tg, nahone), "a14: heart04 is granted");
    CHECK(nahone_blade(tg, nahone) >= 1, "a14: >= 1 blade is granted");
}

/* A15. Rust only_live_card_in_hand_skips */
static void a15_only_live_card_in_hand_skips(void)
{
    TestGame *tg = &t_p;
    fresh(tg);
    int nahone, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &live, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;

    set_stage3(tg, 0, RB_EMPTY_SLOT, nahone, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[1] = { live };
    set_hand(tg, hand_ids, 1);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);

    CHECK(!pending(tg),
          "a15: only the live card in hand -> the optional cost is "
          "unpayable and AUTO-SKIPS (no prompt)");
    CHECK_EQ(0, nahone_h04(tg, nahone), "a15: no heart04");
    CHECK_EQ(0, nahone_blade(tg, nahone), "a15: no blade");
}

/* A16/A17. Nahone at the LEFT and at the RIGHT stage area. */
static void a16_position_variant(int area, const char *tag)
{
    TestGame *tg = (area == AREA_LEFT) ? &t_q : &t_r;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;

    nahone_walk_to_live_start(tg, nahone, area, nahone_hand, live, filler, 0);
    CHECK_EQ(nahone, stage_at(tg, area),
             "a16/%s: Nahone is staged at area %d before the trigger", tag, area);

    CHECK_PROMPTS(tg, 1, "a16/%s: the cost must prompt", tag);
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK_EQ(1, nahone_h04(tg, nahone), "a16/%s: heart04 is granted at area %d", tag, area);
    CHECK(nahone_blade(tg, nahone) >= 1, "a16/%s: >= 1 blade is granted", tag);
}

static void a16_left(void)  { a16_position_variant(AREA_LEFT,  "left"); }
static void a16_right(void) { a16_position_variant(AREA_RIGHT, "right"); }

/* A18. Rust hand_mixed_cards_discard_member_only */
static void a18_mixed_hand_discards_member_only(void)
{
    TestGame *tg = &t_s;
    fresh(tg);
    int nahone, nahone_hand, filler, live;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &filler, HONOKA) || !fx(tg, &live, LIVE_M)) return;

    set_stage3(tg, 0, RB_EMPTY_SLOT, nahone, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[3] = { filler, nahone_hand, live };
    set_hand(tg, hand_ids, 3);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);
    CHECK_EQ(2, hand_n(tg), "a18: hand holds filler then member after the live lifts out");

    CHECK_PROMPTS(tg, 1, "a18: the cost must prompt");
    if (pending(tg)) pick(tg, 1);   /* Rust select_indices(&[1]) */
    drain_choices(tg, 24);

    CHECK(wait_has(tg, nahone_hand), "a18: the chosen MEMBER went to the waitroom");
    CHECK(!wait_has(tg, filler), "a18: the filler was NOT discarded");
    CHECK(hand_has(tg, filler), "a18: the non-discarded card stays in hand");
    CHECK_EQ(1, nahone_h04(tg, nahone), "a18: heart04 is granted");
    CHECK(nahone_blade(tg, nahone) >= 1, "a18: >= 1 blade is granted");
}

/* A19. Rust discard_different_name_member_no_target - the discarded card is a
 * MEMBER but a DIFFERENT name, so the condition passes and no same-name stage
 * member exists. The result must be NO buff, and the buffer must still have
 * consumed the card (the cost is paid either way). */
static void a19_different_name_member_no_target(void)
{
    TestGame *tg = &t_u;
    fresh(tg);
    int nahone, other_member, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &other_member, HONOKA) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    CHECK(!card_is_live(other_member),
          "a19: the discarded card is a MEMBER (so the condition passes)");
    CHECK(strcmp(card_name_of(other_member), card_name_of(nahone)) != 0,
          "a19: the discarded member carries a DIFFERENT name");

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, other_member, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a19: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    CHECK(wait_has(tg, other_member), "a19: the discarded member reached the waitroom");
    CHECK_EQ(0, nahone_h04(tg, nahone),
             "a19: a different-name discarded member leaves no same-name target");
    CHECK_EQ(0, nahone_blade(tg, nahone), "a19: ... and no blade");
}

/* A20. Rust other_same_name_not_buffed */
static void a20_other_same_name_not_buffed(void)
{
    TestGame *tg = &t_v;
    fresh(tg);
    int nahone1, nahone2, nahone_hand, live, filler;
    if (!fx(tg, &nahone1, NAHONE_RP) || !fx(tg, &nahone2, NAHONE_RP) ||
        !fx(tg, &nahone_hand, NAHONE_PR) || !fx(tg, &live, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(nahone1 != nahone2, "a20: the two stage slots are distinct instances");

    set_stage3(tg, 0, nahone1, nahone2, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    int hand_ids[2] = { nahone_hand, live };
    set_hand(tg, hand_ids, 2);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);

    CHECK_PROMPTS(tg, 1, "a20: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_first(tg, 24);

    int h1 = nahone_h04(tg, nahone1), h2 = nahone_h04(tg, nahone2);
    int b1 = nahone_blade(tg, nahone1), b2 = nahone_blade(tg, nahone2);
    CHECK((h1 == 1 && h2 == 0) || (h1 == 0 && h2 == 1),
          "a20: exactly one member is buffed (heart %d / %d)", h1, h2);
    CHECK((b1 >= 1 && b2 == 0) || (b1 == 0 && b2 >= 1),
          "a20: exactly one member gets the blade (%d / %d)", b1, b2);
    if (h1 == 1) {
        CHECK(b1 >= 1, "a20: the heart04 recipient also receives the blade");
        CHECK_EQ(0, b2, "a20: the other member receives nothing");
    } else {
        CHECK(b2 >= 1, "a20: the heart04 recipient also receives the blade");
        CHECK_EQ(0, b1, "a20: the other member receives nothing");
    }
}

/* A21. Rust effect_tracked_as_temporary_live_end */
static void a21_effect_is_temporary_live_end(void)
{
    TestGame *tg = &t_w;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;

    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);

    CHECK_PROMPTS(tg, 1, "a21: the cost must prompt");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);

    int live_end_effects = 0, for_nahone = 0;
    for (int i = 0; i < RB_MAX_TEMP_EFFECTS; i++) {
        const RbTempEffect *t = &tg->state.temp_effects[i];
        if (t->dur == RB_TEMP_LIVE_END) {
            live_end_effects++;
            if (t->card_id == nahone) for_nahone++;
        }
    }
    CHECK(live_end_effects > 0,
          "a21: 「ライブ終了時まで」 is tracked as a temporary effect with "
          "dur == RB_TEMP_LIVE_END (found %d such entries)", live_end_effects);
    CHECK(for_nahone > 0,
          "a21: one of the live-end temporaries belongs to Nahone (got %d)",
          for_nahone);
    CHECK_EQ(1, nahone_h04(tg, nahone), "a21: heart04 is granted");
    CHECK(nahone_blade(tg, nahone) >= 1, "a21: >= 1 blade is granted");
}

/* ================================================================== *
 * main (bootstrapped with section A; sections B..K append below)
 * ================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_ok, n_failed, n_crashed;

/* Every case runs in a FORKED child with a SIGSEGV/SIGBUS/SIGABRT handler
 * naming the test in flight. This is crash ISOLATION, not a hang detector: no
 * watchdog is achievable on this toolchain. */
static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [in-process fallback]\n", failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps, p0 = prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d gap(s), %d prompt(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0), (int)(prompts - p0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    print_heart_mapping();
    dump_requested();

    printf("--- A nahone_live_start_same_name_test.rs "
           "(PL!HS-bp2-007-R+/P/P+/SEC) ---\n");
    run("a1_discard_member_buffs",      a1_discard_member_gives_heart04_and_blade);
    run("a2_skip_cost_no_effect",       a2_skip_cost_no_effect);
    run("a3_discard_non_member",        a3_discard_non_member_no_buff);
    run("a4_empty_hand_skips",          a4_empty_hand_skips);
    run("a5_same_print_as_activating",  a5_discard_member_same_print_as_activating);
    run("a6_self_is_target",            a6_activating_card_is_the_same_name_target);
    run("a7_q30_only_one_buffed",       a7_q30_two_same_name_only_one_buffed);
    run("a8_filter_shape_and_scope",    a8_same_name_filter_shape_and_scope);
    run("a9_goes_to_waitroom",          a9_discarded_card_goes_to_waitroom);
    run("a10_heart04_only",             a10_heart04_only_no_other_colors);
    run("a11_p_variant",                a11_p);
    run("a12_p_plus_variant",           a11_p_plus);
    run("a13_sec_variant",              a11_sec);
    run("a14_non_contiguous_hand",      a14_discard_from_non_contiguous_hand);
    run("a15_only_live_skips",          a15_only_live_card_in_hand_skips);
    run("a16_left_position",            a16_left);
    run("a17_right_position",           a16_right);
    run("a18_mixed_hand",               a18_mixed_hand_discards_member_only);
    run("a19_diff_name_no_target",      a19_different_name_member_no_target);
    run("a20_other_same_name",          a20_other_same_name_not_buffed);
    run("a21_temporary_live_end",       a21_effect_is_temporary_live_end);
    rb_unload();

    printf("\n==== parity_cce_gaps ====\n");
    printf("tests run              : %d\n", n_ok + n_failed + n_crashed);
    printf("tests passed           : %d\n", n_ok);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps          : %d\n", gaps);
    printf("fixture setup bugs     : %d\n", setup_bugs);
    printf("assertions             : %d\n", assertions);
    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_crashed);
        return 1;
    }
    if (n_failed) {
        fprintf(stderr,
                "PARITY CCE GAPS: %d failing test(s) - every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, a Rust\n"
                "fixture bug, or a SETUP BUG naming a print the engine cannot\n"
                "resolve. Not a bug in this file.\n", n_failed);
        return n_failed > 125 ? 125 : n_failed;
    }
    printf("ALL PARITY_CCE_GAPS CHECKS PASSED\n");
    return 0;
}
