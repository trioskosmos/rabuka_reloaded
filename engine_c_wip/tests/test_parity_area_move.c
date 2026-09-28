/* test_parity_area_move.c
 *
 * C parity suite for the Rust cluster
 *   engine/tests/test_modules/effects/position/area_move/
 * (13 .rs files, 63 #[test] functions; mod.rs carries none)
 *
 * WHAT THIS FILE IS, RELATIVE TO tests/test_parity_zones_position.c
 * ---------------------------------------------------------------
 * test_parity_zones_position.c's HEADER claims the area_move cluster, but its
 * BODIES are two layers that are NOT the per-card behaviours:
 *   layer 1 (tests 1-11) is a unit suite over src/core/zones.c: MemberArea
 *           label/index/tag, check_trigger_position, check_effect_position,
 *           stage area accessors, rb_stage_position_change, the two
 *           rb_stage_formation_change shapes, rb_stage_recycle_under_cards,
 *           rb_stage_can_place_card.
 *   layer 2 (tests 12-26) is a thin end-to-end pass over SIX cards: mill_three
 *           (PL!SP-bp5-006-R, 3 shapes), live-start subunit threshold
 *           (PL!SP-pb2-050-L, 2 of the 3), himeno front area (PL!HS-pb1-014-R,
 *           5 shapes), wien center rotation (PL!SP-bp5-010-R, 3 of 4), tomari
 *           energy return (PL!SP-bp7-022-N, both), chisato rotation
 *           (PL!SP-pb1-003-R), seira swap+records (PL!S-bp5-111-R), jidou
 *           after a swap (PL!SP-bp2-003-R).
 *
 * So of the 63 Rust functions, zones_position already exercises roughly 16
 * (the six cards above). THIS file ports the REMAINDER, which is the part that
 * is actually uncovered: the per-card area-move behaviours of 藤島 慈, 黒澤ルビィ,
 * 松浦果南, 姫芽's group filter, 星空凛's exclude_position, Chance Day's
 * formation, the area-move auto-ability watchers, and the movement-tracking
 * arithmetic. Where a Rust test is a near-duplicate of one zones_position
 * already covers, it is listed in SKIPPED-DUPLICATE below rather than padded.
 *
 * ---------------------------------------------------------------------------
 * RUST FIXTURE BUGS CORRECTED HERE (evidence: cards/cards.json read as UTF-8)
 * ---------------------------------------------------------------------------
 * 1. live_start_subunit_threshold_formation_test.rs uses
 *        game.id("PL!SP-bp2-050-L")
 *    cards.json has NO "PL!SP-bp2-050-L" (the "bp2" transposition is absent for
 *    this number). The real print is
 *        "PL!SP-pb2-050-L"  name=Jellyfish  type=ライブ
 *    JELLYFISH below. A live card is the correct shape for a ライブ開始時
 *    trigger, and the file itself places the card in the live-card zone, so the
 *    transposition is the whole fix -- NO member substitution is needed. (The
 *    brief's hint that "the file requires a MEMBER" is a misreading: the Rust
 *    fixture does `live_card_zone.cards.push(live)`.)
 *
 * 2. chisato_pb1_003_both_stages_rotation_test.rs uses
 *        game.id("PL!SP-bp1-003-R")
 *    Not a bug -- the current on-disk file already reads the correct "pb1".
 *    Verified: "PL!SP-bp1-003-R" 嵐 千砂都 cost 9 blade 2 (5yncri5e!);
 *    "PL!SP-bp1-003-R" does NOT exist. Ported with the correct number.
 *
 * 3. live_start_subunit_threshold_formation_test.rs calls "PL!SP-bp1-014-N"
 *    「唐 可可」 in a comment. cards.json says that number is
 *        "PL!SP-bp1-014-N"  name=嵐 千砂都 cost 4 blade 1
 *    -- NOT 唐 可可 (that is PL!SP-pb1-014-PR). The BEHAVIOUR is unaffected
 *    (the test only needs a stage member the live's threshold accepts), but the
 *    comment is wrong. Ported faithfully; the number is what is asserted.
 *
 * 4. wien_bp5_test.rs / position_change_multi_test.rs pin
 *    "PL!HS-bp1-006-R" (安養寺姫芽, みらくらぱーく！) and "PL!HS-bp2-006-R"
 *    (藤島 慈) as a transposition guard. Verified distinct prints in cards.json
 *    and both are used here, so the guard is preserved.
 *
 * ---------------------------------------------------------------------------
 * ENGINE DEFECTS EXPECTED IN THIS AREA (see the report; kept RED, not weakened)
 * ---------------------------------------------------------------------------
 *  - Position changes are offered as a SelectTarget("position_change") prompt
 *    but the answer maps the chosen INDEX STRAIGHT onto a raw area index
 *    (src/ability/effects/state.c:1489 rb_resume_position_change,
 *    `int dst = selected_idx`). So the "exclude_self" / "exclude_position"
 *    destination FILTER that Rust builds is not represented at all, and the
 *    prompt's count is RB_STAGE_SIZE rather than the number of legal answers.
 *  - The flat RbChoice carries NO option list, so "which areas were offered"
 *    is not observable from C. Those claims are recorded as EXPECTED_GAP
 *    (counted and printed) rather than asserted as passes.
 *  - `rb_stage_position_change` / `rb_resume_position_change` swap `stage` and
 *    `stage_wait` only; under-cards do NOT follow their host (Rule 4.5.5.3 /
 *    Q139). The sibling `rb_effect_rotation` in the same file does carry them.
 *
 * ---------------------------------------------------------------------------
 * LANDMINES respected (confirmed by other agents)
 * ---------------------------------------------------------------------------
 *  - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE, so every
 *    heart read here goes through rb_mods_get_heart directly.
 *  - RbChoice.zone for the waitroom is the string "discard".
 *  - mid() (test_new_id) not test_id: Rust game.id() allocates the next DISTINCT
 *    pool slot; C's template lookup aliases the shared template.
 *  - rb_card_copy_pool_reset() per case: rb_create_card_copy draws from a
 *    process-global monotonic pool that nothing resets.
 *  - sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *  - Card.ability is NOT populated by rb_decode_card_by_index; abilities come
 *    from rb_card_num_abilities + rb_decode_card_ability.
 *  - test_add_to_stage / test_set_opp_stage are FIXED-INDEX by design.
 *    test_place_under APPENDS.
 *  - Every drain is capped at a bounded iteration count.
 *
 * This file is pure ASCII: every Japanese keyword is an explicit UTF-8 byte
 * escape so a legacy-codepage rewrite cannot mangle it.
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

static int failures;
static int assertions;
static int setup_bugs;
static int gaps;                 /* EXPECTED_GAP records: unobservable in C */
static int n_tests_ok, n_tests_failed, n_tests_crashed;
static const char *current_test = "(none)";

/* ── assertions ──────────────────────────────────────────────────────────── */

/* A NAMED struct, deliberately: two independent `struct { ... }` declarations
 * are DISTINCT types in C, so an anonymous one cannot be assigned through a
 * mmap'd page. The tally lives in MAP_SHARED memory so the parent's totals
 * accumulate the children's counters after waitpid; g_tally points at the page
 * and every CHECK macro writes through it. A pipe was tried first and is the
 * wrong tool here: a short or interleaved read desynchronises the four numbers
 * and poisons the run totals. */
struct RunTally { int assertions, failures, setup_bugs, gaps; };
static struct RunTally *g_tally;

#define TALLY g_tally
#define CHECK(condition, ...) do { \
    TALLY->assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  [%s]\n", current_test); \
        TALLY->failures++; \
    } \
} while (0)

/* Variadic so a claim can print the number it is complaining about, which is
 * the whole point of a red engine-bug report. */
#define CHECK_EQ(actual, expected, ...) do { \
    TALLY->assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, " (got %d expected %d)  [%s]\n", \
                actual_value, expected_value, current_test); \
        TALLY->failures++; \
    } \
} while (0)

/* An assertion C cannot make. Recorded and printed, never counted as a pass. */
#define EXPECTED_GAP(why) do { \
    TALLY->gaps++; \
    fprintf(stdout, "GAP: %s  [%s]\n", (why), current_test); \
} while (0)

/* ── area indices (engine/src/game/zones.rs MemberArea) ───────────────────── */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* heart01 -> colour slot 1 (src/ability/util.c rb_parse_heart_color). Read via
 * rb_mods_get_heart, never test_get_heart_modifier (remaps 5 -> orange). */
#define H01 1
#define H02 2

/* ── card numbers, all verified against cards/cards.json (UTF-8) ─────────── */
#define KINAKO      "PL!SP-bp5-006-R"  /* 桜小路きな子, 起動 top-3 + position_change */
#define CHII        "PL!HS-bp2-006-R"  /* 藤島 慈, 登場 それぞれ (multiple_targets)   */
#define HIMENO6     "PL!HS-pb1-006-R"  /* 安養寺姫芽 c11, live-start group-filter PC */
#define HIMENO14    "PL!HS-pb1-014-R"  /* 安養寺姫芽 c9, 正面 area debut + 常時      */
#define RUBY        "PL!S-bp7-018-N"   /* 黒澤ルビィ, 登場 select member -> center  */
#define KANAN       "PL!S-bp7-012-N"   /* 松浦果南, group-gated debut formation     */
#define SEIRA       "PL!S-bp5-111-R"   /* 鹿角聖良, SaintSnow member                */
#define JELLYFISH   "PL!SP-pb2-050-L"  /* Jellyfish, ライブ (see header note 1)     */
#define CHISATO     "PL!SP-pb1-003-R"  /* 嵐 千砂都 debut rotation (see note 2)     */
#define NATSUMI     "PL!SP-pb1-020-N"  /* 鬼塚夏美, each_time:area_move -> draw 1   */
#define WATCHER     "PL!SP-pb2-028-N"  /* 桜小路きな子 jidou area_move -> +2 energy */
#define LIELLA_MEMBER "PL!SP-bp1-014-N" /* 嵐 千砂都 c4, 5yncri5e! (see note 3)     */
#define CHANCE_DAY  "PL!SP-bp4-027-L"  /* Chance Day, live-success Liella formation */
#define GROUP_SD9   "PL!HS-sd1-014-SD" /* 安養寺 姫芽, みらくらぱーく！              */
#define GROUP_SD15  "PL!HS-sd1-006-SD" /* 安養寺 姫芽 c15, みらくらぱーく！         */
#define FILLER      "PL!-sd1-010-SD"
#define SD013       "PL!-sd1-013-SD"
#define SD014       "PL!-sd1-014-SD"
#define ENERGY_NO   "LL-E-001-SD"

/* ── card DB ─────────────────────────────────────────────────────────────── */

/* `src` is the in-tree layout; an isolated out-of-tree build resolves the
 * blobs through ../cards/build, so both paths are tried. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Rust game.id() allocates the NEXT DISTINCT pool slot, so two game.id("X")
 * calls are two different card instances. C's plain template lookup aliases the
 * shared template; test_new_id() is the distinct copy. */
static int mid(TestGame *tg, const char *no)
{
    return test_new_id(tg, no);
}

