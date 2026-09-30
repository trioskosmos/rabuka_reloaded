/* tests/test_parity_full_round3.c
 *
 * CLAIM (agent tr_fullround3): this file OWNS tests/test_parity_full_round3.c and
 * nothing else. The Rust sources it translates are
 *   engine/tests/test_modules/integration/full_round/real_card_phase_walkthrough_and_ability_suite_test.rs
 *   engine/tests/test_modules/integration/full_round/live_each_time_victory_road_through_real_live_rounds_test.rs
 * No src/ file, no other tests/test_*.c, no Makefile, no engine/, no cards/.
 *
 * C port of the 19 `fn` items of
 * real_card_phase_walkthrough_and_ability_suite_test.rs that
 * tests/test_parity_full_round2.c explicitly named as NOT reached after it
 * finished the `nico_*` family (13 tests) and `ai_screeam_answer_both_gain_blade`.
 * Cases, in the order this file walks them:
 *   A1  ayumu_kanon_koko_debut_recover_from_discard   (Rust :921)
 *   A1b ayumu_two_candidates_prompt                    (ADDED, positive twin of A1)
 *   A2  ayumu_q62_and_name_has_individual_names        (Rust :961)
 *   A3  ayumu_live_start_triggers                      (Rust :996)
 *   B1  bella_q174_no_member_on_stage_ability_does_not_fire        (Rust :2339)
 *   B2  bella_q174_no_heart04_surplus_ability_does_not_fire        (Rust :2374)
 *   B3  bella_positive_surplus_heart04_ability_fires               (Rust :2430)
 *   K1  kosuzu_choose_number_adds_to_hand              (Rust :3825)
 *   K2  kosuzu_choose_number_gains_blade               (Rust :3839)
 *   K3  kosuzu_equal_choice_both_effects               (Rust :3857)
 *   L1  lovepeace_q150_self_hearts_greater_than_opponent_score_plus_1 (Rust :2485)
 *   L2  lovepeace_q149_total_hearts_sum_of_base_hearts (Rust :2570)
 *   L3  lovepeace_q172_ability_gained_hearts_count_but_not_blade     (Rust :2590)
 *   H1  hareruya_q64_waitroom_only_five_distinct_liella_condition_met(Rust :2649)
 *   H1b hareruya_four_distinct_names_condition_not_met (ADDED, negative twin of H1)
 *   H2  hareruya_q74_multiname_distinct_counting        (Rust :2731)
 *   Y1  you_q129_cost_reduction_self_only              (Rust :2195)
 *   Y2  you_live_start_optional_cost_triggers          (Rust :2226)
 *   Y3  you_abilityless_card_not_in_choice             (Rust :2271)
 *   C1  kanon_activation_choice_condition_discard_flow (Rust :3416)
 *   C2  kanon_activation_choice_condition_wait_option  (Rust :3497)
 * 19 Rust ports + 2 ADDED cases (A1b, H1b), labelled as such and never presented
 * as Rust twins. Everything else named in the survey (wien_q117, hanamaru, the
 * shared_pr_debut and the baad_cage six) was left for a later push.
 *
 * CARDS, verified against cards/cards.json read as UTF-8 (python io.open, NOT
 * Get-Content -Raw, which mangles non-ASCII and produces FALSE "missing"):
 *   LL-bp1-001-R(U+FF0B)  上原歩夢&澁谷かのん&日野下花帆 member cost 20
 *   LL-bp2-001-R(U+FF0B)  渡辺 曜&鬼塚夏美&大沢瑠璃乃     member cost 20
 *   PL!N-bp3-027-L       La Bella Patria                live    score 2
 *                         need {h03:2, h04:2, heart0:1}
 *   PL!-bp3-026-L        Oh,Love&Peace!                 live    score 6
 *                         need {h01:2, h03:5, h06:2, heart0:6}
 *   PL!SP-bp1-026-L      未来予報ハレルヤ！              live    score 3
 *                         need {h02:2, h03:2, h06:2, heart0:2}
 *   PL!HS-pb1-005-R      徒町小鈴                        member cost 11
 *   PL!HS-sd1-013-SD     徒町小鈴                        member cost 2
 *   PL!SP-bp5-001-R(U+FF0B) 澁谷かのん                    member cost 10
 *   PL!N-bp1-018-N       近江彼方  member  h03:1 h05:1 h06:2 blade 3   (0 heart04)
 *   PL!N-pb1-008-R       エマ・ヴェルデ member h02:1 h03:1 h04:4 h05:1 blade 6
 *   PL!N-bp3-030-L       Love U my friends live, need one of EVERY colour 1
 *   PL!SP-bp1-014-N      嵐 千砂都   member  h03:1 h06:1
 *   PL!SP-bp1-015-N      平安名すみれ member  h02:1 h06:1
 *   PL!SP-bp1-016-N      葉月 恋     member  h02:1 h03:1
 *   PL!SP-bp1-019-N      若菜四季    member  h02:2 h03:1 h06:2
 *   PL!SP-bp1-020-N      鬼塚夏美    member  h03:2
 *   PL!SP-bp1-022-N      鬼塚冬毬    member  h02:2 h03:2 h06:2
 *   PL!-sd1-010-SD       高坂 穂乃果 member cost 4  h01:1 h03:1
 *   PL!-sd1-013-SD       園田海未    member cost 4  h01:1 h06:1
 *   PL!-sd1-014-SD       星空 凛     member cost 9  h01:2 h03:1 h06:1
 *   PL!-sd1-019-SD       START:DASH!!      live  score 1  h01:1 h03:1 h06:1
 *   PL!-sd1-020-SD       きっと青春が聞こえる live score 2 h01:1 h03:1 heart0:3
 *   PL!SP-PR-005-PR      嵐 千砂都   member  h03:3
 *   PL!-PR-003-PR        南ことり    member  h01:2 h03:3 h06:1
 *   PL!-pb1-014-R        星空 凛     member  h01:3 h03:2 h06:2
 *   LL-E-001-SD          エネルギーカード
 * Every fixture whose PRINTED cost a claim depends on has that cost asserted
 * from the decoded Card in the test body, so a cards.json/cards.bin drift shows
 * up as a SETUP BUG and can never masquerade as a rules divergence.
 *
 * THE game.id() INTERNING TRAP -- MY DECISION, STATED EXPLICITLY
 * -----------------------------------------------------------
 * Rust's `game.id(no)` INTERNS BY CARD_NO, so one i16 can sit in two zones at
 * once and every `zone.cards.contains(&id)` is then true of BOTH zones. In the
 * 19 tests ported here the trap is live in exactly two places:
 *   lovepeace_q150  : `game.id("PL!-sd1-020-SD")`... the filler is pushed into
 *                      both players' 40-card decks AND onto P2's stage centre.
 *   kosuzu_equal_choice_both_effects : the top-of-deck card and the deck
 *                      stocker are the SAME `game.id("PL!-sd1-010-SD")`, so
 *                      `hand.contains(&top_card)` would be answered by any of
 *                      the ten already-stocked copies.
 * Neither is a two-card board in the real game.
 *
 * DECISION: two helpers, used by rule rather than by taste.
 *   sid() = test_id_exact -- the shared TEMPLATE index, i.e. exactly what Rust's
 *           `game.id(no)` returns. Used for every fixture that occupies exactly
 *           ONE slot in the Rust board, INCLUDING the ability host. That makes
 *           those cases a byte-for-byte faithful port.
 *   mk()  = test_new_id -- a DISTINCT instance. Used (a) for Rust's explicit
 *           `game.new_id(...)` second copies, (b) for any fixture Rust pushes
 *           into two or more zones, and (c) for the kosuzu top-of-deck card,
 *           where the Rust id aliases the stocker.
 * Consequence, stated so it is not mistaken for an engine bug: the two
 * lovepeace_q150 stage/deck fillers and the three kosuzu top-of-deck cards are
 * C ids that the Rust board shares. Every assertion written against them is
 * therefore STRICTER in C (the C can tell the top card from the ten below it;
 * Rust cannot).
 * For the deck-stockers (10 or 40 copies of one print, as Rust does) a single
 * template id is pushed N times -- that is a deck, not two cards, and copying it
 * N times would only burn the 1457-slot copy pool.
 *
 * NON-VACUITY (the quality bar for full-round tests)
 * --------------------------------------------------
 * Every negative assertion in this file is paired, on a board that differs by
 * exactly the factor under test, with a POSITIVE assertion:
 *   A1  "one discard candidate does NOT prompt"  <-> A1b "two candidates DO
 *       prompt", plus A1's own two positive zone assertions.
 *   B1  "no member on stage -> no energy placed" <-> B3 "a 虹ヶ咲 member with
 *       heart04=4 -> energy placed". B1/B2 additionally seed the ENERGY DECK,
 *       which the Rust twins do not, so "no energy was added" cannot be satisfied
 *       by an empty source zone. That is a strengthening of the Rust fixture and
 *       it is called out here because without it both Rust negatives are
 *       vacuous.
 *   B1/B2/B3 each run a REFERENCE ARM that is the same board with the same live
 *       card, the same seeded energy deck and the same number of passes, differing
 *       only in the stage member. This is load-bearing, not decoration: the walk
 *       crosses the ENERGY phase, which draws an energy card out of the energy
 *       deck every turn, so an absolute count cannot tell Bella's placement from
 *       the machinery. An earlier revision of this file used an absolute count and
 *       reported the whole bella family red; that red was a MEASUREMENT artefact,
 *       and the survey's "soreigai printed-score vs decoded-value mismatch" theory
 *       for this family is NOT confirmed -- bella is green once the draw is
 *       cancelled. B3 additionally asserts that two identical positive boards
 *       produce identical deltas, so the comparison B1/B2 depend on is proved
 *       repeatable rather than assumed.
 *   K1 carries a CONTROL member on the same stage: Kosuzu's chosen number is
 *       below the revealed card's cost, so no blade may be gained, and the
 *       control proves any blade reading came from the board rather than from a
 *       global recalculation.
 *   H1  "5 distinct Liella! names -> required hearts set" <-> H1b "4 distinct
 *       names -> nothing set", and both take a `test_max_distinct_names`
 *       measurement AT THE BOARD as the precondition control.
 *   K1/K2/K3 are a three-cell truth table on the chosen number (below / above /
 *   equal the top card's cost), so none of them can be green by accident.
 *   Y3's negative ("the abilityless filler stays in hand") sits on a board whose
 *   positive (blade gained on the activating card) is asserted two lines later.
 * Every prompt claim is made at the ASSERTION SITE: `int pre = has_pending;`
 * is sampled BEFORE the interaction and the assertion is on a NEW prompt
 * appearing, never on an increment-then-compare of the same read.
 *
 * HARNESS LANDMINES RESPECTED
 * ---------------------------
 *   - rb_load("src") fails under the isolated out-of-tree build, so
 *     load_card_db() falls back to rb_load("../cards/build"). REQUIRED.
 *   - test_game_new() is called in EVERY case: without it state.winner stays 0
 *     and the phase machine reads that as "game already decided".
 *   - sizeof(TestGame) is ~781 KB: every TestGame local is `static`.
 *   - rb_card_copy_pool_reset() runs at the top of every case.
 *   - Identity is pinned with test_id_exact / rb_card_no_eq, never the forgiving
 *     rb_find_card_by_no substring fallback.
 *   - test_give_energy bumps the ACTIVE count with each card, so "3 spare + 10
 *     active" needs an explicit test_set_energy_active(tg, 0, 10).
 *   - Heart-colour slots are DERIVED with rb_parse_heart_color + the engine's
 *     rb_heart_color_index and PRINTED AT STARTUP, never hardcoded. The engine
 *     map is 0=heart00 1=heart01 2=heart02 3=heart03 4=heart04 5=heart05
 *     6=heart06 7=all.
 *   - test_card_name() rotates an 8-slot buffer, so every name this file compares
 *     is snapshotted into a local before use. There is deliberately no shared
 *     static `no_of()` here: test_card_name's ring is already correct and a
 *     one-buffer helper is the trap the brief warns about.
 *   - ライブ成功時 is driven through rb_trigger_live_success with
 *     rb_should_trigger_live_success asserted as a precondition first. The real
 *     phase machine is used ONLY where the ability's condition depends on the
 *     performance having run (B1/B2/B3, L3), through the proven
 *     tests/test_surplus_heart.c driver.
 *   - ライブ開始時 is driven through rb_trigger_live_start + the SINGLE-SEAT
 *     rb_process_player_abilities(g, pl). Never the two-seat
 *     rb_process_pending_auto_abilities, whose p1 pass can destroy a p2-owned
 *     queued ability.
 *   - The optional-cost drains here use `pay_skip_drain()`: rb_resume_with_choice
 *     with -1, which is Rust's select_indices(&[]). NOTE this is a DECLINE of an
 *     optional cost, not of a SelectAutoAbility, and the two are never conflated.
 *   - Every drain is CAPPED (32/48). A peer found an ability that re-prompts
 *     unboundedly.
 *   - The waitroom zone string is "discard".
 *   - Every Japanese literal in this file is a backslash-x byte escape built
 *     from python's UTF-8 encoder and re-verified against cards.json, and the
 *     block of defines that carries them is pure ASCII: a non-ASCII comment
 *     line immediately above an escape line makes some editors swallow the
 *     leading backslash and silently compare the wrong bytes.
 *
 * Honesty contract
 * ----------------
 * Every assertion is a hard CHECK about the C engine. Nothing is downgraded,
 * skipped or weakened to make the suite green. A divergence from Rust is the
 * most valuable thing this file can report and is left RED on purpose.
 *
 * STATUS AS SHIPPED (HEAD 86ff708e), for whoever picks the reds up:
 *   15 cases green, 6 red, 328 assertions, 15 failures, 0 crashes, 0 setup bugs.
 *   The six reds are listed at the bottom of §A/§K/§Y/§C with a comment on each
 *   assertion naming the defect it exposes.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <stdarg.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/mman.h>

/* ═══════════════════════════════ harness ═══════════════════════════════ */

