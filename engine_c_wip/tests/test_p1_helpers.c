#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } \
} while (0)

/* The shim suite needs the card database; load it once, lazily. */
static void ensure_cards_loaded(void)
{
    static int tried;
    if (tried) return;
    tried = 1;
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: card database did not load\n");
        failures++;
    }
}

static void t_game_new(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    CHECK(tg.state.winner == -1, "test_game_new starts with no winner");
    CHECK(tg.state.turn == 1, "test_game_new starts on turn 1");
    CHECK(tg.state.phase == RB_PHASE_MAIN, "test_game_new starts in Main");
    CHECK(tg.state.p[0].hand.n == 0, "test_game_new starts with an empty hand");
    CHECK(tg.state.p[0].live.n == 0, "test_game_new starts with an empty live zone");
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK(tg.state.p[0].stage[i] == RB_EMPTY_SLOT, "test_game_new empties the stage");
}

/* REGRESSION: test_set_live_card used to write live.cards[slot] at a FIXED
   index, so the transpiler's three consecutive calls (always slot 0) put ONE
   live card in the zone -- the recorded canary
   live_cards_stuck_in_live_zone_instead_of_discard ("got 1 expected 3"). */
static void t_set_live_card_appends(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0, "live card ids resolve");
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    CHECK(test_hand_len(&tg) == 3, "three cards start in hand");

    /* The transpiler always passes slot 0. */
    test_set_live_card(&tg, 0, a);
    CHECK(tg.state.p[0].live.n == 1, "first set_live_card appends one card");
    test_set_live_card(&tg, 0, b);
    CHECK(tg.state.p[0].live.n == 2, "second set_live_card at slot 0 still appends");
    test_set_live_card(&tg, 0, c);
    CHECK(tg.state.p[0].live.n == 3, "third set_live_card at slot 0 still appends");

    /* Order is append order, not slot order. */
    int ids[8];
    CHECK(test_zone_ids(&tg, 0, "live", ids, 8) == 3, "live zone dumps three cards");
    CHECK(ids[0] == a && ids[1] == b && ids[2] == c, "live zone keeps append order");
    CHECK(test_zone_len(&tg, 0, "live") == 3, "test_zone_len agrees with live.n");
    CHECK(test_zone_len(&tg, 0, "live_card_zone") == 3, "live_card_zone alias resolves");
    CHECK(test_zone_count_of_id(&tg, 0, "live", a) == 1, "each live card appears once");
    CHECK(test_zone_has_id(&tg, 0, "live", c), "the last-appended card is present");
    CHECK(!test_zone_has_id(&tg, 0, "live", 999999), "an absent id is not found");

    /* The card leaves the hand; the census is conserved. */
    CHECK(test_hand_len(&tg) == 0, "placing live cards empties the hand");
    CHECK(test_zone_len(&tg, 0, "hand") == 0, "test_zone_len agrees with hand.n");
    CHECK(test_total_card_count(&tg) == 3, "card census is conserved across the move");

    /* A fourth placement saturates rather than overrunning the 3-card zone. */
    int d = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, d);
    test_set_live_card(&tg, 0, d);
    CHECK(tg.state.p[0].live.n == RB_MAX_LIVE_CARDS, "the live zone caps at 3 cards");
    CHECK(test_total_card_count(&tg) == 3, "a saturated drop does not create a card");
}

/* Count how many zones (across BOTH seats) hold `id`. A card that is lost by a
   placement shows 0; a card double-counted by a fixed-index overwrite (the
   defect that produced the "got 1 expected 3" canary) shows >= 2 somewhere. */
static int census_hits(TestGame *tg, int id)
{
    static const char *const zones[] = {
        "hand", "main_deck", "waitroom", "live", "success", "energy", "energy_deck"
    };
    int hits = 0;
    for (int pl = 0; pl < 2; pl++) {
        hits += test_zone_count_of_id(tg, pl, "stage", id);
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            char z[16];
            snprintf(z, sizeof(z), "under%d", a);
            hits += test_zone_count_of_id(tg, pl, z, id);
        }
        for (size_t i = 0; i < sizeof(zones) / sizeof(zones[0]); i++)
            hits += test_zone_count_of_id(tg, pl, zones[i], id);
    }
    return hits;
}

/* The seat-aware shim exists so a P2-owned live card can be staged at all:
   the old P1-hard-wired fixture put it in P1's live zone, where a P2 scan
   never sees it -- a fixture that LOOKS meaningful and measures nothing. */
