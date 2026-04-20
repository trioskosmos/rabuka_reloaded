# Abilities.json Issues Found

## Issue 1: Parenthetical notes split across sequential actions (FIXED)
**Location**: Line 342-364
**Problem**: Parenthetical notes are extracted but also included in sequential actions, causing incorrect splitting
**Example**:
```json
"effect": {
  "text": "自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）",
  "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"],
  "actions": [
    {
      "text": "残りを控え室に置く。（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は",  // WRONG - includes part of parenthetical
      ...
    },
    {
      "text": "エールで公開する枚数を増やさない。）",  // WRONG - rest of parenthetical as separate action
      ...
    }
  ]
}
```
**Fix**: Strip parenthetical notes from text before parsing sequential actions (Added `strip_parenthetical` function and use it before sequential check)

## Issue 2: Custom cost type for "wait" actions
**Location**: Lines 335-338, 383-386
**Problem**: Costs like "このメンバーをウェイトにしてもよい" classified as "custom" instead of "change_state"
**Example**:
```json
"cost": {
  "text": "このメンバーをウェイトにしてもよい",
  "card_type": "member_card",
  "optional": true,
  "type": "custom"  // Should be "change_state"
}
```
**Fix**: Add pattern for "ウェイトにする" in cost parsing to classify as change_state

## Issue 3: Sequential look_and_select parsed as move_cards
**Location**: Line 147-152
**Problem**: "自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え" parsed as move_cards with count=3
**Example**:
```json
{
  "text": "自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え",
  "source": "deck_top",
  "destination": "hand",
  "count": 3,  // WRONG - this is the look count, not the move count
  "action": "move_cards"  // Should be look_and_select
}
```
**Fix**: Check for "見る。その中から" pattern before sequential parsing

## Issue 4: Conditions with "か" (or) parsed as custom
**Location**: Line 2622-2625
**Problem**: Conditions like "このメンバーが登場か、エリアを移動した" classified as custom
**Example**:
```json
"condition": {
  "text": "このメンバーが登場か、エリアを移動した",
  "type": "custom"  // Should be "either_case" or similar
}
```
**Fix**: Parse "か" in conditions as either_case type

## Issue 5: Cost/effect split failing on multiple colons
**Location**: Line 674-681, 1351-1358, 1805-1816
**Problem**: Text with multiple colons or parentheses splits at wrong position
**Example 1**:
```json
"full_text": "{{kidou.png|起動}}{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く：このカードを控え室からステージに登場させる。この能力は、このカードが控え室にある場合のみ起動できる。",
"effect": {
  "text": "のみ起動できる",  // WRONG - should be full effect text
  ...
}
```
**Example 2**:
```json
"full_text": "手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く：これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを発動させる。\n({{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。)",
"effect": {
  "text": "支払って発動させる。)",  // WRONG - should be full effect text including parenthetical
  ...
}
```
**Expected**: Should split at first colon only and keep parenthetical notes in effect text
**Fix**: Check extract_card_abilities.py split_cost_effect logic - it may be splitting incorrectly when parentheses are present

## Issue 6: "控え室からステージに登場させる" not classified correctly
**Location**: Line 676
**Problem**: Move from discard to stage not recognized as move_cards
**Example**:
```json
"condition": {
  "text": "このカードを控え室からステージに登場させる。この能力は、このカードが控え室にある",
  "location": "discard",
  "type": "custom"  // Should be move_cards with source=discard, destination=stage
}
```
**Fix**: Add "登場させる" pattern for move_cards with destination=stage

