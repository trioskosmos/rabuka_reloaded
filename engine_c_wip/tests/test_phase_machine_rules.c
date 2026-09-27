#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define REQUIRE(c, msg) do { \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
        return; \
    } \
} while (0)

#define REQUIRE_EQ(a, b, msg) do { \
    int actual_ = (a); \
    int expected_ = (b); \
    if (actual_ != expected_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, actual_, expected_); \
        failures++; \
        return; \
    } \
} while (0)

static const char *orientation(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}

static void wait(TestGame *tg, int cid)
{
    rb_mods_set_orientation(&tg->state.mods, cid, "wait");
    tg->state.p[rb_owner_of_card(&tg->state, cid)].stage_wait[0] = 1;
}

static int normal_phase_seat(TestGame *tg)
{
    if (tg->state.phase == RB_PHASE_LIVE_SET ||
        tg->state.phase == RB_PHASE_PERFORMANCE ||
        tg->state.phase == RB_PHASE_VICTORY ||
        tg->state.phase == RB_PHASE_DONE) {
        return -1;
    }
    if (tg->state.active == tg->state.first_attacker) return 0;
    if (tg->state.active == tg->state.second_attacker) return 1;
    return -1;
}

static void pass(TestGame *tg)
{
    test_pass(tg);
    while (test_has_pending_choice(tg)) {
        test_select_indices(tg, NULL, 0);
    }
}

static int pass_until(TestGame *tg, int phase, int seat)
{
    int guard = 0;
    while (!(tg->state.phase == phase && normal_phase_seat(tg) == seat)) {
        guard++;
        if (guard > 24) return 0;
        pass(tg);
    }
    return 1;
}

static void fill_decks(TestGame *tg, int filler)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void fill_energy_deck(TestGame *tg, int player, int count)
{
    int energy = test_id(tg, "LL-E-001-SD");
    for (int i = 0; i < count; i++) {
        test_add_to_energy_deck(tg, player, energy);
    }
}

static int bag_contains(const RbBag *bag, int card)
{
    for (int i = 0; i < bag->n; i++) {
        if (bag->cards[i] == card) return 1;
    }
    return 0;
}

static int bag_equals_pair(const RbBag *bag, int first, int second)
{
    return bag->n == 2 && bag->cards[0] == first && bag->cards[1] == second;
}

static void clear_bag(RbBag *bag)
{
    bag->n = 0;
}

static int set_live_for(TestGame *tg, int pl, int card_id)
{
    if (pl < 0 || pl > 1) return 0;
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] != card_id) continue;
        int card = rb_hand_remove_card(P, i);
        return card >= 0 && rb_live_add_card(P, card) == 0;
    }
    return 0;
}

/* Drive the round from wherever it stands until it rolls over into the next turn's
   Active phase, answering the two prompts a live can raise (the ライブ成功時
   look_and_select and the auto-ability prompt) the same way
   tests/test_performance_phase_rules.c:run_full_turn does. */
static int run_round(TestGame *tg)
{
    for (int i = 0; i < 60; i++) {
        if (tg->state.phase == RB_PHASE_ACTIVE && !test_has_pending_choice(tg)) return 1;
        const char *choice = test_pending_choice_type(tg);
        if (strcmp(choice, "SelectCard") == 0) {
            rb_resume_with_choice(&tg->state, -1);
        } else if (strcmp(choice, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&tg->state, 0);
        } else if (choice[0] == 0) {
            test_pass(tg);
        } else {
            return 0;
        }
    }
    return 0;
}

