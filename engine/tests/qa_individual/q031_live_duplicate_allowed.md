# Q031: Live Duplicate Allowed

## Test Objective
Test that you can put 2+ copies of the same card (same card number or name) in the live card zone.

## Q&A Reference
**Question:** Can you put 2+ copies of same card in live area?
**Answer:** Yes, can put 2+ copies even with same card number or name.

## Card Selection
A live card.

**Primary Card:** Any live card

## Initial Game State

**Player 1:**
- `hand`: [live_id, live_id] (2 copies of same live card), [member cards]
- `main_deck`: [remaining deck cards]
- `stage`: [member_id, member_id, member_id, -1, -1] (3 members for heart requirements)
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

**Step 1: Play members to stage (for heart requirements)**
- Engine function called: `TurnEngine::execute_main_phase_action` (3 times)
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member_id)
  - stage_area: Various areas
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - Members move from hand to stage
  - Energy consumed
- Expected output: success (Ok(()))

**Step 2: Place both live cards in live card zone**
- Engine function called: `TurnEngine::execute_main_phase_action` (2 times)
- Parameters:
  - action: `ActionType::SetLiveCard`
  - card_id: Some(live_id)
- Expected intermediate state changes:
  - live_id moves from hand to live_card_zone (both copies)
- Expected output: success (Ok(()))

## User Choices
None - deterministic

## Expected Final State

**Player 1:**
- `hand`: [remaining cards]
- `main_deck`: [remaining deck cards]
- `stage`: [member_id, member_id, member_id, -1, -1]
- `waitroom`: []
- `energy_zone`: [remaining energy]
- `success_live_card_zone`: []
- `live_card_zone`: [live_id, live_id] (both copies)

**Player 2:**
- Unchanged from initial state

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## Expected Engine Faults
None - this is a normal gameplay scenario

## Verification Assertions
1. Both live cards are in live_card_zone
2. No compilation errors
3. No runtime panics
