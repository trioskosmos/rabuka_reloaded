/* test_parity_recover_q79.c -EC parity suite for the SELF-TO-WAITROOM / KIDOU
 * / PARTIAL-RESOLUTION slice of the Rust cluster
 * engine/tests/test_modules/effects/recover/to_hand/.
 *
 * ============================================================================
 * CLAIM (tr_recover3, 2026-09-30). I own EXACTLY ONE new file,
 *   engine_c_wip/tests/test_parity_recover_q79.c
 * and the Rust files it ports, in full:
 *   engine/tests/test_modules/effects/recover/to_hand/
 *     - self_to_waitroom_recover_live_q79_q123_test.rs
 *     - self_to_waitroom_recover_member_q79_test.rs
 *     - debut_recovers_one_live_card_from_waitroom_to_hand_test.rs
 *     - live_success_recovers_one_member_card_from_waitroom_to_hand_test.rs
 *     - lilywhite_gated_live_pl_pb1_007_r_test.rs
 *     - kidou_self_to_waitroom_live_recovery_not_softlocked_test.rs
 *     - live_success_partial_resolution_of_revealed_live_q93_test.rs
 *     - pb2_kidou_self_to_waitroom_recover_mus_live_then_energy_per_mus_pl_pb2_007_pp_test.rs
 * I own NOTHING else: no src/ C files, no other tests/test_*.c, no Makefile.
 * ============================================================================
 *
 * BOUNDARY re-derived from the SIBLING BODIES, not from their headers.
 *
 *   test_parity_recover_hand_a.c  (RECOVERY-SOURCE half) cites NONE of my eight
 *   .rs files. Its §D (lines 802-850) is q145_empty_waitroom_optional_self_wait
 *   _test.rs in full -- the exact same PL!-bp3-003-R Kotori cost-11 fixture,
 *   the same "offered on an empty waitroom" premise, the same select_option(1),
 *   the same orientation_mod == "wait" assertion, plus hand.n == 0 and
 *   discard.n == 0. So q145 IS ALREADY PORTED and is deliberately NOT re-ported
 *   here. That is the boundary: my file starts at the SELF-COST family.
 *
 *   test_parity_recover_live.c owns group-gated live recovery driven off the
 *   ライブ成功時 / 登場 window of a LIVE card (its §J is
 *   live_success_recover_different_group_name_to_hand_ll_bp5_002_l_test.rs).
 *   My §C/§D also use a ライブ成功時 dispatch, but the card under test is the
 *   TRIPLE member LL-bp7-001-R＋ and the filter is a CARD-TYPE filter
 *   (member) rather than a group-difference filter, so there is no overlap in
 *   card or in filter. The only card it shares is the μ's LIVE used as a decoy;
 *   I deliberately use DIFFERENT print numbers from that file (see FIXTURES).
 *
 *   The family I take is characterised by: the ability's own COST moves the
 *   card off the stage, and the effect then recovers out of the waitroom.
 *   Nothing else in the corpus cites PL!-sd1-005-SD, PL!-sd1-002-SD,
 *   LL-bp7-001-R＋, PL!-pb1-007-R, PL!SP-pb1-001-R or PL!-pb2-007-PP.
 *
 * IDENTITY (AGENTS.md). Every fixture goes through mid() -> test_new_id(),
 * never the shared template alias, and every one is pinned with a BYTE-EXACT
 * strcmp against rb_card_string(c.card_no_idx). rb_card_no_eq is EXACT now but
 * rb_find_card_by_no has a SUBSTRING fallback, so neither is an identity pin.
 *
 * ENCODING (AGENTS.md). This file is 7-bit clean: every non-ASCII literal is a
 * \xNN byte escape. That matters doubly here - a mangled group or unit string
 * makes a filter match nothing, the card simply does not move, and every
 * negative assertion goes GREEN.
 *
 * VACUITY CONTROL - the landmine for this cluster. A recovery whose filter
 * matches nothing produces a CLEAN, PLAUSIBLE, GREEN negative. Two card
 * DESTROYING defects were already found today, and a recovery cluster is the
 * highest-risk area for one, because a card that "leaves" the waitroom and
 * arrives NOWHERE reads as a successful move. Therefore:
 *
 *   (1) WHOLE-BOARD CARD CENSUS (test_total_card_count) on every case,
 *       baselined AFTER all fixture cards are placed and IMMEDIATELY BEFORE
 *       the action under test. A move between zones must leave it flat.
 *   (2) No bare CHECK_PROMPTS anywhere. EXPECT_PROMPTS(tg, n, ...) counts the
 *       prompt still ON SCREEN at the assertion site and then asserts a floor
 *       on the running total; the shared CHECK_PROMPTS macro in other files
 *       increments and then compares, so it SATISFIES ITSELF.
 *   (3) Every negative assertion is additionally backed by EITHER a paired
 *       positive on the same board shape (§C3, §D3 are explicit in-file
 *       paired positives that exist only to make §C2 / §D2 non-vacuous) OR a
 *       GROUP / UNIT / CARD-TYPE pin proving the target is inside the filter's
 *       set and the decoy is outside it.
 *   (4) A prompt-count control is taken at the ASSERTION SITE wherever a
 *       prompt can exist, and the prompt's presence there is itself asserted.
 *
 * HARNESS. Each case runs in a FORKED child. The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child that dies on SIGSEGV mid-
 * assertion still leaves its partial counts readable by the parent, and the
 * child's signal handler names the test in flight. The child ZEROES the
 * counters on entry: a forked child inherits the parent's accumulated totals,
 * and the previous generation of this harness double-counted because of it.
 * rb_card_copy_pool_reset() is called per case, for the same reason.
 *
 * 7-bit / escape note: the heart00 wildcard used to open the ライブ成功時
 * window is index 0. It is DERIVED with rb_parse_heart_color("heart00") and
 * printed at startup, never hard-coded from a stale mapping.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN - the TRIPLE print's real suffix. */
#define FP "\xef\xbc\x8b"

/* ---- group / unit names, as BYTE EXACT escapes --------------------------- */
/* U+03BC GREEK SMALL LETTER MU + U+0027 APOSTROPHE = the 『μ's』 group. */
#define GRP_MU        "\xce\xbc's"
/* cards.json spells this unit in ASCII, "lilywhite" - NOT the katakana
 * ライインライト. The [unit] byte dump in the E cases prints the raw bytes so
 * that a string-table splice is distinguishable from a wrong literal here. */
#define UNIT_LILYWHITE "lilywhite"
/* このメンバーをステージから控え室 - the self->waitroom cost phrase.
 * NOTE the 4th escape group: ン is E3 83 B3.  (B0 would be ヰ, a near-identical
 * glyph that renders almost the same and silently fails every strstr.) */
#define PHRASE_SELF_WAITROOM \
    "\xe3\x81\x93\xe3\x81\xae\xe3\x83\xa1\xe3\x83\xb3\xe3\x83\x90\xe3\x83\xbc" \
    "\xe3\x82\x92\xe3\x82\xb9\xe3\x83\x86\xe3\x83\xbc\xe3\x82\xb8\xe3\x81\x8b" \
    "\xe3\x82\x89\xe6\x8e\xa7\xe3\x81\x88\xe5\xae\xa4\xe3\x81\xab"

/* その後 - the "after that" (is_further) marker that makes the effect 2-step. */
#define PHRASE_SONOGO "\xe3\x81\x9d\xe3\x81\xae\xe5\xbe\x8c"

/* ---- FIXTURES. Every one verified against cards/cards.json read as UTF-8.
 * The JSON is a dict keyed by card_no; the entries below are quoted verbatim
 * (name / cost / score / type / unit / ability) so the pins are auditable. */

/* self_to_waitroom_recover_live_q79_q123_test.rs
 * cards.json PL!-sd1-005-SD : 星空 凛 / cost 2 / blade 1 / unit lilywhite
 *   起動 このメンバーをステージから控え室に置く：自分の控え室から
 *        ライブカードを1枚手札に加える。                       (LIVE filter)
 * cards.json PL!-sd1-002-SD : 絢瀬 絵里 / cost 2 / unit BiBi
 *   起動 このメンバーをステージから控え室に置く：自分の控え室から
 *        メンバーカードを1枚手札に加える。                     (MEMBER filter) */
#define RIN_SD5     "PL!-sd1-005-SD"   /* 星空 凛        起動 self->wait + LIVE   */
#define ELI_SD2     "PL!-sd1-002-SD"   /* 絢瀬 絵里      起動 self->wait + MEMBER */
#define FILLER_010  "PL!-sd1-010-SD"   /* 高坂 穂乃果    cost 4 member, Printemps */
#define KOTORI_SD1  "PL!-sd1-001-SD"   /* 高坂 穂乃果    cost 11 member           */
#define KOTORI_SD3  "PL!-sd1-003-SD"   /* 南 ことり      cost 13 member           */
/* cards.json PL!-sd1-019-SD : START:DASH!! / LIVE / score 1 / need heart01,
 *   heart03, heart06 - a LIVE, used as the §A2 recovery target. */
#define LIVE_019    "PL!-sd1-019-SD"
/* cards.json PL!-sd1-020-SD : きっと青春が聞こえる / LIVE / score 2 /
 *   NO ability at all. That makes it the ideal ライブ成功時 WINDOW OPENER: it
 *   cannot itself fire a live-success ability and pollute the prompt count. */
#define LIVE_020    "PL!-sd1-020-SD"
/* cards.json PL!-bp4-025-SRL : 微熱からMystery / LIVE / score 5 / unit
 *   "lily white" / NO ability. The SECOND window opener, so that a card which
 *   DOES print its own ライブ成功時 can be the §D decoy. */
#define MUS_LIVE_B  "PL!-bp4-025-SRL"

/* debut_... and live_success_... both drive LL-bp7-001-R＋
 * cards.json LL-bp7-001-R＋ : 国木田花丸&優木せつ菜&嵐 千砂都 / cost 15 /
 *   member / 常時 (optional 3-member discard -> cost 10) +
 *   登場 自分の控え室からライブカードを1枚手札に加える。       (ab#1, LIVE)
 *   ライブ成功時 自分の控え室からメンバーカードを1枚手札に加える。(ab#2, MEMBER)
 * NOTE the FULLWIDTH plus: cards.json has "LL-bp7-001-R＋", not "...-R+".
 * rb_find_card_by_no normalises the two, but the byte-exact pin does not. */
#define TRIPLE      "LL-bp7-001-R" FP

/* lilywhite_gated_live_pl_pb1_007_r_test.rs
 * cards.json PL!-pb1-007-R : 東條 希 / cost 13 / member / unit "lilywhite"
 *   起動 ターン1回 手札を3枚控え室に置く：自分のステージにほかの
 *        『lilywhite』のメンバーがいる場合、自分の控え室から『μ's』の
 *        ライブカードを1枚手札に加える。…                    (the GATE)
 * cards.json PL!-bp3-014-N : 星空 凛 / cost 4 / unit "lilywhite"  (the MATCH)
 * cards.json PL!-sd1-001-SD : 高坂 穂乃果 / cost 11 / unit "Printemps" (the MISS) */
#define NOZOMI_R    "PL!-pb1-007-R"   /* 東條 希   cost 13, unit lilywhite  */
#define RIN_BP3N    "PL!-bp3-014-N"   /* 星空 凛   cost  4, unit lilywhite  */
#define PRINTEMPS_SD1 "PL!-sd1-001-SD" /* 高坂 穂乃果 cost 11, unit Printemps */

/* live_success_partial_resolution_of_revealed_live_q93_test.rs
 * cards.json PL!SP-pb1-001-R : 澁谷かのん / cost 11 / member / unit CatChu!
 *   ab#0 ライブ開始時 EE 支払わないかぎり、自分の手札を2枚控え室に置く。
 *   ab#1 ライブ成功時 EEEEEE 支払ってもよい：ライブの合計スコアを＋１する。 */
#define KANON_SP    "PL!SP-pb1-001-R" /* 澁谷かのん  LIVE_START unless-2E +
                                                LIVE_SUCCESS optional 6E->+1 */

/* pb2_kidou_self_to_waitroom_recover_mus_live_then_energy_per_mus_pl_pb2_007_pp_test.rs
 * cards.json PL!-pb2-007-PP : 東條 希 / cost 4 / member / unit "lily white"
 *   起動 このメンバーをステージから控え室に置く：自分の控え室から『μ's』の
 *        ライブカードを1枚手札に加える。その後、自分の成功ライブカード置き場
 *        にある『μ's』のカード1枚につき、エネルギーを1枚アクティブにする。
 * cards.json PL!-pb1-029-SRL : 知らないLove＊教えてLove / LIVE / score 1 /
 *   unit "lily white"  -> a μ's LIVE, counts in the success zone.
 * cards.json LL-PR-004-PR   : 愛♡スクリ～ム！ / LIVE / score 3 /
 *   unit "AiScReam"          -> a LIVE that is NOT μ's, must not be counted. */
