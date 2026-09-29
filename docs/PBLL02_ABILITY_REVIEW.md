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


### 8.6 A real engine bug the 041 weight exposed

`engine/src/ability/effects/score.rs`, in `execute_modify_score`.

The target pool for a live-card score effect is built from several zones:

```rust
let mut ids: SmallVec<[i16; 8]> = player
    .live_card_zone
    .cards
    .iter()
    .chain(player.success_live_card_zone.cards.iter())
    .copied()
    .collect();
```

A live being performed is the live card **and**, once it has succeeded, an entry in the
success live card zone. When both hold the same card, `ids` contains that id twice, and
the per-card filter further down keeps both occurrences. The effect is then applied once
per occurrence, so 「このカードのスコアを＋１する」
(`PL!-pb2-041-L`) paid out **+2 from a single activation**.

The adjacent self-target push already guarded against exactly this with
`if !ids.contains(&act)`, which is the tell: the hazard was known at one insertion point
and unguarded at the other. Fixed with an order-preserving `retain` on `ids` so a card
id is scored at most once per effect, which matches 「このカード」 being one card.

This is not specific to this card or this expansion — any self-targeted or
zone-scanning score effect was affected wherever the engine tracks one card in two
scanned zones.


### 8.7 A real parser bug: a さらに clause that vanished (`PL!-pb2-000-DUO` / `-R`)

The 登場 text is two effects and the parse kept one:

> 『μ's〇のメンバー2人からバトンタッチして登場した場合、
> 自分の控え室にある『μ's〇のライブカードを1枚手牌に加え、
> そのを、そのメンバー2人のコストの合計が15の場合、
> ライブ終了時まで、「常時 ライブの合計スコアを＋１する。」を得る。

The waitroom recovery was silently dropped. The cause is not an effect rule:

**`_split_sentences_nesting` breaks only on `。`, and this ability separates every
clause with `、`.** So it produced a single "sentence", `_matches_furthermore` never
saw two parts, and the first clause was absorbed into the trailing `場合` condition
— which is why the old parse had a `cost_total: 15` condition covering text that was
really an action.

**Fix.** Split on a top-level `その先` itself, which is what the connective means,
then hoist a leading **trigger** gate to the sequential so it scopes the whole ability.
The debut is now `[move_cards, gain_ability]` with the baton gate on top.

**The narrow scope was not obvious and I got it wrong twice.** My first version applied
the marker split to any text where the sentence split failed, which claimed 31 abilities
and broke 14 tests — the per-unit and `きおおり` handlers above own most of
them. Two things had to be excluded, and neither is obvious from the text:

- A segment before `その先` that is a **gate**, not an action.
  `「8枚以上あるかぎり、まださど…」` and
  `「…場合、まださ…」` are a condition plus its effect. Note
  the gate segment parses to a *real* action type, so a `action == "custom"` check does
  not catch it; the giveaway is that its last clause ends in `場合` / `かぎり`.
- A **bare connective** like `「その後、」`.

Requiring the `その先` to sit inside the **first** `。`-sentence, with exactly
two clause segments, brings the blast radius to one ability and leaves 000 correct.

**A test can bless a data loss.** The old file had
`the_waitroom_recovery_is_currently_missing_from_the_parse`, which asserted the clause
was *absent* and passed. That is how a bug got a green test wrapped round it. It is
replaced by a positive assertion, and the reasoning is in the file header so nobody
re-adds an absence test for a thing the card text states unambiguously.

**The runtime trigger is covered now.** This section originally closed by calling the
recovery "verified as DATA" only. That was accurate at the time and the reason was a
missing test helper, not a missing feature: the engine takes its double-baton route only
when the play action carries two area INDICES (`phases.rs::play_double_baton`, which
records two touches), and `play_to_stage_baton` passes one — so against an empty area it
simply places the card and the gate's `min_count = 2` can never be met. `play_to_stage_double_baton`
now exists, and the file runs the whole chain end to end: two recorded touches, the
『μ's〇のライブカード reaching hand, a non-μ's live in the same 控え室 left alone, and the
cost-15 half correctly withheld at a 4 + 4 total.

Worth stating plainly, because it cost a whole investigation: **"the fixture cannot express
this" is a claim about the helpers, and it is checkable.** Before concluding it, look for
the code path that would handle the case — here `play_double_baton` already existed and
was reachable, one argument away.

### 8.8 A validator that did not know about `counted_by`

`extract_card_abilities.py` reported `『μ's〇 in … — PL!-pb2-041-L` as a group
filter with no `group_names` after 041 became typed. The parse was right: the text is
「自分の『μ's〇のカードの効果によって」, which names the effects that
*observe* the weight, and that is `counted_by`, not `group_names`. `FILTER_FIELDS` did
not list it, so the check reported a correct parse as unfiltered. Added, with a comment
recording that `counted_by` must never be used as a card filter.


### 8.9 「同じ選択項を2回以上選でもいい」: two defects behind one red test

`PL!-pb2-016-P+」's file had recorded, for a long time, that selecting a choice option
runs nothing. It was traced to `SelectTargetKind::Choice` calling `clear_choice_state`
without recording the selection, and the fix was attempted twice and reverted both times
because neither attempt changed the behaviour.

**The diagnosis was wrong, and the revert was the right call anyway** — nothing was made
worse, but nothing was learned. Re-doing it by measurement rather than reading found two
defects, neither of which is the one that was written down.

**1. Parser: `target: 「self」` on a member the player CHOOSES.** Option 1 parses as
`change_state` with `target: "self"`, so the engine resolves it to the activating card and
`change_state` auto-selects that through its `is_self_target` shortcut instead of
prompting — the option silently did nothing. 「自分のステージにいるメンバー1人をアクティブにする」 names a member
the player PICKS. Fixed by `_fix_member_selection_target`, which drops `target` for a
counted member on a stage unless the text says 「このメンバー」.

The same clause shape was broken on `PL!-pb2-015-P+」, whose member-activating option had
**no test at all** — its file covered the energy option, and even had a test named
`option_two_activates_no_member` that asserted a *freshly requested* `game.id(BIBI_MEMBER)`
rather than the card actually standing on stage. `game.id()` hands out a new pooled
instance per call, so that test could not fail. It now reads the card back from the stage,
and the missing branch has its own test.

**2. Engine: a per-card scope leaked onto the options.** With the target fixed the option
still found zero candidates, and the filter said why: `f.group=Some(「lily white」)`. The
parent choice's group names are 「自分の成功ライブカード置き場にある『lily white〇のカード1枚につき」
— they scope HOW MANY TIMES the choice is offered, not what an option may touch. But
`should_propagate_group` in `effects/choice.rs` copies a parent's `group_names` onto every
option that lacks its own, so option 1 ended up requiring a *lily white* member. Fixed by
excluding the per-card reading.

