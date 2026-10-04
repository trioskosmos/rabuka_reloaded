/* test_parity_t6_action_setup.c
 *
 * CLUSTER T6 parity suite --the five reachable divergences this cluster closed
 * in the action / driver layer, one case per fix. Every case asserts what RUST
 * asserts and cites the Rust line it comes from. Nothing here is a new
 * expectation: each CHECK is a transcription of a specific Rust statement.
 *
 *   T6-F1  src/turn/phase.c  rb_check_orphaned_under_cards
 *          Rust turn/actions/mod.rs:1851-1856 (check_orphaned_under_cards):
 *              if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
 *                  player.energy_deck.cards.push(cid);
 *              } else {
 *                  player.waitroom.cards.push(cid);
 *              }
 *          The C pushed an orphaned energy card into the ACTIVE ENERGY ZONE.
 *
 *   T6-F2  src/turn/phase.c  rb_check_invalid_resolution_zone
 *          Rust turn/actions/mod.rs:1885-1894 (check_invalid_resolution_zone):
 *              let owner = owners.get(index).copied().unwrap_or(active);
 *              if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
 *                  owner_player.energy_deck.cards.push(cid);
 *              } else {
 *                  owner_player.waitroom.cards.push(cid);
 *              }
 *          The C pushed EVERY resolved card into the active player's discard.
 *          Only the ENERGY half is asserted here --the owner half is
 *          structurally inexpressible (BLOCKED-ON include/rabuka.h, see
 *          AUDIT_actions_setup.md 4-4).
 *
 *   T6-F3  src/turn/phase.c  rb_advance_phase, LiveCardSetSecondAttacker arm
 *          Rust turn/phases.rs:232-233, the first two statements of the arm:
 *              game_state.player1.live_card_set_limit_reduction = 0;
 *              game_state.player2.live_card_set_limit_reduction = 0;
 *          REACHABILITY: 2 of 985 abilities write the field
 *          (execute_reduce_live_card_set_limit, ability/effects/restriction.rs:
 *          217-234) --PL!HS-bp2-018-N and PL!N-bp4-026-L --and both say
 *          "the NEXT live-card-set phase". The C never cleared the field.
 *
 *   T6-F4  src/turn/phase.c  rb_check_timing
 *          Rust turn/actions/mod.rs:1632-1638:
 *              if game_state.player1.is_first_attacker {
 *                  game_state.player1.refresh();
 *                  game_state.player2.refresh();
 *              } else {
 *                  game_state.player2.refresh();
 *                  game_state.player1.refresh();
 *              }
 *          Player::refresh (core/player.rs:634-648) shuffles the waitroom
 *          through the GLOBAL rng, so the refresh ORDER decides which shuffle
 *          consumes the stream first. The C always refreshed seat 0 first.
 *
 *   T6-F5  src/turn/phase.c  activate_wait_members
 *          Rust turn/phases.rs:125-131 (the `to_activate` filter), reading the
 *          set that core/game_state/modifiers.rs:1081-1092 fills:
 *              if game_state.constant_cannot_activate_members.iter()
 *                  .any(|x| x.parse::<i16>() == Ok(cid)) { return None; }
 *          REACHABILITY: 2 of 985 abilities carry a CONSTANT restriction with
 *          restriction_type "cannot_activate" --PL!N-bp5-006-{AR,P,R}
 *          ("this member does not activate in your Active phase", self
 *          target, and therefore the half that matches) and PL!HS-pb1-008-{P,R}
 *          ab#1 (target "opponent", which pushes a PLAYER id that never parses
 *          as i16 and so cannot match --a no-op in Rust too).
 *
 * --
 * Sizes: every GameState / TestGame local below is `static`, because
 * sizeof(GameState) is 885912 B and a stack local would blow the frame.
 * --
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

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* LL-E-001-SD is the corpus energy print used by test_parity_under_member.c
 * for ENERGY_NO; it is a real energy card, so rb_card_is_energy() accepts it. */
#define ENERGY_NO "LL-E-001-SD"
/* A plain member print, used as the non-energy half of the routing assertions. */
#define MEMBER_NO "PL!-sd1-010-SD"
/* PL!N-bp5-006-P (Omiko P) - the self-target CONSTANT cannot_activate
 * card. Printed name is Japanese; the ASCII card number is unambiguous
 * and is what ident() pins, so no escape sequence is needed here. */
