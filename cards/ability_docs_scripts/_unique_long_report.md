# Unique/Long Untested Abilities — Priority Report

_Generated from `cards/abilities.json` (936 abilities) vs `engine/tests` — 250 untested, 686 covered._

> **2026-08-20 UPDATE2: Original 8 lenient fixed in e6216c69 (2383 pass, 0 hidden). Next 15 lenient below are now #0 – see §0.1. Refactored parser+engine under_member, strict zone/pending checks. See §0.**

This report surfaces **structurally novel or long/complex** untested abilities — not just the biggest generic gaps from `ABILITY_MATRIX.md`, but abilities whose _JSON shape or text length_ has **zero tested counterpart** or whose trigger/condition combo is rare.

## 0. Lenient / Bullshit Tests – Top Priority to Harden

### 0.0 DONE – 8 original (2026-08-20, commit e6216c69) now strict, 2383 pass

Found via `grep -rn "assert.*||" + "lenient|TODO|for now"` (2026-08-20). Fixed: parser `PLACEMENT_MARKERS` + `drain_under_cards_to_energy_zone` helper, `choice.rs` Stage is_select_action, `fixed_customs` direct LiveSuccess, `hanamaru` no-pending strict.

| File:line | Pattern | Why lenient | Fix (e6216c69) |
|---|---|---|---|
| `burn_energy_under_test.rs:51` | `score==before \|\| +1` | `0 under+10` should be 0, `energy_zone>=1` vs `preceding_moved` | `parser.py:PLACEMENT_MARKERS` + `drain_under_cards_to_energy_zone` + `Stage is_select_action:true` + `mfi[0]` for filtered [1]; now `assert_eq!(0)` |
| `burn_energy_under_test.rs:66,80,93,106` | `score==0\|\|1` | hides `9→0` vs `10→1` | same + `select_burn_move [0]` + `skip [skip]`; now strict |
| `fixed_customs_test.rs:70` | `has_score\|\|before==0` | hides wait count | `trigger_auto_ability LiveSuccess` + `assert_eq!(+1)` |
| `draw_one_put_bottom_debut_test.rs:457` | `!deck.contains\|\|contains` tautology | always true | split to `assert!(!deck.contains)` + `assert!(main_deck.contains)` |
| `hanamaru_bp3:101` | `TODO そのプレイヤー self` | TODO left | removed, `assert_eq!(deck.last)` + `hand+1` strict, `choice.rs SelfOrOpponent` |
| `hanamaru_bp3:122` | `Err or no effect` | lenient | now `!has_pending && hand==before` strict (engine cost check not Err for Wakana compat) |
| `energy_state:92` | `For now no score` | no assert | `trigger_daisuki_live_start LiveStart` + `assert_eq!(score 1/0)` + `active vs wait vs opponent` |
| `bytecode:132` | `Report but don't fail` | hides mismatch | `assert_eq!(has_json_cost,has_bc_cost)` |
| `position_change:219` | `For now skip TO center` | stub | replaced with real `Shiki PL!SP-bp2-008-R Center→Right` swap `μ's Right→Center` TO center must NOT gain |

**How to find automatically:** `grep -rn -E "assert!\(.*\|\|.*\)|//.*(lenient|TODO|FIXME|for now)" engine/tests --include="*.rs"` + `python cards/test_inventory.py --check`

### 0.1 NEXT 15 lenient (audit 2026-08-20 PM) — STATUS: 12/15 FIXED (commits a4179f00, 111ee89b, 7c1fae10, 69abc7eb)

