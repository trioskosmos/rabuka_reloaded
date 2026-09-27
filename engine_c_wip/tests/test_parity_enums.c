/* test_parity_enums.c  Eexhaustive wire-table parity for
   engine/src/ability/enums.rs.  Every canonical Rust wire string is pushed
   through from_str() -> to_str() and must come back unchanged; every alias
   accepted by Rust's from_str must map to the documented variant; and the
   unknown-input behaviour of every table is pinned.

   Rust references:
     Zone             engine/src/ability/enums.rs:34-167
     TargetPlayer     engine/src/ability/enums.rs:212-247
     ActionType       engine/src/ability/enums.rs:275-537  (wire_tables! 379-458, label 461-530)
     ConditionType    engine/src/ability/enums.rs:553-626  (wire_tables! 591-625)
     SelectTargetKind engine/src/ability/enums.rs:634-696
     EffectCardType   engine/src/ability/enums.rs:720-747
     EffectState      engine/src/ability/enums.rs:799-821
     PAY_SKIP_TARGET  engine/src/ability/types.rs:32            */
#include "rabuka.h"

#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } \
} while (0)

#define CHECK_STR(actual, expected, message) do { \
    checks++; \
    const char *a_ = (actual); \
    if (!a_ || strcmp(a_, (expected)) != 0) { \
        fprintf(stderr, "FAIL: %s (got \"%s\" expected \"%s\")\n", \
                (message), a_ ? a_ : "(null)", (expected)); \
        failures++; \
    } \
} while (0)

/* A wire table row: variant value, its canonical wire string, whether Rust's
   from_str can produce that variant from a string at all, and — for the rows
   it cannot — what from_str must return for that string instead.  Rows with
   parseable == 0 are variants that only exist as to_str outputs:
   Zone::EnergyZone (Rust parses "energy_zone" to Zone::Energy, enums.rs:88),
   Zone::LiveTotal and Zone::Unknown (no from_str arm at all, enums.rs:80-110,
   so the C port returns its Unknown fallback). */
typedef struct {
    int         value;
    const char *wire;
    int         parseable;
    int         not_parseable_yields;
} WireRow;

typedef int (*FromStrFn)(const char *s);
typedef const char *(*ToStrFn)(int v);

/* ---------------------------------------------------------------- Zone -- */

static const WireRow ZONE_ROWS[] = {
    { RB_ABILITY_ZONE_HAND,                "hand",                1, 0 },
    { RB_ABILITY_ZONE_STAGE,               "stage",               1, 0 },
    { RB_ABILITY_ZONE_STAGE_CENTER,        "center",              1, 0 },
    { RB_ABILITY_ZONE_STAGE_LEFT,          "left",                1, 0 },
    { RB_ABILITY_ZONE_STAGE_RIGHT,         "right",               1, 0 },
    { RB_ABILITY_ZONE_DISCARD,             "discard",             1, 0 },
    { RB_ABILITY_ZONE_WAITROOM,            "waitroom",            1, 0 },
    { RB_ABILITY_ZONE_ENERGY,              "energy",              1, 0 },
    { RB_ABILITY_ZONE_ENERGY_ZONE,         "energy_zone",         0, RB_ABILITY_ZONE_ENERGY },
    { RB_ABILITY_ZONE_DECK,                "deck",                1, 0 },
    { RB_ABILITY_ZONE_DECK_TOP,            "deck_top",            1, 0 },
    { RB_ABILITY_ZONE_DECK_BOTTOM,         "deck_bottom",         1, 0 },
    { RB_ABILITY_ZONE_SUCCESS_ZONE,        "success_zone",        1, 0 },
    { RB_ABILITY_ZONE_LIVE_CARD_ZONE,      "live_card_zone",      1, 0 },
    { RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE,   "success_live_zone",   1, 0 },
    { RB_ABILITY_ZONE_ENERGY_DECK,         "energy_deck",         1, 0 },
    { RB_ABILITY_ZONE_EMPTY_AREA,          "empty_area",          1, 0 },
    { RB_ABILITY_ZONE_SAME_AREA,           "same_area",           1, 0 },
    { RB_ABILITY_ZONE_UNDER_MEMBER,        "under_member",        1, 0 },
    { RB_ABILITY_ZONE_LOOKED_AT,           "looked_at",           1, 0 },
    { RB_ABILITY_ZONE_REVEALED_CARDS,      "revealed_cards",      1, 0 },
    { RB_ABILITY_ZONE_SELECTED_CARDS,      "selected_cards",      1, 0 },
    { RB_ABILITY_ZONE_RESOLUTION,          "resolution",          1, 0 },
    { RB_ABILITY_ZONE_EXCLUSION_ZONE,      "exclusion_zone",      1, 0 },
    { RB_ABILITY_ZONE_PRECEDING_MOVED,     "preceding_moved",     1, 0 },
    { RB_ABILITY_ZONE_RECENTLY_MOVED,      "recently_moved",      1, 0 },
    { RB_ABILITY_ZONE_THOSE_CARDS,         "those_cards",         1, 0 },
    { RB_ABILITY_ZONE_LOOKED_AT_REMAINING, "looked_at_remaining", 1, 0 },
    { RB_ABILITY_ZONE_DECK_TOP_OR_BOTTOM,  "deck_top_or_bottom",  1, 0 },
    { RB_ABILITY_ZONE_FRONT,               "front",               1, 0 },
    { RB_ABILITY_ZONE_LIVE_TOTAL,          "live_total",          0, RB_ABILITY_ZONE_UNKNOWN },
    { RB_ABILITY_ZONE_UNKNOWN,             "unknown",             0, RB_ABILITY_ZONE_UNKNOWN },
};
#define N_ZONE_ROWS ((int)(sizeof(ZONE_ROWS) / sizeof(ZONE_ROWS[0])))

