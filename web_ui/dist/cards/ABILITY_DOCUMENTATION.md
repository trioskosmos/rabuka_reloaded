# Card Game Ability Documentation

This document exhaustively describes the schema, semantics, and runtime execution of 
abilities defined in `abilities.json`.

---

## Table of Contents
1. [Execution Pipeline](#execution-pipeline)
2. [Ability Structure](#ability-structure)
3. [Cost Types](#cost-types)
4. [Effect Actions](#effect-actions)
5. [Condition Types](#condition-types)
6. [Compound / Wrapper Effects](#compound--wrapper-effects)
7. [Zones Reference](#zones-reference)
8. [Triggers Reference](#triggers-reference)
9. [Card Types Reference](#card-types-reference)
10. [Common Fields](#common-fields)
11. [Additional Parser Fields](#additional-parser-fields)
12. [Edge Cases & Special Behaviors](#edge-cases--special-behaviors)

---

## Execution Pipeline

Abilities are processed at runtime in a defined pipeline. Understanding this 
pipeline is essential for correctly implementing ability tests.

### 1. Trigger Detection

Triggers fire based on game events:
- `登場` (Debut): When a member card enters the stage
- `ライブ開始時` (Live Start): When a live performance begins
- `ライブ成功時` (Live Success): When a live succeeds
- `起動` (Activation): Player manually activates (once per turn unless specified)
- `常時` (Constant): Always active/passive, evaluated continuously
- `自動` (Auto): Automatic triggers from specific game events
- `メイン` (Main): Usable during main phase
- `baton touch`: Triggers on baton touch

Trigger processing order within a phase:
1. All `常時` (constant) abilities are active from the start
2. Phase-specific triggers fire in a defined order (debut → live_start → main → ...)
3. `起動` abilities require player action to activate

### 2. Ability Resolution Flow

For each ability activation:

```
┌─────────────────────────────────────────────────────┐
│  1. can_activate_effect() check                      │
│     - use_limit (once per turn)                      │
│     - cost validation                                │
│     - condition check                                │
├─────────────────────────────────────────────────────┤
│  2. Pay cost (validate_cost → pay_cost_inner)        │
│     - Sequential cost: pay each sub-cost in order    │
│     - Optional cost: player can skip                 │
│     - Choice condition: player selects option        │
│     - If cost creates a choice, flow pauses (queue)  │
├─────────────────────────────────────────────────────┤
│  3. Execute effect                                   │
│     - Sequential: execute sub-effects in order       │
│     - Conditional: check condition, pick branch      │
│     - If effect creates a choice, flow pauses        │
│     - non_stackable deduplication check              │
│     - target="both" handled generically              │
├─────────────────────────────────────────────────────┤
│  4. Replacement effects (checked before effect)      │
│     - If a replacement effect is registered for      │
│       this action, the replacement runs instead      │
├─────────────────────────────────────────────────────┤
│  5. Cleanup / follow-up                              │
│     - Pending commands from sequential sub-effects   │
│     - Optional cost followup actions                 │
└─────────────────────────────────────────────────────┘
```

### 3. Ability Queue State Machine

The ability system uses a queue-based state machine to handle abilities that
require player choices mid-resolution:

```
Idle → PayingCost → WaitingForChoice → ExecutingEffect → Completed
                    ↕                                      ↑
              (choice resumed) ───────────────────────────┘
```

- When a cost or effect creates a `Choice`, the flow pauses
- The choice is presented to the player
- When the player responds, execution resumes from the saved state
- This supports nested choices (choice inside sequential inside choice, etc.)

### 4. Cost Processing

Costs are validated before payment via `validate_cost()`:
- Checks if source zone has enough cards
- For sequential cost, validates each sub-cost
- For energy cost, checks energy zone count

Cost payment via `pay_cost_inner()`:
- `move_cards`: Moves cards from source → destination (with optional filtering)
- `pay_energy`: Deducts energy from energy zone
- `change_state`: Changes card orientation (active/wait)
- `reveal`: Reveals cards from hand
- `sequential_cost`: Iterates sub-costs, pauses on choice
- `choice_condition`: Creates a choice for the player
- `energy_condition`: Pops energy cards, decrements count
- `place_energy_under_member`: Places energy under a member

Optional costs:
- If a cost has `optional: true`, the player can skip it
- Skipping may trigger a different effect branch
- `handle_optional_cost_payment()` manages the skip/pay decision

### 5. Effect Execution Flow

Effects are dispatched via `execute_effect()` in `engine/src/ability/effects/mod.rs`:

1. Check `can_activate_effect()` -- if false, skip entirely
2. Check `non_stackable` flag -- deduplicate identical effects
3. If `action_by == "opponent"`, recurse with opponent context
4. Reset replacement effect flags for this effect
5. Handle replacement effects (choice-based or auto-apply)
6. Process `target="both"` by executing twice (self + opponent)
7. Dispatch to specific action handler

---

## Ability Structure

Each ability entry in `unique_abilities` has these top-level fields:

| Field | Type | Description |
| :--- | :--- | :--- |
| `full_text` | string | Complete ability text with icon placeholders, e.g. `{{kidou.png\|起動}}...` |
| `triggerless_text` | string | Full text with the trigger prefix removed |
| `card_count` | number | Number of card variants sharing this exact ability |
| `cards` | string[] | Card IDs using this ability, each with a card reference like `PL!-sd1-005-SD \| 星空 凛 (ab#0)` |
| `triggers` | string | Trigger type(s), comma-separated if multiple (e.g. `"起動,自動"`) |
| `use_limit` | number\|null | Limit on uses per turn (e.g. `1` for "once per turn") |
| `is_null` | boolean | True for placeholder abilities (usually from collapsed or null data) |
| `cost` | object\|null | The cost required to activate the ability |
| `effect` | object\|null | The effect that executes when activated |

---

## Cost Types

### `move_cards`
Moves cards from one zone to another as payment.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number of cards to move |
| `source` | string | Source zone (`hand`, `stage`, `discard`, `energy_zone`) |
| `destination` | string | Destination zone (`discard`, `energy_zone`, etc.) |
| `card_type` | string | Filter by card type (`member_card`, `live_card`, `energy_card`, `card`) |
| `optional` | boolean | Whether moving is optional |
| `self_cost` | boolean | Uses the activating card itself as cost |
| `zone` | string | Alternative zone field |
| `placement_order` | string | `any_order`, `deck_top_or_bottom` |
| `shuffle` | boolean | Shuffle source after movement |
| `cost_limit` | number | Maximum total cost of moved cards |
| `cost_limit_operator` | string | `>=`, `<=`, `>`, `<`, `==` |
| `same_unit_name` | boolean | Cards must share the same unit name |
| `characters` | string[] | Specific characters to include |
| `exclude_characters` | string[] | Characters to exclude |
| `group_names` | string[] | Filter by card group |
| `any_number` | boolean | Player can choose any number of cards |
| `exclude_self` | boolean | Exclude the activating card |
| `target` | string | Target player (`self`, `opponent`) |
| `text` | string | Human-readable cost description |

**Processing:** 
- If source is `hand` and NOT `same_unit`: builds filter, counts matching cards, 
  if count is optional or any_number, creates selection choice
- If `same_unit`: groups hand cards by unit name, requires N from same unit
- If source is not hand: validates match count, then moves cards

### `pay_energy`
Requires energy expenditure from the energy zone.

| Field | Type | Description |
| :--- | :--- | :--- |
| `energy` | number | Amount of energy required |
| `count` | number | Alternative count field |
| `zone` | string | `energy_zone` |
| `optional` | boolean | Whether payment is optional |
| `text` | string | Human-readable cost description |

**Processing:** Checks `baton_touch_zero_cost` flag. If optional (not activation), 
creates skip choice. Deducts energy via `player.energy_zone.pay_energy(energy)`.

### `change_state`
Changes a card's orientation (active/wait) as payment.

| Field | Type | Description |
| :--- | :--- | :--- |
| `state_change` | string | Target state (`wait`, `active`) |
| `card_type` | string | Filter by card type |
| `count` | number | Number of cards to change |
| `optional` | boolean | Whether state change is optional |
| `self_cost` | boolean | The activating card changes state |
| `exclude_self` | boolean | Exclude the activating card |
| `group_names` | string[] | Filter by card group |
| `text` | string | Human-readable cost description |

**Processing:** If optional (not activation), creates skip choice.
If `state_change == "wait"`: finds candidates via `get_change_state_candidates()`.
If candidates ≤ count: applies to all. If candidates > count: creates selection choice.

### `reveal`
Requires revealing cards from hand.

| Field | Type | Description |
| :--- | :--- | :--- |
| `card_type` | string | Filter by card type |
| `characters` | string[] | Specific characters to reveal |
| `exclude_characters` | string[] | Characters to exclude |
| `count` | number | Number of cards to reveal |
| `group_names` | string[] | Filter by card group |
| `source` | string | Source zone |
| `zone` | string | Alternative zone field |
| `optional` | boolean | Whether reveal is optional |
| `text` | string | Human-readable cost description |

**Processing:** Gets matching cards from hand by filter. If cards count ≤ explicit count: 
reveals all. Otherwise: creates selection choice for which cards to reveal.

### `sequential_cost`
Combines multiple costs that must all be paid in order.

| Field | Type | Description |
| :--- | :--- | :--- |
| `costs` | array | Array of sub-cost objects |
| `optional` | boolean | Whether entire sequential cost is optional |
| `text` | string | Human-readable cost description |

**Processing:** Iterates through sub-costs via `cost_paid_index`. Validates each then pays.
If a sub-cost creates a pending choice, pauses and waits for resolution before continuing.

### `choice_condition`
Cost that requires choosing between options.

| Field | Type | Description |
| :--- | :--- | :--- |
| `options` | array | Array of available choices (strings) |
| `text` | string | Human-readable cost description |

**Processing:** Creates a `Choice::SelectTarget` with `target: "choice_condition"`.
Presents options as a joined OR string. Sets `ChoiceRoute::ChoiceCost` on the queue entry.

### `energy_condition`
Checks and consumes energy cards from the energy zone (pops to energy_deck).

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number of energy cards to consume |
| `text` | string | Human-readable cost description |

**Processing:** Checks energy zone has `count` cards. Pops `count` cards from 
energy_zone and pushes to energy_deck. Decrements `active_energy_count`.

### `place_energy_under_member`
Places energy cards under a stage member as cost.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number of energy cards |
| `source` | string | Source zone |
| `destination` | string | `under_member` |
| `target` | string | Target player |
| `optional` | boolean | Whether placement is optional |
| `text` | string | Human-readable cost description |

**Processing:** Delegates to `execute_place_energy_under_member()`.

### `custom`
Fallback for complex/unparsed cost patterns (parsed from text when standard 
patterns don't match). Often resolves to `place_energy_under_member` or 
`move_cards` behavior.

| Field | Type | Description |
| :--- | :--- | :--- |
| `card_type` | string | Filter by card type |
| `count` | number | Number of cards |
| `destination` | string | Where to send cards |
| `target` | string | Target player |
| `optional` | boolean | Whether optional |
| `text` | string | Human-readable cost description |

---

## Effect Actions

### Card Movement

#### `move_cards`
Moves cards between zones.

| Field | Type | Description |
| :--- | :--- | :--- |
| `source` | string | Source zone |
| `destination` | string | Destination zone |
| `count` | number | Number of cards to move |
| `card_type` | string | Filter by card type |
| `name_constraint` | string | Required card name(s) |
| `name_constraint_source` | string | Source for name constraint (e.g. `cost_card`) |
| `state_change` | string | Apply state change during movement (`wait`, `active`) |
| `placement_order` | string | How cards are placed (`any_order`) |
| `cost_limit` | number | Filter by card cost (maximum) |
| `cost_limit_operator` | string | Comparison operator for cost_limit |
| `cost_total` | number | Filter by total cost of moved cards |
| `cost_total_operator` | string | Operator for cost_total |
| `need_heart_total` | number | Required heart total |
| `need_heart_operator` | string | Operator for need_heart_total |
| `need_heart_color` | string | Required heart color |
| `or_card_types` | string[] | Multiple allowed card types |
| `max` | boolean | "Up to N" selection limit |
| `distinct` | boolean | Items must have different names |
| `self_target` | boolean | Self is the target |
| `multiple_targets` | boolean | Selecting from multiple targets |
| `quoted_text` | string | Text in 「」 quotes |
| `value` | number | Numeric value (for scored cards) |
| `dynamic_count` | object | Runtime-calculated count |
| `condition` | object | Condition for execution |
| `all` | boolean | Apply to all eligible targets |
| `exclude_self` | boolean | Exclude activating card |
| `group_names` | string[] | Filter by card group |
| `heart_colors` | string[] | Filter by heart color |
| `activation_condition_parsed` | object | Parsed activation condition |
| `optional` | boolean | Whether movement is optional |
| `target` | string | Target player |
| `text` | string | Human-readable effect description |

#### `draw_card`
Draws cards from the deck.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number of cards to draw |
| `source` | string | Source zone (`deck`) |
| `destination` | string | Destination zone (`hand`) |
| `card_type` | string | Filter by card type |
| `condition` | object | When to draw (e.g., if in specific position) |
| `heart_colors` | string[] | Heart color requirements |
| `trigger_condition` | object | Special trigger conditions |
| `trigger_type` | string | Trigger type filter |
| `position` | string | Position-based trigger |
| `original_value` | number | Compare to original value |
| `per_unit` | boolean | Scale with count of something |
| `per_unit_count` | number | Count per unit |
| `per_unit_type` | string | Unit type for per_unit |
| `exclude_self` | boolean | Exclude activating card |
| `duration` | string | Effect duration |
| `state` | string | State filter |
| `activation_condition_parsed` | object | Parsed activation condition |
| `group_names` | string[] | Filter by card group |
| `target` | string | Target player |
| `text` | string | Human-readable effect description |

**Processing:** Handles optional draw, `any_number` selection, dynamic counts.
Source is always deck, destination is always hand.

#### `draw_until_count`
Draws cards until the player's hand reaches a target size.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Target hand count |
| `target_count` | number | Additional count parameter |
| `target` | string | Target player |
| `source` | string | Source zone (`deck`) |
| `destination` | string | Destination zone (`hand`) |
| `condition` | object | Stopping condition |
| `text` | string | Human-readable effect description |

#### `look_and_select`
Look at cards from the deck, then select some to take.

| Field | Type | Description |
| :--- | :--- | :--- |
| `look_action` | object | Action to look at cards (has `action: look_at`) |
| `select_action` | object | Action to select from looked cards (has `action: select_cards`) |
| `condition` | object | Conditional execution |
| `heart_colors` | string[] | Filter by heart colors |
| `exclude_self` | boolean | Exclude activating card |
| `group_names` | string[] | Filter by card group |
| `original_value` | number | Compare to original value |
| `parenthetical` | array | Clarification notes |
| `text` | string | Human-readable effect description |

#### `look_at`
Look at cards without taking them.

| Field | Type | Description |
| :--- | :--- | :--- |
| `source` | string | Source zone (`deck_top`, `revealed_cards`) |
| `count` | number | Number of cards to look at |
| `target` | string | Target player |

#### `select_cards`
Select cards after looking (used inside `look_and_select`).

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number to select |
| `destination` | string | Where selected cards go |
| `placement_order` | string | How to place (`any_order`) |
| `any_number` | boolean | Any number selection |
| `discard_remaining` | boolean | Discard unselected cards |
| `reveal` | boolean | Reveal on selection |

#### `reveal`
Reveal cards from a zone.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number to reveal |
| `card_type` | string | Filter by card type |
| `multiple_targets` | boolean | Reveal from multiple targets |
| `source` | string | Source zone |

### Resource Management

#### `gain_resource`
Increases score, blades, or hearts on stage members.

| Field | Type | Description |
| :--- | :--- | :--- |
| `resource` | string | Type of resource (`heart`, `blade`, `score`) |
| `count` | number | Amount to gain |
| `heart_selection` | boolean | Player-selectable heart color |
| `filter_targets_by_heart_colors` | boolean | Filter by heart color |
| `per_unit` | boolean | Scale with count of something |
| `per_unit_count` | number | Count per unit |
| `per_unit_type` | string | Unit type (e.g. `live_card_zone`, `stage`) |
| `location` | string | Zone to count for per_unit |
| `dynamic_count` | object | Count calculated at runtime |
| `sign` | string | Add or subtract resource (`+`, `-`) |
| `target_count` | number | Number of targets to affect |
| `heart_colors` | string[] | Array of heart color identifiers |
| `group_reference` | string | Reference group for calculation |
| `multiple_targets` | boolean | Multiple target selection |
| `condition` | object | Execution condition |
| `duration` | string | How long the gain lasts |
| `state` | string | State filter |
| `trigger_condition` | object | Trigger condition |
| `trigger_type` | string | Trigger type filter |
| `cost_limit` | number | Cost constraint |
| `cost_limit_operator` | string | Operator for cost_limit |
| `all` | boolean | Apply to all members |
| `max` | boolean | "Up to N" limit |
| `self_target` | boolean | Self is the target |
| `exclude_self` | boolean | Exclude self |
| `position` | string | Position filter |
| `activation_position` | string | Position where activation occurs |
| `card_type` | string | Filter by card type |
| `original_value` | number | Compare to original value |
| `target` | string | Target player |
| `text` | string | Human-readable effect description |

**Processing:** The `gain_resource` action targets stage members and increments
their blade, heart, or score counters. Supports scaling (`per_unit`) based on 
count of cards in a zone, and filtering by heart colors.

#### `pay_energy`
Pays energy as an effect (rather than as a cost).

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Energy to pay |
| `target` | string | Target player |

#### `place_energy_under_member`
Moves energy cards to underneath a stage member.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number of energy cards |
| `destination` | string | `under_member` |
| `energy_count` | number | Energy amount |
| `source` | string | Source zone (`energy_zone`) |
| `target` | string | Target player |
| `card_type` | string | Filter |
| `group_names` | string[] | Filter by group |
| `target_member` | string | Which member to target |
| `state_change` | string | Apply state change |
| `optional` | boolean | Whether placement is optional |
| `condition` | object | Execution condition |
| `cost_limit` | number | Cost constraint |
| `cost_limit_operator` | string | Operator for cost_limit |
| `text` | string | Human-readable effect description |

### State & Position Changes

#### `change_state`
Modifies a card's orientation (active/wait).

| Field | Type | Description |
| :--- | :--- | :--- |
| `state_change` | string | Target state (`wait`, `active`) |
| `count` | number | Number of cards to affect |
| `card_type` | string | Filter by card type |
| `location` | string | Zone where cards are located |
| `per_unit` | boolean | Scale with count |
| `blade_limit` | number | Blade count constraint |
| `blade_limit_operator` | string | Operator for blade_limit |
| `cost_limit` | number | Cost constraint |
| `cost_limit_operator` | string | Operator for cost_limit |
| `distinct` | boolean | Items must have different names |
| `max` | boolean | "Up to N" limit |
| `optional` | boolean | Whether state change is optional |
| `target` | string | Target player |
| `position` | string | Position requirement |
| `original_value` | number | Compare to original/natural value |
| `values` | array | Array of possible values for condition |
| `activation_position` | string | Position where activation occurs |
| `all` | boolean | Apply to all eligible |
| `condition` | object | Execution condition |
| `exclude_self` | boolean | Exclude self |
| `group_names` | string[] | Filter by group |
| `parenthetical` | array | Clarification notes |
| `text` | string | Human-readable effect description |

#### `position_change`
Changes a member's stage position (center, left, right).

| Field | Type | Description |
| :--- | :--- | :--- |
| `position` | string | Target position |
| `source_position` | string | Source position |
| `exclude_position` | string | Position to exclude |
| `count` | number | Number to move |
| `card_type` | string | Filter by card type |
| `destination` | string | Destination zone |
| `multiple_targets` | boolean | Multiple targets |
| `optional` | boolean | Whether optional |
| `target` | string | Target player |
| `target_member` | string | Specific member target |
| `condition` | object | Execution condition |
| `duration` | string | Effect duration |
| `group_names` | string[] | Filter by group |
| `parenthetical` | array | Clarification notes |
| `text` | string | Human-readable effect description |

#### `play_baton_touch`
Executes a baton touch (member replacement on stage).

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number of baton touches |
| `text` | string | Full text |

### Score & Hearts

#### `modify_score`
Adjusts the score value on live cards.

| Field | Type | Description |
| :--- | :--- | :--- |
| `operation` | string | `add`, `subtract`, `set` |
| `value` | number | Amount of change |
| `target` | string | Target player |
| `card_type` | string | Filter by card type |
| `per_unit` | boolean | Scale with count |
| `per_unit_count` | number | Count per unit |
| `per_unit_type` | string | Unit type |
| `heart_colors` | string[] | Heart color filter |
| `location` | string | Zone for per_unit count |
| `position` | string | Position requirement |
| `duration` | string | Effect duration |
| `self_target` | boolean | Self-targeting |
| `condition` | object | Execution condition |
| `activation_position` | string | Position where activation occurs |
| `distinct` | boolean | Different names |
| `group_names` | string[] | Filter by group |
| `state` | string | State filter |
| `source` | string | Source zone reference |
| `text` | string | Human-readable effect description |

#### `modify_required_hearts`
Adjusts the heart requirements on a live card.

| Field | Type | Description |
| :--- | :--- | :--- |
| `operation` | string | Type of modification |
| `value` | number | Amount of change |
| `card_type` | string | Filter by card type |
| `target` | string | Target player |
| `heart_colors` | string[] | Which hearts to modify |
| `location` | string | Zone filter |
| `duration` | string | Effect duration |
| `per_unit` | boolean | Scale with count |
| `position` | string | Position requirement |
| `self_target` | boolean | Self-targeting |
| `timing_condition` | object | When condition is met |
| `condition` | object | Execution condition |
| `count` | number | Count parameter |
| `distinct` | boolean | Different names |
| `group_names` | string[] | Filter by group |
| `non_stackable` | boolean | Prevent stacking |
| `original_count` | number | Original count reference |
| `original_operator` | string | Operator for original_count |
| `original_value` | number | Original value reference |
| `parenthetical` | array | Clarification notes |
| `text` | string | Human-readable effect description |

#### `modify_yell_count`
Adjusts the yell (cheer) activation count.

| Field | Type | Description |
| :--- | :--- | :--- |
| `operation` | string | `add`, `subtract`, `set` |
| `count` | number | New yell count |
| `condition` | object | When to apply |
| `duration` | string | Effect duration |
| `exclude_self` | boolean | Exclude the acting card |
| `text` | string | Human-readable effect description |

### Blade & Heart

#### `set_blade_count`
Sets a member's blade count to a specific value.

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Target blade count |
| `blade_limit` | number | Constraint on setting |
| `blade_limit_operator` | string | Operator for blade_limit |
| `card_type` | string | Filter by card type |
| `target` | string | Target player |
| `duration` | string | Effect duration |
| `original_value` | number | Compare to original value |
| `position` | string | Position requirement |
| `group_names` | string[] | Filter by group |
| `text` | string | Human-readable effect description |

#### `set_blade_type`
Sets the blade type/color (red, blue, green, yellow, purple).

| Field | Type | Description |
| :--- | :--- | :--- |
| `blade_type` | string | The blade type to set |
| `duration` | string | Effect duration |
| `text` | string | Human-readable effect description |

### Ability Manipulation

#### `gain_ability`
Grants a new ability to a card.

| Field | Type | Description |
| :--- | :--- | :--- |
| `ability_gain` | string | The gained ability text |
| `ability_text` | string | Alternative text field |
| `count` | number | Number of abilities |
| `duration` | string | How long ability lasts |
| `target` | string | Target player |
| `group_names` | string[] | Filter by group |
| `activation_condition_parsed` | object | Parsed activation condition |
| `activation_position` | string | Position where activation occurs |
| `max` | boolean | "Up to N" abilities |
| `parenthetical` | array | Clarification notes |
| `card_type` | string | Filter by card type |
| `condition` | object | Execution condition |
| `text` | string | Human-readable effect description |

#### `gain_ability_from_source`
Copies abilities from cards under a member.

| Field | Type | Description |
| :--- | :--- | :--- |
| `card_type` | string | Filter by card type |
| `source_location` | string | Where to look (`under_member`) |
| `cost_limit` | number | Cost constraint |
| `cost_limit_operator` | string | Operator for cost_limit |
| `group_names` | string[] | Filter by group |
| `trigger_filter` | string | Filter trigger types |
| `all` | boolean | Apply to all |
| `text` | string | Human-readable effect description |

#### `activate_ability`
Activates a stored or gained ability (usually on another card).

| Field | Type | Description |
| :--- | :--- | :--- |
| `ability_text` | string | Text of ability to activate |
| `target_trigger` | string | Which trigger to activate |
| `source_card` | string | Source card reference (`cost_card`) |
| `count` | number | Number of activations |
| `parenthetical` | array | Clarification notes |
| `target` | string | Target player |
| `text` | string | Human-readable effect description |

#### `invalidate_ability`
Negates or disables abilities on the activating card.

| Field | none | No special fields |

### Cost Modifications

#### `modify_cost`
Adjusts the printed cost value of cards.

| Field | Type | Description |
| :--- | :--- | :--- |
| `operation` | string | Type of modification |
| `value` | number | Amount of change |
| `non_stackable` | boolean | Prevent stacking |
| `card_type` | string | Filter by card type |
| `target` | string | Target player |
| `condition` | object | When to apply |
| `per_unit` | boolean | Scale with count |
| `location` | string | Zone for per_unit count |
| `duration` | string | Effect duration |
| `original_count` | number | Original count reference |
| `original_operator` | string | Operator for original_count |
| `original_value` | number | Original value reference |
| `cost_limit` | number | Cost constraint |
| `cost_limit_operator` | string | Operator |
| `dynamic_count` | object | Runtime-calculated count |
| `count` | number | Count parameter |
| `destination` | string | Destination reference |
| `exclude_self` | boolean | Exclude self |
| `group_names` | string[] | Filter by group |
| `per_unit_location` | string | Zone for per_unit count |
| `per_unit_count` | number | Count per unit |
| `per_unit_type` | string | Unit type |
| `source` | string | Source reference |
| `text` | string | Human-readable effect description |

### Restrictions

#### `restriction`
Applies a game restriction effect.

| Field | Type | Description |
| :--- | :--- | :--- |
| `restriction_type` | string | Type of restriction |
| `operation` | string | Modify operation |
| `value` | number | Restriction value |
| `target` | string | Target player |
| `self_target` | boolean | Self-targeting |
| `phase` | string | Game phase restriction |
| `card_type` | string | Filter by card type |
| `count` | number | Number of restrictions |
| `condition` | object | Execution condition |
| `all` | boolean | Apply to all |
| `duration` | string | Effect duration |
| `exclude_self` | boolean | Exclude self |
| `group_names` | string[] | Filter by group |
| `heart_colors` | string[] | Filter by heart color |
| `text` | string | Human-readable effect description |

### Identity

#### `set_card_identity`
Sets card identity flags (used for name-based matching).

| Field | Type | Description |
| :--- | :--- | :--- |
| `identities` | string[] | Array of identity strings |
| `all` | boolean | Apply to all |
| `all_regions` | boolean | Apply across all game regions |
| `self_target` | boolean | Self-targeting |
| `group_names` | string[] | Filter by group |
| `text` | string | Human-readable effect description |

### Player Choice

#### `choice`
Forces a player to choose between options.

| Field | Type | Description |
| :--- | :--- | :--- |
| `options` | array | Array of choice options (each with its own `action` and fields) |
| `choice_type` | string | How choice is made |
| `choice_maker` | string | Who makes the choice (`self`, `opponent`) |
| `question` | string | Prompt for the choice |
| `target` | string | Target player |
| `count` | number | Number of choices to make |
| `exclude_self` | boolean | Exclude self from choices |
| `group_reference` | string | Reference group |
| `all` | boolean | Apply to all |
| `condition` | object | Execution condition |
| `group_names` | string[] | Filter by group |
| `heart_colors` | string[] | Filter by heart color |
| `optional` | boolean | Whether choice is optional |
| `original_value` | number | Compare to original value |
| `parenthetical` | array | Clarification notes |
| `position` | string | Position requirement |
| `text` | string | Human-readable effect description |

**Processing:** Creates a `Choice::SelectTarget` with appropriate target type.
The choice options are presented to the player, and the selected option's 
sub-effects execute.

#### `select`
Simpler selection (often for heart color or basic option picking).

| Field | Type | Description |
| :--- | :--- | :--- |
| `count` | number | Number to select |
| `heart_colors` | string[] | Available heart color choices |
| `original_value` | number | Compare to original value |

### Other Actions

#### `opponent_action`
Requires the opponent to take an action (the opponent becomes the actor).

| Field | Type | Description |
| :--- | :--- | :--- |
| `opponent_action` | object | The action the opponent performs |
| `action_by` | string | `opponent` |
| `condition` | object | Execution condition |
| `activation_position` | string | Position filter |
| `group_names` | string[] | Filter by group |
| `parenthetical` | array | Clarification notes |
| `text` | string | Human-readable effect description |

**Processing:** Recurses into `execute_effect()` with the opponent as the actor.

---

## Compound / Wrapper Effects

### `sequential`
Executes an array of sub-actions in order.

| Field | Type | Description |
| :--- | :--- | :--- |
| `actions` | array | Ordered list of effect objects to execute sequentially |
| `character_effects` | array | Effects tied to specific characters (Live-specific) |
| `condition` | object | Condition that gates the entire sequence |
| `conditional` | object | Internal conditional branching within sequence |
| `activation_position` | string | Position where activation occurs |
| `activation_condition_parsed` | object | Parsed activation conditions |
| `all` | boolean | Apply to all |
| `count` | number | Count parameter |
| `destination` | string | Destination reference |
| `distinct` | boolean | Distinct names |
| `duration` | string | Effect duration |
| `exclude_self` | boolean | Exclude self |
| `group_names` | string[] | Filter by group |
| `heart_colors` | string[] | Filter by heart color |
| `max` | boolean | "Up to N" limit |
| `original_value` | number | Original value reference |
| `parenthetical` | array | Clarification notes |
| `position` | string | Position requirement |
| `source` | string | Source reference |
| `target` | string | Target player |
| `text` | string | Human-readable effect description |
| `trigger_condition` | object | Trigger condition |
| `trigger_type` | string | Trigger type |

**Processing:** Iterates through `actions`. Each sub-effect is dispatched through
`execute_effect()`. If a sub-effect creates a pending choice, the sequence pauses 
and resumes from where it left off. `character_effects` provides character-specific
sequencing for live/event cards.

### `conditional_alternative`
If/else branching. Checks a condition, executes `primary_effect` if true, 
`alternative_effect` if false.

| Field | Type | Description |
| :--- | :--- | :--- |
| `condition` | object | The condition to check |
| `primary_effect` | object | Effect if condition is true |
| `alternative_effect` | object | Effect if condition is false |
| `activation_condition_parsed` | object | Parsed activation condition |
| `activation_position` | string | Position filter |
| `group_names` | string[] | Filter by group |
| `parenthetical` | array | Clarification notes |
| `position` | string | Position requirement |
| `text` | string | Human-readable description |

**Processing:** Evaluates the condition at runtime. Calls `execute_effect()` for
the matching branch.

### `conditional_on_result`
Executes a `primary_effect`, then based on `result_condition`, optionally runs
a `followup_action`.

| Field | Type | Description |
| :--- | :--- | :--- |
| `result_condition` | object | Condition evaluated after primary_effect |
| `primary_effect` | object | The main effect |
| `followup_action` | object | Additional action if result_condition met |
| `all` | boolean | Apply to all |
| `group_names` | string[] | Filter by group |
| `heart_colors` | string[] | Filter by heart color |
| `original_value` | number | Original value reference |
| `shuffle` | boolean | Shuffle after result |
| `text` | string | Human-readable description |

### `conditional_on_optional`
Executes based on whether an optional cost was paid or skipped.

| Field | Type | Description |
| :--- | :--- | :--- |
| `optional_action` | object | Action if optional cost was skipped (or paid, depending on flag) |
| `conditional_action` | object | Action if condition is met |
| `conditional_negation` | boolean | Negate the condition flag |
| `text` | string | Human-readable description |

### `repeat_procedure`
Repeats a sub-effect a specified number of times (engine-level, may not appear in JSON).

### `discard_until_count`
Discards cards until the player's hand has a target count.

---

## Condition Types

Conditions are evaluated via `evaluate_condition()` at runtime. The `type` field 
selects the evaluation logic. Common fields across all conditions:

| Field | Type | Description |
| :--- | :--- | :--- |
| `text` | string | Original condition text |
| `type` | string | Condition type identifier |
| `target` | string | Target player (`self`, `opponent`, `both`) |
| `location` | string | Zone to check |
| `locations` | string[] | Multiple zones (array) |
| `count` | number | Count value |
| `operator` | string | Comparison operator (`>=`, `<=`, `>`, `<`, `==`) |
| `card_type` | string | Filter by card type |
| `group_names` | string[] | Filter by card group |
| `characters` | string[] | Specific characters to check |
| `exclude_characters` | string[] | Characters to exclude |
| `negation` | boolean | Negate condition |
| `distinct` | boolean | Distinct names/costs |
| `position` | string | Position requirement |
| `position_compare` | string | Cross-position comparison |
| `all_areas` | boolean | Check all stage areas |

### `card_count_condition`
Counts cards matching filters in a zone, then compares against a threshold.

| Field | Type | Description |
| :--- | :--- | :--- |
| `location` | string | Zone to count (`hand`, `deck`, `discard`, `energy_zone`, `stage`, `live_card_zone`, `waitroom`) |
| `temporal` | string | Timing scope (`this_turn`, `during_live`) |
| `unit` | string | Unit for count (`人`, `枚`, `つ`, `types`) |
| `card_property` | string | Property filter (e.g. `has_all_blade`) |
| `preceding_moved` | boolean | Count cards that were just moved |

**Processing:** Gets count from `count_cards_with_filters()`. If `unit == "types"`, 
counts distinct heart colors instead of card count. Supports `aggregate=total` 
for summing blades.

### `comparison_condition`
Compares a counted value against a threshold.

| Field | Type | Description |
| :--- | :--- | :--- |
| `comparison_type` | string | What to compare (`cost`, `score`) |
| `resource_type` | string | Resource type (`energy`, `hand_count`, `surplus_heart`) |
| `location` | string | Zone for comparison |

**Processing:** Uses `get_count_for_condition()` to resolve the numeric value,
then compares against `count` using `operator`.

### `location_condition`
Checks card presence/absence in a zone (what cards exist there).

| Field | Type | Description |
| :--- | :--- | :--- |
| `location` | string | Zone to check |
| `all_areas` | boolean | Check all stage areas |
| `distinct` | boolean | Distinct names required |
| `card_property` | string | Property check (e.g. heart type, blade) |

**Processing:** Evaluates via `evaluate_location_condition()`. Supports multi-location 
checks, blade/heart content checks, aggregate total checks (summing blades across zone), 
distinct name checks.

### `compound`
Logical AND/OR of sub-conditions.

| Field | Type | Description |
| :--- | :--- | :--- |
| `operator` | string | `and`, `or` (or `any_of` for alternative compound) |
| `conditions` | array | Array of sub-condition objects |
| `all_areas` | boolean | Check all areas |
| `scope` | string | Scope for comparison (`both`) |

**Processing:** Evaluates all sub-conditions with the logical operator. Short-circuits 
on first failure (for AND) or first success (for OR). `AnyOfCondition` is a separate 
type that checks predefined categories (has_member, has_energy, etc.).

### `temporal_condition`
Time-based condition. Checks if we're in a specific timing window.

| Field | Type | Description |
| :--- | :--- | :--- |
| `temporal` | string | Time scope (`this_turn`, `during_live`, `live_end`, `before_live`) |
| `phase` | string | Game phase (`main_phase`, `live_phase`) |
| `condition` | object | Nested condition (evaluated within the temporal scope) |
| `turn_number` | number | Specific turn number |
| `no_excess_heart` | boolean | No surplus heart requirement |

**Processing:** Checks timing first. If the time matches, evaluates nested condition.
Supports `NotMoved`/`HasMoved` as nested conditions within temporal scope.

### `state_condition`
Checks card orientation state (active/wait).

| Field | Type | Description |
| :--- | :--- | :--- |
| `state` | string | State to check (`wait`, `active`) |
| `negation` | boolean | Negate the check |
| `all` | boolean | Check all cards in zone |

### `appearance_condition`
Checks if specific cards are on stage (debut/live start check).

| Field | Type | Description |
| :--- | :--- | :--- |
| `appearance` | boolean | Always true (marker) |
| `location` | string | Zone (`stage`) |
| `baton_touch_trigger` | boolean | Baton touch trigger marker |
| `placement_order` | string | Placement requirement |
| `activation_position` | string | Position requirement |
| `all_areas` | boolean | All areas |
| `cost_reference_character` | string | Character for cost comparison |
| `cost_reference_operator` | string | Comparison operator |
| `cost_reference_type` | string | Reference type |

### `movement_condition`
Checks card movement status (has a card moved this turn?).

| Field | Type | Description |
| :--- | :--- | :--- |
| `movement` | string | Movement type (`moved`, `moves`) |
| `movement_state` | string | State (`has_moved`, `to_stage`, `from_stage`) |
| `negation` | boolean | Negate |
| `self_effect_only` | boolean | Only check own effects |
| `energy_placed` | boolean | Energy placement trigger |

### `or_condition`
Simple OR of sub-conditions (simpler than compound, no operator field).

| Field | Type | Description |
| :--- | :--- | :--- |
| `conditions` | array | Array of sub-conditions |

### `group_condition`
Checks cards matching a group/unit.

| Field | Type | Description |
| :--- | :--- | :--- |
| `group_names` | string[] | Groups to check |
| `all_members` | boolean | All members of group |
| `heart_colors` | string[] | Heart color coverage check |
| `exclude_self` | boolean | Exclude self |

### `position_condition`
Checks which stage positions are occupied.

### `card_blade_condition`
Sums blades from selected/moved cards.

### `ability_filter_condition`
Filters cards by ability presence.

| Field | Type | Description |
| :--- | :--- | :--- |
| `ability` | string | Required ability type |
| `no_ability` | boolean | Card must not have ability |
| `has_ability` | boolean | Card must have ability |
| `no_ability_type` | string | Excluded trigger type |

### `choice_condition`
Checks if choice options exist (non-empty).

### `position_change_condition`
Checks if a position change occurred this turn.

### `state_change_condition`
Checks orientation transitions.

| Field | Type | Description |
| :--- | :--- | :--- |
| `from_state` | string | Source state |
| `to_state` | string | Target state |
| `cost_limit` | number | Cost limit filter |
| `cost_limit_operator` | string | Operator |

### `opponent_choice_condition`
Checks if opponent made/declined a choice.

| Field | Type | Description |
| :--- | :--- | :--- |
| `negation` | boolean | Check if opponent declined |

### `opponent_live_success`
Checks if opponent succeeded in a live this turn.

| Field | Type | Description |
| :--- | :--- | :--- |
| `no_excess_heart` | boolean | Additionally check no excess heart |

### `any_of_condition`
Checks predefined categories:

| Value | Checks |
| :--- | :--- |
| `has_member` | Player has a member on stage |
| `has_energy` | Player has energy |
| `has_hand` | Player has cards in hand |
| `has_blade_heart` | Cards have blade hearts |
| `has_live_card` | Player has a live card |
| `is_active_phase` | Current phase is active |
| `is_main_phase` | Current phase is main |

### Other conditions

| Type | Description |
| :--- | :--- |
| `score_threshold_condition` | Compare cheer blade heart count vs threshold |
| `complex_condition` | Nested cause-effect condition |
| `no_excess_heart` | Check no-excess-heart flag for self or opponent |
| `otherwise_condition` | Catch-all, always true when reached |
| `not_moved` / `has_moved` | Card movement check (only meaningful within temporal) |
| `custom` | Parser-only marker, always true |
| `energy_state_condition` | Check energy zone state |

---

## Zones Reference

| Key | Engine Zone | Description |
| :--- | :--- | :--- |
| `deck` | `Deck` | Main deck |
| `deck_top` | `DeckTop` | Top of the deck |
| `deck_bottom` | `DeckBottom` | Bottom of the deck |
| `deck_position_N` | (parsed) | Specific position N from top (e.g. `deck_position_4`) |
| `discard` | `Discard`/`Waitroom` | Discard pile (控え室) |
| `energy_zone` | `Energy`/`EnergyZone` | Energy area |
| `energy_deck` | `EnergyDeck` | Energy deck (deck of energy cards) |
| `hand` | `Hand` | Player's hand |
| `stage` | `Stage` | Active stage (all positions) |
| `center` | `StageCenter` | Center stage position |
| `left_side` | `StageLeft` | Left side stage area |
| `right_side` | `StageRight` | Right side stage area |
| `empty_area` | `EmptyArea` | Unoccupied stage slot |
| `front` | (parsed) | Front of the stage (for live cards) |
| `same_area` | `SameArea` | Current position |
| `under_member` | `UnderMember` | Underneath a member card |
| `success_zone` | `SuccessZone` | Successful live results area |
| `live_card_zone` | `LiveCardZone` | Live card zone (active live cards) |
| `success_live_zone` | `SuccessLiveZone` | Combined success + live zone |
| `revealed_cards` | `RevealedCards` | Recently revealed cards |
| `looked_at` | `LookedAt` | Cards being looked at |
| `selected_cards` | `SelectedCards` | Currently selected cards |
| `resolution` | `Resolution` | Resolution zone |
| `exclusion_zone` | `ExclusionZone` | Zone for excluded cards |
| `those_cards` | (parsed) | Cards from trigger event |

---

## Triggers Reference

| Key | Engine Constant | Description |
| :--- | :--- | :--- |
| `起動` | `ACTIVATION` | Activated ability (manual activation by player) |
| `自動` | `AUTO` | Auto trigger (automatic ability on condition) |
| `常時` | `CONSTANT` | Continuous/always active (passive ability) |
| `登場` | `DEBUT` | When card enters stage |
| `ライブ開始時` | `LIVE_START` | At start of live performance |
| `ライブ成功時` | `LIVE_SUCCESS` | When live succeeds |
| `メイン` | `MAIN` | Main phase trigger |
| `baton touch` | `BATON_TOUCH` | Baton touch trigger |
| `Debut` | `DEBUT_EN` | English alias for 登場 |
| `live_success` | `LIVE_SUCCESS_EN` | English alias for ライブ成功時 |

---

## Card Types Reference

| Type | Description |
| :--- | :--- |
| `member_card` | Member card (main character card) |
| `live_card` | Live card (performance card) |
| `energy_card` | Energy card |
| `card` | Any card type |

---

## Common Fields

Fields that appear across multiple cost types and effect actions:

| Field | Description |
| :--- | :--- |
| `count` | The number of cards or resources involved |
| `card_type` | Filters to specific card types |
| `source` | The zone where the action originates |
| `destination` | The zone where cards are moved to |
| `target` | The intended target (`self`, `opponent`, `both`, `either`) |
| `condition` | A requirement that must be met |
| `duration` | How long the effect lasts (`live_end`, `this_turn`, `this_live`, `turn_end`) |
| `group_names` | Filters for specific card groups |
| `optional` | Whether the action is mandatory |
| `parenthetical` | Additional context text (array of strings) |
| `all` | Boolean, applies to all eligible targets |
| `exclude_self` | Boolean, excludes the activating card |
| `placement_order` | How cards are placed (`any_order`, `deck_top_or_bottom`) |
| `shuffle` | Boolean, shuffle source zone after movement |
| `self_cost` | Boolean, cost uses the activating card |
| `any_number` | Boolean, player can choose any number |
| `max` | Boolean, "up to N" limit |
| `distinct` | Boolean, items must differ |
| `position` | Position requirement (`center`, `left_side`, `right_side`) |
| `source_position` | Source position for movement |
| `exclude_position` | Position to exclude |
| `per_unit` | Boolean, effect scales with count |
| `per_unit_count` | The count per unit |
| `per_unit_type` | Unit type (`枚`, `人`, etc.) |
| `per_unit_location` | Zone where per_unit count is calculated |
| `text` | Human-readable description of the cost/effect |

---

## Additional Parser Fields

Fields produced by the ability parser that may appear on cost or effect objects:

| Field | Description |
| :--- | :--- |
| `dynamic_count` | Object for runtime-calculated counts |
| `reference` | Reference for dynamic calculation |
| `mode` | Calculation mode (`relative_to`, `fixed`) |
| `base_reference` | Base reference point |
| `calculation` | Calculation type (`add`, `subtract`) |
| `calculation_value` | Value for calculation |
| `modification_type` | Type of cost modification |
| `cost_threshold` | Threshold for cost change |
| `threshold_operator` | Operator for threshold |
| `secondary_effect` | Follow-up effect |
| `cost_total` / `cost_total_operator` | Total cost constraint |
| `choices_made` | Track player selections |
| `trigger_filter` | Filter triggers by name |
| `gained_effect` | Pre-parsed effect from gained ability |
| `quoted_text` | Text in 「」 quotes |
| `replacement` | Replacement effect data |
| `replaces_event` | What event this replaces |
| `restriction_type` | Type of restriction |
| `restricted_destination` | Where restriction applies |
| `energy_count` | Energy count (alternative to `energy`) |
| `activation_condition_parsed` | Parsed activation condition as an object |
| `values` | Array of possible values for condition |
| `or_card_types` | Multiple allowed card types |
| `need_heart_color` | Required heart color |
| `need_heart_operator` | Operator for need_heart_total |
| `need_heart_total` | Required heart total |

---

## Edge Cases & Special Behaviors

### Use Limits
- `use_limit: 1` means the ability can only be used once per turn
- Tracked per-card-instance, not per-ability-definition
- Resets at turn end

### Constant Abilities
- `常時` triggers are evaluated continuously, not as discrete activations
- They don't go through the cost → effect pipeline
- Instead they apply modifiers that are checked as needed

### Optional Costs
- If a cost has `optional: true` and is NOT part of an activation ability, 
  the player gets a skip choice
- Skipping the cost sends the flow to the optional handling path
- `handle_optional_cost_payment()` manages the skip → alternative flow
- The alternative effect may be specified in the cost's `alternative_effect`

### Non-stackable Effects
- The `non_stackable` flag prevents the same effect from being applied multiple times
- Deduplication is based on the `action:text` key
- If an effect with the same key is already active, subsequent applications are skipped

### Replacement Effects
- Registered before the main effect executes
- If a replacement matches the incoming effect, it replaces the behavior
- Can be auto-apply or choice-based
- Tracked via `effect_type == "replacement"`

### Baton Touch Zero Cost
- A special flag `baton_touch_zero_cost` can override energy payment
- If true and energy > 0, the energy cost is skipped
- Used for baton touch abilities that should be free

### `target="both"`
- When an effect's target is `"both"`, the effect executes twice:
  once for self, once for opponent
- Handled generically in the effect dispatch

### `action_by="opponent"`
- When set, the opponent becomes the actor
- The effect recurses through `execute_effect()` with opponent context

### Target Selection
- `SelectTargetKind` defines 14+ kinds of selections:
  `Choice`, `ChoiceString`, `PayOptionalCostSkipOptionalCost`, 
  `DoubleBatonTouch`, `PrimaryAlternative`, `ApplyReplacement`,
  `ChooseRequiredHearts`, `PositionDestination`, `HeartColor`,
  `ChoiceType`, `ChoiceCondition`, `ConditionalOptional`,
  `DrawAnyNumber`, `Order`
- Each triggers different UI/choice behavior

### Cost Type: `unknown`
- Appears when `abilities.json` has no cost (null cost)
- These are typically constant (`常時`) abilities or abilities without costs
- 527 abilities have `unknown` cost type (the largest group)

### Effect Action: `unknown`
- Appears when an effect has no `action` field
- Very rare (only 2 abilities)
