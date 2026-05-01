# Test Plan: Kasumi Optional Cost and Look-and-Select

## 1. Test Objective
Test the optional cost mechanism and look-and-select effect:
- Pay optional cost (discard 1 from hand)
- Look at top 3 of deck
- Select 1 to hand, rest to discard
- Verify all game state changes are correct

## 2. Card Selection

**Primary Card:**
- Card ID: `PL!N-PR-004-PR`
- Name: 中須かすみ
- Series: ラブライブ！虹ヶ咲学園スクールアイドル同好会
- Unit: QU4RTZ
- Cost: 4
- Type: メンバー
- Ability: `{{toujyou.png|登場}}手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`

**Why this card:**
- Tests optional cost mechanism ("手札を1枚控え室に置いてもよい")
- Tests look-and-select effect pattern
- Tests 登場 trigger timing
- Exposes parser/engine handling of look_and_select action type

**Verification:**
- Card exists in cards.json at line 3825-3848
- Ability is correctly parsed in abilities.json with:
  - Cost type: `move_cards`
  - Cost source: `hand`
  - Cost destination: `discard`
  - Cost optional: `true`
  - Effect type: `look_and_select`
  - Effect look_action type: `look_at`
  - Effect look_action source: `deck_top`
  - Effect look_action count: 3
  - Effect select_action destination: `discard`
  - Effect select_action count: 1

**Supporting Cards:**
- Card ID: `PL!-sd1-002-SD` (for hand discard and deck)
- Card ID: `PL!-sd1-003-SD` (for deck)
- Card ID: `PL!-sd1-004-SD` (for deck)
- Purpose: Cards to populate hand and deck for the test

## 3. Initial Game State

**Player 1:**
- `hand`: [PL!N-PR-004-PR, PL!-sd1-002-SD] (Kasumi + 1 card to discard)
- `main_deck`: [PL!-sd1-003-SD, PL!-sd1-004-SD, PL!-sd1-002-SD, ...47 more cards] (top 3 are the ones to look at)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `waitroom`: [] (empty)
- `energy_zone`: [] (empty)
- `success_live_card_zone`: [] (empty)
- `live_card_zone`: [] (empty)

**Player 2:**
- `hand`: [] (empty)
- `main_deck`: [50 cards of PL!-sd1-005-SD] (not used in this test)
- `stage`: [-1, -1, -1, -1, -1] (empty)
- `waitroom`: [] (empty)
- `energy_zone`: [] (empty)
- `success_live_card_zone`: [] (empty)
- `live_card_zone`: [] (empty)

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: MainPhase
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Load card database and get card IDs**
- Engine function: `CardLoader::load_cards_from_file`, `CardDatabase::load_or_create`, `get_card_id`
- Parameters: cards.json path, card IDs "PL!N-PR-004-PR", "PL!-sd1-002-SD", "PL!-sd1-003-SD", "PL!-sd1-004-SD"
- Expected output: Valid card IDs (non-zero)

**Step 2: Create game state with initial setup**
- Engine function: `Player::new`, `GameState::new`
- Parameters: Player names, card database
- Expected output: GameState with players initialized

**Step 3: Set up initial zones**
- Manual state manipulation:
  - `game_state.player1.hand.add_card(kasumi_id)`
  - `game_state.player1.hand.add_card(member1_id)`
  - `game_state.player1.main_deck.cards.push(member2_id)`
  - `game_state.player1.main_deck.cards.push(member3_id)`
  - `game_state.player1.main_deck.cards.push(member1_id)`
  - Add 47 more cards to deck
- Expected intermediate state: Kasumi + 1 card in hand, 50 cards in deck

**Step 4: Get Kasumi's 登場 ability**
- Engine function: `card_db.get_card`, iterate abilities
- Parameters: kasumi_id
- Expected output: Ability with trigger "登場"

**Step 5: Verify cost is optional**
- Engine function: Check cost.optional field
- Parameters: cost object from ability
- Expected output: cost.optional == true

**Step 6: Execute effect (with optional cost)**
- Engine function: `AbilityResolver::execute_effect`
- Parameters: effect object from ability
- Expected intermediate state changes:
  - Pending choice for optional cost (discard from hand)
- Expected output: Ok(()) with pending choice

**Step 7: Handle optional cost choice**
- Engine function: `resolver.provide_choice_result`
- Parameters: ChoiceResult::CardSelected with indices [0] (choose to pay cost)
- Expected intermediate state changes:
  - `game_state.player1.hand` no longer contains member1_id
  - `game_state.player1.waitroom` contains member1_id
- Expected output: Ok(()) with next pending choice

