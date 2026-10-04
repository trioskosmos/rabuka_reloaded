/* tests/test_live_zone_slot_validation.c
 *
 * RULE 10.5.1 -- live-zone slot validation.
 * Rust: turn::actions::mod, `fn check_invalid_live_cards(game_state: &mut GameState,
 * is_p1: bool)`, cited by CONTENT (the Rust tree is being edited, so no line pin):
 *
 *     let seat = if is_p1 { 0 } else { 1 };
 *     if !game_state.seat_player(seat).live_card_zone.face_up { return; }
 *     let invalids = player.live_card_zone.cards.iter().enumerate()
 *         .filter_map(|(i, &card_id)| {
 *             let card = game_state.card_database.get_card(card_id)?;   // unknown id -> kept
 *             if !card.is_live() { Some((i, card_id, card.is_energy())) } else { None }
 *         }).collect::<Vec<_>>();
 *     if invalids.is_empty() { return; }
 *     for &(i, card_id, is_energy) in invalids.iter().rev() {
 *         player.live_card_zone.cards.remove(i);
 *         if is_energy { player.energy_deck.cards.push(card_id); }   // dest "energy_deck"
 *         else          { player.waitroom.add_card(card_id); }      // dest "waitroom"
 *     }
 *     // then, for every card that moved, ONE push_movement_event:
 *     //   (card_id, "live_card_zone", dest_zone, None, player_id, false)
 *
 * The Rust twins this suite ports, by test name:
 *   engine/tests/test_modules/rules/conditions/live_card_zone_movement_test.rs
 *     invalid_live_card_discard_records_turn_movements
 *     invalid_energy_card_in_live_zone_goes_to_energy_deck
 *     check_invalid_live_cards_distinguishes_players
 *     (and the survivor half of live_card_in_live_zone_stays, the paired positive)
 *
 * WHY THIS FILE EXISTS AT ALL.  The four suites that already exercise Rule 10.5.1
 * ("mechanics", "live_card_zone_movement", "parity_rules_conditions",
 * "parity_rules_conditions2") all hand-build a board and call rb_check_timing
 * WITHOUT setting "live_face_up[seat] = 1", which is the one line Rust's own twin
 * carries before it calls check_timing.  The gate is faithful, so those fixtures
 * never reach the rule and report it missing.  That was measured, not assumed:
 * adding "tg.state.live_face_up[0] = 1;" and changing nothing else took
 * "mechanics" from 7 "^FAIL" lines to 0 and "live_card_zone_movement" from 15 to
 * 0, in an isolated snapshot, with both objects rebuilt from source.
 *
 * Those fixtures are in CONTESTED test files this suite's author does not own,
 * so the fix lands here instead: a suite whose fixture is faithful from the start
 * and which ALSO pins the gate itself, so the rule cannot silently go missing
 * again -- and, more importantly, cannot silently become UNREACHABLE again.  Every
 * positive here is paired with a negative on the same board (gate closed, or the
 * card is genuinely live), so no assertion in this file can pass vacuously.
 *
 * The C implementation is src/turn/phase.c `rb_check_invalid_live_cards`, called
 * from `rb_check_timing`.  Its routing table (waitroom for a member, energy deck
 * for an energy card) and its rb_record_card_movement call are both asserted here
 * by CONTENT, not by trusting the comment.
 *
 * CARD NUMBERS, verified against cards/cards.json before use:
 *   PL!-sd1-010-SD  member  (type メンバー)   -- invalid in the live slot
 *   LL-E-001-SD     energy  (type エネルギー) -- invalid in the live slot
 *   PL!N-sd1-025-SD live    (type ライブ)     -- VALID in the live slot
 * The card-number trap is live in this tree: rb_find_card_by_no's substring
 * fallback has already resolved `PL!HS-bp1-002-CL` to a different card
 * (2793, Murano Sayaka).  All three numbers above are exact keys in cards.json.
 *
 * sizeof(GameState) is 1 195 656 B and is pinned by RB_GAMESTATE_BYTES_PINNED, so
 * every TestGame below is `static`; do not add a non-static one.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* Declared here rather than added to include/rabuka.h: the header is being edited
 * concurrently by the coordinator, this is a declaration-only addition with no
 * ABI effect, and a file-local prototype keeps the suite self-contained. The
 * definition is src/turn/phase.c rb_execute_performance_phase. */
