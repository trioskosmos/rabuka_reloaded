# Parser Issues - Round 2 Review

This document lists parsing issues found during a second review of the abilities.json file after the initial fixes.

## Format
For each issue:
- **Full Text**: The original ability text
- **Issue**: Description of what's missing or incorrect
- **Parsed Output**: The current parsed JSON
- **Missing/Incorrect**: What should be captured but isn't

---

### 1. "好きな枚数を好きな順番でデッキの上に置き" parsed as custom action
**Full Text**: "自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。"

**Issue**: The action "好きな枚数を好きな順番でデッキの上に置き" is parsed as `"action": "custom"` instead of being parsed as a move_cards action.

**Parsed Output**:
```json
{
  "text": "好きな枚数を好きな順番でデッキの上に置き",
  "placement_order": "any_order",
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "好きな枚数を好きな順番でデッキの上に置き",
  "action": "move_cards",
  "destination": "deck_top",
  "placement_order": "any_order"
}
```

---

### 2. "手札のライブカードを1枚公開し" parsed as custom cost
**Full Text**: "手札のライブカードを1枚公開し、デッキの一番下に置いてもよい"

**Issue**: The cost action "手札のライブカードを1枚公開し" is parsed as `"type": "custom"` instead of being parsed as a reveal action.

**Parsed Output**:
```json
{
  "text": "手札のライブカードを1枚公開しし",
  "source": "hand",
  "count": 1,
  "card_type": "live_card",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "手札のライブカードを1枚公開し",
  "source": "hand",
  "count": 1,
  "card_type": "live_card",
  "type": "reveal"
}
```

---

### 3. "デッキの一番下に置いてもよい" parsed as custom cost
**Full Text**: "手札のライブカードを1枚公開し、デッキの一番下に置いてもよい"

**Issue**: The cost action "デッキの一番下に置いてもよい" is parsed as `"type": "custom"` instead of being parsed as a move_cards action.

**Parsed Output**:
```json
{
  "text": "デッキの一番下に置いてもよい",
  "destination": "deck_bottom",
  "optional": true,
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "デッキの一番下に置いてもよい",
  "action": "move_cards",
  "destination": "deck_bottom",
  "optional": true
}
```

---

### 4. "このメンバーをウェイトにし" parsed as custom cost
**Full Text**: "{{center.png|センター}}このメンバーをウェイトにし、手札を1枚控え室に置く"

**Issue**: The cost action "このメンバーをウェイトにし" is parsed as `"type": "custom"` instead of being parsed as a change_state action.

**Parsed Output**:
```json
{
  "text": "{{center.png|センター}}このメンバーをウェイトにしし",
  "card_type": "member_card",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "このメンバーをウェイトにし",
  "action": "change_state",
  "state": "wait",
  "card_type": "member_card",
  "position": "center"
}
```

---

### 5. "手札にあるメンバーカードを好きな枚数公開する" missing reveal action
**Full Text**: "手札にあるメンバーカードを好きな枚数公開する：公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。"

**Issue**: The cost "手札にあるメンバーカードを好きな枚数公開する" is parsed as `"type": "move_cards"` instead of being parsed as a reveal action.

**Parsed Output**:
```json
{
  "text": "手札にあるメンバーカードを好きな枚数公開する",
  "source": "hand",
  "destination": "hand",
  "card_type": "member_card",
  "type": "move_cards"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "手札にあるメンバーカードを好きな枚数公開する",
  "action": "reveal",
  "source": "hand",
  "card_type": "member_card",
  "count": "any"
}
```

---

### 6. "公開して手札に加えてもよい" missing reveal action
**Full Text**: "自分のデッキの上からカードを7枚見る。その中から{{heart_02.png|heart02}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}を持つメンバーカードを3枚まで公開して手札に加えてもよい。残りを控え室に置く。"

**Issue**: The action "公開して手札に加えてもよい" is parsed but may not properly capture the reveal aspect.

**Parsed Output**:
```json
{
  "text": "その中から{{heart_02.png|heart02}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}を持つメンバーカードを3枚まで公開して手札に加えてもよい。残りを控え室に置く",
  "action": "look_and_select",
  ...
}
```

**Missing/Incorrect**: Should capture the reveal action and the "3枚まで" (up to 3) limit more explicitly.

---