/* Every string Rust's Zone::from_str accepts (enums.rs:80-111) and the
   variant it yields.  Includes the 5 aliases plus "energy_zone", which is an
   alias of Zone::Energy (Zone::EnergyZone is unreachable from a string). */
static const WireRow ZONE_PARSE[] = {
    { RB_ABILITY_ZONE_HAND,                "hand",                    1, 0 },
    { RB_ABILITY_ZONE_STAGE,               "stage",                   1, 0 },
    { RB_ABILITY_ZONE_STAGE_CENTER,        "center",                  1, 0 },
    { RB_ABILITY_ZONE_STAGE_LEFT,          "left",                    1, 0 },
    { RB_ABILITY_ZONE_STAGE_LEFT,          "left_side",               1, 0 },
    { RB_ABILITY_ZONE_STAGE_RIGHT,         "right",                   1, 0 },
    { RB_ABILITY_ZONE_STAGE_RIGHT,         "right_side",              1, 0 },
    { RB_ABILITY_ZONE_DISCARD,             "discard",                 1, 0 },
    { RB_ABILITY_ZONE_WAITROOM,            "waitroom",                1, 0 },
    { RB_ABILITY_ZONE_ENERGY,              "energy",                  1, 0 },
    { RB_ABILITY_ZONE_ENERGY,              "energy_zone",             1, 0 },
    { RB_ABILITY_ZONE_DECK,                "deck",                    1, 0 },
    { RB_ABILITY_ZONE_DECK_TOP,            "deck_top",                1, 0 },
    { RB_ABILITY_ZONE_DECK_BOTTOM,         "deck_bottom",             1, 0 },
    { RB_ABILITY_ZONE_SUCCESS_ZONE,        "success_zone",            1, 0 },
    { RB_ABILITY_ZONE_LIVE_CARD_ZONE,      "live_card_zone",          1, 0 },
    { RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE,   "success_live_zone",       1, 0 },
    { RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE,   "success_live_card_zone",  1, 0 },
    { RB_ABILITY_ZONE_ENERGY_DECK,         "energy_deck",             1, 0 },
    { RB_ABILITY_ZONE_EMPTY_AREA,          "empty_area",              1, 0 },
    { RB_ABILITY_ZONE_SAME_AREA,           "same_area",               1, 0 },
    { RB_ABILITY_ZONE_UNDER_MEMBER,        "under_member",            1, 0 },
    { RB_ABILITY_ZONE_UNDER_MEMBER,        "under",                   1, 0 },
    { RB_ABILITY_ZONE_LOOKED_AT,           "looked_at",               1, 0 },
    { RB_ABILITY_ZONE_REVEALED_CARDS,      "revealed_cards",          1, 0 },
    { RB_ABILITY_ZONE_SELECTED_CARDS,      "selected_cards",          1, 0 },
    { RB_ABILITY_ZONE_RESOLUTION,          "resolution",              1, 0 },
    { RB_ABILITY_ZONE_RESOLUTION,          "resolution_zone",         1, 0 },
    { RB_ABILITY_ZONE_EXCLUSION_ZONE,      "exclusion_zone",          1, 0 },
    { RB_ABILITY_ZONE_PRECEDING_MOVED,     "preceding_moved",         1, 0 },
    { RB_ABILITY_ZONE_RECENTLY_MOVED,      "recently_moved",          1, 0 },
    { RB_ABILITY_ZONE_THOSE_CARDS,         "those_cards",             1, 0 },
    { RB_ABILITY_ZONE_LOOKED_AT_REMAINING, "looked_at_remaining",     1, 0 },
    { RB_ABILITY_ZONE_DECK_TOP_OR_BOTTOM,  "deck_top_or_bottom",      1, 0 },
    { RB_ABILITY_ZONE_FRONT,               "front",                   1, 0 },
};
#define N_ZONE_PARSE ((int)(sizeof(ZONE_PARSE) / sizeof(ZONE_PARSE[0])))

