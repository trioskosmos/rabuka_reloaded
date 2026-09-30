/* test_parity_wave1a.c -- C parity suite for FIVE Rust clusters that a fresh
 * exhaustive survey (985 Rust files / 4,020 #[test] fns) confirmed are
 * genuinely UNTOUCHED by the C corpus.  Each cluster was verified by three
 * independent negatives: no rare card in the cluster appears anywhere in the C
 * corpus, no C file cites it, and no name hit.
 *
 * CLAIM (agent tr_wave1a) -- the EXACT Rust files this C file ports, and the
 * only ones.  No other agent owns any part of this file.  This file edits no
 * engine code and no Makefile.
 *
 *   1. engine/tests/test_modules/effects/auto/pb2_self_wait_or_cost_paid_activates_and_gains_blade_pl_pb2_021_n_test.rs          (5 tests)
 *   2. engine/tests/test_modules/effects/count/count_as_two_lily_white_pl_pb2_041_l_test.rs                                      (3 tests)
 *   3. engine/tests/test_modules/effects/invalidate/live_success/pb2_invalidate_live_success_gains_heart03_pl_pb2_003_pp_test.rs  (4 tests)
 *   4. engine/tests/test_modules/effects/gain/abilities/debut_gain_mus_constant_blade_when_score_card_pl_pb2_005_pp_test.rs      (6 tests)
 *   5. engine/tests/test_modules/effects/live_start/dazzling_game_test.rs                                                        (3 tests)
 *
 * 21 Rust tests, all ported, 21 run() cases.
 *
 * ══ RESULTS OF THE LAST RUN (10 ok, 11 failed, 0 crashed, 167 assertions,
 * ══ 23 failures, 0 setup bugs).  NO assertion is skipped, deleted or
 * ══ weakened; every red below is reported rather than fixed, because this
 * ══ agent does not own engine code.
 *
 *   A effects/auto                    4 ok  1 FAILED
 *       A1 becoming_wait_...          RED  blade 2, expected 1.  CONFIRMED
 *                                       ENGINE BUG: the 自動/turn-1 fires
 *                                       TWICE.  The trace shows two
 *                                       EXEC_CHANGE_STATE runs on the same
 *                                       card and a STATE_CHANGE re-trigger
 *                                       that re-queues the ability it has
 *                                       already consumed.  A3's in-test
 *                                       control fires the same 自動 for a
 *                                       card that is ALREADY active, so no
 *                                       re-trigger happens and it correctly
 *                                       yields exactly 1 -- which is what
 *                                       isolates the double-fire to the
 *                                       state-change path.  Paired positive:
 *                                       YES (A3 control, blade 1 on the same
 *                                       board), so A1 is not vacuous.
 *   B effects/count                   1 ok  2 FAILED
 *       B1 (parse shape)              GREEN.  count_as_multiple / count_as=2 /
 *                                       counted_by="lily white" all decode
 *                                       correctly -- see the engine-gap note.
 *       B2 alone_counts_as_two         RED  score +0, expected +1.  CONFIRMED
 *                                       ENGINE GAP: the wire data survives the
 *                                       decode but NOTHING CONSUMES IT.  A grep
 *                                       for "count_as" over every .c in
 *                                       engine_c_wip/src returns no hits, so
 *                                       there is no count-weighting in the C
 *                                       engine at all and the success live
 *                                       zone counts 1 card against a
 *                                       2-above threshold.  The in-test
 *                                       control (same flow, zone NOT seeded)
 *                                       also reads +0, which is the right
 *                                       result but does NOT by itself prove
 *                                       non-vacuity -- the non-vacuity proof
 *                                       here is B3, which does fire.
 *       B3 not_per_zone_occurrence    RED  score +2, expected +1.  CONFIRMED
 *                                       ENGINE BUG: the modify_score target
 *                                       pool chains the live card zone and
 *                                       the success live card zone WITHOUT
 *                                       dedup, so the same id is scored
 *                                       twice.  This is the exact bug the
 *                                       Rust file documents as FIXED.  Paired
 *                                       positive: YES (score went to +2, so
 *                                       the effect demonstrably fires).
 *   C effects/invalidate/live_success  1 ok  3 FAILED  -- ALL THREE REDS ARE
 *                                       ARTEFACTS OF THE BUILD-PIPELINE SKEW
 *                                       (see the SKEW REPORT below): the
 *                                       action decodes as "stiming_conditioni"
 *                                       instead of "invalidate_ability".
 *       C2 gains_heart03              RED  heart03=1 as expected, BUT
 *                                       invalidations=0 and no target prompt.
 *       C3 no_live_success_gains_none  RED  heart03=1, expected 0.  The
 *                                       conditional 「これにより無効にした
 *                                       場合」 is not evaluated.
 *       C4 our_own_not_invalidated     RED  heart03=1, expected 0, with the
 *                                       opponent's stage EMPTY.
 *                                       Non-vacuity: the ability is UNLIMITED
 *                                       (use_limit = -1, pinned by an
 *                                       assertion), so its use counter is not
 *                                       a usable control and the paired
 *                                       positive is C2/C3 themselves -- the
 *                                       heart IS produced on all three
 *                                       boards, so the finding is "granted
 *                                       unconditionally", not "nothing ran".
 *   D effects/gain/abilities           4 ok  2 FAILED  -- BOTH REDS ARE
 *                                       ARTEFACTS OF THE SKEW: the
 *                                       `group_names` filter decodes as the
 *                                       four bytes "arde" instead of the
 *                                       mu's group, in BOTH the effect extra
 *                                       and the condition, so no card can
 *                                       ever match.
 *       D1 (positive!)                RED  blade 0 for BOTH mu's members,
 *                                       expected 1 each.  The non-mu'
 *                                       AZALEA member correctly stays 0, so
 *                                       this is a three-way discriminator
 *                                       that fails on two of three.
 *       D6 two_cards_not_double        RED  blade 0, expected 1 -- same cause.
 *   E effects/live_start               0 ok  3 FAILED
 *       E1/E2 blade hearts to purple  RED  all 7 blade hearts land at
 *                                       index 5, NONE at index 6.  The
 *                                       `set_blade_type` / `blade_type =
 *                                       紫ブレード` pair decodes CORRECTLY
 *                                       (no splice) and the engine DOES run
 *                                       set_blade_type, so this one is NOT a
 *                                       skew artefact: the blade-type change
 *                                       never reaches the yell-revealed cards.
 *                                       The 2 COLORLESS hearts DO land at
 *                                       index 0 correctly, so the reveal and
 *                                       the heart routing both work.
 *       E3 q187 exclude_selected       RED  neither selection is prompted.
 *                                       The 3-action sequential DOES run and
 *                                       its first `select` prompts -- but
 *                                       with zone="hand", not "stage", and
 *                                       its `characters` list decodes as a
 *                                       splice of two other names, so the
 *                                       pool cannot be built.  Blades stay 0.
 *
 * A representation gap that is NOT a finding but a porting limit: E1/E2 in Rust
 * assert a PER-CARD blade_hearts[8] and a per-card note_icons out of
 * PerformanceSnapshot; C's RbLiveSnapshot carries only ONE flat
 * yell_blade_hearts[8] plus a reveal list.  E1/E2 are therefore ported as the
 * AGGREGATE those per-card claims imply, with per-card membership proved
 * through the reveal list.
 *
 * ENCODING.  Every Japanese string in a FUNCTIONAL position is a verified
 * \xNN byte escape (see JA_WAIT / JA_COST_PAID below, both generated with
 * s.encode("utf-8"), never hand-typed -- a hand-typed run silently diverges,
 * e.g. a hand-typed U+8671 never matches the engine's U+8679).  The file is
 * otherwise valid UTF-8 and Japanese appears in COMMENTS and ASSERTION
 * MESSAGES only, where a mis-decode would be cosmetic -- this is the peers'
 * convention.  Group names are never written literally: rb_card_group_name() is
 * PRINTED at startup, so a filter mismatch shows up as a visible name instead
 * of a silent "no match".  Card numbers are ASCII; U+FF0B FULLWIDTH PLUS uses
 * FP.  The file was re-read and validated as UTF-8 after every write.
 *
 * CARD IDENTITY.  Every fixture below was read out of cards/cards.json read as
 * UTF-8 (never Get-Content -Raw, which mangles non-ASCII and produces FALSE
 * "missing" results) and is pinned byte-exactly at run time by pin_id(),
 * which compares rb_card_string(c.card_no_idx) with strcmp.  rb_card_no_eq is
 * FUZZY (a peer proved it accepts PL!SP-PR-004-R against the real
 * PL!SP-PR-004-PR) and is NOT used as an identity pin anywhere in this file.
 *
 * LANDMINES respected:
 *   - rb_load("src") fails under an isolated out-of-tree build, so
 *     load_card_db() falls back to ../cards/build and cards/build.
 *   - sizeof(GameState) is ~781 KB: every TestGame is `static`.
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE, so no
 *     heart is read through it.  rb_mods_get_heart is called directly and the
 *     heartNN -> enum map is DERIVED at startup from the engine's own
 *     rb_heart_color_as_str / rb_parse_heart_color and PRINTED, so a mismatch
 *     is visible in the log instead of hiding inside an arithmetic result.
 *   - rb_card_string() takes a STRING-TABLE INDEX, not a card id: card_no_of()
 *     goes through the decoded card's card_no_idx.
 *   - card_no_of() uses a ROTATING ring.  A `no_of()` helper with a single
 *     `static char buf[80]` called 2-3 times in one printf prints the LAST card
 *     repeatedly, which is a false-red generator.
 *   - rb_card_copy_pool_reset() runs per case (fresh()).
 *   - The C deck bag is TOP-INDEX-0, so Rust `cards.push(x)` is an APPEND and
 *     Rust `cards.insert(0, x)` is test_insert_deck_top(); the BOTTOM of a deck
 *     is the END of the array.
 *   - rb_calc_stage_hearts() RECOMPUTES, so the Rust's pre-seeded
 *     `player1.stage_hearts = Some(30 x heart00)` is NOT ported: in C it is
 *     ignored.  The heart budget is supplied the C way, by a real stage.
 *   - test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) destroys the yell
 *     window, so no case steps to PERFORMANCE_SECOND.
 *   - rb_advance_phase is a no-op while a choice is pending: every step is
 *     followed by a drain, and every drain is capped.
 *   - mid() is Rust's game.id() (a fresh pool instance, test_new_id);
 *     test_id (sid) is the shared template and is never used as a board card.
 *   - rb_card_group_name() is SERIES level.  group_idx is empty for all 2,639
 *     cards, so every group/unit claim in this file is made through
 *     rb_card_group_name() / rb_card_unit_name(), never through a raw index.
 *   - the waitroom zone string is "discard".
 *
 * ANTI-VACUITY.  Three separate controls, on EVERY case:
 *   (a) CENSUS: test_total_card_count() is sampled after setup and re-asserted
 *       at the assertion site.  A zone count of 0 looks identical whether a
 *       move did nothing or DESTROYED a card; the census is what tells those
 *       apart, and it is what turned a peer finding from "nothing happened"
 *       into "three cards were deleted".
 *   (b) every NEGATIVE carries a paired POSITIVE on the same board shape
 *       (the Rust's own positive for that cluster) or a PROMPT-COUNT control
 *       via CHECK_PROMPTS, which counts the prompt still ON SCREEN at the
 *       assertion site, not only at answer time.
 *   (c) the grids in cluster D and cluster E had the shape a peer found VACUOUS
 *       BY CONSTRUCTION (every positive and negative had card count == colour
 *       count), so each negative there additionally pins the FIXTURE it is
 *       supposed to break -- the success-zone length, the exact card sitting in
 *       it, the number of revealed cards -- so "nothing was granted" cannot be
 *       confused with "nothing was set up".
 *
 * run() is crash ISOLATION, not a hang detector: no watchdog is achievable on
 * this toolchain.  Every drain is capped.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child killed by SIGSEGV/SIGBUS/
 * SIGABRT still reports the assertions it completed before dying.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as the byte escape. */
#define FP "\xef\xbc\x8b"

/* Japanese needles, as verified UTF-8 byte escapes (GENERATED with
 * s.encode("utf-8"), never hand-typed -- a hand-typed run silently diverges).
 * JA_WAIT   = U+30A6 U+30A7 U+30A4 U+30C8 U+72B6 U+614B, the run that means
 *             "became wait state" in the card text.
 * JA_COST_PAID = U+30B3 U+30B9 U+30C8 U+3092 U+652F U+6255 U+3044, the run
 *             that means "paid a cost". */
