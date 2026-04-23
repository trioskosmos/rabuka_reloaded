# Parser Analysis: Comparison with abilities.json

## Format Comparison

### abilities.json format (desired)
- **Structure**: Groups by unique abilities (deduplication)
- **Fields**:
  - full_text, triggerless_text
  - card_count, cards (list of card IDs)
  - triggers (e.g., "起動", "登場", "ライブ開始時")
  - use_limit (e.g., 1 for turn limit, null for unlimited)
  - cost: {text, source, destination, card_type, type, count, optional}
  - effect: {text, source, destination, count, card_type, target, action}
  - Nested structures for complex effects (look_action, select_action)
- **Action types**: move_cards, look_and_select, gain_resource, pay_energy, custom

### My parsed_abilities_general.json format (current)
- **Structure**: Per-card (no deduplication)
- **Fields**:
  - raw, parsed, timing, conditions, costs, effects, targets, resources, locations, groups, patterns
  - Costs: {type, amount, optional, text} (simpler)
  - Effects: {type, object, duration} (simpler)
  - Pattern detection focus (18 patterns from manual analysis)

## Manual Verification of Specific Cards

### Card: PL!-sd1-002-SD (絢瀬 絪里)
**Raw**: "{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを1枚手札に加える。"

**abilities.json extraction**:
```json
{
  "triggers": "起動",
  "cost": {
    "text": "このメンバーをステージから控え室に置く",
    "source": "stage",
    "destination": "discard",
    "card_type": "member_card",
    "type": "move_cards"
  },
  "effect": {
    "text": "自分の控え室からメンバーカードを1枚手札に加える",
    "source": "discard",
    "destination": "hand",
    "count": 1,
    "card_type": "member_card",
    "target": "self",
    "action": "move_cards"
  }
}
```

**My parser extraction**:
```json
{
  "costs": [
    {
      "type": "card_to_discard",
      "text": "{{kidou.png|起動}}このメンバーをステージから控え室に置く"
    },
    {
      "type": "member_to_discard",
      "text": "{{kidou.png|起動}}このメンバーをステージから控え室に置く"
    }
  ],
  "effects": []  // EMPTY - FAILED TO EXTRACT EFFECT
}
```

**Issues**:
1. ❌ Effect extraction completely failed
2. ❌ Cost includes trigger icon in text (should be stripped)
3. ❌ Duplicate cost entries (card_to_discard AND member_to_discard)
4. ❌ Missing source/destination/card_type fields
5. ❌ Missing trigger extraction as separate field
6. ❌ No action classification

---

### Card: PL!-sd1-003-SD (南 ことり)
**Raw**: "{{toujyou.png|登場}}自分の控え室からコスト4以下の『μ's』のメンバーカードを1枚手札に加える。\n{{live_start.png|ライブ開始時}}手札を1枚控え室に置いてもよい：{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、選んだハートを1つ得る。"

**Note**: This has TWO separate abilities (split by \n)

**My parser**: Treats as one combined ability
- effects: [] (EMPTY)
- timing: ["on_entry", "live_start", "live_end"]

**Issues**:
1. ❌ Doesn't split multiple abilities
2. ❌ No effect extraction for either ability
3. ❌ Timing detection is incomplete

---

### Card: PL!-sd1-011-SD (絢瀬 絃里) - from abilities.json
**Raw**: "{{toujyou.png|登場}}手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。"

**abilities.json extraction**:
```json
{
  "triggers": "登場",
  "cost": {
    "text": "手札を1枚控え室に置いてもよい",
    "source": "hand",
    "destination": "discard",
    "count": 1,
    "optional": true,
    "type": "move_cards"
  },
  "effect": {
    "text": "自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く",
    "action": "look_and_select",
    "look_action": {
      "text": "自分のデッキの上からカードを3枚見る。",
      "source": "deck_top",
      "count": 3,
      "target": "self",
      "action": "look_at"
    },
    "select_action": {
      "text": "1枚を手札に加え、残りを控え室に置く",
      "destination": "discard",
      "count": 1,
      "action": "custom"
    }
  }
}
```

**My parser would extract**:
- effects: [{type: "look"}] (incomplete)

**Issues**:
1. ❌ Only detects "look" action, not the full look_and_select pattern
2. ❌ No nested look_action/select_action structure
3. ❌ Missing cost extraction with optional flag

---

## Key Problems with My Parser

### 1. Effect Extraction is Broken
- My parser's `_extract_effects()` method only looks for simple patterns like "を得る", "スコア", "登場させる"
- It doesn't handle the colon-separated cost:effect structure properly
- After splitting by colon, the effect part is not being parsed

### 2. No Deduplication
- abilities.json groups identical abilities across cards
- My parser is per-card only
- This is useful for different purposes but not what's requested

