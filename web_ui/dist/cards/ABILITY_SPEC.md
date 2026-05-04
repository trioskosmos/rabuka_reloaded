# Ability Action Spec Report
Generated from 602 unique abilities across 23 action types.

## Overview

| Action | Count | % |
|--------|------|---|
| gain_resource | 119 | 19% |
| sequential | 104 | 17% |
| move_cards | 77 | 12% |
| modify_score | 67 | 11% |
| change_state | 58 | 9% |
| look_and_select | 57 | 9% |
| draw_card | 38 | 6% |
| restriction | 21 | 3% |
| position_change | 13 | 2% |
| choice | 8 | 1% |
| modify_cost | 8 | 1% |
| conditional_alternative | 7 | 1% |
| appear | 6 | 0% |
| gain_ability | 5 | 0% |
| __none__ | 3 | 0% |
| modify_yell_count | 3 | 0% |
| activate_ability | 2 | 0% |
| draw_until_count | 1 | 0% |
| play_baton_touch | 1 | 0% |
| set_card_identity | 1 | 0% |
| place_energy_under_member | 1 | 0% |
| set_score | 1 | 0% |
| set_blade_count | 1 | 0% |

---

## gain_resource (119 occurrences)

### Example 1

**Text:** [ライブ開始時][⚡E]支払ってもよい：ライブ終了時まで、[⚔ブレード][⚔ブレード]を得る。

**Trigger:** ライブ開始時 | **Use limit:** None

**Cost:** [pay_energy] [⚡E]支払ってもよい

**Effect (my best guess):**
Gain 2× blade
  [duration: live_end]

**Raw effect keys:** ['action', 'count', 'duration', 'resource', 'text']
```json
{
  "text": "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "duration": "live_end",
  "action": "gain_resource",
  "resource": "blade",
  "count": 2
}
```

### Example 2

**Text:** [ライブ開始時][♥heart01]か[♥heart03]か[♥heart06]のうち、1つを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る。

**Trigger:** ライブ開始時 | **Use limit:** None

**Effect (my best guess):**
Gain 3× heart
  [per-unit: 枚×1]
  [duration: live_end]

**Raw effect keys:** ['action', 'count', 'duration', 'dynamic_count', 'heart_colors', 'location', 'per_unit', 'per_unit_count', 'per_unit_type', 'resource', 'text']
```json
{
  "text": "{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る",
  "action": "gain_resource",
  "resource": "heart",
  "per_unit": true,
  "per_unit_count": 1,
  "per_unit_type": "枚",
  "location": "live_card_zone",
  "duration": "live_end",
  "dynamic_count": {
    "type": "per_unit",
    "reference": "unit_count"
  },
  "count": 3,
  "heart_colors": [
    "heart01",
    "heart03",
    "heart06",
    "heart01",
    "heart03",
    "heart06"
  ]
}
```

---

## sequential (104 occurrences)

### Example 1

**Text:** [登場]カードを1枚引き、手札を1枚控え室に置く。

**Trigger:** 登場 | **Use limit:** None

**Effect (my best guess):**
Sequential (2 steps):
  Draw 1 from deck → hand
  Move 1× card from hand → discard

**Raw effect keys:** ['action', 'actions', 'count', 'text']
```json
{
  "text": "カードを1枚引き、手札を1枚控え室に置く",
  "action": "sequential",
  "actions": [
    {
      "text": "カードを1枚引き",
      "count": 1,
      "action": "draw_card",
      "source": "deck",
      "destination": "hand"
    },
    {
      "text": "手札を1枚控え室に置く",
      "source": "hand",
      "destination": "discard",
      "count": 1,
      "action": "move_cards",
      "card_type": "card"
    }
  ],
  "count": 1
}
```

### Example 2

**Text:** [登場]カードを2枚引き、手札を1枚控え室に置く。

**Trigger:** 登場 | **Use limit:** None

**Effect (my best guess):**
Sequential (2 steps):
  Draw 2 from deck → hand
  Move 1× card from hand → discard