## Issue 7: Destination inference wrong for "手札に加える" ✅ FIXED
**Location**: Lines 58-64, 102-108 (and likely many more)
**Problem**: Effects adding cards to hand have destination="discard" instead of "hand"
**Example**:
```json
"effect": {
  "text": "自分の控え室からライブカードを1枚手札に加える",
  "source": "discard",
  "destination": "discard",  // WRONG - should be "hand"
  "count": 1,
  "card_type": "live_card",
  "target": "self",
  "action": "move_cards"
}
```
**Root cause**: DESTINATION_PATTERNS has ('控え室', 'discard') which is too broad. In text "自分の控え室からライブカードを1枚手札に加える", the pattern '控え室' matches before '手札に加える' is checked, so it returns 'discard' as destination
**Fix**: Removed overly broad ('控え室', 'discard') pattern from DESTINATION_PATTERNS - it was matching source locations

## Issue 8: per_unit extraction uses generic placeholder
**Location**: Line 524, 1028, 7536, 8745
**Problem**: per_unit field contains generic placeholder "あるカード1枚" instead of actual text
**Example**:
```json
"effect": {
  "text": "{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る",
  "per_unit": "あるカード1枚",  // Generic placeholder - should be "自分の成功ライブカード置き場にあるカード1枚"
  ...
}
```
**Impact**: Medium - the information is still in the text field, but per_unit is not useful
**Fix**: Check per_unit extraction logic to use actual text instead of generic placeholder

## Issue 9: "バトンタッチして登場した" parsed as custom condition
**Location**: Line 1309
**Problem**: Baton touch trigger condition classified as custom instead of specific type
**Example**:
```json
"condition": {
  "text": "バトンタッチして登場した",
  "type": "custom"  // Should be "baton_touch" or similar
}
```
**Impact**: Low - still has the text, just not classified
**Fix**: Add pattern for "バトンタッチ" in condition parsing

## Issue 10: "登場か、エリアを移動した" - "か" pattern in conditions not handled
**Location**: Line 2622
**Problem**: Condition with "か" (or) pattern classified as custom
**Example**:
```json
"condition": {
  "text": "このメンバーが登場か、エリアを移動した",
  "type": "custom"  // Should be "either_case" or similar
}
```
**Impact**: Low - still has the text, just not classified
**Fix**: Add pattern for "か" in conditions to classify as "either_case"

## Issue 11: Missing position and count in gain_resource effects ✅ FIXED
**Location**: Line 14779 (and likely many more)
**Problem**: Effects with position (e.g., "センターエリア") and multiple resource icons missing position and count fields
**Example**:
```json
"effect": {
  "text": "ライブ終了時まで、自分のセンターエリアにいる『μ's』のメンバーは、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "card_type": "member_card",
  "target": "self",
  "action": "gain_resource",
  "resource": "blade"
  // Missing: "position": "center", "count": 2
}
```
**Expected**: Should include:
- "position": "center" (from "センターエリア")
- "count": 2 (from two {{icon_blade.png}} icons)
**Fix**: Added position extraction for "センターエリア" and count extraction for multiple resource icons

## Issue 12: Missing comparison information in conditions ✅ FIXED
**Location**: Line 14801 (and likely many more)
**Problem**: Conditions with score comparisons missing comparison details
**Example**:
```json
"condition": {
  "text": "自分の成功ライブカード置き場にあるカードのスコアの合計が相手より高いかぎり",
  "target": "self",
  "location": "success_live_card_zone",
  "card_type": "live_card",
  "type": "location_condition"
  // Missing: "comparison_target": "opponent", "comparison_type": "score", "comparison_operator": ">"
}
```
**Expected**: Should include:
- "comparison_target": "opponent" (from "相手")
- "comparison_type": "score" (from "スコア")
- "comparison_operator": ">" (from "高い")
- "aggregate": "total" (from "合計")
**Fix**: Added comparison extraction logic for conditions with "相手より高い/低い" patterns

## Issue 13: Missing character-specific resource mapping
**Location**: Line 11885 (and likely many more)
**Problem**: Effects where different characters get different resources not capturing the mapping
**Example**:
```json
"effect": {
  "text": "ライブ終了時まで、自分のステージにいる「澁谷かのん」1人は{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を、「唐可可」1人は{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る",
  "count": 1,
  "target": "self",
  "quoted_text": [
    "澁谷かのん",
    "唐可可"
  ],
  "action": "gain_resource",
  "resource": "blade"
  // Missing: character-specific resource mapping
}
```
**Expected**: Should include:
- "character_effects": [
    {"character": "澁谷かのん", "resource": "heart05", "count": 2},
    {"character": "唐可可", "resource": "heart01", "count": 2}
  ]
