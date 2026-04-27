# Q066: Score Comparison No Live

## Test Objective
Test that a condition "total live score is higher than opponent's" is satisfied when the player has a live card in success zone and the opponent has none (opponent treated as score 0).

## Q&A Reference
**Question:** If player has live card in success zone and opponent has none, is condition met?
**Answer:** Yes, condition is met regardless of player's score (opponent treated as 0).

## Card Selection
A live card with score comparison ability (PL!N-bp1-026-L "Poppin' Up!").

**Primary Card:** PL!N-bp1-026-L

## Initial Game State
N/A - This is a condition verification test, not a gameplay test.

## Expected Action Sequence

**Step 1: Verify live card has score**
- Engine function called: `card_database.get_card(card_id)`
- Expected output: card.is_live() = true, card.score > 0

**Step 2: Verify score comparison condition**
- Engine function called: Score comparison evaluation
- Expected output: Condition is true when opponent has no live cards (treated as score 0)

## User Choices
None - deterministic

## Expected Final State
N/A - condition verification only

## Expected Engine Faults
None - this is a condition verification test

## Verification Assertions
1. Card is a live card
2. Card has a score > 0
3. Score comparison "higher than opponent" is satisfied when opponent has no live cards
4. No compilation errors
5. No runtime panics
