# engine_c — C Port of the Rabuka Engine

Status: **Foundation v0 — compiles-and-runs skeleton.** This is *not* a complete
faithful port yet. It proves the data pipeline + decoder + core loop work end to
end on a PC host and is architected so the remaining (large) piece — faithful
ability **effect execution** — slots in without restructuring.

## Key design decision (read this first)

The "C rewrite" is **only the game logic**. The following are *generated artifacts*
and are **not** hand-rewritten in C — they are embedded/loaded as data:

- `cards.bin` — compiled card data (from `cards/compile_cards.py`)
- `abilities_strings.bin` — ability string table
- ability **bytecode** — the 92,901-byte decompiled ability stream (from
  `cards/compile_abilities.py`), embedded as `RBKA_BYTECODE[]`

So the C engine *decodes* that bytecode (mirroring `ability/vm.rs`) and *executes*
the resulting effect tree. The 800 abilities come along for free as data; we only
rewrite the interpreter + game state machine.

## What works now (verified)

| Layer | File | State |
|---|---|---|
| Data load (cards.bin, strings, bytecode) | `src/data.c` | done |
| Bytecode decoder (Ability envelope + effect tree) | `src/vm.c` | done (foundation subset: text/action/source/dest/count/target + nested children + scalar extras) |
| Card decoder (cards.bin → `Card`) | `src/cards.c` | done |
| Game state + setup + turn loop | `src/engine.c` | done (RPS→mulligan→active/energy/draw/main skeleton) |
| Effect executor (subset) | `src/engine.c` `rb_execute_effect` | partial: `draw`, `gain/lose_energy`, `gain/place_heart`, `lose_heart`/`damage`, `gain_score`, `heal`; everything else **logged as unhandled** |
| PC CLI demo | `src/main.c` | done |
| Build (PC host) | `Makefile` | done |
| Unit/smoke test | `tests/test_basic.c` | written (pending compile) |
| Data generators | `tools/gen_from_rs.py`, `tools/gen_bytecode.py` | done |

## What needs to be done (checklist)

### A. Make it compile & green (blocking) — **DONE**
- [x] `Makefile` now compiles `src/gen_data.c` (it defines `RBKA_NUM_ABILITIES`,
      `RBKA_OFFSET_DELTAS[]`, `RBKA_STRINGS_OFFSETS[]` that `data.c` references —
      previously missing from `SRC`, so the build linked undefined symbols).
- [x] Fixed a **double tag-read bug** in `vm.c`: `rb_decode_ability` and
      `decode_effect_body` read the value `tag` themselves, but then called
      `rd_string_val()` which re-read the tag, desyncing the stream.
      `rd_string_val` now takes the already-read `tag` (like `decode_effect_value`
      / `skip_value` already did). This was why `ability[0].full_text` decoded
      to NULL.
- [x] `rb_engine` + `test_basic` compile clean (`-Wall`, no warnings) under the
      msys `gcc`.
- [x] `tests/test_basic.c` prints `ALL TESTS PASSED`.
- [x] Fixed `-Wmisleading-indentation` in `vm.c` (F64 case) and removed two
      genuinely-unused static helpers.

### B. Faithful effect execution (the real work)
The Rust `ability/effects/*` + `resolver.rs` implement ~hundreds of `action` kinds.
Our `rb_execute_effect` dispatches on `action` strings — each kind is one `else if`:
- [ ] Enumerate the full `action` vocabulary actually used (grep `ActionType` / effect `action` strings in `cards/abilities.json`)
- [ ] Implement the common combat/resource actions: `search_deck`, `move_card`, `place_member`/`move_member`, `change_score`, `set_energy`, `draw_until`, `shuffle`, `look`/`select`, `swap`, `reveal`, `discard`
- [ ] Implement condition evaluation (`has_condition`, `Condition` tree) — currently ignored
- [ ] Implement choice/`resume_with_choice` (the `rb_execute_effect` recursion handles children, but player choices need a stack)
- [ ] Implement trigger system (Debut/LiveStart/etc.) — currently no triggers fire
- [ ] Implement the **Live** phase (performance/baton-pass/heart-collection) — currently only the normal-turn loop exists

### C. Match correctness
- [ ] RPS → first-attacker selection
- [ ] Mulligan flow (both players)
- [ ] Energy/draw/main/end phase machine matching `turn/phases.rs`
- [ ] Win/lose/victory determination matching `turn/actions.rs::check_victory_condition`
- [ ] Deck-out, score-threshold, and heart-exhaustion loss paths

### D. Test converter (your "scenario-replay" idea)
Tests aren't ported verbatim — they assert engine *behavior*. The right tool is a
**scenario-replay harness**:
- [ ] Extract the JSON fixtures the Rust `#[test]`s use (game setups + action sequences)
- [ ] Write `tests/replay.c` that loads a fixture, drives the engine through the
      recorded actions, and asserts final state == expected (parity with Rust oracle)
- [ ] This gives real regression value without rewriting each `#[test]`

### E. CD-i (Philips, SCC68070, 1MB RAM, m68k-elf-gcc)
- [ ] `platforms/cdi/cdi_main.c` — text-grid render + joypad/serial input
- [ ] Disc layout: stream `cards.bin` + `abilities_strings.bin` + `RBKA_BYTECODE`
      from CD (don't RAM-load; 1MB is the wall — PORTS.md §CD-i)
- [ ] Linker script (app at $8000, loaded by cdi-serial stub)
- [ ] No allocator needed: static bump heap; `rb_unload` is a no-op on ROM data

### F. Cleanups
- [ ] Real `strdup` (currently local copies in vm.c/engine.c)
- [ ] `rb_unload` should free per-card/ability and string tables properly
- [ ] Remove vendored `src/bytecode_data.c` (superseded by generated `bytecode_blob.c`)

## How to build / run (PC)
```
cd engine_c
python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs
python3 tools/gen_bytecode.py
make            # builds rb_engine
make test       # builds + runs tests/test_basic
./rb_engine     # demo match
```
