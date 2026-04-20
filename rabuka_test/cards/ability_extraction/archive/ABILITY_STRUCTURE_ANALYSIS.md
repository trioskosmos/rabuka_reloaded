# Ability Structure Analysis - Real Examples

## Example 1: Simple cost + effect
**Text:** `このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。`

**Structure:**
```
[cost] ： [effect] 。
```

**Cost analysis:**
- Target: このメンバー (this member)
- Source: ステージ (stage)
- Destination: 控え室 (discard)
- Action: 置く (place)

**Effect analysis:**
- Target: 自分の (self)
- Source: 控え室 (discard)
- Card type: ライブカード (live card)
- Count: 1枚 (1 card)
- Destination: 手札 (hand)
- Action: 加える (add)

**Grammar:**
- Cost: [target] [source]から [destination]に置く
- Effect: [target] [source]から [card_type] [count]枚 [destination]に加える

---

## Example 2: Optional cost + sequential action
**Text:** `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`

**Structure:**
```
[cost] ： [action1]。その後、[action2]、[action3]。
```

**Cost analysis:**
- Source: 手札 (hand)
- Count: 1枚 (1 card)
- Destination: 控え室 (discard)
- Optional: もよい (may)
- Action: 置く (place)

**Effect analysis:**
- Action 1: 見る (look at)
  - Source: 自分のデッキの上 (top of own deck)
  - Count: 3枚 (3 cards)
  
- Action 2: 加える (add)
  - Selection: その中から1枚 (from those, 1 card)
  - Destination: 手札 (hand)
  
- Action 3: 置く (place)
  - Target: 残り (the rest)
  - Destination: 控え室 (discard)

**Grammar:**
- Sequential marker: その後 (after that)
- Selection: その中から [count]枚 (from those, [count] cards)
- Remaining: 残り (the rest)

---

## Example 3: Simple sequential actions
**Text:** `カードを1枚引き、手札を1枚控え室に置く。`

**Structure:**
```
[action1]、[action2]。
```

**Action 1 analysis:**
- Object: カード (card)
- Count: 1枚 (1 card)
- Action: 引く (draw)
- Implied source: デッキ (deck)
- Implied destination: 手札 (hand)

**Action 2 analysis:**
- Source: 手札 (hand)
- Count: 1枚 (1 card)
- Destination: 控え室 (discard)
- Action: 置く (place)

**Grammar:**
- Sequential separator: 、 (comma)
- Implicit source/destination for draw: deck → hand

---

## Example 4: Duration modifier
**Text:** `{{icon_energy.png|E}}支払ってもよい：ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`

**Structure:**
```
[cost] ： [duration]、[action]。
```

**Cost analysis:**
- Payment: {{icon_energy.png|E}} (energy icon)
- Optional: もよい (may)
- Action: 支払う (pay)

**Effect analysis:**
- Duration: ライブ終了時まで (until end of live)
- Resource: {{icon_blade.png|ブレード}} (blade)
- Count: 2 (implied by 2 icons)
- Action: 得る (gain)

**Grammar:**
- Duration modifier: [time]まで (until [time])
- Icon repetition: count = number of icons

---

## Example 5: Complex condition
**Text:** `自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。`

**Structure:**
```
[condition1] かつ [condition2] 場合、[ability]を得る。
```

**Condition 1 analysis:**
- Target: 自分の (self)
- Location: ステージのエリアすべて (all stage areas)
- Group: 『蓮ノ空』 (Renkoku)
- Card type: メンバー (member)
- State: 登場しており (deployed)

**Condition 2 analysis:**
- Property: 名前 (name)
- Comparison: 異なる (different)

**Effect analysis:**
- Action: 得る (gain)
- Object: 「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」 (ability text)

**Grammar:**
- Compound condition: [condition1] かつ [condition2]
- State marker: おり (indicating state)
- Ability text in quotes: 「...」
- Ability gain: [ability]を得る

---

## Example 6: Cost limit condition
**Text:** `相手のステージにいるコスト4以下のメンバーを2人までウェイトにする。`

**Structure:**
```
[target] [location] [property] [operator] [value] [card_type] [count] [modifier] [action]。
```

**Analysis:**
- Target: 相手の (opponent)
- Location: ステージにいる (on stage)
- Property: コスト (cost)
- Operator: 以下 (less than or equal)
- Value: 4
- Card type: メンバー (member)
- Count: 2人 (2 people)
- Modifier: まで (up to)
- Action: ウェイトにする (to wait)

**Grammar:**
- Target + location: [target]の[location]にいる
- Property constraint: [property][operator][value]
- Count with max: [count][modifier]
- Action: [action]

---

## Example 7: Duration condition
**Text:** `このメンバーの下にエネルギーカードが2枚以上置かれているかぎり、ライブの合計スコアを＋１する。`

**Structure:**
```
[condition] かぎり、[action]。
```

**Condition analysis:**
- Location: このメンバーの下 (under this member)
- Card type: エネルギーカード (energy card)
- Count: 2枚 (2 cards)
- Operator: 以上 (greater than or equal)
- State: 置かれている (placed)
- Duration: かぎり (as long as)