The discriminator is not obvious, and the guard is commented with its evidence: of the
choice effects that carry `group_names`, **016 is the only one whose text contains
「つき」**. The others (`PL!S-bp7-003-P`, `PL!S-bp5-004-AR`, `PL!N-bp7-005-P`,
`PL!HS-bp5-022-L`, …) genuinely mean a filter on the options and still propagate. That
was verified by enumerating them, not by assuming.

**Three things I got wrong while getting here, all recorded because they cost real time:**

- **A card number that is not a card number.** My first fixture staged
  `PL!-pb1-010-N`. It is absent from `cards.json` but present in the DATABASE, so
  `game.id()` correctly did not panic and the effect correctly found no candidates —
  indistinguishable, from inside the test, from an engine bug. I spent a full cycle
  chasing that. `game.id()` panicking on an unknown number is not sufficient protection
  when the number resolves to a card that simply is not stage-legal.
- **`MemberArea::Center as usize` is 1, not 0.** Reading the stage back is still the only
  way to be sure which instance the fixture used.
- **Believing the recorded diagnosis.** Every previous note pointed at
  `SelectTargetKind::Choice`. Reading that code confirmed the selection is discarded, and
  it is still not why the option did nothing. Two reverts had already proved the note was
  not actionable; the fix was to instrument, not to keep reading.


### 8.10 The recorded OPEN GAP that was never an engine defect (`PL!-pb2-018-P+」`)

This file carried a confident diagnosis: the positive case did not fire because

> the 登場 ability resolves to a single `SelectAutoAbility` prompt and the optional
> 「3枚控え室に置いてもいい」 cost is never raised afterwards

and called it "engine behaviour for an optional cost on a debut ability whose trigger is
a combined 「登場／ライブ開始時」 list, not a fixture problem".

**It was a fixture problem, and the fixture was trivially unsatisfiable.** `BIBI_2` was
`PL!-pb2-003-PP`, annotated in the test as 大潮にこ — but that card is **南ことり**,
a Printemps member. So of the three "BiBi" discards the cost was asked for, only one was
BiBi. `offer_hand_cost_choice` correctly found fewer than three eligible cards,
auto-skipped the optional cost, and the cost's own skip gate then switched the effect
off. Every link in that chain was right; the input was wrong.

With three genuinely distinct BiBi members the whole thing runs: the auto-ability order
prompt, then 3 + 2 + 1 hand selections, then the wait. BiBi has exactly three members
(絞渡絵里 / 西木野真姬 / 大潮にこ), so the cost can be paid in exactly one
way — the old fixture could never approximate it.

**Two more things the same fixture was hiding.** The old helper drained prompts with
`select_indices(&[])`, which DECLINES the debut's own 「登場」 order prompt — so the
ability carrying the cost was never selected in the first place. And the printed
diagnosis was assembled from that decline, which is why it read as confident and was
wrong: it described what the helper did, and attributed it to the engine.

**The generalisable lesson, now enforced in the file.** A wrong fixture constant is
silent. The card number resolves, the card exists, it is simply the wrong card — and the
resulting failure looks exactly like a deep engine bug. This happened three times in this
expansion, with three different wrong diagnoses:

| Card | Bad fixture | What it looked like |
|---|---|---|
| `PL!-pb2-016-P+」 | `PL!-pb1-010-N` (no unit) | option found no candidates → "the option is not dispatched" |
| `PL!-pb2-015-P+」 | `game.id(BIBI_MEMBER)` re-requested | a vacuous assertion that could not fail |
| `PL!-pb2-018-P+」 | `PL!-pb2-003-PP` is Printemps | "the optional cost is never raised" |

Every fixture constant in the 018 file now carries an explicit group assertion, checked
by a helper that each test calls. In two of the three cases there WAS a real defect
underneath (016's `target: "self"`, 015's missing branch) — but in none of them was the
defect the note described, and in 018's case there was no defect at all.


### 8.11 A standing check for the failure mode that caused three wrong diagnoses

Three separate investigations in this expansion ended the same way: a test fixture named
the wrong card, the resulting failure looked exactly like a deep engine bug, and a
confident multi-paragraph diagnosis was written down and believed. `PL!-pb2-018-P+」`'s
"OPEN GAP" turned out to have no defect behind it at all.

A wrong fixture constant is **silent**. The card number resolves, the card exists, it is
simply a different card, and nothing complains. `cards/audit_fixture_annotations.py` now
checks every annotated constant in the engine test tree against `cards.json`:

- the card number resolves (decoding Rust `\u{ff0b}` escapes, which several files use for
  the full-width plus);
- the character NAME claimed by the comment equals the real name;
- a group claim holds — μ's and friends are checked against `series`, the unit names
  (BiBi / Printemps / AZALEA / 5yncri5e! / …) against `unit`.

428 annotated constants, **19 of which named the wrong character**, all corrected. Real
examples, every one a comment that had been actively misleading:

| Constant | Card | Comment said | Actually |
|---|---|---|---|
| `ERI` | `PL!-pb2-021-N` | 絞渡絵里 | 南ことり |
| `ERI` | `PL!-pb2-010-P+＋` | 絞渡絵里 | 高均穋な果 |
| `UMI` | `PL!-pb2-008-PP` | 園田海未 | 小泉花陽 |
| `NOZOMI` | `PL!-pb2-026-N` | 東條希 | 小泉花陽 |
| `SUMIRE` | `PL!SP-bp1-018-PR` | 平安名すみれ | 米女メイ |
| `KEKE` | `PL!SP-bp1-004-PR` | 唐可可 | 平安名すみれ |

Two constants were **swapped** — `HANAYOME` on `PL!-pb2-007-PP` was labelled 小泉花陽
and `NOZOMI` on `PL!-pb2-026-N` was labelled 東條希, when the cards are the other way round.

**Two of the 19 rewrites were themselves wrong, and were reverted.** The fixer over-applied
to comments that are annotations, not names: `PL!HS-bp6-019-N // 自動無制限` and
`PL!HS-pb1-019-N // 起動` both became `// 大津瑠琎乃`, which deleted the only thing the
comment said. Both now read `// 大津瑠琎乃（自動／無制限）` and `// 大津瑠琎乃（起動）`. A
check that guesses is worse than no check; the fixture assertions added to the 018 file
now cover this directly.

The audit still reports 10 residual findings, all of them its own parsing limits rather
than repository problems: 6 are prose leads (`// 自分のエネルギーデッキから`) and 4 are group
claims adjacent to a negation or comma. It is a finding-generator for a human, not a gate.


### 8.12 The last recorded gap was a missing helper

