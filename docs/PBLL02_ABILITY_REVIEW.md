# PBLL02 ability review — what landed, what is missing, what to build

Companion to [ADDING_AN_EXPANSION.md](ADDING_AN_EXPANSION.md).

Source: **PBLL02 プレミアムブースター ラブライブ！DUO** (113 cards, release
2026.10.03), imported 2026-09-29.

```
cards      2526 -> 2639
abilities   936 -> 985   (+48 new unique abilities, all from this pack)
pairs        -> 2116
bytecode   132622 -> 98045 bytes
tests      3727 passed, 2 failed
```

Everything below is derived from the actual data, not from reading the cards:

- **new ability** = present in `cards/abilities.json` after the import, absent at
  `git show HEAD:cards/abilities.json`, keyed on `(triggerless_text, triggers)`
- **closest existing ability** = argmax `difflib.SequenceMatcher` ratio over the 936
  pre-existing abilities on `triggerless_text` (`Sim` column)
- **its tests** = `#[test]` fns in `engine/tests/**/*.rs` whose body mentions that
  card number, ranked by whether the file/fn name also encodes it
- **actions** = every `action` string in the parsed `effect`/`cost` tree

Re-run: `python cards/ability_extraction/extract_card_abilities.py` then compare.

---

## 1. Headline

| | Count |
|---|---|
| New abilities | 48 |
| New abilities with **no test of their own** | **48** |
| New abilities whose closest analogue has tests | 43 |
| New abilities with **no analogue test to copy** | **5** |
| Abilities parsed to `action: "custom"` (**fails `test_no_custom_actions`**) | **2** |
| Missing-mechanics mismatches from the extractor | **4** |
| Validation issues (missing `group_names`) | **1** |
| Parser warnings (`modify_score` missing `value`) | **1** |

Two abilities in this pack are mechanics the engine has **never implemented**. They
are the whole of the red test suite. Everything else parses into action types that
already have handlers — but "has a handler" is not "behaves correctly", and none of
the 48 has a behavioural test yet.

---

## 2. Blocking: the 2 `custom` actions

`test_no_custom_actions` fails on `unique_abilities[311]` and `[659]`. These are the
two mechanics that must be built before the suite is green.

### 2.1 `PL!-pb2-002-PP` / `-R` — 絢瀬絵里 (常時) — **replacement appearance rule**

> {{jyouji.png|常時}}このメンバーの正面のエリアには、元々持つハートの数が4つ以下のメンバーは、ウェイト状態で登場する。

*"In the area in front of this member, members with 4 or fewer printed hearts appear
in wait state."*

Currently emitted:

```json
{ "action": "custom", "count": 4, "card_type": "member_card",
  "position": "front", "original_value": true }
```

This is a **static replacement effect**: a permanent rule about how *other* cards
enter the stage, not something that fires on an event.

- **Parser** — needs a rule producing a typed replacement-effect node
  (filter: `card_type=member_card`, `position=front`, `original_value<=4`) rather
  than `action: custom`. Note the engine already keys replacement effects off
  `destination=success_live_zone` for the live-zone family, so there is partial
  precedent to follow or generalise.
- **Engine** — needs the rule evaluated at the moment a member would appear in the
  front area: intercept the placement, add wait state. Touches the appearance path
  in `core/game_state` and likely the same seam as the existing
  `live_phase_success_zone_replacement_*` tests.
- **Tests** — positive (4 hearts → wait, 3 hearts → wait) and the boundary (5 hearts
  → active), plus "does not apply to a member appearing in another area" and
  "does not stack with a second 絢瀬絵里".

### 2.2 `PL!-pb2-041-L` — 春情ロマンティック (常時) — **count-as-2**

> {{jyouji.png|常時}}このカードが自分の成功ライブカード置き場にあるかぎり、自分の『lily white』のカードの効果によって、自分の成功ライブカード置き場にあるカードの枚数を数えるとき、このカードは2枚として扱う。

*"While this card is in your success live card zone, when a `lily white` card's
effect counts cards in that zone, count this card as 2."*

Currently emitted:

```json
{ "action": "custom", "duration": "as_long_as", "conditional": true,
  "condition": { "type": "location_condition", "location": "success_live_card_zone",
                 "check_self": true, "self_target": true },
  "count": 2, "group_names": ["lily white"], "source": "success_live_zone" }
```

This is a **counting-weight modifier**: it changes how *another card's* count is
computed, scoped to a group, with a location + as-long-as condition.

- **Parser** — a distinct rule shape (`枚数を数えるとき、このカードはN枚として扱う`).
  Related known limitation to keep in mind: `per_character` on `LL-bp7-001` already
  selects by name but **cannot enforce exactly-one-per-name**; this mechanic is the
  general form of that problem and the fix may be shared.
- **Engine** — the count path in `ability/condition/card.rs`
  (`get_count_for_condition` / `get_group_card_count`) is where the weight must
  apply. Those three dispatchers are already known to overlap
  (`cards/ability_extraction/PARSER_NOTES.md`, "Still open"), so a fix here needs
  characterisation pins per condition shape or it will change which dispatcher wins.
- **Tests** — a `lily white` counting card sees 2; a non-`lily white` counting card
  sees 1; while out of the success zone it counts 1; two copies of 春情 do **not**
  compound.

---

## 3. Non-blocking but real: the 4 missing-mechanics mismatches

Reported by `extract_card_abilities.py`. These parse to a *plausible but wrong*
shape — no test fails, the behaviour is just not the printed behaviour. Per
`AGENTS.md` these are fixed end-to-end, not documented around.

| Card | Rule | Printed text | What is missing |
|---|---|---|---|
| `PL!-pb2-001-PP` | `all_blade` | {{icon_b_all.png\|ALLブレード}}を持つカードがある場合、ライブ終了時まで、{{icon_all.png\|ハート}}を得る | emits `gain_resource` + `heart_type: "all"` but no `all_blade_timing`, so the condition is keyed on the wrong blade kind |
| `PL!-pb2-018-P＋` | `distinct_name` | 手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いてもよい | `distinct: "card_name"` is not carried into the nested cost/effect, so duplicates are not excluded |
| `PL!-pb2-038-L` | `or_location` | このカードが自分のライブカード置き場**か**成功ライブカード置き場にあり… | only one of the two locations survives |
| `PL!-pb2-041-L` | `card_identity` | このカードは2枚として扱う | see §2.2 — same root cause |

Plus two validation/warning items:

| Card | Issue |
|---|---|
| `PL!-pb2-042-L` | GROUP FILTER: `『西木野真姫』` emitted in the text but **no `group_names`** entry — the 自動 ability filters on three named members and only part of the filter is structured |
| `PL!-pb2-012-P＋` / `-R` | `WARNING: Action 'modify_score' missing required field 'value'` on the 起動 ability — see §4.1 |

---

## 4. Per-ability detail on the non-`custom` outliers

### 4.1 `PL!-pb2-012-P＋` / `-R` — 南ことり (起動) — the `modify_score` warning

