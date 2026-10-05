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
              t_y, t_z,
              /* B..K */
              t_b1, t_b2, t_b3, t_b4,
              t_c1, t_c2,
              t_d1,
              t_e1, t_e2, t_e3, t_e4, t_e5, t_e6,
              t_f1, t_f2, t_f3, t_f4,
              t_g1, t_g2, t_g3, t_g4, t_g5,
              t_h1, t_h2, t_h3, t_h4, t_h5,
              t_i1, t_i2, t_i3, t_i4,
              t_j1, t_j2, t_j3, t_j4, t_j5, t_j6, t_j7, t_j8,
              t_k1, t_k2, t_k3, t_k4, t_k5, t_k6, t_k7, t_k8;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** Assertions evaluated before the fault: %d, failures: %d, "
            "prompts: %d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
    /* Re-raise under the DEFAULT disposition: run()'s CRASH verdict is only
     * reachable when WIFSIGNALED(status) is true, and _Exit(128 + sig) turned
     * the crash into an ordinary exit instead. */
    signal(sig, SIG_DFL);
    raise(sig);
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

/* CHECK_EQ(expected, actual, ...) - the ORDER is (expected, actual) and the
 * message prints both, so a red line names which side the engine got wrong.
 * (The first draft of this file had the two swapped, which made every failure
 * read "got 1 expected 0" for an actual value of 0.) */
#define CHECK_EQ(expected, actual, ...) do {                              \
        assertions++;                                                     \
        long long e_ = (long long)(expected);                             \
        long long a_ = (long long)(actual);                               \
        if (a_ != e_) {                                                   \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (expected %lld, actual %lld)\n", e_, a_);   \
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
            fprintf(stderr, "FAIL: [VACUOUS]  ");                          \
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

#define TRIG_LIVE_START "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe9\x96\x8b\xe5\xa7\x8b\xe6\x99\x82"

/* helpers/mod.rs:114 fire_trigger: resolve ONE ability on ONE card, key it
 * "{card_no}_{full_text}", fire it with `cid` as the activating card, publish
 * activating_card, process the queue.
 *
 * The body used to be a hand-rolled copy of that sequence, which recorded NO
 * enqueue-time use — so a 自動/一度だけ ability was still unspent when it
 * resolved and the post-resolution re-scan in
 * rb_process_player_abilities_depth_full enqueued it a SECOND time. Its
 * "{card_no}_{full_text}" key also truncated at 512 bytes. Both are fixed by
 * the shared helper, which is the shape Rust's own helper has. */
static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    (void)trigger_type;   /* every call site here passes trig == trigger_type */
    return test_fire_trigger_on_card(tg, cid, trig);
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
    /* The identity premise, stated as an assertion: the activating card and the
     * discarded card really are two prints of the SAME 百生 吟子.
     *
     * NAME-SOURCE EVIDENCE: cards/cards.json (read as UTF-8) says
     *   "PL!HS-bp2-007-R＋": { "name": "百生 吟子", "type": "メンバー", "cost": 11 }
     *   "PL!HS-PR-007-PR"  : { "name": "百生 吟子", "type": "メンバー", "cost":  9 }
     * The two decoded names are printed rather than compared against a byte
     * literal, because a same-name test is exactly the place where a copy that
     * carries the WRONG Card.name would be invisible. */
    CHECK_EQ(0, strcmp(card_name_of(nahone), card_name_of(nahone_hand)),
             "a1: the stage print and the hand print share ONE name "
             "(stage='%s' hand='%s')",
             card_name_of(nahone), card_name_of(nahone_hand));
    CHECK(nahone != nahone_hand,
          "a1: the two Nahone instances are DISTINCT pool slots");
    CHECK(strstr(card_name_of(nahone), "\xe7\x99\xbe\xe7\x94\x9f") != NULL,
          "a1: the decoded name starts with the printed surname 百生 "
          "(decoded='%s'; cards.json name is 百生 吟子)", card_name_of(nahone));

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
    /* C's test_set_live_card does not remove the live card from the hand, so
     * the hand still holds filler + member and the MEMBER is the LAST entry.
     * (Asserted rather than assumed - a peer-measured divergence from Rust's
     * set_live_card, and guessing would be a false-red generator.) */
    CHECK(hand_has(tg, nahone_hand),
          "a14: the member to discard is in hand (hand n=%d)", hand_n(tg));
    CHECK(!hand_has(tg, live), "a14: the live card is out of the hand");
    int member_idx = -1;
    for (int i = 0; i < hand_n(tg); i++)
        if (tg->state.p[0].hand.cards[i] == nahone_hand) member_idx = i;
    CHECK(member_idx >= 0, "a14: the member's hand index is resolvable");

    CHECK_PROMPTS(tg, 1, "a14: the cost must prompt");
    if (pending(tg) && member_idx >= 0) pick(tg, member_idx);
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
 * A22. ROOT-CAUSE A/B for the whole of section A.
 *
 * Every red in A1-A21 reduces to ONE defect, and this test isolates it.
 *
 * THE ASYMMETRY (src/ability/ability_queue.c, rb_drain_ability_queue,
 * lines 739-801): the queue drain decodes the ability at line 759 and
 * executes ONLY `ab.effect` at line 771:
 *
 *     if (ab.effect) {
 *         ...
 *         rb_execute_effect_ex(g, actor, ab.effect, e->card_id);
 *     }
 *
 * There is NO `rb_pay_cost(g, actor, ab.cost)` anywhere in the function -
 * `ab.cost` is read only at line 789, where it is set to NULL immediately
 * before the tree is freed. So a ライブ開始時 ability reached through the
 * TRIGGER QUEUE runs its EFFECT with its COST never offered and never paid.
 *
 * The other queue consumer, pca_resolve_entry
 * (src/core/game_state_abilities.c:1957-1963), DOES pay it:
 *
 *     if (ab->cost) {
 *         if (!rb_pay_cost(g, actor, ab->cost)) { ...free, advance, return 0; }
 *     }
 *
 * So the SAME card reaches a different answer depending on which of the two
 * queue consumers runs. A4/A15 are green only because an unpayable cost and
 * an unpaid cost look identical from the outside - which is exactly the
 * vacuity this test removes.
 *
 * A22a: the phase-machine / trigger-queue path (what the Rust twins walk).
 * A22b: the resolver path (rb_activate_card / pca_resolve_entry), which pays.
 * If A22b pays the cost and grants the buff while A22a offers nothing, the
 * defect is pinned to the missing rb_pay_cost in rb_drain_ability_queue and
 * not to the parser, not to the same_name filter, and not to the harness.
 * ================================================================== */

/* Drive the ability through the RESOLVER path: publish activating_card, pay the
 * printed cost with rb_pay_cost, and carry the effect across the cost prompt
 * exactly as pca_resolve_entry does. */
static int nahone_resolver_path(TestGame *tg, int nahone, int pay_cost)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)nahone, 1, &ab)) { rb_free_ability(&ab); return 0; }
    tg->state.activating_card = nahone;
    tg->state.n_recently_moved = 0;

    if (pay_cost && ab.cost) {
        if (!rb_pay_cost(&tg->state, 0, ab.cost)) { rb_free_ability(&ab); return 0; }
        if (test_has_pending_choice(tg) && ab.effect) {
            /* engine.c:1091-1098 shape: the cost opened a choice, so the effect
             * becomes the continuation of that choice. */
            tg->state.queue.deferred = ab.effect;
            tg->state.queue.resume_host = nahone;
            tg->state.queue.resume_eff = ab.effect;
            /* The keep-alive handoff must be TOTAL, exactly as the engine's own
             * rb_drain_keep_alive (ability_queue.c:1302-1318) documents: it is a
             * by-value copy of an OWNING Ability, so nulling only ab.cost /
             * ab.effect left full_text / triggerless_text / triggers owned by
             * BOTH copies. rb_free_ability(&ab) below released those three
             * blocks, and the keep-alive release at the end of the prompt
             * (choice.c:5289 rb_free_ability(&g->activation_keepalive)) then
             * freed the SAME three blocks a second time -> heap double free ->
             * mingw abort -> SIGABRT (signal 6) with zero failures recorded.
             * That is verbatim the defect ability_queue.c:1306-1312 records as
             * already fixed engine-side; this test was re-introducing it by
             * hand. Zeroing the local makes the local release a no-op and
             * leaves one owner. */
            tg->state.activation_keepalive = ab;
            tg->state.activation_keepalive_valid = 1;
            memset(&ab, 0, sizeof(ab));
        }
    }
    if (ab.effect && !test_has_pending_choice(tg))
        rb_execute_effect_ex(&tg->state, 0, ab.effect, nahone);
    rb_drain_ability_queue(&tg->state);
    rb_free_ability(&ab);
    return 1;
}

static void a22_root_cause_ab(void)
{
    /* A22a: the TRIGGER-QUEUE path, byte-for-byte the same board as A1. */
    TestGame *tg = &t_x;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);
    /* Fire the same ability by hand so the queue consumer is unambiguous. */
    rb_clear_pending_choice(&tg->state);
    int queued = fire_live_start(tg, nahone);
    CHECK(queued, "a22a: the ライブ開始時 ability was queued");
    CHECK_PROMPTS(tg, 1,
        "a22a: NORMALITY - a live-start ability with a hand in range that "
        "MUST offer its optional cost is expected to prompt. Zero prompts "
        "here is the defect, not a green result.");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 24);
    CHECK(wait_has(tg, nahone_hand),
          "a22a: the optional cost was NOT offered or paid - the member is "
          "still in hand (waitroom n=%d)", wait_n(tg));
    CHECK_EQ(0, nahone_h04(tg, nahone),
             "a22a: with the cost unpaid the location_condition on the "
             "waitroom has no card to inspect, so no heart04 is granted");
}

/* A22b: the RESOLVER path on the IDENTICAL board. Kept in its OWN forked child
 * so a crash here cannot be confused with anything a22a left behind. */
static void a22_resolver_pays_cost(void)
{
    TestGame *tg = &t_y;
    fresh(tg);
    int nahone, nahone_hand, live, filler;
    if (!fx(tg, &nahone, NAHONE_RP) || !fx(tg, &nahone_hand, NAHONE_PR) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    nahone_walk_to_live_start(tg, nahone, AREA_CENTER, nahone_hand, live, filler, 0);
    rb_clear_pending_choice(&tg->state);
    int ok = nahone_resolver_path(tg, nahone, 1);
    CHECK(ok, "a22b: the ability decodes and the cost pays on the resolver path");
    CHECK_PROMPTS(tg, 1,
        "a22b: PAYING the cost through rb_pay_cost DOES offer the pick - this "
        "is the A/B partner of a22a and is what makes a22a's zero "
        "non-vacuous evidence rather than a silent board.");
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(c->zone[0] && !strcmp(c->zone, "hand"),
              "a22b: the paid cost picks out of the hand (got \"%s\")", c->zone);
        pick(tg, 0);
    }
    drain_choices(tg, 24);
    CHECK(wait_has(tg, nahone_hand),
          "a22b: the cost IS paid on the resolver path - the member reached "
          "the waitroom");
    CHECK_EQ(1, nahone_h04(tg, nahone),
             "a22b: with the cost paid, the same_name gain_resource grants "
             "+1 heart04 - the parser and the same_name filter are CORRECT "
             "and the whole of section A reduces to the missing "
             "rb_pay_cost in rb_drain_ability_queue");
    CHECK(nahone_blade(tg, nahone) >= 1,
          "a22b: ... and the blade (got %d)", nahone_blade(tg, nahone));
}

/* ================================================================== *
 * B. live_start_optional_live_discard_chosen_heart_q190_test.rs
 *
 * ミア・テイラー PL!N-bp4-011-R＋ ab#0 ライブ開始時:
 *   手札のライブカードを1枚控え室に置いてもよい：好きなハートの色を1つ指定する。
 *   ライブ終了時まで、そのハートを1つ得る。
 *
 * Q190 RULING (from the Rust doc comment, and the point of the test): the
 * catch-all「ALL」 (heart00) is NOT a colour and must not be offered. Only
 * heart01..heart06 appear. The flat RbChoice carries the option list as the
 * short strings in heart_options[], so the assertion is a set comparison over
 * those strings, not an index.
 *
 * cards.json: PL!N-bp4-011-R＋ = ミア・テイラー, メンバー, R3BIRTH, cost 9.
 * ================================================================== */

/* Rust setup_optional_live_discard_heart_source: Mia staged in the centre,
 * both decks filled, then advance_to_live_set + 2 passes. */
