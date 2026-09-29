/* test_parity_per_card2.c - C parity suite for the Rust clusters
 *
 *   engine/tests/test_modules/effects/recover/per_card/   (14 files / 68 tests)
 *   engine/tests/test_modules/effects/choice/per_card/    (9 files / 25 tests)
 *
 * BOUNDARY (established from the BODIES of the existing C files, not their
 * header claims -- a header claim has been wrong three times now):
 *
 * recover/per_card -- already covered, NOT repeated here:
 *   multi_name_card_name_and_slot_rulings_q62_q65_q105_q207_q208  (5 tests)
 *       test_parity_deploy_recover.c PART B1
 *   cannot_baton_touch_restriction_blocks_baton_touch_action      (2 tests)
 *       deploy_recover.c B2 (+B2b positive control)
 *   movement_tracking_characterization                           (6 tests)
 *       deploy_recover.c B3 (rs:19-108, 4 views) + B4 (rs:117-179, the
 *       activation-discard-cost integration half)
 *   constant_total_score_bonus_requires_distinct_hasunosora_names (3)
 *       deploy_recover.c B5
 *   constant_two_blade_gain_when_either_stage_has_cost13_plus      (6)
 *       deploy_recover.c B6
 *   duplicate_card_ids_on_stage_and_mulligan_toggle               (2)
 *       deploy_recover.c B7
 *   live_success_highest_blade_niji_member_scores_plus_one       (5)
 *       deploy_recover.c B8
 *   live_start_success_zone_plus_three_distinct_names_score_plus_one (4)
 *       deploy_recover.c B9
 *   live_start_aqours_center_cost_nine_blade_gain_or_opponent_wait (8)
 *       deploy_recover.c B10
 *   live_start_activate_three_printemps_wait_members_q178_q179   (3)
 *       deploy_recover.c B11
 *   live_success_per_wait_member_score_and_yell_reveal_to_deck_bottom
 *       test 1 -> deploy_recover.c B12; test 2 -> test_parity_mill_deck.c
 *       (S_BP2_021_L)
 *   live_success_blocks_both_success_zones_when_scores_tied_q36  (2)
 *       test_parity_restriction.c (MEBIUS)
 *   live_success_all_areas_hasunosora_constant_total_score_bonus (8)
 *       the CONDITION half is deploy_recover.c B5 and
 *       test_parity_integration_extra.c (KOZUE_NO / MULTINAME_NO); the
 *       SCORE-TOTAL half (live_score_includes_global_bonus /
 *       live_victory_p1_wins / live_score_no_bonus) is not exercised by
 *       either, and is NOT taken here -- recorded in the final report.
 *
 * recover/per_card -- TAKEN HERE (the one file no C test names at all):
 *   live_start_member_count_branches_draw_discard_heart_and_score_test.rs
 *   (12 tests, PL!-bp5-021-L SUNNY DAY SONG). `PL!-bp5-021-L` appears in
 *   NO file under engine_c_wip/tests/, so the whole block is uncovered.
 *
 * choice/per_card -- already covered, NOT repeated here (bodies of
 * test_parity_target_player.c):
 *   energy_or_recycle_live_pl_n_pb1_010_r_test.rs   (2) GROUP I
 *   mill_or_wait_pl_hs_cl1_004_cl_test.rs           (2) GROUP J
 *   wait_or_draw_pl_s_bp7_025_l_test.rs             (2) GROUP K
 *   discard_self_from_hand_no_named_blade_target_q258 (1) GROUP L
 *   arise_gated_activate_blade_or_wait_private_wars  (3) GROUP M
 *   -- and the 8 tests of conditional_choice_options_execution_test.rs:
 *      dia_choose_blade is the same board+option as GROUP F
 *      (selection_filter/blade_option_exclude_self), the two kotori tests
 *      are test_parity_conditional_gated.c group H (PL!-bp5-003-R+), and
 *      the three dia position_change tests need game.generated_actions()
 *      which rb_generate_action_candidates() does not emit -- see below.
 *
 * choice/per_card -- TAKEN HERE:
 *   tang_keke_energy_or_heart06_choice_test.rs               (6) GROUP B
 *   daydream_mermaid_live_success_energy_or_recover_choice   (3) GROUP C
 *   kanan_bp7_two_debut_abilities_queue_test.rs              (1) GROUP D
 *   conditional_choice_options_execution_test.rs: bouken_*   (3) GROUP E
 *
 * HARNESS FACTS used throughout (all confirmed by peers, all re-measured
 * here rather than assumed):
 *   - `load_card_db()` falls back to ../cards/build: rb_load("src") fails
 *     under the isolated out-of-tree build.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - the flat RbChoice carries no option list, so a multi-option prompt's
 *     per-option labels are an EXPECTED_GAP, never a pass.
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE; the
 *     engine's own table (printed at startup from rb_parse_heart_color) is
 *     0=heart00 .. 6=heart06. All heart reads here go through
 *     rb_mods_get_heart() with an index derived from the engine.
 *   - rb_card_string() takes a STRING-TABLE INDEX, so card_no_of() decodes
 *     the card first and passes c.card_no_idx.
 *   - mid() == test_new_id(), never test_id() (a shared template aliases).
 *   - every TestGame is static: sizeof(GameState) is ~781 KB.
 *   - rb_create_card_copy uses a process-global monotonic pool nothing
 *     resets, so rb_card_copy_pool_reset() runs once per case.
 *
 * VACUITY CONTROL. A per_card cluster is a grid of one card x one behaviour,
 * so a single no-op engine makes a whole column read green. Every negative
 * claim below sits behind CHECK_PROMPTS(n), counting prompts at the point the
 * test states they are ON SCREEN (PROMPT_PRESENT), or behind a paired
 * POSITIVE CONTROL on the same board (the SUNNY delta tests pair SUNNY
 * against a live with no ライブ開始時 at all).
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/mman.h>

/* U+FF0B FULLWIDTH PLUS SIGN, byte-exact. The Write tool can emit CP932 for a
 * literal Japanese byte sequence and silently corrupt the filter, so every
 * non-ASCII literal in this file is an explicit escape. */
#define FP "\xef\xbc\x8b"

/* ---------------------------------------------------------------- fixtures */
#define SUNNY    "PL!-bp5-021-L"   /* SUNNY DAY SONG, live, 3 conditional tiers */
#define CTRL     "PL!-sd1-019-SD"  /* control live: carries NO ライブ開始時      */
#define KEKE     "PL!SP-pb2-002-R"  /* 唐 可可, cost 11, 起動 use_limit 1        */
#define KEKE_BH  "PL!SP-PR-003-PR"  /* 澁谷かのん, cost 2, HAS a blade heart     */
#define KEKE_NB  "PL!SP-PR-004-PR"  /* 唐 可可, cost 4, no blade heart          */
#define KEKE_LIVE "PL!SP-pb2-045-L" /* 絶対的LOVER -- a REAL Liella! live      */
#define MERMAID  "PL!N-bp4-030-L"   /* Daydream Mermaid, live                   */
#define KANAN    "PL!S-bp7-003-P" FP
#define BOUKEN   "PL!S-bp6-020-L"   /* 冒険Type A, B, C!!                      */
#define ENERGY   "LL-E-001-SD"
#define H05      "PL!S-sd1-003-SD"
#define H06      "PL!-PR-015-PR"
#define FILLER   "PL!-sd1-010-SD"   /* 南ことり, mu's                           */
#define RIN      "PL!-sd1-005-SD"   /* 星空 凛,  mu's                           */
#define UMI      "PL!-sd1-006-SD"   /* 西木野 真姫, mu's                        */
#define AQOURS_A "PL!S-sd1-013-SD"  /* 黒澤ダイヤ, Aqours                       */
#define AQOURS_B "PL!S-sd1-010-SD"  /* 高海千歌, Aqours                        */
#define GENERIC  "PL!-sd1-013-SD"  /* 園田海未, mu's but the generic filler    */
#define JOINT_BP3 "LL-bp3-001-R" FP /* 園田海未&津島善子&天王寺璃奈            */

/* "mu's" as bytes. The engine's own group token, read back from the decoded
 * ability (RB_DUMP prints group_names=mu's), so a literal here would be a CP932
 * hazard. "Aqours" is a UNIT-level name, "Liella!" a SERIES-level one: the C
 * matcher accepts both, and a series-level name matches every card of that
 * series, so group guards must be written as the negation when that is the
 * claim (a peer measured this in test_parity_recover_to_hand.c group_routes). */
#define MUS "\xce\xbc's"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ---------------------------------------------------------------- harness */
/* The counters live in a MAP_SHARED anonymous mapping, not in BSS: every case
 * runs in a FORKED child, and a plain static would take the child's tally with
 * it at _Exit(). With the mapping, a child that dies mid-assertion still leaves
 * the parent holding the real numbers. */
typedef struct {
    int assertions, failures, setup_bugs, gaps, prompts;
} Counters;
static Counters *C;
#define assertions (C->assertions)
#define failures   (C->failures)
#define setup_bugs (C->setup_bugs)
#define gaps       (C->gaps)
#define prompts    (C->prompts)

static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB, so every TestGame is static (BSS). One slot
 * per case; cases run in forked children, so slots are never shared. */
static TestGame tg_a, tg_b, tg_c, tg_d, tg_e, tg_f, tg_f2, tg_g, tg_g2, tg_h,
              tg_h2, tg_i, tg_j, tg_j2, tg_k, tg_k2, tg_l, tg_l2, tg_m, tg_m2,
              tg_n, tg_n2, tg_o, tg_o2, tg_p, tg_p2, tg_q, tg_r;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions before the fault: %d, failures: %d, prompts: %d\n",
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

#define CHECK_EQ_STR(actual, expected, ...) do {                          \
        assertions++;                                                     \
        const char *a_ = (actual);                                        \
        const char *e_ = (expected);                                      \
        if (!a_ || strcmp(a_, e_) != 0) {                                 \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got \"%s\" expected \"%s\")\n",            \
                    a_ ? a_ : "(null)", e_);                              \
        }                                                                 \
    } while (0)

/* A Rust assertion with no faithful C entry point. Recorded, never a pass and
 * never a failure. */
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
 * before the negative claim below it can be trusted. */
#define CHECK_PROMPTS(n, ...) do {                                       \
        assertions++;                                                     \
        if (prompts < (n)) {                                              \
            failures++;                                                   \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) were offered, need %d\n",\
                    prompts, (n));                                        \
        }                                                                 \
    } while (0)

/* The single place a prompt is counted: at the moment the test claims it is on
 * screen, so CHECK_PROMPTS can sit BEFORE the answer and still be a control. */
