# Test Plan: Deck Refresh

## 1. Test Objective
Test deck refresh mechanics. This tests:
- Deck refresh trigger conditions
- Waitroom card shuffling into main deck
- Deck refresh timing
- Main deck composition after refresh

## 2. Card Selection
- **Deck:** Standard deck composition
- **Cards:** Various member and live cards
- **Why this selection:** Tests deck refresh with standard deck

## 3. Initial Game State

**Player 1:**
- `main_deck`: [member_card_1, member_card_2, ..., member_card_48, live_card_1, ..., live_card_12]
- `energy_deck`: [energy cards...]
- `hand`: [member_card_49, member_card_50, member_card_51, member_card_52, member_card_53]
- `stage`: [member_card_54, member_card_55, -1, -1, -1]
- `waitroom`: [member_card_56, member_card_57, member_card_58]
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- Same structure (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Draw
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Draw cards until main deck is empty**
- Engine function called: `TurnEngine::execute_draw_phase` repeatedly
- Expected: Cards drawn from main deck
- Expected: Main deck count decreases

**Step 2: Trigger deck refresh**
- Engine function called: `TurnEngine::execute_draw_phase` when main deck is empty
- Expected: Deck refresh triggers
- Expected: Waitroom cards moved to main deck
- Expected: Main deck shuffled

**Step 3: Verify deck refresh execution**
- Expected: Waitroom cards (member_card_56, member_card_57, member_card_58) in main deck
- Expected: Waitroom is empty
- Expected: Main deck is shuffled (random order)

**Step 4: Draw from refreshed deck**
- Engine function called: `TurnEngine::execute_draw_phase`
- Expected: Card drawn from refreshed main deck
- Expected: Card is one of the waitroom cards

**Step 5: Verify deck composition**
- Expected: Main deck contains waitroom cards
- Expected: Main deck count is correct

## 5. User Choices

None - deck refresh is automatic when main deck is empty.

## 6. Expected Final State

**Player 1:**
- `main_deck`: [member_card_56, member_card_57, member_card_58, ...] (shuffled)
- `waitroom`: []
- `hand`: [member_card_49, member_card_50, member_card_51, member_card_52, member_card_53, newly_drawn_card]
- `stage`: [member_card_54, member_card_55, -1, -1, -1]

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Deck refresh triggers when main deck is empty
- Waitroom cards moved to main deck on refresh
- Waitroom is empty after refresh
- Main deck is shuffled after refresh
- Can draw from refreshed deck
- Main deck composition correct after refresh
- Deck refresh timing is correct
