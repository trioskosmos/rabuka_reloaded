# Missing Parsed Information in Abilities

This document lists parts of abilities mentioned in `full_text` that are not captured in the parsed `effect`/`cost` structures.

## Issues Found

### 1. Incomplete per_unit specification (Line ~546)
**Full Text:** "{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る。"

**Issue:** The `per_unit` field says "あるカード1枚" which is incomplete. It should specify the location "成功ライブカード置き場にある" (in the success live card zone).

**Parsed Output:**
```json
"per_unit": "あるカード1枚",
"card_type": "live_card",
"target": "self",
```

**Missing:** Location specification "成功ライブカード置き場にある"

---

### 2. Choice aspect not captured (Line ~741)
**Full Text:** "好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る。"

**Issue:** The effect has `resource: "heart"` but doesn't capture the choice aspect - which heart color to choose.

**Parsed Output:**
```json
"action": "gain_resource",
"resource": "heart"
```

**Missing:** The choice mechanism ("好きなハートの色を1つ指定する" - choose any heart color)

---

### 3. Distinct names condition not captured (Line ~973)
**Full Text:** "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合"

**Issue:** The condition type is `appearance_condition` with `appearance: true`, but it doesn't capture the "名前が異なる" (distinct names) aspect.

**Parsed Output:**
```json
"condition": {
  "text": "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる",
  "type": "appearance_condition",
  "appearance": true
}
```

**Missing:** The "distinct names" constraint

---

### 4. Cost limit not captured (Line ~1004)
**Full Text:** "自分の控え室から4コスト以下の『蓮ノ空』のメンバーカードを1枚手札に加える。"

**Issue:** The effect has a group filter but no `cost_limit` field to capture "4コスト以下".

**Parsed Output:**
```json
"group": {
  "name": "蓮ノ空"
},
"action": "move_cards"
```

**Missing:** `cost_limit: 4` field

---

### 5. Choice aspect not captured (Line ~1100)
**Full Text:** "好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る。"

**Issue:** Same as #2 - the choice aspect is not captured.

**Parsed Output:**
```json
"action": "gain_resource",
"resource": "heart"
```

**Missing:** The choice mechanism

---

### 6. Heart filter criteria not captured (Line ~1135)
**Full Text:** "その中から{{heart_02.png|heart02}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}を持つメンバーカードを3枚まで公開して手札に加えてもよい。"

**Issue:** The select_action doesn't capture the heart filter criteria - it needs to filter for cards with heart02, heart04, or heart05.

**Parsed Output:**
```json
"select_action": {
  "destination": "discard",
  "count": 3,
  "card_type": "member_card",
  "optional": true,
  "max": true,
  "action": "move_cards"
}
```

**Missing:** Filter criteria for cards with specific hearts (heart02, heart04, or heart05)

---

### 7. Turn limit not captured (Line ~1177)
**Full Text:** "{{jidou.png|自動}}［ターン1回］エールにより公開された自分のカードの中にライブカードが1枚以上あるとき..."

**Issue:** The `use_limit` is null but the text says "［ターン1回］" which should be captured as `use_limit: 1`.

**Parsed Output:**
```json
"triggers": "自動",
"use_limit": null,
```

**Missing:** `use_limit: 1` from the ［ターン1回］ text

---

### 8. Complex condition not fully parsed (Line ~1193)
**Full Text:** "［ターン1回］エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合"

**Issue:** The condition combines multiple aspects that aren't fully captured:
1. Turn limit ［ターン1回］
2. Yell-revealed cards containing live cards
3. Hand size condition (7 cards or less)

**Parsed Output:**
```json
"condition": {
  "text": "［ターン1回］エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の",
  "type": "yell_revealed_condition",
  "source": "yell_revealed"
}
```

**Missing:** 
- Turn limit specification
- Hand size condition (7枚以下)
- The live card count condition within yell-revealed cards

---

### 9. Custom action placeholder (Line ~1239)
**Full Text:** "好きな枚数を好きな順番でデッキの上に置き"

**Issue:** Marked as "action": "custom" which is a placeholder for unhandled logic.

**Parsed Output:**
```json
{
  "text": "好きな枚数を好きな順番でデッキの上に置き",
  "action": "custom"
}
```

**Missing:** Proper parsing of the "any number in any order" placement logic

---

### 10. Heart resource amount not specified (Line ~246)
**Full Text:** "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"

**Issue:** The effect has `resource: "blade"` but doesn't specify the amount (2 blades).

**Parsed Output:**
```json
"action": "gain_resource",
"resource": "blade"
```

**Missing:** Resource amount field (should be `count: 2` or similar)

---

### 11. Ability invalidation not captured (Line ~1311)
**Full Text:** "自分のステージにいる『Liella!』のメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした場合、自分の控え室から『Liella!』のカードを1枚手札に加える。"

**Issue:** The condition mentions invalidating abilities but the parsed output doesn't capture the ability invalidation aspect - it only captures the group condition.

**Parsed Output:**
```json
"condition": {
  "text": "自分のステージにいる『Liella!』のメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした",
  "type": "group_condition"
}
```

**Missing:** The ability invalidation mechanism and the conditional effect ("これにより無効にした場合")

---

### 12. Cost limit not captured in cost (Line ~1385)
**Full Text:** "手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く"

**Issue:** The cost doesn't have a `cost_limit` field to capture "コスト4以下".

**Parsed Output:**
```json
"cost": {
  "source": "hand",
  "destination": "discard",
  "count": 1,
  "card_type": "member_card",
  "group": {
    "name": "Liella!"
  },
  "type": "move_cards"
}
```

**Missing:** `cost_limit: 4` field in the cost structure

---

### 13. Resource amount not specified (Line ~1523)
**Full Text:** "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"

**Issue:** The effect has `resource: "blade"` but doesn't specify the amount (3 blades).

**Parsed Output:**
```json
"action": "gain_resource",
"resource": "blade"
```

**Missing:** Resource amount field (should be `count: 3`)

---

### 14. Comparison operator not captured (Line ~1523)
**Full Text:** "このメンバーよりコストの大きいメンバーがいる場合"

**Issue:** The condition has `comparison_type: "cost"` but doesn't capture the "greater than" operator.