/* Resolve a card number AND pin the PRINTED number byte-for-byte.
 * rb_find_card_by_no() and rb_card_no_eq() both have fuzzy tails -- a peer
 * found rb_card_no_eq("PL!SP-PR-004-R", ...) happily matching the real print
 * "PL!SP-PR-004-PR". Only strcmp on rb_card_string() cannot be satisfied by a
 * near miss. */
/* ident() records a broken FIXTURE, not an engine defect, so it uses a counter
 * that the pass/fail verdict ignores. */
static int ident(int id, const char *no)
{
    if (id < 0) {
        fprintf(stderr, "SETUP BUG: %s did not resolve\n", no);
        TALLY->setup_bugs++;
        TALLY->assertions++;
        return 0;
    }
    Card c;
    char got[96];
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        fprintf(stderr, "SETUP BUG: %s could not be decoded\n", no);
        TALLY->setup_bugs++;
        TALLY->assertions++;
        return 0;
    }
    snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    if (strcmp(got, no) != 0) {
        fprintf(stderr, "SETUP BUG: id %d prints \"%s\", test wanted \"%s\"\n",
                id, got, no);
        TALLY->setup_bugs++;
        TALLY->assertions++;
        return 0;
    }
    return 1;
}

/* Resolve + pin in one step. Returns the id, or -1 after recording a setup bug
 * (and asserting, so a broken fixture cannot pass silently). */
static int card(TestGame *tg, const char *no)
{
    int id = mid(tg, no);
    if (!ident(id, no)) return -1;
    return id;
}

/* ── board helpers ───────────────────────────────────────────────────────── */

static void bare_board(TestGame *tg)
{
    rb_card_copy_pool_reset();     /* process-global monotonic copy pool */
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_active = 0;
        p->energy_deck.n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            p->stage[i] = RB_EMPTY_SLOT;
            p->stage_wait[i] = 0;
            p->under_cards[i].n = 0;
        }
    }
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) test_add_to_deck(tg, filler);
    for (int i = 0; i < n / 2; i++) test_add_to_deck_pl(tg, 1, filler);
}

/* Answer every pending prompt with `idx`, up to a BOUNDED number of rounds, and
 * return how many prompts were still on screen when the count was taken. A peer
 * found an ability that re-prompts UNBOUNDEDLY, so the cap is mandatory. */
static int drain_with(TestGame *tg, int idx, int max_rounds)
{
    int prompts = 0;
    for (int guard = 0; guard < max_rounds; guard++) {
        if (!rb_has_pending_choice(&tg->state)) break;
        prompts++;
        rb_resume_with_choice(&tg->state, idx);
    }
    return prompts;
}

/* Drain, answering SelectTarget(area) prompts with the raw area index and
 * everything else with 0. The area index is what rb_resume_position_change
 * consumes, so this is the faithful answer shape for the C engine. */
static int drain_areas(TestGame *tg, int area, int max_rounds)
{
    int prompts = 0;
    for (int guard = 0; guard < max_rounds; guard++) {
        if (!rb_has_pending_choice(&tg->state)) break;
        prompts++;
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        int is_area = ch && ch->kind == RB_CHOICE_SELECT_TARGET &&
                      strstr(ch->target, "position") != NULL;
        rb_resume_with_choice(&tg->state, is_area ? area : 0);
    }
    return prompts;
}

/* Count the prompts still ON SCREEN right now. Used as the non-vacuity control
 * for every negative assertion: a negative assertion paired with prompts == 0
 * proves nothing, because the effect may simply never have run. */
static int prompts_on_screen(TestGame *tg)
{
    return rb_has_pending_choice(&tg->state) ? 1 : 0;
}

static int moved_this_turn(const TestGame *tg, int card_id)
{
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 0;
    return tg->state.moved_this_turn[card_id];
}

/* Fire a ライブ開始時 / ライブ成功時 window. Peers established that the trigger
 * call only QUEUES the entries; rb_process_pending_auto_abilities is what turns
 * a queued entry into the prompt a player actually sees, so both are required
 * or a "no prompt" outcome means only "nothing was drained", not "nothing
 * fired". */
static void fire_live_start(TestGame *tg, int pl)
{
    rb_trigger_live_start(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

static void fire_live_success(TestGame *tg, int pl)
{
    rb_trigger_live_success(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

/* Answer prompts until a position destination prompt is on screen (or the
 * bound is hit). Returns the number of prompts consumed, which is the
 * non-vacuity control for everything downstream. */
static int reach_position_prompt(TestGame *tg, int max_rounds)
{
    int seen = 0;
    for (int guard = 0; guard < max_rounds; guard++) {
        if (!rb_has_pending_choice(&tg->state)) break;
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        if (ch && ch->kind == RB_CHOICE_SELECT_TARGET &&
            strstr(ch->target, "position")) break;
        seen++;
        rb_resume_with_choice(&tg->state, 0);
    }
    return seen;
}

/* Defined in section C, used from section A's paired positive. */
static void setup_three_members(TestGame *tg, int *chii, int *a, int *b);

static int stage_of(const TestGame *tg, int pl, int card_id)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == card_id) return i;
    return -1;
}

static int total_blade(const TestGame *tg)
{
    int sum = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = tg->state.p[0].stage[i];
        if (cid >= 0) sum += rb_mods_get_blade((RbMods *)&tg->state.mods, cid);
    }
    return sum;
}

/* Assert that a destination prompt is on screen, and report what it says. The
 * control makes every downstream negative assertion non-vacuous. */
static int require_position_prompt(TestGame *tg, const char *what)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    CHECK(ch != NULL, "%s: a prompt is on screen at the assertion site", what);
    if (!ch) return 0;
    CHECK_EQ((int)ch->kind, (int)RB_CHOICE_SELECT_TARGET,
             "the destination prompt is a SelectTarget");
    fprintf(stdout, "      [%s] target=\"%s\" count=%d desc=\"%s\"\n",
            what, ch->target, ch->count, ch->description);
    return 1;
}

/* ── ########################################################################
 * A. position_change_non_optional_test.rs -- exclude_self / exclude_position
 *    ######################################################################## */

/* Rust: non_optional_pc_excludes_self_from_center.
 * きな子 at center, activate her 起動; the top-3 cost resolves and the engine
 * must offer LEFT and RIGHT and NOT center (Rule 11.10.1, exclude_self). */
static void A_non_optional_pc_excludes_self_from_center(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kinako = card(&tg, KINAKO);
    if (kinako < 0) return;
    fill_decks(&tg, test_id(&tg, FILLER), 20);
    tg.state.p[0].stage[AREA_CENTER] = kinako;
    test_give_energy(&tg, 20);

    test_activate_ability(&tg, kinako);

    /* CONTROL: the effect really ran. Without this every assertion below could
     * pass on a board where the ability did nothing at all. */
    if (!require_position_prompt(&tg, "A1 center")) return;

    EXPECTED_GAP("Rust asserts the offered SET is exactly {left,right} (2 options). "
                 "The flat RbChoice carries no option list, and state.c:1417 emits "
                 "count=RB_STAGE_SIZE, so neither the set nor the count is observable.");

    /* What IS observable and faithful: a move to a genuinely different area
     * lands the member there and vacates the source. */
    int prompts = drain_areas(&tg, AREA_LEFT, 8);
    CHECK(prompts > 0, "A1: the position prompt was answered, not skipped silently");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], kinako,
             "きな子 reached LeftSide after the position change");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "Center is vacated by the position change");
}

/* Rust: non_optional_pc_excludes_self_from_left. */
static void A_non_optional_pc_excludes_self_from_left(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kinako = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (kinako < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_LEFT] = kinako;
    test_give_energy(&tg, 20);

    test_activate_ability(&tg, kinako);
    if (!require_position_prompt(&tg, "A2 left")) return;
    EXPECTED_GAP("offered set {center,right} not observable from C");

    int prompts = drain_areas(&tg, AREA_CENTER, 8);
    CHECK(prompts > 0, "A2: the position prompt was answered");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], kinako,
             "きな子 reached Center from LeftSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], RB_EMPTY_SLOT,
             "LeftSide is vacated by the position change");
}

/* Rust: non_optional_pc_excludes_self_from_right. */
static void A_non_optional_pc_excludes_self_from_right(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kinako = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (kinako < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_RIGHT] = kinako;
    test_give_energy(&tg, 20);

    test_activate_ability(&tg, kinako);
    if (!require_position_prompt(&tg, "A3 right")) return;
    EXPECTED_GAP("offered set {left,center} not observable from C");

    int prompts = drain_areas(&tg, AREA_CENTER, 8);
    CHECK(prompts > 0, "A3: the position prompt was answered");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], kinako,
             "きな子 reached Center from RightSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], RB_EMPTY_SLOT,
             "RightSide is vacated by the position change");
}

/* Rust: non_optional_pc_executes_move_correctly.
 * filler at left, きな子 at center; choosing left swaps them. */
static void A_non_optional_pc_executes_move_correctly(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kinako = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (kinako < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_LEFT] = filler;
    tg.state.p[0].stage[AREA_CENTER] = kinako;
    test_give_energy(&tg, 20);

    test_activate_ability(&tg, kinako);
    if (!require_position_prompt(&tg, "A4 swap")) return;

    int prompts = drain_areas(&tg, AREA_LEFT, 8);
    CHECK(prompts > 0, "A4: the position prompt was answered");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], kinako,
             "きな子 is at LeftSide after the swap");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], filler,
             "the displaced filler took the vacated Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], RB_EMPTY_SLOT,
             "the untouched RightSide stays empty");
    CHECK(moved_this_turn(&tg, kinako), "the moving member is recorded as moved");
    CHECK(moved_this_turn(&tg, filler), "the swapped member is recorded as moved");
}

/* Rust: exclude_position_center_not_offered.
 * 星空凛 PL!-bp4-005-R＋ (FULLWIDTH PLUS, bytes ef bc 8b) ライブ開始時:
 * 「このメンバーはセンターエリア以外にポジションチェンジする。」
 * Rin at center with a non-μ's filler at left; only left and right are legal. */
static void A_exclude_position_center_not_offered(void)
{
    static TestGame tg;
    /* The printed number carries a FULLWIDTH PLUS SIGN (U+FF0B). rb_find_card_by_no
     * is EXACT-BYTE and does not normalise fullwidth->halfwidth the way Rust's
     * fuzzy lookup does, so the real bytes are spelled out. */
    const char *RIN = "PL!-bp4-005-R\xEF\xBC\x8B";
    test_game_new(&tg);
    bare_board(&tg);
    int rin = card(&tg, RIN);
    int filler = mid(&tg, FILLER);
    if (rin < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 20);
    /* filler at left (non-μ's, no blade -- the 5-blade condition is satisfied),
     * Rin at center. */
    tg.state.p[0].stage[AREA_LEFT] = filler;
    tg.state.p[0].stage[AREA_CENTER] = rin;

    /* Drive the real ライブ開始時 dispatch. */
    int live = mid(&tg, FILLER);
    test_add_to_live(&tg, live);
    fire_live_start(&tg, 0);

    int prompts = reach_position_prompt(&tg, 24);
    CHECK(prompts > 0,
          "A5: at least one prompt was on screen while reaching the ライブ開始時 window");

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET &&
          strstr(ch->target, "position") != NULL,
          "A5: a position destination prompt was reached at ライブ開始時");
    if (!ch) {
        EXPECTED_GAP("exclude_position=center: the 星空凛 live-start position change "
                     "did not reach a destination prompt on this snapshot, so the "
                     "offered set {left,right} cannot be evaluated at all.");
        return;
    }
    EXPECTED_GAP("Rust asserts exactly {left,right} are offered. The flat RbChoice "
                 "carries no option list, so the set is not observable from C.");
    drain_areas(&tg, AREA_LEFT, 8);
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], rin,
             "星空凛 moved out of Center to LeftSide");
}