static int q190_walk(TestGame *tg, int mia, int filler, int hand_a, int hand_b)
{
    set_stage3(tg, 0, RB_EMPTY_SLOT, mia, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    int hand[2] = { hand_a, hand_b };
    set_hand(tg, hand, 2);
    advance_to_live_set(tg, 5);
    test_set_live_card(tg, 0, hand_a);
    advance_to_live_start(tg, 2);
    return 1;
}

/* Rust's filtered_idx: the index WITHIN the hand's live-card subset. Computed
 * from the live hand rather than hard-coded, so the test does not depend on
 * the order the two lives were added in. */
static int q190_live_subset_index(TestGame *tg, int target)
{
    int idx = 0;
    for (int i = 0; i < tg->state.p[0].hand.n; i++) {
        if (tg->state.p[0].hand.cards[i] == target) return idx;
        if (card_is_live(tg->state.p[0].hand.cards[i])) idx++;
    }
    return -1;
}

/* B1. Rust optional_live_discard_heart_q190_heart_selection_excludes_all_and_... */
static void b1_q190_offers_heart01_to_06_only(void)
{
    TestGame *tg = &t_b1;
    fresh(tg);
    int mia, filler, live_zone, live_cost;
    if (!fx(tg, &mia, MIA) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live_zone, LIVE_1) || !fx(tg, &live_cost, LIVE_SOME)) return;
    CHECK(card_is_live(live_zone) && card_is_live(live_cost),
          "b1: both hand fixtures really are LIVE cards (the cost filters on "
          "card_type=live_card)");

    q190_walk(tg, mia, filler, live_zone, live_cost);

    CHECK_PROMPTS(tg, 1,
        "b1: the optional cost must be OFFERED; a heart-colour assertion after "
        "a silent board is vacuous");
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(c->kind == RB_CHOICE_SELECT_CARD, "b1: the cost is a SelectCard (got %s)",
              prompt_shape(tg));
        CHECK(c->zone[0] && !strcmp(c->zone, "hand"),
              "b1: 「手札のライブカードを1枚」 - the source zone is the HAND (got \"%s\")",
              c->zone);
        CHECK_EQ(1, c->count, "b1: the cost asks for exactly 1 card");
        CHECK(rb_choice_allow_skip(c), "b1: the cost is OPTIONAL so skip is offered");
        int fi = q190_live_subset_index(tg, live_cost);
        CHECK(fi >= 0, "b1: the cost live card is still in hand");
        if (fi >= 0) pick(tg, fi);
    }

    /* THE Q190 ASSERTION. */
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL,
          "b1: paying the cost must raise the heart-colour prompt; the C raised "
          "nothing (got %s)", prompt_shape(tg));
    if (c) {
        CHECK(c->kind == RB_CHOICE_SELECT_HEART_COLOR,
              "b1: the prompt is a SelectHeartColor (got %s)", prompt_shape(tg));
        if (c->kind == RB_CHOICE_SELECT_HEART_COLOR) {
            CHECK_EQ(1, c->count, "b1: exactly one colour is specified");
            /* Q190: exactly heart01..heart06, heart00 EXCLUDED. */
            int seen[8] = { 0 };
            for (int i = 0; i < c->n_heart_options; i++) {
                const char *o = c->heart_options[i];
                if (!strcmp(o, "heart00") || !strcmp(o, "h00"))
                    seen[0] = 1;
                else if (!strcmp(o, "heart01")) seen[1] = 1;
                else if (!strcmp(o, "heart02")) seen[2] = 1;
                else if (!strcmp(o, "heart03")) seen[3] = 1;
                else if (!strcmp(o, "heart04")) seen[4] = 1;
                else if (!strcmp(o, "heart05")) seen[5] = 1;
                else if (!strcmp(o, "heart06")) seen[6] = 1;
                else seen[7] = 1;
            }
            CHECK(seen[0] == 0,
                  "b1: Q190 - the catch-all ALL (heart00) must NOT be offered "
                  "(options: %d offered)", c->n_heart_options);
            for (int i = 1; i <= 6; i++)
                CHECK(seen[i] == 1, "b1: heart0%d is offered", i);
            CHECK_EQ(0, seen[7], "b1: no unexpected option strings are offered");
            /* Rust select_option(1) == heart02 in the SORTED heart01..06 list. */
            select_option(tg, 1);
        }
    }

    CHECK_EQ(1, mh_at(tg, mia, "heart02"),
             "b1: the chosen colour is granted until live end");
    CHECK_EQ(0, mh_at(tg, mia, "heart03"),
             "b1: ONLY the specified colour is granted");
    CHECK(wait_has(tg, live_cost), "b1: the paid live card really reached the waitroom");
    CHECK(!pending(tg), "b1: the chain resolved cleanly (got %s)", prompt_shape(tg));
}

/* B2. Rust optional_live_discard_heart_q190_skipping_the_cost_specifies_nothing */
static void b2_q190_skip_specifies_nothing(void)
{
    TestGame *tg = &t_b2;
    fresh(tg);
    int mia, filler, live_zone, live_cost;
    if (!fx(tg, &mia, MIA) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live_zone, LIVE_1) || !fx(tg, &live_cost, LIVE_SOME)) return;

    q190_walk(tg, mia, filler, live_zone, live_cost);

    CHECK_PROMPTS(tg, 1,
        "b2: the pay/skip prompt must be ON SCREEN for a decline to mean "
        "anything");
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(rb_choice_allow_skip(c), "b2: the cost is skippable");
        pick_empty(tg);   /* Rust select_indices(&[]) */
    }
    CHECK(!pending(tg),
          "b2: skipping the optional cost ends the chain - no heart-colour "
          "prompt follows");
    CHECK(hand_has(tg, live_cost), "b2: the skipped live card stays in HAND");
    CHECK(!wait_has(tg, live_cost), "b2: nothing was discarded");
    CHECK_EQ(0, mh_at(tg, mia, "heart02"), "b2: no heart is gained without paying");
}

/* B3. Rust optional_live_discard_heart_q190_choose_heart05_grants_heart05 */
static void b3_q190_choose_heart05(void)
{
    TestGame *tg = &t_b3;
    fresh(tg);
    int mia, filler, live_zone, live_cost;
    if (!fx(tg, &mia, MIA) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live_zone, LIVE_1) || !fx(tg, &live_cost, LIVE_SOME)) return;

    q190_walk(tg, mia, filler, live_zone, live_cost);

    CHECK_PROMPTS(tg, 1, "b3: the cost must prompt");
    if (pending(tg)) {
        int fi = q190_live_subset_index(tg, live_cost);
        if (fi >= 0) pick(tg, fi);
    }
    /* Rust select_option(4) == heart05 in the sorted heart01..06 list. */
    if (pending(tg)) select_option(tg, 4);
    CHECK_EQ(1, mh_at(tg, mia, "heart05"),
             "b3: heart05 is the colour that was chosen, so heart05 is granted");
    CHECK_EQ(0, mh_at(tg, mia, "heart02"),
             "b3: no other colour is granted (heart02 must stay 0)");
}

/* B4. Rust optional_live_discard_heart_q190_no_live_in_hand_resolves_... */
static void b4_q190_no_live_in_hand(void)
{
    TestGame *tg = &t_b4;
    fresh(tg);
    int mia, filler, live_zone;
    if (!fx(tg, &mia, MIA) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live_zone, LIVE_1)) return;
    CHECK(!card_is_live(filler), "b4: the only non-live hand card is a MEMBER");

    q190_walk(tg, mia, filler, live_zone, filler);

    /* Rust tolerates either shape here: a skippable SelectCard, or an
     * auto-skip. Both are accepted; what must hold is that nothing is left
     * standing and no colour is granted. */
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(c->kind == RB_CHOICE_SELECT_CARD,
              "b4: any prompt must be a SelectCard (got %s)", prompt_shape(tg));
        if (c) CHECK(rb_choice_allow_skip(c), "b4: the prompt must be skippable");
        pick_empty(tg);
    }
    CHECK(!pending(tg), "b4: the chain resolves with nothing left standing");
    CHECK(!wait_has(tg, filler), "b4: the member filler is not discarded");
    CHECK_EQ(0, mh_at(tg, mia, "heart01"), "b4: no heart is granted");
    CHECK_EQ(0, mh_at(tg, mia, "heart06"), "b4: no heart is granted (heart06)");
}

/* ================================================================== *
 * C. live_start_optional_unit_discard_draw_unit_cost_gain_test.rs
 *
 * 村野さやか PL!HS-bp6-010-R ライブ開始時: an optional discard of a card in the
 * activating member's UNIT, then a draw, and a COST GAIN on every member of
 * that unit. The DISTINCT behaviour is the unit filter: only DOLLCHESTRA
 * members get the cost modifier, a スリーズブeke member does not.
 *
 * cards.json (UTF-8):
 *   PL!HS-bp6-010-R : 村野さやか, unit DOLLCHESTRA, cost 4
 *   PL!HS-PR-008-PR : 徒町 小鈴, unit DOLLCHESTRA, cost 9
 *   PL!HS-bp1-013-N : 夕霧綴理,  unit DOLLCHESTRA, cost 4  (the COST card)
 *   PL!HS-bp1-012-PR: 乙宗 梢,   unit スリーズブーケ,  cost 4  (NOT DOLLCHESTRA)
 * RUST FIXTURE NOTE: the second Rust test names PL!HS-bp1-012-PR the
 * 「non-DOLLCHESTRA」 card and the unit is スリーズブーケ, which matches. A
 * peer separately reported a 『DOLLCHESTRA』 fixture that was really
 * スリーズブeke; this one is NOT that case - the unit is pinned below so the
 * test cannot silently drift.
 * ================================================================== */

/* Card carries `unit_idx` (a STRING-TABLE index), not a decoded `unit` string,
 * so the unit is read through rb_card_string(unit_idx) - exactly as
 * condition.c:737 and util.c:348 do. */
static const char *card_unit_of(int cid)
{
    static char ring[4][96];
    static int  next;
    char *buf = ring[next];
    next = (next + 1) % 4;
    buf[0] = 0;
    Card c;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.unit_idx != UINT16_MAX) {
            const char *u = rb_card_string(c.unit_idx);
            if (u) snprintf(buf, 96, "%s", u);
        }
        rb_free_card(&c);
    }
    return buf;
}

static int card_unit_is(TestGame *tg, int cid, const char *unit)
{
    (void)tg;
    return strcmp(card_unit_of(cid), unit) == 0;
}

/* A prefix test, for the FULLWIDTH-punctuation-sensitive みらくらぱーく！ unit:
 * an exact compare would turn a U+FF01 vs '!' mismatch into an apparent stage
 * bug. The prefix is the part that carries the identity. */
static int card_unit_has_prefix(int cid, const char *prefix)
{
    return strstr(card_unit_of(cid), prefix) != NULL;
}

/* C1. Rust optional_unit_discard_draw_cost_gain_live_start_modify_cost_only_... */
static void c1_unit_cost_gain_only_for_dollchestra(void)
{
    TestGame *tg = &t_c1;
    fresh(tg);
    int sayaka, doll, non_doll, cost_card, live, filler;
    if (!fx(tg, &sayaka, SAYAKA) || !fx(tg, &doll, KOTSU) ||
        !fx(tg, &non_doll, KOTO) || !fx(tg, &cost_card, TSUYURI) ||
        !fx(tg, &live, LIVE_M) || !fx(tg, &filler, HONOKA)) return;
    /* THE UNIT PREMISE. Without these the +5 / 0 split is unfalsifiable. */
    CHECK(card_unit_is(tg, sayaka, "DOLLCHESTRA"),
          "c1: 村野さやか is DOLLCHESTRA (cards.json unit=DOLLCHESTRA)");
    CHECK(card_unit_is(tg, doll, "DOLLCHESTRA"),
          "c1: 徒町 小鈴 is DOLLCHESTRA");
    CHECK(card_unit_is(tg, non_doll, "\xe3\x82\xb9\xe3\x83\xaa\xe3\x83\xbc\xe3\x82\xba\xe3\x83\x96\xe3\x83\xbc\xe3\x82\xb1"),
          "c1: 乙宗 梢 is スリーズブeke, NOT DOLLCHESTRA - the negative arm of the test");
    CHECK(card_unit_is(tg, cost_card, "DOLLCHESTRA"),
          "c1: the optional cost card is DOLLCHESTRA (so it qualifies)");

    set_stage3(tg, 0, doll, sayaka, non_doll);
    fill_decks(tg, filler, 10);
    int hand[2] = { cost_card, live };
    set_hand(tg, hand, 2);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    /* set_live_card lifts the live out, leaving the cost card in hand. */
    advance_to_live_start(tg, 2);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg, 1);

    CHECK_PROMPTS(tg, 1,
        "c1: the optional unit-discard cost must be OFFERED; a cost-modifier "
        "assertion on a silent board is vacuous");
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(c->zone[0] && !strcmp(c->zone, "hand"),
              "c1: the cost selects from the HAND (got \"%s\")", c->zone);
        pick(tg, 0);
    }
    drain_choices(tg, 24);

    CHECK_EQ(5, cost_of(tg, doll),
             "c1: a DOLLCHESTRA member gets +5 cost");
    CHECK_EQ(5, cost_of(tg, sayaka),
             "c1: the ACTIVATING DOLLCHESTRA member also gets +5 cost "
             "(the unit filter is not exclude_self)");
    CHECK_EQ(0, cost_of(tg, non_doll),
             "c1: a スリーズブeke member gets 0 cost - the unit filter is real");
}

