# Test Plan: Hanayo Sequential Cost and Draw

## 1. Test Objective
Test the sequential cost mechanism and simple draw effect:
- Pay sequential cost (wait self + discard 1 from hand)
- Execute draw effect (draw 1 from deck)
- Verify all game state changes are correct
- Verify turn limit (ターン1回) is respected

## 2. Card Selection

**Primary Card:**
- Card ID: `PL!-PR-012-PR`
- Name: 小泉花陽
- Series: ラブライブ！
- Unit: Printemps
- Cost: 4
- Type: メンバー
- Ability: `{{kidou.png|起動}}{{turn1.png|ターン1回}}このメンバーをウェイトにし、手札を1枚控え室に置く：カードを1枚引く。`

**Why this card:**
- Tests sequential cost (multiple cost steps in sequence)
- Tests state change to wait
- Tests hand discard as cost
- Tests simple draw effect
- Tests turn limit (ターン1回)
- Exposes engine handling of sequential_cost type

**Verification:**
- Card exists in cards.json at line 1598-1623
- Ability is correctly parsed in abilities.json with:
  - Cost type: `sequential_cost`
  - Cost costs array with 2 entries:
    1. type: `change_state`, state_change: `wait`, self_cost: `true`
    2. type: `move_cards`, source: `hand`, destination: `discard`, count: 1
  - Effect type: `draw_card`
  - Effect source: `deck`
  - Effect destination: `hand`
  - Effect count: 1
  - use_limit: 1 (ターン1回)

**Supporting Card:**
- Card ID: `PL!-sd1-005-SD` (for hand discard and deck)
- Purpose: Card to discard from hand and populate deck

## 3. Initial Game State

**Player 1:**
- `hand`: [PL!-sd1-005-SD] (1 card to discard)
- `main_deck`: [50 cards of PL!-sd1-005-SD] (cards to draw from)
- `stage`: [-1, PL!-PR-012-PR, -1, -1, -1] (Hanayo at center position index 1)
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
- Parameters: cards.json path, card IDs "PL!-PR-012-PR" and "PL!-sd1-005-SD"
- Expected output: Valid card IDs (non-zero)

**Step 2: Create game state with initial setup**
- Engine function: `Player::new`, `GameState::new`
- Parameters: Player names, card database
- Expected output: GameState with players initialized

**Step 3: Set up initial zones**
- Manual state manipulation:
  - `game_state.player1.stage.stage[1] = hanayo_id`
  - `game_state.player1.hand.add_card(other_member_id)`
  - Add 50 cards to deck
- Expected intermediate state: Hanayo on stage, 1 card in hand, 50 cards in deck

**Step 4: Get Hanayo's 起動 ability**
- Engine function: `card_db.get_card`, iterate abilities
- Parameters: hanayo_id
- Expected output: Ability with trigger "起動"

**Step 5: Verify turn limit**
- Engine function: Check ability.use_limit field
- Parameters: ability object
- Expected output: use_limit == 1

**Step 6: Pay sequential cost (step 1: wait self)**
- Engine function: `AbilityResolver::pay_cost`
- Parameters: cost object from ability (sequential_cost)
- Expected intermediate state changes:
  - First cost step executed: change_state to wait
  - Hanayo marked as wait state
- Expected output: Ok(()) with pending choice for second cost step

**Step 7: Handle hand discard choice**
- Engine function: `resolver.provide_choice_result`
- Parameters: ChoiceResult::CardSelected with indices [0]
- Expected intermediate state changes:
  - Second cost step executed: move_cards from hand to discard
  - `game_state.player1.hand` no longer contains other_member_id
  - `game_state.player1.waitroom` contains other_member_id
- Expected output: Ok(())

**Step 8: Execute effect (draw)**
- Engine function: `AbilityResolver::execute_effect`
- Parameters: effect object from ability
- Expected intermediate state changes:
  - 1 card drawn from deck to hand
  - `game_state.player1.main_deck.cards.len()` decreases by 1
  - `game_state.player1.hand.cards.len()` increases by 1
- Expected output: Ok(())

## 5. User Choices

**Choice 1: Hand discard for sequential cost**
- Choice type: SelectCard
- Description: Select 1 card from hand to discard
- Available options: [PL!-sd1-005-SD] (only one card)
- Selection: Index 0 (PL!-sd1-005-SD)
- Reason: Only option available
- Expected result: Card moved from hand to discard

## 6. Expected Final State

**Player 1:**
- `hand`: [PL!-sd1-005-SD] (1 card drawn from deck, 1 card discarded)
- `main_deck`: [49 cards of PL!-sd1-005-SD] (1 card drawn)
- `stage`: [-1, PL!-PR-012-PR, -1, -1, -1] (Hanayo still on stage but in wait state)
- `waitroom`: [PL!-sd1-005-SD] (discarded from hand)
- `energy_zone`: [] (unchanged)
- `success_live_card_zone`: [] (unchanged)
- `live_card_zone`: [] (unchanged)

**Note:** Hanayo remains on stage but is in wait state. The wait state is tracked separately from stage position.

**Player 2:**
- All zones unchanged from initial state

**Other State:**
- `turn`: 1 (unchanged)
- `current_player`: Player 1 (unchanged)
- `phase`: MainPhase (unchanged)
- `baton_touch_count`: 0 (unchanged)
- `ability_use_count`: Hanayo's 起動 ability use count incremented to 1

## 7. Expected Engine Faults

**Potential Fault 1: Sequential cost not executing both steps**
- Symptom: Only one cost step executed (either wait or discard, not both)
- Root cause: pay_cost not iterating through costs array in sequential_cost
- Fix: Ensure pay_cost executes all cost steps in sequence

**Potential Fault 2: Wait state not being tracked**
- Symptom: Hanayo not marked as wait after cost payment
- Root cause: change_state not properly setting wait state on card
- Fix: Ensure change_state with state_change="wait" properly marks card as wait

**Potential Fault 3: Turn limit not enforced**
- Symptom: Ability can be used multiple times in same turn
- Root cause: use_limit not being checked before ability activation
- Fix: Implement turn limit checking in ability activation logic

**Potential Fault 4: Draw not working**
- Symptom: Deck size not decreasing or hand size not increasing
- Root cause: execute_draw not removing card from deck or not adding to hand
- Fix: Ensure execute_draw properly moves card from deck to hand

## 8. Verification Assertions

1. Initial state verification:
   - `assert_eq!(game_state.player1.stage.stage[1], hanayo_id, "Hanayo should be on stage")`
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand")`
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 50, "Should have 50 cards in deck")`

2. Turn limit verification:
   - `assert_eq!(kidou_ability.use_limit, Some(1), "Should have turn limit of 1")`

3. Sequential cost step 1 verification (wait):
   - Check that Hanayo is marked as wait state (implementation-specific assertion)

4. Sequential cost step 2 verification (discard):
   - `assert!(!game_state.player1.hand.cards.contains(&other_member_id), "Discarded card should not be in hand")`
   - `assert!(game_state.player1.waitroom.cards.contains(&other_member_id), "Discarded card should be in discard")`

5. Draw effect verification:
   - `assert_eq!(game_state.player1.main_deck.cards.len(), 49, "Should have drawn 1 card from deck")`
   - `assert_eq!(game_state.player1.hand.cards.len(), 1, "Should have 1 card in hand (drawn, original discarded)")

6. Final state verification:
   - `assert_eq!(game_state.player1.stage.stage[1], hanayo_id, "Hanayo should still be on stage")`
   - `assert_eq!(game_state.player1.waitroom.cards.len(), 1, "Discard should have 1 card")`