**Action analysis:**
- Scope: ライブの (live's)
- Property: 合計スコア (total score)
- Operation: ＋１ (plus 1)
- Action: する (do)

**Grammar:**
- Duration condition: [condition] かぎり (as long as [condition])
- Score modification: [scope] [property] [operation]

---

## Example 8: Per-unit modifier
**Text:** `自分のステージにいるメンバー1人につき、カードを1枚引く。その後、これにより引いた枚数と同じ枚数を手札から控え室に置く。`

**Structure:**
```
[per-unit condition]、[action1]。その後、[variable reference action2]。
```

**Condition analysis:**
- Target: 自分の (self)
- Location: ステージにいる (on stage)
- Card type: メンバー (member)
- Count: 1人 (1 person)
- Per-unit marker: につき (per)

**Action 1 analysis:**
- Object: カード (card)
- Count: 1枚 (1 card)
- Action: 引く (draw)

**Action 2 analysis:**
- Sequential: その後 (after that)
- Variable reference: これにより引いた枚数と同じ枚数 (same count as drawn by this)
- Source: 手札から (from hand)
- Destination: 控え室に置く (place in discard)

**Grammar:**
- Per-unit: [count]につき (per [count])
- Variable reference: これにより [action] した [property]
- Dynamic count: [variable]と同じ枚数 (same count as [variable])

---

## Example 9: Choice with conditions
**Text:** `以下から1つを選ぶ。
・自分の控え室にカード名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。
・自分の控え室にグループが異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。`

**Structure:**
```
[choice marker]
・[condition] [action]
・[condition] [action]
```

**Choice marker:** 以下から1つを選ぶ (choose one from below)

**Option 1 analysis:**
- Condition:
  - Location: 自分の控え室 (own discard)
  - Property: カード名が異なる (different card names)
  - Card type: ライブカード (live card)
  - Count: 3枚以上 (3 cards or more)
  - Marker: 場合 (if)
- Action:
  - Source: 自分の控え室から (from own discard)
  - Card type: ライブカード (live card)
  - Count: 1枚 (1 card)
  - Destination: 手札に加える (add to hand)

**Option 2 analysis:**
- Condition:
  - Location: 自分の控え室 (own discard)
  - Property: グループが異なる (different groups)
  - Card type: ライブカード (live card)
  - Count: 3枚以上 (3 cards or more)
  - Marker: 場合 (if)
- Action: (same as option 1)

**Grammar:**
- Choice structure: [marker] followed by bullet points
- Each option: [condition] 場合、[action]
- Bullet points: ・ (Japanese bullet point)

---

## Example 10: Baton touch deploy
**Text:** `『Liella!』のメンバーからバトンタッチして登場しており、かつ自分のエネルギーが7枚以上ある場合、自分のエネルギーデッキから、エネルギーカードを2枚ウェイト状態で置く。`

**Structure:**
```
[condition1] かつ [condition2] 場合、[action]。
```

**Condition 1 analysis:**
- Group: 『Liella!』
- Card type: メンバー (member)
- Action: バトンタッチして登場 (baton touch deploy)
- State: おり (indicating state)

**Condition 2 analysis:**
- Target: 自分の (self)
- Object: エネルギー (energy)
- Count: 7枚 (7 cards)
- Operator: 以上 (greater than or equal)

**Action analysis:**
- Source: 自分のエネルギーデッキから (from own energy deck)
- Card type: エネルギーカード (energy card)
- Count: 2枚 (2 cards)
- State: ウェイト状態 (wait state)
- Action: 置く (place)

**Grammar:**
- Baton touch: [group]のメンバーからバトンタッチして登場
- State marker: おり (indicating completed state)
- Energy count: エネルギーが[count]枚
- State placement: [state]で置く (place in [state])

---

## Grammatical Patterns Summary

### Cost Structure
```
[cost] ： [effect]
```

Cost patterns:
- [target] [source]から [destination]に置く
- [source] [count]枚 [destination]に置いてもよい
- {{icon}} 支払ってもよい
- [target] [source]から [card_type] [count]枚 [destination]に置く

### Effect Structure
Simple action:
```
[verb] [object] [modifiers]
```

Sequential actions:
```
[action1]、[action2]
[action1]。その後、[action2]
```

Choice:
```
以下から1つを選ぶ
・[option1]
・[option2]
```

### Condition Structure
Simple condition:
```
[condition] 場合、[action]
[condition] とき、[action]
```

Compound condition:
```
[condition1] かつ [condition2] 場合、[action]
```

Duration condition:
```
[condition] かぎり、[action]
```

### Condition Components
Target: 自分の, 相手の, 自分と相手の
Location: 控え室, 手札, ステージ, デッキ
Card type: メンバーカード, ライブカード, エネルギーカード
Group: 『group name』
Property: コスト, スコア, ブレード, ハート
Operator: 以上, 以下, より少ない, より多い
Value: number
State: ウェイト状態, アクティブ状態, 登場している

### Action Components
Verb: 引く, 置く, 選ぶ, 発動させる, 得る, 加える, 登場させる
Source: [location]から
Destination: [location]に
Count: [number]枚, [number]人
Modifier: まで, のみ, ずつ, つき