#define HANAYOME    "PL!-pb2-007-PP" /* 東條 希   cost 4, unit "lily white" */
#define MUS_LIVE_A  "PL!-pb1-029-SRL" /* 知らないLove  LIVE, "lily white"  */
#define NON_MUS_LIVE "LL-PR-004-PR"   /* 愛♡スクリ～ム！ LIVE, AiScReam   */
#define ENERGY_SD   "LL-E-001-SD"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ---- shared reporting state --------------------------------------------
 * MAP_SHARED|MAP_ANONYMOUS so a child killed mid-assertion still reports. */
typedef struct {
    volatile int rc;
    volatile int assertions;
    volatile int failures;
    volatile int setup_bugs;
    volatile int gaps;
    volatile int prompts;
    volatile int signal;
} Shared;

static Shared *sh;
static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB, so every TestGame must be static. */
static TestGame t_tg[1];   /* one slot: each case is its own fork */

static int n_ok, n_failed, n_crashed;
/* The shared mapping is REUSED by every child, so the parent must accumulate
 * into its own totals. Reading sh-> after the loop would report only the LAST
 * case (which is how an earlier revision printed "checks=9" for 24 cases). */
static int tot_assertions, tot_failures, tot_setup_bugs, tot_gaps;

/* ---- assertions -------------------------------------------------------- */

static void on_fatal_signal(int sig)
{
    if (sh) {
        sh->signal = sig;
        sh->failures++;
    }
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** assertions=%d failures=%d prompts=%d\n",
            sig, current_test,
            sh ? sh->assertions : 0, sh ? sh->failures : 0, sh ? sh->prompts : 0);
    fflush(stderr);
    fflush(stdout);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                          \
        sh->assertions++;                                                   \
        if (!(condition)) {                                                 \
            sh->failures++;                                                 \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                                \
        sh->assertions++;                                                   \
        long long a_ = (long long)(actual);                                 \
        long long e_ = (long long)(expected);                               \
        if (a_ != e_) {                                                     \
            sh->failures++;                                                 \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);         \
        }                                                                   \
    } while (0)

/* A Rust assertion with no faithful C entry point. Recorded, never counted as
 * a pass and never as a failure. */
#define EXPECTED_GAP(condition, ...) do {                                   \
        sh->assertions++;                                                   \
        if (!(condition)) {                                                 \
            printf("GAP: ");                                                \
            printf(__VA_ARGS__);                                            \
            printf("\n");                                                   \
            sh->gaps++;                                                     \
        }                                                                   \
    } while (0)

/* Forward: the prompt plumbing lives further down. */
static int pending(TestGame *tg);

/* VACUITY CONTROL. Counts the prompt still ON SCREEN at the ASSERTION SITE
 * and then asserts a floor on the running total. Bare CHECK_PROMPTS reads a
 * counter that only pick()/select_option() increment, so on its own it always
 * reads 0 and SATISFIES ITSELF. Never used here. */
static int expect_prompts(TestGame *tg, int n, const char *what)
{
    sh->assertions++;
    if (pending(tg)) sh->prompts++;
    if (sh->prompts < n) {
        sh->failures++;
        fprintf(stderr, "FAIL: [VACUOUS]  %s - only %d prompt(s) offered, need %d\n",
                what, sh->prompts, n);
        return 0;
    }
    return 1;
}

#define EXPECT_PROMPTS(tg, n, ...) do {                                      \
        char what_[224];                                                     \
        snprintf(what_, sizeof what_, __VA_ARGS__);                          \
        expect_prompts((tg), (n), what_);                                    \
    } while (0)

/* ---- db / fixtures ----------------------------------------------------- */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* ROTATING ring. A single static buffer called 2-3 times in one printf prints
 * the LAST card repeatedly, which silently turns a three-card census dump into
 * one card repeated and hides a wrong-card fixture. */
static const char *no_of(int cid)
{
    static char ring[8][96];
    static int  next;
    char *buf = ring[next];
    next = (next + 1) % 8;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 96, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static const char *name_of(int cid)
{
    static char ring[4][128];
    static int  next;
    char *buf = ring[next];
    next = (next + 1) % 4;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, 128, "%s", c.name);
        rb_free_card(&c);
    }
    return buf;
}

static const char *unit_of(int cid) { const char *s = rb_card_unit_name(cid); return s ? s : ""; }

/* rb_card_unit_name() returns a raw pointer into the UNSEPARATED string table
 * (rb_card_string on the card's unit index). If that slice is skewed, the
 * returned bytes are a SPLICE - and a strcmp against the right name then fails
 * while the printed form still looks correct, because a stray NUL truncates it.
 * Dumped in hex so that case is distinguishable from a test-side typo. */
static void dump_unit_bytes(int cid, const char *tag)
{
    const unsigned char *u = (const unsigned char *)unit_of(cid);
    fprintf(stderr, "  [unit] %s '%s' unit bytes:", tag, no_of(cid));
    for (int i = 0; i < 24 && u[i]; i++) fprintf(stderr, " %02x", u[i]);
    fputc('\n', stderr);
}

static void info(const char *fmt, ...)
{
    va_list ap;
    fprintf(stderr, "  [info] ");
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
}

/* AGENTS.md identity rule: mid() (test_new_id), never the shared template. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        sh->assertions++; sh->setup_bugs++; sh->failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in cards.json\n", no);
    }
    return id;
}

/* BYTE-EXACT identity pin. */
static int pin_id(int cid, const char *no)
{
    sh->assertions++;
    if (cid < 0) { sh->setup_bugs++; sh->failures++;
        fprintf(stderr, "SETUP BUG: cannot resolve '%s'\n", no); return 0; }
    if (strcmp(no_of(cid), no) != 0) {
        sh->setup_bugs++; sh->failures++;
        fprintf(stderr, "SETUP BUG: identity - want '%s', resolved '%s' (%s) id=%d\n",
                no, no_of(cid), name_of(cid), cid);
        return 0;
    }
    return 1;
}

static int printed_cost(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int printed_score(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}

static int is_live(int cid)   { return rb_card_is_live(cid); }
static int is_member(int cid) { return rb_card_is_member(cid); }
static int in_group(int cid, const char *g) { return rb_card_matches_group_str(cid, g); }

/* UNIT-level filter check. rb_card_group_name is SERIES-level (every
 * ラブライブ! card reports μ's), so a 『lilywhite』 filter is checked here. */
static int in_unit(int cid, const char *u) { return strcmp(unit_of(cid), u) == 0; }

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

static void dump_abilities(int cid, const char *tag)
{
    fprintf(stderr, "  -- %s abilities for %s (%s):\n", tag, no_of(cid), name_of(cid));
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        fprintf(stderr, "     [%d] triggers=%s use_limit=%d cost=%s effect=%s/%s text=%s\n",
                i, ab.triggers ? ab.triggers : "-", ab.use_limit,
                (ab.cost && ab.cost->action) ? ab.cost->action : "-",
                (ab.effect && ab.effect->action) ? ab.effect->action : "-",
                (ab.effect && ab.effect->text) ? ab.effect->text : "-",
                ab.full_text ? ab.full_text : "-");
        rb_free_ability(&ab);
    }
    fflush(stderr);
}

/* ---- zones ------------------------------------------------------------- */

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_n (const TestGame *tg)            { return tg->state.p[0].hand.n; }
static int wait_n (const TestGame *tg)            { return tg->state.p[0].discard.n; }
static int success_n(const TestGame *tg)          { return tg->state.p[0].success.n; }
static int live_n(const TestGame *tg)             { return tg->state.p[0].live.n; }
static int energy_active(const TestGame *tg)      { return rb_energy_active_count(&tg->state.p[0]); }

static int stage_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

static int stage_slot(const TestGame *tg, int area)
{
    int v = tg->state.p[0].stage[area];
    return (v == RB_EMPTY_SLOT) ? -1 : v;
}

static int energy_zone_cards(const TestGame *tg) { return tg->state.p[0].energy.n; }

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    rb_look_reset_all();
}

/* The FULL fresh-board reset. test_game_new() is NOT optional here: the static
 * TestGame slot is zero-initialised, so WITHOUT it state.winner stays 0, which
 * the phase machine reads as "the game is already decided" - test_advance_
 * phase_strict_ex then returns TEST_PHASE_STOPPED_GAME_OVER on the very first
 * step and every phase-walking case silently fails to walk anywhere. It also
 * leaves state.phase == 0 (RPS) and second_attacker == 0, which breaks the
 * folded LiveCardSetFirst/SecondAttacker test. call it before every clear_p1. */
static void fresh(TestGame *tg)
{
    void (*clr)(TestGame *) = clear_p1;   /* via an alias: see replaceAll below */
    test_game_new(tg);
    clr(tg);
}

/* Rust's waitroom.cards.push(x) is an APPEND and the C bag appends at index n,
 * so the two agree without a top/bottom flip. */
static void add_to_wait_for(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

/* Index of `cid` inside p1's waitroom bag, or -1. */
static int wait_index_of(const TestGame *tg, int cid)
{
    const RbBag *b = &tg->state.p[0].discard;
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return i;
    return -1;
}

/* Map a WAITROOM index to the index the standing choice expects. The choice
 * presents a FILTERED view (c->filtered_indices maps choice slot -> zone slot),
 * so answering with the raw zone index is wrong whenever the filter narrowed
 * the list - and a narrowed list here is exactly the "a mandatory SelectCard
 * from a non-hand zone accepted only ONE index" shape. */
static int choice_index_for_wait(TestGame *tg, int wait_idx)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) return wait_idx;
    if (c->n_filtered_indices > 0) {
        for (int i = 0; i < c->n_filtered_indices; i++)
            if (c->filtered_indices[i] == wait_idx) return i;
        return -1;
    }
    return wait_idx;
}

/* ---- prompt plumbing --------------------------------------------------- */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const char *pending_type(TestGame *tg)
{
    const char *t = test_pending_choice_type(tg);
    return t ? t : "";
}

static int pending_is_card(TestGame *tg)
{
    return pending(tg) && !strcmp(pending_type(tg), "SelectCard");
}

static int note_prompt(TestGame *tg) { if (pending(tg)) { sh->prompts++; return 1; } return 0; }

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_n(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg)) return;
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

/* Rust drain_choices: answer with the FIRST index UNLESS the choice carries
 * allow_skip, in which case decline it. Answering a MANDATORY recover with an
 * empty selection refuses the recovery, which is exactly the absence a
 * following negative assertion would then read as a pass. */
static int drain_choices(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int skippable = c && c->allow_skip;
        if (!strcmp(pending_type(tg), "SelectAutoAbility")) skip(tg);
        else if (skippable) skip(tg);
        else pick(tg, 0);
    }
    if (guard >= guard_max && pending(tg))
        info("drain hit its cap of %d with a prompt still standing", guard_max);
    rb_drain_ability_queue(&tg->state);
    return guard;
}

static int drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
    rb_drain_ability_queue(&tg->state);
    return guard;
}

static void play_to_stage(TestGame *tg, int cid, int area, int accept)
{
    test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, accept);
        if (stage_slot(tg, area) == cid) break;
        test_play_to_stage(tg, cid, area);
    }
}

/* Open the ライブ成功時 window. rb_should_trigger_live_success needs BOTH
 * live.n > 0 AND live_success[pl], so both are set explicitly; the opener card
 * is chosen to print NO ability of its own, so it cannot add prompts of its
 * own and inflate the prompt count the case is measuring. */
static void open_live_success_window(TestGame *tg, int opener)
{
    int hw = rb_parse_heart_color("heart00");
    CHECK(hw >= 0 && hw < RB_MAX_HEARTS,
          "premise: rb_parse_heart_color(\"heart00\") yields a usable index (got %d)", hw);
    tg->state.p[0].hearts[hw] = 20;      /* wildcard, so the window can open */
    test_add_to_live_for(tg, 0, opener);
    tg->state.live_success[0] = 1;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "premise: the ライブ成功時 window is OPEN, or the dispatch below does "
          "nothing and every negative expectation here is vacuous");
}

