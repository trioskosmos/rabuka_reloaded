# Test Plan: Shiki Optional Position Change

## Test Objective
Test the ライブ開始時 ability of PL!SP-bp4-008-R+ 若菜四季 which has:
- Optional effect: position_change (move this member to a different stage area)
- If the target area has a member, swap positions

This tests:
- Optional effect execution (player can choose to position change or not)
- position_change action
- Stage area movement and swapping
- ライブ開始時 trigger with optional effect

## Card Selection
- **Primary card:** PL!SP-bp4-008-R+ 若菜四季 (cost 4)
- **Supporting member for stage:** PL!SP-bp1-001-R 平塚紗矢 (cost 3)
- **Why this card:** Tests position_change action which is a complex stage manipulation mechanic

## Initial Game State

**Player 1:**
- `hand`: [shiki_id, sayaka_id] (2 cards)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, sayaka_id, -1, -1, -1] (sayaka in center position)
- `waitroom`: empty
- `energy_zone`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- Same structure as Player 1 (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ライブ開始時 phase)
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Trigger ライブ開始時 ability**
- Engine function called: `execute_ability_effect` or `resolve_ability`
- Parameters passed: ability with trigger "ライブ開始時"
- Expected intermediate state changes: None (trigger is automatic)
- Expected output: success, pending choice for optional position_change

**Step 2: Present optional position_change choice**
- Engine function called: `execute_effect` on the effect
- Parameters passed: effect with `optional: true`, action "position_change"
- Expected intermediate state changes: `pending_choice` set to allow skip
- Expected output: Ok(()) with pending choice

**Step 3: User chooses to position_change**
- User choice: Choose to position_change (not skip)
- Engine function called: `provide_choice_result` with choice to execute effect
- Parameters passed: choice to execute the optional effect
- Expected intermediate state changes:
  - Engine presents area selection (which area to move to)
- Expected output: success, pending choice for area selection

**Step 4: User selects target area**
- User choice: Select area 0 (left position)
- Engine function called: `provide_choice_result` with area selection
- Parameters passed: selected area index
- Expected intermediate state changes:
  - shiki_id moves from hand to stage area 0
  - If area 0 has a member, swap with that member
- Expected output: success, ability resolution complete

## User Choices

**Choice 1: Optional effect execution**
- Choice type: Skip or Execute
- Available options: Skip (don't position_change), Execute (position_change)
- Which option will be selected: Execute
- Why: To test the position_change mechanic
- Expected result: Area selection choice presented

**Choice 2: Area selection**
- Choice type: SelectArea
- Available options: Stage areas 0, 1, 2, 3, 4 (excluding current area if already on stage)
- Which option will be selected: Area 0 (left position)
- Why: To test moving to an occupied area (sayaka is in area 1, we'll move to area 0 which is empty for simplicity)
- Expected result: shiki_id moves to selected area

## Expected Final State

**Player 1:**
- `hand`: [sayaka_id] (1 card - shiki moved to stage)
- `main_deck`: 50 cards (unchanged)
- `stage`: [shiki_id, sayaka_id, -1, -1, -1] (shiki in left, sayaka in center)
- `waitroom`: empty
- `energy_zone`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- Unchanged

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability resolved, phase continues)
- `baton_touch_count`: 0

## Expected Engine Faults (if any)

**Potential fault 1: position_change action not implemented**
- What fault: Engine may not support position_change action
- Expected failure mode: Effect execution fails
- How to fix: Implement position_change action that moves members between stage areas

**Potential fault 2: Optional effect with choice**
- What fault: Engine may not properly handle optional effects that require user choice
- Expected failure mode: Choice not presented or effect executes immediately without choice
- How to fix: Implement proper optional effect handling with choice presentation

**Potential fault 3: Area selection choice**
- What fault: Engine may not support area selection choices
- Expected failure mode: No way to select which area to move to
- How to fix: Implement SelectArea choice type and handler

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards in hand initially")`
   - `assert!(game_state.player1.hand.cards.contains(&shiki_id), "Shiki should be in hand")`
   - `assert_eq!(game_state.player1.stage.stage[1], sayaka_id, "Sayaka should be on stage")`

2. **After position_change:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand after position_change")`
   - `assert!(!game_state.player1.hand.cards.contains(&shiki_id), "Shiki should not be in hand")`
   - `assert_eq!(game_state.player1.stage.stage[0], shiki_id, "Shiki should be in left area")`
   - `assert_eq!(game_state.player1.stage.stage[1], sayaka_id, "Sayaka should still be in center")`

## Notes

- This test focuses on position_change action with optional effect
- The engine needs to support:
  - Optional effect execution with user choice
  - position_change action
  - Area selection choices
  - Stage area movement and swapping
- If position_change is not implemented, the test may need to be adapted or the engine may need to be fixed
