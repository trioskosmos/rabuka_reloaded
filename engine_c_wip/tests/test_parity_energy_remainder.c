/* test_parity_energy_remainder.c
 *
 * C parity suite for the two TOP-LEVEL Rust files under
 *   engine/tests/test_modules/effects/energy/
 * that the peer file test_parity_under_member.c does NOT cover.
 *
 * BOUNDARY (established by reading that file and enumerating the Rust tree):
 *   effects/energy/ has exactly FOUR .rs files at its top level and TWO
 *   sub-folders:
 *     mod.rs
 *     energy_zone_active_wait_select_test.rs   <-- SECTION A (this file)
 *     wait_self_activate_pl_n_pb1_006_r_test.rs <-- SECTION B (this file)
 *     place/        (14 .rs)  -> taken by test_parity_under_member.c
 *     under_member/  (4 .rs)  -> taken by test_parity_under_member.c
 *   There is NO effects/energy/move/ and NO effects/energy/cost/ sub-folder
 *   in the Rust tree, so those two named clusters do not exist. The energy-deck
 *   tests that DO live under effects/ (pr_energy_place_source_from_energy_deck,
 *   empty_energy_deck_skips_*, margarete_bp4_010_r_energy_deck_pick_is_the_top
 *   _card_without_a_prompt) all sit in place/ and are therefore already ported
 *   by the peer; the remaining energy_deck tests in the Rust tree live under
 *   rules/, not effects/, and are out of this file's scope.
 *
 * SECTION A -- energy_zone_active_wait_select_test.rs (4 tests)
 *   One theme: the energy zone keeps a POSITIONAL convention -- cards[0..active)
 *   are active and the rest are waited -- and the under-a-member placement menu
 *   is the one prompt where BOTH states are legal, so the player must be able to
 *   tell them apart. Three distinct claims, and they are easy to conflate:
 *     A1 the menu TEXT states the composition ("(active: 3, waited: 17)")
 *     A2 removing WAITED cards does not move the active counter
 *     A3 removing ACTIVE cards does
 *     A4 tapping an already-waited card is a no-op for the counter
 *   The fixture is delicate: 20 energy with active_count 16, then Kasumi's
 *   cost 13 -- paying energy only lowers the ACTIVE COUNT (rb_energy_pay leaves
 *   the cards in the zone), so the menu is asked to offer 3 active + 17 waited.
 *
 * SECTION B -- wait_self_activate_pl_n_pb1_006_r_test.rs (3 tests)
 *   PL!N-pb1-006-R (Omi Kotoka), shidou / ACTIVATE: "make THIS member wait :
 *   make 1 energy card active" -- a COLON-SPLIT ability whose first clause is
 *   the COST (put herself in wait) and whose second is the EFFECT (activate 1
 *   energy).
 *   Both halves are asserted, plus the triple-activation loop that has to
 *   re-orient the member by hand between activations.
 *
 * ---------------------------------------------------------------------------
 * RUST FIXTURE BUGS found while porting (evidence: cards/cards.json read as
 * UTF-8, never Get-Content -Raw)
 * ---------------------------------------------------------------------------
 *   NONE. Every card number named by both Rust files resolves to a print whose
 *   identity, cost and ability text match the fixture's own comments:
 *     PL!N-pb1-002-R  NAKASU Kasumi   cost 13, blade 2.  DEBUT (optional)
 *       "you may place 2 energy cards from your energy zone under this
 *       member" -- the "play cost takes 13" claim in mixed_zone_kasumi is
 *       correct.
 *     PL!N-pb1-006-R  OMI Kotoka      cost  9, blade 3.  ACTIVATE "make this
 *       member wait : make 1 energy card active" -- cost = wait-self /
 *       effect = activate-1-energy, exactly as the test claims.
 *     LL-E-001-SD     energy card     (type: energy)
 *     PL!-sd1-010-SD  Kousaka Honoka  cost 4 -- a legal filler for a cost-9
 *       member.
 *
 * ---------------------------------------------------------------------------
 * LANDMINES respected (confirmed by other agents)
 * ---------------------------------------------------------------------------
 *   - test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE; no heart is
 *     read in this file, and orientation goes through rb_mods_get_orientation.
 *   - RbChoice.zone for the waitroom is "discard" -- irrelevant here; the menu
 *     under test is zone "energy".
 *   - game.id() allocates the NEXT DISTINCT pool slot, so mid() (test_new_id) is
 *     used, never test_id, and rb_card_copy_pool_reset() runs per case because
 *     rb_create_card_copy draws from a process-global monotonic pool.
 *   - sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *   - rb_resume_with_choice only ever carries ONE index, so the TWO-card pick in
 *     A2/A3 goes through rb_resume_with_choice_indices.
 *   - rb_card_no_eq is FUZZY and is NOT an identity pin; ident() compares the
 *     PRINTED number byte-for-byte via rb_card_string().
 *
 * ---------------------------------------------------------------------------
 * OPEN ENGINE DEFECTS these tests pin RED on purpose (snapshot: working tree
 * at HEAD 15502921 "feat(engine,c): analysis binaries and expanded parity
 * suite", built with tools/isolated_build.sh tr_enerrem t
 * T=parity_energy_remainder).  Do NOT weaken them; they are the report.
 * ---------------------------------------------------------------------------
 * G1 (A4) rb_effect_selected_energy_zone_cards -- src/ability/effects/move.c
 *     :1565-1566 -- decrements `energy_active` by the number of SELECTED cards
 *     instead of by the number of selected cards that lie inside the ACTIVE
 *     PREFIX. Rust EnergyZone::set_indices_waited (engine/src/core/zones.rs:731)
 *     and EnergyZone::remove_at (:770) both check the positional convention
 *     (`i >= active` stays waited). A4 drives the resolver directly, so this is
 *     isolated from every prompt: 2 energy / active=1, tap index 1 (waited) ->
 *     counter 0 where Rust keeps 1; the same board's second tap (index 0,
 *     active) then confirms the decrement is real. Paired positive on the same
 *     board, so it is NOT vacuous.
 * G2 (A2/A3) The energy-zone -> under-member pick never lands. The landing
 *     code rb_move_place_energy_under_member_selected (move.c:1507) is correct
 *     and is called from rb_move_handle_energy_zone_selection (move.c:1608),
 *     but the resume never gets there: the engine's own trace prints
 *         [RESUME_ANY] kind=1 zone=energy target=under_member route=3 idx=18
 *         [RESUME_CAPTURE] def=0x0 target=0x0 eff=0x0 cur=0 n=0 pending=1
 *         [CHOICE_EPILOGUE] pending=1 options=0 reprompt=0 queue_actions=0
 *     -- no deferred continuation, no resume effect, an empty queue and
 *     queue_actions=0. Result: zone 20 -> 20 and under_cards 0 -> 0.
 *     Because nothing moved, A2's "counter did not move" reads green
 *     VACUOUSLY in this snapshot; A3's "counter 3 -> 1" is the assertion that
 *     actually fires. Both keep their prompt controls.
 * G3 (A1) The under-member energy menu carries NO description at all
 *     (description / description_en / description_ja are all ""). Rust builds
 *     " (active: {n}, waited: {m})" at engine/src/ability/effects/misc.rs:2412
 *     and splices it into the choice text (:2423); the C emitter at
 *     src/ability/effects/misc.c:1161 passes "under_member" as the route TARGET
 *     token and sets no description. Non-vacuous: kind == SELECT_CARD and
 *     zone == "energy" both pass, so the menu IS on screen and empty.
 *
 * The three tests in SECTION B are GREEN: the colon-split ACTIVATE of
 * PL!N-pb1-006-R correctly pays its "wait self" cost and activates exactly one
 * energy, three times in a row, with the card count unchanged.
 *
 * This file is pure ASCII: no Japanese literal appears in it.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

static int failures;
static int assertions;
static int setup_bugs;
static int n_tests_ok, n_tests_failed, n_tests_crashed;
static const char *current_test = "(none)";

/* -- assertions ----------------------------------------------------------- */

