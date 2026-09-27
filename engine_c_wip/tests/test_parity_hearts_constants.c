/* Parity suite for engine/tests/test_modules/effects/gain/hearts/constants/
 *
 * Every test names the Rust file it mirrors. PORT_TRACKER.md marks
 * "Stage/member heart resolution — original hearts, copy, multiplier, override,
 *  additive modifiers, effective needs, and stage aggregation" COMPLETE, and this
 * suite is the adversarial check on that claim: it drives the real
 * rb_recalc_constants + rb_mods_get_heart path for every 常時 (constant) heart
 * ability in the cluster and compares against the Rust expectations verbatim.
 *
 * Heart colour indices follow cards/compile_cards.py HEART_COLORS:
 *   0 = heart00, 1..6 = heart01..heart06, 7 = icon_all ("All").
 * A failure here in a mechanic the tracker calls complete is a parity break.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#define H00 0
#define H01 1
#define H02 2
#define H03 3
#define H04 4
#define H05 5
#define H06 6
#define HALL 7

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    long long actual_value = (long long)(actual); \
    long long expected_value = (long long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %lld expected %lld)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── helpers ─────────────────────────────────────────────────────────────── */

/* test_id() returns -1 for a card number that is not in the database. A -1 id
   silently turns every downstream assertion into a no-op, so every card this
   suite stages goes through here and reports the miss loudly. */
static int cid(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    checks++;
    if (id < 0) {
        fprintf(stderr, "FAIL: card \"%s\" is not in the database\n", no);
        failures++;
        return -1;
    }
    return id;
}

static int cid_new(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    checks++;
    if (id < 0) {
        fprintf(stderr, "FAIL: card \"%s\" has no copy source in the database\n", no);
        failures++;
        return -1;
    }
    return id;
}

static int heart(TestGame *tg, int id, int color)
{
    return rb_mods_get_heart(&tg->state.mods, id, color);
}

/* Mirror stage.stage = [a, b, c]: clear every area, then set the three. */
static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static void stage_center(TestGame *tg, int cid)
{
    set_stage(tg, 0, RB_EMPTY_SLOT, cid, RB_EMPTY_SLOT);
}

/* Mirror energy_zone { cards.clear(); push n; set_active_count(k) }. The C port
   has no clamp in rb_energy_set_active_count, so the count is exact. */
static void set_energy(TestGame *tg, int pl, int n_cards, int n_active)
{
    int e = test_id(tg, "LL-E-001-SD");
    RbPlayer *P = &tg->state.p[pl];
    P->energy.n = 0;
    for (int i = 0; i < n_cards && i < RB_MAX_ZONE; i++)
        P->energy.cards[P->energy.n++] = e;
    rb_energy_set_active_count(P, n_active);
}

static void fill_decks(TestGame *tg, const char *filler_no, int per_player)
{
    int f = test_id(tg, filler_no);
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < per_player; i++) {
            if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = f;
        }
    }
}

/* ===================================================================== */
/* A. 葉月恋 PL!SP-bp5-016-N — energy >= 10 -> heart06 x2                */
/*    ten_energy_constant_heart06_test.rs                                 */
/*    ten_energy_constant_heart06_threshold_boundary_test.rs              */
/* ===================================================================== */

static void test_ten_energy_constant_heart06(void)
{
    TestGame tg; test_game_new(&tg);
    int member = cid(&tg, "PL!SP-bp5-016-N");
    stage_center(&tg, member);
    fill_decks(&tg, "PL!-sd1-010-SD", 15);
    set_energy(&tg, 0, 12, 12);

    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H06), 2, "12 energy >= 10 -> +2 heart06");

    /* Rust's trailing negative step: spend 5 and the as_long_as gain must go. */
    test_spend_energy(&tg, 5);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H06), 0,
             "spending back below the 10-energy threshold removes the heart06 gain");

    /* Boundary, from ten_energy_constant_heart06_threshold_boundary_test.rs:
       exactly 10 grants, one below does not. */
    set_energy(&tg, 0, 10, 10);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H06), 2, "exactly 10 active energy -> +2 heart06");

    set_energy(&tg, 0, 9, 9);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H06), 0, "9 active energy (< 10) -> 0 heart06");
}

/* ===================================================================== */
/* B. ウィーン・マルガレーテ PL!SP-pb2-032-N — sequential 6/8 thresholds */
/*    six_eight_energy_constant_heart06_test.rs                           */
/* ===================================================================== */

static int wien_h06_at_energy(TestGame *tg, int wien, int energy)
{
    set_energy(tg, 0, energy, energy);
    test_recalc(tg);
    return heart(tg, wien, H06);
}

static void test_six_eight_energy_constant_heart06(void)
{
    TestGame tg; test_game_new(&tg);
    int wien = cid(&tg, "PL!SP-pb2-032-N");
    stage_center(&tg, wien);

    CHECK_EQ(wien_h06_at_energy(&tg, wien, 0), 0, "0 energy -> 0 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 5), 0, "5 energy (<6) -> 0 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 6), 1, "6 energy (>=6) -> 1 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 7), 1, "7 energy (>=6, <8) -> 1 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 8), 2, "8 energy (>=6, >=8) -> 2 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 10), 2, "10 energy (>=6, >=8) -> 2 heart06");

    set_energy(&tg, 0, 10, 10);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, wien, H02), 0, "10 energy -> heart02 must stay 0");

    /* Dynamic as_long_as: drop below both thresholds, then back to one. */
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 10), 2, "precondition: 10 energy -> 2 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 5), 0, "dropping to 5 energy -> 0 heart06");
    CHECK_EQ(wien_h06_at_energy(&tg, wien, 7), 1, "raising to 7 energy -> 1 heart06");
}

/* ===================================================================== */
/* C. 蓮 PL!SP-pb2-027-N (heart03) / 薫風 PL!SP-pb2-023-N (heart02)       */
/*    energy_threshold_two_hearts_at_eight_test.rs                        */
/*    energy_threshold_stacked_heart02_six_and_eight_test.rs              */
/* ===================================================================== */

static void test_energy_threshold_two_hearts_at_eight(void)
{
    /* 蓮: >=6 energy -> +1 heart03, >=8 -> a second heart03. */
    struct { int energy; int expect; const char *msg; } cases[] = {
        { 5, 0, "Ren: 5 energy (<6) -> no heart03" },
        { 6, 1, "Ren: 6 energy (>=6, <8) -> exactly one heart03" },
        { 7, 1, "Ren: 7 energy (<8) -> still one heart03" },
        { 8, 2, "Ren: 8 energy (>=8) -> both heart03" },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        TestGame tg; test_game_new(&tg);
        int ren = cid(&tg, "PL!SP-pb2-027-N");
        set_stage(&tg, 0, RB_EMPTY_SLOT, ren, RB_EMPTY_SLOT);
        set_energy(&tg, 0, cases[i].energy, cases[i].energy);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, ren, H03), cases[i].expect, cases[i].msg);
    }
}