#define PROMPT_PRESENT(tg, cond, ...) do {                               \
        int pp_ = pending(tg) && (cond);                                 \
        CHECK(pp_, __VA_ARGS__);                                         \
        if (pp_) prompts++;                                              \
    } while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rotating ring: a single static buffer called 2-3 times in one printf prints
 * the LAST card repeatedly -- a false-red generator confirmed by peers. */
static const char *card_no_of(int cid)
{
    static char buf[8][96];
    static int ring;
    char *b = buf[ring];
    Card c;
    ring = (ring + 1) % 8;
    b[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(b, 96, "%s", s);
        rb_free_card(&c);
    }
    return b;
}

/* AGENTS.md identity rule: mid() (test_new_id), never the shared template. */
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

static int pin_id(int cid, const char *no)
{
    assertions++;
    if (cid < 0 || !rb_card_no_eq(cid, no)) {
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
                no, card_no_of(cid), cid);
        return 0;
    }
    return 1;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int printed_cost(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int score_of(TestGame *tg, int cid)  { return rb_mods_get_score(&tg->state.mods, cid); }

/* Heart reads go through rb_mods_get_heart, never test_get_heart_modifier: the
 * shim REMAPS colour 5 onto RB_HEART_ORANGE while the engine's own table is
 * 0=heart00 .. 6=heart06. Indices come from rb_parse_heart_color() at startup
 * and the resolved map is printed there too. */
static int idx00, idx01, idx02, idx03, idx04, idx05, idx06;
static int heart_of(TestGame *tg, int cid, int idx) { return rb_mods_get_heart(&tg->state.mods, cid, idx); }

/* ---- zone helpers ---- */
static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(const TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].discard, cid); }
static int stage_at(const TestGame *tg, int pl, int i) { return tg->state.p[pl].stage[i]; }
static int stage_count(const TestGame *tg, int pl)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[pl].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}
static int hand_index_of(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == cid) return i;
    return -1;
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0; P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_selected_cards = 0;
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0; P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
}

/* Rust game.new_id(FILLER) x n -- fresh DISTINCT copies each time. */
static int fill_deck(TestGame *tg, int pl, int count)
{
    int made = 0;
    for (int i = 0; i < count; i++) {
        int id = test_new_id(tg, FILLER);
        if (id < 0) break;
        RbPlayer *P = &tg->state.p[pl];
        if (P->deck.n >= RB_MAX_ZONE) break;
        P->deck.cards[P->deck.n++] = id;
        made++;
    }
    return made;
}
static void fill_decks(TestGame *tg, int count) { fill_deck(tg, 0, count); fill_deck(tg, 1, count); }

/* ---- choice plumbing ---- */
static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }
static const char *pending_type(TestGame *tg)
{
    const char *t = test_pending_choice_type(tg);
    return t ? t : "";
}
static int pending_is_card(TestGame *tg)  { return pending(tg) && !strcmp(pending_type(tg), "SelectCard"); }
static int pending_is_auto(TestGame *tg) { return pending(tg) && !strcmp(pending_type(tg), "SelectAutoAbility"); }
static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}
static void select_option(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    rb_resume_with_choice(&tg->state, idx);
}
static void drain_choices(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        if (pending_is_card(tg)) pick(tg, 0);
        else select_option(tg, 0);
    }
}
static void drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
}
static int play_to_stage(TestGame *tg, int cid, int area)
{
    int r = test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, 1);
        if (tg->state.p[0].stage[area] == cid) break;
        r = test_play_to_stage(tg, cid, area);
    }
    return r;
}

/* Rust advance_to_live_card_set_p1: five passes, Main -> Active -> Energy ->
 * Draw -> Main -> LiveCardSet. Stepped one at a time so a standing prompt is
 * visible instead of being auto-answered. */
static void advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) { test_pass(tg); drain_choices(tg, 4); }
}
/* Rust advance_to_live_start: two more passes into PERFORMANCE. */
static void advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* Walks forward until the ライブ成功時 dispatch is on screen or p1's success is
 * already recorded. It STOPS on a standing prompt, never past it.
 * (test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) is deliberately NOT used:
 * it destroys the yell window and answers every standing prompt with index 0.) */
/* Walks forward until the ライブ成功時 dispatch is actually ON SCREEN. It STOPS
 * on the prompt, never past it, and it deliberately does NOT treat
 * state.live_success[] as terminal: measured, that flag is already 1 one pass
 * BEFORE the ability's choice is raised, so returning on it makes the caller's
 * "the choice is offered" check silently vacuous.
 * (test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) is deliberately NOT used:
 * it destroys the yell window and answers every standing prompt with index 0.) */
static int advance_to_live_success(TestGame *tg)
{
    for (int i = 0; i < 10; i++) {
        if (pending(tg)) return 1;
        test_pass(tg);
        if (pending(tg)) return 1;
        drain_choices(tg, 4);
    }
    return pending(tg);
}

/* What the engine actually put on screen, so an assertion about a prompt
 * survives a walk that had to ANSWER it to keep the phase machine moving. */
typedef struct {
    int seen;
    int n_prompts;
    char type[32];
    char zone[32];
    int count;
} SeenChoice;

static void note_choice(TestGame *tg, SeenChoice *s)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    s->seen = 1;
    s->n_prompts++;
    snprintf(s->type, sizeof s->type, "%s", pending_type(tg));
    if (c) { snprintf(s->zone, sizeof s->zone, "%s", c->zone); s->count = c->count; }
}

/* `passes` phase steps, answering each non-SelectCard prompt with option 0 and
 * STOPPING dead on the first SelectCard, which only the test may answer.
 * The option-0 answer is deliberate: for a ライブ開始時 multi-option choice it
 * is the first printed option, so the walk is faithful rather than a shortcut. */
static int walk_passes(TestGame *tg, int passes, SeenChoice *s)
{
    for (int i = 0; i < passes; i++) {
        test_pass(tg);
        int guard = 0;
        while (pending(tg) && guard++ < 8) {
            note_choice(tg, s);
            if (pending_is_card(tg)) return s->seen;
            select_option(tg, 0);
        }
    }
    return s->seen;
}

/* ========================================================================
 * GROUP A -- recover/per_card
 * live_start_member_count_branches_draw_discard_heart_and_score_test.rs
 *
 * PL!-bp5-021-L (SUNNY DAY SONG) ライブ開始時, three conditional tiers:
 *   >=1 member : 自分と相手はカードを1枚引き、手札を1枚控え室に置く
 *   >=2 members: 自分のステージにいる『μ's』のメンバー1人に heart03
 *   >=3 and all names differ: このカードのスコアを＋１
 *
 * The decoded C shape (RB_DUMP=PL!-bp5-021-L) is a FLAT `sequential` with
 * three children and NO member-count condition attached to any of them:
 *   [1] action=sequential  (no condition)
 *     [2] action=sequential  (no condition)
 *       [3] action=draw_card   src=deck dst=hand target=both count=1
 *       [3] action=move_cards  src=hand dst=discard       count=1
 *     [2] action=gain_resource heart03 group=mu's target_count=1
 *     [2] action=modify_score  value=1 operation=add
 * The three tiers are therefore expected to be a PARSER LOSS, and the
 * negatives in A8/A9/A10 are the ones that will go red.
 * ======================================================================== */

typedef struct {
    int p1h, p1w, p2h, p2w, score;
} WindowDeltas;

/* Rust live_window_deltas(live_no, stage_member). Hand `live_no` in, put one
 * member at CENTER if asked, walk the real performance window, and report how
 * much each seat's hand and waitroom grew plus the live's score modifier.
 * Hand size alone cannot be read directly: the first attacker draws inside the
 * window whatever the live is, so the DIFFERENCE against a control live with
 * no ライブ開始時 is the only readable signal for tier 1. */
static int live_window_deltas(TestGame *tg, const char *live_no, const char *member_no,
                              WindowDeltas *out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int live = mid(tg, live_no);
    pin_id(live, live_no);
    int filler = test_new_id(tg, GENERIC);
    if (live < 0 || filler < 0) return 0;
    if (member_no) {
        int m = mid(tg, member_no);
        pin_id(m, member_no);
        if (m < 0) return 0;
        tg->state.p[0].stage[AREA_CENTER] = m;
    }
    test_add_to_hand(tg, live);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, live);
    CHECK_EQ(tg->state.p[0].live.n, 1, "window: exactly one live is in the zone");
    CHECK_EQ(tg->state.p[0].live.cards[0], live, "window: it is the live under test");

    int p1h = tg->state.p[0].hand.n, p1w = tg->state.p[0].discard.n;
    int p2h = tg->state.p[1].hand.n, p2w = tg->state.p[1].discard.n;

    advance_to_live_start(tg);
    drain_pick0(tg, 12);

    out->p1h = tg->state.p[0].hand.n - p1h;
    out->p1w = tg->state.p[0].discard.n - p1w;
    out->p2h = tg->state.p[1].hand.n - p2h;
    out->p2w = tg->state.p[1].discard.n - p2w;
    out->score = score_of(tg, live);
    return 1;
}

/* A1. sunny_branch1_1_member_triggers_draw -- one member, the hand-discard
 * prompt is a SelectCard over the hand with count 1. */
static void test_a1_sunny_one_member_draws_for_both(void)
{
    TestGame *tg = &tg_a;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int rin = mid(tg, RIN);
    int filler = test_new_id(tg, GENERIC);
    pin_id(sunny, SUNNY);
    pin_id(rin, RIN);
    if (sunny < 0 || rin < 0 || filler < 0) return;
    CHECK(card_prints_trigger(sunny, RB_TSTR_LIVE_START), "a1: SUNNY really prints ライブ開始時");
    CHECK(!card_prints_trigger(filler, RB_TSTR_LIVE_START),
          "a1 CONTROL: the filler does not, so nothing else opens the window");

    tg->state.p[0].stage[AREA_CENTER] = rin;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);
    test_add_to_hand_for(tg, 1, filler);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a1: the tier-1 hand-discard SelectCard is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "a1: the hand-discard prompt must be answered before the draw claim");
    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "hand", "a1: the discard is over the HAND");
        CHECK_EQ(c->count, 1, "a1: 「手札を1枚控え室に置く」 -- exactly one card");
    }
    pick(tg, 0);
    drain_pick0(tg, 8);

    int p2_total = tg->state.p[1].hand.n + tg->state.p[1].discard.n;
    CHECK(p2_total > 0, "a1: P2 gained cards in the window (hand=%d wait=%d)",
          tg->state.p[1].hand.n, tg->state.p[1].discard.n);
    CHECK(p2_total >= 2, "a1: 「自分と相手はカードを1枚引き」 then 「手札を1枚控え室に置く」 "
          "-- P2's hand+waitroom total is >= 2 (got %d)", p2_total);
}

