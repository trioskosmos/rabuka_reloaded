/* test_parity_gain_delta.c
 *
 * The DELTA of engine/tests/test_modules/effects/gain/ -- the Rust test
 * functions that have NO equivalent anywhere in engine_c_wip/tests/.
 *
 * ---------------------------------------------------------------------------
 * HOW THIS FILE WAS SELECTED (see the report on the board for the full table)
 * ---------------------------------------------------------------------------
 * effects/gain/ carries 565 `#[test] fn` (740 `fn` of all kinds, helpers
 * included) across nine subclusters. Enumerating the Rust `#[test] fn` names
 * per subcluster and diffing them against the `run(...)` / test-function names
 * already present in the C suites leaves exactly three genuine gaps:
 *
 *   SECTION 0  blades/under_member/
 *              pb2_bibi_member_under_per_blade_pl_pb2_011_p_plus_test.rs
 *              9 #[test]. ZERO C coverage: `PL!-pb2-011-P+` appears in no
 *              C suite under engine_c_wip/tests at all. This is the whole
 *              delta.
 *
 *   SECTION 1  hearts/color_choice/
 *              5 of the 19 #[test] are choice-IDENTITY tests -- they pick two
 *              DIFFERENT options and assert the granted colour follows the
 *              option. The existing C translations of the two abilities
 *              (test_parity_restriction.c: test_sd2_005_... and
 *              test_kasumi_quartz_...) answer the colour prompt but then only
 *              assert "exactly one bucket is boosted, by the right amount",
 *              which an implementation that ignores the selection and always
 *              applies one fixed colour satisfies. Those two C cases even
 *              carry a comment claiming the identity is "checked below"; it
 *              is not. This section restores the pin the Rust file insists on.
 *
 *   SECTION 2  hearts/per_card/
 *              1 of the 26 #[test]: the premise audit. The two behavioural
 *              cases beside it ARE translated (test_parity_small_clusters.c
 *              a4_ranbu_*), and this one is not.
 *
 * Everything else in effects/gain/ is already translated test-for-test
 * somewhere and is deliberately NOT duplicated here; the per-subcluster
 * mapping is in the report, not in code.
 *
 * ---------------------------------------------------------------------------
 * ANTI-VACUITY DISCIPLINE
 * ---------------------------------------------------------------------------
 * Every negative in SECTION 0 ("no blade is granted") is preceded by a run of
 * the canonical POSITIVE board -- one BiBi member under ERI at Centre -- which
 * must read +1. A negative in a family whose positive arm is dead proves
 * nothing, so `CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL ...")` runs first in
 * each of those tests.
 *
 * The negative tests additionally assert that ERI really PRINTS the 常時
 * ability under test (p0_eri_prints_constant), so a "0 blade" can never be
 * explained away by the ability having gone missing from the card record.
 *
 * ---------------------------------------------------------------------------
 * LANDMINES respected (confirmed against cards/cards.json and existing suites)
 * ---------------------------------------------------------------------------
 *   - sizeof(GameState) is ~781 KB: every TestGame local is `static`.
 *   - Card identity is pinned BYTE-EXACTLY via rb_card_string(card_no_idx).
 *     rb_find_card_by_no has a forgiving fallback chain (base+rarity rebuild,
 *     then lowest prefix, then a SUBSTRING search), so a mistyped print
 *     silently resolves to a DIFFERENT real card. rb_card_no_eq is fuzzy for
 *     the same reason and is not used here.
 *   - `PL!-pb2-011-P+` carries a FULLWIDTH PLUS (U+FF0B, bytes ef bc 8b), so
 *     it is written as "\xEF\xBC\x8B". Same for `PL!N-bp1-006-R+`.
 *   - Every tracked card comes from test_new_id (the Rust `game.id()` twin).
 *     test_id aliases the shared template and is used only for deck filler,
 *     where identity does not matter.
 *   - rb_card_copy_pool_reset() runs at every board_begin: the copy pool is
 *     process-global and monotonic, so a long suite would otherwise run dry
 *     and mid() would silently fall back to the template.
 *   - Card.ability is NOT populated by rb_decode_card_by_index. The
 *     ability-shape preconditions go through rb_card_num_abilities +
 *     rb_decode_card_ability.
 *   - Heart reads go through rb_mods_get_heart directly, NOT
 *     test_get_heart_modifier, which remaps colour 5 onto RB_HEART_ORANGE and
 *     would corrupt every heart05 read.
 *   - The C deck bag is TOP-INDEX-0, matching Rust's main_deck.cards, so the
 *     30-filler fill and the deck top agree with Rust's helpers::fill_decks.
 *
* ---------------------------------------------------------------------------
 * THE OBSERVABILITY QUESTION FOR THIS FILE
 * ---------------------------------------------------------------------------
 * The wave briefs state that the flat RbChoice carries NO option list, so a
 * multi-option prompt (notably heart colour choice) is not observable from C
 * and must be filed as EXPECTED_GAP. That is OUT OF DATE for heart colour
 * choices, and this file depends on the correction -- see SECTION 1.
 * ---------------------------------------------------------------------------
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── assertions ─────────────────────────────────────────────────────────── */

static long checks;
static int failures;
static int setup_bugs;
static int n_ok, n_failed, n_crashed;
static const char *current_case = "(none)";
static const char *only_case;

#define CHECK(condition, ...) do {                                   \
        checks++;                                                   \
        if (!(condition)) {                                          \
            failures++;                                              \
            fprintf(stderr, "FAIL: ");                               \
            fprintf(stderr, __VA_ARGS__);                            \
            fprintf(stderr, "   [%s]\n", current_case);             \
        }                                                            \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                         \
        checks++;                                                    \
        long a_ = (long)(actual);                                    \
        long e_ = (long)(expected);                                  \
        if (a_ != e_) {                                              \
            failures++;                                              \
            fprintf(stderr, "FAIL: ");                               \
            fprintf(stderr, __VA_ARGS__);                            \
            fprintf(stderr, "  (got %ld, expected %ld)  [%s]\n",     \
                    a_, e_, current_case);                           \
        }                                                            \
    } while (0)

