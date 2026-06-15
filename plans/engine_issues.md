# Engine "Stupid Shit" — Design Issues

## 1. Two zone matches in `handle_select_card` (choice.rs)
Line 364 has a cost handler match. Line 521 has an effect handler match. Both check `match zone { "discard" => ... }`. The cost handler at line 364 is gated by `if gs.entry_cost()...`, but there's an `allow_skip && !indices.is_empty()` block that ALSO runs for effect-based selections. This block returns `Ok(())` early, **skipping the effect handler entirely**. This is how the position choice got eaten: the code entered the cost path's discard arm, called `execute_selected_cards_from_zone`, but then `return Ok(())` prevented the position choice from being picked up.

**Fix:** Merge the two matches. Don't have a separate `allow_skip` early-return block. Let ALL selections flow through the same zone match, then call `finalize_choice`.

## 2. `finalize_choice` is never called for SelectCard choices
The match arm at `provide_choice_result:186`:
```rust
) => self.handle_select_card(...),
```
This returns the Result from `handle_select_card` directly as the function's return value. `finalize_choice` (called at the end of `handle_select_card` line 818) only runs if the zone match doesn't return early. The `allow_skip` block at line 364 has `return Ok(())` which skips `finalize_choice`.

**Fix:** Remove early returns from zone matches. Use `?` instead of `return` and let all code paths reach `finalize_choice`.

## 3. `clear_choice_state` wipes `self.pending_choice`
`clear_choice_state` at line 1203 does `self.pending_choice = None`. Used indiscriminately. If a position choice was just created by `place_card_with_position_choice`, this wipes it.

**Fix:** Check `self.pending_choice.is_some()` before clearing.

## 4. Resolver state is lost between calls
`actions.rs:322` creates a new `AbilityResolver` for every choice. Fields like `pending_stage_cards` are set during one call but gone on the next. Workaround: manually persist to `AbilityQueueEntry` fields. But there's a new field needed every time someone wants cross-call state.

**Fix:** Don't recreate the resolver. Keep one alive for the duration of the ability resolution. Or: automatically snapshot/resolve all resolver fields to/from the queue entry.

## 5. `store_pending_choice` vs `resolver.get_pending_choice()`
`store_pending_choice` writes to `gs.pending_choice`. But `actions.rs:345` checks `resolver.get_pending_choice()` which reads `self.pending_choice` on the **new** resolver. If `store_pending_choice` was called on the OLD resolver, the new resolver doesn't have it.

**Fix:** After `provide_choice_result`, ALSO check `gs.pending_choice` (from `store_pending_choice`). Or make the resolver's `pending_choice` auto-sync.

## 6. Card copy ID pool (tests)
`TestGame.id()` returns a different i16 copy from a pool each call. The same `card_no` like `PL!-bp3-025-L` returns copy #1 first call, copy #2 second call. Assertions that check `hand.cards.contains(&game.id("X"))` fail because the copy in hand is a DIFFERENT i16 from the one returned by the fresh `game.id()` call.

**Fix:** Store the returned ID in a variable and reuse it. The test helpers already do this in some tests but not consistently.

## 7. Phase system: Active/Energy/Draw return empty actions
`generate_possible_actions` returns `Vec::new()` for Active/Energy/Draw phases. The frontend shows zero actions and can't advance. `settle_single_player_state` auto-advances these phases, but only runs after an action, not during state polling.

**Fix:** Either auto-advance in the state poll, or return a "Wait" action that the frontend can click.

## 8. `eprintln!` vs `AbDebug` for debug output
The test output is cluttered with `[AB]` lines from `eprintln!`. These are mixed with actual test assertions. The `AbDebug::p()` function writes to BOTH `eprintln!` AND the `ABILITY_LOG_BUFFER`. The buffer is drained by `flush_to_rule_log` in `display.rs`. The `eprintln!` part is just for stderr debugging during development, but it's ALWAYS on (const ABILITY_DEBUG = true).

**Fix:** Make ABILITY_DEBUG a cfg flag or runtime toggle. Or remove the eprintln! and only use the buffer.

## 9. `gs.rule_log` grows unbounded
Every rule_log entry is pushed to a `Vec<String>` that is NEVER cleared. Over the course of a game this grows linearly. The frontend slices to the last 200 entries, but the backend keeps all of them.

**Fix:** Add a cap (e.g. 500 entries) and discard oldest.

## 10. Magic strings for zones, destinations, card types
The code uses string literals everywhere: `"hand"`, `"discard"`, `"stage"`, `"member_card"`, `"live_card"`, `"empty_area"`, `"same_area"`. No enums for zone names or destination types. A typo like `"disacrd"` silently creates a new zone that doesn't match any handler.

**Fix:** Create enums for zones, destinations, card types.
