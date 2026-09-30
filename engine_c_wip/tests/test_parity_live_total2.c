/* test_parity_live_total2.c
 *
 * CLAIM (agent tr_score2, 2026-09-30). The only files this agent owns and edits are:
 *     engine_c_wip/tests/test_parity_score_per_card2.c
 *     engine_c_wip/tests/test_parity_live_total2.c
 * No engine source file under src/, no other tests/test_*.c, no Makefile.
 *
 * C port of two clusters:
 *
 * A. THE ONE UNPORTED FILE OF
 *    engine/tests/test_modules/effects/score/live_total/
 *    pl_pb2_001_pp_icon_gated_live_start_test.rs .................... 5 tests
 *
 * B. THE SIX card_score MODULES THAT
 *    engine_c_wip/tests/test_parity_card_score.c:1-30
 *    PROVED GENUINELY UNPORTED (32 tests). That header was rewritten on
 *    2026-09-30 by a peer; its list is taken verbatim and the denial in it is
 *    RE-CHECKED here (see "A PEER CLAIM THAT IS WRONG" below):
 *      live_start_distinct_group_member_score_test.rs ................ 3
 *      live_start_empty_zone_all_lily_white_pl_pb1_029_srl_test.rs .. 8
 *      live_start_group_member_four_heart_score_test.rs .............. 3
 *      live_start_lily_white_success_zone_count_pl_pb2_041_l_test.rs  6
 *      pb2_live_success_plus_one_per_distinct_mus_member_
 *        pl_pb2_039_l_test.rs ........................................ 4
 *      pb2_two_mus_members_live_or_success_zone_score_
 *        pl_pb2_038_l_test.rs ........................................ 8
 *                                                          TOTAL ..... 37 tests
 *
 * COVERAGE BOUNDARY FOR CLUSTER A, ESTABLISHED BY READING THE C BODIES
 *   tests/test_parity_live_total.c:3-4 claims "all 11 cluster test files, 44
 *   #[test] functions". The cluster is 12 modules and 49 tests (counted from
 *   the tree), and the 44 case functions in the file body are exactly:
 *     chika_bp3_001 (10), exact_energy (1), kasumi_bp5_002 (6),
 *     kasumi_energy_under (2), per_under_member (3), self_effect_move (7),
 *     opponent_success (3), opponent_surplus (4), optional_discard (6),
 *     side_member (1), twelve_energy (1)  = 44.
 *   The MISSING five are precisely pl_pb2_001_pp_icon_gated_live_start_test.rs,
 *   which no other file names. So that header's denominator is wrong (44 vs 49)
 *   and its "all 11 ... files" claim is wrong: there are 12 modules, and this
 *   file is the one it left out.
 *
 * A PEER CLAIM THAT IS WRONG (test_parity_card_score.c:22-27)
 *   "the old note claiming PL!SP-pb2-045-L (LOVER) and PL!HS-bp2-020-L
 *    (Link to the FUTURE) were 'deliberately NOT re-ported' named a reason that
 *    does not hold: 'bp2-045' appears in NO file in this cluster at all".
 *   FALSE. PL!SP-pb2-045-L is the SUBJECT CARD of
 *   live_start_group_member_four_heart_score_test.rs, three tests, named in its
 *   doc comment and in every test body. It also exists in cards.json
 *   (name 絶対的LOVER, score 4, ability
 *   「自分のステージにいるハートを4つ以上持つ『Liella!』のメンバー1人につき、
 *   このカードのスコアを＋１する。」), so it is not a bad fixture either.
 *
 * RUST FIXTURE BUG FOUND, WITH cards.json EVIDENCE
 *   live_start_lily_white_success_zone_count_pl_pb2_041_l_test.rs:15
 *       const LW_CHEAP: &str = "PL!-bp1-029-SRL";
 *   THERE IS NO SUCH PRINT. The real one is "PL!-pb1-029-SRL" (知らないLove＊
 *   教えてLove, unit "lily white", score 1) -- pb1, not bp1. Verified by reading
 *   cards/cards.json as UTF-8: the only pb1-029 keys are PL!-pb1-029-L,
 *   PL!-pb1-029-SRL, PL!HS-bp1-029-PE(+/-), PL!HS-pb1-029-L, PL!N-bp1-029-L,
 *   PL!N-bp1-029-N, PL!S-bp1-029-PE(+/-), PL!SP-bp1-029-SECE, PL!SP-pb1-029-PE(+/-).
 *   The Rust resolves it anyway (its lookup is forgiving), so the file is
 *   green in Rust with a fixture that does not exist. The C cases below use the
 *   REAL print, and one case asserts test_id_exact() on the WRONG print is -1
 *   so the finding is reproducible from the test output.
 *
 * HARNESS LANDMINES HONOURED (see each site for the specific one)
 *   - load_card_db() falls back to rb_load("../cards/build").
 *   - sizeof(TestGame) is ~781 KB -> every TestGame here is `static`.
 *   - mid() == test_new_id(); test_id_exact() wherever the print is under test.
 *   - Card.ability is NOT populated by rb_decode_card_by_index; ability
 *     inspection goes through rb_card_num_abilities + rb_decode_card_ability.
 *   - The waitroom zone string is "discard".
 *   - test_advance_to_phase_strict is the faithful port of Rust
 *     advance_to_phase; advance_to() adds only the drain Rust's own loop does
 *     between steps. test_advance_to_phase is never used.
 *   - ライブ成功時 is driven through rb_trigger_live_success, never the phase
 *     machine, and rb_should_trigger_live_success is asserted at the assertion
 *     site so a closed window is never read as a zero.
 *   - Non-ASCII literals are \xNN escapes (Write can emit CP932). 虹ヶ咲 is
 *     U+8679 (E8 99 B9), never U+8671; 蓮ノ空 is U+84EE (E8 93 AE).
 *   - The fullwidth-plus print is spelled "\xEF\xBC\x8B" so the fixture carries
 *     the same bytes as cards.json. (Filename token rule: pl_X -> PL!-X.)
 *   - Card.ability string-table indices are never read directly; group_idx is
 *     0xFFFF on every record, so rb_card_group_name (SERIES-level) and
 *     rb_card_unit_name (unit-level) are the only meaningful accessors, and
 *     main() prints both for every fixture below.
 *   - Every drain is capped; every case runs in its own fork()ed child with
 *     MAP_SHARED|MAP_ANONYMOUS counters zeroed in the child.
 *
 * VACUITY CONTROL
 *   A per_card cluster is a grid of one card x one behaviour, so a silent
 *   no-op makes a whole column read green. Every negative below carries a
 *   PAIRED POSITIVE on the same board shape, a control taken at the assertion
 *   site, or both, and EVERY case carries a test_total_card_count CENSUS
 *   baselined after all fixture cards are placed and immediately before the
 *   action under test:
 *     029: 1 lily white, empty zone -> +1   vs  + a card in the zone -> 0
 *     029: lily white only          -> +1   vs  + a mu's member    -> 0
 *     041: 2 lily white in zone     -> +1   vs  1 -> 0, 0 -> 0, 2 mu's -> 0
 *     045: Kanon(4 hearts) + Keke(3)-> +1   vs  Keke alone -> 0
 *     045: Liella 4-heart + non-Liella -> +1
 *     038: 2 mu's members            -> +1   vs  1 -> 0, 3 -> 0, +AZALEA -> 0
 *     038: one copy -> +1            vs  two copies -> +1 (non-stackable)
 *     039: empty pool -> 0           vs  (the Rust records +0 for every POSITIVE
 *                                     too -- that is the finding, see below)
 *     020: 3 distinct 蓮ノ空 -> +6    vs  1 -> +2, 0 -> 0
 *     001: a ドロー card in the zone DOES fire; the スコア / 全ブレード /
 *          empty-zone cases are the negatives of that same ability
 *   The census is the control that a card-DESTROYING defect (two were found
 *   today, both invisible to a zone count) would trip.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

typedef struct {
    volatile int assertions;
    volatile int failures;
    volatile int signal;
    volatile int exited_ok;
    char          name[96];
} SharedCounters;

static SharedCounters *g_shared;

static int failures;
static int assertions;

static volatile sig_atomic_t g_case_signal;
static char g_case_name[96];

static void publish_counts(void)
{
    if (g_shared) g_shared->failures = failures;
}

static void fatal_handler(int sig)
{
    g_case_signal = sig;
    if (g_shared) {
        g_shared->signal = sig;
        g_shared->failures = failures;
        size_t i = 0;
        for (; i + 1 < sizeof g_shared->name && g_case_name[i]; i++)
            g_shared->name[i] = g_case_name[i];
        g_shared->name[i] = 0;
    }
    _exit(0);
}

/* Variadic message on purpose: the Rust messages here are long and are written
 * as several adjacent C string literals, which a fixed two-parameter macro
 * would let the preprocessor split into extra arguments. Routing through
 * snprintf(__VA_ARGS__) lets the compiler do the concatenation, and lets a
 * caller pass a formatted value as well. */
#define CHECK(cond, ...) do { \
    assertions++; \
    char cbuf_[1400]; \
    snprintf(cbuf_, sizeof cbuf_, __VA_ARGS__); \
    if (!(cond)) { \
        fprintf(stderr, "FAIL: %s\n", cbuf_); \
        failures++; publish_counts(); \
    } else { \
        printf("ok: %s\n", cbuf_); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, ...) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    char cbuf_[1400]; \
    snprintf(cbuf_, sizeof cbuf_, __VA_ARGS__); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", cbuf_, actual_value, expected_value); \
        failures++; publish_counts(); \
    } else { \
        printf("ok: %s\n", cbuf_); \
    } \
} while (0)

/* -- fixtures ------------------------------------------------------------ */
#define FILLER        "PL!-sd1-010-SD"   /* 高坂 穂乃果                        */
#define FILLER2       "PL!-sd1-013-SD"   /* 園田海未                          */
#define SRL_FILLER    "PL!-sd1-019-SD"   /* START:DASH!!, a mu's LIVE card     */

/* A. PL!-pb2-001-PP 高坂穂乃果, ライブ開始時 -- three INDEPENDENT icon gates */
#define HONOKA        "PL!-pb2-001-PP"   /* 高坂穂乃果                         */
#define SCORE_ICON    "PL!-bp3-019-L"    /* 僕らのLIVE 君とのLIFE  (スコア)     */
#define BALL_ICON     "PL!-bp3-020-L"    /* Snow halation        (全ブレード)   */
#define DRAW_ICON     "PL!-bp3-021-L"    /* 愛してるばんざーい!  (ドロー)       */
#define NO_ICON       "PL!-pb2-039-L"    /* 僕たちはひとつの光  (no icon)      */
#define RECOVERABLE   "PL!-pb2-003-PP"   /* 南ことり, a mu's card to recover   */