/* -------------------------------------------------------- TargetPlayer -- */

static const WireRow TARGET_PLAYER_ROWS[] = {
    { RB_TARGET_SELF,     "self",     1, 0 },
    { RB_TARGET_OPPONENT, "opponent", 1, 0 },
    { RB_TARGET_BOTH,     "both",     1, 0 },
    { RB_TARGET_EITHER,   "either",   1, 0 },
};
#define N_TARGET_PLAYER_ROWS ((int)(sizeof(TARGET_PLAYER_ROWS) / sizeof(TARGET_PLAYER_ROWS[0])))

/* ----------------------------------------------------------- ActionType -- */

static const WireRow ACTION_ROWS[] = {
    { RB_ACTION_DRAW_CARD,                        "draw_card",                        1, 0 },
    { RB_ACTION_DRAW_UNTIL_COUNT,                 "draw_until_count",                 1, 0 },
    { RB_ACTION_MOVE_CARDS,                       "move_cards",                       1, 0 },
    { RB_ACTION_DISCARD_CARD,                     "discard_card",                     1, 0 },
    { RB_ACTION_SELECT,                           "select",                           1, 0 },
    { RB_ACTION_SELECT_NUMBER,                    "select_number",                    1, 0 },
    { RB_ACTION_SELECT_CARDS,                     "select_cards",                     1, 0 },
    { RB_ACTION_LOOK_AND_SELECT,                  "look_and_select",                  1, 0 },
    { RB_ACTION_LOOK_AT,                          "look_at",                          1, 0 },
    { RB_ACTION_REVEAL,                           "reveal",                           1, 0 },
    { RB_ACTION_REVEAL_PER_GROUP,                 "reveal_per_group",                 1, 0 },
    { RB_ACTION_REVEAL_UNTIL_LIVE_CARD,           "reveal_until_live_card",           1, 0 },
    { RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD,         "reveal_until_chosen_card",         1, 0 },
    { RB_ACTION_CHANGE_STATE,                     "change_state",                     1, 0 },
    { RB_ACTION_POSITION_CHANGE,                  "position_change",                  1, 0 },
    { RB_ACTION_ROTATION,                         "rotation",                         1, 0 },
    { RB_ACTION_PLACE_ENERGY_UNDER_MEMBER,        "place_energy_under_member",        1, 0 },
    { RB_ACTION_SET_CARD_IDENTITY,                "set_card_identity",                1, 0 },
    { RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS,   "modify_required_hearts_success",   1, 0 },
    { RB_ACTION_GAIN_RESOURCE,                    "gain_resource",                    1, 0 },
    { RB_ACTION_PAY_ENERGY,                       "pay_energy",                       1, 0 },
    { RB_ACTION_GAIN_ABILITY,                     "gain_ability",                     1, 0 },
    { RB_ACTION_GAIN_ABILITY_FROM_SOURCE,         "gain_ability_from_source",         1, 0 },
    { RB_ACTION_INVALIDATE_ABILITY,               "invalidate_ability",               1, 0 },
    { RB_ACTION_SUPPRESS_ABILITY_TRIGGER,         "suppress_ability_trigger",         1, 0 },
    { RB_ACTION_ACTIVATE_ABILITY,                 "activate_ability",                 1, 0 },
    { RB_ACTION_MODIFY_COST,                      "modify_cost",                      1, 0 },
    { RB_ACTION_MODIFY_YELL_SOURCE,               "modify_yell_source",               1, 0 },
    { RB_ACTION_SET_COST,                         "set_cost",                         1, 0 },
    { RB_ACTION_SET_COST_TO_USE,                  "set_cost_to_use",                  1, 0 },
    { RB_ACTION_MODIFY_SCORE,                     "modify_score",                     1, 0 },
    { RB_ACTION_MODIFY_REQUIRED_HEARTS,           "modify_required_hearts",           1, 0 },
    { RB_ACTION_SET_BLADE_TYPE,                   "set_blade_type",                   1, 0 },
    { RB_ACTION_SET_BLADE_COUNT,                  "set_blade_count",                  1, 0 },
    { RB_ACTION_SET_HEART_TYPE,                   "set_heart_type",                   1, 0 },
    { RB_ACTION_SPECIFY_HEART_COLOR,              "specify_heart_color",              1, 0 },
    { RB_ACTION_CHOOSE_REQUIRED_HEARTS,           "choose_required_hearts",           1, 0 },
    { RB_ACTION_SEQUENTIAL,                       "sequential",                       1, 0 },
    { RB_ACTION_CONDITIONAL_ALTERNATIVE,          "conditional_alternative",          1, 0 },
    { RB_ACTION_CONDITIONAL_ON_RESULT,            "conditional_on_result",            1, 0 },
    { RB_ACTION_CONDITIONAL_ON_OPTIONAL,          "conditional_on_optional",          1, 0 },
    { RB_ACTION_RESTRICTION,                      "restriction",                      1, 0 },
    { RB_ACTION_ACTIVATION_RESTRICTION,           "activation_restriction",           1, 0 },
    { RB_ACTION_MODIFY_LIMIT,                     "modify_limit",                     1, 0 },
    { RB_ACTION_SHUFFLE,                          "shuffle",                          1, 0 },
    { RB_ACTION_RE_YELL,                          "re_yell",                          1, 0 },
    { RB_ACTION_CUSTOM,                           "custom",                           1, 0 },
    { RB_ACTION_DO_NOTHING,                       "do_nothing",                       1, 0 },
    { RB_ACTION_CHOICE,                           "choice",                           1, 0 },
    { RB_ACTION_REPEAT_PROCEDURE,                 "repeat_procedure",                 1, 0 },
    { RB_ACTION_DISCARD_UNTIL_COUNT,              "discard_until_count",              1, 0 },
    { RB_ACTION_ALL_BLADE_TIMING,                 "all_blade_timing",                 1, 0 },
    { RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT,       "reduce_live_card_set_limit",       1, 0 },
    { RB_ACTION_CHOOSE_TARGET_PLAYER,             "choose_target_player",             1, 0 },
    { RB_ACTION_PLAY_BATON_TOUCH,                 "play_baton_touch",                 1, 0 },
    { RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL,    "modify_required_hearts_global",    1, 0 },
    { RB_ACTION_MODIFY_YELL_COUNT,                "modify_yell_count",                1, 0 },
    { RB_ACTION_ACTIVATION_COST,                  "activation_cost",                  1, 0 },
    { RB_ACTION_PERFORM_YELL,                     "perform_yell",                     1, 0 },
    { RB_ACTION_CONDITIONAL_OPTIONAL,             "conditional_optional",             1, 0 },
    { RB_ACTION_COMPOUND_ACTION,                  "compound_action",                  1, 0 },
    { RB_ACTION_OPPONENT_ACTION,                  "opponent_action",                  1, 0 },
    { RB_ACTION_ACTION_BY,                        "action_by",                        1, 0 },
    { RB_ACTION_SEQUENTIAL_COST,                  "sequential_cost",                  1, 0 },
    { RB_ACTION_CHOICE_CONDITION,                 "choice_condition",                 1, 0 },
    { RB_ACTION_ENERGY_CONDITION,                 "energy_condition",                 1, 0 },
};
#define N_ACTION_ROWS ((int)(sizeof(ACTION_ROWS) / sizeof(ACTION_ROWS[0])))