static void test_energy_threshold_stacked_heart02(void)
{
    TestGame tg; test_game_new(&tg);
    int kanon = cid(&tg, "PL!SP-pb2-023-N");
    stage_center(&tg, kanon);

    set_energy(&tg, 0, 5, 5);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, kanon, H02), 0, "Kanon: 5 energy -> 0 heart02");

    set_energy(&tg, 0, 7, 7);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, kanon, H02), 1, "Kanon: energy 7 >= 6 -> one heart02");

    set_energy(&tg, 0, 8, 8);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, kanon, H02), 2, "Kanon: energy 8 >= 8 -> one more heart02 (total 2)");
}

/* ===================================================================== */
/* D. 優木せつ菜 PL!N-bp4-007-R＋ — combined energy >= 15 -> heart02 x2    */
/*    combined_energy_fifteen_constant_heart_test.rs                       */
/*    combined_energy_fifteen_heart02_boundary_test.rs                    */
/* ===================================================================== */

static void test_combined_energy_fifteen_heart02(void)
{
    TestGame tg; test_game_new(&tg);
    int setsuna = cid(&tg, "PL!N-bp4-007-R＋");
    stage_center(&tg, setsuna);

    /* 8 own + 7 opponent = 15 total. */
    set_energy(&tg, 0, 8, 8);
    set_energy(&tg, 1, 7, 7);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, setsuna, H02), 2, "combined energy >= 15 -> +2 heart02");

    /* Negative: drop total below 15. */
    set_energy(&tg, 1, 3, 3);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, setsuna, H02), 0, "combined energy < 15 -> no heart02");

    /* Wait-state energy still counts: 15 total cards but only 10 active. An
       active-only implementation would grant nothing here. */
    set_energy(&tg, 1, 7, 2);
    CHECK_EQ(tg.state.p[0].energy.n + tg.state.p[1].energy.n, 15,
             "precondition: 15 total energy cards");
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, setsuna, H02), 2,
             "15 total cards incl. wait-state energy -> +2 heart02");

    /* combined_energy_fifteen_heart02_boundary_test.rs: exactly 15 / 14. */
    {
        TestGame t2; test_game_new(&t2);
        int s2 = cid(&t2, "PL!N-bp4-007-R+");
        stage_center(&t2, s2);
        set_energy(&t2, 0, 10, 10);
        set_energy(&t2, 1, 5, 5);
        test_recalc(&t2);
        CHECK_EQ(heart(&t2, s2, H02), 2, "Hanayo: combined energy = 15 -> +2 heart02");
    }
    {
        TestGame t2; test_game_new(&t2);
        int s2 = cid(&t2, "PL!N-bp4-007-R+");
        stage_center(&t2, s2);
        set_energy(&t2, 0, 10, 10);
        set_energy(&t2, 1, 4, 4);
        test_recalc(&t2);
        CHECK_EQ(heart(&t2, s2, H02), 0, "Hanayo: combined energy = 14 -> no heart02");
    }
}

/* ===================================================================== */
/* E. 平安名すみれ PL!SP-bp2-004-R/-P — center holds the highest cost      */
/*    center_highest_cost_constant_heart03_test.rs                        */
/*    center_highest_cost_constant_heart03_edges_test.rs                  */
/*    center_highest_cost_constant_heart_empty_and_single_member_test.rs */
/* ===================================================================== */

#define SUMIRE_R   "PL!SP-bp2-004-R"    /* cost 9  */
#define SUMIRE_P   "PL!SP-bp2-004-P"    /* cost 9  */
#define COST_11    "PL!SP-pb1-001-R"    /* cost 11 */
#define COST_10    "PL!HS-PR-001-PR"    /* cost 10 */
#define COST_9     "PL!-PR-005-PR"      /* cost 9  */
#define COST_8     "PL!SP-bp5-111-R"    /* cost 8  */
#define COST_7     "PL!N-PR-021-PR"     /* cost 7  */
#define COST_4     "PL!-sd1-010-SD"     /* cost 4  */

/* One row of the "is center strictly the highest-cost member?" matrix. */
static void sumire_case(const char *self_no, const char *left_no,
                        const char *center_no, const char *right_no,
                        int expect, const char *msg)
{
    TestGame tg; test_game_new(&tg);
    int self = cid(&tg, self_no);
    int left  = (left_no   && *left_no)   ? cid(&tg, left_no)   : RB_EMPTY_SLOT;
    int center= (center_no && *center_no) ? cid(&tg, center_no) : RB_EMPTY_SLOT;
    int right = (right_no  && *right_no)  ? cid(&tg, right_no)  : RB_EMPTY_SLOT;
    set_stage(&tg, 0, left, center, right);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, self, H03), expect, msg);
}

