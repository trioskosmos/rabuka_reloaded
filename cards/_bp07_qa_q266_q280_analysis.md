# BP07 QA entries Q266–Q280 — analysis & test coverage

Source of truth: `cards/qa_data.json` (entries Q266–Q280, dated 2026.06.26 / 2026.08.05).
Card ability texts: `cards/cards.json` (`ability` field). Parser output / per-ability decoding:
`cards/abilities.json` + `engine/src/ability/abilities_gen.rs`.

This file records, for each QA entry, the underlying game rule, the card/ability involved,
and where (if anywhere) it is already exercised by a gameplay test. Entries marked **gap**
have no test that drives the real engine rule end-to-end.

Legend: `✓ covered` = a gameplay test in `engine/tests/test_modules/` already drives the
specific rule described. `partial` = some adjacent mechanic is tested but the QA edge is not.
`gap` = no test touches it.

---

## Q266 — 鬼塚夏美 PL!SP-pb2-009 ab#0 (登場/ライブ開始時)
> 登場/ライブ開始時：『Liella!』のメンバー1人をウェイトにしてもよい：相手のステージにいる元々持つブレードの数がこれによりウェイトにしたメンバーが元々持つブレードの数より2つ以上少ないメンバー1人をウェイトにする。
- **QA rule**: paying the wait cost with a 0-blade member means the opponent target must have
  `0 - 2` = impossible; so a 0-blade opponent member cannot be waited. Blade comparison is on
  **original blade count** (元々持つ), and waited members' blades don't count toward yell reveal.
- **Status**: ✓ covered — `bp7_q266_natsumi_blade_wait_test.rs` (5 tests) drives the real
  blade-0 wait cost boundary (costed B0→nothing waitable, B2→blade-0 only, B4→blade-2/3,
  skip-cost→nothing).

## Q267 — 天王寺璃奈 PL!N-bp7-009-R ab#0 (登場)
> 登場：自分と相手はそれぞれ、自身のデッキの上からカードを7枚控え室に置く。
- **QA rule**: if the deck runs exactly to 0 during the mill, a refresh happens immediately
  (before the auto-ability/effect continues), so cards leave the waitroom.
- **Status**: ✓ covered — `bp7_q267_rinna_mill_refresh_test.rs` (4 tests) drives the real
  mill through the card's 登場 (both players each mill 7): deck-0 mid-mill refresh completes
  to 7 total, deck=exactly-7 no-refresh boundary, deck=5 refresh→still 7 total, and both
  players refreshing their own deck.

## Q268 — 三船栞子 PL!N-bp7-010-R ab#0 (起動)
> 起動：エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：自分の控え室からコスト2以下の『虹ヶ咲』のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる。
- **QA rule**: the cost (place 1 energy under self) can be paid **even when there is no empty
  area** to put the new member — cost is payable independent of whether the effect fully resolves.
- **Status**: ✓ covered — `bp7_q268_shioriko_empty_area_deploy_test.rs` (8 tests) drives the
  real 起動: cost is payable with NO empty area (Q268 core), deploy-to-empty-area-in-wait,
  cost>2 / non-虹ヶ咲 target exclusion, exactly-one-of-two deployed, ターン1回 limit, and
  no-target+no-empty-area cost still paid.

## Q269 / Q277 — ミア・テイラー PL!N-bp7-011-R＋ ab#0 (自動) + ab#1 (常時)
> 自動：このカードがデッキから控え室に置かれたとき、手札を1枚控え室に置いてもよい。そうしたとき、控え室からこのカードを手札に加える。
> 常時：このカードをプレイする際、自分の控え室にあるすべてのメンバーカードをシャッフルし、デッキの下に置いてもよい。そうしたとき、このカードのコストは２減る。
- **Q269 rule**: being revealed by エール (yell) is NOT a deck→waitroom move, so the 自動 does
  NOT trigger.
- **Q277 rule**: when a mill empties the deck to exactly 0, refresh occurs **before** the 自動
  ability resolves; the card leaves the waitroom and can no longer be added to hand.
- **Status**: ✓ covered — `bp7_mia_optional_recover_test.rs` covers the optional-discard
  recovery path; `bp7_q269_mia_yell_no_trigger_test.rs` covers the Q269 yell-reveal
  no-trigger edge (3 tests + deck→discard control) and the Q277 refresh-before-resolve
  edge (2 tests + no-refresh control).
