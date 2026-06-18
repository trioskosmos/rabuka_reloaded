# 自動 Ability Trigger Classification

**Total 自動 abilities: 56**
**Unique trigger types: 17**

## Summary Table

| Trigger Type | Count | Cards |
|---|---|---|
| on_yell | 13 | PL!S-bp2-007-R＋, PL!-bp5-004-R＋, PL!N-bp5-001-R＋, PL!S-PR-040-PR, PL!S-bp2-003-R, PL!S-bp2-004-R, PL!SP-bp2-015-N, PL!SP-bp2-020-N, PL!SP-bp2-021-N, PL!S-bp3-020-L, PL!S-sd1-001-SD, PL!S-bp6-021-L, PL!HS-bp6-027-L |
| on_area_move | 12 | PL!SP-bp4-011-R＋, PL!SP-sd2-002-P, PL!SP-pb1-006-R, PL!SP-bp2-003-R, PL!SP-bp4-007-R, PL!S-bp5-111-R, PL!S-bp5-222-R, PL!SP-pb1-020-N, PL!HS-bp5-014-N, PL!SP-sd2-011-SD2, PL!SP-sd2-012-SD2, PL!SP-sd2-022-SD2 |
| on_sent_to_discard_from_stage | 10 | PL!N-bp5-005-R＋, PL!HS-bp5-003-R＋, PL!-PR-001-PR, PL!S-bp2-002-R, PL!HS-bp2-012-N, PL!HS-bp2-013-N, PL!HS-bp2-015-N, PL!HS-bp6-017-N, PL!HS-bp6-018-N, PL!HS-bp6-019-N |
| on_ally_appear_on_stage | 4 | PL!N-bp3-005-R＋, PL!N-pb1-005-R, PL!N-pb1-012-R, PL!HS-bp6-007-R |
| on_state_changed_to_wait | 2 | PL!-pb1-015-R, PL!N-bp4-018-N |
| on_ally_appear_each_time | 2 | PL!HS-pb1-001-R, PL!HS-pb1-009-R |
| on_live_start_ability_resolved | 2 | PL!N-bp5-030-L, PL!-bp6-020-L |
| on_live_success_ability_resolved | 2 | PL!N-bp5-030-L, PL!-bp6-020-L |
| on_move_or_energy_placed | 1 | PL!SP-bp5-004-R＋ |
| on_any_to_discard_each_time | 1 | PL!SP-bp5-005-R＋ |
| on_live_card_zone_to_discard | 1 | PL!S-bp6-002-R＋ |
| on_hand_to_discard_each_time | 1 | PL!HS-pb1-003-R |
| on_baton_touch_appear | 1 | PL!N-PR-025-PR |
| on_discard_to_hand | 1 | PL!N-bp4-026-L |
| on_placed_in_live_card_zone | 1 | PL!N-bp4-026-L |
| on_energy_placed_each_time | 1 | PL!SP-bp4-016-N |
| on_baton_touch_to_discard | 1 | PL!HS-sd1-001-SD |

---

## on_yell (13 abilities)

### 1. PL!S-bp2-007-R＋ | 国木田花丸 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く。`
- **triggerless_text**: `エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、自分の手札が7枚以下の場合、カードを1枚引く。`
- **use_limit**: 1/turn
- **parsed condition type**: `compound`
- **parsed action type**: `draw_card`
- **cards**:
  - `PL!S-bp2-007-R＋ | 国木田花丸 (ab#0)`
  - `PL!S-bp2-007-P | 国木田花丸 (ab#0)`
  - `PL!S-bp2-007-P＋ | 国木田花丸 (ab#0)`
  - `PL!S-bp2-007-SEC | 国木田花丸 (ab#0)`

### 2. PL!-bp5-004-R＋ | 園田海未 (ab#1) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!-bp5-004-R＋ | 園田海未 (ab#1)`
  - `PL!-bp5-004-P | 園田海未 (ab#1)`
  - `PL!-bp5-004-AR | 園田海未 (ab#1)`
  - `PL!-bp5-004-SEC | 園田海未 (ab#1)`

