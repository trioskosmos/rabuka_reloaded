# Parser Gap Analysis — Root Causes & Fix Plan

## Overview

This document catalogs every gap between what raw ability text contains and what
`parser.py` actually outputs into `abilities.json`. Each entry includes the root
cause in the parser/engine code, the effect on the engine runtime, and the fix
plan.

---

## 1. Null Abilities (8 total, 3 are BP6)

### Symptoms
8 abilities have `is_null: true` in `abilities.json`. They produce no trigger,
no cost, and no effect.

### Root Cause: `extract_card_abilities.py:31-145`
In `extract_trigger()`, the function scans icon patterns from the start of the
text. When it encounters `{{center.png|センター}}`, line 99 classifies it as a
cost icon (because `center` is in `cost_icon_patterns`) and skips it. This
consumes the position icon **before** trigger extraction finishes, and because
the position icon is now gone, no trigger remains → `triggers` stays empty →
line 223 marks it `is_null: true`.

The three BP6 nulls are:
- `PL!-bp6-001-P+` — `{{live_start.png|ライブ開始時}}{{center.png|センター}}…`
- `PL!-bp6-003-P+` — `{{live_start.png|ライブ開始時}}{{center.png|センター}}…`
- `PL!S-bp6-002-P+` — `{{jidou.png|自動}}{{turn1.png|ターン1回}}…` (also has `deck_top_or_bottom`)

### Fix
In `extract_trigger()`, don't skip icons in `cost_icon_patterns` at the start
unless there's already a trigger. `center` should only be filtered as cost
when it appears **after** a trigger has been found.

---

## 2. `non_stackable` Flag Missing on Effect Actions

### Symptoms
2 BP6 cards say `この効果は重複しない` but the effect tree has no
`non_stackable: true`:
- `PL!-bp6-019-L` (modify_cost)
- `PL!-bp6-022-L` (modify_required_hearts)

### Root Cause: `parser.py` — No handler for `重複しない`
`_fill_defaults()` and the dispatch table never check for `重複しない`.
The phrase is simply left in raw text and ignored.

### Fix
1. **Parser**: In `_fill_defaults()`, after the action is determined, check
   if `重複しない` is in the text and set `non_stackable: True`.
2. **Engine `AbilityEffect`**: Add `non_stackable: Option<bool>` field.
3. **Engine effects**: When executing `modify_cost`/`modify_required_hearts`,
   check the `non_stackable` flag and skip if already active.

---

## 3. `all: True` Not Propagated for `すべての` / `すべて`

### Symptoms
21 action nodes contain `すべて`/`すべての` in their text but lack `all: True`.
Examples include `look_and_select`, `sequential`, `modify_score`, `move_cards`.

### Root Cause: `parser.py:2068-2073`
`_fill_defaults()` has this regex:
```python
if not action.get("all") and re.search(r"すべての|全ての|全部の|全て|全員|全体", text):
    action["all"] = True
```
But it only runs once at `_fill_defaults()` time. Nodes created inside
sub-structures (sequential children, look_and_select's look_action/select_action,
etc.) go through `parse_action()` independently and never get post-processed.

### Fix
Move the `all` detection into `_fill_defaults()` but ensure it's also applied
recursively in `_normalize_effect_tree._walk()`. The normalizer already walks
sub-trees — propagate `all` there.

---

## 4. `cost_total` Not Set in Conditions

### Symptoms
Abilities checking `コストの合計がN` have `comparison_type: cost`,
`aggregate: total` but no `cost_total` field. Example: `徒町小鈴` checks
`コストの合計が相手より高い` which should set `cost_total`.

### Root Cause: `parser.py:1670-1676` and `parser.py:1694-1705`
`_infer_condition_type()` has two paths:
1. Lines 1670-1676: Sets `cost_total = condition["count"]` only when
   `comparison_type == "cost"` AND `aggregate == "total"` AND there's a count.
2. Lines 1694-1705: Sets `cost_total = condition["count"]` for aggregate=total
   with `コスト` in text BUT only checks `合計がN` pattern.

The problem: when the cost comparison is against `相手より高い` (not a bare
number), the condition has `comparison_target: "opponent"` and `operator: ">"`.
The `count` field might be `None` because there's no threshold number — it's a
relative comparison. So `cost_total` never gets set.

### Fix
Set `cost_total: true` (boolean flag) in the condition whenever
`comparison_type == "cost"` and the condition involves costs, regardless of
whether there's a numeric threshold. The engine should interpret a boolean
`cost_total` flag as "compare total costs". For numeric thresholds, use
`cost_total: N`.

---

## 5. `delayed_restriction` Pattern Not Detected (`アクティブしない`)

### Symptoms
`PL!HS-bp6-006-R+` says `次のターンのアクティブフェイズにアクティブしない` but
parses as `change_state` instead of `restriction`. The `restriction` action type
already exists in the engine (effects.rs line 389).

