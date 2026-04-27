# Test Plan: Constant Heart Timing

## 1. Test Objective
Test the timing of constant heart abilities. This tests:
- Constant ability activation timing (Rule 9.7.2)
- Heart contribution from constant abilities
- Constant ability duration (permanent vs temporary)
- Constant ability vs live_success timing distinction

## 2. Card Selection
- **Card 1:** Member card with constant heart ability (permanent)
- **Card 2:** Member card with constant heart ability (duration: this_turn)
- **Why this selection:** Tests constant ability timing and duration

## 3. Initial Game State

**Player 1:**
- `hand`: [constant_heart_card_1, constant_heart_card_2, member_card_1]
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

**Step 1: Play constant_heart_card_1 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=constant_heart_card_1, area=Center
- Expected: Card placed in center stage
- Expected: Constant heart ability immediately active

**Step 2: Verify constant heart is active**
- Expected: Heart contribution from constant_heart_card_1 is active
- Expected: Heart counted toward totals immediately

**Step 3: Play constant_heart_card_2 to left side**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=constant_heart_card_2, area=LeftSide
- Expected: Card placed in left side stage
- Expected: Constant heart ability immediately active (duration: this_turn)

**Step 4: Verify both constant hearts active**
- Expected: Heart contribution from both cards active
- Expected: Total heart count includes both contributions

**Step 5: End turn**
- Engine function called: `TurnEngine::end_turn`
- Expected: Turn advances to turn 2
- Expected: constant_heart_card_2's heart effect expires (duration: this_turn)
- Expected: constant_heart_card_1's heart effect remains (permanent)

**Step 6: Verify heart contributions after turn end**
- Expected: constant_heart_card_1 still contributes heart
- Expected: constant_heart_card_2 no longer contributes heart

## 5. User Choices

None - constant abilities activate automatically when card is on stage.

## 6. Expected Final State

**Player 1:**
- `stage`: [constant_heart_card_2, constant_heart_card_1, member_card_2, member_card_3, -1]
- Heart contributions: constant_heart_card_1 (active), constant_heart_card_2 (expired)
- Turn: 2

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Constant abilities activate immediately when card is on stage
- Constant heart contributions are counted toward totals
- Duration (this_turn) expires at turn end
- Permanent constant abilities don't expire
- Multiple constant abilities can be active simultaneously
- Constant ability timing follows Rule 9.7.2
