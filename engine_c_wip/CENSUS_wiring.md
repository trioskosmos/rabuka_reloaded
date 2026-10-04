# CENSUS 1 of 5 - WIRING

Every function defined in `engine_c_wip/src/**/*.c` (54 `.c` files, 55 files
including the one in-tree header) classified by who calls it.

Ground truth is the compiler, not a regex:

| axis | source | what it sees | what it misses |
|---|---|---|---|
| cross-TU | `nm` undefined-symbol set of each `-O0` object for all 54 src files | every reference from another translation unit | a static function its OWN TU calls is never undefined |
| intra-TU | `gcc -Wunused-function` plus a comment/string-stripped call map | references from inside the same TU | every unreferenced NON-static function |

Both axes are required. `nm` alone reports 1445 dead functions (false
positives: intra-TU calls are invisible to it). `gcc` alone misses all 434
dead globals. The zero-caller set is the union of the two negatives.

## The five counts

| class | count |
|---|---|
| **WIRED** (has a production caller) | **1853** |
| **TEST-ONLY** (reachable only from `tests/` or `tools/`) | **262** |
| **DEAD-CORRECT** | see split below |
| **DEAD-WRONG** | see split below |
| **DEAD-AND-UNREACHABLE-IN-RUST-TOO** | see split below |
| total definitions | 2604 |

DEAD total: **489** = 53 `static` + 436 global.

## The DEAD split, and exactly how far it is proven

| sub-class | count | basis |
|---|---|---|
| DEAD-AND-UNREACHABLE-IN-RUST-TOO | 254 | no item of that name exists in the live Rust tree; Rust does the work inline. Provable by name index. |
| has a Rust name-twin | 235 | needs a semantic read of the two bodies |

