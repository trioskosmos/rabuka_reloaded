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

/* The positional variant stays positional, for the rare test that needs it. */
static void t_insert_live_card_at(void)
{
    TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-sd1-019-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    int c = test_new_id(&tg, "PL!-sd1-019-SD");
    test_insert_live_card_at(&tg, 1, a);
    CHECK(tg.state.p[0].live.n == 2, "insert at slot 1 grows the zone to cover it");
    CHECK(tg.state.p[0].live.cards[1] == a, "the card lands at slot 1");
    test_insert_live_card_at(&tg, 0, b);
    CHECK(tg.state.p[0].live.n == 2, "insert into a gap does not grow the zone");
    CHECK(tg.state.p[0].live.cards[0] == b, "slot 0 now holds the new card");
    CHECK(tg.state.p[0].live.cards[1] == a, "the displaced card shifted right");
    test_insert_live_card_at(&tg, 9, c);
    CHECK(tg.state.p[0].live.n == 2, "an out-of-range slot is rejected");
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
    CHECK(test_zone_has_id(&tg, 1, "hand", a), "seat 1's hand is empty, so the answer is no");
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
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    test_add_to_live(&tg, c);
    CHECK(test_hand_len(&tg) == 3, "three cost candidates in hand");
    CHECK(test_zone_len(&tg, 0, "live") == 1, "one card staged in the live zone");

    /* Move the first two hand cards, as a "discard 2 from hand" cost would. */
    CHECK(test_move_hand_to_waitroom(&tg, 0, 2) == 2, "two cost cards are moved");
    CHECK(test_hand_len(&tg) == 1, "the hand lost the two cost cards");
    CHECK(test_zone_len(&tg, 0, "waitroom") == 2, "the waitroom gained them");
    CHECK(test_zone_has_id(&tg, 0, "waitroom", a), "the first cost card is in the waitroom");
    CHECK(test_zone_has_id(&tg, 0, "discard", b), "discard and waitroom are the same zone");
    CHECK(test_total_card_count(&tg) == 4, "the cost move conserves the census");

    /* Named picks, drawn from hand or from the live zone. */
    int picks[2];
    picks[0] = b;
    picks[1] = c;
    CHECK(test_move_ids_to_waitroom(&tg, 0, picks, 2) == 2, "both named picks are moved");
    CHECK(test_zone_len(&tg, 0, "waitroom") == 3, "the waitroom now holds three cards");
    CHECK(test_zone_len(&tg, 0, "live") == 0, "the live pick left the live zone");
    CHECK(test_total_card_count(&tg) == 4, "named picks conserve the census too");

    /* A pick that is nowhere to be found is reported, not invented. */
    CHECK(test_move_ids_to_waitroom(&tg, 0, picks, 2) == 0, "an already-moved pick moves nothing");
    CHECK(test_move_hand_to_waitroom(&tg, 0, 9) == 1, "moving more than the hand holds is capped");
    CHECK(test_move_hand_to_waitroom(&tg, 5, 1) == 0, "an out-of-range seat moves nothing");
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
    t_insert_live_card_at();
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
