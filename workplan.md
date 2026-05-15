# Rabuka Engine — Changes & Remaining Work

## Changes Made (388/388 tests passing)

### Instance ID System (new!)
| File | What | Why |
|------|------|------|
| `tests/helpers/mod.rs` | `id()` now **pops** from the copy pool (returns unique ID each call). Added `id_ref()` for stable-reference pattern (peeks, returns same ID). | Multiple copies of the same card on stage now get **distinct numeric IDs** → `recalculate_constants` applies per-card modifiers correctly instead of accumulating shared modifiers. Fixes "3 copies → +6 blade instead of +2 each". |
| `tests/helpers/mod.rs` | Retained `new_id()` (same as `id()` now — both pop). | Backward compat for tests that explicitly used `new_id()`. |
| Various test files | Fixed 5 tests that called `game.id("x")` twice expecting the same ID (now store in a variable). | `id()` now returns a different ID each call, so `set_live_card(game.id("x"))` would search for a different copy than `game.state.hand.push(game.id("x"))` stored. |

### LiveStart Trigger Fix (new!)
| File | What | Why |
|------|------|------|
| `triggers.rs` | `trigger_live_start_abilities` now passes the **numeric stage card ID** as `explicit_card_id` to `trigger_auto_ability`. | `search_player_zones_for_card` searches **hand first**, then stage. It found the hand copy (same `card_no`) before the stage copy, setting `activating_card_id` to the hand/discarded copy instead of the stage copy. Blade/score modifiers were applied to the wrong card. |

### Card ID Determinism (re-applied)
| File | What | Why |
|------|------|------|
| `card.rs` | `load_or_create` now sorts cards by `card_no` before assigning sequential IDs. | Eliminates non-deterministic ID assignment from HashMap iteration order — same cards always get same numeric IDs across runs. |
| `card.rs` | Removed dead `Card.card_id` field (was `#[serde(skip)]`, always `0`, never read). | Dead code cleanup. |
| `card.rs` | Removed `save_mapping()` and mapping file loading (`card_id_mapping.json`). | Unused — IDs are now deterministic via `card_no` sort. |
| — | Deleted `engine/card_id_mapping.json` and `root_files/card_id_mapping.json`. | Stale files with incompatible mappings. |

### Earlier Fixes (still in place)
| File | What |
|------|------|
| `condition.rs` | Player resolution via `activating_card_id` for `evaluate_card_count_condition`, `calculate_location_value`, `check_location_distinct`. Removed `recently_moved_cards` fallback. |
| `player.rs` | Baton touch: guarded unconditional removal with `if !baton_touch_used` (eliminates duplicate waitroom push). Moved `cannot_baton_touch` check before energy payment. |
| `game_setup.rs` + `phases.rs` | Mulligan: uses `card_indices` directly instead of `get_card_index_by_id` (fixes duplicate-card selection). |