#define OMIKO_P "PL!N-bp5-006-P"

/* The card numbers above must resolve AND print what they claim. */
static int ident(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        fprintf(stderr, "SETUP BUG: card %s does not resolve\n", no);
        setup_bugs++; assertions++;
        return -1;
    }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        fprintf(stderr, "SETUP BUG: id %d does not decode\n", id);
        setup_bugs++; assertions++;
        return -1;
    }
    char got[96];
    snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    if (strcmp(got, no) != 0) {
        fprintf(stderr, "SETUP BUG: id %d prints \"%s\", the test wanted \"%s\"\n",
                id, got, no);
        setup_bugs++; assertions++;
        return -1;
    }
    return id;
}

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Empty every zone both seats own, so a count assertion is unambiguous. */
static void bare_board(TestGame *tg)
{
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
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            p->stage[a] = RB_EMPTY_SLOT;
            p->stage_wait[a] = 0;
            p->under_cards[a].n = 0;
        }
    }
    tg->state.resolution.n = 0;
    tg->state.n_revealed = 0;
}

/* ===========================================================================
 * T6-F1 --orphaned under-cards: ENERGY -> energy_deck, not the energy zone.
 *
 * Rust turn/actions/mod.rs:1842-1860:
 *   for (i, member_id) in player.stage.stage.iter().enumerate() {
 *       if *member_id == EMPTY_SLOT {
 *           for card_id in player.stage.under_cards[i].drain(..) {
 *               if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
 *                   player.energy_deck.cards.push(cid);
 *               } else {
 *                   player.waitroom.cards.push(cid);
 *               }
 *           }
 *       }
 *   }
 * Rule 10.5.3 (a member leaves the stage, the card under it is orphaned) and
 * Rule 10.5.4 (an energy card in the waitroom goes back to the ENERGY DECK)
 * are the same routing rule; the card must be re-drawn to be usable again.
 * ======================================================================== */
static void T6_F1_orphan_energy_goes_to_energy_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int en = ident(&tg, ENERGY_NO);
    int mem = ident(&tg, MEMBER_NO);
    CHECK(en > 0 && mem > 0, "F1: fixtures must resolve");
    CHECK(rb_card_is_energy(en), "F1: %s must be an energy card", ENERGY_NO);
    CHECK(!rb_card_is_energy(mem), "F1: %s must NOT be an energy card", MEMBER_NO);

    /* Energy tucked under a stage member, the member then leaves. */
    bare_board(&tg);
    tg.state.p[0].stage[AREA_CENTER] = mem;
    tg.state.p[0].under_cards[AREA_CENTER].n = 0;
    tg.state.p[0].under_cards[AREA_CENTER].cards[0] = en;
    tg.state.p[0].under_cards[AREA_CENTER].n = 1;
    /* The member leaves the stage: slot is empty, the under-card is orphaned. */
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;

    int energy_before   = tg.state.p[0].energy.n;
    int edeck_before    = tg.state.p[0].energy_deck.n;
    int discard_before  = tg.state.p[0].discard.n;

    rb_check_orphaned_under_cards(&tg.state, 0);

    /* Rust: energy_deck.cards.push(cid) --the ENERGY DECK gains it. */
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before + 1,
             "F1: an orphaned energy card must be recycled to the ENERGY DECK");
    CHECK(tg.state.p[0].energy_deck.n == edeck_before + 1 &&
          tg.state.p[0].energy_deck.cards[edeck_before] == en,
          "F1: the recycled card in the energy deck must be the orphaned one");
    /* The regression this closes: the old C put it in the ACTIVE ENERGY ZONE. */
    CHECK_EQ(tg.state.p[0].energy.n, energy_before,
             "F1: the ACTIVE ENERGY ZONE must be unchanged (old C pushed here)");
    CHECK_EQ(tg.state.p[0].discard.n, discard_before,
             "F1: energy must not land in the discard");
    CHECK_EQ(tg.state.p[0].under_cards[AREA_CENTER].n, 0,
             "F1: the under-card slot is drained");

    /* The non-energy half of the same Rust if/else: waitroom (= C discard). */
    bare_board(&tg);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].under_cards[AREA_LEFT].cards[0] = mem;
    tg.state.p[0].under_cards[AREA_LEFT].n = 1;
    edeck_before   = tg.state.p[0].energy_deck.n;
    discard_before = tg.state.p[0].discard.n;
    rb_check_orphaned_under_cards(&tg.state, 0);
    CHECK_EQ(tg.state.p[0].discard.n, discard_before + 1,
             "F1: a non-energy orphan goes to the waitroom (C discard)");
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before,
             "F1: a non-energy orphan must NOT enter the energy deck");

    /* A card under an OCCUPIED slot is not orphaned and must stay put. */
    bare_board(&tg);
    tg.state.p[0].stage[AREA_RIGHT] = mem;
    tg.state.p[0].under_cards[AREA_RIGHT].cards[0] = en;
    tg.state.p[0].under_cards[AREA_RIGHT].n = 1;
    edeck_before = tg.state.p[0].energy_deck.n;
    rb_check_orphaned_under_cards(&tg.state, 0);
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before,
             "F1: an under-card under a LIVE member is not orphaned");
    CHECK_EQ(tg.state.p[0].under_cards[AREA_RIGHT].n, 1,
             "F1: the under-card under a live member stays");
}