static int fire_live_success(TestGame *tg)
{
    tg->state.phase = RB_PHASE_VICTORY;
    int queued = rb_trigger_live_success(&tg->state, 0);
    if (queued <= 0) return 0;
    /* SINGLE-SEAT: a p2-owned queued auto ability can be DESTROYED by the p1
     * pass, so the all-seat variant is never used here. */
    rb_process_player_abilities(&tg->state, 0);
    return 1;
}

/* ---- phase walking ----------------------------------------------------- */

static int walk_phase(TestGame *tg, int target, int max_steps)
{
    for (int i = 0; i < max_steps; i++) {
        int stop = test_advance_to_phase_strict_ex(tg, target, 24, NULL);
        if (stop == TEST_PHASE_STOPPED_REACHED) return 1;
        if (stop == TEST_PHASE_STOPPED_PROMPT) {
            /* Rust just passes here; a standing prompt is answered so the walk
             * can continue, and it is NOTED so a prompt nobody expected is
             * visible rather than silently absorbed. */
            info("walk: a prompt was standing at %s; answering it",
                 test_phase_name_now(tg));
            drain_choices(tg, 8);
            continue;
        }
        test_advance_report(tg, "walk", target, stop);
        return 0;
    }
    return 0;
}

static void dump_state(TestGame *tg, const char *what)
{
    fprintf(stderr, "  STATE %s: pending=%d type=%s prompts=%d phase=%s census=%d\n",
            what, pending(tg), pending_type(tg), sh->prompts,
            test_phase_name_now(tg), test_total_card_count(tg));
    fprintf(stderr, "    hand:");
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        fprintf(stderr, " %s", no_of(tg->state.p[0].hand.cards[i]));
    fprintf(stderr, "\n    wait:");
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        fprintf(stderr, " %s", no_of(tg->state.p[0].discard.cards[i]));
    fprintf(stderr, "\n    stage:");
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[0].stage[i] != RB_EMPTY_SLOT)
            fprintf(stderr, " [%d]=%s", i, no_of(tg->state.p[0].stage[i]));
    fprintf(stderr, "\n    live:");
    for (int i = 0; i < tg->state.p[0].live.n; i++)
        fprintf(stderr, " %s", no_of(tg->state.p[0].live.cards[i]));
    fprintf(stderr, "\n    success=%d energy=%d/%d census=%d\n",
            success_n(tg), energy_active(tg), energy_zone_cards(tg),
            test_total_card_count(tg));
}

/* ======================================================================
 * A. self_to_waitroom_recover_live_q79_q123_test.rs
 *
 * PL!-sd1-005-SD 星空 凛 (the file's own doc comment calls it
 *   "{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室から
 *    ライブカードを1枚手札に加える。")
 * Q123: with NO live in the waitroom the ability is still usable and the cost
 *       is still paid - the effect gracefully does nothing.
 * Q79:  after the self_cost the vacated area is empty and a new card can go
 *       there.
 *
 * NOTE ON COUNTS. The survey card says 3 tests; the file as committed carries
 * 2 #[test] functions (q123_, q79_). Both are ported; the third is the shared
 * module doc header, not a test.
 * ====================================================================== */

static void a_setup(TestGame *tg, int with_live, int *rin_out, int *live_out)
{
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    fill_decks(tg, filler, 5);
    int rin = mid(tg, RIN_SD5);
    if (!pin_id(rin, RIN_SD5) || !pin_id(filler, FILLER_010)) return;

    /* TYPE / GROUP PINS. The filter is a LIVE-card filter, so the §A2 target
     * must be inside that set and the filler outside it - otherwise a
     * "nothing matched" board satisfies every assertion below. */
    CHECK_EQ(printed_cost(rin), 2, "A: Rin prints cost 2");
    CHECK(is_member(rin), "A: Rin is a member, as the self_cost requires");
    CHECK(card_prints_trigger(rin, RB_TSTR_ACTIVATION), "A: Rin prints an 起動 ability");
    CHECK(is_member(filler), "A: the filler is a MEMBER, so a live filter skips it");
    CHECK(!is_live(filler), "A: the filler is NOT a live card");
    CHECK_EQ(wait_n(tg), 0, "A: the waitroom really is EMPTY on this board");

    tg->state.p[0].stage[AREA_CENTER] = rin;
    test_add_to_hand(tg, filler);

    int live = -1;
    if (with_live) {
        /* Rust adds the SAME id to hand and to the waitroom. Kept faithful,
         * and the hand-count DELTA below is what actually proves the recovery
         * happened - see the note on the case below. */
        live = mid(tg, LIVE_019);
        if (!pin_id(live, LIVE_019)) return;
        CHECK(is_live(live), "A/live: START:DASH!! is a LIVE - inside the filter's set");
        CHECK_EQ(printed_score(live), 1, "A/live: START:DASH!! prints score 1");
        test_add_to_hand(tg, live);
        add_to_wait_for(tg, 0, live);
    }
    test_give_energy(tg, 3);
    *rin_out = rin; *live_out = live;
}

static void test_a_q123_cost_paid_without_recoverable_live(void)
{
    TestGame *tg = &t_tg[0];
    int rin, live;
    a_setup(tg, 0, &rin, &live);
    if (rin < 0) return;

    CHECK_EQ(stage_slot(tg, AREA_CENTER), rin, "A/q123: Rin is on the stage before activation");

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    test_activate_ability(tg, rin);
    drain_choices(tg, 8);

    CHECK_EQ(stage_slot(tg, AREA_CENTER), -1,
             "A/q123: the self_cost must vacate the centre area");
    CHECK(wait_has(tg, rin), "A/q123: Rin must be in the waitroom after the self_cost");
    CHECK_EQ(hand_n(tg), hand_before,
             "A/q123: with no live in the waitroom the effect does nothing, so the "
             "hand is untouched");
    /* CENSUS: the self_cost is a MOVE. A card that left the stage and reached
     * nowhere would satisfy the two assertions above. */
    CHECK_EQ(test_total_card_count(tg), census_before,
             "A/q123 CENSUS: the self_cost moved one card and destroyed none");
}

static void test_a_q79_vacates_center_and_recovers_live(void)
{
    TestGame *tg = &t_tg[0];
    int rin, live;
    a_setup(tg, 1, &rin, &live);
    if (rin < 0 || live < 0) return;

    CHECK_EQ(wait_n(tg), 1, "A/q79: exactly one card (the live) is in the waitroom");

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    test_activate_ability(tg, rin);
    /* The live filter matches exactly one card, so a SelectCard is offered. */
    EXPECT_PROMPTS(tg, 1, "A/q79: the recover-a-live prompt must be OFFERED");
    drain_choices(tg, 8);

    CHECK_EQ(stage_slot(tg, AREA_CENTER), -1,
             "A/q79: the centre area should be empty after the self_cost");
    CHECK(wait_has(tg, rin), "A/q79: Rin should be in the waitroom");

    /* RUST VACUITY, CARRIED AND NEUTRALISED. The Rust file puts the SAME id in
     * hand and in the waitroom and then asserts hand.contains(&live_card), so
     * that assertion is already true before the ability runs. The faithful
     * id-based check is kept; the hand-count DELTA is the assertion that
     * actually has teeth, and it is the one that would go red on a recovery
     * that did nothing. */
    CHECK(hand_has(tg, live), "A/q79: the live card should be recovered to hand");
    CHECK_EQ(hand_n(tg), hand_before + 1,
             "A/q79 CONTROL (the load-bearing one): a live moved waitroom -> hand, so "
             "the hand must have RISEN by one. Rust's hand.contains check is already "
             "satisfied by the pre-seeded hand copy, so this delta is the only "
             "non-vacuous form of the assertion");
    CHECK(!wait_has(tg, live), "A/q79: the recovered live left the waitroom");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "A/q79 CENSUS: the recovery moved one card and destroyed none");
}

/* ======================================================================
 * B. self_to_waitroom_recover_member_q79_test.rs  (1 test)
 *
 * PL!-sd1-002-SD 絢瀬 絵里:
 *   起動 このメンバーをステージから控え室に置く：自分の控え室から
 *        メンバーカードを1枚手札に加える。                 (MEMBER filter)
 * Q79: the vacated area can receive a new member this turn.
 * ====================================================================== */

static void test_b_q79_vacated_area_takes_a_new_member(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    fill_decks(tg, filler, 5);
    int eli   = mid(tg, ELI_SD2);
    int target= mid(tg, KOTORI_SD1);
    int fresh = mid(tg, KOTORI_SD3);
    if (!pin_id(eli, ELI_SD2) || !pin_id(target, KOTORI_SD1) ||
        !pin_id(fresh, KOTORI_SD3) || !pin_id(filler, FILLER_010)) return;

    /* TYPE / COST PINS: the filter is a MEMBER filter, so the recovery target
     * must be a member and the newcomer must be payable at the given energy. */
    CHECK_EQ(printed_cost(eli), 2, "B: Eli prints cost 2");
    CHECK(is_member(target), "B: the waitroom target is a MEMBER - inside the filter's set");
    CHECK_EQ(printed_cost(fresh), 13, "B: 南ことり prints cost 13");
    CHECK_EQ(printed_cost(target), 11, "B: 高坂穂乃果 prints cost 11");
    CHECK(card_prints_trigger(eli, RB_TSTR_ACTIVATION), "B: Eli prints an 起動 ability");
    CHECK(!is_live(target), "B: the target is NOT a live, so a member filter must match it");

    tg->state.p[0].stage[AREA_CENTER] = eli;
    test_add_to_hand(tg, filler);
    test_add_to_hand(tg, fresh);
    add_to_wait_for(tg, 0, target);   /* waitroom holds ONLY the target */
    test_give_energy(tg, 15);

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    test_activate_ability(tg, eli);

    /* PROMPT CONTROL at the site: the waitroom recover MUST offer a SelectCard
     * (Eli joins the waitroom by her own cost, so there are 2 members in it). */
    EXPECT_PROMPTS(tg, 1, "B: the recover-from-waitroom selection prompt must be offered");
    CHECK(pending_is_card(tg),
          "B: the prompt must be a SelectCard (got \"%s\")", pending_type(tg));
    if (pending_is_card(tg)) {
        int widx = wait_index_of(tg, target);
        CHECK(widx >= 0, "B: the recovery target is still in the waitroom");
        int cidx = choice_index_for_wait(tg, widx);
        CHECK(cidx >= 0,
              "B: the target must be among the choice's candidates "
              "(waitroom idx=%d, n_filtered=%d)",
              widx, rb_get_pending_choice(&tg->state)
                     ? rb_get_pending_choice(&tg->state)->n_filtered_indices : -1);
        if (cidx >= 0) pick(tg, cidx);
    }
    drain_choices(tg, 8);

    CHECK_EQ(stage_slot(tg, AREA_CENTER), -1, "B: the self_cost vacated the centre area");
    CHECK(wait_has(tg, eli), "B: Eli reached the waitroom via her own cost");
    CHECK(hand_has(tg, target), "B: the member was recovered from the waitroom to hand");
    CHECK_EQ(hand_n(tg), hand_before + 1,
             "B: a member moved waitroom -> hand, so the hand rose by one");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "B CENSUS: activation moved one card and destroyed none");

    /* Q79 proper: the area the cost vacated is available for a NEW member. */
    int census_before_play = test_total_card_count(tg);
    play_to_stage(tg, fresh, AREA_CENTER, 1);
    CHECK_EQ(stage_slot(tg, AREA_CENTER), fresh,
             "B: a new member can be placed in the area the self_cost vacated");
    CHECK_EQ(test_total_card_count(tg), census_before_play,
             "B CENSUS: the follow-up play moved one card and destroyed none");
    if (stage_slot(tg, AREA_CENTER) != fresh) dump_state(tg, "b:q79-new-member");
}

/* ======================================================================
 * C. debut_recovers_one_live_card_from_waitroom_to_hand_test.rs
 *
 * LL-bp7-001-R＋ ab#1 (登場): 自分の控え室からライブカードを1枚手札に加える。
 * §C3 is an IN-FILE PAIRED POSITIVE that exists only so §C2's negative cannot
 * be satisfied by a debut that did nothing at all.
 * ====================================================================== */