/* A2. sunny_branch1_no_members_does_nothing -- SUNNY on an EMPTY stage must
 * match a live with no ライブ開始時 at all, on every one of the four counters,
 * and must carry no score modifier. */
static void test_a2_sunny_no_members_is_a_noop(void)
{
    WindowDeltas s, c;
    if (!live_window_deltas(&tg_b, SUNNY, NULL, &s)) return;
    if (!live_window_deltas(&tg_c, CTRL,  NULL, &c)) return;
    CHECK_EQ(s.p1h, c.p1h, "a2: p1 hand delta matches the control live");
    CHECK_EQ(s.p1w, c.p1w, "a2: p1 waitroom delta matches the control live");
    CHECK_EQ(s.p2h, c.p2h, "a2: p2 hand delta matches the control live");
    CHECK_EQ(s.p2w, c.p2w, "a2: p2 waitroom delta matches the control live");
    CHECK_EQ(s.score, 0, "a2: 「このカードのスコアを＋１する」 needs 3+ differently-named members");
}

/* A3. sunny_branch1_one_member_draws_for_p1 -- one member must make p1 gain one
 * more card inside the window than the control live does. PAIRED POSITIVE
 * CONTROL: the same window with a live that has no ライブ開始時. */
static void test_a3_sunny_one_member_beats_control(void)
{
    WindowDeltas s, c;
    if (!live_window_deltas(&tg_d, SUNNY, FILLER, &s)) return;
    if (!live_window_deltas(&tg_e, CTRL,  FILLER, &c)) return;
    CHECK(s.p1h > c.p1h,
          "a3: 「自分のステージにメンバーが1人以上いる場合 … カードを1枚引く」 -- "
          "p1 gains more than the control live (sunny %d vs control %d)", s.p1h, c.p1h);
}

/* Shared board for the 3-member branches. */
static int sunny_three_board(TestGame *tg, const char *a, const char *b, const char *cc,
                             int *sunny_out, int *m1, int *m2, int *m3)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int x1 = mid(tg, a), x2 = mid(tg, b), x3 = mid(tg, cc);
    pin_id(sunny, SUNNY);
    *sunny_out = sunny; *m1 = x1; *m2 = x2; *m3 = x3;
    if (sunny < 0 || x1 < 0 || x2 < 0 || x3 < 0) return 0;
    CHECK(x1 != x2 && x2 != x3 && x1 != x3,
          "setup: the three stage members are three DISTINCT card instances");
    tg->state.p[0].stage[AREA_CENTER] = x1;
    tg->state.p[0].stage[AREA_LEFT]   = x2;
    tg->state.p[0].stage[AREA_RIGHT]  = x3;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);
    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);
    return 1;
}

/* A4. sunny_branch3_3_members_score_plus_1 */
static void test_a4_sunny_three_distinct_names_score_plus_1(void)
{
    TestGame *tg = &tg_f;
    int sunny, m1, m2, m3;
    if (!sunny_three_board(tg, RIN, FILLER, UMI, &sunny, &m1, &m2, &m3)) return;
    drain_pick0(tg, 12);
    CHECK_EQ(score_of(tg, sunny), 1,
             "a4: 3 differently-named members -> 「このカードのスコアを＋１する」");
}

/* A5. sunny_branch3_3_members_duplicate_name_no_score.
 * PAIRED POSITIVE CONTROL: A4 on the same card with three distinct names.
 * The duplicate here is a SECOND INSTANCE (mid), not the shared template, so
 * the board genuinely holds two Rin and one Kotori. */
static void test_a5_sunny_duplicate_name_no_score(void)
{
    TestGame *tg = &tg_f2;
    int rin_a = mid(tg, RIN), rin_b = mid(tg, RIN), kotori = mid(tg, FILLER);
    pin_id(rin_a, RIN);
    if (rin_a < 0 || rin_b < 0 || kotori < 0) return;
    CHECK(rin_a != rin_b, "a5: the two Rins are two distinct instances, not one aliased id");
    CHECK(rb_card_no_eq(rin_a, RIN) && rb_card_no_eq(rin_b, RIN),
          "a5: both copies really are the PL!-sd1-005-SD print");

    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    pin_id(sunny, SUNNY);
    if (sunny < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = rin_a;
    tg->state.p[0].stage[AREA_LEFT]   = rin_b;
    tg->state.p[0].stage[AREA_RIGHT]  = kotori;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);
    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);
    drain_pick0(tg, 12);

    CHECK_EQ(stage_count(tg, 0), 3, "a5: three members really are on stage");
    CHECK_EQ(score_of(tg, sunny), 0,
             "a5: 3 members but not 3 DIFFERENT names -> no score bonus");
}

/* A6. sunny_branch2_two_mus_grants_heart -- exactly ONE of two mu's members
 * gains heart03, not both. The XOR is the point: an `a>=1 || b>=1` band would
 * pass a grant-to-both bug, which is exactly what this tier can produce. */
static void test_a6_sunny_two_mus_grants_heart_to_exactly_one(void)
{
    TestGame *tg = &tg_g;
    int sunny, m1, m2, m3;
    if (!sunny_three_board(tg, RIN, FILLER, GENERIC, &sunny, &m1, &m2, &m3)) return;
    /* Keep only TWO members: RIN at center, FILLER at left, right empty. */
    tg->state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    m3 = RB_EMPTY_SLOT;
    drain_pick0(tg, 12);

    int h1 = heart_of(tg, m1, idx03);
    int h2 = heart_of(tg, m2, idx03);
    int holders = (h1 == 1) + (h2 == 1);
    CHECK_EQ(holders, 1,
             "a6: 「『μ's』のメンバー1人は … heart03を得る」 -- EXACTLY ONE gains heart03 "
             "(m1=%d m2=%d)", h1, h2);
    CHECK((h1 == 1) ^ (h2 == 1),
          "a6: one has heart03 and the other has none, not a partial amount (m1=%d m2=%d)", h1, h2);
}

/* A7. sunny_branch2_targets_only_owners_mus_members -- heart03 lands on the
 * owner's mu's member and on nothing else: not the non-mu's member beside it,
 * not either opponent member. */
static void test_a7_sunny_heart_target_is_owner_mu_s_only(void)
{
    TestGame *tg = &tg_g2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int p1_mus = mid(tg, RIN);
    int p1_non = mid(tg, AQOURS_A);
    int p2_a = mid(tg, RIN);
    int p2_b = mid(tg, FILLER);
    pin_id(sunny, SUNNY);
    pin_id(p1_mus, RIN);
    if (sunny < 0 || p1_mus < 0 || p1_non < 0 || p2_a < 0 || p2_b < 0) return;
    CHECK(p1_mus != p2_a && p1_mus != p2_b && p2_a != p2_b,
          "a7: the four staged members are four DISTINCT instances");
    CHECK(rb_card_matches_group_str(p1_non, "Aqours"), "a7: setup -- the non-mu's member really is Aqours");
    CHECK(!rb_card_matches_group_str(p1_mus, "Aqours"), "a7: setup -- the target member is not an Aqours member");
    CHECK(rb_card_matches_group_str(p1_mus, MUS), "a7: setup -- the target member really is a mu's member");

    tg->state.p[0].stage[AREA_CENTER] = p1_mus;
    tg->state.p[0].stage[AREA_LEFT]   = p1_non;
    tg->state.p[1].stage[AREA_LEFT]   = p2_a;
    tg->state.p[1].stage[AREA_CENTER] = p2_b;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a7: the tier-1 hand-discard SelectCard is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "a7: the hand-discard prompt must be answered first");
    CHECK_EQ_STR(pending_type(tg), "SelectCard", "a7: it is a SelectCard, as the Rust twin asserts");
    pick(tg, 0);
    CHECK_PROMPTS(2, "a7: after the discard, the tier-2 heart target must ALSO be offered");
    CHECK(!pending(tg),
          "a7: only two prompts total (Rust asserts no prompt survives the second)");
    drain_pick0(tg, 8);

    CHECK_EQ(heart_of(tg, p1_mus, idx03), 1, "a7: the owner's mu's member gained heart03");
    CHECK_EQ(heart_of(tg, p1_non, idx03), 0, "a7: the owner's non-mu's member gained nothing");
    CHECK_EQ(heart_of(tg, p2_a, idx03), 0, "a7: the opponent's mu's member gained nothing");
    CHECK_EQ(heart_of(tg, p2_b, idx03), 0, "a7: the opponent's other member gained nothing");
}

/* A8. sunny_branch2_two_non_mus_no_heart -- the tier-2 CONDITION (count >= 2)
 * is met but there is no mu's member to target, so no heart target prompt and
 * no heart03 anywhere. The tier-1 prompt is what makes the negative readable. */
static void test_a8_sunny_two_non_mus_no_heart(void)
{
    TestGame *tg = &tg_h;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int aq_a = mid(tg, AQOURS_A), aq_b = mid(tg, AQOURS_B);
    pin_id(sunny, SUNNY);
    if (sunny < 0 || aq_a < 0 || aq_b < 0) return;
    CHECK(rb_card_matches_group_str(aq_a, "Aqours") &&
          rb_card_matches_group_str(aq_b, "Aqours"),
          "a8: both staged members really are Aqours, so no mu's target exists");

    tg->state.p[0].stage[AREA_CENTER] = aq_a;
    tg->state.p[0].stage[AREA_LEFT]   = aq_b;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a8: the tier-1 hand-discard SelectCard is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "a8: the hand-discard prompt must be answered before the negative claim");
    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "hand", "a8: the discard is over the HAND");
        CHECK_EQ(c->count, 1, "a8: exactly one card is discarded");
    }
    pick(tg, 0);

    CHECK(!pending(tg),
          "a8: no heart target choice appears when no mu's member exists (got %s)", pending_type(tg));
    drain_pick0(tg, 8);
    CHECK_EQ(heart_of(tg, aq_a, idx03), 0, "a8: the first Aqours member gained no heart03");
    CHECK_EQ(heart_of(tg, aq_b, idx03), 0, "a8: the second Aqours member gained no heart03");
}

/* A9. sunny_branch2_one_member_skips_b2 -- only ONE member, so tier 2's
 * (>= 2) condition is not met and no heart03 is granted. */
