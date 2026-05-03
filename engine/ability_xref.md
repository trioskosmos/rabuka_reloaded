# Ability Cross-Reference Report

**Total unique abilities:** 602
**Cards:** 1806 total, 1057 with abilities, 602 unique

## 1. Triggers
| Value | Engine Location |
|-------|-----------------|
| `ライブ成功時` | `engine/src/triggers.rs:6` |
| `ライブ開始時` | `engine/src/triggers.rs:5` |
| `ライブ開始時, 右サイド` | `engine/src/triggers.rs:5`, 右サイド |
| `ライブ開始時, 左サイド` | `engine/src/triggers.rs:5`, 左サイド |
| `ライブ開始時, 登場` | `engine/src/triggers.rs:5`, 登場 |
| `常時` | `engine/src/triggers.rs:3` |
| `常時, 右サイド` | `engine/src/triggers.rs:3`, 右サイド |
| `常時, 左サイド` | `engine/src/triggers.rs:3`, 左サイド |
| `登場` | `engine/src/triggers.rs:4` |
| `登場, 右サイド` | `engine/src/triggers.rs:4`, 右サイド |
| `登場, 左サイド` | `engine/src/triggers.rs:4`, 左サイド |
| `登場, 左サイド, 右サイド` | `engine/src/triggers.rs:4`, 左サイド, 右サイド |
| `自動` | `engine/src/triggers.rs:2` |
| `起動` | `engine/src/triggers.rs:1` |
| `起動, 左サイド` | `engine/src/triggers.rs:1`, 左サイド |

## 2. Keywords
| Keyword | Struct | Evaluator |
|---------|--------|-----------|

## 3. Cost Types
| Cost Type | Engine Handler |
|-----------|----------------|
| `change_state` | `engine/src/ability/cost.rs:148-174` |
| `choice_condition` | `engine/src/ability/cost.rs:19-27` |
| `energy_condition` | `engine/src/ability/cost.rs:45-52, 209-223` |
| `move_cards` | `engine/src/ability/cost.rs:29-43, 88-147` |
| `pay_energy` | `engine/src/ability/cost.rs:175-208` |
| `place_energy_under_member` | `engine/src/ability/cost.rs:235-244` |
| `reveal` | `engine/src/ability/cost.rs:224-234` |
| `sequential_cost` | `engine/src/ability/cost.rs:10-17` |

## 4. Cost Fields (`AbilityCost` struct)
| Field | Engine Location | Types | Status |
|-------|-----------------|-------|--------|
| `action` | `engine/src/card.rs:439` | str | used |
| `card_type` | `engine/src/card.rs:437` | str | used |
| `characters` | `engine/src/card.rs:455` | list | used |
| `costs` | `engine/src/card.rs:451` | list | used |
| `count` | `engine/src/card.rs:436` | int | used |
| `destination` | `engine/src/card.rs:435` | str | used |
| `energy` | `engine/src/card.rs:441` | int | used |
| `exclude_self` | `engine/src/card.rs:449` | bool | used |
| `group_names` | *not in struct* | list | used |
| `optional` | `engine/src/card.rs:440` | bool | used |
| `options` | `engine/src/card.rs:445` | list | used |
| `self_cost` | `engine/src/card.rs:447` | bool | used |
| `shuffle` | *not in struct* | bool | used |
| `source` | `engine/src/card.rs:434` | str | used |
| `state_change` | `engine/src/card.rs:442` | str | used |
| `target` | `engine/src/card.rs:438` | str | used |
| `text` | `engine/src/card.rs:431` | str | used |
| `type` | `engine/src/card.rs:433` | str | used |
| `cost_limit` | `engine/src/card.rs:453` | -- | *unused* |
| `cost_type` | `engine/src/card.rs:433` | -- | *unused* |
| `position` | `engine/src/card.rs:443` | -- | *unused* |

## 5. Actions (effect dispatch)
| Action | Handler in `effects.rs` |
|--------|------------------------|
| `activate_ability` | `engine/src/ability/effects.rs:82, 866-872` |
| `appear` | `engine/src/ability/effects.rs:94, 1107-1158` |
| `change_state` | `engine/src/ability/effects.rs:76, 537-695` |
| `choice` | `engine/src/ability/effects.rs:95, 1160-1192` |
| `conditional_alternative` | `engine/src/ability/effects.rs:70` |
| `draw_card` | `engine/src/ability/effects.rs:72` |
| `draw_until_count` | `engine/src/ability/effects.rs:73, 343-356` |
| `gain_ability` | `engine/src/ability/effects.rs:84, 881-898` |
| `gain_resource` | `engine/src/ability/effects.rs:75, 419-535` |
| `look_and_select` | `engine/src/ability/effects.rs:71` |
| `modify_cost` | `engine/src/ability/effects.rs:102, 1479-1495` |
| `modify_score` | `engine/src/ability/effects.rs:77, 698-779` |
| `modify_yell_count` | `engine/src/ability/effects.rs:90, 1014-1024` |
| `move_cards` | `engine/src/ability/effects.rs:74` |
| `place_energy_under_member` | `engine/src/ability/effects.rs:91, 358-387` |
| `play_baton_touch` | `engine/src/ability/effects.rs:85, 900-906` |
| `position_change` | `engine/src/ability/effects.rs:93, 1026-1071` |
| `restriction` | `engine/src/ability/effects.rs:100, 1240-1244` |
| `sequential` | `engine/src/ability/effects.rs:69` |
| `set_blade_count` | `engine/src/ability/effects.rs:106, 1297-1311` |
| `set_card_identity` | `engine/src/ability/effects.rs:97, 1202-1209` |
| `set_score` | `engine/src/ability/effects.rs:109, 1328-1334` |