/* C2. Rust optional_unit_discard_draw_cost_gain_skip_cost_no_cost_modifier */
static void c2_unit_cost_gain_skipped(void)
{
    TestGame *tg = &t_c2;
    fresh(tg);
    int sayaka, doll, cost_card, live, filler;
    if (!fx(tg, &sayaka, SAYAKA) || !fx(tg, &doll, KOTSU) ||
        !fx(tg, &cost_card, TSUYURI) || !fx(tg, &live, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(card_unit_is(tg, cost_card, "DOLLCHESTRA"),
          "c2: the cost card is DOLLCHESTRA (so the cost IS payable - the "
          "negative is about the DECLINE, not about an unpayable cost)");

    set_stage3(tg, 0, doll, sayaka, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    int hand[2] = { cost_card, live };
    set_hand(tg, hand, 2);
    test_give_energy(tg, 10);
    advance_to_live_set(tg, 5);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg, 2);

    CHECK_PROMPTS(tg, 1,
        "c2: the optional cost must be ON SCREEN; 'no cost modifier' is only "
        "evidence when a decline was actually possible");
    if (pending(tg)) pick_empty(tg);   /* Rust select_indices(&[]) */
    drain_choices(tg, 24);

    CHECK_EQ(0, cost_of(tg, doll), "c2: declining the cost grants 0 cost");
    CHECK_EQ(0, cost_of(tg, sayaka), "c2: declining the cost grants 0 cost to the activator too");
    CHECK(hand_has(tg, cost_card), "c2: the declined cost card stays in hand");
}

/* ================================================================== *
 * D. live_start_optional_bottom_mill_named_recovery_source_test.rs
 *
 * 小原鞠莉 PL!S-bp7-008-R ライブ開始時 ab#1: an OPTIONAL mill of the BOTTOM
 * deck card, then a named recovery source.
 *
 * The Rust test's own comment records the isolation it needs: it seeds the
 * deck AFTER the live-card-set passes (which draw from the top) so the
 * top/bottom markers survive, and it asserts only that the BOTTOM-most card
 * left the deck, because Mari's follow-up may move it again.
 *
 * DISTINGUISHABLE fixtures are used - the Rust test already does this, and it
 * is what makes the bottom-end assertion non-vacuous. A deck of one filler
 * would pass whether the engine milled the top or the bottom.
 * ================================================================== */

static void d1_mari_bottom_mill(void)
{
    TestGame *tg = &t_d1;
    fresh(tg);
    int mari, filler, top_marker, b1, b2, b3, live;
    if (!fx(tg, &mari, MARI) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &top_marker, YOSHIKO) || !fx(tg, &b1, KANON) ||
        !fx(tg, &b2, ANZE) || !fx(tg, &b3, SUMIRE_SD) ||
        !fx(tg, &live, LIVE_M)) return;
    /* The four markers must be DISTINCT cards or the end cannot be observed. */
    CHECK(top_marker != b1 && top_marker != b2 && top_marker != b3 &&
          b1 != b2 && b1 != b3 && b2 != b3,
          "d1: the top/bottom deck markers are four DISTINCT cards");

    set_stage3(tg, 0, RB_EMPTY_SLOT, mari, RB_EMPTY_SLOT);
    test_give_energy(tg, 3);
    int hand[1] = { live };
    set_hand(tg, hand, 1);
    advance_to_live_set(tg, 5);

    /* Seed the deck AFTER the live-card-set passes. C's deck is TOP-INDEX-0,
     * so a plain append is Rust's cards.push: index 0 is the top and the LAST
     * entry is the bottom. */
    tg->state.p[0].deck.n = 0;
    test_add_to_deck_pl(tg, 0, top_marker);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    test_add_to_deck_pl(tg, 0, b1);
    test_add_to_deck_pl(tg, 0, b2);
    test_add_to_deck_pl(tg, 0, b3);
    CHECK_EQ(0, tg->state.p[0].deck.cards[0] - top_marker,
             "d1: deck index 0 is the TOP marker 津島善子 (premise)");
    CHECK_EQ(0, tg->state.p[0].deck.cards[tg->state.p[0].deck.n - 1] - b3,
             "d1: the LAST deck entry is the BOTTOM marker 平安名すみれ (premise)");

    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg, 2);

    /* Rust: while has_pending_choice() { select_option(1) } - the pay/skip
     * choice is ["No", "Yes"], so option 1 is ACCEPT. */
    int guard = 0;
    while (pending(tg) && guard++ < 4) select_option(tg, 1);
    drain_choices(tg, 12);

    CHECK(!deck_has(tg, b3),
          "d1: the BOTTOM-most card must have left the deck (source is "
          "deck_bottom, not the top); deck still holds it = %s",
          deck_has(tg, b3) ? "yes" : "no");
    CHECK(deck_has(tg, top_marker),
          "d1: the TOP marker must still be in the deck - if it left too, the "
          "engine milled from the wrong end");
    EXPECTED_GAP(!deck_has(tg, b3),
        "d1: the bottom-most deck card was not removed (see FAIL above); this "
        "is the deck-bottom mill, and the C's deck_mills_from_deck is known to "
        "take the BOTTOM count-right/end-wrong");
}

/* ================================================================== *
 * E. discard_score_energy_live_recovery_edges_test.rs
 *
 * 桜坂しずく PL!N-bp5-003-R 起動: 手札を1枚控え室に置く -> 自分の控え室から
 * ライブカードを1枚選ぶ -> そうした場合、選んだカードのスコア分のエネルギーを
 * 支払ってもよい：支払った場合、そのカードを自分の手札に加え、…ブレード×3。
 *
 * NOTE ON OVERLAP: the four pay/decline/score-0/unaffordable cases of this
 * ability are ALREADY covered by block B of
 * tests/test_parity_cost_and_effect.c (test_q214_optional_score_energy_pay_
 * and_decline, cases B1-B4). What is NOT covered there is the three EDGE
 * cases in this file, which are what this section ports.
 * ================================================================== */

/* Rust riko-like helper: 3 stage members, hand cost cards, waitroom live. */
static int q214_edge_setup(TestGame *tg, const char *member_no, int n_wait,
                           int *out_live)
{
    int shizuku, filler, filler2, live;
    if (!fx(tg, &shizuku, member_no) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &filler2, HONOKA) || !fx(tg, &live, LIVE_M)) return 0;
    /* The printed ability must be a 起動 so activate_ability is meaningful. */
    int is_kidou = 0;
    int nab = rb_card_num_abilities((uint32_t)shizuku);
    for (int i = 0; i < nab; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (rb_decode_card_ability((uint32_t)shizuku, i, &ab) && ab.triggers &&
            strstr(ab.triggers, "\xe8\xb5\xb7\xe5\x8b\x95")) is_kidou = 1;
        rb_free_ability(&ab);
    }
    CHECK(is_kidou, "e: the %s print carries a 起動 ability", member_no);
    CHECK_EQ(2, card_score(live), "e: the recovery live really scores 2 "
             "(the cost is scaled by its score)");
    set_stage3(tg, 0, filler, shizuku, filler2);
    add_wait(tg, live);
    for (int i = 0; i < n_wait; i++) {
        int extra;
        if (fx(tg, &extra, LIVE_M)) add_wait(tg, extra);
    }
    test_give_energy(tg, 10);
    *out_live = live;
    return 1;
}

/* E1. Rust discard_score_energy_live_recovery_no_live_in_discard_no_selection */
static void e1_q214_no_live_in_waitroom(void)
{
    TestGame *tg = &t_e1;
    fresh(tg);
    int shizuku, filler, filler2, hand_cost;
    if (!fx(tg, &shizuku, SHIZUKU_R) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &filler2, HONOKA) || !fx(tg, &hand_cost, HONOKA)) return;
    set_stage3(tg, 0, filler, shizuku, filler2);
    int hand[1] = { hand_cost };
    set_hand(tg, hand, 1);
    test_give_energy(tg, 5);
    CHECK_EQ(0, tg->state.p[0].discard.n, "e1: the waitroom really is EMPTY (premise)");

    int fired = fire_trigger(tg, shizuku, "\xe8\xb5\xb7\xe5\x8b\x95",
                             "\xe8\xb5\xb7\xe5\x8b\x95");
    CHECK(fired, "e1: the 起動 ability was queued");
    if (pending(tg)) {
        const RbChoice *c = pending_ch(tg);
        CHECK(c->zone[0] && !strcmp(c->zone, "hand"),
              "e1: the mandatory hand-discard cost prompts over the hand "
              "(got \"%s\")", c->zone);
        pick(tg, 0);
    }
    /* With no live in the waitroom the recovery leg has no candidate, so the
     * ability must complete with NOTHING left standing. */
    CHECK(!pending(tg),
          "e1: no live in the waitroom -> no recovery selection prompt "
          "(got %s)", prompt_shape(tg));
    CHECK(!hand_has(tg, hand_cost),
          "e1: the mandatory hand cost WAS still paid (hand n=%d)", hand_n(tg));
    CHECK_EQ(0, wait_n(tg), "e1: the cost card reached the waitroom");
    /* POSITIVE CONTROL on the same shape: a live in the waitroom DOES make the
     * recovery prompt appear, so the green above is evidence. */
    {
        TestGame *p = &t_e2;
        fresh(p);
        int shizuku2, filler2, filler3, hand_cost2, live2;
        if (!fx(p, &shizuku2, SHIZUKU_R) || !fx(p, &filler2, HONOKA) ||
            !fx(p, &filler3, HONOKA) || !fx(p, &hand_cost2, HONOKA) ||
            !fx(p, &live2, LIVE_M)) return;
        set_stage3(p, 0, filler2, shizuku2, filler3);
        int hand[1] = { hand_cost2 };
        set_hand(p, hand, 1);
        add_wait(p, live2);
        test_give_energy(p, 5);
        CHECK(fire_trigger(p, shizuku2, "\xe8\xb5\xb7\xe5\x8b\x95",
                           "\xe8\xb5\xb7\xe5\x8b\x95"),
              "e1c: the control 起動 was queued");
        if (pending(p)) pick(p, 0);
        CHECK_PROMPTS(p, 1,
            "e1c: POSITIVE CONTROL - with a live in the waitroom the recovery "
            "leg MUST prompt, which is what makes e1's 'no prompt' a real "
            "absence rather than an unpayable board");
    }
}

/* E2. Rust discard_score_energy_live_recovery_turn_limit_blocks_second_... */
static void e2_q214_turn_limit_blocks_second(void)
{
    TestGame *tg = &t_e3;
    fresh(tg);
    int live;
    if (!q214_edge_setup(tg, SHIZUKU_R, 1, &live)) return;
    int hand[2] = { tg->state.p[0].stage[0], tg->state.p[0].stage[2] };
    set_hand(tg, hand, 2);
    /* The 2-card deck filler becomes the hand cost; Rust pushes 2 fillers. */
    set_hand(tg, hand, 2);

    CHECK(fire_trigger(tg, tg->state.p[0].stage[1], "\xe8\xb5\xb7\xe5\x8b\x95",
                       "\xe8\xb5\xb7\xe5\x8b\x95"),
          "e2: the 起動 was queued");
    int guard = 0;
    while (pending(tg) && guard++ < 6) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) select_option(tg, 1);  /* PAY */
        else pick(tg, 0);
    }
    drain_choices(tg, 12);
    CHECK(hand_has(tg, live),
          "e2: the first activation recovered the live card to the hand "
          "(energy %d)", rb_energy_active_count(&tg->state.p[0]));
    int hand_before = hand_n(tg);

    /* Second activation in the SAME turn must be refused -- but Rust refuses it
     * at RESOLUTION, not at ENQUEUE. find_printed_ability
     * (abilities.rs:1262-1277) does not consult remaining uses: it enqueues the
     * printed ability regardless, and use_limit_gate (gates.rs:93-111) rejects
     * the entry once pca_resolve_entry reaches it
     * (game_state_abilities.c:2746-2752 frees the ability, advances the queue
     * and returns 0 -- the entry never runs its effect). So a POSITIVE enqueue
     * count from fire_trigger here is Rust-faithful and asserting the opposite
     * pins behaviour Rust does not have.
     *
     * What the twin asserts is the DISJUNCTION
     *   assert!(res.is_err() || !game.has_pending_choice())   (:83)
     * plus the hand-unchanged half (:85). C's fire_trigger returns the ENQUEUE
     * COUNT, not a Result, and the C entry point has no Err channel at all, so
     * the reachable disjunct is `!has_pending_choice()`. It is asserted
     * DIRECTLY -- deliberately with no drain in between: the refused entry
     * creates no prompt at any point, so a drain would make the assertion
     * vacuous (drain_choices exists to keep mandatory prompts from parking
     * mid-ability, and here parking IS the failure). */
    rb_clear_pending_choice(&tg->state);
    (void)fire_trigger(tg, tg->state.p[0].stage[1], "\xe8\xb5\xb7\xe5\x8b\x95",
                       "\xe8\xb5\xb7\xe5\x8b\x95");
    CHECK(!pending(tg),
          "e2: ターン1回 blocks the second activation at RESOLUTION, so no "
          "prompt is ever opened (the `!has_pending_choice()` disjunct of "
          "discard_score_energy_live_recovery_edges_test.rs:83)");
    CHECK_EQ(hand_before, hand_n(tg),
             "e2: the blocked activation consumes no hand card");
    CHECK(!hand_has(tg, live) || hand_before == hand_n(tg),
          "e2: the live is not retrieved a second time");
}

/* E3. Rust discard_score_energy_live_recovery_p_and_ar_variants_work */
static void e3_q214_p_and_ar_variants(const char *no, const char *tag)
{
    TestGame *tg = (strcmp(tag, "p") == 0) ? &t_e4 : &t_e5;
    fresh(tg);
    int live;
    if (!q214_edge_setup(tg, no, 0, &live)) return;
    int hand[1] = { tg->state.p[0].stage[0] };
    set_hand(tg, hand, 1);

    CHECK(fire_trigger(tg, tg->state.p[0].stage[1], "\xe8\xb5\xb7\xe5\x8b\x95",
                       "\xe8\xb5\xb7\xe5\x8b\x95"),
          "e3/%s: the 起動 was queued on the %s print", tag, no);
    int guard = 0;
    while (pending(tg) && guard++ < 6) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) select_option(tg, 1);  /* PAY */
        else pick(tg, 0);
    }
    drain_choices(tg, 12);
    CHECK(hand_has(tg, live),
          "e3/%s: the %s print also recovers the live card to the hand",
          tag, no);
}

static void e3_p(void)  { e3_q214_p_and_ar_variants(SHIZUKU_P,  "p"); }
static void e3_ar(void) { e3_q214_p_and_ar_variants(SHIZUKU_AR, "ar"); }