/* ===========================================================================
 * T6-F2 --resolution zone: ENERGY -> energy_deck.
 *
 * Rust turn/actions/mod.rs:1864-1895 (check_invalid_resolution_zone):
 *   let cards  = core::mem::take(&mut game_state.resolution_zone.cards);
 *   let owners = core::mem::take(&mut game_state.resolution_zone.owners);
 *   for (index, card_id) in cards.into_iter().enumerate() {
 *       let owner = owners.get(index).copied().unwrap_or(active);
 *       let owner_player = game_state.seat_player_mut(owner);
 *       if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
 *           owner_player.energy_deck.cards.push(card_id);
 *       } else {
 *           owner_player.waitroom.cards.push(card_id);
 *       }
 *   }
 *
 * The population path is real: rb_perform_cheer_check pushes every yelled card
 * through rb_resolution_add (src/core/tracking.c:154), and the yell can turn up
 * an energy card. Only the ENERGY routing is asserted; the owner half needs a
 * field that does not exist in the C (AUDIT_actions_setup.md 4-4).
 * ======================================================================== */
static void T6_F2_resolution_energy_goes_to_energy_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int en = ident(&tg, ENERGY_NO);
    int mem = ident(&tg, MEMBER_NO);
    CHECK(en > 0 && mem > 0, "F2: fixtures must resolve");

    /* The energy half. */
    bare_board(&tg);
    tg.state.active = 0;
    tg.state.resolution.cards[0] = en;
    tg.state.resolution.n = 1;
    int edeck_before   = tg.state.p[0].energy_deck.n;
    int discard_before = tg.state.p[0].discard.n;
    rb_check_invalid_resolution_zone(&tg.state);
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before + 1,
             "F2: a resolved ENERGY card must go to the ENERGY DECK");
    CHECK(tg.state.p[0].energy_deck.n == edeck_before + 1 &&
          tg.state.p[0].energy_deck.cards[edeck_before] == en,
          "F2: the routed card is the resolved energy card");
    CHECK_EQ(tg.state.p[0].discard.n, discard_before,
             "F2: energy must not land in the discard (old C pushed everything here)");
    CHECK_EQ(tg.state.resolution.n, 0, "F2: the resolution zone is drained");

    /* The non-energy half: waitroom == the C discard. */
    bare_board(&tg);
    tg.state.active = 0;
    tg.state.resolution.cards[0] = mem;
    tg.state.resolution.n = 1;
    edeck_before   = tg.state.p[0].energy_deck.n;
    discard_before = tg.state.p[0].discard.n;
    rb_check_invalid_resolution_zone(&tg.state);
    CHECK_EQ(tg.state.p[0].discard.n, discard_before + 1,
             "F2: a resolved non-energy card goes to the waitroom (C discard)");
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before,
             "F2: a non-energy card must NOT enter the energy deck");

    /* An empty resolution zone is a no-op. */
    bare_board(&tg);
    edeck_before = tg.state.p[0].energy_deck.n;
    rb_check_invalid_resolution_zone(&tg.state);
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before,
             "F2: an empty resolution zone changes nothing");
}