### 7. Parenthetical "ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない" not parsed as structured field
**Full Text**: "（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）"

**Issue**: This parenthetical is captured as text but not parsed as a structured field indicating a restriction on cheer reveal.

**Parsed Output**:
```json
"parenthetical": [
  "ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"
]
```

**Missing/Incorrect**: Should be:
```json
"restriction": {
  "type": "cheer_reveal_limit",
  "description": "ウェイト状態のメンバーが持つブレードは、エールで公開する枚数を増やさない"
}
```

---

### 8. "好きな枚数" not captured as variable count
**Full Text**: "自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。"

**Issue**: The phrase "好きな枚数" (any number) is not captured as a specific count field indicating a variable number.

**Parsed Output**:
```json
{
  "text": "好きな枚数を好きな順番でデッキの上に置き",
  "placement_order": "any_order",
  "action": "custom"
}
```

**Missing/Incorrect**: Should include `"count": "variable"` or `"count": "any"` to indicate the player can choose any number.

---

### 9. "各グループ名につき1枚ずつ" per-group pattern incorrectly captured
**Full Text**: "自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く。"

**Issue**: The phrase "各グループ名につき1枚ずつ" (1 per group name) is being captured as a general per_unit pattern instead of a specific per-group selection pattern.

**Parsed Output**:
```json
{
  "per_unit": "自分のデッキの上からカードを5枚見る。その中から各グループ名",
  "per_unit_count": 5,
  "per_unit_type": "card",
  ...
}
```

**Missing/Incorrect**: Should capture per-group selection pattern like:
```json
{
  "per_group": true,
  "per_group_count": 1,
  "per_group_type": "group_name"
}
```

---

### 10. "1枚ずつ公開し" parsed as custom action instead of reveal
**Full Text**: "その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く"

**Issue**: The action "1枚ずつ公開し" is parsed as `"action": "custom"` instead of being parsed as a reveal action.

**Parsed Output**:
```json
{
  "text": "1枚ずつ公開し",
  "count": 1,
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "1枚ずつ公開し",
  "action": "reveal",
  "count": 1
}
```

---

### 11. Position change condition with parenthetical not fully parsed
**Full Text**: "このメンバーはセンターエリア以外にポジションチェンジする。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)"

**Issue**: The position change condition with parenthetical explanation is not fully parsed. The parenthetical contains important rules about what happens when the target area has a member.

**Parsed Output**:
```json
{
  "text": "このメンバーはセンターエリア以外にポジションチェンジする。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる",
  "type": "position_condition",
  "position": "center"
}
```

**Missing/Incorrect**: Should capture the parenthetical rules about member swapping when target area is occupied.

---

### 12. "元々持つ{{icon_blade.png|ブレード}}の数" not captured as original blade count
**Full Text**: "このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のメンバー1人をウェイトにする。"

**Issue**: The phrase "元々持つ{{icon_blade.png|ブレード}}の数" (original blade count) is not captured as a specific field indicating it refers to the original blade count, not the current blade count.

**Parsed Output**: (Need to verify)

**Missing/Incorrect**: Should capture `"blade_count_type": "original"` or similar to distinguish from current blade count.

---

### 13. "メンバーのいないエリア" not captured as empty_area
**Full Text**: "自分の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアに登場させる"

**Issue**: The phrase "メンバーのいないエリア" (area with no members) is not captured as a specific destination type.

**Parsed Output**: (Need to verify)

**Missing/Incorrect**: Should capture `"destination": "empty_area"` or `"destination_condition": {"empty": true}`.

---

### 14. "枚以上ある" count conditions parsed as custom
**Full Text**: "2枚以上ある" / "6枚以上ある"

**Issue**: Count conditions like "2枚以上ある" are parsed as `"type": "custom"` instead of being parsed as proper count conditions with operator ">=".

**Parsed Output**:
```json
{
  "text": "2枚以上ある",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "2枚以上ある",
  "type": "count_condition",
  "count": 2,
  "operator": ">="
}
```

---

### 15. "このターンに～移動していないかぎり" temporal conditions parsed as custom
**Full Text**: "このターンにこのメンバーが移動していないかぎり"

**Issue**: Temporal conditions like "このターンに～移動していないかぎり" are parsed as `"type": "custom"` instead of being parsed as temporal conditions.

