# 自動 Abilities Grouped by Sub-Trigger Type

**Total 自動 abilities: 56**
**Unique sub-trigger types: 1**

## [UNKNOWN (no bracket condition)] — 56 abilities

### 1. PL!S-bp2-007-R＋ | 国木田花丸 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く。`
- **triggerless_text**: `エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く。`
- **parsed effect**: {
  "text": "エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く",
  "condition": {
    "type": "compound",
    "operator": "and",
    "conditions": [
      {
        "type": "card_count_condition",
        "count": 1,
        "operator": ">=",
        "text": "エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、",
        "location": "revealed_cards",
        "card_type": "live_card",
        "target": "self"
      },
      {
        "type": "comparison_condition",
        "resource_type": "hand_count",
        "location": "hand",
        "count": 7,
        "operator": "<=",
        "text": "自分の手札が7枚以下の場合"
      }
    ],
    "text": "エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合"
  },
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand"
}
- **cards**:
  - `PL!S-bp2-007-R＋ | 国木田花丸 (ab#0)`
  - `PL!S-bp2-007-P | 国木田花丸 (ab#0)`
  - `PL!S-bp2-007-P＋ | 国木田花丸 (ab#0)`
  - `PL!S-bp2-007-SEC | 国木田花丸 (ab#0)`

### 2. PL!N-bp3-005-R＋ | 宮下 愛 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く。`
- **triggerless_text**: `このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く。`
- **parsed effect**: {
  "text": "このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く",
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
- **cards**:
  - `PL!N-bp3-005-R＋ | 宮下 愛 (ab#0)`
  - `PL!N-bp3-005-P | 宮下 愛 (ab#0)`
  - `PL!N-bp3-005-P＋ | 宮下 愛 (ab#0)`
  - `PL!N-bp3-005-SEC | 宮下 愛 (ab#0)`

### 3. PL!SP-bp4-011-R＋ | 鬼塚冬毬 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のメンバー1人をウェイトにする。`
- **triggerless_text**: `このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のメンバー1人をウェイトにする。`
- **parsed effect**: {
  "text": "このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のメンバー1人をウェイトにする",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーが登場か、エリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "source": "stage",
  "state_change": "wait",
  "count": 1,
  "card_type": "member_card",
  "target": "opponent",
  "action": "change_state",
  "original_value": true,
  "blade_limit": 3,
  "blade_limit_operator": "<="
}
- **cards**:
  - `PL!SP-bp4-011-R＋ | 鬼塚冬毬 (ab#0)`
  - `PL!SP-bp4-011-P | 鬼塚冬毬 (ab#0)`
  - `PL!SP-bp4-011-P＋ | 鬼塚冬毬 (ab#0)`
  - `PL!SP-bp4-011-SEC | 鬼塚冬毬 (ab#0)`

### 4. PL!-bp5-004-R＋ | 園田海未 (ab#1) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る。`
- **parsed effect**: {
  "text": "自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る",
  "condition": {
    "type": "card_count_condition",
    "count": 3,
    "operator": ">=",
    "text": "自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合",
    "negation": true,
    "location": "revealed_cards",
    "card_type": "member_card",
    "target": "self",
    "comparison_target": "self",
    "card_property": "has_blade_heart"
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "duration": "live_end",
  "heart_type": "all"
}
- **cards**:
  - `PL!-bp5-004-R＋ | 園田海未 (ab#1)`
  - `PL!-bp5-004-P | 園田海未 (ab#1)`
  - `PL!-bp5-004-AR | 園田海未 (ab#1)`
  - `PL!-bp5-004-SEC | 園田海未 (ab#1)`

### 5. PL!N-bp5-001-R＋ | 上原歩夢 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。`
- **parsed effect**: {
  "text": "自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る",
  "action": "sequential",
  "actions": [
    {
      "text": "自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る",
      "condition": {
        "type": "card_count_condition",
        "count": 3,
        "operator": ">=",
        "text": "自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合",
        "unit": "types",
        "location": "revealed_cards",
        "target": "self",
        "comparison_target": "self",
        "heart_colors": [
          "heart01",
          "heart02",
          "heart03",
          "heart04",
          "heart05",
          "heart06"
        ]
      },
      "action": "gain_resource",
      "resource": "heart",
      "count": 1,
      "heart_colors": [
        "heart01"
      ],
      "duration": "live_end"
    },
    {
      "text": "6種類以上ある場合、ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る",
      "condition": {
        "type": "card_count_condition",
        "count": 6,
        "operator": ">=",
        "text": "6種類以上ある場合",
        "unit": "types"
      },
      "action": "gain_ability",
      "ability_gain": "ライブの合計スコアを+1する。",
      "duration": "live_end"
    }
  ],
  "heart_colors": [
    "heart01",
    "heart02",
    "heart03",
    "heart04",
    "heart05",
    "heart06"
  ]
}
- **cards**:
  - `PL!N-bp5-001-R＋ | 上原歩夢 (ab#0)`
  - `PL!N-bp5-001-P | 上原歩夢 (ab#0)`
  - `PL!N-bp5-001-AR | 上原歩夢 (ab#0)`
  - `PL!N-bp5-001-SEC | 上原歩夢 (ab#0)`

### 6. PL!N-bp5-005-R＋ | 宮下 愛 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、さらにカードを1枚引く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、さらにカードを1枚引く。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、さらにカードを1枚引く",
  "action": "sequential",
  "actions": [
    {
      "text": "このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする",
      "condition": {
        "type": "location_condition",
        "card_type": "member_card",
        "location": "stage",
        "text": "このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合",
        "negation": true
      },
      "state_change": "active",
      "count": 2,
      "action": "change_state",
      "card_type": "energy_card"
    },
    {
      "text": "コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、カードを1枚引く",
      "condition": {
        "type": "location_condition",
        "card_type": "member_card",
        "location": "stage",
        "text": "コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合",
        "negation": true
      },
      "count": 1,
      "action": "draw_card",
      "source": "deck",
      "destination": "hand",
      "group_names": [
        "虹ヶ咲"
      ]
    }
  ],
  "group_names": [
    "虹ヶ咲"
  ]
}
- **cards**:
  - `PL!N-bp5-005-R＋ | 宮下 愛 (ab#0)`
  - `PL!N-bp5-005-P | 宮下 愛 (ab#0)`
  - `PL!N-bp5-005-AR | 宮下 愛 (ab#0)`
  - `PL!N-bp5-005-SEC | 宮下 愛 (ab#0)`

### 7. PL!SP-bp5-004-R＋ | 平安名すみれ (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき、カードを1枚引き、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **triggerless_text**: `自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき、カードを1枚引き、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **parsed effect**: {
  "text": "自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき、カードを1枚引き、ライブ終了時まで、{{heart_02.png|heart02}}を得る",
  "condition": {
    "type": "movement_condition",
    "text": "自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき",
    "movement": "moves",
    "self_effect_only": true,
    "energy_placed": true
  },
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
      "text": "{{heart_02.png|heart02}}を得る",
      "duration": "live_end",
      "action": "gain_resource",
      "resource": "heart",
      "count": 1,
      "heart_colors": [
        "heart02"
      ]
    }
  ],
  "heart_colors": [
    "heart02"
  ]
}
- **cards**:
  - `PL!SP-bp5-004-R＋ | 平安名すみれ (ab#0)`
  - `PL!SP-bp5-004-P | 平安名すみれ (ab#0)`
  - `PL!SP-bp5-004-AR | 平安名すみれ (ab#0)`
  - `PL!SP-bp5-004-SEC | 平安名すみれ (ab#0)`