**Parsed Output:**
```json
"condition": {
  "text": "自分のステージに、このメンバーよりコストの大きいメンバーがいる",
  "comparison_type": "cost",
  "type": "location_condition"
}
```

**Missing:** The comparison operator (should be `operator: ">"`)

---

### 15. Area specification not captured (Line ~1595)
**Full Text:** "自分のステージのエリアすべてにメンバーが登場している場合"

**Issue:** The condition doesn't capture "エリアすべて" (all areas) specification.

**Parsed Output:**
```json
"condition": {
  "text": "自分のステージのエリアすべてにメンバーが登場している",
  "type": "appearance_condition",
  "appearance": true
}
```

**Missing:** The "all areas" constraint specification

### 16. Per-unit count not captured (Line ~1694)
**Full Text:** "自分のステージにいるメンバー1人につき、カードを1枚引く。その後、手札を1枚控え室に置く。"

**Issue:** The effect has a sequential action but doesn't capture the "per member" scaling aspect.

**Parsed Output:**
```json
"effect": {
  "action": "sequential",
  "actions": [
    {
      "count": 1,
      "action": "draw"
    },
    {
      "count": 1,
      "action": "move_cards"
    }
  ]
}
```

**Missing:** Per-unit scaling ("自分のステージにいるメンバー1人につき" - per member on stage)

---

### 17. Conditional sequential not properly captured (Line ~1729)
**Full Text:** "自分の成功ライブカード置き場にカードがある場合、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『μ's』のライブカードを1枚手札に加える。"

**Issue:** The conditional aspect ("そうした場合" - in that case/if you do) is marked as `conditional: true` but the condition for the second action is not properly linked to the first action's execution.

**Parsed Output:**
```json
"action": "sequential",
"actions": [
  {
    "text": "自分の成功ライブカード置き場にカードがある場合、手札を1枚控え室に置いてもよい。",
    "optional": true,
    "action": "move_cards"
  },
  {
    "text": "自分の控え室から『μ's』のライブカードを1枚手札に加える",
    "action": "move_cards"
  }
],
"conditional": true
```

**Missing:** The conditional dependency between actions (second action only if first action is taken)

---

### 18. Group filter not captured in baton touch (Line ~1626)
**Full Text:** "このメンバーよりコストが低い『スリーズブーケ』のメンバーからバトンタッチして登場した場合"

**Issue:** The baton_touch_condition has `cost_comparison: "lower"` but doesn't capture the group filter "『スリーズブーケ』".

**Parsed Output:**
```json
"condition": {
  "text": "このメンバーよりコストが低い『スリーズブーケ』のメンバーからバトンタッチして登場した",
  "type": "baton_touch_condition",
  "cost_comparison": "lower"
}
```

**Missing:** Group filter for "スリーズブーケ"

---

### 19. Name matching condition not captured (Line ~1661)
**Full Text:** "控え室に置いたカードと同じ名前を持つメンバー1人は"

**Issue:** The condition checks if a card has the same name as the discarded card, but this name matching logic is not captured.

**Parsed Output:**
```json
"condition": {
  "text": "これにより控え室に置いたカードがメンバーカードの",
  "location": "discard",
  "card_type": "member_card",
  "type": "location_condition"
}
```

**Missing:** The name matching constraint ("同じ名前を持つ")

---

### 20. Multiple resource types not captured (Line ~1661)
**Full Text:** "{{heart_04.png|heart04}}{{icon_blade.png|ブレード}}を得る"

**Issue:** The effect has `resource: "blade"` but doesn't capture that it gains both heart04 AND blade.

**Parsed Output:**
```json
"action": "gain_resource",
"resource": "blade"
```

**Missing:** The second resource type (heart04)

---

### 21. Location condition not captured in sequential action (Line ~1729)
**Full Text:** "自分の成功ライブカード置き場にカードがある場合"

**Issue:** The first action in the sequential has a location condition, but it's not captured as a condition field within the action.

**Parsed Output:**
```json
{
  "text": "自分の成功ライブカード置き場にカードがある場合、手札を1枚控え室に置いてもよい。",
  "destination": "discard",
  "count": 1,
  "card_type": "live_card",
  "optional": true,
  "action": "move_cards"
}
```

**Missing:** The location condition for the first action

---

### 22. Dynamic cost calculation not captured (Line ~1996)
**Full Text:** "そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚"

**Issue:** The cost is calculated dynamically (cost + 2) but this calculation is not captured in the parsed output.

**Parsed Output:**
```json
{
  "text": "自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる。",
  "group": {
    "name": "Aqours"
  },
  "action": "move_cards"
}
```

**Missing:** The dynamic cost calculation (cost + 2)

---

### 23. Cost missing group filter (Line ~1996)
**Full Text:** "このメンバー以外の『Aqours』のメンバー1人をウェイトにし、手札を1枚控え室に置く"

**Issue:** The cost has "このメンバー以外の『Aqours』のメンバー" but the parsed cost doesn't capture the group filter or the exclude_self aspect.

**Parsed Output:**
```json
"cost": {
  "text": "{{center.png|センター}}このメンバーをウェイトにし、手札を1枚控え室に置く",
  "source": "hand",
  "destination": "discard",
  "count": 1,
  "card_type": "member_card",
  "type": "move_cards"
}
```

**Missing:** Group filter for "Aqours" and exclude_self for "このメンバー以外"

---

### 24. Second action incomplete in sequential (Line ~2060)
**Full Text:** "カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。"

**Issue:** The second action in the sequential only captures the draw, but misses the gain_resource effect that happens simultaneously.

**Parsed Output:**
```json
{
  "text": "カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。",
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "action": "draw",
  "source": "deck",
  "destination": "hand"
}
```

**Missing:** The gain_resource effect (2 blades to all members until live end)

---

### 25. Draw until condition not captured (Line ~2106)
**Full Text:** "手札が5枚になるまでカードを引く"

**Issue:** The effect has `count: 5` but this is misleading - it should be a "draw until" condition, not drawing exactly 5 cards.

**Parsed Output:**
```json
"effect": {
  "text": "手札が5枚になるまでカードを引く",
  "count": 5,
  "action": "draw",
  "source": "deck",
  "destination": "hand"
}
```

**Missing:** The "draw until" condition (should draw until hand size reaches 5, not draw exactly 5 cards)

---

