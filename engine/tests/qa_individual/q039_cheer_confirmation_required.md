# Test Plan: Cheer Confirmation Required

## 1. Test Objective
Test that cheer confirmation is required during live execution. This tests:
- Cheer phase mechanics (Rule 8.3)
- User confirmation for cheer cards
- Cheer card placement from hand to success_live_card_zone
- Cheer blade/heart counting for victory determination

## 2. Card Selection
- **Live Card:** Any live card (e.g., PL!S-bp2-024-L)
- **Cheer Card:** A cheer card (member card with cheer icons)
- **Why this selection:** Tests the cheer confirmation and placement mechanics

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, cheer_card_1, member_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- Same structure (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Set live_card_1 as live card**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=live_card_1
- Expected: Live card placed in live_card_zone

**Step 2: Advance to cheer phase**
- Engine function called: `TurnEngine::advance_phase`
- Expected: Phase advances to Cheer
- Expected: Engine presents choice: Select cheer cards to place

**Step 3: User selects cheer_card_1**
- User choice: Select cheer_card_1 to cheer
- Expected: cheer_card_1 moved from hand to success_live_card_zone
- Expected: Cheer blade/heart counted for victory

**Step 4: User confirms cheer selection**
- User choice: Confirm cheer selection (or add more cheer cards)
- Expected: Cheer phase ends, live execution proceeds

**Step 5: Execute live (succeed)**
- Engine function called: Execute live with success
- Expected: Live succeeds
- Expected: Cheer cards contribute to blade/heart total

**Step 6: Verify final state**
- Expected: cheer_card_1 in success_live_card_zone
- Expected: Blade/heart count includes cheer_card_1's contribution

## 5. User Choices

**Choice 1 (cheer card selection):**
- Choice type: SelectCard
- Available options: Member cards in hand with cheer icons
- Selection: cheer_card_1
- Expected result: Card moved to success_live_card_zone

**Choice 2 (confirm cheer):**
- Choice type: SelectOption
- Available options: Confirm, Add more cheer cards
- Selection: Confirm
- Expected result: Cheer phase ends, live proceeds

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1]
- `live_card_zone`: []
- `success_live_card_zone`: [live_card_1, cheer_card_1]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
- Cheer blade/heart count: Includes cheer_card_1's contribution

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Cheer phase presents user choice for cheer card selection
- Selected cheer cards are moved to success_live_card_zone
- Cheer confirmation is required before live execution
- Cheer blade/heart contributions are counted for victory
- User can add multiple cheer cards before confirming
- Cheer phase cannot be skipped without explicit confirmation
