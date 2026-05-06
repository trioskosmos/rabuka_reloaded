# Ability Text vs JSON Structure Issues Report

## Overview
This report identifies specific issues where the ability text content doesn't match the JSON structure, indicating parsing problems or data inconsistencies.

**Analysis Date:** 2026-05-06  
**Total Issues Found:** 12 critical text vs JSON mismatches

## Critical Issues

### 1. **Malformed Full Text with Quotes**
**Severity:** HIGH  
**Examples:**

```json
{
  "full_text": "\"{{toujyou.png|登場}}自分のステージにコスト13以上のメンバーがいる場合、カードを1枚引く。",
  "triggerless_text": "",
  "triggers": null,
  "is_null": true,
  "cost": null,
  "effect": null
}
```

**Problem:** 
- Full text starts with quote character `"` but doesn't end properly
- triggerless_text is empty despite having meaningful content
- JSON marked as null but contains actual ability text
- **Cards affected:** PL!-bp3-009-P+, PL!-bp3-009-SEC (矢澤にこ)

### 2. **Truncated Ability Text**
**Severity:** HIGH  
**Examples:**

```json
{
  "full_text": "\"{{toujyou.png|登場}}手札のライブカードを1枚控え室に置いてもよい：カードを3枚引く。",
  "triggerless_text": "",
  "triggers": null,
  "is_null": true,
  "cost": null,
  "effect": null
}
```

**Problem:**
- Same quote issue as above
- Ability text appears complete but JSON structure is null
- **Cards affected:** PL!S-bp3-003-P+, PL!S-bp3-003-SEC (松浦果南)

### 3. **Complex Ability with Quote Issues**
**Severity:** HIGH  
**Example:**

```json
{
  "full_text": "{{live_start.png|ライブ開始時}}手札を2枚まで控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いたカード1枚につき、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。\"",
  "triggerless_text": "手札を2枚まで控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いたカード1枚につき、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。\""
}
```

**Problem:**
- Text ends with quote character causing parsing issues
- triggerless_text also has trailing quote

### 4. **Parenthetical Abilities Marked as Null**
**Severity:** MEDIUM  
**Examples:**

```json
{
  "full_text": "(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)",
  "triggerless_text": "",
  "triggers": null,
  "is_null": true,
  "cost": null,
  "effect": null
}
```

**Problem:**
- These are rule clarification abilities, not active abilities
- Should probably be handled differently, not as null abilities
- **Cards affected:** 10+ live cards with rule text

## Trigger Parsing Issues

### 5. **Multiple Triggers Not Properly Handled**
**Severity:** MEDIUM  
**Example:**

```json
{
  "full_text": "{{toujyou.png|登場}}/{{live_start.png|ライブ開始時}}このメンバーをウェイトにしてもよい：...",
  "triggers": "ライブ開始時, 登場"
}
```

**Problem:**
- Text shows triggers as "/" separated but JSON uses comma
- Inconsistent trigger format representation

### 6. **Complex Trigger Combinations**
**Severity:** LOW  
**Examples found:**
- `"triggers": "起動"` with `{{turn1.png|ターン1回}}` in text
- `"triggers": "自動"` with complex conditions in text
- `"triggers": "常時"` (constant abilities)

## Text Content vs Structure Mismatches

### 7. **Empty Triggerless Text with Meaningful Content**
**Severity:** MEDIUM  
**Pattern:** Multiple abilities have `"triggerless_text": ""` but the full_text contains meaningful ability text that should be extractable.

**Examples:**
- Abilities with rule text in parentheses
- Complex conditional abilities
- Multi-trigger abilities

### 8. **Count Null Values**
**Severity:** MEDIUM  
**Pattern:** Several abilities have `"count": null` but should have specific counts:

```json
{
  "text": "選んだハートを1つ得る",
  "count": null,
  "action": "gain_resource",
  "resource": "heart"
}
```

**Problem:** Text clearly states "1つ" (1) but count is null.

### 9. **Inconsistent Text Field Content**
**Severity:** LOW  
**Examples:**
- Some abilities have detailed text in nested actions
- Others have minimal text with full description in parent field
- Inconsistent granularity of text descriptions

## Special Character Issues

### 10. **Icon and Template Parsing**
**Severity:** LOW  
**Pattern:** Template syntax like `{{icon_blade.png|ブレード}}` appears in text but may not be properly processed by engine.

**Examples found:**
- `{{heart_01.png|heart01}}` 
- `{{icon_energy.png|E}}`
- `{{turn1.png|ターン1回}}`

### 11. **Japanese Text Encoding Issues**
**Severity:** LOW  
**Pattern:** Some abilities have unusual quote characters or encoding issues.

### 12. **Parenthetical Text Handling**
**Severity:** LOW  
**Pattern:** Rule text in parentheses not consistently handled:

```json
"（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）"
```

## Impact Assessment

### High Impact Issues (3)
1. **Quote parsing errors** - Abilities completely unusable
2. **Null structure with content** - Abilities lost during processing  
3. **Truncated text** - Incomplete ability definitions

### Medium Impact Issues (6)
1. **Multiple trigger handling** - May trigger at wrong times
2. **Empty triggerless text** - Display issues in UI
3. **Null count values** - Incorrect ability execution
4. **Rule text as null abilities** - Missing rule implementations
5. **Complex trigger combinations** - Engine may not support
6. **Text granularity inconsistencies** - UI display problems

### Low Impact Issues (3)
1. **Icon template parsing** - Visual display issues
2. **Character encoding** - Minor text display problems
3. **Parenthetical text** - Rule clarification issues

## Recommendations

### Immediate Fixes Required
1. **Fix quote parsing** in ability extraction script
2. **Handle null abilities with content** properly
3. **Validate trigger text extraction** for multi-trigger abilities

### Medium Priority
1. **Standardize count field population** from text
2. **Improve triggerless text extraction**
3. **Handle rule text separately** from active abilities

### Low Priority
1. **Implement icon template processing**
2. **Standardize text field granularity**
3. **Add encoding validation**

## Affected Cards Summary

- **Critical issues:** 6 cards (P+ and SEC variants)
- **Medium issues:** 50+ cards with complex triggers
- **Low issues:** 100+ cards with template syntax

## Files to Review

1. `cards/abilities.json` - Fix parsing issues
2. `tools/ability_extraction/extract_card_abilities.py` - Fix extraction logic
3. `engine/src/ability/resolver.rs` - Handle complex triggers
4. `engine/src/core/card.rs` - Validate ability structure