static void test_active_phase_stands_only_the_turn_players_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    int p1_member = test_id(&tg, "PL!-sd1-010-SD");
    int p2_member = test_id(&tg, "PL!N-PR-008-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = p1_member;
    tg.state.p[1].stage[0] = p2_member;
    wait(&tg, p1_member);
    wait(&tg, p2_member);

    for (int seat = 0; seat < 2; seat++) {
        int e1 = test_id(&tg, "LL-E-001-SD");
        int e2 = test_id(&tg, "LL-E-001-SD");
        RbPlayer *player = &tg.state.p[seat];
        player->energy.cards[player->energy.n++] = e1;
        player->energy.cards[player->energy.n++] = e2;
        player->energy_active = 1;
    }

    fill_decks(&tg, filler);

    test_pass(&tg);
    REQUIRE_EQ(tg.state.phase, RB_PHASE_ACTIVE,
               "one pass reaches the second attacker's Active phase");
    REQUIRE_EQ(normal_phase_seat(&tg), 1,
               "after one pass from first normal phase the second attacker acts");

    test_pass(&tg);
    REQUIRE_EQ(tg.state.phase, RB_PHASE_ENERGY,
               "one more pass reaches the second attacker's Energy phase");

    const char *p2_orientation = orientation(&tg, p2_member);
    REQUIRE(p2_orientation == NULL || strcmp(p2_orientation, "wait") != 0,
            "7.4.1: the turn player's waited member stands");
    REQUIRE(orientation(&tg, p1_member) != NULL &&
                strcmp(orientation(&tg, p1_member), "wait") == 0,
            "the non-turn player's waited member must stay waited");
    REQUIRE_EQ(tg.state.p[1].energy_active, 2,
               "turn player's waited energy stands");
    REQUIRE_EQ(tg.state.p[0].energy_active, 1,
               "non-turn player's waited energy stays put");
}

static void test_energy_phase_draws_one_and_empty_deck_skips(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    fill_energy_deck(&tg, 0, 1);
    int seeded = tg.state.p[0].energy_deck.cards[0];
    fill_decks(&tg, filler);

    test_pass(&tg);
    test_pass(&tg);
    REQUIRE_EQ(normal_phase_seat(&tg), 1,
               "the second attacker reaches the Energy phase");
    REQUIRE_EQ(tg.state.p[1].energy_deck.n, 0,
               "precondition: their energy deck is empty");
    REQUIRE_EQ(tg.state.p[1].energy.n, 0,
               "empty energy deck places nothing and does not panic");

    REQUIRE(pass_until(&tg, RB_PHASE_ENERGY, 0),
            "the first attacker's Energy phase is reached");
    test_pass(&tg);

    REQUIRE_EQ(tg.state.p[0].energy.n, 1,
               "7.5.2: the turn player's Energy phase moves her top deck card");
    REQUIRE(bag_contains(&tg.state.p[0].energy, seeded),
            "the seeded card was moved in top-of-deck order");
}

static void test_draw_phase_on_empty_main_deck_refreshes_then_draws(void)
{
    TestGame tg;
    test_game_new(&tg);

    tg.state.p[0].deck.n = 0;
    tg.state.p[0].discard.n = 0;
    for (int i = 0; i < 3; i++) {
        int card = test_id(&tg, "PL!-sd1-010-SD");
        tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = card;
    }
    int f = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&tg, 1, f);
    }

    REQUIRE(pass_until(&tg, RB_PHASE_ENERGY, 0),
            "the first attacker's Energy phase is reached before the draw");
    int hand_before = tg.state.p[0].hand.n;

    REQUIRE(pass_until(&tg, RB_PHASE_MAIN, 0),
            "the first attacker's Main phase is reached after the draw");
    REQUIRE_EQ(tg.state.p[0].hand.n, hand_before + 1,
               "7.6.2: exactly one card drawn despite the empty deck");
    REQUIRE_EQ(tg.state.p[0].deck.n, 2,
               "3 waitroom cards shuffled in and 1 drawn out");
}