## 6. Effect Fields (`AbilityEffect` struct)
| Field | Engine Location | Types | Status |
|-------|-----------------|-------|--------|
| `action` | `engine/src/card.rs:463` | str | used |
| `action_by` | `engine/src/card.rs:556` | str | used |
| `actions` | `engine/src/card.rs:473` | list | used |
| `activation_condition` | `engine/src/card.rs:526` | str | used |
| `activation_condition_parsed` | `engine/src/card.rs:527` | dict | used |
| `activation_position` | `engine/src/card.rs:570` | str | used |
| `all_regions` | `engine/src/card.rs:572` | bool | used |
| `alternative_effect` | `engine/src/card.rs:487` | dict | used |
| `card_type` | `engine/src/card.rs:468` | str | used |
| `character_effects` | `engine/src/card.rs:574` | list | used |
| `choice_type` | `engine/src/card.rs:565` | str | used |
| `condition` | `engine/src/card.rs:484` | dict | used |
| `conditional` | `engine/src/card.rs:563` | bool | used |
| `cost_limit` | `engine/src/card.rs:517` | int | used |
| `count` | `engine/src/card.rs:466` | int | used |
| `destination` | `engine/src/card.rs:465` | str, NoneType | used |
| `distinct` | `engine/src/card.rs:520` | str | used |
| `duration` | `engine/src/card.rs:470` | str | used |
| `dynamic_count` | `engine/src/card.rs:515` | dict | used |
| `effect_constraint` | `engine/src/card.rs:479` | str | used |
| `energy_count` | `engine/src/card.rs:502` | int | used |
| `exclude_self` | `engine/src/card.rs:534` | bool | used |
| `group` | `engine/src/card.rs:507` | dict | used |
| `group_names` | `engine/src/card.rs:576` | list | used |
| `heart_colors` | `engine/src/card.rs:500` | list | used |
| `heart_selection` | `engine/src/card.rs:578` | bool | used |
| `identities` | `engine/src/card.rs:553` | list | used |
| `location` | `engine/src/card.rs:580` | str | used |
| `look_action` | `engine/src/card.rs:471` | dict | used |
| `max` | `engine/src/card.rs:478` | bool | used |
| `multiple_targets` | `engine/src/card.rs:582` | bool | used |
| `name_constraint` | `engine/src/card.rs:523` | str | used |
| `name_constraint_source` | `engine/src/card.rs:525` | str | used |
| `operation` | `engine/src/card.rs:496` | str, NoneType | used |
| `opponent_action` | `engine/src/card.rs:558` | dict | used |
| `optional` | `engine/src/card.rs:477` | bool | used |
| `options` | `engine/src/card.rs:506` | list | used |
| `parenthetical` | *not in struct* | list | used |
| `per_unit` | `engine/src/card.rs:483` | bool | used |
| `per_unit_count` | `engine/src/card.rs:508` | int | used |
| `per_unit_type` | `engine/src/card.rs:509` | str | used |
| `placement_order` | `engine/src/card.rs:516` | str | used |
| `position` | `engine/src/card.rs:475` | str | used |
| `primary_effect` | `engine/src/card.rs:485` | dict | used |
| `question` | `engine/src/card.rs:584` | str | used |
| `quoted_text` | `engine/src/card.rs:482` | dict | used |
| `resource` | `engine/src/card.rs:474` | str | used |
| `restriction_type` | `engine/src/card.rs:513` | str | used |
| `select_action` | `engine/src/card.rs:472` | dict | used |
| `source` | `engine/src/card.rs:464` | str | used |
| `state` | `engine/src/card.rs:586` | str | used |
| `state_change` | `engine/src/card.rs:476` | str | used |
| `target` | `engine/src/card.rs:469` | str, NoneType | used |
| `target_count` | `engine/src/card.rs:467` | int | used |
| `target_trigger` | `engine/src/card.rs:588` | str | used |
| `text` | `engine/src/card.rs:461` | str | used |
| `timing_condition` | `engine/src/card.rs:590` | str | used |
| `trigger_type` | `engine/src/card.rs:592` | str | used |
| `value` | `engine/src/card.rs:497` | int | used |
| `ability_gain` | `engine/src/card.rs:481` | -- | *unused* |
| `ability_text` | `engine/src/card.rs:528` | -- | *unused* |
| `alternative_condition` | `engine/src/card.rs:486` | -- | *unused* |
| `any_number` | `engine/src/card.rs:519` | -- | *unused* |
| `blade_type` | `engine/src/card.rs:501` | -- | *unused* |
| `choice` | `engine/src/card.rs:540` | -- | *unused* |
| `choice_based` | `engine/src/card.rs:550` | -- | *unused* |
| `choice_options` | `engine/src/card.rs:505` | -- | *unused* |
| `conditional_action` | `engine/src/card.rs:495` | -- | *unused* |
| `effect_type` | `engine/src/card.rs:537` | -- | *unused* |
| `followup_action` | `engine/src/card.rs:491` | -- | *unused* |
| `heart_color` | `engine/src/card.rs:499` | -- | *unused* |
| `heart_type` | `engine/src/card.rs:567` | -- | *unused* |
| `is_further` | `engine/src/card.rs:512` | -- | *unused* |
| `lose_blade_hearts` | `engine/src/card.rs:561` | -- | *unused* |
| `optional_action` | `engine/src/card.rs:493` | -- | *unused* |
| `repeat_limit` | `engine/src/card.rs:511` | -- | *unused* |
| `replaces_event` | `engine/src/card.rs:548` | -- | *unused* |
| `resource_icon_count` | `engine/src/card.rs:480` | -- | *unused* |
| `restricted_destination` | `engine/src/card.rs:514` | -- | *unused* |
| `result_condition` | `engine/src/card.rs:489` | -- | *unused* |
| `self_cost` | `engine/src/card.rs:532` | -- | *unused* |
| `target_member` | `engine/src/card.rs:503` | -- | *unused* |
| `timing` | `engine/src/card.rs:543` | -- | *unused* |
| `treat_as` | `engine/src/card.rs:545` | -- | *unused* |
| `triggers` | `engine/src/card.rs:530` | -- | *unused* |
| `use_limit` | `engine/src/card.rs:529` | -- | *unused* |

## 7. Condition Types
| Condition Type | Engine Handler |
|----------------|----------------|
| `ability_negation_condition` | `engine/src/ability/condition.rs:21, 383-388` |
| `appearance_condition` | `engine/src/ability/condition.rs:16, 244-273` |
| `card_count_condition` | `engine/src/ability/condition.rs:15, 229-242` |
| `comparison_condition` | `engine/src/ability/condition.rs:11, 56-75` |
| `complex_condition` | `engine/src/ability/condition.rs:30, 464-470` |
| `compound` | `engine/src/ability/condition.rs:10, 44-54` |
| `energy_state_condition` | `engine/src/ability/condition.rs:19, 342-350` |
| `group_condition` | `engine/src/ability/condition.rs:14, 222-227` |
| `location_condition` | `engine/src/ability/condition.rs:12, 77-207` |
| `movement_condition` | `engine/src/ability/condition.rs:20, 352-381` |
| `opponent_choice_condition` | `engine/src/ability/condition.rs:28, 453-458` |
| `or_condition` | `engine/src/ability/condition.rs:22, 390-394` |
| `position_change_condition` | `engine/src/ability/condition.rs:26, 433-442` |
| `position_condition` | `engine/src/ability/condition.rs:13, 209-220` |
| `score_threshold_condition` | `engine/src/ability/condition.rs:24, 416-423` |
| `state_change_condition` | `engine/src/ability/condition.rs:27, 444-451` |
| `state_condition` | `engine/src/ability/condition.rs:18, 332-340` |
| `temporal_condition` | `engine/src/ability/condition.rs:17, 275-330` |