#define JA_WAIT      "\xE3\x82\xA6\xE3\x82\xA7\xE3\x82\xA4\xE3\x83\x88\xE7\x8A\xB6\xE6\x85\x8B"
#define JA_COST_PAID "\xE3\x82\xB3\xE3\x82\xB9\xE3\x83\x88\xE3\x82\x92\xE6\x94\xAF\xE6\x89\x95\xE3\x81\x86"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── fixtures ───────────────────────────────────────────────────────────
 * cards/cards.json read as UTF-8, with the printed metadata that each Rust
 * test asserts, so a wrong number here is visible rather than vacuous.
 *
 *   no                  name / type / rare / cost / score / unit / series
 *   PL!-pb2-021-N   1  N    member cost 4                    Printemps  LL
 *   PL!-sd1-010-SD     SD   member cost 4                    Printemps  LL
 *   PL!-pb2-041-L   7  L    live   score 7                   lily white LL
 *   PL!-pb1-029-SRL 1  SRL  live   score 1                   lily white LL
 *   PL!-pb2-003-PP     PP   member cost 9                    Printemps  LL
 *   PL!-bp6-001-P      P   member cost 2                    Printemps  LL
 *   PL!-bp6-017-N      N   member cost 2                    Printemps  LL
 *   PL!SP-bp4-023-L 8  L    live   score 8                   (unit None) SS!!
 *   PL!HS-pb1-023-N 15 N    member cost 15                   EdelNote  HnS
 *   PL!SP-pb1-001-R     R   member cost 9                    CatChu!   SS!!
 *   PL!SP-sd1-002-SD    SD  member cost 11                   KALEID.   SS!!
 *   PL!-sd1-006-SD     SD  member cost 9                    BiBi      LL
 *   PL!-pb2-005-PP     PP  member cost 5                    lily white LL
 *   PL!S-bp5-012-N      N  member cost 2                    AZALEA    Sunshine
 *   PL!-bp4-026-L   1  L    live   score 1                   BiBi      LL
 *   PL!-bp4-024-L   2  L    live   score 2                   Printemps LL
 *   PL!HS-bp1-019-L 1  L    live   score 1                   (unit None)HnS
 *   PL!-sd1-013-SD     SD  member cost 4                    lilywhite LL
 *   PL!S-PR-017-PR     PR  member cost 4                    CYaRon!   Sunshine
 *   PL!S-PR-015-PR     PR  member cost 4                    AZALEA    Sunshine
 *   PL!S-bp2-015-PR     PR  member cost 4                    GuiltyKiss Sunshine
 *   PL!N-bp1-021-N     N  member cost 4                    QU4RTZ    Nijigasaki
 *   PL!-sd1-020-SD 2  SD   live   score 2                   (unit None)LL
 *   PL!-sd1-019-SD 1  SD   live   score 1                   (unit None)LL
 *   PL!N-bp1-027-L 0  L    live   score 0                   (unit None)Nijigasaki
 *   PL!N-bp7-030-L 0  L    live   score 0                   (unit None)Nijigasaki
 *
 * The MUS_ALY / MUS_NO_ICON fixtures are Printemps and BiBi members, and the
 * DOLLCHESTRA fixture is a Hasunosora live with no unit: the "mu's" filter on
 * Rin Hoshizora's gained constant is a SERIES-level filter, which is exactly
 * why every group claim in this file goes through rb_card_group_name() and
 * rb_card_group_name() is printed at startup.
 */
#define A_ERI        "PL!-pb2-021-N"   /* Nan Kotori, 自動/turn-1  PL!-pb2-021-N */
#define A_FILLER     "PL!-sd1-010-SD"  /* plain Printemps filler                     */
#define B_ROMANTIC   "PL!-pb2-041-L"   /* Harujou Romantic, live, score 7, lily white */
#define B_LW1        "PL!-pb1-029-SRL" /* a SECOND lily white live, score 1           */
#define B_FILLER     "PL!-sd1-010-SD"
#define C_NICO       "PL!-pb2-003-PP"  /* Nan Kotori PP, ライブ開始時 invalidate      */
#define C_OPP_LS     "PL!-bp6-001-P"   /* Honoka P  -- HAS a ライブ成功時 ability     */
#define C_OPP_PLAIN  "PL!-bp6-017-N"   /* Hanayo N   -- has NONE                     */
#define C_FILLER     "PL!-sd1-010-SD"
#define D_RIN        "PL!-pb2-005-PP"  /* Hoshizora Rin PP, 登場 gain_ability, cost 5 */
#define D_MUS_ALLY   "PL!-bp6-017-N"   /* Hanayo -- mu's, no ability of her own      */
#define D_NON_MUS    "PL!S-bp5-012-N"  /* Kanon -- AZALEA, not mu's                  */
#define D_MUS_SCORE  "PL!-bp4-026-L"   /* BiBi live WITH a スコア icon                */
#define D_MUS_NOICON "PL!-bp4-024-L"   /* Printemps live with NO スコア icon          */
#define D_NONMUSCORE "PL!HS-bp1-019-L" /* DOLLCHESTRA live WITH a スコア icon         */
#define D_FILLER     "PL!-sd1-010-SD"
#define E_DAZZLING   "PL!SP-bp4-023-L"  /* Dazzling Game, live, score 8               */
#define E_HOST       "PL!HS-pb1-023-N"  /* Ceraceus Ririenfeld, EdelNote member, c15 */
#define E_EXTRA      "PL!-sd1-006-SD"   /* Maki Nishikino, BiBi member               */
#define E_FILLER     "PL!-sd1-010-SD"
#define E_H01        "PL!-sd1-013-SD"   /* Umi Sonoda, blade heart at index 1        */
#define E_H02        "PL!S-PR-017-PR"   /* Watanabe You,  index 2                     */
#define E_H03        "PL!-sd1-010-SD"   /* filler used as a blade heart at index 3    */
#define E_H04        "PL!S-PR-015-PR"   /* Kanon,          index 4                     */
#define E_H05        "PL!S-bp2-015-PR"  /* Yoshiko,        index 5                     */
#define E_H06        "PL!N-bp1-021-N"   /* Rina Tenma, no colour, all-blade -> 6      */
#define E_ALL        "PL!-sd1-020-SD"   /* ALL-blade card, index 7 -> recoloured to 6 */
#define E_SCORE      "PL!-sd1-019-SD"   /* START:DASH!!, a スコア icon                */
#define E_DRAW       "PL!N-bp1-027-L"   /* Solitude Rain, a ドロー icon               */
#define E_COLORLESS  "PL!N-bp7-030-L"   /* Cheer Mode, two COLOURLESS hearts          */
#define E_KANON      "PL!SP-pb1-001-R"  /* Kanon, one of the three NAMED members      */
#define E_LIELLA     "PL!SP-sd1-002-SD" /* Kako, the OTHER Liella! member             */

/* blade_hearts slot indices (Rust PerformanceSnapshot::blade_hearts[8]) */
#define YH_COLORLESS 0
#define YH_PURPLE    6
#define YH_ALL       7

/* ── shared counters (survive a child that dies mid-assertion) ───────── */

typedef struct {
    int assertions;
    int failures;
    int setup_bugs;
    int prompts;
} Tally;

static Tally *tal;
static const char *current_test = "(none)";

static void tally_init(void)
{
    void *p = mmap(NULL, sizeof(Tally), PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    tal = (p == MAP_FAILED) ? NULL : (Tally *)p;
    if (tal) memset(tal, 0, sizeof *tal);
}

static void on_fatal_signal(int sig)
{
    if (tal) {
        fprintf(stderr,
                "\n*** FATAL SIGNAL %d inside test: %s\n"
                "*** The engine, not this test, is at fault.\n"
                "*** Assertions completed before the fault: %d, failures: %d, "
                "prompts: %d\n",
                sig, current_test, tal->assertions, tal->failures, tal->prompts);
    } else {
        fprintf(stderr, "\n*** FATAL SIGNAL %d inside test: %s\n", sig, current_test);
    }
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        if (tal) tal->assertions++;                                       \
        if (!(condition)) {                                               \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        if (tal) tal->assertions++;                                       \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

/* VACUITY CONTROL.  The peer macro CHECK_PROMPTS() increments tal->prompts and
 * then compares tal->prompts against the threshold, so it can satisfy itself:
 * a case that offered NO prompt still records one.  Every negative here
 * therefore uses CHECK_PROMPTS_GT, which compares the prompt count taken at a
 * point BEFORE the interaction against the count after it, so the control is
 * genuinely about prompts the board produced.  The prompt still ON SCREEN at
 * the assertion site is captured through note_prompt_open() before it is
 * answered, because counting only at answer time reports 0 for a prompt that is
 * plainly open. */
#define CHECK_PROMPTS_GT(before, n, ...) do {                              \
        if (tal) tal->assertions++;                                       \
        int p_ = tal ? tal->prompts : 0;                                  \
        if (p_ <= (int)(before) || (p_ - (int)(before)) < (int)(n)) {     \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - %d prompt(s) were produced after the        \
                    control point, need at least %d\n",                   \
                    p_ - (int)(before), (int)(n));                        \
        }                                                                 \
    } while (0)

/* Same for the census: a card that ARRIVES is legitimate, so the delta is
 * asserted rather than an absolute equality.  A negative delta is a real
 * finding: cards were DESTROYED, which a zone count of 0 cannot distinguish
 * from "nothing happened". */
static void census_check_delta(TestGame *tg, int before, int expect_delta,
                               const char *what)
{
    int now = test_total_card_count(tg);
    if (tal) tal->assertions++;
    int delta = now - before;
    if (delta != expect_delta) {
        if (tal) tal->failures++;
        fprintf(stderr, "FAIL: CENSUS - %s changed the whole-board card count "
                        "by %d, expected %d (%d -> %d). A zone count of 0 looks "
                        "the same whether a move did nothing or DESTROYED a "
                        "card; this is the control that tells them apart.\n",
                what, delta, expect_delta, before, now);
    } else {
        fprintf(stderr, "        [census] whole-board card count %d -> %d "
                        "(delta %d, as expected: %s)\n", before, now, delta, what);
    }
}

/* ── card DB + fixtures ───────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* The heartNN -> RbHeartColor map, derived at run time from the engine's own
 * rb_heart_color_as_str / rb_parse_heart_color rather than hard-coded, because
 * a peer found a SHIPPED test asserting heart06 through RB_HEART_PURPLE.  The
 * engine's mapping is 0=heart00 1=heart01 2=heart02 3=heart03 4=heart04
 * 5=heart05 6=heart06.  PRINTED at startup so a mismatch is visible in the log
 * instead of hiding inside an arithmetic result. */
static int HC[7];

static void hcolor_init(void)
{
    for (int i = 0; i < 7; i++)
        HC[i] = (int)rb_parse_heart_color(rb_heart_color_as_str(i));
}

/* A ROTATING ring, never one shared buffer: a no_of() helper with a single
 * `static char buf[80]` called 2-3 times in one printf prints the LAST card
 * repeatedly, which is a false-red generator. */
#define NO_RING 8
static const char *card_no_of(int cid)
{
    static char ring[NO_RING][96];
    static int next;
    char *buf = ring[next++ % NO_RING];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 96, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: byte-exact strcmp of the decoded card_no, because
 * rb_card_no_eq is FUZZY. */
static int pin_id(int cid, const char *no)
{
    if (tal) tal->assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    if (tal) { tal->failures++; tal->setup_bugs++; }
    fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

/* Rust `game.id(x)` mints a NEW instance per call.  C twin: test_new_id. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* A fresh game with the copy pool reset: a long suite must never exhaust the
 * process-global pool and silently fall back to the shared template. */
static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    rb_look_reset_all();
}

static int printed_cost(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

static int printed_score(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.score;
    rb_free_card(&c);
    return v;
}

/* Rust `a.triggers.as_deref().is_some_and(|t| t.contains(..))`. */
static int card_trigger_contains(int cid, const char *needle)
{
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, needle) != NULL;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int card_trigger_index(int cid, const char *trig)
{
    if (cid < 0) return -1;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (hit) return i;
    }
    return -1;
}

/* Decode the ability that prints `trig` and hand its EFFECT back to the
 * caller, who must rb_free_ability() it.  NULL when the card has no such
 * ability. */
static int ability_effect_of(int cid, const char *trig, Ability *out)
{
    memset(out, 0, sizeof *out);
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            *out = ab;
            return 1;
        }
        rb_free_ability(&ab);
    }
    return 0;
}

static int effect_has_extra(const AbilityEffect *e, const char *k)
{
    if (!e) return 0;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return 1;
    return 0;
}

static const char *effect_extra(const AbilityEffect *e, const char *k)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k))
            return e->extra_v[i] ? e->extra_v[i] : NULL;
    return NULL;
}

/* A decoded condition field's boolean VALUE, read through the tag the engine
 * actually emits.  The wire form is RB_TAG_TRUE (0x02) / RB_TAG_FALSE (0x01),
 * NOT a "true" string -- a first port that required RB_TAG_STR here reported a
 * correct decode as a missing field.  Accepts the string form too. */
static int cond_bool(const Condition *c, const char *key)
{
    if (!c) return 0;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key || strcmp(f->key, key)) continue;
        if (f->v.tag == RB_TAG_TRUE)  return 1;
        if (f->v.tag == RB_TAG_FALSE) return 0;
        if (f->v.tag == RB_TAG_STR && f->v.s) return !strcmp(f->v.s, "true");
        if (f->v.b) return 1;
    }
    return 0;
}

/* ── decoded-shape dump (diagnostic, W1_DUMP=1) ────────────────────────
 * Several of the ported Rust tests are PARSE-SHAPE tests ("this branch must
 * parse as a self-scoped state condition"), and a shape claim can only be
 * settled by looking at what the C decoder actually produced, not at prose. */
static void dump_cond_fields(const char *pfx, const Condition *c)
{
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        const CondValue *v = &f->v;
        fprintf(stderr, "%s   cond.variant=%u field[%u] %s = ", pfx, c->variant, i, f->key);
        if (v->tag == RB_TAG_STR)        fprintf(stderr, "\"%s\"\n", v->s ? v->s : "(null)");
        else if (v->tag == RB_TAG_I64)   fprintf(stderr, "%lld\n", (long long)v->i);
        else if (v->tag == RB_TAG_ARRAY) fprintf(stderr, "[array of %u]\n", v->arr_n);
        else                             fprintf(stderr, "(tag %u)\n", v->tag);
    }
}

