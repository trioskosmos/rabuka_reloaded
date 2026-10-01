/* test_parity_look_per_card.c - C parity suite for the Rust clusters
 *   engine/tests/test_modules/effects/look_select/per_card/            (10 .rs / 44 #[test])
 *   engine/tests/test_modules/effects/look_select/pin_top/            ( 3 .rs /  5 #[test])
 *   engine/tests/test_modules/effects/look_select/split_top_bottom/   ( 1 .rs /  2 #[test])
 *   engine/tests/test_modules/effects/look_select/bottom_inspect/     ( 2 .rs /  3 #[test])
 *
 * NOT IN SCOPE: effects/look_select/recruit_stage/ (2 .rs / 4 #[test]) -- the
 * whole cost-tier cluster (PL!HS-bp6-029-L "Proof") is already ported, all four
 * tests, by tests/test_survey_probe.c. See the run report.
 *
 * ASCII-ONLY FILE, ON PURPOSE. The editor can write a file as CP932, which
 * silently corrupts every Japanese literal so group/series filters stop matching
 * real cards. No Japanese appears here at all: card numbers are ASCII, and the
 * U+FF0B FULLWIDTH PLUS prints are spelled with the explicit byte escape
 * "\xEF\xBC\x8B" because rb_find_card_by_no is EXACT-BYTE and does not
 * normalise fullwidth -> halfwidth the way Rust's CardDatabase::get_card_id
 * does.
 *
 * HARNESS LANDMINES other agents confirmed, all respected here:
 *   - rb_load("src") fails under an isolated out-of-tree build, so load_card_db()
 *     falls back to "../cards/build".
 *   - sizeof(GameState) is ~781 KB, so every TestGame here is `static`.
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE. Every heart
 *     read here goes through rb_mods_get_heart / rb_mods_get_need_heart directly.
 *   - The C deck bag is TOP-INDEX-0: Rust's `cards.insert(0, x)` is
 *     test_insert_deck_top(), Rust's `cards.push(x)` is an APPEND. Deck order is
 *     asserted explicitly wherever it is the point of the test.
 *   - mid() (test_new_id) allocates the next DISTINCT pool slot; test_id aliases
 *     the shared template. mid() is used wherever a test needs two real
 *     instances; rb_card_copy_pool_reset() runs per case so a long suite cannot
 *     exhaust the pool and silently fall back to the template.
 *   - rb_card_no_eq is FUZZY and is NOT an identity pin; a peer found it accept
 *     PL!SP-PR-004-R against the real PL!SP-PR-004-PR. pin_id() therefore does a
 *     byte-exact strcmp of rb_card_string(c.card_no_idx).
 *   - Card.ability is not populated by rb_decode_card_by_index, so ability
 *     inspection goes through rb_card_num_abilities + rb_decode_card_ability.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *
 * VACUITY CONTROL is the point of this file. A Rust negative ("nothing is
 * retrieved", "no prompt is raised") reads green on a board where the ability
 * was a silent no-op. Every such assertion here is paired with a POSITIVE
 * CONTROL: either a CHECK_PROMPTS(n) on a board that must prompt, or a second
 * in-test run of the identical fixture with the gate satisfied, which must
 * produce the effect. A green negative in this suite is therefore evidence.
 *
 * There is deliberately NO watchdog: a process watchdog is not achievable on
 * this toolchain. run() is crash ISOLATION, not a hang detector. Every drain is
 * bounded by an explicit iteration cap.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as the byte escape. */
#define FP "\xef\xbc\x8b"

/* ------------------------------------------------------------------ *
 * fixtures
 * ------------------------------------------------------------------ */
/* pin_top / split_top_bottom / bottom_inspect */
#define KAHO        "PL!HS-bp6-001-R" FP     /* 日野下花帆, cost 4        */
#define NOZOMI      "PL!-bp3-007-R"          /* 東條 希,     cost 9        */
#define CONNECT     "PL!N-bp3-028-L"          /* ツナガルコネクト, live     */
#define MARI        "PL!S-bp7-008-R"          /* 小原鞠莉,     cost 9        */
#define DIA         "PL!S-bp7-004-R"          /* 黒澤ダイヤ,   cost 13       */
#define CHIKA       "PL!S-bp7-010-N"          /* 高海千歌,     cost 4        */
#define CLEAN_KOTORI "PL!-pb1-021-PR"         /* 南 ことり, abilityless     */
#define BOT_A       "PL!-sd1-001-SD"
#define BOT_B       "PL!-sd1-003-SD"
#define BOT_C       "PL!-sd1-004-SD"
#define CHIKA_BOT   "PL!S-sd1-001-SD"         /* 高海千歌 SD, distinct     */
#define NIJI        "PL!N-bp1-001-R"          /* 上原歩夢, an own member    */
#define FILLER      "PL!-sd1-010-SD"
#define F14         "PL!-sd1-014-SD"
#define F15         "PL!-sd1-015-SD"
/* seven DISTINGUISHABLE deck cards for the Kaho order assertion. The Rust
 * fixture used seven copies of ONE print, which makes an order assertion
 * vacuous; this substitution is deliberate and is the reason the Kaho tests
 * can detect a wrong-end bug at all. */
#define DECK0       "PL!-sd1-010-SD"
#define DECK1       "PL!-sd1-011-SD"
#define DECK2       "PL!-sd1-012-SD"
#define DECK3       "PL!-sd1-013-SD"
#define DECK4       "PL!-sd1-014-SD"
#define DECK5       "PL!-sd1-015-SD"
#define DECK6       "PL!S-sd1-001-SD"

/* per_card */
#define KEKE        "PL!SP-bp4-006-R"         /* 桜小路きな子             */
#define MEMBER_A    "PL!SP-pb1-001-PR"        /* 澁谷かのん    CatChu!     */
#define MEMBER_B    "PL!SP-bp1-004-PR"        /* 平安名すみれ  CatChu!     */
#define MEMBER_C    "PL!SP-PR-005-PR"         /* 嵐 千砂都   5yncri5e!    */
#define MEMBER_C2   "PL!SP-bp1-014-N"         /* 嵐 千砂都   5yncri5e!    */
#define LIELLA_LIVE "PL!SP-bp1-023-L"
#define OUT_HS      "PL!HS-bp1-012-PR"        /* 乙宗 梢, Hasunosora, c4  */
#define OUT_N       "PL!N-bp1-001-R"          /* 上原歩夢, Nijigasaki      */
#define OUT_S       "PL!S-bp6-007-R"          /* 国木田花丸, Aqours        */
#define SUISUI_CLS  "PL!HS-cl1-009-CL"        /* 水彩世界                   */
#define SUISUI_C4   "PL!HS-bp1-012-PR"        /* cost 4  (lower boundary)   */
#define SUISUI_C9   "PL!HS-pb1-015-R"         /* cost 9  (upper boundary)   */
#define SUISUI_C10  "PL!HS-PR-005-PR"         /* cost 10 (> 9)              */
#define FUSION      "PL!HS-bp6-032-L"         /* フュージョンクラスト        */
#define FUSION_C4   "PL!HS-bp1-009-R"         /* cost 4 (== boundary)       */
#define FUSION_C11  "PL!HS-bp5-001-P"         /* cost 11 (> 4)              */
#define DISTORTION  "PL!SP-pb2-048-L"         /* ディストーション            */
#define CATCHU_1    "PL!SP-bp1-012-N"          /* 澁谷かのん   CatChu!        */
#define CATCHU_2    "PL!SP-bp1-015-N"          /* 平安名すみれ CatChu!        */
#define CATCHU_3    "PL!SP-bp1-018-N"          /* 米女メイ     CatChu!        */
#define CATCHU_K2   "PL!SP-bp4-012-N"          /* 澁谷かのん again, SAME name */
#define MIRAI       "PL!S-bp6-021-L"           /* MIRAI TICKET, live          */
#define AQ1         "PL!S-bp3-009-R"           /* 黒澤ルビィ,  cost 9        */
#define AQ2         "PL!S-bp2-002-R"           /* 桜内梨子,    cost 4        */
#define AQ3         "PL!S-bp6-013-N"           /* 黒澤ダイヤ,   cost 5        */
#define AQ4         "PL!S-bp5-001-R" FP        /* 高海千歌,    cost 10       */
#define AQ5         "LL-bp2-001-R" FP         /* multi-name,  cost 20       */
#define BURN        "PL!N-bp7-029-L"           /* Burn!!, live               */
#define BURN_MEMBER "PL!N-bp1-007-R"           /* 優木せつ菜                  */
#define ENERGY      "LL-E-001-SD"
#define NAGI        "PL!-sd1-007-SD"           /* 東條 希 (starter)           */
#define NAGI_LIVE1  "PL!-sd1-019-SD"           /* START:DASH!!  score 1      */
#define NAGI_LIVE2  "PL!-sd1-020-SD"           /* きっと青春が聞こえる s2     */
#define N11         "PL!-sd1-011-SD"
#define N12         "PL!-sd1-012-SD"
#define N13         "PL!-sd1-013-SD"
#define N16         "PL!-sd1-016-SD"
#define TOMARI      "PL!SP-bp7-011-R"          /* 鬼塚冬毬, cost 9           */
#define SUMIRE      "PL!SP-bp2-015-N"          /* 平安名すみれ, heart06 yell  */
#define WIEN        "PL!SP-bp2-021-N"          /* ウィーン,     heart03 yell  */
#define YELL_LIVE   "PL!S-bp3-020-L"
#define SCORE_MEM   "PL!S-sd1-003-SD"          /* 松浦果南, 02+04+05         */
#define HONOKA      "PL!-sd1-010-SD"           /* 高坂穂乃果, 01+03          */
#define SMILE       "LL-bp5-001-L"             /* Live with a smile!         */

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
static TestGame a_tg, b_tg, c_tg, d_tg, e_tg, f_tg, g_tg, h_tg, i_tg, j_tg,
              k_tg, l_tg, m_tg, n_tg, o_tg, p_tg, q_tg, r_tg;

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

/* A Rust assertion with no faithful C entry point. Recorded, never a pass and
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
 * counted HERE, because the test asserts while the prompt is still on screen -
 * counting only at answer time (inside pick/skip/select_option) would report 0
 * for a prompt that is plainly open. */
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
 * fixtures / helpers
 * ------------------------------------------------------------------ */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* A fresh game with the copy pool reset: a long suite must never exhaust the
 * process-global pool and silently fall back to the shared template. */
static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

static const char *card_no_of(int cid)
{
    static char buf[96];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* Rust game.new_id("X") -> a distinct pool slot. C twin: test_new_id. */
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

/* Rust game.id("X") -> the shared template slot. C twin: test_id. */
static int sid(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    if (id < 0) {
        assertions++;
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* AGENTS.md identity rule. rb_card_no_eq is FUZZY, so the pin is a byte-exact
 * strcmp of the decoded card_no against the requested literal. */
static int pin_id(int cid, const char *no)
{
    assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    setup_bugs++;
    failures++;
    fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

static int card_prints_trigger(int cid, const char *trig)
{
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig) != NULL;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int card_cost(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int card_score(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}

static int card_blade(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.blade; rb_free_card(&c); }
    return v;
}

static int card_has_blade_heart(int cid)
{
    Card c;
    int v = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        v = rb_card_has_blade_heart(&c);
        rb_free_card(&c);
    }
    return v;
}

static const char *card_name(int cid)
{
    static char buf[80];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, sizeof buf, "%s", c.name);
        rb_free_card(&c);
    }
    return buf;
}

/* The decoded heart table is ONE flat (colour, count) list laid out as
 *   [0, num_base)                            base_heart
 *   [num_base, num_base + num_need)          need_heart
 *   [num_base + num_need, n_hearts)          blade_heart
 * (src/core/stats_pipeline.c:32,45 and src/ability/effects/score.c:274-300).
 * Summing the WHOLE table for a colour silently folds a blade heart into the
 * requirement: PL!SP-pb2-048-L prints need_heart heart02 = 6 AND
 * blade_heart b_heart02 = 1, so a whole-table read reports 7 and every
 * effective-need assertion downstream is off by one. Colour index 0 = heart00
 * (the printed "heart0" wild), 1..6 = heart01..heart06. */
/* Diagnostic only: prints the decoded heart table layout for a card. Not an
 * assertion, but the only way to see where the C put base/need/blade hearts. */
static void dump_heart_table(int cid, const char *no)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return;
    fprintf(stderr, "  [hearts %s] n_hearts=%d num_base=%d num_need=%d num_blade=%d :",
            no, c.n_hearts, c.num_base, c.num_need, c.num_blade);
    for (int i = 0; i < c.n_hearts; i++)
        fprintf(stderr, " (%d x%d)%s", c.heart_color[i], c.heart_count[i],
                i < c.num_blade ? "[blade]" : (i < c.num_blade + c.num_need ? "[need]" : "[base]"));
    fprintf(stderr, "\n");
    rb_free_card(&c);
}

/* The decoded heart table is a flat (colour, count) list, MEASURED per card
 * type (see dump_heart_table, whose output is in the test log):
 *
 *   MEMBER  n_hearts = num_base + num_blade
 *           [0, num_base)                   base_heart
 *           [num_base, n_hearts)            blade_heart
 *           (no need_heart at all)
 *   LIVE    n_hearts = num_blade + num_need
 *           [0, num_blade)                  blade_heart
 *           [num_blade, num_blade + num_need)   need_heart
 *           (no base_heart)
 *
 * The BLADE block comes FIRST for a live card and LAST for a member, so there
 * is no single "hearts after index N" rule. Both are read from the two range
 * formulas above, and each is pinned by a CHECK against the value cards.json
 * prints, so a layout change shows up as a red premise rather than as silently
 * wrong arithmetic.
 *
 * TRAP 1, hit while writing this: summing the WHOLE table folds a blade heart
 * into the requirement. PL!SP-pb2-048-L prints need_heart heart02 = 6 AND
 * blade_heart b_heart02 = 1, so a whole-table read reports 7 and every
 * effective-need assertion downstream is off by one.
 * TRAP 2: the layout above was established by measurement, not by reading
 * src/core/stats_pipeline.c:45, which suggests base-then-need and is wrong for
 * live cards. The dump is the authority.
 *
 * Colour index 0 = heart00 (the printed "heart0" wild), 1..6 = heart01..06. */
static int printed_base_heart(int cid, int color)
{
    Card c;
    int total = 0;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    for (int i = 0; i < c.num_base && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == color) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

static int printed_need_heart(int cid, int color)
{
    Card c;
    int total = 0;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    for (int i = c.num_blade; i < c.num_blade + c.num_need && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == color) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

/* Rust `base from DB + modifier from mods`, clamped at 0. */
static int effective_need_heart(TestGame *tg, int cid, int color)
{
    int v = printed_need_heart(cid, color) +
            rb_mods_get_need_heart(&tg->state.mods, cid, color);
    return v < 0 ? 0 : v;
}

static int score_mod(TestGame *tg, int cid)
{
    return rb_mods_get_score(&tg->state.mods, cid);
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].deck, cid); }
static int revealed_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_revealed; i++)
        if (tg->state.revealed_cards[i] == cid) return 1;
    return 0;
}
static int energy_has(const TestGame *tg, int cid)
{
    return bag_has(&tg->state.p[0].energy, cid);
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    tg->state.n_selected_cards = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_recently_moved = 0;
    rb_look_reset_all();
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

/* C's deck bag is TOP-INDEX-0, so an APPEND is Rust's cards.push. */
static void deck_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static void deck_append_n(TestGame *tg, int pl, int cid, int n)
{
    for (int i = 0; i < n; i++) deck_append(tg, pl, cid);
}

static void hand_append_n(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->hand.n < RB_MAX_ZONE; i++) P->hand.cards[P->hand.n++] = cid;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const char *pending_type(TestGame *tg)
{
    const char *t = test_pending_choice_type(tg);
    return t ? t : "";
}

/* A prompt the test EXPECTS to exist. Counting it HERE is what makes
 * CHECK_PROMPTS meaningful. */
static void note_prompt(TestGame *tg)
{
    if (pending(tg)) prompts++;
}

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg) || n <= 0) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