## 8. Condition Fields (`Condition` struct)
| Field | Engine Location | Types | Status |
|-------|-----------------|-------|--------|
| `action` | `engine/src/card.rs:727` | str | used |
| `aggregate` | *not in struct* | str | used |
| `all_areas` | `engine/src/card.rs:715` | bool | used |
| `appearance` | `engine/src/card.rs:708` | bool | used |
| `baton_touch_source` | `engine/src/card.rs:700` | str | used |
| `baton_touch_trigger` | `engine/src/card.rs:699` | bool | used |
| `card_property` | `engine/src/card.rs:713` | str | used |
| `card_type` | `engine/src/card.rs:685` | str | used |
| `cause` | `engine/src/card.rs:722` | dict | used |
| `comparison_target` | `engine/src/card.rs:703` | str | used |
| `comparison_type` | `engine/src/card.rs:707` | str | used |
| `condition` | `engine/src/card.rs:712` | dict | used |
| `conditions` | `engine/src/card.rs:709` | list | used |
| `cost_limit` | `engine/src/card.rs:696` | int | used |
| `count` | `engine/src/card.rs:683` | int | used |
| `destination` | `engine/src/card.rs:729` | str | used |
| `distinct` | `engine/src/card.rs:693` | bool | used |
| `effect` | `engine/src/card.rs:724` | dict | used |
| `exclude_self` | `engine/src/card.rs:694` | bool | used |
| `from_state` | `engine/src/card.rs:731` | str | used |
| `group` | `engine/src/card.rs:687` | dict | used |
| `heart_type` | `engine/src/card.rs:733` | str | used |
| `location` | `engine/src/card.rs:682` | str | used |
| `movement` | `engine/src/card.rs:704` | str | used |
| `movement_condition` | `engine/src/card.rs:698` | str | used |
| `movement_state` | `engine/src/card.rs:701` | str | used |
| `negation` | `engine/src/card.rs:697` | bool | used |
| `operator` | `engine/src/card.rs:684` | str | used |
| `optional` | `engine/src/card.rs:735` | bool | used |
| `phase` | `engine/src/card.rs:706` | str | used |
| `resource_type` | `engine/src/card.rs:717` | str | used |
| `source` | `engine/src/card.rs:737` | str | used |
| `state` | `engine/src/card.rs:690` | str | used |
| `target` | `engine/src/card.rs:686` | str | used |
| `temporal` | `engine/src/card.rs:705` | str | used |
| `temporal_scope` | `engine/src/card.rs:692` | str | used |
| `text` | `engine/src/card.rs:679` | str | used |
| `to_state` | `engine/src/card.rs:739` | str | used |
| `type` | `engine/src/card.rs:681` | str | used |
| `unit` | `engine/src/card.rs:718` | str | used |
| `values` | `engine/src/card.rs:719` | list | used |
| `any_of` | `engine/src/card.rs:695` | -- | *unused* |
| `characters` | `engine/src/card.rs:689` | -- | *unused* |
| `condition_type` | `engine/src/card.rs:681` | -- | *unused* |
| `energy_state` | `engine/src/card.rs:702` | -- | *unused* |
| `group_names` | `engine/src/card.rs:688` | -- | *unused* |
| `no_excess_heart` | `engine/src/card.rs:716` | -- | *unused* |
| `options` | `engine/src/card.rs:710` | -- | *unused* |
| `position` | `engine/src/card.rs:691` | -- | *unused* |

## 9. Duration Values
| Duration | Used In |
|----------|---------|
| `as_long_as` | `ability/effects.rs` (various handlers) |
| `live_end` | `ability/effects.rs` (various handlers) |
| `this_turn` | `ability/effects.rs` (various handlers) |

## 10. Move Cards Routes
| Route | Location |
|-------|----------|
| `deck→deck_bottom` | `engine/src/ability/move_cards.rs:83` |
| `deck→deck_top` | `engine/src/ability/move_cards.rs:82` |
| `deck→discard` | `engine/src/ability/move_cards.rs:73` |
| `deck→energy_deck` | `engine/src/ability/move_cards.rs:81` |
| `deck→energy_zone` | `engine/src/ability/move_cards.rs:80` |
| `deck→hand` | `engine/src/ability/move_cards.rs:72` |
| `deck→live_card_zone` | `engine/src/ability/move_cards.rs:78` |
| `deck→stage` | `engine/src/ability/move_cards.rs:74-77` |
| `deck→success_live_zone` | `engine/src/ability/move_cards.rs:79` |
| `discard→deck` | `engine/src/ability/move_cards.rs:195-215` |
| `discard→deck_bottom/deck_top` | `engine/src/ability/move_cards.rs:189-194` |
| `discard→hand` | `engine/src/ability/move_cards.rs:183-188` |
| `discard→live_card_zone` | `engine/src/ability/move_cards.rs:216-230` |
| `discard→same_area` | `engine/src/ability/move_cards.rs:231-249` |
| `discard→stage/empty_area` | `engine/src/ability/move_cards.rs:250-291` |
| `energy_zone→discard` | `engine/src/ability/move_cards.rs:294-295` |
| `energy_zone→hand` | `engine/src/ability/move_cards.rs:294-295` |
| `hand→deck_bottom/deck_top` | `engine/src/ability/move_cards.rs:157-162` |
| `hand→discard` | `engine/src/ability/move_cards.rs:151-155` |
| `hand→live_card_zone` | `engine/src/ability/move_cards.rs:176-180` |
| `hand→stage` | `engine/src/ability/move_cards.rs:163-175` |
| `live_card_zone→discard` | `engine/src/ability/move_cards.rs:305` |
| `live_card_zone→hand` | `engine/src/ability/move_cards.rs:305` |
| `live_card_zone→success_live_zone` | `engine/src/ability/move_cards.rs:305` |
| `stage→deck_bottom` | `engine/src/ability/move_cards.rs:104` |
| `stage→deck_top` | `engine/src/ability/move_cards.rs:105` |
| `stage→discard` | `engine/src/ability/move_cards.rs:102` |
| `stage→hand` | `engine/src/ability/move_cards.rs:103` |
| `stage→live_card_zone` | `engine/src/ability/move_cards.rs:110` |
| `stage→same_area` | `engine/src/ability/move_cards.rs:106-109` |
| `stage→success_live_zone` | `engine/src/ability/move_cards.rs:111` |
| `success_live_zone→deck_bottom` | `engine/src/ability/move_cards.rs:314` |
| `success_live_zone→deck_top` | `engine/src/ability/move_cards.rs:314` |
| `success_live_zone→hand` | `engine/src/ability/move_cards.rs:314` |

## 11. Utility Functions
| Function | Location |
|----------|----------|
| `card_matches_all_filters` | `engine/src/ability/util.rs:79-96` |
| `card_matches_characters` | `engine/src/ability/util.rs:28-37` |
| `card_matches_cost_limit` | `engine/src/ability/util.rs:39-41` |
| `card_matches_cost_limit_op` | `engine/src/ability/util.rs:43-56` |
| `card_matches_group` | `engine/src/ability/util.rs:14-19` |
| `card_matches_group_str` | `engine/src/ability/util.rs:21-26` |
| `card_matches_heart_colors` | `engine/src/ability/util.rs:58-70` |
| `card_matches_name_constraint` | `engine/src/ability/util.rs:72-77` |
| `card_matches_type` | `engine/src/ability/util.rs:4-12` |
| `compare_counts` | `engine/src/ability/util.rs:126-136` |
| `count_matching` | `engine/src/ability/util.rs:98-110` |
| `matching_indices` | `engine/src/ability/util.rs:112-124` |
| `sum_score_in_zone` | `engine/src/ability/util.rs:146-151` |
| `zone_card_count` | `engine/src/ability/util.rs:138-144` |

## 12. Top-Level Ability Struct Fields
| Field | Location |
|-------|----------|
| `cost` | `engine/src/card.rs:419` |
| `effect` | `engine/src/card.rs:420` |
| `full_text` | `engine/src/card.rs:412` |
| `is_null` | `engine/src/card.rs:418` |
| `keywords` | `engine/src/card.rs:421` |
| `triggerless_text` | `engine/src/card.rs:414` |
| `triggers` | `engine/src/card.rs:415` |
| `use_limit` | `engine/src/card.rs:416` |

---
# Data Values in abilities.json → Engine Code

This section shows the actual **values** found in abilities.json fields and where each value is handled in the engine source code.


