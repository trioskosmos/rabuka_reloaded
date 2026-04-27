# Test Plan: Partial Effect Resolution

## 1. Test Objective
Test partial effect resolution when some effects fail. This tests:
- Sequential effects with partial failure
- Effect execution order
- State rollback on partial failure
- Effect independence

## 2. Card Selection
- **Card:** A card with ability: "Draw 2 cards, then discard 1 card"
- **Why this selection:** Tests sequential effects where second effect might fail

## 3. Initial Game State

**Player 1:**
- `hand`: [sequential_effect_card, member_card_1]
- `main_deck`: [member_card_2, member_card_3, energy cards...]
- `stage`: [member_card_4, member_card_5, -1, -1, -1]
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

**Step 1: Play sequential_effect_card to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=sequential_effect_card, area=Center
- Expected: Card placed in center stage, debut ability triggers

**Step 2: Debut ability executes - sequential effects**
- Engine function called: `AbilityResolver::execute_sequential`
- Effect 1: Draw 2 cards
- Expected: 2 cards drawn from deck to hand
- Effect 2: Discard 1 card
- Expected: 1 card discarded from hand to waitroom

**Step 3: Verify effect execution**
- Expected: Hand size: initial 2 - 1 played + 2 drawn - 1 discarded = 2 cards
- Expected: Deck size decreased by 2
- Expected: Waitroom size increased by 1

**Step 4: Test partial failure scenario**
- Play card with ability: "Draw 2 cards, then discard 2 cards" when hand has only 1 card
- Engine function called: `AbilityResolver::execute_sequential`
- Effect 1: Draw 2 cards (succeeds)
- Effect 2: Discard 2 cards (fails - only 1 card in hand)
- Expected: Either:
  - Option A: Both effects fail (rollback)
  - Option B: First effect succeeds, second fails (partial)

**Step 5: Verify partial failure handling**
- Expected: Engine handles partial failure per rules
- Expected: State consistent with chosen option

## 5. User Choices

None - sequential effects execute automatically.

## 6. Expected Final State

**Player 1:**
- `stage`: [member_card_4, member_card_5, sequential_effect_card, -1, -1]
- `hand`: [member_card_1, drawn_card_1, drawn_card_2] (after discard)
- `waitroom`: [discarded_card]
- `main_deck`: [energy cards... (top 2 removed)]

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Sequential effects execute in correct order
- Effects execute independently
- Partial failure handled correctly
- State consistent after partial failure
- Rollback implemented if required by rules
- Effect execution order preserved