static void t_set_live_card_for_seats(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0, "seat-aware live card ids resolve");

    /* P2's card goes into P2's hand, then into P2's live zone. */
    test_add_to_hand_for(&tg, 1, a);
    test_add_to_hand_for(&tg, 1, b);
    test_set_live_card_for(&tg, 1, a);
    CHECK(tg.state.p[1].live.n == 1, "a P2 card lands in P2's live zone");
    CHECK(tg.state.p[1].live.cards[0] == a, "P2's live zone holds that exact card");
    CHECK(tg.state.p[0].live.n == 0, "P1's live zone is untouched");
    CHECK(test_zone_len(&tg, 0, "live") == 0, "P1 reads as having no live card");
    CHECK(test_zone_has_id(&tg, 1, "live", a), "the P2 zone accessor finds the card");
    CHECK(!test_zone_has_id(&tg, 0, "live", a), "P1 does not see P2's live card");
    CHECK(test_zone_len(&tg, 1, "hand") == 1, "the card left P2's hand");
    CHECK(test_zone_len(&tg, 0, "hand") == 0, "P1's hand is still empty");
    CHECK(test_total_card_count(&tg) == 2, "the census is conserved");

    /* P1 and P2 keep separate live zones and separate append orders. */
    int d = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand_for(&tg, 0, d);
    test_set_live_card_for(&tg, 0, d);
    test_set_live_card_for(&tg, 1, b);
    CHECK(tg.state.p[0].live.n == 1 && tg.state.p[0].live.cards[0] == d,
          "P1's live zone is independent of P2's");
    CHECK(tg.state.p[1].live.n == 2, "P2's second placement appends");
    CHECK(tg.state.p[1].live.cards[0] == a && tg.state.p[1].live.cards[1] == b,
          "P2's live zone keeps its own append order");
    CHECK(census_hits(&tg, a) == 1 && census_hits(&tg, b) == 1 && census_hits(&tg, d) == 1,
          "every card is in exactly one zone");
    CHECK(census_hits(&tg, c) == 0, "an id that was never placed is in no zone");
    CHECK(test_total_card_count(&tg) == 3, "two seats, one census");

    /* The P1 wrapper is exactly the seat-0 form of the shim. */
    test_game_new(&tg);
    int f = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand_for(&tg, 0, f);
    test_set_live_card(&tg, 0, f);
    CHECK(tg.state.p[0].live.n == 1 && tg.state.p[0].live.cards[0] == f,
          "test_set_live_card is test_set_live_card_for(seat 0)");
    CHECK(tg.state.p[1].live.n == 0, "the wrapper still never touches P2");

    /* Out-of-range seats are rejected rather than indexed. */
    test_set_live_card_for(&tg, 2, f);
    test_set_live_card_for(&tg, -1, f);
    CHECK(test_total_card_count(&tg) == 1, "an out-of-range seat places nothing");
    /* Positional placement is seat-aware too, and leaves the other seat alone. */
    int g = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand_for(&tg, 1, g);
    test_insert_live_card_at_for(&tg, 1, 0, g);
    CHECK(tg.state.p[1].live.n == 1 && tg.state.p[1].live.cards[0] == g,
          "seat-aware positional placement lands in the named seat");
    CHECK(tg.state.p[0].live.n == 1 && tg.state.p[0].live.cards[0] == f,
          "and leaves P1's live zone alone");
    CHECK(test_total_card_count(&tg) == 2, "two seats, one card each");
}

/* The append contract, stressed past the transpiler's three-call pattern and
   across both seats: zone length, append order, and a full card census so no
   card is lost or double-counted. */
static void t_live_append_census(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int ids[3][3];
    int total = 0;
    for (int pl = 0; pl < 2; pl++) {
        for (int i = 0; i < 3; i++) {
            ids[pl][i] = test_new_id(&tg, "PL!-sd1-019-SD");
            CHECK(ids[pl][i] >= 0, "bulk live card ids resolve");
            test_add_to_hand_for(&tg, pl, ids[pl][i]);
            total++;
        }
    }
    CHECK(test_total_card_count(&tg) == 6, "six cards staged in two hands");
    CHECK(test_zone_len(&tg, 0, "hand") == 3 && test_zone_len(&tg, 1, "hand") == 3,
          "both hands hold three");

    /* Interleave the seats, always through the slot-0 call the transpiler
       emits, so a fixed-index write would overwrite across all six. */
    for (int i = 0; i < 3; i++) {
        test_set_live_card_for(&tg, 0, ids[0][i]);
        test_set_live_card_for(&tg, 1, ids[1][i]);
    }
    CHECK(test_zone_len(&tg, 0, "live") == 3, "P1 appended three live cards");
    CHECK(test_zone_len(&tg, 1, "live") == 3, "P2 appended three live cards");
    for (int pl = 0; pl < 2; pl++) {
        for (int i = 0; i < 3; i++)
            CHECK(tg.state.p[pl].live.cards[i] == ids[pl][i],
                  "each seat's live zone keeps its own append order");
    }
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 3; i++)
            CHECK(census_hits(&tg, ids[pl][i]) == 1,
                  "an appended live card is in exactly one zone");
    CHECK(test_total_card_count(&tg) == total, "the census is unchanged by six appends");
    CHECK(test_zone_len(&tg, 0, "hand") == 0 && test_zone_len(&tg, 1, "hand") == 0,
          "both hands are empty afterwards");

    /* A fourth card for a seat is refused: the live zone holds at most 3, and
       the shim DROPS it (the hand removal happens first, exactly as Rust's
       set_live_card moves the card then pushes). The census must therefore
       stay at 6 -- the drop must not duplicate anything, and the refusal must
       not disturb the three cards already in the zone. */
    int extra = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand_for(&tg, 0, extra);
    test_set_live_card_for(&tg, 0, extra);
    CHECK(test_zone_len(&tg, 0, "live") == 3, "the live zone stays at three cards");
    CHECK(test_zone_len(&tg, 1, "live") == 3, "a refused placement leaves the other seat alone");
    CHECK(test_zone_has_id(&tg, 0, "live", ids[0][2]),
          "the third live card is not overwritten by the refused fourth");
    CHECK(test_total_card_count(&tg) == total,
          "a saturated placement neither duplicates nor drops a card");
}

