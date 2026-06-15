# Card Game Ability Documentation

This document describes the schema and semantics of the abilities defined in `abilities.json`.

## Core Structure
Each ability consists of a `cost` and an `effect`. Both use a `type` (cost) or `action` (effect) key to define their primary behavior, supplemented by various parameters that refine the logic.

---

## Common Fields
These fields appear across multiple cost types and effect actions.

| Field | Description |
| :--- | :--- |
| `count` | The number of cards or resources involved. |
| `card_type` | Filters the action to specific card types (e.g., `member_card`, `live_card`, `energy_card`, `card`). |
| `source` | The zone or location where the action originates. |
| `destination` | The zone or location where cards are moved to. |
| `target` | The intended target of the effect (`self`, `opponent`, `both`, or `either`). |
| `condition` | A requirement that must be met for the action to execute. See Condition Types below. |
| `duration` | How long the effect lasts (`live_end`, `this_turn`, `this_live`, `turn_end`). |
| `group_names` | Filters for specific groups of cards/members (e.g., `虹ヶ咲`, `蓮ノ空`). |
| `optional` | Whether the action is mandatory or can be chosen by the player. |
| `parenthetical` | Additional context or clarification provided in the original text (array of strings). |
| `all` | Boolean indicating if the action applies to all eligible targets. |
| `exclude_self` | Boolean indicating if the card itself is excluded from the action. |
| `placement_order` | How cards are placed in destination (`any_order`, `deck_top_or_bottom`). |
| `shuffle` | Boolean indicating whether the source zone is shuffled after movement. |
| `self_cost` | Boolean indicating the cost uses the activating card itself. |
| `any_number` | Boolean indicating player can choose any number of cards. |
| `max` | Boolean indicating "up to N" limit on selection count. |
| `distinct` | Boolean indicating items must have different names/costs/types. |
| `same_unit_name` | Boolean indicating cards must share the same unit name. |
| `zone` | The zone where energy or cards are taken from. |
| `position` | Position requirement (`center`, `left_side`, `right_side`). |
| `source_position` | Source position for movement actions. |
| `exclude_position` | Position to exclude from selection. |
| `per_unit` | Boolean indicating effect scales with count of something. |
| `per_unit_count` | The count per unit for per_unit calculations. |
| `per_unit_type` | The type of unit being counted (e.g., `枚`, `人`). |
| `per_unit_location` | Zone where per_unit count is calculated. |

---

## Cost Types
The `cost.type` defines what must be paid to activate the ability.

### `move_cards`
Moves cards to pay for the ability.
| Field | Description |
| :--- | :--- |
| `count` | Number of cards to move. |
| `source` | Source zone. |
| `destination` | Destination zone. |
| `card_type` | Filter by card type. |
| `optional` | Whether moving is optional. |
| `placement_order` | How cards are placed (for complex placements). |
| `shuffle` | Whether to shuffle after movement. |
| `cost_limit` / `cost_limit_operator` | Maximum total cost constraint. |
| `same_unit_name` | Cards must share the same unit name. |

### `pay_energy`
Requires energy expenditure.
| Field | Description |
| :--- | :--- |
| `energy` / `energy_count` | Amount of energy required. |
| `zone` | Where energy is taken from (`energy_zone`). |
| `optional` | Whether payment is optional. |

### `reveal`
Requires showing a card.
| Field | Description |
| :--- | :--- |
| `characters` | Specific characters to reveal (array). |
| `exclude_characters` | Characters to exclude from reveal (array). |
| `source` | Source zone. |
| `group_names` | Filter by card group. |

### `change_state`
Changes a card's state.
| Field | Description |
| :--- | :--- |
| `state_change` | Target state (`wait`, `active`). |
| `card_type` | Filter by card type. |
| `optional` | Whether state change is optional. |
| `self_cost` | Whether the acting card changes state. |
| `exclude_self` | Exclude the acting card. |