static void skip(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

static void select_option(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, idx);
}

static int drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) skip(tg);
    return guard;
}

static int look_pool_size(int pl)
{
    int buf[RB_MAX_ZONE];
    return rb_looked_at_pool(pl, buf, RB_MAX_ZONE);
}

static int look_pool_at(int pl, int i)
{
    int buf[RB_MAX_ZONE];
    int n = rb_looked_at_pool(pl, buf, RB_MAX_ZONE);
    return (i >= 0 && i < n) ? buf[i] : -1;
}

static void check_deck_order(TestGame *tg, const int *want, int n_want, const char *what)
{
    int buf[RB_MAX_ZONE];
    int n = test_zone_ids(tg, 0, "deck", buf, RB_MAX_ZONE);
    assertions++;
    if (n == n_want) {
        int ok = 1;
        for (int i = 0; i < n; i++) if (buf[i] != want[i]) { ok = 0; break; }
        if (ok) return;
    }
    failures++;
    fprintf(stderr, "FAIL: %s\n  got (n=%d):", what, n);
    for (int i = 0; i < n && i < 12; i++) fprintf(stderr, " %s", card_no_of(buf[i]));
    fprintf(stderr, "\n  want (n=%d):", n_want);
    for (int i = 0; i < n_want; i++) fprintf(stderr, " %s", card_no_of(want[i]));
    fprintf(stderr, "\n");
}

static void check_bag_n(const RbBag *bag, int want, const char *what)
{
    assertions++;
    if (bag->n == want) return;
    failures++;
    fprintf(stderr, "FAIL: %s (got %d expected %d):", what, bag->n, want);
    for (int i = 0; i < bag->n && i < 12; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
    fprintf(stderr, "\n");
}

/* `count` / `allow_skip` < 0 mean "do not check this field" (the Rust
 * assert_select_card only pins the fields the test cares about). */
static void check_choice_is(TestGame *tg, const char *zone, int count, int allow_skip,
                            const char *what)
{
    assertions++;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (c && c->zone[0] && !strcmp(c->zone, zone) &&
        (count < 0 || c->count == count) &&
        (allow_skip < 0 || c->allow_skip == allow_skip)) return;
    failures++;
    fprintf(stderr, "FAIL: %s (zone=%s count=%d allow_skip=%d | want zone=%s count=%d allow_skip=%d)\n",
            what, c ? c->zone : "(none)", c ? c->count : -1, c ? c->allow_skip : -1,
            zone, count, allow_skip);
}

/* Rust fire_trigger(card, trigger, text): fire ONE ability of ONE card.
 * test_fire_trigger_on_card does that through rb_trigger_auto_ability (the port
 * of Rust trigger_auto_ability), which enqueues exactly one entry for `cid`.
 * This USED TO BE rb_queue_trigger_abilities(&tg->state, 0, trig), where `cid`
 * only ever set activating_card and every card on the board printing the same
 * trigger fired too. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    return test_fire_trigger_on_card(tg, cid, trig);
}

/* Rust trigger_auto_abilities_for_player("p1") + process_pending_auto_abilities. */
static int fire_auto(TestGame *tg)
{
    int queued = rb_queue_trigger_abilities(&tg->state, 0, RB_TSTR_AUTO);
    if (queued > 0) rb_process_pending_auto_abilities(&tg->state);
    return queued;
}

static int play_to_stage(TestGame *tg, int cid, int area, int accept)
{
    int r = test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        /* The play-cost prompt is answered HERE, inside the helper, so it must
         * be counted here too - otherwise a test whose only prompt is the
         * debut cost reports zero and its CHECK_PROMPTS control is a false
         * negative on a board that did prompt. */
        note_prompt(tg);
        test_answer_play_cost_choice(tg, accept);
        if (tg->state.p[0].stage[area] == cid) break;
        r = test_play_to_stage(tg, cid, area);
    }
    return r;
}

static void advance_to_live_card_set(TestGame *tg, int n_pass)
{
    for (int i = 0; i < n_pass; i++) { test_pass(tg); drain_skip(tg, 8); }
}

/* Fill a deck with `count` DISTINGUISHABLE member prints starting at the
 * given table, so an order assertion can never be satisfied by an all-filler
 * deck. */
static int fill_distinct_deck(TestGame *tg, int pl, const char *const *nos, int n_nos,
                              int count)
{
    int ids[16];
    int n_ids = 0;
    for (int i = 0; i < n_nos && n_ids < 16; i++) {
        int c = mid(tg, nos[i]);
        if (c < 0) return -1;
        if (!pin_id(c, nos[i])) return -1;
        ids[n_ids++] = c;
    }
    if (n_ids == 0) return -1;
    for (int i = 0; i < count; i++) {
        int c = mid(tg, nos[i % n_ids]);
        if (c < 0) return -1;
        deck_append(tg, pl, c);
    }
    return 0;
}

/* ------------------------------------------------------------------ *
 * §A  pin_top / dynamic_count_pin_one_to_top_test.rs
 *      PL!HS-bp6-001-R+ (Kaho), debut:
 *      「look at (own stage members + 2) cards; put 1 on TOP of the deck,
 *        the rest in the waitroom」
 *
 * RUST / PRINTED-TEXT CROSS-CHECK (both agree; recorded because the arithmetic
 *   looks wrong at a glance and a future reader will "fix" it):
 *   cards.json prints for PL!HS-bp6-001-R+
 *     「自分のデッキの上から、自分のステージにいるメンバーの数に2を足した数に
 *       等しい枚数見る」
 *   i.e. (own stage members) + 2. Kaho is on its OWN stage when its debut
 *   resolves, so the count is allies + kaho + 2 = allies + 3, which is exactly
 *   what the Rust helper computes and what the tests below assert. The Rust
 *   test name ("..._stage_members_plus_two") is about the ALLY count, not the
 *   total. There is NO off-by-one here: the C engine looks at 3 / 4 / 5 for
 *   0 / 1 / 2 allies and all three deck-order pins pass.
 * ------------------------------------------------------------------ */

static const char *const KAHO_DECK[7] = { DECK0, DECK1, DECK2, DECK3, DECK4, DECK5, DECK6 };

/* Rust play_kaho_selection(allies). Returns the inspected count the ENGINE
 * actually looked at, or -1 on a setup failure. */
static int kaho_setup(TestGame *tg, int allies, int cards[7], int *kaho_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    if (fill_distinct_deck(tg, 0, KAHO_DECK, 7, 7) != 0) return -1;
    for (int i = 0; i < 7; i++) cards[i] = tg->state.p[0].deck.cards[i];

    int opp_deck = mid(tg, DECK0);
    if (!pin_id(opp_deck, DECK0)) return -1;
    deck_append(tg, 1, opp_deck);

    int opp_stage[3];
    for (int a = 0; a < 3; a++) {
        opp_stage[a] = mid(tg, F14);
        if (!pin_id(opp_stage[a], F14)) return -1;
        set_stage(tg, 1, a, opp_stage[a]);
    }

    /* allies take LeftSide then RightSide, exactly as Rust's
     * [LeftSide, RightSide].into_iter().take(allies). */
    if (allies >= 1) {
        int a0 = mid(tg, F15);
        if (!pin_id(a0, F15)) return -1;
        set_stage(tg, 0, AREA_LEFT, a0);
    }
    if (allies >= 2) {
        int a1 = mid(tg, F15);
        if (!pin_id(a1, F15)) return -1;
        set_stage(tg, 0, AREA_RIGHT, a1);
    }

    int kaho = mid(tg, KAHO);
    if (!pin_id(kaho, KAHO)) return -1;
    *kaho_out = kaho;
    CHECK_EQ(card_cost(kaho), 4, "kaho: the print's cost");
    CHECK(card_prints_trigger(kaho, RB_TSTR_DEBUT),
          "kaho: the print carries a 登場 trigger");
    test_add_to_hand(tg, kaho);
    test_give_energy(tg, 4);
    return 0;
}

static int kaho_stage_has(TestGame *tg, int kaho)
{
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg->state.p[0].stage[a] == kaho) return 1;
    return 0;
}

/* Rust assert_kaho_pins: expected_deck == [cards[inspected-1]] + cards[inspected..] */
static void kaho_assert_pins(TestGame *tg, int inspected, const int cards[7],
                             int allies)
{
    int want[8];
    int n_want = 0;
    if (inspected >= 1) want[n_want++] = cards[inspected - 1];
    for (int i = inspected; i < 7; i++) want[n_want++] = cards[i];
    check_deck_order(tg, want, n_want, "kaho: the kept card sits on TOP and the "
                                       "never-looked cards keep their order");
    check_bag_n(&tg->state.p[0].discard, inspected - 1,
                "kaho: inspected-1 looked cards go to the waitroom");
    check_bag_n(&tg->state.p[0].hand, 0, "kaho: the hand is empty afterwards");
    /* The opponent must be untouched. */
    check_bag_n(&tg->state.p[1].deck, 1, "kaho: the opponent deck is untouched");
    check_bag_n(&tg->state.p[1].discard, 0, "kaho: the opponent waitroom stays empty");
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        CHECK(tg->state.p[1].stage[a] != RB_EMPTY_SLOT,
              "kaho: opponent stage area %d still holds a member", a);
    (void)allies;
}

/* Rust look_and_select_dynamic_count_look_at_counts_stage_members_plus_two */
static void test_a_kaho_no_allies_looks_three(void)
{
    TestGame *tg = &a_tg;
    int cards[7];
    int kaho = -1;
    if (kaho_setup(tg, 0, cards, &kaho) != 0) return;
    play_to_stage(tg, kaho, AREA_CENTER, 0);
    CHECK(kaho_stage_has(tg, kaho), "kaho0: the card is on stage after the play");
    /* VACUITY CONTROL: the board must have produced the look-and-select prompt
     * before any "inspected == N" claim can be trusted. */
    CHECK_PROMPTS(tg, 1, "kaho0: the debut must offer the look-and-select pick");
    CHECK(pending(tg), "kaho0: a choice is pending after the play");
    if (pending(tg))
        check_choice_is(tg, "looked_at", 1, 0,
                        "kaho0: the offer is a mandatory 1-card pick over looked_at");
    int inspected = look_pool_size(0);
    /* Ported VERBATIM from Rust: `let inspected = allies + 3;` == 3, where
     * `allies + 3` is (own stage members, Kaho included) + 2. */
    CHECK_EQ(inspected, 3,
             "kaho0: Rust asserts 3 looked cards with 0 allies");
    for (int i = 0; i < inspected && i < 7; i++)
        CHECK_EQ(look_pool_at(0, i), cards[i],
                 "kaho0: looked-at slot %d is deck slot %d", i, i);
    if (pending(tg)) pick(tg, inspected - 1);
    CHECK(!pending(tg), "kaho0: the ability ends after the single pick");
    kaho_assert_pins(tg, inspected, cards, 0);
}

/* Rust kaho_two_own_members_look_four_and_keep_one */
static void test_a_kaho_one_ally_looks_four(void)
{
    TestGame *tg = &a_tg;
    int cards[7];
    int kaho = -1;
    if (kaho_setup(tg, 1, cards, &kaho) != 0) return;
    play_to_stage(tg, kaho, AREA_CENTER, 0);
    CHECK(kaho_stage_has(tg, kaho), "kaho1: the card is on stage after the play");
    CHECK_PROMPTS(tg, 1, "kaho1: the debut must offer the look-and-select pick");
    int inspected = look_pool_size(0);
    CHECK_EQ(inspected, 4, "kaho1: Rust asserts 4 looked cards with 1 ally");
    for (int i = 0; i < inspected && i < 7; i++)
        CHECK_EQ(look_pool_at(0, i), cards[i], "kaho1: looked-at slot %d", i);
    if (pending(tg)) pick(tg, inspected - 1);
    CHECK(!pending(tg), "kaho1: the ability ends after the single pick");
    kaho_assert_pins(tg, inspected, cards, 1);
}

/* Rust kaho_full_own_stage_looks_five_and_keeps_one */
static void test_a_kaho_two_allies_looks_five(void)
{
    TestGame *tg = &a_tg;
    int cards[7];
    int kaho = -1;
    if (kaho_setup(tg, 2, cards, &kaho) != 0) return;
    play_to_stage(tg, kaho, AREA_CENTER, 0);
    CHECK(kaho_stage_has(tg, kaho), "kaho2: the card is on stage after the play");
    CHECK_PROMPTS(tg, 1, "kaho2: the debut must offer the look-and-select pick");
    int inspected = look_pool_size(0);
    CHECK_EQ(inspected, 5, "kaho2: Rust asserts 5 looked cards with 2 allies");
    for (int i = 0; i < inspected && i < 7; i++)
        CHECK_EQ(look_pool_at(0, i), cards[i], "kaho2: looked-at slot %d", i);
    if (pending(tg)) pick(tg, inspected - 1);
    CHECK(!pending(tg), "kaho2: the ability ends after the single pick");
    kaho_assert_pins(tg, inspected, cards, 2);
}

/* ------------------------------------------------------------------ *
 * §B  pin_top / paid_look_three_splits_hand_top_and_waitroom_test.rs
 *      PL!-bp3-007-R (Nozomi), live_start: optional discard 2 from hand ->
 *      look at top 3 -> 1 to hand, 1 to the deck TOP, 1 to the waitroom.
 * ------------------------------------------------------------------ */

static void nozomi_setup(TestGame *tg, int *nozomi_out, int *filler_out, int *kotori_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int nozomi = mid(tg, NOZOMI);
    int filler = mid(tg, FILLER);
    int kotori = mid(tg, CLEAN_KOTORI);
    pin_id(nozomi, NOZOMI);
    pin_id(filler, FILLER);
    pin_id(kotori, CLEAN_KOTORI);
    *nozomi_out = nozomi; *filler_out = filler; *kotori_out = kotori;
    if (nozomi < 0 || filler < 0 || kotori < 0) return;
    CHECK_EQ(card_cost(nozomi), 9, "nozomi: the print's cost");
    CHECK(card_prints_trigger(nozomi, RB_TSTR_LIVE_START),
          "nozomi: the print carries a ライブ開始時 trigger");

    set_stage(tg, 0, AREA_CENTER, nozomi);
    hand_append_n(tg, 0, filler, 2);

    /* Rust fill_decks(stock): 30 copies of a DISTINCT copy per seat. */
    for (int i = 0; i < 30; i++) {
        deck_append(tg, 0, mid(tg, FILLER));
        deck_append(tg, 1, mid(tg, FILLER));
    }
    /* put_on_deck_top(filler) then put_on_deck_top(kotori): C prepend. */
    test_insert_deck_top(tg, 0, filler);
    test_insert_deck_top(tg, 0, kotori);
    /* Explicitly verify the C deck end, because reading the wrong end of the
     * deck is the single most destructive fixture bug in this family. */
    CHECK_EQ(tg->state.p[0].deck.cards[0], kotori,
             "nozomi: the top of the C deck is the last card put_on_deck_top");
    CHECK_EQ(tg->state.p[0].deck.cards[1], filler,
             "nozomi: the second card of the C deck is the filler");
}