**Raw effect keys:** ['action', 'actions', 'count', 'text']
```json
{
  "text": "カードを2枚引き、手札を1枚控え室に置く",
  "action": "sequential",
  "actions": [
    {
      "text": "カードを2枚引き",
      "count": 2,
      "action": "draw_card",
      "source": "deck",
      "destination": "hand"
    },
    {
      "text": "手札を1枚控え室に置く",
      "source": "hand",
      "destination": "discard",
      "count": 1,
      "action": "move_cards",
      "card_type": "card"
    }
  ],
  "count": 2
}
```

---

## move_cards (77 occurrences)

### Example 1

**Text:** [起動]このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。

**Trigger:** 起動 | **Use limit:** None

**Cost:** [move_cards] このメンバーをステージから控え室に置く

**Effect (my best guess):**
Move 1× live_card from discard → hand

**Raw effect keys:** ['action', 'card_type', 'count', 'destination', 'source', 'target', 'text']
```json
{
  "text": "自分の控え室からライブカードを1枚手札に加える",
  "source": "discard",
  "destination": "hand",
  "count": 1,
  "card_type": "live_card",
  "target": "self",
  "action": "move_cards"
}
```

### Example 2

**Text:** [起動]このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを1枚手札に加える。

**Trigger:** 起動 | **Use limit:** None

**Cost:** [move_cards] このメンバーをステージから控え室に置く

**Effect (my best guess):**
Move 1× member_card from discard → hand

**Raw effect keys:** ['action', 'card_type', 'count', 'destination', 'source', 'target', 'text']
```json
{
  "text": "自分の控え室からメンバーカードを1枚手札に加える",
  "source": "discard",
  "destination": "hand",
  "count": 1,
  "card_type": "member_card",
  "target": "self",
  "action": "move_cards"
}
```

---

## modify_score (67 occurrences)

### Example 1

**Text:** [起動][ターン1回]手札にあるメンバーカードを好きな枚数公開する：公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、ライブ終了時まで、「[常時]ライブの合計スコアを＋１する。」を得る。

**Trigger:** 起動 | **Use limit:** 1

**Cost:** [reveal] 手札にあるメンバーカードを好きな枚数公開する

**Effect (my best guess):**
Modify score: None ?

**Condition:**
comparison_condition: 公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合

**Raw effect keys:** ['action', 'condition', 'duration', 'operation', 'text']
```json
{
  "text": "「{{jyouji.png|常時}}ライブの合計スコアを＋1する。」を得る",
  "condition": {
    "text": "公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合",
    "comparison_type": "cost",
    "aggregate": "total",
    "values": [
      10,
      20,
      30,
      40,
      50
    ],
    "type": "comparison_condition"
  },
  "action": "modify_score",
  "operation": null,
  "duration": "live_end"
}
```

### Example 2

**Text:** [常時]自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合、「[常時]ライブの合計スコアを＋１する。」を得る。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Modify score: None ?

**Condition:**
Compound (2 sub-conditions)
  appearance_condition: 自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、
  location_condition: 名前が異なる場合

**Raw effect keys:** ['action', 'condition', 'operation', 'text']
```json
{
  "text": "「{{jyouji.png|常時}}ライブの合計スコアを＋1する。」を得る",
  "condition": {
    "type": "compound",
    "operator": "and",
    "conditions": [
      {
        "type": "appearance_condition",
        "appearance": true,
        "text": "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、",
        "all_areas": true
      },
      {
        "type": "location_condition",
        "location": "stage",
        "target": "self",
        "distinct": true,
        "text": "名前が異なる場合"
      }
    ],
    "text": "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合",
    "target": "self",
    "location": "stage",
    "card_type": "member
```

---

## change_state (58 occurrences)

### Example 1

**Text:** [登場]/[ライブ開始時]このメンバーをウェイトにしてもよい：相手のステージにいるコスト4以下のメンバー1人をウェイトにする。（ウェイト状態のメンバーが持つ[⚔ブレード]は、エールで公開する枚数を増やさない。）

**Trigger:** ライブ開始時, 登場 | **Use limit:** None

**Cost:** [change_state] このメンバーをウェイトにしてもよい

**Effect (my best guess):**
Change state to wait (1 items from ? → ?)

