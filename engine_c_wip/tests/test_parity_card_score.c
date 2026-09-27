/* Parity suite for engine/tests/test_modules/effects/score/card_score/ --
 * 24 of the 26 cluster files, against the C engine.
 *
 * test_parity_score_effects.c already owns the *direct* rb_execute_modify_score
 * level for PL!SP-pb2-045-L (LOVER) and PL!HS-bp2-020-L (Link to the FUTURE),
 * so those two cluster files are deliberately NOT re-ported here; every other
 * card_score file is.
 *
 * HARNESS NOTE (deliberate divergence from the Rust helpers, documented so the
 * numbers are not misread): the Rust `fire_trigger(game, cid, trigger, trig)`
 * helper dispatches ONE named ability of ONE card. The C port has no
 * ability-id-addressed entry point; the closest public surface is
 * rb_trigger_live_start(g, pl) / rb_trigger_live_success(g, pl), which scan the
 * live zone and the stage for every ability carrying that trigger. On the
 * fixtures below only the card under test carries such an ability, so the two
 * are equivalent -- but where that is not true the assertion is stronger, not
 * weaker. rb_should_trigger_live_success() additionally gates LiveSuccess on
 * g->live_success[pl]; the Rust helper bypasses that gate by dispatching the
 * ability directly, so the C helper sets live_success[pl] = 1 first, which is
 * the same bypass expressed through the C surface.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* -- helpers -------------------------------------------------------------- */

/* `src` is the in-tree layout; `../cards/build` is where the Makefile (and
   tools/isolated_build.sh) put the blobs, so an isolated out-of-tree build
   finds the database only through this second path. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Mirrors Rust `while has_pending_choice { select_indices(&[]) }` -- decline /
   select-nothing. Auto-ability ordering prompts answer "proceed" (index 0),
   matching test_drain_auto_choices(). */
static void drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        if (ch && ch->kind == RB_CHOICE_SELECT_AUTO_ABILITY)
            test_resume_choice(tg, 0);
        else
            test_resume_choice(tg, -1);
        rb_process_pending_auto_abilities(&tg->state);
    }
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

/* Rust `select_option(1)` on a pay_optional_cost SelectTarget prompt == the
   "pay" branch. The established C idiom (test_parity_integration.c) is
   rb_resume_with_choice(1). */
static int accept_optional_cost(TestGame *tg)
{
    if (!test_has_pending_choice(tg)) return 0;
    const char *t = test_pending_choice_type(tg);
    if (t && strcmp(t, "SelectTarget") != 0) return 0;
    test_resume_choice(tg, 1);
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static int decline_optional_cost(TestGame *tg)
{
    if (!test_has_pending_choice(tg)) return 0;
    const char *t = test_pending_choice_type(tg);
    if (t && strcmp(t, "SelectTarget") != 0) return 0;
    test_resume_choice(tg, -1);
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* Fire every LiveStart ability on `pl`'s live zone + stage. */
static void fire_live_start(TestGame *tg, int pl, int cid)
{
    tg->state.activating_card = cid;
    rb_trigger_live_start(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

/* Fire every LiveSuccess ability on `pl`'s live zone + stage, with the
   LiveSuccess window forced open (the Rust fire_trigger helper bypasses the
   same gate). */
static void fire_live_success(TestGame *tg, int pl, int cid)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[pl] = 1;
    tg->state.activating_card = cid;
    rb_trigger_live_success(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

static int score_of(TestGame *tg, int cid)
{
    return test_get_score_modifier(tg, cid);
}

/* ---- TEMPORARY DIAGNOSTIC (removed before commit) ---- */
static void dbg_card(const char *tag, int cid)
{
    Card c;
    if (cid < 0 || cid == RB_EMPTY_SLOT) { printf("DBG %s: <empty>\n", tag); return; }
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { printf("DBG %s: <nodecode %d>\n", tag, cid); return; }
    printf("DBG %s: cid=%d no=%s name=%s group=%s cost=%d blade=%d score=%d\n",
           tag, cid, c.card_no, c.name, c.group, (int)c.cost, (int)c.blade, (int)c.score);
    rb_free_card(&c);
}
static void dbg_stage(TestGame *tg, int pl, const char *tag)
{
    printf("DBG %s: pl=%d stage=[%d,%d,%d] wait=%d disc=%d under=%d deck=%d live=%d succ=%d\n",
           tag, pl,
           tg->state.p[pl].stage[0], tg->state.p[pl].stage[1], tg->state.p[pl].stage[2],
           tg->state.p[pl].waitroom.n, tg->state.p[pl].discard.n,
           0, tg->state.p[pl].deck.n, tg->state.p[pl].live.n, tg->state.p[pl].success.n);
    for (int i = 0; i < 3; i++) {
        char t[32]; snprintf(t, sizeof t, "%s.stage[%d]", tag, i);
        dbg_card(t, tg->state.p[pl].stage[i]);
    }
    for (int i = 0; i < tg->state.p[pl].waitroom.n; i++) {
        char t[32]; snprintf(t, sizeof t, "%s.wait[%d]", tag, i);
        dbg_card(t, tg->state.p[pl].waitroom.cards[i]);
    }
    for (int i = 0; i < tg->state.p[pl].live.n; i++) {
        char t[32]; snprintf(t, sizeof t, "%s.live[%d]", tag, i);
        dbg_card(t, tg->state.p[pl].live.cards[i]);
    }
}
#define DBG(...) do { printf(__VA_ARGS__); fflush(stdout); } while (0)

static int base_score_of(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int s = (int)c.score;
    rb_free_card(&c);
    return s;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) {
        test_add_to_deck(tg, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust advance_to_live_card_set_p1: 5 passes out of Main. */
static void advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) {
        drain_skip(tg);
        test_pass(tg);
    }
    drain_skip(tg);
}

/* Rust finish_live_setup: LiveCardSetP1 -> LiveCardSetP2 -> LiveStart. */
static void finish_live_setup(TestGame *tg)
{
    drain_skip(tg);
    test_pass(tg);
    drain_skip(tg);
    test_pass(tg);
    drain_skip(tg);
}

/* Full performance + victory determination, mirroring run_performance() in
   test_live_success_rules.c. */
static void run_performance(TestGame *tg)
{
    tg->state.live_batch_mode = 1;
    rb_perform_live(&tg->state, 0);
    rb_perform_live(&tg->state, 1);
    tg->state.live_batch_mode = 0;
    rb_execute_live_victory_determination(&tg->state);
    drain_first(tg);
    if (!test_has_pending_choice(tg) && tg->state.p[0].live.n > 0) {
        int p1 = 0, p2 = 0;
        rb_determine_live_winners(&tg->state, &p1, &p2);
        rb_process_player_live_result(&tg->state, 0, p1, 0, p1);
        rb_process_player_live_result(&tg->state, 1, p2, 0, p2);
    }
    drain_first(tg);
}

/* Index of the p1 performance snapshot holding `cid`, or -1. */
static int snapshot_index_of(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_snapshots; i++)
        for (int j = 0; j < tg->state.snapshots[i].n_lives; j++)
            if (tg->state.snapshots[i].lives[j] == cid) return i;
    return -1;
}

/* Rust TestGame::assert_card_identity -- pins a staged card to its card_no. */
static void assert_identity(TestGame *tg, int cid, const char *no, const char *what)
{
    assertions++;
    if (!rb_card_no_eq(cid, no)) {
        fprintf(stderr, "FAIL: %s: card identity is not %s (id=%d)\n", what, no, cid);
        failures++;
    } else {
        printf("ok: %s: card identity is %s\n", what, no);
    }
}

/* Rust assert_distinct_card_names / assert_same_card_name. */
static void assert_distinct_names(TestGame *tg, int a, int b, const char *what)
{
    assertions++;
    if (strcmp(test_card_name(a), test_card_name(b)) == 0) {
        fprintf(stderr, "FAIL: %s: fixtures must be differently-named cards\n", what);
        failures++;
    } else {
        printf("ok: %s: fixtures are differently-named cards\n", what);
    }
}

static void assert_same_names(TestGame *tg, int a, int b, const char *what)
{
    assertions++;
    if (strcmp(test_card_name(a), test_card_name(b)) != 0) {
        fprintf(stderr, "FAIL: %s: fixtures must be the same card name\n", what);
        failures++;
    } else {
        printf("ok: %s: fixtures share a card name\n", what);
    }
}

/* ------------------------------------------------------------------------
 * liella_heart_total_score_test.rs -- PL!SP-bp5-026-L
 *   LiveStart: Liella! members' BASE hearts total >= 11 -> score +1.
 * ---------------------------------------------------------------------- */
static void test_liella_heart_total_score(void)
{
    {   /* two Liella! members, 9 + 8 = 17 >= 11 */
        TestGame tg; test_game_new(&tg);
        int filler = test_new_id(&tg, "PL!-sd1-010-SD");
        fill_decks(&tg, filler, 10);
        int live = test_id(&tg, "PL!SP-bp5-026-L");
        test_add_to_live(&tg, live);
        int big1 = test_id(&tg, "PL!SP-pb2-005-R");
        int big2 = test_id(&tg, "PL!SP-bp4-004-P");
        assert_identity(&tg, big1, "PL!SP-pb2-005-R", "Liella heart fixture 1");
        assert_identity(&tg, big2, "PL!SP-bp4-004-P", "Liella heart fixture 2");
        tg.state.p[0].stage[0] = big1;
        tg.state.p[0].stage[1] = big2;
        tg.state.p[0].stage[2] = filler;
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 1,
                 "PL!SP-bp5-026-L: two Liella! members' base hearts total 17 >= 11 -> +1");
    }
    {   /* two low-heart Liella! members, 2 + 2 = 4 < 11 */
        TestGame tg; test_game_new(&tg);
        int filler = test_new_id(&tg, "PL!-sd1-010-SD");
        fill_decks(&tg, filler, 10);
        int live = test_id(&tg, "PL!SP-bp5-026-L");
        test_add_to_live(&tg, live);
        int l1 = test_id(&tg, "PL!SP-pb2-036-N");
        int l2 = test_id(&tg, "PL!SP-pb2-037-N");
        assert_identity(&tg, l1, "PL!SP-pb2-036-N", "low-heart Liella fixture 1");
        assert_identity(&tg, l2, "PL!SP-pb2-037-N", "low-heart Liella fixture 2");
        tg.state.p[0].stage[0] = l1;
        tg.state.p[0].stage[1] = l2;
        tg.state.p[0].stage[2] = filler;
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0,
                 "PL!SP-bp5-026-L: Liella heart total 4 < 11 -> +0");
    }
}

/* ------------------------------------------------------------------------
 * live_start_center_cost_comparison_live_score_test.rs -- PL!SP-bp4-024-L
 *   LiveStart: own CENTER Liella! member cost > opponent's center member
 *   cost -> +1. Drives the whole condition truth table, both seats.
 *   (test_parity_score_effects.c only rb_eval_condition_for_host()s three of
 *   these rows; here the real phase flow and the P2 actor are exercised.)
 * ---------------------------------------------------------------------- */
static void nonfiction_p1(int p1_center, int p2_center, int expected, const char *what)
{
    TestGame tg; test_game_new(&tg);
    int nonfiction = test_id(&tg, "PL!SP-bp4-024-L");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, nonfiction);
    if (p1_center >= 0) tg.state.p[0].stage[1] = p1_center;
    if (p2_center >= 0) tg.state.p[1].stage[1] = p2_center;
    fill_decks(&tg, filler, 30);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, nonfiction);
    /* The phase machine itself dispatches LiveStart when it enters
       FirstAttackerPerformance; an extra rb_trigger_live_start() here would
       fire the same ability twice and double the score. */
    finish_live_setup(&tg);
    drain_skip(&tg);
    CHECK_EQ(score_of(&tg, nonfiction), expected, what);
}

