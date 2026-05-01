# Test Plan: Rurino Gain Resource with Duration

## Test Objective
Test the ライブ開始時 ability of PL!HS-PR-018-PR 大沢瑠璃乃 which has:
- Optional cost: pay 1 energy
- Effect: gain 2 blades until live end

This tests:
- Optional energy cost payment
- gain_resource action (blade resource)
- Duration effect (until live end)
- ライブ開始時 trigger

## Card Selection
- **Primary card:** PL!HS-PR-018-PR 大沢瑠璃乃 (cost 4)
- **Why this card:** Tests gain_resource with duration effect, which is a fundamental mechanic that needs to work properly

## Initial Game State

**Player 1:**
- `hand`: [rurino_id] (1 card)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `energy_zone`: [energy_id, energy_id, energy_id] (3 energy cards)
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty
- `blades`: 0 (initial blade count)

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

**Step 2: Present optional energy cost choice**
- Engine function called: `execute_effect` on the cost
- Parameters passed: cost with `optional: true`, type "pay_energy", energy 1
- Expected intermediate state changes: `pending_choice` set to allow skip
- Expected output: Ok(()) with pending choice

**Step 3: User chooses to pay optional cost**
- User choice: Pay 1 energy
- Engine function called: `provide_choice_result` with choice to pay energy
- Parameters passed: choice to pay energy
- Expected intermediate state changes:
  - 1 energy moved from active to wait or removed
- Expected output: success, effect execution continues

**Step 4: Execute gain_resource effect**
- Engine function called: `execute_effect` on the gain_resource effect
- Parameters passed: effect with action "gain_resource", resource "blade", count 2, duration "live_end"
- Expected intermediate state changes:
  - Player gains 2 blades
  - Duration effect is tracked (until live end)
- Expected output: success, ability resolution complete

## User Choices

**Choice 1: Optional energy cost**
- Choice type: Skip or Pay
- Available options: Skip (don't pay energy), Pay (pay 1 energy)
- Which option will be selected: Pay
- Why: To test the optional cost payment and gain_resource effect
- Expected result: 1 energy paid, 2 blades gained

## Expected Final State

**Player 1:**
- `hand`: [rurino_id] (1 card - unchanged)
- `main_deck`: 50 cards (unchanged)
- `stage`: [-1, -1, -1, -1, -1] (unchanged)
- `energy_zone`: [energy_id, energy_id] (2 energy - 1 paid)
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty
- `blades`: 2 (gained from effect)
- `duration_effects`: Contains blade gain with duration "live_end"

**Player 2:**
- Unchanged

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability resolved, phase continues)
- `baton_touch_count`: 0

## Expected Engine Faults (if any)

**Potential fault 1: gain_resource action not implemented**
- What fault: Engine may not support the gain_resource action
- Expected failure mode: Effect execution fails
- How to fix: Implement gain_resource action that adds resources to player

**Potential fault 2: Duration effect tracking**
- What fault: Engine may not track duration effects (until live end)
- Expected failure mode: Effect doesn't expire at live end
- How to fix: Implement proper duration effect tracking and expiration

**Potential fault 3: Blade resource tracking**
- What fault: Engine may not track blade resources separately from hearts
- Expected failure mode: Blades not added or tracked incorrectly
- How to fix: Implement blade resource tracking in player state

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand initially")`
   - `assert!(game_state.player1.hand.cards.contains(&rurino_id), "Rurino should be in hand")`
   - `assert_eq!(game_state.player1.energy_zone.cards.len(), 3, "Should have 3 energy initially")`

2. **After optional cost payment:**
   - `assert_eq!(game_state.player1.energy_zone.cards.len(), 2, "Should have 2 energy after payment")`

3. **After effect execution:**
   - `assert_eq!(game_state.player1.blades, 2, "Should have 2 blades after gain_resource")`
   - Verify that duration_effects contains blade gain with duration "live_end"

## Notes

- This test focuses on gain_resource with duration effect
- The engine needs to support:
  - Optional energy cost payment
  - gain_resource action
  - Blade resource tracking
  - Duration effect tracking (until live end)
- If gain_resource is not implemented, the test may need to be adapted or the engine may need to be fixed