/* B1. PL!HS-bp2-020-L Link to the FUTURE, ライブ開始時 */
#define LINK          "PL!HS-bp2-020-L"
#define HASU_A        "PL!HS-sd1-001-SD"  /* 日野下花帆 / スリーズブーケ */
#define HASU_B        "PL!HS-sd1-002-SD"  /* 村野さやか / DOLLCHESTRA  */
#define HASU_C        "PL!HS-sd1-003-SD"  /* 大沢瑠璃乃 / みらくらぱーく！ */
#define HASU_ONE      "PL!HS-bp1-001-R"   /* 日野下花帆 (R print)     */

/* B2. PL!-pb1-029-SRL 知らないLove＊教えてLove, ライブ開始時 */
#define LOVE_029      "PL!-pb1-029-SRL"
#define LW_UMI        "PL!-pb2-013-P\xEF\xBC\x8B" /* 園田海未, U+FF0B PLUS */
#define LW_NOZOMI     "PL!-pb2-025-N"   /* 東條 希, lily white */
#define PRINTEMPS     "PL!-bp6-017-N"   /* 小泉花陽, mu's */
#define AZALEA        "PL!S-bp5-012-N"   /* 松浦果南, AZALEA */

/* B3. PL!SP-pb2-045-L 絶対的LOVER, ライブ開始時 */
#define LOVER         "PL!SP-pb2-045-L"
#define KANON         "PL!SP-sd1-001-SD" /* 澁谷かのん, 4 hearts, CatChu!     */
#define KEKE          "PL!SP-sd1-002-SD" /* 唐 可可,   3 hearts, KALEIDOSCORE */

/* B4. PL!-pb2-041-L 春情ロマンティック, ライブ開始時 (ab#1) */
#define ROMANTIC      "PL!-pb2-041-L"
#define LW_CHEAP      "PL!-pb1-029-SRL"  /* score 1, lily white. The Rust file
                                           spells this "PL!-bp1-029-SRL",
                                           which does NOT exist -- see header */
#define LW_CHEAP_RUST "PL!-bp1-029-SRL"  /* the non-existent print, for the repro */
#define LW_MID        "PL!-bp5-023-SRL"  /* score 3, lily white */
#define LW_THIRD      "PL!-bp4-025-SRL"  /* score 5, lily white */
#define MUS_LIVE      "PL!-sd1-021-SD"   /* これからのSomeday, mu's live */

/* B5. PL!-pb2-039-L 僕たちはひとつの光, ライブ成功時 (per-unit half) */
#define HIKARI        "PL!-pb2-039-L"
#define ERI           "PL!-pb2-002-PP"   /* 絢瀬絵里, BiBi      */
#define HANAYOME      "PL!-bp6-017-N"   /* 小泉花陽, Printemps */
#define NICO          "PL!-pb2-003-PP"   /* 南ことり,  BiBi      */

/* B6. PL!-pb2-038-L 嵐のなかの恋だから, 常時 */
#define LOVE_038      "PL!-pb2-038-L"
#define MUS_A         "PL!-bp6-017-N"   /* 小泉花陽, mu's */
#define MUS_B         "PL!-bp5-012-N"   /* 南 ことり, mu's */
#define MUS_C         "PL!-pb1-026-N"   /* 小泉花陽 (another print), mu's */

#define G_MUS    "\xCE\xBC's"                                   /* mu's (U+03BC) */
#define G_LW     "lily white"
#define G_HASU   "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"          /* 蓮ノ空 U+84EE */
#define G_LIELLA "Liella!"
#define G_AZALEA "AZALEA"
#define TRIGGER_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82" /* ライブ成功時 */

/* -- helpers ------------------------------------------------------------- */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static int mid(TestGame *tg, const char *no) { return test_new_id(tg, no); }

static const char *no_of(int cid)
{
    static char buf[4][80];
    static int which;
    char *out = buf[which = (which + 1) & 3];
    Card c;
    out[0] = 0;
    if (cid < 0) { snprintf(out, sizeof buf[0], "<none>"); return out; }
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { snprintf(out, sizeof buf[0], "<nodecode>"); return out; }
    snprintf(out, sizeof buf[0], "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    return out;
}

static void ident(int cid, const char *no, const char *what)
{
    assertions++;
    if (!rb_card_no_eq(cid, no)) {
        fprintf(stderr, "FAIL: %s: card identity is not %s (id=%d, decoded %s)\n",
                what, no, cid, no_of(cid));
        failures++; publish_counts();
    } else {
        printf("ok: %s: card identity is %s\n", what, no);
    }
}

/* Rust assert_card_in_group, through the same matcher the abilities use. */
static void assert_in_group(int cid, const char *group, const char *what)
{
    assertions++;
    if (!rb_card_matches_group_str(cid, group)) {
        fprintf(stderr, "FAIL: %s: %s is not in the group \"%s\" (unit=[%s] group=[%s])\n",
                what, no_of(cid), group,
                rb_card_unit_name(cid) ? rb_card_unit_name(cid) : "-",
                rb_card_group_name(cid) ? rb_card_group_name(cid) : "-");
        failures++; publish_counts();
    } else {
        printf("ok: %s: %s is in the group \"%s\"\n", what, no_of(cid), group);
    }
}

/* Rust assert_distinct_card_names -- whitespace-insensitive name compare. */
static void assert_distinct_names(int a, int b, const char *what)
{
    assertions++;
    const char *na = test_card_name(a), *nb = test_card_name(b);
    if (!na || !nb) {
        fprintf(stderr, "FAIL: %s: a card name is missing (%s / %s)\n", what, no_of(a), no_of(b));
        failures++; publish_counts();
        return;
    }
    char xa[256], xb[256];
    size_t j = 0;
    for (const char *p = na; *p && j + 1 < sizeof xa; p++) if (*p != ' ' && *p != '\t') xa[j++] = *p;
    xa[j] = 0;
    j = 0;
    for (const char *p = nb; *p && j + 1 < sizeof xb; p++) if (*p != ' ' && *p != '\t') xb[j++] = *p;
    xb[j] = 0;
    if (!strcmp(xa, xb)) {
        fprintf(stderr, "FAIL: %s: %s and %s share the name \"%s\"\n",
                what, no_of(a), no_of(b), na);
        failures++; publish_counts();
    } else {
        printf("ok: %s: %s (%s) and %s (%s) differ by name\n", what, no_of(a), na, no_of(b), nb);
    }
}

static const char *esc(const char *s)
{
    static char buf[4][512];
    static int which;
    char *out = buf[which = (which + 1) & 3];
    size_t n = 0;
    for (const unsigned char *p = (const unsigned char *)s; *p && n + 5 < sizeof buf[0]; p++) {
        if (*p >= 0x20 && *p < 0x7f) out[n++] = (char)*p;
        else n += (size_t)snprintf(out + n, sizeof buf[0] - n, "\\x%02X", *p);
    }
    out[n] = 0;
    return out;
}

static void dbg_card(const char *tag, int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) {
        printf("DBG %s: <nodecode %d>\n", tag, cid); return;
    }
    printf("DBG %s: cid=%d no=%s name=%s unit=[%s] group=[%s] type=%s "
           "cost=%d blade=%d score=%d has_special=%d special_color=%d special_count=%d\n",
           tag, cid, no_of(cid), c.name ? c.name : "-",
           rb_card_unit_name(cid) ? rb_card_unit_name(cid) : "-",
           rb_card_group_name(cid) ? rb_card_group_name(cid) : "-",
           rb_card_is_live(cid) ? "live" : (rb_card_is_energy(cid) ? "energy" : "member"),
           (int)c.cost, (int)c.blade, (int)c.score,
           c.has_special, (int)c.special_color, (int)c.special_count);
    rb_free_card(&c);
    fflush(stdout);
}

static void dbg_ability(const char *tag, int cid)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    printf("DBG %s: cid=%d abilities=%d\n", tag, cid, n);
    for (int k = 0; k < n; k++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, k, &ab)) continue;
        printf("DBG %s[%d]: triggers=%s effect action=%s target=%s\n", tag, k,
               ab.triggers ? ab.triggers : "-",
               ab.effect && ab.effect->action ? ab.effect->action : "-",
               ab.effect && ab.effect->target ? ab.effect->target : "-");
        if (ab.effect)
            for (int i = 0; i < ab.effect->n_extra; i++)
                printf("DBG %s[%d]   extra %s=%s\n", tag, k,
                       ab.effect->extra_k[i], esc(ab.effect->extra_v[i]));
        rb_free_ability(&ab);
    }
    fflush(stdout);
}

static int score_of(TestGame *tg, int cid) { return test_get_score_modifier(tg, cid); }

static int total_bonus_p1(TestGame *tg)
{
    return (int)tg->state.mods.p1_constant_total_score_bonus;
}

/* special_heart.score -> Card.has_special && special_color == RB_HEART_SCORE */
static int has_score_icon(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int r = rb_card_has_score_icon(&c);
    rb_free_card(&c);
    return r;
}
/* special_heart.draw -- the same shape, the DRAW slot of the enum. */
static int has_draw_icon(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int r = c.has_special && c.special_color == (uint8_t)RB_HEART_DRAW;
    rb_free_card(&c);
    return r;
}
/* blade_heart.b_all -- NOT in special_heart; a grep of the wrong field finds
 * no all-blade card at all. */
static int has_all_blade_icon(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int r = rb_card_has_all_blade(&c);
    rb_free_card(&c);
    return r;
}