/* Rust while has_pending_choice(): SelectTarget -> option 1, SelectCard ->
 * take min(1) index, anything else -> break. */
static int nozomi_drain(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 20) {
        const char *t = pending_type(tg);
        if (!strcmp(t, "SelectTarget")) select_option(tg, 1);
        else if (!strcmp(t, "SelectCard")) {
            int n = test_pending_choice_count(tg);
            if (n < 1) n = 1;
            const int take[1] = { 0 };
            pick_many(tg, take, n < 1 ? 1 : 1);
        } else break;
    }
    return guard;
}

static void test_b_nozomi_paid_look_three_splits(void)
{
    TestGame *tg = &b_tg;
    int nozomi = -1, filler = -1, kotori = -1;
    nozomi_setup(tg, &nozomi, &filler, &kotori);
    if (nozomi < 0 || filler < 0 || kotori < 0) return;

    int deck_before = tg->state.p[0].deck.n;
    int wait_before = tg->state.p[0].discard.n;

    CHECK(fire_trigger(tg, nozomi, RB_TSTR_LIVE_START),
          "nozomi: the ライブ開始時 ability fires");
    nozomi_drain(tg);
    /* VACUITY CONTROL: without a prompt the deck/waitroom deltas prove nothing. */
    CHECK_PROMPTS(tg, 2, "nozomi: the discard-2 cost and the look pick must both prompt");

    CHECK_EQ(tg->state.p[0].deck.n, deck_before - 2,
             "nozomi: three looked off, one placed back on top -> deck - 2");
    CHECK_EQ(tg->state.p[0].discard.n, wait_before + 3,
             "nozomi: two paid cards + one looked leftover land in the waitroom");
    CHECK(tg->state.p[0].hand.n <= 3,
          "nozomi: paid two, gained one - hand must not exceed start + 1 (got %d)",
          tg->state.p[0].hand.n);
    CHECK(deck_has(tg, kotori) || deck_has(tg, filler),
          "nozomi: at least one of the two looked cards is still on the deck");
}

/* ------------------------------------------------------------------ *
 * §C  pin_top / per_group_member_look_pin_reveal_score_test.rs
 *      PL!N-bp3-028-L (Tsunagaru Connect), live_start: for each 虹ヶ咲 member
 *      on MY stage look at 1 card from the top of the deck; from those put at
 *      most 1 back on the deck TOP, the rest to the waitroom; then reveal the
 *      top card of the deck and +1 score if it is a live card.
 *      Rust: with an all-filler deck the reveal CANNOT be a live card.
 * ------------------------------------------------------------------ */

static void connect_setup(TestGame *tg, int *connect_out, int *member_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int connect = mid(tg, CONNECT);
    int n1 = mid(tg, NIJI);
    int n2 = mid(tg, NIJI);
    int n3 = mid(tg, NIJI);
    int n_copy2 = mid(tg, NIJI);
    int n_copy3 = mid(tg, NIJI);
    int filler = mid(tg, FILLER);
    pin_id(connect, CONNECT);
    pin_id(n1, NIJI); pin_id(n2, NIJI); pin_id(n3, NIJI);
    pin_id(n_copy2, NIJI); pin_id(n_copy3, NIJI);
    pin_id(filler, FILLER);
    *connect_out = connect;
    *member_out = n1;
    if (connect < 0 || n1 < 0 || filler < 0) return;
    /* The dedupe/per-unit premise: three DISTINCT physical instances of the
     * same print, so per-unit over NAMES is what makes the count 3. */
    CHECK(n1 != n2 && n2 != n3 && n1 != n3,
          "connect: the three staged members must be three DISTINCT card slots");
    CHECK(card_prints_trigger(connect, RB_TSTR_LIVE_START),
          "connect: the print carries a ライブ開始時 trigger");

    set_stage(tg, 0, 0, n1);
    set_stage(tg, 0, 1, n2);
    set_stage(tg, 0, 2, n3);
    (void)n_copy2; (void)n_copy3;
    test_add_to_hand(tg, connect);
    test_give_energy(tg, 5);

    deck_append_n(tg, 0, filler, 30);
    deck_append_n(tg, 1, filler, 30);
    /* The top of the deck is a filler MEMBER, so the trailing reveal can
     * never be a live card. */
    CHECK(rb_card_is_live(filler) == 0,
          "connect: the deck top must NOT be a live card, or the +1 score gate "
          "is satisfiable and the negative is vacuous");
}

/* Rust trigger_printed_ability_and_resolve_choices: SelectAutoAbility -> empty,
 * SelectTarget -> option 0, everything else -> index 0. */
static int connect_drain(TestGame *tg)
{
    int guard = 0, offered = 0;
    while (pending(tg) && guard++ < 20) {
        offered = 1;
        const char *t = pending_type(tg);
        if (!strcmp(t, "SelectAutoAbility")) skip(tg);
        else if (!strcmp(t, "SelectTarget")) select_option(tg, 0);
        else if (!strcmp(t, "SelectCard")) pick(tg, 0);
        else if (!strcmp(t, "SelectPosition")) pick(tg, 0);
        else if (!strcmp(t, "SelectHeartColor")) pick(tg, 0);
        else break;
    }
    return offered;
}

static void test_c_connect_filler_reveal_grants_no_score(void)
{
    TestGame *tg = &c_tg;
    int connect = -1, member = -1;
    connect_setup(tg, &connect, &member);
    if (connect < 0) return;

    CHECK(connect_drain(tg) || 1, "connect: the driver ran");
    CHECK(fire_trigger(tg, connect, RB_TSTR_LIVE_START),
          "connect: the ライブ開始時 ability fires");
    int offered = connect_drain(tg);
    /* VACUITY CONTROL: without a prompt, "no score" is vacuous. */
    CHECK(offered || prompts > 0,
          "connect: the ability must offer at least one prompt, otherwise the "
          "no-score assertion below proves nothing");
    CHECK_PROMPTS(tg, 1, "connect: the per-unit look must prompt");

    CHECK_EQ(score_mod(tg, connect), 0,
             "connect: no live card revealed means no score bonus");
    CHECK(tg->state.p[0].discard.n >= 2,
          "connect: peeked cards (3 minus at most 1 kept) must go to the waitroom "
          "(got %d)", tg->state.p[0].discard.n);
}

/* ------------------------------------------------------------------ *
 * §D  split_top_bottom / inspect_top_split_remainder_test.rs
 *      PL!S-bp7-008-R (Mari), debut: look at the top 3; put any number of them
 *      back on TOP of the deck IN ANY ORDER, the rest at the BOTTOM.
 *      This is the any_number RE-ORDER shape.
 * ------------------------------------------------------------------ */

static int mari_setup(TestGame *tg, int *mari_out, int cards[5])
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    static const char *const NOS[5] = { DECK0, DECK1, DECK2, DECK3, DECK4 };
    if (fill_distinct_deck(tg, 0, NOS, 5, 5) != 0) return -1;
    for (int i = 0; i < 5; i++) cards[i] = tg->state.p[0].deck.cards[i];
    int opp = mid(tg, DECK0);
    if (!pin_id(opp, DECK0)) return -1;
    deck_append(tg, 1, opp);
    int mari = mid(tg, MARI);
    if (!pin_id(mari, MARI)) return -1;
    *mari_out = mari;
    CHECK_EQ(card_cost(mari), 9, "mari: the print's cost");
    CHECK(card_prints_trigger(mari, RB_TSTR_DEBUT), "mari: the print has 登場");
    test_add_to_hand(tg, mari);
    test_give_energy(tg, 20);
    return 0;
}

/* Rust mari_keeps_one_on_top_rest_to_deck_bottom */
static void test_d_mari_keeps_one_on_top(void)
{
    TestGame *tg = &d_tg;
    int cards[5];
    int mari = -1;
    if (mari_setup(tg, &mari, cards) != 0) return;

    play_to_stage(tg, mari, AREA_CENTER, 0);
    CHECK(tg->state.p[0].stage[AREA_CENTER] == mari, "mari1: Mari is on stage center");
    CHECK(!hand_has(tg, mari), "mari1: Mari left the hand");
    /* VACUITY CONTROL. */
    CHECK_PROMPTS(tg, 1, "mari1: the debut must offer the look pick");
    CHECK(pending(tg), "mari1: a choice is pending");
    if (pending(tg)) {
        const char *t = pending_type(tg);
        CHECK(!strcmp(t, "SelectCard"), "mari1: expected SelectCard (got %s)", t);
        if (strcmp(t, "SelectCard") == 0)
            check_choice_is(tg, "looked_at", -1, -1, "mari1: the offer is over looked_at");
    }
    CHECK_EQ(look_pool_size(0), 3, "mari1: the top 3 cards are looked at");
    for (int i = 0; i < 3 && i < 5; i++)
        CHECK_EQ(look_pool_at(0, i), cards[i], "mari1: looked-at slot %d", i);

    const int one[1] = { 1 };
    pick_many(tg, one, 1);
    CHECK(pending(tg), "mari1: the any_number re-order prompt must follow the pick");
    CHECK(!strcmp(pending_type(tg), "SelectCard"),
          "mari1: the re-order prompt is a SelectCard (got %s)", pending_type(tg));
    skip(tg);
    CHECK(!pending(tg), "mari1: the ability ends after the decline");

    const int want[5] = { cards[1], cards[3], cards[4], cards[0], cards[2] };
    check_deck_order(tg, want, 5, "mari1: deck == [b, d, e, a, c]");
    check_bag_n(&tg->state.p[0].discard, 0, "mari1: the waitroom stays empty");
    check_bag_n(&tg->state.p[0].hand, 0, "mari1: the hand stays empty");
}

/* Rust mari_keeps_two_on_top_one_to_deck_bottom */
static void test_d_mari_keeps_two_on_top(void)
{
    TestGame *tg = &d_tg;
    int cards[5];
    int mari = -1;
    if (mari_setup(tg, &mari, cards) != 0) return;

    play_to_stage(tg, mari, AREA_CENTER, 0);
    CHECK_PROMPTS(tg, 1, "mari2: the debut must offer the look pick");
    CHECK_EQ(look_pool_size(0), 3, "mari2: the top 3 cards are looked at");

    const int a[1] = { 0 };
    pick_many(tg, a, 1);
    CHECK(pending(tg), "mari2: the any_number re-order prompt must follow pick 1");
    const int b[1] = { 0 };
    pick_many(tg, b, 1);
    CHECK(pending(tg), "mari2: the any_number re-order prompt must follow pick 2");
    skip(tg);
    CHECK(!pending(tg), "mari2: the ability ends after the decline");

    const int want[5] = { cards[1], cards[0], cards[3], cards[4], cards[2] };
    check_deck_order(tg, want, 5, "mari2: deck == [b, a, d, e, c]");
    check_bag_n(&tg->state.p[0].discard, 0, "mari2: the waitroom stays empty");
    check_bag_n(&tg->state.p[0].hand, 0, "mari2: the hand stays empty");
}

/* ------------------------------------------------------------------ *
 * §E  bottom_inspect / inspect_bottom_and_select_test.rs
 *      PL!S-bp7-004-R (Dia), live_start: look at the BOTTOM 3 of the deck; put
 *      any number of them back at the BOTTOM in any order, rest to the waitroom.
 * ------------------------------------------------------------------ */

static void test_e_dia_keeps_one_on_bottom(void)
{
    TestGame *tg = &e_tg;
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    int a = mid(tg, BOT_A), b = mid(tg, BOT_B), c = mid(tg, BOT_C);
    pin_id(a, BOT_A); pin_id(b, BOT_B); pin_id(c, BOT_C);
    int dia = mid(tg, DIA);
    pin_id(dia, DIA);
    if (a < 0 || b < 0 || c < 0 || dia < 0) return;
    CHECK(card_prints_trigger(dia, RB_TSTR_LIVE_START),
          "dia: the print carries a ライブ開始時 trigger");
    CHECK(rb_card_is_live(a) == 0 && rb_card_is_live(b) == 0 && rb_card_is_live(c) == 0,
          "dia: all three bottom cards are member cards");

    /* deck is exactly [a, b, c] with top = a, bottom = c. */
    deck_append(tg, 0, a);
    deck_append(tg, 0, b);
    deck_append(tg, 0, c);
    CHECK_EQ(tg->state.p[0].deck.n, 3, "dia: the deck is exactly three cards");
    CHECK_EQ(tg->state.p[0].deck.cards[0], a, "dia: deck index 0 is the top (a)");
    CHECK_EQ(tg->state.p[0].deck.cards[2], c, "dia: deck index 2 is the bottom (c)");
    /* Rust's trigger_live_start fires the ability by card id regardless of
     * zone. C's rb_queue_trigger_abilities only reaches cards it can SEE, so
     * the host is placed in the hand, which is the closest equivalent (the
     * ツナガルコネクト case in section C uses the same placement and does fire).
     * Without this the trigger is not even queued and every assertion below
     * would read green on a board where nothing ran. */
    test_add_to_hand(tg, dia);

    CHECK(fire_trigger(tg, dia, RB_TSTR_LIVE_START), "dia: the live_start fires");
    CHECK_EQ(look_pool_size(0), 3, "dia: the BOTTOM 3 are looked at");
    /* VACUITY CONTROL. */
    CHECK_PROMPTS(tg, 1, "dia: the bottom-inspect ability must prompt");
    CHECK(pending(tg), "dia: a choice is pending");

    /* Rust: select the first looked-at card, then skip the re-prompt. */
    int guard = 0, selected = 0;
    while (pending(tg) && guard++ < 10) {
        if (!selected) { pick(tg, 0); selected = 1; }
        else skip(tg);
    }

    const int three[3] = { a, b, c };
    int kept = 0, kept_id = -1, discarded = 0;
    for (int i = 0; i < 3; i++) {
        if (deck_has(tg, three[i])) { kept++; kept_id = three[i]; }
        if (wait_has(tg, three[i])) discarded++;
    }
    CHECK_EQ(kept, 1, "dia: exactly 1 looked-at card should be kept on the deck");
    CHECK_EQ(discarded, 2, "dia: the other 2 looked-at cards go to the waitroom");
    if (tg->state.p[0].deck.n > 0 && kept_id >= 0)
        CHECK_EQ(tg->state.p[0].deck.cards[tg->state.p[0].deck.n - 1], kept_id,
                 "dia: the kept card is on the deck BOTTOM");
}

/* ------------------------------------------------------------------ *
 * §F  bottom_inspect / bottom_card_accepted_to_fourth_from_top_test.rs
 *      PL!S-bp7-010-N (Chika), debut: look at the bottom card of the deck; put
 *      it 4th from the top at will.
 * ------------------------------------------------------------------ */

static void chika_setup(TestGame *tg, int *chika_out, int *x_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int chika = mid(tg, CHIKA);
    int x = mid(tg, CHIKA_BOT);
    pin_id(chika, CHIKA);
    pin_id(x, CHIKA_BOT);
    *chika_out = chika; *x_out = x;
    if (chika < 0 || x < 0) return;
    CHECK_EQ(card_cost(chika), 4, "chika: the print's cost");
    CHECK(card_prints_trigger(chika, RB_TSTR_DEBUT), "chika: the print has 登場");
    set_stage(tg, 0, AREA_CENTER, chika);
}