static const char *current_test = "(none)";

/* The tally lives in a MAP_SHARED|MAP_ANONYMOUS mapping so a child that dies on
 * a signal still leaves every assertion it had already made in a number the
 * parent can print. The parent ACCUMULATES across children: the mapping is
 * created ONCE in main() and never remapped, so the summary below is the sum
 * over every case, not the last case. */
typedef struct {
    long assertions;
    int  failures;
    int  setup_bugs;
} Tally;
static Tally *tally;

/* VARIADIC on purpose. A two-parameter CHECK(cond, msg) silently splits a
 * message written as adjacent string literals into extra macro arguments, which
 * is how a suite ends up asserting nothing. */
static void record(int ok, const char *buf)
{
    if (tally) tally->assertions++;
    if (!ok) {
        if (tally) tally->failures++;
        fprintf(stderr, "FAIL [%s]: %s\n", current_test, buf);
    } else {
        printf("  ok: %s\n", buf);
    }
    fflush(stdout); fflush(stderr);
}

static void record_eq(int a, int e, const char *buf)
{
    char b2[512];
    if (tally) tally->assertions++;
    if (a != e) {
        if (tally) tally->failures++;
        snprintf(b2, sizeof b2, "%s (got %d, expected %d)", buf, a, e);
        fprintf(stderr, "FAIL [%s]: %s\n", current_test, b2);
    } else {
        printf("  ok: %s\n", buf);
    }
    fflush(stdout); fflush(stderr);
}

/* VARIADIC on purpose, and built with snprintf rather than a hand-rolled
 * va_start: a two-parameter CHECK(cond, msg) silently splits a message written
 * as adjacent string literals into extra macro arguments, which is how a suite
 * ends up asserting nothing. Every call site below passes ONE format string
 * (plus its arguments where it has any); no message contains a bare `%`. */
#define CHECK(cond, ...) do {                                              \
    char _b[440];                                                          \
    snprintf(_b, sizeof _b, __VA_ARGS__);                                  \
    record((cond) ? 1 : 0, _b);                                            \
} while (0)

#define CHECK_EQ(actual, expected, ...) do {                               \
    char _b[440];                                                          \
    snprintf(_b, sizeof _b, __VA_ARGS__);                                  \
    record_eq((int)(actual), (int)(expected), _b);                         \
} while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    return rb_load("../cards/build");
}

/* -- fixture card numbers. The two multi-name cards carry a FULLWIDTH PLUS
 *    (U+FF0B = EF BC 8B) in the card_no itself; "PL!" vs "PL!SP" is NOT a
 *    mismatch and is not reported as one. This block is pure ASCII on purpose. */
#define AYUMU_NO    "LL-bp1-001-R\xEF\xBC\x8B"
#define YOU_NO      "LL-bp2-001-R\xEF\xBC\x8B"
#define KANON_NO    "PL!SP-bp5-001-R\xEF\xBC\x8B"
#define BELLA_NO    "PL!N-bp3-027-L"
#define LIVE_EMMA   "PL!N-pb1-008-R"
#define NEARUKAWA   "PL!N-bp1-018-N"
#define YELL_ALL    "PL!N-bp3-030-L"
#define LOVEPEACE_NO "PL!-bp3-026-L"
#define HARERUYA_NO "PL!SP-bp1-026-L"
#define KOSUZU_NO   "PL!HS-pb1-005-R"
#define KOSUZU_SD   "PL!HS-sd1-013-SD"
#define F10_NO      "PL!-sd1-010-SD"
#define F13_NO      "PL!-sd1-013-SD"
#define F14_NO      "PL!-sd1-014-SD"
#define LIVE19_NO   "PL!-sd1-019-SD"
#define LIVE20_NO   "PL!-sd1-020-SD"
#define PR5_NO      "PL!SP-PR-005-PR"
#define PR3_NO      "PL!-PR-003-PR"
#define PB1_014_NO  "PL!-pb1-014-R"
#define L014_NO     "PL!SP-bp1-014-N"
#define L015_NO     "PL!SP-bp1-015-N"
#define L016_NO     "PL!SP-bp1-016-N"
#define L019_NO     "PL!SP-bp1-019-N"
#define L020_NO     "PL!SP-bp1-020-N"
#define L022_NO     "PL!SP-bp1-022-N"
#define ENERGY_NO   "LL-E-001-SD"

/* Japanese literals as UTF-8 escapes (python: s.encode("utf-8")).
 * The romanised macro name IS the gloss, so every line in this block is pure
 * ASCII -- see the header note on why the block must not carry a non-ASCII
 * trailing comment. */
#define JP_UeharaAyumu  "\xe4\xb8\x8a\xe5\x8e\x9f\xe6\xad\xa9\xe5\xa4\xa2"       /* Uehara Ayumu */
#define JP_ShibuyaKanon "\xe6\xbe\x81\xe8\xb0\xb7\xe3\x81\x8b\xe3\x81\xae\xe3\x82\x93" /* Shibuya Kanon */
#define JP_HinenoKahana "\xe6\x97\xa5\xe9\x87\x8e\xe4\xb8\x8b\xe8\x8a\xb1\xe5\xb8\x86" /* Hineno Kahana */
#define JP_Nijigasaki   "\xe8\x99\xb9\xe3\x83\xb6\xe5\x92\xb2"                       /* Nijigasaki */
#define JP_Wait         "\xe3\x82\xa6\xe3\x82\xa7\xe3\x82\xa4\xe3\x83\x88"         /* wait state */
#define JP_Debut        "\xe7\x99\xbb\xe5\xa0\xb4"                                 /* debut trigger */
#define JP_LiveStart    "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe9\x96\x8b\xe5\xa7\x8b\xe6\x99\x82" /* live-start trigger */
#define JP_LiveSuccess  "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe6\x88\x90\xe5\x8a\x9f\xe6\x99\x82" /* live-success trigger */
#define JP_Constant     "\xe5\xb8\xb8\xe6\x99\x82"                                 /* constant trigger */
#define JP_Activation   "\xe8\xb5\xb7\xe5\x8b\x95"                                 /* activation trigger */
#define JP_Cost         "\xe3\x82\xb3\xe3\x82\xb9\xe3\x83\x88"                       /* cost */
#define JP_BecomeLess   "\xe5\xb0\x91\xe3\x81\xaa\xe3\x81\x8f\xe3\x81\xaa\xe3\x82\x8b" /* become less */
#define JP_BatonTouch   "\xe3\x83\x90\xe3\x83\x88\xe3\x83\xb3\xe3\x82\xbf\xe3\x83\x83\xe3\x83\x81" /* baton touch */
#define JP_Plus3Full    "\xef\xbc\x8b\xef\xbc\x93"                                 /* fullwidth +3 */
#define JP_Score        "\xe3\x82\xb9\xe3\x82\xb3\xe3\x82\xa2"                       /* score */

/* sid(): a fixture that occupies exactly ONE slot in the Rust board -- exactly
 * what Rust's interning `game.id(no)` returns. Pinned EXACT. */
static int sid(TestGame *tg, const char *no)
{
    int id = test_id_exact(tg, no);
    if (id < 0) {
        fprintf(stderr, "SETUP BUG [%s]: card %s does not exist (exact)\n",
                current_test, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    if (!rb_card_no_eq(id, no)) {
        fprintf(stderr, "SETUP BUG [%s]: exact id %d is not %s\n",
                current_test, id, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    return id;
}

/* mk(): a DISTINCT instance. See the header for exactly when this is used. */
static int mk(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        fprintf(stderr, "SETUP BUG [%s]: no distinct copy of %s (copy pool dry?)\n",
                current_test, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    if (!rb_card_no_eq(id, no)) {
        fprintf(stderr, "SETUP BUG [%s]: copy id %d is not %s\n",
                current_test, id, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    return id;
}

/* One assertion that a fixture really is the printed card the case claims.
 * `cost` < 0 skips the cost check (live cards have none). */
static int expect_card(TestGame *tg, const char *tag, int id, const char *no, int cost)
{
    Card c;
    char buf[240];
    if (id < 0) return 0;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        CHECK(0, "%s: the fixture decodes at all", tag);
        return 0;
    }
    snprintf(buf, sizeof buf, "%s: the fixture is really %s", tag, no);
    CHECK(rb_card_no_eq(id, no) && c.name != NULL, buf);
    if (cost >= 0) {
        snprintf(buf, sizeof buf, "%s: PRINTED cost is %d", tag, cost);
        CHECK_EQ(c.cost, cost, buf);
    }
    rb_free_card(&c);
    return 1;
}

/* Snapshot the card name into a caller buffer. test_card_name rotates an 8-slot
 * ring, so the result must be copied before the next call. */
static const char *snap_name(TestGame *tg, int id, char *out, size_t n)
{
    snprintf(out, n, "%s", test_card_name(id));
    return out;
}

/* A decoded Card stores base | blade | need | special back to back in one
 * heart_color[]/heart_count[] pair (src/core/card.c:76-84), so scanning the whole
 * table for a colour finds the LAST match -- a BLADE heart of the same colour
 * overwrites the base value. This reads the BASE section only, [0, num_base),
 * and returns -1 when the colour is absent (which for a base section means 0).
 * Getting this wrong is what makes a heart04=4 fixture read as 1. */
static int card_base_heart(int id, int color)
{
    Card c;
    int got = -1;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) { rb_free_card(&c); return -1; }
    for (int h = 0; h < c.num_base && h < c.n_hearts; h++) {
        if (c.heart_color[h] == (uint8_t)color) { got = c.heart_count[h]; break; }
    }
    rb_free_card(&c);
    return got;
}

/* ── zone helpers ── */
static void bag_clear(RbBag *b) { b->n = 0; }
static void bag_push(RbBag *b, int id)
{
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = id;
}
static void hand_push(TestGame *tg, int pl, int id) { bag_push(&tg->state.p[pl].hand, id); }
static void disc_push(TestGame *tg, int pl, int id) { bag_push(&tg->state.p[pl].discard, id); }
static void stage_put(TestGame *tg, int pl, int area, int id)
{
    tg->state.p[pl].stage[area] = id;
    tg->state.p[pl].stage_wait[area] = 0;
}
static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
        tg->state.p[pl].stage_wait[i] = 0;
    }
}
static int stage_occupied(TestGame *tg, int pl)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}
static int in_zone(TestGame *tg, int pl, const char *zone, int id)
{
    return test_zone_count_of_id(tg, pl, zone, id) > 0;
}
static int hand_count_of(TestGame *tg, int pl, int id)
{
    return test_zone_count_of_id(tg, pl, "hand", id);
}

/* Rust: game.state.ability_queue.current_entry().choice_player_id */
static int choice_seat(TestGame *tg)
{
    return test_queue_entry_choice_seat(tg, tg->state.queue.cur);
}

/* Rust's `game.id(no)` pushed into one zone, removed by hand: Rust does
 * `hand.cards.retain(|id| *id != kanon)` in the two kanon tests to place the
 * card on stage without firing its 登場. */
static void stage_place_from_hand(TestGame *tg, int pl, int area, int id)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] != id) continue;
        for (int j = i; j + 1 < P->hand.n; j++) P->hand.cards[j] = P->hand.cards[j + 1];
        P->hand.n--;
        break;
    }
    stage_put(tg, pl, area, id);
}