/* The positional variant stays positional, for the rare test that needs it.
   It is a real ordered insert: slot == n appends, slot < n shifts the tail
   right (dropping the last card when the 3-card zone is full), and slot > n is
   refused because a hole is not a state the engine can produce. The old
   expectations described a different shim and were red. */
static void t_insert_live_card_at(void){
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    int d = test_new_id(&tg, "PL!-sd1-019-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0 && d >= 0, "positional live card ids resolve");

    test_insert_live_card_at(&tg, 0, a);
    CHECK(tg.state.p[0].live.n == 1, "insert at slot 0 fills the empty zone");
    CHECK(tg.state.p[0].live.cards[0] == a, "the card lands at slot 0");
    test_insert_live_card_at(&tg, 1, b);
    CHECK(tg.state.p[0].live.n == 2, "insert at the tail appends");
    CHECK(tg.state.p[0].live.cards[1] == b, "the card lands at slot 1");
    test_insert_live_card_at(&tg, 0, c);
    CHECK(tg.state.p[0].live.n == 3, "inserting at slot 0 shifts the tail right");
    CHECK(tg.state.p[0].live.cards[0] == c, "slot 0 now holds the new card");
    CHECK(tg.state.p[0].live.cards[1] == a, "the displaced card shifted right");
    CHECK(tg.state.p[0].live.cards[2] == b, "and so did the one behind it");
    CHECK(census_hits(&tg, a) == 1 && census_hits(&tg, b) == 1 && census_hits(&tg, c) == 1,
          "an ordered insert duplicates nothing");
    CHECK(test_total_card_count(&tg) == 3, "the census is conserved by a full zone");

    /* Full zone: the tail card falls off rather than being overwritten. The old
       revision clamped to CAP-1 and wrote cards[slot] over a live card, so the
       card at that slot silently vanished. */
    test_insert_live_card_at(&tg, 2, d);
    CHECK(tg.state.p[0].live.n == 3, "a full zone does not grow past its capacity");
    CHECK(tg.state.p[0].live.cards[0] == c, "the cards before the insert are unmoved");
    CHECK(tg.state.p[0].live.cards[1] == a, "the card before the slot is unmoved");
    CHECK(tg.state.p[0].live.cards[2] == d, "the new card takes the requested slot");
    CHECK(census_hits(&tg, d) == 1, "the inserted card is present once");
    CHECK(census_hits(&tg, a) == 1 && census_hits(&tg, c) == 1, "and nothing else was duplicated");
    CHECK(census_hits(&tg, b) == 0, "the tail card fell off instead of being clobbered in place");
    CHECK(test_total_card_count(&tg) == 3, "the eviction keeps the census size");

    /* A slot past the end of the zone would leave a hole, so it is refused
       rather than filled with whatever the bag happened to hold (0 -- a real
       card index -- on a fresh TestGame). */
    test_game_new(&tg);
    int e = test_new_id(&tg, "PL!-sd1-019-SD");
    int f = test_new_id(&tg, "PL!-sd1-019-SD");
    int g = test_new_id(&tg, "PL!-sd1-019-SD");
    test_insert_live_card_at(&tg, 2, e);
    CHECK(tg.state.p[0].live.n == 0, "a slot past the end of the zone is refused");
    CHECK(test_total_card_count(&tg) == 0, "and it fabricates no card");
    /* The same slot becomes legal once the zone is long enough to reach it. */
    test_insert_live_card_at(&tg, 0, e);
    test_insert_live_card_at(&tg, 1, f);
    test_insert_live_card_at(&tg, 2, g);
    CHECK(tg.state.p[0].live.n == 3, "a slot at the tail is accepted once reachable");
    CHECK(tg.state.p[0].live.cards[0] == e && tg.state.p[0].live.cards[1] == f &&
          tg.state.p[0].live.cards[2] == g, "and holds the cards in insert order");
    CHECK(test_zone_len(&tg, 0, "live") == 3, "the zone readers see all three cards");
    CHECK(census_hits(&tg, e) == 1 && census_hits(&tg, f) == 1 && census_hits(&tg, g) == 1,
          "each card is counted once");
    CHECK(test_total_card_count(&tg) == 3, "and the census is exact");

    test_insert_live_card_at(&tg, 9, d);
    CHECK(tg.state.p[0].live.n == 3, "an out-of-range slot is rejected");
    test_insert_live_card_at(&tg, -1, d);
    CHECK(tg.state.p[0].live.n == 3, "a negative slot is rejected");
}

/* The queue-entry seat accessors. Their reason to exist is DEFECT 3: a live
   start entry pushed with an EMPTY player_id (src/turn/triggers.c pushes via
   rb_queue_push_with_trigger, which never stamps the owner, so
   ability_queue.c's default `memcpy` of choice_player_id copies zero bytes).
   A test can only catch that if "unstamped" is a value it can ASK about, which
   is why the seat accessors answer -1 rather than collapsing to seat 0. */
static void t_queue_entry_seat_accessor(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    CHECK(test_queue_n_entries(&tg) == 0, "a fresh game has an empty queue");
    CHECK(test_queue_entry_seat(&tg, 0) == -1, "no entry means no seat, not seat 0");
    CHECK(test_queue_entry_choice_seat(&tg, 0) == -1, "and no choice seat either");

    char buf[TEST_SEAT_ID_LEN];
    memset(buf, 'Z', sizeof(buf));
    CHECK(test_queue_entry_player_id(&tg, 0, buf, sizeof(buf)) == 0,
          "reading past the end of the queue fails");
    CHECK(buf[0] == '\0', "a failed read clears the buffer instead of leaving garbage");
    CHECK(test_queue_entry_choice_player_id(&tg, -1, buf, sizeof(buf)) == 0,
          "a negative index is rejected");
    CHECK(test_queue_entry_player_id(&tg, 0, NULL, 0) == 0, "a null buffer is rejected");

    /* Stamp two entries the way rb_queue_enqueue does, plus the long form that
       Rust's build_ability_queue_entry normalises (abilities.rs:240-246). */
    RbQueueEntry *e = &tg.state.queue.entries[0];
    memset(e, 0, sizeof(*e));
    e->card_id = 5;
    strcpy(e->player_id, "p2");
    tg.state.queue.n_entries = 1;
    RbQueueEntry *e2 = &tg.state.queue.entries[1];
    memset(e2, 0, sizeof(*e2));
    e2->card_id = 6;
    strcpy(e2->player_id, "player1");
    tg.state.queue.n_entries = 2;

    CHECK(test_queue_n_entries(&tg) == 2, "both stamped entries are visible");
    CHECK(test_queue_entry_seat(&tg, 0) == 1, "entry 0 is owned by p2");
    CHECK(test_queue_entry_owned_by_seat(&tg, 0, 1), "the p2 predicate agrees");
    CHECK(!test_queue_entry_owned_by_seat(&tg, 0, 0), "and rejects p1");
    CHECK(test_queue_entry_seat(&tg, 1) == 0, "the long form normalises to p1");
    CHECK(test_queue_entry_owned_by_seat(&tg, 1, 0), "the p1 predicate agrees on the long form");
    CHECK(test_queue_entry_seat(&tg, 2) == -1, "an index past n_entries is -1");
    CHECK(!test_queue_entry_owned_by_seat(&tg, 0, 2), "an out-of-range seat is never owned");
    CHECK(!test_queue_entry_owned_by_seat(&tg, 0, -1), "a negative seat is never owned");

    CHECK(test_queue_entry_player_id(&tg, 0, buf, sizeof(buf)) == 1, "the raw owner reads back");
    CHECK(strcmp(buf, "p2") == 0, "and it is the short canonical token");
    CHECK(test_queue_entry_player_id(&tg, 1, buf, sizeof(buf)) == 1, "the long form reads back");
    CHECK(strcmp(buf, "player1") == 0, "the raw reader does not normalise");
    CHECK(strcmp(test_seat_id(1), "p2") == 0, "test_seat_id names p2");
    CHECK(strcmp(test_seat_id(0), "p1") == 0, "test_seat_id names p1");
    CHECK(strcmp(test_seat_id(7), "") == 0, "an out-of-range seat has no token");

    /* choice_player_id routes the PAUSE, not the entry: rb_queue_pause_for_choice
       defaults it from the entry's owner (ability_queue.c:296-304). */
    CHECK(test_queue_entry_choice_seat(&tg, 0) == -1,
          "an unstamped choice_player_id is -1, not p1");
    CHECK(test_queue_entry_choice_player_id(&tg, 0, buf, sizeof(buf)) == 1,
          "the raw choice token reads back");
    CHECK(strcmp(buf, "") == 0, "and it really is the empty string");
    CHECK(!test_queue_entry_choice_owned_by_seat(&tg, 0, 0),
          "an empty choice token is owned by neither seat");
    CHECK(!test_queue_entry_choice_owned_by_seat(&tg, 0, 1),
          "an empty choice token is owned by neither seat (p2)");
    tg.state.queue.entries[0].choice_player_id[0] = 'p';
    tg.state.queue.entries[0].choice_player_id[1] = '2';
    tg.state.queue.entries[0].choice_player_id[2] = '\0';
    CHECK(test_queue_entry_choice_seat(&tg, 0) == 1, "a stamped choice token reads as p2");
    CHECK(test_queue_entry_choice_owned_by_seat(&tg, 0, 1), "the choice p2 predicate agrees");
    CHECK(!test_queue_entry_choice_owned_by_seat(&tg, 0, 0), "and rejects p1");
    /* The entry owner and the choice owner are independent: an entry owned by
       p2 may legitimately route its prompt to p1 (the G1/G3 opponent-routing
       fallback in ability_queue.c). */
    CHECK(test_queue_entry_seat(&tg, 0) == 1, "the entry owner is p2");
    strcpy(tg.state.queue.entries[0].choice_player_id, "p1");
    CHECK(test_queue_entry_choice_seat(&tg, 0) == 0, "the choice owner can differ from the entry owner");
    CHECK(test_queue_entry_seat(&tg, 0) == 1, "without disturbing the entry owner");

    /* Truncation must still NUL-terminate. */
    strcpy(e->player_id, "player2");
    CHECK(test_queue_entry_seat(&tg, 0) == 1, "the long form still normalises to p2");
    char small[4];
    memset(small, 'Z', sizeof(small));
    CHECK(test_queue_entry_player_id(&tg, 0, small, sizeof(small)) == 1, "a short buffer still reads");
    CHECK(strcmp(small, "pla") == 0, "a short buffer holds the truncated prefix");
    CHECK(small[3] == '\0', "a truncated read is NUL-terminated");
}

/* rb_max_distinct_names is defined in src/ability/util.c but absent from
   include/rabuka.h, so a test wanting Rust's max_distinct_names had to fall
   back to the declared sibling and measure something else. */
static void t_max_distinct_names_shim(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_id(&tg, "LL-E-001-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0, "distinct-name card ids resolve");
    int one[1] = { a };
    int two[2] = { a, b };
    int mixed[3] = { a, b, c };
    CHECK(test_max_distinct_names(NULL, 0) == 0, "a null list counts nothing");
    CHECK(test_max_distinct_names(one, 0) == 0, "an empty list counts nothing");
    CHECK(test_max_distinct_names(one, 1) == 1, "one card has one name");
    CHECK(test_max_distinct_names(two, 2) == 1, "two copies of one name are not two names");
    CHECK(test_max_distinct_names(mixed, 3) == 2, "two names among three cards");
}

/* REGRESSION: test_select_indices used to keep only indices[0] and treat n == 0
   as a total no-op, so Rust's select_indices(&[]) -- the ubiquitous "decline"
   -- left the prompt pending and a multi-index pick could never complete. */
static void t_select_indices_shapes(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    CHECK(test_pending_choice_count(&tg) == 0, "a fresh game has no pending choice");
    /* With nothing pending, any answer is a no-op rather than a crash. */
    int pick[2] = { 0, 1 };
    test_select_indices(&tg, pick, 2);
    test_select_indices(&tg, NULL, 0);
    CHECK(!test_has_pending_choice(&tg), "answering with no prompt is safe");

    /* Stage a real prompt: an optional look-and-select leaves a SelectCard
       pending, and declining it must actually clear the prompt. */
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_CARD;
    ch.zone[0] = 'h'; ch.zone[1] = 'a'; ch.zone[2] = 'n'; ch.zone[3] = 'd'; ch.zone[4] = '\0';
    ch.count = 1;
    ch.allow_skip = 1;
    rb_queue_set_pending_choice(&tg.state, &ch);
    CHECK(test_has_pending_choice(&tg), "a staged prompt is visible to the shims");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0, "choice kind is reported");
    CHECK(test_pending_choice_count(&tg) == 1, "pending_choice_count is 1 while a prompt stands");
    test_select_indices(&tg, NULL, 0);
    CHECK(!test_has_pending_choice(&tg), "n == 0 declines: the prompt is consumed");

    rb_queue_set_pending_choice(&tg.state, &ch);
    CHECK(test_has_pending_choice(&tg), "a second prompt can be staged");
    /* A hand prompt needs a hand to pick from: the old fixture staged the
       choice over an empty hand, so the single-index answer had nothing to
       resolve and the prompt stood. */
    int pickable = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, pickable);
    int one[1] = { 0 };
    test_select_indices(&tg, one, 1);
    CHECK(!test_has_pending_choice(&tg), "a single-index answer is delivered");
}