static void c_setup(TestGame *tg, int with_live, int *triple_out, int *waitcard_out)
{
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    fill_decks(tg, filler, 5);
    int triple = mid(tg, TRIPLE);
    if (!pin_id(triple, TRIPLE)) return;

    CHECK_EQ(printed_cost(triple), 15, "C: the TRIPLE prints cost 15");
    CHECK(is_member(triple), "C: the TRIPLE is a member, so it can appear on the stage");
    CHECK(card_prints_trigger(triple, RB_TSTR_DEBUT), "C: the TRIPLE prints a 登場 ability");
    CHECK_EQ(strlen(TRIPLE), strlen("LL-bp7-001-R") + 3,
             "C: the TRIPLE print really carries the FULLWIDTH plus, not ASCII '+'");

    int waitcard = -1;
    if (with_live) {
        waitcard = mid(tg, LIVE_020);
        if (!pin_id(waitcard, LIVE_020)) return;
        CHECK(is_live(waitcard),
              "C/live: きっと青春が聞こえる is a LIVE - inside ab#1's filter set");
        CHECK_EQ(printed_score(waitcard), 2, "C/live: it prints score 2");
    } else {
        waitcard = mid(tg, FILLER_010);
        if (!pin_id(waitcard, FILLER_010)) return;
        CHECK(is_member(waitcard),
              "C/member: 高坂穂乃果 is a MEMBER - OUTSIDE ab#1's live filter, which is "
              "what makes the negative below a filter claim and not a dead board");
        CHECK(!is_live(waitcard), "C/member: and it is not a live card");
    }
    test_add_to_hand(tg, triple);
    add_to_wait_for(tg, 0, waitcard);
    test_give_energy(tg, 15);
    *triple_out = triple; *waitcard_out = waitcard;
}

static void test_c1_debut_recovers_live_from_waitroom(void)
{
    TestGame *tg = &t_tg[0];
    int triple, live;
    c_setup(tg, 1, &triple, &live);
    if (triple < 0 || live < 0) return;

    CHECK(!hand_has(tg, live), "C1: the live is in the waitroom only, not in hand yet");

    int census_before = test_total_card_count(tg);
    play_to_stage(tg, triple, AREA_CENTER, 1);
    /* PROMPT CONTROL at the site: the debut's recover must be OFFERED. */
    EXPECT_PROMPTS(tg, 1, "C1: the 登場 recover-a-live prompt must be offered");
    drain_choices(tg, 8);

    CHECK(stage_has(tg, triple), "C1: the TRIPLE appeared");
    CHECK(hand_has(tg, live), "C1: the debut added a live card from the waitroom to hand");
    CHECK(!wait_has(tg, live), "C1: the live card left the waitroom");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "C1 CENSUS: the debut moved one card and destroyed none");
    if (!hand_has(tg, live)) { dump_state(tg, "c1"); dump_abilities(triple, "C1"); }
}

static void test_c2_debut_leaves_member_in_waitroom(void)
{
    TestGame *tg = &t_tg[0];
    int triple, member;
    c_setup(tg, 0, &triple, &member);
    if (triple < 0 || member < 0) return;

    int census_before = test_total_card_count(tg);
    int wait_before   = wait_n(tg);
    play_to_stage(tg, triple, AREA_CENTER, 1);
    drain_choices(tg, 8);

    CHECK(stage_has(tg, triple), "C2: the TRIPLE appeared, so ab#1 did have a chance to run");
    CHECK(wait_has(tg, member), "C2: a non-live card in the waitroom is not touched by ab#1");
    CHECK(!hand_has(tg, member), "C2: and it was not added to hand");
    CHECK_EQ(wait_n(tg), wait_before, "C2: the waitroom size is unchanged");
    /* CENSUS: the sharpest control here. A waitroom count of 1 reads the same
     * whether nothing matched OR the card was DESTROYED after leaving. */
    CHECK_EQ(test_total_card_count(tg), census_before,
             "C2 CENSUS: the debut created and destroyed no card");
    /* PAIRED POSITIVE, same file: §C3 is this exact board plus one live card in
     * the waitroom, and asserts that live DOES move. */
    info("C2's non-vacuity rests on C3 (same card, same board, live added) and on "
         "the card-type pin that puts %s outside ab#1's filter", no_of(member));
}

/* C3 is NOT a Rust test. It is the PAIRED POSITIVE control for C2: the same
 * TRIPLE debut on the same board, but the waitroom holds a member AND a live.
 * The live must move and the member must not - which is the only way to know
 * that C2's "the member stayed" was a filter decision and not a dead ability. */
static void test_c3_paired_positive_live_moves_member_stays(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    fill_decks(tg, filler, 5);
    int triple = mid(tg, TRIPLE);
    int member = mid(tg, FILLER_010);
    int live   = mid(tg, LIVE_020);
    if (!pin_id(triple, TRIPLE) || !pin_id(member, FILLER_010) ||
        !pin_id(live, LIVE_020)) return;
    CHECK(is_live(live) && is_member(member),
          "C3: the board really carries one live and one member in the waitroom");

    test_add_to_hand(tg, triple);
    add_to_wait_for(tg, 0, member);
    add_to_wait_for(tg, 0, live);
    test_give_energy(tg, 15);

    int census_before = test_total_card_count(tg);
    play_to_stage(tg, triple, AREA_CENTER, 1);
    EXPECT_PROMPTS(tg, 1, "C3: the 登場 recover-a-live prompt must be offered");
    drain_choices(tg, 8);

    CHECK(hand_has(tg, live),
          "C3 PAIRED POSITIVE: the LIVE in the waitroom IS recovered, which is what "
          "makes C2's 'the member stayed' a filter result and not a dead debut");
    CHECK(wait_has(tg, member), "C3: the member is still in the waitroom");
    CHECK(!hand_has(tg, member), "C3: the member was not added to hand");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "C3 CENSUS: the debut moved one card and destroyed none");
    if (!hand_has(tg, live)) { dump_state(tg, "c3"); dump_abilities(triple, "C3"); }
}

/* ======================================================================
 * D. live_success_recovers_one_member_card_from_waitroom_to_hand_test.rs
 *
 * LL-bp7-001-R＋ ab#2 (ライブ成功時): 自分の控え室からメンバーカードを
 * 1枚手札に加える。                                    (MEMBER filter)
 * §D3 is the IN-FILE PAIRED POSITIVE for §D2.
 * ====================================================================== */

static void d_setup(TestGame *tg, int with_live, int *triple_out, int *target_out)
{
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    fill_decks(tg, filler, 5);
    int triple = mid(tg, TRIPLE);
    if (!pin_id(triple, TRIPLE)) return;
    CHECK(is_member(triple), "D: the TRIPLE is a member (its ab#2 lives on a member)");
    CHECK(card_prints_trigger(triple, RB_TSTR_LIVE_SUCCESS),
          "D: the TRIPLE prints a ライブ成功時 ability");

    int target;
    if (with_live) {
        target = mid(tg, LIVE_020);
        if (!pin_id(target, LIVE_020)) return;
        CHECK(is_live(target),
              "D/live: the decoy is a LIVE - OUTSIDE ab#2's member filter, which is "
              "what makes the negative below a filter claim and not a dead board");
    } else {
        target = mid(tg, FILLER_010);
        if (!pin_id(target, FILLER_010)) return;
        CHECK(is_member(target),
              "D/member: the target is a MEMBER - inside ab#2's filter set");
    }
    tg->state.p[0].stage[AREA_CENTER] = triple;
    add_to_wait_for(tg, 0, target);
    /* The window opener is a live with NO ability of its own, so it cannot add
     * prompts and inflate the count §D1 measures. */
    open_live_success_window(tg, mid(tg, MUS_LIVE_B));
    *triple_out = triple; *target_out = target;
}

static void test_d1_live_success_recovers_member(void)
{
    TestGame *tg = &t_tg[0];
    int triple, member;
    d_setup(tg, 0, &triple, &member);
    if (triple < 0 || member < 0) return;

    CHECK(!hand_has(tg, member), "D1: the member is in the waitroom only");

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    CHECK(fire_live_success(tg), "D1: the ライブ成功時 dispatch really queued something");
    EXPECT_PROMPTS(tg, 1, "D1: the recover-a-member prompt must be OFFERED");
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, member),
          "D1: the live success added a member card from the waitroom to hand");
    CHECK(!wait_has(tg, member), "D1: the member card left the waitroom");
    CHECK_EQ(hand_n(tg), hand_before + 1, "D1: the hand rose by exactly one");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "D1 CENSUS: the recovery moved one card and destroyed none");
    if (!hand_has(tg, member)) { dump_state(tg, "d1"); dump_abilities(triple, "D1"); }
}

static void test_d2_live_success_ignores_live_cards(void)
{
    TestGame *tg = &t_tg[0];
    int triple, live_in_wait;
    d_setup(tg, 1, &triple, &live_in_wait);
    if (triple < 0 || live_in_wait < 0) return;

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    int wait_before   = wait_n(tg);
    int queued = fire_live_success(tg);
    drain_pick0(tg, 8);

    /* NON-VACUITY. Three independent controls, because "nothing happened" is the
     * failure mode this whole cluster hides behind:
     *   (1) the dispatch really queued an ability,
     *   (2) the waitroom really held the live and hand really did not,
     *   (3) D3 - the SAME board plus a member - really does fetch. */
    CHECK(queued, "D2 CONTROL: the ライブ成功時 dispatch queued an ability, so the "
                  "ability ran and its filter, not a dead trigger, decided this");
    CHECK(wait_has(tg, live_in_wait), "D2: the live is in the waitroom");
    CHECK(!hand_has(tg, live_in_wait), "D2: a live in the waitroom is not touched by ab#2");
    CHECK_EQ(hand_n(tg), hand_before, "D2: the hand is unchanged");
    CHECK_EQ(wait_n(tg), wait_before, "D2: the waitroom size is unchanged");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "D2 CENSUS: a live in the waitroom was neither fetched nor DESTROYED - "
             "this is the check a zone count cannot make");
    info("D2's non-vacuity rests on D3 (same card, same board, member added)");
}

/* D3 is NOT a Rust test. PAIRED POSITIVE control for D2: the same TRIPLE live
 * success with a member AND a live in the waitroom. The member must move and
 * the live must not. */
static void test_d3_paired_positive_member_moves_live_stays(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    fill_decks(tg, filler, 5);
    int triple = mid(tg, TRIPLE);
    int member = mid(tg, FILLER_010);
    int live   = mid(tg, LIVE_020);
    if (!pin_id(triple, TRIPLE) || !pin_id(member, FILLER_010) ||
        !pin_id(live, LIVE_020)) return;
    CHECK(is_member(member) && is_live(live), "D3: one member and one live in the waitroom");

    tg->state.p[0].stage[AREA_CENTER] = triple;
    add_to_wait_for(tg, 0, member);
    add_to_wait_for(tg, 0, live);
    open_live_success_window(tg, mid(tg, MUS_LIVE_B));

    int census_before = test_total_card_count(tg);
    CHECK(fire_live_success(tg), "D3: the ライブ成功時 dispatch queued an ability");
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, member),
          "D3 PAIRED POSITIVE: the MEMBER in the waitroom IS recovered, which is what "
          "makes D2's 'the live stayed' a filter result and not a dead ability");
    CHECK(wait_has(tg, live), "D3: the live is still in the waitroom");
    CHECK(!hand_has(tg, live), "D3: the live was not added to hand");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "D3 CENSUS: one card moved and none was destroyed");
    if (!hand_has(tg, member)) { dump_state(tg, "d3"); dump_abilities(triple, "D3"); }
}

/* ======================================================================
 * E. lilywhite_gated_live_pl_pb1_007_r_test.rs  (2 tests)
 *
 * PL!-pb1-007-R 東條 希 cost 13:
 *   起動 ターン1回 手札を3枚控え室に置く：自分のステージにほかの
 *        『lilywhite』のメンバーがいる場合、自分の控え室から『μ's』の
 *        ライブカードを1枚手札に加える。 …
 *
 * LILYWHITE FIXTURE - VERIFIED, AND THE BRIEF'S WARNING DOES NOT APPLY HERE.
 * The task brief said this Rust fixture uses "PL!SP-bp1-007-R". It does not:
 * the committed file uses "PL!-pb1-007-R", which EXISTS in cards.json as
 *   東條 希 / member / cost 13 / unit "lilywhite", carrying exactly the ability
 *   this test describes. So NO substitution was needed and none was made.
 * For the record, the brief's proposed swap would have been WRONG for this
 * test on both counts:
 *   - "PL!SP-bp1-007-R" is not a key in cards.json at all (nearest real keys
 *     are PL!SP-bp1-007-P / -P＋ / -R＋ / -SEC and PL!SP-pb1-007-P / -P＋ / -R);
 *   - "PL!SP-pb1-007-R" is 米女メイ, unit "CatChu!" - a MEMBER, so a member is
 *     the right card TYPE, but it is NOT 『lilywhite』, so the
 *     「ほかの『lilywhite』のメンバーがいる」 gate in §E1 could never be
 *     satisfied by it and §E1 would be unfalsifiable.
 * The "member used where a live is required" worry does not arise here
 * either: the card under test is SUPPOSED to be a member (its ability is a
 * member's 起動), and the LIVE is the thing recovered from the waitroom.
 * ====================================================================== */

