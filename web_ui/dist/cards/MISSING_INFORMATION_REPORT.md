# Missing Information Report: Abilities.json

**Generated:** 2026-04-21  
**Total Abilities:** 602 unique abilities  
**Analysis Method:** Comparison of full_text with parsed structured fields

---

## Executive Summary

The parser successfully extracts most semantic information from ability texts, but several categories of information are either missing or not fully structured. This report identifies gaps between the raw ability text and the parsed JSON output.

---

## 1. Custom Action Types (4 instances)

Abilities that could not be parsed into a specific action type remain as `"action": "custom"`. These represent unhandled ability patterns.

### 1.1 Re-yell Action
**Location:** Line 7084  
**Full text:** "そのエールで得たブレードハートを失い、もう一度エールを行う"  
**Parsed:** Only condition is extracted (yell_revealed_condition), action is custom  
**Missing:** The actual re-yell action logic - losing blade hearts and performing another yell

### 1.2 Gain Blade with Equality Condition
**Location:** Line 8508  
**Full text:** "{{icon_blade.png|ブレード}}を得る。それぞれのメンバーのコストが同じ"  
**Parsed:** Condition extracted as comparison_condition (equality), action is custom  
**Missing:** The gain_resource action with the equality condition properly integrated

### 1.3 Restriction Effect
**Location:** Line 8913  
**Full text:** "このターン、自分と相手のステージにいるメンバーは、効果によってはアクティブにならない"  
**Parsed:** Target and card_type extracted, action is custom  
**Missing:** The restriction type and condition (effects that can't activate members)

### 1.4 Energy Placement Under Member
**Location:** Line 10063  
**Full text:** Complex condition about placing energy under member  
**Parsed:** Condition extracted (location_count_condition), source is "under_member", action is custom  
**Missing:** The actual place_energy_under_member action with proper parameters

---

## 2. Icon Information in Text

Specific resource icon types (heart_01, heart_02, etc.) are preserved in `full_text` but not always extracted as structured data.

### 2.1 Heart Type Selection
**Example:** Line 536-555  
**Full text:** "{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ"  
**Parsed:** Only `per_unit: true` is set  
**Missing:** 
- The choice options (heart_01, heart_03, heart_06) are not structured
- The selection mechanism is not captured as a choice action

### 2.2 Resource Icon Counts
**Observation:** Many abilities have `resource_icon_count` extracted, but the specific icon types (heart_01 vs heart_02 vs heart_06) are not distinguished in the parsed structure.

---

## 3. Cost Reduction Logic

Abilities that reduce costs based on conditions are not fully structured.

### 3.1 Group-Based Cost Reduction
**Example:** Line 3033-3054  
**Full text:** "この能力を起動するためのコストは自分のステージにいるメンバーの中のグループ名1種類につき、{{icon_energy.png|E}}減る"  
**Parsed:** `per_unit: true`, `per_unit_type: "member"`  
**Missing:** 
- The cost reduction logic is not captured
- The base cost (4 energy) is in cost field, but the reduction is not structured
- Should have a `cost_reduction` field with the reduction amount and condition

### 3.2 Card-Based Cost Reduction
**Example:** Line 3549+  
**Full text:** "コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る"  
**Parsed:** This type of cost modification needs verification

---

## 4. Conditional Branching

Abilities with "if/then" logic that branch based on conditions may not be fully captured.

### 4.1 Look and Select with Conditional Branch
**Example:** Line 3000-3030  
**Full text:** "これにより控え室に置いたカードが『μ's』のカードの場合、自分のデッキの上からカードを4枚見る...『μ's』のカード以外の場合、自分の控え室からライブカードを1枚手札に加える"  
**Parsed:** 
- look_action has the condition text embedded
- select_action only captures the first branch  
**Missing:**
- The conditional branching structure is not captured
- Should be a conditional_alternative or similar structure
- The "otherwise" branch is missing from parsed structure

### 4.2 Sequential Conditional Effects
**Example:** Line 3517-3546  
**Full text:** "これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、このメンバーをアクティブにする。2枚ある場合、さらにライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"  
**Parsed:** Only the second condition is captured  
**Missing:**
- The first condition (1枚以上ある場合) and its effect (activate member) are not in the effect structure
- The sequential conditional nature is not captured

---

## 5. Per-Unit Context

The `per_unit` field is sometimes a boolean (due to recent Rust compatibility changes) instead of preserving the actual text context.

### 5.1 Boolean vs Text
**Observation:** User changed `per_unit` from string to boolean for Rust compatibility  
**Impact:** The actual context (e.g., "自分の成功ライブカード置き場にあるカード1枚") is lost  
**Recommendation:** Keep both the boolean flag and the raw text context for full information

---

## 6. Position Information

Position requirements (center, left_side, right_side) are being commented out for Rust compatibility.

### 6.1 Commented Out Position Extraction
**Location:** Line 1165+ in parser.py  
**Change:** Position extraction code commented out with note "Don't set position as string - Rust expects PositionInfo struct"  
**Impact:** Position information is completely missing from parsed output  
**Examples Missing:**
- "センターエリアにいる場合" → should have position: "center"
- "左サイドエリア" → should have position: "left_side"
- "右サイドエリア" → should have position: "right_side"

---

## 7. Activation Conditions

Some activation conditions are extracted but not fully integrated.

### 7.1 Position-Based Activation
**Example:** Line 2017-2022  
**Full text:** "（この能力はセンターエリアに登場している場合のみ起動できる。）"  
**Parsed:** `activation_position: "center"` extracted  
**Status:** This is working correctly

### 7.2 Card Location Activation
**Example:** Line 10067+  
**Full text:** "この能力は、このカードが手札にある場合のみ起動できる"  
**Parsed:** Should have activation_location condition  
**Status:** Needs verification

---

## 8. Duration Markers

Duration information is generally well-extracted, but some edge cases may be missing.

### 8.1 Complex Duration
**Observation:** Most "ライブ終了時まで" (until live end) are captured as `duration: "live_end"`  
**Status:** This is working correctly

---

## 9. Parenthetical Notes

Parenthetical notes are extracted but may not be fully categorized.

### 9.1 Note Types
**Examples:**
- "(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)" - Rule clarification
- "(エールで出た{{icon_score.png|スコア}}1つにつき、成功したライブのスコアの合計に1を加算する。)" - Scoring rule

**Parsed:** These are correctly marked as `is_null: true` with `effect: null`  
**Status:** This is correct behavior - these are notes, not abilities

---

## Summary Statistics

| Category | Count | Severity |
|----------|-------|----------|
| Custom action types | 4 | High |
| Missing heart type choices | ~5-10 | Medium |
| Cost reduction logic | ~5-10 | Medium |
| Conditional branching | ~5-10 | Medium |
| Position information (commented out) | ~20-30 | High |
| Per-unit context loss | ~50+ | Low |

---

## Recommendations

### High Priority
1. **Fix custom action types** - Add parsers for re-yell, restriction with conditions, and energy placement
2. **Restore position extraction** - Either parse as PositionInfo struct or keep string format with proper Rust mapping
3. **Add cost reduction parsing** - Extract cost modification logic with conditions

### Medium Priority
4. **Structure heart type choices** - Parse choice mechanisms with specific heart options
5. **Capture conditional branching** - Add support for if/then/else structures in abilities
6. **Preserve per-unit context** - Keep both boolean flag and raw text

### Low Priority
7. **Verify activation conditions** - Ensure all card/location activation conditions are captured
8. **Categorize parenthetical notes** - Distinguish between rule clarifications, scoring notes, etc.

---

## Conclusion

The parser captures most semantic information well (30+ condition types, 20+ action types). The main gaps are:
1. 4 custom action types that need new parsers
2. Position extraction disabled for Rust compatibility
3. Complex conditional structures not fully captured
4. Choice mechanisms with specific options not structured

The full_text field preserves all original information, so no data is permanently lost. The missing structured information can be recovered by enhancing the parser with additional patterns and action types.