**Fix**: Add character-specific resource extraction for patterns like "「X」はYを得る"

## Issue 14: Position change abilities missing swap logic and destination details
**Location**: Lines 2063, 2559, 3494, 3550
**Problem**: Position change abilities have complex logic not fully extracted
**Example 1 - Swap logic**:
```json
"effect": {
  "text": "このメンバーはセンターエリア以外にポジションチェンジする。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)",
  "position": "center",
  "action": "position_change"
  // Missing: "swap": true, "destination": "not_center", "exchange_with": true
}
```
**Example 2 - Optional position change with empty actions**:
```json
"effect": {
  "text": "このメンバーをポジションチェンジしてもよい。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)",
  "actions": []
  // Wrong: should be action: "position_change", optional: true
}
```
**Example 3 - Group-based destination**:
```json
"effect": {
  "text": "このメンバーを『Aqours』か『SaintSnow』のメンバーがいるエリアにポジションチェンジする",
  "group": {
    "name": "Aqours",
    "type": "unit"
  },
  "group_names": ["Aqours", "SaintSnow"],
  "action": "position_change"
  // Missing: "destination_criteria": "group_members_present", "destination_groups": ["Aqours", "SaintSnow"]
}
```
**Expected**: Should include:
- "swap": true (when parenthetical mentions exchange)
- "destination": "not_center" or specific area
- "destination_criteria": "group_members_present" (for group-based positioning)
- "exchange_with": true (when members swap positions)
- "optional": true (for optional changes)
**Fix**: Add position_change subtype analysis to detect swap logic, destination criteria, and exchange patterns

## Issue 15: Movement conditions vs actions not properly distinguished ✅ FIXED
**Location**: Lines 2578, 6699, 9765
**Problem**: "移動した" (moved) and "移動させる" (make move) patterns not distinguished
**Example 1 - Movement condition**:
```json
"condition": {
  "text": "このメンバーが登場か、エリアを移動した",
  "card_type": "member_card",
  "movement": true,
  "type": "custom"
}
```
**Example 2 - Movement action**:
```json
"effect": {
  "text": "このメンバーを今いるエリア以外のエリアに移動させる",
  "action": "position_change"
}
```
**Fix**: Added `movement: true` flag for conditions with "移動した"/"移動する", and context-aware action classification for "移動させる" (position_change if involves "エリア", else move_cards)

## Issue 16: Missing temporal conditions, movement states, and activation conditions ✅ FIXED
**Location**: Line 15402 (and likely many more)
**Problem**: Complex conditions with temporal scope, ongoing states, and activation constraints not fully extracted
**Example**:
```json
"full_text": "{{live_start.png|ライブ開始時}}{{leftside.png|左サイド}}このターン、このメンバーがエリアを移動している場合、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。（この能力は左サイドエリアにいる場合のみ発動する。）",
"effect": {
  "text": "ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。（この能力は左サイドエリアにいる場合のみ発動する。）",
  "parenthetical": ["この能力は左サイドエリアにいる場合のみ発動する。"],
  "condition": {
    "text": "このターン、このメンバーがエリアを移動している",
    "card_type": "member_card",
    "movement_state": "has_moved",
    "temporal": "this_turn",
    "type": "custom"
  },
  "activation_condition": "この能力は左サイドエリアにいる場合のみ発動する。",
  "activation_position": "left_side",
  "position": "left_side"
}
```
**Fix**: Added temporal scope extraction ("このターン" → temporal: "this_turn", "このライブ" → temporal: "this_live"), movement state detection ("移動している" → movement_state: "has_moved"), and activation condition parsing for parenthetical notes with "発動する" or "起動できる"