static void dump_cond(const char *pfx, const Condition *c, int depth)
{
    if (!c) return;
    fprintf(stderr, "%s  condition(variant=%u) {\n", pfx, c->variant);
    dump_cond_fields(pfx, c);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_ARRAY) {
            for (uint32_t j = 0; j < f->v.arr_n; j++) {
                fprintf(stderr, "%s     %s[%u] tag=%u s=%s cond=%s\n", pfx, f->key, j,
                        f->v.arr[j].tag, f->v.arr[j].s ? f->v.arr[j].s : "(null)",
                        f->v.arr[j].cond ? "DECODED" : "NULL");
                if (f->v.arr[j].cond) dump_cond(pfx, f->v.arr[j].cond, depth + 2);
            }
        }
        if (f->v.cond) dump_cond(pfx, f->v.cond, depth + 1);
    }
    fprintf(stderr, "%s  }\n", pfx);
}

static void dump_effect(const char *pfx, const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int k = 0; k < depth; k++) fputc(' ', stderr);
    fprintf(stderr, "%s effect action=\"%s\" count=%d src=%s dst=%s target=%s "
                    "card_type=%s is_optional=%d n_child=%d n_options=%d has_cond=%d\n",
            pfx, e->action ? e->action : "(none)", e->count,
            e->source ? e->source : "-", e->destination ? e->destination : "-",
            e->target ? e->target : "-", e->card_type_field,
            e->is_optional, e->n_child, e->n_options, e->has_condition);
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i])
            fprintf(stderr, "%s   extra[%d] %s = %s\n", pfx, i, e->extra_k[i],
                    e->extra_v[i] ? e->extra_v[i] : "(null)");
    if (e->has_condition && e->condition) dump_cond(pfx, e->condition, depth + 1);
    for (int i = 0; i < e->n_child; i++)    dump_effect(pfx, e->child[i], depth + 2);
    for (int i = 0; i < e->n_options; i++)  dump_effect(pfx, e->options[i], depth + 2);
    if (e->primary_effect)     dump_effect(pfx, e->primary_effect, depth + 2);
    if (e->followup_action)    dump_effect(pfx, e->followup_action, depth + 2);
    if (e->optional_action)    dump_effect(pfx, e->optional_action, depth + 2);
    if (e->conditional_action) dump_effect(pfx, e->conditional_action, depth + 2);
    if (e->gained_effect)      dump_effect(pfx, e->gained_effect, depth + 2);
}

static void dump_card_abilities(const char *no)
{
    int cid = rb_find_card_by_no(no);
    if (cid < 0) { fprintf(stderr, "=== %s NOT IN THE DATABASE ===\n", no); return; }
    int n = rb_card_num_abilities((uint32_t)cid);
    fprintf(stderr, "=== decode dump: %s (id=%d, %d abilit%s) ===\n",
            no, cid, n, n == 1 ? "y" : "ies");
    for (int a = 0; a < n; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        fprintf(stderr, "  ab#%d triggers=%s use_limit=%d\n", a,
                ab.triggers ? ab.triggers : "-", ab.use_limit);
        if (ab.cost)   { fprintf(stderr, "  [cost]\n");   dump_effect("  ", ab.cost, 2); }
        if (ab.effect) { fprintf(stderr, "  [effect]\n"); dump_effect("  ", ab.effect, 2); }
        rb_free_ability(&ab);
    }
}

/* ── board helpers ─────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static void note_prompt(TestGame *tg)
{
    if (pending(tg) && tal) tal->prompts++;
}

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, idx);
}

static int drain_pick_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
    return guard;
}

/* Rust `while has_pending_choice() { select_indices(&[0]) }` with the peers'
 * bl_drain_proceed meaning of index 0: ACCEPT / PROCEED.  The C fold of an
 * EMPTY answer is index -1 = DECLINE, which would silently skip a fired
 * trigger and make the whole case vacuous. */
static void drain_auto(TestGame *tg)
{
    drain_pick_first(tg, 24);
}

/* CENSUS CONTROL.  test_total_card_count() is a zone-independent count of
 * every card slot of BOTH seats across every zone.  A card that moves between
 * zones must leave this total unchanged; a drop means cards were DESTROYED,
 * which a per-zone count of 0 cannot distinguish from "nothing happened". */
static int census0;

static void census_take(TestGame *tg)
{
    census0 = test_total_card_count(tg);
    fprintf(stderr, "        [census] whole-board card count after setup = %d\n",
            census0);
}

static void census_check(TestGame *tg, const char *what)
{
    int now = test_total_card_count(tg);
    if (tal) tal->assertions++;
    if (now != census0) {
        if (tal) tal->failures++;
        fprintf(stderr, "FAIL: CENSUS - %s changed the whole-board card count "
                        "(%d -> %d). A zone count of 0 looks the same whether a "
                        "move did nothing or DESTROYED a card; this is the "
                        "control that tells them apart.\n",
                what, census0, now);
    } else {
        fprintf(stderr, "        [census] whole-board card count still %d (%s)\n",
                now, what);
    }
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int stage_has(const TestGame *tg, int pl, int cid)
{
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg->state.p[pl].stage[a] == cid) return 1;
    return 0;
}

static int stage_len(const TestGame *tg, int pl)
{
    int n = 0;
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg->state.p[pl].stage[a] != RB_EMPTY_SLOT) n++;
    return n;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void hand_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = cid;
}

/* Rust `main_deck.cards.push(x)` -- an APPEND, so x lands at the BOTTOM. */
static void deck_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        tg->state.p[pl].deck.n = 0;
        for (int i = 0; i < n; i++) deck_append(tg, pl, filler);
    }
}

/* Rust `deck_from_top`: replace a main deck wholesale, index 0 = TOP. */
static void set_deck_from_top(TestGame *tg, int pl, const int *cards, int n)
{
    RbBag *b = &tg->state.p[pl].deck;
    b->n = 0;
    for (int i = 0; i < n && i < RB_MAX_DECK; i++) b->cards[b->n++] = cards[i];
}

static int blade_of(const TestGame *tg, int cid)
{
    return rb_mods_get_blade((RbMods *)&tg->state.mods, cid);
}

static int score_of(const TestGame *tg, int cid)
{
    return rb_mods_get_score((RbMods *)&tg->state.mods, cid);
}

static int heart_of(const TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart((RbMods *)&tg->state.mods, cid, color);
}

static const char *orientation_of(const TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation((RbMods *)&tg->state.mods, cid);
    return o ? o : "";
}

/* helpers/mod.rs fire_trigger: build "<card_no>_<full_text>" for the ability
 * printing `trig`, queue exactly that ability, set activating_card, process. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static int fire_trigger(TestGame *tg, int cid, const char *trig,
                        const char *trigger_type)
{
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof card_no, "%s", rb_card_string(card.card_no_idx));
    char ability_id[512];
    int found = 0;
    for (int a = 0; a < rb_card_num_abilities((uint32_t)cid) && !found; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            snprintf(ability_id, sizeof ability_id, "%s_%s", card_no,
                     ab.full_text ? ab.full_text : "");
            found = 1;
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
    if (!found) return 0;
    rb_trigger_auto_ability(&tg->state, ability_id, trigger_type, 0, card_no,
                            cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static void print_zone(const char *label, const RbBag *b)
{
    fprintf(stderr, "        [zone] %-22s n=%d :", label, b->n);
    for (int i = 0; i < b->n && i < 12; i++) fprintf(stderr, " %s", card_no_of(b->cards[i]));
    if (b->n > 12) fprintf(stderr, " ...");
    fputc('\n', stderr);
}

static const RbLiveSnapshot *last_snapshot(const TestGame *tg, int pl)
{
    for (int i = tg->state.n_snapshots - 1; i >= 0; i--)
        if (tg->state.snapshots[i].player == pl) return &tg->state.snapshots[i];
    return NULL;
}

static int snapshot_revealed(const RbLiveSnapshot *s, int cid)
{
    for (int i = 0; i < s->n_yell_cards; i++)
        if (s->yell_cards[i] == cid) return 1;
    return 0;
}

/* ── startup diagnostics ───────────────────────────────────────────────── */

/* ── BUILD-PIPELINE SKEW REPORT ──────────────────────────────────────────
 * THIS IS WHY CLUSTERS C, D AND E ARE RED, and it is reported at every run so
 * the reds are self-explaining rather than mysterious.
 *
 * The C engine resolves every decoded string through a (chunk, start, len)
 * triple baked into the COMPILED-IN bytecode
 * (engine_c_wip/src/core/generated/bytecode_blob.c, which is COMMITTED and
 * matches HEAD) while the string TABLE it indexes is a LOCAL, GITIGNORED
 * artifact, cards/build/abilities_strings.bin (.gitignore:189
 * "cards/build/*.bin").  The string table is an UNSEPARATED interned pool --
 * `cost_limit` is stored immediately before `conditional`, `timing_condition`
 * immediately before `invalidate_ability`, `left_side` immediately after
 * `...anged`, and so on -- so a table that is a few bytes out of step with the
 * committed bytecode does not fail loudly: it returns a BYTE SPLICE of two
 * neighbouring pool entries.  Observed on this run:
 *
 *   key   cost_limit        + c(onditional)      -> "cost_limitc"
 *   key   left_side         - l                  -> "eft_side"
 *   value invalidate_ability shifted 1 byte      -> "stiming_conditioni"
 *   value invalidate_triggers                     -> "ationsthose_ca"
 *   value mu's                                     -> "arde"
 *   value <three-name character list>             -> a mix of two other names
 *
 * Every load-bearing group / trigger / character filter in clusters C, D and E
 * is read through exactly those spliced strings, which is why those clusters
 * cannot match anything.  Clusters A and B decode cleanly (see the
 * W1_DUMP=1 output) and their reds are therefore NOT artefacts of this.
 *
 * The skew is currently IN FLIGHT: cards/build/generation_manifest.json was
 * rewritten in the working tree during this run, cards/build/abilities.bin.z is
 * modified, and the manifest's own `output.bytes` disagrees with the
 * abilities.bin sitting next to it.  A re-run after a consistent
 * compile_abilities.py pass is required before the C/D/E reds mean anything. */
static long long file_size_of(const char *path)
{
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return -1; }
    long long n = ftell(f);
    fclose(f);
    return n;
}

static void report_pipeline_skew(void)
{
    static const char *const paths[] = {
        "cards/build/abilities_strings.bin", "../cards/build/abilities_strings.bin",
        "cards/build/abilities.bin",         "../cards/build/abilities.bin",
        "src/core/generated/bytecode_blob.c"
    };
    for (unsigned i = 0; i < sizeof paths / sizeof paths[0]; i++) {
        long long n = file_size_of(paths[i]);
        if (n >= 0)
            fprintf(stderr, "--- pipeline input: %-42s %lld bytes\n", paths[i], n);
    }
    long long manifest = -1;
    FILE *mf = fopen("cards/build/generation_manifest.json", "rb");
    if (!mf) mf = fopen("../cards/build/generation_manifest.json", "rb");
    if (mf) {
        char buf[4096];
        size_t got = fread(buf, 1, sizeof buf - 1, mf);
        buf[got] = 0;
        fclose(mf);
        const char *p = strstr(buf, "\"bytes\"");
        if (p) manifest = atoll(p + 7);
    }
    long long bin = file_size_of("cards/build/abilities.bin");
    if (bin < 0) bin = file_size_of("../cards/build/abilities.bin");
    if (manifest <= 0 || bin < 0)
        fprintf(stderr, "--- pipeline skew: manifest output.bytes=%lld  "
                        "abilities.bin=%lld  -> UNVERIFIABLE HERE (the isolated "
                        "build root carries only cards.bin and "
                        "abilities_strings.bin, not the manifest / "
                        "abilities.bin).  The skew itself is real and was "
                        "measured in the repo: manifest output.bytes=%s while "
                        "cards/build/abilities.bin is 98537 bytes and "
                        "bytecode_blob.c is a COMMITTED file matching HEAD.  See "
                        "the BUILD-PIPELINE SKEW REPORT in the file header.\n",
                manifest, bin,
                "98509 at HEAD / 98542 in the working tree");
    else
        fprintf(stderr, "--- pipeline skew: manifest output.bytes=%lld  "
                        "abilities.bin=%lld  -> %s\n",
                manifest, bin,
                (manifest != bin)
                    ? "MISMATCH, the string table and the compiled bytecode come "
                      "from different compiles; every decoded group/trigger/"
                      "character filter below is a byte splice and the C/D/E reds "
                      "are artefacts"
                    : "consistent");
}

