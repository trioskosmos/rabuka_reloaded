# Test Plan: Kanon Choice Effect with Optional Cost

## Test Objective
Test the ライブ開始時/登場 ability of PL!SP-bp5-001-R+ 澁谷かのん which has:
- Optional cost: pay 1 energy
- Choice effect: choose between two options
  - Option 1: Send opponent's cost 4 or less member to wait
  - Option 2: Draw 1 card

This tests:
- Optional energy cost payment
- Choice effect with multiple options
- User selection between different effect paths
- ライブ開始時 and 登場 triggers

## Card Selection
- **Primary card:** PL!SP-bp5-001-R+ 澁谷かのん (cost 4)
- **Supporting member for opponent stage:** PL!SP-bp1-001-R (cost 3)
- **Why this card:** Tests choice effect which allows player to select between different effect paths

## Initial Game State

**Player 1:**
- `hand`: [kanon_id] (1 card)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `energy_zone`: [energy_id, energy_id, energy_id] (3 energy cards)
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- `hand`: [] (empty)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, opponent_member_id, -1, -1, -1] (opponent member in center, cost 3)
- `energy_zone`: empty
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

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

**Step 4: Present choice effect options**
- Engine function called: `execute_effect` on the choice effect
- Parameters passed: effect with action "choice", options array
- Expected intermediate state changes: `pending_choice` set to SelectOption
- Expected output: Ok(()) with pending choice

**Step 5: User selects option 2 (draw card)**
- User choice: Select option 2 (draw 1 card)
- Engine function called: `provide_choice_result` with option selection
- Parameters passed: selected option index
- Expected intermediate state changes:
  - 1 card drawn from deck to hand
- Expected output: success, ability resolution complete

## User Choices

**Choice 1: Optional energy cost**
- Choice type: Skip or Pay
- Available options: Skip (don't pay energy), Pay (pay 1 energy)
- Which option will be selected: Pay
- Why: To test the optional cost payment
- Expected result: 1 energy paid

**Choice 2: Effect option selection**
- Choice type: SelectOption
- Available options: 
  - Option 1: Send opponent's cost 4 or less member to wait
  - Option 2: Draw 1 card
- Which option will be selected: Option 2 (draw card)
- Why: Simpler to test and verify
- Expected result: 1 card drawn to hand

## Expected Final State

**Player 1:**
- `hand`: [kanon_id, +1 drawn card] (2 cards)
- `main_deck`: 49 cards (50 - 1 drawn)
- `stage`: [-1, -1, -1, -1, -1] (unchanged)
- `energy_zone`: [energy_id, energy_id] (2 energy - 1 paid)
- `waitroom`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- `stage`: [-1, opponent_member_id, -1, -1, -1] (unchanged - option 2 was selected)

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "LiveStart" (ability resolved, phase continues)
- `baton_touch_count`: 0

## Expected Engine Faults (if any)

**Potential fault 1: Choice effect not implemented**
- What fault: Engine may not support the "choice" action type
- Expected failure mode: Effect execution fails
- How to fix: Implement choice effect that presents options and executes selected option

**Potential fault 2: Optional energy cost payment**
- What fault: Engine may not execute optional energy cost payment (similar to optional card cost)
- Expected failure mode: Energy not removed when choice is made
- How to fix: Implement proper optional energy cost execution in provide_choice_result

**Potential fault 3: Option selection and execution**
- What fault: Engine may not properly execute the selected option
- Expected failure mode: Option selected but effect not executed
- How to fix: Implement option execution logic that runs the selected effect

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand initially")`
   - `assert!(game_state.player1.hand.cards.contains(&kanon_id), "Kanon should be in hand")`
   - `assert_eq!(game_state.player1.energy_zone.len(), 3, "Should have 3 energy initially")`

2. **After optional cost payment:**
   - `assert_eq!(game_state.player1.energy_zone.len(), 2, "Should have 2 energy after payment")`

3. **After effect execution:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards in hand after draw")`
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 49, "Deck should have 49 cards (50 - 1 drawn)")`

## Notes

- This test focuses on choice effect with optional cost
- The engine needs to support:
  - Optional energy cost payment
  - Choice effect with multiple options
  - Option selection and execution
  - ライブ開始時 trigger
- If choice effect is not implemented, the test may need to be adapted or the engine may need to be fixed
