# CENSUS 5 of 5 - WORK LIST

Ordered by blast radius, not by file. **FREE** = no uncommitted work in
`git status`; **CONTESTED** = another agent has uncommitted changes in that
file right now, so a fixer must coordinate before editing.

## Order of operations

1. **Header first.** `include/rabuka.h` is the only file every other change
   lands in, and it is the file a stale-prototype fix touches. Doing it first
   unblocks everything else.
2. **The `eff_extra` family (11 ports).** It has 181 call sites; collapsing it
   is a mechanical, compiler-checked change that removes the largest single
   source of divergence in the tree.
3. **The three guard-dropping duplicates.** Each is a NULL-dereference or a
   wrong-answer-on-(-1) that only fires when the code is wired.
4. **The 0W/0R fields.** Cheap to resolve: either wire them or delete them.
5. **The dead globals.** Large and mostly mechanical; safe in bulk because
   `gcc` will prove any that were actually reachable.
6. **The stale citations.** Comment-only, zero risk, do them last.

## Per-file list

### `include/rabuka.h` - 17 items - **CONTESTED**

LANDMINE x7, FALSE-COMMENT x9, DEAD-FIELD x1

- [LANDMINE] rb_activate_ability_effect is declared here and defined nowhere (include/rabuka.h:2247). Any caller is a link error.
- [LANDMINE] rb_cost_clear_deferred is declared here and defined nowhere (include/rabuka.h:2700). Any caller is a link error.
- [LANDMINE] rb_effect_modify_hearts is declared here and defined nowhere (include/rabuka.h:2269). Any caller is a link error.
- [LANDMINE] rb_execute_effect_via_registry is declared here and defined nowhere (include/rabuka.h:3238). Any caller is a link error.
- [LANDMINE] rb_invalidate_ability is declared here and defined nowhere (include/rabuka.h:2245). Any caller is a link error.
- [LANDMINE] rb_load_rom is declared here and defined nowhere (include/rabuka.h:373). Any caller is a link error.
- [LANDMINE] rb_translated_execute_custom_effect is declared here and defined nowhere (include/rabuka.h:3245). Any caller is a link error.
- [FALSE-COMMENT] line 240: success_zone_* comment calls the three ledgers load-bearing and reachable from 214 effect nodes; all three are 0W/0R
- [FALSE-COMMENT] line 1824: constant_need_heart_global comment claims three live consumers; 0W/0R
- [FALSE-COMMENT] line 373: declares rb_load_rom, defined only under #ifdef RB_ROM_STRINGS
- [FALSE-COMMENT] line 2245: rb_invalidate_ability: stale prototype
- [FALSE-COMMENT] line 2247: rb_activate_ability_effect: stale prototype
- [FALSE-COMMENT] line 2269: rb_effect_modify_hearts: stale prototype
- [FALSE-COMMENT] line 2700: rb_cost_clear_deferred: stale prototype
- [FALSE-COMMENT] line 3238: rb_execute_effect_via_registry: stale prototype
- [FALSE-COMMENT] line 3245: rb_translated_execute_custom_effect: stale prototype
- [DEAD-FIELD] 25 fields are 0 writers / 0 readers (full list in CENSUS_fields.md 4a)

### `src/ability/abilities_gen.c` - 1 items - **FREE**

DEAD x1

- [DEAD] src/ability/abilities_gen.c:984 `bytecode_slice` (GLOBAL)

### `src/ability/ability_queue.c` - 2 items - **FREE**

DEAD x2

- [DEAD] src/ability/ability_queue.c:307 `rb_queue_push_with_trigger` (GLOBAL)
- [DEAD] src/ability/ability_queue.c:967 `rb_queue_save_pending_actions` (GLOBAL)

### `src/ability/choice.c` - 32 items - **CONTESTED**

DEAD x32

- [DEAD] src/ability/choice.c:5691 `rb_ability_error_to_string` (GLOBAL)
- [DEAD] src/ability/choice.c:85 `rb_ability_master_id` (GLOBAL)
- [DEAD] src/ability/choice.c:77 `rb_card_db_unit` (GLOBAL)
- [DEAD] src/ability/choice.c:5657 `rb_choice_description_ja` (GLOBAL)
- [DEAD] src/ability/choice.c:86 `rb_choice_destination` (GLOBAL)
- [DEAD] src/ability/choice.c:101 `rb_compound_route_conditional_branch` (GLOBAL)
- [DEAD] src/ability/choice.c:168 `rb_effect_alternative_count_type_any` (GLOBAL)
- [DEAD] src/ability/choice.c:118 `rb_effect_answers_any` (GLOBAL)
- [DEAD] src/ability/choice.c:162 `rb_effect_resource_on_select` (GLOBAL)
- [DEAD] src/ability/choice.c:174 `rb_entry_conditional_choice_effect` (GLOBAL)
- [DEAD] src/ability/choice.c:5649 `rb_gained_ability_index` (GLOBAL)
- [DEAD] src/ability/choice.c:76 `rb_get_card` (GLOBAL)
- [DEAD] src/ability/choice.c:4477 `rb_resolver_apply_effect_modification` (GLOBAL)
- [DEAD] src/ability/choice.c:240 `rb_resolver_build_choice_select_cards` (GLOBAL)
- [DEAD] src/ability/choice.c:1061 `rb_resolver_build_reprompt` (GLOBAL)
- [DEAD] src/ability/choice.c:1072 `rb_resolver_build_reprompt_full` (GLOBAL)
- [DEAD] src/ability/choice.c:297 `rb_resolver_card_name` (GLOBAL)
- [DEAD] src/ability/choice.c:456 `rb_resolver_clear_choice_state_and_resume2` (GLOBAL)
- [DEAD] src/ability/choice.c:309 `rb_resolver_entry_effect` (GLOBAL)
- [DEAD] src/ability/choice.c:3297 `rb_resolver_filter_discard_by_budget` (GLOBAL)
- [DEAD] src/ability/choice.c:737 `rb_resolver_finalize_choice_with_ctx` (GLOBAL)
- [DEAD] src/ability/choice.c:5710 `rb_resolver_handle_energy_zone_selection` (GLOBAL)
- [DEAD] src/ability/choice.c:3054 `rb_resolver_handle_entry_cost_reveal` (GLOBAL)
- [DEAD] src/ability/choice.c:4519 `rb_resolver_handle_position_destination` (GLOBAL)
- [DEAD] src/ability/choice.c:492 `rb_resolver_handle_select_cards_looked_at` (GLOBAL)
- [DEAD] src/ability/choice.c:319 `rb_resolver_look_select_finalize_dest` (GLOBAL)
- [DEAD] src/ability/choice.c:503 `rb_resolver_move_non_selected_hand_to_deck_bottom` (GLOBAL)
- [DEAD] src/ability/choice.c:955 `rb_resolver_provide_choice_result` (GLOBAL)
- [DEAD] src/ability/choice.c:466 `rb_resolver_reschedule_pending_choice` (GLOBAL)
- [DEAD] src/ability/choice.c:576 `rb_resolver_resume_execution_with_ctx` (GLOBAL)
- [DEAD] src/ability/choice.c:743 `rb_resolver_reveal_selected_looked_at` (GLOBAL)
- [DEAD] src/ability/choice.c:395 `rb_selection_context_mfi_count` (GLOBAL)

### `src/ability/choice_renderer.c` - 22 items - **FREE**

DEAD x22

