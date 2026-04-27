# Q028: Debut Without Baton Touch

## Test Objective
Test that you can debut a member to an area that already has a member without using baton touch, by paying the cost equal to the new member's cost. The old member goes to waitroom.

## Q&A Reference
**Question:** Can you debut to area with member without baton touch?
**Answer:** Yes, pay cost equal to new member's cost, old member goes to waitroom.

## Card Selection
Use a specific member card with affordable cost for debut testing.

**Primary Card:** PL!N-bp1-001-R (星空 凛)
- Card ID: PL!N-bp1-001-R
- Card Name: 星空 凛
- Cost: 2 (affordable for testing)
- Rarity: R
- Why this card: Low cost, readily available, represents typical member card

**Secondary Card:** PL!N-bp1-002-R (高海千歌)
- Card ID: PL!N-bp1-002-R
- Card Name: 高海千歌
- Cost: 2
- Why this card: Same cost as primary, allows testing debut replacement

## Initial Game State

**Player 1:**
- `hand`: [PL!N-bp1-001-R, PL!N-bp1-001-R] (2 copies of 星空 凛)
- `main_deck`: [PL!N-bp1-002-R, PL!N-bp1-003-R, PL!N-bp1-004-R, PL!N-bp1-005-R, PL!-EN-001, PL!-EN-002, PL!-EN-003, PL!-EN-004, PL!-EN-005, PL!-EN-006, PL!-EN-007, PL!-EN-008, PL!-EN-009, PL!-EN-010, PL!-EN-011, PL!-EN-012]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [PL!-EN-001, PL!-EN-002, PL!-EN-003, PL!-EN-004, PL!-EN-005, PL!-EN-006, PL!-EN-007, PL!-EN-008, PL!-EN-009, PL!-EN-010]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [PL!N-bp1-006-R, PL!N-bp1-007-R]
- `main_deck`: [PL!N-bp1-008-R, PL!N-bp1-009-R, PL!N-bp1-010-R, PL!-EN-011, PL!-EN-012, PL!-EN-013, PL!-EN-014, PL!-EN-015, PL!-EN-016, PL!-EN-017, PL!-EN-018, PL!-EN-019, PL!-EN-020, PL!-EN-021, PL!-EN-022]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [PL!-EN-023, PL!-EN-024, PL!-EN-025, PL!-EN-026, PL!-EN-027, PL!-EN-028, PL!-EN-029, PL!-EN-030, PL!-EN-031, PL!-EN-032]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Play first member to center area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member_id)
  - stage_area: Some(MemberArea::Center)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - member_id moves from hand to stage[1]
  - Energy consumed equal to member cost
- Expected output: success (Ok(()))

**Step 2: Play second member to same area (without baton touch)**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member_id)
  - stage_area: Some(MemberArea::Center)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - First member_id moves from stage[1] to waitroom
  - Second member_id moves from hand to stage[1]
  - Energy consumed equal to member cost
- Expected output: success (Ok(()))

## User Choices
None - deterministic

## Expected Final State

**Player 1:**
- `hand`: [] (both members played)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, member_id, -1, -1, -1] (second member in center)
- `waitroom`: [member_id] (first member in discard)
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

## Verification Assertions
1. First member debut succeeds
2. Second member debut to same area succeeds (without baton touch)
3. First member is in waitroom
4. Second member is on stage
5. Energy was paid for both debuts
6. No compilation errors
7. No runtime panics