### 8. PL!SP-bp5-005-R＋ | 葉月 恋 (ab#1) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、それらのカードの中から1枚手札に加える。`
- **triggerless_text**: `自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、それらのカードの中から1枚手札に加える。`
- **parsed effect**: {
  "text": "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、それらのカードの中から1枚手札に加える",
  "action": "conditional_on_optional",
  "conditional": true,
  "trigger_type": "each_time",
  "trigger_condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれる",
    "location": "discard",
    "target": "self",
    "source": "preceding_moved"
  },
  "optional_action": {
    "text": "{{icon_energy.png|E}}支払ってもよい。",
    "action": "pay_energy",
    "energy": 1,
    "count": 1
  },
  "conditional_action": {
    "text": "それらのカードの中から1枚手札に加える",
    "source": "those_cards",
    "destination": "hand",
    "count": 1,
    "action": "move_cards",
    "card_type": "card"
  }
}
- **cards**:
  - `PL!SP-bp5-005-R＋ | 葉月 恋 (ab#1)`
  - `PL!SP-bp5-005-P | 葉月 恋 (ab#1)`
  - `PL!SP-bp5-005-AR | 葉月 恋 (ab#1)`
  - `PL!SP-bp5-005-SEC | 葉月 恋 (ab#1)`

### 9. PL!HS-bp5-003-R＋ | 大沢瑠璃乃 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい",
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "count": 1,
  "card_type": "member_card",
  "optional": true,
  "action": "position_change",
  "target": null,
  "target_member": "select"
}
- **cards**:
  - `PL!HS-bp5-003-R＋ | 大沢瑠璃乃 (ab#0)`
  - `PL!HS-bp5-003-P | 大沢瑠璃乃 (ab#0)`
  - `PL!HS-bp5-003-AR | 大沢瑠璃乃 (ab#0)`
  - `PL!HS-bp5-003-SEC | 大沢瑠璃乃 (ab#0)`

### 10. PL!S-bp6-002-R＋ | 桜内梨子 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。`
- **triggerless_text**: `『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。`
- **parsed effect**: {
  "text": "『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい",
  "condition": {
    "text": "『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき",
    "target": "self",
    "location": "live_card_zone",
    "card_type": "live_card",
    "group_names": [
      "Aqours"
    ],
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "source": "those_cards",
  "destination": "deck_top_or_bottom",
  "card_type": "live_card",
  "optional": true,
  "action": "move_cards",
  "count": 1,
  "group_names": [
    "Aqours"
  ]
}
- **cards**:
  - `PL!S-bp6-002-R＋ | 桜内梨子 (ab#0)`
  - `PL!S-bp6-002-P | 桜内梨子 (ab#0)`
  - `PL!S-bp6-002-P＋ | 桜内梨子 (ab#0)`
  - `PL!S-bp6-002-SEC | 桜内梨子 (ab#0)`

### 11. PL!SP-sd2-002-P | 唐 可可 (ab#1) (shared by 3 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart06"
  ],
  "duration": "live_end",
  "parenthetical": [
    "対戦相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-sd2-002-P | 唐 可可 (ab#1)`
  - `PL!SP-sd2-002-SD2 | 唐 可可 (ab#1)`
  - `PL!SP-sd2-013-SD2 | 唐 可可 (ab#0)`

### 12. PL!-PR-001-PR | 高坂穂乃果 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、メンバー1人をアクティブにしてもよい。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、メンバー1人をアクティブにしてもよい。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、メンバー1人をアクティブにしてもよい",
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "count": 1,
  "card_type": "member_card",
  "optional": true,
  "action": "change_state",
  "state_change": "active"
}
- **cards**:
  - `PL!-PR-001-PR | 高坂穂乃果 (ab#0)`
  - `PL!-PR-002-PR | 絢瀬絵里 (ab#0)`