**Raw effect keys:** ['action', 'card_type', 'cost_limit', 'count', 'state_change', 'target', 'text']
```json
{
  "text": "相手のステージにいるコスト4以下のメンバー1人をウェイトにする。",
  "cost_limit": 4,
  "state_change": "wait",
  "count": 1,
  "card_type": "member_card",
  "target": "opponent",
  "action": "change_state"
}
```

### Example 2

**Text:** [登場]手札を1枚控え室に置いてもよい：自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。

**Trigger:** 登場 | **Use limit:** None

**Cost:** [move_cards] 手札を1枚控え室に置いてもよい

**Effect (my best guess):**
Change state to wait (1 items from deck → energy_zone)

**Raw effect keys:** ['action', 'card_type', 'count', 'destination', 'source', 'state_change', 'target', 'text']
```json
{
  "text": "自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く",
  "source": "deck",
  "destination": "energy_zone",
  "state_change": "wait",
  "count": 1,
  "card_type": "energy_card",
  "target": "self",
  "action": "change_state"
}
```

---

## look_and_select (57 occurrences)

### Example 1

**Text:** [登場]手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。

**Trigger:** 登場 | **Use limit:** None

**Cost:** [move_cards] 手札を1枚控え室に置いてもよい

**Effect (my best guess):**
Look then select:
  look_at: 自分のデッキの上からカードを3枚見る。
  Sequential (2 steps):
    Move 1× card from looked_at → hand
    Move 1× card from looked_at_remaining → discard

**Raw effect keys:** ['action', 'count', 'look_action', 'select_action', 'text']
```json
{
  "text": "自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く",
  "action": "look_and_select",
  "look_action": {
    "text": "自分のデッキの上からカードを3枚見る。",
    "source": "deck_top",
    "count": 3,
    "target": "self",
    "action": "look_at"
  },
  "select_action": {
    "action": "sequential",
    "actions": [
      {
        "text": "1枚を手札に加え",
        "count": 1,
        "action": "move_cards",
        "destination": "hand",
        "card_type": "card",
        "source": "looked_at"
      },
      {
        "text": "残りを控え室に置く",
        "destination": "discard",
        "action": "move_cards",
        
```

### Example 2

**Text:** [登場]自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。

**Trigger:** 登場 | **Use limit:** None

**Effect (my best guess):**
Look then select:
  look_at: 自分のデッキの上からカードを3枚見る。
  Sequential (2 steps):
    Move 1× card from looked_at → deck_top
    Move 1× card from looked_at_remaining → discard

**Raw effect keys:** ['action', 'count', 'look_action', 'select_action', 'text']
```json
{
  "text": "自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く",
  "action": "look_and_select",
  "look_action": {
    "text": "自分のデッキの上からカードを3枚見る。",
    "source": "deck_top",
    "count": 3,
    "target": "self",
    "action": "look_at"
  },
  "select_action": {
    "action": "sequential",
    "actions": [
      {
        "text": "好きな枚数を好きな順番でデッキの上に置き",
        "placement_order": "any_order",
        "action": "move_cards",
        "destination": "deck_top",
        "source": "looked_at",
        "card_type": "card",
        "count": 1,
        "any_number": true
      },
      {
       
```

---

## draw_card (38 occurrences)

### Example 1

**Text:** [起動][ターン1回][⚡E][⚡E]：カードを1枚引く。

**Trigger:** 起動 | **Use limit:** 1

**Cost:** [pay_energy] [⚡E][⚡E]

**Effect (my best guess):**
Draw 1 from deck → hand

**Raw effect keys:** ['action', 'count', 'destination', 'source', 'text']
```json
{
  "text": "カードを1枚引く",
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand"
}
```

### Example 2

**Text:** [登場][⚡E][⚡E]支払ってもよい：ステージの左サイドエリアに登場しているなら、カードを2枚引く。

**Trigger:** 登場 | **Use limit:** None

**Cost:** [pay_energy] [⚡E][⚡E]支払ってもよい

**Effect (my best guess):**
Draw 2 from deck → hand
  [condition present]