- **ab#1 (常時)**: ✓ implemented + covered — the play-time "プレイする際…コストは２減る"
  ability is now wired into `handle_play_member_to_stage` (two-phase hook + resume), offering
  the optional choice on play; accepting shuffles waitroom members to deck bottom and reduces
  the cost by 2. Covered by `bp7_mia_play_cost_reduction_test.rs` (accept/decline/no-waitroom).

## Q270 — エマ・ヴェルデ PL!N-bp7-020-N ab#0 (登場)
> 登場：自分のデッキの上からカードを3枚控え室に置く。それらのメンバーカードの中に2種類以上のブレードハートの色がある場合、ライブ終了時まで、heart04を得る。
- **QA rule**: "2 types of blade-heart color" — an ALL-heart card is not a blade-heart color;
  a green blade-heart is one type. So {ALL-heart, green-blade-heart} = only 1 type → no heart04.
  "2種類以上" = 2 or more distinct **blade-heart** colors.
- **Status**: ✓ covered — `bp7_emma_color_diversity_test.rs` covers the 2-distinct-color rule
  (5 tests: 2/3 distinct → heart04, 1 color → none, single member → none, no blade heart →
  none). The ALL-heart-vs-blade-heart counting edge: no member card in the DB has a
  colorless/ALL blade heart, so the "not counted as a blade-heart color" case is subsumed by
  the no-blade-heart test (non-color blade hearts never contribute a distinct color).

## Q271 — Colorful Dreams! Colorful Smiles! PL!N-bp7-025-L ab#1 (ライブ成功時)
> ライブ成功時：エールにより公開された自分のカードの中に heart01〜heart06 のうち3種類以上ある場合、このカードのスコアを＋１する。
- **QA rule**: cards with only 桃/青/ALL **blade-heart** do NOT satisfy a "3 types of heart
  color" condition — blade-heart is not a heart color.
- **Status**: ✓ covered — `bp7_q271_colorful_dreams_test.rs` (9 tests) drives ab#1 (Q271: peach/
  blue/ALL blade-hearts → no score; 3 blade-heart colors mapping to heart colors → no score;
  3 distinct base hearts → +1; single multi-heart card → +1; 2 base hearts → no score; blade
  doesn't add a color; empty reveal → no score) and ab#0 (ライブ開始時 虹ヶ咲 member gains 1 blade;
  non-虹ヶ咲 gains none).
- **Engine fix (Q271)**: `resolve_zone_card_count`'s RevealedCards "types" branch now counts only
  base-heart colors when `heart_source` is not `blade` (previously it also added blade-heart
  colors, so a blade-heart set could wrongly satisfy the condition).

## Q272 — Just Believe!!! PL!N-bp7-026-L ab#0 (ライブ開始時)
> ライブ開始時：手札を2枚まで控え室に置いてもよい：自分のステージにいる『虹ヶ咲』のメンバーを、これにより控え室に置いたカードの枚数に等しい数まで選ぶ。ライブ終了時まで、それらはブレードを得る。
- **QA rule**: the same member cannot be selected multiple times (no-repeat selection).
- **Status**: ✓ covered — `bp7_q272_just_believe_test.rs` (10 tests) drives ab#0 (Q272: discard-1 → 1 blade;
  discard-2 → 2 distinct members each get 1; **1 member + discard 2 → exactly 1 blade (cannot select twice)**;
  non-虹ヶ咲 not selectable; skip discard → no blade) and ab#1 (ライブ成功時: 2+ member cards without
  blade-hearts among revealed → +1 score; 1 only / with-blade / mixed / live card → no score).

## Q273 — 渡辺 曜 PL!S-bp7-005-R＋ ab#2 (起動 センター)
> 起動：手札を2枚控え室に置く：このメンバーと自分のステージにいるほかの『Aqours』のメンバー1人を選ぶ。それらが持つ登場能力それぞれ1つを発動させる。
- **QA rule**: an 登場 ability activated this way still pays its own cost if it has one.
- **Status**: ✓ covered — `bp7_q273_watanabe_cost_test.rs` (2 tests) drives ab#2 firing both
  selected members' 登場 abilities and verifies the fired 登場 ability's cost is offered and
  paid (Q273), plus 渡辺's own 登場 fires (member placed under her).
