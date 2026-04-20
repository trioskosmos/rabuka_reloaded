# Abilities.json Issues Found

## Summary
Total "action": "custom" instances found: 26 (before fixes)
Total empty actions arrays: 2 (fixed by moving either-case check earlier)

## Issues Fixed in This Session

### Issue 1: Cost source extraction missing '控え室にある' pattern
**Location**: Line 2044 and similar
**Problem**: Costs like "控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい" had source not extracted, resulting in type="custom"
**Fix**: Added ('控え室にある', 'discard') to SOURCE_PATTERNS in parser.py

### Issue 2: OR conditions ("か") not parsed correctly
**Location**: Line 2694 and similar
**Problem**: Conditions like "このメンバーが登場か、エリアを移動した" were caught by movement condition check before OR check, resulting in type="movement_condition" instead of type="or_condition"
**Fix**: Moved OR condition check before movement condition check in parse_condition()

### Issue 3: Either-case conditions split incorrectly by general conditional check
**Location**: Line 843-845
**Problem**: Text like "公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、..." was split on "場合" by general conditional check before either-case specific handling, resulting in empty actions array
**Fix**: Moved either-case check before general conditional check in parse_effect()

### Issue 4: Select actions not classified
**Location**: Line 4241 and similar
**Problem**: Actions like "自分の控え室からライブカードを1枚選び" are type="custom"
**Fix**: Added pattern for "～選び" to classify as select action in parser.py

### Issue 5: Discard until count actions not classified
**Location**: Line 3869 and similar
**Problem**: Actions like "自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き" are type="custom"
**Fix**: Added pattern for "～枚になるまで～控え室に置く" to classify as discard_until_count in parser.py

### Issue 6: Reveal per group actions not classified
**Location**: Line 4550 and similar
**Problem**: Actions like "各グループ名につき1枚ずつ公開し" are type="custom"
**Fix**: Added pattern for "各グループ名につき1枚ずつ公開し" to classify as reveal_per_group in parser.py

### Issue 7: Per-trigger payment actions not classified
**Location**: Line 3173 and similar
**Problem**: Actions like "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。" are type="custom"
**Fix**: Added pattern for "～たび、～支払ってもよい" to classify as pay_energy_per_trigger in parser.py

## Remaining Issues

### Issue 8: Alternative effect primary actions not classified
**Location**: Line 1203 and similar
**Problem**: Primary effects in conditional alternative structures are type="custom"
**Fix**: Ensure primary effects in conditional alternatives are properly classified

### Issue 9: Null entries with empty text
**Location**: Lines 274, 3682 and similar
**Problem**: Some entries have empty text and is_null:true, marked as type="custom"
**Fix**: These are null entries and should be ignored or marked as "null" type

## New Structural Issues Found (Manual Review)

### Issue 10: Conditional effects with empty actions arrays ✅ FIXED
**Location**: Lines 711, 863, 1039, 1232, 1295, 1403, 1429, 1453 and similar
**Problem**: Many conditional effects result in empty actions arrays instead of being parsed into condition + action structures
**Examples**:
```json
// Line 711: Activation condition not parsed
"effect": {
  "text": "このカードを控え室からステージに登場させる。この能力は、このカードが控え室にある場合のみ起動できる。",
  "actions": []
}

// Line 863: Energy count condition not parsed
"effect": {
  "text": "自分のエネルギーが11枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。",
  "actions": []
}

// Line 1039: Member presence condition not parsed
"effect": {
  "text": "自分のステージにほかのメンバーがいる場合、好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る。",
  "actions": []
}

// Line 1453: Position condition not parsed
"effect": {
  "text": "自分のステージのエリアすべてにメンバーが登場している場合、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。",
  "actions": []
}
```
**Fix**: Added activation condition extraction for "～場合のみ起動できる" patterns in parser.py

### Issue 11: Ability activation effects ✅ FIXED
**Location**: Line 1295 and similar
**Problem**: Effects that activate other abilities are not being parsed
**Example**:
```json
"effect": {
  "text": "これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを発動させる。\n({{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。)",
  "actions": []
}
```
**Fix**: Added ability activation action type for patterns like "～能力を発動させる" in parser.py

### Issue 12: Ability invalidation with conditional follow-up ✅ FIXED
**Location**: Line 1232 and similar
**Problem**: Complex ability invalidation with "これにより～した場合" conditional follow-up not parsed
**Example**:
```json
"effect": {
  "text": "自分のステージにいる『Liella!』のメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした場合、自分の控え室から『Liella!』のカードを1枚手札に加える。",
  "actions": []
}
```
**Fix**: Added parsing for "これにより～した場合" pattern in parser.py

### Issue 13: Compound conditions ✅ FIXED
**Location**: Line 904 and similar
**Problem**: Conditions with multiple requirements (location + group + distinct) only partially parsed
**Example**:
```json
"condition": {
  "text": "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる",
  "type": "distinct_condition",
  "distinct": "name"
  // Missing: location requirement ("エリアすべて"), group requirement ("蓮ノ空")
}
```
**Fix**: Moved compound condition check before distinct condition check in parser.py to ensure "かつ" splits all requirements