/* ===========================================================================
 * T6-F3 --live_set_limit_reduction is cleared every turn.
 *
 * Rust turn/phases.rs:229-243, the LiveCardSetSecondAttacker arm of
 * advance_phase; :232-233 are its FIRST two statements:
 *     game_state.player1.live_card_set_limit_reduction = 0;
 *     game_state.player2.live_card_set_limit_reduction = 0;
 *     Self::check_timing(game_state);
 *     ...
 *     game_state.current_phase = Phase::FirstAttackerPerformance;
 *
 * The C incremented the field and never cleared it. The two corpus abilities
 * that increment it (PL!HS-bp2-018-N, PL!N-bp4-026-L) promise the reduction
 * applies to the NEXT live-card-set phase only.
 * ======================================================================== */
static void T6_F3_live_set_limit_reduction_cleared_each_turn(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mem = ident(&tg, MEMBER_NO);
    CHECK(mem > 0, "F3: fixture must resolve");

    /* Park the game on the LiveCardSetSecondAttacker window with both seats
     * carrying the reduction, exactly as a fired PL!HS-bp2-018-N leaves them. */
    bare_board(&tg);
    tg.state.phase = RB_PHASE_LIVE_SET_SECOND;
    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;
    tg.state.active = 1;
    tg.state.live_set_limit_reduction[0] = 1;
    tg.state.live_set_limit_reduction[1] = 1;

    /* The first live-set window must NOT clear it --Rust clears only on the
     * SECOND window, i.e. "the next" live-card-set phase is the one still to
     * come. */
    tg.state.phase = RB_PHASE_LIVE_SET;
    tg.state.active = 0;
    rb_advance_phase(&tg.state);
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET_SECOND,
             "F3: the first window hands over to the second");
    CHECK(tg.state.live_set_limit_reduction[0] == 1 &&
          tg.state.live_set_limit_reduction[1] == 1,
          "F3: the FIRST window must not clear the reduction (Rust :232-233 "
          "live in the SECOND window's arm)");

    /* Now advance the second window: both counters must read 0. */
    rb_advance_phase(&tg.state);
    CHECK_EQ(tg.state.phase, RB_PHASE_PERFORMANCE,
             "F3: the second window enters FirstAttackerPerformance");
    CHECK_EQ(tg.state.live_set_limit_reduction[0], 0,
             "F3: player1's live_set_limit_reduction is cleared (phases.rs:232)");
    CHECK_EQ(tg.state.live_set_limit_reduction[1], 0,
             "F3: player2's live_set_limit_reduction is cleared (phases.rs:233)");

    /* A second round must clear a fresh reduction too, not only the first. */
    tg.state.phase = RB_PHASE_LIVE_SET_SECOND;
    tg.state.active = 1;
    tg.state.live_set_limit_reduction[0] = 1;
    tg.state.live_set_limit_reduction[1] = 0;
    rb_advance_phase(&tg.state);
    CHECK_EQ(tg.state.live_set_limit_reduction[0], 0,
             "F3: the reset repeats every round, not only in round one");
}

/* ===========================================================================
 * T6-F4 --check_timing refreshes in FIRST-ATTACKER order.
 *
 * Rust turn/actions/mod.rs:1632-1638 (check_timing), and Player::refresh at
 * core/player.rs:634-648:
 *     pub fn refresh(&mut self) {
 *         if self.main_deck.is_empty() && !self.waitroom.cards.is_empty() {
 *             let mut waitroom_cards = self.waitroom.take_all();
 *             crate::rng::shuffle_slice(&mut waitroom_cards);
 *             for card in waitroom_cards { self.main_deck.cards.push(card); }
 *             self.deck_refreshed_this_turn = true;
 *         }
 *     }
 * `shuffle_slice` draws from the process-global RNG, so WHICH seat refreshes
 * first decides the shuffle order and therefore the resulting deck order.
 *
 * `player1.is_first_attacker` is the C's `g->first_attacker == 0`.
 * ======================================================================== */
