# Winning Strategy Guide

## Game Objective
**Win Condition**: Have 3+ successful live cards while opponent has 2 or fewer

## Current Game State Analysis
- **Turn 1**: RockPaperScissors Phase
- **Critical Issue**: Game state not properly initialized (all zones show 0 cards)
- **Decks**: muse_cup.txt vs aqours_cup.txt

## Phase Flow & Strategic Considerations

### 1. RockPaperScissors Phase
- **Purpose**: Determine first attacker
- **Strategy**: Choose any option, both players must choose to proceed
- **Current Status**: Player 1 chose Rock, waiting for Player 2

### 2. Mulligan Phase
- **Purpose**: Exchange cards from hand for better options
- **Strategy**: Keep hand if it has playable low-cost members
- **Consideration**: Look for energy generation cards and low-cost members

### 3. Main Phase
- **Purpose**: Play members to stage using energy
- **Energy System**: 
  - Start with 3 energy cards
  - Draw 1 energy per turn
  - Energy cards must be active (not wait state) to use
- **Cost System**: Members have position-specific costs (Left/Center/Right)
- **Strategy**: 
  - Play low-cost members first (cost 2-4)
  - Save high-cost members for later turns
  - Consider member abilities and heart generation

### 4. Live Card Set Phase
- **Purpose**: Set live cards for performance
- **Strategy**: 
  - Set high-value live cards (MY, DREAM series)
  - Consider need_heart requirements vs member hearts
  - Can set up to 3 live cards

### 5. Performance Phase
- **Purpose**: Generate hearts and score points
- **Mechanics**:
  - Members contribute hearts based on their base_heart
  - Blade cards from deck provide additional hearts
  - Live cards succeed if need_heart requirements are met
- **Strategy**: 
  - Maximize heart generation from members
  - Choose live cards that match available hearts

## Key Problems Identified

### 1. Deck Initialization Failure
- **Issue**: All zones showing 0 cards despite deck files existing
- **Impact**: Cannot play game properly
- **Expected**: Each player should start with 6 cards in hand, 3 energy

### 2. Card Serialization Issue
- **Issue**: Cards showing as "cards" string instead of objects
- **Impact**: Cannot see card details, costs, abilities
- **Expected**: Cards should display name, cost, type, abilities

### 3. Energy Generation Not Working
- **Issue**: Energy zones remain empty
- **Impact**: Cannot play members to stage
- **Expected**: Should draw 1 energy per turn in Energy Phase

## Winning Strategy (Once Issues Fixed)

### Early Game (Turns 1-3)
1. **Mulligan**: Keep low-cost members (cost 2-4)
2. **Energy Phase**: Build energy base
3. **Main Phase**: Play 1-2 low-cost members
4. **Live Set**: Set 1-2 live cards with low heart requirements

### Mid Game (Turns 4-7)
1. **Energy Phase**: Maintain energy advantage
2. **Main Phase**: Play higher-cost members with better abilities
3. **Live Set**: Set 3 live cards for maximum scoring
4. **Performance**: Aim for 2+ successful live cards

### Late Game (Turns 8+)
1. **Maintain**: Keep stage full of active members
2. **Abilities**: Use member abilities to draw cards, generate hearts
3. **Win Condition**: Reach 3 successful live cards while opponent has 2-

## Ability Types to Watch For

### Draw Abilities
- **Text**: " cards to hand"
- **Value**: Card advantage, more options

### Heart Generation
- **Text**: " hearts" or heart icons
- **Value**: Helps meet live card requirements

### Energy Manipulation
- **Text**: "energy" or "activate"
- **Value**: Play more members per turn

### Card Retrieval
- **Text**: "from discard/zone to hand"
- **Value**: Recycle powerful cards

## Prediction System Accuracy

### Current Predictions
- **RPS choices**: 100% accurate (always succeed)
- **Member placement**: Cannot test (no energy/cards)
- **Live card set**: Cannot test (no cards in hand)

### Success/Failure Analysis
- **Success reasons**: Phase advancement, proper cost payment
- **Failure reasons**: Insufficient resources, invalid actions

## Next Steps
1. Fix deck initialization to get proper starting state
2. Resolve card serialization to see card details
3. Test energy generation mechanics
4. Verify ability implementation against text
5. Develop turn-by-turn optimal strategy