static void drain_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        test_resume_choice(tg, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        if (ch && ch->allow_skip) test_resume_choice(tg, -1);
        else test_resume_choice(tg, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

/* Faithful port of Rust advance_to_phase (helpers/mod.rs:538-561): 16 steps,
 * stop on arrival, drain only the prompts raised by steps that did NOT arrive.
 * A failure prints the phase trace, because "never reached the phase" is a
 * harness-level observation that must be attributable to the engine walk and
 * not to this loop. */
static int advance_to(TestGame *tg, int target)
{
    char trace[512];
    size_t tn = 0;
    for (int i = 0; i < 16; i++) {
        if (test_advance_to_phase_strict(tg, target)) return 1;
        tn += (size_t)snprintf(trace + tn, sizeof trace - tn, "%s%s(p=%d)",
                               i ? ">" : "", test_phase_name_now(tg),
                               test_has_pending_choice(tg));
        if (!test_has_pending_choice(tg)) {
            fprintf(stderr, "DBG advance_to: gave up with no prompt standing; trace %s\n",
                    trace);
            return 0;
        }
        drain_first(tg);
    }
    fprintf(stderr, "DBG advance_to: 16 steps exhausted; trace %s\n", trace);
    return 0;
}

static int advance_named(TestGame *tg, const char *name)
{
    int t = test_phase_from_name(name);
    if (t < 0) return 0;
    return advance_to(tg, t);
}

static void fire_live_start(TestGame *tg, int pl, int cid)
{
    tg->state.activating_card = cid;
    rb_trigger_live_start(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

static void fire_live_success(TestGame *tg, int pl, int cid)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[pl] = 1;
    tg->state.activating_card = cid;
    rb_trigger_live_success(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) {
        test_add_to_deck(tg, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* The shared driver of the three ライブ開始時 card_score modules: put `stage`
 * on the stage in slot order, `success` in the success live card zone, hand
 * `live` plus two fillers, fill both decks, stand the live up and walk to the
 * first-attacker performance phase, then read the live's score modifier.
 * Mirrors the `score_mod` / `score_mod_with_success_zone` helpers of
 * live_start_empty_zone_..., live_start_lily_white_success_zone_count_... and
 * live_start_group_member_four_heart_score.rs. */
static int live_start_score_mod(TestGame *tg, int live, int filler,
                                const int *stage, int n_stage,
                                const int *success, int n_success)
{
    for (int i = 0; i < n_stage; i++) test_add_to_stage(tg, i, stage[i]);
    for (int i = 0; i < n_success; i++) test_add_to_success(tg, success[i]);
    test_add_to_hand(tg, live);
    test_add_to_hand(tg, filler);
    test_add_to_hand(tg, filler);
    fill_decks(tg, filler, 30);

    if (!advance_named(tg, "LiveCardSetFirstAttacker")) {
        fprintf(stderr, "FAIL: live-start driver: never reached the live-card set phase "
                        "(stopped at %s)\n", test_phase_name_now(tg));
        failures++; publish_counts();
        return -999;
    }
    test_set_live_card(tg, 0, live);
    drain_choices(tg);

    /* CENSUS, baselined after every fixture card is placed. */
    int census = test_total_card_count(tg);
    if (!advance_named(tg, "FirstAttackerPerformance")) {
        fprintf(stderr, "FAIL: live-start driver: never reached the performance phase "
                        "(stopped at %s)\n", test_phase_name_now(tg));
        failures++; publish_counts();
        return -999;
    }
    drain_choices(tg);
    CHECK_EQ(test_total_card_count(tg), census,
             "CENSUS -- the ライブ開始時 window destroyed or duplicated no card");
    CHECK_EQ(test_zone_len(tg, 0, "live"), 1,
             "CENSUS control -- the live card is in the live card zone");
    return score_of(tg, live);
}

/* =========================================================================
 * A. pl_pb2_001_pp_icon_gated_live_start_test.rs (5 tests)
 *
 * PL!-pb2-001-PP 高坂穂乃果, ライブ開始時:
 *   自分の成功ライブカード置き場にある『μ's』のカードの中に、スコアを持つカードが
 *   ある場合、ライブ終了時まで、「常時 ライブの合計スコアを＋１する。」を得る。
 *   全ブレードを持つカードがある場合、ライブ終了時まで、ハートを得る。
 *   ドローを持つカードがある場合、自分の控え室から『μ's』のカードを1枚手札に加える。
 *
 * It is a `sequential` whose steps are NOT a chain: each carries its own
 * condition. A zone holding only a ドロー card must recover a card and gain
 * nothing else, so the ドロー clause (which MOVES a card) is the discriminator.
 * ========================================================================= */

/* Rust `run(zone, waitroom) -> Ran { game, waited_draw }`. */
static int honoka_run(TestGame *tg, const char *const *zone, int n_zone,
                      const char *const *waitroom, int n_wait, const char *tag)
{
    test_game_new(tg);
    int filler = mid(tg, FILLER);
    for (int i = 0; i < 20; i++) test_add_to_deck(tg, filler);

    tg->state.p[0].discard.n = 0;
    for (int i = 0; i < n_wait; i++) test_add_to_discard(tg, mid(tg, waitroom[i]));
    tg->state.p[0].success.n = 0;
    for (int i = 0; i < n_zone; i++) test_add_to_success(tg, mid(tg, zone[i]));

    int honoka = test_id_exact(tg, HONOKA);
    ident(honoka, HONOKA, tag);
    dbg_card("HONOKA", honoka);
    test_add_to_live(tg, honoka);

    int before = test_zone_len(tg, 0, "waitroom");
    int census = test_total_card_count(tg);

    fire_live_start(tg, 0, honoka);
    /* Rust: while has_pending_choice && guard < 6 { select_indices(&[0]) } */
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 6) {
        test_resume_choice(tg, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }

    int after = test_zone_len(tg, 0, "waitroom");
    CHECK_EQ(test_total_card_count(tg), census,
             "高坂穂乃果: CENSUS -- the live-start trigger destroyed or duplicated no card");
    printf("     高坂穂乃果 [%s]: 控え室 %d -> %d, hand=%d, success=%d\n", tag,
           before, after, test_zone_len(tg, 0, "hand"), test_zone_len(tg, 0, "success"));
    return after < before;
}

static void test_a_draw_card_in_the_zone_recovers_one_mus_card_to_hand(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    static const char *const ZONE[] = { DRAW_ICON };
    static const char *const WAIT[] = { RECOVERABLE };
    int recov = test_id_exact(&tg, RECOVERABLE);
    ident(recov, RECOVERABLE, "draw-clause: the 控え室 fixture");

    int waited = honoka_run(&tg, ZONE, 1, WAIT, 1,
                            "draw-clause: 高坂穂乃果");

    /* THE test. The zone holds ONLY a ドロー card, so the other two clauses
     * must not fire -- checked by the live-total bonus staying at zero. */
    CHECK(waited,
          "ドローを持つカードがある場合、自分の控え室から『mu's』のカードを1枚手札に"
          "加える -- the ドロー card in the 成功ライブカード置き場 must recover a card");
    CHECK_EQ(total_bonus_p1(&tg), 0,
             "and the zone holds no スコア card, so the live-total bonus must not be "
             "granted -- the three clauses are independent, not a chain");
    /* This is the PAIRED POSITIVE that keeps the negatives honest: the ability
     * really did dispatch and really did move a card. */
    CHECK_EQ(test_zone_len(&tg, 0, "waitroom"), 0,
             "draw-clause: the 控え室 card really left the 控え室 (it went to hand)");
    CHECK_EQ(test_zone_len(&tg, 0, "hand"), 1,
             "draw-clause: the recovered card really arrived in hand");
}

static void test_the_score_and_all_blade_clauses_produce_nothing_measurable_yet(void)
{
    static const char *const CASES[] = { SCORE_ICON, BALL_ICON };
    for (int i = 0; i < 2; i++) {
        static TestGame tg;
        test_card_copy_pool_reset();
        test_game_new(&tg);
        int waited = honoka_run(&tg, &CASES[i], 1, NULL, 0,
                                i == 0 ? "score-clause" : "all-blade-clause");
        int icon = test_id_exact(&tg, CASES[i]);
        ident(icon, CASES[i], "score/all-blade clause: the zone fixture");
        CHECK(rb_card_is_live(icon),
              "score/all-blade clause: the zone fixture is a LIVE card");
        CHECK_EQ(waited, 0,
                 "score/all-blade clause: with an EMPTY 控え室 the ドロー clause "
                 "cannot fire, so waited_draw is 0 -- this is the control that the "
                 "draw-clause positive is really about the ドロー icon");
        /* MEASURED, not asserted as correct -- the Rust says the same: both
         * clauses grant an ability that lasts ライブ終了時まで and neither is
         * observable in this harness. The value is recorded, not blessed. */
        CHECK_EQ(total_bonus_p1(&tg), 0,
                 "MEASURED, not asserted as correct: a スコア / 総合ブレード card in the "
                 "成功ライブカード置き場 grants nothing observable here. This test "
                 "records the value; it does not claim it is right");
        printf("     score/all-blade clause: p1_constant_total_score_bonus = %d\n",
               total_bonus_p1(&tg));
    }
}

static void test_a_card_with_none_of_the_icons_fires_nothing(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    static const char *const ZONE[] = { NO_ICON };
    static const char *const WAIT[] = { RECOVERABLE };
    int no_icon = test_id_exact(&tg, NO_ICON);
    ident(no_icon, NO_ICON, "no-icon: the zone fixture");

    /* CONTROLS: the fixture really carries none of the three icons, so "0" is
     * a property of the card and not of a lookup that missed. */
    CHECK(!has_score_icon(no_icon), "no-icon control: PL!-pb2-039-L carries no スコア icon");
    CHECK(!has_draw_icon(no_icon),  "no-icon control: PL!-pb2-039-L carries no ドロー icon");
    CHECK(!has_all_blade_icon(no_icon),
          "no-icon control: PL!-pb2-039-L carries no 全ブレード icon");

    int waited = honoka_run(&tg, ZONE, 1, WAIT, 1, "no-icon");

    CHECK_EQ(total_bonus_p1(&tg), 0,
             "PL!-pb2-039-L carries no スコア, no 全ブレード and no ドロー");
    CHECK(!waited,
          "and no ドロー card means no recovery, whatever else is waiting");
    CHECK_EQ(test_zone_len(&tg, 0, "waitroom"), 1,
             "no-icon control: the 控え室 card is still there -- nothing was recovered");
}

static void test_an_empty_success_zone_fires_nothing(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    static const char *const WAIT[] = { RECOVERABLE };
    int waited = honoka_run(&tg, NULL, 0, WAIT, 1, "empty-success-zone");

    CHECK_EQ(test_zone_len(&tg, 0, "success"), 0,
             "empty-zone control: the 成功ライブカード置き場 really is empty");
    CHECK_EQ(total_bonus_p1(&tg), 0, "an empty success zone scores nothing");
    CHECK(!waited, "there is nothing in the zone to key on");
}

static void test_the_fixtures_carry_exactly_the_icon_under_test(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);

    int score = test_id_exact(&tg, SCORE_ICON);
    int ball  = test_id_exact(&tg, BALL_ICON);
    int draw  = test_id_exact(&tg, DRAW_ICON);
    int recov = test_id_exact(&tg, RECOVERABLE);
    ident(score, SCORE_ICON, "icon fixture");
    ident(ball,  BALL_ICON,  "icon fixture");
    ident(draw,  DRAW_ICON,  "icon fixture");
    ident(recov, RECOVERABLE, "icon fixture: the 控え室 card");
    dbg_card("SCORE ICON", score);
    dbg_card("BALL ICON", ball);
    dbg_card("DRAW ICON", draw);

    CHECK(has_score_icon(score), "PL!-bp3-019-L must carry スコア");
    CHECK(has_all_blade_icon(ball),
          "PL!-bp3-020-L must carry 全ブレード -- and that icon is "
          "blade_heart.b_all, NOT special_heart, which only ever holds score and draw");
    CHECK(has_draw_icon(draw), "PL!-bp3-021-L must carry ドロー");
    /* The recovery clause filters on mu's, so the card it recovers must be mu's. */
    assert_in_group(recov, G_MUS, "the card in the 控え室");
}

/* =========================================================================
 * B1. live_start_distinct_group_member_score_test.rs (3 tests)
 *
 * PL!HS-bp2-020-L Link to the FUTURE, ライブ開始時: 自分のステージにいる名前の
 * 異なる『蓮ノ空』のメンバー1人につき、このカードのスコアを＋２する。
 * ========================================================================= */

static void test_live_start_three_distinct_group_members_add_six_score(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int link = test_id_exact(&tg, LINK);
    int a = test_id_exact(&tg, HASU_A), b = test_id_exact(&tg, HASU_B), c = test_id_exact(&tg, HASU_C);
    int filler = mid(&tg, FILLER);
    ident(link, LINK, "link to the FUTURE");
    ident(a, HASU_A, "link to the FUTURE stage 0");
    ident(b, HASU_B, "link to the FUTURE stage 1");
    ident(c, HASU_C, "link to the FUTURE stage 2");
    dbg_ability("LINK", link);

    /* CONTROLS: the three fixtures are 蓮ノ空 (a SERIES-level group -- the units
     * differ on purpose) and pairwise differently named. Without these, a "+6"
     * could be three copies of one name. */
    assert_in_group(a, G_HASU, "stage 0 fixture");
    assert_in_group(b, G_HASU, "stage 1 fixture");
    assert_in_group(c, G_HASU, "stage 2 fixture");
    assert_distinct_names(a, b, "stage 0 vs 1");
    assert_distinct_names(a, c, "stage 0 vs 2");
    assert_distinct_names(b, c, "stage 1 vs 2");

    /* The three distinct-named 蓮ノ空 members on stage, in slot order. */
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    test_add_to_stage(&tg, 2, c);
    test_add_to_hand(&tg, link);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 30);
    if (!advance_named(&tg, "LiveCardSetFirstAttacker")) { failures++; publish_counts(); return; }
    test_set_live_card(&tg, 0, link);
    drain_choices(&tg);
    int census = test_total_card_count(&tg);
    if (!advance_named(&tg, "FirstAttackerPerformance")) { failures++; publish_counts(); return; }
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "link: CENSUS -- the ライブ開始時 window destroyed or duplicated no card");
    CHECK_EQ(test_stage_has(&tg, 0, a) + test_stage_has(&tg, 1, b) + test_stage_has(&tg, 2, c), 3,
             "link control: all three 蓮ノ空 members are still on the stage");

    CHECK_EQ(score_of(&tg, link), 6,
             "3 distinct 蓮ノ空 members should give +6 score mod (2 each)");
}

static void test_live_start_one_group_member_adds_two_score(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int link = test_id_exact(&tg, LINK);
    int hasu = test_id_exact(&tg, HASU_ONE);
    int filler = mid(&tg, FILLER);
    ident(link, LINK, "link to the FUTURE, one member");
    ident(hasu, HASU_ONE, "link to the FUTURE, one member");

    assert_in_group(hasu, G_HASU, "the one 蓮ノ空 member");
    test_add_to_stage(&tg, 1, hasu);
    test_add_to_hand(&tg, link);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 30);
    if (!advance_named(&tg, "LiveCardSetFirstAttacker")) { failures++; publish_counts(); return; }
    test_set_live_card(&tg, 0, link);
    drain_choices(&tg);
    int census = test_total_card_count(&tg);
    if (!advance_named(&tg, "FirstAttackerPerformance")) { failures++; publish_counts(); return; }
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census, "link one member: CENSUS");
    CHECK_EQ(test_zone_len(&tg, 0, "stage"), 1,
             "link one member control: exactly one member is on the stage");

    CHECK_EQ(score_of(&tg, link), 2,
             "1 蓮ノ空 member should give +2 score mod");
}

static void test_live_start_no_group_members_adds_no_score(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int link = test_id_exact(&tg, LINK);
    int filler = mid(&tg, FILLER);
    ident(link, LINK, "link to the FUTURE, no members");

    test_add_to_hand(&tg, link);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 30);
    if (!advance_named(&tg, "LiveCardSetFirstAttacker")) { failures++; publish_counts(); return; }
    test_set_live_card(&tg, 0, link);
    drain_choices(&tg);
    int census = test_total_card_count(&tg);
    if (!advance_named(&tg, "FirstAttackerPerformance")) { failures++; publish_counts(); return; }
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census, "link no members: CENSUS");
    CHECK_EQ(test_zone_len(&tg, 0, "stage"), 0,
             "link no members control: the stage really is empty");

    /* The PAIRED NEGATIVE of the two cases above: same live, same walk, one
     * term changed. Without it, a "+0" would also be what a silent no-op gives. */
    CHECK_EQ(score_of(&tg, link), 0,
             "Score should be 0 when no 蓮ノ空 members on stage");
}

