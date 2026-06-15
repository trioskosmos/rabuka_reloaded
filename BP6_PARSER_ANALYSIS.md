# BP6 Parser Analysis — Field-Level Gaps & Novel Patterns

## Overview

- **Total unique abilities in abilities.json:** 762
- **BP6 unique abilities:** 94 (from 121 cards with abilities out of 161 total BP6 cards)
- **Parser success rate:** ~83% (78/94 correctly classified into structured actions)

## Field-Level Issues

44 of 94 BP6 unique abilities had at least one field-level issue.

### Category A: Cost → Effect Boundary

The `optional` flag from cost text is not propagated to the effect.

**Pattern:** `手札をN枚控え室に置いてもよい：` → cost dict has `optional: true`, but effect never sees it.

**Affected abilities (10+):**
| Ability | Card |
|---|---|
| `登場` 手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る...`μ's`のメンバーカード... | PL!-bp6-004-P |
| `登場` 手札を2枚控え室に置いてもよい：自分の控え室にある... | PL!-bp6-005-P |
| `登場` 手札を1枚控え室に置いてもよい：自分のデッキの上から...ライブカード... | PL!HS-bp6-022-N |
| `登場` このメンバーをウェイトにしてもよい：... | PL!S-bp6-018-N |
| `ライブ開始時` 手札を1枚控え室に置いてもよい：...ブレードを得る | PL!HS-bp6-004-P |
| `ライブ開始時` 手札を1枚控え室に置いてもよい：...コスト＋６... | PL!HS-bp6-005-P |
| `ライブ開始時` 手札を1枚控え室に置いてもよい：...heart05... | PL!HS-bp6-025-L |
| `ライブ成功時` `μ's`のメンバー1人をステージから控え室に置いてもよい：... | PL!-bp6-021-L |

**Root cause:** `parse_ability()` parses cost and effect independently. The cost correctly identifies `optional: true`, but nothing copies it back to the effect tree.

**Status after fix (2026-05-26):** ✅ All 12 BP6 abilities with optional costs now have `optional: true` propagated to the effect tree.

### Category B: Field Propagation in Sequential/Choice Effects

When an ability has sub-actions (sequential, conditional_on_optional, choice, look_and_select), these fields are not propagated from the parent or condition context:

#### B1. `group_names` not propagated from condition to sub-actions

**Examples:**
- `『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードを...` → condition has `group_names: ["Aqours"]`, but action part has none
- `エールにより公開された自分のカードの中に、ブレードハートを持たない『μ's』のメンバーカードがある場合、カードを1枚引き...` → condition has `group_names: ["μ's"]`, action has none
- `自分のステージにいるメンバーがすべて『Aqours』の場合、このカードのスコアを＋１し...` → group_names lost

#### B2. `heart_colors` not extracted in action text

**Examples:**
- `ライブ終了時まで、{{heart_02.png|heart02}}と{{heart_04.png|heart04}}を得る` → should have `heart_colors: ["heart02", "heart04"]`
- `{{heart_00.png|heart0}}{{heart_00.png|heart0}}減らす` → heart00 not recognized (atypical naming `heart0` vs `heart01`)

#### B3. `duration` not propagated from parent to sub-actions

**Example:** `ライブ終了時まで、` as prefix → sequential parent has `duration: "live_end"` but individual `gain_resource`/`modify_score` sub-actions don't.

#### B4. `all` flag missing

**Examples:**
- `すべての『μ's』のメンバーは` → should have `all: true`
- `自分の控え室にあるすべてのメンバーカードを...` → shuffle all members

#### B5. `shuffle` flag missing

**Example:** `自分の控え室にあるすべてのメンバーカードをシャッフルし、デッキの下に置く` → should have `shuffle: true`

### Category C: Missing Condition Linking

When `そうした場合` or `場合` splits an ability into conditional+action parts, the condition object is parsed correctly but not linked to the action. The effect becomes a flat `sequential` with no `conditional` flag.

### Category D: Parser Completely Missed

#### D1. Passive Cost Reduction (custom)

```
{{jyouji.png|常時}}手札にあるこのメンバーカードのコストは、自分のステージにいる『みらくらぱーく！』のメンバー1人につき、2少なくなる。
```

Parser gets `per_unit` fields right but action stays `custom`. Should be `modify_cost` with `operation: "subtract"`. The `_try_cost_modification` handler misses this because `コストは` is mid-text, not at the start.

#### D2. Per-Discard-Card Heart Gain (custom)

```
ライブ終了時まで、これにより控え室に置いたそれらのカードが持つハートの色1つにつき、その色のハートを1つずつ得る。
```

Should be `gain_resource` with dynamic heart color per discarded card. Parser has `per_unit_type: "discard"` but action stays `custom`.

#### D3. Null abilities (3 — not extracted at all)

All three have triggers consumed by `{{center.png|センター}}` being treated as a cost icon before trigger extraction:

1. `{{live_start.png|ライブ開始時}}{{center.png|センター}}` 自分のライブカード置き場に `µ's` のカードがある場合...
2. `{{live_start.png|ライブ開始時}}{{center.png|センター}}` 手札にあるコスト2以下の `μ's` のメンバーカード...
3. `{{jidou.png|自動}}{{turn1.png|ターン1回}}` `Aqours` のライブカードが自分のライブカード置き場から控え室に置かれたとき...

## Novel BP6 Ability Patterns (Need Tests)

### Parser Grammar Tests