### 26. Temporal condition not captured (Line ~2106)
**Full Text:** "このターン、自分のステージにメンバーが3回登場したとき"

**Issue:** The condition mentions "this turn" and "3 times" but these temporal and count aspects are not captured.

**Parsed Output:**
```json
"condition": {
  "text": "このターン、自分のステージにメンバーが3回登場した",
  "type": "appearance_condition",
  "appearance": true
}
```

**Missing:** Temporal condition ("this turn") and count threshold (3 times)

---

### 27. Cost missing exclude_self (Line ~2165)
**Full Text:** "このメンバー以外の『虹ヶ咲』のメンバー1人をウェイトにする"

**Issue:** The cost has "このメンバー以外" but the exclude_self aspect is not captured.

**Parsed Output:**
```json
"cost": {
  "text": "このメンバー以外の『虹ヶ咲』のメンバー1人をウェイトにする",
  "state_change": "wait",
  "type": "change_state",
  "count": 1,
  "card_type": "member_card",
  "group": {
    "name": "虹ヶ咲"
  }
}
```

**Missing:** exclude_self field

---

### 28. Target state condition not captured (Line ~2196)
**Full Text:** "自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする"

**Issue:** The target must be in "wait" state but this state condition is not captured in the parsed output.

**Parsed Output:**
```json
{
  "text": "自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。",
  "state_change": "active",
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "action": "change_state"
}
```

**Missing:** Target state condition (wait state) and exclude_self

---

### 29. Multiple conditional outcomes not captured (Line ~2239)
**Full Text:** "それらのカードのコストの合計が、6の場合、カードを1枚引く。合計が8の場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る。合計が25の場合、ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。"

**Issue:** The effect has multiple conditional outcomes based on different cost totals (6, 8, 25), but only one outcome is captured (the 25 case).

**Parsed Output:**
```json
"effect": {
  "text": "「{{jyouji.png|常時}}ライブの合計スコアを＋1する。」を得る",
  "condition": {
    "text": "{{icon_all.png|ハート}}を得る。合計が25の",
    "aggregate": "total",
    "type": "score_threshold_condition"
  },
  "action": "gain_resource"
}
```

**Missing:** The other conditional outcomes (cost total = 6: draw 1 card, cost total = 8: gain heart)

---

### 30. Ability absence condition not captured (Line ~2279)
**Full Text:** "自分のライブ中のライブカードに、{{live_start.png|ライブ開始時}}能力も{{live_success.png|ライブ成功時}}能力も持たないカードがあるかぎり"

**Issue:** The condition checks for cards that DON'T have specific abilities, but this negation is not captured.

**Parsed Output:**
```json
// (need to see the parsed output)
```

**Missing:** The ability absence/negation condition

---

### 31. Custom placement logic not captured (Line ~2239)
**Full Text:** "控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい"

**Issue:** The "好きな順番で" (in any order) aspect is not captured in the parsed output.

**Parsed Output:**
```json
"cost": {
  "text": "控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい",
  "source": "discard",
  "destination": "deck_bottom",
  "count": 2,
  "card_type": "member_card",
  "optional": true,
  "type": "move_cards"
}
```

**Missing:** The "any order" placement logic

---

### 32. Resource amount not captured (Line ~2279)
**Full Text:** "{{heart_06.png|heart06}}{{heart_06.png|heart06}}を得る"

**Issue:** The effect gains 2 heart06 but the amount is not specified.

**Parsed Output:**
```json
"effect": {
  "text": "{{heart_06.png|heart06}}{{heart_06.png|heart06}}を得る",
  // ...
  "action": "gain_resource",
  "resource": "heart"
}
```

**Missing:** Resource amount (should be count: 2)

---

### 33. Complex condition with resource threshold not captured (Line ~2350)
**Full Text:** "自分のステージに{{icon_blade.png|ブレード}}を5つ以上持つ『μ's』のメンバーがいない場合"

**Issue:** The condition checks for members with 5+ blades, but this resource threshold within the member condition is not captured.

**Parsed Output:**
```json
"condition": {
  "text": "このメンバーはセンターエリア以外にポジションチェンジする。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる",
  "type": "position_condition",
  "position": "center"
}
```

