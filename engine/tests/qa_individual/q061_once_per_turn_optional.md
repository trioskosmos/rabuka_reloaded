# Test Plan: Once Per Turn Optional

## 1. Test Objective
Test "once per turn" optional abilities. This tests:
- Optional turn-limited abilities
- User choice for optional abilities
- Ability tracking when optional ability skipped
- Ability reset on turn change

## 2. Card Selection
- **Card:** A member card with optional ability: "Once per turn (optional): Draw 1 card"
- **Why this selection:** Tests optional turn-limited ability

## 3. Initial Game State

**Player 1:**
- `hand`: [optional_card, member_card_1]
- `main_deck`: [member_card_2, member_card_3, energy cards...]
- `stage`: [member_card_4, member_card_5, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
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

**Step 1: Play optional_card to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=optional_card, area=Center
- Expected: Card placed in center stage

**Step 2: Optional ability presented**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=optional_card
- Expected: User choice presented: Use ability or skip

**Step 3: User chooses to skip**
- User selection: Skip
- Expected: Ability not executed
- Expected: Ability not marked as used (optional, not used)

**Step 4: Attempt to use ability again**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=optional_card
- Expected: User choice presented again (not used yet)

**Step 5: User chooses to use ability**
- User selection: Use ability
- Expected: Ability executes: draw 1 card
- Expected: Ability marked as used for this turn

**Step 6: Attempt to use ability again**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=optional_card
- Expected: Fails (already used this turn)

**Step 7: End turn**
- Engine function called: `TurnEngine::end_turn`
- Expected: Turn advances to turn 2
- Expected: Ability reset

**Step 8: Use ability on turn 2**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=optional_card
- Expected: User choice presented (reset)
- Expected: Ability can be used again

## 5. User Choices

**Choice 1 (use or skip):**
- Choice type: SelectOption
- Available options: Use ability, Skip
- Selection: Skip
- Expected result: Ability not executed, not marked as used

**Choice 2 (use or skip):**
- Choice type: SelectOption
- Available options: Use ability, Skip
- Selection: Use ability
- Expected result: Ability executes, marked as used

## 6. Expected Final State

**Player 1:**
- `stage`: [member_card_4, member_card_5, optional_card, -1, -1]
- `hand`: [member_card_1, drawn_card]
- Turn: 2
- Ability used: Yes (on turn 1)

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Optional abilities present user choice
- User can skip optional ability
- Skipped ability not marked as used
- Skipped ability can be used later in same turn
- Used ability cannot be used again in same turn
- Optional abilities reset on turn change
- Once-per-turn restriction applies even when optional