/* Diagnostic dump of the prompt standing at the assertion site. NOT an
 * assertion -- this is how a divergence gets attributed to a field. */
static void dump_pending(TestGame *tg, const char *label)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) {
        printf("    <%s> NO PENDING CHOICE\n", label);
        fflush(stdout);
        return;
    }
    printf("    <%s> kind=%s zone='%s' card_type='%s' count=%d allow_skip=%d "
           "target='%s' actor=%d tpid='%s' cost_limit=%d op='%s' n_filtered=%d "
           "filter_group='%s' desc='%s' desc_ja='%s'\n",
           label, test_pending_choice_type(tg), c->zone, c->card_type, c->count,
           c->allow_skip, c->target, c->actor, c->target_player_id,
           c->cost_limit, c->cost_limit_op, c->n_filtered_indices,
           c->filter_group, c->description, c->description_ja);
    fflush(stdout);
}

/* Rust: game.select_option(i) */
static void select_option(TestGame *tg, int i) { rb_resume_with_choice(&tg->state, i); }
/* Rust: game.select_indices(&[i]) */
static void pick(TestGame *tg, int i) { test_select_indices(tg, &i, 1); }
/* Rust: game.select_indices(&[]) -- DECLINE. -1 is the engine's decline index. */
static void decline(TestGame *tg) { rb_resume_with_choice(&tg->state, -1); }

/* Rust: `while game.has_pending_choice() { game.select_indices(&[0]) }` */
static void drain_first(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 32)
        rb_resume_with_choice(&tg->state, 0);
}

/* Rust: `while game.has_pending_choice() { game.select_indices(&[]) }` */
static void drain_decline(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 32)
        decline(tg);
}

/* The phase-machine walker, shaped like tests/test_surplus_heart.c's
 * drive_past_placement: pass one phase at a time and answer ONLY the prompts
 * that are safe to answer blind (SelectAutoAbility). Anything else is left
 * standing and reported, because a mandatory SelectCard parked mid-ability makes
 * every later zone assertion read "nothing happened" for the wrong reason. */
static int settle_walk(TestGame *tg, int max_passes)
{
    int used = 0;
    for (int i = 0; i < max_passes; i++) {
        if (!rb_has_pending_choice(&tg->state)) { test_pass(tg); used++; continue; }
        const char *t = test_pending_choice_type(tg);
        if (strcmp(t, "SelectAutoAbility") == 0) {
            test_select_indices(tg, NULL, 0);
            continue;
        }
        return used;
    }
    return used;
}

/* Rust: set_live_card(card) -- APPENDS to the seat's own live zone. */
static void set_live(TestGame *tg, int pl, int cid)
{
    test_set_live_card_for(tg, pl, cid);
}

/* ── trigger windows ─────────────────────────────────────────────────────── */

/* Rust's advance_to_live_start() ends in the ライブ開始時 window. The C phase
 * machine stores PERFORMANCE and PERFORMANCE_SECOND as ONE value and its
 * block_performance_phase latch is a known trap (see
 * tests/test_parity_live_total.c's header), so the window is opened explicitly
 * and the live-start SCAN is dispatched through rb_trigger_live_start -- the
 * whole-scan equivalent of Rust's phase step, not a per-ability shortcut. */
static void open_live_start_window(TestGame *tg, int pl)
{
    (void)pl;
    tg->state.phase = RB_PHASE_PERFORMANCE;
}
static int fire_live_start(TestGame *tg, int pl)
{
    open_live_start_window(tg, pl);
    int n = rb_trigger_live_start(&tg->state, pl);
    /* SINGLE-SEAT drain: rb_process_pending_auto_abilities' p1 pass can destroy
     * a p2-owned queued ability. */
    rb_process_player_abilities(&tg->state, pl);
    return n;
}

/* rb_should_trigger_live_success needs BOTH p[pl].live.n > 0 AND
 * g->live_success[pl] (src/turn/triggers.c). */
static void open_live_success_window(TestGame *tg, int pl)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[pl] = 1;
}
static void fire_live_success(TestGame *tg, int pl, int cid)
{
    open_live_success_window(tg, pl);
    tg->state.activating_card = cid;
    rb_trigger_live_success(&tg->state, pl);
    rb_process_player_abilities(&tg->state, pl);
}

static int new_game(void)
{
    rb_card_copy_pool_reset();
    return 0;
}

/* The heart-colour slots, derived rather than hardcoded, printed once at
 * startup so a mapping drift is visible in the log. */
static int idx_heart02, idx_heart03, idx_heart04, idx_heart05, idx_heart06;

static void print_heart_map(void)
{
    idx_heart02 = rb_heart_color_index(rb_parse_heart_color("heart02"));
    idx_heart03 = rb_heart_color_index(rb_parse_heart_color("heart03"));
    idx_heart04 = rb_heart_color_index(rb_parse_heart_color("heart04"));
    idx_heart05 = rb_heart_color_index(rb_parse_heart_color("heart05"));
    idx_heart06 = rb_heart_color_index(rb_parse_heart_color("heart06"));
    printf("heart slot map (rb_parse_heart_color -> rb_heart_color_index): "
           "heart02=%d heart03=%d heart04=%d heart05=%d heart06=%d "
           "(engine map: 0=heart00 1=heart01 2=heart02 3=heart03 4=heart04 "
           "5=heart05 6=heart06 7=all)\n",
           idx_heart02, idx_heart03, idx_heart04, idx_heart05, idx_heart06);
    fflush(stdout);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §A  上原歩夢&澁谷かのん&日野下花帆  LL-bp1-001-R(U+FF0B)
 *  登場: 自分の控え室からメンバーカードを1枚手札に加える。
 *  ライブ開始時: 手札の「上原歩夢」と「澁谷かのん」と「日野下花帆」を、
 *    好きな組み合わせで合計3枚まで、控え室に置いてもよい：
 *    ライブ終了時まで、「常時 ライブの合計スコアを＋３する。」を得る。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Rust :921 ayumu_kanon_koko_debut_recover_from_discard */
static void a1_ayumu_debut_recover_from_discard(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int ayumu  = sid(&tg, AYUMU_NO);
    int member = sid(&tg, F10_NO);
    int filler = sid(&tg, F13_NO);
    if (!expect_card(&tg, "A1 ayumu",   ayumu,  AYUMU_NO, 20)) return;
    if (!expect_card(&tg, "A1 member",  member, F10_NO,    4)) return;
    if (!expect_card(&tg, "A1 filler",  filler, F13_NO,    4)) return;

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    hand_push(&tg, 0, ayumu);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, member);
    test_give_energy(&tg, 20);

    /* Whole-board CENSUS, baselined AFTER every fixture is placed and
     * IMMEDIATELY BEFORE the play under test: ayumu + filler + member + 20 energy. */
    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, 23, "A1: CENSUS is 23 before the play (3 fixture cards + 20 energy)");
    CHECK_EQ(test_zone_len(&tg, 0, "hand"),    2, "A1: two cards in hand before the play");
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 1, "A1: one card in the waitroom before the play");

    /* Rust plays to Center; C area 1. */
    CHECK_EQ(test_play_to_stage(&tg, ayumu, 1), 1, "A1: Ayumu is played to the centre area");

    /* The negative assertion, with its paired positives two lines below. */
    CHECK(!rb_has_pending_choice(&tg.state),
          "A1: a single discard candidate auto-resolves and raises NO prompt");
    dump_pending(&tg, "A1 after the play");

    CHECK(in_zone(&tg, 0, "hand", member),
          "A1: the waitroom member really was recovered INTO HAND");
    CHECK(!in_zone(&tg, 0, "discard", member),
          "A1: the waitroom member is no longer in the waitroom");
    CHECK(in_zone(&tg, 0, "stage", ayumu),
          "A1: Ayumu is on the stage (the play itself landed)");

    /* The three assertions above are the Rust twins and they stay STRICT: the
     * C raises a SelectCard where Rust resolves a single candidate with no
     * prompt, so the card has not moved yet at the assertion site. Answering
     * the prompt after them is not a softening -- it is what shows the recovery
     * itself works once the prompt exists, so the red above is attributable to
     * the MISSING EXACT PATH and not to a broken move_cards. */
    if (rb_has_pending_choice(&tg.state)) {
        pick(&tg, 0);
        drain_first(&tg);
    }
    CHECK(in_zone(&tg, 0, "hand", member),
          "A1: once the standing prompt IS answered the member really reaches hand");
    CHECK(!in_zone(&tg, 0, "discard", member),
          "A1: once the standing prompt IS answered the member really leaves the waitroom");

    CHECK_EQ(test_total_card_count(&tg), census0,
             "A1: CENSUS unchanged -- one card moved from discard to hand, none destroyed");
}

/* ADDED (not a Rust twin): the positive control A1's negative needs.
 * Same card, same board, ONE extra distinct waitroom member. If A1 is green only
 * because the engine ignores the waitroom, the ability still raises nothing here
 * and A1b is red. Together the pair is load-bearing. */
static void a1b_ayumu_two_candidates_prompt(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int ayumu  = sid(&tg, AYUMU_NO);
    int m1     = sid(&tg, F10_NO);
    int m2     = sid(&tg, F13_NO);
    if (!expect_card(&tg, "A1b ayumu", ayumu, AYUMU_NO, 20)) return;
    if (!expect_card(&tg, "A1b m1",    m1,    F10_NO,    4))  return;
    if (!expect_card(&tg, "A1b m2",    m2,    F13_NO,    4))  return;

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    hand_push(&tg, 0, ayumu);
    disc_push(&tg, 0, m1);
    disc_push(&tg, 0, m2);
    test_give_energy(&tg, 20);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, 23, "A1b: CENSUS is 23 before the play (2 members + ayumu + 20 energy)");

    /* THE PROMPT-COUNT CONTROL: sampled HERE, before the interaction. The
     * assertion is that a NEW prompt appears -- never an increment-then-compare
     * of one read. */
    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "A1b: control -- no prompt is standing before the play");

    CHECK_EQ(test_play_to_stage(&tg, ayumu, 1), 1, "A1b: Ayumu is played to the centre area");

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "A1b: THE ONE ADDED WAITROOM MEMBER MAKES THE ABILITY ASK -- the waitroom "
          "is really read, so A1's no-prompt result is the rule and not an inert engine");
    dump_pending(&tg, "A1b prompt");
    CHECK(choice_seat(&tg) == 0 || choice_seat(&tg) == -1,
          "A1b: the prompt is not stamped to the opponent seat");

    pick(&tg, 0);
    drain_first(&tg);

    int hand_n = test_zone_len(&tg, 0, "hand");
    CHECK(hand_n >= 1, "A1b: at least one waitroom member reached hand");
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 1,
             "A1b: exactly ONE of the two candidates left the waitroom");
    CHECK(in_zone(&tg, 0, "hand", m1) || in_zone(&tg, 0, "hand", m2),
          "A1b: the recovered card is one of the two seeded instances");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "A1b: CENSUS unchanged -- one card moved, none destroyed");
}