## Effect Field Values → Engine Code
| Field | Value in abilities.json | Engine Handler |
|-------|------------------------|----------------|
| `action` | `activate_ability` | `engine/src/ability/effects.rs:82, 866-872` |
| `action` | `appear` | `engine/src/ability/effects.rs:94, 1107-1158` |
| `action` | `change_state` | `engine/src/ability/effects.rs:76, 537-695` |
| `action` | `choice` | `engine/src/ability/effects.rs:95, 1160-1192` |
| `action` | `conditional_alternative` | `engine/src/ability/effects.rs:70` |
| `action` | `draw_card` | `engine/src/ability/effects.rs:72` |
| `action` | `draw_until_count` | `engine/src/ability/effects.rs:73, 343-356` |
| `action` | `gain_ability` | `engine/src/ability/effects.rs:84, 881-898` |
| `action` | `gain_resource` | `engine/src/ability/effects.rs:75, 419-535` |
| `action` | `look_and_select` | `engine/src/ability/effects.rs:71` |
| `action` | `modify_cost` | `engine/src/ability/effects.rs:102, 1479-1495` |
| `action` | `modify_score` | `engine/src/ability/effects.rs:77, 698-779` |
| `action` | `modify_yell_count` | `engine/src/ability/effects.rs:90, 1014-1024` |
| `action` | `move_cards` | `engine/src/ability/effects.rs:74` |
| `action` | `place_energy_under_member` | `engine/src/ability/effects.rs:91, 358-387` |
| `action` | `play_baton_touch` | `engine/src/ability/effects.rs:85, 900-906` |
| `action` | `position_change` | `engine/src/ability/effects.rs:93, 1026-1071` |
| `action` | `restriction` | `engine/src/ability/effects.rs:100, 1240-1244` |
| `action` | `sequential` | `engine/src/ability/effects.rs:69` |
| `action` | `set_blade_count` | `engine/src/ability/effects.rs:106, 1297-1311` |
| `action` | `set_card_identity` | `engine/src/ability/effects.rs:97, 1202-1209` |
| `action` | `set_score` | `engine/src/ability/effects.rs:109, 1328-1334` |
| `card_type` | `card` | ⚠ not found: `card` |
| `card_type` | `energy_card` | `engine/src/ability/util.rs:8` |
| `card_type` | `live_card` | `engine/src/ability/util.rs:6` |
| `card_type` | `member_card` | `engine/src/ability/util.rs:7` |
| `destination` | `deck_bottom` | `engine/src/ability/move_cards.rs:83, 104, 161, 193, 320` |
| `destination` | `deck_top` | `engine/src/ability/move_cards.rs:82, 105, 162, 320` |
| `destination` | `discard` | `engine/src/ability/move_cards.rs:73, 102, 137, 155, 295, 300, 311` |
| `destination` | `empty_area` | `engine/src/ability/move_cards.rs:250-291` |
| `destination` | `energy_zone` | `engine/src/ability/move_cards.rs:80` |
| `destination` | `hand` | `engine/src/ability/move_cards.rs:72, 103, 113, 137, 155, 161, 187, 233, 300, 311, 320` |
| `destination` | `live_card_zone` | `engine/src/ability/move_cards.rs:78, 110, 141, 179, 216-230, 305` |
| `destination` | `same_area` | `engine/src/ability/move_cards.rs:106-109, 231-249` |
| `destination` | `stage` | `engine/src/ability/move_cards.rs:74-77, 112, 138, 163-175, 240-247, 250-291` |
| `destination` | `success_live_zone` | `engine/src/ability/move_cards.rs:79, 111, 142, 311, 314` |
| `destination` | `under_member` | ⚠ not found: `under_member` |
| `operation` | `decrease` | `engine/src/ability/effects.rs:792-793 (modify_required_hearts), 1367` |
| `operation` | `increase` | `engine/src/ability/effects.rs:1008-1009 (modify_required_hearts_global), 1367 (modify_required_hearts_success)` |
| `operation` | `subtract` | `engine/src/ability/effects.rs:1019 (modify_yell_count), 1489 (modify_cost)` |
| `position` | `center` | `engine/src/ability/effects.rs:1048; condition.rs:214` |
| `resource` | `blade` | `engine/src/ability/effects.rs:496-513` |
| `resource` | `heart` | `engine/src/ability/effects.rs:515-520` |
| `source` | `deck` | `engine/src/ability/move_cards.rs:58-89` |
| `source` | `deck_top` | `engine/src/ability/move_cards.rs:58-89` |
| `source` | `discard` | `engine/src/ability/move_cards.rs:183-291` |
| `source` | `hand` | `engine/src/ability/move_cards.rs:151-180` |
| `source` | `revealed_cards` | ⚠ not found: `revealed_cards` |
| `source` | `success_live_zone` | `engine/src/ability/move_cards.rs:314-323` |
| `state_change` | `active` | `engine/src/ability/effects.rs:567-568, 686-693` |
| `state_change` | `wait` | `engine/src/ability/effects.rs:566-568, 677-684` |
| `target` | `both` | `engine/src/ability/effects.rs:283-288 (draw_card target both)` |
| `target` | `opponent` | `engine/src/ability/effects.rs:target='opponent' in change_state, modify_score, etc.` |
| `target` | `self` | `engine/src/ability/effects.rs:target='self' in draw (274), modify_score (701), etc.` |
| `target` | `これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力` | ⚠ not found: `これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力` |
| `target` | `相手のライブカード置き場にあるすべてのライブカード` | ⚠ not found: `相手のライブカード置き場にあるすべてのライブカード` |
| `target` | `相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚` | ⚠ not found: `相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚` |

## Cost Field Values → Engine Code
| Field | Value in abilities.json | Engine Handler |
|-------|------------------------|----------------|
| `card_type` | `live_card` | `engine/src/ability/util.rs:6` |
| `card_type` | `member_card` | `engine/src/ability/util.rs:7` |
| `destination` | `deck_bottom` | `engine/src/ability/move_cards.rs:83, 104, 161, 193, 320` |
| `destination` | `discard` | `engine/src/ability/move_cards.rs:73, 102, 137, 155, 295, 300, 311` |
| `destination` | `energy_deck` | `engine/src/ability/move_cards.rs:81` |
| `destination` | `under_member` | ⚠ not found: `under_member` |
| `source` | `deck_top` | `engine/src/ability/move_cards.rs:58-89` |
| `source` | `discard` | `engine/src/ability/move_cards.rs:183-291` |
| `source` | `hand` | `engine/src/ability/move_cards.rs:151-180` |
| `source` | `stage` | `engine/src/ability/move_cards.rs:92-148` |
| `state_change` | `wait` | `engine/src/ability/effects.rs:566-568, 677-684` |
| `target` | `self` | `engine/src/ability/effects.rs:target='self' in draw (274), modify_score (701), etc.` |
| `type` | `change_state` | `engine/src/ability/cost.rs:148-174` |
| `type` | `choice_condition` | `engine/src/ability/cost.rs:19-27` |
| `type` | `energy_condition` | `engine/src/ability/cost.rs:45-52, 209-223` |
| `type` | `move_cards` | `engine/src/ability/cost.rs:29-43, 88-147` |
| `type` | `pay_energy` | `engine/src/ability/cost.rs:175-208` |
| `type` | `place_energy_under_member` | `engine/src/ability/cost.rs:235-244` |
| `type` | `reveal` | `engine/src/ability/cost.rs:224-234` |
| `type` | `sequential_cost` | `engine/src/ability/cost.rs:10-17` |

