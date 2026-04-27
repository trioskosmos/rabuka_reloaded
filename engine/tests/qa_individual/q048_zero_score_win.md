# Test Plan: Zero Score Win

## 1. Test Objective
Test that a player can win with zero score if opponent also has zero. This tests:
- Victory conditions with zero scores
- Tie-breaking rules (Rule 8.4.2)
- Score calculation when no score icons present
- Live success with no score contribution

## 2. Card Selection
- **Live Card 1:** A live card with no score icons
- **Live Card 2:** A live card with no score icons
- **Cheer Cards:** Cheer cards with no score icons
- **Why this selection:** Tests zero score scenario

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1 (no score), cheer_card_1 (no score)]
- `main_deck`: [energy cards...]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [live_card_2 (no score), cheer_card_2 (no score)]
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
- Expected: Both lives succeed (no score requirements)
- Expected: Player 1 score: 0 (no score icons)
- Expected: Player 2 score: 0 (no score icons)

**Step 4: Victory determination**
- Engine function called: `TurnEngine::determine_live_victory`
- Expected: Both scores are 0
- Expected: Tie-breaking rules applied (Rule 8.4.2)
- Expected: Winner determined by tie-breaking (e.g., first attacker wins on tie)

**Step 5: Verify winner**
- Expected: One player declared winner despite zero score
- Expected: Victory based on tie-breaking rules

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
- Score: 0
- Result: Winner or Loser (based on tie-breaking)

**Player 2:**
- `success_live_card_zone`: [live_card_2, cheer_card_2]
- Score: 0
- Result: Loser or Winner (based on tie-breaking)

## 7. Verification Assertions

- Zero score is correctly calculated when no score icons present
- Tie-breaking rules applied when scores are equal
- Winner declared even with zero score
- Live success possible with zero score
- Score calculation only includes score icons
- Victory determination follows Rule 8.4.2