/* REGRESSION: zone_bag indexed p[pl] with no range check, so test_zone_has_id /
   test_zone_has_card_no read out of bounds for any pl outside 0..1. */
static void t_zone_helpers_validate_seat(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, a);
    CHECK(test_zone_has_id(&tg, 0, "hand", a), "seat 0 sees its own hand");
    CHECK(test_zone_has_id(&tg, 1, "hand", a) == 0, "seat 1's hand is empty, so the answer is no");
    /* Out-of-range seats must answer "not found", not read p[] out of bounds. */
    CHECK(test_zone_has_id(&tg, -1, "hand", a) == 0, "a negative seat is rejected");
    CHECK(test_zone_has_id(&tg, 2, "hand", a) == 0, "seat 2 is rejected");
    CHECK(test_zone_len(&tg, 7, "hand") == 0, "test_zone_len rejects an out-of-range seat");
    CHECK(test_zone_ids(&tg, 7, "hand", NULL, 0) == 0, "test_zone_ids rejects a null buffer");
    CHECK(test_zone_has_id(&tg, 0, "no_such_zone", a) == 0, "an unknown zone is not found");
    CHECK(test_zone_len(&tg, 0, "no_such_zone") == 0, "an unknown zone has length 0");
    CHECK(test_stage_has(&tg, RB_STAGE_SIZE, a) == 0, "an out-of-range stage area is rejected");
    CHECK(test_stage_has(&tg, -1, a) == 0, "a negative stage area is rejected");
    CHECK(rb_card_no_eq(-1, "PL!-sd1-019-SD") == 0, "a negative card id matches nothing");
    CHECK(rb_card_no_eq(a, NULL) == 0, "a null card number matches nothing");
}

