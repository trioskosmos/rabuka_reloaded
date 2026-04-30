# Test Plan: Sequential Effects - Draw and Discard

## 1. Test Objective
Test a live_success ability with sequential effects (draw cards, then discard). This tests:
- Live_success ability triggering
- Sequential effect execution (multiple actions in sequence)
- Draw card action
- Discard card action
- Order of execution (draw first, then discard)

## 2. Card Selection
- **Card ID:** PL!S-bp2-024-L
- **Card Name:** 君のこころは輝いてるかい？
- **Full Ability:** {{live_success.png|ライブ成功時}}カードを2枚引き、手札を1枚控え室に置く。
- **Why this card:** This card has a live_success ability with sequential effects (draw 2, discard 1). It tests the sequential action execution order.

## 3. Initial Game State

**Player 1:**
- `hand`: [君のこころは輝いてるかい？ (live card), member_card_1]
- `main_deck`: [member_card_2, member_card_3, member_card_4, energy cards...]
- `stage`: [member_card_5, -1, -1, -1, -1]
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

**Step 1: Set live card and member on stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=君のこころは輝いてるかい？
- Expected: Live card placed in live_card_zone

**Step 2: Execute live (succeed)**
- Engine function called: Execute live with success
- Expected: Live_success ability triggers

**Step 3: Sequential effect executes**
- Action 1: Draw 2 cards from deck
- Expected: 2 cards moved from deck to hand
- Action 2: Discard 1 card from hand
- Expected: 1 card moved from hand to waitroom

**Step 4: Verify final state**
- Expected: Hand size increased by 1 (2 drawn - 1 discarded)
- Expected: Deck size decreased by 2
- Expected: Waitroom size increased by 1

## 5. User Choices

None - sequential effects execute automatically without user choice.

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1, member_card_2, member_card_3] (initial 2 + 2 drawn - 1 discarded)
- `main_deck`: [member_card_4, energy cards...] (top 2 removed)
- `stage`: [member_card_5, -1, -1, -1, -1]
- `waitroom`: [member_card_6] (1 discarded)
- `live_card_zone`: [君のこころは輝いてるかい？]

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Live_success ability triggers on live success
- 2 cards are drawn from deck
- 2 cards are added to hand
- 1 card is discarded from hand
- 1 card is added to waitroom
- Final hand size = initial + 1
- Final deck size = initial - 2
- Final waitroom size = initial + 1