**Condition:**
appearance_condition: ステージの左サイドエリアに登場しているなら

**Raw effect keys:** ['action', 'condition', 'count', 'destination', 'source', 'text']
```json
{
  "text": "カードを2枚引く",
  "condition": {
    "type": "appearance_condition",
    "appearance": true,
    "text": "ステージの左サイドエリアに登場しているなら"
  },
  "count": 2,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand"
}
```

---

## restriction (21 occurrences)

### Example 1

**Text:** [常時]相手のライブカード置き場にあるすべてのライブカードは、成功させるための必要ハートが[♥heart0]多くなる。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Restriction: modify_required_hearts_global

**Raw effect keys:** ['action', 'count', 'heart_colors', 'operation', 'restriction_type', 'target', 'text']
```json
{
  "text": "相手のライブカード置き場にあるすべてのライブカードは、成功させるための必要ハートが{{heart_00.png|heart0}}多くなる",
  "action": "restriction",
  "restriction_type": "modify_required_hearts_global",
  "operation": "increase",
  "target": "相手のライブカード置き場にあるすべてのライブカード",
  "count": 1,
  "heart_colors": [
    "heart00",
    "heart00"
  ]
}
```

### Example 2

**Text:** [常時]このメンバーは自分のアクティブフェイズにアクティブにしない。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Restriction: cannot_activate

**Raw effect keys:** ['action', 'card_type', 'count', 'restriction_type', 'target', 'text']
```json
{
  "text": "このメンバーは自分のアクティブフェイズにアクティブにしない",
  "card_type": "member_card",
  "target": "self",
  "action": "restriction",
  "restriction_type": "cannot_activate",
  "count": 1
}
```

---

## position_change (13 occurrences)

### Example 1

**Text:** [ライブ開始時]自分のステージに[⚔ブレード]を5つ以上持つ『μ's』のメンバーがいない場合、このメンバーはセンターエリア以外にポジションチェンジする。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)

**Trigger:** ライブ開始時 | **Use limit:** None

**Effect (my best guess):**
Position change → center

**Condition:**
group_condition: 自分のステージに[⚔ブレード]を5つ以上持つ『μ's』のメンバーがいない場合

**Raw effect keys:** ['action', 'condition', 'parenthetical', 'position', 'target', 'text']
```json
{
  "text": "このメンバーはセンターエリア以外にポジションチェンジする",
  "condition": {
    "text": "自分のステージに{{icon_blade.png|ブレード}}を5つ以上持つ『μ's』のメンバーがいない場合",
    "target": "self",
    "location": "stage",
    "card_type": "member_card",
    "count": 5,
    "operator": ">=",
    "negation": true,
    "group": {
      "name": "μ's"
    },
    "type": "group_condition"
  },
  "position": "center",
  "action": "position_change",
  "target": null,
  "parenthetical": [
    "このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"
  ]
}
```

### Example 2

**Text:** [ライブ開始時]このメンバーをポジションチェンジしてもよい。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)

**Trigger:** ライブ開始時 | **Use limit:** None

**Effect (my best guess):**
Position change → any

**Raw effect keys:** ['action', 'card_type', 'optional', 'target', 'text']
```json
{
  "text": "このメンバーをポジションチェンジしてもよい。",
  "card_type": "member_card",
  "optional": true,
  "action": "position_change",
  "target": null
}
```

---

## choice (8 occurrences)

### Example 1

**Text:** [登場]/[ライブ開始時][⚡E]支払ってもよい：以下から1つを選ぶ。
・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
・カードを1枚引く。

**Trigger:** ライブ開始時, 登場 | **Use limit:** None

**Cost:** [pay_energy] [⚡E]支払ってもよい

**Effect (my best guess):**
Choice (2 options)

**Raw effect keys:** ['action', 'count', 'options', 'text']
```json
{
  "text": "以下から1つを選ぶ。\n・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。\n・カードを1枚引く",
  "action": "choice",
  "options": [
    {
      "text": "相手のステージにいるコスト4以下のメンバー1人をウェイトにする。",
      "cost_limit": 4,
      "state_change": "wait",
      "count": 1,
      "card_type": "member_card",
      "target": "opponent",
      "action": "change_state"
    },
    {
      "text": "カードを1枚引く",
      "count": 1,
      "action": "draw_card",
      "source": "deck",
      "destination": "hand"
    }
  ],
  "count": 1
}
```

