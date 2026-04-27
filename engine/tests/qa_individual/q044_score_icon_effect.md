# Test Plan: Score Icon Effect

## 1. Test Objective
Test the score icon effect on cards. This tests:
- Score icon contribution to live score
- Score icon vs blade/heart distinction
- Score icon timing (when it's counted)
- Multiple score icons on same card

## 2. Card Selection
- **Live Card:** A live card with score icon
- **Cheer Card:** A cheer card with score icon
- **Why this selection:** Tests score icon on both live and cheer cards

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1 (score icon: +5), cheer_card_1 (score icon: +3), member_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [cheer_card_2 (score icon: +2), member_card_4]
- `stage`: [member_card_5, member_card_6, -1, -1, -1]
- `success_live_card_zone`: []
- `live_card_zone`: []

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
- Expected: Score icon +5 tracked for live score

**Step 2: Cheer phase - Player 1 places cheer_card_1**
- User selects: cheer_card_1
- Expected: cheer_card_1 moved to success_live_card_zone
- Expected: Score icon +3 tracked for live score

**Step 3: Cheer phase - Player 2 places cheer_card_2**
- User selects: cheer_card_2
- Expected: cheer_card_2 moved to success_live_card_zone
- Expected: Score icon +2 tracked for live score

**Step 4: Execute live performance**
- Engine function called: Execute live performance
- Expected: Score icons counted toward live score
- Expected: Player 1 score: +5 (live) + +3 (cheer) = +8
- Expected: Player 2 score: +2 (cheer) = +2

**Step 5: Live success**
- Engine function called: Live succeeds
- Expected: Final scores calculated with score icons

**Step 6: Verify score calculation**
- Expected: Player 1 total score includes +8 from score icons
- Expected: Player 2 total score includes +2 from score icons

## 5. User Choices

**Choice 1 (Player 1 cheer selection):**
- Choice type: SelectCard
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_1
- Expected result: Card moved to success_live_card_zone

**Choice 2 (Player 2 cheer selection):**
- Choice type: SelectCard
- Available options: Member cards with cheer icons in hand
- Selection: cheer_card_2
- Expected result: Card moved to success_live_card_zone

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1]
- `success_live_card_zone`: [live_card_1, cheer_card_1]
- `stage`: [member_card_2, member_card_3, -1, -1, -1]
- Score icon contribution: +8

**Player 2:**
- `hand`: [member_card_4]
- `success_live_card_zone`: [cheer_card_2]
- `stage`: [member_card_5, member_card_6, -1, -1, -1]
- Score icon contribution: +2

## 7. Verification Assertions

- Score icons on live cards are counted toward live score
- Score icons on cheer cards are counted toward live score
- Score icons are counted during performance phase
- Score icons are counted only once per live
- Multiple score icons on different cards sum correctly
- Score icon contribution is independent of blade/heart totals