/* There is deliberately NO EXPECTED_GAP macro in this file, which is itself a
 * finding. Every case below was translatable against the C API as it stands,
 * so nothing here is filed as an unobservable gap.
 *
 * The wave briefs all asserted that a heart-colour prompt is unobservable
 * ("the flat RbChoice carries NO option list") and should be recorded as an
 * EXPECTED_GAP. That is OUT OF DATE: RbChoice does carry
 *   char heart_options[8][24]; int n_heart_options;
 * and both the prompt builder and the answer handler use them. See
 * p1_heart_colour_prompt_offers_heart01_to_heart06 below, which asserts the
 * list directly instead of shrugging at it. Recording a gap there would have
 * hidden five real assertions. */

/* ── crash harness ──────────────────────────────────────────────────────── */

static void on_signal(int sig)
{
    fprintf(stderr,
            "\n*** SIGNAL %d in test_parity_gain_delta\n"
            "*** case in flight: %s  (engine fault, not a test bug)\n"
            "*** checks so far: %ld  failures: %d\n",
            sig, current_case, checks, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHILD_OK       0
#define CHILD_FAILURES 1

struct child_totals { long checks; int failures; };
static int g_pipe[2];

static void run(const char *name, void (*fn)(void))
{
    if (only_case && !strstr(name, only_case)) return;
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        /* No fork available: run inline and bracket the delta ourselves. */
        long c0 = checks;
        int f0 = failures;
        current_case = name;
        fn();
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld checks]\n", failures > f0 ? "FAILED" : "ok",
               name, checks - c0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long c0 = checks;
        int f0 = failures;
        current_case = name;
        fn();
        struct child_totals t = { checks - c0, failures - f0 };
        ssize_t ignored = write(g_pipe[1], &t, sizeof t);
        (void)ignored;
        printf("        %s: %ld check(s), %d failure(s)\n", name, t.checks,
               t.failures);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    struct child_totals t;
    memset(&t, 0, sizeof t);
    ssize_t got = read(g_pipe[0], &t, sizeof t);
    if (got == (ssize_t)sizeof t) {
        checks += t.checks;
        failures += t.failures;
    }
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

/* ── card database ──────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* ── trigger keywords (byte-exact, copied from the card dump) ───────────── */
#define TRIG_LIVE_START "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82" /* ライブ開始時 */
#define TRIG_CONSTANT    "\xE5\xB8\xB8\xE6\x99\x82"                       /* 常時 */

/* ── heart colour slots (src/ability/util.c:rb_parse_heart_color) ────────── */
#define H01 1
#define H02 2
#define H03 3
#define H04 4
#define H05 5
#define H06 6

/* ── stage areas (engine/src/game/zones.rs MemberArea) ───────────────────── */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── card numbers ─────────────────────────────────────────────────────────
 * Every one of these was read out of cards/cards.json (UTF-8) and its
 * group/unit field recorded alongside, because group membership is exactly
 * what SECTION 0 is about. */
#define ERI        "PL!-pb2-011-P\xEF\xBC\x8B"  /* 絢瀬絵里   series ラブライブ！    unit BiBi        */
#define BIBI_ERI   "PL!-bp5-011-N"              /* 絢瀬絵里   series ラブライブ！    unit BiBi        */
#define BIBI_ERI2  "PL!-bp6-002-P"              /* 絢瀬絵里   series ラブライブ！    unit BiBi        */
#define BIBI_MAKI  "PL!-bp5-006-P"              /* 西木野真姫 series ラブライブ！    unit BiBi        */
#define PRINTEMPS  "PL!-bp6-017-N"              /* 小泉花陽   series ラブライブ！    unit Printemps  */
#define AZALEA     "PL!S-bp5-012-N"             /* 松浦果南   series サンシャイン!!  unit AZALEA      */
#define OTHER_ERI  "PL!-pb2-025-N"              /* 東條 希    series ラブライブ！    unit lily white  */

#define AI_SD2     "PL!N-sd2-005-SD2"           /* 宮下 愛    DiverDiva */
#define SD1_001    "PL!S-sd1-001-SD"            /* 高海千歌  Aqours    */
#define FILLER     "PL!-sd1-010-SD"             /* 高坂穂乃果 μ's       */

#define KASUMI_Q   "PL!N-bp3-002-R"             /* 中須かすみ QU4RTZ (虹ヶ咲) */
#define OMIKARI    "PL!N-bp1-006-R\xEF\xBC\x8B" /* 近江彼方   QU4RTZ (虹ヶ咲) */
#define AQOUR_MEM  "PL!S-bp5-009-R"             /* 黒澤ルビィ CYaRon！(Aqours) */

#define RANBU_N    "PL!N-bp7-024-N"             /* 鐘 嵐珠    R3BIRTH-print premise audit */
#define R3BIRTH_A  "PL!N-bp1-023-N"             /* ミア・テイラー */
#define R3BIRTH_B  "PL!N-bp1-024-PR"            /* 鐘 嵐珠 */
#define R3BIRTH_C  "PL!N-PR-012-PR"             /* 三船栞子 */

/* The printed group this premise audit is ABOUT. */
#define PRINTED_GROUP "R3BIRTH"
/* 虹ヶ咲学園スクールアイドル同好会 -- the tail of RANBU_N's series. 16 chars,
 * 48 UTF-8 bytes, ENCODED rather than hand-typed: the first draft of this
 * constant mistyped 虹 (E8 B6 9B, not E8 99 B9) and dropped the katakana
 * prolonged sound mark (U+30FC) inside スクール, which would have made the
 * strstr below vacuously false and the whole premise audit silently useless. */
#define NIJI_SERIES \
    "\xE8\xB6\x9B\xE3\x83\xB3\xE5\x92\xB2\xE5\xAD\xA6\xE5\x9C\x92\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE3\x83\xBC\xE5\x90\x8C\xE5\xA5\xBD\xE4\xBC\x9A"

/* ── card resolution + identity ────────────────────────────────────────── */

static const char *card_no_of(int cid)
{
    static char buf[160];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* Rust assert_card_identity: a BYTE-EXACT strcmp of the printed card_no.
 * Quiet on success; a miss is a SETUP BUG and is counted separately from an
 * engine verdict so a wrong fixture can never be reported as "the engine
 * granted nothing". */
static int pin_id(int cid, const char *want)
{
    checks++;
    if (cid < 0) {
        fprintf(stderr, "SETUP BUG: card \"%s\" did not resolve in the card DB\n",
                want);
        setup_bugs++;
        return 0;
    }
    const char *got = card_no_of(cid);
    if (strcmp(got, want) == 0) return 1;
    fprintf(stderr,
            "SETUP BUG: id %d prints \"%s\", the test wanted \"%s\"\n",
            cid, got, want);
    setup_bugs++;
    return 0;
}

/* Rust game.id(card_no): the next DISTINCT pool slot. test_new_id is the C
 * twin; test_id aliases the shared template, so a tracked card must never use
 * it. Falls back to the template when the copy pool is exhausted, matching
 * Rust's documented fallback. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id >= 0) return id;
    return test_id(tg, no);
}

/* Rust game.id_ref(card_no): the SHARED template slot. */
static int id_ref(TestGame *tg, const char *no) { return test_id(tg, no); }

/* Resolve + pin in one call. Returns -1 after reporting a setup bug so the
 * caller can return early rather than assert against the wrong card. */
static int card(TestGame *tg, const char *no)
{
    int id = mid(tg, no);
    if (!pin_id(id, no)) return -1;
    return id;
}

/* ── board helpers ──────────────────────────────────────────────────────── */

static void clear_seats(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        P->hand.n = 0;
        P->discard.n = 0;
        P->live.n = 0;
        P->success.n = 0;
        P->energy.n = 0;
        P->energy_deck.n = 0;
        P->energy_active = 0;
        P->deck_refreshed_this_turn = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->stage_wait[i] = 0;
            P->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_batch_movements = 0;
    tg->state.yell_occurred = 0;
    tg->state.re_yell_occurred = 0;
    rb_clear_pending_choice(&tg->state);
}

/* Every board starts here. The pool reset is REQUIRED, not cosmetic:
 * rb_create_card_copy draws from a process-global monotonic pool that nothing
 * else resets, so a suite that lets it run dry gets -1 from test_new_id and
 * mid() silently falls back to the shared template -- after which two different
 * prints in one case resolve to the SAME id and every identity pin goes
 * vacuous. */
static void new_board(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    clear_seats(tg);
}

static void set_stage(TestGame *tg, int pl, int left, int center, int right)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[AREA_LEFT] = left;
    P->stage[AREA_CENTER] = center;
    P->stage[AREA_RIGHT] = right;
}

static int under_n(TestGame *tg, int pl, int area)
{
    return tg->state.p[pl].under_cards[area].n;
}

/* Rust mods.get_blade_modifier(cid). */
static int blade_of(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}

/* Rust mods.get_heart_modifier(cid, color). rb_mods_get_heart DIRECTLY:
 * test_get_heart_modifier remaps colour 5 onto RB_HEART_ORANGE. */
static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}

/* Rust helpers::fill_decks: clear both main decks, 30 fillers each. */
static void fill_decks30(TestGame *tg, int filler)
{
    RbPlayer *P = &tg->state.p[0];
    RbPlayer *O = &tg->state.p[1];
    P->deck.n = 0;
    O->deck.n = 0;
    for (int i = 0; i < 30; i++) {
        P->deck.cards[P->deck.n++] = filler;
        O->deck.cards[O->deck.n++] = filler;
    }
}

/* Diagnostic: name every heart bucket the engine actually wrote, so a failure
 * report says WHAT was granted instead of only what was expected. Bucket 0 is
 * reported separately because it is the wildcard that would silently tint all
 * six. */
static void report_hearts(TestGame *tg, int cid, const char *label)
{
    fprintf(stderr, "  %s %s: wild(pink)=%d", label, card_no_of(cid),
            rb_mods_get_heart(&tg->state.mods, cid, 0));
    for (int c = 1; c <= 6; c++)
        fprintf(stderr, " heart%02d=%d", c,
                rb_mods_get_heart(&tg->state.mods, cid, c));
    fputc('\n', stderr);
}

/* ── choice answering ───────────────────────────────────────────────────── */

/* Rust select_indices(&[i]). */
static void pick(TestGame *tg, int idx)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

/* Rust select_indices(&[i, j]). */
static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Rust select_option(n). */
static void opt(TestGame *tg, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    rb_resume_with_choice(&tg->state, n);
}

/* ── trigger shim (Rust helpers::fire_trigger) ───────────────────────────
 * Resolve ONE ability on ONE card, key it "{card_no}_{full_text}", fire it
 * with `cid` as the activating card, then process the queue.
 * Does NOT drain beyond that, so the caller still observes the prompt Rust
 * observes.
 *
 * `nth` selects the Nth PRINTED ability carrying the trigger, which is exactly
 * Rust's `fire_trigger_nth`; the shared test_fire_trigger_on_card_nth already
 * implements that selection, so this wrapper no longer re-derives it.
 *
 * The body used to be a hand-rolled copy of that sequence, which recorded NO
 * enqueue-time use — so a 自動/一度だけ ability was still unspent at
 * resolution and the post-resolution re-scan in
 * rb_process_player_abilities_depth_full enqueued it a SECOND time. Its
 * "{card_no}_{full_text}" key also truncated at 1024 bytes. */
static int fire_trigger(TestGame *tg, int cid, const char *trig,
                        const char *trigger_type, int nth)
{
    (void)trigger_type;   /* every call site here passes trig == trigger_type */
    if (test_fire_trigger_on_card_nth(tg, cid, trig, nth)) return 1;
    /* Diagnostic: a mismatch here is a HARNESS bug, not an engine bug, so
     * it must be loud rather than silently becoming a vacuous negative. */
    {
        char card_no[160];
        card_no[0] = '\0';
        Card c;
        if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
            snprintf(card_no, sizeof(card_no), "%s", rb_card_string(c.card_no_idx));
            rb_free_card(&c);
        }
        int nab = cid >= 0 ? rb_card_num_abilities((uint32_t)cid) : 0;
        fprintf(stderr, "[TRIGGER-MISS] card %s: no ability with triggers \"%s\"; has:",
                card_no, trig);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            fprintf(stderr, " [%d]=\"%s\"", a,
                    ab.triggers ? ab.triggers : "(null)");
            rb_free_ability(&ab);
        }
        fprintf(stderr, "\n");
    }
    return 0;
}