**Step 8: Handle look_at step**
- Engine function: `execute_look_and_select` -> `execute_effect` on look_action
- Parameters: look_action from effect
- Expected intermediate state changes:
  - Top 3 cards removed from deck
  - Cards stored in `looked_at_cards` temporary zone
- Expected output: Ok(()) with pending choice for selection

**Step 9: Handle select choice**
- Engine function: `resolver.provide_choice_result`
- Parameters: ChoiceResult::CardSelected with indices [0] (select first card from looked_at)
- Expected intermediate state changes:
  - Selected card moved to hand
  - Remaining 2 cards moved to discard
- Expected output: Ok(())

## 5. User Choices

**Choice 1: Optional cost**
- Choice type: SelectCard
- Description: Select 1 card from hand to discard (or skip)
- Available options: [PL!-sd1-002-SD] (the non-Kasumi card)
- Selection: Index 0 (PL!-sd1-002-SD)
- Reason: Choose to pay the optional cost to enable the effect
- Expected result: Card moved from hand to discard

**Choice 2: Card selection from looked_at**
- Choice type: SelectCard
- Description: Select 1 card from the 3 looked-at cards to add to hand
- Available options: [PL!-sd1-003-SD, PL!-sd1-004-SD, PL!-sd1-002-SD] (top 3 of deck)
- Selection: Index 0 (PL!-sd1-003-SD)
- Reason: First card from top of deck
- Expected result: PL!-sd1-003-SD moved to hand, PL!-sd1-004-SD and PL!-sd1-002-SD moved to discard

## 6. Expected Final State

**Player 1:**
- `hand`: [PL!N-PR-004-PR, PL!-sd1-003-SD] (Kasumi remains, selected card added, discarded card removed)
- `main_deck`: [47 cards] (top 3 removed)
- `stage`: [-1, -1, -1, -1, -1] (unchanged)
- `waitroom`: [PL!-sd1-002-SD, PL!-sd1-004-SD, PL!-sd1-002-SD] (discarded from hand + 2 from looked_at)
- `energy_zone`: [] (unchanged)
- `success_live_card_zone`: [] (unchanged)
- `live_card_zone`: [] (unchanged)

**Player 2:**
- All zones unchanged from initial state

**Other State:**
- `turn`: 1 (unchanged)
- `current_player`: Player 1 (unchanged)
- `phase`: MainPhase (unchanged)
- `baton_touch_count`: 0 (unchanged)

## 7. Expected Engine Faults

**Potential Fault 1: Optional cost not being offered**
- Symptom: execute_effect succeeds immediately without pending choice
- Root cause: Engine not respecting optional flag in cost
- Fix: Ensure optional costs generate pending choices

**Potential Fault 2: look_and_select not implemented correctly**
- Symptom: execute_effect returns error for look_and_select action
- Root cause: execute_look_and_select not handling look_action or select_action
- Fix: Implement proper look_and_select execution with looked_at_cards temporary zone

**Potential Fault 3: Cards not moved from looked_at to correct destinations**
- Symptom: Selected card not in hand, or remaining cards not in discard
- Root cause: select_action not moving cards correctly from looked_at zone
- Fix: Ensure select_action moves selected cards to destination and rest to discard

**Potential Fault 4: Deck not reduced by 3 after look_at**
- Symptom: Deck still has 50 cards after look_at
- Root cause: look_at not removing cards from deck
- Fix: Ensure look_at removes cards from deck_top and stores in looked_at_cards

## 8. Verification Assertions

1. Initial state verification:
   - `assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards in hand")`
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 50, "Should have 50 cards in deck")`

2. Cost optional verification:
   - `assert!(cost.optional.unwrap_or(false), "Cost should be optional")`

3. Optional cost payment verification:
   - `assert!(!game_state.player1.hand.cards.contains(&member1_id), "Discarded card should not be in hand")`
   - `assert!(game_state.player1.waitroom.cards.contains(&member1_id), "Discarded card should be in discard")`

4. Look_at verification:
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 47, "Deck should have 3 fewer cards")`
   - `assert_eq!(resolver.looked_at_cards.len(), 3, "Should have 3 looked-at cards")`

5. Select verification:
   - `assert!(game_state.player1.hand.cards.contains(&member2_id), "Selected card should be in hand")`
   - `assert!(game_state.player1.waitroom.cards.contains(&member3_id), "Unselected card should be in discard")`
   - `assert!(game_state.player1.waitroom.cards.contains(&member1_id), "Unselected card should be in discard")`

6. Final state verification:
   - `assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards in hand (Kasumi + selected)")`
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 47, "Deck should have 47 cards")`
   - `assert_eq!(game_state.player1.waitroom.cards.len(), 3, "Discard should have 3 cards")