## Condition Field Values → Engine Code
| Field | Value in abilities.json | Engine Handler |
|-------|------------------------|----------------|
| `card_type` | `live_card` | `engine/src/ability/util.rs:6` |
| `card_type` | `member_card` | `engine/src/ability/util.rs:7` |
| `operator` | `<` | `engine/src/ability/util.rs:131` |
| `operator` | `=` | `engine/src/ability/util.rs:132` |
| `operator` | `>` | `engine/src/ability/util.rs:129` |
| `operator` | `>=` | `engine/src/ability/util.rs:128` |
| `operator` | `and` | ⚠ not found: `and` |
| `target` | `both` | `engine/src/ability/effects.rs:283-288 (draw_card target both)` |
| `target` | `either` | `engine/src/ability/condition.rs:121-151 (location condition either)` |
| `target` | `opponent` | `engine/src/ability/effects.rs:target='opponent' in change_state, modify_score, etc.` |
| `target` | `self` | `engine/src/ability/effects.rs:target='self' in draw (274), modify_score (701), etc.` |
| `type` | `ability_negation_condition` | `engine/src/ability/condition.rs:21, 383-388` |
| `type` | `appearance_condition` | `engine/src/ability/condition.rs:16, 244-273` |
| `type` | `card_count_condition` | `engine/src/ability/condition.rs:15, 229-242` |
| `type` | `comparison_condition` | `engine/src/ability/condition.rs:11, 56-75` |
| `type` | `complex_condition` | `engine/src/ability/condition.rs:30, 464-470` |
| `type` | `compound` | `engine/src/ability/condition.rs:10, 44-54` |
| `type` | `energy_state_condition` | `engine/src/ability/condition.rs:19, 342-350` |
| `type` | `group_condition` | `engine/src/ability/condition.rs:14, 222-227` |
| `type` | `location_condition` | `engine/src/ability/condition.rs:12, 77-207` |
| `type` | `movement_condition` | `engine/src/ability/condition.rs:20, 352-381` |
| `type` | `opponent_choice_condition` | `engine/src/ability/condition.rs:28, 453-458` |
| `type` | `or_condition` | `engine/src/ability/condition.rs:22, 390-394` |
| `type` | `position_change_condition` | `engine/src/ability/condition.rs:26, 433-442` |
| `type` | `position_condition` | `engine/src/ability/condition.rs:13, 209-220` |
| `type` | `score_threshold_condition` | `engine/src/ability/condition.rs:24, 416-423` |
| `type` | `state_change_condition` | `engine/src/ability/condition.rs:27, 444-451` |
| `type` | `state_condition` | `engine/src/ability/condition.rs:18, 332-340` |
| `type` | `temporal_condition` | `engine/src/ability/condition.rs:17, 275-330` |

---
# Implementation Audit

## Legend
| Status | Meaning |
|--------|---------|
| `IMPLEMENTED` | Field is read by engine logic (`.as_ref()`, `.unwrap()`, match, etc.) |
| `DEAD` | Field exists in the Rust struct but no engine code reads it — likely leftover from refactoring |
| `PARSER_ONLY` | Field is only emitted by `parser.py` — no corresponding Rust struct field exists |