/* ------------------------------------------------------- ConditionType -- */

static const WireRow CONDTYPE_ROWS[] = {
    { RB_CONDTYPE_COMPOUND,                       "compound",                       1, 0 },
    { RB_CONDTYPE_COMPARISON,                     "comparison_condition",           1, 0 },
    { RB_CONDTYPE_LOCATION,                       "location_condition",             1, 0 },
    { RB_CONDTYPE_CARD_COUNT,                     "card_count_condition",           1, 0 },
    { RB_CONDTYPE_CARD_BLADE,                     "card_blade_condition",           1, 0 },
    { RB_CONDTYPE_GROUP,                          "group_condition",                1, 0 },
    { RB_CONDTYPE_POSITION,                       "position_condition",             1, 0 },
    { RB_CONDTYPE_APPEARANCE,                     "appearance_condition",           1, 0 },
    { RB_CONDTYPE_TEMPORAL,                       "temporal_condition",             1, 0 },
    { RB_CONDTYPE_STATE,                          "state_condition",                1, 0 },
    { RB_CONDTYPE_ENERGY_STATE,                   "energy_state_condition",         1, 0 },
    { RB_CONDTYPE_MOVEMENT,                       "movement_condition",             1, 0 },
    { RB_CONDTYPE_ABILITY_FILTER,                 "ability_filter_condition",       1, 0 },
    { RB_CONDTYPE_OR,                             "or_condition",                   1, 0 },
    { RB_CONDTYPE_ANY_OF,                         "any_of_condition",               1, 0 },
    { RB_CONDTYPE_SCORE_THRESHOLD,                "score_threshold_condition",      1, 0 },
    { RB_CONDTYPE_CHOICE,                         "choice_condition",               1, 0 },
    { RB_CONDTYPE_POSITION_CHANGE,                "position_change_condition",      1, 0 },
    { RB_CONDTYPE_STATE_CHANGE,                   "state_change_condition",         1, 0 },
    { RB_CONDTYPE_OPPONENT_CHOICE,                "opponent_choice_condition",      1, 0 },
    { RB_CONDTYPE_OPPONENT_LIVE_SUCCESS,          "opponent_live_success",          1, 0 },
    { RB_CONDTYPE_COMPLEX,                        "complex_condition",              1, 0 },
    { RB_CONDTYPE_NO_EXCESS_HEART,                "no_excess_heart",                1, 0 },
    { RB_CONDTYPE_OTHERWISE,                      "otherwise_condition",            1, 0 },
    { RB_CONDTYPE_BOTH,                           "both_condition",                 1, 0 },
    { RB_CONDTYPE_NOT_MOVED,                      "not_moved",                      1, 0 },
    { RB_CONDTYPE_HAS_MOVED,                      "has_moved",                      1, 0 },
    { RB_CONDTYPE_RESOURCE,                       "resource_condition",             1, 0 },
    { RB_CONDTYPE_ACTION_SUCCESS,                 "action_success_condition",       1, 0 },
    { RB_CONDTYPE_ALL_COST_COMPARISON,            "all_cost_comparison_condition",  1, 0 },
    { RB_CONDTYPE_HIGHEST_COST_ON_STAGE,          "highest_cost_on_stage_condition",1, 0 },
    { RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR, "all_revealed_match_heart_color", 1, 0 },
    { RB_CONDTYPE_CUSTOM,                         "custom",                         1, 0 },
};
#define N_CONDTYPE_ROWS ((int)(sizeof(CONDTYPE_ROWS) / sizeof(CONDTYPE_ROWS[0])))

