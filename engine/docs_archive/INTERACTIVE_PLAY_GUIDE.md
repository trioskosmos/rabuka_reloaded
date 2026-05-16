# Interactive Gameplay & Bug Hunting Guide

This guide describes how to interact with the `rabuka_engine` web server to manually play the game, identify discrepancies between the game rules and the engine implementation, and verify ability behavior.

## 0. Server Setup & Startup

```bash
# Kill any stale engine processes
taskkill //F //IM rabuka_engine.exe 2>nul

# Start the server
cd engine && cargo run --release --bin rabuka_engine web-server
```

The server starts on `http://127.0.0.1:8080`.

### Available API Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/init` | Start a new game |
| GET | `/api/game-state` | Get current game state, phase, players, legal actions |
| GET | `/api/actions` | Get legal actions only |
| POST | `/api/execute-action` | Execute an action (play card, use ability, pass, etc.) |
| POST | `/api/exec` | **Cheat** — add cards/energy. Uses format: `player_idx=0;amount=5;draw_energy` or `player_idx=0;card_no="PL!S-bp2-009-P";add_card` |
| POST | `/api/undo` | Undo the last action |
| POST | `/api/redo` | Redo a previously undone action |
| GET | `/api/get_card_registry` | Get all cards with their IDs, names, card_no, abilities |

---

## 1. The Gameplay Loop (Step-by-Step)

Play the game one move at a time, like a human. After every action, verify the result.

### 1.1 Initialize the Game

```
POST /api/init
```

Response includes the game state with `phase: "RockPaperScissors"`. Both players have 3 energy cards.

### 1.2 Play Through Setup Phases

The game must go through these phases before reaching Main:

#### Phase A: RockPaperScissors
The actions returned are `rock_choice`, `paper_choice`, `scissors_choice`. Send ONE choice for each player:

```json
POST /api/execute-action  {"action_index": 0, "action_type": "rock_choice"}
```

Wait — RPS needs BOTH players to pick. In single-player mode, you send P1's choice first. The engine auto-plays P2's RPS choice. After submitting P1's choice, check the game state and send P2's choice:

```json
POST /api/execute-action  {"action_index": 0, "action_type": "paper_choice"}
```

#### Phase B: Choose First Attacker
The RPS winner chooses who goes first:

```json
POST /api/execute-action  {"action_index": 0, "action_type": "choose_first_attacker"}
```

This draws both players to 6 cards.

#### Phase C: Mulligan (P1 then P2)
Each player can choose to mulligan or skip:

```json
POST /api/execute-action  {"action_index": 0, "action_type": "skip_mulligan"}
```

P1 mulligan first, then P2.

#### Phase D: Main Phase
Now the game is in `Main` phase. Legal actions include `pass`, `play_member_to_stage`, and `use_ability`.

### 1.3 Playing Cards in Main Phase

**IMPORTANT**: Always get card IDs from the **current game state's legal actions**. Card IDs change with every `POST /api/init` because the deck is shuffled.

```python
# CORRECT WAY — get card_id from legal_actions
state = GET /api/game-state
for action in state.legal_actions:
    if action.action_type == "play_member_to_stage":
        card_id = action.parameters.card_id
        # Pick the first available area
        for area in action.parameters.available_areas:
            if area.available:
                POST /api/execute-action {
                    "action_index": 0,
                    "action_type": "play_member_to_stage",
                    "card_id": card_id,
                    "stage_area": area.area  # "left", "center", or "right"
                }
```

### 1.4 Using Abilities in Main Phase

When a card with a `起動` (activation) or `メイン` (main) trigger is on stage, `use_ability` actions appear in legal actions.

```python
state = GET /api/game-state
for action in state.legal_actions:
    if action.action_type == "use_ability":
        card_id = action.parameters.card_id
        stage_area = action.parameters.stage_area
        POST /api/execute-action {
            "action_index": 0,
            "action_type": "use_ability",
            "card_id": card_id,
            "stage_area": stage_area
        }
```

### 1.5 Ending Main Phase

```json
POST /api/execute-action  {"action_index": 0, "action_type": "pass"}
```

The game auto-advances through `Active` → `Energy` → `Draw` phases and back to `Main` for the next turn, or to `LiveCardSet` phases.

### 1.6 Live Card Set Phase