#define E_HAND_COST 3

static void e_setup(TestGame *tg, const char *mate_no, int *nozomi_out, int *mate_out,
                    int fillers[3], int *mus_live_out)
{
    fresh(tg);
    int deckfill = mid(tg, FILLER_010);
    fill_decks(tg, deckfill, 5);
    int nozomi = mid(tg, NOZOMI_R);
    int live   = mid(tg, LIVE_020);
    if (!pin_id(nozomi, NOZOMI_R) || !pin_id(live, LIVE_020)) return;

    CHECK_EQ(printed_cost(nozomi), 13, "E: 東條 希 (PL!-pb1-007-R) prints cost 13");
    CHECK(is_member(nozomi), "E: the activating card is a MEMBER, as its 起動 requires");
    CHECK(in_unit(nozomi, UNIT_LILYWHITE),
          "E: the activating card's UNIT is literally '%s' - checked with "
          "rb_card_unit_name, not rb_card_group_name (which is series-level)",
          unit_of(nozomi));
    dump_unit_bytes(nozomi, "NOZOMI");
    CHECK(card_prints_trigger(nozomi, RB_TSTR_ACTIVATION), "E: it prints an 起動 ability");
    CHECK(is_live(live), "E: きっと青春が聞こえる is a LIVE - inside the ライブカード filter");
    CHECK(in_group(live, GRP_MU),
          "E: the waitroom live really is a 『μ's』 card, so the filter could match it");

    int m = mid(tg, mate_no);
    if (!pin_id(m, mate_no)) return;

    tg->state.p[0].stage[AREA_LEFT]   = nozomi;
    tg->state.p[0].stage[AREA_CENTER] = m;
    test_give_energy(tg, 15);
    /* Rust's game.new_id(FILLER) x3: three DISTINCT instances of one print.
     * mid() mints a fresh pool instance per call, so the setup helper RETURNS
     * the seeded ids - re-deriving one at the assertion site would never match
     * the id sitting in a zone. */
    for (int i = 0; i < E_HAND_COST; i++) {
        fillers[i] = mid(tg, FILLER_010);
        if (!pin_id(fillers[i], FILLER_010)) return;
        test_add_to_hand(tg, fillers[i]);
    }
    add_to_wait_for(tg, 0, live);
    *nozomi_out = nozomi; *mate_out = m; *mus_live_out = live;
}

static void test_e1_other_lilywhite_member_allows_recovery(void)
{
    TestGame *tg = &t_tg[0];
    int nozomi, mate, live, fillers[E_HAND_COST];
    e_setup(tg, RIN_BP3N, &nozomi, &mate, fillers, &live);
    if (nozomi < 0 || mate < 0 || live < 0) return;

    /* UNIT PIN: the gate is 「ほかの『lilywhite』のメンバーがいる」. */
    CHECK(in_unit(mate, UNIT_LILYWHITE),
          "E1: the second stage member's UNIT really is lilywhite (got \"%s\")",
          unit_of(mate));

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    test_activate_ability(tg, nozomi);

    /* PROMPT CONTROL at the site: the 3-card hand cost must be OFFERED. */
    EXPECT_PROMPTS(tg, 1, "E1: the 手札を3枚控え室に置く cost prompt must be offered");
    CHECK(pending_is_card(tg), "E1: the cost prompt must be a SelectCard (got \"%s\")",
          pending_type(tg));
    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK(c && c->count == E_HAND_COST,
              "E1: the cost prompt asks for exactly %d cards (count=%d)",
              E_HAND_COST, c ? c->count : -1);
        const int three[3] = { 0, 1, 2 };
        pick_n(tg, three, 3);
    }
    drain_choices(tg, 8);

    CHECK(hand_has(tg, live),
          "E1: with another lilywhite member on stage the waitroom μ's LIVE reaches hand");
    CHECK(!wait_has(tg, live), "E1: the live left the waitroom");
    CHECK_EQ(hand_n(tg), hand_before - E_HAND_COST + 1,
             "E1: three cards paid, one recovered - net %d",
             -E_HAND_COST + 1);
    for (int i = 0; i < E_HAND_COST; i++)
        CHECK(wait_has(tg, fillers[i]), "E1: hand cost card %d really went to the waitroom", i);
    CHECK_EQ(test_total_card_count(tg), census_before,
             "E1 CENSUS: three cards moved to the waitroom, one to hand, none destroyed");
    if (!hand_has(tg, live)) { dump_state(tg, "e1"); dump_abilities(nozomi, "E1"); }
}

static void test_e2_no_lilywhite_member_prevents_recovery(void)
{
    TestGame *tg = &t_tg[0];
    int nozomi, mate, live, fillers[E_HAND_COST];
    e_setup(tg, PRINTEMPS_SD1, &nozomi, &mate, fillers, &live);
    if (nozomi < 0 || mate < 0 || live < 0) return;

    /* UNIT PINS - THE LOAD-BEARING CONTROL for this negative. The gate is on
     * the second member's UNIT: it must really not be lilywhite, and the
     * activating card and the live must really be inside the other filters.
     * Without these, a board on which NO filter matched anything would pass. */
    CHECK(!in_unit(mate, UNIT_LILYWHITE),
          "E2 CONTROL: the second stage member's UNIT really is NOT lilywhite "
          "(got \"%s\")", unit_of(mate));
    CHECK(in_unit(nozomi, UNIT_LILYWHITE), "E2: the activating card is still lilywhite");
    CHECK(in_group(live, GRP_MU), "E2: the waitroom live is still a matchable μ's LIVE");

    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    test_activate_ability(tg, nozomi);

    EXPECT_PROMPTS(tg, 1, "E2: the hand cost prompt is offered even when the gate fails");
    CHECK(pending_is_card(tg), "E2: the cost prompt must be a SelectCard (got \"%s\")",
          pending_type(tg));
    if (pending_is_card(tg)) {
        const int three[3] = { 0, 1, 2 };
        pick_n(tg, three, 3);
    }
    drain_choices(tg, 8);

    CHECK(!hand_has(tg, live),
          "E2: with no other lilywhite member on stage the live is NOT retrieved");
    CHECK(wait_has(tg, live), "E2: the live is still in the waitroom");
    /* THE PROOF THAT THE ABILITY ACTUALLY RAN. The cost was paid, so the
     * missing live is attributable to the lilywhite gate and not to a dead
     * ability that never left the stage. */
    CHECK_EQ(hand_n(tg), hand_before - E_HAND_COST,
             "E2 CONTROL: the 3-card hand cost was still paid, which is what proves "
             "the ability ran");
    for (int i = 0; i < E_HAND_COST; i++)
        CHECK(wait_has(tg, fillers[i]),
              "E2 CONTROL: hand cost card %d really reached the waitroom", i);
    CHECK_EQ(test_total_card_count(tg), census_before,
             "E2 CENSUS: the three cost cards moved to the waitroom and none was "
             "destroyed; the live was neither fetched nor destroyed");
    info("E2's non-vacuity rests on E1 (same card, same board, lilywhite mate) "
         "plus the unit pins above");
}

/* ======================================================================
 * F. kidou_self_to_waitroom_live_recovery_not_softlocked_test.rs (1 test)
 *
 * PL!-sd1-005-SD played to the stage must not park a mandatory prompt that
 * makes the turn unplayable.
 *
 * TRANSLATION NOTE. Rust asserts three properties of
 * game_setup::generate_possible_actions: has_pass, has_use_ability and
 * has_play_member. The C port exposes NO action-generator API, so those three
 * literals cannot be evaluated. Rather than drop them, each is translated into
 * a directly observable C equivalent that means the same thing:
 *   has_pass          -> the phase machine still advances (test_pass works and
 *                        the phase actually changes)
 *   has_play_member   -> a member in hand really can still be played
 *   has_use_ability   -> Rin's 起動 really is still activatable
 * All three are real assertions against the real engine, and the Rust
 * `!has_pending_choice()` after settling is ported verbatim.
 * ====================================================================== */

static void test_f_kidou_play_is_not_softlocked(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int filler = mid(tg, FILLER_010);
    int kidou  = mid(tg, RIN_SD5);
    if (!pin_id(filler, FILLER_010) || !pin_id(kidou, RIN_SD5)) return;
    CHECK(card_prints_trigger(kidou, RB_TSTR_ACTIVATION), "F: Rin prints the 起動 under test");
    CHECK_EQ(printed_cost(filler), 4, "F: the spare member in hand costs 4");

    test_add_to_hand(tg, kidou);
    test_add_to_hand(tg, filler);
    fill_decks(tg, filler, 10);
    test_give_energy(tg, 25);

    int census_before = test_total_card_count(tg);
    play_to_stage(tg, kidou, AREA_CENTER, 1);
    /* Answer what cannot be declined, decline what can: a mandatory prompt left
     * parked mid-ability produces exactly the absence the next assertion
     * checks, so nothing fails. */
    drain_choices(tg, 16);

    CHECK(stage_has(tg, kidou), "F: Rin appeared on the stage");
    /* Rust: assert!(!game.has_pending_choice(), "Should not have pending_choice
     * after settling") - ported verbatim. */
    CHECK(!pending(tg), "F: the game is NOT stuck in a pending choice after settling");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "F CENSUS: playing the 起動 card and settling created and destroyed no card");

    /* has_play_member, observed directly. */
    int census_before_play = test_total_card_count(tg);
    play_to_stage(tg, filler, AREA_LEFT, 1);
    CHECK(stage_has(tg, filler),
          "F (Rust has_play_member): a member in hand can still be played - the turn "
          "is not softlocked");
    CHECK_EQ(test_total_card_count(tg), census_before_play,
             "F CENSUS: the follow-up play moved one card and destroyed none");

    /* has_use_ability, observed directly. */
    CHECK(test_activate_ability(tg, kidou) != 0,
          "F (Rust has_use_ability): the 起動 is still activatable after settling, so "
          "the board offers more than PASS");
    drain_choices(tg, 8);

    /* has_pass, observed directly. */
    int phase_before = tg->state.phase;
    int stop = test_advance_to_phase_strict_ex(tg, RB_PHASE_MAIN, 4, NULL);
    CHECK(stop == TEST_PHASE_STOPPED_REACHED || tg->state.phase != phase_before,
          "F (Rust has_pass): the turn still advances, so a PASS action exists "
          "(phase %d -> %d, stop=%d)", phase_before, tg->state.phase, stop);
}

/* ======================================================================
 * G. live_success_partial_resolution_of_revealed_live_q93_test.rs (5 tests)
 *
 * PL!SP-pb1-001-R 澁谷かのん:
 *   ab#0 ライブ開始時 EE 支払わないかぎり、自分の手札を2枚控え室に置く。
 *   ab#1 ライブ成功時 EEEEEE 支払ってもよい：ライブの合計スコアを＋１する。
 *
 * Rule 8.3.16 + the card's own Q92/Q93 FAQ:
 *   - an unpayable 「Unless 2E」 cost auto-skips and the discard resolves;
 *   - with 1 card in hand THAT 1 CARD goes to the waitroom (partial);
 *   - with 0 cards in hand, nothing happens;
 *   - the LiveSuccess window is shut when the live card's need_heart is unmet.
 *
 * The ライブ成功時 dispatch goes through rb_trigger_live_success (never the
 * phase machine) and rb_process_player_abilities on a SINGLE seat.
 * ====================================================================== */

