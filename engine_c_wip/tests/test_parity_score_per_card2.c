/* test_parity_score_per_card2.c
 *
 * CLAIM (agent tr_score2, 2026-09-30). The only files this agent owns and edits are:
 *     engine_c_wip/tests/test_parity_score_per_card2.c
 *     engine_c_wip/tests/test_parity_live_total2.c
 * No engine source file under src/, no other tests/test_*.c, no Makefile.
 *
 * C port of the THREE clusters of
 *   engine/tests/test_modules/effects/score/per_card/
 * that the two existing suites' curated headers leave unclaimed:
 *
 *   aurora_flower_set_card_identity_group_match_test.rs ............ 5 tests
 *   energy_state_condition_active_energy_gate_test.rs ............. 7 tests
 *   live_success_remaining_yell_live_score_q253_test.rs ........... 4 tests
 *                                                        TOTAL ..... 16 tests
 *
 * WHERE THE GAP LIST CAME FROM (read, not re-surveyed)
 *   tests/test_parity_per_card.c:28-51 lists, under "NOT COVERED, by name and
 *   reason", the score/per_card files nobody had ported. Of those, this file
 *   takes the three that file's own list still names as gaps and that no other
 *   suite claims:
 *     - aurora_flower_set_card_identity_group_match_test.rs  (5)
 *     - energy_state_condition_active_energy_gate_test.rs   (7)
 *     - live_success_remaining_yell_live_score_q253_test.rs  (4)
 *   tests/test_parity_per_card3.c:36-64 REWRITES that list. It takes
 *   tokimeki (5), shizuku (4), strawberry (5) and excess_heart (2) itself, and
 *   it downgrades to "covered elsewhere" solitude_rain (2, via
 *   test_parity_card_score.c:742), vitamin_summer (3, via
 *   test_parity_characterization_extra.c:1803), live_success_surplus (3 of 8,
 *   at effect level in test_parity_score_effects.c:655) and takaramono (2, via
 *   test_parity_cost_and_effect.c:616). It leaves three of the per_card3
 *   "SKIPPED -- budget" entries open and untouched -- exactly the three taken
 *   here -- and states its reasons for skipping them were budget, not
 *   impossibility. So per_card3's list contributed the BOUNDARY (what is
 *   already covered, and by which body) and this file took only what is left.
 *   NOTHING from the per_card3 "NOT PORTABLE" list is retried here:
 *   neutral_live_success_test.rs (6) and replacement_choose_order_on_live_event
 *   _test.rs (2) stay skipped for the reasons per_card3 already established
 *   (no C shim for generated_actions()/select_generated()/
 *   assert_pending_choice_type(); no per-entry causer on recently_moved[]).
 *
 * HARNESS LANDMINES HONOURED (see each site for the specific one)
 *   - load_card_db() falls back to rb_load("../cards/build"); rb_load("src")
 *     alone fails under the isolated out-of-tree build.
 *   - sizeof(TestGame) is ~781 KB, so every TestGame here is `static`.
 *   - mid() == test_new_id(), never test_id(): the sumire/galaxy/aurora
 *     fixtures each need a real second instance, and a single static buffer
 *     used two or three times in one printf prints the LAST value every time,
 *     so card numbers go through a ROTATING ring (no_of).
 *   - test_id_exact(), not test_id(), wherever the print itself is under test:
 *     rb_find_card_by_no falls back to a SUBSTRING search and will happily
 *     resolve a mistyped print to some other real card.
 *   - Heart reads go through rb_mods_get_heart with an index derived from
 *     rb_parse_heart_color("heart02"); main() PRINTS the mapping. The
 *     RB_HEART_* enum is NOT the engine's colour map (5 is heart05, 6 is
 *     heart06) and test_get_heart_modifier no longer remaps.
 *   - test_advance_to_phase_strict is the faithful port of Rust
 *     advance_to_phase (it stops on a standing prompt instead of eating it);
 *     advance_to() below adds only the drain Rust's own loop performs BETWEEN
 *     steps. test_advance_to_phase is never used.
 *   - ライブ成功時 is driven through rb_trigger_live_success, never the phase
 *     machine; rb_should_trigger_live_success is asserted at the assertion
 *     site so a closed window can never be mistaken for a zero.
 *   - Every drain is capped (guard). A peer found an ability that re-prompts
 *     unboundedly.
 *   - Card.ability is NOT populated by rb_decode_card_by_index, so ability
 *     inspection goes through rb_card_num_abilities + rb_decode_card_ability.
 *   - The waitroom zone string is "discard".
 *   - All non-ASCII literals are \xNN escapes: a hand-typed 蓮ノ空 (U+84EE)
 *     or 虹ヶ咲 (U+8679, NOT U+8671) silently mismatches the engine's bytes,
 *     and the Write tool can emit CP932 instead of UTF-8.
 *   - rb_card_group_name returns a SERIES-level group (every ラブライブ！ card
 *     reports mu's); rb_card_unit_name is the unit-level check. main() prints
 *     both for the fixtures this file names.
 *
 * VACUITY CONTROL. A per_card cluster is a grid of one card x one behaviour, so
 * a whole column can read green while the engine does literally nothing. Every
 * negative below therefore carries either
 *   (a) a PAIRED POSITIVE on the same board shape -- the same live, the same
 *       board, one term changed; or
 *   (b) a prompt-count / zone-count control taken AT THE ASSERTION SITE, or
 *   (c) a test_total_card_count CENSUS baselined after every fixture card is
 *       placed and immediately before the action under test.
 * Every case carries (c). The pairs are:
 *     aoku: 3 suzu lives in the waitroom -> +1  vs  2 -> 0  vs 1 -> 0   PAIRS
 *     sumire: energy_active 1 -> +2 heart02   vs  energy_active 0 -> 0     PAIRS
 *     daisuki: energy_active 1 -> +1 score   vs  energy_active 0 -> 0     PAIRS
 *     daisuki: p1 active energy -> +1         vs  p2 active energy -> 0    PAIRS
 *     galaxy: a revealed LIVE -> fires        vs  a revealed non-live -> 0 PAIRS
 * The census is the control that a card-DESTROYING defect (two were found
 * today, both invisible to a zone count) would trip.
 *
 * NOT EXPRESSIBLE, reported rather than faked
 *   live_success_remaining_yell_live_score_q253_test.rs:86 asserts
 *   `performance_snapshots[0].lives[0].score - .base_score == 1`.
 *   RbLiveSnapshot (rabuka.h:1220-1240) has NO base_score field, so the delta
 *   cannot be written. The expressible half of that same test
 *   (score_mod == 0 after the live) IS asserted strictly, and the snapshot is
 *   asserted to EXIST and to record the live, so a future regression of the
 *   bonus cannot hide behind a missing snapshot.
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

/* Counters live in a MAP_SHARED|MAP_ANONYMOUS mapping so a child that dies on
 * a fatal signal half way through an assertion still reports everything it
 * reached. */
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

