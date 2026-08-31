# Rust→C Test Port — Remaining Work

Generated from audit of `engine_c/tests/test_ported_generated.c` (2650 fns; 1382 failing
checks across 966 fns). Failures are dominated by porting gaps, not engine crashes.

## Backlog

- [x] **1. Map `energy_deck.cards.push` to a C shim** (S) — DONE
  - Root cause was multi-line method chains, not a missing mapping: `energy_deck.cards.push` already mapped to `test_add_to_deck_pl` for single-line form.
  - Added `join_method_continuations()` so chained calls (`.player1`/`.cards`/`.push` on separate lines) rejoin into one line → existing rules fire. (Must concatenate with no separator, not a space.)
  - Guarded push rules with `_map_game_id_safe()` so unresolved `game.id(...)` degrades to `// TODO` instead of breaking the C compile.

- [x] **2. Add `main_deck` replace / `insert(0,…)` shims** (M) — DONE
  - `test_insert_deck_top(pl, card)` shim added to `test_game.c`/`test_game.h`.
  - Transpiler rules for `main_deck.cards = vec![a,b,c].into()` (clear+push in order) and `vec![x; N].into()` (repeat-constructor, unrolled) and `main_deck.cards.insert(0, x)` (prepend).
  - Also fixed type-annotated `let X: TYPE = …` so those bindings declare instead of TODO.

- [ ] **3. Investigate `-1` card-not-found** (M)
  - 54 failures show `test_id` returning -1 (`rb_find_card_by_no` miss) for some card_no that exists in Rust.
  - Find which card_nos fail and why (DB gen / encoding mismatch).

- [ ] **4. Investigate live→discard transition gap** (M)
  - `live_cards_stuck_in_live_zone_instead_of_discard` (got 1 expected 3 / got 0 expected 3).
  - Live cards not moving to waitroom/discard at live-end in C engine.

- [ ] **5. Support simple `if let Some(x) = …` destructuring** (L)
  - `match` / `if let` / `while let` cause whole test fns to be skipped (`gen_tests.py:1548`).
  - At minimum handle `if let Some(x) = expr { … }` single-binding form so more tests port.

- [ ] **6. Wire zone-content / mods `assert_eq!` to existing getters** (L)
  - 1712 `assert_eq` + 1020 `assert` TODOs compare zone contents / `mods.*_modifiers.get(&id)`.
  - `test_zone_has_id` / `test_get_*_modifier` already exist but aren't used by the assert resolver.