- [DEAD] src/ability/choice_renderer.c:1689 `rb_rend_text_idle` (GLOBAL)
- [DEAD] src/ability/choice_renderer.c:1696 `rb_rend_text_n_presses` (GLOBAL)
- [DEAD] src/ability/choice_renderer.c:1514 `t_clear_screen` (static)
- [DEAD] src/ability/choice_renderer.c:1595 `t_draw_card_image_narrow` (static)
- [DEAD] src/ability/choice_renderer.c:1606 `t_open_start_menu_narrow` (static)
- [DEAD] src/ability/choice_renderer.c:1523 `t_poll_input` (static)
- [DEAD] src/ability/choice_renderer.c:1540 `t_pressed_a` (static)
- [DEAD] src/ability/choice_renderer.c:1541 `t_pressed_b` (static)
- [DEAD] src/ability/choice_renderer.c:1545 `t_pressed_down` (static)
- [DEAD] src/ability/choice_renderer.c:1542 `t_pressed_l` (static)
- [DEAD] src/ability/choice_renderer.c:1546 `t_pressed_left` (static)
- [DEAD] src/ability/choice_renderer.c:1543 `t_pressed_r` (static)
- [DEAD] src/ability/choice_renderer.c:1547 `t_pressed_right` (static)
- [DEAD] src/ability/choice_renderer.c:1549 `t_pressed_select` (static)
- [DEAD] src/ability/choice_renderer.c:1548 `t_pressed_start` (static)
- [DEAD] src/ability/choice_renderer.c:1544 `t_pressed_up` (static)
- [DEAD] src/ability/choice_renderer.c:1518 `t_println` (static)
- [DEAD] src/ability/choice_renderer.c:1535 `t_reset_vram` (static)
- [DEAD] src/ability/choice_renderer.c:1565 `t_show_board_overlay` (static)
- [DEAD] src/ability/choice_renderer.c:1598 `t_show_detail_screen_narrow` (static)
- [DEAD] src/ability/choice_renderer.c:1522 `t_swap_buffers` (static)
- [DEAD] src/ability/choice_renderer.c:1534 `t_wait_vblank` (static)

### `src/ability/compound.c` - 9 items - **FREE**

DUPLICATION x2, DEAD x7

- [DUPLICATION] cached_condition_verdict:242 / cond_cache_key:222 / store_condition_verdict:252 (duplicates of resolver.c)
- [DUPLICATION] rb_on_optional_branch:374 DEAD static
- [DEAD] src/ability/compound.c:767 `rb_compound_choice_action` (GLOBAL)
- [DEAD] src/ability/compound.c:762 `rb_compound_choice_string` (GLOBAL)
- [DEAD] src/ability/compound.c:777 `rb_compound_handle_choice_string_selection` (GLOBAL)
- [DEAD] src/ability/compound.c:811 `rb_compound_handle_choice_string_store` (GLOBAL)
- [DEAD] src/ability/compound.c:370 `rb_compound_route_branch` (GLOBAL)
- [DEAD] src/ability/compound.c:350 `rb_compound_save_remaining` (GLOBAL)
- [DEAD] src/ability/compound.c:374 `rb_on_optional_branch` (static)

### `src/ability/condition.c` - 3 items - **CONTESTED**

DUPLICATION x2, DEAD x1

- [DUPLICATION] card_appeared_this_turn:1775 has the guard misc.c:294 lacks
- [DUPLICATION] card_total_hearts:4011 is NOT a duplicate of util.c:705 - different Rust items
- [DEAD] src/ability/condition.c:2839 `rb_condition_allows` (GLOBAL)

### `src/ability/cost.c` - 11 items - **FREE**

DUPLICATION x4, DEAD x7

- [DUPLICATION] eff_extra:108
- [DUPLICATION] player_prefix:604 (different arity + zone set)
- [DUPLICATION] cost_count_in_source:2725 and cost_move_from_source:2736 DEAD statics
- [DUPLICATION] eff_int:119 DEAD static - note the name says int but the body returns a bool predicate
- [DEAD] src/ability/cost.c:2725 `cost_count_in_source` (static)
- [DEAD] src/ability/cost.c:2736 `cost_move_from_source` (static)
- [DEAD] src/ability/cost.c:119 `eff_int` (static)
- [DEAD] src/ability/cost.c:2407 `rb_cost_has_skip_prompt` (GLOBAL)
- [DEAD] src/ability/cost.c:2412 `rb_get_change_state_candidates` (GLOBAL)
- [DEAD] src/ability/cost.c:2429 `rb_pay_cost_change_state` (GLOBAL)
- [DEAD] src/ability/cost.c:2420 `rb_pay_cost_move_cards` (GLOBAL)

### `src/ability/debug.c` - 2 items - **FREE**

DEAD x2

- [DEAD] src/ability/debug.c:35 `rb_abdebug_dropped_count` (GLOBAL)
- [DEAD] src/ability/debug.c:36 `rb_abdebug_dropped_reset` (GLOBAL)

### `src/ability/describe.c` - 8 items - **FREE**

DUPLICATION x3, DEAD x5

- [DUPLICATION] d_extra:272 is a 6th-11th eff_extra port
- [DUPLICATION] d_zone_label:485 + rb_desc_zone_label:1832 duplicate look.c:104 / move.c:564 / triggers.c:1634
- [DUPLICATION] rb_desc_push_cond_verdict:2860 DEAD
- [DEAD] src/ability/describe.c:2897 `rb_desc_apply_bilingual` (GLOBAL)
- [DEAD] src/ability/describe.c:2835 `rb_desc_cond_actual` (GLOBAL)
- [DEAD] src/ability/describe.c:2831 `rb_desc_cond_expectation` (GLOBAL)
- [DEAD] src/ability/describe.c:2860 `rb_desc_push_cond_verdict` (GLOBAL)
- [DEAD] src/ability/describe.c:1813 `rb_desc_sequential_cost` (GLOBAL)

### `src/ability/effects/ability.c` - 6 items - **FREE**

DUPLICATION x1, DEAD x5

- [DUPLICATION] 5 dead globals
- [DEAD] src/ability/effects/ability.c:340 `rb_ability_ref_cache_size` (GLOBAL)
- [DEAD] src/ability/effects/ability.c:273 `rb_ability_ref_decode` (GLOBAL)
- [DEAD] src/ability/effects/ability.c:89 `rb_ability_ref_idx` (GLOBAL)
- [DEAD] src/ability/effects/ability.c:82 `rb_ability_ref_index` (GLOBAL)
- [DEAD] src/ability/effects/ability.c:291 `rb_ability_ref_to_arc` (GLOBAL)

### `src/ability/effects/ability_effects.c` - 1 items - **FREE**

DUPLICATION x1

- [DUPLICATION] effect_extra:7

### `src/ability/effects/draw.c` - 4 items - **FREE**

DEAD x4

- [DEAD] src/ability/effects/draw.c:406 `rb_draw_resolve_dynamic_count` (GLOBAL)
- [DEAD] src/ability/effects/draw.c:647 `rb_execute_draw_wrapper` (GLOBAL)
- [DEAD] src/ability/effects/draw.c:1060 `rb_make_card_effect_data` (GLOBAL)
- [DEAD] src/ability/effects/draw.c:1018 `rb_resolve_gain_heart_color` (GLOBAL)

### `src/ability/effects/executor.c` - 1 items - **FREE**

DUPLICATION x1

- [DUPLICATION] effect_extra:123 (3rd copy incl. game_setup.c:298)

### `src/ability/effects/look.c` - 17 items - **FREE**

DUPLICATION x5, DEAD x12

- [DUPLICATION] look_zone_label:104 and look_zone_label_ja:127 DEAD, duplicate describe.c:485
- [DUPLICATION] look_card_plural:97 DEAD, duplicate cost.c:404 / describe.c:468
- [DUPLICATION] card_is_live_pred:876 / card_type_pred:886 / card_type_cost_pred:894 DEAD
- [DUPLICATION] look_or_card_types:315 DEAD
- [DUPLICATION] look_player_prefix:190 4th player_prefix
- [DEAD] src/ability/effects/look.c:876 `card_is_live_pred` (static)
- [DEAD] src/ability/effects/look.c:894 `card_type_cost_pred` (static)
- [DEAD] src/ability/effects/look.c:886 `card_type_pred` (static)
- [DEAD] src/ability/effects/look.c:97 `look_card_plural` (static)
- [DEAD] src/ability/effects/look.c:315 `look_or_card_types` (static)
- [DEAD] src/ability/effects/look.c:171 `look_push_reveal_group_log` (static)
- [DEAD] src/ability/effects/look.c:161 `look_push_reveal_log` (static)
- [DEAD] src/ability/effects/look.c:180 `look_push_reveal_until_log` (static)
- [DEAD] src/ability/effects/look.c:227 `look_push_select_log` (static)
- [DEAD] src/ability/effects/look.c:104 `look_zone_label` (static)
- [DEAD] src/ability/effects/look.c:127 `look_zone_label_ja` (static)
- [DEAD] src/ability/effects/look.c:833 `rb_look_resume` (GLOBAL)

