# Rust→C Test Port — Remaining Work

> 2026-09-14 PROCESS (the loop — follow it, in order, repeat):
> 1. Transpile correctly (`tools/gen_tests.py` was deleted 2026-09-14 as
>    unfixable accretion; replacement is rewritten from the seed list below).
> 2. For failing ported tests: manually rewrite ONE, work out what the
>    script needs to handle that pattern, fix the script — never patch the
>    generated C by hand.
> 3. Failures that survive a correct port are engine gaps: fix `engine_c`.
> 4. Repeat. Scenario-replay (`tests/replay.c` trace/scenario mode +
>    `engine/tests/run_all.rs::scenario_oracle`) stays as the cross-check
>    for parity-critical behaviors, not the migration vehicle.
>
> HAND-PORT RESULTS 2026-09-14 (simple eri + complex suki, kept in
> /tmp/opencode/handtest.c, NOT repo): both pass on the C engine with zero
> engine changes — struct mapping (`p[0].stage[0]`) is trivially 1:1, and
> the engine already handles recalc/orientation, 7-pass live flow, pending
> SelectCard, select_option, score mods. For these behaviors the ONLY gap
> is transpiler coverage. The earlier suki-trace divergence was runner
> state pollution (queue/transients not cleared on load), not an engine
> bug — after clearing, trace is byte-identical 179/179.
>
> SEED LIST for the new script (derived from the two hand ports):
> 1. trivial id-helpers in expression position (`riko(game)`); 2. `let`
>    with type annotations + tuple destructuring (symbol table — kills most
>    of the 91 errors); 3. deck-fill `for _ in 0..N` -> C `for`;
> 4. multi-statement setup-helper inlining; 5. pass/select_option/
>    set_live_card (shims exist); 6. `mods.*_modifiers.get(&id).map_or`
>    chains; 7. `assert!(has_pending_choice())`; 8. `card!()` consts ->
>    comment-out-and-continue; 9. whole-line drop (never inline-hollow)
>    for anything unresolvable.

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