## AbilityEffect — Implementation Status
| Field | In abilities.json | In Rust struct | Engine reads it | Status |
|-------|:-:|:-:|:-:|--------|
| `ability_gain` |  | ✓ | ✓ | IMPLEMENTED |
| `ability_text` |  | ✓ | ✓ | IMPLEMENTED |
| `action` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `action_by` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `actions` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `activation_condition` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `activation_condition_parsed` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `activation_position` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `all_regions` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `alternative_condition` |  | ✓ | ✓ | IMPLEMENTED |
| `alternative_effect` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `any_number` |  | ✓ | ✓ | IMPLEMENTED |
| `blade_type` |  | ✓ | ✓ | IMPLEMENTED |
| `card_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `character_effects` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `choice` |  | ✓ | ✓ | IMPLEMENTED |
| `choice_based` |  | ✓ | ✓ | IMPLEMENTED |
| `choice_options` |  | ✓ | ✓ | IMPLEMENTED |
| `choice_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `condition` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `conditional` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `conditional_action` |  | ✓ | ✓ | IMPLEMENTED |
| `cost_limit` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `destination` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `distinct` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `duration` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `dynamic_count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `effect_constraint` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `effect_type` |  | ✓ | ✓ | IMPLEMENTED |
| `energy_count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `exclude_self` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `followup_action` |  | ✓ | ✓ | IMPLEMENTED |
| `group` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `group_names` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `heart_color` |  | ✓ | ✓ | IMPLEMENTED |
| `heart_colors` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `heart_selection` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `heart_type` |  | ✓ | ✓ | IMPLEMENTED |
| `identities` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `is_further` |  | ✓ | ✓ | IMPLEMENTED |
| `location` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `look_action` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `lose_blade_hearts` |  | ✓ | ✓ | IMPLEMENTED |
| `max` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `multiple_targets` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `name_constraint` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `name_constraint_source` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `operation` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `opponent_action` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `optional` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `optional_action` |  | ✓ | ✓ | IMPLEMENTED |
| `options` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `per_unit` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `per_unit_count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `per_unit_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `placement_order` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `position` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `primary_effect` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `question` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `quoted_text` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `repeat_limit` |  | ✓ | ✓ | IMPLEMENTED |
| `replaces_event` |  | ✓ | ✓ | IMPLEMENTED |
| `resource` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `resource_icon_count` |  | ✓ | ✓ | IMPLEMENTED |
| `restricted_destination` |  | ✓ | ✓ | IMPLEMENTED |
| `restriction_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `result_condition` |  | ✓ | ✓ | IMPLEMENTED |
| `select_action` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `self_cost` |  | ✓ | ✓ | IMPLEMENTED |
| `source` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `state` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `state_change` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `target` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `target_count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `target_member` |  | ✓ | ✓ | IMPLEMENTED |
| `target_trigger` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `text` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `timing` |  | ✓ | ✓ | IMPLEMENTED |
| `timing_condition` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `treat_as` |  | ✓ | ✓ | IMPLEMENTED |
| `trigger_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `triggers` |  | ✓ | ✓ | IMPLEMENTED |
| `type` |  |  | ✓ | IMPLEMENTED |
| `use_limit` |  | ✓ | ✓ | IMPLEMENTED |
| `value` | ✓ | ✓ | ✓ | IMPLEMENTED |

## Condition — Implementation Status
| Field | In abilities.json | In Rust struct | Engine reads it | Status |
|-------|:-:|:-:|:-:|--------|
| `action` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `all_areas` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `any_of` |  | ✓ | ✓ | IMPLEMENTED |
| `appearance` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `baton_touch_source` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `baton_touch_trigger` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `card_property` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `card_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `cause` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `characters` |  | ✓ | ✓ | IMPLEMENTED |
| `comparison_target` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `comparison_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `condition` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `condition_type` |  | ✓ | ✓ | IMPLEMENTED |
| `conditions` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `cost_limit` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `destination` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `distinct` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `effect` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `energy_state` |  | ✓ | ✓ | IMPLEMENTED |
| `exclude_self` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `from_state` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `group` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `group_names` |  | ✓ | ✓ | IMPLEMENTED |
| `heart_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `location` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `movement` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `movement_condition` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `movement_state` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `negation` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `no_excess_heart` |  | ✓ | ✓ | IMPLEMENTED |
| `operator` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `optional` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `options` |  | ✓ | ✓ | IMPLEMENTED |
| `phase` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `position` |  | ✓ | ✓ | IMPLEMENTED |
| `resource_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `source` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `state` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `target` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `temporal` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `temporal_scope` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `text` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `to_state` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `unit` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `values` | ✓ | ✓ | ✓ | IMPLEMENTED |

## AbilityCost — Implementation Status
| Field | In abilities.json | In Rust struct | Engine reads it | Status |
|-------|:-:|:-:|:-:|--------|
| `action` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `card_type` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `characters` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `cost_limit` |  | ✓ | ✓ | IMPLEMENTED |
| `cost_type` |  | ✓ | ✓ | IMPLEMENTED |
| `costs` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `count` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `destination` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `energy` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `exclude_self` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `optional` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `options` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `position` |  | ✓ | ✓ | IMPLEMENTED |
| `self_cost` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `source` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `state_change` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `target` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `text` | ✓ | ✓ | ✓ | IMPLEMENTED |
| `type` | ✓ | ✓ | ✓ | IMPLEMENTED |

## Summary
- **153/153** fields are properly implemented (engine reads them)
- **0/153** fields are **dead code** — in struct, never read by engine
- **0/153** fields are **parser-only** — not in any Rust struct

### Dead field cleanup candidates
These struct fields are never read by engine code and should be removed:

### Gap: Parser emits but engine has no field
These need Rust struct fields adding:

### Value-level gaps: Engine doesn't handle this value
These values appear in abilities.json but the engine has no specific handler:
- `effect.card_type` = `card` — no engine handler found
- `effect.destination` = `under_member` — no engine handler found
- `effect.source` = `revealed_cards` — no engine handler found
- `effect.target` = `これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力` — no engine handler found
- `effect.target` = `相手のライブカード置き場にあるすべてのライブカード` — no engine handler found
- `effect.target` = `相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚` — no engine handler found
- `cost.destination` = `under_member` — no engine handler found
- `condition.operator` = `and` — no engine handler found

---
# Frequency Distribution & Field Necessity

How often each field+value appears across the 602 unique abilities. Low-frequency items are candidates for removal or simplification.

## Trigger Frequency
| Trigger | Count | % of abilities |
|---------|-------|-------|
| `登場` | 175 | 29.1% |
| `ライブ開始時` | 174 | 28.9% |
| `起動` | 67 | 11.1% |
| `ライブ成功時` | 65 | 10.8% |
| `常時` | 65 | 10.8% |
| `自動` | 40 | 6.6% |
| `ライブ開始時, 登場` | 6 | 1.0% |
| `登場, 左サイド` | 1 | 0.2% |
| `登場, 右サイド` | 1 | 0.2% |
| `起動, 左サイド` | 1 | 0.2% |
| `常時, 左サイド` | 1 | 0.2% |
| `常時, 右サイド` | 1 | 0.2% |
| `登場, 左サイド, 右サイド` | 1 | 0.2% |
| `ライブ開始時, 左サイド` | 1 | 0.2% |
| `ライブ開始時, 右サイド` | 1 | 0.2% |

## Action Frequency
| Action | Count | % of abilities | Engine handler |
|--------|-------|-------|----------------|
| `gain_resource` | 119 | 19.8% | `ability/effects.rs:75, 419-535` |
| `sequential` | 104 | 17.3% | `ability/effects.rs:69` |
| `move_cards` | 77 | 12.8% | `ability/effects.rs:74` |
| `modify_score` | 67 | 11.1% | `ability/effects.rs:77, 698-779` |
| `change_state` | 58 | 9.6% | `ability/effects.rs:76, 537-695` |
| `look_and_select` | 57 | 9.5% | `ability/effects.rs:71` |
| `draw_card` | 38 | 6.3% | `ability/effects.rs:72` |
| `restriction` | 21 | 3.5% | `ability/effects.rs:100, 1240-1244` |
| `position_change` | 13 | 2.2% | `ability/effects.rs:93, 1026-1071` |
| `choice` | 8 | 1.3% | `ability/effects.rs:95, 1160-1192` |
| `modify_cost` | 8 | 1.3% | `ability/effects.rs:102, 1479-1495` |
| `conditional_alternative` | 7 | 1.2% | `ability/effects.rs:70` |
| `appear` | 6 | 1.0% | `ability/effects.rs:94, 1107-1158` |
| `gain_ability` | 5 | 0.8% | `ability/effects.rs:84, 881-898` |
| `modify_yell_count` | 3 | 0.5% | `ability/effects.rs:90, 1014-1024` |
| `activate_ability` | 2 | 0.3% | `ability/effects.rs:82, 866-872` |
| `draw_until_count` | 1 | 0.2% | `ability/effects.rs:73, 343-356` |
| `play_baton_touch` | 1 | 0.2% | `ability/effects.rs:85, 900-906` |
| `set_card_identity` | 1 | 0.2% | `ability/effects.rs:97, 1202-1209` |
| `place_energy_under_member` | 1 | 0.2% | `ability/effects.rs:91, 358-387` |
| `set_score` | 1 | 0.2% | `ability/effects.rs:109, 1328-1334` |
| `set_blade_count` | 1 | 0.2% | `ability/effects.rs:106, 1297-1311` |

## Cost Type Frequency
| Cost Type | Count | % of abilities |
|-----------|-------|-------|
| `move_cards` | 95 | 15.8% |
| `pay_energy` | 53 | 8.8% |
| `change_state` | 22 | 3.7% |
| `sequential_cost` | 22 | 3.7% |
| `reveal` | 5 | 0.8% |
| `place_energy_under_member` | 3 | 0.5% |
| `choice_condition` | 1 | 0.2% |
| `energy_condition` | 1 | 0.2% |

## Condition Type Frequency
| Condition Type | Count | % of abilities |
|----------------|-------|-------|
| `location_condition` | 82 | 13.6% |
| `card_count_condition` | 52 | 8.6% |
| `comparison_condition` | 25 | 4.2% |
| `group_condition` | 25 | 4.2% |
| `compound` | 18 | 3.0% |
| `temporal_condition` | 13 | 2.2% |
| `complex_condition` | 8 | 1.3% |
| `appearance_condition` | 7 | 1.2% |
| `position_condition` | 7 | 1.2% |
| `state_condition` | 6 | 1.0% |
| `movement_condition` | 5 | 0.8% |
| `ability_negation_condition` | 2 | 0.3% |
| `opponent_choice_condition` | 2 | 0.3% |
| `score_threshold_condition` | 1 | 0.2% |
| `or_condition` | 1 | 0.2% |
| `position_change_condition` | 1 | 0.2% |
| `state_change_condition` | 1 | 0.2% |
| `energy_state_condition` | 1 | 0.2% |

## Effect Field Value Frequencies
| Field | Value | Count | % of abilities | Engine handler |
|-------|-------|-------|-------|----------------|
| `action` | `gain_resource` | 119 | 19.8% | `ability/effects.rs:75, 419-535` |
| `action` | `sequential` | 104 | 17.3% | `ability/effects.rs:69` |
| `action` | `move_cards` | 77 | 12.8% | `ability/effects.rs:74` |
| `action` | `modify_score` | 67 | 11.1% | `ability/effects.rs:77, 698-779` |
| `action` | `change_state` | 58 | 9.6% | `ability/effects.rs:76, 537-695` |
| `action` | `look_and_select` | 57 | 9.5% | `ability/effects.rs:71` |
| `action` | `draw_card` | 38 | 6.3% | `ability/effects.rs:72` |
| `action` | `restriction` | 21 | 3.5% | `ability/effects.rs:100, 1240-1244` |
| `action` | `position_change` | 13 | 2.2% | `ability/effects.rs:93, 1026-1071` |
| `action` | `choice` | 8 | 1.3% | `ability/effects.rs:95, 1160-1192` |
| `action` | `modify_cost` | 8 | 1.3% | `ability/effects.rs:102, 1479-1495` |
| `action` | `conditional_alternative` | 7 | 1.2% | `ability/effects.rs:70` |
| `action` | `appear` | 6 | 1.0% | `ability/effects.rs:94, 1107-1158` |
| `action` | `gain_ability` | 5 | 0.8% | `ability/effects.rs:84, 881-898` |
| `action` | `modify_yell_count` | 3 | 0.5% | `ability/effects.rs:90, 1014-1024` |
| `action` | `activate_ability` | 2 | 0.3% | `ability/effects.rs:82, 866-872` |
| `action` | `draw_until_count` | 1 | 0.2% | `ability/effects.rs:73, 343-356` |
| `action` | `play_baton_touch` | 1 | 0.2% | `ability/effects.rs:85, 900-906` |
| `action` | `set_card_identity` | 1 | 0.2% | `ability/effects.rs:97, 1202-1209` |
| `action` | `place_energy_under_member` | 1 | 0.2% | `ability/effects.rs:91, 358-387` |
| `action` | `set_score` | 1 | 0.2% | `ability/effects.rs:109, 1328-1334` |
| `action` | `set_blade_count` | 1 | 0.2% | `ability/effects.rs:106, 1297-1311` |
| `card_type` | `member_card` | 95 | 15.8% | `ability/util.rs:7` |
| `card_type` | `live_card` | 40 | 6.6% | `ability/util.rs:6` |
| `card_type` | `energy_card` | 15 | 2.5% | `ability/util.rs:8` |
| `card_type` | `card` | 15 | 2.5% | ⚠ no handler |
| `destination` | `hand` | 93 | 15.4% | `ability/move_cards.rs:72, 103, 113, 137, 155, 161, 187, 233, 300, 311, 320` |
| `destination` | `energy_zone` | 20 | 3.3% | `ability/move_cards.rs:80` |
| `destination` | `stage` | 6 | 1.0% | `ability/move_cards.rs:74-77, 112, 138, 163-175, 240-247, 250-291` |
| `destination` | `discard` | 6 | 1.0% | `ability/move_cards.rs:73, 102, 137, 155, 295, 300, 311` |
| `destination` | `deck_top` | 4 | 0.7% | `ability/move_cards.rs:82, 105, 162, 320` |
| `destination` | `empty_area` | 3 | 0.5% | `ability/move_cards.rs:250-291` |
| `destination` | `deck_bottom` | 1 | 0.2% | `ability/move_cards.rs:83, 104, 161, 193, 320` |
| `destination` | `same_area` | 1 | 0.2% | `ability/move_cards.rs:106-109, 231-249` |
| `destination` | `under_member` | 1 | 0.2% | ⚠ no handler |
| `destination` | `success_live_zone` | 1 | 0.2% | `ability/move_cards.rs:79, 111, 142, 311, 314` |
| `destination` | `live_card_zone` | 1 | 0.2% | `ability/move_cards.rs:78, 110, 141, 179, 216-230, 305` |
| `operation` | `increase` | 3 | 0.5% | `ability/effects.rs:1008-1009 (modify_required_hearts_global), 1367 (modify_required_hearts_success)` |
| `operation` | `decrease` | 3 | 0.5% | `ability/effects.rs:792-793 (modify_required_hearts), 1367` |
| `operation` | `subtract` | 2 | 0.3% | `ability/effects.rs:1019 (modify_yell_count), 1489 (modify_cost)` |
| `position` | `center` | 13 | 2.2% | `ability/effects.rs:1048; condition.rs:214` |
| `resource` | `blade` | 69 | 11.5% | `ability/effects.rs:496-513` |
| `resource` | `heart` | 50 | 8.3% | `ability/effects.rs:515-520` |
| `source` | `discard` | 68 | 11.3% | `ability/move_cards.rs:183-291` |
| `source` | `deck` | 59 | 9.8% | `ability/move_cards.rs:58-89` |
| `source` | `hand` | 9 | 1.5% | `ability/move_cards.rs:151-180` |
| `source` | `deck_top` | 4 | 0.7% | `ability/move_cards.rs:58-89` |
| `source` | `success_live_zone` | 1 | 0.2% | `ability/move_cards.rs:314-323` |
| `source` | `revealed_cards` | 1 | 0.2% | ⚠ no handler |
| `state_change` | `wait` | 37 | 6.1% | `ability/effects.rs:566-568, 677-684` |
| `state_change` | `active` | 23 | 3.8% | `ability/effects.rs:567-568, 686-693` |
| `target` | `self` | 125 | 20.8% | `ability/effects.rs:target='self' in draw (274), modify_score (701), etc.` |
| `target` | `opponent` | 12 | 2.0% | `ability/effects.rs:target='opponent' in change_state, modify_score, etc.` |
| `target` | `both` | 5 | 0.8% | `ability/effects.rs:283-288 (draw_card target both)` |
| `target` | `相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚` | 2 | 0.3% | ⚠ no handler |
| `target` | `これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力` | 1 | 0.2% | ⚠ no handler |
| `target` | `相手のライブカード置き場にあるすべてのライブカード` | 1 | 0.2% | ⚠ no handler |

## Cost Field Value Frequencies
| Field | Value | Count | % of abilities | Engine handler |
|-------|-------|-------|-------|----------------|
| `card_type` | `member_card` | 37 | 6.1% | `ability/util.rs:7` |
| `card_type` | `live_card` | 4 | 0.7% | `ability/util.rs:6` |
| `destination` | `discard` | 93 | 15.4% | `ability/move_cards.rs:73, 102, 137, 155, 295, 300, 311` |
| `destination` | `under_member` | 3 | 0.5% | ⚠ no handler |
| `destination` | `deck_bottom` | 2 | 0.3% | `ability/move_cards.rs:83, 104, 161, 193, 320` |
| `destination` | `energy_deck` | 1 | 0.2% | `ability/move_cards.rs:81` |
| `source` | `hand` | 89 | 14.8% | `ability/move_cards.rs:151-180` |
| `source` | `stage` | 7 | 1.2% | `ability/move_cards.rs:92-148` |
| `source` | `discard` | 2 | 0.3% | `ability/move_cards.rs:183-291` |
| `source` | `deck_top` | 2 | 0.3% | `ability/move_cards.rs:58-89` |
| `state_change` | `wait` | 22 | 3.7% | `ability/effects.rs:566-568, 677-684` |
| `target` | `self` | 2 | 0.3% | `ability/effects.rs:target='self' in draw (274), modify_score (701), etc.` |
| `type` | `move_cards` | 95 | 15.8% | `ability/cost.rs:29-43, 88-147` |
| `type` | `pay_energy` | 53 | 8.8% | `ability/cost.rs:175-208` |
| `type` | `change_state` | 22 | 3.7% | `ability/cost.rs:148-174` |
| `type` | `sequential_cost` | 22 | 3.7% | `ability/cost.rs:10-17` |
| `type` | `reveal` | 5 | 0.8% | `ability/cost.rs:224-234` |
| `type` | `place_energy_under_member` | 3 | 0.5% | `ability/cost.rs:235-244` |
| `type` | `choice_condition` | 1 | 0.2% | `ability/cost.rs:19-27` |
| `type` | `energy_condition` | 1 | 0.2% | `ability/cost.rs:45-52, 209-223` |

## Condition Field Value Frequencies
| Field | Value | Count | % of abilities | Engine handler |
|-------|-------|-------|-------|----------------|
| `card_type` | `member_card` | 82 | 13.6% | `ability/util.rs:7` |
| `card_type` | `live_card` | 28 | 4.7% | `ability/util.rs:6` |
| `operator` | `>=` | 84 | 14.0% | `ability/util.rs:128` |
| `operator` | `and` | 18 | 3.0% | ⚠ no handler |
| `operator` | `>` | 14 | 2.3% | `ability/util.rs:129` |
| `operator` | `<` | 11 | 1.8% | `ability/util.rs:131` |
| `operator` | `=` | 5 | 0.8% | `ability/util.rs:132` |
| `target` | `self` | 115 | 19.1% | `ability/effects.rs:target='self' in draw (274), modify_score (701), etc.` |
| `target` | `both` | 8 | 1.3% | `ability/effects.rs:283-288 (draw_card target both)` |
| `target` | `opponent` | 6 | 1.0% | `ability/effects.rs:target='opponent' in change_state, modify_score, etc.` |
| `target` | `either` | 1 | 0.2% | `ability/condition.rs:121-151 (location condition either)` |
| `type` | `location_condition` | 82 | 13.6% | `ability/condition.rs:12, 77-207` |
| `type` | `card_count_condition` | 52 | 8.6% | `ability/condition.rs:15, 229-242` |
| `type` | `comparison_condition` | 25 | 4.2% | `ability/condition.rs:11, 56-75` |
| `type` | `group_condition` | 25 | 4.2% | `ability/condition.rs:14, 222-227` |
| `type` | `compound` | 18 | 3.0% | `ability/condition.rs:10, 44-54` |
| `type` | `temporal_condition` | 13 | 2.2% | `ability/condition.rs:17, 275-330` |
| `type` | `complex_condition` | 8 | 1.3% | `ability/condition.rs:30, 464-470` |
| `type` | `appearance_condition` | 7 | 1.2% | `ability/condition.rs:16, 244-273` |
| `type` | `position_condition` | 7 | 1.2% | `ability/condition.rs:13, 209-220` |
| `type` | `state_condition` | 6 | 1.0% | `ability/condition.rs:18, 332-340` |
| `type` | `movement_condition` | 5 | 0.8% | `ability/condition.rs:20, 352-381` |
| `type` | `ability_negation_condition` | 2 | 0.3% | `ability/condition.rs:21, 383-388` |
| `type` | `opponent_choice_condition` | 2 | 0.3% | `ability/condition.rs:28, 453-458` |
| `type` | `score_threshold_condition` | 1 | 0.2% | `ability/condition.rs:24, 416-423` |
| `type` | `or_condition` | 1 | 0.2% | `ability/condition.rs:22, 390-394` |
| `type` | `position_change_condition` | 1 | 0.2% | `ability/condition.rs:26, 433-442` |
| `type` | `state_change_condition` | 1 | 0.2% | `ability/condition.rs:27, 444-451` |
| `type` | `energy_state_condition` | 1 | 0.2% | `ability/condition.rs:19, 342-350` |

---
# Field Necessity Analysis

Based on frequency distributions and implementation status. Fields used in <5% of abilities or that are dead code are candidates for removal/consolidation.

## Rarely-used effect fields (<5% of abilities)
- `effect.action_by` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.activation_condition` — appears 13/602 times (2.2%), status: IMPLEMENTED
- `effect.activation_position` — appears 6/602 times (1.0%), status: IMPLEMENTED
- `effect.all_regions` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.choice_type` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `effect.conditional` — appears 16/602 times (2.7%), status: IMPLEMENTED
- `effect.cost_limit` — appears 24/602 times (4.0%), status: IMPLEMENTED
- `effect.distinct` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.effect_constraint` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.energy_count` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.exclude_self` — appears 4/602 times (0.7%), status: IMPLEMENTED
- `effect.heart_selection` — appears 4/602 times (0.7%), status: IMPLEMENTED
- `effect.location` — appears 20/602 times (3.3%), status: IMPLEMENTED
- `effect.max` — appears 20/602 times (3.3%), status: IMPLEMENTED
- `effect.multiple_targets` — appears 5/602 times (0.8%), status: IMPLEMENTED
- `effect.name_constraint` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.name_constraint_source` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.operation` — appears 8/602 times (1.3%), status: IMPLEMENTED
- `effect.placement_order` — appears 3/602 times (0.5%), status: IMPLEMENTED
- `effect.position` — appears 13/602 times (2.2%), status: IMPLEMENTED
- `effect.question` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `effect.restriction_type` — appears 21/602 times (3.5%), status: IMPLEMENTED
- `effect.state` — appears 4/602 times (0.7%), status: IMPLEMENTED
- `effect.target_count` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.target_trigger` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `effect.timing_condition` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `effect.trigger_type` — appears 6/602 times (1.0%), status: IMPLEMENTED
- `effect.value` — appears 4/602 times (0.7%), status: IMPLEMENTED

## Rarely-used cost fields (<5% of abilities)
- `cost.action` — appears 7/602 times (1.2%), status: IMPLEMENTED
- `cost.exclude_self` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `cost.self_cost` — appears 22/602 times (3.7%), status: IMPLEMENTED
- `cost.shuffle` — appears 1/602 times (0.2%), status: ?
- `cost.state_change` — appears 22/602 times (3.7%), status: IMPLEMENTED
- `cost.target` — appears 2/602 times (0.3%), status: IMPLEMENTED

## Rarely-used condition fields (<5% of abilities)
- `condition.action` — appears 3/602 times (0.5%), status: IMPLEMENTED
- `condition.aggregate` — appears 28/602 times (4.7%), status: ?
- `condition.all_areas` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `condition.appearance` — appears 7/602 times (1.2%), status: IMPLEMENTED
- `condition.baton_touch_source` — appears 4/602 times (0.7%), status: IMPLEMENTED
- `condition.baton_touch_trigger` — appears 13/602 times (2.2%), status: IMPLEMENTED
- `condition.card_property` — appears 3/602 times (0.5%), status: IMPLEMENTED
- `condition.comparison_target` — appears 16/602 times (2.7%), status: IMPLEMENTED
- `condition.cost_limit` — appears 4/602 times (0.7%), status: IMPLEMENTED
- `condition.destination` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `condition.distinct` — appears 3/602 times (0.5%), status: IMPLEMENTED
- `condition.exclude_self` — appears 7/602 times (1.2%), status: IMPLEMENTED
- `condition.from_state` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `condition.heart_type` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `condition.movement` — appears 7/602 times (1.2%), status: IMPLEMENTED
- `condition.movement_condition` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `condition.movement_state` — appears 5/602 times (0.8%), status: IMPLEMENTED
- `condition.negation` — appears 11/602 times (1.8%), status: IMPLEMENTED
- `condition.optional` — appears 3/602 times (0.5%), status: IMPLEMENTED
- `condition.phase` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `condition.resource_type` — appears 10/602 times (1.7%), status: IMPLEMENTED
- `condition.source` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `condition.state` — appears 7/602 times (1.2%), status: IMPLEMENTED
- `condition.temporal` — appears 14/602 times (2.3%), status: IMPLEMENTED
- `condition.temporal_scope` — appears 2/602 times (0.3%), status: IMPLEMENTED
- `condition.to_state` — appears 1/602 times (0.2%), status: IMPLEMENTED
- `condition.unit` — appears 16/602 times (2.7%), status: IMPLEMENTED

## Verdict
- **0 dead fields** — safe to remove, no engine code reads them
- **61 rarely-used fields** (<5% of abilities) — consider consolidating

Removing all 49 dead fields would reduce struct size by ~40% without affecting functionality.