/* ================================================================== *
 * F. live_start_optional_unit_position_change_heart_blade_test.rs
 *
 * 安養寺姫芽 PL!HS-pb1-006-R ライブ開始時: an OPTIONAL position change into an
 * area occupied by another 『みらくらぱーく！』 member, then heart01 + blade.
 *
 * THE REGRESSION THIS FILE EXISTS FOR (Rust comment): after resolving one
 * position change, a SECOND position change is offered. extra_position_changes
 * must be 0.
 *
 * FULLWIDTH HAZARD (a known live defect, recorded here so a red is not
 * misread): the printed unit is 「みらくらぱーく！」 with a FULLWIDTH U+FF01
 * EXCLAMATION MARK, while cards.json stores units in a mix of forms. The
 * group filter is therefore asserted through the STAGE CONTENT (which
 * mirakura members are staged) rather than by matching the unit string, and
 * the unit string is checked with a strstr on the shared prefix so a
 * fullwidth/ASCII punctuation mismatch cannot masquerade as a stage bug.
 *
 * cards.json: PL!HS-pb1-006-R = 安養寺姫芽, みらくらぱーく！, cost 11
 *             PL!HS-sd1-014-SD = 安養寺 姫芽, みらくらぱーく！, cost 9
 *             PL!HS-sd1-006-SD = 安養寺 姫芽, みらくらぱーく！, cost 15
 * NOTE: all three carry the SAME NAME (安養寺 姫芽 vs 安養寺姫芽). The Rust
 * test treats mk_a and mk_b as two "mirakura destinations" by UNIT, not by
 * name - correct, because the ability filters on the unit. The name equality
 * is asserted below so the distinction is explicit rather than accidental.
 * ================================================================== */

#define MIRAKURA_PREFIX "\xe3\x81\xbf\xe3\x82\x89\xe3\x81\x8f\xe3\x82\x89\xe3\x81\xb1\xe3\x83\xbc\xe3\x81\x8f"  /* みらくらぱーく */

static int f_himeno_walk(TestGame *tg, int himeno, int filler, int live,
                         int a, int b, int c)
{
    set_stage3(tg, 0, a, b, c);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 11);
    advance_to_live_set(tg, 5);
    int hand[1] = { live };
    set_hand(tg, hand, 1);
    test_set_live_card(tg, 0, live);
    clear_decks(tg);
    advance_to_live_start(tg, 2);
    (void)himeno;
    return 1;
}

/* Count and resolve standing 「position|destination」 prompts, returning how
 * many there were. Rust resolve_and_count_position_changes. */
static int f_count_position_changes(TestGame *tg, int resolve_first)
{
    int count = 0, guard = 0;
    while (pending(tg) && guard++ < 12) {
        const RbChoice *c = pending_ch(tg);
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_TARGET &&
            strstr(c->target, "position") != NULL) {
            count++;
            if (!resolve_first && count == 1) return count;
            select_option(tg, 0);
            continue;
        }
        if (c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) { select_option(tg, 0); continue; }
        break;
    }
    return count;
}

/* F1. Rust live_start_unit_swap_two_mirakura_one_position_change */
static void f1_two_mirakura_one_change(void)
{
    TestGame *tg = &t_f1;
    fresh(tg);
    int himeno, mk_a, mk_b, filler, live;
    if (!fx(tg, &himeno, HIMENO6) || !fx(tg, &mk_a, HIMENO_SD) ||
        !fx(tg, &mk_b, HIMENO_SD2) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_M)) return;
    CHECK(mk_a != mk_b, "f1: the two mirakura fixtures are DISTINCT instances");
    CHECK(card_unit_has_prefix(himeno, MIRAKURA_PREFIX),
          "f1: the printed unit starts with みらくらぱーく (got '%s')",
          card_unit_of(himeno));
    CHECK(card_unit_has_prefix(mk_a, MIRAKURA_PREFIX) &&
          card_unit_has_prefix(mk_b, MIRAKURA_PREFIX),
          "f1: both destinations are みらくらぱーく (got '%s' / '%s')",
          card_unit_of(mk_a), card_unit_of(mk_b));

    f_himeno_walk(tg, himeno, filler, live, himeno, mk_a, mk_b);
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1,
        "f1: the optional position change must be OFFERED; the blade "
        "assertion below is vacuous on a silent board");
    int first = f_count_position_changes(tg, 1);
    CHECK_EQ(1, first,
             "f1: exactly ONE position change is resolved (the Rust "
             "assert!(extra_position_changes == 0) is folded in here)");
    /* The swap actually happened: Himeno moved into the chosen destination. */
    CHECK(stage_at(tg, AREA_CENTER) == himeno || stage_at(tg, AREA_RIGHT) == himeno,
          "f1: Himeno ended in a mirakura area (stage = %d/%d/%d)",
          stage_at(tg, AREA_LEFT), stage_at(tg, AREA_CENTER), stage_at(tg, AREA_RIGHT));
    CHECK(blade_of(tg, himeno) > 0,
          "f1: Himeno gained blade (got %d)", blade_of(tg, himeno));
    int extra = f_count_position_changes(tg, 1);
    CHECK_EQ(0, extra,
             "f1: THE REGRESSION - no SECOND position change may be offered "
             "after the first is resolved (got %d)", extra);
}

/* F2. Rust live_start_unit_swap_one_mirakura_one_position_change */
static void f2_one_mirakura_one_change(void)
{
    TestGame *tg = &t_f2;
    fresh(tg);
    int himeno, mk, filler, filler2, live;
    if (!fx(tg, &himeno, HIMENO6) || !fx(tg, &mk, HIMENO_SD) ||
        !fx(tg, &filler, HONOKA) || !fx(tg, &filler2, HONOKA) ||
        !fx(tg, &live, LIVE_M)) return;

    f_himeno_walk(tg, himeno, filler, live, himeno, mk, filler2);
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1, "f2: the position change must be offered");
    int n = f_count_position_changes(tg, 1);
    CHECK_EQ(1, n, "f2: exactly ONE position change (only the centre is mirakura)");
    CHECK_EQ(mk, stage_at(tg, AREA_LEFT), "f2: mk ended at Left");
    CHECK_EQ(himeno, stage_at(tg, AREA_CENTER), "f2: Himeno ended at Center");
    CHECK(blade_of(tg, himeno) > 0, "f2: Himeno gained blade (got %d)",
          blade_of(tg, himeno));
    CHECK_EQ(0, f_count_position_changes(tg, 1),
             "f2: no extra position change follows");
}

/* F3. Rust live_start_unit_swap_skip_no_resources */
static void f3_skip_no_resources(void)
{
    TestGame *tg = &t_f3;
    fresh(tg);
    int himeno, mk, filler, filler2, live;
    if (!fx(tg, &himeno, HIMENO6) || !fx(tg, &mk, HIMENO_SD) ||
        !fx(tg, &filler, HONOKA) || !fx(tg, &filler2, HONOKA) ||
        !fx(tg, &live, LIVE_M)) return;

    f_himeno_walk(tg, himeno, filler, live, himeno, mk, filler2);
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1,
        "f3: the position prompt must be ON SCREEN for a skip to mean "
        "anything; 'no blade' on a silent board is vacuous");
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL && c->kind == RB_CHOICE_SELECT_TARGET &&
          strstr(c->target, "position") != NULL,
          "f3: the prompt is a position|destination SelectTarget (got %s)",
          prompt_shape(tg));
    if (pending(tg)) select_option(tg, -1);   /* skip */
    drain_choices(tg, 12);

    CHECK_EQ(himeno, stage_at(tg, AREA_LEFT), "f3: Himeno stays at Left");
    CHECK_EQ(mk, stage_at(tg, AREA_CENTER), "f3: mk stays at Center");
    CHECK_EQ(0, blade_of(tg, himeno),
             "f3: no blade when the position change is skipped");
}

/* F4. Rust live_start_unit_swap_no_valid_destinations_skips */
static void f4_no_destinations_skips(void)
{
    TestGame *tg = &t_f4;
    fresh(tg);
    int himeno, filler, filler2, live;
    if (!fx(tg, &himeno, HIMENO6) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &filler2, HONOKA) || !fx(tg, &live, LIVE_M)) return;
    CHECK(card_unit_has_prefix(himeno, MIRAKURA_PREFIX),
          "f4: Himeno is みらくらぱーく (got '%s')", card_unit_of(himeno));
    CHECK(!card_unit_has_prefix(filler, MIRAKURA_PREFIX),
          "f4: the filler is NOT みらくらぱーく (got '%s') - otherwise the "
          "premise of 'no valid destination' is false", card_unit_of(filler));

    f_himeno_walk(tg, himeno, filler, live, himeno, filler, filler2);
    drain_auto_abilities(tg, 8);

    CHECK(!pending(tg),
          "f4: no mirakura member besides Himeno -> no destination exists, so "
          "the optional position change must AUTO-SKIP with no prompt "
          "(got %s)", prompt_shape(tg));
    CHECK_EQ(himeno, stage_at(tg, AREA_LEFT), "f4: Himeno stays at Left");
    CHECK_EQ(0, blade_of(tg, himeno), "f4: no position change -> no blade");
}

/* ================================================================== *
 * G. live_start_discard_reactivate_other_member_both_heart04_test.rs
 *
 * エマ・ヴェルデ PL!N-bp3-008-R＋ ライブ開始時:
 *   手札を2枚控え室に置いてもよい：自分のステージにいるこのメンバー以外の
 *   ウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、
 *   これによりアクティブにしたメンバーと、このメンバーは、それぞれ heart04 を得る。
 *
 * THE BEHAVIOURS THAT MATTER (each Rust test isolates one):
 *   G1 one wait member auto-activates with NO second prompt
 *   G2 two wait members -> a real stage choice; ONLY the activated one + Emma
 *   G3 no wait members -> nothing is granted (exclude_self keeps Emma out)
 *   G4 Emma alone -> no other member, so no target
 *   G5 an already-ACTIVE member is never the target
 *
 * The discard-2 hand cost is answered with indices {0,1}, as the Rust does.
 * ================================================================== */

static int g_emma_walk(TestGame *tg, int emma, int filler, int a, int b, int c)
{
    set_stage3(tg, 0, a, b, c);
    int hand[2] = { filler, filler };
    set_hand(tg, hand, 2);
    fill_decks(tg, filler, 30);
    clear_decks(tg);
    /* Rust trigger_discard_reactivate_heart_live_start: queue the FIRST
     * ライブ開始時 ability by its "<card_no>_<full_text>" id, publish
     * activating_card, process. */
    int fired = fire_live_start(tg, emma);
    CHECK(fired, "g: Emma's ライブ開始時 ability was queued");
    return fired;
}

/* The two-card hand cost, answered the way Rust answers it. */
static void g_pay_two_card_cost(TestGame *tg)
{
    if (!pending(tg)) return;
    const RbChoice *c = pending_ch(tg);
    CHECK(c->zone[0] && !strcmp(c->zone, "hand"),
          "g: the discard-2 cost prompts over the HAND (got \"%s\")", c->zone);
    const int two[2] = { 0, 1 };
    pick_many(tg, two, 2);
}

/* G1. Rust live_start_discard_reactivate_heart_activates_wait_both_gain_... */
static void g1_one_wait_member(void)
{
    TestGame *tg = &t_g1;
    fresh(tg);
    int emma, wait_member, filler;
    if (!fx(tg, &emma, EMMA) || !fx(tg, &wait_member, NIJI_A) ||
        !fx(tg, &filler, HONOKA)) return;
    if (!g_emma_walk(tg, emma, filler, wait_member, emma, RB_EMPTY_SLOT)) return;
    rb_mods_set_orientation(&tg->state.mods, wait_member, "wait");
    /* The orientation must be set BEFORE the ability runs; re-arm it here
     * because g_emma_walk already queued the trigger. */
    set_stage3(tg, 0, wait_member, emma, RB_EMPTY_SLOT);
    rb_clear_pending_choice(&tg->state);
    int hand[2] = { filler, filler };
    set_hand(tg, hand, 2);
    fire_live_start(tg, emma);

    CHECK_PROMPTS(tg, 1, "g1: the optional discard-2 cost must be OFFERED");
    g_pay_two_card_cost(tg);
    drain_first(tg, 24);

    CHECK(!pending(tg),
          "g1: a SINGLE wait candidate auto-activates with no selection prompt "
          "(got %s)", prompt_shape(tg));
    CHECK_EQ(1, mh_at(tg, emma, "heart04"), "g1: Emma gets +1 heart04");
    CHECK_EQ(1, mh_at(tg, wait_member, "heart04"),
             "g1: the ACTIVATED member gets +1 heart04");
    CHECK(!is_waited(tg, wait_member),
          "g1: the activated member is no longer in the wait state");
}