### 13. PL!S-PR-040-PR | 国木田花丸 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}{{heart_04.png|heart04}}を得る。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}{{heart_04.png|heart04}}を得る。`
- **parsed effect**: {
  "text": "自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}{{heart_04.png|heart04}}を得る",
  "condition": {
    "type": "card_count_condition",
    "count": 3,
    "operator": ">=",
    "text": "自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合",
    "location": "revealed_cards",
    "card_type": "member_card",
    "target": "self",
    "comparison_target": "self"
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 2,
  "heart_colors": [
    "heart01",
    "heart04"
  ],
  "duration": "live_end",
  "filter_targets_by_heart_colors": true,
  "group_reference": "same_group_name"
}
- **cards**:
  - `PL!S-PR-040-PR | 国木田花丸 (ab#0)`
  - `PL!N-PR-023-PR | 上原歩夢 (ab#0)`

### 14. PL!SP-pb1-006-R | 桜小路きな子 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "duration": "live_end",
  "action": "gain_resource",
  "resource": "blade",
  "count": 2,
  "trigger_type": "each_time",
  "parenthetical": [
    "対戦相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-pb1-006-R | 桜小路きな子 (ab#0)`
  - `PL!SP-pb1-006-P＋ | 桜小路きな子 (ab#0)`

### 15. PL!S-bp2-002-R | 桜内梨子 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『Aqours』のライブカードを1枚手札に加える。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『Aqours』のライブカードを1枚手札に加える。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『Aqours』のライブカードを1枚手札に加える",
  "action": "sequential",
  "actions": [
    {
      "text": "手札を1枚控え室に置いてもよい。",
      "source": "hand",
      "destination": "discard",
      "count": 1,
      "optional": true,
      "action": "move_cards",
      "card_type": "card"
    },
    {
      "text": "自分の控え室から『Aqours』のライブカードを1枚手札に加える",
      "source": "discard",
      "destination": "hand",
      "count": 1,
      "card_type": "live_card",
      "target": "self",
      "group_names": [
        "Aqours"
      ],
      "action": "move_cards"
    }
  ],
  "conditional": true,
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "group_names": [
    "Aqours"
  ]
}
- **cards**:
  - `PL!S-bp2-002-R | 桜内梨子 (ab#0)`
  - `PL!S-bp2-002-P | 桜内梨子 (ab#0)`

### 16. PL!S-bp2-003-R | 松浦果南 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、{{heart_03.png|緑ハート}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、{{heart_03.png|緑ハート}}を得る。`
- **parsed effect**: {
  "text": "エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、{{heart_03.png|緑ハート}}を得る",
  "condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "エールにより公開された自分のカードの中にライブカードが1枚以上あるとき",
    "location": "revealed_cards",
    "card_type": "live_card",
    "target": "self"
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "duration": "live_end",
  "heart_colors": [
    "heart03"
  ]
}
- **cards**:
  - `PL!S-bp2-003-R | 松浦果南 (ab#0)`
  - `PL!S-bp2-003-P | 松浦果南 (ab#0)`

### 17. PL!S-bp2-004-R | 黒澤ダイヤ (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **triggerless_text**: `エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **parsed effect**: {
  "text": "エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う",
  "action": "conditional_on_result",
  "primary_effect": {
    "text": "エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい",
    "source": "revealed_cards",
    "dynamic_count": {
      "type": "revealed_cards",
      "reference": "previous_reveal"
    },
    "destination": "discard",
    "card_type": "card",
    "optional": true,
    "action": "move_cards",
    "all": true,
    "condition": {
      "type": "location_condition",
      "location": "revealed_cards",
      "target": "self",
      "text": "エールにより公開された自分のカードの中にライブカードがないとき",
      "negation": true
    }
  },
  "result_condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "これにより1枚以上のカードが控え室に置かれた場合",
    "source": "preceding_moved"
  },
  "followup_action": {
    "text": "そのエールで得たブレードハートを失い、もう一度エールを行う",
    "action": "re_yell",
    "lose_blade_hearts": true
  },
  "all": true
}
- **cards**:
  - `PL!S-bp2-004-R | 黒澤ダイヤ (ab#0)`
  - `PL!S-bp2-004-P | 黒澤ダイヤ (ab#0)`

### 18. PL!SP-bp2-003-R | 嵐 千砂都 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "source": "energy_deck",
  "destination": "energy_zone",
  "state_change": "wait",
  "count": 1,
  "card_type": "energy_card",
  "target": "self",
  "action": "move_cards"
}
- **cards**:
  - `PL!SP-bp2-003-R | 嵐 千砂都 (ab#0)`
  - `PL!SP-bp2-003-P | 嵐 千砂都 (ab#0)`

### 19. PL!-pb1-015-R | 西木野真姫 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く。`
- **triggerless_text**: `自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く。`
- **parsed effect**: {
  "text": "自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く",
  "condition": {
    "type": "state_change_condition",
    "text": "自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき",
    "from_state": "active",
    "to_state": "wait",
    "target": "opponent",
    "location": "stage",
    "card_type": "member_card",
    "operator": "<=",
    "comparison_type": "cost",
    "cost_limit": 4
  },
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand"
}
- **cards**:
  - `PL!-pb1-015-R | 西木野真姫 (ab#1)`
  - `PL!-pb1-015-P＋ | 西木野真姫 (ab#1)`

### 20. PL!SP-bp4-007-R | 米女メイ (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の『Liella!』のライブカードを1枚手札に加える。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の『Liella!』のライブカードを1枚手札に加える。`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の『Liella!』のライブカードを1枚手札に加える",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "source": "discard",
  "destination": "hand",
  "cost_limit": 3,
  "cost_limit_operator": "<=",
  "count": 1,
  "card_type": "live_card",
  "target": "self",
  "group_names": [
    "Liella!"
  ],
  "action": "move_cards"
}
- **cards**:
  - `PL!SP-bp4-007-R | 米女メイ (ab#0)`
  - `PL!SP-bp4-007-P | 米女メイ (ab#0)`