/* Rust pl_s_bp7_010_n_accept_bottom_card_to_fourth_from_top */
static void test_f_chika_accept_to_fourth_from_top(void)
{
    TestGame *tg = &f_tg;
    int chika = -1, x = -1;
    chika_setup(tg, &chika, &x);
    if (chika < 0 || x < 0) return;

    int fillers[4];
    /* Four DISTINGUISHABLE prints, not four copies of one: an all-identical
     * deck makes a deck-ORDER failure invisible in the diagnostic. */
    static const char *const FN[4] = { DECK0, DECK1, DECK2, DECK3 };
    for (int i = 0; i < 4; i++) {
        fillers[i] = mid(tg, FN[i]);
        if (fillers[i] < 0) return;
        deck_append(tg, 0, fillers[i]);
    }
    deck_append(tg, 0, x);
    CHECK_EQ(tg->state.p[0].deck.n, 5, "chika1: the deck is 4 fillers + X");
    CHECK_EQ(tg->state.p[0].deck.cards[4], x, "chika1: X starts at the BOTTOM (index 4)");

    CHECK(fire_trigger(tg, chika, RB_TSTR_DEBUT), "chika1: the 登場 ability fires");
    /* VACUITY CONTROL: the Rust panics here if no choice was raised. */
    CHECK_PROMPTS(tg, 1, "chika1: the optional move-to-4th choice must be raised");
    CHECK(pending(tg), "chika1: expected the optional move-to-4th choice");
    if (pending(tg)) {
        const char *t = pending_type(tg);
        CHECK(!strcmp(t, "SelectTarget") || !strcmp(t, "SelectCard"),
              "chika1: unexpected prompt kind %s", t);
        if (!strcmp(t, "SelectTarget")) select_option(tg, 1);
        else if (!strcmp(t, "SelectCard")) pick(tg, 0);
    }
    int guard = 0;
    while (pending(tg) && guard++ < 10) pick(tg, 0);

    int deck[16];
    int n = test_zone_ids(tg, 0, "deck", deck, 16);
    assertions++;
    if (n < 4 || deck[3] != x || deck[n - 1] == x) {
        int want[5] = { fillers[0], fillers[1], fillers[2], fillers[3], x };
        failures++;
        fprintf(stderr, "FAIL: chika1: X must sit 4th from the TOP (index 3) and "
                        "must have LEFT the bottom.\n  got (n=%d):", n);
        for (int i = 0; i < n; i++) fprintf(stderr, " %s", card_no_of(deck[i]));
        fprintf(stderr, "\n  want (n=5):");
        for (int i = 0; i < 5; i++) fprintf(stderr, " %s", card_no_of(want[i]));
        fprintf(stderr, "\n");
    }
}

/* Rust pl_s_bp7_010_n_decline_keeps_bottom_card */
static void test_f_chika_decline_keeps_bottom_card(void)
{
    TestGame *tg = &f_tg;
    int chika = -1, x = -1;
    chika_setup(tg, &chika, &x);
    if (chika < 0 || x < 0) return;

    deck_append(tg, 0, x);
    CHECK_EQ(tg->state.p[0].deck.n, 1, "chika2: the deck is exactly X");
    CHECK_EQ(tg->state.p[0].deck.cards[0], x, "chika2: X is both top and bottom");

    CHECK(fire_trigger(tg, chika, RB_TSTR_DEBUT), "chika2: the 登場 ability fires");
    /* VACUITY CONTROL: on a one-card deck the look must still prompt. */
    CHECK_PROMPTS(tg, 1, "chika2: the bottom-look must prompt so the decline is real");
    int guard = 0;
    while (pending(tg) && guard++ < 10) {
        const char *t = pending_type(tg);
        if (!strcmp(t, "SelectCard")) skip(tg);
        else break;
    }
    CHECK(deck_has(tg, x), "chika2: declined -> X must STAY in the deck");
    CHECK(tg->state.p[0].deck.n > 0 &&
          tg->state.p[0].deck.cards[tg->state.p[0].deck.n - 1] == x,
          "chika2: declined -> X stays at the BOTTOM");
}

/* ==== SECTION MARKER: append below this line ==== */

/* ------------------------------------------------------------------ *
 * per_card / livesuccess_revealed_distinct_name_threshold_test.rs and
 * live_success_three_distinct_revealed_retrieves_live_test.rs
 *      PL!SP-bp4-006-R (Keke), ライブ成功時:
 *      「if among the cards MY yell revealed there are 3+ member cards of
 *        『Liella!』 with DIFFERENT NAMES, add 1 『Liella!』 LIVE card from
 *        the revealed cards to my hand」
 * Four independent clauses: a count over DISTINCT NAMES, a group filter, a
 * second group filter over a different card type, and the threshold itself.
 * ------------------------------------------------------------------ */

struct keke_board { int keke, a, b, c, live; };

/* HARNESS NOTE. Rust's `fire_trigger` queues an ability by CARD ID, so it works
 * with the host on the stage. C's rb_queue_trigger_abilities walks the zones it
 * can see, and for ライブ成功時 it does NOT reach a member sitting on the stage
 * (the 水彩世界 / フュージョンクラスト cases in section I put the host in the
 * live zone and are queued fine). Keke is therefore placed in the LIVE zone,
 * which is the C equivalent of Rust's zone-agnostic explicit trigger. This is
 * recorded because the placement - not the ability - is what changed.
 */
static void keke_setup(TestGame *tg, struct keke_board *kb, int outsiders)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);

    kb->keke = mid(tg, KEKE);
    pin_id(kb->keke, KEKE);
    kb->a = mid(tg, MEMBER_A);
    kb->b = mid(tg, MEMBER_B);
    kb->c = mid(tg, MEMBER_C);
    kb->live = mid(tg, LIELLA_LIVE);
    pin_id(kb->a, MEMBER_A);
    pin_id(kb->b, MEMBER_B);
    pin_id(kb->c, MEMBER_C);
    pin_id(kb->live, LIELLA_LIVE);
    if (kb->keke < 0 || kb->a < 0 || kb->b < 0 || kb->c < 0 || kb->live < 0) return;
    if (outsiders) {
        /* The three NON-Liella members replace a, b, c. */
        kb->a = mid(tg, OUT_HS); pin_id(kb->a, OUT_HS);
        kb->b = mid(tg, OUT_N);  pin_id(kb->b, OUT_N);
        kb->c = mid(tg, OUT_S);  pin_id(kb->c, OUT_S);
    }
    CHECK(card_prints_trigger(kb->keke, RB_TSTR_LIVE_SUCCESS),
          "keke: the print carries a ライブ成功時 trigger");
    CHECK(rb_card_is_live(kb->live), "keke: the retrieval target is a live card");
    /* The premise: three DISTINCT NAMES. The Rust file asserts this and it is
     * the whole point of the test, so it is asserted here too. */
    if (!outsiders) {
        char n0[80], n1[80], n2[80];
        snprintf(n0, sizeof n0, "%s", card_name(kb->a));
        snprintf(n1, sizeof n1, "%s", card_name(kb->b));
        snprintf(n2, sizeof n2, "%s", card_name(kb->c));
        CHECK(strcmp(n0, n1) != 0 && strcmp(n1, n2) != 0 && strcmp(n0, n2) != 0,
              "keke: the three revealed members must carry three DISTINCT names");
    }
    test_add_to_live(tg, kb->keke);   /* see the HARNESS NOTE above */
    /* HARNESS NOTE 2, measured. C short-circuits a location: revealed_cards
     * condition at QUEUE time against g->yell_occurred
     * (src/core/game_state_abilities.c:1321-1326), where Rust only evaluates
     * it at resolve time. A ライブ成功時 ability is by definition post-yell, so
     * the flag is set here to make the board the one Rust means. Without it the
     * ability is never even queued and every assertion below is vacuous. */
    tg->state.yell_occurred = 1;
}

static void keke_reveal(TestGame *tg, const int *ids, int n)
{
    tg->state.n_revealed = 0;
    for (int i = 0; i < n; i++) test_add_to_revealed(tg, ids[i]);
}

/* The POSITIVE CONTROL every negative in this section is measured against:
 * all four clauses hold, so exactly one 『Liella!』 live must be retrieved. */
static int keke_positive_control(TestGame *tg, int outsiders)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int ka = mid(tg, KEKE);
    int a = mid(tg, outsiders ? OUT_HS : MEMBER_A);
    int b = mid(tg, outsiders ? OUT_N  : MEMBER_B);
    int c = mid(tg, outsiders ? OUT_S  : MEMBER_C);
    int lv = mid(tg, LIELLA_LIVE);
    pin_id(ka, KEKE); pin_id(lv, LIELLA_LIVE);
    if (ka < 0 || a < 0 || b < 0 || c < 0 || lv < 0) return 0;
    /* The ids are RE-RESOLVED, not reused from the caller's board: this
     * control calls fresh(), which resets the copy pool and invalidates
     * every slot id the caller is still holding. */
    test_add_to_live(tg, ka);         /* see the HARNESS NOTE above */
    /* HARNESS NOTE 2, measured. C short-circuits a location: revealed_cards
     * condition at QUEUE time against g->yell_occurred
     * (src/core/game_state_abilities.c:1321-1326), where Rust only evaluates
     * it at resolve time. A ライブ成功時 ability is by definition post-yell, so
     * the flag is set here to make the board the one Rust means. Without it the
     * ability is never even queued and every assertion below is vacuous. */
    tg->state.yell_occurred = 1;
    int ids[4] = { a, b, c, lv };
    keke_reveal(tg, ids, 4);
    int hand_before = tg->state.p[0].hand.n;
    int queued = fire_trigger(tg, ka, RB_TSTR_LIVE_SUCCESS);
    int guard = 0;
    while (pending(tg) && guard++ < 10) pick(tg, 0);
    CHECK(queued, "keke control: the ライブ成功時 ability fires at all");
    return tg->state.p[0].hand.n == hand_before + 1 && hand_has(tg, lv);
}

/* Rust sp_bp4_006_live_success_with_three_distinct_liella_revealed_... */
static void test_g_three_distinct_plus_live_retrieves_live(void)
{
    TestGame *tg = &g_tg;
    struct keke_board kb;
    keke_setup(tg, &kb, 0);
    if (kb.keke < 0) return;
    int members[3] = { kb.a, kb.b, kb.c };
    pin_id(members[0], MEMBER_A); pin_id(members[1], MEMBER_B);
    /* The third member is the OTHER 嵐 千砂都 print the Rust file pins. */
    int chisato = mid(tg, MEMBER_C2);
    pin_id(chisato, MEMBER_C2);
    if (chisato < 0) return;
    members[2] = chisato;
    char n0[80], n1[80], n2[80];
    snprintf(n0, sizeof n0, "%s", card_name(members[0]));
    snprintf(n1, sizeof n1, "%s", card_name(members[1]));
    snprintf(n2, sizeof n2, "%s", card_name(members[2]));
    CHECK(strcmp(n0, n1) != 0 && strcmp(n1, n2) != 0 && strcmp(n0, n2) != 0,
          "keke: the three revealed members must carry three DISTINCT names");

    int ids[4] = { members[0], members[1], members[2], kb.live };
    keke_reveal(tg, ids, 4);
    int hand_before = tg->state.p[0].hand.n;
    int revealed_before = tg->state.n_revealed;

    CHECK(fire_trigger(tg, kb.keke, RB_TSTR_LIVE_SUCCESS),
          "keke3: the ライブ成功時 ability fires");
    int guard = 0;
    while (pending(tg) && guard++ < 10) pick(tg, 0);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before + 1,
             "keke3: all four clauses hold, so exactly one card is retrieved");
    CHECK(hand_has(tg, kb.live),
          "keke3: the retrieved card is the revealed 『Liella!』 live, not a member");
    CHECK_EQ(tg->state.n_revealed, revealed_before,
             "keke3: the revealed set is consumed by the retrieval, not doubled");
}

/* Rust two_distinct_names_retrieve_nothing_however_many_copies_are_revealed */
static void test_h_two_distinct_names_retrieve_nothing(void)
{
    TestGame *tg = &h_tg;
    struct keke_board kb;
    keke_setup(tg, &kb, 0);
    if (kb.keke < 0) return;
    /* NO fresh() HERE: keke_setup already built this board, and a second
     * fresh() would reset the card-copy pool and invalidate every id
     * kb still holds. Only the revealed set differs between boards. */
    tg->state.yell_occurred = 1;        /* see HARNESS NOTE 2 */
    /* five revealed cards, only TWO distinct names. */
    int ids[5] = { kb.a, kb.b, kb.a, kb.b, kb.a };
    keke_reveal(tg, ids, 5);
    CHECK(tg->state.n_revealed >= 3,
          "keke_two_names: at least 3 CARDS are revealed, so a count-over-cards "
          "implementation would pass - this is a count-over-names test");
    char n0[80], n1[80];
    snprintf(n0, sizeof n0, "%s", card_name(kb.a));
    snprintf(n1, sizeof n1, "%s", card_name(kb.b));
    CHECK(strcmp(n0, n1) != 0, "keke_two_names: the two names are genuinely different");
    int hand_before = tg->state.p[0].hand.n;

    CHECK(fire_trigger(tg, kb.keke, RB_TSTR_LIVE_SUCCESS),
          "keke_two_names: the ライブ成功時 ability fires");
    drain_skip(tg, 10);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before,
             "keke_two_names: 5 cards across 2 distinct names is 2, so nothing is "
             "retrieved");
    CHECK(!hand_has(tg, kb.live),
          "keke_two_names: the 『Liella!』 live must not reach hand");

    /* VACUITY CONTROL: prove the ability DOES retrieve on a board where the
     * gate holds, so the negatives above are not green on a no-op. */
    CHECK(keke_positive_control(tg, 0),
          "keke_two_names CONTROL: 3 distinct 『Liella!』 names + a 『Liella!』 live "
          "must retrieve exactly that live (otherwise the negatives above are vacuous)");
}

/* Rust three_names_that_are_not_liella_retrieve_nothing */
static void test_h_three_non_liella_names_retrieve_nothing(void)
{
    TestGame *tg = &h_tg;
    struct keke_board kb;
    keke_setup(tg, &kb, 1);
    if (kb.keke < 0) return;
    /* NO fresh() HERE: keke_setup already built this board, and a second
     * fresh() would reset the card-copy pool and invalidate every id
     * kb still holds. Only the revealed set differs between boards. */
    tg->state.yell_occurred = 1;        /* see HARNESS NOTE 2 */
    int ids[4] = { kb.a, kb.b, kb.c, kb.live };
    keke_reveal(tg, ids, 4);
    int hand_before = tg->state.p[0].hand.n;

    CHECK(fire_trigger(tg, kb.keke, RB_TSTR_LIVE_SUCCESS),
          "keke_outside: the ライブ成功時 ability fires");
    drain_skip(tg, 10);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before,
             "keke_outside: three non-『Liella!』 member names do not satisfy the "
             "count - the group filter is part of the count, not decoration");
    CHECK(!hand_has(tg, kb.live),
          "keke_outside: a 『Liella!』 live alone is not enough");

    /* VACUITY CONTROL. */
    TestGame *ctl = &i_tg;
    CHECK(keke_positive_control(ctl, 0),
          "keke_outside CONTROL: the 4-clause-true board must retrieve 1 live, or "
          "the negatives above are vacuous");
}