/* The message is variadic on purpose. Rust test messages here are long and are
 * written as several adjacent C string literals; a fixed two-parameter macro
 * would let the preprocessor split them into extra arguments, so the message is
 * routed through snprintf(__VA_ARGS__), where the compiler performs the
 * concatenation. Callers that want a formatted value pass the value too. */
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
/* Every non-ASCII byte form is an escape; the codepoint is in the comment. */
#define AURORA_FLOWER   "PL!HS-bp5-018-L"              /* AURORA FLOWER     */
#define AOKU_HARUKA     "PL!HS-bp2-022-L"              /* アオクハルカ      */
#define HOLIDAY         "PL!HS-bp1-021-L"              /* Holiday inf Holiday */
#define KAGAYAKU        "PL!HS-bp2-021-L"              /* 眩耀夜行          */
#define SUZU_LIVE       "PL!HS-bp1-021-L"              /* same print, alias */
#define NON_SUZU_MUS    "PL!-sd1-019-SD"               /* START:DASH!!, mu's */
#define NON_SUZU_NIJI   "PL!N-sd1-025-SD"              /* Colorful Dreams!, 虹ヶ咲 */
#define FILLER          "PL!-sd1-010-SD"               /* 高坂 穂乃果       */
#define FILLER2         "PL!-sd1-013-SD"               /* 園田海未          */
#define SUMIRE          "PL!SP-pb2-026-N"              /* 平安名すみれ      */
#define DAISUKI         "PL!SP-bp4-028-L"              /* DAISUKI FULL POWER */
#define ENERGY_CARD     "LL-E-001-SD"                  /* エネルギーカード  */
#define LINK_FUTURE     "PL!HS-bp2-020-L"              /* Link to the FUTURE */
#define HASU_A          "PL!HS-sd1-001-SD"             /* 日野下花帆        */
#define HASU_B          "PL!HS-sd1-002-SD"             /* 村野さやか        */
#define HASU_C          "PL!HS-sd1-003-SD"             /* 大沢瑠璃乃        */
#define HASU_ONE        "PL!HS-bp1-001-R"              /* 日野下花帆 (R)    */
#define GALAXY          "PL!S-bp6-023-L"               /* GALAXY HidE and SeeK */
#define YELL_LIVE_1     "PL!-sd1-020-SD"               /* きっと青春が聞こえる */
#define YELL_LIVE_2     "PL!-sd1-021-SD"               /* これからのSomeday  */
#define GK_L            "PL!S-bp2-017-N"               /* 小原鞠莉          */
#define GK_C            "PL!S-bp3-015-N"               /* 津島善子          */
#define GK_R_PR         "PL!S-PR-014-PR"               /* 桜内梨子          */
#define KANAN           "PL!S-pb1-003-R"               /* 松浦果南 (AZALEA) */

/* group strings, byte-exact */
#define G_SUZU   "\xE3\x82\xB9\xE3\x83\xAA\xE3\x83\xBC\xE3\x82\xBA\xE3\x83\x96\xE3\x83\xBC\xE3\x82\xB1" /* スリーズブeke -> スリーズブーケ */
#define G_DOLL   "DOLLCHESTRA"
#define G_MIRAK  "\xE3\x81\xBF\xE3\x82\x89\xE3\x81\x8F\xE3\x82\x89\xE3\x81\xB1\xE3\x83\xBC\xE3\x81\x8F\xEF\xBC\x81" /* みらくらぱーく！ */
#define G_HASU   "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA" /* 蓮ノ空 (U+84EE) */
#define G_MUS    "\xCE\xBC's"                             /* mu's (U+03BC) */
#define G_AQOURS "Aqours"
#define G_LIELLA "Liella!"
#define G_NIJI   "\xE8\x99\xB9\xE3\x82\xB1\xE3\x82\xB5\xE3\x82\xAF" /* 虹ヶ咲 (U+8679, NOT U+8671) */
#define TRIGGER_LIVE_START  "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82" /* ライブ開始時 */
#define TRIGGER_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82" /* ライブ成功時 */

/* -- helpers ------------------------------------------------------------- */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rust's `mid()` / `game.new_id`: a DISTINCT copy of the print, not the shared
 * template index. */
static int mid(TestGame *tg, const char *no) { return test_new_id(tg, no); }

/* ROTATING ring: a helper with one static buffer, called 2-3 times in a single
 * printf, prints the LAST card every time -- a false-red generator. */
static const char *no_of(int cid)
{
    static char buf[4][80];
    static int which;
    char *out = buf[which = (which + 1) & 3];
    Card c;
    out[0] = 0;
    if (cid < 0) { snprintf(out, sizeof buf[0], "<none>"); return out; }
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { snprintf(out, sizeof buf[0], "<nodecode>"); return out; }
    /* card_no_idx is a STRING-TABLE INDEX, not a value. */
    snprintf(out, sizeof buf[0], "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    return out;
}

/* Rust assert_card_identity. Byte-exact: rb_card_no_eq refuses an empty needle
 * but rb_find_card_by_no does not, so the id is resolved with test_id_exact. */
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

/* Non-ASCII wire bytes are re-printed as \xNN: this console transcodes UTF-8
 * output, so a Japanese decoded value comes back as mojibake. */
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
    /* card_no_idx / group_idx are STRING-TABLE INDICES, not values, and
     * group_idx is 0xFFFF (empty) on every record -- so the DERIVED accessors
     * are the only meaningful source. */
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

/* heart02 through the DERIVED index, never the RB_HEART_* enum. */
static int HEART02 = 2;
static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}