### Example 2

**Text:** [登場]以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。

**Trigger:** 登場 | **Use limit:** None

**Effect (my best guess):**
Choice (2 options)

**Raw effect keys:** ['action', 'count', 'options', 'text']
```json
{
  "text": "以下から1つを選ぶ。\n・カードを1枚引き、手札を1枚控え室に置く。\n・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする",
  "action": "choice",
  "options": [
    {
      "text": "カードを1枚引き、手札を1枚控え室に置く。",
      "source": "deck",
      "destination": "hand",
      "count": 1,
      "action": "draw_card"
    },
    {
      "text": "相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする",
      "cost_limit": 2,
      "state_change": "wait",
      "card_type": "member_card",
      "target": "opponent",
      "action": "change_state",
      "count": 1
    }
  ],
  "count": 1
}
```

---

## modify_cost (8 occurrences)

### Example 1

**Text:** [常時]自分のエネルギーが10枚以上ある場合、ステージにいるこのメンバーのコストを＋４する。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Modify cost: decrease by ?

**Condition:**
card_count_condition: 自分のエネルギーが10枚以上ある場合

**Raw effect keys:** ['action', 'card_type', 'condition', 'text']
```json
{
  "text": "ステージにいるこのメンバーのコストを＋4する",
  "condition": {
    "type": "card_count_condition",
    "count": 10,
    "operator": ">=",
    "text": "自分のエネルギーが10枚以上ある場合"
  },
  "card_type": "member_card",
  "action": "modify_cost"
}
```

### Example 2

**Text:** [常時]自分の成功ライブカード置き場に『lilywhite』のカードがある場合、手札にあるこのメンバーカードのコストは2減る。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Modify cost: decrease by 2

**Condition:**
group_condition: 自分の成功ライブカード置き場に『lilywhite』のカードがある場合

**Raw effect keys:** ['action', 'condition', 'operation', 'text', 'value']
```json
{
  "text": "手札にあるこのメンバーカードのコストは2減る",
  "condition": {
    "text": "自分の成功ライブカード置き場に『lilywhite』のカードがある場合",
    "target": "self",
    "location": "success_live_card_zone",
    "card_type": "live_card",
    "group": {
      "name": "lilywhite"
    },
    "type": "group_condition"
  },
  "action": "modify_cost",
  "operation": "decrease",
  "value": 2
}
```

---

## conditional_alternative (7 occurrences)

### Example 1

**Text:** [常時]自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合、「[ライブ成功時]エールにより公開された自分のカードの中にライブカードが1枚以上ある場合、ライブの合計スコアを＋１する。ライブカードが3枚以上ある場合、代わりに合計スコアを＋２する。」を得る。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Conditional alternative:
  Modify score: None 1
  Modify score: None 1

**Raw effect keys:** ['action', 'alternative_effect', 'count', 'primary_effect', 'text']
```json
{
  "text": "自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合、「{{live_success.png|ライブ成功時}}エールにより公開された自分のカードの中にライブカードが1枚以上ある場合、ライブの合計スコアを＋1する。ライブカードが3枚以上ある場合、代わりに合計スコアを＋2する。」を得る",
  "action": "conditional_alternative",
  "primary_effect": {
    "text": "自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合、「{{live_success.png|ライブ成功時}}エールにより公開された自分のカードの中にライブカードが1枚以上ある場合、ライブの合計スコアを＋1する。ライブカードが3枚以上ある場合、",
    "count": 1,
    "card_type": "member_card",
    "target": "self",
    "group": {
      "name": "Aqours"
    },
    "group_names": [
      "Aqours"
    ],
    "action": "modify_score",
    "operation":
```

### Example 2

**Text:** [起動][ターン1回][⚡E][⚡E]手札を1枚控え室に置く：これにより控え室に置いたカードが『μ's』のカードの場合、自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く。『μ's』のカード以外の場合、自分の控え室からライブカードを1枚手札に加える。