/* G2. Rust live_start_discard_reactivate_heart_three_members_only_activated_... */
static void g2_three_members_only_activated(void)
{
    TestGame *tg = &t_g2;
    fresh(tg);
    int emma, member_a, member_b, filler;
    if (!fx(tg, &emma, EMMA) || !fx(tg, &member_a, NIJI_A) ||
        !fx(tg, &member_b, NIJI_B) || !fx(tg, &filler, HONOKA)) return;
    set_stage3(tg, 0, member_a, emma, member_b);
    rb_mods_set_orientation(&tg->state.mods, member_a, "wait");
    rb_mods_set_orientation(&tg->state.mods, member_b, "wait");
    int hand[2] = { filler, filler };
    set_hand(tg, hand, 2);
    CHECK(fire_live_start(tg, emma), "g2: the ライブ開始時 was queued");

    CHECK_PROMPTS(tg, 1, "g2: the optional discard-2 cost must be offered");
    if (pending(tg)) {
        CHECK(strcmp(test_pending_choice_type(tg), "SelectCard") == 0,
              "g2: the cost is a SelectCard (got %s)", prompt_shape(tg));
        const int two[2] = { 0, 1 };
        pick_many(tg, two, 2);
    }
    /* TWO wait members -> a real stage choice. */
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL,
          "g2: two wait members must raise a member-selection prompt (got %s)",
          prompt_shape(tg));
    if (c) {
        CHECK(c->zone[0] && !strcmp(c->zone, "stage"),
              "g2: the member is selected from the STAGE (got \"%s\")", c->zone);
        CHECK_EQ(1, c->count, "g2: exactly one member is activated");
        CHECK(!rb_choice_allow_skip(c),
              "g2: the activation is MANDATORY once the cost is paid "
              "(allow_skip must be 0)");
        pick(tg, 0);
    }
    drain_choices(tg, 16);

    CHECK_EQ(1, mh_at(tg, emma, "heart04"), "g2: Emma gets +1 heart04");
    CHECK_EQ(1, mh_at(tg, member_a, "heart04"),
             "g2: the ACTIVATED member A gets +1 heart04");
    CHECK_EQ(0, mh_at(tg, member_b, "heart04"),
             "g2: the NON-activated wait member B gets nothing");
    CHECK(!pending(tg), "g2: the ability resolved cleanly (got %s)", prompt_shape(tg));
}

/* G3. Rust live_start_discard_reactivate_heart_no_wait_members_no_heart */
static void g3_no_wait_members(void)
{
    TestGame *tg = &t_g3;
    fresh(tg);
    int emma, active_member, filler;
    if (!fx(tg, &emma, EMMA) || !fx(tg, &active_member, NIJI_A) ||
        !fx(tg, &filler, HONOKA)) return;
    set_stage3(tg, 0, active_member, emma, RB_EMPTY_SLOT);
    int hand[2] = { filler, filler };
    set_hand(tg, hand, 2);
    CHECK(fire_live_start(tg, emma), "g3: the ライブ開始時 was queued");

    CHECK_PROMPTS(tg, 1,
        "g3: the cost must still be OFFERED even with no wait member; a 'no "
        "heart' on a board where the ability never ran is vacuous");
    {
        const int two[2] = { 0, 1 };
        if (pending(tg)) pick_many(tg, two, 2);
    }
    drain_first(tg, 24);

    CHECK(!pending(tg), "g3: the ability completes with nothing standing (got %s)",
          prompt_shape(tg));
    CHECK_EQ(0, mh_at(tg, emma, "heart04"),
             "g3: no wait member -> Emma gets NO heart04 (exclude_self keeps "
             "her out of her own activation target)");
    CHECK_EQ(0, mh_at(tg, active_member, "heart04"),
             "g3: the active member gets NO heart04");
    /* POSITIVE CONTROL: G2 already proved a wait member DOES yield heart04 on
     * this same ability, so the two greens together are evidence. */
}

/* G4. Rust live_start_discard_reactivate_heart_alone_no_other_members_... */
static void g4_alone_no_other_members(void)
{
    TestGame *tg = &t_g4;
    fresh(tg);
    int emma, filler;
    if (!fx(tg, &emma, EMMA) || !fx(tg, &filler, HONOKA)) return;
    set_stage3(tg, 0, RB_EMPTY_SLOT, emma, RB_EMPTY_SLOT);
    int hand[2] = { filler, filler };
    set_hand(tg, hand, 2);
    CHECK(fire_live_start(tg, emma), "g4: the ライブ開始時 was queued");

    CHECK_PROMPTS(tg, 1, "g4: the cost must be offered even with no other member");
    {
        const int two[2] = { 0, 1 };
        if (pending(tg)) pick_many(tg, two, 2);
    }
    drain_first(tg, 24);

    CHECK(!pending(tg), "g4: nothing is left standing (got %s)", prompt_shape(tg));
    CHECK_EQ(0, mh_at(tg, emma, "heart04"),
             "g4: Emma ALONE gets no heart04 (exclude_self: the activating "
             "member cannot be her own target)");
}

/* G5. Rust live_start_discard_reactivate_heart_active_member_not_affected */
static void g5_active_member_not_affected(void)
{
    TestGame *tg = &t_g5;
    fresh(tg);
    int emma, active_member, wait_member, filler;
    if (!fx(tg, &emma, EMMA) || !fx(tg, &active_member, NIJI_A) ||
        !fx(tg, &wait_member, NIJI_B) || !fx(tg, &filler, HONOKA)) return;
    set_stage3(tg, 0, active_member, emma, wait_member);
    rb_mods_set_orientation(&tg->state.mods, wait_member, "wait");
    int hand[2] = { filler, filler };
    set_hand(tg, hand, 2);
    CHECK(fire_live_start(tg, emma), "g5: the ライブ開始時 was queued");

    CHECK_PROMPTS(tg, 1, "g5: the cost must be offered");
    {
        const int two[2] = { 0, 1 };
        if (pending(tg)) pick_many(tg, two, 2);
    }
    drain_first(tg, 24);

    CHECK(!pending(tg),
          "g5: only the wait member is eligible, so it auto-activates with no "
          "selection prompt (got %s)", prompt_shape(tg));
    CHECK_EQ(1, mh_at(tg, emma, "heart04"), "g5: Emma gets +1 heart04");
    CHECK_EQ(1, mh_at(tg, wait_member, "heart04"),
             "g5: the activated WAIT member gets +1 heart04");
    CHECK_EQ(0, mh_at(tg, active_member, "heart04"),
             "g5: the already-ACTIVE member gets NOTHING");
}

/* ================================================================== *
 * H. live_start_energy_under_member_draw_all_blades_q157_q158_q184_test.rs
 *
 * 上原歩夢 PL!N-bp3-001-R＋ ライブ開始時 ab#0:
 *   自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。
 *   そうした場合、カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは
 *   ブレード×2 を得る。
 *
 *   Q158  blade +2 applies to ALL members on stage, not just this one
 *   Q157  WAIT energy can be placed (any energy state works)
 *   Q184  energy under a member does NOT count toward the energy count
 *
 * The prompt is a SelectCard over zone "energy" with allow_skip set; the
 * decline is Rust's select_indices(&[]) - an EMPTY selection, not a Skip.
 * ================================================================== */

static int h_ayumu_walk(TestGame *tg, int ayumu, int filler, int live,
                        int a, int b, int c, int n_energy)
{
    set_stage3(tg, 0, a, b, c);
    fill_decks(tg, filler, 10);
    int hand[1] = { live };
    set_hand(tg, hand, 1);
    test_give_energy(tg, n_energy);
    advance_to_live_set(tg, 5);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg, 2);
    (void)ayumu;
    return 1;
}

/* H1. Rust live_start_energy_under_member_q158_blade_all_members */
static void h1_q158_all_members(void)
{
    TestGame *tg = &t_h1;
    fresh(tg);
    int ayumu, left, right, filler, live;
    if (!fx(tg, &ayumu, AYUMU) || !fx(tg, &left, HONOKA) ||
        !fx(tg, &right, HONOKA_13) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_1)) return;
    CHECK(left != right, "h1: the two flanking members are DISTINCT instances");

    h_ayumu_walk(tg, ayumu, filler, live, left, ayumu, right, 3);
    int energy_before = energy_n(tg);

    CHECK_PROMPTS(tg, 1,
        "h1: the optional energy placement must be OFFERED; the blade+2 "
        "assertions below are vacuous on a silent board");
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL && c->kind == RB_CHOICE_SELECT_CARD,
          "h1: the prompt is a SelectCard (got %s)", prompt_shape(tg));
    if (c) {
        CHECK(c->zone[0] && !strcmp(c->zone, "energy"),
              "h1: the energy is selected from the ENERGY zone (got \"%s\")",
              c->zone);
        CHECK(rb_choice_allow_skip(c), "h1: the placement is OPTIONAL (allow_skip)");
        pick(tg, 0);
    }
    drain_choices(tg, 20);

    CHECK_EQ(2, blade_of(tg, left), "h1: Q158 - the LEFT member gets +2 blade");
    CHECK_EQ(2, blade_of(tg, ayumu), "h1: Q158 - the CENTRE member gets +2 blade");
    CHECK_EQ(2, blade_of(tg, right), "h1: Q158 - the RIGHT member gets +2 blade");
    CHECK_EQ(1, under_n(tg, AREA_CENTER),
             "h1: the energy really went UNDER Ayumu (under = %d/%d/%d)",
             under_n(tg, AREA_LEFT), under_n(tg, AREA_CENTER), under_n(tg, AREA_RIGHT));
    CHECK_EQ(energy_before - 1, energy_n(tg),
             "h1: Q184 - the energy left the energy zone");
}

/* H2. Rust live_start_energy_under_member_q158_blade_single_member */
static void h2_q158_single_member(void)
{
    TestGame *tg = &t_h2;
    fresh(tg);
    int ayumu, filler, live;
    if (!fx(tg, &ayumu, AYUMU) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_1)) return;

    h_ayumu_walk(tg, ayumu, filler, live, RB_EMPTY_SLOT, ayumu, RB_EMPTY_SLOT, 3);
    CHECK_PROMPTS(tg, 1, "h2: the energy placement must be offered");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);
    CHECK_EQ(2, blade_of(tg, ayumu), "h2: the single member gets +2 blade");
}

/* H3. Rust live_start_energy_under_member_q158_blade_not_gained_if_not_placed */
static void h3_q158_skip_gives_no_blade(void)
{
    TestGame *tg = &t_h3;
    fresh(tg);
    int ayumu, filler, live;
    if (!fx(tg, &ayumu, AYUMU) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_1)) return;

    h_ayumu_walk(tg, ayumu, filler, live, RB_EMPTY_SLOT, ayumu, RB_EMPTY_SLOT, 3);

    CHECK_PROMPTS(tg, 1,
        "h3: the energy prompt must be ON SCREEN; 'blade stays 0' is only "
        "evidence when the placement was actually offered");
    const RbChoice *c = pending_ch(tg);
    if (c) {
        CHECK(c->zone[0] && !strcmp(c->zone, "energy"),
              "h3: zone is \"energy\" (got \"%s\")", c->zone);
        CHECK(rb_choice_allow_skip(c), "h3: the placement is optional");
        pick_empty(tg);   /* Rust select_indices(&[]) */
    }
    drain_choices(tg, 20);
    CHECK_EQ(0, blade_of(tg, ayumu),
             "h3: declining the placement means no 「そうした場合」 branch and no blade");
    CHECK_EQ(0, under_n(tg, AREA_CENTER), "h3: no energy is tucked under the member");
}

/* H4. Rust live_start_energy_under_member_q157_energy_under_member_uses_any_... */
static void h4_q157_wait_energy(void)
{
    TestGame *tg = &t_h4;
    fresh(tg);
    int ayumu, filler, live, energy_card;
    if (!fx(tg, &ayumu, AYUMU) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_1) || !fx(tg, &energy_card, ENERGY_C)) return;

    set_stage3(tg, 0, RB_EMPTY_SLOT, ayumu, RB_EMPTY_SLOT);
    fill_decks(tg, filler, 10);
    int hand[1] = { live };
    set_hand(tg, hand, 1);
    test_give_energy(tg, 1);
    advance_to_live_set(tg, 5);
    /* Add WAIT energy after the live-card-set passes: it occupies a zone slot
     * but does not raise the active count. */
    add_energy(tg, energy_card);
    CHECK(energy_n(tg) >= 2, "h4: both an active and a wait energy are in the zone (n=%d)",
          energy_n(tg));
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg, 2);

    CHECK_PROMPTS(tg, 1, "h4: Q157 - the energy prompt must be offered");
    if (pending(tg)) pick(tg, 1);   /* Rust select_energy_from_zone(1) */
    drain_choices(tg, 20);
    CHECK_EQ(2, blade_of(tg, ayumu),
             "h4: Q157 - blade +2 applies with ANY energy state, including "
             "wait energy (got %d)", blade_of(tg, ayumu));
}

/* H5. Rust live_start_energy_under_member_q184_energy_under_member_not_counted */
static void h5_q184_energy_not_counted(void)
{
    TestGame *tg = &t_h5;
    fresh(tg);
    int ayumu, filler, live;
    if (!fx(tg, &ayumu, AYUMU) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &live, LIVE_1)) return;

    h_ayumu_walk(tg, ayumu, filler, live, RB_EMPTY_SLOT, ayumu, RB_EMPTY_SLOT, 3);
    int energy_before = energy_n(tg);
    int active_before = rb_energy_active_count(&tg->state.p[0]);

    CHECK_PROMPTS(tg, 1, "h5: the energy prompt must be offered");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);

    CHECK_EQ(energy_before - 1, energy_n(tg),
             "h5: Q184 - exactly 1 energy card is removed from the zone "
             "(%d -> %d)", energy_before, energy_n(tg));
    CHECK_EQ(active_before, rb_energy_active_count(&tg->state.p[0]),
             "h5: Q184 - tucking energy under a member does NOT change the "
             "ACTIVE energy count (%d -> %d)", active_before,
             rb_energy_active_count(&tg->state.p[0]));
    CHECK_EQ(1, under_n(tg, AREA_CENTER),
             "h5: the energy is under the member, not in the energy zone");
}

/* ================================================================== *
 * I. self_discard_same_name_same_area_deploy_energy_under_q157_q184_test.rs
 *
 * 優木せつ菜 PL!N-bp3-007-R 起動 2E: このメンバーをステージから控え室に置く：
 * 自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、
 * このメンバーが置かれていたエリアに登場させる。その後、自分のエネルギー置き場
 * にあるエネルギー1枚をこのメンバーの下に置く。
 *
 * THE TWO REGRESSIONS: the debut must land in the VACATED area, not in
 * stage_first_empty's priority area (Center), and the energy must go under
 * the member in that SAME vacated area.
 *
 * cards.json: PL!N-bp3-007-R = 優木せつ菜, コスト?; PL!N-PR-009-PR = 優木せつ菜.
 * The Rust picks PR-009 because it is cost 2 <= 13, i.e. the printed
 * cost filter is satisfiable - asserted below so a Rust fixture that became
 * unpayable cannot slip through.
 * ================================================================== */

