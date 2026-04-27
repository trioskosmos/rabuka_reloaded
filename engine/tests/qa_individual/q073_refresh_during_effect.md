# Q073: Refresh During Effect Resolution

## Test Objective
Test that when a deck runs out during effect resolution (e.g., a debut ability that reveals cards until a live card is found), the refresh operation excludes the cards revealed by the ability, then resumes effect resolution.

## Q&A Reference
**Question:** Debut ability reveals deck cards until live card is found, adds live card to hand, discards others. If deck runs out during effect resolution, how does refresh work?
**Answer:** Refresh excludes cards revealed by the ability, then resumes effect resolution.

## Card Selection
A member card with a debut ability that reveals cards from the deck until a live card is found.

**Primary Card:** PL!N-bp1-011-R (or any member with similar ability)

## Initial Game State

**Player 1:**
- `hand`: [member_id] (member with reveal ability)
- `main_deck`: [very few cards, arranged so live card is near bottom to trigger refresh]
- `stage`: [-1, -1, -1, -1, -1]
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
- `phase`: Main
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Debut member to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters:
  - action: `ActionType::PlayMemberToStage`
  - card_id: Some(member_id)
  - stage_area: Some(MemberArea::Center)
  - use_baton_touch: Some(false)
- Expected intermediate state changes:
  - member_id moves from hand to stage
  - Energy consumed
  - Debut ability triggers
- Expected output: success (Ok(()))

**Step 2: Debut ability executes (reveal until live card found)**
- Engine function called: Ability resolution via debut trigger
- Expected intermediate state changes:
  - Deck cards revealed one by one
  - When deck runs out, refresh triggers
  - Refresh excludes revealed cards
  - Effect resumes with remaining deck
  - Live card found and added to hand
  - Revealed non-live cards discarded
- Expected output: success (Ok(()))

## User Choices
None - all actions are deterministic

## Expected Final State

**Player 1:**
- `hand`: [live_id] (live card added by ability)
- `main_deck`: [remaining deck after refresh and reveal]
- `stage`: [-1, member_id, -1, -1, -1] (member on stage)
- `waitroom`: [revealed_non_live_cards] (discarded revealed cards)
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

## Expected Engine Faults
None - this is a normal gameplay scenario

## Verification Assertions
1. member_id is on stage
2. live_id is in hand
3. Deck was refreshed (deck count increased)
4. Revealed cards are in discard zone
5. No compilation errors
6. No runtime panics

## Implementation Notes
This test is complex because it requires:
1. Setting up a deck with very few cards to trigger refresh
2. Triggering a debut ability that reveals cards
3. Verifying refresh behavior during effect resolution

If the engine doesn't fully support this scenario yet, the test may need to be marked as pending or simplified to test the refresh mechanism independently.
