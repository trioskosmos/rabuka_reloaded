# Test Plan: Once Per Turn Per Card

## 1. Test Objective
Test "once per turn per card" restriction. This tests:
- Turn-limited ability tracking per card
- Ability reuse prevention within same turn
- Ability reset on turn change
- Multiple cards with turn-limited abilities

## 2. Card Selection
- **Card 1:** Member card with ability: "Once per turn: Draw 1 card"
- **Card 2:** Member card with ability: "Once per turn: Draw 1 card"
- **Why this selection:** Tests turn-limited ability per card

## 3. Initial Game State

**Player 1:**
- `hand`: [turn_limited_card_1, turn_limited_card_2, member_card_1]
- `main_deck`: [member_card_2, member_card_3, member_card_4, energy cards...]
- `stage`: [member_card_5, member_card_6, -1, -1, -1]
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

**Step 1: Play turn_limited_card_1 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=turn_limited_card_1, area=Center
- Expected: Card placed in center stage

**Step 2: Activate turn_limited_card_1's ability**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=turn_limited_card_1
- Expected: Ability executes: draw 1 card
- Expected: Ability marked as used for this turn

**Step 3: Attempt to activate turn_limited_card_1's ability again**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=turn_limited_card_1
- Expected: Ability fails (already used this turn)
- Expected: No card drawn

**Step 4: Play turn_limited_card_2 to left side**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=turn_limited_card_2, area=LeftSide
- Expected: Card placed in left side stage

**Step 5: Activate turn_limited_card_2's ability**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=turn_limited_card_2
- Expected: Ability executes (different card, not used this turn)
- Expected: Ability marked as used for this turn

**Step 6: End turn**
- Engine function called: `TurnEngine::end_turn`
- Expected: Turn advances to turn 2
- Expected: Turn-limited abilities reset

**Step 7: Activate turn_limited_card_1's ability on turn 2**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::UseAbility, card_id=turn_limited_card_1
- Expected: Ability executes (reset on turn change)
- Expected: Ability marked as used for turn 2

## 5. User Choices

**Choice 1 (activate ability):**
- Choice type: SelectOption
- Available options: Activate ability, Skip
- Selection: Activate ability
- Expected result: Ability executes

## 6. Expected Final State

**Player 1:**
- `stage`: [turn_limited_card_2, turn_limited_card_1, member_card_5, member_card_6, -1]
- `hand`: [member_card_1, drawn_card_1, drawn_card_2, drawn_card_3]
- Turn: 2
- Abilities used: turn_limited_card_1 (turn 2), turn_limited_card_2 (turn 1)

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Turn-limited ability can be used once per turn per card
- Ability reuse prevented within same turn for same card
- Different cards can use their turn-limited abilities independently
- Turn-limited abilities reset on turn change
- Tracking is per card, not global
- Ability marked as used after activation