### `src/ability/effects/misc.c` - 6 items - **CONTESTED**

DUPLICATION x4, DEAD x2

- [DUPLICATION] eff_extra:52 has NO null guards (4 other copies have them)
- [DUPLICATION] card_appeared_this_turn:294 lacks the cid<0 guard condition.c:1775 has
- [DUPLICATION] extra_true:58
- [DUPLICATION] record_movement:169
- [DEAD] src/ability/effects/misc.c:1861 `rb_misc_finalize_formation_change` (GLOBAL)
- [DEAD] src/ability/effects/misc.c:1823 `rb_misc_position_destinations` (GLOBAL)

### `src/ability/effects/move.c` - 11 items - **FREE**

DUPLICATION x2, DEAD x9

- [DUPLICATION] cmf_extra:166 lacks the !e/!k null guards move_missing.c:432 has
- [DUPLICATION] cmf_extra_true:108 is a 3rd extra_true port
- [DEAD] src/ability/effects/move.c:134 `mc_set_recently_moved_batch` (static)
- [DEAD] src/ability/effects/move.c:2510 `rb_move_execute_move_cards` (GLOBAL)
- [DEAD] src/ability/effects/move.c:2224 `rb_move_from_revealed` (GLOBAL)
- [DEAD] src/ability/effects/move.c:2270 `rb_move_from_under_member` (GLOBAL)
- [DEAD] src/ability/effects/move.c:1674 `rb_move_resolve_from_revealed_cards` (GLOBAL)
- [DEAD] src/ability/effects/move.c:1763 `rb_move_resolve_from_selected_cards` (GLOBAL)
- [DEAD] src/ability/effects/move.c:1666 `rb_move_resolve_from_standard_zone` (GLOBAL)
- [DEAD] src/ability/effects/move.c:1725 `rb_move_resolve_from_those_cards` (GLOBAL)
- [DEAD] src/ability/effects/move.c:1780 `rb_move_resolve_source_revealed_cards` (GLOBAL)

### `src/ability/effects/move_missing.c` - 2 items - **FREE**

DUPLICATION x1, DEAD x1

- [DUPLICATION] extra_true:439 and cmf_extra:432 and cmf_extra_true
- [DEAD] src/ability/effects/move_missing.c:490 `rb_move_ask_optional_move_gate` (GLOBAL)

### `src/ability/effects/rules.c` - 50 items - **FREE**

DUPLICATION x2, DEAD x48

- [DUPLICATION] 4 DEAD restriction twins: rb_rules_execute_restriction:1819, rb_rules_execute_activation_restriction:1932, rb_rules_n_delayed_prohibitions:2257, rb_rules_delayed_prohibition:2262
- [DUPLICATION] cost_extra:1384 is another eff_extra port
- [DEAD] src/ability/effects/rules.c:2389 `rb_rules_apply_pending_replacement` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2251 `rb_rules_cannot_activate_player` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2245 `rb_rules_cannot_live_player` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2340 `rb_rules_choice_repeats_remaining` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2345 `rb_rules_consume_choice_repeat` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2371 `rb_rules_count_at_save` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2262 `rb_rules_delayed_prohibition` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1017 `rb_rules_execute_activation_cost` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1932 `rb_rules_execute_activation_restriction` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1651 `rb_rules_execute_choice` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:859 `rb_rules_execute_choose_required_hearts` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1541 `rb_rules_execute_choose_target_player` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:936 `rb_rules_execute_modify_cost` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1956 `rb_rules_execute_modify_limit` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:595 `rb_rules_execute_modify_required_hearts` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:727 `rb_rules_execute_modify_required_hearts_standard` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:762 `rb_rules_execute_modify_required_hearts_success` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2119 `rb_rules_execute_modify_yell_count` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2028 `rb_rules_execute_perform_yell` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2005 `rb_rules_execute_re_yell` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1944 `rb_rules_execute_reduce_live_card_set_limit` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1819 `rb_rules_execute_restriction` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1580 `rb_rules_execute_select_effect` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1458 `rb_rules_execute_select_number` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:919 `rb_rules_execute_set_cost` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1036 `rb_rules_execute_set_cost_to_use` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:805 `rb_rules_execute_specify_heart_color` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:140 `rb_rules_forget` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2333 `rb_rules_get_conditional_choice` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1304 `rb_rules_handle_both_targets` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1114 `rb_rules_implicit_heart_target` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2365 `rb_rules_keep_shuffle_count` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2362 `rb_rules_keep_shuffle_phase` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1639 `rb_rules_make_card_effect_data` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2257 `rb_rules_n_delayed_prohibitions` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2353 `rb_rules_note_keep_shuffle_selected` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2306 `rb_rules_note_moved_cards` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2315 `rb_rules_note_selected_cards` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2374 `rb_rules_owner_executing` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1051 `rb_rules_position_limited_targets` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2139 `rb_rules_register_replacement_effect` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:822 `rb_rules_resolve_gain_heart_color` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2377 `rb_rules_resolving_replacement` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2167 `rb_rules_run_replacement_effects` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2324 `rb_rules_set_conditional_choice` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2368 `rb_rules_sub_choice_created` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:2380 `rb_rules_take_pending_replacement` (GLOBAL)
- [DEAD] src/ability/effects/rules.c:1139 `rb_rules_try_create_target_selection_choice` (GLOBAL)

### `src/ability/effects/state.c` - 5 items - **FREE**

DUPLICATION x1, DEAD x4

- [DUPLICATION] rb_record_movement:1610 vs misc.c:169 / move.c:507
- [DEAD] src/ability/effects/state.c:1398 `rb_all_blade_timing_record` (GLOBAL)
- [DEAD] src/ability/effects/state.c:2072 `rb_card_need_heart_satisfied` (GLOBAL)
- [DEAD] src/ability/effects/state.c:2114 `rb_heart_color_from_index` (GLOBAL)
- [DEAD] src/ability/effects/state.c:2120 `rb_heart_color_short_label` (GLOBAL)

### `src/ability/log.c` - 4 items - **FREE**

DEAD x4

- [DEAD] src/ability/log.c:44 `rb_log_dropped_count` (GLOBAL)
- [DEAD] src/ability/log.c:45 `rb_log_dropped_reset` (GLOBAL)
- [DEAD] src/ability/log.c:198 `rb_log_push_verdict_cost` (GLOBAL)
- [DEAD] src/ability/log.c:249 `rb_log_push_verdict_item` (GLOBAL)

### `src/ability/resolver.c` - 13 items - **FREE**

DUPLICATION x3, DEAD x10

- [DUPLICATION] cached_condition_verdict:316 / cond_cache_key:221 / store_condition_verdict:326 (duplicates of compound.c)
- [DUPLICATION] rb_resolver_cached_condition_verdict:298 TEST-ONLY 3rd copy
- [DUPLICATION] player_prefix:888 arity differs from cost.c:604
- [DEAD] src/ability/resolver.c:682 `rb_resolver_card_matches_cost_limit` (GLOBAL)
- [DEAD] src/ability/resolver.c:679 `rb_resolver_card_matches_type` (GLOBAL)
- [DEAD] src/ability/resolver.c:380 `rb_resolver_emit_pay_skip_gate` (GLOBAL)
- [DEAD] src/ability/resolver.c:686 `rb_resolver_fmt_card` (GLOBAL)
- [DEAD] src/ability/resolver.c:695 `rb_resolver_fmt_ids` (GLOBAL)
- [DEAD] src/ability/resolver.c:279 `rb_resolver_get_pending_choice` (GLOBAL)
- [DEAD] src/ability/resolver.c:705 `rb_resolver_merge_group_names` (GLOBAL)
- [DEAD] src/ability/resolver.c:283 `rb_resolver_pending_choice` (GLOBAL)
- [DEAD] src/ability/resolver.c:847 `rb_resolver_trigger_infos` (GLOBAL)
- [DEAD] src/ability/resolver.c:253 `rb_resolver_zone_for_card` (GLOBAL)

