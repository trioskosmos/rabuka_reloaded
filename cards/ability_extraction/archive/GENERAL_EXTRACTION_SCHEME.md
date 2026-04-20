# General Extraction Scheme

## Core Principles

1. **Data-driven, not guess-based** - All patterns derived from analysis of 609 actual abilities
2. **Bidirectional mapping** - Consider both Japanese→code and code→multiple Japanese
3. **Structural parsing** - Parse grammatical structure, not just phrase matching
4. **State vs Location separation** - Distinguish card states (wait/active) from locations (discard/hand/stage)
5. **Card movement model** - All movement is source→destination with count, optional state change

## Parser Architecture

### Phase 1: Tokenization
Break text into semantic units:
- Direction markers: から (from), に (to), で (at/in)
- Locations: 控え室, 手札, ステージ, デッキ, エネルギーデッキ, etc.
- States: ウェイト状態, アクティブ状態
- Operators: 以上, 以下, より少ない, より多い
- Counts: N枚, N人
- Targets: 自分の, 相手の
- Card types: メンバーカード, ライブカード, エネルギーカード
- Groups: 『group name』
- Verbs: 引く, 置く, 選ぶ, 登場させる, etc.

### Phase 2: Structural Parsing
Identify high-level structure:
- Cost:effect split (： separator)
- Condition markers (場合, とき, かぎり, なら)
- Sequential markers (その後, 、)
- Choice markers (以下から1つを選ぶ)
- Compound operators (かつ)
- Duration modifiers (ライブ終了時まで)

### Phase 3: Component Extraction
For each segment, extract:
- Source (if direction is "from")
- Destination (if direction is "to")
- Count
- Card type
- Target (self/opponent/both)
- Group
- State change (wait/active)
- Cost limit
- Position (center/left/right)

### Phase 4: Semantic Assembly
Combine components into structured objects:
- Condition objects: target + location + card_type + operator + value + group
- Action objects: action + source + destination + count + state_change
- Cost objects: type + source + destination + count + optional
- Effect objects: condition + action + duration

## Correct Pattern Mappings

### Source Patterns (FROM)
| Japanese | Code | Notes |
|----------|------|-------|
| 控え室から | source: 'discard' | FROM discard zone |
| 手札から | source: 'hand' | FROM hand |
| デッキから | source: 'deck' | FROM deck |
| デッキの上から | source: 'deck_top' | FROM top of deck |
| ステージから | source: 'stage' | FROM stage |
| エネルギーデッキから | source: 'energy_deck' | FROM energy deck |

### Destination Patterns (TO)
| Japanese | Code | Notes |
|----------|------|-------|
| 控え室に置く | destination: 'discard' | TO discard zone |
| 手札に加える | destination: 'hand' | TO hand |
| ステージに登場させる | destination: 'stage' | TO stage |
| デッキの上に置く | destination: 'deck_top' | TO top of deck |
| メンバーのいないエリア | destination: 'empty_area' | TO empty area |

### State Change Patterns
| Japanese | Code | Notes |
|----------|------|-------|
| ウェイトにする | action: 'change_state', state: 'wait' | Change to wait state (horizontal) |
| アクティブにする | action: 'change_state', state: 'active' | Change to active state (vertical) |
| ウェイト状態で置く | state: 'wait' | Place in wait state |

### Action Patterns
| Japanese | Code | Notes |
|----------|------|-------|
| 引く | action: 'draw' | Implies deck→hand |
| 選ぶ | action: 'select' | Select cards |
| 見る | action: 'look_at' | Look at cards |
| 公開する | action: 'reveal' | Reveal cards |

## Code → Multiple Japanese Mappings

### DRAW (deck → hand)
- カードを引く
- カードをN枚引く
- 手札に加える

### DISCARD (any → discard zone)
- 控え室に置く
- 手札を控え室に置く
- ステージから控え室に置く

### WAIT STATE CHANGE
- ウェイトにする
- ウェイト状態で置く

### DEPLOY (any → stage)
- 登場させる
- ステージに登場させる
- メンバーのいないエリアに登場させる

## Parsing Algorithm