static void test_center_highest_cost_constant_heart03(void)
{
    /* center_highest_cost_constant_heart03_test.rs */
    sumire_case(SUMIRE_P, SUMIRE_P, COST_9, NULL, 0,
                "P print: center cost ties the left -> no heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, NULL, NULL, 0,
                "lone left member with an empty center -> no heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, COST_11, NULL, 1,
                "center 11 > left 9 -> exactly one heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, NULL, NULL, 0,
                "empty center with a lone member -> no heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, COST_9, COST_9, 0,
                "all three cost 9 -> center not strictly highest, no heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, COST_11, COST_4, 1,
                "center 11 above both cost-4 sides -> one heart03");
    sumire_case(SUMIRE_R, COST_11, COST_10, COST_4, 0,
                "left 11 outranks center 10 -> no heart03");
    sumire_case(SUMIRE_R, COST_11, COST_4, COST_10, 0,
                "center lowest with high sides -> no heart03");
    sumire_case(SUMIRE_R, NULL, NULL, NULL, 0,
                "no members on stage -> no heart03");
    sumire_case(SUMIRE_P, SUMIRE_P, COST_11, NULL, 1,
                "P print: center highest with left self -> one heart03");
    sumire_case(SUMIRE_P, SUMIRE_P, COST_9, NULL, 0,
                "P print: tied center cost with left self -> no heart03");
    sumire_case(SUMIRE_R, COST_4, COST_11, SUMIRE_R, 1,
                "self at right with center highest -> one heart03");
    sumire_case(SUMIRE_R, COST_11, NULL, COST_10, 0,
                "empty center with two side members -> no heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, COST_10, COST_11, 0,
                "right 11 above center 10 -> no heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, COST_4, NULL, 0,
                "center 4 below left 9 -> no heart03");
    sumire_case(SUMIRE_R, NULL, SUMIRE_R, NULL, 1,
                "lone self at center is trivially highest -> one heart03");
    sumire_case(SUMIRE_R, SUMIRE_R, COST_4, COST_10, 0,
                "center 4 lowest, right 10 -> no heart03");

    /* center_highest_cost_constant_heart03_edges_test.rs — these read the
       modifier BEFORE the first recalc, so they also pin "no constant is
       granted until recalculate_constants runs". */
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_11);
        int right = cid(&tg, COST_4);
        set_stage(&tg, 0, self, center, right);
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03) - before, 1,
                 "edges: center 11 > self 9 and right 4 -> +1 heart03 on recalc");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_7);
        int right = cid(&tg, COST_4);
        set_stage(&tg, 0, self, center, right);
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03), before,
                 "edges: center 7 < self 9 -> no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_9);
        int right = cid(&tg, COST_4);
        set_stage(&tg, 0, self, center, right);
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03), before,
                 "edges: center 9 == self 9 -> strict > means no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_11);
        int right = cid(&tg, COST_10);
        set_stage(&tg, 0, self, center, right);
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03) - before, 1,
                 "edges: center 11 above 9 and 10 -> +1 heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_8);
        int right = cid(&tg, COST_10);
        set_stage(&tg, 0, self, center, right);
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03), before,
                 "edges: center 8 < self 9 < right 10 -> no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_11);
        set_stage(&tg, 0, self, center, RB_EMPTY_SLOT);
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03) - before, 1,
                 "edges: two members only, center 11 > 9 -> +1 heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int low = cid(&tg, COST_4);
        set_stage(&tg, 0, low, self, cid(&tg, COST_4));
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03) - before, 1,
                 "edges: self at center 9 above two cost-4 sides -> +1 heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int center = cid(&tg, COST_11);
        set_stage(&tg, 0, self, center, cid(&tg, COST_8));
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03) - before, 1,
                 "edges: center 11 > self 9 > right 8 -> +1 heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int self = cid(&tg, SUMIRE_R);
        int nine = cid(&tg, COST_9);
        set_stage(&tg, 0, self, nine, cid_new(&tg, COST_9));
        int before = heart(&tg, self, H03);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, self, H03), before,
                 "edges: center tied with right -> not strictly highest, no heart03");
    }

    /* center_highest_cost_constant_heart_empty_and_single_member_test.rs */
    {
        TestGame tg; test_game_new(&tg);
        int sumire = cid(&tg, SUMIRE_R);
        set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, sumire, H03), 0,
                 "empty stage leaves the source without a heart03 bonus");
    }
    {
        TestGame tg; test_game_new(&tg);
        int sumire = cid(&tg, SUMIRE_R);
        set_stage(&tg, 0, RB_EMPTY_SLOT, sumire, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, sumire, H03), 1,
                 "single center member is highest by default -> 1 heart03");
    }
}

/* ===================================================================== */
/* F. 本田奈央 PL!-bp5-003-R＋ — 3 distinct member names -> heart03        */
/*    three_distinct_names_constant_heart03_test.rs                        */
/*    three_distinct_stage_names_constant_heart03_test.rs                 */
/* ===================================================================== */

static void test_three_distinct_names_constant_heart03(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int kotori = cid(&tg, "PL!-bp5-003-R＋");
        int fa = cid(&tg, "PL!-sd1-010-SD");
        int fb = cid(&tg, "PL!-sd1-014-SD");
        set_stage(&tg, 0, kotori, fa, fb);
        test_recalc(&tg);
        CHECK(heart(&tg, kotori, H03) > 0,
              "Honoka: 3 distinct names on stage grants heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kotori = cid(&tg, "PL!-bp5-003-R＋");
        int f = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, kotori, f, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kotori, H03), 0,
                 "Honoka: only 2 distinct names on stage -> no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int honoka = cid(&tg, "PL!-bp5-003-R+");
        int m2 = cid(&tg, "PL!-sd1-001-SD");
        int m3 = cid(&tg, "PL!-sd1-002-SD");
        set_stage(&tg, 0, honoka, m2, m3);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, honoka, H03), 1,
                 "Honoka: exactly 3 distinct-named members -> +1 heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int honoka = cid(&tg, "PL!-bp5-003-R+");
        int m2 = cid(&tg, "PL!-sd1-002-SD");
        set_stage(&tg, 0, RB_EMPTY_SLOT, honoka, m2);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, honoka, H03), 0,
                 "Honoka: only 2 distinct members -> no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int honoka = cid(&tg, "PL!-bp5-003-R+");
        int same = cid_new(&tg, "PL!-bp5-003-R+");
        int m3 = cid(&tg, "PL!-sd1-002-SD");
        set_stage(&tg, 0, honoka, same, m3);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, honoka, H03), 0,
                 "Honoka: 2 copies of the same member are not 3 distinct names");
    }
    {
        TestGame tg; test_game_new(&tg);
        int honoka = cid(&tg, "PL!-bp5-003-R+");
        int m2 = cid(&tg, "PL!-sd1-001-SD");
        int m3 = cid(&tg, "PL!-sd1-002-SD");
        set_stage(&tg, 0, honoka, m2, m3);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, honoka, H03), 1, "Honoka: precondition, 3 distinct names -> 1");
        set_stage(&tg, 0, RB_EMPTY_SLOT, m2, m3);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, honoka, H03), 0,
                 "Honoka: the 3-name heart03 clears when the source leaves the stage");
    }
}

/* ===================================================================== */
/* G. 澁谷かのん PL!SP-bp5-012-N — Liella! live need-heart total >= 8      */
/*    group_live_need_heart_eight_constant_heart03_test.rs                */
/* ===================================================================== */

