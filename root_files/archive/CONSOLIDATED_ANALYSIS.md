# Consolidated Analysis - Parser and Engine Issues

This document consolidates all analysis from ABILITIES_ANALYSIS_COMPLETE.md, PARSER_ENGINE_ALIGNMENT_SUMMARY.md, PARSER_ENGINE_GAPS.md, and PARSER_ENGINE_ISSUES.md.

---

# Comprehensive Abilities.json to Engine Analysis

## Summary

Systematically analyzed all abilities in `abilities.json` (100 unique abilities) and compared every field with the Rust engine implementation in `ability_resolver.rs` and related files.

## Field Analysis

### Fields Added to Rust Structs

The following fields were missing from Rust structs and have been added:

1. **AbilityEffect struct** (card.rs):
   - `lose_blade_hearts: Option<bool>` - For re_yell action
   - `conditional: Option<bool>` - For sequential effects
   - `choice_type: Option<String>` - For choice actions
   - `heart_type: Option<String>` - For set_heart_type action
   - `values: Option<Vec<u32>>` - For comparison conditions

2. **Condition struct** (card.rs):
   - `values: Option<Vec<u32>>` - For comparison conditions with multiple valid values

### Parser Verification

All missing fields are correctly extracted by `parser.py`:
- `lose_blade_hearts` - Line 2435
- `conditional` - Lines 2030, 2176
- `choice_type` - Line 2541
- `heart_type` - Line 1506
- `values` - Line 768

## Action Type Analysis

### Action Usage Counts

- move_cards: 222
- gain_resource: 145
- draw_card: 90
- sequential: 87
- change_state: 69
- look_and_select: 58
- modify_score: 53
- position_change: 13
- select: 18
- gain_ability: 12
- reveal: 8
- choice: 9
- appear: 7
- restriction: 7
- modify_required_hearts: 9
- place_energy_under_member: 5
- set_blade_type: 2
- modify_cost: 5
- activation_cost: 1
- activation_restriction: 1
- choose_required_hearts: 1
- conditional_alternative: 6
- discard_until_count: 1
- draw_until_count: 1
- gain_ability: 12
- invalidate_ability: 1
- modify_limit: 1
- modify_required_hearts_global: 3
- modify_yell_count: 1
- pay_energy: 1
- play_baton_touch: 1
- re_yell: 2
- set_blade_count: 1
- set_card_identity: 1
- set_cost: 1
- set_heart_type: 1
- set_required_hearts: 1
- set_score: 1

### Unused Actions (0 occurrences in abilities.json)

- conditional_on_optional
- conditional_on_result
- modify_required_hearts_success
- reveal_per_group
- set_card_identity_all_regions
- set_cost_to_use
- specify_heart_color
- shuffle
- all_blade_timing

## Engine Implementation Verification

### High-Usage Actions (Verified)

1. **move_cards** (222 occurrences)
   - ✅ Handles: source, destination, count, target, card_type, group, cost_limit, max, optional, placement_order, position, effect_constraint
   - ✅ Infers source/destination from text if not specified
   - ✅ Validates zone counts before execution

2. **gain_resource** (145 occurrences)
   - ✅ Handles: resource, count, target, duration, card_type, group, per_unit, per_unit_count, per_unit_type
   - ✅ Creates temporary effects for duration-based modifiers
   - ✅ Calculates per-unit scaling correctly

3. **draw_card** (90 occurrences)
   - ✅ Handles: count, target, source, destination, card_type, per_unit, per_unit_count, per_unit_type
   - ✅ Supports "both" target for both players
   - ✅ Calculates per-unit scaling

4. **sequential** (87 occurrences)
   - ✅ Handles: actions list, conditional flag, condition evaluation
   - ✅ Executes actions in order
   - ✅ Skips if condition not met when conditional=true

5. **change_state** (69 occurrences)
   - ✅ Handles: state_change, count, target, card_type, group, cost_limit, optional, source, destination
   - ✅ Properly handles optional costs with user choice
   - ✅ Distinguishes between activation and auto abilities for mandatory vs optional costs

6. **look_and_select** (58 occurrences)
   - ✅ Handles: look_action, select_action, placement_order, any_number, optional
   - ✅ Stores looked-at cards for selection
   - ✅ Prompts user for selection when placement_order or any_number is specified
   - ✅ Handles selection_remaining (moves unselected cards to discard)