/* Stage slots are positional (Rust stage.set_area); the under-card bags append. */
static void t_stage_and_under_placement(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_stage(&tg, 2, a);
    test_add_to_stage(&tg, 0, b);
    CHECK(test_stage_has(&tg, 2, a), "a member placed at area 2 stays at area 2");
    CHECK(test_zone_len(&tg, 0, "stage") == 2, "the stage counts occupied slots only");
    CHECK(test_stage_has(&tg, 0, a) == 0, "a member is not also at area 0");
    test_place_under(&tg, 0, 2, c);
    test_place_under(&tg, 0, 2, c);
    CHECK(test_zone_len(&tg, 0, "under2") == 2, "under-cards append, they do not overwrite");
    CHECK(test_zone_count_of_id(&tg, 0, "under2", c) == 2, "both tucked cards are counted");
    CHECK(test_zone_len(&tg, 0, "under0") == 0, "an empty under-card bag reads 0");
    CHECK(test_total_card_count(&tg) == 4, "stage and under-cards both count toward the census");
}

/* REGRESSION: the energy helpers bumped energy_active even when the card did
   not land, breaking the energy_active <= energy.n invariant that
   rb_energy_activate_all in zones.c depends on. */
static void t_energy_active_tracks_the_bag(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    test_give_energy(&tg, 3);
    CHECK(tg.state.p[0].energy.n == 3, "give_energy adds three energy cards");
    CHECK(tg.state.p[0].energy_active == 3, "give_energy makes them all active");
    CHECK(tg.state.p[0].energy_active <= tg.state.p[0].energy.n,
          "active energy never exceeds the energy bag");
    test_give_energy_for(&tg, 1, 2);
    CHECK(tg.state.p[1].energy.n == 2, "give_energy_for targets the requested seat");
    CHECK(tg.state.p[0].energy.n == 3, "give_energy_for leaves the other seat alone");
    test_give_opp_energy(&tg, 1);
    CHECK(tg.state.p[1].energy.n == 3, "give_opp_energy is give_energy_for(1)");
    CHECK(tg.state.p[1].energy_active == 3, "opponent energy is active too");
    CHECK(test_zone_len(&tg, 1, "energy_zone") == 3, "the energy zone reports its length");

    test_add_to_energy(&tg, 0, test_id(&tg, "LL-E-001-SD"));
    CHECK(tg.state.p[0].energy.n == 4, "add_to_energy appends");
    CHECK(tg.state.p[0].energy_active == 4, "add_to_energy activates the new card");
    CHECK(tg.state.p[0].energy_active <= tg.state.p[0].energy.n,
          "the invariant survives add_to_energy");

    test_set_energy_active(&tg, 0, 1);
    CHECK(tg.state.p[0].energy_active == 1, "set_energy_active is a plain setter");
    test_spend_energy(&tg, 5);
    CHECK(tg.state.p[0].energy_active == 0, "spend_energy saturates at zero");

    /* Saturating a full bag must not invent active energy. */
    test_game_new(&tg);
    test_give_energy(&tg, RB_MAX_ZONE + 5);
    CHECK(tg.state.p[0].energy.n == RB_MAX_ZONE, "the energy bag saturates");
    CHECK(tg.state.p[0].energy_active == tg.state.p[0].energy.n,
          "a saturated give_energy leaves active == bag");
}

