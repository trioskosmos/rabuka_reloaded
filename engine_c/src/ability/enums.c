#include "rabuka.h"
#include <string.h>

/* ====================================================================
 *  Port of engine/src/ability/enums.rs
 *  Strongly-typed ability enums: Zone, TargetPlayer, PlacementTarget,
 *  ActionType, ConditionType, SelectTargetKind, EffectCardType,
 *  EffectState, plus their wire-string helpers.
 * ==================================================================== */

/* ── TargetPlayer helpers ── */
int rb_target_player_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "self")) return RB_TARGET_SELF;
    if (!strcmp(s, "opponent")) return RB_TARGET_OPPONENT;
    if (!strcmp(s, "both")) return RB_TARGET_BOTH;
    if (!strcmp(s, "either")) return RB_TARGET_EITHER;
    return -1;
}

const char *rb_target_player_to_str(int tp) {
    switch (tp) {
        case RB_TARGET_SELF:      return "self";
        case RB_TARGET_OPPONENT:  return "opponent";
        case RB_TARGET_BOTH:      return "both";
        case RB_TARGET_EITHER:    return "either";
        default:                  return NULL;
    }
}

const char *rb_target_player_as_str(int tp) {
    return rb_target_player_to_str(tp);
}

/* ── ActionType helpers ── */
int rb_action_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "draw_card")) return RB_ACTION_DRAW_CARD;
    if (!strcmp(s, "draw_until_count")) return RB_ACTION_DRAW_UNTIL_COUNT;
    if (!strcmp(s, "move_cards")) return RB_ACTION_MOVE_CARDS;
    if (!strcmp(s, "discard_card")) return RB_ACTION_DISCARD_CARD;
    if (!strcmp(s, "select")) return RB_ACTION_SELECT;
    if (!strcmp(s, "select_number")) return RB_ACTION_SELECT_NUMBER;
    if (!strcmp(s, "select_cards")) return RB_ACTION_SELECT_CARDS;
    if (!strcmp(s, "look_and_select")) return RB_ACTION_LOOK_AND_SELECT;
    if (!strcmp(s, "look_at")) return RB_ACTION_LOOK_AT;
    if (!strcmp(s, "reveal")) return RB_ACTION_REVEAL;
    if (!strcmp(s, "reveal_per_group")) return RB_ACTION_REVEAL_PER_GROUP;
    if (!strcmp(s, "reveal_until_live_card")) return RB_ACTION_REVEAL_UNTIL_LIVE_CARD;
    if (!strcmp(s, "reveal_until_chosen_card")) return RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD;
    if (!strcmp(s, "change_state")) return RB_ACTION_CHANGE_STATE;
    if (!strcmp(s, "position_change")) return RB_ACTION_POSITION_CHANGE;
    if (!strcmp(s, "rotation")) return RB_ACTION_ROTATION;
    if (!strcmp(s, "place_energy_under_member")) return RB_ACTION_PLACE_ENERGY_UNDER_MEMBER;
    if (!strcmp(s, "modify_required_hearts_success")) return RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS;
    if (!strcmp(s, "gain_resource")) return RB_ACTION_GAIN_RESOURCE;
    if (!strcmp(s, "pay_energy")) return RB_ACTION_PAY_ENERGY;
    if (!strcmp(s, "gain_ability")) return RB_ACTION_GAIN_ABILITY;
    if (!strcmp(s, "gain_ability_from_source")) return RB_ACTION_GAIN_ABILITY_FROM_SOURCE;
    if (!strcmp(s, "invalidate_ability")) return RB_ACTION_INVALIDATE_ABILITY;
    if (!strcmp(s, "suppress_ability_trigger")) return RB_ACTION_SUPPRESS_ABILITY_TRIGGER;
    if (!strcmp(s, "activate_ability")) return RB_ACTION_ACTIVATE_ABILITY;
    if (!strcmp(s, "modify_cost")) return RB_ACTION_MODIFY_COST;
    if (!strcmp(s, "modify_yell_source")) return RB_ACTION_MODIFY_YELL_SOURCE;
    if (!strcmp(s, "set_cost")) return RB_ACTION_SET_COST;
    if (!strcmp(s, "set_card_identity")) return RB_ACTION_SET_CARD_IDENTITY;
    if (!strcmp(s, "set_cost_to_use")) return RB_ACTION_SET_COST_TO_USE;
    if (!strcmp(s, "modify_score")) return RB_ACTION_MODIFY_SCORE;
    if (!strcmp(s, "modify_required_hearts")) return RB_ACTION_MODIFY_REQUIRED_HEARTS;
    if (!strcmp(s, "set_blade_type")) return RB_ACTION_SET_BLADE_TYPE;
    if (!strcmp(s, "set_blade_count")) return RB_ACTION_SET_BLADE_COUNT;
    if (!strcmp(s, "set_heart_type")) return RB_ACTION_SET_HEART_TYPE;
    if (!strcmp(s, "specify_heart_color")) return RB_ACTION_SPECIFY_HEART_COLOR;
    if (!strcmp(s, "choose_required_hearts")) return RB_ACTION_CHOOSE_REQUIRED_HEARTS;
    if (!strcmp(s, "sequential")) return RB_ACTION_SEQUENTIAL;
    if (!strcmp(s, "conditional_alternative")) return RB_ACTION_CONDITIONAL_ALTERNATIVE;
    if (!strcmp(s, "conditional_on_result")) return RB_ACTION_CONDITIONAL_ON_RESULT;
    if (!strcmp(s, "conditional_on_optional")) return RB_ACTION_CONDITIONAL_ON_OPTIONAL;
    if (!strcmp(s, "restriction")) return RB_ACTION_RESTRICTION;
    if (!strcmp(s, "activation_restriction")) return RB_ACTION_ACTIVATION_RESTRICTION;
    if (!strcmp(s, "modify_limit")) return RB_ACTION_MODIFY_LIMIT;
    if (!strcmp(s, "shuffle")) return RB_ACTION_SHUFFLE;
    if (!strcmp(s, "re_yell")) return RB_ACTION_RE_YELL;
    if (!strcmp(s, "custom")) return RB_ACTION_CUSTOM;
    if (!strcmp(s, "do_nothing")) return RB_ACTION_DO_NOTHING;
    if (!strcmp(s, "choice")) return RB_ACTION_CHOICE;
    if (!strcmp(s, "repeat_procedure")) return RB_ACTION_REPEAT_PROCEDURE;
    if (!strcmp(s, "discard_until_count")) return RB_ACTION_DISCARD_UNTIL_COUNT;
    if (!strcmp(s, "all_blade_timing")) return RB_ACTION_ALL_BLADE_TIMING;
    if (!strcmp(s, "reduce_live_card_set_limit")) return RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT;
    if (!strcmp(s, "choose_target_player")) return RB_ACTION_CHOOSE_TARGET_PLAYER;
    if (!strcmp(s, "play_baton_touch")) return RB_ACTION_PLAY_BATON_TOUCH;
    if (!strcmp(s, "modify_required_hearts_global")) return RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL;
    if (!strcmp(s, "modify_yell_count")) return RB_ACTION_MODIFY_YELL_COUNT;
    if (!strcmp(s, "activation_cost")) return RB_ACTION_ACTIVATION_COST;
    if (!strcmp(s, "perform_yell")) return RB_ACTION_PERFORM_YELL;
    if (!strcmp(s, "conditional_optional")) return RB_ACTION_CONDITIONAL_OPTIONAL;
    if (!strcmp(s, "compound_action")) return RB_ACTION_COMPOUND_ACTION;
    if (!strcmp(s, "opponent_action")) return RB_ACTION_OPPONENT_ACTION;
    if (!strcmp(s, "action_by")) return RB_ACTION_ACTION_BY;
    if (!strcmp(s, "sequential_cost")) return RB_ACTION_SEQUENTIAL_COST;
    if (!strcmp(s, "choice_condition")) return RB_ACTION_CHOICE_CONDITION;
    if (!strcmp(s, "energy_condition")) return RB_ACTION_ENERGY_CONDITION;
    return -1;
}

