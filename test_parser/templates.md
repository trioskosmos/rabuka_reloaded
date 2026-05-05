# Atomic Ability Templates

Each template is a parameterized sentence pattern with slots.
The output is `{template_id, slots}` which IS the ability — invertible.

## draw_card (1 template, 88 uses)

Template: `{count}枚引く`
Slots: `count: u32` (default: 1)
Defaults: `{source: "deck", destination: "hand"}`

Texts:
- カードを1枚引く → `{count: 1}`
- カードを2枚引く → `{count: 2}`

## move_cards — source→destination (7 sentence patterns)

### T1: 加える (add to hand)
Template: `{target}の{source}から{cost_limit?}の{group?}{card_type}を{count}枚{destination}に加える`
Slots: `target: str, source: str, destination: str, count: u32`,
       `?card_type: str, ?group: str, ?cost_limit: u32, ?max: bool`
Texts:
- 自分の控え室からライブカードを1枚手札に加える → `{target:self, source:discard, card_type:live_card, count:1, destination:hand}`
- 自分の控え室から4コスト以下の『A-RISE』のメンバーカードを1枚手札に加える → `{target:self, source:discard, cost_limit:4, group:A-RISE, card_type:member_card, count:1, destination:hand}`
- 相手の控え室からカードを1枚手札に加える → `{target:opponent, source:discard, card_type:card, count:1, destination:hand}`

### T2: 置く (place to zone)
Template: `{target}の{source}から{group?}{card_type}を{count}枚{destination}に{state?}置く`
Slots: `?target, source, destination, count, ?card_type, ?group, ?state, ?max`
Texts:
- 自分のエネルギーデッキからエネルギーカードを1枚ウェイト状態で置く → `{target:self, source:energy_deck, card_type:energy_card, count:1, destination:energy_zone, state:wait}`
- 自分の控え室からライブカードを1枚までデッキの一番下に置く → `{target:self, source:discard, card_type:live_card, count:1, destination:deck_bottom, max:true}`

### T3: 登場させる (deploy to stage)
Template: `{target}の{source}から{card_type}を{count}枚{destination}に登場させる`
Slots: `?target, source, destination:stage, count, card_type`
Texts:
- 自分の控え室からメンバーカードを1枚ステージに登場させる → `{target:self, source:discard, card_type:member_card, count:1, destination:stage}`

### T4: 送る (send to discard)
Template: `{target}の{card_type}を{destination}に送る`
Slots: `?target, ?card_type, destination:discard`
Texts:
- 相手のメンバーを控え室に送る → `{target:opponent, card_type:member, destination:discard}`

### T5: 戻す (return to deck)
Template: `{target}の{source}から{card_type}を{count}枚{destination}に戻す`
Slots: `?target, source, destination, count, ?card_type`
Texts:
- 自分の控え室からライブカードを1枚デッキに戻す → `{target:self, source:discard, card_type:live_card, count:1, destination:deck}`

### T6: 手札を控え室に置く (discard from hand — cost pattern)
Template: `手札を{count}枚控え室に置く`
Slots: `count: u32`
Texts:
- 手札を1枚控え室に置く → `{count: 1}`
- 手札を2枚控え室に置く → `{count: 2}`

### T7: デッキの上から見る (look at deck top)
Template: `{target}の{source}からカードを{count}枚見る`
Slots: `?target, source:deck_top, count: u32`
Texts:
- 自分のデッキの上からカードを3枚見る → `{target:self, source:deck_top, count:3}`

## gain_resource (2 templates)

