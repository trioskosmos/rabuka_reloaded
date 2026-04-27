# Test Plan: Auto Ability Once Per Timing

## 1. Test Objective
Test that auto abilities can only trigger once per timing/event. This tests:
- Auto ability triggering rules (Rule 9.7.4)
- Once-per-timing restriction for auto abilities
- Multiple auto abilities on the same card
- Ability execution order when multiple trigger simultaneously

## 2. Card Selection
- **Card ID:** PL!N-bp1-051-R (高坂 穂乃果)
- **Card Name:** 高坂 穂乃果
- **Why this card:** Has multiple auto abilities that trigger on debut and area movement, testing the once-per-timing rule when multiple conditions are met simultaneously

## 3. Initial Game State

**Player 1:**
- `hand`: [member_card_1 (with multiple auto abilities), member_card_2, member_card_3]
- `main_deck`: [energy cards...]
- `stage`: [-1, -1, -1, -1, -1]
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

**Step 1: Play member_card_1 to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=member_card_1, area=Center
- Expected: Card placed in center stage, debut auto ability triggers

**Step 2: Verify only debut auto ability triggers**
- Expected: Only the debut-triggered auto ability executes
- Expected: Auto abilities with other triggers (e.g., area movement) do NOT trigger
- Expected: Only one execution of the debut auto ability (not multiple)

**Step 3: Move member_card_1 to different area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::MoveMember, card_id=member_card_1, from_area=Center, to_area=LeftSide
- Expected: Member moves, area movement auto ability triggers

**Step 4: Verify only area movement auto ability triggers**
- Expected: Only the area movement auto ability executes
- Expected: Debut auto ability does NOT trigger again
- Expected: Only one execution of the area movement auto ability

**Step 5: Move member_card_1 again to another area**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::MoveMember, card_id=member_card_1, from_area=LeftSide, to_area=RightSide
- Expected: Member moves, area movement auto ability triggers

**Step 6: Verify area movement auto ability triggers again**
- Expected: Area movement auto ability executes (new timing/event)
- Expected: This is allowed because it's a new area movement event
- Expected: Only one execution for this event

## 5. User Choices

None - auto abilities execute automatically without user choice (unless they have costs or selection requirements).

## 6. Expected Final State

**Player 1:**
- `stage`: [-1, -1, member_card_1, -1, -1] (in RightSide)
- `waitroom`: []
- Auto abilities triggered: debut (once), area movement (twice - once per move)

**Player 2:**
- Unchanged

## 7. Verification Assertions for the same event
- Auto abilities with different triggers trigger independently when their conditions are met
- Ability execution order follows the correct priority rules
- Engine tracks which abilities have triggered for each timing/event
