# CENSUS 2 of 5 - DUPLICATION

Two passes, because they find different things.

- **Pass A, name-exact:** the same identifier defined in 2+ files. 15 names.
  This is what a per-cluster audit sees.
- **Pass B, by Rust twin:** strip the file-local role prefix (`d_`, `look_`,
  `cmf_`, `cost_`, `rb_desc_`, `rb_rules_`, ...) and group the survivors by
  what they port. **99 families.** This is the pass that finds duplication
  spanning clusters, which is the only kind a per-cluster audit cannot see.

Pass B compares C bodies to C bodies. That test is valid. A C-to-Rust token
similarity score is NOT - an earlier pass of this census computed one, produced
221 spurious "divergences" on functions that are faithful ports, and was thrown
away.

## Verifying before calling it a duplicate

`card_total_hearts` is reported by Pass A as a duplicate pair. It is **not** one.
`src/ability/condition.c:4011` ports Rust `Card::total_hearts` (base hearts if any,
else the need-heart window); `src/ability/util.c:705` ports
`original_heart_total` (the same window, via `heart_count_in_range`). Two
different Rust items with confusable names. **Merging them would have deleted a
correct function.** They are excluded from the fix list.

## Closure status of the nine already-known families

| # | family | status | evidence |
|---|---|---|---|
| 1 | `execute_gain_ability_from_source` | **CLOSED** | exactly one definition, `rb_translated_execute_gain_ability_from_source` at `ability_effects.c:612`, one caller at `executor.c:476` |
| 2 | `cached_condition_verdict` / `cond_cache_key` / `store_condition_verdict` | **PARTIALLY CLOSED - the gate is unified, the copies are not** | `cond_cache_key`: 2 copies, byte-identical. `cached_condition_verdict`/`store_condition_verdict`: 2 copies each, textually divergent but now SEMANTICALLY IDENTICAL - both call `rb_condition_get_cache` (compound.c:238 `cond_get_cache_flag`, resolver.c:113 `cond_get_cache`). A third live copy exists in resolver.c as `rb_resolver_cached_condition_verdict` (TEST-ONLY). |
| 3 | `eff_extra` (5 statics / 181 call sites) | **NOT CLOSED, AND WORSE THAN REPORTED** | **11 live ports**, not 5: `compound.c:32`, `cost.c:108`, `describe.c:272` (`d_extra`), `effects/look.c:79` (`look_extra`), `effects/misc.c:52`, `effects/move.c:166` (`cmf_extra`), `effects/move_missing.c:432` (`cmf_extra`), `resolver.c:92`, `util.c:258`, `core/modifiers.c:1384` (`cost_extra`), `engine.c:218` (`extra`). Three different null-guard policies. |
| 4 | `player_prefix` (7 copies) | **NOT CLOSED - 4 live copies remain** | `cost.c:604` `(g, card_id)` scanning stage/hand/**energy**; `effects/misc.c:127` and `resolver.c:888` no-`card_id`, scanning stage/**live**/hand with an `active` fallback; `effects/look.c:190` (`look_player_prefix`). `resolver.c` and `misc.c` take different arities. Confirmed NOT one of them: `rules.c:210`, which emits `player1`/`player2`. |
| 5 | `card_appeared_this_turn` | **NOT CLOSED** | 2 copies. `condition.c:1775` has the `cid < 0` guard; `effects/misc.c:294` does NOT, so a -1 card id returns 1 there. |
| 6 | `effect_extra` | **NOT CLOSED** | 2 copies (`ability_effects.c:7`, `executor.c:123`) differing only in brace style, plus a third port `game_setup.c:298` (`setup_effect_extra`). Semantically equal, still three bodies. |
| 7 | `remove_card_from_any_zone` | **CLOSED** | exactly one definition, `effects/move.c:476` |
| 8 | `cmf_extra` / `extra_true` | **NOT CLOSED** | `cmf_extra`: `move.c:166` omits the `!e \|\| !k` guard that `move_missing.c:432` has - a NULL effect dereferences on the move.c path. `extra_true`: 3 copies (`misc.c:58`, `move.c:108` `cmf_extra_true`, `move_missing.c:439`). |
| 9 | `restriction` handler family | **NOT CLOSED** | 4 dead twins remain in `effects/rules.c` (`rb_rules_execute_restriction`:1819, `rb_rules_execute_activation_restriction`:1932, `rb_rules_n_delayed_prohibitions`:2257, `rb_rules_delayed_prohibition`:2262) with zero callers, while the LIVE restriction handling is `h_restriction`/`h_activation_restriction` in `effects/misc.c`. |