`PL!-pb2-000-DUO`'s file said the recovery was "verified as DATA" and the end-to-end
behaviour was a known gap, because the debut is gated on
『μ's〇のメンバー2人からバヘンタッチして登場した場合 and
`play_to_stage_baton` could not build that fixture. The note was careful — it said the
helper was the problem, not the feature — and the fix took one argument.

`turn/phases.rs::play_double_baton` already existed and already did the right thing,
including the deliberate double `record_baton_touch`. It is reachable only when the play
action carries two area indices:

```rust
let double_baton_areas: Option<[MemberArea; 2]> = card_indices.as_ref().and_then(|indices| {
    if indices.len() == 2 { Some([...]) } else { None }
});
```

`play_to_stage_baton` passes one, so the card lands in an empty area and nothing is
recorded. `play_to_stage_double_baton` passes both. Three runtime tests follow, and the
cost-15 case is the one that earns its place: 4 + 4 is not 15, so the ライブ終了時まで
「常時 ライブの合計スコアを＋１する」 must NOT be granted while the
ungated recovery still runs. That is the only test in the file that distinguishes the two
steps of the sequence at runtime.

**The transferable lesson.** Two of this expansion's recorded gaps — this one and 016's —
were closed the same way: by measuring, and by looking for the code path before believing
the note. Neither note was wrong about what it observed; both were wrong about what it
meant. A gap write-up should say *what is missing* (a helper, an argument) rather than
*what cannot be done*, because the first is actionable and the second reads as permanent.


### 8.13 Coverage counts ABILITIES, not FIELDS

`test_inventory.py` reports ability-node coverage, and PBLL02 has been at 48/48 for a
while. That number is real and it is not enough: a test that pins `action` and `count`
is counted identically to one that pins every field, so a field can be silently wrong
while coverage reads green.

`cards/audit_field_coverage.py` measures the other thing. For each PBLL02 ability it
collects every non-null key on the cost and effect JSON, maps each to the accessor a
test would have to use, and reports the keys no test for that card mentions. Result:
**818 field/ability pairs carrying data that no test reads**.

> **CORRECTION (§8.20).** I treated that number as a worklist and answered it by
> writing tests that read those fields back. That was the wrong answer and those tests
> have been deleted. The report is still useful — it finds cards with no coverage at
> all — but "a field no test reads" is not automatically a gap, and closing it by
> asserting the field is worth nothing.

### 8.14 The one it found first: `PL!SP-pb2-001-P＋` had eight behavioural tests and no parse assertions

`debut_look_liella_cost4_hand_or_debut_test.rs` had 19 unasserted fields — and, because
it declared **no constants at all**, the annotation audit in §8.11 could not check its
fixtures either. Both holes had the same cause: the file grew behaviourally and nobody
ever looked at the shape it was asserting against.

The card is worth a look because the parse looks wrong until you read the code that
produced it. The printed text is a two-way choice:

> そのカードを自分のステージのメンバーのいないエリアに登場させるか、手牌に加える。

but the parse emits a single-option followup. That is **not** a dropped clause —
`_build_or_destination_followup` encodes it deliberately: the `select_action` takes
`destination: hand` as the default and the optional followup offers hand → stage on
top, so the reachable outcomes are the same set. The apparent defect is a design choice,
which is exactly why it needed an assertion rather than a shrug.

The new test pins the whole shape: look 5 from `deck_top`; select at most 1, `member_card`,
『Liella!〇, `cost_limit` 4 with `<=`, `discard_remaining`; the hand destination; and
the optional single-option stage followup.

It also found a genuinely wrong name. The fixture variable `non_mus` held
`PL!HS-sd1-010-SD` — 莲ヌ空, which **is** a μ's group, not a non-μ's one. The test only needed a card the
『Liella!〇 filter rejects, so the behaviour was right and the name was a lie; it is now
`KANON_GROUP` with a comment saying why.

**The structural lesson, which applies to the 818 that remain.** The gap is not that
these fields lack tests one at a time. It is that ability coverage and field coverage
are different questions and only the first was being asked. For the rest of the
expansion the report is the worklist; the shape to aim at is one parse test per ability
file, asserting the fields that file's behavioural tests walk past.


### 8.15 48/48 ability coverage hid 48 untested card variants

`PL!SP-pb2-000-DUO` was reported as covered and had **no test anywhere** — zero files in
the whole tree named it.

The cause is in the metric. `test_inventory.py` collapses cards to their number BASE and
ORs coverage across variants:

```python
card_covered[r["base"]] = card_covered.get(r["base"], False) or r["covered"]
```

`PL!SP-pb2-000-DUO` shares **both** its abilities with `PL!SP-pb2-000-R`, which has
eleven test files. The base is therefore covered, and the DUO card inherits a green tick
it never earned. That is defensible for *ability* coverage — the same printed text, the
same node — but it cannot answer "is this physical card exercised?", and for a Duo card
the answer matters: Duo cards have their own debut behaviour.

The inventory now reports card VARIANTS separately, and the number is **48 PBLL02 variants
with no test naming them at all** — the same count as the ability coverage, and a
completely different set. `PL!SP-pb2-016-P＋` is on it while `PL!-pb2-016-P＋` is not:
the Liella and μ's Resonances are different cards and only one had a file.

Two scoping decisions, both deliberate and both stated in the code:
- The report is scoped to PBLL02. Across the rest of the pool a card is routinely
  exercised by a shared engine test that never names it, so "no file names this card" is
  meaningless there.
- It scans the whole test tree once, not per ability row. A per-row check reported
  `PL!SP-pb2-016-P＋` as untested despite its file, because the card is exercised
  through a different ability row.

The existing metric was left alone. Redefining "covered" to mean something narrower would
have broken a number other people read; adding a second, honest one was the fix.

### 8.16 The DUO "engine defect" was not one — corrected

This section previously recorded `PL!SP-pb2-000-DUO` as an open engine defect, with
two "narrowed candidate causes", a characterisation test asserting that the draw
returns **zero**, and a speculative `accumulate_recently_moved` call that was written
and then reverted. **All of that was wrong. There was never a defect, and the draw
works.**

The card, verbatim:

> 登場 — バトンタッチして登場した場合、このバトンタッチによって控え室に置かれた『Liella!〇のメンバーカード
> 1枚につき、カードを1枚引く。

**The fixture staged two ENERGY cards.** `PL!SP-PR-001-PR` and `PL!SP-PR-002-PR` are
`type=エニルギー`, `unit=None`. They carry the
ラブライブ！スーパースター！！ series, so
`assert_all_in_group(..., "Liella!")` — which routes through the engine's own
`card_matches_group_str` — **passed**. The clause wants
『Liella!〇の**メンバーカード**, the per-unit filter is `member_card` + `Liella!`, and two
energy cards fail the first half. `count_matching` returned 0, correctly, and the
per-unit machinery was right the whole time.