### 21. PL!N-pb1-005-R | 宮下 愛 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージにコスト10のメンバーが登場したとき、カードを1枚引く。`
- **triggerless_text**: `自分のステージにコスト10のメンバーが登場したとき、カードを1枚引く。`
- **parsed effect**: {
  "text": "自分のステージにコスト10のメンバーが登場したとき、カードを1枚引く",
  "condition": {
    "type": "appearance_condition",
    "appearance": true,
    "text": "自分のステージにコスト10のメンバーが登場したとき",
    "location": "stage",
    "target": "self"
  },
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand"
}
- **cards**:
  - `PL!N-pb1-005-R | 宮下 愛 (ab#0)`
  - `PL!N-pb1-005-P＋ | 宮下 愛 (ab#0)`

### 22. PL!N-pb1-012-R | 鐘 嵐珠 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **triggerless_text**: `自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **parsed effect**: {
  "text": "自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く",
  "condition": {
    "type": "appearance_condition",
    "appearance": true,
    "text": "自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき",
    "location": "stage",
    "exclude_self": true,
    "target": "self"
  },
  "source": "energy_deck",
  "destination": "energy_zone",
  "state_change": "wait",
  "count": 1,
  "card_type": "energy_card",
  "target": "self",
  "action": "move_cards"
}
- **cards**:
  - `PL!N-pb1-012-R | 鐘 嵐珠 (ab#0)`
  - `PL!N-pb1-012-P＋ | 鐘 嵐珠 (ab#0)`

### 23. PL!S-bp5-111-R | 鹿角聖良 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがエリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が2つ以下のメンバー1人をウェイトにする。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が2つ以下のメンバー1人をウェイトにする。`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が2つ以下のメンバー1人をウェイトにする",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "source": "stage",
  "state_change": "wait",
  "count": 1,
  "card_type": "member_card",
  "target": "opponent",
  "action": "change_state",
  "original_value": true,
  "blade_limit": 2,
  "blade_limit_operator": "<="
}
- **cards**:
  - `PL!S-bp5-111-R | 鹿角聖良 (ab#1)`
  - `PL!S-bp5-111-P＋ | 鹿角聖良 (ab#1)`

### 24. PL!S-bp5-222-R | 鹿角理亞 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、エネルギーを2枚アクティブにする。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、エネルギーを2枚アクティブにする。`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、エネルギーを2枚アクティブにする",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "state_change": "active",
  "count": 2,
  "action": "change_state",
  "card_type": "energy_card"
}
- **cards**:
  - `PL!S-bp5-222-R | 鹿角理亞 (ab#1)`
  - `PL!S-bp5-222-P＋ | 鹿角理亞 (ab#1)`

### 25. PL!HS-pb1-001-R | 日野下花帆 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn2.png|ターン2回}}自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクティブにする。`
- **triggerless_text**: `自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクティブにする。`
- **parsed effect**: {
  "text": "自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクティブにする",
  "action": "conditional_on_optional",
  "conditional": true,
  "trigger_type": "each_time",
  "trigger_condition": {
    "type": "appearance_condition",
    "appearance": true,
    "text": "自分のステージにほかの『スリーズブーケ』のメンバーが登場する",
    "location": "stage",
    "exclude_self": true,
    "target": "self"
  },
  "exclude_self": true,
  "group_names": [
    "スリーズブーケ"
  ],
  "conditional_action": {
    "text": "エネルギーを2枚アクティブにする",
    "state_change": "active",
    "count": 2,
    "action": "change_state",
    "card_type": "energy_card",
    "exclude_self": true
  }
}
- **cards**:
  - `PL!HS-pb1-001-R | 日野下花帆 (ab#0)`
  - `PL!HS-pb1-001-P＋ | 日野下花帆 (ab#0)`

### 26. PL!HS-pb1-003-R | 大沢瑠璃乃 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn2.png|ターン2回}}自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る。`
- **parsed effect**: {
  "text": "自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る",
  "duration": "live_end",
  "action": "sequential",
  "actions": [
    {
      "action": "gain_resource",
      "resource": "blade",
      "count": 1,
      "duration": "live_end",
      "text": "自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る"
    },
    {
      "action": "gain_resource",
      "resource": "heart",
      "heart_colors": [
        "heart01"
      ],
      "count": 1,
      "duration": "live_end",
      "text": "自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る"
    }
  ],
  "heart_colors": [
    "heart01"
  ],
  "count": 2,
  "trigger_type": "each_time",
  "trigger_condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "自分の手札からカードが1枚以上控え室に置かれる",
    "location": "discard",
    "target": "self",
    "source": "preceding_moved"
  }
}
- **cards**:
  - `PL!HS-pb1-003-R | 大沢瑠璃乃 (ab#1)`
  - `PL!HS-pb1-003-P＋ | 大沢瑠璃乃 (ab#1)`

### 27. PL!HS-pb1-009-R | 日野下花帆 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{center.png|センター}}{{turn2.png|ターン2回}}自分のステージに『蓮ノ空』のメンバーが登場するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `{{center.png|センター}}自分のステージに『蓮ノ空』のメンバーが登場するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **parsed effect**: {
  "text": "自分のステージに『蓮ノ空』のメンバーが登場するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "duration": "live_end",
  "action": "gain_resource",
  "resource": "blade",
  "count": 2,
  "trigger_type": "each_time",
  "trigger_condition": {
    "type": "appearance_condition",
    "appearance": true,
    "text": "{{center.png|センター}}自分のステージに『蓮ノ空』のメンバーが登場する",
    "location": "stage",
    "target": "self",
    "position": "center"
  },
  "activation_position": "center",
  "position": "center"
}
- **cards**:
  - `PL!HS-pb1-009-R | 日野下花帆 (ab#0)`
  - `PL!HS-pb1-009-P＋ | 日野下花帆 (ab#0)`