7. **modify_score** (53 occurrences)
   - ✅ Handles: operation, value, target, duration, card_type, group, effect_constraint
   - ✅ Creates temporary effects for duration-based modifiers
   - ✅ Handles min/max value constraints

### Remaining Actions (Verified)

All remaining actions have corresponding handler functions in ability_resolver.rs:
- appear ✅
- position_change ✅
- select ✅
- gain_ability ✅
- reveal ✅
- play_baton_touch ✅
- restriction ✅
- set_blade_type ✅
- set_blade_count ✅
- set_card_identity ✅
- discard_until_count ✅
- place_energy_under_member ✅
- modify_cost ✅
- modify_limit ✅
- modify_required_hearts ✅
- modify_required_hearts_global ✅
- modify_yell_count ✅
- set_cost ✅
- set_required_hearts ✅
- set_score ✅
- activation_cost ✅
- activation_restriction ✅
- choose_required_hearts ✅
- invalidate_ability ✅
- pay_energy ✅

## Critical Field Handling

### placement_order
- ✅ Handled in execute_look_and_select (line 1649)
- ✅ Prompts user for card order when specified

### any_number
- ✅ Handled in execute_look_and_select (line 1652)
- ✅ Allows selecting 0 to all available cards

### selection_remaining
- ✅ Handled in execute_selected_looked_at_cards (line 403)
- ✅ Moves unselected cards to discard

### duration
- ✅ Handled in gain_resource, modify_score, change_state
- ✅ Creates temporary effects with proper duration tracking
- ✅ Duration expiration implemented in expire_live_end_effects (ability_resolver.rs lines 157-164)

## Verification Results (Updated 2026-04-27)

All previously identified issues have been verified and are now implemented:

1. **Duration Expiration** - ✅ IMPLEMENTED
   - Expiration logic exists in `expire_live_end_effects` (ability_resolver.rs lines 157-164)
   - Effects with duration="live_end" expire correctly

2. **Blade Heart Removal in re_yell** - ✅ IMPLEMENTED
   - lose_blade_hearts field recognized (card.rs line 597)
   - Blade heart removal logic implemented (ability_resolver.rs lines 5134-5148)

3. **Target Selection for Multiple Valid Targets** - ✅ IMPLEMENTED
   - Engine prompts user when multiple valid targets exist (ability_resolver.rs lines 3887, 3916)

4. **"This Member" Targeting** - ✅ IMPLEMENTED
   - activating_card tracking in game_state (line 142)
   - Self-cost handling for activating card (ability_resolver.rs lines 2351-2427)

5. **action_by (Opponent Actions)** - ✅ IMPLEMENTED
   - Field exists in card.rs (line 592)
   - Handled in ability_resolver.rs (lines 1637-1638)

6. **deck position placement** - ✅ IMPLEMENTED
   - Position handling in ability_resolver.rs (lines 2250-2251)

7. **effect_constraint** - ✅ IMPLEMENTED
   - Field exists in card.rs (line 506)
   - Constraint enforcement in ability_resolver.rs (lines 2242-2243, 4012-4013)

8. **placement_order** - ✅ IMPLEMENTED
   - Field exists in card.rs (line 552)
   - Handled in execute_move_cards and execute_look_and_select (ability_resolver.rs lines 1901-1931, 2240, 2260, 2861-2864)

9. **distinct** - ✅ IMPLEMENTED
   - Field exists in card.rs (lines 557, 673)
   - Distinct name checking in ability_resolver.rs (lines 699, 736-857, 4463-4488)

10. **cost_limit** - ✅ IMPLEMENTED
    - Field exists in card.rs (lines 479, 553, 677)
    - Extensive cost limit filtering in ability_resolver.rs (lines 1970, 2020, 2259, 2336, 2542, etc.)

11. **swap_action / has_member_swapping** - ⚠️ FIELD EXISTS IN RUST BUT NOT SET BY PARSER
    - Field exists in card.rs (line 566) and ability_resolver.rs (line 6034)
    - Parser does not set this field (not found in abilities.json)
    - PARSER_STATUS.md mentions extraction but not currently implemented

