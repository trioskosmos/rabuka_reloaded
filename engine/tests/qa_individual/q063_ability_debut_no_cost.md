# Q063: Ability Debut No Cost

## Test Objective
Test that when using an ability to debut a member to stage, you do not pay the member's cost separately from the ability cost.

## Q&A Reference
**Question:** When using an ability to debut a member to stage, do you pay the member's cost separately from the ability cost?
**Answer:** No, you don't pay the member's cost when debuting via ability effect.

## Card Selection
A member card that can be debuted via an ability (PL!N-bp1-002-R+).

**Primary Card:** PL!N-bp1-002-R+

## Initial Game State

**Player 1:**
- `hand`: [member_id] (member to be debuted via ability)
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [5 energy cards - less than member cost]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [member cards]
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## Expected Action Sequence

**Step 1: Verify member has cost > 0**
- Engine function called: `card_database.get_card(card_id)`
- Expected output: card.cost > 0

**Step 2: Verify player has less energy than member cost**
- Engine function called: Check `game_state.player1.energy_zone.cards.len()`
- Expected output: energy_count < member_cost

**Step 3: Execute ability that debuts member**
- Engine function called: Ability execution (specific to card ability)
- Expected intermediate state changes:
  - member_id moves from hand to stage
  - No additional energy payment for member cost
- Expected output: success (Ok(()))

## User Choices
None - deterministic

## Expected Final State

**Player 1:**
- `hand`: [] (member debuted via ability)
- `main_deck`: [remaining deck cards]
- `stage`: [member_id in some area]
- `waitroom`: []
- `energy_zone`: [same energy as before - member cost not paid]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- Unchanged from initial state

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## Expected Engine Faults
None - this is a normal gameplay scenario

## Verification Assertions
1. Member has cost > 0
2. Player has less energy than member cost
3. Ability debut succeeds despite insufficient energy for member cost
4. No compilation errors
5. No runtime panics