### `src/ability/types.c` - 80 items - **FREE**

DUPLICATION x1, DEAD x79

- [DUPLICATION] 76 dead globals - the largest concentration in the tree
- [DEAD] src/ability/types.c:352 `rb_ability_error_format` (GLOBAL)
- [DEAD] src/ability/types.c:895 `rb_arcstr_deserialize` (GLOBAL)
- [DEAD] src/ability/types.c:884 `rb_arcstr_serialize` (GLOBAL)
- [DEAD] src/ability/types.c:844 `rb_choice_builder_blind` (GLOBAL)
- [DEAD] src/ability/types.c:764 `rb_choice_builder_card_type` (GLOBAL)
- [DEAD] src/ability/types.c:801 `rb_choice_builder_characters` (GLOBAL)
- [DEAD] src/ability/types.c:769 `rb_choice_builder_cost_limit` (GLOBAL)
- [DEAD] src/ability/types.c:778 `rb_choice_builder_cost_total` (GLOBAL)
- [DEAD] src/ability/types.c:787 `rb_choice_builder_cost_values` (GLOBAL)
- [DEAD] src/ability/types.c:859 `rb_choice_builder_description_en` (GLOBAL)
- [DEAD] src/ability/types.c:864 `rb_choice_builder_description_ja` (GLOBAL)
- [DEAD] src/ability/types.c:824 `rb_choice_builder_destination` (GLOBAL)
- [DEAD] src/ability/types.c:829 `rb_choice_builder_discard_remaining` (GLOBAL)
- [DEAD] src/ability/types.c:869 `rb_choice_builder_filtered_indices` (GLOBAL)
- [DEAD] src/ability/types.c:760 `rb_choice_builder_free` (GLOBAL)
- [DEAD] src/ability/types.c:796 `rb_choice_builder_group` (GLOBAL)
- [DEAD] src/ability/types.c:809 `rb_choice_builder_heart_colors` (GLOBAL)
- [DEAD] src/ability/types.c:849 `rb_choice_builder_is_reveal` (GLOBAL)
- [DEAD] src/ability/types.c:834 `rb_choice_builder_is_select_action` (GLOBAL)
- [DEAD] src/ability/types.c:819 `rb_choice_builder_name_fragments` (GLOBAL)
- [DEAD] src/ability/types.c:814 `rb_choice_builder_require_all_heart_colors` (GLOBAL)
- [DEAD] src/ability/types.c:839 `rb_choice_builder_target_player_id` (GLOBAL)
- [DEAD] src/ability/types.c:618 `rb_choice_result_free` (GLOBAL)
- [DEAD] src/ability/types.c:587 `rb_choice_result_is_skip` (GLOBAL)
- [DEAD] src/ability/types.c:583 `rb_choice_result_kind` (GLOBAL)
- [DEAD] src/ability/types.c:605 `rb_choice_result_kind_from_str` (GLOBAL)
- [DEAD] src/ability/types.c:591 `rb_choice_result_kind_to_str` (GLOBAL)
- [DEAD] src/ability/types.c:556 `rb_choice_result_new_card_selected` (GLOBAL)
- [DEAD] src/ability/types.c:566 `rb_choice_result_new_skip` (GLOBAL)
- [DEAD] src/ability/types.c:574 `rb_choice_result_new_target` (GLOBAL)
- [DEAD] src/ability/types.c:654 `rb_choice_route_from_kind` (GLOBAL)
- [DEAD] src/ability/types.c:666 `rb_choice_route_kind_from_header` (GLOBAL)
- [DEAD] src/ability/types.c:648 `rb_choice_route_new` (GLOBAL)
- [DEAD] src/ability/types.c:626 `rb_choice_route_to_str` (GLOBAL)
- [DEAD] src/ability/types.c:274 `rb_effect_pipeline_free` (GLOBAL)
- [DEAD] src/ability/types.c:267 `rb_effect_pipeline_new` (GLOBAL)
- [DEAD] src/ability/types.c:732 `rb_effect_spawn_context_init` (GLOBAL)
- [DEAD] src/ability/types.c:393 `rb_exec_context_kind_from_str` (GLOBAL)
- [DEAD] src/ability/types.c:383 `rb_exec_context_kind_to_str` (GLOBAL)
- [DEAD] src/ability/types.c:511 `rb_full_choice_allow_skip` (GLOBAL)
- [DEAD] src/ability/types.c:507 `rb_full_choice_description_ja` (GLOBAL)
- [DEAD] src/ability/types.c:550 `rb_full_choice_free` (GLOBAL)
- [DEAD] src/ability/types.c:421 `rb_full_choice_new_select_card` (GLOBAL)
- [DEAD] src/ability/types.c:458 `rb_full_choice_new_select_heart_color` (GLOBAL)
- [DEAD] src/ability/types.c:446 `rb_full_choice_new_select_position` (GLOBAL)
- [DEAD] src/ability/types.c:499 `rb_full_choice_set_aa_options` (GLOBAL)
- [DEAD] src/ability/types.c:503 `rb_full_choice_set_ls_options` (GLOBAL)
- [DEAD] src/ability/types.c:515 `rb_full_choice_to_header` (GLOBAL)
- [DEAD] src/ability/types.c:543 `rb_full_choice_to_json` (GLOBAL)
- [DEAD] src/ability/types.c:411 `rb_las_kind_from_str` (GLOBAL)
- [DEAD] src/ability/types.c:402 `rb_las_kind_to_str` (GLOBAL)
- [DEAD] src/ability/types.c:927 `rb_look_and_select_step_finalize` (GLOBAL)
- [DEAD] src/ability/types.c:906 `rb_look_and_select_step_look` (GLOBAL)
- [DEAD] src/ability/types.c:915 `rb_look_and_select_step_select` (GLOBAL)
- [DEAD] src/ability/types.c:680 `rb_repeat_prompt_choice` (GLOBAL)
- [DEAD] src/ability/types.c:53 `rb_step_output_accepted` (GLOBAL)
- [DEAD] src/ability/types.c:17 `rb_step_output_from_value` (GLOBAL)
- [DEAD] src/ability/types.c:45 `rb_step_output_has_cards` (GLOBAL)
- [DEAD] src/ability/types.c:13 `rb_step_output_init` (GLOBAL)
- [DEAD] src/ability/types.c:77 `rb_step_output_to_json` (GLOBAL)
- [DEAD] src/ability/types.c:49 `rb_step_output_value` (GLOBAL)
- [DEAD] src/ability/types.c:326 `rb_step_state_free` (GLOBAL)
- [DEAD] src/ability/types.c:311 `rb_step_state_get` (GLOBAL)
- [DEAD] src/ability/types.c:286 `rb_step_state_new` (GLOBAL)
- [DEAD] src/ability/types.c:341 `rb_step_state_record_cards` (GLOBAL)
- [DEAD] src/ability/types.c:332 `rb_step_state_record_value` (GLOBAL)
- [DEAD] src/ability/types.c:251 `rb_trace_node_add_child` (GLOBAL)
- [DEAD] src/ability/types.c:243 `rb_trace_node_from_game_state` (GLOBAL)
- [DEAD] src/ability/types.c:236 `rb_trace_node_with_after` (GLOBAL)
- [DEAD] src/ability/types.c:217 `rb_trace_node_with_card` (GLOBAL)
- [DEAD] src/ability/types.c:700 `rb_trigger_event_add_appeared` (GLOBAL)
- [DEAD] src/ability/types.c:695 `rb_trigger_event_add_moved` (GLOBAL)
- [DEAD] src/ability/types.c:726 `rb_trigger_event_copy` (GLOBAL)
- [DEAD] src/ability/types.c:714 `rb_trigger_event_has_appeared` (GLOBAL)
- [DEAD] src/ability/types.c:722 `rb_trigger_event_has_energy_placed` (GLOBAL)
- [DEAD] src/ability/types.c:710 `rb_trigger_event_has_moved` (GLOBAL)
- [DEAD] src/ability/types.c:718 `rb_trigger_event_has_position_change` (GLOBAL)
- [DEAD] src/ability/types.c:691 `rb_trigger_event_init` (GLOBAL)
- [DEAD] src/ability/types.c:170 `rb_value_ref_merge` (GLOBAL)

