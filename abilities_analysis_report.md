# Abilities.json vs Engine Code Analysis Report

## Overview
This report analyzes potential issues found when comparing the `abilities.json` file with the engine's ability handling implementation in the Rust codebase.

**Analysis Date:** 2026-05-06  
**Files Analyzed:**
- `cards/abilities.json` (1,399 abilities, 651 unique abilities)
- `engine/src/ability/` modules
- `engine/src/core/card.rs` (Ability, AbilityCost, AbilityEffect structs)

## Critical Issues

### 1. **Missing Dynamic Count Support in Engine**
**Severity:** HIGH  
**Location:** `engine/src/ability/effects.rs`

The JSON contains extensive use of `dynamic_count` fields that are not handled by the engine:

```json
"dynamic_count": {
  "type": "remaining_looked_at",
  "reference": "previous_look"
}
```

**Examples found in abilities.json:**
- 30+ instances of `"type": "remaining_looked_at"`
- 5+ instances of `"type": "dynamic_count"` with `"mode": "max"`
- 3+ instances of `"type": "per_unit"`

**Impact:** Abilities with dynamic counting will fail or use incorrect counts.

### 2. **Generic "card_type" Usage**
**Severity:** MEDIUM  
**Location:** Multiple abilities in `abilities.json`

Many abilities use `"card_type": "card"` which is overly generic:

```json
"card_type": "card"
```

**Issues:**
- Engine expects specific card types (`"member_card"`, `"live_card"`, `"energy_card"`)
- May cause filtering issues in card selection logic
- Found in 50+ ability entries

### 3. **Unhandled Special Source Zones**
**Severity:** MEDIUM  
**Location:** `engine/src/ability/move_cards.rs` (likely)

JSON uses special source zones not standard in engine:

```json
"source": "looked_at_remaining"
"source": "looked_at"
```

**Impact:** Look-and-select abilities may fail when moving remaining cards.

## Structural Issues

### 4. **Complex Nested Action Structures**
**Severity:** MEDIUM

JSON contains deeply nested action structures that may not be fully supported:

```json
{
  "action": "sequential",
  "actions": [
    {
      "action": "look_and_select",
      "look_action": { ... },
      "select_action": {
        "action": "sequential",
        "actions": [ ... ]
      }
    }
  ]
}
```

### 5. **Missing Field Validation**
**Severity:** LOW

JSON contains fields that may not be validated in engine:
- `placement_order: "any_order"`
- `any_number: true`
- `dynamic_count` variations
- `activation_condition_parsed` (complex nested structures)

## Data Consistency Issues

### 6. **Inconsistent Card Type References**
**Severity:** LOW

Mixed usage patterns found:
- Some abilities use specific types: `"live_card"`, `"member_card"`
- Others use generic: `"card"`
- No clear pattern for when to use which

### 7. **Complex Count Logic**
**Severity:** MEDIUM

Several abilities have complex count calculations that may not be handled:

```json
"dynamic_count": {
  "type": "dynamic_count",
  "reference": "自分のライブの合計スコアに2を足した数",
  "mode": "equals"
}
```

## Engine-Specific Concerns

### 8. **EffectAction Enum Coverage**
**Severity:** MEDIUM

In `effects.rs`, unknown actions fall back to `DoNothing`:

```rust
_ => { eprintln!("Unknown effect action: '{}'", s); Self::DoNothing }
```

**Risk:** Any new action types in JSON will be silently ignored.

### 9. **Missing Error Handling**
**Severity:** LOW

No evidence of proper error handling for:
- Invalid dynamic_count types
- Missing required fields
- Malformed action structures

## Recommendations

### Immediate Actions (High Priority)
1. **Implement Dynamic Count Support** in `engine/src/ability/effects.rs`
2. **Add Validation** for `card_type` field usage
3. **Handle Special Source Zones** (`looked_at`, `looked_at_remaining`)

### Medium Priority
1. **Review Nested Action Support** in sequential execution
2. **Add Comprehensive Error Logging** for unknown actions
3. **Standardize Card Type Usage** across all abilities

### Low Priority
1. **Add Field Validation** for optional fields
2. **Improve Documentation** for supported action types
3. **Create Unit Tests** for complex ability patterns

## Statistics

- **Total abilities analyzed:** 1,399
- **Unique abilities:** 651  
- **Cards with abilities:** 1,113
- **Critical issues found:** 3
- **Medium issues found:** 6
- **Low issues found:** 3

## Testing Recommendations

1. **Test all look_and_select abilities** with dynamic counts
2. **Verify card type filtering** works correctly
3. **Test sequential actions** with nested structures
4. **Validate error handling** for malformed abilities

## Files to Review

1. `engine/src/ability/effects.rs` - Add dynamic count support
2. `engine/src/ability/move_cards.rs` - Handle special source zones  
3. `engine/src/ability/util.rs` - Add card type validation
4. `engine/src/core/card.rs` - Review AbilityEffect struct completeness
