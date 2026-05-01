# Ability System Cleanup Plan

## Priority 1: Easy wins (zero behavioral change, just extraction)

| # | What | Why | Impact |
|---|------|-----|--------|
| 1 | Extract `matches_card_type`, `matches_group`, `matches_cost_limit` closures as methods on `AbilityResolver` | Appears 6× in effects.rs, move_cards.rs, choice.rs, cost.rs — ~90 lines of identical closures | -80 lines |
| 2 | Extract comparison operator helper (`>=`, `>`, `<=`, etc.) | Appears 5× in condition.rs | -25 lines |
| 3 | Extract `target_to_str()`, `count_to_u32()`, `duration_to_str()` helpers | Appears 20+ times in ae_from_ir | -50 lines |
| 4 | Extract zone-to-count mapping helper | Duplicated in `get_count_for_condition` and `get_count_for_target` | -12 lines |
| 5 | Extract per_unit multiplier helper | Duplicated in `execute_gain_resource` and `execute_modify_score` | -10 lines |

## Priority 2: Split massive functions (same behavior, better organization)

| # | What | Why | Impact |
|---|------|-----|--------|
| 6 | Break `provide_choice_result` into handler methods | 299-line function with 10+ independent target branches | -150 lines |
| 7 | Break `pay_cost` optional cost setup into helper | Same 15-line `PendingAbilityExecution` block 3× | -25 lines |
| 8 | Break `execute_change_state` wait/active into shared path | 18 lines of duplicated orientation logic | -10 lines |
| 9 | Break `execute_reveal`/`execute_select` shared source collection | 8 lines of card-ID collection | -10 lines |

## Priority 3: High-impact refactors (risk of behavioral change — test carefully)

| # | What | Why | Impact |
|---|------|-----|--------|
| 10 | Extract generic `move_cards_from_to()` helper | Replaces 42 copy-pasted source×destination paths in move_cards.rs | -600 lines |
| 11 | Extract `place_on_stage()` helper | Stage placement triple-if duplicated 3× | -20 lines |
| 12 | Extract reverse-remove-and-add pattern | 17 uses across move_cards.rs | -50 lines |
| 13 | Extract `prompt_card_selection()` helper | 6 identical SelectCard choice setups | -40 lines |
| 14 | Extract score-sum pattern in condition.rs | 4 identical zone→score→sum blocks | -15 lines |

## Priority 4: Bugfixes (change behavior)

| # | What | Why |
|---|------|-----|
| 15 | `execute_shuffle` always targets player1 | Effect target is misread as zone selector, never checks self/opponent |
| 16 | `execute_conditional_on_result` ignores `result_condition` | Followup always fires regardless of primary's result |
| 17 | `execute_conditional_on_optional` ignores `optional_action` | Both actions always execute unconditionally |

## Priority 5: Delete dead code (zero risk)

| # | What | Lines |
|---|------|-------|
| 18 | `CheerSystem` (cheer_system.rs) — never constructed | ~200 |
| 19 | `SelectionSystem` (selection_system.rs) — never constructed | ~300 |
| 20 | `CardMatchingSystem` (card_matching.rs) — never constructed | ~270 |
| 21 | `AutoAbilityListener` / `register_listener` (events.rs) — never constructed/called | ~90 |
| 22 | `Transactional` / `resolve_ability_atomic` (transaction.rs) — never used | ~60 |
| 23 | `ir/filter.rs` (CardFilter, OrientationFilter) — never imported | ~80 |
| 24 | Dead trigger constants (triggers.rs) — 7 unused | ~7 |
| 25 | `INITIAL_DRAW_COUNT` (constants.rs) — unused | ~1 |
| 26 | `parse_blade_color` (zones.rs) — never called | ~20 |
| 27 | `Choice::SelectHeartColor` / `SelectHeartType` (types.rs) — never constructed | ~10 |
| 28 | `is_human_decision_phase`, `is_player2_decision_phase`, `execute_player2_ai_action`, `rooms_list` (web_server.rs) — dead | ~60 |

**Total estimated lines removed:** ~2900 (1300 from extraction + 1100 from dead code)