static void test_group_live_need_heart_eight_constant_heart03(void)
{
    /* PL!SP-bp1-024-L: Liella!, need_heart total 8. */
    /* PL!SP-bp1-023-L: Liella!, need_heart total 4. */
    /* PL!-sd1-019-SD: non-Liella, need_heart total 3. */
    {
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        test_add_to_stage(&tg, 1, kanon);
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 0,
                 "Kanon: empty live zone -> condition fails, heart03 = 0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int tiny_stars = cid(&tg, "PL!SP-bp1-024-L");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_live(&tg, tiny_stars);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 1,
                 "Kanon: Liella! live card with need_heart 8 -> +1 heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int start_true = cid(&tg, "PL!SP-bp1-023-L");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_live(&tg, start_true);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 0,
                 "Kanon: Liella! live card need_heart 4 (<8) -> no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int start_dash = cid(&tg, "PL!-sd1-019-SD");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_live(&tg, start_dash);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 0,
                 "Kanon: a non-Liella! live card fails the group filter -> no heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int a = cid(&tg, "PL!SP-bp1-023-L");
        int b = cid_new(&tg, "PL!SP-bp1-023-L");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_live(&tg, a);
        test_add_to_live(&tg, b);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 1,
                 "Kanon: two Liella! live cards (4+4=8) -> +1 heart03");
    }
    {
        /* The aggregate must be zone-aware: the same qualifying card in the
           SUCCESS zone must not satisfy the live_card_zone condition. */
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int tiny_stars = cid(&tg, "PL!SP-bp1-024-L");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_success(&tg, tiny_stars);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 0,
                 "Kanon: a card in the success zone is the wrong zone -> heart03 = 0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = cid(&tg, "PL!SP-bp5-012-N");
        int tiny_stars = cid(&tg, "PL!SP-bp1-024-L");
        int filler = cid(&tg, "PL!-sd1-010-SD");
        set_stage(&tg, 0, filler, kanon, RB_EMPTY_SLOT);
        test_add_to_live(&tg, tiny_stars);
        test_add_to_hand(&tg, filler);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 1, "Kanon: precondition, live card grants +1");
        tg.state.p[0].live.n = 0;
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, kanon, H03), 0,
                 "Kanon: the bonus is removed once the live card leaves the zone");
    }
}

/* ===================================================================== */
/* H. Combined stage member counts (6 members across both stages)          */
/*    combined_stage_count_constant_heart_test.rs                         */
/* ===================================================================== */

static void test_combined_stage_count_constant_heart(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int me = cid(&tg, "PL!SP-PR-022-PR");
        int f1 = cid_new(&tg, COST_4);
        int f2 = cid_new(&tg, COST_4);
        set_stage(&tg, 0, me, f1, f2);
        set_stage(&tg, 1, cid_new(&tg, COST_4), cid_new(&tg, COST_4), cid_new(&tg, COST_4));
        test_recalc(&tg);
        CHECK(heart(&tg, me, H02) > 0, "SP-PR-022: 6 combined members -> heart02");
        CHECK(heart(&tg, me, H03) > 0, "SP-PR-022: 6 combined members -> heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int me = cid(&tg, "PL!S-PR-042-PR");
        set_stage(&tg, 0, me, cid_new(&tg, COST_4), cid_new(&tg, COST_4));
        set_stage(&tg, 1, cid_new(&tg, COST_4), cid_new(&tg, COST_4), cid_new(&tg, COST_4));
        test_recalc(&tg);
        CHECK(heart(&tg, me, H02) > 0, "S-PR-042: 6 staged members -> heart02");
        CHECK(heart(&tg, me, H04) > 0, "S-PR-042: 6 staged members -> heart04");
    }
    {
        /* Karin: exactly 6 combined members -> exactly +1 heart02 and heart05. */
        TestGame tg; test_game_new(&tg);
        int karin = cid(&tg, "PL!N-PR-027-PR");
        CHECK(rb_card_no_eq(karin, "PL!N-PR-027-PR"),
              "Karin identity: PL!N-PR-027-PR resolves to the right card");
        set_stage(&tg, 0, karin, cid_new(&tg, COST_4), cid_new(&tg, COST_4));
        int p2_left = cid_new(&tg, COST_4);
        int p2_center = cid_new(&tg, COST_4);
        set_stage(&tg, 1, p2_left, p2_center, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, karin, H02), 0, "Karin: 5 combined members -> 0 heart02");
        CHECK_EQ(heart(&tg, karin, H05), 0, "Karin: 5 combined members -> 0 heart05");

        int p2_right = cid_new(&tg, COST_4);
        set_stage(&tg, 1, p2_left, p2_center, p2_right);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, karin, H02), 1, "Karin: exactly 6 members -> +1 heart02");
        CHECK_EQ(heart(&tg, karin, H05), 1, "Karin: exactly 6 members -> +1 heart05");
        CHECK_EQ(heart(&tg, karin, H01), 0, "Karin: no collateral heart01");

        set_stage(&tg, 1, p2_left, p2_center, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, karin, H02), 0, "Karin: back to 5 members -> 0 heart02");
        CHECK_EQ(heart(&tg, karin, H05), 0, "Karin: back to 5 members -> 0 heart05");
    }
    {
        /* Six physical instances of the SAME member count as six: the gate is
           a card count, not a distinct-name count. */
        TestGame tg; test_game_new(&tg);
        int k[6];
        k[0] = cid(&tg, "PL!N-PR-027-PR");
        for (int i = 1; i < 6; i++) k[i] = cid_new(&tg, "PL!N-PR-027-PR");
        for (int i = 1; i < 6; i++) CHECK(k[i] != k[i - 1], "Karin copies are distinct instances");
        set_stage(&tg, 0, k[0], k[1], k[2]);
        set_stage(&tg, 1, k[3], k[4], RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, k[0], H02), 0, "Karin x5: no heart02 below the 6-member gate");
        CHECK_EQ(heart(&tg, k[0], H05), 0, "Karin x5: no heart05 below the 6-member gate");

        set_stage(&tg, 1, k[3], k[4], k[5]);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, k[0], H02), 1, "Karin x6: +1 heart02 across both stages");
        CHECK_EQ(heart(&tg, k[0], H05), 1, "Karin x6: +1 heart05 across both stages");
    }
}

/* ===================================================================== */
/* I. PL!S-bp7-014-N — opponent energy ahead -> heart02                    */
/*    opponent_energy_ahead_constant_heart_test.rs                         */
/* ===================================================================== */

static void test_opponent_energy_ahead_constant_heart(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int me = cid(&tg, "PL!S-bp7-014-N");
        set_stage(&tg, 0, me, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        set_energy(&tg, 0, 1, 1);
        set_energy(&tg, 1, 3, 3);
        test_recalc(&tg);
        CHECK(heart(&tg, me, H02) > 0, "opponent energy ahead -> heart02 granted");
    }
    {
        TestGame tg; test_game_new(&tg);
        int me = cid(&tg, "PL!S-bp7-014-N");
        set_stage(&tg, 0, me, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        set_energy(&tg, 0, 2, 2);
        set_energy(&tg, 1, 2, 2);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, me, H02), 0, "tied energy -> no heart02 (strict >)");
    }
}

/* ===================================================================== */
/* J. A-RISE per-other-member heart05 — PL!-bp5-111-R                      */
/*    other_subunit_member_heart05_constant_test.rs                        */
/*    per_other_group_member_constant_heart_test.rs                       */
/* ===================================================================== */