### `src/ability/util.c` - 31 items - **FREE**

DUPLICATION x4, DEAD x27

- [DUPLICATION] eff_extra:258 has no !k guard
- [DUPLICATION] zone_bag:1053 zone-alias table differs from test_game.c:723
- [DUPLICATION] rb_max_distinct_names_greedy:2895 DEAD-WRONG - drops Rust's `collision` output
- [DUPLICATION] rb_debug_group_match:382 DEAD-WRONG - empty body against a real Rust function
- [DEAD] src/ability/util.c:1790 `card_filter_matches_card` (static)
- [DEAD] src/ability/util.c:1785 `card_filter_matches_skip` (static)
- [DEAD] src/ability/util.c:2907 `cmp_u64` (static)
- [DEAD] src/ability/util.c:1769 `local_has_filter` (static)
- [DEAD] src/ability/util.c:2153 `rb_build_candidate_pool` (GLOBAL)
- [DEAD] src/ability/util.c:2767 `rb_calculate_per_unit_multiplier` (GLOBAL)
- [DEAD] src/ability/util.c:682 `rb_card_matches_cost_limit_op` (GLOBAL)
- [DEAD] src/ability/util.c:992 `rb_count_in_zone_filtered` (GLOBAL)
- [DEAD] src/ability/util.c:2130 `rb_count_matching_distinct` (GLOBAL)
- [DEAD] src/ability/util.c:382 `rb_debug_group_match` (static)
- [DEAD] src/ability/util.c:3140 `rb_exclude_self` (GLOBAL)
- [DEAD] src/ability/util.c:2081 `rb_filter_distinct` (GLOBAL)
- [DEAD] src/ability/util.c:3104 `rb_filter_from_parts` (GLOBAL)
- [DEAD] src/ability/util.c:3117 `rb_filter_from_parts_full` (GLOBAL)
- [DEAD] src/ability/util.c:2241 `rb_get_selection_indices` (GLOBAL)
- [DEAD] src/ability/util.c:3134 `rb_has_cannot_baton_touch_protection_util` (GLOBAL)
- [DEAD] src/ability/util.c:2169 `rb_matching_ids_filtered` (GLOBAL)
- [DEAD] src/ability/util.c:1945 `rb_matching_indices` (GLOBAL)
- [DEAD] src/ability/util.c:2895 `rb_max_distinct_names_greedy` (static)
- [DEAD] src/ability/util.c:1334 `rb_move_cards` (GLOBAL)
- [DEAD] src/ability/util.c:171 `rb_norm_group_name` (static)
- [DEAD] src/ability/util.c:2911 `rb_prune_dominated` (GLOBAL)
- [DEAD] src/ability/util.c:2996 `rb_push_temporary_effect` (GLOBAL)
- [DEAD] src/ability/util.c:2270 `rb_resolve_selection` (GLOBAL)
- [DEAD] src/ability/util.c:3065 `rb_set_card_identity` (GLOBAL)
- [DEAD] src/ability/util.c:846 `rb_target_player_label` (GLOBAL)
- [DEAD] src/ability/util.c:955 `rb_zone_card_ids` (GLOBAL)

### `src/ability/vm.c` - 7 items - **FREE**

DEAD x7

- [DEAD] src/ability/vm.c:2794 `build_filter` (GLOBAL)
- [DEAD] src/ability/vm.c:605 `effect_decode_extra` (static)
- [DEAD] src/ability/vm.c:2810 `offset_of` (GLOBAL)
- [DEAD] src/ability/vm.c:2845 `populate_from_json` (GLOBAL)
- [DEAD] src/ability/vm.c:2863 `rb_zone_as_str` (GLOBAL)
- [DEAD] src/ability/vm.c:2822 `rd_i64` (GLOBAL)
- [DEAD] src/ability/vm.c:2835 `rd_key` (GLOBAL)

### `src/choice_renderer.c` - 1 items - **FREE**

DUPLICATION x1

- [DUPLICATION] 20 dead statics

### `src/core/alloc.c` - 3 items - **FREE**

DEAD x3

- [DEAD] src/core/alloc.c:20 `rb_alloc_reset` (GLOBAL)
- [DEAD] src/core/alloc.c:98 `rb_drop_value` (GLOBAL)
- [DEAD] src/core/alloc.c:92 `rb_put` (GLOBAL)

### `src/core/alloc_counter.c` - 22 items - **FREE**

DUPLICATION x1, DEAD x21

- [DUPLICATION] rb_allocctr_free:680 DEAD; 17 functions in this file are dead globals
- [DEAD] src/core/alloc_counter.c:267 `rb_allocctr_alloc_buckets` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:715 `rb_allocctr_alloc_count` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:719 `rb_allocctr_dealloc_count` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:765 `rb_allocctr_flag_action_display_enabled` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:773 `rb_allocctr_flag_logging_enabled` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:777 `rb_allocctr_flag_set_training_mode` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:680 `rb_allocctr_free` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:652 `rb_allocctr_guard_release` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:748 `rb_allocctr_ledger_overflow_count` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:723 `rb_allocctr_live_bytes` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:481 `rb_allocctr_maybe_arm` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:302 `rb_allocctr_note_alloc` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:306 `rb_allocctr_note_alloc_at` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:738 `rb_allocctr_outstanding_allocs` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:727 `rb_allocctr_peak_bytes` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:640 `rb_allocctr_start` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:705 `rb_allocctr_strdup2` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:731 `rb_allocctr_total_allocated` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:394 `rb_allocctr_trace_armed` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:454 `rb_allocctr_trace_guard_release` (GLOBAL)
- [DEAD] src/core/alloc_counter.c:744 `rb_allocctr_unmatched_free_count` (GLOBAL)

### `src/core/card.c` - 17 items - **FREE**

DEAD x17

- [DEAD] src/core/card.c:229 `pair_order_cmp` (static)
- [DEAD] src/core/card.c:534 `rb_ability_filter_from_str` (GLOBAL)
- [DEAD] src/core/card.c:524 `rb_ability_filter_str` (GLOBAL)
- [DEAD] src/core/card.c:484 `rb_card_property_from_str` (GLOBAL)
- [DEAD] src/core/card.c:475 `rb_card_property_str` (GLOBAL)
- [DEAD] src/core/card.c:457 `rb_card_state_from_str` (GLOBAL)
- [DEAD] src/core/card.c:449 `rb_card_state_str` (GLOBAL)
- [DEAD] src/core/card.c:470 `rb_comparison_target_from_str` (GLOBAL)
- [DEAD] src/core/card.c:462 `rb_comparison_target_str` (GLOBAL)
- [DEAD] src/core/card.c:515 `rb_comparison_type_from_str` (GLOBAL)
- [DEAD] src/core/card.c:504 `rb_comparison_type_str` (GLOBAL)
- [DEAD] src/core/card.c:552 `rb_condition_card_type_str` (GLOBAL)
- [DEAD] src/core/card.c:542 `rb_condition_target_str` (GLOBAL)
- [DEAD] src/core/card.c:495 `rb_distinct_type_str` (GLOBAL)
- [DEAD] src/core/card.c:567 `rb_location_str` (GLOBAL)
- [DEAD] src/core/card.c:429 `rb_parse_operation` (GLOBAL)
- [DEAD] src/core/card.c:491 `rb_placement_order_str` (GLOBAL)

