# Test Plan: Full Cost Required

## 1. Test Objective
Test that full cost must be paid for abilities. This tests:
- Cost payment requirements
- Partial cost payment rejection
- Cost validation before effect execution
- Cost rollback on failure

## 2. Card Selection
- **Card:** A member card with ability requiring cost: "Pay 2 energy: Draw 1 card"
- **Why this selection:** Tests cost payment requirements

## 3. Initial Game State

**Player 1:**
- `hand`: [cost_card, member_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy_card_1, energy_card_2, energy_card_3] (3 active energy)
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- Same structure (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Play cost_card to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=cost_card, area=Center
- Expected: Card placed in center stage, debut ability triggers

**Step 2: Attempt to activate ability with insufficient energy**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=cost_card
- Expected: Cost validation fails (only 3 energy, need 2 but maybe other constraints)
- Expected: Ability not activated
- Expected: No energy paid

**Step 3: Activate ability with sufficient energy**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=cost_card
- Expected: Cost validation passes
- Expected: 2 energy deactivated
- Expected: Ability effect executes: draw 1 card

**Step 4: Verify cost payment**
- Expected: 2 energy deactivated
- Expected: 1 energy still active
- Expected: 1 card drawn

**Step 5: Attempt partial cost payment**
- Test scenario where user tries to pay only 1 energy for 2 energy cost
- Expected: Partial payment rejected
- Expected: No energy paid
- Expected: Ability not activated

## 5. User Choices

**Choice 1 (activate ability):**
- Choice type: SelectOption
- Available options: Activate ability, Skip
- Selection: Activate ability
- Expected result: Cost payment requested

**Choice 2 (pay cost):**
- Choice type: SelectCard
- Available options: Active energy cards
- Selection: 2 energy cards
- Expected result: Energy deactivated, ability executes

## 6. Expected Final State

**Player 1:**
- `stage`: [member_card_2, member_card_3, cost_card, -1, -1]
- `hand`: [member_card_1, drawn_card]
- `energy_zone`: [energy_card_3] (1 active, 2 deactivated)
- Cost paid: 2 energy

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Full cost must be paid for ability activation
- Partial cost payment rejected
- Cost validated before effect execution
- Cost rolled back on effect failure
- Energy deactivated when cost paid
- Ability effect executes only after full cost paid
