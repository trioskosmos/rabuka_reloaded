# Capability Matrix

Generated from 602 unique abilities

## Action Types

| Action | In Parser | In Rust | Count | Notes |
|--------|-----------|---------|-------|-------|
| `activate_ability` | 2 | yes | 2 |  |
| `activation_cost` | 0 | yes | 0 | rust-only (stale handler?) |
| `activation_restriction` | 0 | yes | 0 | rust-only (stale handler?) |
| `all_blade_timing` | 0 | yes | 0 | rust-only (stale handler?) |
| `appear` | 6 | yes | 6 |  |
| `change_state` | 58 | yes | 58 |  |
| `choice` | 8 | yes | 8 |  |
| `choose_required_hearts` | 0 | yes | 0 | rust-only (stale handler?) |
| `conditional_alternative` | 7 | yes | 7 |  |
| `conditional_on_optional` | 0 | yes | 0 | rust-only (stale handler?) |
| `conditional_on_result` | 0 | yes | 0 | rust-only (stale handler?) |
| `discard_until_count` | 0 | yes | 0 | rust-only (stale handler?) |
| `draw_card` | 38 | yes | 38 |  |
| `draw_until_count` | 1 | yes | 1 |  |
| `gain_ability` | 5 | yes | 5 |  |
| `gain_resource` | 119 | yes | 119 |  |
| `invalidate_ability` | 0 | yes | 0 | rust-only (stale handler?) |
| `look_and_select` | 57 | yes | 57 |  |
| `look_at` | 0 | yes | 0 | rust-only (stale handler?) |
| `looked_at` | 0 | yes | 0 | rust-only (stale handler?) |
| `modify_cost` | 8 | yes | 8 |  |
| `modify_limit` | 0 | yes | 0 | rust-only (stale handler?) |
| `modify_required_hearts` | 0 | yes | 0 | rust-only (stale handler?) |
| `modify_required_hearts_global` | 0 | yes | 0 | rust-only (stale handler?) |
| `modify_required_hearts_success` | 0 | yes | 0 | rust-only (stale handler?) |
| `modify_score` | 67 | yes | 67 |  |
| `modify_yell_count` | 3 | yes | 3 |  |
| `move_cards` | 77 | yes | 77 |  |
| `pay_energy` | 0 | yes | 0 | rust-only (stale handler?) |
| `place_energy_under_member` | 1 | yes | 1 |  |
| `play_baton_touch` | 1 | yes | 1 |  |
| `position_change` | 13 | yes | 13 |  |
| `re_yell` | 0 | yes | 0 | rust-only (stale handler?) |
| `repeat_procedure` | 0 | yes | 0 | rust-only (stale handler?) |
| `restriction` | 21 | yes | 21 |  |
| `reveal` | 0 | yes | 0 | rust-only (stale handler?) |
| `reveal_per_group` | 0 | yes | 0 | rust-only (stale handler?) |
| `select` | 0 | yes | 0 | rust-only (stale handler?) |
| `sequential` | 104 | yes | 104 |  |
| `set_blade_count` | 1 | yes | 1 |  |
| `set_blade_type` | 0 | yes | 0 | rust-only (stale handler?) |
| `set_card_identity` | 1 | yes | 1 |  |
| `set_card_identity_all_regions` | 0 | yes | 0 | rust-only (stale handler?) |
| `set_cost` | 0 | yes | 0 | rust-only (stale handler?) |
| `set_cost_to_use` | 0 | yes | 0 | rust-only (stale handler?) |
| `set_heart_type` | 0 | yes | 0 | rust-only (stale handler?) |
| `set_required_hearts` | 0 | yes | 0 | rust-only (stale handler?) |
| `set_score` | 1 | yes | 1 |  |
| `shuffle` | 0 | yes | 0 | rust-only (stale handler?) |
| `specify_heart_color` | 0 | yes | 0 | rust-only (stale handler?) |

## Condition Types

| Type | In Parser | In Rust | Count |
|------|-----------|---------|-------|
| `ability_negation_condition` | 2 | unclear | 2 |
| `active` | 0 | yes | 0 |
| `any` | 0 | yes | 0 |
| `appearance_condition` | 14 | unclear | 14 |
| `baton_touch` | 0 | yes | 0 |
| `before_live` | 0 | yes | 0 |
| `card_count_condition` | 59 | unclear | 59 |
| `center` | 0 | yes | 0 |
| `comparison_condition` | 31 | unclear | 31 |
| `complex_condition` | 8 | unclear | 8 |
| `compound` | 18 | unclear | 18 |
| `cost` | 0 | yes | 0 |
| `custom` | 1 | unclear | 1 |
| `deck` | 0 | yes | 0 |
| `discard` | 0 | yes | 0 |
| `either` | 0 | yes | 0 |
| `energy` | 0 | yes | 0 |
| `energy_card` | 0 | yes | 0 |
| `energy_state_condition` | 1 | unclear | 1 |
| `energy_zone` | 0 | yes | 0 |
| `first_turn` | 0 | yes | 0 |
| `from_stage` | 0 | yes | 0 |
| `group_condition` | 39 | unclear | 39 |
| `hand` | 0 | yes | 0 |
| `has_blade_heart` | 0 | yes | 0 |
| `has_energy` | 0 | yes | 0 |
| `has_hand` | 0 | yes | 0 |
| `has_live_card` | 0 | yes | 0 |
| `has_member` | 0 | yes | 0 |
| `has_moved` | 5 | unclear | 5 |
| `is_active_phase` | 0 | yes | 0 |
| `is_main_phase` | 0 | yes | 0 |
| `left_side` | 0 | yes | 0 |
| `live_card` | 0 | yes | 0 |
| `live_card_set` | 0 | yes | 0 |
| `live_card_zone` | 0 | yes | 0 |
| `live_end` | 0 | yes | 0 |
| `live_performance` | 0 | yes | 0 |
| `live_victory` | 0 | yes | 0 |
| `location_condition` | 96 | unclear | 96 |
| `member_card` | 0 | yes | 0 |
| `moved` | 0 | yes | 0 |
| `movement_condition` | 6 | unclear | 6 |
| `not_moved` | 1 | unclear | 1 |
| `notmoved` | 0 | yes | 0 |
| `opponent_choice_condition` | 2 | unclear | 2 |
| `opponent_live_success` | 1 | unclear | 1 |
| `or_condition` | 1 | unclear | 1 |
| `position_change_condition` | 1 | unclear | 1 |
| `position_condition` | 7 | unclear | 7 |
| `right_side` | 0 | yes | 0 |
| `score` | 0 | yes | 0 |
| `score_threshold_condition` | 1 | unclear | 1 |
| `stage` | 0 | yes | 0 |
| `state_change_condition` | 1 | unclear | 1 |
| `state_condition` | 8 | unclear | 8 |
| `success_live_zone` | 0 | yes | 0 |
| `temporal_condition` | 15 | unclear | 15 |
| `this_live` | 0 | yes | 0 |
| `this_turn` | 0 | yes | 0 |
| `to_discard` | 0 | yes | 0 |
| `to_stage` | 0 | yes | 0 |
| `wait` | 0 | yes | 0 |
| `waitroom` | 0 | yes | 0 |

## Summary

- Parser action types: 22
- Rust handler action types: 50
- Parser condition types: 22
- All parser action types have Rust handlers