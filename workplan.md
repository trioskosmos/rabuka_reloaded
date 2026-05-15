# Rabuka Engine — Changes & Remaining Work

## Changes Made (388/388 tests passing)

### Engine: Condition Evaluation Fixes

| File | What | Why |
|------|------|-----|
| `engine/src/ability/condition.rs` | `evaluate_card_count_condition`: resolve `target="self"` using `activating_card_id` by scanning all player zones (stage, hand, live_card_zone, energy_zone) instead of `ability_queue.current_entry()` | Queue entry is cleaned up before nested sequential-action conditions run; was returning wrong player |
| `engine/src/ability/condition.rs` | `evaluate_location_condition` → `calculate_location_value`: same `activating_card_id`-based player resolution for `target="self"` | Same root cause: queue entry gone → wrong player checked for appearance‑trigger conditions |
| `engine/src/ability/condition.rs` | `check_location_distinct`: same `activating_card_id`-based player resolution | Consistency with above |
| `engine/src/ability/condition.rs` | Removed `game_state.recently_moved_cards` fallback in the no-location (`""`) branch of `evaluate_card_count_condition` | An earlier sub-action's draw+discard set `recently_moved_cards`, causing later sub-actions to count 1 moved card instead of 3 stage members |

### Engine: Baton Touch Duplicate Fix

| File | What | Why |
|------|------|-----|
| `engine/src/core/player.rs` | Guarded unconditional old-card removal with `if !baton_touch_used` | When baton-touch, the old member was pushed to waitroom TWICE (once by the unconditional block, once by the baton-touch-specific block) |
| `engine/src/core/player.rs` | Moved `cannot_baton_touch` protection check BEFORE energy payment | Was after — if check failed, energy had already been spent and new card already on stage, leaving corrupted state |

### Engine: Mulligan Selection Fix

| File | What | Why |
|------|------|-----|
| `engine/src/game/game_setup.rs` | `SelectMulligan` action now passes `card_indices: Some(vec![hand_index])` instead of `card_index` + `stage_area` | The dispatch only forwards `card_indices` and `card_id`, never `card_index`; `stage_area` was a copy-paste artifact |
| `engine/src/turn/phases.rs` | `handle_mulligan_selection` simplified to consume `card_indices[0]` directly | Removed `get_card_index_by_id()` linear search that always found the first duplicate, making the second copy impossible to independently mulligan |

## Remaining Issues

### 1. Instance IDs for Modifier Tracking (stage duplicates)

**Problem**: If 3 copies of the same card are on stage, all share the same numeric ID. A 常時 ability that gives "this card +2 blade" would accumulate as +6 total instead of +2 per copy.

**Current state**: The test helper infrastructure already has a `copy_pool` (`TestGame::new()` pre-creates 5 copies per template). `new_id()` returns distinct copies by popping from the pool. However `id()` peeks (returns the same copy every call), so patterns like `for _ in 0..5 { deck.push(game.id("filler")) }` still push the same ID.

**Candidate fix**: Change `id()` to pop from the pool, add `id_ref()` for stable-reference patterns. This is NOT yet implemented because it would require auditing every test for patterns that call `id()` twice expecting the same ID.

### 2. Card ID Determinism

**Current state**: IDs are assigned from arbitrary HashMap iteration order (non‑deterministic). Reverted `sort_by(card_no)` + removed `card_id_mapping.json` and dead `Card.card_id` field in the revert.

**Needed**: Re-apply deterministic sorting and dead-code cleanup after instance ID fix.

### 3. For Examination (user-reported)

- PL!-pb1-011-R (Eli) BiBi group check returns `actual=0` despite BiBi cards on stage — should be fixed by the `calculate_location_value` / `check_location_distinct` player-resolution fix in `condition.rs`
- `cannot_baton_touch` protection check order — fixed

## Test Strategy

All changes verified by running `cargo test` — 388/388 passing.