## Issue 17: Energy count conditions not classified
**Location**: Lines 881, 2505 (and likely many more)
**Problem**: Conditions about energy count are type: "custom" instead of specific energy condition type
**Example**:
```json
"condition": {
  "text": "自分のエネルギーが11枚以上ある",
  "target": "self",
  "count": 11,
  "operator": ">=",
  "type": "custom"
  // Missing: "location": "energy_zone", "card_type": "energy_card"
}
```
**Expected**: Should include:
- "location": "energy_zone"
- "card_type": "energy_card"
- "type": "location_count_condition"
**Fix**: Add energy count condition detection for patterns like "エネルギーが～枚"

## Issue 18: Empty actions arrays for complex conditional effects
**Location**: Lines 1442, 1859, 1951, 2576, 4622 (and likely many more)
**Problem**: Effects with "そうした場合" (if so/then) patterns result in empty actions array instead of parsed sequential actions
**Example**:
```json
"effect": {
  "text": "このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。そうした場合、自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる。（この能力はセンターエリアに登場している場合のみ起動できる。）",
  "actions": []
}
```
**Expected**: Should be parsed as sequential actions:
```json
"action": "sequential",
"actions": [
  {"action": "move_cards", "source": "stage", "destination": "discard", ...},
  {"action": "move_cards", "source": "discard", "destination": "stage", ...}
]
```
**Fix**: Add parsing for "そうした場合" (if so/then) pattern to split into sequential conditional actions

## Issue 19: Position-based conditions not classified
**Location**: Line 811 (and likely many more)
**Problem**: Conditions with position information like "左サイドエリアに登場している" are type: "custom" instead of "location_condition"
**Example**:
```json
"condition": {
  "text": "ステージの左サイドエリアに登場している",
  "location": "stage",
  "position": "left_side",
  "type": "custom"
  // Should be: "type": "location_condition"
}
```
**Expected**: Should be:
```json
"condition": {
  "text": "ステージの左サイドエリアに登場している",
  "location": "stage",
  "position": "left_side",
  "type": "location_condition"
}
```
**Fix**: Update condition type classification to use "location_condition" when location and position are present

## Issue 20: Baton touch conditions not classified
**Location**: Lines 1329, 1596, 2820 (and likely many more)
**Problem**: Conditions about baton touch (バトンタッチ) appearance are type: "custom" instead of specific condition type
**Example**:
```json
"condition": {
  "text": "バトンタッチして登場した",
  "type": "custom"
  // Missing: "trigger_type": "baton_touch"
}
```
**Example 2**:
```json
"condition": {
  "text": "能力を持たないメンバーからバトンタッチして登場した",
  "card_type": "member_card",
  "type": "custom"
  // Missing: "trigger_type": "baton_touch", "source_ability": "none"
}
```
**Expected**: Should include:
- "trigger_type": "baton_touch"
- "source_group": "スリーズブーケ" (if specified)
- "source_ability": "none" (if "能力を持たない")
**Fix**: Add baton touch trigger condition detection for patterns like "バトンタッチして登場した"

## Issue 21: Cost reduction effects not classified
**Location**: Line 2845 (and likely many more)
**Problem**: Effects that reduce costs are classified as action: "custom" instead of specific cost modification type
**Example**:
```json
"effect": {
  "text": "能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る",
  "source": "hand",
  "card_type": "member_card",
  "target": "self",
  "action": "custom"
  // Should be: "action": "modify_cost", "modification": "decrease", "value": 1
}
```
**Expected**: Should include:
- "action": "modify_cost"
- "modification": "decrease" or "increase"
- "value": 1 (or other amount)
- "target_card_type": "member_card" (if specified like "能力を持たないメンバーカード")
**Fix**: Add cost modification effect detection for patterns like "コストは～減る" (cost decreases by) or "コストは～増える" (cost increases by)