### 28. PL!HS-bp6-007-R | セラス 柳田 リリエンフェルト (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。`
- **triggerless_text**: `自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。`
- **parsed effect**: {
  "text": "自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする",
  "condition": {
    "type": "appearance_condition",
    "appearance": true,
    "text": "自分のステージに『EdelNote』のメンバーが登場したとき",
    "location": "stage",
    "target": "self"
  },
  "source": "stage",
  "state_change": "wait",
  "state": "active",
  "count": 1,
  "card_type": "member_card",
  "action": "change_state",
  "target": "opponent",
  "action_by": "opponent",
  "group_names": [
    "EdelNote"
  ]
}
- **cards**:
  - `PL!HS-bp6-007-R | セラス 柳田 リリエンフェルト (ab#0)`
  - `PL!HS-bp6-007-P | セラス 柳田 リリエンフェルト (ab#0)`

### 29. PL!N-PR-025-PR | 優木せつ菜 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn2.png|ターン2回}}自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く。`
- **triggerless_text**: `自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く。`
- **parsed effect**: {
  "text": "自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く",
  "condition": {
    "type": "movement_condition",
    "movement": "baton_touch",
    "target": "self",
    "baton_touch_trigger": true,
    "text": "自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき",
    "location": "stage",
    "exclude_self": true
  },
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand",
  "exclude_self": true
}
- **cards**:
  - `PL!N-PR-025-PR | 優木せつ菜 (ab#0)`

### 30. PL!SP-pb1-020-N | 鬼塚夏美 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動するたび、カードを1枚引く",
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand",
  "trigger_type": "each_time",
  "trigger_condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動する",
    "movement": "moves"
  },
  "parenthetical": [
    "対戦相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-pb1-020-N | 鬼塚夏美 (ab#0)`

### 31. PL!SP-bp2-015-N | 平安名すみれ (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。`
- **parsed effect**: {
  "text": "エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る",
  "condition": {
    "type": "location_condition",
    "location": "revealed_cards",
    "target": "self",
    "text": "エールにより公開された自分のカードの中にブレードハートを持つカードがないとき",
    "negation": true,
    "card_property": "has_blade_heart",
    "heart_colors": [
      "heart06"
    ]
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart06"
  ],
  "duration": "live_end",
  "filter_targets_by_heart_colors": true
}
- **cards**:
  - `PL!SP-bp2-015-N | 平安名すみれ (ab#0)`

### 32. PL!SP-bp2-020-N | 鬼塚夏美 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **parsed effect**: {
  "text": "エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る",
  "condition": {
    "type": "location_condition",
    "location": "revealed_cards",
    "target": "self",
    "text": "エールにより公開された自分のカードの中にブレードハートを持つカードがないとき",
    "negation": true,
    "card_property": "has_blade_heart",
    "heart_colors": [
      "heart02"
    ]
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart02"
  ],
  "duration": "live_end",
  "filter_targets_by_heart_colors": true
}
- **cards**:
  - `PL!SP-bp2-020-N | 鬼塚夏美 (ab#0)`

### 33. PL!SP-bp2-021-N | ウィーン・マルガレーテ (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。`
- **parsed effect**: {
  "text": "エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る",
  "condition": {
    "type": "location_condition",
    "location": "revealed_cards",
    "target": "self",
    "text": "エールにより公開された自分のカードの中にブレードハートを持つカードがないとき",
    "negation": true,
    "card_property": "has_blade_heart",
    "heart_colors": [
      "heart03"
    ]
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart03"
  ],
  "duration": "live_end",
  "filter_targets_by_heart_colors": true
}
- **cards**:
  - `PL!SP-bp2-021-N | ウィーン・マルガレーテ (ab#0)`

### 34. PL!HS-bp2-012-N | 乙宗 梢 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く",
  "action": "look_and_select",
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "look_action": {
    "text": "自分のデッキの上からカードを5枚見る。",
    "source": "deck_top",
    "count": 5,
    "target": "self",
    "action": "look_at"
  },
  "select_action": {
    "action": "select_cards",
    "destination": "hand",
    "discard_remaining": true,
    "reveal": true,
    "count": 1,
    "card_type": "member_card",
    "optional": true,
    "text": "このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く"
  }
}
- **cards**:
  - `PL!HS-bp2-012-N | 乙宗 梢 (ab#0)`

### 35. PL!HS-bp2-013-N | 夕霧綴理 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く",
  "action": "look_and_select",
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "look_action": {
    "text": "自分のデッキの上からカードを5枚見る。",
    "source": "deck_top",
    "count": 5,
    "target": "self",
    "action": "look_at"
  },
  "select_action": {
    "action": "select_cards",
    "destination": "hand",
    "discard_remaining": true,
    "reveal": true,
    "count": 1,
    "card_type": "live_card",
    "optional": true,
    "text": "このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く"
  }
}
- **cards**:
  - `PL!HS-bp2-013-N | 夕霧綴理 (ab#0)`

### 36. PL!HS-bp2-015-N | 藤島 慈 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を1枚控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を1枚控え室に置く。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を1枚控え室に置く",
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
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
  ]
}
- **cards**:
  - `PL!HS-bp2-015-N | 藤島 慈 (ab#0)`

### 37. PL!S-bp3-020-L | ダイスキだったらダイジョウブ！ (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **triggerless_text**: `エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **parsed effect**: {
  "text": "エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。そのエールで得たブレードハートを失い、もう一度エールを行う",
  "condition": {
    "type": "card_count_condition",
    "count": 1,
    "operator": ">=",
    "text": "エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合",
    "target": "self"
  },
  "action": "sequential",
  "actions": [
    {
      "text": "それらのカードをすべて控え室に置いてもよい",
      "destination": "discard",
      "optional": true,
      "action": "move_cards",
      "source": "revealed_cards",
      "card_type": "card",
      "dynamic_count": {
        "type": "revealed_cards",
        "reference": "previous_reveal"
      },
      "all": true
    },
    {
      "text": "そのエールで得たブレードハートを失い、もう一度エールを行う",
      "action": "re_yell",
      "lose_blade_hearts": true,
      "all": true
    }
  ],
  "all": true
}
- **cards**:
  - `PL!S-bp3-020-L | ダイスキだったらダイジョウブ！ (ab#0)`