static int i_setsuna_setup(TestGame *tg, int setsuna, int target, int filler, int n_energy)
{
    set_stage3(tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, setsuna);
    int hand[1] = { target };
    set_hand(tg, hand, 1);
    test_give_energy(tg, n_energy);
    CHECK(card_cost(target) >= 0 && card_cost(target) <= 13,
          "i: the debut target really costs <= 13 (printed cost %d), so the "
          "「コスト13以下」 filter is satisfiable", card_cost(target));
    return 0;
}

/* The Rust helper is `game.activate_ability(me)`, i.e. the real 起動 path, not
 * a bare trigger queue. test_activate_ability falls through to rb_activate_card
 * for a member already on stage, which is pca_resolve_entry - the queue
 * consumer that DOES pay the cost. Firing a 起動 through rb_queue_trigger_
 * abilities instead (fire_trigger) skips the cost entirely, which is the same
 * defect section A pins, and it is why the first draft of these four tests saw
 * zero prompts. */
static int i_fire_setsuna(TestGame *tg, int setsuna)
{
    return test_activate_ability(tg, setsuna);
}

/* I1. Rust self_discard_same_name_deploy_q157_wait_energy_can_be_placed_... */
static void i1_q157_wait_energy_placed(void)
{
    TestGame *tg = &t_i1;
    fresh(tg);
    int setsuna, target, filler, energy_card;
    if (!fx(tg, &setsuna, SETSUNA) || !fx(tg, &target, SETSUNA_PR) ||
        !fx(tg, &filler, HONOKA) || !fx(tg, &energy_card, ENERGY_C)) return;
    (void)filler;
    i_setsuna_setup(tg, setsuna, target, filler, 2);
    /* 2 active for the 2E cost + 1 wait for the placement. */
    add_energy(tg, energy_card);
    int total_before = energy_n(tg);
    int active_before = rb_energy_active_count(&tg->state.p[0]);
    CHECK_EQ(2, active_before, "i1: exactly 2 ACTIVE energy for the 2E cost");
    CHECK_EQ(3, total_before, "i1: 3 energy CARDS in the zone (2 active + 1 wait)");

    CHECK(i_fire_setsuna(tg, setsuna), "i1: the 起動 was fired on the real "
          "activate path (rb_activate_card)");
    /* The hand debut target is a single candidate and auto-resolves; the engine
     * then prompts once for the energy. Skip any leading prompts by taking
     * index 0, then require the energy prompt. */
    int guard = 0;
    while (pending(tg) && guard++ < 4) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->zone[0] && !strcmp(c->zone, "energy")) break;
        pick(tg, 0);
    }
    CHECK_PROMPTS(tg, 1, "i1: the energy-under-member SelectCard must appear");
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL && c->kind == RB_CHOICE_SELECT_CARD &&
          c->zone[0] && !strcmp(c->zone, "energy"),
          "i1: the prompt is a SelectCard over zone=energy (got %s)", prompt_shape(tg));
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);

    CHECK_EQ(active_before - 2, rb_energy_active_count(&tg->state.p[0]),
             "i1: 2 active consumed for the cost");
    CHECK_EQ(total_before - 1, energy_n(tg),
             "i1: 1 card popped for the under-member placement "
             "(%d -> %d)", total_before, energy_n(tg));
}

/* I2. Rust self_discard_same_name_deploy_q184_energy_under_member_not_counted */
static void i2_q184_not_counted(void)
{
    TestGame *tg = &t_i2;
    fresh(tg);
    int setsuna, target, filler;
    if (!fx(tg, &setsuna, SETSUNA) || !fx(tg, &target, SETSUNA_PR) ||
        !fx(tg, &filler, HONOKA)) return;
    (void)filler;
    i_setsuna_setup(tg, setsuna, target, filler, 3);

    CHECK(i_fire_setsuna(tg, setsuna), "i2: the 起動 was fired on the real activate path");
    int guard = 0;
    while (pending(tg) && guard++ < 4) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->zone[0] && !strcmp(c->zone, "energy")) break;
        pick(tg, 0);
    }
    CHECK_PROMPTS(tg, 1, "i2: the energy-under-member prompt must appear");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);

    CHECK_EQ(2, energy_n(tg),
             "i2: Q184 - 1 card removed for the placement, 2 remain (got %d)",
             energy_n(tg));
    CHECK_EQ(0, rb_energy_active_count(&tg->state.p[0]),
             "i2: Q184 - 2 active consumed for the cost, 1 removed for the "
             "placement -> 0 remaining (got %d)",
             rb_energy_active_count(&tg->state.p[0]));
}

/* I3. Rust self_discard_same_name_deploy_debuts_to_vacated_area_... */
static void i3_debut_to_vacated_area(void)
{
    TestGame *tg = &t_i3;
    fresh(tg);
    int setsuna, target, filler;
    if (!fx(tg, &setsuna, SETSUNA) || !fx(tg, &target, SETSUNA_PR) ||
        !fx(tg, &filler, HONOKA)) return;
    (void)filler;
    i_setsuna_setup(tg, setsuna, target, filler, 2);

    CHECK(i_fire_setsuna(tg, setsuna), "i3: the 起動 was fired on the real activate path");
    drain_choices(tg, 24);

    CHECK_EQ(target, stage_at(tg, AREA_RIGHT),
             "i3: THE REGRESSION - the new card debuts to the VACATED RightSide "
             "area, not to Center (stage = %d/%d/%d)",
             stage_at(tg, AREA_LEFT), stage_at(tg, AREA_CENTER),
             stage_at(tg, AREA_RIGHT));
    CHECK(!stage_has(tg, setsuna), "i3: the original Setsuna is off the stage");
    CHECK(wait_has(tg, setsuna), "i3: the original Setsuna is in the waitroom");
}

/* I4. Rust self_discard_same_name_deploy_energy_placed_under_vacated_area_... */
static void i4_energy_under_vacated_area(void)
{
    TestGame *tg = &t_i4;
    fresh(tg);
    int setsuna, target, filler;
    if (!fx(tg, &setsuna, SETSUNA) || !fx(tg, &target, SETSUNA_PR) ||
        !fx(tg, &filler, HONOKA)) return;
    (void)filler;
    i_setsuna_setup(tg, setsuna, target, filler, 3);

    CHECK(i_fire_setsuna(tg, setsuna), "i4: the 起動 was fired on the real activate path");
    int guard = 0;
    while (pending(tg) && guard++ < 4) {
        const RbChoice *c = pending_ch(tg);
        if (c && c->zone[0] && !strcmp(c->zone, "energy")) break;
        pick(tg, 0);
    }
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);

    CHECK_EQ(target, stage_at(tg, AREA_RIGHT), "i4: the new card is on RightSide");
    CHECK_EQ(1, under_n(tg, AREA_RIGHT),
             "i4: THE REGRESSION - the energy is under the member on the "
             "RIGHT side, not under Center (under = %d/%d/%d)",
             under_n(tg, AREA_LEFT), under_n(tg, AREA_CENTER),
             under_n(tg, AREA_RIGHT));
    CHECK_EQ(0, under_n(tg, AREA_CENTER), "i4: Center has no energy under it");
    CHECK_EQ(0, under_n(tg, AREA_LEFT), "i4: LeftSide has no energy under it");
}

/* ================================================================== *
 * J. live_start_keep_or_discard_live_then_reveal_to_gain_blade_and_score_test.rs
 *
 * ツナガルコネクト PL!N-bp3-028-L ライブ開始時: look at the top N cards, where
 * N is the number of 『虹ヶ咲』 members on stage; keep any of them on the deck
 * top and send the rest to the waitroom; then REVEAL the new deck top, and gain
 * +1 score if the revealed card is a live card.
 *
 * THE TWO DISTINCT BEHAVIOURS: the KEEP/DISCARD split of the looked-at cards
 * (which is a count-scaled look), and the REVEAL-leg score bonus that fires
 * only on a live card.
 *
 * NOTE ON THE RUST FIXTURE: the Rust test uses `game.new_id(FILLER)` for all
 * 30 deck fillers, i.e. ONE print. That is fine here because no assertion is
 * about WHICH filler moved; the kept-vs-discarded distinction is made by
 * COUNT and by ZONE, not by identity. It is reproduced faithfully.
 * ================================================================== */

/* RUST FIXTURE BUG, CONFIRMED AGAINST cards/cards.json (read as UTF-8).
 * The Rust file uses `game.id("PL!N-bp1-001-R")` and comments it
 * 「// 虹ヶ咲 member cost 9」. cards.json says:
 *     "PL!N-bp1-001-R": { "name": "上原歩夢", "unit": "A・ZU・NA", "cost": 9 }
 * 上原歩夢 is an A・ZU・NA member, NOT a 虹ヶ咲 one. 虹ヶ咲 is a SERIES in
 * cards.json, and NO card in the whole database carries it as a UNIT - the
 * 20 distinct member units are Printemps, BiBi, lilywhite, GuiltyKiss,
 * CYaRon!, AZALEA, A・ZU・NA, QU4RTZ, DiverDiva, R3BIRTH, CatChu!,
 * KALEIDOSCORE, 5yncri5e!, みらくらぱーく！, DOLLCHESTRA, スリーズブーヶ,
 * 蓮ノ空, EdelNote, A-RISE, SaintSnow and SunnyPassion.
 *
 * The printed filter (cards/abilities.json, PL!N-bp3-028-L ab#0) is
 *     "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true
 * i.e. it counts STAGE members whose GROUP is 虹ヶ咲. The Rust fixture
 * therefore does not build the board the test claims. This port keeps the
 * Rust fixture (so the port is faithful) AND adds the series premise as an
 * explicit assertion, so the C result can be read against what was really
 * staged.
 *
 * A real 虹ヶ咲-series member for reference: cards.json series
 * 「ラブライブ！虹ヶ咲学園スクールアイドル同好会」 belongs to
 * PL!N-bp1-017-PR 宮下 愛 (unit DiverDiva).
 */
#define NAGI_CARD "PL!N-bp1-001-R"   /* 上原歩夢, unit A・ZU・NA, cost 9  */
/* 「ラブライブ！虹ヶ咲」 as bytes, for the series premise. The last escape was
 * E3 82 B5 (U+30B5 サ); the corpus series is
 * 「ラブライブ！虹ヶ咲学園スクールアイドル同好会」 with U+54B2 咲, so the
 * strstr at :2967 never matched and card_unit_is reported every Nijigasaki card
 * as non-Nijigasaki. */
#define NIJI_SERIES "\xe3\x83\xa9\xe3\x83\x96\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xef\xbc\x81\xe8\x99\xb9\xe3\x83\xb6\xe5\x92\xb2"

/* Does the decoded card carry 虹ヶ咲 in its SERIES? */
static int card_series_has_niji(int cid)
{
    static char ring[4][256];
    static int  next;
    char *buf = ring[next];
    next = (next + 1) % 4;
    buf[0] = 0;
    Card c;
    memset(&c, 0, sizeof c);
    int found = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.series_idx != UINT16_MAX) {
            const char *s = rb_card_string(c.series_idx);
            if (s) { snprintf(buf, 256, "%s", s); found = strstr(s, NIJI_SERIES) != NULL; }
        }
        rb_free_card(&c);
    }
    return found;
}

static int j_deck_top(TestGame *tg)
{
    return tg->state.p[0].deck.n > 0 ? tg->state.p[0].deck.cards[0] : -1;
}

static int j_walk(TestGame *tg, int tsunagaru, int filler, int n_niji,
                  const int *niji_ids, int deck_top_override)
{
    set_stage3(tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    for (int i = 0; i < n_niji; i++)
        tg->state.p[0].stage[i] = niji_ids[i];
    tg->state.p[0].stage[AREA_RIGHT] = (n_niji >= 3) ? niji_ids[2] : RB_EMPTY_SLOT;
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    tg->state.p[0].deck.n = 0;
    if (deck_top_override >= 0) test_add_to_deck_pl(tg, 0, deck_top_override);
    for (int i = 0; i < 30 - (deck_top_override >= 0 ? 1 : 0); i++)
        test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);
    return 1;
}

