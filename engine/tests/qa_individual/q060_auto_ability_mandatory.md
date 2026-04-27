# Test Plan: Auto Ability Mandatory

## 1. Test Objective
Test that auto abilities are mandatory (cannot be skipped). This tests:
- Auto ability mandatory execution
- No user choice for auto abilities
- Auto ability triggering conditions
- Auto ability vs activation ability distinction

## 2. Card Selection
- **Card:** A member card with mandatory auto ability on debut
- **Why this selection:** Tests mandatory auto ability execution

## 3. Initial Game State

**Player 1:**
- `hand`: [auto_ability_card, member_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
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

**Step 1: Play auto_ability_card to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=auto_ability_card, area=Center
- Expected: Card placed in center stage
- Expected: Auto ability triggers immediately
- Expected: No user choice presented (mandatory)

**Step 2: Auto ability executes**
- Engine function called: `AbilityResolver::execute_auto_ability`
- Expected: Auto ability effect executes automatically
- Expected: No user intervention required

**Step 3: Verify auto ability executed**
- Expected: Auto ability effect applied (e.g., draw card, add blade, etc.)
- Expected: No user choice was presented

**Step 4: Compare with activation ability**
- Play card with activation ability
- Expected: User choice presented (activate or skip)
- Expected: Activation ability is optional

**Step 5: Verify distinction**
- Expected: Auto abilities are mandatory
- Expected: Activation abilities are optional

## 5. User Choices

None - auto abilities are mandatory and execute automatically.

## 6. Expected Final State

**Player 1:**
- `stage`: [member_card_2, member_card_3, auto_ability_card, -1, -1]
- `hand`: [member_card_1]
- Auto ability effect: Applied

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Auto abilities trigger when conditions met
- Auto abilities execute automatically
- No user choice presented for auto abilities
- Auto abilities cannot be skipped
- Auto abilities are mandatory
- Activation abilities are optional (for comparison)
- Auto ability vs activation ability distinction maintained