/* ---------------------------------------------------- SelectTargetKind -- */

static const WireRow STK_ROWS[] = {
    { RB_STK_CHOICE,                       "choice",                       1, 0 },
    { RB_STK_CHOICE_STRING,                "choice_string",                1, 0 },
    { RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST, "pay_optional_cost:skip_optional_cost", 1, 0 },
    { RB_STK_DOUBLE_BATON_TOUCH,           "double_baton_touch",           1, 0 },
    { RB_STK_PRIMARY_ALTERNATIVE,          "primary|alternative",          1, 0 },
    { RB_STK_APPLY_REPLACEMENT,            "apply_replacement",            1, 0 },
    { RB_STK_CHOOSE_REQUIRED_HEARTS,       "choose_required_hearts",       1, 0 },
    { RB_STK_POSITION_DESTINATION,         "position|destination",         1, 0 },
    { RB_STK_HEART_COLOR,                  "heart_color",                  1, 0 },
    { RB_STK_CHOICE_TYPE,                  "choice_type",                  1, 0 },
    { RB_STK_CHOICE_CONDITION,             "choice_condition",             1, 0 },
    { RB_STK_CONDITIONAL_OPTIONAL,         "conditional_optional",         1, 0 },
    { RB_STK_DRAW_ANY_NUMBER,              "draw_any_number",              1, 0 },
    { RB_STK_ORDER,                        "order",                        1, 0 },
    { RB_STK_SELF_OR_OPPONENT,             "self_or_opponent",             1, 0 },
    { RB_STK_PAY_COST_ALL_DISCARD,         "pay_cost_all:discard_all",     1, 0 },
};
#define N_STK_ROWS ((int)(sizeof(STK_ROWS) / sizeof(STK_ROWS[0])))

