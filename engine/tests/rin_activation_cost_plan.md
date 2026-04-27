# Test Plan: Rin Activation Cost and Live Card Recovery

## 1. Test Objective
Test the basic activation cost + effect execution flow:
- Pay activation cost (move self from stage to discard)
- Execute effect (move live card from discard to hand)
- Verify all game state changes are correct

## 2. Card Selection

**Primary Card:**
- Card ID: `PL!-sd1-005-SD`
- Name: 星空 凛
- Series: ラブライブ！
- Unit: lilywhite
- Cost: 2
- Type: メンバー
- Ability: `{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。`

**Why this card:**
- Simple, clear activation cost (move self from stage to discard)
- Simple effect (add live card from discard to hand)
- Tests fundamental ability resolution flow without complex mechanics
- No conditions, no optional costs, no sequential effects

**Verification:**
- Card exists in cards.json at line 242-246
- Ability is correctly parsed in abilities.json with:
  - Cost type: `move_cards`
  - Cost source: `stage`
  - Cost destination: `discard`
  - Cost self_cost: `true`
  - Effect type: `move_cards`
  - Effect source: `discard`
  - Effect destination: `hand`
  - Effect card_type: `live_card`

**Supporting Card:**
- Card ID: `PL!-sd1-019-SD`
- Name: START:DASH!!
- Type: ライブカード
- Purpose: Live card to be moved from discard to hand by the effect

## 3. Initial Game State

**Player 1:**
- `hand`: [] (empty)
- `main_deck`: [50 cards of PL!-sd1-005-SD] (not used in this test)
- `stage`: [-1, PL!-sd1-005-SD, -1, -1, -1] (Rin at center position index 1)
- `waitroom`: [PL!-sd1-019-SD] (START:DASH!! in discard)
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
- Parameters: cards.json path, card IDs "PL!-sd1-005-SD" and "PL!-sd1-019-SD"
- Expected output: Valid card IDs (non-zero)

**Step 2: Create game state with initial setup**
- Engine function: `Player::new`, `GameState::new`
- Parameters: Player names, card database
- Expected output: GameState with players initialized

**Step 3: Set up initial zones**
- Manual state manipulation:
  - `game_state.player1.stage.stage[1] = rin_id`
  - `game_state.player1.waitroom.add_card(aqours_live_id)`
- Expected intermediate state: Rin on stage, START:DASH!! in discard

**Step 4: Get Rin's 起動 ability**
- Engine function: `card_db.get_card`, iterate abilities
- Parameters: rin_id
- Expected output: Ability with trigger "起動"

**Step 5: Pay activation cost**
- Engine function: `AbilityResolver::pay_cost`
- Parameters: cost object from ability
- Expected intermediate state changes:
  - `game_state.player1.stage.stage[1]` becomes -1
  - `game_state.player1.waitroom` contains rin_id
- Expected output: Ok(())

**Step 6: Execute effect**
- Engine function: `AbilityResolver::execute_effect`
- Parameters: effect object from ability
- Expected intermediate state changes:
  - `game_state.player1.hand` contains aqours_live_id
  - `game_state.player1.waitroom` no longer contains aqours_live_id
- Expected output: Ok(())

**Step 7: Handle pending choice (if any)**
- Engine function: `resolver.get_pending_choice`, `resolver.provide_choice_result`
- Parameters: ChoiceResult::CardSelected with indices [0]
- Expected output: Ok(())

## 5. User Choices

**Choice 1 (if required):**
- Choice type: SelectCard
- Description: Select 1 live card from discard to add to hand
- Available options: [PL!-sd1-019-SD] (only one card)
- Selection: Index 0 (PL!-sd1-019-SD)
- Reason: Only option available
- Expected result: Card moved from discard to hand

## 6. Expected Final State

**Player 1:**
- `hand`: [PL!-sd1-019-SD] (START:DASH!! added)
- `main_deck`: [50 cards of PL!-sd1-005-SD] (unchanged)
- `stage`: [-1, -1, -1, -1, -1] (Rin removed)
- `waitroom`: [PL!-sd1-005-SD] (Rin added, START:DASH!! removed)
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

**Potential Fault 1: Cost payment not removing card from stage**
- Symptom: `game_state.player1.stage.stage[1]` still contains rin_id after cost payment
- Root cause: `execute_move_cards` not handling self_cost correctly
- Fix: Ensure self_cost flag properly moves activating card from stage to discard

**Potential Fault 2: Effect not moving card from discard to hand**
- Symptom: `game_state.player1.hand` does not contain aqours_live_id after effect execution
- Root cause: `execute_move_cards` not handling live_card type filter correctly
- Fix: Ensure card_type filter "live_card" correctly identifies live cards

**Potential Fault 3: Pending choice not handled**
- Symptom: execute_effect returns error "Pending choice required"
- Root cause: Test not providing choice result
- Fix: Add choice handling in test

## 8. Verification Assertions

1. Initial state verification:
   - `assert_eq!(game_state.player1.stage.stage[1], rin_id, "Rin should be on stage")`
   - `assert!(game_state.player1.waitroom.cards.contains(&aqours_live_id), "Live card should be in discard")`

2. Cost payment verification:
   - `assert_eq!(game_state.player1.stage.stage[1], -1, "Rin should be removed from stage")`
   - `assert!(game_state.player1.waitroom.cards.contains(&rin_id), "Rin should be in discard")`

3. Effect execution verification:
   - `assert!(game_state.player1.hand.cards.contains(&aqours_live_id), "Live card should be in hand")`
   - `assert!(!game_state.player1.waitroom.cards.contains(&aqours_live_id), "Live card should not be in discard")`

4. Final state verification:
   - `assert_eq!(game_state.player1.stage.stage[1], -1, "Stage position should be empty")`
   - `assert!(game_state.player1.waitroom.cards.contains(&rin_id), "Rin should be in discard")`
   - `assert!(game_state.player1.hand.cards.contains(&aqours_live_id), "Live card should be in hand")`