/* The 16 fixture ids are allocated ONCE, into a process-global pool that
 * test_game_new() does not touch, so both halves of the comparison below
 * place the very same card instances. Allocating them per half (the first
 * draft of this case did) compares two different sets of ids and proves
 * nothing. Eight cards per seat, not four: a shuffle that happens to return the
 * identity permutation is 1/40320, so a same-order result means the order
 * really is not being consulted rather than an unlucky draw.
 *
 * Sixteen DISTINCT COPIES of one print, not sixteen prints: the test only needs
 * sixteen ids that shuffle observably, and a hard-coded list of card numbers
 * rots the moment the corpus renames one (the first draft listed
 * PL!-sd1-023-SD / -024-SD / -025-SD, none of which exist - the DB has the -P
 * prints, which the identity guard caught and reported). rb_create_card_copy
 * hands out a fresh id per call, so the ids stay distinct. */
#define F4_N 8
static int f4_p1_ids[F4_N];
static int f4_p2_ids[F4_N];
static int f4_ids_ready;

static void F4_make_ids(TestGame *tg)
{
    if (f4_ids_ready) return;
    for (int i = 0; i < F4_N; i++) f4_p1_ids[i] = test_new_id(tg, "PL!-sd1-010-SD");
    for (int i = 0; i < F4_N; i++) f4_p2_ids[i] = test_new_id(tg, "PL!-sd1-010-SD");
    for (int i = 0; i < F4_N; i++)
        if (f4_p1_ids[i] < 0 || f4_p2_ids[i] < 0) {
            fprintf(stderr, "SETUP BUG: F4 fixture id %d failed to allocate\n", i);
            setup_bugs++; assertions++;
            return;
        }
    f4_ids_ready = 1;
}

/* Both decks empty, both waitrooms (= the C `discard`) full. */
static void F4_fixture(TestGame *tg)
{
    bare_board(tg);
    for (int i = 0; i < F4_N; i++) {
        if (f4_p1_ids[i] < 0) continue;
        RbPlayer *p = &tg->state.p[0];
        p->discard.cards[p->discard.n++] = f4_p1_ids[i];
    }
    for (int i = 0; i < F4_N; i++) {
        if (f4_p2_ids[i] < 0) continue;
        RbPlayer *p = &tg->state.p[1];
        p->discard.cards[p->discard.n++] = f4_p2_ids[i];
    }
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
}

/* Is `deck[0..n)` exactly `want[0..F4_N)` in some order? */
static int F4_is_permutation_of(const int *deck, int n, const int *want)
{
    if (n != F4_N) return 0;
    int used[F4_N];
    for (int i = 0; i < F4_N; i++) used[i] = 0;
    for (int i = 0; i < n; i++) {
        int hit = -1;
        for (int k = 0; k < F4_N; k++)
            if (!used[k] && want[k] == deck[i]) { hit = k; break; }
        if (hit < 0) return 0;
        used[hit] = 1;
    }
    return 1;
}