static void test_per_other_group_member_constant_heart05(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int c = cid(&tg, "PL!-bp5-111-R");
        set_stage(&tg, 0, RB_EMPTY_SLOT, c, RB_EMPTY_SLOT);
        fill_decks(&tg, COST_4, 15);
        set_energy(&tg, 0, 5, 5);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, c, H05), 0,
                 "no other A-RISE member means no heart05 (self is excluded)");
    }
    {
        TestGame tg; test_game_new(&tg);
        int c = cid(&tg, "PL!-bp5-111-R");
        int anju = cid(&tg, "PL!-bp5-222-R");
        set_stage(&tg, 0, anju, c, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, c, H05), 1, "1 other A-RISE member grants exactly 1 heart05");
    }
    {
        TestGame tg; test_game_new(&tg);
        int member = cid(&tg, "PL!-bp5-111-R");
        set_stage(&tg, 0, cid(&tg, "PL!-bp5-222-R"), member, cid(&tg, "PL!-bp5-333-R"));
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, member, H05), 2,
                 "two other A-RISE members -> exactly +2 heart05");
    }
    {
        /* The count is per physical member, not per distinct name. */
        TestGame tg; test_game_new(&tg);
        int member = cid(&tg, "PL!-bp5-111-R");
        int a1 = cid_new(&tg, "PL!-bp5-333-R");
        int a2 = cid_new(&tg, "PL!-bp5-333-R");
        set_stage(&tg, 0, member, a1, a2);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, member, H05), 2,
                 "two copies of the same A-RISE member still grant +2 heart05");
    }
}

/* ===================================================================== */
/* K. 優木せつ菜 PL!N-bp7-007-R＋                                          */
/*    per_under_member_energy_constant_heart_test.rs                       */
/*    setsuna_under_member_and_energy_over_six_constant_hearts_test.rs    */
/* ===================================================================== */

static void test_setsuna_under_member_and_energy_over_six(void)
{
    /* ab#0: heart02 per energy card under this member. */
    {
        TestGame tg; test_game_new(&tg);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        int e = cid(&tg, "LL-E-001-SD");
        test_place_under(&tg, 0, 1, e);
        test_place_under(&tg, 0, 1, e);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 2, "2 under-member energies grant exactly +2 heart02");
    }
    {
        /* Only energy under THIS member counts. */
        TestGame tg; test_game_new(&tg);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        int e = cid(&tg, "LL-E-001-SD");
        int other_member = cid(&tg, "PL!SP-sd1-001-SD");
        set_stage(&tg, 0, other_member, s, RB_EMPTY_SLOT);
        test_place_under(&tg, 0, 0, e);
        test_place_under(&tg, 0, 1, other_member);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 0,
                 "energy under a DIFFERENT area plus a member under center -> still 0");
    }
    {
        TestGame tg; test_game_new(&tg);
        set_energy(&tg, 0, 4, 4);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 0, "ab#0: no energy under and energy <= 6 -> no heart02");
    }
    {
        TestGame tg; test_game_new(&tg);
        set_energy(&tg, 0, 4, 4);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        int e = cid(&tg, "LL-E-001-SD");
        test_place_under(&tg, 0, 1, e);
        test_place_under(&tg, 0, 1, e);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 2, "ab#0: 2 energy cards under -> 2 heart02");
    }

    /* ab#1: heart02 x (energy - 6) while energy > 6. */
    {
        TestGame tg; test_game_new(&tg);
        set_energy(&tg, 0, 6, 6);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 0, "ab#1: energy == 6 -> no heart02");
    }
    {
        TestGame tg; test_game_new(&tg);
        set_energy(&tg, 0, 4, 4);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 0, "ab#1: energy < 6 -> no heart02");
    }
    {
        TestGame tg; test_game_new(&tg);
        set_energy(&tg, 0, 8, 8);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 2, "ab#1: 8 - 6 = 2 heart02");
    }
    {
        TestGame tg; test_game_new(&tg);
        set_energy(&tg, 0, 12, 12);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 6, "ab#1: 12 - 6 = 6 heart02");
    }
    {
        TestGame tg; test_game_new(&tg);
        int s = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, s);
        set_energy(&tg, 0, 7, 7);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 1, "ab#1: 7 - 6 = 1 heart02");
        set_energy(&tg, 0, 12, 12);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 6, "ab#1: 12 - 6 = 6 heart02 after the gain rises");
        set_energy(&tg, 0, 6, 6);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, s, H02), 0, "ab#1: back to 6 energy -> 0 heart02");
    }
}

/* ===================================================================== */
/* L. PL!-pb1-002-R — heart06 per WAITED opponent member                  */
/*    per_waited_opponent_member_constant_heart_test.rs                   */
/* ===================================================================== */

static void test_per_waited_opponent_member_constant_heart06(void)
{
    TestGame tg; test_game_new(&tg);
    int member = cid(&tg, "PL!-pb1-002-R");
    int opp_waited = cid_new(&tg, "PL!-sd1-001-SD");
    test_set_opp_stage(&tg, 0, opp_waited);
    rb_mods_set_orientation(&tg.state.mods, opp_waited, "wait");
    stage_center(&tg, member);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H06), 1,
             "one waited opponent member -> exactly +1 heart06");
    CHECK_EQ(tg.state.p[0].stage[1], member, "setup guard: the constant source is in play");
}

/* ===================================================================== */
/* M. 冬木透 PL!SP-bp5-011-R — three hearts of the position's colour        */
/*    position_color_three_hearts_constant_test.rs                        */
/*    position_exclusive_constant_heart_color_switch_test.rs              */
/* ===================================================================== */

static void test_position_color_three_hearts_constant(void)
{
    TestGame tg; test_game_new(&tg);
    int member = cid(&tg, "PL!SP-bp5-011-R");

    set_stage(&tg, 0, member, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H02), 3, "left -> +3 heart02");
    CHECK_EQ(heart(&tg, member, H03), 0, "left -> heart03 must be 0");
    CHECK_EQ(heart(&tg, member, H05), 0, "left -> heart05 must be 0");

    set_stage(&tg, 0, RB_EMPTY_SLOT, member, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H03), 3, "center -> +3 heart03");
    CHECK_EQ(heart(&tg, member, H02), 0, "center -> the left heart02 grant is replaced");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, member);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H05), 3, "right -> +3 heart05");
    CHECK_EQ(heart(&tg, member, H03), 0, "right -> the center heart03 grant is replaced");
}

/* ===================================================================== */
/* N. 夏川natsumi PL!SP-bp7-009-R — heart02 on either side, not center     */
/*    side_position_constant_heart_test.rs                                 */
/*    side_position_constant_heart02_positive_test.rs                     */
/* ===================================================================== */