static void test_a9_sunny_one_member_skips_tier2(void)
{
    TestGame *tg = &tg_h2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int rin = mid(tg, RIN);
    pin_id(sunny, SUNNY);
    pin_id(rin, RIN);
    if (sunny < 0 || rin < 0) return;
    CHECK_EQ(stage_count(tg, 0), 0, "a9: setup -- the stage is empty before staging Rin");

    tg->state.p[0].stage[AREA_CENTER] = rin;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a9: the tier-1 hand-discard SelectCard is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "a9: the hand-discard prompt must be answered before the negative claim");
    pick(tg, 0);
    CHECK(!pending(tg), "a9: tier 2 does not trigger with only 1 member (got %s)", pending_type(tg));
    drain_pick0(tg, 8);
    CHECK_EQ(heart_of(tg, rin, idx03), 0, "a9: no heart03 is granted with only 1 member");
}

/* A10. sunny_q210_joint_card_counts_as_one_member -- the multi-name card is ONE
 * member, so tier 2 (>= 2) does not fire and it gains no heart03.
 * Note the fixture is a THIRD copy of 園田海未 while GENERIC is the plain
 * 上原/園田 print; both are mu's, so the "one member" reading is the only thing
 * under test. */
static void test_a10_sunny_joint_card_counts_as_one_member(void)
{
    TestGame *tg = &tg_i;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int joint = mid(tg, JOINT_BP3);
    pin_id(sunny, SUNNY);
    pin_id(joint, JOINT_BP3);
    if (sunny < 0 || joint < 0) return;
    CHECK(rb_card_is_member(joint), "a10: the multi-name fixture really is one member card");

    tg->state.p[0].stage[AREA_CENTER] = joint;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);
    drain_pick0(tg, 12);

    CHECK_EQ(stage_count(tg, 0), 1, "a10: 1 joint card == 1 member, not 3");
    CHECK_EQ(heart_of(tg, joint, idx03), 0,
             "a10: no heart03 with 1 joint member (tier 2 must not fire)");
}

/* A11. sunny_q211_joint_card_targetable_for_mus_heart -- the joint card plus
 * one other member is 2 members, so tier 2 fires and the joint card is a legal
 * mu's target. Rust walks a three-step chain: SelectAutoAbility ordering, the
 * tier-1 hand discard, then the tier-2 stage heart target. */
static void test_a11_sunny_joint_card_is_a_mu_s_heart_target(void)
{
    TestGame *tg = &tg_j;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int joint = mid(tg, JOINT_BP3);
    int other = mid(tg, GENERIC);
    pin_id(sunny, SUNNY);
    pin_id(joint, JOINT_BP3);
    if (sunny < 0 || joint < 0 || other < 0) return;
    CHECK(joint != other, "a11: the joint card and the other member are distinct instances");

    tg->state.p[0].stage[AREA_CENTER] = joint;
    tg->state.p[0].stage[AREA_LEFT]   = other;
    test_add_to_hand(tg, sunny);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);

    /* Step 1: Rust expects a SelectAutoAbility ordering prompt here because the
     * joint card has its OWN ライブ開始時. Whether the C engine raises it is
     * part of the finding, so it is asserted, not assumed. */
    PROMPT_PRESENT(tg, pending_is_auto(tg),
                   "a11 step1: a SelectAutoAbility ordering prompt is offered (got %s)",
                   pending_type(tg));
    if (pending_is_auto(tg)) pick(tg, 0);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a11 step2: the tier-1 hand-discard SelectCard is offered (got %s)",
                   pending_type(tg));
    CHECK_PROMPTS(2, "a11: at least the tier-1 discard and the tier-2 target must both be offered");
    if (pending_is_card(tg)) pick(tg, 0);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a11 step3: the tier-2 stage heart target is offered (got %s)", pending_type(tg));
    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "stage", "a11: the heart target is selected over the STAGE");
    }
    if (pending(tg)) pick(tg, 0);
    drain_pick0(tg, 8);

    int hj = heart_of(tg, joint, idx03);
    int ho = heart_of(tg, other, idx03);
    CHECK(hj >= 1 || ho >= 1,
          "a11: 「『μ's』のメンバー1人」 -- the joint card or the other member gains heart03 "
          "(joint=%d other=%d)", hj, ho);
}

/* A12. sunny_branch1_discard_is_not_restricted_to_mus_q210 -- the tier-1
 * hand-discard is UNFILTERED: the offered index set must cover BOTH the Aqours
 * hand card and the mu's hand card. The tier-1 prompt is the positive control
 * (it must be there at all) and the index set is the assertion. */
static void test_a12_sunny_tier1_discard_is_unfiltered(void)
{
    TestGame *tg = &tg_j2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int sunny = mid(tg, SUNNY);
    int aq = mid(tg, AQOURS_A);
    int mus = mid(tg, RIN);
    pin_id(sunny, SUNNY);
    if (sunny < 0 || aq < 0 || mus < 0) return;
    CHECK(rb_card_matches_group_str(aq, "Aqours"), "a12: the Aqours hand card really is Aqours");
    CHECK(!rb_card_matches_group_str(mus, "Aqours"),
          "a12: the other hand card is a different unit, so the two are distinguishable");
    CHECK(rb_card_matches_group_str(mus, MUS), "a12: the other hand card is a mu's member");

    tg->state.p[0].stage[AREA_CENTER] = mus;
    test_add_to_hand(tg, sunny);
    test_add_to_hand(tg, aq);
    test_add_to_hand(tg, mus);
    fill_decks(tg, 5);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, sunny);
    advance_to_live_start(tg);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "a12: the tier-1 hand-discard SelectCard is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "a12: the hand-discard prompt must be answered before the index-set claim");
    if (!pending_is_card(tg)) return;
    {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "hand", "a12: the discard is over the HAND (not a filtered sub-list)");
        int aq_i = hand_index_of(tg, aq);
        int mus_i = hand_index_of(tg, mus);
        CHECK(aq_i >= 0 && mus_i >= 0,
              "a12: both hand cards are still in hand at the prompt (aq=%d mus=%d)", aq_i, mus_i);
        if (c->n_filtered_indices == 0) {
            /* No filter at all == the whole hand is offered, which is what the
             * Rust twin asserts (an unfiltered hand discard). */
            CHECK(1, "a12: the prompt carries NO filter, so the whole hand is discardable");
        } else {
            int has_aq = 0, has_mus = 0;
            for (int i = 0; i < c->n_filtered_indices; i++) {
                if (c->filtered_indices[i] == aq_i) has_aq = 1;
                if (c->filtered_indices[i] == mus_i) has_mus = 1;
            }
            CHECK(has_aq, "a12: the Aqours hand card (index %d) is offered", aq_i);
            CHECK(has_mus, "a12: the mu's hand card (index %d) is offered", mus_i);
        }
    }
    pick(tg, 0);
    drain_pick0(tg, 8);
}

/* ========================================================================
 * GROUP B -- choice/per_card
 * tang_keke_energy_or_heart06_choice_test.rs
 *
 * PL!SP-pb2-002-R (唐 可可) 起動, use_limit 1.
 *   COST: discard 1 Liella! card from hand.
 *   EFFECT: choose 1 of
 *     ・自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
 *     ・ライブ終了時まで、自分のステージにいるこのメンバー以外の『Liella!』の
 *       メンバー1人は、heart06 を得る。
 *   If the discarded card is a Liella! MEMBER WITHOUT a blade heart, choose
 *   1+ (any_number) instead of exactly 1.
 * ======================================================================== */

static int keke_setup(TestGame *tg, int *keke_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_card_copy_pool_reset();
    int keke = mid(tg, KEKE);
    pin_id(keke, KEKE);
    int energy = mid(tg, ENERGY);
    if (keke < 0 || energy < 0) return -1;
    *keke_out = keke;
    tg->state.p[0].stage[AREA_LEFT] = keke;
    test_give_energy(tg, 5);
    for (int i = 0; i < 5; i++) test_add_to_energy_deck(tg, 0, energy);
    return keke;
}

/* B1. tang_keke_discard_member_with_blade_heart_choose_one -- discarding a
 * Liella! member that HAS a blade heart leaves count at exactly 1, so the
 * choice resolves after a single energy option with NO re-prompt. */
static void test_b1_keke_with_blade_heart_chooses_exactly_one(void)
{
    TestGame *tg = &tg_k;
    int keke;
    if (keke_setup(tg, &keke) < 0) return;
    int withbh = mid(tg, KEKE_BH);
    pin_id(withbh, KEKE_BH);
    if (withbh < 0) return;
    CHECK(rb_card_matches_group_str(withbh, "Liella!"), "b1: the discarded card really is a Liella! member");
    /* The Rust twin's premise: 「控え室に置いたカードがブレードハートを持たない
     * メンバーカードの場合」 -- this print HAS a blade heart, so count stays 1.
     * Card.blade is the card's blade COUNT (a top-level cards.json `blade`
     * field PL!SP-PR-003-PR does not even carry); the blade HEART is a separate
     * `blade_heart` entry, so rb_card_has_blade_heart() is the right reader and
     * reading Card.blade here would be a harness artefact, not a finding. */
    {
        Card c;
        int has_bh = 0, blade_count = -1;
        memset(&c, 0, sizeof c);
        if (rb_decode_card_by_index((uint32_t)withbh, &c)) {
            has_bh = rb_card_has_blade_heart(&c);
            blade_count = c.blade;
        }
        rb_free_card(&c);
        CHECK(has_bh,
              "b1 PREMISE: PL!SP-PR-003-PR really carries a blade heart, so the "
              "「ブレードハートを持たない」 branch must NOT be taken (Card.blade=%d is a "
              "different field and reads %d here)", blade_count, blade_count);
    }

    test_add_to_hand(tg, withbh);
    int energy_before = tg->state.p[0].energy.n;
    test_activate_ability(tg, keke);

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "b1: the Liella!-from-hand discard COST prompt is offered (got %s)",
                   pending_type(tg));
    CHECK_PROMPTS(1, "b1: the cost prompt must be answered before the energy claim");
    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "hand", "b1: the cost is paid from the HAND");
        CHECK_EQ(c->count, 1, "b1: exactly one Liella! card is discarded");
    }
    pick(tg, 0);

    PROMPT_PRESENT(tg, 1,
                   "b1: the two-option choice is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(2, "b1: BOTH the cost prompt and the choice must be answered");
    if (pending(tg)) select_option(tg, 0);
    drain_pick0(tg, 6);

    CHECK_EQ(tg->state.p[0].energy.n, energy_before + 1,
             "b1: 「自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く」 "
             "-- exactly one energy card was placed");
    CHECK(wait_has(tg, withbh), "b1: the discarded card is in the waitroom");
    CHECK(!hand_has(tg, withbh), "b1: the discarded card left the hand");
}

