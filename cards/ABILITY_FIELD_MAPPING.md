# Ability System Status Report

## Overview

This document describes all action types, condition types, cost types, and trigger types found in `abilities.json` and their implementation status in the Rust engine (`engine/src/ability/`).

## Action Types

### Handled in `effects.rs` dispatch (51 variants)

| Action | Handler | Status | Notes |
|--------|---------|--------|-------|
| `sequential` | `execute_sequential_effect` | ✅ | OK |
| `conditional_alternative` | `execute_conditional_alternative` | ✅ | OK |
| `look_and_select` | `execute_look_and_select` | ✅ | OK |
| `draw` / `draw_card` | `execute_draw` | ✅ | OK |
| `draw_until_count` | `execute_draw_until_count` | ✅ | OK |
| `move_cards` | `execute_move_cards` | ✅ | OK |
| `gain_resource` (blade) | `execute_gain_resource` | ✅ | Applies blade modifiers |
| `gain_resource` (heart) | `execute_gain_resource` | ✅ | Applies heart modifiers |
| `gain_resource` (other) | `execute_gain_resource` | ⚠️ | `score`/`energy` resources silently no-op |
| `change_state` | `execute_change_state` | ✅ | OK |
| `modify_score` | `execute_modify_score` | ✅ | OK |
| `modify_required_hearts` | `execute_modify_required_hearts` | ✅ | OK |
| `set_cost` | `execute_set_cost` | ✅ | OK |
| `set_blade_type` | `execute_set_blade_type` | ✅ | OK |
| `set_heart_type` | `execute_set_heart_type` | ✅ | OK |
| `activate_ability` | `execute_activate_ability` | ✅ | OK |
| `invalidate_ability` | `execute_invalidate_ability` | ✅ | OK |
| `gain_ability` | `execute_gain_ability` | ✅ | OK |
| `play_baton_touch` | `execute_play_baton_touch` | ✅ | OK |
| `reveal` | `execute_reveal` | ✅ | OK |
| `select` | `execute_select` | ✅ | OK |
| `look_at` | `execute_look_at` | ✅ | OK |
| `modify_required_hearts_global` | `execute_modify_required_hearts_global` | ✅ | OK |
| `modify_yell_count` | `execute_modify_yell_count` | ✅ | OK |
| `place_energy_under_member` | `execute_place_energy_under_member` | ✅ | OK |
| `activation_cost` | `execute_activation_cost` | ✅ | OK |
| `position_change` | `execute_position_change` | ✅ | OK |
| `appear` | `execute_appear` | ✅ | OK |
| `choice` | `execute_choice` | ✅ | OK |
| `pay_energy` | `execute_pay_energy` | ✅ | OK |
| `set_card_identity` | `execute_set_card_identity` | ✅ | OK |
| `repeat_procedure` | `execute_repeat_procedure` | ❌ | Stub — no-op |
| `discard_until_count` | `execute_discard_until_count` | ✅ | OK |
| `restriction` | `execute_restriction` | ✅ | OK |
| `re_yell` | `execute_re_yell` | ✅ | OK |
| `modify_cost` | `execute_modify_cost` | ✅ | OK |
| `activation_restriction` | `execute_activation_restriction` | ✅ | OK |
| `choose_required_hearts` | `execute_choose_required_hearts` | ✅ | OK |
| `modify_limit` | `execute_modify_limit` | ✅ | OK |
| `set_blade_count` | `execute_set_blade_count` | ✅ | OK |
| `do_nothing` | `Ok(())` | ✅ | OK |
| `set_required_hearts` | `execute_set_required_hearts` | ✅ | OK |
| `set_score` | `execute_set_score` | ✅ | OK |
| `specify_heart_color` | `execute_specify_heart_color` | ✅ | OK |
| `modify_required_hearts_success` | `execute_modify_required_hearts_success` | ✅ | OK |
| `set_cost_to_use` | `execute_set_cost_to_use` | ✅ | OK |
| `all_blade_timing` | `execute_all_blade_timing` | ✅ | OK |
| `set_card_identity_all_regions` | `execute_set_card_identity_all_regions` | ✅ | OK |
| `shuffle` | `execute_shuffle` | ✅ | OK |
| `reveal_per_group` | `execute_reveal_per_group` | ✅ | OK |
| `conditional_on_result` | `execute_conditional_on_result` | ✅ | OK |
| `conditional_on_optional` | `execute_conditional_on_optional` | ✅ | OK |
| `custom` | `Ok(())` | ❌ | 7 uses — parser should map to proper types |