#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  [%s]\n", current_test); \
        failures++; \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)  [%s]\n", message, \
                actual_value, expected_value, current_test); \
        failures++; \
    } \
} while (0)

/* area indices (engine/src/game/zones.rs MemberArea) */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

#define KASUMI_NO "PL!N-pb1-002-R"   /* Kasumi, printed cost 13 */
#define KANATA_NO "PL!N-pb1-006-R"   /* Kotoka, printed cost  9 */
#define ENERGY_NO "LL-E-001-SD"
#define FILLER_NO "PL!-sd1-010-SD"   /* Honoka, printed cost 4 */

/* -- card DB --------------------------------------------------------------- */

/* `src` is the in-tree layout; an isolated out-of-tree build resolves the
 * blobs through ../cards/build, so both paths are tried. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Rust TestGame::id() allocates the NEXT DISTINCT pool slot, so two game.id("X")
 * calls are two different card instances. C's plain template lookup aliases the
 * shared template; test_new_id() is the distinct copy. */
static int mid(TestGame *tg, const char *no)
{
    return test_new_id(tg, no);
}

/* Pin identity by the PRINTED number, byte-for-byte. rb_find_card_by_no has a
 * fuzzy tail and rb_card_no_eq is fuzzy too, so only strcmp of
 * rb_card_string(c.card_no_idx) cannot be satisfied by a near miss. */
