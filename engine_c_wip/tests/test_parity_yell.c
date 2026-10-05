/* test_parity_yell.c — C port of the parts of engine/tests/test_modules/
 * jidou/yell/ that the existing C corpus does NOT yet carry.
 *
 * BOUNDARY ESTABLISHED FIRST (see the final report for the full grep):
 *   ALREADY COVERED elsewhere, NOT re-ported here:
 *     - no_blade_heart_reveal_heart02_test.rs            -> test_parity_jidou_extra.c J
 *     - no_blade_heart_reveal_gain_heart06_or_heart03_test.rs
 *     - no_blade_heart_reveal_gain_heart_edges_q112_test.rs
 *     - no_blade_heart_reveal_two_watchers_modifier_bounds_test.rs
 *     - wien_yell_no_blade_heart_gain_heart03_until_live_end_test.rs
 *     - no_blade_heart_reveal_heart06_member_as_live_q112_q113_test.rs
 *         -> test_parity_jidou_move_yell.c Y1/Y2, test_parity_integration.c O,
 *            test_parity_look_per_card.c O (real-pipeline heart06/heart03)
 *     - distinct_blade_heart_types_three_and_six_gain_heart01_test.rs
 *     - blade_heartless_reveals_threshold_grants_all_heart_test.rs
 *     - live_reveal_hand_limit_draw_test.rs
 *     - live_success_no_blade_heart_or_two_excess_hearts_scores_four_test.rs
 *     - 6 of 11 blade_heart_two_or_fewer_discard_all_and_re_yell_test.rs
 *     - 3 of 4 live_success_draw_per_aqours_member_..._test.rs
 *     - no_live_card_revealed_discard_all_and_re_yell_test.rs
 *     - yell_discard_liella_live_grants_two_extra_yells_test.rs
 *          -> ALL of the above in test_parity_jidou_move_yell.c (sections Y1..Y10)
 *
 *   PORTED HERE (the genuinely uncovered remainder):
 *     A. discard_revealed_then_re_yell/yell_revealed_no_blade_heart_discard_q251_test.rs
 *        (6) — MITSUKI_AUTO, PL!HS-bp6-027-L. The 自動 the database carries but
 *        test_parity_jidou_move_yell.c declared as #define and NEVER RAN.
 *        This is the `card_property=has_blade_heart` + negation filter cluster.
 *     B. discard_revealed_then_re_yell/zero_yell_optional_live_discard_q264_test.rs (1)
 *     C. discard_revealed_then_re_yell/dia_daisuki_both_activate_order_from_
 *        mixed_deck_test.rs (3) + one new discrimination case
 *     D. the 3 remaining distinct cases of
 *        blade_heart_two_or_fewer_discard_all_and_re_yell_test.rs (3)
 *     E. live_success_draw_per_aqours_member_...: jimo_ai_draw_3_discard_3_limited_hand
 *
 * C-API differences from Rust's TestGame that matter here (all confirmed
 * against include/test_game.h and the src tree):
 *   - `rb_load("src")` fails under an isolated out-of-tree build, so
 *     load_card_db() falls back to ../cards/build.
 *   - `test_get_heart_modifier` REMAPS colour 5 (heart06) onto RB_HEART_ORANGE;
 *     nothing here reads heart06, but heart03/heart02 go through rb_mods_get_heart.
 *   - Rust `game.id()` aliases the shared template, `game.new_id()` allocates the
 *     next DISTINCT pool slot. Rust's q251/yell fixtures call new_id for every
 *     card, so this file uses test_new_id (aliased below as mid()).
 *   - `rb_card_no_eq` is FUZZY and is not an identity pin; ident() below is a
 *     byte-exact strcmp of rb_card_string(c.card_no_idx).
 *   - rb_queue_trigger_abilities short-circuits a `location: revealed_cards`
 *     condition at QUEUE time against g->yell_occurred, so every fixture here
 *     sets yell_occurred = 1 (or 0, for the Q264 zero-yell case) explicitly.
 *   - C has no `state.re_yell_revealed_cards`; the re-yell observable is
 *     `state.re_yell_occurred`.
 *   - The flat RbChoice exposes no option LABELS, so "SelectAutoAbility option
 *     0 is DIA" is asserted indirectly, by the shape of the NEXT prompt.
 *   - sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *
 * Every case runs in a forked child; a fault is reported as CRASH, never as a
 * pass, and a SIGSEGV/SIGBUS/SIGABRT handler names the case in flight.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ====================================================================== */
/* 0. harness                                                             */
/* ====================================================================== */

static long assertions;
static int  failures;
static int  setup_bugs;

static const char *current_case = "(none)";
static int n_ok, n_failed, n_crashed;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_yell\n"
            "*** case in flight: %s  (engine fault, not a test bug)\n"
            "*** assertions so far: %ld  failures: %d\n",
            sig, current_case, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                       \
        assertions++;                                                   \
        if (!(condition)) {                                              \
            failures++;                                                  \
            fprintf(stderr, "FAIL: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                \
            fputc('\n', stderr);                                         \
        }                                                                \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                            \
        assertions++;                                                   \
        long a_ = (long)(actual);                                       \
        long e_ = (long)(expected);                                     \
        if (a_ != e_) {                                                  \
            failures++;                                                  \
            fprintf(stderr, "FAIL: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);        \
        }                                                                \
    } while (0)

