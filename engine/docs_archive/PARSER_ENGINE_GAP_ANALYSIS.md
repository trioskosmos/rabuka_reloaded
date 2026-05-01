# Parser-Engine Gap Analysis

## Overview
This document identifies gaps between the Python ability parser and the Rust engine's ability resolver.

## Parser Actions vs Engine Actions

### Parser Actions (from parser.py)
- move_cards
- draw_card
- draw_until_count
- gain_resource
- change_state
- modify_score
- modify_required_hearts
- modify_required_hearts_success
- set_cost
- set_cost_to_use
- set_blade_type
- set_heart_type
- set_blade_count
- set_required_hearts
- set_score
- activate_ability
- invalidate_ability
- gain_ability
- play_baton_touch
- reveal
- reveal_per_group
- select
- look_at
- modify_required_hearts_global
- modify_yell_count
- place_energy_under_member
- activation_cost
- position_change
- appear
- choice
- pay_energy
- set_card_identity
- set_card_identity_all_regions
- discard_until_count
- restriction
- re_yell
- modify_cost
- activation_restriction
- choose_required_hearts
- modify_limit
- specify_heart_color
- all_blade_timing
- shuffle
- conditional_on_result
- conditional_on_optional
- look_and_select
- sequential
- conditional_alternative

### Engine Actions (from ability_resolver.rs)
- draw / draw_card
- draw_until_count
- move_cards
- gain_resource
- change_state
- modify_score
- modify_required_hearts
- set_cost
- set_blade_type
- set_heart_type
- activate_ability
- invalidate_ability
- gain_ability
- play_baton_touch
- reveal
- select
- look_at
- modify_required_hearts_global
- modify_yell_count
- place_energy_under_member
- activation_cost
- position_change
- appear
- choice
- pay_energy
- set_card_identity
- discard_until_count
- restriction
- re_yell
- modify_cost
- activation_restriction
- choose_required_hearts
- modify_limit
- set_blade_count
- set_required_hearts
- set_score
- specify_heart_color (mapped to set_heart_type)
- modify_required_hearts_success (mapped to modify_required_hearts)
- set_cost_to_use (mapped to set_cost)
- all_blade_timing (mapped to set_blade_type)
- set_card_identity_all_regions (mapped to set_card_identity)
- custom
- sequential (handled by execute_sequential_effect)
- conditional_alternative (handled by execute_conditional_alternative)

## Identified Gaps

### 1. Missing Engine Actions
The following parser actions have no dedicated engine handler or are mapped incorrectly:

- **reveal_per_group**: Parser generates this for "各グループ名につき1枚ずつ公開し" pattern, but engine has no handler
- **shuffle**: Parser generates this for "シャッフルする" pattern, but engine has no handler
- **conditional_on_result**: Parser generates this for "これにより～した場合" pattern, but engine has no handler
- **conditional_on_optional**: Parser generates this for "そうした場合" pattern, but engine has no handler
- **look_and_select**: Parser generates this for "その中から" pattern, but engine has incomplete handler

### 2. Incorrect Action Mappings
The following parser actions are mapped to different engine actions:

- **specify_heart_color** → mapped to `set_heart_type` (should have dedicated handler)
- **modify_required_hearts_success** → mapped to `modify_required_hearts` (should have dedicated handler for success-specific modification)
- **set_cost_to_use** → mapped to `set_cost` (should have dedicated handler for use cost vs current cost)
- **all_blade_timing** → mapped to `set_blade_type` (should have dedicated handler for timing-specific effects)
- **set_card_identity_all_regions** → mapped to `set_card_identity` (should have dedicated handler for all regions)

### 3. Incomplete Condition Handling
Parser generates complex condition structures that engine may not fully evaluate:

- **or_condition**: Parser generates OR conditions (か、), engine may not handle
- **compound conditions**: Parser generates AND conditions (かつ/あり、), engine may not handle nested conditions
- **temporal_condition**: Parser generates temporal conditions (このターン/このライブ), engine may not track temporal state properly
- **movement_condition**: Parser generates movement conditions (移動した/移動していない), engine may not track movement state properly
- **distinct condition**: Parser generates distinct conditions (名前が異なる), engine may not enforce distinctness
- **baton_touch_trigger**: Parser generates baton touch conditions, engine may not handle properly