### `sequential_cost`
Combines multiple costs that must all be paid.
| Field | Description |
| :--- | :--- |
| `costs` | Array of cost objects. |
| `optional` | Whether entire sequential cost is optional. |

### `choice_condition`
Cost that requires choosing an option.
| Field | Description |
| :--- | :--- |
| `options` | Array of available choices. |

### `custom`
Custom/unstructured cost (parsed from complex patterns).
| Field | Description |
| :--- | :--- |
| `target` | Target player. |
| `destination` | Where to send cards. |

---

## Effect Actions
The `effect.action` defines what the ability actually does.

### `move_cards`
Moves cards between zones.
| Field | Description |
| :--- | :--- |
| `source` | Source zone. |
| `destination` | Destination zone. |
| `count` | Number of cards to move. |
| `card_type` | Filter by card type. |
| `name_constraint` | Required card name(s). |
| `name_constraint_source` | Source for name constraint. |
| `state_change` | Apply state change during movement (`wait`, `active`). |
| `placement_order` | How cards are placed (`any_order`). |
| `cost_limit` / `cost_limit_operator` | Filter by card cost. |
| `cost_total` / `cost_total_operator` | Filter by total cost. |
| `need_heart_total` / `need_heart_operator` / `need_heart_color` | Based on heart counts. |
| `or_card_types` | Multiple allowed card types. |
| `max` | Boolean for "up to N" selection limit. |
| `distinct` | Items must have different names. |
| `self_target` | Boolean indicating self is the target. |
| `multiple_targets` | Boolean for selecting multiple targets. |
| `quoted_text` | Text in 「」 quotes. |
| `value` | Numeric value (for scored cards). |

### `gain_resource`
Increases score, blades, or hearts.
| Field | Description |
| :--- | :--- |
| `resource` | Type of resource (`heart`, `blade`, `score`). |
| `count` | Amount to gain. |
| `heart_selection` | Boolean for player-selectable heart color. |
| `filter_targets_by_heart_colors` | Filter by heart color. |
| `per_unit` | Scale with count of something. |
| `per_unit_count` | Count per unit. |
| `per_unit_type` | Unit type (e.g., `live_card_zone`). |
| `location` | Zone to count for per_unit. |
| `dynamic_count` | Count calculated at runtime. |
| `sign` | Add or subtract resource. |
| `target_count` | Number of targets to affect. |
| `heart_colors` | Array of heart color identifiers. |
| `group_reference` | Reference group for calculation. |
| `multiple_targets` | Boolean for multiple target selection. |

### `change_state`
Modifies a card's status.
| Field | Description |
| :--- | :--- |
| `state_change` | Target state (`wait`, `active`). |
| `count` | Number of cards to affect. |
| `card_type` | Filter by card type. |
| `location` | Zone where cards are located. |
| `per_unit` | Scale with count. |
| `blade_limit` / `blade_limit_operator` | Blade count constraint. |
| `cost_limit` / `cost_limit_operator` | Cost constraint. |
| `distinct` | Items must have different names. |
| `max` | Boolean for "up to N" limit. |
| `optional` | Whether state change is optional. |
| `target` | Target player. |
| `position` | Position requirement. |
| `original_value` | Compare to original/natural value. |
| `values` | Array of possible values for condition. |

### `draw_card`
Draws cards from the deck.
| Field | Description |
| :--- | :--- |
| `count` | Number of cards to draw. |
| `source` | Source zone (`deck`). |
| `destination` | Destination zone (`hand`). |
| `card_type` | Filter by card type. |
| `condition` | When to draw (e.g., if in specific position). |
| `heart_colors` | Heart color requirements. |
| `trigger_condition` / `trigger_type` | Special triggers. |
| `position` | Position-based trigger. |
| `original_value` | Compare to original value. |

### `sequential`
Executes multiple actions in order.
| Field | Description |
| :--- | :--- |
| `actions` | Array of effect objects to execute. |
| `character_effects` | Effects tied to specific characters. |
| `condition` | Condition for executing sequence. |