#define CHILD_OK       0
#define CHILD_FAILURES 1
#define CHILD_CRASHED  2

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        current_case = "(none)";
        if ((long)failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld assertions]\n",
               (long)failures > f0 ? "FAILED" : "ok", name, assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        printf("        %s: %ld assertion(s), %ld failure(s)\n",
               name, assertions - a0, (long)failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit((long)failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
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
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ====================================================================== */
/* 0b. TRIGGER-MISS dump                                                  */
/*                                                                         */
/* rb_queue_yell_ability (src/core/game_state_abilities.c:546) returns 0     */
/* BEFORE it logs [YELL_QUEUE] whenever any of these is true:                */
/*   !rb_ability_matches_trigger(&ab, "自動")                                */
/*   !ab.effect || !ab.effect->condition                                     */
/*   !rb_condition_requires_yell(ab.effect->condition)                       */
/*   rb_effect_is_ability_resolution_watcher(ab.effect)                      */
/* so a missing [YELL_QUEUE] line is silent. This dump prints exactly what   */
/* each card actually carries, in hex, so a missing yell-trigger can be       */
/* attributed to one of those gates instead of guessed at.                   */
/*                                                                         */
/* Reference bytes: 登場場 E7 99 BB E5 A0 B4 / 登 E7 99 BB /                 */
/* 自動 E8 87 AA E5 8A A8 / 常時 E5 B8 B8 E6 99 82 /                          */
/* ライブ開始時 E3 83 A9 E3 82 A4 E3 83 96 E9 96 8B E5 A7 8B E6 99 82 E6 97 B6 E6 9C AF */
/* ====================================================================== */

static void dump_hex(const char *label, const char *s)
{
    fprintf(stderr, "[TRIGGER-MISS] %s = \"", label);
    if (!s) { fprintf(stderr, "(null)\n"); return; }
    for (const unsigned char *p = (const unsigned char *)s; *p; p++)
        fprintf(stderr, "%02X ", (unsigned)*p);
    fprintf(stderr, "\" utf8=\"%s\"\n", s);
}

static void dump_trigger_miss(const char *card_no)
{
    int cid = rb_find_card_by_no(card_no);
    if (cid < 0) { fprintf(stderr, "[TRIGGER-MISS] %s NOT IN cards.json\n", card_no); return; }
    int n = rb_card_num_abilities((uint32_t)cid);
    fprintf(stderr, "[TRIGGER-MISS] ==== %s (idx %d, %d abilities) ====\n",
            card_no, cid, n);
    for (int a = 0; a < n; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        fprintf(stderr, "[TRIGGER-MISS] ab#%d  matches_trigger(自動)=%d  "
                        "effect=%p cond=%p use_limit=%d is_null=%d has_keywords=%d\n",
                a, rb_ability_matches_trigger(&ab, "自動"),
                (void *)ab.effect, (void *)(ab.effect ? ab.effect->condition : NULL),
                ab.use_limit, ab.is_null, ab.has_keywords);
        dump_hex("triggers       ", ab.triggers);
        dump_hex("full_text      ", ab.full_text);
        rb_free_ability(&ab);
    }
}

/* ====================================================================== */
/* 1. card constants                                                      */
/*                                                                         */
/* Every card number below was verified against cards/cards.json read as  */
/* UTF-8 (Python json.load on the raw bytes). The properties the tests     */
/* turn on are quoted with the evidence.                                    */
/* ====================================================================== */

/* The group name the ability filters on, pinned to EXPLICIT BYTES so a future
 * CP932 re-encode of this file cannot silently turn it into a different
 * string.  蓮ノ空 = E8 8E B2 / E3 83 8E / E7 A9 BA.  core/card.c maps the
 * series ラブライブ！蓮ノ空女学院スクールアイドルクラブ to exactly this. */
#define HASU_GROUP     "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"
/* The lead escape was E8 8E B2 (U+8236), a variant 蓮 that occurs NOWHERE in
 * cards/cards.json or in engine/src/*.rs. src/core/card.c:374 and
 * engine/src/core/card.rs map_series_to_group both return U+84EE = E8 93 AE,
 * so rb_card_matches_group_str was comparing a token no card record carries. */

#define MITSUKI        "PL!HS-bp6-027-L"  /* 月夜見海月, unit スリーズブーケ
                                            * 自動 ターン1回: on yell, up to 3
                                            * revealed BLADE-HEARTLESS cards of the
                                            * Hasu group to the waitroom; yell again
                                            * once per card placed.  */
#define HASU_NO_BH     "PL!HS-bp1-010-N"  /* 日野下花帆 N, cost 4,
                                           *   blade_heart: null  -> qualifies   */
#define HASU_BH        "PL!HS-bp1-001-R"  /* 日野下花帆 R, cost 11,
                                           *   blade_heart: {b_heart04:1}        */
#define HASU_LIVE_S    "PL!HS-bp1-019-L"  /* Dream Believers,
                                           *   blade_heart: null,
                                           *   special_heart: {score:1}          */
#define NON_HASU       "PL!-sd1-010-SD"   /* 高坂穂乃果, series ラブライブ！ (not
                                           *   蓮ノ空) -> excluded by group      */
#define FILLER         "PL!-sd1-010-SD"
#define ENERGY         "LL-E-001-SD"
#define LIVE_FILLER    "PL!-sd1-020-SD"
#define REVEALED_LIVE  "PL!-bp3-026-L"
#define PERFORMED_LIVE "PL!-sd1-020-SD"   /* the card actually in the live zone */

#define NATSUMI        "PL!SP-pb2-020-R"  /* 鬼塚夏美 R: yell -> may place a
                                           *   『Liella!』 LIVE from hand, and if
                                           *   you do, yell 2 more               */
#define NATSUMI_N      "PL!SP-bp2-020-N"  /* the N print, a DIFFERENT card        */
#define LIELLA_LIVE    "PL!SP-sd1-023-SD"  /* WE WILL!!, special_heart {score:1}   */

#define DAISUKI        "PL!S-bp3-020-L"   /* ab#0: >=1 revealed and <=2 of them
                                           *   carry a blade heart -> place them
                                           *   all in the waitroom, re-yell     */
#define DIA            "PL!S-bp2-004-R"   /* ab#0: NO live card among the
                                           *   revealed -> place them all, re-yell */

#define JIMOAI         "PL!S-sd1-020-SD"  /* ab#0 ライブ成功時: draw 1 per
                                           *   『Aqours』 member, then discard that
                                           *   same count                        */
#define AQ_MEMBER      "PL!S-sd1-001-SD"  /* 高海千歌, ラブライブ！サンシャイン!! */

/* ====================================================================== */
/* 2. helpers                                                             */
/* ====================================================================== */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

/* BYTE-EXACT identity pin; rb_card_no_eq is fuzzy. */
static int card_no_of(int cid, char *buf, size_t n)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { if (n) buf[0] = '\0'; return 0; }
    snprintf(buf, n, "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    return 1;
}

static int ident(int cid, const char *no)
{
    char buf[128];
    if (cid < 0 || !card_no_of(cid, buf, sizeof buf)) {
        assertions++; setup_bugs++;
        fprintf(stderr, "SETUP BUG: card id %d could not be decoded (expected %s)\n",
                cid, no);
        return 0;
    }
    if (strcmp(buf, no) != 0) {
        assertions++; setup_bugs++;
        fprintf(stderr, "SETUP BUG: card id %d is \"%s\", the test names \"%s\"\n",
                cid, buf, no);
        return 0;
    }
    return 1;
}

#define mid(tg, no) test_new_id((tg), (no))

static int pending(TestGame *tg)
{
    return rb_has_pending_choice(&tg->state);
}

static int pending_kind(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return RB_CHOICE_NONE;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? (int)c->kind : RB_CHOICE_NONE;
}

static int pending_count(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return 0;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->count : 0;
}

static const char *pending_zone(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return "";
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->zone : "";
}

static int pending_n_filtered(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return -1;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->n_filtered_indices : -1;
}

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

/* Rust `while game.has_pending_choice() { game.select_indices(&[]) }` */
static void decline_all(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200)
        rb_resume_with_choice(&tg->state, -1);
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int waitroom_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].hand, cid); }
static int revealed_has(TestGame *tg, int cid) {
    for (int i = 0; i < tg->state.n_revealed; i++)
        if (tg->state.revealed_cards[i] == cid) return 1;
    return 0;
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` +
 * `state.process_pending_auto_abilities`. */
static void tas(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
}

static int re_yell_happened(TestGame *tg)
{
    return tg->state.re_yell_occurred != 0;
}

/* The yell window: rb_performance (src/turn/live.c:448) does the yell, queues
 * the yell-triggered 自動 and leaves the prompt standing, all inside ONE
 * test_pass. test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) therefore runs
 * the WHOLE live and the yell state is gone by the assertion site. Step one
 * pass at a time and stop the moment a prompt is standing or the revealed set
 * is non-empty; the pass that produced it is the yell window. */
static int step_to_yell_window(TestGame *tg)
{
    for (int i = 0; i < 24; i++) {
        if (rb_has_pending_choice(&tg->state)) return 1;
        if (tg->state.n_revealed > 0) return 1;
        if (tg->state.performance_resume_pending) return 1;
        test_pass(tg);
    }
    return rb_has_pending_choice(&tg->state) ||
           tg->state.n_revealed > 0 ||
           tg->state.performance_resume_pending;
}

/* Rust helpers::fill_decks in the per-file forms: 60 copies into BOTH decks. */
static void fill_decks_60(TestGame *tg)
{
    int filler = mid(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 60; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust `advance_to_phase(LiveCardSetFirstAttacker)` + `set_live_card`. */
static void set_live_at_set_phase(TestGame *tg, const char *live_no)
{
    int live = test_id(tg, live_no);
    test_add_to_hand(tg, live);
    test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    test_set_live_card(tg, 0, live);
}

/* ====================================================================== */
/* A. Q251 — 月夜見海月 PL!HS-bp6-027-L                                    */
/*                                                                         */
/* cards.json (UTF-8) for the host:                                       */
/*   "ability": "{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールした */
/*   とき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』の */
/*   カードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置 */
/*   いた数に等しい枚数のエールを追加で行う。"                              */
/*   faq[0]: "Q251 … {{jidou.png|自動}}で、【スコア】を持つカードを控え室に   */
/*   置くことはできますか？" answer: "いいえ。できません。"                  */
/*                                                                         */
/* The offer is a SelectCard over `location: revealed_cards` with           */
/* card_property=has_blade_heart + negation=true, PLUS a group filter on    */
/* the 蓮ノ空 series. Every negative below is paired with a POSITIVE board   */
/* built identically except for the single discriminating card, so no        */
/* assertion here can pass vacuously on a board where the ability never       */
/* fired.                                                                   */
/* ====================================================================== */

/* Rust `setup_with_revealed` + `trigger_mitsuki_auto`'s effect. Rust hands
 * the ability id straight to trigger_auto_ability; C has no such entry, so
 * the closest faithful equivalent is a real 自動 scan with the host in the
 * live card zone, which is exactly where Rust parks it. */
static void q251_setup(TestGame *tg, const int *rev, int n, int *out_mitsuki)
{
    *out_mitsuki = mid(tg, MITSUKI);
    ident(*out_mitsuki, MITSUKI);
    test_add_to_live(tg, *out_mitsuki);

    tg->state.n_revealed = 0;
    for (int i = 0; i < n && i < RB_MAX_REVEALED_CARDS; i++) {
        tg->state.revealed_cards[i] = rev[i];
        tg->state.n_revealed = i + 1;
    }
    /* HARNESS NOTE: rb_queue_trigger_abilities short-circuits a
     * `location: revealed_cards` condition at QUEUE time against
     * g->yell_occurred. Without this the ability is never queued and every
     * assertion below would be vacuously true. */
    tg->state.yell_occurred = n > 0;

    int filler = mid(tg, FILLER);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);

    /* Rust pins current_phase = Main. */
    tg->state.phase = RB_PHASE_MAIN;
}

static int q251_scan_offers(TestGame *tg, int *out_count, int *out_n_filtered)
{
    tas(tg, 0);
    int kind = pending_kind(tg);
    if (kind != RB_CHOICE_SELECT_CARD) {
        *out_count = 0;
        *out_n_filtered = -1;
        return 0;
    }
    *out_count = pending_count(tg);
    *out_n_filtered = pending_n_filtered(tg);
    return 1;
}

/* The exact card the NEXT SelectCard would move. For a single-card offer the
 * flat RbChoice exposes no id list, so this reads the filtered index list when
 * the choice publishes one and otherwise returns -1, which the caller treats
 * as "the offer has exactly one card, and the board holds only one candidate"
 * (true for every fixture that has a single revealed card). */
static int q251_offered_first(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (c && c->n_filtered_indices > 0)
        return c->filtered_indices[0];
    return -1;
}

/* Rust yell_revealed_no_blade_heart_member_moves_to_waitroom_q251 */
static void test_q251_blade_heartless_member_moves_to_waitroom(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    int member = mid(&game, HASU_NO_BH);
    CHECK(ident(member, HASU_NO_BH), "the revealed member is the PL!HS-bp1-010-N print");
    {
        Card c; memset(&c, 0, sizeof c);
        if (rb_decode_card_by_index((uint32_t)member, &c)) {
            CHECK_EQ(rb_card_has_blade_heart_strict(&c), 0,
                     "fixture: PL!HS-bp1-010-N has blade_heart: null (cards.json) "
                     "-> it is the one card the Q251 offer MUST include");
            rb_free_card(&c);
        }
    }
    int mitsuki = -1;
    int rev[1] = { member };
    q251_setup(&game, rev, 1, &mitsuki);

    int count = 0, nf = -1;
    CHECK(q251_scan_offers(&game, &count, &nf),
          "Q251: a blade-heartless 蓮ノ空 member in the yell IS offered");
    CHECK(count > 0, "VACUITY CONTROL: the SelectCard prompt is on screen here (count>0)");
    CHECK(strcmp(pending_zone(&game), "discard") == 0 ||
          pending_n_filtered(&game) > 0,
          "Q251: the offer really is a SelectCard over the REVEALED set "
          "(RbChoice.zone for that pool is the string \"discard\")");

    if (pending(&game)) {
        int offered = q251_offered_first(&game);
        CHECK(offered < 0 || offered == member,
              "the single-card offer is the revealed member itself");
        int idx = 1;
        test_select_indices(&game, &idx, 1);
        while (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY)
            rb_resume_with_choice(&game.state, -1);
    }
    CHECK(waitroom_has(&game, member),
          "Q251: the 蓮ノ空 member without a blade heart really moved to the waitroom");
    CHECK(!revealed_has(&game, member),
          "and it left the revealed set");
}

/* Rust yell_revealed_score_heart_live_stays_revealed_q251, paired with the
 * positive above so the negative is provably non-vacuous. */
static void test_q251_score_heart_live_stays_revealed(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    /* CONTROL BOARD: identical fixture, the blade-heartless member. */
    {
        static TestGame ctl;
        test_game_new(&ctl);
        rb_card_copy_pool_reset();
        int member = mid(&ctl, HASU_NO_BH);
        int mitsuki = -1;
        int rev[1] = { member };
        q251_setup(&ctl, rev, 1, &mitsuki);
        int count = 0, nf = -1;
        CHECK(q251_scan_offers(&ctl, &count, &nf),
              "VACUITY CONTROL: the same board DOES offer when the revealed card "
              "has no blade heart — so the negative below is about the filter, "
              "not about a dead scan");
    }

    int live = mid(&game, HASU_LIVE_S);
    CHECK(ident(live, HASU_LIVE_S), "the revealed live is the PL!HS-bp1-019-L print");
    {
        Card c; memset(&c, 0, sizeof c);
        if (rb_decode_card_by_index((uint32_t)live, &c)) {
            /* rb_card_has_blade_heart mirrors card.rs::has_blade_heart, which
             * returns TRUE for a card carrying any special_heart as well —
             * that is exactly the Q251 ruling ("【スコア】 cards are excluded"),
             * and Rust relies on it. The database fact is blade_heart: null,
             * which the _strict variant reports. */
            CHECK_EQ(rb_card_has_blade_heart_strict(&c), 0,
                     "fixture: PL!HS-bp1-019-L has blade_heart: null (cards.json)");
            CHECK_EQ(rb_card_has_score_icon(&c), 1,
                     "fixture: PL!HS-bp1-019-L has special_heart {score:1} (cards.json)");
            CHECK_EQ(rb_card_has_blade_heart(&c), 1,
                     "…and card.rs::has_blade_heart is true for it BECAUSE of that "
                     "special_heart, which is what excludes it from the offer");
            rb_free_card(&c);
        }
    }
    int mitsuki = -1;
    int rev[1] = { live };
    q251_setup(&game, rev, 1, &mitsuki);

    int count = 0, nf = -1;
    CHECK(!q251_scan_offers(&game, &count, &nf),
          "Q251: a [Score] live card is excluded by the has_blade_heart filter");
    CHECK_EQ(pending(&game), 0, "…and no prompt is left parked");
    CHECK(revealed_has(&game, live), "…it stays in the revealed set");
    CHECK(!waitroom_has(&game, live), "…and is NOT in the waitroom");
}

/* Rust yell_revealed_blade_heart_member_offers_no_discard_q251 */
static void test_q251_blade_heart_member_offers_no_discard(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    /* CONTROL BOARD: same fixture, the N print (no blade heart). */
    {
        static TestGame ctl;
        test_game_new(&ctl);
        rb_card_copy_pool_reset();
        int member = mid(&ctl, HASU_NO_BH);
        int mitsuki = -1;
        int rev[1] = { member };
        q251_setup(&ctl, rev, 1, &mitsuki);
        int count = 0, nf = -1;
        CHECK(q251_scan_offers(&ctl, &count, &nf),
              "VACUITY CONTROL: the same board DOES offer for PL!HS-bp1-010-N, so "
              "the negative below is about the blade-heart filter alone");
    }

    int member = mid(&game, HASU_BH);
    CHECK(ident(member, HASU_BH), "the revealed member is the PL!HS-bp1-001-R print");
    {
        Card c; memset(&c, 0, sizeof c);
        if (rb_decode_card_by_index((uint32_t)member, &c)) {
            CHECK_EQ(rb_card_has_blade_heart_strict(&c), 1,
                     "fixture: PL!HS-bp1-001-R carries blade_heart {b_heart04:1} "
                     "(cards.json) -> the negation must exclude it");
            rb_free_card(&c);
        }
    }
    int mitsuki = -1;
    int rev[1] = { member };
    q251_setup(&game, rev, 1, &mitsuki);

    int count = 0, nf = -1;
    CHECK(!q251_scan_offers(&game, &count, &nf),
          "Q251: a 蓮ノ空 member WITH a blade heart is excluded by the negation");
    CHECK_EQ(pending(&game), 0, "…no prompt is left parked");
    CHECK(!waitroom_has(&game, member), "…and nothing moved to the waitroom");
}

/* Rust yell_revealed_wrong_group_offers_no_discard_q251 */
static void test_q251_wrong_group_offers_no_discard(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    /* CONTROL BOARD: same fixture, a genuine 蓮ノ空 member. */
    {
        static TestGame ctl;
        test_game_new(&ctl);
        rb_card_copy_pool_reset();
        int member = mid(&ctl, HASU_NO_BH);
        int mitsuki = -1;
        int rev[1] = { member };
        q251_setup(&ctl, rev, 1, &mitsuki);
        int count = 0, nf = -1;
        CHECK(q251_scan_offers(&ctl, &count, &nf),
              "VACUITY CONTROL: the same board DOES offer for a 蓮ノ空 member, so "
              "the negative below is about the group filter alone");
    }

    int non = mid(&game, NON_HASU);
    CHECK(ident(non, NON_HASU), "the revealed card is the PL!-sd1-010-SD print");
    {
        const char *grp = rb_card_group_name(rb_find_card_by_no(NON_HASU));
        CHECK(grp && strcmp(grp, HASU_GROUP) != 0,
              "fixture: PL!-sd1-010-SD is NOT in the Hasu group (cards.json: "
              "series is the plain Love Live line, unit is mu's)");
    }
    int mitsuki = -1;
    int rev[1] = { non };
    q251_setup(&game, rev, 1, &mitsuki);

    int count = 0, nf = -1;
    CHECK(!q251_scan_offers(&game, &count, &nf),
          "Q251: a non-蓮ノ空 card is filtered out by the group condition");
    CHECK_EQ(pending(&game), 0, "…no prompt is left parked");
}

/* Rust yell_revealed_mixed_pool_moves_member_but_keeps_score_heart_live_q251 */
static void test_q251_mixed_pool_moves_member_keeps_score_heart_live(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    int member = mid(&game, HASU_NO_BH);
    int live   = mid(&game, HASU_LIVE_S);
    CHECK(ident(member, HASU_NO_BH), "the member is the PL!HS-bp1-010-N print");
    CHECK(ident(live, HASU_LIVE_S), "the live is the PL!HS-bp1-019-L print");
    int mitsuki = -1;
    int rev[2] = { member, live };
    q251_setup(&game, rev, 2, &mitsuki);

    int count = 0, nf = -1;
    CHECK(q251_scan_offers(&game, &count, &nf),
          "Q251: a mixed pool of 2 (1 qualifying, 1 [Score]) still offers");
    CHECK(count > 0, "VACUITY CONTROL: the prompt is on screen here (count>0)");

    if (pending(&game)) {
        int idx = 1;
        test_select_indices(&game, &idx, 1);
        while (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY)
            rb_resume_with_choice(&game.state, -1);
    }
    CHECK(waitroom_has(&game, member),
          "Q251: the qualifying 蓮ノ空 member moved to the waitroom");
    CHECK(!revealed_has(&game, member), "…and left the revealed set");
    CHECK(revealed_has(&game, live),
          "…while the [Score] live card stays revealed (Q251: 【スコア】 excluded)");
    CHECK(!waitroom_has(&game, live), "…and is not in the waitroom");
}

/* Rust yell_revealed_optional_discard_decline_keeps_member_revealed_q251 */
static void test_q251_optional_decline_keeps_member_revealed(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    int m[4];
    for (int i = 0; i < 4; i++) { m[i] = mid(&game, HASU_NO_BH); ident(m[i], HASU_NO_BH); }
    int mitsuki = -1;
    q251_setup(&game, m, 4, &mitsuki);

    int count = 0, nf = -1;
    CHECK(q251_scan_offers(&game, &count, &nf),
          "Q251: with 4 qualifying cards the 「3枚まで」 prompt appears");
    CHECK(count > 0, "VACUITY CONTROL: the prompt is on screen here (count>0)");

    /* Rust: game.select_indices(&[]) — decline, and 「3枚まで」 includes
     * declining, so the ability resolves without moving anything. */
    decline_all(&game);

    CHECK(revealed_has(&game, m[0]), "declined: the card stays in the revealed set");
    CHECK(!waitroom_has(&game, m[0]), "declined: it did NOT move to the waitroom");
    for (int i = 1; i < 4; i++)
        CHECK(!waitroom_has(&game, m[i]),
              "declined: no card from the pool moved");
}

/* ====================================================================== */
/* B. Q264 — a ZERO-card yell must not fire 鬼塚夏美 PL!SP-pb2-020-R         */
/*                                                                         */
/* 「自動 ターン1回 自分がエールしたとき、手札にある『Liella!』のライブ    */
/*  カードを1枚控え室に置いてもよい。そうした場合、追加で2枚エールを行う。」  */
/* ====================================================================== */

static void q264_board(TestGame *tg, int natsumi_wait, int *out_prompt, int *out_reveal)
{
    int natsumi = test_id(tg, NATSUMI);
    CHECK(ident(natsumi, NATSUMI), "the staged jidou is the PL!SP-pb2-020-R print");
    CHECK(ident(rb_find_card_by_no(NATSUMI_N), NATSUMI_N),
          "the N print PL!SP-bp2-020-N exists too and is a DIFFERENT card "
          "(cards.json: the R print is 鬼塚夏美; using the N print here would be "
          "the wrong card type)");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[1] = natsumi;
    if (natsumi_wait) rb_mods_set_orientation(&tg->state.mods, natsumi, "wait");

    int fill = mid(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, fill);
    test_give_energy(tg, 5);

    set_live_at_set_phase(tg, LIELLA_LIVE);
    /* Stop INSIDE the performance, at the yell, before the live resolves the
     * card to the success zone.
     *
     * A NON-zero yell always trips step_to_yell_window (n_revealed > 0 or a
     * standing prompt), which is the control board.
     *
     * A ZERO-card yell raises no prompt, leaves n_revealed at 0 and clears
     * yell_occurred before the pass returns, so it is stopped by the PHASE the
     * yell runs in. Reading the live's zone one pass later would find it
     * already in the success zone — correct behaviour, but it would make the
     * assertion meaningless. */
    int live = test_id(tg, LIELLA_LIVE);
    if (!natsumi_wait) {
        step_to_yell_window(tg);
    } else {
        for (int i = 0; i < 24; i++) {
            if (rb_has_pending_choice(&tg->state) ||
                tg->state.n_revealed > 0 || tg->state.performance_resume_pending)
                break;
            if (tg->state.phase == RB_PHASE_PERFORMANCE) break;
            if (!bag_has(&tg->state.p[0].live, live)) break;
            test_pass(tg);
        }
    }
    *out_prompt = pending(tg);
    *out_reveal = tg->state.n_revealed;
}

/* Rust zero_yell_q264_does_not_discard_live_for_additional_yells, with a
 * positive control board (natsumi ACTIVE, so the yell really reveals). */
static void test_q264_zero_yell_does_not_discard_live(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    /* VACUITY CONTROL: the same board with Natsumi ACTIVE really yells, so the
     * negative below is about the zero-card yell and not a dead fixture. */
    {
        static TestGame ctl;
        test_game_new(&ctl);
        rb_card_copy_pool_reset();
        int p = 0, r = 0;
        q264_board(&ctl, 0, &p, &r);
        CHECK(r > 0,
              "VACUITY CONTROL: with Natsumi ACTIVE the yell really reveals cards");
    }

    int live = test_id(&game, LIELLA_LIVE);
    CHECK(ident(live, LIELLA_LIVE), "the live card is the PL!SP-sd1-023-SD print");
    test_add_to_hand(&game, mid(&game, LIELLA_LIVE));
    int p = 0, r = 0;
    q264_board(&game, 1, &p, &r);

    CHECK_EQ(r, 0, "Q264 premise: Natsumi WAIT -> total blade 0 -> a ZERO-card yell");
    CHECK_EQ(game.state.yell_occurred, 0,
             "…and yell_occurred is false, so 「自分がエールしたとき」 is not met");
    CHECK(!waitroom_has(&game, live),
          "Q264: the 『Liella!』 live card must NOT have been discarded");
    CHECK(hand_has(&game, live) || bag_has(&game.state.p[0].live, live),
          "Q264: it should still be in hand or the live card zone");
}

/* ====================================================================== */
/* C. 黒澤ダイヤ + ダイスキだったらダイジョウブ！ — BOTH 自動 queue on one  */
/*    yell, and the player picks the order.                                 */
/*                                                                         */
/* The flat RbChoice publishes no option LABELS, so "option 0 is DIA" is     */
/* asserted by the SHAPE of the next prompt: DIA places ALL the revealed     */
/* cards, so its SelectCard count equals the reveal count.                   */
/* ====================================================================== */

/* Rust's `setup`: stage[2] = dia, stage[0..1] = fillers, DAISUKI to hand.
 * `mixed` selects Rust's mixed-deck shape (30 energy + 30 filler) instead of
 * the pure-filler deck, and `live_in_deck` forces a LIVE card into the yell
 * (which is DIA's blocking condition). */
static void order_scenario(TestGame *tg, int mixed, int live_in_deck,
                           int *out_prompt, int *out_reveal, int *out_deck)
{
    int dia     = test_id(tg, DIA);
    int daisuki = test_id(tg, DAISUKI);
    CHECK(ident(dia, DIA), "the staged 自動 host is the PL!S-bp2-004-R print");
    CHECK(ident(daisuki, DAISUKI), "the live card is the PL!S-bp3-020-L print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[0] = mid(tg, FILLER);
    tg->state.p[0].stage[1] = mid(tg, FILLER);
    tg->state.p[0].stage[2] = dia;
    test_add_to_hand(tg, daisuki);

    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    if (mixed) {
        int e = test_id(tg, ENERGY);
        int f = test_id(tg, FILLER);
        for (int i = 0; i < 30; i++) { test_add_to_deck_pl(tg, 0, e); test_add_to_deck_pl(tg, 1, e); }
        for (int i = 0; i < 30; i++) { test_add_to_deck_pl(tg, 0, f); test_add_to_deck_pl(tg, 1, f); }
        /* Front-load a LIVE so the yell reveals one -> DIA is blocked. */
        if (live_in_deck) {
            int lv = test_id(tg, REVEALED_LIVE);
            test_insert_deck_top(tg, 0, lv);
            test_insert_deck_top(tg, 0, lv);
        }
    } else {
        fill_decks_60(tg);
    }
    test_give_energy(tg, 15);

    set_live_at_set_phase(tg, DAISUKI);
    /* Into the yell window and STOP on arrival — the SelectAutoAbility under
     * test is raised there. */
    step_to_yell_window(tg);

    *out_prompt = pending(tg);
    *out_reveal = tg->state.n_revealed;
    *out_deck   = test_deck_len(tg);
}

/* Rust filler_deck_select_dia_first */
static void test_order_filler_deck_dia_first(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int p = 0, r = 0, d = 0;
    order_scenario(&game, 0, 0, &p, &r, &d);

    CHECK(r > 0, "the yell really revealed cards");
    CHECK(p, "a real yell revealed cards");
    CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_AUTO_ABILITY,
             "both 自動 queue on one yell -> SelectAutoAbility ordering prompt");
    if (p) answer(&game, 0);

    CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_CARD,
             "answering option 0 opens a card discard (DIA: 「それらのカードを"
             "すべて控え室に置いてもよい」)");
    int count = pending_count(&game);
    CHECK_EQ(count, r,
             "DIA places ALL the revealed cards, so its count == the reveal count");
    if (pending(&game)) {
        int idx[RB_MAX_ZONE];
        int n = count < RB_MAX_ZONE ? count : RB_MAX_ZONE;
        for (int i = 0; i < n; i++) idx[i] = i;
        test_select_indices(&game, idx, n);
    }
    while (pending(&game)) {
        if (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&game, 0);
        else answer(&game, -1);
    }
    CHECK(re_yell_happened(&game), "Q107: a re-yell happened");
}

/* Rust mixed_deck_dia_first_then_daisuki */
static void test_order_mixed_deck_dia_first_then_daisuki(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int p = 0, r = 0, d = 0;
    order_scenario(&game, 1, 0, &p, &r, &d);

    CHECK(r > 0, "the mixed deck still produces a real yell");
    CHECK(p, "a SelectAutoAbility ordering prompt is on screen");
    CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_AUTO_ABILITY,
             "…and it really is the 自動-ordering prompt");
    if (p) answer(&game, 0);

    int guard = 0;
    while (pending(&game) && guard++ < 64) {
        if (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&game, 0);
        else {
            int n = pending_count(&game);
            if (pending_kind(&game) == RB_CHOICE_SELECT_CARD && n > 0) {
                int idx[RB_MAX_ZONE];
                int m = n < RB_MAX_ZONE ? n : RB_MAX_ZONE;
                for (int i = 0; i < m; i++) idx[i] = i;
                test_select_indices(&game, idx, m);
            } else {
                answer(&game, -1);
            }
        }
    }
    CHECK(re_yell_happened(&game),
          "Q107: picking DIA first and then DAISUKI still produces a re-yell");
}

/* Rust mixed_deck_daisuki_first_then_dia */
static void test_order_mixed_deck_daisuki_first_then_dia(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int p = 0, r = 0, d = 0;
    order_scenario(&game, 1, 0, &p, &r, &d);

    CHECK(r > 0, "the mixed deck still produces a real yell");
    CHECK(p, "a SelectAutoAbility ordering prompt is on screen");
    CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_AUTO_ABILITY,
             "…and it really is the 自動-ordering prompt");
    if (p) answer(&game, 1);   /* the NON-default option: DAISUKI first */

    int guard = 0;
    while (pending(&game) && guard++ < 64) {
        if (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&game, 0);
        else {
            int n = pending_count(&game);
            if (pending_kind(&game) == RB_CHOICE_SELECT_CARD && n > 0) {
                int idx[RB_MAX_ZONE];
                int m = n < RB_MAX_ZONE ? n : RB_MAX_ZONE;
                for (int i = 0; i < m; i++) idx[i] = i;
                test_select_indices(&game, idx, m);
            } else {
                answer(&game, -1);
            }
        }
    }
    CHECK(re_yell_happened(&game),
          "Q156/Q107: picking DAISUKI first still resolves DIA afterwards and "
          "produces a re-yell");
}

/* NOT in the Rust file — the discrimination the ordering tests cannot show
 * through a label-free choice. ダイスキ's condition is "<=2 blade hearts in
 * the reveal", 黒澤ダイヤ's is "NO live card in the reveal".  When the yell
 * reveals a LIVE card, DIA is blocked and ONLY DAISUKI may be offered; a
 * second board without the live must offer the ordering prompt. */
static void test_order_revealed_live_blocks_dia_only(void)
{
    /* CONTROL: no live in the reveal -> DIA qualifies, ordering prompt. */
    {
        static TestGame ctl;
        test_game_new(&ctl);
        rb_card_copy_pool_reset();
        int p = 0, r = 0, d = 0;
        order_scenario(&ctl, 1, 0, &p, &r, &d);
        CHECK(p, "VACUITY CONTROL: with no live among the revealed, the ordering "
                 "prompt is on screen (so the next board's difference is the live)");
    }

    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int p = 0, r = 0, d = 0;
    order_scenario(&game, 1, 1, &p, &r, &d);

    CHECK(r > 0, "a real yell still happened");
    if (p) {
        /* With DIA blocked, at most one 自動 is offered. Answering it must
         * still open DAISUKI's discard (and never a "place all revealed" one
         * for DIA). */
        CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_AUTO_ABILITY,
                 "a live in the yell leaves only ダイスキ's 自動 to order");
        answer(&game, 0);
        CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_CARD,
                 "answering the single 自動 opens DAISUKI's 「それらのカードを"
                 "すべて控え室に置いてもよい」");
        int count = pending_count(&game);
        CHECK_EQ(count, r,
                 "DAISUKI also places all the revealed cards -> count == reveal count");
        int idx[RB_MAX_ZONE];
        int m = count < RB_MAX_ZONE ? count : RB_MAX_ZONE;
        for (int i = 0; i < m; i++) idx[i] = i;
        if (m > 0) test_select_indices(&game, idx, m);
    } else {
        /* Nothing offered at all would be a different bug; assert it loudly. */
        CHECK(p, "at least ダイスキ's 自動 must be offered");
    }
    int guard = 0;
    while (pending(&game) && guard++ < 64) answer(&game, 0);
    CHECK(re_yell_happened(&game),
          "Q156: DAISUKI alone still re-yells after placing the revealed cards");
}

/* ====================================================================== */
/* D. ダイスキだったらダイジョウブ！ PL!S-bp3-020-L — the three distinct    */
/*    cases of blade_heart_two_or_fewer_discard_all_and_re_yell_test.rs     */
/*    that test_parity_jidou_move_yell.c Y3 does not carry.                  */
/*    (condition_0_revealed_no_trigger and full_flow_accept_discard_and_     */
/*     re_yell ARE Y3's daisuki_zero_stage_no_reveal_no_trigger and         */
/*     daisuki_accept_discard_then_re_yells — skipped as near-duplicates.)   */
/* ====================================================================== */

static void daisuki_scenario(TestGame *tg, int n_members, int blades_each,
                              int *out_prompt, int *out_reveal, int *out_blade)
{
    int daisuki = test_id(tg, DAISUKI);
    CHECK(ident(daisuki, DAISUKI), "the live card is the PL!S-bp3-020-L print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    int e = test_id(tg, ENERGY);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 60; i++) { test_add_to_deck_pl(tg, 0, e); test_add_to_deck_pl(tg, 1, e); }
    test_give_energy(tg, 15);

    int total_blade = 0;
    for (int i = 0; i < n_members && i < 3; i++) {
        int m = mid(tg, FILLER);
        tg->state.p[0].stage[i] = m;
        if (blades_each) { rb_mods_add_blade(&tg->state.mods, m, blades_each); total_blade += blades_each; }
        else total_blade += 1;   /* PL!-sd1-010-SD prints b_heart03:1 */
    }

    set_live_at_set_phase(tg, DAISUKI);
    step_to_yell_window(tg);

    *out_prompt = pending(tg);
    *out_reveal = tg->state.n_revealed;
    *out_blade  = total_blade;
}

/* Rust condition_1_revealed_triggers — 「1枚以上公開したとき」 boundary. */
static void test_daisuki_one_revealed_triggers(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int p = 0, r = 0, b = 0;
    daisuki_scenario(&game, 1, 0, &p, &r, &b);
    CHECK_EQ(r, 1, "1 stage member -> the yell reveals exactly 1 card");
    CHECK(p, "「1枚以上公開したとき」: exactly 1 revealed still opens the discard");
}

/* Rust condition_5_revealed_all_blade_heart_no_trigger — the upper boundary.
 * Rust puts 5 blade hearts across 3 members; a 3-area stage cannot hold 5
 * members, so the blade total is the pinned quantity and the reveal count is
 * asserted only as a non-vacuity control. */
static void test_daisuki_five_blade_hearts_blocks(void)
{
    /* CONTROL: 2 blade hearts (2 <= 2) DOES prompt. */
    {
        static TestGame ctl;
        test_game_new(&ctl);
        rb_card_copy_pool_reset();
        int p = 0, r = 0, b = 0;
        daisuki_scenario(&ctl, 2, 0, &p, &r, &b);
        CHECK_EQ(b, 2, "control board: 2 blade hearts is inside the 「2枚以下」 gate");
        CHECK(p, "VACUITY CONTROL: with 2 blade hearts the discard prompt IS on "
                 "screen, so the 6-blade negative below is about the threshold");
    }

    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int p = 0, r = 0, b = 0;
    daisuki_scenario(&game, 3, 2, &p, &r, &b);
    CHECK_EQ(b, 6, "the boundary board carries 3 x 2 = 6 blade hearts (> 2)");
    CHECK(r > 0, "a real yell still happened (the negative must not be vacuous)");
    CHECK_EQ(p, 0, "「ブレードハートを持つカードが2枚以下の場合」: 6 > 2 blocks");
    CHECK_EQ(re_yell_happened(&game), 0, "…and no re-yell");
}

/* Rust once_per_turn_does_not_trigger_again_same_turn, plus Rust's pinned
 * "the re-yell consumed 5 deck cards" figure. */
static void test_daisuki_once_per_turn_does_not_retrigger(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();

    int daisuki = test_id(&game, DAISUKI);
    CHECK(ident(daisuki, DAISUKI), "the live card is the PL!S-bp3-020-L print");
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = mid(&game, FILLER);
    game.state.p[0].stage[1] = mid(&game, FILLER);
    int e = test_id(&game, ENERGY);
    game.state.p[0].deck.n = 0;
    game.state.p[1].deck.n = 0;
    for (int i = 0; i < 60; i++) { test_add_to_deck_pl(&game, 0, e); test_add_to_deck_pl(&game, 1, e); }
    test_give_energy(&game, 15);

    int deck_before = test_deck_len(&game);
    set_live_at_set_phase(&game, DAISUKI);
    step_to_yell_window(&game);

    int r = game.state.n_revealed;
    CHECK(r > 0, "a real yell revealed cards");
    CHECK(pending(&game), "the first trigger's discard prompt is on screen");
    CHECK_EQ(pending_kind(&game), RB_CHOICE_SELECT_CARD, "…and it is a SelectCard");
    int count = pending_count(&game);
    CHECK_EQ(count, r, "「それらのカードをすべて」 -> count == reveal count");
    if (pending(&game)) {
        int idx[RB_MAX_ZONE];
        int n = count < RB_MAX_ZONE ? count : RB_MAX_ZONE;
        for (int i = 0; i < n; i++) idx[i] = i;
        test_select_indices(&game, idx, n);
    }
    int guard = 0;
    while (pending(&game) && guard++ < 64) {
        if (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&game, -1);
        else if (pending_kind(&game) == RB_CHOICE_SELECT_CARD && pending_count(&game) > 0) {
            int n = pending_count(&game);
            int idx[RB_MAX_ZONE];
            int m = n < RB_MAX_ZONE ? n : RB_MAX_ZONE;
            for (int i = 0; i < m; i++) idx[i] = i;
            test_select_indices(&game, idx, m);
        } else answer(&game, -1);
    }
    CHECK(re_yell_happened(&game), "the first yell re-yells");
    /* Rust pins this as a NUMBER, not a derived value: the 2 discarded blade
     * hearts plus 3 from the yell itself. */
    CHECK_EQ(deck_before - test_deck_len(&game), 5,
             "the re-yell consumed 5 deck cards (2 discarded + 3 from the yell)");
    int revealed_after = game.state.n_revealed;
    int deck_after = test_deck_len(&game);

    /* Second scan in the SAME turn. */
    tas(&game, 0);
    while (pending(&game)) {
        if (pending_kind(&game) == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&game, -1);
        else answer(&game, -1);
    }
    CHECK_EQ(pending(&game), 0,
             "「{{turn1.png|ターン1回}}」: a second scan in the same turn raises no "
             "discard prompt (prompt count 0 at this site, against a first prompt "
             "that WAS on screen above — so this is not vacuous)");
    CHECK_EQ(game.state.n_revealed, revealed_after,
             "…and the blocked firing reveals nothing further");
    CHECK_EQ(test_deck_len(&game), deck_after,
             "…and consumes no further deck cards");
}

/* ====================================================================== */
/* E. JIMO-AI Dash! PL!S-sd1-020-SD — the hand-limited case                */
/*    (the other three live in test_parity_jidou_move_yell.c Y8).           */
/* ====================================================================== */

static void jimoai_scenario(TestGame *tg, int extra_hand, int n_aqours,
                            int *out_deck_delta, int *out_hand)
{
    int sd20 = test_id(tg, JIMOAI);
    CHECK(ident(sd20, JIMOAI), "the host is the PL!S-sd1-020-SD print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].deck.n = 0;
    int filler = mid(tg, FILLER);
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < n_aqours && i < 3; i++)
        tg->state.p[0].stage[i] = mid(tg, AQ_MEMBER);
    for (int i = 0; i < extra_hand; i++) test_add_to_hand(tg, mid(tg, FILLER));
    /* rb_trigger_live_success scans the board's zones, so the host must
     * actually be in one. */
    tg->state.p[0].stage[2] = sd20;
    /* rb_should_trigger_live_success (src/turn/triggers.c:197) needs BOTH
     * `p[pl].live.n > 0` AND `live_success[pl]`. The live is a SEPARATE card:
     * PL!S-sd1-020-SD is a member (cards.json: it carries special_heart {} and
     * b_all, and is not a ライブ), so the performed live is staged explicitly. */
    int performed = test_id(tg, PERFORMED_LIVE);
    test_add_to_live(tg, performed);

    int deck_before = test_deck_len(tg);
    int hand_before = test_hand_len(tg);

    /* rb_should_trigger_live_success (src/turn/triggers.c:197) needs BOTH
     * `p[pl].live.n > 0` AND `live_success[pl]`. The live is a SEPARATE card:
     * PL!S-sd1-020-SD is a member (cards.json: b_all + special_heart {}), so
     * the performed live is staged explicitly. */
    tg->state.live_success[0] = 1;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "precondition: a live in the zone and live_success[0] set, so the "
          "ライブ成功時 scan is permitted to run at all");

    rb_trigger_live_success(&tg->state, 0);

    int prompts = 0, guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 64) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        prompts++;
        if (c && c->kind == RB_CHOICE_SELECT_CARD && c->count > 0) {
            int idx[RB_MAX_ZONE];
            int n = c->count < RB_MAX_ZONE ? c->count : RB_MAX_ZONE;
            for (int i = 0; i < n; i++) idx[i] = i;
            test_select_indices(tg, idx, n);
        } else {
            rb_resume_with_choice(&tg->state, 0);
        }
    }
    CHECK(prompts > 0, "VACUITY CONTROL: the ライブ成功時 discard prompt was "
                       "actually raised and answered");
    *out_deck_delta = deck_before - test_deck_len(tg);
    *out_hand = test_hand_len(tg);
    (void)hand_before;
}

static void test_jimoai_draw_3_discard_3_limited_hand(void)
{
    static TestGame game;
    test_game_new(&game);
    rb_card_copy_pool_reset();
    int dd = 0, h = 0;
    /* 3 『Aqours』 members, 1 card in hand: draw 3 (hand 4), discard 3 -> 1. */
    jimoai_scenario(&game, 1, 3, &dd, &h);
    CHECK_EQ(dd, 3, "3 『Aqours』 members -> draw 3");
    CHECK_EQ(h, 1, "…then discard 3 of the 4 in hand, leaving the original 1");
}

/* ====================================================================== */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- TRIGGER-MISS dump (every yell-relevant card) ---\n");
    dump_trigger_miss(DIA);
    dump_trigger_miss(DAISUKI);
    dump_trigger_miss(NATSUMI);
    dump_trigger_miss(MITSUKI);
    dump_trigger_miss(JIMOAI);
    dump_trigger_miss(HASU_NO_BH);

    printf("--- A: Q251 PL!HS-bp6-027-L (revealed blade-heartless 蓮ノ空) ---\n");
    run("q251_blade_heartless_member_moves_to_waitroom",
        test_q251_blade_heartless_member_moves_to_waitroom);
    run("q251_score_heart_live_stays_revealed",
        test_q251_score_heart_live_stays_revealed);
    run("q251_blade_heart_member_offers_no_discard",
        test_q251_blade_heart_member_offers_no_discard);
    run("q251_wrong_group_offers_no_discard",
        test_q251_wrong_group_offers_no_discard);
    run("q251_mixed_pool_moves_member_keeps_score_heart_live",
        test_q251_mixed_pool_moves_member_keeps_score_heart_live);
    run("q251_optional_decline_keeps_member_revealed",
        test_q251_optional_decline_keeps_member_revealed);

    printf("--- B: Q264 PL!SP-pb2-020-R (zero-card yell must not fire) ---\n");
    run("q264_zero_yell_does_not_discard_live",
        test_q264_zero_yell_does_not_discard_live);

    printf("--- C: Q107/Q156 activation ORDER on one yell ---\n");
    run("order_filler_deck_dia_first", test_order_filler_deck_dia_first);
    run("order_mixed_deck_dia_first_then_daisuki",
        test_order_mixed_deck_dia_first_then_daisuki);
    run("order_mixed_deck_daisuki_first_then_dia",
        test_order_mixed_deck_daisuki_first_then_dia);
    run("order_revealed_live_blocks_dia_only",
        test_order_revealed_live_blocks_dia_only);

    printf("--- D: Q156 threshold + ターン1回 ---\n");
    run("daisuki_one_revealed_triggers", test_daisuki_one_revealed_triggers);
    run("daisuki_five_blade_hearts_blocks", test_daisuki_five_blade_hearts_blocks);
    run("daisuki_once_per_turn_does_not_retrigger",
        test_daisuki_once_per_turn_does_not_retrigger);

    printf("--- E: JIMO-AI Dash! hand-limited case ---\n");
    run("jimoai_draw_3_discard_3_limited_hand",
        test_jimoai_draw_3_discard_3_limited_hand);

    printf("\n== parity_yell: %d ok, %d failed, %d crashed "
           "(case in flight: %s)\n", n_ok, n_failed, n_crashed, current_case);
    printf("== %ld assertions total, %d assertion failures, %d setup bugs\n",
           assertions, failures, setup_bugs);
    return (n_failed || n_crashed) ? 1 : 0;
}
