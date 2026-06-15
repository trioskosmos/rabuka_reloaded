# BP6 Parser Fixes - 2026-05-31

## Fix #1: comparison_target for "Xより" patterns
**Card:** PL!S-bp6-009-R 黒澤ルビィ
**Ability:** 相手の成功ライブカード置き場にあるカードの枚数が自分より多いかぎり、その差に等しい数のブレードを得る。
**Bug:** `comparison_target` was "opponent" instead of "self" because non-contiguous fallback matched "相手" at position 0 with "より" from "自分より".
**Fix:** `_extract_generic_fields`: contiguous match first (priority), then non-contiguous fallback only if no contiguous match found.
**File:** parser.py `_extract_generic_fields` ~line 1613

## Fix #2: Choice cost for "verb+か" without comma
**Card:** PL!S-bp6-007-R 国木田花丸
**Ability:** E2支払うか手札を2枚控え室に置いてもよい（自己的成功ライブカード置き場にカードがなく、相手に2枚以上ある場合...）
**Bug:** Parsed as `sequential_cost` instead of `choice_condition`. The "か" (OR) marker between the two cost options was missed because the existing choice parser only matched "か、" (with trailing comma).
**Fix:** `parse_cost`: added verb+か choice detection (`(.*(?:支払う|置く|加える|公開する))か(.+)`) BEFORE both energy handler and sequential split.
**File:** parser.py `parse_cost` ~line 3470

## Fix #3+5: look_action source from condition zone
**Card:** PL!-bp6-006-R 西木野真姫
**Ability:** 手札を1枚控え室に置く：好きなハートの色を1つ指定する。その後、デッキの上から5枚公開。その中からμ'sを1枚手札に加え...
**Bug:** No `look_action.source` set — the parser knew the condition checked a zone but didn't propagate that zone as where to look.
**Fix:** `_try_look_and_select`: when condition mentions a specific zone (not "stage" or "hand"), propagate as `look_action.source`.
**File:** parser.py `_try_look_and_select` ~line 4390

## Fix #4: {{icon_all.png|ハート}} count
**Card:** PL!S-bp6-002-R 桜内梨子
**Ability:** ライブ終了時まで、{{icon_all.png|ハート}}{{icon_all.png|ハート}}を得る
**Bug:** count was 1 instead of 2. `infer_count_from_icons` counted `{{heart_XX.png}}` patterns but not `{{icon_all.png|ハート}}`.
**Fix:** `infer_count_from_icons`: added `effect_text.count("{{icon_all.png|ハート}}")` check before the heart regex.
**File:** parser.py `infer_count_from_icons` ~line 1999

## Fix #6: Cross-position (右サイドエリアと左サイドエリア)
**Card:** PL!-bp6-009-R 矢澤にこ
**Ability:** 右サイドエリアと左サイドエリアに、元々持つブレードの数が2つのメンバーがいるかぎり、スコア+1
**Bug:** `position_compare` not set — parser only extracted one position keyword, not both sides.
**Fix:** `parse_action`: detect cross-position from POSITION_KEYWORDS, set `position` + `position_compare` when multiple match.
**File:** parser.py `parse_action` ~line 2560

## Fix #7: Excessive sequential nesting
**Card:** PL!HS-bp6-005-R 徒町小鈴
**Ability:** 手札を1枚控え室に置いてもよい：このメンバーのコストを+6する。その後、蓮ノ空のコスト合計が高い場合、ハート+ブレードを得る
**Bug:** 3-level sequential nesting (sequential > sequential > sequential). The second part of "此后、" was a conditional action that got double-wrapped.
**Fix:** `_try_sequential`: when sa is a sequential wrapping a single conditional action, flatten by pulling condition onto the action directly.
**File:** parser.py `_try_sequential` ~line 4690