During `LiveCardSetP1Turn` / `LiveCardSetP2Turn`, you can place live cards from hand to the live zone:

```json
POST /api/execute-action {
    "action_index": 0,
    "action_type": "set_live_card",
    "card_id": LIVE_CARD_ID
}
```

Pass to finish:

```json
POST /api/execute-action  {"action_index": 0, "action_type": "pass"}
```

---

## 2. Using the Cheat Endpoint

The `/api/exec` cheat endpoint can add cards to hand or draw extra energy for testing.

**Format**: `key1=value1;key2=value2;command`

```bash
# Draw 10 energy cards for player 0:
POST /api/exec  {"code": "player_idx=0;amount=10;draw_energy"}

# Add a specific card to hand:
POST /api/exec  {"code": "player_idx=0;card_no=\"PL!S-bp2-009-P\";add_card"}
```

After exec, query `GET /api/game-state` to verify the changes took effect.

---

## 3. How to Find Bugs

This is the most important section. Every action in the game loop is an opportunity to find a bug.

### 3.1 Verifying `play_member_to_stage`

After playing a member to the stage, verify:

1. **The card left the hand**: Check `player1.hand.cards` — the card ID should no longer be there.
2. **The card appeared on stage**: Check `player1.stage.center/left_side/right_side` — the card should be there with correct orientation and attributes.
3. **Energy was deducted**: Check `player1.energy.cards.length` decreased by the card's cost.
4. **Debut/auto abilities triggered**: Check `pending_choice` — the engine might be waiting for a choice from a debut ability.
5. **If baton touch**: The replaced card should be in the waitroom, and the new card should be on stage with the old card in the `_under` position.

**Check against rules.txt**: Rule 7.7.2.2 — Main phase member play rules.

### 3.2 Verifying Ability Activation (CRITICAL)

When a `use_ability` action is available and you execute it, examine **every** aspect of the ability:

1. **Was the ability generated?** Check that `use_ability` appears in legal actions for cards that SHOULD have activatable abilities (trigger = `起動` or `メイン`). Cross-reference with `abilities.json`.

2. **Was the cost paid correctly?**
   - If the ability has an energy cost, verify energy was deducted.
   - If it requires discarding cards from hand, verify the hand shrank.
   - If it requires sending cards from deck to waitroom, verify the deck shrank.

3. **Is there a pending choice?** If the ability targets cards or requires a decision, `pending_choice` will be non-null in the game state. Examine the choice JSON carefully:
   - `zone` — is the right zone being targeted?
   - `card_type` — is the correct card type specified?
   - `count` / `choose_count` — is the correct number requested?
   - `allow_skip` — is skipping allowed when it should be?
   - `description` — does the prompt match the ability text?

4. **Did the effect execute?** After providing a choice response:
   - Check cards moved between zones (hand, stage, waitroom, deck, energy, live_zone, etc.)
   - Check stat modifications (blade, hearts, etc.)
   - Check state changes (orientation, etc.)

5. **Did auto-abilities trigger afterwards?** Many abilities trigger other abilities. Check if the engine correctly queues and processes them.

**Checking ability text**: Look up the card in `cards/abilities.json` under `unique_abilities[].cards`. Match by card_no to get the full ability text, triggers, costs, and effects.

**Checking conditions**: Use `GET /api/debug/conditions` to evaluate all conditions on all cards in play. This shows which conditions evaluate to `true` or `false`.

### 3.3 Example: Debugging an Ability Activation

```python
# 1. Before using ability, record state
before = GET /api/game-state
p1_hand_before = [...], p1_energy_before = N, p1_deck_before = M

# 2. Look up the card's ability in abilities.json
card_no = "..."
# Find ability text, triggers, cost, effect

# 3. Execute the ability
POST /api/execute-action {... use_ability ...}

# 4. Check for pending choices
after = GET /api/game-state
if after.pending_choice:
    print("Engine requires a choice:", after.pending_choice)
    # Make sure the choice matches what the ability should do
    # Cross-reference with abilities.json effect fields

# 5. If there's a choice, respond to it
POST /api/execute-action {
    "action_index": 0,
    "action_type": "decision",  # or "select_card", "select_position", etc.
    "card_id": CHOSEN_CARD_ID
}

# 6. Verify final state
final = GET /api/game-state
# Check: hand changed by correct amount?
# Check: energy changed by correct amount?
# Check: deck changed by correct amount?
# Check: waitroom has expected cards?
# Check: conditions on debug endpoint are correct?
```