/* B2. tang_keke_discard_member_without_blade_heart_choose_any_number -- the
 * alternative branch swaps count=1 for any_number, so the engine must RE-PROMPT
 * for the remaining option. The energy delta must be exactly 1, not 2. */
static void test_b2_keke_without_blade_heart_chooses_any_number(void)
{
    TestGame *tg = &tg_k2;
    int keke;
    if (keke_setup(tg, &keke) < 0) return;
    int nobh = mid(tg, KEKE_NB);
    int target = mid(tg, KEKE_BH);
    pin_id(nobh, KEKE_NB);
    pin_id(target, KEKE_BH);
    if (nobh < 0 || target < 0) return;
    CHECK(rb_card_matches_group_str(nobh, "Liella!"), "b2: the discarded card is a Liella! member");
    CHECK(nobh != target && keke != target,
          "b2: the heart06 target is a DIFFERENT Liella! member from the one discarded");
    tg->state.p[0].stage[AREA_CENTER] = target;
    CHECK_EQ(stage_at(tg, 0, AREA_CENTER), target, "b2: the target member is on stage beside KEKE");

    test_add_to_hand(tg, nobh);
    int energy_before = tg->state.p[0].energy.n;

    test_activate_ability(tg, keke);
    PROMPT_PRESENT(tg, pending_is_card(tg), "b2: the discard cost prompt is offered (got %s)",
                   pending_type(tg));
    CHECK_PROMPTS(1, "b2: the cost prompt must be answered first");
    pick(tg, 0);

    PROMPT_PRESENT(tg, 1, "b2: the choice is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(2, "b2: the choice must be answered");
    if (pending(tg)) select_option(tg, 0);

    PROMPT_PRESENT(tg, 1,
                   "b2: any_number must RE-PROMPT for the remaining option (got %s)", pending_type(tg));
    CHECK_PROMPTS(3, "b2: the re-prompt is the whole point of the any_number branch");
    if (pending(tg)) select_option(tg, 0);
    drain_pick0(tg, 6);

    CHECK_EQ(tg->state.p[0].energy.n - energy_before, 1,
             "b2: exactly ONE energy was placed; a value > 1 here would pass if the "
             "engine placed it twice");
    CHECK(wait_has(tg, nobh), "b2: the discarded card is in the waitroom");
    CHECK(!pending(tg), "b2: no prompt survives after both options are taken");
}

/* B3. tang_keke_discard_non_member_liella_card_choose_one.
 *
 * RUST FIXTURE BUG, evidence-backed. The Rust twin (rs:117-138) writes
 *   let liella_live = game.id("PL!SP-pb2-002-R");
 * with the comment "PL!SP-pb2-002 itself is a Liella! live card -- use it as
 * discard fodder". cards/cards.json read as UTF-8 says otherwise:
 *   "PL!SP-pb2-002-R": { "type": "メンバー", "cost": 11, "name": "唐 可可" }
 * It is a MEMBER, not a live. So the twin's "card_type condition fails -> count=1"
 * premise never held; the discarded card was a member and took the any_number
 * branch instead. Both halves are run here:
 *   b3a  -- the Rust fixture, kept verbatim, with its broken premise asserted;
 *   b3b  -- a REAL Liella! live (PL!SP-pb2-045-L 絶対的LOVER, type=ライブ),
 *           which is what the test meant to exercise.
 */
static void test_b3_keke_non_member_liella_cost_choose_one(void)
{
    /* --- b3a: the Rust fixture, kept, premise pinned --- */
    {
        TestGame *tg = &tg_l;
        int keke;
        if (keke_setup(tg, &keke) >= 0) {
            int liella_live = mid(tg, KEKE);
            pin_id(liella_live, KEKE);
            if (liella_live >= 0) {
                CHECK(liella_live != keke, "b3a: the fodder is a second instance");
                CHECK(rb_card_is_member(liella_live) && !rb_card_is_live(liella_live),
                      "b3a FIXTURE BUG: cards.json types PL!SP-pb2-002-R as a MEMBER "
                      "(type=メンバー, cost 11), not the live the Rust comment claims");
                test_add_to_hand(tg, liella_live);
                int e0 = tg->state.p[0].energy.n;
                test_activate_ability(tg, keke);
                PROMPT_PRESENT(tg, pending_is_card(tg), "b3a: the discard cost prompt is offered (got %s)",
                               pending_type(tg));
                if (pending_is_card(tg)) pick(tg, 0);
                PROMPT_PRESENT(tg, 1, "b3a: the choice is offered (got %s)", pending_type(tg));
                if (pending(tg)) select_option(tg, 0);
                drain_pick0(tg, 6);
                CHECK_EQ(tg->state.p[0].energy.n, e0 + 1, "b3a: exactly one energy was placed");
                CHECK(wait_has(tg, liella_live), "b3a: the discarded card reached the waitroom");
            }
        }
    }
    /* --- b3b: a real Liella! LIVE, which is the case the twin meant --- */
    {
        TestGame *tg = &tg_l2;
        int keke;
        if (keke_setup(tg, &keke) >= 0) {
            int live = mid(tg, KEKE_LIVE);
            pin_id(live, KEKE_LIVE);
            if (live >= 0) {
                CHECK(rb_card_is_live(live) && !rb_card_is_member(live),
                      "b3b: the substitute really is a LIVE card, so the card_type "
                      "condition genuinely fails");
                CHECK(rb_card_matches_group_str(live, "Liella!"),
                      "b3b: the substitute really is a Liella! card, so the cost filter matches");
                test_add_to_hand(tg, live);
                int e0 = tg->state.p[0].energy.n;
                test_activate_ability(tg, keke);
                PROMPT_PRESENT(tg, pending_is_card(tg), "b3b: the discard cost prompt is offered (got %s)",
                               pending_type(tg));
                CHECK_PROMPTS(1, "b3b: the cost prompt must be answered first");
                if (pending_is_card(tg)) pick(tg, 0);
                PROMPT_PRESENT(tg, 1, "b3b: the choice is offered (got %s)", pending_type(tg));
                CHECK_PROMPTS(2, "b3b: the choice must be answered");
                if (pending(tg)) select_option(tg, 0);
                drain_pick0(tg, 6);
                CHECK_EQ(tg->state.p[0].energy.n, e0 + 1, "b3b: exactly one energy was placed");
                CHECK(wait_has(tg, live), "b3b: the discarded LIVE card reached the waitroom");
            }
        }
    }
}

/* B4. tang_keke_use_limit_blocks_second_activation. The Rust precondition --
 * "the first activation must have recorded its use, else the second refusal
 * proves nothing about the limit" -- is kept as a live control: the C twin is
 * rb_resolver_use_limit_reached(), queried after the first activation. */
static void test_b4_keke_use_limit_blocks_second_activation(void)
{
    TestGame *tg = &tg_m;
    int keke;
    if (keke_setup(tg, &keke) < 0) return;
    int withbh = mid(tg, KEKE_BH);
    if (withbh < 0) return;
    int copies[2] = { withbh, 0 };
    copies[1] = mid(tg, KEKE_BH);
    pin_id(copies[0], KEKE_BH);
    pin_id(copies[1], KEKE_BH);
    if (copies[0] < 0 || copies[1] < 0) return;
    CHECK(copies[0] != copies[1], "b4: two distinct copies for the two attempts");
    test_add_to_hand(tg, copies[0]);
    test_add_to_hand(tg, copies[1]);

    test_activate_ability(tg, keke);
    PROMPT_PRESENT(tg, pending_is_card(tg), "b4: the first attempt's cost prompt is offered (got %s)",
                   pending_type(tg));
    CHECK_PROMPTS(1, "b4: the first activation's cost must be answered");
    if (pending_is_card(tg)) pick(tg, 0);
    PROMPT_PRESENT(tg, 1, "b4: the first attempt's choice is offered (got %s)", pending_type(tg));
    if (pending(tg)) select_option(tg, 0);
    drain_pick0(tg, 6);

    /* The precondition: use_limit == 1 really is decoded. */
    int use_limit = -99;
    {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (rb_decode_card_ability((uint32_t)keke, 0, &ab)) use_limit = ab.use_limit;
        rb_free_ability(&ab);
    }
    CHECK_EQ(use_limit, 1, "b4: the print really carries use_limit = 1");
    CHECK(rb_resolver_use_limit_reached(&tg->state, keke, 0, use_limit),
          "b4: PRECONDITION -- the first activation recorded its use, else the second "
          "refusal below would prove nothing about the limit");

    int energy_before = tg->state.p[0].energy.n;
    int hand_before = tg->state.p[0].hand.n;
    int ok = test_activate_ability(tg, keke);
    CHECK_EQ(ok, 0, "b4: ターン1回 -- the second activation in the same turn is refused");
    CHECK(!pending(tg), "b4: a use-limit refusal must not open the discard cost prompt");
    CHECK_EQ(tg->state.p[0].energy.n, energy_before,
             "b4: a refused activation must not place another energy card");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "b4: the refusal costs nothing");
    CHECK(hand_has(tg, copies[1]), "b4: the cost was paid only once; the second copy is in hand");
}

/* B5. tang_keke_no_liella_in_hand_cost_cannot_pay -- no Liella! card in hand at
 * all, so the cost cannot be paid: no prompt, no effect, hand stays empty. */
static void test_b5_keke_no_liella_in_hand_cost_unpayable(void)
{
    TestGame *tg = &tg_m2;
    int keke;
    if (keke_setup(tg, &keke) < 0) return;
    tg->state.p[0].hand.n = 0;
    int energy_before = tg->state.p[0].energy.n;

    test_activate_ability(tg, keke);
    CHECK(!pending(tg), "b5: an unpayable cost must not open a discard prompt (got %s)",
          pending_type(tg));
    CHECK_EQ(tg->state.p[0].energy.n, energy_before,
             "b5: no effect may resolve when the cost was not paid");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "b5: nothing to discard, the hand stays empty");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "b5: nothing reached the waitroom either");
}

/* B6. tang_keke_nonmatching_hand_refuses_cost_no_prompt -- a non-empty hand
 * holding only a NON-Liella! card must be refused by the filter-aware cost
 * validation (Rule 9.4.2.3 / Q56), not open a prompt and fizzle later.
 * PAIRED POSITIVE CONTROL: B1 proves the same prompt DOES appear when a
 * matching Liella! card is in hand. */