/* The test-side stand-in for the hand-selection cost gap: move the picks the
   engine records but never routes, and prove the census stays conserved. */
static void t_cost_selection_zone_move(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    int d = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    /* `d` lives ONLY in the live zone. The old fixture used `c` for both, and
       then re-picked `b` after `b` had already been moved -- so the "two named
       picks" it asserted could only ever move one card. */
    test_add_to_live_for(&tg, 0, d);
    CHECK(test_hand_len(&tg) == 3, "three cost candidates in hand");
    CHECK(test_zone_len(&tg, 0, "live") == 1, "one card staged in the live zone");
    CHECK(test_total_card_count(&tg) == 4, "four distinct cards are placed");

    /* Move the first two hand cards, as a "discard 2 from hand" cost would. */
    CHECK(test_move_hand_to_waitroom(&tg, 0, 2) == 2, "two cost cards are moved");
    CHECK(test_hand_len(&tg) == 1, "the hand lost the two cost cards");
    CHECK(test_zone_len(&tg, 0, "waitroom") == 2, "the waitroom gained them");
    CHECK(test_zone_has_id(&tg, 0, "waitroom", a), "the first cost card is in the waitroom");
    CHECK(test_zone_has_id(&tg, 0, "discard", b), "discard and waitroom are the same zone");
    CHECK(test_total_card_count(&tg) == 4, "the cost move conserves the census");

    /* Named picks, one drawn from hand and one from the live zone. */
    int picks[2];
    picks[0] = c;
    picks[1] = d;
    CHECK(test_move_ids_to_waitroom(&tg, 0, picks, 2) == 2, "both named picks are moved");
    CHECK(test_zone_len(&tg, 0, "waitroom") == 4, "the waitroom now holds four cards");
    CHECK(test_zone_len(&tg, 0, "live") == 0, "the live pick left the live zone");
    CHECK(test_hand_len(&tg) == 0, "the hand pick left the hand");
    CHECK(census_hits(&tg, a) == 1 && census_hits(&tg, b) == 1 &&
          census_hits(&tg, c) == 1 && census_hits(&tg, d) == 1,
          "each cost card ended up in exactly one zone");
    CHECK(test_total_card_count(&tg) == 4, "named picks conserve the census too");

    /* A pick that is nowhere to be found is reported, not invented. */
    CHECK(test_move_ids_to_waitroom(&tg, 0, picks, 2) == 0, "an already-moved pick moves nothing");
    CHECK(test_move_ids_to_waitroom(&tg, 0, picks, 0) == 0, "an empty pick list moves nothing");
    CHECK(test_move_ids_to_waitroom(&tg, 0, NULL, 2) == 0, "a null pick list moves nothing");
    CHECK(test_move_hand_to_waitroom(&tg, 0, 9) == 0, "an empty hand moves nothing");
    CHECK(test_move_hand_to_waitroom(&tg, 5, 1) == 0, "an out-of-range seat moves nothing");
    /* Requesting more than the hand holds moves exactly what is there. */
    int e = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, e);
    CHECK(test_move_hand_to_waitroom(&tg, 0, 9) == 1, "moving more than the hand holds is capped");
    CHECK(test_hand_len(&tg) == 0, "and it really did empty the hand");
}

