# Test Plan: Honoka Discard Until Count

## Test Objective
Test the 登場 ability of PL!-bp5-007-R 東條 希 which has:
- Condition: 登場 via baton touch from lower cost member
- Effect: sequential action
  1. discard_until_count: both players discard until hand has 3 cards
  2. draw_card: both players draw 3 cards

This tests:
- Conditional trigger (baton touch from lower cost)
- discard_until_count action
- Sequential actions
- Both players affected

## Card Selection
- **Primary card:** PL!-bp5-007-R 東條 希 (cost 7)
- **Supporting member for baton touch:** PL!-bp1-001-R (cost 3) - lower cost for baton touch
- **Why this card:** Tests discard_until_count action which is the inverse of draw_until_count

## Initial Game State

**Player 1:**
- `hand`: [honoka_id, card1, card2, card3, card4, card5] (6 cards)
- `main_deck`: 50 cards (any cards)
- `stage`: [supporting_member_id, -1, -1, -1, -1] (supporting member for baton touch)
- `energy_zone`: empty
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- `hand`: [card6, card7, card8, card9] (4 cards)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `energy_zone`: empty
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability triggered after 登場)
- `baton_touch_count`: 1 (simulating baton touch from lower cost member)

## Expected Action Sequence

**Step 1: Trigger 登場 ability**
- Engine function called: `execute_ability_effect` or `resolve_ability`
- Parameters passed: ability with trigger "登場"
- Expected intermediate state changes: None (trigger is automatic)
- Expected output: success, effect execution continues

**Step 2: Execute sequential actions**
- Engine function called: `execute_effect` on the sequential effect
- Parameters passed: effect with action "sequential", actions array
- Expected intermediate state changes:
  - Player 1 discards 3 cards (from 6 to 3)
  - Player 2 discards 1 card (from 4 to 3)
  - Player 1 draws 3 cards (from 3 to 6)
  - Player 2 draws 3 cards (from 3 to 6)
- Expected output: success, ability resolution complete

## User Choices

None - this ability has no user choices.

## Expected Final State

**Player 1:**
- `hand`: 6 cards (3 remaining after discard + 3 drawn)
- `main_deck`: 47 cards (50 - 3 drawn)
- `stage`: [honoka_id, -1, -1, -1, -1] (honoka on stage after baton touch)
- `discard`: 3 cards (discarded from hand)

**Player 2:**
- `hand`: 6 cards (3 remaining after discard + 3 drawn)
- `main_deck`: 47 cards (50 - 3 drawn)
- `discard`: 1 card (discarded from hand)

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability resolved, phase continues)
- `baton_touch_count`: 1

## Expected Engine Faults (if any)

**Potential fault 1: discard_until_count action not implemented**
- What fault: Engine may not support the discard_until_count action
- Expected failure mode: Effect execution fails
- How to fix: Implement discard_until_count action that discards cards until hand reaches target count

**Potential fault 2: Sequential actions not properly executed**
- What fault: Engine may not execute sequential actions in order
- Expected failure mode: Only first action executes
- How to fix: Ensure sequential action implementation executes all actions in order

**Potential fault 3: Both players affected**
- What fault: Engine may not apply effects to both players when target is "both"
- Expected failure mode: Only one player affected
- How to fix: Implement proper "both" target handling

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 6, "Should have 6 cards in hand initially")`
   - `assert_eq!(game_state.player2.hand.cards.len(), 4, "Should have 4 cards in hand initially")`

2. **After effect execution:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 6, "Should have 6 cards in hand after discard and draw")`
   - `assert_eq!(game_state.player2.hand.cards.len(), 6, "Should have 6 cards in hand after discard and draw")`
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 47, "Deck should have 47 cards (50 - 3 drawn)")`
   - `assert_eq!(game_state.player2.main_deck.cards.len(), 47, "Deck should have 47 cards (50 - 3 drawn)")`

## Notes

- This test focuses on discard_until_count action
- The engine needs to support:
  - discard_until_count action
  - Sequential actions
  - Both players affected by effects
- If discard_until_count is not implemented, the test may need to be adapted or the engine may need to be fixed
