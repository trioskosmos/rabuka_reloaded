# Nico Test — 自分と相手はそれぞれ  empty_area + wait state

## The Ability

**Card:** 矢澤にこ (PL!-pb1-018-R)

**Full text:**
```
{{toujyou.png|登場}}自分と相手はそれぞれ、自身の控え室からコスト2以下の
メンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる。
（この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。）
```

**Translation:**
When this card appears (debut), both you and the opponent each take 1 member card
with cost ≤2 from your own discard, and deploy it in **wait state** to an **empty area**
on your stage. The parenthetical note restricts re-entering the area this turn.

---

## Parser Output

### Current `abilities.json` entry

```json
{
  "source": "discard",
  "destination": "empty_area",
  "cost_limit": 2,
  "cost_limit_operator": "<=",
  "state_change": "wait",
  "count": 1,
  "card_type": "member_card",
  "target": "both",
  "multiple_targets": true,
  "action": "move_cards"
}
```

### Key fields

| Field | Value | Meaning |
|-------|-------|---------|
| `action` | `move_cards` | Card movement effect |
| `source` | `discard` | Take from discard (控え室) |
| `destination` | `empty_area` | Place in an empty stage area (メンバーのいないエリア) |
| `state_change` | `wait` | Deploy in wait state (ウェイト状態) |
| `target` | `both` | Affects both players (自分と相手) |
| `multiple_targets` | `true` | Each player acts independently (それぞれ) |
| `cost_limit` | `2` | Only cards with cost ≤2 |
| `card_type` | `member_card` | Only member cards |

---

## Engine Execution Flow

### 1. `execute_move_cards()` in `move_cards.rs`

1. Resolves target player(s) — `"both"` triggers processing for both P1 and P2
2. For each player:
   a. Selects cards from `discard` matching filter (cost ≤2, member_card)
   b. If multiple candidates → `SelectCard` choice prompt
   c. If exactly one → auto-selects
   d. If none → skips
3. **Places cards in destination:**
   - If `destination == "empty_area"` AND exactly 1 card selected AND multiple empty slots:
     → Creates `SelectPosition` choice for area selection
     → Returns early, waiting for choice resolution
   - Otherwise → auto-places via `place_card_in_zone()` (auto to Center→Left→Right)
4. Clears old modifiers via `clear_modifiers_for_card()`
5. Records card movement via `record_card_movement()`
6. Applies `state_change: "wait"` via `add_orientation_modifier(card_id, "wait")`

### 2. Position Choice Flow

```
execute_move_cards()
  → detects multiple empty slots
  → sets pending_choice = SelectPosition
  → sets execution_context = MoveCardsPosition { card_id, state_change }
  → returns Ok(())

Test calls select_option(1)
  → resume_with_choice(card_id=Some(1))
  → build_choice_result() maps card_id=1 → "center"
  → provide_choice_result(PositionSelected { position: "center" })
  → handle_select_position("center", MoveCardsPosition { ... })
  → places card in position 1 (center)
  → applies state_change, records movement
  → clears pending_choice
```

---

## Fixes Applied

### Fix 1: Parser — `empty_area` destination

**File:** `parser.py:extract_destination()`

The literal check `'メンバーのいないエリアに登場させる' in text` failed because
the actual text has `"メンバーのいないエリアにウェイト状態で登場させる"`.

**Fix:** Added alternate pattern:
```python
if 'メンバーのいないエリアに登場させる' in text or \
   'メンバーのいないエリアにウェイト状態で登場させる' in text:
    return 'empty_area'
```

### Fix 2: Engine — Wait state ordering

**File:** `move_cards.rs`

The original code applied `state_change` THEN cleared modifiers, which immediately
removed the wait orientation:

```rust
// OLD ORDER (broken):
add_orientation_modifier(card_id, "wait");  // wait set
clear_modifiers_for_card(card_id);           // wait cleared!
record_card_movement(card_id);

// NEW ORDER (fixed):
clear_modifiers_for_card(card_id);           // clear old first
record_card_movement(card_id);
add_orientation_modifier(card_id, "wait");   // wait persists
```

### Fix 3: Engine — Area position choice

**Files:** `types.rs`, `move_cards.rs`, `choice.rs`

Added `MoveCardsPosition` variant to `ExecutionContext` to support player choice
of which empty area to deploy to when multiple are available.

When `destination == "empty_area"` and multiple empty slots exist:
```rust
self.pending_choice = Some(Choice::SelectPosition { ... });
self.execution_context = ExecutionContext::MoveCardsPosition {
    card_id: taken[0],
    state_change: _effect.state_change.clone()
};
return Ok(());  // wait for choice resolution
```

### Fix 4: Tests — Area choice + wait state verification

**File:** `gameplay_test.rs`

