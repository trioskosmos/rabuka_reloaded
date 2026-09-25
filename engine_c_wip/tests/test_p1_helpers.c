#include "rabuka.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } \
} while (0)

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
    if (failures) return 1;
    puts("P1 helper tests passed");
    return 0;
}