Of the 235 with a Rust twin, **53 were adjudicated by reading both bodies**
(every `static` dead function: the class a future agent wires "to activate the
code"). The remaining 182 are listed in full below as UNADJUDICATED rather than
guessed - a cross-language token-similarity score is not evidence of divergence,
and an earlier pass of this census was discarded for producing exactly that.

```
DEAD                        489   (exhaustive list in section 4)
  static, compiler-proven      27   gcc -Wunused-function (own TU)
  static, dead-reference chain 26   referenced only by other dead statics
  global, zero callers         436   declared in rabuka.h, never called
```

Consistency checks that passed on the final data:

- 0 WIRED functions appear in gcc's `-Wunused-function` list.
- 0 TEST-ONLY functions have a src-tree caller.
- 371 of the 436 dead globals have only 1-2 textual occurrences tree-wide
  (definition + header declaration), which is what dead looks like; 40 were
  sampled at random and every one checked by hand.
- The 27 gcc-proven statics are all DEAD or TEST-ONLY; the two TEST-ONLY are
  `card_name` (misc.c:2237) and `stage_area_of` (move.c:385), dead inside
  their own TU and called only from `tests/` - exactly the TEST-ONLY class.

## 1a. Declared but NOT defined - the link-error landmines

7 symbols are declared in a header and defined nowhere in `src/**/*.c`.

| symbol | declared at | state |
|---|---|---|
| `rb_activate_ability_effect` | include/rabuka.h:2247 | tombstoned - the file says it was "zero-caller"; the prototype outlived the body; last body at src/ability/effects/ability.c:600 |
| `rb_cost_clear_deferred` | include/rabuka.h:2700 | tombstoned - explicit `/* ── DELETED: rb_cost_clear_deferred ──` block; last body at src/ability/cost.c:1998 |
| `rb_effect_modify_hearts` | include/rabuka.h:2269 | tombstoned - `DELETED 2026-10-02`; last body at src/ability/effects/state.c:1764 |
| `rb_execute_effect_via_registry` | include/rabuka.h:3238 | tombstoned - `DELETED 2026-10-02`; last body at src/ability/effects/executor.c:592 |
| `rb_invalidate_ability` | include/rabuka.h:2245 | tombstoned - `DELETED 2026-10-02`; last body at src/ability/effects/ability.c:478 |
| `rb_load_rom` | include/rabuka.h:373 | defined ONLY inside `#ifdef RB_ROM_STRINGS` - verified by rebuilding all 54 objects with and without the macro; it is the ONLY conditional definition in the tree; last body at src/core/data.c:147 |
| `rb_translated_execute_custom_effect` | include/rabuka.h:3245 | tombstoned - `DELETED 2026-10-02`; last body at src/ability/effects/ability_effects.c:626 |

Six of the seven are a documented deletion whose **prototype was never
removed from the header**. They are not currently link errors only because
nothing calls them - which is the same property that makes them invisible.
`rb_load_rom` is worse: any caller added to a normal (non-ROM) build gets an
undefined reference even though the source file "defines" it.

## 1b. Defined but never declared in any header

| storage | count | note |
|---|---|---|
| `GLOBAL` (no prototype anywhere) | **442** | one definition per translation unit; a second TU that calls it compiles anyway and fails at link |
| `static` | 1140 | no header declaration needed; these compile by accident |

The T3 finding (45 undeclared `rb_gsc_*` in one file) is now **3**:

- `src/core/game_state_core.c:347` `rb_gsc_mulligan_owner_index`
- `src/core/game_state_core.c:474` `rb_gsc_recently_moved_cards`
- `src/core/game_state_core.c:480` `rb_gsc_set_recently_moved_cards`

Top files by undeclared GLOBAL count:

| `src/ability/effects/rules.c` | 58 |
| `src/ability/vm.c` | 39 |
| `src/game/game_setup.c` | 39 |
| `src/core/alloc_counter.c` | 35 |
| `src/ability/effects/move.c` | 34 |
| `src/core/zones.c` | 32 |
| `src/ability/choice_renderer.c` | 24 |
| `src/turn/phase.c` | 23 |
| `src/ability/choice.c` | 22 |
| `src/ability/util.c` | 21 |
| `src/core/card.c` | 19 |
| `src/core/game_state_abilities.c` | 18 |

## 1c. The full per-file rollup

| file | WIRED | TEST-ONLY | DEAD | total |
|---|---|---|---|---|
| `include/rabuka.h` | 3 | 2 | 0 | 5 |
| `src/ability/abilities_gen.c` | 1 | 0 | 1 | 2 |
| `src/ability/ability_queue.c` | 41 | 21 | 2 | 64 |
| `src/ability/choice.c` | 101 | 1 | 32 | 134 |
| `src/ability/choice_renderer.c` | 71 | 14 | 22 | 107 |
| `src/ability/compound.c` | 30 | 1 | 7 | 38 |
| `src/ability/condition.c` | 117 | 0 | 1 | 118 |
| `src/ability/cost.c` | 70 | 1 | 7 | 78 |
| `src/ability/debug.c` | 14 | 1 | 2 | 17 |
| `src/ability/describe.c` | 73 | 14 | 5 | 92 |
| `src/ability/dynamic_count.c` | 18 | 0 | 0 | 18 |
| `src/ability/effects/ability.c` | 13 | 0 | 5 | 18 |
| `src/ability/effects/ability_effects.c` | 16 | 0 | 0 | 16 |
| `src/ability/effects/custom.c` | 10 | 0 | 0 | 10 |
| `src/ability/effects/draw.c` | 23 | 0 | 4 | 27 |
| `src/ability/effects/executor.c` | 10 | 0 | 0 | 10 |
| `src/ability/effects/look.c` | 51 | 0 | 12 | 63 |
| `src/ability/effects/misc.c` | 60 | 1 | 2 | 63 |
| `src/ability/effects/move.c` | 61 | 1 | 9 | 71 |
| `src/ability/effects/move_missing.c` | 11 | 0 | 1 | 12 |
| `src/ability/effects/rules.c` | 55 | 0 | 48 | 103 |
| `src/ability/effects/score.c` | 33 | 0 | 0 | 33 |
| `src/ability/effects/state.c` | 46 | 1 | 4 | 51 |
| `src/ability/enums.c` | 10 | 11 | 0 | 21 |
| `src/ability/log.c` | 14 | 3 | 4 | 21 |
| `src/ability/resolver.c` | 37 | 5 | 10 | 52 |
| `src/ability/types.c` | 20 | 11 | 79 | 110 |
| `src/ability/util.c` | 90 | 3 | 27 | 120 |
| `src/ability/vm.c` | 79 | 14 | 7 | 100 |
| `src/core/alloc.c` | 11 | 0 | 3 | 14 |
| `src/core/alloc_counter.c` | 28 | 0 | 21 | 49 |
| `src/core/card.c` | 64 | 25 | 17 | 106 |
| `src/core/data.c` | 39 | 11 | 14 | 64 |
| `src/core/game_state_abilities.c` | 91 | 6 | 34 | 131 |
| `src/core/game_state_core.c` | 16 | 0 | 39 | 55 |
| `src/core/gen_data_rom.c` | 3 | 0 | 0 | 3 |
| `src/core/generated/gen_data_cdi.c` | 1 | 0 | 0 | 1 |
| `src/core/modifiers.c` | 62 | 8 | 10 | 80 |
| `src/core/stats_pipeline.c` | 15 | 1 | 0 | 16 |
| `src/core/tracking.c` | 8 | 2 | 1 | 11 |
| `src/core/zones.c` | 30 | 20 | 28 | 78 |
| `src/debug_umi.c` | 1 | 0 | 0 | 1 |
| `src/engine.c` | 38 | 5 | 3 | 46 |
| `src/game/deck_builder.c` | 6 | 1 | 0 | 7 |
| `src/game/deck_parser.c` | 16 | 2 | 0 | 18 |
| `src/game/game_setup.c` | 48 | 0 | 8 | 56 |
| `src/main.c` | 4 | 0 | 0 | 4 |
| `src/test_game.c` | 29 | 74 | 7 | 110 |
| `src/turn/live.c` | 53 | 1 | 4 | 58 |
| `src/turn/phase.c` | 48 | 0 | 8 | 56 |
| `src/turn/triggers.c` | 33 | 1 | 1 | 35 |
| `src/web_server.c` | 31 | 0 | 0 | 31 |



## 2. Every DEAD function (exhaustive)

### 2a. The 53 dead `static` functions - the class a fixer wires by accident

| file:line | function | verdict | the specific divergence |
|---|---|---|---|
| `src/ability/choice_renderer.c:1514` | `t_clear_screen` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1518` | `t_println` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1522` | `t_swap_buffers` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1523` | `t_poll_input` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1534` | `t_wait_vblank` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1535` | `t_reset_vram` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1540` | `t_pressed_a` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1541` | `t_pressed_b` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1542` | `t_pressed_l` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1543` | `t_pressed_r` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1544` | `t_pressed_up` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1545` | `t_pressed_down` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1546` | `t_pressed_left` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1547` | `t_pressed_right` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1548` | `t_pressed_start` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1549` | `t_pressed_select` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1565` | `t_show_board_overlay` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1595` | `t_draw_card_image_narrow` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1598` | `t_show_detail_screen_narrow` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1606` | `t_open_start_menu_narrow` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/compound.c:374` | `rb_on_optional_branch` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/cost.c:119` | `eff_int` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/cost.c:2725` | `cost_count_in_source` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/cost.c:2736` | `cost_move_from_source` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:97` | `look_card_plural` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:104` | `look_zone_label` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:127` | `look_zone_label_ja` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:161` | `look_push_reveal_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:171` | `look_push_reveal_group_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:180` | `look_push_reveal_until_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:227` | `look_push_select_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:315` | `look_or_card_types` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:876` | `card_is_live_pred` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:886` | `card_type_pred` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:894` | `card_type_cost_pred` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/move.c:134` | `mc_set_recently_moved_batch` | DEAD-WRONG | Comments claim it REPLACES the batch (mod.rs:1683). Being unreferenced, no live path performs that replacement; `g->those_cards` is therefore only ever APPENDED to. |
| `src/ability/util.c:171` | `rb_norm_group_name` | DEAD-WRONG | Rust `util.rs:699` normalises FULLWIDTH '!' (U+FF01) and micro (U+00B5). A C `norm_group_name` that the group gate needs; unreferenced, so the group gate cannot use it. |
| `src/ability/util.c:382` | `rb_debug_group_match` | DEAD-WRONG | The C body is empty. Rust `util.rs:652 debug_group_match` performs unit/group/name/series checks under `RABUKA_DEBUG_GROUP=1` + trace level. The C comment calls it a 'Mirror' and then says 'no-op in the C port'; the word Mirror is false. |
| `src/ability/util.c:1769` | `local_has_filter` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/util.c:1785` | `card_filter_matches_skip` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/util.c:1790` | `card_filter_matches_card` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/util.c:2895` | `rb_max_distinct_names_greedy` | DEAD-WRONG | Rust `util.rs:1305 max_distinct_names_greedy` returns `DistinctNamesResult{distinct, collision}`; the C returns only the count and silently DROPS `had_collision`. The C comment says 'delegates to greedy', which is true and is exactly the problem - the delegate has two outputs. |
| `src/ability/util.c:2907` | `cmp_u64` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/vm.c:605` | `effect_decode_extra` | DEAD-WRONG | A bytecode decoder arm with no caller: the `extra` tag it decodes has no C consumer, so any effect `extra` the generator emits through this arm is dropped rather than stored. |
| `src/core/card.c:229` | `pair_order_cmp` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/core/game_state_abilities.c:775` | `rb_is_stale_discard_condition` | DEAD-CORRECT | Faithful to `abilities.rs:486`, including the parenthesised final conjunction; adds only a null guard Rust cannot express. |
| `src/core/game_state_abilities.c:850` | `rb_requires_this_move` | DEAD-CORRECT | Faithful to `abilities.rs:570`; `nloc >= 2` is `l.len() < 2` negated, `unwrap_or(false)` is the 0 default. |
| `src/core/game_state_abilities.c:894` | `rb_set_just_completed` | DEAD-WRONG | Rust `abilities.rs:183 set_just_completed` sets the key AND copies `recently_moved_cards` into `just_completed_moved`. The C setter writes a separate static `s_just_completed[]` side-table instead, and is itself unreferenced. |
| `src/core/game_state_abilities.c:917` | `rb_clear_just_completed` | DEAD-WRONG | The counterpart of the above and equally unreferenced. Note `g->just_completed_ability_key` IS cleared on the live path at `game_state_abilities.c:2479`, so this dead twin is a second, never-used clearing mechanism rather than a live bug. |
| `src/core/game_state_abilities.c:924` | `rb_just_completed_batch_matches` | DEAD-WRONG | Rust `abilities.rs:195` compares the stored moved-batch slice; the C compares against its private static table. Both halves of the Rust comparison are unreferenced, so the batch check is not implemented on any live path. |
| `src/core/game_state_abilities.c:950` | `rb_claim_batch_slot` | DEAD-WRONG | Rust `abilities.rs:510 claim_batch_slot` pushes into a `Vec`, which grows. The C adds `if (g->n_batch_triggered_keys >= cap) return 0;` - once the fixed array fills, the C SILENTLY DROPS triggers Rust would still fire. |
| `src/game/game_setup.c:1548` | `setup_gba_cmp` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/turn/phase.c:1859` | `rb_ability_offer_sink` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |

**Adjudicated dead statics: 9 DEAD-WRONG, 2 DEAD-CORRECT, 42 DEAD-AND-UNREACHABLE-IN-RUST-TOO.**

### 2b. The 436 dead GLOBAL functions (declared in a header, called nowhere)

Every one of these is declared in `include/rabuka.h` or `include/test_game.h`
and defined exactly once in `src/`, with zero call sites in `src/`, `tests/`
and `tools/`. They read as a complete API surface; nothing can call them.

| file:line | function | storage |
|---|---|---|
| `src/ability/abilities_gen.c:984` | `bytecode_slice` | GLOBAL |
| `src/ability/ability_queue.c:307` | `rb_queue_push_with_trigger` | GLOBAL |
| `src/ability/ability_queue.c:967` | `rb_queue_save_pending_actions` | GLOBAL |
| `src/ability/choice.c:76` | `rb_get_card` | GLOBAL |
| `src/ability/choice.c:77` | `rb_card_db_unit` | GLOBAL |
| `src/ability/choice.c:85` | `rb_ability_master_id` | GLOBAL |
| `src/ability/choice.c:86` | `rb_choice_destination` | GLOBAL |
| `src/ability/choice.c:101` | `rb_compound_route_conditional_branch` | GLOBAL |
| `src/ability/choice.c:118` | `rb_effect_answers_any` | GLOBAL |
| `src/ability/choice.c:162` | `rb_effect_resource_on_select` | GLOBAL |
| `src/ability/choice.c:168` | `rb_effect_alternative_count_type_any` | GLOBAL |
| `src/ability/choice.c:174` | `rb_entry_conditional_choice_effect` | GLOBAL |
| `src/ability/choice.c:240` | `rb_resolver_build_choice_select_cards` | GLOBAL |
| `src/ability/choice.c:297` | `rb_resolver_card_name` | GLOBAL |
| `src/ability/choice.c:309` | `rb_resolver_entry_effect` | GLOBAL |
| `src/ability/choice.c:319` | `rb_resolver_look_select_finalize_dest` | GLOBAL |
| `src/ability/choice.c:395` | `rb_selection_context_mfi_count` | GLOBAL |
| `src/ability/choice.c:456` | `rb_resolver_clear_choice_state_and_resume2` | GLOBAL |
| `src/ability/choice.c:466` | `rb_resolver_reschedule_pending_choice` | GLOBAL |
| `src/ability/choice.c:492` | `rb_resolver_handle_select_cards_looked_at` | GLOBAL |
| `src/ability/choice.c:503` | `rb_resolver_move_non_selected_hand_to_deck_bottom` | GLOBAL |
| `src/ability/choice.c:576` | `rb_resolver_resume_execution_with_ctx` | GLOBAL |
| `src/ability/choice.c:737` | `rb_resolver_finalize_choice_with_ctx` | GLOBAL |
| `src/ability/choice.c:743` | `rb_resolver_reveal_selected_looked_at` | GLOBAL |
| `src/ability/choice.c:955` | `rb_resolver_provide_choice_result` | GLOBAL |
| `src/ability/choice.c:1061` | `rb_resolver_build_reprompt` | GLOBAL |
| `src/ability/choice.c:1072` | `rb_resolver_build_reprompt_full` | GLOBAL |
| `src/ability/choice.c:3054` | `rb_resolver_handle_entry_cost_reveal` | GLOBAL |
| `src/ability/choice.c:3297` | `rb_resolver_filter_discard_by_budget` | GLOBAL |
| `src/ability/choice.c:4477` | `rb_resolver_apply_effect_modification` | GLOBAL |
| `src/ability/choice.c:4519` | `rb_resolver_handle_position_destination` | GLOBAL |
| `src/ability/choice.c:5649` | `rb_gained_ability_index` | GLOBAL |
| `src/ability/choice.c:5657` | `rb_choice_description_ja` | GLOBAL |
| `src/ability/choice.c:5691` | `rb_ability_error_to_string` | GLOBAL |
| `src/ability/choice.c:5710` | `rb_resolver_handle_energy_zone_selection` | GLOBAL |
| `src/ability/choice_renderer.c:1689` | `rb_rend_text_idle` | GLOBAL |
| `src/ability/choice_renderer.c:1696` | `rb_rend_text_n_presses` | GLOBAL |
| `src/ability/compound.c:350` | `rb_compound_save_remaining` | GLOBAL |
| `src/ability/compound.c:370` | `rb_compound_route_branch` | GLOBAL |
| `src/ability/compound.c:762` | `rb_compound_choice_string` | GLOBAL |
| `src/ability/compound.c:767` | `rb_compound_choice_action` | GLOBAL |
| `src/ability/compound.c:777` | `rb_compound_handle_choice_string_selection` | GLOBAL |
| `src/ability/compound.c:811` | `rb_compound_handle_choice_string_store` | GLOBAL |
| `src/ability/condition.c:2839` | `rb_condition_allows` | GLOBAL |
| `src/ability/cost.c:2407` | `rb_cost_has_skip_prompt` | GLOBAL |
| `src/ability/cost.c:2412` | `rb_get_change_state_candidates` | GLOBAL |
| `src/ability/cost.c:2420` | `rb_pay_cost_move_cards` | GLOBAL |
| `src/ability/cost.c:2429` | `rb_pay_cost_change_state` | GLOBAL |
| `src/ability/debug.c:35` | `rb_abdebug_dropped_count` | GLOBAL |
| `src/ability/debug.c:36` | `rb_abdebug_dropped_reset` | GLOBAL |
| `src/ability/describe.c:1813` | `rb_desc_sequential_cost` | GLOBAL |
| `src/ability/describe.c:2831` | `rb_desc_cond_expectation` | GLOBAL |
| `src/ability/describe.c:2835` | `rb_desc_cond_actual` | GLOBAL |
| `src/ability/describe.c:2860` | `rb_desc_push_cond_verdict` | GLOBAL |
| `src/ability/describe.c:2897` | `rb_desc_apply_bilingual` | GLOBAL |
| `src/ability/effects/ability.c:82` | `rb_ability_ref_index` | GLOBAL |
| `src/ability/effects/ability.c:89` | `rb_ability_ref_idx` | GLOBAL |
| `src/ability/effects/ability.c:273` | `rb_ability_ref_decode` | GLOBAL |
| `src/ability/effects/ability.c:291` | `rb_ability_ref_to_arc` | GLOBAL |
| `src/ability/effects/ability.c:340` | `rb_ability_ref_cache_size` | GLOBAL |
| `src/ability/effects/draw.c:406` | `rb_draw_resolve_dynamic_count` | GLOBAL |
| `src/ability/effects/draw.c:647` | `rb_execute_draw_wrapper` | GLOBAL |
| `src/ability/effects/draw.c:1018` | `rb_resolve_gain_heart_color` | GLOBAL |
| `src/ability/effects/draw.c:1060` | `rb_make_card_effect_data` | GLOBAL |
| `src/ability/effects/look.c:833` | `rb_look_resume` | GLOBAL |
| `src/ability/effects/misc.c:1823` | `rb_misc_position_destinations` | GLOBAL |
| `src/ability/effects/misc.c:1861` | `rb_misc_finalize_formation_change` | GLOBAL |
| `src/ability/effects/move.c:1666` | `rb_move_resolve_from_standard_zone` | GLOBAL |
| `src/ability/effects/move.c:1674` | `rb_move_resolve_from_revealed_cards` | GLOBAL |
| `src/ability/effects/move.c:1725` | `rb_move_resolve_from_those_cards` | GLOBAL |
| `src/ability/effects/move.c:1763` | `rb_move_resolve_from_selected_cards` | GLOBAL |
| `src/ability/effects/move.c:1780` | `rb_move_resolve_source_revealed_cards` | GLOBAL |
| `src/ability/effects/move.c:2224` | `rb_move_from_revealed` | GLOBAL |
| `src/ability/effects/move.c:2270` | `rb_move_from_under_member` | GLOBAL |
| `src/ability/effects/move.c:2510` | `rb_move_execute_move_cards` | GLOBAL |
| `src/ability/effects/move_missing.c:490` | `rb_move_ask_optional_move_gate` | GLOBAL |
| `src/ability/effects/rules.c:140` | `rb_rules_forget` | GLOBAL |
| `src/ability/effects/rules.c:595` | `rb_rules_execute_modify_required_hearts` | GLOBAL |
| `src/ability/effects/rules.c:727` | `rb_rules_execute_modify_required_hearts_standard` | GLOBAL |
| `src/ability/effects/rules.c:762` | `rb_rules_execute_modify_required_hearts_success` | GLOBAL |
| `src/ability/effects/rules.c:805` | `rb_rules_execute_specify_heart_color` | GLOBAL |
| `src/ability/effects/rules.c:822` | `rb_rules_resolve_gain_heart_color` | GLOBAL |
| `src/ability/effects/rules.c:859` | `rb_rules_execute_choose_required_hearts` | GLOBAL |
| `src/ability/effects/rules.c:919` | `rb_rules_execute_set_cost` | GLOBAL |
| `src/ability/effects/rules.c:936` | `rb_rules_execute_modify_cost` | GLOBAL |
| `src/ability/effects/rules.c:1017` | `rb_rules_execute_activation_cost` | GLOBAL |
| `src/ability/effects/rules.c:1036` | `rb_rules_execute_set_cost_to_use` | GLOBAL |
| `src/ability/effects/rules.c:1051` | `rb_rules_position_limited_targets` | GLOBAL |
| `src/ability/effects/rules.c:1114` | `rb_rules_implicit_heart_target` | GLOBAL |
| `src/ability/effects/rules.c:1139` | `rb_rules_try_create_target_selection_choice` | GLOBAL |
| `src/ability/effects/rules.c:1304` | `rb_rules_handle_both_targets` | GLOBAL |
| `src/ability/effects/rules.c:1458` | `rb_rules_execute_select_number` | GLOBAL |
| `src/ability/effects/rules.c:1541` | `rb_rules_execute_choose_target_player` | GLOBAL |
| `src/ability/effects/rules.c:1580` | `rb_rules_execute_select_effect` | GLOBAL |
| `src/ability/effects/rules.c:1639` | `rb_rules_make_card_effect_data` | GLOBAL |
| `src/ability/effects/rules.c:1651` | `rb_rules_execute_choice` | GLOBAL |
| `src/ability/effects/rules.c:1819` | `rb_rules_execute_restriction` | GLOBAL |
| `src/ability/effects/rules.c:1932` | `rb_rules_execute_activation_restriction` | GLOBAL |
| `src/ability/effects/rules.c:1944` | `rb_rules_execute_reduce_live_card_set_limit` | GLOBAL |
| `src/ability/effects/rules.c:1956` | `rb_rules_execute_modify_limit` | GLOBAL |
| `src/ability/effects/rules.c:2005` | `rb_rules_execute_re_yell` | GLOBAL |
| `src/ability/effects/rules.c:2028` | `rb_rules_execute_perform_yell` | GLOBAL |
| `src/ability/effects/rules.c:2119` | `rb_rules_execute_modify_yell_count` | GLOBAL |
| `src/ability/effects/rules.c:2139` | `rb_rules_register_replacement_effect` | GLOBAL |
| `src/ability/effects/rules.c:2167` | `rb_rules_run_replacement_effects` | GLOBAL |
| `src/ability/effects/rules.c:2245` | `rb_rules_cannot_live_player` | GLOBAL |
| `src/ability/effects/rules.c:2251` | `rb_rules_cannot_activate_player` | GLOBAL |
| `src/ability/effects/rules.c:2257` | `rb_rules_n_delayed_prohibitions` | GLOBAL |
| `src/ability/effects/rules.c:2262` | `rb_rules_delayed_prohibition` | GLOBAL |
| `src/ability/effects/rules.c:2306` | `rb_rules_note_moved_cards` | GLOBAL |
| `src/ability/effects/rules.c:2315` | `rb_rules_note_selected_cards` | GLOBAL |
| `src/ability/effects/rules.c:2324` | `rb_rules_set_conditional_choice` | GLOBAL |
| `src/ability/effects/rules.c:2333` | `rb_rules_get_conditional_choice` | GLOBAL |
| `src/ability/effects/rules.c:2340` | `rb_rules_choice_repeats_remaining` | GLOBAL |
| `src/ability/effects/rules.c:2345` | `rb_rules_consume_choice_repeat` | GLOBAL |
| `src/ability/effects/rules.c:2353` | `rb_rules_note_keep_shuffle_selected` | GLOBAL |
| `src/ability/effects/rules.c:2362` | `rb_rules_keep_shuffle_phase` | GLOBAL |
| `src/ability/effects/rules.c:2365` | `rb_rules_keep_shuffle_count` | GLOBAL |
| `src/ability/effects/rules.c:2368` | `rb_rules_sub_choice_created` | GLOBAL |
| `src/ability/effects/rules.c:2371` | `rb_rules_count_at_save` | GLOBAL |
| `src/ability/effects/rules.c:2374` | `rb_rules_owner_executing` | GLOBAL |
| `src/ability/effects/rules.c:2377` | `rb_rules_resolving_replacement` | GLOBAL |
| `src/ability/effects/rules.c:2380` | `rb_rules_take_pending_replacement` | GLOBAL |
| `src/ability/effects/rules.c:2389` | `rb_rules_apply_pending_replacement` | GLOBAL |
| `src/ability/effects/state.c:1398` | `rb_all_blade_timing_record` | GLOBAL |
| `src/ability/effects/state.c:2072` | `rb_card_need_heart_satisfied` | GLOBAL |
| `src/ability/effects/state.c:2114` | `rb_heart_color_from_index` | GLOBAL |
| `src/ability/effects/state.c:2120` | `rb_heart_color_short_label` | GLOBAL |
| `src/ability/log.c:44` | `rb_log_dropped_count` | GLOBAL |
| `src/ability/log.c:45` | `rb_log_dropped_reset` | GLOBAL |
| `src/ability/log.c:198` | `rb_log_push_verdict_cost` | GLOBAL |
| `src/ability/log.c:249` | `rb_log_push_verdict_item` | GLOBAL |
| `src/ability/resolver.c:253` | `rb_resolver_zone_for_card` | GLOBAL |
| `src/ability/resolver.c:279` | `rb_resolver_get_pending_choice` | GLOBAL |
| `src/ability/resolver.c:283` | `rb_resolver_pending_choice` | GLOBAL |
| `src/ability/resolver.c:380` | `rb_resolver_emit_pay_skip_gate` | GLOBAL |
| `src/ability/resolver.c:679` | `rb_resolver_card_matches_type` | GLOBAL |
| `src/ability/resolver.c:682` | `rb_resolver_card_matches_cost_limit` | GLOBAL |
| `src/ability/resolver.c:686` | `rb_resolver_fmt_card` | GLOBAL |
| `src/ability/resolver.c:695` | `rb_resolver_fmt_ids` | GLOBAL |
| `src/ability/resolver.c:705` | `rb_resolver_merge_group_names` | GLOBAL |
| `src/ability/resolver.c:847` | `rb_resolver_trigger_infos` | GLOBAL |
| `src/ability/types.c:13` | `rb_step_output_init` | GLOBAL |
| `src/ability/types.c:17` | `rb_step_output_from_value` | GLOBAL |
| `src/ability/types.c:45` | `rb_step_output_has_cards` | GLOBAL |
| `src/ability/types.c:49` | `rb_step_output_value` | GLOBAL |
| `src/ability/types.c:53` | `rb_step_output_accepted` | GLOBAL |
| `src/ability/types.c:77` | `rb_step_output_to_json` | GLOBAL |
| `src/ability/types.c:170` | `rb_value_ref_merge` | GLOBAL |
| `src/ability/types.c:217` | `rb_trace_node_with_card` | GLOBAL |
| `src/ability/types.c:236` | `rb_trace_node_with_after` | GLOBAL |
| `src/ability/types.c:243` | `rb_trace_node_from_game_state` | GLOBAL |
| `src/ability/types.c:251` | `rb_trace_node_add_child` | GLOBAL |
| `src/ability/types.c:267` | `rb_effect_pipeline_new` | GLOBAL |
| `src/ability/types.c:274` | `rb_effect_pipeline_free` | GLOBAL |
| `src/ability/types.c:286` | `rb_step_state_new` | GLOBAL |
| `src/ability/types.c:311` | `rb_step_state_get` | GLOBAL |
| `src/ability/types.c:326` | `rb_step_state_free` | GLOBAL |
| `src/ability/types.c:332` | `rb_step_state_record_value` | GLOBAL |
| `src/ability/types.c:341` | `rb_step_state_record_cards` | GLOBAL |
| `src/ability/types.c:352` | `rb_ability_error_format` | GLOBAL |
| `src/ability/types.c:383` | `rb_exec_context_kind_to_str` | GLOBAL |
| `src/ability/types.c:393` | `rb_exec_context_kind_from_str` | GLOBAL |
| `src/ability/types.c:402` | `rb_las_kind_to_str` | GLOBAL |
| `src/ability/types.c:411` | `rb_las_kind_from_str` | GLOBAL |
| `src/ability/types.c:421` | `rb_full_choice_new_select_card` | GLOBAL |
| `src/ability/types.c:446` | `rb_full_choice_new_select_position` | GLOBAL |
| `src/ability/types.c:458` | `rb_full_choice_new_select_heart_color` | GLOBAL |
| `src/ability/types.c:499` | `rb_full_choice_set_aa_options` | GLOBAL |
| `src/ability/types.c:503` | `rb_full_choice_set_ls_options` | GLOBAL |
| `src/ability/types.c:507` | `rb_full_choice_description_ja` | GLOBAL |
| `src/ability/types.c:511` | `rb_full_choice_allow_skip` | GLOBAL |
| `src/ability/types.c:515` | `rb_full_choice_to_header` | GLOBAL |
| `src/ability/types.c:543` | `rb_full_choice_to_json` | GLOBAL |
| `src/ability/types.c:550` | `rb_full_choice_free` | GLOBAL |
| `src/ability/types.c:556` | `rb_choice_result_new_card_selected` | GLOBAL |
| `src/ability/types.c:566` | `rb_choice_result_new_skip` | GLOBAL |
| `src/ability/types.c:574` | `rb_choice_result_new_target` | GLOBAL |
| `src/ability/types.c:583` | `rb_choice_result_kind` | GLOBAL |
| `src/ability/types.c:587` | `rb_choice_result_is_skip` | GLOBAL |
| `src/ability/types.c:591` | `rb_choice_result_kind_to_str` | GLOBAL |
| `src/ability/types.c:605` | `rb_choice_result_kind_from_str` | GLOBAL |
| `src/ability/types.c:618` | `rb_choice_result_free` | GLOBAL |
| `src/ability/types.c:626` | `rb_choice_route_to_str` | GLOBAL |
| `src/ability/types.c:648` | `rb_choice_route_new` | GLOBAL |
| `src/ability/types.c:654` | `rb_choice_route_from_kind` | GLOBAL |
| `src/ability/types.c:666` | `rb_choice_route_kind_from_header` | GLOBAL |
| `src/ability/types.c:680` | `rb_repeat_prompt_choice` | GLOBAL |
| `src/ability/types.c:691` | `rb_trigger_event_init` | GLOBAL |
| `src/ability/types.c:695` | `rb_trigger_event_add_moved` | GLOBAL |
| `src/ability/types.c:700` | `rb_trigger_event_add_appeared` | GLOBAL |
| `src/ability/types.c:710` | `rb_trigger_event_has_moved` | GLOBAL |
| `src/ability/types.c:714` | `rb_trigger_event_has_appeared` | GLOBAL |
| `src/ability/types.c:718` | `rb_trigger_event_has_position_change` | GLOBAL |
| `src/ability/types.c:722` | `rb_trigger_event_has_energy_placed` | GLOBAL |
| `src/ability/types.c:726` | `rb_trigger_event_copy` | GLOBAL |
| `src/ability/types.c:732` | `rb_effect_spawn_context_init` | GLOBAL |
| `src/ability/types.c:760` | `rb_choice_builder_free` | GLOBAL |
| `src/ability/types.c:764` | `rb_choice_builder_card_type` | GLOBAL |
| `src/ability/types.c:769` | `rb_choice_builder_cost_limit` | GLOBAL |
| `src/ability/types.c:778` | `rb_choice_builder_cost_total` | GLOBAL |
| `src/ability/types.c:787` | `rb_choice_builder_cost_values` | GLOBAL |
| `src/ability/types.c:796` | `rb_choice_builder_group` | GLOBAL |
| `src/ability/types.c:801` | `rb_choice_builder_characters` | GLOBAL |
| `src/ability/types.c:809` | `rb_choice_builder_heart_colors` | GLOBAL |
| `src/ability/types.c:814` | `rb_choice_builder_require_all_heart_colors` | GLOBAL |
| `src/ability/types.c:819` | `rb_choice_builder_name_fragments` | GLOBAL |
| `src/ability/types.c:824` | `rb_choice_builder_destination` | GLOBAL |
| `src/ability/types.c:829` | `rb_choice_builder_discard_remaining` | GLOBAL |
| `src/ability/types.c:834` | `rb_choice_builder_is_select_action` | GLOBAL |
| `src/ability/types.c:839` | `rb_choice_builder_target_player_id` | GLOBAL |
| `src/ability/types.c:844` | `rb_choice_builder_blind` | GLOBAL |
| `src/ability/types.c:849` | `rb_choice_builder_is_reveal` | GLOBAL |
| `src/ability/types.c:859` | `rb_choice_builder_description_en` | GLOBAL |
| `src/ability/types.c:864` | `rb_choice_builder_description_ja` | GLOBAL |
| `src/ability/types.c:869` | `rb_choice_builder_filtered_indices` | GLOBAL |
| `src/ability/types.c:884` | `rb_arcstr_serialize` | GLOBAL |
| `src/ability/types.c:895` | `rb_arcstr_deserialize` | GLOBAL |
| `src/ability/types.c:906` | `rb_look_and_select_step_look` | GLOBAL |
| `src/ability/types.c:915` | `rb_look_and_select_step_select` | GLOBAL |
| `src/ability/types.c:927` | `rb_look_and_select_step_finalize` | GLOBAL |
| `src/ability/util.c:682` | `rb_card_matches_cost_limit_op` | GLOBAL |
| `src/ability/util.c:846` | `rb_target_player_label` | GLOBAL |
| `src/ability/util.c:955` | `rb_zone_card_ids` | GLOBAL |
| `src/ability/util.c:992` | `rb_count_in_zone_filtered` | GLOBAL |
| `src/ability/util.c:1334` | `rb_move_cards` | GLOBAL |
| `src/ability/util.c:1945` | `rb_matching_indices` | GLOBAL |
| `src/ability/util.c:2081` | `rb_filter_distinct` | GLOBAL |
| `src/ability/util.c:2130` | `rb_count_matching_distinct` | GLOBAL |
| `src/ability/util.c:2153` | `rb_build_candidate_pool` | GLOBAL |
| `src/ability/util.c:2169` | `rb_matching_ids_filtered` | GLOBAL |
| `src/ability/util.c:2241` | `rb_get_selection_indices` | GLOBAL |
| `src/ability/util.c:2270` | `rb_resolve_selection` | GLOBAL |
| `src/ability/util.c:2767` | `rb_calculate_per_unit_multiplier` | GLOBAL |
| `src/ability/util.c:2911` | `rb_prune_dominated` | GLOBAL |
| `src/ability/util.c:2996` | `rb_push_temporary_effect` | GLOBAL |
| `src/ability/util.c:3065` | `rb_set_card_identity` | GLOBAL |
| `src/ability/util.c:3104` | `rb_filter_from_parts` | GLOBAL |
| `src/ability/util.c:3117` | `rb_filter_from_parts_full` | GLOBAL |
| `src/ability/util.c:3134` | `rb_has_cannot_baton_touch_protection_util` | GLOBAL |
| `src/ability/util.c:3140` | `rb_exclude_self` | GLOBAL |
| `src/ability/vm.c:2794` | `build_filter` | GLOBAL |
| `src/ability/vm.c:2810` | `offset_of` | GLOBAL |
| `src/ability/vm.c:2822` | `rd_i64` | GLOBAL |
| `src/ability/vm.c:2835` | `rd_key` | GLOBAL |
| `src/ability/vm.c:2845` | `populate_from_json` | GLOBAL |
| `src/ability/vm.c:2863` | `rb_zone_as_str` | GLOBAL |
| `src/core/alloc.c:20` | `rb_alloc_reset` | GLOBAL |
| `src/core/alloc.c:92` | `rb_put` | GLOBAL |
| `src/core/alloc.c:98` | `rb_drop_value` | GLOBAL |
| `src/core/alloc_counter.c:267` | `rb_allocctr_alloc_buckets` | GLOBAL |
| `src/core/alloc_counter.c:302` | `rb_allocctr_note_alloc` | GLOBAL |
| `src/core/alloc_counter.c:306` | `rb_allocctr_note_alloc_at` | GLOBAL |
| `src/core/alloc_counter.c:394` | `rb_allocctr_trace_armed` | GLOBAL |
| `src/core/alloc_counter.c:454` | `rb_allocctr_trace_guard_release` | GLOBAL |
| `src/core/alloc_counter.c:481` | `rb_allocctr_maybe_arm` | GLOBAL |
| `src/core/alloc_counter.c:640` | `rb_allocctr_start` | GLOBAL |
| `src/core/alloc_counter.c:652` | `rb_allocctr_guard_release` | GLOBAL |
| `src/core/alloc_counter.c:680` | `rb_allocctr_free` | GLOBAL |
| `src/core/alloc_counter.c:705` | `rb_allocctr_strdup2` | GLOBAL |
| `src/core/alloc_counter.c:715` | `rb_allocctr_alloc_count` | GLOBAL |
| `src/core/alloc_counter.c:719` | `rb_allocctr_dealloc_count` | GLOBAL |
| `src/core/alloc_counter.c:723` | `rb_allocctr_live_bytes` | GLOBAL |
| `src/core/alloc_counter.c:727` | `rb_allocctr_peak_bytes` | GLOBAL |
| `src/core/alloc_counter.c:731` | `rb_allocctr_total_allocated` | GLOBAL |
| `src/core/alloc_counter.c:738` | `rb_allocctr_outstanding_allocs` | GLOBAL |
| `src/core/alloc_counter.c:744` | `rb_allocctr_unmatched_free_count` | GLOBAL |
| `src/core/alloc_counter.c:748` | `rb_allocctr_ledger_overflow_count` | GLOBAL |
| `src/core/alloc_counter.c:765` | `rb_allocctr_flag_action_display_enabled` | GLOBAL |
| `src/core/alloc_counter.c:773` | `rb_allocctr_flag_logging_enabled` | GLOBAL |
| `src/core/alloc_counter.c:777` | `rb_allocctr_flag_set_training_mode` | GLOBAL |
| `src/core/card.c:429` | `rb_parse_operation` | GLOBAL |
| `src/core/card.c:449` | `rb_card_state_str` | GLOBAL |
| `src/core/card.c:457` | `rb_card_state_from_str` | GLOBAL |
| `src/core/card.c:462` | `rb_comparison_target_str` | GLOBAL |
| `src/core/card.c:470` | `rb_comparison_target_from_str` | GLOBAL |
| `src/core/card.c:475` | `rb_card_property_str` | GLOBAL |
| `src/core/card.c:484` | `rb_card_property_from_str` | GLOBAL |
| `src/core/card.c:491` | `rb_placement_order_str` | GLOBAL |
| `src/core/card.c:495` | `rb_distinct_type_str` | GLOBAL |
| `src/core/card.c:504` | `rb_comparison_type_str` | GLOBAL |
| `src/core/card.c:515` | `rb_comparison_type_from_str` | GLOBAL |
| `src/core/card.c:524` | `rb_ability_filter_str` | GLOBAL |
| `src/core/card.c:534` | `rb_ability_filter_from_str` | GLOBAL |
| `src/core/card.c:542` | `rb_condition_target_str` | GLOBAL |
| `src/core/card.c:552` | `rb_condition_card_type_str` | GLOBAL |
| `src/core/card.c:567` | `rb_location_str` | GLOBAL |
| `src/core/data.c:219` | `rb_load_streaming` | GLOBAL |
| `src/core/data.c:667` | `rb_effect_data_clone` | GLOBAL |
| `src/core/data.c:717` | `rb_turn_phase_display` | GLOBAL |
| `src/core/data.c:745` | `rb_phase_display` | GLOBAL |
| `src/core/data.c:765` | `rb_effect_data_card_id` | GLOBAL |
| `src/core/data.c:785` | `rb_effect_data_items` | GLOBAL |
| `src/core/data.c:819` | `rb_effect_data_is_p1` | GLOBAL |
| `src/core/data.c:825` | `rb_effect_data_old_value` | GLOBAL |
| `src/core/data.c:831` | `rb_effect_data_count` | GLOBAL |
| `src/core/data.c:837` | `rb_effect_data_color` | GLOBAL |
| `src/core/data.c:851` | `rb_phase_label_jp` | GLOBAL |
| `src/core/data.c:871` | `rb_effect_type_as_str` | GLOBAL |
| `src/core/data.c:1068` | `rb_zone_matches_dest` | GLOBAL |
| `src/core/data.c:1095` | `rb_effect_data_amount` | GLOBAL |
| `src/core/game_state_abilities.c:284` | `rb_expire_gained_abilities` | GLOBAL |
| `src/core/game_state_abilities.c:471` | `rb_collect_constant_hand_effect_ids` | GLOBAL |
| `src/core/game_state_abilities.c:485` | `rb_collect_constant_stage_effect_ids` | GLOBAL |
| `src/core/game_state_abilities.c:495` | `rb_resolve_constant_ability` | GLOBAL |
| `src/core/game_state_abilities.c:592` | `rb_process_with_completed_key` | GLOBAL |
| `src/core/game_state_abilities.c:1354` | `rb_effective_activation_cost` | GLOBAL |
| `src/core/game_state_abilities.c:1453` | `rb_mark_replacement_effect_applied` | GLOBAL |
| `src/core/game_state_abilities.c:1487` | `rb_inject_choice_ability_context` | GLOBAL |
| `src/core/game_state_abilities.c:1516` | `rb_entry_has_pending_choice` | GLOBAL |
| `src/core/game_state_abilities.c:1608` | `rb_entry_ability_copy` | GLOBAL |
| `src/core/game_state_abilities.c:1617` | `rb_entry_destination_copy` | GLOBAL |
| `src/core/game_state_abilities.c:1686` | `rb_entry_snapshot_last_energy_placed_by_effect` | GLOBAL |
| `src/core/game_state_abilities.c:1691` | `rb_entry_snapshot_last_energy_placed_by_player` | GLOBAL |
| `src/core/game_state_abilities.c:1696` | `rb_entry_snapshot_last_area_move_card_id` | GLOBAL |
| `src/core/game_state_abilities.c:1701` | `rb_entry_snapshot_last_area_move_by_player` | GLOBAL |
| `src/core/game_state_abilities.c:2541` | `rb_fire_all_auto` | GLOBAL |
| `src/core/game_state_abilities.c:2548` | `rb_fire_auto_and_pending` | GLOBAL |
| `src/core/game_state_abilities.c:2564` | `rb_record_event` | GLOBAL |
| `src/core/game_state_abilities.c:2570` | `rb_fire_recorded_auto` | GLOBAL |
| `src/core/game_state_abilities.c:3362` | `rb_process_player_abilities_depth` | GLOBAL |
| `src/core/game_state_abilities.c:3399` | `rb_apply_ability_effects` | GLOBAL |
| `src/core/game_state_abilities.c:3423` | `rb_collect_constant_hand` | GLOBAL |
| `src/core/game_state_abilities.c:3445` | `rb_collect_live_modifiers` | GLOBAL |
| `src/core/game_state_abilities.c:3507` | `rb_get_pending_choice_json` | GLOBAL |
| `src/core/game_state_abilities.c:3533` | `rb_entry_choice_card_no` | GLOBAL |
| `src/core/game_state_abilities.c:3538` | `rb_entry_conditional_choice` | GLOBAL |
| `src/core/game_state_abilities.c:3545` | `rb_build_ability_queue_entry_public_full` | GLOBAL |
| `src/core/game_state_abilities.c:3606` | `rb_process_player_abilities_depth_full` | GLOBAL |
| `src/core/game_state_core.c:154` | `rb_gsc_first_attacker_index` | GLOBAL |
| `src/core/game_state_core.c:155` | `rb_gsc_second_attacker_index` | GLOBAL |
| `src/core/game_state_core.c:185` | `rb_gsc_active_player` | GLOBAL |
| `src/core/game_state_core.c:191` | `rb_gsc_is_first_turn` | GLOBAL |
| `src/core/game_state_core.c:219` | `rb_gsc_phase_invariant` | GLOBAL |
| `src/core/game_state_core.c:246` | `rb_gsc_can_player_act` | GLOBAL |
| `src/core/game_state_core.c:300` | `rb_gsc_opponent_has_performed` | GLOBAL |
| `src/core/game_state_core.c:347` | `rb_gsc_mulligan_owner_index` | GLOBAL |
| `src/core/game_state_core.c:376` | `rb_gsc_concede_both` | GLOBAL |
| `src/core/game_state_core.c:387` | `rb_gsc_seat_player_mut` | GLOBAL |
| `src/core/game_state_core.c:388` | `rb_gsc_seat_player` | GLOBAL |
| `src/core/game_state_core.c:389` | `rb_gsc_const_active_player` | GLOBAL |
| `src/core/game_state_core.c:394` | `rb_gsc_seated_pair` | GLOBAL |
| `src/core/game_state_core.c:409` | `rb_gsc_try_player_by_id_mut` | GLOBAL |
| `src/core/game_state_core.c:410` | `rb_gsc_try_player_by_id` | GLOBAL |
| `src/core/game_state_core.c:415` | `rb_gsc_seat_index_by_id` | GLOBAL |
| `src/core/game_state_core.c:419` | `rb_gsc_cheer_count_by_id` | GLOBAL |
| `src/core/game_state_core.c:423` | `rb_gsc_set_cheer_count_by_id` | GLOBAL |
| `src/core/game_state_core.c:427` | `rb_gsc_seat_cheer_count` | GLOBAL |
| `src/core/game_state_core.c:433` | `rb_gsc_score_bonus_by_id` | GLOBAL |
| `src/core/game_state_core.c:437` | `rb_gsc_seat_score_bonus` | GLOBAL |
| `src/core/game_state_core.c:446` | `rb_gsc_clear_effect_tracking` | GLOBAL |
| `src/core/game_state_core.c:459` | `rb_gsc_last_area_move_card_id` | GLOBAL |
| `src/core/game_state_core.c:462` | `rb_gsc_last_area_move_by_player` | GLOBAL |
| `src/core/game_state_core.c:468` | `rb_gsc_last_cost_wait_member` | GLOBAL |
| `src/core/game_state_core.c:474` | `rb_gsc_recently_moved_cards` | GLOBAL |
| `src/core/game_state_core.c:480` | `rb_gsc_set_recently_moved_cards` | GLOBAL |
| `src/core/game_state_core.c:488` | `rb_gsc_last_energy_placed_by_effect` | GLOBAL |
| `src/core/game_state_core.c:491` | `rb_gsc_last_energy_placed_by_player` | GLOBAL |
| `src/core/game_state_core.c:497` | `rb_gsc_current_ability_source_card_id` | GLOBAL |
| `src/core/game_state_core.c:510` | `rb_gsc_push_revealed_card` | GLOBAL |
| `src/core/game_state_core.c:518` | `rb_gsc_player_prefix_id` | GLOBAL |
| `src/core/game_state_core.c:535` | `rb_gsc_zone_snapshot` | GLOBAL |
| `src/core/game_state_core.c:802` | `rb_gsc_push_rule_log_fmt` | GLOBAL |
| `src/core/game_state_core.c:821` | `rb_gsc_log_entry` | GLOBAL |
| `src/core/game_state_core.c:837` | `rb_gsc_push_choice_offered` | GLOBAL |
| `src/core/game_state_core.c:860` | `rb_gsc_push_choice_resolved` | GLOBAL |
| `src/core/game_state_core.c:874` | `rb_gsc_push_performance_snapshot` | GLOBAL |
| `src/core/game_state_core.c:883` | `rb_gsc_record_ability_application` | GLOBAL |
| `src/core/modifiers.c:164` | `rb_mods_clear_blade_set` | GLOBAL |
| `src/core/modifiers.c:172` | `rb_mods_clear_score_set` | GLOBAL |
| `src/core/modifiers.c:187` | `rb_mods_remove_blade` | GLOBAL |
| `src/core/modifiers.c:192` | `rb_mods_remove_heart` | GLOBAL |
| `src/core/modifiers.c:223` | `rb_mods_remove_score` | GLOBAL |
| `src/core/modifiers.c:263` | `rb_mods_remove_heart_override` | GLOBAL |
| `src/core/modifiers.c:290` | `rb_mods_clear_blade_type` | GLOBAL |
| `src/core/modifiers.c:630` | `rb_zone_id_to_movement_zone` | GLOBAL |
| `src/core/modifiers.c:968` | `rb_remove_revealed_card` | GLOBAL |
| `src/core/modifiers.c:1341` | `rb_clear_revealed_cards` | GLOBAL |
| `src/core/tracking.c:163` | `rb_check_required_hearts` | GLOBAL |
| `src/core/zones.c:51` | `rb_zone_total_blades` | GLOBAL |
| `src/core/zones.c:79` | `rb_zone_draw_bottom` | GLOBAL |
| `src/core/zones.c:85` | `rb_zone_draw_multiple` | GLOBAL |
| `src/core/zones.c:536` | `rb_energy_can_place_card` | GLOBAL |
| `src/core/zones.c:576` | `rb_energy_add_active` | GLOBAL |
| `src/core/zones.c:619` | `rb_live_clear` | GLOBAL |
| `src/core/zones.c:646` | `rb_hand_len` | GLOBAL |
| `src/core/zones.c:650` | `rb_hand_is_empty` | GLOBAL |
| `src/core/zones.c:661` | `rb_waitroom_take_all` | GLOBAL |
| `src/core/zones.c:669` | `rb_waitroom_shuffle` | GLOBAL |
| `src/core/zones.c:674` | `rb_waitroom_len` | GLOBAL |
| `src/core/zones.c:702` | `rb_success_len` | GLOBAL |
| `src/core/zones.c:789` | `rb_resolution_swap_slots` | GLOBAL |
| `src/core/zones.c:813` | `rb_deck_shuffle` | GLOBAL |
| `src/core/zones.c:819` | `rb_deck_is_empty` | GLOBAL |
| `src/core/zones.c:825` | `rb_deck_len` | GLOBAL |
| `src/core/zones.c:919` | `rb_stage_get_available_hearts` | GLOBAL |
| `src/core/zones.c:928` | `rb_stage_get_available_hearts_i32` | GLOBAL |
| `src/core/zones.c:968` | `rb_player_set_main_deck` | GLOBAL |
| `src/core/zones.c:989` | `rb_player_get_card_index_by_id` | GLOBAL |
| `src/core/zones.c:1000` | `rb_player_is_area_locked` | GLOBAL |
| `src/core/zones.c:1037` | `rb_player_move_card_from_hand_to_stage` | GLOBAL |
| `src/core/zones.c:1096` | `rb_player_calculate_stage_hearts` | GLOBAL |
| `src/core/zones.c:1102` | `rb_player_activate_all_energy` | GLOBAL |
| `src/core/zones.c:1110` | `rb_player_activate_all_energy_exclude` | GLOBAL |
| `src/core/zones.c:1120` | `rb_player_all_card_ids` | GLOBAL |
| `src/core/zones.c:1166` | `rb_player_draw_energy` | GLOBAL |
| `src/core/zones.c:1180` | `rb_player_contains_card` | GLOBAL |
| `src/engine.c:223` | `heart_color_of` | GLOBAL |
| `src/engine.c:1460` | `rb_play_card` | GLOBAL |
| `src/engine.c:1867` | `i18n_self_check` | GLOBAL |
| `src/game/game_setup.c:406` | `rb_setup_is_live_card_set_phase` | GLOBAL |
| `src/game/game_setup.c:413` | `rb_setup_auto_advance_one` | GLOBAL |
| `src/game/game_setup.c:449` | `rb_setup_settle_auto` | GLOBAL |
| `src/game/game_setup.c:882` | `rb_setup_execute_action_for_player` | GLOBAL |
| `src/game/game_setup.c:1401` | `rb_setup_build_two_decks` | GLOBAL |
| `src/game/game_setup.c:1523` | `rb_setup_extract_base` | GLOBAL |
| `src/game/game_setup.c:1567` | `rb_setup_flatten_and_sort_for_gba` | GLOBAL |
| `src/test_game.c:204` | `test_find_live_by_score` | GLOBAL |
| `src/test_game.c:381` | `test_expire_effects` | GLOBAL |
| `src/test_game.c:586` | `test_success_count` | GLOBAL |
| `src/test_game.c:824` | `test_zone_has_card_no` | GLOBAL |
| `src/test_game.c:1151` | `test_advance_to_phase_rust` | GLOBAL |
| `src/test_game.c:1567` | `test_flat_verdict_failed` | GLOBAL |
| `src/test_game.c:1582` | `test_flat_failed_count` | GLOBAL |
| `src/turn/live.c:2541` | `rb_try_surplus_compositions` | GLOBAL |
| `src/turn/live.c:2585` | `rb_bt_search` | GLOBAL |
| `src/turn/live.c:2614` | `rb_try_phase4` | GLOBAL |
| `src/turn/live.c:2673` | `rb_try_all_distribution` | GLOBAL |
| `src/turn/phase.c:1108` | `rb_has_distinct_assignment_k` | GLOBAL |
| `src/turn/phase.c:1315` | `rb_log_phase` | GLOBAL |
| `src/turn/phase.c:1322` | `_3ds_tdbg` | GLOBAL |
| `src/turn/phase.c:1329` | `rb_log_turn_start` | GLOBAL |
| `src/turn/phase.c:1615` | `rb_3ds_tdbg` | GLOBAL |
| `src/turn/phase.c:1747` | `rb_play_time_cost_reduction_hook` | GLOBAL |
| `src/turn/phase.c:2135` | `rb_generate_possible_actions` | GLOBAL |
| `src/turn/triggers.c:1641` | `rb_trigger_to_texticon` | GLOBAL |

### 2c. The 262 TEST-ONLY functions

Exercised by `tests/` or `tools/`, never reachable from `rb_engine`'s real
paths. **A passing test on any of these proves less than it appears to**: the
function is not in the shipped binary.

| file:line | function | storage | test/tool call sites |
|---|---|---|---|
| `include/rabuka.h:186` | `rb_saturate_i16` | static inline (hdr) | 2 |
| `include/rabuka.h:2553` | `rb_card_filter_subset` | static inline (hdr) | 1 |
| `src/ability/ability_queue.c:103` | `rb_queue_init` | GLOBAL | 2 |
| `src/ability/ability_queue.c:151` | `rb_queue_is_idle` | GLOBAL | 4 |
| `src/ability/ability_queue.c:163` | `rb_queue_is_waiting_for_choice` | GLOBAL | 4 |
| `src/ability/ability_queue.c:197` | `rb_queue_len` | GLOBAL | 10 |
| `src/ability/ability_queue.c:201` | `rb_queue_is_empty` | GLOBAL | 2 |
| `src/ability/ability_queue.c:205` | `rb_queue_has_pending` | GLOBAL | 1 |
| `src/ability/ability_queue.c:209` | `rb_queue_iter` | GLOBAL | 2 |
| `src/ability/ability_queue.c:326` | `rb_queue_start_next` | GLOBAL | 16 |
| `src/ability/ability_queue.c:626` | `rb_queue_pause_for_auto_ability_choice` | GLOBAL | 1 |
| `src/ability/ability_queue.c:650` | `rb_queue_resume_with_choice` | GLOBAL | 4 |
| `src/ability/ability_queue.c:717` | `rb_queue_push_constant_context` | GLOBAL | 2 |
| `src/ability/ability_queue.c:736` | `rb_queue_pop_constant_context` | GLOBAL | 2 |
| `src/ability/ability_queue.c:756` | `rb_queue_promote_entry` | GLOBAL | 3 |
| `src/ability/ability_queue.c:787` | `rb_queue_promote_entry_by_abs` | GLOBAL | 2 |
| `src/ability/ability_queue.c:804` | `rb_queue_set_current_entry` | GLOBAL | 2 |
| `src/ability/ability_queue.c:1073` | `rb_queue_take_resolver` | GLOBAL | 4 |
| `src/ability/ability_queue.c:1080` | `rb_queue_set_resolver` | GLOBAL | 2 |
| `src/ability/ability_queue.c:1085` | `rb_queue_has_resolver` | GLOBAL | 8 |
| `src/ability/ability_queue.c:1103` | `rb_queue_has_entry_with_id` | GLOBAL | 2 |
| `src/ability/ability_queue.c:1113` | `rb_queue_pending_entries` | GLOBAL | 1 |
| `src/ability/ability_queue.c:1132` | `rb_queue_dump_state` | GLOBAL | 2 |
| `src/ability/choice.c:5663` | `rb_choice_allow_skip` | GLOBAL | 9 |
| `src/ability/choice_renderer.c:1368` | `rb_rend_render_card_choice_grid` | GLOBAL | 2 |
| `src/ability/choice_renderer.c:1608` | `rb_rend_text_new` | GLOBAL | 4 |
| `src/ability/choice_renderer.c:1664` | `rb_rend_text_free` | GLOBAL | 18 |
| `src/ability/choice_renderer.c:1675` | `rb_rend_text_script` | GLOBAL | 4 |
| `src/ability/choice_renderer.c:1695` | `rb_rend_text_stuck` | GLOBAL | 4 |
| `src/ability/choice_renderer.c:1697` | `rb_rend_text_n_frames` | GLOBAL | 12 |
| `src/ability/choice_renderer.c:1698` | `rb_rend_text_n_lines` | GLOBAL | 1 |
| `src/ability/choice_renderer.c:1699` | `rb_rend_text_n_draws` | GLOBAL | 3 |
| `src/ability/choice_renderer.c:1700` | `rb_rend_text_overflowed` | GLOBAL | 3 |
| `src/ability/choice_renderer.c:1701` | `rb_rend_text_line` | GLOBAL | 1 |
| `src/ability/choice_renderer.c:1705` | `rb_rend_text_frame_line` | GLOBAL | 2 |
| `src/ability/choice_renderer.c:1713` | `rb_rend_text_draw` | GLOBAL | 8 |
| `src/ability/choice_renderer.c:1726` | `rb_rend_text_frame_draws` | GLOBAL | 2 |
| `src/ability/choice_renderer.c:1865` | `rb_rend_render_choice` | GLOBAL | 3 |
| `src/ability/compound.c:366` | `rb_route_conditional_branch` | GLOBAL | 4 |
| `src/ability/cost.c:1956` | `rb_validate_cost` | GLOBAL | 8 |
| `src/ability/debug.c:12` | `rb_ability_debug_set` | GLOBAL | 21 |
| `src/ability/describe.c:1776` | `rb_desc_cost_en` | GLOBAL | 1 |
| `src/ability/describe.c:1780` | `rb_desc_cost_ja` | GLOBAL | 1 |
| `src/ability/describe.c:1818` | `rb_desc_sequential_cost_en` | GLOBAL | 1 |
| `src/ability/describe.c:1823` | `rb_desc_sequential_cost_ja` | GLOBAL | 1 |
| `src/ability/describe.c:1832` | `rb_desc_zone_label` | GLOBAL | 3 |
| `src/ability/describe.c:1836` | `rb_desc_zone_label_ja` | GLOBAL | 3 |
| `src/ability/describe.c:1840` | `rb_desc_card_type_label` | GLOBAL | 3 |
| `src/ability/describe.c:1844` | `rb_desc_card_type_label_ja` | GLOBAL | 2 |
| `src/ability/describe.c:1848` | `rb_desc_state_verb` | GLOBAL | 4 |
| `src/ability/describe.c:1856` | `rb_desc_resource_label` | GLOBAL | 4 |
| `src/ability/describe.c:1860` | `rb_desc_resource_label_ja` | GLOBAL | 3 |
| `src/ability/describe.c:1864` | `rb_desc_duration_label` | GLOBAL | 7 |
| `src/ability/describe.c:1874` | `rb_desc_duration_label_ja` | GLOBAL | 6 |
| `src/ability/describe.c:2946` | `rb_desc_translate_choice_prompt_en_to_ja` | GLOBAL | 5 |
| `src/ability/effects/misc.c:2270` | `card_name` | static | 14 |
| `src/ability/effects/move.c:385` | `stage_area_of` | static | 34 |
| `src/ability/effects/state.c:2044` | `rb_card_total_hearts` | GLOBAL | 6 |
| `src/ability/enums.c:33` | `rb_target_player_from_str` | GLOBAL | 3 |
| `src/ability/enums.c:52` | `rb_target_player_as_str` | GLOBAL | 1 |
| `src/ability/enums.c:57` | `rb_action_type_from_str` | GLOBAL | 12 |
| `src/ability/enums.c:212` | `rb_action_type_label` | GLOBAL | 3 |
| `src/ability/enums.c:291` | `rb_condition_type_from_str` | GLOBAL | 4 |
| `src/ability/enums.c:329` | `rb_condition_type_to_str` | GLOBAL | 1 |
| `src/ability/enums.c:390` | `rb_select_target_kind_to_str` | GLOBAL | 2 |
| `src/ability/enums.c:431` | `rb_effect_card_type_default` | GLOBAL | 1 |
| `src/ability/enums.c:436` | `rb_effect_state_from_str` | GLOBAL | 5 |
| `src/ability/enums.c:443` | `rb_effect_state_as_str` | GLOBAL | 4 |
| `src/ability/enums.c:452` | `rb_effect_state_default` | GLOBAL | 1 |
| `src/ability/log.c:51` | `rb_log_set_enabled` | GLOBAL | 20 |
| `src/ability/log.c:223` | `rb_log_push_verdict_child` | GLOBAL | 1 |
| `src/ability/log.c:269` | `rb_log_clear_verdicts` | GLOBAL | 13 |
| `src/ability/resolver.c:298` | `rb_resolver_cached_condition_verdict` | GLOBAL | 4 |
| `src/ability/resolver.c:340` | `rb_resolver_store_condition_verdict` | GLOBAL | 2 |
| `src/ability/resolver.c:1210` | `rb_resolve_ability` | GLOBAL | 7 |
| `src/ability/resolver.c:1223` | `rb_resolver_drain_verdicts` | GLOBAL | 1 |
| `src/ability/resolver.c:1235` | `rb_resolver_push_verdict` | GLOBAL | 1 |
| `src/ability/types.c:101` | `rb_value_ref_init_literal` | GLOBAL | 2 |
| `src/ability/types.c:107` | `rb_value_ref_init_step` | GLOBAL | 4 |
| `src/ability/types.c:113` | `rb_value_ref_init_accepted` | GLOBAL | 3 |
| `src/ability/types.c:119` | `rb_value_ref_init_offset` | GLOBAL | 3 |
| `src/ability/types.c:136` | `rb_value_ref_kind_str` | GLOBAL | 1 |
| `src/ability/types.c:140` | `rb_value_ref_kind_from_str` | GLOBAL | 2 |
| `src/ability/types.c:149` | `rb_value_ref_resolve` | GLOBAL | 13 |
| `src/ability/types.c:174` | `rb_value_ref_is_literal` | GLOBAL | 1 |
| `src/ability/types.c:738` | `rb_choice_builder_new` | GLOBAL | 1 |
| `src/ability/types.c:751` | `rb_choice_builder_build` | GLOBAL | 1 |
| `src/ability/types.c:854` | `rb_choice_builder_picker` | GLOBAL | 1 |
| `src/ability/util.c:311` | `rb_card_has_blade_heart_strict` | GLOBAL | 7 |
| `src/ability/util.c:781` | `rb_card_matches_name_constraint` | GLOBAL | 3 |
| `src/ability/util.c:960` | `rb_get_zone_card_count` | GLOBAL | 2 |
| `src/ability/vm.c:635` | `rb_probe_nkeys` | GLOBAL | 2 |
| `src/ability/vm.c:636` | `rb_probe_key` | GLOBAL | 1 |
| `src/ability/vm.c:637` | `rb_probe_keycount` | GLOBAL | 1 |
| `src/ability/vm.c:646` | `rb_probe_reset` | GLOBAL | 1 |
| `src/ability/vm.c:648` | `rb_probe_ndropped_keys` | GLOBAL | 3 |
| `src/ability/vm.c:649` | `rb_probe_dropped_key` | GLOBAL | 3 |
| `src/ability/vm.c:650` | `rb_probe_dropped_tag` | GLOBAL | 3 |
| `src/ability/vm.c:1257` | `rb_decode_keywords` | GLOBAL | 2 |
| `src/ability/vm.c:1273` | `rb_count_empty_bytecode_abilities` | GLOBAL | 3 |
| `src/ability/vm.c:1328` | `rb_get_ability` | GLOBAL | 6 |
| `src/ability/vm.c:2731` | `decode_condition_direct` | GLOBAL | 6 |
| `src/ability/vm.c:2889` | `rb_decode_fallback_count` | GLOBAL | 22 |
| `src/ability/vm.c:2895` | `rb_decode_fallback_abilities` | GLOBAL | 3 |
| `src/ability/vm.c:3055` | `rb_verify_generated_tables` | GLOBAL | 3 |
| `src/core/card.c:21` | `rb_card_type_from_str` | GLOBAL | 4 |
| `src/core/card.c:29` | `rb_card_type_str` | GLOBAL | 2 |
| `src/core/card.c:147` | `rb_card_database_create_copy` | GLOBAL | 1 |
| `src/core/card.c:378` | `rb_map_series_to_group` | GLOBAL | 8 |
| `src/core/card.c:589` | `rb_card_short_label` | GLOBAL | 1 |
| `src/core/card.c:731` | `rb_effect_per_unit_heart_colors_any` | GLOBAL | 1 |
| `src/core/card.c:734` | `rb_effect_options_any` | GLOBAL | 1 |
| `src/core/card.c:753` | `rb_effect_destination_any` | GLOBAL | 2 |
| `src/core/card.c:758` | `rb_effect_is_under_self` | GLOBAL | 1 |
| `src/core/card.c:765` | `rb_effect_action_by` | GLOBAL | 1 |
| `src/core/card.c:768` | `rb_effect_picker_any` | GLOBAL | 2 |
| `src/core/card.c:771` | `rb_effect_non_stackable_any` | GLOBAL | 1 |
| `src/core/card.c:813` | `rb_card_get_card` | GLOBAL | 2 |
| `src/core/card.c:817` | `rb_card_has_trigger` | GLOBAL | 1 |
| `src/core/card.c:846` | `rb_card_triggerless_text` | GLOBAL | 2 |
| `src/core/card.c:1292` | `rb_card_fires_on_opponent_effects` | GLOBAL | 1 |
| `src/core/card.c:1339` | `rb_card_energy_cost_total` | GLOBAL | 2 |
| `src/core/card.c:1346` | `rb_card_has_optional_payment` | GLOBAL | 2 |
| `src/core/card.c:1353` | `rb_card_effective_energy_cost_total` | GLOBAL | 1 |
| `src/core/card.c:1376` | `rb_heartmap_is_empty` | GLOBAL | 3 |
| `src/core/card.c:1380` | `rb_heartmap_values_sum` | GLOBAL | 7 |
| `src/core/card.c:1398` | `rb_heartmap_contains_key` | GLOBAL | 8 |
| `src/core/card.c:1434` | `rb_heartmap_remove` | GLOBAL | 1 |
| `src/core/card.c:1455` | `rb_heartmap_clear` | GLOBAL | 1 |
| `src/core/card.c:1466` | `rb_heartmap_entry_or_default` | GLOBAL | 1 |
| `src/core/data.c:331` | `rb_card_stridx` | GLOBAL | 2 |
| `src/core/data.c:530` | `rb_card_no_scratch_layout_ok` | GLOBAL | 2 |
| `src/core/data.c:915` | `rb_zone_id_from_ability_zone` | GLOBAL | 2 |
| `src/core/data.c:922` | `rb_zone_id_to_ability_zone` | GLOBAL | 2 |
| `src/core/data.c:1129` | `rb_score_to_i32` | GLOBAL | 4 |
| `src/core/data.c:1144` | `rb_score_to_i32_rounded` | GLOBAL | 4 |
| `src/core/data.c:1167` | `rb_overridden_stat` | GLOBAL | 4 |
| `src/core/data.c:1178` | `rb_floored_cost` | GLOBAL | 4 |
| `src/core/data.c:1186` | `rb_count_u32` | GLOBAL | 2 |
| `src/core/data.c:1195` | `rb_id_u32` | GLOBAL | 3 |
| `src/core/data.c:1204` | `rb_f32_count` | GLOBAL | 2 |
| `src/core/game_state_abilities.c:67` | `rb_ability_has_remaining_uses` | GLOBAL | 15 |
| `src/core/game_state_abilities.c:1425` | `rb_is_loop_detected` | GLOBAL | 5 |
| `src/core/game_state_abilities.c:1462` | `rb_set_opponent_live_success` | GLOBAL | 1 |
| `src/core/game_state_abilities.c:1885` | `rb_trigger_auto_ability_by_index` | GLOBAL | 9 |
| `src/core/game_state_abilities.c:3373` | `rb_queue_reset` | GLOBAL | 36 |
| `src/core/game_state_abilities.c:3481` | `rb_queue_set_pending_choice` | GLOBAL | 3 |
| `src/core/modifiers.c:176` | `rb_mods_get_cost_set` | GLOBAL | 1 |
| `src/core/modifiers.c:258` | `rb_mods_set_heart_override` | GLOBAL | 3 |
| `src/core/modifiers.c:268` | `rb_mods_get_heart_override` | GLOBAL | 1 |
| `src/core/modifiers.c:316` | `rb_mods_trace_len` | GLOBAL | 7 |
| `src/core/modifiers.c:697` | `rb_get_baton_touch_count` | GLOBAL | 28 |
| `src/core/modifiers.c:926` | `rb_clear_card_movement_tracking` | GLOBAL | 2 |
| `src/core/modifiers.c:1195` | `rb_mods_weighted_zone_count` | GLOBAL | 7 |
| `src/core/modifiers.c:1260` | `rb_mods_replacement_appear_wait_applies` | GLOBAL | 8 |
| `src/core/stats_pipeline.c:243` | `rb_need_satisfied` | GLOBAL | 5 |
| `src/core/tracking.c:38` | `rb_reset_keyword_tracking` | GLOBAL | 1 |
| `src/core/tracking.c:109` | `rb_perform_cheer_check` | GLOBAL | 17 |
| `src/core/zones.c:12` | `rb_member_area_to_str` | GLOBAL | 4 |
| `src/core/zones.c:18` | `rb_member_area_front` | GLOBAL | 4 |
| `src/core/zones.c:39` | `rb_zone_invariant` | GLOBAL | 3 |
| `src/core/zones.c:118` | `rb_zone_refresh` | GLOBAL | 3 |
| `src/core/zones.c:138` | `rb_member_area_to_tag` | GLOBAL | 4 |
| `src/core/zones.c:146` | `rb_member_area_from_tag` | GLOBAL | 5 |
| `src/core/zones.c:186` | `rb_stage_get_area` | GLOBAL | 5 |
| `src/core/zones.c:190` | `rb_stage_set_area` | GLOBAL | 3 |
| `src/core/zones.c:271` | `rb_stage_position_change` | GLOBAL | 6 |
| `src/core/zones.c:371` | `rb_stage_can_place_card` | GLOBAL | 4 |
| `src/core/zones.c:413` | `rb_stage_formation_change` | GLOBAL | 6 |
| `src/core/zones.c:569` | `rb_energy_set_active_count` | GLOBAL | 20 |
| `src/core/zones.c:600` | `rb_live_can_place_card` | GLOBAL | 2 |
| `src/core/zones.c:756` | `rb_resolution_clear` | GLOBAL | 1 |
| `src/core/zones.c:765` | `rb_resolution_len` | GLOBAL | 2 |
| `src/core/zones.c:805` | `rb_member_area_from_index` | GLOBAL | 5 |
| `src/core/zones.c:842` | `rb_energy_deck_is_empty` | GLOBAL | 3 |
| `src/core/zones.c:885` | `rb_live_calculate_score` | GLOBAL | 4 |
| `src/core/zones.c:959` | `rb_exclusion_add` | GLOBAL | 2 |
| `src/core/zones.c:1154` | `rb_player_draw_card` | GLOBAL | 1 |
| `src/engine.c:22` | `rb_rng_checkpoint` | GLOBAL | 2 |
| `src/engine.c:23` | `rb_rng_restore` | GLOBAL | 1 |
| `src/engine.c:26` | `rb_lcg_new` | GLOBAL | 1 |
| `src/engine.c:34` | `rb_lcg_range` | GLOBAL | 1 |
| `src/engine.c:388` | `rb_execute_effect` | GLOBAL | 15 |
| `src/game/deck_builder.c:121` | `rb_build_deck_from_list` | GLOBAL | 2 |
| `src/game/deck_parser.c:330` | `rb_deck_status_str` | GLOBAL | 1 |
| `src/game/deck_parser.c:343` | `rb_deck_list_to_card_numbers` | GLOBAL | 1 |
| `src/test_game.c:22` | `test_id` | GLOBAL | 3009 |
| `src/test_game.c:26` | `test_new_id` | GLOBAL | 1661 |
| `src/test_game.c:58` | `test_card_copy_pool_reset` | GLOBAL | 76 |
| `src/test_game.c:86` | `test_id_ref` | GLOBAL | 1 |
| `src/test_game.c:103` | `test_id_exact` | GLOBAL | 256 |
| `src/test_game.c:122` | `test_add_to_hand` | GLOBAL | 1830 |
| `src/test_game.c:123` | `test_add_to_discard` | GLOBAL | 498 |
| `src/test_game.c:127` | `test_add_to_stage` | GLOBAL | 443 |
| `src/test_game.c:132` | `test_place_under` | GLOBAL | 103 |
| `src/test_game.c:140` | `test_add_to_success` | GLOBAL | 127 |
| `src/test_game.c:154` | `test_add_to_deck` | GLOBAL | 236 |
| `src/test_game.c:158` | `test_add_to_deck_pl` | GLOBAL | 419 |
| `src/test_game.c:164` | `test_add_to_energy_deck` | GLOBAL | 56 |
| `src/test_game.c:172` | `test_insert_deck_top` | GLOBAL | 67 |
| `src/test_game.c:180` | `test_add_to_energy` | GLOBAL | 44 |
| `src/test_game.c:191` | `test_set_energy_active` | GLOBAL | 63 |
| `src/test_game.c:195` | `test_add_to_revealed` | GLOBAL | 59 |
| `src/test_game.c:199` | `test_give_opp_energy` | GLOBAL | 7 |
| `src/test_game.c:218` | `test_give_energy` | GLOBAL | 1115 |
| `src/test_game.c:253` | `test_play_to_stage` | GLOBAL | 644 |
| `src/test_game.c:309` | `test_try_play_to_stage` | GLOBAL | 52 |
| `src/test_game.c:336` | `test_play_to_stage_baton` | GLOBAL | 1 |
| `src/test_game.c:360` | `test_play_to_stage_double_baton` | GLOBAL | 1 |
| `src/test_game.c:368` | `test_recalc` | GLOBAL | 562 |
| `src/test_game.c:369` | `test_clear_mods_for_card` | GLOBAL | 8 |
| `src/test_game.c:370` | `test_set_opp_stage` | GLOBAL | 55 |
| `src/test_game.c:375` | `test_add_to_opp_live` | GLOBAL | 17 |
| `src/test_game.c:376` | `test_add_to_opp_success` | GLOBAL | 25 |
| `src/test_game.c:380` | `test_fire_debut` | GLOBAL | 10 |
| `src/test_game.c:382` | `test_activate_ability` | GLOBAL | 287 |
| `src/test_game.c:402` | `test_spend_energy` | GLOBAL | 6 |
| `src/test_game.c:407` | `test_drain_auto_choices` | GLOBAL | 9 |
| `src/test_game.c:418` | `test_resume_choice` | GLOBAL | 91 |
| `src/test_game.c:431` | `test_select_indices` | GLOBAL | 124 |
| `src/test_game.c:436` | `test_has_pending_choice` | GLOBAL | 492 |
| `src/test_game.c:467` | `test_pending_choice_count` | GLOBAL | 20 |
| `src/test_game.c:474` | `test_set_live_card` | GLOBAL | 414 |
| `src/test_game.c:508` | `test_insert_live_card_at` | GLOBAL | 32 |
| `src/test_game.c:563` | `test_card_name` | GLOBAL | 62 |
| `src/test_game.c:578` | `test_stage_has` | GLOBAL | 23 |
| `src/test_game.c:582` | `test_hand_has` | GLOBAL | 82 |
| `src/test_game.c:587` | `test_print_board` | GLOBAL | 2 |
| `src/test_game.c:593` | `test_pending_choice_type` | GLOBAL | 200 |
| `src/test_game.c:607` | `test_get_blade_modifier` | GLOBAL | 129 |
| `src/test_game.c:608` | `test_get_score_modifier` | GLOBAL | 98 |
| `src/test_game.c:609` | `test_get_cost_modifier` | GLOBAL | 31 |
| `src/test_game.c:625` | `test_get_heart_modifier` | GLOBAL | 39 |
| `src/test_game.c:630` | `test_answer_play_cost_choice` | GLOBAL | 19 |
| `src/test_game.c:640` | `test_filler_hand` | GLOBAL | 1 |
| `src/test_game.c:662` | `test_fill_decks` | GLOBAL | 1 |
| `src/test_game.c:684` | `test_fill_energy_deck` | GLOBAL | 2 |
| `src/test_game.c:713` | `test_stage_occupant` | GLOBAL | 5 |
| `src/test_game.c:768` | `test_zone_ids` | GLOBAL | 16 |
| `src/test_game.c:778` | `test_zone_count_of_id` | GLOBAL | 36 |
| `src/test_game.c:787` | `test_total_card_count` | GLOBAL | 290 |
| `src/test_game.c:833` | `test_zone_has_id` | GLOBAL | 232 |
| `src/test_game.c:852` | `test_move_hand_to_waitroom` | GLOBAL | 5 |
| `src/test_game.c:864` | `test_move_ids_to_waitroom` | GLOBAL | 4 |
| `src/test_game.c:1083` | `test_advance_to_phase_named` | GLOBAL | 10 |
| `src/test_game.c:1118` | `test_advance_report` | GLOBAL | 3 |
| `src/test_game.c:1176` | `test_advance_to_phase` | GLOBAL | 88 |
| `src/test_game.c:1189` | `test_deck_len` | GLOBAL | 35 |
| `src/test_game.c:1190` | `test_hand_len` | GLOBAL | 46 |
| `src/test_game.c:1211` | `test_seat_id` | GLOBAL | 3 |
| `src/test_game.c:1224` | `test_queue_n_entries` | GLOBAL | 11 |
| `src/test_game.c:1244` | `test_queue_entry_player_id` | GLOBAL | 5 |
| `src/test_game.c:1251` | `test_queue_entry_choice_player_id` | GLOBAL | 4 |
| `src/test_game.c:1258` | `test_queue_entry_owned_by_seat` | GLOBAL | 5 |
| `src/test_game.c:1262` | `test_queue_entry_choice_owned_by_seat` | GLOBAL | 4 |
| `src/test_game.c:1274` | `test_max_distinct_names` | GLOBAL | 9 |
| `src/test_game.c:1503` | `test_fire_trigger_on_card_seat` | GLOBAL | 8 |
| `src/test_game.c:1508` | `test_fire_trigger_on_card` | GLOBAL | 38 |
| `src/test_game.c:1513` | `test_fire_trigger_on_card_nth` | GLOBAL | 7 |
| `src/test_game.c:1518` | `test_fire_trigger_at_index` | GLOBAL | 1 |
| `src/turn/live.c:42` | `rb_calc_stage_hearts` | GLOBAL | 20 |
| `src/turn/triggers.c:165` | `rb_trigger_from_token` | GLOBAL | 3 |

## 3. Every WIRED function and its call sites

1853 functions. The exhaustive table is machine-generated; the per-file counts
are in section 1c. A representative sample with full call-site attribution:

| file:line | function | intra-TU call sites | cross-TU referencing files |
|---|---|---|---|
| `src/core/card.c:130` | `rb_free_card` | 595 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c, src/ability/cost.c |
| `src/core/card.c:84` | `rb_decode_card_by_index` | 138,144,592 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c, src/ability/cost.c |
| `src/ability/vm.c:1338` | `rb_free_ability` | 1313 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c |
| `src/core/card.c:272` | `rb_card_num_abilities` | 660,818,1293 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c |
| `src/core/card.c:286` | `rb_decode_card_ability` | 664,821,850,1296,1341,1348 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c |
| `src/ability/choice.c:4691` | `rb_has_pending_choice` | 568,578,642,665,674,731 | src/ability/ability_queue.c, src/ability/compound.c, src/ability/effects/executor.c, src/ability/effects/look.c |
| `src/ability/debug.c:8` | `rb_ability_debug_enabled` | 84,124,173,208,233 | src/ability/ability_queue.c, src/ability/compound.c, src/ability/condition.c, src/ability/describe.c |
| `src/ability/util.c:491` | `rb_card_matches_group_str` | 567,638,1006,1018,1029,1498 | src/ability/choice.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/look.c |
| `src/core/alloc.c:19` | `rb_free` | 69,69,69,70,70,74 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/debug.c, src/ability/describe.c |
| `src/core/alloc.c:27` | `rb_strdup2` | - | src/ability/choice.c, src/ability/compound.c, src/ability/describe.c, src/ability/effects/ability.c |
| `src/ability/log.c:172` | `rb_log_push_verdict` | - | src/ability/choice.c, src/ability/cost.c, src/ability/debug.c, src/ability/effects/ability_effects.c |
| `src/ability/util.c:330` | `rb_card_matches_type` | 1005,1017,1028,1547 | src/ability/choice.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/ability_effects.c |
| `src/core/data.c:341` | `rb_card_string` | 370,418 | src/ability/choice_renderer.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/look.c |
| `src/turn/triggers.c:1326` | `rb_recalc_constants` | 1323 | src/ability/choice.c, src/ability/cost.c, src/ability/effects/ability.c, src/ability/effects/ability_effects.c |
| `src/ability/ability_queue.c:537` | `rb_queue_pause_for_choice` | - | src/ability/choice.c, src/ability/cost.c, src/ability/effects/ability_effects.c, src/ability/effects/draw.c |
| `src/ability/choice.c:5598` | `rb_emit_choice` | 3796,3883,4120 | src/ability/compound.c, src/ability/cost.c, src/ability/effects/ability_effects.c, src/ability/effects/draw.c |
| `src/core/modifiers.c:121` | `rb_mods_get_orientation` | 1285 | src/ability/condition.c, src/ability/cost.c, src/ability/describe.c, src/ability/dynamic_count.c |
| `src/core/alloc.c:218` | `rb_effect_deep_clone` | 244,250,254,255,256,257 | src/ability/ability_queue.c, src/ability/cost.c, src/ability/effects/ability.c, src/ability/effects/ability_effects.c |
| `src/core/alloc.c:12` | `rb_malloc` | 30,132,141,220 | src/ability/choice_renderer.c, src/ability/debug.c, src/ability/describe.c, src/ability/effects/ability.c |
| `src/core/card.c:38` | `rb_card_is_live` | - | src/ability/condition.c, src/ability/effects/draw.c, src/ability/effects/rules.c, src/ability/util.c |
| `src/engine.c:263` | `rb_execute_effect_ex` | 305,396,895,1215,1403,1404 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/compound.c, src/ability/cost.c |
| `src/ability/util.c:3149` | `rb_parse_heart_color` | 728,747,1640,1663,3163 | src/ability/choice.c, src/ability/condition.c, src/ability/effects/draw.c, src/ability/effects/look.c |
| `src/core/card.c:1238` | `rb_effect_filter_subset` | - | src/ability/cost.c, src/ability/dynamic_count.c, src/ability/effects/look.c, src/ability/effects/move.c |
| `src/engine.c:47` | `rb_shuffle` | 1712 | src/ability/effects/draw.c, src/ability/effects/move.c, src/ability/effects/rules.c, src/core/tracking.c |
| `src/ability/condition.c:2832` | `rb_eval_condition_for_host` | - | src/ability/choice.c, src/ability/compound.c, src/ability/effects/misc.c, src/ability/resolver.c |
| `src/core/alloc.c:67` | `rb_effect_free` | 72,73,75,76,77,78 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/cost.c, src/ability/effects/custom.c |
| `src/core/card.c:42` | `rb_card_is_energy` | - | src/ability/condition.c, src/ability/effects/draw.c, src/ability/effects/move.c, src/ability/util.c |
| `src/ability/ability_queue.c:1353` | `rb_drain_ability_queue` | - | src/ability/choice.c, src/ability/cost.c, src/core/game_state_abilities.c, src/engine.c |
| `src/ability/util.c:668` | `rb_card_matches_cost_limit` | 683,1568,1584,1586 | src/ability/choice.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/ability_effects.c |
| `src/core/card.c:400` | `rb_card_normalize_name` | 594 | src/ability/condition.c, src/ability/cost.c, src/ability/describe.c, src/ability/dynamic_count.c |
| `src/core/data.c:627` | `rb_find_card_by_no` | - | src/ability/effects/state.c, src/core/card.c, src/core/game_state_abilities.c, src/debug_umi.c |
| `src/core/data.c:65` | `rb_num_cards` | - | src/ability/effects/draw.c, src/ability/effects/rules.c, src/core/card.c, src/game/deck_builder.c |
| `src/core/game_state_abilities.c:1361` | `rb_resolve_target_player` | - | src/ability/choice.c, src/ability/cost.c, src/ability/describe.c, src/ability/effects/ability_effects.c |
| `src/ability/ability_queue.c:1637` | `rb_choice_set_route` | - | src/ability/choice.c, src/ability/effects/misc.c, src/ability/effects/move.c, src/ability/effects/move_missing.c |
| `src/ability/ability_queue.c:1229` | `rb_owner_of_card` | 1412 | src/ability/condition.c, src/ability/describe.c, src/ability/effects/ability_effects.c, src/core/game_state_abilities.c |
| `src/ability/util.c:648` | `rb_card_matches_characters` | 1610 | src/ability/choice.c, src/ability/cost.c, src/ability/effects/look.c, src/ability/effects/misc.c |
| `src/ability/util.c:929` | `rb_zone_cards` | 956,1025,1345,2157,2616 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/effects/move.c, src/ability/effects/state.c |
| `src/core/card.c:600` | `rb_ability_has_trigger` | 822 | src/ability/resolver.c, src/core/game_state_abilities.c, src/game/game_setup.c, src/turn/live.c |
| `src/core/card.c:46` | `rb_card_is_member` | - | src/ability/effects/draw.c, src/ability/effects/rules.c, src/core/modifiers.c, src/core/zones.c |
| `src/core/modifiers.c:27` | `rb_mods_clear_card` | 1668 | src/ability/choice.c, src/ability/effects/misc.c, src/ability/effects/move.c, src/ability/effects/move_missing.c |



## 2. Every DEAD function (exhaustive)

### 2a. The 53 dead `static` functions - the class a fixer wires by accident

| file:line | function | verdict | the specific divergence |
|---|---|---|---|
| `src/ability/choice_renderer.c:1514` | `t_clear_screen` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1518` | `t_println` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1522` | `t_swap_buffers` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1523` | `t_poll_input` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1534` | `t_wait_vblank` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1535` | `t_reset_vram` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1540` | `t_pressed_a` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1541` | `t_pressed_b` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1542` | `t_pressed_l` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1543` | `t_pressed_r` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1544` | `t_pressed_up` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1545` | `t_pressed_down` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1546` | `t_pressed_left` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1547` | `t_pressed_right` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1548` | `t_pressed_start` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1549` | `t_pressed_select` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1565` | `t_show_board_overlay` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1595` | `t_draw_card_image_narrow` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1598` | `t_show_detail_screen_narrow` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/choice_renderer.c:1606` | `t_open_start_menu_narrow` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/compound.c:374` | `rb_on_optional_branch` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/cost.c:119` | `eff_int` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/cost.c:2725` | `cost_count_in_source` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/cost.c:2736` | `cost_move_from_source` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:97` | `look_card_plural` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:104` | `look_zone_label` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:127` | `look_zone_label_ja` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:161` | `look_push_reveal_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:171` | `look_push_reveal_group_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:180` | `look_push_reveal_until_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:227` | `look_push_select_log` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:315` | `look_or_card_types` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:876` | `card_is_live_pred` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:886` | `card_type_pred` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/look.c:894` | `card_type_cost_pred` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/effects/move.c:134` | `mc_set_recently_moved_batch` | DEAD-WRONG | Comments claim it REPLACES the batch (mod.rs:1683). Being unreferenced, no live path performs that replacement; `g->those_cards` is therefore only ever APPENDED to. |
| `src/ability/util.c:171` | `rb_norm_group_name` | DEAD-WRONG | Rust `util.rs:699` normalises FULLWIDTH '!' (U+FF01) and micro (U+00B5). A C `norm_group_name` that the group gate needs; unreferenced, so the group gate cannot use it. |
| `src/ability/util.c:382` | `rb_debug_group_match` | DEAD-WRONG | The C body is empty. Rust `util.rs:652 debug_group_match` performs unit/group/name/series checks under `RABUKA_DEBUG_GROUP=1` + trace level. The C comment calls it a 'Mirror' and then says 'no-op in the C port'; the word Mirror is false. |
| `src/ability/util.c:1769` | `local_has_filter` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/util.c:1785` | `card_filter_matches_skip` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/util.c:1790` | `card_filter_matches_card` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/util.c:2895` | `rb_max_distinct_names_greedy` | DEAD-WRONG | Rust `util.rs:1305 max_distinct_names_greedy` returns `DistinctNamesResult{distinct, collision}`; the C returns only the count and silently DROPS `had_collision`. The C comment says 'delegates to greedy', which is true and is exactly the problem - the delegate has two outputs. |
| `src/ability/util.c:2907` | `cmp_u64` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/ability/vm.c:605` | `effect_decode_extra` | DEAD-WRONG | A bytecode decoder arm with no caller: the `extra` tag it decodes has no C consumer, so any effect `extra` the generator emits through this arm is dropped rather than stored. |
| `src/core/card.c:229` | `pair_order_cmp` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/core/game_state_abilities.c:775` | `rb_is_stale_discard_condition` | DEAD-CORRECT | Faithful to `abilities.rs:486`, including the parenthesised final conjunction; adds only a null guard Rust cannot express. |
| `src/core/game_state_abilities.c:850` | `rb_requires_this_move` | DEAD-CORRECT | Faithful to `abilities.rs:570`; `nloc >= 2` is `l.len() < 2` negated, `unwrap_or(false)` is the 0 default. |
| `src/core/game_state_abilities.c:894` | `rb_set_just_completed` | DEAD-WRONG | Rust `abilities.rs:183 set_just_completed` sets the key AND copies `recently_moved_cards` into `just_completed_moved`. The C setter writes a separate static `s_just_completed[]` side-table instead, and is itself unreferenced. |
| `src/core/game_state_abilities.c:917` | `rb_clear_just_completed` | DEAD-WRONG | The counterpart of the above and equally unreferenced. Note `g->just_completed_ability_key` IS cleared on the live path at `game_state_abilities.c:2479`, so this dead twin is a second, never-used clearing mechanism rather than a live bug. |
| `src/core/game_state_abilities.c:924` | `rb_just_completed_batch_matches` | DEAD-WRONG | Rust `abilities.rs:195` compares the stored moved-batch slice; the C compares against its private static table. Both halves of the Rust comparison are unreferenced, so the batch check is not implemented on any live path. |
| `src/core/game_state_abilities.c:950` | `rb_claim_batch_slot` | DEAD-WRONG | Rust `abilities.rs:510 claim_batch_slot` pushes into a `Vec`, which grows. The C adds `if (g->n_batch_triggered_keys >= cap) return 0;` - once the fixed array fills, the C SILENTLY DROPS triggers Rust would still fire. |
| `src/game/game_setup.c:1548` | `setup_gba_cmp` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |
| `src/turn/phase.c:1859` | `rb_ability_offer_sink` | DEAD-AND-UNREACHABLE-IN-RUST-TOO | no Rust item of this name; C-only shim |

**Adjudicated dead statics: 9 DEAD-WRONG, 2 DEAD-CORRECT, 42 DEAD-AND-UNREACHABLE-IN-RUST-TOO.**

### 2b. The 436 dead GLOBAL functions (declared in a header, called nowhere)

Every one of these is declared in `include/rabuka.h` or `include/test_game.h`
and defined exactly once in `src/`, with zero call sites in `src/`, `tests/`
and `tools/`. They read as a complete API surface; nothing can call them.

| file:line | function | storage |
|---|---|---|
| `src/ability/abilities_gen.c:984` | `bytecode_slice` | GLOBAL |
| `src/ability/ability_queue.c:307` | `rb_queue_push_with_trigger` | GLOBAL |
| `src/ability/ability_queue.c:967` | `rb_queue_save_pending_actions` | GLOBAL |
| `src/ability/choice.c:76` | `rb_get_card` | GLOBAL |
| `src/ability/choice.c:77` | `rb_card_db_unit` | GLOBAL |
| `src/ability/choice.c:85` | `rb_ability_master_id` | GLOBAL |
| `src/ability/choice.c:86` | `rb_choice_destination` | GLOBAL |
| `src/ability/choice.c:101` | `rb_compound_route_conditional_branch` | GLOBAL |
| `src/ability/choice.c:118` | `rb_effect_answers_any` | GLOBAL |
| `src/ability/choice.c:162` | `rb_effect_resource_on_select` | GLOBAL |
| `src/ability/choice.c:168` | `rb_effect_alternative_count_type_any` | GLOBAL |
| `src/ability/choice.c:174` | `rb_entry_conditional_choice_effect` | GLOBAL |
| `src/ability/choice.c:240` | `rb_resolver_build_choice_select_cards` | GLOBAL |
| `src/ability/choice.c:297` | `rb_resolver_card_name` | GLOBAL |
| `src/ability/choice.c:309` | `rb_resolver_entry_effect` | GLOBAL |
| `src/ability/choice.c:319` | `rb_resolver_look_select_finalize_dest` | GLOBAL |
| `src/ability/choice.c:395` | `rb_selection_context_mfi_count` | GLOBAL |
| `src/ability/choice.c:456` | `rb_resolver_clear_choice_state_and_resume2` | GLOBAL |
| `src/ability/choice.c:466` | `rb_resolver_reschedule_pending_choice` | GLOBAL |
| `src/ability/choice.c:492` | `rb_resolver_handle_select_cards_looked_at` | GLOBAL |
| `src/ability/choice.c:503` | `rb_resolver_move_non_selected_hand_to_deck_bottom` | GLOBAL |
| `src/ability/choice.c:576` | `rb_resolver_resume_execution_with_ctx` | GLOBAL |
| `src/ability/choice.c:737` | `rb_resolver_finalize_choice_with_ctx` | GLOBAL |
| `src/ability/choice.c:743` | `rb_resolver_reveal_selected_looked_at` | GLOBAL |
| `src/ability/choice.c:955` | `rb_resolver_provide_choice_result` | GLOBAL |
| `src/ability/choice.c:1061` | `rb_resolver_build_reprompt` | GLOBAL |
| `src/ability/choice.c:1072` | `rb_resolver_build_reprompt_full` | GLOBAL |
| `src/ability/choice.c:3054` | `rb_resolver_handle_entry_cost_reveal` | GLOBAL |
| `src/ability/choice.c:3297` | `rb_resolver_filter_discard_by_budget` | GLOBAL |
| `src/ability/choice.c:4477` | `rb_resolver_apply_effect_modification` | GLOBAL |
| `src/ability/choice.c:4519` | `rb_resolver_handle_position_destination` | GLOBAL |
| `src/ability/choice.c:5649` | `rb_gained_ability_index` | GLOBAL |
| `src/ability/choice.c:5657` | `rb_choice_description_ja` | GLOBAL |
| `src/ability/choice.c:5691` | `rb_ability_error_to_string` | GLOBAL |
| `src/ability/choice.c:5710` | `rb_resolver_handle_energy_zone_selection` | GLOBAL |
| `src/ability/choice_renderer.c:1689` | `rb_rend_text_idle` | GLOBAL |
| `src/ability/choice_renderer.c:1696` | `rb_rend_text_n_presses` | GLOBAL |
| `src/ability/compound.c:350` | `rb_compound_save_remaining` | GLOBAL |
| `src/ability/compound.c:370` | `rb_compound_route_branch` | GLOBAL |
| `src/ability/compound.c:762` | `rb_compound_choice_string` | GLOBAL |
| `src/ability/compound.c:767` | `rb_compound_choice_action` | GLOBAL |
| `src/ability/compound.c:777` | `rb_compound_handle_choice_string_selection` | GLOBAL |
| `src/ability/compound.c:811` | `rb_compound_handle_choice_string_store` | GLOBAL |
| `src/ability/condition.c:2839` | `rb_condition_allows` | GLOBAL |
| `src/ability/cost.c:2407` | `rb_cost_has_skip_prompt` | GLOBAL |
| `src/ability/cost.c:2412` | `rb_get_change_state_candidates` | GLOBAL |
| `src/ability/cost.c:2420` | `rb_pay_cost_move_cards` | GLOBAL |
| `src/ability/cost.c:2429` | `rb_pay_cost_change_state` | GLOBAL |
| `src/ability/debug.c:35` | `rb_abdebug_dropped_count` | GLOBAL |
| `src/ability/debug.c:36` | `rb_abdebug_dropped_reset` | GLOBAL |
| `src/ability/describe.c:1813` | `rb_desc_sequential_cost` | GLOBAL |
| `src/ability/describe.c:2831` | `rb_desc_cond_expectation` | GLOBAL |
| `src/ability/describe.c:2835` | `rb_desc_cond_actual` | GLOBAL |
| `src/ability/describe.c:2860` | `rb_desc_push_cond_verdict` | GLOBAL |
| `src/ability/describe.c:2897` | `rb_desc_apply_bilingual` | GLOBAL |
| `src/ability/effects/ability.c:82` | `rb_ability_ref_index` | GLOBAL |
| `src/ability/effects/ability.c:89` | `rb_ability_ref_idx` | GLOBAL |
| `src/ability/effects/ability.c:273` | `rb_ability_ref_decode` | GLOBAL |
| `src/ability/effects/ability.c:291` | `rb_ability_ref_to_arc` | GLOBAL |
| `src/ability/effects/ability.c:340` | `rb_ability_ref_cache_size` | GLOBAL |
| `src/ability/effects/draw.c:406` | `rb_draw_resolve_dynamic_count` | GLOBAL |
| `src/ability/effects/draw.c:647` | `rb_execute_draw_wrapper` | GLOBAL |
| `src/ability/effects/draw.c:1018` | `rb_resolve_gain_heart_color` | GLOBAL |
| `src/ability/effects/draw.c:1060` | `rb_make_card_effect_data` | GLOBAL |
| `src/ability/effects/look.c:833` | `rb_look_resume` | GLOBAL |
| `src/ability/effects/misc.c:1823` | `rb_misc_position_destinations` | GLOBAL |
| `src/ability/effects/misc.c:1861` | `rb_misc_finalize_formation_change` | GLOBAL |
| `src/ability/effects/move.c:1666` | `rb_move_resolve_from_standard_zone` | GLOBAL |
| `src/ability/effects/move.c:1674` | `rb_move_resolve_from_revealed_cards` | GLOBAL |
| `src/ability/effects/move.c:1725` | `rb_move_resolve_from_those_cards` | GLOBAL |
| `src/ability/effects/move.c:1763` | `rb_move_resolve_from_selected_cards` | GLOBAL |
| `src/ability/effects/move.c:1780` | `rb_move_resolve_source_revealed_cards` | GLOBAL |
| `src/ability/effects/move.c:2224` | `rb_move_from_revealed` | GLOBAL |
| `src/ability/effects/move.c:2270` | `rb_move_from_under_member` | GLOBAL |
| `src/ability/effects/move.c:2510` | `rb_move_execute_move_cards` | GLOBAL |
| `src/ability/effects/move_missing.c:490` | `rb_move_ask_optional_move_gate` | GLOBAL |
| `src/ability/effects/rules.c:140` | `rb_rules_forget` | GLOBAL |
| `src/ability/effects/rules.c:595` | `rb_rules_execute_modify_required_hearts` | GLOBAL |
| `src/ability/effects/rules.c:727` | `rb_rules_execute_modify_required_hearts_standard` | GLOBAL |
| `src/ability/effects/rules.c:762` | `rb_rules_execute_modify_required_hearts_success` | GLOBAL |
| `src/ability/effects/rules.c:805` | `rb_rules_execute_specify_heart_color` | GLOBAL |
| `src/ability/effects/rules.c:822` | `rb_rules_resolve_gain_heart_color` | GLOBAL |
| `src/ability/effects/rules.c:859` | `rb_rules_execute_choose_required_hearts` | GLOBAL |
| `src/ability/effects/rules.c:919` | `rb_rules_execute_set_cost` | GLOBAL |
| `src/ability/effects/rules.c:936` | `rb_rules_execute_modify_cost` | GLOBAL |
| `src/ability/effects/rules.c:1017` | `rb_rules_execute_activation_cost` | GLOBAL |
| `src/ability/effects/rules.c:1036` | `rb_rules_execute_set_cost_to_use` | GLOBAL |
| `src/ability/effects/rules.c:1051` | `rb_rules_position_limited_targets` | GLOBAL |
| `src/ability/effects/rules.c:1114` | `rb_rules_implicit_heart_target` | GLOBAL |
| `src/ability/effects/rules.c:1139` | `rb_rules_try_create_target_selection_choice` | GLOBAL |
| `src/ability/effects/rules.c:1304` | `rb_rules_handle_both_targets` | GLOBAL |
| `src/ability/effects/rules.c:1458` | `rb_rules_execute_select_number` | GLOBAL |
| `src/ability/effects/rules.c:1541` | `rb_rules_execute_choose_target_player` | GLOBAL |
| `src/ability/effects/rules.c:1580` | `rb_rules_execute_select_effect` | GLOBAL |
| `src/ability/effects/rules.c:1639` | `rb_rules_make_card_effect_data` | GLOBAL |
| `src/ability/effects/rules.c:1651` | `rb_rules_execute_choice` | GLOBAL |
| `src/ability/effects/rules.c:1819` | `rb_rules_execute_restriction` | GLOBAL |
| `src/ability/effects/rules.c:1932` | `rb_rules_execute_activation_restriction` | GLOBAL |
| `src/ability/effects/rules.c:1944` | `rb_rules_execute_reduce_live_card_set_limit` | GLOBAL |
| `src/ability/effects/rules.c:1956` | `rb_rules_execute_modify_limit` | GLOBAL |
| `src/ability/effects/rules.c:2005` | `rb_rules_execute_re_yell` | GLOBAL |
| `src/ability/effects/rules.c:2028` | `rb_rules_execute_perform_yell` | GLOBAL |
| `src/ability/effects/rules.c:2119` | `rb_rules_execute_modify_yell_count` | GLOBAL |
| `src/ability/effects/rules.c:2139` | `rb_rules_register_replacement_effect` | GLOBAL |
| `src/ability/effects/rules.c:2167` | `rb_rules_run_replacement_effects` | GLOBAL |
| `src/ability/effects/rules.c:2245` | `rb_rules_cannot_live_player` | GLOBAL |
| `src/ability/effects/rules.c:2251` | `rb_rules_cannot_activate_player` | GLOBAL |
| `src/ability/effects/rules.c:2257` | `rb_rules_n_delayed_prohibitions` | GLOBAL |
| `src/ability/effects/rules.c:2262` | `rb_rules_delayed_prohibition` | GLOBAL |
| `src/ability/effects/rules.c:2306` | `rb_rules_note_moved_cards` | GLOBAL |
| `src/ability/effects/rules.c:2315` | `rb_rules_note_selected_cards` | GLOBAL |
| `src/ability/effects/rules.c:2324` | `rb_rules_set_conditional_choice` | GLOBAL |
| `src/ability/effects/rules.c:2333` | `rb_rules_get_conditional_choice` | GLOBAL |
| `src/ability/effects/rules.c:2340` | `rb_rules_choice_repeats_remaining` | GLOBAL |
| `src/ability/effects/rules.c:2345` | `rb_rules_consume_choice_repeat` | GLOBAL |
| `src/ability/effects/rules.c:2353` | `rb_rules_note_keep_shuffle_selected` | GLOBAL |
| `src/ability/effects/rules.c:2362` | `rb_rules_keep_shuffle_phase` | GLOBAL |
| `src/ability/effects/rules.c:2365` | `rb_rules_keep_shuffle_count` | GLOBAL |
| `src/ability/effects/rules.c:2368` | `rb_rules_sub_choice_created` | GLOBAL |
| `src/ability/effects/rules.c:2371` | `rb_rules_count_at_save` | GLOBAL |
| `src/ability/effects/rules.c:2374` | `rb_rules_owner_executing` | GLOBAL |
| `src/ability/effects/rules.c:2377` | `rb_rules_resolving_replacement` | GLOBAL |
| `src/ability/effects/rules.c:2380` | `rb_rules_take_pending_replacement` | GLOBAL |
| `src/ability/effects/rules.c:2389` | `rb_rules_apply_pending_replacement` | GLOBAL |
| `src/ability/effects/state.c:1398` | `rb_all_blade_timing_record` | GLOBAL |
| `src/ability/effects/state.c:2072` | `rb_card_need_heart_satisfied` | GLOBAL |
| `src/ability/effects/state.c:2114` | `rb_heart_color_from_index` | GLOBAL |
| `src/ability/effects/state.c:2120` | `rb_heart_color_short_label` | GLOBAL |
| `src/ability/log.c:44` | `rb_log_dropped_count` | GLOBAL |
| `src/ability/log.c:45` | `rb_log_dropped_reset` | GLOBAL |
| `src/ability/log.c:198` | `rb_log_push_verdict_cost` | GLOBAL |
| `src/ability/log.c:249` | `rb_log_push_verdict_item` | GLOBAL |
| `src/ability/resolver.c:253` | `rb_resolver_zone_for_card` | GLOBAL |
| `src/ability/resolver.c:279` | `rb_resolver_get_pending_choice` | GLOBAL |
| `src/ability/resolver.c:283` | `rb_resolver_pending_choice` | GLOBAL |
| `src/ability/resolver.c:380` | `rb_resolver_emit_pay_skip_gate` | GLOBAL |
| `src/ability/resolver.c:679` | `rb_resolver_card_matches_type` | GLOBAL |
| `src/ability/resolver.c:682` | `rb_resolver_card_matches_cost_limit` | GLOBAL |
| `src/ability/resolver.c:686` | `rb_resolver_fmt_card` | GLOBAL |
| `src/ability/resolver.c:695` | `rb_resolver_fmt_ids` | GLOBAL |
| `src/ability/resolver.c:705` | `rb_resolver_merge_group_names` | GLOBAL |
| `src/ability/resolver.c:847` | `rb_resolver_trigger_infos` | GLOBAL |
| `src/ability/types.c:13` | `rb_step_output_init` | GLOBAL |
| `src/ability/types.c:17` | `rb_step_output_from_value` | GLOBAL |
| `src/ability/types.c:45` | `rb_step_output_has_cards` | GLOBAL |
| `src/ability/types.c:49` | `rb_step_output_value` | GLOBAL |
| `src/ability/types.c:53` | `rb_step_output_accepted` | GLOBAL |
| `src/ability/types.c:77` | `rb_step_output_to_json` | GLOBAL |
| `src/ability/types.c:170` | `rb_value_ref_merge` | GLOBAL |
| `src/ability/types.c:217` | `rb_trace_node_with_card` | GLOBAL |
| `src/ability/types.c:236` | `rb_trace_node_with_after` | GLOBAL |
| `src/ability/types.c:243` | `rb_trace_node_from_game_state` | GLOBAL |
| `src/ability/types.c:251` | `rb_trace_node_add_child` | GLOBAL |
| `src/ability/types.c:267` | `rb_effect_pipeline_new` | GLOBAL |
| `src/ability/types.c:274` | `rb_effect_pipeline_free` | GLOBAL |
| `src/ability/types.c:286` | `rb_step_state_new` | GLOBAL |
| `src/ability/types.c:311` | `rb_step_state_get` | GLOBAL |
| `src/ability/types.c:326` | `rb_step_state_free` | GLOBAL |
| `src/ability/types.c:332` | `rb_step_state_record_value` | GLOBAL |
| `src/ability/types.c:341` | `rb_step_state_record_cards` | GLOBAL |
| `src/ability/types.c:352` | `rb_ability_error_format` | GLOBAL |
| `src/ability/types.c:383` | `rb_exec_context_kind_to_str` | GLOBAL |
| `src/ability/types.c:393` | `rb_exec_context_kind_from_str` | GLOBAL |
| `src/ability/types.c:402` | `rb_las_kind_to_str` | GLOBAL |
| `src/ability/types.c:411` | `rb_las_kind_from_str` | GLOBAL |
| `src/ability/types.c:421` | `rb_full_choice_new_select_card` | GLOBAL |
| `src/ability/types.c:446` | `rb_full_choice_new_select_position` | GLOBAL |
| `src/ability/types.c:458` | `rb_full_choice_new_select_heart_color` | GLOBAL |
| `src/ability/types.c:499` | `rb_full_choice_set_aa_options` | GLOBAL |
| `src/ability/types.c:503` | `rb_full_choice_set_ls_options` | GLOBAL |
| `src/ability/types.c:507` | `rb_full_choice_description_ja` | GLOBAL |
| `src/ability/types.c:511` | `rb_full_choice_allow_skip` | GLOBAL |
| `src/ability/types.c:515` | `rb_full_choice_to_header` | GLOBAL |
| `src/ability/types.c:543` | `rb_full_choice_to_json` | GLOBAL |
| `src/ability/types.c:550` | `rb_full_choice_free` | GLOBAL |
| `src/ability/types.c:556` | `rb_choice_result_new_card_selected` | GLOBAL |
| `src/ability/types.c:566` | `rb_choice_result_new_skip` | GLOBAL |
| `src/ability/types.c:574` | `rb_choice_result_new_target` | GLOBAL |
| `src/ability/types.c:583` | `rb_choice_result_kind` | GLOBAL |
| `src/ability/types.c:587` | `rb_choice_result_is_skip` | GLOBAL |
| `src/ability/types.c:591` | `rb_choice_result_kind_to_str` | GLOBAL |
| `src/ability/types.c:605` | `rb_choice_result_kind_from_str` | GLOBAL |
| `src/ability/types.c:618` | `rb_choice_result_free` | GLOBAL |
| `src/ability/types.c:626` | `rb_choice_route_to_str` | GLOBAL |
| `src/ability/types.c:648` | `rb_choice_route_new` | GLOBAL |
| `src/ability/types.c:654` | `rb_choice_route_from_kind` | GLOBAL |
| `src/ability/types.c:666` | `rb_choice_route_kind_from_header` | GLOBAL |
| `src/ability/types.c:680` | `rb_repeat_prompt_choice` | GLOBAL |
| `src/ability/types.c:691` | `rb_trigger_event_init` | GLOBAL |
| `src/ability/types.c:695` | `rb_trigger_event_add_moved` | GLOBAL |
| `src/ability/types.c:700` | `rb_trigger_event_add_appeared` | GLOBAL |
| `src/ability/types.c:710` | `rb_trigger_event_has_moved` | GLOBAL |
| `src/ability/types.c:714` | `rb_trigger_event_has_appeared` | GLOBAL |
| `src/ability/types.c:718` | `rb_trigger_event_has_position_change` | GLOBAL |
| `src/ability/types.c:722` | `rb_trigger_event_has_energy_placed` | GLOBAL |
| `src/ability/types.c:726` | `rb_trigger_event_copy` | GLOBAL |
| `src/ability/types.c:732` | `rb_effect_spawn_context_init` | GLOBAL |
| `src/ability/types.c:760` | `rb_choice_builder_free` | GLOBAL |
| `src/ability/types.c:764` | `rb_choice_builder_card_type` | GLOBAL |
| `src/ability/types.c:769` | `rb_choice_builder_cost_limit` | GLOBAL |
| `src/ability/types.c:778` | `rb_choice_builder_cost_total` | GLOBAL |
| `src/ability/types.c:787` | `rb_choice_builder_cost_values` | GLOBAL |
| `src/ability/types.c:796` | `rb_choice_builder_group` | GLOBAL |
| `src/ability/types.c:801` | `rb_choice_builder_characters` | GLOBAL |
| `src/ability/types.c:809` | `rb_choice_builder_heart_colors` | GLOBAL |
| `src/ability/types.c:814` | `rb_choice_builder_require_all_heart_colors` | GLOBAL |
| `src/ability/types.c:819` | `rb_choice_builder_name_fragments` | GLOBAL |
| `src/ability/types.c:824` | `rb_choice_builder_destination` | GLOBAL |
| `src/ability/types.c:829` | `rb_choice_builder_discard_remaining` | GLOBAL |
| `src/ability/types.c:834` | `rb_choice_builder_is_select_action` | GLOBAL |
| `src/ability/types.c:839` | `rb_choice_builder_target_player_id` | GLOBAL |
| `src/ability/types.c:844` | `rb_choice_builder_blind` | GLOBAL |
| `src/ability/types.c:849` | `rb_choice_builder_is_reveal` | GLOBAL |
| `src/ability/types.c:859` | `rb_choice_builder_description_en` | GLOBAL |
| `src/ability/types.c:864` | `rb_choice_builder_description_ja` | GLOBAL |
| `src/ability/types.c:869` | `rb_choice_builder_filtered_indices` | GLOBAL |
| `src/ability/types.c:884` | `rb_arcstr_serialize` | GLOBAL |
| `src/ability/types.c:895` | `rb_arcstr_deserialize` | GLOBAL |
| `src/ability/types.c:906` | `rb_look_and_select_step_look` | GLOBAL |
| `src/ability/types.c:915` | `rb_look_and_select_step_select` | GLOBAL |
| `src/ability/types.c:927` | `rb_look_and_select_step_finalize` | GLOBAL |
| `src/ability/util.c:682` | `rb_card_matches_cost_limit_op` | GLOBAL |
| `src/ability/util.c:846` | `rb_target_player_label` | GLOBAL |
| `src/ability/util.c:955` | `rb_zone_card_ids` | GLOBAL |
| `src/ability/util.c:992` | `rb_count_in_zone_filtered` | GLOBAL |
| `src/ability/util.c:1334` | `rb_move_cards` | GLOBAL |
| `src/ability/util.c:1945` | `rb_matching_indices` | GLOBAL |
| `src/ability/util.c:2081` | `rb_filter_distinct` | GLOBAL |
| `src/ability/util.c:2130` | `rb_count_matching_distinct` | GLOBAL |
| `src/ability/util.c:2153` | `rb_build_candidate_pool` | GLOBAL |
| `src/ability/util.c:2169` | `rb_matching_ids_filtered` | GLOBAL |
| `src/ability/util.c:2241` | `rb_get_selection_indices` | GLOBAL |
| `src/ability/util.c:2270` | `rb_resolve_selection` | GLOBAL |
| `src/ability/util.c:2767` | `rb_calculate_per_unit_multiplier` | GLOBAL |
| `src/ability/util.c:2911` | `rb_prune_dominated` | GLOBAL |
| `src/ability/util.c:2996` | `rb_push_temporary_effect` | GLOBAL |
| `src/ability/util.c:3065` | `rb_set_card_identity` | GLOBAL |
| `src/ability/util.c:3104` | `rb_filter_from_parts` | GLOBAL |
| `src/ability/util.c:3117` | `rb_filter_from_parts_full` | GLOBAL |
| `src/ability/util.c:3134` | `rb_has_cannot_baton_touch_protection_util` | GLOBAL |
| `src/ability/util.c:3140` | `rb_exclude_self` | GLOBAL |
| `src/ability/vm.c:2794` | `build_filter` | GLOBAL |
| `src/ability/vm.c:2810` | `offset_of` | GLOBAL |
| `src/ability/vm.c:2822` | `rd_i64` | GLOBAL |
| `src/ability/vm.c:2835` | `rd_key` | GLOBAL |
| `src/ability/vm.c:2845` | `populate_from_json` | GLOBAL |
| `src/ability/vm.c:2863` | `rb_zone_as_str` | GLOBAL |
| `src/core/alloc.c:20` | `rb_alloc_reset` | GLOBAL |
| `src/core/alloc.c:92` | `rb_put` | GLOBAL |
| `src/core/alloc.c:98` | `rb_drop_value` | GLOBAL |
| `src/core/alloc_counter.c:267` | `rb_allocctr_alloc_buckets` | GLOBAL |
| `src/core/alloc_counter.c:302` | `rb_allocctr_note_alloc` | GLOBAL |
| `src/core/alloc_counter.c:306` | `rb_allocctr_note_alloc_at` | GLOBAL |
| `src/core/alloc_counter.c:394` | `rb_allocctr_trace_armed` | GLOBAL |
| `src/core/alloc_counter.c:454` | `rb_allocctr_trace_guard_release` | GLOBAL |
| `src/core/alloc_counter.c:481` | `rb_allocctr_maybe_arm` | GLOBAL |
| `src/core/alloc_counter.c:640` | `rb_allocctr_start` | GLOBAL |
| `src/core/alloc_counter.c:652` | `rb_allocctr_guard_release` | GLOBAL |
| `src/core/alloc_counter.c:680` | `rb_allocctr_free` | GLOBAL |
| `src/core/alloc_counter.c:705` | `rb_allocctr_strdup2` | GLOBAL |
| `src/core/alloc_counter.c:715` | `rb_allocctr_alloc_count` | GLOBAL |
| `src/core/alloc_counter.c:719` | `rb_allocctr_dealloc_count` | GLOBAL |
| `src/core/alloc_counter.c:723` | `rb_allocctr_live_bytes` | GLOBAL |
| `src/core/alloc_counter.c:727` | `rb_allocctr_peak_bytes` | GLOBAL |
| `src/core/alloc_counter.c:731` | `rb_allocctr_total_allocated` | GLOBAL |
| `src/core/alloc_counter.c:738` | `rb_allocctr_outstanding_allocs` | GLOBAL |
| `src/core/alloc_counter.c:744` | `rb_allocctr_unmatched_free_count` | GLOBAL |
| `src/core/alloc_counter.c:748` | `rb_allocctr_ledger_overflow_count` | GLOBAL |
| `src/core/alloc_counter.c:765` | `rb_allocctr_flag_action_display_enabled` | GLOBAL |
| `src/core/alloc_counter.c:773` | `rb_allocctr_flag_logging_enabled` | GLOBAL |
| `src/core/alloc_counter.c:777` | `rb_allocctr_flag_set_training_mode` | GLOBAL |
| `src/core/card.c:429` | `rb_parse_operation` | GLOBAL |
| `src/core/card.c:449` | `rb_card_state_str` | GLOBAL |
| `src/core/card.c:457` | `rb_card_state_from_str` | GLOBAL |
| `src/core/card.c:462` | `rb_comparison_target_str` | GLOBAL |
| `src/core/card.c:470` | `rb_comparison_target_from_str` | GLOBAL |
| `src/core/card.c:475` | `rb_card_property_str` | GLOBAL |
| `src/core/card.c:484` | `rb_card_property_from_str` | GLOBAL |
| `src/core/card.c:491` | `rb_placement_order_str` | GLOBAL |
| `src/core/card.c:495` | `rb_distinct_type_str` | GLOBAL |
| `src/core/card.c:504` | `rb_comparison_type_str` | GLOBAL |
| `src/core/card.c:515` | `rb_comparison_type_from_str` | GLOBAL |
| `src/core/card.c:524` | `rb_ability_filter_str` | GLOBAL |
| `src/core/card.c:534` | `rb_ability_filter_from_str` | GLOBAL |
| `src/core/card.c:542` | `rb_condition_target_str` | GLOBAL |
| `src/core/card.c:552` | `rb_condition_card_type_str` | GLOBAL |
| `src/core/card.c:567` | `rb_location_str` | GLOBAL |
| `src/core/data.c:219` | `rb_load_streaming` | GLOBAL |
| `src/core/data.c:667` | `rb_effect_data_clone` | GLOBAL |
| `src/core/data.c:717` | `rb_turn_phase_display` | GLOBAL |
| `src/core/data.c:745` | `rb_phase_display` | GLOBAL |
| `src/core/data.c:765` | `rb_effect_data_card_id` | GLOBAL |
| `src/core/data.c:785` | `rb_effect_data_items` | GLOBAL |
| `src/core/data.c:819` | `rb_effect_data_is_p1` | GLOBAL |
| `src/core/data.c:825` | `rb_effect_data_old_value` | GLOBAL |
| `src/core/data.c:831` | `rb_effect_data_count` | GLOBAL |
| `src/core/data.c:837` | `rb_effect_data_color` | GLOBAL |
| `src/core/data.c:851` | `rb_phase_label_jp` | GLOBAL |
| `src/core/data.c:871` | `rb_effect_type_as_str` | GLOBAL |
| `src/core/data.c:1068` | `rb_zone_matches_dest` | GLOBAL |
| `src/core/data.c:1095` | `rb_effect_data_amount` | GLOBAL |
| `src/core/game_state_abilities.c:284` | `rb_expire_gained_abilities` | GLOBAL |
| `src/core/game_state_abilities.c:471` | `rb_collect_constant_hand_effect_ids` | GLOBAL |
| `src/core/game_state_abilities.c:485` | `rb_collect_constant_stage_effect_ids` | GLOBAL |
| `src/core/game_state_abilities.c:495` | `rb_resolve_constant_ability` | GLOBAL |
| `src/core/game_state_abilities.c:592` | `rb_process_with_completed_key` | GLOBAL |
| `src/core/game_state_abilities.c:1354` | `rb_effective_activation_cost` | GLOBAL |
| `src/core/game_state_abilities.c:1453` | `rb_mark_replacement_effect_applied` | GLOBAL |
| `src/core/game_state_abilities.c:1487` | `rb_inject_choice_ability_context` | GLOBAL |
| `src/core/game_state_abilities.c:1516` | `rb_entry_has_pending_choice` | GLOBAL |
| `src/core/game_state_abilities.c:1608` | `rb_entry_ability_copy` | GLOBAL |
| `src/core/game_state_abilities.c:1617` | `rb_entry_destination_copy` | GLOBAL |
| `src/core/game_state_abilities.c:1686` | `rb_entry_snapshot_last_energy_placed_by_effect` | GLOBAL |
| `src/core/game_state_abilities.c:1691` | `rb_entry_snapshot_last_energy_placed_by_player` | GLOBAL |
| `src/core/game_state_abilities.c:1696` | `rb_entry_snapshot_last_area_move_card_id` | GLOBAL |
| `src/core/game_state_abilities.c:1701` | `rb_entry_snapshot_last_area_move_by_player` | GLOBAL |
| `src/core/game_state_abilities.c:2541` | `rb_fire_all_auto` | GLOBAL |
| `src/core/game_state_abilities.c:2548` | `rb_fire_auto_and_pending` | GLOBAL |
| `src/core/game_state_abilities.c:2564` | `rb_record_event` | GLOBAL |
| `src/core/game_state_abilities.c:2570` | `rb_fire_recorded_auto` | GLOBAL |
| `src/core/game_state_abilities.c:3362` | `rb_process_player_abilities_depth` | GLOBAL |
| `src/core/game_state_abilities.c:3399` | `rb_apply_ability_effects` | GLOBAL |
| `src/core/game_state_abilities.c:3423` | `rb_collect_constant_hand` | GLOBAL |
| `src/core/game_state_abilities.c:3445` | `rb_collect_live_modifiers` | GLOBAL |
| `src/core/game_state_abilities.c:3507` | `rb_get_pending_choice_json` | GLOBAL |
| `src/core/game_state_abilities.c:3533` | `rb_entry_choice_card_no` | GLOBAL |
| `src/core/game_state_abilities.c:3538` | `rb_entry_conditional_choice` | GLOBAL |
| `src/core/game_state_abilities.c:3545` | `rb_build_ability_queue_entry_public_full` | GLOBAL |
| `src/core/game_state_abilities.c:3606` | `rb_process_player_abilities_depth_full` | GLOBAL |
| `src/core/game_state_core.c:154` | `rb_gsc_first_attacker_index` | GLOBAL |
| `src/core/game_state_core.c:155` | `rb_gsc_second_attacker_index` | GLOBAL |
| `src/core/game_state_core.c:185` | `rb_gsc_active_player` | GLOBAL |
| `src/core/game_state_core.c:191` | `rb_gsc_is_first_turn` | GLOBAL |
| `src/core/game_state_core.c:219` | `rb_gsc_phase_invariant` | GLOBAL |
| `src/core/game_state_core.c:246` | `rb_gsc_can_player_act` | GLOBAL |
| `src/core/game_state_core.c:300` | `rb_gsc_opponent_has_performed` | GLOBAL |
| `src/core/game_state_core.c:347` | `rb_gsc_mulligan_owner_index` | GLOBAL |
| `src/core/game_state_core.c:376` | `rb_gsc_concede_both` | GLOBAL |
| `src/core/game_state_core.c:387` | `rb_gsc_seat_player_mut` | GLOBAL |
| `src/core/game_state_core.c:388` | `rb_gsc_seat_player` | GLOBAL |
| `src/core/game_state_core.c:389` | `rb_gsc_const_active_player` | GLOBAL |
| `src/core/game_state_core.c:394` | `rb_gsc_seated_pair` | GLOBAL |
| `src/core/game_state_core.c:409` | `rb_gsc_try_player_by_id_mut` | GLOBAL |
| `src/core/game_state_core.c:410` | `rb_gsc_try_player_by_id` | GLOBAL |
| `src/core/game_state_core.c:415` | `rb_gsc_seat_index_by_id` | GLOBAL |
| `src/core/game_state_core.c:419` | `rb_gsc_cheer_count_by_id` | GLOBAL |
| `src/core/game_state_core.c:423` | `rb_gsc_set_cheer_count_by_id` | GLOBAL |
| `src/core/game_state_core.c:427` | `rb_gsc_seat_cheer_count` | GLOBAL |
| `src/core/game_state_core.c:433` | `rb_gsc_score_bonus_by_id` | GLOBAL |
| `src/core/game_state_core.c:437` | `rb_gsc_seat_score_bonus` | GLOBAL |
| `src/core/game_state_core.c:446` | `rb_gsc_clear_effect_tracking` | GLOBAL |
| `src/core/game_state_core.c:459` | `rb_gsc_last_area_move_card_id` | GLOBAL |
| `src/core/game_state_core.c:462` | `rb_gsc_last_area_move_by_player` | GLOBAL |
| `src/core/game_state_core.c:468` | `rb_gsc_last_cost_wait_member` | GLOBAL |
| `src/core/game_state_core.c:474` | `rb_gsc_recently_moved_cards` | GLOBAL |
| `src/core/game_state_core.c:480` | `rb_gsc_set_recently_moved_cards` | GLOBAL |
| `src/core/game_state_core.c:488` | `rb_gsc_last_energy_placed_by_effect` | GLOBAL |
| `src/core/game_state_core.c:491` | `rb_gsc_last_energy_placed_by_player` | GLOBAL |
| `src/core/game_state_core.c:497` | `rb_gsc_current_ability_source_card_id` | GLOBAL |
| `src/core/game_state_core.c:510` | `rb_gsc_push_revealed_card` | GLOBAL |
| `src/core/game_state_core.c:518` | `rb_gsc_player_prefix_id` | GLOBAL |
| `src/core/game_state_core.c:535` | `rb_gsc_zone_snapshot` | GLOBAL |
| `src/core/game_state_core.c:802` | `rb_gsc_push_rule_log_fmt` | GLOBAL |
| `src/core/game_state_core.c:821` | `rb_gsc_log_entry` | GLOBAL |
| `src/core/game_state_core.c:837` | `rb_gsc_push_choice_offered` | GLOBAL |
| `src/core/game_state_core.c:860` | `rb_gsc_push_choice_resolved` | GLOBAL |
| `src/core/game_state_core.c:874` | `rb_gsc_push_performance_snapshot` | GLOBAL |
| `src/core/game_state_core.c:883` | `rb_gsc_record_ability_application` | GLOBAL |
| `src/core/modifiers.c:164` | `rb_mods_clear_blade_set` | GLOBAL |
| `src/core/modifiers.c:172` | `rb_mods_clear_score_set` | GLOBAL |
| `src/core/modifiers.c:187` | `rb_mods_remove_blade` | GLOBAL |
| `src/core/modifiers.c:192` | `rb_mods_remove_heart` | GLOBAL |
| `src/core/modifiers.c:223` | `rb_mods_remove_score` | GLOBAL |
| `src/core/modifiers.c:263` | `rb_mods_remove_heart_override` | GLOBAL |
| `src/core/modifiers.c:290` | `rb_mods_clear_blade_type` | GLOBAL |
| `src/core/modifiers.c:630` | `rb_zone_id_to_movement_zone` | GLOBAL |
| `src/core/modifiers.c:968` | `rb_remove_revealed_card` | GLOBAL |
| `src/core/modifiers.c:1341` | `rb_clear_revealed_cards` | GLOBAL |
| `src/core/tracking.c:163` | `rb_check_required_hearts` | GLOBAL |
| `src/core/zones.c:51` | `rb_zone_total_blades` | GLOBAL |
| `src/core/zones.c:79` | `rb_zone_draw_bottom` | GLOBAL |
| `src/core/zones.c:85` | `rb_zone_draw_multiple` | GLOBAL |
| `src/core/zones.c:536` | `rb_energy_can_place_card` | GLOBAL |
| `src/core/zones.c:576` | `rb_energy_add_active` | GLOBAL |
| `src/core/zones.c:619` | `rb_live_clear` | GLOBAL |
| `src/core/zones.c:646` | `rb_hand_len` | GLOBAL |
| `src/core/zones.c:650` | `rb_hand_is_empty` | GLOBAL |
| `src/core/zones.c:661` | `rb_waitroom_take_all` | GLOBAL |
| `src/core/zones.c:669` | `rb_waitroom_shuffle` | GLOBAL |
| `src/core/zones.c:674` | `rb_waitroom_len` | GLOBAL |
| `src/core/zones.c:702` | `rb_success_len` | GLOBAL |
| `src/core/zones.c:789` | `rb_resolution_swap_slots` | GLOBAL |
| `src/core/zones.c:813` | `rb_deck_shuffle` | GLOBAL |
| `src/core/zones.c:819` | `rb_deck_is_empty` | GLOBAL |
| `src/core/zones.c:825` | `rb_deck_len` | GLOBAL |
| `src/core/zones.c:919` | `rb_stage_get_available_hearts` | GLOBAL |
| `src/core/zones.c:928` | `rb_stage_get_available_hearts_i32` | GLOBAL |
| `src/core/zones.c:968` | `rb_player_set_main_deck` | GLOBAL |
| `src/core/zones.c:989` | `rb_player_get_card_index_by_id` | GLOBAL |
| `src/core/zones.c:1000` | `rb_player_is_area_locked` | GLOBAL |
| `src/core/zones.c:1037` | `rb_player_move_card_from_hand_to_stage` | GLOBAL |
| `src/core/zones.c:1096` | `rb_player_calculate_stage_hearts` | GLOBAL |
| `src/core/zones.c:1102` | `rb_player_activate_all_energy` | GLOBAL |
| `src/core/zones.c:1110` | `rb_player_activate_all_energy_exclude` | GLOBAL |
| `src/core/zones.c:1120` | `rb_player_all_card_ids` | GLOBAL |
| `src/core/zones.c:1166` | `rb_player_draw_energy` | GLOBAL |
| `src/core/zones.c:1180` | `rb_player_contains_card` | GLOBAL |
| `src/engine.c:223` | `heart_color_of` | GLOBAL |
| `src/engine.c:1460` | `rb_play_card` | GLOBAL |
| `src/engine.c:1867` | `i18n_self_check` | GLOBAL |
| `src/game/game_setup.c:406` | `rb_setup_is_live_card_set_phase` | GLOBAL |
| `src/game/game_setup.c:413` | `rb_setup_auto_advance_one` | GLOBAL |
| `src/game/game_setup.c:449` | `rb_setup_settle_auto` | GLOBAL |
| `src/game/game_setup.c:882` | `rb_setup_execute_action_for_player` | GLOBAL |
| `src/game/game_setup.c:1401` | `rb_setup_build_two_decks` | GLOBAL |
| `src/game/game_setup.c:1523` | `rb_setup_extract_base` | GLOBAL |
| `src/game/game_setup.c:1567` | `rb_setup_flatten_and_sort_for_gba` | GLOBAL |
| `src/test_game.c:204` | `test_find_live_by_score` | GLOBAL |
| `src/test_game.c:381` | `test_expire_effects` | GLOBAL |
| `src/test_game.c:586` | `test_success_count` | GLOBAL |
| `src/test_game.c:824` | `test_zone_has_card_no` | GLOBAL |
| `src/test_game.c:1151` | `test_advance_to_phase_rust` | GLOBAL |
| `src/test_game.c:1567` | `test_flat_verdict_failed` | GLOBAL |
| `src/test_game.c:1582` | `test_flat_failed_count` | GLOBAL |
| `src/turn/live.c:2541` | `rb_try_surplus_compositions` | GLOBAL |
| `src/turn/live.c:2585` | `rb_bt_search` | GLOBAL |
| `src/turn/live.c:2614` | `rb_try_phase4` | GLOBAL |
| `src/turn/live.c:2673` | `rb_try_all_distribution` | GLOBAL |
| `src/turn/phase.c:1108` | `rb_has_distinct_assignment_k` | GLOBAL |
| `src/turn/phase.c:1315` | `rb_log_phase` | GLOBAL |
| `src/turn/phase.c:1322` | `_3ds_tdbg` | GLOBAL |
| `src/turn/phase.c:1329` | `rb_log_turn_start` | GLOBAL |
| `src/turn/phase.c:1615` | `rb_3ds_tdbg` | GLOBAL |
| `src/turn/phase.c:1747` | `rb_play_time_cost_reduction_hook` | GLOBAL |
| `src/turn/phase.c:2135` | `rb_generate_possible_actions` | GLOBAL |
| `src/turn/triggers.c:1641` | `rb_trigger_to_texticon` | GLOBAL |

### 2c. The 262 TEST-ONLY functions

Exercised by `tests/` or `tools/`, never reachable from `rb_engine`'s real
paths. **A passing test on any of these proves less than it appears to**: the
function is not in the shipped binary.

| file:line | function | storage | test/tool call sites |
|---|---|---|---|
| `include/rabuka.h:186` | `rb_saturate_i16` | static inline (hdr) | 2 |
| `include/rabuka.h:2553` | `rb_card_filter_subset` | static inline (hdr) | 1 |
| `src/ability/ability_queue.c:103` | `rb_queue_init` | GLOBAL | 2 |
| `src/ability/ability_queue.c:151` | `rb_queue_is_idle` | GLOBAL | 4 |
| `src/ability/ability_queue.c:163` | `rb_queue_is_waiting_for_choice` | GLOBAL | 4 |
| `src/ability/ability_queue.c:197` | `rb_queue_len` | GLOBAL | 10 |
| `src/ability/ability_queue.c:201` | `rb_queue_is_empty` | GLOBAL | 2 |
| `src/ability/ability_queue.c:205` | `rb_queue_has_pending` | GLOBAL | 1 |
| `src/ability/ability_queue.c:209` | `rb_queue_iter` | GLOBAL | 2 |
| `src/ability/ability_queue.c:326` | `rb_queue_start_next` | GLOBAL | 16 |
| `src/ability/ability_queue.c:626` | `rb_queue_pause_for_auto_ability_choice` | GLOBAL | 1 |
| `src/ability/ability_queue.c:650` | `rb_queue_resume_with_choice` | GLOBAL | 4 |
| `src/ability/ability_queue.c:717` | `rb_queue_push_constant_context` | GLOBAL | 2 |
| `src/ability/ability_queue.c:736` | `rb_queue_pop_constant_context` | GLOBAL | 2 |
| `src/ability/ability_queue.c:756` | `rb_queue_promote_entry` | GLOBAL | 3 |
| `src/ability/ability_queue.c:787` | `rb_queue_promote_entry_by_abs` | GLOBAL | 2 |
| `src/ability/ability_queue.c:804` | `rb_queue_set_current_entry` | GLOBAL | 2 |
| `src/ability/ability_queue.c:1073` | `rb_queue_take_resolver` | GLOBAL | 4 |
| `src/ability/ability_queue.c:1080` | `rb_queue_set_resolver` | GLOBAL | 2 |
| `src/ability/ability_queue.c:1085` | `rb_queue_has_resolver` | GLOBAL | 8 |
| `src/ability/ability_queue.c:1103` | `rb_queue_has_entry_with_id` | GLOBAL | 2 |
| `src/ability/ability_queue.c:1113` | `rb_queue_pending_entries` | GLOBAL | 1 |
| `src/ability/ability_queue.c:1132` | `rb_queue_dump_state` | GLOBAL | 2 |
| `src/ability/choice.c:5663` | `rb_choice_allow_skip` | GLOBAL | 9 |
| `src/ability/choice_renderer.c:1368` | `rb_rend_render_card_choice_grid` | GLOBAL | 2 |
| `src/ability/choice_renderer.c:1608` | `rb_rend_text_new` | GLOBAL | 4 |
| `src/ability/choice_renderer.c:1664` | `rb_rend_text_free` | GLOBAL | 18 |
| `src/ability/choice_renderer.c:1675` | `rb_rend_text_script` | GLOBAL | 4 |
| `src/ability/choice_renderer.c:1695` | `rb_rend_text_stuck` | GLOBAL | 4 |
| `src/ability/choice_renderer.c:1697` | `rb_rend_text_n_frames` | GLOBAL | 12 |
| `src/ability/choice_renderer.c:1698` | `rb_rend_text_n_lines` | GLOBAL | 1 |
| `src/ability/choice_renderer.c:1699` | `rb_rend_text_n_draws` | GLOBAL | 3 |
| `src/ability/choice_renderer.c:1700` | `rb_rend_text_overflowed` | GLOBAL | 3 |
| `src/ability/choice_renderer.c:1701` | `rb_rend_text_line` | GLOBAL | 1 |
| `src/ability/choice_renderer.c:1705` | `rb_rend_text_frame_line` | GLOBAL | 2 |
| `src/ability/choice_renderer.c:1713` | `rb_rend_text_draw` | GLOBAL | 8 |
| `src/ability/choice_renderer.c:1726` | `rb_rend_text_frame_draws` | GLOBAL | 2 |
| `src/ability/choice_renderer.c:1865` | `rb_rend_render_choice` | GLOBAL | 3 |
| `src/ability/compound.c:366` | `rb_route_conditional_branch` | GLOBAL | 4 |
| `src/ability/cost.c:1956` | `rb_validate_cost` | GLOBAL | 8 |
| `src/ability/debug.c:12` | `rb_ability_debug_set` | GLOBAL | 21 |
| `src/ability/describe.c:1776` | `rb_desc_cost_en` | GLOBAL | 1 |
| `src/ability/describe.c:1780` | `rb_desc_cost_ja` | GLOBAL | 1 |
| `src/ability/describe.c:1818` | `rb_desc_sequential_cost_en` | GLOBAL | 1 |
| `src/ability/describe.c:1823` | `rb_desc_sequential_cost_ja` | GLOBAL | 1 |
| `src/ability/describe.c:1832` | `rb_desc_zone_label` | GLOBAL | 3 |
| `src/ability/describe.c:1836` | `rb_desc_zone_label_ja` | GLOBAL | 3 |
| `src/ability/describe.c:1840` | `rb_desc_card_type_label` | GLOBAL | 3 |
| `src/ability/describe.c:1844` | `rb_desc_card_type_label_ja` | GLOBAL | 2 |
| `src/ability/describe.c:1848` | `rb_desc_state_verb` | GLOBAL | 4 |
| `src/ability/describe.c:1856` | `rb_desc_resource_label` | GLOBAL | 4 |
| `src/ability/describe.c:1860` | `rb_desc_resource_label_ja` | GLOBAL | 3 |
| `src/ability/describe.c:1864` | `rb_desc_duration_label` | GLOBAL | 7 |
| `src/ability/describe.c:1874` | `rb_desc_duration_label_ja` | GLOBAL | 6 |
| `src/ability/describe.c:2946` | `rb_desc_translate_choice_prompt_en_to_ja` | GLOBAL | 5 |
| `src/ability/effects/misc.c:2270` | `card_name` | static | 14 |
| `src/ability/effects/move.c:385` | `stage_area_of` | static | 34 |
| `src/ability/effects/state.c:2044` | `rb_card_total_hearts` | GLOBAL | 6 |
| `src/ability/enums.c:33` | `rb_target_player_from_str` | GLOBAL | 3 |
| `src/ability/enums.c:52` | `rb_target_player_as_str` | GLOBAL | 1 |
| `src/ability/enums.c:57` | `rb_action_type_from_str` | GLOBAL | 12 |
| `src/ability/enums.c:212` | `rb_action_type_label` | GLOBAL | 3 |
| `src/ability/enums.c:291` | `rb_condition_type_from_str` | GLOBAL | 4 |
| `src/ability/enums.c:329` | `rb_condition_type_to_str` | GLOBAL | 1 |
| `src/ability/enums.c:390` | `rb_select_target_kind_to_str` | GLOBAL | 2 |
| `src/ability/enums.c:431` | `rb_effect_card_type_default` | GLOBAL | 1 |
| `src/ability/enums.c:436` | `rb_effect_state_from_str` | GLOBAL | 5 |
| `src/ability/enums.c:443` | `rb_effect_state_as_str` | GLOBAL | 4 |
| `src/ability/enums.c:452` | `rb_effect_state_default` | GLOBAL | 1 |
| `src/ability/log.c:51` | `rb_log_set_enabled` | GLOBAL | 20 |
| `src/ability/log.c:223` | `rb_log_push_verdict_child` | GLOBAL | 1 |
| `src/ability/log.c:269` | `rb_log_clear_verdicts` | GLOBAL | 13 |
| `src/ability/resolver.c:298` | `rb_resolver_cached_condition_verdict` | GLOBAL | 4 |
| `src/ability/resolver.c:340` | `rb_resolver_store_condition_verdict` | GLOBAL | 2 |
| `src/ability/resolver.c:1210` | `rb_resolve_ability` | GLOBAL | 7 |
| `src/ability/resolver.c:1223` | `rb_resolver_drain_verdicts` | GLOBAL | 1 |
| `src/ability/resolver.c:1235` | `rb_resolver_push_verdict` | GLOBAL | 1 |
| `src/ability/types.c:101` | `rb_value_ref_init_literal` | GLOBAL | 2 |
| `src/ability/types.c:107` | `rb_value_ref_init_step` | GLOBAL | 4 |
| `src/ability/types.c:113` | `rb_value_ref_init_accepted` | GLOBAL | 3 |
| `src/ability/types.c:119` | `rb_value_ref_init_offset` | GLOBAL | 3 |
| `src/ability/types.c:136` | `rb_value_ref_kind_str` | GLOBAL | 1 |
| `src/ability/types.c:140` | `rb_value_ref_kind_from_str` | GLOBAL | 2 |
| `src/ability/types.c:149` | `rb_value_ref_resolve` | GLOBAL | 13 |
| `src/ability/types.c:174` | `rb_value_ref_is_literal` | GLOBAL | 1 |
| `src/ability/types.c:738` | `rb_choice_builder_new` | GLOBAL | 1 |
| `src/ability/types.c:751` | `rb_choice_builder_build` | GLOBAL | 1 |
| `src/ability/types.c:854` | `rb_choice_builder_picker` | GLOBAL | 1 |
| `src/ability/util.c:311` | `rb_card_has_blade_heart_strict` | GLOBAL | 7 |
| `src/ability/util.c:781` | `rb_card_matches_name_constraint` | GLOBAL | 3 |
| `src/ability/util.c:960` | `rb_get_zone_card_count` | GLOBAL | 2 |
| `src/ability/vm.c:635` | `rb_probe_nkeys` | GLOBAL | 2 |
| `src/ability/vm.c:636` | `rb_probe_key` | GLOBAL | 1 |
| `src/ability/vm.c:637` | `rb_probe_keycount` | GLOBAL | 1 |
| `src/ability/vm.c:646` | `rb_probe_reset` | GLOBAL | 1 |
| `src/ability/vm.c:648` | `rb_probe_ndropped_keys` | GLOBAL | 3 |
| `src/ability/vm.c:649` | `rb_probe_dropped_key` | GLOBAL | 3 |
| `src/ability/vm.c:650` | `rb_probe_dropped_tag` | GLOBAL | 3 |
| `src/ability/vm.c:1257` | `rb_decode_keywords` | GLOBAL | 2 |
| `src/ability/vm.c:1273` | `rb_count_empty_bytecode_abilities` | GLOBAL | 3 |
| `src/ability/vm.c:1328` | `rb_get_ability` | GLOBAL | 6 |
| `src/ability/vm.c:2731` | `decode_condition_direct` | GLOBAL | 6 |
| `src/ability/vm.c:2889` | `rb_decode_fallback_count` | GLOBAL | 22 |
| `src/ability/vm.c:2895` | `rb_decode_fallback_abilities` | GLOBAL | 3 |
| `src/ability/vm.c:3055` | `rb_verify_generated_tables` | GLOBAL | 3 |
| `src/core/card.c:21` | `rb_card_type_from_str` | GLOBAL | 4 |
| `src/core/card.c:29` | `rb_card_type_str` | GLOBAL | 2 |
| `src/core/card.c:147` | `rb_card_database_create_copy` | GLOBAL | 1 |
| `src/core/card.c:378` | `rb_map_series_to_group` | GLOBAL | 8 |
| `src/core/card.c:589` | `rb_card_short_label` | GLOBAL | 1 |
| `src/core/card.c:731` | `rb_effect_per_unit_heart_colors_any` | GLOBAL | 1 |
| `src/core/card.c:734` | `rb_effect_options_any` | GLOBAL | 1 |
| `src/core/card.c:753` | `rb_effect_destination_any` | GLOBAL | 2 |
| `src/core/card.c:758` | `rb_effect_is_under_self` | GLOBAL | 1 |
| `src/core/card.c:765` | `rb_effect_action_by` | GLOBAL | 1 |
| `src/core/card.c:768` | `rb_effect_picker_any` | GLOBAL | 2 |
| `src/core/card.c:771` | `rb_effect_non_stackable_any` | GLOBAL | 1 |
| `src/core/card.c:813` | `rb_card_get_card` | GLOBAL | 2 |
| `src/core/card.c:817` | `rb_card_has_trigger` | GLOBAL | 1 |
| `src/core/card.c:846` | `rb_card_triggerless_text` | GLOBAL | 2 |
| `src/core/card.c:1292` | `rb_card_fires_on_opponent_effects` | GLOBAL | 1 |
| `src/core/card.c:1339` | `rb_card_energy_cost_total` | GLOBAL | 2 |
| `src/core/card.c:1346` | `rb_card_has_optional_payment` | GLOBAL | 2 |
| `src/core/card.c:1353` | `rb_card_effective_energy_cost_total` | GLOBAL | 1 |
| `src/core/card.c:1376` | `rb_heartmap_is_empty` | GLOBAL | 3 |
| `src/core/card.c:1380` | `rb_heartmap_values_sum` | GLOBAL | 7 |
| `src/core/card.c:1398` | `rb_heartmap_contains_key` | GLOBAL | 8 |
| `src/core/card.c:1434` | `rb_heartmap_remove` | GLOBAL | 1 |
| `src/core/card.c:1455` | `rb_heartmap_clear` | GLOBAL | 1 |
| `src/core/card.c:1466` | `rb_heartmap_entry_or_default` | GLOBAL | 1 |
| `src/core/data.c:331` | `rb_card_stridx` | GLOBAL | 2 |
| `src/core/data.c:530` | `rb_card_no_scratch_layout_ok` | GLOBAL | 2 |
| `src/core/data.c:915` | `rb_zone_id_from_ability_zone` | GLOBAL | 2 |
| `src/core/data.c:922` | `rb_zone_id_to_ability_zone` | GLOBAL | 2 |
| `src/core/data.c:1129` | `rb_score_to_i32` | GLOBAL | 4 |
| `src/core/data.c:1144` | `rb_score_to_i32_rounded` | GLOBAL | 4 |
| `src/core/data.c:1167` | `rb_overridden_stat` | GLOBAL | 4 |
| `src/core/data.c:1178` | `rb_floored_cost` | GLOBAL | 4 |
| `src/core/data.c:1186` | `rb_count_u32` | GLOBAL | 2 |
| `src/core/data.c:1195` | `rb_id_u32` | GLOBAL | 3 |
| `src/core/data.c:1204` | `rb_f32_count` | GLOBAL | 2 |
| `src/core/game_state_abilities.c:67` | `rb_ability_has_remaining_uses` | GLOBAL | 15 |
| `src/core/game_state_abilities.c:1425` | `rb_is_loop_detected` | GLOBAL | 5 |
| `src/core/game_state_abilities.c:1462` | `rb_set_opponent_live_success` | GLOBAL | 1 |
| `src/core/game_state_abilities.c:1885` | `rb_trigger_auto_ability_by_index` | GLOBAL | 9 |
| `src/core/game_state_abilities.c:3373` | `rb_queue_reset` | GLOBAL | 36 |
| `src/core/game_state_abilities.c:3481` | `rb_queue_set_pending_choice` | GLOBAL | 3 |
| `src/core/modifiers.c:176` | `rb_mods_get_cost_set` | GLOBAL | 1 |
| `src/core/modifiers.c:258` | `rb_mods_set_heart_override` | GLOBAL | 3 |
| `src/core/modifiers.c:268` | `rb_mods_get_heart_override` | GLOBAL | 1 |
| `src/core/modifiers.c:316` | `rb_mods_trace_len` | GLOBAL | 7 |
| `src/core/modifiers.c:697` | `rb_get_baton_touch_count` | GLOBAL | 28 |
| `src/core/modifiers.c:926` | `rb_clear_card_movement_tracking` | GLOBAL | 2 |
| `src/core/modifiers.c:1195` | `rb_mods_weighted_zone_count` | GLOBAL | 7 |
| `src/core/modifiers.c:1260` | `rb_mods_replacement_appear_wait_applies` | GLOBAL | 8 |
| `src/core/stats_pipeline.c:243` | `rb_need_satisfied` | GLOBAL | 5 |
| `src/core/tracking.c:38` | `rb_reset_keyword_tracking` | GLOBAL | 1 |
| `src/core/tracking.c:109` | `rb_perform_cheer_check` | GLOBAL | 17 |
| `src/core/zones.c:12` | `rb_member_area_to_str` | GLOBAL | 4 |
| `src/core/zones.c:18` | `rb_member_area_front` | GLOBAL | 4 |
| `src/core/zones.c:39` | `rb_zone_invariant` | GLOBAL | 3 |
| `src/core/zones.c:118` | `rb_zone_refresh` | GLOBAL | 3 |
| `src/core/zones.c:138` | `rb_member_area_to_tag` | GLOBAL | 4 |
| `src/core/zones.c:146` | `rb_member_area_from_tag` | GLOBAL | 5 |
| `src/core/zones.c:186` | `rb_stage_get_area` | GLOBAL | 5 |
| `src/core/zones.c:190` | `rb_stage_set_area` | GLOBAL | 3 |
| `src/core/zones.c:271` | `rb_stage_position_change` | GLOBAL | 6 |
| `src/core/zones.c:371` | `rb_stage_can_place_card` | GLOBAL | 4 |
| `src/core/zones.c:413` | `rb_stage_formation_change` | GLOBAL | 6 |
| `src/core/zones.c:569` | `rb_energy_set_active_count` | GLOBAL | 20 |
| `src/core/zones.c:600` | `rb_live_can_place_card` | GLOBAL | 2 |
| `src/core/zones.c:756` | `rb_resolution_clear` | GLOBAL | 1 |
| `src/core/zones.c:765` | `rb_resolution_len` | GLOBAL | 2 |
| `src/core/zones.c:805` | `rb_member_area_from_index` | GLOBAL | 5 |
| `src/core/zones.c:842` | `rb_energy_deck_is_empty` | GLOBAL | 3 |
| `src/core/zones.c:885` | `rb_live_calculate_score` | GLOBAL | 4 |
| `src/core/zones.c:959` | `rb_exclusion_add` | GLOBAL | 2 |
| `src/core/zones.c:1154` | `rb_player_draw_card` | GLOBAL | 1 |
| `src/engine.c:22` | `rb_rng_checkpoint` | GLOBAL | 2 |
| `src/engine.c:23` | `rb_rng_restore` | GLOBAL | 1 |
| `src/engine.c:26` | `rb_lcg_new` | GLOBAL | 1 |
| `src/engine.c:34` | `rb_lcg_range` | GLOBAL | 1 |
| `src/engine.c:388` | `rb_execute_effect` | GLOBAL | 15 |
| `src/game/deck_builder.c:121` | `rb_build_deck_from_list` | GLOBAL | 2 |
| `src/game/deck_parser.c:330` | `rb_deck_status_str` | GLOBAL | 1 |
| `src/game/deck_parser.c:343` | `rb_deck_list_to_card_numbers` | GLOBAL | 1 |
| `src/test_game.c:22` | `test_id` | GLOBAL | 3009 |
| `src/test_game.c:26` | `test_new_id` | GLOBAL | 1661 |
| `src/test_game.c:58` | `test_card_copy_pool_reset` | GLOBAL | 76 |
| `src/test_game.c:86` | `test_id_ref` | GLOBAL | 1 |
| `src/test_game.c:103` | `test_id_exact` | GLOBAL | 256 |
| `src/test_game.c:122` | `test_add_to_hand` | GLOBAL | 1830 |
| `src/test_game.c:123` | `test_add_to_discard` | GLOBAL | 498 |
| `src/test_game.c:127` | `test_add_to_stage` | GLOBAL | 443 |
| `src/test_game.c:132` | `test_place_under` | GLOBAL | 103 |
| `src/test_game.c:140` | `test_add_to_success` | GLOBAL | 127 |
| `src/test_game.c:154` | `test_add_to_deck` | GLOBAL | 236 |
| `src/test_game.c:158` | `test_add_to_deck_pl` | GLOBAL | 419 |
| `src/test_game.c:164` | `test_add_to_energy_deck` | GLOBAL | 56 |
| `src/test_game.c:172` | `test_insert_deck_top` | GLOBAL | 67 |
| `src/test_game.c:180` | `test_add_to_energy` | GLOBAL | 44 |
| `src/test_game.c:191` | `test_set_energy_active` | GLOBAL | 63 |
| `src/test_game.c:195` | `test_add_to_revealed` | GLOBAL | 59 |
| `src/test_game.c:199` | `test_give_opp_energy` | GLOBAL | 7 |
| `src/test_game.c:218` | `test_give_energy` | GLOBAL | 1115 |
| `src/test_game.c:253` | `test_play_to_stage` | GLOBAL | 644 |
| `src/test_game.c:309` | `test_try_play_to_stage` | GLOBAL | 52 |
| `src/test_game.c:336` | `test_play_to_stage_baton` | GLOBAL | 1 |
| `src/test_game.c:360` | `test_play_to_stage_double_baton` | GLOBAL | 1 |
| `src/test_game.c:368` | `test_recalc` | GLOBAL | 562 |
| `src/test_game.c:369` | `test_clear_mods_for_card` | GLOBAL | 8 |
| `src/test_game.c:370` | `test_set_opp_stage` | GLOBAL | 55 |
| `src/test_game.c:375` | `test_add_to_opp_live` | GLOBAL | 17 |
| `src/test_game.c:376` | `test_add_to_opp_success` | GLOBAL | 25 |
| `src/test_game.c:380` | `test_fire_debut` | GLOBAL | 10 |
| `src/test_game.c:382` | `test_activate_ability` | GLOBAL | 287 |
| `src/test_game.c:402` | `test_spend_energy` | GLOBAL | 6 |
| `src/test_game.c:407` | `test_drain_auto_choices` | GLOBAL | 9 |
| `src/test_game.c:418` | `test_resume_choice` | GLOBAL | 91 |
| `src/test_game.c:431` | `test_select_indices` | GLOBAL | 124 |
| `src/test_game.c:436` | `test_has_pending_choice` | GLOBAL | 492 |
| `src/test_game.c:467` | `test_pending_choice_count` | GLOBAL | 20 |
| `src/test_game.c:474` | `test_set_live_card` | GLOBAL | 414 |
| `src/test_game.c:508` | `test_insert_live_card_at` | GLOBAL | 32 |
| `src/test_game.c:563` | `test_card_name` | GLOBAL | 62 |
| `src/test_game.c:578` | `test_stage_has` | GLOBAL | 23 |
| `src/test_game.c:582` | `test_hand_has` | GLOBAL | 82 |
| `src/test_game.c:587` | `test_print_board` | GLOBAL | 2 |
| `src/test_game.c:593` | `test_pending_choice_type` | GLOBAL | 200 |
| `src/test_game.c:607` | `test_get_blade_modifier` | GLOBAL | 129 |
| `src/test_game.c:608` | `test_get_score_modifier` | GLOBAL | 98 |
| `src/test_game.c:609` | `test_get_cost_modifier` | GLOBAL | 31 |
| `src/test_game.c:625` | `test_get_heart_modifier` | GLOBAL | 39 |
| `src/test_game.c:630` | `test_answer_play_cost_choice` | GLOBAL | 19 |
| `src/test_game.c:640` | `test_filler_hand` | GLOBAL | 1 |
| `src/test_game.c:662` | `test_fill_decks` | GLOBAL | 1 |
| `src/test_game.c:684` | `test_fill_energy_deck` | GLOBAL | 2 |
| `src/test_game.c:713` | `test_stage_occupant` | GLOBAL | 5 |
| `src/test_game.c:768` | `test_zone_ids` | GLOBAL | 16 |
| `src/test_game.c:778` | `test_zone_count_of_id` | GLOBAL | 36 |
| `src/test_game.c:787` | `test_total_card_count` | GLOBAL | 290 |
| `src/test_game.c:833` | `test_zone_has_id` | GLOBAL | 232 |
| `src/test_game.c:852` | `test_move_hand_to_waitroom` | GLOBAL | 5 |
| `src/test_game.c:864` | `test_move_ids_to_waitroom` | GLOBAL | 4 |
| `src/test_game.c:1083` | `test_advance_to_phase_named` | GLOBAL | 10 |
| `src/test_game.c:1118` | `test_advance_report` | GLOBAL | 3 |
| `src/test_game.c:1176` | `test_advance_to_phase` | GLOBAL | 88 |
| `src/test_game.c:1189` | `test_deck_len` | GLOBAL | 35 |
| `src/test_game.c:1190` | `test_hand_len` | GLOBAL | 46 |
| `src/test_game.c:1211` | `test_seat_id` | GLOBAL | 3 |
| `src/test_game.c:1224` | `test_queue_n_entries` | GLOBAL | 11 |
| `src/test_game.c:1244` | `test_queue_entry_player_id` | GLOBAL | 5 |
| `src/test_game.c:1251` | `test_queue_entry_choice_player_id` | GLOBAL | 4 |
| `src/test_game.c:1258` | `test_queue_entry_owned_by_seat` | GLOBAL | 5 |
| `src/test_game.c:1262` | `test_queue_entry_choice_owned_by_seat` | GLOBAL | 4 |
| `src/test_game.c:1274` | `test_max_distinct_names` | GLOBAL | 9 |
| `src/test_game.c:1503` | `test_fire_trigger_on_card_seat` | GLOBAL | 8 |
| `src/test_game.c:1508` | `test_fire_trigger_on_card` | GLOBAL | 38 |
| `src/test_game.c:1513` | `test_fire_trigger_on_card_nth` | GLOBAL | 7 |
| `src/test_game.c:1518` | `test_fire_trigger_at_index` | GLOBAL | 1 |
| `src/turn/live.c:42` | `rb_calc_stage_hearts` | GLOBAL | 20 |
| `src/turn/triggers.c:165` | `rb_trigger_from_token` | GLOBAL | 3 |

## 3. Every WIRED function and its call sites

1853 functions. The exhaustive table is machine-generated; the per-file counts
are in section 1c. A representative sample with full call-site attribution:

| file:line | function | intra-TU call sites | cross-TU referencing files |
|---|---|---|---|
| `src/core/card.c:130` | `rb_free_card` | 595 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c, src/ability/cost.c |
| `src/core/card.c:84` | `rb_decode_card_by_index` | 138,144,592 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c, src/ability/cost.c |
| `src/ability/vm.c:1338` | `rb_free_ability` | 1313 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c |
| `src/core/card.c:272` | `rb_card_num_abilities` | 660,818,1293 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c |
| `src/core/card.c:286` | `rb_decode_card_ability` | 664,821,850,1296,1341,1348 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/choice_renderer.c, src/ability/condition.c |
| `src/ability/choice.c:4691` | `rb_has_pending_choice` | 568,578,642,665,674,731 | src/ability/ability_queue.c, src/ability/compound.c, src/ability/effects/executor.c, src/ability/effects/look.c |
| `src/ability/debug.c:8` | `rb_ability_debug_enabled` | 84,124,173,208,233 | src/ability/ability_queue.c, src/ability/compound.c, src/ability/condition.c, src/ability/describe.c |
| `src/ability/util.c:491` | `rb_card_matches_group_str` | 567,638,1006,1018,1029,1498 | src/ability/choice.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/look.c |
| `src/core/alloc.c:19` | `rb_free` | 69,69,69,70,70,74 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/debug.c, src/ability/describe.c |
| `src/core/alloc.c:27` | `rb_strdup2` | - | src/ability/choice.c, src/ability/compound.c, src/ability/describe.c, src/ability/effects/ability.c |
| `src/ability/log.c:172` | `rb_log_push_verdict` | - | src/ability/choice.c, src/ability/cost.c, src/ability/debug.c, src/ability/effects/ability_effects.c |
| `src/ability/util.c:330` | `rb_card_matches_type` | 1005,1017,1028,1547 | src/ability/choice.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/ability_effects.c |
| `src/core/data.c:341` | `rb_card_string` | 370,418 | src/ability/choice_renderer.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/look.c |
| `src/turn/triggers.c:1326` | `rb_recalc_constants` | 1323 | src/ability/choice.c, src/ability/cost.c, src/ability/effects/ability.c, src/ability/effects/ability_effects.c |
| `src/ability/ability_queue.c:537` | `rb_queue_pause_for_choice` | - | src/ability/choice.c, src/ability/cost.c, src/ability/effects/ability_effects.c, src/ability/effects/draw.c |
| `src/ability/choice.c:5598` | `rb_emit_choice` | 3796,3883,4120 | src/ability/compound.c, src/ability/cost.c, src/ability/effects/ability_effects.c, src/ability/effects/draw.c |
| `src/core/modifiers.c:121` | `rb_mods_get_orientation` | 1285 | src/ability/condition.c, src/ability/cost.c, src/ability/describe.c, src/ability/dynamic_count.c |
| `src/core/alloc.c:218` | `rb_effect_deep_clone` | 244,250,254,255,256,257 | src/ability/ability_queue.c, src/ability/cost.c, src/ability/effects/ability.c, src/ability/effects/ability_effects.c |
| `src/core/alloc.c:12` | `rb_malloc` | 30,132,141,220 | src/ability/choice_renderer.c, src/ability/debug.c, src/ability/describe.c, src/ability/effects/ability.c |
| `src/core/card.c:38` | `rb_card_is_live` | - | src/ability/condition.c, src/ability/effects/draw.c, src/ability/effects/rules.c, src/ability/util.c |
| `src/engine.c:263` | `rb_execute_effect_ex` | 305,396,895,1215,1403,1404 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/compound.c, src/ability/cost.c |
| `src/ability/util.c:3149` | `rb_parse_heart_color` | 728,747,1640,1663,3163 | src/ability/choice.c, src/ability/condition.c, src/ability/effects/draw.c, src/ability/effects/look.c |
| `src/core/card.c:1238` | `rb_effect_filter_subset` | - | src/ability/cost.c, src/ability/dynamic_count.c, src/ability/effects/look.c, src/ability/effects/move.c |
| `src/engine.c:47` | `rb_shuffle` | 1712 | src/ability/effects/draw.c, src/ability/effects/move.c, src/ability/effects/rules.c, src/core/tracking.c |
| `src/ability/condition.c:2832` | `rb_eval_condition_for_host` | - | src/ability/choice.c, src/ability/compound.c, src/ability/effects/misc.c, src/ability/resolver.c |
| `src/core/alloc.c:67` | `rb_effect_free` | 72,73,75,76,77,78 | src/ability/ability_queue.c, src/ability/choice.c, src/ability/cost.c, src/ability/effects/custom.c |
| `src/core/card.c:42` | `rb_card_is_energy` | - | src/ability/condition.c, src/ability/effects/draw.c, src/ability/effects/move.c, src/ability/util.c |
| `src/ability/ability_queue.c:1353` | `rb_drain_ability_queue` | - | src/ability/choice.c, src/ability/cost.c, src/core/game_state_abilities.c, src/engine.c |
| `src/ability/util.c:668` | `rb_card_matches_cost_limit` | 683,1568,1584,1586 | src/ability/choice.c, src/ability/condition.c, src/ability/cost.c, src/ability/effects/ability_effects.c |
| `src/core/card.c:400` | `rb_card_normalize_name` | 594 | src/ability/condition.c, src/ability/cost.c, src/ability/describe.c, src/ability/dynamic_count.c |
| `src/core/data.c:627` | `rb_find_card_by_no` | - | src/ability/effects/state.c, src/core/card.c, src/core/game_state_abilities.c, src/debug_umi.c |
| `src/core/data.c:65` | `rb_num_cards` | - | src/ability/effects/draw.c, src/ability/effects/rules.c, src/core/card.c, src/game/deck_builder.c |
| `src/core/game_state_abilities.c:1361` | `rb_resolve_target_player` | - | src/ability/choice.c, src/ability/cost.c, src/ability/describe.c, src/ability/effects/ability_effects.c |
| `src/ability/ability_queue.c:1637` | `rb_choice_set_route` | - | src/ability/choice.c, src/ability/effects/misc.c, src/ability/effects/move.c, src/ability/effects/move_missing.c |
| `src/ability/ability_queue.c:1229` | `rb_owner_of_card` | 1412 | src/ability/condition.c, src/ability/describe.c, src/ability/effects/ability_effects.c, src/core/game_state_abilities.c |
| `src/ability/util.c:648` | `rb_card_matches_characters` | 1610 | src/ability/choice.c, src/ability/cost.c, src/ability/effects/look.c, src/ability/effects/misc.c |
| `src/ability/util.c:929` | `rb_zone_cards` | 956,1025,1345,2157,2616 | src/ability/choice.c, src/ability/choice_renderer.c, src/ability/effects/move.c, src/ability/effects/state.c |
| `src/core/card.c:600` | `rb_ability_has_trigger` | 822 | src/ability/resolver.c, src/core/game_state_abilities.c, src/game/game_setup.c, src/turn/live.c |
| `src/core/card.c:46` | `rb_card_is_member` | - | src/ability/effects/draw.c, src/ability/effects/rules.c, src/core/modifiers.c, src/core/zones.c |
| `src/core/modifiers.c:27` | `rb_mods_clear_card` | 1668 | src/ability/choice.c, src/ability/effects/misc.c, src/ability/effects/move.c, src/ability/effects/move_missing.c |

