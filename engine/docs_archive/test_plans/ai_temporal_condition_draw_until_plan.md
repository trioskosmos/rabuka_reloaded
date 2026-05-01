# Test Plan: Ai Temporal Condition and Draw Until Count

## Test Objective
Test the 自動 ability of PL!N-bp3-005-R+ 宮下 愛 which has:
- Trigger: 自動
- Temporal condition: This turn, when 3 members have appeared on stage
- Effect: Draw cards until hand has 5 cards

This tests:
- Temporal conditions (tracking events during a turn)
- draw_until_count action
- 自動 trigger with condition
- Turn-based event tracking

## Card Selection
- **Primary card:** PL!N-bp3-005-R+ 宮下 愛 (cost 4)
- **Supporting members for stage appearances:** 
  - PL!N-bp1-001-R 優木せつ菜 (cost 3)
  - PL!N-bp1-002-R 三船栞子 (cost 3)
  - PL!N-bp1-003-R 平塚紗矢 (cost 3)
- **Why this card:** Tests temporal conditions which track events across a turn, a more complex mechanic than previous tests

## Initial Game State

**Player 1:**
- `hand`: [ai_id, setsuna_id, shiori_id, sayaka_id] (4 cards)
- `main_deck`: 50 cards (any cards)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `waitroom`: empty
- `energy_zone`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- Same structure as Player 1 (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "Main" (or appropriate phase for member appearances)
- `baton_touch_count`: 0
- `member_appearance_count`: 0 (tracked for temporal condition)

## Expected Action Sequence

**Step 1: First member appearance (setsuna_id)**
- Action: Play setsuna_id to stage position 0
- Engine function called: `play_member_to_stage` or similar
- Expected intermediate state changes:
  - setsuna_id moved from hand to stage
  - member_appearance_count incremented to 1
- Expected output: success, no ability trigger yet (condition not met)

**Step 2: Second member appearance (shiori_id)**
- Action: Play shiori_id to stage position 1
- Engine function called: `play_member_to_stage` or similar
- Expected intermediate state changes:
  - shiori_id moved from hand to stage
  - member_appearance_count incremented to 2
- Expected output: success, no ability trigger yet (condition not met)

**Step 3: Third member appearance (sayaka_id)**
- Action: Play sayaka_id to stage position 2
- Engine function called: `play_member_to_stage` or similar
- Expected intermediate state changes:
  - sayaka_id moved from hand to stage
  - member_appearance_count incremented to 3
  - Temporal condition now met (3 appearances this turn)
  - Ai's 自動 ability triggers
- Expected output: success, ability triggers and executes

**Step 4: Execute draw_until_count effect**
- Engine function called: `execute_effect` on the draw_until_count action
- Parameters passed: target_count = 5
- Expected intermediate state changes:
  - Engine calculates cards needed: 5 - current_hand_size = 5 - 1 = 1 card
  - 1 card drawn from deck to hand
- Expected output: success, hand now has 2 cards (ai_id + drawn card)

## User Choices

None - this is an automatic ability with no user choices.

## Expected Final State

**Player 1:**
- `hand`: [ai_id, +1 drawn card] (2 cards total)
- `main_deck`: 49 cards (50 - 1 drawn)
- `stage`: [setsuna_id, shiori_id, sayaka_id, -1, -1] (3 members)
- `waitroom`: empty
- `energy_zone`: empty
- `success_live_card_zone`: empty
- `live_card_zone`: empty

**Player 2:**
- Unchanged

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: "Main" (ability resolved, phase continues)
- `baton_touch_count`: 0
- `member_appearance_count`: 3

## Expected Engine Faults (if any)

**Potential fault 1: Temporal condition tracking**
- What fault: Engine may not track member appearances across the turn
- Expected failure mode: Condition never triggers because appearance count is not tracked
- How to fix: Implement turn-based event tracking for temporal conditions

**Potential fault 2: draw_until_count action**
- What fault: Engine may not support draw_until_count (draw until X cards in hand)
- Expected failure mode: Effect execution fails or draws wrong number of cards
- How to fix: Implement draw_until_count action that calculates needed cards and draws accordingly

**Potential fault 3: 自動 trigger with condition**
- What fault: Engine may not check conditions before triggering 自動 abilities
- Expected failure mode: Ability triggers immediately when card is on stage, not when condition is met
- How to fix: Implement condition checking for 自動 triggers

## Verification Assertions

1. **Initial state:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 4, "Should have 4 cards in hand initially")`
   - `assert!(game_state.player1.hand.cards.contains(&ai_id), "Ai should be in hand")`

2. **After 3 member appearances:**
   - `assert_eq!(game_state.player1.stage.stage[0], setsuna_id, "Setsuna should be on stage")`
   - `assert_eq!(game_state.player1.stage.stage[1], shiori_id, "Shiori should be on stage")`
   - `assert_eq!(game_state.player1.stage.stage[2], sayaka_id, "Sayaka should be on stage")`
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand (Ai only) before draw")`

3. **After ability execution:**
   - `assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards in hand after draw_until_count")`
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 49, "Deck should have 49 cards (50 - 1 drawn)")`

## Notes

- This test focuses on temporal conditions and draw_until_count
- The engine needs to support:
  - Turn-based event tracking (member appearances)
  - Temporal condition evaluation (count events this turn)
  - draw_until_count action (calculate needed cards and draw)
  - 自動 trigger with condition checking
- If temporal tracking is not implemented, the test may need to be adapted or the engine may need to be fixed
