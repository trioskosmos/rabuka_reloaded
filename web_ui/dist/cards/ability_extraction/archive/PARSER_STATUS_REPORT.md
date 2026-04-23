# Parser Status Report

## Current State (After Punctuation-Aware Parsing)

### Overall Statistics
- Total abilities: 609
- Custom actions (unparsed): 120 (19.7%)
- Custom costs (unparsed): 123 (20.2%)
- Missing effects: 0

### Punctuation Extraction Results
- **group_names extracted**: 88 abilities (from 『』 brackets)
- **quoted_text extracted**: 26 abilities (from 「」 quotes)
- **parenthetical extracted**: 24 abilities (from （） parentheses)
- **except_quoted extracted**: 3 abilities (「name」以外 patterns)

### Pattern Coverage

**Successfully parsed:**
- Sequential marker (その後): 15/15 (100%)
- Choice marker (以下から1つを選ぶ): 9/9 (100%)
- Duration marker (かぎり): 31/33 (94%)
- Conditions parsed: 308/609 (50.6%)

**Partially parsed:**
- Compound marker (かつ): 1/20 (5%) - parsing logic too strict
- Wait marker (ウェイト): 49/91 (54%) - many not recognized
- Position requirement: 13/26 (50%) - half not recognized
- Per-unit marker (につき): 50/50 (pattern present but action type may be custom)

## Punctuation-Aware Parsing Improvements

### Punctuation Analysis
The parser now recognizes and extracts Japanese punctuation markers that indicate names and special text:

- **『』 (group brackets)**: 201 abilities use these for group names like 『Liella!』, 『Aqours』, 『虹ヶ咲』
- **「」 (quotes)**: 49 abilities use these for ability names or character names
- **（） (parentheses)**: 24 abilities use these for parenthetical notes/clarifications

### New Extraction Functions
- `extract_group_names(text)` - extracts all group names within 『』 brackets
- `extract_quoted_text(text)` - extracts all text within 「」 quotes
- `extract_parenthetical(text)` - extracts all text within （） parentheses

### Integration Points
- **parse_condition**: Adds `group_names` field for conditions with 『』
- **parse_action**: Adds `group_names` and `quoted_text` fields
- **parse_cost**: Adds `group_names` and `quoted_text` fields
- **parse_effect**: Adds `parenthetical` field, extracts `activation_condition` from parenthetical

### Punctuation-Aware Pattern Improvements
- **Condition type detection**: When `group_names` is present, conditions are classified as `group_condition` or `group_location_count_condition`
- **Except conditions**: Added `except_quoted` field for 「name」以外 patterns
- **Ability gain**: Uses `quoted_text` for ability names when present
- **Activation conditions**: Parenthetical notes containing "起動できる" are extracted as `activation_condition`
- **Look and select**: Added pattern for "その中から" (from among them) indicating look_at + select + action

### Impact
- 88 abilities now have extracted group names
- 26 abilities now have extracted quoted text
- 24 abilities now have extracted parenthetical notes
- 3 abilities now have extracted except_quoted information

## Patterns Added

### Source/Destination Patterns
- `枚控え室に置く` → destination: 'discard' (handles "手札を1枚控え室に置く")
- `デッキの一番下に置く` → destination: 'deck_bottom'
- `成功ライブカード置き場に置く` → destination: 'success_live_card_zone'
- `手札を` + `控え室に置く` → source: 'hand'

### Action Types
- `look_at` - for "見る" patterns
- `reveal` - for "公開する" patterns
- `activate_ability` - for "能力を発動させる"
- `gain_ability` - for "能力を得る"
- `modify_required_hearts` - for "必要ハートが...多くなる/少なくなる"
- `modify_required_hearts_global` - for global modifier "～は、～" patterns
- `position_change` - for "ポジションチェンジ"
- `modify_reveal_count` - for "枚数が減る/増える"
- `play_baton_touch` - for "プレイに際し、バトンタッチしてもよい"

### Condition Types
- Compound (かつ) - with recursive parsing
- Except (以外) - with target extraction
- Either-case (いずれかの場合)
- Cost sum condition (コストの合計が、X、Y、Zのいずれか)
- Baton touch (バトンタッチ...場合)