static void print_wire_map(void)
{
    fprintf(stderr, "--- heart index map (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 7; i++)
        fprintf(stderr, "    index %d -> \"%s\" -> enum %d\n",
                i, rb_heart_color_as_str(i), HC[i]);

    fprintf(stderr, "--- group / unit names (rb_card_group_name is SERIES level; "
                    "group_idx is empty for ALL 2,639 cards) ---\n");
    static const char *const watch[] = {
        A_ERI, A_FILLER, B_ROMANTIC, B_LW1, C_NICO, C_OPP_LS, C_OPP_PLAIN,
        D_RIN, D_MUS_ALLY, D_NON_MUS, D_MUS_SCORE, D_MUS_NOICON, D_NONMUSCORE,
        E_DAZZLING, E_HOST, E_EXTRA, E_KANON, E_LIELLA, E_H06, E_ALL,
        E_SCORE, E_DRAW, E_COLORLESS
    };
    for (unsigned i = 0; i < sizeof watch / sizeof watch[0]; i++) {
        int id = rb_find_card_by_no(watch[i]);
        if (id < 0) { fprintf(stderr, "    %-22s NOT FOUND\n", watch[i]); continue; }
        fprintf(stderr, "    %-22s id=%-6d group=\"%s\" unit=\"%s\" "
                        "cost=%d score=%d\n",
                watch[i], id, rb_card_group_name(id), rb_card_unit_name(id),
                printed_cost(id), printed_score(id));
    }

    if (getenv("W1_DUMP")) {
        static const char *const dump[] = {
            A_ERI, B_ROMANTIC, C_NICO, C_OPP_LS, C_OPP_PLAIN,
            D_RIN, D_MUS_SCORE, D_MUS_NOICON, D_NONMUSCORE,
            E_DAZZLING, E_KANON, E_LIELLA
        };
        for (unsigned i = 0; i < sizeof dump / sizeof dump[0]; i++)
            dump_card_abilities(dump[i]);
    }
}

/* =========================================================================
 * A. effects/auto -- pb2_self_wait_or_cost_paid_activates_and_gains_blade_
 *    pl_pb2_021_n_test.rs  (5 tests, card PL!-pb2-021-N)
 *
 * A note on the host print the brief asked about: the Rust file's own fixtures
 * are unambiguous.  PL!-pb2-021-N is PL!-pb2-021-N in cards.json, and the
 * brief's "bp4-023 vs pb1-023 transposition" does NOT apply to THIS cluster --
 * it is a different cluster (E, below), where both numbers are real and
 * correct.  See the E section header.
 * ======================================================================= */

/* Rust `after_auto(waited)`: stage ERI at index 0, optionally put her in wait,
 * fire the 自動, drain, then read (orientation, blade). */
static void a_after_auto(TestGame *tg, int waited, int eri)
{
    int filler = mid(tg, A_FILLER);
    deck_append(tg, 0, filler);
    for (int i = 0; i < 11; i++) deck_append(tg, 0, filler);
    set_stage(tg, 0, AREA_CENTER, eri);
    if (waited)
        rb_mods_set_orientation((RbMods *)&tg->state.mods, eri, "wait");
    CHECK(fire_trigger(tg, eri, RB_TSTR_AUTO, RB_TSTR_AUTO),
          "A: the %s card carries a %s ability in the compiled record",
          A_ERI, RB_TSTR_AUTO);
    drain_auto(tg);
}

/* Rust: becoming_wait_activates_her_and_grants_a_blade */
static void a1_becoming_wait_activates_and_grants_a_blade(void)
{
    static TestGame tg;
    fresh(&tg);
    int eri = mid(&tg, A_ERI);
    pin_id(eri, A_ERI);
    CHECK_EQ(printed_cost(eri), 4,
             "A1 PREMISE: cards.json prints cost 4 for " A_ERI);

    a_after_auto(&tg, 1, eri);
    census_take(&tg);
    fprintf(stderr, "        [info] A1 after the wait-branch fire: orientation=\"%s\" "
                    "blade=%d pending=%d\n",
            orientation_of(&tg, eri), blade_of(&tg, eri), pending(&tg));
    census_check(&tg, "the 自動 fire");

    CHECK_EQ(blade_of(&tg, eri), 1,
             "A1: the wait branch of the `or` grants exactly one blade");
    /* 「このメンバーをアクティブにし」 -- the SAME activation must also flip her
     * out of wait.  Asserting both halves is what makes this a positive rather
     * than a lone +1 that some other modifier could have produced. */
    CHECK(strcmp(orientation_of(&tg, eri), "active") == 0,
          "A1: the wait branch of the `or` also makes her active "
          "(orientation is \"%s\")", orientation_of(&tg, eri));
}

/* Rust: an_empty_setup_grants_nothing */
static void a2_empty_setup_grants_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int filler = mid(&tg, A_FILLER);
    for (int i = 0; i < 12; i++) deck_append(&tg, 0, filler);
    int eri = mid(&tg, A_ERI);
    pin_id(eri, A_ERI);
    /* She is not even on stage: `game.id(ERI)` mints a bare id. */
    CHECK(!stage_has(&tg, 0, eri),
          "A2 PREMISE: the card is not on stage in this setup");
    rb_recalc_constants(&tg.state);
    census_take(&tg);
    CHECK_EQ(blade_of(&tg, eri), 0,
             "A2: a plain recalculation grants nothing -- the 自動 is what "
             "grants the blade (this is the Rust precondition for A1)");
    census_check(&tg, "a bare recalculation");
}

/* Rust: it_does_not_fire_for_another_cards_ability */
static void a3_does_not_fire_for_another_cards_ability(void)
{
    static TestGame tg;
    fresh(&tg);
    int filler = mid(&tg, A_FILLER);
    for (int i = 0; i < 12; i++) deck_append(&tg, 0, filler);
    int eri  = mid(&tg, A_ERI);
    int other = mid(&tg, A_ERI);
    pin_id(eri, A_ERI);
    pin_id(other, A_ERI);
    /* Two DISTINCT instances of the SAME print: `other` is staged, `eri` is
     * not, and the ability belongs to the member that gained it. */
    set_stage(&tg, 0, AREA_CENTER, other);
    CHECK(stage_has(&tg, 0, other) && !stage_has(&tg, 0, eri),
          "A3 PREMISE: the staged instance is `other`, the bare id is `eri`");
    rb_recalc_constants(&tg.state);
    census_take(&tg);
    CHECK_EQ(blade_of(&tg, eri), 0,
             "A3: the ability belongs to the member that gained it, so the "
             "unstaged twin gains nothing");
    census_check(&tg, "the twin-identity setup");

    /* ANTI-VACUITY DISCRIMINATOR.  "eri gains nothing" is also what a dead
     * engine produces, so the IDENTICAL setup is then driven through the 自動
     * with the STAGED instance and must gain.  Without this control A3 would
     * pass on an engine that never grants a blade at all. */
    CHECK(fire_trigger(&tg, other, RB_TSTR_AUTO, RB_TSTR_AUTO),
          "A3 CONTROL: the staged instance has a 自動 in the compiled record");
    drain_auto(&tg);
    fprintf(stderr, "        [info] A3 control: after firing for the STAGED twin, "
                    "eri blade=%d, other blade=%d\n",
            blade_of(&tg, eri), blade_of(&tg, other));
    CHECK(blade_of(&tg, other) > 0,
          "A3 CONTROL: on this very board the STAGED instance DOES gain a blade, "
          "so the un-staged twin's zero is a real scoping result and not a dead "
          "engine");
}

/* Rust: the_wait_branch_parses_as_a_self_scoped_state_condition */
static void a4_wait_branch_parses_as_self_scoped_state_condition(void)
{
    static TestGame tg;
    fresh(&tg);
    int eri = mid(&tg, A_ERI);
    Ability ab;
    if (!ability_effect_of(eri, RB_TSTR_AUTO, &ab)) {
        CHECK(0, "A4: the card has a %s ability to inspect", RB_TSTR_AUTO);
        return;
    }
    CHECK_EQ(card_trigger_index(eri, RB_TSTR_AUTO) >= 0, 1,
             "A4 PREMISE: the 自動 is present");
    const AbilityEffect *e = ab.effect;
    CHECK(e != NULL, "A4: the %s ability has an effect", RB_TSTR_AUTO);
    if (!e) { rb_free_ability(&ab); return; }
    CHECK(e->has_condition && e->condition != NULL,
          "A4: the ability is conditional (a bare effect would make the "
          "two-branch assertion vacuous)");
    if (!e->condition) { rb_free_ability(&ab); return; }

    const Condition *c = e->condition;
    const CondField *cf = NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, "conditions")) cf = &c->fields[i];
    CHECK(cf != NULL, "A4: the or_condition carries its branches under \"conditions\"");
    if (!cf) { dump_cond("A4", c, 0); rb_free_ability(&ab); return; }
    CHECK_EQ(cf->v.arr_n, 2, "A4: two branches");
    if (cf->v.arr_n != 2) {
        dump_cond("A4", c, 0);
        rb_free_ability(&ab);
        return;
    }

    /* Branch 1 is the wait branch; the engine's own state value is the ASCII
     * semantic handle, so the Japanese needle is only used to CONFIRM which
     * branch is which, never to find it. */
    const Condition *b0 = cf->v.arr[0].cond;
    const Condition *b1 = cf->v.arr[1].cond;
    CHECK(b0 && b1, "A4: both branches decoded");
    if (!b0 || !b1) { dump_cond("A4", c, 0); rb_free_ability(&ab); return; }

    const Condition *wb = NULL;
    for (int k = 0; k < 2; k++) {
        const Condition *b = k ? b1 : b0;
        for (uint32_t i = 0; i < b->n_fields; i++)
            if (b->fields[i].key && !strcmp(b->fields[i].key, "state") &&
                b->fields[i].v.tag == RB_TAG_STR && b->fields[i].v.s &&
                !strcmp(b->fields[i].v.s, "wait")) wb = b;
    }
    const Condition *ob;
    ob = (wb == b0) ? b1 : b0;
    CHECK(wb != NULL, "A4: one branch carries state=\"wait\"");
    if (!wb) { dump_cond("A4", c, 0); rb_free_ability(&ab); return; }
    CHECK(ob != NULL, "A4: the other branch is a real condition, not NULL");

    int st_wait = 0, has_count = 0, has_op = 0;
    for (uint32_t i = 0; i < wb->n_fields; i++) {
        const CondField *f = &wb->fields[i];
        if (!f->key) continue;
        if (!strcmp(f->key, "state") && f->v.tag == RB_TAG_STR && f->v.s &&
            !strcmp(f->v.s, "wait")) st_wait = 1;
        if (!strcmp(f->key, "count")) has_count = 1;
        if (!strcmp(f->key, "operator")) has_op = 1;
    }
    dump_cond("A4 wait-branch", wb, 0);
    CHECK(st_wait, "A4: the wait branch is a STATE change (state=\"wait\")");
    /* The wire form of the boolean is the RB_TAG_TRUE tag, not a "true"
     * string; reading it the wrong way turns a correct decode into a red. */
    fprintf(stderr, "        [info] A4 wait-branch self_target decoded as %d\n",
            cond_bool(wb, "self_target"));
    CHECK(cond_bool(wb, "self_target"),
          "A4: the wait branch is self-scoped (self_target=true) -- otherwise "
          "any waited member on stage would satisfy it");
    CHECK(!has_count,
          "A4: the vacuous `count >= 1` form must not come back -- with no "
          "operator it compares as >= and is always true");
    (void)has_op;

    /* Confirm by the printed text as well, so the branch we called the wait
     * branch really is the Japanese clause. */
    int text_hit = 0;
    for (uint32_t i = 0; i < wb->n_fields; i++) {
        const CondField *f = &wb->fields[i];
        if (f->key && f->v.tag == RB_TAG_STR && f->v.s && strstr(f->v.s, JA_WAIT))
            text_hit = 1;
    }
    CHECK(text_hit,
          "A4: the branch identified by state=\"wait\" is the clause whose text "
          "contains the wait-state phrase");
    rb_free_ability(&ab);
}

/* Rust: the_cost_paid_branch_is_still_a_vacuous_comparison */
static void a5_cost_paid_branch_is_still_a_vacuous_comparison(void)
{
    static TestGame tg;
    fresh(&tg);
    int eri = mid(&tg, A_ERI);
    Ability ab;
    if (!ability_effect_of(eri, RB_TSTR_AUTO, &ab)) {
        CHECK(0, "A5: the card has a %s ability to inspect", RB_TSTR_AUTO);
        return;
    }
    const AbilityEffect *e = ab.effect;
    if (!e || !e->condition) {
        CHECK(0, "A5: the ability has a condition to inspect");
        rb_free_ability(&ab);
        return;
    }
    const Condition *c = e->condition;
    const CondField *cf = NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, "conditions")) cf = &c->fields[i];
    if (!cf || cf->v.arr_n != 2) {
        CHECK(0, "A5: the or_condition carries two branches (got %s)",
              cf ? "a different arity" : "no branch list");
        dump_cond("A5", c, 0);
        rb_free_ability(&ab);
        return;
    }
    const Condition *wb = NULL, *cb = NULL;
    for (int k = 0; k < 2; k++) {
        const Condition *b = cf->v.arr[k].cond;
        if (!b) continue;
        int is_wait = 0;
        for (uint32_t i = 0; i < b->n_fields; i++)
            if (b->fields[i].key && !strcmp(b->fields[i].key, "state") &&
                b->fields[i].v.tag == RB_TAG_STR && b->fields[i].v.s &&
                !strcmp(b->fields[i].v.s, "wait")) is_wait = 1;
        if (is_wait) wb = b; else cb = b;
    }
    CHECK(wb && cb, "A5: the two branches are distinguishable (one is state=\"wait\")");
    if (!cb) { dump_cond("A5", c, 0); rb_free_ability(&ab); return; }

    int text_hit = 0, has_op = 0, has_count = 0;
    const char *variant_txt = "";
    for (uint32_t i = 0; i < cb->n_fields; i++) {
        const CondField *f = &cb->fields[i];
        if (!f->key) continue;
        if (!strcmp(f->key, "text") && f->v.tag == RB_TAG_STR && f->v.s &&
            strstr(f->v.s, JA_COST_PAID)) text_hit = 1;
        if (!strcmp(f->key, "operator")) has_op = 1;
        if (!strcmp(f->key, "count")) has_count = 1;
    }
    if (!text_hit)
        for (uint32_t i = 0; i < cb->n_fields; i++) {
            const CondField *f = &cb->fields[i];
            if (f->v.tag == RB_TAG_STR && f->v.s && strstr(f->v.s, JA_COST_PAID))
                text_hit = 1;
        }
    if (!text_hit) variant_txt = " (no cost-paid needle found on the branch)";
    dump_cond("A5 cost-branch", cb, 0);
    fprintf(stderr, "        [info] A5 cost-branch variant=%u%s\n",
            cb->variant, variant_txt);
    CHECK(text_hit,
          "A5: the non-wait branch is the clause whose text contains the "
          "cost-paid phrase");
    /* THE PIN.  compare_counts defaults to >=, so a branch with no operator
     * and no count is `cost >= 0` -- always true, which is the reason this 自動
     * over-fires.  The Rust file pins this CURRENTLY-VACUOUS state on purpose. */
    CHECK(!has_op && !has_count,
          "A5 (OPEN GAP pinned by the Rust file): the cost-paid branch is a "
          "COMPARISON with no operator and no count, so compare_counts defaults "
          "to >= and the branch is vacuously true -- the reason the 自動 fires "
          "even with no trigger");
    rb_free_ability(&ab);
}