/* ------------------------------------------------------------- helpers -- */

static void check_distinct(const WireRow *rows, int n, const char *table)
{
    for (int i = 0; i < n; i++) {
        for (int j = i + 1; j < n; j++) {
            char msg[256];
            snprintf(msg, sizeof(msg), "%s: duplicate wire string at rows %d and %d",
                     table, i, j);
            CHECK(strcmp(rows[i].wire, rows[j].wire) != 0, msg);
            snprintf(msg, sizeof(msg), "%s: duplicate variant at rows %d and %d",
                     table, i, j);
            CHECK(rows[i].value != rows[j].value, msg);
        }
    }
}

/* Round-trips every row: to_str(value) == wire, and from_str(wire) returns
   the same variant whenever Rust can produce it from a string. */
static void check_round_trip(const WireRow *rows, int n, const char *table,
                             FromStrFn from_str, ToStrFn to_str)
{
    for (int i = 0; i < n; i++) {
        char msg[256];

        snprintf(msg, sizeof(msg), "%s: to_str(%d)", table, rows[i].value);
        CHECK_STR(to_str(rows[i].value), rows[i].wire, msg);

        int back = from_str(rows[i].wire);
        if (rows[i].parseable) {
            snprintf(msg, sizeof(msg), "%s: from_str(\"%s\") == %d",
                     table, rows[i].wire, rows[i].value);
            CHECK(back == rows[i].value, msg);

            snprintf(msg, sizeof(msg), "%s: round-trip \"%s\"", table, rows[i].wire);
            CHECK_STR(to_str(back), rows[i].wire, msg);
        } else {
            /* Rust's from_str cannot yield this row's variant: either it has
               no arm for the string (None -> the C table's Unknown fallback)
               or the string is an alias of a different variant. */
            snprintf(msg, sizeof(msg),
                     "%s: from_str(\"%s\") yields %d, not %d",
                     table, rows[i].wire, rows[i].not_parseable_yields,
                     rows[i].value);
            CHECK(back == rows[i].not_parseable_yields, msg);
        }
    }
}

/* --------------------------------------------------------------- tests -- */

static void test_zone(void)
{
    check_distinct(ZONE_ROWS, N_ZONE_ROWS, "Zone");
    check_round_trip(ZONE_ROWS, N_ZONE_ROWS, "Zone",
                     rb_ability_zone_from_str, rb_ability_zone_to_str);

    for (int i = 0; i < N_ZONE_PARSE; i++) {
        char msg[256];
        int back = rb_ability_zone_from_str(ZONE_PARSE[i].wire);
        snprintf(msg, sizeof(msg), "Zone: from_str(\"%s\") == %d",
                 ZONE_PARSE[i].wire, ZONE_PARSE[i].value);
        CHECK(back == ZONE_PARSE[i].value, msg);
    }

    /* Unknown input: Rust's from_str returns None; the C port has no Option
       and maps it to Zone::Unknown, exactly like Zone::from_source_str
       (enums.rs:158-160). */
    CHECK(rb_ability_zone_from_str("no_such_zone") == RB_ABILITY_ZONE_UNKNOWN,
          "Zone: garbage maps to Unknown (Rust from_str -> None)");
    CHECK(rb_ability_zone_from_str("") == RB_ABILITY_ZONE_UNKNOWN,
          "Zone: empty string maps to Unknown");
    CHECK(rb_ability_zone_from_str(NULL) == -1,
          "Zone: null is the -1 sentinel, not the Unknown fallback");
    CHECK(rb_ability_zone_from_source_str("no_such_zone") == RB_ABILITY_ZONE_UNKNOWN,
          "Zone: from_source_str garbage is Unknown");
    CHECK(rb_ability_zone_from_source_str("under") == RB_ABILITY_ZONE_UNDER_MEMBER,
          "Zone: from_source_str keeps known markers");
    CHECK(rb_ability_zone_to_str(-1) == NULL,
          "Zone: to_str of an out-of-range value is NULL");
    CHECK_STR(rb_ability_zone_as_str(RB_ABILITY_ZONE_HAND), "hand",
              "Zone: as_str mirrors to_str");
}