### Structural Patterns
- Each-time triggers (たび) - with event extraction
- Per-unit modifiers (につき) - with base extraction
- "そうした場合" (if so) - conditional on optional
- Implicit sequential - comma-separated actions without explicit marker
- Activation conditions (～場合のみ起動できる) - in cost

### Position Patterns
- Added `左サイド` and `右サイド` variants

## Remaining Gaps

### High-Priority Unparsed Patterns

**1. Sequential Draw + Discard (multiple variations)**
- Pattern: "カードをN枚引き、手札をM枚控え室に置く"
- Issue: Implicit sequential not catching these
- Count: ~10 abilities

**2. Complex Costs with Multiple Actions**
- Pattern: "カードを見る。その中から1枚を手札に加え、残りを控え室に置く"
- Issue: Multi-step actions in cost
- Count: ~20 abilities

**3. Baton Touch Specific Conditions**
- Pattern: "バトンタッチして登場した場合、このバトンタッチで控え室に置かれた..."
- Issue: Specific baton touch context not fully parsed
- Count: ~5 abilities

**4. Activate Ability with Context**
- Pattern: "これにより控え室に置いたメンバーカードの登場能力1つを発動させる"
- Issue: Contextual ability activation
- Count: ~5 abilities

**5. Cost Sum Conditions**
- Pattern: "公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合"
- Issue: Pattern added but may not be matching correctly
- Count: ~5 abilities

### Medium-Priority Gaps

**6. Compound Conditions (かつ)**
- Only 1/20 parsed
- Issue: Parsing logic requires both parts to be non-custom, which is too strict
- Need: More lenient compound condition parsing

**7. Wait State Recognition**
- Only 49/91 parsed
- Issue: Various "ウェイト" patterns not recognized
- Need: More comprehensive wait state patterns

**8. Position Requirements**
- Only 13/26 parsed
- Issue: Some position patterns not matching
- Need: More position pattern variants

### Low-Priority Gaps

**9. Each-Time Triggers**
- 6 abilities with marker
- Pattern is recognized but action may be custom
- Need: More comprehensive each-time action parsing

**10. Per-Unit Modifiers**
- 50 abilities with marker
- Pattern is recognized but action type may be custom
- Need: Better integration with action types

## Conceptual Model Corrections Applied

1. **Wait as State Change**: ウェイト is now correctly parsed as a state change, not a destination
2. **Source vs Destination**: 控え室から is source: 'discard', 控え室に置く is destination: 'discard'
3. **Card Movement Model**: Draw/discard modeled as source→destination with count
4. **Code → Multiple Japanese**: Handled through pattern lists (e.g., multiple ways to express discard)

## Recommendations for Further Improvement

1. **Relax Compound Condition Parsing**: Allow compound conditions even if one part is custom
2. **Add More Wait Patterns**: Expand STATE_CHANGE_PATTERNS to cover more variations
3. **Improve Sequential Detection**: Better handling of comma-separated actions
4. **Add Contextual Action Parsing**: For patterns like "activate ability of X"
5. **Multi-Action Cost Parsing**: Handle costs with multiple sequential actions
6. **Baton Touch Context**: Better parsing of baton touch-specific conditions
7. **Cost Sum Validation**: Verify the cost sum condition pattern is matching correctly

## Files Modified

- `parser.py` - Complete rewrite with structural approach
- `extract_card_abilities.py` - Updated imports
- `GENERAL_EXTRACTION_SCHEME.md` - New documentation
- `CONCEPTUAL_MODEL_CORRECTIONS.md` - New documentation
- `PATTERN_VALIDATION_FINDINGS.md` - New documentation
- Test scripts moved to `analysis/` subfolder

## Next Steps

The parser now handles the most common patterns well (sequential, choice, duration). The remaining 121 custom actions and 123 custom costs are complex, rare, or highly contextual patterns. Further improvement would require:

1. Manual analysis of the remaining 244 unparsed abilities
2. Adding specific patterns for each unique case
3. More sophisticated context-aware parsing
4. Possibly a rule-based system for handling edge cases