/* Mirrors Rust `while has_pending_choice { select_indices(&[0]) }`. */
static void drain_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        test_resume_choice(tg, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

/* Mirrors Rust drain_choices: skip what allows skip, answer 0 otherwise. */
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

/* Faithful port of Rust advance_to_phase (helpers/mod.rs:538-561): step one
 * phase at a time, hand back a prompt raised by the step into the target, and
 * drain only the prompts raised by steps that did NOT arrive. */
static int advance_to(TestGame *tg, int target)
{
    for (int i = 0; i < 16; i++) {
        if (test_advance_to_phase_strict(tg, target)) return 1;
        if (!test_has_pending_choice(tg)) return 0;
        drain_first(tg);
    }
    return 0;
}

static int advance_named(TestGame *tg, const char *name)
{
    int t = test_phase_from_name(name);
    if (t < 0) return 0;
    return advance_to(tg, t);
}

/* Fire every ライブ開始時 ability on `pl`'s live zone + stage. This is the C
 * twin of the Rust fire_trigger(game, cid, LiveStart, ..) helper: the port has
 * no ability-id-addressed entry point, and on these fixtures only the card
 * under test carries a live-start ability. */
static void fire_live_start(TestGame *tg, int pl, int cid)
{
    tg->state.activating_card = cid;
    rb_trigger_live_start(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

/* ライブ成功時, with the window forced open. rb_should_trigger_live_success
 * gates on BOTH live.n > 0 and live_success[pl]; the Rust helper bypasses that
 * gate by dispatching directly, and setting live_success[pl] is the same
 * bypass expressed through the C surface. */
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

/* Rust `main_deck.cards.insert(at, id)` -- the deck bag has no insert shim. */
static void deck_insert(TestGame *tg, int pl, int at, int cid)
{
    RbBag *b = &tg->state.p[pl].deck;
    int n = b->n;
    if (n >= RB_MAX_ZONE || at < 0 || at > n) return;
    for (int i = n; i > at; i--) b->cards[i] = b->cards[i - 1];
    b->cards[at] = cid;
    b->n = n + 1;
}

static int live_total_p1(TestGame *tg)
{
    return (int)tg->state.mods.p1_constant_total_score_bonus;
}

/* =========================================================================
 * 1. aurora_flower_set_card_identity_group_match_test.rs (5 tests)
 *
 * PL!HS-bp5-018-L AURORA FLOWER, 常時: すべての領域にあるこのカードは
 * 『スリーズブーケ』、『DOLLCHESTRA』、『みらくらぱーく！』として扱う。
 * PL!HS-bp2-022-L アオクハルカ, ライブ開始時: 自分の控え室に『スリーズブーケ』の
 * ライブカードが3枚以上ある場合、このカードのスコアを＋１する。
 * ========================================================================= */

/* AURORA on the stage must satisfy all three set_card_identity names. */
static void test_aurora_flower_treated_as_three_groups_in_stage(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int aurora = test_id_exact(&tg, AURORA_FLOWER);
    ident(aurora, AURORA_FLOWER, "aurora_flower_treated_as_three_groups_in_stage");
    dbg_card("AURORA", aurora);

    test_add_to_stage(&tg, 1, aurora);
    test_recalc(&tg);

    /* CONTROL: the card really is on the stage, so the matcher below is reading
     * a placed card and not an unplaced template index. */
    CHECK_EQ(test_stage_has(&tg, 1, aurora), 1,
             "AURORA FLOWER control: the card is on the centre stage slot");

    CHECK(rb_card_matches_group_str(aurora, G_SUZU),
          "AURORA FLOWER should be treated as スリーズブーケ via set_card_identity");
    CHECK(rb_card_matches_group_str(aurora, G_DOLL),
          "AURORA FLOWER should be treated as DOLLCHESTRA via set_card_identity");
    CHECK(rb_card_matches_group_str(aurora, G_MIRAK),
          "AURORA FLOWER should be treated as みらくらぱーく！ via set_card_identity");
}

/* The identities are SPECIFIC, not a match-everything flag. The positives of
 * the previous case are the paired control: a matcher that returned 1 for
 * every query would make these four negatives pass while proving nothing. */
static void test_aurora_flower_does_not_match_unrelated_groups(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int aurora = test_id_exact(&tg, AURORA_FLOWER);
    int mus    = test_id_exact(&tg, FILLER);   /* Printemps, series ラブライブ！ */
    ident(aurora, AURORA_FLOWER, "aurora_flower_does_not_match_unrelated_groups");
    ident(mus, FILLER, "aurora_flower_does_not_match_unrelated_groups: mu's control");
    dbg_card("AURORA", aurora);
    dbg_card("MUS-CTL", mus);

    test_add_to_stage(&tg, 1, aurora);
    test_recalc(&tg);

    /* PAIRED POSITIVE CONTROL, taken before the negatives: a plain mu's card
     * DOES answer to the mu's query. Without it, "AURORA does not match mu's"
     * would also be satisfied by a matcher that never matches anything. */
    CHECK(rb_card_matches_group_str(mus, G_MUS),
          "control: a ラブライブ！ member DOES match the mu's group query");
    CHECK(rb_card_matches_group_str(aurora, G_HASU),
          "AURORA FLOWER's static group 蓮ノ空 must still match");

    CHECK(!rb_card_matches_group_str(aurora, G_AQOURS),
          "AURORA FLOWER has no set_card_identity for Aqours and is not in Aqours series");
    CHECK(!rb_card_matches_group_str(aurora, G_MUS),
          "AURORA FLOWER has no set_card_identity for mu's");
    CHECK(!rb_card_matches_group_str(aurora, G_NIJI),
          "AURORA FLOWER has no set_card_identity for 虹ヶ咲");
    CHECK(!rb_card_matches_group_str(aurora, G_LIELLA),
          "AURORA FLOWER has no set_card_identity for Liella!");
}

/* アオクハルカ: three suzu lives in the waitroom -> +1. The prompt-count
 * control is taken at the ASSERTION SITE, and the CENSUS is baselined after
 * every fixture card is placed. */
static void test_aoku_haruka_live_start_scores_with_aurora_in_discard(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int aoku     = test_id_exact(&tg, AOKU_HARUKA);
    int aurora   = test_id_exact(&tg, AURORA_FLOWER);
    int holiday  = test_id_exact(&tg, HOLIDAY);
    int kagayaku = test_id_exact(&tg, KAGAYAKU);
    int filler   = mid(&tg, FILLER);
    ident(aoku, AOKU_HARUKA, "aoku positive");
    ident(aurora, AURORA_FLOWER, "aoku positive");
    ident(holiday, HOLIDAY, "aoku positive");
    ident(kagayaku, KAGAYAKU, "aoku positive");
    dbg_card("AOKU", aoku);

    /* the two static suzu lives must be DIFFERENT cards, one letter apart */
    CHECK(test_card_name(holiday) && test_card_name(kagayaku) &&
          strcmp(test_card_name(holiday), test_card_name(kagayaku)) != 0,
          "the two static スリーズブeke lives must differ by name");

    test_add_to_hand(&tg, aoku);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 40);
    test_add_to_discard(&tg, aurora);
    test_add_to_discard(&tg, holiday);
    test_add_to_discard(&tg, kagayaku);
    test_give_energy(&tg, 10);

    CHECK(advance_named(&tg, "LiveCardSetFirstAttacker"),
          "aoku positive: the turn reaches the live-card set phase");
    test_set_live_card(&tg, 0, aoku);
    drain_choices(&tg);

    /* CENSUS: baselined after every fixture card is placed and immediately
     * before the action under test (the ライブ開始時 window). */
    int census = test_total_card_count(&tg);
    int prompts_before = test_pending_choice_count(&tg);

    CHECK(advance_named(&tg, "FirstAttackerPerformance"),
          "aoku positive: the turn reaches the first-attacker performance phase");
    drain_choices(&tg);

    CHECK_EQ(test_total_card_count(&tg), census,
             "aoku positive: CENSUS -- the live-start window destroyed or duplicated no card");
    printf("     aoku positive: prompt count %d -> %d\n", prompts_before,
           test_pending_choice_count(&tg));
    /* ZONE CONTROL at the assertion site: the three waitroom cards are still
     * there, so a +1 cannot have come from a zone that emptied. */
    CHECK_EQ(test_zone_len(&tg, 0, "waitroom"), 3,
             "aoku positive control: the 控え室 still holds exactly the three placed cards");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1,
             "aoku positive control: the live card is in the live card zone");

    CHECK_EQ(score_of(&tg, aoku), 1,
             "アオクハルカ should gain +1 score: 2 static スリーズブeke live cards "
             "(holiday, kagayaku) + AURORA FLOWER (スリーズブeke via set_card_identity) "
             "= 3 cards in the 控え室");
}

/* The PAIRED NEGATIVE of the case above: same board, AURORA removed. Without
 * it, the positive above would also pass on a board where the effect is a
 * no-op. */
static void test_aoku_haruka_live_start_no_score_when_aurora_missing(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int aoku     = test_id_exact(&tg, AOKU_HARUKA);
    int holiday  = test_id_exact(&tg, HOLIDAY);
    int kagayaku = test_id_exact(&tg, KAGAYAKU);
    int filler   = mid(&tg, FILLER);
    ident(aoku, AOKU_HARUKA, "aoku negative (no AURORA)");
    ident(holiday, HOLIDAY, "aoku negative (no AURORA)");
    ident(kagayaku, KAGAYAKU, "aoku negative (no AURORA)");

    test_add_to_hand(&tg, aoku);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 40);
    test_add_to_discard(&tg, holiday);
    test_add_to_discard(&tg, kagayaku);
    test_give_energy(&tg, 10);

    CHECK(advance_named(&tg, "LiveCardSetFirstAttacker"),
          "aoku negative: the turn reaches the live-card set phase");
    test_set_live_card(&tg, 0, aoku);
    drain_choices(&tg);

    int census = test_total_card_count(&tg);
    CHECK(advance_named(&tg, "FirstAttackerPerformance"),
          "aoku negative: the turn reaches the first-attacker performance phase");
    drain_choices(&tg);

    CHECK_EQ(test_total_card_count(&tg), census,
             "aoku negative: CENSUS -- the live-start window destroyed or duplicated no card");
    CHECK_EQ(test_zone_len(&tg, 0, "waitroom"), 2,
             "aoku negative control: exactly two cards in the 控え室, and no AURORA");

    CHECK_EQ(score_of(&tg, aoku), 0,
             "Without AURORA FLOWER, only 2 static スリーズブeke live cards in "
             "the 控え室 -- threshold of 3 not met, no score modifier");
}