## Issue 22: Required hearts modification effects not classified
**Location**: Line 12759 (and likely many more)
**Problem**: Effects that modify required hearts are sometimes classified as action: "custom" instead of "modify_required_hearts"
**Example**:
```json
"effect": {
  "text": "自分のステージにいる、このターン中に登場、またはエリアを移動した『5yncri5e!』のメンバー1人につき、このカードを成功させるための必要ハートを{{heart_00.png|heart0}}減らす",
  "per_unit": "登場、またはエリアを移動した『5yncri5e!』のメンバー1人",
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "group": {
    "name": "5yncri5e!",
    "type": "unit"
  },
  "group_names": ["5yncri5e!"],
  "action": "custom"
  // Should be: "action": "modify_required_hearts", "operation": "decrease", "value": 1
}
```
**Expected**: Should include:
- "action": "modify_required_hearts"
- "operation": "decrease" or "increase"
- "value": 1 (or other amount)
**Fix**: Add required hearts modification detection for patterns like "必要ハートを～減らす" or "必要ハートが～多くなる"

## Issue 23: Limit modification effects not classified
**Location**: Line 13186 (and likely many more)
**Problem**: Effects that modify limits (like card placement limits) are classified as action: "custom" instead of specific limit modification type
**Example**:
```json
"effect": {
  "text": "自分の控え室からライブカードを1枚、表向きでライブカード置き場に置く。次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る",
  "source": "discard",
  "destination": "live_card_zone",
  "count": 1,
  "card_type": "live_card",
  "target": "self",
  "action": "move_cards"
  // Missing: sequential action for limit modification
}
```
**Expected**: Should be parsed as sequential actions:
```json
"action": "sequential",
"actions": [
  {"action": "move_cards", "source": "discard", "destination": "live_card_zone", ...},
  {"action": "modify_limit", "limit_type": "card_placement", "operation": "decrease", "value": 1, "target": "self"}
]
```
**Fix**: Add limit modification detection for patterns like "～枚数の上限が～減る" or "～枚数の上限が～増える"

## Issue 24: OR conditions not split into sub-conditions
**Location**: Line 2596 (and likely many more)
**Problem**: Conditions with "か、" (or) are not being split into separate sub-conditions
**Example**:
```json
"condition": {
  "text": "このメンバーが登場か、エリアを移動した",
  "card_type": "member_card",
  "movement": true,
  "type": "custom"
  // Should be: "type": "or_condition", "conditions": [...]
}
```
**Expected**: Should be parsed as:
```json
"condition": {
  "text": "このメンバーが登場か、エリアを移動した",
  "type": "or_condition",
  "conditions": [
    {"text": "このメンバーが登場", "trigger_type": "appearance"},
    {"text": "エリアを移動した", "movement": true}
  ]
}
```
**Fix**: Add OR condition detection for patterns like "～か、～" to split into sub-conditions

## Issue 25: Conditional alternative effects not classified
**Location**: Lines 1227, 8617, 14234 (and likely many more)
**Problem**: Effects with "代わりに" (instead/otherwise) indicating conditional alternatives are not being parsed as such
**Example**:
```json
"effect": {
  "text": "ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋1する。」を得る。2枚以上ある場合、代わりに「{{jyouji.png|常時}}ライブの合計スコアを＋2する。」を得る。",
  "parenthetical": ["この能力はセンターエリアに登場した場合のみ発動する。"],
  "activation_condition": "この能力はセンターエリアに登場した場合のみ発動する。",
  "activation_position": "center",
  "condition": {...},
  "action": "gain_resource"
  // Missing: alternative effect for "代わりに" case
}
```
**Expected**: Should be parsed as conditional alternative:
```json
"action": "conditional_alternative",
"primary_effect": {"action": "gain_resource", ...},
"alternative_condition": {"text": "2枚以上ある", ...},
"alternative_effect": {"action": "gain_resource", "value": 2}
```
**Fix**: Add conditional alternative detection for patterns like "～場合、代わりに～"

