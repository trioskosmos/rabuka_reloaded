# Test Plan: Live Failure No Score

## 1. Test Objective
Test that failed lives do not award score. This tests:
- Live failure conditions
- Score calculation on live failure
- Live card handling on failure
- Cheer card handling on failure

## 2. Card Selection
- **Live Card:** A live card with high score requirement
- **Cheer Cards:** Cheer cards with insufficient blade/heart to meet requirement
- **Why this selection:** Tests live failure and score handling

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1 (requires 10 blades), cheer_card_1 (2 blades), cheer_card_2 (3 blades)]
- `main_deck`: [energy cards...]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
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

**Step 1: Set live_card_1**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=live_card_1
- Expected: Live card placed in live_card_zone

**Step 2: Cheer phase - place cheer cards**
- User selects: cheer_card_1, cheer_card_2
- Expected: Both moved to success_live_card_zone
- Expected: Total blades: 2 + 3 = 5 (insufficient for 10 requirement)

**Step 3: Execute live performance**
- Engine function called: Execute live performance
- Expected: Live fails (5 blades < 10 required)

**Step 4: Verify live failure handling**
- Expected: live_card_1 moved to waitroom (failure)
- Expected: cheer_card_1 moved to waitroom (failure)
- Expected: cheer_card_2 moved to waitroom (failure)
- Expected: No score awarded for failed live

**Step 5: Verify zones after failure**
- Expected: success_live_card_zone is empty
- Expected: waitroom contains live_card_1, cheer_card_1, cheer_card_2
- Expected: No score modifiers active

## 5. User Choices

**Choice 1 (cheer card selection):**
- Choice type: SelectCard (multiple)
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_1, cheer_card_2
- Expected result: Both moved to success_live_card_zone

**Choice 2 (confirm cheer):**
- Choice type: SelectOption
- Available options: Confirm, Add more
- Selection: Confirm
- Expected result: Cheer phase ends

## 6. Expected Final State

**Player 1:**
- `hand`: []
- `success_live_card_zone`: []
- `waitroom`: [live_card_1, cheer_card_1, cheer_card_2]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
- Score awarded: 0 (live failed)

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Live fails when blade/heart requirements not met
- No score awarded for failed live
- Live card moved to waitroom on failure
- Cheer cards moved to waitroom on failure
- success_live_card_zone is empty after failure
- No score modifiers active after failure
