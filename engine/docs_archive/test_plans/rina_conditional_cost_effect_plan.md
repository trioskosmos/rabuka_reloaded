# Test Plan: Satoko Conditional Effect with Optional Cost

## Test Objective
Test the ライブ開始時 (Live Start) ability of PL!HS-bp1-006-R+ 藤島 慈 which has:
- Optional cost: discard 1 card from hand
- Conditional effect: if another member is on stage, gain heart (all colors) until live end

This tests:
- Optional cost with ライブ開始時 trigger (different from 登場 trigger tested with Kasumi)
- Conditional effect based on stage condition (another member on stage)
- Gain resource with duration (until live end)
- Heart selection (choose which heart color to gain)

## Card Selection
- **Primary card:** PL!HS-bp1-006-R+ 藤島 慈 (cost 4)
- **Supporting card for hand discard:** PL!HS-bp1-001-R 日野下花帆 (cost 3)
- **Supporting card for stage condition:** PL!HS-bp1-002-R 村野さやか (cost 4)
- **Why this card:** Tests conditional effect with stage condition, which is a different type of condition than the look_and_select tested with Kasumi

## Initial Game State

**Player 1:**
- `hand`: [satoko_id, hanafu_id] (2 cards)
- `main_deck`: 50 cards (any cards, not relevant to this test)
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
- Expected output: success, pending choice for optional cost

**Step 2: Present optional cost choice**
- Engine function called: `execute_effect` on the cost
- Parameters passed: cost with `optional: true`, source "hand", destination "discard", count 1
- Expected intermediate state changes: `pending_choice` set to SelectCard from hand with allow_skip
- Expected output: Ok(()) with pending choice

**Step 3: User selects card for optional cost**
- User choice: Select index [0] from hand (hanafu_id)
- Engine function called: `provide_choice_result` with CardSelected { indices: [0] }
- Parameters passed: selected card index
- Expected intermediate state changes: 
  - Card removed from hand
  - Card added to discard
- Expected output: success, effect execution continues

**Step 4: Execute conditional effect**
- Engine function called: `execute_effect` on the conditional effect
- Parameters passed: effect with condition "another member on stage"
- Expected intermediate state changes:
  - Engine checks stage condition (sayaka_id is on stage, excluding self)
  - Condition is true
  - Presents heart color choice
- Expected output: success, pending choice for heart color

**Step 5: User selects heart color**
- User choice: Select heart color (e.g., "heart04")
- Engine function called: `provide_choice_result` with heart color selection
- Parameters passed: selected heart color
- Expected intermediate state changes:
  - Heart resource added with duration "live_end"
- Expected output: success, ability resolution complete

## User Choices

**Choice 1: Optional cost payment**
- Choice type: SelectCard from hand
- Available options: All cards in hand (satoko_id, hanafu_id)
- Which option will be selected: hanafu_id (index [0])
- Why: To pay the optional cost and enable the effect
- Expected result: hanafu_id moved to discard

**Choice 2: Heart color selection**
- Choice type: SelectOption (heart color)
- Available options: heart02, heart03, heart04, heart05, heart06
- Which option will be selected: heart04
- Why: Arbitrary choice for testing
- Expected result: Player gains heart04 until live end

## Expected Final State

**Player 1:**
- `hand`: [satoko_id] (1 card - hanafu discarded)
- `main_deck`: 50 cards (unchanged)
- `stage`: [-1, sayaka_id, -1, -1, -1] (unchanged)
- `waitroom`: [hanafu_id] (1 card - discarded)
- `energy_zone`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty
- `duration_effects`: Contains heart04 gain with duration "live_end"

**Player 2:**
- Unchanged

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability resolved, phase continues)
- `baton_touch_count`: 0

## Expected Engine Faults (if any)

**Potential fault 1: Conditional effect based on stage condition**
- What fault: Engine may not correctly check the "another member on stage" condition
- Expected failure mode: Condition check fails or wrong effect is executed
- How to fix: Implement proper location_condition evaluation with exclude_self

**Potential fault 2: Heart color selection**
- What fault: Engine may not present heart color choice or may not correctly apply the selected heart
- Expected failure mode: No choice presented or heart not added to player
- How to fix: Implement heart selection UI and gain_resource with heart parameter

**Potential fault 3: Duration effect tracking**
- What fault: Engine may not correctly track duration effects (until live end)
- Expected failure mode: Effect doesn't expire at live end
- How to fix: Implement proper duration effect tracking and expiration

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards in hand initially")`
   - `assert!(game_state.player1.hand.cards.contains(&satoko_id), "Satoko should be in hand")`
   - `assert!(game_state.player1.hand.cards.contains(&hanafu_id), "Hanafu should be in hand")`
   - `assert_eq!(game_state.player1.stage.stage[1], sayaka_id, "Sayaka should be on stage")`

2. **After optional cost payment:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand after discard")`
   - `assert!(!game_state.player1.hand.cards.contains(&hanafu_id), "Hanafu should not be in hand")`
   - `assert!(game_state.player1.waitroom.cards.contains(&hanafu_id), "Hanafu should be in discard")`

3. **After effect execution:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should still have 1 card in hand")`
   - Verify that duration_effects contains heart04 gain with duration "live_end"

## Notes

- This test focuses on the conditional effect with stage condition
- The engine needs to support:
  - Location condition evaluation (another member on stage, excluding self)
  - Heart color selection
  - Duration effect tracking (until live end)
  - Optional cost with ライブ開始時 trigger