### G1: 得る (gain)
Template: `{resource}を得る`
Slots: `resource: str, ?count: u32, ?duration: str, ?per_unit, ?per_unit_count`
Texts:
- {{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る → `{resource:blade, count:2}`
- {{heart_01.png|heart01}}を得る → `{resource:heart, count:1}`

### G2: 指定する (specify/heart selection)
Template: `好きな{noun}を{count}つ指定する`
Slots: `count: u32`
Texts:
- 好きなハートの色を1つ指定する → `{count:1}`

## change_state (2 templates)

### C1: アクティブにする (activate)
Template: `{?target}の{?card_type}{?count}人{?max}をアクティブにする`
Slots: `?target, ?card_type, ?count, ?max, state_change:active`
Texts:
- エネルギーを2枚アクティブにする → `{card_type:energy, count:2, state_change:active}`
- 相手のステージにいるコスト4以下のメンバー1人をアクティブにする → `{target:opponent, card_type:member, count:1, cost_limit:4, state_change:active}`

### C2: ウェイトにする (wait)
Template: `{?target}の{?card_type}{?count}人{?max}をウェイトにする`
Slots: same as C1 but state_change:wait
Texts:
- 相手のステージにいるコスト4以下のメンバー1人をウェイトにする → `{target:opponent, card_type:member, count:1, cost_limit:4, state_change:wait}`

## modify_score (2 templates)

### M1: スコアを+ (add to score)
Template: `{?target}の{?scope}スコアを{operation}{value}する`
Slots: `operation:add, ?value, ?target, ?scope`
Texts:
- ライブの合計スコアを+1する → `{operation:add, value:1, scope:total}`
- このカードのスコアを+1する → `{operation:add, value:1, target:self}`

### M2: スコアになる (set score)
Template: `{?target}のスコアを{value}になる`
Slots: `?target, value`
Texts:
- このカードのスコアを4になる → `{target:self, value:4}`

## reveal (1 template)

### R1: 公開する
Template: `{?card_type}{?count}枚{?max}公開する`
Slots: `?card_type, ?count, ?max`
Texts:
- メンバーカードを3枚まで公開する → `{card_type:member, count:3, max:true}`
- ライブカードを1枚公開する → `{card_type:live, count:1}`

## select (2 templates)

### S1: 選ぶ
Template: `{?source}から{?card_type}を{count}枚{?条件}選ぶ`
Slots: `?source, ?card_type, ?count`
Texts:
- 自分の控え室からライブカードを1枚選ぶ → `{source:discard, card_type:live, count:1}`
- その中から1枚を選ぶ → `{source:looked_at, count:1}`

## restriction (1 template per restriction type)

### R1: cannot patterns
Template: fixed strings per restriction_type
Texts:
- ライブできない → `{type:cannot_live}`
- アクティブにしない → `{type:cannot_activate}`
- 置くことができない → `{type:cannot_place}`
- etc.

## Position change (1 template)

Template: `{?target}を{?source_area}から{?dest_area}にポジションチェンジする`
Slots: `?target, ?source_area, ?dest_area`
Texts:
- このメンバーをポジションチェンジしてもよい → `{target:self}`
- 自分のステージのセンターにいるメンバーを左サイドに移動させる → `{target:self, source_area:center, dest_area:left_side}`

## modify_cost (1 template)

Template: `{target}の{source}から{card_type}を登場させるためのコストは{value}減る`
Or simpler: コストは{value}{operation}
Slots: `value, operation, ?target, ?source, ?card_type`
Texts:
- コストは1減る → `{value:1, operation:decrease}`
- コストは+4増える → `{value:4, operation:increase}`

## gain_ability (1 template)

Template: `「{ability_text}」を得る`
Slots: `ability_text`
Texts:
- 「常時ライブの合計スコアを+1する」を得る → `{ability_text:常時ライブの合計スコアを+1する}`

## modify_required_hearts (1 template)

Template: `{?target}の必要ハートを{?heart_color}{count}{operation}`
Slots: `count, operation:decrease/increase, ?target, ?heart_color`
Texts:
- このカードを成功させるための必要ハートを減らす → `{target:self, operation:decrease}`
- 必要ハートが{heart00}多くなる → `{operation:increase, heart_color:heart00}`

## Composition templates (how atomics combine)

### CP1:  sequential (A、B or A。B)
Template: `{action1}、{action2}` or `{action1}。{action2}`
Used: 145 times

### CP2: conditional (X場合、A)
Template: `{condition}場合、{action}`
Used: ~90 times

### CP3: conditional_sequential (A。そうした場合、B)
Template: `{action1}。そうした場合、{action2}`
Used: ~20 times

### CP4: per_unit (Xにつき、A)
Template: `{per_unit_reference}1{unit}につき、{action}`
Used: ~56 times

### CP5: choice (以下から1つを選ぶ)
Template: `以下から1つを選ぶ。\n・{option1}\n・{option2}`
Used: 9 times

### CP6: sequential_marker (A。その後、B)
Template: `{action1}。その後、{action2}`
Used: ~15 times

### CP7: each_time (Xたび、A)
Template: `{trigger}たび、{action}`
Used: ~12 times

### CP8: conditional_alternative (A。代わりに、B)
Template: `{condition}代わりに、{action_b}`
Used: 6 times