const char *rb_action_type_to_str(int at) {
    switch (at) {
        case RB_ACTION_DRAW_CARD:                  return "draw_card";
        case RB_ACTION_DRAW_UNTIL_COUNT:            return "draw_until_count";
        case RB_ACTION_MOVE_CARDS:                  return "move_cards";
        case RB_ACTION_DISCARD_CARD:                return "discard_card";
        case RB_ACTION_SELECT:                      return "select";
        case RB_ACTION_SELECT_NUMBER:               return "select_number";
        case RB_ACTION_SELECT_CARDS:                return "select_cards";
        case RB_ACTION_LOOK_AND_SELECT:             return "look_and_select";
        case RB_ACTION_LOOK_AT:                     return "look_at";
        case RB_ACTION_REVEAL:                      return "reveal";
        case RB_ACTION_REVEAL_PER_GROUP:            return "reveal_per_group";
        case RB_ACTION_REVEAL_UNTIL_LIVE_CARD:      return "reveal_until_live_card";
        case RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD:    return "reveal_until_chosen_card";
        case RB_ACTION_CHANGE_STATE:                return "change_state";
        case RB_ACTION_POSITION_CHANGE:             return "position_change";
        case RB_ACTION_ROTATION:                    return "rotation";
        case RB_ACTION_PLACE_ENERGY_UNDER_MEMBER:   return "place_energy_under_member";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS: return "modify_required_hearts_success";
        case RB_ACTION_GAIN_RESOURCE:               return "gain_resource";
        case RB_ACTION_PAY_ENERGY:                  return "pay_energy";
        case RB_ACTION_GAIN_ABILITY:                return "gain_ability";
        case RB_ACTION_GAIN_ABILITY_FROM_SOURCE:    return "gain_ability_from_source";
        case RB_ACTION_INVALIDATE_ABILITY:          return "invalidate_ability";
        case RB_ACTION_SUPPRESS_ABILITY_TRIGGER:    return "suppress_ability_trigger";
        case RB_ACTION_ACTIVATE_ABILITY:            return "activate_ability";
        case RB_ACTION_MODIFY_COST:                 return "modify_cost";
        case RB_ACTION_MODIFY_YELL_SOURCE:          return "modify_yell_source";
        case RB_ACTION_SET_COST:                    return "set_cost";
        case RB_ACTION_SET_CARD_IDENTITY:           return "set_card_identity";
        case RB_ACTION_SET_COST_TO_USE:             return "set_cost_to_use";
        case RB_ACTION_MODIFY_SCORE:                return "modify_score";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS:      return "modify_required_hearts";
        case RB_ACTION_SET_BLADE_TYPE:              return "set_blade_type";
        case RB_ACTION_SET_BLADE_COUNT:             return "set_blade_count";
        case RB_ACTION_SET_HEART_TYPE:              return "set_heart_type";
        case RB_ACTION_SPECIFY_HEART_COLOR:         return "specify_heart_color";
        case RB_ACTION_CHOOSE_REQUIRED_HEARTS:      return "choose_required_hearts";
        case RB_ACTION_SEQUENTIAL:                  return "sequential";
        case RB_ACTION_CONDITIONAL_ALTERNATIVE:     return "conditional_alternative";
        case RB_ACTION_CONDITIONAL_ON_RESULT:       return "conditional_on_result";
        case RB_ACTION_CONDITIONAL_ON_OPTIONAL:     return "conditional_on_optional";
        case RB_ACTION_RESTRICTION:                 return "restriction";
        case RB_ACTION_ACTIVATION_RESTRICTION:      return "activation_restriction";
        case RB_ACTION_MODIFY_LIMIT:                return "modify_limit";
        case RB_ACTION_SHUFFLE:                     return "shuffle";
        case RB_ACTION_RE_YELL:                     return "re_yell";
        case RB_ACTION_CUSTOM:                      return "custom";
        case RB_ACTION_DO_NOTHING:                  return "do_nothing";
        case RB_ACTION_CHOICE:                      return "choice";
        case RB_ACTION_REPEAT_PROCEDURE:            return "repeat_procedure";
        case RB_ACTION_DISCARD_UNTIL_COUNT:         return "discard_until_count";
        case RB_ACTION_ALL_BLADE_TIMING:            return "all_blade_timing";
        case RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT:  return "reduce_live_card_set_limit";
        case RB_ACTION_CHOOSE_TARGET_PLAYER:        return "choose_target_player";
        case RB_ACTION_PLAY_BATON_TOUCH:            return "play_baton_touch";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL: return "modify_required_hearts_global";
        case RB_ACTION_MODIFY_YELL_COUNT:           return "modify_yell_count";
        case RB_ACTION_ACTIVATION_COST:             return "activation_cost";
        case RB_ACTION_PERFORM_YELL:                return "perform_yell";
        case RB_ACTION_CONDITIONAL_OPTIONAL:        return "conditional_optional";
        case RB_ACTION_COMPOUND_ACTION:             return "compound_action";
        case RB_ACTION_OPPONENT_ACTION:             return "opponent_action";
        case RB_ACTION_ACTION_BY:                   return "action_by";
        case RB_ACTION_SEQUENTIAL_COST:             return "sequential_cost";
        case RB_ACTION_CHOICE_CONDITION:            return "choice_condition";
        case RB_ACTION_ENERGY_CONDITION:            return "energy_condition";
        default:                                    return NULL;
    }
}