static int fire_live_start(TestGame *tg, int cid)
{
    return fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START, 0);
}

/* ════════════════════════════════════════════════════════════════════════════
 * SECTION 0 -- the entire delta of blades/under_member/
 *   pb2_bibi_member_under_per_blade_pl_pb2_011_p_plus_test.rs   (9 #[test])
 *
 *   絢瀬絵里 `PL!-pb2-011-P+` ab#0 常時:
 *     gain_resource{ blade, count:1, per_unit:true, per_unit_count:1,
 *                     per_unit_type:"枚", card_type:"member_card",
 *                     group_names:["BiBi"], location:"under_member" }
 *
 *   「このメンバーの下に置かれている『BiBi』のメンバーカード1枚につき、
 *     ブレードを得る」 -- TWO constraints stack: the card must be physically
 *   under THIS member, AND it must be a BiBi member. Each is isolated below,
 *   plus the "under a different member" case, which is what distinguishes
 *   このメンバーの下 from "any under-card".
 *
 *   ZERO existing C coverage: `PL!-pb2-011` appears in no engine_c_wip/tests/
 *   file. Note the neighbouring `PL!SP-pb2-011-R` (鬼塚冬毬) is a DIFFERENT
 *   card and is out of scope; every fixture here is byte-pinned so the
 *   pb2/bp2 confusion cannot creep in.
 * ═════════════════════════════════════════════════════════════════════════ */

