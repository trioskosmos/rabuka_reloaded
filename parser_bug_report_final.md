# Parser Bug Analysis Report

## Summary of Findings

I found **13 categories** of parser bugs in `cards/ability_extraction/parser.py` affecting **~130 unique ability texts** (out of 602) across **hundreds of card instances** (1806 total cards, 1057 with abilities).

---

### 1. Missing `position` field on effect actions [HIGH IMPACT]

**Bug:** Actions that trigger from or reference specific stage areas (センターエリア, 左サイドエリア, 右サイドエリア) are missing the `position` field.

**Root cause:** `extract_position()` at parser.py:1092 is called but `POSITION_KEYWORDS` matching at parser.py:1467-1470 only checks the action text after effect splitting. When position keywords appear in parenthetical notes like `（この能力は左サイドエリアにいる場合のみ発動する。）`, they're stripped by `strip_parenthetical()` before position extraction.

**Example:**
- Full text: `カードを2枚引き、手札を2枚控え室に置く。（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）`
- Parsed: missing `position` entirely
- Should have: `position: "left_side"` or activation condition with position info

**Cards affected:** ~22 unique abilities, ~37+ card instances

---

### 2. Missing `exclude_self` on actions with 「このメンバー以外」 [HIGH IMPACT]

**Bug:** The parser correctly detects `exclude_self` in *conditions* (via `_extract_generic_fields` at parser.py:1068-1071) but FAILS to propagate it to *actions*. The `parse_action()` function at parser.py:1473 checks `このメンバー以外` but by that point, the text has already been split by the effect handler cascade. The effect-level handlers (choice, sequential, per_unit) create sub-actions that inherit fields from the parent but `exclude_self` is not propagated.

**Example (choice sub-action):**
- Full text: `以下から1つを選ぶ。\n・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。`
- Parsed: action with `exclude_self=None`
- Should have: `exclude_self=True`

**Cards affected:** 13 unique abilities, ~42 card instances

---

### 3. Missing `state_change` field on `change_state` costs [MEDIUM IMPACT]

**Bug:** When a cost contains `ウェイトにする` combined with other cost patterns (choice `か、` or sequential `し、`), the cost parser selects `choice_condition` or `sequential_cost` as the type instead of detecting the `state_change` within each sub-cost.

**Example:**
- Full text: `このメンバーをウェイトにするか、手札を1枚控え室に置く：エネルギーを1枚アクティブにする。`
- Parsed cost: `{type: "choice_condition", options: [...]}` — the "ウェイトにする" option within the choice is parsed as `custom` instead of `change_state` with `state_change: "wait"`

**Cards affected:** 2 unique abilities, ~6 card instances

---

### 4. `do_nothing` actions inserted as artifacts of text splitting [HIGH IMPACT]

**Bug:** When the parser splits text on `。` (period) or `、` (comma) for implicit sequential effects, parenthetical notes in separate sentences (especially `（対戦相手のカードの効果でも発動する。）`) are parsed as separate actions that fall through to `parse_action()` and match `何もしない` or empty text -> `do_nothing`.