static int ident(int id, const char *no)
{
    if (id < 0) {
        fprintf(stderr, "SETUP BUG: card \"%s\" does not resolve in the card DB\n", no);
        setup_bugs++;
        assertions++;
        return 0;
    }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        fprintf(stderr, "SETUP BUG: card id %d does not decode\n", id);
        setup_bugs++;
        assertions++;
        return 0;
    }
    char got[128];
    snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    if (strcmp(got, no) != 0) {
        fprintf(stderr, "SETUP BUG: id %d prints \"%s\", the test wanted \"%s\"\n",
                id, got, no);
        setup_bugs++;
        assertions++;
        return 0;
    }
    return 1;
}

static int printed_cost(int id)
{
    Card c;
    if (id < 0 || !rb_decode_card_by_index((uint32_t)id, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

/* -- board helpers --------------------------------------------------------- */

/* Empty every zone both seats own so each card-count assertion is unambiguous,
 * and reset the global card-copy pool (rb_create_card_copy never resets it). */
static void bare_board(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_deck.n = 0;
        p->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            p->stage[i] = RB_EMPTY_SLOT;
            p->stage_wait[i] = 0;
            p->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_recently_moved = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    tg->state.queue.deferred = NULL;
    tg->state.queue.resume_eff = NULL;
    tg->state.activation_keepalive_valid = 0;
    rb_clear_pending_choice(&tg->state);
}

/* wait_self_activate_pl_n_pb1_006_r_test::fill_decks -- `n` fillers in BOTH
 * main decks (the card is already cleared by bare_board). */
static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static int zone_n(TestGame *tg)        { return tg->state.p[0].energy.n; }
static int active_n(TestGame *tg)      { return rb_energy_active_count(&tg->state.p[0]); }
static int under_n(TestGame *tg, int a){ return tg->state.p[0].under_cards[a].n; }

/* 1 == the orientation modifier reads back as "wait". The string is reported in
 * the assertion so a red says WHICH state the card is actually in, not just
 * "not wait" -- the engine really does apply the wait (its own DEBUG trace
 * prints "after: ori=wait") and something downstream may move it again. */
static const char *ori_str(TestGame *tg, int card)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, card);
    return o ? o : "(none)";
}

static int is_wait(TestGame *tg, int card)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, card);
    return o != NULL && strcmp(o, "wait") == 0;
}

/* -- choice answering ------------------------------------------------------ */