/* Precondition, read off the decoded ability rather than assumed: ERI's ab#0
 * really is the 常時 gain_resource{blade, per_unit, under_member, BiBi}.
 * Card.ability is not populated by rb_decode_card_by_index, so this goes
 * through rb_card_num_abilities + rb_decode_card_ability.
 *
 * `tg` must already have been through new_board(). */
static void p0_assert_ability_shape(TestGame *tg)
{
    int eri = id_ref(tg, ERI);
    if (eri < 0) {
        CHECK(0, "precondition: %s must resolve", ERI);
        return;
    }
    int nab = rb_card_num_abilities((uint32_t)eri);
    int hits = 0;
    for (int a = 0; a < nab; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)eri, a, &ab)) continue;
        int trig_hit = ab.triggers && strcmp(ab.triggers, TRIG_CONSTANT) == 0;
        const AbilityEffect *e = ab.effect;
        if (trig_hit && e && e->action && strcmp(e->action, "gain_resource") == 0
            && e->per_unit) {
            const char *loc = NULL;
            for (int k = 0; k < e->n_extra; k++)
                if (e->extra_k[k] && strcmp(e->extra_k[k], "location") == 0)
                    loc = e->extra_v[k];
            fprintf(stderr,
                    "  [WIRE] %s ab#%d gain_resource per_unit=%d "
                    "per_unit_count=%d card_type=%s location=%s\n",
                    ERI, a, e->per_unit, e->per_unit_count,
                    e->card_type_field[0] ? e->card_type_field : "(none)",
                    loc ? loc : "(none)");
            hits++;
        }
        rb_free_ability(&ab);
    }
    CHECK_EQ(hits, 1,
             "precondition: ERI carries exactly one 常時 per_unit "
             "gain_resource ability");
}

/* Self-contained wrapper: owns a board so the shape precondition can be run
 * from any test without the caller borrowing a 781 KB stack slot. */
static void p0_wire_shape_check(void)
{
    static TestGame tg;
    new_board(&tg);
    p0_assert_ability_shape(&tg);
}

/* Rust `fn blade_with_under(under: &[&str])`: ERI in the centre, each named
 * card placed under her, recalculate_constants, read ERI's blade modifier.
 * Returns -1 on a setup bug. */
static int p0_blade_with_under(TestGame *tg, const char *const *under, int n)
{
    new_board(tg);
    int eri = card(tg, ERI);
    if (eri < 0) return -1;
    set_stage(tg, 0, RB_EMPTY_SLOT, eri, RB_EMPTY_SLOT);
    for (int i = 0; i < n; i++) {
        int id = card(tg, under[i]);
        if (id < 0) return -1;
        test_place_under(tg, 0, AREA_CENTER, id);
    }
    CHECK_EQ(under_n(tg, 0, AREA_CENTER), n, "setup: under-stack really holds n cards");
    test_recalc(tg);
    return blade_of(tg, eri);
}

/* ANTI-VACUITY CONTROL: the canonical positive board. Every negative in this
 * section runs it first and requires +1, so a "0 blade" below can only mean
 * the filter rejected the card -- never "the engine granted nothing at all".
 * Returns -1 on a setup bug. */
static int p0_control_one_bibi(void)
{
    static TestGame tg;
    const char *const only[1] = { BIBI_ERI };
    return p0_blade_with_under(&tg, only, 1);
}

/* 1 :: one_bibi_under_grants_one_blade */
static void p0_one_bibi_under_grants_one_blade(void)
{
    static TestGame tg;
    p0_wire_shape_check();
    const char *const under[1] = { BIBI_ERI };
    int got = p0_blade_with_under(&tg, under, 1);
    if (got < 0) return;
    CHECK_EQ(got, 1,
             "『BiBi』のメンバーカード1枚につき -- one BiBi member under is +1 blade");
}

/* 2 :: two_bibi_under_grant_two_blades */
static void p0_two_bibi_under_grant_two_blades(void)
{
    static TestGame tg;
    const char *const under[2] = { BIBI_ERI, BIBI_MAKI };
    int got = p0_blade_with_under(&tg, under, 2);
    if (got < 0) return;
    CHECK_EQ(got, 2, "1枚につき scales -- two BiBi members under are +2");
}

/* 3 :: three_bibi_under_grant_three_blades */
static void p0_three_bibi_under_grant_three_blades(void)
{
    static TestGame tg;
    const char *const under[3] = { BIBI_ERI, BIBI_MAKI, BIBI_ERI2 };
    int got = p0_blade_with_under(&tg, under, 3);
    if (got < 0) return;
    CHECK_EQ(got, 3,
             "1枚につき keeps scaling to 3, including two prints of one character");
}

/* 4 :: nothing_under_grants_nothing */
static void p0_nothing_under_grants_nothing(void)
{
    static TestGame tg;
    CHECK_EQ(p0_control_one_bibi(), 1,
             "ANTI-VACUITY CONTROL: one BiBi member under must grant +1 before "
             "the no-card negative means anything");
    int got = p0_blade_with_under(&tg, NULL, 0);
    if (got < 0) return;
    CHECK_EQ(got, 0, "このメンバーの下に置かれているカードが0枚 is +0");
}

/* 5 :: non_bibi_member_under_is_not_counted */
static void p0_non_bibi_member_under_is_not_counted(void)
{
    static TestGame tg;
    CHECK_EQ(p0_control_one_bibi(), 1,
             "ANTI-VACUITY CONTROL: one BiBi member under must grant +1");
    const char *const under[1] = { PRINTEMPS };
    int got = p0_blade_with_under(&tg, under, 1);
    if (got < 0) return;
    CHECK_EQ(got, 0,
             "小泉花陽 is a Printemps member, not 『BiBi』, so it is excluded");
}