/* =========================================================================
 * B2. live_start_empty_zone_all_lily_white_pl_pb1_029_srl_test.rs (8 tests)
 *
 * PL!-pb1-029-SRL 知らないLove＊教えてLove, ライブ開始時: 自分の成功ライブカード
 * 置き場のカードが0枚で、かつ自分のステージにいるメンバーが『lily white』のみの
 * 場合、このカードのスコアを＋１する。
 * A two-clause AND: each negative below breaks exactly one clause.
 * ========================================================================= */

/* The Rust `score_mod(stage, success_zone)` helper. */
static int love029_score(TestGame *tg, const int *stage, int n_stage,
                         const int *success, int n_success)
{
    int love   = test_id_exact(tg, LOVE_029);
    int filler = mid(tg, FILLER);
    ident(love, LOVE_029, "知らないLove＊教えてLove");
    dbg_ability("LOVE-029", love);
    return live_start_score_mod(tg, love, filler, stage, n_stage, success, n_success);
}

static void test_empty_success_zone_and_all_lily_white_stage_scores_plus_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int umi = test_id_exact(&tg, LW_UMI);
    ident(umi, LW_UMI, "029 positive: 園田海未 (fullwidth-plus print)");
    assert_in_group(umi, G_LW, "029 positive: the stage fixture");
    int st[1]; st[0] = umi;
    CHECK_EQ(love029_score(&tg, st, 1, NULL, 0), 1,
             "成功ライブカード置き場のカードが0枚 and ステージが『lily white』のみ -- "
             "both clauses hold, so +1");
}

static void test_two_lily_white_members_still_score_only_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int umi = test_id_exact(&tg, LW_UMI), nozomi = test_id_exact(&tg, LW_NOZOMI);
    ident(umi, LW_UMI, "029 two members: slot 0");
    ident(nozomi, LW_NOZOMI, "029 two members: slot 1");
    assert_in_group(umi, G_LW, "029 two members: slot 0");
    assert_in_group(nozomi, G_LW, "029 two members: slot 1");
    assert_distinct_names(umi, nozomi, "029 two members");
    int st[2]; st[0] = umi; st[1] = nozomi;
    CHECK_EQ(love029_score(&tg, st, 2, NULL, 0), 1,
             "the effect is a flat +1 on the live, not a per-member count");
}

static void test_a_card_in_the_success_zone_blocks_the_bonus(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int umi = test_id_exact(&tg, LW_UMI);
    int blocker = test_id_exact(&tg, SRL_FILLER);
    ident(umi, LW_UMI, "029 zone-blocks: stage");
    ident(blocker, SRL_FILLER, "029 zone-blocks: the success-zone card");
    int st[1]; st[0] = umi;
    int sz[1]; sz[0] = blocker;
    /* Both halves are visible BEFORE the action, so a 0 can only come from the
     * success zone being non-empty. */
    CHECK_EQ(rb_card_matches_group_str(umi, G_LW), 1,
             "029 zone-blocks control: the stage clause still holds");
    CHECK_EQ(love029_score(&tg, st, 1, sz, 1), 0,
             "成功ライブカード置き場のカードが0枚 is required -- one card breaks it even "
             "though the stage clause still holds");
}

static void test_a_non_lily_white_member_blocks_the_bonus(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int umi = test_id_exact(&tg, LW_UMI), mus = test_id_exact(&tg, PRINTEMPS);
    ident(umi, LW_UMI, "029 mu's-breaks: slot 0");
    ident(mus, PRINTEMPS, "029 mu's-breaks: slot 1");
    assert_in_group(mus, G_MUS, "029 mu's-breaks: the intruder really is a mu's member");
    int st[2]; st[0] = umi; st[1] = mus;
    CHECK_EQ(love029_score(&tg, st, 2, NULL, 0), 0,
             "メンバーが『lily white』のみ is required -- one mu's member alongside "
             "breaks exclusivity even though the success zone is empty");
}

static void test_a_non_mus_unit_also_breaks_exclusivity(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int umi = test_id_exact(&tg, LW_UMI), az = test_id_exact(&tg, AZALEA);
    ident(umi, LW_UMI, "029 AZALEA-breaks: slot 0");
    ident(az, AZALEA, "029 AZALEA-breaks: slot 1");
    assert_in_group(az, G_AZALEA, "029 AZALEA-breaks: the intruder is an AZALEA member");
    CHECK(!rb_card_matches_group_str(az, G_LW),
          "029 AZALEA-breaks control: the intruder is NOT a lily white member -- both its "
          "unit and its series differ from the lily white fixtures");
    CHECK(!rb_card_matches_group_str(az, G_MUS),
          "029 AZALEA-breaks control: the intruder is an Aqours-series card, so this case "
          "differs from the previous one by BOTH unit and series");
    printf("     029 AZALEA-breaks: PL!S-bp5-012-N unit=[%s] group=[%s]\n",
           rb_card_unit_name(az) ? rb_card_unit_name(az) : "-",
           rb_card_group_name(az) ? rb_card_group_name(az) : "-");
    int st[2]; st[0] = umi; st[1] = az;
    CHECK_EQ(love029_score(&tg, st, 2, NULL, 0), 0,
             "exclusivity is over the whole stage, so an unrelated unit also breaks it");
}