### Issue 14: Cost comparison conditions ✅ FIXED
**Location**: Line 1473 and similar
**Problem**: Baton touch conditions with cost comparison partially parsed
**Example**:
```json
"condition": {
  "text": "このメンバーよりコストが低い『スリーズブーケ』のメンバーからバトンタッチして登場した",
  "type": "baton_touch_condition",
  "cost_comparison": "lower"
  // Missing: source_group ("スリーズブーケ")
}
```
**Fix**: Added source_group extraction for baton touch conditions with cost comparison in parser.py

### Issue 15: Name-based matching conditions ✅ FIXED
**Location**: Line 1493 and similar
**Problem**: Conditions based on card/member names not captured
**Example**:
```json
"full_text": "手札を1枚控え室に置いてもよい：これにより控え室に置いたカードがメンバーカードの場合、控え室に置いたカードと同じ名前を持つメンバー1人は、ライブ終了時まで、{{heart_04.png|heart04}}{{icon_blade.png|ブレード}}を得る。"
```
**Fix**: Added name-based matching condition for patterns like "～と同じ名前を持つ" in parser.py

### Issue 16: Sequential effects with conditional follow-up ✅ FIXED
**Location**: Line 1848 and similar
**Problem**: Sequential actions with "そうした場合" (if so/then) conditional follow-up
**Example**:
```json
"full_text": "手札を2枚控え室に置いてもよい：自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、これによりアクティブにしたメンバーと、このメンバーは、それぞれ{{heart_04.png|heart04}}を得る。"
```
**Fix**: Improved "そうした場合" parsing to handle multiple targets with "それぞれ" in parser.py

### Issue 17: Parenthetical notes included in action text
**Location**: Line 376 and similar
**Problem**: Parenthetical notes are extracted but also included in select_action text
**Example**:
```json
"select_action": {
  "text": "好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）",
  "destination": "discard",
  "count": 1,
  "card_type": "member_card",
  "action": "move_cards"
}
```
**Fix**: Strip parenthetical notes from action text after extraction

### Issue 18: Generic per_unit placeholders
**Location**: Line 545 and similar
**Problem**: per_unit field contains generic placeholder instead of actual text
**Example**:
```json
"per_unit": "あるカード1枚",  // Should be "自分の成功ライブカード置き場にあるカード1枚"
```
**Fix**: Use actual text from condition instead of generic placeholder

### Issue 12: Ability invalidation with conditional follow-up not parsed
**Location**: Line 1232 and similar
**Problem**: Complex ability invalidation with "これにより～した場合" conditional follow-up not parsed
**Example**:
```json
"effect": {
  "text": "自分のステージにいる『Liella!』のメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした場合、自分の控え室から『Liella!』のカードを1枚手札に加える。",
  "actions": []
}
```
**Fix**: Parse "これにより～した場合" as conditional sequential action following ability invalidation

### Issue 13: Compound conditions not fully captured
**Location**: Line 904 and similar
**Problem**: Conditions with multiple requirements (location + group + distinct) only partially parsed
**Example**:
```json
"condition": {
  "text": "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる",
  "type": "distinct_condition",
  "distinct": "name"
  // Missing: location requirement ("エリアすべて"), group requirement ("蓮ノ空")
}
```
**Fix**: Parse compound conditions with "かつ" (and) to capture all requirements

### Issue 14: Cost comparison conditions not fully captured
**Location**: Line 1473 and similar
**Problem**: Baton touch conditions with cost comparison partially parsed
**Example**:
```json
"condition": {
  "text": "このメンバーよりコストが低い『スリーズブーケ』のメンバーからバトンタッチして登場した",
  "type": "baton_touch_condition",
  "cost_comparison": "lower"
  // Missing: source_group ("スリーズブーケ")
}
```
**Fix**: Extract source group from baton touch conditions with cost comparison

### Issue 15: Name-based matching conditions not captured
**Location**: Line 1493 and similar
**Problem**: Conditions based on card/member names not captured
**Example**:
```json
"full_text": "手札を1枚控え室に置いてもよい：これにより控え室に置いたカードがメンバーカードの場合、控え室に置いたカードと同じ名前を持つメンバー1人は、ライブ終了時まで、{{heart_04.png|heart04}}{{icon_blade.png|ブレード}}を得る。"
```
**Fix**: ✅ FIXED - Added name_match_condition type for patterns like "～と同じ名前を持つ"

### Issue 16: Sequential effects with conditional follow-up not fully parsed
**Location**: Line 1225 and similar
**Problem**: "そうした場合" (if so) conditional follow-up actions not parsed correctly for multiple targets
**Example**:
```json
"effect": {
  "text": "相手のステージにいるメンバー1人をウェイトにする。そうした場合、自分のデッキの上からカードを1枚引く。"
}
```
**Fix**: ✅ FIXED - Enhanced "そうした場合" parsing to handle "それぞれ" for multiple targets

