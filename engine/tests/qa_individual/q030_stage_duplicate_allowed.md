# Q030: Stage Duplicate Allowed

## Test Objective
Test that you can debut 2+ copies of the same card (same card number or name) to stage.

## Q&A Reference
**Question:** Can you debut 2+ copies of same card to stage?
**Answer:** Yes, can debut 2+ copies even with same card number or name.

## Card Selection
A member card with affordable cost (≤3).

**Primary Card:** PL!N-bp1-001-R (星空 凛)
- Card ID: PL!N-bp1-001-R
- Card Name: 星空 凛
- Cost: 2 (affordable for testing)
- Rarity: R
- Why this card: Low cost, readily available, represents typical member card

## Initial Game State

**Player 1:**
- `hand`: [member_id, member_id] (2 copies of same member)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [10 energy cards]
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

**Step 1: Play first member to center area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member_id)
  - stage_area: Some(MemberArea::Center)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - member_id moves from hand to stage[1]
  - Energy consumed
- Expected output: success (Ok(()))

**Step 2: Play second member to left side area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member_id)
  - stage_area: Some(MemberArea::LeftSide)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - member_id moves from hand to stage[0]
  - Energy consumed
- Expected output: success (Ok(()))

## User Choices
None - deterministic

## Expected Final State

**Player 1:**
- `hand`: [] (both members played)
- `main_deck`: [remaining deck cards]
- `stage`: [member_id, member_id, -1, -1, -1] (both copies on stage)
- `waitroom`: []
- `energy_zone`: [remaining energy]
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
2. Second member debut succeeds
3. Both copies of same card are on stage (different areas)
4. No compilation errors
5. No runtime panics