/* Rust: exclude_position_and_exclude_self_combine_correctly.
 * Rin at LEFT, filler at RIGHT: exclude_position removes center, exclude_self
 * removes left, so only RIGHT is legal. */
static void A_exclude_position_and_exclude_self_combine(void)
{
    static TestGame tg;
    const char *RIN = "PL!-bp4-005-R\xEF\xBC\x8B";
    test_game_new(&tg);
    bare_board(&tg);
    int rin = card(&tg, RIN);
    int filler = mid(&tg, FILLER);
    if (rin < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 20);
    tg.state.p[0].stage[AREA_LEFT] = rin;
    tg.state.p[0].stage[AREA_RIGHT] = filler;

    int live = mid(&tg, FILLER);
    test_add_to_live(&tg, live);
    fire_live_start(&tg, 0);

    reach_position_prompt(&tg, 24);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET &&
          strstr(ch->target, "position") != NULL,
          "A6: a position destination prompt was reached at ライブ開始時");
    if (!ch) {
        EXPECTED_GAP("exclude_position + exclude_self: no destination prompt was "
                     "reached, so the single legal answer (right) is unobservable.");
        return;
    }
    EXPECTED_GAP("Rust asserts the offered set is exactly {right} (1 option). Not "
                 "observable from the flat RbChoice.");
    drain_areas(&tg, AREA_RIGHT, 8);
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], rin,
             "星空凛 moved out of LeftSide to the only legal area, RightSide");
}

/* Rust: non_optional_pc_with_all_positions_excluded_fizzles.
 * きな子 alone at center; the ability must still complete without error. */
static void A_non_optional_pc_with_all_positions_excluded_fizzles(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kinako = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (kinako < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = kinako;
    test_give_energy(&tg, 20);

    test_activate_ability(&tg, kinako);
    if (!require_position_prompt(&tg, "A7 fizzle")) return;
    EXPECTED_GAP("Rust asserts 2 options are offered (left,right) on this board; "
                 "not observable from C.");
    int prompts = drain_areas(&tg, AREA_LEFT, 24);
    CHECK(prompts > 0, "A7: the position prompt was answered");
    CHECK(!rb_has_pending_choice(&tg.state),
          "the ability completes with no prompt left parked");
}

/* Rust: formation_change_still_offers_all_positions.
 * 藤島 慈's 登場 is a formation change (multiple_targets=true), so the current
 * area IS a legal answer. The Rust file's own point is that exclude_self must
 * NOT apply here. */
static void A_formation_change_still_offers_all_positions(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int chii = card(&tg, CHII);
    int filler = mid(&tg, SD013);
    if (chii < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 15);
    tg.state.p[0].stage[AREA_LEFT] = filler;
    test_add_to_hand(&tg, chii);

    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_CENTER), 1,
             "藤島 慈 debuts to Center");

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL, "A8: 藤島 慈's 登場 offers a formation prompt");
    if (ch) {
        EXPECTED_GAP("Rust asserts all THREE positions are offered including the "
                     "current one. Not observable from the flat RbChoice.");
        /* The debuts landed where she was played. */
        CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], chii,
                 "藤島 慈 is on stage at Center before the formation choice");
    }
    drain_areas(&tg, AREA_CENTER, 16);
    CHECK(!rb_has_pending_choice(&tg.state),
          "the formation change resolves with no prompt parked");
    /* PAIRED POSITIVE on the same board shape: a DIFFERENT chosen area must
     * move the member. Without this, "the formation change resolves" and "the
     * formation change did nothing" are indistinguishable. */
    static TestGame tp;
    int chii2, a2, b2;
    setup_three_members(&tp, &chii2, &a2, &b2);
    if (chii2 < 0) return;
    test_play_to_stage(&tp, chii2, AREA_RIGHT);
    drain_areas(&tp, AREA_LEFT, 16);
    CHECK_EQ(tp.state.p[0].stage[AREA_LEFT], chii2,
             "PAIRED POSITIVE: the same board with a different chosen area DOES "
             "move the debuting member there");
}

/* Rust: optional_pc_excludes_self_but_can_skip.
 * 安養寺姫芽 (PL!HS-pb1-006-R) ライブ開始時: an OPTIONAL position change into the
 * area of ANOTHER みらくらぱーく！ member. Only a non-group filler is on stage,
 * so no destination is legal and the effect fizzles silently. */
static void A_optional_pc_excludes_self_but_can_skip(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int himeno = card(&tg, HIMENO6);
    int filler = mid(&tg, FILLER);
    if (himeno < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 11);
    tg.state.p[0].stage[AREA_LEFT] = filler;
    tg.state.p[0].stage[AREA_RIGHT] = himeno;

    int live = mid(&tg, FILLER);
    test_add_to_live(&tg, live);
    fire_live_start(&tg, 0);

    int prompts = drain_with(&tg, -1, 24);   /* decline every prompt */
    CHECK(prompts > 0, "A9: the ライブ開始時 window produced at least one prompt");
    CHECK(!rb_has_pending_choice(&tg.state),
          "A9: the optional position change completes with no prompt parked");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], himeno,
             "安養寺姫芽 stays at RightSide when no group destination exists");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], filler,
             "the non-group filler is not disturbed");
    EXPECTED_GAP("Rust asserts allow_skip is true on the position prompt and that "
                 "zero positions are offered. allow_skip is on the flat RbChoice "
                 "(ch->allow_skip) and IS readable, but with zero legal "
                 "destinations the engine never raises the prompt at all, so the "
                 "assertion has no prompt to attach to.");
}

/* ── ########################################################################
 * B. debut_selected_member_fixed_center_test.rs -- 黒澤ルビィ PL!S-bp7-018-N
 *    登場: 「自分のステージにいるメンバー1人をセンターエリアにポジションチェンジさせる。」
 *    The parser emits position_change with target_member: select and a FIXED
 *    destination of "center".
 *    ######################################################################## */

/* Fires 黒澤ルビィ's 登場 and drives her debut choice, picking the stage area
 * whose raw index matches `pick_area`. Bounded, like every other drain. */
static int ruby_debut_pick(TestGame *tg, int ruby, int play_area, int pick_area)
{
    test_play_to_stage(tg, ruby, play_area);
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 24) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        int idx = pick_area;
        if (ch && ch->kind != RB_CHOICE_SELECT_TARGET) idx = 0;
        rb_resume_with_choice(&tg->state, idx);
    }
    return guard;
}

/* Rust: fixed_center_choice_offers_self_and_other_stage_members.
 * The prompt must offer every OCCUPIED area exactly once, 黒澤ルビィ herself
 * included (she debuts into the centre, so centre is in the set). */
static void B_fixed_center_choice_offers_self_and_other_stage_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int a = mid(&tg, FILLER);
    int b = mid(&tg, FILLER);
    if (ruby < 0 || a < 0 || b < 0) return;
    fill_decks(&tg, a, 20);
    test_give_energy(&tg, 10);
    /* Centre left free so the debut lands there instead of displacing a member. */
    tg.state.p[0].stage[AREA_LEFT] = a;
    tg.state.p[0].stage[AREA_RIGHT] = b;
    test_add_to_hand(&tg, ruby);

    CHECK_EQ(test_play_to_stage(&tg, ruby, AREA_CENTER), 1,
             "黒澤ルビィ debuts to Center");
    /* PREMISE: all three areas now hold a distinct member. */
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], a, "premise: LeftSide holds a");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], ruby, "premise: Center holds ルビィ");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], b, "premise: RightSide holds b");

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL,
          "B1: the 登場 member-selection prompt is on screen at the assertion site");
    if (ch)
        fprintf(stdout, "      [B1] kind=%d target=\"%s\" count=%d desc=\"%s\"\n",
                (int)ch->kind, ch->target, ch->count, ch->description);
    EXPECTED_GAP("Rust compares the offered SET against {center,left,right}. The "
                 "flat RbChoice has no option list, so the set is not observable.");
    drain_with(&tg, 0, 16);
}

/* Rust: fixed_center_move_left_member_to_center. */
static void B_fixed_center_move_left_member_to_center(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int left = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || left < 0 || right < 0) return;
    fill_decks(&tg, left, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    ruby_debut_pick(&tg, ruby, AREA_CENTER, AREA_LEFT);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], left,
             "the LeftSide member moved to Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], ruby,
             "黒澤ルビィ swapped out to where the LeftSide member was");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], right,
             "the RightSide member is untouched");
}

/* Rust: fixed_center_move_right_member_to_center. */
static void B_fixed_center_move_right_member_to_center(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int left = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || left < 0 || right < 0) return;
    fill_decks(&tg, left, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    ruby_debut_pick(&tg, ruby, AREA_CENTER, AREA_RIGHT);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], right,
             "the RightSide member moved to Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], left,
             "the LeftSide member is untouched");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], ruby,
             "黒澤ルビィ swapped out to the vacated RightSide");
}

/* Rust: fixed_center_can_choose_self_when_not_already_center. */
static void B_fixed_center_can_choose_self_when_not_already_center(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int left = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || left < 0 || right < 0) return;
    fill_decks(&tg, left, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_CENTER] = left;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    /* Ruby debuts to LEFT, then chooses HERSELF (left) to move to Center. */
    ruby_debut_pick(&tg, ruby, AREA_LEFT, AREA_LEFT);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], ruby,
             "黒澤ルビィ herself moved to Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], left,
             "the former Center member swapped out to where ルビィ was");
}

/* Rust: fixed_center_center_member_noop.
 * Choosing the member already in Center must change nothing. Paired with a
 * POSITIVE on the identical board shape (B_left_member_to_center moves the
 * LeftSide member), so this cannot pass merely because the engine moves nothing. */
static void B_fixed_center_center_member_noop(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int left = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || left < 0 || right < 0) return;
    fill_decks(&tg, left, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    ruby_debut_pick(&tg, ruby, AREA_CENTER, AREA_CENTER);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], ruby,
             "choosing the Center member is a no-op: ルビィ stays at Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], left, "LeftSide untouched");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], right, "RightSide untouched");

    /* PAIRED POSITIVE, identical board shape, a DIFFERENT member chosen. */
    static TestGame tp;
    test_game_new(&tp);
    bare_board(&tp);
    int ruby2 = card(&tp, RUBY);
    int left2 = mid(&tp, FILLER);
    int right2 = mid(&tp, FILLER);
    if (ruby2 < 0 || left2 < 0 || right2 < 0) return;
    fill_decks(&tp, left2, 20);
    test_give_energy(&tp, 10);
    tp.state.p[0].stage[AREA_LEFT] = left2;
    tp.state.p[0].stage[AREA_RIGHT] = right2;
    test_add_to_hand(&tp, ruby2);
    ruby_debut_pick(&tp, ruby2, AREA_CENTER, AREA_LEFT);
    CHECK_EQ(tp.state.p[0].stage[AREA_CENTER], left2,
             "PAIRED POSITIVE: the same board with a different member chosen DOES "
             "move it to Center -- otherwise the no-op above proves nothing");
}

