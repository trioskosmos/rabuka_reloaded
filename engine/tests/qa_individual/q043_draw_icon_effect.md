# Test Plan: Draw Icon Effect

## 1. Test Objective
Test the draw icon effect on cards. This tests:
- Draw icon triggering when card is drawn
- Draw icon effect execution
- Draw icon vs normal card draw distinction
- Multiple draw icons on same card

## 2. Card Selection
- **Card:** A member card with draw icon effect
- **Card Name:** [Select card with draw icon, e.g., "When drawn, draw 1 card"]
- **Why this card:** Tests draw icon triggering and effect execution

## 3. Initial Game State

**Player 1:**
- `hand`: [member_card_1]
- `main_deck`: [draw_icon_card, member_card_2, member_card_3, energy cards...]
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
- `phase`: Draw
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Draw card from deck**
- Engine function called: `TurnEngine::execute_draw_phase`
- Expected: draw_icon_card drawn from deck to hand
- Expected: draw_icon_card's draw icon effect triggers

**Step 2: Draw icon effect executes**
- Engine function called: `AbilityResolver::execute_draw_icon_effect`
- Expected: Effect executes (e.g., "draw 1 card")
- Expected: Additional card drawn from deck to hand

**Step 3: Verify final hand size**
- Expected: Hand contains 3 cards (initial 1 + drawn card + draw icon effect card)
- Expected: Deck size decreased by 2

**Step 4: Draw another card (no draw icon)**
- Engine function called: `TurnEngine::execute_draw_phase` (or manual draw)
- Expected: member_card_2 drawn to hand
- Expected: No additional effect (no draw icon)

**Step 5: Verify hand size after normal draw**
- Expected: Hand contains 4 cards
- Expected: Deck size decreased by 1

## 5. User Choices

None - draw icon effects execute automatically when card is drawn.

## 6. Expected Final State

**Player 1:**
- `hand`: [member_card_1, draw_icon_card, member_card_3, member_card_2]
- `main_deck`: [energy cards... (top 3 removed)]
- `stage`: [-1, -1, -1, -1, -1]
- Deck size: initial - 3

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Draw icon triggers when card is drawn from deck
- Draw icon effect executes correctly
- Normal card draws (without draw icon) don't trigger effects
- Hand size increases correctly with draw icon effects
- Deck size decreases correctly
- Draw icon effects can draw additional cards