/* 6 :: mixed_under_counts_only_bibi_members */
static void p0_mixed_under_counts_only_bibi_members(void)
{
    static TestGame tg;
    CHECK_EQ(p0_control_one_bibi(), 1,
             "ANTI-VACUITY CONTROL: one BiBi member under must grant +1");
    const char *const under[4] = { PRINTEMPS, BIBI_ERI, AZALEA, BIBI_MAKI };
    int got = p0_blade_with_under(&tg, under, 4);
    if (got < 0) return;
    CHECK_EQ(got, 2, "of four cards under her, only the two BiBi members count");
}

/* 7 :: bibi_under_a_different_member_is_not_counted */
static void p0_bibi_under_a_different_member_is_not_counted(void)
{
    static TestGame tg;
    CHECK_EQ(p0_control_one_bibi(), 1,
             "ANTI-VACUITY CONTROL: one BiBi member under must grant +1");

    new_board(&tg);
    int eri = card(&tg, ERI);
    int other = card(&tg, OTHER_ERI);
    if (eri < 0 || other < 0) return;
    set_stage(&tg, 0, other, eri, RB_EMPTY_SLOT);

    /* Hang the BiBi member under the LEFT slot, not under ERI in the centre. */
    int bibi = card(&tg, BIBI_ERI);
    if (bibi < 0) return;
    test_place_under(&tg, 0, AREA_LEFT, bibi);
    CHECK_EQ(under_n(&tg, 0, AREA_LEFT), 1, "setup: the BiBi member is under the LEFT slot");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, eri), 0,
             "このメンバーの下 -- a BiBi member under a DIFFERENT stage member "
             "does not count for ERI");
}

/* 8 :: bibi_members_elsewhere_are_not_counted */
static void p0_bibi_members_elsewhere_are_not_counted(void)
{
    static TestGame tg;
    CHECK_EQ(p0_control_one_bibi(), 1,
             "ANTI-VACUITY CONTROL: one BiBi member under must grant +1");

    new_board(&tg);
    int eri = card(&tg, ERI);
    if (eri < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, eri, RB_EMPTY_SLOT);

    int hand_eri = card(&tg, BIBI_ERI);
    int discard_maki = card(&tg, BIBI_MAKI);
    if (hand_eri < 0 || discard_maki < 0) return;
    test_add_to_hand(&tg, hand_eri);
    test_add_to_discard(&tg, discard_maki);
    /* ...and one genuinely underneath, to prove the fixture is not dead. */
    int under_eri2 = card(&tg, BIBI_ERI2);
    if (under_eri2 < 0) return;
    test_place_under(&tg, 0, AREA_CENTER, under_eri2);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1,
             "setup: exactly one card really is underneath");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, eri), 1,
             "only the card physically underneath counts; hand and discard "
             "BiBi members do not");
}

/* 9 :: off_stage_grants_nothing */
static void p0_off_stage_grants_nothing(void)
{
    static TestGame tg;
    CHECK_EQ(p0_control_one_bibi(), 1,
             "ANTI-VACUITY CONTROL: one BiBi member under must grant +1");

    new_board(&tg);
    int eri = card(&tg, ERI);
    int bibi = card(&tg, BIBI_ERI);
    if (eri < 0 || bibi < 0) return;
    /* ERI is deliberately NOT placed in any stage slot. */
    test_place_under(&tg, 0, AREA_CENTER, bibi);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: the BiBi member is under the centre slot");
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK_EQ(tg.state.p[0].stage[i], RB_EMPTY_SLOT,
                 "setup: P1 stage slot %d is empty (ERI is off stage)", i);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, eri), 0, "常時 -- off stage she contributes no blade");
}

/* ════════════════════════════════════════════════════════════════════════════
 * SECTION 1 -- hearts/color_choice/ choice-IDENTITY delta   (5 #[test])
 *
 * These are the tests the Rust file was REWRITTEN around. Its header is
 * explicit about why:
 *   "The previous version of this file checked the SHAPE of the outcome --
 *    exactly one colour boosted, by exactly +2 -- by reading whichever colour
 *    ended up boosted. That is satisfied by an implementation that ignores
 *    the player's selection and always applies the first option, so the six
 *    options were indistinguishable to it. These tests therefore name the
 *    expected colour, and pick a different one in each, so 'always option 0'
 *    fails immediately."
 *
 * The C translation in test_parity_restriction.c answered the colour prompt
 * with opt(3) / opt(4) and then asserted only `n_boosted == 1` and
 * `amount == 2` / `== 1`. That is exactly the shape assertion the Rust
 * comment rejects. The fix is not a new mechanism, it is pinning WHICH bucket
 * the engine wrote -- which is fully observable through rb_mods_get_heart.
 *
 * Each case selects ONE option index and then reads back which heart bucket
 * the engine actually wrote. The index-to-colour mapping that makes that
 * meaningful is asserted outright by
 * p1_heart_colour_prompt_offers_heart01_to_heart06 below.
 * ═════════════════════════════════════════════════════════════════════════ */

/* paid_discard_color_choice_heart_test::boosted_colour_for_option:
 * drive 宮下愛's ライブ開始時, PAY the optional discard-2 cost, pick `option`,
 * and report the single heart colour the engine boosted. Writes the colour
 * (1..6) into *out_colour and the amount into *out_amount; returns the count
 * of boosted buckets. Returns -1 on a setup bug. */