12. **group_matching** - ⚠️ FIELD EXISTS IN RUST BUT NOT USED - GROUP FILTERING WORKS VIA DIFFERENT FIELDS
    - Field exists in card.rs (lines 544, 717) and ability_resolver.rs (line 6013)
    - Parser does NOT use this field
    - **Group filtering IS working** via `group` and `group_names` fields instead
    - abilities.json contains 100+ entries with `"group": {"name": "..."}` and `"group_names": [...]`
    - Engine uses `effect.group.as_ref().and_then(|g| Some(&g.name))` and `condition.group_names.as_ref()` for filtering
    - The `group_matching` field appears to be a legacy/unused field that was never integrated

13. **execute_activation_restriction** - ✅ IMPLEMENTED
    - Handler exists at ability_resolver.rs lines 1759, 5156
    - Rarely used (1 card in abilities.json)

14. **execute_activation_cost** - ✅ IMPLEMENTED
    - Handler exists at ability_resolver.rs lines 1749, 2183
    - Rarely used (1 card in abilities.json)

## Parser to Engine Mapping

All 48 action types produced by the parser have corresponding handler functions in the engine. The mapping is complete.

## Conclusion

The parser and engine are well-aligned. All fields extracted by the parser are now recognized by the engine. All previously identified implementation gaps have been resolved:
1. ✅ Duration expiration implementation
2. ✅ Blade heart removal logic for re_yell
3. ✅ Target selection UI for multiple valid targets
4. ✅ Specific member targeting in costs
5. ✅ Opponent action handling
6. ✅ Effect constraint enforcement
7. ✅ Placement order handling
8. ✅ Distinct name enforcement
9. ✅ Cost limit enforcement
10. ✅ Deck position placement

Remaining fields (swap_action, group_matching) exist but are unused by current card data.

---

# Parser-Engine Alignment Summary

## Completed Work

### Parser Fixes
- Fixed `ability` array to `ability_gain` string in `extract_card_abilities.py`
- Parser now outputs `ability_gain` as a string matching engine expectations
- Removed duplicate state condition checks in parser.py (lines 574-585)

### Engine Additions
- Added `destination_choice` field to `AbilityEffect` struct
- Added `options` field to `Condition` struct for choice conditions
- Added `evaluate_choice_condition` handler in ability_resolver.rs
- Added `baton_touch_trigger` logic in `evaluate_location_condition`
- Implemented `empty_area` destination logic in game_state.rs
- Verified `group_matching` field exists but is not used by parser

### Engine Architectural Fixes
- Added `ExecutionContext` enum to track execution state (None, SingleEffect, SequentialEffects, LookAndSelect)
- Added `LookAndSelectStep` enum to track look_and_select steps (LookAt, Select, Finalize)
- Added `resume_execution` method to continue execution after user provides choice
- Modified `provide_choice_result` to save context and resume execution
- Modified `execute_look_and_select` to store execution context when setting pending choice
- Implemented stage area selection: checks available areas, presents choice to user, places in selected area
- Modified `provide_choice_result` to handle `Choice::SelectPosition` for stage area selection

### Previous Engine Fixes
- Deck filtering bug - continues drawing instead of stopping (ability_resolver.rs lines 1926-1980)
- Card type validation for live_card_zone (4 locations in ability_resolver.rs)
- Card type validation for stage destination (ability_resolver.rs lines 2237-2248)
- Zone count validation with warning system (ability_resolver.rs lines 1769-1785)

### Tests Created
Created `engine/tests/test_parser_engine_alignment.rs` with 12 passing tests:
- `test_empty_area_destination` - places card in first empty stage area
- `test_empty_area_destination_fills_in_order` - fills left → center → right
- `test_empty_area_destination_no_empty_areas` - stops when no empty areas
- `test_ability_gain_field_parsing` - verifies ability_gain is a string
- `test_destination_choice_field_parsing` - verifies field extraction
- `test_destination_choice_extraction` - verifies field values
- `test_baton_touch_trigger_condition` - verifies baton touch trigger logic
- `test_choice_condition_handler` - verifies handler exists
- `test_card_type_restrictions` - tests member_card vs live_card filtering
- `test_area_selection` - tests stage area position filtering
- `test_negation_condition` - tests negation field (not fully implemented)
- `test_count_comparison_condition` - tests count operators (not fully implemented)