### `src/core/data.c` - 15 items - **FREE**

DUPLICATION x1, DEAD x14

- [DUPLICATION] 12 dead globals
- [DEAD] src/core/data.c:1095 `rb_effect_data_amount` (GLOBAL)
- [DEAD] src/core/data.c:765 `rb_effect_data_card_id` (GLOBAL)
- [DEAD] src/core/data.c:667 `rb_effect_data_clone` (GLOBAL)
- [DEAD] src/core/data.c:837 `rb_effect_data_color` (GLOBAL)
- [DEAD] src/core/data.c:831 `rb_effect_data_count` (GLOBAL)
- [DEAD] src/core/data.c:819 `rb_effect_data_is_p1` (GLOBAL)
- [DEAD] src/core/data.c:785 `rb_effect_data_items` (GLOBAL)
- [DEAD] src/core/data.c:825 `rb_effect_data_old_value` (GLOBAL)
- [DEAD] src/core/data.c:871 `rb_effect_type_as_str` (GLOBAL)
- [DEAD] src/core/data.c:219 `rb_load_streaming` (GLOBAL)
- [DEAD] src/core/data.c:745 `rb_phase_display` (GLOBAL)
- [DEAD] src/core/data.c:851 `rb_phase_label_jp` (GLOBAL)
- [DEAD] src/core/data.c:717 `rb_turn_phase_display` (GLOBAL)
- [DEAD] src/core/data.c:1068 `rb_zone_matches_dest` (GLOBAL)

### `src/core/game_state_abilities.c` - 36 items - **CONTESTED**

DUPLICATION x2, DEAD x34

- [DUPLICATION] rb_claim_batch_slot:896 DEAD-WRONG (capacity truncation Rust does not have)
- [DUPLICATION] rb_set_just_completed:840 / rb_clear_just_completed:863 / rb_just_completed_batch_matches:870 all DEAD
- [DEAD] src/core/game_state_abilities.c:3399 `rb_apply_ability_effects` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3545 `rb_build_ability_queue_entry_public_full` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:950 `rb_claim_batch_slot` (static)
- [DEAD] src/core/game_state_abilities.c:917 `rb_clear_just_completed` (static)
- [DEAD] src/core/game_state_abilities.c:3423 `rb_collect_constant_hand` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:471 `rb_collect_constant_hand_effect_ids` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:485 `rb_collect_constant_stage_effect_ids` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3445 `rb_collect_live_modifiers` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1354 `rb_effective_activation_cost` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1608 `rb_entry_ability_copy` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3533 `rb_entry_choice_card_no` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3538 `rb_entry_conditional_choice` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1617 `rb_entry_destination_copy` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1516 `rb_entry_has_pending_choice` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1701 `rb_entry_snapshot_last_area_move_by_player` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1696 `rb_entry_snapshot_last_area_move_card_id` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1686 `rb_entry_snapshot_last_energy_placed_by_effect` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1691 `rb_entry_snapshot_last_energy_placed_by_player` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:284 `rb_expire_gained_abilities` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:2541 `rb_fire_all_auto` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:2548 `rb_fire_auto_and_pending` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:2570 `rb_fire_recorded_auto` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3507 `rb_get_pending_choice_json` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:1487 `rb_inject_choice_ability_context` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:775 `rb_is_stale_discard_condition` (static)
- [DEAD] src/core/game_state_abilities.c:924 `rb_just_completed_batch_matches` (static)
- [DEAD] src/core/game_state_abilities.c:1453 `rb_mark_replacement_effect_applied` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3362 `rb_process_player_abilities_depth` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:3606 `rb_process_player_abilities_depth_full` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:592 `rb_process_with_completed_key` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:2564 `rb_record_event` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:850 `rb_requires_this_move` (static)
- [DEAD] src/core/game_state_abilities.c:495 `rb_resolve_constant_ability` (GLOBAL)
- [DEAD] src/core/game_state_abilities.c:894 `rb_set_just_completed` (static)

### `src/core/game_state_core.c` - 42 items - **FREE**

FALSE-COMMENT x3, DEAD x39

