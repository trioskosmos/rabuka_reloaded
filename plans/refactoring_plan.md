# Engine Refactoring Plan — Four Approaches

## Overview

The engine has four structural problems that make changes fragile. Each is addressed
below with a proposed approach, estimated effort, and expected impact.

---

## 1. Fix the Choice Round-Trip (highest-leverage bug source)

### Problem

The `AbilityResolver` is destroyed and recreated across choice boundaries. State is
split across 3 locations (resolver fields, queue entry fields, `GameState` fields)
with manual save/restore via `ResolverState`. This is the root cause of items 1–5
in `engine_issues.md`:

- Two zone matches in `handle_select_card` (cost vs effect) with early returns
  that skip `finalize_choice`
- `clear_choice_state` wipes `self.pending_choice` indiscriminately
- Resolver state lost between create/destroy cycles (new fields always missed)
- `store_pending_choice` vs `get_pending_choice()` desync between old/new resolver
- `finalize_choice` never called for SelectCard choices due to early `return Ok(())`

### Approach: Persistent Resolver

Instead of this:

```rust
// Current: create, use, discard
let mut resolver = AbilityResolver::new(game_state);
resolver.provide_choice_result(game_state, result);
// resolver dropped here, state manually saved to queue entry
```

Do this:

```rust
// Proposed: keep resolver alive on the queue entry
let resolver = game_state.ability_queue.current_resolver_mut();
resolver.provide_choice_result(game_state, result);
// resolver stays alive, state is directly in its fields
```

**Changes needed:**
- Move `AbilityResolver` from ephemeral stack variable to owned field on
  `AbilityQueueEntry` (or `AbilityQueue`)
- Remove `ResolverState` save/restore entirely
- `AbilityResolver::new()` becomes a one-time constructor, not a round-trip restore
- Remove `selected_card_ids`, `pending_stage_cards`, `last_effect_target` etc. from
  `AbilityQueueEntry` — they live on the resolver now
- `actions.rs:resume_queue_with_choice()` borrows the resolver from the queue
  instead of creating a fresh one

**Effort:** Medium (2–3 days)
**Impact:** Eliminates 5 known bugs, removes ~200 lines of save/restore boilerplate

---

## 2. String → Enum Migration (mechanical, prevents entire bug class)

### Problem

~150 unique string values used for zones, action types, targets, heart colors,
durations, operations, states, card types, and per-unit types. No enums = no
compiler checking. A typo like `"disacrd"` silently creates a new zone that
never matches any handler.

### Approach

Create enums and migrate incrementally:

**Priority 1 — Zone enum (most impactful, ~100 match sites):**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Hand,
    Stage,
    Waitroom,     // discard
    EnergyZone,
    EnergyDeck,
    MainDeck,
    LiveCardZone,
    SuccessLiveZone,
    ExclusionZone,
    Resolution,
    UnderMember,
    LookedAt,
    RevealedCards,
    SelectedCards,
    DeckTop,
    DeckBottom,
    SameArea,
    EmptyArea,
}
```

All current string zone references get replaced with `Zone::Hand` etc. The
`impl FromStr for Zone` + `impl Display for Zone` bridges to JSON serialization.

**Priority 2 — ActionType enum:**

Match the ~50 action strings in `effects/mod.rs`. This single change eliminates
the giant `match action_str.as_str()` that silently produces a no-op on typo.

**Priority 3 — Target, Duration, Operation, State, HeartColor:**

These have existing enums in `types.rs` and `card.rs` but are inconsistently used
(e.g. `HeartColor` enum exists but `heart_colors` field is `Vec<String>`).

**Migration strategy:**
1. Define the enum + parse/display impls
2. Add a helper function `zone(s: &str) -> Option<Zone>` for the gradual migration
3. One-by-one, change function signatures from `&str` to `Zone`, fix compiler errors
4. In the effects JSON deserialization, add a `#[serde(deserialize_with = "...")]`
   adapter that parses strings into enums

**Effort:** Large (4–5 days for Zone, 2–3 days for ActionType, 1 day each for others)
**Impact:** Entire class of bugs eliminated. Code becomes self-documenting.
           Autocomplete in IDE works. Zone handling becomes `match` with
           exhaustiveness checking.

---

## 3. Kill Dead Code + Split Monoliths

### Problem