static int p1_sd2_005_boosted(TestGame *tg, int option, int *out_colour,
                              int *out_amount)
{
    new_board(tg);
    int filler = card(tg, FILLER);
    if (filler < 0) return -1;
    fill_decks30(tg, filler);

    int me = card(tg, AI_SD2);
    if (me < 0) return -1;
    set_stage(tg, 0, RB_EMPTY_SLOT, me, RB_EMPTY_SLOT);

    int h1 = card(tg, FILLER);
    int h2 = card(tg, SD1_001);
    if (h1 < 0 || h2 < 0) return -1;
    test_add_to_hand(tg, h1);
    test_add_to_hand(tg, h2);

    if (!fire_live_start(tg, me)) {
        CHECK(0, "precondition: %s must carry a ライブ開始時 ability", AI_SD2);
        return -1;
    }
    /* OPTIONAL COST -- a prompt we did not open means the ability never ran. */
    CHECK(rb_has_pending_choice(&tg->state), "discard-2 cost prompted");
    if (!rb_has_pending_choice(&tg->state)) return -1;
    {
        const int two[2] = { 0, 1 };
        pick_many(tg, two, 2);
    }
    CHECK(rb_has_pending_choice(&tg->state),
          "好きなハートの色を1つ指定する -- the colour choice must be offered "
          "after the cost");
    if (!rb_has_pending_choice(&tg->state)) return -1;
    opt(tg, option);

    int n_boosted = 0;
    int chosen = -1;
    for (int c = 1; c <= 6; c++) {
        int v = heart_of(tg, me, c);
        if (v > 0) { n_boosted++; chosen = c; }
    }
    if (n_boosted == 1) {
        *out_colour = chosen;
        *out_amount = heart_of(tg, me, chosen);
    } else {
        *out_colour = -1;
        *out_amount = 0;
    }
    report_hearts(tg, me, "sd2-005[opt]");
    return n_boosted;
}

/* 7 :: pl_n_sd2_005_sd2_paid_discard_grants_exactly_two_hearts_of_one_color */
static void p1_sd2_005_option3_is_heart04_by_2(void)
{
    static TestGame t1;
    int colour = -1, amount = 0;
    int n = p1_sd2_005_boosted(&t1, 3, &colour, &amount);
    if (n < 0) return;
    CHECK_EQ(n, 1, "そのハートを2つ得る -- exactly one colour is boosted, whichever was chosen");
    if (n != 1) return;
    CHECK_EQ(colour, H04,
             "the colour the PLAYER chose must be the one boosted -- option 3 of "
             "heart01..heart06 is heart04. A different colour here means the "
             "selection is being ignored");
    CHECK_EQ(amount, 2, "そのハートを2つ得る -- exactly +2, not the whole colour set");
}

/* 8 :: pl_n_sd2_005_sd2_the_chosen_colour_follows_the_option_not_a_fixed_default */
static void p1_sd2_005_colour_follows_the_option(void)
{
    static TestGame t1, t2;
    int c1 = -1, a1 = 0, c2 = -1, a2 = 0;
    int n1 = p1_sd2_005_boosted(&t1, 3, &c1, &a1);
    int n2 = p1_sd2_005_boosted(&t2, 5, &c2, &a2);
    if (n1 < 0 || n2 < 0) return;
    CHECK_EQ(n1, 1, "option 3 run: exactly one colour boosted");
    CHECK_EQ(n2, 1, "option 5 run: exactly one colour boosted");
    if (n1 != 1 || n2 != 1) return;
    CHECK_EQ(c1, H04, "option 3 is heart04");
    CHECK_EQ(c2, H06, "option 5 is heart06");
    CHECK(c1 != c2,
          "two different options must grant two different colours -- if these are "
          "equal the engine is not reading the player's selection at all");
    CHECK_EQ(a1, 2, "option 3 grants +2 of that colour");
    CHECK_EQ(a2, 2, "option 5 grants +2 of that colour");
}

/* 9 :: pl_n_sd2_005_sd2_first_option_grants_heart01_not_a_default */
static void p1_sd2_005_option0_is_heart01(void)
{
    static TestGame t1;
    int colour = -1, amount = 0;
    int n = p1_sd2_005_boosted(&t1, 0, &colour, &amount);
    if (n < 0) return;
    CHECK_EQ(n, 1, "option 0 run: exactly one colour boosted");
    if (n != 1) return;
    CHECK_EQ(colour, H01,
             "option 0 is heart01; this pins the first option so the default "
             "cannot pass by coincidence with the other cases");
    CHECK_EQ(amount, 2, "そのハートを2つ得る");
}

/* paid_heart_color_choice_test::granted_colour_to_other_for_option:
 * drive 中須かすみ's ライブ開始時, PAY the optional discard-1 cost, pick
 * `option`, and report the single heart colour granted to the OTHER 虹ヶ咲
 * member. かすみ herself and the Aqours member must gain nothing.
 * Returns the number of boosted buckets on `other`; -1 on a setup bug. */
static int p1_kasumi_granted_to_other(TestGame *tg, int option,
                                       int *out_colour, int *out_amount)
{
    new_board(tg);
    int kasumu = card(tg, KASUMI_Q);
    int other = card(tg, OMIKARI);
    int outsider = card(tg, AQOUR_MEM);
    if (kasumu < 0 || other < 0 || outsider < 0) return -1;
    set_stage(tg, 0, other, kasumu, outsider);

    int filler = card(tg, FILLER);
    if (filler < 0) return -1;
    test_add_to_hand(tg, filler);

    if (!fire_live_start(tg, kasumu)) {
        CHECK(0, "precondition: %s must carry a ライブ開始時 ability", KASUMI_Q);
        return -1;
    }
    /* OPTIONAL COST ~~~~1~~~~ is presented directly as a skippable hand
     * SelectCard: pick the single card to PAY. */
    CHECK(rb_has_pending_choice(&tg->state), "cost prompt appears");
    if (!rb_has_pending_choice(&tg->state)) return -1;
    pick(tg, 0);

    CHECK(rb_has_pending_choice(&tg->state),
          "好きなハートの色を1つ指定する -- the colour specification must be asked");
    if (!rb_has_pending_choice(&tg->state)) return -1;
    opt(tg, option);

    /* exclude_self and the group filter, both asserted exhaustively. */
    for (int c = 1; c <= 6; c++) {
        CHECK_EQ(heart_of(tg, kasumu, c), 0,
                 "このメンバー以外 -- かすみ herself gains nothing (heart%02d)", c);
        CHECK_EQ(heart_of(tg, outsider, c), 0,
                 "『虹ヶ咲』のメンバー only -- an Aqours member gains nothing (heart%02d)", c);
    }

    int n_granted = 0;
    int chosen = -1;
    for (int c = 1; c <= 6; c++) {
        int v = heart_of(tg, other, c);
        if (v > 0) { n_granted++; chosen = c; }
    }
    if (n_granted == 1) {
        *out_colour = chosen;
        *out_amount = heart_of(tg, other, chosen);
    } else {
        *out_colour = -1;
        *out_amount = 0;
    }
    report_hearts(tg, other, "kasumi-other[opt]");
    report_hearts(tg, kasumu, "kasumi-self[opt]");
    return n_granted;
}