const char *rb_action_type_label(int at) {
    switch (at) {
        case RB_ACTION_DRAW_CARD:                       return "Draw Card";
        case RB_ACTION_DRAW_UNTIL_COUNT:                return "Draw Until Count";
        case RB_ACTION_CHOOSE_TARGET_PLAYER:            return "Choose Target Player";
        case RB_ACTION_MOVE_CARDS:                      return "Move Cards";
        case RB_ACTION_DISCARD_CARD:                    return "Discard Card";
        case RB_ACTION_SELECT:                          return "Select";
        case RB_ACTION_SELECT_NUMBER:                   return "Select Number";
        case RB_ACTION_SELECT_CARDS:                    return "Select Cards";
        case RB_ACTION_LOOK_AND_SELECT:                 return "Look and Select";
        case RB_ACTION_LOOK_AT:                         return "Look At";
        case RB_ACTION_REVEAL:                          return "Reveal";
        case RB_ACTION_REVEAL_PER_GROUP:                return "Reveal Per Group";
        case RB_ACTION_REVEAL_UNTIL_LIVE_CARD:          return "Reveal Until Live Card";
        case RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD:        return "Reveal Until Chosen Card";
        case RB_ACTION_CHANGE_STATE:                    return "Change State";
        case RB_ACTION_POSITION_CHANGE:                 return "Position Change";
        case RB_ACTION_ROTATION:                        return "Rotation";
        case RB_ACTION_PLACE_ENERGY_UNDER_MEMBER:       return "Place Energy Under Member";
        case RB_ACTION_SET_CARD_IDENTITY:               return "Set Card Identity";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS:  return "Modify Required Hearts (Success)";
        case RB_ACTION_GAIN_RESOURCE:                   return "Gain Resource";
        case RB_ACTION_PAY_ENERGY:                      return "Pay Energy";
        case RB_ACTION_GAIN_ABILITY:                    return "Gain Ability";
        case RB_ACTION_GAIN_ABILITY_FROM_SOURCE:        return "Gain Ability from Source";
        case RB_ACTION_INVALIDATE_ABILITY:              return "Invalidate Ability";
        case RB_ACTION_SUPPRESS_ABILITY_TRIGGER:        return "Suppress Ability Trigger";
        case RB_ACTION_ACTIVATE_ABILITY:                return "Activate Ability";
        case RB_ACTION_MODIFY_SCORE:                    return "Modify Score";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS:          return "Modify Required Hearts";
        case RB_ACTION_MODIFY_YELL_SOURCE:              return "Modify Yell Source";
        case RB_ACTION_MODIFY_COST:                     return "Modify Cost";
        case RB_ACTION_SET_COST:                        return "Set Cost";
        case RB_ACTION_SET_COST_TO_USE:                 return "Set Cost to Use";
        case RB_ACTION_SET_BLADE_TYPE:                  return "Set Blade Type";
        case RB_ACTION_SET_BLADE_COUNT:                 return "Set Blade Count";
        case RB_ACTION_SET_HEART_TYPE:                  return "Set Heart Type";
        case RB_ACTION_SPECIFY_HEART_COLOR:             return "Specify Heart Color";
        case RB_ACTION_CHOOSE_REQUIRED_HEARTS:          return "Choose Required Hearts";
        case RB_ACTION_SEQUENTIAL:                      return "Sequential";
        case RB_ACTION_CONDITIONAL_ALTERNATIVE:         return "Conditional Alternative";
        case RB_ACTION_CONDITIONAL_ON_RESULT:           return "Conditional on Result";
        case RB_ACTION_CONDITIONAL_ON_OPTIONAL:         return "Conditional on Optional";
        case RB_ACTION_RESTRICTION:                     return "Restriction";
        case RB_ACTION_ACTIVATION_RESTRICTION:          return "Activation Restriction";
        case RB_ACTION_MODIFY_LIMIT:                    return "Modify Limit";
        case RB_ACTION_SHUFFLE:                         return "Shuffle";
        case RB_ACTION_RE_YELL:                         return "Re Yell";
        case RB_ACTION_CUSTOM:                          return "Custom";
        case RB_ACTION_DO_NOTHING:                      return "Do Nothing";
        case RB_ACTION_CHOICE:                          return "Choice";
        case RB_ACTION_REPEAT_PROCEDURE:                return "Repeat Procedure";
        case RB_ACTION_DISCARD_UNTIL_COUNT:             return "Discard Until Count";
        case RB_ACTION_ALL_BLADE_TIMING:                return "All Blade Timing";
        case RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT:      return "Reduce Live Card Set Limit";
        case RB_ACTION_PLAY_BATON_TOUCH:                return "Play Baton Touch";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL:   return "Modify Required Hearts (Global)";
        case RB_ACTION_MODIFY_YELL_COUNT:               return "Modify Yell Count";
        case RB_ACTION_ACTIVATION_COST:                 return "Activation Cost";
        case RB_ACTION_PERFORM_YELL:                    return "Perform Yell";
        case RB_ACTION_CONDITIONAL_OPTIONAL:            return "Conditional Optional";
        case RB_ACTION_COMPOUND_ACTION:                 return "Compound Action";
        case RB_ACTION_OPPONENT_ACTION:                 return "Opponent Action";
        case RB_ACTION_ACTION_BY:                       return "Action By";
        case RB_ACTION_SEQUENTIAL_COST:                 return "Sequential Cost";
        case RB_ACTION_CHOICE_CONDITION:                return "Choice Condition";
        case RB_ACTION_ENERGY_CONDITION:                return "Energy Condition";
        default:                                        return NULL;
    }
}