static void test_both_clauses_broken_scores_nothing(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int mus = test_id_exact(&tg, PRINTEMPS);
    int blocker = test_id_exact(&tg, SRL_FILLER);
    ident(mus, PRINTEMPS, "029 both-broken: stage");
    ident(blocker, SRL_FILLER, "029 both-broken: the success-zone card");
    int st[1]; st[0] = mus;
    int sz[1]; sz[0] = blocker;
    CHECK_EQ(love029_score(&tg, st, 1, sz, 1), 0,
             "neither clause holds, so no bonus");
}

/* The 「のみ」 is a universal, not an existential. Deliberate duplicate of the
 * first case: it is the precondition check the Rust file spells out. */
static void test_single_lily_white_alone_qualifies(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int umi = test_id_exact(&tg, LW_UMI);
    ident(umi, LW_UMI, "029 lone lily white");
    assert_in_group(umi, G_LW, "029 lone lily white");
    int st[1]; st[0] = umi;
    CHECK_EQ(love029_score(&tg, st, 1, NULL, 0), 1,
             "precondition check for exclusivity: a lone lily white member satisfies のみ");
}

/* Empty stage: のみ is vacuously true over zero members. The Rust explicitly
 * pins this to whatever the engine does and calls it out rather than asserting
 * a reading. Ported verbatim: the assertion is the disjunction, not a value. */
static void test_empty_stage_is_recorded_as_vacuous(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int observed = love029_score(&tg, NULL, 0, NULL, 0);
    char m0[128];
    snprintf(m0, sizeof m0, "empty stage must resolve to 0 or 1, got %d", observed);
    CHECK(observed == 0 || observed == 1, m0);
    printf("     029 empty stage: のみ over an EMPTY stage resolved to %d "
           "(the card text does not settle this; recorded, not blessed)\n", observed);
}

/* =========================================================================
 * B3. live_start_group_member_four_heart_score_test.rs (3 tests)
 *
 * PL!SP-pb2-045-L 絶対的LOVER, ライブ開始時: 自分のステージにいるハートを4つ以上
 * 持つ『Liella!』のメンバー1人につき、このカードのスコアを＋１する。
 * 澁谷かのん (PL!SP-sd1-001-SD) has base_heart heart02 x2 + heart06 x2 = 4.
 * 唐 可可   (PL!SP-sd1-002-SD) has base_heart heart02 x1 + heart06 x2 = 3.
 * (both verified against cards/cards.json read as UTF-8)
 * ========================================================================= */

static void test_zettai_lover_heart4_member_gets_score_one_heart3_does_not(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int lover = test_id_exact(&tg, LOVER);
    int kanon = test_id_exact(&tg, KANON);
    int keke  = test_id_exact(&tg, KEKE);
    int filler = mid(&tg, FILLER);
    ident(lover, LOVER, "zettai LOVER");
    ident(kanon, KANON, "zettai LOVER: 澁谷かのん (4 hearts)");
    ident(keke, KEKE, "zettai LOVER: 唐 可可 (3 hearts)");
    dbg_ability("LOVER", lover);

    /* The threshold discriminator. If the engine read the wrong heart total
     * these two would swap, so the totals are pinned, not assumed. The card text
     * says ハートを4つ以上持つ -- that is the BASE heart total, so only the
     * num_base prefix of the decoded heart table is summed. (Summing every entry
     * adds the blade hearts too: 澁谷かのん is 4 base + 1 b_heart06 = 5 entries,
     * which would make this a false red -- the same trap as counting a blade-
     * FIRST heart table.) */
    Card ck; Card cke;
    int kanon_hearts = 0, keke_hearts = 0;
    if (rb_decode_card_by_index((uint32_t)kanon, &ck))
        for (int h = 0; h < ck.num_base && h < ck.n_hearts; h++) kanon_hearts += ck.heart_count[h];
    if (rb_decode_card_by_index((uint32_t)keke, &cke))
        for (int h = 0; h < cke.num_base && h < cke.n_hearts; h++) keke_hearts += cke.heart_count[h];
    rb_free_card(&ck); rb_free_card(&cke);
    CHECK_EQ(kanon_hearts, 4, "澁谷かのん must carry 4 BASE hearts (qualifies)");
    CHECK_EQ(keke_hearts, 3, "唐 可可 must carry 3 BASE hearts (does not qualify)");
    assert_in_group(kanon, G_LIELLA, "澁谷かのん is a Liella! member");
    assert_in_group(keke, G_LIELLA, "唐 可可 is a Liella! member");

    int st[2]; st[0] = keke; st[1] = kanon;
    int got = live_start_score_mod(&tg, lover, filler, st, 2, NULL, 0);
    char m1[192], m2[192];
    snprintf(m1, sizeof m1,
             "Live card should have at least +1 score from 1 qualifying member (got %d)", got);
    CHECK(got >= 1, m1);
    /* Keke (3 hearts) must NOT qualify: the total is only +1, not +2. This is
     * the real discriminator -- it is the ONLY thing that tells "+1 per 4-heart
     * member" from "+1 per member". */
    snprintf(m2, sizeof m2,
             "Live card should have at most +1 score (only 1 member with 4+ hearts, got %d)", got);
    CHECK(got <= 1, m2);
}

static void test_zettai_lover_no_member_meets_heart_threshold(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int lover = test_id_exact(&tg, LOVER);
    int keke  = test_id_exact(&tg, KEKE);
    int filler = mid(&tg, FILLER);
    ident(lover, LOVER, "zettai LOVER: no member qualifies");
    ident(keke, KEKE, "zettai LOVER: 唐 可可");

    int st[1]; st[0] = keke;
    CHECK_EQ(live_start_score_mod(&tg, lover, filler, st, 1, NULL, 0), 0,
             "No score bonus when no member has 4+ hearts");
    /* PAIRED POSITIVE control: this is the SAME live and the SAME walk one case
     * above, with one extra qualifying member. */
    CHECK_EQ(test_zone_len(&tg, 0, "stage"), 1,
             "zettai LOVER negative control: exactly one member, and it is the "
             "3-heart one");
}

static void test_zettai_lover_non_liella_high_heart_not_counted(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int lover  = test_id_exact(&tg, LOVER);
    int kanon  = test_id_exact(&tg, KANON);
    int honoka = test_id_exact(&tg, FILLER);   /* mu's, Printemps */
    int filler = mid(&tg, FILLER2);
    ident(lover, LOVER, "zettai LOVER: non-Liella excluded");
    ident(kanon, KANON, "zettai LOVER: 澁谷かのん");
    ident(honoka, FILLER, "zettai LOVER: 高坂 穂乃果 (mu's, NOT Liella!)");
    assert_in_group(kanon, G_LIELLA, "澁谷かのん is a Liella! member");
    assert_in_group(honoka, G_MUS, "高坂 穂乃果 is a mu's member, not Liella!");
    CHECK(!rb_card_matches_group_str(honoka, G_LIELLA),
          "zettai LOVER control: the excluded member really is outside 『Liella!』");

    int st[2]; st[0] = kanon; st[1] = honoka;
    CHECK_EQ(live_start_score_mod(&tg, lover, filler, st, 2, NULL, 0), 1,
             "Only 1 Liella! member should count (non-Liella! excluded)");
}

/* =========================================================================
 * B4. live_start_lily_white_success_zone_count_pl_pb2_041_l_test.rs (6 tests)
 *
 * PL!-pb2-041-L 春情ロマンティック (ab#1), ライブ開始時: 自分の成功ライブカード
 * 置き場に『lily white』のカードが2枚以上ある場合、このカードのスコアを＋１する。
 * ========================================================================= */

static void test_two_lily_white_success_cards_score_plus_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int rom = test_id_exact(&tg, ROMANTIC);
    int cheap = test_id_exact(&tg, LW_CHEAP), midc = test_id_exact(&tg, LW_MID);
    ident(rom, ROMANTIC, "041 positive");
    ident(cheap, LW_CHEAP, "041 positive: the cheap lily white");
    ident(midc, LW_MID, "041 positive: the mid lily white");
    dbg_ability("ROMANTIC", rom);

    /* The Rust fixture bug, made reproducible. The file spells this print
     * "PL!-bp1-029-SRL"; there is no such card. */
    CHECK(test_id_exact(&tg, LW_CHEAP_RUST) < 0,
          "RUST FIXTURE BUG: PL!-bp1-029-SRL (as spelled in "
          "live_start_lily_white_success_zone_count_pl_pb2_041_l_test.rs:15) does not "
          "exist in cards.json; the real print is PL!-pb1-029-SRL, used below");

    /* The printed score the Rust asserts with assert_card_score(romantic, 7):
     * assert the ENGINE's decoded value too, so a printed/compiled mismatch
     * cannot make a score threshold unreachable silently. */
    Card c;
    int engine_score = -1;
    if (rb_decode_card_by_index((uint32_t)rom, &c)) { engine_score = c.score; rb_free_card(&c); }
    CHECK_EQ(engine_score, 7,
             "PL!-pb2-041-L: the ENGINE's decoded score is 7 (cards.json also says 7)");
    CHECK(cheap != midc, "041 positive control: two distinct cards");

    assert_in_group(cheap, G_LW, "041 positive: success-zone fixture");
    assert_in_group(midc, G_LW, "041 positive: success-zone fixture");
    int sz[2]; sz[0] = cheap; sz[1] = midc;
    int filler = mid(&tg, FILLER);
    CHECK_EQ(live_start_score_mod(&tg, rom, filler, NULL, 0, sz, 2), 1,
             "自分の成功ライブカード置き場に『lily white』のカードが2枚以上ある場合、"
             "このカードのスコアを+1する -- two qualifying cards must give exactly +1");
}

static void test_one_lily_white_success_card_scores_zero(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int rom = test_id_exact(&tg, ROMANTIC);
    int cheap = test_id_exact(&tg, LW_CHEAP);
    ident(rom, ROMANTIC, "041 one card");
    ident(cheap, LW_CHEAP, "041 one card");
    assert_in_group(cheap, G_LW, "041 one card: the fixture");
    int sz[1]; sz[0] = cheap;
    int filler = mid(&tg, FILLER);
    CHECK_EQ(live_start_score_mod(&tg, rom, filler, NULL, 0, sz, 1), 0,
             "the threshold is 2枚以上 -- a single qualifying card must not score");
}