/* 10 :: kasumi_n_bp3_002_live_start_discard_cost_grants_chosen_heart_only_to_other_nijigasaki */
static void p1_kasumi_option4_is_heart05_by_1(void)
{
    static TestGame t1;
    int colour = -1, amount = 0;
    int n = p1_kasumi_granted_to_other(&t1, 4, &colour, &amount);
    if (n < 0) return;
    CHECK_EQ(n, 1, "exactly one specified colour granted to the other 虹ヶ咲 member");
    if (n != 1) return;
    CHECK_EQ(colour, H05,
             "the colour the PLAYER chose must be the one granted -- option 4 of "
             "heart01..heart06 is heart05. Any other colour means the selection "
             "is being ignored");
    CHECK_EQ(amount, 1, "そのハートを1つ得る -- exactly +1");
}

/* 11 :: kasumi_the_granted_colour_follows_the_option_not_a_fixed_default */
static void p1_kasumi_colour_follows_the_option(void)
{
    static TestGame t1, t2;
    int c1 = -1, a1 = 0, c2 = -1, a2 = 0;
    int n1 = p1_kasumi_granted_to_other(&t1, 4, &c1, &a1);
    int n2 = p1_kasumi_granted_to_other(&t2, 1, &c2, &a2);
    if (n1 < 0 || n2 < 0) return;
    CHECK_EQ(n1, 1, "option 4 run: exactly one colour granted");
    CHECK_EQ(n2, 1, "option 1 run: exactly one colour granted");
    if (n1 != 1 || n2 != 1) return;
    CHECK_EQ(c1, H05, "option 4 is heart05");
    CHECK_EQ(c2, H02, "option 1 is heart02");
    CHECK(c1 != c2,
          "two different options must grant two different colours -- if these are "
          "equal the engine is not reading the player's selection at all");
    CHECK_EQ(a1, 1, "option 4 grants +1 of that colour");
    CHECK_EQ(a2, 1, "option 1 grants +1 of that colour");
}

/* The heart-colour prompt's OPTION LIST -- and a correction to a standing
 * assumption in this wave's briefs.
 *
 * The briefs for this wave all say "the flat RbChoice carries NO option list,
 * so a heart colour choice is NOT observable from C -- record it as
 * EXPECTED_GAP". That is OUT OF DATE for this prompt kind, and recording a GAP
 * here would understate what the tree can actually prove:
 *
 *   src/ability/effects/state.c:1099-1116  rb_effect_specify_heart_color
 *     ch->kind          = RB_CHOICE_SELECT_HEART_COLOR
 *     ch->n_heart_options = 6
 *     ch->heart_options[0..5] = "heart01".."heart06"
 *
 * and src/ability/choice.c:3996-3998 maps the answer back through
 *   rb_parse_heart_color(g->queue.pending.heart_options[selected_idx])
 * so option index i means heart0(i+1) -- the same order Rust's
 * Choice::SelectHeartColor presents (engine/src/ability/effects/
 * required_hearts.rs:352-366, which also excludes heart00: 「Q190 (2025.11.17):
 * ALL heart (heart00) cannot be selected」).
 *
 * So the list is directly readable and is asserted below, rather than being
 * filed as an unobservable gap. Asserting it is what makes the option indices
 * in SECTION 1 mean anything: an index only pins a colour if index i is really
 * heart0(i+1), and that is an engine claim, not an assumption.
 *
 * (The genuinely flat prompts -- cost options, target lists and so on -- still
 * carry no list. That is a separate limitation and is not what this case
 * measures.) */
static void p1_heart_colour_prompt_offers_heart01_to_heart06(void)
{
    static TestGame tg;
    new_board(&tg);
    int filler = card(&tg, FILLER);
    if (filler < 0) return;
    fill_decks30(&tg, filler);
    int me = card(&tg, AI_SD2);
    if (me < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, me, RB_EMPTY_SLOT);
    int h1 = card(&tg, FILLER);
    if (h1 < 0) return;
    test_add_to_hand(&tg, h1);

    if (!fire_live_start(&tg, me)) {
        CHECK(0, "precondition: %s must carry a ライブ開始時 ability", AI_SD2);
        return;
    }
    /* Pay the optional discard-2 cost, then the colour prompt is next. */
    {
        const int two[2] = { 0, 1 };
        pick_many(&tg, two, 2);
    }
    const RbChoice *c = rb_get_pending_choice(&tg.state);
    CHECK(c != NULL,
          "好きなハートの色を1つ指定する -- the colour prompt follows the cost");
    if (!c) return;

    printf("  [CHOICE] kind=%d count=%d n_heart_options=%d zone=\"%s\" target=\"%s\"\n",
           (int)c->kind, c->count, c->n_heart_options,
           c->zone[0] ? c->zone : "", c->target[0] ? c->target : "");
    for (int i = 0; i < c->n_heart_options && i < 8; i++)
        printf("  [CHOICE] heart_options[%d] = \"%s\"\n", i, c->heart_options[i]);

    CHECK_EQ((int)c->kind, (int)RB_CHOICE_SELECT_HEART_COLOR,
             "the colour prompt is a SelectHeartColor");
    CHECK_EQ(c->count, 1, "Rust offers count: 1 -- one colour is chosen");
    CHECK_EQ(c->n_heart_options, 6,
             "the six individual colours are offered (Q190: heart00 is excluded)");
    static const char *const want[6] = { "heart01", "heart02", "heart03",
                                         "heart04", "heart05", "heart06" };
    for (int i = 0; i < 6; i++)
        CHECK(strcmp(c->heart_options[i], want[i]) == 0,
              "option index %d is %s (read \"%s\") -- this is what makes "
              "select_option(i) in SECTION 1 pin a specific colour",
              i, want[i], c->heart_options[i]);

    rb_clear_pending_choice(&tg.state);
}

