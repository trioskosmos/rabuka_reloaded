# Rules Validation Report

This document validates the engine's ability implementations against the official rules.txt and qa_data.json.

## Summary of Official Rules

### Card Types
- **Member (メンバー)**: Cards used for live judgment, have cost and hearts
- **Live (ライブ)**: Cards that perform live, have score and required hearts
- **Energy (エネルギー)**: Cards used to pay for member costs

### Areas
- **Stage (ステージ)**: Contains 3 member areas (left, center, right)
- **Live card placement area (ライブカード置き場)**: Where live cards are placed
- **Energy placement area (エネルギー置き場)**: Where energy cards are placed
- **Main deck (メインデッキ置き場)**: Main deck, shuffled, cards drawn from top
- **Energy deck (エネルギーデッキ置き場)**: Energy deck, cards moved to energy placement
- **Hand (手札)**: Non-public area for unused cards
- **Discard/Waitroom (控え室)**: Public area for used cards
- **Exclusion area (除外領域)**: Cards removed from game
- **Resolution area (解決領域)**: Temporary area for abilities and cheer

### Game Phases
1. **Active phase (アクティブフェイズ)**: Activate all wait cards
2. **Energy phase (エネルギーフェイズ)**: Move 1 energy card from energy deck to energy placement
3. **Draw phase (ドローフェイズ)**: Draw 1 card
4. **Main phase (メインフェイズ)**: Play abilities or member cards
5. **Live phase (ライブフェイズ)**:
   - Live card set phase: Set up to 3 live cards face-down, draw equal amount
   - Performance phase (first/second): Execute live with cheer mechanics
   - Live win/loss judgment phase: Compare scores, move successful live cards

### Key Mechanics

#### Cheer (エール)
- Sum all active member blades
- Move top card from deck to resolution area, repeat N times (where N = total blades)
- Check blade heart icons on cards in resolution area
- Each blade heart icon = draw 1 card

#### Hearts
- Sum all member heart icons + blade heart icons from resolution area
- This is "live owned hearts" (ライブ所有ハート)
- Check if live card's required hearts (必要ハート) are satisfied
- Wild heart (ALL) can be treated as any color

#### Cost Payment
- Member cost = pay equal number of energy (E)
- If insufficient energy, can use **baton touch (バトンタッチ)**: send 1 member from stage to discard to reduce cost by that member's cost
- Must execute all cost actions in order from top to bottom

#### Check Timing (チェックタイミング)
- Execute all rule processing first
- Then execute automatic abilities that have triggered
- Active player chooses order of their abilities
- Non-active player then chooses order of theirs

#### Refresh (リフレッシュ)
- When main deck is empty and discard has cards
- Shuffle discard and move all to bottom of deck
- Can happen at any time, even during effect resolution

#### Position Change (ポジションチェンジ)
- Move member to different area
- If destination has member, that member moves to source area
- If affecting opponent's member, opponent chooses destination

#### Formation Change (フォーメーションチェンジ)
- Move all stage members to any areas
- Cannot move 2+ members to same area

### Ability Types
- **Activation (起動能力)**: Played during play timing, has cost, format: `(条件)：（効果）`
- **Automatic (自動能力)**: Triggers on specific event, format: `（効果）` or `（条件）：（効果）`
- **Continuous (常時能力)**: Always active while valid, format: `(効果)`

### Effect Types
- **Single-shot (単発効果)**: Execute once and done (e.g., "draw 1 card")
- **Continuous (継続効果)**: Active for duration (e.g., "until end of turn")
- **Replacement (置換効果)**: Replace one event with another (e.g., "when X, instead do Y")

### Keywords
- **Turn 1 (ターン1回)**: Can only play once per turn
- **Turn 2 (ターン2回)**: Can only play twice per turn
- **登場**: Triggers when member moves to member area from elsewhere
- **ライブ開始時**: Triggers when live starts (performance phase)
- **ライブ成功時**: Triggers when live succeeds
- **センター**: Only valid when member is in center area
- **左サイド**: Only valid when member is in left side area
- **右サイド**: Only valid when member is in right side area

## Key Q&A Insights

### Card Identity
- Multi-character cards (e.g., "上原歩夢&澁谷かのん&日野下花帆") count as **1 member**
- Cards with same name but different versions (e.g., "Dream Believers" vs "Dream Believers (104期Ver.)") are **different cards**

### Energy Under Members
- Energy cards placed under members **do NOT count** toward energy count
- They are only counted when in energy placement area

### Position Change
- When affecting opponent's member, **opponent chooses destination**

### Baton Touch
- Requires both members to be from **previous turn** (not current turn)
- Members placed this turn cannot be used for baton touch

### Cost
- Cost can be reduced to **0**
- If deck has fewer cards than required for cost, **cannot pay**

### Blade Count
- Blade count modification affects **base blades** first
- Then gained blades are added

### Heart Conditions
- Check **all stage members**, not just one
- Each member doesn't need to have all hearts individually

### Card Movement
- Cards moving from non-member/non-live areas to non-member/non-live areas are treated as **new cards**
- Effects don't carry over

### Automatic Abilities
- If cost is optional and not paid, ability **does not trigger**
- Can trigger multiple times if condition met multiple times
- State triggers (e.g., "when hand has no cards") trigger once when state occurs, then can trigger again after resolution

### Score
- Score icons (スコア) add to **total score**, not live card's base score
- Score modifiers apply to total score

## Engine Validation

### ✅ Correctly Implemented
- **Card types**: Member, Live, Energy properly distinguished
- **Areas**: All areas implemented (stage, deck, hand, discard, etc.)
- **Draw**: Implemented correctly with "draw_card" alias fix
- **Move cards**: Implemented correctly
- **Gain resource**: Implemented correctly
- **Change state**: Implemented correctly
- **Condition checking**: All condition types implemented
- **Card count conditions**: Working correctly (tested)