static void T6_F4_refresh_order_is_first_attacker_first(void)
{
    static TestGame tg;
    int deck_p1_first[F4_N], deck_p2_first[F4_N];
    int deck_p1_second[F4_N], deck_p2_second[F4_N];

    /* -- first_attacker == 0: seat 1's waitroom is shuffled FIRST -- */
    test_game_new(&tg);
    F4_make_ids(&tg);
    F4_fixture(&tg);
    tg.state.first_attacker = 0;
    rb_seed(0x5EED1234u);
    rb_check_timing(&tg.state);
    CHECK_EQ(tg.state.p[0].deck.n, F4_N, "F4a: seat 1 refreshed its whole waitroom");
    CHECK_EQ(tg.state.p[1].deck.n, F4_N, "F4a: seat 2 refreshed its whole waitroom");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "F4a: seat 1's waitroom is emptied");
    CHECK_EQ(tg.state.p[1].discard.n, 0, "F4a: seat 2's waitroom is emptied");
    for (int i = 0; i < F4_N; i++) {
        deck_p1_first[i] = tg.state.p[0].deck.cards[i];
        deck_p2_first[i] = tg.state.p[1].deck.cards[i];
    }

    /* -- first_attacker == 1: seat 2's waitroom is shuffled FIRST -- */
    test_game_new(&tg);
    F4_fixture(&tg);
    tg.state.first_attacker = 1;
    rb_seed(0x5EED1234u);
    rb_check_timing(&tg.state);
    CHECK_EQ(tg.state.p[0].deck.n, F4_N, "F4b: seat 1 refreshed its whole waitroom");
    CHECK_EQ(tg.state.p[1].deck.n, F4_N, "F4b: seat 2 refreshed its whole waitroom");
    for (int i = 0; i < F4_N; i++) {
        deck_p1_second[i] = tg.state.p[0].deck.cards[i];
        deck_p2_second[i] = tg.state.p[1].deck.cards[i];
    }

    /* Each seat still holds exactly its own waitroom, permuted. */
    CHECK(F4_is_permutation_of(deck_p1_first, F4_N, f4_p1_ids),
          "F4a: seat 1's deck is a permutation of its own waitroom");
    CHECK(F4_is_permutation_of(deck_p2_first, F4_N, f4_p2_ids),
          "F4a: seat 2's deck is a permutation of its own waitroom");
    CHECK(F4_is_permutation_of(deck_p1_second, F4_N, f4_p1_ids),
          "F4b: seat 1's deck is a permutation of its own waitroom");
    CHECK(F4_is_permutation_of(deck_p2_second, F4_N, f4_p2_ids),
          "F4b: seat 2's deck is a permutation of its own waitroom");

    /* THE assertion. Both shuffles draw from ONE process-global stream, so
     * reversing the seat order reverses which shuffle gets the first draw and
     * each seat's deck comes out in a DIFFERENT order. A C that refreshed
     * 0-then-1 unconditionally would produce byte-identical decks here. */
    int p1_same = 1, p2_same = 1;
    for (int i = 0; i < F4_N; i++) {
        if (deck_p1_first[i] != deck_p1_second[i]) p1_same = 0;
        if (deck_p2_first[i] != deck_p2_second[i]) p2_same = 0;
    }
    CHECK(!p1_same,
          "F4: seat 1's deck order must depend on who is the first attacker "
          "(actions/mod.rs:1632-1638 refreshes the first attacker first)");
    CHECK(!p2_same,
          "F4: seat 2's deck order must depend on who is the first attacker "
          "(actions/mod.rs:1632-1638 refreshes the first attacker first)");

    /* And with first_attacker == 0 the FIRST draw must have gone to seat 1:
     * seat 2 cannot have produced seat 1's order from the same stream
     * position. That is the direction the fix pins. */
    CHECK(memcmp(deck_p1_first, deck_p2_second, sizeof(int) * F4_N) != 0,
          "F4: the first RNG draw belongs to the first attacker (seat 1 here)");
}

/* ===========================================================================
 * T6-F5 --a CONSTANT "cannot activate" member keeps its wait orientation.
 *
 * Rust turn/phases.rs:110-138, the Active-phase `to_activate` builder:
 *     let to_activate: Vec<i16> = turn_player.stage.stage.iter()
 *         .filter_map(|&cid| {
 *             if cid == -1 { return None; }
 *             if game_state.mods.get_orientation_modifier(cid) != Some("wait") {
 *                 return None;
 *             }
 *             if game_state.constant_cannot_activate_members.iter()
 *                 .any(|x| x.parse::<i16>() == Ok(cid)) { return None; }
 *             if game_state.mods.is_delayed_cannot_active(cid) { return None; }
 *             Some(cid)
 *         }).collect();
 *     for &cid in &to_activate {
 *         game_state.mods.add_orientation_modifier(cid, "active");
 *     }
 *
 * The C has neither the GameState field nor the filter, so it activated the
 * member. 1 of the 985 corpus abilities reaches the self-target half.
 * ======================================================================== */