/* Breaks the group half of the condition while keeping the count. */
static void test_aoku_haruka_live_start_fails_with_only_non_suzu_live_cards(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int aoku    = test_id_exact(&tg, AOKU_HARUKA);
    int aurora  = test_id_exact(&tg, AURORA_FLOWER);
    int mus_live = test_id_exact(&tg, NON_SUZU_MUS);
    int niji    = test_id_exact(&tg, NON_SUZU_NIJI);
    int filler  = mid(&tg, FILLER);
    ident(aoku, AOKU_HARUKA, "aoku non-suzu");
    ident(aurora, AURORA_FLOWER, "aoku non-suzu");
    ident(mus_live, NON_SUZU_MUS, "aoku non-suzu");
    ident(niji, NON_SUZU_NIJI, "aoku non-suzu");
    dbg_card("NON-SUZU mu's", mus_live);
    dbg_card("NON-SUZU niji", niji);

    /* PAIRED CONTROL: the two substitutes really are outside the group, so
     * "count 3, group 0" is what the board is, not a lookup that matched. */
    CHECK(!rb_card_matches_group_str(mus_live, G_SUZU),
          "control: START:DASH!! is not a スリーズブeke live card");
    CHECK(!rb_card_matches_group_str(niji, G_SUZU),
          "control: Colorful Dreams! is not a スリーズブeke live card");
    CHECK(rb_card_is_live(mus_live) && rb_card_is_live(niji),
          "control: both substitutes really are LIVE cards, so the count stays 3");

    test_add_to_hand(&tg, aoku);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 40);
    test_add_to_discard(&tg, aurora);
    test_add_to_discard(&tg, mus_live);
    test_add_to_discard(&tg, niji);
    test_give_energy(&tg, 10);

    CHECK(advance_named(&tg, "LiveCardSetFirstAttacker"),
          "aoku non-suzu: the turn reaches the live-card set phase");
    test_set_live_card(&tg, 0, aoku);
    drain_choices(&tg);

    int census = test_total_card_count(&tg);
    CHECK(advance_named(&tg, "FirstAttackerPerformance"),
          "aoku non-suzu: the turn reaches the first-attacker performance phase");
    drain_choices(&tg);

    CHECK_EQ(test_total_card_count(&tg), census,
             "aoku non-suzu: CENSUS -- the live-start window destroyed or duplicated no card");
    CHECK_EQ(test_zone_len(&tg, 0, "waitroom"), 3,
             "aoku non-suzu control: three cards in the 控え室 -- the COUNT half holds");

    CHECK_EQ(score_of(&tg, aoku), 0,
             "Only AURORA FLOWER matches スリーズブeke (1 card), below threshold 3");
}