### ✅ Fully Implemented
- **Cheer (エール)**: ✅ Fully implemented in turn.rs execute_performance_phase (lines 1015-1184) - blade sum calculation, card movement to resolution area, blade heart icon checking, drawing, heart satisfaction all working
- **Live phase**: ✅ Fully implemented - live card set, performance phases (FirstAttackerPerformance, SecondAttackerPerformance), victory determination all working
- **Cost payment**: ✅ Fully implemented - baton touch fully implemented with proper cost reduction and member selection
- **Refresh**: ✅ Fully implemented in player.rs (refresh(), refresh_if_needed()) with deck empty handling
- **Position change**: ✅ Fully implemented in zones.rs (position_change()) with proper area swapping
- **Formation change**: ✅ Fully implemented in zones.rs (formation_change()) with duplicate area prevention
- **Baton touch**: ✅ Fully implemented - proper cost reduction, member selection from previous turn, energy gain logic
- **Blade heart icons**: ✅ Fully checked during cheer with proper counting and drawing
- **Heart satisfaction**: ✅ Fully implemented for live success with wildcard (ALL/Heart00) handling
- **Automatic ability timing**: ✅ Fully implemented - check timing system in turn.rs with proper sequencing
- **Rule Processing (Section 10)**: ✅ Fully implemented - Rule 10.1 (check timing), Rule 10.2 (refresh), Rule 10.3 (victory processing), Rule 10.4 (duplicate member processing), Rule 10.5 (invalid card processing with energy card handling), Rule 10.6 (invalid resolution zone processing)
- **Permanent Loop Detection (Rule 12.1)**: ✅ Fully implemented - game state history tracking, loop detection, draw result when loop detected
- **Keywords**: ✅ Fully implemented - Turn1, Turn2, Debut, LiveStart, LiveSuccess, Center, LeftSide, RightSide, PositionChange, FormationChange all working with proper tracking

### ⚠️ Partially Implemented / Needs Review
- **Continuous effects**: ✅ Partially implemented - TemporaryEffect system exists with basic effect layering (creation_order tracking, get_temporary_effects_in_order method). Full dependency resolution not implemented.
- **Replacement effects**: ✅ Partially implemented - ReplacementEffect struct and registration/checking logic implemented in game_state.rs and ability_resolver.rs. Choice-based replacement effects need player input support.

### ❓ Unknown / Needs Testing
- **Sequential effects**: Implemented but needs testing
- **Conditional alternative**: Implemented but needs testing
- **Look and select**: Implemented but needs testing
- **Look at**: Implemented but needs testing
- **Reveal**: Implemented but needs testing
- **Select**: Implemented but needs testing
- **Choice**: Basic implementation (always picks first), needs player input
- **Modify score**: Implemented but needs testing
- **Pay energy**: Implemented but needs testing
- **Discard until count**: Implemented but needs testing
- **Place energy under member**: Implemented but needs testing
- **Activation cost**: Implemented but needs testing
- **Play baton touch**: Stub implementation
- **Set card identity**: Stub implementation
- **Restriction**: Stub implementation
- **Re-yell**: Stub implementation
- **Modify cost**: Stub implementation

## Critical Issues

### 1. Replacement Effects
- **Replacement effects** partially implemented - basic registration and checking logic in place
- **Multiple replacement effects** handling simplified - applies first effect (Rule 9.10.2 requires player choice)
- **Choice-based replacement** needs player input support (Rule 9.10.3)
- **Replacement effect application order** defined but simplified implementation

### 2. Effect Layering
- **Continuous effects** partially layered - creation_order tracking and get_temporary_effects_in_order method added
- **Effect application order** basic layering implemented - effects sorted by creation_order (Rule 9.9.1.7)
- **Dependency resolution** not implemented - complex dependency resolution between effects not done

### 3. Remaining Issues
- **Player input for choice effects** - many effects always pick first option, need proper UI integration
- **Effect dependency resolution** - complex system for resolving effect dependencies not implemented

## Recommendations

### High Priority
1. Add **player input** for choice effects (currently always picks first)
2. Add **dependency resolution** for continuous effects
3. Implement full **replacement effect player choice** for multiple effects (Rule 9.10.2)

### Medium Priority
4. Add comprehensive tests for all ability types
5. Implement stub handlers (set_card_identity, restriction, re_yell, modify_cost)
6. Add proper error messages for rule violations

### Low Priority
7. Test sequential effects, conditional alternative, look and select, reveal, select
8. Test modify score, pay energy, discard until count, place energy under member, activation cost

## Conclusion

The engine has a solid foundation with most game mechanics fully implemented. The RULES_VALIDATION.md was significantly outdated - cheer, live phase, refresh, check timing, position change, formation change, baton touch, keywords (Turn1, Turn2, Debut, LiveStart, LiveSuccess, Center, LeftSide, RightSide, PositionChange, FormationChange), heart satisfaction, and automatic ability timing are all working correctly.

Recent updates have implemented:
1. Basic replacement effects system with registration and checking logic
2. Basic effect layering for continuous effects with creation_order tracking
3. Fixed PositionChange/FormationChange keyword placeholders with proper tracking
4. Full Rule Processing (Section 10) implementation including duplicate member processing, invalid card processing with energy card handling, and invalid resolution zone processing
5. Permanent loop detection (Rule 12.1) with game state history tracking

The condition system is well-implemented and working correctly. The basic action handlers (draw, move_cards, gain_resource, change_state) are functional. The main remaining gaps are:
1. Player input for choice effects (currently always picks first)
2. Full dependency resolution for continuous effects
3. Player choice for multiple replacement effects (Rule 9.10.2)