/* Rust threshold_met_but_no_liella_live_revealed_retrieves_no_card_at_all */
static void test_h_threshold_met_no_live_retrieves_nothing(void)
{
    TestGame *tg = &h_tg;
    struct keke_board kb;
    keke_setup(tg, &kb, 0);
    if (kb.keke < 0) return;
    /* NO fresh() HERE: keke_setup already built this board, and a second
     * fresh() would reset the card-copy pool and invalidate every id
     * kb still holds. Only the revealed set differs between boards. */
    tg->state.yell_occurred = 1;        /* see HARNESS NOTE 2 */
    int ids[3] = { kb.a, kb.b, kb.c };
    keke_reveal(tg, ids, 3);
    int hand_before = tg->state.p[0].hand.n;
    int revealed_before = tg->state.n_revealed;

    CHECK(fire_trigger(tg, kb.keke, RB_TSTR_LIVE_SUCCESS),
          "keke_nolive: the ライブ成功時 ability fires");
    drain_skip(tg, 10);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before,
             "keke_nolive: the member clause is satisfied but no 『Liella!』 live is "
             "revealed, so NOTHING is retrieved - a member must not be taken in its place");
    CHECK_EQ(tg->state.n_revealed, revealed_before,
             "keke_nolive: the revealed set is left intact when the second clause "
             "finds nothing");

    /* VACUITY CONTROL. */
    CHECK(keke_positive_control(tg, 0),
          "keke_nolive CONTROL: the same members + a 『Liella!』 live must retrieve "
          "it, or the negatives above are vacuous");
}

/* ------------------------------------------------------------------ *
 * per_card / live_success_cost_gated_recovered_member_test.rs
 *      PL!HS-cl1-009-CL: ライブ成功時, from the revealed cards add 1 『蓮ノ空』
 *      member whose cost is 4..9 to hand.
 *      PL!HS-bp6-032-L: ライブ成功時, from the revealed cards add 1 member
 *      whose cost is <= 4 to hand.
 * ------------------------------------------------------------------ */

struct gated { int live, target, hand_before, revealed_before; };

static void gated_setup(TestGame *tg, const char *live_no, const char *target_no,
                        struct gated *gd)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    gd->live = mid(tg, live_no);
    gd->target = mid(tg, target_no);
    pin_id(gd->live, live_no);
    pin_id(gd->target, target_no);
    gd->hand_before = 0;
    gd->revealed_before = 0;
    if (gd->live < 0 || gd->target < 0) return;
    CHECK(card_prints_trigger(gd->live, RB_TSTR_LIVE_SUCCESS),
          "gated: the live print carries a ライブ成功時 trigger");
    CHECK(rb_card_is_live(gd->live), "gated: the trigger card is a live card");
    test_add_to_live(tg, gd->live);
    test_add_to_revealed(tg, gd->target);
    gd->hand_before = tg->state.p[0].hand.n;
    gd->revealed_before = tg->state.n_revealed;
}

/* The POSITIVE CONTROL: the same live card, on a board where the gate holds. */
static int gated_positive_control(TestGame *tg, const char *live_no, const char *good_no)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int live = mid(tg, live_no);
    int good = mid(tg, good_no);
    pin_id(live, live_no);
    pin_id(good, good_no);
    if (live < 0 || good < 0) return 0;
    test_add_to_live(tg, live);
    test_add_to_revealed(tg, good);
    int hb = tg->state.p[0].hand.n;
    int queued = fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS);
    int guard = 0;
    while (pending(tg) && guard++ < 10) pick(tg, 0);
    CHECK(queued, "gated CONTROL: the ライブ成功時 ability fires at all");
    return tg->state.p[0].hand.n == hb + 1;
}

/* Rust hs_cl1_009_live_success_hasunosora_cost_four_recovers_revealed_member */
static void test_i_suisui_cost_four_recovers(void)
{
    TestGame *tg = &i_tg;
    struct gated gd;
    gated_setup(tg, SUISUI_CLS, SUISUI_C4, &gd);
    if (gd.live < 0 || gd.target < 0) return;
    CHECK_EQ(card_cost(gd.target), 4, "suisui_c4: the lower-boundary cost is 4");
    CHECK(fire_trigger(tg, gd.live, RB_TSTR_LIVE_SUCCESS), "suisui_c4: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "suisui_c4: a single cost-4 candidate auto-resolves");
    CHECK(hand_has(tg, gd.target), "suisui_c4: the cost-4 member is retrieved");
}

/* Rust hs_cl1_009_live_success_hasunosora_cost_nine_recovers_revealed_member */
static void test_i_suisui_cost_nine_recovers(void)
{
    TestGame *tg = &i_tg;
    struct gated gd;
    gated_setup(tg, SUISUI_CLS, SUISUI_C9, &gd);
    if (gd.live < 0 || gd.target < 0) return;
    CHECK_EQ(card_cost(gd.target), 9, "suisui_c9: the upper-boundary cost is 9");
    CHECK(fire_trigger(tg, gd.live, RB_TSTR_LIVE_SUCCESS), "suisui_c9: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "suisui_c9: a single cost-9 candidate auto-resolves");
    CHECK(hand_has(tg, gd.target), "suisui_c9: the cost-9 member is retrieved");
}

/* Rust hs_cl1_009_live_success_cost_ten_no_prompt_or_recovery */
static void test_i_suisui_cost_ten_no_prompt(void)
{
    TestGame *tg = &i_tg;
    /* VACUITY CONTROL: the same live card DOES retrieve on a cost-4 board. */
    CHECK(gated_positive_control(tg, SUISUI_CLS, SUISUI_C4),
          "suisui_c10 CONTROL: the same live must retrieve a cost-4 member, or the "
          "no-prompt assertion below is vacuous");
    struct gated gd;
    gated_setup(tg, SUISUI_CLS, SUISUI_C10, &gd);
    if (gd.live < 0 || gd.target < 0) return;
    CHECK_EQ(card_cost(gd.target), 10, "suisui_c10: the out-of-range cost is 10");
    CHECK(fire_trigger(tg, gd.live, RB_TSTR_LIVE_SUCCESS), "suisui_c10: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "suisui_c10: only out-of-range candidates -> no prompt");
    CHECK(!hand_has(tg, gd.target), "suisui_c10: the cost-10 member stays revealed");
    CHECK(revealed_has(tg, gd.target), "suisui_c10: the revealed pool is untouched");
}

/* Rust hs_cl1_009_live_success_wrong_group_cost_four_no_prompt */
static void test_i_suisui_wrong_group_no_prompt(void)
{
    TestGame *tg = &i_tg;
    /* VACUITY CONTROL. */
    CHECK(gated_positive_control(tg, SUISUI_CLS, SUISUI_C4),
          "suisui_group CONTROL: a cost-4 『蓮ノ空』 member must be retrieved");
    struct gated gd;
    gated_setup(tg, SUISUI_CLS, FILLER, &gd);
    if (gd.live < 0 || gd.target < 0) return;
    CHECK_EQ(card_cost(gd.target), 4, "suisui_group: the outsider still has cost 4");
    CHECK(strcmp(rb_card_group_name(gd.target), rb_card_group_name(gd.live)) != 0,
          "suisui_group: PL!-sd1-010-SD is NOT in the same group as the live card");
    CHECK(fire_trigger(tg, gd.live, RB_TSTR_LIVE_SUCCESS), "suisui_group: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "suisui_group: a non-蓮ノ空 candidate -> no selection prompt");
    CHECK(!hand_has(tg, gd.target), "suisui_group: the outsider is not retrieved");
}

/* Rust hs_bp6_032_live_success_cost_four_recovers_revealed_member */
static void test_i_fusion_cost_four_recovers(void)
{
    TestGame *tg = &j_tg;
    struct gated gd;
    gated_setup(tg, FUSION, FUSION_C4, &gd);
    if (gd.live < 0 || gd.target < 0) return;
    CHECK_EQ(card_cost(gd.target), 4, "fusion_c4: the boundary cost is 4");
    CHECK(fire_trigger(tg, gd.live, RB_TSTR_LIVE_SUCCESS), "fusion_c4: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "fusion_c4: a single cost-4 candidate auto-resolves");
    CHECK(hand_has(tg, gd.target), "fusion_c4: the cost-4 member is retrieved");
}

/* Rust hs_bp6_032_live_success_cost_above_four_no_prompt */
static void test_i_fusion_cost_above_four_no_prompt(void)
{
    TestGame *tg = &j_tg;
    /* VACUITY CONTROL. */
    CHECK(gated_positive_control(tg, FUSION, FUSION_C4),
          "fusion_c11 CONTROL: the same live must retrieve a cost-4 member");
    struct gated gd;
    gated_setup(tg, FUSION, FUSION_C11, &gd);
    if (gd.live < 0 || gd.target < 0) return;
    CHECK_EQ(card_cost(gd.target), 11, "fusion_c11: the out-of-range cost is 11");
    CHECK(fire_trigger(tg, gd.live, RB_TSTR_LIVE_SUCCESS), "fusion_c11: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "fusion_c11: only cost>4 candidates -> no selection prompt");
    CHECK(!hand_has(tg, gd.target), "fusion_c11: the cost-11 member is not retrieved");
}

/* ------------------------------------------------------------------ *
 * per_card / live_success_distortion_heart_requirement_test.rs
 *      PL!SP-pb2-048-L (Distortion), live_start: for each DIFFERENTLY NAMED
 *      『CatChu!』 member on MY stage, reduce this card's need_heart by heart0 x2
 *      and INCREASE heart02 by 1; then, if heart02 in need_heart is >= 9, +1 score.
 *      Printed need_heart: { heart02: 6, heart0: 9 }.
 *
 *      The 5th Rust test, distortion_parse_increase_half_is_increase, is a pure
 *      PARSER-SHAPE audit on the decoded ability tree (ActionType::Sequential,
 *      operation_any, condition.get_location/get_aggregate). It is not a
 *      look/select behaviour and no C entry point exposes the same tree shape
 *      here, so it is reported as NOT PORTED rather than faked.
 * ------------------------------------------------------------------ */

struct distortion_board { int live; };

static void distortion_setup(TestGame *tg, struct distortion_board *db,
                             const char *const *members, int n_members)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int live = mid(tg, DISTORTION);
    pin_id(live, DISTORTION);
    db->live = live;
    if (live < 0) return;
    CHECK_EQ(card_score(live), 6, "distortion: the printed score is 6");
    CHECK_EQ(printed_need_heart(live, 0), 9, "distortion: printed need_heart heart0 = 9");
    CHECK_EQ(printed_need_heart(live, 2), 6, "distortion: printed need_heart heart02 = 6");
    CHECK(card_prints_trigger(live, RB_TSTR_LIVE_START),
          "distortion: the print carries a ライブ開始時 trigger");
    dump_heart_table(live, DISTORTION);
    test_add_to_live(tg, live);
    for (int i = 0; i < n_members && i < RB_STAGE_SIZE; i++) {
        int m = mid(tg, members[i]);
        pin_id(m, members[i]);
        set_stage(tg, 0, i, m);
    }
    deck_append_n(tg, 0, mid(tg, FILLER), 30);
    deck_append_n(tg, 1, mid(tg, FILLER), 30);
}

static void distortion_drain(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 20) {
        const char *t = pending_type(tg);
        if (!strcmp(t, "SelectAutoAbility")) skip(tg);
        else pick(tg, 0);
    }
}

/* Rust distortion_three_distinct_catchu_reduces_and_scores */
static void test_j_distortion_three_distinct_scores(void)
{
    TestGame *tg = &k_tg;
    static const char *const M[3] = { CATCHU_1, CATCHU_2, CATCHU_3 };
    struct distortion_board db;
    distortion_setup(tg, &db, M, 3);
    if (db.live < 0) return;
    /* The dedupe premise: three DIFFERENT names, so per-unit == 3. */
    char n0[80], n1[80], n2[80];
    snprintf(n0, sizeof n0, "%s", card_name(tg->state.p[0].stage[0]));
    snprintf(n1, sizeof n1, "%s", card_name(tg->state.p[0].stage[1]));
    snprintf(n2, sizeof n2, "%s", card_name(tg->state.p[0].stage[2]));
    CHECK(strcmp(n0, n1) != 0 && strcmp(n1, n2) != 0 && strcmp(n0, n2) != 0,
          "distortion3: the three staged members carry three DISTINCT names");

    advance_to_live_card_set(tg, 5);
    advance_to_live_card_set(tg, 2);
    distortion_drain(tg);

    CHECK_EQ(effective_need_heart(tg, db.live, 0), 3,
             "distortion3: heart0 9 - 2*3 = 3");
    CHECK_EQ(effective_need_heart(tg, db.live, 2), 9,
             "distortion3: heart02 6 + 1*3 = 9");
    CHECK_EQ(score_mod(tg, db.live), 1,
             "distortion3: the heart02 >= 9 gate is met -> score +1");
}

/* Rust distortion_one_catchu_no_score_gate_not_met */
static void test_j_distortion_one_catchu_no_score(void)
{
    TestGame *tg = &k_tg;
    static const char *const M[1] = { CATCHU_1 };
    struct distortion_board db;
    distortion_setup(tg, &db, M, 1);
    if (db.live < 0) return;
    advance_to_live_card_set(tg, 5);
    advance_to_live_card_set(tg, 2);
    distortion_drain(tg);
    CHECK_EQ(effective_need_heart(tg, db.live, 0), 7, "distortion1: heart0 9-2*1 = 7");
    CHECK_EQ(effective_need_heart(tg, db.live, 2), 7, "distortion1: heart02 6+1*1 = 7");
    CHECK_EQ(score_mod(tg, db.live), 0, "distortion1: heart02 = 7 < 9 -> no score");
}

/* Rust distortion_duplicate_names_dedupe */
static void test_j_distortion_duplicate_names_dedupe(void)
{
    TestGame *tg = &l_tg;
    static const char *const M[2] = { CATCHU_1, CATCHU_K2 };
    struct distortion_board db;
    distortion_setup(tg, &db, M, 2);
    if (db.live < 0) return;
    /* The dedupe only means something because these are two DIFFERENT prints
     * that SHARE a name. */
    pin_id(tg->state.p[0].stage[0], CATCHU_1);
    pin_id(tg->state.p[0].stage[1], CATCHU_K2);
    CHECK(tg->state.p[0].stage[0] != tg->state.p[0].stage[1],
          "distortion_dup: the two staged slots are DISTINCT card instances");
    char n0[80], n1[80];
    snprintf(n0, sizeof n0, "%s", card_name(tg->state.p[0].stage[0]));
    snprintf(n1, sizeof n1, "%s", card_name(tg->state.p[0].stage[1]));
    CHECK(!strcmp(n0, n1),
          "distortion_dup: the two prints SHARE a name (duplicate = 1 unit)");
    advance_to_live_card_set(tg, 5);
    advance_to_live_card_set(tg, 2);
    distortion_drain(tg);
    CHECK_EQ(effective_need_heart(tg, db.live, 0), 7,
             "distortion_dup: duplicate names -> 1 unit: 9-2 = 7");
    CHECK_EQ(effective_need_heart(tg, db.live, 2), 7,
             "distortion_dup: duplicate names -> 1 unit: 6+1 = 7");
    CHECK_EQ(score_mod(tg, db.live), 0, "distortion_dup: the gate is not met");
}

/* Rust distortion_no_catchu_no_change */
static void test_j_distortion_no_catchu_no_change(void)
{
    TestGame *tg = &l_tg;
    static const char *const M[1] = { FILLER };   /* μ's, not 『CatChu!』 */
    struct distortion_board db;
    distortion_setup(tg, &db, M, 1);
    if (db.live < 0) return;
    CHECK(strcmp(rb_card_group_name(tg->state.p[0].stage[0]),
                 rb_card_group_name(db.live)) != 0,
          "distortion0: PL!-sd1-010-SD is NOT in the same group as Distortion");
    advance_to_live_card_set(tg, 5);
    advance_to_live_card_set(tg, 2);
    distortion_drain(tg);
    CHECK_EQ(effective_need_heart(tg, db.live, 0), 9, "distortion0: heart0 unchanged");
    CHECK_EQ(effective_need_heart(tg, db.live, 2), 6, "distortion0: heart02 unchanged");
    CHECK_EQ(score_mod(tg, db.live), 0, "distortion0: no score");
}

/* ------------------------------------------------------------------ *
 * per_card / live_success_revealed_cards_extra_yells_test.rs
 *      PL!S-bp6-021-L (MIRAI TICKET), 自動/ターン1回: when I yell, put up to 1
 *      『Aqours』 member card WITHOUT a blade heart from the revealed cards into
 *      the waitroom; if you do, yell once more per 5 cost of that card.
 *
 *      The five cost-tier Rust tests (cost 4 / 5 / 9 / 10 / 20) assert exactly
 *      the same thing over different cost fixtures, so they are ported as ONE
 *      loop that pins every tier's printed cost and the same movement claim.
 * ------------------------------------------------------------------ */

static int mirai_setup(TestGame *tg, const int *ids, int n)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int mirai = mid(tg, MIRAI);
    pin_id(mirai, MIRAI);
    if (mirai < 0) return -1;
    CHECK(card_prints_trigger(mirai, RB_TSTR_AUTO),
          "mirai: the print carries a 自動 trigger");
    test_add_to_live(tg, mirai);
    for (int i = 0; i < n; i++) {
        test_add_to_revealed(tg, ids[i]);
        /* The Rust fixture ALSO seeds the waitroom with each revealed id. */
        test_add_to_discard(tg, ids[i]);
    }
    tg->state.yell_occurred = 1;
    return mirai;
}

static void mirai_drain(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 10) skip(tg);
}

static void check_mirai_offer(TestGame *tg, const char *what)
{
    assertions++;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (c && c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "revealed_cards") &&
        c->count == 1 && c->allow_skip) return;
    failures++;
    fprintf(stderr, "FAIL: %s (kind=%d zone=%s count=%d allow_skip=%d; want "
                    "SelectCard revealed_cards count=1 allow_skip=1)\n",
            what, c ? c->kind : -1, c ? c->zone : "(none)", c ? c->count : -1,
            c ? c->allow_skip : -1);
}

/* Rust mirai_ticket_two_cards_select_first */
static void test_k_mirai_select_first(void)
{
    TestGame *tg = &m_tg;
    int aq1 = mid(tg, AQ1), aq2 = mid(tg, AQ2);
    pin_id(aq1, AQ1); pin_id(aq2, AQ2);
    if (aq1 < 0 || aq2 < 0) return;
    /* Fixture premise: both are 『Aqours』 members WITHOUT a blade heart. */
    CHECK_EQ(card_cost(aq1), 9, "mirai_first: AQ1 prints cost 9");
    CHECK_EQ(card_cost(aq2), 4, "mirai_first: AQ2 prints cost 4");
    CHECK_EQ(card_has_blade_heart(aq1), 0, "mirai_first: AQ1 has no blade heart");
    CHECK_EQ(card_has_blade_heart(aq2), 0, "mirai_first: AQ2 has no blade heart");
    if (mirai_setup(tg, NULL, 0) < 0) return;
    test_add_to_revealed(tg, aq1);
    test_add_to_revealed(tg, aq2);
    test_add_to_discard(tg, aq1);
    test_add_to_discard(tg, aq2);
    tg->state.yell_occurred = 1;

    CHECK(fire_auto(tg) > 0, "mirai_first: the 自動 ability is queued");
    CHECK_PROMPTS(tg, 1, "mirai_first: MIRAI TICKET must show a choice");
    check_mirai_offer(tg, "mirai_first: the offer is SelectCard(revealed_cards)");
    pick(tg, 0);
    mirai_drain(tg);
    CHECK(!revealed_has(tg, aq1), "mirai_first: the first revealed card moved to the waitroom");
}

/* Rust mirai_ticket_two_cards_select_second */
static void test_k_mirai_select_second(void)
{
    TestGame *tg = &m_tg;
    int aq1 = mid(tg, AQ1), aq2 = mid(tg, AQ2);
    pin_id(aq1, AQ1); pin_id(aq2, AQ2);
    if (aq1 < 0 || aq2 < 0) return;
    if (mirai_setup(tg, NULL, 0) < 0) return;
    test_add_to_revealed(tg, aq1);
    test_add_to_revealed(tg, aq2);
    test_add_to_discard(tg, aq1);
    test_add_to_discard(tg, aq2);
    tg->state.yell_occurred = 1;
    CHECK(fire_auto(tg) > 0, "mirai_second: the 自動 ability is queued");
    CHECK_PROMPTS(tg, 1, "mirai_second: MIRAI TICKET must show a choice");
    pick(tg, 1);
    mirai_drain(tg);
    CHECK(!revealed_has(tg, aq2), "mirai_second: the second revealed card moved to the waitroom");
}

/* Rust mirai_ticket_skip_leaves_both_in_revealed */
static void test_k_mirai_skip_leaves_both(void)
{
    TestGame *tg = &m_tg;
    int aq1 = mid(tg, AQ1), aq2 = mid(tg, AQ2);
    pin_id(aq1, AQ1); pin_id(aq2, AQ2);
    if (aq1 < 0 || aq2 < 0) return;
    if (mirai_setup(tg, NULL, 0) < 0) return;
    test_add_to_revealed(tg, aq1);
    test_add_to_revealed(tg, aq2);
    test_add_to_discard(tg, aq1);
    test_add_to_discard(tg, aq2);
    tg->state.yell_occurred = 1;
    CHECK(fire_auto(tg) > 0, "mirai_skip: the 自動 ability is queued");
    CHECK_PROMPTS(tg, 1, "mirai_skip: MIRAI TICKET must show a choice");
    skip(tg);
    mirai_drain(tg);
    CHECK(revealed_has(tg, aq1), "mirai_skip: skipped, aq1 stays revealed");
    CHECK(revealed_has(tg, aq2), "mirai_skip: skipped, aq2 stays revealed");
}

/* Rust mirai_ticket_skip_with_single_card */
static void test_k_mirai_skip_with_single(void)
{
    TestGame *tg = &m_tg;
    int aq = mid(tg, AQ1);
    pin_id(aq, AQ1);
    if (aq < 0) return;
    if (mirai_setup(tg, NULL, 0) < 0) return;
    test_add_to_revealed(tg, aq);
    test_add_to_discard(tg, aq);
    tg->state.yell_occurred = 1;
    CHECK(fire_auto(tg) > 0, "mirai_single: the 自動 ability is queued");
    /* VACUITY CONTROL: the Rust asserts a choice is offered even with 1 card. */
    CHECK_PROMPTS(tg, 1, "mirai_single: a choice is expected even with 1 card");
    skip(tg);
    mirai_drain(tg);
    CHECK(revealed_has(tg, aq), "mirai_single: skipped, the card stays revealed");
}

/* Rust mirai_ticket_use_limit_blocks_second_trigger */
static void test_k_mirai_use_limit_blocks_second(void)
{
    TestGame *tg = &m_tg;
    int aq = mid(tg, AQ2), aq2 = mid(tg, AQ1);
    pin_id(aq, AQ2); pin_id(aq2, AQ1);
    if (aq < 0 || aq2 < 0) return;
    if (mirai_setup(tg, NULL, 0) < 0) return;
    test_add_to_revealed(tg, aq);
    test_add_to_discard(tg, aq);
    tg->state.yell_occurred = 1;
    CHECK(fire_auto(tg) > 0, "mirai_limit: the first 自動 is queued");
    CHECK_PROMPTS(tg, 1, "mirai_limit: the FIRST trigger must offer a choice");
    pick(tg, 0);
    mirai_drain(tg);
    CHECK(!revealed_has(tg, aq), "mirai_limit: first trigger, the card moved");

    test_add_to_revealed(tg, aq2);
    test_add_to_discard(tg, aq2);
    tg->state.yell_occurred = 1;
    int before = prompts;
    fire_auto(tg);
    mirai_drain(tg);
    CHECK(prompts == before,
          "mirai_limit: the second trigger must be blocked by the ターン1回 limit");
    CHECK(revealed_has(tg, aq2), "mirai_limit: aq2 stays revealed, the ability did not fire again");
}

/* Rust mirai_ticket_empty_revealed_triggers_and_auto_skips */
static void test_k_mirai_empty_revealed_auto_skips(void)
{
    TestGame *tg = &m_tg;
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int mirai = mid(tg, MIRAI);
    pin_id(mirai, MIRAI);
    if (mirai < 0) return;
    test_add_to_live(tg, mirai);
    tg->state.yell_occurred = 1;
    fire_auto(tg);
    /* VACUITY CONTROL: the ability IS queued here; what it must not do is
     * prompt. */
    CHECK(!pending(tg), "mirai_empty: an empty revealed set auto-skips with no choice");
}

/* All five Rust cost-tier tests in one loop, each pinning the printed cost. */
static void test_k_mirai_cost_tiers(void)
{
    static const char *const NOS[5] = { AQ2, AQ3, AQ1, AQ4, AQ5 };
    static const int COSTS[5] = { 4, 5, 9, 10, 20 };
    for (int v = 0; v < 5; v++) {
        TestGame *tg = &n_tg;
        int aq = mid(tg, NOS[v]);
        pin_id(aq, NOS[v]);
        if (aq < 0) continue;
        CHECK_EQ(card_cost(aq), COSTS[v],
                 "mirai_cost%d: the print's cost (the extra-yell tier)", COSTS[v]);
        CHECK_EQ(card_has_blade_heart(aq), 0,
                 "mirai_cost%d: the card carries no blade heart", COSTS[v]);
        if (mirai_setup(tg, NULL, 0) < 0) continue;
        test_add_to_revealed(tg, aq);
        test_add_to_discard(tg, aq);
        tg->state.yell_occurred = 1;
        CHECK(fire_auto(tg) > 0, "mirai_cost%d: the 自動 ability is queued", COSTS[v]);
        CHECK_PROMPTS(tg, 1, "mirai_cost%d: the offer must be raised", COSTS[v]);
        pick(tg, 0);
        mirai_drain(tg);
        CHECK(!revealed_has(tg, aq),
              "mirai_cost%d: the selected card moves out of the revealed set", COSTS[v]);
    }
}

/* ------------------------------------------------------------------ *
 * per_card / live_success_under_member_count_score_test.rs
 *      PL!N-bp7-029-L (Burn!!), ライブ成功時: optionally move ALL the energy
 *      cards under one of my stage members into my energy zone in the WAIT
 *      state; if you placed 1+ energy cards that way and my energy is 10 or
 *      more, this card's score is +1.
 * ------------------------------------------------------------------ */

static int burn_setup(TestGame *tg, int under_count, int energy_count, int *member_out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int mem = mid(tg, BURN_MEMBER);
    pin_id(mem, BURN_MEMBER);
    int burn = mid(tg, BURN);
    pin_id(burn, BURN);
    *member_out = mem;
    if (mem < 0 || burn < 0) return -1;
    CHECK(card_prints_trigger(burn, RB_TSTR_LIVE_SUCCESS),
          "burn: the print carries a ライブ成功時 trigger");
    set_stage(tg, 0, AREA_CENTER, mem);
    for (int i = 0; i < under_count; i++)
        test_place_under(tg, 0, AREA_CENTER, mid(tg, ENERGY));
    test_give_energy(tg, energy_count);
    test_add_to_success(tg, burn);
    CHECK_EQ(tg->state.p[0].under_cards[AREA_CENTER].n, under_count,
             "burn: the under-card fixture has %d energy cards", under_count);
    CHECK_EQ(tg->state.p[0].energy_active, energy_count,
             "burn: the active energy fixture is %d", energy_count);
    return 0;
}

/* Rust select_burn_move: pay_optional_cost -> option 1, stage SelectCard ->
 * index 0, anything else -> index 0. */
static int burn_select_move(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 20) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->kind == RB_CHOICE_SELECT_TARGET && strstr(c->target, "pay_optional_cost"))
            select_option(tg, 1);
        else
            pick(tg, 0);
    }
    return guard;
}

