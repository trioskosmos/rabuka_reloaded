# Plan: Make Ability Debugging Fast

## Problem

Debugging ability test failures takes too long. When a test breaks, you can't
quickly see **what happened** — which effect ran, what cards moved, where a
choice interrupted, or which condition failed. The root causes:

1. **Control flow jumps unpredictably** — `effects.rs` → `compound.rs` → `resolver.rs` → back,
   with recursive `execute_effect()` calls at every level
2. **State lives in 3 places** — `AbilityResolver` fields, `AbilityQueueEntry` fields,
   `GameState` fields — and the resolver is destroyed/recreated across choice boundaries
3. **Debug output is scattered** — inconsistent `eprintln!` calls with no structure,
   no zone snapshots, no execution tree
4. **`execute_effect` is a 4240-line monolith** — the dispatch + 53 helpers all in one file
5. **Test failures give no intermediate state** — just `assertion failed` at the end

## Phases

### Phase 1: Structured Execution Trace (do first)

Replace every `eprintln!` / `dbg.p()` call with a structured trace recorder.

**What it does:**
- Records every ability execution step as a tree node (not flat text)
- Each node captures: effect action, card name + card_no, zone snapshots before/after,
  choice descriptions, condition evaluations
- The trace lives on `EffectPipeline` so it survives resolver recreation across choices
- The trace is append-only: you build it up as the ability resolves
- A `TestGame::print_trace()` method dumps the full tree at the end
- On test panic, auto-prints the trace before unwinding

**Trace node structure:**
```
AbilityTrace {
  label: "sequential[2/4]: position_change",
  card: "若菜四季 (PL!SP-pb1-008-R)",
  before: ZoneSnapshot { hand: 3, stage: [Wakana, -, -], waitroom: 1, energy: 12 },
  after:  ZoneSnapshot { hand: 3, stage: [-, Wakana, -], waitroom: 1, energy: 12 },
  children: [...sub-traces...],
  choice: Some("Select area to move to (exclude Center)"),
  condition: Some(CondEval { type: "card_count_condition", passed: true, op: "==", actual: 1 }),
}
```

**Files to change:**
- `engine/src/ability/types.rs` — add `AbilityTrace` / `ZoneSnapshot` structs
- `engine/src/ability/debug.rs` — replace `AbDebug::p()` with trace recorder
- `engine/src/ability/effects.rs` — add trace calls before/after each effect dispatch
- `engine/src/ability/compound.rs` — add trace nodes for sequential/conditional
- `engine/src/ability/resolver.rs` — root trace node for the whole ability
- `engine/src/core/game_state/abilities.rs` — flush trace on completion
- `engine/tests/helpers/mod.rs` — add `TestGame::print_trace()` + auto-print on panic

### Phase 2: Split effects.rs into domain modules

The `engine/src/ability/effects/` directory exists but is empty. Move the 53 helper
methods into focused modules. The dispatch stays in `effects.rs`.

```
engine/src/ability/
├── effects.rs          ← execute_effect dispatch only (~200 lines)
├── effects/
│   ├── mod.rs          ← re-exports
│   ├── draw.rs         ← execute_draw, execute_draw_wrapper, execute_draw_until_count, execute_look_and_select, execute_select_cards, execute_reveal_effect
│   ├── move_cards.rs   ← execute_move_cards, execute_position_change, execute_rotation, execute_place_energy_under_member, execute_appear
│   ├── score.rs        ← execute_modify_score, execute_modify_required_hearts, execute_set_score, execute_modify_limit
│   ├── state.rs        ← execute_change_state, execute_activation_cost, execute_set_cost, execute_set_blade_type, execute_set_heart_type
│   ├── ability.rs      ← execute_gain_ability, execute_gain_ability_from_source, execute_invalidate_ability, execute_activate_ability
│   └── misc.rs         ← execute_restriction, execute_shuffle, execute_re_yell, execute_modify_cost, etc.
```

Each method stays on `impl AbilityResolver` (no structural refactor) — just file
splitting so you can find code by domain instead of scrolling 4240 lines.

### Phase 3: Better test failure messages

Add helpers that make test failures self-diagnosing:

- `TestGame::assert_hand(n, msg)` — `assert_eq!(hand.len(), n, "hand count: expected {n} got {}", hand.len())`
- `TestGame::assert_stage_pos(pos, card_no)` — with card name in failure message
- `TestGame::assert_energy(n)`
- `TestGame::assert_pending_choice_type(expected)`
- `TestGame::assert_trace_contains(pattern)` — regex search through trace

These are simple wrappers but save the "what value did it actually have?" hunt.

## Implementation order

```
Phase 1 ──→ Phase 2 ──→ Phase 3
 (trace)     (split)     (helpers)
```

Each phase is independently useful. Phase 1 alone will cut debugging time
significantly because every test failure will dump exactly what happened.