### 4. Missing Field Usage
Parser generates fields that engine may not use:

- **per_unit_count**: Parser generates count for per-unit scaling, engine may not use
- **per_unit_type**: Parser generates type for per-unit scaling, engine may not use
- **placement_order**: Parser generates "好きな順番で" (any order), engine may not respect
- **effect_constraint**: Parser generates minimum/maximum value constraints, engine may not enforce
- **deck_position**: Parser generates specific deck positions (一番上から4枚目), engine may not handle
- **group_matching**: Parser generates group matching conditions, engine may not handle
- **repeat_limit**: Parser generates repeat limits, engine may not enforce
- **repeat_optional**: Parser generates optional repeat, engine may not handle
- **any_number**: Parser generates "好きな枚数" (any number), engine may not handle
- **choice_modifier**: Parser generates choice modifiers, engine may not use
- **choice_condition**: Parser generates conditions for choices, engine may not evaluate
- **multiple_targets**: Parser generates multiple target patterns, engine may not handle
- **action_by**: Parser generates "opponent action" patterns, engine may not handle
- **opponent_action**: Parser generates opponent action structures, engine may not execute

### 5. Sequential Action Handling
Parser generates complex sequential structures:

- **sequential with conditions**: Parser generates "その後、[condition]かぎり、[action]", engine may not handle duration conditions on sequential actions
- **sequential with opponent actions**: Parser generates "～、相手は～", engine may not handle opponent actions in sequence
- **conditional sequential**: Parser generates "そうした場合" patterns, engine may not handle conditional sequential properly
- **further conditional**: Parser generates "さらに" patterns for additional conditional effects, engine may not handle

### 6. Cost Handling Gaps
Parser generates complex cost structures:

- **sequential_cost**: Parser generates sequential costs (～し、～), engine may not handle
- **choice_condition**: Parser generates choice costs (～か、～), engine may not handle properly
- **self_cost**: Parser generates self-cost markers, engine may not use properly
- **reveal_condition**: Parser generates reveal costs, engine may not handle as cost

### 7. Duration Handling
Parser generates duration information:

- **duration prefixes**: Parser strips duration prefixes (ライブ終了時まで、このターンの間), engine may not track duration expiration
- **as_long_as**: Parser generates "かぎり" duration, engine may not handle properly
- **unless**: Parser generates "かぎり" as "unless", engine may not handle negation

## Priority Fixes

### High Priority (Critical Gameplay Impact)
1. Implement missing engine handlers:
   - shuffle
   - reveal_per_group
   - conditional_on_result
   - conditional_on_optional
   - look_and_select (complete implementation)

2. Fix incorrect action mappings:
   - specify_heart_color (dedicated handler)
   - modify_required_hearts_success (dedicated handler)
   - set_cost_to_use (dedicated handler)
   - all_blade_timing (dedicated handler)
   - set_card_identity_all_regions (dedicated handler)

3. Complete condition handling:
   - or_condition
   - compound conditions
   - temporal_condition (proper tracking)
   - movement_condition (proper tracking)
   - distinct condition (enforcement)

### Medium Priority (Gameplay Impact)
4. Implement missing field usage:
   - per_unit_count and per_unit_type
   - placement_order
   - effect_constraint
   - deck_position
   - group_matching

5. Complete sequential action handling:
   - sequential with conditions
   - sequential with opponent actions
   - further conditional

6. Complete cost handling:
   - sequential_cost
   - choice_condition

### Low Priority (Edge Cases)
7. Duration tracking and expiration
8. Repeat limits and optional repeat
9. Choice modifiers and conditions
10. Opponent action handling

## Test Strategy

For each gap, create end-to-end gameplay tests that:
1. Use actual cards from abilities.json with the relevant ability
2. Set up realistic game state
3. Execute the ability through normal gameplay flow
4. Verify all expected state changes occur
5. Verify all conditions are properly evaluated
6. Verify all duration effects expire correctly
7. Verify all optional/choice mechanics work correctly
