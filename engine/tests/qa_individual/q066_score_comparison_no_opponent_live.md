# Q066: Score Comparison with No Opponent Live Card

## Test Objective
Test that when comparing live scores, if player has a live card and opponent has no live card, the player's score is considered higher regardless of actual values.

## Q&A Reference
**Question:** If you have a live card in your live card zone and opponent has no live card in their live card zone, is the condition "live total score is higher than opponent's" satisfied?
**Answer:** Yes, it is satisfied. When you have a live card and opponent doesn't, your live total score is treated as higher than opponent's regardless of the actual values.

## Card Selection
**Primary Card:** PL!N-bp1-026-L "Poppin' Up!" or PL!SP-bp1-023-L "START!! True dreams" - cards with "ライブの合計スコアが相手より高い場合" condition

## Initial Game State

**Player 1:**
- `live_card_zone`: [live_card_id] (has live card)
- `stage`: Minimal setup (1 member with low score)
- `success_live_card_zone`: Empty
- Other zones: Standard setup

**Player 2:**
- `live_card_zone`: [] (no live card)
- `stage`: Empty or minimal setup
- Other zones: Standard setup

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Live Success or Live End (when score comparison occurs)

## Expected Action Sequence

**Step 1: Evaluate score comparison condition**
- Engine function called: Condition evaluation for "score comparison" type
- Parameters: Compare player1's total score vs player2's total score
- Expected intermediate state: Engine detects player2 has no live card
- Expected output: Condition evaluates to true (player1 score > player2 score)

**Step 2: Verify score comparison logic**
- Engine function called: Score calculation
- Expected output: Player1 score is treated as higher even if actual value is 0

## User Choices
None - deterministic condition evaluation

## Expected Final State

**Player 1:**
- Live card active
- Condition evaluated as true

**Player 2:**
- No live card
- Score treated as 0 or lower than player1

## Verification Assertions
1. Score comparison condition evaluates to true when opponent has no live card
2. Player1's score is considered higher regardless of actual value
3. No runtime errors when comparing scores with missing live card