### 38. PL!N-bp4-018-N | 近江彼方 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき、カードを1枚引き、手札を1枚控え室に置く。`
- **triggerless_text**: `自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき、カードを1枚引き、手札を1枚控え室に置く。`
- **parsed effect**: {
  "text": "自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき、カードを1枚引き、手札を1枚控え室に置く",
  "condition": {
    "type": "state_change_condition",
    "text": "自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき",
    "from_state": "active",
    "to_state": "wait",
    "phase": "main",
    "target": "self",
    "card_type": "member_card"
  },
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
  ]
}
- **cards**:
  - `PL!N-bp4-018-N | 近江彼方 (ab#0)`

### 39. PL!N-bp4-026-L | DIVE! (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}自分のメインフェイズにこのカードが控え室から手札に加えられたとき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。そうした場合、次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る。`
- **triggerless_text**: `自分のメインフェイズにこのカードが控え室から手札に加えられたとき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。そうした場合、次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る。`
- **parsed effect**: {
  "text": "自分のメインフェイズにこのカードが控え室から手札に加えられたとき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。そうした場合、次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る",
  "action": "sequential",
  "actions": [
    {
      "text": "自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。",
      "source": "hand",
      "destination": "live_card_zone",
      "count": 1,
      "card_type": "live_card",
      "target": "self",
      "characters": [
        "DIVE!"
      ],
      "quoted_text": {
        "text": "DIVE!",
        "quoted_type": "character"
      },
      "optional": true,
      "action": "move_cards"
    },
    {
      "text": "次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る",
      "count": 1,
      "card_type": "live_card",
      "action": "set_card_identity"
    }
  ],
  "conditional": true,
  "condition": {
    "text": "自分のメインフェイズにこのカードが控え室から手札に加えられたとき",
    "target": "self",
    "location": "discard",
    "locations": [
      "discard",
      "hand"
    ],
    "type": "location_condition"
  }
}
- **cards**:
  - `PL!N-bp4-026-L | DIVE! (ab#0)`

### 40. PL!N-bp4-026-L | DIVE! (ab#1) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このカードが表向きでライブカード置き場に置かれたとき、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `このカードが表向きでライブカード置き場に置かれたとき、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **parsed effect**: {
  "text": "このカードが表向きでライブカード置き場に置かれたとき、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
  "condition": {
    "text": "このカードが表向きでライブカード置き場に置かれたとき",
    "location": "live_card_zone",
    "card_type": "live_card",
    "type": "location_condition"
  },
  "count": 2,
  "card_type": "member_card",
  "target": "self",
  "group_names": [
    "虹ヶ咲"
  ],
  "target_count": 1,
  "action": "gain_resource",
  "resource": "blade",
  "duration": "live_end"
}
- **cards**:
  - `PL!N-bp4-026-L | DIVE! (ab#1)`

### 41. PL!SP-bp4-016-N | 葉月 恋 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれるたび、ライブ終了時まで、{{heart_06.png|heart06}}を得る。(相手のカードの効果でも発動する。)`
- **triggerless_text**: `カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれるたび、ライブ終了時まで、{{heart_06.png|heart06}}を得る。(相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれるたび、ライブ終了時まで、{{heart_06.png|heart06}}を得る",
  "duration": "live_end",
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart06"
  ],
  "trigger_type": "each_time",
  "trigger_condition": {
    "text": "カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれる",
    "target": "self",
    "location": "energy_zone",
    "resource_type": "energy",
    "card_type": "energy_card",
    "type": "comparison_condition"
  },
  "parenthetical": [
    "相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-bp4-016-N | 葉月 恋 (ab#0)`

### 42. PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}自分のステージにいるメンバーの{{live_start.png|ライブ開始時}}能力が解決するたび、そのメンバーが{{icon_all.png|ハート}}を持たない場合、ライブ終了時まで、そのメンバーは{{icon_all.png|ハート}}を得る。`
- **triggerless_text**: `自分のステージにいるメンバーの{{live_start.png|ライブ開始時}}能力が解決するたび、そのメンバーが{{icon_all.png|ハート}}を持たない場合、ライブ終了時まで、そのメンバーは{{icon_all.png|ハート}}を得る。`
- **parsed effect**: {
  "text": "自分のステージにいるメンバーの{{live_start.png|ライブ開始時}}能力が解決するたび、そのメンバーが{{icon_all.png|ハート}}を持たない場合、ライブ終了時まで、そのメンバーは{{icon_all.png|ハート}}を得る",
  "condition": {
    "type": "location_condition",
    "card_type": "member_card",
    "location": "stage",
    "text": "そのメンバーが{{icon_all.png|ハート}}を持たない場合",
    "negation": true,
    "heart_type": "all"
  },
  "card_type": "member_card",
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "duration": "live_end",
  "trigger_type": "each_time",
  "trigger_condition": {
    "text": "自分のステージにいるメンバーの{{live_start.png|ライブ開始時}}能力が解決する",
    "target": "self",
    "location": "stage",
    "card_type": "member_card",
    "type": "location_condition"
  },
  "heart_type": "all"
}
- **cards**:
  - `PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#0)`

