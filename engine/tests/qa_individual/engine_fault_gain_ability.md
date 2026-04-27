# Q248: Gain Ability Full Implementation

## 1. Test Objective
Test that the `gain_ability` action actually grants abilities to cards, not just tracks them as prohibition effects. Currently the engine only tracks gain_ability as a prohibition effect but doesn't actually grant the ability to the target card.

## 2. Card Selection
Card ID: Any card with `gain_ability` effect
Example: PL!SP-bp1-007-R+ | 米女メイ

Full ability text: "{{toujyou.png|登場}}自分のエネルギーが11枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。\n{{jyouji.png|常時}}自分のライブ中のカードが3枚以上あり、その中に『虹ヶ咲』のライブカードを1枚以上含む場合、{{icon_all.png|ハート}}{{icon_all.png|ハート}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。\n{{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}：自分の控え室からライブカードを1枚手札に加える。"

Why this card was chosen: This card has a constant ability that can be granted via gain_ability. The constant ability grants hearts and blades when certain conditions are met. This is a good test case for verifying that gained abilities actually function.

Verification: The card exists in cards.json and the ability is correctly parsed with action "gain_ability".

## 3. Initial Game State

**Player 1:**
- `hand`: [member_with_gain_ability_id, target_member_id]
- `main_deck`: [energy_card_ids...] (top to bottom)
- `stage`: [-1, -1, -1, -1, -1] (all empty)
- `waitroom`: []
- `energy_zone`: [energy_card_ids...]
- `success_live_card_zone`: []
- `live_card_zone`: [live_card_ids...] (3+ cards including 虹ヶ咲)

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

**Step 1: Play member_with_gain_ability to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, member_with_gain_ability_id, None, Some(MemberArea::Center), Some(false)
- Expected intermediate state changes: Member moves from hand to center stage, energy paid, debut ability triggers
- Expected output: success

**Step 2: Process debut auto ability**
- Engine function called: `game_state.process_pending_auto_abilities`
- Parameters: "player1"
- Expected intermediate state changes: Debut ability executes (if any)
- Expected output: success

**Step 3: Use activation ability to grant constant ability to target**
- Engine function called: Need to find activation ability function
- Parameters: Target member, ability to grant
- Expected intermediate state changes: Target member gains the constant ability
- Expected output: success, ability is actually added to target's abilities

**Step 4: Play target_member to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, target_member_id, None, Some(MemberArea::LeftSide), Some(false)
- Expected intermediate state changes: Target member moves to stage with gained ability
- Expected output: success

**Step 5: Verify gained ability functions**
- Engine function called: Check if constant ability applies
- Parameters: target_member_id
- Expected intermediate state changes: Constant ability should grant hearts/blades
- Expected output: success, target has gained ability effects

## 5. User Choices (if applicable)
- During activation: Choose target member to grant ability to

## 6. Expected Final State

**Player 1:**
- `hand`: []
- `main_deck`: [remaining deck cards...]
- `stage`: [target_member_id, member_with_gain_ability_id, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [remaining energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: [live_card_ids...]
- Target member should have gained ability active

**Player 2:**
- Unchanged

## 7. Expected Engine Faults (if any)
Fault: The engine's `execute_gain_ability` function only tracks the ability as a prohibition effect in `prohibition_effects` vector. It doesn't actually add the ability to the target card's abilities list or create a mechanism for the gained ability to function. The ability text is stored but never parsed or executed.

## 8. Verification Assertions
- gain_ability was called and tracked
- Target member has the gained ability in its abilities list
- Gained ability's effects are applied (hearts/blades granted)
- Prohibition effects vector contains the gain_ability entry
