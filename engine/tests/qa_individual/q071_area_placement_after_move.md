# Q071: Area Placement After Move

## Test Objective
Test that when a member card is placed in an area and then moves to another zone (discard, waitroom, etc.), another member can be debuted/placed in that area during the same turn. This validates the area placement restriction is cleared when the original member leaves the area.

## Q&A Reference
**Question:** When a member card is placed in an area and then moves to another zone, can you debut/place another member in that area during the same turn?
**Answer:** Yes, you can.

## Card Selection
Need a member card with an ability that can send itself or another member to discard/waitroom from stage. Alternatively, test with a member that can be destroyed/sent to discard through gameplay.

**Primary Card:** Any member card with cost ≤ 5 (for energy affordability)
**Secondary Card:** Another member card to place in the same area after first leaves

## Initial Game State

**Player 1:**
- `hand`: [member1_id, member2_id] (two member cards)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1] (all empty)
- `waitroom`: []
- `energy_zone`: [energy_card_1, energy_card_2, energy_card_3, energy_card_4, energy_card_5] (5 energy cards)
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [member cards]
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Play member1 to Center area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: 
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member1_id)
  - stage_area: Some(MemberArea::Center)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - member1_id moves from hand to stage[1]
  - Energy zone loses cost amount of energy
  - area_placed_this_turn[1] = true
- Expected output: success (Ok(()))

**Step 2: Send member1 to discard (simulate ability effect or destruction)**
- Engine function called: Manual simulation of effect that sends member to discard
- Since engine may not have direct "send to discard" function, this may need to be simulated through ability execution or direct state manipulation for this specific test case
- Expected intermediate state changes:
  - member1_id moves from stage[1] to waitroom
  - stage[1] = -1
  - area_placed_this_turn[1] should be cleared (this is what we're testing)
- Expected output: success

**Step 3: Play member2 to Center area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member2_id)
  - stage_area: Some(MemberArea::Center)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - member2_id moves from hand to stage[1]
  - Energy zone loses cost amount
  - area_placed_this_turn[1] = true (for member2)
- Expected output: success (Ok(())) - this should succeed because area restriction was cleared

## User Choices
None - all actions are deterministic

## Expected Final State

**Player 1:**
- `hand`: [] (both members played)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, member2_id, -1, -1, -1] (member2 in center)
- `waitroom`: [member1_id] (member1 in discard)
- `energy_zone`: [remaining energy after paying both costs]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- Unchanged from initial state

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0
- `area_placed_this_turn`: [false, true, false, false, false] (center marked for member2)

## Expected Engine Faults
None - this is a normal gameplay scenario

## Verification Assertions
1. member1_id is in waitroom (discard zone)
2. member2_id is in stage[1] (center area)
3. hand is empty (both members played)
4. area_placed_this_turn[1] is true (member2 placed this turn)
5. No compilation errors
6. No runtime panics about area placement restrictions