void rb_execute_performance_phase(GameState *g, int is_first);

static int failures;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", (msg)); failures++; FLAT_VERDICT(); } \
    else printf("ok: %s\n", (msg)); \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", (msg), a_, e_); \
        failures++; FLAT_VERDICT(); \
    } else printf("ok: %s\n", (msg)); \
} while (0)

static const char *CARD_MEMBER = "PL!-sd1-010-SD";
static const char *CARD_ENERGY = "LL-E-001-SD";
static const char *CARD_LIVE    = "PL!N-sd1-025-SD";

static void fill_decks(TestGame *tg, int filler) {
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Append straight into the live zone. test_add_to_live removes the card from the
 * hand first and is seat 0 only, so a per-seat raw push is what the Rust twins
 * do (`live_card_zone.cards.push(member)`). */
static int push_live(TestGame *tg, int pl, int card_id) {
    RbBag *b = &tg->state.p[pl].live;
    if (b->n >= RB_MAX_LIVE_CARDS) return 0;
    b->cards[b->n++] = card_id;
    return 1;
}

static int bag_has(const RbBag *b, int card_id) {
    for (int i = 0; i < b->n; i++) if (b->cards[i] == card_id) return 1;
    return 0;
}

static const RbBatchMovement *movement_at(const GameState *g, int i) {
    return (i >= 0 && i < g->n_batch_movements) ? &g->batch_movements[i] : NULL;
}

/* The index of the LAST batch_movements entry for `card_id`, or -1. Rust reads
 * `turn_movements.last()`; the C folds batch_movements / turn_movements into one
 * ordered log (src/core/modifiers.c, `rb_record_card_movement`), so "last for this
 * card" is the faithful read of the same event. */
static int movement_index_for(const GameState *g, int card_id) {
    int found = -1;
    for (int i = 0; i < g->n_batch_movements; i++) {
        if (g->batch_movements[i].moved_card_id == card_id) found = i;
    }
    return found;
}

/* ── 1 ── an invalid MEMBER leaves the live zone for its owner's WAITROOM, and the
 *      move is recorded.  Rust: invalid_live_card_discard_records_turn_movements. */
static void member_leaves_live_zone_for_the_waitroom(void) {
static    TestGame tg;
    test_game_new(&tg);
    int member = test_new_id(&tg, CARD_MEMBER);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (member < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (member and filler are DISTINCT card instances)");
        return;
    }
    CHECK(member != filler, "test_new_id gave two DISTINCT member instances, not one alias");
    CHECK(!rb_card_is_live(member), "the fixture card is genuinely NOT a live card");
    push_live(&tg, 0, member);
    tg.state.p[0].stage[0] = filler;
    fill_decks(&tg, filler);
    /* THE LINE RUST'S TWIN CARRIES AND THE CONTESTED FIXTURES OMIT. */
    tg.state.live_face_up[0] = 1;

    CHECK_EQ(tg.state.n_batch_movements, 0, "no movement recorded before the timing check");
    CHECK(bag_has(&tg.state.p[0].live, member), "precondition: the member is in the live zone");

    rb_check_timing(&tg.state);

    CHECK(!bag_has(&tg.state.p[0].live, member),
          "RULE 10.5.1: the invalid live member leaves the live zone");
    CHECK(bag_has(&tg.state.p[0].discard, member),
          "RULE 10.5.1: the invalid live member lands in its owner's WAITROOM");
    CHECK(!bag_has(&tg.state.p[1].discard, member),
          "the member did not go to the OPPONENT's waitroom");
    CHECK(!bag_has(&tg.state.p[0].energy_deck, member),
          "a member is not an energy card, so it does not go to the energy deck");
    CHECK(tg.state.moved_this_turn[member],
          "the movement is recorded in the per-card moved_this_turn set");
    int mi = movement_index_for(&tg.state, member);
    CHECK(mi >= 0, "RULE 10.5.1: a movement event is recorded for the evicted card");
    if (mi >= 0) {
        const RbBatchMovement *m = movement_at(&tg.state, mi);
        CHECK_EQ(m->source_zone, RB_ZONEID_LIVE_CARD_ZONE,
                 "the movement event names live_card_zone as the source");
        CHECK_EQ(m->dest_zone, RB_ZONEID_WAITROOM,
                 "the movement event names waitroom as the destination");
        CHECK_EQ(m->cause_player_id, 0,
                 "the movement event is attributed to the seat that owned the card");
    }
}

/* ── 2 ── an invalid ENERGY card returns to the owner's ENERGY DECK, and to
 *      nowhere else.  Rust: invalid_energy_card_in_live_zone_goes_to_energy_deck. */
static void energy_returns_to_the_energy_deck(void) {
static    TestGame tg;
    test_game_new(&tg);
    int energy = test_id(&tg, CARD_ENERGY);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (energy < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (an energy card and a member filler)");
        return;
    }
    CHECK(rb_card_is_energy(energy), "the fixture card is genuinely an energy card");
    CHECK(!rb_card_is_live(energy), "an energy card is genuinely NOT a live card");
    push_live(&tg, 0, energy);
    tg.state.p[0].stage[0] = filler;
    fill_decks(&tg, filler);
    tg.state.live_face_up[0] = 1;
    CHECK(bag_has(&tg.state.p[0].live, energy), "precondition: the energy card is in the live zone");

    rb_check_timing(&tg.state);

    CHECK(!bag_has(&tg.state.p[0].live, energy),
          "RULE 10.5.1: the invalid energy card leaves the live zone");
    CHECK(bag_has(&tg.state.p[0].energy_deck, energy),
          "RULE 10.5.1: the invalid energy card returns to the owner's ENERGY DECK");
    CHECK(!bag_has(&tg.state.p[0].discard, energy),
          "the energy card did NOT go to the waitroom");
    int mi = movement_index_for(&tg.state, energy);
    CHECK(mi >= 0, "a movement event is recorded for the evicted energy card");
    if (mi >= 0) {
        const RbBatchMovement *m = movement_at(&tg.state, mi);
        CHECK_EQ(m->source_zone, RB_ZONEID_LIVE_CARD_ZONE,
                 "the energy movement names live_card_zone as the source");
        CHECK_EQ(m->dest_zone, RB_ZONEID_ENERGY_DECK,
                 "the energy movement names energy_deck as the destination");
    }
}

/* ── 3 ── one zone, one of each kind: the ROUTING is per card, not per zone.
 *      Strictly stronger than Rust's twins, which test the member and the energy
 *      card on separate boards; it pins that the two branches of the same `if`
 *      are independent rather than one board's state leaking into the other. */
static void mixed_zone_routes_each_card_by_its_own_type(void) {
static    TestGame tg;
    test_game_new(&tg);
    int member = test_new_id(&tg, CARD_MEMBER);
    int energy = test_id(&tg, CARD_ENERGY);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (member < 0 || energy < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (member, energy and a filler)");
        return;
    }
    push_live(&tg, 0, member);
    push_live(&tg, 0, energy);
    tg.state.p[0].stage[0] = filler;
    fill_decks(&tg, filler);
    tg.state.live_face_up[0] = 1;

    rb_check_timing(&tg.state);

    CHECK_EQ(tg.state.p[0].live.n, 0, "both invalid cards left the live zone");
    CHECK(bag_has(&tg.state.p[0].discard, member), "the member went to the waitroom");
    CHECK(bag_has(&tg.state.p[0].energy_deck, energy), "the energy card went to the energy deck");
    CHECK(!bag_has(&tg.state.p[0].energy_deck, member), "the member did NOT go to the energy deck");
    CHECK(!bag_has(&tg.state.p[0].discard, energy), "the energy card did NOT go to the waitroom");
    CHECK(movement_index_for(&tg.state, member) >= 0, "the member's movement was recorded");
    CHECK(movement_index_for(&tg.state, energy) >= 0, "the energy card's movement was recorded");
}

/* ── 4 ── per-seat isolation and per-seat attribution.
 *      Rust: check_invalid_live_cards_distinguishes_players. */
static void each_seat_evicts_only_its_own_live_zone(void) {
static    TestGame tg;
    test_game_new(&tg);
    int m1 = test_new_id(&tg, CARD_MEMBER);
    int m2 = test_new_id(&tg, CARD_MEMBER);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (m1 < 0 || m2 < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (two DISTINCT member instances and a filler)");
        return;
    }
    CHECK(m1 != m2, "the two members are distinct instances, so per-card reads can tell them apart");
    push_live(&tg, 0, m1);
    push_live(&tg, 1, m2);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[1].stage[0] = filler;
    fill_decks(&tg, filler);
    tg.state.live_face_up[0] = 1;
    tg.state.live_face_up[1] = 1;

    rb_check_timing(&tg.state);

    CHECK(bag_has(&tg.state.p[0].discard, m1), "seat 0's member reached seat 0's waitroom");
    CHECK(bag_has(&tg.state.p[1].discard, m2), "seat 1's member reached seat 1's waitroom");
    CHECK(!bag_has(&tg.state.p[0].discard, m2), "seat 0's waitroom did NOT gain seat 1's member");
    CHECK(!bag_has(&tg.state.p[1].discard, m1), "seat 1's waitroom did NOT gain seat 0's member");
    int a = movement_index_for(&tg.state, m1);
    int b = movement_index_for(&tg.state, m2);
    CHECK(a >= 0, "seat 0's member produced a movement event");
    CHECK(b >= 0, "seat 1's member produced a movement event");
    if (a >= 0) CHECK_EQ(movement_at(&tg.state, a)->cause_player_id, 0, "seat 0's movement is attributed to seat 0");
    if (b >= 0) CHECK_EQ(movement_at(&tg.state, b)->cause_player_id, 1, "seat 1's movement is attributed to seat 1");
}

/* ── 5 ── THE GATE, as a negative on the SAME board as test 1.
 *      Rust: the early `return` above the `invalids` collection.  This is the
 *      paired control for every positive in this file: if the gate were removed,
 *      this test would go red -- and so would nothing else, which is exactly the
 *      failure mode it exists to prevent. */
static void gate_closed_leaves_the_live_zone_untouched(void) {
static    TestGame tg;
    test_game_new(&tg);
    int member = test_new_id(&tg, CARD_MEMBER);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (member < 0 || filler < 0) {
        CHECK(0, "fixtures resolve");
        return;
    }
    push_live(&tg, 0, member);
    tg.state.p[0].stage[0] = filler;
    fill_decks(&tg, filler);
    /* face_up deliberately LEFT AT ITS RESET VALUE (0). Rust: `face_up: false` in
     * LiveCardZone::new, so this is the state of a seat whose live card has been
     * set but whose performance window has not opened. */
    CHECK_EQ(tg.state.live_face_up[0], 0, "the gate starts CLOSED after test_game_new");

    rb_check_timing(&tg.state);

    CHECK(bag_has(&tg.state.p[0].live, member),
          "RULE 10.5.1 GATE: with face_up false the invalid card STAYS in the live zone");
    CHECK(!bag_has(&tg.state.p[0].discard, member),
          "no waitroom entry while the gate is closed");
    CHECK(!tg.state.moved_this_turn[member],
          "no movement is recorded while the gate is closed");
}

/* ── 6 ── THE PAIRED POSITIVE for the `!card.is_live()` predicate.  Without this,
 *      an implementation that evicted EVERY live-zone card would pass tests 1-4.
 *      Rust's twin for the survivor half asserts the live card is still there. */
static void a_real_live_card_survives_the_check(void) {
static    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, CARD_LIVE);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (live < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (a real live card and a member filler)");
        return;
    }
    CHECK(rb_card_is_live(live), "the fixture card is genuinely a LIVE card");
    push_live(&tg, 0, live);
    tg.state.p[0].stage[0] = filler;
    fill_decks(&tg, filler);
    tg.state.live_face_up[0] = 1;

    rb_check_timing(&tg.state);

    CHECK(bag_has(&tg.state.p[0].live, live),
          "a valid LIVE card is NOT evicted -- the rule filters on is_live, not on 'in the zone'");
    CHECK(!bag_has(&tg.state.p[0].discard, live), "the live card did not go to the waitroom");
    CHECK(!bag_has(&tg.state.p[0].energy_deck, live), "the live card did not go to the energy deck");
    CHECK(movement_index_for(&tg.state, live) < 0, "no movement event is recorded for a surviving live card");
}