static void test_target_player(void)
{
    check_distinct(TARGET_PLAYER_ROWS, N_TARGET_PLAYER_ROWS, "TargetPlayer");
    check_round_trip(TARGET_PLAYER_ROWS, N_TARGET_PLAYER_ROWS, "TargetPlayer",
                     rb_target_player_from_str, rb_target_player_to_str);
    CHECK(rb_target_player_from_str("deck") < 0,
          "TargetPlayer: non-player string is rejected (Rust -> None)");
    CHECK(rb_target_player_from_str("") < 0, "TargetPlayer: empty string rejected");
    CHECK(rb_target_player_from_str(NULL) < 0, "TargetPlayer: null rejected");
    CHECK(rb_target_player_to_str(-1) == NULL,
          "TargetPlayer: to_str of an unknown value is NULL");
    CHECK_STR(rb_target_player_as_str(RB_TARGET_BOTH), "both",
              "TargetPlayer: as_str mirrors to_str");
}

static void test_action_type(void)
{
    check_distinct(ACTION_ROWS, N_ACTION_ROWS, "ActionType");
    check_round_trip(ACTION_ROWS, N_ACTION_ROWS, "ActionType",
                     rb_action_type_from_str, rb_action_type_to_str);
    for (int i = 0; i < N_ACTION_ROWS; i++) {
        char msg[256];
        const char *label = rb_action_type_label(ACTION_ROWS[i].value);
        snprintf(msg, sizeof(msg), "ActionType: label(%d) is non-empty", ACTION_ROWS[i].value);
        CHECK(label && label[0] != '\0', msg);
    }
    /* enums.rs:533-537: Default is ActionType::Custom. */
    CHECK(rb_action_type_default() == RB_ACTION_CUSTOM,
          "ActionType: default() is Custom");
    CHECK_STR(rb_action_type_to_str(rb_action_type_default()), "custom",
              "ActionType: default() wire string");
    CHECK(rb_action_type_from_str("no_such_action") < 0,
          "ActionType: garbage rejected (Rust -> None)");
    CHECK(rb_action_type_from_str("") < 0, "ActionType: empty string rejected");
    CHECK(rb_action_type_from_str(NULL) < 0, "ActionType: null rejected");
    CHECK(rb_action_type_to_str(-1) == NULL,
          "ActionType: to_str of an unknown value is NULL");
    CHECK(rb_action_type_label(-1) == NULL,
          "ActionType: label of an unknown value is NULL");
}

static void test_condition_type(void)
{
    check_distinct(CONDTYPE_ROWS, N_CONDTYPE_ROWS, "ConditionType");
    check_round_trip(CONDTYPE_ROWS, N_CONDTYPE_ROWS, "ConditionType",
                     rb_condition_type_from_str, rb_condition_type_to_str);
    CHECK(rb_condition_type_from_str("no_such_condition") < 0,
          "ConditionType: garbage rejected (Rust -> None)");
    CHECK(rb_condition_type_from_str("") < 0,
          "ConditionType: empty string rejected");
    CHECK(rb_condition_type_from_str(NULL) < 0,
          "ConditionType: null rejected");
    CHECK(rb_condition_type_to_str(-1) == NULL,
          "ConditionType: to_str of an unknown value is NULL");
}

static void test_select_target_kind(void)
{
    check_distinct(STK_ROWS, N_STK_ROWS, "SelectTargetKind");
    check_round_trip(STK_ROWS, N_STK_ROWS, "SelectTargetKind",
                     rb_select_target_kind_from_str, rb_select_target_kind_to_str);
    /* enums.rs:658,680 and types.rs:32 use the shared constant, not a
       duplicated literal, on both sides of the table. */
    CHECK_STR(RB_PAY_SKIP_TARGET, "pay_optional_cost:skip_optional_cost",
              "SelectTargetKind: PAY_SKIP_TARGET constant value");
    CHECK_STR(rb_select_target_kind_to_str(RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST),
              RB_PAY_SKIP_TARGET,
              "SelectTargetKind: to_str uses the PAY_SKIP_TARGET constant");
    CHECK(rb_select_target_kind_from_str(RB_PAY_SKIP_TARGET)
              == RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST,
          "SelectTargetKind: from_str uses the PAY_SKIP_TARGET constant");
    CHECK(rb_select_target_kind_from_str("no_such_target") < 0,
          "SelectTargetKind: garbage rejected (Rust -> None)");
    CHECK(rb_select_target_kind_from_str("") < 0,
          "SelectTargetKind: empty string rejected");
    CHECK(rb_select_target_kind_from_str(NULL) < 0,
          "SelectTargetKind: null rejected");
    CHECK(rb_select_target_kind_to_str(-1) == NULL,
          "SelectTargetKind: to_str of an unknown value is NULL");
}