static void test_three_lily_white_success_cards_still_score_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int rom = test_id_exact(&tg, ROMANTIC);
    int cheap = test_id_exact(&tg, LW_CHEAP), midc = test_id_exact(&tg, LW_MID),
        third = test_id_exact(&tg, LW_THIRD);
    ident(rom, ROMANTIC, "041 three cards");
    ident(cheap, LW_CHEAP, "041 three cards");
    ident(midc, LW_MID, "041 three cards");
    ident(third, LW_THIRD, "041 three cards");
    assert_in_group(third, G_LW, "041 three cards: the third fixture");
    int sz[3]; sz[0] = cheap; sz[1] = midc; sz[2] = third;
    int filler = mid(&tg, FILLER);
    CHECK_EQ(live_start_score_mod(&tg, rom, filler, NULL, 0, sz, 3), 1,
             "2枚以上 gates the effect on once -- three qualifying cards must still be "
             "+1, not +2");
}

static void test_lily_white_cards_in_hand_do_not_count(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int rom = test_id_exact(&tg, ROMANTIC);
    int lw1 = test_id_exact(&tg, LW_CHEAP), lw2 = test_id_exact(&tg, LW_MID);
    ident(rom, ROMANTIC, "041 hand case");
    ident(lw1, LW_CHEAP, "041 hand case");
    ident(lw2, LW_MID, "041 hand case");
    assert_in_group(lw1, G_LW, "041 hand case: hand fixture");
    assert_in_group(lw2, G_LW, "041 hand case: hand fixture");
    int filler = mid(&tg, FILLER);
    test_add_to_hand(&tg, lw1);
    test_add_to_hand(&tg, lw2);
    CHECK_EQ(live_start_score_mod(&tg, rom, filler, NULL, 0, NULL, 0), 0,
             "the condition reads 自分の成功ライブカード置き場 only -- two lily white "
             "cards in hand must not satisfy it");
}

static void test_non_lily_white_success_cards_do_not_count(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int rom = test_id_exact(&tg, ROMANTIC);
    int mus1 = test_id_exact(&tg, MUS_LIVE), mus2 = test_id_exact(&tg, MUS_LIVE);
    ident(rom, ROMANTIC, "041 mu's case");
    ident(mus1, MUS_LIVE, "041 mu's case");
    assert_in_group(mus1, G_MUS, "041 mu's case: the success-zone fixture");
    CHECK(!rb_card_matches_group_str(mus1, G_LW),
          "041 mu's case control: the fixture really is outside 『lily white』");
    CHECK(rb_card_is_live(mus1), "041 mu's case control: the fixture really is a live card");
    CHECK(mus1 == mus2,
          "041 mu's case: the Rust pushes the SAME id twice (game.id is not new_id); "
          "kept faithful -- the zone holds two entries either way");
    int sz[2]; sz[0] = mus1; sz[1] = mus2;
    int filler = mid(&tg, FILLER);
    CHECK_EQ(live_start_score_mod(&tg, rom, filler, NULL, 0, sz, 2), 0,
             "『lily white』のカードが2枚以上 -- two mu's lives do not satisfy the group filter");
}

static void test_empty_success_zone_scores_zero(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int rom = test_id_exact(&tg, ROMANTIC);
    ident(rom, ROMANTIC, "041 empty zone");
    int filler = mid(&tg, FILLER);
    CHECK_EQ(live_start_score_mod(&tg, rom, filler, NULL, 0, NULL, 0), 0,
             "an empty 成功ライブカード置き場 cannot satisfy 2枚以上");
}

/* =========================================================================
 * B5. pb2_live_success_plus_one_per_distinct_mus_member_pl_pb2_039_l_test.rs
 *     (4 tests)
 *
 * PL!-pb2-039-L 僕たちはひとつの光 (ab#1), ライブ成功時: 自分のステージのメンバーと
 * エールにより公開された自分のメンバーカードの中にある、名前の異なる『μ's』の
 * メンバー1人につき、このカードのスコアを＋１する。
 *
 * The Rust file RECORDS that every positive case yields +0 and calls it "a
 * general gap ... modify_score combined with per_unit appears 11 times in the
 * corpus and none of the group-filtered ones is covered". Only the cases that
 * hold today are asserted; the ones that do not are left as the finding.
 * ========================================================================= */

/* Rust score_with(stage, revealed): stage members, revealed cards, succeed the
 * live, read the score modifier. The Heart00 wildcard of the Rust is the C
 * engine's success window, forced open the same way (live_success[0] = 1). */
static int hikari_score_with(TestGame *tg, const int *stage, int n_stage,
                              const int *revealed, int n_revealed)
{
    int hikari = test_id_exact(tg, HIKARI);
    for (int i = 0; i < n_stage; i++) test_add_to_stage(tg, i, stage[i]);
    for (int i = 0; i < n_revealed; i++) test_add_to_revealed(tg, revealed[i]);
    test_add_to_live(tg, hikari);
    fill_decks(tg, mid(tg, FILLER), 12);

    /* CONTROLS at the assertion site. */
    CHECK_EQ(test_zone_len(tg, 0, "live"), 1,
             "039: the subject live is in the live card zone");
    CHECK_EQ(tg->state.n_revealed, n_revealed,
             "039: the revealed-card list holds exactly what was placed");

    int census = test_total_card_count(tg);
    /* rb_should_trigger_live_success needs BOTH live.n > 0 and live_success[pl];
     * fire_live_success sets the second, so it is set HERE, at the assertion
     * site, and the control below reads an open window rather than a closed
     * one. (An earlier revision asserted before the window was opened and the
     * control was a guaranteed-false red -- a test bug, not a finding.) */
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "039: the ライブ成功時 window is genuinely open (live zone non-empty AND "
          "live_success[0] set) -- a zero below cannot be a closed window");
    fire_live_success(tg, 0, hikari);
    drain_choices(tg);
    CHECK_EQ(test_total_card_count(tg), census,
             "039: CENSUS -- the live-success trigger destroyed or duplicated no card");
    return score_of(tg, hikari);
}

static void test_fixtures_span_distinct_mus_names_across_units(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int eri = test_id_exact(&tg, ERI), hana = test_id_exact(&tg, HANAYOME),
        nico = test_id_exact(&tg, NICO);
    ident(eri, ERI, "039 fixture");
    ident(hana, HANAYOME, "039 fixture");
    ident(nico, NICO, "039 fixture");
    assert_in_group(eri, G_MUS, "039 fixture: must be a mu's member");
    assert_in_group(hana, G_MUS, "039 fixture: must be a mu's member");
    assert_in_group(nico, G_MUS, "039 fixture: must be a mu's member");
    assert_distinct_names(eri, hana, "039 fixture");
    assert_distinct_names(eri, nico, "039 fixture");
    assert_distinct_names(hana, nico, "039 fixture");
    /* The two fixtures sit in different units (BiBi / Printemps) on purpose --
     * mu's spans both. */
    const char *ue = rb_card_unit_name(eri), *uh = rb_card_unit_name(hana);
    CHECK(ue && uh && strcmp(ue, uh) != 0,
          "039 fixture: the two fixtures sit in DIFFERENT units on purpose, and mu's spans both");    printf("     039 units: %s = [%s], %s = [%s]\n", ERI, ue ? ue : "-", HANAYOME, uh ? uh : "-");
}

static void test_an_empty_pool_scores_nothing(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    CHECK_EQ(hikari_score_with(&tg, NULL, 0, NULL, 0), 0, "no members at all -- no bonus");
}

static void test_a_non_mus_member_produces_no_bonus(void)
{
    {
        static TestGame tg;
        test_card_copy_pool_reset();
        test_game_new(&tg);
        int az = test_id_exact(&tg, AZALEA);
        ident(az, AZALEA, "039 negative: staged");
        CHECK(!rb_card_matches_group_str(az, G_MUS),
              "039 negative control: a staged AZALEA member is NOT mu's (Aqours series)");
        int st[1]; st[0] = az;
        CHECK_EQ(hikari_score_with(&tg, st, 1, NULL, 0), 0,
                 "『mu's』のメンバー -- a staged AZALEA member is not mu's");
    }
    {
        static TestGame tg;
        test_card_copy_pool_reset();
        test_game_new(&tg);
        int az = test_id_exact(&tg, AZALEA);
        ident(az, AZALEA, "039 negative: revealed");
        CHECK(!rb_card_matches_group_str(az, G_MUS),
              "039 negative control: a revealed AZALEA member is NOT mu's either");
        int rv[1]; rv[0] = az;
        CHECK_EQ(hikari_score_with(&tg, NULL, 0, rv, 1), 0,
                 "『mu's』のメンバー -- a revealed AZALEA member is not mu's either");
    }
}

static void test_the_live_success_harness_fires_the_ability(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int hikari = test_id_exact(&tg, HIKARI);
    ident(hikari, HIKARI, "039: the subject live");
    dbg_ability("HIKARI", hikari);

    int found = 0, action_ok = 0, per_unit_ok = 0, distinct_ok = 0;
    int n = rb_card_num_abilities((uint32_t)hikari);
    for (int k = 0; k < n && !found; k++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)hikari, k, &ab)) continue;
        if (ab.triggers && !strcmp(ab.triggers, TRIGGER_LIVE_SUCCESS) && ab.effect) {
            found = 1;
            action_ok    = ab.effect->action && !strcmp(ab.effect->action, "modify_score");
            for (int i = 0; i < ab.effect->n_extra; i++) {
                if (!strcmp(ab.effect->extra_k[i], "per_unit") &&
                    !strcmp(ab.effect->extra_v[i], "true")) per_unit_ok = 1;
                if (!strcmp(ab.effect->extra_k[i], "distinct") &&
                    !strcmp(ab.effect->extra_v[i], "card_name")) distinct_ok = 1;
            }
        }
        rb_free_ability(&ab);
    }
    CHECK(found,
          "PL!-pb2-039-L must have a ライブ成功時 ability for this file to be about "
          "anything");
    CHECK(action_ok, "このカードのスコアを+1する (action=modify_score)");
    CHECK(per_unit_ok, "1人につき -- per_unit");
    CHECK(distinct_ok, "名前の異なる『mu's』のメンバー -- distinct by name");

    /* The positive control the Rust does NOT have: fire it on a board with one
     * qualifying mu's member, so a future reader can tell "the effect parses"
     * from "the effect applies". The Rust records +0 here; that is the gap. */
    int eri = test_id_exact(&tg, ERI);
    int nico = test_id_exact(&tg, NICO);
    int st[2]; st[0] = eri; st[1] = nico;
    int got = hikari_score_with(&tg, st, 2, NULL, 0);
    printf("     039 POSITIVE CONTROL: 2 distinct mu's members on stage -> score mod %d\n", got);
    CHECK_EQ(got, 2,
             "POSITIVE CONTROL: 2 differently-named mu's members must give +2 score "
             "(Rust records +0 for every case in this file -- see the header; if this "
             "line is red, the per-unit + group-filtered modify_score path is still the "
             "unfixed general gap the Rust documents)");
}