int rb_action_type_default(void) {
    return RB_ACTION_CUSTOM;
}

/* ── ConditionType helpers ── */
int rb_condition_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "compound")) return RB_CONDTYPE_COMPOUND;
    if (!strcmp(s, "comparison_condition")) return RB_CONDTYPE_COMPARISON;
    if (!strcmp(s, "location_condition")) return RB_CONDTYPE_LOCATION;
    if (!strcmp(s, "card_count_condition")) return RB_CONDTYPE_CARD_COUNT;
    if (!strcmp(s, "card_blade_condition")) return RB_CONDTYPE_CARD_BLADE;
    if (!strcmp(s, "group_condition")) return RB_CONDTYPE_GROUP;
    if (!strcmp(s, "position_condition")) return RB_CONDTYPE_POSITION;
    if (!strcmp(s, "appearance_condition")) return RB_CONDTYPE_APPEARANCE;
    if (!strcmp(s, "temporal_condition")) return RB_CONDTYPE_TEMPORAL;
    if (!strcmp(s, "state_condition")) return RB_CONDTYPE_STATE;
    if (!strcmp(s, "energy_state_condition")) return RB_CONDTYPE_ENERGY_STATE;
    if (!strcmp(s, "movement_condition")) return RB_CONDTYPE_MOVEMENT;
    if (!strcmp(s, "ability_filter_condition")) return RB_CONDTYPE_ABILITY_FILTER;
    if (!strcmp(s, "or_condition")) return RB_CONDTYPE_OR;
    if (!strcmp(s, "any_of_condition")) return RB_CONDTYPE_ANY_OF;
    if (!strcmp(s, "score_threshold_condition")) return RB_CONDTYPE_SCORE_THRESHOLD;
    if (!strcmp(s, "choice_condition")) return RB_CONDTYPE_CHOICE;
    if (!strcmp(s, "position_change_condition")) return RB_CONDTYPE_POSITION_CHANGE;
    if (!strcmp(s, "state_change_condition")) return RB_CONDTYPE_STATE_CHANGE;
    if (!strcmp(s, "opponent_choice_condition")) return RB_CONDTYPE_OPPONENT_CHOICE;
    if (!strcmp(s, "opponent_live_success")) return RB_CONDTYPE_OPPONENT_LIVE_SUCCESS;
    if (!strcmp(s, "complex_condition")) return RB_CONDTYPE_COMPLEX;
    if (!strcmp(s, "no_excess_heart")) return RB_CONDTYPE_NO_EXCESS_HEART;
    if (!strcmp(s, "otherwise_condition")) return RB_CONDTYPE_OTHERWISE;
    if (!strcmp(s, "both_condition")) return RB_CONDTYPE_BOTH;
    if (!strcmp(s, "not_moved")) return RB_CONDTYPE_NOT_MOVED;
    if (!strcmp(s, "has_moved")) return RB_CONDTYPE_HAS_MOVED;
    if (!strcmp(s, "resource_condition")) return RB_CONDTYPE_RESOURCE;
    if (!strcmp(s, "action_success_condition")) return RB_CONDTYPE_ACTION_SUCCESS;
    if (!strcmp(s, "all_cost_comparison_condition")) return RB_CONDTYPE_ALL_COST_COMPARISON;
    if (!strcmp(s, "highest_cost_on_stage_condition")) return RB_CONDTYPE_HIGHEST_COST_ON_STAGE;
    if (!strcmp(s, "all_revealed_match_heart_color")) return RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR;
    if (!strcmp(s, "custom")) return RB_CONDTYPE_CUSTOM;
    return -1;
}