/* =========================================================================
 * 2. energy_state_condition_active_energy_gate_test.rs (7 tests)
 *
 * PL!SP-pb2-026-N 平安名すみれ, 常時: 自分のアクティブ状態のエネルギーがあるかぎり、
 * heart02 x2 を得る。
 * PL!SP-bp4-028-L DAISUKI FULL POWER, ライブ開始時: アクティブ状態の自分の
 * エネルギーがある場合、このカードのスコアを＋１する。
 *
 * The engine's energy zone is an unordered bag plus a SEPARATE energy_active
 * count (src/zones.c), so "tapped" == energy_active lowered, which is what
 * test_spend_energy does. That is the whole discriminator this file is about,
 * and every negative is paired with a positive on the same board.
 * ========================================================================= */

static void test_sumire_no_active_energy_no_hearts(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int sumire = test_id_exact(&tg, SUMIRE);
    ident(sumire, SUMIRE, "sumire: no active energy");
    dbg_ability("SUMIRE", sumire);

    test_add_to_stage(&tg, 1, sumire);
    /* CONTROL at the assertion site: the board really has ZERO energy cards and
     * zero active, so a 0-heart read is the effect being off and not an empty
     * stage. */
    CHECK_EQ(test_zone_len(&tg, 0, "energy"), 0,
             "sumire control: the energy zone is empty");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "sumire control: no active energy");

    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "sumire: CENSUS -- recalculate_constants destroyed or duplicated no card");

    CHECK_EQ(heart_of(&tg, sumire, HEART02), 0,
             "no active energy => no hearts");
}

static void test_sumire_with_one_active_energy_gains_two_hearts(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int sumire = test_id_exact(&tg, SUMIRE);
    ident(sumire, SUMIRE, "sumire: one active energy");

    test_add_to_stage(&tg, 1, sumire);
    test_give_energy(&tg, 1);

    /* CONTROL: exactly one energy card, active. The paired negative below is
     * this same board with energy_active lowered. */
    CHECK_EQ(test_zone_len(&tg, 0, "energy"), 1,
             "sumire positive control: one energy card in the zone");
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "sumire positive control: one ACTIVE energy");

    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "sumire: CENSUS -- recalculate_constants destroyed or duplicated no card");

    CHECK_EQ(heart_of(&tg, sumire, HEART02), 2,
             "1 active => +2 heart02 (index derived from rb_parse_heart_color, "
             "NOT the RB_HEART_YELLOW enum constant)");
}

static void test_sumire_inactive_energy_does_not_count(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int sumire = test_id_exact(&tg, SUMIRE);
    ident(sumire, SUMIRE, "sumire: inactive energy");

    test_add_to_stage(&tg, 1, sumire);
    test_give_energy(&tg, 1);
    test_spend_energy(&tg, 1);          /* tap it */

    /* CONTROL: the card is STILL in the zone -- it is the ACTIVE COUNT that is
     * zero. Without this, "no hearts" could be explained by the energy never
     * having been there. */
    CHECK_EQ(test_zone_len(&tg, 0, "energy"), 1,
             "sumire negative control: the energy card is still in the zone");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "sumire negative control: the energy is WAIT, not active");

    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "sumire: CENSUS -- recalculate_constants destroyed or duplicated no card");

    CHECK_EQ(heart_of(&tg, sumire, HEART02), 0,
             "wait energy should not satisfy the active check");
}

static void test_sumire_toggling_active_recalculates(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int sumire = test_id_exact(&tg, SUMIRE);
    ident(sumire, SUMIRE, "sumire: toggling active");

    test_add_to_stage(&tg, 1, sumire);
    test_give_energy(&tg, 2);

    int census = test_total_card_count(&tg);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, sumire, HEART02), 2,
             "2 active energy => +2 heart02 (the effect is a flat 2, not 2 per energy)");

    test_spend_energy(&tg, 2);
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "sumire toggle control: both energy cards are now WAIT");
    CHECK_EQ(heart_of(&tg, sumire, HEART02), 0,
             "tapping both energy cards removes the +2 heart02");

    test_set_energy_active(&tg, 0, 1);
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "sumire toggle control: one energy re-activated");
    CHECK_EQ(heart_of(&tg, sumire, HEART02), 2,
             "re-activating one energy restores the +2 heart02");

    CHECK_EQ(test_total_card_count(&tg), census,
             "sumire: CENSUS -- the whole toggle sequence destroyed or duplicated no card");
}

static void test_daisuki_score_plus_one_when_active_energy_exists(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, DAISUKI);
    ident(live, DAISUKI, "daisuki: active energy");
    dbg_ability("DAISUKI", live);

    test_add_to_live(&tg, live);
    test_give_energy(&tg, 1);

    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "daisuki positive control: one ACTIVE energy before the trigger");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1,
             "daisuki positive control: the live card is in the live card zone");

    int census = test_total_card_count(&tg);
    fire_live_start(&tg, 0, live);
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "daisuki: CENSUS -- the live-start trigger destroyed or duplicated no card");

    CHECK_EQ(test_pending_choice_count(&tg), 0,
             "daisuki positive: アクティブ状態の自分のエネルギーがある場合 is a pure "
             "condition -- it must raise no prompt at all");

    CHECK_EQ(score_of(&tg, live), 1,
             "active energy should give score+1");
}

static void test_daisuki_score_not_added_when_no_active_energy(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, DAISUKI);
    ident(live, DAISUKI, "daisuki: no active energy");

    test_add_to_live(&tg, live);
    test_give_energy(&tg, 1);
    test_spend_energy(&tg, 1);          /* make it wait */

    /* CONTROL: the energy card is still there; only the ACTIVE COUNT is zero.
     * This is the exact opposite of the paired positive one case above. */
    CHECK_EQ(test_zone_len(&tg, 0, "energy"), 1,
             "daisuki negative control: the energy card is still in the zone");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "daisuki negative control: the energy is WAIT, not active");

    int census = test_total_card_count(&tg);
    fire_live_start(&tg, 0, live);
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "daisuki: CENSUS -- the live-start trigger destroyed or duplicated no card");

    CHECK_EQ(test_pending_choice_count(&tg), 0,
             "daisuki negative: the condition fails silently -- it must raise no prompt");

    CHECK_EQ(score_of(&tg, live), 0,
             "wait energy should not satisfy the active check");
}