### Custom action types (7 remaining, need parser fixes)

| # | Text | Problem | Fix needed |
|---|------|---------|------------|
| 1 | `"コストは相手にエマパを委託する"` | Should be `modify_cost` operation | Parser: detect opponent cost transfer pattern |
| 2 | `"ライブの合計スコアが相手より高い場合..."` | Has `per_unit`, `per_unit_count`, `per_unit_type` | Parser: detect conditional score effect with per_unit |
| 3 | Character effects text | Has `character_effects` field | Parser: detect per-character effects |
| 4 | Cost reduction per hand card | Has `per_unit`, `location="hand"` | Parser: detect `appear` cost reduction with per_unit |
| 5-7 | Heart search effects in `select_action` | `action: "custom"` with `count`, `card_type` | Parser: detect heart condition search |

## Condition Types

### Handled in `condition.rs` dispatch (21 types)

| Condition Type | Handler | Status | Used in JSON |
|---------------|---------|--------|:-----------:|
| `compound` | `evaluate_compound_condition` | ✅ | Yes |
| `comparison_condition` | `evaluate_comparison_condition` | ✅ | Yes |
| `location_condition` | `evaluate_location_condition` | ✅ | Yes |
| `position_condition` | `evaluate_position_condition` | ✅ | Yes |
| `group_condition` | `evaluate_group_condition` | ✅ | Yes |
| `card_count_condition` | `evaluate_card_count_condition` | ✅ | Yes |
| `appearance_condition` | `evaluate_appearance_condition` | ✅ | Yes |
| `temporal_condition` | `evaluate_temporal_condition` | ✅ | Yes |
| `state_condition` | `evaluate_state_condition` | ✅ | Yes |
| `energy_state_condition` | `evaluate_energy_state_condition` | ✅ | Yes |
| `movement_condition` | `evaluate_movement_condition` | ✅ | Yes |
| `ability_negation_condition` | `evaluate_ability_negation_condition` | ✅ | Yes |
| `or_condition` | `evaluate_or_condition` | ✅ | Yes |
| `any_of_condition` | `evaluate_any_of_condition` | ✅ | No (dead code) |
| `score_threshold_condition` | `evaluate_score_threshold_condition` | ✅ | Yes |
| `choice_condition` | `evaluate_choice_condition` | ✅ | No (cost-only) |
| `position_change_condition` | `evaluate_position_change_condition` | ✅ | Yes |
| `state_change_condition` | `evaluate_state_change_condition` | ✅ | Yes |
| `opponent_choice_condition` | `evaluate_opponent_choice_condition` | ✅ | Yes |
| `opponent_live_success` | `evaluate_opponent_live_success_condition` | ✅ | Yes |
| `complex_condition` | `evaluate_complex_condition` | ✅ | Yes |
| `custom` | default `_ => true` | ❌ | 1 remaining (result-based: "無効にした場合") |

### Known condition `count` bug (FIXED)

All evaluators previously used `condition.count.unwrap_or(0)` which made `>= 0` comparisons always true. Fixed in `condition.rs`:
- `evaluate_location_condition`: defaults to `1` when filters present
- `evaluate_comparison_condition`: defaults to `1` when location/card_type set
- `evaluate_group_condition`: defaults to `1`
- `evaluate_card_count_condition`: defaults to `1`
- `evaluate_score_threshold_condition`: defaults to `1`