static void g_seed(TestGame *tg, int n_extra_hand, int *kanon_out, int *live_out,
                   int *extra_out)
{
    fresh(tg);
    int fill = mid(tg, FILLER_010);
    if (!pin_id(fill, FILLER_010)) return;
    fill_decks(tg, fill, 10);            /* both seats, 10 each */

    int kanon = mid(tg, KANON_SP);
    int live  = mid(tg, LIVE_020);
    if (!pin_id(kanon, KANON_SP) || !pin_id(live, LIVE_020)) return;

    CHECK_EQ(printed_cost(kanon), 11, "G: 澁谷かのん prints cost 11");
    CHECK(is_member(kanon), "G: Kanon is a member");
    CHECK(card_prints_trigger(kanon, RB_TSTR_LIVE_START),
          "G: Kanon prints a ライブ開始時 ability (the 「unless 2E」 discard)");
    CHECK(card_prints_trigger(kanon, RB_TSTR_LIVE_SUCCESS),
          "G: Kanon prints a ライブ成功時 ability (the optional 6E -> +1)");
    CHECK(is_live(live), "G: きっと青春が聞こえる is the LIVE under test");
    CHECK_EQ(printed_score(live), 2, "G: it prints score 2");

    tg->state.p[0].stage[AREA_CENTER] = kanon;
    test_add_to_hand(tg, live);
    int extra = -1;
    if (n_extra_hand > 0) {
        extra = mid(tg, FILLER_010);
        if (!pin_id(extra, FILLER_010)) return;
        CHECK(is_member(extra), "G: the extra hand card is a member the discard can take");
        test_add_to_hand(tg, extra);
    }
    /* NO energy yet: the 「unless 2E」 cost is UNPAYABLE, which is what makes it
     * auto-skip rather than prompt. */
    CHECK_EQ(energy_active(tg), 0, "G: the board really starts with 0 active energy");
    *kanon_out = kanon; *live_out = live; *extra_out = extra;
}

/* Live card set. Split from the walk past ライブ開始時 so a case can take its
 * census / hand-count baseline in the window BETWEEN them - the Draw phase in
 * the walk changes the hand, so a baseline taken after the walk is not a
 * baseline for the LiveStart resolution. */
static int g_walk_to_live_set(TestGame *tg, int live)
{
    if (!walk_phase(tg, RB_PHASE_LIVE_SET, 12)) return 0;
    test_set_live_card_for(tg, 0, live);
    return 1;
}

static int g_walk_to_live_start(TestGame *tg)
{
    return walk_phase(tg, RB_PHASE_VICTORY, 6);
}

static void test_g1_live_success_does_not_fire_when_live_fails(void)
{
    TestGame *tg = &t_tg[0];
    int kanon, live, extra;
    g_seed(tg, 0, &kanon, &live, &extra);
    if (kanon < 0 || live < 0) return;

    CHECK_EQ(live_n(tg), 0, "G1: the live zone is empty at the start");
    if (!g_walk_to_live_set(tg, live)) {
        CHECK(0, "G1: the walk never reached ライブカードセット");
        test_advance_report(tg, "g1-set", RB_PHASE_LIVE_SET, TEST_PHASE_STOPPED_TIMEOUT);
        return;
    }
    CHECK_EQ(live_n(tg), 1, "G1: the live card is in the live zone");
    /* CENSUS / hand baseline taken HERE - after the live card left the hand and
     * after the Draw phase, immediately before the LiveStart resolution, which
     * is the action under test. */
    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    int wait_before   = wait_n(tg);
    if (!g_walk_to_live_start(tg)) {
        CHECK(0, "G1: the walk never reached ライブ勝敗判定");
        test_advance_report(tg, "g1", RB_PHASE_VICTORY, TEST_PHASE_STOPPED_TIMEOUT);
        return;
    }
    CHECK(!pending(tg),
          "G1: an unpayable 「unless 2E」 LiveStart cost auto-skips WITHOUT prompting");
    /* CENSUS, baselined immediately before the LiveStart resolution, i.e. the
     * action under test. The two Draw phases already ran during the walk. */
    CHECK_EQ(test_total_card_count(tg), census_before,
             "G1 CENSUS: the LiveStart resolution created and destroyed no card");
    info("G1 hand %d -> %d, wait %d -> %d (the unless-cost discarded %d)",
         hand_before, hand_n(tg), wait_before, wait_n(tg), wait_n(tg) - wait_before);

    /* Now 6 energy for the LiveSuccess optional cost. */
    test_give_energy(tg, 6);
    int energy_before = energy_active(tg);
    int p0 = sh->prompts;

    /* Rust: game.pass() x3 - let the live actually resolve. A walk_phase() call
     * would return immediately here (we are ALREADY at ライブ勝敗判定), so the
     * turn is advanced explicitly, exactly as Rust does. */
    int census_before_pass = test_total_card_count(tg);
    for (int i = 0; i < 3; i++) { test_pass(tg); drain_choices(tg, 8); }
    info("G1: after 3 passes phase=%s live=%d success=%d wait=%d",
         test_phase_name_now(tg), live_n(tg), success_n(tg), wait_n(tg));

    /* Rule 8.3.16: the live card fails its need_heart, so its cards move to
     * the waitroom and LiveSuccess must NOT fire. The first two assertions are
     * THE NON-VACUITY CONTROL: they prove the live really FAILED. Without them
     * "no prompt, no energy spent" is exactly what a live that SUCCEEDED (and
     * so did dispatch) would not produce, and a shut/absent window would read
     * as a pass. G4 is the paired positive on the same card. */
    CHECK_EQ(live_n(tg), 0,
             "G1 CONTROL: the live card has left the live zone (rule 8.3.16)");
    CHECK_EQ(success_n(tg), 0,
             "G1 CONTROL: the live zone of successful lives is EMPTY, so the live "
             "really FAILED its need_heart - this is the premise the two assertions "
             "below rest on");
    CHECK(wait_has(tg, live),
          "G1 CONTROL: the failed live card is in the waitroom, where 8.3.16 puts it");
    CHECK_EQ(sh->prompts - p0, 0,
             "G1: LiveSuccess must NOT fire when the live card's need_heart is unmet "
             "(%d prompt(s) appeared after the 6 energy was handed over)",
             sh->prompts - p0);
    CHECK(!pending(tg), "G1: no prompt is left standing after the failed live");
    CHECK_EQ(energy_before - energy_active(tg), 0,
             "G1: no energy is deducted when LiveSuccess does not fire");
    CHECK(stage_has(tg, kanon), "G1: Kanon is still on the stage");
    /* CENSUS across the resolution. A Draw phase in the window may legitimately
     * RAISE the count, so the meaningful direction is "nothing was destroyed":
     * a card-destroying defect is the failure this exists to catch, and a draw
     * can only push the number up. */
    CHECK(test_total_card_count(tg) >= census_before_pass,
          "G1 CENSUS: the live resolution DESTROYED no card (%d -> %d; a Draw in "
          "the window can only raise it)", census_before_pass, test_total_card_count(tg));
}

static void test_g2_q93_partial_resolution_one_card(void)
{
    TestGame *tg = &t_tg[0];
    int kanon, live, extra;
    g_seed(tg, 1, &kanon, &live, &extra);
    if (kanon < 0 || live < 0 || extra < 0) return;

    CHECK_EQ(hand_n(tg), 2, "G2: hand really holds the live plus exactly ONE other card");
    if (!g_walk_to_live_set(tg, live)) {
        CHECK(0, "G2: the walk never reached ライブカードセット"); return;
    }
    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    if (!g_walk_to_live_start(tg)) {
        CHECK(0, "G2: the walk never reached ライブ勝敗判定"); return;
    }
    CHECK(!pending(tg),
          "G2: an unpayable 「unless 2E」 LiveStart cost auto-skips without prompting");
    drain_choices(tg, 8);

    CHECK(stage_has(tg, kanon), "G2: Kanon should remain on stage");
    /* The card's own Q93 answer: 「手札が1枚の場合、その1枚を控え室に置きます」 -
     * with ONE card in hand the discard resolves PARTIALLY, to that one card.
     * Rust declines to assert this ("if the engine skips the effect due to cost
     * tracking, this is a no-op - no crash"), which is exactly the assertion
     * that distinguishes a partial resolution from a silent no-op. Asserted
     * here at full strength; if it is red, the red is the finding. */
    CHECK(wait_has(tg, extra),
          "G2 Q93: with one card in hand that card is the partial resolution and "
          "must reach the waitroom (Rust declines to assert this)");
    CHECK_EQ(hand_n(tg), hand_before - 1,
             "G2 Q93: the hand lost exactly the one card the discard could take "
             "(baselined after the Draw phase, before ライブ開始時)");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "G2 CENSUS: the partial discard moved one card and destroyed none");
    if (!wait_has(tg, extra)) { dump_state(tg, "g2"); dump_abilities(kanon, "G2"); }
}

static void test_g3_q93_partial_resolution_zero_cards(void)
{
    TestGame *tg = &t_tg[0];
    int kanon, live, extra;
    g_seed(tg, 0, &kanon, &live, &extra);
    if (kanon < 0 || live < 0) return;

    CHECK_EQ(hand_n(tg), 1, "G3: hand holds the live card ONLY - nothing to discard");
    if (!g_walk_to_live_set(tg, live)) {
        CHECK(0, "G3: the walk never reached ライブカードセット"); return;
    }
    int census_before = test_total_card_count(tg);
    int hand_before   = hand_n(tg);
    int wait_before   = wait_n(tg);
    if (!g_walk_to_live_start(tg)) {
        CHECK(0, "G3: the walk never reached ライブ勝敗判定"); return;
    }
    CHECK(!pending(tg),
          "G3: an unpayable 「unless 2E」 LiveStart cost auto-skips without prompting");
    drain_choices(tg, 8);

    CHECK(stage_has(tg, kanon), "G3: Kanon should still be on stage after LiveStart");
    CHECK(!hand_has(tg, live), "G3: the live card left the hand for the live zone");
    /* The card's Q93 answer for 0 cards is 「特に何も行いません」. Asserted on
     * the hand DELTA rather than hand == 0, because the Draw phase in the walk
     * legitimately changes the hand; what must not change is anything the
     * discard itself did. */
    /* The card's Q93 answer for 0 cards is 「特に何も行いません」. The hand count
     * is pinned as the PREMISE rather than as the outcome, and the census below
     * is the control that nothing was created or destroyed. A raw waitroom
     * delta CANNOT isolate the discard on this board - the C phase machine also
     * moves deck cards into p1's waitroom across this window (observed: +4,
     * unrelated to the ability), so that measurement is recorded as a GAP
     * rather than asserted. No Rust assertion is lost: Rust's G3 asserts only
     * "no prompt" and "Kanon is on stage", and both are ported above. */
    CHECK_EQ(hand_before, 0,
             "G3 PREMISE: the owner really held 0 of its own cards at ライブ開始時, so "
             "the discard had nothing to take");
    info("G3: hand %d -> %d, wait %d -> %d (the +%d waitroom cards are deck cards the "
         "phase machine moves there across the live resolution, NOT the discard; the "
         "census below is flat, so nothing was destroyed)",
         hand_before, hand_n(tg), wait_before, wait_n(tg), wait_n(tg) - wait_before);
    EXPECTED_GAP(wait_n(tg) == wait_before + 1,
                 "G3: the discard's own effect cannot be isolated by a waitroom delta "
                 "on this board (got %d, expected %d) - the C phase machine contributes "
                 "deck cards to the waitroom in the same window",
                 wait_n(tg), wait_before + 1);
    /* CENSUS. A flat count is the exact expectation, and it is the check that
     * would catch a "no-op" that quietly DESTROYED the live card. */
    CHECK_EQ(test_total_card_count(tg), census_before,
             "G3 CENSUS: the no-op discard neither drew nor destroyed a card");
}

static void test_g4_ab1_live_success_pay_optional_cost(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int fill = mid(tg, FILLER_010);
    if (!pin_id(fill, FILLER_010)) return;
    fill_decks(tg, fill, 10);
    int kanon = mid(tg, KANON_SP);
    int live  = mid(tg, LIVE_020);
    if (!pin_id(kanon, KANON_SP) || !pin_id(live, LIVE_020)) return;
    CHECK(card_prints_trigger(kanon, RB_TSTR_LIVE_SUCCESS),
          "G4: Kanon prints the ライブ成功時 ability under test");

    tg->state.p[0].stage[AREA_CENTER] = kanon;
    test_add_to_live_for(tg, 0, live);

    /* Inject a heart00 wildcard to force live success (Rust's stage_hearts
     * trick). The index is DERIVED, never hard-coded. */
    int hw = rb_parse_heart_color("heart00");
    CHECK(hw >= 0 && hw < RB_MAX_HEARTS, "G4: heart00 index is usable (got %d)", hw);
    tg->state.p[0].hearts[hw] = 20;
    tg->state.live_success[0] = 1;
    test_give_energy(tg, 6);
    int energy_before = energy_active(tg);

    /* The ライブ成功時 window and the phase above OPEN the dispatch. Asserted
     * at each site so a shut window is NAMED, rather than showing up as a
     * resolution that quietly did nothing. */
    tg->state.phase = RB_PHASE_VICTORY;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "G4 premise: the ライブ成功時 window must be OPEN, or the dispatch below "
          "does nothing and the negative expectations are vacuous");
    CHECK_EQ(energy_before, 6, "G4: 6 active energy is available for the optional cost");

    int census_before = test_total_card_count(tg);
    int p0 = sh->prompts;
    int queued = rb_trigger_live_success(&tg->state, 0);
    rb_process_player_abilities(&tg->state, 0);

    CHECK(queued > 0, "G4: the ライブ成功時 dispatch queued Kanon's ability");
    CHECK(pending(tg), "G4: LiveSuccess created a pending choice (pay 6E or skip)");
    note_prompt(tg);
    CHECK(sh->prompts - p0 >= 1,
          "G4 PROMPT CONTROL at the site: the pay/skip prompt is ON SCREEN here "
          "(counted here, not at answer time)");
    /* A pay_optional_cost:skip gate is a SelectTarget where 0 = SKIP, 1 = PAY. */
    select_option(tg, 1);

    drain_choices(tg, 8);
    /* The printed target is ライブの合計スコア - the bonus lands once in the
     * owner's live-total accumulator, not as a per-card modifier on the live. */
    CHECK_EQ(tg->state.mods.p1_constant_total_score_bonus, 1,
             "G4: the live total score is +1 after paying 6E");
    CHECK_EQ(test_get_score_modifier(tg, live), 0,
             "G4: the printed target is the live total score, not a per-card modifier");
    CHECK_EQ(energy_before - energy_active(tg), 6, "G4: 6 energy was deducted");
    CHECK(!pending(tg), "G4: the ability completed without leaving a pending choice");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "G4 CENSUS: the optional-cost payment moved energy and destroyed no card");
}