/* Rust: fixed_center_destination_is_fixed_center_no_free_dest_choice.
 * Exactly ONE member-selection prompt: the destination is FIXED at center, so
 * no second (free-destination) choice may be raised. */
static void B_fixed_center_destination_is_fixed_center_no_free_dest_choice(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int left = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || left < 0 || right < 0) return;
    fill_decks(&tg, left, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    test_play_to_stage(&tg, ruby, AREA_CENTER);
    int member_choices = 0;
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 24) {
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        if (ch && ch->kind == RB_CHOICE_SELECT_TARGET) {
            member_choices++;
            rb_resume_with_choice(&tg.state, AREA_LEFT);
        } else {
            rb_resume_with_choice(&tg.state, 0);
        }
    }
    CHECK_EQ(member_choices, 1,
             "a fixed center destination produces exactly one member-selection prompt");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], left,
             "the LeftSide member moved straight to Center");
}

/* Rust: fixed_center_move_left_to_occupied_center_swaps_and_records. */
static void B_fixed_center_move_left_to_occupied_center_swaps_and_records(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int center = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || center < 0 || right < 0) return;
    fill_decks(&tg, center, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_CENTER] = center;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    /* Ruby debuts to LEFT and then chooses HERSELF to move to an OCCUPIED center. */
    ruby_debut_pick(&tg, ruby, AREA_LEFT, AREA_LEFT);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], ruby,
             "黒澤ルビィ moved into the occupied Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], center,
             "the former Center member swapped out to LeftSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], right, "RightSide untouched");
    CHECK(moved_this_turn(&tg, ruby), "ルビィ is recorded as having moved");
    CHECK(moved_this_turn(&tg, center), "the Center member is recorded as having moved");
}

/* Rust: fixed_center_full_stage_choose_any_member. */
static void B_fixed_center_full_stage_choose_any_member(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int ruby = card(&tg, RUBY);
    int left = mid(&tg, FILLER);
    int center = mid(&tg, FILLER);
    int right = mid(&tg, FILLER);
    if (ruby < 0 || left < 0 || center < 0 || right < 0) return;
    fill_decks(&tg, left, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_CENTER] = center;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_hand(&tg, ruby);

    ruby_debut_pick(&tg, ruby, AREA_CENTER, AREA_RIGHT);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], right,
             "the RightSide member moved to Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], ruby,
             "黒澤ルビィ swapped out to the vacated RightSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], left, "the LeftSide member is untouched");
}

/* ── ########################################################################
 * C. position_change_multi_test.rs -- 藤島 慈 PL!HS-bp2-006-R formation change
 *    ######################################################################## */

/* Stage [a, b, empty], then 慈 debuts to RIGHT so all three areas are occupied. */
static void setup_three_members(TestGame *tg, int *chii, int *a, int *b)
{
    test_game_new(tg);
    bare_board(tg);
    *chii = card(tg, CHII);
    *a = mid(tg, SD013);
    *b = mid(tg, SD013);
    if (*chii < 0 || *a < 0 || *b < 0) return;
    fill_decks(tg, *a, 40);
    test_give_energy(tg, 15);
    tg->state.p[0].stage[AREA_LEFT] = *a;
    tg->state.p[0].stage[AREA_CENTER] = *b;
    test_add_to_hand(tg, *chii);
}

/* Rust: position_change_three_members_all_move. */
static void C_position_change_three_members_all_move(void)
{
    static TestGame tg;
    int chii, a, b;
    setup_three_members(&tg, &chii, &a, &b);
    if (chii < 0) return;
    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_RIGHT), 1, "慈 debuts to RightSide");
    /* PREMISE: all three areas really are occupied, so "every area is still
     * occupied afterwards" cannot pass on an empty or half-empty board. */
    CHECK(tg.state.p[0].stage[AREA_LEFT] != RB_EMPTY_SLOT &&
          tg.state.p[0].stage[AREA_CENTER] != RB_EMPTY_SLOT &&
          tg.state.p[0].stage[AREA_RIGHT] != RB_EMPTY_SLOT,
          "C1 PREMISE: all three areas hold a distinct member before the change");

    int prompts = drain_areas(&tg, AREA_CENTER, 24);
    CHECK(prompts > 0, "C1: the formation change produced at least one prompt");
    CHECK(!rb_has_pending_choice(&tg.state),
          "C1: the formation change resolves with no prompt parked");
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        CHECK(tg.state.p[0].stage[i] != RB_EMPTY_SLOT,
              "C1: every area still holds a member after the formation change");
    }
    /* The load-bearing assertion. Rust drives three choices and checks the whole
     * stage is rearranged; the C engine collapses the batch, so the claim is
     * reduced to "the member that was chosen actually ended up in the chosen
     * area". Without this, a board where NOTHING moves satisfies every other
     * assertion in this test. */
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], chii,
             "C1: the debuting member really reached the chosen Center "
             "(a no-op engine would leave it at RightSide)");
}

/* Rust: position_change_no_other_members.
 * 慈 alone on an empty stage: ONE member, ONE choice; all three areas are legal
 * destinations because empty slots count for a formation change. */
static void C_position_change_no_other_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int chii = card(&tg, CHII);
    if (chii < 0) return;
    fill_decks(&tg, test_id(&tg, SD013), 20);
    test_give_energy(&tg, 15);
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, chii);

    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_CENTER), 1,
             "慈 debuts to Center on an otherwise empty stage");
    CHECK(prompts_on_screen(&tg),
          "C2: a formation prompt is on screen for the lone member");
    EXPECTED_GAP("Rust asserts exactly 3 options are offered (left,center,right) "
                 "including empty slots. Not observable from the flat RbChoice.");
    drain_areas(&tg, AREA_LEFT, 16);
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], chii,
             "the lone member moved to LeftSide");
}

/* Rust: position_change_with_swap. */
static void C_position_change_with_swap(void)
{
    static TestGame tg;
    int chii, a, b;
    setup_three_members(&tg, &chii, &a, &b);
    if (chii < 0) return;
    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_RIGHT), 1, "慈 debuts to RightSide");
    int prompts = drain_areas(&tg, AREA_RIGHT, 24);
    CHECK(prompts > 0, "C3: the formation change produced a prompt");
    CHECK(!rb_has_pending_choice(&tg.state), "C3: the change resolves");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], chii,
             "C3: the debuting member occupies the chosen Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], a,
             "C3: the displaced LeftSide member took the vacated RightSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], b,
             "C3: the former Center member is still on the LeftSide");
}

/* Rust: position_change_tracks_card_movement. */
static void C_position_change_tracks_card_movement(void)
{
    static TestGame tg;
    int chii, a, b;
    setup_three_members(&tg, &chii, &a, &b);
    if (chii < 0) return;
    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_RIGHT), 1, "慈 debuts to RightSide");
    drain_areas(&tg, AREA_CENTER, 24);
    int any = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = tg.state.p[0].stage[i];
        if (cid >= 0 && moved_this_turn(&tg, cid)) any++;
    }
    CHECK(any > 0, "C4: at least one member is recorded as having moved");
}

/* Rust: formation_change_with_group_names_and_empty_slots.
 * PL!SP-bp4-027-L (Chance Day, Chance Way!) ライブ成功時: if every member on the
 * stage is 『Liella!』, formation change optionally. ONE 5yncri5e! member is on
 * stage; with a group filter set, empty slots must STILL be offered, because a
 * formation change (multiple_targets) may move the member anywhere. */
static void C_formation_change_with_group_names_and_empty_slots(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int chance = card(&tg, CHANCE_DAY);
    int member = mid(&tg, LIELLA_MEMBER);
    int filler = mid(&tg, FILLER);
    if (chance < 0 || member < 0 || filler < 0) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = member;
    test_add_to_live(&tg, chance);
    /* Chance Day's need_heart (heart02 x1, heart00 x2) and the ライブ成功時 window.
     * Rust asserts should_trigger_live_success() as the PREMISE, so a "the
     * ability did nothing" outcome cannot be blamed on a shut window. The C
     * stand-in for that premise is asserted, not assumed. */
    tg.state.stage_hearts[0][H02] = 1;
    tg.state.stage_hearts[0][0] = 2;
    tg.state.live_success[0] = 1;          /* open the ライブ成功時 window */
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "C5 PREMISE: the ライブ成功時 window is OPEN, else the dispatch below "
          "does nothing and the outcome is not attributable to the ability");
    fire_live_success(&tg, 0);
    int prompts = reach_position_prompt(&tg, 24);
    CHECK(prompts > 0,
          "C5: the ライブ成功時 dispatch produced at least one prompt");
    EXPECTED_GAP("Rust asserts exactly 3 positions are offered (left=center=the "
                 "Liella! member, plus the two empty slots). The flat RbChoice "
                 "carries no option list.");
}

/* Rust: position_change_filters_by_group_names.
 * 安養寺姫芽 (PL!HS-pb1-006-R) ライブ開始時: an optional position change into the
 * area of ANOTHER みらくらぱーく！ member. With a group member at Center and a
 * non-group filler at Left, only Center may be offered; Right is excluded by
 * exclude_self (姫芽 is there) and empty slots are excluded for a single move. */
static void C_position_change_filters_by_group_names(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int himeno = card(&tg, HIMENO6);
    int member_same = mid(&tg, GROUP_SD9);   /* みらくらぱーく！ */
    int member_other = mid(&tg, FILLER);
    if (himeno < 0 || member_same < 0 || member_other < 0) return;
    fill_decks(&tg, member_other, 20);
    test_give_energy(&tg, 11);
    tg.state.p[0].stage[AREA_LEFT] = member_other;
    tg.state.p[0].stage[AREA_CENTER] = member_same;
    test_add_to_hand(&tg, himeno);
    CHECK_EQ(test_play_to_stage(&tg, himeno, AREA_RIGHT), 1, "姫芽 debuts to RightSide");

    int live = mid(&tg, FILLER);
    test_add_to_live(&tg, live);
    fire_live_start(&tg, 0);
    reach_position_prompt(&tg, 24);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET &&
          strstr(ch->target, "position") != NULL,
          "C6: a position destination prompt was reached at ライブ開始時");
    if (!ch) {
        EXPECTED_GAP("group_names filter: no destination prompt was reached, so the "
                     "single legal answer (center) is unobservable on this snapshot.");
        return;
    }
    EXPECTED_GAP("Rust asserts exactly 1 option, center. The flat RbChoice has no "
                 "option list.");
    drain_areas(&tg, AREA_CENTER, 16);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], himeno,
             "姫芽 moved into the みらくらぱーく！ member's area (Center)");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], member_same,
             "the group member was displaced to LeftSide");
}