## Deleted Files
All PARSER_ENGINE_*.md files, ENGINE_FIXES_REPORT.md, PARSER_CONDENSATION_OPPORTUNITIES.md, and ENGINE_ARCHITECTURAL_ISSUES.md have been consolidated into this summary.

---

# Parser-Engine Gap Analysis

This document identifies gaps between the Python parser (`cards/ability_extraction/parser.py`) and the Rust engine (`engine/src/ability_resolver.rs`).

## Summary

The parser extracts many action types and fields, but the engine does not fully utilize all of them. Some parser actions have no engine handler, some fields are extracted but not implemented, and some engine handlers are never triggered by parser output.

## Critical Gaps

### 1. Parser Actions with NO Engine Handler

#### `choose_heart`
- **Parser**: Line 1431 in `parser.py` extracts `action['action'] = 'choose_heart'` when text contains "選ぶ" and heart icons
- **Engine**: No `execute_choose_heart` function exists
- **Impact**: Heart selection abilities may not work correctly
- **Example**: Cards with text like "{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ"
- **Status**: These abilities are currently parsed as `sequential` with `select` + `gain_resource` instead (see abilities.json lines 1889-1904)

#### `action_by` (Opponent Actions)
- **Parser**: Line 1918 in `parser.py` sets `effect['action_by'] = 'opponent'` for opponent actions
- **Parser**: Line 1920 sets `effect['opponent_action']` with parsed opponent action
- **Engine**: No handling of `action_by` field in `execute_effect`
- **Impact**: Opponent-triggered abilities may not execute correctly
- **Example**: abilities.json line 9134 has `"action_by": "opponent"` with opponent_action
- **Status**: Engine ignores opponent action metadata

### 2. Parser Fields Extracted but NOT Implemented in Engine

#### `position.position` (Deck Position)
- **Parser**: Lines 1951-1956 in `parser.py` extract deck position (e.g., "4th from top")
- **Engine**: Line 1954 in `ability_resolver.rs` logs it but comments "full implementation would place card at specific position"
- **Impact**: Cards that need to be placed at specific deck positions won't work correctly
- **Example**: abilities.json line 16744 has `"position": {"position": "4"}`
- **Status**: Partially extracted, not implemented

#### `effect_constraint`
- **Parser**: Lines 1067-1078 in `parser.py` extract effect constraints (e.g., minimum_value)
- **Engine**: Line 1947 in `ability_resolver.rs` logs it but comments "full implementation would enforce the constraint"
- **Impact**: Effect value constraints are not enforced
- **Example**: abilities.json line 5171 has `"effect_constraint": "minimum_value:0"`
- **Status**: Partially extracted, not implemented

#### `placement_order` (Outside look_and_select)
- **Parser**: Lines 1274-1275, 1358-1359 in `parser.py` extract "any_order" placement
- **Engine**: Only used in `execute_look_and_select` (lines 1602-1634), not in `execute_move_cards`
- **Impact**: Cards that can be placed in any order may not respect player choice
- **Example**: abilities.json has 15 occurrences of `"placement_order": "any_order"`
- **Status**: Extracted but not fully utilized

#### `swap_action` / `has_member_swapping`
- **Parser**: Not set by parser
- **Engine**: Line 5076 has `swap_action: None`, line 5077 has `has_member_swapping: None`
- **Impact**: Member swapping abilities may not work
- **Status**: Not implemented on either side

#### `group_matching`
- **Parser**: Not set by parser
- **Engine**: Line 5055 has `group_matching: None`
- **Impact**: Group matching conditions may not work
- **Status**: Not implemented on either side

### 3. Engine Handlers Never Triggered by Parser

#### `execute_activation_restriction`
- **Engine**: Handler exists at line 4311
- **Parser**: Line 1158 sets `action['action'] = 'activation_restriction'` but only in specific contexts
- **Usage**: Only 1 card in abilities.json uses this (line 8108)
- **Status**: Rarely used, may be edge case

#### `execute_activation_cost`
- **Engine**: Handler exists at line 1885
- **Parser**: Line 1161 sets `action['action'] = 'activation_cost'` but only in specific contexts
- **Usage**: Only 1 card in abilities.json uses this (line 1422)
- **Status**: Rarely used, may be edge case

### 4. Parser Actions with Engine Handlers but No Cards Use

#### `specify_heart_color`
- **Parser**: Line 1557 in `parser.py` extracts this action
- **Engine**: Handler exists at line 4467
- **Usage**: No cards in abilities.json use this action
- **Status**: Implemented but unused

