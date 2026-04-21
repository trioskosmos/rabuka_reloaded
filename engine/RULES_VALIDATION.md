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

### ⚠️ Partially Implemented / Needs Review
- **Cheer (エール)**: Not fully implemented - needs blade sum calculation, card movement to resolution area, blade heart icon checking
- **Live phase**: Not implemented - needs live card set, performance, judgment
- **Cost payment**: Partially implemented - baton touch not fully implemented
- **Refresh**: Not implemented
- **Position change**: Basic implementation (only swaps center/left), needs full implementation
- **Formation change**: Not implemented
- **Baton touch**: Not fully implemented
- **Blade heart icons**: Not checked during cheer
- **Heart satisfaction**: Not implemented for live success
- **Automatic ability timing**: Not fully implemented - check timing system needed
- **Continuous effects**: Not fully implemented - effect layering system needed
- **Replacement effects**: Not implemented
- **Keywords**: Not implemented (Turn 1, Turn 2, 登場, ライブ開始時, etc.)

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

### 1. Missing Core Game Mechanics
- **Live phase** completely unimplemented
- **Cheer mechanics** unimplemented
- **Live success determination** unimplemented
- **Heart satisfaction** unimplemented
- **Score calculation** unimplemented

### 2. Ability Timing System
- **Check timing** not implemented
- **Automatic ability triggering** not properly sequenced
- **Rule processing** not separated from ability processing

### 3. Effect Layering
- **Continuous effects** not properly layered
- **Effect application order** not implemented (base → give/remove abilities → non-numeric continuous → set numeric → modify numeric)
- **Dependency resolution** not implemented

### 4. Replacement Effects
- **Replacement effects** not implemented
- **Multiple replacement effects** handling not implemented
- **Choice-based replacement** not implemented

### 5. Keywords
- **Position keywords** (center, left, right) not enforced
- **Turn limit keywords** (Turn 1, Turn 2) not implemented
- **Timing keywords** (登場, ライブ開始時, ライブ成功時) not implemented

### 6. Cost Payment
- **Baton touch** not fully implemented
- **Cost reduction** not properly validated
- **Energy under members** not handled correctly

### 7. Card Identity
- **Multi-character cards** counted as 1 member (need to verify)
- **Card version differences** need to be handled

### 8. Refresh
- **Deck empty handling** not implemented
- **Discard shuffling** not implemented

## Recommendations

### High Priority
1. Implement **live phase** with cheer mechanics
2. Implement **check timing system** for automatic abilities
3. Implement **continuous effect layering**
4. Implement **replacement effects**
5. Implement **keyword system** for position and timing restrictions

### Medium Priority
6. Complete **baton touch** implementation
7. Implement **refresh** mechanics
8. Complete **position change** implementation
9. Implement **formation change**
10. Add **player input** for choice effects

### Low Priority
11. Implement stub handlers (set_card_identity, restriction, re_yell, modify_cost)
12. Add comprehensive tests for all ability types
13. Implement effect dependency resolution
14. Add proper error messages for rule violations

## Conclusion

The engine has a solid foundation with most basic ability actions implemented. However, critical game mechanics (live phase, cheer, automatic ability timing, continuous effects) are missing or incomplete. The engine can currently handle simple abilities but cannot correctly simulate a full game of Rabuka.

The condition system is well-implemented and working correctly. The basic action handlers (draw, move_cards, gain_resource, change_state) are functional. The main gaps are in:
1. Game phase management
2. Ability timing and sequencing
3. Effect layering and replacement
4. Keyword enforcement
5. Live-specific mechanics