/* ── 7 ── live + invalid in the SAME zone, gate OPEN: only the invalid one moves.
 *      Together with 3 and 6 this makes both halves of the predicate observable
 *      in one pass, which is what "slot validation" actually means. */
static void a_valid_and_an_invalid_card_share_the_slot(void) {
static    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, CARD_LIVE);
    int member = test_new_id(&tg, CARD_MEMBER);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (live < 0 || member < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (a live card, an invalid member and a filler)");
        return;
    }
    push_live(&tg, 0, live);
    push_live(&tg, 0, member);
    tg.state.p[0].stage[0] = filler;
    fill_decks(&tg, filler);
    tg.state.live_face_up[0] = 1;

    rb_check_timing(&tg.state);

    CHECK_EQ(tg.state.p[0].live.n, 1, "exactly one card remains in the live zone");
    CHECK(bag_has(&tg.state.p[0].live, live), "the card that REMAINS is the valid live card");
    CHECK(bag_has(&tg.state.p[0].discard, member), "the invalid member was evicted to the waitroom");
    CHECK(!bag_has(&tg.state.p[0].live, member), "the invalid member is no longer in the live zone");
}

/* ── 8 ── THE WRITE ORDER, which is a silent failure when it is wrong.
 *      Rust `execute_performance_phase` writes `performer.live_card_zone.face_up =
 *      true` and then calls `Self::check_timing(game_state)`.  If the C wrote the
 *      flag AFTER rb_check_timing, the eviction would be deferred to the NEXT
 *      timing pass -- one phase later, on a different board -- and every assertion
 *      here would still pass because they all set the flag by hand.  This is the
 *      only test in the file that goes through the production path, and it is the
 *      one that pins the ordering. */
