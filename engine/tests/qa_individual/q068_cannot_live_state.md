# Q068: Cannot Live State

## Test Objective
Test the "cannot live" state: player can place live cards face-down in live card set phase, but in performance phase, even if live cards are revealed, all cards (including live cards) go to discard zone. Result: no live cards in live card zone, so live is not performed (no live start abilities, no cheer).

## Q&A Reference
**Question:** What is the "cannot live" state?
**Answer:** Player can place live cards face-down in live card set phase, but in performance phase, even if live cards are revealed, all cards (including live cards) go to discard zone. Result: no live cards in live card zone, so live is not performed (no live start abilities, no cheer).

## Card Selection
Any live card.

**Primary Card:** Any live card

## Initial Game State

**Player 1:**
- `hand`: [live_id]
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: []
- `energy_zone`: [5 energy cards]
- `success_live_card_zone`: []
- `live_card_zone`: []
- `cannot_live`: true

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

**Step 1: Set player to "cannot live" state**
- Engine function called: Set `game_state.player1.cannot_live = true`
- Expected intermediate state changes:
  - cannot_live flag set to true
- Expected output: success

**Step 2: Place live card face-down in live card set phase**
- Engine function called: Live card placement (face-down)
- Expected intermediate state changes:
  - live_id moves from hand to live_card_zone (face-down)
- Expected output: success

**Step 3: Reveal live card in performance phase**
- Engine function called: Live card reveal
- Expected intermediate state changes:
  - live_id moves from live_card_zone to waitroom (due to cannot_live state)
- Expected output: success

## User Choices
None - deterministic

## Expected Final State

**Player 1:**
- `hand`: []
- `main_deck`: [remaining deck cards]
- `stage`: [-1, -1, -1, -1, -1]
- `waitroom`: [live_id] (live card discarded)
- `energy_zone`: [5 energy cards]
- `success_live_card_zone`: []
- `live_card_zone`: [] (no live cards)
- `cannot_live`: true

**Player 2:**
- Unchanged from initial state

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Performance
- `baton_touch_count`: 0

## Verification Assertions
1. cannot_live state is set
2. Live card placed face-down succeeds
3. Live card is discarded when revealed (due to cannot_live state)
4. No live cards remain in live_card_zone
5. No live start abilities trigger
6. No cheer occurs
7. No compilation errors
8. No runtime panics
