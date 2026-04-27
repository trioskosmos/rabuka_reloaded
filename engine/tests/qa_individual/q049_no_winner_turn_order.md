# Test Plan: No Winner Turn Order

## 1. Test Objective
Test turn order when no winner is determined (draw). This tests:
- Draw conditions in live
- Turn order after draw
- Live card handling on draw
- Continued gameplay after draw

## 2. Card Selection
- **Live Card 1:** A live card
- **Live Card 2:** A live card with identical score/blade/heart
- **Why this selection:** Creates draw scenario

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, cheer_card_1 (3 blades, 2 hearts)]
- `main_deck`: [energy cards...]
- `stage`: [member_card_1, member_card_2, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [live_card_2, cheer_card_2 (3 blades, 2 hearts)]
- `stage`: [member_card_3, member_card_4, -1, -1, -1]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Both players set live cards**
- Player 1 sets live_card_1
- Player 2 sets live_card_2
- Expected: Both live cards in respective live_card_zone

**Step 2: Both players cheer phase**
- Player 1 places cheer_card_1
- Player 2 places cheer_card_2
- Expected: Both cheer cards in respective success_live_card_zone

**Step 3: Execute live performance**
- Engine function called: Execute live performance
- Expected: Both lives succeed
- Expected: Player 1: 3 blades, 2 hearts
- Expected: Player 2: 3 blades, 2 hearts

**Step 4: Victory determination**
- Engine function called: `TurnEngine::determine_live_victory`
- Expected: Scores are equal (draw)
- Expected: No winner declared (GameResult::Draw)

**Step 5: Verify turn order after draw**
- Expected: Turn order continues normally
- Expected: Phase advances appropriately
- Expected: Gameplay continues

**Step 6: Verify card handling**
- Expected: Live cards remain in success_live_card_zone
- Expected: Cheer cards remain in success_live_card_zone

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
- `success_live_card_zone`: [live_card_1, cheer_card_1]
- Score: Equal to Player 2
- Result: Draw

**Player 2:**
- `success_live_card_zone`: [live_card_2, cheer_card_2]
- Score: Equal to Player 1
- Result: Draw

**Game State:** GameResult::Draw, turn order continues

## 7. Verification Assertions

- Draw condition correctly detected when scores are equal
- No winner declared on draw
- Turn order continues normally after draw
- Gameplay continues after draw
- Live cards remain in success_live_card_zone on draw
- Cheer cards remain in success_live_card_zone on draw
- GameResult::Draw correctly set
