# Abilities.json Issues Documentation

This document lists structural issues and missing information found in `abilities.json`.

## Summary

The `abilities.json` file contains 19145 lines with a consistent overall structure for card abilities. Several issues have been identified and one has been resolved. Note that two entries marked as `is_null: true` are intentional - they represent rules/mechanics text rather than card abilities.

## Work Completed

- **Issue #3 (Extra Quotes):** RESOLVED - Fixed extra leading quotes in cards.json for PL!-bp3-009, PL!S-bp3-003, and PL!N-bp3-005 P+ and SEC variants. Regenerated abilities.json using `cards/ability_extraction/archive/extract_card_abilities.py`.

## Issues Found

### 1. Null Ability Entries (is_null: true) - NOT AN ISSUE

**Status:** These entries are correctly marked as null and represent rules text, not card abilities.

**Locations:**
- Line 270: Entry for 10 cards (PL!HS-PR-010-PR through PL!SP-sd1-025-SD)
  - `full_text`: "(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)"
  - This describes a rule about how ALLブレード works during heart confirmation.

- Line 4101: Entry for 3 cards (PL!HS-bp1-019-L through PL!SP-sd1-023-SD)
  - `full_text`: "(エールで出た{{icon_score.png|スコア}}1つにつき、成功したライブのスコアの合計に1を加算する。)"
  - This describes a scoring mechanic.

**Recommendation:** No action needed. These are correctly marked as null abilities and represent non-ability card text (rules/mechanics descriptions).

### 2. Missing Trigger Information (triggers: null)

**Issue:** Three entries have `triggers: null` but contain valid effects. This indicates missing trigger information that should be extracted from the full_text.

**Locations:**
- Line 7902: Entry for PL!-bp3-009 cards (矢澤にこ)
  - `full_text` has trigger info: "{{toujyou.png|登場}}"
  - Condition text also has extra quotes
- Line 8045: Entry for PL!S-bp3-003 cards (松浦果南)
  - `full_text` has trigger info: "{{toujyou.png|登場}}"
  - Condition text also has extra quotes
- Line 8474: Entry for PL!N-bp3-005 cards (宮下 愛)
  - `full_text` has trigger info: "{{jidou.png|自動}}"
  - Condition text also has extra quotes

**Note:** Lines 268 and 4099 are part of the null ability entries (rules text) and correctly have null triggers.

**Recommendation:** Extract trigger information from the `full_text` field and populate the `triggers` field appropriately.

### 3. Extra Quotes in Text Fields - RESOLVED

**Issue:** Several entries had extra leading quotes in their `full_text`, `triggerless_text`, and condition text fields in cards.json.

**Status:** FIXED - The extra quotes were removed from cards.json and abilities.json was regenerated using parser.py.

**Locations:**
- PL!-bp3-009 (矢澤にこ) - P+ and SEC variants had extra quotes
- PL!S-bp3-003 (松浦果南) - P+ and SEC variants had extra quotes
- PL!N-bp3-005 (宮下 愛) - P+ and SEC variants had extra quotes

**Resolution:** Removed leading quote characters from the ability fields in cards.json, then ran `cards/ability_extraction/archive/extract_card_abilities.py` to regenerate abilities.json. The regenerated file now has correct text without extra quotes.

### 4. Empty Text Fields

**Issue:** One entry has an empty text field in a condition.

**Locations:**
- Line 16513: Alternative condition text in conditional_alternative effect
  - This is part of a complex conditional alternative structure where the condition text is empty but the type is "custom"

**Recommendation:** Investigate whether the empty condition text is intentional or if the condition should be populated with actual text.


## Structural Observations

### Positive Findings

1. **Consistent Overall Structure:** The file maintains a consistent structure across all entries with fields: `full_text`, `triggerless_text`, `card_count`, `cards`, `triggers`, `use_limit`, `is_null`, `cost`, and `effect`.

2. **No Empty Card Arrays:** All entries have non-empty `cards` arrays.

3. **No Null Card Counts:** All entries have valid `card_count` values.

4. **No Null full_text Fields:** All entries have non-null `full_text` values.

5. **Proper JSON Syntax:** The file parses correctly as valid JSON.

### Action Types Observed

The following action types are present in the file:
- `draw`
- `move_cards`
- `change_state`
- `gain_resource`
- `look_and_select`
- `sequential`
- `choice`
- `custom`
- `modify_score`
- `modify_required_hearts`
- `restriction`
- `invalidate_ability`
- `set_score`
- `position_change`
- `reveal`
- `modify_limit`
- `set_blade_type`
- `set_heart_type`
- `set_required_hearts`
- `modify_required_hearts_global`

### Condition Types Observed

The following condition types are present in the file:
- `location_condition`
- `comparison_condition`
- `group_condition`
- `compound`
- `temporal_condition`
- `state_condition`
- `baton_touch_condition`
- `yell_revealed_condition`
- `ability_negation_condition`
- `movement_condition`
- `state_transition_condition`
- `action_restriction_condition`
- `distinct_condition`
- `position_condition`
- `cost_limit_condition`
- `yell_action_condition`
- `heart_variety_condition`
- `appearance_condition`
- `group_location_count_condition`

## Priority Recommendations

1. ~~**High Priority:** Fix the extra quotes in text fields (Issue #3) - this is a simple formatting issue.~~ **RESOLVED**
2. **High Priority:** Populate missing trigger information (Issue #2) - extract from full_text where available.
3. **Low Priority:** Investigate empty condition text in line 16513 (Issue #4) - may be intentional for custom conditions.