### 3. PL!N-bp5-001-R＋ | 上原歩夢 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `none`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!N-bp5-001-R＋ | 上原歩夢 (ab#0)`
  - `PL!N-bp5-001-P | 上原歩夢 (ab#0)`
  - `PL!N-bp5-001-AR | 上原歩夢 (ab#0)`
  - `PL!N-bp5-001-SEC | 上原歩夢 (ab#0)`

### 4. PL!S-PR-040-PR | 国木田花丸 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}{{heart_04.png|heart04}}を得る。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードの中に同じグループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}{{heart_04.png|heart04}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!S-PR-040-PR | 国木田花丸 (ab#0)`
  - `PL!N-PR-023-PR | 上原歩夢 (ab#0)`

### 5. PL!S-bp2-003-R | 松浦果南 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、{{heart_03.png|緑ハート}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にライブカードが1枚以上あるとき、ライブ終了時まで、{{heart_03.png|緑ハート}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!S-bp2-003-R | 松浦果南 (ab#0)`
  - `PL!S-bp2-003-P | 松浦果南 (ab#0)`

### 6. PL!S-bp2-004-R | 黒澤ダイヤ (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **triggerless_text**: `エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **use_limit**: 1/turn
- **parsed condition type**: `none`
- **parsed action type**: `conditional_on_result`
- **cards**:
  - `PL!S-bp2-004-R | 黒澤ダイヤ (ab#0)`
  - `PL!S-bp2-004-P | 黒澤ダイヤ (ab#0)`

### 7. PL!SP-bp2-015-N | 平安名すみれ (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `location_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-bp2-015-N | 平安名すみれ (ab#0)`

### 8. PL!SP-bp2-020-N | 鬼塚夏美 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `location_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-bp2-020-N | 鬼塚夏美 (ab#0)`