static void test_daisuki_opponent_active_does_not_satisfy_self(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int live = test_id_exact(&tg, DAISUKI);
    int eng  = test_id_exact(&tg, ENERGY_CARD);
    ident(live, DAISUKI, "daisuki: opponent active");
    ident(eng, ENERGY_CARD, "daisuki: opponent active");

    test_add_to_live(&tg, live);
    /* opponent: an energy card in the zone, raised as ACTIVE */
    test_add_to_energy(&tg, 1, eng);
    test_set_energy_active(&tg, 1, 1);

    /* CONTROL at the assertion site: the opponent really does have an active
     * energy and P1 really does not -- otherwise the zero is vacuous. */
    CHECK_EQ(tg.state.p[1].energy_active, 1,
             "daisuki opponent control: P2 has one ACTIVE energy");
    CHECK_EQ(test_zone_len(&tg, 1, "energy"), 1,
             "daisuki opponent control: the opponent's energy card is in the zone");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "daisuki opponent control: P1 has NO active energy");

    int census = test_total_card_count(&tg);
    fire_live_start(&tg, 0, live);
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "daisuki: CENSUS -- the live-start trigger destroyed or duplicated no card");

    CHECK_EQ(test_pending_choice_count(&tg), 0,
             "daisuki opponent: the condition fails silently -- no prompt");

    CHECK_EQ(score_of(&tg, live), 0,
             "opponent active should not satisfy 自分のエネルギー");
}

/* =========================================================================
 * 3. live_success_remaining_yell_live_score_q253_test.rs (4 tests)
 *
 * PL!S-bp6-023-L GALAXY HidE and SeeK, ライブ成功時: エールにより公開された自分の
 * カードの中にライブカードがある場合、このカードのスコアを＋１する。
 *
 * The Rust drives the WHOLE live (advance_to_live_start, then three
 * pass+drain rounds: first performance, second performance, victory). That is
 * reproduced here step for step. The Rust pair of the first three cases is
 * internally inconsistent about WHEN the modifier clears (case 1 expects 0
 * after the same walk that case 3 expects 1 from) -- both are ported verbatim
 * and whatever C does is reported, not reconciled.
 * ========================================================================= */

static void setup_q253_stage(TestGame *tg, int left, int center, int right)
{
    test_add_to_stage(tg, 0, left);
    test_add_to_stage(tg, 1, center);
    test_add_to_stage(tg, 2, right);
}

/* Rust `answer_prompts`: decline a prompt that can be skipped, otherwise take
 * option 0. That is drain_choices() -- kept as a separate name so the two
 * drains in this file are not silently interchanged. */
static void answer_prompts(TestGame *tg) { drain_choices(tg); }

/* The one-off loop of Rust case 2: take a card for Kanan's NON-optional move,
 * but skip everything the yell-triggered abilities raise. Breaks after the
 * first mandatory pick, exactly as the Rust does. */
static void take_first_mandatory(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        if (ch && !ch->allow_skip) { test_resume_choice(tg, 0); return; }
        test_resume_choice(tg, -1);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

static void test_live_success_revealed_live_adds_one_to_final_score_q253(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int galaxy    = test_id_exact(&tg, GALAXY);
    int yell_live = mid(&tg, YELL_LIVE_1);
    int filler    = mid(&tg, FILLER);
    ident(galaxy, GALAXY, "q253 case 1");
    ident(yell_live, YELL_LIVE_1, "q253 case 1");
    dbg_ability("GALAXY", galaxy);
    dbg_card("YELL LIVE", yell_live);

    setup_q253_stage(&tg,
        test_id_exact(&tg, GK_L), test_id_exact(&tg, GK_C), test_id_exact(&tg, GK_R_PR));
    /* CONTROL: no AZALEA member with a yell-recovery ability is on the stage,
     * so nothing can take the revealed live away before GALAXY reads it. */
    CHECK_EQ(test_stage_has(&tg, 0, test_id_exact(&tg, KANAN)), 0,
             "q253 case 1 control: 松浦果南 is NOT on the stage in this case");

    for (int i = 0; i < 20; i++) test_add_to_deck(&tg, filler);
    deck_insert(&tg, 0, 1, yell_live);

    CHECK(advance_named(&tg, "LiveCardSetFirstAttacker"),
          "q253 case 1: the turn reaches the live-card set phase");
    test_add_to_hand(&tg, galaxy);
    test_set_live_card(&tg, 0, galaxy);
    CHECK(advance_named(&tg, "FirstAttackerPerformance"),
          "q253 case 1: the turn reaches the first-attacker performance phase");
    drain_choices(&tg);

    int census = test_total_card_count(&tg);
    test_pass(&tg); drain_choices(&tg);
    test_pass(&tg); drain_choices(&tg);
    test_pass(&tg); drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "q253 case 1: CENSUS -- the three post-live passes destroyed or duplicated no card");

    CHECK_EQ(score_of(&tg, galaxy), 0,
             "LiveSuccess score bonus cleared after live");

    /* NOT EXPRESSIBLE: Rust asserts performance_snapshots[0].lives[0].score -
     * .base_score == 1. RbLiveSnapshot (rabuka.h:1220-1240) has NO base_score
     * field, so the delta cannot be written in C. What IS asserted is that the
     * snapshot exists and records the live, so the bonus cannot hide behind a
     * missing snapshot. */
    const RbLiveSnapshot *s = NULL;
    for (int i = tg.state.n_snapshots - 1; i >= 0; i--)
        if (tg.state.snapshots[i].player == 0) { s = &tg.state.snapshots[i]; break; }
    CHECK(s != NULL,
          "q253 case 1: the live must produce a P1 performance snapshot, otherwise "
          "the +1 in the final score is unverifiable (Rust: score - base_score == 1; "
          "RbLiveSnapshot has no base_score, so the delta is not expressible in C)");
    if (s) {
        CHECK_EQ(s->n_lives, 1, "q253 case 1: the snapshot records the one live card");
        char msg[256];
        snprintf(msg, sizeof msg,
                 "q253 case 1: the live scored above zero (got %d) -- the printed base "
                 "score of %s carries the +1 the Rust delta is measuring",
                 s->live_score_detail[0], GALAXY);
        CHECK(s->live_score_detail[0] > 0, msg);
    }
}