#### `all_blade_timing`
- **Parser**: Line 1585 in `parser.py` extracts this action
- **Engine**: Handler exists at line 4567
- **Usage**: No cards in abilities.json use this action
- **Status**: Implemented but unused

### 5. Partially Implemented Features

#### `cost_limit`
- **Parser**: Extracted in `parse_action` (line 1067)
- **Engine**: Used extensively in filtering (lines 1722, 2037, etc.) but not in cost payment
- **Impact**: Cost limits may not be enforced during cost payment
- **Status**: Extracted and used for filtering, but not for cost enforcement

#### `distinct` (in select action)
- **Parser**: Line 1294 sets `action['distinct'] = 'card_name'` for select actions
- **Engine**: Used in condition evaluation (line 498) but not in select execution
- **Impact**: Distinct card selection may not be enforced
- **Status**: Extracted but not fully utilized

## Implementation Status

### High Priority - COMPLETED
1. ~~**Implement `choose_heart` handler** or ensure parser correctly maps to sequential (select + gain_resource)~~ - Parser correctly maps to sequential, no action needed
2. ~~**Implement opponent action handling** for `action_by` field~~ - **COMPLETED**: Added opponent action execution in `execute_effect` (lines 1368-1376)
3. ~~**Implement deck position placement** for `position.position` field~~ - **COMPLETED**: Already implemented in `execute_move_cards` (lines 2548-2567)
4. ~~**Implement effect constraint enforcement** for `effect_constraint` field~~ - **COMPLETED**: Added constraint enforcement in `execute_modify_score` (lines 3446-3514)

### Medium Priority - COMPLETED
5. ~~**Extend `placement_order` usage** to `execute_move_cards` beyond just `look_and_select`~~ - **COMPLETED**: Added placement_order handling in deck destination (lines 2528-2574)
6. ~~**Implement `distinct` enforcement** in select actions~~ - **COMPLETED**: Added distinct field handling in `execute_select` (lines 3925-3951)
7. ~~**Implement cost limit enforcement** during cost payment~~ - **COMPLETED**: Added cost limit validation in `pay_cost` (lines 4943-5003)

### Low Priority - NOT IMPLEMENTED
8. **Implement `swap_action` / `has_member_swapping`** if needed - No cards currently use this
9. **Implement `group_matching`** if needed - No cards currently use this
10. **Review rare handlers** (`activation_restriction`, `activation_cost`) for edge cases - Rarely used, may be edge cases

## Testing Strategy

For each gap identified:
1. Find cards in abilities.json that use the feature
2. Create gameplay tests in `engine/tests/qa_individual/direct_engine_faults.rs`
3. Verify the ability works as expected
4. If not, implement the missing engine handler or fix parser output

---

# Parser and Engine Issues - Comprehensive Analysis

## Overview
This document contains all identified parser and engine issues from the systematic re-analysis of abilities.json starting from line 8911.

---

## PARSER ISSUES

### Choice Mechanisms Missing
- Line 544-566: Missing choice mechanism for heart type selection
- Line 753: Missing choice mechanism for heart color selection
- Line 1112: Missing choice mechanism for heart color selection
- Line 1869: Missing choice mechanism for heart type selection
- Line 8343: Missing heart_type choice mechanism
- Line 9068: Missing 'select and reveal from hand' mechanism
- Line 9538-9566: Missing 'self or opponent' choice mechanism (select action incomplete)

### Compound Conditions Missing
- Line 1184: Missing compound condition for cheer+hand size
- Line 2759: Missing compound condition (baton touch + energy)
- Line 3159: Missing compound condition (center + cost equality)
- Line 3649: Missing compound condition (moves OR energy placed)
- Line 7015: Missing compound condition (self 0 AND opponent 1+)
- Line 9385-9426: Compound condition correctly parsed (verified)

### Conditional Logic Missing
- Line 1650: Missing cost comparison in baton touch condition
- Line 2278: Missing multi-branch conditional based on cost totals
- Line 3006: Missing conditional branching based on cost result
- Line 3057: Missing dynamic cost calculation based on groups
- Line 3563: Missing conditional effects based on cost result
- Line 3757: Missing cost result reference for live card check
- Line 4221: Missing conditional effect based on cost result (live card)
- Line 5162: Missing +1 case (no surplus hearts), multi-branch conditional
- Line 8238: Missing conditional effect based on selected card properties

