# Test Plan: Zone Move Resets Restrictions

## 1. Test Objective
Test that moving a card to a different zone resets its restrictions. This tests:
- Area placement restrictions
- Restriction reset on zone move
- Card movement between zones
- Restriction tracking per zone

## 2. Card Selection
- **Card 1:** Member card
- **Card 2:** Member card
- **Why this selection:** Tests area placement and restriction reset

## 3. Initial Game State

**Player 1:**
- `hand`: [member_card_1, member_card_2, member_card_3]
- `main_deck`: [energy cards...]
- `stage`: [member_card_4, -1, -1, -1, -1]
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

**Step 1: Play member_card_1 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=member_card_1, area=Center
- Expected: Card placed in center stage
- Expected: Center area marked as used this turn

**Step 2: Attempt to play member_card_2 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=member_card_2, area=Center
- Expected: Fails (center area already used this turn)

**Step 3: Move member_card_1 to waitroom**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::MoveMember, card_id=member_card_1, to_zone=waitroom
- Expected: member_card_1 moved to waitroom
- Expected: Center area restriction reset

**Step 4: Play member_card_2 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=member_card_2, area=Center
- Expected: Succeeds (restriction reset)
- Expected: Card placed in center stage

**Step 5: Move member_card_2 to left side**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::MoveMember, card_id=member_card_2, from_area=Center, to_area=LeftSide
- Expected: member_card_2 moved to left side
- Expected: Center area restriction reset

**Step 6: Play member_card_3 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=member_card_3, area=Center
- Expected: Succeeds (restriction reset after move)

## 5. User Choices

None - zone moves and restrictions are automatic.

## 6. Expected Final State

**Player 1:**
- `stage`: [member_card_2, member_card_3, member_card_4, -1, -1]
- `waitroom`: [member_card_1]
- `hand`: []
- Area restrictions: All reset

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Area placement restrictions enforced
- Restrictions reset when card leaves zone
- Restrictions reset when card moves to different area
- Card can be played to area after restriction reset
- Restriction tracking is per area
- Zone moves correctly reset restrictions
