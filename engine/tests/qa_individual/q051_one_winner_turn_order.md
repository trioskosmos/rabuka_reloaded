# Test Plan: One Winner Turn Order

## 1. Test Objective
Test turn order when one player wins. This tests:
- Single victory conditions
- Turn order after single victory
- Live card handling on single victory
- Game end condition

## 2. Card Selection
- **Live Card 1:** A live card with high score
- **Live Card 2:** A live card with low score
- **Why this selection:** Creates single victory scenario

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1 (high score), cheer_card_1 (5 blades)]
- `main_deck`: [energy cards...]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [live_card_2 (low score), cheer_card_2 (1 blade)]
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
- Expected: Player 1: high score + 5 blades
- Expected: Player 2: low score + 1 blade

**Step 4: Victory determination**
- Engine function called: `TurnEngine::determine_live_victory`
- Expected: Player 1 wins (higher score)
- Expected: Player 2 loses
- Expected: GameResult::FirstAttackerWins or SecondAttackerWins

**Step 5: Verify turn order after victory**
- Expected: Game may end or continue based on rules
- Expected: Loser's turn order affected

**Step 6: Verify card handling**
- Expected: Winner's cards handled per victory rules
- Expected: Loser's cards handled per defeat rules

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
- Result: Loser

**Game State:** GameResult::FirstAttackerWins (or SecondAttackerWins), game may end

## 7. Verification Assertions

- Single victory condition correctly detected
- Correct player declared winner
- GameResult set correctly
- Turn order handles single victory correctly
- Game ends if victory is game-ending condition
- Winner's cards handled per victory rules
- Loser's cards handled per defeat rules