static void nonfiction_p2(int p1_center, int p2_center, int expected, const char *what)
{
    TestGame tg; test_game_new(&tg);
    int nonfiction = test_id(&tg, "PL!SP-bp4-024-L");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int p1_live = test_id(&tg, "PL!-sd1-020-SD");
    if (p1_center >= 0) tg.state.p[0].stage[1] = p1_center;
    if (p2_center >= 0) tg.state.p[1].stage[1] = p2_center;
    tg.state.p[1].hand.cards[tg.state.p[1].hand.n++] = nonfiction;
    test_add_to_hand(&tg, p1_live);
    fill_decks(&tg, filler, 30);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, p1_live);
    drain_skip(&tg);
    test_pass(&tg);
    tg.state.p[1].live.cards[0] = nonfiction;
    tg.state.p[1].live.n = 1;
    drain_skip(&tg);
    test_pass(&tg);
    /* The phase machine dispatches P2's LiveStart itself when the second
       attacker starts; no extra rb_trigger_live_start() here. */
    drain_skip(&tg);
    DBG("DBG nonfiction_p2 p1c=%d p2c=%d exp=%d got=%d\n", p1_center, p2_center, expected, score_of(&tg, nonfiction));
    dbg_card("nfp2 p1.center", tg.state.p[0].stage[1]);
    dbg_card("nfp2 p2.center", tg.state.p[1].stage[1]);
    CHECK_EQ(score_of(&tg, nonfiction), expected, what);
}
static void test_nonfiction_center_cost_comparison(void)
{
    {   /* fixture identity guard: the three costs the truth table rests on */
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        int rich = test_id(&tg, "PL!SP-bp4-004-R＋");
        assert_identity(&tg, kanon, "PL!SP-pb1-001-R", "center-cost fixture (cost 11)");
        assert_identity(&tg, filler, "PL!-sd1-010-SD", "center-cost fixture (cost 4)");
        assert_identity(&tg, rich, "PL!SP-bp4-004-R＋", "center-cost fixture (cost 22)");
        Card a, b, c;
        rb_decode_card_by_index((uint32_t)kanon, &a);
        rb_decode_card_by_index((uint32_t)filler, &b);
        rb_decode_card_by_index((uint32_t)rich, &c);
        CHECK_EQ(a.cost, 11, "center-cost fixture PL!SP-pb1-001-R really costs 11");
        CHECK_EQ(b.cost, 4, "center-cost fixture PL!-sd1-010-SD really costs 4");
        CHECK_EQ(c.cost, 22, "center-cost fixture PL!SP-bp4-004-R＋ really costs 22");
        rb_free_card(&a); rb_free_card(&b); rb_free_card(&c);
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        nonfiction_p1(kanon, filler, 1,
                       "PL!SP-bp4-024-L: P1 center cost 11 > P2 center cost 4 -> +1");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        nonfiction_p1(kanon, -1, 1,
                       "PL!SP-bp4-024-L: P1 center cost 11 > P2 empty center (0) -> +1");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        nonfiction_p1(filler, kanon, 0,
                       "PL!SP-bp4-024-L: P1 center cost 4 < P2 center cost 11 -> +0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        nonfiction_p1(filler, filler, 0,
                       "PL!SP-bp4-024-L: P1 center is not Liella! -> +0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int kanon2 = test_new_id(&tg, "PL!SP-pb1-001-R");
        nonfiction_p1(kanon, kanon2, 0,
                       "PL!SP-bp4-024-L: P1 cost 11 == P2 cost 11 -> +0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int rich = test_id(&tg, "PL!SP-bp4-004-R＋");
        nonfiction_p1(kanon, rich, 0,
                       "PL!SP-bp4-024-L: P1 cost 11 < P2 cost 22 -> +0");
    }
    /* P2 as the actor -- this is the target:both -> self regression. */
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        nonfiction_p2(filler, kanon, 1,
                       "PL!SP-bp4-024-L: P2 center cost 11 > P1 center cost 4 -> +1");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        nonfiction_p2(-1, kanon, 1,
                       "PL!SP-bp4-024-L: P2 center cost 11 > P1 empty center (0) -> +1");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        nonfiction_p2(kanon, filler, 0,
                       "PL!SP-bp4-024-L: P2 center cost 4 < P1 center cost 11 -> +0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int kanon = test_id(&tg, "PL!SP-pb1-001-R");
        int kanon2 = test_new_id(&tg, "PL!SP-pb1-001-R");
        nonfiction_p2(kanon2, kanon, 0,
                       "PL!SP-bp4-024-L: P2 cost 11 == P1 cost 11 -> +0");
    }
}

/* ------------------------------------------------------------------------
 * live_start_reciprocal_live_heart01_threshold_live_score_test.rs
 *   PL!N-pb1-038-L (PHOENIX) and PL!N-pb1-039-L (Stellar Stream) fire
 *   reciprocally at LiveStart, and 9.3.4.3 says an ability is only active
 *   while its own card is in the LIVE zone.
 *   (The "chooses one of several members" case is already covered by
 *   test_ported_generated.c::generated_target_selection and is not repeated.)
 * ---------------------------------------------------------------------- */
static void niji_stage(TestGame *tg, int member)
{
    tg->state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg->state.p[0].stage[1] = member;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
    /* heart01=7, heart02=2, heart06=6, heart00=10 (Rust HeartMap) */
    tg->state.stage_hearts[0][RB_HEART_PINK] = 7;
    tg->state.stage_hearts[0][RB_HEART_RED] = 2;
    tg->state.stage_hearts[0][RB_HEART_ORANGE] = 6;
    tg->state.stage_hearts[0][RB_HEART_ALL] = 10;
    tg->state.phase = RB_PHASE_LIVE_SET;
}

static void test_phoenix_stellar_reciprocal(void)
{
    {   /* both in the live zone: each sees the other and both resolve */
        TestGame tg; test_game_new(&tg);
        int member = test_id(&tg, "PL!N-PR-003-PR");
        int phoenix = test_id(&tg, "PL!N-pb1-038-L");
        int stellar = test_id(&tg, "PL!N-pb1-039-L");
        assert_identity(&tg, phoenix, "PL!N-pb1-038-L", "PHOENIX fixture");
        assert_identity(&tg, stellar, "PL!N-pb1-039-L", "Stellar Stream fixture");
        niji_stage(&tg, member);
        test_add_to_live(&tg, phoenix);
        test_add_to_live(&tg, stellar);
        fire_live_start(&tg, 0, phoenix);
        drain_first(&tg);
        CHECK_EQ(score_of(&tg, phoenix), 1,
                 "PHOENIX +1 score from Stellar Stream (heart01=4) in the live zone");
        CHECK_EQ(test_get_heart_modifier(&tg, member, RB_HEART_ORANGE), 4,
                 "Stellar Stream +4 heart06 to the member from PHOENIX in the live zone");
    }
    {   /* PHOENIX live / Stellar success: the condition searches both zones,
           but Stellar's own ability is inactive outside the live zone. */
        TestGame tg; test_game_new(&tg);
        int member = test_id(&tg, "PL!N-PR-003-PR");
        int phoenix = test_id(&tg, "PL!N-pb1-038-L");
        int stellar = test_id(&tg, "PL!N-pb1-039-L");
        niji_stage(&tg, member);
        test_add_to_live(&tg, phoenix);
        test_add_to_success(&tg, stellar);
        fire_live_start(&tg, 0, phoenix);
        drain_first(&tg);
        CHECK_EQ(score_of(&tg, phoenix), 1,
                 "PHOENIX in live zone finds Stellar Stream in the success zone -> +1");
        CHECK_EQ(test_get_heart_modifier(&tg, member, RB_HEART_ORANGE), 0,
                 "Stellar Stream in the success zone: ability inactive, no heart06");
    }
    {   /* the mirror image */
        TestGame tg; test_game_new(&tg);
        int member = test_id(&tg, "PL!N-PR-003-PR");
        int phoenix = test_id(&tg, "PL!N-pb1-038-L");
        int stellar = test_id(&tg, "PL!N-pb1-039-L");
        niji_stage(&tg, member);
        test_add_to_success(&tg, phoenix);
        test_add_to_live(&tg, stellar);
        fire_live_start(&tg, 0, stellar);
        drain_first(&tg);
        CHECK_EQ(score_of(&tg, phoenix), 0,
                 "PHOENIX in the success zone: ability inactive, no score");
        CHECK_EQ(test_get_heart_modifier(&tg, member, RB_HEART_ORANGE), 4,
                 "Stellar Stream in the live zone finds PHOENIX in the success zone -> +4 heart06");
    }
    {   /* both in the success zone: neither fires */
        TestGame tg; test_game_new(&tg);
        int member = test_id(&tg, "PL!N-PR-003-PR");
        int phoenix = test_id(&tg, "PL!N-pb1-038-L");
        int stellar = test_id(&tg, "PL!N-pb1-039-L");
        niji_stage(&tg, member);
        test_add_to_success(&tg, phoenix);
        test_add_to_success(&tg, stellar);
        fire_live_start(&tg, 0, phoenix);
        drain_first(&tg);
        CHECK_EQ(score_of(&tg, phoenix), 0, "PHOENIX in the success zone: no fire");
        CHECK_EQ(test_get_heart_modifier(&tg, member, RB_HEART_ORANGE), 0,
                 "Stellar Stream in the success zone: no fire");
    }
    {   /* a non-Nijigasaki live card satisfies neither reciprocal condition */
        TestGame tg; test_game_new(&tg);
        int member = test_id(&tg, "PL!N-PR-003-PR");
        int phoenix = test_id(&tg, "PL!N-pb1-038-L");
        int filler_live = test_id(&tg, "PL!-sd1-019-SD");
        niji_stage(&tg, member);
        test_add_to_live(&tg, phoenix);
        test_add_to_live(&tg, filler_live);
        fire_live_start(&tg, 0, phoenix);
        drain_first(&tg);
        dbg_card("phoenix filler_live", filler_live);
        dbg_stage(&tg, 0, "phoenixneg");
        CHECK_EQ(score_of(&tg, phoenix), 0,
                 "PHOENIX gets no score: the other live card is not Nijigasaki");
        CHECK_EQ(test_get_heart_modifier(&tg, member, RB_HEART_ORANGE), 0,
                 "Stellar Stream does not trigger: the other live card is not Nijigasaki");
    }
}

/* ------------------------------------------------------------------------
 * live_start_distinct_member_name_and_cost_score_test.rs -- PL!HS-bp5-018-L
 *   AURORA: 3 stage members with 3 distinct NAMES and 3 distinct COSTS -> +1.
 * ---------------------------------------------------------------------- */
static void aurora_flower(int a, int b, int c, int expected, const char *what)
{
    TestGame tg; test_game_new(&tg);
    int aurora = test_id(&tg, "PL!HS-bp5-018-L");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[1] = b;
    tg.state.p[0].stage[2] = c;
    test_add_to_hand(&tg, aurora);
    fill_decks(&tg, filler, 10);
    tg.state.p[1].hand.cards[tg.state.p[1].hand.n++] = filler;
    test_give_energy(&tg, 10);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, aurora);
    finish_live_setup(&tg);
    drain_skip(&tg);
    dbg_card("aurora a", tg.state.p[0].stage[0]);
    dbg_card("aurora b", tg.state.p[0].stage[1]);
    dbg_card("aurora c", tg.state.p[0].stage[2]);
    CHECK_EQ(score_of(&tg, aurora), expected, what);
}

static void test_aurora_distinct_name_and_cost(void)
{
    {
        TestGame tg; test_game_new(&tg);
        int m_a = test_id(&tg, "PL!HS-bp1-012-PR");
        int m_b = test_id(&tg, "PL!HS-bp5-012-N");
        int m_c = test_id(&tg, "PL!HS-bp5-010-N");
        assert_distinct_names(&tg, m_a, m_b, "AURORA 3-distinct fixture");
        assert_distinct_names(&tg, m_b, m_c, "AURORA 3-distinct fixture");
        Card a, b, c;
        rb_decode_card_by_index((uint32_t)m_a, &a);
        rb_decode_card_by_index((uint32_t)m_b, &b);
        rb_decode_card_by_index((uint32_t)m_c, &c);
        CHECK_EQ(a.cost, 4, "AURORA fixture PL!HS-bp1-012-PR really costs 4");
        CHECK_EQ(b.cost, 5, "AURORA fixture PL!HS-bp5-012-N really costs 5");
        CHECK_EQ(c.cost, 7, "AURORA fixture PL!HS-bp5-010-N really costs 7");
        rb_free_card(&a); rb_free_card(&b); rb_free_card(&c);
        aurora_flower(m_a, m_b, m_c, 1,
                      "AURORA: 3 members with distinct names AND distinct costs -> +1");
    }
    {
        TestGame tg; test_game_new(&tg);
        int m_a = test_id(&tg, "PL!HS-bp1-012-PR");
        int m_b = test_id(&tg, "PL!HS-PR-004-PR");
        int m_c = test_id(&tg, "PL!HS-bp5-010-N");
        assert_distinct_names(&tg, m_a, m_b, "AURORA same-cost fixture");
        Card a, b;
        rb_decode_card_by_index((uint32_t)m_a, &a);
        rb_decode_card_by_index((uint32_t)m_b, &b);
        CHECK_EQ(a.cost, 4, "AURORA same-cost fixture PL!HS-bp1-012-PR costs 4");
        CHECK_EQ(b.cost, 4, "AURORA same-cost fixture PL!HS-PR-004-PR costs 4");
        rb_free_card(&a); rb_free_card(&b);
        aurora_flower(m_a, m_b, m_c, 0,
                      "AURORA: only 2 distinct costs -> +0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int m_a = test_id(&tg, "PL!HS-sd1-007-SD");
        int m_b = test_id(&tg, "PL!HS-bp1-020-N");
        int m_c = test_id(&tg, "PL!HS-bp5-010-N");
        assert_same_names(&tg, m_a, m_b, "AURORA same-name fixture");
        Card a, b;
        rb_decode_card_by_index((uint32_t)m_a, &a);
        rb_decode_card_by_index((uint32_t)m_b, &b);
        CHECK_EQ(a.cost, 4, "AURORA same-name fixture PL!HS-sd1-007-SD costs 4");
        CHECK_EQ(b.cost, 15, "AURORA same-name fixture PL!HS-pb1-023-N costs 15");
        rb_free_card(&a); rb_free_card(&b);
        aurora_flower(m_a, m_b, m_c, 0,
                      "AURORA: only 2 distinct names -> +0");
    }
    {
        TestGame tg; test_game_new(&tg);
        int m_a = test_id(&tg, "PL!HS-bp1-012-PR");
        int m_c = test_id(&tg, "PL!HS-bp5-010-N");
        aurora_flower(m_a, RB_EMPTY_SLOT, m_c, 0,
                      "AURORA: only 2 members on stage -> +0");
    }
}

/* ------------------------------------------------------------------------
 * live_start_total_blade_threshold_live_score_q116_test.rs -- PL!N-sd1-028-SD
 *   Dream with You: total stage blade >= 10 at LiveStart -> +1, independent of
 *   how many cards the yell actually revealed.
 * ---------------------------------------------------------------------- */
static void test_dream_with_you_total_blade(void)
{
    {   /* 6 + 6 = 12 blade */
        TestGame tg; test_game_new(&tg);
        int dream = test_id(&tg, "PL!N-sd1-028-SD");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        int blader = test_id(&tg, "PL!S-PR-014-PR");
        int blader2 = test_new_id(&tg, "PL!S-PR-014-PR");
        assert_identity(&tg, blader, "PL!S-PR-014-PR", "Dream with You blader fixture");
        Card c; rb_decode_card_by_index((uint32_t)blader, &c);
        CHECK_EQ(c.blade, 6, "PL!S-PR-014-PR really has 6 blade");
        rb_free_card(&c);
        fill_decks(&tg, filler, 40);
        tg.state.p[0].stage[0] = blader;
        tg.state.p[0].stage[1] = blader2;
        tg.state.p[0].stage[2] = filler;
        test_add_to_hand(&tg, dream);
        test_add_to_hand(&tg, filler);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, dream);
        finish_live_setup(&tg);
        drain_skip(&tg);
        dbg_stage(&tg, 0, "dream12");
        CHECK_EQ(score_of(&tg, dream), 1,
                 "Dream with You: total stage blade 12 >= 10 -> +1");
    }
    {   /* 6 + 1 = 7 blade */
        TestGame tg; test_game_new(&tg);
        int dream = test_id(&tg, "PL!N-sd1-028-SD");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        int blader = test_id(&tg, "PL!S-PR-014-PR");
        int low = test_id(&tg, "PL!-sd1-002-SD");
        assert_identity(&tg, low, "PL!-sd1-002-SD", "Dream with You low-blade fixture");
        Card c; rb_decode_card_by_index((uint32_t)low, &c);
        CHECK_EQ(c.blade, 1, "PL!-sd1-002-SD really has 1 blade");
        rb_free_card(&c);
        fill_decks(&tg, filler, 40);
        tg.state.p[0].stage[0] = blader;
        tg.state.p[0].stage[1] = low;
        tg.state.p[0].stage[2] = filler;
        test_add_to_hand(&tg, dream);
        test_add_to_hand(&tg, filler);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, dream);
        finish_live_setup(&tg);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, dream), 0,
                 "Dream with You: total stage blade 7 < 10 -> +0");
    }
    {   /* Q116: Wien's reduced yell count must not block the threshold check */
        TestGame tg; test_game_new(&tg);
        int dream = test_id(&tg, "PL!N-sd1-028-SD");
        int wien = test_id(&tg, "PL!SP-bp2-010-R＋");
        int blader = test_id(&tg, "PL!S-PR-014-PR");
        int blader2 = test_new_id(&tg, "PL!S-PR-014-PR");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        assert_identity(&tg, wien, "PL!SP-bp2-010-R＋", "Q116 Wien fixture");
        fill_decks(&tg, filler, 40);
        tg.state.p[0].stage[0] = wien;
        tg.state.p[0].stage[1] = blader;
        tg.state.p[0].stage[2] = blader2;
        test_add_to_hand(&tg, dream);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, dream);
        finish_live_setup(&tg);
        drain_skip(&tg);
        CHECK(score_of(&tg, dream) >= 1,
               "Q116: the total-blade threshold is independent of Wien's reduced yell count");
    }
}

