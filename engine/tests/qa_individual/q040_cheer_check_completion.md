# Test Plan: Cheer Check Completion

## 1. Test Objective
Test that cheer check is completed correctly when live performance ends. This tests:
- Cheer blade/heart counting at live completion
- Victory determination based on cheer totals (Rule 8.4.2)
- Multiple cheer cards contributing to totals
- Cheer card removal after live completion

## 2. Card Selection
- **Live Card:** Any live card (e.g., PL!S-bp2-024-L)
- **Cheer Cards:** Multiple member cards with varying blade/heart values
- **Why this selection:** Tests comprehensive cheer counting and victory determination

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, cheer_card_1 (2 blades), cheer_card_2 (1 blade, 1 heart), member_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_2, member_card_3, member_card_4, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [cheer_card_3 (1 blade), member_card_5]
- `stage`: [member_card_6, member_card_7, -1, -1, -1]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Player 1 sets live_card_1**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=live_card_1
- Expected: Live card placed in live_card_zone

**Step 2: Player 1 cheer phase**
- User selects: cheer_card_1 and cheer_card_2
- Expected: Both cards moved to success_live_card_zone
- Expected: Cheer blade count: 2 + 1 = 3 blades
- Expected: Cheer heart count: 0 + 1 = 1 heart

**Step 3: Player 2 cheer phase**
- User selects: cheer_card_3
- Expected: Card moved to success_live_card_zone
- Expected: Cheer blade count: 1 blade
- Expected: Cheer heart count: 0 hearts

**Step 4: Execute live (succeed)**
- Engine function called: Execute live with success
- Expected: Live succeeds for both players
- Expected: Cheer blade/heart totals calculated

**Step 5: Victory determination**
- Engine function called: `TurnEngine::determine_live_victory`
- Expected: Blade totals: Player 1 = 3, Player 2 = 1
- Expected: Heart totals: Player 1 = 1, Player 2 = 0
- Expected: Player 1 wins (higher blade total)

**Step 6: Verify cheer cards remain in success zone**
- Expected: All cheer cards in respective success_live_card_zone

## 5. User Choices

**Choice 1 (Player 1 cheer selection):**
- Choice type: SelectCard (multiple)
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_1, cheer_card_2
- Expected result: Both moved to success_live_card_zone

**Choice 2 (Player 2 cheer selection):**
- Choice type: SelectCard
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_3
- Expected result: Card moved to success_live_card_zone

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1]
- `success_live_card_zone`: [live_card_1, cheer_card_1, cheer_card_2]
- `stage`: [member_card_2, member_card_3, member_card_4, -1, -1]
- Cheer blade total: 3
- Cheer heart total: 1

**Player 2:**
- `hand`: [member_card_5]
- `success_live_card_zone`: [cheer_card_3]
- `stage`: [member_card_6, member_card_7, -1, -1, -1]
- Cheer blade total: 1
- Cheer heart total: 0

**Game Result:** Player 1 wins live

## 7. Verification Assertions

- Cheer blade totals are calculated correctly for each player
- Cheer heart totals are calculated correctly for each player
- Victory determination uses correct blade/heart totals
- Multiple cheer cards contribute to totals
- Cheer cards remain in success_live_card_zone after live
- Victory is awarded to player with higher blade total (or heart if blades tied)