/* =========================================================================
 * B6. pb2_two_mus_members_live_or_success_zone_score_pl_pb2_038_l_test.rs (8)
 *
 * PL!-pb2-038-L 嵐のなかの恋だから, 常時: このカードが自分のライブカード置き場か
 * 成功ライブカード置き場にあり、かつ自分のステージに2人の『mu's』のメンバーのみが
 * いる場合、ライブの合計スコアを+1する。この効果は重複しない。
 * This targets ライブの合計スコア (target=live_total), so the bonus lands in the
 * PLAYER's constant, not in the card's own score modifier.
 * ========================================================================= */

/* Rust score_mod_in(zone, members). */
static int score_mod_in(TestGame *tg, const char *zone, const int *members, int n_members)
{
    int love = test_id_exact(tg, LOVE_038);
    ident(love, LOVE_038, "038: 嵐のなかの恋だから");
    Card c;
    int engine_score = -1;
    if (rb_decode_card_by_index((uint32_t)love, &c)) { engine_score = c.score; rb_free_card(&c); }
    CHECK_EQ(engine_score, 5,
             "PL!-pb2-038-L: the ENGINE's decoded score is 5 (cards.json also says 5)");
    /* Dumped once per case so the reported finding carries the COMPILED shape:
     * whether the 常時 is present at all, and whether it still carries the
     * zone disjunction and the live_total target the Rust case is about. */
    static int dumped = 0;
    if (!dumped) { dumped = 1; dbg_card("LOVE-038", love); dbg_ability("LOVE-038", love); }
    for (int i = 0; i < n_members; i++) test_add_to_stage(tg, i, members[i]);
    if (!strcmp(zone, "live")) test_add_to_live(tg, love);
    else if (!strcmp(zone, "success")) test_add_to_success(tg, love);
    else { fprintf(stderr, "FAIL: 038: unknown zone %s\n", zone); failures++; publish_counts(); }
    int census = test_total_card_count(tg);
    test_recalc(tg);
    CHECK_EQ(test_total_card_count(tg), census,
             "038: CENSUS -- recalculate_constants destroyed or duplicated no card");
    return total_bonus_p1(tg);
}

static void test_in_success_live_zone_with_two_mus_members_scores_plus_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int a = test_id_exact(&tg, MUS_A), b = test_id_exact(&tg, MUS_B);
    ident(a, MUS_A, "038 success-zone positive: slot 0");
    ident(b, MUS_B, "038 success-zone positive: slot 1");
    assert_in_group(a, G_MUS, "038 stage fixture");
    assert_in_group(b, G_MUS, "038 stage fixture");
    int st[2]; st[0] = a; st[1] = b;
    CHECK_EQ(score_mod_in(&tg, "success", st, 2), 1,
             "成功ライブカード置き場にあり and 2人の『mu's』のメンバーのみ -- both "
             "clauses hold");
}

static void test_in_live_card_zone_with_two_mus_members_also_scores_plus_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int a = test_id_exact(&tg, MUS_A), b = test_id_exact(&tg, MUS_B);
    ident(a, MUS_A, "038 live-zone positive: slot 0");
    ident(b, MUS_B, "038 live-zone positive: slot 1");
    int st[2]; st[0] = a; st[1] = b;
    CHECK_EQ(score_mod_in(&tg, "live", st, 2), 1,
             "ライブカード置き場**か**成功ライブカード置き場 -- the live card zone is "
             "the other half of the same disjunction, so it scores too");
}

static void test_one_member_is_not_enough(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int a = test_id_exact(&tg, MUS_A);
    ident(a, MUS_A, "038 one member");
    int st[1]; st[0] = a;
    CHECK_EQ(score_mod_in(&tg, "success", st, 1), 0,
             "2人の『mu's』のメンバーのみ -- a single member does not satisfy 2人");
}

static void test_three_members_do_not_satisfy_exactly_two(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int a = test_id_exact(&tg, MUS_A), b = test_id_exact(&tg, MUS_B), c = test_id_exact(&tg, MUS_C);
    ident(a, MUS_A, "038 three members: slot 0");
    ident(b, MUS_B, "038 three members: slot 1");
    ident(c, MUS_C, "038 three members: slot 2");
    assert_in_group(c, G_MUS, "038 stage fixture: the third mu's member");
    int st[3]; st[0] = a; st[1] = b; st[2] = c;
    CHECK_EQ(score_mod_in(&tg, "success", st, 3), 0,
             "2人のみ is a count of exactly two, so three mu's members must NOT qualify");
}

static void test_a_non_mus_member_breaks_exclusivity(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int a = test_id_exact(&tg, MUS_A), az = test_id_exact(&tg, AZALEA);
    ident(a, MUS_A, "038 AZALEA breaks: slot 0");
    ident(az, AZALEA, "038 AZALEA breaks: slot 1");
    assert_in_group(az, G_AZALEA, "038 AZALEA breaks: the intruder is an AZALEA member");
    int st[2]; st[0] = a; st[1] = az;
    CHECK_EQ(score_mod_in(&tg, "success", st, 2), 0,
             "『mu's』のメンバーのみ -- an AZALEA member alongside breaks exclusivity");
}

static void test_out_of_both_live_zones_grants_nothing(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int love = test_id_exact(&tg, LOVE_038);
    int a = test_id_exact(&tg, MUS_A), b = test_id_exact(&tg, MUS_B);
    ident(love, LOVE_038, "038: out of both zones");
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    test_add_to_hand(&tg, love);      /* deliberately in NEITHER zone */
    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(test_total_card_count(&tg), census, "038: CENSUS");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 0,
             "038 out-of-zones control: the live card zone really is empty");
    CHECK_EQ(test_zone_len(&tg, 0, "success"), 0,
             "038 out-of-zones control: the success live card zone really is empty");
    CHECK_EQ(total_bonus_p1(&tg), 0,
             "このカードが自分のライブカード置き場か成功ライブカード置き場にあり -- a "
             "card in neither zone is not the subject, so the constant does not apply");
}

static void test_two_copies_do_not_stack(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int love = test_id_exact(&tg, LOVE_038);
    int a = test_id_exact(&tg, MUS_A), b = test_id_exact(&tg, MUS_B);
    ident(love, LOVE_038, "038 non-stackable");
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);

    /* mid(), not test_id(): two copies of ONE print must be two card ids, or
     * the "two copies" board is really one card listed twice. */
    int first  = mid(&tg, LOVE_038);
    int second = mid(&tg, LOVE_038);
    CHECK(first > 0 && second > 0, "038 non-stackable: both copies were minted");
    CHECK(first != second,
          "038 non-stackable control: the two copies are DISTINCT card ids (mid(), "
          "not test_id())");

    test_add_to_success(&tg, first);
    /* CENSUS after EVERY fixture card is placed -- the two copies are placed
     * one at a time because the precondition is read in between, so the
     * baseline moves with them. */
    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(total_bonus_p1(&tg), 1,
             "precondition: a single copy contributes +1 to ライブの合計スコア");

    test_add_to_success(&tg, second);
    int census2 = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(total_bonus_p1(&tg), 1,
             "この効果は重複しない -- a second copy must not raise the total to 2");
    CHECK_EQ(test_total_card_count(&tg), census2,
             "038 non-stackable: CENSUS -- adding the second copy destroyed or "
             "duplicated no card");
    CHECK_EQ(census + 1, census2,
             "038 non-stackable: CENSUS -- exactly one card was added between the two "
             "baselines");
}

/* The configuration two_copies_do_not_stack does not build: one copy in EACH
 * zone. 「この効果は重複しない」 is about the ability, not the zone, and the card's
 * zone clause is a disjunction -- so a de-dup keyed per-zone instead of
 * per-ability would quietly pay twice here. */
static void test_one_copy_in_each_live_zone_still_scores_only_one(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int performing = test_id_exact(&tg, LOVE_038);
    int succeeded  = mid(&tg, LOVE_038);
    int a = test_id_exact(&tg, MUS_A), b = test_id_exact(&tg, MUS_B);
    ident(performing, LOVE_038, "038 one-per-zone: the performing copy");
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    CHECK(performing != succeeded,
          "038 one-per-zone control: the two copies are DISTINCT card ids (mid(), "
          "not test_id())");
    test_add_to_live(&tg, performing);
    test_add_to_success(&tg, succeeded);

    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(test_total_card_count(&tg), census, "038 one-per-zone: CENSUS");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1,
             "038 one-per-zone control: one copy really is in the live card zone");
    CHECK_EQ(test_zone_len(&tg, 0, "success"), 1,
             "038 one-per-zone control: one copy really is in the success live card zone");

    CHECK_EQ(total_bonus_p1(&tg), 1,
             "one copy in ライブカード置き場 and one in 成功ライブカード置き場 -- both "
             "satisfy the zone disjunction, but この効果は重複しない, so the total must "
             "stay at +1. A value of 2 means non_stackable is de-duplicating per zone "
             "rather than per ability");
}

/* -- fork-per-case runner ---------------------------------------------- */
typedef void (*CaseFn)(void);
typedef struct { const char *name; CaseFn fn; } Case;