### Root Cause: `parser.py` dispatch table
The dispatch runs `change_state` (state_change = "wait") before checking for
`restriction`. The phrase `ウェイトにし、…アクティブしない` triggers
`change_state` on the `ウェイトにする` part, and the `アクティブしない` part
is lost.

### Fix
In `parse_action()`, after the main dispatch, if the text contains
`アクティブしない`, add a sub-action with `action: "restriction"`,
`restriction_type: "cannot_active"`. Or better: detect it as a combined
`sequential` with `change_state` + `restriction`.

---

## 6. `heart_00` / `heart0` Atypical Format Not Extracted on Some Nodes

### Symptoms
Several actions (`restriction`, `sequential`, `look_and_select`) have
`{{heart_00.png|heart0}}` in text but no `heart_color`/`heart_colors`.
The format `heart0` (without zero-padding) vs `heart01` causes regex
mismatches.

### Root Cause: `parser.py:1941-1944`
```python
hm = re.search(r"\{\{heart_(\d+)\.png\|heart\d+\}\}", text)
if hm:
    color = f"heart{hm.group(1).zfill(2)}"
    action.setdefault("heart_colors", []).append(color)
```
This only runs for `gain_resource` actions (inside `_fill_defaults()` under
`if a == "gain_resource"`). Other action types like `restriction`,
`modify_required_hearts`, etc. don't get heart_color extraction.

Also, `extract_heart_types` in parser_utils.py uses `heart_(\d+)` which only
matches `heart_00`, not `heart0`.

### Fix
Move heart_color extraction from `_fill_defaults` to the general
post-processing in `_normalize_effect_tree._walk()`, so ALL action types get
heart_colors extracted. Fix the regex to also match the atypical `heart0`
format: `(?:heart_(\d+)|heart(\d+))`.

---

## 7. Remaining Custom Actions (7)

### 7a. `」を得る` Fragment — `上原歩夢`
**Root cause**: The full ability text has `…{{heart_01.png|heart01}}、…、{{heart_05.png|heart05}}」を得る` where the closing `」` quotes an
implied card reference. The parser's `split_cost_effect()` or
`split_condition_action()` creates a fragment.

### 7b. Condition Text as `look_action` — `黒澤ダイヤ`
**Root cause**: `look_and_select` structure misparse. The condition
`自分のライブカード置き場にカードが2枚以上ある場合` is being placed as the
`look_action` node instead of being the condition on the parent action.

### 7c. Bracket Heart Format `［緑ハート］` — `松浦果南` & `Awakening Promise`
**Root cause**: The parser only recognizes `{{heart_NN.png|heartNN}}` icon
format for hearts. The bracket notation `［緑ハート］` is not handled.

### 7d. `conditional_alternative` Misplace — `錯覚CROSSROADS`
**Root cause**: `代わりに…置く場合` pattern. The parser's
`split_condition_action()` doesn't recognize this as a conditional alternative
placement because `場合` is embedded mid-text.

### 7e. `余剰ハートをすべて失う` — `コワレヤスキ`
**Root cause**: No `lose_resource` action type. The engine has no handler for
"lose all surplus hearts". This should be a new action type or use `gain_resource`
with negative sign.

### 7f. Per-Discard Heart Gain — `南ことり&…`
**Root cause**: Dynamic heart color per discard. The engine's `execute_custom()`
(rs:654-693) has special handling at line 677 for `duration.is_some()` which
routes to `gain_ability`. But the data has `duration: live_end` so it bypasses
the engine's existing custom handler and gets treated as a generic ability gain
instead of the per-discard-heart-gain logic at effect.rs:1132-1189.

---

## 8. `ability_negation` → Rename to `ability_filter`

### Current Semantics (Broken)
The parser sets `type: "ability_negation_condition"` when it sees
`能力を持たない` (cards without abilities, e.g. `能力を持たない『μ's』のカード`).

The engine at `condition.rs:1580-1587` evaluates it as:
```rust
fn evaluate_ability_negation_condition(&self, condition: &Condition) -> bool {
    let negation = condition.negation.unwrap_or(false);
    if negation { self.game_state.prohibition_effects.is_empty() }
    else { true }
}
```
This checks for **prohibition effects** (abilities negated by other effects,
i.e. `能力を無効にする`). This is completely wrong for the "cards without
abilities" semantic.

### Actual Intended Use
`能力を持たない` means "does not have an ability". It's a **card filter**
used in `look_and_select` / `select_cards` actions to restrict which cards
can be chosen. For example:
- `能力を持たないメンバーカード` → select a member card that has no abilities
- `能力を持つ『μ's』のカード` → select a μ's card that has abilities

### Fix Plan
1. **Rename** `ability_negation` → `ability_filter` in parser, Condition struct,
   engine condition evaluation.
2. **Parser**: Set `ability_filter: "no_ability"` for `能力を持たない`,
   `ability_filter: "has_ability"` for `能力を持つ`.
3. **Engine**: Instead of checking `prohibition_effects`, evaluate this as a
   card filter: the activating card's position in the database determines
   whether it has abilities. Cards with no ability text → `ability_filter: "no_ability"`.
