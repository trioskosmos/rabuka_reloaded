# Test Plan: Kaoru Reveal and Search

## Test Objective
Test the 起動 ability of PL!HS-bp5-001-R＋ 日野下花帆 which has:
- Cost: pay 2 energy + reveal 1 live card from hand
- Effect: search discard for live card containing all card names from revealed card

This tests:
- reveal action in cost
- Card name matching/searching
- move_cards with card name condition
- Sequential cost (energy + reveal)

## Card Selection
- **Primary card:** PL!HS-bp5-001-R＋ 日野下花帆 (cost 6)
- **Live card to reveal:** Any live card with a name that exists in discard
- **Why this card:** Tests reveal action and card name-based search

## Initial Game State

**Player 1:**
- `hand`: [kaoru_id, live_card_id1, live_card_id2] (3 cards)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `energy_zone`: [energy_id, energy_id, energy_id] (3 energy cards)
- `waitroom`: empty
- `discard`: [live_card_id3] (1 live card with matching name)
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- Same structure as Player 1 (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability can be activated)
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Pay energy cost**
- Engine function called: `pay_cost` or similar
- Parameters passed: cost with type "pay_energy", energy 2
- Expected intermediate state changes: 2 energy removed from active zone
- Expected output: success, continue to next cost

**Step 2: Reveal live card from hand**
- Engine function called: `pay_cost` or `execute_effect` on reveal
- Parameters passed: cost with type "reveal", source "hand", count 1, card_type "live_card"
- Expected intermediate state changes: Card is marked as revealed
- Expected output: success, effect execution continues

**Step 3: Execute search effect**
- Engine function called: `execute_effect` on the move_cards effect
- Parameters passed: effect with action "move_cards", source "discard", destination "hand", card_type "live_card", card name condition
- Expected intermediate state changes:
  - Live card matching revealed card's name moved from discard to hand
- Expected output: success, ability resolution complete

## User Choices

**Choice 1: Which live card to reveal**
- Choice type: Select card from hand
- Available options: All live cards in hand
- Which option will be selected: live_card_id1
- Why: To test the reveal and search mechanism
- Expected result: Card is revealed, then matching card is searched

## Expected Final State

**Player 1:**
- `hand`: [kaoru_id, live_card_id1, live_card_id2, live_card_id3] (4 cards - revealed card stays, searched card added)
- `main_deck`: 50 cards (unchanged)
- `stage`: [-1, -1, -1, -1, -1] (unchanged)
- `energy_zone`: [energy_id] (1 energy - 2 paid)
- `waitroom`: empty
- `discard`: empty (live_card_id3 moved to hand)

**Player 2:**
- Unchanged

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability resolved, phase continues)
- `baton_touch_count`: 0

## Expected Engine Faults (if any)

**Potential fault 1: reveal action not implemented**
- What fault: Engine may not support the reveal action in costs
- Expected failure mode: Cost payment fails
- How to fix: Implement reveal action that marks cards as revealed

**Potential fault 2: Card name matching not implemented**
- What fault: Engine may not support card name-based search
- Expected failure mode: Effect execution fails or wrong card selected
- How to fix: Implement card name matching logic in move_cards

**Potential fault 3: Sequential cost not properly executed**
- What fault: Engine may not execute sequential costs in order
- Expected failure mode: Only first cost executes
- How to fix: Ensure sequential cost implementation executes all costs in order

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 3, "Should have 3 cards in hand initially")`
   - `assert_eq!(game_state.player1.energy_zone.cards.len(), 3, "Should have 3 energy initially")`
   - `assert_eq!(game_state.player1.discard.cards.len(), 1, "Should have 1 card in discard initially")`

2. **After cost payment:**
   - `assert_eq!(game_state.player1.energy_zone.cards.len(), 1, "Should have 1 energy after payment")`

3. **After effect execution:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 4, "Should have 4 cards in hand after search")`
   - `assert_eq!(game_state.player1.discard.cards.len(), 0, "Discard should be empty after search")`

## Notes

- This test focuses on reveal action and card name-based search
- The engine needs to support:
  - reveal action in costs
  - Card name matching
  - Sequential costs
- If reveal is not implemented, the test may need to be adapted or the engine may need to be fixed