/* ════════════════════════════════════════════════════════════════════════════
 * SECTION 2 -- hearts/per_card/ premise audit   (1 #[test])
 *
 *   subunit_stage_count_gated_debut_heart_test.rs
 *     :: n_bp7_024_n_fixture_group_membership_is_documented
 *
 * The two BEHAVIOURAL cases in that file are already translated
 * (test_parity_small_clusters.c a4_ranbu_three_r3birth_gains_heart01 /
 * a4_ranbu_two_r3birth_no_heart01). This one is the premise audit that sits
 * above them and is not.
 *
 * It is a CORPUS test, not an engine test: it reads `series` off four card
 * records and asserts that no card is tagged R3BIRTH (because no `series` value
 * in the corpus contains that substring -- it is a sub-group the series field
 * does not model) and that 鐘嵐珠's series IS 虹ヶ咲. If the corpus ever grows
 * a real R3BIRTH series, the 3人 premise of the two behavioural tests becomes
 * checkable and this assertion fails loudly, which is the whole point.
 * ═════════════════════════════════════════════════════════════════════════ */

/* The engine-decoded `series` string for an ALREADY-PINNED card id.
 * Deliberately takes the id rather than a card number: rb_find_card_by_no has
 * a forgiving fallback chain, so resolving the number a second time could
 * quietly land on a different card than the one pin_id just byte-verified.
 * Card.ability is not populated by rb_decode_card_by_index, but series_idx is,
 * and rb_card_string() resolves the shared string table. */
static const char *series_of_id(int id)
{
    static char buf[256];
    buf[0] = 0;
    Card c;
    if (id < 0 || !rb_decode_card_by_index((uint32_t)id, &c)) return buf;
    const char *s = rb_card_string(c.series_idx);
    if (s) snprintf(buf, sizeof buf, "%s", s);
    rb_free_card(&c);
    return buf;
}

static void p2_n_bp7_024_n_fixture_group_membership_is_documented(void)
{
    static TestGame tg;
    new_board(&tg);

    const char *fixtures[4] = { RANBU_N, R3BIRTH_A, R3BIRTH_B, R3BIRTH_C };
    int ranbu_id = RB_EMPTY_SLOT;
    for (int i = 0; i < 4; i++) {
        const char *no = fixtures[i];
        int id = mid(&tg, no);
        if (!pin_id(id, no)) continue;
        if (i == 0) ranbu_id = id;
        const char *series = series_of_id(id);
        /* The group filter is carried in the WIRE, so record what the
         * group→series derivation makes of each fixture too. */
        char group[128];
        rb_map_series_to_group(series, group, sizeof group);
        int blade = 0;
        Card c;
        if (rb_decode_card_by_index((uint32_t)id, &c)) {
            blade = rb_card_has_blade_heart(&c);
            rb_free_card(&c);
        }
        printf("%-24s series=\"%s\" -> group \"%s\" blade_heart=%s\n", no,
               series, group, blade ? "yes" : "no");
        CHECK(strstr(series, PRINTED_GROUP) == NULL,
              "premise %s: series does NOT contain \"%s\"", no, PRINTED_GROUP);
    }

    const char *me_series = series_of_id(ranbu_id);
    CHECK(me_series[0] != 0,
          "premise: %s must resolve so its series can be read", RANBU_N);
    CHECK(strstr(me_series, NIJI_SERIES) != NULL,
          "If %s's series is no longer 虹ヶ咲 (read \"%s\"), the premise of the "
          "two behavioural cases has changed and the 『R3BIRTH』 question needs "
          "reopening.", RANBU_N, me_series);
}

/* ════════════════════════════════════════════════════════════════════════════
 * harness
 * ═════════════════════════════════════════════════════════════════════════ */

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    setvbuf(stderr, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_signal);
    signal(SIGBUS, on_signal);
    signal(SIGABRT, on_signal);

    if (pipe(g_pipe) != 0) {
        fprintf(stderr, "FAIL: could not create the child-report pipe\n");
        return 1;
    }

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database "
                        "(tried \"src\" and \"../cards/build\")\n");
        return 1;
    }

    const char *only = (argc > 1) ? argv[1] : NULL;
    only_case = only;
    printf("==== test_parity_gain_delta ====\n");

    printf("--- SECTION 0 blades/under_member/pb2_bibi_member_under (PL!-pb2-011-P+) ---\n");
    run("one_bibi_under_grants_one_blade",
        p0_one_bibi_under_grants_one_blade);
    run("two_bibi_under_grant_two_blades",
        p0_two_bibi_under_grant_two_blades);
    run("three_bibi_under_grant_three_blades",
        p0_three_bibi_under_grant_three_blades);
    run("nothing_under_grants_nothing",
        p0_nothing_under_grants_nothing);
    run("non_bibi_member_under_is_not_counted",
        p0_non_bibi_member_under_is_not_counted);
    run("mixed_under_counts_only_bibi_members",
        p0_mixed_under_counts_only_bibi_members);
    run("bibi_under_a_different_member_is_not_counted",
        p0_bibi_under_a_different_member_is_not_counted);
    run("bibi_members_elsewhere_are_not_counted",
        p0_bibi_members_elsewhere_are_not_counted);
    run("off_stage_grants_nothing",
        p0_off_stage_grants_nothing);

    printf("--- SECTION 1 hearts/color_choice choice-identity delta ---\n");
    run("pl_n_sd2_005_sd2_paid_discard_grants_exactly_two_hearts_of_one_color",
        p1_sd2_005_option3_is_heart04_by_2);
    run("pl_n_sd2_005_sd2_the_chosen_colour_follows_the_option_not_a_fixed_default",
        p1_sd2_005_colour_follows_the_option);
    run("pl_n_sd2_005_sd2_first_option_grants_heart01_not_a_default",
        p1_sd2_005_option0_is_heart01);
    run("kasumi_n_bp3_002_live_start_discard_cost_grants_chosen_heart_only_to_other_nijigasaki",
        p1_kasumi_option4_is_heart05_by_1);
    run("kasumi_the_granted_colour_follows_the_option_not_a_fixed_default",
        p1_kasumi_colour_follows_the_option);
    run("heart_colour_prompt_offers_heart01_to_heart06",
        p1_heart_colour_prompt_offers_heart01_to_heart06);

    printf("--- SECTION 2 hearts/per_card R3BIRTH premise audit ---\n");
    run("n_bp7_024_n_fixture_group_membership_is_documented",
        p2_n_bp7_024_n_fixture_group_membership_is_documented);

    printf("\n==== parity_gain_delta: %d cases (%d ok, %d failed, %d crashed), "
           "%ld checks, %d failures, %d setup bugs ====\n",
           n_ok + n_failed + n_crashed, n_ok, n_failed, n_crashed, checks,
           failures, setup_bugs);

    rb_unload();
    return (failures == 0 && setup_bugs == 0) ? 0 : 1;
}