const char *rb_condition_type_to_str(int ct) {
    switch (ct) {
        case RB_CONDTYPE_COMPOUND:                     return "compound";
        case RB_CONDTYPE_COMPARISON:                   return "comparison_condition";
        case RB_CONDTYPE_LOCATION:                     return "location_condition";
        case RB_CONDTYPE_CARD_COUNT:                   return "card_count_condition";
        case RB_CONDTYPE_CARD_BLADE:                   return "card_blade_condition";
        case RB_CONDTYPE_GROUP:                        return "group_condition";
        case RB_CONDTYPE_POSITION:                     return "position_condition";
        case RB_CONDTYPE_APPEARANCE:                   return "appearance_condition";
        case RB_CONDTYPE_TEMPORAL:                     return "temporal_condition";
        case RB_CONDTYPE_STATE:                        return "state_condition";
        case RB_CONDTYPE_ENERGY_STATE:                 return "energy_state_condition";
        case RB_CONDTYPE_MOVEMENT:                     return "movement_condition";
        case RB_CONDTYPE_ABILITY_FILTER:               return "ability_filter_condition";
        case RB_CONDTYPE_OR:                           return "or_condition";
        case RB_CONDTYPE_ANY_OF:                       return "any_of_condition";
        case RB_CONDTYPE_SCORE_THRESHOLD:              return "score_threshold_condition";
        case RB_CONDTYPE_CHOICE:                       return "choice_condition";
        case RB_CONDTYPE_POSITION_CHANGE:              return "position_change_condition";
        case RB_CONDTYPE_STATE_CHANGE:                 return "state_change_condition";
        case RB_CONDTYPE_OPPONENT_CHOICE:              return "opponent_choice_condition";
        case RB_CONDTYPE_OPPONENT_LIVE_SUCCESS:        return "opponent_live_success";
        case RB_CONDTYPE_COMPLEX:                      return "complex_condition";
        case RB_CONDTYPE_NO_EXCESS_HEART:              return "no_excess_heart";
        case RB_CONDTYPE_OTHERWISE:                    return "otherwise_condition";
        case RB_CONDTYPE_BOTH:                         return "both_condition";
        case RB_CONDTYPE_NOT_MOVED:                    return "not_moved";
        case RB_CONDTYPE_HAS_MOVED:                    return "has_moved";
        case RB_CONDTYPE_RESOURCE:                     return "resource_condition";
        case RB_CONDTYPE_ACTION_SUCCESS:               return "action_success_condition";
        case RB_CONDTYPE_ALL_COST_COMPARISON:           return "all_cost_comparison_condition";
        case RB_CONDTYPE_HIGHEST_COST_ON_STAGE:         return "highest_cost_on_stage_condition";
        case RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR: return "all_revealed_match_heart_color";
        case RB_CONDTYPE_CUSTOM:                       return "custom";
        default:                                       return NULL;
    }
}

