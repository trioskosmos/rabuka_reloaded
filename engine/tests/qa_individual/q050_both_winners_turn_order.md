# Test Plan: Both Winners Turn Order

## 1. Test Objective
Test turn order when both players win (double victory). This tests:
- Double victory conditions
- Turn order after double victory
- Live card handling on double victory
- Game end condition

## 2. Card Selection
- **Live Card 1:** A live card with victory condition
- **Live Card 2:** A live card with victory condition
- **Why this selection:** Tests double victory scenario

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, cheer_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [live_card_2, cheer_card_2]
- `stage`: [member_card_3, member_card_4, -1, -1, -1]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Both players set live cards**
- Player 1 sets live_card_1
- Player 2 sets live_card_2
- Expected: Both live cards in respective live_card_zone

**Step 2: Both players cheer phase**
- Player 1 places cheer_card_1
- Player 2 places cheer_card_2
- Expected: Both cheer cards in respective success_live_card_zone

**Step 3: Execute live performance**
- Engine function called: Execute live performance
- Expected: Both lives succeed
- Expected: Both players meet victory conditions

**Step 4: Victory determination**
- Engine function called: `TurnEngine::determine_live_victory`
- Expected: Both players win (double victory)
- Expected: GameResult::Draw (both win)

**Step 5: Verify turn order after double victory**
- Expected: Game may end or continue based on rules
- Expected: Turn order handling per rule set

**Step 6: Verify card handling**
- Expected: Live cards handled per double victory rules
- Expected: Cheer cards handled per double victory rules

## 5. User Choices

**Choice 1 (Player 1 cheer selection):**
- Choice type: SelectCard
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_1
- Expected result: Card moved to success_live_card_zone

**Choice 2 (Player 2 cheer selection):**
- Choice type: SelectCard
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_2
- Expected result: Card moved to success_live_card_zone

## 6. Expected Final State

**Player 1:**
- `success_live_card_zone`: [live_card_1, cheer_card_1]
- Result: Winner

**Player 2:**
- `success_live_card_zone`: [live_card_2, cheer_card_2]
- Result: Winner

**Game State:** GameResult::Draw (both win), game may end

## 7. Verification Assertions

- Double victory condition correctly detected
- Both players declared winners
- GameResult::Draw set for double victory
- Turn order handles double victory correctly
- Game ends if double victory is game-ending condition
- Live cards handled per double victory rules
- Cheer cards handled per double victory rules