static int er_prompt(TestGame *tg) { return test_has_pending_choice(tg); }

static const RbChoice *er_choice(TestGame *tg)
{
    return rb_get_pending_choice(&tg->state);
}

/* The C queue opens an ability with a SelectAutoAbility ("shall this ability
 * run?") BEFORE the ability's own prompt; Rust's play_to_stage / activate_ability
 * helpers had already drained that step, so a Rust `has_pending_choice()` is
 * the REAL prompt. Proceed the auto-ability step, then hand back whatever is on
 * screen. *reached is set to 1 only when a NON-auto prompt is up, which is what
 * makes every "the menu must have appeared" assertion non-vacuous. */
static const RbChoice *er_reach_real_prompt(TestGame *tg, int *reached)
{
    int guard = 0;
    *reached = 0;
    while (er_prompt(tg) && guard++ < 8) {
        const RbChoice *c = er_choice(tg);
        if (c && c->kind != RB_CHOICE_SELECT_AUTO_ABILITY) {
            *reached = 1;
            return c;
        }
        rb_resume_with_choice(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
    return er_choice(tg);
}

/* `while game.has_pending_choice() { game.select_indices(&[0]) }`, bounded: a
 * peer found an ability that re-prompts UNBOUNDEDLY, so every drain here is
 * capped and reports how many prompts it actually answered. */
static int er_drain_pick0(TestGame *tg, int rounds)
{
    int n = 0, guard = 0;
    while (er_prompt(tg) && guard++ < rounds) {
        n++;
        rb_resume_with_choice(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
    return n;
}

/* ========================================================================
 * SECTION A -- energy_zone_active_wait_select_test.rs
 * ======================================================================== */

/* mixed_zone_kasumi: 20 energy with active_count 16, then Kasumi (cost 13) is
 * played. Paying energy lowers the ACTIVE COUNT only -- rb_energy_pay
 * (src/core/zones.c:279) never removes a card -- so the board the menu is asked
 * to render is 20 cards, 3 active, 17 waited. The mix is set BEFORE the play
 * because the menu text is rendered at prompt time. */
static int er_mixed_zone_kasumi(TestGame *tg, int *kasumi_out)
{
    int kasumi = mid(tg, KASUMI_NO);
    if (!ident(kasumi, KASUMI_NO)) return 0;
    /* Fixture guard: the cost 13 is what the whole arithmetic (16 - 13 = 3)
     * rests on, so it is checked against the print rather than assumed. */
    CHECK_EQ(printed_cost(kasumi), 13, "fixture: kasumi's printed cost is 13");
    test_add_to_hand(tg, kasumi);
    test_give_energy(tg, 20);
    test_set_energy_active(tg, 0, 16);
    *kasumi_out = kasumi;
    return test_play_to_stage(tg, kasumi, AREA_CENTER) == 1;
}

/* A1 :: under_member_menu_shows_active_waited_counts
 * The under-member menu is the one prompt where an ACTIVE and a WAITED energy
 * card are both legal placements, so the description must spell the pile out.
 * Rust: desc.contains("(active: 3, waited: 17)"). */
static void A_menu_shows_active_waited_counts(void)
{
    static TestGame tg;
    int kasumi = 0;
    bare_board(&tg);
    if (!er_mixed_zone_kasumi(&tg, &kasumi)) {
        CHECK(0, "PROMPT CONTROL: kasumi must reach Center (cost 13)");
        return;
    }
    /* rb_energy_pay only lowers the ACTIVE COUNT (src/core/zones.c:279); it
     * never removes a card, so 20 cards are still in the zone and 16-13 = 3 of
     * them are active. Getting this backwards would silently invalidate the
     * "(active: 3, waited: 17)" claim below, so it is asserted, not assumed. */
    CHECK_EQ(zone_n(&tg), 20, "sanity: paying the cost removed NO card from the zone");
    CHECK_EQ(active_n(&tg), 3, "sanity: 16 active minus the 13 cost leaves 3 active");

    int reached = 0;
    const RbChoice *c = er_reach_real_prompt(&tg, &reached);
    /* PROMPT CONTROL: the assertion below is about the MENU, so prove the menu
     * is the thing actually on screen (a SELECT_CARD over the energy zone). */
    CHECK(reached, "PROMPT CONTROL: a non-auto prompt is on screen after the play");
    if (!c) {
        CHECK(0, "PROMPT CONTROL: the under-member energy menu prompted");
        return;
    }
    CHECK_EQ(c->kind, RB_CHOICE_SELECT_CARD,
             "PROMPT CONTROL: the prompt is a card selection");
    CHECK(strcmp(c->zone, "energy") == 0,
          "PROMPT CONTROL: the menu selects from the energy zone, got zone \"%s\"",
          c->zone);
    CHECK(c->description[0] != '\0',
          "PROMPT CONTROL: the menu carries a description at all "
          "(description=\"%s\" description_en=\"%s\" description_ja=\"%s\" target=\"%s\")",
          c->description, c->description_en, c->description_ja, c->target);
    /* Rust formats the suffix in misc.rs:2412-2413 and splices it into the
     * choice description (misc.rs:2423). Kept strict: the C emitter is
     * expected to produce the same text. */
    CHECK(strstr(c->description, "(active: 3, waited: 17)") != NULL,
          "menu states the pile composition; description was \"%s\"", c->description);
}

/* A2 :: under_member_remove_waited_keeps_active_count
 * Indices 18,19 are WAITED (the active prefix is 0..3). Removing them must
 * lower the card count and leave the active counter alone. */
static void A_remove_waited_keeps_active_count(void)
{
    static TestGame tg;
    int kasumi = 0;
    bare_board(&tg);
    if (!er_mixed_zone_kasumi(&tg, &kasumi)) {
        CHECK(0, "PROMPT CONTROL: kasumi must reach Center (cost 13)");
        return;
    }

    int reached = 0;
    const RbChoice *c = er_reach_real_prompt(&tg, &reached);
    CHECK(reached, "PROMPT CONTROL: a non-auto prompt is on screen after the play");
    if (!c || c->kind != RB_CHOICE_SELECT_CARD || strcmp(c->zone, "energy") != 0) {
        CHECK(0, "PROMPT CONTROL: the under-member energy menu is the pending prompt");
        return;
    }

    int idx[2];
    idx[0] = 18; idx[1] = 19;
    rb_resume_with_choice_indices(&tg.state, idx, 2);
    rb_process_pending_auto_abilities(&tg.state);
    int more = er_drain_pick0(&tg, 12);
    /* PROMPT CONTROL: the answer was CONSUMED. Without it, "the counter did
     * not move" below would also read green on a board where the pick was
     * simply never taken. The paired positive is the zone/under-card count. */
    CHECK(!er_prompt(&tg), "PROMPT CONTROL: the menu is closed after the 2-card pick (%d more prompt(s) drained)", more);

    CHECK_EQ(active_n(&tg), 3,
             "removing WAITED cards must not move the active counter");
    CHECK_EQ(zone_n(&tg), 18, "2 energy left the zone");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 2, "2 energy placed under the member");
}

/* A3 :: under_member_remove_active_decrements_counter
 * The same menu, but indices 0,1 are inside the active prefix 0..3, so the
 * counter must drop 3 -> 1 while the card count falls by the same 2. */
static void A_remove_active_decrements_counter(void)
{
    static TestGame tg;
    int kasumi = 0;
    bare_board(&tg);
    if (!er_mixed_zone_kasumi(&tg, &kasumi)) {
        CHECK(0, "PROMPT CONTROL: kasumi must reach Center (cost 13)");
        return;
    }

    int reached = 0;
    const RbChoice *c = er_reach_real_prompt(&tg, &reached);
    CHECK(reached, "PROMPT CONTROL: a non-auto prompt is on screen after the play");
    if (!c || c->kind != RB_CHOICE_SELECT_CARD || strcmp(c->zone, "energy") != 0) {
        CHECK(0, "PROMPT CONTROL: the under-member energy menu is the pending prompt");
        return;
    }

    int idx[2];
    idx[0] = 0; idx[1] = 1;
    rb_resume_with_choice_indices(&tg.state, idx, 2);
    rb_process_pending_auto_abilities(&tg.state);
    int more = er_drain_pick0(&tg, 12);
    /* PROMPT CONTROL: the answer was CONSUMED. Without it, "the counter did
     * not drop" below would also read green on a board where the pick was
     * simply never taken. The paired positive is the zone card count. */
    CHECK(!er_prompt(&tg), "PROMPT CONTROL: the menu is closed after the 2-card pick (%d more prompt(s) drained)", more);

    CHECK_EQ(active_n(&tg), 1, "removing 2 ACTIVE cards drops the counter 3 -> 1");
    CHECK_EQ(zone_n(&tg), 18, "2 energy left the zone");
}

/* A4 :: tap_already_waited_energy_leaves_counter
 * Rust drives the resolver DIRECTLY (no prompt), so this is a pure unit test
 * with a paired positive on the SAME board: tapping the waited card at index 1
 * must not move the counter, then tapping the active card at index 0 must. */
static void A_tap_already_waited_leaves_counter(void)
{
    static TestGame tg;
    bare_board(&tg);
    int e1 = mid(&tg, ENERGY_NO);
    int e2 = mid(&tg, ENERGY_NO);
    if (!ident(e1, ENERGY_NO) || !ident(e2, ENERGY_NO)) return;
    /* Rust game.new_id twice => two DISTINCT instances; the positional
     * convention is about positions, but a same-id bag would hide a bug that
     * re-keys the zone, so keep them distinct. */
    CHECK(e1 != e2, "fixture: the two energy cards are distinct instances");

    test_add_to_energy(&tg, 0, e1);
    test_add_to_energy(&tg, 0, e2);
    test_set_energy_active(&tg, 0, 1);
    CHECK_EQ(zone_n(&tg), 2, "sanity: 2 energy in the zone");
    CHECK_EQ(active_n(&tg), 1, "sanity: active prefix is index 0 only");

    int waited_idx[1] = { 1 };
    rb_effect_selected_energy_zone_cards(&tg.state, 0, waited_idx, 1);
    CHECK_EQ(active_n(&tg), 1,
             "tapping an ALREADY-WAITED card must not decrement the counter");
    CHECK_EQ(zone_n(&tg), 2, "tapping does not remove a card from the zone");
    CHECK(is_wait(&tg, e2), "the tapped waited card is marked wait (orientation \"%s\")",
          ori_str(&tg, e2));

    int active_idx[1] = { 0 };
    rb_effect_selected_energy_zone_cards(&tg.state, 0, active_idx, 1);
    CHECK_EQ(active_n(&tg), 0, "tapping the ACTIVE card decrements the counter");
    CHECK_EQ(zone_n(&tg), 2, "tapping still does not remove a card");
    CHECK(is_wait(&tg, e1), "the active card is marked wait (orientation \"%s\")",
          ori_str(&tg, e1));
}

/* ========================================================================
 * SECTION B -- wait_self_activate_pl_n_pb1_006_r_test.rs
 * ======================================================================== */

/* Shared B fixture: 30 fillers in both decks, `energy` active energy, the card
 * in hand, then a real play to Center and a drain of the debut. Returns the
 * card id, or -1 when the fixture could not be built. */
static int er_kanata_on_stage(TestGame *tg, int energy)
{
    int card = mid(tg, KANATA_NO);
    int filler = mid(tg, FILLER_NO);
    if (!ident(card, KANATA_NO)) return -1;
    if (!ident(filler, FILLER_NO)) return -1;
    CHECK_EQ(printed_cost(card), 9, "fixture: kanata's printed cost is 9");
    CHECK(energy >= 9,
          "fixture: the play needs at least 9 active energy, was given %d", energy);

    test_give_energy(tg, energy);
    fill_decks(tg, filler, 30);
    test_add_to_hand(tg, card);
    if (test_play_to_stage(tg, card, AREA_CENTER) != 1) {
        CHECK(0, "PROMPT CONTROL: kanata must reach Center");
        return -1;
    }
    int prompts = er_drain_pick0(tg, 12);
    /* The drain count is a control, not an assertion: a debut that prompts at
     * all must be fully consumed before the activation, otherwise the
     * activation below would be answered instead of the debut. */
    CHECK(prompts >= 0, "the post-play drain ran (%d prompt(s))", prompts);
    return card;
}

/* B1 :: pl_n_pb1_006_r_wait_self_activates_one_energy
 * COST = "make this member wait" (self -> wait); EFFECT = activate 1 energy.
 * Exactly ONE energy may become active per activation. */
static void B_wait_self_activates_one_energy(void)
{
    static TestGame tg;
    bare_board(&tg);
    int card = er_kanata_on_stage(&tg, 10);
    if (card < 0) return;
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], card, "kanata occupies Center");

    int energy_before = active_n(&tg);
    int act = test_activate_ability(&tg, card);
    /* PROMPT CONTROL: the activation must actually have been accepted; without
     * it both the orientation and the count assertions below read green on a
     * board where nothing ran. */
    CHECK(act != 0, "PROMPT CONTROL: the activation was accepted and ran");
    er_drain_pick0(&tg, 12);

    CHECK(is_wait(&tg, card),
          "COST: the member must be in wait state after the activation "
          "(orientation reads \"%s\", stage_wait=%d)", ori_str(&tg, card),
          tg.state.p[0].stage_wait[AREA_CENTER]);
    CHECK_EQ(active_n(&tg), energy_before + 1,
             "EFFECT: exactly 1 energy was activated");
}

/* B2 :: pl_n_pb1_006_r_activates_wait_energy_without_changing_card_count
 * Same activation from a FULLY-WAITED zone (active_count 0): activating an
 * energy card flips a state, it does not add or remove a card. */
static void B_activates_wait_energy_without_changing_card_count(void)
{
    static TestGame tg;
    bare_board(&tg);
    int card = er_kanata_on_stage(&tg, 15);
    if (card < 0) return;

    test_set_energy_active(&tg, 0, 0);
    int active_before = active_n(&tg);
    int total_before = zone_n(&tg);
    CHECK_EQ(active_before, 0, "sanity: the zone starts fully waited");
    CHECK_EQ(total_before, 15, "sanity: paying the cost removed no card");

    int act = test_activate_ability(&tg, card);
    CHECK(act != 0, "PROMPT CONTROL: the activation was accepted and ran");
    er_drain_pick0(&tg, 12);

    CHECK(is_wait(&tg, card),
          "COST: the member must be in wait (orientation reads \"%s\")",
          ori_str(&tg, card));
    CHECK_EQ(active_n(&tg), active_before + 1,
             "EFFECT: exactly 1 energy became active from the waited pile");
    CHECK_EQ(zone_n(&tg), total_before, "the energy card count is unchanged");
}

/* B3 :: pl_n_pb1_006_r_reactivated_member_can_activate_energy_three_times
 * The cost puts the member in wait, so each further activation has to re-orient
 * it by hand first (Rust adds the "active" orientation modifier). */
static void B_reactivated_member_can_activate_energy_three_times(void)
{
    static TestGame tg;
    bare_board(&tg);
    int card = er_kanata_on_stage(&tg, 15);
    if (card < 0) return;

    int energy_before = active_n(&tg);
    for (int i = 0; i < 3; i++) {
        int act = test_activate_ability(&tg, card);
        /* PROMPT CONTROL, per iteration: a refused activation would leave the
         * count untouched and the "was N, now N+i+1" check would go red for
         * the wrong reason, so the acceptance is asserted explicitly. */
        CHECK(act != 0, "PROMPT CONTROL: activation %d was accepted and ran", i + 1);
        er_drain_pick0(&tg, 12);
        CHECK(is_wait(&tg, card),
              "COST: after each activation the member is in wait "
              "(orientation reads \"%s\")", ori_str(&tg, card));
        CHECK_EQ(active_n(&tg), energy_before + i + 1,
                 "EFFECT: this activation gave exactly 1 energy");
        if (i < 2)
            rb_mods_set_orientation(&tg.state.mods, card, "active");
    }
    CHECK(is_wait(&tg, card),
          "the card is still wait after the third activation (orientation \"%s\")",
          ori_str(&tg, card));
}

/* -- dump (RB_ER_DUMP="PL!N-pb1-002-R,PL!N-pb1-006-R") ---------------------- */

static void er_dump_card(const char *card_no)
{
    static TestGame tg;
    static int probe_done = 0;
    if (!probe_done) { test_game_new(&tg); probe_done = 1; }
    int id = test_new_id(&tg, card_no);
    if (id < 0) { fprintf(stderr, "[DUMP] %s: NOT FOUND\n", card_no); return; }
    int n = rb_card_num_abilities((uint32_t)id);
    fprintf(stderr, "[DUMP] %s id=%d n_abilities=%d name=%s cost=%d\n", card_no, id,
            n, test_card_name(id), printed_cost(id));
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)id, i, &ab)) continue;
        fprintf(stderr, "  ability[%d] triggers=%s use_limit=%d is_null=%d has_cost=%d has_effect=%d\n",
                i, ab.triggers ? ab.triggers : "-", ab.use_limit, ab.is_null,
                ab.cost != NULL, ab.effect != NULL);
        rb_free_ability(&ab);
    }
}