- **Engine fix (Q273)**: `execute_activate_ability` previously fired only the LAST selected card's
  ability and executed its effect directly (skipping the cost). It now (a) fires EVERY selected
  member's 登場 ability (「それらが…それぞれ」) and (b) enqueues each fired ability through the
  normal queue so its own cost is paid before the effect resolves; the trigger is inferred as 登場
  when the parser leaves `target_trigger` null.

## Q274 / Q275 — 松浦果南 PL!S-bp7-003-R＋ ab#1 option 1 (登場)
> 登場：以下から1つを選ぶ。・ライブ終了時まで、自分のステージにいる元々持つブレードの数が3つ以下の『Aqours』のメンバーは、相手の効果によってはウェイトしない。
- **Q274 rule**: an opponent MAY still *select* your wait-immune member as the target of their
  wait effect (selection is allowed; the wait simply doesn't apply). Cannot-immune ≠ cannot-select.
- **Q275 rule**: when an effect forces *you* to wait a member (e.g. セラス柳田リリエンフェルト
  PL!HS-bp6-007-R), a wait-immune member is NOT a legal choice — you must pick a waitable member.
- **Status**: `bp7_kanan_wait_immunity_test.rs` + `bp7_wait_immunity_helpers.rs` (G4) cover the
  immunity blocking an opponent's wait. The Q274 (still selectable) and Q275 (not legal choice
  for your own forced wait) selection edges: **gap**.

## Q276 — Cheer Mode PL!N-bp7-030-L ab#1 (ライブ成功時)
> ライブ成功時：このカードをライブカード置き場から手札に戻す。その後、手札を1枚控え室に置く。
- **QA rule**: winning a live with only this card still forces it back to hand (ab#1 is
  mandatory), so it cannot be left in the success live-card zone.
- **Status**: `blade_heart_colorless_test.rs` references `PL!N-bp7-030` (colorless/blade-heart
  scope). The success-zone-vs-return-to-hand edge: **partial**.

## Q278 / Q279 — 桜坂しずく PL!N-bp7-003-R＋ ab#1 (ライブ開始時)
> ライブ開始時：ライブ終了時まで、このメンバーの下に置かれている名前の異なるメンバーカード1枚につき、ブレードを得る。
- **Q278 rule**: under cards = 上原歩夢 `PL!N-bp1-001-R` + 上原歩夢&澁谷かのん&日野下花帆
  `LL-bp1-001-R＋` → **2 blades**.
- **Q279 rule**: under cards = 上原歩夢 + 澁谷かのん + 日野下花帆 + the `LL-bp1-001-R＋` joint
  card → **3 blades**. The joint card does NOT add a 4th distinct name slot when its constituent
  names are already present.
- **Status**: `bp7_under_member_per_unit_blade_test.rs` (B1) covers per-distinct-name blade under
  member with single-name cards (dedup by card_name via
  `apply_distinct_filter` → `DistinctType::CardName`, `engine/src/ability/util.rs:2237`).
  The **joint-card (multi-name) counting** edge (Q278/279) is **gap** and likely needs engine
  work: `apply_distinct_filter` dedups by full card name string, so the joint card would count
  as a distinct name in Q279 (giving 4), not 3.

## Q280 — 米女メイ PL!SP-bp7-007-R＋ ab#1 (ライブ成功時)
> ライブ成功時：自分のエネルギーデッキから、エネルギーカードを2枚ウェイト状態で置く。それらのエネルギーカードは、次のターンのアクティブフェイズにアクティブしない。
- **QA rule**: an energy whose "do-not-activate" (アクティブしない) is in force stays non-activating
  next turn even if it was moved / a live-success ability tried to activate it; the
  do-not-activate effect persists until its end condition.
- **Status**: **gap**. No test references `PL!SP-bp7-007`.

---

## Also: PL!N-sd2 rows already marked in `_bp07_ability_gaps_hand_analysis.md`