static void test_live_card_set_refill_draws_placed_count(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live1 = test_id(&tg, "PL!-sd1-020-SD");
    int live2 = test_id(&tg, "PL!-sd1-020-SD");

    fill_decks(&tg, filler);
    test_add_to_hand(&tg, live1);

    int guard = 0;
    while (tg.state.phase != RB_PHASE_LIVE_SET && guard < 12) {
        guard++;
        pass(&tg);
    }

    test_set_live_card(&tg, 0, live1);
    if (tg.state.p[0].live.n < RB_MAX_ZONE) {
        tg.state.p[0].live.cards[tg.state.p[0].live.n++] = live2;
    }

    int top_b = test_id(&tg, "PL!-sd1-010-SD");
    int top_a = test_id(&tg, "PL!-sd1-010-SD");
    test_insert_deck_top(&tg, 0, top_b);
    test_insert_deck_top(&tg, 0, top_a);

    int p2_hand_before = tg.state.p[1].hand.n;

    test_pass(&tg);

    REQUIRE(bag_equals_pair(&tg.state.p[0].hand, top_a, top_b),
            "8.2.2: refill equals placed count 2 and uses deck top order");

    int p2_hand_at_refill = tg.state.p[1].hand.n;
    REQUIRE_EQ(p2_hand_before, p2_hand_at_refill,
               "the second attacker's boundary refills zero cards");
    test_pass(&tg);
    REQUIRE_EQ(tg.state.p[1].hand.n, p2_hand_at_refill,
               "placed 0 live cards means 0 refill draws for the second attacker");
}

/* Rule 8.4.13 — the VICTORY-phase rollover must NOT run its own first-attacker
   election, and the seats must follow the SUCCESS-ZONE DELTA that live.c's
   rb_move_to_success_and_update_attacker already applied.

   Rust has exactly two writers of `is_first_attacker`: actions/mod.rs:230-231
   (RPS) and live.rs:1283-1289, the latter inside move_to_success_and_update_attacker,
   which runs from execute_live_victory_determination — i.e. from inside
   rb_execute_performance_phase, BEFORE advance_phase's VICTORY branch. The Rust
   counterpart of that branch (phases.rs:265-311) writes no seat at all. The Rust
   election reads p1_added / p2_added, i.e. whether a card actually LANDED in the
   success zone — not whether that seat won the live.

   The winner-based election that used to live in phase.c disagreed exactly when a
   sole winner placed nothing. The fixture below builds that case:
     - the seats start as (first=1, second=0), so P1 is the SECOND attacker;
     - P1 stands a member and sets PL!-sd1-019-SD, so she passes and is the SOLE
       live winner; P2 sets nothing, so P2 cannot win;
     - a standing "cannot place into the success live zone" prohibition stops P1's
       live card from being placed, so process_player_live_result (live.rs:1437-1458)
       routes it to the waitroom and the success-zone DELTA is 0/0.
   Rust therefore keeps the previous first attacker. The removed phase.c election
   saw "P1 won alone" and flipped the seats to (0, 1) anyway. */