**2 of 9 closed. 7 of 9 still open, one of them worse than reported.**

## Pass A - name-exact duplicates (15 names)

| name | copies | locations | verdict |
|---|---|---|---|
| `bag_push` | 2 | cost.c:2713, engine.c:39 | IDENTICAL - harmless |
| `cached_condition_verdict` | 3 | compound.c:242, resolver.c:316, resolver.c:298 | semantically identical (both delegate to rb_condition_get_cache) |
| `card_appeared_this_turn` | 2 | condition.c:1775, effects/misc.c:294 | DIVERGE - missing `cid < 0` guard in misc.c |
| `card_total_hearts` | 2 | condition.c:4011, util.c:705 | NOT A DUPLICATE - different Rust items (Card::total_hearts vs original_heart_total) |
| `cmf_extra` | 2 | effects/move.c:166, effects/move_missing.c:432 | DIVERGE - move.c lacks the !e/!k null guards |
| `cond_cache_key` | 3 | compound.c:214, compound.c:222, resolver.c:221 | IDENTICAL - harmless |
| `eff_extra` | 5 | compound.c:32, cost.c:108, effects/misc.c:52, resolver.c:92, util.c:258 | DIVERGE - three null-guard policies |
| `effect_extra` | 2 | effects/ability_effects.c:7, effects/executor.c:123 | IDENTICAL apart from brace style |
| `extra_true` | 2 | effects/misc.c:58, effects/move_missing.c:439 | semantically identical |
| `main` | 2 | debug_umi.c:5, main.c:107 | NOT A DUPLICATE - two mutually exclusive entry points, one per make target |
| `player_prefix` | 3 | cost.c:604, effects/misc.c:127, resolver.c:888 | DIVERGE - two arities, three zone sets, two fallbacks |
| `rb_load_gen_data` | 2 | core/gen_data_rom.c:106, core/generated/gen_data_cdi.c:60 | DIVERGE - two different generated-data loaders, only one per target |
| `rb_modifier_total` | 2 | core/stats_pipeline.c (static), include/rabuka.h:199 (static inline) | REAL DUPLICATION - a file-local static shadowing a header inline of the same name |
| `store_condition_verdict` | 3 | compound.c:252, resolver.c:326, resolver.c:340 | semantically identical |
| `zone_bag` | 2 | ability/util.c:1053, test_game.c:723 | DIVERGE - different zone-alias tables (util.c has deck_top/deck_bottom/success_live_zone; test_game.c has energy_deck and no deck_top/deck_bottom) |

## Pass B - by Rust twin: the 99 families a per-cluster audit cannot see

Ranked by number of distinct files. `max-sim` is C-to-C body similarity.