The verdict table rows 54–63 (`PL!N-sd2-001/006/010/013/015/017/019/021` + `PL!N-sd2-026`) are
marked ✅ there, but that marks **parser faithfulness only** (the parser produced correct JSON),
NOT gameplay-test coverage. Coverage status of those sd2 cards in
`engine/tests/test_modules/`:

| Card | Ability | md verdict | Test file reference |
|------|---------|-----------|---------------------|
| PL!N-sd2-026-P Fire Bird ab#0 | blade≥4 → heart02×2 | ✅ (CLEAN-G14) | `bp7_fire_bird_blade_gain_test.rs` |
| PL!N-sd2-001-SD2 上原歩夢 ab#0 | E2 → 虹ヶ咲 live to hand | ✅ (E2) | referenced in `bp7_audrey_blade_max_test.rs` |
| PL!N-sd2-006-SD2 近江彼方 ab#0 | wait 虹ヶ咲 → blade2 | ✅ | **no test** (gap) |
| PL!N-sd2-010-SD2 三船栞子 ab#0/1 | draw2 / wait→discard→active+blade2 | ✅ | **no test** (gap) |
| PL!N-sd2-013-SD2 上原歩夢 ab#0 | 虹ヶ咲 only → opp blade≤2 wait | ✅ | **no test** (gap) |
| PL!N-sd2-015-SD2 桜坂しずく ab#0 | wait + discard → draw | ✅ | **no test** (gap) |
| PL!N-sd2-017-SD2 宮下愛 ab#0 | E optional → active 1 | ✅ | **no test** (gap) |
| PL!N-sd2-019-SD2 優木せつ菜 ab#0/1 | heart05 / opp cost≤2 wait | ✅ | **no test** (gap) |
| PL!N-sd2-021-SD2 天王寺璃奈 ab#0 | opp cost≤4 wait | ✅ | **no test** (gap) |

So the ✅ verdicts in `_bp07_ability_gaps_hand_analysis.md` are NOT test coverage — those sd2
cards (except Fire Bird) have no gameplay test.

---

## Summary of gaps to close (Q266–Q280)

| QA | Card | Rule | Status |
|----|------|------|--------|
| Q266 | PL!SP-pb2-009 鬼塚夏美 | blade-0 wait cost ⇒ cannot wait 0-blade opp | ✓ `bp7_q266_natsumi_blade_wait_test` |
| Q267 | PL!N-bp7-009 天王寺璃奈 | deck-to-0 refresh mid-mill | ✓ `bp7_q267_rinna_mill_refresh_test` |
| Q268 | PL!N-bp7-010 三船栞子 | cost payable with no empty area | ✓ `bp7_q268_shioriko_empty_area_deploy_test` |
| Q269 | PL!N-bp7-011 ミア | yell reveal does not trigger 自動 | ✓ `bp7_q269_mia_yell_no_trigger_test` |
| Q270 | PL!N-bp7-020 エマ | ALL-heart not a blade-heart color | ✓ `bp7_emma_color_diversity_test` |
| Q277 | PL!N-bp7-011 ミア | refresh before 自動 resolve | ✓ `bp7_q269_mia_yell_no_trigger_test` |
| Q271 | PL!N-bp7-025 Colorful Dreams | blade-heart ≠ heart color (score) | ✓ `bp7_q271_colorful_dreams_test` |
| Q272 | PL!N-bp7-026 Just Believe | no-repeat select | ✓ `bp7_q272_just_believe_test` |
| Q273 | PL!S-bp7-005 渡辺曜 | activated 登場 ability pays cost | ✓ `bp7_q273_watanabe_cost_test` |
| Q274 | PL!S-bp7-003 松浦果南 | wait-immune member still selectable | **gap** |
| Q275 | PL!S-bp7-003 松浦果南 | not a legal forced-wait choice | **gap** |
| Q276 | PL!N-bp7-030 Cheer Mode | return-to-hand beats success zone | partial |
| Q278 | PL!N-bp7-003 桜坂しずく | joint card = 2 blades | **gap (engine)** |
| Q279 | PL!N-bp7-003 桜坂しずく | joint card ≠ extra distinct name | **gap (engine)** |
| Q280 | PL!SP-bp7-007 米女メイ | do-not-activate persists | **gap** |

Plus the 8 PL!N-sd2 cards (rows 55–63) marked ✅-parser-only that have **no gameplay test**.