**Parsed Output**:
```json
{
  "text": "このターンにこのメンバーが移動していないかぎり",
  "card_type": "member_card",
  "temporal": "this_turn",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "このターンにこのメンバーが移動していないかぎり",
  "type": "temporal_condition",
  "temporal": "this_turn",
  "condition": {
    "type": "not_moved",
    "card_type": "member_card"
  }
}
```

---

### 16. "それらが両方ある" conditions parsed as custom
**Full Text**: "それらが両方ある"

**Issue**: Conditions like "それらが両方ある" are parsed as `"type": "custom"` instead of being parsed as "both" conditions.

**Parsed Output**:
```json
{
  "text": "それらが両方ある",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "それらが両方ある",
  "type": "both_condition"
}
```

---

### 17. "手札の枚数が3枚になるまで手札を控え室に置き" parsed as custom
**Full Text**: "自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き"

**Issue**: Discard until count actions like "手札の枚数が3枚になるまで手札を控え室に置き" are parsed as `"action": "custom"` instead of being parsed as "discard_until_count".

**Parsed Output**:
```json
{
  "text": "自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き",
  "count": 3,
  "card_type": "member_card",
  "target": "both",
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き",
  "action": "discard_until_count",
  "target_count": 3,
  "source": "hand",
  "destination": "discard",
  "target": "both"
}
```

---

### 18. "E支払ってもよい" energy costs parsed as custom
**Full Text**: "{{icon_energy.png|E}}支払ってもよい"

**Issue**: Energy payment costs like "{{icon_energy.png|E}}支払ってもよい" are parsed as `"action": "custom"` instead of being parsed as "pay_energy" costs.

**Parsed Output**:
```json
{
  "text": "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。",
  "count": 1,
  "resource_icon_count": 1,
  "target": "self",
  "optional": true,
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。",
  "type": "pay_energy",
  "energy": 1,
  "optional": true,
  "target": "self"
}
```

---

### 19. "ステージに登場させてもよい" parsed as custom instead of appear
**Full Text**: "自分の手札からコスト4以下のメンバーカードを1枚ステージに登場させてもよい"

**Issue**: Appearance effects like "ステージに登場させてもよい" are parsed as `"action": "custom"` instead of being parsed as "appear" action.

**Parsed Output**:
```json
{
  "text": "自分の手札からコスト4以下のメンバーカードを1枚ステージに登場させてもよい",
  "condition": {
    "text": "このメンバーよりコストが低いメンバーからバトンタッチして登場した",
    "type": "baton_touch_condition",
    "cost_comparison": "lower"
  },
  "cost_limit": 4,
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "optional": true,
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "自分の手札からコスト4以下のメンバーカードを1枚ステージに登場させてもよい",
  "action": "appear",
  "source": "hand",
  "destination": "stage",
  "count": 1,
  "card_type": "member_card",
  "cost_limit": 4,
  "target": "self",
  "optional": true
}
```

---

### 20. "このターン、相手が余剰のハートを持たずにライブを成功させていた" temporal condition parsed as custom
**Full Text**: "このターン、相手が余剰のハートを持たずにライブを成功させていた"

**Issue**: Temporal conditions about opponent's live success are parsed as `"type": "custom"`.

**Parsed Output**:
```json
{
  "text": "このターン、相手が余剰のハートを持たずにライブを成功させていた",
  "temporal": "this_turn",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "このターン、相手が余剰のハートを持たずにライブを成功させていた",
  "type": "temporal_condition",
  "temporal": "this_turn",
  "condition": {
    "type": "opponent_live_success",
    "no_excess_heart": true
  }
}
```

---

### 21. "このゲームの1ターン目のライブフェイズの" temporal condition parsed as custom
**Full Text**: "このゲームの1ターン目のライブフェイズの"

**Issue**: Temporal conditions about specific turn phases are parsed as `"type": "custom"`.

**Parsed Output**:
```json
{
  "text": "このゲームの1ターン目のライブフェイズの",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "このゲームの1ターン目のライブフェイズの",
  "type": "temporal_condition",
  "turn": 1,
  "phase": "live_phase"
}
```

---

### 22. "このターン、このメンバーがエリアを移動している" temporal condition parsed as custom
**Full Text**: "このターン、このメンバーがエリアを移動している"

