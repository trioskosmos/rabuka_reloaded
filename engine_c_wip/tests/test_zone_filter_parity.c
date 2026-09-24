#include "rabuka.h"
#include "test_game.h"
#include "deck_parser.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void zone_round_trip(void)
{
    static const RbAbilityZone zones[] = {
        RB_ABILITY_ZONE_STAGE, RB_ABILITY_ZONE_HAND, RB_ABILITY_ZONE_DECK,
        RB_ABILITY_ZONE_DISCARD, RB_ABILITY_ZONE_ENERGY,
        RB_ABILITY_ZONE_LIVE_CARD_ZONE, RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE
    };
    for (size_t i = 0; i < sizeof(zones) / sizeof(zones[0]); i++) {
        RbZoneId id = rb_zone_id_from_ability_zone(zones[i]);
        RbAbilityZone round_trip;
        CHECK(id != RB_ZONEID_UNKNOWN, "known ability zone maps to a zone id");
        CHECK_EQ(rb_zone_id_to_ability_zone(id, &round_trip), 0,
                 "zone id maps back to ability zone");
        CHECK_EQ(round_trip, zones[i], "ability zone round trip preserves zone");
    }
}

static void zone_aliases(void)
{
    CHECK_EQ(rb_zone_id_from_str("energy"), RB_ZONEID_ENERGY, "energy alias resolves");
    CHECK_EQ(rb_zone_id_from_str("energy_zone"), RB_ZONEID_ENERGY, "energy_zone alias resolves");
    CHECK_EQ(rb_zone_id_from_str("discard"), RB_ZONEID_DISCARD, "discard resolves");
    CHECK_EQ(rb_zone_id_from_str("waitroom"), RB_ZONEID_WAITROOM, "waitroom resolves");
    CHECK(rb_zone_equivalent(rb_zone_id_from_str("discard"), rb_zone_id_from_str("waitroom")),
          "discard and waitroom are equivalent");
    CHECK(rb_zone_equivalent(rb_zone_id_from_str("energy"), rb_zone_id_from_str("energy_zone")),
          "energy and energy_zone are equivalent");
}

static void unknown_zones_are_contained(void)
{
    CHECK_EQ(rb_zone_id_from_str("unknown combined zone"), RB_ZONEID_UNKNOWN,
             "unknown combined zone resolves to unknown");
    CHECK_EQ(rb_zone_id_from_str(""), RB_ZONEID_UNKNOWN,
             "empty zone resolves to unknown");
}

static void group_filter_excludes_self(void)
{
    TestGame tg;
    RbCardFilter filter;
    int cards[3];
    int matches[3];
    int n;
    int yoshiko;
    int chika;
    int riko;

    test_game_new(&tg);
    yoshiko = test_id(&tg, "PL!S-bp3-006-R＋");
    chika = test_id(&tg, "PL!S-bp2-001-R");
    riko = test_id(&tg, "PL!S-bp2-002-R");
    CHECK(yoshiko >= 0 && chika >= 0 && riko >= 0,
          "targeting fixture cards resolve");
    if (yoshiko < 0 || chika < 0 || riko < 0) return;

    tg.state.p[0].stage[0] = chika;
    tg.state.p[0].stage[1] = yoshiko;
    tg.state.p[0].stage[2] = riko;
    cards[0] = tg.state.p[0].stage[0];
    cards[1] = tg.state.p[0].stage[1];
    cards[2] = tg.state.p[0].stage[2];

    memset(&filter, 0, sizeof(filter));
    filter.has_filter = 1;
    filter.has_group = 1;
    snprintf(filter.group, sizeof(filter.group), "Aqours");
    filter.has_exclude_self = 1;
    filter.exclude_self_id = yoshiko;
    n = rb_matching_ids(&filter, cards, 3, matches, 3);
    CHECK_EQ(n, 2, "self-excluding group filter returns two members");
    if (n == 2) {
        CHECK(matches[0] == chika && matches[1] == riko,
              "self-excluding group filter returns the two outer members");
    }
}

static void add_condition_field(Condition *condition, const char *key, uint8_t tag, int64_t integer, const char *text)
{
    condition->fields[condition->n_fields].key = (char *)key;
    condition->fields[condition->n_fields].v.tag = tag;
    condition->fields[condition->n_fields].v.i = integer;
    condition->fields[condition->n_fields].v.s = (char *)text;
    condition->n_fields++;
}

static void turn_number_and_phase_are_conjunctive(void)
{
    TestGame tg;
    Condition condition;
    test_game_new(&tg);
    memset(&condition, 0, sizeof(condition));
    condition.variant = RB_COND_TEMPORAL;
    add_condition_field(&condition, "turn_number", RB_TAG_I64, 1, NULL);
    add_condition_field(&condition, "operator", RB_TAG_STR, 0, "=");
    add_condition_field(&condition, "phase", RB_TAG_STR, 0, "live_phase");

    tg.state.turn = 1;
    tg.state.phase = RB_PHASE_LIVE_SET;
    CHECK(rb_eval_condition(&tg.state, 0, &condition),
          "turn one live phase satisfies both temporal gates");
    tg.state.turn = 2;
    CHECK(!rb_eval_condition(&tg.state, 0, &condition),
          "turn two fails the turn-number gate");
    tg.state.turn = 1;
    tg.state.phase = RB_PHASE_MAIN;
    CHECK(!rb_eval_condition(&tg.state, 0, &condition),
          "turn one main phase fails the phase gate");
    tg.state.phase = RB_PHASE_LIVE_SET;
    condition.n_fields = 1;
    CHECK(rb_eval_condition(&tg.state, 0, &condition),
          "missing phase constraint remains backward compatible");
}

static void deck_parser_matches_rust_formats(void)
{
    const char *content = "3 x PL!N-bp1-026-L\nPL!N-bp1-029-L x 2\n// comment\nPL!N-bp3-001-SEC";
    char **cards = NULL;
    size_t count = 0;
    CHECK_EQ(rb_parse_deck_content(content, &cards, &count), 0,
             "deck content parser accepts supported formats");
    CHECK_EQ(count, 6, "deck parser expands quantities");
    if (count == 6) {
        CHECK(!strcmp(cards[0], "PL!N-bp1-026-L"), "prefix quantity card is first");
        CHECK(!strcmp(cards[3], "PL!N-bp1-029-L"), "suffix quantity card follows");
        CHECK(!strcmp(cards[5], "PL!N-bp3-001-SEC"), "bare card has quantity one");
    }
    for (size_t i = 0; i < count; i++) free(cards[i]);
    free(cards);
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    zone_round_trip();
    zone_aliases();
    unknown_zones_are_contained();
    group_filter_excludes_self();
    turn_number_and_phase_are_conjunctive();
    deck_parser_matches_rust_formats();
    rb_unload();
    if (failures) return 1;
    printf("ALL ZONE AND FILTER CHECKS PASSED\n");
    return 0;
}