### `look_and_select`
Look at cards then choose from them.
| Field | Description |
| :--- | :--- |
| `look_action` | Action to look at cards (has `action: look_at`). |
| `select_action` | Action to select from looked cards. |
| `condition` | Conditional execution. |
| `heart_colors` | Filter by heart colors. |

### `look_at`
Look at cards without taking them.
| Field | Description |
| :--- | :--- |
| `source` | Source zone (`deck_top`, `revealed_cards`). |
| `count` | Number of cards to look at. |
| `target` | Target player. |

### `select_cards`
Select cards after looking.
| Field | Description |
| :--- | :--- |
| `count` | Number to select. |
| `destination` | Where selected cards go. |
| `placement_order` | How to place (e.g., `any_order`). |
| `any_number` | Boolean for any number selection. |
| `discard_remaining` | Boolean to discard unselected cards. |
| `reveal` | Boolean for reveal-on-selection. |

### `gain_ability`
Grants a new ability.
| Field | Description |
| :--- | :--- |
| `ability_gain` | The gained ability text. |
| `ability_text` | Alternative text field. |
| `count` | Number of abilities. |
| `duration` | How long ability lasts. |
| `target` | Target player. |
| `group_names` | Filter by group. |
| `activation_condition_parsed` | Parse activation condition. |
| `activation_position` | Position where activation occurs. |
| `max` | Boolean for "up to N" abilities. |
| `parenthetical` | Clarification notes. |

### `activate_ability`
Trigger an ability on another card.
| Field | Description |
| :--- | :--- |
| `ability_text` | Text of ability to activate. |
| `target_trigger` | Which trigger to activate. |
| `source_card` | Source card reference (`cost_card`). |
| `count` | Number of activations. |
| `parenthetical` | Clarification notes. |
| `target` | Target player. |

### `modify_score`
Adjusts score.
| Field | Description |
| :--- | :--- |
| `operation` | `add`, `subtract`, `set`. |
| `value` | Amount of change. |
| `target` | Target player. |
| `card_type` | Filter by card type. |
| `per_unit` | Scale with count. |
| `per_unit_count` / `per_unit_type` | Per-unit calculation. |
| `heart_colors` | Heart color filter. |
| `location` | Zone for per_unit count. |
| `position` | Position requirement. |
| `duration` | Effect duration. |
| `self_target` | Boolean for self-targeting. |

### `modify_cost`
Adjusts card costs.
| Field | Description |
| :--- | :--- |
| `operation` | Type of modification. |
| `value` | Amount of change. |
| `non_stackable` | Boolean to prevent stacking. |
| `card_type` | Filter by card type. |
| `target` | Target player. |
| `condition` | When to apply. |
| `per_unit` | Scale with count. |
| `location` | Zone for per_unit count. |
| `duration` | Effect duration. |
| `original_count` / `original_operator` / `original_value` | Reference to original values. |
| `cost_limit` / `cost_limit_operator` | Cost constraint. |

### `modify_required_hearts`
Adjusts heart requirements.
| Field | Description |
| :--- | :--- |
| `operation` | Type of modification. |
| `value` | Amount of change. |
| `card_type` | Filter by card type. |
| `target` | Target player. |
| `heart_colors` | Which hearts to modify. |
| `location` | Zone filter. |
| `duration` | Effect duration. |
| `per_unit` | Scale with count. |
| `position` | Position requirement. |
| `self_target` | Boolean for self-targeting. |
| `timing_condition` | When condition is met. |

### `modify_yell_count`
Adjusts yell activation count.
| Field | Description |
| :--- | :--- |
| `operation` | Type of modification. |
| `count` | New yell count. |
| `condition` | When to apply. |
| `duration` | Effect duration. |
| `exclude_self` | Exclude the acting card. |

### `choice`
Forces a player to choose between options.
| Field | Description |
| :--- | :--- |
| `options` | Array of available choices. |
| `choice_type` | How choice is made. |
| `question` | Prompt for the choice. |
| `target` | Target player. |
| `count` | Number of choices. |
| `exclude_self` | Exclude self from choices. |
| `group_reference` | Reference group. |