**Issue**: Temporal conditions about member movement are parsed as `"type": "custom"` even though they have movement_state extracted.

**Parsed Output**:
```json
{
  "text": "このターン、このメンバーがエリアを移動している",
  "card_type": "member_card",
  "movement_state": "has_moved",
  "temporal": "this_turn",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "このターン、このメンバーがエリアを移動している",
  "type": "temporal_condition",
  "temporal": "this_turn",
  "condition": {
    "type": "has_moved",
    "card_type": "member_card"
  }
}
```

---

### 23. "2人以上いる" count condition parsed as custom
**Full Text**: "2人以上いる"

**Issue**: Count conditions like "2人以上いる" are parsed as `"type": "custom"` instead of being parsed as count conditions.

**Parsed Output**:
```json
{
  "text": "2人以上いる",
  "count": 2,
  "operator": ">=",
  "type": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "2人以上いる",
  "type": "count_condition",
  "count": 2,
  "operator": ">=",
  "unit": "人"
}
```

---

### 24. "それらのメンバーのユニット名がそれぞれ異なる" distinct condition parsed as custom
**Full Text**: "それらのメンバーのユニット名がそれぞれ異なる"

**Issue**: Distinct conditions like "ユニット名がそれぞれ異なる" are parsed as `"type": "custom"` even though they have distinct field extracted.

**Parsed Output**:
```json
{
  "text": "それらのメンバーのユニット名がそれぞれ異なる",
  "type": "custom",
  "distinct": "unit_name",
  "card_type": "member_card"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "それらのメンバーのユニット名がそれぞれ異なる",
  "type": "distinct_condition",
  "distinct": "unit_name",
  "card_type": "member_card"
}
```

---

### 25. "～を登場させる" parsed as custom instead of appear
**Full Text**: "自分の控え室からコスト15以下の『蓮ノ空』のメンバーカードを1枚、このメンバーがいたエリアに登場させる"

**Issue**: Appearance effects like "～を登場させる" are parsed as `"action": "custom"` instead of being parsed as "appear" action.

**Parsed Output**:
```json
{
  "text": "自分の控え室からコスト15以下の『蓮ノ空』のメンバーカードを1枚、このメンバーがいたエリアに登場させる",
  "cost_limit": 15,
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "group": {
    "name": "蓮ノ空"
  },
  "group_names": [
    "蓮ノ空"
  ],
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "自分の控え室からコスト15以下の『蓮ノ空』のメンバーカードを1枚、このメンバーがいたエリアに登場させる",
  "action": "appear",
  "source": "discard",
  "destination": "stage",
  "count": 1,
  "card_type": "member_card",
  "cost_limit": 15,
  "target": "self",
  "group": {
    "name": "蓮ノ空"
  }
}
```

---

### 26. "コストを＋4する" parsed as custom instead of modify_cost
**Full Text**: "ステージにいるこのメンバーのコストを＋4する"

**Issue**: Cost modification effects like "コストを＋4する" are parsed as `"action": "custom"` instead of being parsed as "modify_cost" action.

**Parsed Output**:
```json
{
  "text": "ステージにいるこのメンバーのコストを＋4する",
  "condition": {
    "text": "自分のエネルギーが10枚以上ある",
    "type": "count_condition",
    "count": 10,
    "operator": ">="
  },
  "card_type": "member_card",
  "action": "custom"
}
```

**Missing/Incorrect**: Should be:
```json
{
  "text": "ステージにいるこのメンバーのコストを＋4する",
  "action": "modify_cost",
  "operation": "add",
  "value": 4,
  "card_type": "member_card"
}
```

---

### 27. "ブレードハートを失い、もう一度エールを行う" parsed as custom
**Full Text**: "そのエールで得たブレードハートを失い、もう一度エールを行う"

**Issue**: Complex actions involving losing blade hearts and re-doing yell are parsed as `"action": "custom"`.

**Parsed Output**:
```json
{
  "text": "そのエールで得たブレードハートを失い、もう一度エールを行う",
  "condition": {
    "text": "［ターン1回］エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた",
    "type": "yell_revealed_condition",
    "source": "yell_revealed"
  },
  "action": "custom"
}
```

**Missing/Incorrect**: Should be parsed as sequential action with lose_resource and repeat_yell actions.

---