static void run_case(const Case *c)
{
    g_shared->assertions = 0;
    g_shared->failures   = 0;
    g_shared->signal     = 0;
    g_shared->exited_ok  = 0;
    g_shared->name[0]    = 0;
    snprintf(g_case_name, sizeof g_case_name, "%s", c->name);
    memcpy(g_shared->name, g_case_name, sizeof g_case_name);

    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "FAIL: fork() failed for %s\n", c->name);
        assertions++; failures++;
        return;
    }
    if (pid == 0) {
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_handler = fatal_handler;
        sigemptyset(&sa.sa_mask);
        sa.sa_flags = SA_NODEFER | SA_RESETHAND;
        sigaction(SIGSEGV, &sa, NULL);
        sigaction(SIGBUS,  &sa, NULL);
        sigaction(SIGABRT, &sa, NULL);
        sigaction(SIGFPE,  &sa, NULL);
        assertions = 0;
        failures   = 0;
        rb_card_copy_pool_reset();
        c->fn();
        g_shared->assertions = assertions;
        g_shared->failures   = failures;
        g_shared->exited_ok  = 1;
        fflush(stdout); fflush(stderr);
        _exit(0);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) status = -1;

    assertions += g_shared->assertions;
    failures   += g_shared->failures;

    if (g_shared->signal) {
        const char *nm = "signal";
        switch (g_shared->signal) {
            case SIGSEGV: nm = "SIGSEGV"; break;
            case SIGBUS:  nm = "SIGBUS";  break;
            case SIGABRT: nm = "SIGABRT"; break;
            case SIGFPE:  nm = "SIGFPE";  break;
            default: break;
        }
        fprintf(stderr, "FAIL: %s: fatal %s (%d) in child; %d assertion(s) reached, %d failing\n",
                g_shared->name[0] ? g_shared->name : c->name, nm, g_shared->signal,
                g_shared->assertions, g_shared->failures);
        failures++; assertions++;
    } else if (!g_shared->exited_ok) {
        fprintf(stderr, "FAIL: %s: child terminated abnormally (wait status %d) after %d assertion(s)\n",
                c->name, status, g_shared->assertions);
        failures++; assertions++;
    }
    printf("== case %-58s assertions=%d failures=%d\n",
           c->name, g_shared->assertions, g_shared->failures);
    if (g_shared->failures > g_shared->assertions) {
        fprintf(stderr, "FAIL: %s: shared counters inconsistent (%d failures > %d "
                        "assertions) -- the shared mapping is not coherent on this "
                        "toolchain and the per-case totals cannot be trusted\n",
                c->name, g_shared->failures, g_shared->assertions);
        failures++;
    }
    fflush(stdout);
}

int main(void)
{
    if (load_card_db() != 0) { fprintf(stderr, "FAIL: database load\n"); return 1; }
    setvbuf(stdout, NULL, _IONBF, 0);

    /* The engine's colour indices are 0=heart00 .. 6=heart06, which is NOT the
     * RB_HEART_* enum order. Derived, never assumed. */
    printf("== heart colour mapping (rb_parse_heart_color)\n");
    {
        static const char *const NAMES[7] = {
            "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"
        };
        for (int i = 0; i < 7; i++)
            printf("   %s -> %d\n", NAMES[i], (int)rb_parse_heart_color(NAMES[i]));
        printf("   RB_HEART_PINK=%d RB_HEART_RED=%d RB_HEART_YELLOW=%d RB_HEART_GREEN=%d "
               "RB_HEART_BLUE=%d RB_HEART_PURPLE=%d RB_HEART_ORANGE=%d RB_HEART_ALL=%d "
               "RB_HEART_DRAW=%d RB_HEART_SCORE=%d\n",
               (int)RB_HEART_PINK, (int)RB_HEART_RED, (int)RB_HEART_YELLOW,
               (int)RB_HEART_GREEN, (int)RB_HEART_BLUE, (int)RB_HEART_PURPLE,
               (int)RB_HEART_ORANGE, (int)RB_HEART_ALL, (int)RB_HEART_DRAW,
               (int)RB_HEART_SCORE);
    }

    /* rb_card_group_name is SERIES-level (every ラブライブ！ card reports mu's);
     * rb_card_unit_name is the unit-level check, and 『lily white』/『AZALEA』 are
     * UNIT names, so the two accessors are printed for every fixture. */
    printf("== fixture group / unit (group is SERIES-level by design)\n");
    {
        static const char *const FIX[] = {
            HONOKA, SCORE_ICON, BALL_ICON, DRAW_ICON, NO_ICON, RECOVERABLE,
            LINK, HASU_A, HASU_B, HASU_C, HASU_ONE,
            LOVE_029, LW_UMI, LW_NOZOMI, PRINTEMPS, AZALEA,
            LOVER, KANON, KEKE,
            ROMANTIC, LW_CHEAP, LW_MID, LW_THIRD, MUS_LIVE,
            HIKARI, ERI, HANAYOME, NICO,
            LOVE_038, MUS_A, MUS_B, MUS_C,
            FILLER, FILLER2, SRL_FILLER, NULL
        };
        for (int i = 0; FIX[i]; i++) {
            int cid = rb_find_card_by_no(FIX[i]);
            printf("   %-18s id=%-6d unit=[%s] group=[%s]\n", FIX[i], cid,
                   cid >= 0 && rb_card_unit_name(cid) ? rb_card_unit_name(cid) : "-",
                   cid >= 0 && rb_card_group_name(cid) ? rb_card_group_name(cid) : "-");
        }
        int missing = rb_find_card_by_no(LW_CHEAP_RUST);
        printf("   RUST FIXTURE BUG: rb_find_card_by_no(\"%s\") = %d (the real print is %s)\n",
               LW_CHEAP_RUST, missing, LW_CHEAP);
    }

    g_shared = mmap(NULL, sizeof *g_shared, PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (g_shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: mmap(MAP_SHARED|MAP_ANONYMOUS) for the shared counters "
                        "failed; running without crash isolation\n");
        g_shared = NULL;
    }

    static const Case CASES[] = {
        /* A -- the one live_total module test_parity_live_total.c never took */
        { "001_draw_icon_recovers_one_mus_card",              test_a_draw_card_in_the_zone_recovers_one_mus_card_to_hand },
        { "001_score_and_all_blade_clauses_produce_nothing",  test_the_score_and_all_blade_clauses_produce_nothing_measurable_yet },
        { "001_card_with_none_of_the_icons_fires_nothing",    test_a_card_with_none_of_the_icons_fires_nothing },
        { "001_empty_success_zone_fires_nothing",             test_an_empty_success_zone_fires_nothing },
        { "001_fixtures_carry_exactly_the_icon_under_test",   test_the_fixtures_carry_exactly_the_icon_under_test },
        /* B1 -- PL!HS-bp2-020-L Link to the FUTURE */
        { "020_three_distinct_group_members_add_six",         test_live_start_three_distinct_group_members_add_six_score },
        { "020_one_group_member_adds_two",                    test_live_start_one_group_member_adds_two_score },
        { "020_no_group_members_adds_no_score",               test_live_start_no_group_members_adds_no_score },
        /* B2 -- PL!-pb1-029-SRL */
        { "029_empty_zone_and_all_lily_white_scores_plus_one", test_empty_success_zone_and_all_lily_white_stage_scores_plus_one },
        { "029_two_lily_white_members_still_score_only_one",  test_two_lily_white_members_still_score_only_one },
        { "029_a_card_in_the_success_zone_blocks_the_bonus",  test_a_card_in_the_success_zone_blocks_the_bonus },
        { "029_a_non_lily_white_member_blocks_the_bonus",     test_a_non_lily_white_member_blocks_the_bonus },
        { "029_a_non_mus_unit_also_breaks_exclusivity",       test_a_non_mus_unit_also_breaks_exclusivity },
        { "029_both_clauses_broken_scores_nothing",           test_both_clauses_broken_scores_nothing },
        { "029_single_lily_white_alone_qualifies",            test_single_lily_white_alone_qualifies },
        { "029_empty_stage_is_recorded_as_vacuous",           test_empty_stage_is_recorded_as_vacuous },
        /* B3 -- PL!SP-pb2-045-L */
        { "045_heart4_member_gets_score_one_heart3_not",      test_zettai_lover_heart4_member_gets_score_one_heart3_does_not },
        { "045_no_member_meets_heart_threshold",              test_zettai_lover_no_member_meets_heart_threshold },
        { "045_non_liella_high_heart_not_counted",           test_zettai_lover_non_liella_high_heart_not_counted },
        /* B4 -- PL!-pb2-041-L */
        { "041_two_lily_white_success_cards_score_plus_one",  test_two_lily_white_success_cards_score_plus_one },
        { "041_one_lily_white_success_card_scores_zero",      test_one_lily_white_success_card_scores_zero },
        { "041_three_lily_white_success_cards_still_one",     test_three_lily_white_success_cards_still_score_one },
        { "041_lily_white_cards_in_hand_do_not_count",        test_lily_white_cards_in_hand_do_not_count },
        { "041_non_lily_white_success_cards_do_not_count",    test_non_lily_white_success_cards_do_not_count },
        { "041_empty_success_zone_scores_zero",               test_empty_success_zone_scores_zero },
        /* B5 -- PL!-pb2-039-L */
        { "039_fixtures_span_distinct_mus_names",             test_fixtures_span_distinct_mus_names_across_units },
        { "039_an_empty_pool_scores_nothing",                 test_an_empty_pool_scores_nothing },
        { "039_a_non_mus_member_produces_no_bonus",           test_a_non_mus_member_produces_no_bonus },
        { "039_the_live_success_harness_fires_the_ability",   test_the_live_success_harness_fires_the_ability },
        /* B6 -- PL!-pb2-038-L */
        { "038_in_success_zone_with_two_mus_scores_plus_one", test_in_success_live_zone_with_two_mus_members_scores_plus_one },
        { "038_in_live_zone_with_two_mus_also_plus_one",      test_in_live_card_zone_with_two_mus_members_also_scores_plus_one },
        { "038_one_member_is_not_enough",                     test_one_member_is_not_enough },
        { "038_three_members_do_not_satisfy_exactly_two",     test_three_members_do_not_satisfy_exactly_two },
        { "038_a_non_mus_member_breaks_exclusivity",          test_a_non_mus_member_breaks_exclusivity },
        { "038_out_of_both_live_zones_grants_nothing",        test_out_of_both_live_zones_grants_nothing },
        { "038_two_copies_do_not_stack",                      test_two_copies_do_not_stack },
        { "038_one_copy_in_each_live_zone_still_scores_one",  test_one_copy_in_each_live_zone_still_scores_only_one },
    };

    for (unsigned i = 0; i < sizeof CASES / sizeof CASES[0]; i++)
        run_case(&CASES[i]);

    printf("\n%d cases, %d assertions, %d failures\n",
           (int)(sizeof CASES / sizeof CASES[0]), assertions, failures);
    if (failures) { printf("LIVE TOTAL-2 PARITY: RED\n"); return 1; }
    printf("ALL LIVE TOTAL-2 PARITY CHECKS PASSED\n");
    return 0;
}
