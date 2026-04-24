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
