# Overengineering Analysis

## 1. Event Bus that does literally nothing
`events.rs:38` — `EventBus::flush` takes `&mut GameState` but ignores it, just clears the queue. No listeners, no subscribers, no handlers. 21 `GameEvent` variants, zero actual event-driven behavior.

## 2. `#![allow(dead_code)]` as a lifestyle
Half the engine source files start with `#![allow(dead_code)]`. `triggers.rs` has 17 constants, at least 7 unused. `RuleConfig` has 15 bool fields all defaulting to `true`, none gate any behavior.

## 3. Legacy phase aliases everywhere
`Phase::Mulligan` (alias for `MulliganP1Turn`) and `Phase::LiveCardSet` get special-case branches throughout `turn.rs` and `game_state.rs`. Also a parallel `current_live_card_set_player: u8` flag system.

## 4. Eight nearly-identical modifier HashMaps with 24 boilerplate methods
`blade_modifiers`, `blade_type_modifiers`, `heart_modifiers`, `orientation_modifiers`, `cost_modifiers`, `score_modifiers`, `need_heart_modifiers` — each has `add_X`/`remove_X`/`get_X` copy-pasted. `Modifier<T>` would collapse this.

## 5. Macro explosion for a trait that can't work generically
`move_cards.rs` — `CardCollection` trait + `impl_card_collection!` for 6 SmallVec sizes. But `SmallVec<[i16; 60]>` is different from `SmallVec<[i16; 3]>`, so the trait is never used generically.

## 6. Vanity refactoring — ability/ sub-module
`ability_resolver.rs` is 5 lines re-exporting. 9 files in `ability/` but `impl AbilityResolver` blocks are scattered across 4 files, making it harder to find logic.

## 7. Massive single-file god objects
- `turn.rs` = 1678 lines
- `game_state.rs` = 2489 lines
- `effects.rs` = 1452 lines
- `choice.rs` = 535 lines

## 8. Seven player-getting methods with duplicated dispatch logic
`active_player()`, `active_player_mut()`, `first_attacker()`, `first_attacker_mut()`, `second_attacker()`, `second_attacker_mut()`, `non_active_player()` — each with overlapping match logic.

## 9. Mulligan/RPS handler duplication
Three mulligan handlers each duplicate the `if p1_is_first` phase-advancement branching. Two RPS handlers do the same thing minus the field they write to.

## 10. `check_timing` is a dumpster fire
`turn.rs:1054` — does event flush, player refresh, deck refresh, victory check, invalid card check, invalid resolution zone check, permanent loop check, AND auto-ability resolution.

## 11. SmallVec dependency in Cargo.toml but never used by zones
Actual zone types (`MainDeck`, `Hand`, `Waitroom`) all use `Vec<i16>`.

## 12. `modifier_invariant()` is O(n) per GameState::new()
Loops through every key in every modifier HashMap to check cards exist. Called on every `GameState::new()`.

## 13. Collect-then-trigger pattern repeated 4 times
Four trigger methods all collect into intermediate `Vec<(String, String)>` then loop — workaround for borrow checker that shouldn't exist with better architecture.

## Root Cause
Engine grew organically with feature-driven development. New game rules got fields/methods bolted on rather than refactored in. IR module already gutted per ENGINE_OVERHAUL.md, but the rest needs the same treatment.

## Priority Order
1. **Event bus** — remove or make real
2. **Modifier maps** — collapse into generic
3. **Dead code** — purge unused
4. **Legacy phases** — remove aliases and branches
5. **SmallVec** — purge dependency
6. **Player methods** — consolidate
7. **modifier_invariant()** — remove O(n) assertion