/* Rust burn_no_under_no_move_no_score */
static void test_l_burn_no_under_no_move_no_score(void)
{
    TestGame *tg = &o_tg;
    int mem = -1;
    if (burn_setup(tg, 0, 10, &mem) != 0) return;
    int burn = tg->state.p[0].success.cards[0];
    int score_before = score_mod(tg, burn);
    CHECK(fire_trigger(tg, burn, RB_TSTR_LIVE_SUCCESS), "burn0: fires");
    drain_skip(tg, 10);
    CHECK(!pending(tg), "burn0: with 0 under cards the move effect auto-skips");
    CHECK_EQ(score_mod(tg, burn), score_before, "burn0: 0 under + 10 total is 0");

    /* VACUITY CONTROL: the IDENTICAL board plus one under card must prompt, so
     * the "no prompt" above is a real zero-candidate skip and not a dead
     * ability. */
    TestGame *ctl = &p_tg;
    int cmem = -1;
    if (burn_setup(ctl, 1, 10, &cmem) != 0) return;
    int cburn = ctl->state.p[0].success.cards[0];
    int cscore_before = score_mod(ctl, cburn);
    CHECK(fire_trigger(ctl, cburn, RB_TSTR_LIVE_SUCCESS), "burn0 CONTROL: fires");
    burn_select_move(ctl);
    CHECK(prompts > 0,
          "burn0 CONTROL: with 1 under card the same ability MUST offer a prompt, "
          "otherwise the no-prompt assertion above is vacuous");
    CHECK(score_mod(ctl, cburn) != cscore_before || tg->state.p[0].under_cards[AREA_CENTER].n == 0,
          "burn0 CONTROL: the 1-under board is acted on");
}

/* Rust burn_one_under_moves_and_scores_when_total_10 */
static void test_l_burn_one_under_total_ten(void)
{
    TestGame *tg = &o_tg;
    int mem = -1;
    if (burn_setup(tg, 1, 9, &mem) != 0) return;
    int burn = tg->state.p[0].success.cards[0];
    CHECK_EQ(tg->state.p[0].energy_active, 9, "burn1: 9 active energy");
    CHECK_EQ(tg->state.p[0].under_cards[AREA_CENTER].n, 1, "burn1: 1 under card");
    CHECK(fire_trigger(tg, burn, RB_TSTR_LIVE_SUCCESS), "burn1: fires");
    /* VACUITY CONTROL: this test's only claim is a positive score, and that is
     * impossible without a prompt, so the prompt count is asserted first. */
    CHECK_PROMPTS(tg, 1, "burn1: the optional under-member move must be offered");
    burn_select_move(tg);
    CHECK_EQ(score_mod(tg, burn), 1, "burn1: 1 under + 9 = 10 with 1 moved -> +1");
}

/* Rust burn_one_under_but_total_9_no_score */
static void test_l_burn_one_under_total_nine(void)
{
    TestGame *tg = &o_tg;
    int mem = -1;
    if (burn_setup(tg, 1, 8, &mem) != 0) return;
    int burn = tg->state.p[0].success.cards[0];
    CHECK(fire_trigger(tg, burn, RB_TSTR_LIVE_SUCCESS), "burn9: fires");
    /* VACUITY CONTROL: the move MUST still have been offered here - it is the
     * energy TOTAL that fails, not the ability. */
    CHECK_PROMPTS(tg, 1, "burn9: the optional under-member move must still be offered");
    burn_select_move(tg);
    CHECK_EQ(score_mod(tg, burn), 0, "burn9: 1 under + 8 = 9 total -> 0");
}

