# Test Plan: Prohibition Priority

## 1. Test Objective
Test prohibition effect priority and resolution. This tests:
- Multiple prohibition effects active simultaneously
- Prohibition priority rules
- Prohibition effect stacking
- Prohibition vs ability execution order

## 2. Card Selection
- **Card 1:** Member card with prohibition: "Cannot play member cards with cost 3 or less"
- **Card 2:** Member card with prohibition: "Cannot play member cards with cost 5 or more"
- **Card 3:** Member card to play (cost 4)
- **Why this selection:** Tests multiple prohibitions and priority

## 3. Initial Game State

**Player 1:**
- `hand`: [prohibition_card_1, prohibition_card_2, member_card_3 (cost 4)]
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

**Step 1: Play prohibition_card_1 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=prohibition_card_1, area=Center
- Expected: Card placed in center stage, prohibition effect active

**Step 2: Play prohibition_card_2 to left side**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=prohibition_card_2, area=LeftSide
- Expected: Card placed in left side stage, prohibition effect active

**Step 3: Attempt to play member_card_3 (cost 4)**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=member_card_3, area=RightSide
- Expected: Prohibition check:
  - prohibition_card_1: cost 4 > 3, so NOT prohibited
  - prohibition_card_2: cost 4 < 5, so NOT prohibited
- Expected: Card can be played (not prohibited by either)

**Step 4: Verify prohibition priority**
- Expected: Both prohibitions checked
- Expected: Prohibition with higher priority checked first
- Expected: Card allowed if not prohibited by any

**Step 5: Test with prohibited card**
- Attempt to play member_card_4 (cost 2)
- Expected: Prohibition check:
  - prohibition_card_1: cost 2 <= 3, so PROHIBITED
- Expected: Card cannot be played

## 5. User Choices

None - prohibition checks are automatic.

## 6. Expected Final State

**Player 1:**
- `stage`: [prohibition_card_2, prohibition_card_1, member_card_1, member_card_2, member_card_3]
- `hand`: []
- Prohibition effects: Both active

**Player 2:**
- Unchanged

## 7. Verification Assertions
- Prohibition effects checked before card play
- Multiple prohibitions checked simultaneously
- Prohibition priority rules respected
- Card prohibited if any prohibition applies
- Card allowed if no prohibition applies
- Prohibition effects stack correctly