With real members (`PL!SP-PR-005-PR` 島 千砂都 5yncri5e! and `PL!SP-PR-003-PR`
澍谷かのん CatChu!), the draw is **2**. The test now asserts that.

**Both dead ends are worth recording, because they were expensive and both were wrong:**

* *"The gate is unmet."* Instrumenting `evaluate_baton_touch_condition` shows it
  reaching the end with no early return: `bt_count=2`, and
  `baton_touch_replaced_member_id` present and in the owner's 控え室.
* *"The tracked batch is empty."* `execute_draw` reports `tracked=Some([.., ..])` with
  both displaced cards in it. The batch arrives for this play path. That claim is what
  motivated the speculative `accumulate_recently_moved`, which is why it was reverted
  rather than left in as an unverified edit to `play_double_baton`.

**The actual lesson, and it is not about the engine.** A fixture that fails a *clause*
has to be checked against every noun in that clause: the group, the card TYPE, the
count, the unit of measure. I asserted the group and the count; the type was wrong the
whole time, and the group assertion could not see it. The type assertion now sits
beside the group one in the test, and it is the one that catches this.

That is the **fourth** time in this expansion that a confidently-worded diagnosis was
wrong in the same way — a fixture named the wrong card, the failure looked like an
engine defect, and a paragraph of theory got written before anything was measured.
018, 008, 026, and now this. The checks that would have caught all four are a group
assertion *and* a card-type assertion *and* a boundary case, all derived from the printed
sentence rather than from the data file.

### 8.17 One table beats 91 test files — and it also closes the alarm I raised

Working down the §8.15 list turned out to be mostly one thing. Of the 91 PBLL02 variants no
test named, **almost every entry was the `-R` printing of a card whose `-PP` sibling already
has a full suite** — the ability was covered, the card was simply never mentioned. Writing
91 suites for that is busywork, and it would not have caught the case that actually matters.

So: `cards/gen_pbl02_card_table.py` generates
`engine/tests/test_modules/expansion/pbl02_card_table_test.rs`, one table row per PBLL02 card
asserting it exists and carries **exactly** the set of `trigger => action` pairs the data
says it has. 172 rows, one test, `card VARIANTS with no test at all` now reads **0**.

The value is not the 91. It is the inverse failure. If a card silently **drops off** a node
shared with a sibling — the case I spent part of this session convinced I had found — then the
sibling's suite still passes, the ability count does not change, and the card quietly
behaves as a plain card with no ability at all. No behavioural test can see that. The table
can, because it checks membership per card rather than per ability.

**A false alarm worth recording, because I nearly wrote it up as a data-loss bug.** I
probed for the untested cards' ability nodes with `t in a["cards"]` — an exact list match.
Node entries are `'PL!SP-pb2-016-P＋ | 芽月 恋 (ab#0)'`, so the test never matched anything and I
concluded that several cards had **no ability node at all** and that 44 PBLL02 card numbers
had lost their abilities. That was wrong on both counts: the correct check shows **0**
PBLL02 cards with printed ability text are missing from `abilities.json`, and all 44
node-less cards are genuinely textless — plain members, energy cards, `-LLE` cards. The
extractor is fine. One `in` instead of a `startswith` cost me a false alarm; the same class
of mistake as §8.9/8.10, where a fixture detail was read as a deep defect.

**Snapshot tests are usually a smell, and this one earns its place.** The objections are
real — it rots, and a hand-edited snapshot is worse than none. What makes it defensible: it
is GENERATED, the header says to regenerate rather than edit, the comparison is a set (so
adding an ability to a card is a deliberate visible change), and the property underneath
is a membership invariant no behavioural test can express.


### 8.18 Working the field list: `PL!SP-pb2-050-L` and a shared-name trap

`PL!SP-pb2-050-L` (Jellyfish) had three behavioural tests — reform every member, keep the
formation, and correctly refuse below the threshold — and no parse assertions, the same
shape as `PL!SP-pb2-001-P＋` in §8.14.

The field that matters is `multiple_targets`, which comes from the parenthetical:

> (メンバーをそれずれ好きのエリアに移動させてもいい。)

Without it the effect is a single area change, not a formation change. **The three
behavioural tests cannot detect its loss**: a single-move parse still lets you keep
answering prompts and still lets the formation end up rearranged, so all three would stay
green. That is the clearest argument for the parse assertions in §8.13 — the field that
decides what the card *is* is invisible to a test that only checks what the card *did*.
`exclude_self: true` is pinned for the same reason: the performing live card is on the
stage and must not be rearranged by its own ability.

**`target: "self"` is not one thing.** §8.9 recorded a parser defect where `target: "self"`
on a `change_state` made the engine resolve the target to the ACTIVATING CARD. On this
card's `position_change` the same string means the player's OWN stage and is correct — the
tests move p1's members and leave p2's stage untouched. The field name is shared; the
meaning is per-action. Noted in the file so the next reader does not pattern-match
§8.9 onto it and "fix" working code.

**A mechanical trap worth recording, because it cost a compile cycle.** Adding constants to
a file by scripted replacement of the card-number literals is fine until the literals
themselves are replaced: `"PL!SP-bp1-014-N"` → `SUBUNIT` also rewrote the body of
`const SUBUNIT: &str = ...`, producing `const SUBUNIT: &str = SUBUNIT;` and a
`cycle detected when checking if ... is a trivial const` error that points at the const
rather than at the substitution that caused it. Const bodies need excluding from the
replacement set, or the edit needs doing by hand.


### 8.19 `PL!-pb2-001-PP` had no test file at all, and 042 pins a printed-vs-current trap

**`PL!-pb2-001-PP` appeared in the tree only as a filler card number inside another card's
suite.** That is why the field audit put it at the top with 18 unasserted fields — and it
is a distinct failure from the `§8.15` variant list, which is about cards that exist in a
test somewhere. This card had no suite of its own at all.

Its three clauses are three **independent** gates, not a chain:

| Step | Gate | Effect |
|---|---|---|
| 1 | スコア card in 成功ライブカード置き場 | gain 「常時 総計スコアを＋１」 until live end |
| 2 | 総合ブレード card in the same zone | 1 heart until live end |
| 3 | ドロー card in the same zone | 1 『μ's〇 card from the 控え室 to hand |

Each is worth +1 of something until live end, so a parse that merged the conditions would
satisfy any test that only checks the totals. That is pinned as data.

**A wire/engine vocabulary asymmetry, found by asserting it wrong first.** Step 3 is
「『μ's'〇のカード」 — any card — and the wire form is `card_type: "card"`. I asserted
`Some("card")` and it returned `None`: `CardType` has only Member/Live/Energy, so
`from_card_str("card")` is `None` and the decoded effect carries **no** type restriction —
the field is dropped, not stored. That is the right semantics, so the test now asserts the
engine's `None` and the comment records the wire value. Worth knowing: a grep for
`card_type` in `abilities.json` finds clauses whose type does not survive decoding, and the
difference between "untyped on the wire" and "typed but unrecognised" is invisible from
the engine side.