### Exclusion Filters Missing
- Line 6104: Missing 'other' (ほかの) exclusion filter
- Line 6928: Missing 'other' (ほかの) exclusion filter
- Line 6981: Missing 'excluding Onitsuka Fumari' filter
- Line 8530: Missing 'excluding ミア・テイラー' filter
- Line 5513: Missing non-Series Bouquet exclusion filter
- Line 3195: Missing appeared-this-turn filter, non-Aqours targeting

### Dynamic Counts/Calculations Missing
- Line 1991: per_unit should reference cost result
- Line 2059: Missing cost calculation (cost+2)
- Line 2426: Missing dynamic count based on opponent wait members
- Line 2945: Missing dynamic count (live score + 2)
- Line 3444: Missing dynamic count (energy under member + 1)
- Line 3695: per_unit should reference cost result
- Line 4647: Count should reference cost result (discarded cards)
- Line 4964: Missing dynamic cost (energy = selected card score)
- Line 5406: Missing repeat mechanic (up to 4 more times), conditional effect
- Line 6513: Draw count should reference discarded count
- Line 8887: Missing dynamic cost calculation (3 - success_live_card_zone count)
- Line 8913: per_unit should reference cost result (members put to wait)

### Distinct Name/Group Checks Missing
- Line 2656: Missing distinct card name condition
- Line 2974: Missing distinct names check
- Line 5196: Missing distinct card names, distinct group names checks
- Line 3792: Missing contains all card names matching logic
- Line 9007: Missing 'distinct names' filter

### Multi-Trigger Separation Missing
- Line 5842: Missing multi-trigger separation (登場 + ライブ開始時)
- Line 7673: Missing multi-trigger separation (ライブ開始時 + ライブ成功時)
- Line 9109-9143: Multi-trigger (登場 + ライブ開始時) correctly parsed (verified)

### Sequential Structure Issues
- Line 1719: Missing discard after draw
- Line 2399: Incorrectly combined draw and change_state
- Line 3994: Choice option incorrectly parsed as discard instead of sequential
- Line 8068: Missing sequential with 3 separate moves
- Line 8144: Missing reveal action, sequential structure incomplete
- Line 8286: Missing reveal action, sequential structure incomplete
- Line 8586: Missing reveal action, sequential structure incomplete
- Line 9198: Missing reveal action, sequential structure incomplete
- Line 9362: Look_action incorrectly has card_type filter
- Line 9369: Missing reveal action, sequential structure incomplete

### Reveal Until Condition Loop Missing
- Line 6241: Missing 'reveal until live card' loop mechanic
- Line 8655: Missing 'reveal until condition' loop mechanic

### Heart/Blade Context Missing
- Line 3079: Missing cheer-revealed cards context
- Line 3236: Missing heart variety check, cheer-revealed context
- Line 3281: Missing no-blade-hearts check, baton touch tracking
- Line 7084: Missing cheer-revealed cards context
- Line 7110: Missing cheer-revealed context, full re-yell mechanic
- Line 7756: Missing cheer-revealed cards context
- Line 7673: Missing cheer-revealed context
- Line 8168: Missing cheer-revealed cards count context

### Baton Touch Context Missing
- Line 2711: Missing baton touch from 2 members condition
- Line 7553: Missing 'lower cost' and group filter in baton touch condition
- Line 7585: Missing 'lower cost' and group filter in baton touch condition
- Line 9249: Appearance_condition missing 'from Printemps' and 'baton touch' context

### Cost/Score Filters Missing
- Line 3590: Missing cost 10 filter
- Line 4793: Missing 2+ heart04 filter
- Line 5998: Missing 'all have heart04' condition
- Line 6052: Missing 'all have heart01' condition
- Line 6140: Missing 'total cost ≤4' constraint
- Line 6165: Missing 'lower cost than discarded card' comparison
- Line 6426: Missing 2E payment in cost
- Line 6469: Missing 2E payment in cost
- Line 6642: Missing 2E payment in cost
- Line 6679: Missing 'unless 2E paid' conditional cost
- Line 7224: Missing 'center has highest cost' comparison
- Line 8933: Missing 'original blade count ≤1' filter
- Line 9007: Missing 'cost ≤4' filter
- Line 9120: Missing 'BiBi group' filter in cost
- Line 9362: Look_action incorrectly has card_type filter