- [FALSE-COMMENT] line 13: banner says cards_paid_cost_this_turn / player1-2_cheer_revealed_cards / revealed_cost_cards have "no GameState field"; all four ARE declared (rabuka.h:1490, 1518-1523, 1527)
- [FALSE-COMMENT] line 14: same banner, line 14
- [FALSE-COMMENT] line 15: same banner, line 15
- [DEAD] src/core/game_state_core.c:185 `rb_gsc_active_player` (GLOBAL)
- [DEAD] src/core/game_state_core.c:246 `rb_gsc_can_player_act` (GLOBAL)
- [DEAD] src/core/game_state_core.c:419 `rb_gsc_cheer_count_by_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:446 `rb_gsc_clear_effect_tracking` (GLOBAL)
- [DEAD] src/core/game_state_core.c:376 `rb_gsc_concede_both` (GLOBAL)
- [DEAD] src/core/game_state_core.c:389 `rb_gsc_const_active_player` (GLOBAL)
- [DEAD] src/core/game_state_core.c:497 `rb_gsc_current_ability_source_card_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:154 `rb_gsc_first_attacker_index` (GLOBAL)
- [DEAD] src/core/game_state_core.c:191 `rb_gsc_is_first_turn` (GLOBAL)
- [DEAD] src/core/game_state_core.c:462 `rb_gsc_last_area_move_by_player` (GLOBAL)
- [DEAD] src/core/game_state_core.c:459 `rb_gsc_last_area_move_card_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:468 `rb_gsc_last_cost_wait_member` (GLOBAL)
- [DEAD] src/core/game_state_core.c:488 `rb_gsc_last_energy_placed_by_effect` (GLOBAL)
- [DEAD] src/core/game_state_core.c:491 `rb_gsc_last_energy_placed_by_player` (GLOBAL)
- [DEAD] src/core/game_state_core.c:821 `rb_gsc_log_entry` (GLOBAL)
- [DEAD] src/core/game_state_core.c:347 `rb_gsc_mulligan_owner_index` (GLOBAL)
- [DEAD] src/core/game_state_core.c:300 `rb_gsc_opponent_has_performed` (GLOBAL)
- [DEAD] src/core/game_state_core.c:219 `rb_gsc_phase_invariant` (GLOBAL)
- [DEAD] src/core/game_state_core.c:518 `rb_gsc_player_prefix_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:837 `rb_gsc_push_choice_offered` (GLOBAL)
- [DEAD] src/core/game_state_core.c:860 `rb_gsc_push_choice_resolved` (GLOBAL)
- [DEAD] src/core/game_state_core.c:874 `rb_gsc_push_performance_snapshot` (GLOBAL)
- [DEAD] src/core/game_state_core.c:510 `rb_gsc_push_revealed_card` (GLOBAL)
- [DEAD] src/core/game_state_core.c:802 `rb_gsc_push_rule_log_fmt` (GLOBAL)
- [DEAD] src/core/game_state_core.c:474 `rb_gsc_recently_moved_cards` (GLOBAL)
- [DEAD] src/core/game_state_core.c:883 `rb_gsc_record_ability_application` (GLOBAL)
- [DEAD] src/core/game_state_core.c:433 `rb_gsc_score_bonus_by_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:427 `rb_gsc_seat_cheer_count` (GLOBAL)
- [DEAD] src/core/game_state_core.c:415 `rb_gsc_seat_index_by_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:388 `rb_gsc_seat_player` (GLOBAL)
- [DEAD] src/core/game_state_core.c:387 `rb_gsc_seat_player_mut` (GLOBAL)
- [DEAD] src/core/game_state_core.c:437 `rb_gsc_seat_score_bonus` (GLOBAL)
- [DEAD] src/core/game_state_core.c:394 `rb_gsc_seated_pair` (GLOBAL)
- [DEAD] src/core/game_state_core.c:155 `rb_gsc_second_attacker_index` (GLOBAL)
- [DEAD] src/core/game_state_core.c:423 `rb_gsc_set_cheer_count_by_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:480 `rb_gsc_set_recently_moved_cards` (GLOBAL)
- [DEAD] src/core/game_state_core.c:410 `rb_gsc_try_player_by_id` (GLOBAL)
- [DEAD] src/core/game_state_core.c:409 `rb_gsc_try_player_by_id_mut` (GLOBAL)
- [DEAD] src/core/game_state_core.c:535 `rb_gsc_zone_snapshot` (GLOBAL)

### `src/core/modifiers.c` - 12 items - **FREE**

FALSE-COMMENT x1, DUPLICATION x1, DEAD x10

- [FALSE-COMMENT] line 904: ":2018 mods.opp_cause_fired_keys -> LANDED, field at rabuka.h:270-271" - field is at rabuka.h:282 and is 0W/0R
- [DUPLICATION] cost_group_name_first:1428 vs cost.c:367 / util.c:608
- [DEAD] src/core/modifiers.c:1341 `rb_clear_revealed_cards` (GLOBAL)
- [DEAD] src/core/modifiers.c:164 `rb_mods_clear_blade_set` (GLOBAL)
- [DEAD] src/core/modifiers.c:290 `rb_mods_clear_blade_type` (GLOBAL)
- [DEAD] src/core/modifiers.c:172 `rb_mods_clear_score_set` (GLOBAL)
- [DEAD] src/core/modifiers.c:187 `rb_mods_remove_blade` (GLOBAL)
- [DEAD] src/core/modifiers.c:192 `rb_mods_remove_heart` (GLOBAL)
- [DEAD] src/core/modifiers.c:263 `rb_mods_remove_heart_override` (GLOBAL)
- [DEAD] src/core/modifiers.c:223 `rb_mods_remove_score` (GLOBAL)
- [DEAD] src/core/modifiers.c:968 `rb_remove_revealed_card` (GLOBAL)
- [DEAD] src/core/modifiers.c:630 `rb_zone_id_to_movement_zone` (GLOBAL)

### `src/core/tracking.c` - 1 items - **FREE**

DEAD x1

- [DEAD] src/core/tracking.c:163 `rb_check_required_hearts` (GLOBAL)

### `src/core/zones.c` - 28 items - **FREE**

DEAD x28

- [DEAD] src/core/zones.c:819 `rb_deck_is_empty` (GLOBAL)
- [DEAD] src/core/zones.c:825 `rb_deck_len` (GLOBAL)
- [DEAD] src/core/zones.c:813 `rb_deck_shuffle` (GLOBAL)
- [DEAD] src/core/zones.c:576 `rb_energy_add_active` (GLOBAL)
- [DEAD] src/core/zones.c:536 `rb_energy_can_place_card` (GLOBAL)
- [DEAD] src/core/zones.c:650 `rb_hand_is_empty` (GLOBAL)
- [DEAD] src/core/zones.c:646 `rb_hand_len` (GLOBAL)
- [DEAD] src/core/zones.c:619 `rb_live_clear` (GLOBAL)
- [DEAD] src/core/zones.c:1102 `rb_player_activate_all_energy` (GLOBAL)
- [DEAD] src/core/zones.c:1110 `rb_player_activate_all_energy_exclude` (GLOBAL)
- [DEAD] src/core/zones.c:1120 `rb_player_all_card_ids` (GLOBAL)
- [DEAD] src/core/zones.c:1096 `rb_player_calculate_stage_hearts` (GLOBAL)
- [DEAD] src/core/zones.c:1180 `rb_player_contains_card` (GLOBAL)
- [DEAD] src/core/zones.c:1166 `rb_player_draw_energy` (GLOBAL)
- [DEAD] src/core/zones.c:989 `rb_player_get_card_index_by_id` (GLOBAL)
- [DEAD] src/core/zones.c:1000 `rb_player_is_area_locked` (GLOBAL)
- [DEAD] src/core/zones.c:1037 `rb_player_move_card_from_hand_to_stage` (GLOBAL)
- [DEAD] src/core/zones.c:968 `rb_player_set_main_deck` (GLOBAL)
- [DEAD] src/core/zones.c:789 `rb_resolution_swap_slots` (GLOBAL)
- [DEAD] src/core/zones.c:919 `rb_stage_get_available_hearts` (GLOBAL)
- [DEAD] src/core/zones.c:928 `rb_stage_get_available_hearts_i32` (GLOBAL)
- [DEAD] src/core/zones.c:702 `rb_success_len` (GLOBAL)
- [DEAD] src/core/zones.c:674 `rb_waitroom_len` (GLOBAL)
- [DEAD] src/core/zones.c:669 `rb_waitroom_shuffle` (GLOBAL)
- [DEAD] src/core/zones.c:661 `rb_waitroom_take_all` (GLOBAL)
- [DEAD] src/core/zones.c:79 `rb_zone_draw_bottom` (GLOBAL)
- [DEAD] src/core/zones.c:85 `rb_zone_draw_multiple` (GLOBAL)
- [DEAD] src/core/zones.c:51 `rb_zone_total_blades` (GLOBAL)

### `src/engine.c` - 4 items - **FREE**

DUPLICATION x1, DEAD x3

- [DUPLICATION] extra:218 is another eff_extra port
- [DEAD] src/engine.c:223 `heart_color_of` (GLOBAL)
- [DEAD] src/engine.c:1867 `i18n_self_check` (GLOBAL)
- [DEAD] src/engine.c:1460 `rb_play_card` (GLOBAL)

### `src/game/game_setup.c` - 10 items - **FREE**

DUPLICATION x2, DEAD x8

- [DUPLICATION] setup_effect_extra:298 is a 3rd effect_extra port
- [DUPLICATION] zone_bag:723 is a DIVERGENT twin of ability/util.c:1053
- [DEAD] src/game/game_setup.c:413 `rb_setup_auto_advance_one` (GLOBAL)
- [DEAD] src/game/game_setup.c:1401 `rb_setup_build_two_decks` (GLOBAL)
- [DEAD] src/game/game_setup.c:882 `rb_setup_execute_action_for_player` (GLOBAL)
- [DEAD] src/game/game_setup.c:1523 `rb_setup_extract_base` (GLOBAL)
- [DEAD] src/game/game_setup.c:1567 `rb_setup_flatten_and_sort_for_gba` (GLOBAL)
- [DEAD] src/game/game_setup.c:406 `rb_setup_is_live_card_set_phase` (GLOBAL)
- [DEAD] src/game/game_setup.c:449 `rb_setup_settle_auto` (GLOBAL)
- [DEAD] src/game/game_setup.c:1548 `setup_gba_cmp` (static)

### `src/test_game.c` - 7 items - **CONTESTED**

DEAD x7

- [DEAD] src/test_game.c:1151 `test_advance_to_phase_rust` (GLOBAL)
- [DEAD] src/test_game.c:381 `test_expire_effects` (GLOBAL)
- [DEAD] src/test_game.c:204 `test_find_live_by_score` (GLOBAL)
- [DEAD] src/test_game.c:1582 `test_flat_failed_count` (GLOBAL)
- [DEAD] src/test_game.c:1567 `test_flat_verdict_failed` (GLOBAL)
- [DEAD] src/test_game.c:586 `test_success_count` (GLOBAL)
- [DEAD] src/test_game.c:824 `test_zone_has_card_no` (GLOBAL)

### `src/turn/live.c` - 4 items - **FREE**

DEAD x4

- [DEAD] src/turn/live.c:2585 `rb_bt_search` (GLOBAL)
- [DEAD] src/turn/live.c:2673 `rb_try_all_distribution` (GLOBAL)
- [DEAD] src/turn/live.c:2614 `rb_try_phase4` (GLOBAL)
- [DEAD] src/turn/live.c:2541 `rb_try_surplus_compositions` (GLOBAL)

### `src/turn/phase.c` - 8 items - **FREE**

DEAD x8

- [DEAD] src/turn/phase.c:1322 `_3ds_tdbg` (GLOBAL)
- [DEAD] src/turn/phase.c:1615 `rb_3ds_tdbg` (GLOBAL)
- [DEAD] src/turn/phase.c:1859 `rb_ability_offer_sink` (static)
- [DEAD] src/turn/phase.c:2135 `rb_generate_possible_actions` (GLOBAL)
- [DEAD] src/turn/phase.c:1108 `rb_has_distinct_assignment_k` (GLOBAL)
- [DEAD] src/turn/phase.c:1315 `rb_log_phase` (GLOBAL)
- [DEAD] src/turn/phase.c:1329 `rb_log_turn_start` (GLOBAL)
- [DEAD] src/turn/phase.c:1747 `rb_play_time_cost_reduction_hook` (GLOBAL)

### `src/turn/triggers.c` - 1 items - **FREE**

DEAD x1

- [DEAD] src/turn/triggers.c:1641 `rb_trigger_to_texticon` (GLOBAL)

## Condensed: file -> n items -> FREE/CONTESTED

| file | items | status | top item |
|---|---|---|---|
| `include/rabuka.h` | 17 | **CONTESTED** | rb_activate_ability_effect is declared here and defined nowhere (include/rabuka.h:2247). A |
| `src/ability/abilities_gen.c` | 1 | FREE | src/ability/abilities_gen.c:984 `bytecode_slice` (GLOBAL) |
| `src/ability/ability_queue.c` | 2 | FREE | src/ability/ability_queue.c:307 `rb_queue_push_with_trigger` (GLOBAL) |
| `src/ability/choice.c` | 32 | **CONTESTED** | src/ability/choice.c:5691 `rb_ability_error_to_string` (GLOBAL) |
| `src/ability/choice_renderer.c` | 22 | FREE | src/ability/choice_renderer.c:1689 `rb_rend_text_idle` (GLOBAL) |
| `src/ability/compound.c` | 9 | FREE | cached_condition_verdict:242 / cond_cache_key:222 / store_condition_verdict:252 (duplicate |
| `src/ability/condition.c` | 3 | **CONTESTED** | card_appeared_this_turn:1775 has the guard misc.c:294 lacks |
| `src/ability/cost.c` | 11 | FREE | eff_extra:108 |
| `src/ability/debug.c` | 2 | FREE | src/ability/debug.c:35 `rb_abdebug_dropped_count` (GLOBAL) |
| `src/ability/describe.c` | 8 | FREE | d_extra:272 is a 6th-11th eff_extra port |
| `src/ability/effects/ability.c` | 6 | FREE | 5 dead globals |
| `src/ability/effects/ability_effects.c` | 1 | FREE | effect_extra:7 |
| `src/ability/effects/draw.c` | 4 | FREE | src/ability/effects/draw.c:406 `rb_draw_resolve_dynamic_count` (GLOBAL) |
| `src/ability/effects/executor.c` | 1 | FREE | effect_extra:123 (3rd copy incl. game_setup.c:298) |
| `src/ability/effects/look.c` | 17 | FREE | look_zone_label:104 and look_zone_label_ja:127 DEAD, duplicate describe.c:485 |
| `src/ability/effects/misc.c` | 6 | **CONTESTED** | eff_extra:52 has NO null guards (4 other copies have them) |
| `src/ability/effects/move.c` | 11 | FREE | cmf_extra:166 lacks the !e/!k null guards move_missing.c:432 has |
| `src/ability/effects/move_missing.c` | 2 | FREE | extra_true:439 and cmf_extra:432 and cmf_extra_true |
| `src/ability/effects/rules.c` | 50 | FREE | 4 DEAD restriction twins: rb_rules_execute_restriction:1819, rb_rules_execute_activation_r |
| `src/ability/effects/state.c` | 5 | FREE | rb_record_movement:1610 vs misc.c:169 / move.c:507 |
| `src/ability/log.c` | 4 | FREE | src/ability/log.c:44 `rb_log_dropped_count` (GLOBAL) |
| `src/ability/resolver.c` | 13 | FREE | cached_condition_verdict:316 / cond_cache_key:221 / store_condition_verdict:326 (duplicate |
| `src/ability/types.c` | 80 | FREE | 76 dead globals - the largest concentration in the tree |
| `src/ability/util.c` | 31 | FREE | eff_extra:258 has no !k guard |
| `src/ability/vm.c` | 7 | FREE | src/ability/vm.c:2794 `build_filter` (GLOBAL) |
| `src/choice_renderer.c` | 1 | FREE | 20 dead statics |
| `src/core/alloc.c` | 3 | FREE | src/core/alloc.c:20 `rb_alloc_reset` (GLOBAL) |
| `src/core/alloc_counter.c` | 22 | FREE | rb_allocctr_free:680 DEAD; 17 functions in this file are dead globals |
| `src/core/card.c` | 17 | FREE | src/core/card.c:229 `pair_order_cmp` (static) |
| `src/core/data.c` | 15 | FREE | 12 dead globals |
| `src/core/game_state_abilities.c` | 36 | **CONTESTED** | rb_claim_batch_slot:896 DEAD-WRONG (capacity truncation Rust does not have) |
| `src/core/game_state_core.c` | 42 | FREE | line 13: banner says cards_paid_cost_this_turn / player1-2_cheer_revealed_cards / revealed |
| `src/core/modifiers.c` | 12 | FREE | line 904: ":2018 mods.opp_cause_fired_keys -> LANDED, field at rabuka.h:270-271" - field i |
| `src/core/tracking.c` | 1 | FREE | src/core/tracking.c:163 `rb_check_required_hearts` (GLOBAL) |
| `src/core/zones.c` | 28 | FREE | src/core/zones.c:819 `rb_deck_is_empty` (GLOBAL) |
| `src/engine.c` | 4 | FREE | extra:218 is another eff_extra port |
| `src/game/game_setup.c` | 10 | FREE | setup_effect_extra:298 is a 3rd effect_extra port |
| `src/test_game.c` | 7 | **CONTESTED** | src/test_game.c:1151 `test_advance_to_phase_rust` (GLOBAL) |
| `src/turn/live.c` | 4 | FREE | src/turn/live.c:2585 `rb_bt_search` (GLOBAL) |
| `src/turn/phase.c` | 8 | FREE | src/turn/phase.c:1322 `_3ds_tdbg` (GLOBAL) |
| `src/turn/triggers.c` | 1 | FREE | src/turn/triggers.c:1641 `rb_trigger_to_texticon` (GLOBAL) |

## Reachability over the real corpus

Every "DEAD" in this list is measured against `src/ + tests/ + tools/` as built,
not against source text. A DEAD function has zero call sites in all three. When
a fixer adds a caller, `gcc -Wunused-function` on that file will now stop
reporting it, which is the independent check that the change landed.

Every "TEST-ONLY" function IS exercised, so deleting one breaks a test - which
is the point: the test is currently proving something the shipped binary does
not do.

## Constraints on the fixer wave

- **No git writes.** `stash@{0}` has been destroyed three times. Use
  `git stash list`, never `git rev-parse 'stash@{0}'` (it fails in PowerShell).
- **Never leave `.o` or `rb_engine*` in the live tree.**
- **Never run `make run-all-tests` in the shared tree.**
- **Run no generator.** `src/abilities_strings.bin` is CORRECT at 726831 B /
  `89204df0bbde08a0a7f978a56a80d6e6431691ec7d6d658979a329956b06344`; verified
  before and after this census, unchanged. `cards/build/...` is 726649 B and
  STALE, and in-repo `make data` refuses by design - freeze it.
- `-fsyntax-only` is not equivalent to a real compile for the warning classes
  that matter here (they are optimizer-emitted). Use `make`. An unreferenced
  function gets less analysis, which is why gcc never diagnoses this class and
  why this census is manual.
- The Rust reference is being edited (`condition.rs`, `util.rs` carry
  uncommitted changes). Cite Rust by content and say which tree you read.