static void test_live_success_recovery_removes_only_revealed_live_leaving_zero_score_modifier_q253(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int galaxy    = test_id_exact(&tg, GALAXY);
    int yell_live = mid(&tg, YELL_LIVE_1);
    int filler    = mid(&tg, FILLER);
    int kanan     = test_id_exact(&tg, KANAN);
    ident(galaxy, GALAXY, "q253 case 2");
    ident(yell_live, YELL_LIVE_1, "q253 case 2");
    ident(kanan, KANAN, "q253 case 2: 松浦果南");
    dbg_ability("KANAN", kanan);

    setup_q253_stage(&tg, kanan, test_id_exact(&tg, GK_L), test_id_exact(&tg, GK_C));
    CHECK_EQ(test_stage_has(&tg, 0, kanan), 1,
             "q253 case 2 control: 松浦果南 IS on the stage in this case");

    for (int i = 0; i < 20; i++) test_add_to_deck(&tg, filler);
    deck_insert(&tg, 0, 1, yell_live);

    CHECK(advance_named(&tg, "LiveCardSetFirstAttacker"),
          "q253 case 2: the turn reaches the live-card set phase");
    test_add_to_hand(&tg, galaxy);
    test_set_live_card(&tg, 0, galaxy);
    CHECK(advance_named(&tg, "FirstAttackerPerformance"),
          "q253 case 2: the turn reaches the first-attacker performance phase");
    drain_choices(&tg);

    int census = test_total_card_count(&tg);
    test_pass(&tg);
    take_first_mandatory(&tg);
    drain_choices(&tg);
    test_pass(&tg); drain_choices(&tg);
    test_pass(&tg); drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "q253 case 2: CENSUS -- the post-live passes destroyed or duplicated no card");

    CHECK_EQ(score_of(&tg, galaxy), 0,
             "Q253: Kanan took the live card from the yell, GALAXY should get +0");
}

static void test_live_success_recovery_leaves_revealed_live_for_one_score_modifier_q253(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int galaxy     = test_id_exact(&tg, GALAXY);
    int yell_live1 = mid(&tg, YELL_LIVE_1);
    int yell_live2 = mid(&tg, YELL_LIVE_2);
    int filler     = mid(&tg, FILLER);
    int kanan      = test_id_exact(&tg, KANAN);
    ident(galaxy, GALAXY, "q253 case 3");
    ident(yell_live1, YELL_LIVE_1, "q253 case 3");
    ident(yell_live2, YELL_LIVE_2, "q253 case 3");
    ident(kanan, KANAN, "q253 case 3: 松浦果南");

    setup_q253_stage(&tg, kanan, test_id_exact(&tg, GK_L), test_id_exact(&tg, GK_C));
    CHECK(test_card_name(yell_live1) && test_card_name(yell_live2) &&
          strcmp(test_card_name(yell_live1), test_card_name(yell_live2)) != 0,
          "q253 case 3 control: the two yell lives are different prints");
    CHECK(rb_card_is_live(yell_live1) && rb_card_is_live(yell_live2),
          "q253 case 3 control: both yell cards are LIVE cards");
    CHECK(yell_live1 != yell_live2,
          "q253 case 3 control: the two yell lives are two distinct card ids "
          "(mid(), not test_id(), or the zone would hold one card twice)");

    for (int i = 0; i < 20; i++) test_add_to_deck(&tg, filler);
    deck_insert(&tg, 0, 1, yell_live1);
    deck_insert(&tg, 0, 2, yell_live2);

    CHECK(advance_named(&tg, "LiveCardSetFirstAttacker"),
          "q253 case 3: the turn reaches the live-card set phase");
    test_add_to_hand(&tg, galaxy);
    test_set_live_card(&tg, 0, galaxy);
    CHECK(advance_named(&tg, "FirstAttackerPerformance"),
          "q253 case 3: the turn reaches the first-attacker performance phase");
    answer_prompts(&tg);

    int census = test_total_card_count(&tg);
    test_pass(&tg); answer_prompts(&tg);
    /* MEASURED mid-walk, before the live ends. The Rust's own case 1 asserts 0
     * after the SAME three passes ("LiveSuccess score bonus cleared after
     * live") while this case asserts 1 after them, so the pair is only
     * satisfiable if the bonus is NOT cleared -- this measurement says which of
     * the two the C engine does, and is reported rather than asserted. */
    printf("     q253 case 3: GALAXY score mod after the FIRST performance pass = %d\n",
           score_of(&tg, galaxy));
    test_pass(&tg); answer_prompts(&tg);
    test_pass(&tg); answer_prompts(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "q253 case 3: CENSUS -- the post-live passes destroyed or duplicated no card");

    CHECK_EQ(score_of(&tg, galaxy), 1,
             "Two live cards in the yell -- Kanan takes one, GALAXY still has one -> +1");

    /* DIAGNOSTIC, run on a SEPARATE board: the same live, the same trigger,
     * two revealed LIVE cards and no yell walk at all. It separates "the
     * ability never fires" from "the yell walk did not leave a revealed live
     * behind", and it is the PAIRED POSITIVE that keeps case 4's zero honest.
     * It is reported, not asserted as a substitute for the line above. */
    {
        static TestGame dg;
        rb_card_copy_pool_reset();
        test_game_new(&dg);
        int g2 = test_id_exact(&dg, GALAXY);
        int l1 = mid(&dg, YELL_LIVE_1), l2 = mid(&dg, YELL_LIVE_2);
        int nl = mid(&dg, FILLER);
        test_add_to_live(&dg, g2);
        test_add_to_revealed(&dg, l1);
        test_add_to_revealed(&dg, l2);
        test_add_to_revealed(&dg, nl);
        dg.state.phase = RB_PHASE_VICTORY;
        dg.state.live_success[0] = 1;
        CHECK(rb_should_trigger_live_success(&dg.state, 0),
              "q253 case 3 diagnostic control: the ライブ成功時 window is open");
        fire_live_success(&dg, 0, g2);
        drain_choices(&dg);
        int diag = score_of(&dg, g2);
        printf("     q253 case 3 DIAGNOSTIC: 2 revealed LIVE cards + 1 non-live, no yell "
               "walk -> GALAXY score mod %d (0 means the ability itself does not fire; "
               "1 means the yell walk above is what loses the card)\n", diag);
    }
}

/* The pure negative, dispatched directly, so it does not depend on the whole
 * live walk. Its PAIRED POSITIVE is the board of the three cases above: the
 * only difference is whether the revealed card is a live card. */