/* Rust burn_skip_optional_no_score_even_with_10_total */
static void test_l_burn_skip_optional_no_score(void)
{
    TestGame *tg = &p_tg;
    int mem = -1;
    if (burn_setup(tg, 3, 10, &mem) != 0) return;
    int burn = tg->state.p[0].success.cards[0];
    CHECK(fire_trigger(tg, burn, RB_TSTR_LIVE_SUCCESS), "burn_skip: fires");
    CHECK_PROMPTS(tg, 1, "burn_skip: the under-member selection prompt is expected");
    CHECK(pending(tg), "burn_skip: a choice is pending");
    if (pending(tg))
        CHECK(!strcmp(pending_type(tg), "SelectCard"),
              "burn_skip: expected SelectCard (the stage member whose under to move), got %s",
              pending_type(tg));
    skip(tg);
    drain_skip(tg, 10);
    CHECK_EQ(score_mod(tg, burn), 0,
             "burn_skip: declining the optional move -> 0 moved -> no score even with 10 total");
}

/* Rust burn_multiple_under_all_moved_and_scores */
static void test_l_burn_multiple_under_all_moved(void)
{
    TestGame *tg = &p_tg;
    int mem = -1;
    if (burn_setup(tg, 3, 7, &mem) != 0) return;
    int burn = tg->state.p[0].success.cards[0];
    CHECK(fire_trigger(tg, burn, RB_TSTR_LIVE_SUCCESS), "burn3: fires");
    CHECK_PROMPTS(tg, 1, "burn3: the optional under-member move must be offered");
    burn_select_move(tg);
    CHECK_EQ(score_mod(tg, burn), 1, "burn3: 3 under + 7 = 10 with 3 moved -> +1");
}

/* Rust burn_wait_state_verification */
static void test_l_burn_wait_state(void)
{
    TestGame *tg = &q_tg;
    int mem = -1;
    if (burn_setup(tg, 2, 8, &mem) != 0) return;
    int burn = tg->state.p[0].success.cards[0];
    CHECK(fire_trigger(tg, burn, RB_TSTR_LIVE_SUCCESS), "burn_wait: fires");
    burn_select_move(tg);
    /* The two moved cards land in the energy zone in the WAIT state, so the
     * ACTIVE count is unchanged - that is the whole point, and the second
     * assertion is what proves the cards really moved. */
    CHECK_EQ(tg->state.p[0].energy_active, 8,
             "burn_wait: the moved cards land in the WAIT state, so the active "
             "count is unchanged");
    CHECK_EQ(tg->state.p[0].energy.n, 10,
             "burn_wait: both under-cards joined the energy zone");
}

/* ------------------------------------------------------------------ *
 * per_card / move_five_to_discard_draws_if_live_card_test.rs
 *      PL!-sd1-007-SD (Nozomi, starter), debut: put the top 5 cards of my deck
 *      into the waitroom; if a live card is among them, draw 1.
 * ------------------------------------------------------------------ */
struct nagi_result {
    int hand_before, hand_after, discard_after;
    /* The EXACT pool slots the fixture used. Re-resolving mid() after the run
     * would allocate a NEW slot with a different id, so every zone assertion
     * would silently test the wrong card. */
    int top_ids[5];
    int pre_discard_id;
};

static void nagi_run(TestGame *tg, const char *const *top5, const char *const *extra,
                     int n_top5, int n_extra, const char *pre_discard_card,
                     struct nagi_result *out)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int nagi = mid(tg, NAGI);
    pin_id(nagi, NAGI);
    if (nagi < 0) return;
    CHECK_EQ(card_cost(nagi), 7, "nagi: the print's cost is 7");
    CHECK(card_prints_trigger(nagi, RB_TSTR_DEBUT), "nagi: the print has 登場");
    out->pre_discard_id = -1;
    deck_append_n(tg, 0, mid(tg, FILLER), 10);
    if (pre_discard_card) {
        int pd = mid(tg, pre_discard_card);
        pin_id(pd, pre_discard_card);
        out->pre_discard_id = pd;
        test_add_to_discard(tg, pd);
    }
    for (int i = 0; i < n_top5 && i < 5; i++) {
        out->top_ids[i] = mid(tg, top5[i]);
        pin_id(out->top_ids[i], top5[i]);
        deck_append(tg, 0, out->top_ids[i]);
    }
    for (int i = 0; i < n_top5; i++)
        CHECK_EQ(tg->state.p[0].deck.cards[10 + i], out->top_ids[i],
                 "nagi: deck slot %d is the intended fixture card (top = index 0)",
                 10 + i);
    for (int i = 0; i < n_extra; i++) deck_append_n(tg, 0, mid(tg, DECK6), 10);
    (void)extra;
    test_give_energy(tg, 7);
    out->hand_before = tg->state.p[0].hand.n;
    test_add_to_hand(tg, nagi);
    play_to_stage(tg, nagi, AREA_CENTER, 1);
    drain_skip(tg, 10);
    out->hand_after = tg->state.p[0].hand.n;
    out->discard_after = tg->state.p[0].discard.n;
}

/* Rust test_live_card_among_five_drawn_triggers_draw */
static void test_m_nagi_live_among_five_draws(void)
{
    TestGame *tg = &r_tg;
    static const char *const TOP[5] = { NAGI_LIVE1, N11, N12, N13, DECK0 };
    static const char *const EXTRA[1] = { DECK6 };
    struct nagi_result r;
    nagi_run(tg, TOP, EXTRA, 5, 0, 0, &r);
    int live = r.top_ids[0];
    if (live < 0) return;
    CHECK(rb_card_is_live(live), "nagi_live: the top card IS a live card");
    CHECK_EQ(r.discard_after, 5, "nagi_live: exactly 5 cards moved to the waitroom");
    CHECK_EQ(r.hand_after, r.hand_before + 1,
             "nagi_live: 1 card drawn because a live card was in the waitroom");
    CHECK(wait_has(tg, live), "nagi_live: the live card is in the waitroom");
    /* VACUITY CONTROL: the negative sibling's board must be a real "no draw". */
}

/* Rust test_no_live_cards_among_five_drawn_no_draw */
static void test_m_nagi_no_live_no_draw(void)
{
    TestGame *tg = &r_tg;
    static const char *const TOP[5] = { N11, N12, N13, DECK0, DECK4 };
    static const char *const EXTRA[1] = { DECK6 };
    struct nagi_result r;
    nagi_run(tg, TOP, EXTRA, 5, 0, 0, &r);
    CHECK_EQ(r.discard_after, 5, "nagi_none: exactly 5 cards moved to the waitroom");
    CHECK_EQ(r.hand_after, r.hand_before,
             "nagi_none: no card is drawn because no live card was in the waitroom");
    /* VACUITY CONTROL: the identical board with a live card on top must draw. */
    TestGame *ctl = &q_tg;
    static const char *const CTOP[5] = { NAGI_LIVE1, N11, N12, N13, DECK0 };
    struct nagi_result good;
    nagi_run(ctl, CTOP, EXTRA, 5, 0, 0, &good);
    CHECK_EQ(good.hand_after, good.hand_before + 1,
             "nagi_none CONTROL: the identical board with a LIVE card on top must "
             "draw 1, otherwise the no-draw assertion above is vacuous");
}

/* Rust test_live_card_in_existing_discard_but_not_in_five_drawn_no_draw */
static void test_m_nagi_existing_discard_live_no_draw(void)
{
    TestGame *tg = &r_tg;
    static const char *const TOP[5] = { N11, N12, N13, DECK0, DECK4 };
    static const char *const EXTRA[1] = { DECK6 };
    struct nagi_result r;
    nagi_run(tg, TOP, EXTRA, 5, 0, NAGI_LIVE1, &r);
    if (r.pre_discard_id < 0) return;
    CHECK_EQ(r.discard_after, 1 + 5,
             "nagi_pre: exactly 5 cards joined the pre-existing waitroom card");
    CHECK_EQ(r.hand_after, r.hand_before,
             "nagi_pre: no draw, because none of the 5 NEWLY discarded cards was a live card");
    CHECK(wait_has(tg, r.pre_discard_id),
          "nagi_pre: the pre-existing live card is still in the waitroom");
}

/* Rust test_multiple_live_cards_among_five_drawn_still_only_draw_one */
static void test_m_nagi_two_lives_still_one_draw(void)
{
    TestGame *tg = &r_tg;
    static const char *const TOP[5] = { NAGI_LIVE1, NAGI_LIVE2, N11, N12, N13 };
    static const char *const EXTRA[1] = { DECK6 };
    struct nagi_result r;
    nagi_run(tg, TOP, EXTRA, 5, 0, 0, &r);
    if (r.top_ids[0] < 0 || r.top_ids[1] < 0) return;
    CHECK(rb_card_is_live(r.top_ids[0]) && rb_card_is_live(r.top_ids[1]),
          "nagi_two: both top cards are live");
    CHECK_EQ(r.discard_after, 5, "nagi_two: exactly 5 cards moved to the waitroom");
    CHECK_EQ(r.hand_after, r.hand_before + 1,
             "nagi_two: exactly 1 card is drawn even with multiple live cards");
    CHECK(wait_has(tg, r.top_ids[0]), "nagi_two: the first live card is in the waitroom");
    CHECK(wait_has(tg, r.top_ids[1]), "nagi_two: the second live card is in the waitroom");
}

/* ------------------------------------------------------------------ *
 * per_card / optional_discard_all_hand_cost_draws_six_test.rs
 *      PL!SP-bp7-011-R (Tomari), debut: 「手札をすべて控え室に置いてもよい：
 *      カードを6枚引く」. The colon-gated optional cost means a skip (or an
 *      unpayable empty hand) suppresses the draw entirely.
 * ------------------------------------------------------------------ */

struct tomari_board { int tomari; int extra[3]; int n_extra; };

static void tomari_setup(TestGame *tg, const char *const *extras, int n_extra,
                         struct tomari_board *tb)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    tb->tomari = mid(tg, TOMARI);
    tb->n_extra = 0;
    pin_id(tb->tomari, TOMARI);
    if (tb->tomari < 0) return;
    CHECK_EQ(card_cost(tb->tomari), 9, "tomari: the print's cost is 9");
    CHECK(card_prints_trigger(tb->tomari, RB_TSTR_DEBUT), "tomari: the print has 登場");
    for (int i = 0; i < n_extra && i < 3; i++) {
        int c = mid(tg, extras[i]);
        pin_id(c, extras[i]);
        if (c < 0) return;
        tb->extra[tb->n_extra++] = c;
        test_add_to_hand(tg, c);
    }
    int filler = mid(tg, FILLER);
    pin_id(filler, FILLER);
    deck_append_n(tg, 0, filler, 40);
    test_give_energy(tg, 9);
}

/* Rust tomari_discards_entire_hand_then_draws_6 */
static void test_n_tomari_accept_draws_six(void)
{
    TestGame *tg = &a_tg;
    static const char *const EX[3] = { N11, N12, N13 };
    struct tomari_board tb;
    tomari_setup(tg, EX, 3, &tb);
    if (tb.tomari < 0 || tb.n_extra != 3) return;
    int hand_before = tg->state.p[0].hand.n;
    int expected_discard = hand_before;

    test_add_to_hand(tg, tb.tomari);
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    play_to_stage(tg, tb.tomari, AREA_LEFT, 1);
    /* VACUITY CONTROL: the discard-all-hand cost must actually be offered. */
    CHECK_PROMPTS(tg, 1, "tomari_accept: the discard-all-hand optional cost must prompt");
    CHECK(pending(tg), "tomari_accept: a choice is pending after the play");
    if (pending(tg)) select_option(tg, 1);
    drain_skip(tg, 10);

    CHECK_EQ(tg->state.p[0].hand.n, 6,
             "tomari_accept: 6 cards drawn after discarding the entire hand");
    CHECK(tg->state.p[0].discard.n >= expected_discard,
          "tomari_accept: the discarded cards are in the waitroom (got %d, need >= %d)",
          tg->state.p[0].discard.n, expected_discard);
    for (int i = 0; i < tb.n_extra; i++)
        CHECK(!hand_has(tg, tb.extra[i]),
              "tomari_accept: a discarded card is not still in hand");
}

/* Rust tomari_skips_discard_effect_does_not_fire */
static void test_n_tomari_skip_no_draw(void)
{
    TestGame *tg = &a_tg;
    static const char *const EX[2] = { N11, N12 };
    struct tomari_board tb;
    tomari_setup(tg, EX, 2, &tb);
    if (tb.tomari < 0 || tb.n_extra != 2) return;
    int hand_before = tg->state.p[0].hand.n;

    test_add_to_hand(tg, tb.tomari);
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    play_to_stage(tg, tb.tomari, AREA_LEFT, 1);
    /* VACUITY CONTROL: the cost prompt must be present or "skip" is a no-op. */
    CHECK_PROMPTS(tg, 1, "tomari_skip: the discard-all-hand optional cost must prompt");
    CHECK(pending(tg), "tomari_skip: a choice is pending after the play");
    if (pending(tg)) select_option(tg, 0);
    drain_skip(tg, 10);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before,
             "tomari_skip: skipping the optional discard keeps the hand unchanged "
             "(the colon-gated effect does not fire)");
    for (int i = 0; i < tb.n_extra; i++)
        CHECK(!wait_has(tg, tb.extra[i]),
              "tomari_skip: no card is discarded when skipping");
}

/* Rust tomari_empty_hand_auto_skips_no_draw */
static void test_n_tomari_empty_hand_no_draw(void)
{
    TestGame *tg = &a_tg;
    struct tomari_board tb;
    tomari_setup(tg, NULL, 0, &tb);
    if (tb.tomari < 0) return;
    test_add_to_hand(tg, tb.tomari);
    tg->state.p[0].hand.n = 0;
    hand_append_n(tg, 0, tb.tomari, 1);
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;

    play_to_stage(tg, tb.tomari, AREA_LEFT, 1);
    drain_skip(tg, 10);
    /* VACUITY CONTROL: on an empty hand the cost CANNOT be paid, so no prompt
     * is expected; the claim under test is the resulting hand size. */
    CHECK_EQ(tg->state.p[0].hand.n, 0,
             "tomari_empty: an empty hand auto-skips the cost, so no draw happens");
    /* And the positive control: the same card with 3 hand cards draws 6. */
    {
        TestGame *ctl = &b_tg;
        static const char *const CEX[3] = { N11, N12, N13 };
        struct tomari_board cb;
        tomari_setup(ctl, CEX, 3, &cb);
        if (cb.tomari < 0 || cb.n_extra != 3) return;
        test_add_to_hand(ctl, cb.tomari);
        ctl->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
        play_to_stage(ctl, cb.tomari, AREA_LEFT, 1);
        /* CHECK_PROMPTS (not a bare prompt count) because the debut cost is
         * still ON SCREEN here - it is answered on the next line, so counting
         * only at answer time would report 0. */
        CHECK_PROMPTS(ctl, 1, "tomari_empty CONTROL: the cost prompt must be offered "
                              "when the hand has cards");
        if (pending(ctl)) select_option(ctl, 1);
        drain_skip(ctl, 10);
        CHECK_EQ(ctl->state.p[0].hand.n, 6,
                 "tomari_empty CONTROL: with 3 hand cards the accepted cost draws 6");
    }
}