static void test_side_position_constant_heart02(void)
{
    TestGame tg; test_game_new(&tg);
    int member = cid(&tg, "PL!SP-bp7-009-R");
    CHECK(rb_card_no_eq(member, "PL!SP-bp7-009-R"),
          "natsumi identity: PL!SP-bp7-009-R resolves to the right card");

    stage_center(&tg, member);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H02), 0,
             "center is neither 左サイド nor 右サイド -> no heart02");

    set_stage(&tg, 0, member, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H02), 1, "左サイド -> heart02 +1, exactly one");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, member);
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, H02), 1, "右サイド -> heart02 +1 again, exactly one");
}

/* ===================================================================== */
/* O. 絵恋 PL!-bp5-333-P＋ / -R — heart05 while self is WAITED             */
/*    waited_self_heart05_constant_test.rs                                 */
/* ===================================================================== */

static void test_waited_self_heart05_constant(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int erena = cid(&tg, "PL!-bp5-333-P＋");
        set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        fill_decks(&tg, COST_4, 40);
        rb_mods_set_orientation(&tg.state.mods, erena, "wait");
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, erena, H05), 1,
                 "a waited self on the parallel print gains 1 heart05");
    }
    {
        TestGame tg; test_game_new(&tg);
        int erena = cid(&tg, "PL!-bp5-333-R");
        set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        fill_decks(&tg, COST_4, 40);
        rb_mods_set_orientation(&tg.state.mods, erena, "wait");
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, erena, H05), 1, "a waited self gains 1 heart05");
        rb_mods_set_orientation(&tg.state.mods, erena, "active");
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, erena, H05), 0,
                 "reactivating a waited self removes the constant heart05");
    }
}

/* ===================================================================== */
/* P. PL!SP-sd2-008-SD2 — a cost-13+ stage member gates heart03           */
/*    stage_member_cost_gated_constant_heart_test.rs                      */
/* ===================================================================== */

static void test_stage_member_cost_gated_constant_heart(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int me = cid(&tg, "PL!SP-sd2-008-SD2");
        int big = cid(&tg, "PL!HS-bp5-004-R");   /* cost 15 */
        set_stage(&tg, 0, me, big, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK(heart(&tg, me, H03) > 0, "a cost-15 member on stage -> self gains heart03");
    }
    {
        TestGame tg; test_game_new(&tg);
        int me = cid(&tg, "PL!SP-sd2-008-SD2");
        int small = cid(&tg, COST_4);            /* cost 4 */
        set_stage(&tg, 0, me, small, RB_EMPTY_SLOT);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, me, H03), 0, "no cost-13+ member on stage -> no heart03");
    }
}

/* ===================================================================== */
/* Q. 花陽 PL!-bp5-008-R — success-zone score sum >= 6 -> heart03 x2      */
/*    success_score_sum_constant_heart_test.rs                            */
/* ===================================================================== */

static void test_success_score_sum_constant_heart(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int hanayo = cid(&tg, "PL!-bp5-008-R");
        int live3 = cid(&tg, "PL!-sd1-021-SD");   /* printed score 3 */
        stage_center(&tg, hanayo);
        test_add_to_success(&tg, live3);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, hanayo, H03), 0, "score total 3 < 6 -> no heart03");

        int live3b = cid_new(&tg, "PL!-sd1-021-SD");
        test_add_to_success(&tg, live3b);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, hanayo, H03), 2, "score total exactly 6 -> +2 heart03");

        /* Stage aggregation must move by exactly the constant's 2. */
        int out_with[8];
        rb_calc_stage_hearts(&tg.state, 0, out_with);
        tg.state.p[0].success.n = 0;
        test_recalc(&tg);
        int out_off[8];
        rb_calc_stage_hearts(&tg.state, 0, out_off);
        CHECK_EQ(out_with[H03] - out_off[H03], 2,
                 "stage-aggregate heart03 grows by exactly the constant's 2");
        CHECK_EQ(heart(&tg, hanayo, H03), 0,
                 "the as_long_as bonus is removed once the total drops below 6");
    }
    {
        TestGame tg; test_game_new(&tg);
        int hanayo = cid(&tg, "PL!-bp5-008-R");
        int big_live = cid(&tg, "PL!S-pb1-023-L");   /* score 9 */
        stage_center(&tg, hanayo);
        test_add_to_success(&tg, big_live);
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, hanayo, H03), 2, "total 9 >= 6 still grants exactly +2");
        CHECK_EQ(heart(&tg, hanayo, H01), 0, "no collateral heart01");
        CHECK_EQ(rb_mods_get_blade(&tg.state.mods, hanayo), 0,
                 "no collateral blade either");
    }
    {
        TestGame tg; test_game_new(&tg);
        int hanayo = cid(&tg, "PL!-bp5-008-R");
        stage_center(&tg, hanayo);
        test_add_to_opp_success(&tg, cid_new(&tg, "PL!S-pb1-023-L"));
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, hanayo, H03), 0,
                 "the opponent's success-zone total must not count for you");
    }
    {
        /* aggregate = score, not card count. */
        TestGame tg; test_game_new(&tg);
        int hanayo = cid(&tg, "PL!-bp5-008-R");
        int zero_live = cid(&tg, "PL!-bp3-019-L");   /* printed score 0 */
        stage_center(&tg, hanayo);
        test_add_to_success(&tg, zero_live);
        for (int i = 0; i < 5; i++) test_add_to_success(&tg, cid_new(&tg, "PL!-bp3-019-L"));
        CHECK_EQ(tg.state.p[0].success.n, 6, "precondition: six live cards in the success zone");
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, hanayo, H03), 0,
                 "six score-0 lives give a score sum of 0 < 6 -> no hearts");
    }
    {
        TestGame tg; test_game_new(&tg);
        int hanayo = cid(&tg, "PL!-bp5-008-R");
        test_add_to_success(&tg, cid(&tg, "PL!SP-bp1-027-L"));  /* score 6 */
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, hanayo, H03), 0,
                 "the constant only works while its source is on the stage");
    }
    {
        TestGame tg; test_game_new(&tg);
        int a = cid(&tg, "PL!-bp5-008-R");
        int b = cid_new(&tg, "PL!-bp5-008-R");
        set_stage(&tg, 0, b, a, RB_EMPTY_SLOT);
        test_add_to_success(&tg, cid(&tg, "PL!SP-bp1-027-L"));
        test_recalc(&tg);
        CHECK_EQ(heart(&tg, a, H03), 2, "each of two copies independently gets +2 heart03");
        CHECK_EQ(heart(&tg, b, H03), 2, "each of two copies independently gets +2 heart03");
    }
}