> {{kidou.png|起動}}{{turn1.png|ターン1回}}このメンバーをウェイトにする：この能力を起動するための追加コストとして、手札を2枚控え室に置く**か**、『Printemps』のメンバーを2人ウェイトにする。自分の控え室から『Printemps』のライブカードを1枚手札に加える。

The optional-cost branch ("discard 2 **or** wait 2 members") is being mis-typed as
`modify_score`, which has no `value`. The constant ability on the same card parses
correctly (`modify_cost`, `value: 2`, `count: 2`, `distinct: "card_name"`), so this
is an optional-cost-branch pattern the parser is missing, not a new mechanic.

- **Parser** — route the "N枚…**か**…N人をウェイト" optional-cost branch to the
  choice/sequential cost shape already used by similar 起動 costs.
- **Engine** — likely fine once typed correctly.
- **Tests** — pay by discard; pay by waiting 2 Printemps; decline; both branches
  produce the same live-card recovery.

### 4.2 `PL!-pb2-042-L` — PSYCHIC FIRE (自動)

> {{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のカードの中に「矢澤にこ」と「西木野真姫」と「絢瀬絵里」のメンバーカードがあり、かつ自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合、相手のステージにいる元々持つハートの数が4つ以下のメンバー1人をウェイトにする。

Four stacked conditions: named-member filter (3 names, **all** required), a
zone+cost filter, an original-blade count on the *opponent*, and a single-target
selection. `cost_and_blade_filtered_opponent_wait_test.rs` is the shape to copy.

### 4.3 `PL!-pb2-038-L` — 嵐のなかの恋だから (常時)

> このカードが自分のライブカード置き場か成功ライブカード置き場にあり、かつ自分のステージに2人の『μ's』のメンバーのみがいる場合、ライブの合計スコアを＋１する。**この効果は重複しない。**

Non-stackable + compound condition. Nearest existing coverage is
`success_zone_group_gated_constant_score_test.rs` (Angelic Angel), which already
covers "does not bleed to live set zone" — that test's `live_set_zone` case is
literally the first half of this card's condition.

### 4.4 `PL!-pb2-001-PP` — 高坂穂乃果 (ライブ開始時)

Three independent branches in one `sequential`, each gated on a different card
feature in the success live zone (`[スコア]` → score+1, `[ALLブレード]` → heart,
`[ドロー]` → recover a μ's card). The `sequential` wrapper and the three
`gain_ability` / `gain_resource` / `move_cards` arms are all typed; only the
`all_blade` filter is wrong (§3).

---

## 5. All 48 new abilities

`Sim` = text similarity to the closest pre-existing ability. Low `Sim` is expected —
these are new cards, not retexts. `**none**` in the tests column means there is no
existing test to model on; those are the highest-risk rows.

| # | Card | Name | Trig | Actions | Closest existing ability | Sim | That ability's tests |
|---|------|------|------|---------|------------------------|-----|----------------------|
| 1 | `PL!-pb2-001-PP` | 高坂穂乃果 | ライブ開始時 | `sequential`, `gain_ability`, `modify_score`, `gain_resource`, `move_cards` | `PL!S-bp6-002-R＋` 桜内梨子 | 0.48 | **none** |
| 2 | `PL!-pb2-002-PP` | 絢瀬絵里 | 常時 | **`custom`** | `PL!S-bp5-111-R` 鹿角聖良 | 0.52 | `self_control_position_change_test.rs`::seira_area_move_wait_blocked_by_immunity<br>`self_control_position_change_test.rs`::position_change_triggered_grant_blade_to_moved_members |
| 3 | `PL!-pb2-003-PP` | 南ことり | ライブ開始時 | `conditional_on_result`, `invalidate_ability`, `gain_resource` | `PL!SP-bp2-001-R＋` 澁谷かのん | 0.61 | `zero_tested_action_types_test.rs`::kanon_invalidate_liella_live_start |
| 4 | `PL!-pb2-004-PP` | 園田海未 | 常時 | `gain_resource` | `PL!-sd1-001-SD` 高坂 穂乃果 | 0.77 | `success_score_gated_look_fetches_musume_test.rs`::pl_bp4_006_r_score_three_or_more_fetches_mus_member |
| 5 | `PL!-pb2-004-PP` | 園田海未 | 自動 | `perform_yell` | `PL!SP-PR-024-PR` 平安名すみれ | 0.66 | **needs own test** (extra yell per revealed card — no existing file) |
| 6 | `PL!-pb2-005-PP` | 星空 凛 | 登場 | `gain_ability` | `PL!-bp6-001-R＋` 高坂穂乃果 | 0.69 | `live_start_stage_cost_ten_places_waitroom_lives_on_deck_top_test.rs`::can_select_2_of_4_available |
| 7 | `PL!-pb2-006-PP` | 西木野真姫 | 起動 | `change_state` | `PL!-bp5-333-R` 統堂英玲奈 | 0.74 | `arise_gated_activate_blade_or_wait_private_wars_test.rs`::setup |
| 8 | `PL!-pb2-006-PP` | 西木野真姫 | ライブ開始時 | `change_state` | `PL!-bp5-333-R` 統堂英玲奈 | 0.78 | `waited_self_heart05_constant_test.rs`::reactivating_waited_self_removes_constant_heart05 |
| 9 | `PL!-pb2-007-PP` | 東條 希 | 起動 | `sequential`, `move_cards`, `change_state` | `PL!-PR-017-PR` 矢澤にこ | 0.85 | `mus_live_recovery_success_score_energy_pl_pr_017_pr_test.rs`::pl_pr_017_pr_activation_recovers_mus_live_and_activates_two_energy_at_nine_score |
| 10 | `PL!-pb2-008-PP` | 小泉花陽 | 登場 | `look_and_select`, `look_at`, `select_cards` | `PL!SP-bp4-002-R` 唐 可可 | 0.95 | `debut_optional_wait_look_two_reorder_test.rs`::card_count_integrity |
| 11 | `PL!-pb2-009-PP` | 矢澤にこ | 自動 | `change_state` | `PL!N-bp5-005-R＋` 宮下 愛 | 0.70 | `live_success_compare_revealed_draw_behaviour_test.rs`::you_does_not_draw_when_own_reveal_count_is_higher |
| 12 | `PL!-pb2-010-P＋` | 高坂穂乃果 | ライブ開始時 | `gain_resource` | `PL!S-bp5-022-L` SELF CONTROL!! | 0.70 | `self_control_position_change_test.rs`::position_change_triggered_grant_blade_to_moved_members |
| 13 | `PL!-pb2-011-P＋` | 絢瀬絵里 | 常時 | `gain_resource` | `PL!SP-bp7-003-R＋` 嵐 千砂都 | 0.94 | `bp7_under_member_per_unit_blade_test.rs`::chika_constant_blade_per_member_under |
| 14 | `PL!-pb2-011-P＋` | 絢瀬絵里 | 自動 | `move_cards` | `PL!N-bp4-004-R＋` 朝香果林 | 0.55 | `waited_opponent_nijigasaki_topdeck_pl_n_bp4_004_r_plus_test.rs`::pl_n_bp4_004_r_plus_live_start_waited_opponents_recover_nijigasaki_member_to_deck_top |
| 15 | `PL!-pb2-012-P＋` | 南ことり | 常時 | `modify_cost` | `PL!N-bp7-011-R＋` ミア・テイラー | 0.66 | `success_score_sum_constant_heart_test.rs`::success_score_sum_hearts_require_stage_presence_pl_bp5_008_r |
| 16 | `PL!-pb2-012-P＋` | 南ことり | 起動 | `sequential`, `sequential`, **`modify_score` (no `value`)**, `move_cards`, `change_state` | `PL!N-bp3-004-R` 朝香果林 | 0.61 | `paid_discard_filtered_look_reveal_to_hand_test.rs`::pl_n_bp3_012_r_paid_discard_looks_four_reveals_nijigasaki_card_to_hand |
| 17 | `PL!-pb2-013-P＋` | 園田海未 | 登場 | `sequential`, `reveal`, `move_cards`, `move_cards` | `PL!-sd1-004-SD` 園田海未 | 0.61 | `self_wait_draw_discard_pl_n_bp7_023_n_test.rs`::pl_n_bp7_023_n_activation_waits_self_draws_two_discards_two |
| 18 | `PL!-pb2-014-P＋` | 星空 凛 | 登場 | `move_cards` | `PL!-sd1-006-SD` 西木野 真姫 | 0.91 | `live_phase_success_zone_replacement_q256_test.rs`::debut_reveal_success_zone_replacement_preserves_target_identity_pl_bp6_022_l |
| 19 | `PL!-pb2-015-P＋` | 西木野真姫 | 自動 | `choice`, `change_state`, `change_state` | `PL!-pb1-003-R` 南ことり | 0.52 | `self_wait_printemps_energy_pl_pb1_003_r_test.rs`::pl_pb1_003_r_declined_self_wait_preserves_active_energy_and_ends_choices |
| 20 | `PL!-pb2-016-P＋` | 東條 希 | ライブ開始時 | `draw_card` | `PL!-bp5-024-L` Private Wars | 0.50 | **none** |
| 21 | `PL!-pb2-017-P＋` | 小泉花陽 | 登場 | `move_cards` | `PL!N-PR-026-PR` 天王寺璃奈 | 0.70 | `rina_gain_live_success_from_under_member_test.rs`::rina_gains_ability_from_under_member |
| 22 | `PL!-pb2-017-P＋` | 小泉花陽 | ライブ開始時 | `sequential`, `place_energy_under_member`, `change_state` | `PL!-pb1-003-R` 南ことり | 0.61 | `self_wait_printemps_energy_pl_pb1_003_r_test.rs`::pl_pb1_003_r_paid_self_wait_activates_energy_per_printemps_member |
| 23 | `PL!-pb2-018-P＋` | 矢澤にこ | 登場 | `draw_card` | `PL!-pb1-008-R` 小泉花陽 | 0.65 | `optional_wait_members_draw_per_cost_member_q183_test.rs`::optional_wait_cost_draw_q183_pay_waits_self_members_draws_one_each |
| 24 | `PL!-pb2-018-P＋` | 矢澤にこ | ライブ開始時, 登場 | `change_state` | `PL!SP-pb2-047-L` Welcome to 僕らのセカイ | 0.77 | `all_liella_gated_opponent_wait_test.rs`::welcome_to_bokura_no_sekai_sp_pb2_047_live_start_with_non_liella_and_discard_does_not_wait_enemy |
| 25 | `PL!-pb1-029-SRL` | 知らないLove＊教えてLove | ライブ開始時 | `modify_score` | `PL!-pb1-029-L` 知らないLove＊教えてLove | 0.99 | `copied_ability_registration_lifecycle_test.rs`::real_phase_walk_to_live_success_registers_and_fires_the_copy_once |
| 26 | `PL!-pb2-000-DUO` | 星空 凛&小泉花陽 | 登場 | `gain_ability`, `modify_score` | `PL!-sd1-009-SD` 矢澤 にこ | 0.65 | `zero_tested_action_types_test.rs`::vitamin_summer_live_success_hand_condition |
| 27 | `PL!-pb2-000-R` | 星空 凛&小泉花陽 | 登場 | `gain_ability`, `modify_score` | `PL!-sd1-009-SD` 矢澤 にこ | 0.65 | `zero_tested_action_types_test.rs`::vitamin_summer_live_success_hand_condition |
| 28 | `PL!-pb2-019-N` | 高坂穂乃果 | ライブ開始時 | `gain_resource` | `PL!-bp4-011-N` 絢瀬絵里 | 0.74 | `live_start_paid_self_wait_center_member_blades_test.rs`::pl_bp4_011_n_paid_self_wait_grants_two_blades_only_to_center_mus_member |
| 29 | `PL!-pb2-020-N` | 絢瀬絵里 | 常時 | `gain_resource` | `PL!-bp5-008-R` 小泉花陽 | 0.97 | `success_score_sum_constant_heart_test.rs`::success_score_sum_grants_fixed_exact_color_hearts_pl_bp5_008_r |
| 30 | `PL!-pb2-021-N` | 南ことり | 自動 | `sequential`, `change_state`, `gain_resource` | `PL!HS-bp5-014-N` 安養寺 姫芽 | 0.60 | `parser_issues_e2e_test.rs`::issue10_dream_with_you_high_blade_plus_1_score |
| 31 | `PL!-pb2-023-N` | 星空 凛 | 常時 | `gain_resource` | `PL!-sd1-001-SD` 高坂 穂乃果 | 0.88 | `success_score_gated_look_fetches_musume_test.rs`::pl_bp4_006_r_score_three_or_more_fetches_mus_member |
| 32 | `PL!-pb2-024-N` | 西木野真姫 | ライブ開始時 | `change_state` | `PL!SP-pb2-047-L` Welcome to 僕らのセカイ | 0.82 | `all_liella_gated_opponent_wait_test.rs`::fire_enemy_wait_test |
| 33 | `PL!-pb2-025-N` | 東條 希 | 常時 | `gain_resource` | `PL!-sd1-001-SD` 高坂 穂乃果 | 0.89 | `success_score_gated_look_fetches_musume_test.rs`::pl_bp4_006_r_score_three_or_more_fetches_mus_member |
| 34 | `PL!-pb2-026-N` | 小泉花陽 | 起動 | `look_and_select`, `look_at`, `select_cards` | `PL!-bp3-014-PR` 星空 凛 | 0.75 | `debut_optional_wait_look_two_reorder_test.rs`::cost_paid_applies_wait_state |
| 35 | `PL!-pb2-028-N` | 高坂穂乃果 | ライブ開始時 | `gain_resource` | `PL!N-bp3-006-R` 近江彼方 | 0.77 | `optional_member_topdeck_blade_pl_s_bp3_021_l_test.rs`::pl_s_bp3_021_l_live_start_member_to_deck_top_grants_blade |
| 36 | `PL!-pb2-029-N` | 絢瀬絵里 | ライブ開始時, 登場 | `change_state` | `PL!N-sd2-013-SD2` 上原歩夢 | 0.98 | **none** |
| 37 | `PL!-pb2-030-N` | 南ことり | 常時 | `gain_resource` | `PL!-sd1-001-SD` 高坂 穂乃果 | 0.91 | `success_score_gated_look_fetches_musume_test.rs`::pl_bp4_006_r_score_three_or_more_fetches_mus_member |
| 38 | `PL!-pb2-031-N` | 園田海未 | ライブ開始時 | `gain_resource` | `PL!HS-bp6-025-L` ツバサ・ラ・リベルテ | 0.73 | `q257_wait_self_then_score_four_live_from_waitroom_to_hand_test.rs`::izumi_bp6_q257_multiple_matches_use_filtered_waitroom_indices |
| 39 | `PL!-pb2-032-N` | 星空 凛 | 登場 | `look_and_select`, `look_at`, `select_cards` | `PL!N-bp7-018-N` 近江彼方 | 0.93 | **none** |
| 40 | `PL!-pb2-033-N` | 西木野真姫 | ライブ開始時, 登場 | `change_state` | `PL!N-sd2-019-SD2` 優木せつ菜 | 0.93 | **none** |
| 41 | `PL!-pb2-037-L` | Shangri-La Shower | ライブ成功時 | `move_cards` | `PL!SP-bp4-006-R` 桜小路きな子 | 0.56 | **none** |
| 42 | `PL!-pb2-038-L` | 嵐のなかの恋だから | 常時 | `modify_score` (**or_location gap**) | `PL!-bp4-019-L` Angelic Angel | 0.69 | `success_zone_group_gated_constant_score_test.rs`::angelic_angel_does_not_bleed_to_live_set_zone |
| 43 | `PL!-pb2-039-L` | 僕たちはひとつの光 | ライブ開始時 | `modify_yell_count` | `PL!SP-bp2-010-R＋` ウィーン・マルガレーテ | 0.69 | `real_card_phase_walkthrough_and_ability_suite_test.rs`::hareruya_q74_multiname_distinct_counting |
| 44 | `PL!-pb2-039-L` | 僕たちはひとつの光 | ライブ成功時 | `modify_score` | `PL!HS-bp2-020-L` Link to the FUTURE | 0.68 | `success_score_gated_look_fetches_musume_test.rs`::pl_bp4_006_r_low_success_score_does_not_fetch_mus_member |
| 45 | `PL!-pb2-040-L` | Love marginal | ライブ開始時 | `sequential`, `modify_required_hearts` ×3 | `PL!N-bp5-026-L` TOKIMEKI Runners | 0.67 | `tokimeki_runners_all_heart_colors_live_success_score_test.rs`::tokimeki_all_6_colors_collectively_score_plus_1 |
| 46 | `PL!-pb2-041-L` | 春情ロマンティック | 常時 | **`custom`** | `PL!-bp4-019-L` Angelic Angel | 0.55 | `success_zone_group_gated_constant_score_test.rs`::angelic_angel_compound_and_both_required |
| 47 | `PL!-pb2-041-L` | 春情ロマンティック | ライブ開始時 | `modify_score` | `PL!-bp3-024-L` 夏色えがおで1,2,Jump! | 0.87 | `score_condition_integration_test.rs`::rin_score_condition_below_then_above |
| 48 | `PL!-pb2-042-L` | PSYCHIC FIRE | 自動 | `change_state` (**missing `group_names`**) | `PL!HS-pb1-010-R` 村野さやか | 0.52 | `cost_and_blade_filtered_opponent_wait_test.rs`::sayaka_hs_pb1_010_debut_with_cost_ten_member_waits_cost_four_opponent |

### 5.1 The five with no analogue to copy

These have no existing test that exercises a comparable mechanic, so they need
tests written from the card text rather than adapted:

| Card | Text | Why nothing to copy |
|---|---|---|
| `PL!-pb2-001-PP` | three feature-gated branches in one ライブ開始時 | no existing 3-way feature gate over one zone |
| `PL!-pb2-016-P＋` | {{live_start}}…{{icon_b_all}}… | condition is *any-colour* blade in a zone |
| `PL!-pb2-029-N` | 手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いてもよい：自分のステージにいるメンバーが『BiBi』のみの場合… | optional discard **plus** a stage-exclusivity condition |
| `PL!-pb2-032-N` | 自分のデッキの上からカードを5枚見る…ブレードハートを持たない『μ's』のメンバー | look-and-select filtered on *absence* of blade hearts |
| `PL!-pb2-037-L` | エールにより公開された自分のメンバーカードがすべて『Printemps』か、すべて『lily white』か、すべて『BiBi』のいずれかの場合 | all-one-of-N alternative over a group set |

`PL!-pb2-033-N` is a sixth if you count `PL!N-sd2-019-SD2` as unusable; it is a
compound 登場/ライブ開始時 double-trigger, for which
`q257_wait_self_then_score_four_live_from_waitroom_to_hand_test.rs` is close enough
to adapt.

---

## 6. What is needed, in dependency order

### Parser (`cards/ability_extraction/parser.py`)

| # | Work | Unblocks |
|---|---|---|
| P1 | Static replacement-appearance rule → typed node (not `custom`) | `PL!-pb2-002` |
| P2 | Count-as-N rule for `枚数を数えるとき、このカードはN枚として扱う` | `PL!-pb2-041` |
| P3 | `all_blade_timing` on the `[ALLブレード]` filter | `PL!-pb2-001` |
| P4 | Carry `distinct: "card_name"` into nested optional costs | `PL!-pb2-018` |
| P5 | `or_location` — keep both locations | `PL!-pb2-038` |
| P6 | `group_names` for named-member filters in 自動 clauses | `PL!-pb2-042` |
| P7 | Optional-cost `N枚…か…N人をウェイト` branch (stop emitting bare `modify_score`) | `PL!-pb2-012` 起動 |

After each: regenerate and confirm `git diff --numstat cards/abilities.json` shows
**only** the intended entries changed. A parser edit that shifts unrelated output is
how the `action_047` incident in `PARSER_NOTES.md` deleted three live rules.

### Engine (`engine/src/`)

| # | Work | Where |
|---|---|---|
| E1 | Replacement effect evaluated at member placement (front area, printed hearts ≤ 4) | appearance path; reuse the live-zone replacement seam |
| E2 | Counting weight in the shared count dispatchers | `ability/condition/card.rs` — **needs characterisation pins first** (the three dispatchers already overlap) |
| E3 | `distinct` enforcement in nested optional costs | cost handlers |
| E4 | `all_blade` condition matching | condition module |
| E5 | `or_location` in `ConditionCommon` | condition model → regenerate decoders |

`EffectFilter` currently has 159 fields and is the single source of truth for shared
effect fields; new fields go there, then the decoders are regenerated — never edit
`effect_decoder_gen.rs` / `condition_decoder_gen.rs` by hand.

### Tests (`engine/tests/`)

- **48 behavioural tests, one per new ability.** None exist today. Per `AGENTS.md`,
  write the printed-behaviour assertion first, then change the parser/model/engine.
- Each needs the four paths, not just the fire path: **positive**, **no-fire**,
  **boundary**, and **duration/cleanup** where the ability has one.
- Priority order: the 2 `custom` mechanics (2, 46), the 5 with no analogue
  (1, 20, 36, 39, 41), then the 4 missing-mechanics cards (42, 23, 16, 48), then the
  rest by `Sim` descending — the high-`Sim` rows are near-clones of a tested
  mechanic and are the cheapest confidence.
- `zero_tested_action_types_test.rs` and `docs/ABILITY_MATRIX.md` are regenerated by
  `python cards/test_inventory.py`; re-run it after adding tests or CI's
  `--check` gate fails.

### Gates

```bash
python cards/ability_extraction/extract_card_abilities.py   # zero custom actions
python cards/test_inventory.py
cd engine && cargo test --test run_all                       # zero failures
```

---

## 7. Method notes / caveats

- The `cards` field in `abilities.json` holds display strings
  (`"PL!-pb2-002-PP | 絢瀬絵里 (ab#0)"`), **not** card numbers. Any tooling that
  looks these up directly gets `None` for every card.
- "Closest existing ability" is pure text similarity. For rows where `Sim < 0.6` it
  is a rough pointer, not a semantic match — read the card before copying the test.
- The test column lists tests that *mention* the analogue card, ranked by whether
  the file or function name encodes it. It is a starting point for a test to model,
  not a claim that the test covers the same mechanic.
- The extractor currently returns 0 for `card_count` mismatches on this pack, so
  none of the 48 is a pure retext of an existing ability.

---

## 8. Test campaign log

Work done ability-by-ability, least work first. Each entry records what the test
found, because **most of these were not visible from reading the parse**.

Method per ability: write the test to the *printed card text*, run it, and treat a
failure as a defect in the parser/engine rather than a wrong test — except where the
parse is defensibly reading a genuinely ambiguous card, which is called out.

| # | Card | Tests | Defect found | Fix |
|---|------|------:|--------------|-----|
| 47 | `PL!-pb2-041-L` | 6 | none | — |
| 29 | `PL!-pb2-020-N` | 8 | none | — |
| 31 | `PL!-pb2-023-N` | 4 | none | — |
| 33 | `PL!-pb2-025-N` | 8 | none | — |
| 4 | `PL!-pb2-004-PP` 常時 | 9 | **`{{icon_score}}を持つ` filter dropped from per-unit 「〜につき」 counts.** The effect kept `per_unit`/`location` but lost `card_property`, so every μ's live in the zone counted, not just score-icon ones. | `_fix_condition_enrichment` only applied the icon to `move_cards`/`select`; extended to per-unit effects. Diff: 2 lines, no other ability moved. |
| — | *(corpus repair)* | — | **`special_heart: {}` on 89 live cards.** `Card::has_score_icon` reads it, so 88 lives that print a スコア/ドロー icon matched nothing and the yell printed no icon (`turn/live.rs:173`). **72 of the 89 predate PBLL02**; I had copied the defect from a sample instead of the convention. | `scrape_expansion.py` now emits `{"score": 1}` / `{"draw": 1}`; all 89 repaired from the site. Verified against the API: 42/42 agreement with the 43 pre-existing correct cards, 0 conflicts. **Zero existing tests broke.** |
| 13 | `PL!-pb2-011-P＋` | 9 | none | — |
| 37 | `PL!-pb2-030-N` | 10 | **「スコアの合計5につき」 counted cards, not points.** `合計5` matches none of the 人/枚/つ/コスト patterns, so `per_unit_count` stayed 1 and the ability became "one blade per card". 4 score-1 lives read as 4 blades. | New `per_unit_aggregate: "score"` (parser) + `resolve_per_unit_score_sum` (engine) + `heart_total`-style `EffectFilter` field and generated decoder. |
| 25 | `PL!-pb1-029-SRL` | 8 | none | — |
| 28 | `PL!-pb2-019-N` | 4 | **declining an optional `sequential_cost` still applied the deferred self-wait.** `pay_deferred_costs` ran unconditionally in `finalize_choice`, so 「ウェイトにし、手札を1枚控え室に置いてもよい」 waited her with no discard. | `finalize_choice` now drops deferred costs when the answer was a decline (rule 9.4.2.2, as the handler's own comment already claimed). |
| 6,7,8 | `PL!-pb2-006-PP` | 8 | **「元々持つハートの数が1つ以下」 was dropped.** `original_value: true` survived but the numeric ceiling did not, so a 2-heart member was a legal target. | New `heart_total_limit` / `heart_total_operator` on the effect and `CardFilter::original_heart_total`, checked in `matches()`; wired into `change_state`'s hand-built filter. The same rule also recovered two further clauses: `PL!-pb2-033-N` (≤3) and `PL!-pb2-042-L` (≤4). |
| 6b | `PL!-pb2-005-PP` | 6 | **Four stacked defects, found only because the test asserted the printed text.** (a) `_enrich_gain_abilities` stripped *every* `【…】`, deleting resource icons, so 「【常時】…【ブレード】を得る」 reparsed to a bare 「、を得る」 → `custom` → **no `gained_effect` at all**; the card gained an ability the engine could not execute. First card in the corpus to gain an ability containing a printed icon, so 20 prior gains never hit it. (b) `_try_heart_possession` accepted ANY `{{icon_*}}` before を持つ, so `{{icon_score}}を持つ『μ's』のカード` was frozen into `card_type: member_card, location: stage` — unsatisfiable, since a member carries no スコア icon. (c) The first two conditions of 001's elided branches parsed to unsupported `custom`. (d) `check_card_property` in `condition/card.rs` only collected cards for `RevealedCards` and `Stage`, then returned **true** on an empty list — so a `card_property` on any other zone was never checked. | (a) strip only the leading trigger bracket, and map known labels back to `{{icon}}` via `_GAIN_ICON_LABELS`. (b) gate on `_HEART_ICON_HELD_RE` (heart icons only) and decline anaphoric 「それら」, which belongs to the generic extractor. (c) new `zone_card_property` handler + `_propagate_elided_condition_location` for elided sibling branches, inheriting **only** the zone. (d) fall back to `zone_cards()` for every other zone. Plus an engine fix: `stage_member_targeting` (ability_ops.rs) must not fire when the *granted* text itself names stage members. |

| 24 | `PL!-pb2-024-N` | 5 | none in the engine — **my fixture was wrong** | `PL!SP-pb1-014-N` (cost 2) and `PL!SP-bp1-014-N` (cost 4) are different cards with the same number and name. I transcribed the wrong prefix, and without `assert_card_cost` the test failed for a reason that had nothing to do with the ability. |
| 33 | `PL!-pb2-033-N` | 7 | none — this is the regression guard for the `PL!-pb2-006` fix, at a different bound (≤3) | — |
| 12 | `PL!-pb2-012-P＋` | 7 | **the 「2人」 was a condition and got dropped.** 「…『Printemps』のメンバーを2人ウェイトにしてもよい」 left `count` unset, so the condition degraded to "any Printemps member on stage" and the −2 applied with ONE ally. | Count extracted in `_enrich_condition_result` (the shared enrichment, **not** `_extract_group_scope` — the priority-5 `distinct` handler claims this exact text and short-circuits the generic extractor). |
| 38 | `PL!-pb2-038-L` | 4 + 3 ignored | **OPEN — see §8.3 item 8.** A 常時 `live_total` effect on a live card in a live zone has no routing at all. | Not fixed; root cause traced to two functions and the ignored tests assert the correct value. |

Suite after each step: only the 2 pre-existing `custom` failures remained
(3825 passing at the time of writing).

### 8.0 Two corrections I had to make to my own reasoning

Recorded because both cost real time and both would be easy to repeat.

- **Empty `special_heart` is not a convention, it is a defect — but mostly not
  mine.** I attributed the 89 empty `special_heart` entries to my own scraper.
  Cross-checking the same cards against the API showed **72 of the 89 predate
  PBLL02**; I had sampled one set, seen the key *present* on 35/35 lives, and copied
  the majority defect. The falsifiable check is the API comparison (42/42 agreement
  with the 43 correctly-populated cards), not agreement between two sampled sets.
- **`select_option(0)` is not a decline.** On a `SelectCard` prompt it *selects the
  first card*. The decline idiom is `select_indices(&[])`. On a `SelectTarget`
  pay/skip gate `0` is decline, so the same index means opposite things on the two
  prompt types — worth checking `pending_choice_type()` before writing either.

### 8.1 Three parser paths for the same feature

The single most expensive thing to learn: **per-unit 「〜につき」 clauses are parsed by
two independent implementations**, and `gain_ability` clauses by more than one. A fix
in the wrong one is silently inert.

- `_extract_per_unit_info_from_text` (parser.py) — emits `per_unit_type:
  "success_live_card_zone"`, handles the ブレードハート property filter.
- `_per_unit_count_type` + `_per_unit_filters` — emits `per_unit_type: "card" / "枚"`,
  and the keyword loop picks the type from 「カード」/「人」/「ブレード」/「スコア」.
- `_check_ability_gain_from_text` handles the quoted 「…「X」を得る」 shape for *some*
  paths, while other paths build the same `gain_ability` node and then run
  `_fill_defaults` over the **full** text — so a fix in the first has no effect on the
  second.

Two mechanisms make this worse:

- **`_PROPAGATE_FIELDS` is a whitelist.** A new field added to one setter and not the
  other is dropped with no error. Adding a per-unit field means adding it to
  `_PROPAGATE_FIELDS` *and* to both setters.
- **Condition handlers are priority-ordered with a greedy last resort.**
  `_try_heart_possession` sits at tier 7 and was claiming any `{{icon_*}}…を持つ`
  text. Narrowing a handler can hand a text to a *different* wrong handler rather
  than to the right one, so every narrowing needs a `git diff` check on the affected
  card, not just a suite run.

When adding a per-unit or gained-ability field: add it to `_PROPAGATE_FIELDS` and to
every producing setter, confirm with `git diff cards/abilities.json` that the change
is not inert, and re-check any neighbouring ability whose parse the change touched.

### 8.2 Constancy also needs a mechanical check

`special_heart: {}` was present-but-empty on 95 cards and "looked right" because I
sampled one set where 35/35 lives had the key. Empty-dict is behaviourally identical
to absent everywhere the engine reads it (`card.rs:4193` iterates nothing;
`live.rs:173` credits no icons), so a sampled eyeball cannot distinguish a convention
from a defect. The cross-check that settled it: fetch the same cards from the API and
compare — 42/42 agreed, which proved the mapping right *and* proved the empties wrong.

### 8.3 Untestable / open questions

These cannot be settled by a test and are recorded rather than guessed.

1. **「メンバーが『lily white』のみの場合」 over an EMPTY stage** (`PL!-pb1-029-SRL`).
   のみ is a universal, so an empty stage is vacuously true → `+1` is the defensible
   reading; `0` is the conservative one. The card does not say. The engine currently
   returns `0`. `empty_stage_is_recorded_as_vacuous` in the test file asserts only
   `0 | 1` and must not be tightened until the ruling is made.
2. **The trailing parenthetical on `PL!-pb2-019-N`** — 「（ウェイト状態のメンバーが
   持つブレードは、エールで公開する枚数を増やさない。）」 is a rules clarification about
   the yell, not an effect. It is not asserted anywhere and there is no engine
   surface for it.
3. **`PL!-pb2-037-L`** (all-of-N alternative over a group set) — the equivalent
   三選一 across three groups. No existing mechanic to characterise against, and the
   printed text does not settle whether an empty reveal satisfies 「すべて…のいずれか」.
4. **The `b_heart07` (無 blade heart) encoding.** `compile_cards.py` resolves
   `b_heart07` through `HEART_COLORS["heart07"]`, which is absent, so it falls to the
   default `0` — the 無 colour. That happens to be the right *colour* but the wrong
   *value*, and 4 pre-existing cards plus `PL!-pb2-039-L` share it. Not introduced by
   this pack and left alone deliberately; needs its own decision.
5. **`PL!-pb2-001` — three branches sharing one zone clause.** The printed ライブ開始時
   states the subject once and then gives three per-icon clauses. Only the first names
   the zone; the other two are elided and parse to an unsupported `custom` without
   `_propagate_elided_condition_location` (see the table above). Its third branch tests
   `{{icon_draw}}`, and **there is no `has_draw_icon` card property** — the engine
   supports exactly three (`has_blade_heart`, `has_score_icon`, `has_all_blade`) — so
   that branch is stored but cannot be evaluated and degrades to "any card in the
   zone". This ability is not tested: a test would have to assert behaviour the engine
   cannot produce.
6. **`PL!-pb2-032-N` — the select prompt ignores 「加えてもよい」.** The take step parses
   with `optional: true`, but the resulting `SelectCard` prompt is **not skippable**:
   answering it with an empty selection is rejected. A second, separate observation from
   the same file: answering with an index does not place the card in hand. Both are
   recorded rather than asserted — the candidate filter *is* pinned (the negation
   `ブレードハートを持たない` crossed with the group filter, proven by offering 1 of 3
   candidates), so the gap is in the prompt plumbing, not the filter. Whether the take
   is a `select_cards` with `optional` or a `choice_condition` needs a decision, since
   the two give different decline semantics.
7. **`PL!-pb2-010-P＋` — a per-unit blade whose count is always 0.** The parse is right
   (`per_unit`, `state: "wait"`, `location: "stage"`, `group_names: ["Printemps"]`),
   the member IS in wait with unit Printemps, and `orientation_matches_state` handles
   wait correctly — but the count is 0 whether the member is in wait or active, so it
   is the group/zone resolution in `resolve_per_unit_count`, not the state filter.
   The three negative branches pass and are kept in a file of their own; the positives
   are not asserted until the count is traced.
8. **The baton-touch trap, and how widespread it is.** `play_to_stage` into an area
   that already holds a member is a BATON TOUCH: the occupant is replaced and goes to
   the waitroom. A test can stage a card intending "these two are both on stage",
   play the subject into the same area, and quietly end up with one — a *passing but
   wrong* negative. That is exactly what happened to `PL!-pb2-029-N`'s
   「メンバーが『μ's』のみ」 case: the stage has three slots, the card takes the
   centre, and staging two allies means playing into an occupied centre, which
   baton-touched the AZALEA member the negative depended on.

   **137 tests in this suite rely on an implicit baton touch.** Measured by adding a
   temporary assertion to `TestGame::play_to_stage` that the target area is empty,
   and counting the failures — not by static analysis, which was wrong three times
   (it could not distinguish `player1.stage.stage[1]` from `player2.stage.stage[1]`,
   and could not tell a deliberate swap from an accident).

   The assertion was **reverted rather than landed**: 137 failures each need a
   judgement about whether the swap was intended, and converting them all to the
   explicit variant would silence the check without resolving which ones are wrong.
   What landed instead is additive and safe:
   - `play_to_stage_baton(...)` for tests that *mean* to swap, and
   - `TestGame::stage_occupant(card, area)`, so a test can assert its own setup —
     which is the assertion §8.3's vacuous negative should have had.

   Triaging the 137 is the outstanding work: each is either a deliberate swap (rename
   to `play_to_stage_baton`) or a setup that no longer matches what the assertions
   describe. Until that is done the hazard stays silent for those tests.
9. **`PL!-pb2-015-P＋` — branch 1 is untested; I first blamed the engine and was
   wrong.** I reported this card as applying BOTH branches of
   「以下から1つを選ぶ」. That was my test, twice over:

   - activation is recorded as the orientation modifier `"active"`, **not** as the
     absence of one — `get_orientation_modifier` returns `Some("active")`;
   - a blind `drain_auto_ability_choices()` after the choice was answering branch 1's
     「どのメンバー」 follow-up prompt on the *branch-2* run, which is what made the
     member appear to activate either way.

   With the prompts answered deliberately, branch 2 is correct end to end (exactly two
   energy cards activate, no follow-up prompt, no member activated) and that is
   pinned. Branch 1 is **not** pinned: it needs a BiBi member in wait, and a member
   written into the stage with `add_orientation_modifier(..., "wait")` reads back with
   no orientation modifier by the time the ability fires, so the fixture could not be
   built reliably.

   The mechanism itself is shared with 19 other cards (all `action: choice` with N
   sub-effects; `PL!S-bp6-020-L` and `PL!SP-pb2-011-PP` have 3) and works — a claim
   worth not making twice.
10. **`PL!-pb2-009-PP` works; the ABSOLUTE value is the trap.** The ability is correct
    and pinned (4 tests), but only after two measurement mistakes that are worth
    recording because they make a correct ability look broken:

    - *Comparing different incoming cards conflates cost with effect.* Paying for the
      play draws from the energy zone, so a cost-22 baton left 29 active against 40
      for a cost-9 one. Fixed by playing the SAME card twice — once displacing the
      subject, once into an empty centre — so the cost cancels.
    - *The absolute is still wrong.* Displacing 矢澤にこ yields **9** energy by
      itself; the qualifying case yields **11**. 「エネルギーを2枚」 is the **margin**
      over that baseline, not the absolute. Reading 11 against a written 2 is what
      made this look like a bug.

    Generalisable: when an effect is measured through a shared resource, pin the
    baseline and assert the **difference**, and check whether a same-setup control
    already moves that resource before concluding the effect did nothing.
11. ~~**`PL!-pb2-039-L` — `modify_yell_count` inverted, count group-blind.**~~
    **NEITHER WAS REAL. Both were my test; both are now pinned (5 tests).**
    - *Direction.* `cheer_checks_required` **is** the number of cards the player must
      reveal — `perform_cheer_check` draws one card per check and completes the cheer
      when `cheer_checks_done >= cheer_checks_required` (tracking.rs). So
      「10枚増える」 means the requirement goes **up** by 10, which is what the engine
      does. I had assumed "more revealed" must mean fewer required checks and asserted
      the opposite sign. The convention matches `PL!SP-bp2-010-R＋` Wien, whose
      「8枚減る」 lowers the requirement by 8.
    - *Group filter.* My "non-μ's" control, `PL!-sd1-019-SD`, has `unit = None` but
      `series = ラブライブ！`, and `card_series_matches_group` therefore makes it a μ's
      card. A card with no series (`LL-PR-004-PR`, AiScReam) is the real control, and
      with it the filter behaves correctly.
12. ~~**`lily white` and `lilywhite` are the same group and the matcher disagrees.**~~
    **FIXED.** The corpus spells it both ways — `lily white` on 4 lives (incl.
    `PL!-pb1-029-SRL`, `PL!-pb2-041-L`) and `lilywhite` on members incl.
    `PL!-PR-004-PR` 園田海未, `PL!-PR-005-PR` 星空 凛, `PL!-PR-007-PR` 東條 希 — and a
    raw `==` in `card_matches_group_str` made a `『lily white』` filter skip every
    unspaced card, so any ability gated on that group under-triggered.
    `CardDatabase::normalize_name` already stripped whitespace; it was simply not used
    on that path. Now compared with an allocation-free `eq_ignoring_whitespace`,
    applied to the unit, the `group` field and the name substrings. Pinned by 5 tests
    that also fix what must NOT merge: a different group, and a group that is a
    substring of another (`lil` must not match `BiBi`) — the comparison is
    equality-after-stripping, not `contains`.
13. **`modify_score` + `per_unit` does not fire — a general gap, not one card.**
    `PL!-pb2-039-L`'s ライブ成功時 parses correctly (`modify_score` +1, `per_unit`,
    `人`, `group_names: ["μ's"]`, `distinct: card_name`) and returns **+0** for every
    positive case — one μ's member or three, on stage or revealed. The nearest
    pre-existing analogue, `PL!HS-bp2-020-L` (蓮ノ空, ライブ開始時, identical shape
    `人` + group + `distinct: card_name`), also returns **+0** — so this is a gap in the
    per-unit SCORE path, not a parser or fixture problem, and it is untested on both
    cards. `modify_score` combined with `per_unit` occurs **11 times** in the corpus
    and none of the group-filtered instances is covered. Only the over-firing cases
    (empty pool, non-μ's staged, non-μ's revealed) are pinned, so a future
    over-firing regression is caught while the under-firing gap stays open.
14. **`PL!-pb2-021-N` — the 自動 over-fires; one of its two `or` branches is a
    vacuous comparison. Parser half FIXED, runtime half open.**
    「このメンバーが、自分のカードの能力のコストを支払うため**か**、自分のカードの効果に
    よってウェイト状態になったとき」 — the wait branch works (she is activated and
    gains a blade, pinned), but the ability fires even with her active, having paid
    nothing and never been waited.

    - *FIXED.* Branch 2 was parsed as a bare `comparison_condition` with `count >= 1`
      and no type or location — vacuous. It is now a self-scoped `state_condition`
      (`state: wait`, `target: self`, `self_target: true`) via a new
      `self_became_wait` condition pattern. Blast radius checked: 11 `state_condition`
      nodes exist in the corpus and **none carries heart colours**, so all 11 were
      previously decided by an unrelated path; the suite stays green.
    - *OPEN.* Branch 1, 「…能力のコストを支払うため」, is an EVENT ("she paid a cost")
      but is parsed as a cost **comparison** with no operator and no count.
      `compare_counts` defaults to `>=` (util.rs:2128), so it evaluates as
      `cost >= 0` — **always true**. With one branch vacuous the `or` always passes.
      The correct fix is to parse 「コストを支払う／支払った」 as a trigger event
      rather than a comparison; there is currently no such rule.

15. **The two `custom` abilities — one fixed, one not.** Neither was a typing problem:
    the parser produced a usable node for each, and only the action name was `custom`.

   - **`PL!-pb2-002-PP` 絢瀬絵里 — FIXED and tested** (8 tests). A REPLACEMENT appearance
     rule, 「このメンバーの正面のエリアには、元々持つハートの数が4つ以下のメンバーは、
     ウェイト状態で登場する」.
     - *Engine*: resolved ONCE at placement
       (`GameState::replacement_appear_wait_applies`, called from
       `handle_play_member_to_stage`). Deliberately not a constant: constants re-apply
       every recalculation, so the member could never leave wait and activations paid
       for in play would be clobbered. `the_member_can_still_be_activated_afterwards`
       is the test that fails under a constant implementation.
     - *What cost the most time*: the fields decoded fine but came back `None`. The
       cause is already written up in `docs/ABILITY_PIPELINE.md` §"The decoder is
       generic, but the post-processing is manual" — `populate_from_json()` maps an
       action to a typed `EffectKind`, and **an action with no entry in that map
       silently loses its whole filter** while still decoding its `action` and
       `text`. Adding `replacement_appear_wait` to the `ChangeState` arm of that map
       fixed it. Read that section before adding an action, not after.
     - Two bugs the tests caught that reading the code did not: the host's own rule
       matched the host on the turn she arrived (she is on stage by the time the check
       runs) and waited her, which then disabled her rule for everyone else; and the
       `position` check compared the host's area to itself, so it matched every area.
   - **`PL!-pb2-041-L` 春情ロマンティック — NOT fixed, still `custom` and still
     failing.** A counting weight, 「…枚数を数えるとき、このカードは2枚として扱う」.
     Most of it is written and works in isolation: `count_as_multiple_weight` returns
     2 for the card when the counting card is in a `counted_by` group, and the shared
     `count_filtered` closure adds the extra weight on top of the normal count, deciding
     membership with the same filter applied one card at a time so a weighted card can
     only ever be a card the count already included.

     It fails because the condition evaluates as if no weight were present: the
     counting card is resolved from `activating_card` / the ability queue's current
     entry, and both are empty (`activating_card` is `-1`) when 041's ライブ開始時
     condition runs. Threading the running effect into condition evaluation is a
     change to how `ConditionContext` identifies its effect, and `PARSER_NOTES` already
     records that `get_count_for_condition` / `get_group_card_count` /
     `get_count_for_target` overlap — so it needs its own pass, with characterisation
     pins per condition shape.

     It is left as `custom` on purpose: typing it would make `test_no_custom_actions`
     pass while the mechanic did nothing, which is the same false green that was
     reverted twice during this work. The code, the probe results and the reason are
     in `_type_count_as_multiple`'s docstring.
16. **「自分のセンターエリアにいる『X』のメンバー」 cannot be `all`.** `modifiers.rs:591`
   grants an `all: true` constant to every matching member on the whole stage, so
   setting `all` on the centre-clause cards (`PL!-bp4-011-N`, `PL!-bp4-017-N`,
   `PL!-pb2-019-N`) breaks three existing tests. They are correct today only because
   the centre area holds exactly one member, which makes all-vs-one indistinguishable —
   so **no test currently distinguishes them either**, and a real fix needs
   `all` *and* a centre restriction together. Recorded rather than asserted.
17. ~~**`PL!-pb2-038-L` — a 常時 on a LIVE card in a live zone, targeting
   ライブの合計スコア.**~~ **FIXED.** Three changes were needed, and the third was
   only visible once the first two made the effect fire:
   - the success-zone constant evaluator's `ModifyScore` arm had no `live_total`
     case, so the effect fell into "target the live set zone cards" and — with that
     zone empty, the normal state for a card in the success zone — contributed 0;
   - there was no constant scan of the **live card zone** at all, so the
     ライブカード置き場 half of the 「か」 never contributed either;
   - `extract_locations` deliberately dropped `live_card_zone` whenever the success
     zone was present (to fix a substring artefact) and so ate the genuine
     「A**か**B」 disjunction. Fixed to distinguish the two, with a new `any_location`
     marker so the multi-location path stops reading a disjunction as a
     「AをBに」 move.

   And then the card's own negatives failed, which is the point worth recording:
   **`evaluate_group_condition` with `all_members` only ever asked "are they all in
   the group"** — it never checked `count`. So 「2人の『μ's』のメンバーのみがいる」 was
   satisfied by one member and by three. Those two tests had been passing
   *vacuously*, because the constant they were testing never fired. A negative test
   that can only fail if the positive path works is not yet a real test — worth
   checking negatives survive the moment a positive starts passing.

### 8.4 Things a test cannot tell you

Two defects produced parses that were *plausible* rather than empty, which is why the
suite stayed green until each card was tested against its printed text:

- `check_card_property` returned **true** on an empty card list, so a filter that was
  never evaluated read as a filter that passed. Empty-means-fail would have surfaced
  it as a hard failure instead of a silent over-permissive condition.
- `per_unit_count` silently defaulted to `1` when the text's number was not preceded
  by 人/枚/つ, turning 「スコアの合計5につき」 into "per card". The count was present,
  just wrong, so nothing downstream complained.

General rule this campaign supports: for any new filter, prefer **fail-closed** on an
unresolved input, and pin each filter with a fixture that specifically *lacks* the
filtered property — a positive fixture alone cannot tell a working filter from an
absent one.