**Trigger:** 起動 | **Use limit:** 1

**Cost:** [sequential_cost] [⚡E][⚡E]手札を1枚控え室に置く

**Effect (my best guess):**
Conditional alternative:
  Look then select:
    look_at: 自分のデッキの上からカードを4枚見る。
    Sequential (2 steps):
      Move 2× card from looked_at → hand
      Move 1× card from looked_at_remaining → discard
  Move 1× live_card from discard → hand

**Condition:**
group_condition: これにより控え室に置いたカードが『μ's』のカードの場合

**Raw effect keys:** ['action', 'alternative_effect', 'condition', 'count', 'primary_effect', 'text']
```json
{
  "text": "これにより控え室に置いたカードが『μ's』のカードの場合、自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く。『μ's』のカード以外の場合、自分の控え室からライブカードを1枚手札に加える",
  "action": "conditional_alternative",
  "condition": {
    "text": "これにより控え室に置いたカードが『μ's』のカードの場合",
    "location": "discard",
    "group": {
      "name": "μ's"
    },
    "type": "group_condition"
  },
  "primary_effect": {
    "text": "自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く",
    "action": "look_and_select",
    "look_action": {
      "text": "自分のデッキの上からカードを4枚見る。",
      "source": "deck_top",
      "count": 4,
      "target": "self",
      "action": "look
```

---

## appear (6 occurrences)

### Example 1

**Text:** [常時]能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Appear (search deck/discard → stage)

**Raw effect keys:** ['action', 'card_type', 'source', 'target', 'text']
```json
{
  "text": "能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る",
  "source": "hand",
  "card_type": "member_card",
  "target": "self",
  "action": "appear"
}
```

### Example 2

**Text:** [常時]コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Appear (search deck/discard → stage)

**Raw effect keys:** ['action', 'card_type', 'group', 'group_names', 'source', 'target', 'text']
```json
{
  "text": "コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る",
  "source": "hand",
  "card_type": "member_card",
  "target": "self",
  "group": {
    "name": "Liella!"
  },
  "group_names": [
    "Liella!"
  ],
  "action": "appear"
}
```

---

## gain_ability (5 occurrences)

### Example 1

**Text:** [自動][ターン1回]自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、[ハート]を得る。

**Trigger:** 自動 | **Use limit:** 1

**Effect (my best guess):**
Gain ability

**Condition:**
card_count_condition: 自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合

**Raw effect keys:** ['action', 'condition', 'count', 'duration', 'text']
```json
{
  "text": "{{icon_all.png|ハート}}を得る",
  "condition": {
    "type": "card_count_condition",
    "count": 3,
    "operator": ">=",
    "text": "自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合"
  },
  "action": "gain_ability",
  "count": 1,
  "duration": "live_end"
}
```

### Example 2

**Text:** [自動]［ターン1回］エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、［緑ハート］を得る。

**Trigger:** 自動 | **Use limit:** 1

**Effect (my best guess):**
Gain ability

**Condition:**
card_count_condition: エールにより公開された自分のカードの中にライブカードが1枚以上あるとき

**Raw effect keys:** ['action', 'condition', 'duration', 'text']
```json
{
  "text": "［緑ハート］を得る",
  "condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "エールにより公開された自分のカードの中にライブカードが1枚以上あるとき"
  },
  "action": "gain_ability",
  "duration": "live_end"
}
```

---

## __none__ (3 occurrences)

### Example 1

**Text:** (必要ハートを確認する時、エールで出た[🔰ALLブレード]は任意の色のハートとして扱う。)

**Trigger:** None | **Use limit:** None

**Effect (my best guess):**
?: 

**Raw effect keys:** []
```json
{}
```

### Example 2

**Text:** (エールで出た[+スコア]1つにつき、成功したライブのスコアの合計に1を加算する。)

**Trigger:** None | **Use limit:** None

**Effect (my best guess):**
?: 

**Raw effect keys:** []
```json
{}
```

---

## modify_yell_count (3 occurrences)

### Example 1