static void T6_F5_constant_cannot_activate_blocks_active_phase(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int omiko = ident(&tg, OMIKO_P);
    CHECK(omiko > 0, "F5: %s must resolve", OMIKO_P);

    /* The card must really carry the constant restriction this test is about;
     * otherwise the case would pass vacuously. */
    int has_const_cannot = 0;
    {
        int n = rb_card_num_abilities((uint32_t)omiko);
        for (int i = 0; i < n; i++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)omiko, i, &ab)) continue;
            if (rb_ability_has_trigger(&ab, RB_TK_CONSTANT) && ab.effect &&
                ab.effect->action && !strcmp(ab.effect->action, "restriction"))
                has_const_cannot = 1;
            rb_free_ability(&ab);
        }
    }
    CHECK(has_const_cannot,
          "F5: %s must carry a CONSTANT restriction ability "
          "(modifiers.rs:1081-1092)", OMIKO_P);

    /* Seat 0 holds Omiko in WAIT orientation; the Active phase must leave it
     * waited. */
    bare_board(&tg);
    tg.state.phase = RB_PHASE_ACTIVE;
    tg.state.active = 0;
    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;
    tg.state.p[0].stage[AREA_CENTER] = omiko;
    tg.state.p[0].stage_wait[AREA_CENTER] = 1;
    rb_mods_set_orientation(&tg.state.mods, omiko, "wait");

    rb_advance_phase(&tg.state);

    CHECK_EQ(tg.state.phase, RB_PHASE_ENERGY,
             "F5: the Active phase still completes");
    const char *orient = rb_mods_get_orientation(&tg.state.mods, omiko);
    CHECK(orient && !strcmp(orient, "wait"),
          "F5: %s keeps its WAIT orientation across the Active phase "
          "(phases.rs:125-131) --got \"%s\"",
          OMIKO_P, orient ? orient : "(null)");
    CHECK_EQ(tg.state.p[0].stage_wait[AREA_CENTER], 1,
             "F5: stage_wait stays set for a constant cannot_activate member");

    /* Control: a member with NO such restriction must still activate, so the
     * filter above cannot be passing merely because activation is broken. */
    {
        static TestGame tg2;
        test_game_new(&tg2);
        int plain = ident(&tg2, MEMBER_NO);
        CHECK(plain > 0, "F5: control fixture must resolve");
        bare_board(&tg2);
        tg2.state.phase = RB_PHASE_ACTIVE;
        tg2.state.active = 0;
        tg2.state.first_attacker = 0;
        tg2.state.second_attacker = 1;
        tg2.state.p[0].stage[AREA_CENTER] = plain;
        tg2.state.p[0].stage_wait[AREA_CENTER] = 1;
        rb_mods_set_orientation(&tg2.state.mods, plain, "wait");
        rb_advance_phase(&tg2.state);
        const char *o2 = rb_mods_get_orientation(&tg2.state.mods, plain);
        CHECK(o2 && !strcmp(o2, "active"),
              "F5: a member without the restriction DOES activate "
              "(got \"%s\")", o2 ? o2 : "(null)");
        CHECK_EQ(tg2.state.p[0].stage_wait[AREA_CENTER], 0,
                 "F5: the control member's stage_wait is cleared");
    }
}

/* -- runner -- */
static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures;
        current_test = name; fn(); current_test = "(none)";
        if (failures - f0 > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions]\n", (failures - f0) ? "FAILED" : "ok",
               name, assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s)\n", name,
               assertions, failures - f0);
        fflush(stdout); fflush(stderr);
        _exit(failures > f0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    int crashed = WIFSIGNALED(status);
    if (crashed) n_tests_crashed++; else n_tests_ok++;
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FATAL: could not load the card DB\n");
        return 2;
    }
    test_card_copy_pool_reset();

    printf("-- T6-F1 check_orphaned_under_cards routing (actions/mod.rs:1851-1856) --\n");
    run("t6_f1_orphan_energy_goes_to_energy_deck", T6_F1_orphan_energy_goes_to_energy_deck);

    printf("-- T6-F2 check_invalid_resolution_zone routing (actions/mod.rs:1885-1894) --\n");
    run("t6_f2_resolution_energy_goes_to_energy_deck", T6_F2_resolution_energy_goes_to_energy_deck);

    printf("-- T6-F3 live_set_limit_reduction reset (phases.rs:232-233) --\n");
    run("t6_f3_live_set_limit_reduction_cleared_each_turn",
        T6_F3_live_set_limit_reduction_cleared_each_turn);

    printf("-- T6-F4 check_timing refresh order (actions/mod.rs:1632-1638) --\n");
    run("t6_f4_refresh_order_is_first_attacker_first",
        T6_F4_refresh_order_is_first_attacker_first);

    printf("-- T6-F5 constant cannot_activate (phases.rs:125-131 + modifiers.rs:1081-1092) --\n");
    run("t6_f5_constant_cannot_activate_blocks_active_phase",
        T6_F5_constant_cannot_activate_blocks_active_phase);

    printf("\n==== parity_t6_action_setup: %d tests ok, %d failed, %d crashed "
           "(%d setup bugs) ====\n",
           n_tests_ok, n_tests_failed, n_tests_crashed, setup_bugs);
    return failures ? 1 : 0;
}