**Missing:** The resource threshold condition (5+ blades on μ's members) and the negation (not having such members)

---

### 34. Sequential action missing first part (Line ~2374)
**Full Text:** "カードを1枚引く。相手のステージにいるコスト9以下のメンバーを1人までウェイトにする。"

**Issue:** The effect only captures the second action (change_state), missing the first action (draw 1 card).

**Parsed Output:**
```json
"effect": {
  "text": "カードを1枚引く。相手のステージにいるコスト9以下のメンバーを1人までウェイトにする",
  "state_change": "wait",
  "count": 1,
  "card_type": "member_card",
  "target": "opponent",
  "cost_limit": 9,
  "max": true,
  "action": "change_state"
}
```

**Missing:** The first action (draw 1 card) - should be sequential

---

### 35. Dynamic count based on opponent not captured (Line ~2399)
**Full Text:** "相手のステージにいるウェイト状態のメンバーの数まで、自分の控え室にある『虹ヶ咲』のメンバーカードを選ぶ"

**Issue:** The count is dynamic (based on opponent's wait state members) but this is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "相手のステージにいるウェイト状態のメンバーの数まで、自分の控え室にある『虹ヶ咲』のメンバーカードを選ぶ。それらを好きな順番でデッキの上に置く",
  "source": "discard",
  "destination": "deck_top",
  "card_type": "member_card",
  "target": "both",
  "group": {
    "name": "虹ヶ咲"
  },
  "action": "move_cards"
}
```

**Missing:** Dynamic count based on opponent's wait state members

---

### 36. Both players effect not captured (Line ~2428)
**Full Text:** "自分と相手はそれぞれ、自身の控え室からライブカードを1枚手札に加える"

**Issue:** The effect applies to both players but the parsed output doesn't capture this.

**Parsed Output:**
```json
"effect": {
  "text": "自分と相手はそれぞれ、自身の控え室からライブカードを1枚手札に加える",
  "source": "discard",
  "destination": "hand",
  "count": 1,
  "card_type": "live_card",
  "action": "move_cards"
}
```

**Missing:** The "both players" aspect (target should be "both" or similar)

---

### 37. Both players effect not captured (Line ~2480)
**Full Text:** "自分と相手はそれぞれ、自身のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く"

**Issue:** Same as #36 - effect applies to both players but not captured.

**Parsed Output:**
```json
"effect": {
  "text": "自分と相手はそれぞれ、自身のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く",
  "state_change": "wait",
  "count": 1,
  "card_type": "energy_card",
  "action": "change_state"
}
```

**Missing:** The "both players" aspect

---

### 38. Resource amount not captured (Line ~2451)
**Full Text:** "{{heart_02.png|heart02}}{{heart_02.png|heart02}}を得る"

**Issue:** The effect gains 2 heart02 but the amount is not specified.

**Parsed Output:**
```json
"effect": {
  "text": "{{heart_02.png|heart02}}{{heart_02.png|heart02}}を得る",
  "condition": { ... },
  "duration": "as_long_as",
  "action": "gain_resource"
}
```

**Missing:** Resource amount (should be count: 2)

---

### 39. Card name matching condition not captured (Line ~2554)
**Full Text:** "それと同じカード名のカードが自分の成功ライブカード置き場にある場合"

**Issue:** The condition checks for cards with the same name as a selected card, but this name matching is not captured.

**Parsed Output:**
```json
"condition": {
  "text": "自分のライブ中の『虹ヶ咲』のライブカードを1枚選ぶ。それと同じカード名のカードが自分の成功ライブカード置き場にある",
  "target": "self",
  "location": "success_live_card_zone",
  "card_type": "live_card",
  "count": 1,
  "operator": "=",
  "comparison_type": "equality",
  "type": "group_condition",
  "group": {
    "name": "虹ヶ咲"
  }
}
```

**Missing:** The card name matching constraint ("それと同じカード名")

---

### 40. Distinct card names condition not captured (Line ~2615)
**Full Text:** "自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが3枚以上ある場合"

**Issue:** The condition checks for cards with distinct names, but this "distinct names" constraint is not captured.

**Parsed Output:**
```json
{
  "text": "自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが3枚以上ある場合、自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える",
  "source": "discard",
  "destination": "hand",
  "count": 3,
  "card_type": "live_card",
  "target": "self",
  "group": {
    "name": "虹ヶ咲"
  },
  "group_names": [
    "虹ヶ咲",
    "虹ヶ咲"
  ],
  "action": "move_cards"
}
```

**Missing:** The "distinct card names" constraint

---

### 41. Baton touch count not captured in condition (Line ~2680)
**Full Text:** "{{center.png|センター}}『Liella!』のメンバー2人からバトンタッチして登場している場合"

**Issue:** The condition specifies baton touch from 2 members, but this count is not captured in the parsed condition.

**Parsed Output:**
```json
"condition": {
  "text": "{{center.png|センター}}『Liella!』のメンバー2人からバトンタッチして登場している",
  "type": "appearance_condition",
  "appearance": true
}
```

**Missing:** The baton touch count (2 members) and group filter (Liella!)

---

### 42. Empty area condition IS captured (Line ~2680) - NOT AN ISSUE
**Full Text:** "自分のステージのメンバーのいないエリアに登場させる"

**Issue:** The destination must be an empty area - this IS captured as `"destination": "empty_area"`.

**Parsed Output:**
```json
{
  "text": "自分の控え室にあるコスト4以下の『Liella!』のメンバーカード1枚を自分のステージのメンバーのいないエリアに登場させる",
  "source": "discard",
  "destination": "empty_area",
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "group": {
    "name": "Liella!"
  },
  "cost_limit": 4,
  "action": "move_cards"
}
```

**Status:** This is correctly captured

---

### 42. Compound condition not fully captured (Line ~2729)
**Full Text:** "『Liella!』のメンバーからバトンタッチして登場しており、かつ自分のエネルギーが7枚以上ある場合"

**Issue:** The condition has two parts (baton touch AND energy count >= 7), but only the baton touch aspect is captured as appearance_condition.

**Parsed Output:**
```json
"condition": {
  "text": "『Liella!』のメンバーからバトンタッチして登場しており、かつ自分のエネルギーが7枚以上ある",
  "type": "appearance_condition",
  "appearance": true
}
```

**Missing:** The energy count condition (7+ energy) and the group filter (Liella!)

---

### 43. Resource amount not captured (Line ~2757)
**Full Text:** "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"

**Issue:** The effect gains 3 blades but the amount is not specified.

**Parsed Output:**
```json
"effect": {
  "text": "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "condition": { ... },
  "duration": "as_long_as",
  "action": "gain_resource",
  "resource": "blade"
}
```

**Missing:** Resource amount (should be count: 3)

---

### 44. Position trigger IS captured (Line ~2786) - NOT AN ISSUE
**Full Text:** "{{toujyou.png|登場}}{{leftside.png|左サイド}}カードを2枚引き、手札を1枚控え室に置く。"

**Issue:** The trigger has a position requirement (left side) - this IS captured as `"triggers": "登場, 左サイド"`.

**Parsed Output:**
```json
"triggers": "登場, 左サイド",
```

**Status:** This is correctly captured

---

### 45. Dynamic count based on score not captured (Line ~2907)
**Full Text:** "自分のデッキの上から、自分のライブの合計スコアに2を足した数に等しい枚数見る"

**Issue:** The count is dynamic (score + 2) but this calculation is not captured.

**Parsed Output:**
```json
"look_action": {
  "text": "自分のデッキの上から、自分のライブの合計スコアに2を足した数に等しい枚数見る。",
  "source": "deck_top",
  "target": "self",
  "action": "look_at"
}
```

**Missing:** The dynamic count calculation (score + 2)

---

### 46. Distinct condition IS captured (Line ~2945) - NOT AN ISSUE
**Full Text:** "自分のステージに名前が異なるメンバーが3人以上いるかぎり"

**Issue:** The distinct names condition IS captured with `"type": "distinct_condition"` and `"distinct": "name"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分のステージに名前が異なるメンバーが3人以上いるかぎり",
  "type": "distinct_condition",
  "distinct": "name"
}
```

**Status:** This is correctly captured

---

### 47. Conditional alternative based on card group not captured (Line ~2970)
**Full Text:** "これにより控え室に置いたカードが『μ's』のカードの場合、自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く。『μ's』のカード以外の場合、自分の控え室からライブカードを1枚手札に加える。"

**Issue:** The effect has two different outcomes based on whether the discarded card is μ's or not, but this conditional alternative is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "これにより控え室に置いたカードが『μ's』のカードの場合、自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く。『μ's』のカード以外の場合、自分の控え室からライブカードを1枚手札に加える",
  "action": "look_and_select"
}
```