static void test_b6_keke_nonmatching_hand_refuses_cost(void)
{
    TestGame *tg = &tg_n;
    int keke;
    if (keke_setup(tg, &keke) < 0) return;
    int aqours = mid(tg, AQOURS_A);
    pin_id(aqours, AQOURS_A);
    if (aqours < 0) return;
    CHECK(!rb_card_matches_group_str(aqours, "Liella!"),
          "b6: the hand card really is NOT a Liella! card");
    test_add_to_hand(tg, aqours);
    int energy_before = tg->state.p[0].energy.n;

    test_activate_ability(tg, keke);
    CHECK(!pending(tg), "b6: an unpayable FILTERED cost must not open a discard prompt (got %s)",
          pending_type(tg));
    CHECK_EQ(tg->state.p[0].energy.n, energy_before, "b6: no effect resolves when the cost is unpaid");
    CHECK(hand_has(tg, aqours), "b6: the non-matching card stays in hand");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "b6: nothing may be discarded when the cost is refused");
}

/* ========================================================================
 * GROUP C -- choice/per_card
 * daydream_mermaid_live_success_energy_or_recover_choice_test.rs
 *
 * PL!N-bp4-030-L (Daydream Mermaid) ライブ成功時, choose 1 of
 *   ・自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
 *   ・自分の控え室からメンバーカードを1枚手札に加える。
 * (any_number instead, if the success zone already holds a Nijigasaki card.)
 * ======================================================================== */

/* Rust setup_daydream_energy_choice: H06 + H05 on stage, mermaid in hand,
 * 15 filler per deck, 5 energy in the energy deck. */
static int mermaid_setup(TestGame *tg, int *mermaid_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_card_copy_pool_reset();
    int mermaid = mid(tg, MERMAID);
    int h05 = mid(tg, H05);
    int h06 = mid(tg, H06);
    int energy = mid(tg, ENERGY);
    pin_id(mermaid, MERMAID);
    *mermaid_out = mermaid;
    if (mermaid < 0 || h05 < 0 || h06 < 0 || energy < 0) return -1;
    CHECK(card_prints_trigger(mermaid, RB_TSTR_LIVE_SUCCESS), "c: Daydream really prints ライブ成功時");
    CHECK(!card_prints_trigger(h06, RB_TSTR_LIVE_SUCCESS),
          "c CONTROL: the stage members do not open the ライブ成功時 window themselves");
    tg->state.p[0].stage[AREA_LEFT]   = h06;
    tg->state.p[0].stage[AREA_CENTER] = h05;
    test_add_to_hand(tg, mermaid);
    fill_decks(tg, 15);
    for (int i = 0; i < 5; i++) test_add_to_energy_deck(tg, 0, energy);
    advance_to_live_set(tg);
    test_set_live_card(tg, 0, mermaid);
    advance_to_live_start(tg);
    advance_to_live_success(tg);
    return mermaid;
}

/* C1. daydream_mermaid_choice_appears_and_selects_energy */
static void test_c1_mermaid_energy_option_places_one_card(void)
{
    TestGame *tg = &tg_n2;
    int mermaid;
    if (mermaid_setup(tg, &mermaid) < 0) return;
    PROMPT_PRESENT(tg, 1, "c1: the ライブ成功時 choice appears (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "c1: the choice must be answered before the energy claim");
    if (!pending(tg)) return;
    int energy_before = tg->state.p[0].energy.n;
    select_option(tg, 0);
    drain_pick0(tg, 6);
    CHECK_EQ(tg->state.p[0].energy.n, energy_before + 1,
             "c1: 「自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く」 "
             "-- exactly one energy card was placed");
}

/* C2. daydream_mermaid_choice_cannot_be_applied_twice_q191 -- the same choice
 * must not be re-offered after it resolved. */
static void test_c2_mermaid_choice_cannot_be_applied_twice(void)
{
    TestGame *tg = &tg_o;
    int mermaid;
    if (mermaid_setup(tg, &mermaid) < 0) return;
    PROMPT_PRESENT(tg, 1, "c2: the ライブ成功時 choice appears (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "c2: the choice must be answered once");
    if (!pending(tg)) return;
    select_option(tg, 0);
    drain_pick0(tg, 6);
    int energy_after = tg->state.p[0].energy.n;
    CHECK(!pending(tg), "c2 (Q191): the choice is NOT re-offered after it resolved (got %s)",
          pending_type(tg));
    CHECK_EQ(tg->state.p[0].energy.n, energy_after, "c2: the energy zone did not grow a second time");
}

/* C3. daydream_mermaid_choice_selects_recover -- 「自分の控え室から、メンバーカードを
 * 1枚手札に加える」. The waitroom is stocked so the prompt must be a real
 * member selection, and the hand must grow by EXACTLY ONE (a band would also
 * pass if every filtered member had been recovered). */
static void test_c3_mermaid_recover_option_adds_exactly_one_member(void)
{
    TestGame *tg = &tg_o2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_card_copy_pool_reset();
    int mermaid = mid(tg, MERMAID);
    int h05 = mid(tg, H05);
    int h06 = mid(tg, H06);
    pin_id(mermaid, MERMAID);
    if (mermaid < 0 || h05 < 0 || h06 < 0) return;
    tg->state.p[0].stage[AREA_LEFT]   = h06;
    tg->state.p[0].stage[AREA_CENTER] = h05;
    test_add_to_hand(tg, mermaid);
    fill_decks(tg, 15);
    /* Eight distinct members in the waitroom: the prompt filters to members,
     * so this makes the "exactly one" assertion non-vacuous. */
    int wait[8];
    for (int i = 0; i < 8; i++) {
        wait[i] = mid(tg, FILLER);
        if (wait[i] < 0) return;
        test_add_to_discard(tg, wait[i]);
    }
    CHECK_EQ(tg->state.p[0].discard.n, 8, "c3: setup -- eight members are in the waitroom");
    advance_to_live_set(tg);
    test_set_live_card(tg, 0, mermaid);
    advance_to_live_start(tg);
    advance_to_live_success(tg);

    PROMPT_PRESENT(tg, 1, "c3: the ライブ成功時 choice appears (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "c3: the two-option choice must be answered");
    int hand_before = tg->state.p[0].hand.n;
    if (pending(tg)) select_option(tg, 1);   /* the recover option */

    PROMPT_PRESENT(tg, pending_is_card(tg),
                   "c3: the member recovery card selection appears (got %s)", pending_type(tg));
    CHECK_PROMPTS(2, "c3: the recover option must actually raise its card selection");
    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "discard",
                     "c3: the pick is over the WAITROOM (C spells the zone \"discard\")");
        CHECK_EQ(c->count, 1, "c3: 「メンバーカードを1枚手札に加える」 -- exactly one");
    }
    int wait_before_pick = tg->state.p[0].discard.n;
    pick(tg, 0);
    drain_pick0(tg, 6);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before + 1,
             "c3: exactly ONE member joined the hand; 'the hand grew' would also pass "
             "if every filtered member had been recovered");
    /* Relative, not absolute: the performance walk legitimately moves cards into
     * the waitroom, so only the DELTA across the pick is a claim about the
     * effect. Measured at the pick site, not at setup. */
    CHECK_EQ(wait_before_pick - tg->state.p[0].discard.n, 1,
             "c3: exactly one member left the waitroom across the pick (%d -> %d)",
             wait_before_pick, tg->state.p[0].discard.n);
}

/* ========================================================================
 * GROUP D -- choice/per_card
 * kanan_bp7_two_debut_abilities_queue_test.rs
 *
 * PL!S-bp7-003-P+ (松浦果南) has TWO 登場 abilities:
 *   ab#0: 登場/ライブ開始時 -- look at the top card of your deck, you may put
 *         it on the BOTTOM of your deck.
 *   ab#1: 登場 -- choose 1 of {Aqours cannot-wait-by-effect restriction /
 *         position change into an area with an Aqours or SaintSnow member}.
 * Both must be visible in the ability queue FROM THE START of the debut, not
 * just the first with the second appearing only after the first resolves.
 *
 * The C decode confirms both abilities exist and both print 登場:
 *   ab#0 triggers="ライブ開始時, 登場"
 *   ab#1 triggers="登場"
 * ab#1's choice carries group_names="Aqours,SaintSnow" -- the CSV-vs-single-token
 * defect a peer confirmed. That defect is NOT exercised here (this test is
 * about queue depth), but it is recorded in the final report.
 * ======================================================================== */
static void test_d_kanan_both_debut_abilities_queued_from_start(void)
{
    TestGame *tg = &tg_p;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_card_copy_pool_reset();
    int kanan = mid(tg, KANAN);
    pin_id(kanan, KANAN);
    if (kanan < 0) return;

    /* Fixture guard: BOTH abilities decode and BOTH print 登場. */
    int n_ab = rb_card_num_abilities((uint32_t)kanan);
    CHECK_EQ(n_ab, 2, "d: the print decodes exactly TWO abilities");
    int both_debut = 0;
    for (int i = 0; i < n_ab; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)kanan, i, &ab)) continue;
        if (ab.triggers && strstr(ab.triggers, RB_TSTR_DEBUT)) both_debut++;
        rb_free_ability(&ab);
    }
    CHECK_EQ(both_debut, 2, "d: BOTH abilities print 登場, so the debut must queue two entries");

    /* --- measure at the C's own enqueue point --- */
    /* test_play_to_stage drives the whole debut to completion, so the queue is
     * empty by the time it returns. The claim under test is about the DEPTH AT
     * ENQUEUE TIME, so the 登場 scan is invoked directly and the queue is read
     * before anything resolves. */
    fill_decks(tg, 10);
    tg->state.p[0].stage[AREA_CENTER] = kanan;
    tg->state.activating_card = kanan;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, RB_TSTR_DEBUT);
    int total = tg->state.queue.n_entries;
    int pending_n = rb_queue_has_pending(&tg->state.queue) ? 1 : 0;

    fprintf(stderr,
            "  [QUEUE SHAPE] d: 登場 scan queued=%d, queue depth=%d, has_pending=%d\n",
            queued, total, pending_n);
    CHECK_EQ(queued, 2, "d: the 登場 scan enqueues BOTH debut abilities at once (got %d)", queued);
    CHECK_EQ(total, 2, "d: BOTH debut abilities are in the queue FROM THE START (got %d)", total);
    CHECK_EQ(pending_n, 1, "d: the queue is PENDING on both entries from the start");

    /* The depth assertion alone would pass on a queue holding two copies of
     * ONE ability, so pin the identity of each entry: two DISTINCT ability
     * indices of the same card. */
    int idx0 = total > 0 ? tg->state.queue.entries[0].ability_idx : -1;
    int idx1 = total > 1 ? tg->state.queue.entries[1].ability_idx : -1;
    CHECK_EQ(tg->state.queue.entries[0].card_id, kanan, "d: entry 0 belongs to 果南");
    CHECK_EQ(tg->state.queue.entries[1].card_id, kanan, "d: entry 1 belongs to 果南");
    CHECK(idx0 != idx1,
          "d: the two entries are TWO DIFFERENT abilities of the same card (%d and %d)", idx0, idx1);

    /* --- the same claim through a REAL debut, for the record --- */
    /* play_to_stage drives the debut to completion, so nothing is left in the
     * queue; the post-play depth is printed so a red "0 queue entries" reading
     * from the play path is not mistaken for the depth-at-enqueue measurement
     * above. */
    {
        TestGame *tg2 = &tg_p2;
        test_game_new(tg2);
        clear_p1(tg2);
        clear_p2(tg2);
        int kanan2 = mid(tg2, KANAN);
        pin_id(kanan2, KANAN);
        if (kanan2 < 0) return;
        fill_decks(tg2, 10);
        test_add_to_hand(tg2, kanan2);
        test_give_energy(tg2, 20);
        int played = play_to_stage(tg2, kanan2, AREA_CENTER);
        CHECK_EQ(played, 1, "d2: 果南 debuts to center for real");
        CHECK_EQ(stage_at(tg2, 0, AREA_CENTER), kanan2, "d2: she is on stage center");
        fprintf(stderr,
                "  [QUEUE SHAPE] d2: after a real play_to_stage the queue depth is %d "
                "(the debut resolved inside the shim, so this is NOT the depth-at-enqueue "
                "measurement of d above)\n",
                test_queue_n_entries(tg2));
    }
}