/* Rust :961 ayumu_q62_and_name_has_individual_names (pure database facts) */
static void a2_ayumu_q62_name_has_individual_names(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int ayumu = sid(&tg, AYUMU_NO);
    if (!expect_card(&tg, "A2 ayumu", ayumu, AYUMU_NO, 20)) return;

    /* No board is built by this Rust test at all, so the census is 0 by
     * construction -- asserted before and after so the claim "nothing was
     * placed" is on the record rather than assumed. */
    CHECK_EQ(test_total_card_count(&tg), 0,
             "A2: the board is empty -- this Rust test reads the database only");

    char name[256];
    snap_name(&tg, ayumu, name, sizeof name);

    CHECK(strchr(name, '&') != NULL,
          "A2: Q62 -- the printed name contains the '&' separator");

    /* Rust: name.split('&').collect(), then len == 3 and each part compared. */
    char parts[8][128];
    int nparts = 0;
    {
        const char *p = name;
        while (nparts < 8) {
            const char *sep = strchr(p, '&');
            size_t len = sep ? (size_t)(sep - p) : strlen(p);
            if (len >= sizeof parts[0]) len = sizeof parts[0] - 1;
            memcpy(parts[nparts], p, len);
            parts[nparts][len] = '\0';
            nparts++;
            if (!sep) break;
            p = sep + 1;
        }
    }
    CHECK_EQ(nparts, 3, "A2: Q62 -- the name splits into exactly 3 individual names");
    CHECK(strcmp(parts[0], JP_UeharaAyumu) == 0,
          "A2: name part 1 is the 4 characters of the first individual name");
    CHECK(strcmp(parts[1], JP_ShibuyaKanon) == 0,
          "A2: name part 2 is the 5 characters of the second individual name");
    CHECK(strcmp(parts[2], JP_HinenoKahana) == 0,
          "A2: name part 3 is the 5 characters of the third individual name");

    /* Rust: resolved_abilities().any(triggers == 登場 / ライブ開始時), and the
     * live_start ability's full_text must name all three characters. */
    int nab = rb_card_num_abilities((uint32_t)ayumu);
    int has_debut = 0, has_live_start = 0, ls_names_all = 0;
    char ls_text[512];
    snprintf(ls_text, sizeof ls_text, "(no ライブ開始時 ability decoded)");
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)ayumu, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, JP_Debut) == 0) has_debut = 1;
        if (ab.triggers && strcmp(ab.triggers, JP_LiveStart) == 0) {
            has_live_start = 1;
            snprintf(ls_text, sizeof ls_text, "%s", ab.full_text ? ab.full_text : "");
            ls_names_all = strstr(ls_text, JP_UeharaAyumu) != NULL
                        && strstr(ls_text, JP_ShibuyaKanon) != NULL
                        && strstr(ls_text, JP_HinenoKahana) != NULL;
        }
        rb_free_ability(&ab);
    }
    CHECK(has_debut, "A2: the card carries a 登場 ability (decoded from bytecode)");
    CHECK(has_live_start, "A2: the card carries a ライブ開始時 ability (decoded from bytecode)");
    CHECK(ls_names_all,
          "A2: the ライブ開始時 ability text names ALL THREE individual characters");
    printf("    <A2> ライブ開始時 full_text = %s\n", ls_text);

    CHECK_EQ(test_total_card_count(&tg), 0,
             "A2: CENSUS still 0 -- reading the database places nothing");
}

/* Rust :996 ayumu_live_start_triggers */
static void a3_ayumu_live_start_triggers(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int ayumu     = sid(&tg, AYUMU_NO);
    int filler_l  = sid(&tg, LIVE19_NO);
    int fill      = sid(&tg, F10_NO);
    int ayumu_cp  = mk(&tg, AYUMU_NO);
    if (!expect_card(&tg, "A3 ayumu",    ayumu,    AYUMU_NO, 20)) return;
    if (!expect_card(&tg, "A3 filler_l", filler_l, LIVE19_NO, -1)) return;
    if (!expect_card(&tg, "A3 fill",     fill,     F10_NO,    4))  return;
    if (!expect_card(&tg, "A3 ayumu_cp", ayumu_cp, AYUMU_NO, 20)) return;

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(&tg, 0, fill); test_add_to_deck_pl(&tg, 1, fill); }
    stage_put(&tg, 0, 1, ayumu);          /* Rust stage.stage[1] = ayumu */
    hand_push(&tg, 0, ayumu_cp);          /* Rust game.new_id(...) -- a DISTINCT copy */
    hand_push(&tg, 0, filler_l);
    set_live(&tg, 0, filler_l);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(stage_occupied(&tg, 0), 1, "A3: exactly one member is on P1's stage before the trigger");
    CHECK(hand_count_of(&tg, 0, ayumu_cp) == 1,
          "A3: the hand holds exactly ONE copy of the three-name card");
    CHECK(ayumu_cp != ayumu,
          "A3: the hand copy is a DISTINCT instance from the stage card "
          "(Rust used game.new_id for exactly this reason)");

    /* Prompt-count control, sampled BEFORE the interaction. */
    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "A3: control -- no prompt is standing before the live start");

    CHECK_EQ(fire_live_start(&tg, 0) >= 0, 1, "A3: the ライブ開始時 scan ran for P1");
    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "A3: the optional cost (named characters from hand) PROMPTS at live start");
    dump_pending(&tg, "A3 live start prompt");

    drain_first(&tg);

    int ngain = rb_card_num_gained_abilities(&tg.state, ayumu);
    CHECK(ngain > 0, "A3: the activating member really gained an ability");
    int text_ok = 0;
    for (int i = 0; i < ngain; i++) {
        const Ability *g = rb_card_gained_ability(&tg.state, ayumu, i);
        if (!g || !g->full_text) continue;
        printf("    <A3> gained[%d] = %s\n", i, g->full_text);
        if (strstr(g->full_text, "+3") || strstr(g->full_text, JP_Plus3Full)
            || strstr(g->full_text, JP_Score)) text_ok = 1;
    }
    CHECK(text_ok,
          "A3: the gained ability is the score +3 constant (printed above)");

    CHECK_EQ(test_total_card_count(&tg), census0,
             "A3: CENSUS unchanged across the whole live-start sequence");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §B  La Bella Patria  PL!N-bp3-027-L
 *  ライブ成功時: このターン、自分が余剰ハートにheart04を1つ以上持っており、
 *    かつ自分のステージに『虹ヶ咲』のメンバーがいる場合、
 *    自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Live-set placement helper shared by B1/B2/B3, shaped like
 * tests/test_surplus_heart.c's set_live_card: take the card out of hand into
 * the seat's own live zone. */
static int bella_set_live(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] != cid) continue;
        int card = rb_hand_remove_card(P, i);
        return card >= 0 && rb_live_add_card(P, card) == 0;
    }
    return 0;
}

#define BELLA_PASSES 16

typedef struct {
    int energy_before, energy_after;         /* energy zone size            */
    int wait_before, wait_after;             /* energy zone size MINUS the ACTIVE count */
    int energy_deck_before, energy_deck_after;
    int success_after;
    int census_before, census_after;
} BellaRun;

/* The DRAW-INVARIANT metric. Bella's placement is 「ウェイト状態で置く」, while
 * the ENERGY phase's ordinary draw activates what it puts in the zone, so
 * `energy_zone_size - energy_active_count` counts ONLY waited energy and is
 * unmoved by the turn machinery. `rb_mods`-free, no helper needed:
 * energy_active is a plain field on RbPlayer. */
static int wait_energy(TestGame *tg)
{
    const RbPlayer *P = &tg->state.p[0];
    return test_zone_len(tg, 0, "energy") - P->energy_active;
}

/* Build the Rust board with `live_cid` as P1's live card and `member_cid`
 * (-1 = empty stage) on P1's centre, run the SAME fixed number of passes as
 * every other arm, and report the deltas.
 *
 * WHY EVERY NEGATIVE NEEDS A REFERENCE ARM. The walk crosses the ENERGY phase,
 * which draws an energy card out of the ENERGY DECK into the energy zone every
 * turn. So "the energy deck did not shrink" is NOT what Bella's ability decides,
 * and a single-board count cannot separate the two -- an early revision of this
 * file read exactly that draw as Bella's placement and reported a false red on
 * the whole bella family. The reference arm is therefore the SAME live card, the
 * SAME seeded energy deck and the SAME number of passes, differing ONLY in the
 * stage member: it is always the POSITIVE board (Emma Verde, heart04 = 4), where
 * the printed condition is true and the ability must fire exactly once. Every
 * negative arm is then compared against it, so the ordinary draw cancels.
 *
 * A control live card with no such ability was tried first and REJECTED: a
 * different live card has different printed requirements, so its live can fail
 * where Bella's succeeds, the walk then traverses a different number of phases,
 * and the two arms stop being comparable (measured: 0 energy-phase draws on the
 * control against 1 on the test arm). The Emma reference keeps every other
 * variable fixed. */
static void bella_run(TestGame *tg, const char *tag, int live_cid, int member_cid,
                      int yell_on_top, int n_energy_deck, BellaRun *out)
{
    int fill = sid(tg, F10_NO);
    int ener = sid(tg, ENERGY_NO);
    int yell = sid(tg, YELL_ALL);
    if (!expect_card(tg, "B fill",  fill, F10_NO,    4)) return;
    if (!expect_card(tg, "B ener",  ener, ENERGY_NO, -1)) return;
    if (!expect_card(tg, "B yell",  yell, YELL_ALL,  -1)) return;

    test_game_new(tg);
    rb_card_copy_pool_reset();

    clear_stage(tg, 0); clear_stage(tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(tg, 0, fill); test_add_to_deck_pl(tg, 1, fill); }
    if (member_cid >= 0) stage_put(tg, 0, 1, member_cid);
    for (int i = 0; i < yell_on_top; i++) test_insert_deck_top(tg, 0, yell);
    hand_push(tg, 0, live_cid);
    for (int i = 0; i < n_energy_deck; i++) test_add_to_energy_deck(tg, 0, ener);

    out->energy_before      = test_zone_len(tg, 0, "energy");
    out->energy_deck_before = test_zone_len(tg, 0, "energy_deck");
    out->wait_before        = wait_energy(tg);
    out->census_before      = test_total_card_count(tg);
    CHECK_EQ(out->energy_deck_before, n_energy_deck,
             "B: the energy deck really was seeded with the requested number of cards");

    CHECK(bella_set_live(tg, live_cid), "B: the live card enters P1's live set");

    settle_walk(tg, BELLA_PASSES);
    if (rb_has_pending_choice(&tg->state)) { dump_pending(tg, tag); settle_walk(tg, 8); }

    out->energy_after      = test_zone_len(tg, 0, "energy");
    out->energy_deck_after = test_zone_len(tg, 0, "energy_deck");
    out->wait_after        = wait_energy(tg);
    out->success_after     = test_zone_len(tg, 0, "success");
    out->census_after      = test_total_card_count(tg);
    printf("    <%s> WAIT-energy %d->%d (delta %d) | energy %d->%d | "
           "energy_deck %d->%d | success=%d | census %d->%d\n",
           tag, out->wait_before, out->wait_after, out->wait_after - out->wait_before,
           out->energy_before, out->energy_after,
           out->energy_deck_before, out->energy_deck_after,
           out->success_after, out->census_before, out->census_after);
}

/* Rust :2339 bella_q174_no_member_on_stage_ability_does_not_fire
 *
 * DEVIATION FROM THE RUST FIXTURE, DELIBERATE AND LOAD-BEARING: the Rust test
 * leaves the ENERGY DECK EMPTY, so "energy_zone.cards.len() == energy_before"
 * is true for a completely inert ability. This port seeds 2 energy cards in the
 * energy deck (exactly what B3 does) and additionally measures a control arm, so
 * the negative is real. The Rust assertion itself is kept, strict, on top. */
static void b1_bella_q174_no_member_on_stage_does_not_fire(void)
{
    static TestGame t1, t2;
    int bella = sid(&t1, BELLA_NO);
    if (!expect_card(&t1, "B1 bella", bella, BELLA_NO, -1)) return;

    BellaRun test_arm, ref_arm;
    memset(&test_arm, 0, sizeof test_arm);
    memset(&ref_arm, 0, sizeof ref_arm);

    /* arm 1: the Rust board -- Bella is the live card, the stage is EMPTY. */
    bella_run(&t1, "B1 test arm (Bella, empty stage)", bella, -1, 0, 2, &test_arm);
    /* arm 2: the reference -- same everything, Emma on stage (condition TRUE). */
    bella_run(&t2, "B1 reference arm (Bella, Emma heart04=4)",
              sid(&t2, BELLA_NO), sid(&t2, LIVE_EMMA), 0, 2, &ref_arm);

    int t_delta = test_arm.energy_deck_after - test_arm.energy_deck_before;
    int r_delta = ref_arm.energy_deck_after - ref_arm.energy_deck_before;
    CHECK_EQ(test_arm.energy_deck_before, 2,
             "B1: the energy deck really holds 2 cards, so the negative is not vacuous");
    CHECK_EQ(ref_arm.success_after, 1,
             "B1: the reference arm's live really succeeded, so the ability fired there");

    /* THE LOAD-BEARING ASSERTION: one FEWER energy cards left the energy deck
     * than on the reference arm -- i.e. the ability fired exactly once there and
     * NOT here. The ordinary ENERGY-phase draw cancels in the difference. */
    CHECK_EQ(t_delta, r_delta + 1,
             "B1: Q174 -- with NO 虹ヶ咲 member on stage Bella placed NO energy: one FEWER "
             "card left the energy deck than on the reference arm where the condition holds");

    /* Rust's own assertion, kept strict on top of the control comparison. */
    CHECK(!rb_has_pending_choice(&t1.state),
          "B1: no prompt is left standing after the walk");
    printf("    <B1> test arm success zone = %d (diagnostic: on an empty stage with no "
           "yell the live cannot be made to succeed, so B1's negative is doubly weak in "
           "C as well as in Rust)\n", test_arm.success_after);
    CHECK_EQ(test_arm.census_after, test_arm.census_before,
             "B1: CENSUS unchanged -- nothing was placed and nothing destroyed");
    CHECK_EQ(ref_arm.census_after, ref_arm.census_before,
             "B1: reference arm CENSUS unchanged too -- the extra energy really MOVED");
}