**Missing:** The conditional alternative structure (if μ's card: look_and_select; else: move live card to hand)

---

### 48. Dynamic cost reduction based on group variety IS captured (Line ~3035) - NOT AN ISSUE
**Full Text:** "この能力を起動するためのコストは自分のステージにいるメンバーの中のグループ名1種類につき、{{icon_energy.png|E}}減る"

**Issue:** The dynamic cost reduction IS captured with `"per_unit": "いるメンバーの中のグループ名1種類"` and `"energy": 1`.

**Parsed Output:**
```json
"effect": {
  "per_unit": "いるメンバーの中のグループ名1種類",
  "energy": 1,
  "action": "pay_energy"
}
```

**Status:** This is correctly captured

---

### 49. Heart negation condition IS captured (Line ~3065) - NOT AN ISSUE
**Full Text:** "エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合"

**Issue:** The heart negation condition IS captured with `"type": "heart_negation_condition"` and `"heart_negation": true`.

**Parsed Output:**
```json
"condition": {
  "text": "自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある",
  "type": "heart_negation_condition",
  "heart_negation": true
}
```

**Status:** This is correctly captured

---

### 50. Ability negation in baton touch IS captured (Line ~3091) - NOT AN ISSUE
**Full Text:** "能力を持たないメンバーからバトンタッチして登場した場合"

**Issue:** The ability negation IS captured with `"ability_negation": true`.

**Parsed Output:**
```json
"condition": {
  "text": "能力を持たないメンバーからバトンタッチして登場した",
  "type": "baton_touch_condition",
  "ability_negation": true
}
```

**Status:** This is correctly captured

---

### 51. Cost modification for ability-less cards IS captured (Line ~3118) - NOT AN ISSUE
**Full Text:** "能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る"

**Issue:** The cost modification IS captured with `"action": "modify_cost"`.

**Parsed Output:**
```json
"effect": {
  "text": "能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る",
  "source": "hand",
  "card_type": "member_card",
  "target": "self",
  "action": "modify_cost",
  "operation": "decrease"
}
```

**Status:** This is correctly captured

---

### 52. Cost equality condition not fully captured (Line ~3141)
**Full Text:** "自分のステージの右サイドエリアと左サイドエリアにいるメンバーのコストが同じ場合"

**Issue:** The condition checks if costs are equal between left and right side areas, but only the left side position is captured.

**Parsed Output:**
```json
"condition": {
  "text": "{{center.png|センター}}自分のステージの右サイドエリアと左サイドエリアにいるメンバーのコストが同じ",
  "type": "position_condition",
  "position": "left_side"
}
```

**Missing:** The cost equality comparison between left and right side areas

---

### 53. Turn-based member filter not captured (Line ~3169)
**Full Text:** "自分のステージにいるこのターンに登場したメンバーのうち、『Aqours』以外のすべてのメンバー"

**Issue:** The target filter includes "members that appeared this turn" and "not Aqours", but these filters are not captured.

**Parsed Output:**
```json
"effect": {
  "text": "{{heart_03.png|heart03}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいるこのターンに登場したメンバーのうち、『Aqours』以外のすべてのメンバーは選んだハートを1つ得る",
  "card_type": "member_card",
  "target": "self",
  "group": {
    "name": "Aqours"
  },
  "action": "gain_resource",
  "resource": "heart"
}
```

**Missing:** The temporal filter (this turn) and group exclusion (not Aqours) - the group filter is for Aqours but should be exclusion

---

### 54. Heart variety condition IS captured (Line ~3204) - NOT AN ISSUE
**Full Text:** "エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合"

**Issue:** The heart variety condition IS captured with `"type": "heart_variety_condition"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある",
  "type": "heart_variety_condition"
}
```

**Status:** This is correctly captured

---

### 55. Cost thresholds with heart negation not fully captured (Line ~3245)
**Full Text:** "このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合"

**Issue:** The condition has cost threshold (10+) AND heart negation AND group filter, but only heart negation is captured.

**Parsed Output:**
```json
"condition": {
  "text": "このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた",
  "type": "heart_negation_condition",
  "heart_negation": true
}
```

**Missing:** Cost threshold (10+), group filter (虹ヶ咲), and baton touch condition

---

### 56. Card count equality between players IS captured (Line ~3289) - NOT AN ISSUE
**Full Text:** "自分と相手の成功ライブカード置き場にあるカードの枚数が同じ場合"

**Issue:** The equality condition IS captured with `"target": "both"` and `"comparison_type": "equality"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分と相手の成功ライブカード置き場にあるカードの枚数が同じ",
  "target": "both",
  "location": "success_live_card_zone",
  "card_type": "live_card",
  "operator": "=",
  "comparison_type": "equality",
  "type": "location_condition"
}
```

**Status:** This is correctly captured

---

### 57. Surplus heart condition IS captured (Line ~3318) - NOT AN ISSUE
**Full Text:** "自分が余剰ハートを1つ以上持っている場合"

**Issue:** The surplus heart condition IS captured with `"resource_type": "surplus_heart"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分が余剰ハートを1つ以上持っている",
  "resource_type": "surplus_heart",
  "count": 1,
  "operator": ">=",
  "type": "resource_count_condition"
}
```

**Status:** This is correctly captured

---

### 58. Energy under member cost not fully captured (Line ~3360)
**Full Text:** "エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く"

**Issue:** The cost involves placing energy under a specific member, but the target_member specification is not captured.

**Parsed Output:**
```json
"cost": {
  "text": "エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く",
  "source": "energy_zone",
  "destination": "under_member",
  "count": 1,
  "card_type": "member_card",
  "type": "move_cards"
}
```

**Missing:** The target_member specification (this_member)

---

### 59. Dynamic count based on energy under member not captured (Line ~3399)
**Full Text:** "このメンバーの下にあるエネルギーカードの枚数に1を足した枚数のエネルギーカードをウェイト状態で置く"

**Issue:** The count is dynamic (energy cards under member + 1) but this calculation is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "自分のエネルギーデッキから、このメンバーの下にあるエネルギーカードの枚数に1を足した枚数のエネルギーカードをウェイト状態で置く",
  "condition": { ... },
  "state_change": "wait",
  "card_type": "member_card",
  "target": "self",
  "action": "change_state"
}
```

**Missing:** The dynamic count calculation (energy under member + 1) and the source (energy_deck)

---

### 60. Choice effect IS captured (Line ~3429) - NOT AN ISSUE
**Full Text:** "以下から1つを選ぶ。\n・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。\n・カードを1枚引く。"

**Issue:** The choice effect IS captured with `"action": "choice"` and an `"options"` array.

**Parsed Output:**
```json
"effect": {
  "text": "以下から1つを選ぶ。\n・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。\n・カードを1枚引く",
  "action": "choice",
  "options": [
    {
      "text": "相手のステージにいるコスト4以下のメンバー1人をウェイトにする。",
      "state_change": "wait",
      "cost_limit": 4,
      "action": "change_state"
    },
    {
      "text": "カードを1枚引く",
      "action": "draw"
    }
  ]
}
```

**Status:** This is correctly captured

---

### 61. Choice cost IS captured (Line ~3471) - NOT AN ISSUE
**Full Text:** "このメンバーをウェイトにするか、手札を1枚控え室に置く"

**Issue:** The choice cost IS captured with `"type": "choice_condition"` and an `"options"` array.

**Parsed Output:**
```json
"cost": {
  "text": "このメンバーをウェイトにするか、手札を1枚控え室に置く",
  "type": "choice_condition",
  "options": [
    {
      "text": "このメンバーをウェイトにする",
      "state_change": "wait",
      "type": "change_state"
    },
    {
      "text": "手札を1枚控え室に置く",
      "source": "hand",
      "destination": "discard",
      "type": "move_cards"
    }
  ]
}
```

**Status:** This is correctly captured

---

### 62. Complex sequential with conditional effects not fully captured (Line ~3510)
**Full Text:** "カードを3枚引き、手札を2枚控え室に置く。これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、このメンバーをアクティブにする。2枚ある場合、さらにライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。"

**Issue:** The effect has multiple parts (draw/discard, then conditional activation based on discarded cards, then additional effect based on count), but only the final effect is captured.

**Parsed Output:**
```json
"effect": {
  "text": "さらにライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "condition": {
    "text": "このメンバーをアクティブにする。2枚ある",
    "card_type": "member_card",
    "count": 2,
    "type": "card_count_condition"
  },
  "action": "gain_resource",
  "resource": "blade"
}
```

**Missing:** The initial sequential actions (draw 3, discard 2), the conditional activation based on discarded cards without blade hearts, and the resource amount (2 blades)

---

### 63. Cost modification with specific cost not captured (Line ~3541)
**Full Text:** "コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る"

**Issue:** The cost modification has a specific cost threshold (cost 10) that is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る",
  "source": "hand",
  "card_type": "member_card",
  "target": "self",
  "group": {
    "name": "Liella!"
  },
  "action": "modify_cost",
  "operation": "decrease"
}
```

