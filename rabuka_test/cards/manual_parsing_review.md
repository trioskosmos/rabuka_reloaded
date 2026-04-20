# Manual Parsing Review - Card Ability Texts vs Parsed Outputs

## Issues Found

### Issue 1: "好きな枚数を好きな順番でデッキの上に置き" not correctly parsed
**Location**: Lines 277-310, 340-382
**Original Text**: "自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。"
**Parsed Output**:
```json
{
  "select_action": {
    "text": "好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く",
    "destination": "discard",
    "count": 1,
    "card_type": "member_card",
    "action": "move_cards"
  }
}
```
**Problem**: 
- Destination is set to "discard" but should be "deck_top" for the first action
- The sequential nature (put some on deck, then discard rest) is not captured
- Should be parsed as sequential actions or a more complex structure
- Missing the "好きな枚数" (any number) aspect

**Severity**: High - incorrect parsing of card placement logic

---

### Issue 2: Choice effect with per_unit not capturing selection
**Location**: Lines 527-549
**Original Text**: "{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る。"
**Parsed Output**:
```json
{
  "effect": {
    "text": "...",
    "per_unit": "あるカード1枚",
    "card_type": "live_card",
    "target": "self",
    "action": "gain_resource",
    "resource": "heart"
  }
}
```
**Problem**:
- per_unit is truncated to "あるカード1枚" instead of full "自分の成功ライブカード置き場にあるカード1枚"
- The choice aspect (selecting which heart) is not captured in the parsed structure
- Should have a choice/selection structure for the heart selection

**Severity**: Medium - functional but missing information

---

### Issue 3: Parenthetical rules marked as null abilities
**Location**: Lines 252-276
**Original Text**: "(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)"
**Parsed Output**:
```json
{
  "is_null": true,
  "cost": null,
  "effect": {
    "text": "",
    "action": "custom"
  }
}
```
**Problem**:
- These are game rule clarifications, not abilities
- Currently marked as is_null: true with custom action
- Should have a separate classification like "rule_clarification" or be filtered out entirely

**Severity**: Low - correctly marked as null, but could be better classified

---

### Issue 4: Activation restriction split incorrectly
**Location**: Lines 688-726
**Original Text**: "このカードを控え室からステージに登場させる。この能力は、このカードが控え室にある場合のみ起動できる"
**Parsed Output**:
```json
{
  "actions": [
    {
      "text": "このカードを控え室からステージに登場させる。この能力は",
      "source": "discard",
      "destination": "stage",
      "action": "move_cards"
    },
    {
      "text": "このカードが控え室にある場合のみ起動できる",
      "source": "discard",
      "action": "activation_restriction",
      "restriction_type": "only"
    }
  ]
}
```
**Problem**:
- The activation restriction is incorrectly parsed as a sequential action
- It should be a separate field on the ability (e.g., `activation_condition`)
- The first action's text incorrectly includes "この能力は"

**Severity**: Medium - functional but structurally incorrect

---

### Issue 5: Heart color selection not captured
**Location**: Lines 727-751
**Original Text**: "好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る。"
**Parsed Output**:
```json
{
  "text": "好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る",
  "action": "gain_resource",
  "resource": "heart"
}
```
**Problem**:
- The "好きなハートの色を1つ指定する" (choose any heart color) aspect is not captured
- Should have a selection/choice structure
- Missing which heart was selected

**Severity**: Medium - functional but missing player choice information

---

## Review Notes

### Correctly Parsed Examples
- Simple move cards abilities (lines 13-109)
- Sequential actions with clear separators (lines 171-215, 490-525)
- Energy costs with effects (lines 217-251, 588-615)
- State changes with cost limits (lines 384-417, 666-687)
- Group-based card retrieval (lines 451-489)
- Compound conditions (lines 785-828)
- Position conditions (lines 830-844)

### Pattern Recognition
- Parser handles simple sequential actions well
- Parenthetical notes are captured but not always processed
- Choice effects need better structure
- Complex placement logic (deck top vs discard) needs improvement
- Activation restrictions are sometimes parsed as sequential actions instead of conditions