/* Deck top insertion is positional; the deck itself appends. */
static void t_deck_placement(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    CHECK(test_deck_len(&tg) == 2, "add_to_deck appends");
    test_insert_deck_top(&tg, 0, c);
    CHECK(test_deck_len(&tg) == 3, "insert_deck_top grows the deck");
    CHECK(tg.state.p[0].deck.cards[0] == c, "the inserted card is on top");
    CHECK(tg.state.p[0].deck.cards[1] == a, "the old top shifted down");
    CHECK(tg.state.p[0].deck.cards[2] == b, "the deck tail is preserved");
    CHECK(test_zone_len(&tg, 0, "main_deck") == 3, "main_deck aliases the deck bag");
    CHECK(test_total_card_count(&tg) == 3, "deck placement conserves the census");
}

static void test_testgame_shims(void)
{
    t_game_new();
    t_set_live_card_appends();
    t_set_live_card_for_seats();
    t_live_append_census();
    t_insert_live_card_at();
    t_queue_entry_seat_accessor();
    t_max_distinct_names_shim();
    t_select_indices_shapes();
    t_zone_helpers_validate_seat();
    t_stage_and_under_placement();
    t_energy_active_tracks_the_bag();
    t_cost_selection_zone_move();
    t_deck_placement();
}

static void test_canonical_trigger(void)
{
    CHECK(strcmp(rb_canonical_trigger(NULL), "unknown") == 0, "null trigger is unknown");
    CHECK(strcmp(rb_canonical_trigger("prefix 起動 suffix"), "activation") == 0, "activation canonicalization");
    CHECK(strcmp(rb_canonical_trigger("Debut"), "debut") == 0, "English debut canonicalization");
    CHECK(strcmp(rb_canonical_trigger("ライブ開始時, 登場"), "debut") == 0, "debut has Rust priority");
    CHECK(strcmp(rb_canonical_trigger("live_success"), "live_success") == 0, "English live success canonicalization");
    CHECK(strcmp(rb_canonical_trigger("sometimes"), "unknown") == 0, "unknown trigger canonicalization");
}

static void test_phase_and_rps_strings(void)
{
    CHECK(strcmp(rb_phase_name(RB_PHASE_OPENING), "Opening") == 0, "opening phase string");
    CHECK(strcmp(rb_phase_name(RB_PHASE_LIVE_SET), "LiveCardSet") == 0, "live set phase string");
    CHECK(strcmp(rb_phase_name(999), "Unknown") == 0, "invalid phase string");
    CHECK(strcmp(rb_rps_choice_name(0), "グー") == 0, "rock choice string");
    CHECK(strcmp(rb_rps_choice_name(1), "パー") == 0, "paper choice string");
    CHECK(strcmp(rb_rps_choice_name(2), "チョキ") == 0, "scissors choice string");
    CHECK(strcmp(rb_rps_choice_name(3), "?") == 0, "invalid RPS choice string");
    GameState g;
    memset(&g, 0, sizeof(g));
    g.player1_rps_choice = 0;
    g.player2_rps_choice = 2;
    CHECK(rb_resolve_rps_if_both_chosen(&g) == 1, "RPS choices resolve");
    CHECK(g.rps_winner == 1, "rock beats scissors");
}

static void test_zone_strings(void)
{
    CHECK(strcmp(rb_zone_id_as_str(RB_ZONEID_DECK_TOP_OR_BOTTOM), "deck_top_or_bottom") == 0, "zone id string");
    CHECK(strcmp(rb_zone_id_as_str(RB_ZONEID_UNKNOWN), "unknown") == 0, "unknown zone id string");
    CHECK(strcmp(rb_ability_zone_to_str(RB_ABILITY_ZONE_LIVE_TOTAL), "live_total") == 0, "live_total zone string");
    CHECK(strcmp(rb_ability_zone_to_str(RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE), "success_live_zone") == 0, "success live zone string");
    CHECK(strcmp(rb_ability_zone_to_str(RB_ABILITY_ZONE_UNKNOWN), "unknown") == 0, "unknown zone string");
    CHECK(rb_ability_zone_to_str(-1) == NULL, "invalid zone string is null");
    CHECK(strcmp(rb_trigger_zone_id(0), "live_card_zone") == 0, "live trigger zone id");
    CHECK(strcmp(rb_trigger_zone_id(1), "stage") == 0, "stage trigger zone id");
    CHECK(strcmp(rb_trigger_zone_label(0), "ライブ置場") == 0, "live trigger zone label");
    CHECK(strcmp(rb_trigger_zone_label(1), "ステージ") == 0, "stage trigger zone label");
}