/* ------------------------------------------------------------------------
 * live_start_unique_group_heart_color_live_score_test.rs -- PL!N-bp1-027-L
 *   Solitude Rain: +1 per DISTINCT printed heart colour across Nijigasaki
 *   members. (The `members_no_matching_colors` Rust case is a self-declared
 *   duplicate of `one_member_two_colors` and is not repeated.)
 * ---------------------------------------------------------------------- */
static void solitude_rain_stage(TestGame *tg, int rain, int a, int b, int c)
{
    int filler = test_id(tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 40; i++) test_add_to_deck(tg, filler);
    tg->state.p[0].stage[0] = a;
    tg->state.p[0].stage[1] = b;
    tg->state.p[0].stage[2] = c;
    test_add_to_live(tg, rain);
    tg->state.phase = RB_PHASE_PERFORMANCE;
    fire_live_start(tg, 0, rain);
    drain_skip(tg);
}

static void test_solitude_rain_unique_heart_color(void)
{
    {   /* no members at all */
        TestGame tg; test_game_new(&tg);
        int rain = test_id(&tg, "PL!N-bp1-027-L");
        solitude_rain_stage(&tg, rain, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        CHECK_EQ(score_of(&tg, rain), 0, "Solitude Rain: empty stage -> +0");
    }
    {   /* PL!N-sd1-019-PR: heart02 + heart06 */
        TestGame tg; test_game_new(&tg);
        int rain = test_id(&tg, "PL!N-bp1-027-L");
        int m = test_id(&tg, "PL!N-sd1-019-PR");
        assert_identity(&tg, m, "PL!N-sd1-019-PR", "Solitude Rain 2-colour fixture");
        solitude_rain_stage(&tg, rain, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        CHECK_EQ(score_of(&tg, rain), 2, "Solitude Rain: heart02 + heart06 -> +2");
    }
    {   /* A: heart02+heart06, B: heart02+heart05 -> unique {02,05,06} = 3 */
        TestGame tg; test_game_new(&tg);
        int rain = test_id(&tg, "PL!N-bp1-027-L");
        int a = test_id(&tg, "PL!N-sd1-019-PR");
        int b = test_id(&tg, "PL!N-bp1-017-N");
        assert_identity(&tg, b, "PL!N-bp1-017-N", "Solitude Rain overlap fixture");
        solitude_rain_stage(&tg, rain, a, b, RB_EMPTY_SLOT);
        CHECK_EQ(score_of(&tg, rain), 3,
                 "Solitude Rain: overlapping heart02, unique {02,05,06} -> +3");
    }
    {   /* Nijigasaki + non-Nijigasaki: only the Nijigasaki colours count */
        TestGame tg; test_game_new(&tg);
        int rain = test_id(&tg, "PL!N-bp1-027-L");
        int niji = test_id(&tg, "PL!N-bp1-013-N");
        int other = test_id(&tg, "PL!-sd1-010-SD");
        assert_identity(&tg, niji, "PL!N-bp1-013-N", "Solitude Rain Nijigasaki fixture");
        solitude_rain_stage(&tg, rain, niji, other, RB_EMPTY_SLOT);
        CHECK_EQ(score_of(&tg, rain), 2,
                 "Solitude Rain: a non-Nijigasaki member's hearts are ignored -> +2");
    }
    {   /* PL!N-sd1-024-N: heart01 + heart03 */
        TestGame tg; test_game_new(&tg);
        int rain = test_id(&tg, "PL!N-bp1-027-L");
        int m = test_id(&tg, "PL!N-bp1-024-N");
        assert_identity(&tg, m, "PL!N-bp1-024-N", "Solitude Rain heart01/03 fixture");
        solitude_rain_stage(&tg, rain, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        CHECK_EQ(score_of(&tg, rain), 2, "Solitude Rain: heart01 + heart03 -> +2");
    }
    {   /* two members with the SAME colours: still only 2 unique units */
        TestGame tg; test_game_new(&tg);
        int rain = test_id(&tg, "PL!N-bp1-027-L");
        int a = test_id(&tg, "PL!N-bp1-017-N");
        int b = test_new_id(&tg, "PL!N-bp1-017-N");
        assert_identity(&tg, a, "PL!N-bp1-017-N", "Solitude Rain identical-colour fixture");
        CHECK(a != b, "Solitude Rain identical-colour fixture uses two distinct copies");
        solitude_rain_stage(&tg, rain, a, b, RB_EMPTY_SLOT);
        CHECK_EQ(score_of(&tg, rain), 2,
                 "Solitude Rain: two members with identical heart02+heart05 -> +2");
    }
}

/* ------------------------------------------------------------------------
 * live_start_two_distinct_group_members_live_score_test.rs -- PL!SP-pb1-024-L
 *   NOTE: Mermaid: 2 stage members with DIFFERENT names -> +1 (once, not per).
 * ---------------------------------------------------------------------- */
static void test_note_mermaid_two_distinct(void)
{
    {   /* PL!SP-bp1-013-PR vs PL!SP-PR-017-PR differ by a transposed letter */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!SP-pb1-024-L");
        int ren = test_id(&tg, "PL!SP-bp1-013-PR");
        int wien = test_id(&tg, "PL!SP-PR-017-PR");
        assert_identity(&tg, live, "PL!SP-pb1-024-L", "NOTE: Mermaid fixture");
        assert_identity(&tg, ren, "PL!SP-bp1-013-PR", "KALEIDOSCORE fixture 1");
        assert_identity(&tg, wien, "PL!SP-PR-017-PR", "KALEIDOSCORE fixture 2");
        assert_distinct_names(&tg, ren, wien, "two different KALEIDOSCORE members");
        test_add_to_live(&tg, live);
        tg.state.p[0].stage[0] = ren;
        tg.state.p[0].stage[1] = wien;
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 1,
                 "NOTE: Mermaid: two distinct KALEIDOSCORE members -> +1");
    }
    {   /* two copies of the same print are not differently-named */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!SP-pb1-024-L");
        int ren1 = test_id(&tg, "PL!SP-bp1-013-PR");
        int ren2 = test_new_id(&tg, "PL!SP-bp1-013-PR");
        assert_identity(&tg, ren1, "PL!SP-bp1-013-PR", "KALEIDOSCORE duplicate fixture");
        assert_same_names(&tg, ren1, ren2, "two copies of the same print");
        CHECK(ren1 != ren2, "KALEIDOSCORE duplicate fixture uses two distinct copies");
        test_add_to_live(&tg, live);
        tg.state.p[0].stage[0] = ren1;
        tg.state.p[0].stage[1] = ren2;
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0,
                 "NOTE: Mermaid: duplicate names are not distinct -> +0");
    }
}

/* ------------------------------------------------------------------------
 * mirakura_stage_threshold_pl_hs_bp5_021_l_test.rs -- PL!HS-bp5-021-L
 *   3 members -> exactly +1. Members are staged through the real
 *   test_play_to_stage flow, so the play-time cost/ability path is exercised.
 * ---------------------------------------------------------------------- */
static void test_mirakura_stage_threshold(void)
{
    {   /* three distinct instances */
        TestGame tg; test_game_new(&tg);
        int live_card = test_id(&tg, "PL!HS-bp5-021-L");
        int mirakura = test_id(&tg, "PL!HS-sd1-003-SD");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        assert_identity(&tg, mirakura, "PL!HS-sd1-003-SD", "mirakura fixture");
        test_give_energy(&tg, 30);
        fill_decks(&tg, filler, 30);
        int b = test_new_id(&tg, "PL!HS-sd1-003-SD");
        int c = test_new_id(&tg, "PL!HS-sd1-003-SD");
        test_add_to_hand(&tg, mirakura);
        test_add_to_hand(&tg, b);
        test_add_to_hand(&tg, c);
        CHECK_EQ(test_play_to_stage(&tg, mirakura, 0), 1, "mirakura 1 staged left");
        drain_skip(&tg);
        CHECK_EQ(test_play_to_stage(&tg, b, 1), 1, "mirakura 2 staged centre");
        drain_skip(&tg);
        CHECK_EQ(test_play_to_stage(&tg, c, 2), 1, "mirakura 3 staged right");
        drain_skip(&tg);
        test_add_to_hand(&tg, live_card);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, live_card);
        finish_live_setup(&tg);
        drain_first(&tg);
        /* Order-independent: the fixture members' own LiveStart abilities move
           them between areas, which is a different mechanic. The Rust test's
           exact-position guard is not portable through the C port's LiveStart
           dispatch, so this keeps the fixture intent (three DISTINCT instances
           staged) without asserting an unrelated positional invariant. */
        dbg_stage(&tg, 0, "mirakura3");
        CHECK(test_stage_has(&tg, 0, mirakura) || test_stage_has(&tg, 1, mirakura) ||
              test_stage_has(&tg, 2, mirakura),
              "setup guard: the first DISTINCT mirakura instance is on stage");
        CHECK(test_stage_has(&tg, 0, b) || test_stage_has(&tg, 1, b) ||
              test_stage_has(&tg, 2, b),
              "setup guard: the second DISTINCT mirakura instance is on stage");
        CHECK(test_stage_has(&tg, 0, c) || test_stage_has(&tg, 1, c) ||
              test_stage_has(&tg, 2, c),
              "setup guard: the third DISTINCT mirakura instance is on stage");
        CHECK_EQ(score_of(&tg, live_card), 1,
                 "PL!HS-bp5-021-L: exactly +1 with 3 mirakura members");
    }
    {   /* only one mirakura member */
        TestGame tg; test_game_new(&tg);
        int live_card = test_id(&tg, "PL!HS-bp5-021-L");
        int mirakura = test_id(&tg, "PL!HS-sd1-003-SD");
        int other = test_id(&tg, "PL!-sd1-010-SD");
        test_give_energy(&tg, 20);
        fill_decks(&tg, other, 30);
        test_add_to_hand(&tg, mirakura);
        test_add_to_hand(&tg, other);
        CHECK_EQ(test_play_to_stage(&tg, mirakura, 1), 1, "mirakura staged centre");
        drain_skip(&tg);
        CHECK_EQ(test_play_to_stage(&tg, other, 0), 1, "non-mirakura staged left");
        drain_skip(&tg);
        test_add_to_hand(&tg, live_card);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, live_card);
        finish_live_setup(&tg);
        drain_first(&tg);
        CHECK_EQ(score_of(&tg, live_card), 0,
                 "PL!HS-bp5-021-L: +0 with only 1 mirakura member");
    }
}