### `select`
Select a heart color or option.
| Field | Description |
| :--- | :--- |
| `count` | Number to select. |
| `heart_colors` | Available heart color choices. |
| `original_value` | Compare to original value. |

### `set_blade_count`
Sets blade count to a specific value.
| Field | Description |
| :--- | :--- |
| `count` | Target blade count. |
| `blade_limit` / `blade_limit_operator` | Constraint on setting. |
| `card_type` | Filter by card type. |
| `target` | Target player. |
| `duration` | Effect duration. |
| `original_value` | Compare to original value. |
| `position` | Position requirement. |

### `set_blade_type`
Sets blade type/color.
| Field | Description |
| :--- | :--- |
| `blade_type` | The blade type to set. |
| `duration` | Effect duration. |

### `set_card_identity`
Sets card identity flags.
| Field | Description |
| :--- | :--- |
| `identities` | Array of identity strings. |
| `all` / `all_regions` | Apply to all regions. |
| `self_target` | Boolean for self-targeting. |
| `group_names` | Filter by group. |

### `restriction`
Applies a restriction effect.
| Field | Description |
| :--- | :--- |
| `restriction_type` | Type of restriction. |
| `operation` | Modify operation. |
| `value` | Restriction value. |
| `target` | Target player. |
| `self_target` | Boolean for self-targeting. |
| `phase` | Game phase restriction. |
| `card_type` | Filter by card type. |
| `count` | Number of restrictions. |

### `position_change`
Changes card position on stage.
| Field | Description |
| :--- | :--- |
| `card_type` | Filter by card type. |
| `count` | Number of cards. |
| `destination` | Destination zone. |
| `multiple_targets` | Boolean for multiple targets. |
| `optional` | Whether change is optional. |
| `parenthetical` | Clarification notes. |
| `position` | Target position. |
| `source_position` | Source position. |
| `exclude_position` | Excluded position. |
| `target` | Target player. |
| `target_member` | Target member selection. |

### `opponent_action`
Requires opponent to take an action.
| Field | Description |
| :--- | :--- |
| `action_by` | Who performs the action. |
| `opponent_action` | The action object. |

### `conditional_alternative`
Alternative effect under condition.
| Field | Description |
| :--- | :--- |
| `condition` | When to use alternative. |
| `primary_effect` | Main effect. |
| `alternative_effect` | Alternative effect. |
| `text` | Full text. |

### `conditional_on_optional`
Conditional on optional cost being paid/skipped.
| Field | Description |
| :--- | :--- |
| `optional_action` | Action if optional cost skipped. |
| `conditional_action` | Action based on condition. |
| `conditional_negation` | Negate condition. |

### `conditional_on_result`
Conditional on action result.
| Field | Description |
| :--- | :--- |
| `result_condition` | When result meets condition. |
| `primary_effect` | Main effect. |
| `followup_action` | Action after result. |
| `heart_colors` | Heart color filter. |

### `draw_until_count`
Draw until a certain count is reached.
| Field | Description |
| :--- | :--- |
| `count` | Target count. |
| `target_count` | Additional count parameter. |
| `target` | Target player. |
| `source` | Source zone. |
| `condition` | Stopping condition. |

### `gain_ability_from_source`
Gain ability from cards under a member.
| Field | Description |
| :--- | :--- |
| `card_type` | Filter by card type. |
| `source_location` | Where to look for source (`under_member`). |
| `cost_limit` / `cost_limit_operator` | Cost constraint. |
| `group_names` | Filter by group. |
| `trigger_filter` | Filter triggers. |

### `place_energy_under_member`
Place energy under a member.
| Field | Description |
| :--- | :--- |
| `card_type` | Filter by card type. |
| `count` | Number of energy cards. |
| `destination` | Destination zone. |
| `energy_count` | Energy amount. |
| `group_names` | Filter by group. |
| `optional` | Whether placement is optional. |
| `source` | Source zone. |
| `state_change` | Apply state change. |
| `target` | Target player. |
| `target_member` | Which member to target. |