Tests updated to:
1. Check `pending_choice_type()` for `"SelectPosition"`
2. Use `select_option(1)` to choose center
3. Use `select_option(2)` to choose right
4. Verify wait state: `assert_eq!(game.state.get_orientation_modifier(card), Some(&"wait".to_string()))`

---

## Fixed Issue: Player Targeting

### Problem (Fixed)

`handle_select_position` was always using `player1`:
```rust
ExecutionContext::MoveCardsPosition { card_id, state_change } => {
    let player = &mut self.game_state.player1;  // WRONG for P2!
```

When `target: "both"`, P2's card should be placed on P2's stage, not P1's.

### Applied Fix

Determined the correct player from the ability queue context. The execution flow
for `target: "both"` creates separate ability queue entries for P1 and P2.
The current entry's `player_id` field indicates which player.

```rust
let is_p2 = self.game_state.ability_queue.current_entry()
    .map(|e| e.player_id == "player2" || e.player_id == "p2")
    .unwrap_or(false);
let player = if is_p2 {
    &mut self.game_state.player2
} else {
    &mut self.game_state.player1
};
```

**File:** `choice.rs:handle_select_position()` (lines 721-728)

---

## Additional Parser Fixes Applied (2026-05-12)

### Fix 1: Group Names Extraction

**Problem:** `group_names` field was not being extracted for 『Liella!』 patterns in select actions.

**Solution:** Added `group_names` extraction to both `_build_reveal_add_discard()` and `_enrich_from_text()` functions:

```python
# In both functions:
gns = extract_group_names(text)
if gns: result['group_names'] = gns
```

**Result:** Card 1 now correctly extracts `group_names: ['Liella!']`

### Fix 2: Cost Limit and Operator Extraction

**Problem:** `cost_limit` and `cost_limit_operator` were not being extracted for "コスト11以上" patterns.

**Solution:** Added cost limit and operator extraction to both `_build_reveal_add_discard()` and `_enrich_from_text()` functions:

```python
# In both functions:
cl = extract_cost_limit(text)
if cl: result['cost_limit'] = cl
op = extract_operator(text)
if op: result['cost_limit_operator'] = op
```

**Result:** Card 2 now correctly extracts `cost_limit: 11` and `cost_limit_operator: >=`

### Fix 3: Ability Gain Parsing Test Correction

**Problem:** Test was looking for `ability_gain` at wrong level in parsed structure.

**Solution:** Updated test to correctly look in the `actions` array for `gain_ability` entries:

```python
actions = e3.get('actions', [])
gain_action = next((a for a in actions if a.get('action') == 'gain_ability'), None)
if gain_action:
    print('ability_gain:', gain_action.get('ability_gain','')[:40])
    print('duration:', gain_action.get('duration'))
```

**Result:** Card 3 now correctly shows `ability_gain: ライブの合計スコアを+1する。` and `duration: live_end`

### Verification

All fixes have been tested and are working correctly:
- ✅ Card 1: `group_names: ['Liella!']` 
- ✅ Card 2: `cost_limit: 11`, `cost_limit_operator: >=`
- ✅ Card 3: `ability_gain` and `gained_effect` parsing working
- ✅ Nico card: `empty_area` destination + `wait` state working correctly

---

## Test Structure

### `gameplay_test.rs` — Nico tests (lines 558–830)

| Test | Line | Description |
|------|------|-------------|
| `nico_q168_both_appear_from_discard` | 558 | Both players appear a member from discard |
| `nico_q168_no_suitable_card_skips` | 604 | No valid card → skip gracefully |
| `nico_q170_turn_player_appears_first` | 641 | Turn player resolves first |
| `nico_q181_area_freed_after_card_leaves` | 670 | Freed area after card removed |
| `nico_requires_empty_area` | 721 | No empty area → effect does nothing |
| `nico_cost_filter_only_shows_eligible` | 761 | Only cost ≤2 cards shown as choices |
| `nico_q169_no_baton_touch_from_appeared_area` | 804 | Restriction is natural |



---

## Key Code Locations

| File | Lines | What |
|------|-------|------|
| `parser.py:extract_destination()` | 303-304 | `empty_area` pattern matching |
| `move_cards.rs:execute_move_cards()` | 460-473 | Area choice creation |
| `move_cards.rs` | 489-499 | Modifier ordering fix |
| `choice.rs:handle_select_position()` | 720-741 | Area choice handling |
| `choice.rs:resume_execution()` | 8-11 | `MoveCardsPosition` no-op handler |
| `types.rs:ExecutionContext` | 57 | `MoveCardsPosition` variant |
| `types.rs:build_choice_result()` | 136-138 | Position mapping (0=left,1=center,2=right) |
| `gameplay_test.rs` | 558-830 | All nico integration tests |