**Example:**
- Full text: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。（対戦相手のカードの効果でも発動する）`
- Parsed actions: `['do_nothing', 'do_nothing', 'gain_resource']`
- Should be: `['gain_resource']` (with optional `trigger_scope: "any"` or similar)

**Cards affected:** 11 unique abilities, ~24 card instances

---

### 5. `conditional` missing on sequential actions containing 「そうした場合」 [MEDIUM IMPACT]

**Bug:** Sequential actions produced by `_try_choose_self_opponent()` and `_try_opponent_after_conditional()` don't set `conditional: true`. Only `_try_conditional_sequential()` sets this flag, but sequential patterns that happen to contain "そうした場合" after other structural separators miss it.

**Example:**
- Full text: `自分か相手を選ぶ。自分は、そのプレイヤーの控え室にあるライブカードを1枚、そのプレイヤーのデッキの一番下に置く。そうした場合、自分はカードを1枚引く。`
- Parsed: `{action: "sequential", actions: [...], conditional: null}`

**Cards affected:** 3 unique abilities, ~6 card instances

---

### 6. Wrong `card_type` on energy-related effects [MEDIUM IMPACT]

**Bug:** `_infer_card_type()` at parser.py:1210-1227 has flawed fallback logic. When no card type is explicitly mentioned in text, it falls back to `card` or `member_card` based on the source, but misses energy card patterns in complex texts.

**Example:**
- Full text: `ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、このメンバーの下にあるエネルギーカードの枚数に1を足した枚数のエネルギーカードをウェイト状態で置く。`
- Parsed: `card_type: "member_card"` (should be `"energy_card"`)

**Cards affected:** 2 unique abilities, ~6 card instances

---

### 7. `"グループ名"` patterns not parsed as group conditions [LOW IMPACT]

**Bug:** Effect text containing "グループ名" (group name) patterns like "同じグループ名を持つ" or "グループ名が異なる" are not extracted as `group` fields on the parsed action. The `extract_group()` function (parser.py:318-326) only looks for `『』` (Japanese corner brackets) to identify groups.

**Example:**
- Full text: `手札を1枚控え室に置いてもよい：ライブ終了時まで、これにより控え室に置いたカードと同じグループ名を持つメンバー1人は、{{heart_01.png|heart01}}を得る。`
- Parsed: Missing `group` or `group_reference` field
- Should have: `group_reference: "discarded_card_group"` or similar

**Cards affected:** 8 unique abilities, ~18 card instances

---

### 8. `名前が異なる` in compound conditions loses distinct flag [MEDIUM IMPACT]

**Bug:** When the condition matches `_try_compound()`, the `名前が異なる` pattern is only checked in `_extract_generic_fields()` which is skipped for compounds (they return early). The compound handler (parser.py:630-656) does post-process sub-conditions for `comparison_condition` types but misses the `distinct` flag on `location_condition` sub-types.

**Example:**
- Full text: `自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合、...`
- Parsed: compound with `conditions: [{appearance_condition}, {location_condition}]` — second sub-condition missing `distinct: True`
- Should have second sub-condition with `distinct: True`

**Cards affected:** 2 unique abilities, ~8 card instances

---

### 9. `能力を持たない` (no ability) in baton touch context parsed as wrong type [LOW IMPACT]

**Bug:** `_try_baton_touch()` (parser.py:738-755) matches before `_try_ability_negation()` (parser.py:897-900), so "能力を持たないメンバーからバトンタッチして登場した" gets parsed as a baton touch condition with type `location_condition` instead of an ability negation condition.

**Example:**
- Full text: `能力を持たないメンバーからバトンタッチして登場した場合、カードを1枚引く。`
- Parsed condition: `{type: "location_condition", baton_touch_trigger: true, ...}`
- Should have: `{type: "ability_negation_condition", ...}` AND `{baton_touch_trigger: true}`

**Cards affected:** 1 unique ability, ~4 card instances

---

### 10. `登場か、エリアを移動` (OR of appearance+movement) in each_time triggers [MEDIUM IMPACT]

**Bug:** When the pattern "登場か、エリアを移動" appears inside a "たび" (each_time) clause, the `_try_each_time()` handler (parser.py:2405-2416) runs BEFORE `_try_or()` which is only called from `parse_condition()`. The `_try_each_time()` handler passes the whole text to `parse_effect()` which doesn't try `_try_or()`, so the OR condition is lost.

**Example:**
- Full text: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
- Parsed: No condition parsed at all (action=sequential with do_nothing artifacts)
- Should have: `trigger_type: "each_time"` with condition `{type: "or_condition", conditions: [{appearance}, {movement}]}`

**Cards affected:** 2 unique abilities, ~6 card instances

---

### 11. `"元々持つ"` (original/natural) - set_blade_count parsed for text that should be an action type [LOW IMPACT]

**Bug:** The ability "ライブ終了時まで、自分のステージにいる『蓮ノ空』のメンバー1人が元々持つハートをすべて{{heart_01.png|heart01}}にする。" is parsed as `gain_resource` but should be `modify_original_hearts` (a conceptual type change, not gaining a resource).

**Example:**
- Full text: `ライブ終了時まで、自分のステージにいる『蓮ノ空』のメンバー1人が元々持つハートをすべて{{heart_01.png|heart01}}にする。`
- Parsed: `{action: "gain_resource", resource: "heart", ...}` — WRONG
- Should be: `{action: "set_original_hearts", ...}`

**Cards affected:** 4+ unique abilities, ~8 card instances

---

### 12. Missing `"all"` flag for "すべての" patterns [MEDIUM IMPACT]

**Bug:** The "すべての" (all) pattern detection at parser.py:1768 uses `re.search(r'すべての|全ての|全部の|全て|全員|全体', text)` but this only runs in `parse_action()` for simple actions. For actions within sequential/choice/conditional structures, the sub-actions may not get the `all` flag.

**Example:**
- Full text: `自分のステージにいる『Liella!』のメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。`
- Parsed: sub-action missing `all: true`
- Expected: `all: true`

**Cards affected:** 3 unique abilities, ~8 card instances

---

### 13. Missing `per_unit` flag for per-unit patterns in parenthetical notes [LOW IMPACT]

**Bug:** When per-unit patterns (につき) appear inside parenthetical notes `（エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。）`, the `strip_parenthetical()` removes them before `_try_per_unit()` runs. These notes are stored separately but the per-unit effect is lost from the main effect.

**Example:** Many abilities with parenthetical "(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。)"
- Parsed: main effect lacks the per-unit draw behavior
- The parenthetical is extracted but not parsed as a separate effect

**Cards affected:** 7 unique abilities, ~7 card instances

---

## Estimated Total Impact

| Category | Unique Abilities | Card Instances | Severity |
|---|---|---|---|
| 1. Missing position | ~22 | ~37+ | HIGH |
| 2. Missing exclude_self | ~13 | ~42 | HIGH |
| 3. Missing state_change in costs | ~2 | ~6 | MEDIUM |
| 4. do_nothing artifacts | ~11 | ~24 | HIGH |
| 5. Missing conditional flag | ~3 | ~6 | MEDIUM |
| 6. Wrong card_type (energy) | ~2 | ~6 | MEDIUM |
| 7. Group name not parsed | ~8 | ~18 | LOW |
| 8. Distinct flag in compounds | ~2 | ~8 | MEDIUM |
| 9. Ability negation + baton touch | ~1 | ~4 | LOW |
| 10. OR in each_time | ~2 | ~6 | MEDIUM |
| 11. Wrong action type (元々) | ~4 | ~8 | LOW |
| 12. Missing all flag | ~3 | ~8 | MEDIUM |
| 13. Per-unit in parenthetical | ~7 | ~7 | LOW |

**Total: ~80 unique abilities affected (13% of 602), touching ~180 card instances (10% of 1806 total cards, 17% of 1057 cards with abilities)**
