# Test Plan: Cheer Card Timing

## 1. Test Objective
Test the timing of when cheer cards can be placed and their effects. This tests:
- Cheer phase timing (Rule 8.3)
- Cheer card placement before live execution
- Cheer card effects during live performance
- Cheer card contribution to blade/heart totals at correct timing

## 2. Card Selection
- **Live Card:** A live card with live_success abilities
- **Cheer Card:** A cheer card with abilities that trigger during live
- **Why this selection:** Tests timing of cheer card placement and ability triggering

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, cheer_card_1 (with live_start ability), member_card_1]
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

**Step 1: Set live_card_1**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=live_card_1
- Expected: Live card placed in live_card_zone
- Expected: live_start abilities trigger (from live card only, not cheer cards yet)

**Step 2: Advance to cheer phase**
- Engine function called: `TurnEngine::advance_phase`
- Expected: Phase advances to Cheer
- Expected: Engine presents choice for cheer card selection

**Step 3: Place cheer_card_1**
- User selects: cheer_card_1
- Expected: cheer_card_1 moved to success_live_card_zone
- Expected: cheer_card_1's live_start ability does NOT trigger (already past live_start timing)

**Step 4: Confirm cheer selection**
- User confirms cheer selection
- Expected: Cheer phase ends, live execution begins

**Step 5: Execute live (performance phase)**
- Engine function called: Execute live performance
- Expected: Cheer cards contribute blade/heart during performance
- Expected: Cheer card abilities (if any) trigger at appropriate timings

**Step 6: Live success**
- Engine function called: Live succeeds
- Expected: live_success abilities trigger (from live card)
- Expected: Cheer card live_success abilities trigger (if any)

**Step 7: Verify timing**
- Expected: cheer_card_1 did NOT trigger live_start (wrong timing)
- Expected: cheer_card_1 contributed blade/heart during performance
- Expected: cheer_card_1's live_success triggered (if applicable)

## 5. User Choices

**Choice 1 (cheer card selection):**
- Choice type: SelectCard
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_1
- Expected result: Card moved to success_live_card_zone

**Choice 2 (confirm cheer):**
- Choice type: SelectOption
- Available options: Confirm, Add more
- Selection: Confirm
- Expected result: Cheer phase ends

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1]
- `success_live_card_zone`: [live_card_1, cheer_card_1]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
- Cheer blade/heart counted: Yes
- cheer_card_1 live_start: Did NOT trigger (timing)
- cheer_card_1 live_success: Triggered (if applicable)

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Cheer cards are placed during cheer phase only
- Cheer cards do NOT trigger live_start abilities (timing passed)
- Cheer cards contribute blade/heart during performance phase
- Cheer cards trigger live_success abilities when live succeeds
- Cheer card timing follows Rule 8.3 correctly
- Abilities trigger at correct timings based on card type and zone
