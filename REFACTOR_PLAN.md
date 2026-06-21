# Refactor Plan: Auto Ability Trigger System

## Status: 1087 tests passing — all items complete

### Step 1 — Unify `condition` + `trigger_condition`
- Removed `trigger_condition: Option<Box<Condition>>` from `AbilityEffect`
- `condition: Option<Condition>` is the single field
- `trigger_condition` key in JSON is handled by:
  - Parser no longer emits it (outputs `condition` instead)
  - `card_loader.rs` has a safety-net merge for old abilities.json
- TAS pre-filter evaluates `condition` during scanning for **event-based** types:
  - `movement_condition` with `"moved"` or `"moves"`
  - `appearance_condition`
  - `card_count_condition` with `source: "preceding_moved"`
  - Compound conditions checked recursively
- Heuristic guard preserved: `each_time` + `comparison_condition` on
  `energy_zone` needs `last_energy_placed_by_effect` flag

### Step 2 — Introduce `TriggerEvent`
- `TriggerEvent` struct in `ability::types` carries explicit scan context:
  - `moved_cards`, `moved_from_zone`, `position_change_occurred`
  - `appeared_cards`, `energy_placed_by_effect`, `energy_placed_by_player`
- Core TAS function `trigger_auto_abilities_for_player_with_event` takes
  `&TriggerEvent` — pre-filter reads from event, not from `self` flags
- Legacy wrapper `trigger_auto_abilities_for_player` constructs event from
  flags (backward compatible)
- Internal callers (`process_player_abilities`, `process_current_ability`)
  now construct proper events

### Step 3 — Single `record_card_movement` call site
- `execute_position_change` (misc.rs:1893) was missing `record_card_movement`
  calls. Added them — both position-change code paths now record persistently.
