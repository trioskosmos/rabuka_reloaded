# Remaining Ability JSON Structure Issues

## Overview
This document identifies remaining issues in abilities.json structure after quote character fixes have been implemented.

**Analysis Date:** 2026-05-06  
**Total Issues Found:** 8 categories of remaining problems

## High Priority Issues

### 1. **Null Count Fields with Clear Text Values**
**Severity:** HIGH  
**Examples:**

```json
{
  "text": "自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る",
  "count": null,
  "action": "gain_resource",
  "resource": "heart",
  "per_unit": true
}
```

**Problem:** 
- Text clearly states "1つ" (1) but count field is null
- Found in 8+ instances across abilities.json
- **Lines affected:** 616, 2204, 15035, 18687, 19113, 19151, 21211, 21472, 23842

**Root Cause:** Parser not extracting explicit count values from text when they're embedded in descriptive text

### 2. **Per-Unit Logic Without Proper Count Handling**
**Severity:** HIGH  
**Example:**

```json
{
  "text": "自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る",
  "count": null,
  "per_unit": true,
  "per_unit_count": 1,
  "per_unit_type": "枚"
}
```

**Problem:** 
- Per-unit logic correctly parsed (`per_unit: true`)
- Base count value not extracted from text
- Engine needs both base count and per-unit multiplier

## Medium Priority Issues

### 3. **Complex Activation Conditions**
**Severity:** MEDIUM  
**Example:**

```json
{
  "text": "ライブの合計スコアが相手より高い場合、このカードを手札に加えてもよい",
  "activation_condition": "この能力は、このカードが自分のエールによって公開されている場合のみ発動する",
  "activation_condition_parsed": {
    "text": "このカードが自分のエールによって公開されている場合",
    "target": "self",
    "count": 1,
    "operator": ">=",
    "type": "comparison_condition"
  }
}
```

**Problem:** 
- Text has two separate conditions (score comparison AND reveal state)
- Only one condition parsed into structured format
- Missing score comparison logic

### 4. **Parenthetical Text Handling**
**Severity:** MEDIUM  
**Example:**

```json
{
  "text": "自分の控え室から『μ's』のメンバーカードを1枚手札に加える。",
  "parenthetical": [
    "ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"
  ]
}
```

**Problem:**
- Parenthetical rules extracted correctly
- But not processed into engine-readable format
- Should be separate rule objects or flags

## Low Priority Issues

### 5. **Group Name Extraction Inconsistencies**
**Severity:** LOW  
**Pattern:** Some abilities have both `group` and `group_names` fields, others only one.

**Example:**
```json
"group": {
  "name": "μ's"
},
"group_names": [
  "μ's"
]
```

**Problem:** Redundant data structure, potential for inconsistency

### 6. **Missing Card Type Validation**
**Severity:** LOW  
**Pattern:** Generic `"card_type": "card"` usage without specific type constraints.

**Problem:** Engine may not enforce proper card type filtering in all cases

### 7. **Optional Cost Ambiguity**
**Severity:** LOW  
**Example:**

```json
{
  "text": "手札を1枚控え室に置いてもよい",
  "optional": true,
  "source": "hand",
  "destination": "discard",
  "count": 1
}
```

**Problem:** 
- "てもよい" (may) correctly parsed as optional
- But some costs might be mandatory with conditional effects

### 8. **State Change Action Complexity**
**Severity:** LOW  
**Example:**

```json
{
  "text": "このメンバーをウェイトにしてもよい",
  "state_change": "wait",
  "optional": true,
  "self_cost": true,
  "type": "change_state"
}
```

**Problem:** 
- Simple state change parsed correctly
- But complex state transitions (like "アクティブにする") may need more structure

## Engine Impact Analysis

### Critical for Game Logic
1. **Null count fields** - Abilities won't execute properly
2. **Missing conditions** - Wrong timing/triggering
3. **Per-unit math** - Incorrect resource calculations

### UI/Display Issues
1. **Parenthetical text** - Rules not shown to players
2. **Group name redundancy** - Inconsistent display

### Minor Gameplay Effects
1. **Card type validation** - May allow illegal moves
2. **Optional cost handling** - Player choice confusion

## Recommended Fixes

### Immediate (High Priority)
1. **Fix count extraction** from text patterns:
   - "1つ得る" → count: 1
   - "2つ得る" → count: 2
   - "1枚加える" → count: 1

2. **Complete condition parsing** for multi-condition abilities:
   - Parse score comparisons
   - Parse reveal states
   - Combine with AND/OR logic

### Medium Priority
1. **Structure parenthetical rules** as separate rule objects
2. **Standardize group fields** (use only one format)
3. **Validate card type constraints** in all move actions

### Low Priority
1. **Review optional cost logic** for edge cases
2. **Enhance state change parsing** for complex transitions
3. **Add validation rules** for consistency

## Files to Update

1. `cards/ability_extraction/parser.py` - Fix extraction logic
2. `engine/src/ability/effects.rs` - Handle null counts
3. `engine/src/ability/resolver.rs` - Process complex conditions
4. `engine/src/core/card.rs` - Update AbilityEffect struct

## Testing Requirements

1. **Unit tests** for count extraction from text
2. **Integration tests** for multi-condition abilities
3. **UI tests** for parenthetical rule display
4. **Gameplay tests** for per-unit calculations

## Affected Ability Count

- **Critical issues:** 15-20 abilities
- **Medium issues:** 30-40 abilities  
- **Low issues:** 50+ abilities
- **Total affected:** ~100+ abilities (40% of total)

## Next Steps

1. Implement count extraction fixes
2. Add multi-condition parsing
3. Structure parenthetical rules properly
4. Add comprehensive validation
5. Update engine to handle all cases
6. Create test suite for all scenarios