/* =========================================================================
 * B. effects/count -- count_as_two_lily_white_pl_pb2_041_l_test.rs
 *    (3 tests, card PL!-pb2-041-L, Harujou Romantic, score 7, lily white)
 * ======================================================================= */

/* Rust: the_constant_parses_as_a_counting_weight_of_two */
static void b1_constant_parses_as_a_counting_weight_of_two(void)
{
    static TestGame tg;
    fresh(&tg);
    int rom = mid(&tg, B_ROMANTIC);
    pin_id(rom, B_ROMANTIC);
    /* The Rust asserts score 7 from cards.json; the COMPILED record is
     * asserted too, because a printed score that disagrees with cards.bin
     * silently makes a threshold unreachable. */
    CHECK_EQ(printed_score(rom), 7,
             "B1 PREMISE: cards.json and the compiled record both print "
             "score 7 for " B_ROMANTIC);
    /* The weighted card must itself be lily white.  cards.json carries
     * unit="lily white"; the engine's UNIT accessor is the only safe read
     * (group_idx is empty for all 2,639 records). */
    const char *u = rb_card_unit_name(rom);
    fprintf(stderr, "        [info] B1 " B_ROMANTIC " group=\"%s\" unit=\"%s\"\n",
            rb_card_group_name(rom), u ? u : "(null)");
    CHECK(u && strstr(u, "lily") != NULL,
          "B1 PREMISE: the weighted card is itself a lily white card "
          "(rb_card_unit_name is \"%s\")", u ? u : "(null)");

    Ability ab;
    if (!ability_effect_of(rom, RB_TSTR_CONSTANT, &ab)) {
        CHECK(0, "B1: the card has a %s ability to inspect", RB_TSTR_CONSTANT);
        return;
    }
    const AbilityEffect *e = ab.effect;
    if (!e) {
        CHECK(0, "B1: the %s ability has an effect", RB_TSTR_CONSTANT);
        rb_free_ability(&ab);
        return;
    }
    dump_effect("B1", e, 0);
    CHECK(e->action && strcmp(e->action, "count_as_multiple") == 0,
          "B1: the action is count_as_multiple (decoded action is \"%s\") -- a "
          "counting weight, not a custom action",
          e->action ? e->action : "(none)");

    /* count_as: the weight itself, 2 for "2 sheets as treated".  The wire key
     * is `count_as` (Rust's AbilityEffect field is `count_as`), NOT
     * `count_as_any` -- `count_as_any` is only the Rust ACCESSOR name, and a
     * first port that looked for it reported a correctly decoded card as
     * having no weight.  counted_by: the groups whose effects SEE the weight
     * -- NOT a filter on the counted card, so it must not sit in group_names. */
    const char *w = effect_extra(e, "count_as");
    fprintf(stderr, "        [info] B1 extras: count_as=%s counted_by=%s "
                    "group_names=%s\n",
            w ? w : "(absent)",
            effect_extra(e, "counted_by") ? effect_extra(e, "counted_by") : "(absent)",
            effect_extra(e, "group_names") ? effect_extra(e, "group_names") : "(absent)");
    CHECK(w && atoi(w) == 2, "B1: the weight is 2 (\"2 sheets as treated\")");

    /* counted_by is a Vec<String> in Rust; in C it reaches the effect through
     * the generic extra pairs, so a flattened string is compared by substring.
     * The Rust asserts the list is EXACTLY ["lily white"], so the flattened
     * form must contain it and must NOT name any other group. */
    const char *by = effect_extra(e, "counted_by");
    int by_ok = 0;
    if (by) {
        by_ok = strstr(by, "lily white") != NULL;
        if (by_ok) {
            /* exactly one entry: no second group name may follow */
            const char *p = strstr(by, "lily white") + strlen("lily white");
            for (; *p; p++)
                if ((*p >= 'A' && *p <= 'Z') || (*p >= 'a' && *p <= 'z')) { by_ok = 0; break; }
        }
    }
    CHECK(by_ok,
          "B1: counted_by names EXACTLY [\"lily white\"] -- the effects that SEE "
          "the weight, not a filter on the counted card");
    CHECK(!effect_has_extra(e, "group_names"),
          "B1: counted_by must not sit in group_names (it names observers, not "
          "a filter)");
    rb_free_ability(&ab);
}

/* Rust: this_card_alone_counts_as_two_and_earns_the_score_bonus
 * Drives the real phase flow: the card is in the success zone AND is the live
 * being performed, and the 常時 weight is what makes the 2-card threshold fire. */
static int b_run_score_bonus(TestGame *tg, int rom, int seed_zone)
{
    if (seed_zone) {
        int rom2 = rom;
        test_add_to_success(tg, rom2);
        print_zone("success (seeded)", &tg->state.p[0].success);
    }
    int filler = mid(tg, B_FILLER);
    test_add_to_hand(tg, rom);
    test_add_to_hand(tg, filler);
    test_add_to_hand(tg, filler);
    fill_decks(tg, filler, 12);

    /* Rust pre-seeds `player1.stage_hearts = Some(30 x heart00)`.  In C
     * rb_calc_stage_hearts() RECOMPUTES from the stage, so a pre-seeded
     * stage_hearts[] is ignored -- it is NOT ported, and the success
     * determination therefore depends on the real stage. */
    if (!test_advance_to_phase(tg, RB_PHASE_LIVE_SET)) {
        for (int i = 0; i < 8; i++) {
            test_pass(tg);
            drain_auto(tg);
            if (tg->state.phase == RB_PHASE_LIVE_SET) break;
        }
    }
    CHECK_EQ(tg->state.phase, RB_PHASE_LIVE_SET,
             "B2: P1 reached the live-card-set window");
    test_set_live_card(tg, 0, rom);
    print_zone("live", &tg->state.p[0].live);
    drain_auto(tg);
    if (!test_advance_to_phase(tg, RB_PHASE_PERFORMANCE)) {
        for (int i = 0; i < 10 && tg->state.phase != RB_PHASE_PERFORMANCE; i++) {
            test_pass(tg);
            drain_auto(tg);
        }
    }
    CHECK_EQ(tg->state.phase, RB_PHASE_PERFORMANCE,
             "B2: P1 reached performance, where the ライブ開始時 actually runs");
    print_zone("success (at assertion)", &tg->state.p[0].success);
    return score_of(tg, rom);
}

static void b2_alone_counts_as_two_and_earns_the_score_bonus(void)
{
    static TestGame tg;
    fresh(&tg);
    int rom = mid(&tg, B_ROMANTIC);
    pin_id(rom, B_ROMANTIC);

    int got = b_run_score_bonus(&tg, rom, 1);
    census_take(&tg);
    fprintf(stderr, "        [info] B2 score modifier on " B_ROMANTIC " = %d\n", got);
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "B2 PREMISE: the success live card zone holds exactly this one card");
    CHECK_EQ(got, 1,
             "B2: alone in the success live card zone this card is ONE lily "
             "white card against a 2-above threshold, so the 常時 weight is the "
             "only thing that can satisfy it, and the +1 must follow");
    census_check(&tg, "the phase flow to performance");

    /* ANTI-VACUITY CONTROL: the identical flow with the success zone NOT
     * pre-seeded.  A +1 of 0 here proves the bonus above came from the 常時
     * weight and not from the live's printed score or some other modifier. */
    {
        static TestGame ctl;
        fresh(&ctl);
        int rom2 = mid(&ctl, B_ROMANTIC);
        pin_id(rom2, B_ROMANTIC);
        int ctl_got = b_run_score_bonus(&ctl, rom2, 0);
        fprintf(stderr, "        [info] B2 CONTROL (empty success zone) score "
                        "modifier = %d, zone n=%d\n",
                ctl_got, ctl.state.p[0].success.n);
        CHECK_EQ(ctl_got, 0,
                 "B2 CONTROL: with the success live card zone NOT pre-seeded the "
                 "same flow grants no +1, so the +1 in the case above is "
                 "attributable to the 常時 weight and the case is not vacuous");
    }
}

/* Rust: self_score_is_not_applied_once_per_zone_occurrence */
static void b3_self_score_is_not_applied_once_per_zone_occurrence(void)
{
    static TestGame tg;
    fresh(&tg);
    int rom = mid(&tg, B_ROMANTIC);
    int lw1 = mid(&tg, B_LW1);
    pin_id(rom, B_ROMANTIC);
    pin_id(lw1, B_LW1);

    /* A plain qualifying setup, then the SAME id also left in the success
     * zone -- the live being performed is the live card AND an entry in the
     * success live card zone once it has succeeded. */
    test_add_to_success(&tg, lw1);
    test_add_to_success(&tg, rom);
    print_zone("success (seeded)", &tg.state.p[0].success);

    int filler = mid(&tg, B_FILLER);
    test_add_to_hand(&tg, rom);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 12);

    if (!test_advance_to_phase(&tg, RB_PHASE_LIVE_SET)) {
        for (int i = 0; i < 8; i++) {
            test_pass(&tg);
            drain_auto(&tg);
            if (tg.state.phase == RB_PHASE_LIVE_SET) break;
        }
    }
    test_set_live_card(&tg, 0, rom);
    drain_auto(&tg);
    if (!test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE)) {
        for (int i = 0; i < 10 && tg.state.phase != RB_PHASE_PERFORMANCE; i++) {
            test_pass(&tg);
            drain_auto(&tg);
        }
    }
    print_zone("success (at assertion)", &tg.state.p[0].success);
    census_take(&tg);
    int got = score_of(&tg, rom);
    fprintf(stderr, "        [info] B3 score modifier on " B_ROMANTIC " = %d, "
                    "lw1 score modifier = %d\n", got, score_of(&tg, lw1));
    /* lw1 + this card, which the 常時 weight counts as two -> the count is 3,
     * well over the 2-above threshold, so the effect fires.  It must still be
     * worth exactly +1: the target pool must not score the card once per zone
     * it appears in, which made ONE activation pay out +2. */
    CHECK_EQ(got, 1,
             "B3: the score target pool must not score the card once per zone it "
             "appears in, which made one activation pay out +2");
    census_check(&tg, "the two-card success zone flow");
}

/* =========================================================================
 * C. effects/invalidate/live_success --
 *    pb2_invalidate_live_success_gains_heart03_pl_pb2_003_pp_test.rs
 *    (4 tests, card PL!-pb2-003-PP, Nan Kotori PP, ライブ開始時)
 *
 * The heart is CONDITIONAL on the invalidation having actually done something.
 * The fixture difference the whole test rests on is pinned by C1, which is why
 * C3's zero is not a vacuous "the ability did nothing" result.
 * ======================================================================= */

/* Rust `fired_against(opponent)`.  `ally_here` puts the ライブ成功時 carrier on
 * OUR stage instead of the opponent's (Rust's fourth test).
 *
 * The prompt count is sampled IMMEDIATELY BEFORE the ability is fired, so the
 * prompt control in the callers measures prompts the ability itself produced
 * and cannot be satisfied by the five phase passes above. */
static int c_fire_prompt_base;   /* tal->prompts just before the fire       */
static int c_fire_prompted;      /* 1 if a choice was open at the fire site */
static int c_fire_uses;          /* the ability's use counter (UNUSABLE here:
                                  * the compiled record prints use_limit = -1) */
static int c_fire_use_limit;     /* the compiled record's own use_limit      */
static int c_fire_queue_n;       /* queue entries the fire left behind      */

/* Rust `ability.use_limit` as the C decoder reports it. */
static int ability_use_limit(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strcmp(ab.triggers, trig) == 0;
        int lim = ab.use_limit;
        rb_free_ability(&ab);
        if (hit) return lim;
    }
    return -2;
}

