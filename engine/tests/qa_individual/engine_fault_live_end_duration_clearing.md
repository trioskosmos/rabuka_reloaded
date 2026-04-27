# Q247: Live End Duration Effects Clearing

## 1. Test Objective
Test that effects with duration "ライブ終了時まで" (until live end) are properly cleared when the live phase ends, even if no live was performed in that turn. This tests Rule 9.7.5.1 and the duration management system.

## 2. Card Selection
Card ID: PL!SP-bp1-006-R | 桜小路きな子

Full ability text: "{{live_start.png|ライブ開始時}}{{icon_energy.png|E}}支払ってもよい：ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。"

Why this card was chosen: This card has a simple live_start ability that optionally pays 1 energy to gain 2 blades with duration "ライブ終了時まで". The rule specifies that these effects expire at the end of the live victory determination phase, regardless of whether a live was performed.

Verification: The card exists in cards.json (line 209-234) and the ability is correctly parsed with duration "live_end".

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

**Step 1: Play sakurakoji_kinako to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, sakurakoji_kinako_id, None, Some(MemberArea::Center), Some(false)
- Expected intermediate state changes: Member moves from hand to center stage, energy paid, debut ability triggers
- Expected output: success

**Step 2: Process debut auto ability (live_start ability)**
- Engine function called: `game_state.process_pending_auto_abilities`
- Parameters: "player1"
- Expected intermediate state changes: Live start ability executes, player chooses to pay 1 energy and gain 2 blades with live_end duration
- Expected output: success, temporary effect added with Duration::LiveEnd

**Step 3: Verify blade modifier is granted**
- Engine function called: `game_state.get_blade_modifier`
- Parameters: sakurakoji_kinako_id
- Expected intermediate state changes: None (just verification)
- Expected output: blade modifier >= 2

**Step 4: Advance turn without performing live**
- Engine function called: Manually set current_turn_phase to end turn state
- Parameters: Set current_turn_phase to something other than Live
- Expected intermediate state changes: Turn advances, check_expired_effects is called
- Expected output: success

**Step 5: Call check_expired_effects**
- Engine function called: `game_state.check_expired_effects`
- Parameters: None
- Expected intermediate state changes: Effects with Duration::LiveEnd should be removed
- Expected output: success, blade modifier removed

## 5. User Choices (if applicable)
- During live start ability: Choose to pay 1 energy to gain blades

## 6. Expected Final State

**Player 1:**
- `hand`: []
- `main_deck`: [remaining deck cards...]
- `stage`: [-1, sakurakoji_kinako_id, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [remaining energy cards... - 1 paid]
- `success_live_card_zone`: []
- `live_card_zone`: []
- Blade modifier should be 0 (cleared after turn end)

**Player 2:**
- Unchanged

## 7. Expected Engine Faults (if any)
Potential fault: The engine may not properly clear Duration::LiveEnd effects when the turn ends without a live being performed. The check_expired_effects function may only check current_turn_phase != TurnPhase::Live, but if the phase never was Live, the effect might not expire correctly.

## 8. Verification Assertions
- Blade modifier was granted during live start (>= 2)
- Blade modifier is 0 after check_expired_effects (even without live)
- No other temporary effects remain