/* ── SelectTargetKind helpers ── */
int rb_select_target_kind_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "choice")) return RB_STK_CHOICE;
    if (!strcmp(s, "choice_string")) return RB_STK_CHOICE_STRING;
    if (!strcmp(s, "pay_optional_cost:skip_optional_cost")) return RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST;
    if (!strcmp(s, "double_baton_touch")) return RB_STK_DOUBLE_BATON_TOUCH;
    if (!strcmp(s, "primary|alternative")) return RB_STK_PRIMARY_ALTERNATIVE;
    if (!strcmp(s, "apply_replacement")) return RB_STK_APPLY_REPLACEMENT;
    if (!strcmp(s, "choose_required_hearts")) return RB_STK_CHOOSE_REQUIRED_HEARTS;
    if (!strcmp(s, "position|destination")) return RB_STK_POSITION_DESTINATION;
    if (!strcmp(s, "heart_color")) return RB_STK_HEART_COLOR;
    if (!strcmp(s, "choice_type")) return RB_STK_CHOICE_TYPE;
    if (!strcmp(s, "choice_condition")) return RB_STK_CHOICE_CONDITION;
    if (!strcmp(s, "conditional_optional")) return RB_STK_CONDITIONAL_OPTIONAL;
    if (!strcmp(s, "draw_any_number")) return RB_STK_DRAW_ANY_NUMBER;
    if (!strcmp(s, "order")) return RB_STK_ORDER;
    if (!strcmp(s, "self_or_opponent")) return RB_STK_SELF_OR_OPPONENT;
    if (!strcmp(s, "pay_cost_all:discard_all")) return RB_STK_PAY_COST_ALL_DISCARD;
    return -1;
}