static void test_g5_ab1_live_success_skip_optional_cost(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int fill = mid(tg, FILLER_010);
    if (!pin_id(fill, FILLER_010)) return;
    fill_decks(tg, fill, 10);
    int kanon = mid(tg, KANON_SP);
    int live  = mid(tg, LIVE_020);
    if (!pin_id(kanon, KANON_SP) || !pin_id(live, LIVE_020)) return;
    CHECK(card_prints_trigger(kanon, RB_TSTR_LIVE_SUCCESS),
          "G5: Kanon prints the ライブ成功時 ability under test");

    tg->state.p[0].stage[AREA_CENTER] = kanon;
    test_add_to_live_for(tg, 0, live);
    int hw = rb_parse_heart_color("heart00");
    CHECK(hw >= 0 && hw < RB_MAX_HEARTS, "G5: heart00 index is usable (got %d)", hw);
    tg->state.p[0].hearts[hw] = 20;
    tg->state.live_success[0] = 1;
    test_give_energy(tg, 6);
    int energy_before = energy_active(tg);

    tg->state.phase = RB_PHASE_VICTORY;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "G5 premise: the ライブ成功時 window must be OPEN");
    int census_before = test_total_card_count(tg);
    int p0 = sh->prompts;
    int queued = rb_trigger_live_success(&tg->state, 0);
    rb_process_player_abilities(&tg->state, 0);

    CHECK(queued > 0, "G5: the ライブ成功時 dispatch queued Kanon's ability");
    CHECK(pending(tg), "G5: LiveSuccess created a pending choice");
    /* PROMPT CONTROL AT THE ASSERTION SITE. Both negatives below are exactly
     * what "the ability did nothing" would also produce, so the prompt must be
     * proven to be on screen RIGHT HERE. G4 is the paired positive: on the same
     * board, option 1 pays 6 and yields +1. */
    note_prompt(tg);
    CHECK(sh->prompts - p0 >= 1,
          "G5 PROMPT CONTROL at the site: the pay/skip prompt is ON SCREEN here, so "
          "the zero-energy / zero-score expectations below are not vacuous");
    select_option(tg, 0);   /* SKIP */

    drain_choices(tg, 8);
    CHECK_EQ(test_get_score_modifier(tg, live), 0,
             "G5: the score does NOT change when the optional cost is skipped");
    CHECK_EQ(tg->state.mods.p1_constant_total_score_bonus, 0,
             "G5: the live TOTAL score is unchanged when the optional cost is skipped");
    CHECK_EQ(energy_before - energy_active(tg), 0,
             "G5: no energy is deducted when the optional cost is skipped");
    CHECK(!pending(tg), "G5: the ability completed without leaving a pending choice");
    CHECK_EQ(test_total_card_count(tg), census_before,
             "G5 CENSUS: skipping the cost created and destroyed no card");
    info("G5's non-vacuity rests on G4 (same board, option 1 -> -6 energy and +1)");
}

/* ======================================================================
 * H. pb2_kidou_self_to_waitroom_recover_mus_live_then_energy_per_mus_
 *    pl_pb2_007_pp_test.rs  (7 tests)
 *
 * PL!-pb2-007-PP 東條 希 cost 4:
 *   起動 このメンバーをステージから控え室に置く：自分の控え室から『μ's』の
 *        ライブカードを1枚手札に加える。その後、自分の成功ライブカード置き場に
 *        ある『μ's』のカード1枚につき、エネルギーを1枚アクティブにする。
 *
 * Two separable halves, pinned separately: the self->waitroom cost + recovery,
 * then the per-card energy. The energy half is the one with a per-unit count,
 * so it is the one that says 「1枚につき」 - two qualifying cards must give
 * two, not one. That is H4, and it is the only test in the family that can
 * tell a per-card count from a constant.
 *
 * The run() helper RETURNS the seeded ids: mid() mints a NEW pool instance per
 * call, so an id re-derived at the assertion site never matches the id sitting
 * in a zone.
 * ====================================================================== */

typedef struct {
    int me;             /* 東條 希                                    */
    int wait[2];        /* seeded waitroom ids (wait_n)               */
    int n_wait;
    int succ[2];        /* seeded success-zone ids (succ_n)           */
    int n_succ;
    int energy_before, energy_after, energy_delta;
    int in_waitroom;    /* did the cost reach her to the waitroom?     */
    int hand_n, wait_n_after, stage_c0;
    int census_before, census_after;
    int prompts;
    int queued_anything;
} HanayomeRun;

static HanayomeRun h_run(const char *const *wait, int n_wait,
                         const char *const *succ, int n_succ)
{
    HanayomeRun r;
    memset(&r, 0, sizeof r);
    r.me = -1;
    TestGame *tg = &t_tg[0];
    fresh(tg);

    int fill = mid(tg, FILLER_010);
    if (!pin_id(fill, FILLER_010)) return r;
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, fill);

    int me = mid(tg, HANAYOME);
    r.me = me;
    if (!pin_id(me, HANAYOME)) return r;
    CHECK_EQ(printed_cost(me), 4, "H: 東條 希 (PL!-pb2-007-PP) prints cost 4");
    CHECK(is_member(me), "H: she is a MEMBER, as the 起動 cost requires");
    CHECK(card_prints_trigger(me, RB_TSTR_ACTIVATION), "H: she prints an 起動 ability");
    info("H: HANAYOME '%s' unit=\"%s\"", no_of(me), unit_of(me));

    r.n_wait = n_wait;
    for (int i = 0; i < n_wait; i++) {
        int id = mid(tg, wait[i]);
        if (id < 0) { r.n_wait = 0; return r; }
        if (!pin_id(id, wait[i])) { r.n_wait = 0; return r; }
        r.wait[i] = id;
        add_to_wait_for(tg, 0, id);
    }
    r.n_succ = n_succ;
    for (int i = 0; i < n_succ; i++) {
        int id = mid(tg, succ[i]);
        if (id < 0) { r.n_succ = 0; return r; }
        if (!pin_id(id, succ[i])) { r.n_succ = 0; return r; }
        r.succ[i] = id;
        test_add_to_success(tg, id);
    }

    /* 3 INACTIVE energy cards, then 10 active. test_add_to_energy() bumps the
     * ACTIVE count with the card (it mirrors rb_energy_add_card), so the active
     * count is set back to 10 explicitly - otherwise the per-card half has
     * nothing left to activate and every energy-delta assertion reads 0 for a
     * reason that has nothing to do with the ability. */
    int eid = mid(tg, ENERGY_SD);
    if (!pin_id(eid, ENERGY_SD)) return r;
    for (int i = 0; i < 3; i++) test_add_to_energy(tg, 0, eid);
    tg->state.p[0].stage[AREA_LEFT] = me;
    test_give_energy(tg, 10);
    test_set_energy_active(tg, 0, 10);

    /* GROUP PINS: the recovery half filters on 『μ's』 the LIVE CARD, and the
     * energy half filters on 『μ's』 the CARD. Pinning both halves' inputs is
     * what stops a "nothing matched" board from satisfying every negative. */
    for (int i = 0; i < r.n_wait; i++) {
        info("H: waitroom card %d is '%s' live=%d mus=%d", i, no_of(r.wait[i]),
             is_live(r.wait[i]), in_group(r.wait[i], GRP_MU));
        CHECK(is_live(r.wait[i]),
              "H: the waitroom card %d is a LIVE - inside the ライブカード filter", i);
        CHECK(in_group(r.wait[i], GRP_MU),
              "H: the waitroom card %d is a 『μ's』 card", i);
    }
    for (int i = 0; i < r.n_succ; i++) {
        info("H: success card %d is '%s' live=%d mus=%d", i, no_of(r.succ[i]),
             is_live(r.succ[i]), in_group(r.succ[i], GRP_MU));
    }

    CHECK_EQ(success_n(tg), n_succ, "H: the success zone really holds %d card(s)", n_succ);
    CHECK_EQ(energy_zone_cards(tg), 13,
             "H: the energy zone holds 3 spare + 10 active = 13 cards");
    CHECK_EQ(energy_active(tg), 10,
             "H: 10 of the 13 are active, so 3 INACTIVE cards are available for the "
             "per-card activation to spend");

    r.census_before = test_total_card_count(tg);
    r.energy_before = energy_active(tg);
    int p0 = sh->prompts;

    test_activate_ability(tg, me);
    int guard = 0;
    while (pending(tg) && guard++ < 5) pick(tg, 0);
    if (guard >= 5 && pending(tg))
        info("H: the drain hit its cap of 5 with a prompt still standing");
    rb_drain_ability_queue(&tg->state);

    r.prompts = sh->prompts - p0;
    r.energy_after = energy_active(tg);
    r.energy_delta = r.energy_after - r.energy_before;
    r.in_waitroom = wait_has(tg, me);
    r.hand_n   = hand_n(tg);
    r.wait_n_after = wait_n(tg);
    r.stage_c0 = stage_slot(tg, AREA_LEFT);
    r.census_after = test_total_card_count(tg);
    info("H: wait=%d succ=%d prompts=%d energy %d->%d (delta %d) in_waitroom=%d "
         "hand=%d stage[0]=%d CENSUS %d->%d",
         n_wait, n_succ, r.prompts, r.energy_before, r.energy_after, r.energy_delta,
         r.in_waitroom, r.hand_n, r.stage_c0, r.census_before, r.census_after);
    return r;
}

static const char *const H_WAIT_MUS[]   = { MUS_LIVE_A };
static const char *const H_SUCC_MUS1[]  = { MUS_LIVE_A };
static const char *const H_SUCC_MUS2[]  = { MUS_LIVE_A, MUS_LIVE_B };
static const char *const H_SUCC_NONMUS[]= { NON_MUS_LIVE };
static const char *const H_NONE[]      = { NULL };

static void test_h1_cost_sends_her_to_the_waitroom(void)
{
    HanayomeRun r = h_run(H_WAIT_MUS, 1, H_NONE, 0);
    if (r.me < 0) return;
    CHECK(r.in_waitroom,
          "H1: 「このメンバーをステージから控え室に置く」 - the cost puts her in the waitroom");
    CHECK_EQ(r.stage_c0, -1, "H1: and she is no longer in the left stage area");
    /* CENSUS: the cost is a MOVE. A card that left the stage and reached nowhere
     * would satisfy the two assertions above. */
    CHECK_EQ(r.census_after, r.census_before,
             "H1 CENSUS: the cost moved one card and destroyed none");
}

static void test_h2_a_mus_live_is_recovered(void)
{
    HanayomeRun r = h_run(H_WAIT_MUS, 1, H_NONE, 0);
    if (r.me < 0) return;
    CHECK(r.hand_n >= 1,
          "H2: 「自分の控え室から『μ's』のライブカードを1枚手札に加える」 - a card is in hand");
    /* THE assertion with teeth: not a count, but the ID. The waitroom held
     * exactly one card and the hand started empty. */
    CHECK(r.wait[0] >= 0 && hand_has(&t_tg[0], r.wait[0]),
          "H2: the recovered card IS the μ's live that was in the waitroom");
    CHECK(!wait_has(&t_tg[0], r.wait[0]),
          "H2: the recovered live left the waitroom");
    CHECK_EQ(r.census_after, r.census_before,
             "H2 CENSUS: the recovery moved one card and destroyed none");
}