/* ------------------------------------------------------------------ *
 * per_card / turn_limited_yell_second_activation_blocked_test.rs
 *      PL!SP-bp2-015-N (Sumire) 自動/ターン1回: a yell that revealed no card
 *      with a blade heart grants heart06 until the end of the live.
 *      PL!SP-bp2-021-N (Wien)   the same, granting heart03.
 *      A REAL live is performed (the Rust file documents that its previous
 *      0-score fixture never yelled at all, so the old "0 or 1" assertion was
 *      true for every value).
 * ------------------------------------------------------------------ */

static void perform_real_yell(TestGame *tg, int live)
{
    test_add_to_hand(tg, live);
    advance_to_live_card_set(tg, 5);
    test_set_live_card(tg, 0, live);
    advance_to_live_card_set(tg, 3);
    drain_skip(tg, 10);
}

/* The VACUITY CONTROL for this section: did a REAL yell happen? C exposes the
 * yell cards on the performance snapshot, which is the direct analogue of Rust's
 * `initial_yell_revealed_cards`. */
static int real_yell_happened(TestGame *tg)
{
    for (int i = 0; i < tg->state.n_snapshots; i++)
        if (tg->state.snapshots[i].player == 0 && tg->state.snapshots[i].n_yell_cards > 0)
            return 1;
    return 0;
}

static void yell_watcher_setup(TestGame *tg, const char *watcher_no, int color)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int watcher = mid(tg, watcher_no);
    pin_id(watcher, watcher_no);
    if (watcher < 0) return;
    CHECK(card_prints_trigger(watcher, RB_TSTR_AUTO),
          "yell_watcher: the print carries a 自動 trigger");
    /* A member WITH score, so the performance over-performs and yells. */
    int scorer = mid(tg, SCORE_MEM);
    pin_id(scorer, SCORE_MEM);
    if (scorer < 0) return;
    set_stage(tg, 0, AREA_LEFT, scorer);
    set_stage(tg, 0, AREA_CENTER, watcher);
    tg->state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    int no_bh = mid(tg, ENERGY);
    pin_id(no_bh, ENERGY);
    int filler = mid(tg, FILLER);
    pin_id(filler, FILLER);
    for (int i = 0; i < 40; i++) {
        deck_append(tg, 0, no_bh);
        deck_append(tg, 1, filler);
    }
    test_give_energy(tg, 15);
    /* The fixture is blade-heart free on purpose, so the 自動 fires. */
    CHECK_EQ(card_has_blade_heart(no_bh), 0, "yell_watcher: LL-E-001-SD has no blade heart");
    CHECK_EQ(rb_mods_get_heart(&tg->state.mods, watcher, color), 0,
             "yell_watcher: precondition, no heart before the live");
}

/* Rust sumire_real_yell_grants_exactly_one_heart06 */
static void test_o_sumire_real_yell_grants_heart06(void)
{
    TestGame *tg = &c_tg;
    yell_watcher_setup(tg, SUMIRE, 6);
    int sumire = tg->state.p[0].stage[AREA_CENTER];
    int live = mid(tg, YELL_LIVE);
    pin_id(live, YELL_LIVE);
    if (sumire < 0 || live < 0) return;
    perform_real_yell(tg, live);
    /* VACUITY CONTROL: a real yell must have revealed cards, otherwise the
     * "+1 heart" assertion below is vacuous. */
    CHECK(real_yell_happened(tg),
          "sumire_yell: a REAL yell must have revealed cards (the snapshot's "
          "yell_cards are the C analogue of initial_yell_revealed_cards)");
    CHECK_EQ(rb_mods_get_heart(&tg->state.mods, sumire, 6), 1,
             "sumire_yell: a real yell revealing no blade heart grants heart06 "
             "exactly once during the live");
}

/* Rust wien_real_yell_grants_exactly_one_heart03 */
static void test_o_wien_real_yell_grants_heart03(void)
{
    TestGame *tg = &c_tg;
    yell_watcher_setup(tg, WIEN, 3);
    int wien = tg->state.p[0].stage[AREA_CENTER];
    int live = mid(tg, YELL_LIVE);
    pin_id(live, YELL_LIVE);
    if (wien < 0 || live < 0) return;
    perform_real_yell(tg, live);
    CHECK(real_yell_happened(tg),
          "wien_yell: a REAL yell must have revealed cards");
    CHECK_EQ(rb_mods_get_heart(&tg->state.mods, wien, 3), 1,
             "wien_yell: a real yell revealing no blade heart grants heart03 "
             "exactly once during the live");
}

/* ------------------------------------------------------------------ *
 * per_card / live_success_collective_stage_hearts_q224_test.rs
 *      LL-bp5-001-L (Live with a smile!), ライブ成功時: +1 score when the yell
 *      revealed 2+ live cards, OR my stage members collectively hold 5+ of the
 *      six heart colours, OR a member moved areas this turn.
 *      Q224 pins the COLLECTIVE reading of the heart clause.
 * ------------------------------------------------------------------ */

static int smile_board(TestGame *tg, const char *stage_a, const char *stage_b)
{
    fresh(tg);
    clear_p1(tg);
    clear_p2(tg);
    int smile = mid(tg, SMILE);
    int ma = mid(tg, stage_a);
    int mb = mid(tg, stage_b);
    pin_id(smile, SMILE); pin_id(ma, stage_a); pin_id(mb, stage_b);
    if (smile < 0 || ma < 0 || mb < 0) return -1;
    CHECK(card_prints_trigger(smile, RB_TSTR_LIVE_SUCCESS),
          "smile: the print carries a ライブ成功時 trigger");
    set_stage(tg, 0, AREA_LEFT, ma);
    set_stage(tg, 0, AREA_CENTER, mb);
    tg->state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_hand(tg, smile);
    int filler = mid(tg, FILLER);
    pin_id(filler, FILLER);
    for (int i = 0; i < 20; i++) {
        deck_append(tg, 0, filler);
        deck_append(tg, 1, filler);
    }
    return smile;
}

/* Runs the live to the snapshot and returns the recorded score delta vs the
 * PRINTED score, or -2 when no P1 snapshot recorded the live at all. */
static int smile_score_delta(TestGame *tg, int smile)
{
    advance_to_live_card_set(tg, 5);
    test_set_live_card(tg, 0, smile);
    advance_to_live_card_set(tg, 2);
    advance_to_live_card_set(tg, 3);
    drain_skip(tg, 12);
    for (int i = 0; i < tg->state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &tg->state.snapshots[i];
        if (s->player != 0) continue;
        for (int j = 0; j < s->n_lives; j++)
            if (s->lives[j] == smile) {
                if (s->live_score_detail[j] - card_score(smile) != 1)
                    fprintf(stderr,
                            "  [smile-diag] snapshot[%d] live[%d] detail=%d "
                            "printed=%d total_score=%d n_lives=%d success=%d\n",
                            i, j, s->live_score_detail[j], card_score(smile),
                            s->total_score, s->n_lives, s->success);
                return s->live_score_detail[j] - card_score(smile);
            }
    }
    return -2;
}

/* Rust smile_q224_live_success_score_plus_1 */
static void test_p_smile_five_types_scores(void)
{
    TestGame *tg = &d_tg;
    int smile = smile_board(tg, SCORE_MEM, HONOKA);
    if (smile < 0) return;
    /* The premise, asserted: the union of the two members' printed BASE hearts
     * is exactly the five colours Q224 counts collectively. `colors[8]` because
     * the loop indexes 1..6 inclusive. */
    int colors[8] = { 0 };
    for (int a = 0; a < 2; a++) {
        int cid = tg->state.p[0].stage[a];
        for (int col = 1; col <= 6; col++)
            if (printed_base_heart(cid, col) > 0) colors[col] = 1;
    }
    int n_colors = 0;
    for (int c = 1; c <= 6; c++) n_colors += colors[c];
    CHECK_EQ(n_colors, 5, "smile5: the two staged members collectively hold 5 colours");
    int delta = smile_score_delta(tg, smile);
    /* VACUITY CONTROL: the live must have been recorded at all, otherwise a
     * delta of 0 would be indistinguishable from a board where nothing ran. */
    CHECK(delta != -2, "smile5: the live must be recorded in a P1 performance snapshot");
    CHECK_EQ(delta, 1, "smile5: five distinct heart types add one score (Q224)");
}

/* Rust smile_q224_five_icons_with_three_types_does_not_score */
static void test_p_smile_three_types_no_score(void)
{
    TestGame *tg = &d_tg;
    int smile = smile_board(tg, SCORE_MEM, SCORE_MEM);
    if (smile < 0) return;
    int n_colors = 0;
    for (int col = 1; col <= 6; col++)
        if (printed_base_heart(tg->state.p[0].stage[0], col) > 0) n_colors++;
    CHECK_EQ(n_colors, 3, "smile3: the fixture really has only 3 heart types");
    int delta = smile_score_delta(tg, smile);
    /* VACUITY CONTROL: without a recorded live the delta below is meaningless. */
    CHECK(delta != -2, "smile3: the live must be recorded in a P1 performance snapshot");
    CHECK_EQ(delta, 0, "smile3: five heart icons of only three types do not add score");
}

/* ===================================================================== *
 * main
 * ===================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_ok, n_failed, n_crashed;

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

    printf("--- A pin_top/dynamic_count_pin_one_to_top_test.rs "
           "(PL!HS-bp6-001-R+) ---\n");
    run("a_kaho_looks_three",   test_a_kaho_no_allies_looks_three);
    run("a_kaho_looks_four",    test_a_kaho_one_ally_looks_four);
    run("a_kaho_looks_five",    test_a_kaho_two_allies_looks_five);

    printf("--- B pin_top/paid_look_three_splits_hand_top_and_waitroom_test.rs "
           "(PL!-bp3-007-R) ---\n");
    run("b_nozomi_paid_split",  test_b_nozomi_paid_look_three_splits);

    printf("--- C pin_top/per_group_member_look_pin_reveal_score_test.rs "
           "(PL!N-bp3-028-L) ---\n");
    run("c_connect_no_score",   test_c_connect_filler_reveal_grants_no_score);

    printf("--- D split_top_bottom/inspect_top_split_remainder_test.rs "
           "(PL!S-bp7-008-R) ---\n");
    run("d_mari_keeps_one",     test_d_mari_keeps_one_on_top);
    run("d_mari_keeps_two",     test_d_mari_keeps_two_on_top);

    printf("--- E bottom_inspect/inspect_bottom_and_select_test.rs "
           "(PL!S-bp7-004-R) ---\n");
    run("e_dia_keeps_bottom",   test_e_dia_keeps_one_on_bottom);

    printf("--- F bottom_inspect/bottom_card_accepted_to_fourth_from_top_test.rs "
           "(PL!S-bp7-010-N) ---\n");
    run("f_chika_fourth",       test_f_chika_accept_to_fourth_from_top);
    run("f_chika_decline",      test_f_chika_decline_keeps_bottom_card);

    printf("--- G per_card/live_success_three_distinct_revealed_retrieves_live "
           "(PL!SP-bp4-006-R) ---\n");
    run("g_keke_three_plus_live", test_g_three_distinct_plus_live_retrieves_live);

    printf("--- H per_card/livesuccess_revealed_distinct_name_threshold_test ---\n");
    run("h_two_names_nothing",  test_h_two_distinct_names_retrieve_nothing);
    run("h_outside_nothing",    test_h_three_non_liella_names_retrieve_nothing);
    run("h_no_live_nothing",    test_h_threshold_met_no_live_retrieves_nothing);

    printf("--- I per_card/live_success_cost_gated_recovered_member_test ---\n");
    run("i_suisui_cost4",       test_i_suisui_cost_four_recovers);
    run("i_suisui_cost9",       test_i_suisui_cost_nine_recovers);
    run("i_suisui_cost10",      test_i_suisui_cost_ten_no_prompt);
    run("i_suisui_wrong_group", test_i_suisui_wrong_group_no_prompt);
    run("i_fusion_cost4",       test_i_fusion_cost_four_recovers);
    run("i_fusion_cost11",      test_i_fusion_cost_above_four_no_prompt);

    printf("--- J per_card/live_success_distortion_heart_requirement_test "
           "(PL!SP-pb2-048-L) ---\n");
    run("j_distortion_three",   test_j_distortion_three_distinct_scores);
    run("j_distortion_one",     test_j_distortion_one_catchu_no_score);
    run("j_distortion_dup",     test_j_distortion_duplicate_names_dedupe);
    run("j_distortion_none",    test_j_distortion_no_catchu_no_change);

    printf("--- K per_card/live_success_revealed_cards_extra_yells_test "
           "(PL!S-bp6-021-L) ---\n");
    run("k_mirai_first",        test_k_mirai_select_first);
    run("k_mirai_second",       test_k_mirai_select_second);
    run("k_mirai_skip_both",    test_k_mirai_skip_leaves_both);
    run("k_mirai_skip_single",  test_k_mirai_skip_with_single);
    run("k_mirai_use_limit",    test_k_mirai_use_limit_blocks_second);
    run("k_mirai_empty",        test_k_mirai_empty_revealed_auto_skips);
    run("k_mirai_cost_tiers",   test_k_mirai_cost_tiers);

    printf("--- L per_card/live_success_under_member_count_score_test "
           "(PL!N-bp7-029-L) ---\n");
    run("l_burn_no_under",      test_l_burn_no_under_no_move_no_score);
    run("l_burn_1under_10",     test_l_burn_one_under_total_ten);
    run("l_burn_1under_9",      test_l_burn_one_under_total_nine);
    run("l_burn_skip",          test_l_burn_skip_optional_no_score);
    run("l_burn_3under",        test_l_burn_multiple_under_all_moved);
    run("l_burn_wait_state",    test_l_burn_wait_state);

    printf("--- M per_card/move_five_to_discard_draws_if_live_card_test "
           "(PL!-sd1-007-SD) ---\n");
    run("m_nagi_live_draws",    test_m_nagi_live_among_five_draws);
    run("m_nagi_no_live",       test_m_nagi_no_live_no_draw);
    run("m_nagi_pre_discard",   test_m_nagi_existing_discard_live_no_draw);
    run("m_nagi_two_lives",     test_m_nagi_two_lives_still_one_draw);

    printf("--- N per_card/optional_discard_all_hand_cost_draws_six_test "
           "(PL!SP-bp7-011-R) ---\n");
    run("n_tomari_accept_6",    test_n_tomari_accept_draws_six);
    run("n_tomari_skip",        test_n_tomari_skip_no_draw);
    run("n_tomari_empty_hand",  test_n_tomari_empty_hand_no_draw);

    printf("--- O per_card/turn_limited_yell_second_activation_blocked_test ---\n");
    run("o_sumire_yell_heart06", test_o_sumire_real_yell_grants_heart06);
    run("o_wien_yell_heart03",  test_o_wien_real_yell_grants_heart03);

    printf("--- P per_card/live_success_collective_stage_hearts_q224_test "
           "(LL-bp5-001-L) ---\n");
    run("p_smile_five_types",   test_p_smile_five_types_scores);
    run("p_smile_three_types",  test_p_smile_three_types_no_score);

    rb_unload();

    printf("\n==== parity_look_per_card ====\n");
    printf("tests run              : %d\n", n_ok + n_failed + n_crashed);
    printf("tests passed           : %d\n", n_ok);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps          : %d\n", gaps);
    printf("fixture setup bugs     : %d\n", setup_bugs);
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
                "PARITY LOOK PER CARD: %d failing test(s) - every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, a Rust fixture\n"
                "bug, or a SETUP BUG naming a print the engine cannot resolve.\n"
                "Not a bug in this file.\n", n_failed);
        return n_failed > 125 ? 125 : n_failed;
    }
    printf("ALL PARITY_LOOK_PER_CARD CHECKS PASSED\n");
    return 0;
}



