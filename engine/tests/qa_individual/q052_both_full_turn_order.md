# Test Plan: Both Full Turn Order

## 1. Test Objective
Test turn order when both players have full decks. This tests:
- Turn order with full deck composition
- Deck refresh timing
- Turn order continuation after deck refresh
- Full deck vs half deck turn order

## 2. Card Selection
- **Deck 1:** Full deck (48 member + 12 live)
- **Deck 2:** Full deck (48 member + 12 live)
- **Why this selection:** Tests full deck turn order

## 3. Initial Game State

**Player 1:**
- `main_deck`: Full deck (48 member + 12 live)
- `energy_deck`: Full energy deck
- `hand`: [member_card_1, member_card_2, member_card_3, member_card_4, member_card_5]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `main_deck`: Full deck (48 member + 12 live)
- `energy_deck`: Full energy deck
- `hand`: [member_card_6, member_card_7, member_card_8, member_card_9, member_card_10]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Play multiple turns**
- Engine function called: `TurnEngine::execute_turn` for multiple turns
- Expected: Turn order alternates correctly
- Expected: Both players draw cards each turn
- Expected: Both players play cards each turn

**Step 2: Deplete main deck**
- Continue playing until main deck is low
- Expected: Deck refresh triggers when main deck is empty
- Expected: Waitroom cards shuffled into main deck

**Step 3: Verify deck refresh**
- Engine function called: `TurnEngine::refresh_deck`
- Expected: Waitroom cards moved to main deck
- Expected: Main deck shuffled
- Expected: Turn order continues

**Step 4: Continue gameplay after refresh**
- Expected: Turn order continues normally
- Expected: Cards drawn from refreshed deck
- Expected: Gameplay unaffected by refresh

**Step 5: Verify turn order consistency**
- Expected: Turn order remains consistent throughout
- Expected: No turn order disruption from deck refresh

## 5. User Choices

**Choice 1 (card to play):**
- Choice type: SelectCard
- Available options: Cards in hand
- Selection: Various cards throughout turns
- Expected result: Cards played to stage

**Choice 2 (live card selection):**
- Choice type: SelectCard
- Available options: Live cards in hand
- Selection: Live card when available
- Expected result: Live card set

## 6. Expected Final State

**Player 1:**
- `main_deck`: Refreshed (contains waitroom cards)
- `waitroom`: Cleared after refresh
- Turn order: Alternated correctly

**Player 2:**
- `main_deck`: Refreshed (contains waitroom cards)
- `waitroom`: Cleared after refresh
- Turn order: Alternated correctly

**Game State:** Turn order consistent, gameplay continued

## 7. Verification Assertions

- Turn order alternates correctly with full decks
- Deck refresh triggers when main deck is empty
- Waitroom cards shuffled into main deck on refresh
- Turn order continues normally after deck refresh
- Gameplay unaffected by deck refresh
- Full deck composition maintained after refresh
- Turn order consistency throughout game