static int c_fire(TestGame *tg, const char *opponent, int ally_here,
                  int *prompted)
{
    int filler = mid(tg, C_FILLER);
    fill_decks(tg, filler, 12);
    int opp = mid(tg, opponent);
    if (!pin_id(opp, opponent)) return -1;

    if (ally_here) set_stage(tg, 0, AREA_CENTER, opp);
    else           set_stage(tg, 1, AREA_CENTER, opp);
    /* FIXED-INDEX placement is by design: test_set_opp_stage is fixed-index. */

    int nico = mid(tg, C_NICO);
    if (!pin_id(nico, C_NICO)) return -1;
    hand_append(tg, 0, nico);
    test_give_energy(tg, 20);
    for (int i = 0; i < 5; i++) {
        test_pass(tg);
        drain_auto(tg);
    }
    test_set_live_card(tg, 0, nico);
    drain_auto(tg);

    int ab_idx = card_trigger_index(nico, RB_TSTR_LIVE_START);
    c_fire_use_limit = ability_use_limit(nico, RB_TSTR_LIVE_START);
    c_fire_prompt_base = tal ? tal->prompts : 0;
    if (!fire_trigger(tg, nico, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START)) {
        CHECK(0, "C: " C_NICO " carries a %s ability in the compiled record",
              RB_TSTR_LIVE_SUCCESS);
        return -1;
    }
    /* The prompt still ON SCREEN, counted BEFORE it is answered: counting only
     * at answer time would report 0 for a prompt that was plainly offered. */
    *prompted = pending(tg) ? 1 : 0;
    c_fire_prompted = *prompted;
    if (pending(tg)) {
        fprintf(stderr, "        [choice] kind=%d zone=\"%s\" count=%d "
                        "allow_skip=%d target=\"%s\"\n",
                rb_get_pending_choice(&tg->state)->kind,
                rb_get_pending_choice(&tg->state)->zone,
                rb_get_pending_choice(&tg->state)->count,
                rb_get_pending_choice(&tg->state)->allow_skip,
                rb_get_pending_choice(&tg->state)->target);
        if (tal) tal->prompts++;
        pick(tg, 0);
    }
    drain_auto(tg);
    c_fire_uses = ab_idx >= 0 ? rb_ability_uses_used(&tg->state, nico, ab_idx) : -1;
    c_fire_queue_n = tg->state.queue.n_entries;
    fprintf(stderr, "        [info] C fire: ab#%d use_limit=%d uses_used=%d "
                    "queue_n=%d prompts since control point=%d\n",
            ab_idx, c_fire_use_limit, c_fire_uses, c_fire_queue_n,
            (tal ? tal->prompts : 0) - c_fire_prompt_base);
    return nico;
}

/* Rust: fixture_live_success_abilities_differ */
static void c1_fixture_live_success_abilities_differ(void)
{
    static TestGame tg;
    fresh(&tg);
    int has     = mid(&tg, C_OPP_LS);
    int has_not = mid(&tg, C_OPP_PLAIN);
    pin_id(has, C_OPP_LS);
    pin_id(has_not, C_OPP_PLAIN);
    int h1 = card_trigger_contains(has, RB_TSTR_LIVE_SUCCESS);
    int h0 = card_trigger_contains(has_not, RB_TSTR_LIVE_SUCCESS);
    fprintf(stderr, "        [info] C1 %s has %s: %d | %s has %s: %d\n",
            C_OPP_LS, RB_TSTR_LIVE_SUCCESS, h1,
            C_OPP_PLAIN, RB_TSTR_LIVE_SUCCESS, h0);
    CHECK(h1, C_OPP_LS " must have a %s ability to invalidate", RB_TSTR_LIVE_SUCCESS);
    CHECK(!h0,
          C_OPP_PLAIN " must have none, or the conditional heart could not be "
          "tested");
    /* ANTI-VACUITY: the fixture claim is only worth something if BOTH targets
     * are otherwise interchangeable, so assert the two differ ONLY here. */
    CHECK_EQ(printed_cost(has) >= 0, 1, "C1: the invalidatable target resolves");
    CHECK_EQ(printed_cost(has_not) >= 0, 1, "C1: the plain target resolves");
}

/* Rust: invalidating_a_live_success_ability_gains_heart03 */
static void c2_invalidating_a_live_success_ability_gains_heart03(void)
{
    static TestGame tg;
    fresh(&tg);
    int prompted = 0;
    int nico = c_fire(&tg, C_OPP_LS, 0, &prompted);
    if (nico < 0) return;
    census_take(&tg);
    int n_inv = tg.state.n_ability_invalidations;
    int h = heart_of(&tg, nico, HC[3]);
    fprintf(stderr, "        [info] C2 heart03=%d invalidations=%d "
                    "invalidate(opp,%s)=%d\n",
            h, n_inv, RB_TSTR_LIVE_SUCCESS,
            rb_ability_is_invalidated(&tg.state, -1, RB_TSTR_LIVE_SUCCESS));
    /* NON-VACUITY.  The 1-per-live use counter is NOT a usable control here:
     * the compiled record prints use_limit = -1 (unlimited) for this ability,
     * so the counter never moves and reading it as a control manufactures a
     * false red on a perfectly working ability.  The real control is that
     * heart03 == 1 is the PAIRED POSITIVE for C3's and C4's zero -- on these
     * very boards, with this very code path, the ability's effect IS produced.
     * A board where nothing fired would read heart03 == 0 everywhere. */
    CHECK_EQ(c_fire_use_limit, -1,
             "C2 PREMISE: the compiled record prints use_limit = -1 "
             "(unlimited), which is why the use counter cannot serve as the "
             "non-vacuity control here");
    /* MECHANISM CONTROL: the conditional heart is 「これにより無効にした場合」,
     * so the invalidation must actually be on the record. */
    CHECK(n_inv > 0,
          "C2: the target really carries a %s ability, so an invalidation was "
          "recorded", RB_TSTR_LIVE_SUCCESS);
    /* The target selection is 相手のステージにいるメンバー1人, so a target
     * prompt is owed whenever the opponent has a member on stage. */
    CHECK(prompted, "C2: a target prompt was open at the assertion site");
    CHECK_PROMPTS_GT(c_fire_prompt_base, 1,
                     "C2: the target selection really was prompted -- without "
                     "a prompt the heart could not be attributable to this "
                     "ability");
    fprintf(stderr, "        [info] C2: invalidations=%d, target prompt=%d, "
                    "prompts produced by the fire=%d\n",
            n_inv, prompted, (tal ? tal->prompts : 0) - c_fire_prompt_base);
    CHECK_EQ(h, 1,
             "C2: a target that HAS a %s ability is invalidated, so the "
             "conditional heart is granted", RB_TSTR_LIVE_SUCCESS);
    census_check(&tg, "the invalidating fire");
}

/* Rust: a_target_with_no_live_success_ability_gains_nothing */
static void c3_target_with_no_live_success_ability_gains_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int prompted = 0;
    int nico = c_fire(&tg, C_OPP_PLAIN, 0, &prompted);
    if (nico < 0) return;
    census_take(&tg);
    int n_inv = tg.state.n_ability_invalidations;
    int h = heart_of(&tg, nico, HC[3]);
    fprintf(stderr, "        [info] C3 heart03=%d invalidations=%d prompted=%d\n",
            h, n_inv, prompted);
    /* FIXTURE CONTROL: the difference from C2 is the TARGET, and the target is
     * the ONLY thing that differs.  Without this, "heart03 == 0" would also be
     * what a board where nothing at all happened looks like. */
    CHECK_EQ(stage_len(&tg, 1), 1,
             "C3 PREMISE: the opponent's stage still holds exactly one member");
    CHECK_EQ(card_trigger_contains(rb_find_card_by_no(C_OPP_PLAIN),
                                   RB_TSTR_LIVE_SUCCESS), 0,
             "C3 PREMISE: the target on the opponent's stage has no %s ability",
             RB_TSTR_LIVE_SUCCESS);
    /* NON-VACUITY: this ability is UNLIMITED (use_limit = -1 in the compiled
     * record), so its use counter never moves and cannot be a control.  The
     * real control is C2: identical code, identical prompt, identical board
     * shape, one fixture different -- and there heart03 IS 1.  A board where
     * the ability did nothing would read heart03 == 0 in every case. */
    CHECK_EQ(c_fire_use_limit, -1,
             "C3 PREMISE: the ability is unlimited, so the paired positive C2 is "
             "the non-vacuity control for this negative");
    /* PROMPT CONTROL: the same code path offered the same prompt as C2 (the
     * card text names no group filter), so the zero below is the CONDITIONAL
     * not a dead ability. */
    CHECK(prompted,
          "C3: a target prompt WAS offered -- the card text names no group "
          "filter, so C2 and C3 reach the engine identically up to the answer");
    CHECK_EQ(n_inv, 0,
             "C3: nothing was invalidated, so the conditional heart must not be "
             "granted. This is the case the card text singles out.");
    CHECK_EQ(h, 0,
             "C3: 「これにより無効にした場合」 -- a target with no %s ability "
             "gains nothing", RB_TSTR_LIVE_SUCCESS);
    census_check(&tg, "the non-invalidating fire");
}

/* Rust: our_own_live_success_abilities_are_not_invalidated */
static void c4_our_own_live_success_abilities_are_not_invalidated(void)
{
    static TestGame tg;
    fresh(&tg);
    int prompted = 0;
    int nico = c_fire(&tg, C_OPP_LS, 1, &prompted);
    if (nico < 0) return;
    census_take(&tg);
    int h = heart_of(&tg, nico, HC[3]);
    int n_inv = tg.state.n_ability_invalidations;
    fprintf(stderr, "        [info] C4 heart03=%d invalidations=%d prompted=%d "
                    "p1 stage_len=%d p2 stage_len=%d\n",
            h, n_inv, prompted, stage_len(&tg, 0), stage_len(&tg, 1));
    /* FIXTURE CONTROL: 「相手のステージにいるメンバー1人」 -- our own carrier is
     * on OUR stage and the opponent's stage is empty. */
    CHECK_EQ(stage_len(&tg, 0), 1,
             "C4 PREMISE: the %s carrier is on OUR stage", RB_TSTR_LIVE_SUCCESS);
    CHECK_EQ(stage_len(&tg, 1), 0,
             "C4 PREMISE: the opponent's stage is empty, so there is nothing to "
             "invalidate");
    /* NON-VACUITY: the ability is UNLIMITED (use_limit = -1 in the compiled
     * record), so the paired positive C2 -- same code path, same board, with
     * the carrier on the OPPONENT's stage -- is the control for this zero. */
    CHECK_EQ(c_fire_use_limit, -1,
             "C4 PREMISE: the ability is unlimited, so the paired positive C2 is "
             "the non-vacuity control for this negative");
    CHECK_EQ(n_inv, 0, "C4: no invalidation was recorded");
    CHECK_EQ(h, 0,
             "C4: with no opponent member there is nothing to invalidate, so the "
             "conditional heart is not granted");
    census_check(&tg, "the own-stage setup");
}

/* =========================================================================
 * D. effects/gain/abilities --
 *    debut_gain_mus_constant_blade_when_score_card_pl_pb2_005_pp_test.rs
 *    (6 tests, card PL!-pb2-005-PP, Hoshizora Rin PP, 登場 gain_ability)
 *
 * Two independent filters gate the gain -- a スコア-icon card AND a mu's card
 * in the success zone -- so the fixtures vary each one separately.  Every
 * negative here additionally pins the FIXTURE it is supposed to break (the
 * success-zone length and the exact card sitting in it), because this grid has
 * the vacuous-by-construction shape a peer found in a sibling cluster: a zone
 * count of 0 and a wrong-icon zone otherwise look identical from the outside.
 * ======================================================================= */

/* Rust `debut_with(success_zone)`.  The ids actually seeded into the success
 * zone are handed back in `zone_ids` because `mid()` mints a NEW pool instance
 * per call: re-deriving the id at the assertion site would compare a fresh
 * instance against the one already in the zone and always miss, which is a
 * false red that looks exactly like "the card never arrived". */
static void d_debut_with(TestGame *tg, const char *const *zone, int n_zone,
                         int *rin, int *ally, int *non_mus, int *zone_ids)
{
    int filler = mid(tg, D_FILLER);
    fill_decks(tg, filler, 12);

    int a = mid(tg, D_MUS_ALLY);
    int n = mid(tg, D_NON_MUS);
    set_stage(tg, 0, AREA_RIGHT, a);    /* MemberArea::RightSide */
    set_stage(tg, 0, AREA_LEFT, n);     /* MemberArea::LeftSide  */
    for (int i = 0; i < n_zone; i++) {
        int cid = mid(tg, zone[i]);
        zone_ids[i] = cid;
        if (cid < 0) continue;
        test_add_to_success(tg, cid);
    }
    print_zone("success (seeded)", &tg->state.p[0].success);

    int r = mid(tg, D_RIN);
    pin_id(r, D_RIN);
    CHECK_EQ(printed_cost(r), 5, "D PREMISE: cards.json prints cost 5 for " D_RIN);
    hand_append(tg, 0, r);
    test_give_energy(tg, 10);
    test_play_to_stage(tg, r, AREA_CENTER);
    drain_auto(tg);
    rb_recalc_constants(&tg->state);
    fprintf(stderr, "        [info] D after debut: rin blade=%d ally blade=%d "
                    "non_mus blade=%d (rin at stage[%d]=%s) uses(rin,0)=%d\n",
            blade_of(tg, r), blade_of(tg, a), blade_of(tg, n), AREA_CENTER,
            stage_has(tg, 0, r) ? "yes" : "NO",
            rb_ability_uses_used(&tg->state, r, 0));
    *rin = r; *ally = a; *non_mus = n;
}

static void d1_mus_score_card_grants_the_constant_to_every_mus_member(void)
{
    static TestGame tg;
    fresh(&tg);
    int rin, ally, non_mus, zid[4];
    const char *zone[1] = { D_MUS_SCORE };
    d_debut_with(&tg, zone, 1, &rin, &ally, &non_mus, zid);
    census_take(&tg);
    print_zone("success (at assertion)", &tg.state.p[0].success);

    /* THE FIXTURE DISCIPMINATOR: the whole grid turns on one qualifying card
     * actually being in the zone, so pin its identity and the zone length. */
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "D1 PREMISE: exactly one card in the success live card zone");
    CHECK_EQ(bag_has(&tg.state.p[0].success, zid[0]), 1,
             "D1 PREMISE: that card is the mu's live carrying the score icon "
             "(" D_MUS_SCORE ")");
    CHECK(stage_has(&tg, 0, rin), "D1 PREMISE: Rin is on stage");

    /* THREE-WAY discrimination on one board: the other mu's member gains, she
     * gains (she is mu's too), and the AZALEA member does not.  A 0/0/0 board
     * cannot satisfy this, and neither can a 1/1/1 board. */
    CHECK_EQ(blade_of(&tg, ally), 1,
             "D1: the other mu's member gains +1 blade");
    CHECK_EQ(blade_of(&tg, rin), 1,
             "D1: she is a mu's member too, so the gained constant applies to her");
    CHECK_EQ(blade_of(&tg, non_mus), 0,
             "D1: an AZALEA member gains nothing");
    census_check(&tg, "the debut + recalculation");
}

