# Q246: Area Movement Trigger Auto Ability

## 1. Test Objective
Test that auto abilities triggered by area movement (領域移動誘発) work correctly when a card moves from one area to another. Specifically test Rule 9.7.4.1.2: when a card moves from stage to another area, the auto ability should use the card's information while it was on stage.

## 2. Card Selection
Card ID: PL!SP-pb1-006-R | 桜小路きな子

Full ability text: "{{jidou.png|自動}}このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。(対戦相手のカードの効果でも発動する。)"

Why this card was chosen: This card has an auto ability that triggers when it appears or moves areas ("登場か、エリアを移動するたび"), testing the area movement trigger mechanism. The ability grants blades to the opponent's member, which we can verify.

Verification: The card exists in cards.json (line 6914-6933) and the ability is correctly parsed with trigger_type "each_time".

## 3. Initial Game State

**Player 1:**
- `hand`: [sakurakoji_kinako_id]
- `main_deck`: [energy_card_ids...] (top to bottom)
- `stage`: [-1, -1, -1, -1, -1] (all empty)
- `waitroom`: []
- `energy_zone`: [energy_card_ids...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: [opponent_member_id]
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

**Step 1: Play sakurakoji_kinako to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, sakurakoji_kinako_id, None, Some(MemberArea::Center), Some(false)
- Expected intermediate state changes: Member moves from hand to center stage, energy paid, debut ability triggers
- Expected output: success

**Step 2: Play opponent member to stage**
- Engine function called: `TurnEngine::execute_main_phase_action` (for player2)
- Parameters: ActionType::PlayMemberToStage, opponent_member_id, None, Some(MemberArea::Center), Some(false)
- Expected intermediate state changes: Opponent member on stage
- Expected output: success

**Step 3: Move sakurakoji_kinako from center to left_side (area movement)**
- Engine function called: Need to find/implement area movement function
- Parameters: sakurakoji_kinako_id, MemberArea::LeftSide
- Expected intermediate state changes: Card moves from center to left_side, auto ability should trigger
- Expected output: success, auto ability queued

**Step 4: Process pending auto abilities**
- Engine function called: `game_state.process_pending_auto_abilities`
- Parameters: "player1"
- Expected intermediate state changes: Auto ability executes, opponent member gains 2 blades
- Expected output: success

## 5. User Choices (if applicable)
None expected for this test - auto abilities are mandatory.

## 6. Expected Final State

**Player 1:**
- `hand`: []
- `main_deck`: [remaining deck cards...]
- `stage`: [-1, sakurakoji_kinako_id, -1, -1, -1] (moved to left_side)
- `waitroom`: []
- `energy_zone`: [remaining energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- `hand`: []
- `main_deck`: [remaining deck cards...]
- `stage`: [-1, opponent_member_id, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [remaining energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []
- Opponent member should have +2 blades from the auto ability

## 7. Expected Engine Faults (if any)
Potential fault: The engine may not correctly trigger auto abilities on area movement (only on debut). The trigger_type "each_time" for area movement may not be implemented. Also, the engine may not track which card moved to trigger the ability.

## 8. Verification Assertions
- sakurakoji_kinako is in left_side after movement
- Auto ability was triggered and executed
- Opponent member gained 2 blades
- No other cards were affected incorrectly