static void performance_phase_opens_the_gate_before_the_timing_check(void) {
static    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, CARD_LIVE);
    int member = test_new_id(&tg, CARD_MEMBER);
    int stage_member = test_new_id(&tg, CARD_MEMBER);
    if (live < 0 || member < 0 || stage_member < 0) {
        CHECK(0, "fixtures resolve (a live card, an invalid live-slot member, a stage member)");
        return;
    }
    push_live(&tg, 0, live);
    push_live(&tg, 0, member);
    tg.state.p[0].stage[0] = stage_member;
    tg.state.p[1].stage[0] = stage_member;
    fill_decks(&tg, stage_member);
    /* face_up is NOT set here on purpose: the production path must set it. */

    CHECK_EQ(tg.state.live_face_up[0], 0, "the gate is CLOSED going into the performance window");
    rb_execute_performance_phase(&tg.state, 1 /* is_first */);

    CHECK(tg.state.live_face_up[tg.state.first_attacker] == 1,
          "the performance window left the PERFORMER's gate open");
    CHECK(!bag_has(&tg.state.p[0].live, member),
          "ORDERING: the invalid member was evicted during the performance window "
          "itself, not deferred to a later timing pass");
    CHECK(bag_has(&tg.state.p[0].discard, member),
          "ORDERING: it landed in the performer's waitroom");
    int mi = movement_index_for(&tg.state, member);
    CHECK(mi >= 0, "ORDERING: the eviction recorded its live_card_zone -> waitroom movement");
    if (mi >= 0) {
        const RbBatchMovement *m = movement_at(&tg.state, mi);
        CHECK_EQ(m->source_zone, RB_ZONEID_LIVE_CARD_ZONE,
                 "ORDERING: the recorded movement's source is live_card_zone");
        CHECK_EQ(m->dest_zone, RB_ZONEID_WAITROOM,
                 "ORDERING: the recorded movement's destination is waitroom");
    }
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    member_leaves_live_zone_for_the_waitroom();
    energy_returns_to_the_energy_deck();
    mixed_zone_routes_each_card_by_its_own_type();
    each_seat_evicts_only_its_own_live_zone();
    gate_closed_leaves_the_live_zone_untouched();
    a_real_live_card_survives_the_check();
    a_valid_and_an_invalid_card_share_the_slot();
    performance_phase_opens_the_gate_before_the_timing_check();
    rb_unload();
    if (failures) {
        printf("live_zone_slot_validation: %d failures\n", failures);
        return 1;
    }
    printf("live_zone_slot_validation: all checks passed\n");
    return 0;
}