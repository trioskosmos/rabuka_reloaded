# Test Plan: Multiple Success Draw

## 1. Test Objective
Test drawing cards from multiple live successes. This tests:
- Multiple live_success abilities triggering
- Sequential draw effects from multiple sources
- Draw effect stacking
- Hand size management

## 2. Card Selection
- **Live Card 1:** Live card with live_success: draw 2 cards
- **Live Card 2:** Live card with live_success: draw 1 card
- **Cheer Card:** Cheer card with live_success: draw 1 card
- **Why this selection:** Tests multiple draw effects from live_success

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, live_card_2, cheer_card_1, member_card_1]
- `main_deck`: [member_card_2, member_card_3, member_card_4, member_card_5, member_card_6, energy cards...]
- `stage`: [member_card_7, member_card_8, -1, -1, -1]
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

**Step 2: Cheer phase - place cheer_card_1**
- User selects: cheer_card_1
- Expected: cheer_card_1 moved to success_live_card_zone

**Step 3: Execute live (succeed)**
- Engine function called: Execute live with success
- Expected: live_card_1's live_success triggers: draw 2 cards
- Expected: cheer_card_1's live_success triggers: draw 1 card
- Expected: Total drawn: 2 + 1 = 3 cards

**Step 4: Verify hand size**
- Expected: Hand size increased by 3
- Expected: Deck size decreased by 3

**Step 5: Set live_card_2 and succeed**
- Engine function called: Set live_card_2, execute live with success
- Expected: live_card_2's live_success triggers: draw 1 card
- Expected: Hand size increased by 1
- Expected: Deck size decreased by 1

**Step 6: Verify total draws**
- Expected: Total drawn: 3 (first live) + 1 (second live) = 4 cards
- Expected: Hand size: initial 4 - 3 played + 4 drawn = 5 cards

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
- `hand`: [member_card_1, drawn_card_1, drawn_card_2, drawn_card_3, drawn_card_4]
- `main_deck`: [energy cards... (top 4 removed)]
- `success_live_card_zone`: [live_card_1, cheer_card_1, live_card_2]
- `stage`: [member_card_7, member_card_8, -1, -1, -1]
- Total drawn: 4 cards

**Player 2:**
- Unchanged

## 7. Verification Assertions

- All live_success abilities trigger on live success
- Draw effects from multiple sources sum correctly
- Hand size updated correctly after each draw
- Deck size decreased correctly
- Multiple live successes can trigger in sequence
- Draw effects execute in correct order