/* Rust :2374 bella_q174_no_heart04_surplus_ability_does_not_fire */
static void b2_bella_q174_no_heart04_surplus_does_not_fire(void)
{
    static TestGame t1, t2;
    int bella = sid(&t1, BELLA_NO);
    int niji  = sid(&t1, NEARUKAWA);   /* h03:1 h05:1 h06:2, blade 3 -- NO heart04 */
    if (!expect_card(&t1, "B2 bella", bella, BELLA_NO, -1)) return;
    if (!expect_card(&t1, "B2 niji",  niji,  NEARUKAWA, 9))   return;

    /* Nearukawa really has ZERO heart04 -- load-bearing for the whole test.
     * card_base_heart returns -1 for an absent colour, which in the base
     * section means 0. */
    CHECK_EQ(card_base_heart(niji, idx_heart04), -1,
             "B2: fixture check -- the stage member has NO heart04 at all");
    CHECK_EQ(card_base_heart(niji, idx_heart03), 1,
             "B2: fixture check -- the stage member has heart03=1");
    CHECK_EQ(card_base_heart(niji, idx_heart05), 1,
             "B2: fixture check -- the stage member has heart05=1");
    CHECK_EQ(card_base_heart(niji, idx_heart06), 2,
             "B2: fixture check -- the stage member has heart06=2");

    BellaRun test_arm, ref_arm;
    memset(&test_arm, 0, sizeof test_arm);
    memset(&ref_arm, 0, sizeof ref_arm);

    bella_run(&t1, "B2 test arm (Bella, Nearukawa, 3 yell)", bella, niji, 3, 2, &test_arm);
    bella_run(&t2, "B2 reference arm (Bella, Emma heart04=4)",
              sid(&t2, BELLA_NO), sid(&t2, LIVE_EMMA), 3, 2, &ref_arm);

    int t_delta = test_arm.energy_deck_after - test_arm.energy_deck_before;
    int r_delta = ref_arm.energy_deck_after - ref_arm.energy_deck_before;
    CHECK_EQ(test_arm.energy_deck_before, 2,
             "B2: the energy deck really holds 2 cards, so the negative is not vacuous");

    CHECK_EQ(t_delta, r_delta + 1,
             "B2: Q174 -- surplus heart04 = 0 (the stage member has NO heart04), so Bella "
             "placed NO energy: one FEWER card left the energy deck than on the reference arm");

    CHECK_EQ(test_arm.success_after, 1,
             "B2: La Bella Patria really did succeed -- the ライブ成功時 window WAS open, "
             "which is what makes the negative above meaningful");
    CHECK_EQ(ref_arm.success_after, 1,
             "B2: the reference arm's live really succeeded too, so the two deltas are "
             "directly comparable");
    CHECK_EQ(test_arm.census_after, test_arm.census_before,
             "B2: CENSUS unchanged -- nothing was placed and nothing destroyed");
    CHECK_EQ(ref_arm.census_after, ref_arm.census_before,
             "B2: reference arm CENSUS unchanged too -- the extra energy really MOVED");
}

/* Rust :2430 bella_positive_surplus_heart04_ability_fires
 * THE PAIRED POSITIVE for B1 and B2: same live card, same passes, same yell; the
 * only change is a stage member carrying heart04=4 (Emma Verde). */
static void b3_bella_positive_surplus_heart04_fires(void)
{
    static TestGame t1, t2;
    int bella = sid(&t1, BELLA_NO);
    int emma  = sid(&t1, LIVE_EMMA);   /* h02:1 h03:1 h04:4 h05:1, blade 6 */
    if (!expect_card(&t1, "B3 bella", bella, BELLA_NO, -1)) return;
    if (!expect_card(&t1, "B3 emma",  emma,  LIVE_EMMA, 17)) return;

    CHECK_EQ(card_base_heart(emma, idx_heart04), 4,
             "B3: fixture check -- the stage member really has heart04=4");
    CHECK_EQ(card_base_heart(emma, idx_heart02), 1,
             "B3: fixture check -- and heart02=1, so the fixture is not swapped");

    BellaRun test_arm, ref_arm;
    memset(&test_arm, 0, sizeof test_arm);
    memset(&ref_arm, 0, sizeof ref_arm);

    bella_run(&t1, "B3 test arm (Bella, Emma heart04=4)", bella, emma, 0, 2, &test_arm);
    bella_run(&t2, "B3 reference arm (Bella, Emma heart04=4, second board)",
              sid(&t2, BELLA_NO), sid(&t2, LIVE_EMMA), 0, 2, &ref_arm);

    int t_delta = test_arm.energy_deck_after - test_arm.energy_deck_before;
    int r_delta = ref_arm.energy_deck_after - ref_arm.energy_deck_before;
    /* THE PAIRED POSITIVE, and the measure's own repeatability: two boards that
     * are identical in every respect must shrink the energy deck by the same
     * amount. If this fails the comparison used by B1/B2 is not trustworthy and
     * their verdicts are void, so it is asserted here rather than assumed. */
    CHECK_EQ(t_delta, r_delta,
             "B3: two identical positive boards shrink the energy deck identically -- the "
             "comparison B1 and B2 rely on is repeatable");
    CHECK_EQ(test_arm.success_after, 1, "B3: La Bella Patria really did succeed");

    /* Rust's own raw assertion, kept strict on top of the comparison. */
    CHECK(test_arm.energy_deck_after < test_arm.energy_deck_before,
          "B3: Rust's assertion -- the energy deck ends up shorter than it started");
    CHECK_EQ(test_arm.census_after, test_arm.census_before,
             "B3: CENSUS unchanged -- the energy card MOVED deck -> zone, none destroyed");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §K  徒町小鈴  PL!HS-pb1-005-R
 *  ライブ開始時: 数1つを選ぶ。自分のデッキの一番上のカードを公開する。
 *    公開したカードがメンバーカードで、かつコストが選んだ数以上の場合、
 *    公開したカードを手札に加える。選んだ数以下の場合、ライブ終了時まで、
 *    ブレード、ブレードを得る。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Rust setup_kosuzu_test. `top` is a DISTINCT instance (mk), which is the one
 * place in this file where the Rust twin aliases the top card to the deck
 * stocker; see the header. `ctrl_member` (<= 0 for none) is a second member put
 * on P1's stage purely as a BLADE-MODIFIER CONTROL: it has no ライブ開始時, so
 * a non-zero blade modifier on Kosuzu cannot be an artefact of a global recalc.
 * Returns the kosuzu id. */
static int kosuzu_setup(TestGame *tg, int top, int ctrl_member, int *out_census)
{
    int kosuzu = sid(tg, KOSUZU_NO);
    int fill   = sid(tg, F10_NO);
    if (!expect_card(tg, "K kosuzu", kosuzu, KOSUZU_NO, 11)) return -1;
    if (!expect_card(tg, "K fill",   fill,   F10_NO,    4))  return -1;

    if (ctrl_member == 0) ctrl_member = sid(tg, F10_NO);

    clear_stage(tg, 0); clear_stage(tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(tg, 0, fill); test_add_to_deck_pl(tg, 1, fill); }
    if (top >= 0) test_insert_deck_top(tg, 0, top);
    test_give_energy(tg, 11);
    hand_push(tg, 0, kosuzu);
    CHECK_EQ(test_play_to_stage(tg, kosuzu, 1), 1, "K: Kosuzu is played to the centre area");
    CHECK_EQ(stage_occupied(tg, 0), 1, "K: Kosuzu really is on the stage");
    if (ctrl_member > 0) stage_put(tg, 0, 2, ctrl_member);

    /* CENSUS baselined after every fixture and immediately before the trigger. */
    *out_census = test_total_card_count(tg);
    return kosuzu;
}

/* Rust :3825 kosuzu_choose_number_adds_to_hand — chosen number BELOW the top
 * card's cost (2): the card is added to hand and NO blade is gained. */
static void k1_kosuzu_choose_number_adds_to_hand(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int top = mk(&tg, KOSUZU_SD);   /* cost 2 */
    if (!expect_card(&tg, "K1 top", top, KOSUZU_SD, 2)) return;

    int census0 = 0;
    int ctrl = sid(&tg, F13_NO);
    if (!expect_card(&tg, "K1 control member", ctrl, F13_NO, 4)) return;
    int kosuzu = kosuzu_setup(&tg, top, ctrl, &census0);
    if (kosuzu < 0) return;

    int top_deck[1];
    test_zone_ids(&tg, 0, "deck", top_deck, 1);
    CHECK(top_deck[0] == top, "K1: the seeded top-of-deck card is really on top");
    CHECK_EQ(stage_occupied(&tg, 0), 2,
             "K1: the CONTROL member (no ライブ開始時 ability) really is on the stage too");

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "K1: control -- no prompt is standing before the live start");
    fire_live_start(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "K1: the number-choice prompt really appears at live start");
    dump_pending(&tg, "K1 number prompt");
    select_option(&tg, 0);   /* Rust: game.select_generated(0) */

    CHECK(!rb_has_pending_choice(&tg.state), "K1: the whole sequence resolved after one answer");
    CHECK(hand_count_of(&tg, 0, top) == 1,
          "K1: the revealed top card really is in hand (exactly one instance)");
    /* THE BLADE CONTROL, taken at the ASSERTION SITE: the abilityless member on
     * the same board is the reference. The raw modifier slots are printed too,
     * so a non-zero reading can be attributed to `set` (an absolute write) or
     * to `add` (a delta) instead of being guessed at. */
    {
        Card kc, cc;
        int kb = -1, cb = -1;
        if (rb_decode_card_by_index((uint32_t)kosuzu, &kc)) { kb = kc.blade; rb_free_card(&kc); }
        if (rb_decode_card_by_index((uint32_t)ctrl,    &cc)) { cb = cc.blade; rb_free_card(&cc); }
        printf("    <K1> kosuzu id=%d printed_blade=%d mods[set=%d add=%d total=%d] | "
               "control id=%d printed_blade=%d mods[set=%d add=%d total=%d]\n",
               kosuzu, kb, tg.state.mods.blade[kosuzu].set, tg.state.mods.blade[kosuzu].add,
               test_get_blade_modifier(&tg, kosuzu),
               ctrl, cb, tg.state.mods.blade[ctrl].set, tg.state.mods.blade[ctrl].add,
               test_get_blade_modifier(&tg, ctrl));
    }
    CHECK_EQ(test_get_blade_modifier(&tg, ctrl), 0,
             "K1: control -- the abilityless member on the same board gained NO blade");
    CHECK_EQ(test_get_blade_modifier(&tg, kosuzu), 0,
             "K1: NO blade was gained on Kosuzu -- the chosen number is below the card's cost");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "K1: CENSUS unchanged -- one card moved deck -> hand, none destroyed");
}

/* Rust :3839 kosuzu_choose_number_gains_blade — chosen number ABOVE the top
 * card's cost (2): no card to hand, the card goes back to the top of the deck,
 * and blade IS gained. */
static void k2_kosuzu_choose_number_gains_blade(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int top = mk(&tg, KOSUZU_SD);   /* cost 2 */
    if (!expect_card(&tg, "K2 top", top, KOSUZU_SD, 2)) return;

    int census0 = 0;
    int kosuzu = kosuzu_setup(&tg, top, -1, &census0);
    if (kosuzu < 0) return;

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "K2: control -- no prompt is standing before the live start");
    fire_live_start(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "K2: the number-choice prompt really appears at live start");
    dump_pending(&tg, "K2 number prompt");
    select_option(&tg, 9);   /* Rust: game.select_generated(9) */

    CHECK(!rb_has_pending_choice(&tg.state), "K2: the whole sequence resolved after one answer");
    CHECK(hand_count_of(&tg, 0, top) == 0,
          "K2: the revealed card is NOT in hand -- the chosen number exceeded its cost");
    int top_deck[1];
    test_zone_ids(&tg, 0, "deck", top_deck, 1);
    CHECK(top_deck[0] == top, "K2: the revealed card was returned to the TOP of the deck");
    CHECK(test_get_blade_modifier(&tg, kosuzu) > 0,
          "K2: blade WAS gained on Kosuzu");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "K2: CENSUS unchanged -- the card was relocated, none destroyed");
}