/* ===================================================================== */
/* R. Success-zone live card of the matching subunit gates one heart       */
/*    success_zone_subunit_constant_heart_test.rs                         */
/* ===================================================================== */

static void success_zone_subunit_case(const char *member_no, const char *live_no,
                                      int color, const char *msg)
{
    TestGame tg; test_game_new(&tg);
    int member = cid(&tg, member_no);
    stage_center(&tg, member);

    /* Negative first: an empty success zone grants nothing. */
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, color), 0, msg);

    test_add_to_success(&tg, cid(&tg, live_no));
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, color), 1, msg);

    /* A non-matching subunit live card must drop the modifier back to 0. */
    tg.state.p[0].success.n = 0;
    test_add_to_success(&tg, cid_new(&tg, "PL!S-pb1-021-L"));
    test_recalc(&tg);
    CHECK_EQ(heart(&tg, member, color), 0, msg);
}

static void test_success_zone_subunit_constant_heart(void)
{
    success_zone_subunit_case("PL!-bp6-012-N", "PL!-pb1-028-L", H03,
        "a Printemps live in the success zone -> exactly +1 heart03");
    success_zone_subunit_case("PL!-bp6-014-N", "PL!-pb1-029-L", H01,
        "a lilywhite live in the success zone -> exactly +1 heart01");
    success_zone_subunit_case("PL!-bp6-015-N", "PL!-pb1-030-L", H06,
        "a BiBi live in the success zone -> exactly +1 heart06");
}

/* ===================================================================== */
/* S. 優木せつ菜 PL!N-pb1-007-R — all-six-heart-types -> icon_all heart    */
/*    six_heart_types_constant_all_heart_contribution_test.rs             */
/*    six_heart_types_constant_all_heart_matrix_test.rs                   */
/*    (the parts reachable without driving the live phases)               */
/* ===================================================================== */

static void test_six_heart_types_constant_all_heart(void)
{
    {
        /* setsuna_q205_no_bonus_outside_live: the temporal "during_live"
           condition must keep the icon_all heart out of the Main phase. */
        TestGame tg; test_game_new(&tg);
        int setsuna = cid(&tg, "PL!N-pb1-007-R");
        int filler = cid(&tg, COST_4);
        set_stage(&tg, 0, setsuna, filler, RB_EMPTY_SLOT);
        fill_decks(&tg, COST_4, 40);
        set_energy(&tg, 0, 15, 15);
        CHECK_EQ(heart(&tg, setsuna, HALL), 0, "no icon_all heart modifier outside the live phase");
    }
    {
        /* setsuna_pb1_verify_card_metadata: the card carries a 常時 ability
           with a decoded effect, and the grant is the icon_all colour. */
        int setsuna = rb_find_card_by_no("PL!N-pb1-007-R");
        CHECK(setsuna >= 0, "PL!N-pb1-007-R is in the database");
        if (setsuna >= 0) {
            CHECK(rb_card_no_eq(setsuna, "PL!N-pb1-007-R"),
                  "PL!N-pb1-007-R identity: the resolved card matches its number");
            int n = rb_card_num_abilities((uint32_t)setsuna);
            CHECK(n > 0, "PL!N-pb1-007-R has at least one ability");
            int found = 0;
            for (int i = 0; i < n && !found; i++) {
                Ability ab;
                memset(&ab, 0, sizeof ab);
                if (!rb_decode_card_ability((uint32_t)setsuna, i, &ab)) continue;
                if (ab.effect && ab.effect->action &&
                    !strcmp(ab.effect->action, "gain_resource")) {
                    const char *c = NULL;
                    for (int k = 0; k < ab.effect->n_extra; k++)
                        if (ab.effect->extra_k[k] && ab.effect->extra_v[k] &&
                            !strcmp(ab.effect->extra_k[k], "heart_color")) c = ab.effect->extra_v[k];
                    if (c && !strcmp(c, "all")) found = 1;
                }
                rb_free_ability(&ab);
            }
            CHECK(found, "the all-six-heart-types ability grants the icon_all (heart_color=all) heart");
        }
    }
    {
        /* the P＋ variant is a DIFFERENT card that must carry the same ability */
        int p_plus = rb_find_card_by_no("PL!N-pb1-007-P＋");
        CHECK(p_plus >= 0, "PL!N-pb1-007-P＋ (the P＋ variant) is in the database");
        if (p_plus >= 0) {
            CHECK(p_plus != rb_find_card_by_no("PL!N-pb1-007-R"),
                  "PL!N-pb1-007-P＋ and PL!N-pb1-007-R are distinct card records");
            int n = rb_card_num_abilities((uint32_t)p_plus);
            CHECK(n > 0, "PL!N-pb1-007-P＋ has at least one ability");
        }
    }
}

/* ── HC_TRACE diagnostic ─────────────────────────────────────────────────
 * HC_TRACE=<case> runs exactly one scenario with the engine's own [recalc]
 * trace enabled, so a failing constant can be attributed to "the condition
 * was never evaluated" vs "the condition evaluated wrongly". Read-only; it
 * never changes an assertion. Unset by default. */