/* Rust: mus_card_without_score_icon_grants_nothing */
static void d2_mus_card_without_score_icon_grants_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int rin, ally, non_mus, zid[4];
    const char *zone[1] = { D_MUS_NOICON };
    d_debut_with(&tg, zone, 1, &rin, &ally, &non_mus, zid);
    census_take(&tg);
    print_zone("success (at assertion)", &tg.state.p[0].success);

    /* THE DISCRIMINATOR: this is the icon half broken, so the zone must be
     * NON-EMPTY and hold a mu's live.  A 0-length zone is D4, a different
     * case; conflating them is exactly how this grid goes vacuous. */
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "D2 PREMISE: the zone is NOT empty -- only the icon half is broken");
    CHECK_EQ(bag_has(&tg.state.p[0].success, zid[0]), 1,
             "D2 PREMISE: that card is the mu's live with no score icon "
             "(" D_MUS_NOICON ")");
    CHECK_EQ(blade_of(&tg, ally), 0,
             "D2: a mu's live with no score icon does not qualify");
    census_check(&tg, "the no-icon debut");
}

/* Rust: non_mus_score_card_grants_nothing */
static void d3_non_mus_score_card_grants_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int rin, ally, non_mus, zid[4];
    const char *zone[1] = { D_NONMUSCORE };
    d_debut_with(&tg, zone, 1, &rin, &ally, &non_mus, zid);
    census_take(&tg);
    print_zone("success (at assertion)", &tg.state.p[0].success);

    /* THE DISCRIMINATOR: the group half is broken, so the zone must be
     * NON-EMPTY and hold a card that DOES carry the score icon. */
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "D3 PREMISE: the zone is NOT empty -- only the group half is broken");
    CHECK_EQ(bag_has(&tg.state.p[0].success, zid[0]), 1,
             "D3 PREMISE: that card is the non-mu's live carrying the score icon "
             "(" D_NONMUSCORE ")");
    CHECK_EQ(blade_of(&tg, ally), 0,
             "D3: a DOLLCHESTRA live carrying the score icon fails the group "
             "filter");
    census_check(&tg, "the wrong-group debut");
}

/* Rust: empty_success_zone_grants_nothing */
static void d4_empty_success_zone_grants_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int rin, ally, non_mus, zid[4];
    d_debut_with(&tg, NULL, 0, &rin, &ally, &non_mus, zid);
    census_take(&tg);
    print_zone("success (at assertion)", &tg.state.p[0].success);

    CHECK_EQ(tg.state.p[0].success.n, 0,
             "D4 PREMISE: the success live card zone really is empty (this is "
             "the case that must be distinguished from D2 and D3)");
    CHECK(stage_has(&tg, 0, rin),
          "D4 PREMISE: Rin still debuted onto the stage, so the board is live");
    CHECK_EQ(blade_of(&tg, ally), 0,
             "D4: no cards in the success zone, so the ability is not gained");
    census_check(&tg, "the empty-zone debut");
}

/* Rust: a_score_card_arriving_after_debut_does_not_backfill */
static void d5_score_card_arriving_after_debut_does_not_backfill(void)
{
    static TestGame tg;
    fresh(&tg);
    int rin, ally, non_mus, zid[4];
    d_debut_with(&tg, NULL, 0, &rin, &ally, &non_mus, zid);
    census_take(&tg);
    CHECK_EQ(blade_of(&tg, ally), 0,
             "D5 step 1: precondition -- debuted with an empty success zone");
    int z0 = tg.state.p[0].success.n;

    int late = mid(&tg, D_MUS_SCORE);
    test_add_to_success(&tg, late);
    rb_recalc_constants(&tg.state);
    print_zone("success (after late add)", &tg.state.p[0].success);
    fprintf(stderr, "        [info] D5 zone went %d -> %d, ally blade=%d\n",
            z0, tg.state.p[0].success.n, blade_of(&tg, ally));

    /* THE DISCRIMINATOR: the second zero is only meaningful if the add
     * actually HAPPENED.  Without this the case passes on a board where the
     * late card never arrived -- a different thing entirely.  The census is
     * asserted as a DELTA of +1: one card entered a zone, and nothing was
     * destroyed to make room for it. */
    CHECK_EQ(tg.state.p[0].success.n, z0 + 1,
             "D5: the score-icon card really DID arrive after the debut");
    CHECK_EQ(bag_has(&tg.state.p[0].success, late), 1,
             "D5: the arriving card is the mu's live carrying the score icon");
    census_check_delta(&tg, census0, +1,
                       "one card entering the success live card zone");
    CHECK_EQ(blade_of(&tg, ally), 0,
             "D5: the 登場 evaluated the condition once at debut; a card "
             "arriving afterwards does not retroactively grant the ability");
}

/* Rust: two_mus_score_cards_do_not_double_the_constant */
static void d6_two_mus_score_cards_do_not_double_the_constant(void)
{
    static TestGame tg;
    fresh(&tg);
    int rin, ally, non_mus, zid[4];
    const char *zone[2] = { D_MUS_SCORE, B_LW1 };
    d_debut_with(&tg, zone, 2, &rin, &ally, &non_mus, zid);
    census_take(&tg);
    print_zone("success (at assertion)", &tg.state.p[0].success);

    CHECK_EQ(tg.state.p[0].success.n, 2,
             "D6 PREMISE: TWO qualifying cards are in the zone");
    CHECK(bag_has(&tg.state.p[0].success, zid[0]) &&
          bag_has(&tg.state.p[0].success, zid[1]),
          "D6 PREMISE: both are mu's lives carrying the score icon ("
          D_MUS_SCORE " and " B_LW1 ")");
    /* The Rust's value is 1, and 1 is the discriminator: the constant is a
     * flat +1 per mu's member, not scaled by how many qualifying lives are in
     * the zone.  A "grant twice" bug lands on 2 and is caught. */
    CHECK_EQ(blade_of(&tg, ally), 1,
             "D6: gaining the ability is not per qualifying card -- two "
             "score-icon lives are still one 常時 worth of +1");
    census_check(&tg, "the two-card debut");
}

/* =========================================================================
 * E. effects/live_start -- dazzling_game_test.rs (3 tests, PL!SP-bp4-023-L)
 *
 * THE TRANSPOSITION THE BRIEF ASKED ABOUT.  Verified against cards/cards.json
 * read as UTF-8:
 *   PL!SP-bp4-023-L  exists, name "Dazzling Game", type ライブ, rare L,
 *                    series ラブライブ！スーパースター!!, score 8.
 *   PL!HS-pb1-023-N exists, name "セラス 柳田 リリエンフェルト", type メンバー,
 *                    rare N, unit EdelNote, cost 15, series
 *                    ラブライブ！蓮ノ空女学院スクールアイドルクラブ.
 * So there is NO bp4-023/pb1-023 transposition: the LIVE is the bp4-023 print
 * and the HOST is a genuinely different, real card (an EdelNote member -- the
 * live performer and the yeller are different roles).  The brief's "host is
 * called 桜内梨子" does not hold either: NO card in cards/cards.json has that
 * name at all, so it is not a mistyped number, it is a name that does not
 * exist in the corpus.  Both card numbers used below are pinned byte-exactly by
 * pin_id() and their printed metadata is printed at startup.
 *
 * The 3 Rust tests: two of them (E1, E2) assert a PER-CARD blade_hearts[8] and
 * per-card note_icons out of a Rust PerformanceSnapshot.  C's RbLiveSnapshot
 * carries only ONE flat yell_blade_hearts[8] plus a reveal list
 * (yell_cards[] / n_yell_cards) with NO per-card heart array -- the same
 * representation gap test_parity_int_gaps.c already records.  E1/E2 are
 * therefore ported as the AGGREGATE the per-card claims imply (nothing is left
 * at the recoloured-away indices, and the recoloured hearts land at index 6),
 * with the per-card membership proved through the reveal list, and the gap is
 * reported rather than hidden.
 * ======================================================================= */

static int e_run_dazzling(TestGame *tg, const int *layout, int n_layout,
                          int *host, int *extra)
{
    int live = mid(tg, E_DAZZLING);
    int h    = mid(tg, E_HOST);
    int x    = mid(tg, E_EXTRA);
    int filler = mid(tg, E_FILLER);
    *host = h; *extra = x;
    if (!pin_id(live, E_DAZZLING)) return -1;
    if (!pin_id(h, E_HOST)) return -1;
    if (!pin_id(x, E_EXTRA)) return -1;

    tg->state.p[0].hand.n = 0;
    tg->state.p[0].discard.n = 0;
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].hand.n = 0;
    tg->state.p[1].deck.n = 0;
    set_stage(tg, 0, AREA_LEFT, h);
    set_stage(tg, 0, AREA_CENTER, x);
    set_stage(tg, 0, AREA_RIGHT, RB_EMPTY_SLOT);
    for (int i = 0; i < 40; i++) { deck_append(tg, 0, filler); deck_append(tg, 1, filler); }
    test_add_to_hand(tg, live);

    for (int i = 0; i < 5; i++) { test_pass(tg); drain_auto(tg); }
    fprintf(stderr, "        [info] E after 5 passes: phase=%s\n",
            rb_phase_name(tg->state.phase));
    CHECK_EQ(tg->state.phase, RB_PHASE_LIVE_SET,
             "E: five passes from Main land on the live-card-set window");
    test_set_live_card(tg, 0, live);
    drain_auto(tg);
    print_zone("live", &tg->state.p[0].live);

    /* The yell deck, index 0 = TOP. */
    set_deck_from_top(tg, 0, layout, n_layout);

    for (int i = 0; i < 2; i++) { test_pass(tg); drain_auto(tg); }
    drain_auto(tg);
    test_pass(tg);
    drain_auto(tg);
    fprintf(stderr, "        [info] E after the yell: phase=%s n_snapshots=%d "
                    "pending=%d blades(host)=%d blades(extra)=%d\n",
            rb_phase_name(tg->state.phase), tg->state.n_snapshots, pending(tg),
            blade_of(tg, h), blade_of(tg, x));
    CHECK(!pending(tg),
          "E: no unexpected choice is left pending after the yell");
    return live;
}

/* Rust: dazzling_live_start_recolors_listed_blade_hearts_to_purple */
static void e1_recolors_listed_blade_hearts_to_purple(void)
{
    static TestGame tg;
    fresh(&tg);
    int filler = mid(&tg, E_FILLER);
    int h01 = mid(&tg, E_H01);
    int h02 = mid(&tg, E_H02);
    int h03 = mid(&tg, E_H03);
    int h04 = mid(&tg, E_H04);
    int h05 = mid(&tg, E_H05);
    int h06 = mid(&tg, E_H06);
    int all = mid(&tg, E_ALL);
    int scr = mid(&tg, E_SCORE);
    int layout[9] = { filler, h01, h02, h03, h04, h05, h06, all, scr };

    int host = 0, extra = 0;
    if (e_run_dazzling(&tg, layout, 9, &host, &extra) < 0) return;
    census_take(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "E1: P1 produced a performance snapshot"); return; }
    fprintf(stderr, "        [snap] n_yell_cards=%d note_icons=%d "
                    "yell_blade_hearts=[", s->n_yell_cards, s->note_icons);
    for (int i = 0; i < 8; i++) fprintf(stderr, "%d%s", s->yell_blade_hearts[i],
                                        i == 7 ? "" : ",");
    fprintf(stderr, "]\n");
    fprintf(stderr, "        [snap] reveals:");
    for (int i = 0; i < s->n_yell_cards; i++)
        fprintf(stderr, " %s", card_no_of(s->yell_cards[i]));
    fputc('\n', stderr);

    /* PER-CARD MEMBERSHIP: the Rust reads each named card's own blade_hearts.
     * C cannot, but the REVEAL LIST can prove the card's hearts were resolved
     * at all, and the reveal list is produced by the yell, not by the
     * recolour -- so this is not circular. */
    CHECK(snapshot_revealed(s, h01), "E1: " E_H01 " was revealed by the yell");
    CHECK(snapshot_revealed(s, h02), "E1: " E_H02 " was revealed by the yell");
    CHECK(snapshot_revealed(s, h03), "E1: " E_H03 " was revealed by the yell");
    CHECK(snapshot_revealed(s, h04), "E1: " E_H04 " was revealed by the yell");
    CHECK(snapshot_revealed(s, h05), "E1: " E_H05 " was revealed by the yell");
    CHECK(snapshot_revealed(s, h06), "E1: " E_H06 " was revealed by the yell");
    CHECK(snapshot_revealed(s, all), "E1: " E_ALL " was revealed by the yell");
    CHECK(snapshot_revealed(s, scr), "E1: " E_SCORE " was revealed by the yell");

    /* AGGREGATE CONSEQUENCE of the Rust's per-card claims: every one of those
     * cards ends with a purple blade heart and nothing at its original index,
     * so indices 1..5 must be empty and index 7 (the ALL wild) must be empty
     * too (the ALL card's wild was recoloured to purple). */
    int sum15 = 0;
    for (int i = 1; i <= 5; i++) sum15 += s->yell_blade_hearts[i];
    CHECK_EQ(sum15, 0,
             "E1: every listed blade heart was recoloured TO purple, so nothing "
             "is left at indices 1..5");
    CHECK_EQ(s->yell_blade_hearts[YH_ALL], 0,
             "E1: the ALL-blade wildcard of the all-card was also recoloured "
             "to purple, so index 7 is empty");

    /* The recoloured count, computed from the REVEAL LIST (independent of the
     * recolour) rather than hard-coded, so this does not assume how many of
     * the nine cards the yell happened to reach. */
    int n_recolour = snapshot_revealed(s, h01) + snapshot_revealed(s, h02) +
                     snapshot_revealed(s, h03) + snapshot_revealed(s, h04) +
                     snapshot_revealed(s, h05) + snapshot_revealed(s, h06) +
                     snapshot_revealed(s, all);
    CHECK_EQ(s->yell_blade_hearts[YH_PURPLE], n_recolour,
             "E1: exactly the revealed listed cards carry a purple blade heart "
             "at index 6");
    census_check(&tg, "the Dazzling yell");
}