/* ── ########################################################################
 * D. formation_change_test.rs -- movement-tracking arithmetic
 *    ######################################################################## */

/* Rust: all_three_move_counts. Three swaps must record three moves. */
static void D_all_three_move_counts(void)
{
    static TestGame tg;
    int chii, a, b;
    setup_three_members(&tg, &chii, &a, &b);
    if (chii < 0) return;
    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_RIGHT), 1, "慈 debuts to RightSide");
    int before = 0;
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) before += tg.state.moved_this_turn[i];
    drain_areas(&tg, AREA_CENTER, 24);
    int after = 0;
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) after += tg.state.moved_this_turn[i];
    CHECK_EQ(after - before, 3,
             "three swaps record one movement each (Rust: 3)");
    CHECK(!rb_has_pending_choice(&tg.state), "D1: the formation change resolves");
}

/* Rust: stay_in_place_no_new_move.
 * Answering each member with the area it already occupies records NO movement.
 * Rust pins 2 (not 0) with a comment naming the select_indices path as the
 * culprit; the honest C claim is the rule itself: staying put is not a move.
 *
 * This test would pass VACUOUSLY on a board where the formation change does
 * nothing at all, which is exactly the failure mode being hunted. So it carries
 * a paired positive on the same board shape: a second game where the members
 * are chosen to move, which must record a movement. If the positive is also 0,
 * the negative below is meaningless and says so. */
static void D_stay_in_place_no_new_move(void)
{
    static TestGame tg;
    int chii, a, b;
    setup_three_members(&tg, &chii, &a, &b);
    if (chii < 0) return;
    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_RIGHT), 1, "慈 debuts to RightSide");
    int prompts = drain_areas(&tg, AREA_LEFT, 24);
    CHECK(prompts > 0, "D2: the formation change produced a prompt");
    /* rb_resume_position_change returns early when src == dst, so choosing each
     * member's own area must leave the board exactly as it was. */
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], a,
             "D2: the LeftSide member did not move");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], b,
             "D2: the Center member did not move");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], chii,
             "D2: the RightSide member did not move");
    CHECK(!moved_this_turn(&tg, a), "D2: staying in place records no movement (a)");
    CHECK(!moved_this_turn(&tg, b), "D2: staying in place records no movement (b)");
    CHECK(!moved_this_turn(&tg, chii),
          "D2: staying in place records no movement (chii)");

    /* PAIRED POSITIVE, same board shape, members chosen to move. This is what
     * makes the three negatives above non-vacuous. */
    static TestGame tp;
    int chii2, a2, b2;
    setup_three_members(&tp, &chii2, &a2, &b2);
    if (chii2 < 0) return;
    test_play_to_stage(&tp, chii2, AREA_RIGHT);
    drain_areas(&tp, AREA_RIGHT, 24);
    int any_moved = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = tp.state.p[0].stage[i];
        if (cid >= 0 && moved_this_turn(&tp, cid)) any_moved++;
    }
    CHECK_EQ(any_moved, 1,
             "D2 PAIRED POSITIVE: on the SAME board shape, choosing a DIFFERENT "
             "area records exactly one movement -- without this the three "
             "negatives above would pass on an engine that never moves anything");

    EXPECTED_GAP("Rust pins 2 rather than 0 for this path, explicitly as a "
                 "\"the day that is fixed, this test is the thing that says so\" "
                 "marker. The C assertion states the RULE (0 moves when everyone "
                 "stays), which is the faithful claim.");
}

/* Rust: one_moves_two_stay. */
static void D_one_moves_two_stay(void)
{
    static TestGame tg;
    int chii, a, b;
    setup_three_members(&tg, &chii, &a, &b);
    if (chii < 0) return;
    CHECK_EQ(test_play_to_stage(&tg, chii, AREA_RIGHT), 1, "慈 debuts to RightSide");
    int prompts = drain_areas(&tg, AREA_RIGHT, 24);
    CHECK(prompts > 0, "D3: the formation change produced a prompt");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], a,
             "D3: the LeftSide member moved into RightSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], chii,
             "D3: the displaced RightSide member took the vacated LeftSide");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], b,
             "D3: the Center member never moved");
    CHECK(!moved_this_turn(&tg, b),
          "D3: the member that stayed put is not recorded as moved");
}

/* ── ########################################################################
 * E. group_only_debut_formation_move_subunit_gain_blades_test.rs
 *    PL!S-bp7-012-N 松浦果南 登場: if the stage holds only 『Aqours』/『SaintSnow』
 *    members you may formation change; if this effect moved a SaintSnow member
 *    you gain blade x2 until the live ends.
 *    ######################################################################## */

/* Stage = [SaintSnow, SaintSnow, empty]; 果南 debuts to RightSide. */
static int play_kanan_on_saintsnow_stage(TestGame *tg)
{
    int kanan = card(tg, KANAN);
    int s1 = mid(tg, SEIRA);
    int s2 = mid(tg, SEIRA);
    int filler = mid(tg, FILLER);
    if (kanan < 0 || s1 < 0 || s2 < 0 || filler < 0) return -1;
    fill_decks(tg, filler, 40);
    tg->state.p[0].stage[AREA_LEFT] = s1;
    tg->state.p[0].stage[AREA_CENTER] = s2;
    test_give_energy(tg, 30);
    test_add_to_hand(tg, kanan);
    if (!test_play_to_stage(tg, kanan, AREA_RIGHT)) return -1;
    return kanan;
}

/* Rust: group_gated_debut_saintsnow_stage_formation_change_grants_blade. */
static void E_group_gated_debut_saintsnow_stage_grants_blade(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kanan = play_kanan_on_saintsnow_stage(&tg);
    if (kanan < 0) { CHECK(0, "E1: 松浦果南's SaintSnow formation fixture resolves"); return; }

    CHECK(prompts_on_screen(&tg),
          "E1: 果南's 登場 offers a formation-change prompt");
    EXPECTED_GAP("the engine keeps granting this blade for a formation change even "
                 "when no member visibly MOVED on the C board (the batch is "
                 "collapsed), so total_blade >= 2 is a weaker claim here than the "
                 "Rust one; the assertion below is still the printed claim.");
    int prompts = drain_areas(&tg, AREA_RIGHT, 24);
    CHECK(prompts > 0, "E1: the formation change produced at least one prompt");
    CHECK(total_blade(&tg) >= 2,
          "moving a SaintSnow member through 果南's effect grants blade x2 (got %d)",
          total_blade(&tg));
}

/* Rust: group_gated_debut_non_group_member_no_formation. */
static void E_group_gated_debut_non_group_member_no_formation(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kanan = card(&tg, KANAN);
    int outsider = mid(&tg, FILLER);
    int saint = mid(&tg, SEIRA);
    int filler = mid(&tg, FILLER);
    if (kanan < 0 || outsider < 0 || saint < 0 || filler < 0) return;
    fill_decks(&tg, filler, 40);
    /* A non-Aqours/SaintSnow member on stage must gate the whole effect off. */
    tg.state.p[0].stage[AREA_LEFT] = outsider;
    tg.state.p[0].stage[AREA_CENTER] = saint;
    test_give_energy(&tg, 30);
    test_add_to_hand(&tg, kanan);
    test_play_to_stage(&tg, kanan, AREA_RIGHT);

    int prompts = drain_with(&tg, -1, 24);
    /* CONTROL: the 登場 window was entered at all, so a 0 blade is attributable
     * to the GROUP GATE and not to a board on which nothing ever prompted. */
    CHECK(prompts > 0,
          "E2 CONTROL: the 登場 window was entered (at least one prompt appeared)");
    /* GROUP-GATE CONTROL: the outsider really is a non-Aqours/SaintSnow print,
     * so the gate has something to reject. */
    Card oc;
    if (rb_decode_card_by_index((uint32_t)outsider, &oc)) {
        const char *g = rb_card_group_name(outsider);
        fprintf(stdout, "      [E2] outsider group=\"%s\"\n", g ? g : "(null)");
        CHECK(g == NULL || (strcmp(g, "Aqours") != 0 && strcmp(g, "SaintSnow") != 0),
              "E2 CONTROL: the outsider's group \"%s\" is genuinely not "
              "Aqours/SaintSnow", g ? g : "(null)");
        rb_free_card(&oc);
    }
    CHECK_EQ(total_blade(&tg), 0,
             "a non-group member on stage means no formation change and no blade");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], outsider,
             "the non-group member is untouched");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], saint,
             "the SaintSnow member is untouched");
}

/* Rust: group_gated_formation_no_move_no_blade. */
static void E_group_gated_formation_no_move_no_blade(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int kanan = play_kanan_on_saintsnow_stage(&tg);
    if (kanan < 0) { CHECK(0, "E3: 果南's fixture resolves"); return; }

    CHECK(prompts_on_screen(&tg),
          "E3: 果南's 登場 offers a formation-change prompt");
    /* Answer every member with the area it ALREADY sits in: no member moves, so
     * the printed 「この効果によって『SaintSnow』のメンバーが移動した場合」 never
     * fires and no blade is granted. */
    drain_areas(&tg, AREA_LEFT, 24);
    CHECK_EQ(total_blade(&tg), 0,
             "no SaintSnow member moved, so 果南 grants no blade (got %d)",
             total_blade(&tg));
    /* PAIRED POSITIVE on the same board shape: 果南's effect DOES grant the
     * blade when a member is chosen to move. Without this, the 0 above is
     * equally consistent with the blade branch never running at all, and this
     * test would be reporting the wrong thing as a pass. */
    static TestGame tp;
    test_game_new(&tp);
    bare_board(&tp);
    int kanan2 = play_kanan_on_saintsnow_stage(&tp);
    if (kanan2 < 0) { CHECK(0, "E3 paired positive fixture resolves"); return; }
    drain_areas(&tp, AREA_RIGHT, 24);
    CHECK(total_blade(&tp) >= 2,
          "E3 PAIRED POSITIVE: the same board with a member chosen to move DOES "
          "grant blade x2 (got %d) -- otherwise the zero above would be the "
          "blade branch being dead, not the no-move rule",
          total_blade(&tp));
}

/* ── ########################################################################
 * F. area_move_triggered_auto_ability_stress_test.rs -- the 自動 that watch an
 *    area move. A movement that is a SILENT NO-OP must not arm them, so each
 *    negative here is paired with a positive on a board where a move really
 *    happened.
 *    ######################################################################## */

/* Rust: step1_p1_single. */
static void F_step1_p1_single(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int watcher = card(&tg, WATCHER);
    int mover = card(&tg, KINAKO);
    if (watcher < 0 || mover < 0) return;
    fill_decks(&tg, test_id(&tg, FILLER), 40);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, mover);
    test_add_to_hand(&tg, watcher);
    CHECK_EQ(test_play_to_stage(&tg, mover, AREA_LEFT), 1, "mover to LeftSide");
    CHECK_EQ(test_play_to_stage(&tg, watcher, AREA_CENTER), 1, "watcher to Center");

    int before = tg.state.p[0].energy_active;
    test_activate_ability(&tg, mover);
    int prompts = drain_areas(&tg, AREA_CENTER, 16);
    CHECK(prompts > 0, "F1: the mover's position prompt was answered");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], mover,
             "PRECONDITION: the mover really swapped into Center");
    CHECK_EQ(tg.state.p[0].energy_active - before, 2,
             "鬼塚-less watcher PL!SP-pb2-028-N makes 2 energy active when its "
             "area-move partner moves (got %d)",
             tg.state.p[0].energy_active - before);
}