## Cost Types

| Cost Type | Handler | Status |
|-----------|---------|--------|
| `pay_energy` | `execute_pay_energy` | ✅ |
| `move_cards` | via `execute_move_cards` | ✅ |
| `reveal` | via `execute_reveal` | ✅ |
| `change_state` | via `execute_change_state` | ✅ |
| `choice_condition` | choice prompt | ✅ |
| `energy_condition` | energy check | ✅ |
| `sequential_cost` | sequential cost | ✅ |
| `custom` | no-op (optional: "支払ってもよい") | ⚠️ Treated as optional cost — handled |

## Trigger Types

| Trigger | Meaning | Status | Notes |
|---------|---------|--------|-------|
| `起動` | Activation | ✅ | Player-activatable |
| `自動` | Auto | ✅ | Auto-triggered |
| `常時` | Constant | ⚠️ | Replaced by `recalculate_constant_blade_modifiers` |
| `登場` | Debut | ✅ | When card enters stage |
| `ライブ開始時` | Live Start | ✅ | When live phase starts |
| `ライブ成功時` | Live Success | ✅ | When live succeeds |
| `左サイド` | Left Side | ❌ | **Not filtered** — should only trigger at left stage position |
| `右サイド` | Right Side | ❌ | **Not filtered** — should only trigger at right stage position |

### Multi-trigger combos found:
- `登場, 左サイド` / `登場, 右サイド` — Debut + position
- `起動, 左サイド` / `起動, 右サイド` — Activation + position
- `常時, 左サイド` / `常時, 右サイド` — Constant + position
- `ライブ開始時, 登場` — Live Start + Debut
- `ライブ開始時, 左サイド` / `ライブ開始時, 右サイド`
- `登場, 左サイド, 右サイド`

## Blade Modifier Integration

| Component | Uses blade_modifiers? | Status |
|-----------|----------------------|--------|
| `CardDisplay` (frontend sends `total_blade`) | ✅ | Yes — now sends `total_blade` |
| `HeaderStats` (total blade count) | ✅ | Yes — reads `total_blade` from stage cards |
| `card_to_display` | ✅ | Yes — accepts `blade_modifier` parameter |
| `Stage::total_blades()` | ❌ | **No** — used in performance/cheer/blade cost checks |
| `card_count_condition("member_card")` | ❌ | **No** — uses `Stage::total_blades()` |
| `zone_len("stage")` | ❌ | **No** — uses `Stage::total_blades()` |

## Constant Ability Recalculation

| Resource Type | Handled in `recalculate_constant_blade_modifiers` | Status |
|--------------|---------------------------------------------------|--------|
| `gain_resource` → blade | ✅ | Evaluates condition, applies/removes modifier |
| `gain_resource` → heart | ❌ | Not yet handled |
| `modify_score` | ❌ | Not yet handled |
| `modify_cost` | ❌ | Not yet handled |
| `appear` (cost reduction) | ❌ | Not yet handled |

## Required Engine Fixes

### Priority 1: Constant ability recalculate for hearts/score/cost
- Add `constant_heart_bonuses`, `constant_score_bonuses`, `constant_cost_bonuses` tracking to GameState
- Extend `recalculate_constant_ability_effects` to evaluate and apply heart/score/cost modifiers from 常時 abilities

### Priority 2: Position trigger filtering
- When generating available actions, check if card's stage position matches `左サイド`/`右サイド` trigger requirements
- `handle_use_ability` should reject ability use if card position doesn't match

### Priority 3: Blade modifiers in `Stage::total_blades()`
- Pass `blade_modifiers` to `Stage::total_blades()` and include modifier sums
- This affects: cheer check, performance calculation, blade cost payment

### Priority 4: Result-based condition (1 remaining `custom`)
- The condition `"無効にした場合"` (if invalidated) is a result check on the preceding action
- Need engine-level support: when processing sequential effects, track whether each effect succeeded and make that accessible to subsequent `condition` evaluations
