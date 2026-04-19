# Love Live! Official Card Game - Comprehensive Analysis

## Table of Contents
1. [Game Overview](#game-overview)
2. [Card Information](#card-information)
3. [Player Information](#player-information)
4. [Game Zones](#game-zones)
5. [Game Initialization](#game-initialization)
6. [Turn Structure](#turn-structure)
7. [Live Phase Mechanics](#live-phase-mechanics)
8. [Abilities and Effects](#abilities-and-effects)
9. [Rule Processing](#rule-processing)
10. [Keywords](#keywords)

---

## Game Overview

### Game Format
- **Players**: 2 players (first attacker vs second attacker)
- **Victory Condition**: First player to have 3+ cards in Success Live Card Zone while opponent has 2- cards
- **Draw Condition**: Both players simultaneously have 3+ cards in Success Live Card Zone

### Game Termination
1. **Victory/Defeat**: Immediate game end when a player wins/loses
2. **Surrender**: Any player may surrender at any time (immediate defeat, not affected by card effects)
3. **Card-Induced Victory/Defeat**: Some cards can directly cause victory/defeat (immediate, no check timing)

### General Principles
1. **Card Text Priority**: Card text overrides rules when in conflict
2. **Impossible Actions**: If an action is impossible, it simply doesn't happen
3. **Already in State**: If already in a state, putting it in that state again does nothing
4. **Zero or Negative Values**: Actions with 0 or negative counts don't happen (no reverse actions)
5. **Multiple Impossible Requirements**: Do as many as possible
6. **Negative Values Allowed**: Player/card values can be 0 or negative unless specified otherwise
7. **Prohibition Priority**: Prohibiting effects always override enabling effects
8. **Simultaneous Choices**: Active player chooses first, non-active player knows the choice
9. **Number Selection**: Must be 0 or positive integer (no fractions or negatives)
10. **"Up to X"**: Can choose 0 unless specified otherwise

---

## Card Information

### Card Types
1. **Member (メンバー)**
   - Used for live card judgment
   - Has cost and heart icons
   - Played to member area

2. **Live (ライブ)**
   - Victory condition cards
   - Has score and required hearts
   - Judged during live phase

3. **Energy (エネルギー)**
   - Used to pay costs for members
   - Marked with "エネルギーカード" in text

### Card Components
1. **Card Name (カード名)**: Unique identifier, referenced by abilities
2. **Group Name (グループ名)**: Idol group (μ's, Aqours, Nijigasaki, Liella!, Hasunosora, etc.)
3. **Unit Name (ユニット名)**: Subunit (Printemps, BiBi, etc.)
4. **Cost (コスト)**: Energy cost to play member
5. **Blade Heart (ブレードハート)**: Icons for cheer processing
6. **Blade (ブレード)**: Number of cards to reveal during cheer
7. **Heart (ハート)**: Heart icons for live success judgment
8. **Score (スコア)**: Points gained on live success
9. **Required Heart (必要ハート)**: Hearts needed for live success
10. **Card Text (カードテキスト)**: Card abilities
11. **Illustration (イラスト)**: Visual (no game effect)
12. **Additional Info (付帯情報)**: Card number, illustrator, copyright

### Non-Ability Text Components

#### Card Data Structure (from cards.json)

Each card in the database contains the following fields:

```json
{
  "card_no": "PL!SP-bp2-009-R＋",
  "img": "https://llofficial-cardgame.com/wordpress/wp-content/images/cardlist/BP02/PL!SP-bp2-009-R2.png",
  "name": "鬼塚夏美",
  "product": "ブースターパックNEXTSTEP",
  "type": "メンバー",
  "series": "ラブライブ！スーパースター!!",
  "unit": "5yncri5e!",
  "cost": 13,
  "base_heart": {
    "heart02": 1,
    "heart03": 3,
    "heart06": 2
  },
  "blade_heart": {
    "b_heart03": 1
  },
  "blade": 1,
  "rare": "R＋",
  "ability": "{{live_start.png|ライブ開始時}}ライブ終了時まで、自分の手札2枚につき、{{icon_blade.png|ブレード}}を得る。\n{{live_success.png|ライブ成功時}}カードを2枚引き、手札を1枚控え室に置く。",
  "faq": [...]
}
```

#### Card Metadata Fields
1. **Card Number (カードナンバー)**: Unique identifier (e.g., "PL!SP-bp2-009-R＋")
   - Used for deck construction rules (max 4 copies per card number)
   - Format: Series-Package-Number-Rarity
   - Example breakdown:
     - `PL!`: Series code (Love Live! Superstar!!)
     - `SP`: Package type (Special/Booster Pack)
     - `bp2`: Package number (Booster Pack 2)
     - `009`: Card number
     - `R＋`: Rarity (Rare Plus)

2. **Product (製品)**: Booster pack or product name
   - Examples: "ブースターパックNEXTSTEP", "スターターデッキ"
   - Indicates which product the card belongs to
   - Used for deck building restrictions in some formats

3. **Series (シリーズ)**: Anime/series name
   - Examples: "ラブライブ！スーパースター!!", "ラブライブ！虹ヶ咲学園スクールアイドル同好会"
   - Identifies the Love Live! series the character belongs to
   - Referenced by abilities using `『series_name』`

4. **Unit (ユニット)**: Subunit name
   - Examples: "5yncri5e!", "Printemps", "BiBi"
   - Identifies the idol subunit
   - Referenced by abilities using `『unit_name』`

5. **Rarity (レア)**: Card rarity
   - Examples: "N", "R", "R＋", "SR", "SR＋", "PR", "SD"
   - Affects card distribution in products
   - Rarity levels:
     - N: Normal
     - R: Rare
     - R＋: Rare Plus
     - SR: Super Rare
     - SR＋: Super Rare Plus
     - PR: Promo
     - SD: Special Deck (starter deck exclusive)

6. **Image URL (img)**: Path to card artwork
   - Format: URL or local path
   - Used for display purposes, no game effect
   - Example: `https://llofficial-cardgame.com/wordpress/wp-content/images/cardlist/BP02/PL!SP-bp2-009-R2.png`

7. **Type (タイプ)**: Card type
   - "メンバー" (Member): Character cards played to stage
   - "ライブ" (Live): Victory condition cards
   - "エネルギー" (Energy): Resource cards for costs

#### Numerical Card Attributes
1. **Cost (コスト)**: Energy cost to play member
   - Range: Typically 1-15
   - Paid by activating energy cards
   - Can be reduced by Baton Touch

2. **Blade (ブレード)**: Number of cards to reveal during cheer
   - Range: Typically 0-3
   - Determines how many cards are moved from deck to resolution zone during live

3. **Base Heart (base_heart)**: Base heart composition
   - Object mapping heart colors to counts
   - Example: `{"heart02": 1, "heart03": 3, "heart06": 2}`
   - Used for live success calculation

4. **Blade Heart (blade_heart)**: Heart icons gained from cheer
   - Object mapping blade heart colors to counts
   - Example: `{"b_heart03": 1}` (1 yellow heart)
   - Added to live owned hearts during performance phase

5. **Score (スコア)**: Points gained on live success (Live cards only)
   - Range: Typically 1-5
   - Used to determine live winner

#### FAQ Text (FAQ)
- **Structure**: Array of FAQ entries
- **Each Entry Contains**:
  - **title**: Question identifier (e.g., "Q109（2025.05.30）")
  - **question**: The question text
  - **answer**: The answer text
  - **relation**: Related cards (card_no and name)
- **Purpose**: Clarifies card interactions and edge cases
- **Game Effect**: FAQ has no game effect, only clarifies rules

#### Icon References in Ability Text
Ability text uses icon references to denote triggers and effects:

**Trigger Icons**:
- `{{kidou.png|起動}}`: Activated ability
- `{{live_start.png|ライブ開始時}}`: Live Start automatic ability
- `{{live_success.png|ライブ成功時}}`: Live Success automatic ability
- `{{deploy.png|登場}}`: Deploy automatic ability
- `{{turn1.png|ターン1回}}`: Turn 1 limit keyword
- `{{turn2.png|ターン2回}}`: Turn 2 limit keyword

**Effect Icons**:
- `{{icon_blade.png|ブレード}}`: Blade icon
- Energy icons (represented by `{{icon_energy.png|エネルギー}}` or similar)

**Format**: `{{filename|display_text}}`
- `filename`: Image file name
- `display_text`: Text displayed in ability

#### Text Formatting in Ability Text
- **Newline**: `\n` separates multiple abilities
- **Colon `:`**: Separates cost from effect in activated abilities
- **Parentheses `（）`**: Used for explanatory notes (注釈文)
  - These are clarifications, not game-affecting text
- **Brackets `『』`**: Group or unit name references
- **Quotation marks `「」`**: Card name references
- **Slash `/`**: Indicates ability is both types (e.g., `A / B`)

#### Card Name Variations
- **Single Name**: Most cards have one character name
  - Example: "鬼塚夏美"
- **Multiple Names**: Some cards have `&` in name
  - Example: "高坂穂乃果＆絢瀬絵里"
  - Each name separated by `&` is a separate member name
  - Each part has corresponding group name
- **Name with Title**: Some cards include honorifics or titles
  - Example: "桜小路きな子"

#### Text References in Abilities
- **Card Name Reference**: `「name」`
  - Refers to cards with that exact name
  - Example: `「鬼塚夏美」` refers to cards named "鬼塚夏美"
- **Group Name Reference**: `『group_name』`
  - Refers to cards with that group name
  - Example: `『Liella!』` refers to all Liella! group cards
- **Unit Name Reference**: `『unit_name』`
  - Refers to cards with that unit name
  - Example: `『5yncri5e!』` refers to all 5yncri5e! unit cards
- **Partial Name Match**: `「partial」`
  - Refers to cards with that partial name
  - Example: `「夏美」` could match "鬼塚夏美"

#### Non-Ability Text Usage in Game
1. **Deck Construction**: Card number limits (max 4 per number)
2. **Card Identification**: Name, group, unit for ability targeting
3. **Cost Calculation**: Cost field for member play
4. **Live Calculation**: Score, base_heart, blade for live success
5. **Visual Display**: Image, rarity for UI
6. **Rule Clarification**: FAQ for edge cases
7. **Series/Unit Matching**: For ability references

### Heart Icons
- **Heart01-Heart06**: Pink, Red, Yellow, Green, Blue, Purple
- **Heart00**: Wild (can be any color)
- Multiple stacked icons = multiple of that color
- Blade heart icons count as regular hearts (not blades)

### Required Hearts
- Represented by heart notes (vertical icon)
- Each note has color (top) and count (bottom)
- Multiple notes = all must be satisfied simultaneously
- **Satisfaction Conditions**:
  1. For each non-wild note, have that color heart >= required count
  2. Total heart count >= sum of all required counts

---

## Player Information

### Owner vs Master
- **Owner**: Physical owner of the card (player who put it in their deck)
- **Master**: Current user of the card/ability/effect
  - Card in zone: Master is the zone's owner
  - Constant ability: Master is the card/ability owner
  - Activated ability: Master is the player who played it
  - Automatic ability: Master is the card/ability owner
  - Effect: Master is the ability's master

---

## Game Zones

### Zone Properties
- **Public Zones**: All players can see card content
- **Private Zones**: Only specified player can see content
- **Ordered Zones**: Card order is managed (cannot change unless specified)
- **Unordered Zones**: Card order is not managed

### Zone List

#### 1. Stage (ステージ)
- **Composition**: 3 Member Areas (Left Side, Center, Right Side)
- **Visibility**: Public
- **Order**: Unordered
- **Card State**: Orientation (Active/Wait)
- **Energy Stacking**: Energy cards can be stacked under members
- **Movement**: Energy under member moves with member to other member areas
- **Exit**: When member leaves stage, energy stays then moves to energy deck via rule processing

#### 2. Member Area (メンバーエリア)
- **Types**: Left Side (左サイドエリア), Center (センターエリア), Right Side (右サイドエリア)
- **Visibility**: Public
- **Order**: Unordered
- **Card State**: Orientation (Active/Wait)
- **Note**: "Area" in text refers to Member Area

#### 3. Live Card Zone (ライブカード置き場)
- **Purpose**: Place live cards during live phase
- **Visibility**: Public (but cards can be temporarily face-down)
- **Order**: Unordered

#### 4. Energy Zone (エネルギー置き場)
- **Purpose**: Place energy cards
- **Visibility**: Public
- **Order**: Unordered
- **Card State**: Orientation (Active/Wait)
- **Note**: "Energy" in text refers to Energy Zone cards

#### 5. Main Deck Zone (メインデッキ置き場)
- **Purpose**: Place main deck
- **Visibility**: Private
- **Order**: Ordered (stack)
- **Movement**: Move 1 card at a time when moving multiple
- **Note**: "Deck" in text refers to Main Deck Zone

#### 6. Energy Deck Zone (エネルギーデッキ置き場)
- **Purpose**: Place energy deck
- **Visibility**: Private
- **Order**: Unordered
- **Movement**: Move 1 card at a time when moving multiple
- **Note**: "Energy Deck" in text refers to Energy Deck Zone

#### 7. Success Live Card Zone (成功ライブカード置き場)
- **Purpose**: Place successful live cards
- **Visibility**: Public
- **Order**: Ordered (stack on top of existing cards)

#### 8. Hand (手札)
- **Purpose**: Hold unused cards
- **Visibility**: Private (only owner can see their own)
- **Order**: Unordered
- **Note**: "X cards in hand" is written as "hand X cards"

#### 9. Waitroom (控え室)
- **Purpose**: Discard pile for used cards
- **Visibility**: Public
- **Order**: Unordered

#### 10. Exclusion Zone (除外領域)
- **Purpose**: Removed from game cards
- **Visibility**: Public (default face-up unless specified)
- **Card State**: Face state (Face-up/Face-down)

#### 11. Resolution Zone (解決領域)
- **Purpose**: Temporary placement during game progression
- **Visibility**: Public
- **Order**: Unordered
- **Note**: Shared by both players (only 1 exists)

### Zone Reference Rules
- If zone owner not specified, refers to card master's zone
- Card moving from member area/live card zone to elsewhere = new card in new zone (previous effects don't apply)
- Exception: If effect explicitly references the moved card in new zone, can use new zone reference
- Card moving to non-owner zone = goes to owner's zone

---

## Game Initialization

### Deck Preparation (Rule 6.1)
**Main Deck Requirements**:
- Exactly 48 Member cards
- Exactly 12 Live cards
- Maximum 4 copies of each card number

**Energy Deck Requirements**:
- Exactly 12 Energy cards

**Deck Building Abilities**:
- Constant abilities that modify deck construction conditions are replacement effects
- Become invalid after game start

### Pre-Game Procedure (Rule 6.2)
1. **Deck Presentation**: Each player shows their deck
2. **Main Deck Setup**: 
   - Place main deck in Main Deck Zone
   - Shuffle
3. **Energy Deck Setup**: Place energy deck in Energy Deck Zone
4. **First Attacker Selection**: Randomly choose a player to determine first attacker
5. **Initial Draw**: Each player draws 6 cards from main deck to hand
6. **Mulligan** (in order, first attacker first):
   - Choose any number of cards from hand
   - Place them face-down aside
   - Draw same number of cards from main deck
   - Place aside cards back into main deck
   - If 1+ cards were returned, shuffle main deck
7. **Initial Energy**: Each player draws 3 cards from energy deck to Energy Zone

---

## Turn Structure

### Turn Overview
- Game progresses through repeated "Turns"
- Each turn has: First Attacker and Second Attacker
- Turn phases (in order):
  1. First Attacker Normal Phase
  2. Second Attacker Normal Phase
  3. Live Phase

### Normal Phase Structure
Each Normal Phase consists of:
1. **Active Phase** (アクティブフェイズ)
2. **Energy Phase** (エネルギーフェイズ)
3. **Draw Phase** (ドローフェイズ)
4. **Main Phase** (メインフェイズ)

### Active Player Definition
- **Turn Player Designated Phases**: Turn player is active player
- **Simultaneous Phases**: First attacker is active player
- **Non-active Player**: The other player

---

### Phase 1: Active Phase (Rule 7.4)

**Steps**:
1. **Activate All**: Turn player activates all wait cards in:
   - Energy Zone
   - Member Areas (all three)
2. **Trigger Conditions**: 
   - "At the start of turn" conditions trigger
   - "At the start of active phase" conditions trigger
   - If this is the first turn of the game, "At the start of game" conditions trigger
3. **Check Timing**: Execute check timing (see Rule Processing section)
4. **End Phase**: Active phase ends after check timing completes

---

### Phase 2: Energy Phase (Rule 7.5)

**Steps**:
1. **Trigger Conditions**: "At the start of energy phase" conditions trigger
2. **Check Timing**: Execute check timing
3. **Draw Energy**: Turn player draws 1 card from Energy Deck to Energy Zone
4. **Check Timing**: Execute check timing
5. **End Phase**: Energy phase ends after check timing completes

---

### Phase 3: Draw Phase (Rule 7.6)

**Steps**:
1. **Trigger Conditions**: "At the start of draw phase" conditions trigger
2. **Check Timing**: Execute check timing
3. **Draw Card**: Turn player draws 1 card from Main Deck to Hand
4. **Check Timing**: Execute check timing
5. **End Phase**: Draw phase ends after check timing completes

---

### Phase 4: Main Phase (Rule 7.7)

**Steps**:
1. **Trigger Conditions**: "At the start of main phase" conditions trigger
2. **Check Timing**: Execute check timing
3. **Play Timing**: Turn player receives play timing - can do:
   - Play one of their card's activated abilities
   - Play one member card from hand
4. **End Phase**: Main phase ends (player can continue until they choose to stop)

**Playing a Member Card**:
1. **Specify Card**: Choose member card from hand
2. **Specify Area**: Choose one member area
   - **Restriction**: Cannot choose an area where a member moved from non-stage to stage this turn
3. **Reveal**: If card is in private zone, reveal it and move to Resolution Zone
4. **Choose Options**: If card/ability requires choices, make them
5. **Pay Cost**: 
   - Cost = "Pay energy equal to card's cost"
   - **Baton Touch**: If paying 1+ energy, can instead move 1 member from chosen area to Waitroom
     - Reduces energy cost by that member's cost
     - Triggers "Baton Touch" event
6. **Resolve**: Place card in specified member area

---

### Phase 5: Live Phase (Rule 7.8, 8)

**Structure**:
1. **Live Card Set Phase** (Rule 8.2)
2. **First Attacker Performance Phase** (Rule 8.3)
3. **Second Attacker Performance Phase** (Rule 8.3)
4. **Live Victory Determination Phase** (Rule 8.4)

---

## Live Phase Mechanics

### Live Card Set Phase (Rule 8.2)

**Steps**:
1. **Trigger Conditions**: 
   - "At the start of live phase" conditions trigger
   - "At the start of live card set phase" conditions trigger
2. **Check Timing**: Execute check timing
3. **First Attacker Sets**:
   - Choose up to 3 cards from hand
   - Place them face-down in Live Card Zone
   - Draw same number of cards from Main Deck
4. **Check Timing**: Execute check timing
5. **Second Attacker Sets**:
   - Choose up to 3 cards from hand
   - Place them face-down in Live Card Zone
   - Draw same number of cards from Main Deck
6. **Check Timing**: Execute check timing
7. **End Phase**: Live card set phase ends after check timing completes

---

### Performance Phase (Rule 8.3)

**Overview**: Turn player executes live procedures

**Steps**:
1. **Trigger Conditions**: "At the start of performance phase" conditions trigger
2. **Check Timing**: Execute check timing
3. **Reveal Cards**: Turn player:
   - Flips all cards in Live Card Zone face-up
   - Moves all non-live cards to Waitroom
   - **If "Cannot Live" state**: Move all face-up cards to Waitroom
4. **Check Timing**: Execute check timing
5. **Check for Live Cards**: If no live cards in Live Card Zone, end phase
6. **Execute Live** (if live cards exist):
   - **Live Start Event**: Triggers "Live Start" automatic abilities
   - **Check Timing**: Execute check timing
   - **Calculate Total Blades**: Sum of blades from all active members
   - **Execute Cheer (エール)**: 
     - Repeat [total blades] times:
       - Move top card of Main Deck to Resolution Zone
   - **Check Blade Hearts**: For each heart icon in Resolution Zone cards, draw 1 card
   - **Check Timing**: Execute check timing
   - **Calculate Live Owned Hearts**:
     - Sum of all heart icons from:
       - All members
       - Blade hearts from cheer (only turn player's cards)
   - **Check Required Hearts**: For each live card in Live Card Zone:
     - Check if current Live Owned Hearts satisfy its Required Hearts
     - **Wild Hearts (Heart00)**: Can be any single color
     - If satisfied: Subtract required hearts from Live Owned Hearts
   - **Live Failure**: If any live card's required hearts not satisfied:
     - Move all live cards in Live Card Zone to Waitroom
7. **Check Timing**: Execute check timing
8. **End Phase**: Performance phase ends after check timing completes

---

### Live Victory Determination Phase (Rule 8.4)

**Steps**:
1. **Trigger Conditions**: "At the start of live determination phase" conditions trigger
2. **Check Timing**: Execute check timing
3. **Calculate Scores**: Each player with cards in Live Card Zone:
   - Sum score of all live cards
   - Add 1 point for each cheer icon
4. **Compare Scores**:
   - **Both empty**: Scores are equal
   - **One has cards, other empty**: Card player has higher score
   - **Both have cards**: Compare actual scores
5. **Trigger Live Success**: Players with cards in Live Card Zone trigger "Live Success"
6. **Check Timing**: Execute check timing
7. **Determine Winner**:
   - **Both empty**: No winner
   - **One has cards**: Higher score player wins (or both if equal)
   - **Both have cards**: Higher score wins (or both if equal)
8. **Winner Moves Card**: Winner chooses 1 live card from Live Card Zone, moves to Success Live Card Zone
   - **Exception**: If both won and player has 2 cards in Live Card Zone, don't move
9. **Cleanup**: Each player moves:
   - Remaining live cards to Waitroom
   - Cheer-revealed cards to Waitroom
10. **Check Timing**: Execute check timing
11. **Trigger "At End of Turn"**: Automatic abilities with "At end of turn" trigger (if not already triggered this turn)
12. **Check Timing**: Execute check timing
13. **End Effects**: Effects with duration "Until end of turn" or "Until end of live" expire
14. **Loop Check**: If new automatic abilities triggered or rule processing occurred, go back to step 9
15. **Update First/Second Attacker**:
    - If only one player moved card to Success Live Card Zone: They become first attacker
    - Otherwise: Current first attacker remains
16. **End Turn**: Turn ends

---

## Abilities and Effects

### Ability Types (Rule 9.1)

#### 1. Activated Ability (起動能力)
- **Definition**: Ability that player actively executes by paying cost during play timing
- **Card Text Format**: `{{icon}} (condition): (effect)`
- **Condition**: Requirement to play the ability
- **Effect**: What happens when ability resolves

#### 2. Automatic Ability (自動能力)
- **Definition**: Ability that automatically plays when specified event occurs
- **Card Text Format**: 
  - `{{icon}} (effect)` - triggers on specified event
  - `{{icon}} (condition): (effect)` - triggers when condition is met
- **Trigger Condition (誘発条件)**: Event that causes ability to trigger
- **Triggered (誘発している)**: Trigger condition is met
- **Activate/Trigger (発動する)**: Ability is triggered

#### 3. Constant Ability (常時能力)
- **Definition**: Ability that is always active while valid
- **Card Text Format**: `{{icon}} (effect)`
- **Never Played**: Always active, never needs to be played

---

### Effect Types (Rule 9.2)

#### 1. One-Time Effect (単発効果)
- **Definition**: Effect that executes once during resolution then ends
- **Examples**: "Draw 1 card", "Put this character in waitroom"

#### 2. Continuous Effect (継続効果)
- **Definition**: Effect that remains active for a specified duration (including "this game")
- **Duration**: Can be specified or indefinite ("this game")

#### 3. Replacement Effect (置換効果)
- **Definition**: When an event occurs, replace it with a different event
- **Card Text Format**: "When (action A), instead do (action B)"
- **Optional Replacement**: "When (action A), instead [may] do (action B). If so, do (action B)"
- **Result**: Original event never occurs

---

### Valid vs Invalid Abilities (Rule 9.3)

**Invalid Effect**:
- Part or all of effect is invalid under certain conditions
- Effect exists as ability but doesn't trigger
- If effect requires choice, don't make choice

**Valid Effect**:
- Part or all of effect is valid under certain conditions
- If condition not met, that part is invalid

**Default Validity**:
- Abilities valid in specific area/situation = valid there
- Member cards: Valid while in Member Area
- Live cards: Valid while in Live Card Zone

---

### Cost and Payment (Rule 9.4)

**Cost Definition**: Actions before `:` in activated/automatic abilities

**Payment**: Execute actions specified by cost

**Payment Rules**:
1. Execute from beginning to end
2. If any part impossible to pay, cannot pay at all
3. Energy icons = "Pay energy"

---

### Check Timing and Play Timing (Rule 9.5)

#### Check Timing (チェックタイミング)
- **Definition**: Time to execute rule processing and play automatic abilities
- **Execution Order**:
  1. Execute all rule processing simultaneously
  2. If new rule processing generated, repeat
  3. Play/resolve 1 automatic ability owned by active player (choose which)
  4. Go back to step 1
  5. If active player has no more abilities, play/resolve 1 owned by non-active player
  6. Go back to step 1
  7. If no more abilities, end check timing
- **Automatic Ability Play**: Mandatory, cannot choose not to play
- **Multiple Waiting**: Can choose order, but must play one

#### Play Timing (プレイタイミング)
- **Definition**: Time when player can actively perform actions
- **Execution**:
  1. Check timing occurs first
  2. After check timing complete, actual play timing given
  3. Player chooses to:
     - Perform action (if action taken, play timing given again)
     - Do nothing (play timing ends, phase advances)
- **Before Action**: Check timing always occurs before player can act

---

### Play and Resolution (Rule 9.6)

**Play Procedure**:
1. **Specify**: Choose card/ability to play
   - If card in private zone: Reveal and move to Resolution Zone
   - If member: Specify member area
     - **Restriction**: Cannot choose area where member moved from non-stage to stage this turn
   - If ability: Move to Resolution Zone as pseudo-card
2. **Choose**: Make any required choices
3. **Pay Cost**: Determine and pay all costs
   - **Member Cost**: "Pay energy equal to card's cost"
   - **Baton Touch**: If paying 1+ energy, can move 1 member from area to Waitroom instead
4. **Resolve**: Execute resolution
   - **Member**: Place in specified member area
   - **Ability**: Execute effect
   - **Ability Resolution**: Even if original card gone, ability still resolves

**Selection Rules**:
- "Choose X": Must choose X if possible
- "Choose up to X": Can choose 0 to X
- If cannot choose specified number, cannot play
- If cannot choose any, none chosen, related effects ignored
- **Private Zone Selection**: Not guaranteed to have matching cards, can choose not to

---

### Automatic Ability Processing (Rule 9.7)

**Waiting State**: When trigger condition met, ability enters waiting state

**Multiple Triggers**: Each trigger = 1 waiting state

**Check Timing Play**: Player chooses 1 waiting ability to play, then 1 waiting state removed

**Mandatory Play**: Must play, cannot choose not to
- **Exception**: If ability has optional cost payment, can choose not to pay and not play

**Cannot Play**: If cannot play, waiting state removed (1 count)

**Area Move Trigger (領域移動誘発)**: Abilities that trigger on card area movement
- **Public to Private/Private to Public**: Use public zone state
- **Stage to Non-Stage/Owner Change**: Use stage state
- **Public to Public**: Use post-movement state

**Entry Trigger**: Card with area move trigger enters area at same time as another card triggers that area's move trigger = trigger considered met

**Time Limit Trigger (時限誘発)**: Ability created to trigger at future time
- Default: Triggers once only

**State Trigger (状態誘発)**: Triggers on condition being met (not event occurring)
- Triggers once when state first occurs
- After resolution, if condition still met, triggers again

**Zone Change**: Even if card's zone changed during waiting, must still play

---

### One-Time Effect Processing (Rule 9.8)
- Execute specified action once

---

### Continuous Effect Processing (Rule 9.9)

**Application Order**:
1. **Base Value**: Card's printed information is baseline
2. **Grant/Lose/Enable/Disable**: Apply effects that grant/lose/enable/disable abilities
3. **Non-Numerical**: Apply all continuous effects that don't change numerical values
4. **Set to Specific**: Apply effects that set numerical values to specific numbers
   - Includes setting heart/blade counts
5. **Numerical Changes**: Apply effects that add/subtract numerical values
   - Includes adding/subtracting heart/blade counts
6. **Dependency**: If effect A determines what effect B applies to, B depends on A
   - Dependent effects always processed after what they depend on
7. **Creation Order**: Effects without defined order processed in creation order
   - **Constant Ability**: Time card placed in current zone
   - **Other Abilities**: Time ability was played

**Zone Exit**: Non-constant continuous effects don't apply to cards that moved from Member Area to elsewhere

**Zone Entry**: Continuous effects that change zone info apply when card enters zone

**Entry Trigger**: Automatic abilities that trigger on card with specific info entering zone apply after continuous effects applied

---

### Replacement Effect Processing (Rule 9.10)

**Application**: When event occurs that replacement effect applies to, don't execute original event, execute replacement instead

**Result**: Original event never occurred

**Multiple Replacements**: If multiple replacement effects apply to same event, affected player chooses order
- **Card/Ability**: Master chooses
- **Game Action**: Action executor or card master chooses
- **Limit**: Each replacement effect can apply max once to same event

**Optional Replacement**: If cannot execute optional choice, cannot apply replacement effect

---

### Final Information (Rule 9.11)
- If effect references card info and card moved from area during effect execution, effect uses card's last state in that area

---

### Source (Rule 9.12)
- **Ability Source**: Card with ability, or card that created time-limit trigger
- **Effect Source**: Ability's source

---

## Rule Processing (Rule 10)

### Overview
- Automatic processing triggered by specific events occurring/being present
- Except Refresh, only checked during Check Timing

### Refresh (リフレッシュ) (Rule 10.2)
- **Exception**: Can occur anytime, not just check timing
- **Interrupts**: If occurs during other action, interrupt, execute refresh, then continue
- **Trigger Conditions**:
  1. Main Deck empty AND Waitroom has cards
  2. "Look at top X cards of Main Deck" instruction AND Main Deck has < X cards
- **Execution**: 
  - Shuffle Waitroom cards (private state)
  - Move all to Main Deck (under existing cards if any)
- **Simultaneous**: If both players meet condition, current first attacker refreshes first

### Victory Processing (勝利処理) (Rule 10.3)
- **Trigger**: Any player has 3+ cards in Success Live Card Zone
- **Result**: That player wins the game

### Duplicate Member Processing (重複メンバー処理) (Rule 10.4)
- **Trigger**: Any member area has multiple members
- **Result**: Keep most recently placed member, send others to owner's Waitroom

### Invalid Card Processing (不正カード処理) (Rule 10.5)
- **Trigger 1**: Live Card Zone has face-up non-live card
  - **Result**: Send to owner's Waitroom
- **Trigger 2**: Energy Zone has non-energy card
  - **Result**: Send to owner's Waitroom
- **Trigger 3**: Member Area has energy card without member on top
  - **Result**: Send to Energy Deck Zone
- **Energy Card Exception**: If card to send to Waitroom is energy, send to Energy Deck Zone instead

### Invalid Resolution Zone Processing (不正解決領域処理) (Rule 10.6)
- **Trigger**: Resolution Zone has cards that are not:
  - Currently being played
  - Currently being resolved
  - Currently being cheered (エール処理中)
- **Result**: Send to owner's Waitroom

---

## Keywords (Rule 11)

### Overview
- Keywords abbreviate specific ability patterns
- **Format**: `【Automatic】(effect)` or `【Automatic】(cost):(effect)` for automatic abilities with cost
- **Format**: `A / B` means ability is both A and B

### Turn 1 Limit (ターン1回) (Rule 11.2)
- **Definition**: Limits ability play
- **Effect**: If same ability already played this turn, cannot play again
- **Format**: `A / B` means if either condition used to play, cannot use other

### Turn 2 Limit (ターン2回) (Rule 11.3)
- **Definition**: Limits ability play
- **Effect**: If same ability played 2 times this turn, cannot play again

### Deploy (登場) (Rule 11.4)
- **Definition**: Automatic ability that triggers on member entering member area
- **Format**: `【Deploy】(effect)` = `【Automatic】When this card enters member area from outside member area, (effect)`

### Live Start (ライブ開始時) (Rule 11.5)
- **Definition**: Automatic ability that triggers on live starting
- **Format**: `【Live Start】(effect)` = `【Automatic】When you are turn player during live start of performance phase, (effect)`
- **Exception**: If no live cards after revealing, live start event doesn't occur

### Live Success (ライブ成功時) (Rule 11.6)
- **Definition**: Automatic ability that triggers on live success
- **Format**: `【Live Success】(effect)` = `【Automatic】When your live succeeds, (effect)`

### Center (センター) (Rule 11.7)
- **Definition**: Limits ability play/triggering/validity to center area
- **Activated**: Can only activate if member in center area
- **Automatic**: Only triggers if member in center area
- **Constant**: Only valid if member in center area

### Left Side (左サイド) (Rule 11.8)
- **Definition**: Limits ability play/triggering/validity to left side area
- **Activated**: Can only activate if member in left side area
- **Automatic**: Only triggers if member in left side area
- **Constant**: Only valid if member in left side area

### Right Side (右サイド) (Rule 11.9)
- **Definition**: Limits ability play/triggering/validity to right side area
- **Activated**: Can only activate if member in right side area
- **Automatic**: Only triggers if member in right side area
- **Constant**: Only valid if member in right side area

### Position Change (ポジションチェンジ) (Rule 11.10)
- **Definition**: Move member to different member area
- **Effect**: If member already in target area, that member moves to original area

### Formation Change (フォーメーションチェンジ) (Rule 11.11)
- **Definition**: Move all members on stage to different areas
- **Restriction**: Cannot move 2+ members to same area

---

## Other Rules (Rule 12)

### Infinite Loop (永久循環) (Rule 12.1)
- **Definition**: Situation where action can be performed infinitely or must be performed infinitely
- **Resolution**:
  1. Active player shows sequence of actions in loop and how many times
  2. Non-active player chooses to:
     - Accept that many times
     - Force fewer times + do different action
  3. Execute according to choice
- **Once-Per-Turn**: If player performs action and game returns to identical state later, cannot perform same action again
- **No Escape**: If neither player can stop loop, game ends in draw

---

## Appendix Information

### Group Names (作品名一覧)
- μ's
- Aqours
- Nijigasaki (虹ヶ咲)
- Liella!
- Hasunosora (蓮ノ空)
- A-RISE
- Saint Snow
- Sunny Passion

### Unit Names (ユニット名称)
- Printemps, BiBi, lily white
- CYaRon!, AZALEA, Guilty Kiss
- QU4RTZ, A・ZU・NA, DiverDiva
- R3BIRTH, CatChu!, KALEIDOSCORE
- 5yncri5e!
- Series Bouquet, DOLLCHESTRA
- Mirakura Park!, Edel Note, AiScReam

### Member List (メンバー一覧)
(Complete list of all members with their groups - 58 members total across all groups)

---

## Summary of Game Flow

1. **Initialization**: Deck setup, mulligan, initial energy
2. **Turn Loop** (repeated):
   - **Active Phase**: Activate cards, trigger turn start
   - **Energy Phase**: Draw energy
   - **Draw Phase**: Draw card
   - **Main Phase**: Play abilities/members
   - **Live Phase**:
     - Set live cards
     - First attacker performs live
     - Second attacker performs live
     - Determine winner, move to success zone
   - **Check Victory**: If 3+ success cards, game ends
3. **Rule Processing**: Continuous check for refresh, victory, invalid cards, etc.
