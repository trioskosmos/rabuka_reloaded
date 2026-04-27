# Q249: Card Removal Clears Modifiers

## 1. Test Objective
Test that when a card is removed from any zone (hand, stage, energy_zone, etc.), all its modifiers (blade_modifiers, heart_modifiers, orientation_modifiers, cost_modifiers, etc.) are cleared from the GameState. Currently the engine has a `clear_modifiers_for_card` function but it is not called when cards are removed from zones in most places.

## 2. Card Selection
Card ID: Any member card with blade/heart modifiers
Example: Any card that can gain blade/heart modifiers

Why this card was chosen: We need a card that can have modifiers applied to it, then test that those modifiers are cleared when the card is removed from play.

## 3. Initial Game State

**Player 1:**
- `hand`: [member_card_id]
- `main_deck`: [energy_card_ids...] (top to bottom)
- `stage`: [-1, -1, -1, -1, -1] (all empty)
- `waitroom`: []
- `energy_zone`: [energy_card_ids...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: []
- `main_deck`: [deck_cards...]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy_card_ids...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Other State:**
- `turn`: 1
- `current_player`: "player1"
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Play member to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, member_card_id, None, Some(MemberArea::Center), Some(false)
- Expected intermediate state changes: Member moves from hand to center stage, energy paid
- Expected output: success

**Step 2: Add blade modifier to member**
- Engine function called: `game_state.add_blade_modifier`
- Parameters: member_card_id, 2
- Expected intermediate state changes: Blade modifier added to member
- Expected output: success

**Step 3: Verify blade modifier exists**
- Engine function called: `game_state.get_blade_modifier`
- Parameters: member_card_id
- Expected intermediate state changes: None (just verification)
- Expected output: blade modifier >= 2

**Step 4: Remove member from stage (send to waitroom)**
- Engine function called: Need to find function to remove card from stage
- Parameters: member_card_id
- Expected intermediate state changes: Member removed from stage, sent to waitroom
- Expected output: success

**Step 5: Verify blade modifier is cleared**
- Engine function called: `game_state.get_blade_modifier`
- Parameters: member_card_id
- Expected intermediate state changes: None (just verification)
- Expected output: blade modifier == 0

## 5. User Choices (if applicable)
- None

## 6. Expected Final State

**Player 1:**
- `hand`: []
- `main_deck`: [remaining deck cards...]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: [member_card_id]
- `energy_zone`: [remaining energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []
- Blade modifier should be 0 (cleared after removal)

**Player 2:**
- Unchanged

## 7. Expected Engine Faults (if any)
Fault: When cards are removed from zones (hand.cards.remove(i), stage.remove, etc.), the `clear_modifiers_for_card` function is not called. This means blade_modifiers, heart_modifiers, orientation_modifiers, cost_modifiers, etc. remain in the GameState even after the card is removed from play. This could cause memory leaks or incorrect state if the same card_id is reused.

## 8. Verification Assertions
- Blade modifier was added to member
- Blade modifier is 0 after member is removed from stage
- No other modifiers remain for the removed card
