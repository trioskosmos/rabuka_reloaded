# Refactor Plan: Auto Ability Trigger System

## Problem

`trigger_auto_abilities_for_player` brute-force scans ALL stage cards with
`triggers == "自動"` and enqueues them unconditionally.  The condition check
is deferred to ability resolution, so abilities queue even when their
triggering event hasn't occurred (e.g. "when this member moves" queues even
though the card hasn't moved).

The tracking state is fragmented across multiple overlapping fields with
different lifetimes:
- `recently_moved_cards: Vec<i16>`  (cleared per-ability-loop)
- `cards_moved_this_turn: HashSet<i16>`  (persistent for turn)
- `position_change_occurred_this_turn: bool`
- `last_area_move_card_id: Option<i16>`
- `cards_appeared_this_turn: HashSet<i16>`
- `last_energy_placed_by_effect: bool`
- `last_energy_placed_by_player: Option<String>`

Additionally, `AbilityEffect` has TWO condition fields that are the same type:
- `condition: Option<Condition>` — gated during ability resolution
- `trigger_condition: Option<Box<Condition>>` — gated during TAS scanning

This forces callers to track what "happened" via side-effect flags rather than
by passing the actual event into TAS.

## Plan (execute in order, run tests after each step)

### Step 1 — Unify `condition` + `trigger_condition`

`trigger_condition` is already `Option<Box<Condition>>` — literally the same
struct as `condition`.  Remove `trigger_condition` from `AbilityEffect` and
move all scanning-time checks into `condition`.  In TAS, evaluate `condition`
during scanning for types that describe the triggering event (movement,
appearance).  During resolution, `condition` is still evaluated normally for
effect gating.

### Step 2 — Introduce `TriggerEvent`

Replace fragmented tracking flags with a single `TriggerEvent` enum passed
into TAS:

```rust
enum TriggerEvent {
    None,
    CardMoved { card_ids: Vec<i16>, is_position_change: bool },
    CardAppeared { card_ids: Vec<i16> },
    EnergyPlaced,
    Mixed { ... },
}
```

TAS uses the event to pre-filter conditions without reading stale flags.
All call sites that currently fiddle with flags construct a `TriggerEvent`
instead.

### Step 3 — Single `record_card_movement` call site

Currently `execute_position_change` (misc.rs:1893) updates
`recently_moved_cards` but does NOT call `record_card_movement` (which
updates `cards_moved_this_turn`).  Only
`execute_position_change_with_destination` calls both.  Unify so every
position change code path records movement the same way.

### (Removed — see user note) ~~Remove Main phase guards~~
Main phase guards at abilities.rs:639 and :920 are kept — some abilities
explicitly specify "during main phase only" in their text.