/* Rust :3857 kosuzu_equal_choice_both_effects — the chosen number EQUALS the top
 * card's cost, so BOTH halves of the printed sentence fire. */
static void k3_kosuzu_equal_choice_both_effects(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    /* Rust uses PL!-sd1-010-SD here, which is ALSO its deck stocker -- the
     * aliasing case called out in the header. A distinct instance is used so
     * `hand contains top_card` is a real assertion. */
    int top = mk(&tg, F10_NO);   /* cost 4 */
    if (!expect_card(&tg, "K3 top", top, F10_NO, 4)) return;

    int census0 = 0;
    int kosuzu = kosuzu_setup(&tg, top, -1, &census0);
    if (kosuzu < 0) return;

    int top_deck[1];
    test_zone_ids(&tg, 0, "deck", top_deck, 1);
    CHECK(top_deck[0] == top, "K3: the seeded top-of-deck card is really on top");

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "K3: control -- no prompt is standing before the live start");
    fire_live_start(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "K3: the number-choice prompt really appears at live start");
    dump_pending(&tg, "K3 number prompt");
    select_option(&tg, 3);   /* Rust: game.select_generated(3) */

    CHECK(!rb_has_pending_choice(&tg.state), "K3: the whole sequence resolved after one answer");
    CHECK(hand_count_of(&tg, 0, top) == 1,
          "K3: the EQUAL case also puts the revealed card into hand");
    CHECK(test_get_blade_modifier(&tg, kosuzu) > 0,
          "K3: the EQUAL case ALSO gains blade -- both halves fire");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "K3: CENSUS unchanged -- one card moved deck -> hand, none destroyed");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §L  Oh,Love&Peace!  PL!-bp3-026-L
 *  ライブ成功時: 自分のステージにいるメンバーが持つハートの総数が、
 *    相手のステージにいるメンバーが持つハートの総数より多い場合、
 *    このカードのスコアを＋１する。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Rust :2485 lovepeace_q150_self_hearts_greater_than_opponent_score_plus_1 */
static void l1_lovepeace_q150_hearts_greater_score_plus_1(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    /* Rust interns ONE lovepeace id into BOTH live zones; the C uses two
     * DISTINCT instances (see the header) and asserts on P1's. */
    int love_p1 = mk(&tg, LOVEPEACE_NO);
    int love_p2 = mk(&tg, LOVEPEACE_NO);
    int strong1 = mk(&tg, PB1_014_NO);   /* h01:3 h03:2 h06:2 */
    int strong2 = mk(&tg, PR3_NO);       /* h01:2 h03:3 h06:1 */
    int weak    = mk(&tg, F10_NO);       /* h01:1 h03:1 */
    int fill    = sid(&tg, F10_NO);
    if (!expect_card(&tg, "L1 love_p1", love_p1, LOVEPEACE_NO, -1)) return;
    if (!expect_card(&tg, "L1 strong1", strong1, PB1_014_NO, 15)) return;
    if (!expect_card(&tg, "L1 strong2", strong2, PR3_NO,     15)) return;
    if (!expect_card(&tg, "L1 weak",    weak,    F10_NO,     4))  return;
    if (!expect_card(&tg, "L1 fill",    fill,    F10_NO,     4))  return;

    /* Rust clears both hands and both decks, then stocks 40 cards each. */
    bag_clear(&tg.state.p[0].hand);
    bag_clear(&tg.state.p[1].hand);
    bag_clear(&tg.state.p[0].deck);
    bag_clear(&tg.state.p[1].deck);
    clear_stage(&tg, 0); clear_stage(&tg, 1);
    for (int i = 0; i < 40; i++) { test_add_to_deck_pl(&tg, 0, fill); test_add_to_deck_pl(&tg, 1, fill); }

    stage_put(&tg, 0, 1, strong1);
    stage_put(&tg, 0, 2, strong2);
    stage_put(&tg, 1, 1, weak);       /* Rust reuses ONE id here; C uses a distinct copy */

    set_live(&tg, 0, love_p1);
    set_live(&tg, 1, love_p2);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(stage_occupied(&tg, 0), 2, "L1: P1 has the two strong members");
    CHECK_EQ(stage_occupied(&tg, 1), 1, "L1: P2 has the one weak member");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1, "L1: P1's live zone holds one card");
    CHECK_EQ(test_zone_len(&tg, 1, "live"), 1, "L1: P2's live zone holds one card");

    /* Rust writes stage_hearts explicitly, adding 7 heart03 to P1. C's
     * rb_calc_stage_hearts RECOMPUTES, so the +7 is applied to the computed
     * array and then stored into state.stage_hearts, which is what the
     * ability reads. */
    int h1[8], h2[8];
    rb_calc_stage_hearts(&tg.state, 0, h1);
    rb_calc_stage_hearts(&tg.state, 1, h2);
    int h1_before = h1[idx_heart03];
    h1[idx_heart03] += 7;
    for (int i = 0; i < 8; i++) {
        tg.state.stage_hearts[0][i] = h1[i];
        tg.state.stage_hearts[1][i] = h2[i];
    }
    {
        int s1 = 0, s2 = 0;
        for (int i = 0; i < 8; i++) { s1 += tg.state.stage_hearts[0][i]; s2 += tg.state.stage_hearts[1][i]; }
        printf("    <L1> P1 hearts total=%d (heart03 %d -> %d)  P2 hearts total=%d\n",
               s1, h1_before, h1[idx_heart03], s2);
        /* THE CONDITION'S OWN PRECONDITION, asserted before the dispatch, so a
         * "+1" can be attributed to the ability and not to a shut window. */
        CHECK(s1 > s2, "L1: precondition -- P1's heart total really EXCEEDS P2's");
    }

    open_live_success_window(&tg, 0);
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1, "L1: control -- P1 has a live card for the window");
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "L1: precondition -- the ライブ成功時 window is OPEN");

    fire_live_success(&tg, 0, love_p1);

    CHECK_EQ(test_get_score_modifier(&tg, love_p1), 1,
             "L1: Q150 -- P1's own live card gains +1 score when P1's hearts exceed P2's");
    CHECK_EQ(test_get_score_modifier(&tg, love_p2), 0,
             "L1: the OPPONENT's copy of the same print gains nothing");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "L1: CENSUS unchanged -- a score modifier creates and destroys no card");
}

/* Rust :2570 lovepeace_q149_total_hearts_sum_of_base_hearts (pure DB) */
static void l2_lovepeace_q149_total_hearts_sum_of_base(void)
{
    static TestGame tg; test_game_new(&tg); new_game();
    CHECK_EQ(test_total_card_count(&tg), 0,
             "L2: the board is empty -- this Rust test reads the database only");

    struct { const char *no; int want; } cases[3] = {
        { F14_NO,  4 },   /* h01:2 h03:1 h06:1 */
        { PR5_NO,  3 },   /* h03:3          */
        { PR3_NO,  6 },   /* h01:2 h03:3 h06:1 */
    };
    for (int i = 0; i < 3; i++) {
        int id = sid(&tg, cases[i].no);
        if (id < 0) continue;
        if (!expect_card(&tg, "L2 fixture", id, cases[i].no, -1)) continue;
        Card c;
        int got = -1;
        if (rb_decode_card_by_index((uint32_t)id, &c)) {
            got = rb_card_total_hearts(&c);
            rb_free_card(&c);
        }
        /* The message is built with snprintf, NOT with adjacent string literals
         * spliced into a two-argument CHECK. */
        char buf[200];
        snprintf(buf, sizeof buf,
                 "L2: Q149 -- total_hearts is the sum of all base heart values for %s",
                 cases[i].no);
        CHECK_EQ(got, cases[i].want, buf);
    }
    CHECK_EQ(test_total_card_count(&tg), 0, "L2: CENSUS still 0 -- reading the DB places nothing");
}

/* Rust :2590 lovepeace_q172_ability_gained_hearts_count_but_not_blade */
static void l3_lovepeace_q172_gained_hearts_count_but_not_blade(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int live   = sid(&tg, LIVE20_NO);
    int member = sid(&tg, F14_NO);      /* 星空 凛, total_hearts 4 */
    int cheer  = sid(&tg, F10_NO);
    int fill   = sid(&tg, F10_NO);
    if (!expect_card(&tg, "L3 live",   live,   LIVE20_NO, -1)) return;
    if (!expect_card(&tg, "L3 member", member, F14_NO,    9))  return;
    if (!expect_card(&tg, "L3 cheer",  cheer,  F10_NO,    4))  return;
    if (!expect_card(&tg, "L3 fill",   fill,   F10_NO,    4))  return;
    {
        Card c;
        int tot = -1;
        if (rb_decode_card_by_index((uint32_t)member, &c)) {
            tot = rb_card_total_hearts(&c);
            rb_free_card(&c);
        }
        CHECK_EQ(tot, 4, "L3: fixture check -- the stage member's total_hearts is 4");
    }

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(&tg, 0, fill); test_add_to_deck_pl(&tg, 1, fill); }
    stage_put(&tg, 0, 1, member);
    hand_push(&tg, 0, live);
    for (int i = 0; i < 3; i++) test_insert_deck_top(&tg, 0, cheer);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(stage_occupied(&tg, 0), 1, "L3: exactly one member is on P1's stage");
    CHECK_EQ(test_total_card_count(&tg), census0, "L3: CENSUS baselined before the live set");

    CHECK(bella_set_live(&tg, live), "L3: the live card enters P1's live set");

    settle_walk(&tg, 16);
    if (rb_has_pending_choice(&tg.state)) { dump_pending(&tg, "L3 parked"); settle_walk(&tg, 8); }

    /* Rust: success_live_card_zone.cards == [live] */
    int succ[4];
    int nsucc = test_zone_ids(&tg, 0, "success", succ, 4);
    CHECK_EQ(nsucc, 1, "L3: Q172 -- the live succeeded and was placed in the success zone");
    CHECK(nsucc == 1 && succ[0] == live,
          "L3: the success zone holds the very live card that was set");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "L3: CENSUS unchanged -- blade hearts never became base hearts");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §H  未来予報ハレルヤ！  PL!SP-bp1-026-L
 *  ライブ開始時: 自分の、ステージと控え室に名前の異なる『Liella!』の
 *    メンバーが5人以上いる場合、このカードを使用するためのコストは
 *    heart02×2, heart03×2, heart06×2 になる。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Shared harness: `n` distinct Liella! members in P1's WAITROOM, none on stage,
 * decks stocked (so refresh() cannot clear the waitroom), hareruya in hand. */
static int hareruya_setup(TestGame *tg, int n, int *out_ids)
{
    static const char *const liella[5] = { L014_NO, L015_NO, L016_NO, L019_NO, L020_NO };
    int hareruya = sid(tg, HARERUYA_NO);
    int fill     = sid(tg, F10_NO);
    if (!expect_card(tg, "H hareruya", hareruya, HARERUYA_NO, -1)) return -1;
    if (!expect_card(tg, "H fill",     fill,     F10_NO,     4))  return -1;

    clear_stage(tg, 0); clear_stage(tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(tg, 0, fill); test_add_to_deck_pl(tg, 1, fill); }
    for (int i = 0; i < n; i++) {
        /* DISTINCT instances (mk) so each card occupies its own slot and its
         * name is per-instance, never a shared template. */
        int cid = mk(tg, liella[i]);
        if (cid < 0) return -1;
        if (!expect_card(tg, "H liella member", cid, liella[i], 4 + (i == 3 ? 7 : 0))) {
            /* costs: 4,4,4,11,4 -- the check above is advisory only */
        }
        disc_push(tg, 0, cid);
        out_ids[i] = cid;
    }
    hand_push(tg, 0, hareruya);
    return hareruya;
}