static void test_live_success_non_live_revealed_card_does_not_score_galaxy_q253(void)
{
    static TestGame tg;
    test_card_copy_pool_reset();
    test_game_new(&tg);
    int galaxy   = test_id_exact(&tg, GALAXY);
    int non_live = mid(&tg, FILLER);
    ident(galaxy, GALAXY, "q253 case 4");
    ident(non_live, FILLER, "q253 case 4");

    test_add_to_live(&tg, galaxy);
    test_add_to_revealed(&tg, non_live);

    /* CONTROLS at the assertion site, taken BEFORE the trigger. */
    CHECK_EQ(tg.state.n_revealed, 1, "q253 case 4 control: exactly one revealed card");
    CHECK_EQ(tg.state.revealed_cards[0], non_live,
             "q253 case 4 control: the revealed card is the one placed");
    CHECK(!rb_card_is_live(non_live),
          "q253 case 4 control: the revealed card really is NOT a live card -- "
          "this is the term under test");

    int census = test_total_card_count(&tg);
    /* rb_should_trigger_live_success needs BOTH live.n > 0 and live_success[pl];
     * fire_live_success sets the second. It is set HERE, at the assertion site,
     * so the control below reads an open window rather than a closed one. */
    tg.state.phase = RB_PHASE_VICTORY;
    tg.state.live_success[0] = 1;
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "q253 case 4 control: the ライブ成功時 window is genuinely open "
          "(live zone non-empty AND live_success[0] set)");

    fire_live_success(&tg, 0, galaxy);
    drain_choices(&tg);
    CHECK_EQ(test_total_card_count(&tg), census,
             "q253 case 4: CENSUS -- the live-success trigger destroyed or duplicated no card");
    CHECK_EQ(test_pending_choice_count(&tg), 0,
             "q253 case 4: a false condition raises no prompt");

    CHECK_EQ(score_of(&tg, galaxy), 0,
             "a revealed NON-live card must not score GALAXY");
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
        /* The counters are process globals the parent has been accumulating
         * across cases, so the child must start from zero. */
        assertions = 0;
        failures   = 0;
        /* The card-copy pool is process-global and monotonic; a case must not
         * inherit the previous case's copies. */
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
    /* The corpus verdict, in the shape every other FORK suite prints. The
     * `== case ...` line below stays for its per-case counts. */
    printf(g_shared->failures > 0 ? "FAILED: %s\n" : "ok: %s\n", c->name);
    printf("== case %-56s assertions=%d failures=%d\n",
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
     * RB_HEART_* enum order a reader would assume. Derived, never assumed. */
    printf("== heart colour mapping (rb_parse_heart_color)\n");
    {
        static const char *const NAMES[7] = {
            "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"
        };
        for (int i = 0; i < 7; i++)
            printf("   %s -> %d\n", NAMES[i], (int)rb_parse_heart_color(NAMES[i]));
        printf("   RB_HEART_PINK=%d RB_HEART_RED=%d RB_HEART_YELLOW=%d RB_HEART_GREEN=%d "
               "RB_HEART_BLUE=%d RB_HEART_PURPLE=%d RB_HEART_ORANGE=%d RB_HEART_ALL=%d\n",
               (int)RB_HEART_PINK, (int)RB_HEART_RED, (int)RB_HEART_YELLOW,
               (int)RB_HEART_GREEN, (int)RB_HEART_BLUE, (int)RB_HEART_PURPLE,
               (int)RB_HEART_ORANGE, (int)RB_HEART_ALL);
    }
    HEART02 = (int)rb_parse_heart_color("heart02");
    printf("   this file reads heart02 as index %d\n", HEART02);

    /* rb_card_group_name is SERIES-level (every ラブライブ！ card reports mu's);
     * rb_card_unit_name is the unit-level check. Print both for every fixture
     * whose group membership is load-bearing below. */
    printf("== fixture group / unit (group is SERIES-level by design)\n");
    {
        static const char *const FIX[] = {
            AURORA_FLOWER, AOKU_HARUKA, HOLIDAY, KAGAYAKU, NON_SUZU_MUS, NON_SUZU_NIJI,
            SUMIRE, DAISUKI, LINK_FUTURE, HASU_A, HASU_B, HASU_C, HASU_ONE,
            GALAXY, KANAN, YELL_LIVE_1, YELL_LIVE_2, FILLER, NULL
        };
        for (int i = 0; FIX[i]; i++) {
            int cid = rb_find_card_by_no(FIX[i]);
            printf("   %-18s unit=[%s] group=[%s]\n", FIX[i],
                   rb_card_unit_name(cid) ? rb_card_unit_name(cid) : "-",
                   rb_card_group_name(cid) ? rb_card_group_name(cid) : "-");
        }
    }

    g_shared = mmap(NULL, sizeof *g_shared, PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (g_shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: mmap(MAP_SHARED|MAP_ANONYMOUS) for the shared counters "
                        "failed; running without crash isolation\n");
        g_shared = NULL;
    }

    static const Case CASES[] = {
        { "aurora_flower_treated_as_three_groups_in_stage",   test_aurora_flower_treated_as_three_groups_in_stage },
        { "aurora_flower_does_not_match_unrelated_groups",   test_aurora_flower_does_not_match_unrelated_groups },
        { "aoku_haruka_live_start_scores_with_aurora",        test_aoku_haruka_live_start_scores_with_aurora_in_discard },
        { "aoku_haruka_live_start_no_score_when_aurora_missing", test_aoku_haruka_live_start_no_score_when_aurora_missing },
        { "aoku_haruka_live_start_fails_with_only_non_suzu",  test_aoku_haruka_live_start_fails_with_only_non_suzu_live_cards },
        { "sumire_no_active_energy_no_hearts",                test_sumire_no_active_energy_no_hearts },
        { "sumire_with_one_active_energy_gains_two_hearts",   test_sumire_with_one_active_energy_gains_two_hearts },
        { "sumire_inactive_energy_does_not_count",            test_sumire_inactive_energy_does_not_count },
        { "sumire_toggling_active_recalculates",              test_sumire_toggling_active_recalculates },
        { "daisuki_score_plus_one_when_active_energy_exists", test_daisuki_score_plus_one_when_active_energy_exists },
        { "daisuki_score_not_added_when_no_active_energy",    test_daisuki_score_not_added_when_no_active_energy },
        { "daisuki_opponent_active_does_not_satisfy_self",   test_daisuki_opponent_active_does_not_satisfy_self },
        { "q253_revealed_live_adds_one_to_final_score",       test_live_success_revealed_live_adds_one_to_final_score_q253 },
        { "q253_recovery_removes_only_revealed_live",         test_live_success_recovery_removes_only_revealed_live_leaving_zero_score_modifier_q253 },
        { "q253_recovery_leaves_revealed_live_for_one",       test_live_success_recovery_leaves_revealed_live_for_one_score_modifier_q253 },
        { "q253_non_live_revealed_card_does_not_score",       test_live_success_non_live_revealed_card_does_not_score_galaxy_q253 },
    };

    for (unsigned i = 0; i < sizeof CASES / sizeof CASES[0]; i++)
        run_case(&CASES[i]);

    printf("\n%d cases, %d assertions, %d failures\n",
           (int)(sizeof CASES / sizeof CASES[0]), assertions, failures);
    if (failures) { printf("SCORE PER_CARD-2 PARITY: RED\n"); return 1; }
    printf("ALL SCORE PER_CARD-2 PARITY CHECKS PASSED\n");
    return 0;
}