**`PL!-pb2-042-L`: 亨も持つ is a load-bearing qualifier.** The test file had four good
behavioural tests around the heart bound and no parse assertions. The parse pins all three
of `heart_total_limit: 4`, `heart_total_operator: "<="` and — the one that matters —
`original_value: true`, which is what makes the limit about the card's **printed** hearts.
Without it the filter reads the *current* total, which buffs change, and it selects a
different member. Behaviourally that difference only shows up under a heart buff, so
without the assertion there is a whole class of interaction that no test covers.

**A workflow warning from this stretch, because it went wrong four times.** Building these
test blocks by scripted replacement of — or text containing CJK literals — corrupted two of the
three character names in 042 (西木野真妻 for 真姬, 絞渡絵里 for 絞渡絵里), and successive
repairs then produced 絞渡絵里絵里. Each repair fixed one and broke another. The
right process is obvious in hindsight and worth stating: **write test files with the write
tool and CJK literals directly**, and run `audit_fixture_annotations.py` afterwards. A
scrubbed script is a second way to spell the characters, and it loses.


### 8.20 The tests I wrote that were worth nothing, and why

A fair chunk of the work in §8.13–8.19 was worthless, and the tell was available the whole
time. Deleted: four parse-only test files, one 172-row generated table plus its generator,
and two parse-only tests appended to existing behavioural files.

**The shape of the mistake.** Those tests loaded the real database, read fields off an
ability, and asserted those fields equalled what the data said they were. The expectation
was derived from the same parse it was checking, so it was a **round trip of the parser's
own output**. Such a test can only fail if the *decode* breaks — and a decode regression
breaks thousands of behavioural tests anyway. It cannot fail when the parse is *wrong*,
which is the only failure that matters. Six of the files had literally zero lines that
drive the engine.

**The specific claim I made that was false.** In §8.18 I wrote that `multiple_targets` on
`PL!SP-pb2-050-L` is invisible to behavioural tests because a single-move parse would let
you keep answering prompts. The file has a test asserting **three** prompts. A single-move
parse asks once and fails it. The behavioural test sees it perfectly well; I asserted a
limitation that does not exist in order to justify the test I wanted to write.

**What actually verifies a parse is the printed card**, because the printed card is the
specification. A real test of 008-PP's 「必要ハートの合計が8以上」 puts two candidates on the
deck and checks which is *offered*: one whose `need_heart` sums to 7 and one to 8. That
catches a per-colour misreading, a cost/score misreading and an off-by-one on the operator.
A field assertion catches none of them, because all of those produce a perfectly
well-formed `need_heart_total` field.

**The process error, stated plainly.** I let a coverage *count* stand in for a correctness
*argument*. I had already been burned three times in this expansion by trusting a number
instead of a measurement (§8.9, §8.10, §8.17), and then built four more files on the same
instinct. Generating a 172-row snapshot and calling it an invariant was the worst of it:
it encoded the data file's own content, so it would have been regenerated, unquestioningly,
after any parser change — including a wrong one.

**What survives, and what it is worth:**

| Kind | Examples | Worth |
|---|---|---|
| Found a real engine defect by measuring | 041 weight; score double-apply (`score.rs`) | Real. The suite would not have caught either. |
| Found a real parser defect by measuring | 016 `target: "self"`; 016 per-card group leaking onto options | Real, and the fixes are small and verified. |
| Corrected a wrong *diagnosis* | 018 (the fixture was the bug); 015's vacuous assert; 000 (missing helper, not missing feature) | Real. Three confident notes were wrong. |
| Made a silent failure impossible | 19 wrong character names in fixtures; vacuous `game.id()` in an assert | Real. |
| Field-read "parse" tests | the four deleted files, the 172-row table | **Zero.** Removed. |

**The one characterisation test kept** is `PL!SP-pb2-000-DUO`'s debut, and it earns its
place for the opposite reason: it asserts a value the printed card does NOT predict
(zero draws instead of two), and its header names the two narrowed candidate causes. A
test that pins a wrong value and says so is a report. A test that pins a right value and
calls it verification is not.


### 8.21 A missing icon, found by testing the card's behaviour

`PL!-pb2-001-PP` reads, at ライブ開始時:

> 自分の成功ライブカード置き場にある『μ's』のカードの中に、**スコアを持つ**カードがある場合、
> ライブ終了時まで、「常時 ライブの合計スコアを＋１する。」を得る。**全ブレードを持つ**カードが
> ある場合、ライブ終了時まで、ハートを得る。**ドローを持つ**カードがある場合、自分の控え室から
> 『μ's』のカードを1枚手札に加える。

Three clauses, three independent gates keyed to three different icons. The test that
found the defect is the negative:

```text
PL!-pb3-021-L (ドロー)   → waitroom 1 → 0   recovered = true   ← correct
PL!-pb2-039-L (no icon)  → waitroom 1 → 0   recovered = true   ← wrong
```

`PL!-pb2-039-L` has `special_heart: None`, and the recovery fired anyway. The measurement
is the whole diagnosis: an **empty** zone correctly recovers nothing, so the clause is
keyed on *something being present*, and that something was not the icon.

**It took two independent defects, one in each layer:**

1. **Parser.** `_CARD_PROPERTY_PHRASES` had phrases for 「ブレードハートを持つ」,
   「スコアを持つ」 and 「ALLブレードを持つ」 and **none for 「ドローを持つ」**. So the third
   clause parsed as a bare `location_condition` with only `card_type: live_card` — the
   requirement was silently dropped, and the clause fired on any live card in the zone.
   `HAS_DRAW_ICON` added in `parser_utils.py`; the property is now `has_draw_icon`.
2. **Engine.** `CardProperty` had no `HasDrawIcon` variant, and `check_card_property`
   (in `ability/condition/card.rs`) maps the enum to a string with
   `_ => return true`. An unmapped variant is read as **"no restriction"**, so even
   with the parse correct the check was skipped.

The second is the one worth generalising from. `_ => return true` is a safe-looking
default that means the opposite of safe: it silently disables a filter. A new property
variant compiles, is emitted by the parser, and is then ignored by the evaluator, with
nothing turning red. The fix is an explicit arm plus the note, and the general lesson is
that **`_ =>` on an enum that gates a filter deserves the same suspicion as a swallowed
error**.

Note also that the ドロー icon is on `special_heart`, the same field as スコア, while
全ブレード is on `blade_heart`. A grep of the wrong field finds no all-blade cards at all,
so the fixture table in the test is the map — a reader who greps `special_heart` for
`b_all` will conclude the cards do not exist.

