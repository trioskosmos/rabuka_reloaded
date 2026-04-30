# Game Mechanics Documentation

## Current Observations

### Game Flow
1. **RockPaperScissors Phase**: Both players must choose their RPS option before proceeding
2. **First Attacker Choice**: After RPS, winner chooses who goes first
3. **Main Phase**: Players can play members to stage (requires energy)
4. **LiveCardSet Phase**: Players set live cards
5. **Turn progression**: Each turn consists of Main phases for both players

### Key Issues Identified
- **Energy System**: Players have 0 energy in main phase, cannot play members
- **Card Display**: Hand shows "cards" string instead of actual card objects
- **Stage System**: Stage shows 3/3 members initially but they're all -1 (empty)

### Game State Structure
- **Hand**: Currently showing placeholder strings instead of card objects
- **Energy Zone**: Empty (0 cards) - need to understand how to generate energy
- **Stage**: Array of 3 positions, -1 means empty
- **Deck**: 0 cards after initialization
- **Life Zone**: 0 cards
- **Waiting Room**: 0 cards

### Cost System
- Members have costs like "Left: 2, Center: 2, Right: 2"
- Need to understand energy generation to pay these costs

### Energy System (from rules.txt)
- **Energy Phase**: Each turn, player draws 1 energy card from energy deck to energy zone
- **Active Phase**: All energy cards in energy zone and member areas are set to active
- **Cost Payment**: To play members, pay energy by setting energy cards to "wait" state
- **Energy Under Members**: Energy cards can be placed under member cards for support

### Deck Initialization (from rules.txt)
- **Main Deck**: 48 member cards + 12 live cards = 60 cards total
- **Energy Deck**: 12 energy cards
- **Setup**: Draw 6 cards to hand, draw 3 energy cards to energy zone
- **Mulligan**: Can exchange any number of cards from hand

### Next Steps to Investigate
1. Why are cards showing as "cards" string instead of objects?
2. What triggers deck initialization in current implementation?
3. How do abilities interact with the game state?
4. Verify energy generation is working properly

### Current Game Observations (Turn 1)
- **Phase Flow**: RPS -> Mulligan -> Main -> LiveCardSet -> Performance
- **Decks Used**: muse_cup.txt vs aqours_cup.txt
- **Critical Issues Identified**:
  1. Cards showing as "cards" string instead of objects
  2. All zones showing 0 cards (deck, hand, energy, stage)
  3. RPS phase requires both players to choose before proceeding
  4. Game state not properly initializing from deck files

### Action Predictions System
- **play_member_to_stage**: Checks energy vs cost requirements
- **set_live_card**: Should always succeed
- **pass**: Should always succeed
- **RPS choices**: Required for both players to proceed

### Turn Limit
- Game stops at turn 10 to avoid infinite loops
- Most games should conclude before this limit

### Problems to Fix
1. **Deck Loading**: Deck files not being read properly
2. **Card Serialization**: Cards becoming strings instead of objects
3. **Game Initialization**: Starting state not set up correctly
4. **Energy Generation**: Energy phase not working
5. **RPS Phase Loop**: Game stuck in RockPaperScissors despite both players choosing

### Critical Issue: RPS Phase Not Advancing
**Observation**: Both players have made RPS choices (P1: Rock, P2: Scissors) but game remains in RockPaperScissors phase
**Expected**: After both players choose, game should advance to ChooseFirstAttacker or Mulligan phase
**Current**: Actions still show RPS choices, game state unchanged
**Possible Causes**:
- Server not tracking both player choices properly
- Phase transition logic missing or broken
- Game initialization preventing proper phase flow
- Web server expecting different action format for phase completion

### Prediction System Accuracy Update
- **RPS choices**: Successfully execute but don't advance phase (unexpected)
- **Phase advancement**: Predictions incorrect - game not progressing as expected
- **Game state**: Consistently shows 0 cards in all zones (critical issue)

### Ability Testing Status
- **Current**: No abilities visible due to card serialization issue
- **Impact**: Cannot test ability implementation vs text
- **Priority**: Must fix deck/card loading before ability testing