**Text:** [ライブ開始時]自分のステージにこのメンバー以外のメンバーが1人以上いる場合、ライブ終了時まで、エールによって公開される自分のカードの枚数が8枚減る。

**Trigger:** ライブ開始時 | **Use limit:** None

**Effect (my best guess):**
Modify yell count

**Condition:**
card_count_condition: 自分のステージにこのメンバー以外のメンバーが1人以上いる場合

**Raw effect keys:** ['action', 'condition', 'count', 'duration', 'operation', 'text']
```json
{
  "text": "自分のステージにこのメンバー以外のメンバーが1人以上いる場合、エールによって公開される自分のカードの枚数が8枚減る",
  "condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "自分のステージにこのメンバー以外のメンバーが1人以上いる場合",
    "unit": "人"
  },
  "action": "modify_yell_count",
  "operation": "subtract",
  "count": 8,
  "duration": "live_end"
}
```

### Example 2

**Text:** [ライブ開始時]ライブ終了時まで、エールによって公開される自分のカードが持つ[桃ブレード]、[赤ブレード]、[黄ブレード]、[緑ブレード]、[紫ブレード]、[🔰ALLブレード]は、すべて[青ブレード]になる。

**Trigger:** ライブ開始時 | **Use limit:** None

**Effect (my best guess):**
Modify yell count

**Raw effect keys:** ['action', 'duration', 'target', 'text']
```json
{
  "text": "エールによって公開される自分のカードが持つ[桃ブレード]、[赤ブレード]、[黄ブレード]、[緑ブレード]、[紫ブレード]、{{icon_b_all.png|ALLブレード}}は、すべて[青ブレード]になる",
  "duration": "live_end",
  "target": "self",
  "action": "modify_yell_count"
}
```

---

## activate_ability (2 occurrences)

### Example 1

**Text:** [起動][ターン1回]手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く：これにより控え室に置いたメンバーカードの[登場]能力1つを発動させる。
([登場]能力がコストを持つ場合、支払って発動させる。)

**Trigger:** 起動 | **Use limit:** 1

**Cost:** [move_cards] 手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く

**Effect (my best guess):**
Activate ability: これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力

**Raw effect keys:** ['action', 'count', 'parenthetical', 'target', 'target_trigger', 'text']
```json
{
  "text": "これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを発動させる。",
  "action": "activate_ability",
  "target": "これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力",
  "target_trigger": "toujyou.png|登場",
  "parenthetical": [
    "{{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。"
  ],
  "count": 1
}
```

### Example 2

**Text:** [登場]自分の控え室にあるコスト4以下の『虹ヶ咲』のメンバーカードを1枚選ぶ。そのカードの[登場]能力1つを発動させる。
（[登場]能力がコストを持つ場合、支払って発動させる。）

**Trigger:** 登場 | **Use limit:** None

**Effect (my best guess):**
Activate ability: 自分の控え室にあるコスト4以下の『虹ヶ咲』のメンバーカードを1枚選ぶ。そのカード

**Raw effect keys:** ['action', 'count', 'parenthetical', 'target', 'target_trigger', 'text']
```json
{
  "text": "自分の控え室にあるコスト4以下の『虹ヶ咲』のメンバーカードを1枚選ぶ。そのカードの{{toujyou.png|登場}}能力1つを発動させる。",
  "action": "activate_ability",
  "target": "自分の控え室にあるコスト4以下の『虹ヶ咲』のメンバーカードを1枚選ぶ。そのカードの{{toujyou.png|登場}}能力",
  "target_trigger": "toujyou.png|登場",
  "parenthetical": [
    "{{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。"
  ],
  "count": 1
}
```

---

## draw_until_count (1 occurrences)

### Example 1

**Text:** [自動]このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く。

**Trigger:** 自動 | **Use limit:** None

**Effect (my best guess):**
draw_until_count: 手札が5枚になるまでカードを引く

**Condition:**
temporal_condition: このターン、自分のステージにメンバーが3回登場したとき

