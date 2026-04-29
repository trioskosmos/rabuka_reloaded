# Ability Testing Framework

## Current Status
- **Game State**: Stuck in RockPaperScissors phase (Turn 1)
- **Card Loading**: Cards showing as "cards" strings instead of objects
- **Testing Capability**: Cannot test abilities until deck initialization is fixed

## Framework Components Created

### 1. Ability Documentation System
```python
def document_abilities_from_state(state):
    # Scans hand and stage for cards with abilities
    # Returns: card name, location, ability text, type, cost
```

### 2. Ability Text Analysis
```python
def analyze_ability_texts(abilities):
    # Identifies ability types (Activation, Automatic, Continuous)
    # Detects effect patterns (draw, retrieval, hearts, energy, stage)
    # Analyzes cost patterns (self-sacrifice, energy payment)
```

### 3. Test Plan Generator
```python
def create_ability_test_plan(abilities, state):
    # Determines if abilities can be tested now
    # Provides testing methods and expected results
    # Status: READY or WAITING based on game state
```

## Ability Categories Identified

### Activation Abilities ({{kidou.png|}})
- **Trigger**: Manual activation by player
- **Testing**: Requires card to be on stage
- **Expected**: Ability should trigger and resolve when activated

### Automatic Abilities ({{jidou.png|}})
- **Trigger**: Game conditions (phase start, end, etc.)
- **Testing**: Trigger during appropriate phase
- **Expected**: Ability should trigger automatically

### Continuous Abilities ({{joki.png|}})
- **Trigger**: Always active while card is in play
- **Testing**: Verify continuous effect is applied
- **Expected**: Effect should be constantly applied

## Effect Patterns to Test

### Card Draw Effects
- **Text Pattern**: "cards to hand"
- **Test**: Verify hand size increases by correct amount
- **Expected**: +X cards in hand after ability resolves

### Retrieval Effects
- **Text Pattern**: "from discard" or "from waiting room"
- **Test**: Move specific cards from discard to hand
- **Expected**: Target cards appear in hand, disappear from discard

### Heart Generation
- **Text Pattern**: "hearts" or "heart"
- **Test**: Check heart count in appropriate zones
- **Expected**: Heart count increases by specified amount

### Energy Manipulation
- **Text Pattern**: "energy"
- **Test**: Verify energy zone changes
- **Expected**: Energy cards activated/deactivated as specified

### Stage Manipulation
- **Text Pattern**: "stage"
- **Test**: Check stage member positions
- **Expected**: Members moved/added/removed as specified

## Cost Verification

### Self-Sacrifice Costs
- **Pattern**: "this member to discard"
- **Test**: Verify member moves from stage to discard
- **Expected**: Member disappears from stage, appears in discard

### Energy Payment Costs
- **Pattern**: "energy" + "cost"
- **Test**: Verify energy cards set to wait state
- **Expected**: Energy count decreases, cards become inactive

## Testing Readiness Matrix

| Ability Type | Location | Current Readiness | Required For Testing |
|-------------|----------|------------------|-------------------|
| Activation | Hand | WAITING | Card must be played to stage |
| Activation | Stage | READY | Can test immediately |
| Automatic | Any | WAITING | Need appropriate phase trigger |
| Continuous | Any | WAITING | Need card in play to verify effect |

## Verification Against Rules.txt

### Rule 9.1: Ability Types
- **Activation ({{kidou.png|}})**: Manual trigger
- **Automatic ({{jidou.png|}})**: Game trigger  
- **Continuous ({{joki.png|}})**: Always active
- **Status**: Framework ready to verify implementation

### Rule 9.2: Effect Types
- **OneShot**: Single effect
- **ContinuousEffect**: Ongoing effect
- **Replacement**: Event replacement
- **Status**: Framework can detect and test these patterns

## Verification Against qa_data.json

### Common Ability Patterns
From QA analysis, common abilities to test:
1. **Card Draw**: "X cards to hand"
2. **Retrieval**: "from discard to hand"
3. **Heart Generation**: "X hearts"
4. **Energy Manipulation**: "energy activation"
5. **Stage Control**: "move member to/from stage"

## Implementation Verification Plan

### Phase 1: Fix Deck Loading
1. Resolve card serialization issue
2. Get proper card objects in game state
3. Verify deck initialization works

### Phase 2: Basic Ability Testing
1. Test simple activation abilities
2. Verify cost payment mechanics
3. Check effect resolution

### Phase 3: Complex Ability Testing
1. Test automatic abilities with phase triggers
2. Verify continuous ability effects
3. Test multi-step ability sequences

### Phase 4: Edge Case Testing
1. Test ability interactions
2. Verify ability timing rules
3. Test ability failure conditions

## Success Criteria

### Ability Text Accuracy
- [ ] Ability text matches implementation exactly
- [ ] All effect types work as written
- [ ] Cost payments work correctly
- [ ] Timing triggers work as specified

### Rules Compliance
- [ ] Abilities follow Rule 9.1 (types)
- [ ] Effects follow Rule 9.2 (types)
- [ ] Cost payment follows energy rules
- [ ] Timing follows phase rules

### QA Data Alignment
- [ ] Common ability patterns work
- [ ] Edge cases from QA work correctly
- [ ] Ability interactions are consistent

## Next Steps
1. **Priority**: Fix deck initialization to get proper card objects
2. **Priority**: Resolve RPS phase transition issue
3. **Then**: Begin ability testing with framework
4. **Finally**: Comprehensive ability verification against rules and QA data

The framework is ready and waiting for the core game initialization issues to be resolved before actual ability testing can begin.
