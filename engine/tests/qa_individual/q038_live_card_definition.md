# Test Plan: Live Card Definition

## 1. Test Objective
Test the definition and handling of live cards. This tests:
- Live card identification (cards with "live" type)
- Live card placement in live_card_zone
- Live card vs member card distinction
- Live card ability triggering (live_start, live_success)

## 2. Card Selection
- **Card ID:** PL!S-bp2-024-L (君のこころは輝いてるかい？)
- **Card Name:** 君のこころは輝いてるかい？
- **Why this card:** This is a live card with live_success abilities, testing live card identification and ability triggering

## 3. Initial Game State

**Player 1:**
- `hand`: [live_card_1, member_card_1, member_card_2]
- `main_deck`: [energy cards...]
- `stage`: [member_card_3, -1, -1, -1, -1]
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

**Step 1: Verify live_card_1 is identified as live card**
- Engine function called: `card_database.get_card(live_card_1)`
- Expected: card.is_live() returns true
- Expected: card.is_member() returns false

**Step 2: Set live_card_1 as live card**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::SetLiveCard, card_id=live_card_1
- Expected: Live card placed in live_card_zone
- Expected: live_start abilities trigger (if any)

**Step 3: Verify live card is in correct zone**
- Check: player1.live_card_zone.cards contains live_card_1
- Check: player1.hand.cards does NOT contain live_card_1

**Step 4: Attempt to play live card as member to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=live_card_1, area=Center
- Expected: This should fail - live cards cannot be played as members

**Step 5: Execute live with live_card_1**
- Engine function called: Execute live (succeed)
- Expected: live_success abilities trigger (if any)
- Expected: Live card moves to success_live_card_zone on success

**Step 6: Verify live card moved to success zone**
- Check: player1.success_live_card_zone.cards contains live_card_1
- Check: player1.live_card_zone.cards does NOT contain live_card_1

## 5. User Choices

None - this tests engine card type identification and zone placement rules.

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1, member_card_2]
- `live_card_zone`: []
- `success_live_card_zone`: [live_card_1]
- `stage`: [member_card_3, -1, -1, -1, -1]

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Live cards are correctly identified by card.is_live()
- Live cards cannot be played as members to stage
- Live cards are placed in live_card_zone when set
- Live_start abilities trigger when live card is set
- Live_success abilities trigger when live succeeds
- Live cards move to success_live_card_zone on successful live
- Member cards are not identified as live cards