static int trace_case(const char *which)
{
    TestGame tg; test_game_new(&tg);
    rb_ability_debug_set(1);
    if (!strcmp(which, "position")) {
        int m = cid(&tg, "PL!SP-bp5-011-R");
        set_stage(&tg, 0, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    } else if (!strcmp(which, "ten_energy")) {
        int m = cid(&tg, "PL!SP-bp5-016-N");
        stage_center(&tg, m);
        set_energy(&tg, 0, 0, 0);
    } else if (!strcmp(which, "wien")) {
        int m = cid(&tg, "PL!SP-pb2-032-N");
        stage_center(&tg, m);
        set_energy(&tg, 0, 0, 0);
    } else if (!strcmp(which, "per_unit")) {
        int m = cid(&tg, "PL!N-bp7-007-R＋");
        stage_center(&tg, m);
    } else if (!strcmp(which, "combined_energy")) {
        int m = cid(&tg, "PL!N-bp4-007-R＋");
        stage_center(&tg, m);
        set_energy(&tg, 0, 8, 8);
        set_energy(&tg, 1, 7, 7);
    } else if (!strcmp(which, "combined_stage")) {
        int m = cid(&tg, "PL!N-PR-027-PR");
        set_stage(&tg, 0, m, cid_new(&tg, COST_4), cid_new(&tg, COST_4));
        int a = cid_new(&tg, COST_4), b = cid_new(&tg, COST_4), c = cid_new(&tg, COST_4);
        set_stage(&tg, 1, a, b, c);
    } else if (!strcmp(which, "cost13")) {
        int m = cid(&tg, "PL!SP-sd2-008-SD2");
        set_stage(&tg, 0, m, cid(&tg, "PL!HS-bp5-004-R"), RB_EMPTY_SLOT);
    } else if (!strcmp(which, "opponent_energy")) {
        int m = cid(&tg, "PL!S-bp7-014-N");
        set_stage(&tg, 0, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        set_energy(&tg, 0, 2, 2);
        set_energy(&tg, 1, 2, 2);
    } else if (!strcmp(which, "distinct_names")) {
        int m = cid(&tg, "PL!-bp5-003-R+");
        set_stage(&tg, 0, m, cid_new(&tg, "PL!-bp5-003-R+"), cid(&tg, "PL!-sd1-002-SD"));
    } else {
        fprintf(stderr, "unknown HC_TRACE case \"%s\"\n", which);
        return 1;
    }
    test_recalc(&tg);
    rb_ability_debug_set(0);
    {
        int probe[8];
        int any = -1;
        for (int z = 0; z < 8; z++) {
            RbPlayer *P = &tg.state.p[0];
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (P->stage[s] != RB_EMPTY_SLOT)
                    probe[z] = rb_mods_get_heart(&tg.state.mods, P->stage[s], z);
            if (probe[z]) any = z;
        }
        printf("[trace] constant_heart by color:");
        for (int z = 0; z < 8; z++) printf(" c%d=%d", z, probe[z]);
        printf(" (last nonzero color=%d)\n", any);
    }
    return 0;
}

/* ── load the card DB ────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* HC_DUMP="<card_no>[,<card_no>...]" prints the decoded 常時 ability tree for
   each named card. Diagnostic only; the suite is unchanged when it is unset.
   (Same pattern as RB_DUMP_ABILITY in test_parity_draw_until_count.c.) */
static void dump_cond(const Condition *c, int depth);

static void dump_val(const CondValue *v)
{
    if (!v) return;
    switch (v->tag) {
    case RB_TAG_I64: printf("i64=%lld", (long long)v->i); break;
    case RB_TAG_STR: printf("str=\"%s\"", v->s ? v->s : "(null)"); break;
    case RB_TAG_TRUE: printf("true"); break;
    case RB_TAG_FALSE: printf("false"); break;
    case RB_TAG_ARRAY:
        printf("arr[");
        for (uint32_t i = 0; i < v->arr_n; i++) { if (i) printf(","); dump_val(&v->arr[i]); }
        printf("]");
        break;
    case RB_TAG_OBJVAR: printf("objvar(%d)", v->i); break;
    default: printf("tag=0x%02x", v->tag); break;
    }
}

static void dump_cond(const Condition *c, int depth)
{
    if (!c) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("cond variant=%d nfields=%u\n", c->variant, c->n_fields);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        for (int k = 0; k < depth + 1; k++) fputs("  ", stdout);
        printf("%s: ", c->fields[i].key ? c->fields[i].key : "?");
        dump_val(&c->fields[i].v);
        printf("\n");
        if (c->fields[i].v.cond) dump_cond(c->fields[i].v.cond, depth + 2);
    }
}

static void dump_effect(const char *tag, const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("%s action=%s target=%s count=%d per_unit=%d per_unit_count=%d "
           "distinct=%d repeat_limit=%d nchild=%d has_cond=%d\n",
           tag, e->action ? e->action : "(null)", e->target ? e->target : "(null)",
           e->count, e->per_unit, e->per_unit_count, e->distinct_flag,
           e->repeat_limit, e->n_child, e->has_condition);
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i])
            printf("      extra %s = %s\n", e->extra_k[i],
                   e->extra_v[i] ? e->extra_v[i] : "(null)");
    if (e->has_condition) dump_cond(e->condition, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_effect("child", e->child[i], depth + 1);
    if (e->primary_effect) dump_effect("primary", e->primary_effect, depth + 1);
    if (e->alternative_effect) dump_effect("alt", e->alternative_effect, depth + 1);
    if (e->followup_action) dump_effect("followup", e->followup_action, depth + 1);
    if (e->conditional_action) dump_effect("cond", e->conditional_action, depth + 1);
}

static void dump_card(const char *no)
{
    int c = rb_find_card_by_no(no);
    if (c < 0) { printf("=== %s : NOT IN DB\n", no); return; }
    Card card;
    printf("=== %s (id=%d)\n", no, c);
    if (!rb_decode_card_by_index((uint32_t)c, &card)) { printf("  decode failed\n"); return; }
    printf("  resolved_card_no=%s name=%s\n", rb_card_string(card.card_no_idx),
           card.name ? card.name : "(null)");
    printf("  cost=%d hearts:", card.cost);
    for (int i = 0; i < card.num_base; i++)
        printf(" %d:%d", card.heart_color[i], card.heart_count[i]);
    printf("\n");
    int n = rb_card_num_abilities((uint32_t)c);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)c, i, &ab)) continue;
        printf("  ab#%d triggers=%s\n", i, ab.triggers ? ab.triggers : "(null)");
        dump_effect("effect", ab.effect, 2);
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    if (getenv("HC_DUMP")) {
        const char *list = getenv("HC_DUMP");
        char buf[1024];
        snprintf(buf, sizeof buf, "%s", list);
        for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ","))
            dump_card(tok);
        rb_unload();
        return 0;
    }

    if (getenv("HC_TRACE")) {
        int rc = trace_case(getenv("HC_TRACE"));
        rb_unload();
        return rc;
    }

    test_ten_energy_constant_heart06();
    test_six_eight_energy_constant_heart06();
    test_energy_threshold_two_hearts_at_eight();
    test_energy_threshold_stacked_heart02();
    test_combined_energy_fifteen_heart02();
    test_center_highest_cost_constant_heart03();
    test_three_distinct_names_constant_heart03();
    test_group_live_need_heart_eight_constant_heart03();
    test_combined_stage_count_constant_heart();
    test_opponent_energy_ahead_constant_heart();
    test_per_other_group_member_constant_heart05();
    test_setsuna_under_member_and_energy_over_six();
    test_per_waited_opponent_member_constant_heart06();
    test_position_color_three_hearts_constant();
    test_side_position_constant_heart02();
    test_waited_self_heart05_constant();
    test_stage_member_cost_gated_constant_heart();
    test_success_score_sum_constant_heart();
    test_success_zone_subunit_constant_heart();
    test_six_heart_types_constant_all_heart();

    rb_unload();
    printf("\n%d assertions, %d failures\n", checks, failures);
    if (failures) return 1;
    printf("ALL HEARTS CONSTANTS PARITY CHECKS PASSED\n");
    return 0;
}