static void test_queue_helpers(void)
{
    GameState g;
    memset(&g, 0, sizeof(g));
    g.queue.state = RB_QUEUE_IDLE;
    g.queue.cur = 0;
    g.queue.n_entries = 0;

    CHECK(!rb_queue_has_resolver(&g), "idle queue has no resolver");
    g.queue.n_entries = 1;
    g.queue.state = RB_QUEUE_PAYING_COST;
    CHECK(!rb_queue_has_resolver(&g), "fresh entry has no resolver");
    rb_queue_set_resolver(&g);
    CHECK(rb_queue_has_resolver(&g), "attached resolver is visible");
    CHECK(rb_queue_take_resolver(&g) == 1, "attached resolver can be taken");
    CHECK(!rb_queue_has_resolver(&g), "taken resolver is detached");
    CHECK(rb_queue_take_resolver(&g) == 0, "resolver cannot be taken twice");

    g.queue.state = RB_QUEUE_PAYING_COST;
    g.queue.entries[0].player_id[0] = 'p';
    g.queue.entries[0].player_id[1] = '1';
    g.queue.entries[0].player_id[2] = '\0';
    strcpy(g.queue.entries[0].spawn_target, "opponent");
    RbChoice choice;
    memset(&choice, 0, sizeof(choice));
    choice.kind = RB_CHOICE_SELECT_CARD;
    strcpy(choice.target, "target_player_id:opponent");
    rb_queue_pause_for_choice(&g, &choice);
    CHECK(strcmp(g.queue.entries[0].choice_player_id, "p2") == 0, "opponent spawn routes choice to opponent");
}

static void test_deployment_tracking(void)
{
    GameState g;
    memset(&g, 0, sizeof(g));
    g.p[0].stage[1] = 42;
    CHECK(rb_zone_track_deployment(&g, 0, 42) == 1, "deployed stage card is tracked");
    CHECK(g.stage_arrived[0][1] == 1, "deployment marks its stage area");
    CHECK(rb_zone_track_deployment(&g, 0, 42) == 1, "deployment tracking is idempotent");
    CHECK(rb_zone_track_deployment(&g, 0, 99) == 0, "non-stage card is not tracked");
}

static void test_backtracking(void)
{
    int pool[8] = {0, 1, 2, 0, 0, 0, 0, 1};
    int needs[16] = {
        0, 2, 0, 0, 0, 0, 0, 0,
        0, 0, 2, 0, 0, 0, 0, 0
    };
    int allocs[16];
    memset(allocs, 0, sizeof(allocs));
    CHECK(rb_backtrack_allocate(pool, needs, 2, allocs, 16) == 1,
          "backtracking reserves surplus for a later live");
    int has_first_wildcard = 0;
    int has_first_color02 = 0;
    int has_second_color02 = 0;
    for (int i = 0; i < 16; i++) {
        if (allocs[i] == 1) has_first_wildcard = 1;
        if (allocs[i] == 2) has_first_color02 = 1;
        if (allocs[i] == 10) has_second_color02 = 1;
    }
    CHECK(has_first_wildcard, "first live uses icon-all for its red deficit");
    CHECK(!has_first_color02, "first live preserves color02 for the second live");
    CHECK(has_second_color02, "second live receives both color02 hearts");
}

static void test_deferred_reyell(void)
{
    GameState g;
    memset(&g, 0, sizeof(g));
    g.turn = 3;
    g.re_yell_pending = 1;
    g.re_yell_owner = 0;
    g.re_yell_occurred = 1;
    g.re_yell_blade_hearts[0] = 3;
    g.re_yell_note_icons = 2;
    g.revealed_cards[0] = 101;
    g.revealed_cards[1] = 102;
    g.n_revealed = 2;
    g.snapshots[0].turn = 3;
    g.snapshots[0].player = 0;
    g.snapshots[0].n_lives = 1;
    g.snapshots[0].yell_cards[0] = 999;
    g.snapshots[0].n_yell_cards = 1;
    g.n_snapshots = 1;
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.p[0].stage[i] = RB_EMPTY_SLOT;

    rb_apply_deferred_reyell(&g);
    CHECK(!g.re_yell_pending, "deferred re-yell is consumed");
    CHECK(!g.re_yell_occurred, "deferred re-yell occurrence is cleared");
    CHECK(g.snapshots[0].n_yell_cards == 2, "re-yell replaces snapshot yell cards");
    CHECK(g.snapshots[0].yell_cards[0] == 101, "first re-yell card is retained");
    CHECK(g.snapshots[0].yell_cards[1] == 102, "second re-yell card is retained");
    CHECK(g.snapshots[0].note_icons == 2, "snapshot note icons are rebuilt");
    CHECK(g.snapshots[0].total_hearts[0] == 3, "re-yell hearts rebuild snapshot total");
}

int main(void)
{
    test_canonical_trigger();
    test_phase_and_rps_strings();
    test_zone_strings();
    test_deployment_tracking();
    test_queue_helpers();
    test_backtracking();
    test_deferred_reyell();
    test_testgame_shims();
    if (failures) return 1;
    puts("P1 helper tests passed");
    return 0;
}