### `play_baton_touch`
Baton touch activation.
| Field | Description |
| :--- | :--- |
| `count` | Number of baton touches. |
| `text` | Full text. |

---

## Condition Types
The `condition` object appears in effects and nested contexts. Its `type` field determines structure.

### Common Condition Fields
| Field | Description |
| :--- | :--- |
| `text` | Original condition text. |
| `type` | Condition type identifier. |
| `target` | Target player (`self`, `opponent`, `both`). |
| `location` | Zone to check. |
| `locations` | Multiple zones (array). |
| `count` | Count value. |
| `operator` | Comparison operator (`>=`, `<=`, `>`, `<`, `==`). |
| `card_type` | Filter by card type. |
| `group_names` | Filter by card group. |
| `characters` | Specific characters to check. |
| `exclude_characters` | Characters to exclude. |
| `negation` | Boolean to negate condition. |
| `distinct` | Boolean for distinct names/costs. |
| `position` | Position requirement. |
| `position_compare` | Cross-position comparison. |
| `all_areas` | Boolean for all stage areas. |

### `card_count_condition`
Check count of cards in a zone.
| Field | Description |
| :--- | :--- |
| `location` | Zone to count (`hand`, `deck`, `discard`, `energy_zone`, `stage`, `live_card_zone`, `success_live_zone`). |
| `temporal` | Timing scope (`this_turn`, `during_live`). |
| `unit` | Unit for count (`人`, `枚`, `つ`, `types`). |
| `card_property` | Property filter (e.g., `has_all_blade`). |

### `comparison_condition`
Compare values.
| Field | Description |
| :--- | :--- |
| `comparison_type` | What to compare (`cost`, `score`). |
| `resource_type` | Resource type (`energy`, `hand_count`, `surplus_heart`). |
| `location` | Zone for comparison. |

### `compound`
Logical AND/OR of conditions.
| Field | Description |
| :--- | :--- |
| `operator` | `and` or `or`. |
| `conditions` | Array of sub-conditions. |
| `all_areas` | Boolean for all areas check. |
| `scope` | Scope for comparison (`both`). |

### `temporal_condition`
Time-based condition.
| Field | Description |
| :--- | :--- |
| `temporal` | Time scope (`this_turn`, `during_live`). |
| `phase` | Game phase (`main_phase`, `live_phase`). |
| `condition` | Nested condition. |
| `turn_number` | Specific turn number. |
| `no_excess_heart` | Boolean for no surplus heart requirement. |

### `location_condition`
Location-based condition.
| Field | Description |
| :--- | :--- |
| `location` | Zone to check. |
| `all_areas` | Boolean for all stage areas. |
| `distinct` | Boolean for distinct names. |
| `card_property` | Property check. |

### `appearance_condition`
Check if specific cards are on stage.
| Field | Description |
| :--- | :--- |
| `appearance` | Boolean (always true). |
| `location` | Zone (`stage`). |
| `baton_touch_trigger` | Boolean for baton touch. |
| `placement_order` | Placement requirement. |
| `activation_position` | Position requirement. |
| `all_areas` | Boolean for all areas. |
| `cost_reference_character` | Character for cost comparison. |
| `cost_reference_operator` | Comparison operator. |
| `cost_reference_type` | Reference type. |

### `state_condition`
Check card state.
| Field | Description |
| :--- | :--- |
| `state` | State to check (`wait`, `active`). |
| `negation` | Boolean to negate. |
| `all` | Boolean for all cards. |

### `movement_condition`
Check card movement status.
| Field | Description |
| :--- | :--- |
| `movement` | Movement type (`moved`, `moves`). |
| `movement_state` | State (`has_moved`). |
| `negation` | Boolean to negate. |
| `self_effect_only` | Boolean for own effects only. |
| `energy_placed` | Boolean for energy placement trigger. |