4. **Remove** `ability_negation_condition` condition type entirely. This is NOT
   a condition — it's a filter on select actions.

---

## 9. `元々持つブレードの数` Not Detected as Condition

### Symptoms
12 abilities check `元々持つ{{icon_blade.png|ブレード}}の数がNつ以下` but
none have `blade_limit` on the condition or action node.

### Root Cause: `parser.py:2143-2146`
```python
if "blade_limit" not in action and "ブレード" in text:
    bl = extract_blade_limit(text)
    if bl:
        action.update(bl)
```
This runs in `_fill_defaults()` which only processes action nodes. It's never
called for condition nodes. And unlike `original_value`, there's no
`_extract_generic_fields` path for blade limits.

### Fix
Add `blade_limit` extraction to `_extract_generic_fields()` in `parse_condition()`,
and move the blade_limit extraction logic from `_fill_defaults` to the normalizer.

---

## 10. `group_names` Propagation Gaps

### Symptoms
Some sub-actions inside sequential/compound nodes have `『group』` in their
text but no `group_names` field.

### Root Cause: `parser.py` normalizer
The `_normalize_effect_tree._walk()` does propagate `group_names` from parent
to children, but only when the child's text doesn't already contain a group
marker. It should merge parent and child groups.

### Fix
In `_walk()`, when propagating `group_names`, merge parent groups with any
groups found in the child's own text.

---

## 11. `extra_yell_per_unit` Pattern Not Detected

### Symptoms
`MIRAI TICKET` and `月夜見海月` have `追加でエールを行う` with dynamic counts
(per discarded card), but this stays as `custom`.

### Root Cause
No dispatch rule for `追加でエール` / `エールを追加で行う`. The parser doesn't
recognize "additional yell" as `modify_yell_count`.

### Fix
Add dispatch rule for `追加でエールを行う` or `エールを追加で行う` → `re_yell`
action with `lose_blade_hearts` flag.

---

## 12. `slashed_dual_trigger` Not Detected

### Symptoms
Trigger text with `/` separator (e.g. `登場/ライブ開始時`) is not extracted
as dual trigger. The `triggers` field currently only captures icons, not
slash-separated text.

### Root Cause: `extract_card_abilities.py`
The `SLASH_TRIGGER_PATTERN` regex only handles `/{{icon}}` format. But some
abilities use `{{trigger1}}{{trigger2}}` with no slash, just consecutive icons.

### Fix
In `extract_trigger()`, collect ALL consecutive trigger icons at the start
(not just first one), and join them with `/` for dual triggers.

---

## 13. `original_value_blade_check` — `元々持つブレード` Not Parsed as Condition

### Symptoms
`元々持つ{{icon_blade.png|ブレード}}の数がNつ以下のメンバー` is not being
detected as a condition type. It falls through to `custom`.

### Root Cause
`_try_blade_count` in `parse_condition()` only checks for blade threshold
patterns (`ブレードがNつ以上`, etc.) but does NOT check for the `元々持つ`
prefix. So it misses the "original blade" variant.

### Fix
In `_try_blade_count()`, strip `元々持つ` before matching, and add
`original_value: true` to the result. Also add a condition type
`card_blade_condition` to handle this.

---

## 14. `deck_top_or_bottom` — Null Ability

### Symptoms
`PL!S-bp6-002-P+` (deck_top_or_bottom) is null. The trigger extraction fails
because it also has `{{turn1.png|ターン1回}}` as a use limit icon.

### Root Cause
The trigger extraction loop skips `turn` icons as use limits (line 104-123).
But the `{{jidou.png|自動}}` trigger is consumed by the slash/use_limit logic,
leaving no triggers → `is_null`.

### Fix
Fix trigger extraction to properly handle `{{trigger}}{{turn_limit}}` patterns
where the trigger comes BEFORE the turn limit.

---

## Fix Priority Order

1. **extract_card_abilities.py**: Fix null abilities (BP6 trigger extraction) — unlocks all BP6 testing
2. **parser.py `_fill_defaults()`**: Add `non_stackable` flag, `all` propagation
3. **parser.py `_infer_condition_type()`**: Fix `cost_total` in conditions
4. **parser.py parse_action()**: Add `delayed_restriction` pattern
5. **parser.py normalizer**: Move `heart_colors` extraction to cover all action types
6. **parser.py `_extract_generic_fields()`**: Add `blade_limit` extraction for conditions
7. **parser.py normalizer**: Fix `group_names` propagation
8. **parser.py**: Rename `ability_negation` → `ability_filter` with correct semantics
9. **parser.py**: Fix remaining custom actions (bracket hearts, extra yell)
10. **parser.py**: Fix `conditional_alternative` for `代わりに…置く場合`
11. **Engine `card.rs`**: Add `non_stackable`, `ability_filter` fields
12. **Engine `condition.rs`**: Fix `ability_filter` evaluation, add `cost_total` support
13. **Engine `effects.rs`**: Handle `non_stackable` flag, `delayed_restriction`
14. **Engine `execute_custom()`**: Route per-discard-heart-gain correctly