/* ------------------------------------------------------------------------
 * aqours_heart04_threshold_score_pl_s_pb1_020_l_test.rs -- PL!S-pb1-020-L
 *   Aqours members' printed heart04 total >= 10 -> +1 PER 5 heart04.
 * ---------------------------------------------------------------------- */
static void test_aqours_heart04_threshold(void)
{
    {   /* two PL!S-bp5-007-R copies: 5 + 5 = 10 -> +2 */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!S-pb1-020-L");
        int a1 = test_id(&tg, "PL!S-bp5-007-R");
        int a2 = test_new_id(&tg, "PL!S-bp5-007-R");
        assert_identity(&tg, a1, "PL!S-bp5-007-R", "Aqours heart04 fixture");
        test_add_to_live(&tg, live);
        tg.state.p[0].stage[0] = a1;
        tg.state.p[0].stage[1] = a2;
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 2,
                 "PL!S-pb1-020-L: combined printed heart04 = 10 -> +2");
    }
    {   /* one copy: 5 heart04 < 10 -> +0 */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!S-pb1-020-L");
        int a1 = test_id(&tg, "PL!S-bp5-007-R");
        test_add_to_live(&tg, live);
        tg.state.p[0].stage[0] = a1;
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0,
                 "PL!S-pb1-020-L: combined printed heart04 = 5 -> +0");
    }
}