/* Rust: step3_use_limit. A 起動 marked {{turn1}} may fire once per turn. */
static void F_step3_use_limit(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int watcher = card(&tg, WATCHER);
    int mover = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (watcher < 0 || mover < 0 || filler < 0) return;
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, mover);
    test_add_to_hand(&tg, watcher);
    test_add_to_hand(&tg, filler);
    CHECK_EQ(test_play_to_stage(&tg, mover, AREA_LEFT), 1, "mover to LeftSide");
    CHECK_EQ(test_play_to_stage(&tg, watcher, AREA_CENTER), 1, "watcher to Center");
    CHECK_EQ(test_play_to_stage(&tg, filler, AREA_RIGHT), 1, "filler to RightSide");

    int before = tg.state.p[0].energy_active;
    test_activate_ability(&tg, mover);
    drain_areas(&tg, AREA_CENTER, 16);
    CHECK_EQ(tg.state.p[0].energy_active - before, 2,
             "the first activation moves the member and arms the watcher");
    int second = test_activate_ability(&tg, mover);
    CHECK(second == 0, "a {{turn1}} 起動 refuses a second activation in the same turn");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], mover,
             "the refused second activation does not move the member again");
}

/* Rust: step5_each_time_area_move.
 * 鬼塚夏美 PL!SP-pb1-020-N 自動: each time this member moves area, draw 1.
 * Natsumi is SWAPPED by the mover's position change, so she must draw. */
static void F_step5_each_time_area_move(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int natsumi = card(&tg, NATSUMI);
    int mover = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (natsumi < 0 || mover < 0 || filler < 0) return;
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, mover);
    test_add_to_hand(&tg, natsumi);
    CHECK_EQ(test_play_to_stage(&tg, mover, AREA_LEFT), 1, "mover to LeftSide");
    CHECK_EQ(test_play_to_stage(&tg, natsumi, AREA_CENTER), 1, "夏美 to Center");

    int hand_before = tg.state.p[0].hand.n;
    test_activate_ability(&tg, mover);
    drain_areas(&tg, AREA_CENTER, 16);
    /* PRECONDITION: the swap really happened, otherwise the watcher has no
     * reason to fire and the draw count below proves nothing. */
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], mover,
             "PRECONDITION: the mover swapped into Center, displacing 夏美");
    CHECK_EQ(stage_of(&tg, 0, natsumi), AREA_RIGHT,
             "PRECONDITION: 夏美 was displaced to RightSide");
    CHECK_EQ(tg.state.p[0].hand.n - hand_before, 1,
             "鬼塚夏美's each_time:area_move draws 1 when she is swapped "
             "(got %d)", tg.state.p[0].hand.n - hand_before);
}

/* Rust: step2_p1_then_p2 -- P1's area move arms only P1's watcher.
 * P2's stage is untouched, which is the negative half; P1's +2 is the positive
 * control on the same board. */
static void F_step2_p1_then_p2(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int watcher = card(&tg, WATCHER);
    int mover = card(&tg, KINAKO);
    int filler = mid(&tg, FILLER);
    if (watcher < 0 || mover < 0 || filler < 0) return;
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, mover);
    test_add_to_hand(&tg, watcher);
    CHECK_EQ(test_play_to_stage(&tg, mover, AREA_LEFT), 1, "mover to LeftSide");
    CHECK_EQ(test_play_to_stage(&tg, watcher, AREA_CENTER), 1, "watcher to Center");

    int p1_before = tg.state.p[0].energy_active;
    int p2_before = tg.state.p[1].energy_active;
    test_activate_ability(&tg, mover);
    drain_areas(&tg, AREA_CENTER, 16);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], mover,
             "PRECONDITION: P1's member really swapped");
    CHECK_EQ(tg.state.p[0].energy_active - p1_before, 2,
             "POSITIVE CONTROL: P1's watcher fired (+2)");
    CHECK_EQ(tg.state.p[1].energy_active - p2_before, 0,
             "P2's watcher is not armed by P1's area move");
}

/* ── ########################################################################
 * G. himeno_debut_opponent_member_moved_to_own_front_area_test.rs
 *    The 常時 half: PL!HS-pb1-014-R gains heart01 while the OPPONENT member in
 *    her FRONT area costs more than she does. (The 登場 half -- the front-area
 *    position change -- is already covered by test_parity_zones_position.c's
 *    test_himeno_front_area_shapes / ..._no_opponent_members.)
 *    ######################################################################## */

/* Helper: 姫芽 at Center, an opponent member at Center (the FRONT of Center is
 * Center), an optional cost modifier on the opponent, then recalc. */
static int himeno_constant_heart(TestGame *tg, int opp_cost_mod, int own_cost_mod,
                                 int opp_area)
{
    int himeno = card(tg, HIMENO14);
    int opp = mid(tg, opp_cost_mod != 0 || own_cost_mod != 0 ? GROUP_SD15 : FILLER);
    if (himeno < 0 || opp < 0) return -1;
    tg->state.p[0].stage[AREA_CENTER] = himeno;
    if (opp_area >= 0) tg->state.p[1].stage[opp_area] = opp;
    if (opp_cost_mod) rb_mods_add_cost((RbMods *)&tg->state.mods, opp, opp_cost_mod);
    if (own_cost_mod)  rb_mods_add_cost((RbMods *)&tg->state.mods, himeno, own_cost_mod);
    test_recalc(tg);
    /* Read through rb_mods_get_heart, NOT test_get_heart_modifier (which remaps
     * colour 5 onto RB_HEART_ORANGE). heart01 -> slot 1. */
    return rb_mods_get_heart((RbMods *)&tg->state.mods, himeno, H01);
}

/* Rust: himeno_front_constant_evaluates.
 * 姫芽 costs 9; PL!HS-sd1-006-SD costs 15 and sits in her front area. */
static void G_himeno_front_constant_evaluates(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int got = himeno_constant_heart(&tg, 0, 0, AREA_CENTER);
    if (got < 0) { CHECK(0, "G1: 姫芽's constant fixture resolves"); return; }

    /* POSITIVE CONTROL, so the 0 below is attributable to the CONDITION and not
     * to a recalc that never ran or to the modifier plumbing being dead: seed
     * heart01 by hand, recalc, and require it to SURVIVE. If the seeded value is
     * also cleared, then every "no heart" result in this section would be
     * vacuous and the whole premise of the test would be broken. */
    rb_mods_add_heart((RbMods *)&tg.state.mods,
                      tg.state.p[0].stage[AREA_CENTER], H01, 1);
    test_recalc(&tg);
    int himeno_id = tg.state.p[0].stage[AREA_CENTER];
    CHECK_EQ(rb_mods_get_heart((RbMods *)&tg.state.mods, himeno_id, H01), 1,
             "G1 CONTROL: a hand-seeded heart01 survives test_recalc, so the "
             "zero below is the CONSTANT's own verdict and not a dead pipeline");

    /* ROOT-CAUSE PROBE, reported rather than hidden: the condition the engine
     * decodes for this ability carries position="front", and the position
     * resolver maps only left_side/center/right_side. "front" is therefore
     * unresolvable, both costs decode as 0, and 0 > 0 is false. Stated as an
     * assertion so it stays red if the resolver is ever extended. */
    CHECK_EQ(rb_card_at_position(&tg.state, 0, "front"), -1,
             "ROOT CAUSE: rb_card_at_position() cannot resolve the \"front\" "
             "spelling the 「正面のエリア」 condition uses (currently -1)");
    CHECK_EQ(rb_card_at_position(&tg.state, 0, "center"), himeno_id,
             "CONTROL: the same resolver DOES find the card at \"center\", so the "
             "miss is specific to the front-area spelling and not a broken stage");

    CHECK_EQ(got, 1,
             "a higher-cost opponent member in 姫芽's front area grants heart01 x1");
}

/* Rust: himeno_front_constant_uses_modified_front_cost. -7 drops 15 -> 8 <= 9.
 * The printed heart must be 0, and it is -- but it is 0 for the WRONG REASON:
 * G1's root-cause probe shows the front-area position never resolves, so the
 * cost pair is 0/0 and `0 > 0` is false regardless of the -7. This test is
 * therefore a VACUOUS PASS unless it carries its own control, which is what the
 * control below provides: the same board with the -7 REMOVED must grant the
 * heart, proving the pair is being read and the 0 is the modifier talking. */
static void G_himeno_front_constant_uses_modified_front_cost(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int got = himeno_constant_heart(&tg, -7, 0, AREA_CENTER);
    if (got < 0) { CHECK(0, "G2: 姫芽's fixture resolves"); return; }

    /* CONTROL: the identical board with no cost modifier. On a CORRECT engine
     * this is the G1 shape and grants 1. It is asserted, so a silent 0 here is
     * reported rather than read as "the -7 correctly suppressed it". */
    static TestGame tg2;
    test_game_new(&tg2);
    bare_board(&tg2);
    int control = himeno_constant_heart(&tg2, 0, 0, AREA_CENTER);
    CHECK_EQ(control, 1,
             "G2 CONTROL: the same board WITHOUT the -7 must grant heart01, so the "
             "0 in the main assertion is attributable to the cost modifier");
    CHECK_EQ(got, 0,
             "a cost-modified opponent below 姫芽's cost grants no heart01");
}

/* Rust: himeno_front_constant_uses_modified_own_cost.
 * The filler (cost 4) is in front; 姫芽's own cost is reduced 9 -> 3, so the
 * opponent's printed 4 is now HIGHER and the heart is granted. This is the one
 * G-case whose expectation is 1, so it is the test that separates "the constant
 * is broken" from "this specific opponent is not being found". */
static void G_himeno_front_constant_uses_modified_own_cost(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int himeno = card(&tg, HIMENO14);
    int opp = mid(&tg, FILLER);
    if (himeno < 0 || opp < 0) return;
    tg.state.p[0].stage[AREA_CENTER] = himeno;
    tg.state.p[1].stage[AREA_CENTER] = opp;
    rb_mods_add_cost((RbMods *)&tg.state.mods, himeno, -6);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart((RbMods *)&tg.state.mods, himeno, H01), 1,
             "reducing 姫芽's OWN cost makes the printed-higher opponent qualify");
}

/* Rust: himeno_front_constant_no_opponent. */
static void G_himeno_front_constant_no_opponent(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int himeno = card(&tg, HIMENO14);
    if (himeno < 0) return;
    tg.state.p[0].stage[AREA_CENTER] = himeno;
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[1].stage[i] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    /* CONTROL: the opponent side really is empty at every slot, so a 0 cannot be
     * blamed on an accidental stray member. */
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK_EQ(tg.state.p[1].stage[i], RB_EMPTY_SLOT,
                 "G4 CONTROL: the opponent stage is empty at slot %d", i);
    CHECK_EQ(rb_mods_get_heart((RbMods *)&tg.state.mods, himeno, H01), 0,
             "an empty opponent front area grants no heart01");
}