**Missing:** The cost threshold (cost 10) and the reduction amount (2)

---

### 64. Multiple targets in single effect not captured (Line ~3570)
**Full Text:** "自分のステージにいるすべての『Liella!』のメンバーと、自分のすべてのエネルギーをアクティブにする"

**Issue:** The effect targets both members AND energy, but only one target type is captured.

**Parsed Output:**
```json
"effect": {
  "text": "{{center.png|センター}}自分のステージにいるすべての『Liella!』のメンバーと、自分のすべてのエネルギーをアクティブにする",
  "state_change": "active",
  "card_type": "member_card",
  "target": "self"
}
```

**Missing:** The second target type (energy) - should be sequential or have multiple targets

---

### 65. OR condition not fully captured (Line ~3598)
**Full Text:** "このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき"

**Issue:** The condition has an OR (movement OR energy placement), but only one aspect is captured.

**Parsed Output:**
```json
"condition": {
  "text": "自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれた",
  "target": "self",
  "location": "energy_zone",
  "card_type": "member_card",
  "movement": true,
  "type": "location_condition"
}
```

**Missing:** The OR condition structure and the movement aspect

---

### 66. Per-unit with group filter IS captured (Line ~3638) - NOT AN ISSUE
**Full Text:** "これにより控え室に置いた『Liella!』のメンバーカード1枚につき"

**Issue:** The per-unit with group filter IS captured with `"group": {"name": "Liella!"}`.

**Parsed Output:**
```json
"effect": {
  "per_unit": "置いた『Liella!』のメンバーカード1枚",
  "card_type": "member_card",
  "group": {
    "name": "Liella!"
  },
  "action": "gain_resource",
  "resource": "blade"
}
```

**Status:** This is correctly captured

---

### 67. Trigger type "each_time" IS captured (Line ~3673) - NOT AN ISSUE
**Full Text:** "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび"

**Issue:** The trigger type IS captured with `"trigger_type": "each_time"`.

**Parsed Output:**
```json
"effect": {
  "trigger_type": "each_time",
  "trigger_event": "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれる",
  "action": "sequential"
}
```

**Status:** This is correctly captured

---

### 68. Sequential action missing first part (Line ~3710)
**Full Text:** "自分のデッキの上からカードを4枚控え室に置く。それらの中にライブカードがある場合、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。"

**Issue:** The effect only captures the gain_resource, missing the initial move_cards action.

**Parsed Output:**
```json
"effect": {
  "text": "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "condition": {
    "text": "自分のデッキの上からカードを4枚控え室に置く。それらの中にライブカードがある",
    "source": "deck_top",
    "destination": "discard",
    "count": 4,
    "type": "location_condition"
  },
  "action": "gain_resource",
  "resource": "blade"
}
```

**Missing:** The initial action (move 4 cards from deck_top to discard) and resource amount (2 blades)

---

### 69. Card name inclusion condition not captured (Line ~3741)
**Full Text:** "これにより公開したカードのカード名がすべて含まれるライブカード"