### Issue 17: Parenthetical notes in action text not handled
**Location**: Line 1502 and similar
**Problem**: Parenthetical notes (e.g., "(このメンバーを今いるエリア...)") mixed into action text causing parsing errors
**Example**:
```json
"condition": {
  "except_target": "このメンバーをポジションチェンジしてもよい。(このメンバーを今いるエリア...",
  "type": "custom"
}
```
**Fix**: Add parenthetical note extraction to condition parsing before condition type determination

### Issue 18: Generic per_unit placeholders not resolved
**Location**: Various locations
**Problem**: Generic per_unit placeholders (e.g., "につき") not properly parsed into specific counts
**Example**:
```json
"per_unit": true
```
**Fix**: Add per_unit parsing to extract actual count values

### Issue 19: Location count conditions not classified
**Location**: Line 803 and similar
**Problem**: Conditions like "自分のライブ中のカードが3枚以上" have target, count, operator but type "custom"
**Example**:
```json
{
  "text": "自分のライブ中のカードが3枚以上",
  "target": "self",
  "count": 3,
  "operator": ">=",
  "type": "custom"
}
```
**Fix**: Add location count condition classification for patterns with target + count + operator

### Issue 20: Score threshold conditions not classified
**Location**: Line 2070 and similar
**Problem**: Conditions like "合計が25の" have aggregate but type "custom"
**Example**:
```json
{
  "text": "{{icon_all.png|ハート}}を得る。合計が25の",
  "aggregate": "total",
  "type": "custom"
}
```
**Fix**: Add score threshold condition classification for patterns with aggregate + total

### Issue 21: Parenthetical text in except conditions not handled
**Location**: Line 2683 and similar
**Problem**: Parenthetical text incorrectly parsed as except_target
**Example**:
```json
{
  "except_target": "このメンバーをポジションチェンジしてもよい。(このメンバーを今いるエリア",
  "type": "custom"
}
```
**Fix**: Strip parenthetical notes from except_target before classification

### Issue 22: Member appearance conditions not classified
**Location**: Line 2710 and similar
**Problem**: Conditions like "このメンバーが登場" have card_type but type "custom"
**Example**:
```json
{
  "text": "このメンバーが登場",
  "card_type": "member_card",
  "type": "custom"
}
```
**Fix**: ✅ FIXED - Added appearance condition classification for patterns like "～が登場"

### Issue 23: Energy state conditions not fully classified
**Location**: Line 16280
**Problem**: Conditions like "アクティブ状態の自分のエネルギーがある" have target but type "custom"
**Example**:
```json
{
  "text": "アクティブ状態の自分のエネルギーがある",
  "target": "self",
  "type": "custom"
}
```
**Fix**: ✅ FIXED - Moved energy state check before state condition check to ensure proper classification for patterns with "エネルギーがある"

### Summary
✅ ALL CUSTOM TYPE INSTANCES ELIMINATED

The parser now correctly classifies:
- Compound conditions (かつ, あり、)
- Name-based matching conditions (～と同じ名前を持つ)
- Sequential effects with conditional follow-up (そうした場合)
- Ability invalidation with conditional follow-up (これにより～した場合)
- Cost comparison conditions (バトンタッチ with cost comparison)
- Location count conditions (target + count + operator)
- Score threshold conditions (aggregate + total)
- Movement count conditions (movement + count)
- Action restriction conditions (except + card_type)
- Card count conditions (card_type + count)
- Except count conditions (except + count)
- Appearance conditions (登場)
- Temporal conditions (ターン目)
- State transition conditions (から～なった)
- Parenthetical text stripping in except conditions
- Character presence conditions with multiple characters (と, か)
- State conditions with energy (アクティブ状態のエネルギーがある)
- Energy state conditions (エネルギーがある)
- Distinct conditions for unit names (ユニット名がそれぞれ異なる)
- Yell action conditions (エールした)
- Position conditions

Status: **0 custom type instances remaining** in abilities.json

### Issue 16: Sequential effects with conditional follow-up
**Location**: Line 1848 and similar
**Problem**: Sequential actions with "そうした場合" (if so/then) conditional follow-up
**Example**:
```json
"full_text": "手札を2枚控え室に置いてもよい：自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、これによりアクティブにしたメンバーと、このメンバーは、それぞれ{{heart_04.png|heart04}}を得る。"
```
**Fix**: Parse "そうした場合" as conditional sequential action pattern

### Issue 17: Parenthetical notes included in action text
**Location**: Line 376 and similar
**Problem**: Parenthetical notes are extracted but also included in select_action text
**Example**:
```json
"select_action": {
  "text": "好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）",
  "destination": "discard",
  "count": 1,
  "card_type": "member_card",
  "action": "move_cards"
}
```
**Fix**: Strip parenthetical notes from action text after extraction

### Issue 18: Generic per_unit placeholders
**Location**: Line 545 and similar
**Problem**: per_unit field contains generic placeholder instead of actual text
**Example**:
```json
"per_unit": "あるカード1枚",  // Should be "自分の成功ライブカード置き場にあるカード1枚"
```
**Fix**: Use actual text from condition instead of generic placeholder