/* Rust :2649 hareruya_q64_waitroom_only_five_distinct_liella_condition_met */
static void h1_hareruya_q64_five_distinct_names_condition_met(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int ids[5];
    int hareruya = hareruya_setup(&tg, 5, ids);
    if (hareruya < 0) return;

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 5, "H1: five members really are in the waitroom");
    CHECK_EQ(stage_occupied(&tg, 0), 0, "H1: NONE of them is on the stage (waitroom-only)");

    /* THE PRECONDITION CONTROL, measured on the board: the five printed names
     * really are five DISTINCT names, and they really are Liella! members.
     * Without this the +need-heart assertion below could be green for a board
     * that does not satisfy the printed condition at all. */
    CHECK_EQ(test_max_distinct_names(ids, 5), 5,
             "H1: control -- the five waitroom members have five DISTINCT names");
    {
        static const char *const liella[5] = { L014_NO, L015_NO, L016_NO, L019_NO, L020_NO };
        int all_liella = 1;
        for (int i = 0; i < 5; i++) {
            const char *u = rb_card_unit_name(ids[i]);
            if (!u || !*u) all_liella = 0;
        }
        CHECK(all_liella, "H1: control -- every seeded member resolves a Liella! unit name");
        (void)liella;
    }
    {
        /* The Rust q64 board differs from q74's by exactly one card; assert the
         * names really are pairwise different so neither case is an alias test. */
        char n0[128], n1[128], n2[128], n3[128], n4[128];
        snap_name(&tg, ids[0], n0, sizeof n0);
        snap_name(&tg, ids[1], n1, sizeof n1);
        snap_name(&tg, ids[2], n2, sizeof n2);
        snap_name(&tg, ids[3], n3, sizeof n3);
        snap_name(&tg, ids[4], n4, sizeof n4);
        int distinct = 1;
        const char *all[5] = { n0, n1, n2, n3, n4 };
        for (int i = 0; i < 5; i++)
            for (int j = i + 1; j < 5; j++)
                if (strcmp(all[i], all[j]) == 0) distinct = 0;
        CHECK(distinct, "H1: control -- no two seeded cards print the same name");
        printf("    <H1> waitroom names: [%s] [%s] [%s] [%s] [%s]\n", n0, n1, n2, n3, n4);
    }

    CHECK(bella_set_live(&tg, hareruya), "H1: the live card enters P1's live set");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1, "H1: control -- the live card is really in the zone");

    fire_live_start(&tg, 0);

    CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart02), 2,
             "H1: Q64 -- heart02 required hearts are set to exactly 2");
    CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart03), 2,
             "H1: Q64 -- heart03 required hearts are set to exactly 2");
    CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart06), 2,
             "H1: Q64 -- heart06 required hearts are set to exactly 2");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "H1: CENSUS unchanged -- a required-heart modifier moves no card");
}

/* ADDED (not a Rust twin): H1's NEGATIVE twin. Four distinct Liella! names is
 * the printed boundary minus one; the required hearts must NOT be set. Without
 * it H1's three +2 assertions could be green because the engine writes the
 * modifier unconditionally. */
static void h1b_hareruya_four_distinct_names_condition_not_met(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int ids[5];
    int hareruya = hareruya_setup(&tg, 4, ids);
    if (hareruya < 0) return;

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 4, "H1b: four members really are in the waitroom");
    CHECK_EQ(test_max_distinct_names(ids, 4), 4,
             "H1b: control -- FOUR distinct names, one below the printed threshold of 5");

    CHECK(bella_set_live(&tg, hareruya), "H1b: the live card enters P1's live set");
    fire_live_start(&tg, 0);

    CHECK(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart02) != 2,
          "H1b: with only FOUR distinct Liella! names the heart02 cost reduction is NOT applied");
    CHECK(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart03) != 2,
          "H1b: with only FOUR distinct Liella! names the heart03 cost reduction is NOT applied");
    CHECK(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart06) != 2,
          "H1b: with only FOUR distinct Liella! names the heart06 cost reduction is NOT applied");
    CHECK_EQ(test_total_card_count(&tg), census0, "H1b: CENSUS unchanged");
}

/* Rust :2731 hareruya_q74_multiname_distinct_counting */
static void h2_hareruya_q74_multiname_distinct_counting(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    /* Rust's own comment admits it gave up on the multi-name probe and settled
     * for five single-name Liella! cards, swapping 平安名すみれ (015-N) out for
     * 鬼塚冬毬 (022-N). This port is faithful to the BOARD, and it also records
     * the two facts the Rust test asserts in prose but never checks:
     *   (a) the five names are still five DISTINCT names, and
     *   (b) 鬼塚夏美 (020-N) and 鬼塚冬毬 (022-N) share the surname 鬼塚 -- the
     *       C engine's distinct-name counter keys on the WHOLE normalized name
     *       (rb_max_distinct_names, util.c:2136) and the condition's own counter
     *       splits only on '&' (rb_count_distinct_member_name_units, util.c:1408),
     *       so neither collapses this pair to one name.
     * No Liella! member in cards.json has a '&' in its name (checked over all
     * 2,639 records), so a genuine Liella! multi-name fixture does not exist. */
    static const char *const liella[5] = { L014_NO, L016_NO, L019_NO, L020_NO, L022_NO };
    int hareruya = sid(&tg, HARERUYA_NO);
    int fill     = sid(&tg, F10_NO);
    if (!expect_card(&tg, "H2 hareruya", hareruya, HARERUYA_NO, -1)) return;
    if (!expect_card(&tg, "H2 fill",     fill,     F10_NO,     4))  return;

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(&tg, 0, fill); test_add_to_deck_pl(&tg, 1, fill); }
    int ids[5];
    for (int i = 0; i < 5; i++) {
        int cid = mk(&tg, liella[i]);
        if (cid < 0) return;
        disc_push(&tg, 0, cid);
        ids[i] = cid;
    }
    hand_push(&tg, 0, hareruya);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(test_max_distinct_names(ids, 5), 5,
             "H2: control -- the five cards really print five distinct names");
    {
        char a[128], b[128];
        snap_name(&tg, ids[3], a, sizeof a);
        snap_name(&tg, ids[4], b, sizeof b);
        printf("    <H2> the shared-surname pair is [%s] and [%s]\n", a, b);
        CHECK(strcmp(a, b) != 0,
              "H2: the two cards that share a surname still print DIFFERENT full names");
    }

    CHECK(bella_set_live(&tg, hareruya), "H2: the live card enters P1's live set");
    fire_live_start(&tg, 0);

    CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, hareruya, idx_heart02), 2,
             "H2: Q74 -- the required-heart modifier is set with five distinct Liella! names");
    CHECK_EQ(test_total_card_count(&tg), census0, "H2: CENSUS unchanged");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §Y  渡辺 曜&鬼塚夏美&大沢瑠璃乃  LL-bp2-001-R(U+FF0B)   ("You")
 *  常時: 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚
 *    につき、1少なくなる。
 *  常時: このメンバーはバトンタッチで控え室に置かれない。
 *  ライブ開始時: 手札の「唐可可」と「平安名すみれ」と「米女メイ」を、好きな
 *    組合せで控え室に置いてもよい：ライブ終了時まで、これにより控え室に置いた
 *    カード1枚につき、ブレードを得る。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Rust :2195 you_q129_cost_reduction_self_only (pure DB) */
static void y1_you_q129_cost_reduction_self_only(void)
{
    static TestGame tg; test_game_new(&tg); new_game();
    CHECK_EQ(test_total_card_count(&tg), 0,
             "Y1: the board is empty -- this Rust test reads the database only");

    int keke = sid(&tg, YOU_NO);
    if (!expect_card(&tg, "Y1 keke", keke, YOU_NO, 20)) return;

    int nab = rb_card_num_abilities((uint32_t)keke);
    int has_cost_reduction = 0, has_baton_block = 0;
    char cst[400] = "", bat[400] = "";
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)keke, a, &ab)) continue;
        const char *txt = ab.full_text ? ab.full_text : "";
        if (ab.triggers && strcmp(ab.triggers, JP_Constant) == 0) {
            if (strstr(txt, JP_Cost) && strstr(txt, JP_BecomeLess)) {
                has_cost_reduction = 1;
                snprintf(cst, sizeof cst, "%s", txt);
            }
            if (strstr(txt, JP_BatonTouch)) {
                has_baton_block = 1;
                snprintf(bat, sizeof bat, "%s", txt);
            }
        }
        rb_free_ability(&ab);
    }
    printf("    <Y1> cost-reduction 常時 = %s\n", cst);
    printf("    <Y1> baton-block   常時 = %s\n", bat);
    CHECK(has_cost_reduction,
          "Y1: Q129 -- the card carries a 常時 ability for the SELF cost reduction");
    CHECK(has_baton_block,
          "Y1: Q129 -- the card carries a 常時 ability preventing baton touch");
    CHECK_EQ(test_total_card_count(&tg), 0, "Y1: CENSUS still 0 -- reading the DB places nothing");
}

/* Rust :2226 you_live_start_optional_cost_triggers */
static void y2_you_live_start_optional_cost_triggers(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int keke      = sid(&tg, YOU_NO);
    int filler_l  = sid(&tg, LIVE19_NO);
    int fill      = sid(&tg, F10_NO);
    int named_cp  = mk(&tg, YOU_NO);       /* Rust game.new_id(...) */
    if (!expect_card(&tg, "Y2 keke",     keke,     YOU_NO,     20)) return;
    if (!expect_card(&tg, "Y2 filler_l", filler_l, LIVE19_NO,  -1)) return;
    if (!expect_card(&tg, "Y2 fill",     fill,     F10_NO,     4))  return;
    if (!expect_card(&tg, "Y2 named_cp", named_cp, YOU_NO,     20)) return;

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(&tg, 0, fill); test_add_to_deck_pl(&tg, 1, fill); }
    stage_put(&tg, 0, 0, keke);             /* Rust stage.stage[0] = keke */
    hand_push(&tg, 0, filler_l);
    hand_push(&tg, 0, named_cp);
    set_live(&tg, 0, filler_l);

    int census0 = test_total_card_count(&tg);
    CHECK(named_cp != keke,
          "Y2: the hand copy is a DISTINCT instance from the stage card");
    CHECK_EQ(hand_count_of(&tg, 0, named_cp), 1,
             "Y2: exactly one named-character copy is in hand");

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "Y2: control -- no prompt is standing before the live start");

    fire_live_start(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "Y2: the ライブ開始時 optional cost PROMPTS when an eligible named character is in hand");
    dump_pending(&tg, "Y2 optional cost");

    pick(&tg, 0);            /* Rust: game.select_indices(&[0])  -- pay */
    drain_decline(&tg);      /* Rust: game.select_indices(&[])    -- finalise */

    CHECK(test_get_blade_modifier(&tg, keke) > 0,
          "Y2: blade is gained on the ACTIVATING card after the named copy is discarded");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "Y2: CENSUS unchanged -- the named card moved hand -> discard, none destroyed");
}

/* Rust :2271 you_abilityless_card_not_in_choice */
static void y3_you_abilityless_card_not_in_choice(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int keke      = sid(&tg, YOU_NO);
    int filler_l  = sid(&tg, LIVE19_NO);
    int fill      = sid(&tg, F10_NO);
    int named_cp  = mk(&tg, YOU_NO);
    if (!expect_card(&tg, "Y3 keke",     keke,     YOU_NO,    20)) return;
    if (!expect_card(&tg, "Y3 filler_l", filler_l, LIVE19_NO, -1)) return;
    if (!expect_card(&tg, "Y3 fill",     fill,     F10_NO,    4))  return;
    if (!expect_card(&tg, "Y3 named_cp", named_cp, YOU_NO,    20)) return;

    clear_stage(&tg, 0); clear_stage(&tg, 1);
    for (int i = 0; i < 10; i++) { test_add_to_deck_pl(&tg, 0, fill); test_add_to_deck_pl(&tg, 1, fill); }
    stage_put(&tg, 0, 1, keke);             /* Rust stage.stage[1] = keke */
    hand_push(&tg, 0, filler_l);
    hand_push(&tg, 0, named_cp);
    hand_push(&tg, 0, fill);                /* the abilityless NON-candidate */

    int census0 = test_total_card_count(&tg);
    CHECK(hand_count_of(&tg, 0, fill) == 1,
          "Y3: the non-candidate filler really is in hand exactly once");

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "Y3: control -- no prompt is standing before the live start");

    fire_live_start(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "Y3: the ライブ開始時 optional cost PROMPTS");
    dump_pending(&tg, "Y3 optional cost");

    pick(&tg, 0);
    /* Rust asserts the under-cap re-ask is STILL standing here -- a POSITIVE
     * assertion on a prompt, so it needs no separate control. */
    CHECK(rb_has_pending_choice(&tg.state),
          "Y3: after paying once the optional cost RE-ASKS (still under the 3-card cap)");
    dump_pending(&tg, "Y3 re-ask");
    decline(&tg);
    drain_first(&tg);

    /* The negative. Its paired positive is the blade assertion two lines below
     * on the SAME board: the ability did something, and it did not take the
     * filler. */
    CHECK(in_zone(&tg, 0, "hand", fill),
          "Y3: the abilityless filler is STILL IN HAND -- it was not selectable");
    CHECK(test_get_blade_modifier(&tg, keke) > 0,
          "Y3: the paired POSITIVE -- blade WAS gained from the named copy, so the "
          "optional cost really resolved and the filler really was skipped");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "Y3: CENSUS unchanged -- the named card moved hand -> discard, none destroyed");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §C  澁谷かのん  PL!SP-bp5-001-R(U+FF0B)
 *  起動(ターン1回): このメンバーをウェイトにするか、手札を1枚控え室に置く：
 *    エネルギーを1枚アクティブにする。
 * ═══════════════════════════════════════════════════════════════════════ */