/* ========================================================================
 * GROUP E -- choice/per_card
 * conditional_choice_options_execution_test.rs, the Bouken half.
 *
 * PL!S-bp6-020-L (冒険Type A, B, C!!) ライブ開始時, choose 1 of THREE:
 *   ・このカードは「ライブ成功時カードを1枚引く。」を得る。
 *   ・ライブ終了時まで、このターンにバトンタッチして登場した『Aqours』の
 *     メンバー1人は heart02 を得る。
 *   ・自分の成功ライブカード置き場にカードが2枚以上ある場合、このカードの
 *     スコアを＋１する。
 * (dia_choose_blade and the two kotori tests of the same Rust file are
 *  already covered: the first by test_parity_target_player.c GROUP F, the
 *  others by test_parity_conditional_gated.c group H. The three
 *  dia position_change tests need game.generated_actions() /
 *  select_generated(), which rb_generate_action_candidates() does not emit --
 *  they are NOT taken here and are named in the final report.)
 * ======================================================================== */

/* Bouken is handed in, NOT pre-placed: the C engine auto-places p1's live card
 * at the LiveCardSet phase (measured -- LIVE_ZONE_REFILL), so pre-placing it
 * skips the very step the Rust twin walks through. The same shape as the SUNNY
 * board, which reaches ライブ開始時. */
static int bouken_setup(TestGame *tg, int *bouken_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_card_copy_pool_reset();
    int bouken = mid(tg, BOUKEN);
    int filler = test_new_id(tg, FILLER);
    /* The STAGE, not the energy deck, is the supply: adding energy cards left
     * filled=[0,0,1,6,2,1,0,0] unchanged (measured). Bouken's printed need is
     * [4,0,2,0,2,2,0,0] = 12, two members supply 10, so EARLY_SEAT computed
     * surplus=-1 and p1 LOST -- the ライブ成功時 window never opened. A third
     * member closes the gap, which is what the mermaid board gets for free from
     * its cheaper 8-heart need. */
    int stage_h06 = mid(tg, H06);
    int stage_h05 = mid(tg, H05);
    int stage_filler = test_new_id(tg, FILLER);
    int energy = mid(tg, ENERGY);
    pin_id(bouken, BOUKEN);
    *bouken_out = bouken;
    if (bouken < 0 || filler < 0 || stage_h06 < 0 || stage_h05 < 0 ||
        stage_filler < 0 || energy < 0) return -1;
    CHECK(!card_prints_trigger(stage_filler, RB_TSTR_LIVE_START),
          "e CONTROL: the third member does not open the LiveStart window either");
    CHECK(card_prints_trigger(bouken, RB_TSTR_LIVE_START), "e: Bouken really prints ライブ開始時");
    CHECK(!card_prints_trigger(stage_h06, RB_TSTR_LIVE_START) &&
          !card_prints_trigger(stage_h05, RB_TSTR_LIVE_START) &&
          !card_prints_trigger(filler, RB_TSTR_LIVE_START),
          "e CONTROL: the staged members do not open the LiveStart window themselves");
    /* The SAME two stage members the mermaid board uses. With any other pair p1
     * collects no yell hearts, loses the live, and the ライブ成功時 dispatch in
     * e2 is vacuous -- that was measured, not assumed. */
    tg->state.p[0].stage[AREA_LEFT]   = stage_h06;
    tg->state.p[0].stage[AREA_CENTER] = stage_h05;
    tg->state.p[0].stage[AREA_RIGHT]  = stage_filler;
    /* Bouken's printed need is [4,0,2,0,2,2,0,0] -- it has NO wildcard column,
     * so leftover hearts cannot absorb the shortfall the way they do for
     * mermaid's [4,0,0,0,0,1,3,0]. Two more members still left colour2 and
     * colour5 one short (EARLY_SEAT surplus=-1, p1 loses, the ライブ成功時
     * window never opens), so the two short columns are topped up on the
     * third member through the engine's own modifier path. */
    rb_mods_add_heart_with_trace(&tg->state.mods, stage_filler, RB_HEART_YELLOW, 4,
                                 stage_filler, "e2 supply: cover need[2]");
    rb_mods_add_heart_with_trace(&tg->state.mods, stage_filler, RB_HEART_PURPLE, 3,
                                 stage_filler, "e2 supply: cover need[5]");
    test_add_to_hand(tg, bouken);
    fill_decks(tg, 15);
    for (int i = 0; i < 5; i++) test_add_to_energy_deck(tg, 0, energy);
    return bouken;
}

/* E1. bouken_three_options -- the Rust twin reads get_pending_choice_json()
 * and asserts opts.len() == 3. The flat RbChoice carries NO option list, so
 * that half is an EXPECTED_GAP, never a pass. The PARSE half is checked for
 * real: the decoded choice must really carry three options. */
static void test_e1_bouken_three_options(void)
{
    TestGame *tg = &tg_p2;
    int bouken;
    if (bouken_setup(tg, &bouken) < 0) return;

    /* Parse layer: three options really are decoded. */
    {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        int n_opt = -1;
        if (rb_decode_card_ability((uint32_t)bouken, 0, &ab)) {
            if (ab.effect) n_opt = ab.effect->n_options;
        }
        rb_free_ability(&ab);
        CHECK_EQ(n_opt, 3, "e1: the decoded ライブ開始時 choice really carries 3 options");
    }

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, bouken);
    {
        SeenChoice sc;
        memset(&sc, 0, sizeof sc);
        walk_passes(tg, 2, &sc);
        CHECK(sc.seen && !strcmp(sc.type, "SelectTarget"),
              "e1: the three-option LiveStart choice is offered (prompts seen=%d, "
              "last type \"%s\")", sc.n_prompts, sc.type);
        CHECK_PROMPTS(sc.seen ? 1 : 0,
                      "e1: the choice must be answered, not skipped");
    }
    {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int has_list = (c != NULL);
        EXPECTED_GAP(has_list && c->n_heart_options == 3,
            "e1: the flat RbChoice has NO option list at all (only n_heart_options, for "
            "heart-pick prompts), so game.get_pending_choice_json().options.len() == 3 "
            "has no faithful C entry point -- the parse-layer count above is the "
            "strongest available check");
    }
    drain_pick0(tg, 6);
    CHECK(!pending(tg), "e1: no prompt survives option 0");
    CHECK(rb_card_num_gained_abilities(&tg->state, bouken) >= 1,
          "e1: option 0 registered the runtime ability on the live card");
}

/* E2. bouken_gain_ability_draws_on_live_success -- option 0 registers a
 * RUNTIME ability on the live card, and that ability draws 1 on ライブ成功時.
 * The heart injection is the PRECONDITION that opens the window: without it the
 * dispatch below is vacuous, and the Rust twin asserts exactly that. */
static void test_e2_bouken_gained_ability_draws_on_live_success(void)
{
    TestGame *tg = &tg_q;
    int bouken;
    if (bouken_setup(tg, &bouken) < 0) return;

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, bouken);
    {
        SeenChoice sc;
        memset(&sc, 0, sizeof sc);
        walk_passes(tg, 2, &sc);
        CHECK(sc.seen && !strcmp(sc.type, "SelectTarget"),
              "e2: the three-option choice is offered (prompts seen=%d, last type \"%s\")",
              sc.n_prompts, sc.type);
        CHECK_PROMPTS(sc.seen ? 1 : 0, "e2: the choice must be answered");
    }
    drain_pick0(tg, 6);

    int n_gained = rb_card_num_gained_abilities(&tg->state, bouken);
    CHECK(n_gained >= 1, "e2: the gained ability is stored on the live card (got %d)", n_gained);
    if (n_gained >= 1) {
        const Ability *g = rb_card_gained_ability(&tg->state, bouken, 0);
        CHECK(g && g->triggers && strstr(g->triggers, RB_TSTR_LIVE_SUCCESS),
              "e2: the gained ability's trigger is ライブ成功時 (got \"%s\")",
              (g && g->triggers) ? g->triggers : "(null)");
    }
    int hand_at_live_start = tg->state.p[0].hand.n;

    /* PRECONDITION: the window is opened by the ENGINE's own victory
     * determination, not by a hand-injected stage_hearts array. The Rust twin
     * injects `BaseHeart{Heart00: 20}` and asserts should_trigger_live_success;
     * in the C port rb_should_trigger_live_success() does NOT open on
     * stage_hearts alone (measured: it returns 0 with 20 wildcard hearts set),
     * so the real walk is used and the OPEN window is read off the state the
     * engine itself produced. A closed window here would make the draw
     * assertion vacuous, which is why it is asserted. */
    advance_to_live_success(tg);
    /* PRECONDITION, on the engine's OWN signal and not on a prompt: the gained
     * ability is a plain draw, so the dispatch resolves with nothing left on
     * screen and `pending()` would be a false premise. p1's live must have
     * actually succeeded, which is what the engine records in live_success[]. */
    CHECK(tg->state.live_success[0] != 0,
          "e2 PRECONDITION: p1's live SUCCEEDED, so the ライブ成功時 dispatch below is "
          "not vacuous (phase=%d success=%d)", tg->state.phase, tg->state.live_success[0]);
    drain_pick0(tg, 6);

    CHECK_EQ(tg->state.p[0].hand.n, hand_at_live_start + 1,
             "e2: 「ライブ成功時カードを1枚引く。」 -- the GAINED ability drew exactly 1 "
             "(hand %d -> %d)", hand_at_live_start, tg->state.p[0].hand.n);
    CHECK(rb_card_num_gained_abilities(&tg->state, bouken) >= 1,
          "e2: the gained ability persists after the LiveSuccess dispatch");
}

