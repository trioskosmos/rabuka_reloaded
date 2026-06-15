# Ability Consolidation Tracker

This note tracks places where the ability / condition system is split across closely related branches.

Legend:
- `[x]` already consolidated
- `[ ]` still a good consolidation candidate
- `[~]` related, but probably keep separate unless a later refactor gives a clear win

## Already Consolidated

- [x] Centralized effect gating so `can_activate_effect` and `execute_effect` no longer duplicate the same condition check.
- [x] Added shared trigger matching and queue-entry helpers so auto-trigger lookup and trigger filtering stay aligned.
- [x] Added shared constant-ability collection so blade, cost, and full recalculation paths reuse the same stage scan.
- [x] Factored small repeated count-default helpers in `condition.rs` for comparison and location-style thresholds.
- [x] Reduced repeated choice teardown / resume branches with shared helpers.
- [x] Consolidated nested boolean composition through a shared condition-list evaluator.
- [x] Centralized target-to-player resolution for the state, movement, appearance, and heart-related condition paths.
- [x] Unified standalone and live-success `no_excess_heart` checks behind one helper.
- [x] Unified the shared counting core behind `comparison_condition`, `location_condition`, `card_count_condition`, and `group_condition`.
- [x] Collapsed `get_count_for_condition`, `get_count_for_target`, and `get_group_card_count` onto shared helper paths.
- [x] Removed dead / unused helper code that was no longer needed after the consolidation pass.

## Probably Keep Separate

- [~] `appearance_condition` and `position_condition`.
  - They are related, but one is about presence / visibility and the other is about exact slot occupancy.
- [~] `comparison_condition` and `score_threshold_condition`.
  - Both compare counts, but they are semantically different enough that they can stay as distinct entry points.
- [~] `evaluate_location_condition` and `evaluate_multi_location_condition`.
  - They should share helpers, but the multi-location union / equality behavior is different enough to justify a separate branch.

## Notes

- The `Condition` struct in [`engine/src/core/card.rs`](../core/card.rs) is broad, so some separation is expected.
- The best long-term gain is to extract a shared evaluation core for counting and boolean composition while keeping the existing schema names as wrappers.
- If we continue the refactor, the safest order is:
  1. shared counting core
  2. shared composition helpers
  3. state-transition helper cleanup
  4. targeted removal of duplicate special cases