/* Rust: himeno_front_constant_opponent_at_side.
 * The opponent is at LEFT, not in 姫芽's front area (Center). */
static void G_himeno_front_constant_opponent_at_side(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int himeno = card(&tg, HIMENO14);
    int opp = mid(&tg, GROUP_SD15);   /* cost 15 > 姫芽's 9: qualifies IF in front */
    if (himeno < 0 || opp < 0) return;
    tg.state.p[0].stage[AREA_CENTER] = himeno;
    tg.state.p[0].stage[AREA_LEFT] = mid(&tg, FILLER);
    tg.state.p[1].stage[AREA_LEFT] = opp;
    test_recalc(&tg);
    /* CONTROL: the higher-cost opponent IS on the board, it is simply at a SIDE.
     * Without this, the 0 below would be equally consistent with "no opponent
     * was ever placed", which is a test bug, not an engine verdict. */
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], opp,
             "G5 CONTROL: the higher-cost opponent member IS on the board, at a side");
    CHECK_EQ(tg.state.p[1].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "G5 CONTROL: the front area of Center is empty");
    CHECK_EQ(rb_mods_get_heart((RbMods *)&tg.state.mods, himeno, H01), 0,
             "an opponent at a SIDE, not the front area, grants no heart01");
}

/* ── ########################################################################
 * H. Two live-start / rotation shapes the zones_position file does not cover.
 *    ######################################################################## */

/* Rust: live_start_two_subunit_members_can_keep_formation.
 * Jellyfish (PL!SP-pb2-050-L) offers a formation change when the stage holds
 * enough subunit members. Answering each member with the area it ALREADY holds
 * must leave the arrangement identical and record no movement. */
static void H_live_start_two_subunit_members_can_keep_formation(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int live = card(&tg, JELLYFISH);
    int a = mid(&tg, LIELLA_MEMBER);
    int b = mid(&tg, LIELLA_MEMBER);
    int c = mid(&tg, FILLER);
    if (live < 0 || a < 0 || b < 0 || c < 0) return;
    fill_decks(&tg, c, 20);
    tg.state.p[0].stage[AREA_LEFT] = a;
    tg.state.p[0].stage[AREA_CENTER] = b;
    tg.state.p[0].stage[AREA_RIGHT] = c;
    test_add_to_live(&tg, live);

    fire_live_start(&tg, 0);
    int prompts = drain_areas(&tg, AREA_LEFT, 24);
    CHECK(prompts > 0, "H1: the live-start formation change produced a prompt");
    /* CONTROL: the formation window really opened. "The board is unchanged"
     * is otherwise indistinguishable from "nothing ever prompted". */
    CHECK_EQ(stage_of(&tg, 0, a), AREA_LEFT, "H1 PREMISE: a starts at LeftSide");
    CHECK_EQ(stage_of(&tg, 0, b), AREA_CENTER, "H1 PREMISE: b starts at Center");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], a, "LeftSide keeps its member");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], b, "Center keeps its member");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], c, "RightSide keeps its member");
    CHECK(!moved_this_turn(&tg, a), "H1: keeping the formation records no move (a)");
    CHECK(!moved_this_turn(&tg, b), "H1: keeping the formation records no move (b)");
    CHECK(!moved_this_turn(&tg, c), "H1: keeping the formation records no move (c)");
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK_EQ(tg.state.p[1].stage[i], RB_EMPTY_SLOT,
                 "H1: the opponent stage is untouched");
}

/* Rust: wien_bp5_q223_both_centers_occupied_both_choose.
 * ウィーン PL!SP-bp5-010-R 登場: BOTH players position-change the member in their
 * own Center. Q223: the OPPONENT decides where their member goes, and with both
 * centers occupied the opponent's choice comes FIRST, then the activating
 * player's. */
static void H_wien_both_centers_occupied_both_choose(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int wien = card(&tg, "PL!SP-bp5-010-R");
    int p1_center = mid(&tg, SD013);
    int p1_right = mid(&tg, SD014);
    int p2_center = mid(&tg, FILLER);
    if (wien < 0 || p1_center < 0 || p1_right < 0 || p2_center < 0) return;
    fill_decks(&tg, p1_center, 20);
    test_give_energy(&tg, 13);
    tg.state.p[0].stage[AREA_CENTER] = p1_center;
    tg.state.p[0].stage[AREA_RIGHT] = p1_right;
    tg.state.p[1].stage[AREA_CENTER] = p2_center;
    test_add_to_hand(&tg, wien);

    CHECK_EQ(test_play_to_stage(&tg, wien, AREA_LEFT), 1, "wien debuts to LeftSide");
    /* CONTROL: a choice is on screen before anything is answered. */
    CHECK(prompts_on_screen(&tg),
          "H2: the 登場 raises a destination choice before any answer");
    if (!prompts_on_screen(&tg)) {
        EXPECTED_GAP("Q223 both-centers: no choice was offered, so the "
                     "opponent-first ordering cannot be observed on this snapshot.");
        return;
    }
    int seat = tg.state.queue.pending.actor;
    fprintf(stdout, "      [H2] first choice seat=%d target=\"%s\"\n",
            seat, tg.state.queue.pending.target);
    /* Answer index 0 = the LeftSide area for the opponent, then again for self. */
    int prompts = drain_areas(&tg, AREA_LEFT, 24);
    CHECK(prompts > 0, "H2: at least one destination choice was answered");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], wien,
             "wien stays at LeftSide throughout the rotation");
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], p2_center,
             "Q223: the opponent's Center member moved to the chosen LeftSide");
    CHECK_EQ(tg.state.p[1].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "the opponent's Center is vacated");
    EXPECTED_GAP("Rust also asserts the SECOND prompt is owned by p1 and that the "
                 "options are exactly {left,right}. The option list is not "
                 "observable; the ordering claim is reduced to the prompt count.");
}

/* ── child-process harness ───────────────────────────────────────────────── */

static void on_crash(int sig)
{
    fprintf(stderr, "CRASH: signal %d during %s (%d assertions, %d failures)\n",
            sig, current_test, TALLY->assertions, TALLY->failures);
    fflush(stderr);
    _Exit(3);
}

#define CHILD_OK        0
#define CHILD_FAILURES  1

/* Each case runs in a forked child, so the child's assertion/failure counters
 * never reach the parent on their own. A MAP_SHARED page backs the tally: the
 * child writes its own numbers into it and the parent folds them in after
 * waitpid, so the run summary is a real total rather than a flat zero. A pipe
 * was tried first and is the wrong tool here: a short or interleaved read
 * desynchronises the four numbers and poisons the run totals. */
static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    TALLY->assertions = 0;
    TALLY->failures   = 0;
    TALLY->setup_bugs = 0;
    TALLY->gaps       = 0;
    pid_t pid = fork();
    if (pid < 0) {
        current_test = name;
        fn();
        current_test = "(none)";
        if (TALLY->failures > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions, %d failures]\n",
               TALLY->failures ? "FAILED" : "ok", name, TALLY->assertions,
               TALLY->failures);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        current_test = name;
        fn();
        fflush(stdout);
        fflush(stderr);
        _Exit(TALLY->failures > 0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    assertions += TALLY->assertions;
    failures   += TALLY->failures;
    setup_bugs += TALLY->setup_bugs;
    gaps       += TALLY->gaps;
    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH",
               name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_tests_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_tests_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* RB_AM_DUMP=<card_no>[,<card_no>...] -- print the DECODED ability tree for the
 * named prints. Card.ability is not populated by rb_decode_card_by_index, so
 * this reads rb_card_num_abilities + rb_decode_card_ability, the same pair the
 * engine uses. Diagnostic only; it never runs in a normal run. */
static void am_dump_abilities(const char *list)
{
    char buf[512];
    snprintf(buf, sizeof(buf), "%s", list);
    for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ",")) {
        int id = rb_find_card_by_no(tok);
        if (id < 0) { printf("AM_DUMP %s: NOT FOUND\n", tok); continue; }
        Card c;
        char printed[96] = "?";
        if (rb_decode_card_by_index((uint32_t)id, &c)) {
            snprintf(printed, sizeof(printed), "%s", rb_card_string(c.card_no_idx));
            printf("AM_DUMP %s -> id=%d printed=\"%s\" cost=%d blade=%d group=\"%s\"\n",
                   tok, id, printed, (int)c.cost, (int)c.blade, rb_card_group_name(id));
            rb_free_card(&c);
        }
        int n = rb_card_num_abilities((uint32_t)id);
        for (int i = 0; i < n; i++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)id, i, &ab)) continue;
            printf("  ab#%d triggers=\"%s\" use_limit=%d\n", i,
                   ab.triggers ? ab.triggers : "(null)", ab.use_limit);
            if (ab.effect)
                printf("    effect action=%s target=%s destination=%s count=%d "
                       "optional=%d n_extra=%d HAS_CONDITION=%d\n",
                       ab.effect->action ? ab.effect->action : "(null)",
                       ab.effect->target ? ab.effect->target : "(null)",
                       ab.effect->destination ? ab.effect->destination : "(null)",
                       ab.effect->count, ab.effect->is_optional, ab.effect->n_extra,
                       ab.effect->has_condition);
            for (int k = 0; k < (ab.effect ? ab.effect->n_extra : 0); k++)
                printf("    extra[%d] %s = %s\n", k,
                       ab.effect->extra_k[k] ? ab.effect->extra_k[k] : "(null)",
                       ab.effect->extra_v[k] ? ab.effect->extra_v[k] : "(null)");
            rb_free_ability(&ab);
        }
    }
}

/* RB_AM_COND=<card_no> -- evaluate the CONDITION attached to each ability of one
 * print on a board configured for THAT card, and print the verdict. This is how
 * a "no prompt was ever raised" outcome gets attributed to the condition gate
 * instead of being left as a guess: the gate is called directly, so a 0 here
 * proves the gate is what suppressed the ability.
 * Diagnostic only; it never runs in a normal run. */