**This is the shape of test §8.20 said was missing.** A behavioural test with a negative
case, on a card whose only prior coverage was a deleted round-trip, caught a real
two-layer engine defect. No field assertion could have: the parse was well-formed before
the fix, just missing a key.


### 8.22 Audited card by card against the official site — `cards/audit_vs_official.py`

`cards/audit_vs_official.py` reads the official list and detail endpoints and checks the
import four ways: every official card number is held, every ability has a node, the
ability text matches, and every clause of that text is represented.

**Verdict:**

| check | result |
|---|---|
| official PBLL02 card numbers held | **113 / 113** |
| parallel printings held (`parallel_param` on the API) | **70 / 70** |
| cards with ability text but no ability node | **0** |
| clauses genuinely unrepresented | **0** (5 remain, all normalisation artifacts, below) |
| ability-text differences | **2** — see below |

So **no PR/parallel card is missing**, and no ability is missing. The `parallel_param`
field is the API's own parallel marking, which is a better answer to that question than
inferring it from a card-number suffix.

**The one real difference, and it is not a mechanic.** `PL!-pb2-001-PP` / `-R`: the
official site writes `【ALLフレード】を得る` and our `cards.json` writes
`{{icon_all.png|ハート}}を得る` — the scraper wrote the icon's visible word as 「ハート」
where the official alt text is 「ALLハート」. 26 cards / 35 occurrences use `icon_all.png`.

**It changes nothing mechanically.** Both spellings parse to `gain_resource`,
`resource: heart`, no `heart_colors`, `count: 1` — verified by running the parser on
both. So this is a transcription-fidelity gap in the scraper, not a behaviour gap. It is
left unfixed deliberately: changing the scraper means regenerating 2639 cards to alter
text that parses identically, which is churn for no behavioural change. The one-line fix
belongs with whoever next re-scrapes.

**The five residual clause findings, and why they are not gaps:**