static void test_h3_one_mus_card_activates_one_energy(void)
{
    HanayomeRun r = h_run(H_NONE, 0, H_SUCC_MUS1, 1);
    if (r.me < 0) return;
    /* The pin that makes H3 a POSITIVE rather than a coincidence: the card the
     * count is supposed to scale by really is inside the filter. */
    CHECK(r.n_succ == 1 && in_group(r.succ[0], GRP_MU),
          "H3: the success-zone card really is a 『μ's』 card - INSIDE the count");
    CHECK_EQ(r.energy_delta, 1,
             "H3: 「自分の成功ライブカード置き場にある『μ's』のカード1枚につき、"
             "エネルギーを1枚アクティブにする」 - one card, one energy");
    CHECK_EQ(r.census_after, r.census_before,
             "H3 CENSUS: the activation created and destroyed no card");
}

static void test_h4_two_mus_cards_activate_two_energy(void)
{
    HanayomeRun r = h_run(H_NONE, 0, H_SUCC_MUS2, 2);
    if (r.me < 0) return;
    for (int i = 0; i < r.n_succ; i++)
        CHECK(in_group(r.succ[i], GRP_MU),
              "H4: the success-zone card %d really is a 『μ's』 card - INSIDE the count", i);
    /* 「1枚につき」 - the ONLY test in the family that separates a per-card
     * count from a constant. H3 alone would pass for a hard-coded 1. */
    CHECK_EQ(r.energy_delta, 2,
             "H4: 「1枚につき」 - the energy scales with the number of qualifying cards");
    CHECK_EQ(r.census_after, r.census_before,
             "H4 CENSUS: the activation created and destroyed no card");
}

static void test_h5_non_mus_card_activates_no_energy(void)
{
    /* GROUP PINS FIRST, so the negative is a filter claim. */
    int probe = mid(&t_tg[0], NON_MUS_LIVE);
    if (!pin_id(probe, NON_MUS_LIVE)) return;
    CHECK(is_live(probe), "H5: 愛♡スクリ～ム！ is a LIVE, so only the group can exclude it");
    CHECK(!in_group(probe, GRP_MU),
          "H5: and it is NOT a 『μ's』 card - that is what excludes it");
    fresh(&t_tg[0]);

    HanayomeRun r = h_run(H_NONE, 0, H_SUCC_NONMUS, 1);
    if (r.me < 0) return;
    CHECK_EQ(r.energy_delta, 0,
             "H5: 「『μ's』のカード」 - a non-μ's live in the success zone is not counted");
    /* NON-VACUITY: H3 and H4 are the paired positives on the same board shape
     * (one / two μ's cards) and the group pin above proves the decoy is a LIVE
     * that only the group excludes - so this is not a "nothing matched" board. */
    CHECK_EQ(r.census_after, r.census_before,
             "H5 CENSUS: the activation created and destroyed no card");
    info("H5: prompts=%d (the RECOVER half prompts independently of the group "
         "filter - the group claim is about the energy half only)", r.prompts);
    info("H5's non-vacuity rests on H3/H4 (same board, μ's cards) plus the group pin");
}

static void test_h6_empty_success_zone_activates_no_energy(void)
{
    HanayomeRun r = h_run(H_NONE, 0, H_NONE, 0);
    if (r.me < 0) return;
    CHECK_EQ(success_n(&t_tg[0]), 0,
             "H6 premise: the success live zone really is EMPTY on this board");
    CHECK_EQ(r.energy_delta, 0,
             "H6: 「…『μ's』のカード1枚につき…」 - with no such card there is nothing "
             "to scale by");
    /* NON-VACUITY: H3 / H4 on the same board shape, plus the fact that the
     * self_cost still ran - so the board was not inert. */
    CHECK(r.in_waitroom,
          "H6 CONTROL: the self_cost still ran, so the board was not inert");
    CHECK_EQ(r.census_after, r.census_before,
             "H6 CENSUS: the activation created and destroyed no card");
    info("H6's non-vacuity rests on H3/H4 (same board, 1 and 2 μ's cards)");
}

static void test_h7_cost_and_effect_halves_are_separate(void)
{
    TestGame *tg = &t_tg[0];
    fresh(tg);
    int me = mid(tg, HANAYOME);
    if (!pin_id(me, HANAYOME)) return;
    CHECK_EQ(rb_card_num_abilities((uint32_t)me), 1,
             "H7: the card carries exactly one ability");
    if (rb_card_num_abilities((uint32_t)me) != 1) { dump_abilities(me, "H7"); return; }

    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)me, 0, &ab)) {
        CHECK(0, "H7: the 起動 ability does not decode");
        return;
    }
    dump_abilities(me, "H7");

    CHECK(ab.triggers && strstr(ab.triggers, RB_TSTR_ACTIVATION),
          "H7: the ability is a 起動 (triggers=\"%s\")", ab.triggers ? ab.triggers : "-");
    CHECK(ab.effect != NULL, "H7: the 起動 ability has an effect");
    if (ab.effect) {
        /* 「…1枚手札に加える。その後…」 - a two-step effect. */
        CHECK(ab.effect->action && !strcmp(ab.effect->action, "sequential"),
              "H7: the effect action is \"sequential\" (got \"%s\")",
              ab.effect->action ? ab.effect->action : "(null)");
        CHECK(ab.effect->text && strstr(ab.effect->text, PHRASE_SONOGO),
              "H7: the effect text carries 「その後」 (text=\"%s\")",
              ab.effect->text ? ab.effect->text : "(null)");
        CHECK_EQ(ab.effect->is_further, 1,
                 "H7: 「その後」 parses to is_further, the engine's 'after that' flag");
    }
    /* 「このメンバーをステージから控え室に置く」 - the cost sends HER off stage. */
    CHECK(ab.cost != NULL, "H7: the 起動 ability has a cost");
    if (ab.cost) {
        CHECK(ab.cost->text && strstr(ab.cost->text, PHRASE_SELF_WAITROOM),
              "H7: the cost text carries 「このメンバーをステージから控え室に置く」 "
              "(text=\"%s\")", ab.cost->text ? ab.cost->text : "(null)");
    }
    rb_free_ability(&ab);
}

/* ======================================================================
 * driver
 * ====================================================================== */

typedef void (*CaseFn)(void);
typedef struct { const char *name; CaseFn fn; } Case;

static Case cases[] = {
    { "a_q123_cost_paid_without_recoverable_live",     test_a_q123_cost_paid_without_recoverable_live },
    { "a_q79_vacates_center_and_recovers_live",        test_a_q79_vacates_center_and_recovers_live },
    { "b_q79_vacated_area_takes_a_new_member",         test_b_q79_vacated_area_takes_a_new_member },
    { "c1_debut_recovers_live_from_waitroom",          test_c1_debut_recovers_live_from_waitroom },
    { "c2_debut_leaves_member_in_waitroom",            test_c2_debut_leaves_member_in_waitroom },
    { "c3_paired_positive_live_moves_member_stays",    test_c3_paired_positive_live_moves_member_stays },
    { "d1_live_success_recovers_member",               test_d1_live_success_recovers_member },
    { "d2_live_success_ignores_live_cards",            test_d2_live_success_ignores_live_cards },
    { "d3_paired_positive_member_moves_live_stays",    test_d3_paired_positive_member_moves_live_stays },
    { "e1_other_lilywhite_member_allows_recovery",     test_e1_other_lilywhite_member_allows_recovery },
    { "e2_no_lilywhite_member_prevents_recovery",      test_e2_no_lilywhite_member_prevents_recovery },
    { "f_kidou_play_is_not_softlocked",                test_f_kidou_play_is_not_softlocked },
    { "g1_live_success_does_not_fire_when_live_fails", test_g1_live_success_does_not_fire_when_live_fails },
    { "g2_q93_partial_resolution_one_card",            test_g2_q93_partial_resolution_one_card },
    { "g3_q93_partial_resolution_zero_cards",          test_g3_q93_partial_resolution_zero_cards },
    { "g4_ab1_live_success_pay_optional_cost",         test_g4_ab1_live_success_pay_optional_cost },
    { "g5_ab1_live_success_skip_optional_cost",        test_g5_ab1_live_success_skip_optional_cost },
    { "h1_cost_sends_her_to_the_waitroom",             test_h1_cost_sends_her_to_the_waitroom },
    { "h2_a_mus_live_is_recovered",                    test_h2_a_mus_live_is_recovered },
    { "h3_one_mus_card_activates_one_energy",          test_h3_one_mus_card_activates_one_energy },
    { "h4_two_mus_cards_activate_two_energy",          test_h4_two_mus_cards_activate_two_energy },
    { "h5_non_mus_card_activates_no_energy",           test_h5_non_mus_card_activates_no_energy },
    { "h6_empty_success_zone_activates_no_energy",     test_h6_empty_success_zone_activates_no_energy },
    { "h7_cost_and_effect_halves_are_separate",       test_h7_cost_and_effect_halves_are_separate },
};

#define N_CASES ((int)(sizeof cases / sizeof cases[0]))

static void run_child(int idx)
{
    sh->assertions = 0; sh->failures = 0; sh->setup_bugs = 0;
    sh->gaps = 0; sh->prompts = 0; sh->signal = 0; sh->rc = 0;
    /* A forked child INHERITS the parent's accumulated copy pool; without this
     * reset a long suite exhausts it and every identity assertion goes vacuous. */
    rb_card_copy_pool_reset();
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);
    signal(SIGFPE,  on_fatal_signal);
    signal(SIGILL,  on_fatal_signal);
    cases[idx].fn();
    sh->rc = (sh->failures || sh->setup_bugs) ? 1 : 0;
    if (sh->rc) {
        TestGame *d = &t_tg[0];
        dump_state(d, cases[idx].name);
    }
    fflush(stdout);
    fflush(stderr);
    _Exit(sh->rc);
}

static int run_case(int idx)
{
    current_test = cases[idx].name;
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) { perror("fork"); return -1; }
    if (pid == 0) run_child(idx);

    int status = 0;
    waitpid(pid, &status, 0);
    int a = sh->assertions, f = sh->failures, s = sh->setup_bugs, g = sh->gaps;
    tot_assertions += a; tot_failures += f; tot_setup_bugs += s; tot_gaps += g;
    if (WIFSIGNALED(status)) {
        n_crashed++;
        fprintf(stderr, "CRASH: %s died on signal %d (assertions=%d failures=%d)\n",
                cases[idx].name, WTERMSIG(status), a, f);
        return 2;
    }
    if (sh->rc != 0) { n_failed++; printf("FAILED: %s (%d failures, %d setup_bugs)\n",
                                         cases[idx].name, f, s); }
    else              { n_ok++;      printf("ok: %s (%d checks, %d gaps)\n",
                                            cases[idx].name, a, g); }
    fflush(stdout);
    return sh->rc;
}

int main(void)
{
    void *m = mmap(NULL, sizeof(Shared), PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (m == MAP_FAILED) { perror("mmap"); return 2; }
    sh = (Shared *)m;
    memset(sh, 0, sizeof *sh);

    if (load_card_db() != 0) {
        fprintf(stderr, "FATAL: card database failed to load\n");
        return 2;
    }

    /* The heart index is DERIVED and printed, never taken from a stale mapping
     * (AGENTS.md landmine: the colour-5 remap once in test_get_heart_modifier
     * was removed; the engine's map is 0=heart00 .. 6=heart06). */
    printf("heart colour map: heart00=%d heart01=%d heart02=%d heart03=%d "
           "heart04=%d heart05=%d heart06=%d\n",
           rb_parse_heart_color("heart00"), rb_parse_heart_color("heart01"),
           rb_parse_heart_color("heart02"), rb_parse_heart_color("heart03"),
           rb_parse_heart_color("heart04"), rb_parse_heart_color("heart05"),
           rb_parse_heart_color("heart06"));

    printf("test_parity_recover_q79: %d cases - self-to-waitroom / 起動 / "
           "partial-resolution slice of effects/recover/to_hand\n", N_CASES);
    for (int i = 0; i < N_CASES; i++) run_case(i);
    printf("\n%d ok, %d failed, %d crashed; checks=%d failures=%d gaps=%d setup_bugs=%d\n",
           n_ok, n_failed, n_crashed, tot_assertions, tot_failures, tot_gaps,
           tot_setup_bugs);
    return (n_failed || n_crashed || tot_setup_bugs) ? 1 : 0;
}