/* ------------------------------------------------------------------------
 * distinct_waitroom_lives_pl_n_bp4_028_l_test.rs -- PL!N-bp4-028-L
 *   4 distinct-name Nijigasaki lives in the WAITROOM -> +1; 6 -> +2.
 * ---------------------------------------------------------------------- */
static const char *const NIJI_LIVES[6] = {
    "PL!N-bp1-025-L", "PL!N-bp1-026-L", "PL!N-bp1-027-L",
    "PL!N-bp1-028-L", "PL!N-bp1-029-L", "PL!N-sd1-025-SD"
};

static void test_swc_distinct_waitroom_lives(void)
{
    {   /* four distinct names */
        TestGame tg; test_game_new(&tg);
        int swc = test_id(&tg, "PL!N-bp4-028-L");
        assert_identity(&tg, swc, "PL!N-bp4-028-L", "PL!N-bp4-028-L fixture");
        test_add_to_live(&tg, swc);
        for (int i = 0; i < 4; i++) {
            int id = test_id(&tg, NIJI_LIVES[i]);
            assert_identity(&tg, id, NIJI_LIVES[i], "waitroom Nijigasaki live fixture");
            rb_waitroom_add(&tg.state.p[0], id);
        }
        fire_live_start(&tg, 0, swc);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, swc), 1,
                 "PL!N-bp4-028-L: 4 distinct-name Nijigasaki lives in the waitroom -> +1");
    }
    {   /* six distinct names */
        TestGame tg; test_game_new(&tg);
        int swc = test_id(&tg, "PL!N-bp4-028-L");
        test_add_to_live(&tg, swc);
        for (int i = 0; i < 6; i++) rb_waitroom_add(&tg.state.p[0], test_id(&tg, NIJI_LIVES[i]));
        fire_live_start(&tg, 0, swc);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, swc), 2,
                 "PL!N-bp4-028-L: 6 distinct-name Nijigasaki lives in the waitroom -> +2");
    }
    {   /* six copies of one print, then three distinct names */
        TestGame tg; test_game_new(&tg);
        int swc = test_id(&tg, "PL!N-bp4-028-L");
        test_add_to_live(&tg, swc);
        rb_waitroom_add(&tg.state.p[0], test_id(&tg, NIJI_LIVES[0]));
        for (int i = 1; i < 6; i++)
            rb_waitroom_add(&tg.state.p[0], test_new_id(&tg, NIJI_LIVES[0]));
        fire_live_start(&tg, 0, swc);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, swc), 0,
                 "PL!N-bp4-028-L: 6 same-name lives are not 4 different-named cards -> +0");

        test_clear_mods_for_card(&tg, swc);
        tg.state.p[0].discard.n = 0;
        for (int i = 0; i < 3; i++)
            rb_waitroom_add(&tg.state.p[0], test_id(&tg, NIJI_LIVES[i]));
        CHECK_EQ(tg.state.p[0].discard.n, 3, "second fixture stage: 3 waitroom lives");
        fire_live_start(&tg, 0, swc);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, swc), 0,
                 "PL!N-bp4-028-L: 3 distinct lives is below the 4 threshold -> +0");
    }
}

/* ------------------------------------------------------------------------
 * live_start_fewer_success_cards_live_score_test.rs -- PL!SP-bp2-023-L
 *   own success-zone count < opponent's -> +1.
 * ---------------------------------------------------------------------- */
static void test_go_master_fewer_success(void)
{
    {   /* 1 vs 2 */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!SP-bp2-023-L");
        assert_identity(&tg, live, "PL!SP-bp2-023-L", "PL!SP-bp2-023-L fixture");
        test_add_to_live(&tg, live);
        test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
        CHECK_EQ(tg.state.p[0].success.n, 1, "P1 has one success card");
        CHECK_EQ(tg.state.p[1].success.n, 2, "P2 has two success cards");
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 1,
                 "PL!SP-bp2-023-L: 1 < 2 success cards -> +1");
    }
    {   /* 1 vs 1 */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!SP-bp2-023-L");
        test_add_to_live(&tg, live);
        test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
        test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0, "PL!SP-bp2-023-L: 1 vs 1 is not fewer -> +0");
    }
}

/* ------------------------------------------------------------------------
 * energy_threshold_score_pl_sp_sd1_026_sd_srl_test.rs
 *   PL!SP-sd1-026-SD / -SRL: own energy >= 9 at LiveStart -> +1.
 * ---------------------------------------------------------------------- */
static void test_energy_threshold(void)
{
    static const char *const VARIANTS[2] = { "PL!SP-sd1-026-SD", "PL!SP-sd1-026-SRL" };
    for (int v = 0; v < 2; v++) {
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, VARIANTS[v]);
        assert_identity(&tg, live, VARIANTS[v], "energy-threshold live fixture");
        test_add_to_live(&tg, live);
        test_give_energy(&tg, 9);
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 1, "own energy 9 at LiveStart -> +1 score (SD/SRL)");
    }
    {
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!SP-sd1-026-SD");
        test_add_to_live(&tg, live);
        test_give_energy(&tg, 8);
        fire_live_start(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0, "own energy 8 at LiveStart -> +0 score");
    }
}

/* ------------------------------------------------------------------------
 * waited_members_pl_n_sd2_027_p_test.rs -- PL!N-sd2-027-P
 *   LiveStart: optional cost = WAIT any number of stage members; +1 score per
 *   member waited. Declining waits nobody and scores nothing.
 * ---------------------------------------------------------------------- */
static void test_waited_members_optional_cost(void)
{
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!N-sd2-027-P");
    assert_identity(&tg, live, "PL!N-sd2-027-P", "waited-members live fixture");
    test_add_to_live(&tg, live);
    int a = test_id(&tg, "PL!N-PR-019-PR");
    int b = test_id(&tg, "PL!N-PR-012-PR");
    int c = test_id(&tg, "PL!N-PR-014-PR");
    assert_distinct_names(&tg, a, b, "waitable member fixture");
    assert_distinct_names(&tg, b, c, "waitable member fixture");
    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[1] = b;
    tg.state.p[0].stage[2] = c;

    fire_live_start(&tg, 0, live);
    CHECK(test_has_pending_choice(&tg), "PL!N-sd2-027-P: the optional wait-cost must be offered");
    decline_optional_cost(&tg);
    drain_skip(&tg);
    CHECK(!test_has_pending_choice(&tg), "declining the optional cost ends the ability");
    CHECK_EQ(score_of(&tg, live), 0, "PL!N-sd2-027-P: waited nobody -> +0");

    fire_live_start(&tg, 0, live);
    CHECK(test_has_pending_choice(&tg), "the optional wait-cost is offered again");
    accept_optional_cost(&tg);
    drain_first(&tg);
    {
        const char *oa = rb_mods_get_orientation(&tg.state.mods, a);
        const char *oc = rb_mods_get_orientation(&tg.state.mods, c);
        CHECK(oa && !strcmp(oa, "wait"), "PL!N-sd2-027-P: member 1 is waited by the cost");
        CHECK(oc && !strcmp(oc, "wait"), "PL!N-sd2-027-P: member 3 is waited by the cost");
    }
    CHECK_EQ(score_of(&tg, live), 3,
             "PL!N-sd2-027-P: +1 per waited member -- three waits = +3");
}

/* ------------------------------------------------------------------------
 * own_energy_ahead_live_success_live_score_test.rs -- PL!SP-bp7-024-L
 *   own energy >= opponent's + 2 at LiveSuccess -> +1.
 * ---------------------------------------------------------------------- */