**Raw effect keys:** ['action', 'condition', 'count', 'destination', 'source', 'target_count', 'text']
```json
{
  "text": "手札が5枚になるまでカードを引く",
  "condition": {
    "type": "temporal_condition",
    "temporal": "this_turn",
    "text": "このターン、自分のステージにメンバーが3回登場したとき",
    "count": 3,
    "location": "stage",
    "card_type": "member_card",
    "target": "self"
  },
  "count": 5,
  "action": "draw_until_count",
  "source": "deck",
  "destination": "hand",
  "target_count": 5
}
```

---

## play_baton_touch (1 occurrences)

### Example 1

**Text:** [常時]このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Baton touch allowed

**Raw effect keys:** ['action', 'count', 'optional', 'text']
```json
{
  "text": "このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい",
  "action": "play_baton_touch",
  "count": 2,
  "optional": true
}
```

---

## set_card_identity (1 occurrences)

### Example 1

**Text:** [常時]すべての領域にあるこのカードは『スリーズブーケ』、『DOLLCHESTRA』、『みらくらぱーく！』として扱う。

**Trigger:** 常時 | **Use limit:** None

**Effect (my best guess):**
Set card identity

**Raw effect keys:** ['action', 'all_regions', 'group', 'group_names', 'identities', 'text']
```json
{
  "text": "すべての領域にあるこのカードは『スリーズブーケ』、『DOLLCHESTRA』、『みらくらぱーく！』として扱う",
  "group": {
    "name": "スリーズブーケ"
  },
  "group_names": [
    "スリーズブーケ",
    "DOLLCHESTRA",
    "みらくらぱーく！"
  ],
  "action": "set_card_identity",
  "identities": [
    "スリーズブーケ",
    "DOLLCHESTRA",
    "みらくらぱーく！"
  ],
  "all_regions": true
}
```

---

## place_energy_under_member (1 occurrences)

### Example 1

**Text:** [登場]自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい。

**Trigger:** 登場 | **Use limit:** None

**Effect (my best guess):**
place_energy_under_member: 自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい

**Raw effect keys:** ['action', 'card_type', 'count', 'destination', 'energy_count', 'optional', 'target', 'text']
```json
{
  "text": "自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい",
  "destination": "under_member",
  "count": 2,
  "card_type": "member_card",
  "target": "self",
  "optional": true,
  "action": "place_energy_under_member",
  "energy_count": 2
}
```

---

## set_score (1 occurrences)

### Example 1

**Text:** [ライブ成功時]このターン、エールにより公開された自分のカードの中にブレードハートを持たないカードが0枚の場合か、または自分が余剰ハートを2つ以上持っている場合、このカードのスコアは４になる。

**Trigger:** ライブ成功時 | **Use limit:** None

**Effect (my best guess):**
set_score: 4

**Condition:**
comparison_condition: または自分が余剰ハートを2つ以上持っている場合

**Raw effect keys:** ['action', 'condition', 'text', 'value']
```json
{
  "text": "このカードのスコアは4になる",
  "condition": {
    "text": "または自分が余剰ハートを2つ以上持っている場合",
    "resource_type": "surplus_heart",
    "count": 2,
    "operator": ">=",
    "type": "comparison_condition"
  },
  "action": "set_score",
  "value": 4
}
```

---

## set_blade_count (1 occurrences)

### Example 1

**Text:** [ライブ開始時]ライブ終了時まで、自分のステージのセンターエリアにいる『Liella!』のメンバーが元々持つ[⚔ブレード]の数は3つになる。

**Trigger:** ライブ開始時 | **Use limit:** None

**Effect (my best guess):**
set_blade_count: 3

**Raw effect keys:** ['action', 'card_type', 'count', 'duration', 'group', 'group_names', 'position', 'target', 'text']
```json
{
  "text": "自分のステージのセンターエリアにいる『Liella!』のメンバーが元々持つ{{icon_blade.png|ブレード}}の数は3つになる",
  "duration": "live_end",
  "count": 3,
  "card_type": "member_card",
  "target": "self",
  "position": "center",
  "group": {
    "name": "Liella!"
  },
  "group_names": [
    "Liella!"
  ],
  "action": "set_blade_count"
}
```

---