static void test_effect_card_type_and_state(void)
{
    CHECK(rb_effect_card_type_from_str("member_card") == RB_ECT_MEMBER_CARD,
          "EffectCardType: member_card");
    CHECK(rb_effect_card_type_from_str("live_card") == RB_ECT_LIVE_CARD,
          "EffectCardType: live_card");
    CHECK(rb_effect_card_type_from_str("energy_card") == RB_ECT_ENERGY_CARD,
          "EffectCardType: energy_card");
    CHECK_STR(rb_effect_card_type_as_str(RB_ECT_MEMBER_CARD), "member_card",
              "EffectCardType: as_str member_card");
    CHECK_STR(rb_effect_card_type_as_str(RB_ECT_LIVE_CARD), "live_card",
              "EffectCardType: as_str live_card");
    CHECK_STR(rb_effect_card_type_as_str(RB_ECT_ENERGY_CARD), "energy_card",
              "EffectCardType: as_str energy_card");
    CHECK(rb_effect_card_type_default() == RB_ECT_OTHER,
          "EffectCardType: default is Other(\"\")");

    /* Known DIVERGENCE from Rust, pinned deliberately: Rust's
       EffectCardType::Other(ArcStr) preserves the unrecognized string and
       as_str returns it verbatim (enums.rs:735, 744). The C enum has no
       string payload, so the value collapses to ""  Ea defect in the enum
       shape declared in include/rabuka.h, not in enums.c. */
    CHECK(rb_effect_card_type_from_str("no_such_card_type") == RB_ECT_OTHER,
          "EffectCardType: unknown maps to Other (Rust keeps the string  Eknown gap)");
    CHECK(rb_effect_card_type_from_str(NULL) == -1,
          "EffectCardType: null is the -1 sentinel, not Other");
    CHECK_STR(rb_effect_card_type_as_str(RB_ECT_OTHER), "",
              "EffectCardType: Other as_str is empty in C (Rust echoes the input)");
    CHECK(rb_effect_card_type_as_str(-1) == NULL,
          "EffectCardType: as_str of an unknown value is NULL");

    CHECK(rb_effect_state_from_str("active") == RB_ES_ACTIVE,
          "EffectState: active");
    CHECK(rb_effect_state_from_str("wait") == RB_ES_WAIT,
          "EffectState: wait");
    CHECK_STR(rb_effect_state_as_str(RB_ES_ACTIVE), "active",
              "EffectState: as_str active");
    CHECK_STR(rb_effect_state_as_str(RB_ES_WAIT), "wait",
              "EffectState: as_str wait");
    CHECK(rb_effect_state_default() == RB_ES_OTHER,
          "EffectState: default is Other(\"\")");
    /* Same known divergence as EffectCardType above (enums.rs:810, 818). */
    CHECK(rb_effect_state_from_str("no_such_state") == RB_ES_OTHER,
          "EffectState: unknown maps to Other (Rust keeps the string  Eknown gap)");
    CHECK(rb_effect_state_from_str(NULL) == -1,
          "EffectState: null is the -1 sentinel, not Other");
    CHECK_STR(rb_effect_state_as_str(RB_ES_OTHER), "",
              "EffectState: Other as_str is empty in C (Rust echoes the input)");
    CHECK(rb_effect_state_as_str(-1) == NULL,
          "EffectState: as_str of an unknown value is NULL");
}

int main(void)
{
    test_zone();
    test_target_player();
    test_action_type();
    test_condition_type();
    test_select_target_kind();
    test_effect_card_type_and_state();

    printf("parity_enums: %d assertions, %d failures\n", checks, failures);
    if (failures) return 1;
    puts("parity_enums: all enum wire tables round-trip");
    return 0;
}