| family | copies | files | max-sim | live / dead mix |
|---|---|---|---|---|
| `extra` | 11 | 11 | see below | 11 live / 0 dead-or-test |
| `player_prefix` | 4 | 4 | see below | 4 live / 0 dead-or-test |
| `zone_label` | 5 | 4 | see below | 3 live / 2 dead-or-test |
| `effect_extra` | 3 | 3 | see below | 3 live / 0 dead-or-test |
| `extra_true` | 3 | 3 | see below | 3 live / 0 dead-or-test |
| `free` | 4 | 3 | see below | 3 live / 1 dead-or-test |
| `group_name_first` | 3 | 3 | see below | 3 live / 0 dead-or-test |
| `record_movement` | 3 | 3 | see below | 3 live / 0 dead-or-test |
| `bag_push` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `bool` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `cache_key` | 3 | 2 | see below | 3 live / 0 dead-or-test |
| `cached_condition_verdict` | 3 | 2 | see below | 2 live / 1 dead-or-test |
| `card_appeared_this_turn` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `card_ids` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `card_is_member` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `card_name` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `card_plural` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `card_total_hearts` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `card_type` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `card_type_any` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `card_type_from_str` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `card_type_label` | 3 | 2 | see below | 2 live / 1 dead-or-test |
| `card_type_label_ja` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `check_card_property` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `count` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `count_in_zone` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `destination` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `drain_verdicts` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `drain_verdicts_since` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `draw` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `draw_card` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `effective_blade` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `effective_energy_cost_total` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `emit_pay_skip_gate` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `entry_effect` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_modify_limit` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_modify_required_hearts` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_modify_required_hearts_standard` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_modify_required_hearts_success` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_modify_yell_count` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_move_cards` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `execute_selected_cards_from_zone` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `extra_int` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `filter_subset` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `fires_on_opponent_effects` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `floored_cost` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `get_bool` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_cache` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_card` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `get_distinct` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_group_names` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_pending_choice` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `get_position` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_positions_characters` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_score` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_str` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `get_string` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `group_name` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `handle_both_targets` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `handle_energy_zone_selection` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `handle_select_cards_looked_at` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `init` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `is_empty` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `is_energy` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `join_csv` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `len` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `load_gen_data` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `main` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `make_card_effect_data` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `malloc` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `move_non_selected_hand_to_deck_bottom` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `no_excess_heart_flag` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `num_abilities` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `orientation_matches_state` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `position_change` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `push_temporary_effect` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `push_verdict` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `refresh` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `resolve_dynamic_count` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `resolve_gain_heart_color` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `resolve_target_player` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `resume_pending_actions` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `resume_with_choice` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `set_blade_type` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `set_card_identity` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `set_cost` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `set_recently_moved_batch` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `shuffle` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `split_csv` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `state` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `state_from_str` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `store_condition_verdict` | 3 | 2 | see below | 2 live / 1 dead-or-test |
| `strdup2` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `translate_choice_prompt_en_to_ja` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `try_create_target_selection_choice` | 2 | 2 | see below | 1 live / 1 dead-or-test |
| `type_from_str` | 2 | 2 | see below | 0 live / 2 dead-or-test |
| `use_limit_reached` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `zone_bag` | 2 | 2 | see below | 2 live / 0 dead-or-test |
| `zone_label_ja` | 2 | 2 | see below | 0 live / 2 dead-or-test |

### The families worth fixing first

#### `extra` - 11 ports
- `src/ability/compound.c:32` `eff_extra` (static, WIRED)
- `src/ability/cost.c:108` `eff_extra` (static, WIRED)
- `src/ability/describe.c:272` `d_extra` (static, WIRED)
- `src/ability/effects/look.c:79` `look_extra` (static, WIRED)
- `src/ability/effects/misc.c:52` `eff_extra` (static, WIRED)
- `src/ability/effects/move.c:166` `cmf_extra` (static, WIRED)
- `src/ability/effects/move_missing.c:432` `cmf_extra` (static, WIRED)
- `src/ability/resolver.c:92` `eff_extra` (static, WIRED)
- `src/ability/util.c:258` `eff_extra` (static, WIRED)
- `src/core/modifiers.c:1384` `cost_extra` (static, WIRED)
- `src/engine.c:218` `extra` (static, WIRED)

#### `player_prefix` - 4 ports
- `src/ability/cost.c:604` `player_prefix` (static, WIRED)
- `src/ability/effects/look.c:190` `look_player_prefix` (static, WIRED)
- `src/ability/effects/misc.c:127` `player_prefix` (static, WIRED)
- `src/ability/resolver.c:888` `player_prefix` (static, WIRED)

#### `zone_label` - 5 ports
- `src/ability/describe.c:485` `d_zone_label` (static, WIRED)
- `src/ability/describe.c:1832` `rb_desc_zone_label` (GLOBAL, TEST-ONLY)
- `src/ability/effects/look.c:104` `look_zone_label` (static, DEAD)
- `src/ability/effects/move.c:564` `move_zone_label` (static, WIRED)
- `src/turn/triggers.c:1634` `rb_trigger_zone_label` (GLOBAL, WIRED)

#### `group_name_first` - 3 ports
- `src/ability/cost.c:367` `group_name_first` (static, WIRED)
- `src/ability/util.c:608` `rb_group_name_first` (GLOBAL, WIRED)
- `src/core/modifiers.c:1428` `cost_group_name_first` (static, WIRED)

#### `effect_extra` - 3 ports
- `src/ability/effects/ability_effects.c:7` `effect_extra` (static, WIRED)
- `src/ability/effects/executor.c:123` `effect_extra` (static, WIRED)
- `src/game/game_setup.c:298` `setup_effect_extra` (static, WIRED)

#### `extra_true` - 3 ports
- `src/ability/effects/misc.c:58` `extra_true` (static, WIRED)
- `src/ability/effects/move.c:108` `cmf_extra_true` (static, WIRED)
- `src/ability/effects/move_missing.c:439` `extra_true` (static, WIRED)

#### `record_movement` - 3 ports
- `src/ability/effects/misc.c:169` `record_movement` (static, WIRED)
- `src/ability/effects/move.c:507` `mc_record_movement` (static, WIRED)
- `src/ability/effects/state.c:1610` `rb_record_movement` (GLOBAL, WIRED)

#### `cached_condition_verdict` - 3 ports
- `src/ability/compound.c:242` `cached_condition_verdict` (static, WIRED)
- `src/ability/resolver.c:298` `rb_resolver_cached_condition_verdict` (GLOBAL, TEST-ONLY)
- `src/ability/resolver.c:316` `cached_condition_verdict` (static, WIRED)

#### `card_type_any` - 2 ports
- `src/ability/compound.c:307` `eff_card_type_any` (static, WIRED)
- `src/ability/cost.c:150` `cost_card_type_any` (static, WIRED)

#### `card_is_member` - 2 ports
- `src/ability/condition.c:2885` `cond_card_is_member` (static, WIRED)
- `src/engine.c:213` `card_is_member` (static, WIRED)

#### `card_total_hearts` - 2 ports
- `src/ability/condition.c:4011` `card_total_hearts` (static, WIRED)
- `src/ability/util.c:705` `card_total_hearts` (static, WIRED)

#### `check_card_property` - 2 ports
- `src/ability/condition.c:3242` `check_card_property` (static, WIRED)
- `src/ability/util.c:3029` `rb_check_card_property` (GLOBAL, WIRED)

#### `get_pending_choice` - 2 ports
- `src/ability/choice.c:4692` `rb_get_pending_choice` (GLOBAL, WIRED)
- `src/ability/resolver.c:279` `rb_resolver_get_pending_choice` (GLOBAL, DEAD)

#### `card_name` - 2 ports
- `src/ability/choice.c:297` `rb_resolver_card_name` (GLOBAL, DEAD)
- `src/ability/effects/misc.c:2270` `card_name` (static, TEST-ONLY)

#### `entry_effect` - 2 ports
- `src/ability/choice.c:309` `rb_resolver_entry_effect` (GLOBAL, DEAD)
- `src/core/game_state_abilities.c:1586` `rb_entry_effect` (GLOBAL, WIRED)

#### `get_card` - 2 ports
- `src/ability/choice.c:76` `rb_get_card` (GLOBAL, DEAD)
- `src/core/card.c:813` `rb_card_get_card` (GLOBAL, TEST-ONLY)

#### `free` - 4 ports
- `src/ability/describe.c:1750` `rb_desc_free` (GLOBAL, WIRED)
- `src/core/alloc.c:19` `rb_free` (GLOBAL, WIRED)
- `src/core/alloc.c:67` `rb_effect_free` (GLOBAL, WIRED)
- `src/core/alloc_counter.c:680` `rb_allocctr_free` (GLOBAL, DEAD)

#### `card_appeared_this_turn` - 2 ports
- `src/ability/condition.c:1775` `card_appeared_this_turn` (static, WIRED)
- `src/ability/effects/misc.c:294` `card_appeared_this_turn` (static, WIRED)

#### `zone_bag` - 2 ports
- `src/ability/util.c:1053` `zone_bag` (static, WIRED)
- `src/test_game.c:723` `zone_bag` (static, WIRED)