| # | File:line | Status | Fix |
|---|---|---|---|
| 1 | `ayumu_azuna_test.rs` ×5 | ✅ FIXED | `assert!(has_pending)` + `zone=="energy"` + `allow_skip` strict |
| 2 | `ayumu_azuna_test.rs:149` skip | ✅ FIXED | both branches, `allow_skip` asserted, zone `energy` (not energy_zone) |
| 3 | `fixed_customs_test.rs:109` | ✅ FIXED | `assert_eq!(zone,"revealed_cards")`, `other=>panic`, manual fallback kept but asserts pending after |
| 4 | `position_change_non_optional_test.rs:433,558` | ✅ FIXED | `\|\| true` tautology → `assert!(!has_pending)` |
| 5 | `chisato_live_success_test.rs:16` | ✅ FIXED | `accept_swap_to` asserts pending + SelectTarget + dest offered (69abc7eb) |
| 6 | `pl_hs_bp6_004_test.rs:38` | ✅ FIXED | ginako strict: SelectAutoAbility asserted, ab#0 auto-pick verified via orientation wait (single legal target cost 2≤9), ab#1 hand SelectCard allow_skip, blade==2, discarded==second_ginako (a4179f00) |
| 7 | `ability_engine_fixes_test.rs` kanon/keke | ✅ FIXED | both unless_pay branches assert pending + discard select offered; keke cost+look_select asserted |
| 8 | `cards_6_thru_13_test.rs` c13 | ✅ FIXED | `trigger()` returns `offered` bool; `setup_bp6_005` mixes qualifying card so prompt IS offered; rejected cards asserted in waitroom (filter tested, not auto-skip) |
| 9 | `kinako_each_time_blade_test.rs:16` | ✅ FIXED | `accept_position_swap` asserts pending + dest offered |
| 10 | `bp7_mia_play_cost_reduction_test.rs` | ✅ ALREADY STRICT | answer_play_choice asserted by all 3 tests |
| 11 | `energy_and_member_under_test.rs` mia/sayaka | ✅ FIXED | cost energy/reveal select + live retrieval select asserted (111ee89b) |
| 12 | `rin_bp6_test.rs:131` | ✅ FIXED | skip branch asserts hand unchanged (7c1fae10) |
| 13 | `live_cards_disappear_test.rs` | ⚠️ PARTIAL | card-conservation assert exists; `eprintln` diagnostics remain (acceptable, asserts present) |
| 14 | `hanamaru_test.rs:97` | ✅ FIXED | no-live test now asserts `hand==6` (condition fails → no draw) |
| 15 | `fixed_customs_test.rs:113` zone `\|\|` | ✅ FIXED | `assert_eq!(zone,"revealed_cards")` |

**Remaining known-lenient patterns (next sweep):** `emma_test.rs:53,92,98` eprintln-only; `hanamaru_test.rs:57` eprintln (has assert after); `live_cards_disappear` diagnostics. Also engine `needs_prompt` auto-pick for single-target `change_state` (state.rs:436) — deliberate UX, tests now verify outcome via orientation modifiers instead of expecting a prompt.

**How to find next automatically:** `grep -rn "has_pending_choice" engine/tests --include="*.rs" | grep -A2 "else"` + `grep -rn "allow_skip" engine/tests --include="*.rs"` (37 hits, ~60% one-branch) + `grep -rn "eprintln" engine/tests --include="*.rs"` (127 hits, 9 tests with no co-located `assert`).

Refactor opportunities (engine): `choice.rs:3500` split into `choice/{stage,under_member,area_select}.rs`, `move_cards.rs:3400` extract `drain_under_cards_to_energy_zone` (done) + `resolve_from_*` per zone, `condition/card.rs:4000` split `card.rs`/`state.rs`, `parser.py:13000` extract `PLACEMENT_MARKERS` helper (done) + split handlers. Warnings: `karin:67 hs:40 filler` unused, `ruby:314 left`, `fixed_customs:9 dead_code`, `bp7_sd2:237 Result` – fix next.

## 0.1 Gap Matrix Recap (from `docs/ABILITY_MATRIX.md`)

Biggest uncovered trigger×action cells (untested count):
| Uncovered | Trigger | Action | Covered |
|---|---|---|---|
| 26 | 登場 | look_and_select | 31/57 |
| 25 | 常時 | gain_resource | 42/67 |
| 24 | ライブ開始時 | sequential | 58/82 |
| 21 | 登場 | sequential | 41/62 |
| 19 | ライブ開始時 | gain_resource | 41/60 |
| 14 | 登場 | move_cards | 42/56 |
| 11 | ライブ成功時 | sequential | 13/24 |
| 11 | ライブ成功時 | move_cards | 29/40 |
| 10 | ライブ開始時 | modify_score | 22/32 |
| 9 | 起動 | move_cards | 29/38 |

> But these cells hide _inner uniqueness_: e.g. `登場 look_and_select` 26/57 uncovered contains 15 different JSON shapes, some with cost=pay_energy, some with `sequential` nested conditionals.

## 1. TOP 15 Longest Texts (character complexity)

`chars` = full_text length, `jLen` = effect JSON length, `shape` = top-level effect keys. These are the most branching texts; tested suite rarely covers this length/complexity.