**Issue:** The condition checks for cards whose name contains all the names of the revealed card, but this name inclusion logic is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "自分の控え室から、これにより公開したカードのカード名がすべて含まれるライブカードを1枚手札に加える",
  "source": "discard",
  "destination": "hand",
  "count": 1,
  "card_type": "live_card",
  "target": "self",
  "action": "move_cards"
}
```

**Missing:** The card name inclusion filter

---

### 70. Distinct cost condition IS captured (Line ~3771) - NOT AN ISSUE
**Full Text:** "自分のステージにコストがそれぞれ異なるメンバーが3人以上いるかぎり"

**Issue:** The distinct cost condition IS captured with `"distinct": "cost"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分のステージにコストがそれぞれ異なるメンバーが3人以上いるかぎり",
  "target": "self",
  "location": "stage",
  "card_type": "member_card",
  "count": 3,
  "operator": ">=",
  "comparison_type": "cost",
  "distinct": "cost",
  "type": "location_count_condition"
}
```

**Status:** This is correctly captured

---

### 71. Empty area destination IS captured (Line ~3803) - NOT AN ISSUE
**Full Text:** "メンバーのいないエリアに登場させる"

**Issue:** The empty area destination IS captured with `"destination": "empty_area"`.

**Parsed Output:**
```json
"effect": {
  "source": "discard",
  "destination": "empty_area",
  "count": 1,
  "card_type": "member_card",
  "cost_limit": 2,
  "action": "move_cards"
}
```

**Status:** This is correctly captured

---

### 72. Position change with optional IS captured (Line ~3832) - NOT AN ISSUE
**Full Text:** "メンバー1人をポジションチェンジさせてもよい"

**Issue:** The optional position change IS captured with `"optional": true`.

**Parsed Output:**
```json
"effect": {
  "text": "メンバー1人をポジションチェンジさせてもよい",
  "count": 1,
  "optional": true,
  "action": "position_change"
}
```

**Status:** This is correctly captured

---

### 73. Group matching condition not captured (Line ~3860)
**Full Text:** "これにより控え室に置いたカードと同じグループ名を持つメンバー"

**Issue:** The target filter matches members with the same group as the discarded card, but this group matching is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "ライブ終了時まで、これにより控え室に置いたカードと同じグループ名を持つメンバー1人は、{{heart_01.png|heart01}}を得る",
  "duration": "live_end",
  "card_type": "member_card",
  "action": "gain_resource"
}
```

**Missing:** The group matching filter (same group as discarded card)

---

### 74. Multiple group options not captured (Line ~3888)
**Full Text:** "このメンバーを『Aqours』か『SaintSnow』のメンバーがいるエリアにポジションチェンジする"

**Issue:** The destination has multiple group options (Aqours OR SaintSnow), but only one group is captured.

**Parsed Output:**
```json
"effect": {
  "text": "このメンバーを『Aqours』か『SaintSnow』のメンバーがいるエリアにポジションチェンジする",
  "card_type": "member_card",
  "group": {
    "name": "Aqours"
  },
  "group_names": [
    "Aqours",
    "SaintSnow"
  ],
  "action": "position_change"
}
```

**Missing:** The OR condition between the two groups (Aqours OR SaintSnow)

---

### 75. Exact count condition IS captured (Line ~3919) - NOT AN ISSUE
**Full Text:** "自分のエネルギーがちょうど8枚あるかぎり"

**Issue:** The exact count condition IS captured with `"operator": "="`.

**Parsed Output:**
```json
"condition": {
  "text": "自分のエネルギーがちょうど8枚あるかぎり",
  "target": "self",
  "resource_type": "energy",
  "count": 8,
  "operator": "=",
  "type": "energy_condition"
}
```

**Status:** This is correctly captured

---

### 76. Choice effect parsed as sequential instead of choice (Line ~3949)
**Full Text:** "以下から1つを選ぶ。\n・カードを1枚引き、手札を1枚控え室に置く。\n・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。"

**Issue:** The effect is a choice between two options, but it's parsed as sequential actions instead of a choice structure.

**Parsed Output:**
```json
"effect": {
  "text": "以下から1つを選ぶ。\n・カードを1枚引き、手札を1枚控え室に置く。\n・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする",
  "action": "sequential",
  "actions": [
    {
      "text": "以下から1つを選ぶ。\n・カードを1枚引き",
      "count": 1,
      "action": "draw"
    },
    {
      "text": "手札を1枚控え室に置く。\n・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする",
      "action": "change_state"
    }
  ]
}
```

**Missing:** Should be `"action": "choice"` with an `"options"` array

---

### 77. Cost with sequential actions not captured (Line ~3987)
**Full Text:** "このメンバーをウェイトにし、手札を1枚控え室に置く"

**Issue:** The cost has two sequential actions (change state + move cards), but only one is captured.

**Parsed Output:**
```json
"cost": {
  "text": "このメンバーをウェイトにし、手札を1枚控え室に置く",
  "source": "hand",
  "destination": "discard",
  "count": 1,
  "card_type": "member_card",
  "type": "move_cards"
}
```

**Missing:** The first action (change this member to wait state)

---

### 78. "either" target IS captured (Line ~4035) - NOT AN ISSUE
**Full Text:** "自分か相手のステージにコスト13以上のメンバーがいる場合"

**Issue:** The "either" target IS captured with `"target": "either"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分か相手のステージにコスト13以上のメンバーがいる",
  "target": "either",
  "location": "stage",
  "card_type": "member_card",
  "operator": ">=",
  "comparison_type": "cost",
  "cost_limit": 13,
  "type": "cost_limit_condition"
}
```

**Status:** This is correctly captured

---

### 79. Complex compound condition not fully captured (Line ~4064)
**Full Text:** "自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合"

**Issue:** The condition has two parts (other members exist AND no live cards in revealed hand), but only one aspect is captured in the look_action.

**Parsed Output:**
```json
"look_action": {
  "text": "自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合、自分のデッキの上からカードを5枚見る。",
  "source": "deck_top",
  "count": 5,
  "card_type": "member_card",
  "target": "self",
  "action": "look_at"
}
```

**Missing:** The compound condition structure (other members AND no live cards in revealed hand)

---

### 80. OR condition for card types not captured (Line ~4102)
**Full Text:** "コスト2以下のメンバーカードか、スコア２以下のライブカード"