### 9. PL!SP-bp2-021-N | ウィーン・マルガレーテ (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。`
- **triggerless_text**: `エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `location_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-bp2-021-N | ウィーン・マルガレーテ (ab#0)`

### 10. PL!S-bp3-020-L | ダイスキだったらダイジョウブ！ (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **triggerless_text**: `エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。そのエールで得たブレードハートを失い、もう一度エールを行う。`
- **use_limit**: 1/turn
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!S-bp3-020-L | ダイスキだったらダイジョウブ！ (ab#0)`

### 11. PL!S-sd1-001-SD | 高海千歌 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、ライブ終了時まで、エールにより公開された自分のカードの中のライブカード1枚につき、{{heart_02.png|heart02}}を得る。この能力では{{heart_02.png|heart02}}は3つまでしか得られない。`
- **triggerless_text**: `自分がエールしたとき、ライブ終了時まで、エールにより公開された自分のカードの中のライブカード1枚につき、{{heart_02.png|heart02}}を得る。この能力では{{heart_02.png|heart02}}は3つまでしか得られない。`
- **use_limit**: 1/turn
- **parsed condition type**: `none`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!S-sd1-001-SD | 高海千歌 (ab#0)`

### 12. PL!S-bp6-021-L | MIRAI TICKET (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いたカードのコスト５につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いたカードのコスト５につき、追加で1枚エールを行う。この能力では4枚までしか追加でエールできない。`
- **use_limit**: 1/turn
- **parsed condition type**: `custom`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!S-bp6-021-L | MIRAI TICKET (ab#0)`

### 13. PL!HS-bp6-027-L | 月夜見海月 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う。`
- **triggerless_text**: `自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う。`
- **use_limit**: 1/turn
- **parsed condition type**: `custom`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!HS-bp6-027-L | 月夜見海月 (ab#0)`

---

## on_area_move (12 abilities)

### 1. PL!SP-bp4-011-R＋ | 鬼塚冬毬 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のメンバー1人をウェイトにする。`
- **triggerless_text**: `このメンバーが登場か、エリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のメンバー1人をウェイトにする。`
- **parsed condition type**: `movement_condition`
- **parsed action type**: `change_state`
- **cards**:
  - `PL!SP-bp4-011-R＋ | 鬼塚冬毬 (ab#0)`
  - `PL!SP-bp4-011-P | 鬼塚冬毬 (ab#0)`
  - `PL!SP-bp4-011-P＋ | 鬼塚冬毬 (ab#0)`
  - `PL!SP-bp4-011-SEC | 鬼塚冬毬 (ab#0)`

### 2. PL!SP-sd2-002-P | 唐 可可 (ab#1) (shared by 3 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **use_limit**: 1/turn
- **opponent_effect**: also triggers on opponent's card effects
- **parsed condition type**: `movement_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-sd2-002-P | 唐 可可 (ab#1)`
  - `PL!SP-sd2-002-SD2 | 唐 可可 (ab#1)`
  - `PL!SP-sd2-013-SD2 | 唐 可可 (ab#0)`

### 3. PL!SP-pb1-006-R | 桜小路きな子 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **opponent_effect**: also triggers on opponent's card effects
- **parsed condition type**: `none`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-pb1-006-R | 桜小路きな子 (ab#0)`
  - `PL!SP-pb1-006-P＋ | 桜小路きな子 (ab#0)`

### 4. PL!SP-bp2-003-R | 嵐 千砂都 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **use_limit**: 1/turn
- **parsed condition type**: `movement_condition`
- **parsed action type**: `move_cards`
- **cards**:
  - `PL!SP-bp2-003-R | 嵐 千砂都 (ab#0)`
  - `PL!SP-bp2-003-P | 嵐 千砂都 (ab#0)`

### 5. PL!SP-bp4-007-R | 米女メイ (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の『Liella!』のライブカードを1枚手札に加える。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、自分の控え室から、スコア3以下の『Liella!』のライブカードを1枚手札に加える。`
- **use_limit**: 1/turn
- **parsed condition type**: `movement_condition`
- **parsed action type**: `move_cards`
- **cards**:
  - `PL!SP-bp4-007-R | 米女メイ (ab#0)`
  - `PL!SP-bp4-007-P | 米女メイ (ab#0)`

### 6. PL!S-bp5-111-R | 鹿角聖良 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがエリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が2つ以下のメンバー1人をウェイトにする。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が2つ以下のメンバー1人をウェイトにする。`
- **parsed condition type**: `movement_condition`
- **parsed action type**: `change_state`
- **cards**:
  - `PL!S-bp5-111-R | 鹿角聖良 (ab#1)`
  - `PL!S-bp5-111-P＋ | 鹿角聖良 (ab#1)`

### 7. PL!S-bp5-222-R | 鹿角理亞 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、エネルギーを2枚アクティブにする。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、エネルギーを2枚アクティブにする。`
- **use_limit**: 1/turn
- **parsed condition type**: `movement_condition`
- **parsed action type**: `change_state`
- **cards**:
  - `PL!S-bp5-222-R | 鹿角理亞 (ab#1)`
  - `PL!S-bp5-222-P＋ | 鹿角理亞 (ab#1)`

### 8. PL!SP-pb1-020-N | 鬼塚夏美 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
- **opponent_effect**: also triggers on opponent's card effects
- **parsed condition type**: `none`
- **parsed action type**: `draw_card`
- **cards**:
  - `PL!SP-pb1-020-N | 鬼塚夏美 (ab#0)`

### 9. PL!HS-bp5-014-N | 安養寺 姫芽 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `movement_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!HS-bp5-014-N | 安養寺 姫芽 (ab#0)`

### 10. PL!SP-sd2-011-SD2 | 鬼塚冬毬 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **use_limit**: 1/turn
- **opponent_effect**: also triggers on opponent's card effects
- **parsed condition type**: `movement_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-sd2-011-SD2 | 鬼塚冬毬 (ab#0)`

### 11. PL!SP-sd2-012-SD2 | 澁谷かのん (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **use_limit**: 1/turn
- **opponent_effect**: also triggers on opponent's card effects
- **parsed condition type**: `movement_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-sd2-012-SD2 | 澁谷かのん (ab#0)`

### 12. PL!SP-sd2-022-SD2 | 鬼塚冬毬 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **triggerless_text**: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。
(対戦相手のカードの効果でも発動する。)`
- **use_limit**: 1/turn
- **opponent_effect**: also triggers on opponent's card effects
- **parsed condition type**: `movement_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-sd2-022-SD2 | 鬼塚冬毬 (ab#0)`

---

## on_sent_to_discard_from_stage (10 abilities)

### 1. PL!N-bp5-005-R＋ | 宮下 愛 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、さらにカードを1枚引く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーとバトンタッチしていた場合、エネルギーを2枚アクティブにする。コスト15以上のブレードハートを持たない『虹ヶ咲』のメンバーの場合、さらにカードを1枚引く。`
- **parsed condition type**: `none`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!N-bp5-005-R＋ | 宮下 愛 (ab#0)`
  - `PL!N-bp5-005-P | 宮下 愛 (ab#0)`
  - `PL!N-bp5-005-AR | 宮下 愛 (ab#0)`
  - `PL!N-bp5-005-SEC | 宮下 愛 (ab#0)`

### 2. PL!HS-bp5-003-R＋ | 大沢瑠璃乃 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `position_change`
- **cards**:
  - `PL!HS-bp5-003-R＋ | 大沢瑠璃乃 (ab#0)`
  - `PL!HS-bp5-003-P | 大沢瑠璃乃 (ab#0)`
  - `PL!HS-bp5-003-AR | 大沢瑠璃乃 (ab#0)`
  - `PL!HS-bp5-003-SEC | 大沢瑠璃乃 (ab#0)`

### 3. PL!-PR-001-PR | 高坂穂乃果 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、メンバー1人をアクティブにしてもよい。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、メンバー1人をアクティブにしてもよい。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `change_state`
- **cards**:
  - `PL!-PR-001-PR | 高坂穂乃果 (ab#0)`
  - `PL!-PR-002-PR | 絢瀬絵里 (ab#0)`

### 4. PL!S-bp2-002-R | 桜内梨子 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『Aqours』のライブカードを1枚手札に加える。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室から『Aqours』のライブカードを1枚手札に加える。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!S-bp2-002-R | 桜内梨子 (ab#0)`
  - `PL!S-bp2-002-P | 桜内梨子 (ab#0)`

### 5. PL!HS-bp2-012-N | 乙宗 梢 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `look_and_select`
- **cards**:
  - `PL!HS-bp2-012-N | 乙宗 梢 (ab#0)`

### 6. PL!HS-bp2-013-N | 夕霧綴理 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `look_and_select`
- **cards**:
  - `PL!HS-bp2-013-N | 夕霧綴理 (ab#0)`

### 7. PL!HS-bp2-015-N | 藤島 慈 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を1枚控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を1枚控え室に置く。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!HS-bp2-015-N | 藤島 慈 (ab#0)`

### 8. PL!HS-bp6-017-N | 日野下花帆 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!HS-bp6-017-N | 日野下花帆 (ab#0)`

### 9. PL!HS-bp6-018-N | 村野さやか (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうした場合、ライブ終了時まで、自分のステージにいるメンバー1人は、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を得る。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!HS-bp6-018-N | 村野さやか (ab#0)`

### 10. PL!HS-bp6-019-N | 大沢瑠璃乃 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を2枚控え室に置く。`
- **triggerless_text**: `このメンバーがステージから控え室に置かれたとき、カードを2枚引き、手札を2枚控え室に置く。`
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!HS-bp6-019-N | 大沢瑠璃乃 (ab#0)`

---

## on_ally_appear_on_stage (4 abilities)

### 1. PL!N-bp3-005-R＋ | 宮下 愛 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く。`
- **triggerless_text**: `このターン、自分のステージにメンバーが3回登場したとき、手札が5枚になるまでカードを引く。`
- **parsed condition type**: `temporal_condition`
- **parsed action type**: `draw_until_count`
- **cards**:
  - `PL!N-bp3-005-R＋ | 宮下 愛 (ab#0)`
  - `PL!N-bp3-005-P | 宮下 愛 (ab#0)`
  - `PL!N-bp3-005-P＋ | 宮下 愛 (ab#0)`
  - `PL!N-bp3-005-SEC | 宮下 愛 (ab#0)`

### 2. PL!N-pb1-005-R | 宮下 愛 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージにコスト10のメンバーが登場したとき、カードを1枚引く。`
- **triggerless_text**: `自分のステージにコスト10のメンバーが登場したとき、カードを1枚引く。`
- **use_limit**: 1/turn
- **parsed condition type**: `appearance_condition`
- **parsed action type**: `draw_card`
- **cards**:
  - `PL!N-pb1-005-R | 宮下 愛 (ab#0)`
  - `PL!N-pb1-005-P＋ | 宮下 愛 (ab#0)`

### 3. PL!N-pb1-012-R | 鐘 嵐珠 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **triggerless_text**: `自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
- **use_limit**: 1/turn
- **parsed condition type**: `appearance_condition`
- **parsed action type**: `move_cards`
- **cards**:
  - `PL!N-pb1-012-R | 鐘 嵐珠 (ab#0)`
  - `PL!N-pb1-012-P＋ | 鐘 嵐珠 (ab#0)`

### 4. PL!HS-bp6-007-R | セラス 柳田 リリエンフェルト (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。`
- **triggerless_text**: `自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。`
- **use_limit**: 1/turn
- **parsed condition type**: `appearance_condition`
- **parsed action type**: `change_state`
- **cards**:
  - `PL!HS-bp6-007-R | セラス 柳田 リリエンフェルト (ab#0)`
  - `PL!HS-bp6-007-P | セラス 柳田 リリエンフェルト (ab#0)`

---

## on_state_changed_to_wait (2 abilities)

### 1. PL!-pb1-015-R | 西木野真姫 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く。`
- **triggerless_text**: `自分のカードの効果によって、相手のステージにいるアクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く。`
- **use_limit**: 1/turn
- **parsed condition type**: `state_change_condition`
- **parsed action type**: `draw_card`
- **cards**:
  - `PL!-pb1-015-R | 西木野真姫 (ab#1)`
  - `PL!-pb1-015-P＋ | 西木野真姫 (ab#1)`

### 2. PL!N-bp4-018-N | 近江彼方 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき、カードを1枚引き、手札を1枚控え室に置く。`
- **triggerless_text**: `自分のメインフェイズの間、このメンバーがアクティブ状態からウェイト状態になったとき、カードを1枚引き、手札を1枚控え室に置く。`
- **use_limit**: 1/turn
- **parsed condition type**: `state_change_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!N-bp4-018-N | 近江彼方 (ab#0)`

---

## on_ally_appear_each_time (2 abilities)

### 1. PL!HS-pb1-001-R | 日野下花帆 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn2.png|ターン2回}}自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクティブにする。`
- **triggerless_text**: `自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクティブにする。`
- **use_limit**: 2/turn
- **parsed condition type**: `none`
- **parsed action type**: `conditional_on_optional`
- **cards**:
  - `PL!HS-pb1-001-R | 日野下花帆 (ab#0)`
  - `PL!HS-pb1-001-P＋ | 日野下花帆 (ab#0)`

### 2. PL!HS-pb1-009-R | 日野下花帆 (ab#0) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{center.png|センター}}{{turn2.png|ターン2回}}自分のステージに『蓮ノ空』のメンバーが登場するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `{{center.png|センター}}自分のステージに『蓮ノ空』のメンバーが登場するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **use_limit**: 2/turn
- **position**: center required
- **parsed condition type**: `none`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!HS-pb1-009-R | 日野下花帆 (ab#0)`
  - `PL!HS-pb1-009-P＋ | 日野下花帆 (ab#0)`

---

## on_live_start_ability_resolved (2 abilities)

### 1. PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}自分のステージにいるメンバーの{{live_start.png|ライブ開始時}}能力が解決するたび、そのメンバーが{{icon_all.png|ハート}}を持たない場合、ライブ終了時まで、そのメンバーは{{icon_all.png|ハート}}を得る。`
- **triggerless_text**: `自分のステージにいるメンバーの{{live_start.png|ライブ開始時}}能力が解決するたび、そのメンバーが{{icon_all.png|ハート}}を持たない場合、ライブ終了時まで、そのメンバーは{{icon_all.png|ハート}}を得る。`
- **parsed condition type**: `location_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#0)`

### 2. PL!-bp6-020-L | Dancing stars on me! (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_start.png|ライブ開始時}}能力が解決したとき、そのメンバーをポジションチェンジする。`
- **triggerless_text**: `自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_start.png|ライブ開始時}}能力が解決したとき、そのメンバーをポジションチェンジする。`
- **use_limit**: 1/turn
- **parsed condition type**: `group_condition`
- **parsed action type**: `position_change`
- **cards**:
  - `PL!-bp6-020-L | Dancing stars on me! (ab#0)`

---

## on_live_success_ability_resolved (2 abilities)

### 1. PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#1) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}自分のステージにいるメンバーの{{live_success.png|ライブ成功時}}能力が解決するたび、カードを1枚引く。`
- **triggerless_text**: `自分のステージにいるメンバーの{{live_success.png|ライブ成功時}}能力が解決するたび、カードを1枚引く。`
- **parsed condition type**: `none`
- **parsed action type**: `draw_card`
- **cards**:
  - `PL!N-bp5-030-L | 繚乱！ビクトリーロード (ab#1)`

### 2. PL!-bp6-020-L | Dancing stars on me! (ab#1) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_success.png|ライブ成功時}}能力が解決したとき、そのメンバーがこのターン中に移動している場合、このカードのスコアを＋１する。`
- **triggerless_text**: `自分のステージのセンターエリアにいる『μ's』のメンバーの{{live_success.png|ライブ成功時}}能力が解決したとき、そのメンバーがこのターン中に移動している場合、このカードのスコアを＋１する。`
- **use_limit**: 1/turn
- **parsed condition type**: `temporal_condition`
- **parsed action type**: `modify_score`
- **cards**:
  - `PL!-bp6-020-L | Dancing stars on me! (ab#1)`

---

## on_move_or_energy_placed (1 abilities)

### 1. PL!SP-bp5-004-R＋ | 平安名すみれ (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき、カードを1枚引き、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **triggerless_text**: `自分のカードの効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき、カードを1枚引き、ライブ終了時まで、{{heart_02.png|heart02}}を得る。`
- **use_limit**: 1/turn
- **parsed condition type**: `movement_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!SP-bp5-004-R＋ | 平安名すみれ (ab#0)`
  - `PL!SP-bp5-004-P | 平安名すみれ (ab#0)`
  - `PL!SP-bp5-004-AR | 平安名すみれ (ab#0)`
  - `PL!SP-bp5-004-SEC | 平安名すみれ (ab#0)`

---

## on_any_to_discard_each_time (1 abilities)

### 1. PL!SP-bp5-005-R＋ | 葉月 恋 (ab#1) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、それらのカードの中から1枚手札に加える。`
- **triggerless_text**: `自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、それらのカードの中から1枚手札に加える。`
- **use_limit**: 1/turn
- **parsed condition type**: `none`
- **parsed action type**: `conditional_on_optional`
- **cards**:
  - `PL!SP-bp5-005-R＋ | 葉月 恋 (ab#1)`
  - `PL!SP-bp5-005-P | 葉月 恋 (ab#1)`
  - `PL!SP-bp5-005-AR | 葉月 恋 (ab#1)`
  - `PL!SP-bp5-005-SEC | 葉月 恋 (ab#1)`

---

## on_live_card_zone_to_discard (1 abilities)

### 1. PL!S-bp6-002-R＋ | 桜内梨子 (ab#0) (shared by 4 cards)

- **full_text**: `{{jidou.png|自動}}{{turn1.png|ターン1回}}『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。`
- **triggerless_text**: `『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。`
- **use_limit**: 1/turn
- **parsed condition type**: `card_count_condition`
- **parsed action type**: `move_cards`
- **cards**:
  - `PL!S-bp6-002-R＋ | 桜内梨子 (ab#0)`
  - `PL!S-bp6-002-P | 桜内梨子 (ab#0)`
  - `PL!S-bp6-002-P＋ | 桜内梨子 (ab#0)`
  - `PL!S-bp6-002-SEC | 桜内梨子 (ab#0)`

---

## on_hand_to_discard_each_time (1 abilities)

### 1. PL!HS-pb1-003-R | 大沢瑠璃乃 (ab#1) (shared by 2 cards)

- **full_text**: `{{jidou.png|自動}}{{turn2.png|ターン2回}}自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `自分の手札からカードが1枚以上控え室に置かれるたび、ライブ終了時まで、{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る。`
- **use_limit**: 2/turn
- **parsed condition type**: `none`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!HS-pb1-003-R | 大沢瑠璃乃 (ab#1)`
  - `PL!HS-pb1-003-P＋ | 大沢瑠璃乃 (ab#1)`

---

## on_baton_touch_appear (1 abilities)

### 1. PL!N-PR-025-PR | 優木せつ菜 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}{{turn2.png|ターン2回}}自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く。`
- **triggerless_text**: `自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき、カードを1枚引く。`
- **use_limit**: 2/turn
- **parsed condition type**: `movement_condition`
- **parsed action type**: `draw_card`
- **cards**:
  - `PL!N-PR-025-PR | 優木せつ菜 (ab#0)`

---

## on_discard_to_hand (1 abilities)

### 1. PL!N-bp4-026-L | DIVE! (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}自分のメインフェイズにこのカードが控え室から手札に加えられたとき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。そうした場合、次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る。`
- **triggerless_text**: `自分のメインフェイズにこのカードが控え室から手札に加えられたとき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きでライブカード置き場に置いてもよい。そうした場合、次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る。`
- **parsed condition type**: `location_condition`
- **parsed action type**: `sequential`
- **cards**:
  - `PL!N-bp4-026-L | DIVE! (ab#0)`

---

## on_placed_in_live_card_zone (1 abilities)

### 1. PL!N-bp4-026-L | DIVE! (ab#1) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このカードが表向きでライブカード置き場に置かれたとき、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **triggerless_text**: `このカードが表向きでライブカード置き場に置かれたとき、ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- **parsed condition type**: `location_condition`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!N-bp4-026-L | DIVE! (ab#1)`

---

## on_energy_placed_each_time (1 abilities)

### 1. PL!SP-bp4-016-N | 葉月 恋 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれるたび、ライブ終了時まで、{{heart_06.png|heart06}}を得る。(相手のカードの効果でも発動する。)`
- **triggerless_text**: `カードの効果によって自分のエネルギー置き場にエネルギーカードが置かれるたび、ライブ終了時まで、{{heart_06.png|heart06}}を得る。(相手のカードの効果でも発動する。)`
- **parsed condition type**: `none`
- **parsed action type**: `gain_resource`
- **cards**:
  - `PL!SP-bp4-016-N | 葉月 恋 (ab#0)`

---

## on_baton_touch_to_discard (1 abilities)

### 1. PL!HS-sd1-001-SD | 日野下花帆 (ab#0) (shared by 1 cards)

- **full_text**: `{{jidou.png|自動}}このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき、エネルギーを2枚アクティブにする。`
- **triggerless_text**: `このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして控え室に置かれたとき、エネルギーを2枚アクティブにする。`
- **parsed condition type**: `movement_condition`
- **parsed action type**: `change_state`
- **cards**:
  - `PL!HS-sd1-001-SD | 日野下花帆 (ab#0)`

---