### Group Filters Missing
- Line 5920: Missing same group name filter
- Line 5574: Missing same group name filter
- Line 5310: Missing 1 per group name distinct selection logic
- Line 3908: Missing same group as discarded card targeting
- Line 8755: 'Printemps group' filter not captured
- Line 8887: Missing 'other lilywhite member' filter

### Position/Area Context Missing
- Line 2875: Custom condition placeholder for position change
- Line 2896: Missing base blades distinction
- Line 3934: Missing area with group condition in position_change
- Line 6904: Missing position change swap mechanic details
- Line 7337: Missing position change swap mechanic details
- Line 8635: Missing 'center' position requirement in cost
- Line 8696: Missing 'original blade count ≤3' filter
- Line 8726: 'opponent's stage' and 'wait state' filters not captured
- Line 8857: 'opponent's stage' and 'wait state' filters not captured
- Line 9277: empty_area destination not implemented in engine

### Action Destination Issues
- Line 156: look_and_select destination should be hand not discard
- Line 1159: Destination should be hand, missing heart filter
- Line 4288: select_action destination should be hand not discard
- Line 4454: select_action destination should be hand not discard
- Line 4754: select_action destination should be hand not discard
- Line 5132: select_action destination should be hand not discard
- Line 5365: select_action destination should be hand not discard
- Line 5700: select_action destination should be hand not discard
- Line 5746: select_action destination should be hand not discard
- Line 6336: select_action destination should be hand not discard
- Line 6444: select_action destination should be hand not discard
- Line 6578: select_action destination should be hand not discard
- Line 6622: select_action destination should be hand not discard
- Line 7174: select_action destination should be hand not discard
- Line 7260: select_action destination should be hand not discard
- Line 7304: select_action destination should be hand not discard

### Other Parser Issues
- Line 1334: Missing invalidate_ability action
- Line 1692: Missing same-name matching logic
- Line 1893: Select action missing options array
- Line 2378: Missing condition for no μ's member with 5+ blades
- Line 2481: Missing both players energy total in condition
- Line 2583: Missing select live card mechanism
- Line 3107: Missing no-ability member check
- Line 3334: Missing same count comparison between players
- Line 3368: surplus_heart condition marked as custom
- Line 3407: place_energy_under_member cost marked as custom
- Line 3818: Missing different costs check
- Line 4121: Missing compound condition, reveal all incomplete, exclude_self in deck look
- Line 4163: Missing cheer-revealed source, OR condition (cost/score)
- Line 4494: Missing active state destination
- Line 4616: Missing required hearts filter (3+ heart06)
- Line 4682: exclude_self not properly integrated with targeting
- Line 5003: Missing base blades distinction, parenthetical note
- Line 5075: place_energy_under_member cost marked as custom
- Line 5271: Position change condition marked as custom
- Line 5433: Missing center targeting, both players targeting
- Line 5543: Missing select member, dynamic cost setting, conditional effect
- Line 6363: Missing first unconditional draw
- Line 6534: Missing 'all are member cards' condition
- Line 6754: Missing 'only' (のみ) condition, rotation pattern, both players targeting
- Line 7403: comparison_target field not implemented in engine
- Line 7513: destination_choice field not implemented in engine
- Line 7616: comparison_target field not implemented in engine
- Line 7643: 'opponent didn't discard' condition marked as custom
- Line 7699: comparison_target field not implemented in engine
- Line 7733: 'opponent didn't discard' condition marked as custom
- Line 7785: Missing 'self or opponent' choice mechanism
- Line 7825: Missing 'both players' targeting
- Line 7843: 'opponent's stage' and 'wait state' filters not captured
- Line 8029: 'success live card zone' source not captured
- Line 8091: comparison_target field not implemented in engine
- Line 8108: activation_restriction action not implemented in engine
- Line 8200: Missing 'self or opponent' choice mechanism
- Line 8201: deck_bottom destination not implemented in engine
- Line 8385: select action not implemented in engine
- Line 8470: Cost should be sequential_cost (pay_energy + move_cards)
- Line 8470: Missing 'same area' destination
- Line 8487: appear action not implemented in engine
- Line 8491: Energy placement should be place_energy_under_member
- Line 8518: Missing 'self or opponent' choice mechanism
- Line 8518: Missing 'up to 2' (max) field
- Line 8522: deck_bottom destination not implemented in engine
- Line 8530: Missing multi-condition (same heart OR same cost OR same original blades)
- Line 8530: Missing 'select member' mechanism
- Line 8655: Missing 'choose card type' choice mechanism
- Line 8659: select action not implemented in engine
- Line 8698: 'only BiBi' condition needs verification (all members vs at least one)
- Line 8782: primary_effect missing actual ability structure
- Line 8779: activation_position not implemented in engine
- Line 9273: Parenthetical restriction on area appearance not implemented
- Line 9470: action_by opponent not implemented in engine
- Line 9508: Condition comparing energy counts missing comparison_type
- Line 9538-9566: Select action incomplete - missing choice mechanism