~2900 lines of dead code identified in `ABILITY_CLEANUP_PLAN.md`:
- `CheerSystem` (~200 lines)
- `SelectionSystem` (~300 lines)
- `CardMatchingSystem` (~270 lines)
- `AutoAbilityListener`
- `Transactional` abstractions
- `ir/filter.rs`

The `effects/mod.rs` dispatcher is a 4240-line monolith with 53 helper functions.

### Approach

**Phase 1 — Deletion (1 day):**
Simply remove all identified dead code, fix any resulting compile errors (imports).

**Phase 2 — Split effects (2–3 days):**
Effects are already partially domain-split (`draw.rs`, `score.rs`, `state.rs`,
`misc.rs`, `ability_effects.rs`) but the main dispatch `execute_effect()` in
`mod.rs` still contains ~50 match arms. Extract each arm into its own function
in the appropriate sub-module, leaving only dispatch logic.

**Phase 3 — Split condition.rs (1–2 days):**
`condition.rs` is 2465 lines. Split by condition category:
- `condition/compound.rs` — AND/OR/any_of/not logic
- `condition/card.rs` — card count, location, position, group matching
- `condition/state.rs` — phase, duration, movement, score conditions
- `condition/mod.rs` — the `ConditionContext` struct and `evaluate_condition` dispatch

**Effort:** 4–6 days total
**Impact:** ~2900 lines deleted, monoliths become navigable modules

---

## 4. Split GameState (the god struct)

### Problem

`GameState` has ~100 fields covering players, ability queue, card database,
modifiers, resolution zone, tracking state, phase state, performance data,
and debug traces. Everything takes `&mut GameState`, making dependencies
impossible to see.

### Approach

Continue the pattern started with `GameModifiers` extraction. Group fields into
coherent sub-structs:

```rust
pub struct GameState {
    pub player1: Player,
    pub player2: Player,
    pub ability_queue: AbilityQueue,
    pub card_database: Arc<CardDatabase>,
    pub mods: GameModifiers,                    // already extracted
    pub phase: PhaseState,                      // NEW: current_phase, turn_phase,
                                                //   turn_number, heart_color_decision_phase,
                                                //   game_result, game_ended, loop_count
    pub performance: PerformanceData,           // NEW: performance_snapshots,
                                                //   live_owned_hearts, etc.
    pub tracking: EffectTracking,               // NEW: temporary_effects,
                                                //   prohibition_effects, cannot_activate_members,
                                                //   replacement_effects, gained_abilities,
                                                //   negated_abilities, etc.
    pub execution: ExecutionState,              // NEW: activating_card, looked_at_cards,
                                                //   revealed_cards, resolution_zone,
                                                //   pending_choice (wait, this overlaps...)
    pub history: HistoryData,                   // NEW: rule_log, game_state_history,
                                                //   last_ability_trace, performance_snapshots
    pub turn_state: TurnState,                  // NEW: areas_placed_this_turn,
                                                //   cards_appeared_this_turn,
                                                //   cards_moved_this_turn, etc.
}
```

Each sub-struct gets its own file and can implement its own methods, reducing
the `GameState` method surface.

**Optional further step:** Split into `GameState` (game objects) + `GameContext`
(metadata, logging, database) so engine functions that only need to read card
data don't need `&mut GameState`.

**Effort:** 3–5 days
**Impact:** Better cohesion, smaller compile units, makes the other refactors
           easier to reason about

---

## Recommendation & Ordering

| Order | Approach | Effort | Impact | Risk |
|-------|----------|--------|--------|------|
| 1 | Fix choice round-trip | 2–3 days | Highest (fixes 5 bugs, removes fragility) | Medium |
| 2 | Kill dead code | 1 day | Low-med (less noise, smaller codebase) | Low |
| 3 | Zone enum first | 4–5 days | High (eliminates bug class) | Medium |
| 4 | Split GameState | 3–5 days | Medium (better structure) | Medium |
| 5 | ActionType enum | 2–3 days | High (eliminates bug class) | Medium |
| 6 | Split monoliths | 3–4 days | Medium (navigability) | Low |

**Do first:** Choice round-trip fix. It's the smallest effort for the largest
correctness gain, and it unblocks the position-choice feature that was abandoned
in `current_changes.md`.

**Then:** Dead code deletion (free improvement, no risk).

**Then:** Enum migration (Zone first, then ActionType) — huge payoff for
mechanical work.

**Then:** Split GameState and monoliths — makes the codebase maintainable
long-term.