/* E3. bouken_gain_ability_no_draw_on_failed_live -- the OPPOSITE premise, and
 * deliberately so: with no hearts the window is SHUT, and asserting it shut is
 * what makes the negative meaningful. */
static void test_e3_bouken_no_draw_on_failed_live(void)
{
    TestGame *tg = &tg_r;
    int bouken;
    if (bouken_setup(tg, &bouken) < 0) return;

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, bouken);
    {
        SeenChoice sc;
        memset(&sc, 0, sizeof sc);
        walk_passes(tg, 2, &sc);
        CHECK(sc.seen && !strcmp(sc.type, "SelectTarget"),
              "e3: the three-option choice is offered (prompts seen=%d, last type \"%s\")",
              sc.n_prompts, sc.type);
        CHECK_PROMPTS(sc.seen ? 1 : 0, "e3: the choice must be answered");
    }
    drain_pick0(tg, 6);
    CHECK(rb_card_num_gained_abilities(&tg->state, bouken) >= 1,
          "e3: the gained ability is stored regardless of the outcome");
    int hand_at_live_start = tg->state.p[0].hand.n;

    /* PRECONDITION: no hearts on either stage, so the ライブ成功時 window is
     * SHUT. This is the deliberate OPPOSITE of e2's premise, and it is what
     * makes the negative below mean anything -- an unstated premise is exactly
     * what lets a closed window masquerade as the requirement working. */
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 0;
    for (int i = 0; i < 8; i++) tg->state.stage_hearts[0][i] = 0;
    for (int i = 0; i < 8; i++) tg->state.stage_hearts[1][i] = 0;
    CHECK(!rb_should_trigger_live_success(&tg->state, 0),
          "e3 PRECONDITION: with no hearts the ライブ成功時 window must be SHUT, "
          "which is what this test's negative rests on");

    int hand_before = tg->state.p[0].hand.n;
    tg->state.activating_card = bouken;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_pick0(tg, 6);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "e3: no draw on a failed live");
    CHECK_EQ(hand_before, hand_at_live_start, "e3: p1's hand is back to where the window found it");
}

/* ========================================================================
 *                         ability dumper (diagnostic)
 * ======================================================================== */
static void dump_effect(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("[%d] action=%s src=%s dst=%s target=%s count=%d opt=%d",
           depth, e->action ? e->action : "-", e->source ? e->source : "-",
           e->destination ? e->destination : "-", e->target ? e->target : "-",
           e->count, e->is_optional);
    for (int i = 0; i < e->n_extra; i++)
        printf(" [%s=%s]", e->extra_k[i] ? e->extra_k[i] : "?",
               e->extra_v[i] ? e->extra_v[i] : "?");
    printf("\n");
    dump_effect(e->primary_effect, depth + 1);
    dump_effect(e->alternative_effect, depth + 1);
    dump_effect(e->followup_action, depth + 1);
    dump_effect(e->optional_action, depth + 1);
    dump_effect(e->conditional_action, depth + 1);
    dump_effect(e->look_action, depth + 1);
    dump_effect(e->select_action, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_effect(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) dump_effect(e->options[i], depth + 1);
}

static int run_dump(const char *list)
{
    char buf[512];
    snprintf(buf, sizeof buf, "%s", list);
    for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ",")) {
        int cid = rb_find_card_by_no(tok);
        printf("== %s -> id=%d resolved=%s cost=%d\n", tok, cid, card_no_of(cid),
               printed_cost(cid));
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int i = 0; i < n; i++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
            printf("-- ability %d triggers=%s use_limit=%d text=%s\n", i,
                   ab.triggers ? ab.triggers : "-", ab.use_limit,
                   ab.triggerless_text ? ab.triggerless_text : "-");
            if (ab.cost) { printf("  COST:\n"); dump_effect(ab.cost, 1); }
            if (ab.effect) { printf("  EFFECT:\n"); dump_effect(ab.effect, 1); }
            rb_free_ability(&ab);
        }
    }
    return 0;
}

/* Each case runs in a FORKED child with a SIGSEGV/SIGBUS/SIGABRT handler
 * naming the test in flight. No watchdog is achievable on this toolchain, so
 * every drain inside a case is explicitly capped. */
struct Case { const char *name; void (*fn)(void); };

static const struct Case cases[] = {
    { "a1_sunny_one_member_draws",   test_a1_sunny_one_member_draws_for_both },
    { "a2_sunny_no_members",         test_a2_sunny_no_members_is_a_noop },
    { "a3_sunny_beats_control",      test_a3_sunny_one_member_beats_control },
    { "a4_sunny_score_plus_1",       test_a4_sunny_three_distinct_names_score_plus_1 },
    { "a5_sunny_duplicate_name",     test_a5_sunny_duplicate_name_no_score },
    { "a6_sunny_two_mus_heart",       test_a6_sunny_two_mus_grants_heart_to_exactly_one },
    { "a7_sunny_owner_mu_s_only",     test_a7_sunny_heart_target_is_owner_mu_s_only },
    { "a8_sunny_two_non_mu_s",       test_a8_sunny_two_non_mus_no_heart },
    { "a9_sunny_one_member_skips",   test_a9_sunny_one_member_skips_tier2 },
    { "a10_sunny_joint_one_member",  test_a10_sunny_joint_card_counts_as_one_member },
    { "a11_sunny_joint_target",      test_a11_sunny_joint_card_is_a_mu_s_heart_target },
    { "a12_sunny_tier1_unfiltered",  test_a12_sunny_tier1_discard_is_unfiltered },
    { "b1_keke_with_blade_heart",    test_b1_keke_with_blade_heart_chooses_exactly_one },
    { "b2_keke_any_number",          test_b2_keke_without_blade_heart_chooses_any_number },
    { "b3_keke_non_member",          test_b3_keke_non_member_liella_cost_choose_one },
    { "b4_keke_use_limit",           test_b4_keke_use_limit_blocks_second_activation },
    { "b5_keke_no_liella",           test_b5_keke_no_liella_in_hand_cost_unpayable },
    { "b6_keke_nonmatching_hand",    test_b6_keke_nonmatching_hand_refuses_cost },
    { "c1_mermaid_energy",           test_c1_mermaid_energy_option_places_one_card },
    { "c2_mermaid_not_twice",        test_c2_mermaid_choice_cannot_be_applied_twice },
    { "c3_mermaid_recover",          test_c3_mermaid_recover_option_adds_exactly_one_member },
    { "d_kanan_two_debut",           test_d_kanan_both_debut_abilities_queued_from_start },
    { "e1_bouken_three_options",     test_e1_bouken_three_options },
    { "e2_bouken_gained_draws",      test_e2_bouken_gained_ability_draws_on_live_success },
    { "e3_bouken_no_draw",           test_e3_bouken_no_draw_on_failed_live },
};

#define N_CASES ((int)(sizeof cases / sizeof cases[0]))

static void run_all(const char *only)
{
    for (int i = 0; i < N_CASES; i++) {
        if (only && *only && !strstr(cases[i].name, only)) continue;
        printf("--- case %s\n", cases[i].name);
        fflush(stdout);
        current_test = cases[i].name;
        pid_t pid = fork();
        if (pid < 0) {
            fprintf(stderr, "fork() failed for %s -- running inline\n", cases[i].name);
            cases[i].fn();
            continue;
        }
        if (pid == 0) {
            signal(SIGSEGV, on_fatal_signal);
            signal(SIGBUS,  on_fatal_signal);
            signal(SIGABRT, on_fatal_signal);
            cases[i].fn();
            fflush(stdout);
            fflush(stderr);
            _Exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        if (WIFSIGNALED(status)) {
            int sig = WTERMSIG(status);
            if (sig == SIGSEGV || sig == SIGBUS || sig == SIGABRT) {
                assertions++;
                failures++;
                fprintf(stderr, "FAIL: case %s died on signal %d (the ENGINE, not this test)\n",
                        cases[i].name, sig);
            } else {
                fprintf(stderr, "case %s died on signal %d\n", cases[i].name, sig);
            }
        }
        current_test = "(none)";
    }
}

int main(void)
{
    C = (Counters *)mmap(NULL, sizeof(Counters), PROT_READ | PROT_WRITE,
                         MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (C == MAP_FAILED) { perror("mmap"); return 2; }
    memset(C, 0, sizeof *C);
    if (load_card_db() != 0) { fprintf(stderr, "load_cards failed\n"); return 1; }
    {
        const char *d = getenv("RB_DUMP");
        if (d && *d) return run_dump(d);
    }
    idx00 = (int)rb_parse_heart_color("heart00");
    idx01 = (int)rb_parse_heart_color("heart01");
    idx02 = (int)rb_parse_heart_color("heart02");
    idx03 = (int)rb_parse_heart_color("heart03");
    idx04 = (int)rb_parse_heart_color("heart04");
    idx05 = (int)rb_parse_heart_color("heart05");
    idx06 = (int)rb_parse_heart_color("heart06");
    printf("heart index map (from the engine, not the shim): "
           "heart00=%d heart01=%d heart02=%d heart03=%d heart04=%d heart05=%d heart06=%d\n",
           idx00, idx01, idx02, idx03, idx04, idx05, idx06);
    printf("rb_heart_color_as_str: RB_HEART_YELLOW(%d)=%s RB_HEART_PURPLE(%d)=%s "
           "RB_HEART_ORANGE(%d)=%s\n",
           RB_HEART_YELLOW, rb_heart_color_as_str(RB_HEART_YELLOW),
           RB_HEART_PURPLE, rb_heart_color_as_str(RB_HEART_PURPLE),
           RB_HEART_ORANGE, rb_heart_color_as_str(RB_HEART_ORANGE));
    run_all(getenv("RB_ONLY"));
    rb_unload();
    printf("cases=%d assertions=%d failures=%d setup_bugs=%d gaps=%d prompts=%d\n",
           N_CASES, assertions, failures, setup_bugs, gaps, prompts);
    if (failures) { printf("PARITY PER_CARD2: RED\n"); return 1; }
    printf("ALL PARITY PER_CARD2 CHECKS PASSED\n");
    return 0;
}