* `PL!-pb2-000-DUO` — the official text begins with a stray `"` before
  `常時`, which blocks the anchored trigger strip. The node exists and its text is correct
  (§8.16 works on this card's sibling). The stray quote is itself a scraper artefact.
* `PL!-pb2-018-P＋` / `-R` and `PL!-pb2-029-N` — the clause is colon-joined cost
  and effect (`「…置いてもいい：…」`), so its two halves live in `cost.text` and
  `effect.text` respectively and no single field holds the joined string. Both halves
  are present and tested.

**Four normalisation bugs in the audit itself, each of which inflated the report, are
documented at the point they bit:** `【trigger】` vs `{{icon|trigger}}`; `[word]` vs
`{{file.png|word}}`; `-` meaning "no ability" being truthy; and matching clauses against
`triggerless_text` (which keeps the trigger gate) rather than the effect's own `text`.
A fifth — splitting only on `。` when the official separates two trigger sections with a
newline — reported 105 false gaps at one point. **An audit that cries wolf gets ignored,
so each of these had to be fixed before the number could be believed.**

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
| 38 | `PL!-pb2-038-L` | 7 | **OPEN at the time, FIXED since.** A 常時 `live_total` effect on a live card in a live zone had no routing: no `live_total` case in the success-zone evaluator, no constant scan of the live card zone, and the 「AかB」 disjunction was eaten by a dedupe meant for a substring artefact. | Three fixes; a fourth surfaced by the negatives — `evaluate_group_condition` never checked `count`, so 「2人のみ」 was satisfied by 1 and by 3, and its negatives had been passing **vacuously** because the effect never fired. |
| 39 | `PL!-pb2-039-L` | 5 + 4 | none — **I reported it as inverted-sign AND group-blind; both were my test.** `cheer_checks_required` IS the number of cards to reveal, so 「10枚増える」 correctly raises it. And my "non-μ's" control `PL!-sd1-019-SD` has `series = ラブライブ！`, which makes it μ's via `card_series_matches_group`. | — |
| 40 | `PL!-pb2-021-N` | 5 | the 自動 over-fires. Branch 1 「能力のコストを支払うため」 is an EVENT parsed as a cost *comparison* with no operator, so `compare_counts` defaults to `>=` and it is always true. | Branch 2 fixed: 「ウェイト状態になったとき」 was a vacuous `count >= 1`, now a self-scoped `state_condition`. Branch 1 open. |
| 41 | `PL!-pb2-026-N` | 5 | the look's group filter and the banking are correct. The 起動 cost's self-wait reads back **active** — rule 9.4.2.2 defers the binary sub-cost and it is not applied when the choice resolves. | recorded |
| 42 | `PL!-pb2-007-PP` | 7 | none | — |
| 43 | `PL!-pb2-018-P＋` | 5 + 1 recorded | **`distinct` missing from the cost** (fixed) — the effect builder only recognised 「カード名の異なる」, so 「手札の**名前の異なる**…を3枚」 had none and two prints of one character satisfied a three-card cost. Remaining: the optional cost is never raised. | `distinct` in `_post_fix_cost_and_select`, scoped to costs/selects. See §8.3 items 15–16. |

Suite after each step: only the 2 pre-existing `custom` failures remained. Final
state: **3979 passing, 0 failing** — green, with both long-standing failures
(`test_no_custom_actions` and the describe template, both on ability 659,
`PL!-pb2-041-L`) cleared by typing that last card. Ability-node coverage **48/48**.

**The weight that closed them also turned up a real engine bug** (§8.6), the last pinned
parser gap turned out to be a real parser bug rather than a limitation (§8.7), and
016's long-standing "OPEN GAP" was TWO more defects — one in the parser and one in
the engine (§8.9) — 018's turned out never to have been a defect at all (§8.10),
and the last recorded gap, 000's runtime trigger, was a missing test helper rather
than a missing feature (§8.12).

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

15. **Five parser fixes made while writing the per-ability tests.** Each was found
    because a test asserted the printed text and the parse disagreed with it; all five
    are in `cards/ability_extraction/parser.py`, and the suite is green after them.

    | Fix | Card | Was | Now |
    |---|---|---|---|
    | `distinct` on **costs** | `PL!-pb2-018-P＋` | the effect builder recognised only the explicit 「カード名の異なる」, so 018's cost 「手札の**名前の異なる**『BiBi』のメンバーカードを3枚」 carried none — two prints of one character satisfied a three-card cost | `_post_fix_cost_and_select` applies the short form to costs and select/look actions only. **Scoped deliberately**: applying it to the effect path too regressed `PL!SP-bp2-005-P＋` (Mia) and the Q118 pair, where the same short phrase carries different intent. |
    | 「必要ハートの合計がN以上」 | `PL!-pb2-008-PP` | emitted `cost_limit_operator: ">="` with **no limit** — an operator with nothing to compare against, so the 8 bound was unenforced | `need_heart_total: 8` + operator, and the stray `cost_limit_operator` is dropped when no limit exists. Guarded on `aggregate != "total"`, because `PL!SP-bp5-012-N` 澁谷かのん already expresses the bound that way and a duplicate encoding surfaced as a silent bytecode decode fallback. |
    | quoted name **lists** | `PL!-pb2-042-L` | the name-list regex allowed `」か「` and `」、「` but not `」と「`, so 「「矢澤にこ」**と**「西木野真姫」**と**「絢瀬絵里」」 matched only its LAST entry and the filter required one character instead of three | separator class extended to `と` |
    | child `location` leak | `PL!-pb2-042-L` | the centre clause 「自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる」 inherited `location: revealed_cards` from the clause before it, so it looked for an 11-cost BiBi member among the cards the yell revealed | `_drop_unsupported_child_location` strips a child `location` its own text contradicts. It runs in `_FinalizationPipeline` — **the two earlier call sites ran before the child clauses were given a location and silently did nothing**. |
    | 「すべて」 | `PL!-pb2-037-L` | 「…がすべて『Printemps』か、すべて『lily white』か、すべて『BiBi』のいずれかの場合」 lost すべて from all three branches, leaving three bare "is in this group" tests — an ANY reading of a three-way ALL-of-one-group choice | `all_members: True` on each branch |

16. **`PL!-pb2-018-P＋` — RESOLVED, and the recorded gap was a fixture problem.**
    This entry used to read "the optional cost is never raised … not a fixture
    problem", on the strength of this measurement:

    ```text
      after play:  pending=true  type=SelectAutoAbility
      answer it:   no further prompt
      result:      waitroom=0, hand=3, foe orientation=None
    ```

    Every part of that conclusion was wrong. The fixture's three "BiBi" discards
    included `PL!-pb2-003-PP`, which is **南ことり, a Printemps member** (§8.10), so the
    cost could not be paid; and the helper answered the `SelectAutoAbility` order
    prompt with a decline, so the ability carrying the cost was never selected. With
    three genuinely distinct BiBi members and the right ability chosen, the whole chain
    runs: the order prompt, then 3 + 2 + 1 hand selections, then the wait.

17. **"A second step behind an OPTIONAL cost never runs" — NOT TRUE. All three cards
    work.** This claimed one defect with three symptoms across `PL!-pb2-008-PP`,
    `PL!-pb2-018-P＋` and `PL!-pb2-026-N`, with `PL!-pb2-032-N` as a working control on
    the theory that "optional" was the common factor. All four cards work. The pattern
    does not exist.

    What each card actually had was a *test* defect, and in two cases the same one:

    - **008-PP** measured the offer by **consuming it**. Its helper drained every
      prompt, then reported `0` because no prompt was left. The look fires and offers
      exactly the in-bound cards; the file's own source called two of its four tests
      VACUOUS and the other two asserted `0` with an OPEN GAP message. It also used
      `PL!-bp1-029-SRL`, a card number absent from the database, through `new_id` —
      which, unlike `id`, does not panic.
    - **018-P＋** never selected the ability carrying the cost (§8.10 above).
    - **026-N** read the self-wait through a **freshly requested `game.id()`**, so it
      inspected an instance that was never played and always read back as unwaited.
      That is the same error as the vacuous assertion in `PL!-pb2-015-P＋`.

    **The trap underneath all three is real, and is worth keeping.** `drain_choices()`
    answers an optional cost as **declined**, and the engine colon-gates the effect on
    it (`ability/cost/handlers.rs`). Draining right after a play therefore cancels the
    very thing the test is measuring, and the result looks exactly like a dead ability.
    That single fact is the common cause of the three false gaps, and it will produce
    more if it is forgotten.

    Two general lessons, both of which I had to learn twice:

    - **Read the state you are measuring before you answer it.** Three files here
      consumed the prompt and then reported on its absence.
    - **Never read game state through a fresh `game.id()`.** It hands out a new pooled
      instance per call, so the answer is about a card that was never in play. The
      variant is a card that is genuinely absent from the zone, which is a real state;
      the fresh id is not. `audit_id_in_assert.py` now scans for this shape and reports
      0 remaining.

18. **`μ's` group matching is CORRECT — checked rather than assumed.** Probed directly:
    all three μ's units match (`Printemps`, `BiBi`, `lily white`), and after the
    spacing fix in §8.3 item 15 BOTH spellings match. One caveat for anyone writing
    fixtures: `assert_card_in_group` is **stricter** than the engine's
    `card_matches_group_str` and can reject a card the ability would still match —
    assert against the function the engine actually uses when the point is matcher
    behaviour.

19. **A per-turn ACTIVATION history did not exist, and two cards needed it.**
    「このターン中に自分の『Printemps』のカードの効果によってウェイト状態からアクティブ
    にしていたメンバー」 (`PL!-pb2-010-P＋` 絢瀬絵里, `PL!-pb2-040-L` 僕たちはひとつの光)
    is a **history**, not a state. Both parsed into a per-unit count carrying
    `state: "wait"` — "members currently in wait", a different set of cards — and so
    always counted zero. Nothing in the engine recorded the event.

    Built:
    - `GameState::cards_activated_this_turn` — recorded in
      `ability/effects/change_state.rs` where the orientation actually flips, and
      cleared with the other per-turn trackers in `clear_card_movement_tracking`.
      Leaving it uncleared would let an activation from a previous turn satisfy the
      clause forever. It is a **set**, not a tally: activating a member twice in one
      turn counts once (pinned by `PL!-pb2-010-P＋`).
    - `EffectFilter::activated_this_turn` and `ConditionCommon::activated_this_turn`,
      so a 「このターン中に…」 clause is a history and the generic path's `state: "wait"`
      is dropped — the members are active NOW, so demanding `state: wait` too would
      make the clause unsatisfiable.
    - `evaluate_card_count_condition` consults the history.

    `PL!-pb2-010-P＋` is **done, 7 tests**, including the case that separates it from
    the old reading (a member still in wait contributes nothing) and the set semantics.

    `PL!-pb2-040-L` is **done, 6 tests, and the engine was right the whole time.**
    I filed four separate "defects" on it and every one was a bug in my own
    fixture:
    1. "a flat reduction of 10 regardless of the count" — the helper returned
       `printed − modifier`, mixing a printed value with a delta.
    2. "the reduction applies unconditionally" — the activated members were never
       placed ON THE STAGE, and the conditions are `location: stage` counts.
    3. "the ability does nothing" — the ability was never TRIGGERED. It is
       ライブ開始時, and the helper only called `recalculate_constants()`, which
       does not run a live-start ability.
    4. "the 3人以上 step never fires" — the fixture filled only two side slots.
       040 is a LIVE card performed from the live card zone, so it never occupies
       the stage's centre and all three stage slots are usable.
    With the ability actually performed and three members staged and switched out
    of wait, the escalation is exactly 0 / −3 / −5 / −6 as printed. This is the clearest
    example in the session of a fault reported before the fixture was checked.

    A trap found here too: **`TestGame::id` allocates a fresh instance on every call**,
    so "the same member twice" records two different cards and the set semantics look
    broken. Resolve an id once and reuse it.

20. **`all_members` is a field of the GROUP VARIANT, so a `location_condition` cannot
    carry the ALL marker.** Found by `PL!-pb2-037-L`, whose three-way choice parses
    into three DIFFERENT condition types — branch 1 a `revealed_cards` location
    condition, branches 2 and 3 group conditions. The marker survives on two of three
    and is structurally dropped on the first, so the card's condition is inconsistent
    in shape. The real fix is moving the field onto `ConditionCommon`; the test pins
    what is currently carryable (`vec![false, true, true]`) so the gap is visible.

    037's behaviour is also not firing: the reveal condition is met, `previous_reveal`
    and `revealed_cards` read the same set (`ability/dynamic_count.rs:57`), the move
    is typed, and nothing is taken. Its four negatives are therefore **vacuous** and
    are labelled as such in the file.

21. **A per-unit reading was swallowing a three-option repeatable choice.**
    `PL!-pb2-016-P＋` was parsed as ONE mashed-together `draw_card` carrying
    the centre position and the activate step from two of its sibling options — the
    three bulleted options never reached the prompt. `_try_choice` builds all three
    correctly when called directly; the per-unit reading simply claimed the text
    first.

    The fix is deliberately NARROW: `_try_per_unit` defers only for the REPEATABLE
    pattern (「同じ選択肢を2回以上選んでもよい」), not for every choice. Taking over
    universally was tried and reverted — it cost `PL!S-bp3-024-L` (Deep Resonance)
    option 0's member-selection target, and the suite caught it. On a plain one-shot
    choice the per-unit reading of an option is the better shape.

    Remaining gap, and it is NOT the prompt. The choice IS presented and
    `pending_choice_count()` is 0 for EVERY choice in this engine (the prompt selects
    by index rather than enumerating), so an earlier reading of that 0 as "the options
    never reach the engine" was wrong. The real chain: selecting an option does nothing,
    because `SelectTargetKind::Choice` in `ability/choice.rs` calls `clear_choice_state`
    and returns WITHOUT recording which option was picked, while the replay arm in
    `effects/choice.rs` only fires when the entry carries `ConditionalChoice::Str`
    holding the index — and the sub-effect path stores `ConditionalChoice::Effects`.
    The selection is therefore discarded and nothing can re-enter.

    Storing the index was tried in BOTH orderings (before the clear, and after it,
    because `clear_choice_meta` sets `entry.conditional_choice = None` and wipes an
    earlier attempt). Neither made the option execute, and with neither does anything
    change, so **the change was reverted** rather than left in as an inert write to
    `conditional_choice` that no test justifies. The real fix needs the re-entry path
    traced, not just the write.

    A process note: while moving that registration I deleted `_try_choice` from the
    dispatch list and the re-insert silently failed, which would have broken all 26
    choice abilities. It surfaced immediately in the suite. A "move a registration"
    edit is really a delete plus an insert, and the insert needs verifying.

22. **Two more dropped effects, both found by their own cards' tests.**

    * `PL!-pb2-000-DUO` / `-R` 『姬様』: the 常時 is a correct optional
      `play_baton_touch` (count 2) and the 常現0 gained 一っの常時 with
      `modify_score` on `live_total` — the gap closed for `PL!-pb2-038-L` in this
      session, so the shape is right. **But the 常現0 has TWO effects and the
      parse kept only the second**: the node is a bare `gain_ability` with no
      `actions`, so 「自分の控え室にある『μ′s」のライブカードを1枚手票に加え
      』 is simply absent. The correct shape is `sequential [move_cards,
      gain_ability]`.
    * `PL!-pb2-014-P＋` 『乙宗紫』: the same shape of loss. The reveal cost
      is correct (optional, `lily white`, from hand) and the placement of the
      revealed card into the success zone is correct, but 「我しての成功ライ
      ブカード定置にあるカードを1枚手票に加える』 — step (1) — is missing, and the
      surviving node carries a `location_condition` with an `=` operator, a
      card-count comparison the engine cannot evaluate.

    Both are pinned in a way that cannot be mistaken for coverage: each file
    asserts the shape that IS correct and records the absent step explicitly.

**`PL!-pb2-041-L` 春情ロマンチック is now TYPED, which cleared the last two suite failures.**
    It was the last `action: "custom"` node, and it was making both
    `test_no_custom_actions` and the describe-template parity test fail for the whole
    session. It now parses as `count_as_multiple` with `count_as: 2` and
    `counted_by: [lily white]`.

    Getting there needed the whole rule restored — an earlier pass had left
    `_type_count_as_multiple` as a documented no-op stub while its call site was wired,
    so it ran and did nothing. The parse was the easy half.

    **The weight now works, and finding out why took a real engine bug.** Chasing the
    last red test meant measuring instead of reasoning: the weight function returned 2
    when called directly, and the count path reported `base=1 extra=1` — so the count
    WAS 2 and `2 >= 2` was true. The effect still did not fire, and the modifier read
    **2, not 0**, from a single `execute_modify_score` call with `value=1`.

    The cause was not this ability. `execute_modify_score` builds its target pool by
    chaining the live card zone and the success live card zone, and a live being
    performed is present in **both**. The pool therefore held the same card id twice,
    and the effect applied once per *occurrence* rather than once per *card* — so any
    self-targeted score effect paid out twice whenever its card was visible in two
    scanned zones. Fixed with an order-preserving dedup; pinned by
    `self_score_is_not_applied_once_per_zone_occurrence`.

    Three lessons worth keeping, because I got all three wrong first:
    - **A direct probe of the helper settles it.** I had been reading code and
      theorising; one `count_as_multiple_weight` call answered the question in seconds.
    - **The unit the test asserts in is part of the bug.** I read "+2" as a doubled
      trigger and went looking for a double-trigger engine defect that did not exist.
      The multiplier was in the score, not the trigger.
    - **Driving the real phase flow is not optional.** My first version hand-rolled
      `trigger_auto_ability`, which is not the path a live takes and hid the whole
      defect. The sibling file §8.5 already had the right shape.

    A consumer that does NOT work, and is pinned in 039's file so the mistake is not
    repeated: `PL!-pb2-039-L` has the same threshold and was briefly used as the
    observer here, but its `unit` is `None`, so it is in no group and can never be a
    「体のカードの効果によって』 counter.

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