/* J1. Rust one_member_keep_card */
static void j1_one_member_keep(void)
{
    TestGame *tg = &t_j1;
    fresh(tg);
    int tsunagaru, niji, filler;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &niji, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA)) return;
    /* THE PREMISE, corrected against cards.json. The Rust comment calls
     * PL!N-bp1-001-R a 虹ヶ咲 member; its UNIT is A・ZU・NA. Asserted here so
     * a green 'no peek' is not read as the 虹ヶ咲 count working. */
    CHECK(!card_unit_is(tg, niji, NIJI_SERIES),
          "j1: the staged member's UNIT is not 虹ヶ咲 (unit='%s') - cards.json "
          "says PL!N-bp1-001-R is A・ZU・NA, so the Rust fixture's "
          "「虹ヶ咲 member cost 9」 comment is wrong", card_unit_of(niji));
    EXPECTED_GAP(card_series_has_niji(niji),
        "j1: EXPECTED GAP - Card.series_idx is not populated by "
        "rb_decode_card_by_index in this build, so the 虹ヶ咲 SERIES premise "
        "cannot be asserted from C. It is established from cards.json instead: "
        "PL!N-bp1-001-R = { name 上原歩夢, unit A・ZU・NA, "
        "series ラブライブ！虹ヶ咲学園スクールアイドル同好会, cost 9 }, so the "
        "card IS 虹ヶ咲 by series while its UNIT is A・ZU・NA.");
    CHECK_EQ(0, card_cost(niji) - 9,
             "j1: the Rust comment's 'cost 9' is right even though the unit is not");

    set_stage3(tg, 0, RB_EMPTY_SLOT, niji, RB_EMPTY_SLOT);
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j1: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1, "j1: the look_and_select prompt must be offered");
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL && c->zone[0] && !strcmp(c->zone, "looked_at"),
          "j1: the prompt selects out of the LOOKED_AT pool (got %s)",
          prompt_shape(tg));
    if (c) {
        CHECK_EQ(1, c->count, "j1: one 虹ヶ咲 member -> look at exactly 1 card");
        CHECK(rb_choice_allow_skip(c), "j1: keeping is optional (skip = keep none)");
        pick(tg, 0);
    }
    drain_choices(tg, 20);

    CHECK_EQ(filler, j_deck_top(tg),
             "j1: the KEPT card is back on the deck top");
    CHECK_EQ(0, wait_n(tg),
             "j1: nothing is discarded when the only looked-at card is kept "
             "(waitroom n=%d)", wait_n(tg));
    CHECK_EQ(1, card_score(tsunagaru),
             "j1: the printed score is 1 (a non-live reveal adds no score)");
    CHECK_EQ(0, score_mod_of(tg, tsunagaru),
             "j1: no score modifier from revealing a member");
}

/* J2. Rust one_member_discard_card */
static void j2_one_member_discard(void)
{
    TestGame *tg = &t_j2;
    fresh(tg);
    int tsunagaru, niji, filler;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &niji, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA)) return;

    set_stage3(tg, 0, RB_EMPTY_SLOT, niji, RB_EMPTY_SLOT);
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j2: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1, "j2: the look_and_select prompt must be offered");
    if (pending(tg)) pick_empty(tg);   /* Rust select_indices(&[]) */
    drain_choices(tg, 20);
    CHECK(wait_has(tg, filler),
          "j2: the looked-at card goes to the WAITROOM when nothing is kept");
    CHECK(filler == j_deck_top(tg),
          "j2: the next deck card is now on top");
}

/* J3. Rust three_members_keep_one */
static void j3_three_members_keep_one(void)
{
    TestGame *tg = &t_j3;
    fresh(tg);
    int tsunagaru, n1, n2, n3, filler;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &n1, NAGI_CARD) ||
        !fx(tg, &n2, NAGI_CARD) || !fx(tg, &n3, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(n1 != n2 && n1 != n3 && n2 != n3,
          "j3: the three 虹ヶ咲 members are DISTINCT instances");
    const int niji[3] = { n1, n2, n3 };

    j_walk(tg, tsunagaru, filler, 3, niji, -1);
    CHECK_PROMPTS(tg, 1, "j3: the look_and_select prompt must be offered");
    const RbChoice *c = pending_ch(tg);
    if (c) {
        CHECK_EQ(3, c->count, "j3: three 虹ヶ咲 members -> look at exactly 3 cards");
        pick(tg, 0);
    }
    drain_choices(tg, 20);
    CHECK_EQ(filler, j_deck_top(tg), "j3: the kept card is on the deck top");
    CHECK_EQ(2, wait_n(tg),
             "j3: the other TWO looked-at cards go to the waitroom (got %d)",
             wait_n(tg));
}

/* J4. Rust three_members_skip_all */
static void j4_three_members_skip_all(void)
{
    TestGame *tg = &t_j4;
    fresh(tg);
    int tsunagaru, n1, n2, n3, filler;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &n1, NAGI_CARD) ||
        !fx(tg, &n2, NAGI_CARD) || !fx(tg, &n3, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA)) return;
    const int niji[3] = { n1, n2, n3 };

    j_walk(tg, tsunagaru, filler, 3, niji, -1);
    CHECK_PROMPTS(tg, 1, "j4: the look_and_select prompt must be offered");
    if (pending(tg)) pick_empty(tg);
    drain_choices(tg, 20);
    CHECK_EQ(3, wait_n(tg),
             "j4: ALL THREE looked-at cards go to the waitroom (got %d)",
             wait_n(tg));
    CHECK_EQ(filler, j_deck_top(tg), "j4: the next deck card is on top");
}

/* J5. Rust reveal_live_card_gets_score_boost */
static void j5_reveal_live_scores(void)
{
    TestGame *tg = &t_j5;
    fresh(tg);
    int tsunagaru, niji, filler, live_card;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &niji, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA) || !fx(tg, &live_card, MUS_LIVE)) return;
    CHECK(card_is_live(live_card),
          "j5: PL!-bp3-019-L really is a LIVE card (the score bonus depends on "
          "it, so this is the premise)");

    set_stage3(tg, 0, RB_EMPTY_SLOT, niji, RB_EMPTY_SLOT);
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    tg->state.p[0].deck.n = 0;
    test_add_to_deck_pl(tg, 0, live_card);
    for (int i = 0; i < 29; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j5: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1, "j5: the look_and_select prompt must be offered");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);
    CHECK_EQ(live_card, j_deck_top(tg),
             "j5: the kept live card is back on the deck top");
    CHECK_EQ(1, score_mod_of(tg, tsunagaru),
             "j5: revealing a LIVE card grants +1 score");
}

/* J6. Rust reveal_non_live_card_no_boost - the paired negative to J5. */
static void j6_reveal_non_live_no_boost(void)
{
    TestGame *tg = &t_j6;
    fresh(tg);
    int tsunagaru, niji, filler;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &niji, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(!card_is_live(filler),
          "j6: the deck-top card is a MEMBER, not a live (the negative arm)");

    set_stage3(tg, 0, RB_EMPTY_SLOT, niji, RB_EMPTY_SLOT);
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    tg->state.p[0].deck.n = 0;
    test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < 29; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j6: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1,
        "j6: the prompt must be ON SCREEN; a zero score modifier on a board "
        "where the ability never ran is vacuous. J5 is the paired positive.");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);
    CHECK_EQ(0, score_mod_of(tg, tsunagaru),
             "j6: revealing a NON-live card grants no score");
}

/* J7. Rust no_niji_members_no_peek */
static void j7_no_niji_no_peek(void)
{
    TestGame *tg = &t_j7;
    fresh(tg);
    int tsunagaru, non_niji, filler;
    /* W9. `tsunagaru` was NEVER assigned in this test: the
       `fx(tg, &tsunagaru, TSUNAGARU)` leg that every sibling (j1 :3005, j2
       :3063, j5 :3139, j6 :3171, j8 :3235) has in its guard was missing here.
       gcc predicted it and nobody read the log --
       `tests/test_parity_cce_gaps.c:438` is where the -Wmaybe-uninitialized
       came from, `-Werror=maybe-uninitialized` now gates tests/.
       The consequence was a live assertion running on an indeterminate value:
       :3212 put that garbage into the hand and :3218 scanned for it, so
       "j7: the ライブ開始時 was queued" could never pass and was failing for a
       setup reason rather than an engine reason. Adding the leg STRENGTHENS
       the test; no assertion is weakened, deleted or ignored. */
    if (!fx(tg, &tsunagaru, TSUNAGARU) ||
        !fx(tg, &non_niji, HONOKA) ||
        !fx(tg, &filler, HONOKA)) return;
    CHECK(!card_unit_is(tg, non_niji, NIJI_SERIES),
          "j7: the staged member's UNIT is not 虹ヶ咲 (unit='%s')", card_unit_of(non_niji));
    EXPECTED_GAP(card_series_has_niji(non_niji),
        "j7: EXPECTED GAP - Card.series_idx is not populated by "
        "rb_decode_card_by_index, so the 虹ヶ咲-series premise cannot be "
        "asserted from C (see the j1 note)");

    set_stage3(tg, 0, RB_EMPTY_SLOT, non_niji, RB_EMPTY_SLOT);
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j7: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);

    CHECK(!pending(tg),
          "j7: 0 虹ヶ咲 members -> NO peek and NO prompt (got %s)",
          prompt_shape(tg));
    CHECK_EQ(30, deck_n(tg), "j7: the deck is unchanged (got %d)", deck_n(tg));
    CHECK_EQ(0, wait_n(tg), "j7: the waitroom is empty");
    /* POSITIVE CONTROL: J5/J6 raise a prompt on a board WITH a 虹ヶ咲 member. */
}

/* J8. Rust three_members_skip_reveal_live_gets_boost */
static void j8_three_skip_reveal_live(void)
{
    TestGame *tg = &t_j8;
    fresh(tg);
    int tsunagaru, n1, n2, n3, filler, live_card;
    if (!fx(tg, &tsunagaru, TSUNAGARU) || !fx(tg, &n1, NAGI_CARD) ||
        !fx(tg, &n2, NAGI_CARD) || !fx(tg, &n3, NAGI_CARD) ||
        !fx(tg, &filler, HONOKA) || !fx(tg, &live_card, MUS_LIVE)) return;
    const int niji[3] = { n1, n2, n3 };
    CHECK(card_is_live(live_card), "j8: the 4th deck card is a LIVE card");

    set_stage3(tg, 0, n1, n2, n3);
    int hand[1] = { tsunagaru };
    set_hand(tg, hand, 1);
    /* 3 look-at fillers, then the live at position 3 (the reveal). */
    tg->state.p[0].deck.n = 0;
    test_add_to_deck_pl(tg, 0, filler);
    test_add_to_deck_pl(tg, 0, filler);
    test_add_to_deck_pl(tg, 0, filler);
    test_add_to_deck_pl(tg, 0, live_card);
    for (int i = 0; i < 26; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    CHECK(fire_live_start(tg, tsunagaru), "j8: the ライブ開始時 was queued");
    drain_auto_abilities(tg, 8);

    CHECK_PROMPTS(tg, 1, "j8: the look_and_select prompt must be offered");
    if (pending(tg)) pick_empty(tg);
    drain_choices(tg, 20);
    CHECK_EQ(3, wait_n(tg),
             "j8: the three looked-at cards go to the waitroom (got %d)",
             wait_n(tg));
    CHECK_EQ(live_card, j_deck_top(tg),
             "j8: the REVEALED live card is on the deck top");
    CHECK_EQ(1, score_mod_of(tg, tsunagaru),
             "j8: revealing that live card grants +1 score");
}

/* ================================================================== *
 * K. live_success_success_count_scales_cost_and_lilywhite_live_retrieval_
 *    pl_pb1_007_test.rs
 *
 * 東條 希 PL!-pb1-007-R 起動 ターン1回: 手札を3枚控え室に置く：自分のステージに
 * ほかの lilywhite のメンバーがいる場合、自分の控え室からμ'sのライブカードを1枚
 * 手札に加える。コストは成功ライブ1枚につき1減る。
 *
 * NOTE ON OVERLAP: the cost-scaling ladder and the group filter are ALREADY
 * covered by tests/test_parity_cost_and_effect.c
 * (test_pb1_007_cost_scales_with_success_count and
 * test_pb1_007_insufficient_hand_and_group_filter). What that file does not
 * cover is the per-step table below expressed as a LADDER with the group
 * gate, the non-μ's live negative, and the ターン1回 re-offer. Those are the
 * behaviours ported here, with a shared board so the ladder is comparable.
 * ================================================================== */

/* The Rust setup_game_with_success: 东條 希 + a lilywhite member, `success`
 * live cards in the SUCCESS zone, `n` fillers in hand, a μ's live in the
 * waitroom. */
static int k_pb1_setup(TestGame *tg, int n_success, int n_hand, int wait_no,
                       int *out_mus, int *out_me, int *out_lily)
{
    int me, lily, mus, filler, succ;
    if (!fx(tg, &me, CHIKA_PB1) || !fx(tg, &lily, LILY_PB3) ||
        !fx(tg, &mus, LIVE_M) || !fx(tg, &filler, HONOKA) ||
        !fx(tg, &succ, SUCC_LIVE)) return 0;
    set_stage3(tg, 0, me, lily, RB_EMPTY_SLOT);
    for (int i = 0; i < n_success; i++) add_success(tg, succ);
    int hand[16];
    for (int i = 0; i < n_hand && i < 16; i++) {
        int f = mid(tg, HONOKA);
        if (f < 0) return 0;
        hand[i] = f;
    }
    set_hand(tg, hand, n_hand > 16 ? 16 : n_hand);
    if (wait_no) add_wait(tg, mus);
    test_give_energy(tg, 10);
    *out_mus = mus; *out_me = me; *out_lily = lily;
    return 1;
}

/* One ladder rung: `success` successes, hand of `n_hand`, expect the cost
 * prompt's count to be `want_count`. */
static void k_pb1_rung(int n_success, int n_hand, int want_count,
                       const char *label, TestGame *tg)
{
    int mus, me, lily;
    fresh(tg);
    if (!k_pb1_setup(tg, n_success, n_hand, 0, &mus, &me, &lily)) return;
    add_wait(tg, mus);
    CHECK(fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95"),
          "k/%s: the 起動 was queued", label);
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL,
          "k/%s: with %d success(es) and %d hand cards the cost MUST prompt "
          "(got %s)", label, n_success, n_hand, prompt_shape(tg));
    if (!c) return;
    CHECK(c->kind == RB_CHOICE_SELECT_CARD && c->zone[0] &&
          !strcmp(c->zone, "hand"),
          "k/%s: the cost is a SelectCard over the hand (got %s)", label,
          prompt_shape(tg));
    if (c) {
        CHECK_EQ(want_count, c->count,
                 "k/%s: 成功ライブ1枚につき1減る - %d success(es) must make the "
                 "cost exactly %d (got %d)", label, n_success, want_count, c->count);
        int idx[4];
        int n = want_count > 0 && want_count <= 4 ? want_count : 0;
        for (int i = 0; i < n; i++) idx[i] = i;
        if (n > 0) pick_many(tg, idx, n);
    }
    drain_choices(tg, 20);
    CHECK(hand_has(tg, mus),
          "k/%s: the μ's live is retrieved to the HAND", label);
}