| # | Pattern | Example Text | Priority |
|---|---|---|---|
| 1 | **passive_cost_reduction** | 手札にあるこのメンバーカードのコストは、`『グループ』`のメンバー1人につき、N少なくなる。 | High |
| 2 | **conditional_alternative_placement** | このカードを成功ライブカード置き場に置く場合、代わりに自分の控え室にある...を1枚置いてもよい。 | High |
| 3 | **dynamic_count_look_at** | 自分のデッキの上から、自分のステージにいるメンバーの数にNを足した数に等しい枚数見る。 | High |
| 4 | **opponent_energy_comparison** | 相手のエネルギーが自分より多い場合、このカードのスコアを＋１する。 | Medium |
| 5 | **ability_negation_filter** | 能力を持たない `『グループ』` のカード / 常時能力を持つ `『グループ』` のカード | High |
| 6 | **choice_gain_ability** | 以下から1つを選ぶ。・このカードは「`{trigger}` カードを1枚引く。」を得る。 | High |
| 7 | **position_change_on_resolve** | `『グループ』`のメンバーの `{trigger}` 能力が解決したとき、そのメンバーをポジションチェンジする。 | Medium |
| 8 | **delayed_restriction** | このメンバーをウェイトにし、次のターンのアクティブフェイズにアクティブしない。 | High |
| 9 | **extra_yell_per_unit** | エールにより公開された...をN枚まで控え室に置いてもよい。置いた数に等しい枚数のエールを追加で行う。 | High |
| 10 | **opponent_self_target** | 相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。 | Medium |
| 11 | **slash_dual_trigger** | `{trigger1}/{trigger2}` 相手のステージにいる... | Medium |
| 12 | **cost_total_condition** | コストの合計がN以上の場合、... | High |
| 13 | **surplus_heart_condition** | このターン、自分が余剰ハートをNつ以上持っている場合、... | High |
| 14 | **deck_top_or_bottom** | そのライブカードをデッキの一番上か一番下に置いてもよい。 | High |
| 15 | **same_area_appear** | そのメンバーがいたエリアに登場させる。 | High |
| 16 | **all_heart_gain** | `{{icon_all.png|ハート}}` (ALL-type heart resource) | Medium |
| 17 | **non_stackable_marker** | この効果は重複しない。 | Medium |
| 18 | **heart00_format** | `{{heart_00.png|heart0}}` atypical icon format | High |
| 19 | **original_value_blade_check** | 元々持つ `{{icon_blade.png|ブレード}}` の数がNつのメンバー | High |

### Field-Propagation Tests

| # | Test | Description | Priority |
|---|---|---|---|
| 20 | **optional_from_cost** | `optional: true` from cost propagates to effect tree | High |
| 21 | **group_names_propagation** | group_names from condition → all sub-actions | High |
| 22 | **heart_colors_sub_actions** | Heart icons extracted in sequential gain_resource sub-actions | High |
| 23 | **duration_propagation** | `ライブ終了時まで` on individual sub-actions (not just parent) | High |
| 24 | **all_flag_すべて** | `all: true` when text contains すべて | Medium |
| 25 | **shuffle_flag** | `shuffle: true` when text contains シャッフル | Medium |
| 26 | **count_inference** | Infer `count` from N枚/N人 in text | Medium |

---

## Parser Fixes Applied (2026-05-26)

### Changes to `parser.py`

| Fix | Description | Lines Changed |
|---|---|---|
| **passive_cost_reduction** | Extended `_try_cost_modification` to handle `少なくなる` (was only checking `減る`). Also updated `_try_per_unit` exclusion. | ~3 lines |
| **optional propagation** | In `parse_ability()`, after parsing cost and effect, copies `optional: true` from cost dict to effect tree (parent + all sub-actions). | ~10 lines |
| **group_names propagation** | In `_normalize_effect_tree._walk()`, extracts `『group』` patterns from both node text and parent context text. In `_clean_action_list`, propagates `group_names` from parent to sub-actions. | ~5 lines |
| **heart_colors extraction** | In `_walk()`, extracts heart icons from `gain_resource`/`modify_required_hearts`/`move_cards`/`select_cards` nodes. Checks own text first, then falls back to parent context. | ~15 lines |
| **shuffle propagation** | In `_walk()`, sets `shuffle: true` when `シャッフル` in text context. Propagates via `_clean_action_list`. | ~3 lines |
| **duration propagation** | Added `draw_card`, `move_cards`, `look_at` to the list of action types that receive `duration` from parent sequential. | ~2 lines |

### Results After Fixes

| Metric | Before | After |
|---|---|---|
| Passive cost reduction | `custom` | `modify_cost` / `operation: subtract` ✅ |
| `optional` from cost → effect | 0/12 | 12/12 ✅ |
| `group_names` on action nodes | 0 | 48/51 ✅ (3 remaining = null abilities) |
| `heart_colors` on gain_resource | ~3 | 11/14 ✅ (3 are on sub-actions) |
| `shuffle` on move_cards | missing | set |
| Custom actions | 2 | 1 (the per-discard-heart-gain pattern) |
| Null abilities | 3 | 3 (need fix in `extract_card_abilities.py`) |

### Remaining Issues

1. **Per-discard-heart-gain** (1 custom ability): "これにより控え室に置いたそれらのカードが持つハートの色1つにつき、その色のハートを1つずつ得る" — parser needs to detect `ハートを...得る` as `gain_resource` even when words separate the verb.

2. **3 null abilities** — All have `{{trigger}}{{center.png|センター}}` where the position icon follows the trigger. The trigger extraction in `extract_card_abilities.py` doesn't handle this. The `{{center.png}}` position icon is detected as a cost icon before triggers are found. Fix needed in `extract_card_abilities.py`.

3. **`cost_limit` in text not parsed into effect** — 2 semantic validation warnings for cards with cost limit conditions in the effect text but not extracted into the structured effect.