static void er_run_dump(const char *list)
{
    char buf[1024];
    snprintf(buf, sizeof(buf), "%s", list);
    char *cursor = buf;
    while (cursor && *cursor) {
        char *comma = strchr(cursor, ',');
        if (comma) *comma = '\0';
        if (*cursor) er_dump_card(cursor);
        cursor = comma ? comma + 1 : NULL;
    }
}

/* -- runner ---------------------------------------------------------------- */

static void on_segv(int sig)
{
    fprintf(stderr,
            "\n*** SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(3);
}

#define CHILD_OK        0
#define CHILD_FAILURES  1

/* Each test builds its own board; run() only brackets the pass/fail delta.
 * The test still runs in a forked child so an engine fault is attributed to
 * ONE test and the sections after it still execute. */
static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = failures - f0;
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions]\n", nf ? "FAILED" : "ok", name,
               assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s)\n", name,
               assertions, failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
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

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    {
        const char *d = getenv("RB_ER_DUMP");
        if (d && *d) { er_run_dump(d); rb_unload(); return 0; }
    }

    printf("--- A energy/energy_zone_active_wait_select_test.rs ---\n");
    run("under_member_menu_shows_active_waited_counts",
        A_menu_shows_active_waited_counts);
    run("under_member_remove_waited_keeps_active_count",
        A_remove_waited_keeps_active_count);
    run("under_member_remove_active_decrements_counter",
        A_remove_active_decrements_counter);
    run("tap_already_waited_energy_leaves_counter",
        A_tap_already_waited_leaves_counter);

    printf("--- B energy/wait_self_activate_pl_n_pb1_006_r_test.rs ---\n");
    run("pl_n_pb1_006_r_wait_self_activates_one_energy",
        B_wait_self_activates_one_energy);
    run("pl_n_pb1_006_r_activates_wait_energy_without_changing_card_count",
        B_activates_wait_energy_without_changing_card_count);
    run("pl_n_pb1_006_r_reactivated_member_can_activate_energy_three_times",
        B_reactivated_member_can_activate_energy_three_times);

    printf("\n==== parity_energy_remainder: %d tests ok, %d failed, %d crashed "
           "(%d setup bugs) ====\n",
           n_tests_ok, n_tests_failed, n_tests_crashed, setup_bugs);
    return failures ? 1 : 0;
}