const char *rb_select_target_kind_to_str(int stk) {
    switch (stk) {
        case RB_STK_CHOICE:                       return "choice";
        case RB_STK_CHOICE_STRING:                return "choice_string";
        case RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST: return "pay_optional_cost:skip_optional_cost";
        case RB_STK_DOUBLE_BATON_TOUCH:           return "double_baton_touch";
        case RB_STK_PRIMARY_ALTERNATIVE:          return "primary|alternative";
        case RB_STK_APPLY_REPLACEMENT:            return "apply_replacement";
        case RB_STK_CHOOSE_REQUIRED_HEARTS:       return "choose_required_hearts";
        case RB_STK_POSITION_DESTINATION:         return "position|destination";
        case RB_STK_HEART_COLOR:                  return "heart_color";
        case RB_STK_CHOICE_TYPE:                  return "choice_type";
        case RB_STK_CHOICE_CONDITION:             return "choice_condition";
        case RB_STK_CONDITIONAL_OPTIONAL:         return "conditional_optional";
        case RB_STK_DRAW_ANY_NUMBER:              return "draw_any_number";
        case RB_STK_ORDER:                        return "order";
        case RB_STK_SELF_OR_OPPONENT:             return "self_or_opponent";
        case RB_STK_PAY_COST_ALL_DISCARD:         return "pay_cost_all:discard_all";
        default:                                  return NULL;
    }
}

/* ── EffectCardType helpers ── */
int rb_effect_card_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "member_card")) return RB_ECT_MEMBER_CARD;
    if (!strcmp(s, "live_card")) return RB_ECT_LIVE_CARD;
    if (!strcmp(s, "energy_card")) return RB_ECT_ENERGY_CARD;
    return RB_ECT_OTHER;
}

const char *rb_effect_card_type_as_str(int ect) {
    switch (ect) {
        case RB_ECT_MEMBER_CARD: return "member_card";
        case RB_ECT_LIVE_CARD:   return "live_card";
        case RB_ECT_ENERGY_CARD: return "energy_card";
        case RB_ECT_OTHER:       return "";
        default:                 return NULL;
    }
}

RbEffectCardType rb_effect_card_type_default(void) {
    return RB_ECT_OTHER;
}

/* ── EffectState helpers ── */
int rb_effect_state_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "active")) return RB_ES_ACTIVE;
    if (!strcmp(s, "wait")) return RB_ES_WAIT;
    return RB_ES_OTHER;
}

const char *rb_effect_state_as_str(int es) {
    switch (es) {
        case RB_ES_ACTIVE: return "active";
        case RB_ES_WAIT:   return "wait";
        case RB_ES_OTHER:  return "";
        default:           return NULL;
    }
}

RbEffectState rb_effect_state_default(void) {
    return RB_ES_OTHER;
}

/* ── AbilityZone helpers (RbAbilityZone enum declared in rabuka.h) ── */

int rb_ability_zone_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "hand")) return RB_ABILITY_ZONE_HAND;
    if (!strcmp(s, "stage")) return RB_ABILITY_ZONE_STAGE;
    if (!strcmp(s, "center")) return RB_ABILITY_ZONE_STAGE_CENTER;
    if (!strcmp(s, "left") || !strcmp(s, "left_side")) return RB_ABILITY_ZONE_STAGE_LEFT;
    if (!strcmp(s, "right") || !strcmp(s, "right_side")) return RB_ABILITY_ZONE_STAGE_RIGHT;
    if (!strcmp(s, "discard")) return RB_ABILITY_ZONE_DISCARD;
    if (!strcmp(s, "waitroom")) return RB_ABILITY_ZONE_WAITROOM;
    if (!strcmp(s, "energy") || !strcmp(s, "energy_zone")) return RB_ABILITY_ZONE_ENERGY;
    if (!strcmp(s, "deck")) return RB_ABILITY_ZONE_DECK;
    if (!strcmp(s, "deck_top")) return RB_ABILITY_ZONE_DECK_TOP;
    if (!strcmp(s, "deck_bottom")) return RB_ABILITY_ZONE_DECK_BOTTOM;
    if (!strcmp(s, "success_zone")) return RB_ABILITY_ZONE_SUCCESS_ZONE;
    if (!strcmp(s, "live_card_zone")) return RB_ABILITY_ZONE_LIVE_CARD_ZONE;
    if (!strcmp(s, "success_live_zone") || !strcmp(s, "success_live_card_zone")) return RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE;
    if (!strcmp(s, "energy_deck")) return RB_ABILITY_ZONE_ENERGY_DECK;
    if (!strcmp(s, "empty_area")) return RB_ABILITY_ZONE_EMPTY_AREA;
    if (!strcmp(s, "same_area")) return RB_ABILITY_ZONE_SAME_AREA;
    if (!strcmp(s, "under_member") || !strcmp(s, "under")) return RB_ABILITY_ZONE_UNDER_MEMBER;
    if (!strcmp(s, "looked_at")) return RB_ABILITY_ZONE_LOOKED_AT;
    if (!strcmp(s, "revealed_cards")) return RB_ABILITY_ZONE_REVEALED_CARDS;
    if (!strcmp(s, "selected_cards")) return RB_ABILITY_ZONE_SELECTED_CARDS;
    if (!strcmp(s, "resolution") || !strcmp(s, "resolution_zone")) return RB_ABILITY_ZONE_RESOLUTION;
    if (!strcmp(s, "exclusion_zone")) return RB_ABILITY_ZONE_EXCLUSION_ZONE;
    if (!strcmp(s, "preceding_moved")) return RB_ABILITY_ZONE_PRECEDING_MOVED;
    if (!strcmp(s, "recently_moved")) return RB_ABILITY_ZONE_RECENTLY_MOVED;
    if (!strcmp(s, "those_cards")) return RB_ABILITY_ZONE_THOSE_CARDS;
    if (!strcmp(s, "looked_at_remaining")) return RB_ABILITY_ZONE_LOOKED_AT_REMAINING;
    if (!strcmp(s, "deck_top_or_bottom")) return RB_ABILITY_ZONE_DECK_TOP_OR_BOTTOM;
    if (!strcmp(s, "front")) return RB_ABILITY_ZONE_FRONT;
    return RB_ABILITY_ZONE_UNKNOWN;
}

