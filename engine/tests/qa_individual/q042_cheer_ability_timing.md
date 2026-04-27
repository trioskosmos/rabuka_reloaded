# Test Plan: Cheer Ability Timing

## 1. Test Objective
Test the timing of abilities on cheer cards when used as cheer. This tests:
- Cheer card ability triggering rules
- live_start abilities on cheer cards (should NOT trigger)
- live_success abilities on cheer cards (should trigger)
- Constant abilities on cheer cards (should be active during live)

## 2. Card Selection
- **Live Card:** A live card
- **Cheer Card 1:** Member card with live_start ability
- **Cheer Card 2:** Member card with live_success ability
- **Cheer Card 3:** Member card with constant ability
- **Why this selection:** Tests different ability types on cheer cards and their timing

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, cheer_card_1 (live_start), cheer_card_2 (live_success), cheer_card_3 (constant)]
- `main_deck`: [energy cards...]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
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

**Step 1: Set live_card_1**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=live_card_1
- Expected: Live card placed in live_card_zone
- Expected: live_start ability from live_card_1 triggers

**Step 2: Cheer phase - place all three cheer cards**
- User selects: cheer_card_1, cheer_card_2, cheer_card_3
- Expected: All three moved to success_live_card_zone
- Expected: cheer_card_1's live_start does NOT trigger (timing passed)
- Expected: cheer_card_3's constant ability becomes active (if applicable)

**Step 3: Execute live performance**
- Engine function called: Execute live performance
- Expected: cheer_card_3's constant ability is active during performance
- Expected: Blade/heart from all cheer cards counted

**Step 4: Live success**
- Engine function called: Live succeeds
- Expected: live_card_1's live_success triggers
- Expected: cheer_card_2's live_success triggers (cheer card in success zone)
- Expected: cheer_card_3's constant ability remains active (if duration allows)

**Step 5: Verify ability triggering**
- Expected: cheer_card_1 live_start: Did NOT trigger
- Expected: cheer_card_2 live_success: Triggered
- Expected: cheer_card_3 constant: Active during performance

## 5. User Choices

**Choice 1 (cheer card selection):**
- Choice type: SelectCard (multiple)
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_1, cheer_card_2, cheer_card_3
- Expected result: All moved to success_live_card_zone

**Choice 2 (confirm cheer):**
- Choice type: SelectOption
- Available options: Confirm, Add more
- Selection: Confirm
- Expected result: Cheer phase ends

## 6. Expected Final State

**Player 1:**
- `hand`: []
- `success_live_card_zone`: [live_card_1, cheer_card_1, cheer_card_2, cheer_card_3]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
- Ability triggers: cheer_card_1 live_start (NO), cheer_card_2 live_success (YES), cheer_card_3 constant (YES)

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Cheer cards with live_start do NOT trigger when placed as cheer
- Cheer cards with live_success trigger when live succeeds
- Cheer cards with constant abilities are active during performance
- Ability timing follows correct rules for cheer cards
- Cheer cards contribute blade/heart regardless of ability triggering
- Multiple cheer cards with different ability types all behave correctly