```python
def parse_ability(text):
    # Phase 1: Tokenize
    tokens = tokenize(text)
    
    # Phase 2: Identify structure
    structure = identify_structure(tokens)
    
    # Phase 3: Parse each segment
    result = {}
    
    if structure.has_cost:
        result['cost'] = parse_cost(structure.cost_segment)
    
    if structure.has_condition:
        result['condition'] = parse_condition(structure.condition_segment)
    
    if structure.has_effect:
        result['effect'] = parse_effect(structure.effect_segment)
    
    return result

def parse_cost(text):
    # Extract source (if "から" present)
    source = extract_source(text)
    
    # Extract destination (if "に置く" present)
    destination = extract_destination(text)
    
    # Extract count
    count = extract_count(text)
    
    # Extract optional flag
    optional = extract_optional(text)
    
    return {
        'type': 'move_cards' if source or destination else 'pay_energy',
        'source': source,
        'destination': destination,
        'count': count,
        'optional': optional,
        'text': text
    }

def parse_effect(text):
    # Check for sequential
    if 'その後' in text:
        return parse_sequential(text)
    
    # Check for choice
    if '以下から1つを選ぶ' in text:
        return parse_choice(text)
    
    # Check for condition
    if '場合' in text or 'とき' in text:
        return parse_conditional_effect(text)
    
    # Check for duration
    if 'かぎり' in text:
        return parse_duration_effect(text)
    
    # Extract direction
    source = extract_source(text)
    destination = extract_destination(text)
    
    # Extract state change
    state_change = extract_state_change(text)
    
    # Extract count
    count = extract_count(text)
    
    # Extract action
    action = extract_action(text)
    
    return {
        'action': action,
        'source': source,
        'destination': destination,
        'count': count,
        'state_change': state_change,
        'text': text
    }
```

## Key Functions

### extract_source(text)
```python
def extract_source(text):
    patterns = [
        ('控え室から', 'discard'),
        ('手札から', 'hand'),
        ('デッキから', 'deck'),
        ('デッキの上から', 'deck_top'),
        ('ステージから', 'stage'),
        ('エネルギーデッキから', 'energy_deck'),
    ]
    for pattern, code in patterns:
        if pattern in text:
            return code
    return None
```

### extract_destination(text)
```python
def extract_destination(text):
    patterns = [
        ('控え室に置く', 'discard'),
        ('手札に加える', 'hand'),
        ('ステージに登場させる', 'stage'),
        ('デッキの上に置く', 'deck_top'),
    ]
    for pattern, code in patterns:
        if pattern in text:
            return code
    return None
```

### extract_state_change(text)
```python
def extract_state_change(text):
    if 'ウェイトにする' in text or 'ウェイト状態で' in text:
        return 'wait'
    if 'アクティブにする' in text:
        return 'active'
    return None
```

## Validation

The parser must:
1. Distinguish source vs destination (not just "discard")
2. Treat wait as state change, not destination
3. Handle multiple Japanese expressions for same code concept
4. Parse grammatical structure, not just phrase match
5. Apply the 30 rules from comprehensive rule set
6. Handle nested structures (conditions within conditions, actions within actions)

## Implementation Priority

1. **High priority** (appears in 10+ abilities):
   - Cost:effect structure
   - Conditional structure (場合/とき)
   - Card movement (draw, discard, add to hand)
   - Resource gain (blades, hearts)
   - Basic conditions (cost limits, counts)

2. **Medium priority** (appears in 3-9 abilities):
   - Choice effects
   - Compound conditions
   - Sequential with explicit marker
   - Duration modifiers
   - Per-unit modifiers

3. **Low priority** (appears in 1-2 abilities):
   - Position change
   - Cost sum conditions
   - Heart icon selection
   - Live card count conditions
   - Card reveal actions

## Testing Strategy

1. Test on the 19 登場させる abilities to verify source/destination parsing
2. Test on the 28 abilities with both FROM and TO discard
3. Test on abilities with ウェイト to verify state change parsing
4. Test on the 9 choice effects
5. Test on the 7 sequential effects
6. Test on the 4 compound conditions
7. Audit for unparsed effects (should decrease from 157)

## Files Structure

```
ability_extraction/
├── parser.py                    # Main parser (new implementation)
├── parser_utils.py              # Utility functions
├── extract_card_abilities.py   # Extraction script
├── analysis/                   # Analysis scripts
│   ├── analyze_all_abilities.py
│   ├── audit_abilities.py
│   ├── audit_parser_output.py
│   ├── exploratory_analysis.py
│   ├── examine_toujou.py
│   ├── show_toujou_abilities.py
│   ├── validate_patterns.py
│   └── TOUJOU_ABILITIES.txt
├── ABILITY_STRUCTURE_ANALYSIS.md
├── ABILITY_PATTERNS_AUDIT.md
├── COMPREHENSIVE_RULE_SET.md
├── CONCEPTUAL_MODEL_CORRECTIONS.md
├── PATTERN_VALIDATION_FINDINGS.md
└── GENERAL_EXTRACTION_SCHEME.md
```
