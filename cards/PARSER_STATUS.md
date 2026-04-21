# Parser and Abilities.json Status Report

## Overview

The parser (`cards/ability_extraction/parser.py`, 2721 lines) and parsed output (`cards/abilities.json`, 17087 lines, 1324 abilities across 602 unique abilities) have been enhanced to extract more semantic information.

**Status: ✅ ALL CUSTOM TYPES ELIMINATED | ENHANCED SEMANTIC EXTRACTION**

---

## Current State

### Parser Quality
- **0 custom type instances** in abilities.json
- **30+ condition types** correctly classified
- **20+ action types** correctly classified
- Complex structures handled: sequential actions, conditional effects, compound conditions, OR conditions, choice effects

### Recent Enhancements (Latest)
1. **Parenthetical restrictions** - Now parsed as structured fields with types like `cheer_reveal_limit`, `cost_payment_restriction`, `member_leave_trigger`, `position_change_rules`
2. **Original vs current blade count** - Distinguished via `blade_count_type: "original"` field when "元々" is present
3. **Empty area destination** - Added pattern for "メンバーのいないエリア" and "エリアに登場させる"
4. **Per-unit context preservation** - Full context preserved in `per_unit` field instead of generic placeholders, with `per_unit_location` extraction
5. **Position change member swapping** - Extracted `has_member_swapping` and `swap_action` fields from position_change actions
6. **Previous effect references** - New `repeat_previous_effect` action type with `reference_type` and `condition` fields

### Earlier Fixes
1. **Fixed empty alternative_condition text** - Empty condition text in conditional_alternative structures now returns None instead of custom type
2. **Fixed syntax error** - Renamed `except` field to `has_except` (Python keyword conflict)
3. **Fixed variable scope** - Extracted `count` and `card_type` from condition dict before type determination

---

## What's Working Well

### Condition Types (30+)
- location_condition
- comparison_condition
- group_condition
- compound (かつ, あり、)
- temporal_condition
- state_condition
- baton_touch_condition
- yell_revealed_condition
- ability_negation_condition
- movement_condition
- state_transition_condition
- action_restriction_condition
- distinct_condition
- position_condition
- cost_limit_condition
- yell_action_condition
- heart_variety_condition
- appearance_condition
- group_location_count_condition
- resource_count_condition
- group_resource_count_condition
- energy_condition
- surplus_heart_condition
- move_action_condition
- score_threshold_condition
- movement_count_condition
- except_count_condition
- card_count_condition
- character_presence_condition
- name_match_condition
- both_condition
- or_condition
- any_of_condition
- negative_choice_condition
- active_energy_condition

### Action Types (20+)
- draw
- move_cards
- change_state
- gain_resource
- look_and_select
- sequential
- choice
- custom
- modify_score
- modify_required_hearts
- restriction
- invalidate_ability
- set_score
- position_change
- reveal
- modify_limit
- set_blade_type
- set_heart_type
- set_required_hearts
- modify_required_hearts_global
- shuffle
- swap
- pay_energy
- place_energy_under_member
- draw_until_count
- discard_until_count
- pay_energy_per_trigger
- appear
- modify_cost
- reveal_per_group
- modify_yell_count
- select
- activation_restriction
- activation_cost
- conditional_alternative

### Extraction Quality
- Group names from 『』 brackets
- Character names from 「」 quotes
- Costs, counts, operators
- Positions (center, left_side, right_side)
- Locations (hand, discard, stage, deck, energy_zone, etc.)
- Card types (member_card, live_card, energy_card)
- Targets (self, opponent, both, either)
- Duration prefixes
- Parenthetical notes
- Per-unit patterns
- Ability gain patterns

---

## Remaining Code Quality Issues (Low Priority)

### From parser_generalization_issues.md
- Hardcoded magic numbers (MAX_CHARACTER_NAME_LENGTH = 10, SPLIT_LIMIT = 1)
- Hardcoded pattern strings not extracted to constants (lines 102-110)
- Missing error handling throughout
- Hyperspecific character name filtering (assumes ability names contain {{)
- Hardcoded action type strings (should use enums)
- Hardcoded resource type checks

These are maintainability issues, not parsing issues. They don't affect the correctness of the output.

---

## Statistics

- **Total abilities**: 1324
- **Unique abilities**: 602
- **Cards with abilities**: 1057/1806 (58.5%)
- **Parser size**: 2649 lines
- **Output size**: 17087 lines
- **Custom type instances**: 0 ✅

---

## Documentation

Outdated documentation files have been deleted:
- `cards/parsing_issues_round2.md` (deleted - issues resolved)
- `cards/abilities_issues.md` (deleted - outdated)
- `cards/ability_extraction/abilities_issues.md` (deleted - outdated)

Current documentation:
- `cards/ability_extraction/parser_generalization_issues.md` (code quality issues - still relevant)