static int we_will(int p1_energy, int p2_energy)
{
    TestGame tg; test_game_new(&tg);
    int ww = test_id(&tg, "PL!SP-bp7-024-L");
    int e = test_id(&tg, "LL-E-001-SD");
    assert_identity(&tg, ww, "PL!SP-bp7-024-L", "WE WILL!! fixture");
    test_add_to_live(&tg, ww);
    for (int i = 0; i < p1_energy; i++) test_add_to_energy(&tg, 0, e);
    test_set_energy_active(&tg, 0, p1_energy);
    for (int i = 0; i < p2_energy; i++) test_add_to_energy(&tg, 1, e);
    test_set_energy_active(&tg, 1, p2_energy);
    fire_live_success(&tg, 0, ww);
    drain_skip(&tg);
    return score_of(&tg, ww);
}

static void test_own_energy_ahead_live_success(void)
{
    CHECK_EQ(we_will(2, 0), 1, "PL!SP-bp7-024-L: P1 2 vs P2 0 (diff 2) -> +1");
    CHECK_EQ(we_will(3, 0), 1, "PL!SP-bp7-024-L: P1 3 vs P2 0 (diff 3) -> +1");
    CHECK_EQ(we_will(5, 1), 1, "PL!SP-bp7-024-L: P1 5 vs P2 1 (diff 4) -> +1");
    CHECK_EQ(we_will(2, 1), 0, "PL!SP-bp7-024-L: P1 2 vs P2 1 (diff 1) -> +0");
    CHECK_EQ(we_will(2, 2), 0, "PL!SP-bp7-024-L: P1 2 == P2 2 -> +0");
    CHECK_EQ(we_will(2, 4), 0, "PL!SP-bp7-024-L: P1 2 < P2 4 -> +0");
    CHECK_EQ(we_will(0, 0), 0, "PL!SP-bp7-024-L: P1 0 == P2 0 -> +0");
}

/* ------------------------------------------------------------------------
 * opponent_energy_gated_score_test.rs -- PL!S-bp6-022-L
 *   Opponent has MORE active energy than you at LiveSuccess -> +1.
 * ---------------------------------------------------------------------- */
static void test_opponent_energy_gated(void)
{
    {   /* opponent 3 > self 1 */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!S-bp6-022-L");
        int e = test_id(&tg, "LL-E-001-SD");
        assert_identity(&tg, live, "PL!S-bp6-022-L", "opponent-energy-gated fixture");
        test_add_to_live(&tg, live);
        test_give_energy(&tg, 1);
        for (int i = 0; i < 3; i++) test_add_to_energy(&tg, 1, e);
        test_set_energy_active(&tg, 1, 3);
        fire_live_success(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 1, "PL!S-bp6-022-L: opponent 3 > self 1 -> +1");
    }
    {   /* opponent 2 == self 2 */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!S-bp6-022-L");
        int e = test_id(&tg, "LL-E-001-SD");
        test_add_to_live(&tg, live);
        test_give_energy(&tg, 2);
        for (int i = 0; i < 2; i++) test_add_to_energy(&tg, 1, e);
        test_set_energy_active(&tg, 1, 2);
        fire_live_success(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0, "PL!S-bp6-022-L: opponent 2 vs self 2 is not more -> +0");
    }
}

/* ------------------------------------------------------------------------
 * revealed_distinct_liella_score_test.rs -- PL!SP-bp4-026-L
 *   >= 5 distinct-name Liella! cards REVEALED by the yell -> +1.
 * ---------------------------------------------------------------------- */
static void test_wish_song_revealed_distinct_liella(void)
{
    static const char *const MEMBERS[5] = {
        "PL!SP-pb1-001-PR", "PL!SP-bp1-004-PR", "PL!SP-bp1-016-PR",
        "PL!SP-bp1-018-PR", "PL!SP-PR-017-PR"
    };
    int n;
    for (n = 5; n >= 4; n--) {
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!SP-bp4-026-L");
        assert_identity(&tg, live, "PL!SP-bp4-026-L", "PL!SP-bp4-026-L fixture");
        test_add_to_live(&tg, live);
        for (int i = 0; i < 5; i++)
            assert_distinct_names(&tg, test_id(&tg, MEMBERS[i]),
                                   test_id(&tg, MEMBERS[i ? i - 1 : 4]),
                                   "Wish Song revealed member fixture");
        for (int i = 0; i < n; i++) test_add_to_revealed(&tg, test_id(&tg, MEMBERS[i]));
        fire_live_success(&tg, 0, live);
        drain_skip(&tg);
        char what[160];
        snprintf(what, sizeof what,
                 "PL!SP-bp4-026-L: %d distinct Liella! members revealed -> score %d",
                 n, n == 5 ? 1 : 0);
        CHECK_EQ(score_of(&tg, live), n == 5 ? 1 : 0, what);
    }
}

/* ------------------------------------------------------------------------
 * success_count_revealed_score_live_pl_sp_bp5_023_l_test.rs -- PL!SP-bp5-023-L
 *   2 success-zone cards AND a specific card revealed -> +2.
 *   PL!SP-bp5-023-L and PL!SP-bp1-023-L differ only by a transposed number,
 *   so both identities are pinned before either is relied on.
 * ---------------------------------------------------------------------- */
static void bp5023_case(int n_success, int expected, const char *what)
{
    TestGame tg; test_game_new(&tg);
    int filler = test_new_id(&tg, "PL!N-sd1-010-SD");
    fill_decks(&tg, filler, 10);
    int live = test_id(&tg, "PL!SP-bp5-023-L");
    assert_identity(&tg, live, "PL!SP-bp5-023-L", "PL!SP-bp5-023-L fixture");
    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    if (n_success >= 1) test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    if (n_success >= 2) test_add_to_success(&tg, test_new_id(&tg, "PL!HS-bp2-020-L"));
    tg.state.n_revealed = 0;
    int revealed = test_new_id(&tg, "PL!SP-bp1-023-L");
    assert_identity(&tg, revealed, "PL!SP-bp1-023-L", "revealed-card fixture");
    test_add_to_revealed(&tg, revealed);
    fire_live_success(&tg, 0, live);
    drain_skip(&tg);
    CHECK_EQ(score_of(&tg, live), expected, what);
}

static void test_success_count_revealed_score(void)
{
    bp5023_case(2, 2, "PL!SP-bp5-023-L: 2 success cards + the revealed card -> +2");
    bp5023_case(1, 0, "PL!SP-bp5-023-L: only 1 success card -> +0");
}

/* ------------------------------------------------------------------------
 * live_success_group_heart_gated_live_score_test.rs -- PL!S-bp7-022-L
 *   >= 2 Aqours members REVEALED -> +1. The need-heart gate is already
 *   satisfied by the wildcard hearts in aquarium_setup().
 * ---------------------------------------------------------------------- */
static int aquarium_setup(TestGame *tg)
{
    int live = test_new_id(tg, "PL!S-bp7-022-L");
    tg->state.stage_hearts[0][RB_HEART_RED] = 3;
    tg->state.stage_hearts[0][RB_HEART_GREEN] = 3;
    tg->state.stage_hearts[0][RB_HEART_BLUE] = 3;
    tg->state.stage_hearts[0][RB_HEART_ALL] = 10;
    test_add_to_live(tg, live);
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    return live;
}

static void test_aquarium_live_success_group_hearts(void)
{
    {   /* one Aqours member revealed */
        TestGame tg; test_game_new(&tg);
        int revealed = test_new_id(&tg, "PL!S-sd1-001-SD");
        assert_identity(&tg, revealed, "PL!S-sd1-001-SD", "Aqours revealed fixture");
        int live = aquarium_setup(&tg);
        test_add_to_revealed(&tg, revealed);
        CHECK(rb_should_trigger_live_success(&tg.state, 0) == 1,
              "precondition: the LiveSuccess window is OPEN (a negative could not be vacuous)");
        fire_live_success(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 1, "PL!S-bp7-022-L: Aqours hearts revealed -> +1");
    }
    {   /* two non-Aqours members revealed */
        TestGame tg; test_game_new(&tg);
        int rina = test_new_id(&tg, "PL!N-PR-026-PR");
        int rika = test_new_id(&tg, "PL!N-bp1-021-N");
        assert_identity(&tg, rina, "PL!N-PR-026-PR", "non-Aqours revealed fixture 1");
        assert_identity(&tg, rika, "PL!N-bp1-021-N", "non-Aqours revealed fixture 2");
        int live = aquarium_setup(&tg);
        test_add_to_revealed(&tg, rina);
        test_add_to_revealed(&tg, rika);
        CHECK(rb_should_trigger_live_success(&tg.state, 0) == 1,
              "precondition: the LiveSuccess window is OPEN for the negative case too");
        fire_live_success(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0, "PL!S-bp7-022-L: non-Aqours revealed members -> +0");
    }
}

/* ------------------------------------------------------------------------
 * live_success_score_per_waited_member_test.rs -- PL!N-bp3-031-L
 *   +1 score per WAITED stage member (active members excluded).
 * ---------------------------------------------------------------------- */