/* Rust: dazzling_live_start_leaves_draw_and_colorless_icons_unchanged */
static void e2_leaves_draw_and_colorless_icons_unchanged(void)
{
    static TestGame tg;
    fresh(&tg);
    int filler = mid(&tg, E_FILLER);
    int draw   = mid(&tg, E_DRAW);
    int colour = mid(&tg, E_COLORLESS);
    int extra  = mid(&tg, E_FILLER);
    int layout[9] = { filler, draw, colour, extra, extra, extra, extra, extra, extra };

    int host = 0, ex = 0;
    if (e_run_dazzling(&tg, layout, 9, &host, &ex) < 0) return;
    census_take(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "E2: P1 produced a performance snapshot"); return; }
    fprintf(stderr, "        [snap] n_yell_cards=%d note_icons=%d "
                    "yell_blade_hearts=[", s->n_yell_cards, s->note_icons);
    for (int i = 0; i < 8; i++) fprintf(stderr, "%d%s", s->yell_blade_hearts[i],
                                        i == 7 ? "" : ",");
    fprintf(stderr, "]\n");

    CHECK(snapshot_revealed(s, draw), "E2: " E_DRAW " was revealed by the yell");
    CHECK(snapshot_revealed(s, colour),
          "E2: " E_COLORLESS " was revealed by the yell");

    /* The draw card: a coloured blade heart is recoloured to purple and its
     * own index is emptied; only indices 1..5 are claimed empty by the Rust,
     * and this board's only recoloured card is this one. */
    CHECK_EQ(s->yell_blade_hearts[5], 0,
             "E2: the draw-icon card's blade heart left index 5");
    /* The colorless card: two COLOURLESS hearts (index 0) stay colorless, and
     * it gains NO purple.  C's aggregate cannot separate the two cards, so
     * this is asserted on the AGGREGATE and the per-card value is reported in
     * the dump above. */
    CHECK_EQ(s->yell_blade_hearts[YH_COLORLESS] - 2 * (snapshot_revealed(s, colour) ? 1 : 0), 0,
             "E2: exactly two colorless blade hearts survive, i.e. the colorless "
             "card was left alone and the draw card contributed none");
    CHECK_EQ(s->yell_blade_hearts[YH_PURPLE],
             snapshot_revealed(s, draw) ? 1 : 0,
             "E2: only the draw-icon card carries a recoloured purple blade "
             "heart; the colorless card gains none");
    census_check(&tg, "the Dazzling yell (draw/colorless board)");
}

/* Rust: dazzling_q187_exclude_selected_liella_other_pickable */
static void e3_q187_exclude_selected_liella_other_pickable(void)
{
    static TestGame tg;
    fresh(&tg);
    int filler = mid(&tg, E_FILLER);
    int dazzling = mid(&tg, E_DAZZLING);
    int kanon  = mid(&tg, E_KANON);
    int liella = mid(&tg, E_LIELLA);
    pin_id(dazzling, E_DAZZLING);
    pin_id(kanon, E_KANON);
    pin_id(liella, E_LIELLA);

    tg.state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) deck_append(&tg, 0, filler);
    tg.state.p[1].deck.n = 0;
    for (int i = 0; i < 40; i++) deck_append(&tg, 1, filler);

    /* Stage: one of the three NAMED members + another Liella! member + a
     * non-Liella! filler that must NOT gain a blade.  The filler is the
     * discriminator: without it a "both members gained" result would also be
     * what a "grant to everyone" bug produces. */
    int third = mid(&tg, E_FILLER);
    set_stage(&tg, 0, AREA_LEFT, kanon);
    set_stage(&tg, 0, AREA_CENTER, liella);
    set_stage(&tg, 0, AREA_RIGHT, third);
    hand_append(&tg, 0, dazzling);

    CHECK_EQ(stage_len(&tg, 0), 3, "E3 PREMISE: three members are on stage");
    fprintf(stderr, "        [info] E3 kanon group=\"%s\" unit=\"%s\" | "
                    "liella group=\"%s\" unit=\"%s\" | third group=\"%s\"\n",
            rb_card_group_name(kanon), rb_card_unit_name(kanon),
            rb_card_group_name(liella), rb_card_unit_name(liella),
            rb_card_group_name(third));

    for (int i = 0; i < 5; i++) { test_pass(&tg); drain_auto(&tg); }
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET,
             "E3: five passes from Main land on the live-card-set window");
    test_set_live_card(&tg, 0, dazzling);
    drain_auto(&tg);
    /* The prompt control point is taken AFTER the live is set, so it measures
     * only the two sequential-selection prompts the LiveStart owes. */
    int pbase = tal ? tal->prompts : 0;
    test_pass(&tg);
    test_pass(&tg);
    drain_auto(&tg);
    census_take(&tg);

    /* LiveStart fires: 3 sequential actions.
     * Action 0: select 1 from the 3 named members. */
    CHECK(pending(&tg), "E3: the first member selection must be prompted");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "E3: expected a SelectCard stage prompt (got \"%s\")",
          test_pending_choice_type(&tg));
    CHECK_PROMPTS_GT(pbase, 1,
                     "E3: the first real selection prompt was offered");
    if (pending(&tg)) {
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        fprintf(stderr, "        [choice] zone=\"%s\" count=%d n_filtered=%d "
                        "target=\"%s\"\n", ch->zone, ch->count,
                ch->n_filtered_indices, ch->target);
        for (int i = 0; i < ch->n_filtered_indices && i < 8; i++)
            fprintf(stderr, "          filtered[%d] = %s\n", i,
                    card_no_of(ch->filtered_indices[i]));
    }
    pick(&tg, 0);
    /* Action 1: select 1 Liella! member OTHER than the first pick
     * (exclude_selected=true). */
    CHECK(pending(&tg),
          "E3: the exclude_selected second selection must be prompted");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "E3: expected a SelectCard stage prompt on the second selection "
          "(got \"%s\")", test_pending_choice_type(&tg));
    CHECK_PROMPTS_GT(pbase, 2,
                     "E3: a SECOND real prompt was offered (a single-prompt "
                     "board would make the first CHECK vacuous)");
    if (pending(&tg)) {
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        fprintf(stderr, "        [choice] 2nd: zone=\"%s\" count=%d "
                        "n_filtered=%d\n", ch->zone, ch->count,
                ch->n_filtered_indices);
        for (int i = 0; i < ch->n_filtered_indices && i < 8; i++)
            fprintf(stderr, "          filtered[%d] = %s\n", i,
                    card_no_of(ch->filtered_indices[i]));
    }
    pick(&tg, 0);
    drain_auto(&tg);

    fprintf(stderr, "        [info] E3 blades: kanon=%d liella=%d third=%d\n",
            blade_of(&tg, kanon), blade_of(&tg, liella), blade_of(&tg, third));
    CHECK(blade_of(&tg, kanon) > 0,
          "E3: Kanon gains a blade from Dazzling Game");
    CHECK(blade_of(&tg, liella) > 0,
          "E3: the other Liella! member gains a blade from Dazzling Game");
    /* THE DISCRIMINATOR: the non-Liella! stage member must gain nothing, so
     * the two blades above cannot be a blanket "everyone on stage" grant. */
    CHECK_EQ(blade_of(&tg, third), 0,
             "E3 DISCRIMINATOR: the non-Liella! member on stage gains nothing, "
             "so the two blades above are a real two-member selection");
    census_check(&tg, "the two sequential selections");
}

/* =========================================================================
 * main -- each case runs in a FORKED child with a SIGSEGV/SIGBUS/SIGABRT
 * handler naming the test in flight.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child that dies mid-assertion still
 * reports what it completed.  run() is crash isolation, not a hang detector:
 * no watchdog is achievable on this toolchain, and every drain is capped.
 * ======================================================================= */

#define CHILD_OK        0
#define CHILD_FAILURES  1

static int n_ok, n_failed, n_crashed, n_setup_bugs;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = tal->failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (tal->failures > f0) n_failed++; else n_ok++;
        printf("%-6s %s  [in-process fallback]\n",
               tal->failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = tal->assertions, f0 = tal->failures;
        int s0 = tal->setup_bugs, p0 = tal->prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d prompt(s)\n",
               name, tal->assertions - a0, tal->failures - f0,
               tal->setup_bugs - s0, tal->prompts - p0);
        fflush(stdout);
        fflush(stderr);
        _Exit(tal->failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("ok     %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("FAILED %s\n", name);
    } else {
        n_crashed++;
        printf("%-6s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    tally_init();
    if (!tal) { fprintf(stderr, "FAIL: could not map the shared tally\n"); return 1; }
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    hcolor_init();
    report_pipeline_skew();
    print_wire_map();

    printf("--- A effects/auto: pb2_self_wait_or_cost_paid... " A_ERI " (5) ---\n");
    run("a1_becoming_wait_activates_and_grants_a_blade",
        a1_becoming_wait_activates_and_grants_a_blade);
    run("a2_empty_setup_grants_nothing", a2_empty_setup_grants_nothing);
    run("a3_does_not_fire_for_another_cards_ability",
        a3_does_not_fire_for_another_cards_ability);
    run("a4_wait_branch_parses_as_self_scoped_state_condition",
        a4_wait_branch_parses_as_self_scoped_state_condition);
    run("a5_cost_paid_branch_is_still_a_vacuous_comparison",
        a5_cost_paid_branch_is_still_a_vacuous_comparison);

    printf("--- B effects/count: count_as_two_lily_white " B_ROMANTIC " (3) ---\n");
    run("b1_constant_parses_as_a_counting_weight_of_two",
        b1_constant_parses_as_a_counting_weight_of_two);
    run("b2_alone_counts_as_two_and_earns_the_score_bonus",
        b2_alone_counts_as_two_and_earns_the_score_bonus);
    run("b3_self_score_is_not_applied_once_per_zone_occurrence",
        b3_self_score_is_not_applied_once_per_zone_occurrence);

    printf("--- C effects/invalidate/live_success: " C_NICO " (4) ---\n");
    run("c1_fixture_live_success_abilities_differ", c1_fixture_live_success_abilities_differ);
    run("c2_invalidating_a_live_success_ability_gains_heart03",
        c2_invalidating_a_live_success_ability_gains_heart03);
    run("c3_target_with_no_live_success_ability_gains_nothing",
        c3_target_with_no_live_success_ability_gains_nothing);
    run("c4_our_own_live_success_abilities_are_not_invalidated",
        c4_our_own_live_success_abilities_are_not_invalidated);

    printf("--- D effects/gain/abilities: debut_gain_mus_constant " D_RIN " (6) ---\n");
    run("d1_mus_score_card_grants_the_constant_to_every_mus_member",
        d1_mus_score_card_grants_the_constant_to_every_mus_member);
    run("d2_mus_card_without_score_icon_grants_nothing",
        d2_mus_card_without_score_icon_grants_nothing);
    run("d3_non_mus_score_card_grants_nothing",
        d3_non_mus_score_card_grants_nothing);
    run("d4_empty_success_zone_grants_nothing", d4_empty_success_zone_grants_nothing);
    run("d5_score_card_arriving_after_debut_does_not_backfill",
        d5_score_card_arriving_after_debut_does_not_backfill);
    run("d6_two_mus_score_cards_do_not_double_the_constant",
        d6_two_mus_score_cards_do_not_double_the_constant);

    printf("--- E effects/live_start: dazzling_game " E_DAZZLING " (3) ---\n");
    run("e1_recolors_listed_blade_hearts_to_purple",
        e1_recolors_listed_blade_hearts_to_purple);
    run("e2_leaves_draw_and_colorless_icons_unchanged",
        e2_leaves_draw_and_colorless_icons_unchanged);
    run("e3_q187_exclude_selected_liella_other_pickable",
        e3_q187_exclude_selected_liella_other_pickable);

    n_setup_bugs = tal->setup_bugs;
    rb_unload();

    printf("\n=== test_parity_wave1a summary ===\n");
    printf("  cases: %d ok, %d failed, %d crashed\n", n_ok, n_failed, n_crashed);
    printf("  assertions: %d, failures: %d, setup bugs: %d, prompts seen: %d\n",
           tal->assertions, tal->failures, n_setup_bugs, tal->prompts);
    if (n_failed || n_crashed || tal->failures) {
        fprintf(stderr, "%d failure(s) across %d crashed case(s)\n",
                tal->failures, n_crashed);
        return 1;
    }
    printf("ALL %d WAVE1A PARITY CHECKS PASSED\n", tal->assertions);
    return 0;
}