const char *rb_ability_zone_to_str(int z) {
    switch (z) {
        case RB_ABILITY_ZONE_HAND:                return "hand";
        case RB_ABILITY_ZONE_STAGE:               return "stage";
        case RB_ABILITY_ZONE_STAGE_CENTER:        return "center";
        case RB_ABILITY_ZONE_STAGE_LEFT:          return "left";
        case RB_ABILITY_ZONE_STAGE_RIGHT:         return "right";
        case RB_ABILITY_ZONE_DISCARD:             return "discard";
        case RB_ABILITY_ZONE_WAITROOM:            return "waitroom";
        case RB_ABILITY_ZONE_ENERGY:              return "energy";
        case RB_ABILITY_ZONE_ENERGY_ZONE:         return "energy_zone";
        case RB_ABILITY_ZONE_DECK:                return "deck";
        case RB_ABILITY_ZONE_DECK_TOP:            return "deck_top";
        case RB_ABILITY_ZONE_DECK_BOTTOM:         return "deck_bottom";
        case RB_ABILITY_ZONE_SUCCESS_ZONE:        return "success_zone";
        case RB_ABILITY_ZONE_LIVE_CARD_ZONE:      return "live_card_zone";
        case RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE:   return "success_live_zone";
        case RB_ABILITY_ZONE_ENERGY_DECK:         return "energy_deck";
        case RB_ABILITY_ZONE_EMPTY_AREA:          return "empty_area";
        case RB_ABILITY_ZONE_SAME_AREA:           return "same_area";
        case RB_ABILITY_ZONE_UNDER_MEMBER:        return "under_member";
        case RB_ABILITY_ZONE_LOOKED_AT:           return "looked_at";
        case RB_ABILITY_ZONE_REVEALED_CARDS:      return "revealed_cards";
        case RB_ABILITY_ZONE_SELECTED_CARDS:      return "selected_cards";
        case RB_ABILITY_ZONE_RESOLUTION:          return "resolution";
        case RB_ABILITY_ZONE_EXCLUSION_ZONE:      return "exclusion_zone";
        case RB_ABILITY_ZONE_PRECEDING_MOVED:     return "preceding_moved";
        case RB_ABILITY_ZONE_RECENTLY_MOVED:      return "recently_moved";
        case RB_ABILITY_ZONE_THOSE_CARDS:         return "those_cards";
        case RB_ABILITY_ZONE_LOOKED_AT_REMAINING: return "looked_at_remaining";
        case RB_ABILITY_ZONE_DECK_TOP_OR_BOTTOM:  return "deck_top_or_bottom";
        case RB_ABILITY_ZONE_FRONT:               return "front";
        case RB_ABILITY_ZONE_UNKNOWN:             return "unknown";
        default:                                   return NULL;
    }
}

const char *rb_ability_zone_as_str(int z) {
    return rb_ability_zone_to_str(z);
}

int rb_ability_zone_from_source_str(const char *s) {
    int z = rb_ability_zone_from_str(s);
    return (z < 0) ? RB_ABILITY_ZONE_UNKNOWN : z;
}