static void setup_sole_winner_places_nothing(TestGame *tg, int *live_p1)
{
    test_game_new(tg);
    int live = test_id(tg, "PL!-sd1-019-SD");
    int member = test_id(tg, "PL!-sd1-001-SD");
    int filler = test_id(tg, "PL!-sd1-010-SD");
    *live_p1 = live;

    clear_bag(&tg->state.p[0].deck);
    clear_bag(&tg->state.p[0].hand);
    clear_bag(&tg->state.p[0].discard);
    clear_bag(&tg->state.p[0].success);
    clear_bag(&tg->state.p[0].energy_deck);
    clear_bag(&tg->state.p[1].deck);
    clear_bag(&tg->state.p[1].hand);
    clear_bag(&tg->state.p[1].discard);
    clear_bag(&tg->state.p[1].success);
    clear_bag(&tg->state.p[1].energy_deck);
    for (int i = 0; i < 40; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    /* Only P1 stands a member, so only P1 can satisfy the live's h01 demand. */
    tg->state.p[0].stage[1] = member;
    test_add_to_hand(tg, live);

    /* P2 opens the live: the seats are deliberately (first=1, second=0) so the
       removed winner-based election would have been VISIBLE if it ran. */
    tg->state.first_attacker = 1;
    tg->state.second_attacker = 0;

    /* Standing restriction: the live card may not be placed into the success zone,
       so the sole winner's success-zone DELTA stays 0. */
    snprintf(tg->state.prohibition[tg->state.n_prohibition], 48,
             "restriction:cannot_place:success_live_zone");
    tg->state.n_prohibition++;
}

/* Drive one live through to the roll-over. P2 sets nothing and P1 sets one card, so
   the round is: P2 live set, P1 live set, first window, second window, victory. */
static void play_sole_winner_live(TestGame *tg, int live_p1)
{
    test_pass(tg);                                   /* MAIN -> P2 live card set */
    test_pass(tg);                                   /* P2 places nothing -> P1 */
    if (set_live_for(tg, 0, live_p1)) test_pass(tg); /* P1 sets + confirms */
}

static void test_first_attacker_survives_a_win_that_placed_nothing(void)
{
    TestGame tg;
    int live_p1 = -1;
    setup_sole_winner_places_nothing(&tg, &live_p1);

    play_sole_winner_live(&tg, live_p1);
    REQUIRE(run_round(&tg), "the round rolls over into the next Active phase");

    REQUIRE_EQ(tg.state.p1_live_won, 1, "precondition: P1 is the sole live winner");
    REQUIRE_EQ(tg.state.p2_live_won, 0, "precondition: P2 did not win the live");
    REQUIRE_EQ(tg.state.p[0].success.n, 0,
               "8.4.13: the winner placed no success card, so the delta is 0");
    REQUIRE(test_zone_has_id(&tg, 0, "discard", live_p1),
            "an unplaceable live card is routed to the waitroom");

    REQUIRE_EQ(tg.state.first_attacker, 1,
               "8.4.13: a win that placed nothing does NOT re-elect the winner");
    REQUIRE_EQ(tg.state.second_attacker, 0,
               "8.4.13: the seat order is left untouched by a no-placement win");
    REQUIRE_EQ(tg.state.active, 1,
               "the next round opens with the retained first attacker");
}

/* Companion half of the same rule, and the guard against over-correcting: when the
   sole winner DOES place a success card, the seats must still move to that seat.
   This is the election live.c owns (live.rs:1283-1289) and the one the removed
   phase.c lines happened to agree with — proving the fix delegates rather than
   disables the promotion. */
static void test_first_attacker_follows_a_placed_success_card(void)
{
    TestGame tg;
    int live_p1 = -1;
    setup_sole_winner_places_nothing(&tg, &live_p1);
    tg.state.n_prohibition = 0; /* lift the restriction: P1 can now place */

    play_sole_winner_live(&tg, live_p1);
    REQUIRE(run_round(&tg), "the round rolls over into the next Active phase");

    REQUIRE_EQ(tg.state.p1_live_won, 1, "precondition: P1 is the sole live winner");
    REQUIRE_EQ(tg.state.p[0].success.n, 1,
               "the sole winner's live card lands in the success zone");

    REQUIRE_EQ(tg.state.first_attacker, 0,
               "8.4.13: the sole placer becomes the first attacker");
    REQUIRE_EQ(tg.state.second_attacker, 1,
               "8.4.13: the other seat becomes the second attacker");
    REQUIRE_EQ(tg.state.active, 0,
               "the next round opens with the newly elected first attacker");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_active_phase_stands_only_the_turn_players_cards();
    test_energy_phase_draws_one_and_empty_deck_skips();
    test_draw_phase_on_empty_main_deck_refreshes_then_draws();
    test_live_card_set_refill_draws_placed_count();
    test_first_attacker_survives_a_win_that_placed_nothing();
    test_first_attacker_follows_a_placed_success_card();

    rb_unload();
    if (failures) return 1;
    printf("ALL PHASE MACHINE RULE CHECKS PASSED\n");
    return 0;
}