### 43. PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#1) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}自分のステージにいるメンバーの{{live_success.png|ライブ成功時}}能力が解決するたび、カードを1枚引く。`
- **triggerless_text**: `自分のステージにいるメンバーの{{live_success.png|ライブ成功時}}能力が解決するたび、カードを1枚引く。`
- **parsed effect**: {
  "text": "自分のステージにいるメンバーの{{live_success.png|ライブ成功時}}能力が解決するたび、カードを1枚引く",
  "count": 1,
  "action": "draw_card",
  "source": "deck",
  "destination": "hand",
  "trigger_type": "each_time",
  "trigger_condition": {
    "text": "自分のステージにいるメンバーの{{live_success.png|ライブ成功時}}能力が解決する",
    "target": "self",
    "location": "stage",
    "card_type": "member_card",
    "type": "location_condition"
  }
}
- **cards**:
  - `PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#1)`

### 44. PL!HS-bp5-014-N | 安養寺 姫芽 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "action": "gain_resource",
  "resource": "blade",
  "count": 1,
  "duration": "live_end"
}
- **cards**:
  - `PL!HS-bp5-014-N | 安養寺 姫芽 (ab#0)`

### 45. PL!HS-sd1-001-SD | 日野下花帆 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき、エネルギーを2枚アクティブにする。`
- **triggerless_text**: `このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき、エネルギーを2枚アクティブにする。`
- **parsed effect**: {
  "text": "このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき、エネルギーを2枚アクティブにする",
  "condition": {
    "type": "movement_condition",
    "movement": "baton_touch",
    "target": "self",
    "baton_touch_trigger": true,
    "text": "このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき",
    "location": "discard",
    "cost_limit": 10,
    "cost_limit_operator": ">=",
    "group_names": [
      "蓮ノ空"
    ]
  },
  "state_change": "active",
  "count": 2,
  "action": "change_state",
  "card_type": "energy_card"
}
- **cards**:
  - `PL!HS-sd1-001-SD | 日野下花帆 (ab#0)`

### 46. PL!S-sd1-001-SD | 高海千歌 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、ライブ終了時まで、エールにより公開された自分のカードの中のライブカード1枚につき、{{heart_02.png|heart02}}を得る。この能力では{{heart_02.png|heart02}}は3つまでしか得られない。`
- **triggerless_text**: `自分がエールしたとき、ライブ終了時まで、エールにより公開された自分のカードの中のライブカード1枚につき、{{heart_02.png|heart02}}を得る。この能力では{{heart_02.png|heart02}}は3つまでしか得られない。`
- **parsed effect**: {
  "text": "自分がエールしたとき、ライブ終了時まで、エールにより公開された自分のカードの中のライブカード1枚につき、{{heart_02.png|heart02}}を得る。この能力では{{heart_02.png|heart02}}は3つまでしか得られない",
  "count": 3,
  "action": "gain_resource",
  "resource": "heart",
  "heart_colors": [
    "heart02"
  ],
  "per_unit": true,
  "per_unit_count": 1,
  "per_unit_type": "枚",
  "duration": "live_end",
  "target": "self"
}
- **cards**:
  - `PL!S-sd1-001-SD | 高海千歌 (ab#0)`

### 47. PL!-bp6-020-L | Dancing stars on me! (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_start.png|ライブ開始時}}能力が解決したとき、そのメンバーをポジションチェンジする。`
- **triggerless_text**: `自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_start.png|ライブ開始時}}能力が解決したとき、そのメンバーをポジションチェンジする。`
- **parsed effect**: {
  "text": "自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_start.png|ライブ開始時}}能力が解決したとき、そのメンバーをポジションチェンジする",
  "condition": {
    "text": "自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_start.png|ライブ開始時}}能力が解決したとき",
    "target": "self",
    "location": "stage",
    "card_type": "member_card",
    "group_names": [
      "μ's"
    ],
    "position": "center",
    "type": "group_condition"
  },
  "card_type": "member_card",
  "action": "position_change",
  "target": null,
  "group_names": [
    "μ's"
  ],
  "position": "center"
}
- **cards**:
  - `PL!-bp6-020-L | Dancing stars on me! (ab#0)`

### 48. PL!-bp6-020-L | Dancing stars on me! (ab#1) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_success.png|ライブ成功時}}能力が解決したとき、そのメンバーがこのターン中に移動している場合、このカードのスコアを＋１する。`
- **triggerless_text**: `自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_success.png|ライブ成功時}}能力が解決したとき、そのメンバーがこのターン中に移動している場合、このカードのスコアを＋１する。`
- **parsed effect**: {
  "text": "自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_success.png|ライブ成功時}}能力が解決したとき、そのメンバーがこのターン中に移動している場合、このカードのスコアを+1する",
  "condition": {
    "type": "temporal_condition",
    "temporal": "this_turn",
    "condition": {
      "type": "has_moved"
    },
    "text": "自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_success.png|ライブ成功時}}能力が解決したとき、そのメンバーがこのターン中に移動している場合",
    "card_type": "member_card",
    "position": "center"
  },
  "action": "modify_score",
  "value": 1,
  "operation": "add",
  "self_target": true,
  "group_names": [
    "μ's"
  ],
  "position": "center"
}
- **cards**:
  - `PL!-bp6-020-L | Dancing stars on me! (ab#1)`

### 49. PL!S-bp6-021-L | MIRAI TICKET (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いたカードのコスト５につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いたカードのコスト５につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない。`
- **parsed effect**: {
  "text": "自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いたカードのコスト5につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない",
  "action": "sequential",
  "actions": [
    {
      "text": "エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。",
      "source": "revealed_cards",
      "dynamic_count": {
        "type": "revealed_cards",
        "reference": "previous_reveal"
      },
      "destination": "discard",
      "count": 1,
      "card_type": "member_card",
      "target": "self",
      "group_names": [
        "Aqours"
      ],
      "optional": true,
      "max": true,
      "action": "move_cards"
    },
    {
      "text": "これにより控え室に置いたカードのコスト5につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない",
      "count": 4,
      "max": true,
      "action": "modify_score",
      "operation": "add",
      "max_repeats": 4,
      "per_unit": true,
      "per_unit_count": 5,
      "per_unit_type": "discard",
      "per_unit_source": "previous_moved_cards",
      "group_names": [
        "Aqours"
      ]
    }
  ],
  "conditional": true,
  "condition": {
    "text": "自分がエールしたとき",
    "type": "custom"
  },
  "group_names": [
    "Aqours"
  ]
}
- **cards**:
  - `PL!S-bp6-021-L | MIRAI TICKET (ab#0)`

