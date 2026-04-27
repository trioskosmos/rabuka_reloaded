# Q072: Live Set With No Stage Members

## Test Objective
Test that cards can be placed in the live card zone during the live card set phase even when the player has no members on stage. This validates that live card placement is not restricted by having members on stage.

## Q&A Reference
**Question:** Can you place cards in live card zone during live card set phase when you have no members on stage?
**Answer:** Yes, you can.

## Card Selection
Any live card from the card database.

**Primary Card:** Any live card (is_live() == true)

## Initial Game State

**Player 1:**
- `hand`: [live_id] (one live card)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1] (all empty - no members on stage)
- `waitroom`: []
- `energy_zone`: [energy cards]
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
- `phase`: LiveCardSet
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Place live card in live card zone**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::SetLiveCard`
  - card_id: Some(live_id)
  - stage_area: None
  - use_baton_touch: None
- Expected intermediate state changes:
  - live_id moves from hand to live_card_zone
- Expected output: success (Ok(()))

## User Choices
None - action is deterministic

## Expected Final State

**Player 1:**
- `hand`: [] (live card placed)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1] (still empty)
- `waitroom`: []
- `energy_zone`: [energy cards]
- `success_live_card_zone`: []
- `live_card_zone`: [live_id] (live card placed)

**Player 2:**
- Unchanged from initial state

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: LiveCardSet
- `baton_touch_count`: 0

## Expected Engine Faults
None - this is a normal gameplay scenario

## Verification Assertions
1. live_id is in live_card_zone
2. live_id is not in hand
3. Stage is still empty (no members on stage)
4. No compilation errors
5. No runtime panics