### 3.4 Using Undo/Redo to Recheck

When you suspect a bug, use undo/redo to replay the same situation:

```python
# Step 1: Record state before action
before = GET /api/game-state

# Step 2: Execute action
POST /api/execute-action {...}

# Step 3: Check state after
after = GET /api/game-state

# Step 4: If something looks wrong, undo and re-examine
POST /api/undo
restored = GET /api/game-state

# Verify restored == before (should be identical)
# Then redo and verify after is reached again
POST /api/redo
```

**Undo/redo saves the FULL game state snapshot**, so you can go back-and-forth to verify deterministic behavior.

### 3.5 Checking Phase Transitions

The engine auto-advances through `Active` → `Energy` → `Draw` phases. This logic is in `settle_single_player_state` in `web_server.rs`. Watch for:

- **Skipped phases**: Does the engine skip a phase it shouldn't?
- **Stuck in wrong phase**: Does `pass` in Main correctly advance? Does LiveCardSet correctly transition?
- **Turn number**: Does the turn increment correctly at the right time?

### 3.6 Comparing Against qa_data.json

The file `cards/qa_data.json` contains official rule clarifications (Q&As). Each entry has:

```json
{
  "id": "Q237",
  "question": "...",
  "answer": "はい、可能です。" or "いいえ、できません。",
  "related_cards": [{"card_no": "...", "name": "..."}]
}
```

To test a QA entry:
1. Set up the game state to match the QA scenario (use `/api/exec` to add specific cards).
2. Execute the sequence of actions described in the question.
3. Check if the engine's behavior matches the expected `answer`.
4. Report YES/NO mismatches as bugs.

### 3.7 Error Message Analysis

When an action returns HTTP 400 with a JSON error body, always check:

- Is the error message correct for the situation?
- Is the error preventing a LEGAL action from executing?
- Could the same situation in a real card game produce a different result?

---

## 4. Common API Patterns

### Getting card_no → ID mapping

```python
reg = GET /api/get_card_registry
# reg.cards is an array of {id, name, card_no, card_type, ...}
id_map = {c.card_no: c.id for c in reg.cards}
```

### Reading hand cards

```python
state = GET /api/game-state
# Hand cards have: id, card_no, name, type, base_heart, blade, cost
p1_hand = state.player1.hand.cards
for card in p1_hand:
    print(card.id, card.card_no, card.name, card.cost)
```

### Reading stage cards

```python
stage = state.player1.stage
left = stage.left_side       # None or {id, card_no, name, ...}
center = stage.center
right = stage.right_side
left_under = stage.left_under  # Cards under after baton touch
```

### Reading energy zone

```python
energy = state.player1.energy.cards  # Array of card objects
active_count = len(energy)  # Active energy count
```

---

## 5. Minimum Viable Script Structure

```python
import requests, json
BASE = "http://127.0.0.1:8080/api"

def gs():
    return requests.get(f"{BASE}/game-state").json()

def exec_a(**kw):
    r = requests.post(f"{BASE}/execute-action", json={"action_index": 0, **kw})
    return r.json() if r.status_code == 200 else {"error": r.text}

# Init
requests.post(f"{BASE}/init")

# Setup phases
exec_a(action_type="rock_choice")
exec_a(action_type="paper_choice")  # or rock/scissors for P2
exec_a(action_type="choose_first_attacker")
exec_a(action_type="skip_mulligan")
exec_a(action_type="skip_mulligan")

# Main phase: play cards, use abilities, pass
# ... game loop ...

# When finished, pass to advance turn
exec_a(action_type="pass")
```

---

## 6. Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| HTTP 400 on play_member_to_stage | Stale card_id from previous game | Re-read state and use current card_id |
| `"Card not found in hand"` | Card not in player's hand | Check hand IDs before sending |
| Exec add_card doesn't work | Parser can't handle the format | Use `player_idx=0;card_no="CARD_NO";add_card` |
| Port 8080 already in use | Stale server process | `taskkill //F //IM rabuka_engine.exe` |
| Game stuck in a phase | Auto-advance stuck | Check `pending_choice` — engine may be waiting for input |
| Ability not in legal actions | Card may not have activatable ability | Check `abilities.json` for card's triggers |