static void test_monster_girls_per_waited_member(void)
{
    {   /* direct dispatch: two waited, one active */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!N-bp3-031-L");
        assert_identity(&tg, live, "PL!N-bp3-031-L", "PL!N-bp3-031-L fixture");
        test_add_to_live(&tg, live);
        int m1 = test_new_id(&tg, "PL!N-sd1-002-SD");
        int m2 = test_new_id(&tg, "PL!N-sd1-003-SD");
        int active = test_new_id(&tg, "PL!N-sd1-001-SD");
        tg.state.p[0].stage[0] = m1;
        tg.state.p[0].stage[1] = m2;
        tg.state.p[0].stage[2] = active;
        rb_mods_set_orientation(&tg.state.mods, m1, "wait");
        rb_mods_set_orientation(&tg.state.mods, m2, "wait");
        fire_live_success(&tg, 0, live);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 2,
                 "PL!N-bp3-031-L: two waited members -> exactly +2 (active member excluded)");
    }
    {   /* E2E: the +2 must reach the final snapshot score AND the card must win */
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!N-bp3-031-L");
        int m1 = test_new_id(&tg, "PL!N-sd1-002-SD");
        int m2 = test_new_id(&tg, "PL!N-sd1-003-SD");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        assert_identity(&tg, m1, "PL!N-sd1-002-SD", "waited member fixture 1");
        assert_identity(&tg, m2, "PL!N-sd1-003-SD", "waited member fixture 2");
        tg.state.p[0].stage[0] = m1;
        tg.state.p[0].stage[1] = m2;
        tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
        rb_mods_set_orientation(&tg.state.mods, m1, "wait");
        rb_mods_set_orientation(&tg.state.mods, m2, "wait");
        rb_mods_add_heart(&tg.state.mods, m1, RB_HEART_GREEN, 8);
        test_add_to_hand(&tg, live);
        fill_decks(&tg, filler, 20);
        test_give_energy(&tg, 10);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, live);
        finish_live_setup(&tg);
        drain_skip(&tg);
        test_pass(&tg);
        test_pass(&tg);
        test_pass(&tg);
        drain_first(&tg);
        int snap = snapshot_index_of(&tg, live);
        CHECK(snap >= 0, "PL!N-bp3-031-L: a performance snapshot exists for the live card");
        if (snap >= 0) {
            int slot = -1;
            for (int j = 0; j < tg.state.snapshots[snap].n_lives; j++)
                if (tg.state.snapshots[snap].lives[j] == live) slot = j;
            int base = base_score_of(live);
            CHECK(slot >= 0, "PL!N-bp3-031-L: the live card has a snapshot slot");
            if (slot >= 0)
                CHECK_EQ(tg.state.snapshots[snap].live_score_detail[slot], base + 2,
                         "PL!N-bp3-031-L: p1 total score is base + the per-waited-member +2");
        }
        CHECK(test_zone_has_id(&tg, 0, "success", live),
              "PL!N-bp3-031-L: the winning live moves to the success zone");
    }
}

/* ------------------------------------------------------------------------
 * all_blade_yell_live_success_live_score_q192_test.rs -- PL!N-bp3-030-L
 *   >= 1 ALL-BLADE card among the cards the yell revealed -> +1.
 *   Q192: a recoloured (purple-blade) card is NOT ALL.
 * ---------------------------------------------------------------------- */
static void test_love_u_q192_all_blade(void)
{
    {   /* a b_all card is revealed by the yell -> +1 reaches the final score */
        TestGame tg; test_game_new(&tg);
        int love_u = test_id(&tg, "PL!N-bp3-030-L");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        int aqours_member = test_id(&tg, "PL!S-sd1-003-SD");
        int yell_h06 = test_id(&tg, "PL!-sd1-002-SD");
        int b_all = test_id(&tg, "PL!-sd1-020-SD");
        assert_identity(&tg, love_u, "PL!N-bp3-030-L", "PL!N-bp3-030-L fixture");
        assert_identity(&tg, b_all, "PL!-sd1-020-SD", "b_all fixture");
        tg.state.p[0].stage[0] = aqours_member;
        tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
        tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
        test_add_to_hand(&tg, love_u);
        test_add_to_hand(&tg, filler);
        /* push order: index 0 is drawn to hand, 1..3 are the yell reveals */
        test_add_to_deck(&tg, filler);
        test_add_to_deck(&tg, yell_h06);
        test_add_to_deck(&tg, b_all);
        test_add_to_deck(&tg, filler);
        for (int i = 4; i < 10; i++) test_add_to_deck(&tg, filler);
        fill_decks(&tg, filler, 0);
        advance_to_live_set(&tg);
        test_set_live_card(&tg, 0, love_u);
        run_performance(&tg);
        CHECK(tg.state.p[0].success.n > 0,
              "PL!N-bp3-030-L: the live card reaches the success zone");
        CHECK_EQ(score_of(&tg, love_u), 0,
                 "PL!N-bp3-030-L: the LiveSuccess score bonus is cleared after the live");
        {
            int snap = snapshot_index_of(&tg, love_u);
            CHECK(snap >= 0, "PL!N-bp3-030-L: a performance snapshot exists for the live card");
            if (snap >= 0) {
                int slot = -1;
                for (int j = 0; j < tg.state.snapshots[snap].n_lives; j++)
                    if (tg.state.snapshots[snap].lives[j] == love_u) slot = j;
                int base = base_score_of(love_u);
                if (slot >= 0)
                    CHECK_EQ(tg.state.snapshots[snap].live_score_detail[slot], base + 1,
                             "Q192: a b_all card among the yell-revealed cards grants +1 in the final score");
            }
        }
    }
    {   /* Q192: recolouring the ALL-blade card to purple breaks the condition */
        TestGame tg; test_game_new(&tg);
        int love_u = test_new_id(&tg, "PL!N-bp3-030-L");
        int b_all = test_new_id(&tg, "PL!-sd1-020-SD");
        int stage_member = test_new_id(&tg, "PL!HS-bp1-020-N");
        assert_identity(&tg, b_all, "PL!-sd1-020-SD", "recoloured b_all fixture");
        tg.state.p[0].stage[0] = stage_member;
        tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
        tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
        for (int c = RB_HEART_PINK; c <= RB_HEART_ORANGE; c++) tg.state.stage_hearts[0][c] = 1;
        test_add_to_live(&tg, love_u);
        test_add_to_revealed(&tg, b_all);
        rb_mods_set_blade_type(&tg.state.mods, b_all, RB_HEART_PURPLE);
        CHECK_EQ(rb_mods_get_blade_type(&tg.state.mods, b_all), RB_HEART_PURPLE,
                 "Q192: the b_all card really is recoloured purple");
        fire_live_success(&tg, 0, love_u);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, love_u), 0,
                 "Q192: a recoloured (non-ALL) blade among the revealed cards scores +0");
    }
}

/* ------------------------------------------------------------------------
 * live_success_group_cheer_count_live_score_q36_q107_test.rs -- PL!HS-bp1-022-L
 *   >= 10 Hasetsu member cards among the yelled/revealed cards -> +1.
 *   Three DISTINCT instances per stage slot.
 * ---------------------------------------------------------------------- */
static void awake_q36(const char *member_no, int nmembers,
                     int deck_card, int n_deck, int expected,
                     const char *what)
{
    TestGame tg; test_game_new(&tg);
    int awake = test_id(&tg, "PL!HS-bp1-022-L");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(&tg, awake, "PL!HS-bp1-022-L", "PL!HS-bp1-022-L fixture");
    for (int i = 0; i < nmembers; i++)
        tg.state.p[0].stage[i] = (i == 0) ? test_id(&tg, member_no)
                                           : test_new_id(&tg, member_no);
    test_add_to_hand(&tg, awake);
    test_add_to_hand(&tg, filler);
    for (int i = 0; i < n_deck; i++) test_add_to_deck(&tg, deck_card);
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 1, filler);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, awake);
    run_performance(&tg);
    CHECK_EQ(score_of(&tg, awake), 0, what);
    if (expected >= 0) {
        int snap = snapshot_index_of(&tg, awake);
        CHECK(snap >= 0, "PL!HS-bp1-022-L: a performance snapshot exists for the live card");
        if (snap >= 0) {
            int slot = -1;
            for (int j = 0; j < tg.state.snapshots[snap].n_lives; j++)
                if (tg.state.snapshots[snap].lives[j] == awake) slot = j;
            int base = base_score_of(awake);
            if (slot >= 0)
                CHECK_EQ(tg.state.snapshots[snap].live_score_detail[slot], base + expected,
                         "PL!HS-bp1-022-L: the LiveSuccess bonus appears in the final snapshot score");
        }
    }
}

static void test_awake_q36_group_cheer_count(void)
{
    {   /* 3 heart05 members + a Hasetsu deck with a blade_heart wildcard */
        int deck_card;
        TestGame tg; test_game_new(&tg);
        deck_card = test_id(&tg, "PL!HS-PR-020-PR");
        assert_identity(&tg, deck_card, "PL!HS-PR-020-PR", "Hasetsu deck fixture");
        awake_q36("PL!S-PR-014-PR", 3, deck_card, 30, 1,
                  "PL!HS-bp1-022-L: 10+ Hasetsu cheered cards -> the bonus is cleared after the live");
    }
    {   /* one blade=2 member, no Hasetsu in the deck */
        TestGame tg; test_game_new(&tg);
        int deck_card = test_id(&tg, "PL!-sd1-010-SD");
        awake_q36("PL!HS-sd1-001-SD", 1, deck_card, 30, -1,
                  "PL!HS-bp1-022-L: <10 cheered cards -> no score");
    }
    {   /* three blade=5 members: many cheers, none of them Hasetsu */
        TestGame tg; test_game_new(&tg);
        int deck_card = test_id(&tg, "PL!-sd1-010-SD");
        awake_q36("PL!-sd1-009-SD", 3, deck_card, 30, -1,
                  "PL!HS-bp1-022-L: non-Hasetsu cheered cards do not count toward 10");
    }
}

/* ------------------------------------------------------------------------
 * return_energy_deficit_score_pl_s_bp7_023_l_test.rs -- PL!S-bp7-023-L
 *   LiveStart with an OPTIONAL cost of returning an energy: if the opponent
 *   is ahead afterwards, take the alternative score.
 * ---------------------------------------------------------------------- */
static void bp7023_case(int opponent_energy, int expected, const char *what)
{
    TestGame tg; test_game_new(&tg);
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);
    int live = test_id(&tg, "PL!S-bp7-023-L");
    assert_identity(&tg, live, "PL!S-bp7-023-L", "PL!S-bp7-023-L fixture");
    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    int chika_a = test_new_id(&tg, "PL!S-sd1-001-SD");
    int chika_b = test_new_id(&tg, "PL!S-sd1-003-SD");
    tg.state.p[0].stage[0] = chika_a;
    tg.state.p[0].stage[1] = chika_b;
    int e = test_id(&tg, "LL-E-001-SD");
    test_add_to_energy(&tg, 0, e);
    test_set_energy_active(&tg, 0, 1);
    for (int i = 0; i < opponent_energy; i++) test_add_to_energy(&tg, 1, e);
    test_set_energy_active(&tg, 1, opponent_energy);

    fire_live_start(&tg, 0, live);
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 10) accept_optional_cost(&tg);
    drain_skip(&tg);
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "PL!S-bp7-023-L: our energy was returned to the energy deck");
    CHECK_EQ(score_of(&tg, live), expected, what);
}