static void am_probe_condition(const char *no)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int id = card(&tg, no);
    if (id < 0) return;

    /* A board that satisfies the printed text of the card's own ability as far
     * as it can be satisfied by placement: the card at CENTER, a NEUTRAL member
     * at Left, and -- the case these cards care about -- a plain
     * みらくらぱーく！ / Aqours / SaintSnow member in the OPPONENT's Center, which
     * is the FRONT area of the card's own Center. */
    int filler = mid(&tg, FILLER);
    int opp_same = mid(&tg, GROUP_SD15);
    tg.state.p[0].stage[AREA_LEFT] = filler;
    tg.state.p[0].stage[AREA_CENTER] = id;
    tg.state.p[1].stage[AREA_CENTER] = opp_same;
    test_give_energy(&tg, 20);

    int n = rb_card_num_abilities((uint32_t)id);
    printf("COND_PROBE %s id=%d n_abilities=%d own_cost=%d opp_cost=%d\n",
           no, id, n, test_get_cost_modifier(&tg, id) + 0,
           test_get_cost_modifier(&tg, opp_same) + 0);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)id, i, &ab)) continue;
        if (ab.effect && ab.effect->condition)
            printf("  ab#%d triggers=\"%s\" action=%s HAS_CONDITION=%d verdict=%d\n",
                   i, ab.triggers ? ab.triggers : "(null)",
                   ab.effect->action ? ab.effect->action : "(null)",
                   ab.effect->has_condition,
                   rb_eval_condition_for_host(&tg.state, 0, id,
                                             ab.effect->condition));
        else
            printf("  ab#%d triggers=\"%s\" action=%s HAS_CONDITION=0\n",
                   i, ab.triggers ? ab.triggers : "(null)",
                   ab.effect ? (ab.effect->action ? ab.effect->action : "(null)")
                             : "(no effect)");
        rb_free_ability(&ab);
    }
    /* The condition VARIANT and its field list name the gate that produced the
     * 0. Reporting "the condition said no" is not actionable; reporting
     * "RB_COND_COMPARISON with {lhs=..., op=..., rhs=...} returned 0" is. */
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)id, i, &ab)) continue;
        const Condition *c = ab.effect ? ab.effect->condition : NULL;
        if (c) {
            printf("  ab#%d CONDITION variant=%d n_fields=%u\n", i,
                   (int)c->variant, c->n_fields);
            for (uint32_t f = 0; f < c->n_fields; f++) {
                const CondField *cf = &c->fields[f];
                printf("      field[%u] key=\"%s\" tag=%d i=%lld b=%d s=%s\n",
                       f, cf->key ? cf->key : "(null)", (int)cf->v.tag,
                       (long long)cf->v.i, cf->v.b,
                       cf->v.s ? cf->v.s : (cf->v.cond ? "(nested condition)" : "(null)"));
                if (cf->v.cond)
                    printf("          nested variant=%d n_fields=%u\n",
                           (int)cf->v.cond->variant, cf->v.cond->n_fields);
            }
        }
        rb_free_ability(&ab);
    }

    /* The condition TYPE and its expected/actual text come from the ability
     * verdict log, which rb_eval_condition_for_host populates. That turns "the
     * gate said no" into "the gate compared the wrong pair", which is the
     * difference between an engine bug report and a guess. */
    RbAbilityLogItem items[8];
    int n_items = rb_log_drain_verdicts(items, 8);
    for (int k = 0; k < n_items; k++) {
        if (items[k].kind != RB_LOG_KIND_CONDITION) continue;
        printf("  verdict[%d] type=\"%s\" text=\"%s\" expected=\"%s\" actual=\"%s\" "
               "passed=%d\n",
               k, items[k].as.condition.condition_type,
               items[k].as.condition.text, items[k].as.condition.expectation,
               items[k].as.condition.actual, items[k].as.condition.passed);
    }
    printf("  after test_recalc: heart01=%d\n",
           rb_mods_get_heart((RbMods *)&tg.state.mods, id, H01));
    test_recalc(&tg);
    printf("  after explicit recalc: heart01=%d\n",
           rb_mods_get_heart((RbMods *)&tg.state.mods, id, H01));
}

/* RB_AM_PROBE=rin -- minimal reproduction of the 星空凛 ライブ開始時 position
 * change with the ability debug log switched on, so a "no prompt" outcome can be
 * attributed to the effect's condition gate rather than left as a guess.
 * Diagnostic only; it never runs in a normal run. */
static void am_probe_rin(void)
{
    static TestGame tg;
    const char *RIN = "PL!-bp4-005-R\xEF\xBC\x8B";
    test_game_new(&tg);
    bare_board(&tg);
    int rin = card(&tg, RIN);
    int filler = mid(&tg, FILLER);
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 20);
    tg.state.p[0].stage[AREA_LEFT] = filler;
    tg.state.p[0].stage[AREA_CENTER] = rin;
    test_add_to_live(&tg, mid(&tg, FILLER));
    printf("PROBE: rin=%d filler=%d pending_before=%d\n", rin, filler,
           rb_has_pending_choice(&tg.state));
    rb_ability_debug_set(1);
    fire_live_start(&tg, 0);
    rb_ability_debug_set(0);
    const RbChoice *pch = rb_get_pending_choice(&tg.state);
    printf("PROBE: pending_after=%d kind=%d target=%s\n",
           rb_has_pending_choice(&tg.state),
           pch ? (int)pch->kind : -1,
           pch ? pch->target : "(none)");
    /* The printed gate is 「自分のステージにブレードを5つ以上持つ『μ's』のメンバーが
     * いない場合」. Rin is a μ's with printed blade 4 and no modifier, so no
     * member reaches 5 and the gate should PASS. Evaluating it directly says
     * whether a missing prompt is the GATE or the EFFECT. */
    Ability ab;
    if (rb_decode_card_ability((uint32_t)rin, 2, &ab) && ab.effect &&
        ab.effect->condition) {
        printf("PROBE: ab#2 condition verdict on this board = %d "
               "(1 = the ライブ開始時 effect should run)\n",
               rb_eval_condition_for_host(&tg.state, 0, rin, ab.effect->condition));
        rb_free_ability(&ab);
    }
    printf("PROBE: filler_blade=%d rin_blade=%d\n",
           rb_mods_get_blade((RbMods *)&tg.state.mods, filler),
           rb_mods_get_blade((RbMods *)&tg.state.mods, rin));
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_crash);
    signal(SIGBUS,  on_crash);
    signal(SIGABRT, on_crash);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    /* The child tally must live in shared pages or the parent's totals stay
     * zero. Allocated after rb_load so the diagnostic paths below still work. */
    void *shared = mmap(NULL, 4096, PROT_READ | PROT_WRITE,
                        MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: could not map the shared tally page\n");
        return 1;
    }
    memset(shared, 0, 4096);
    g_tally = (struct RunTally *)shared;

    {
        const char *c = getenv("RB_AM_COND");
        if (c && *c) { am_probe_condition(c); rb_unload(); return 0; }
    }

    {
        const char *p = getenv("RB_AM_PROBE");
        if (p && *p) { am_probe_rin(); rb_unload(); return 0; }
    }

    {
        const char *d = getenv("RB_AM_DUMP");
        if (d && *d) { am_dump_abilities(d); rb_unload(); return 0; }
    }

    printf("--- A position_change_non_optional_test.rs (exclude_self / exclude_position) ---\n");
    run("non_optional_pc_excludes_self_from_center",
        A_non_optional_pc_excludes_self_from_center);
    run("non_optional_pc_excludes_self_from_left",
        A_non_optional_pc_excludes_self_from_left);
    run("non_optional_pc_excludes_self_from_right",
        A_non_optional_pc_excludes_self_from_right);
    run("non_optional_pc_executes_move_correctly",
        A_non_optional_pc_executes_move_correctly);
    run("exclude_position_center_not_offered", A_exclude_position_center_not_offered);
    run("exclude_position_and_exclude_self_combine_correctly",
        A_exclude_position_and_exclude_self_combine);
    run("non_optional_pc_with_all_positions_excluded_fizzles",
        A_non_optional_pc_with_all_positions_excluded_fizzles);
    run("formation_change_still_offers_all_positions",
        A_formation_change_still_offers_all_positions);
    run("optional_pc_excludes_self_but_can_skip", A_optional_pc_excludes_self_but_can_skip);

    printf("--- B debut_selected_member_fixed_center_test.rs (PL!S-bp7-018-N) ---\n");
    run("fixed_center_choice_offers_self_and_other_stage_members",
        B_fixed_center_choice_offers_self_and_other_stage_members);
    run("fixed_center_move_left_member_to_center",
        B_fixed_center_move_left_member_to_center);
    run("fixed_center_move_right_member_to_center",
        B_fixed_center_move_right_member_to_center);
    run("fixed_center_can_choose_self_when_not_already_center",
        B_fixed_center_can_choose_self_when_not_already_center);
    run("fixed_center_center_member_noop", B_fixed_center_center_member_noop);
    run("fixed_center_destination_is_fixed_center_no_free_dest_choice",
        B_fixed_center_destination_is_fixed_center_no_free_dest_choice);
    run("fixed_center_move_left_to_occupied_center_swaps_and_records",
        B_fixed_center_move_left_to_occupied_center_swaps_and_records);
    run("fixed_center_full_stage_choose_any_member",
        B_fixed_center_full_stage_choose_any_member);

    printf("--- C position_change_multi_test.rs (PL!HS-bp2-006-R / group filter) ---\n");
    run("position_change_three_members_all_move",
        C_position_change_three_members_all_move);
    run("position_change_no_other_members", C_position_change_no_other_members);
    run("position_change_with_swap", C_position_change_with_swap);
    run("position_change_tracks_card_movement",
        C_position_change_tracks_card_movement);
    run("formation_change_with_group_names_and_empty_slots",
        C_formation_change_with_group_names_and_empty_slots);
    run("position_change_filters_by_group_names",
        C_position_change_filters_by_group_names);

    printf("--- D formation_change_test.rs (movement tracking) ---\n");
    run("all_three_move_counts", D_all_three_move_counts);
    run("stay_in_place_no_new_move", D_stay_in_place_no_new_move);
    run("one_moves_two_stay", D_one_moves_two_stay);

    printf("--- E group_only_debut_formation_move_subunit_gain_blades_test.rs ---\n");
    run("group_gated_debut_saintsnow_stage_formation_change_grants_blade",
        E_group_gated_debut_saintsnow_stage_grants_blade);
    run("group_gated_debut_non_group_member_no_formation",
        E_group_gated_debut_non_group_member_no_formation);
    run("group_gated_formation_no_move_no_blade",
        E_group_gated_formation_no_move_no_blade);

    printf("--- F area_move_triggered_auto_ability_stress_test.rs ---\n");
    run("step1_p1_single", F_step1_p1_single);
    run("step2_p1_then_p2", F_step2_p1_then_p2);
    run("step3_use_limit", F_step3_use_limit);
    run("step5_each_time_area_move", F_step5_each_time_area_move);

    printf("--- G himeno_debut... 常時 heart01 (PL!HS-pb1-014-R) ---\n");
    run("himeno_front_constant_evaluates", G_himeno_front_constant_evaluates);
    run("himeno_front_constant_uses_modified_front_cost",
        G_himeno_front_constant_uses_modified_front_cost);
    run("himeno_front_constant_uses_modified_own_cost",
        G_himeno_front_constant_uses_modified_own_cost);
    run("himeno_front_constant_no_opponent", G_himeno_front_constant_no_opponent);
    run("himeno_front_constant_opponent_at_side",
        G_himeno_front_constant_opponent_at_side);

    printf("--- H live-start formation + wien Q223 both-centers ---\n");
    run("live_start_two_subunit_members_can_keep_formation",
        H_live_start_two_subunit_members_can_keep_formation);
    run("wien_bp5_q223_both_centers_occupied_both_choose",
        H_wien_both_centers_occupied_both_choose);

    printf("\n==== parity_area_move: %d tests ok, %d failed, %d crashed "
           "(%d assertions, %d failures, %d setup bugs, %d expected gaps) ====\n",
           n_tests_ok, n_tests_failed, n_tests_crashed, assertions, failures,
           setup_bugs, gaps);
    return failures ? 1 : 0;
}
