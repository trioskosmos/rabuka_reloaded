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
}

static void test_zone_strings(void)
{
    CHECK(strcmp(rb_zone_id_as_str(RB_ZONEID_DECK_TOP_OR_BOTTOM), "deck_top_or_bottom") == 0, "zone id string");
    CHECK(strcmp(rb_zone_id_as_str(RB_ZONEID_UNKNOWN), "unknown") == 0, "unknown zone id string");
    CHECK(strcmp(rb_ability_zone_to_str(RB_ABILITY_ZONE_LIVE_TOTAL), "live_total") == 0, "live_total zone string");
    CHECK(strcmp(rb_ability_zone_to_str(RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE), "success_live_zone") == 0, "success live zone string");
    CHECK(strcmp(rb_ability_zone_to_str(RB_ABILITY_ZONE_UNKNOWN), "unknown") == 0, "unknown zone string");
    CHECK(rb_ability_zone_to_str(-1) == NULL, "invalid zone string is null");
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

int main(void)
{
    test_canonical_trigger();
    test_phase_and_rps_strings();
    test_zone_strings();
    test_deployment_tracking();
    test_queue_helpers();
    if (failures) return 1;
    puts("P1 helper tests passed");
    return 0;
}