### 50. PL!HS-bp6-017-N | 日野下花帆 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える",
  "action": "sequential",
  "actions": [
    {
      "text": "手札を1枚控え室に置いてもよい。",
      "source": "hand",
      "destination": "discard",
      "count": 1,
      "optional": true,
      "action": "move_cards",
      "card_type": "card"
    },
    {
      "text": "自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える",
      "source": "discard",
      "destination": "hand",
      "count": 1,
      "target": "self",
      "max": true,
      "action": "sequential",
      "actions": [
        {
          "text": "自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える",
          "action": "move_cards",
          "source": "discard",
          "destination": "hand",
          "card_type": "live_card",
          "count": 1,
          "max": true,
          "target": "self",
          "multiple_targets": true
        },
        {
          "text": "自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える",
          "action": "move_cards",
          "source": "discard",
          "destination": "hand",
          "card_type": "member_card",
          "count": 1,
          "max": true,
          "target": "self",
          "multiple_targets": true
        }
      ],
      "multiple_targets": true
    }
  ],
  "conditional": true,
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  }
}
- **cards**:
  - `PL!HS-bp6-017-N | 日野下花帆 (ab#0)`

### 51. PL!HS-bp6-018-N | 村野さやか (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を得る。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を得る",
  "action": "sequential",
  "actions": [
    {
      "text": "手札を1枚控え室に置いてもよい。",
      "source": "hand",
      "destination": "discard",
      "count": 1,
      "optional": true,
      "action": "move_cards",
      "card_type": "card",
      "heart_colors": [
        "heart05"
      ]
    },
    {
      "text": "自分のステージにいるメンバー1人は、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を得る",
      "duration": "live_end",
      "count": 1,
      "card_type": "member_card",
      "target": "self",
      "target_count": 1,
      "action": "gain_resource",
      "resource": "blade"
    }
  ],
  "conditional": true,
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
  "heart_colors": [
    "heart05"
  ]
}
- **cards**:
  - `PL!HS-bp6-018-N | 村野さやか (ab#0)`

### 52. PL!HS-bp6-019-N | 大沢瑠璃乃 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を2枚控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を2枚控え室に置く。`
- **parsed effect**: {
  "text": "このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を2枚控え室に置く",
  "condition": {
    "text": "このメンバーがステージから控え室に置かれたとき",
    "location": "discard",
    "card_type": "member_card",
    "type": "card_count_condition",
    "source": "preceding_moved",
    "operator": ">=",
    "count": 1
  },
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
      "text": "手札を2枚控え室に置く",
      "source": "hand",
      "destination": "discard",
      "count": 2,
      "action": "move_cards",
      "card_type": "card"
    }
  ]
}
- **cards**:
  - `PL!HS-bp6-019-N | 大沢瑠璃乃 (ab#0)`

### 53. PL!HS-bp6-027-L | 月夜見海月 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う。`
- **parsed effect**: {
  "text": "自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う",
  "action": "sequential",
  "actions": [
    {
      "text": "エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。",
      "source": "revealed_cards",
      "dynamic_count": {
        "type": "revealed_cards",
        "reference": "previous_reveal"
      },
      "destination": "discard",
      "count": 3,
      "target": "self",
      "group_names": [
        "蓮ノ空"
      ],
      "optional": true,
      "max": true,
      "action": "move_cards",
      "card_type": "card"
    },
    {
      "text": "これにより控え室に置いた数に等しい枚数のエールを追加で行う",
      "dynamic_count": {
        "type": "dynamic_count",
        "reference": "これにより控え室に置いた数",
        "mode": "equals"
      },
      "action": "modify_score",
      "operation": "add",
      "group_names": [
        "蓮ノ空"
      ]
    }
  ],
  "conditional": true,
  "condition": {
    "text": "自分がエールしたとき",
    "type": "custom"
  },
  "group_names": [
    "蓮ノ空"
  ]
}
- **cards**:
  - `PL!HS-bp6-027-L | 月夜見海月 (ab#0)`

### 54. PL!SP-sd2-011-SD2 | 鬼塚冬毬 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "action": "gain_resource",
  "resource": "blade",
  "count": 1,
  "duration": "live_end",
  "parenthetical": [
    "対戦相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-sd2-011-SD2 | 鬼塚冬毬 (ab#0)`

### 55. PL!SP-sd2-012-SD2 | 澁谷かのん (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart02"
  ],
  "duration": "live_end",
  "parenthetical": [
    "対戦相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-sd2-012-SD2 | 澁谷かのん (ab#0)`

### 56. PL!SP-sd2-022-SD2 | 鬼塚冬毬 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **parsed effect**: {
  "text": "このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る",
  "condition": {
    "type": "movement_condition",
    "text": "このメンバーがエリアを移動したとき",
    "movement": "moved",
    "movement_state": "has_moved"
  },
  "action": "gain_resource",
  "resource": "heart",
  "count": 1,
  "heart_colors": [
    "heart03"
  ],
  "duration": "live_end",
  "parenthetical": [
    "対戦相手のカードの効果でも発動する。"
  ]
}
- **cards**:
  - `PL!SP-sd2-022-SD2 | 鬼塚冬毬 (ab#0)`

---