| # | idx | chars | jLen | trigger | action | cond | sample card | text (160) | shape |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 712 | 265 | 723 | ライブ開始時 | gain_resource | location_condition | `PL!N-bp5-015-N` | {{live_start.png/ライブ開始時}}自分のステージにいるメンバーが持つハートの中に{{heart_01.png/heart01}}、{{heart_02.png/he | `action,condition,count,duration,resource,text` |
| 2 | 743 | 220 | 1018 | ライブ開始時 | choice | comparison_condition | `PL!HS-bp5-022-L` | {{live_start.png/ライブ開始時}}{{icon_energy.png/E}}{{icon_energy.png/E}}支払ってもよい：自分のステージにコスト9以上の | `action,condition,count,group_names,heart_colors,options,text` |
| 3 | 717 | 218 | 890 | ライブ開始時 | sequential | comparison_condition | `PL!N-bp5-028-L` | {{live_start.png/ライブ開始時}}自分のステージに{{heart_02.png/heart02}}を4つ以上持つメンバーがいる場合、このカードのスコアを＋２し、必要 | `action,actions,condition,heart_colors,text` |
| 4 | 867 | 218 | 1294 | ライブ開始時 | sequential | none | `PL!SP-pb2-048-L` | {{live_start.png/ライブ開始時}}自分のステージにいる名前の異なる『CatChu!』のメンバー1人につき、このカードの必要ハートを{{heart_00.png/he | `action,actions,distinct,group_names,heart_colors,text` |
| 5 | 464 | 214 | 1021 | ライブ開始時 | gain_ability | compound | `PL!S-bp6-007-R` | {{live_start.png/ライブ開始時}}{{icon_energy.png/E}}{{icon_energy.png/E}}支払うか手札を2枚控え室に置いてもよい：自分の | `ability_gain,ability_gain_trigger,action,card_type,condition` |
| 6 | 461 | 210 | 801 | 起動 | sequential | none | `PL!S-bp6-003-R` | {{kidou.png/起動}}{{turn1.png/ターン1回}}{{icon_energy.png/E}}{{icon_energy.png/E}}手札を1枚控え室に置く：こ | `action,actions,conditional,exclude_self,group_names,text` |
| 7 | 227 | 205 | 1204 | ライブ開始時 | conditional_on_result | none | `PL!HS-bp5-005-R` | {{live_start.png/ライブ開始時}}手札の『DOLLCHESTRA』のカードを1枚控え室に置いてもよい：自分のステージにいる『DOLLCHESTRA』のメンバー1人を | `action,followup_action,group_names,heart_colors,original_val` |
| 8 | 661 | 198 | 600 | ライブ開始時 | gain_resource | ability_filter_condition | `PL!-bp4-014-N` | {{live_start.png/ライブ開始時}}自分のライブ中のライブカードに、{{live_start.png/ライブ開始時}}能力も{{live_success.png/ライ | `action,card_type,condition,count,duration,exclude_self,resou` |
| 9 | 747 | 189 | 1048 | ライブ開始時 | choice | group_condition | `PL!-bp5-024-L` | {{live_start.png/ライブ開始時}}自分のステージに『A-RISE』のメンバーがいる場合、以下から1つを選ぶ。 ・ウェイト状態のメンバー1人をアクティブにし、ライブ終 | `action,condition,count,options,original_value,text` |
| 10 | 621 | 171 | 723 | 登場 | look_and_select | none | `PL!S-pb1-013-N` | {{toujyou.png/登場}}手札を1枚控え室に置いてもよい：自分のデッキの上からカードを4枚見る。その中からハートに{{heart_04.png/heart04}}を2個以 | `action,heart_colors,look_action,optional,select_action,text` |
| 11 | 622 | 171 | 723 | 登場 | look_and_select | none | `PL!S-pb1-014-N` | {{toujyou.png/登場}}手札を1枚控え室に置いてもよい：自分のデッキの上からカードを4枚見る。その中からハートに{{heart_02.png/heart02}}を2個以 | `action,heart_colors,look_action,optional,select_action,text` |
| 12 | 818 | 170 | 263 | 起動 | move_cards | none | `PL!HS-bp6-016-R` | {{kidou.png/起動}}{{turn1.png/ターン1回}}{{icon_energy.png/E}}{{icon_energy.png/E}}{{icon_energy | `action,card_type,cost_limit,cost_limit_operator,count,destin` |
| 13 | 344 | 169 | 1013 | 起動 | sequential | none | `PL!-pb1-007-R` | {{kidou.png/起動}}{{turn1.png/ターン1回}}手札を3枚控え室に置く：自分のステージにほかの『lilywhite』のメンバーがいる場合、自分の控え室から『μ | `action,actions,exclude_self,group_names,text` |
| 14 | 730 | 168 | 655 | ライブ開始時 | sequential | none | `PL!SP-bp5-024-L` | {{live_start.png/ライブ開始時}}{{heart_01.png/heart01}}か{{heart_02.png/heart02}}か{{heart_06.png/ | `action,actions,all,heart_colors,text` |
| 15 | 786 | 163 | 1174 | ライブ開始時 | sequential | none | `PL!HS-pb1-029-L` | {{live_start.png/ライブ開始時}}自分のステージに、元々持つハートの数より多い数のハートを持つ『みらくらぱーく！』のメンバーが1人以上いる場合、カードを1枚引く。2 | `action,actions,group_names,heart_colors,original_value,text` |

**Why these matter vs tested:**
- Median tested full_text length = ~70 chars; these are 150-265 chars (2-3×). Tested `sequential` usually 2 steps with simple `draw + discard`; these have 3+ steps with _conditional branches inside sequential_ (e.g. idx 717: `if have 4x heart02 then score+2 and need-3 hearts` is a _double sequential modification_ not seen in tested `sequential`).
- 7 of top 15 have `sequential` or `choice` with `cost=pay_energy OR discard` — tested covers `pay_energy` alone, but not the `pay_energy OR discard 2` choice cost (idx 461 is pay 2E + discard 1 → move from stage to hand + fetch from discard scaled by cost).

## 2. Completely Uncovered Mechanics (0% cells — no tested path exists)

| Mechanic | idx | card | trigger | text | why novel vs tested |
|---|---|---|---|---|---|---|
| energy_state_condition | 855 | `PL!SP-pb2-026-N` | 常時 | {{jyouji.png/常時}}自分のアクティブ状態のエネルギーがあるかぎり、{{heart_02.png/heart02}}{{heart_02.png/h | Tested `gain_resource` with `card_count_condition`/`comparison`, but **zero** tested with `energy_state_condition`. Engine's `is_active` check never exercised. |
| energy_state_condition | 389 | `PL!SP-bp4-028-L` | ライブ開始時 | {{live_start.png/ライブ開始時}}アクティブ状態の自分のエネルギーがある場合、このカードのスコアを＋１する。 | Same family — `modify_score` gated on active energy. No tested `modify_score` uses energy_state. |
| choose_target_player | 325 | `PL!S-bp3-007-R` | 起動 | {{kidou.png/起動}}{{turn1.png/ターン1回}}{{icon_energy.png/E}}：自分か相手を選ぶ。自分は、そのプレイヤーの控え | Tested `choose_target_player` only for `ライブ成功時 draw`. This is `起動 + pay E + target_player + move_cards(player-targeted) + conditional_on_result(draw)`. No other ability targets _opponent's discard → deck bottom_. |
| gain_ability + compound | 238 | `PL!-PR-020-PR` | ライブ開始時 | {{live_start.png/ライブ開始時}}{{center.png/センター}}自分のライブカード置き場にあるライブカードのスコアの合計が８以上の場合、 | Tested `gain_ability` exists (9/11 covered) but this is `gain_ability` with `ability_gain_trigger=常時` + `compound condition (score total)` + `duration=until_end_of_live`. Zero tested grants a _constant scoring ability_ conditionally. |
| dynamic_count | 129 | `PL!S-bp6-009-R＋` | 常時 | {{jyouji.png/常時}}相手の成功ライブカード置き場にあるカードの枚数が自分より多いかぎり、その差に等しい数の{{icon_blade.png/ブレー | Tested `gain_resource`常時は usually `if have X then +2 blades (fixed)`. This is `dynamic_count = opponent_success - self_success` — value is _variable_, not fixed. Parser key `dynamic_count` never tested. |
| conditional_on_result + move from under_member | 900 | `PL!N-bp7-029-L` | ライブ成功時 | {{live_success.png/ライブ成功時}}自分のステージにいるメンバー1人の下にあるすべてのエネルギーカードを、自分のエネルギー置き場にウェイト状態 | Tested `conditional_on_result` covers single card moves; this moves _all cards under a member_ (source=`under_member`, zone=`energy_zone`) then checks _result count + total energy 10+_. No tested covers `under_member` as source. |
| followup + set_cost by source card | 227 | `PL!HS-bp5-005-R` | ライブ開始時 | {{live_start.png/ライブ開始時}}手札の『DOLLCHESTRA』のカードを1枚控え室に置いてもよい：自分のステージにいる『DOLLCHESTR | Tested `modify_cost` etc. exist but this is `conditional_on_result` that _copies cost from discarded card_ to stage member (`original_value` from source). Tested `modify_cost` is static ±N, never dynamic copy. |

**Contrast with nearest tested:**
- Tested `PL!SP-bp4-012-N` (ライブ開始時 pay E → get heart02) covers `pay_energy → gain_resource`, but does **not** cover `pay_energy + choose` nested or `energy_state` checks.
- Tested `PL!N-bp4-021-L` (ライブ開始時 if success zone score≥6 reduce need hearts) is a simple `modify_required_hearts`; idx 717 is `sequential: modify_score AND modify_required_hearts in same ability` — a _dual effect_ not seen in tested singles.

## 3. Novel JSON Shapes — `effect` key combos with 0 tested coverage

Each `shape` = sorted top-level keys of `effect` JSON. If `tested=0`, that structure has **never been exercised**, even though `action` alone might be covered.

| Shape (keys) | # untested | example idx | card | how different from nearest tested |
|---|---|---|---|---|
| `action,activation_position,count,heart_colors,position,resource,text` | 3 | 223 | `PL!SP-bp5-011-R` | nearest tested shares 6/7 keys; extra keys={'heart_colors'} |
| `action,actions,exclude_self,group_names,text` | 2 | 329 | `PL!N-bp3-002-R` | nearest tested shares 5/5 keys; extra keys=set() |
| `action,activation_condition_parsed,activation_position,condition,count,duration,parenthetical,position,resource,text` | 2 | 684 | `PL!SP-bp4-017-N` | nearest tested shares 7/10 keys; extra keys={'activation_condition_parsed', 'count', 'parenthetical'} |
| `action,card_type,count,duration,group_names,position,resource,target,text` | 2 | 659 | `PL!-bp4-011-N` | nearest tested shares 9/9 keys; extra keys=set() |
| `action,condition,count,destination,original_value,source,text` | 2 | 482 | `PL!SP-pb2-004-R` | nearest tested shares 7/7 keys; extra keys=set() |
| `ability_gain,ability_gain_trigger,action,activation_position,condition,gained_effect,position,text` | 1 | 238 | `PL!-PR-020-PR` | nearest tested shares 6/8 keys; extra keys={'activation_position', 'position'} |
| `ability_gain,ability_gain_trigger,action,card_type,condition,count,duration,gained_effect,group_names,max,source,target,text` | 1 | 464 | `PL!S-bp6-007-R` | nearest tested shares 11/13 keys; extra keys={'duration', 'max'} |
| `action,actions,all,heart_colors,text` | 1 | 730 | `PL!SP-bp5-024-L` | nearest tested shares 5/5 keys; extra keys=set() |
| `action,actions,conditional,exclude_self,group_names,text` | 1 | 461 | `PL!S-bp6-003-R` | nearest tested shares 6/6 keys; extra keys=set() |
| `action,actions,distinct,group_names,heart_colors,text` | 1 | 867 | `PL!SP-pb2-048-L` | nearest tested shares 6/6 keys; extra keys=set() |
| `action,actions,group_names,heart_colors,original_value,text` | 1 | 786 | `PL!HS-pb1-029-L` | nearest tested shares 5/6 keys; extra keys={'original_value'} |
| `action,activation_position,blade_limit,blade_limit_operator,card_type,count,original_value,position,source,state_change,target,text` | 1 | 533 | `PL!SP-bp7-009-R` | nearest tested shares 10/12 keys; extra keys={'activation_position', 'position'} |
| `action,activation_position,condition,conditional,duration,operation,text,value` | 1 | 459 | `PL!-bp6-009-R` | nearest tested shares 7/8 keys; extra keys={'activation_position'} |
| `action,activation_position,count,heart_colors,position,position_compare,resource,text` | 1 | 532 | `PL!SP-bp7-009-R` | nearest tested shares 6/8 keys; extra keys={'heart_colors', 'position_compare'} |
| `action,all,card_type,source,state_change,target,text` | 1 | 319 | `PL!-bp3-005-R` | nearest tested shares 7/7 keys; extra keys=set() |

Highlights:
- `action,activation_position,count,heart_colors,position,resource,text` (x3, idx 223-225): 位置付き常時ハート付与 (左サイド/センター/右サイドで色が違う). Tested `gain_resource`常時は `group_condition` or `comparison` only; these have `activation_position` + `heart_colors` keys never seen together.
- `action,ability_gain_trigger,condition,gained_effect,…` (idx 238, 464): `gain_ability` that grants a _new trigger type_ (`ability_gain_trigger=常時`) — tested `gain_ability` just grants a one-time effect, not a persistent trigger.
- `action,card_type,cost_limit,cost_limit_operator,count,destination,max,source,target,text` (idx 46): Debut that fetches up to 2 members with `cost_limit <=2` — tested `move_cards` debuts fetch by `group_name`, not by cost range.

## 4. Long Sequential / Choice — deepest untested vs tested median

| idx | card | trig | steps | step actions | full text (120) | tested comparator (why different) |
|---|---|---|---|---|---|---|
| 867 | `PL!SP-pb2-048-L` | ライブ開始時 | 3 | modify_required_hearts,modify_required_h | {{live_start.png/ライブ開始時}}自分のステージにいる名前の異なる『CatChu!』のメンバー1人につき、このカードの必要ハートを{{heart_00.png/he | vs tested `PL!SP-bp2-048-L` none? tested sequential usually 2 steps (draw/discard); this is `modify_required_hearts per CatChu! count + modify_yell?` with `heart_colors` + `distinct` |
| 777 | `PL!HS-pb1-020-N` | 登場 | 2 | move_cards,sequential | {{toujyou.png/登場}}自分の控え室にライブカードが3枚以上ある場合、手札を2枚控え室に置いてもよい。そうした場合、自分の控え室から『スリーズブーケ』のメンバーカード1 |  |
| 786 | `PL!HS-pb1-029-L` | ライブ開始時 | 2 | draw_card,modify_required_hearts | {{live_start.png/ライブ開始時}}自分のステージに、元々持つハートの数より多い数のハートを持つ『みらくらぱーく！』のメンバーが1人以上いる場合、カードを1枚引く。2 | vs tested `PL!SP-pb2-047-L` sequential? This is `if have 1+ mirakurapark with extra hearts → draw; if 2+ → also reduce need hearts by 2×`. Two-stage conditional inside one ability — tested has no `card Count branching + double need reduction` |
| 884 | `PL!S-bp7-021-L` | ライブ開始時 | 3 | move_cards,draw_card,modify_score | {{live_start.png/ライブ開始時}}自分のステージにメンバーが3人以上いる場合、自分のデッキの下からカードを5枚控え室に置く。それらの中にメンバーカードが3枚以上ある |  |
| 344 | `PL!-pb1-007-R` | 起動 | 2 | move_cards,modify_cost | {{kidou.png/起動}}{{turn1.png/ターン1回}}手札を3枚控え室に置く：自分のステージにほかの『lilywhite』のメンバーがいる場合、自分の控え室から『μ | vs tested `PL!-pb1-007-R`? Wait this IS idx344 itself (untested). Its cost is `discard 3 cards` reduced by success zone count — `pay cost` variable based on zone count, tested cost is fixed |
| 667 | `PL!-bp4-021-L` | ライブ開始時 | 2 | modify_required_hearts,modify_score | {{live_start.png/ライブ開始時}}自分の成功ライブカード置き場にあるカードのスコアの合計が６以上の場合、このカードを成功させるための必要ハートを{{heart_00 |  |
| 717 | `PL!N-bp5-028-L` | ライブ開始時 | 2 | modify_score,modify_required_hearts | {{live_start.png/ライブ開始時}}自分のステージに{{heart_02.png/heart02}}を4つ以上持つメンバーがいる場合、このカードのスコアを＋２し、必要 | vs tested `PL!N-bp5-028-L` sequential? Actually this is it: `if have 4× heart02 → score+2 AND need hearts change (replace 4× heart02 with 3× heart00)`. Tested `modify_score` and `modify_required_hearts` never co-occur in same sequential |
| 538 | `PL!N-sd2-007-P` | ライブ成功時 | 2 | draw_card,sequential | {{live_success.png/ライブ成功時}}カードを1枚引く。このターン、相手もライブを成功している場合、さらにカードを1枚引き、手札を1枚控え室に置く。 |  |
| 461 | `PL!S-bp6-003-R` | 起動 | 2 | move_cards,move_cards | {{kidou.png/起動}}{{turn1.png/ターン1回}}{{icon_energy.png/E}}{{icon_energy.png/E}}手札を1枚控え室に置く：こ |  |
| 854 | `PL!SP-pb2-023-N` | 常時 | 2 | gain_resource,gain_resource | {{jyouji.png/常時}}自分のエネルギーが6枚以上あるかぎり、{{heart_02.png/heart02}}を得る。8枚以上あるかぎり、さらに{{heart_02.pn |  |
| 856 | `PL!SP-pb2-027-N` | 常時 | 2 | gain_resource,gain_resource | {{jyouji.png/常時}}自分のエネルギーが6枚以上あるかぎり、{{heart_03.png/heart03}}を得る。8枚以上あるかぎり、さらに{{heart_03.pn |  |
| 245 | `PL!N-PR-032-PR` | 登場 | 2 | move_cards,move_cards | {{toujyou.png/登場}}自分の控え室にあるカードの枚数が8枚未満の場合、自分のデッキの上からカードをその差に等しい枚数控え室に置く。その後、これにより控え室に置いたカー |  |

**Pattern:** Tested `sequential` median JSON length ~420, untested median ~720. Untested sequential often has **3+ actions** with **nested conditions per step**, while tested sequential is typically `draw → discard` or `look 3 → add 1`. For example:
- **Idx 884** (`PL!S-bp7-021-L`): `if stage has 3+ members → under-deck 5 → if those 5 contain 3+ members → draw 1; if all 5 are members → extra effect` — a _two-stage condition checking the result of the first move_. No tested does `move then inspect moved cards count`.

## 5. Recommended Priority Queue (unique first)

If you want tests that maximize _new engine paths_ per test, pick in this order (most novel first):

0. **Lenient tests above (§0) – fix `burn` `recently_moved` vs `energy_zone` and make `||` strict** – unblocks all other priorities (currently `2383` pass with `5` hidden failures).
1. **`energy_state_condition` ×2 (idx 389, 855)** — 0% covered condition type. One test proves the `active` energy check plumbing works for both `modify_score` and `gain_resource`.
2. **`choose_target_player` 起動 (idx 325 `PL!S-bp3-007-R`)** — 0/1 covered trigger×action; exercises player-targeted deck/bottom manipulation + conditional draw. Also covers `or` path with opponent's discard.
3. **`PL!HS-bp5-005-R` idx 227** — longest `conditional_on_result` with dynamic cost copy; stresses `cost source=hand → effect copies cost` path. No tested does dynamic cost transfer.
4. **`PL!N-bp7-029-L` idx 900** (`under_member` source)** — tests `source=under_member` which is unique to this card; engine must move energies _under_ a member. Also `conditional_on_result` with `count>=1 AND total>=10`.
5. **`PL!SP-bp2-048-L` idx 867** (CatChu! need-heart reduction) — tests `modify_required_hearts` per-unit with `distinct` group + sequential `modify_yell`. LNG=1294 JSON longest; exercises batched `per_unit` logic.
6. **Position-gated gain_resource (idx 684/686 left/right side)** — `activation_position` + `temporal_condition` (area_move this turn). Tested never combines position + temporal.
7. **Cost-variable activation (idx 344 `PL!-pb1-007-R`)** — `discard 3 minus success zone count`; tests variable cost computation, unlike fixed costs elsewhere.
8. **`PL!-PR-020-PR` idx 238** (`gain_ability` granting 常時)** — meta-ability; verifies engine handles _gaining a constant ability_ and its duration `until_end_of_live`.
9. **Look_and_select with heart-color filter on _both_ member and live (idx 621/622 `PL!S-pb1-013-N/014-N`)** — `look 4 → filter has heart04×2 OR need heart04≥2` — exercises OR filter inside selection, vs tested `look 5 → filter by group_name` only.
10. **`PL!S-bp6-003-R` idx 461** (起動 pay 2E + discard 1 → stage→discard then discard→hand scaled by cost+2)** — tests chained `move_cards` with `cost` → `result-dependent fetch` (cost+2 range). Stresses target selection after cost payment.

---
Appendix: total untested=250; with cost=82; sequential=67; look_and_select=32