### 3. Missing Structured Fields
- No source/destination extraction for moves
- No card_type extraction
- No action classification (move_cards, gain_resource, etc.)
- No trigger extraction as separate field
- No use_limit detection

### 4. Multi-Ability Handling
- Cards can have multiple abilities separated by \n
- My parser treats them as one combined ability
- abilities.json splits them correctly

### 5. Cost Extraction Issues
- Includes trigger icons in cost text
- Creates duplicate cost entries
- Missing optional flag in many cases

## Comparison with Existing parser.py

The existing `parser.py` (kept in ability_extraction root, not archived) has **much better extraction logic** for the abilities.json format:

### parser.py Strengths:
1. **Proper cost/effect splitting**: Uses `split_cost_effect()` to split by "：" (colon)
2. **Structured cost parsing**: `parse_cost()` extracts:
   - source, destination, card_type, count
   - optional flag, max flag
   - state_change
   - type classification (move_cards, pay_energy, change_state, custom)
3. **Structured effect parsing**: `parse_effect()` extracts:
   - Complex action types (sequential, choice, look_and_select, conditional_on_optional)
   - Nested structures (look_action, select_action)
   - Condition parsing with location/group/count/operator
   - Action classification (move_cards, gain_resource, draw, look_at, etc.)
4. **Proper action parsing**: `parse_action()` classifies actions into specific game-relevant types
5. **Pattern detection**: Handles "その中から", "そうした場合", sequential markers, choice markers

### extract_card_abilities.py Strengths:
1. **Multi-ability handling**: Splits abilities by `\n` and handles continuations
2. **Trigger extraction**: `extract_trigger()` properly extracts trigger icons from start of text
3. **Use limit detection**: Detects turn restrictions (ターン1回)
4. **Deduplication**: Groups identical abilities across cards
5. **Parenthetical handling**: Appends parenthetical notes to previous ability

### My general_parser.py Weaknesses:
1. **Effect extraction is broken**: Returns empty effects array for most abilities
2. **No cost/effect splitting**: Doesn't properly handle colon-separated cost:effect
3. **No action classification**: Only detects generic types (gain, score_modification, summon, etc.)
4. **No source/destination extraction**: Missing critical game-relevant fields
5. **No multi-ability handling**: Treats multiple abilities as one
6. **No deduplication**: Per-card only, no grouping
7. **Pattern detection focus**: Designed for grammatical analysis, not game extraction

## Root Cause Analysis

My parser's `_extract_effects()` method only looks for simple keyword patterns:
```python
if 'を得る' in text:
    # Extract gain
if 'スコア' in text and ('＋' in text or '−' in text):
    # Extract score modification
```

But it **doesn't**:
- Parse the effect part after splitting by colon
- Classify actions into game-relevant types (move_cards, draw, look_at, etc.)
- Extract source/destination for card movements
- Handle nested structures (look_and_select)

The existing parser.py has ~850 lines of sophisticated parsing logic that handles all these cases.

## Recommendation

**My general_parser.py is NOT suitable for abilities.json format.**

### Options:

**Option 1: Use existing parser.py (RECOMMENDED)**
- The existing `parser.py` + `extract_card_abilities.py` already generate abilities.json correctly
- They have proper cost/effect parsing, action classification, and deduplication
- Just run: `python archive/extract_card_abilities.py` to regenerate abilities.json

**Option 2: Rewrite my parser to match abilities.json format**
Would require:
1. Complete rewrite of cost/effect extraction logic
2. Implement proper source/destination/card_type extraction
3. Implement action classification (move_cards, gain_resource, draw, look_at, etc.)
4. Handle multi-ability splitting by \n
5. Implement trigger extraction (like extract_trigger())
6. Implement deduplication logic
7. Handle nested structures (look_and_select with look_action/select_action)
8. Add use_limit detection

This is essentially recreating the existing parser.py - not worth the effort.

**Option 3: Hybrid approach**
- Keep my general_parser.py for pattern detection and grammatical analysis (its original purpose)
- Use existing parser.py for abilities.json generation
- They serve different purposes:
  - general_parser.py: Linguistic analysis, pattern detection
  - parser.py: Game-relevant extraction for engine implementation

## Conclusion

My general_parser.py was designed for **grammatical pattern analysis** based on MANUAL_GRAMMAR_ANALYSIS.md, not for **game-relevant extraction** like abilities.json requires.

The existing parser.py (849 lines) has sophisticated parsing logic that properly:
- Splits cost/effect by colon
- Extracts source/destination/card_type/count
- Classifies actions into game-relevant types
- Handles complex structures (sequential, choice, look_and_select)
- Deduplicates abilities across cards

**Recommendation**: Use the existing parser.py + extract_card_abilities.py for abilities.json generation. My general_parser.py can be kept for grammatical analysis if needed, but it cannot produce the abilities.json format without a complete rewrite.