## Issue 26: Ability invalidation effects not classified
**Location**: Lines 1268, 13628 (and likely many more)
**Problem**: Effects that invalidate/nullify abilities are not being classified as specific action type
**Example**:
```json
"condition": {
  "text": "自分のステージにいる『Liella!』のメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした",
  "target": "self",
  "location": "stage",
  "card_type": "member_card",
  "count": 1,
  "group": {...},
  "type": "group_condition"
  // Missing: ability invalidation action
}
```
**Expected**: Should include:
- "action": "invalidate_ability"
- "target_ability_trigger": "ライブ開始時"
- "target_ability_group": "Liella!"
- "optional": true (for "してもよい")
**Fix**: Add ability invalidation detection for patterns like "～能力を無効にする" or "～能力を無効にしてもよい"

## Issue 27: Surplus heart conditions not classified
**Location**: Lines 3040, 4415, 4715 (and likely many more)
**Problem**: Conditions about surplus hearts (余剰ハート) are type: "custom" instead of specific condition type
**Example**:
```json
"condition": {
  "text": "自分が余剰ハートを1つ以上持っている",
  "operator": ">=",
  "type": "custom"
  // Should be: "type": "surplus_heart_condition", "count": 1
}
```
**Expected**: Should include:
- "type": "surplus_heart_condition"
- "count": 1 (or other amount)
- "target": "self" or "opponent"
**Fix**: Add surplus heart condition detection for patterns like "余剰ハートが～ある"

## Issue 28: Effect constraints not extracted
**Location**: Line 4715 (and likely many more)
**Problem**: Constraints on effects (like "score won't go below 0") are not being extracted
**Example**:
```json
"effect": {
  "text": "ライブの合計スコアを＋1する。自分が余剰ハートを2つ以上持つ場合、ライブの合計スコアを－1する。この効果ではライブの合計スコアは0未満にはならない",
  "condition": {...},
  "action": "modify_score",
  "operation": "add"
  // Missing: "constraint": {"type": "minimum_value", "value": 0}
}
```
**Expected**: Should include:
- "constraint": {"type": "minimum_value", "value": 0} (or "maximum_value")
**Fix**: Add effect constraint detection for patterns like "～未満にはならない" or "～以上にはならない"

## Issue 29: Distinct card name and group name conditions not fully classified
**Location**: Line 4725 (and likely many more)
**Problem**: Conditions about distinct card names ("カード名が異なる") and group names ("グループ名が異なる") should use specific distinct types
**Example**:
```json
"option": {
  "text": "自分の控え室にカード名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。",
  "source": "discard",
  "destination": "hand",
  "count": 3,
  "card_type": "live_card",
  "target": "self",
  "action": "move_cards"
  // Missing: "distinct": "card_name" in condition
}
```
**Expected**: Should include:
- "condition": {"distinct": "card_name"} (for "カード名が異なる")
- "condition": {"distinct": "group_name"} (for "グループ名が異なる")
- "condition": {"distinct": "name"} (for "名前が異なる" - member names)
**Fix**: Update distinct pattern detection to distinguish between member names, card names, and group names

## Issue 30: Dynamic cost based on card score not extracted
**Location**: Line 4513 (and likely many more)
**Problem**: Costs that depend on a card's score ("そのカードのスコアに等しい数の") are not being extracted
**Example**:
```json
"effect": {
  "text": "そのライブカードを手札に加える",
  "condition": {
    "text": "自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{{icon_energy.png|E}}を支払ってもよい。そうした",
    "target": "self",
    "location": "discard",
    "card_type": "live_card",
    "count": 1,
    "comparison_type": "score",
    "type": "location_condition"
  },
  "destination": "hand",
  "card_type": "live_card",
  "action": "move_cards"
  // Missing: dynamic cost information
}
```
**Expected**: Should include:
- "dynamic_cost": {"type": "pay_energy", "source": "card_score", "optional": true}
**Fix**: Add dynamic cost detection for patterns like "～に等しい数の～を支払う"