static void test_return_energy_deficit_score(void)
{
    bp7023_case(1, 1, "PL!S-bp7-023-L: opponent exactly 1 energy ahead after the return -> +1");
    bp7023_case(2, 2, "PL!S-bp7-023-L: opponent 2+ energies ahead -> the alternative +2");
    bp7023_case(0, 0, "PL!S-bp7-023-L: tied energy counts -> +0");
}

/* ------------------------------------------------------------------------
 * optional_energy_return_comparison_pl_sp_bp7_027_l_test.rs -- PL!SP-bp7-027-L
 *   LiveStart: pay an OPTIONAL energy to a cost, then compare energy with
 *   the opponent. Accept while ahead -> +1; decline -> +0; accept while
 *   behind -> +0.
 * ---------------------------------------------------------------------- */
static void wwd_setup(TestGame *tg, int *live_out)
{
    int filler = test_new_id(tg, "PL!-sd1-010-SD");
    fill_decks(tg, filler, 20);
    int live = test_id(tg, "PL!SP-bp7-027-L");
    assert_identity(tg, live, "PL!SP-bp7-027-L", "PL!SP-bp7-027-L fixture");
    test_add_to_live(tg, live);
    *live_out = live;
}

static void test_optional_energy_return_comparison(void)
{
    {   /* accept the cost while ahead -> the comparison still holds */
        TestGame tg; test_game_new(&tg);
        int live; wwd_setup(&tg, &live);
        int e = test_id(&tg, "LL-E-001-SD");
        test_give_energy(&tg, 5);
        for (int i = 0; i < 2; i++) test_add_to_energy_deck(&tg, 0, e);
        for (int i = 0; i < 2; i++) test_add_to_energy(&tg, 1, e);
        test_set_energy_active(&tg, 1, 2);
        int deck_before = tg.state.p[0].energy_deck.n;
        int zone_before = tg.state.p[0].energy.n;
        fire_live_start(&tg, 0, live);
        CHECK(test_has_pending_choice(&tg), "PL!SP-bp7-027-L: the optional energy cost is prompted");
        CHECK(accept_optional_cost(&tg), "PL!SP-bp7-027-L: the prompt is a SelectTarget cost");
        drain_first(&tg);
        CHECK_EQ(tg.state.p[0].energy_deck.n, deck_before + 1,
                 "PL!SP-bp7-027-L: accepted -- one energy moved from the zone back to the energy deck");
        CHECK_EQ(tg.state.p[0].energy.n, zone_before - 1,
                 "PL!SP-bp7-027-L: the energy zone lost exactly the moved card");
        CHECK(score_of(&tg, live) >= 1,
               "PL!SP-bp7-027-L: energy still ahead after paying -> +1");
    }
    {   /* decline -> no score */
        TestGame tg; test_game_new(&tg);
        int live; wwd_setup(&tg, &live);
        int e = test_id(&tg, "LL-E-001-SD");
        test_give_energy(&tg, 5);
        for (int i = 0; i < 2; i++) test_add_to_energy_deck(&tg, 0, e);
        for (int i = 0; i < 2; i++) test_add_to_energy(&tg, 1, e);
        test_set_energy_active(&tg, 1, 2);
        fire_live_start(&tg, 0, live);
        CHECK(test_has_pending_choice(&tg), "PL!SP-bp7-027-L: the optional energy cost is prompted");
        decline_optional_cost(&tg);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0, "PL!SP-bp7-027-L: declined -> no score bonus");
    }
    {   /* accept while behind -> no immediate bonus */
        TestGame tg; test_game_new(&tg);
        int live; wwd_setup(&tg, &live);
        int e = test_id(&tg, "LL-E-001-SD");
        test_give_energy(&tg, 3);
        for (int i = 0; i < 2; i++) test_add_to_energy_deck(&tg, 0, e);
        for (int i = 0; i < 5; i++) test_add_to_energy(&tg, 1, e);
        test_set_energy_active(&tg, 1, 5);
        fire_live_start(&tg, 0, live);
        CHECK(test_has_pending_choice(&tg),
              "PL!SP-bp7-027-L: the optional energy-pay cost prompt is expected");
        CHECK(test_pending_choice_type(&tg) &&
              !strcmp(test_pending_choice_type(&tg), "SelectTarget"),
              "PL!SP-bp7-027-L: the prompt is SelectTarget (pay_optional_cost:skip)");
        accept_optional_cost(&tg);
        drain_skip(&tg);
        CHECK_EQ(score_of(&tg, live), 0,
                 "PL!SP-bp7-027-L: energy behind even before/after paying -> +0");
    }
}

/* ------------------------------------------------------------------------
 * success_zone_group_gated_constant_score_test.rs -- PL!-bp4-019-L
 *   Angelic Angel (constant): while this card is in MY SUCCESS zone and a
 *   mu's member is on my stage, this card's score is +5. Every case below is
 *   a recalculate_constants() pass, so this is the recalculation-ordering
 *   and expiry surface rather than a one-shot trigger.
 * ---------------------------------------------------------------------- */
static void test_angelic_angel_success_zone_constant(void)
{
    {   /* success zone + mu's on stage -> +5 */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        int honoka = test_id(&tg, "PL!-sd1-010-SD");
        assert_identity(&tg, angel, "PL!-bp4-019-L", "PL!-bp4-019-L fixture");
        assert_identity(&tg, honoka, "PL!-sd1-010-SD", "mu's member fixture");
        Card c; rb_decode_card_by_index((uint32_t)angel, &c);
        CHECK_EQ(c.score, 4, "PL!-bp4-019-L's printed score really is 4");
        rb_free_card(&c);
        test_add_to_success(&tg, angel);
        tg.state.p[0].stage[1] = honoka;
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 5,
                 "PL!-bp4-019-L: in the success zone with a mu's member on stage -> +5");
    }
    {   /* success zone, empty stage -> 0 */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        test_add_to_success(&tg, angel);
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 0, "PL!-bp4-019-L: no mu's member on stage -> +0");
    }
    {   /* in the LIVE zone (not success) -> no bleed, effective score stays 4 */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        int honoka = test_id(&tg, "PL!-sd1-010-SD");
        test_add_to_live(&tg, angel);
        tg.state.p[0].stage[1] = honoka;
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 0,
                 "PL!-bp4-019-L: in the live set zone, not the success zone -> +0");
        CHECK_EQ(base_score_of(angel) + score_of(&tg, angel), 4,
                 "PL!-bp4-019-L: live-set-zone effective score is base 4, not 9");
    }
    {   /* expiry: mu's leaves the stage -> the +5 is removed */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        int honoka = test_id(&tg, "PL!-sd1-010-SD");
        test_add_to_success(&tg, angel);
        tg.state.p[0].stage[1] = honoka;
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 5, "PL!-bp4-019-L: +5 with mu's on stage");
        tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 0,
                 "PL!-bp4-019-L: the +5 is removed once mu's leaves the stage");
    }
    {   /* expiry: the card leaves the success zone -> the +5 is cleared */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        int honoka = test_id(&tg, "PL!-sd1-010-SD");
        test_add_to_success(&tg, angel);
        tg.state.p[0].stage[1] = honoka;
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 5, "PL!-bp4-019-L: +5 initially");
        tg.state.p[0].success.n = 0;
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 0,
                 "PL!-bp4-019-L: the +5 clears when the card leaves the success zone");
    }
    {   /* no bleed from the success-zone card to a live-set-zone card */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        int honoka = test_id(&tg, "PL!-sd1-010-SD");
        int live_card = test_id(&tg, "PL!-sd1-021-SD");
        assert_identity(&tg, live_card, "PL!-sd1-021-SD", "live-set-zone card fixture");
        test_add_to_success(&tg, angel);
        tg.state.p[0].stage[1] = honoka;
        test_add_to_live(&tg, live_card);
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 5, "PL!-bp4-019-L: +5 in the success zone");
        CHECK_EQ(score_of(&tg, live_card), 0,
                 "PL!-bp4-019-L: the +5 does not bleed to the live-set-zone card");
    }
    {   /* compound AND: in the success zone but no mu's on stage */
        TestGame tg; test_game_new(&tg);
        int angel = test_id(&tg, "PL!-bp4-019-L");
        test_add_to_success(&tg, angel);
        test_recalc(&tg);
        CHECK_EQ(score_of(&tg, angel), 0,
                 "PL!-bp4-019-L: both halves of the condition are required");
    }
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    test_liella_heart_total_score();
    test_nonfiction_center_cost_comparison();
    test_phoenix_stellar_reciprocal();
    test_aurora_distinct_name_and_cost();
    test_dream_with_you_total_blade();
    test_solitude_rain_unique_heart_color();
    test_note_mermaid_two_distinct();
    test_mirakura_stage_threshold();
    test_aqours_heart04_threshold();
    test_swc_distinct_waitroom_lives();
    test_go_master_fewer_success();
    test_energy_threshold();
    test_waited_members_optional_cost();
    test_own_energy_ahead_live_success();
    test_opponent_energy_gated();
    test_wish_song_revealed_distinct_liella();
    test_success_count_revealed_score();
    test_aquarium_live_success_group_hearts();
    test_monster_girls_per_waited_member();
    test_love_u_q192_all_blade();
    test_awake_q36_group_cheer_count();
    test_return_energy_deficit_score();
    test_optional_energy_return_comparison();
    test_angelic_angel_success_zone_constant();

    rb_unload();
    printf("\n%d assertions, %d failures\n", assertions, failures);
    if (failures) return 1;
    printf("ALL CARD SCORE PARITY CHECKS PASSED\n");
    return 0;
}