### `or_condition`
Logical OR of conditions.
| Field | Description |
| :--- | :--- |
| `conditions` | Array of sub-conditions. |

---

## Zones Reference
| Key | Description |
| :--- | :--- |
| `deck` | Main deck. |
| `deck_top` | Top of the deck. |
| `deck_bottom` | Bottom of the deck. |
| `deck_position_N` | Specific position N from top (e.g., `deck_position_4`). |
| `discard` | Discard pile (控え室). |
| `energy_zone` | Energy area. |
| `energy_deck` | Energy deck. |
| `hand` | Player's hand. |
| `stage` | Active stage. |
| `empty_area` | Unoccupied stage slot. |
| `front` | Front of the stage. |
| `same_area` | Current position. |
| `under_member` | Underneath a member card. |
| `success_live_zone` | Successful live results area. |
| `revealed_cards` | Recently revealed cards. |
| `revealed_remaining` | Remaining revealed cards after initial selection. |
| `those_cards` | Cards from trigger event. |

---

## Triggers Reference
| Trigger | Description |
| :--- | :--- |
| `起動` | Activated ability (manual activation). |
| `登場` | When card enters stage. |
| `ライブ開始時` | At start of live performance. |
| `ライブ成功時` | When live succeeds. |
| `ライブ終了時` | At end of live performance. |
| `常時` | Continuous/always (passive ability). |
| `ターン1回` | Once per turn limitation. |

---

## Card Types Reference
| Type | Description |
| :--- | :--- |
| `member_card` | Member card. |
| `live_card` | Live card. |
| `energy_card` | Energy card. |
| `card` | Any card type. |

---

## Resources Reference
| Resource | Description |
| :--- | :--- |
| `heart` | Heart symbol. |
| `blade` | Blade symbol. |
| `score` | Score value. |

---

## Position Reference
| Position | Description |
| :--- | :--- |
| `center` | Center stage area. |
| `left_side` | Left side stage area. |
| `right_side` | Right side stage area. |
| `front` | Front of stage (for live cards). |

---

## Operators Reference
| Operator | Description |
| :--- | :--- |
| `>=` | Greater than or equal. |
| `<=` | Less than or equal. |
| `>` | Greater than. |
| `<` | Less than. |
| `==` | Equal to. |

---

## Additional Fields from Parser Implementation

### Dynamic Count Fields
| Field | Description |
| :--- | :--- |
| `dynamic_count` | Object for runtime-calculated counts. |
| `reference` | Reference for dynamic calculation. |
| `mode` | Calculation mode (`relative_to`, `fixed`). |
| `base_reference` | Base reference point. |
| `calculation` | Calculation type (`add`, `subtract`). |
| `calculation_value` | Value for calculation. |

### Cost Modifiction Fields
| Field | Description |
| :--- | :--- |
| `modification_type` | Type of cost modification. |
| `cost_threshold` | Threshold for cost change. |
| `threshold_operator` | Operator for threshold. |

### Additional Effect Fields
| Field | Description |
| :--- | :--- |
| `secondary_effect` | Follow-up effect. |
| `cost_total` / `cost_total_operator` | Total cost constraint. |
| `choices_made` | Track player selections. |
| `trigger_filter` | Filter triggers by name. |
| `gained_effect` | Pre-parsed effect from gained ability. |
| `quoted_text` | Text in 「」 quotes. |
| `replacement` | Replacement effect data. |
| `replaces_event` | What event this replaces. |
| `restriction_type` | Type of restriction. |
| `restricted_destination` | Where restriction applies. |

---

## Fields from Ability Struct
| Field | Description |
| :--- | :--- |
| `full_text` | Complete ability text with icons. |
| `triggerless_text` | Text without trigger prefix. |
| `triggers` | Trigger type(s), comma-separated if multiple. |
| `use_limit` | Limit on uses (e.g., 1 for "once per turn"). |
| `is_null` | Boolean indicating null/collapsed condition-only ability. |
| `cost` | Cost object. |
| `effect` | Effect object. |