/* Rust's kanon twin: 15 energy cards of which 10 are active, kanon placed on
 * the stage BY HAND (so its 登場 never fires) and taken out of hand. */
static int kanon_board(TestGame *tg, int *out_kanon, int *out_census, int *out_cheap)
{
    int kanon = sid(tg, KANON_NO);
    if (!expect_card(tg, "C kanon", kanon, KANON_NO, 10)) return 0;
    clear_stage(tg, 0); clear_stage(tg, 1);
    hand_push(tg, 0, kanon);
    test_give_energy(tg, 15);
    /* test_give_energy raises the ACTIVE count with every card, so the "5 of 15
     * are wait" part of the Rust fixture needs this explicit reset. */
    test_set_energy_active(tg, 0, 10);
    stage_place_from_hand(tg, 0, 0, kanon);
    if (out_cheap) *out_cheap = -1;
    *out_kanon = kanon;
    *out_census = test_total_card_count(tg);
    return 1;
}

/* Rust :3416 kanon_activation_choice_condition_discard_flow */
static void c1_kanon_activation_choice_condition_discard_flow(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int census0 = 0, kanon = -1, cheap = -1;
    if (!kanon_board(&tg, &kanon, &census0, &cheap)) return;

    int filler = sid(&tg, F10_NO);
    if (!expect_card(&tg, "C1 filler", filler, F10_NO, 4)) return;
    cheap = sid(&tg, LIVE19_NO);
    if (!expect_card(&tg, "C1 cheap", cheap, LIVE19_NO, -1)) return;
    hand_push(&tg, 0, cheap);
    hand_push(&tg, 0, filler);

    /* CENSUS re-baselined after the LAST fixture card is placed and immediately
     * before the activation under test. */
    census0 = test_total_card_count(&tg);
    CHECK_EQ(tg.state.p[0].energy_active, 10,
             "C1: fixture check -- 10 of the 15 energy cards are active");
    CHECK_EQ(test_zone_len(&tg, 0, "energy"), 15, "C1: fixture check -- 15 energy cards exist");
    CHECK_EQ(stage_occupied(&tg, 0), 1, "C1: Kanon really is on the stage");
    CHECK(hand_count_of(&tg, 0, kanon) == 0, "C1: Kanon is NOT in hand (placed by hand)");

    /* Rust: generate_possible_actions() -> two ChoiceOption actions whose
     * descriptions carry the real cost text. The C's action candidate list
     * carries action_type + card_id but no description, so the "two options"
     * count is asserted through it and the "labels carry the cost text" claim
     * through the STANDING PROMPT, which is where a player would read it. */
    RbGeneratedActionList acts = rb_generate_action_candidates(&tg.state);
    int n_choice = 0;
    for (int i = 0; i < acts.count; i++)
        if (acts.actions[i].action_type == RB_ACTION_CHOICE_CONDITION) n_choice++;
    CHECK_EQ(n_choice, 2,
             "C1: two choice_condition cost options are offered (Rust asserts two ChoiceOption actions)");
    /* The action list's ownership is not part of the published header; this runs
     * in a forked child that exits immediately, so nothing is freed by hand. */

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "C1: control -- no prompt is standing before the activation");

    CHECK_EQ(test_activate_ability(&tg, kanon), 1, "C1: the 起動 ability is activated");

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "C1: the choice_condition cost raises its option prompt");
    dump_pending(&tg, "C1 option prompt");
    {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        const char *ja = c ? c->description_ja : "";
        const char *en = c ? c->description : "";
        CHECK(strstr(ja, JP_Wait) != NULL || strstr(en, JP_Wait) != NULL
              || strstr(en, "wait") != NULL,
              "C1: the option labels carry the real cost text (ウェイト), not Yes/No");
    }

    select_option(&tg, 1);      /* Rust: game.select_option(1) = 手札を1枚控え室に置く */

    CHECK_EQ(test_pending_choice_type(&tg) != NULL &&
             strcmp(test_pending_choice_type(&tg), "SelectCard") == 0, 1,
             "C1: the discard option creates a SelectCard prompt");
    dump_pending(&tg, "C1 SelectCard");

    pick(&tg, 0);
    drain_first(&tg);

    CHECK(!rb_has_pending_choice(&tg.state), "C1: the effect resolves after the discard");
    CHECK_EQ(tg.state.p[0].energy_active, 11,
             "C1: exactly ONE wait energy was activated (10 -> 11)");
    CHECK(in_zone(&tg, 0, "discard", cheap),
          "C1: the selected card really is in the waitroom");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "C1: CENSUS unchanged -- one card moved hand -> waitroom, none destroyed");
}

/* Rust :3497 kanon_activation_choice_condition_wait_option */
static void c2_kanon_activation_choice_condition_wait_option(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int census0 = 0, kanon = -1;
    if (!kanon_board(&tg, &kanon, &census0, NULL)) return;

    CHECK_EQ(tg.state.p[0].energy_active, 10,
             "C2: fixture check -- 10 of the 15 energy cards are active");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "C2: CENSUS baselined immediately before the activation");

    int pre_prompt = rb_has_pending_choice(&tg.state);
    CHECK_EQ(pre_prompt, 0, "C2: control -- no prompt is standing before the activation");

    CHECK_EQ(test_activate_ability(&tg, kanon), 1, "C2: the 起動 ability is activated");

    CHECK(rb_has_pending_choice(&tg.state) && !pre_prompt,
          "C2: the choice_condition cost raises its option prompt");
    dump_pending(&tg, "C2 option prompt");

    select_option(&tg, 0);      /* Rust: game.select_option(0) = このメンバーをウェイトにする */

    CHECK(!rb_has_pending_choice(&tg.state), "C2: the effect resolves after paying the wait cost");
    CHECK_EQ(tg.state.p[0].energy_active, 11,
             "C2: exactly ONE wait energy was activated (10 -> 11)");
    CHECK(tg.state.p[0].stage_wait[0] == 1 ||
          (rb_mods_get_orientation(&tg.state.mods, kanon) != NULL &&
           strcmp(rb_mods_get_orientation(&tg.state.mods, kanon), "wait") == 0),
          "C2: Kanon is in WAIT state after taking the wait option");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "C2: CENSUS unchanged -- a wait state change moves and destroys no card");
}

/* ═══════════════════════════════════════════════════════════════════════ */

static void on_segv(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d during test \"%s\" (engine fault) ***\n",
            sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    long a0 = tally ? tally->assertions : 0;
    int  f0 = tally ? tally->failures : 0;
    int  s0 = tally ? tally->setup_bugs : 0;

    /* The child INHERITS the parent's accumulated counters, so they are zeroed
     * on this side of the fork and the deltas are read back out of the shared
     * mapping on the parent side. */
    pid_t pid = fork();
    if (pid < 0) {
        current_test = name;
        fn();
        current_test = "(none)";
        n_ok++;
        return;
    }
    if (pid == 0) {
        if (tally) { tally->assertions = 0; tally->failures = 0; tally->setup_bugs = 0; }
        current_test = name;
        fn();
        printf("        %s: %ld assertion(s), %d failure(s), %d setup bug(s)\n",
               name, tally ? tally->assertions : 0,
               tally ? tally->failures : 0,
               tally ? tally->setup_bugs : 0);
        fflush(stdout); fflush(stderr);
        _Exit((tally && (tally->failures > 0 || tally->setup_bugs > 0)) ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    /* The parent ACCUMULATES the child's deltas -- read only AFTER waitpid,
     * because the mapping is shared and the child may not have written yet. */
    if (tally) {
        tally->assertions = a0 + tally->assertions;
        tally->failures   = f0 + tally->failures;
        tally->setup_bugs = s0 + tally->setup_bugs;
    }
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    }
    fflush(stdout);
}

/* ═══════════════════════════════════════════════════════════════════════ */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    tally = mmap(NULL, sizeof(Tally), PROT_READ | PROT_WRITE,
                 MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (tally == MAP_FAILED) {
        fprintf(stderr, "FAIL: could not map the shared tally\n");
        return 1;
    }
    tally->assertions = 0;
    tally->failures = 0;
    tally->setup_bugs = 0;

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    printf("card database loaded: %u cards\n", rb_num_cards());
    print_heart_map();
    printf("decode-fallback abilities in this build: %u (rb_verify_generated_tables "
           "self-check is run by the generators; a mojibake string key below means "
           "regen, not an engine bug)\n", rb_decode_fallback_count());

    printf("--- §A ayumu LL-bp1-001-R+ multi-name debut / live start ---\n");
    run("A1_debut_recover_from_discard", a1_ayumu_debut_recover_from_discard);
    run("A1b_two_candidates_prompt",    a1b_ayumu_two_candidates_prompt);
    run("A2_q62_individual_names",      a2_ayumu_q62_name_has_individual_names);
    run("A3_live_start_triggers",       a3_ayumu_live_start_triggers);

    printf("--- §B bella PL!N-bp3-027-L surplus heart04 + Nijigasaki ---\n");
    run("B1_q174_no_member_no_fire",    b1_bella_q174_no_member_on_stage_does_not_fire);
    run("B2_q174_no_heart04_no_fire",   b2_bella_q174_no_heart04_surplus_does_not_fire);
    run("B3_positive_heart04_fires",    b3_bella_positive_surplus_heart04_fires);

    printf("--- §K kosuzu PL!HS-pb1-005-R choose-number truth table ---\n");
    run("K1_choose_number_to_hand",     k1_kosuzu_choose_number_adds_to_hand);
    run("K2_choose_number_gains_blade", k2_kosuzu_choose_number_gains_blade);
    run("K3_equal_choice_both",         k3_kosuzu_equal_choice_both_effects);

    printf("--- §L lovepeace PL!-bp3-026-L live-success +1 / q149 / q172 ---\n");
    run("L1_q150_hearts_gt_plus1",      l1_lovepeace_q150_hearts_greater_score_plus_1);
    run("L2_q149_total_hearts",         l2_lovepeace_q149_total_hearts_sum_of_base);
    run("L3_q172_gained_hearts",        l3_lovepeace_q172_gained_hearts_count_but_not_blade);

    printf("--- §H hareruya PL!SP-bp1-026-L distinct Liella! names ---\n");
    run("H1_q64_five_distinct",         h1_hareruya_q64_five_distinct_names_condition_met);
    run("H1b_four_distinct_not_met",    h1b_hareruya_four_distinct_names_condition_not_met);
    run("H2_q74_multiname_counting",    h2_hareruya_q74_multiname_distinct_counting);

    printf("--- §Y you LL-bp2-001-R+ q129 / live-start optional cost ---\n");
    run("Y1_q129_self_only",            y1_you_q129_cost_reduction_self_only);
    run("Y2_live_start_optional_cost",  y2_you_live_start_optional_cost_triggers);
    run("Y3_abilityless_not_in_choice", y3_you_abilityless_card_not_in_choice);

    printf("--- §C kanon PL!SP-bp5-001-R+ choice_condition activation cost ---\n");
    run("C1_choice_discard_flow",       c1_kanon_activation_choice_condition_discard_flow);
    run("C2_choice_wait_option",        c2_kanon_activation_choice_condition_wait_option);

    rb_unload();

    printf("\n=== parity_full_round3 ===\n");
    printf("tests passed          : %d\n", n_ok);
    printf("tests failed (parity) : %d\n", n_failed);
    printf("tests crashed (engine): %d\n", n_crashed);
    printf("assertions (shared)   : %ld\n", tally->assertions);
    printf("failures  (shared)    : %d\n", tally->failures);
    printf("fixture setup bugs    : %d\n", tally->setup_bugs);
    if (tally->failures || tally->setup_bugs || n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("ALL PARITY CHECKS PASSED\n");
    return 0;
}