/* K1. Rust pb1_007_cost_3_with_0_success */
static void k1_cost3_zero_success(void)  { k_pb1_rung(0, 5, 3, "cost3", &t_k1); }
/* K2. Rust pb1_007_cost_2_with_1_success */
static void k2_cost2_one_success(void)   { k_pb1_rung(1, 5, 2, "cost2", &t_k2); }
/* K3. Rust pb1_007_cost_1_with_2_success */
static void k3_cost1_two_success(void)   { k_pb1_rung(2, 5, 1, "cost1", &t_k3); }

/* K4. Rust pb1_007_cost_0_with_3_success - documented in Rust as NOT reachable
 * (the game ends at 3 successes), so the C twin tests the pre-win maximum
 * explicitly and says so, exactly as the Rust test does. */
static void k4_cost0_three_success_is_unreachable(void)
{
    TestGame *tg = &t_k4;
    int mus, me, lily;
    fresh(tg);
    if (!k_pb1_setup(tg, 3, 5, 0, &mus, &me, &lily)) return;
    add_wait(tg, mus);
    CHECK(fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95"),
          "k4: the 起動 was queued with 3 successes staged");
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL,
          "k4: with 3 successes the cost would be 3-3 = 0, so the ability must "
          "still run and must still reach its effect (got %s)", prompt_shape(tg));
    if (c) {
        /* The cost is clamped at 0, so nothing is asked for. */
        CHECK_EQ(0, c->count,
                 "k4: the cost is CLAMPED at 0 with 3 successes (got %d)", c->count);
        pick_empty(tg);
    }
    drain_choices(tg, 20);
    CHECK(hand_has(tg, mus),
          "k4: with a free cost the μ's live is still retrieved");
    EXPECTED_GAP(1, "k4: whether a 3-success board is reachable at all - the "
                    "Rust test explicitly calls this unreachable (the game ends "
                    "at 3 successes) and tests the 2-success maximum instead, "
                    "which section K3 does");
}

/* K5. Rust pb1_007_insufficient_hand_blocked */
static void k5_insufficient_hand_blocked(void)
{
    TestGame *tg = &t_k5;
    int mus, me, lily;
    fresh(tg);
    if (!k_pb1_setup(tg, 0, 2, 0, &mus, &me, &lily)) return;
    add_wait(tg, mus);
    CHECK_EQ(2, hand_n(tg), "k5: the hand really holds only 2 (premise)");
    int fired = fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95");
    const RbChoice *c = pending_ch(tg);
    /* Rust accepts EITHER shape: the activation errors out, or the prompt
     * opens with the full 3 and cannot be answered from 2 cards. What it must
     * NOT do is silently succeed. */
    if (fired && c) {
        CHECK_EQ(3, c->count,
                 "k5: if a prompt is opened with 0 successes it must still ask "
                 "for the full 3 (got %d)", c->count);
        drain_choices(tg, 20);
    } else {
        CHECK(0,
              "k5: with 0 successes and 2 hand cards the activation is refused "
              "outright (fired=%d, no prompt opened) or the full cost is asked; "
              "neither happened", fired);
    }
    CHECK(!hand_has(tg, mus),
          "k5: an unsatisfiable cost must not retrieve the μ's live");
}

/* K6. Rust pb1_007_non_muse_live_not_retrieved */
static void k6_non_muse_live_not_retrieved(void)
{
    TestGame *tg = &t_k6;
    int mus, me, lily, liella_live;
    fresh(tg);
    if (!k_pb1_setup(tg, 1, 5, 0, &mus, &me, &lily)) return;
    if (!fx(tg, &liella_live, LIELLA_LV)) return;
    CHECK(card_is_live(liella_live),
          "k6: PL!S-bp2-024-L really is a LIVE card (it must be LIVE to be a "
          "meaningful negative: the filter is on the μ's GROUP, not the type)");
    add_wait(tg, liella_live);
    CHECK(fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95"),
          "k6: the 起動 was queued");
    const RbChoice *c = pending_ch(tg);
    CHECK(c != NULL, "k6: the cost must still prompt (got %s)", prompt_shape(tg));
    if (c) {
        CHECK_EQ(2, c->count, "k6: 1 success -> cost 2 (got %d)", c->count);
        const int two[2] = { 0, 1 };
        pick_many(tg, two, 2);
    }
    drain_choices(tg, 20);
    CHECK(!hand_has(tg, liella_live),
          "k6: a NON-μ's live is not retrieved even though the cost was paid "
          "and a lilywhite member is on stage");
    CHECK(wait_has(tg, liella_live),
          "k6: the 非μ's live stays in the waitroom");
}

/* K7. Rust pb1_007_turn1_blocks_second */
static void k7_turn1_blocks_second(void)
{
    TestGame *tg = &t_k7;
    int mus, mus2, me, lily;
    fresh(tg);
    if (!k_pb1_setup(tg, 1, 10, 0, &mus, &me, &lily)) return;
    if (!fx(tg, &mus2, LIVE_M)) return;
    add_wait(tg, mus);
    add_wait(tg, mus2);
    CHECK(fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95"),
          "k7: the first 起動 was queued");
    const RbChoice *c = pending_ch(tg);
    if (c) {
        int n = c->count > 0 && c->count <= 4 ? c->count : 1;
        int idx[4];
        for (int i = 0; i < n; i++) idx[i] = i;
        pick_many(tg, idx, n);
    }
    /* There are TWO μ's lives in the waitroom, so a second choice selects
     * which one to retrieve. */
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 20);
    CHECK(hand_has(tg, mus) || hand_has(tg, mus2),
          "k7: the first activation retrieved one of the two μ's lives");
    int hand_after_first = hand_n(tg);

    /* Second activation same turn. Rust's twin is
     *     let res = game.try_activate_ability(me);
     *     assert!(res.is_err(), "turn1 should block second activation")
     * (live_success_success_count_scales_cost_and_lilywhite_live_retrieval_
     *  pl_pb1_007_test.rs:176-177). try_activate_ability forwards to
     * run_main_action, which ENQUEUES and then RESOLVES, so the Err is produced
     * by the resolution-time use_limit_gate (gates.rs:93-111) -- never by the
     * enqueue (abilities.rs:1262-1277).
     *
     * The C has no Result on this path: fire_trigger returns the ENQUEUE COUNT
     * (test_game.c:1173 `int queued = ... - before`), so `res.is_err()` has no
     * C spelling and asserting `!fire_trigger(...)` would pin behaviour Rust
     * does not have. The two companion behavioural assertions are what the
     * refused resolution is observable through: it opens no prompt and it
     * consumes no hand card. Both are asserted without a drain in between,
     * because a refused entry creates no prompt at any point. */
    rb_clear_pending_choice(&tg->state);
    (void)fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95");
    CHECK(!pending(tg), "k7: the blocked activation opens no prompt");
    CHECK_EQ(hand_after_first, hand_n(tg),
             "k7: the blocked activation consumes no hand card");
}

/* K8. Rust pb1_007_cost_0_with_3_success's sibling - the LILYWHITE GATE.
 * The whole ability is gated on 「自分のステージにほかのlilywhiteのメンバーがいる」,
 * so removing the lilywhite member must remove the retrieval. This is the one
 * behaviour in the file that no peer C test covers. */
static void k8_lilywhite_gate_required(void)
{
    TestGame *tg = &t_k8;
    int mus, me, lily, filler;
    fresh(tg);
    if (!fx(tg, &me, CHIKA_PB1) || !fx(tg, &mus, LIVE_M) ||
        !fx(tg, &filler, HONOKA)) return;
    /* 東條 希 alone: no OTHER lilywhite member. */
    set_stage3(tg, 0, me, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    int hand[5];
    for (int i = 0; i < 5; i++) { int f = mid(tg, HONOKA); hand[i] = f; }
    set_hand(tg, hand, 5);
    add_wait(tg, mus);
    test_give_energy(tg, 10);
    CHECK(fire_trigger(tg, me, "\xe8\xb5\xb7\xe5\x8b\x95", "\xe8\xb5\xb7\xe5\x8b\x95"),
          "k8: the 起動 was queued with NO lilywhite ally");
    drain_choices(tg, 24);
    CHECK(!hand_has(tg, mus),
          "k8: without another lilywhite member the μ's live is NOT retrieved");
    /* POSITIVE CONTROL: K1-K3 put a lilywhite member on stage and the same
     * board shape DOES retrieve, so this green is evidence. */
    (void)lily;
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
    run("a22a_queue_path_skips_cost", a22_root_cause_ab);
    run("a22b_resolver_path_pays",    a22_resolver_pays_cost);

    printf("--- B live_start_optional_live_discard_chosen_heart_q190_test.rs "
           "(PL!N-bp4-011-R＋) ---\n");
    run("b1_q190_excludes_all",         b1_q190_offers_heart01_to_06_only);
    run("b2_q190_skip",                 b2_q190_skip_specifies_nothing);
    run("b3_q190_choose_heart05",       b3_q190_choose_heart05);
    run("b4_q190_no_live_in_hand",      b4_q190_no_live_in_hand);

    printf("--- C live_start_optional_unit_discard_draw_unit_cost_gain_test.rs "
           "(PL!HS-bp6-010-R) ---\n");
    run("c1_unit_gain_only_doll",       c1_unit_cost_gain_only_for_dollchestra);
    run("c2_unit_gain_skipped",         c2_unit_cost_gain_skipped);

    printf("--- D live_start_optional_bottom_mill_named_recovery_source_test.rs "
           "(PL!S-bp7-008-R) ---\n");
    run("d1_mari_bottom_mill",          d1_mari_bottom_mill);

    printf("--- E discard_score_energy_live_recovery_edges_test.rs "
           "(PL!N-bp5-003-R/P/AR) ---\n");
    run("e1_q214_no_live_in_waitroom",  e1_q214_no_live_in_waitroom);
    run("e2_q214_turn1_blocks",         e2_q214_turn_limit_blocks_second);
    run("e3_q214_p_variant",            e3_p);
    run("e3_q214_ar_variant",           e3_ar);

    printf("--- F live_start_optional_unit_position_change_heart_blade_test.rs "
           "(PL!HS-pb1-006-R) ---\n");
    run("f1_two_mirakura_one_change",   f1_two_mirakura_one_change);
    run("f2_one_mirakura_one_change",   f2_one_mirakura_one_change);
    run("f3_skip_no_resources",         f3_skip_no_resources);
    run("f4_no_destinations_skips",     f4_no_destinations_skips);

    printf("--- G live_start_discard_reactivate_other_member_both_heart04_test.rs "
           "(PL!N-bp3-008-R＋) ---\n");
    run("g1_one_wait_member",           g1_one_wait_member);
    run("g2_three_only_activated",      g2_three_members_only_activated);
    run("g3_no_wait_members",           g3_no_wait_members);
    run("g4_alone_no_members",          g4_alone_no_other_members);
    run("g5_active_not_affected",       g5_active_member_not_affected);

    printf("--- H live_start_energy_under_member_draw_all_blades_... "
           "(PL!N-bp3-001-R＋) ---\n");
    run("h1_q158_all_members",          h1_q158_all_members);
    run("h2_q158_single_member",        h2_q158_single_member);
    run("h3_q158_skip_no_blade",        h3_q158_skip_gives_no_blade);
    run("h4_q157_wait_energy",          h4_q157_wait_energy);
    run("h5_q184_not_counted",          h5_q184_energy_not_counted);

    printf("--- I self_discard_same_name_same_area_deploy_energy_under_... "
           "(PL!N-bp3-007-R) ---\n");
    run("i1_q157_wait_energy_placed",   i1_q157_wait_energy_placed);
    run("i2_q184_not_counted",          i2_q184_not_counted);
    run("i3_debut_to_vacated_area",     i3_debut_to_vacated_area);
    run("i4_energy_under_vacated",      i4_energy_under_vacated_area);

    printf("--- J live_start_keep_or_discard_live_then_reveal_... "
           "(PL!N-bp3-028-L) ---\n");
    run("j1_one_member_keep",           j1_one_member_keep);
    run("j2_one_member_discard",        j2_one_member_discard);
    run("j3_three_keep_one",            j3_three_members_keep_one);
    run("j4_three_skip_all",            j4_three_members_skip_all);
    run("j5_reveal_live_scores",        j5_reveal_live_scores);
    run("j6_reveal_non_live_no_boost",  j6_reveal_non_live_no_boost);
    run("j7_no_niji_no_peek",           j7_no_niji_no_peek);
    run("j8_three_skip_reveal_live",    j8_three_skip_reveal_live);

    printf("--- K live_success_success_count_scales_cost_... "
           "(PL!-pb1-007-R) ---\n");
    run("k1_cost3_zero_success",        k1_cost3_zero_success);
    run("k2_cost2_one_success",         k2_cost2_one_success);
    run("k3_cost1_two_success",         k3_cost1_two_success);
    run("k4_cost0_unreachable",         k4_cost0_three_success_is_unreachable);
    run("k5_insufficient_hand",         k5_insufficient_hand_blocked);
    run("k6_non_muse_live",             k6_non_muse_live_not_retrieved);
    run("k7_turn1_blocks_second",       k7_turn1_blocks_second);
    run("k8_lilywhite_gate_required",   k8_lilywhite_gate_required);
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
