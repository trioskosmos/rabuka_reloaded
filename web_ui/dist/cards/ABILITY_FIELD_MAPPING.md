# Ability Field Mapping Report

This document describes all action types, condition types, and their fields found in abilities.json

## Action Types

### change_state

**Fields:**

- `action`
  - Example values: reveal, move_cards, change_state, draw
- `card_type`
  - Example values: live_card, member_card
- `count`
  - Example values: 1, 2, 6
- `state_change`
  - Example values: wait
- `text`
  - Example values: 自分の控え室から『μ's』のライブカードを1枚手札に加える, このカードを控え室からステージに登場させる, 手札にあるメンバーカードを好きな枚数公開する, 手札のライブカードを1枚公開し, デッキの一番下に置いてもよい

### draw

**Fields:**

- `action`
  - Example values: reveal, move_cards, change_state, draw
- `activation_condition`
  - Example values: この能力は、自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合のみ起動できる, この能力は、このカードが控え室にある場合のみ起動できる, この能力は、このカードが手札にある場合のみ起動できる
- `activation_condition_parsed`
- `card_type`
  - Example values: live_card, member_card
- `count`
  - Example values: 1, 2, 6
- `destination`
  - Example values: deck_bottom, hand, stage
- `group`
- `resource_icon_count`
  - Example values: 1
- `source`
  - Example values: hand, discard, deck
- `target`
  - Example values: self
- `text`
  - Example values: 自分の控え室から『μ's』のライブカードを1枚手札に加える, このカードを控え室からステージに登場させる, 手札にあるメンバーカードを好きな枚数公開する, 手札のライブカードを1枚公開し, デッキの一番下に置いてもよい

### move_cards

**Fields:**

- `action`
  - Example values: reveal, move_cards, change_state, draw
- `activation_condition`
  - Example values: この能力は、自分の成功ライブカード置き場にあるカードのスコアの合計が6以上の場合のみ起動できる, この能力は、このカードが控え室にある場合のみ起動できる, この能力は、このカードが手札にある場合のみ起動できる
- `activation_condition_parsed`
- `card_type`
  - Example values: live_card, member_card
- `count`
  - Example values: 1, 2, 6
- `destination`
  - Example values: deck_bottom, hand, stage
- `group`
- `optional`
  - Example values: True
- `source`
  - Example values: hand, discard, deck
- `target`
  - Example values: self
- `text`
  - Example values: 自分の控え室から『μ's』のライブカードを1枚手札に加える, このカードを控え室からステージに登場させる, 手札にあるメンバーカードを好きな枚数公開する, 手札のライブカードを1枚公開し, デッキの一番下に置いてもよい
- `type`
  - Example values: reveal, move_cards

### reveal

**Fields:**

- `action`
  - Example values: reveal, move_cards, change_state, draw
- `card_type`
  - Example values: live_card, member_card
- `count`
  - Example values: 1, 2, 6
- `source`
  - Example values: hand, discard, deck
- `text`
  - Example values: 自分の控え室から『μ's』のライブカードを1枚手札に加える, このカードを控え室からステージに登場させる, 手札にあるメンバーカードを好きな枚数公開する, 手札のライブカードを1枚公開し, デッキの一番下に置いてもよい
- `type`
  - Example values: reveal, move_cards

## Condition Types

### location_condition

**Fields:**

- `aggregate`
- `card_type`
- `comparison_type`
- `location`
- `operator`
- `source`
- `target`
- `text`
- `type`

### move_action_condition

**Fields:**

- `destination`
- `location`
- `source`
- `text`
- `type`

## Cost Types

### change_state

**Fields:**

- `card_type`
- `count`
- `exclude_self`
- `group`
- `max`
- `optional`
- `state_change`
- `text`
- `type`

### choice_condition

**Fields:**

- `options`
- `text`
- `type`

### energy_condition

**Fields:**

- `count`
- `destination`
- `text`
- `type`

### move_cards

**Fields:**

- `action`
- `card_type`
- `count`
- `destination`
- `group`
- `max`
- `optional`
- `source`
- `target`
- `text`
- `type`

### pay_energy

**Fields:**

- `count`
- `energy`
- `optional`
- `target`
- `text`
- `type`

### reveal

**Fields:**

- `action`
- `card_type`
- `count`
- `source`
- `text`
- `type`

### sequential_cost

**Fields:**

- `costs`
- `text`
- `type`

## Common Field Values

### Source Locations

| Value | Count |
|-------|-------|
| hand | 7 |
| discard | 5 |
| deck | 2 |

### Destination Locations

| Value | Count |
|-------|-------|
| deck_bottom | 3 |
| hand | 3 |
| stage | 1 |

### Card Types

| Value | Count |
|-------|-------|
| live_card | 6 |
| member_card | 3 |

### Target Players

| Value | Count |
|-------|-------|
| self | 7 |