**Issue:** The target has an OR condition (member card with cost <= 2 OR live card with score <= 2), but only one card type is captured.

**Parsed Output:**
```json
"effect": {
  "text": "エールにより公開された自分のカードの中から、コスト2以下のメンバーカードか、スコア2以下のライブカードを1枚手札に加える",
  "destination": "hand",
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "cost_limit": 2,
  "action": "move_cards"
}
```

**Missing:** The OR condition between card types and the score threshold for live cards

---

### 81. Simple energy count condition IS captured (Line ~4132) - NOT AN ISSUE
**Full Text:** "自分のエネルギーが7枚以上ある場合"

**Issue:** The energy count condition IS captured with `"type": "resource_count_condition"`.

**Parsed Output:**
```json
"condition": {
  "text": "自分のエネルギーが7枚以上ある",
  "target": "self",
  "resource_type": "energy",
  "count": 7,
  "operator": ">=",
  "type": "resource_count_condition"
}
```

**Status:** This is correctly captured

---

### 82. Conditional effect based on discarded card type not captured (Line ~4161)
**Full Text:** "これによりライブカードを控え室に置いた場合、さらにカードを1枚引く"

**Issue:** The conditional effect depends on whether the discarded card was a live card, but this card type check is not captured.

**Parsed Output:**
```json
"effect": {
  "text": "さらにカードを1枚引く",
  "condition": {
    "text": "ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。これによりライブカードを控え室に置いた",
    "location": "discard"
  }
}
```

**Missing:** The card type filter (live card) and the initial effect (gain blade until live end)

---

## Summary

The main categories of missing information are:

1. **Choice mechanisms** - "好きなハートの色を1つ指定する" type choices
2. **Location specifications** - incomplete per_unit descriptions
3. **Cost limits** - "4コスト以下" type constraints (sometimes missing in cost, sometimes in effect)
4. **Distinct constraints** - "名前が異なる" type conditions
5. **Turn limits** - ［ターン1回］ type restrictions
6. **Filter criteria** - heart/icon requirements on cards
7. **Resource amounts** - when multiple resources are gained (e.g., {{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}})
8. **Custom actions** - complex logic marked as "custom"
9. **Ability invalidation** - "無効にしてもよい" type effects
10. **Comparison operators** - "より大きい" type comparisons
11. **Area specifications** - "エリアすべて" type constraints
12. **Conditional effects** - "これにより無効にした場合" type conditions
13. **Per-unit scaling** - "メンバー1人につき" type scaling
14. **Group filters in conditions** - group requirements not captured in baton touch, etc.
15. **Name matching** - "同じ名前を持つ" type constraints
16. **Multiple resource types** - gaining heart AND blade together
17. **Conditional sequential actions** - "そうした場合" type dependencies
18. **Dynamic cost calculations** - "コストに2を足した数" type calculations
19. **Target state conditions** - "ウェイト状態のメンバー" type state requirements
20. **Multiple conditional outcomes** - different effects based on different thresholds
21. **Ability absence conditions** - cards that DON'T have specific abilities
22. **Placement order logic** - "好きな順番で" type placement options
23. **Draw until conditions** - "手札が5枚になるまで" type conditions
24. **Temporal conditions** - "このターン" type time constraints
25. **exclude_self in costs** - "このメンバー以外" type exclusions
26. **Resource thresholds in member conditions** - members with X amount of resources
27. **Sequential action splitting** - when multiple actions are parsed as one
28. **Dynamic counts** - count based on game state (opponent's wait members, etc.)
29. **Both players effects** - "自分と相手はそれぞれ" type effects
30. **Card name matching** - "それと同じカード名" type constraints
31. **Distinct card names** - "カード名の異なる" type constraints
32. **Baton touch counts** - "メンバー2人からバトンタッチ" type counts
33. **Compound conditions** - multiple conditions combined with "かつ" (and)
34. **Position triggers** - "{{leftside.png|左サイド}}" type position requirements (these ARE captured)
35. **Dynamic score-based counts** - "合計スコアに2を足した数" type calculations
36. **Conditional alternatives based on card properties** - different effects based on card group/type
37. **Cost equality conditions** - "コストが同じ" type comparisons between areas
38. **Turn-based member filters** - "このターンに登場したメンバー" type temporal filters
39. **Group exclusion in targets** - "『Aqours』以外" type negative group filters
40. **Cost thresholds in conditions** - "コスト10以上" type thresholds combined with other conditions
41. **Target member specifications** - "このメンバーの下に置く" type specific target references
42. **Dynamic counts based on card state** - "このメンバーの下にあるエネルギーカードの枚数" type calculations
43. **Cost modification thresholds** - "コスト10の" type specific cost requirements in cost modifications
44. **Multiple target types in single effect** - "メンバーと、エネルギーを" type multi-target effects
45. **OR conditions** - "エリアを移動するかエネルギーが置かれた" type disjunctive conditions
46. **Card name inclusion** - "カード名がすべて含まれる" type name inclusion filters
47. **Distinct cost constraints** - "コストがそれぞれ異なる" type distinct cost conditions
48. **Group matching with previous action** - "控え室に置いたカードと同じグループ名" type dynamic group matching
49. **Multiple group options** - "『Aqours』か『SaintSnow』" type OR group filters
50. **Choice parsed as sequential** - choice effects incorrectly parsed as sequential actions
51. **Sequential costs** - costs with multiple actions parsed as single action
52. **OR conditions for card types** - "メンバーカードか、ライブカード" type disjunctive card type filters
53. **Conditional effects based on discarded card type** - "ライブカードを控え室に置いた場合" type conditional effects

---

## Verification Summary

This document was investigated on April 21, 2026 by checking the actual parsed data in abilities.json against each claimed issue.

### Confirmed REAL PROBLEMS (18 issues):
- Issues #1, #2, #3, #4, #10, #13, #14, #15, #16, #17, #25, #26, #27, #31, #35, #36, #76, #77
- These represent genuine parsing gaps that need to be addressed

### Confirmed NOT ISSUES (correctly marked):
- Issues #42, #44, #46, #48, #49, #50, #51, #54, #56, #57, #60, #61, #66, #67, #70, #71, #72, #75, #78, #81
- These features are already correctly captured in the parsed data

### Not yet verified:
- Remaining issues in the document should be individually verified against abilities.json