---

## ENGINE ISSUES

### Action Implementations Missing
- `restriction` action (line 8955)
- `activation_restriction` action (line 8108)
- `select` action (line 8385, 8659, 9538-9566)
- `appear` action (line 8487)
- `reveal` action (multiple lines - part of look_and_select structure)
- `place_energy_under_member` action (line 8491)
- `modify_cost` action (line 9104, 9427-9457)
- `gain_ability` action (line 9068, 9385-9426)
- `set_card_identity` action (not yet seen but referenced)
- `discard_until_count` action (not yet seen but referenced)

### Field Implementations Missing
- `comparison_target` field in condition evaluation (lines 7403, 7513, 7616, 7699, 8091, 9305, 9508, 9567)
- `destination_choice` field (line 7513)
- `same_area` destination (line 6993, 8470)
- `deck_bottom` destination (line 8201, 8522)
- `empty_area` destination (line 9277)
- `activation_position` enforcement (line 8779, 9133)
- `action_by: "opponent"` field (line 9134, 9470)
- `max` field usage for 'up to X' logic (line 8518)
- `target_member: this_member` in place_energy_under_member (line 8491)
- `parenthetical` area restriction enforcement (line 9273)

### Trigger Type Support Missing
- `each_time` trigger type (line 6854)
- `auto` trigger type (line 9152)
- Position-based triggers (left side, center, right side) (line 9133)

### Condition Logic Missing
- State transition tracking for conditions (line 9158)
- Temporal_condition appearance count tracking (line 8887)
- Either target (self OR opponent) in conditions (line 7825)
- Appearance_condition with baton touch tracking (line 9249)
- Compound condition with OR logic (line 4163)
- Multi-branch conditional based on cost results

### Mechanic Implementations Missing
- Reveal until condition loop mechanic (line 6241, 8655)
- Repeat mechanic for multi-step abilities (line 5406)
- Select and reveal from hand mechanism (line 9068)
- Cheer-revealed cards count context (line 8168)
- Full re-yell mechanic (line 7110)

### Verification Needed
- Appearance_condition position checking
- Exclude_self in condition evaluation

---

## PRIORITIZED FIX ORDER

### Phase 1: High-Impact Parser Fixes
1. Fix all look_and_select reveal action issues (lines 156, 8144, 8286, 8586, 9198, 9369)
2. Fix select_action destination issues (multiple lines)
3. Fix choice mechanism implementations (heart type/color, self or opponent)
4. Fix compound condition parsing

### Phase 2: Engine Core Implementations
1. Implement `comparison_target` field usage
2. Implement `gain_ability` action
3. Implement `modify_cost` action
4. Implement `restriction` action
5. Implement `select` action
6. Implement `appear` action
7. Implement `place_energy_under_member` action

### Phase 3: Destination Handling
1. Implement `deck_bottom` destination
2. Implement `same_area` destination
3. Implement `empty_area` destination

### Phase 4: Advanced Mechanics
1. Implement reveal until condition loop
2. Implement repeat mechanic
3. Implement state transition tracking
4. Implement baton touch tracking

### Phase 5: Trigger Types
1. Implement `auto` trigger type
2. Implement `each_time` trigger type
3. Implement position-based triggers

### Phase 6: Remaining Parser Issues
1. Fix dynamic cost calculations
2. Fix distinct name/group checks
3. Fix multi-trigger separations
4. Fix heart/blade context issues
