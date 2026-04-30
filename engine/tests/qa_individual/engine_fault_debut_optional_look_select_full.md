# Test Plan: Full End-to-End Debut with Optional Cost and Look-and-Select

## 1. Test Objective
Test a debut ability with optional cost and look_and_select effect end-to-end. This tests:
- Debut ability triggering when playing member to stage
- Optional cost presentation and execution
- Look_and_select effect execution
- User choice handling for both optional cost and card selection
- Card movement from deck to hand and discard

## 2. Card Selection
- **Card ID:** PL!-sd1-011-SD
- **Card Name:** 絢瀬 絵里
- **Full Ability:** {{toujyou.png|登場}}手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。
- **Why this card:** This card has a debut ability with optional cost followed by look_and_select. It tests the complete flow of debut trigger, optional cost payment, deck look, user selection, and card movement using only real engine functions.

## 3. Initial Game State

**Player 1:**
- `hand`: [絢瀬 絵里 (card to play), member_card_2 (for optional cost), member_card_3]
- `main_deck`: [member_card_4, member_card_5, member_card_6, energy cards...]
- `stage`: [-1, -1, -1, -1, -1]
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

**Step 1: Play 絢瀬 絵里 to stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=絢瀬 絵里, area=Center
- Expected: Card placed in center stage, debut ability triggers

**Step 2: Optional cost choice presented**
- Engine presents choice: Pay optional cost (discard 1 card from hand) or skip
- User selects: Pay cost (discard member_card_2)
- Expected: member_card_2 moved to waitroom

**Step 3: Look action executes**
- Engine executes look_at: top 3 cards from deck
- Expected: Cards [member_card_4, member_card_5, member_card_6] moved to looked_at_cards buffer

**Step 4: User selects card from looked-at set**
- Engine presents choice: Select 1 card to add to hand
- User selects: member_card_5
- Expected: member_card_5 moved to hand, remaining cards [member_card_4, member_card_6] moved to waitroom

**Step 5: Verify final state**
- Expected: 絢瀬 絵里 on stage, member_card_2 in waitroom (cost), member_card_5 in hand (selected), member_card_4 and member_card_6 in waitroom (unselected)

## 5. User Choices

**Choice 1 (optional cost):**
- Choice type: SelectCard (optional)
- Available options: All cards in hand except the debuting card
- Selection: member_card_2
- Expected result: Card discarded to waitroom

**Choice 2 (select from looked-at):**
- Choice type: SelectCard
- Available options: member_card_4, member_card_5, member_card_6
- Selection: member_card_5
- Expected result: Card added to hand, others to waitroom

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_3, member_card_5]
- `main_deck`: [energy cards... (top 3 removed)]
- `stage`: [-1, 絢瀬 絵里, -1, -1, -1]
- `waitroom`: [member_card_2, member_card_4, member_card_6]
- `energy_zone`: [energy cards...]

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Debut ability triggers when member is played to stage
- Optional cost choice is presented
- Selected cost card is moved to waitroom
- Look action removes top 3 cards from deck
- User choice is presented for selecting from looked-at cards
- Selected card is moved to hand
- Unselected cards are moved to waitroom
- Debut card is on stage
