# Parser Fix Plan — Missing/Incorrect Detail in abilities.json

## Overview

This document catalogs issues found by comparing `abilities.json` against what `parser.py` should generate. Each issue includes test strategy and implementation approach.

---

## Fix 1: `propagate_count_source` — convert `location_condition`/`group_condition`

**Impact:** 4 entries in abilities.json

**Problem:** The propagation block in `_walk()` (line 6739-6745) handles "それらの中に"/"これにより" but only for `card_count_condition`. `location_condition` and `group_condition` types are skipped, so they don't get `source: "preceding_moved"`.

**Fix:** Extend the "それらの中に"/"これにより" branch to also convert `location_condition` and `group_condition` to `card_count_condition` with `source: "preceding_moved"`, the same way the "すべて"/"全部" branch already does for `group_condition`.

**Test:** Parse ability text and verify conditions with "これにより控え室に置いたカードが..." get `type: "card_count_condition"` and `source: "preceding_moved"`.

---

## Fix 2: `recently_moved` override — too aggressive

**Impact:** 2 entries in abilities.json

**Problem:** Both `_try_conditional` (line 5394) and `_try_baton_touch_effect` (line 5502) override `source` from `"discard"` to `"recently_moved"` whenever `baton_touch_trigger` is true AND `action.action` is `"move_cards"` AND `action.source` is `"discard"`. But for abilities where the text says "自分の控え室から" (from own discard room — a general search), not "このバトンタッチで控え室に置かれた" (placed by this specific baton touch), the source should remain `"discard"`.

**Fix:** Only override to `"recently_moved"` when the action text explicitly contains "このバトンタッチで控え室に置かれ" or similar phrases indicating the specific baton-touch-moved card. Otherwise leave the source as `"discard"`.

**Text to check:** Check action text for `"このバトンタッチで控え室に置かれ"` before overriding.

**Test:** Parse both types of baton touch abilities and verify source field is correct.

---

## Fix 3: Wrong source for yell-revealed cards

**Impact:** 1 entry in abilities.json (ダイスキだったらダイジョウブ！ line 21022)

**Problem:** After a yell reveal action, when the text says "それらのカードをすべて控え室に置いてもよい" (you may put all of those cards in the waiting room), the action gets `source: "hand"` (default fallback) instead of `source: "revealed_cards"`.

**Fix:** In `_fill_defaults` or post-processing of `move_cards` actions: when the action text contains "それらのカード" and is preceded by a yell reveal context, set `source: "revealed_cards"`. Alternatively, detect this in the sequential action post-processing where the preceding action reveals cards.

**Test:** Parse the yell ability and verify the follow-up discard action has `source: "revealed_cards"`.

---

## Fix 4: Wrong condition location for yell-revealed cards

**Impact:** 1 entry in abilities.json (桜小路きな子 line 13102)

**Problem:** Condition text "エールにより公開された自分のカードの中に、名前が異なる『Liella!』のメンバーカードが3枚以上ある場合" gets `location: "stage"` instead of `location: "revealed_cards"`.

**Fix:** In the condition parsing cascade, when detecting "エールにより公開された" or "これにより公開された" before the condition type is assigned, ensure `location` is set to `"revealed_cards"`. This could be done in `_extract_generic_fields` or a dedicated fixup pass.

**Test:** Parse the condition text and verify `location: "revealed_cards"`.

---

## Fix 5: `exclude_group_names` vs `group_names` for "以外" patterns

**Impact:** 1 entry in abilities.json (line 7064)

**Problem:** The text "『スリーズブーケ』以外のメンバー1人につき" (for each member other than スリーズブーケ) uses `group_names: ["スリーズブーケ"]` instead of `exclude_group_names: ["スリーズブーケ"]`.

**Review:** Check if the existing code at line 2136-2145 already handles this. If the エリア pattern ("エリア" in text) interferes, fix the filter to not skip the "以外" detection.

**Test:** Parse text with "以外" pattern and verify `exclude_group_names` is set.

---

## Fix 6: `heart_selection` — not for set-heart patterns

**Impact:** 1 entry in abilities.json (line 25394)

**Problem:** "自分のステージにいる『蓮ノ空』のメンバー1人が元々持つハートをすべてheart01にする" sets `heart_selection: true`, but this is "set to specific color" not "player chooses color".

