# Inverted Abilities Index (Condensed)

Source: abilities.json (762 unique abilities)

**Total unique JSON fingerprints: 2258**

---

```json
{"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札を1枚控え室に置いてもよい (x63)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}
```

- カードを1枚引く (x15)
- カードを1枚引き (x14)

```json
{"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを5枚見る。 (x28)

```json
{"reference": "previous_reveal", "type": "revealed_cards"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}
```

- 手札を1枚控え室に置く (x23)

```json
{"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}支払ってもよい (x20)
- {icon_energy.png|E} (x1)
- {icon_energy.png|E}を2つまで支払ってもよい (x1)

```json
{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E} (x20)
- {icon_energy.png|E}{icon_energy.png|E}支払う (x1)

```json
{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}
```

- カードを2枚引き (x16)
- カードを2枚引く (x2)

```json
{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}
```

- このメンバーをウェイトにしてもよい (x11)
- このメンバーをウェイトにし (x6)

```json
{"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札を1枚控え室に置く (x17)

```json
{"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E}支払ってもよい (x15)
- {icon_energy.png|E}{icon_energy.png|E} (x1)

```json
{"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}
```

- このメンバーをステージから控え室に置く (x11)
- 、このメンバーをステージから控え室に置く (x1)

```json
{"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを2枚見る。 (x12)

```json
{"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}
```

- このメンバーをウェイトにする (x10)
- このメンバーをウェイトにし (x2)

```json
{"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札を2枚控え室に置いてもよい (x9)
- 手札を2枚まで控え室に置いてもよい (x2)
- 手札の同じグループ名を持つカード2枚を控え室に置いてもよい (x1)

```json
{"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを3枚見る。 (x10)
- 自分のデッキの上からカードを3枚見る (x1)

```json
{"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}
```

- このメンバーがエリアを移動したとき (x9)
- このメンバーが登場か、エリアを移動したとき (x1)

```json
{"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを4枚見る。 (x10)

```json
{"quoted_type": "character"}
```

- ミア・テイラー (x2)
- 中須かすみ (x2)
- DIVE! (x1)
- 上原歩夢 (x1)
- 優木せつ菜 (x1)
- 安養寺姫芽 (x1)
- 宮下愛 (x1)
- 桜坂しずく (x1)

```json
{"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}
```


```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}
```

- {icon_blade.png|ブレード}を得る (x9)

```json
{"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}
```

- このメンバーがステージから控え室に置かれたとき (x8)

```json
{"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札を2枚控え室に置く (x8)

```json
{"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合 (x6)
- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上であるかぎり (x2)

```json
{"type": "has_moved"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードを1枚手札に加える (x5)
- 自分の控え室から、スコア6以上のライブカードを1枚手札に加える (x1)
- 自分の控え室にあるライブカードを1枚手札に加える (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の控え室からメンバーカードを1枚手札に加える (x5)
- 自分の控え室から、これにより控え室に置いたメンバーカードより、コストの低いメンバーカードを1枚手札に加える (x1)
- 自分の控え室からメンバーカードを1枚手札に加える。 (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}
```

- カードを1枚引き、手札を1枚控え室に置く (x6)
- カードを1枚引き、手札を1枚控え室に置く。 (x1)

```json
{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x6)
- 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。 (x1)

```json
{"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}
```

- {icon_energy.png|E}{icon_energy.png|E}手札を1枚控え室に置く (x7)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}
```


```json
{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}
```

- 手札を2枚控え室に置く (x6)

```json
{"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E} (x6)

```json
{"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}
```

- このメンバーをウェイトにし、手札を1枚控え室に置いてもよい (x6)

```json
{"appearance": true, "location": "stage", "type": "appearance_condition"}
```

- 控え室から登場している場合 (x4)
- このメンバーが手札以外からステージに登場している場合 (x1)
- これにより登場したメンバーがブレードハートを持つ場合 (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}
```

- 手札を1枚控え室に置いてもよい。 (x3)
- そのカードを控え室に置いてもよい (x1)
- 手札を1枚控え室に置いてもよい (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを3枚控え室に置く (x4)
- 自分のデッキの上からカードを3枚控え室に置く。 (x1)

```json
{"state": "wait", "type": "state_condition"}
```

- 相手のステージにウェイト状態のメンバーがいる場合 (x2)
- このターン、自分の『虹ヶ咲』のカードの効果によってウェイト状態の自分のエネルギーをアクティブにしていた場合 (x1)
- このメンバーがウェイト状態であるかぎり (x1)
- 自分のステージにウェイト状態の『虹ヶ咲』のメンバーがいるかぎり (x1)

```json
{"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の成功ライブカード置き場にカードが2枚以上ある場合 (x5)

```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x4)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える (x4)

```json
{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}
```

- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_06.png|heart06}のうち、1つを選ぶ (x4)

```json
{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}
```

- 好きなハートの色を1つ指定する (x4)

```json
{"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージにほかのメンバーがいる場合 (x2)
- 自分のステージにこのメンバー以外のメンバーがいる場合 (x1)
- 自分のステージにほかのメンバーがおり、 (x1)

```json
{"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- エールにより公開された自分のカードの中にライブカードが1枚以上あるとき (x2)
- エールにより公開された自分のカードの中に{icon_score.png|スコア}を持つライブカードが1枚以上ある場合 (x1)
- エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、 (x1)

```json
{"action": "modify_score", "operation": "add", "value": 2}
```

- スコアを+2する (x3)
- 合計スコアを+2する。」を得る (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}
```

- 自分の控え室から『μ's』のライブカードを1枚手札に加える (x3)
- 自分の控え室から『μ's』のライブカード1枚を手札に加える (x1)

```json
{"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーが7枚以上ある場合 (x4)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}
```

- {heart_01.png|heart01}を得る (x2)

```json
{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}
```

- ライブの合計スコアが相手より高い場合 (x4)

```json
{"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}], "type": "sequential_cost"}
```

- {icon_energy.png|E}{icon_energy.png|E}このメンバーをステージから控え室に置く (x3)
- {icon_energy.png|E}{icon_energy.png|E}、このメンバーをステージから控え室に置く (x1)

```json
{"action": "position_change", "card_type": "member_card"}
```

- このメンバーをそのエリアに移動する (x2)
- このメンバーはポジションチェンジする (x1)
- このメンバーをポジションチェンジする (x1)

```json
{"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}
```

- 自分のステージに『EdelNote』のメンバーが登場したとき (x1)
- 自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき (x1)
- 自分のステージにほかの『スリーズブーケ』のメンバーが登場する (x1)
- 自分のステージにコスト10のメンバーが登場したとき (x1)

```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを2枚見る。その中から「天王寺璃奈」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)
- 自分のデッキの上からカードを2枚見る。その中から「朝香果林」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)
- 自分のデッキの上からカードを2枚見る。その中から「近江彼方」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)
- 自分のデッキの上からカードを2枚見る。その中から「鐘嵐珠」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに、このターン中にバトンタッチして登場した『蓮ノ空』のメンバーが2人以上いる場合 (x3)
- 自分のステージにコスト10以上の『蓮ノ空』のメンバーが2人以上いる場合 (x1)

```json
{"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}
```

- このメンバーが元々持つハートは選んだハートになる (x4)

```json
{"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}
```

- このターン、このメンバーがエリアを移動している場合 (x1)
- このターン、自分のステージにいるほかのメンバーがエリアを移動している場合 (x1)
- このターンに自分のステージにいるメンバーがエリアを移動している場合 (x1)
- 自分のステージにいる『Liella!』のメンバーがこのターンにエリアを移動しているかぎり (x1)

```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}
```

- 自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く (x2)
- 自分のデッキの上からカードを3枚見る。その中から1枚を手札に加える。残りを控え室に置く (x1)

```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}
```

- 自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く (x3)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "heart"}
```

- そのハートを1つ得る (x3)

```json
{"action": "change_state", "card_type": "energy_card", "count": 2, "state_change": "active"}
```

- エネルギーを2枚アクティブにする (x3)

```json
{"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}
```

- 自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、 (x1)
- 自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、 (x1)
- 自分のステージのエリアすべてにメンバーが登場している場合 (x1)

```json
{"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}
```

- 名前が異なる場合 (x2)
- それぞれ名前が異なる場合 (x1)

```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}
```

- 自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く (x3)

```json
{"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}
```

- 自分の成功ライブカード置き場にカードがある場合 (x2)
- このカードが自分の成功ライブカード置き場にあり、 (x1)

```json
{"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}
```

- この能力はセンターエリアに登場している場合のみ起動できる。 (x2)
- この能力はセンターエリアに登場した場合のみ発動する。 (x1)

```json
{"card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札のライブカードを1枚控え室に置いてもよい (x3)

```json
{"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを5枚控え室に置く (x3)

```json
{"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーが10枚以上あるかぎり (x2)
- 自分のエネルギーが10枚以上ある場合 (x1)

```json
{"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}
```

- このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合 (x1)
- それがブレードハートを持たないメンバーカードの場合 (x1)
- コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合 (x1)

```json
{"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるコスト4以下のメンバー1人をウェイトにする (x2)
- 相手のステージにいるコスト4以下のメンバー1人をウェイトにする。 (x1)

```json
{"action": "change_state", "card_type": "energy_card", "count": 1, "state_change": "active"}
```

- エネルギーを1枚アクティブにする (x2)
- エネルギーを1枚アクティブにする。 (x1)

```json
{"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のライブカード置き場にカードが2枚以上ある場合 (x3)

```json
{"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}
```

- これにより相手が余剰ハートを2つ以上失っている場合 (x1)
- または自分が余剰ハートを2つ以上持っている場合 (x1)
- 自分が余剰ハートを2つ以上持つ場合 (x1)

```json
{"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室にカード名が異なるライブカードが3枚以上ある場合 (x1)
- 自分の控え室にグループ名が異なるライブカードが3枚以上ある場合 (x1)
- 自分の控え室にライブカードが3枚以上ある場合 (x1)

```json
{"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}
```

- 自分のステージにほかのメンバーがいないかぎり (x2)
- 自分のステージにほかのメンバーがいない場合 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}
```

- 自分の控え室から『Aqours』のライブカードを1枚手札に加える (x2)
- 自分の控え室から{icon_score.png|スコア}を持つ『Aqours』のライブカードを1枚手札に加える (x1)

```json
{"card_type": "member_card", "comparison_type": "cost", "cost_limit": 13, "cost_total": 13, "count": 13, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにコスト13以上のメンバーがいる場合 (x2)
- 自分のステージにコスト13以上のメンバーがいるかぎり (x1)

```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart"}], "heart_colors": ["heart01", "heart03", "heart06"]}
```

- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、選んだハートを1つ得る (x2)
- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、選んだハートを1つ得る。" (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart"}
```

- 選んだハートを1つ得る (x3)

```json
{"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにいるメンバーのコストの合計が相手より低い場合 (x2)
- 自分のステージにいるメンバーのコストの合計が相手より低いかぎり (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室から『蓮ノ空』のカードを1枚手札に加える (x3)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}
```

- カードを1枚引き (x2)
- カードを1枚引く (x1)

```json
{"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージに『蓮ノ空』のメンバーがいる場合 (x3)

```json
{"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}
```

- heart00×1 (x3)

```json
{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}
```

- 自分か相手の成功ライブカード置き場にカードが2枚以上あり、 (x2)
- 自分か相手の成功ライブカード置き場にカードが2枚以上ある場合 (x1)

```json
{"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるコスト4以下のメンバー1人をウェイトにする (x2)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}
```

- カードを2枚引き、手札を1枚控え室に置く (x2)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}
```

- 自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る (x2)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}]}
```

- カードを2枚引き、手札を2枚控え室に置く (x2)

```json
{"count": 3, "energy": 3, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E} (x2)

```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "heart"}]}
```

- 好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る (x2)

```json
{"count": 3, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のライブ中のカードが3枚以上 (x1)
- 自分のライブ中のカードが3枚以上ある場合 (x1)

```json
{"count": 11, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーが11枚以上ある場合 (x2)

```json
{"card_type": "member_card", "conditions": [{"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合 (x1)
- 自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合 (x1)

```json
{"action": "look_at", "count": 7, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを7枚見る。 (x2)

```json
{"card_type": "live_card", "count": 1, "optional": true, "source": "hand", "type": "reveal", "zone": "hand"}
```

- 手札のライブカードを1枚公開し (x1)
- 手札のライブカードを1枚公開してもよい (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}
```

- 自分の控え室から『Liella!』のカードを1枚手札に加える (x2)

```json
{"card_type": "member_card", "comparison_target": "self", "comparison_type": "cost", "location": "stage", "operator": ">", "target": "self", "type": "comparison_condition"}
```

- 自分のステージに、このメンバーよりコストが高いメンバーがいる場合 (x1)
- 自分のステージに、このメンバーよりコストの大きいメンバーがいる場合 (x1)

```json
{"costs": [{"card_type": "member_card", "position": "center", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "position": "center", "type": "sequential_cost"}
```

- {center.png|センター}このメンバーをウェイトにし、手札を1枚控え室に置く (x2)

```json
{"card_type": "member_card", "position": "center", "self_cost": true, "state_change": "wait", "type": "change_state"}
```

- {center.png|センター}このメンバーをウェイトにし (x2)

```json
{"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "optional": true, "target": "self"}
```

- 自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。 (x2)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "exclude_self": true, "state_change": "active", "target": "self"}
```

- 自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。 (x1)
- 自分のステージにいるほかのメンバー1人をアクティブにする (x1)

```json
{"ability_filter": "no_ability_type", "ability_filter_triggers": ["live_start", "live_success"], "type": "ability_filter_condition"}
```

- 自分のライブ中のライブカードに、{live_start.png|ライブ開始時}能力も{live_success.png|ライブ成功時}能力も持たないカードがあるかぎり (x1)
- 自分のライブ中のライブカードに、{live_start.png|ライブ開始時}能力も{live_success.png|ライブ成功時}能力も持たないカードがある場合 (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}
```

- それらを好きな順番でデッキの上に置く (x2)

```json
{"action": "position_change", "card_type": "member_card", "optional": true, "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}
```

- このメンバーをポジションチェンジしてもよい (x2)

```json
{"action": "select_cards", "count": 2, "destination": "hand", "discard_remaining": true}
```


```json
{"count": 4, "energy": 4, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E} (x2)

```json
{"reference": "unit_count", "type": "per_unit"}
```


```json
{"card_type": "member_card", "count": 1, "destination": "under_member", "type": "custom"}
```

- エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く (x2)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}
```

- {heart_02.png|heart02}を得る (x2)

```json
{"count": 3, "destination": "discard", "source": "deck_top", "type": "move_cards", "zone": "deck_top"}
```

- デッキの上からカードを3枚控え室に置く (x2)

```json
{"count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれる (x1)
- 自分の手札からカードが1枚以上控え室に置かれる (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 4, "destination": "discard", "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを4枚控え室に置く (x2)

```json
{"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}
```

- それらの中にライブカードがある場合 (x2)

```json
{"card_type": "live_card", "count": 1, "source": "hand", "type": "reveal", "zone": "hand"}
```

- 手札のライブカードを1枚公開する (x2)

```json
{"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "blade"}
```


```json
{"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}
```


```json
{"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "reveal": false}
```


```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}
```

- ライブ終了時まで、{heart_05.png|heart05}を得る (x1)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "wait"}
```

- このメンバーをウェイトにし (x1)
- このメンバーをウェイトにする (x1)

```json
{"costs": [{"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}
```

- このメンバーをウェイトにし、手札を1枚控え室に置く (x2)

```json
{"card_type": "live_card", "location": "revealed_cards", "negation": true, "type": "location_condition"}
```

- これにより公開されたカードの中にライブカードがない場合 (x1)
- これにより公開した手札の中にライブカードがない場合 (x1)

```json
{"count": 6, "energy": 6, "optional": true, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}支払ってもよい (x2)

```json
{"card_type": "live_card", "location": "discard", "type": "location_condition"}
```

- これによりライブカードを控え室に置いた場合 (x1)
- これにより控え室に置いたカードがライブカードの場合 (x1)

```json
{"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}
```


```json
{"action": "move_cards", "activation_condition_parsed": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}
```

- 自分の控え室から『μ's』のライブカードを1枚手札に加える (x2)

```json
{"baton_touch_trigger": true, "comparison_type": "cost", "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}
```

- このメンバーよりコストが低いメンバーからバトンタッチして登場した場合 (x2)

```json
{"count": 1, "destination": "discard", "group_names": ["DOLLCHESTRA"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の『DOLLCHESTRA』のカードを1枚控え室に置いてもよい (x2)

```json
{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを2枚控え室に置く (x2)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 15, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト15以下の『蓮ノ空』のメンバーカードを1枚、このメンバーがいたエリアに登場させる (x2)

```json
{"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}
```

- それらがすべてメンバーカードの場合 (x2)

```json
{"count": 12, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーが12枚以上ある場合 (x2)

```json
{"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}, "destination": "same_area"}
```

- 選んだエリアにメンバーがいる場合、そのメンバーは、このメンバーがいたエリアに移動させる (x2)

```json
{"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}
```

- 選んだエリアにメンバーがいる場合 (x2)

```json
{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}
```

- 自分の成功ライブカード置き場のカードが0枚で、 (x2)

```json
{"action": "position_change", "card_type": "member_card", "multiple_targets": true, "optional": true, "target": "self"}
```

- 自分のステージにいるメンバーを、それぞれ好きなエリアに移動させてもよい (x1)
- 自分のステージにいるメンバーをフォーメーションチェンジしてもよい (x1)

```json
{"action": "do_nothing"}
```


```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "duration": "live_end"}
```

- 「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x2)

```json
{"comparison_target": "self", "operator": ">", "resource_type": "energy", "target": "opponent", "type": "comparison_condition"}
```

- 相手のエネルギーが自分より多い場合 (x2)

```json
{"action": "look_at", "count": 2, "source": "deck_top"}
```

- デッキの上のカードを2枚見る。 (x1)
- 自分か相手を選ぶ。自分は、そのプレイヤーのデッキの上からカードを2枚見る。 (x1)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "max": true, "state_change": "active", "target": "self"}
```

- 自分のステージにいるメンバーを1人までアクティブにする (x2)

```json
{"action": "draw_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 13, "cost_total": 13, "count": 13, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のステージにコスト13以上のメンバーがいる場合、カードを1枚引く (x2)

```json
{"action": "draw_card", "count": 3, "destination": "hand", "source": "deck"}
```

- カードを3枚引く (x2)

```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_type": "discard", "resource": "blade"}
```

- これによって控え室に置いたカード1枚につき、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)
- これによって控え室に置いたカード1枚につき、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る。" (x1)

```json
{"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "source": "discard", "target": "self"}, {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "target": "self"}], "conditional": true, "target": "self"}], "conditional": true}
```

- 自分か相手を選ぶ。自分は、そのプレイヤーの控え室にあるライブカードを1枚、そのプレイヤーのデッキの一番下に置く。そうした場合、自分はカードを1枚引く (x2)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "source": "discard", "target": "self"}, {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "target": "self"}], "conditional": true, "target": "self"}
```

- 自分は、そのプレイヤーの控え室にあるライブカードを1枚、そのプレイヤーのデッキの一番下に置く。そうした場合、自分はカードを1枚引く (x2)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "source": "discard", "target": "self"}
```

- 自分は、そのプレイヤーの控え室にあるライブカードを1枚、そのプレイヤーのデッキの一番下に置く。 (x2)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "target": "self"}
```

- 自分はカードを1枚引く (x2)

```json
{"action": "look_at", "count": 6, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを6枚見る。 (x2)

```json
{"action": "draw_until_count", "condition": {"card_type": "member_card", "count": 3, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "count": 5, "destination": "hand", "source": "deck", "target_count": 5}
```

- このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く (x2)

```json
{"card_type": "member_card", "count": 3, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}
```

- このターン、自分のステージにメンバーが3回登場したとき (x2)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "duration": "live_end"}
```

- このターン、自分のステージにメンバーが2回以上登場している場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)
- このターン、自分のステージにメンバーが2回以上登場している場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る。" (x1)

```json
{"card_type": "member_card", "count": 2, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}
```

- このターン、自分のステージにメンバーが2回以上登場している場合 (x2)

```json
{"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}
```

- そのメンバーが持つハートと、このメンバーが持つハートの中に同じ色のハートがある場合 (x1)
- それぞれのメンバーのコストが同じ場合 (x1)

```json
{"count": 2, "distinct": "card_name", "group_names": ["BiBi"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージに名前の異なる『BiBi』のメンバーが2人以上いる場合 (x2)

```json
{"card_type": "live_card", "group_names": ["lilywhite"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分の成功ライブカード置き場に『lilywhite』のカードがあるかぎり (x1)
- 自分の成功ライブカード置き場に『lilywhite』のカードがある場合 (x1)

```json
{"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 3, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が3以上の場合 (x2)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}
```


```json
{"action": "change_state", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}
```

- 自身のステージにいるアクティブ状態のメンバー1人をウェイトにする (x2)

```json
{"action": "look_at", "count": 1, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを1枚見る (x1)
- 自分は、そのプレイヤーのデッキの一番上のカードを見る (x1)

```json
{"all_members": true, "card_type": "member_card", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーが『Liella!』のみで、 (x1)
- 自分のステージにいるメンバーが『Liella!』のみの場合 (x1)

```json
{"distinct": "card_name", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "location_condition"}
```

- エールにより公開された自分のカードの中に、名前が異なる『Liella!』のメンバーカードが3枚以上ある場合 (x1)
- エールにより公開された自分のカードの中に名前が異なる『Liella!』のメンバーカードが5枚以上ある場合 (x1)

```json
{"destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}
```

- このカードを手札から控え室に置く (x2)

```json
{"check_self": true, "count": 1, "location": "hand", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- このカードが手札にある場合 (x2)

```json
{"action": "reveal", "count": 1, "source": "deck_top", "target": "self"}
```

- 自分のデッキの一番上のカードを公開する (x2)

```json
{"target": "self", "temporal": "during_live", "type": "temporal_condition"}
```

- 自分のライブ中のカードにスコア2以下のライブカードがある場合 (x1)
- 自分のライブ中のライブカードの必要ハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}がそれぞれ1以上含まれるかぎり (x1)

```json
{"action": "change_state", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるコスト9以下のメンバー1人をウェイトにする (x2)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}
```

- {heart_04.png|heart04}を得る (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "blade"}
```


```json
{"action": "gain_resource", "activation_position": "center", "all": true, "card_type": "member_card", "condition": {"card_type": "live_card", "group_names": ["µ's"], "location": "live_card_zone", "position": "center", "target": "self", "type": "group_condition"}, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}
```

- 自分のライブカード置き場に『µ's』のカードがある場合、ライブ終了時まで、自分のステージにいるすべての『μ's』のメンバーは{icon_blade.png|ブレード}を得る (x2)

```json
{"card_type": "live_card", "group_names": ["µ's"], "location": "live_card_zone", "position": "center", "target": "self", "type": "group_condition"}
```

- {center.png|センター}自分のライブカード置き場に『µ's』のカードがある場合 (x2)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "under_member", "group_names": ["μ's"], "optional": true, "position": "center"}, {"action": "sequential", "actions": [{"action": "gain_resource", "activation_position": "center", "count": 1, "heart_selection": true, "position": "center", "resource": "heart"}, {"action": "gain_resource", "activation_position": "center", "count": 1, "duration": "live_end", "position": "center", "resource": "heart"}], "activation_position": "center", "group_names": ["μ's"], "position": "center"}], "activation_position": "center", "conditional": true, "group_names": ["μ's"], "position": "center"}
```

- 手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、このメンバーの下に置いてもよい。そうした場合、好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る (x2)

```json
{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "under_member", "group_names": ["μ's"], "optional": true, "position": "center"}
```

- 手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、このメンバーの下に置いてもよい。 (x2)

```json
{"action": "sequential", "actions": [{"action": "gain_resource", "activation_position": "center", "count": 1, "heart_selection": true, "position": "center", "resource": "heart"}, {"action": "gain_resource", "activation_position": "center", "count": 1, "duration": "live_end", "position": "center", "resource": "heart"}], "activation_position": "center", "group_names": ["μ's"], "position": "center"}
```

- 好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る (x2)

```json
{"action": "gain_resource", "activation_position": "center", "count": 1, "heart_selection": true, "position": "center", "resource": "heart"}
```

- 好きなハートの色を1つ指定する (x2)

```json
{"action": "gain_resource", "activation_position": "center", "count": 1, "duration": "live_end", "position": "center", "resource": "heart"}
```

- そのハートを1つ得る (x2)

```json
{"action": "place_energy_under_member", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "energy_count": 1, "group_names": ["μ's"], "optional": true, "source": "under_member", "target_member": "this_member"}
```

- このメンバーの下にあるコスト2以下の『μ's』のメンバーカードを1枚、メンバーのいないエリアに登場させてもよい (x1)
- このメンバーの下にあるコスト2以下の『μ's』のメンバーカードを1枚、メンバーのいないエリアに登場させてもよい。" (x1)

```json
{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "locations": ["live_card_zone", "discard"], "target": "self", "type": "group_condition"}, "count": 1, "destination": "deck_top_or_bottom", "group_names": ["Aqours"], "optional": true}
```

- 『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい (x2)

```json
{"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "locations": ["live_card_zone", "discard"], "target": "self", "type": "group_condition"}
```

- 『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき (x2)

```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "conditions": [{"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "count": 12, "heart_colors": ["heart02", "heart04", "heart05"], "operator": "=", "type": "comparison_condition"}], "location": "live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02", "heart04", "heart05"], "resource": "heart"}
```

- 自分のライブカード置き場にあるカードが『Aqours』のみで、かつそれらの必要ハートに含まれる{heart_02.png|heart02}と{heart_04.png|heart04}と{heart_05.png|heart05}の合計が12以上の場合、ライブ終了時まで、{icon_all.png|ハート}{icon_all.png|ハート}を得る (x1)
- 自分のライブカード置き場にあるカードが『Aqours』のみで、かつそれらの必要ハートに含まれる{heart_02.png|heart02}と{heart_04.png|heart04}と{heart_05.png|heart05}の合計が12以上の場合、ライブ終了時まで、{icon_all.png|ハート}{icon_all.png|ハート}を得る。" (x1)

```json
{"aggregate": "total", "card_type": "live_card", "conditions": [{"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "count": 12, "heart_colors": ["heart02", "heart04", "heart05"], "operator": "=", "type": "comparison_condition"}], "location": "live_card_zone", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のライブカード置き場にあるカードが『Aqours』のみで、かつそれらの必要ハートに含まれる{heart_02.png|heart02}と{heart_04.png|heart04}と{heart_05.png|heart05}の合計が12以上の場合 (x2)

```json
{"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分のライブカード置き場にあるカードが『Aqours』のみで、 (x2)

```json
{"aggregate": "total", "comparison_type": "cost", "count": 12, "heart_colors": ["heart02", "heart04", "heart05"], "operator": "=", "type": "comparison_condition"}
```

- それらの必要ハートに含まれる{heart_02.png|heart02}と{heart_04.png|heart04}と{heart_05.png|heart05}の合計が12以上の場合 (x2)

```json
{"action": "move_cards", "card_type": "card", "count": 10, "destination": "discard", "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを10枚控え室に置く (x2)

```json
{"count": 2, "destination": "discard", "optional": true, "same_unit_name": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の同じユニット名を持つカード2枚を控え室に置いてもよい (x2)

```json
{"card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージに『虹ヶ咲』のメンバーがいる場合 (x2)

```json
{"count": 9, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーが9枚以上ある場合 (x2)

```json
{"action": "set_card_identity", "card_type": "live_card", "count": 1}
```

- 次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る (x2)

```json
{"action": "modify_score", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- 自分の成功ライブカード置き場にカードが2枚以上ある場合、このカードのスコアを+1する (x2)

```json
{"action": "modify_score", "operation": "add", "self_target": true, "value": 1}
```

- このカードのスコアを+1する (x1)
- このカードのスコアを+1する。それらが両方ある場合、 (x1)

```json
{"action": "gain_resource", "count": 3, "duration": "live_end", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x2)

```json
{"action": "draw_card", "condition": {"card_type": "live_card", "group_names": ["μ's"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}
```

- 自分の成功ライブカード置き場に『μ's』のカードがある場合、カードを1枚引く (x2)

```json
{"card_type": "live_card", "group_names": ["μ's"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分の成功ライブカード置き場に『μ's』のカードがある場合 (x2)

```json
{"card_type": "live_card", "check_self": true, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}
```

- このカードが自分の成功ライブカード置き場にあるかぎり (x2)

```json
{"action": "modify_score", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self", "value": 2}
```

- このカードのスコアを+2し (x2)

```json
{"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "center", "temporal": "this_turn", "type": "temporal_condition"}
```

- 自分のステージのセンターエリアにいる『Liella!』のメンバーが、このターン中に移動している場合 (x1)
- 自分のステージのセンターエリアにいる『μ's』のメンバーの{live_success.png|ライブ成功時}能力が解決したとき、そのメンバーがこのターン中に移動している場合 (x1)

```json
{"location": "success_live_card_zone", "target": "self", "temporal": "during_live", "type": "temporal_condition"}
```

- 自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{heart_01.png|heart01}が3の『虹ヶ咲』のライブカードがある場合 (x1)
- 自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{heart_01.png|heart01}が4の『虹ヶ咲』のライブカードがある場合 (x1)

```json
{"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}
```

- 2人以上いる場合 (x2)

```json
{"card_type": "member_card", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージにいるメンバーが持つハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}がすべてある場合 (x2)

```json
{"card_type": "member_card", "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージにいるメンバーの{live_start.png|ライブ開始時}能力が解決する (x1)
- 自分のステージにいるメンバーの{live_success.png|ライブ成功時}能力が解決する (x1)

```json
{"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに『蓮ノ空』のメンバー1人を含むメンバーが2人以上おり、 (x1)
- 自分のステージにメンバーが2人以上いる場合 (x1)

```json
{"action": "change_state", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるコスト2以下のメンバー1人をウェイトにする (x2)

```json
{"type": "custom"}
```

- 自分がエールしたとき (x2)

```json
{"card_count": 28, "cards": ["PL!-sd1-005-SD | 星空 凛 (ab#0)", "PL!S-PR-026-PR | 桜内梨子 (ab#0)", "PL!N-PR-009-PR | 優木せつ菜 (ab#0)", "PL!N-PR-012-PR | 三船栞子 (ab#0)", "PL!N-PR-014-PR | 鐘 嵐珠 (ab#0)", "PL!N-PR-019-PR | 中須かすみ (ab#0)", "PL!HS-PR-026-PR | 村野さやか (ab#0)", "PL!SP-bp1-011-R | 鬼塚冬毬 (ab#0)", "PL!SP-bp1-011-P | 鬼塚冬毬 (ab#0)", "PL!N-sd1-011-SD | ミア・テイラー (ab#0)", "PL!SP-sd1-006-SD | 桜小路きな子 (ab#0)", "PL!SP-pb1-018-N | 米女メイ (ab#0)", "PL!S-bp2-009-R | 黒澤ルビィ (ab#0)", "PL!S-bp2-009-P | 黒澤ルビィ (ab#0)", "PL!HS-bp2-004-R | 夕霧綴理 (ab#0)", "PL!HS-bp2-004-P | 夕霧綴理 (ab#0)", "PL!S-pb1-004-R | 黒澤ダイヤ (ab#0)", "PL!S-pb1-004-P＋ | 黒澤ダイヤ (ab#0)", "PL!-pb1-024-N | 西木野真姫 (ab#0)", "PL!-bp4-003-R | 南 ことり (ab#0)", "PL!-bp4-003-P | 南 ことり (ab#0)", "PL!-sd1-005-RM | 星空 凛 (ab#0)", "PL!N-PR-009-RM | 優木せつ菜 (ab#0)", "PL!N-PR-012-RM | 三船栞子 (ab#0)", "PL!N-PR-014-RM | 鐘 嵐珠 (ab#0)", "PL!HS-sd1-009-SD | 日野下花帆 (ab#0)", "PL!S-sd1-015-SD | 津島善子 (ab#0)", "PL!SP-sd2-010-SD2 | ウィーン・マルガレーテ (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 21, "cards": ["PL!-sd1-002-SD | 絢瀬 絵里 (ab#0)", "PL!S-PR-025-PR | 高海千歌 (ab#0)", "PL!S-PR-027-PR | 松浦果南 (ab#0)", "PL!HS-PR-014-PR | 日野下花帆 (ab#0)", "PL!N-sd1-006-SD | 近江彼方 (ab#0)", "PL!SP-pb1-021-N | ウィーン・マルガレーテ (ab#0)", "PL!S-bp2-016-N | 国木田花丸 (ab#0)", "PL!-pb1-019-N | 高坂穂乃果 (ab#0)", "PL!-pb1-025-N | 東條 希 (ab#0)", "PL!N-bp4-017-N | 宮下 愛 (ab#0)", "PL!N-bp4-020-N | エマ・ヴェルデ (ab#0)", "PL!SP-bp4-015-N | 平安名すみれ (ab#0)", "PL!SP-bp4-019-N | 若菜四季 (ab#0)", "PL!S-PR-025-RM | 高海千歌 (ab#0)", "PL!S-PR-027-RM | 松浦果南 (ab#0)", "PL!HS-PR-014-RM | 日野下花帆 (ab#0)", "PL!HS-sd1-015-SD | セラス 柳田 リリエンフェルト (ab#0)", "PL!S-sd1-008-SD | 小原鞠莉 (ab#0)", "PL!HS-pb1-019-N | 大沢瑠璃乃 (ab#0)", "PL!S-bp6-014-N | 渡辺 曜 (ab#0)", "PL!SP-sd2-014-SD2 | 嵐 千砂都 (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 20, "cards": ["PL!-sd1-011-SD | 絢瀬 絵里 (ab#0)", "PL!-sd1-012-SD | 南 ことり (ab#0)", "PL!-sd1-016-SD | 東條 希 (ab#0)", "PL!-sd1-011-PR | 絢瀬絵里 (ab#0)", "PL!-sd1-016-PR | 東條 希 (ab#0)", "PL!N-PR-004-PR | 中須かすみ (ab#0)", "PL!N-PR-006-PR | 朝香果林 (ab#0)", "PL!N-PR-013-PR | ミア・テイラー (ab#0)", "PL!HS-PR-001-PR | 日野下花帆 (ab#0)", "PL!HS-PR-002-PR | 村野さやか (ab#0)", "PL!HS-PR-005-PR | 大沢瑠璃乃 (ab#0)", "PL!N-bp1-007-R | 優木せつ菜 (ab#0)", "PL!N-bp1-007-P | 優木せつ菜 (ab#0)", "PL!N-bp1-010-R | 三船栞子 (ab#0)", "PL!N-bp1-010-P | 三船栞子 (ab#0)", "PL!N-sd1-002-SD | 中須かすみ (ab#0)", "PL!N-sd1-003-SD | 桜坂しずく (ab#0)", "PL!HS-pb1-011-R | 大沢瑠璃乃 (ab#0)", "PL!HS-pb1-011-P＋ | 大沢瑠璃乃 (ab#0)", "PL!HS-cl1-007-CL | セラス 柳田 リリエンフェルト (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 16, "cards": ["PL!N-bp1-014-PRproteinbar | 中須かすみ (ab#0)", "PL!N-bp1-015-PRproteinbar | 桜坂しずく (ab#0)", "PL!N-bp1-019-PR | 優木せつ菜 (ab#0)", "PL!N-bp1-019-PRproteinbar | 優木せつ菜 (ab#0)", "PL!N-sd1-013-PRproteinbar | 上原歩夢 (ab#0)", "PL!N-sd1-021-PRproteinbar | 天王寺璃奈 (ab#0)", "PL!N-sd1-022-PRproteinbar | 三船栞子 (ab#0)", "PL!N-bp1-014-N | 中須かすみ (ab#0)", "PL!N-bp1-015-N | 桜坂しずく (ab#0)", "PL!N-bp1-019-N | 優木せつ菜 (ab#0)", "PL!HS-bp1-010-N | 日野下花帆 (ab#0)", "PL!HS-bp1-014-N | 大沢瑠璃乃 (ab#0)", "PL!N-sd1-013-SD | 上原歩夢 (ab#0)", "PL!N-sd1-021-SD | 天王寺璃奈 (ab#0)", "PL!N-sd1-022-SD | 三船栞子 (ab#0)", "PL!HS-bp6-020-N | 百生 吟子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 14, "cards": ["PL!HS-PR-018-PR | 大沢瑠璃乃 (ab#0)", "PL!HS-PR-022-PR | セラス 柳田 リリエンフェルト (ab#0)", "PL!SP-bp1-006-R | 桜小路きな子 (ab#0)", "PL!SP-bp1-006-P | 桜小路きな子 (ab#0)", "PL!SP-bp2-019-N | 若菜四季 (ab#0)", "PL!SP-bp2-022-N | 鬼塚冬毬 (ab#0)", "PL!S-pb1-016-N | 国木田花丸 (ab#0)", "PL!S-pb1-017-N | 小原鞠莉 (ab#0)", "PL!S-pb1-018-N | 黒澤ルビィ (ab#0)", "PL!-bp4-010-N | 高坂穂乃果 (ab#0)", "PL!N-bp4-013-N | 上原歩夢 (ab#0)", "PL!HS-PR-018-RM | 大沢瑠璃乃 (ab#0)", "PL!HS-sd1-006-SD | 安養寺 姫芽 (ab#1)", "PL!HS-cl1-005-CL | 徒町 小鈴 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 7, "cards": ["PL!-bp3-014-PR | 星空 凛 (ab#0)", "PL!-bp3-014-N | 星空 凛 (ab#0)", "PL!-bp3-017-N | 小泉花陽 (ab#0)", "PL!-bp3-018-N | 矢澤にこ (ab#0)", "PL!N-bp3-022-N | 三船栞子 (ab#0)", "PL!N-bp4-016-N | 朝香果林 (ab#0)", "PL!S-bp6-018-N | 黒澤ルビィ (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}
```

- 自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く (x1)

```json
{"card_count": 7, "cards": ["PL!S-PR-028-PR | 黒澤ダイヤ (ab#0)", "PL!S-PR-032-PR | 小原鞠莉 (ab#0)", "PL!S-PR-033-PR | 黒澤ルビィ (ab#0)", "PL!N-bp1-002-R＋ | 中須かすみ (ab#0)", "PL!N-bp1-002-P | 中須かすみ (ab#0)", "PL!N-bp1-002-P＋ | 中須かすみ (ab#0)", "PL!N-bp1-002-SEC | 中須かすみ (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 7, "cards": ["PL!N-bp1-006-R＋ | 近江彼方 (ab#1)", "PL!N-bp1-006-P | 近江彼方 (ab#1)", "PL!N-bp1-006-P＋ | 近江彼方 (ab#1)", "PL!N-bp1-006-SEC | 近江彼方 (ab#1)", "PL!HS-bp1-007-R | 百生 吟子 (ab#0)", "PL!HS-bp1-007-P | 百生 吟子 (ab#0)", "PL!SP-bp5-020-N | 鬼塚夏美 (ab#0)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 6, "cards": ["PL!-PR-007-PR | 東條 希 (ab#0)", "PL!-PR-009-PR | 矢澤にこ (ab#0)", "PL!S-bp3-012-N | 松浦果南 (ab#0)", "PL!S-bp3-017-N | 小原鞠莉 (ab#0)", "PL!N-bp3-017-N | 宮下 愛 (ab#0)", "PL!N-bp3-023-N | ミア・テイラー (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"card_count": 6, "cards": ["PL!SP-PR-004-PR | 唐 可可 (ab#0)", "PL!SP-PR-006-PR | 平安名すみれ (ab#0)", "PL!SP-PR-013-PR | 鬼塚冬毬 (ab#0)", "PL!SP-bp1-021-N | ウィーン・マルガレーテ (ab#0)", "PL!SP-sd1-014-SD | 嵐 千砂都 (ab#0)", "PL!SP-sd1-016-SD | 葉月 恋 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 6, "cards": ["PL!N-bp1-003-R＋ | 桜坂しずく (ab#0)", "PL!N-bp1-003-P | 桜坂しずく (ab#0)", "PL!N-bp1-003-P＋ | 桜坂しずく (ab#0)", "PL!N-bp1-003-SEC | 桜坂しずく (ab#0)", "PL!N-bp5-019-N | 優木せつ菜 (ab#0)", "PL!N-bp5-022-N | 三船栞子 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 6, "cards": ["PL!HS-bp1-006-R＋ | 藤島 慈 (ab#0)", "PL!HS-bp1-006-P | 藤島 慈 (ab#0)", "PL!HS-bp1-006-P＋ | 藤島 慈 (ab#0)", "PL!HS-bp1-006-SEC | 藤島 慈 (ab#0)", "PL!N-sd1-010-SD | 三船栞子 (ab#0)", "PL!HS-sd1-008-SD | 桂城 泉 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 6, "cards": ["PL!S-bp2-024-L | 君のこころは輝いてるかい？ (ab#1)", "PL!SP-bp2-009-R＋ | 鬼塚夏美 (ab#1)", "PL!SP-bp2-009-P | 鬼塚夏美 (ab#1)", "PL!SP-bp2-009-P＋ | 鬼塚夏美 (ab#1)", "PL!SP-bp2-009-SEC | 鬼塚夏美 (ab#1)", "PL!S-bp2-024-SECL | 君のこころは輝いてるかい？ (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 5, "cards": ["PL!-bp3-012-PR | 南 ことり (ab#0)", "PL!-bp3-011-N | 絢瀬絵里 (ab#0)", "PL!-bp3-012-N | 南ことり (ab#0)", "PL!-bp3-013-N | 園田海未 (ab#0)", "PL!-bp3-012-RM | 南 ことり (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}], "heart_colors": ["heart01", "heart03", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}], "heart_colors": ["heart01", "heart03", "heart06"]}
```

- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る (x1)

```json
{"card_count": 5, "cards": ["PL!N-PR-005-PR | 桜坂しずく (ab#0)", "PL!N-PR-007-PR | 宮下 愛 (ab#0)", "PL!N-PR-011-PR | 天王寺璃奈 (ab#0)", "PL!S-bp2-010-N | 高海千歌 (ab#0)", "PL!N-bp3-024-N | 鐘 嵐珠 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 5, "cards": ["PL!N-bp1-012-R＋ | 鐘 嵐珠 (ab#1)", "PL!N-bp1-012-P | 鐘 嵐珠 (ab#1)", "PL!N-bp1-012-P＋ | 鐘 嵐珠 (ab#1)", "PL!N-bp1-012-SEC | 鐘 嵐珠 (ab#1)", "PL!SP-sd1-005-SD | 葉月 恋 (ab#0)"], "cost": {"count": 3, "energy": 3, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 5, "cards": ["PL!-bp3-002-R | 絢瀬絵里 (ab#0)", "PL!-bp3-002-P | 絢瀬絵里 (ab#0)", "PL!N-bp4-005-R | 宮下 愛 (ab#0)", "PL!N-bp4-005-P | 宮下 愛 (ab#0)", "PL!HS-bp5-016-N | 桂城 泉 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 2, "max": true, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 2, "max": true, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるコスト4以下のメンバーを2人までウェイトにする (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp1-002-R＋ | 中須かすみ (ab#1)", "PL!N-bp1-002-P | 中須かすみ (ab#1)", "PL!N-bp1-002-P＋ | 中須かすみ (ab#1)", "PL!N-bp1-002-SEC | 中須かすみ (ab#1)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "activation_condition_parsed": {"check_self": true, "count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "card", "count": 1, "destination": "stage", "self_target": true, "source": "discard"}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "move_cards", "activation_condition_parsed": {"check_self": true, "count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "card", "count": 1, "destination": "stage", "self_target": true, "source": "discard"}
```

- このカードを控え室からステージに登場させる (x1)

```json
{"check_self": true, "count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- このカードが控え室にある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp1-003-R＋ | 桜坂しずく (ab#1)", "PL!N-bp1-003-P | 桜坂しずく (ab#1)", "PL!N-bp1-003-P＋ | 桜坂しずく (ab#1)", "PL!N-bp1-003-SEC | 桜坂しずく (ab#1)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "heart"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 4, "cards": ["PL!N-bp1-006-R＋ | 近江彼方 (ab#0)", "PL!N-bp1-006-P | 近江彼方 (ab#0)", "PL!N-bp1-006-P＋ | 近江彼方 (ab#0)", "PL!N-bp1-006-SEC | 近江彼方 (ab#0)"], "cost": {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "count": 1, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "count": 2, "state_change": "active"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "count": 1, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "count": 2, "state_change": "active"}
```

- このターン、自分のステージに『虹ヶ咲』のメンバーが登場している場合、エネルギーを2枚アクティブにする (x1)

```json
{"card_type": "member_card", "count": 1, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}
```

- このターン、自分のステージに『虹ヶ咲』のメンバーが登場している場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp1-012-R＋ | 鐘 嵐珠 (ab#0)", "PL!N-bp1-012-P | 鐘 嵐珠 (ab#0)", "PL!N-bp1-012-P＋ | 鐘 嵐珠 (ab#0)", "PL!N-bp1-012-SEC | 鐘 嵐珠 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "conditions": [{"count": 3, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "operator": ">=", "type": "card_count_condition"}], "operator": "and", "target": "self", "type": "compound"}, "count": 2, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "conditions": [{"count": 3, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "operator": ">=", "type": "card_count_condition"}], "operator": "and", "target": "self", "type": "compound"}, "count": 2, "resource": "blade"}
```

- 自分のライブ中のカードが3枚以上あり、その中に『虹ヶ咲』のライブカードを1枚以上含む場合、{icon_all.png|ハート}{icon_all.png|ハート}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "live_card", "conditions": [{"count": 3, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "operator": ">=", "type": "card_count_condition"}], "operator": "and", "target": "self", "type": "compound"}
```

- 自分のライブ中のカードが3枚以上あり、その中に『虹ヶ咲』のライブカードを1枚以上含む場合 (x1)

```json
{"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "operator": ">=", "type": "card_count_condition"}
```

- その中に『虹ヶ咲』のライブカードを1枚以上含む場合 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp1-002-R＋ | 唐 可可 (ab#0)", "PL!SP-bp1-002-P | 唐 可可 (ab#0)", "PL!SP-bp1-002-P＋ | 唐 可可 (ab#0)", "PL!SP-bp1-002-SEC | 唐 可可 (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "draw_card", "condition": {"appearance": true, "location": "stage", "position": "left_side", "type": "appearance_condition"}, "count": 2, "destination": "hand", "position": "left_side", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"appearance": true, "location": "stage", "position": "left_side", "type": "appearance_condition"}, "count": 2, "destination": "hand", "position": "left_side", "source": "deck"}
```

- ステージの左サイドエリアに登場しているなら、カードを2枚引く (x1)

```json
{"appearance": true, "location": "stage", "position": "left_side", "type": "appearance_condition"}
```

- ステージの左サイドエリアに登場しているなら (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp1-003-R＋ | 嵐 千砂都 (ab#0)", "PL!SP-bp1-003-P | 嵐 千砂都 (ab#0)", "PL!SP-bp1-003-P＋ | 嵐 千砂都 (ab#0)", "PL!SP-bp1-003-SEC | 嵐 千砂都 (ab#0)"], "cost": {"card_type": "member_card", "source": "hand", "type": "reveal"}, "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"aggregate": "total", "comparison_type": "cost", "cost_total": 10, "count": 10, "location": "revealed_cards", "operator": "=", "type": "comparison_condition", "values": [10, 20, 30, 40, 50]}, "duration": "live_end"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "source": "hand", "type": "reveal"}
```

- 手札にあるメンバーカードを好きな枚数公開する (x1)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"aggregate": "total", "comparison_type": "cost", "cost_total": 10, "count": 10, "location": "revealed_cards", "operator": "=", "type": "comparison_condition", "values": [10, 20, 30, 40, 50]}, "duration": "live_end"}
```

- 公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"aggregate": "total", "comparison_type": "cost", "cost_total": 10, "count": 10, "location": "revealed_cards", "operator": "=", "type": "comparison_condition", "values": [10, 20, 30, 40, 50]}
```

- 公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp1-007-R＋ | 米女メイ (ab#0)", "PL!SP-bp1-007-P | 米女メイ (ab#0)", "PL!SP-bp1-007-P＋ | 米女メイ (ab#0)", "PL!SP-bp1-007-SEC | 米女メイ (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"count": 11, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"count": 11, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分のエネルギーが11枚以上ある場合、自分の控え室からライブカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp1-001-R | 日野下花帆 (ab#0)", "PL!HS-bp1-001-P | 日野下花帆 (ab#0)", "PL!N-sd1-008-SD | エマ・ヴェルデ (ab#0)", "PL!N-sd1-008-RM | エマ・ヴェルデ (ab#0)"], "effect": {"action": "change_state", "card_type": "energy_card", "count": 2, "state_change": "active"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 4, "cards": ["PL!HS-bp1-003-R＋ | 乙宗 梢 (ab#0)", "PL!HS-bp1-003-P | 乙宗 梢 (ab#0)", "PL!HS-bp1-003-P＋ | 乙宗 梢 (ab#0)", "PL!HS-bp1-003-SEC | 乙宗 梢 (ab#0)"], "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"card_type": "member_card", "conditions": [{"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["蓮ノ空"]}, "is_null": false, "triggers": "常時"}
```


```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"card_type": "member_card", "conditions": [{"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["蓮ノ空"]}
```

- 自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp1-003-R＋ | 乙宗 梢 (ab#1)", "PL!HS-bp1-003-P | 乙宗 梢 (ab#1)", "PL!HS-bp1-003-P＋ | 乙宗 梢 (ab#1)", "PL!HS-bp1-003-SEC | 乙宗 梢 (ab#1)"], "cost": {"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室から4コスト以下の『蓮ノ空』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp1-004-R＋ | 夕霧綴理 (ab#0)", "PL!HS-bp1-004-P | 夕霧綴理 (ab#0)", "PL!HS-bp1-004-P＋ | 夕霧綴理 (ab#0)", "PL!HS-bp1-004-SEC | 夕霧綴理 (ab#0)"], "cost": {"count": 3, "energy": 3, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室から『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp1-004-R＋ | 夕霧綴理 (ab#1)", "PL!HS-bp1-004-P | 夕霧綴理 (ab#1)", "PL!HS-bp1-004-P＋ | 夕霧綴理 (ab#1)", "PL!HS-bp1-004-SEC | 夕霧綴理 (ab#1)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_type": "live_card_zone", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_type": "live_card_zone", "resource": "blade", "target": "self"}
```

- 自分のライブ中のカード1枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp1-006-R＋ | 藤島 慈 (ab#1)", "PL!HS-bp1-006-P | 藤島 慈 (ab#1)", "PL!HS-bp1-006-P＋ | 藤島 慈 (ab#1)", "PL!HS-bp1-006-SEC | 藤島 慈 (ab#1)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "heart"}], "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, "exclude_self": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "heart"}], "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, "exclude_self": true}
```

- 自分のステージにほかのメンバーがいる場合、好きなハートの色を1つ指定する。ライブ終了時まで、そのハートを1つ得る (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp2-005-R＋ | 渡辺 曜 (ab#0)", "PL!S-bp2-005-P | 渡辺 曜 (ab#0)", "PL!S-bp2-005-P＋ | 渡辺 曜 (ab#0)", "PL!S-bp2-005-SEC | 渡辺 曜 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "heart_colors": ["heart02", "heart04", "heart05"], "look_action": {"action": "look_at", "count": 7, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 3, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02", "heart04", "heart05"], "max": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart02", "heart04", "heart05"], "look_action": {"action": "look_at", "count": 7, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 3, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02", "heart04", "heart05"], "max": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを7枚見る。その中から{heart_02.png|heart02}か{heart_04.png|heart04}か{heart_05.png|heart05}を持つメンバーカードを3枚まで公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 3, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02", "heart04", "heart05"], "max": true, "optional": true, "reveal": true}
```


```json
{"card_count": 4, "cards": ["PL!S-bp2-007-R＋ | 国木田花丸 (ab#0)", "PL!S-bp2-007-P | 国木田花丸 (ab#0)", "PL!S-bp2-007-P＋ | 国木田花丸 (ab#0)", "PL!S-bp2-007-SEC | 国木田花丸 (ab#0)"], "effect": {"action": "draw_card", "condition": {"conditions": [{"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"count": 7, "location": "hand", "operator": "<=", "resource_type": "hand_count", "type": "comparison_condition"}], "operator": "and", "type": "compound"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "draw_card", "condition": {"conditions": [{"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"count": 7, "location": "hand", "operator": "<=", "resource_type": "hand_count", "type": "comparison_condition"}], "operator": "and", "type": "compound"}, "count": 1, "destination": "hand", "source": "deck"}
```

- エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く (x1)

```json
{"conditions": [{"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"count": 7, "location": "hand", "operator": "<=", "resource_type": "hand_count", "type": "comparison_condition"}], "operator": "and", "type": "compound"}
```

- エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合 (x1)

```json
{"count": 7, "location": "hand", "operator": "<=", "resource_type": "hand_count", "type": "comparison_condition"}
```

- 自分の手札が7枚以下の場合 (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp2-007-R＋ | 国木田花丸 (ab#1)", "PL!S-bp2-007-P | 国木田花丸 (ab#1)", "PL!S-bp2-007-P＋ | 国木田花丸 (ab#1)", "PL!S-bp2-007-SEC | 国木田花丸 (ab#1)"], "cost": {"costs": [{"card_type": "live_card", "count": 1, "optional": true, "source": "hand", "type": "reveal", "zone": "hand"}, {"destination": "deck_bottom", "optional": true, "type": "move_cards"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"costs": [{"card_type": "live_card", "count": 1, "optional": true, "source": "hand", "type": "reveal", "zone": "hand"}, {"destination": "deck_bottom", "optional": true, "type": "move_cards"}], "optional": true, "type": "sequential_cost"}
```

- 手札のライブカードを1枚公開し、デッキの一番下に置いてもよい (x1)

```json
{"destination": "deck_bottom", "optional": true, "type": "move_cards"}
```

- デッキの一番下に置いてもよい (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp2-008-R＋ | 小原鞠莉 (ab#0)", "PL!S-bp2-008-P | 小原鞠莉 (ab#0)", "PL!S-bp2-008-P＋ | 小原鞠莉 (ab#0)", "PL!S-bp2-008-SEC | 小原鞠莉 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "max": true, "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードを1枚までデッキの一番下に置く (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp2-008-R＋ | 小原鞠莉 (ab#1)", "PL!S-bp2-008-P | 小原鞠莉 (ab#1)", "PL!S-bp2-008-P＋ | 小原鞠莉 (ab#1)", "PL!S-bp2-008-SEC | 小原鞠莉 (ab#1)"], "effect": {"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"card_type": "member_card", "conditions": [{"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["Aqours"], "primary_effect": {"action": "modify_score", "card_type": "live_card", "count": 1, "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "operation": "add", "source": "revealed_cards", "target": "self", "value": 1}}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"card_type": "member_card", "conditions": [{"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["Aqours"], "primary_effect": {"action": "modify_score", "card_type": "live_card", "count": 1, "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "operation": "add", "source": "revealed_cards", "target": "self", "value": 1}}
```

- 自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合、「{live_success.png|ライブ成功時}エールにより公開された自分のカードの中にライブカードが1枚以上ある場合、ライブの合計スコアを+1する。ライブカードが3枚以上ある場合、代わりに合計スコアを+2する。」を得る (x1)

```json
{"action": "modify_score", "card_type": "live_card", "count": 1, "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "operation": "add", "source": "revealed_cards", "target": "self", "value": 1}
```

- 「{live_success.png|ライブ成功時}エールにより公開された自分のカードの中にライブカードが1枚以上ある場合、ライブの合計スコアを+1する。ライブカードが3枚以上ある場合、 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp2-001-R＋ | 澁谷かのん (ab#0)", "PL!SP-bp2-001-P | 澁谷かのん (ab#0)", "PL!SP-bp2-001-P＋ | 澁谷かのん (ab#0)", "PL!SP-bp2-001-SEC | 澁谷かのん (ab#0)"], "effect": {"action": "conditional_on_result", "all": true, "followup_action": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}, "group_names": ["Liella!"], "primary_effect": {"action": "invalidate_ability", "all": false, "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Liella!"], "optional": true, "target": "self"}, "result_condition": {"action_reference": "invalidate_ability", "type": "action_success_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "conditional_on_result", "all": true, "followup_action": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}, "group_names": ["Liella!"], "primary_effect": {"action": "invalidate_ability", "all": false, "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Liella!"], "optional": true, "target": "self"}, "result_condition": {"action_reference": "invalidate_ability", "type": "action_success_condition"}}
```

- 自分のステージにいる『Liella!』のメンバー1人のすべての{live_start.png|ライブ開始時}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした場合、自分の控え室から『Liella!』のカードを1枚手札に加える (x1)

```json
{"action": "invalidate_ability", "all": false, "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Liella!"], "optional": true, "target": "self"}
```

- 自分のステージにいる『Liella!』のメンバー1人のすべての{live_start.png|ライブ開始時}能力を、ライブ終了時まで、無効にしてもよい。 (x1)

```json
{"action_reference": "invalidate_ability", "type": "action_success_condition"}
```

- これにより無効にした場合 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp2-006-R＋ | 桜小路きな子 (ab#0)", "PL!SP-bp2-006-P | 桜小路きな子 (ab#0)", "PL!SP-bp2-006-P＋ | 桜小路きな子 (ab#0)", "PL!SP-bp2-006-SEC | 桜小路きな子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "condition": {"baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "recently_moved"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "condition": {"baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "recently_moved"}
```

- バトンタッチして登場した場合、このバトンタッチで控え室に置かれた『Liella!』のメンバーカードを1枚手札に加える (x1)

```json
{"baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- バトンタッチして登場した場合 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp2-006-R＋ | 桜小路きな子 (ab#1)", "PL!SP-bp2-006-P | 桜小路きな子 (ab#1)", "PL!SP-bp2-006-P＋ | 桜小路きな子 (ab#1)", "PL!SP-bp2-006-SEC | 桜小路きな子 (ab#1)"], "cost": {"card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "discard", "group_names": ["Liella!"], "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"ability_text": "登場_ability", "action": "activate_ability", "count": 1, "parenthetical": ["{{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。"], "source_card": "cost_card", "target": "これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力", "target_trigger": "登場"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "discard", "group_names": ["Liella!"], "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く (x1)

```json
{"ability_text": "登場_ability", "action": "activate_ability", "count": 1, "parenthetical": ["{{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。"], "source_card": "cost_card", "target": "これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力", "target_trigger": "登場"}
```

- これにより控え室に置いたメンバーカードの{toujyou.png|登場}能力1つを発動させる (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp2-009-R＋ | 鬼塚夏美 (ab#0)", "PL!SP-bp2-009-P | 鬼塚夏美 (ab#0)", "PL!SP-bp2-009-P＋ | 鬼塚夏美 (ab#0)", "PL!SP-bp2-009-SEC | 鬼塚夏美 (ab#0)"], "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 2, "per_unit_type": "枚", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 2, "per_unit_type": "枚", "resource": "blade", "target": "self"}
```

- 自分の手札2枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp2-010-R＋ | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp2-010-P | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp2-010-P＋ | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp2-010-SEC | ウィーン・マルガレーテ (ab#0)"], "effect": {"action": "modify_required_hearts_global", "all": true, "heart_colors": ["heart00"], "operation": "increase", "target": "opponent", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_required_hearts_global", "all": true, "heart_colors": ["heart00"], "operation": "increase", "target": "opponent", "value": 1}
```

- 相手のライブカード置き場にあるすべてのライブカードは、成功させるための必要ハートが{heart_00.png|heart0}多くなる (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp2-010-R＋ | ウィーン・マルガレーテ (ab#1)", "PL!SP-bp2-010-P | ウィーン・マルガレーテ (ab#1)", "PL!SP-bp2-010-P＋ | ウィーン・マルガレーテ (ab#1)", "PL!SP-bp2-010-SEC | ウィーン・マルガレーテ (ab#1)"], "effect": {"action": "modify_yell_count", "condition": {"card_type": "member_card", "count": 1, "exclude_self": true, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 8, "duration": "live_end", "exclude_self": true, "operation": "subtract"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_yell_count", "condition": {"card_type": "member_card", "count": 1, "exclude_self": true, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 8, "duration": "live_end", "exclude_self": true, "operation": "subtract"}
```

- 自分のステージにこのメンバー以外のメンバーが1人以上いる場合、ライブ終了時まで、エールによって公開される自分のカードの枚数が8枚減る (x1)

```json
{"card_type": "member_card", "count": 1, "exclude_self": true, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージにこのメンバー以外のメンバーが1人以上いる場合 (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp2-002-R＋ | 村野さやか (ab#0)", "PL!HS-bp2-002-P | 村野さやか (ab#0)", "PL!HS-bp2-002-P＋ | 村野さやか (ab#0)", "PL!HS-bp2-002-SEC | 村野さやか (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 2, "destination": "hand", "max": true, "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 2, "destination": "hand", "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室からコスト2以下のメンバーカードを2枚まで手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp2-002-R＋ | 村野さやか (ab#1)", "PL!HS-bp2-002-P | 村野さやか (ab#1)", "PL!HS-bp2-002-P＋ | 村野さやか (ab#1)", "PL!HS-bp2-002-SEC | 村野さやか (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "self", "comparison_type": "cost", "location": "stage", "operator": ">", "target": "self", "type": "comparison_condition"}, "count": 3, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "self", "comparison_type": "cost", "location": "stage", "operator": ">", "target": "self", "type": "comparison_condition"}, "count": 3, "resource": "blade"}
```

- 自分のステージに、このメンバーよりコストの大きいメンバーがいる場合、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp2-005-R＋ | 大沢瑠璃乃 (ab#0)", "PL!HS-bp2-005-P | 大沢瑠璃乃 (ab#0)", "PL!HS-bp2-005-P＋ | 大沢瑠璃乃 (ab#0)", "PL!HS-bp2-005-SEC | 大沢瑠璃乃 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "source": "discard", "target": "self"}
```

- 自分のステージにほかのメンバーがいる場合、自分の控え室から『みらくらぱーく！』のカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp2-005-R＋ | 大沢瑠璃乃 (ab#1)", "PL!HS-bp2-005-P | 大沢瑠璃乃 (ab#1)", "PL!HS-bp2-005-P＋ | 大沢瑠璃乃 (ab#1)", "PL!HS-bp2-005-SEC | 大沢瑠璃乃 (ab#1)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "condition": {"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"all_areas": true, "appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- 自分のステージのエリアすべてにメンバーが登場している場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp2-007-R＋ | 百生 吟子 (ab#0)", "PL!HS-bp2-007-P | 百生 吟子 (ab#0)", "PL!HS-bp2-007-P＋ | 百生 吟子 (ab#0)", "PL!HS-bp2-007-SEC | 百生 吟子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["スリーズブーケ"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "recently_moved", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["スリーズブーケ"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "recently_moved", "target": "self"}
```

- このメンバーよりコストが低い『スリーズブーケ』のメンバーからバトンタッチして登場した場合、自分の控え室から『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["スリーズブーケ"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}
```

- このメンバーよりコストが低い『スリーズブーケ』のメンバーからバトンタッチして登場した場合 (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp2-007-R＋ | 百生 吟子 (ab#1)", "PL!HS-bp2-007-P | 百生 吟子 (ab#1)", "PL!HS-bp2-007-P＋ | 百生 吟子 (ab#1)", "PL!HS-bp2-007-SEC | 百生 吟子 (ab#1)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "heart_colors": ["heart04"], "location": "discard", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "heart_colors": ["heart04"], "location": "discard", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade", "target_count": 1}
```

- これにより控え室に置いたカードがメンバーカードの場合、控え室に置いたカードと同じ名前を持つメンバー1人は、ライブ終了時まで、{heart_04.png|heart04}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "heart_colors": ["heart04"], "location": "discard", "type": "location_condition"}
```

- これにより控え室に置いたカードがメンバーカードの場合 (x1)

```json
{"card_count": 4, "cards": ["PL!-bp3-004-R＋ | 園田海未 (ab#0)", "PL!-bp3-004-P | 園田海未 (ab#0)", "PL!-bp3-004-P＋ | 園田海未 (ab#0)", "PL!-bp3-004-SEC | 園田海未 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}
```

- 自分のステージにいるメンバー1人につき、カードを1枚引く。その後、手札を1枚控え室に置く (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "target": "self"}
```

- カードを1枚引く。 (x1)

```json
{"card_count": 4, "cards": ["PL!-bp3-004-R＋ | 園田海未 (ab#1)", "PL!-bp3-004-P | 園田海未 (ab#1)", "PL!-bp3-004-P＋ | 園田海未 (ab#1)", "PL!-bp3-004-SEC | 園田海未 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}], "condition": {"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "group_names": ["μ's"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}], "condition": {"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "group_names": ["μ's"]}
```

- 自分の成功ライブカード置き場にカードがある場合、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『μ's』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!-bp3-008-R＋ | 小泉花陽 (ab#0)", "PL!-bp3-008-P | 小泉花陽 (ab#0)", "PL!-bp3-008-P＋ | 小泉花陽 (ab#0)", "PL!-bp3-008-SEC | 小泉花陽 (ab#0)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 4, "cards": ["PL!-bp3-008-R＋ | 小泉花陽 (ab#1)", "PL!-bp3-008-P | 小泉花陽 (ab#1)", "PL!-bp3-008-P＋ | 小泉花陽 (ab#1)", "PL!-bp3-008-SEC | 小泉花陽 (ab#1)"], "cost": {"card_type": "member_card", "count": 1, "group_names": ["μ's"], "optional": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_type": "member_card", "count": 1, "group_names": ["μ's"], "optional": true, "state_change": "wait", "type": "change_state"}
```

- 『μ's』のメンバー1人をウェイトにしてもよい (x1)

```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart03"], "resource": "heart"}
```

- {heart_03.png|heart03}{heart_03.png|heart03}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp3-001-R＋ | 高海千歌 (ab#0)", "PL!S-bp3-001-P | 高海千歌 (ab#0)", "PL!S-bp3-001-P＋ | 高海千歌 (ab#0)", "PL!S-bp3-001-SEC | 高海千歌 (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "position": "center", "state_change": "wait", "type": "change_state"}, "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}, "activation_position": "center", "card_type": "member_card", "duration": "live_end", "parenthetical": ["この能力はセンターエリアに登場している場合のみ起動できる。"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "count": 1, "position": "center", "state_change": "wait", "type": "change_state"}
```

- {center.png|センター}メンバー1人をウェイトにする (x1)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}, "activation_position": "center", "card_type": "member_card", "duration": "live_end", "parenthetical": ["この能力はセンターエリアに登場している場合のみ起動できる。"]}
```

- これによってウェイト状態になったメンバーは、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp3-006-R＋ | 津島善子 (ab#0)", "PL!S-bp3-006-P | 津島善子 (ab#0)", "PL!S-bp3-006-P＋ | 津島善子 (ab#0)", "PL!S-bp3-006-SEC | 津島善子 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "position": "center", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "position": "center", "type": "sequential_cost"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "count": 1, "destination": "discard", "exclude_self": true, "group_names": ["Aqours"], "source": "stage", "target": "self"}, {"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit_operator": "=", "cost_offset": 2, "cost_reference": "previous_moved_card", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}], "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}, "activation_position": "center", "conditional": true, "exclude_self": true, "group_names": ["Aqours"], "parenthetical": ["この能力はセンターエリアに登場している場合のみ起動できる。"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "count": 1, "destination": "discard", "exclude_self": true, "group_names": ["Aqours"], "source": "stage", "target": "self"}, {"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit_operator": "=", "cost_offset": 2, "cost_reference": "previous_moved_card", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}], "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}, "activation_position": "center", "conditional": true, "exclude_self": true, "group_names": ["Aqours"], "parenthetical": ["この能力はセンターエリアに登場している場合のみ起動できる。"]}
```

- このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。そうした場合、自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる (x1)

```json
{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "count": 1, "destination": "discard", "exclude_self": true, "group_names": ["Aqours"], "source": "stage", "target": "self"}
```

- このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。 (x1)

```json
{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit_operator": "=", "cost_offset": 2, "cost_reference": "previous_moved_card", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}
```

- 自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp3-001-R＋ | 上原歩夢 (ab#0)", "PL!N-bp3-001-P | 上原歩夢 (ab#0)", "PL!N-bp3-001-P＋ | 上原歩夢 (ab#0)", "PL!N-bp3-001-SEC | 上原歩夢 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "optional": true, "target": "self"}, {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self"}]}], "conditional": true, "parenthetical": ["メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "optional": true, "target": "self"}, {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self"}]}], "conditional": true, "parenthetical": ["メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。"]}
```

- 自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。そうした場合、カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self"}]}
```

- カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self"}
```

- 自分のステージにいるメンバーは{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp3-008-R＋ | エマ・ヴェルデ (ab#0)", "PL!N-bp3-008-P | エマ・ヴェルデ (ab#0)", "PL!N-bp3-008-P＋ | エマ・ヴェルデ (ab#0)", "PL!N-bp3-008-SEC | エマ・ヴェルデ (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "exclude_self": true, "group_names": ["虹ヶ咲"], "state_change": "wait", "type": "change_state"}, "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "count": 1, "exclude_self": true, "group_names": ["虹ヶ咲"], "state_change": "wait", "type": "change_state"}
```

- このメンバー以外の『虹ヶ咲』のメンバー1人をウェイトにする (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp3-008-R＋ | エマ・ヴェルデ (ab#1)", "PL!N-bp3-008-P | エマ・ヴェルデ (ab#1)", "PL!N-bp3-008-P＋ | エマ・ヴェルデ (ab#1)", "PL!N-bp3-008-SEC | エマ・ヴェルデ (ab#1)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "exclude_self": true, "state_change": "active", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "multiple_targets": true, "resource": "heart"}], "conditional": true, "exclude_self": true, "heart_colors": ["heart04"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "exclude_self": true, "state_change": "active", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "multiple_targets": true, "resource": "heart"}], "conditional": true, "exclude_self": true, "heart_colors": ["heart04"]}
```

- 自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、これによりアクティブにしたメンバーと、このメンバーは、それぞれ{heart_04.png|heart04}を得る (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "multiple_targets": true, "resource": "heart"}
```

- これによりアクティブにしたメンバーと、このメンバーは、それぞれ{heart_04.png|heart04}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp3-009-R＋ | 天王寺璃奈 (ab#0)", "PL!N-bp3-009-P | 天王寺璃奈 (ab#0)", "PL!N-bp3-009-P＋ | 天王寺璃奈 (ab#0)", "PL!N-bp3-009-SEC | 天王寺璃奈 (ab#0)"], "cost": {"card_type": "member_card", "count": 2, "destination": "deck_bottom", "optional": true, "placement_order": "any_order", "source": "discard", "type": "move_cards", "zone": "discard"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "condition": {"aggregate": "total", "comparison_type": "cost", "cost_total": 6, "count": 6, "operator": "=", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "condition": {"aggregate": "total", "comparison_type": "cost", "count": 8, "operator": "=", "type": "comparison_condition"}, "count": 1, "duration": "live_end", "resource": "heart"}, {"action": "modify_score", "condition": {"aggregate": "total", "comparison_type": "cost", "count": 25, "operator": "=", "type": "comparison_condition"}, "duration": "live_end", "operation": "add", "value": 1}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_type": "member_card", "count": 2, "destination": "deck_bottom", "optional": true, "placement_order": "any_order", "source": "discard", "type": "move_cards", "zone": "discard"}
```

- 控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "condition": {"aggregate": "total", "comparison_type": "cost", "cost_total": 6, "count": 6, "operator": "=", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "condition": {"aggregate": "total", "comparison_type": "cost", "count": 8, "operator": "=", "type": "comparison_condition"}, "count": 1, "duration": "live_end", "resource": "heart"}, {"action": "modify_score", "condition": {"aggregate": "total", "comparison_type": "cost", "count": 25, "operator": "=", "type": "comparison_condition"}, "duration": "live_end", "operation": "add", "value": 1}]}
```

- それらのカードのコストの合計が、6の場合、カードを1枚引く。合計が8の場合、ライブ終了時まで、{icon_all.png|ハート}を得る。合計が25の場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"action": "draw_card", "condition": {"aggregate": "total", "comparison_type": "cost", "cost_total": 6, "count": 6, "operator": "=", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- それらのカードのコストの合計が、6の場合、カードを1枚引く (x1)

```json
{"aggregate": "total", "comparison_type": "cost", "cost_total": 6, "count": 6, "operator": "=", "type": "comparison_condition"}
```

- それらのカードのコストの合計が、6の場合 (x1)

```json
{"action": "gain_resource", "condition": {"aggregate": "total", "comparison_type": "cost", "count": 8, "operator": "=", "type": "comparison_condition"}, "count": 1, "duration": "live_end", "resource": "heart"}
```

- 合計が8の場合、ライブ終了時まで、{icon_all.png|ハート}を得る (x1)

```json
{"aggregate": "total", "comparison_type": "cost", "count": 8, "operator": "=", "type": "comparison_condition"}
```

- 合計が8の場合 (x1)

```json
{"action": "modify_score", "condition": {"aggregate": "total", "comparison_type": "cost", "count": 25, "operator": "=", "type": "comparison_condition"}, "duration": "live_end", "operation": "add", "value": 1}
```

- 合計が25の場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する (x1)

```json
{"aggregate": "total", "comparison_type": "cost", "count": 25, "operator": "=", "type": "comparison_condition"}
```

- 合計が25の場合 (x1)

```json
{"card_count": 4, "cards": ["PL!-bp4-002-R＋ | 絢瀬絵里 (ab#0)", "PL!-bp4-002-P | 絢瀬絵里 (ab#0)", "PL!-bp4-002-P＋ | 絢瀬絵里 (ab#0)", "PL!-bp4-002-SEC | 絢瀬絵里 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"ability_filter": "no_ability_type", "ability_filter_triggers": ["live_start", "live_success"], "type": "ability_filter_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"ability_filter": "no_ability_type", "ability_filter_triggers": ["live_start", "live_success"], "type": "ability_filter_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}
```

- {heart_06.png|heart06}{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!-bp4-005-R＋ | 星空 凛 (ab#0)", "PL!-bp4-005-P | 星空 凛 (ab#0)", "PL!-bp4-005-P＋ | 星空凛 (ab#0)", "PL!-bp4-005-SEC | 星空凛 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の控え室からコスト2以下のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!-bp4-005-R＋ | 星空 凛 (ab#1)", "PL!-bp4-005-P | 星空 凛 (ab#1)", "PL!-bp4-005-P＋ | 星空凛 (ab#1)", "PL!-bp4-005-SEC | 星空凛 (ab#1)"], "effect": {"action": "modify_score", "activation_position": "center", "operation": "add", "position": "center", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "activation_position": "center", "operation": "add", "position": "center", "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"card_count": 4, "cards": ["PL!-bp4-005-R＋ | 星空 凛 (ab#2)", "PL!-bp4-005-P | 星空 凛 (ab#2)", "PL!-bp4-005-P＋ | 星空凛 (ab#2)", "PL!-bp4-005-SEC | 星空凛 (ab#2)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 5, "group_names": ["μ's"], "location": "stage", "negation": true, "operator": ">=", "target": "self", "type": "group_condition"}, "exclude_position": "center", "group_names": ["μ's"], "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 5, "group_names": ["μ's"], "location": "stage", "negation": true, "operator": ">=", "target": "self", "type": "group_condition"}, "exclude_position": "center", "group_names": ["μ's"], "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}
```

- 自分のステージに{icon_blade.png|ブレード}を5つ以上持つ『μ's』のメンバーがいない場合、このメンバーはセンターエリア以外にポジションチェンジする (x1)

```json
{"card_type": "member_card", "count": 5, "group_names": ["μ's"], "location": "stage", "negation": true, "operator": ">=", "target": "self", "type": "group_condition"}
```

- 自分のステージに{icon_blade.png|ブレード}を5つ以上持つ『μ's』のメンバーがいない場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-004-R＋ | 朝香果林 (ab#0)", "PL!N-bp4-004-P | 朝香果林 (ab#0)", "PL!N-bp4-004-P＋ | 朝香果林 (ab#0)", "PL!N-bp4-004-SEC | 朝香果林 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "change_state", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "max": true, "state_change": "wait", "target": "opponent"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "change_state", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "max": true, "state_change": "wait", "target": "opponent"}]}
```

- カードを1枚引く。相手のステージにいるコスト9以下のメンバーを1人までウェイトにする (x1)

```json
{"action": "change_state", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "max": true, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるコスト9以下のメンバーを1人までウェイトにする (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-004-R＋ | 朝香果林 (ab#1)", "PL!N-bp4-004-P | 朝香果林 (ab#1)", "PL!N-bp4-004-P＋ | 朝香果林 (ab#1)", "PL!N-bp4-004-SEC | 朝香果林 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "dynamic_count": {"mode": "max", "reference": "相手のステージにいるウェイト状態のメンバー", "type": "dynamic_count"}, "group_names": ["虹ヶ咲"], "source": "discard", "target": "both"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}], "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "dynamic_count": {"mode": "max", "reference": "相手のステージにいるウェイト状態のメンバー", "type": "dynamic_count"}, "group_names": ["虹ヶ咲"], "source": "discard", "target": "both"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}], "group_names": ["虹ヶ咲"]}
```

- 相手のステージにいるウェイト状態のメンバーの数まで、自分の控え室にある『虹ヶ咲』のメンバーカードを選ぶ。それらを好きな順番でデッキの上に置く (x1)

```json
{"action": "select", "card_type": "member_card", "dynamic_count": {"mode": "max", "reference": "相手のステージにいるウェイト状態のメンバー", "type": "dynamic_count"}, "group_names": ["虹ヶ咲"], "source": "discard", "target": "both"}
```

- 相手のステージにいるウェイト状態のメンバーの数まで、自分の控え室にある『虹ヶ咲』のメンバーカードを選ぶ (x1)

```json
{"mode": "max", "reference": "相手のステージにいるウェイト状態のメンバー", "type": "dynamic_count"}
```


```json
{"card_count": 4, "cards": ["PL!N-bp4-007-R＋ | 優木せつ菜 (ab#0)", "PL!N-bp4-007-P | 優木せつ菜 (ab#0)", "PL!N-bp4-007-P＋ | 優木せつ菜 (ab#0)", "PL!N-bp4-007-SEC | 優木せつ菜 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "multiple_targets": true, "source": "discard", "target": "both"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "multiple_targets": true, "source": "discard", "target": "both"}
```

- 自分と相手はそれぞれ、自身の控え室からライブカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-007-R＋ | 優木せつ菜 (ab#1)", "PL!N-bp4-007-P | 優木せつ菜 (ab#1)", "PL!N-bp4-007-P＋ | 優木せつ菜 (ab#1)", "PL!N-bp4-007-SEC | 優木せつ菜 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "count": 15, "location": "energy_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "count": 15, "location": "energy_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02"], "resource": "heart"}
```

- {heart_02.png|heart02}{heart_02.png|heart02}を得る (x1)

```json
{"aggregate": "total", "count": 15, "location": "energy_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}
```

- 自分と相手のエネルギーの合計が15枚以上あるかぎり (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-007-R＋ | 優木せつ菜 (ab#2)", "PL!N-bp4-007-P | 優木せつ菜 (ab#2)", "PL!N-bp4-007-P＋ | 優木せつ菜 (ab#2)", "PL!N-bp4-007-SEC | 優木せつ菜 (ab#2)"], "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "multiple_targets": true, "source": "energy_deck", "state_change": "wait", "target": "both"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "multiple_targets": true, "source": "energy_deck", "state_change": "wait", "target": "both"}
```

- 自分と相手はそれぞれ、自身のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-010-R＋ | 三船栞子 (ab#0)", "PL!N-bp4-010-P | 三船栞子 (ab#0)", "PL!N-bp4-010-P＋ | 三船栞子 (ab#0)", "PL!N-bp4-010-SEC | 三船栞子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "group_names": ["虹ヶ咲"], "optional": true, "source": "success_live_zone", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "success_live_zone", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}], "conditional": true, "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "group_names": ["虹ヶ咲"], "optional": true, "source": "success_live_zone", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "success_live_zone", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}], "conditional": true, "group_names": ["虹ヶ咲"]}
```

- 自分の成功ライブカード置き場にある『虹ヶ咲』のライブカードを1枚控え室に置いてもよい。そうした場合、自分の控え室にある『虹ヶ咲』のライブカードを1枚成功ライブカード置き場に置く (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "group_names": ["虹ヶ咲"], "optional": true, "source": "success_live_zone", "target": "self"}
```

- 自分の成功ライブカード置き場にある『虹ヶ咲』のライブカードを1枚控え室に置いてもよい。 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "success_live_zone", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分の控え室にある『虹ヶ咲』のライブカードを1枚成功ライブカード置き場に置く (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-010-R＋ | 三船栞子 (ab#1)", "PL!N-bp4-010-P | 三船栞子 (ab#1)", "PL!N-bp4-010-P＋ | 三船栞子 (ab#1)", "PL!N-bp4-010-SEC | 三船栞子 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_type": "equality", "location": "success_live_card_zone", "operator": "=", "reference_card": "previous_selected", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}], "group_names": ["虹ヶ咲"], "heart_colors": ["heart04"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_type": "equality", "location": "success_live_card_zone", "operator": "=", "reference_card": "previous_selected", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}], "group_names": ["虹ヶ咲"], "heart_colors": ["heart04"]}
```

- 自分のライブ中の『虹ヶ咲』のライブカードを1枚選ぶ。それと同じカード名のカードが自分の成功ライブカード置き場にある場合、ライブ終了時まで、{heart_04.png|heart04}を得る (x1)

```json
{"action": "select", "card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "target": "self"}
```

- 自分のライブ中の『虹ヶ咲』のライブカードを1枚選ぶ (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_type": "equality", "location": "success_live_card_zone", "operator": "=", "reference_card": "previous_selected", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}
```

- それと同じカード名のカードが自分の成功ライブカード置き場にある場合、ライブ終了時まで、{heart_04.png|heart04}を得る (x1)

```json
{"card_type": "live_card", "comparison_type": "equality", "location": "success_live_card_zone", "operator": "=", "reference_card": "previous_selected", "target": "self", "type": "location_condition"}
```

- それと同じカード名のカードが自分の成功ライブカード置き場にある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp4-011-R＋ | ミア・テイラー (ab#0)", "PL!N-bp4-011-P | ミア・テイラー (ab#0)", "PL!N-bp4-011-P＋ | ミア・テイラー (ab#0)", "PL!N-bp4-011-SEC | ミア・テイラー (ab#0)"], "cost": {"card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "heart"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 4, "cards": ["PL!N-bp4-011-R＋ | ミア・テイラー (ab#1)", "PL!N-bp4-011-P | ミア・テイラー (ab#1)", "PL!N-bp4-011-P＋ | ミア・テイラー (ab#1)", "PL!N-bp4-011-SEC | ミア・テイラー (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}], "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}], "group_names": ["虹ヶ咲"]}
```

- 自分のデッキの上からカードを5枚控え室に置く。その後、自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが3枚以上ある場合、自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが3枚以上ある場合、自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える (x1)

```json
{"card_type": "live_card", "count": 3, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが3枚以上ある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-004-R＋ | 平安名すみれ (ab#0)", "PL!SP-bp4-004-P | 平安名すみれ (ab#0)", "PL!SP-bp4-004-P＋ | 平安名すみれ (ab#0)", "PL!SP-bp4-004-SEC | 平安名すみれ (ab#0)"], "effect": {"action": "play_baton_touch", "count": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "play_baton_touch", "count": 2}
```

- このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-004-R＋ | 平安名すみれ (ab#1)", "PL!SP-bp4-004-P | 平安名すみれ (ab#1)", "PL!SP-bp4-004-P＋ | 平安名すみれ (ab#1)", "PL!SP-bp4-004-SEC | 平安名すみれ (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "center", "count": 2, "destination": "hand", "group_names": ["Liella!"], "position": "center", "source": "deck"}, {"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["Liella!"], "position": "center", "source": "discard", "target": "self"}], "activation_position": "center", "condition": {"appearance": true, "baton_touch_trigger": true, "group_names": ["Liella!"], "location": "stage", "min_baton_touch_count": 2, "position": "center", "type": "appearance_condition"}, "group_names": ["Liella!"], "position": "center"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "center", "count": 2, "destination": "hand", "group_names": ["Liella!"], "position": "center", "source": "deck"}, {"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["Liella!"], "position": "center", "source": "discard", "target": "self"}], "activation_position": "center", "condition": {"appearance": true, "baton_touch_trigger": true, "group_names": ["Liella!"], "location": "stage", "min_baton_touch_count": 2, "position": "center", "type": "appearance_condition"}, "group_names": ["Liella!"], "position": "center"}
```

- 『Liella!』のメンバー2人からバトンタッチして登場している場合、カードを2枚引き、自分の控え室にあるコスト4以下の『Liella!』のメンバーカード1枚を自分のステージのメンバーのいないエリアに登場させる (x1)

```json
{"appearance": true, "baton_touch_trigger": true, "group_names": ["Liella!"], "location": "stage", "min_baton_touch_count": 2, "position": "center", "type": "appearance_condition"}
```

- {center.png|センター}『Liella!』のメンバー2人からバトンタッチして登場している場合 (x1)

```json
{"action": "draw_card", "activation_position": "center", "count": 2, "destination": "hand", "group_names": ["Liella!"], "position": "center", "source": "deck"}
```

- カードを2枚引き (x1)

```json
{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["Liella!"], "position": "center", "source": "discard", "target": "self"}
```

- 自分の控え室にあるコスト4以下の『Liella!』のメンバーカード1枚を自分のステージのメンバーのいないエリアに登場させる (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-005-R＋ | 葉月 恋 (ab#0)", "PL!SP-bp4-005-P | 葉月 恋 (ab#0)", "PL!SP-bp4-005-P＋ | 葉月 恋 (ab#0)", "PL!SP-bp4-005-SEC | 葉月 恋 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "member_card", "conditions": [{"baton_touch_trigger": true, "group_names": ["Liella!"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}], "operator": "and", "target": "self", "type": "compound"}, "count": 2, "destination": "energy_zone", "group_names": ["Liella!"], "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "member_card", "conditions": [{"baton_touch_trigger": true, "group_names": ["Liella!"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}], "operator": "and", "target": "self", "type": "compound"}, "count": 2, "destination": "energy_zone", "group_names": ["Liella!"], "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 『Liella!』のメンバーからバトンタッチして登場しており、かつ自分のエネルギーが7枚以上ある場合、自分のエネルギーデッキから、エネルギーカードを2枚ウェイト状態で置く (x1)

```json
{"card_type": "member_card", "conditions": [{"baton_touch_trigger": true, "group_names": ["Liella!"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}], "operator": "and", "target": "self", "type": "compound"}
```

- 『Liella!』のメンバーからバトンタッチして登場しており、かつ自分のエネルギーが7枚以上ある場合 (x1)

```json
{"baton_touch_trigger": true, "group_names": ["Liella!"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 『Liella!』のメンバーからバトンタッチして登場しており、 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-005-R＋ | 葉月 恋 (ab#1)", "PL!SP-bp4-005-P | 葉月 恋 (ab#1)", "PL!SP-bp4-005-P＋ | 葉月 恋 (ab#1)", "PL!SP-bp4-005-SEC | 葉月 恋 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "count": 3, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "count": 3, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-008-R＋ | 若菜四季 (ab#0)", "PL!SP-bp4-008-P | 若菜四季 (ab#0)", "PL!SP-bp4-008-P＋ | 若菜四季 (ab#0)", "PL!SP-bp4-008-SEC | 若菜四季 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side", "count": 2, "destination": "hand", "position": "left_side", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 1, "destination": "discard", "position": "left_side", "source": "hand"}], "activation_position": "left_side", "position": "left_side"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side", "count": 2, "destination": "hand", "position": "left_side", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 1, "destination": "discard", "position": "left_side", "source": "hand"}], "activation_position": "left_side", "position": "left_side"}
```

- カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"action": "draw_card", "activation_position": "left_side", "count": 2, "destination": "hand", "position": "left_side", "source": "deck"}
```

- カードを2枚引き (x1)

```json
{"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 1, "destination": "discard", "position": "left_side", "source": "hand"}
```

- 手札を1枚控え室に置く (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-008-R＋ | 若菜四季 (ab#1)", "PL!SP-bp4-008-P | 若菜四季 (ab#1)", "PL!SP-bp4-008-P＋ | 若菜四季 (ab#1)", "PL!SP-bp4-008-SEC | 若菜四季 (ab#1)"], "effect": {"action": "change_state", "activation_position": "right_side", "card_type": "energy_card", "count": 2, "position": "right_side", "state_change": "active"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "activation_position": "right_side", "card_type": "energy_card", "count": 2, "position": "right_side", "state_change": "active"}
```

- エネルギーを2枚アクティブにする (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-008-R＋ | 若菜四季 (ab#2)", "PL!SP-bp4-008-P | 若菜四季 (ab#2)", "PL!SP-bp4-008-P＋ | 若菜四季 (ab#2)", "PL!SP-bp4-008-SEC | 若菜四季 (ab#2)"], "effect": {"action": "position_change", "card_type": "member_card", "optional": true, "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 4, "cards": ["PL!SP-bp4-011-R＋ | 鬼塚冬毬 (ab#0)", "PL!SP-bp4-011-P | 鬼塚冬毬 (ab#0)", "PL!SP-bp4-011-P＋ | 鬼塚冬毬 (ab#0)", "PL!SP-bp4-011-SEC | 鬼塚冬毬 (ab#0)"], "effect": {"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}
```

- このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が3つ以下のメンバー1人をウェイトにする (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp4-013-N | 唐 可可 (ab#0)", "PL!SP-sd2-005-SD2 | 葉月 恋 (ab#0)", "PL!SP-sd2-007-SD2 | 米女メイ (ab#0)", "PL!SP-sd2-016-SD2 | 葉月 恋 (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "optional": true, "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 4, "cards": ["PL!-bp5-001-R＋ | 高坂穂乃果 (ab#0)", "PL!-bp5-001-P | 高坂穂乃果 (ab#0)", "PL!-bp5-001-AR | 高坂穂乃果 (ab#0)", "PL!-bp5-001-SEC | 高坂穂乃果 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 1, "dynamic_count": {"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "total_live_score", "type": "dynamic_count"}, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 1, "dynamic_count": {"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "total_live_score", "type": "dynamic_count"}, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}
```

- 自分のデッキの上から、自分のライブの合計スコアに2を足した数に等しい枚数見る。その中からカードを1枚手札に加える。残りを控え室に置く (x1)

```json
{"action": "look_at", "count": 1, "dynamic_count": {"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "total_live_score", "type": "dynamic_count"}, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上から、自分のライブの合計スコアに2を足した数に等しい枚数見る。 (x1)

```json
{"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "total_live_score", "type": "dynamic_count"}
```


```json
{"card_count": 4, "cards": ["PL!-bp5-003-R＋ | 南 ことり (ab#0)", "PL!-bp5-003-P | 南 ことり (ab#0)", "PL!-bp5-003-AR | 南 ことり (ab#0)", "PL!-bp5-003-SEC | 南 ことり (ab#0)"], "effect": {"action": "gain_resource", "condition": {"count": 3, "distinct": "card_name", "heart_colors": ["heart03"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"count": 3, "distinct": "card_name", "heart_colors": ["heart03"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}
```

- {heart_03.png|heart03}を得る (x1)

```json
{"count": 3, "distinct": "card_name", "heart_colors": ["heart03"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージに名前が異なるメンバーが3人以上いるかぎり (x1)

```json
{"card_count": 4, "cards": ["PL!-bp5-003-R＋ | 南 ことり (ab#1)", "PL!-bp5-003-P | 南 ことり (ab#1)", "PL!-bp5-003-AR | 南 ことり (ab#1)", "PL!-bp5-003-SEC | 南 ことり (ab#1)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "conditional_alternative", "alternative_effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "condition": {"group_names": ["μ's"], "location": "discard", "type": "group_condition"}, "group_names": ["μ's"], "primary_effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 2, "destination": "hand", "discard_remaining": true}}}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "conditional_alternative", "alternative_effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "condition": {"group_names": ["μ's"], "location": "discard", "type": "group_condition"}, "group_names": ["μ's"], "primary_effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 2, "destination": "hand", "discard_remaining": true}}}
```

- これにより控え室に置いたカードが『μ's』のカードの場合、自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く。『μ's』のカード以外の場合、自分の控え室からライブカードを1枚手札に加える (x1)

```json
{"group_names": ["μ's"], "location": "discard", "type": "group_condition"}
```

- これにより控え室に置いたカードが『μ's』のカードの場合 (x1)

```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 2, "destination": "hand", "discard_remaining": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からカードを2枚手札に加える。残りを控え室に置く (x1)

```json
{"card_count": 4, "cards": ["PL!-bp5-004-R＋ | 園田海未 (ab#0)", "PL!-bp5-004-P | 園田海未 (ab#0)", "PL!-bp5-004-AR | 園田海未 (ab#0)", "PL!-bp5-004-SEC | 園田海未 (ab#0)"], "cost": {"count": 4, "energy": 4, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "modify_cost", "count": 1, "dynamic_count": {"reference": "unit_count", "type": "per_unit"}, "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_location": "stage", "per_unit_type": "group_name"}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "modify_cost", "count": 1, "dynamic_count": {"reference": "unit_count", "type": "per_unit"}, "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_location": "stage", "per_unit_type": "group_name"}
```

- {icon_energy.png|E}減る (x1)

```json
{"card_count": 4, "cards": ["PL!-bp5-004-R＋ | 園田海未 (ab#1)", "PL!-bp5-004-P | 園田海未 (ab#1)", "PL!-bp5-004-AR | 園田海未 (ab#1)", "PL!-bp5-004-SEC | 園田海未 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 3, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 3, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "heart"}
```

- 自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{icon_all.png|ハート}を得る (x1)

```json
{"card_type": "member_card", "comparison_target": "self", "count": 3, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp5-001-R＋ | 高海千歌 (ab#0)", "PL!S-bp5-001-P | 高海千歌 (ab#0)", "PL!S-bp5-001-AR | 高海千歌 (ab#0)", "PL!S-bp5-001-SEC | 高海千歌 (ab#0)"], "effect": {"action": "draw_card", "condition": {"ability_filter": "no_ability", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"ability_filter": "no_ability", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 能力を持たないメンバーからバトンタッチして登場した場合、カードを1枚引く (x1)

```json
{"ability_filter": "no_ability", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 能力を持たないメンバーからバトンタッチして登場した場合 (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp5-001-R＋ | 高海千歌 (ab#1)", "PL!S-bp5-001-P | 高海千歌 (ab#1)", "PL!S-bp5-001-AR | 高海千歌 (ab#1)", "PL!S-bp5-001-SEC | 高海千歌 (ab#1)"], "effect": {"action": "modify_cost", "card_type": "member_card", "destination": "stage", "location": "hand", "operation": "subtract", "source": "hand", "target": "self", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "destination": "stage", "location": "hand", "operation": "subtract", "source": "hand", "target": "self", "value": 1}
```

- 能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp5-002-R＋ | 桜内梨子 (ab#0)", "PL!S-bp5-002-P | 桜内梨子 (ab#0)", "PL!S-bp5-002-AR | 桜内梨子 (ab#0)", "PL!S-bp5-002-SEC | 桜内梨子 (ab#0)"], "effect": {"action": "change_state", "activation_position": "center", "all": true, "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"card_type": "member_card", "comparison_type": "equality", "location": "stage", "operator": "=", "position": "left_side", "position_compare": "right_side", "target": "self", "type": "location_condition"}, "original_value": true, "position": "left_side", "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "change_state", "activation_position": "center", "all": true, "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"card_type": "member_card", "comparison_type": "equality", "location": "stage", "operator": "=", "position": "left_side", "position_compare": "right_side", "target": "self", "type": "location_condition"}, "original_value": true, "position": "left_side", "state_change": "wait", "target": "opponent"}
```

- 自分のステージの右サイドエリアと左サイドエリアにいるメンバーのコストが同じ場合、相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が3つ以下のすべてのメンバーをウェイトにする (x1)

```json
{"card_type": "member_card", "comparison_type": "equality", "location": "stage", "operator": "=", "position": "left_side", "position_compare": "right_side", "target": "self", "type": "location_condition"}
```

- {center.png|センター}自分のステージの右サイドエリアと左サイドエリアにいるメンバーのコストが同じ場合 (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp5-005-R＋ | 渡辺 曜 (ab#0)", "PL!S-bp5-005-P | 渡辺 曜 (ab#0)", "PL!S-bp5-005-AR | 渡辺 曜 (ab#0)", "PL!S-bp5-005-SEC | 渡辺 曜 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "select", "all": true, "count": 1, "group_names": ["Aqours"], "heart_colors": ["heart03", "heart04", "heart05"]}, {"action": "gain_resource", "all": true, "card_type": "member_card", "duration": "live_end", "exclude_group_names": ["Aqours"], "group_names": null, "heart_colors": ["heart03", "heart04", "heart05"], "resource": "heart", "target": "self", "timing_condition": "appeared_this_turn"}], "all": true, "group_names": ["Aqours"], "heart_colors": ["heart03", "heart04", "heart05"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "all": true, "count": 1, "group_names": ["Aqours"], "heart_colors": ["heart03", "heart04", "heart05"]}, {"action": "gain_resource", "all": true, "card_type": "member_card", "duration": "live_end", "exclude_group_names": ["Aqours"], "group_names": null, "heart_colors": ["heart03", "heart04", "heart05"], "resource": "heart", "target": "self", "timing_condition": "appeared_this_turn"}], "all": true, "group_names": ["Aqours"], "heart_colors": ["heart03", "heart04", "heart05"]}
```

- {heart_03.png|heart03}か{heart_04.png|heart04}か{heart_05.png|heart05}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいるこのターンに登場したメンバーのうち、『Aqours』以外のすべてのメンバーは選んだハートを1つ得る (x1)

```json
{"action": "select", "all": true, "count": 1, "group_names": ["Aqours"], "heart_colors": ["heart03", "heart04", "heart05"]}
```

- {heart_03.png|heart03}か{heart_04.png|heart04}か{heart_05.png|heart05}のうち、1つを選ぶ (x1)

```json
{"action": "gain_resource", "all": true, "card_type": "member_card", "duration": "live_end", "exclude_group_names": ["Aqours"], "group_names": null, "heart_colors": ["heart03", "heart04", "heart05"], "resource": "heart", "target": "self", "timing_condition": "appeared_this_turn"}
```

- 自分のステージにいるこのターンに登場したメンバーのうち、『Aqours』以外のすべてのメンバーは選んだハートを1つ得る (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-001-R＋ | 上原歩夢 (ab#0)", "PL!N-bp5-001-P | 上原歩夢 (ab#0)", "PL!N-bp5-001-AR | 上原歩夢 (ab#0)", "PL!N-bp5-001-SEC | 上原歩夢 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "condition": {"comparison_target": "self", "count": 3, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}, "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}, {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"count": 6, "operator": ">=", "type": "card_count_condition", "unit": "types"}, "duration": "live_end"}], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"]}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "condition": {"comparison_target": "self", "count": 3, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}, "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}, {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"count": 6, "operator": ">=", "type": "card_count_condition", "unit": "types"}, "duration": "live_end"}], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"]}
```

- 自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}、{icon_all.png|ハート}のうち、3種類以上ある場合、ライブ終了時まで、{heart_01.png|heart01}を得る。6種類以上ある場合、さらにライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"action": "gain_resource", "condition": {"comparison_target": "self", "count": 3, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}, "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}
```

- 自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}、{icon_all.png|ハート}のうち、3種類以上ある場合、ライブ終了時まで、{heart_01.png|heart01}を得る (x1)

```json
{"comparison_target": "self", "count": 3, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}
```

- 自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}、{icon_all.png|ハート}のうち、3種類以上ある場合 (x1)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"count": 6, "operator": ">=", "type": "card_count_condition", "unit": "types"}, "duration": "live_end"}
```

- 6種類以上ある場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"count": 6, "operator": ">=", "type": "card_count_condition", "unit": "types"}
```

- 6種類以上ある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-005-R＋ | 宮下 愛 (ab#0)", "PL!N-bp5-005-P | 宮下 愛 (ab#0)", "PL!N-bp5-005-AR | 宮下 愛 (ab#0)", "PL!N-bp5-005-SEC | 宮下 愛 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "count": 2, "state_change": "active"}, {"action": "draw_card", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "deck"}], "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "count": 2, "state_change": "active"}, {"action": "draw_card", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "deck"}], "group_names": ["虹ヶ咲"]}
```

- このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、さらにカードを1枚引く (x1)

```json
{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "count": 2, "state_change": "active"}
```

- このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする (x1)

```json
{"action": "draw_card", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "deck"}
```

- コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、カードを1枚引く (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-007-R＋ | 優木せつ菜 (ab#0)", "PL!N-bp5-007-P | 優木せつ菜 (ab#0)", "PL!N-bp5-007-AR | 優木せつ菜 (ab#0)", "PL!N-bp5-007-SEC | 優木せつ菜 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_type": "equality", "heart_colors": ["heart02"], "location": "success_live_card_zone", "operator": "=", "scope": "both", "target": "both", "type": "location_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_type": "equality", "heart_colors": ["heart02"], "location": "success_live_card_zone", "operator": "=", "scope": "both", "target": "both", "type": "location_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}
```

- 自分と相手の成功ライブカード置き場にあるカードの枚数が同じ場合、ライブ終了時まで、{heart_02.png|heart02}{heart_02.png|heart02}を得る (x1)

```json
{"card_type": "live_card", "comparison_type": "equality", "heart_colors": ["heart02"], "location": "success_live_card_zone", "operator": "=", "scope": "both", "target": "both", "type": "location_condition"}
```

- 自分と相手の成功ライブカード置き場にあるカードの枚数が同じ場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-007-R＋ | 優木せつ菜 (ab#1)", "PL!N-bp5-007-P | 優木せつ菜 (ab#1)", "PL!N-bp5-007-AR | 優木せつ菜 (ab#1)", "PL!N-bp5-007-SEC | 優木せつ菜 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"count": 1, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"count": 1, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}}
```

- 自分が余剰ハートを1つ以上持っている場合、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"count": 1, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}
```

- 自分が余剰ハートを1つ以上持っている場合 (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-012-R＋ | 鐘 嵐珠 (ab#0)", "PL!N-bp5-012-P | 鐘 嵐珠 (ab#0)", "PL!N-bp5-012-AR | 鐘 嵐珠 (ab#0)", "PL!N-bp5-012-SEC | 鐘 嵐珠 (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "destination": "under_member", "type": "custom"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "heart_colors": ["heart01"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "heart_colors": ["heart01"]}
```

- カードを1枚引き、ライブ終了時まで、{heart_01.png|heart01}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-012-R＋ | 鐘 嵐珠 (ab#1)", "PL!N-bp5-012-P | 鐘 嵐珠 (ab#1)", "PL!N-bp5-012-AR | 鐘 嵐珠 (ab#1)", "PL!N-bp5-012-SEC | 鐘 嵐珠 (ab#1)"], "effect": {"action": "place_energy_under_member", "card_type": "member_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "destination": "energy_zone", "energy_count": 1, "source": "under_member", "state_change": "wait", "target": "self", "target_member": "this_member"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "place_energy_under_member", "card_type": "member_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "destination": "energy_zone", "energy_count": 1, "source": "under_member", "state_change": "wait", "target": "self", "target_member": "this_member"}
```

- ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、このメンバーの下にあるエネルギーカードの枚数に1を足した枚数のエネルギーカードをウェイト状態で置く (x1)

```json
{"card_count": 4, "cards": ["PL!N-bp5-016-N | 朝香果林 (ab#0)", "PL!N-bp5-023-N | ミア・テイラー (ab#0)", "PL!S-sd1-014-SD | 渡辺 曜 (ab#0)", "PL!SP-sd2-017-SD2 | 桜小路きな子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 4, "cards": ["PL!SP-bp5-001-R＋ | 澁谷かのん (ab#0)", "PL!SP-bp5-001-P | 澁谷かのん (ab#0)", "PL!SP-bp5-001-AR | 澁谷かのん (ab#0)", "PL!SP-bp5-001-SEC | 澁谷かのん (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "choice", "count": 1, "options": [{"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}]}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"action": "choice", "count": 1, "options": [{"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}]}
```

- 以下から1つを選ぶ。
・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
・カードを1枚引く (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-001-R＋ | 澁谷かのん (ab#3)", "PL!SP-bp5-001-P | 澁谷かのん (ab#3)", "PL!SP-bp5-001-AR | 澁谷かのん (ab#3)", "PL!SP-bp5-001-SEC | 澁谷かのん (ab#3)"], "cost": {"options": [{"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "choice_condition"}, "effect": {"action": "change_state", "card_type": "energy_card", "count": 1, "state_change": "active"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"options": [{"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "choice_condition"}
```

- このメンバーをウェイトにするか、手札を1枚控え室に置く (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-002-R＋ | 唐 可可 (ab#0)", "PL!SP-bp5-002-P | 唐 可可 (ab#0)", "PL!SP-bp5-002-AR | 唐 可可 (ab#0)", "PL!SP-bp5-002-SEC | 唐 可可 (ab#0)"], "cost": {"card_type": "member_card", "position": "left_side", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side", "count": 3, "destination": "hand", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "activation_position": "left_side"}, {"action": "change_state", "activation_position": "left_side", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "negation": true, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "state_change": "active"}, {"action": "gain_resource", "activation_position": "left_side", "condition": {"card_type": "member_card", "count": 2, "negation": true, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}], "activation_position": "left_side"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "position": "left_side", "self_cost": true, "state_change": "wait", "type": "change_state"}
```

- {leftside.png|左サイド}このメンバーをウェイトにする (x1)

```json
{"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side", "count": 3, "destination": "hand", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "activation_position": "left_side"}, {"action": "change_state", "activation_position": "left_side", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "negation": true, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "state_change": "active"}, {"action": "gain_resource", "activation_position": "left_side", "condition": {"card_type": "member_card", "count": 2, "negation": true, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}], "activation_position": "left_side"}
```

- カードを3枚引き、手札を2枚控え室に置く。これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、このメンバーをアクティブにする。2枚ある場合、さらにライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side", "count": 3, "destination": "hand", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "activation_position": "left_side"}
```

- カードを3枚引き、手札を2枚控え室に置く (x1)

```json
{"action": "draw_card", "activation_position": "left_side", "count": 3, "destination": "hand", "source": "deck"}
```

- カードを3枚引き (x1)

```json
{"action": "move_cards", "activation_position": "left_side", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}
```

- 手札を2枚控え室に置く (x1)

```json
{"action": "change_state", "activation_position": "left_side", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "negation": true, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "state_change": "active"}
```

- これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、このメンバーをアクティブにする (x1)

```json
{"card_type": "member_card", "count": 1, "negation": true, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}
```

- これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合 (x1)

```json
{"action": "gain_resource", "activation_position": "left_side", "condition": {"card_type": "member_card", "count": 2, "negation": true, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- 2枚ある場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "count": 2, "negation": true, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}
```

- 2枚ある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-003-R＋ | 嵐 千砂都 (ab#0)", "PL!SP-bp5-003-P | 嵐 千砂都 (ab#0)", "PL!SP-bp5-003-AR | 嵐 千砂都 (ab#0)", "PL!SP-bp5-003-SEC | 嵐 千砂都 (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": "=", "destination": "stage", "group_names": ["Liella!"], "location": "hand", "operation": "subtract", "source": "hand", "target": "self", "value": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": "=", "destination": "stage", "group_names": ["Liella!"], "location": "hand", "operation": "subtract", "source": "hand", "target": "self", "value": 2}
```

- コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-003-R＋ | 嵐 千砂都 (ab#1)", "PL!SP-bp5-003-P | 嵐 千砂都 (ab#1)", "PL!SP-bp5-003-AR | 嵐 千砂都 (ab#1)", "PL!SP-bp5-003-SEC | 嵐 千砂都 (ab#1)"], "effect": {"action": "change_state", "activation_position": "center", "all": true, "card_type": "member_card", "group_names": ["Liella!"], "position": "center", "state_change": "active", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "change_state", "activation_position": "center", "all": true, "card_type": "member_card", "group_names": ["Liella!"], "position": "center", "state_change": "active", "target": "self"}
```

- 自分のステージにいるすべての『Liella!』のメンバーと、自分のすべてのエネルギーをアクティブにする (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-004-R＋ | 平安名すみれ (ab#0)", "PL!SP-bp5-004-P | 平安名すみれ (ab#0)", "PL!SP-bp5-004-AR | 平安名すみれ (ab#0)", "PL!SP-bp5-004-SEC | 平安名すみれ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}], "condition": {"energy_placed": true, "movement": "moves", "self_effect_only": true, "type": "movement_condition"}, "heart_colors": ["heart02"]}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}], "condition": {"energy_placed": true, "movement": "moves", "self_effect_only": true, "type": "movement_condition"}, "heart_colors": ["heart02"]}
```

- 自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき、カードを1枚引き、ライブ終了時まで、{heart_02.png|heart02}を得る (x1)

```json
{"energy_placed": true, "movement": "moves", "self_effect_only": true, "type": "movement_condition"}
```

- 自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-005-R＋ | 葉月 恋 (ab#0)", "PL!SP-bp5-005-P | 葉月 恋 (ab#0)", "PL!SP-bp5-005-AR | 葉月 恋 (ab#0)", "PL!SP-bp5-005-SEC | 葉月 恋 (ab#0)"], "cost": {"count": 3, "destination": "discard", "source": "deck_top", "type": "move_cards", "zone": "deck_top"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Liella!"], "per_unit": true, "per_unit_count": 1, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard", "resource": "blade"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Liella!"], "per_unit": true, "per_unit_count": 1, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard", "resource": "blade"}
```

- これにより控え室に置いた『Liella!』のメンバーカード1枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-005-R＋ | 葉月 恋 (ab#1)", "PL!SP-bp5-005-P | 葉月 恋 (ab#1)", "PL!SP-bp5-005-AR | 葉月 恋 (ab#1)", "PL!SP-bp5-005-SEC | 葉月 恋 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "pay_energy", "count": 1, "energy": 1, "optional": true}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "those_cards"}], "conditional": true, "trigger_condition": {"count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "pay_energy", "count": 1, "energy": 1, "optional": true}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "those_cards"}], "conditional": true, "trigger_condition": {"count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "trigger_type": "each_time"}
```

- 自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{icon_energy.png|E}支払ってもよい。そうした場合、それらのカードの中から1枚手札に加える (x1)

```json
{"action": "pay_energy", "count": 1, "energy": 1, "optional": true}
```

- {icon_energy.png|E}支払ってもよい。 (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "those_cards"}
```

- それらのカードの中から1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp5-001-R＋ | 日野下花帆 (ab#0)", "PL!HS-bp5-001-P | 日野下花帆 (ab#0)", "PL!HS-bp5-001-AR | 日野下花帆 (ab#0)", "PL!HS-bp5-001-SEC | 日野下花帆 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 4, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 4, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}]}
```

- 自分のデッキの上からカードを4枚控え室に置く。それらの中にライブカードがある場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- それらの中にライブカードがある場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp5-001-R＋ | 日野下花帆 (ab#1)", "PL!HS-bp5-001-P | 日野下花帆 (ab#1)", "PL!HS-bp5-001-AR | 日野下花帆 (ab#1)", "PL!HS-bp5-001-SEC | 日野下花帆 (ab#1)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "live_card", "count": 1, "source": "hand", "type": "reveal", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "name_constraint": "contains_all", "name_constraint_source": "revealed_card", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "live_card", "count": 1, "source": "hand", "type": "reveal", "zone": "hand"}], "type": "sequential_cost"}
```

- {icon_energy.png|E}{icon_energy.png|E}手札のライブカードを1枚公開する (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "name_constraint": "contains_all", "name_constraint_source": "revealed_card", "source": "discard", "target": "self"}
```

- 自分の控え室から、これにより公開したカードのカード名がすべて含まれるライブカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp5-002-R＋ | 村野さやか (ab#0)", "PL!HS-bp5-002-P | 村野さやか (ab#0)", "PL!HS-bp5-002-AR | 村野さやか (ab#0)", "PL!HS-bp5-002-SEC | 村野さやか (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"card_type": "member_card", "count": 3, "distinct": "cost", "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart05"]}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"card_type": "member_card", "count": 3, "distinct": "cost", "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart05"]}
```

- {heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "count": 3, "distinct": "cost", "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージにコストがそれぞれ異なるメンバーが3人以上いるかぎり (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp5-002-R＋ | 村野さやか (ab#1)", "PL!HS-bp5-002-P | 村野さやか (ab#1)", "PL!HS-bp5-002-AR | 村野さやか (ab#1)", "PL!HS-bp5-002-SEC | 村野さやか (ab#1)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "source": "discard", "target": "self"}
```

- 自分の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアに登場させる (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp5-003-R＋ | 大沢瑠璃乃 (ab#0)", "PL!HS-bp5-003-P | 大沢瑠璃乃 (ab#0)", "PL!HS-bp5-003-AR | 大沢瑠璃乃 (ab#0)", "PL!HS-bp5-003-SEC | 大沢瑠璃乃 (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "count": 1, "optional": true, "target_member": "select"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "count": 1, "optional": true, "target_member": "select"}
```

- このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp5-003-R＋ | 大沢瑠璃乃 (ab#1)", "PL!HS-bp5-003-P | 大沢瑠璃乃 (ab#1)", "PL!HS-bp5-003-AR | 大沢瑠璃乃 (ab#1)", "PL!HS-bp5-003-SEC | 大沢瑠璃乃 (ab#1)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "group_reference": "same_group_name", "heart_colors": ["heart01"], "resource": "heart", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "group_reference": "same_group_name", "heart_colors": ["heart01"], "resource": "heart", "target_count": 1}
```

- これにより控え室に置いたカードと同じグループ名を持つメンバー1人は、{heart_01.png|heart01}を得る (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp5-111-R | 鹿角聖良 (ab#0)", "PL!S-bp5-111-P＋ | 鹿角聖良 (ab#0)", "PL!S-bp5-222-R | 鹿角理亞 (ab#0)", "PL!S-bp5-222-P＋ | 鹿角理亞 (ab#0)"], "cost": {"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "position_change", "card_type": "member_card", "group_names": ["Aqours", "SaintSnow"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "position_change", "card_type": "member_card", "group_names": ["Aqours", "SaintSnow"]}
```

- このメンバーを『Aqours』か『SaintSnow』のメンバーがいるエリアにポジションチェンジする (x1)

```json
{"card_count": 4, "cards": ["PL!SP-bp5-111-R | 柊摩央 (ab#0)", "PL!SP-bp5-111-P＋ | 柊摩央 (ab#0)", "PL!SP-bp5-222-R | 聖澤悠奈 (ab#0)", "PL!SP-bp5-222-P＋ | 聖澤悠奈 (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 8, "location": "energy_zone", "operator": "=", "target": "self", "type": "card_count_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "condition": {"count": 8, "location": "energy_zone", "operator": "=", "target": "self", "type": "card_count_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"count": 8, "location": "energy_zone", "operator": "=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーがちょうど8枚あるかぎり (x1)

```json
{"card_count": 4, "cards": ["PL!-bp6-006-R＋ | 西木野真姫 (ab#0)", "PL!-bp6-006-P | 西木野真姫 (ab#0)", "PL!-bp6-006-P＋ | 西木野真姫 (ab#0)", "PL!-bp6-006-SEC | 西木野真姫 (ab#0)"], "cost": {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "condition": {"aggregate": "total", "card_type": "member_card", "location": "stage", "type": "location_condition"}, "group_names": ["μ's"], "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "reveal": false}}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "look_and_select", "condition": {"aggregate": "total", "card_type": "member_card", "location": "stage", "type": "location_condition"}, "group_names": ["μ's"], "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "reveal": false}}
```

- 好きなハートの色を1つ指定する。その後、自分のデッキの上からカードを5枚公開する。公開されたカードの中に指定した色のハートを持つメンバーカードと必要ハートに指定した色を含むライブカードが合計5枚含まれる場合、その中から『μ's』のカードを1枚手札に加え、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る。公開した残りのカードを控え室に置く (x1)

```json
{"aggregate": "total", "card_type": "member_card", "location": "stage", "type": "location_condition"}
```

- 好きなハートの色を1つ指定する。その後、自分のデッキの上からカードを5枚公開する。公開されたカードの中に指定した色のハートを持つメンバーカードと必要ハートに指定した色を含むライブカードが合計5枚含まれる場合 (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "reveal": false}
```


```json
{"card_count": 4, "cards": ["PL!-bp6-007-R＋ | 東條 希 (ab#0)", "PL!-bp6-007-P | 東條 希 (ab#0)", "PL!-bp6-007-P＋ | 東條 希 (ab#0)", "PL!-bp6-007-SEC | 東條 希 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "deck_top", "target": "self"}, {"action": "modify_score", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "operation": "add", "value": 1}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "deck_top", "target": "self"}, {"action": "modify_score", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "operation": "add", "value": 1}]}
```

- 自分のデッキの一番上のカードを公開し、手札に加える。それがブレードハートを持たないメンバーカードの場合、ライブの合計スコアを+1する (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "deck_top", "target": "self"}
```

- 自分のデッキの一番上のカードを公開し、手札に加える (x1)

```json
{"action": "modify_score", "condition": {"card_type": "member_card", "location": "stage", "negation": true, "type": "location_condition"}, "operation": "add", "value": 1}
```

- それがブレードハートを持たないメンバーカードの場合、ライブの合計スコアを+1する (x1)

```json
{"card_count": 4, "cards": ["PL!S-bp6-004-R＋ | 黒澤ダイヤ (ab#0)", "PL!S-bp6-004-P | 黒澤ダイヤ (ab#0)", "PL!S-bp6-004-P＋ | 黒澤ダイヤ (ab#0)", "PL!S-bp6-004-SEC | 黒澤ダイヤ (ab#0)"], "effect": {"action": "look_and_select", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["Aqours"], "heart_colors": ["heart02", "heart04"], "look_action": {"action": "look_at", "group_names": ["Aqours"], "source": "live_card_zone", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "deck_top", "discard_remaining": true, "group_names": ["Aqours"], "heart_colors": ["heart02", "heart04"], "optional": true, "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "look_and_select", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["Aqours"], "heart_colors": ["heart02", "heart04"], "look_action": {"action": "look_at", "group_names": ["Aqours"], "source": "live_card_zone", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "deck_top", "discard_remaining": true, "group_names": ["Aqours"], "heart_colors": ["heart02", "heart04"], "optional": true, "reveal": false}}
```

- 自分のライブカード置き場にカードが2枚以上ある場合、その中から{live_start.png|ライブ開始時}能力を持たない『Aqours』のライブカードを1枚選び、デッキの一番上に置いてもよい。そうした場合、ライブ終了時まで、{heart_02.png|heart02}と{heart_04.png|heart04}を得る (x1)

```json
{"action": "look_at", "group_names": ["Aqours"], "source": "live_card_zone", "target": "self"}
```


```json
{"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "deck_top", "discard_remaining": true, "group_names": ["Aqours"], "heart_colors": ["heart02", "heart04"], "optional": true, "reveal": false}
```


```json
{"card_count": 4, "cards": ["PL!S-bp6-009-R＋ | 黒澤ルビィ (ab#0)", "PL!S-bp6-009-P | 黒澤ルビィ (ab#0)", "PL!S-bp6-009-P＋ | 黒澤ルビィ (ab#0)", "PL!S-bp6-009-SEC | 黒澤ルビィ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_target": "self", "location": "success_live_card_zone", "operator": ">", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "dynamic_count": {"mode": "equals", "reference": "その差", "type": "dynamic_count"}, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "comparison_target": "self", "location": "success_live_card_zone", "operator": ">", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "dynamic_count": {"mode": "equals", "reference": "その差", "type": "dynamic_count"}, "resource": "blade"}
```

- その差に等しい数の{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "live_card", "comparison_target": "self", "location": "success_live_card_zone", "operator": ">", "target": "opponent", "type": "comparison_condition"}
```

- 相手の成功ライブカード置き場にあるカードの枚数が自分より多いかぎり (x1)

```json
{"mode": "equals", "reference": "その差", "type": "dynamic_count"}
```


```json
{"card_count": 4, "cards": ["PL!S-bp6-009-R＋ | 黒澤ルビィ (ab#1)", "PL!S-bp6-009-P | 黒澤ルビィ (ab#1)", "PL!S-bp6-009-P＋ | 黒澤ルビィ (ab#1)", "PL!S-bp6-009-SEC | 黒澤ルビィ (ab#1)"], "effect": {"action": "modify_score", "activation_position": "center", "condition": {"location": "revealed_cards", "position": "center", "target": "self", "type": "location_condition"}, "group_names": ["Aqours"], "operation": "add", "position": "center", "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "activation_position": "center", "condition": {"location": "revealed_cards", "position": "center", "target": "self", "type": "location_condition"}, "group_names": ["Aqours"], "operation": "add", "position": "center", "value": 1}
```

- エールにより公開された自分のカードの中に、{icon_score.png|スコア}を持つ『Aqours』のライブカードがある場合、ライブの合計スコアを+1する (x1)

```json
{"location": "revealed_cards", "position": "center", "target": "self", "type": "location_condition"}
```

- {center.png|センター}エールにより公開された自分のカードの中に、{icon_score.png|スコア}を持つ『Aqours』のライブカードがある場合 (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp6-001-R＋ | 日野下花帆 (ab#0)", "PL!HS-bp6-001-P | 日野下花帆 (ab#0)", "PL!HS-bp6-001-P＋ | 日野下花帆 (ab#0)", "PL!HS-bp6-001-SEC | 日野下花帆 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "card_type": "member_card", "count": 1, "dynamic_count": {"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "自分のデッキの上から、自分のステージにいるメンバーの数", "type": "dynamic_count"}, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "reveal": false}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "card_type": "member_card", "count": 1, "dynamic_count": {"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "自分のデッキの上から、自分のステージにいるメンバーの数", "type": "dynamic_count"}, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "reveal": false}}
```

- 自分のデッキの上から、自分のステージにいるメンバーの数に2を足した数に等しい枚数見る。その中から1枚をデッキの一番上に置き、残りを控え室に置く (x1)

```json
{"action": "look_at", "card_type": "member_card", "count": 1, "dynamic_count": {"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "自分のデッキの上から、自分のステージにいるメンバーの数", "type": "dynamic_count"}, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上から、自分のステージにいるメンバーの数に2を足した数に等しい枚数見る。 (x1)

```json
{"calculation": "add", "calculation_value": 2, "mode": "equals", "reference": "自分のデッキの上から、自分のステージにいるメンバーの数", "type": "dynamic_count"}
```


```json
{"card_count": 4, "cards": ["PL!HS-bp6-001-R＋ | 日野下花帆 (ab#1)", "PL!HS-bp6-001-P | 日野下花帆 (ab#1)", "PL!HS-bp6-001-P＋ | 日野下花帆 (ab#1)", "PL!HS-bp6-001-SEC | 日野下花帆 (ab#1)"], "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "optional": true, "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "optional": true, "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、カードを1枚デッキの一番上に置いてもよい (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp6-005-R＋ | 徒町 小鈴 (ab#0)", "PL!HS-bp6-005-P | 徒町 小鈴 (ab#0)", "PL!HS-bp6-005-P＋ | 徒町 小鈴 (ab#0)", "PL!HS-bp6-005-SEC | 徒町 小鈴 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "modify_cost", "card_type": "member_card", "duration": "live_end", "group_names": ["蓮ノ空"], "operation": "add", "value": 6}, {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["蓮ノ空"], "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}, "duration": "live_end", "group_names": ["蓮ノ空"]}], "duration": "live_end", "group_names": ["蓮ノ空"]}], "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart05"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_cost", "card_type": "member_card", "duration": "live_end", "group_names": ["蓮ノ空"], "operation": "add", "value": 6}, {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["蓮ノ空"], "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}, "duration": "live_end", "group_names": ["蓮ノ空"]}], "duration": "live_end", "group_names": ["蓮ノ空"]}], "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart05"]}
```

- このメンバーのコストを+6する。その後、自分のステージにいる『蓮ノ空』のメンバーのコストの合計が、相手のステージにいるメンバーのコストの合計より高い場合、さらにライブ終了時まで、{heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "modify_cost", "card_type": "member_card", "duration": "live_end", "group_names": ["蓮ノ空"], "operation": "add", "value": 6}
```

- このメンバーのコストを+6する (x1)

```json
{"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["蓮ノ空"], "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}, "duration": "live_end", "group_names": ["蓮ノ空"]}], "duration": "live_end", "group_names": ["蓮ノ空"]}
```

- その後、自分のステージにいる『蓮ノ空』のメンバーのコストの合計が、相手のステージにいるメンバーのコストの合計より高い場合、ライブ終了時まで、{heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["蓮ノ空"], "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}, "duration": "live_end", "group_names": ["蓮ノ空"]}
```

- 自分のステージにいる『蓮ノ空』のメンバーのコストの合計が、相手のステージにいるメンバーのコストの合計より高い場合、ライブ終了時まで、{heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["蓮ノ空"], "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}
```

- 自分のステージにいる『蓮ノ空』のメンバーのコストの合計が、相手のステージにいるメンバーのコストの合計より高い場合 (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}
```


```json
{"card_count": 4, "cards": ["PL!HS-bp6-005-R＋ | 徒町 小鈴 (ab#1)", "PL!HS-bp6-005-P | 徒町 小鈴 (ab#1)", "PL!HS-bp6-005-P＋ | 徒町 小鈴 (ab#1)", "PL!HS-bp6-005-SEC | 徒町 小鈴 (ab#1)"], "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["DOLLCHESTRA"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["DOLLCHESTRA"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、『DOLLCHESTRA』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp6-006-R＋ | 安養寺 姫芽 (ab#0)", "PL!HS-bp6-006-P | 安養寺 姫芽 (ab#0)", "PL!HS-bp6-006-P＋ | 安養寺 姫芽 (ab#0)", "PL!HS-bp6-006-SEC | 安養寺 姫芽 (ab#0)"], "effect": {"action": "modify_cost", "group_names": ["みらくらぱーく！"], "location": "hand", "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_location": "stage", "per_unit_type": "人", "value": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "group_names": ["みらくらぱーく！"], "location": "hand", "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_location": "stage", "per_unit_type": "人", "value": 2}
```

- 手札にあるこのメンバーカードのコストは、自分のステージにいる『みらくらぱーく！』のメンバー1人につき、2少なくなる (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp6-006-R＋ | 安養寺 姫芽 (ab#1)", "PL!HS-bp6-006-P | 安養寺 姫芽 (ab#1)", "PL!HS-bp6-006-P＋ | 安養寺 姫芽 (ab#1)", "PL!HS-bp6-006-SEC | 安養寺 姫芽 (ab#1)"], "effect": {"action": "restriction", "card_type": "member_card", "count": 1, "exclude_group_names": ["みらくらぱーく！"], "group_names": null, "restriction_type": "cannot_baton_touch"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "restriction", "card_type": "member_card", "count": 1, "exclude_group_names": ["みらくらぱーく！"], "group_names": null, "restriction_type": "cannot_baton_touch"}
```

- このメンバーは『みらくらぱーく！』以外のメンバーカードとのバトンタッチで控え室に置けない (x1)

```json
{"card_count": 4, "cards": ["PL!HS-bp6-006-R＋ | 安養寺 姫芽 (ab#2)", "PL!HS-bp6-006-P | 安養寺 姫芽 (ab#2)", "PL!HS-bp6-006-P＋ | 安養寺 姫芽 (ab#2)", "PL!HS-bp6-006-SEC | 安養寺 姫芽 (ab#2)"], "effect": {"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "wait"}, {"action": "restriction", "count": 1, "delayed": true, "restriction_type": "cannot_active"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "wait"}, {"action": "restriction", "count": 1, "delayed": true, "restriction_type": "cannot_active"}]}
```

- このメンバーをウェイトにし、次のターンのアクティブフェイズにアクティブしない (x1)

```json
{"action": "restriction", "count": 1, "delayed": true, "restriction_type": "cannot_active"}
```

- 次のターンのアクティブフェイズにアクティブしない (x1)

```json
{"card_count": 3, "cards": ["PL!-PR-005-PR | 星空 凛 (ab#0)", "PL!-PR-006-PR | 西木野真姫 (ab#0)", "PL!-PR-008-PR | 小泉花陽 (ab#0)"], "effect": {"action": "choice", "all": true, "count": 1, "options": [{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, {"action": "change_state", "all": true, "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "state_change": "wait", "target": "opponent"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "choice", "all": true, "count": 1, "options": [{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, {"action": "change_state", "all": true, "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "state_change": "wait", "target": "opponent"}]}
```

- 以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする (x1)

```json
{"action": "change_state", "all": true, "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする (x1)

```json
{"card_count": 3, "cards": ["PL!-PR-012-PR | 小泉花陽 (ab#0)", "PL!S-PR-038-PR | 黒澤ダイヤ (ab#0)", "PL!SP-PR-017-PR | ウィーン・マルガレーテ (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 3, "cards": ["PL!S-PR-016-PR | 黒澤ダイヤ (ab#0)", "PL!S-PR-020-PR | 小原鞠莉 (ab#0)", "PL!S-PR-021-PR | 黒澤ルビィ (ab#0)"], "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 3, "cards": ["PL!S-PR-029-PR | 渡辺 曜 (ab#0)", "PL!S-PR-030-PR | 津島善子 (ab#0)", "PL!S-PR-031-PR | 国木田花丸 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"cost_limit": 13, "location": "stage", "operator": ">=", "target": "either", "type": "location_condition"}, "count": 2, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"cost_limit": 13, "location": "stage", "operator": ">=", "target": "either", "type": "location_condition"}, "count": 2, "resource": "blade"}
```

- 自分か相手のステージにコスト13以上のメンバーがいる場合、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"cost_limit": 13, "location": "stage", "operator": ">=", "target": "either", "type": "location_condition"}
```

- 自分か相手のステージにコスト13以上のメンバーがいる場合 (x1)

```json
{"card_count": 3, "cards": ["PL!N-PR-003-PR | 上原歩夢 (ab#0)", "PL!N-PR-008-PR | 近江彼方 (ab#0)", "PL!N-PR-010-PR | エマ・ヴェルデ (ab#0)"], "cost": {"source": "hand", "type": "reveal", "zone": "hand"}, "effect": {"action": "look_and_select", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, {"card_type": "live_card", "location": "revealed_cards", "negation": true, "type": "location_condition"}], "location": "hand", "operator": "and", "target": "self", "type": "compound"}, "exclude_self": true, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "exclude_self": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"source": "hand", "type": "reveal", "zone": "hand"}
```

- 手札をすべて公開する (x1)

```json
{"action": "look_and_select", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, {"card_type": "live_card", "location": "revealed_cards", "negation": true, "type": "location_condition"}], "location": "hand", "operator": "and", "target": "self", "type": "compound"}, "exclude_self": true, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "exclude_self": true, "optional": true, "reveal": true}}
```

- 自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, {"card_type": "live_card", "location": "revealed_cards", "negation": true, "type": "location_condition"}], "location": "hand", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合 (x1)

```json
{"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "exclude_self": true, "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!N-PR-021-PR | 鐘 嵐珠 (ab#0)", "PL!SP-PR-016-PR | 嵐 千砂都 (ab#0)", "PL!HS-PR-027-PR | 徒町小鈴 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "or_card_types": ["live_card", "member_card"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "or_card_types": ["live_card", "member_card"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、コスト2以下のメンバーカードか、スコア2以下のライブカードを1枚手札に加える (x1)

```json
{"card_count": 3, "cards": ["PL!SP-pb1-001-PR | 澁谷かのん (ab#0)", "PL!SP-pb1-001-R | 澁谷かのん (ab#0)", "PL!SP-pb1-001-P＋ | 澁谷かのん (ab#0)"], "effect": {"action": "conditional_on_optional", "conditional_action": {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand", "target": "self"}, "conditional_negation": true, "optional_action": {"action": "pay_energy", "count": 2, "energy": 2, "target": "self"}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_on_optional", "conditional_action": {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand", "target": "self"}, "conditional_negation": true, "optional_action": {"action": "pay_energy", "count": 2, "energy": 2, "target": "self"}}
```

- {icon_energy.png|E}{icon_energy.png|E}支払わないかぎり、自分の手札を2枚控え室に置く (x1)

```json
{"action": "pay_energy", "count": 2, "energy": 2, "target": "self"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand", "target": "self"}
```

- 自分の手札を2枚控え室に置く (x1)

```json
{"card_count": 3, "cards": ["PL!SP-pb1-001-PR | 澁谷かのん (ab#1)", "PL!SP-pb1-001-R | 澁谷かのん (ab#1)", "PL!SP-pb1-001-P＋ | 澁谷かのん (ab#1)"], "cost": {"count": 6, "energy": 6, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "modify_score", "operation": "add", "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "operation": "add", "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"card_count": 3, "cards": ["PL!SP-PR-003-PR | 澁谷かのん (ab#0)", "PL!SP-PR-007-PR | 葉月 恋 (ab#0)", "PL!SP-PR-010-PR | 若菜四季 (ab#0)"], "effect": {"action": "draw_card", "condition": {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のエネルギーが7枚以上ある場合、カードを1枚引く (x1)

```json
{"card_count": 3, "cards": ["PL!SP-PR-009-PR | 米女メイ (ab#0)", "PL!SP-PR-011-PR | 鬼塚夏美 (ab#0)", "PL!SP-PR-012-PR | ウィーン・マルガレーテ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, {"action": "draw_card", "condition": {"card_type": "live_card", "location": "discard", "type": "location_condition"}, "count": 1, "destination": "hand", "duration": "live_end", "source": "deck"}], "duration": "live_end"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, {"action": "draw_card", "condition": {"card_type": "live_card", "location": "discard", "type": "location_condition"}, "count": 1, "destination": "hand", "duration": "live_end", "source": "deck"}], "duration": "live_end"}
```

- {icon_blade.png|ブレード}を得る。これによりライブカードを控え室に置いた場合、さらにカードを1枚引く (x1)

```json
{"action": "draw_card", "condition": {"card_type": "live_card", "location": "discard", "type": "location_condition"}, "count": 1, "destination": "hand", "duration": "live_end", "source": "deck"}
```

- これによりライブカードを控え室に置いた場合、カードを1枚引く (x1)

```json
{"card_count": 3, "cards": ["PL!HS-PR-001-PR | 日野下花帆 (ab#1)", "PL!HS-PR-002-PR | 村野さやか (ab#1)", "PL!HS-PR-005-PR | 大沢瑠璃乃 (ab#1)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 3, "cards": ["PL!SP-bp1-012-N | 澁谷かのん (ab#0)", "PL!SP-sd1-008-SD | 若菜四季 (ab#0)", "PL!SP-sd1-017-SD | 桜小路きな子 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 3, "cards": ["PL!HS-bp1-011-N | 村野さやか (ab#0)", "PL!-bp3-010-N | 高坂穂乃果 (ab#0)", "PL!HS-bp6-022-N | 安養寺 姫芽 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 3, "cards": ["PL!SP-sd1-003-SD | 嵐 千砂都 (ab#0)", "PL!SP-sd1-003-P | 嵐 千砂都 (ab#0)", "PL!SP-sd1-003-SD2 | 嵐 千砂都 (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 5, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "count": 5, "duration": "live_end", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!SP-sd1-011-SD | 鬼塚冬毬 (ab#0)", "PL!SP-sd1-011-P | 鬼塚冬毬 (ab#0)", "PL!SP-sd1-011-SD2 | 鬼塚冬毬 (ab#0)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 3, "cards": ["PL!SP-bp2-013-N | 唐 可可 (ab#0)", "PL!SP-bp2-014-N | 嵐 千砂都 (ab#0)", "PL!SP-bp2-018-N | 米女メイ (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "max": true, "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室からカードを1枚までデッキの一番上に置く (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp2-020-L | Link to the FUTURE (ab#0)", "PL!HS-bp5-018-L | AURORA FLOWER (ab#0)", "PL!HS-sd1-020-SD | Link to the FUTURE（104期Ver.） (ab#0)"], "effect": {"action": "set_card_identity", "all": true, "all_regions": true, "group_names": ["スリーズブーケ", "DOLLCHESTRA", "みらくらぱーく！"], "identities": ["スリーズブーケ", "DOLLCHESTRA", "みらくらぱーく！"], "self_target": true}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "set_card_identity", "all": true, "all_regions": true, "group_names": ["スリーズブーケ", "DOLLCHESTRA", "みらくらぱーく！"], "identities": ["スリーズブーケ", "DOLLCHESTRA", "みらくらぱーく！"], "self_target": true}
```

- すべての領域にあるこのカードは『スリーズブーケ』、『DOLLCHESTRA』、『みらくらぱーく！』として扱う (x1)

```json
{"card_count": 3, "cards": ["PL!-bp4-002-R＋ | 絢瀬絵里 (ab#1)", "PL!-bp4-002-P | 絢瀬絵里 (ab#1)", "PL!-bp4-002-P＋ | 絢瀬絵里 (ab#1)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "activation_condition_parsed": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 3, "cards": ["PL!-bp5-002-R | 絢瀬絵里 (ab#0)", "PL!-bp5-002-P | 絢瀬絵里 (ab#0)", "PL!-bp5-002-AR | 絢瀬絵里 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からコスト9以上の『μ's』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!-bp5-005-R | 星空凛 (ab#0)", "PL!-bp5-005-P | 星空凛 (ab#0)", "PL!-bp5-005-AR | 星空凛 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "target": "self"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合、自分のエネルギーデッキから、エネルギーカードを1枚アクティブ状態で置く (x1)

```json
{"card_count": 3, "cards": ["PL!-bp5-006-R | 西木野真姫 (ab#0)", "PL!-bp5-006-P | 西木野真姫 (ab#0)", "PL!-bp5-006-AR | 西木野真姫 (ab#0)"], "effect": {"action": "draw_card", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "draw_card", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のライブカード置き場にカードが2枚以上ある場合、カードを1枚引く (x1)

```json
{"card_count": 3, "cards": ["PL!-bp5-007-R | 東條 希 (ab#0)", "PL!-bp5-007-P | 東條 希 (ab#0)", "PL!-bp5-007-AR | 東條 希 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "discard_until_count", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 3, "destination": "discard", "multiple_targets": true, "source": "hand", "target": "both", "target_count": 3}, {"action": "draw_card", "count": 3, "destination": "hand", "multiple_targets": true, "source": "deck", "target": "both"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "discard_until_count", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 3, "destination": "discard", "multiple_targets": true, "source": "hand", "target": "both", "target_count": 3}, {"action": "draw_card", "count": 3, "destination": "hand", "multiple_targets": true, "source": "deck", "target": "both"}]}
```

- このメンバーよりコストが低いメンバーからバトンタッチして登場した場合、自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き、その後、自分と相手はそれぞれカードを3枚引く (x1)

```json
{"action": "discard_until_count", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 3, "destination": "discard", "multiple_targets": true, "source": "hand", "target": "both", "target_count": 3}
```

- このメンバーよりコストが低いメンバーからバトンタッチして登場した場合、自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き、 (x1)

```json
{"action": "draw_card", "count": 3, "destination": "hand", "multiple_targets": true, "source": "deck", "target": "both"}
```

- 自分と相手はそれぞれカードを3枚引く (x1)

```json
{"card_count": 3, "cards": ["PL!-bp5-008-R | 小泉花陽 (ab#0)", "PL!-bp5-008-P | 小泉花陽 (ab#0)", "PL!-bp5-008-AR | 小泉花陽 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}
```

- {heart_03.png|heart03}{heart_03.png|heart03}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!-bp5-009-R | 矢澤にこ (ab#0)", "PL!-bp5-009-P | 矢澤にこ (ab#0)", "PL!-bp5-009-AR | 矢澤にこ (ab#0)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart06"], "need_heart_color": "heart06", "need_heart_operator": ">=", "need_heart_total": 3, "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart06"], "need_heart_color": "heart06", "need_heart_operator": ">=", "need_heart_total": 3, "source": "discard", "target": "self"}
```

- 自分の控え室から必要ハートに{heart_06.png|heart06}を3以上含むライブカードを1枚手札に加える (x1)

```json
{"card_count": 3, "cards": ["PL!S-bp5-003-R | 松浦果南 (ab#0)", "PL!S-bp5-003-P | 松浦果南 (ab#0)", "PL!S-bp5-003-AR | 松浦果南 (ab#0)"], "cost": {"card_type": "member_card", "count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "destination": "hand", "dynamic_count": {"mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}, "group_names": ["Aqours"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_type": "member_card", "count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札のブレードハートを持たないメンバーカードを2枚まで控え室に置いてもよい (x1)

```json
{"action": "move_cards", "card_type": "live_card", "destination": "hand", "dynamic_count": {"mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}, "group_names": ["Aqours"], "source": "discard", "target": "self"}
```

- 自分の控え室から、これにより控え室に置いたカードと同じ枚数の『Aqours』のライブカードを手札に加える (x1)

```json
{"mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}
```


```json
{"card_count": 3, "cards": ["PL!S-bp5-004-R | 黒澤ダイヤ (ab#0)", "PL!S-bp5-004-P | 黒澤ダイヤ (ab#0)", "PL!S-bp5-004-AR | 黒澤ダイヤ (ab#0)"], "effect": {"action": "choice", "count": 1, "exclude_self": true, "group_names": ["Aqours", "SaintSnow"], "options": [{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["Aqours"], "resource": "blade", "target": "self", "target_count": 1}, {"action": "position_change", "card_type": "member_card", "count": 1, "group_names": ["SaintSnow"], "target": "self", "target_member": "select"}], "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "choice", "count": 1, "exclude_self": true, "group_names": ["Aqours", "SaintSnow"], "options": [{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["Aqours"], "resource": "blade", "target": "self", "target_count": 1}, {"action": "position_change", "card_type": "member_card", "count": 1, "group_names": ["SaintSnow"], "target": "self", "target_member": "select"}], "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}
```

- 以下から1つを選ぶ。
・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{icon_blade.png|ブレード}を得る。
・自分のステージにいる『SaintSnow』のメンバー1人をポジションチェンジさせる (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["Aqours"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{icon_blade.png|ブレード}を得る。 (x1)

```json
{"action": "position_change", "card_type": "member_card", "count": 1, "group_names": ["SaintSnow"], "target": "self", "target_member": "select"}
```

- 自分のステージにいる『SaintSnow』のメンバー1人をポジションチェンジさせる (x1)

```json
{"card_count": 3, "cards": ["PL!S-bp5-006-R | 津島善子 (ab#0)", "PL!S-bp5-006-P | 津島善子 (ab#0)", "PL!S-bp5-006-AR | 津島善子 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "group_names": ["Aqours"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Aqours"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からコスト9以上の『Aqours』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!S-bp5-007-R | 国木田花丸 (ab#0)", "PL!S-bp5-007-P | 国木田花丸 (ab#0)", "PL!S-bp5-007-AR | 国木田花丸 (ab#0)"], "effect": {"action": "look_and_select", "heart_colors": ["heart04"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart04"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart04"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart04"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からハートに{heart_04.png|heart04}を2つ以上持つメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart04"], "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!S-bp5-008-R | 小原鞠莉 (ab#0)", "PL!S-bp5-008-P | 小原鞠莉 (ab#0)", "PL!S-bp5-008-AR | 小原鞠莉 (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "target": "self", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "target": "self", "value": 1}
```

- 自分のライブの合計スコアを+1する (x1)

```json
{"count": 2, "operator": ">=", "resource_type": "surplus_heart", "target": "opponent", "type": "comparison_condition"}
```

- 相手の余剰ハートが2つ以上あるかぎり (x1)

```json
{"card_count": 3, "cards": ["PL!S-bp5-009-R | 黒澤ルビィ (ab#0)", "PL!S-bp5-009-P | 黒澤ルビィ (ab#0)", "PL!S-bp5-009-AR | 黒澤ルビィ (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["SaintSnow"], "source": "discard", "target": "self"}, {"action": "gain_resource", "count": 2, "duration": "live_end", "resource": "blade"}], "conditional": true, "group_names": ["SaintSnow"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["SaintSnow"], "source": "discard", "target": "self"}, {"action": "gain_resource", "count": 2, "duration": "live_end", "resource": "blade"}], "conditional": true, "group_names": ["SaintSnow"]}
```

- 自分の控え室から『SaintSnow』のカードを1枚手札に加える。そうした場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["SaintSnow"], "source": "discard", "target": "self"}
```

- 自分の控え室から『SaintSnow』のカードを1枚手札に加える。 (x1)

```json
{"card_count": 3, "cards": ["PL!S-bp5-014-N | 渡辺 曜 (ab#0)", "PL!S-sd1-017-SD | 小原鞠莉 (ab#0)", "PL!S-sd1-018-SD | 黒澤ルビィ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_bottom", "source": "hand"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_bottom", "source": "hand"}]}
```

- カードを1枚引き、手札を1枚デッキの一番下に置く (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_bottom", "source": "hand"}
```

- 手札を1枚デッキの一番下に置く (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-002-R | 中須かすみ (ab#0)", "PL!N-bp5-002-P | 中須かすみ (ab#0)", "PL!N-bp5-002-AR | 中須かすみ (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "location": "stage", "scope": "both", "type": "location_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "location": "stage", "scope": "both", "type": "location_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"card_type": "member_card", "location": "stage", "scope": "both", "type": "location_condition"}
```

- 自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-003-R | 桜坂しずく (ab#0)", "PL!N-bp5-003-P | 桜坂しずく (ab#0)", "PL!N-bp5-003-AR | 桜坂しずく (ab#0)"], "cost": {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "live_card", "count": 1, "optional": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "selected_cards"}], "conditional": true}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "live_card", "count": 1, "optional": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "selected_cards"}], "conditional": true}
```

- 自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{icon_energy.png|E}を支払ってもよい。そうした場合、そのライブカードを手札に加える (x1)

```json
{"action": "select", "card_type": "live_card", "count": 1, "optional": true, "source": "discard", "target": "self"}
```

- 自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{icon_energy.png|E}を支払ってもよい。 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "selected_cards"}
```

- そのライブカードを手札に加える (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-004-R | 朝香果林 (ab#0)", "PL!N-bp5-004-P | 朝香果林 (ab#0)", "PL!N-bp5-004-AR | 朝香果林 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "blade_limit": 4, "blade_limit_operator": "==", "card_type": "member_card", "count": 1, "original_value": true, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"action": "change_state", "blade_limit": 4, "blade_limit_operator": "==", "card_type": "member_card", "count": 1, "original_value": true, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいる元々持つ{icon_blade.png|ブレード}の数がちょうど4つのメンバー1人をウェイトにする (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-006-R | 近江彼方 (ab#0)", "PL!N-bp5-006-P | 近江彼方 (ab#0)", "PL!N-bp5-006-AR | 近江彼方 (ab#0)"], "effect": {"action": "restriction", "card_type": "member_card", "count": 1, "restriction_type": "cannot_activate", "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "restriction", "card_type": "member_card", "count": 1, "restriction_type": "cannot_activate", "target": "self"}
```

- このメンバーは自分のアクティブフェイズにアクティブにしない (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-006-R | 近江彼方 (ab#1)", "PL!N-bp5-006-P | 近江彼方 (ab#1)", "PL!N-bp5-006-AR | 近江彼方 (ab#1)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "exclude_self": true, "state_change": "wait"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "exclude_self": true, "state_change": "wait"}
```

- 自分のステージにこのメンバー以外のメンバーがいる場合、このメンバーをウェイトにする (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-008-R | エマ・ヴェルデ (ab#0)", "PL!N-bp5-008-P | エマ・ヴェルデ (ab#0)", "PL!N-bp5-008-AR | エマ・ヴェルデ (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "destination": "under_member", "type": "custom"}, "effect": {"action": "change_state", "card_type": "energy_card", "count": 2, "state_change": "active"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 3, "cards": ["PL!N-bp5-009-R | 天王寺璃奈 (ab#0)", "PL!N-bp5-009-P | 天王寺璃奈 (ab#0)", "PL!N-bp5-009-AR | 天王寺璃奈 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "group_names": ["虹ヶ咲"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["虹ヶ咲"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からコスト9以上の『虹ヶ咲』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!N-bp5-010-R | 三船栞子 (ab#0)", "PL!N-bp5-010-P | 三船栞子 (ab#0)", "PL!N-bp5-010-AR | 三船栞子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "condition": {"negation": true, "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "add", "value": 1}, {"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "remove", "value": 1}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "condition": {"negation": true, "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "add", "value": 1}, {"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "remove", "value": 1}]}
```

- 自分が余剰ハートを持たない場合、ライブの合計スコアを+1する。自分が余剰ハートを2つ以上持つ場合、ライブの合計スコアを-1する。この効果ではライブの合計スコアは0未満にはならない (x1)

```json
{"action": "modify_score", "condition": {"negation": true, "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "add", "value": 1}
```

- 自分が余剰ハートを持たない場合、ライブの合計スコアを+1する (x1)

```json
{"negation": true, "resource_type": "surplus_heart", "type": "comparison_condition"}
```

- 自分が余剰ハートを持たない場合 (x1)

```json
{"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "remove", "value": 1}
```

- 自分が余剰ハートを2つ以上持つ場合、ライブの合計スコアを-1する (x1)

```json
{"card_count": 3, "cards": ["PL!N-bp5-011-R | ミア・テイラー (ab#0)", "PL!N-bp5-011-P | ミア・テイラー (ab#0)", "PL!N-bp5-011-AR | ミア・テイラー (ab#0)"], "effect": {"action": "choice", "count": 1, "group_reference": "different_group_names", "options": [{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "destination": "hand", "group_reference": "different_group_names", "source": "discard", "target": "self"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "choice", "count": 1, "group_reference": "different_group_names", "options": [{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "destination": "hand", "group_reference": "different_group_names", "source": "discard", "target": "self"}]}
```

- 以下から1つを選ぶ。
・自分の控え室にカード名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。
・自分の控え室にグループ名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを2枚手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の控え室にカード名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "destination": "hand", "group_reference": "different_group_names", "source": "discard", "target": "self"}
```

- 自分の控え室にグループ名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを2枚手札に加える (x1)

```json
{"card_count": 3, "cards": ["PL!SP-bp5-006-R | 桜小路きな子 (ab#0)", "PL!SP-bp5-006-P | 桜小路きな子 (ab#0)", "PL!SP-bp5-006-AR | 桜小路きな子 (ab#0)"], "cost": {"count": 3, "destination": "discard", "source": "deck_top", "type": "move_cards", "zone": "deck_top"}, "effect": {"action": "position_change", "card_type": "member_card", "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "position_change", "card_type": "member_card", "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}
```

- このメンバーはポジションチェンジする (x1)

```json
{"card_count": 3, "cards": ["PL!SP-bp5-007-R | 米女メイ (ab#0)", "PL!SP-bp5-007-P | 米女メイ (ab#0)", "PL!SP-bp5-007-AR | 米女メイ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 3, "destination": "hand", "discard_remaining": true, "max": true, "optional": true, "per_group": true, "per_group_count": 1, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 3, "destination": "hand", "discard_remaining": true, "max": true, "optional": true, "per_group": true, "per_group_count": 1, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 3, "destination": "hand", "discard_remaining": true, "max": true, "optional": true, "per_group": true, "per_group_count": 1, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!SP-bp5-008-R | 若菜四季 (ab#0)", "PL!SP-bp5-008-P | 若菜四季 (ab#0)", "PL!SP-bp5-008-AR | 若菜四季 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からコスト9以上の『Liella!』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!SP-bp5-009-R | 鬼塚夏美 (ab#0)", "PL!SP-bp5-009-P | 鬼塚夏美 (ab#0)", "PL!SP-bp5-009-AR | 鬼塚夏美 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "deck_top", "target": "self"}, {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}], "conditional": true}, {"action": "change_state", "card_type": "member_card", "condition": {"card_type": "live_card", "location": "discard", "type": "location_condition"}, "count": 1, "state_change": "wait"}, {"action": "repeat_procedure", "max_repeats": 4, "optional": true}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "deck_top", "target": "self"}, {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}], "conditional": true}, {"action": "change_state", "card_type": "member_card", "condition": {"card_type": "live_card", "location": "discard", "type": "location_condition"}, "count": 1, "state_change": "wait"}, {"action": "repeat_procedure", "max_repeats": 4, "optional": true}]}
```

- 自分のデッキの一番上のカードを控え室に置いてもよい。そうした場合、ライブ終了時まで、{icon_blade.png|ブレード}を得る。これにより控え室に置いたカードがライブカードの場合、このメンバーをウェイトにする。自分はこの手順をさらに4回まで繰り返してもよい (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "deck_top", "target": "self"}
```

- 自分のデッキの一番上のカードを控え室に置いてもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}], "conditional": true}
```

- そうした場合、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "change_state", "card_type": "member_card", "condition": {"card_type": "live_card", "location": "discard", "type": "location_condition"}, "count": 1, "state_change": "wait"}
```

- これにより控え室に置いたカードがライブカードの場合、このメンバーをウェイトにする (x1)

```json
{"action": "repeat_procedure", "max_repeats": 4, "optional": true}
```

- 自分はこの手順を4回まで繰り返してもよい (x1)

```json
{"card_count": 3, "cards": ["PL!SP-bp5-010-R | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp5-010-P | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp5-010-AR | ウィーン・マルガレーテ (ab#0)"], "effect": {"action": "position_change", "activation_position": "center", "card_type": "member_card", "parenthetical": ["センターにいるメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはセンターエリアに移動させる。"], "source_position": "center", "target": "both"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "position_change", "activation_position": "center", "card_type": "member_card", "parenthetical": ["センターにいるメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはセンターエリアに移動させる。"], "source_position": "center", "target": "both"}
```

- 自分と相手は、自身のステージのセンターにいるメンバーをポジションチェンジする (x1)

```json
{"card_count": 3, "cards": ["PL!SP-bp5-011-R | 鬼塚冬毬 (ab#0)", "PL!SP-bp5-011-P | 鬼塚冬毬 (ab#0)", "PL!SP-bp5-011-AR | 鬼塚冬毬 (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "left_side", "count": 3, "heart_colors": ["heart02"], "position": "left_side", "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "activation_position": "left_side", "count": 3, "heart_colors": ["heart02"], "position": "left_side", "resource": "heart"}
```

- {heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!SP-bp5-011-R | 鬼塚冬毬 (ab#1)", "PL!SP-bp5-011-P | 鬼塚冬毬 (ab#1)", "PL!SP-bp5-011-AR | 鬼塚冬毬 (ab#1)"], "effect": {"action": "gain_resource", "activation_position": "center", "count": 3, "heart_colors": ["heart03"], "position": "center", "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "activation_position": "center", "count": 3, "heart_colors": ["heart03"], "position": "center", "resource": "heart"}
```

- {heart_03.png|heart03}{heart_03.png|heart03}{heart_03.png|heart03}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!SP-bp5-011-R | 鬼塚冬毬 (ab#2)", "PL!SP-bp5-011-P | 鬼塚冬毬 (ab#2)", "PL!SP-bp5-011-AR | 鬼塚冬毬 (ab#2)"], "effect": {"action": "gain_resource", "activation_position": "right_side", "count": 3, "heart_colors": ["heart05"], "position": "right_side", "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "activation_position": "right_side", "count": 3, "heart_colors": ["heart05"], "position": "right_side", "resource": "heart"}
```

- {heart_05.png|heart05}{heart_05.png|heart05}{heart_05.png|heart05}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp5-004-R | 百生 吟子 (ab#0)", "PL!HS-bp5-004-P | 百生 吟子 (ab#0)", "PL!HS-bp5-004-AR | 百生 吟子 (ab#0)"], "effect": {"action": "gain_resource", "cost_limit": 4, "cost_limit_operator": ">=", "count": 2, "group_names": ["スリーズブーケ"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "cost_limit": 4, "cost_limit_operator": ">=", "count": 2, "group_names": ["スリーズブーケ"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "blade", "target": "self"}
```

- 自分のステージにいるコスト4以上の『スリーズブーケ』以外のメンバー1人につき、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp5-005-R | 徒町 小鈴 (ab#0)", "PL!HS-bp5-005-P | 徒町 小鈴 (ab#0)", "PL!HS-bp5-005-AR | 徒町 小鈴 (ab#0)"], "cost": {"count": 1, "destination": "discard", "group_names": ["DOLLCHESTRA"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "conditional_on_result", "followup_action": {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}, "group_names": ["DOLLCHESTRA"], "heart_colors": ["heart05"], "original_value": true, "primary_effect": {"action": "select", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["DOLLCHESTRA"], "original_value": true, "target": "self"}, "result_condition": {"comparison_type": "cost", "cost_limit": 10, "cost_total": 10, "count": 10, "operator": ">=", "type": "comparison_condition"}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_on_result", "followup_action": {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}, "group_names": ["DOLLCHESTRA"], "heart_colors": ["heart05"], "original_value": true, "primary_effect": {"action": "select", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["DOLLCHESTRA"], "original_value": true, "target": "self"}, "result_condition": {"comparison_type": "cost", "cost_limit": 10, "cost_total": 10, "count": 10, "operator": ">=", "type": "comparison_condition"}}
```

- 自分のステージにいる『DOLLCHESTRA』のメンバー1人を選ぶ。ライブ終了時まで、このメンバーのコストは、選んだメンバーが元々持つコストより1低い値に等しくなる。これによりこのカードのコストが10以上になった場合、ライブ終了時まで、{heart_05.png|heart05}を得る (x1)

```json
{"action": "select", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["DOLLCHESTRA"], "original_value": true, "target": "self"}
```

- 自分のステージにいる『DOLLCHESTRA』のメンバー1人を選ぶ。ライブ終了時まで、このメンバーのコストは、選んだメンバーが元々持つコストより1低い値に等しくなる。 (x1)

```json
{"comparison_type": "cost", "cost_limit": 10, "cost_total": 10, "count": 10, "operator": ">=", "type": "comparison_condition"}
```

- これによりこのカードのコストが10以上になった場合 (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp5-006-R | 安養寺 姫芽 (ab#0)", "PL!HS-bp5-006-P | 安養寺 姫芽 (ab#0)", "PL!HS-bp5-006-AR | 安養寺 姫芽 (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}
```

- {heart_01.png|heart01}{heart_01.png|heart01}を得る (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp5-007-R | セラス 柳田 リリエンフェルト (ab#0)", "PL!HS-bp5-007-P | セラス 柳田 リリエンフェルト (ab#0)", "PL!HS-bp5-007-AR | セラス 柳田 リリエンフェルト (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["EdelNote"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["EdelNote"], "source": "discard", "target": "self"}
```

- 自分の控え室から『EdelNote』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp5-007-R | セラス 柳田 リリエンフェルト (ab#1)", "PL!HS-bp5-007-P | セラス 柳田 リリエンフェルト (ab#1)", "PL!HS-bp5-007-AR | セラス 柳田 リリエンフェルト (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["EdelNote"], "location": "stage", "target": "self", "type": "group_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["EdelNote"], "location": "stage", "target": "self", "type": "group_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "exclude_self": true, "group_names": ["EdelNote"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにこのメンバー以外の『EdelNote』のメンバーがいるかぎり (x1)

```json
{"card_count": 3, "cards": ["PL!HS-bp5-008-R | 桂城 泉 (ab#0)", "PL!HS-bp5-008-P | 桂城 泉 (ab#0)", "PL!HS-bp5-008-AR | 桂城 泉 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "group_names": ["蓮ノ空"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["蓮ノ空"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["蓮ノ空"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["蓮ノ空"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からコスト9以上の『蓮ノ空』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["蓮ノ空"], "optional": true, "reveal": true}
```


```json
{"card_count": 3, "cards": ["PL!S-sd1-013-SD | 黒澤ダイヤ (ab#0)", "PL!S-bp6-012-N | 松浦果南 (ab#0)", "PL!S-bp6-017-N | 小原鞠莉 (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 3, "cards": ["PL!SP-sd2-002-P | 唐 可可 (ab#1)", "PL!SP-sd2-002-SD2 | 唐 可可 (ab#1)", "PL!SP-sd2-013-SD2 | 唐 可可 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart06"], "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart06"], "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "heart"}
```

- このメンバーがエリアを移動したとき、ライブ終了時まで、{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-sd1-015-SD | 西木野 真姫 (ab#0)", "PL!HS-bp2-010-N | 日野下花帆 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!-PR-001-PR | 高坂穂乃果 (ab#0)", "PL!-PR-002-PR | 絢瀬絵里 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "count": 1, "optional": true, "state_change": "active"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "count": 1, "optional": true, "state_change": "active"}
```

- このメンバーがステージから控え室に置かれたとき、メンバー1人をアクティブにしてもよい (x1)

```json
{"card_count": 2, "cards": ["PL!-PR-015-PR | 西木野真姫 (ab#0)", "PL!SP-PR-020-PR | 桜小路きな子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "optional": true, "source": "hand", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "optional": true, "source": "hand", "target": "self"}
```

- このメンバーよりコストが低いメンバーからバトンタッチして登場した場合、自分の手札からコスト4以下のメンバーカードを1枚ステージに登場させてもよい (x1)

```json
{"card_count": 2, "cards": ["PL!-PR-018-PR | 東條 希 (ab#0)", "PL!HS-PR-032-PR | セラス 柳田 リリエンフェルト (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!S-PR-013-PR | 高海千歌 (ab#0)", "PL!S-PR-019-PR | 国木田花丸 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true}}
```

- 自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。{live_start.png|ライブ開始時}{icon_energy.png|E}{icon_energy.png|E}支払ってもよい：ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true}
```


```json
{"card_count": 2, "cards": ["PL!S-PR-037-PR | 松浦果南 (ab#0)", "PL!N-PR-020-PR | エマ・ヴェルデ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"card_type": "member_card", "count": 2, "heart_colors": ["heart05"], "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart05"]}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}], "condition": {"card_type": "member_card", "count": 2, "heart_colors": ["heart05"], "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart05"]}
```

- {heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "count": 2, "heart_colors": ["heart05"], "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}
```

- 自分のステージにいるメンバーがちょうど2人であるかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!S-PR-039-PR | 渡辺 曜 (ab#0)", "PL!N-PR-024-PR | 桜坂しずく (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "count": 4, "location": "success_live_card_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "count": 4, "location": "success_live_card_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"aggregate": "total", "count": 4, "location": "success_live_card_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}
```

- 自分と相手の成功ライブカード置き場にカードが合計4枚以上あるかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!S-PR-040-PR | 国木田花丸 (ab#0)", "PL!N-PR-023-PR | 上原歩夢 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 3, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "filter_targets_by_heart_colors": true, "group_reference": "same_group_name", "heart_colors": ["heart01", "heart04"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 3, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "filter_targets_by_heart_colors": true, "group_reference": "same_group_name", "heart_colors": ["heart01", "heart04"], "resource": "heart"}
```

- 自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、{heart_01.png|heart01}{heart_04.png|heart04}を得る (x1)

```json
{"card_type": "member_card", "comparison_target": "self", "count": 3, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-PR-028-PR | 宮下 愛 (ab#0)", "PL!HS-PR-031-PR | 日野下花帆 (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "draw_until_count", "count": 5, "destination": "hand", "source": "deck", "target": "self", "target_count": 5}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_until_count", "count": 5, "destination": "hand", "source": "deck", "target": "self", "target_count": 5}
```

- 自分の手札が5枚になるまでカードを引く (x1)

```json
{"card_count": 2, "cards": ["PL!N-sd1-005-PRproteinbar | 宮下 愛 (ab#0)", "PL!N-sd1-005-SD | 宮下 愛 (ab#0)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分の控え室から『虹ヶ咲』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-011-PR | 村野さやか (ab#0)", "PL!HS-bp2-011-N | 村野さやか (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top"}
```

- デッキの上からカードを5枚控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-PR-019-PR | 百生 吟子 (ab#0)", "PL!HS-PR-019-RM | 百生 吟子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart04"], "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart04"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart04"], "resource": "heart"}], "heart_colors": ["heart04"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart04"], "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart04"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart04"], "resource": "heart"}], "heart_colors": ["heart04"]}
```

- 自分のデッキの上からカードを3枚控え室に置く。それらがすべて{heart_04.png|heart04}を持つメンバーカードの場合、ライブ終了時まで、{heart_04.png|heart04}を得る (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart04"], "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを3枚控え室に置く (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart04"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart04"], "resource": "heart"}
```

- それらがすべて{heart_04.png|heart04}を持つメンバーカードの場合、ライブ終了時まで、{heart_04.png|heart04}を得る (x1)

```json
{"card_type": "member_card", "count": 3, "heart_colors": ["heart04"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}
```

- それらがすべて{heart_04.png|heart04}を持つメンバーカードの場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-PR-020-PR | 徒町 小鈴 (ab#0)", "PL!HS-PR-023-PR | 桂城 泉 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 2, "destination": "deck_top", "placement_order": "any_order", "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 2, "destination": "deck_top", "placement_order": "any_order", "source": "discard", "target": "self"}
```

- 自分の控え室にあるメンバーカード2枚を好きな順番でデッキの一番上に置く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-PR-021-PR | 安養寺 姫芽 (ab#0)", "PL!HS-PR-021-RM | 安養寺 姫芽 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart01"], "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart01"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart01"], "resource": "heart"}], "heart_colors": ["heart01"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart01"], "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart01"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart01"], "resource": "heart"}], "heart_colors": ["heart01"]}
```

- 自分のデッキの上からカードを3枚控え室に置く。それらがすべて{heart_01.png|heart01}を持つメンバーカードの場合、ライブ終了時まで、{heart_01.png|heart01}を得る (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart01"], "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを3枚控え室に置く (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart01"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart01"], "resource": "heart"}
```

- それらがすべて{heart_01.png|heart01}を持つメンバーカードの場合、ライブ終了時まで、{heart_01.png|heart01}を得る (x1)

```json
{"card_type": "member_card", "count": 3, "heart_colors": ["heart01"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}
```

- それらがすべて{heart_01.png|heart01}を持つメンバーカードの場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp1-001-R | 上原歩夢 (ab#0)", "PL!N-bp1-001-P | 上原歩夢 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp1-004-R | 朝香果林 (ab#0)", "PL!N-bp1-004-P | 朝香果林 (ab#0)"], "effect": {"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "exclude_self": true, "state_change": "active"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "exclude_self": true, "state_change": "active"}
```

- 自分のステージにほかの『虹ヶ咲』のメンバーがいる場合、エネルギーを1枚アクティブにする (x1)

```json
{"card_type": "member_card", "exclude_self": true, "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにほかの『虹ヶ咲』のメンバーがいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp1-005-R | 宮下 愛 (ab#0)", "PL!N-bp1-005-P | 宮下 愛 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp1-008-R | エマ・ヴェルデ (ab#0)", "PL!N-bp1-008-P | エマ・ヴェルデ (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札のメンバーカードを1枚控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp1-009-R | 天王寺璃奈 (ab#0)", "PL!N-bp1-009-P | 天王寺璃奈 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}]}
```

- 自分のデッキの上からカードを2枚控え室に置く。その後、自分の控え室からメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp1-011-R | ミア・テイラー (ab#0)", "PL!N-bp1-011-P | ミア・テイラー (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "reveal_until_live_card", "all": true, "source": "deck_top", "target": "self"}, {"action": "move_cards", "all": true, "card_type": "live_card", "count": 1, "destination": "hand", "exclude_self": true, "source": "looked_at"}, {"action": "move_cards", "all": true, "destination": "discard", "exclude_self": true, "source": "looked_at_remaining"}], "all": true, "exclude_self": true}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "reveal_until_live_card", "all": true, "source": "deck_top", "target": "self"}, {"action": "move_cards", "all": true, "card_type": "live_card", "count": 1, "destination": "hand", "exclude_self": true, "source": "looked_at"}, {"action": "move_cards", "all": true, "destination": "discard", "exclude_self": true, "source": "looked_at_remaining"}], "all": true, "exclude_self": true}
```

- ライブカードが公開されるまで、自分のデッキの一番上のカードを公開し続ける。そのライブカードを手札に加え、これにより公開されたほかのすべてのカードを控え室に置く (x1)

```json
{"action": "reveal_until_live_card", "all": true, "source": "deck_top", "target": "self"}
```


```json
{"action": "move_cards", "all": true, "card_type": "live_card", "count": 1, "destination": "hand", "exclude_self": true, "source": "looked_at"}
```

- そのライブカードを手札に加え (x1)

```json
{"action": "move_cards", "all": true, "destination": "discard", "exclude_self": true, "source": "looked_at_remaining"}
```

- これにより公開されたほかのすべてのカードを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp1-001-R | 澁谷かのん (ab#0)", "PL!SP-bp1-001-P | 澁谷かのん (ab#0)"], "effect": {"action": "restriction", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "exclude_self": true, "restriction_type": "cannot_live"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "restriction", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "exclude_self": true, "restriction_type": "cannot_live"}
```

- 自分のステージにほかのメンバーがいない場合、自分はライブできない (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp1-004-R | 平安名すみれ (ab#0)", "PL!SP-bp1-004-P | 平安名すみれ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"location": "stage", "position": "center", "type": "position_condition"}, "count": 5, "position": "center", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"location": "stage", "position": "center", "type": "position_condition"}, "count": 5, "position": "center", "resource": "blade"}
```

- ステージのセンターエリアにいる場合、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"location": "stage", "position": "center", "type": "position_condition"}
```

- ステージのセンターエリアにいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp1-005-R | 葉月 恋 (ab#0)", "PL!SP-bp1-005-P | 葉月 恋 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "max": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "max": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『Liella!』のカードを1枚まで公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "max": true, "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!SP-bp1-008-R | 若菜四季 (ab#0)", "PL!SP-bp1-008-P | 若菜四季 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "draw_card", "condition": {"characters": ["米女メイ"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "draw_card", "condition": {"characters": ["米女メイ"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}
```

- カードを1枚引く。自分のステージに「米女メイ」がいる場合、さらにカードを1枚引く (x1)

```json
{"action": "draw_card", "condition": {"characters": ["米女メイ"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のステージに「米女メイ」がいる場合、カードを1枚引く (x1)

```json
{"characters": ["米女メイ"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「米女メイ」がいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp1-009-R | 鬼塚夏美 (ab#0)", "PL!SP-bp1-009-P | 鬼塚夏美 (ab#0)"], "cost": {"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!SP-bp1-010-R | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp1-010-P | ウィーン・マルガレーテ (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『Liella!』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp1-002-R | 村野さやか (ab#0)", "PL!HS-bp1-002-P | 村野さやか (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 15, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 2, "cards": ["PL!HS-bp1-005-R | 大沢瑠璃乃 (ab#0)", "PL!HS-bp1-005-P | 大沢瑠璃乃 (ab#0)"], "cost": {"count": 3, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "draw_card", "count": 0, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"count": 3, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札を3枚まで控え室に置いてもよい (x1)

```json
{"action": "draw_card", "count": 0, "destination": "hand", "source": "deck"}
```

- これにより置いた枚数分カードを引く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp1-008-R | 徒町 小鈴 (ab#0)", "PL!HS-bp1-008-P | 徒町 小鈴 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "draw_card", "condition": {"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "draw_card", "condition": {"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}
```

- 自分のデッキの上からカードを3枚控え室に置く。それらがすべてメンバーカードの場合、カードを1枚引く (x1)

```json
{"action": "draw_card", "condition": {"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- それらがすべてメンバーカードの場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp1-009-R | 安養寺 姫芽 (ab#0)", "PL!HS-bp1-009-P | 安養寺 姫芽 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["みらくらぱーく！"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["みらくらぱーく！"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["みらくらぱーく！"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["みらくらぱーく！"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『みらくらぱーく！』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["みらくらぱーく！"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!N-sd1-009-SD | 天王寺璃奈 (ab#0)", "PL!N-bp5-014-N | 中須かすみ (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!SP-pb1-002-R | 唐 可可 (ab#0)", "PL!SP-pb1-002-P＋ | 唐 可可 (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 12, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "condition": {"count": 12, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "value": 1}
```

- 自分のエネルギーが12枚以上ある場合、ライブの合計スコアを+1する (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-003-R | 嵐 千砂都 (ab#0)", "PL!SP-pb1-003-P＋ | 嵐 千砂都 (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["5yncri5e!"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["5yncri5e!"], "multiple_targets": true, "position": "left_side", "position_compare": "right_side", "target": "both"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["5yncri5e!"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["5yncri5e!"], "multiple_targets": true, "position": "left_side", "position_compare": "right_side", "target": "both"}
```

- 自分のステージにいるメンバーが『5yncri5e!』のみの場合、自分と対戦相手は、センターエリアのメンバーを左サイドエリアに、左サイドエリアのメンバーを右サイドエリアに、右サイドエリアのメンバーをセンターエリアに、それぞれ移動させる (x1)

```json
{"all_members": true, "card_type": "member_card", "group_names": ["5yncri5e!"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーが『5yncri5e!』のみの場合 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-004-R | 平安名すみれ (ab#0)", "PL!SP-pb1-004-P＋ | 平安名すみれ (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!SP-pb1-004-R | 平安名すみれ (ab#1)", "PL!SP-pb1-004-P＋ | 平安名すみれ (ab#1)"], "cost": {"count": 3, "energy": 3, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"count": 3, "energy": 3, "optional": true, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}支払ってもよい (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-005-R | 葉月 恋 (ab#0)", "PL!SP-pb1-005-P＋ | 葉月 恋 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!SP-pb1-006-R | 桜小路きな子 (ab#0)", "PL!SP-pb1-006-P＋ | 桜小路きな子 (ab#0)"], "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "blade", "trigger_type": "each_time"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "blade", "trigger_type": "each_time"}
```

- このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-007-R | 米女メイ (ab#0)", "PL!SP-pb1-007-P＋ | 米女メイ (ab#0)"], "effect": {"action": "change_state", "card_type": "energy_card", "count": 2, "state_change": "active"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!SP-pb1-008-R | 若菜四季 (ab#0)", "PL!SP-pb1-008-P＋ | 若菜四季 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "sequential", "actions": [{"action": "select", "count": 1, "target": "self"}, {"action": "position_change", "card_type": "member_card"}, {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}, "destination": "same_area"}]}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "sequential", "actions": [{"action": "select", "count": 1, "target": "self"}, {"action": "position_change", "card_type": "member_card"}, {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}, "destination": "same_area"}]}]}
```

- カードを1枚引く。その後、登場したエリアとは別の自分のエリア1つを選ぶ。このメンバーをそのエリアに移動する。選んだエリアにメンバーがいる場合、そのメンバーは、このメンバーがいたエリアに移動させる (x1)

```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "target": "self"}, {"action": "position_change", "card_type": "member_card"}, {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}, "destination": "same_area"}]}
```

- 登場したエリアとは別の自分のエリア1つを選ぶ。このメンバーをそのエリアに移動する。選んだエリアにメンバーがいる場合、そのメンバーは、このメンバーがいたエリアに移動させる (x1)

```json
{"action": "select", "count": 1, "target": "self"}
```

- 登場したエリアとは別の自分のエリア1つを選ぶ (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-009-R | 鬼塚夏美 (ab#0)", "PL!SP-pb1-009-P＋ | 鬼塚夏美 (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["5yncri5e!"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "exclude_self": true, "group_names": ["5yncri5e!"], "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["5yncri5e!"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "exclude_self": true, "group_names": ["5yncri5e!"], "source": "deck"}
```

- 自分のステージにほかの『5yncri5e!』のメンバーがいる場合、カードを1枚引く (x1)

```json
{"card_type": "member_card", "exclude_self": true, "group_names": ["5yncri5e!"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにほかの『5yncri5e!』のメンバーがいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-010-R | ウィーン・マルガレーテ (ab#0)", "PL!SP-pb1-010-P＋ | ウィーン・マルガレーテ (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "condition": {"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "value": 4}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "condition": {"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "value": 4}
```

- 自分のエネルギーが10枚以上ある場合、ステージにいるこのメンバーのコストを+4する (x1)

```json
{"card_count": 2, "cards": ["PL!SP-pb1-011-R | 鬼塚冬毬 (ab#0)", "PL!SP-pb1-011-P＋ | 鬼塚冬毬 (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "destination": "discard", "exclude_characters": ["鬼塚冬毬"], "exclude_self": true, "group_names": ["Liella!"], "optional": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "same_area", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_type": "member_card", "count": 1, "destination": "discard", "exclude_characters": ["鬼塚冬毬"], "exclude_self": true, "group_names": ["Liella!"], "optional": true, "source": "stage", "type": "move_cards", "zone": "stage"}
```

- 「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "same_area", "source": "discard", "target": "self"}
```

- 自分の控え室から、これにより控え室に置いたメンバーカードを1枚、そのメンバーがいたエリアに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp2-001-R | 高海千歌 (ab#0)", "PL!S-bp2-001-P | 高海千歌 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "conditions": [{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "count": 3, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "conditions": [{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "count": 3, "resource": "blade"}
```

- 自分の成功ライブカード置き場のカードが0枚で、かつ相手の成功ライブカード置き場にカードが1枚以上ある場合、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "live_card", "conditions": [{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}
```

- 自分の成功ライブカード置き場のカードが0枚で、かつ相手の成功ライブカード置き場にカードが1枚以上ある場合 (x1)

```json
{"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}
```

- 相手の成功ライブカード置き場にカードが1枚以上ある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp2-002-R | 桜内梨子 (ab#0)", "PL!S-bp2-002-P | 桜内梨子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "conditional": true, "group_names": ["Aqours"]}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "conditional": true, "group_names": ["Aqours"]}
```

- このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『Aqours』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp2-003-R | 松浦果南 (ab#0)", "PL!S-bp2-003-P | 松浦果南 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart03"], "resource": "heart"}
```

- エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、{heart_03.png|緑ハート}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp2-004-R | 黒澤ダイヤ (ab#0)", "PL!S-bp2-004-P | 黒澤ダイヤ (ab#0)"], "effect": {"action": "conditional_on_result", "all": true, "followup_action": {"action": "re_yell", "lose_blade_hearts": true}, "primary_effect": {"action": "move_cards", "all": true, "card_type": "live_card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "optional": true, "source": "revealed_cards", "target": "self"}, "result_condition": {"count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "conditional_on_result", "all": true, "followup_action": {"action": "re_yell", "lose_blade_hearts": true}, "primary_effect": {"action": "move_cards", "all": true, "card_type": "live_card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "optional": true, "source": "revealed_cards", "target": "self"}, "result_condition": {"count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}}
```

- エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う (x1)

```json
{"action": "move_cards", "all": true, "card_type": "live_card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "optional": true, "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。 (x1)

```json
{"count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}
```

- これにより1枚以上のカードが控え室に置かれた場合 (x1)

```json
{"action": "re_yell", "lose_blade_hearts": true}
```

- そのエールで得たブレードハートを失い、もう一度エールを行う (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp2-006-R | 津島善子 (ab#0)", "PL!S-bp2-006-P | 津島善子 (ab#0)"], "cost": {"count": 4, "energy": 4, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_total": 4, "cost_total_operator": "<=", "count": 2, "destination": "stage", "max": true, "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"count": 4, "energy": 4, "optional": true, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}{icon_energy.png|E}支払ってもよい (x1)

```json
{"action": "move_cards", "card_type": "member_card", "cost_total": 4, "cost_total_operator": "<=", "count": 2, "destination": "stage", "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室から、コストの合計が4以下になるようにメンバーカードを2枚までステージに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp2-024-L | 君のこころは輝いてるかい？ (ab#0)", "PL!S-bp2-024-SECL | 君のこころは輝いてるかい？ (ab#0)"], "effect": {"action": "restriction", "card_type": "live_card", "count": 1, "destination": "success_live_zone", "restriction_type": "cannot_place", "self_target": true}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "restriction", "card_type": "live_card", "count": 1, "destination": "success_live_zone", "restriction_type": "cannot_place", "self_target": true}
```

- このカードは成功ライブカード置き場に置くことができない (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp2-002-R | 唐 可可 (ab#0)", "PL!SP-bp2-002-P | 唐 可可 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "cost_limit": 11, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "cost_limit": 11, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを3枚見る。その中からコスト11以上のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "cost_limit": 11, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!SP-bp2-003-R | 嵐 千砂都 (ab#0)", "PL!SP-bp2-003-P | 嵐 千砂都 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- このメンバーがエリアを移動したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp2-004-R | 平安名すみれ (ab#0)", "PL!SP-bp2-004-P | 平安名すみれ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "cost", "location": "stage", "operator": ">", "position": "center", "target": "self", "type": "comparison_condition"}, "count": 1, "filter_targets_by_heart_colors": true, "heart_colors": ["heart03"], "position": "center", "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "cost", "location": "stage", "operator": ">", "position": "center", "target": "self", "type": "comparison_condition"}, "count": 1, "filter_targets_by_heart_colors": true, "heart_colors": ["heart03"], "position": "center", "resource": "heart"}
```

- 自分のステージにいるメンバーのうち、センターエリアにいるメンバーが最も大きいコストを持つ場合、{heart_03.png|heart03}を得る (x1)

```json
{"card_type": "member_card", "comparison_type": "cost", "location": "stage", "operator": ">", "position": "center", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにいるメンバーのうち、センターエリアにいるメンバーが最も大きいコストを持つ場合 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp2-005-R | 葉月 恋 (ab#0)", "PL!SP-bp2-005-P | 葉月 恋 (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 7, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 7, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを7枚見る。その中から『Liella!』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp2-007-R | 米女メイ (ab#0)", "PL!SP-bp2-007-P | 米女メイ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『Liella!』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!SP-bp2-008-R | 若菜四季 (ab#0)", "PL!SP-bp2-008-P | 若菜四季 (ab#0)"], "cost": {"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "count": 1, "target": "self"}, {"action": "position_change", "card_type": "member_card"}, {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}, "destination": "same_area"}]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "count": 1, "target": "self"}, {"action": "position_change", "card_type": "member_card"}, {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 1, "operator": ">=", "type": "card_count_condition"}, "destination": "same_area"}]}
```

- このメンバーがいるエリアとは別の自分のエリア1つを選ぶ。このメンバーをそのエリアに移動する。選んだエリアにメンバーがいる場合、そのメンバーは、このメンバーがいたエリアに移動させる (x1)

```json
{"action": "select", "card_type": "member_card", "count": 1, "target": "self"}
```

- このメンバーがいるエリアとは別の自分のエリア1つを選ぶ (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp2-011-R | 鬼塚冬毬 (ab#0)", "PL!SP-bp2-011-P | 鬼塚冬毬 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "live_card", "count": 2, "distinct": "card_name", "source": "discard", "target": "self"}, {"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "select", "count": 1, "source": "selected_cards"}}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "selected_cards", "target": "self"}]}], "conditional": true}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "live_card", "count": 2, "distinct": "card_name", "source": "discard", "target": "self"}, {"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "select", "count": 1, "source": "selected_cards"}}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "selected_cards", "target": "self"}]}], "conditional": true}
```

- 自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。そうした場合、相手はそれらのカードのうち1枚を選ぶ。これにより相手に選ばれたカードを自分の手札に加える (x1)

```json
{"action": "select", "card_type": "live_card", "count": 2, "distinct": "card_name", "source": "discard", "target": "self"}
```

- 自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。 (x1)

```json
{"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "select", "count": 1, "source": "selected_cards"}}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "selected_cards", "target": "self"}]}
```

- 相手はそれらのカードのうち1枚を選ぶ。これにより相手に選ばれたカードを自分の手札に加える (x1)

```json
{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "select", "count": 1, "source": "selected_cards"}}
```

- 相手はそれらのカードのうち1枚を選ぶ。 (x1)

```json
{"action": "select", "count": 1, "source": "selected_cards"}
```

- それらのカードのうち1枚を選ぶ (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "selected_cards", "target": "self"}
```

- これにより相手に選ばれたカードを自分の手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp2-024-L | ビタミンSUMMER！ (ab#0)", "PL!SP-bp2-024-SECL | ビタミンSUMMER! (ab#0)"], "effect": {"action": "modify_score", "condition": {"comparison_target": "opponent", "location": "hand", "operator": ">", "target": "self", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"comparison_target": "opponent", "location": "hand", "operator": ">", "target": "self", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- 自分の手札の枚数が相手より多い場合、このカードのスコアを+1する (x1)

```json
{"comparison_target": "opponent", "location": "hand", "operator": ">", "target": "self", "type": "comparison_condition"}
```

- 自分の手札の枚数が相手より多い場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-001-R | 日野下花帆 (ab#0)", "PL!HS-bp2-001-P | 日野下花帆 (ab#0)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "live_card", "cost_limit": 3, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "cost_limit": 3, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室からスコア3以下の『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-003-R | 乙宗 梢 (ab#0)", "PL!HS-bp2-003-P | 乙宗 梢 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!HS-bp2-006-R | 藤島 慈 (ab#0)", "PL!HS-bp2-006-P | 藤島 慈 (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "multiple_targets": true, "optional": true, "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!HS-bp2-006-R | 藤島 慈 (ab#1)", "PL!HS-bp2-006-P | 藤島 慈 (ab#1)"], "effect": {"action": "gain_resource", "count": 1, "group_names": ["みらくらぱーく！"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "count": 1, "group_names": ["みらくらぱーく！"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "blade", "target": "self"}
```

- 自分のステージにいるほかの『みらくらぱーく！』のメンバー1人につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-008-R | 徒町 小鈴 (ab#0)", "PL!HS-bp2-008-P | 徒町 小鈴 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["DOLLCHESTRA"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "gain_resource", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["DOLLCHESTRA"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- このメンバーよりコストが低い『DOLLCHESTRA』のメンバーからバトンタッチして登場した場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["DOLLCHESTRA"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}
```

- このメンバーよりコストが低い『DOLLCHESTRA』のメンバーからバトンタッチして登場した場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-009-R | 安養寺 姫芽 (ab#0)", "PL!HS-bp2-009-P | 安養寺 姫芽 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["みらくらぱーく！"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "gain_resource", "condition": {"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["みらくらぱーく！"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}
```

- このメンバーよりコストが低い『みらくらぱーく！』のメンバーからバトンタッチして登場した場合、ライブ終了時まで、{heart_01.png|heart01}{heart_01.png|heart01}を得る (x1)

```json
{"baton_touch_trigger": true, "comparison_type": "cost", "group_names": ["みらくらぱーく！"], "location": "stage", "movement": "baton_touch", "operator": "<", "target": "self", "type": "movement_condition"}
```

- このメンバーよりコストが低い『みらくらぱーく！』のメンバーからバトンタッチして登場した場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-016-N | 百生 吟子 (ab#0)", "PL!HS-pb1-024-N | 桂城 泉 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!HS-bp2-022-L | アオクハルカ (ab#0)", "PL!HS-bp2-022-L＋ | アオクハルカ (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "live_card", "count": 3, "group_names": ["スリーズブーケ"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["スリーズブーケ"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "live_card", "count": 3, "group_names": ["スリーズブーケ"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["スリーズブーケ"], "operation": "add", "self_target": true, "value": 1}
```

- 自分の控え室に『スリーズブーケ』のライブカードが3枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"card_type": "live_card", "count": 3, "group_names": ["スリーズブーケ"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室に『スリーズブーケ』のライブカードが3枚以上ある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-024-L | レディバグ (ab#0)", "PL!HS-bp2-024-L＋ | レディバグ (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"conditions": [{"appearance": true, "characters": ["徒町小鈴"], "location": "stage", "target": "self", "type": "appearance_condition"}, {"appearance": true, "characters": ["村野さやか"], "cost_reference_character": "徒町小鈴", "cost_reference_operator": ">", "cost_reference_type": "cost", "location": "stage", "type": "appearance_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 3, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"conditions": [{"appearance": true, "characters": ["徒町小鈴"], "location": "stage", "target": "self", "type": "appearance_condition"}, {"appearance": true, "characters": ["村野さやか"], "cost_reference_character": "徒町小鈴", "cost_reference_operator": ">", "cost_reference_type": "cost", "location": "stage", "type": "appearance_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 3, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 自分のステージに「徒町小鈴」が登場しており、かつ「徒町小鈴」よりコストの大きい「村野さやか」が登場している場合、このカードを成功させるための必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"conditions": [{"appearance": true, "characters": ["徒町小鈴"], "location": "stage", "target": "self", "type": "appearance_condition"}, {"appearance": true, "characters": ["村野さやか"], "cost_reference_character": "徒町小鈴", "cost_reference_operator": ">", "cost_reference_type": "cost", "location": "stage", "type": "appearance_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージに「徒町小鈴」が登場しており、かつ「徒町小鈴」よりコストの大きい「村野さやか」が登場している場合 (x1)

```json
{"appearance": true, "characters": ["徒町小鈴"], "location": "stage", "target": "self", "type": "appearance_condition"}
```

- 自分のステージに「徒町小鈴」が登場しており、 (x1)

```json
{"appearance": true, "characters": ["村野さやか"], "cost_reference_character": "徒町小鈴", "cost_reference_operator": ">", "cost_reference_type": "cost", "location": "stage", "type": "appearance_condition"}
```

- 「徒町小鈴」よりコストの大きい「村野さやか」が登場している場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp2-026-L | みらくりえーしょん (ab#0)", "PL!HS-bp2-026-L＋ | みらくりえーしょん (ab#0)"], "effect": {"action": "modify_score", "condition": {"appearance": true, "characters": ["藤島慈"], "location": "stage", "position": "center", "position_compare": "left_side", "target": "self", "type": "appearance_condition"}, "operation": "add", "position": "center", "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"appearance": true, "characters": ["藤島慈"], "location": "stage", "position": "center", "position_compare": "left_side", "target": "self", "type": "appearance_condition"}, "operation": "add", "position": "center", "self_target": true, "value": 2}
```

- 自分のステージの右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに「安養寺姫芽」が、センターエリアに「藤島慈」がそれぞれ登場している場合、このカードのスコアを+2する (x1)

```json
{"appearance": true, "characters": ["藤島慈"], "location": "stage", "position": "center", "position_compare": "left_side", "target": "self", "type": "appearance_condition"}
```

- 自分のステージの右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに「安養寺姫芽」が、センターエリアに「藤島慈」がそれぞれ登場している場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-001-R | 高海千歌 (ab#0)", "PL!S-pb1-001-P＋ | 高海千歌 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"comparison_target": "self", "count": 2, "location": "hand", "operator": ">=", "target": "opponent", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"comparison_target": "self", "count": 2, "location": "hand", "operator": ">=", "target": "opponent", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 相手の手札の枚数が自分より2枚以上多い場合、自分の控え室からライブカードを1枚手札に加える (x1)

```json
{"comparison_target": "self", "count": 2, "location": "hand", "operator": ">=", "target": "opponent", "type": "card_count_condition"}
```

- 相手の手札の枚数が自分より2枚以上多い場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-002-R | 桜内梨子 (ab#0)", "PL!S-pb1-002-P＋ | 桜内梨子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}}, {"action": "conditional_on_optional", "conditional_action": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "duration": "live_end"}, "conditional_negation": true, "optional_action": {"action": "do_nothing"}}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}}, {"action": "conditional_on_optional", "conditional_action": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "duration": "live_end"}, "conditional_negation": true, "optional_action": {"action": "do_nothing"}}]}
```

- 相手は手札からライブカードを1枚控え室に置いてもよい。そうしなかった場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}}
```

- 相手は手札からライブカードを1枚控え室に置いてもよい。 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}
```

- 手札からライブカードを1枚控え室に置いてもよい (x1)

```json
{"action": "conditional_on_optional", "conditional_action": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "duration": "live_end"}, "conditional_negation": true, "optional_action": {"action": "do_nothing"}}
```

- そうしなかった場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-003-R | 松浦果南 (ab#0)", "PL!S-pb1-003-P＋ | 松浦果南 (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "set_heart_type", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "heart_type": "heart04", "original_value": true, "self_target": true}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "duration": "live_end", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "heart_colors": ["heart04"], "source": "revealed_cards", "target": "self"}], "duration": "live_end", "heart_colors": ["heart04"], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "set_heart_type", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "heart_type": "heart04", "original_value": true, "self_target": true}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "duration": "live_end", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "heart_colors": ["heart04"], "source": "revealed_cards", "target": "self"}], "duration": "live_end", "heart_colors": ["heart04"], "original_value": true}
```

- このメンバーが元々持つハートはすべて{heart_04.png|heart04}になる。{live_success.png|ライブ成功時}エールにより公開された自分のカードの中から、ライブカードを1枚手札に加える (x1)

```json
{"action": "set_heart_type", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "heart_type": "heart04", "original_value": true, "self_target": true}
```

- このメンバーが元々持つハートはすべて{heart_04.png|heart04}になる (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "duration": "live_end", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "heart_colors": ["heart04"], "source": "revealed_cards", "target": "self"}
```

- {live_success.png|ライブ成功時}エールにより公開された自分のカードの中から、ライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-005-R | 渡辺 曜 (ab#0)", "PL!S-pb1-005-P＋ | 渡辺 曜 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"comparison_target": "self", "operator": ">", "resource_type": "energy", "target": "opponent", "type": "comparison_condition"}, "count": 3, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"comparison_target": "self", "operator": ">", "resource_type": "energy", "target": "opponent", "type": "comparison_condition"}, "count": 3, "resource": "blade"}
```

- 相手のエネルギーが自分より多い場合、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-006-R | 津島善子 (ab#0)", "PL!S-pb1-006-P＋ | 津島善子 (ab#0)"], "cost": {"card_type": "live_card", "count": 1, "source": "hand", "type": "reveal", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}}, {"action": "conditional_on_optional", "conditional_action": {"action": "gain_resource", "count": 4, "duration": "live_end", "resource": "blade"}, "conditional_negation": true, "optional_action": {"action": "do_nothing"}}]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}}, {"action": "conditional_on_optional", "conditional_action": {"action": "gain_resource", "count": 4, "duration": "live_end", "resource": "blade"}, "conditional_negation": true, "optional_action": {"action": "do_nothing"}}]}
```

- 相手は手札を1枚控え室に置いてもよい。そうしなかった場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}}
```

- 相手は手札を1枚控え室に置いてもよい。 (x1)

```json
{"action": "conditional_on_optional", "conditional_action": {"action": "gain_resource", "count": 4, "duration": "live_end", "resource": "blade"}, "conditional_negation": true, "optional_action": {"action": "do_nothing"}}
```

- そうしなかった場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "count": 4, "duration": "live_end", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-007-R | 国木田花丸 (ab#0)", "PL!S-pb1-007-P＋ | 国木田花丸 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-008-R | 小原鞠莉 (ab#0)", "PL!S-pb1-008-P＋ | 小原鞠莉 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}
```

- 自分か相手を選ぶ。自分は、そのプレイヤーのデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-009-R | 黒澤ルビィ (ab#0)", "PL!S-pb1-009-P＋ | 黒澤ルビィ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "count": 3, "location": "success_live_card_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}, "count": 3, "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "count": 3, "location": "success_live_card_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}, "count": 3, "resource": "blade"}
```

- 自分と相手の成功ライブカード置き場にカードが合計3枚以上ある場合、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"aggregate": "total", "count": 3, "location": "success_live_card_zone", "operator": ">=", "scope": "both", "target": "both", "type": "card_count_condition"}
```

- 自分と相手の成功ライブカード置き場にカードが合計3枚以上ある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-022-L | 逃走迷走メビウスループ (ab#0)", "PL!S-pb1-022-L＋ | 逃走迷走メビウスループ (ab#0)"], "effect": {"action": "restriction", "card_type": "live_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "location": "live_card_zone", "operator": "=", "resource_type": "score", "scope": "both", "target": "both", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, "count": 1, "destination": "success_live_zone", "duration": "live_end", "restriction_type": "cannot_place", "target": "both"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "restriction", "card_type": "live_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "location": "live_card_zone", "operator": "=", "resource_type": "score", "scope": "both", "target": "both", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, "count": 1, "destination": "success_live_zone", "duration": "live_end", "restriction_type": "cannot_place", "target": "both"}
```

- このターン、ライブに勝利するプレイヤーを決定するとき、自分と相手のライブの合計スコアが同じ場合、ライブ終了時まで、自分と相手は成功ライブカード置き場にカードを置くことができない (x1)

```json
{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "location": "live_card_zone", "operator": "=", "resource_type": "score", "scope": "both", "target": "both", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}
```

- このターン、ライブに勝利するプレイヤーを決定するとき、自分と相手のライブの合計スコアが同じ場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-pb1-024-L | 僕らの走ってきた道は・・・ (ab#0)", "PL!-bp6-011-N | 絢瀬絵里 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp3-001-R | 高坂穂乃果 (ab#0)", "PL!-bp3-001-P | 高坂穂乃果 (ab#0)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"]}
```

- カードを1枚引き、手札を1枚控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!-bp3-001-R | 高坂穂乃果 (ab#1)", "PL!-bp3-001-P | 高坂穂乃果 (ab#1)"], "effect": {"action": "change_state", "card_type": "member_card", "count": 1, "max": true, "state_change": "active", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp3-002-R | 絢瀬絵里 (ab#1)", "PL!-bp3-002-P | 絢瀬絵里 (ab#1)"], "effect": {"action": "gain_resource", "count": 1, "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "blade", "state": "wait", "target": "opponent"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "count": 1, "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "blade", "state": "wait", "target": "opponent"}
```

- 相手のステージにいるウェイト状態のメンバー1人につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-bp3-003-R | 南ことり (ab#0)", "PL!-bp3-003-P | 南ことり (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "source": "discard", "target": "self"}
```

- 自分の控え室から『μ's』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!-bp3-005-R | 星空 凛 (ab#0)", "PL!-bp3-005-P | 星空 凛 (ab#0)"], "effect": {"action": "change_state", "all": true, "card_type": "member_card", "state_change": "active", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "all": true, "card_type": "member_card", "state_change": "active", "target": "self"}
```

- 自分のステージにいるすべてのメンバーをアクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!-bp3-006-R | 西木野真姫 (ab#0)", "PL!-bp3-006-P | 西木野真姫 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "blade", "target": "self"}
```

- 自分の成功ライブカード置き場にあるカード1枚につき、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-bp3-007-R | 東條 希 (ab#0)", "PL!-bp3-007-P | 東條 希 (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "reveal": false}}
```

- 自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、1枚をデッキの上に置き、1枚を控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!-bp3-009-R＋ | 矢澤にこ (ab#0)", "PL!-bp3-009-P | 矢澤にこ (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 13, "cost_total": 13, "count": 13, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!-bp3-009-R＋ | 矢澤にこ (ab#1)", "PL!-bp3-009-P | 矢澤にこ (ab#1)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart"}], "heart_colors": ["heart01", "heart03", "heart06"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!-bp3-009-P＋ | 矢澤にこ (ab#0)", "PL!-bp3-009-SEC | 矢澤にこ (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 13, "cost_total": 13, "count": 13, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!-bp3-009-P＋ | 矢澤にこ (ab#1)", "PL!-bp3-009-SEC | 矢澤にこ (ab#1)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart"}], "heart_colors": ["heart01", "heart03", "heart06"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-002-R | 桜内梨子 (ab#0)", "PL!S-bp3-002-P | 桜内梨子 (ab#0)"], "effect": {"action": "move_cards", "activation_condition_parsed": {"count": 1, "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "card", "count": 1, "destination": "hand", "optional": true, "self_target": true, "source": "revealed_card"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "activation_condition_parsed": {"count": 1, "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "card", "count": 1, "destination": "hand", "optional": true, "self_target": true, "source": "revealed_card"}
```

- ライブの合計スコアが相手より高い場合、このカードを手札に加えてもよい (x1)

```json
{"count": 1, "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- このカードが自分のエールによって公開されている場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp3-003-R＋ | 松浦果南 (ab#0)", "PL!S-bp3-003-P | 松浦果南 (ab#0)"], "cost": {"card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "draw_card", "count": 3, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-003-R＋ | 松浦果南 (ab#1)", "PL!S-bp3-003-P | 松浦果南 (ab#1)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_type": "discard", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-003-P＋ | 松浦果南 (ab#0)", "PL!S-bp3-003-SEC | 松浦果南 (ab#0)"], "cost": {"card_type": "live_card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "draw_card", "count": 3, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-003-P＋ | 松浦果南 (ab#1)", "PL!S-bp3-003-SEC | 松浦果南 (ab#1)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_type": "discard", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-004-R | 黒澤ダイヤ (ab#0)", "PL!S-bp3-004-P | 黒澤ダイヤ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp3-005-R | 渡辺 曜 (ab#0)", "PL!S-bp3-005-P | 渡辺 曜 (ab#0)"], "effect": {"action": "draw_card", "condition": {"comparison_target": "opponent", "location": "revealed_cards", "operator": "<", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"comparison_target": "opponent", "location": "revealed_cards", "operator": "<", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- エールにより公開された自分のカードの枚数が、相手がエールによって公開したカードの枚数より少ない場合、カードを1枚引く (x1)

```json
{"comparison_target": "opponent", "location": "revealed_cards", "operator": "<", "target": "self", "type": "comparison_condition"}
```

- エールにより公開された自分のカードの枚数が、相手がエールによって公開したカードの枚数より少ない場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp3-007-R | 国木田花丸 (ab#0)", "PL!S-bp3-007-P | 国木田花丸 (ab#0)"], "cost": {"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "source": "discard", "target": "self"}, {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "target": "self"}], "conditional": true, "target": "self"}], "conditional": true}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-008-R | 小原鞠莉 (ab#0)", "PL!S-bp3-008-P | 小原鞠莉 (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, {"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "live_card", "comparison_type": "score", "count": 6, "group_names": ["Aqours"], "operator": ">=", "type": "comparison_condition"}, "count": 4, "state_change": "active"}], "group_names": ["Aqours"]}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, {"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "live_card", "comparison_type": "score", "count": 6, "group_names": ["Aqours"], "operator": ">=", "type": "comparison_condition"}, "count": 4, "state_change": "active"}], "group_names": ["Aqours"]}
```

- 自分の控え室からライブカードを1枚手札に加える。それがスコア6以上の『Aqours』のライブカードの場合、エネルギーを4枚アクティブにする (x1)

```json
{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "live_card", "comparison_type": "score", "count": 6, "group_names": ["Aqours"], "operator": ">=", "type": "comparison_condition"}, "count": 4, "state_change": "active"}
```

- それがスコア6以上の『Aqours』のライブカードの場合、エネルギーを4枚アクティブにする (x1)

```json
{"card_type": "live_card", "comparison_type": "score", "count": 6, "group_names": ["Aqours"], "operator": ">=", "type": "comparison_condition"}
```

- それがスコア6以上の『Aqours』のライブカードの場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp3-009-R | 黒澤ルビィ (ab#0)", "PL!S-bp3-009-P | 黒澤ルビィ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["Aqours"], "look_action": {"action": "look_at", "count": 6, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Aqours"], "look_action": {"action": "look_at", "count": 6, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを6枚見る。その中から『Aqours』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!S-bp3-010-N | 高海千歌 (ab#0)", "PL!S-bp3-011-N | 桜内梨子 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "count": 1, "max": true, "state_change": "active", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-002-R | 中須かすみ (ab#0)", "PL!N-bp3-002-P | 中須かすみ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["虹ヶ咲"], "resource": "heart", "target": "self", "target_count": 1}], "exclude_self": true, "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "heart_selection": true, "resource": "heart"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["虹ヶ咲"], "resource": "heart", "target": "self", "target_count": 1}], "exclude_self": true, "group_names": ["虹ヶ咲"]}
```

- 好きなハートの色を1つ指定する。ライブ終了時まで、自分のステージにいるこのメンバー以外の『虹ヶ咲』のメンバー1人は、そのハートを1つ得る (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["虹ヶ咲"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいるこのメンバー以外の『虹ヶ咲』のメンバー1人は、そのハートを1つ得る (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp3-003-R | 桜坂しずく (ab#0)", "PL!N-bp3-003-P | 桜坂しずく (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, {"ability_text": "登場_ability", "action": "activate_ability", "count": 1, "group_names": ["虹ヶ咲"], "target": "そのカードの{{toujyou.png|登場}}能力", "target_trigger": "登場"}], "group_names": ["虹ヶ咲"], "parenthetical": ["{{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, {"ability_text": "登場_ability", "action": "activate_ability", "count": 1, "group_names": ["虹ヶ咲"], "target": "そのカードの{{toujyou.png|登場}}能力", "target_trigger": "登場"}], "group_names": ["虹ヶ咲"], "parenthetical": ["{{toujyou.png|登場}}能力がコストを持つ場合、支払って発動させる。"]}
```

- 自分の控え室にあるコスト4以下の『虹ヶ咲』のメンバーカードを1枚選ぶ。そのカードの{toujyou.png|登場}能力1つを発動させる (x1)

```json
{"action": "select", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分の控え室にあるコスト4以下の『虹ヶ咲』のメンバーカードを1枚選ぶ (x1)

```json
{"ability_text": "登場_ability", "action": "activate_ability", "count": 1, "group_names": ["虹ヶ咲"], "target": "そのカードの{{toujyou.png|登場}}能力", "target_trigger": "登場"}
```

- そのカードの{toujyou.png|登場}能力1つを発動させる (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp3-004-R | 朝香果林 (ab#0)", "PL!N-bp3-004-P | 朝香果林 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-005-R＋ | 宮下 愛 (ab#0)", "PL!N-bp3-005-P | 宮下 愛 (ab#0)"], "effect": {"action": "draw_until_count", "condition": {"card_type": "member_card", "count": 3, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "count": 5, "destination": "hand", "source": "deck", "target_count": 5}, "is_null": false, "triggers": "自動"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-005-R＋ | 宮下 愛 (ab#1)", "PL!N-bp3-005-P | 宮下 愛 (ab#1)"], "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "duration": "live_end"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-005-P＋ | 宮下 愛 (ab#0)", "PL!N-bp3-005-SEC | 宮下 愛 (ab#0)"], "effect": {"action": "draw_until_count", "condition": {"card_type": "member_card", "count": 3, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "count": 5, "destination": "hand", "source": "deck", "target_count": 5}, "is_null": false, "triggers": "自動"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-005-P＋ | 宮下 愛 (ab#1)", "PL!N-bp3-005-SEC | 宮下 愛 (ab#1)"], "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "target": "self", "temporal": "this_turn", "type": "temporal_condition"}, "duration": "live_end"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-006-R | 近江彼方 (ab#0)", "PL!N-bp3-006-P | 近江彼方 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "count": 1, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "count": 1, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait"}
```

- このメンバーをウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp3-007-R | 優木せつ菜 (ab#0)", "PL!N-bp3-007-P | 優木せつ菜 (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}], "type": "sequential_cost"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "characters": ["優木せつ菜"], "cost_limit": 13, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "quoted_text": {"quoted_type": "character"}, "source": "hand", "target": "self"}, {"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "target": "self"}], "parenthetical": ["メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。"]}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "characters": ["優木せつ菜"], "cost_limit": 13, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "quoted_text": {"quoted_type": "character"}, "source": "hand", "target": "self"}, {"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "target": "self"}], "parenthetical": ["メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。"]}
```

- 自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、このメンバーがいたエリアに登場させる。その後、自分のエネルギー置き場にあるエネルギー1枚をそのメンバーの下に置く (x1)

```json
{"action": "move_cards", "card_type": "member_card", "characters": ["優木せつ菜"], "cost_limit": 13, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "quoted_text": {"quoted_type": "character"}, "source": "hand", "target": "self"}
```

- 自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、このメンバーがいたエリアに登場させる (x1)

```json
{"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "target": "self"}
```

- 自分のエネルギー置き場にあるエネルギー1枚をそのメンバーの下に置く (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp3-010-R | 三船栞子 (ab#0)", "PL!N-bp3-010-P | 三船栞子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "count": 2, "destination": "deck_bottom", "max": true, "placement_order": "any_order", "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 2, "destination": "deck_bottom", "max": true, "placement_order": "any_order", "source": "discard", "target": "self"}
```

- 自分は、そのプレイヤーの控え室にあるメンバーカードを2枚まで、好きな順番でデッキの一番下に置く (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp3-011-R | ミア・テイラー (ab#0)", "PL!N-bp3-011-P | ミア・テイラー (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "characters": ["ミア・テイラー"], "count": 1, "exclude_self": true, "quoted_text": {"quoted_type": "character"}, "target": "opponent"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}, "count": 1, "multiple_targets": true, "original_value": true, "resource": "blade"}], "original_value": true}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "characters": ["ミア・テイラー"], "count": 1, "exclude_self": true, "quoted_text": {"quoted_type": "character"}, "target": "opponent"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}, "count": 1, "multiple_targets": true, "original_value": true, "resource": "blade"}], "original_value": true}
```

- 相手のステージにいる「ミア・テイラー」以外のメンバーを1人選ぶ。そのメンバーが持つハートと、このメンバーが持つハートの中に同じ色のハートがある場合、ライブ終了時まで、{icon_blade.png|ブレード}を得る。それぞれのメンバーのコストが同じ場合、元々の{icon_blade.png|ブレード}の数が同じ場合についても同じことを行う (x1)

```json
{"action": "select", "card_type": "member_card", "characters": ["ミア・テイラー"], "count": 1, "exclude_self": true, "quoted_text": {"quoted_type": "character"}, "target": "opponent"}
```

- 相手のステージにいる「ミア・テイラー」以外のメンバーを1人選ぶ (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}
```

- そのメンバーが持つハートと、このメンバーが持つハートの中に同じ色のハートがある場合、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "equality", "count": 1, "operator": "=", "type": "card_count_condition"}, "count": 1, "multiple_targets": true, "original_value": true, "resource": "blade"}
```

- それぞれのメンバーのコストが同じ場合、元々の{icon_blade.png|ブレード}の数が同じ場合についても同じことを行う (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp3-012-R | 鐘 嵐珠 (ab#0)", "PL!N-bp3-012-P | 鐘 嵐珠 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["虹ヶ咲"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["虹ヶ咲"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを4枚見る。その中から『虹ヶ咲』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!N-bp3-028-L | ツナガルコネクト (ab#0)", "PL!N-bp3-028-SECL | ツナガルコネクト (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "look_at", "count": 1, "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck_top", "target": "self"}, {"action": "look_and_select", "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "max": true, "reveal": false}, "target": "self"}], "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "target": "self"}, {"action": "reveal", "card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "self_target": true, "source": "deck_top", "target": "self"}], "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "look_at", "count": 1, "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck_top", "target": "self"}, {"action": "look_and_select", "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "max": true, "reveal": false}, "target": "self"}], "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "target": "self"}, {"action": "reveal", "card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "self_target": true, "source": "deck_top", "target": "self"}], "group_names": ["虹ヶ咲"]}
```

- 自分のステージにいる『虹ヶ咲』のメンバー1人につき、自分のデッキの上からカードを1枚見る。その中から1枚までをデッキの上に置き、残りを控え室に置く。その後、自分のデッキの一番上のカードを1枚公開する。これによりライブカードを公開した場合、このカードのスコアを+1する (x1)

```json
{"action": "sequential", "actions": [{"action": "look_at", "count": 1, "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck_top", "target": "self"}, {"action": "look_and_select", "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "max": true, "reveal": false}, "target": "self"}], "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "target": "self"}
```


```json
{"action": "look_at", "count": 1, "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを1枚見る (x1)

```json
{"action": "look_and_select", "group_names": ["虹ヶ咲"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "select_action": {"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "max": true, "reveal": false}, "target": "self"}
```

- その中から1枚までをデッキの上に置き、残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "deck_top", "discard_remaining": true, "max": true, "reveal": false}
```


```json
{"action": "reveal", "card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "self_target": true, "source": "deck_top", "target": "self"}
```

- 自分のデッキの一番上のカードを1枚公開する。これによりライブカードを公開した場合、このカードのスコアを+1する (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-001-R | 高坂穂乃果 (ab#0)", "PL!-pb1-001-P＋ | 高坂穂乃果 (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "position": "center", "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "position": "center", "type": "sequential_cost"}, "effect": {"action": "sequential", "actions": [{"action": "select", "activation_position": "center", "all": false, "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "exclude_self": true, "or_card_types": ["live_card", "member_card"]}, {"action": "reveal", "activation_position": "center", "all": false, "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "exclude_self": true, "multiple_targets": true, "source": "deck_top"}, {"action": "move_cards", "activation_position": "center", "all": false, "count": 1, "destination": "hand", "exclude_self": true, "source": "looked_at"}, {"action": "move_cards", "activation_position": "center", "all": true, "destination": "discard", "exclude_self": true, "source": "looked_at_remaining"}], "activation_position": "center", "all": true, "exclude_self": true}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "select", "activation_position": "center", "all": false, "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "exclude_self": true, "or_card_types": ["live_card", "member_card"]}, {"action": "reveal", "activation_position": "center", "all": false, "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "exclude_self": true, "multiple_targets": true, "source": "deck_top"}, {"action": "move_cards", "activation_position": "center", "all": false, "count": 1, "destination": "hand", "exclude_self": true, "source": "looked_at"}, {"action": "move_cards", "activation_position": "center", "all": true, "destination": "discard", "exclude_self": true, "source": "looked_at_remaining"}], "activation_position": "center", "all": true, "exclude_self": true}
```

- ライブカードかコスト10以上のメンバーカードのどちらか1つを選ぶ。選んだカードが公開されるまで、自分のデッキの一番上からカードを1枚ずつ公開する。そのカードを手札に加え、これにより公開されたほかのすべてのカードを控え室に置く (x1)

```json
{"action": "select", "activation_position": "center", "all": false, "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "exclude_self": true, "or_card_types": ["live_card", "member_card"]}
```


```json
{"action": "reveal", "activation_position": "center", "all": false, "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "exclude_self": true, "multiple_targets": true, "source": "deck_top"}
```


```json
{"action": "move_cards", "activation_position": "center", "all": false, "count": 1, "destination": "hand", "exclude_self": true, "source": "looked_at"}
```


```json
{"action": "move_cards", "activation_position": "center", "all": true, "destination": "discard", "exclude_self": true, "source": "looked_at_remaining"}
```


```json
{"card_count": 2, "cards": ["PL!-pb1-002-R | 絢瀬絵里 (ab#0)", "PL!-pb1-002-P＋ | 絢瀬絵里 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["BiBi"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["BiBi"], "original_value": true, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["BiBi"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["BiBi"], "original_value": true, "state_change": "wait", "target": "opponent"}
```

- 自分のステージにいるメンバーが『BiBi』のみの場合、相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が3つ以下のメンバー1人をウェイトにする (x1)

```json
{"all_members": true, "card_type": "member_card", "group_names": ["BiBi"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーが『BiBi』のみの場合 (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-002-R | 絢瀬絵里 (ab#1)", "PL!-pb1-002-P＋ | 絢瀬絵里 (ab#1)"], "effect": {"action": "gain_resource", "count": 1, "heart_colors": ["heart06"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "heart", "state": "wait", "target": "opponent"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "count": 1, "heart_colors": ["heart06"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "heart", "state": "wait", "target": "opponent"}
```

- 相手のステージにいるウェイト状態のメンバー1人につき、{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-003-R | 南ことり (ab#0)", "PL!-pb1-003-P＋ | 南ことり (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "card_type": "energy_card", "count": 1, "group_names": ["Printemps"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "state_change": "active", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "energy_card", "count": 1, "group_names": ["Printemps"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "state_change": "active", "target": "self"}
```

- 自分のステージにいる『Printemps』のメンバー1人につき、エネルギーを1枚アクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-004-R | 園田海未 (ab#0)", "PL!-pb1-004-P＋ | 園田海未 (ab#0)"], "effect": {"action": "conditional_alternative", "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}, "activation_position": "center", "alternative_effect": {"ability_gain": "ライブの合計スコアを+2する。", "action": "gain_ability", "activation_position": "center"}, "condition": {"count": 1, "group_names": ["μ's"], "location": "success_live_card_zone", "operator": "=", "position": "center", "target": "self", "type": "card_count_condition"}, "group_names": ["μ's"], "parenthetical": ["この能力はセンターエリアに登場した場合のみ発動する。"], "position": "center", "primary_effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "activation_position": "center", "count": 2, "duration": "live_end"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "conditional_alternative", "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "center", "type": "appearance_condition"}, "activation_position": "center", "alternative_effect": {"ability_gain": "ライブの合計スコアを+2する。", "action": "gain_ability", "activation_position": "center"}, "condition": {"count": 1, "group_names": ["μ's"], "location": "success_live_card_zone", "operator": "=", "position": "center", "target": "self", "type": "card_count_condition"}, "group_names": ["μ's"], "parenthetical": ["この能力はセンターエリアに登場した場合のみ発動する。"], "position": "center", "primary_effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "activation_position": "center", "count": 2, "duration": "live_end"}}
```

- 自分の成功ライブカード置き場に{icon_score.png|スコア}を持つ『μ's』のカードが1枚ある場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る。2枚以上ある場合、代わりに「{jyouji.png|常時}ライブの合計スコアを+2する。」を得る (x1)

```json
{"ability_gain": "ライブの合計スコアを+2する。", "action": "gain_ability", "activation_position": "center"}
```

- 「{jyouji.png|常時}ライブの合計スコアを+2する。」を得る (x1)

```json
{"count": 1, "group_names": ["μ's"], "location": "success_live_card_zone", "operator": "=", "position": "center", "target": "self", "type": "card_count_condition"}
```

- {center.png|センター}自分の成功ライブカード置き場に{icon_score.png|スコア}を持つ『μ's』のカードが1枚ある場合 (x1)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "activation_position": "center", "count": 2, "duration": "live_end"}
```

- 「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る。2枚以上ある場合、 (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-005-R | 星空 凛 (ab#0)", "PL!-pb1-005-P＋ | 星空 凛 (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分の成功ライブカード置き場にカードがある場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-006-R | 西木野真姫 (ab#0)", "PL!-pb1-006-P＋ | 西木野真姫 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_top", "group_names": ["μ's"], "max": true, "source": "discard", "target": "self"}, {"action": "draw_card", "condition": {"state": "wait", "type": "state_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}], "group_names": ["μ's"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_top", "group_names": ["μ's"], "max": true, "source": "discard", "target": "self"}, {"action": "draw_card", "condition": {"state": "wait", "type": "state_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}], "group_names": ["μ's"]}
```

- 自分の控え室から『μ's』のライブカードを1枚までデッキの一番上に置く。その後、相手のステージにウェイト状態のメンバーがいる場合、カードを1枚引く (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_top", "group_names": ["μ's"], "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室から『μ's』のライブカードを1枚までデッキの一番上に置く (x1)

```json
{"action": "draw_card", "condition": {"state": "wait", "type": "state_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}
```

- 相手のステージにウェイト状態のメンバーがいる場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-007-R | 東條 希 (ab#0)", "PL!-pb1-007-P＋ | 東條 希 (ab#0)"], "cost": {"count": 3, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "modify_cost", "card_type": "card", "count": 1, "destination": "discard", "dynamic_count": {"reference": "unit_count", "type": "per_unit"}, "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_type": "live_card_zone", "source": "hand"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"count": 3, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札を3枚控え室に置く (x1)

```json
{"action": "modify_cost", "card_type": "card", "count": 1, "destination": "discard", "dynamic_count": {"reference": "unit_count", "type": "per_unit"}, "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_type": "live_card_zone", "source": "hand"}
```

- 控え室に置く手札の数が1枚減る (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-008-R | 小泉花陽 (ab#0)", "PL!-pb1-008-P＋ | 小泉花陽 (ab#0)"], "cost": {"card_type": "member_card", "count": 3, "optional": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "draw_card", "count": 1, "destination": "hand", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "state": "wait"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_type": "member_card", "count": 3, "optional": true, "state_change": "wait", "type": "change_state"}
```

- メンバーを3人までウェイトにしてもよい (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "state": "wait"}
```

- これによりウェイト状態にしたメンバー1人につき、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-009-R | 矢澤にこ (ab#0)", "PL!-pb1-009-P＋ | 矢澤にこ (ab#0)"], "effect": {"action": "change_state", "blade_limit": 1, "blade_limit_operator": "<=", "card_type": "member_card", "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "blade_limit": 1, "blade_limit_operator": "<=", "card_type": "member_card", "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が1つ以下のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-009-R | 矢澤にこ (ab#1)", "PL!-pb1-009-P＋ | 矢澤にこ (ab#1)"], "effect": {"action": "restriction", "card_type": "member_card", "duration": "this_turn", "restriction_type": "cannot_activate_by_effect", "target": "both"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "restriction", "card_type": "member_card", "duration": "this_turn", "restriction_type": "cannot_activate_by_effect", "target": "both"}
```

- このターン、自分と相手のステージにいるメンバーは、効果によってはアクティブにならない (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-010-R | 高坂穂乃果 (ab#0)", "PL!-pb1-010-P＋ | 高坂穂乃果 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "resource": "blade", "target": "self"}
```

- 自分のステージにいるほかのメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-011-R | 絢瀬絵里 (ab#0)", "PL!-pb1-011-P＋ | 絢瀬絵里 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["BiBi"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "distinct": "card_name", "group_names": ["BiBi"], "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["BiBi"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "distinct": "card_name", "group_names": ["BiBi"], "state_change": "wait", "target": "opponent"}
```

- 自分のステージに名前の異なる『BiBi』のメンバーが2人以上いる場合、相手のステージにいるコスト4以下のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-012-R | 南ことり (ab#0)", "PL!-pb1-012-P＋ | 南ことり (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["Printemps"], "max": true, "state_change": "active", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["Printemps"], "max": true, "state_change": "active", "target": "self"}
```

- 自分のステージにいる『Printemps』のメンバーを1人までアクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-013-R | 園田海未 (ab#0)", "PL!-pb1-013-P＋ | 園田海未 (ab#0)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "conditional_on_result", "followup_action": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "card_type": "member_card", "duration": "live_end"}, "primary_effect": {"action": "reveal", "blind": true, "count": 1, "source": "hand", "target": "self"}, "result_condition": {"card_type": "live_card", "location": "revealed_cards", "type": "location_condition"}}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "conditional_on_result", "followup_action": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "card_type": "member_card", "duration": "live_end"}, "primary_effect": {"action": "reveal", "blind": true, "count": 1, "source": "hand", "target": "self"}, "result_condition": {"card_type": "live_card", "location": "revealed_cards", "type": "location_condition"}}
```

- 自分の手札を、相手は見ないで1枚選び公開する。これにより公開されたカードがライブカードの場合、ライブ終了時まで、このメンバーは「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"action": "reveal", "blind": true, "count": 1, "source": "hand", "target": "self"}
```

- 自分の手札を、相手は見ないで1枚選び公開する。 (x1)

```json
{"card_type": "live_card", "location": "revealed_cards", "type": "location_condition"}
```

- これにより公開されたカードがライブカードの場合 (x1)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "card_type": "member_card", "duration": "live_end"}
```

- ライブ終了時まで、このメンバーは「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-014-R | 星空 凛 (ab#0)", "PL!-pb1-014-P＋ | 星空 凛 (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "condition": {"card_type": "live_card", "group_names": ["lilywhite"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "group_names": ["lilywhite"], "location": "hand", "operation": "subtract", "value": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "condition": {"card_type": "live_card", "group_names": ["lilywhite"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "group_names": ["lilywhite"], "location": "hand", "operation": "subtract", "value": 2}
```

- 自分の成功ライブカード置き場に『lilywhite』のカードがある場合、手札にあるこのメンバーカードのコストは2減る (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-015-R | 西木野真姫 (ab#0)", "PL!-pb1-015-P＋ | 西木野真姫 (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "group_names": ["BiBi"], "optional": true, "position": "center", "state_change": "wait", "type": "change_state"}, "effect": {"action": "opponent_action", "action_by": "opponent", "activation_position": "center", "opponent_action": {"action": "change_state", "activation_position": "center", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}, "parenthetical": ["この能力はセンターエリアにいる場合のみ発動する。"]}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"card_type": "member_card", "count": 1, "group_names": ["BiBi"], "optional": true, "position": "center", "state_change": "wait", "type": "change_state"}
```

- {center.png|センター}『BiBi』のメンバー1人をウェイトにしてもよい (x1)

```json
{"action": "opponent_action", "action_by": "opponent", "activation_position": "center", "opponent_action": {"action": "change_state", "activation_position": "center", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}, "parenthetical": ["この能力はセンターエリアにいる場合のみ発動する。"]}
```

- 相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする (x1)

```json
{"action": "change_state", "activation_position": "center", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}
```

- 自身のステージにいるアクティブ状態のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-015-R | 西木野真姫 (ab#1)", "PL!-pb1-015-P＋ | 西木野真姫 (ab#1)"], "effect": {"action": "draw_card", "condition": {"from_state": "active", "target": "both", "to_state": "wait", "type": "state_change_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "draw_card", "condition": {"from_state": "active", "target": "both", "to_state": "wait", "type": "state_change_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く (x1)

```json
{"from_state": "active", "target": "both", "to_state": "wait", "type": "state_change_condition"}
```

- 自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-016-R | 東條 希 (ab#0)", "PL!-pb1-016-P＋ | 東條 希 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["lilywhite"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["lilywhite"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["lilywhite"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["lilywhite"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを4枚見る。その中から『lilywhite』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["lilywhite"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!-pb1-017-R | 小泉花陽 (ab#0)", "PL!-pb1-017-P＋ | 小泉花陽 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Printemps"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "condition": {"appearance": true, "baton_touch_trigger": true, "group_names": ["Printemps"], "location": "stage", "type": "appearance_condition"}, "count": 1, "destination": "discard", "duration": "unless", "source": "hand"}], "group_names": ["Printemps"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Printemps"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "condition": {"appearance": true, "baton_touch_trigger": true, "group_names": ["Printemps"], "location": "stage", "type": "appearance_condition"}, "count": 1, "destination": "discard", "duration": "unless", "source": "hand"}], "group_names": ["Printemps"]}
```

- カードを1枚引く。その後、このメンバーが『Printemps』のメンバーからバトンタッチして登場していないかぎり、手札を1枚控え室に置く (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Printemps"], "source": "deck"}
```

- カードを1枚引く。 (x1)

```json
{"action": "move_cards", "card_type": "card", "condition": {"appearance": true, "baton_touch_trigger": true, "group_names": ["Printemps"], "location": "stage", "type": "appearance_condition"}, "count": 1, "destination": "discard", "duration": "unless", "source": "hand"}
```

- 手札を1枚控え室に置く (x1)

```json
{"appearance": true, "baton_touch_trigger": true, "group_names": ["Printemps"], "location": "stage", "type": "appearance_condition"}
```

- このメンバーが『Printemps』のメンバーからバトンタッチして登場していない (x1)

```json
{"card_count": 2, "cards": ["PL!-pb1-018-R | 矢澤にこ (ab#0)", "PL!-pb1-018-P＋ | 矢澤にこ (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "multiple_targets": true, "parenthetical": ["この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。"], "source": "discard", "state_change": "wait", "target": "both"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "multiple_targets": true, "parenthetical": ["この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。"], "source": "discard", "state_change": "wait", "target": "both"}
```

- 自分と相手はそれぞれ、自身の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-001-R | 高坂穂乃果 (ab#0)", "PL!-bp4-001-P | 高坂穂乃果 (ab#0)"], "effect": {"action": "draw_card", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "draw_card", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のステージにいるメンバーのコストの合計が相手より低い場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-004-R | 園田海未 (ab#0)", "PL!-bp4-004-P | 園田海未 (ab#0)"], "effect": {"action": "change_state", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "state_change": "active"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "state_change": "active"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合、エネルギーを2枚アクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-006-R | 西木野真姫 (ab#0)", "PL!-bp4-006-P | 西木野真姫 (ab#0)"], "effect": {"action": "look_and_select", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 3, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 3, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が3以上の場合、自分のデッキの上からカードを5枚見る。その中から『μ's』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-007-R | 東條 希 (ab#0)", "PL!-bp4-007-P | 東條 希 (ab#0)"], "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"aggregate": "total", "card_type": "live_card", "conditions": [{"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"aggregate": "total", "comparison_type": "score", "cost_limit": 1, "operator": "<=", "type": "comparison_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "duration": "live_end"}, "is_null": false, "triggers": "登場"}
```


```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"aggregate": "total", "card_type": "live_card", "conditions": [{"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"aggregate": "total", "comparison_type": "score", "cost_limit": 1, "operator": "<=", "type": "comparison_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "duration": "live_end"}
```

- 自分の成功ライブカード置き場にカードが1枚以上あり、かつスコアの合計が1以下の場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"aggregate": "total", "card_type": "live_card", "conditions": [{"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"aggregate": "total", "comparison_type": "score", "cost_limit": 1, "operator": "<=", "type": "comparison_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}
```

- 自分の成功ライブカード置き場にカードが1枚以上あり、かつスコアの合計が1以下の場合 (x1)

```json
{"count": 1, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の成功ライブカード置き場にカードが1枚以上あり、 (x1)

```json
{"aggregate": "total", "comparison_type": "score", "cost_limit": 1, "operator": "<=", "type": "comparison_condition"}
```

- スコアの合計が1以下の場合 (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-008-R | 小泉花陽 (ab#0)", "PL!-bp4-008-P | 小泉花陽 (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 3}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 3}
```

- ステージにいるこのメンバーのコストを+3する (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-009-R | 矢澤にこ (ab#0)", "PL!-bp4-009-P | 矢澤にこ (ab#0)"], "effect": {"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "change_state", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "change_state", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}}
```

- 相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-016-N | 東條 希 (ab#0)", "PL!-bp5-015-N | 西木野真姫 (ab#0)"], "effect": {"action": "draw_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 3, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 3, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が3以上の場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!-bp4-022-L | No brand girls (ab#0)", "PL!-bp4-022-SECL | No brand girls (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 9, "group_names": ["μ's"], "operator": ">=", "position": "center", "target": "self", "type": "group_condition"}, "group_names": ["μ's"], "operation": "add", "position": "center", "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 9, "group_names": ["μ's"], "operator": ">=", "position": "center", "target": "self", "type": "group_condition"}, "group_names": ["μ's"], "operation": "add", "position": "center", "self_target": true, "value": 2}
```

- 自分のセンターエリアに{icon_blade.png|ブレード}を9つ以上持つ『μ's』のメンバーがいる場合、このカードのスコアを+2する (x1)

```json
{"card_type": "member_card", "count": 9, "group_names": ["μ's"], "operator": ">=", "position": "center", "target": "self", "type": "group_condition"}
```

- 自分のセンターエリアに{icon_blade.png|ブレード}を9つ以上持つ『μ's』のメンバーがいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-001-R | 上原歩夢 (ab#0)", "PL!N-bp4-001-P | 上原歩夢 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"comparison_target": "opponent", "operator": "<", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"comparison_target": "opponent", "operator": "<", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 自分のエネルギーが相手より少ない場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"comparison_target": "opponent", "operator": "<", "resource_type": "energy", "target": "self", "type": "comparison_condition"}
```

- 自分のエネルギーが相手より少ない場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-002-R | 中須かすみ (ab#0)", "PL!N-bp4-002-P | 中須かすみ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "look_at", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "target": "self"}], "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "look_at", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "target": "self"}], "target": "self"}
```

- 自分は、そのプレイヤーのデッキの一番上のカードを見る。自分はそのカードを控え室に置いてもよい (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand", "target": "self"}
```

- 自分はそのカードを控え室に置いてもよい (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-003-R | 桜坂しずく (ab#0)", "PL!N-bp4-003-P | 桜坂しずく (ab#0)"], "effect": {"action": "draw_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- ライブの合計スコアが相手より高い場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-006-R | 近江彼方 (ab#0)", "PL!N-bp4-006-P | 近江彼方 (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "conditional_on_result", "followup_action": {"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "wait"}, "group_names": ["虹ヶ咲"], "primary_effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "group_names": ["虹ヶ咲"], "source": "hand", "target": "self"}, "result_condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "conditional_on_result", "followup_action": {"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "wait"}, "group_names": ["虹ヶ咲"], "primary_effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "group_names": ["虹ヶ咲"], "source": "hand", "target": "self"}, "result_condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}}
```

- 自分の手札からコスト4以下の『虹ヶ咲』のメンバーカードを1枚ステージに登場させる。これにより登場したメンバーがブレードハートを持つ場合、このメンバーをウェイトにする (x1)

```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "group_names": ["虹ヶ咲"], "source": "hand", "target": "self"}
```

- 自分の手札からコスト4以下の『虹ヶ咲』のメンバーカードを1枚ステージに登場させる。 (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-008-R | エマ・ヴェルデ (ab#0)", "PL!N-bp4-008-P | エマ・ヴェルデ (ab#0)"], "cost": {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["虹ヶ咲"], "state_change": "active"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["虹ヶ咲"], "state_change": "active"}
```

- エネルギー1枚か『虹ヶ咲』のメンバー1人をアクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-009-R | 天王寺璃奈 (ab#0)", "PL!N-bp4-009-P | 天王寺璃奈 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "source": "hand", "target": "self"}], "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "source": "hand", "target": "self"}], "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}}
```

- 自分のステージにいるメンバーのコストの合計が相手より低い場合、カードを2枚引き、自分の手札を1枚デッキの一番上に置く (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "source": "hand", "target": "self"}
```

- 自分の手札を1枚デッキの一番上に置く (x1)

```json
{"card_count": 2, "cards": ["PL!N-bp4-012-R | 鐘 嵐珠 (ab#0)", "PL!N-bp4-012-P | 鐘 嵐珠 (ab#0)"], "effect": {"action": "modify_score", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "comparison_condition"}
```

- 相手の成功ライブカード置き場にあるカードのスコアの合計が6以上であるかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-001-R | 澁谷かのん (ab#0)", "PL!SP-bp4-001-P | 澁谷かのん (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "member_card", "conditions": [{"all_members": true, "card_type": "member_card", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "group_condition"}, {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 1, "destination": "energy_zone", "group_names": ["Liella!"], "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "member_card", "conditions": [{"all_members": true, "card_type": "member_card", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "group_condition"}, {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 1, "destination": "energy_zone", "group_names": ["Liella!"], "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 自分のステージにいるメンバーが『Liella!』のみで、かつ自分のエネルギーが7枚以上ある場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_type": "member_card", "conditions": [{"all_members": true, "card_type": "member_card", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "group_condition"}, {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージにいるメンバーが『Liella!』のみで、かつ自分のエネルギーが7枚以上ある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-002-R | 唐 可可 (ab#0)", "PL!SP-bp4-002-P | 唐 可可 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Liella!"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを4枚見る。その中から必要ハートの合計が8以上の『Liella!』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "live_card", "cost_limit_operator": ">=", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Liella!"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!SP-bp4-003-R | 嵐 千砂都 (ab#0)", "PL!SP-bp4-003-P | 嵐 千砂都 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side,right_side", "count": 2, "destination": "hand", "position": "left_side", "position_compare": "right_side", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side,right_side", "card_type": "card", "count": 2, "destination": "discard", "position": "left_side", "source": "hand"}], "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "left_side", "position_compare": "right_side", "type": "appearance_condition"}, "activation_position": "left_side,right_side", "parenthetical": ["この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。"], "position": "left_side"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "activation_position": "left_side,right_side", "count": 2, "destination": "hand", "position": "left_side", "position_compare": "right_side", "source": "deck"}, {"action": "move_cards", "activation_position": "left_side,right_side", "card_type": "card", "count": 2, "destination": "discard", "position": "left_side", "source": "hand"}], "activation_condition_parsed": {"appearance": true, "location": "stage", "position": "left_side", "position_compare": "right_side", "type": "appearance_condition"}, "activation_position": "left_side,right_side", "parenthetical": ["この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。"], "position": "left_side"}
```

- カードを2枚引き、手札を2枚控え室に置く (x1)

```json
{"action": "draw_card", "activation_position": "left_side,right_side", "count": 2, "destination": "hand", "position": "left_side", "position_compare": "right_side", "source": "deck"}
```

- カードを2枚引き (x1)

```json
{"action": "move_cards", "activation_position": "left_side,right_side", "card_type": "card", "count": 2, "destination": "discard", "position": "left_side", "source": "hand"}
```

- 手札を2枚控え室に置く (x1)

```json
{"appearance": true, "location": "stage", "position": "left_side", "position_compare": "right_side", "type": "appearance_condition"}
```

- この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。 (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-003-R | 嵐 千砂都 (ab#1)", "PL!SP-bp4-003-P | 嵐 千砂都 (ab#1)"], "effect": {"action": "gain_resource", "activation_position": "center", "count": 2, "position": "center", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "activation_position": "center", "count": 2, "position": "center", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-006-R | 桜小路きな子 (ab#0)", "PL!SP-bp4-006-P | 桜小路きな子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"distinct": "card_name", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Liella!"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"distinct": "card_name", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Liella!"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中に、名前が異なる『Liella!』のメンバーカードが3枚以上ある場合、エールにより公開された自分のカードの中から『Liella!』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-007-R | 米女メイ (ab#0)", "PL!SP-bp4-007-P | 米女メイ (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "cost_limit": 3, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "cost_limit": 3, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}
```

- このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の『Liella!』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-009-R | 鬼塚夏美 (ab#0)", "PL!SP-bp4-009-P | 鬼塚夏美 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 3, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "location": "stage", "operator": "<", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 3, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp4-010-R | ウィーン・マルガレーテ (ab#0)", "PL!SP-bp4-010-P | ウィーン・マルガレーテ (ab#0)"], "cost": {"costs": [{"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"costs": [{"count": 1, "energy": 1, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}], "type": "sequential_cost"}
```

- {icon_energy.png|E}このメンバーをウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-001-R | 上原歩夢 (ab#0)", "PL!N-pb1-001-P＋ | 上原歩夢 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 11, "exclude_self": true, "location": "stage", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 11, "exclude_self": true, "location": "stage", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分のステージにこのメンバー以外のコスト11のメンバーがいる場合、自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える (x1)

```json
{"card_type": "member_card", "comparison_type": "cost", "cost_limit": 11, "exclude_self": true, "location": "stage", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにこのメンバー以外のコスト11のメンバーがいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-001-R | 上原歩夢 (ab#1)", "PL!N-pb1-001-P＋ | 上原歩夢 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "count": 2, "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "count": 2, "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "live_card", "count": 2, "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のライブ中のライブカードが2枚以上あるかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-002-R | 中須かすみ (ab#0)", "PL!N-pb1-002-P＋ | 中須かすみ (ab#0)"], "effect": {"action": "place_energy_under_member", "card_type": "member_card", "count": 2, "destination": "under_member", "energy_count": 2, "optional": true, "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "place_energy_under_member", "card_type": "member_card", "count": 2, "destination": "under_member", "energy_count": 2, "optional": true, "target": "self"}
```

- 自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-002-R | 中須かすみ (ab#1)", "PL!N-pb1-002-P＋ | 中須かすみ (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 2, "location": "energy_zone", "operator": ">=", "type": "card_count_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "parenthetical": ["メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに戻す。"], "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 2, "location": "energy_zone", "operator": ">=", "type": "card_count_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "parenthetical": ["メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに戻す。"], "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"card_type": "member_card", "count": 2, "location": "energy_zone", "operator": ">=", "type": "card_count_condition"}
```

- このメンバーの下にエネルギーカードが2枚以上置かれているかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-003-R | 桜坂しずく (ab#0)", "PL!N-pb1-003-P＋ | 桜坂しずく (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "draw_card", "activation_condition_parsed": {"check_self": true, "count": 1, "location": "hand", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "member_card", "count": 1, "destination": "hand", "duration": "live_end", "group_names": ["虹ヶ咲"], "source": "deck", "target": "self", "target_count": 1}, "is_null": false, "triggers": "起動"}
```


```json
{"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}
```

- {icon_energy.png|E}{icon_energy.png|E}このカードを手札から控え室に置く (x1)

```json
{"action": "draw_card", "activation_condition_parsed": {"check_self": true, "count": 1, "location": "hand", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "member_card", "count": 1, "destination": "hand", "duration": "live_end", "group_names": ["虹ヶ咲"], "source": "deck", "target": "self", "target_count": 1}
```

- カードを1枚引き、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-004-R | 朝香果林 (ab#0)", "PL!N-pb1-004-P＋ | 朝香果林 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "condition": {"type": "not_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "condition": {"type": "not_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "condition": {"type": "not_moved"}, "temporal": "this_turn", "type": "temporal_condition"}
```

- このターンにこのメンバーが移動していないかぎり (x1)

```json
{"type": "not_moved"}
```


```json
{"card_count": 2, "cards": ["PL!N-pb1-004-R | 朝香果林 (ab#1)", "PL!N-pb1-004-P＋ | 朝香果林 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "reveal", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "discard"}, {"action": "position_change", "card_type": "member_card"}, {"action": "move_cards", "card_type": "card", "condition": {"type": "otherwise_condition"}, "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "reveal", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "discard"}, {"action": "position_change", "card_type": "member_card"}, {"action": "move_cards", "card_type": "card", "condition": {"type": "otherwise_condition"}, "count": 1, "destination": "discard", "source": "hand"}]}
```

- 自分のデッキの一番上のカードを公開する。公開したカードがコスト9以下のメンバーカードの場合、公開したカードを手札に加え、このメンバーはポジションチェンジする。それ以外の場合、公開したカードを控え室に置く (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "source": "discard"}
```

- 公開したカードを手札に加え (x1)

```json
{"action": "move_cards", "card_type": "card", "condition": {"type": "otherwise_condition"}, "count": 1, "destination": "discard", "source": "hand"}
```

- それ以外の場合、公開したカードを控え室に置く (x1)

```json
{"type": "otherwise_condition"}
```

- それ以外の場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-005-R | 宮下 愛 (ab#0)", "PL!N-pb1-005-P＋ | 宮下 愛 (ab#0)"], "effect": {"action": "draw_card", "condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "draw_card", "condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のステージにコスト10のメンバーが登場したとき、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-006-R | 近江彼方 (ab#0)", "PL!N-pb1-006-P＋ | 近江彼方 (ab#0)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "card_type": "energy_card", "count": 1, "state_change": "active"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 2, "cards": ["PL!N-pb1-007-R | 優木せつ菜 (ab#0)", "PL!N-pb1-007-P＋ | 優木せつ菜 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"target": "self", "temporal": "during_live", "type": "temporal_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"target": "self", "temporal": "during_live", "type": "temporal_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "resource": "heart"}
```

- {icon_all.png|ハート}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-008-R | エマ・ヴェルデ (ab#0)", "PL!N-pb1-008-P＋ | エマ・ヴェルデ (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "condition": {"state": "wait", "type": "state_condition"}, "conditional": true, "duration": "as_long_as", "location": "hand", "operation": "subtract", "value": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "condition": {"state": "wait", "type": "state_condition"}, "conditional": true, "duration": "as_long_as", "location": "hand", "operation": "subtract", "value": 2}
```

- 手札にあるこのメンバーカードのコストは2減る (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-008-R | エマ・ヴェルデ (ab#1)", "PL!N-pb1-008-P＋ | エマ・ヴェルデ (ab#1)"], "effect": {"action": "change_state", "card_type": "member_card", "count": 2, "state_change": "active", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "count": 2, "state_change": "active", "target": "self"}
```

- 自分のステージにいるメンバー1人か、エネルギーを2枚アクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-009-R | 天王寺璃奈 (ab#0)", "PL!N-pb1-009-P＋ | 天王寺璃奈 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "count": 3, "duration": "live_end", "heart_colors": ["heart03", "heart05", "heart06"], "resource": "heart"}], "condition": {"card_type": "member_card", "heart_colors": ["heart03", "heart05", "heart06"], "location": "stage", "negation": true, "type": "location_condition"}, "heart_colors": ["heart03", "heart05", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "count": 3, "duration": "live_end", "heart_colors": ["heart03", "heart05", "heart06"], "resource": "heart"}], "condition": {"card_type": "member_card", "heart_colors": ["heart03", "heart05", "heart06"], "location": "stage", "negation": true, "type": "location_condition"}, "heart_colors": ["heart03", "heart05", "heart06"]}
```

- このターン、ブレードハートを持たないメンバーカードが自分のライブカード置き場から控え室に置かれている場合、カードを1枚引き、ライブ終了時まで、{heart_03.png|heart03}{heart_05.png|heart05}{heart_06.png|heart06}を得る (x1)

```json
{"card_type": "member_card", "heart_colors": ["heart03", "heart05", "heart06"], "location": "stage", "negation": true, "type": "location_condition"}
```

- このターン、ブレードハートを持たないメンバーカードが自分のライブカード置き場から控え室に置かれている場合 (x1)

```json
{"action": "gain_resource", "count": 3, "duration": "live_end", "heart_colors": ["heart03", "heart05", "heart06"], "resource": "heart"}
```

- {heart_03.png|heart03}{heart_05.png|heart05}{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-010-R | 三船栞子 (ab#0)", "PL!N-pb1-010-P＋ | 三船栞子 (ab#0)"], "effect": {"action": "choice", "count": 1, "group_names": ["虹ヶ咲"], "options": [{"action": "change_state", "card_type": "energy_card", "count": 1, "state_change": "active"}, {"action": "move_cards", "card_type": "live_card", "count": 2, "destination": "deck_top", "group_names": ["虹ヶ咲"], "max": true, "placement_order": "any_order", "source": "discard", "target": "self"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "choice", "count": 1, "group_names": ["虹ヶ咲"], "options": [{"action": "change_state", "card_type": "energy_card", "count": 1, "state_change": "active"}, {"action": "move_cards", "card_type": "live_card", "count": 2, "destination": "deck_top", "group_names": ["虹ヶ咲"], "max": true, "placement_order": "any_order", "source": "discard", "target": "self"}]}
```

- 以下から1つを選ぶ。
・エネルギーを1枚アクティブにする。
・自分の控え室にある『虹ヶ咲』のライブカードを2枚まで好きな順番でデッキの上に置く (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 2, "destination": "deck_top", "group_names": ["虹ヶ咲"], "max": true, "placement_order": "any_order", "source": "discard", "target": "self"}
```

- 自分の控え室にある『虹ヶ咲』のライブカードを2枚まで好きな順番でデッキの上に置く (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-011-R | ミア・テイラー (ab#0)", "PL!N-pb1-011-P＋ | ミア・テイラー (ab#0)"], "effect": {"action": "gain_resource", "card_type": "energy_card", "count": 1, "location": "under_member", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "card_type": "energy_card", "count": 1, "location": "under_member", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "blade"}
```

- このメンバーの下にあるエネルギーカード1枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-011-R | ミア・テイラー (ab#1)", "PL!N-pb1-011-P＋ | ミア・テイラー (ab#1)"], "cost": {"card_type": "member_card", "count": 1, "destination": "under_member", "target": "self", "type": "custom"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "parenthetical": ["メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに戻す。"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "count": 1, "destination": "under_member", "target": "self", "type": "custom"}
```

- 自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "parenthetical": ["メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに戻す。"], "source": "discard", "target": "self"}
```

- 自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-012-R | 鐘 嵐珠 (ab#0)", "PL!N-pb1-012-P＋ | 鐘 嵐珠 (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-012-R | 鐘 嵐珠 (ab#1)", "PL!N-pb1-012-P＋ | 鐘 嵐珠 (ab#1)"], "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["虹ヶ咲"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["虹ヶ咲"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、『虹ヶ咲』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-013-R | 上原歩夢 (ab#0)", "PL!N-pb1-013-P＋ | 上原歩夢 (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "characters": ["上原歩夢"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "characters": ["上原歩夢"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}
```

- 手札からコスト4以下の「上原歩夢」のメンバーカードを1枚ステージに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-014-R | 中須かすみ (ab#0)", "PL!N-pb1-014-P＋ | 中須かすみ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "中須かすみ", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "中須かすみ", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}
```

- 「中須かすみ」からバトンタッチして登場した場合、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"baton_touch_source": "中須かすみ", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 「中須かすみ」からバトンタッチして登場した場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-015-R | 桜坂しずく (ab#0)", "PL!N-pb1-015-P＋ | 桜坂しずく (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "characters": ["桜坂しずく"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "characters": ["桜坂しずく"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}
```

- 手札からコスト4以下の「桜坂しずく」のメンバーカードを1枚ステージに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-016-R | 朝香果林 (ab#0)", "PL!N-pb1-016-P＋ | 朝香果林 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!N-pb1-017-R | 宮下 愛 (ab#0)", "PL!N-pb1-017-P＋ | 宮下 愛 (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "characters": ["宮下愛"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "characters": ["宮下愛"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}
```

- 手札からコスト4以下の「宮下愛」のメンバーカードを1枚ステージに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-018-R | 近江彼方 (ab#0)", "PL!N-pb1-018-P＋ | 近江彼方 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!N-pb1-019-R | 優木せつ菜 (ab#0)", "PL!N-pb1-019-P＋ | 優木せつ菜 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "優木せつ菜", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "優木せつ菜", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}
```

- 「優木せつ菜」からバトンタッチして登場した場合、カードを2枚引き、手札を2枚控え室に置く (x1)

```json
{"baton_touch_source": "優木せつ菜", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 「優木せつ菜」からバトンタッチして登場した場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-020-R | エマ・ヴェルデ (ab#0)", "PL!N-pb1-020-P＋ | エマ・ヴェルデ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "エマ・ヴェルデ", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "エマ・ヴェルデ", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}
```

- 「エマ・ヴェルデ」からバトンタッチして登場した場合、カードを2枚引き、手札を2枚控え室に置く (x1)

```json
{"baton_touch_source": "エマ・ヴェルデ", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 「エマ・ヴェルデ」からバトンタッチして登場した場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-021-R | 天王寺璃奈 (ab#0)", "PL!N-pb1-021-P＋ | 天王寺璃奈 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!N-pb1-022-R | 三船栞子 (ab#0)", "PL!N-pb1-022-P＋ | 三船栞子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "三船栞子", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"baton_touch_source": "三船栞子", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}}
```

- 「三船栞子」からバトンタッチして登場した場合、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"baton_touch_source": "三船栞子", "baton_touch_trigger": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 「三船栞子」からバトンタッチして登場した場合 (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-023-R | ミア・テイラー (ab#0)", "PL!N-pb1-023-P＋ | ミア・テイラー (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "characters": ["ミア・テイラー"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "characters": ["ミア・テイラー"], "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "quoted_text": {"quoted_type": "character"}, "source": "hand"}
```

- 手札からコスト4以下の「ミア・テイラー」のメンバーカードを1枚ステージに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!N-pb1-024-R | 鐘 嵐珠 (ab#0)", "PL!N-pb1-024-P＋ | 鐘 嵐珠 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!N-pb1-028-N | 朝香果林 (ab#0)", "PL!N-pb1-035-N | ミア・テイラー (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}
```

- 自分のデッキの上からカードを2枚見る。その中から1枚を手札に加え、残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp5-011-N | 大沢瑠璃乃 (ab#0)", "PL!SP-sd2-009-SD2 | 鬼塚夏美 (ab#0)"], "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!-bp5-111-R | 綺羅ツバサ (ab#0)", "PL!-bp5-111-P＋ | 綺羅ツバサ (ab#0)"], "effect": {"action": "gain_resource", "count": 1, "group_names": ["A-RISE"], "heart_colors": ["heart05"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "heart", "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "count": 1, "group_names": ["A-RISE"], "heart_colors": ["heart05"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "resource": "heart", "target": "self"}
```

- 自分のステージにいるこのメンバー以外の『A-RISE』のメンバー1人につき、{heart_05.png|heart05}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-bp5-111-R | 綺羅ツバサ (ab#1)", "PL!-bp5-111-P＋ | 綺羅ツバサ (ab#1)"], "cost": {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "conditional_on_result", "followup_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "primary_effect": {"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "active"}, "result_condition": {"card_type": "member_card", "location": "stage", "target": "opponent", "type": "location_condition"}}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "conditional_on_result", "followup_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "primary_effect": {"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "active"}, "result_condition": {"card_type": "member_card", "location": "stage", "target": "opponent", "type": "location_condition"}}
```

- ウェイト状態のメンバー1人をアクティブにする。これにより相手のステージにいるメンバーをアクティブにした場合、自分の控え室からライブカードを1枚手札に加える (x1)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "state_change": "active"}
```

- ウェイト状態のメンバー1人をアクティブにする。 (x1)

```json
{"card_type": "member_card", "location": "stage", "target": "opponent", "type": "location_condition"}
```

- これにより相手のステージにいるメンバーをアクティブにした場合 (x1)

```json
{"card_count": 2, "cards": ["PL!-bp5-222-R | 優木あんじゅ (ab#0)", "PL!-bp5-222-P＋ | 優木あんじゅ (ab#0)"], "cost": {"costs": [{"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!-bp5-333-R | 統堂英玲奈 (ab#0)", "PL!-bp5-333-P＋ | 統堂英玲奈 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 2, "cards": ["PL!-bp5-333-R | 統堂英玲奈 (ab#1)", "PL!-bp5-333-P＋ | 統堂英玲奈 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"state": "wait", "type": "state_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"state": "wait", "type": "state_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart05"], "resource": "heart"}
```

- {heart_05.png|heart05}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp5-111-R | 鹿角聖良 (ab#1)", "PL!S-bp5-111-P＋ | 鹿角聖良 (ab#1)"], "effect": {"action": "change_state", "blade_limit": 2, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "change_state", "blade_limit": 2, "blade_limit_operator": "<=", "card_type": "member_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}
```

- このメンバーがエリアを移動したとき、相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が2つ以下のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp5-222-R | 鹿角理亞 (ab#1)", "PL!S-bp5-222-P＋ | 鹿角理亞 (ab#1)"], "effect": {"action": "change_state", "card_type": "energy_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 2, "state_change": "active"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "change_state", "card_type": "energy_card", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 2, "state_change": "active"}
```

- このメンバーがエリアを移動したとき、エネルギーを2枚アクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp5-111-R | 柊摩央 (ab#1)", "PL!SP-bp5-111-P＋ | 柊摩央 (ab#1)"], "cost": {"count": 2, "destination": "energy_deck", "type": "custom"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"count": 2, "destination": "energy_deck", "type": "custom"}
```

- エネルギー2枚をエネルギーデッキに置く (x1)

```json
{"card_count": 2, "cards": ["PL!SP-bp5-222-R | 聖澤悠奈 (ab#1)", "PL!SP-bp5-222-P＋ | 聖澤悠奈 (ab#1)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!HS-sd1-018-SD | Dream Believers（105期Ver.） (ab#0)", "PL!HS-sd1-018-SECL | Dream Believers（105期Ver.） (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 3, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, {"card_type": "live_card", "location": "discard", "target": "self", "type": "location_condition"}], "location": "discard", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["蓮ノ空"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 3, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, {"card_type": "live_card", "location": "discard", "target": "self", "type": "location_condition"}], "location": "discard", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["蓮ノ空"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージに『蓮ノ空』のメンバーが3人以上いて、かつ自分の控え室にカード名に「DreamBelievers」を含むライブカードがある場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 3, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, {"card_type": "live_card", "location": "discard", "target": "self", "type": "location_condition"}], "location": "discard", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージに『蓮ノ空』のメンバーが3人以上いて、かつ自分の控え室にカード名に「DreamBelievers」を含むライブカードがある場合 (x1)

```json
{"card_type": "member_card", "count": 3, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに『蓮ノ空』のメンバーが3人以上いて、 (x1)

```json
{"card_type": "live_card", "location": "discard", "target": "self", "type": "location_condition"}
```

- 自分の控え室にカード名に「DreamBelievers」を含むライブカードがある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-001-R | 日野下花帆 (ab#0)", "PL!HS-pb1-001-P＋ | 日野下花帆 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "pay_energy", "count": 1, "energy": 1, "exclude_self": true, "group_names": ["スリーズブーケ"], "optional": true}, {"action": "change_state", "card_type": "energy_card", "count": 2, "exclude_self": true, "state_change": "active"}], "conditional": true, "exclude_self": true, "group_names": ["スリーズブーケ"], "trigger_condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動", "use_limit": 2}
```


```json
{"action": "sequential", "actions": [{"action": "pay_energy", "count": 1, "energy": 1, "exclude_self": true, "group_names": ["スリーズブーケ"], "optional": true}, {"action": "change_state", "card_type": "energy_card", "count": 2, "exclude_self": true, "state_change": "active"}], "conditional": true, "exclude_self": true, "group_names": ["スリーズブーケ"], "trigger_condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "trigger_type": "each_time"}
```

- 自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{icon_energy.png|E}支払ってもよい。そうした場合、エネルギーを2枚アクティブにする (x1)

```json
{"action": "pay_energy", "count": 1, "energy": 1, "exclude_self": true, "group_names": ["スリーズブーケ"], "optional": true}
```

- {icon_energy.png|E}支払ってもよい。 (x1)

```json
{"action": "change_state", "card_type": "energy_card", "count": 2, "exclude_self": true, "state_change": "active"}
```

- エネルギーを2枚アクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-001-R | 日野下花帆 (ab#1)", "PL!HS-pb1-001-P＋ | 日野下花帆 (ab#1)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}], "count": 2, "duration": "live_end", "heart_colors": ["heart04"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}], "count": 2, "duration": "live_end", "heart_colors": ["heart04"]}
```

- {heart_04.png|heart04}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade"}
```


```json
{"card_count": 2, "cards": ["PL!HS-pb1-002-R | 村野さやか (ab#0)", "PL!HS-pb1-002-P＋ | 村野さやか (ab#0)"], "cost": {"card_type": "member_card", "characters": ["村野さやか"], "count": 1, "source": "hand", "type": "reveal", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "member_card", "destination": "under_member", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_type": "member_card", "characters": ["村野さやか"], "count": 1, "source": "hand", "type": "reveal", "zone": "hand"}
```

- 手札の「村野さやか」のメンバーカードを1枚公開する (x1)

```json
{"action": "move_cards", "card_type": "member_card", "destination": "under_member", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}
```

- これにより公開したカードをこのメンバーの下に置く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-002-R | 村野さやか (ab#1)", "PL!HS-pb1-002-P＋ | 村野さやか (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "modify_cost", "card_type": "member_card", "duration": "live_end", "location": "under_member", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "value": 4}, {"action": "gain_resource", "card_type": "member_card", "count": 3, "duration": "live_end", "heart_colors": ["heart05"], "location": "under_member", "max": true, "max_repeats": 3, "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart"}], "duration": "live_end", "heart_colors": ["heart05"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_cost", "card_type": "member_card", "duration": "live_end", "location": "under_member", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "value": 4}, {"action": "gain_resource", "card_type": "member_card", "count": 3, "duration": "live_end", "heart_colors": ["heart05"], "location": "under_member", "max": true, "max_repeats": 3, "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart"}], "duration": "live_end", "heart_colors": ["heart05"]}
```

- このメンバーの下にあるメンバーカード1枚につき、このカードのコストを+4して{heart_05.png|heart05}を得る。この能力では下にあるメンバーカードは3枚までしか数えない (x1)

```json
{"action": "modify_cost", "card_type": "member_card", "duration": "live_end", "location": "under_member", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "value": 4}
```

- このカードのコストを+4 (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 3, "duration": "live_end", "heart_colors": ["heart05"], "location": "under_member", "max": true, "max_repeats": 3, "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart"}
```

- {heart_05.png|heart05}を得る。この能力では下にあるメンバーカードは3枚までしか数えない (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-003-R | 大沢瑠璃乃 (ab#0)", "PL!HS-pb1-003-P＋ | 大沢瑠璃乃 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "any_number": true, "card_type": "member_card", "destination": "discard", "group_names": ["みらくらぱーく！"], "source": "hand"}, {"action": "draw_card", "destination": "hand", "dynamic_count": {"calculation": "add", "calculation_value": 1, "mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}, "group_names": ["みらくらぱーく！"], "source": "deck"}], "group_names": ["みらくらぱーく！"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "any_number": true, "card_type": "member_card", "destination": "discard", "group_names": ["みらくらぱーく！"], "source": "hand"}, {"action": "draw_card", "destination": "hand", "dynamic_count": {"calculation": "add", "calculation_value": 1, "mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}, "group_names": ["みらくらぱーく！"], "source": "deck"}], "group_names": ["みらくらぱーく！"]}
```

- 手札の『みらくらぱーく！』のメンバーカードを好きな枚数控え室に置き、その後、その枚数に1を足した枚数のカードを引く (x1)

```json
{"action": "move_cards", "any_number": true, "card_type": "member_card", "destination": "discard", "group_names": ["みらくらぱーく！"], "source": "hand"}
```

- 手札の『みらくらぱーく！』のメンバーカードを好きな枚数控え室に置き、 (x1)

```json
{"action": "draw_card", "destination": "hand", "dynamic_count": {"calculation": "add", "calculation_value": 1, "mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}, "group_names": ["みらくらぱーく！"], "source": "deck"}
```

- その枚数に1を足した枚数のカードを引く (x1)

```json
{"calculation": "add", "calculation_value": 1, "mode": "equals", "reference": "previous_moved_cards", "type": "dynamic_count"}
```


```json
{"card_count": 2, "cards": ["PL!HS-pb1-003-R | 大沢瑠璃乃 (ab#1)", "PL!HS-pb1-003-P＋ | 大沢瑠璃乃 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "count": 2, "duration": "live_end", "heart_colors": ["heart01"], "trigger_condition": {"count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動", "use_limit": 2}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "count": 2, "duration": "live_end", "heart_colors": ["heart01"], "trigger_condition": {"count": 1, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "trigger_type": "each_time"}
```

- 自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{heart_01.png|heart01}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-004-R | 百生吟子 (ab#0)", "PL!HS-pb1-004-P＋ | 百生吟子 (ab#0)"], "cost": {"costs": [{"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ"], "source": "discard", "target": "self"}], "group_names": ["スリーズブーケ"]}, "is_null": false, "triggers": "登場"}
```


```json
{"costs": [{"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}
```

- {icon_energy.png|E}手札を1枚控え室に置いてもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ"], "source": "discard", "target": "self"}], "group_names": ["スリーズブーケ"]}
```

- 自分のデッキの上からカードを3枚控え室に置く。その後、自分の控え室から『スリーズブーケ』のライブカードを1枚手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ"], "source": "discard", "target": "self"}
```

- 自分の控え室から『スリーズブーケ』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-005-R | 徒町小鈴 (ab#0)", "PL!HS-pb1-005-P＋ | 徒町小鈴 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1}, {"action": "reveal", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "location": "revealed_cards", "type": "location_condition"}, {"comparison_type": "cost", "operator": ">=", "type": "comparison_condition"}], "operator": "and", "type": "compound"}, "count": 1, "destination": "hand", "source": "discard"}, {"action": "gain_resource", "condition": {"count": 1, "operator": "<=", "target": "self", "type": "comparison_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}], "parenthetical": ["公開したカードがメンバーカード以外の場合、何も起こらない。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1}, {"action": "reveal", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "location": "revealed_cards", "type": "location_condition"}, {"comparison_type": "cost", "operator": ">=", "type": "comparison_condition"}], "operator": "and", "type": "compound"}, "count": 1, "destination": "hand", "source": "discard"}, {"action": "gain_resource", "condition": {"count": 1, "operator": "<=", "target": "self", "type": "comparison_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}], "parenthetical": ["公開したカードがメンバーカード以外の場合、何も起こらない。"]}
```

- 数1つを選ぶ。自分のデッキの一番上のカードを公開する。公開したカードがメンバーカードで、かつコストが選んだ数以上の場合、公開したカードを手札に加える。選んだ数以下の場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "select", "count": 1}
```

- 数1つを選ぶ (x1)

```json
{"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "location": "revealed_cards", "type": "location_condition"}, {"comparison_type": "cost", "operator": ">=", "type": "comparison_condition"}], "operator": "and", "type": "compound"}, "count": 1, "destination": "hand", "source": "discard"}
```

- 公開したカードがメンバーカードで、かつコストが選んだ数以上の場合、公開したカードを手札に加える (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "member_card", "location": "revealed_cards", "type": "location_condition"}, {"comparison_type": "cost", "operator": ">=", "type": "comparison_condition"}], "operator": "and", "type": "compound"}
```

- 公開したカードがメンバーカードで、かつコストが選んだ数以上の場合 (x1)

```json
{"card_type": "member_card", "location": "revealed_cards", "type": "location_condition"}
```

- 公開したカードがメンバーカードで、 (x1)

```json
{"comparison_type": "cost", "operator": ">=", "type": "comparison_condition"}
```

- コストが選んだ数以上の場合 (x1)

```json
{"action": "gain_resource", "condition": {"count": 1, "operator": "<=", "target": "self", "type": "comparison_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- 選んだ数以下の場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"count": 1, "operator": "<=", "target": "self", "type": "comparison_condition"}
```

- 選んだ数以下の場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-006-R | 安養寺姫芽 (ab#0)", "PL!HS-pb1-006-P＋ | 安養寺姫芽 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "position_change", "card_type": "member_card", "exclude_self": true, "group_names": ["みらくらぱーく！"], "optional": true, "target": "self"}, {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "count": 2, "duration": "live_end", "exclude_self": true, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"]}], "conditional": true, "exclude_self": true, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"], "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "position_change", "card_type": "member_card", "exclude_self": true, "group_names": ["みらくらぱーく！"], "optional": true, "target": "self"}, {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "count": 2, "duration": "live_end", "exclude_self": true, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"]}], "conditional": true, "exclude_self": true, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"], "parenthetical": ["このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。"]}
```

- 自分のステージにいるほかの『みらくらぱーく！』のメンバーがいるエリアにポジションチェンジしてもよい。そうした場合、ライブ終了時まで、{heart_01.png|heart01}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "position_change", "card_type": "member_card", "exclude_self": true, "group_names": ["みらくらぱーく！"], "optional": true, "target": "self"}
```

- 自分のステージにいるほかの『みらくらぱーく！』のメンバーがいるエリアにポジションチェンジしてもよい。 (x1)

```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "blade"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}], "count": 2, "duration": "live_end", "exclude_self": true, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"]}
```

- {heart_01.png|heart01}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-007-R | セラス 柳田 リリエンフェルト (ab#0)", "PL!HS-pb1-007-P＋ | セラス 柳田 リリエンフェルト (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"costs": [{"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "optional": true, "type": "sequential_cost"}
```

- {icon_energy.png|E}{icon_energy.png|E}手札を1枚控え室に置いてもよい (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-007-R | セラス 柳田 リリエンフェルト (ab#1)", "PL!HS-pb1-007-P＋ | セラス 柳田 リリエンフェルト (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 2, "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}, {"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}], "location": "stage", "operator": "and", "target": "both", "type": "compound"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 2, "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}, {"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}], "location": "stage", "operator": "and", "target": "both", "type": "compound"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}
```

- {heart_06.png|heart06}を得る (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 2, "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}, {"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}], "location": "stage", "operator": "and", "target": "both", "type": "compound"}
```

- 自分のステージにメンバーがちょうど2人おり、かつ相手のステージにメンバーが3人以上いるかぎり (x1)

```json
{"card_type": "member_card", "count": 2, "location": "stage", "operator": "=", "target": "self", "type": "location_condition"}
```

- 自分のステージにメンバーがちょうど2人おり、 (x1)

```json
{"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}
```

- 相手のステージにメンバーが3人以上いるかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-008-R | 桂城 泉 (ab#0)", "PL!HS-pb1-008-P＋ | 桂城 泉 (ab#0)"], "effect": {"action": "change_state", "all": true, "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "original_value": true, "state_change": "wait", "target": "both"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "all": true, "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "original_value": true, "state_change": "wait", "target": "both"}
```

- 自分と相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が3つ以下のすべてのメンバーをウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-008-R | 桂城 泉 (ab#1)", "PL!HS-pb1-008-P＋ | 桂城 泉 (ab#1)"], "effect": {"action": "restriction", "card_type": "member_card", "phase": "active_phase", "restriction_type": "cannot_activate", "target": "opponent"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "restriction", "card_type": "member_card", "phase": "active_phase", "restriction_type": "cannot_activate", "target": "opponent"}
```

- 相手のステージにいるメンバーはアクティブフェイズにアクティブにならない (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-009-R | 日野下花帆 (ab#0)", "PL!HS-pb1-009-P＋ | 日野下花帆 (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "center", "count": 2, "duration": "live_end", "position": "center", "resource": "blade", "trigger_condition": {"appearance": true, "location": "stage", "position": "center", "target": "self", "type": "appearance_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動", "use_limit": 2}
```


```json
{"action": "gain_resource", "activation_position": "center", "count": 2, "duration": "live_end", "position": "center", "resource": "blade", "trigger_condition": {"appearance": true, "location": "stage", "position": "center", "target": "self", "type": "appearance_condition"}, "trigger_type": "each_time"}
```

- 自分のステージに『蓮ノ空』のメンバーが登場するたび、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"appearance": true, "location": "stage", "position": "center", "target": "self", "type": "appearance_condition"}
```

- {center.png|センター}自分のステージに『蓮ノ空』のメンバーが登場する (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-009-R | 日野下花帆 (ab#1)", "PL!HS-pb1-009-P＋ | 日野下花帆 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"count": 8, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"count": 8, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}}
```

- このメンバーが持つ{icon_blade.png|ブレード}の数が8つ以上の場合、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"count": 8, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}
```

- このメンバーが持つ{icon_blade.png|ブレード}の数が8つ以上の場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-010-R | 村野さやか (ab#0)", "PL!HS-pb1-010-P＋ | 村野さやか (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 10, "cost_total": 10, "count": 10, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 10, "cost_total": 10, "count": 10, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}
```

- 自分のステージにコスト10以上のメンバーがいる場合、相手のステージにいるコスト4以下のメンバー1人をウェイトにする (x1)

```json
{"card_type": "member_card", "comparison_type": "cost", "cost_limit": 10, "cost_total": 10, "count": 10, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにコスト10以上のメンバーがいる場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-012-R | 百生吟子 (ab#0)", "PL!HS-pb1-012-P＋ | 百生吟子 (ab#0)"], "effect": {"action": "conditional_on_result", "all": true, "followup_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "duration": "live_end", "source": "discard", "target": "self"}, "primary_effect": {"action": "move_cards", "all": true, "card_type": "member_card", "destination": "deck_bottom", "multiple_targets": true, "shuffle": true, "source": "discard", "target": "deck"}, "result_condition": {"aggregate": "total", "count": 20, "operator": ">=", "scope": "both", "source": "preceding_moved", "target": "both", "type": "card_count_condition"}, "shuffle": true}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "conditional_on_result", "all": true, "followup_action": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "duration": "live_end", "source": "discard", "target": "self"}, "primary_effect": {"action": "move_cards", "all": true, "card_type": "member_card", "destination": "deck_bottom", "multiple_targets": true, "shuffle": true, "source": "discard", "target": "deck"}, "result_condition": {"aggregate": "total", "count": 20, "operator": ">=", "scope": "both", "source": "preceding_moved", "target": "both", "type": "card_count_condition"}, "shuffle": true}
```

- 自分と相手はそれぞれ、自身の控え室にあるすべてのメンバーカードをシャッフルし、自身のデッキの下に置く。これにより自分と相手のカードが合計20枚以上デッキの下に置かれた場合、自分の控え室からライブカードを1枚手札に加え、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "all": true, "card_type": "member_card", "destination": "deck_bottom", "multiple_targets": true, "shuffle": true, "source": "discard", "target": "deck"}
```

- 自分と相手はそれぞれ、自身の控え室にあるすべてのメンバーカードをシャッフルし、自身のデッキの下に置く。 (x1)

```json
{"aggregate": "total", "count": 20, "operator": ">=", "scope": "both", "source": "preceding_moved", "target": "both", "type": "card_count_condition"}
```

- これにより自分と相手のカードが合計20枚以上デッキの下に置かれた場合 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "duration": "live_end", "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードを1枚手札に加え、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-013-R | 徒町小鈴 (ab#0)", "PL!HS-pb1-013-P＋ | 徒町小鈴 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!HS-pb1-013-R | 徒町小鈴 (ab#1)", "PL!HS-pb1-013-P＋ | 徒町小鈴 (ab#1)"], "effect": {"action": "draw_card", "condition": {"card_type": "member_card", "comparison_target": "self", "comparison_type": "cost", "location": "stage", "operator": ">", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"card_type": "member_card", "comparison_target": "self", "comparison_type": "cost", "location": "stage", "operator": ">", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分のステージに、このメンバーよりコストが高いメンバーがいる場合、カードを1枚引く (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-014-R | 安養寺姫芽 (ab#0)", "PL!HS-pb1-014-P＋ | 安養寺姫芽 (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["みらくらぱーく！"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "destination": "front", "group_names": ["みらくらぱーく！"], "position": "front", "target": "opponent", "target_member": "select"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["みらくらぱーく！"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "destination": "front", "group_names": ["みらくらぱーく！"], "position": "front", "target": "opponent", "target_member": "select"}
```

- 自分のステージにいるメンバーが『みらくらぱーく！』のみの場合、相手のステージにいるメンバー1人をこのメンバーの正面のエリアにポジションチェンジさせる (x1)

```json
{"all_members": true, "card_type": "member_card", "group_names": ["みらくらぱーく！"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーが『みらくらぱーく！』のみの場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-014-R | 安養寺姫芽 (ab#1)", "PL!HS-pb1-014-P＋ | 安養寺姫芽 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "operator": ">", "position": "front", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "operator": ">", "position": "front", "target": "opponent", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart01"], "resource": "heart"}
```

- {heart_01.png|heart01}を得る (x1)

```json
{"card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "operator": ">", "position": "front", "target": "opponent", "type": "comparison_condition"}
```

- このメンバーの正面のエリアにいる相手のメンバーのコストが、このメンバーのコストより高いかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-015-R | セラス 柳田 リリエンフェルト (ab#0)", "PL!HS-pb1-015-P＋ | セラス 柳田 リリエンフェルト (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}, "conditional": true, "count": 3, "duration": "as_long_as", "resource": "blade", "sign": "negative"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}, "conditional": true, "count": 3, "duration": "as_long_as", "resource": "blade", "sign": "negative"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を失う (x1)

```json
{"card_count": 2, "cards": ["PL!HS-pb1-016-R | 桂城 泉 (ab#0)", "PL!HS-pb1-016-P＋ | 桂城 泉 (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "filter_targets_by_heart_colors": true, "heart_colors": ["heart06"], "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "filter_targets_by_heart_colors": true, "heart_colors": ["heart06"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいるこのメンバー以外の{heart_06.png|heart06}を持つメンバー1人は、ライブ終了時まで、{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!-bp6-001-R＋ | 高坂穂乃果 (ab#0)", "PL!-bp6-001-P | 高坂穂乃果 (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "center", "all": true, "card_type": "member_card", "condition": {"card_type": "live_card", "group_names": ["µ's"], "location": "live_card_zone", "position": "center", "target": "self", "type": "group_condition"}, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-001-R＋ | 高坂穂乃果 (ab#1)", "PL!-bp6-001-P | 高坂穂乃果 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_property": "has_blade_heart", "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "group_names": ["μ's"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_property": "has_blade_heart", "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "group_names": ["μ's"]}
```

- エールにより公開された自分のカードの中に、ブレードハートを持たない『μ's』のメンバーカードがある場合、カードを1枚引き、手札を1枚控え室に置く (x1)

```json
{"card_property": "has_blade_heart", "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}
```

- エールにより公開された自分のカードの中に、ブレードハートを持たない『μ's』のメンバーカードがある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!-bp6-001-P＋ | 高坂穂乃果 (ab#0)", "PL!-bp6-001-SEC | 高坂穂乃果 (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "center", "all": true, "card_type": "member_card", "condition": {"card_type": "live_card", "group_names": ["µ's"], "location": "live_card_zone", "position": "center", "target": "self", "type": "group_condition"}, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-001-P＋ | 高坂穂乃果 (ab#1)", "PL!-bp6-001-SEC | 高坂穂乃果 (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "group_names": ["μ's"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "group_names": ["μ's"]}
```

- エールにより公開された自分のカードの中に、ブレードハートを持たない『μ's』のメンバーカードがある場合、カードを1枚引き、手札を1枚控え室に置く。" (x1)

```json
{"card_count": 2, "cards": ["PL!-bp6-002-R | 絢瀬絵里 (ab#0)", "PL!-bp6-002-P | 絢瀬絵里 (ab#0)"], "effect": {"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's", "μ's"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's", "μ's"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを2枚見る。その中から能力を持たない『μ's』のカードか{jyouji.png|常時}能力を持つ『μ's』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's", "μ's"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-003-R＋ | 南ことり (ab#0)", "PL!-bp6-003-P | 南ことり (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "under_member", "group_names": ["μ's"], "optional": true, "position": "center"}, {"action": "sequential", "actions": [{"action": "gain_resource", "activation_position": "center", "count": 1, "heart_selection": true, "position": "center", "resource": "heart"}, {"action": "gain_resource", "activation_position": "center", "count": 1, "duration": "live_end", "position": "center", "resource": "heart"}], "activation_position": "center", "group_names": ["μ's"], "position": "center"}], "activation_position": "center", "conditional": true, "group_names": ["μ's"], "position": "center"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-003-R＋ | 南ことり (ab#1)", "PL!-bp6-003-P | 南ことり (ab#1)"], "effect": {"action": "place_energy_under_member", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "energy_count": 1, "group_names": ["μ's"], "optional": true, "source": "under_member", "target_member": "this_member"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-003-P＋ | 南ことり (ab#0)", "PL!-bp6-003-SEC | 南ことり (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "activation_position": "center", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "under_member", "group_names": ["μ's"], "optional": true, "position": "center"}, {"action": "sequential", "actions": [{"action": "gain_resource", "activation_position": "center", "count": 1, "heart_selection": true, "position": "center", "resource": "heart"}, {"action": "gain_resource", "activation_position": "center", "count": 1, "duration": "live_end", "position": "center", "resource": "heart"}], "activation_position": "center", "group_names": ["μ's"], "position": "center"}], "activation_position": "center", "conditional": true, "group_names": ["μ's"], "position": "center"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-003-P＋ | 南ことり (ab#1)", "PL!-bp6-003-SEC | 南ことり (ab#1)"], "effect": {"action": "place_energy_under_member", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "energy_count": 1, "group_names": ["μ's"], "optional": true, "source": "under_member", "target_member": "this_member"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-004-R | 園田海未 (ab#0)", "PL!-bp6-004-P | 園田海未 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『μ's』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 2, "cards": ["PL!-bp6-005-R | 星空 凛 (ab#0)", "PL!-bp6-005-P | 星空 凛 (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室にある{heart_03.png|heart03}を持つメンバーカード1枚までと、必要ハートに{heart_03.png|heart03}を含むライブカード1枚までを手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室にある{heart_03.png|heart03}を持つメンバーカード1枚までと、必要ハートに{heart_03.png|heart03}を含むライブカード1枚までを手札に加える (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室にある{heart_03.png|heart03}を持つメンバーカード1枚までと、必要ハートに{heart_03.png|heart03}を含むライブカード1枚までを手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!-bp6-008-R | 小泉花陽 (ab#0)", "PL!-bp6-008-P | 小泉花陽 (ab#0)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "change_state", "card_type": "member_card", "count": 1, "exclude_self": true, "state_change": "active", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!-bp6-009-R | 矢澤にこ (ab#0)", "PL!-bp6-009-P | 矢澤にこ (ab#0)"], "effect": {"action": "modify_score", "activation_position": "center", "condition": {"blade_limit": 2, "blade_limit_operator": "==", "card_type": "member_card", "count": 2, "location": "stage", "position": "left_side", "position_compare": "right_side", "target": "self", "type": "location_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "activation_position": "center", "condition": {"blade_limit": 2, "blade_limit_operator": "==", "card_type": "member_card", "count": 2, "location": "stage", "position": "left_side", "position_compare": "right_side", "target": "self", "type": "location_condition"}, "conditional": true, "duration": "as_long_as", "operation": "add", "value": 1}
```

- ライブの合計スコアを+1する (x1)

```json
{"blade_limit": 2, "blade_limit_operator": "==", "card_type": "member_card", "count": 2, "location": "stage", "position": "left_side", "position_compare": "right_side", "target": "self", "type": "location_condition"}
```

- {center.png|センター}自分のステージの右サイドエリアと左サイドエリアに、元々持つ{icon_blade.png|ブレード}の数が2つのメンバーがいるかぎり (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp6-001-R | 高海千歌 (ab#0)", "PL!S-bp6-001-P | 高海千歌 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "cost_limit": 13, "cost_limit_operator": ">=", "count": 1, "position": "left_side", "position_compare": "right_side", "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "cost_limit": 13, "cost_limit_operator": ">=", "count": 1, "position": "left_side", "position_compare": "right_side", "state_change": "wait", "target": "opponent"}
```

- 控え室から登場している場合、相手のステージの右サイドエリアか左サイドエリアにいるコスト13以上のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp6-002-R＋ | 桜内梨子 (ab#0)", "PL!S-bp6-002-P | 桜内梨子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "locations": ["live_card_zone", "discard"], "target": "self", "type": "group_condition"}, "count": 1, "destination": "deck_top_or_bottom", "group_names": ["Aqours"], "optional": true}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!S-bp6-002-R＋ | 桜内梨子 (ab#1)", "PL!S-bp6-002-P | 桜内梨子 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "conditions": [{"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "count": 12, "heart_colors": ["heart02", "heart04", "heart05"], "operator": "=", "type": "comparison_condition"}], "location": "live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02", "heart04", "heart05"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!S-bp6-002-P＋ | 桜内梨子 (ab#0)", "PL!S-bp6-002-SEC | 桜内梨子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "locations": ["live_card_zone", "discard"], "target": "self", "type": "group_condition"}, "count": 1, "destination": "deck_top_or_bottom", "group_names": ["Aqours"], "optional": true}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!S-bp6-002-P＋ | 桜内梨子 (ab#1)", "PL!S-bp6-002-SEC | 桜内梨子 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "conditions": [{"card_type": "live_card", "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "count": 12, "heart_colors": ["heart02", "heart04", "heart05"], "operator": "=", "type": "comparison_condition"}], "location": "live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02", "heart04", "heart05"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 2, "cards": ["PL!S-bp6-003-R | 松浦果南 (ab#0)", "PL!S-bp6-003-P | 松浦果南 (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "discard", "exclude_self": true, "group_names": ["Aqours"], "source": "stage", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "cost_limit_operator": "=", "cost_offset": 2, "cost_reference": "previous_moved_card", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}], "conditional": true, "exclude_self": true, "group_names": ["Aqours"]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "discard", "exclude_self": true, "group_names": ["Aqours"], "source": "stage", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "cost_limit_operator": "=", "cost_offset": 2, "cost_reference": "previous_moved_card", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}], "conditional": true, "exclude_self": true, "group_names": ["Aqours"]}
```

- このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。そうした場合、自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "discard", "exclude_self": true, "group_names": ["Aqours"], "source": "stage", "target": "self"}
```

- このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。 (x1)

```json
{"action": "move_cards", "card_type": "member_card", "cost_limit_operator": "=", "cost_offset": 2, "cost_reference": "previous_moved_card", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}
```

- 自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp6-005-R | 渡辺 曜 (ab#0)", "PL!S-bp6-005-P | 渡辺 曜 (ab#0)"], "effect": {"action": "look_and_select", "heart_colors": ["heart02", "heart04", "heart05"], "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02", "heart04", "heart05"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart02", "heart04", "heart05"], "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02", "heart04", "heart05"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを2枚見る。その中から{heart_02.png|heart02}と{heart_04.png|heart04}と{heart_05.png|heart05}をすべて持つメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02", "heart04", "heart05"], "optional": true, "reveal": true}
```


```json
{"card_count": 2, "cards": ["PL!S-bp6-006-R | 津島善子 (ab#0)", "PL!S-bp6-006-P | 津島善子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "count": 3, "duration": "live_end", "resource": "blade"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "gain_resource", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "count": 3, "duration": "live_end", "resource": "blade"}]}
```

- カードを2枚引く。その後、控え室から登場している場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "count": 3, "duration": "live_end", "resource": "blade"}
```

- 控え室から登場している場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp6-007-R | 国木田花丸 (ab#0)", "PL!S-bp6-007-P | 国木田花丸 (ab#0)"], "cost": {"options": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "choice_condition"}, "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "card_type": "member_card", "condition": {"card_type": "live_card", "conditions": [{"card_type": "live_card", "location": "success_live_card_zone", "negation": true, "target": "self", "type": "location_condition"}, {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "count": 2, "duration": "live_end", "group_names": ["Aqours"], "max": true, "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"options": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "choice_condition"}
```

- {icon_energy.png|E}{icon_energy.png|E}支払うか手札を2枚控え室に置いてもよい (x1)

```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "card_type": "member_card", "condition": {"card_type": "live_card", "conditions": [{"card_type": "live_card", "location": "success_live_card_zone", "negation": true, "target": "self", "type": "location_condition"}, {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "count": 2, "duration": "live_end", "group_names": ["Aqours"], "max": true, "target": "self"}
```

- 自分の成功ライブカード置き場にカードがなく、かつ相手の成功ライブカード置き場にカードが2枚以上ある場合、ライブ終了時まで、自分のステージにいる『Aqours』のメンバー2人までは「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"card_type": "live_card", "conditions": [{"card_type": "live_card", "location": "success_live_card_zone", "negation": true, "target": "self", "type": "location_condition"}, {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}
```

- 自分の成功ライブカード置き場にカードがなく、かつ相手の成功ライブカード置き場にカードが2枚以上ある場合 (x1)

```json
{"card_type": "live_card", "location": "success_live_card_zone", "negation": true, "target": "self", "type": "location_condition"}
```

- 自分の成功ライブカード置き場にカードがなく、 (x1)

```json
{"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "opponent", "type": "card_count_condition"}
```

- 相手の成功ライブカード置き場にカードが2枚以上ある場合 (x1)

```json
{"card_count": 2, "cards": ["PL!S-bp6-008-R | 小原鞠莉 (ab#0)", "PL!S-bp6-008-P | 小原鞠莉 (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 17, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 17, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "group_names": ["Aqours"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト17以下の『Aqours』のメンバーカードを1枚、このメンバーがいたエリアに登場させる (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-002-R | 村野さやか (ab#0)", "PL!HS-bp6-002-P | 村野さやか (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "exclude_self": true, "location": "stage", "negation": true, "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-003-R | 大沢瑠璃乃 (ab#0)", "PL!HS-bp6-003-P | 大沢瑠璃乃 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["みらくらぱーく！"], "optional": true, "state_change": "active", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "source": "discard", "target": "self"}], "conditional": true, "group_names": ["みらくらぱーく！"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["みらくらぱーく！"], "optional": true, "state_change": "active", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "source": "discard", "target": "self"}], "conditional": true, "group_names": ["みらくらぱーく！"]}
```

- 自分のステージにいるウェイト状態の『みらくらぱーく！』のメンバー1人をアクティブにしてもよい。そうした場合、自分の控え室から『みらくらぱーく！』のライブカードを1枚手札に加える (x1)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["みらくらぱーく！"], "optional": true, "state_change": "active", "target": "self"}
```

- 自分のステージにいるウェイト状態の『みらくらぱーく！』のメンバー1人をアクティブにしてもよい。 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "source": "discard", "target": "self"}
```

- 自分の控え室から『みらくらぱーく！』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-003-R | 大沢瑠璃乃 (ab#1)", "PL!HS-bp6-003-P | 大沢瑠璃乃 (ab#1)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"], "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["みらくらぱーく！"], "heart_colors": ["heart01"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいる『みらくらぱーく！』のメンバー1人は、{heart_01.png|heart01}を得る (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-004-R | 百生 吟子 (ab#0)", "PL!HS-bp6-004-P | 百生 吟子 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"card_count": 2, "cards": ["PL!HS-bp6-004-R | 百生 吟子 (ab#1)", "PL!HS-bp6-004-P | 百生 吟子 (ab#1)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "location": "discard", "type": "location_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}], "duration": "live_end"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "location": "discard", "type": "location_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}], "duration": "live_end"}
```

- {icon_blade.png|ブレード}を得る。これにより「百生吟子」のメンバーカードを控え室に置いた場合、さらに{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "location": "discard", "type": "location_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}
```

- これにより「百生吟子」のメンバーカードを控え室に置いた場合、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "location": "discard", "type": "location_condition"}
```

- これにより「百生吟子」のメンバーカードを控え室に置いた場合 (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-007-R | セラス 柳田 リリエンフェルト (ab#0)", "PL!HS-bp6-007-P | セラス 柳田 リリエンフェルト (ab#0)"], "effect": {"action": "opponent_action", "action_by": "opponent", "condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "group_names": ["EdelNote"], "opponent_action": {"action": "change_state", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "opponent_action", "action_by": "opponent", "condition": {"appearance": true, "location": "stage", "target": "self", "type": "appearance_condition"}, "group_names": ["EdelNote"], "opponent_action": {"action": "change_state", "card_type": "member_card", "count": 1, "state": "active", "state_change": "wait"}}
```

- 自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-008-R | 桂城 泉 (ab#0)", "PL!HS-bp6-008-P | 桂城 泉 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["蓮ノ空"], "state_change": "wait"}, {"action": "move_cards", "card_type": "live_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}], "group_names": ["蓮ノ空"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["蓮ノ空"], "state_change": "wait"}, {"action": "move_cards", "card_type": "live_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}], "group_names": ["蓮ノ空"]}
```

- このメンバーをウェイトにする。その後、自分の控え室からスコア4以下の『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "group_names": ["蓮ノ空"], "state_change": "wait"}
```

- このメンバーをウェイトにする (x1)

```json
{"action": "move_cards", "card_type": "live_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室からスコア4以下の『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 2, "cards": ["PL!HS-bp6-008-R | 桂城 泉 (ab#1)", "PL!HS-bp6-008-P | 桂城 泉 (ab#1)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"target": "self", "temporal": "during_live", "type": "temporal_condition"}, "count": 1, "state_change": "active"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"target": "self", "temporal": "during_live", "type": "temporal_condition"}, "count": 1, "state_change": "active"}
```

- 自分のライブ中のカードにスコア2以下のライブカードがある場合、このメンバーをアクティブにする (x1)

```json
{"card_count": 2, "cards": ["PL!SP-sd2-002-P | 唐 可可 (ab#0)", "PL!SP-sd2-002-SD2 | 唐 可可 (ab#0)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "position_change", "card_type": "member_card"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 2, "cards": ["PL!SP-sd2-023-P | 始まりは君の空 (ab#0)", "PL!SP-sd2-023-SD2 | 始まりは君の空 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "operation": "add", "self_target": true, "value": 5}, {"action": "modify_required_hearts", "count": 12, "heart_colors": ["heart00", "heart02", "heart03", "heart06"], "operation": "set"}], "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "heart_colors": ["heart02", "heart03", "heart06", "heart00"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "operation": "add", "self_target": true, "value": 5}, {"action": "modify_required_hearts", "count": 12, "heart_colors": ["heart00", "heart02", "heart03", "heart06"], "operation": "set"}], "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "heart_colors": ["heart02", "heart03", "heart06", "heart00"]}
```

- 自分の成功ライブカード置き場にカードが2枚以上ある場合、このカードのスコアを+5し、必要ハートは{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}{heart_03.png|heart03}{heart_03.png|heart03}{heart_03.png|heart03}{heart_06.png|heart06}{heart_06.png|heart06}{heart_06.png|heart06}{heart_00.png|heart0}{heart_00.png|heart0}{heart_00.png|heart0}になる (x1)

```json
{"action": "modify_score", "operation": "add", "self_target": true, "value": 5}
```

- このカードのスコアを+5し (x1)

```json
{"action": "modify_required_hearts", "count": 12, "heart_colors": ["heart00", "heart02", "heart03", "heart06"], "operation": "set"}
```

- 必要ハートは{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}{heart_03.png|heart03}{heart_03.png|heart03}{heart_03.png|heart03}{heart_06.png|heart06}{heart_06.png|heart06}{heart_06.png|heart06}{heart_00.png|heart0}{heart_00.png|heart0}{heart_00.png|heart0}になる (x1)

```json
{"card_count": 2, "cards": ["PL!SP-sd2-025-P | Aspire (ab#0)", "PL!SP-sd2-025-SD2 | Aspire (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end", "group_names": ["Liella!"]}, {"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "blade"}], "all": true, "duration": "live_end", "group_names": ["Liella!"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end", "group_names": ["Liella!"]}, {"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "blade"}], "all": true, "duration": "live_end", "group_names": ["Liella!"]}
```

- 自分のステージにいる、このターン中にエリアを移動したすべての『Liella!』のメンバーは、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end", "group_names": ["Liella!"]}
```

- このターン中にエリアを移動したすべての『Liella!』のメンバーは (x1)

```json
{"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "blade"}
```

- {icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-001-SD | 高坂 穂乃果 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の成功ライブカード置き場にカードが2枚以上ある場合、自分の控え室からライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-001-SD | 高坂 穂乃果 (ab#1)"], "effect": {"action": "gain_resource", "count": 1, "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "count": 1, "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "blade", "target": "self"}
```

- 自分の成功ライブカード置き場にあるカード1枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-003-SD | 南 ことり (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト4以下の『μ's』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-003-SD | 南 ことり (ab#1)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart"}], "heart_colors": ["heart01", "heart03", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!-sd1-004-SD | 園田海未 (ab#0)"], "effect": {"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["μ's"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『μ's』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["μ's"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!-sd1-006-SD | 西木野 真姫 (ab#0)"], "cost": {"card_type": "live_card", "count": 1, "optional": true, "source": "hand", "type": "reveal", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "success_live_zone", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "destination": "success_live_zone", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}], "conditional": true}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "success_live_zone", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "destination": "success_live_zone", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}], "conditional": true}
```

- 自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、これにより公開したカードを自分の成功ライブカード置き場に置く (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "source": "success_live_zone", "target": "self"}
```

- 自分の成功ライブカード置き場にあるカードを1枚手札に加える。 (x1)

```json
{"action": "move_cards", "card_type": "live_card", "destination": "success_live_zone", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}
```

- これにより公開したカードを自分の成功ライブカード置き場に置く (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-007-SD | 東條 希 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "draw_card", "condition": {"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 5, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "draw_card", "condition": {"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}
```

- 自分のデッキの上からカードを5枚控え室に置く。それらの中にライブカードがある場合、カードを1枚引く (x1)

```json
{"action": "draw_card", "condition": {"card_type": "live_card", "count": 1, "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- それらの中にライブカードがある場合、カードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-008-SD | 小泉 花陽 (ab#0)"], "cost": {"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "card", "count": 10, "destination": "discard", "source": "deck_top", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 1, "cards": ["PL!-sd1-009-SD | 矢澤 にこ (ab#0)"], "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"count": 25, "group_names": ["μ's"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "duration": "live_end", "group_names": ["μ's"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "condition": {"count": 25, "group_names": ["μ's"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "duration": "live_end", "group_names": ["μ's"]}
```

- 自分の控え室に『μ's』のカードが25枚以上ある場合、ライブ終了時まで、「{jyouji.png|常時}ライブの合計スコアを+1する。」を得る (x1)

```json
{"count": 25, "group_names": ["μ's"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室に『μ's』のカードが25枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-sd1-019-SD | START:DASH!! (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 1, "cards": ["PL!-sd1-022-SD | 僕らは今のなかで (ab#0)"], "effect": {"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart00"], "location": "success_live_zone", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart00"], "location": "success_live_zone", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self"}
```

- 自分の成功ライブカード置き場にあるカード1枚につき、このカードを成功させるための必要ハートは{heart_00.png|heart0}{heart_00.png|heart0}少なくなる (x1)

```json
{"card_count": 1, "cards": ["PL!-PR-003-PR | 南ことり (ab#0)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "need_heart_color": "heart03", "need_heart_operator": ">=", "need_heart_total": 3, "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart03"], "need_heart_color": "heart03", "need_heart_operator": ">=", "need_heart_total": 3, "source": "discard", "target": "self"}
```

- 自分の控え室から必要ハートに{heart_03.png|heart03}を3以上含むライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-PR-004-PR | 園田海未 (ab#0)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart01"], "need_heart_color": "heart01", "need_heart_operator": ">=", "need_heart_total": 3, "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "heart_colors": ["heart01"], "need_heart_color": "heart01", "need_heart_operator": ">=", "need_heart_total": 3, "source": "discard", "target": "self"}
```

- 自分の控え室から必要ハートに{heart_01.png|heart01}を3以上含むライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-PR-014-PR | 園田海未 (ab#0)"], "effect": {"action": "conditional_on_result", "followup_action": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "primary_effect": {"action": "reveal", "blind": true, "count": 3, "source": "hand", "target": "opponent"}, "result_condition": {"card_type": "live_card", "location": "revealed_cards", "negation": true, "type": "location_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "conditional_on_result", "followup_action": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "primary_effect": {"action": "reveal", "blind": true, "count": 3, "source": "hand", "target": "opponent"}, "result_condition": {"card_type": "live_card", "location": "revealed_cards", "negation": true, "type": "location_condition"}}
```

- 相手の手札を、自分は見ないで3枚選び公開する。これにより公開されたカードの中にライブカードがない場合、カードを1枚引く (x1)

```json
{"action": "reveal", "blind": true, "count": 3, "source": "hand", "target": "opponent"}
```

- 相手の手札を、自分は見ないで3枚選び公開する。 (x1)

```json
{"card_count": 1, "cards": ["PL!-PR-017-PR | 矢澤にこ (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, {"action": "change_state", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 9, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "state_change": "active"}], "group_names": ["μ's"]}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, {"action": "change_state", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 9, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "state_change": "active"}], "group_names": ["μ's"]}
```

- 自分の控え室から『μ's』のライブカードを1枚手札に加える。自分の成功ライブカード置き場にあるカードのスコアの合計が9以上の場合、エネルギーを2枚アクティブにする (x1)

```json
{"action": "change_state", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 9, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "state_change": "active"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が9以上の場合、エネルギーを2枚アクティブにする (x1)

```json
{"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 9, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が9以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-PR-041-PR | 黒澤ルビィ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "source": "discard", "target": "self"}, {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "target": "self"}], "conditional": true, "target": "self"}], "conditional": true}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!S-PR-042-PR | 小原鞠莉 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart04"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02", "heart04"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart04"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02", "heart04"], "resource": "heart"}
```

- {heart_02.png|heart02}{heart_04.png|heart04}を得る (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart04"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}
```

- 自分と相手のステージにメンバーが合計6人いるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!N-PR-022-PR | エマ・ヴェルデ (ab#0)"], "effect": {"action": "choice", "all": true, "choice_maker": "opponent", "choice_type": "answer_based", "options": [{"action": "gain_resource", "all": true, "answers": ["お願いします"], "card_type": "member_card", "duration": "live_end", "resource": "blade", "target": "opponent"}, {"action": "do_nothing", "answers": ["それ以外"]}], "parenthetical": ["相手を傷つけないよう、やさしく、愛をこめて魔法のパンチをすること。"], "question": "直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "choice", "all": true, "choice_maker": "opponent", "choice_type": "answer_based", "options": [{"action": "gain_resource", "all": true, "answers": ["お願いします"], "card_type": "member_card", "duration": "live_end", "resource": "blade", "target": "opponent"}, {"action": "do_nothing", "answers": ["それ以外"]}], "parenthetical": ["相手を傷つけないよう、やさしく、愛をこめて魔法のパンチをすること。"], "question": "直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい"}
```

- 直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。
回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終了時まで、相手のステージにいるすべてのメンバーは、{icon_blade.png|ブレード}を得る。
回答がそれ以外の場合、何もしない (x1)

```json
{"action": "gain_resource", "all": true, "answers": ["お願いします"], "card_type": "member_card", "duration": "live_end", "resource": "blade", "target": "opponent"}
```

- 自分は相手にエマパンチする。ライブ終了時まで、相手のステージにいるすべてのメンバーは、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "do_nothing", "answers": ["それ以外"]}
```

- 何もしない (x1)

```json
{"card_count": 1, "cards": ["PL!N-PR-025-PR | 優木せつ菜 (ab#0)"], "effect": {"action": "draw_card", "condition": {"baton_touch_trigger": true, "exclude_self": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "exclude_self": true, "source": "deck"}, "is_null": false, "triggers": "自動", "use_limit": 2}
```


```json
{"action": "draw_card", "condition": {"baton_touch_trigger": true, "exclude_self": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "exclude_self": true, "source": "deck"}
```

- 自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く (x1)

```json
{"baton_touch_trigger": true, "exclude_self": true, "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき (x1)

```json
{"card_count": 1, "cards": ["PL!N-PR-026-PR | 天王寺璃奈 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "destination": "under_member", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "count": 1, "destination": "under_member", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト9以下の『虹ヶ咲』のメンバーカード1枚をこのメンバーの下に置く (x1)

```json
{"card_count": 1, "cards": ["PL!N-PR-026-PR | 天王寺璃奈 (ab#1)"], "effect": {"action": "gain_ability_from_source", "all": true, "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "group_names": ["虹ヶ咲"], "source_location": "under_member", "trigger_filter": ["ライブ成功時"]}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_ability_from_source", "all": true, "card_type": "member_card", "cost_limit": 9, "cost_limit_operator": "<=", "group_names": ["虹ヶ咲"], "source_location": "under_member", "trigger_filter": ["ライブ成功時"]}
```

- このメンバーは、このメンバーの下に置かれているコスト9以下の『虹ヶ咲』のメンバーカードが持つ{live_success.png|ライブ成功時}能力をすべて得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-PR-027-PR | 朝香果林 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart05"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02", "heart05"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart05"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02", "heart05"], "resource": "heart"}
```

- {heart_02.png|heart02}{heart_05.png|heart05}を得る (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart05"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}
```

- 自分と相手のステージにメンバーが合計6人いるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!SP-PR-018-PR | 澁谷かのん (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"count": 7, "group_names": ["Liella!"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "energy_zone", "group_names": ["Liella!"], "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"count": 7, "group_names": ["Liella!"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "energy_zone", "group_names": ["Liella!"], "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- エールにより公開された自分のカードの中に『Liella!』のカードが7枚以上ある場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"count": 7, "group_names": ["Liella!"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- エールにより公開された自分のカードの中に『Liella!』のカードが7枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-PR-021-PR | 澁谷かのん (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "condition": {"aggregate": "total", "card_type": "member_card", "count": 5, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}, "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "change_state", "card_type": "member_card", "condition": {"aggregate": "total", "card_type": "member_card", "count": 5, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}, "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}
```

- 自分のステージにいるメンバーが持つハートが合計5つ以上ある場合、相手のステージにいるコスト2以下のメンバー1人をウェイトにする (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 5, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のステージにいるメンバーが持つハートが合計5つ以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-PR-022-PR | 若菜四季 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart03"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02", "heart03"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart03"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart02", "heart03"], "resource": "heart"}
```

- {heart_02.png|heart02}{heart_03.png|heart03}を得る (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 6, "heart_colors": ["heart02", "heart03"], "location": "stage", "scope": "both", "target": "both", "type": "location_condition"}
```

- 自分と相手のステージにメンバーが合計6人いるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!HS-PR-016-PR | 日野下花帆 (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "same_unit_name": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade"}, {"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}], "count": 4, "duration": "live_end", "heart_colors": ["heart04"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade"}, {"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}], "count": 4, "duration": "live_end", "heart_colors": ["heart04"]}
```

- {heart_04.png|heart04}{heart_04.png|heart04}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart04"], "resource": "blade"}
```


```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}
```


```json
{"card_count": 1, "cards": ["PL!HS-PR-017-PR | 村野さやか (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "same_unit_name": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}], "count": 4, "duration": "live_end", "heart_colors": ["heart05"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}, {"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}], "count": 4, "duration": "live_end", "heart_colors": ["heart05"]}
```

- {heart_05.png|heart05}{heart_05.png|heart05}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade"}
```


```json
{"action": "gain_resource", "count": 2, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}
```


```json
{"card_count": 1, "cards": ["PL!HS-PR-028-PR | Echoes Beyond (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "member_card", "count": 1, "location": "stage", "operator": ">", "original_value": true, "type": "location_condition"}, "count": 1, "destination": "hand", "original_value": true, "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"card_type": "member_card", "count": 1, "location": "stage", "operator": ">", "original_value": true, "type": "location_condition"}, "count": 1, "destination": "hand", "original_value": true, "source": "deck"}
```

- 自分のステージに、元々持つハートの数より多い数のハートを持つメンバーがいる場合、カードを1枚引く (x1)

```json
{"card_type": "member_card", "count": 1, "location": "stage", "operator": ">", "original_value": true, "type": "location_condition"}
```

- 自分のステージに、元々持つハートの数より多い数のハートを持つメンバーがいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-PR-029-PR | 大沢瑠璃乃 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["LL-PR-004-PR | 愛♡スクリ～ム！ (ab#0)"], "effect": {"action": "choice", "choice_maker": "opponent", "choice_type": "answer_based", "options": [{"action": "move_cards", "answers": ["チョコミント", "ストロベリーフレイバー", "クッキー＆クリーム"], "card_type": "card", "count": 1, "destination": "discard", "source": "hand", "target": "both"}, {"action": "draw_card", "answers": ["あなた"], "count": 1, "destination": "hand", "source": "deck", "target": "both"}, {"action": "gain_resource", "answers": ["それ以外"], "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target": "both"}], "question": "相手に何が好き？と聞く"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "choice", "choice_maker": "opponent", "choice_type": "answer_based", "options": [{"action": "move_cards", "answers": ["チョコミント", "ストロベリーフレイバー", "クッキー＆クリーム"], "card_type": "card", "count": 1, "destination": "discard", "source": "hand", "target": "both"}, {"action": "draw_card", "answers": ["あなた"], "count": 1, "destination": "hand", "source": "deck", "target": "both"}, {"action": "gain_resource", "answers": ["それ以外"], "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target": "both"}], "question": "相手に何が好き？と聞く"}
```

- 相手に何が好き？と聞く。
回答がチョコミントかストロベリーフレイバーかクッキー＆クリームの場合、自分と相手は手札を1枚控え室に置く。
回答があなたの場合、自分と相手はカードを1枚引く。
回答がそれ以外の場合、ライブ終了時まで、自分と相手のステージにいるメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "answers": ["チョコミント", "ストロベリーフレイバー", "クッキー＆クリーム"], "card_type": "card", "count": 1, "destination": "discard", "source": "hand", "target": "both"}
```

- 自分と相手は手札を1枚控え室に置く (x1)

```json
{"action": "draw_card", "answers": ["あなた"], "count": 1, "destination": "hand", "source": "deck", "target": "both"}
```

- 自分と相手はカードを1枚引く (x1)

```json
{"action": "gain_resource", "answers": ["それ以外"], "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target": "both"}
```

- 自分と相手のステージにいるメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["LL-bp1-001-R＋ | 上原歩夢&澁谷かのん&日野下花帆 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["LL-bp1-001-R＋ | 上原歩夢&澁谷かのん&日野下花帆 (ab#1)"], "cost": {"any_number": true, "characters": ["上原歩夢", "澁谷かのん", "日野下花帆"], "count": 3, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"ability_gain": "ライブの合計スコアを+3する。", "action": "gain_ability", "duration": "live_end", "parenthetical": ["手札のこのカードもこの効果で控え室に置ける。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"any_number": true, "characters": ["上原歩夢", "澁谷かのん", "日野下花帆"], "count": 3, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の「上原歩夢」と「澁谷かのん」と「日野下花帆」を、好きな組み合わせで合計3枚、控え室に置いてもよい (x1)

```json
{"ability_gain": "ライブの合計スコアを+3する。", "action": "gain_ability", "duration": "live_end", "parenthetical": ["手札のこのカードもこの効果で控え室に置ける。"]}
```

- 「{jyouji.png|常時}ライブの合計スコアを+3する。」を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp1-026-L | Poppin' Up! (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["虹ヶ咲"], "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["虹ヶ咲"], "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "revealed_cards", "target": "self"}
```

- ライブの合計スコアが相手より高い場合、エールにより公開された自分のカードの中から、『虹ヶ咲』のカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp1-027-L | Solitude Rain (ab#0)"], "effect": {"action": "modify_score", "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart04", "heart05", "heart02", "heart03", "heart06"], "location": "stage", "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "member", "self_target": true, "target": "self", "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart04", "heart05", "heart02", "heart03", "heart06"], "location": "stage", "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "member", "self_target": true, "target": "self", "value": 1}
```

- 自分のステージにいる『虹ヶ咲』のメンバーが持つ{heart_01.png|heart01}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_06.png|heart06}のうち1色につき、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp1-028-L | Butterfly (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["虹ヶ咲"], "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["虹ヶ咲"], "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}
```

- 自分のステージに『虹ヶ咲』のメンバーがいる場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp1-029-L | Eutopia (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 3, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"count": 3, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 2}
```

- 自分のライブ中のカードが3枚以上ある場合、このカードのスコアを+2する (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp1-023-L | START!! True dreams (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "count": 1, "destination": "energy_zone", "parenthetical": ["エールで出た{{icon_score.png|スコア}}1つにつき、成功したライブのスコアの合計に1を加算する。"], "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "operator": ">", "type": "comparison_condition"}, "count": 1, "destination": "energy_zone", "parenthetical": ["エールで出た{{icon_score.png|スコア}}1つにつき、成功したライブのスコアの合計に1を加算する。"], "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp1-024-L | Tiny Stars (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "card_type": "member_card", "characters": ["澁谷かのん"], "count": 1, "duration": "live_end", "heart_colors": ["heart05", "heart01"], "resource": "blade", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "characters": ["澁谷かのん"], "count": 1, "duration": "live_end", "heart_color": "heart05", "heart_colors": ["heart05", "heart01"], "resource": "heart", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "characters": ["唐可可"], "count": 1, "duration": "live_end", "heart_colors": ["heart05", "heart01"], "resource": "blade", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "characters": ["唐可可"], "count": 1, "duration": "live_end", "heart_color": "heart01", "heart_colors": ["heart05", "heart01"], "resource": "heart", "target": "self"}], "character_effects": [{"character": "澁谷かのん", "count": 1, "resources": "{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}"}, {"character": "唐可可", "count": 1, "resources": "{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}"}], "duration": "live_end", "heart_colors": ["heart05", "heart01"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "card_type": "member_card", "characters": ["澁谷かのん"], "count": 1, "duration": "live_end", "heart_colors": ["heart05", "heart01"], "resource": "blade", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "characters": ["澁谷かのん"], "count": 1, "duration": "live_end", "heart_color": "heart05", "heart_colors": ["heart05", "heart01"], "resource": "heart", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "characters": ["唐可可"], "count": 1, "duration": "live_end", "heart_colors": ["heart05", "heart01"], "resource": "blade", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "characters": ["唐可可"], "count": 1, "duration": "live_end", "heart_color": "heart01", "heart_colors": ["heart05", "heart01"], "resource": "heart", "target": "self"}], "character_effects": [{"character": "澁谷かのん", "count": 1, "resources": "{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}"}, {"character": "唐可可", "count": 1, "resources": "{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}"}], "duration": "live_end", "heart_colors": ["heart05", "heart01"]}
```

- 自分のステージにいる「澁谷かのん」1人は{heart_05.png|heart05}{icon_blade.png|ブレード}を、「唐可可」1人は{heart_01.png|heart01}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "characters": ["澁谷かのん"], "count": 1, "duration": "live_end", "heart_colors": ["heart05", "heart01"], "resource": "blade", "target": "self"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "characters": ["澁谷かのん"], "count": 1, "duration": "live_end", "heart_color": "heart05", "heart_colors": ["heart05", "heart01"], "resource": "heart", "target": "self"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "characters": ["唐可可"], "count": 1, "duration": "live_end", "heart_colors": ["heart05", "heart01"], "resource": "blade", "target": "self"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "characters": ["唐可可"], "count": 1, "duration": "live_end", "heart_color": "heart01", "heart_colors": ["heart05", "heart01"], "resource": "heart", "target": "self"}
```


```json
{"character": "澁谷かのん", "count": 1, "resources": "{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}"}
```


```json
{"character": "唐可可", "count": 1, "resources": "{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}"}
```


```json
{"card_count": 1, "cards": ["PL!SP-bp1-024-L | Tiny Stars (ab#1)"], "effect": {"action": "draw_card", "condition": {"characters": ["唐可可"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"characters": ["唐可可"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "deck"}
```

- 自分のステージに「澁谷かのん」と「唐可可」がいる場合、カードを1枚引く (x1)

```json
{"characters": ["唐可可"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「澁谷かのん」と「唐可可」がいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp1-026-L | 未来予報ハレルヤ！ (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"count": 5, "distinct": "card_name", "group_names": ["Liella!"], "heart_colors": ["heart02", "heart03", "heart06"], "locations": ["discard", "stage"], "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 2, "distinct": "card_name", "group_names": ["Liella!"], "heart_colors": ["heart02", "heart03", "heart06"], "operation": "set", "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "replace_all": true, "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"count": 5, "distinct": "card_name", "group_names": ["Liella!"], "heart_colors": ["heart02", "heart03", "heart06"], "locations": ["discard", "stage"], "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 2, "distinct": "card_name", "group_names": ["Liella!"], "heart_colors": ["heart02", "heart03", "heart06"], "operation": "set", "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "replace_all": true, "self_target": true}
```

- 自分の、ステージと控え室に名前の異なる『Liella!』のメンバーが5人以上いる場合、このカードを使用するためのコストは{heart_02.png|heart02}{heart_02.png|heart02}{heart_03.png|heart03}{heart_03.png|heart03}{heart_06.png|heart06}{heart_06.png|heart06}になる (x1)

```json
{"count": 5, "distinct": "card_name", "group_names": ["Liella!"], "heart_colors": ["heart02", "heart03", "heart06"], "locations": ["discard", "stage"], "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分の、ステージと控え室に名前の異なる『Liella!』のメンバーが5人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp1-027-L | Sing！Shine！Smile！ (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 12, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"count": 12, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}
```

- 自分のエネルギーが12枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp1-021-L | Holiday∞Holiday (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp1-022-L | AWOKE (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 10, "group_names": ["蓮ノ空"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["蓮ノ空"], "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 10, "group_names": ["蓮ノ空"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["蓮ノ空"], "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}
```

- エールにより公開された自分のカードの中に『蓮ノ空』のメンバーカードが10枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "count": 10, "group_names": ["蓮ノ空"], "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- エールにより公開された自分のカードの中に『蓮ノ空』のメンバーカードが10枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp1-023-L | ド！ド！ド！ (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "type": "comparison_condition"}, {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 1, "destination": "energy_zone", "group_names": ["蓮ノ空"], "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "type": "comparison_condition"}, {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 1, "destination": "energy_zone", "group_names": ["蓮ノ空"], "parenthetical": ["必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。"], "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- ライブの合計スコアが相手より高く、かつ自分のステージに『蓮ノ空』のメンバーがいる場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"aggregate": "total", "card_type": "member_card", "conditions": [{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "type": "comparison_condition"}, {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- ライブの合計スコアが相手より高く、かつ自分のステージに『蓮ノ空』のメンバーがいる場合 (x1)

```json
{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "type": "comparison_condition"}
```

- ライブの合計スコアが相手より高く、 (x1)

```json
{"card_count": 1, "cards": ["PL!N-sd1-001-SD | 上原歩夢 (ab#0)"], "effect": {"action": "look_and_select", "group_names": ["虹ヶ咲"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "max": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["虹ヶ咲"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "max": true, "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『虹ヶ咲』のライブカードを1枚まで公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["虹ヶ咲"], "max": true, "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!N-sd1-001-SD | 上原歩夢 (ab#1)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self"}
```

- 自分のステージにいるほかの『虹ヶ咲』のメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-sd1-004-SD | 朝香果林 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!N-sd1-007-SD | 優木せつ菜 (ab#0)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 1, "cards": ["PL!N-sd1-010-SD | 三船栞子 (ab#1)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!N-sd1-028-SD | Dream with You (ab#0)"], "effect": {"action": "modify_score", "condition": {"aggregate": "total", "card_type": "member_card", "count": 10, "location": "stage", "operator": ">=", "target": "self", "type": "location_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"aggregate": "total", "card_type": "member_card", "count": 10, "location": "stage", "operator": ">=", "target": "self", "type": "location_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}
```

- 自分のステージにいるメンバーが持つ{icon_blade.png|ブレード}の合計が10以上の場合、このカードのスコアを+1する (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 10, "location": "stage", "operator": ">=", "target": "self", "type": "location_condition"}
```

- 自分のステージにいるメンバーが持つ{icon_blade.png|ブレード}の合計が10以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd1-001-SD | 澁谷かのん (ab#0)"], "effect": {"action": "draw_card", "count": 1, "destination": "hand", "per_unit": true, "per_unit_count": 6, "per_unit_type": "枚", "source": "deck", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "count": 1, "destination": "hand", "per_unit": true, "per_unit_count": 6, "per_unit_type": "枚", "source": "deck", "target": "self"}
```

- 自分のエネルギー6枚につき、カードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd1-002-SD | 唐 可可 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "group_names": ["Liella!"], "optional": true, "parenthetical": ["この効果で既にメンバーがいるエリアにも登場できる。ただし、このターンにステージに登場したメンバーがいるエリアには登場できない。"], "source": "hand"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "stage", "group_names": ["Liella!"], "optional": true, "parenthetical": ["この効果で既にメンバーがいるエリアにも登場できる。ただし、このターンにステージに登場したメンバーがいるエリアには登場できない。"], "source": "hand"}
```

- 手札からコスト4以下の『Liella!』のメンバーカードを1枚ステージに登場させてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd1-004-SD | 平安名すみれ (ab#0)"], "effect": {"ability_gain": "ライブの合計スコアを+1する。", "action": "gain_ability", "duration": "live_end"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!SP-sd1-007-SD | 米女メイ (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}
```

- 自分の控え室から『Liella!』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd1-009-SD | 鬼塚夏美 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "look_and_select", "condition": {"count": 9, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "condition": {"count": 9, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}
```

- 自分のエネルギーが9枚以上ある場合、自分のデッキの上からカードを5枚見る。その中から1枚を手札に加え、残りを控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd1-026-SD | 私のSymphony 〜澁谷かのんVer.〜 (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 9, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"count": 9, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "parenthetical": ["エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。"], "self_target": true, "value": 1}
```

- 自分のエネルギーが9枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!SP-pb1-015-N | 平安名すみれ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["CatChu!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["CatChu!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["CatChu!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["CatChu!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『CatChu!』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["CatChu!"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!SP-pb1-016-N | 葉月 恋 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["KALEIDOSCORE"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["KALEIDOSCORE"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["KALEIDOSCORE"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["KALEIDOSCORE"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『KALEIDOSCORE』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["KALEIDOSCORE"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!SP-pb1-017-N | 桜小路きな子 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["5yncri5e!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["5yncri5e!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["5yncri5e!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["5yncri5e!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『5yncri5e!』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["5yncri5e!"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!SP-pb1-020-N | 鬼塚夏美 (ab#0)"], "effect": {"action": "draw_card", "count": 1, "destination": "hand", "parenthetical": ["対戦相手のカードの効果でも発動する。"], "source": "deck", "trigger_condition": {"movement": "moves", "type": "movement_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "draw_card", "count": 1, "destination": "hand", "parenthetical": ["対戦相手のカードの効果でも発動する。"], "source": "deck", "trigger_condition": {"movement": "moves", "type": "movement_condition"}, "trigger_type": "each_time"}
```

- このメンバーがエリアを移動するたび、カードを1枚引く (x1)

```json
{"movement": "moves", "type": "movement_condition"}
```

- このメンバーがエリアを移動する (x1)

```json
{"card_count": 1, "cards": ["PL!SP-pb1-023-L | ディストーション (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "change_state", "card_type": "energy_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["CatChu!"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 6, "distinct": "card_name", "max": true, "state_change": "active"}, {"action": "modify_score", "condition": {"all": true, "resource_type": "energy", "state": "active", "type": "state_condition"}, "group_names": ["CatChu!"], "operation": "add", "self_target": true, "value": 1}], "distinct": "card_name", "group_names": ["CatChu!"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "change_state", "card_type": "energy_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["CatChu!"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 6, "distinct": "card_name", "max": true, "state_change": "active"}, {"action": "modify_score", "condition": {"all": true, "resource_type": "energy", "state": "active", "type": "state_condition"}, "group_names": ["CatChu!"], "operation": "add", "self_target": true, "value": 1}], "distinct": "card_name", "group_names": ["CatChu!"]}
```

- 自分のステージに名前の異なる『CatChu!』のメンバーが2人以上いる場合、エネルギーを6枚までアクティブにする。その後、自分のエネルギーがすべてアクティブ状態の場合、このカードのスコアを+1する (x1)

```json
{"action": "change_state", "card_type": "energy_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["CatChu!"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 6, "distinct": "card_name", "max": true, "state_change": "active"}
```

- 自分のステージに名前の異なる『CatChu!』のメンバーが2人以上いる場合、エネルギーを6枚までアクティブにする (x1)

```json
{"count": 2, "distinct": "card_name", "group_names": ["CatChu!"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージに名前の異なる『CatChu!』のメンバーが2人以上いる場合 (x1)

```json
{"action": "modify_score", "condition": {"all": true, "resource_type": "energy", "state": "active", "type": "state_condition"}, "group_names": ["CatChu!"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のエネルギーがすべてアクティブ状態の場合、このカードのスコアを+1する (x1)

```json
{"all": true, "resource_type": "energy", "state": "active", "type": "state_condition"}
```

- 自分のエネルギーがすべてアクティブ状態の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-pb1-024-L | ニュートラル (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 2, "distinct": "card_name", "group_names": ["KALEIDOSCORE"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "distinct": "card_name", "group_names": ["KALEIDOSCORE"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"count": 2, "distinct": "card_name", "group_names": ["KALEIDOSCORE"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "distinct": "card_name", "group_names": ["KALEIDOSCORE"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージに名前の異なる『KALEIDOSCORE』のメンバーが2人以上いる場合、このカードのスコアを+1する (x1)

```json
{"count": 2, "distinct": "card_name", "group_names": ["KALEIDOSCORE"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージに名前の異なる『KALEIDOSCORE』のメンバーが2人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-pb1-025-L | Jellyfish (ab#0)"], "effect": {"action": "modify_required_hearts", "count": 1, "group_names": ["5yncri5e!"], "heart_colors": ["heart00"], "location": "stage", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "target": "self", "timing_condition": "appeared_or_moved_this_turn"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "count": 1, "group_names": ["5yncri5e!"], "heart_colors": ["heart00"], "location": "stage", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "target": "self", "timing_condition": "appeared_or_moved_this_turn"}
```

- 自分のステージにいる、このターン中に登場、またはエリアを移動した『5yncri5e!』のメンバー1人につき、このカードを成功させるための必要ハートを{heart_00.png|heart0}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp2-021-L | 未体験HORIZON (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "max": true, "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck_bottom", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "max": true, "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、ライブカードを1枚までデッキの一番下に置く (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp2-022-L | 未熟DREAMER (ab#0)"], "effect": {"action": "modify_score", "condition": {"location": "deck", "target": "self", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "location_condition"}, "operation": "add", "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"location": "deck", "target": "self", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "location_condition"}, "operation": "add", "self_target": true, "value": 2}
```

- このターン、自分のデッキがリフレッシュしていた場合、このカードのスコアを+2する (x1)

```json
{"location": "deck", "target": "self", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "location_condition"}
```

- このターン、自分のデッキがリフレッシュしていた場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp2-023-L | MY舞☆TONIGHT (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "live_card", "exclude_characters": ["MY舞☆TONIGHT"], "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "duration": "live_end", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "live_card", "exclude_characters": ["MY舞☆TONIGHT"], "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "duration": "live_end", "resource": "blade", "target": "self"}
```

- 自分のライブカード置き場に「MY舞☆TONIGHT」以外の『Aqours』のライブカードがある場合、ライブ終了時まで、自分のステージのメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "live_card", "exclude_characters": ["MY舞☆TONIGHT"], "group_names": ["Aqours"], "location": "live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分のライブカード置き場に「MY舞☆TONIGHT」以外の『Aqours』のライブカードがある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp2-025-L | 青空Jumping Heart (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}
```

- 自分の成功ライブカード置き場にカードが2枚以上ある場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp2-015-N | 平安名すみれ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_property": "has_blade_heart", "heart_colors": ["heart06"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"card_property": "has_blade_heart", "heart_colors": ["heart06"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart06"], "resource": "heart"}
```

- エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{heart_06.png|heart06}を得る (x1)

```json
{"card_property": "has_blade_heart", "heart_colors": ["heart06"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}
```

- エールにより公開された自分のカードの中にブレードハートを持つカードがないとき (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp2-020-N | 鬼塚夏美 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_property": "has_blade_heart", "heart_colors": ["heart02"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart02"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"card_property": "has_blade_heart", "heart_colors": ["heart02"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart02"], "resource": "heart"}
```

- エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{heart_02.png|heart02}を得る (x1)

```json
{"card_property": "has_blade_heart", "heart_colors": ["heart02"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}
```

- エールにより公開された自分のカードの中にブレードハートを持つカードがないとき (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp2-021-N | ウィーン・マルガレーテ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_property": "has_blade_heart", "heart_colors": ["heart03"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"card_property": "has_blade_heart", "heart_colors": ["heart03"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart03"], "resource": "heart"}
```

- エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{heart_03.png|heart03}を得る (x1)

```json
{"card_property": "has_blade_heart", "heart_colors": ["heart03"], "location": "revealed_cards", "negation": true, "target": "self", "type": "location_condition"}
```

- エールにより公開された自分のカードの中にブレードハートを持つカードがないとき (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp2-023-L | Go!! リスタート (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "live_card", "comparison_target": "opponent", "location": "success_live_card_zone", "operator": "<", "target": "self", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "live_card", "comparison_target": "opponent", "location": "success_live_card_zone", "operator": "<", "target": "self", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- 自分の成功ライブカード置き場のカード枚数が相手より少ない場合、このカードのスコアを+1する (x1)

```json
{"card_type": "live_card", "comparison_target": "opponent", "location": "success_live_card_zone", "operator": "<", "target": "self", "type": "comparison_condition"}
```

- 自分の成功ライブカード置き場のカード枚数が相手より少ない場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp2-025-L | Bubble Rise (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "condition": {"count": 2, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 1, "destination": "hand", "distinct": "card_name", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "card", "condition": {"count": 2, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 1, "destination": "hand", "distinct": "card_name", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}
```

- 自分のステージに「澁谷かのん」、「ウィーン・マルガレーテ」、「鬼塚冬毬」のうち、名前の異なるメンバーが2人以上いる場合、エールにより公開された自分のカードの中から、カードを1枚手札に加える (x1)

```json
{"count": 2, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージに「澁谷かのん」、「ウィーン・マルガレーテ」、「鬼塚冬毬」のうち、名前の異なるメンバーが2人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-012-N | 乙宗 梢 (ab#0)"], "effect": {"action": "look_and_select", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "look_and_select", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-013-N | 夕霧綴理 (ab#0)"], "effect": {"action": "look_and_select", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "look_and_select", "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "reveal": true}}
```

- このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-014-N | 大沢瑠璃乃 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "restriction", "count": 1, "duration": "live_end", "restriction_type": "cannot_live"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "restriction", "count": 1, "duration": "live_end", "restriction_type": "cannot_live"}]}
```

- カードを1枚引く。ライブ終了時まで、自分はライブできない (x1)

```json
{"action": "restriction", "count": 1, "duration": "live_end", "restriction_type": "cannot_live"}
```

- 自分はライブできない (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-015-N | 藤島 慈 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}}
```

- このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-017-N | 徒町 小鈴 (ab#0)"], "effect": {"action": "draw_card", "condition": {"count": 10, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"count": 10, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- 自分の控え室にカードが10枚以上ある場合、カードを1枚引く (x1)

```json
{"count": 10, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室にカードが10枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-018-N | 安養寺 姫芽 (ab#0)"], "cost": {"optional": true, "target": "self", "type": "custom"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "live_card_zone", "source": "discard", "target": "self"}, {"action": "set_card_identity", "card_type": "live_card", "count": 1}]}, "is_null": false, "triggers": "登場"}
```


```json
{"optional": true, "target": "self", "type": "custom"}
```

- 自分のメインフェイズの場合、{icon_energy.png|E}{icon_energy.png|E}支払ってもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "live_card_zone", "source": "discard", "target": "self"}, {"action": "set_card_identity", "card_type": "live_card", "count": 1}]}
```

- 自分の控え室からライブカードを1枚、表向きでライブカード置き場に置く。次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "live_card_zone", "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードを1枚、表向きでライブカード置き場に置く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-019-L | Bloom the smile, Bloom the dream! (ab#0)"], "effect": {"action": "choice", "condition": {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart00", "heart04", "heart05"], "optional": true, "options": [{"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart01"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}, {"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart04"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}, {"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart05"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "choice", "condition": {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart00", "heart04", "heart05"], "optional": true, "options": [{"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart01"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}, {"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart04"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}, {"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart05"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}]}
```

- 自分のステージに『蓮ノ空』のメンバーがいる場合、このカードを成功させるための必要ハートは、{heart_01.png|heart01}{heart_01.png|heart01}{heart_00.png|heart0}か、{heart_04.png|heart04}{heart_04.png|heart04}{heart_00.png|heart0}か、{heart_05.png|heart05}{heart_05.png|heart05}{heart_00.png|heart0}のうち、選んだ1つにしてもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart01"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}
```

- {heart_01.png|heart01}{heart_01.png|heart01}{heart_00.png|heart0} (x1)

```json
{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart01"], "operation": "set", "self_target": true}
```

- heart01×2 (x1)

```json
{"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart04"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}
```

- {heart_04.png|heart04}{heart_04.png|heart04}{heart_00.png|heart0} (x1)

```json
{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart04"], "operation": "set", "self_target": true}
```

- heart04×2 (x1)

```json
{"action": "sequential", "actions": [{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart05"], "operation": "set", "self_target": true}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "operation": "set", "self_target": true}]}
```

- {heart_05.png|heart05}{heart_05.png|heart05}{heart_00.png|heart0} (x1)

```json
{"action": "modify_required_hearts", "count": 2, "heart_colors": ["heart05"], "operation": "set", "self_target": true}
```

- heart05×2 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-020-L | Link to the FUTURE (ab#1)"], "effect": {"action": "modify_score", "distinct": "card_name", "group_names": ["蓮ノ空"], "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "target": "self", "value": 2}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "distinct": "card_name", "group_names": ["蓮ノ空"], "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "target": "self", "value": 2}
```

- 自分のステージにいる名前の異なる『蓮ノ空』のメンバー1人につき、このカードのスコアを+2する (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-021-L | 眩耀夜行 (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart04"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart04"], "operation": "decrease", "self_target": true}
```

- 自分のステージに、このターン中にバトンタッチして登場した『蓮ノ空』のメンバーが2人以上いる場合、このカードを成功させるための必要ハートを{heart_04.png|heart04}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-023-L | Mirage Voyage (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "operation": "decrease", "self_target": true}
```

- 自分のステージに、このターン中にバトンタッチして登場した『蓮ノ空』のメンバーが2人以上いる場合、このカードを成功させるための必要ハートを{heart_05.png|heart05}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp2-025-L | ココン東西 (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart01"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "group_names": ["蓮ノ空"], "heart_colors": ["heart01"], "operation": "decrease", "self_target": true}
```

- 自分のステージに、このターン中にバトンタッチして登場した『蓮ノ空』のメンバーが2人以上いる場合、このカードを成功させるための必要ハートを{heart_01.png|heart01}減らす (x1)

```json
{"card_count": 1, "cards": ["LL-bp2-001-R＋ | 渡辺 曜&鬼塚夏美&大沢瑠璃乃 (ab#0)"], "effect": {"action": "modify_cost", "exclude_self": true, "location": "hand", "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "exclude_self": true, "location": "hand", "operation": "subtract", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "value": 1}
```

- 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる (x1)

```json
{"card_count": 1, "cards": ["LL-bp2-001-R＋ | 渡辺 曜&鬼塚夏美&大沢瑠璃乃 (ab#1)"], "effect": {"action": "restriction", "card_type": "member_card", "count": 1, "restriction_type": "cannot_baton_touch"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "restriction", "card_type": "member_card", "count": 1, "restriction_type": "cannot_baton_touch"}
```

- このメンバーはバトンタッチで控え室に置けない (x1)

```json
{"card_count": 1, "cards": ["LL-bp2-001-R＋ | 渡辺 曜&鬼塚夏美&大沢瑠璃乃 (ab#2)"], "cost": {"any_number": true, "characters": ["渡辺曜", "鬼塚夏美", "大沢瑠璃乃"], "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "parenthetical": ["手札のこのカードもこの効果で控え室に置ける。"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "discard", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"any_number": true, "characters": ["渡辺曜", "鬼塚夏美", "大沢瑠璃乃"], "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の「渡辺曜」と「鬼塚夏美」と「大沢瑠璃乃」を、好きな枚数控え室に置いてもよい (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "parenthetical": ["手札のこのカードもこの効果で控え室に置ける。"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "discard", "resource": "blade"}
```

- これによって控え室に置いた枚数1枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!S-pb1-013-N | 黒澤ダイヤ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "heart_colors": ["heart04"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart04"], "optional": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart04"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart04"], "optional": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からハートに{heart_04.png|heart04}を2個以上持つメンバーカードか、必要ハートに{heart_04.png|heart04}を2以上含むライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart04"], "optional": true}
```


```json
{"card_count": 1, "cards": ["PL!S-pb1-014-N | 渡辺 曜 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "heart_colors": ["heart02"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02"], "optional": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart02"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02"], "optional": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からハートに{heart_02.png|heart02}を2個以上持つメンバーカードか、必要ハートに{heart_02.png|heart02}を2以上含むライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart02"], "optional": true}
```


```json
{"card_count": 1, "cards": ["PL!S-pb1-015-N | 津島善子 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "heart_colors": ["heart05"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart05"], "optional": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart05"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart05"], "optional": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からハートに{heart_05.png|heart05}を2個以上持つメンバーカードか、必要ハートに{heart_05.png|heart05}を2以上含むライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart05"], "optional": true}
```


```json
{"card_count": 1, "cards": ["PL!S-pb1-019-L | 元気全開DAY！DAY！DAY！ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "invalidate_ability", "group_names": ["Aqours"], "self_target": true}, {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "heart_colors": ["heart02"], "source": "energy_deck", "state_change": "wait"}], "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "group_names": ["Aqours"], "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, "group_names": ["Aqours"], "heart_colors": ["heart02"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "invalidate_ability", "group_names": ["Aqours"], "self_target": true}, {"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "heart_colors": ["heart02"], "source": "energy_deck", "state_change": "wait"}], "condition": {"aggregate": "total", "card_type": "member_card", "count": 6, "group_names": ["Aqours"], "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, "group_names": ["Aqours"], "heart_colors": ["heart02"]}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_02.png|heart02}が合計6個以上ある場合、このカードの{live_success.png|ライブ成功時}能力を無効にする。{live_success.png|ライブ成功時}相手は、エネルギーデッキからエネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 6, "group_names": ["Aqours"], "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_02.png|heart02}が合計6個以上ある場合 (x1)

```json
{"action": "invalidate_ability", "group_names": ["Aqours"], "self_target": true}
```

- このカードの{live_success.png|ライブ成功時}能力を無効にする (x1)

```json
{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "heart_colors": ["heart02"], "source": "energy_deck", "state_change": "wait"}
```

- {live_success.png|ライブ成功時}相手は、エネルギーデッキからエネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_count": 1, "cards": ["PL!S-pb1-020-L | トリコリコPLEASE!! (ab#0)"], "effect": {"action": "modify_score", "condition": {"aggregate": "total", "card_type": "member_card", "count": 10, "group_names": ["Aqours"], "heart_colors": ["heart04"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, "group_names": ["Aqours"], "heart_colors": ["heart04"], "operation": "add", "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"aggregate": "total", "card_type": "member_card", "count": 10, "group_names": ["Aqours"], "heart_colors": ["heart04"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, "group_names": ["Aqours"], "heart_colors": ["heart04"], "operation": "add", "self_target": true, "value": 2}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_04.png|heart04}が合計10個以上ある場合、このカードのスコアを+2する (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 10, "group_names": ["Aqours"], "heart_colors": ["heart04"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_04.png|heart04}が合計10個以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-pb1-021-L | Strawberry Trapper (ab#0)"], "effect": {"action": "modify_score", "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"aggregate": "total", "card_type": "member_card", "count": 4, "group_names": ["Aqours"], "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, {"condition": {"no_excess_heart": true, "type": "opponent_live_success"}, "temporal": "this_turn", "type": "temporal_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["Aqours"], "heart_colors": ["heart05"], "operation": "add", "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"aggregate": "total", "card_type": "member_card", "count": 4, "group_names": ["Aqours"], "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, {"condition": {"no_excess_heart": true, "type": "opponent_live_success"}, "temporal": "this_turn", "type": "temporal_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["Aqours"], "heart_colors": ["heart05"], "operation": "add", "self_target": true, "value": 2}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_05.png|heart05}が合計4個以上あり、このターン、相手が余剰のハートを持たずにライブを成功させていた場合、このカードのスコアを+2する (x1)

```json
{"aggregate": "total", "card_type": "member_card", "conditions": [{"aggregate": "total", "card_type": "member_card", "count": 4, "group_names": ["Aqours"], "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, {"condition": {"no_excess_heart": true, "type": "opponent_live_success"}, "temporal": "this_turn", "type": "temporal_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_05.png|heart05}が合計4個以上あり、このターン、相手が余剰のハートを持たずにライブを成功させていた場合 (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 4, "group_names": ["Aqours"], "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}
```

- 自分のステージにいる『Aqours』のメンバーが持つハートに、{heart_05.png|heart05}が合計4個以上 (x1)

```json
{"condition": {"no_excess_heart": true, "type": "opponent_live_success"}, "temporal": "this_turn", "type": "temporal_condition"}
```

- このターン、相手が余剰のハートを持たずにライブを成功させていた場合 (x1)

```json
{"no_excess_heart": true, "type": "opponent_live_success"}
```


```json
{"card_count": 1, "cards": ["PL!-bp3-019-L | 僕らのLIVE 君とのLIFE (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 2, "group_names": ["μ's"], "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["μ's"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"count": 2, "group_names": ["μ's"], "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["μ's"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のライブ中の『μ's』のカードが2枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"count": 2, "group_names": ["μ's"], "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のライブ中の『μ's』のカードが2枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp3-022-L | ユメノトビラ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "reveal", "count": 1, "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "hand", "target": "both"}, {"action": "modify_score", "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "both", "value": 1}], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "target": "both"}, {"action": "move_cards", "card_type": "card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "reveal", "count": 1, "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "hand", "target": "both"}, {"action": "modify_score", "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "both", "value": 1}], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "target": "both"}, {"action": "move_cards", "card_type": "card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}]}
```

- 自分のデッキの上から、自分と相手のステージにいるメンバー1人につき、1枚公開する。それらの中にあるライブカード1枚につき、このカードのスコアを+1する。その後、これにより公開したカードを控え室に置く (x1)

```json
{"action": "sequential", "actions": [{"action": "reveal", "count": 1, "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "hand", "target": "both"}, {"action": "modify_score", "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "both", "value": 1}], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "target": "both"}
```


```json
{"action": "reveal", "count": 1, "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "hand", "target": "both"}
```

- 1枚公開する (x1)

```json
{"action": "modify_score", "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "both", "value": 1}
```

- それらの中にあるライブカード1枚につき、このカードのスコアを+1する (x1)

```json
{"action": "move_cards", "card_type": "card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}
```

- これにより公開したカードを控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!-bp3-023-L | ミはμ'sicのミ (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "member_card", "count": 10, "heart_colors": ["heart00"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "member_card", "count": 10, "heart_colors": ["heart00"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 自分のステージにいるメンバーが持つ{icon_blade.png|ブレード}の合計が10以上の場合、このカードを成功させるための必要ハートは{heart_00.png|heart0}{heart_00.png|heart0}少なくなる (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 10, "heart_colors": ["heart00"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition"}
```

- 自分のステージにいるメンバーが持つ{icon_blade.png|ブレード}の合計が10以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp3-024-L | 夏色えがおで1,2,Jump! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart", "target": "self", "target_count": 1}], "condition": {"card_type": "live_card", "heart_colors": ["heart01", "heart03", "heart06"], "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"]}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart", "target": "self", "target_count": 1}], "condition": {"card_type": "live_card", "heart_colors": ["heart01", "heart03", "heart06"], "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"]}
```

- 自分の成功ライブカード置き場にカードがある場合、{heart_01.png|heart01}か{heart_03.png|heart03}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいる『μ's』のメンバー1人は、選んだハートを1つ得る (x1)

```json
{"card_type": "live_card", "heart_colors": ["heart01", "heart03", "heart06"], "location": "success_live_card_zone", "target": "self", "type": "location_condition"}
```

- 自分の成功ライブカード置き場にカードがある場合 (x1)

```json
{"action": "select", "count": 1, "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"]}
```

- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_06.png|heart06}のうち、1つを選ぶ (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "heart_colors": ["heart01", "heart03", "heart06"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいる『μ's』のメンバー1人は、選んだハートを1つ得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp3-024-L | 夏色えがおで1,2,Jump! (ab#1)"], "effect": {"action": "modify_score", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!-bp3-025-L | タカラモノズ (ab#0)"], "effect": {"action": "modify_score", "condition": {"condition": {"no_excess_heart": true, "type": "no_excess_heart"}, "temporal": "this_turn", "type": "temporal_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"condition": {"no_excess_heart": true, "type": "no_excess_heart"}, "temporal": "this_turn", "type": "temporal_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- このターン、自分が余剰ハートを持たない場合、このカードのスコアを+1する (x1)

```json
{"condition": {"no_excess_heart": true, "type": "no_excess_heart"}, "temporal": "this_turn", "type": "temporal_condition"}
```

- このターン、自分が余剰ハートを持たない場合 (x1)

```json
{"no_excess_heart": true, "type": "no_excess_heart"}
```


```json
{"card_count": 1, "cards": ["PL!-bp3-026-L | Oh,Love&Peace! (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 3, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 3, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいるメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp3-026-L | Oh,Love&Peace! (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "comparison_target": "opponent", "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "comparison_target": "opponent", "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージにいるメンバーが持つハートの総数が、相手のステージにいるメンバーが持つハートの総数より多い場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "comparison_target": "opponent", "location": "stage", "operator": ">", "target": "both", "type": "comparison_condition"}
```

- 自分のステージにいるメンバーが持つハートの総数が、相手のステージにいるメンバーが持つハートの総数より多い場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp3-016-N | 国木田花丸 (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self", "value": 1}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self", "value": 1}
```

- 自分の成功ライブカード置き場にあるカード1枚につき、ステージにいるこのメンバーのコストを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp3-019-L | MIRACLE WAVE (ab#0)"], "effect": {"action": "modify_score", "condition": {"conditions": [{"card_property": "has_blade_heart", "count": 1, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "location_condition"}, {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}], "type": "or_condition"}, "operation": "set", "self_target": true, "value": 4}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"conditions": [{"card_property": "has_blade_heart", "count": 1, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "location_condition"}, {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}], "type": "or_condition"}, "operation": "set", "self_target": true, "value": 4}
```

- このターン、エールにより公開された自分のカードの中にブレードハートを持たないカードが0枚の場合か、または自分が余剰ハートを2つ以上持っている場合、このカードのスコアは4になる (x1)

```json
{"conditions": [{"card_property": "has_blade_heart", "count": 1, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "location_condition"}, {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}], "type": "or_condition"}
```

- このターン、エールにより公開された自分のカードの中にブレードハートを持たないカードが0枚の場合か、または自分が余剰ハートを2つ以上持っている場合、このカードのスコアは4になる (x1)

```json
{"card_property": "has_blade_heart", "count": 1, "location": "revealed_cards", "negation": true, "operator": ">=", "target": "self", "type": "location_condition"}
```

- このターン、エールにより公開された自分のカードの中にブレードハートを持たないカードが0枚の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp3-020-L | ダイスキだったらダイジョウブ！ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "all": true, "card_type": "card", "destination": "discard", "optional": true, "source": "hand"}, {"action": "re_yell", "all": true, "lose_blade_hearts": true}], "all": true, "condition": {"count": 1, "operator": ">=", "target": "self", "type": "card_count_condition"}}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "all": true, "card_type": "card", "destination": "discard", "optional": true, "source": "hand"}, {"action": "re_yell", "all": true, "lose_blade_hearts": true}], "all": true, "condition": {"count": 1, "operator": ">=", "target": "self", "type": "card_count_condition"}}
```

- エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。そのエールで得たブレードハートを失い、もう一度エールを行う (x1)

```json
{"count": 1, "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合 (x1)

```json
{"action": "move_cards", "all": true, "card_type": "card", "destination": "discard", "optional": true, "source": "hand"}
```

- それらのカードをすべて控え室に置いてもよい (x1)

```json
{"action": "re_yell", "all": true, "lose_blade_hearts": true}
```

- そのエールで得たブレードハートを失い、もう一度エールを行う (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp3-021-L | 想いよひとつになれ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "deck_top", "optional": true, "source": "discard", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}], "conditional": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "deck_top", "optional": true, "source": "discard", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}], "conditional": true}
```

- 自分の控え室にあるメンバーカード1枚をデッキの一番上に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "deck_top", "optional": true, "source": "discard", "target": "self"}
```

- 自分の控え室にあるメンバーカード1枚をデッキの一番上に置いてもよい。 (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいるメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp3-024-L | Deep Resonance (ab#0)"], "effect": {"action": "choice", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 9, "cost_total": 9, "count": 9, "group_names": ["Aqours"], "location": "stage", "operator": ">=", "position": "center", "target": "self", "type": "comparison_condition"}, "count": 1, "group_names": ["Aqours"], "options": [{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}, {"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}], "position": "center"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "choice", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 9, "cost_total": 9, "count": 9, "group_names": ["Aqours"], "location": "stage", "operator": ">=", "position": "center", "target": "self", "type": "comparison_condition"}, "count": 1, "group_names": ["Aqours"], "options": [{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}, {"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}], "position": "center"}
```

- 自分のステージのセンターエリアにコスト9以上の『Aqours』のメンバーがいる場合、以下から1つを選ぶ。
・ライブ終了時まで、自分のステージにいるメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る。
・相手のステージにいるコスト4以下のメンバー1人をウェイトにする (x1)

```json
{"card_type": "member_card", "comparison_type": "cost", "cost_limit": 9, "cost_total": 9, "count": 9, "group_names": ["Aqours"], "location": "stage", "operator": ">=", "position": "center", "target": "self", "type": "comparison_condition"}
```

- 自分のステージのセンターエリアにコスト9以上の『Aqours』のメンバーがいる場合 (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}
```

- ライブ終了時まで、自分のステージにいるメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る。 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp3-025-L | SUKI for you, DREAM for you! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "count": 1, "group_names": ["Aqours"], "target": "self"}, {"action": "modify_score", "condition": {"count": 6, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}, "group_names": ["Aqours"], "operation": "add", "self_target": true, "value": 1}], "group_names": ["Aqours"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "count": 1, "group_names": ["Aqours"], "target": "self"}, {"action": "modify_score", "condition": {"count": 6, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}, "group_names": ["Aqours"], "operation": "add", "self_target": true, "value": 1}], "group_names": ["Aqours"]}
```

- 自分のステージにいる『Aqours』のメンバー1人を選ぶ。そのメンバーが持つ{icon_blade.png|ブレード}が6つ以上の場合、このカードのスコアを+1する (x1)

```json
{"action": "select", "card_type": "member_card", "count": 1, "group_names": ["Aqours"], "target": "self"}
```

- 自分のステージにいる『Aqours』のメンバー1人を選ぶ (x1)

```json
{"action": "modify_score", "condition": {"count": 6, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}, "group_names": ["Aqours"], "operation": "add", "self_target": true, "value": 1}
```

- そのメンバーが持つ{icon_blade.png|ブレード}が6つ以上の場合、このカードのスコアを+1する (x1)

```json
{"count": 6, "operator": ">=", "source": "selected_cards", "type": "card_blade_condition"}
```

- そのメンバーが持つ{icon_blade.png|ブレード}が6つ以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-013-N | 上原歩夢 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "optional": true, "target": "self"}, {"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}], "conditional": true, "parenthetical": ["メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "place_energy_under_member", "card_type": "member_card", "count": 1, "destination": "under_member", "energy_count": 1, "optional": true, "target": "self"}, {"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}], "conditional": true, "parenthetical": ["メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。"]}
```

- 自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。そうした場合、カードを2枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-014-N | 中須かすみ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart04"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart01", "heart03", "heart04"], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart04"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart01", "heart03", "heart04"], "original_value": true}
```

- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_04.png|heart04}のうち1つを選ぶ。ライブ終了時まで、このメンバーが元々持つハートは選んだハートになる (x1)

```json
{"action": "select", "count": 1, "heart_colors": ["heart01", "heart03", "heart04"]}
```

- {heart_01.png|heart01}か{heart_03.png|heart03}か{heart_04.png|heart04}のうち1つを選ぶ (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-015-N | 桜坂しずく (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart02", "heart05", "heart06"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart02", "heart05", "heart06"], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart02", "heart05", "heart06"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart02", "heart05", "heart06"], "original_value": true}
```

- {heart_02.png|heart02}か{heart_05.png|heart05}か{heart_06.png|heart06}のうち1つを選ぶ。ライブ終了時まで、このメンバーが元々持つハートは選んだハートになる (x1)

```json
{"action": "select", "count": 1, "heart_colors": ["heart02", "heart05", "heart06"]}
```

- {heart_02.png|heart02}か{heart_05.png|heart05}か{heart_06.png|heart06}のうち1つを選ぶ (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-025-L | Awakening Promise (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "place_energy_under_member", "any_number": true, "card_type": "member_card", "energy_count": 1, "optional": true, "source": "under_member", "target": "self", "target_member": "this_member"}, {"action": "gain_resource", "card_type": "energy_card", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart"}], "conditional": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "place_energy_under_member", "any_number": true, "card_type": "member_card", "energy_count": 1, "optional": true, "source": "under_member", "target": "self", "target_member": "this_member"}, {"action": "gain_resource", "card_type": "energy_card", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart"}], "conditional": true}
```

- 自分のステージにいるメンバー1人の下にあるエネルギーカードを、好きな枚数エネルギーデッキに置いてもよい。そうした場合、ライブ終了時まで、そのメンバーは、これによって置いたエネルギーカード1枚につき、{heart_01.png|赤ハート}{heart_01.png|赤ハート}{heart_01.png|赤ハート}を得る (x1)

```json
{"action": "place_energy_under_member", "any_number": true, "card_type": "member_card", "energy_count": 1, "optional": true, "source": "under_member", "target": "self", "target_member": "this_member"}
```

- 自分のステージにいるメンバー1人の下にあるエネルギーカードを、好きな枚数エネルギーデッキに置いてもよい。 (x1)

```json
{"action": "gain_resource", "card_type": "energy_card", "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart"}
```

- そのメンバーは、これによって置いたエネルギーカード1枚につき、{heart_01.png|赤ハート}{heart_01.png|赤ハート}{heart_01.png|赤ハート}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-026-L | サイコーハート (ab#0)"], "effect": {"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"card_type": "live_card", "comparison_type": "score", "location": "success_live_card_zone", "target": "self", "type": "comparison_condition"}, "primary_effect": {"action": "modify_score", "operation": "add", "self_target": true, "value": 1}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"card_type": "live_card", "comparison_type": "score", "location": "success_live_card_zone", "target": "self", "type": "comparison_condition"}, "primary_effect": {"action": "modify_score", "operation": "add", "self_target": true, "value": 1}}
```

- 自分の成功ライブカード置き場にスコアが1か5のカードがある場合、このカードのスコアを+1する。それらが両方ある場合、代わりにスコアを+2する (x1)

```json
{"card_type": "live_card", "comparison_type": "score", "location": "success_live_card_zone", "target": "self", "type": "comparison_condition"}
```

- 自分の成功ライブカード置き場にスコアが1か5のカードがある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-027-L | La Bella Patria (ab#0)"], "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "member_card", "conditions": [{"count": 1, "heart_colors": ["heart04"], "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, {"card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 1, "destination": "energy_zone", "group_names": ["虹ヶ咲"], "heart_colors": ["heart04"], "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"card_type": "member_card", "conditions": [{"count": 1, "heart_colors": ["heart04"], "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, {"card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 1, "destination": "energy_zone", "group_names": ["虹ヶ咲"], "heart_colors": ["heart04"], "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- このターン、自分が余剰ハートに{heart_04.png|heart04}を1つ以上持っており、かつ自分のステージに『虹ヶ咲』のメンバーがいる場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"card_type": "member_card", "conditions": [{"count": 1, "heart_colors": ["heart04"], "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, {"card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- このターン、自分が余剰ハートに{heart_04.png|heart04}を1つ以上持っており、かつ自分のステージに『虹ヶ咲』のメンバーがいる場合 (x1)

```json
{"count": 1, "heart_colors": ["heart04"], "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}
```

- このターン、自分が余剰ハートに{heart_04.png|heart04}を1つ以上持っており、 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-030-L | Love U my friends (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_property": "has_all_blade", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_property": "has_all_blade", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- エールにより公開された自分のカードの中に{icon_b_all.png|ALLブレード}を持つカードが1枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"card_property": "has_all_blade", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- エールにより公開された自分のカードの中に{icon_b_all.png|ALLブレード}を持つカードが1枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp3-031-L | MONSTER GIRLS (ab#0)"], "effect": {"action": "modify_score", "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "state": "wait", "target": "self", "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "location": "stage", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "state": "wait", "target": "self", "value": 1}
```

- 自分のステージにいるウェイト状態のメンバー1人につき、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["LL-bp3-001-R＋ | 園田海未&津島善子&天王寺璃奈 (ab#0)"], "cost": {"characters": ["園田海未", "津島善子", "天王寺璃奈"], "count": 6, "destination": "deck_bottom", "shuffle": true, "source": "discard", "target": "self", "type": "move_cards", "zone": "discard"}, "effect": {"action": "change_state", "card_type": "energy_card", "count": 6, "max": true, "state_change": "active"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"characters": ["園田海未", "津島善子", "天王寺璃奈"], "count": 6, "destination": "deck_bottom", "shuffle": true, "source": "discard", "target": "self", "type": "move_cards", "zone": "discard"}
```

- 自分の控え室にある「園田海未」と「津島善子」と「天王寺璃奈」を、合計6枚をシャッフルしてデッキの一番下に置く (x1)

```json
{"action": "change_state", "card_type": "energy_card", "count": 6, "max": true, "state_change": "active"}
```

- エネルギーを6枚までアクティブにする (x1)

```json
{"card_count": 1, "cards": ["LL-bp3-001-R＋ | 園田海未&津島善子&天王寺璃奈 (ab#1)"], "cost": {"count": 6, "energy": 6, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 3, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!-pb1-028-L | WAO-WAO Powerful day! (ab#0)"], "effect": {"action": "conditional_on_result", "followup_action": {"action": "modify_score", "operation": "add", "self_target": true, "value": 1}, "group_names": ["Printemps"], "primary_effect": {"action": "change_state", "card_type": "member_card", "group_names": ["Printemps"], "state_change": "active", "target": "self"}, "result_condition": {"count": 3, "from_state": "wait", "operator": ">=", "to_state": "active", "type": "state_change_condition", "unit": "人"}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_on_result", "followup_action": {"action": "modify_score", "operation": "add", "self_target": true, "value": 1}, "group_names": ["Printemps"], "primary_effect": {"action": "change_state", "card_type": "member_card", "group_names": ["Printemps"], "state_change": "active", "target": "self"}, "result_condition": {"count": 3, "from_state": "wait", "operator": ">=", "to_state": "active", "type": "state_change_condition", "unit": "人"}}
```

- 自分のステージにいる『Printemps』のメンバーをアクティブにする。これによりウェイト状態のメンバーが3人以上アクティブ状態になったとき、このカードのスコアを+1する (x1)

```json
{"action": "change_state", "card_type": "member_card", "group_names": ["Printemps"], "state_change": "active", "target": "self"}
```

- 自分のステージにいる『Printemps』のメンバーをアクティブにする。 (x1)

```json
{"count": 3, "from_state": "wait", "operator": ">=", "to_state": "active", "type": "state_change_condition", "unit": "人"}
```

- これによりウェイト状態のメンバーが3人以上アクティブ状態になったとき (x1)

```json
{"card_count": 1, "cards": ["PL!-pb1-029-L | 知らないLove＊教えてLove (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"all_members": true, "card_type": "member_card", "group_names": ["lilywhite"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["lilywhite"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"all_members": true, "card_type": "member_card", "group_names": ["lilywhite"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["lilywhite"], "operation": "add", "self_target": true, "value": 1}
```

- 自分の成功ライブカード置き場のカードが0枚で、かつ自分のステージにいるメンバーが『lilywhite』のみの場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "live_card", "count": 0, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"all_members": true, "card_type": "member_card", "group_names": ["lilywhite"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}
```

- 自分の成功ライブカード置き場のカードが0枚で、かつ自分のステージにいるメンバーが『lilywhite』のみの場合 (x1)

```json
{"all_members": true, "card_type": "member_card", "group_names": ["lilywhite"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーが『lilywhite』のみの場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-pb1-030-L | Cutie Panther (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"state": "wait", "type": "state_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"state": "wait", "type": "state_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 相手のステージにウェイト状態のメンバーがいる場合、このカードを成功させるための必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!-pb1-030-L | Cutie Panther (ab#1)"], "effect": {"action": "move_cards", "card_type": "member_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["BiBi"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 1, "destination": "hand", "distinct": "card_name", "group_names": ["BiBi"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "condition": {"count": 2, "distinct": "card_name", "group_names": ["BiBi"], "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 1, "destination": "hand", "distinct": "card_name", "group_names": ["BiBi"], "source": "discard", "target": "self"}
```

- 自分のステージに名前の異なる『BiBi』のメンバーが2人以上いる場合、自分の控え室から『BiBi』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-pb1-031-L | 輝夜の城で踊りたい (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["μ's"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["μ's"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、『μ's』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-pb1-032-L | SENTIMENTAL StepS (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "live_card", "group_names": ["μ's"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 1, "cards": ["PL!-bp4-002-SEC | 絢瀬絵里 (ab#1)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "activation_condition_parsed": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 1, "cards": ["PL!-bp4-011-N | 絢瀬絵里 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}
```

- 自分のセンターエリアにいる『μ's』のメンバーは、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-013-N | 園田海未 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "heart_colors": ["heart01"], "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "heart_colors": ["heart01"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいるこのメンバー以外のメンバー1人は、{heart_01.png|heart01}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-014-N | 星空 凛 (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"ability_filter": "no_ability_type", "ability_filter_triggers": ["live_start", "live_success"], "type": "ability_filter_condition"}, "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"ability_filter": "no_ability_type", "ability_filter_triggers": ["live_start", "live_success"], "type": "ability_filter_condition"}, "count": 2, "duration": "live_end", "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のライブ中のライブカードに、{live_start.png|ライブ開始時}能力も{live_success.png|ライブ成功時}能力も持たないカードがある場合、ライブ終了時まで、自分のステージにいるこのメンバー以外のメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-017-N | 小泉花陽 (ab#0)"], "cost": {"card_type": "member_card", "optional": true, "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}
```

- 自分のセンターエリアにいる『μ's』のメンバーは、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-018-N | 矢澤にこ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_target": "opponent", "comparison_type": "score", "location": "success_live_card_zone", "operator": ">", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_target": "opponent", "comparison_type": "score", "location": "success_live_card_zone", "operator": ">", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"aggregate": "total", "card_type": "live_card", "comparison_target": "opponent", "comparison_type": "score", "location": "success_live_card_zone", "operator": ">", "target": "self", "type": "comparison_condition"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が相手より高いかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-019-L | Angelic Angel (ab#0)"], "effect": {"action": "modify_score", "card_type": "live_card", "condition": {"card_type": "member_card", "conditions": [{"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "conditional": true, "duration": "as_long_as", "operation": "add", "self_target": true, "source": "success_live_zone", "target": "self", "value": 5}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_score", "card_type": "live_card", "condition": {"card_type": "member_card", "conditions": [{"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}, "conditional": true, "duration": "as_long_as", "operation": "add", "self_target": true, "source": "success_live_zone", "target": "self", "value": 5}
```

- 自分の成功ライブカード置き場にあるこのカードのスコアを+5する (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "live_card", "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, {"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "self", "type": "compound"}
```

- このカードが自分の成功ライブカード置き場にあり、かつ自分のステージに『μ's』のメンバーがいるかぎり (x1)

```json
{"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージに『μ's』のメンバーがいるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-020-L | Love wing bell (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["μ's"], "optional": true, "target": "self", "target_member": "select"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["μ's"], "optional": true, "target": "self", "target_member": "select"}
```

- 自分のステージにいるメンバーが『μ's』のみの場合、自分のステージにいるメンバー1人をポジションチェンジさせてもよい (x1)

```json
{"all_members": true, "card_type": "member_card", "group_names": ["μ's"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーが『μ's』のみの場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-020-L | Love wing bell (ab#1)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "live_card", "check_self": true, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "live_card", "check_self": true, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "group_names": ["μ's"], "position": "center", "resource": "blade", "target": "self"}
```

- 自分のセンターエリアにいる『μ's』のメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-021-L | ?←HEARTBEAT (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, {"action": "modify_score", "condition": {"aggregate": "total", "comparison_type": "score", "count": 9, "operator": ">=", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}], "heart_colors": ["heart00"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, {"action": "modify_score", "condition": {"aggregate": "total", "comparison_type": "score", "count": 9, "operator": ">=", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}], "heart_colors": ["heart00"]}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合、このカードを成功させるための必要ハートを{heart_00.png|heart0}減らす。スコアの合計が9以上の場合、さらにこのカードのスコアを+1する (x1)

```json
{"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合、このカードを成功させるための必要ハートを{heart_00.png|heart0}減らす (x1)

```json
{"action": "modify_score", "condition": {"aggregate": "total", "comparison_type": "score", "count": 9, "operator": ">=", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- スコアの合計が9以上の場合、このカードのスコアを+1する (x1)

```json
{"aggregate": "total", "comparison_type": "score", "count": 9, "operator": ">=", "type": "comparison_condition"}
```

- スコアの合計が9以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-023-L | もぎゅっと\"love\"で接近中！ (ab#0)"], "effect": {"action": "draw_card", "condition": {"count": 1, "heart_colors": ["heart01"], "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "count": 1, "destination": "hand", "heart_colors": ["heart01"], "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"count": 1, "heart_colors": ["heart01"], "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "count": 1, "destination": "hand", "heart_colors": ["heart01"], "source": "deck"}
```

- 自分が余剰ハートに{heart_01.png|heart01}を1つ以上持つ場合、カードを1枚引く (x1)

```json
{"count": 1, "heart_colors": ["heart01"], "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}
```

- 自分が余剰ハートに{heart_01.png|heart01}を1つ以上持つ場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp4-024-L | 小夜啼鳥恋詩 (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["μ's"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいる『μ's』のメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-018-N | 近江彼方 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"from_state": "active", "phase": "main", "target": "self", "to_state": "wait", "type": "state_change_condition"}}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"from_state": "active", "phase": "main", "target": "self", "to_state": "wait", "type": "state_change_condition"}}
```

- 自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき、カードを1枚引き、手札を1枚控え室に置く (x1)

```json
{"from_state": "active", "phase": "main", "target": "self", "to_state": "wait", "type": "state_change_condition"}
```

- 自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-021-N | 天王寺璃奈 (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "optional": true, "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "optional": true, "source": "discard", "target": "self"}
```

- 自分の控え室にあるカード1枚をデッキの一番上に置いてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-023-N | ミア・テイラー (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "optional": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"card_type": "member_card", "count": 1, "optional": true, "state_change": "wait", "type": "change_state"}
```

- 『虹ヶ咲」のメンバー1人をウェイトにしてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-025-L | VIVID WORLD (ab#0)"], "effect": {"action": "set_blade_type", "blade_type": "青ブレード", "duration": "live_end"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "set_blade_type", "blade_type": "青ブレード", "duration": "live_end"}
```

- エールによって公開される自分のカードが持つ[桃ブレード]、[赤ブレード]、[黄ブレード]、[緑ブレード]、[紫ブレード]、{icon_b_all.png|ALLブレード}は、すべて[青ブレード]になる (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-025-L | VIVID WORLD (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "target": "self", "type": "group_condition"}, "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "target": "self", "type": "group_condition"}, "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "operation": "add", "self_target": true, "value": 1}
```

- エールにより公開された自分の『虹ヶ咲』のメンバーカードが持つハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}がある場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "group_names": ["虹ヶ咲"], "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "revealed_cards", "target": "self", "type": "group_condition"}
```

- エールにより公開された自分の『虹ヶ咲』のメンバーカードが持つハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}がある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-026-L | DIVE! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "characters": ["DIVE!"], "count": 1, "destination": "live_card_zone", "optional": true, "quoted_text": {"quoted_type": "character"}, "source": "hand", "target": "self"}, {"action": "set_card_identity", "card_type": "live_card", "count": 1}], "condition": {"location": "discard", "locations": ["discard", "hand"], "target": "self", "type": "location_condition"}, "conditional": true}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "characters": ["DIVE!"], "count": 1, "destination": "live_card_zone", "optional": true, "quoted_text": {"quoted_type": "character"}, "source": "hand", "target": "self"}, {"action": "set_card_identity", "card_type": "live_card", "count": 1}], "condition": {"location": "discard", "locations": ["discard", "hand"], "target": "self", "type": "location_condition"}, "conditional": true}
```

- 自分のメインフェイズにこのカードが控え室から手札に加えられたとき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。そうした場合、次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る (x1)

```json
{"action": "move_cards", "card_type": "live_card", "characters": ["DIVE!"], "count": 1, "destination": "live_card_zone", "optional": true, "quoted_text": {"quoted_type": "character"}, "source": "hand", "target": "self"}
```

- 自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。 (x1)

```json
{"location": "discard", "locations": ["discard", "hand"], "target": "self", "type": "location_condition"}
```

- 自分のメインフェイズにこのカードが控え室から手札に加えられたとき (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-026-L | DIVE! (ab#1)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "live_card", "location": "live_card_zone", "type": "location_condition"}, "count": 2, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "live_card", "location": "live_card_zone", "type": "location_condition"}, "count": 2, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self", "target_count": 1}
```

- このカードが表向きでライブカード置き場に置かれたとき、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "live_card", "location": "live_card_zone", "type": "location_condition"}
```

- このカードが表向きでライブカード置き場に置かれたとき (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-027-L | EMOTION (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self", "value": 2}, {"action": "modify_required_hearts", "count": 3, "heart_colors": ["heart00"], "location": "success_live_zone", "operation": "increase", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self"}], "heart_colors": ["heart00"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self", "value": 2}, {"action": "modify_required_hearts", "count": 3, "heart_colors": ["heart00"], "location": "success_live_zone", "operation": "increase", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self"}], "heart_colors": ["heart00"]}
```

- 自分の成功ライブカード置き場にあるカード名が「EMOTION」のカード1枚につき、このカードのスコアを+2し、成功させるための必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}{heart_00.png|heart0}増やす (x1)

```json
{"action": "modify_required_hearts", "count": 3, "heart_colors": ["heart00"], "location": "success_live_zone", "operation": "increase", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self"}
```

- 成功させるための必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}{heart_00.png|heart0}増やす (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-028-L | stars we chase (ab#0)"], "effect": {"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"card_type": "live_card", "count": 4, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["虹ヶ咲"], "primary_effect": {"action": "modify_score", "count": 6, "operation": "add", "self_target": true, "value": 1}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"card_type": "live_card", "count": 4, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "group_names": ["虹ヶ咲"], "primary_effect": {"action": "modify_score", "count": 6, "operation": "add", "self_target": true, "value": 1}}
```

- 自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが4枚以上ある場合、このカードのスコアを+1する。6枚以上ある場合、代わりにスコアを+2する (x1)

```json
{"card_type": "live_card", "count": 4, "group_names": ["虹ヶ咲"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが4枚以上ある場合 (x1)

```json
{"action": "modify_score", "count": 6, "operation": "add", "self_target": true, "value": 1}
```

- このカードのスコアを+1する。6枚以上ある場合、 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-029-L | Rise Up High! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "group_names": ["虹ヶ咲"], "operation": "add", "self_target": true, "value": 1}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self", "target_count": 1}], "condition": {"phase": "live_phase", "turn_number": 1, "type": "temporal_condition"}, "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "group_names": ["虹ヶ咲"], "operation": "add", "self_target": true, "value": 1}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self", "target_count": 1}], "condition": {"phase": "live_phase", "turn_number": 1, "type": "temporal_condition"}, "group_names": ["虹ヶ咲"]}
```

- このゲームの1ターン目のライブフェイズの場合、このカードのスコアを+1し、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"phase": "live_phase", "turn_number": 1, "type": "temporal_condition"}
```

- このゲームの1ターン目のライブフェイズの場合 (x1)

```json
{"action": "modify_score", "group_names": ["虹ヶ咲"], "operation": "add", "self_target": true, "value": 1}
```

- このカードのスコアを+1し (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["虹ヶ咲"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいる『虹ヶ咲』のメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-030-L | Daydream Mermaid (ab#0)"], "effect": {"action": "choice", "alternative_condition": {"card_type": "live_card", "group_names": ["虹ヶ咲"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "alternative_count_type": "any_number", "choice_condition": {"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "group_condition"}, "choice_modifier": "。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。", "count": 1, "group_names": ["虹ヶ咲"], "options": [{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "choice", "alternative_condition": {"card_type": "live_card", "group_names": ["虹ヶ咲"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "alternative_count_type": "any_number", "choice_condition": {"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "group_condition"}, "choice_modifier": "。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。", "count": 1, "group_names": ["虹ヶ咲"], "options": [{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}]}
```

- 以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
・自分の控え室からメンバーカードを1枚手札に加える (x1)

```json
{"card_type": "live_card", "count": 1, "group_names": ["虹ヶ咲"], "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "group_condition"}
```

- 。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。 (x1)

```json
{"card_type": "live_card", "group_names": ["虹ヶ咲"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}
```

- 。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp4-031-L | NEO SKY, NEO MAP! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 3, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 3, "destination": "deck_top", "placement_order": "any_order", "source": "hand", "target": "self"}], "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"all_areas": true, "card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["虹ヶ咲"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 3, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 3, "destination": "deck_top", "placement_order": "any_order", "source": "hand", "target": "self"}], "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"all_areas": true, "card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["虹ヶ咲"]}
```

- 自分のステージのエリアすべてに『虹ヶ咲』のメンバーがいて、かつそれらのコストの合計が20以上の場合、カードを3枚引き、自分の手札を3枚好きな順番でデッキの上に置く (x1)

```json
{"aggregate": "total", "card_type": "member_card", "conditions": [{"all_areas": true, "card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}, {"aggregate": "total", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージのエリアすべてに『虹ヶ咲』のメンバーがいて、かつそれらのコストの合計が20以上の場合 (x1)

```json
{"all_areas": true, "card_type": "member_card", "group_names": ["虹ヶ咲"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージのエリアすべてに『虹ヶ咲』のメンバーがいて、 (x1)

```json
{"aggregate": "total", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}
```

- それらのコストの合計が20以上の場合 (x1)

```json
{"action": "draw_card", "count": 3, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "deck"}
```

- カードを3枚引き (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 3, "destination": "deck_top", "placement_order": "any_order", "source": "hand", "target": "self"}
```

- 自分の手札を3枚好きな順番でデッキの上に置く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-012-N | 澁谷かのん (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!SP-bp4-016-N | 葉月 恋 (ab#0)"], "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart06"], "parenthetical": ["相手のカードの効果でも発動する。"], "resource": "heart", "trigger_condition": {"card_type": "energy_card", "location": "energy_zone", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "heart_colors": ["heart06"], "parenthetical": ["相手のカードの効果でも発動する。"], "resource": "heart", "trigger_condition": {"card_type": "energy_card", "location": "energy_zone", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "trigger_type": "each_time"}
```

- カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれるたび、ライブ終了時まで、{heart_06.png|heart06}を得る (x1)

```json
{"card_type": "energy_card", "location": "energy_zone", "resource_type": "energy", "target": "self", "type": "comparison_condition"}
```

- カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれる (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-017-N | 桜小路きな子 (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "left_side", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "left_side", "temporal": "this_turn", "type": "temporal_condition"}, "count": 2, "duration": "live_end", "parenthetical": ["この能力は左サイドエリアにいる場合のみ発動する。"], "position": "left_side", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "activation_position": "left_side", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "left_side", "temporal": "this_turn", "type": "temporal_condition"}, "count": 2, "duration": "live_end", "parenthetical": ["この能力は左サイドエリアにいる場合のみ発動する。"], "position": "left_side", "resource": "blade"}
```

- このターン、このメンバーがエリアを移動している場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "left_side", "temporal": "this_turn", "type": "temporal_condition"}
```

- {leftside.png|左サイド}このターン、このメンバーがエリアを移動している場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-018-N | 米女メイ (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 1, "cards": ["PL!SP-bp4-020-N | 鬼塚夏美 (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "right_side", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "right_side", "temporal": "this_turn", "type": "temporal_condition"}, "count": 2, "duration": "live_end", "parenthetical": ["この能力は右サイドエリアにいる場合のみ発動する。"], "position": "right_side", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "activation_position": "right_side", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "right_side", "temporal": "this_turn", "type": "temporal_condition"}, "count": 2, "duration": "live_end", "parenthetical": ["この能力は右サイドエリアにいる場合のみ発動する。"], "position": "right_side", "resource": "blade"}
```

- このターン、このメンバーがエリアを移動している場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "right_side", "temporal": "this_turn", "type": "temporal_condition"}
```

- {rightside.png|右サイド}このターン、このメンバーがエリアを移動している場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-021-N | ウィーン・マルガレーテ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"comparison_target": "opponent", "operator": ">", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"comparison_target": "opponent", "operator": ">", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}
```

- {heart_06.png|heart06}を得る (x1)

```json
{"comparison_target": "opponent", "operator": ">", "resource_type": "energy", "target": "self", "type": "comparison_condition"}
```

- 自分のエネルギーが相手より多いかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-022-N | 鬼塚冬毬 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "resource": "blade"}
```

- 支払った{icon_energy.png|E}につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-023-L | Dazzling Game (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "characters": ["澁谷かのん", "ウィーン・マルガレーテ", "鬼塚冬毬"], "count": 1, "duration": "live_end", "group_names": ["Liella!"]}, {"action": "select", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_selected": true, "group_names": ["Liella!"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}], "duration": "live_end", "group_names": ["Liella!"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "characters": ["澁谷かのん", "ウィーン・マルガレーテ", "鬼塚冬毬"], "count": 1, "duration": "live_end", "group_names": ["Liella!"]}, {"action": "select", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_selected": true, "group_names": ["Liella!"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}], "duration": "live_end", "group_names": ["Liella!"]}
```

- 自分のステージにいる、「澁谷かのん」「ウィーン・マルガレーテ」「鬼塚冬毬」のうちのメンバー1人と、これにより選んだメンバー以外の『Liella!』のメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "select", "card_type": "member_card", "characters": ["澁谷かのん", "ウィーン・マルガレーテ", "鬼塚冬毬"], "count": 1, "duration": "live_end", "group_names": ["Liella!"]}
```


```json
{"action": "select", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_selected": true, "group_names": ["Liella!"]}
```


```json
{"card_count": 1, "cards": ["PL!SP-bp4-023-L | Dazzling Game (ab#1)"], "effect": {"action": "set_blade_type", "blade_type": "紫ブレード", "duration": "live_end"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "set_blade_type", "blade_type": "紫ブレード", "duration": "live_end"}
```

- エールによって公開される自分のカードが持つ[桃ブレード]、[赤ブレード]、[黄ブレード]、[緑ブレード]、[青ブレード]、{icon_b_all.png|ALLブレード}は、すべて[紫ブレード]になる (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-024-L | ノンフィクション!! (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["Liella!"], "operator": ">", "position": "center", "target": "both", "type": "comparison_condition"}, "group_names": ["Liella!"], "operation": "add", "position": "center", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["Liella!"], "operator": ">", "position": "center", "target": "both", "type": "comparison_condition"}, "group_names": ["Liella!"], "operation": "add", "position": "center", "self_target": true, "value": 1}
```

- 自分のセンターエリアにいる『Liella!』のメンバーのコストが、相手のセンターエリアにいるメンバーより高い場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "comparison_target": "opponent", "comparison_type": "cost", "group_names": ["Liella!"], "operator": ">", "position": "center", "target": "both", "type": "comparison_condition"}
```

- 自分のセンターエリアにいる『Liella!』のメンバーのコストが、相手のセンターエリアにいるメンバーより高い場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-024-L | ノンフィクション!! (ab#1)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 3, "group_names": ["Liella!"], "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "position": "left_side", "resource_type": "heart_02", "target": "self", "type": "comparison_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02"], "position": "left_side", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 3, "group_names": ["Liella!"], "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "position": "left_side", "resource_type": "heart_02", "target": "self", "type": "comparison_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart02"], "position": "left_side", "resource": "blade"}
```

- 自分のステージの左サイドエリアにいる『Liella!』のメンバーが{heart_02.png|heart02}を3つ以上持つ場合、そのメンバーは、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "count": 3, "group_names": ["Liella!"], "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "position": "left_side", "resource_type": "heart_02", "target": "self", "type": "comparison_condition"}
```

- 自分のステージの左サイドエリアにいる『Liella!』のメンバーが{heart_02.png|heart02}を3つ以上持つ場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-025-L | Special Color (ab#0)"], "effect": {"action": "set_blade_count", "blade_limit": 3, "blade_limit_operator": "==", "card_type": "member_card", "count": 3, "duration": "live_end", "group_names": ["Liella!"], "original_value": true, "position": "center", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "set_blade_count", "blade_limit": 3, "blade_limit_operator": "==", "card_type": "member_card", "count": 3, "duration": "live_end", "group_names": ["Liella!"], "original_value": true, "position": "center", "target": "self"}
```

- 自分のステージのセンターエリアにいる『Liella!』のメンバーが元々持つ{icon_blade.png|ブレード}の数は3つになる (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-025-L | Special Color (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "center", "temporal": "this_turn", "type": "temporal_condition"}, "group_names": ["Liella!"], "operation": "add", "position": "center", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "center", "temporal": "this_turn", "type": "temporal_condition"}, "group_names": ["Liella!"], "operation": "add", "position": "center", "self_target": true, "value": 1}
```

- 自分のステージのセンターエリアにいる『Liella!』のメンバーが、このターン中に移動している場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-026-L | Wish Song (ab#0)"], "effect": {"action": "modify_score", "condition": {"distinct": "card_name", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "location_condition"}, "group_names": ["Liella!"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"distinct": "card_name", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "location_condition"}, "group_names": ["Liella!"], "operation": "add", "self_target": true, "value": 1}
```

- エールにより公開された自分のカードの中に名前が異なる『Liella!』のメンバーカードが5枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-026-L | Wish Song (ab#1)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"count": 11, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"count": 11, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}}
```

- 自分のエネルギーが11枚以上ある場合、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-027-L | Chance Day, Chance Way! (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["Liella!"], "multiple_targets": true, "optional": true, "parenthetical": ["メンバーをそれぞれ好きなエリアに移動させる。この効果で1つのエリアに2人以上のメンバーを移動させることはできない。"], "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"all_members": true, "card_type": "member_card", "group_names": ["Liella!"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["Liella!"], "multiple_targets": true, "optional": true, "parenthetical": ["メンバーをそれぞれ好きなエリアに移動させる。この効果で1つのエリアに2人以上のメンバーを移動させることはできない。"], "target": "self"}
```

- 自分のステージにいるメンバーが『Liella!』のみの場合、自分のステージにいるメンバーをフォーメーションチェンジしてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp4-028-L | DAISUKI FULL POWER (ab#0)"], "effect": {"action": "modify_score", "condition": {"state": "active", "type": "energy_state_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"state": "active", "type": "energy_state_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- アクティブ状態の自分のエネルギーがある場合、このカードのスコアを+1する (x1)

```json
{"state": "active", "type": "energy_state_condition"}
```

- アクティブ状態の自分のエネルギーがある場合 (x1)

```json
{"card_count": 1, "cards": ["LL-bp4-001-R＋ | 絢瀬絵里&朝香果林&葉月 恋 (ab#0)"], "effect": {"action": "look_and_select", "followup_action": {"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "cost_from_revealed": true, "cost_limit_operator": "<=", "count": 3, "original_value": true, "state_change": "wait", "target": "opponent"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "original_value": true, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "original_value": true, "reveal": true}}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"action": "look_and_select", "followup_action": {"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "cost_from_revealed": true, "cost_limit_operator": "<=", "count": 3, "original_value": true, "state_change": "wait", "target": "opponent"}, "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "original_value": true, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "original_value": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から「絢瀬絵里」か「朝香果林」か「葉月恋」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。その後、相手のステージにいる、これにより公開したカードのコスト以下で、かつ元々持つ{icon_blade.png|ブレード}の数が3つ以下のメンバーをすべてウェイトにする (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "optional": true, "original_value": true, "reveal": true}
```


```json
{"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "cost_from_revealed": true, "cost_limit_operator": "<=", "count": 3, "original_value": true, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいる、これにより公開したカードのコスト以下で、かつ元々持つ{icon_blade.png|ブレード}の数が3つ以下のメンバーをすべてウェイトにする (x1)

```json
{"card_count": 1, "cards": ["PL!N-pb1-034-N | 三船栞子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart03", "heart04", "heart05"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart03", "heart04", "heart05"], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart03", "heart04", "heart05"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart03", "heart04", "heart05"], "original_value": true}
```

- {heart_03.png|heart03}か{heart_04.png|heart04}か{heart_05.png|heart05}のうち1つを選ぶ。ライブ終了時まで、このメンバーが元々持つハートは選んだハートになる (x1)

```json
{"action": "select", "count": 1, "heart_colors": ["heart03", "heart04", "heart05"]}
```

- {heart_03.png|heart03}か{heart_04.png|heart04}か{heart_05.png|heart05}のうち1つを選ぶ (x1)

```json
{"card_count": 1, "cards": ["PL!N-pb1-036-N | 鐘 嵐珠 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart02", "heart06"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart01", "heart02", "heart06"], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart01", "heart02", "heart06"]}, {"action": "set_heart_type", "card_type": "member_card", "duration": "live_end", "original_value": true, "self_target": true}], "heart_colors": ["heart01", "heart02", "heart06"], "original_value": true}
```

- {heart_01.png|heart01}か{heart_02.png|heart02}か{heart_06.png|heart06}のうち1つを選ぶ。ライブ終了時まで、このメンバーが元々持つハートは選んだハートになる (x1)

```json
{"action": "select", "count": 1, "heart_colors": ["heart01", "heart02", "heart06"]}
```

- {heart_01.png|heart01}か{heart_02.png|heart02}か{heart_06.png|heart06}のうち1つを選ぶ (x1)

```json
{"card_count": 1, "cards": ["PL!N-pb1-037-L | Cara Tesoro (ab#0)"], "effect": {"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"state": "wait", "type": "state_condition"}, "group_names": ["虹ヶ咲"], "primary_effect": {"action": "modify_score", "card_type": "member_card", "group_names": ["虹ヶ咲"], "operation": "add", "self_target": true, "target": "self", "value": 1}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_alternative", "alternative_effect": {"action": "modify_score", "operation": "add", "value": 2}, "condition": {"state": "wait", "type": "state_condition"}, "group_names": ["虹ヶ咲"], "primary_effect": {"action": "modify_score", "card_type": "member_card", "group_names": ["虹ヶ咲"], "operation": "add", "self_target": true, "target": "self", "value": 1}}
```

- このターン、自分の『虹ヶ咲』のカードの効果によってウェイト状態の自分のエネルギーをアクティブにしていた場合、このカードのスコアを+1する。さらに、自分の『虹ヶ咲』のカードの効果によって自分のステージにいるウェイト状態のメンバーもアクティブにしていた場合、代わりにスコアを+2する (x1)

```json
{"action": "modify_score", "card_type": "member_card", "group_names": ["虹ヶ咲"], "operation": "add", "self_target": true, "target": "self", "value": 1}
```

- このカードのスコアを+1する。さらに、自分の『虹ヶ咲』のカードの効果によって自分のステージにいるウェイト状態のメンバーもアクティブにしていた場合、 (x1)

```json
{"card_count": 1, "cards": ["PL!N-pb1-038-L | PHOENIX (ab#0)"], "effect": {"action": "modify_score", "condition": {"location": "success_live_card_zone", "target": "self", "temporal": "during_live", "type": "temporal_condition"}, "group_names": ["虹ヶ咲"], "heart_colors": ["heart01"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"location": "success_live_card_zone", "target": "self", "temporal": "during_live", "type": "temporal_condition"}, "group_names": ["虹ヶ咲"], "heart_colors": ["heart01"], "operation": "add", "self_target": true, "value": 1}
```

- 自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{heart_01.png|heart01}が4の『虹ヶ咲』のライブカードがある場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!N-pb1-039-L | Stellar Stream (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"location": "success_live_card_zone", "target": "self", "temporal": "during_live", "type": "temporal_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "group_names": ["虹ヶ咲"], "heart_colors": ["heart06"], "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"location": "success_live_card_zone", "target": "self", "temporal": "during_live", "type": "temporal_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "group_names": ["虹ヶ咲"], "heart_colors": ["heart06"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{heart_01.png|heart01}が3の『虹ヶ咲』のライブカードがある場合、ライブ終了時まで、自分のステージにいる{heart_06.png|heart06}を持つ『虹ヶ咲』のメンバー1人は{heart_06.png|heart06}{heart_06.png|heart06}{heart_06.png|heart06}{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-pb1-042-L | Eternalize Love!! (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["虹ヶ咲"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 3, "group_names": ["虹ヶ咲"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "group_names": ["虹ヶ咲"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 3, "group_names": ["虹ヶ咲"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 自分のステージに同じ名前の『虹ヶ咲』のメンバーが2人以上いる場合、このカードを成功させるための必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"card_type": "member_card", "count": 2, "group_names": ["虹ヶ咲"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに同じ名前の『虹ヶ咲』のメンバーが2人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-010-N | 高坂穂乃果 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["A-RISE"], "source": "discard", "target": "self"}], "group_names": ["A-RISE"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["A-RISE"], "source": "discard", "target": "self"}], "group_names": ["A-RISE"]}
```

- 自分のデッキの上からカードを3枚控え室に置く。その後、自分の控え室から『A-RISE』のメンバーカードを1枚手札に加える (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["A-RISE"], "source": "discard", "target": "self"}
```

- 自分の控え室から『A-RISE』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-011-N | 絢瀬絵里 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart04", "heart05", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}], "heart_colors": ["heart04", "heart05", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "heart_colors": ["heart04", "heart05", "heart06"]}, {"action": "gain_resource", "count": 1, "duration": "live_end", "location": "success_live_zone", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}], "heart_colors": ["heart04", "heart05", "heart06"]}
```

- {heart_04.png|heart04}か{heart_05.png|heart05}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る (x1)

```json
{"action": "select", "count": 1, "heart_colors": ["heart04", "heart05", "heart06"]}
```

- {heart_04.png|heart04}か{heart_05.png|heart05}か{heart_06.png|heart06}のうち、1つを選ぶ (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-013-N | 園田海未 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!-bp5-014-N | 星空凛 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "heart_colors": ["heart05", "heart06"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart05", "heart06"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "heart_colors": ["heart05", "heart06"], "look_action": {"action": "look_at", "count": 4, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart05", "heart06"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを4枚見る。その中からハートに{heart_05.png|heart05}か{heart_06.png|heart06}を持つメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "heart_colors": ["heart05", "heart06"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!-bp5-020-L | Wonder zone (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "group_names": ["μ's"], "position": "center", "target": "self", "type": "group_condition"}, "count": 3, "group_names": ["μ's"], "heart_colors": ["heart00"], "operation": "decrease", "per_unit": true, "per_unit_count": 2, "per_unit_type": "つ", "position": "center", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"card_type": "member_card", "group_names": ["μ's"], "position": "center", "target": "self", "type": "group_condition"}, "count": 3, "group_names": ["μ's"], "heart_colors": ["heart00"], "operation": "decrease", "per_unit": true, "per_unit_count": 2, "per_unit_type": "つ", "position": "center", "self_target": true}
```

- 自分のセンターエリアに『μ's』のメンバーがいる場合、そのメンバーが持つ{heart_03.png|heart03}2つにつき、このカードの必要ハートを{heart_00.png|heart0}減らす。この能力では{heart_00.png|heart0}は3つまでしか減らない (x1)

```json
{"card_type": "member_card", "group_names": ["μ's"], "position": "center", "target": "self", "type": "group_condition"}
```

- 自分のセンターエリアに『μ's』のメンバーがいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-021-L | SUNNY DAY SONG (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck", "target": "both"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "count": 1, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["μ's"]}, {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}, "count": 1, "duration": "live_end", "group_names": ["μ's"], "heart_colors": ["heart03"], "resource": "heart", "target": "self", "target_count": 1}, {"action": "modify_score", "condition": {"conditions": [{"card_type": "member_card", "count": 3, "operator": ">=", "type": "card_count_condition", "unit": "人"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "operator": "and", "type": "compound"}, "group_names": ["μ's"], "multiple_targets": true, "operation": "add", "self_target": true, "value": 1}], "group_names": ["μ's"], "heart_colors": ["heart03"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck", "target": "both"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "count": 1, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["μ's"]}, {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}, "count": 1, "duration": "live_end", "group_names": ["μ's"], "heart_colors": ["heart03"], "resource": "heart", "target": "self", "target_count": 1}, {"action": "modify_score", "condition": {"conditions": [{"card_type": "member_card", "count": 3, "operator": ">=", "type": "card_count_condition", "unit": "人"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "operator": "and", "type": "compound"}, "group_names": ["μ's"], "multiple_targets": true, "operation": "add", "self_target": true, "value": 1}], "group_names": ["μ's"], "heart_colors": ["heart03"]}
```

- 自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札を1枚控え室に置く。2人以上いる場合、さらに自分のステージにいる『μ's』のメンバー1人は、ライブ終了時まで、{heart_03.png|heart03}を得る。3人以上おり、かつそれぞれ名前が異なる場合、さらにこのカードのスコアを+1する (x1)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck", "target": "both"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "count": 1, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["μ's"]}
```

- 自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札を1枚控え室に置く (x1)

```json
{"card_type": "member_card", "count": 1, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージにメンバーが1人以上いる場合 (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck", "target": "both"}
```

- 自分と相手はカードを1枚引き (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}, "count": 1, "duration": "live_end", "group_names": ["μ's"], "heart_colors": ["heart03"], "resource": "heart", "target": "self", "target_count": 1}
```

- 2人以上いる場合、自分のステージにいる『μ's』のメンバー1人は、ライブ終了時まで、{heart_03.png|heart03}を得る (x1)

```json
{"action": "modify_score", "condition": {"conditions": [{"card_type": "member_card", "count": 3, "operator": ">=", "type": "card_count_condition", "unit": "人"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "operator": "and", "type": "compound"}, "group_names": ["μ's"], "multiple_targets": true, "operation": "add", "self_target": true, "value": 1}
```

- 3人以上おり、かつそれぞれ名前が異なる場合、このカードのスコアを+1する (x1)

```json
{"conditions": [{"card_type": "member_card", "count": 3, "operator": ">=", "type": "card_count_condition", "unit": "人"}, {"distinct": "card_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "card_name", "operator": "and", "type": "compound"}
```

- 3人以上おり、かつそれぞれ名前が異なる場合 (x1)

```json
{"card_type": "member_card", "count": 3, "operator": ">=", "type": "card_count_condition", "unit": "人"}
```

- 3人以上おり、 (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-022-L | A song for You! You? You!! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self", "value": 2}, {"action": "modify_required_hearts", "count": 4, "heart_colors": ["heart00", "heart01", "heart03", "heart06"], "location": "success_live_zone", "operation": "increase", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self"}], "heart_colors": ["heart01", "heart03", "heart06", "heart00"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "location": "success_live_zone", "operation": "add", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self", "value": 2}, {"action": "modify_required_hearts", "count": 4, "heart_colors": ["heart00", "heart01", "heart03", "heart06"], "location": "success_live_zone", "operation": "increase", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self"}], "heart_colors": ["heart01", "heart03", "heart06", "heart00"]}
```

- 自分の成功ライブカード置き場にあるカード1枚につき、このカードのスコアを+2し、必要ハートを{heart_01.png|heart01}{heart_03.png|heart03}{heart_06.png|heart06}{heart_00.png|heart0}増やす (x1)

```json
{"action": "modify_required_hearts", "count": 4, "heart_colors": ["heart00", "heart01", "heart03", "heart06"], "location": "success_live_zone", "operation": "increase", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "target": "self"}
```

- 必要ハートを{heart_01.png|heart01}{heart_03.png|heart03}{heart_06.png|heart06}{heart_00.png|heart0}増やす (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-023-L | 乙姫心で恋宮殿 (ab#0)"], "effect": {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "location": "stage", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart00"], "location": "stage", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "self_target": true, "target": "self"}
```

- 自分のステージにいる{heart_01.png|heart01}と{heart_06.png|heart06}以外の色のハートを持つメンバー1人につき、このカードの必要ハートを{heart_00.png|heart0}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-010-N | 高海千歌 (ab#0)"], "effect": {"action": "modify_required_hearts_global", "condition": {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}, "heart_colors": ["heart00"], "operation": "increase", "target": "opponent", "value": 1}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "modify_required_hearts_global", "condition": {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}, "heart_colors": ["heart00"], "operation": "increase", "target": "opponent", "value": 1}
```

- 自分のステージにいるメンバーが持つハートに{heart_02.png|heart02}が合計5つ以上ある場合、相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚は、成功させるための必要ハートが{heart_00.png|heart0}多くなる (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のステージにいるメンバーが持つハートに{heart_02.png|heart02}が合計5つ以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-011-N | 桜内梨子 (ab#0)"], "effect": {"action": "modify_required_hearts_global", "condition": {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}, "heart_colors": ["heart00"], "operation": "increase", "target": "opponent", "value": 1}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "modify_required_hearts_global", "condition": {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}, "heart_colors": ["heart00"], "operation": "increase", "target": "opponent", "value": 1}
```

- 自分のステージにいるメンバーが持つハートに{heart_05.png|heart05}が合計5つ以上ある場合、相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚は、成功させるための必要ハートが{heart_00.png|heart0}多くなる (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart05"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のステージにいるメンバーが持つハートに{heart_05.png|heart05}が合計5つ以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-013-N | 黒澤ダイヤ (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "count": 4, "heart_colors": ["heart04"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "count": 4, "heart_colors": ["heart04"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart04"], "resource": "heart"}
```

- 自分のライブカード置き場にあるカードの必要ハートに含まれる{heart_04.png|heart04}の合計が4以上の場合、ライブ終了時まで、{heart_04.png|heart04}を得る (x1)

```json
{"aggregate": "total", "card_type": "live_card", "count": 4, "heart_colors": ["heart04"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "location_condition"}
```

- 自分のライブカード置き場にあるカードの必要ハートに含まれる{heart_04.png|heart04}の合計が4以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-015-N | 津島善子 (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "count": 10, "destination": "discard", "source": "deck_top", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!S-bp5-016-N | 国木田花丸 (ab#0)"], "effect": {"action": "gain_resource", "all": true, "condition": {"card_type": "member_card", "comparison_source": "opponent", "comparison_target": "self", "comparison_type": "cost", "location": "ステージ", "operator": ">", "type": "all_cost_comparison_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "all": true, "condition": {"card_type": "member_card", "comparison_source": "opponent", "comparison_target": "self", "comparison_type": "cost", "location": "ステージ", "operator": ">", "type": "all_cost_comparison_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- 相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "comparison_source": "opponent", "comparison_target": "self", "comparison_type": "cost", "location": "ステージ", "operator": ">", "type": "all_cost_comparison_condition"}
```

- 相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-017-N | 小原鞠莉 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "count": 4, "heart_colors": ["heart05"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "count": 4, "heart_colors": ["heart05"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "location_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "heart"}
```

- 自分のライブカード置き場にあるカードの必要ハートに含まれる{heart_05.png|heart05}の合計が4以上の場合、ライブ終了時まで、{heart_05.png|heart05}を得る (x1)

```json
{"aggregate": "total", "card_type": "live_card", "count": 4, "heart_colors": ["heart05"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "location_condition"}
```

- 自分のライブカード置き場にあるカードの必要ハートに含まれる{heart_05.png|heart05}の合計が4以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-019-L | not ALONE not HITORI (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "condition": {"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, "count": 2, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "max": true, "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "condition": {"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, "count": 2, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "max": true, "source": "revealed_cards", "target": "self"}
```

- 自分か相手の成功ライブカード置き場にカードが2枚以上ある場合、エールにより公開された自分のカードの中から、メンバーカードを2枚まで手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-020-L | Landing action Yeah!! (ab#0)"], "effect": {"action": "modify_score", "condition": {"count": 3, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"count": 3, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- 自分が余剰ハートを3つ以上持っている場合、それらをすべて失い、このカードのスコアを+1する (x1)

```json
{"count": 3, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}
```

- 自分が余剰ハートを3つ以上持っている場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-013-N | 上原歩夢 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "location": "stage", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "location": "stage", "resource_type": "energy", "target": "self", "type": "comparison_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart01"], "resource": "heart"}
```

- 自分のステージにエネルギーカードが下にあるメンバーがいる場合、ライブ終了時まで、{heart_01.png|heart01}を得る (x1)

```json
{"card_type": "member_card", "location": "stage", "resource_type": "energy", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにエネルギーカードが下にあるメンバーがいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-015-N | 桜坂しずく (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "resource": "blade"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 2, "duration": "live_end", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "resource": "blade"}
```

- 自分のステージにいるメンバーが持つハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}がすべてある場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-021-N | 天王寺璃奈 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck", "optional": true, "position": {"position": "4"}, "source": "discard", "target": "self"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck", "optional": true, "position": {"position": "4"}, "source": "discard", "target": "self"}]}
```

- 自分のデッキの上からカードを2枚控え室に置く。その後、自分の控え室からライブカード1枚を自分のデッキの一番上から4枚目に置いてもよい (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "deck", "optional": true, "position": {"position": "4"}, "source": "discard", "target": "self"}
```

- 自分の控え室からライブカード1枚を自分のデッキの一番上から4枚目に置いてもよい (x1)

```json
{"position": "4"}
```


```json
{"card_count": 1, "cards": ["PL!N-bp5-026-L | TOKIMEKI Runners (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "target": "self", "type": "location_condition"}, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "target": "self", "type": "location_condition"}, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージにいるメンバーが持つハートの中に{heart_01.png|heart01}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_06.png|heart06}がすべてある場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-026-L | TOKIMEKI Runners (ab#1)"], "effect": {"action": "move_cards", "card_type": "card", "condition": {"comparison_type": "score", "type": "comparison_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "card", "condition": {"comparison_type": "score", "type": "comparison_condition"}, "count": 1, "destination": "hand", "group_names": ["虹ヶ咲"], "source": "discard", "target": "self"}
```

- このカードのスコアが3の場合、自分の控え室にある『虹ヶ咲』のカードを1枚手札に加える (x1)

```json
{"comparison_type": "score", "type": "comparison_condition"}
```

- このカードのスコアが3の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-027-L | ミラクル STAY TUNE！ (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, {"count": 3, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}], "distinct": "card_name", "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "distinct": "card_name", "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, {"count": 3, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}], "distinct": "card_name", "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "distinct": "card_name", "operation": "add", "self_target": true, "value": 1}
```

- 自分か相手の成功ライブカード置き場にカードが2枚以上あり、かつ自分のステージに名前の異なるメンバーが3人以上いる場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "conditions": [{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, {"count": 3, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}], "distinct": "card_name", "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}
```

- 自分か相手の成功ライブカード置き場にカードが2枚以上あり、かつ自分のステージに名前の異なるメンバーが3人以上いる場合 (x1)

```json
{"count": 3, "distinct": "card_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージに名前の異なるメンバーが3人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-028-L | CHASE! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "operation": "add", "self_target": true, "value": 2}, {"action": "modify_required_hearts", "count": 5, "heart_colors": ["heart02"], "operation": "set"}], "condition": {"card_type": "member_card", "count": 4, "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "resource_type": "heart_02", "target": "self", "type": "comparison_condition"}, "heart_colors": ["heart02"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "operation": "add", "self_target": true, "value": 2}, {"action": "modify_required_hearts", "count": 5, "heart_colors": ["heart02"], "operation": "set"}], "condition": {"card_type": "member_card", "count": 4, "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "resource_type": "heart_02", "target": "self", "type": "comparison_condition"}, "heart_colors": ["heart02"]}
```

- 自分のステージに{heart_02.png|heart02}を4つ以上持つメンバーがいる場合、このカードのスコアを+2し、必要ハートは{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}になる (x1)

```json
{"card_type": "member_card", "count": 4, "heart_colors": ["heart02"], "location": "stage", "operator": ">=", "resource_type": "heart_02", "target": "self", "type": "comparison_condition"}
```

- 自分のステージに{heart_02.png|heart02}を4つ以上持つメンバーがいる場合 (x1)

```json
{"action": "modify_score", "operation": "add", "self_target": true, "value": 2}
```

- このカードのスコアを+2し (x1)

```json
{"action": "modify_required_hearts", "count": 5, "heart_colors": ["heart02"], "operation": "set"}
```

- 必要ハートは{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}{heart_02.png|heart02}になる (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-029-L | 無敵級*ビリーバー (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "reveal", "all": true, "count": 4, "source": "deck_top", "target": "self"}, {"action": "select", "all": true, "characters": ["中須かすみ"], "count": 1, "quoted_text": {"quoted_type": "character"}}, {"action": "gain_resource", "all": true, "characters": ["中須かすみ"], "count": 1, "duration": "live_end", "multiple_targets": true, "quoted_text": {"quoted_type": "character"}, "resource": "heart", "target": "self", "target_count": 1}, {"action": "move_cards", "all": true, "card_type": "card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}], "all": true, "condition": {"characters": ["中須かすみ"], "location": "stage", "target": "self", "type": "location_condition"}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "reveal", "all": true, "count": 4, "source": "deck_top", "target": "self"}, {"action": "select", "all": true, "characters": ["中須かすみ"], "count": 1, "quoted_text": {"quoted_type": "character"}}, {"action": "gain_resource", "all": true, "characters": ["中須かすみ"], "count": 1, "duration": "live_end", "multiple_targets": true, "quoted_text": {"quoted_type": "character"}, "resource": "heart", "target": "self", "target_count": 1}, {"action": "move_cards", "all": true, "card_type": "card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}], "all": true, "condition": {"characters": ["中須かすみ"], "location": "stage", "target": "self", "type": "location_condition"}}
```

- 自分のステージに「中須かすみ」がいる場合、自分のデッキの上からカードを4枚公開する。自分はそれらの中から「中須かすみ」のカードを1枚選ぶ。ライブ終了時まで、自分のステージにいる「中須かすみ」1人は、これにより選んだカードが持つ色のハートを1つずつ得る。公開したカードをすべて控え室に置く (x1)

```json
{"characters": ["中須かすみ"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「中須かすみ」がいる場合 (x1)

```json
{"action": "reveal", "all": true, "count": 4, "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを4枚公開する (x1)

```json
{"action": "select", "all": true, "characters": ["中須かすみ"], "count": 1, "quoted_text": {"quoted_type": "character"}}
```

- 自分はそれらの中から「中須かすみ」のカードを1枚選ぶ (x1)

```json
{"action": "gain_resource", "all": true, "characters": ["中須かすみ"], "count": 1, "duration": "live_end", "multiple_targets": true, "quoted_text": {"quoted_type": "character"}, "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいる「中須かすみ」1人は、これにより選んだカードが持つ色のハートを1つずつ得る (x1)

```json
{"action": "move_cards", "all": true, "card_type": "card", "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}
```

- 公開したカードをすべて控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "heart_type": "all", "location": "stage", "negation": true, "type": "location_condition"}, "count": 1, "duration": "live_end", "resource": "heart", "trigger_condition": {"card_type": "member_card", "location": "stage", "target": "self", "type": "location_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "heart_type": "all", "location": "stage", "negation": true, "type": "location_condition"}, "count": 1, "duration": "live_end", "resource": "heart", "trigger_condition": {"card_type": "member_card", "location": "stage", "target": "self", "type": "location_condition"}, "trigger_type": "each_time"}
```

- 自分のステージにいるメンバーの{live_start.png|ライブ開始時}能力が解決するたび、そのメンバーが{icon_all.png|ハート}を持たない場合、ライブ終了時まで、そのメンバーは{icon_all.png|ハート}を得る (x1)

```json
{"card_type": "member_card", "heart_type": "all", "location": "stage", "negation": true, "type": "location_condition"}
```

- そのメンバーが{icon_all.png|ハート}を持たない場合 (x1)

```json
{"card_count": 1, "cards": ["PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#1)"], "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "trigger_condition": {"card_type": "member_card", "location": "stage", "target": "self", "type": "location_condition"}, "trigger_type": "each_time"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck", "trigger_condition": {"card_type": "member_card", "location": "stage", "target": "self", "type": "location_condition"}, "trigger_type": "each_time"}
```

- 自分のステージにいるメンバーの{live_success.png|ライブ成功時}能力が解決するたび、カードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-012-N | 澁谷かのん (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "count": 8, "group_names": ["Liella!"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "card_type": "live_card", "count": 8, "group_names": ["Liella!"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}
```

- {heart_03.png|heart03}を得る (x1)

```json
{"aggregate": "total", "card_type": "live_card", "count": 8, "group_names": ["Liella!"], "location": "live_card_zone", "operator": ">=", "target": "self", "type": "group_condition"}
```

- 自分のライブカード置き場に必要ハートの合計が8以上の『Liella!』のライブカードがあるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-013-N | 唐 可可 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["SunnyPassion", "Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["SunnyPassion", "Liella!"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["SunnyPassion", "Liella!"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["SunnyPassion", "Liella!"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『SunnyPassion』のメンバーカードかブレードハートを持つ『Liella!』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["SunnyPassion", "Liella!"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!SP-bp5-014-N | 嵐 千砂都 (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "count": 1, "destination": "hand", "exclude_self": true, "source": "deck"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "draw_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "count": 1, "destination": "hand", "exclude_self": true, "source": "deck"}
```

- このターン、自分のステージにいるほかのメンバーがエリアを移動している場合、カードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-015-N | 平安名すみれ (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "center", "count": 2, "duration": "live_end", "position": "center", "resource": "blade"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "gain_resource", "activation_position": "center", "count": 2, "duration": "live_end", "position": "center", "resource": "blade"}
```

- ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-016-N | 葉月 恋 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"count": 10, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}
```

- {heart_06.png|heart06}{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-017-N | 桜小路きな子 (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "conditional": true, "duration": "as_long_as", "location": "hand", "operation": "subtract", "value": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "conditional": true, "duration": "as_long_as", "location": "hand", "operation": "subtract", "value": 2}
```

- 手札にあるこのメンバーカードのコストは2減る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-020-N | 鬼塚夏美 (ab#1)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_count": 1, "cards": ["PL!SP-bp5-021-N | ウィーン・マルガレーテ (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "move_cards", "card_type": "energy_card", "condition": {"count": 6, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "move_cards", "card_type": "energy_card", "condition": {"count": 6, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "energy_zone", "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 自分のエネルギーが6枚以上ある場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く (x1)

```json
{"count": 6, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分のエネルギーが6枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-023-L | Shooting Voice!! (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "live_card", "conditions": [{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "operation": "add", "self_target": true, "value": 2}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "live_card", "conditions": [{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}, "operation": "add", "self_target": true, "value": 2}
```

- 自分か相手の成功ライブカード置き場にカードが2枚以上あり、かつエールにより公開された自分のカードの中に{icon_score.png|スコア}を持つライブカードが1枚以上ある場合、このカードのスコアを+2する (x1)

```json
{"card_type": "live_card", "conditions": [{"count": 2, "location": "success_live_zone", "operator": ">=", "target": "either", "type": "card_count_condition"}, {"card_type": "live_card", "count": 1, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}], "location": "success_live_card_zone", "operator": "and", "target": "both", "type": "compound"}
```

- 自分か相手の成功ライブカード置き場にカードが2枚以上あり、かつエールにより公開された自分のカードの中に{icon_score.png|スコア}を持つライブカードが1枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-024-L | MIRACLE NEW STORY (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "all": true, "count": 1, "heart_colors": ["heart01", "heart02", "heart06"]}, {"action": "sequential", "actions": [{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end"}, {"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "heart"}], "all": true, "duration": "live_end"}], "all": true, "heart_colors": ["heart01", "heart02", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "all": true, "count": 1, "heart_colors": ["heart01", "heart02", "heart06"]}, {"action": "sequential", "actions": [{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end"}, {"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "heart"}], "all": true, "duration": "live_end"}], "all": true, "heart_colors": ["heart01", "heart02", "heart06"]}
```

- {heart_01.png|heart01}か{heart_02.png|heart02}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいる、このターン中にエリアを移動しているすべてのメンバーは、選んだハートを1つ得る (x1)

```json
{"action": "select", "all": true, "count": 1, "heart_colors": ["heart01", "heart02", "heart06"]}
```

- {heart_01.png|heart01}か{heart_02.png|heart02}か{heart_06.png|heart06}のうち、1つを選ぶ (x1)

```json
{"action": "sequential", "actions": [{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end"}, {"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "heart"}], "all": true, "duration": "live_end"}
```

- 自分のステージにいる、このターン中にエリアを移動しているすべてのメンバーは、選んだハートを1つ得る (x1)

```json
{"action": "position_change", "all": true, "card_type": "member_card", "duration": "live_end"}
```

- このターン中にエリアを移動しているすべてのメンバーは (x1)

```json
{"action": "gain_resource", "all": true, "count": 1, "duration": "live_end", "resource": "heart"}
```

- 選んだハートを1つ得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-025-L | 常夏☆サンシャイン (ab#0)"], "cost": {"any_number": true, "count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "modify_score", "operation": "add", "per_unit": true, "per_unit_count": 4, "per_unit_type": "つ", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"any_number": true, "count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}
```

- {icon_energy.png|E}を好きな数支払ってもよい (x1)

```json
{"action": "modify_score", "operation": "add", "per_unit": true, "per_unit_count": 4, "per_unit_type": "つ", "self_target": true, "value": 1}
```

- これにより支払った{icon_energy.png|E}4つにつき、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-026-L | Let's be ONE (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 11, "group_names": ["Liella!"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, "group_names": ["Liella!"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 11, "group_names": ["Liella!"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}, "group_names": ["Liella!"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージにいる『Liella!』のメンバーが持つハートの総数が11以上の場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "count": 11, "group_names": ["Liella!"], "location": "stage", "operator": ">=", "target": "self", "type": "group_condition"}
```

- 自分のステージにいる『Liella!』のメンバーが持つハートの総数が11以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp1-002-RM | 村野さやか (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 15, "cost_limit_operator": "<=", "count": 1, "destination": "same_area", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 1, "cards": ["PL!HS-bp5-013-N | 徒町 小鈴 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}]}
```

- 自分のデッキの上からカードを3枚控え室に置く。それらがすべてメンバーカードの場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 2, "duration": "live_end", "resource": "blade"}
```

- それらがすべてメンバーカードの場合、ライブ終了時まで、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-014-N | 安養寺 姫芽 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}
```

- このメンバーがエリアを移動したとき、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-016-N | 桂城 泉 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}
```

- {heart_06.png|heart06}を得る (x1)

```json
{"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "opponent", "type": "card_count_condition", "unit": "人"}
```

- 相手のステージにウェイト状態のメンバーが2人以上いるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-017-L | Dream Believers（104期Ver.） (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, {"distinct": "group_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "group_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["蓮ノ空"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, {"distinct": "group_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "group_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "group_names": ["蓮ノ空"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージに『蓮ノ空』のメンバー1人を含むメンバーが2人以上おり、かつそれらのメンバーのユニット名がそれぞれ異なる場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "conditions": [{"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, {"distinct": "group_name", "location": "stage", "target": "self", "type": "location_condition"}], "distinct": "group_name", "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージに『蓮ノ空』のメンバー1人を含むメンバーが2人以上おり、かつそれらのメンバーのユニット名がそれぞれ異なる場合 (x1)

```json
{"distinct": "group_name", "location": "stage", "target": "self", "type": "location_condition"}
```

- それらのメンバーのユニット名がそれぞれ異なる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-018-L | AURORA FLOWER (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージに名前とコストが両方ともそれぞれ異なるメンバーが3人以上いる場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "count": 3, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに名前とコストが両方ともそれぞれ異なるメンバーが3人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-019-L | ハナムスビ (ab#0)"], "effect": {"action": "modify_required_hearts", "count": 2, "group_names": ["蓮ノ空"], "heart_colors": ["heart04"], "location": "live_card_zone", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "count": 2, "group_names": ["蓮ノ空"], "heart_colors": ["heart04"], "location": "live_card_zone", "operation": "decrease", "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "self_target": true, "target": "self"}
```

- 自分のライブカード置き場にあるこのカード以外の『蓮ノ空』のカード1枚につき、このカードの必要ハートを{heart_04.png|heart04}{heart_04.png|heart04}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-020-L | バアドケージ (ab#0)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["蓮ノ空"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 2, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["蓮ノ空"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージにコスト10以上の『蓮ノ空』のメンバーが2人以上いる場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-021-L | ジョーショーキリュー (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart01"], "heart_selection": true, "original_value": true, "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart01"], "heart_selection": true, "original_value": true, "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいる『蓮ノ空』のメンバー1人が元々持つハートをすべて{heart_01.png|heart01}にする (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-021-L | ジョーショーキリュー (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "count": 3, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["みらくらぱーく！"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "count": 3, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "group_names": ["みらくらぱーく！"], "operation": "add", "self_target": true, "value": 1}
```

- 自分のステージに『みらくらぱーく！』のメンバーが3人以上いる場合、このカードのスコアを+1する (x1)

```json
{"card_type": "member_card", "count": 3, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに『みらくらぱーく！』のメンバーが3人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp5-022-L | Retrofuture (ab#0)"], "cost": {"count": 2, "energy": 2, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "choice", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 9, "cost_total": 9, "count": 9, "group_names": ["EdelNote"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "group_names": ["EdelNote"], "heart_colors": ["heart06"], "options": [{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "source": "discard", "target": "self"}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart06"], "operation": "decrease", "self_target": true}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "choice", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 9, "cost_total": 9, "count": 9, "group_names": ["EdelNote"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "group_names": ["EdelNote"], "heart_colors": ["heart06"], "options": [{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "source": "discard", "target": "self"}, {"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart06"], "operation": "decrease", "self_target": true}]}
```

- 自分のステージにコスト9以上の『EdelNote』のメンバーがいる場合、以下から1つを選ぶ。
・自分の控え室からコスト4以下の『EdelNote』のメンバーカードを1枚、メンバーのいないエリアに登場させる。
・このカードの必要ハートを{heart_06.png|heart06}減らす (x1)

```json
{"card_type": "member_card", "comparison_type": "cost", "cost_limit": 9, "cost_total": 9, "count": 9, "group_names": ["EdelNote"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにコスト9以上の『EdelNote』のメンバーがいる場合 (x1)

```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト4以下の『EdelNote』のメンバーカードを1枚、メンバーのいないエリアに登場させる。 (x1)

```json
{"action": "modify_required_hearts", "count": 1, "heart_colors": ["heart06"], "operation": "decrease", "self_target": true}
```

- このカードの必要ハートを{heart_06.png|heart06}減らす (x1)

```json
{"card_count": 1, "cards": ["LL-bp5-001-L | Live with a smile! (ab#0)"], "effect": {"action": "modify_score", "condition": {"aggregate": "total", "conditions": [{"card_type": "live_card", "count": 2, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}, {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}], "type": "or_condition"}, "heart_colors": ["heart01", "heart04", "heart05", "heart02", "heart03", "heart06"], "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"aggregate": "total", "conditions": [{"card_type": "live_card", "count": 2, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}, {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}], "type": "or_condition"}, "heart_colors": ["heart01", "heart04", "heart05", "heart02", "heart03", "heart06"], "operation": "add", "self_target": true, "value": 1}
```

- エールにより公開された自分のカードの中にライブカードが2枚以上あるか、自分のステージにいるメンバーが持つハートの中に{heart_01.png|heart01}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_06.png|heart06}のうち合計5種類以上あるか、このターンに自分のステージにいるメンバーがエリアを移動している場合、このカードのスコアを+1する (x1)

```json
{"aggregate": "total", "conditions": [{"card_type": "live_card", "count": 2, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}, {"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}, {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}], "type": "or_condition"}
```

- エールにより公開された自分のカードの中にライブカードが2枚以上あるか、自分のステージにいるメンバーが持つハートの中に{heart_01.png|heart01}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_06.png|heart06}のうち合計5種類以上あるか、このターンに自分のステージにいるメンバーがエリアを移動している場合 (x1)

```json
{"card_type": "live_card", "count": 2, "location": "revealed_cards", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- エールにより公開された自分のカードの中にライブカードが2枚以上ある (x1)

```json
{"aggregate": "total", "card_type": "member_card", "count": 5, "heart_colors": ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "types"}
```

- 自分のステージにいるメンバーが持つハートの中に{heart_01.png|heart01}、{heart_04.png|heart04}、{heart_05.png|heart05}、{heart_02.png|heart02}、{heart_03.png|heart03}、{heart_06.png|heart06}のうち合計5種類以上ある (x1)

```json
{"card_count": 1, "cards": ["LL-bp5-002-L | Bring the LOVE！ (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"count": 3, "distinct": "group_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 1, "duration": "live_end", "group_reference": "different_group_names", "position": "center", "resource": "heart", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"count": 3, "distinct": "group_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 1, "duration": "live_end", "group_reference": "different_group_names", "position": "center", "resource": "heart", "target": "self"}
```

- 自分のステージにグループ名がそれぞれ異なるメンバーが3人以上いる場合、ライブ終了時まで、自分のセンターエリアにいるメンバーは{icon_all.png|ハート}を得る (x1)

```json
{"count": 3, "distinct": "group_name", "location": "stage", "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分のステージにグループ名がそれぞれ異なるメンバーが3人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["LL-bp5-002-L | Bring the LOVE！ (ab#1)"], "effect": {"action": "move_cards", "all": true, "card_type": "member_card", "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "all": true, "card_type": "member_card", "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の控え室にある、自分のステージにいるすべてのメンバーと異なるグループ名を持つカード1枚を手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-bp5-024-L | Private Wars (ab#0)"], "effect": {"action": "choice", "condition": {"card_type": "member_card", "group_names": ["A-RISE"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["A-RISE"], "options": [{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target_count": 1}, {"action": "change_state", "card_type": "member_card", "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "choice", "condition": {"card_type": "member_card", "group_names": ["A-RISE"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "group_names": ["A-RISE"], "options": [{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target_count": 1}, {"action": "change_state", "card_type": "member_card", "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}], "original_value": true}
```

- 自分のステージに『A-RISE』のメンバーがいる場合、以下から1つを選ぶ。
・ウェイト状態のメンバー1人をアクティブにし、ライブ終了時まで、そのメンバーは{icon_blade.png|ブレード}を得る。
・相手のステージにいる元々持つ{icon_blade.png|ブレード}が3つ以下のメンバー1人をウェイトにする (x1)

```json
{"card_type": "member_card", "group_names": ["A-RISE"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージに『A-RISE』のメンバーがいる場合 (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "resource": "blade", "target_count": 1}
```

- ウェイト状態のメンバー1人をアクティブにし、ライブ終了時まで、そのメンバーは{icon_blade.png|ブレード}を得る。 (x1)

```json
{"action": "change_state", "card_type": "member_card", "count": 1, "original_value": true, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいる元々持つ{icon_blade.png|ブレード}が3つ以下のメンバー1人をウェイトにする (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-022-L | SELF CONTROL!! (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "count": 1, "duration": "live_end", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "position_change", "card_type": "member_card", "count": 1, "duration": "live_end", "target": "self"}
```

- 自分のステージにいる、このターン中にエリアを移動したメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-022-L | SELF CONTROL!! (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "live_card", "comparison_target": "opponent", "operator": ">", "target": "both", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"card_type": "live_card", "comparison_target": "opponent", "operator": ">", "target": "both", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- エールにより公開されている自分のライブカードの枚数が、エールにより公開されている相手のライブカードの枚数より多い場合、このカードのスコアを+1する (x1)

```json
{"card_type": "live_card", "comparison_target": "opponent", "operator": ">", "target": "both", "type": "comparison_condition"}
```

- エールにより公開されている自分のライブカードの枚数が、エールにより公開されている相手のライブカードの枚数より多い場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp5-023-L | Awaken the power (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"card_type": "member_card", "group_names": ["Aqours", "SaintSnow"], "location": "stage", "target": "self", "type": "group_condition"}, {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 4, "destination": "deck_top", "group_names": ["Aqours", "SaintSnow"], "max": true, "optional": true, "placement_order": "any_order", "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"aggregate": "total", "card_type": "member_card", "conditions": [{"card_type": "member_card", "group_names": ["Aqours", "SaintSnow"], "location": "stage", "target": "self", "type": "group_condition"}, {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}, "count": 4, "destination": "deck_top", "group_names": ["Aqours", "SaintSnow"], "max": true, "optional": true, "placement_order": "any_order", "source": "discard", "target": "self"}
```

- 自分のステージに『Aqours』のメンバーと『SaintSnow』のメンバーがいて、かつそれらのメンバーのコストが合計20以上の場合、自分の控え室にある『Aqours』と『SaintSnow』のライブカードを4枚まで好きな順番でデッキの上に置いてもよい (x1)

```json
{"aggregate": "total", "card_type": "member_card", "conditions": [{"card_type": "member_card", "group_names": ["Aqours", "SaintSnow"], "location": "stage", "target": "self", "type": "group_condition"}, {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}], "location": "stage", "operator": "and", "target": "self", "type": "compound"}
```

- 自分のステージに『Aqours』のメンバーと『SaintSnow』のメンバーがいて、かつそれらのメンバーのコストが合計20以上の場合 (x1)

```json
{"card_type": "member_card", "group_names": ["Aqours", "SaintSnow"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージに『Aqours』のメンバーと『SaintSnow』のメンバーがいて、 (x1)

```json
{"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "operator": ">=", "type": "comparison_condition"}
```

- それらのメンバーのコストが合計20以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-bp5-027-L | HOT PASSION!! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "optional": true, "source": "energy_deck", "state_change": "wait", "target": "self"}, {"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}}], "conditional": true}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "optional": true, "source": "energy_deck", "state_change": "wait", "target": "self"}, {"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}}], "conditional": true}
```

- 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置いてもよい。そうした場合、相手はカードを1枚引く (x1)

```json
{"action": "move_cards", "card_type": "energy_card", "count": 1, "destination": "energy_zone", "optional": true, "source": "energy_deck", "state_change": "wait", "target": "self"}
```

- 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置いてもよい。 (x1)

```json
{"action": "opponent_action", "action_by": "opponent", "opponent_action": {"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}}
```

- 相手はカードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-001-SD | 日野下花帆 (ab#0)"], "effect": {"action": "change_state", "card_type": "energy_card", "condition": {"baton_touch_trigger": true, "cost_limit": 10, "cost_limit_operator": ">=", "group_names": ["蓮ノ空"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 2, "state_change": "active"}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "change_state", "card_type": "energy_card", "condition": {"baton_touch_trigger": true, "cost_limit": 10, "cost_limit_operator": ">=", "group_names": ["蓮ノ空"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 2, "state_change": "active"}
```

- このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき、エネルギーを2枚アクティブにする (x1)

```json
{"baton_touch_trigger": true, "cost_limit": 10, "cost_limit_operator": ">=", "group_names": ["蓮ノ空"], "location": "discard", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-002-SD | 村野さやか (ab#0)"], "cost": {"count": 2, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "look_and_select", "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。これにより『蓮ノ空』のカードを手札に加えた場合、ライブ終了時まで、{heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "select_cards", "card_type": "member_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!HS-sd1-003-SD | 大沢瑠璃乃 (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01"], "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいるこのメンバー以外の『蓮ノ空』のメンバー1人は、{heart_01.png|heart01}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-004-SD | 百生吟子 (ab#0)"], "cost": {"count": 1, "destination": "discard", "group_names": ["蓮ノ空"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"count": 1, "destination": "discard", "group_names": ["蓮ノ空"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の『蓮ノ空』のカードを1枚控え室に置いてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-004-SD | 百生吟子 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"characters": ["日野下花帆", "徒町小鈴", "安養寺姫芽"], "heart_colors": ["heart04"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart04"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"characters": ["日野下花帆", "徒町小鈴", "安養寺姫芽"], "heart_colors": ["heart04"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart04"], "resource": "heart"}
```

- {heart_04.png|heart04}を得る (x1)

```json
{"characters": ["日野下花帆", "徒町小鈴", "安養寺姫芽"], "heart_colors": ["heart04"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「日野下花帆」か「徒町小鈴」か「安養寺姫芽」がいるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-005-SD | 徒町小鈴 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"baton_touch_trigger": true, "group_names": ["蓮ノ空"], "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "recently_moved", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"baton_touch_trigger": true, "group_names": ["蓮ノ空"], "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "recently_moved", "target": "self"}
```

- 「徒町小鈴」以外の『蓮ノ空』のメンバーからバトンタッチして登場した場合、自分の控え室からライブカードを1枚手札に加える (x1)

```json
{"baton_touch_trigger": true, "group_names": ["蓮ノ空"], "location": "stage", "movement": "baton_touch", "target": "self", "type": "movement_condition"}
```

- 「徒町小鈴」以外の『蓮ノ空』のメンバーからバトンタッチして登場した場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-005-SD | 徒町小鈴 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"characters": ["村野さやか", "百生吟子", "安養寺姫芽"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"characters": ["村野さやか", "百生吟子", "安養寺姫芽"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}を得る (x1)

```json
{"characters": ["村野さやか", "百生吟子", "安養寺姫芽"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「村野さやか」か「百生吟子」か「安養寺姫芽」がいるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-006-SD | 安養寺 姫芽 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"characters": ["大沢瑠璃乃", "百生吟子", "徒町小鈴"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"characters": ["大沢瑠璃乃", "百生吟子", "徒町小鈴"], "location": "stage", "target": "self", "type": "location_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分のステージに「大沢瑠璃乃」か「百生吟子」か「徒町小鈴」がいる場合、エネルギーを1枚アクティブにし、自分の控え室から『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"characters": ["大沢瑠璃乃", "百生吟子", "徒町小鈴"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「大沢瑠璃乃」か「百生吟子」か「徒町小鈴」がいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-008-SD | 桂城 泉 (ab#1)"], "cost": {"count": 2, "destination": "discard", "group_names": ["蓮ノ空"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "select", "count": 1, "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"]}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"], "resource": "heart", "target": "self", "target_count": 1}], "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"count": 2, "destination": "discard", "group_names": ["蓮ノ空"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の『蓮ノ空』のカードを2枚控え室に置いてもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "select", "count": 1, "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"]}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"], "resource": "heart", "target": "self", "target_count": 1}], "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"]}
```

- {heart_01.png|heart01}か{heart_04.png|heart04}か{heart_05.png|heart05}か{heart_06.png|heart06}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいるこのメンバー以外の『蓮ノ空』のメンバー1人は、選んだハートを2つ得る (x1)

```json
{"action": "select", "count": 1, "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"]}
```

- {heart_01.png|heart01}か{heart_04.png|heart04}か{heart_05.png|heart05}か{heart_06.png|heart06}のうち、1つを選ぶ (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "exclude_self": true, "group_names": ["蓮ノ空"], "heart_colors": ["heart01", "heart04", "heart05", "heart06"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいるこのメンバー以外の『蓮ノ空』のメンバー1人は、選んだハートを2つ得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-013-SD | 徒町小鈴 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart05"], "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart05"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart05"], "resource": "heart"}], "heart_colors": ["heart05"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart05"], "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart05"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart05"], "resource": "heart"}], "heart_colors": ["heart05"]}
```

- 自分のデッキの上からカードを3枚控え室に置く。それらがすべて{heart_05.png|heart05}を持つメンバーカードの場合、ライブ終了時まで、{heart_05.png|heart05}を得る (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "heart_colors": ["heart05"], "source": "deck_top", "target": "self"}
```

- 自分のデッキの上からカードを3枚控え室に置く (x1)

```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "count": 3, "heart_colors": ["heart05"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "filter_targets_by_heart_colors": true, "heart_colors": ["heart05"], "resource": "heart"}
```

- それらがすべて{heart_05.png|heart05}を持つメンバーカードの場合、ライブ終了時まで、{heart_05.png|heart05}を得る (x1)

```json
{"card_type": "member_card", "count": 3, "heart_colors": ["heart05"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}
```

- それらがすべて{heart_05.png|heart05}を持つメンバーカードの場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-014-SD | 安養寺 姫芽 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!HS-sd1-017-SD | 夏めきペイン (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["蓮ノ空"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "group_names": ["蓮ノ空"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["蓮ノ空"]}
```

- 自分のステージに『蓮ノ空』のメンバーがいる場合、カードを1枚引き、手札を1枚控え室に置く (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "deck"}
```

- カードを1枚引き (x1)

```json
{"card_count": 1, "cards": ["PL!HS-sd1-020-SD | Link to the FUTURE（104期Ver.） (ab#1)"], "cost": {"card_type": "member_card", "count": 3, "destination": "discard", "group_names": ["蓮ノ空"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard", "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_type": "member_card", "count": 3, "destination": "discard", "group_names": ["蓮ノ空"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の『蓮ノ空』のメンバーカードを3枚まで控え室に置いてもよい (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "per_unit": true, "per_unit_count": 1, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard", "resource": "blade", "target": "self"}
```

- 自分のステージのメンバー1人は、これにより控え室に置いたカード1枚につき、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-001-SD | 高海千歌 (ab#0)"], "effect": {"action": "gain_resource", "count": 3, "duration": "live_end", "heart_colors": ["heart02"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "count": 3, "duration": "live_end", "heart_colors": ["heart02"], "per_unit": true, "per_unit_count": 1, "per_unit_type": "枚", "resource": "heart", "target": "self"}
```

- 自分がエールしたとき、ライブ終了時まで、エールにより公開された自分のカードの中のライブカード1枚につき、{heart_02.png|heart02}を得る。この能力では{heart_02.png|heart02}は3つまでしか得られない (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-002-SD | 桜内梨子 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}
```

- 自分の控え室から『Aqours』のカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-003-SD | 松浦果南 (ab#0)"], "effect": {"action": "look_and_select", "group_names": ["Aqours"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["Aqours"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『Aqours』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "card_type": "live_card", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["Aqours"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!S-sd1-004-SD | 黒澤ダイヤ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "optional": true}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}], "conditional": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "optional": true}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}], "conditional": true}
```

- カードを1枚引いてもよい。そうした場合、手札2枚を好きな順番でデッキの上に置く (x1)

```json
{"action": "draw_card", "count": 1, "optional": true}
```

- カードを1枚引いてもよい。 (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 2, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}
```

- 手札2枚を好きな順番でデッキの上に置く (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-005-SD | 渡辺 曜 (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 1, "cards": ["PL!S-sd1-006-SD | 津島善子 (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["Aqours"], "parenthetical": ["この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["Aqours"], "parenthetical": ["この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト2以下の『Aqours』のメンバーカードを1枚、メンバーのいないエリアに登場させる (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-007-SD | 国木田花丸 (ab#0)"], "cost": {"count": 2, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 1, "cards": ["PL!S-sd1-009-SD | 黒澤ルビィ (ab#0)"], "cost": {"count": 1, "group_names": ["Aqours"], "optional": true, "source": "hand", "type": "reveal", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "destination": "deck_top_or_bottom", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"count": 1, "group_names": ["Aqours"], "optional": true, "source": "hand", "type": "reveal", "zone": "hand"}
```

- 手札の『Aqours』のカードを1枚公開してもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "destination": "deck_top_or_bottom", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}, {"action": "gain_resource", "count": 1, "duration": "live_end", "resource": "blade"}]}
```

- これにより公開したカードをデッキの一番上か一番下に置き、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "card_type": "card", "destination": "deck_top_or_bottom", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards"}
```

- これにより公開したカードをデッキの一番上か一番下に置き (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-019-SD | 未来の僕らは知ってるよ (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Aqours"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Aqours"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、『Aqours』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!S-sd1-020-SD | JIMO-AI Dash! (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "target": "self"}, {"action": "draw_card", "destination": "hand", "dynamic_count": {"reference": "previous_draw", "type": "drawn_cards"}, "group_names": ["Aqours"], "source": "deck"}], "group_names": ["Aqours"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "target": "self"}, {"action": "draw_card", "destination": "hand", "dynamic_count": {"reference": "previous_draw", "type": "drawn_cards"}, "group_names": ["Aqours"], "source": "deck"}], "group_names": ["Aqours"]}
```

- 自分のステージにいる『Aqours』のメンバー1人につき、カードを1枚引く。その後、これにより引いた枚数と同じ枚数を手札から控え室に置く (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "location": "stage", "per_unit": true, "per_unit_count": 1, "per_unit_type": "人", "source": "deck", "target": "self"}
```

- カードを1枚引く。 (x1)

```json
{"action": "draw_card", "destination": "hand", "dynamic_count": {"reference": "previous_draw", "type": "drawn_cards"}, "group_names": ["Aqours"], "source": "deck"}
```

- これにより引いた枚数と同じ枚数を手札から控え室に置く (x1)

```json
{"reference": "previous_draw", "type": "drawn_cards"}
```


```json
{"card_count": 1, "cards": ["PL!S-sd1-022-SD | Jump up HIGH!! (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Aqours"], "resource": "blade", "target": "self"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Aqours"], "resource": "blade", "target": "self"}
```

- 自分のステージにいる『Aqours』のメンバーは{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-018-N | 村野さやか (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "look_and_select", "group_names": ["DOLLCHESTRA"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["DOLLCHESTRA"], "optional": true, "reveal": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "group_names": ["DOLLCHESTRA"], "look_action": {"action": "look_at", "count": 5, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["DOLLCHESTRA"], "optional": true, "reveal": true}}
```

- 自分のデッキの上からカードを5枚見る。その中から『DOLLCHESTRA』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true, "group_names": ["DOLLCHESTRA"], "optional": true, "reveal": true}
```


```json
{"card_count": 1, "cards": ["PL!HS-pb1-020-N | 百生吟子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "optional": true, "source": "hand"}, {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "source": "discard", "target": "self"}], "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "group_names": ["スリーズブーケ", "蓮ノ空"]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "optional": true, "source": "hand"}, {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "source": "discard", "target": "self"}], "condition": {"card_type": "live_card", "count": 3, "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "conditional": true, "group_names": ["スリーズブーケ", "蓮ノ空"]}
```

- 自分の控え室にライブカードが3枚以上ある場合、手札を2枚控え室に置いてもよい。そうした場合、自分の控え室から『スリーズブーケ』のメンバーカード1枚と『蓮ノ空』のライブカード1枚を手札に加える (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "optional": true, "source": "hand"}
```

- 手札を2枚控え室に置いてもよい。 (x1)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室から『スリーズブーケ』のメンバーカード1枚と『蓮ノ空』のライブカード1枚を手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室から『スリーズブーケ』のメンバーカード1枚と『蓮ノ空』のライブカード1枚を手札に加える (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "group_names": ["スリーズブーケ", "蓮ノ空"], "max": true, "source": "discard", "target": "self"}
```

- 自分の控え室から『スリーズブーケ』のメンバーカード1枚と『蓮ノ空』のライブカード1枚を手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-021-N | 徒町小鈴 (ab#0)"], "effect": {"action": "draw_card", "condition": {"card_type": "live_card", "group_names": ["DOLLCHESTRA"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "deck"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "draw_card", "condition": {"card_type": "live_card", "group_names": ["DOLLCHESTRA"], "location": "live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "deck"}
```

- 自分のライブカード置き場に『DOLLCHESTRA』のカードがある場合、カードを1枚引く (x1)

```json
{"card_type": "live_card", "group_names": ["DOLLCHESTRA"], "location": "live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分のライブカード置き場に『DOLLCHESTRA』のカードがある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-022-N | 安養寺姫芽 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"characters": ["大沢瑠璃乃"], "heart_colors": ["heart01"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"characters": ["大沢瑠璃乃"], "heart_colors": ["heart01"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "heart_colors": ["heart01"], "resource": "heart"}
```

- {heart_01.png|heart01}{heart_01.png|heart01}を得る (x1)

```json
{"characters": ["大沢瑠璃乃"], "heart_colors": ["heart01"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「大沢瑠璃乃」がいるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-022-N | 安養寺姫芽 (ab#1)"], "effect": {"action": "gain_resource", "condition": {"characters": ["藤島慈"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"characters": ["藤島慈"], "location": "stage", "target": "self", "type": "location_condition"}, "conditional": true, "count": 2, "duration": "as_long_as", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"characters": ["藤島慈"], "location": "stage", "target": "self", "type": "location_condition"}
```

- 自分のステージに「藤島慈」がいるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-025-L | 抱きしめる花びら (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 10, "group_names": ["蓮ノ空"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart04"], "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "condition": {"card_type": "member_card", "count": 10, "group_names": ["蓮ノ空"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart04"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分の控え室に『蓮ノ空』のメンバーカードが10枚以上ある場合、ライブ終了時まで、自分のステージにいる『蓮ノ空』のメンバー1人は、{heart_04.png|heart04}を得る (x1)

```json
{"card_type": "member_card", "count": 10, "group_names": ["蓮ノ空"], "location": "discard", "operator": ">=", "target": "self", "type": "card_count_condition"}
```

- 自分の控え室に『蓮ノ空』のメンバーカードが10枚以上ある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-025-L | 抱きしめる花びら (ab#1)"], "effect": {"action": "move_cards", "card_type": "member_card", "condition": {"count": 6, "location": "hand", "operator": "<=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "condition": {"count": 6, "location": "hand", "operator": "<=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分の手札が6枚以下の場合、自分の控え室からメンバーカードを1枚手札に加える (x1)

```json
{"count": 6, "location": "hand", "operator": "<=", "target": "self", "type": "comparison_condition"}
```

- 自分の手札が6枚以下の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-026-L | 雪舞う空と二秒の永遠 (ab#0)"], "effect": {"action": "modify_required_hearts", "condition": {"count": 6, "distinct": "card_name", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "locations": ["discard", "stage"], "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 2, "distinct": "card_name", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "modify_required_hearts", "condition": {"count": 6, "distinct": "card_name", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "locations": ["discard", "stage"], "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}, "count": 2, "distinct": "card_name", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 自分の、ステージと控え室に名前の異なる『蓮ノ空』のメンバーが6人以上いる場合、このカードの必要ハートは{heart_00.png|heart0}{heart_00.png|heart0}減る (x1)

```json
{"count": 6, "distinct": "card_name", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "locations": ["discard", "stage"], "operator": ">=", "target": "self", "type": "location_condition", "unit": "人"}
```

- 自分の、ステージと控え室に名前の異なる『蓮ノ空』のメンバーが6人以上いる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-027-L | ユメワズライ (ab#0)"], "effect": {"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "group_names": ["スリーズブーケ"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 4, "destination": "discard", "group_names": ["スリーズブーケ"], "optional": true, "source": "deck_top", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "card", "condition": {"card_type": "member_card", "group_names": ["スリーズブーケ"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 4, "destination": "discard", "group_names": ["スリーズブーケ"], "optional": true, "source": "deck_top", "target": "self"}
```

- 自分のステージに『スリーズブーケ』のメンバーがいる場合、自分のデッキの上からカードを4枚控え室に置いてもよい (x1)

```json
{"card_type": "member_card", "group_names": ["スリーズブーケ"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージに『スリーズブーケ』のメンバーがいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-028-L | COMPASS (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "group_names": ["DOLLCHESTRA"], "target": "self"}, {"ability_text": "ライブ開始時_ability", "action": "activate_ability", "count": 1, "group_names": ["DOLLCHESTRA"], "target": "そのメンバーの{{live_start.png|ライブ開始時}}能力", "target_trigger": "ライブ開始時"}], "group_names": ["DOLLCHESTRA"], "parenthetical": ["{{live_start.png|ライブ開始時}}能力がコストを持つ場合、支払って発動させる。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "select", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "group_names": ["DOLLCHESTRA"], "target": "self"}, {"ability_text": "ライブ開始時_ability", "action": "activate_ability", "count": 1, "group_names": ["DOLLCHESTRA"], "target": "そのメンバーの{{live_start.png|ライブ開始時}}能力", "target_trigger": "ライブ開始時"}], "group_names": ["DOLLCHESTRA"], "parenthetical": ["{{live_start.png|ライブ開始時}}能力がコストを持つ場合、支払って発動させる。"]}
```

- 自分のステージにいるコスト10以上の『DOLLCHESTRA』のメンバー1人を選ぶ。そのメンバーの{live_start.png|ライブ開始時}能力1つを発動させてもよい (x1)

```json
{"action": "select", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": ">=", "count": 1, "group_names": ["DOLLCHESTRA"], "target": "self"}
```

- 自分のステージにいるコスト10以上の『DOLLCHESTRA』のメンバー1人を選ぶ (x1)

```json
{"ability_text": "ライブ開始時_ability", "action": "activate_ability", "count": 1, "group_names": ["DOLLCHESTRA"], "target": "そのメンバーの{{live_start.png|ライブ開始時}}能力", "target_trigger": "ライブ開始時"}
```

- そのメンバーの{live_start.png|ライブ開始時}能力1つを発動させてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-029-L | 全方位キュン♡ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 1, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "original_value": true, "source": "deck"}, {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}, "count": 2, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}], "group_names": ["みらくらぱーく！"], "heart_colors": ["heart00"], "original_value": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 1, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "original_value": true, "source": "deck"}, {"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}, "count": 2, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}], "group_names": ["みらくらぱーく！"], "heart_colors": ["heart00"], "original_value": true}
```

- 自分のステージに、元々持つハートの数より多い数のハートを持つ『みらくらぱーく！』のメンバーが1人以上いる場合、カードを1枚引く。2人以上いる場合、さらにこのカードの必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"action": "draw_card", "condition": {"card_type": "member_card", "comparison_target": "self", "count": 1, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "count": 1, "destination": "hand", "group_names": ["みらくらぱーく！"], "original_value": true, "source": "deck"}
```

- 自分のステージに、元々持つハートの数より多い数のハートを持つ『みらくらぱーく！』のメンバーが1人以上いる場合、カードを1枚引く (x1)

```json
{"card_type": "member_card", "comparison_target": "self", "count": 1, "group_names": ["みらくらぱーく！"], "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}
```

- 自分のステージに、元々持つハートの数より多い数のハートを持つ『みらくらぱーく！』のメンバーが1人以上いる場合 (x1)

```json
{"action": "modify_required_hearts", "condition": {"card_type": "member_card", "count": 2, "operator": ">=", "type": "card_count_condition", "unit": "人"}, "count": 2, "group_names": ["みらくらぱーく！"], "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 2人以上いる場合、このカードの必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"card_count": 1, "cards": ["PL!HS-pb1-030-L | Edelied (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "resource": "blade", "target": "self", "target_count": 1}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "distinct": "card_name", "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "resource": "heart", "target_count": 1}], "distinct": "card_name", "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "resource": "blade", "target": "self", "target_count": 1}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "distinct": "card_name", "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "resource": "heart", "target_count": 1}], "distinct": "card_name", "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"]}
```

- 自分のステージにいる『EdelNote』のメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得て、そのメンバーとは名前の異なる『EdelNote』のメンバー1人は、{heart_06.png|heart06}{heart_06.png|heart06}を得る (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 2, "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいる『EdelNote』のメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "distinct": "card_name", "duration": "live_end", "group_names": ["EdelNote"], "heart_colors": ["heart06"], "resource": "heart", "target_count": 1}
```

- そのメンバーとは名前の異なる『EdelNote』のメンバー1人は、{heart_06.png|heart06}{heart_06.png|heart06}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-010-N | 高坂穂乃果 (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "parenthetical": ["ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。"], "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 1, "cards": ["PL!-bp6-012-N | 南ことり (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "group_names": ["Printemps"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "group_names": ["Printemps"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}
```

- {heart_03.png|heart03}を得る (x1)

```json
{"card_type": "live_card", "group_names": ["Printemps"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分の成功ライブカード置き場に『Printemps』のカードがあるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-013-N | 園田海未 (ab#0)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"aggregate": "total", "card_type": "live_card", "comparison_type": "score", "count": 6, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}
```

- 自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合、自分の控え室から『μ's』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-014-N | 星空 凛 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "group_names": ["lilywhite"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart01"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "group_names": ["lilywhite"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart01"], "resource": "heart"}
```

- {heart_01.png|heart01}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-015-N | 西木野真姫 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "live_card", "group_names": ["BiBi"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "live_card", "group_names": ["BiBi"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart06"], "resource": "heart"}
```

- {heart_06.png|heart06}を得る (x1)

```json
{"card_type": "live_card", "group_names": ["BiBi"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}
```

- 自分の成功ライブカード置き場に『BiBi』のカードがあるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-016-N | 東條 希 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top", "placement_order": "any_order", "source": "hand"}]}
```

- 自分のデッキの上からカードを3枚見る。それらを好きな順番でデッキの上に置く (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-019-L | Music S.T.A.R.T!! (ab#0)"], "effect": {"action": "modify_cost", "card_type": "member_card", "condition": {"card_type": "live_card", "check_self": true, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "cost_limit": 17, "cost_limit_operator": ">=", "count": 17, "destination": "stage", "duration": "as_long_as", "group_names": ["μ's"], "location": "hand", "non_stackable": true, "operation": "subtract", "original_count": 17, "original_operator": ">=", "original_value": true, "source": "hand", "target": "self", "value": 2}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_cost", "card_type": "member_card", "condition": {"card_type": "live_card", "check_self": true, "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "cost_limit": 17, "cost_limit_operator": ">=", "count": 17, "destination": "stage", "duration": "as_long_as", "group_names": ["μ's"], "location": "hand", "non_stackable": true, "operation": "subtract", "original_count": 17, "original_operator": ">=", "original_value": true, "source": "hand", "target": "self", "value": 2}
```

- 元々のコストが17以上の『μ's』のメンバーカードを自分の手札から登場させるためのコストは2減る。この効果は重複しない (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-020-L | Dancing stars on me! (ab#0)"], "effect": {"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "position": "center", "target": "self", "type": "group_condition"}, "group_names": ["μ's"], "position": "center"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "position_change", "card_type": "member_card", "condition": {"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "position": "center", "target": "self", "type": "group_condition"}, "group_names": ["μ's"], "position": "center"}
```

- 自分のステージのセンターエリアにいる『μ's』のメンバーの{live_start.png|ライブ開始時}能力が解決したとき、そのメンバーをポジションチェンジする (x1)

```json
{"card_type": "member_card", "group_names": ["μ's"], "location": "stage", "position": "center", "target": "self", "type": "group_condition"}
```

- 自分のステージのセンターエリアにいる『μ's』のメンバーの{live_start.png|ライブ開始時}能力が解決したとき (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-020-L | Dancing stars on me! (ab#1)"], "effect": {"action": "modify_score", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "center", "temporal": "this_turn", "type": "temporal_condition"}, "group_names": ["μ's"], "operation": "add", "position": "center", "self_target": true, "value": 1}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "modify_score", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "position": "center", "temporal": "this_turn", "type": "temporal_condition"}, "group_names": ["μ's"], "operation": "add", "position": "center", "self_target": true, "value": 1}
```

- 自分のステージのセンターエリアにいる『μ's』のメンバーの{live_success.png|ライブ成功時}能力が解決したとき、そのメンバーがこのターン中に移動している場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-021-L | Wonderful Rush (ab#0)"], "cost": {"card_type": "member_card", "count": 1, "destination": "discard", "group_names": ["μ's"], "optional": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "sequential", "actions": [{"action": "modify_score", "group_names": ["μ's"], "operation": "add", "self_target": true, "value": 1}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}], "group_names": ["μ's"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"card_type": "member_card", "count": 1, "destination": "discard", "group_names": ["μ's"], "optional": true, "source": "stage", "type": "move_cards", "zone": "stage"}
```

- 『μ's』のメンバー1人をステージから控え室に置いてもよい (x1)

```json
{"action": "sequential", "actions": [{"action": "modify_score", "group_names": ["μ's"], "operation": "add", "self_target": true, "value": 1}, {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "discard", "target": "self"}], "group_names": ["μ's"]}
```

- このカードのスコアを+1し、自分の控え室から『μ's』のライブカード1枚を手札に加える (x1)

```json
{"action": "modify_score", "group_names": ["μ's"], "operation": "add", "self_target": true, "value": 1}
```

- このカードのスコアを+1し (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-022-L | Dreamin' Go! Go!! (ab#0)"], "effect": {"action": "modify_required_hearts", "card_type": "live_card", "condition": {"card_type": "live_card", "check_self": true, "heart_colors": ["heart00"], "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "count": 5, "duration": "as_long_as", "group_names": ["μ's"], "heart_colors": ["heart00"], "non_stackable": true, "operation": "decrease", "original_count": 5, "original_operator": ">=", "original_value": true, "target": "self"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "modify_required_hearts", "card_type": "live_card", "condition": {"card_type": "live_card", "check_self": true, "heart_colors": ["heart00"], "location": "success_live_card_zone", "target": "self", "type": "location_condition"}, "conditional": true, "count": 5, "duration": "as_long_as", "group_names": ["μ's"], "heart_colors": ["heart00"], "non_stackable": true, "operation": "decrease", "original_count": 5, "original_operator": ">=", "original_value": true, "target": "self"}
```

- 自分の元々のスコアが5以上の『μ's』のライブカードの必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}減らす。この効果は重複しない (x1)

```json
{"card_type": "live_card", "check_self": true, "heart_colors": ["heart00"], "location": "success_live_card_zone", "target": "self", "type": "location_condition"}
```

- このカードが自分の成功ライブカード置き場にあるかぎり (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-023-L | sweet&sweet holiday (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, {"action": "draw_card", "condition": {"card_type": "live_card", "group_names": ["μ's"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}], "group_names": ["μ's"]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}, {"action": "draw_card", "condition": {"card_type": "live_card", "group_names": ["μ's"], "location": "success_live_card_zone", "target": "self", "type": "group_condition"}, "count": 1, "destination": "hand", "group_names": ["μ's"], "source": "deck"}], "group_names": ["μ's"]}
```

- カードを1枚引く。自分の成功ライブカード置き場に『μ's』のカードがある場合、さらにカードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!-bp6-024-L | 錯覚CROSSROADS (ab#0)"], "effect": {"action": "conditional_alternative", "alternative_effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "group_names": ["μ's"], "optional": true, "source": "discard", "target": "self"}, "condition": {"card_type": "live_card", "location": "success_live_card_zone", "target_event": "placing_in_success_zone", "type": "location_condition"}, "group_names": ["μ's"]}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "conditional_alternative", "alternative_effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "group_names": ["μ's"], "optional": true, "source": "discard", "target": "self"}, "condition": {"card_type": "live_card", "location": "success_live_card_zone", "target_event": "placing_in_success_zone", "type": "location_condition"}, "group_names": ["μ's"]}
```

- このカードを成功ライブカード置き場に置く場合、代わりに自分の控え室にある『μ's』のライブカードを1枚置いてもよい (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "group_names": ["μ's"], "optional": true, "source": "discard", "target": "self"}
```

- 自分の控え室にある『μ's』のライブカードを1枚置いてもよい (x1)

```json
{"card_type": "live_card", "location": "success_live_card_zone", "target_event": "placing_in_success_zone", "type": "location_condition"}
```

- このカードを成功ライブカード置き場に置く場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-010-N | 高海千歌 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"aggregate": "total", "target": "self", "temporal": "during_live", "type": "temporal_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "condition": {"aggregate": "total", "target": "self", "temporal": "during_live", "type": "temporal_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "resource": "heart"}
```

- 自分のライブ中のライブカードの必要ハートに含まれる{heart_02.png|heart02}の合計が4以上の場合、ライブ終了時まで、{heart_02.png|heart02}を得る (x1)

```json
{"aggregate": "total", "target": "self", "temporal": "during_live", "type": "temporal_condition"}
```

- 自分のライブ中のライブカードの必要ハートに含まれる{heart_02.png|heart02}の合計が4以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-011-N | 桜内梨子 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}], "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}}
```

- 控え室から登場している場合、カードを2枚引き、手札を1枚控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-013-N | 黒澤ダイヤ (ab#0)"], "effect": {"action": "gain_resource", "count": 2, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!S-bp6-015-N | 津島善子 (ab#0)"], "effect": {"action": "change_state", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!S-bp6-016-N | 国木田花丸 (ab#0)"], "effect": {"action": "look_and_select", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}, "look_action": {"action": "look_at", "count": 3, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "discard_remaining": true}}
```

- 控え室から登場している場合、自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-019-L | Step! ZERO to ONE (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "modify_score", "group_names": ["Aqours"], "operation": "add", "self_target": true, "value": 1}, {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top_or_bottom", "source": "hand"}], "group_names": ["Aqours"]}], "condition": {"card_type": "member_card", "group_names": ["Aqours"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["Aqours"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "modify_score", "group_names": ["Aqours"], "operation": "add", "self_target": true, "value": 1}, {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top_or_bottom", "source": "hand"}], "group_names": ["Aqours"]}], "condition": {"card_type": "member_card", "group_names": ["Aqours"], "location": "stage", "target": "self", "type": "group_condition"}, "group_names": ["Aqours"]}
```

- 自分のステージにいるメンバーがすべて『Aqours』の場合、このカードのスコアを+1し、カードを1枚引き、手札からカードを1枚デッキの一番上か一番下に置く (x1)

```json
{"card_type": "member_card", "group_names": ["Aqours"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにいるメンバーがすべて『Aqours』の場合 (x1)

```json
{"action": "modify_score", "group_names": ["Aqours"], "operation": "add", "self_target": true, "value": 1}
```

- このカードのスコアを+1し (x1)

```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top_or_bottom", "source": "hand"}], "group_names": ["Aqours"]}
```

- カードを1枚引き、手札からカードを1枚デッキの一番上か一番下に置く (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["Aqours"], "source": "deck"}
```

- カードを1枚引き (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "deck_top_or_bottom", "source": "hand"}
```

- 手札からカードを1枚デッキの一番上か一番下に置く (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-020-L | 冒険Type A, B, C!! (ab#0)"], "effect": {"action": "choice", "count": 1, "group_names": ["Aqours"], "heart_colors": ["heart02"], "options": [{"ability_gain": "カードを1枚引く。", "action": "gain_ability", "count": 1, "self_target": true}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Aqours"], "heart_colors": ["heart02"], "resource": "heart", "target_count": 1}, {"action": "modify_score", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "self_target": true, "value": 1}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "choice", "count": 1, "group_names": ["Aqours"], "heart_colors": ["heart02"], "options": [{"ability_gain": "カードを1枚引く。", "action": "gain_ability", "count": 1, "self_target": true}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Aqours"], "heart_colors": ["heart02"], "resource": "heart", "target_count": 1}, {"action": "modify_score", "condition": {"count": 2, "location": "success_live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "operation": "add", "self_target": true, "value": 1}]}
```

- 以下から1つを選ぶ。
・このカードは「{live_success.png|ライブ成功時}カードを1枚引く。」を得る。
・ライブ終了時まで、このターンにバトンタッチして登場した『Aqours』のメンバー1人は{heart_02.png|heart02}を得る。
・自分の成功ライブカード置き場にカードが2枚以上ある場合、このカードのスコアを+1する (x1)

```json
{"ability_gain": "カードを1枚引く。", "action": "gain_ability", "count": 1, "self_target": true}
```

- このカードは「{live_success.png|ライブ成功時}カードを1枚引く。」を得る。 (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["Aqours"], "heart_colors": ["heart02"], "resource": "heart", "target_count": 1}
```

- ライブ終了時まで、このターンにバトンタッチして登場した『Aqours』のメンバー1人は{heart_02.png|heart02}を得る。 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-021-L | MIRAI TICKET (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Aqours"], "max": true, "optional": true, "source": "revealed_cards", "target": "self"}, {"action": "modify_score", "count": 4, "group_names": ["Aqours"], "max": true, "max_repeats": 4, "operation": "add", "per_unit": true, "per_unit_count": 5, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard"}], "condition": {"type": "custom"}, "conditional": true, "group_names": ["Aqours"]}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Aqours"], "max": true, "optional": true, "source": "revealed_cards", "target": "self"}, {"action": "modify_score", "count": 4, "group_names": ["Aqours"], "max": true, "max_repeats": 4, "operation": "add", "per_unit": true, "per_unit_count": 5, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard"}], "condition": {"type": "custom"}, "conditional": true, "group_names": ["Aqours"]}
```

- 自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いたカードのコスト5につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["Aqours"], "max": true, "optional": true, "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。 (x1)

```json
{"action": "modify_score", "count": 4, "group_names": ["Aqours"], "max": true, "max_repeats": 4, "operation": "add", "per_unit": true, "per_unit_count": 5, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard"}
```

- これにより控え室に置いたカードのコスト5につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-022-L | 近未来ハッピーエンド (ab#0)"], "effect": {"action": "modify_score", "condition": {"comparison_target": "self", "operator": ">", "resource_type": "energy", "target": "opponent", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"comparison_target": "self", "operator": ">", "resource_type": "energy", "target": "opponent", "type": "comparison_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- 相手のエネルギーが自分より多い場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-023-L | GALAXY HidE and SeeK (ab#0)"], "effect": {"action": "modify_score", "condition": {"location": "revealed_cards", "target": "self", "type": "location_condition"}, "operation": "add", "self_target": true, "value": 1}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "modify_score", "condition": {"location": "revealed_cards", "target": "self", "type": "location_condition"}, "operation": "add", "self_target": true, "value": 1}
```

- エールにより公開された自分のカードの中にライブカードがある場合、このカードのスコアを+1する (x1)

```json
{"location": "revealed_cards", "target": "self", "type": "location_condition"}
```

- エールにより公開された自分のカードの中にライブカードがある場合 (x1)

```json
{"card_count": 1, "cards": ["PL!S-bp6-024-L | コワレヤスキ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "duration": "live_end", "opponent_action": {"action": "gain_resource", "all": true, "resource": "surplus_heart", "sign": "negative"}}, {"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "duration": "live_end", "operation": "add", "self_target": true, "value": 1}], "duration": "live_end"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "opponent_action", "action_by": "opponent", "duration": "live_end", "opponent_action": {"action": "gain_resource", "all": true, "resource": "surplus_heart", "sign": "negative"}}, {"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "duration": "live_end", "operation": "add", "self_target": true, "value": 1}], "duration": "live_end"}
```

- 相手は余剰ハートをすべて失う。これにより相手が余剰ハートを2つ以上失っている場合、このカードのスコアを+1する (x1)

```json
{"action": "opponent_action", "action_by": "opponent", "duration": "live_end", "opponent_action": {"action": "gain_resource", "all": true, "resource": "surplus_heart", "sign": "negative"}}
```

- 相手は余剰ハートをすべて失う。 (x1)

```json
{"action": "gain_resource", "all": true, "resource": "surplus_heart", "sign": "negative"}
```

- 余剰ハートをすべて失う (x1)

```json
{"action": "modify_score", "condition": {"count": 2, "operator": ">=", "resource_type": "surplus_heart", "type": "comparison_condition"}, "duration": "live_end", "operation": "add", "self_target": true, "value": 1}
```

- これにより相手が余剰ハートを2つ以上失っている場合、このカードのスコアを+1する (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-009-R | 日野下花帆 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 4, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"count": 4, "group_names": ["蓮ノ空"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}], "group_names": ["蓮ノ空"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 4, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "gain_resource", "condition": {"count": 4, "group_names": ["蓮ノ空"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}], "group_names": ["蓮ノ空"]}
```

- 自分のデッキの上からカードを4枚控え室に置く。それらがすべて『蓮ノ空』のカードの場合、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "condition": {"count": 4, "group_names": ["蓮ノ空"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}, "count": 1, "duration": "live_end", "resource": "blade"}
```

- それらがすべて『蓮ノ空』のカードの場合、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"count": 4, "group_names": ["蓮ノ空"], "operator": "=", "source": "preceding_moved", "type": "card_count_condition"}
```

- それらがすべて『蓮ノ空』のカードの場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-010-R | 村野さやか (ab#0)"], "cost": {"count": 1, "destination": "discard", "group_names": ["DOLLCHESTRA"], "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "deck"}, {"action": "modify_cost", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["DOLLCHESTRA"], "operation": "add", "target": "self", "value": 5}], "group_names": ["DOLLCHESTRA"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "deck"}, {"action": "modify_cost", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["DOLLCHESTRA"], "operation": "add", "target": "self", "value": 5}], "group_names": ["DOLLCHESTRA"]}
```

- カードを1枚引き、ライブ終了時まで、自分のステージにいる『DOLLCHESTRA』のメンバー1人のコストを+5する (x1)

```json
{"action": "draw_card", "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "deck"}
```

- カードを1枚引き (x1)

```json
{"action": "modify_cost", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["DOLLCHESTRA"], "operation": "add", "target": "self", "value": 5}
```

- 自分のステージにいる『DOLLCHESTRA』のメンバー1人のコストを+5する (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-011-R | 大沢瑠璃乃 (ab#0)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"card_count": 1, "cards": ["PL!HS-bp6-012-R | 百生 吟子 (ab#0)"], "effect": {"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["スリーズブーケ"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "exclude_self": true, "state_change": "active"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "change_state", "card_type": "energy_card", "condition": {"card_type": "member_card", "exclude_self": true, "group_names": ["スリーズブーケ"], "location": "stage", "target": "self", "type": "group_condition"}, "count": 1, "exclude_self": true, "state_change": "active"}
```

- 自分のステージにほかの『スリーズブーケ』のメンバーがいる場合、エネルギーを1枚アクティブにする (x1)

```json
{"card_type": "member_card", "exclude_self": true, "group_names": ["スリーズブーケ"], "location": "stage", "target": "self", "type": "group_condition"}
```

- 自分のステージにほかの『スリーズブーケ』のメンバーがいる場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-013-R | 徒町 小鈴 (ab#0)"], "effect": {"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "count": 1, "exclude_group_names": ["DOLLCHESTRA"], "group_names": null, "original_value": true, "state_change": "wait", "target": "opponent"}, "is_null": false, "triggers": "ライブ開始時, 登場"}
```


```json
{"action": "change_state", "blade_limit": 3, "blade_limit_operator": "<=", "card_type": "member_card", "count": 1, "exclude_group_names": ["DOLLCHESTRA"], "group_names": null, "original_value": true, "state_change": "wait", "target": "opponent"}
```

- 相手のステージにいる元々持つ{icon_blade.png|ブレード}の数が3つ以下の『DOLLCHESTRA』以外のメンバー1人をウェイトにする (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-014-R | 安養寺 姫芽 (ab#0)"], "cost": {"destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "draw_card", "activation_condition_parsed": {"check_self": true, "count": 1, "location": "hand", "operator": ">=", "target": "self", "type": "comparison_condition"}, "characters": ["藤島慈", "大沢瑠璃乃"], "count": 1, "destination": "hand", "duration": "live_end", "source": "deck", "target": "self", "target_count": 1}, "is_null": false, "triggers": "起動"}
```


```json
{"action": "draw_card", "activation_condition_parsed": {"check_self": true, "count": 1, "location": "hand", "operator": ">=", "target": "self", "type": "comparison_condition"}, "characters": ["藤島慈", "大沢瑠璃乃"], "count": 1, "destination": "hand", "duration": "live_end", "source": "deck", "target": "self", "target_count": 1}
```

- カードを1枚引き、ライブ終了時まで、自分のステージにいる「藤島慈」か「大沢瑠璃乃」のうち1人は{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-015-R | セラス 柳田 リリエンフェルト (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"appearance": true, "location": "stage", "type": "appearance_condition"}}
```

- このメンバーが手札以外からステージに登場している場合、カードを2枚引き、手札を2枚控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-016-R | 桂城 泉 (ab#0)"], "cost": {"count": 4, "energy": 4, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "empty_area", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分の控え室からコスト4以下の『蓮ノ空』のメンバーカードを1枚、メンバーのいないエリアに登場させる (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-017-N | 日野下花帆 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}, {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "conditional": true}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}, {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "conditional": true}
```

- このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える (x1)

```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}], "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える (x1)

```json
{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "max": true, "multiple_targets": true, "source": "discard", "target": "self"}
```

- 自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-018-N | 村野さやか (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "heart_colors": ["heart05"], "optional": true, "source": "hand"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade", "target": "self", "target_count": 1}], "condition": {"card_type": "member_card", "heart_colors": ["heart05"], "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "conditional": true, "heart_colors": ["heart05"]}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "heart_colors": ["heart05"], "optional": true, "source": "hand"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade", "target": "self", "target_count": 1}], "condition": {"card_type": "member_card", "heart_colors": ["heart05"], "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}, "conditional": true, "heart_colors": ["heart05"]}
```

- このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "heart_colors": ["heart05"], "optional": true, "source": "hand"}
```

- 手札を1枚控え室に置いてもよい。 (x1)

```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "heart_colors": ["heart05"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいるメンバー1人は、{heart_05.png|heart05}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_type": "member_card", "heart_colors": ["heart05"], "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}
```

- このメンバーがステージから控え室に置かれたとき (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-019-N | 大沢瑠璃乃 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}}, "is_null": false, "triggers": "自動"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 2, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 2, "destination": "discard", "source": "hand"}], "condition": {"card_type": "member_card", "location": "discard", "locations": ["discard", "stage"], "type": "location_condition"}}
```

- このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を2枚控え室に置く (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-025-L | ツバサ・ラ・リベルテ (ab#0)"], "cost": {"count": 1, "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "resource": "heart", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["蓮ノ空"], "heart_colors": ["heart05"], "resource": "heart", "target": "self", "target_count": 1}
```

- 自分のステージにいる『蓮ノ空』のメンバー1人は、{heart_05.png|heart05}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-025-L | ツバサ・ラ・リベルテ (ab#1)"], "effect": {"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "cost_limit": 3, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "live_card", "condition": {"card_type": "member_card", "count": 2, "location": "stage", "operator": ">=", "target": "self", "type": "card_count_condition", "unit": "人"}, "cost_limit": 3, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "source": "discard", "target": "self"}
```

- 自分のステージにメンバーが2人以上いる場合、自分の控え室からスコア3以下のライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-027-L | 月夜見海月 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "max": true, "optional": true, "source": "revealed_cards", "target": "self"}, {"action": "modify_score", "dynamic_count": {"mode": "equals", "reference": "これにより控え室に置いた数", "type": "dynamic_count"}, "group_names": ["蓮ノ空"], "operation": "add"}], "condition": {"type": "custom"}, "conditional": true, "group_names": ["蓮ノ空"]}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "sequential", "actions": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "max": true, "optional": true, "source": "revealed_cards", "target": "self"}, {"action": "modify_score", "dynamic_count": {"mode": "equals", "reference": "これにより控え室に置いた数", "type": "dynamic_count"}, "group_names": ["蓮ノ空"], "operation": "add"}], "condition": {"type": "custom"}, "conditional": true, "group_names": ["蓮ノ空"]}
```

- 自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う (x1)

```json
{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "max": true, "optional": true, "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。 (x1)

```json
{"action": "modify_score", "dynamic_count": {"mode": "equals", "reference": "これにより控え室に置いた数", "type": "dynamic_count"}, "group_names": ["蓮ノ空"], "operation": "add"}
```

- これにより控え室に置いた数に等しい枚数のエールを追加で行う (x1)

```json
{"mode": "equals", "reference": "これにより控え室に置いた数", "type": "dynamic_count"}
```


```json
{"card_count": 1, "cards": ["PL!HS-bp6-028-L | ブルウモーメント (ab#0)"], "effect": {"action": "look_and_select", "condition": {"count": 1, "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "look_and_select", "condition": {"count": 1, "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}, "look_action": {"action": "look_at", "count": 2, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "any_number": true, "destination": "deck_top", "discard_remaining": true, "placement_order": "any_order", "reveal": false}}
```

- このターン、自分が余剰ハートを1つ以上持っている場合、自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く (x1)

```json
{"count": 1, "operator": ">=", "resource_type": "surplus_heart", "temporal": "this_turn", "temporal_scope": "this_turn", "type": "comparison_condition"}
```

- このターン、自分が余剰ハートを1つ以上持っている場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-029-L | Proof (ab#0)"], "effect": {"action": "look_and_select", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "followup_action": {"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 30, "count": 30, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "look_action": {"action": "look_at", "count": 2, "source": "deck_top"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "remainder_destination": "deck_top", "reveal": false}}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "look_and_select", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "followup_action": {"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 30, "count": 30, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}, "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "look_action": {"action": "look_at", "count": 2, "source": "deck_top"}, "select_action": {"action": "select_cards", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "remainder_destination": "deck_top", "reveal": false}}
```

- 自分のステージにいる『蓮ノ空』のメンバーのコストが合計20以上の場合、デッキの上のカードを2枚見る。その中から1枚を手札に加え、残りをデッキの上に戻す。30以上の場合、さらにこのカードの必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 20, "count": 20, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 自分のステージにいる『蓮ノ空』のメンバーのコストが合計20以上の場合 (x1)

```json
{"action": "select_cards", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "heart_colors": ["heart00"], "remainder_destination": "deck_top", "reveal": false}
```


```json
{"action": "modify_required_hearts", "condition": {"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 30, "count": 30, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "count": 2, "heart_colors": ["heart00"], "operation": "decrease", "self_target": true}
```

- 30以上の場合、さらにこのカードの必要ハートを{heart_00.png|heart0}{heart_00.png|heart0}減らす (x1)

```json
{"aggregate": "total", "card_type": "member_card", "comparison_type": "cost", "cost_total": 30, "count": 30, "group_names": ["蓮ノ空"], "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}
```

- 30以上の場合 (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-030-L | Very! Very! COCO夏っ (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "source": "hand"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"card_count": 1, "cards": ["PL!HS-bp6-031-L | ファンファーレ！！！ (ab#0)"], "effect": {"action": "conditional_on_result", "all": true, "followup_action": {"action": "gain_resource", "characters": ["安養寺姫芽"], "count": 3, "duration": "live_end", "quoted_text": {"quoted_type": "character"}, "resource": "blade", "target": "self", "target_count": 1}, "group_names": ["みらくらぱーく！"], "primary_effect": {"action": "move_cards", "all": true, "card_type": "member_card", "destination": "deck_bottom", "optional": true, "shuffle": true, "source": "discard", "target": "deck"}, "result_condition": {"count": 15, "group_names": ["みらくらぱーく！"], "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "shuffle": true}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "conditional_on_result", "all": true, "followup_action": {"action": "gain_resource", "characters": ["安養寺姫芽"], "count": 3, "duration": "live_end", "quoted_text": {"quoted_type": "character"}, "resource": "blade", "target": "self", "target_count": 1}, "group_names": ["みらくらぱーく！"], "primary_effect": {"action": "move_cards", "all": true, "card_type": "member_card", "destination": "deck_bottom", "optional": true, "shuffle": true, "source": "discard", "target": "deck"}, "result_condition": {"count": 15, "group_names": ["みらくらぱーく！"], "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}, "shuffle": true}
```

- 自分の控え室にあるすべてのメンバーカードをシャッフルし、デッキの下に置いてもよい。これにより『みらくらぱーく！』のカードを15枚以上デッキの下に置いた場合、ライブ終了時まで、自分のステージにいる「安養寺姫芽」1人は{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "move_cards", "all": true, "card_type": "member_card", "destination": "deck_bottom", "optional": true, "shuffle": true, "source": "discard", "target": "deck"}
```

- 自分の控え室にあるすべてのメンバーカードをシャッフルし、デッキの下に置いてもよい。 (x1)

```json
{"count": 15, "group_names": ["みらくらぱーく！"], "operator": ">=", "source": "preceding_moved", "type": "card_count_condition"}
```

- これにより『みらくらぱーく！』のカードを15枚以上デッキの下に置いた場合 (x1)

```json
{"action": "gain_resource", "characters": ["安養寺姫芽"], "count": 3, "duration": "live_end", "quoted_text": {"quoted_type": "character"}, "resource": "blade", "target": "self", "target_count": 1}
```

- ライブ終了時まで、自分のステージにいる「安養寺姫芽」1人は{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-bp6-032-L | フュージョンクラスト (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit": 4, "cost_limit_operator": "<=", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、コスト4以下のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["LL-bp6-001-R＋ | 南 ことり&黒澤ダイヤ&徒町小鈴 (ab#0)"], "effect": {"action": "look_and_select", "look_action": {"action": "look_at", "count": 6, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 2, "destination": "hand", "discard_remaining": true}}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "look_and_select", "look_action": {"action": "look_at", "count": 6, "source": "deck_top", "target": "self"}, "select_action": {"action": "select_cards", "count": 2, "destination": "hand", "discard_remaining": true}}
```

- 自分のデッキの上からカードを6枚見る。その中からカードを2枚手札に加え、残りを控え室に置く (x1)

```json
{"card_count": 1, "cards": ["LL-bp6-001-R＋ | 南 ことり&黒澤ダイヤ&徒町小鈴 (ab#1)"], "cost": {"any_number": true, "characters": ["南ことり", "黒澤ダイヤ", "徒町小鈴"], "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}, "effect": {"action": "gain_resource", "count": 1, "duration": "live_end", "multiple_targets": true, "per_unit": true, "per_unit_count": 1, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard", "resource": "heart"}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"any_number": true, "characters": ["南ことり", "黒澤ダイヤ", "徒町小鈴"], "destination": "discard", "optional": true, "source": "hand", "type": "move_cards", "zone": "hand"}
```

- 手札の「南ことり」と「黒澤ダイヤ」と「徒町小鈴」を、好きな枚数控え室に置いてもよい (x1)

```json
{"action": "gain_resource", "count": 1, "duration": "live_end", "multiple_targets": true, "per_unit": true, "per_unit_count": 1, "per_unit_source": "previous_moved_cards", "per_unit_type": "discard", "resource": "heart"}
```

- これにより控え室に置いたそれらのカードが持つハートの色1つにつき、その色のハートを1つずつ得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-001-CL | 日野下花帆 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "look_at", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "look_at", "count": 1, "source": "deck_top", "target": "self"}, {"action": "move_cards", "card_type": "card", "count": 1, "destination": "discard", "optional": true, "source": "hand"}]}
```

- 自分のデッキの上からカードを1枚見る。そのカードを控え室に置いてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-002-CL | 村野さやか (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["DOLLCHESTRA"], "source": "discard", "target": "self"}
```

- 自分の控え室から『DOLLCHESTRA』のカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-003-CL | 大沢瑠璃乃 (ab#0)"], "cost": {"card_type": "member_card", "self_cost": true, "state_change": "wait", "type": "change_state"}, "effect": {"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["みらくらぱーく！"], "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "duration": "live_end", "group_names": ["みらくらぱーく！"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいる『みらくらぱーく！』のメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-004-CL | 百生 吟子 (ab#0)"], "effect": {"action": "choice", "count": 1, "options": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "change_state", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}]}, "is_null": false, "triggers": "登場"}
```


```json
{"action": "choice", "count": 1, "options": [{"action": "move_cards", "card_type": "card", "count": 3, "destination": "discard", "source": "deck_top", "target": "self"}, {"action": "change_state", "card_type": "member_card", "cost_limit": 2, "cost_limit_operator": "<=", "count": 1, "state_change": "wait", "target": "opponent"}]}
```

- 以下から1つを選ぶ。
・自分のデッキの上からカードを3枚控え室に置く。
・相手のステージにいるコスト2以下のメンバー1人をウェイトにする (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-006-CL | 安養寺 姫芽 (ab#0)"], "effect": {"action": "gain_resource", "count": 3, "duration": "live_end", "resource": "blade"}, "is_null": false, "triggers": "登場"}
```


```json
{"card_count": 1, "cards": ["PL!HS-cl1-008-CL | 桂城 泉 (ab#0)"], "cost": {"card_type": "member_card", "destination": "discard", "self_cost": true, "source": "stage", "type": "move_cards", "zone": "stage"}, "effect": {"action": "move_cards", "card_type": "card", "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動"}
```


```json
{"card_count": 1, "cards": ["PL!HS-cl1-009-CL | 水彩世界 (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "cost_limit_max": 9, "cost_limit_min": 4, "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "cost_limit_max": 9, "cost_limit_min": 4, "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "group_names": ["蓮ノ空"], "source": "revealed_cards", "target": "self"}
```

- エールにより公開された自分のカードの中から、コスト4以上9以下の『蓮ノ空』のメンバーカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-010-CL | AWOKE (ab#0)"], "effect": {"action": "gain_resource", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": ">=", "count": 2, "duration": "live_end", "group_names": ["蓮ノ空"], "resource": "blade", "target": "self", "target_count": 1}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "cost_limit": 10, "cost_limit_operator": ">=", "count": 2, "duration": "live_end", "group_names": ["蓮ノ空"], "resource": "blade", "target": "self", "target_count": 1}
```

- 自分のステージにいるコスト10以上の『蓮ノ空』のメンバー1人は、{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-011-CL | ド！ド！ド！ (ab#0)"], "cost": {"count": 1, "energy": 1, "optional": true, "type": "pay_energy", "zone": "energy_zone"}, "effect": {"action": "choice", "count": 1, "group_names": ["蓮ノ空"], "options": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "choice", "count": 1, "group_names": ["蓮ノ空"], "options": [{"action": "move_cards", "card_type": "member_card", "count": 1, "destination": "hand", "source": "discard", "target": "self"}, {"action": "move_cards", "card_type": "live_card", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}]}
```

- 以下から1つを選ぶ。
・自分の控え室からメンバーカードを1枚手札に加える。
・自分のライブカード置き場にカードが2枚以上ある場合、自分の控え室から『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"action": "move_cards", "card_type": "live_card", "condition": {"count": 2, "location": "live_card_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "count": 1, "destination": "hand", "group_names": ["蓮ノ空"], "source": "discard", "target": "self"}
```

- 自分のライブカード置き場にカードが2枚以上ある場合、自分の控え室から『蓮ノ空』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!HS-cl1-012-CL | Edelied (ab#0)"], "effect": {"action": "move_cards", "card_type": "member_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "location": "live_card_zone", "operator": "=", "resource_type": "score", "scope": "both", "target": "both", "type": "comparison_condition"}, "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "move_cards", "card_type": "member_card", "condition": {"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "location": "live_card_zone", "operator": "=", "resource_type": "score", "scope": "both", "target": "both", "type": "comparison_condition"}, "cost_limit": 9, "cost_limit_operator": ">=", "count": 1, "destination": "hand", "dynamic_count": {"reference": "previous_reveal", "type": "revealed_cards"}, "source": "revealed_cards", "target": "self"}
```

- 自分と相手のライブの合計スコアが同じ場合、エールにより公開された自分のカードの中から、コスト9以上のメンバーカードを1枚手札に加える (x1)

```json
{"aggregate": "total", "comparison_target": "opponent", "comparison_type": "score", "location": "live_card_zone", "operator": "=", "resource_type": "score", "scope": "both", "target": "both", "type": "comparison_condition"}
```

- 自分と相手のライブの合計スコアが同じ場合 (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-001-SD2 | 澁谷かのん (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "position_change", "card_type": "member_card", "multiple_targets": true, "optional": true, "target": "self"}], "parenthetical": ["メンバーをそれぞれ好きなエリアに移動させる。この効果で1つのエリアに2人以上のメンバーを移動させることはできない。"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "position_change", "card_type": "member_card", "multiple_targets": true, "optional": true, "target": "self"}], "parenthetical": ["メンバーをそれぞれ好きなエリアに移動させる。この効果で1つのエリアに2人以上のメンバーを移動させることはできない。"]}
```

- カードを1枚引く。その後、自分のステージにいるメンバーをフォーメーションチェンジしてもよい (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-003-SD2 | 嵐 千砂都 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "draw_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}, "is_null": false, "triggers": "ライブ成功時"}
```


```json
{"action": "sequential", "actions": [{"action": "draw_card", "count": 1, "destination": "hand", "source": "deck"}, {"action": "draw_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "count": 1, "destination": "hand", "source": "deck"}]}
```

- カードを1枚引く。このターン、このメンバーがエリアを移動している場合、さらにカードを1枚引く (x1)

```json
{"action": "draw_card", "condition": {"card_type": "member_card", "condition": {"type": "has_moved"}, "temporal": "this_turn", "type": "temporal_condition"}, "count": 1, "destination": "hand", "source": "deck"}
```

- このターン、このメンバーがエリアを移動している場合、カードを1枚引く (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-004-SD2 | 平安名すみれ (ab#0)"], "effect": {"action": "gain_resource", "activation_position": "center", "count": 4, "position": "center", "resource": "blade"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "activation_position": "center", "count": 4, "position": "center", "resource": "blade"}
```

- {icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-006-SD2 | 桜小路きな子 (ab#0)"], "cost": {"costs": [{"count": 2, "energy": 2, "type": "pay_energy", "zone": "energy_zone"}, {"count": 1, "destination": "discard", "source": "hand", "type": "move_cards", "zone": "hand"}], "type": "sequential_cost"}, "effect": {"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}, "is_null": false, "triggers": "起動", "use_limit": 1}
```


```json
{"action": "move_cards", "card_type": "live_card", "count": 1, "destination": "hand", "group_names": ["Liella!"], "source": "discard", "target": "self"}
```

- 自分の控え室から『Liella!』のライブカードを1枚手札に加える (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-008-SD2 | 若菜四季 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 13, "cost_total": 13, "count": 13, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}, "is_null": false, "triggers": "常時"}
```


```json
{"action": "gain_resource", "condition": {"card_type": "member_card", "comparison_type": "cost", "cost_limit": 13, "cost_total": 13, "count": 13, "location": "stage", "operator": ">=", "target": "self", "type": "comparison_condition"}, "conditional": true, "count": 1, "duration": "as_long_as", "heart_colors": ["heart03"], "resource": "heart"}
```

- {heart_03.png|heart03}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-011-SD2 | 鬼塚冬毬 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "blade"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "blade"}
```

- このメンバーがエリアを移動したとき、ライブ終了時まで、{icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-012-SD2 | 澁谷かのん (ab#0)"], "effect": {"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart02"], "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "heart"}
```

- このメンバーがエリアを移動したとき、ライブ終了時まで、{heart_02.png|heart02}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-020-SD2 | 鬼塚夏美 (ab#0)"], "effect": {"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "resource": "blade", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "exclude_self": true, "group_names": ["Liella!"], "resource": "blade", "target_count": 1}], "condition": {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "exclude_self": true, "group_names": ["Liella!"]}, "is_null": false, "triggers": "ライブ開始時"}
```


```json
{"action": "sequential", "actions": [{"action": "gain_resource", "count": 1, "resource": "blade", "target": "self"}, {"action": "gain_resource", "card_type": "member_card", "count": 1, "exclude_self": true, "group_names": ["Liella!"], "resource": "blade", "target_count": 1}], "condition": {"count": 7, "location": "energy_zone", "operator": ">=", "target": "self", "type": "card_count_condition"}, "exclude_self": true, "group_names": ["Liella!"]}
```

- 自分のエネルギーが7枚以上ある場合、ライブ終了時まで、このメンバーと自分のステージにいるほかの『Liella!』のメンバー1人は、{icon_blade.png|ブレード}を得る (x1)

```json
{"action": "gain_resource", "count": 1, "resource": "blade", "target": "self"}
```


```json
{"action": "gain_resource", "card_type": "member_card", "count": 1, "exclude_self": true, "group_names": ["Liella!"], "resource": "blade", "target_count": 1}
```

- {icon_blade.png|ブレード}を得る (x1)

```json
{"card_count": 1, "cards": ["PL!SP-sd2-022-SD2 | 鬼塚冬毬 (ab#0)"], "effect": {"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart03"], "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "heart"}, "is_null": false, "triggers": "自動", "use_limit": 1}
```


```json
{"action": "gain_resource", "condition": {"movement": "moved", "movement_state": "has_moved", "type": "movement_condition"}, "count": 1, "duration": "live_end", "heart_colors": ["heart03"], "parenthetical": ["対戦相手のカードの効果でも発動する。"], "resource": "heart"}
```

- このメンバーがエリアを移動したとき、ライブ終了時まで、{heart_03.png|heart03}を得る (x1)