**Fix:** In the dispatch table at line 3226-3228, the pattern `lambda t: "ハートの色を" in t or ("ハートを" in t and "にする" in t)` matches both "ハートを選ぶ" AND "ハートをすべてXXにする". Add exclusion for "すべて" + icon patterns that specify a fixed color.

**Test:** Parse both types of patterns and verify `heart_selection` is only present for choose-color patterns.

---

## Fix 7: Missing `activation_condition_parsed` for "いる場合のみ"

**Impact:** 3 entries in abilities.json (lines 12159, 22806, 22872)

**Problem:** The parenthetical text "この能力はセンター/左サイド/右サイドエリアにいる場合のみ発動する。" is correctly detected and sets `activation_position`, but does NOT set `activation_condition_parsed`.

**Fix:** In `_merge_parenthetical` (line 6064) and `parse_effect` (line 6114), the check `"センター" in note or "サイド" in note or "エリアにいる場合" in note` should already pass. Investigate why `activation_condition_parsed` isn't set. The issue is that `parse_condition(note)` is called but may return `type: "custom"`, which is filtered out at line 6066. The condition "この能力はセンターエリアにいる場合のみ発動する。" may not match any specific handler.

**Test:** Parse the parenthetical text and verify the parsed condition is stored in `activation_condition_parsed`.

---

## Fix 8: Missing `or_card_types` in select actions

**Impact:** 3 entries in abilities.json (lines 20419-20552)

**Problem:** The pattern "ハートにXXを2個以上持つメンバーカードか、必要ハートにXXを2以上含むライブカードを1枚公開して手札に加えてもよい" has a `select_cards` action with only `card_type: "member_card"`, missing `or_card_types: ["member_card", "live_card"]`.

**Fix:** This pattern has "メンバーカードか...ライブカード" but the "公開して" pattern routes through `reveal` not `select`. Need to detect the OR card type pattern in the `reveal` → `select_cards` path and propagate the `or_card_types`.

**Test:** Parse the ability text and verify the select_cards action has `or_card_types`.

---

## Fix 9: Dynamic count improvements

**Impact:** 3 entries in abilities.json (lines 3939, 4923, 6456)

### 9a: Wrong reference prefix (line 4923)
**Problem:** `"reference": "自分のデッキの上から、自分のステージにいるメンバーの数"` includes the extraneous prefix "自分のデッキの上から、"

**Fix:** In `extract_dynamic_count` or post-processing: trim the reference string by removing known prefixes like "自分のデッキの上から、"

### 9b: Missing dynamic_count for energy under member + 1 (line 3939)
**Problem:** "このメンバーの下にあるエネルギーカードの枚数に1を足した枚数" should produce a `dynamic_count` but doesn't.

**Fix:** In `_fill_defaults` or `extract_dynamic_count`: detect the pattern "の下にあるエネルギーカードの枚数にNを足した枚数" and generate `dynamic_count` with appropriate reference.

### 9c: Missing score-based energy cost (line 6456)
**Problem:** "そのカードのスコアに等しい数のEを支払ってもよい" (pay energy equal to card's score) has no dynamic count representation.

**Fix:** Detect score-based variable energy costs and add `dynamic_count`.

**Test:** Parse each text pattern and verify the correct dynamic_count fields.

---

## Fix 10: Missing `group_reference` in cost/effect

**Impact:** 2 entries in abilities.json (lines 7150, 25617)

### 10a: Cost context (line 7150)
**Problem:** "手札の同じグループ名を持つカード2枚を控え室に置いてもよい" lacks `group_reference: "same_group_name"` in the cost JSON.

**Fix:** In `_extract_basic_cost_fields`: detect "同じグループ名を持つ" pattern and set cost's `group_reference`.

### 10b: Effect context (line 25617)
**Problem:** "自分の控え室にある、自分のステージにいるすべてのメンバーと異なるグループ名を持つカード1枚を手札に加える" lacks `group_reference: "different_group_names"`.

**Fix:** In `_fill_defaults` or effect post-processing: detect "異なるグループ名" pattern and set `group_reference`.

**Test:** Parse both patterns and verify group_reference field.

---

## Regression Testing

After all fixes:
1. Regenerate `abilities.json` by running the ability extraction
2. Run `cargo test --no-fail-fast` — expect < 10 failures (currently 1 failure before changes)
3. Run the Python test scripts to verify specific patterns
