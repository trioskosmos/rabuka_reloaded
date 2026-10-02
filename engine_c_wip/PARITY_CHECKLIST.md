# Parity Closure Checklist — engine_c_wip vs Rust

Every item here was diagnosed and measured by a peer agent. **None is "someday".**
Each has an owner file. An agent closing one must either land it or prove it
impossible with evidence. No report-only outcomes.

Status legend: `[ ]` open · `[~]` in progress · `[x]` closed

---

## A. Header — include/rabuka.h (owned by `main`)

- [x] A1 `choice_route_raw[64]` + `has_choice_route_raw` on RbQueueEntry
- [x] A2 `RbSetupActionType` + `RbSetupAction` moved out of game_setup.c
- [x] A3 `RB_COND_UNSUPPORTED = 20` on RbConditionVariant
- [x] A4 `rule_log[500][192]` + `n_rule_log` on GameState
- [x] A5 `RbMemberContribution` struct
- [x] A6 `performance_need_heart_modifiers[]` on RbLiveSnapshot
- [x] A7 `cards_paid_cost_this_turn[]` + `_cheer_revealed_cards` + `revealed_cost_cards` RELOCATED from RbLiveSnapshot to GameState
- [x] A8 `turn_area_movements[]`, `opp_cause_fired_keys[]`, `cards_activated_this_turn[]`
- [x] A9 `cause_card_id` on RbBatchMovement
- [x] A10 `self_no_excess_heart_this_turn` + `opponent_live_no_excess_heart_this_turn`
- [x] A11 description{,_en,_ja} 128 -> 512 (RbChoice + RbFullChoice)
- [x] A12 `rb_card_has_draw_icon` prototype
- [x] A13 `rb_ability_ref_flush` / `_flush_slot` — definitions were already deleted from src/ability/ability.c; removed the two orphan declarations and the stale comment that told callers to use them. Header re-verified with `gcc -fsyntax-only`.
- [x] A14 12 prototypes: rb_score_to_i32, _rounded, rb_effective_stat, rb_overridden_stat, rb_floored_cost, rb_count_u32, rb_id_u32, rb_f32_count, rb_phase_is_live_phase, rb_zone_matches_source, _dest, rb_step_output_to_json
- [x] A15 `rb_process_yell_revealed_card_icons` + `rb_fmt_player_id` exports
- [x] A16 `rb_validate_mandatory_cost`
- [x] A17 six CardFilter carriers: original_heart_total(+op), exclude_group_names, cost_limit_min/max, groups[], exclude_cards[], exclude_names[]
- [x] A18 `member_contributions[2][RB_STAGE_SIZE]` + `n_member_contributions[2]` on RbLiveSnapshot. Required MOVING the `RbMemberContribution` typedef from ~line 3431 to ~line 1253 (C needs the complete type before the enclosing struct); that was the actual reason live.c used a file-static. Header re-verified.
- [~] A19 `spare` / `base_score_total` / `card_bonus_total` / `transform_delta` — fields exist on RbMemberContribution and are now REACHABLE via A18; the `finalize_snapshot_fields` + `enrich_from_applications` port is cluster I.
- [x] A20 header declares all four (`rb_group_name_first`, `rb_effect_group_name`, `rb_card_matches_effect_group`, `rb_get_selection_indices_filter`). Note the real path is `src/ability/util.c`, not `src/core/util.c`; `rb_get_selection_indices_filter` had only tentative forward declarations in util.c:887 and effects/move.c:104 with its single definition at util.c:2070. The four local twins in cost.c / score.c / misc.c / ability_effects.c are still to be deleted by their file owners.

---

## B. Queue / choice

- [~] B1 choice.c:4907 bare `cur++; n_entries = 0;` -> `rb_queue_complete_current` + `!rb_has_pending_choice` guard (agent running)
- [ ] B2 choice.rs:815-865 same-unit remainder re-prompt -> choice.c:2136-2146 must pass filtered_indices for the CHOSEN unit and take cost_count from `cost->count`, not `pending->count`
- [ ] B3 choice.c:1120 excludes same-unit costs from `choice_cost_opens_hand_prompt` — verify against Rust
- [ ] B4 rb_resolver_spawn_target (choice.c:286) — wire or delete with Rust proof
- [ ] B5 choice.c:4907 adjacent: `rb_queue_clear_completed` + `rb_queue_is_entry_available` + `rb_queue_entry_player_id` externs should be hoisted to A20-style header decls

## C. Cost handlers

- [ ] C1 game_setup.c:1199 uses `rb_validate_cost`; Rust passes `Some(card_id)` -> switch to `rb_validate_mandatory_cost` (game_setup.rs:2188, :2295). NOTE: strictly weaker, so this REMOVES refusals — expect movement.
- [ ] C2 engine.c ~1125 `activate_resolved_ability`: ADD `rb_validate_mandatory_cost` (turn/actions/mod.rs:558-564)
- [ ] C3 `hand_cost_matching_indices` `distinct` dedupe (handlers.rs:476-480). Unreachable today (0 of 138 costs carry `distinct`).
- [ ] C4 `pay_cost_change_state` handles `rest`/`rested` (cost.c ~1520) where Rust matches only `"wait"` (handlers.rs:895-906)
- [ ] C5 delete 4 dead wrappers + their rabuka.h lines: rb_cost_has_skip_prompt, rb_get_change_state_candidates, rb_pay_cost_move_cards, rb_pay_cost_change_state (all Rust-`private`, 0 external callers)
- [ ] C6 `s_stage_select_intent` — 3 writers, 0 readers. Needs a reader for Rust's `awaiting.stage_select_intent`.
- [ ] C7 `pay_cost_move_cards` non-hand optional gate: Rust has none in handlers.rs; the E3 shape needs it from move/selection.rs:134 on the EFFECT side.

## D. Condition

- [~] D1 `evaluate_front_comparison` (card.rs:406) — called at card.rs:87-89. ~20 lines, no external dep. (agent running)
- [ ] D2 `check_heart_type_all_per_card` (predicates.rs:68) — needs faithful `card_has_all_heart` (predicates.rs:17): base-heart Heart00 membership + `heart_modifier(cid, All) > 0` + `constant_heart_bonuses[cid][All] > 0`
- [ ] D3 wire `rb_check_heart_type_all` / `rb_check_heart_colors` — needs Rust's negation-skip exceptions at condition.rs:610-623 (C only exempts one of three). `heart_type:"all"` is COMMON, so wiring without exception 3 double-applies negation.
- [ ] D4 `evaluate_comparison_condition` (comparison.rs:47): add `paid_cost_this_turn`(:60), `energy_relative`(:71), `cost`+`activating_card`(:74), `original_value`+`score`(:81), `position=="front"`(:87), `cost_total`(:139), `revealed_cards` post-check(:143), and Rust's own double-negation at :153-157. NEEDS a corpus census of "Comparison + negation" first.
- [ ] D5 `eval_highest_cost` — comparison.rs:458 uses the `position` slot and `effective_cost`; C hardcodes `stage[1]` and printed cost
- [ ] D6 `get_count_for_condition` full port — card.rs:3974-4198 (225 lines) vs a much smaller C twin. Needs total_effective_cost / heart_count_at_or_all / count_for_preceding_moved_score split out + `comparison_type=="cost"` max-cost path
- [ ] D7 `eval_no_excess` vs `no_excess_heart_flag` — state.rs:15-24 two-value flag; C reconstructs from score snapshots. A8 fields now exist.
- [ ] D8 24 `Condition` keys with 0 hits in condition.c. Highest value first: blade_limit(+operator), cost_limit_operator, self_effect_only, watches_area_move, trigger_event, action_reference, cost_total(+operator) on the CONDITION side, original_value, reference_card, require_position_cards, same_name, area_direction, blade_greater_than_all, activated_this_turn, paid_cost_this_turn, cost_reference_*, baton_touch_source, energy_placed, any_location, original_heart_total

## E. Modifiers (core/game_state/modifiers.rs + game_modifiers.rs)

- [ ] E1 **`evaluate_success_zone_constant_modifiers`** (modifiers.rs:2188) — LARGEST GAP IN THE MODULE. Absent from C entirely. Live at mod.rs:1064 + flow/late-success.
- [ ] E2 `apply_success_zone_effect` (modifiers.rs:2309) — private, downstream of E1
- [ ] E3 `evaluate_success_zone_constant_abilities` (modifiers.rs:2147) — absent; live at live.rs:1326, trigger_scan.rs:360
- [ ] E4 `restore_performance_need_heart_modifiers` (modifiers.rs:2159) — absent; live at live.rs:1327, trigger_scan.rs:365. A6 field exists.
- [ ] E5 `remove_revealed_cost_card` (modifiers.rs:2053) — field exists, no fn. Live at move/mod.rs:1536, :1955
- [ ] E6 `remove_from_source_hands` (modifiers.rs:2071) — absent. Live at move/mod.rs:614
- [ ] E7 `set_heart_override` (modifiers.rs:1656) — absent; live at choice.rs:3806
- [ ] E8 `clear_gained_abilities_for_card` (modifiers.rs:2112) — live at mod.rs:855
- [ ] E9 `record_stage_position_swap` (modifiers.rs:1799) — absent; live at position.rs:1022, :1061, :1102
- [ ] E10 `count_as_multiple_weight` (modifiers.rs:227) — absent; live at choice.rs:3678, :2842, :5893, :2702
- [ ] E11 `weighted_zone_count` (modifiers.rs:308) — absent; live at card.rs:2460, choice.rs:2662, :2702, :5906
- [ ] E12 `replacement_appear_wait_applies` (modifiers.rs:360) — absent; live at abilities.rs:1050
- [ ] E13 wire `rb_mods_add_blade_with_trace` / `_heart_with_trace` — the trace ring's only missing writers. Needs `ability_text` threaded through misc.c:218/224 (see F3)
- [ ] E14 wire the 9 zero-caller rb_mods_* setters/clearers (remove_blade, remove_heart, remove_score, clear_blade_set, clear_score_set, clear_blade_type, remove_cost_set, remove_heart_override, get_cost_set) — the correct targets are the Rust `clear_*` calls at abilities.rs:3061/3110/3164/3173
- [ ] E15 delete `rb_mods_remove_need_heart` + its rabuka.h:306 line — Rust has NO `remove_need_heart_modifier` (0 hits in engine/src). Use `rb_mods_add_need_heart(cid,color,-delta)` per game_state/modifiers.rs:615-619.
- [ ] E16 `eval_group`'s temporal arm (card.rs:1642-1655) keys on `m.cause_card_id` — A9 field now exists, so this is unblocked.
- [ ] E17 retire the file-static `s_opp_cause_fired[]` side table (game_state_abilities.c:2081-2114) in favour of `mods.opp_cause_fired_keys` (A8)

## F. Misc / effects

- [ ] F1 misc.c `h_custom` vs custom.c — ONE is live (misc.c:1052). custom.c is a third copy, uncalled, and PROVABLY UNSAFE to promote: its arms deep-clone an effect, hand it to a callee that parks `resume_eff`/`deferred`, then free the clone. Fix custom.c's clone lifetime FIRST, then flip executor.c:341, then delete h_custom.
- [ ] F2 misc.c:653/721 — Rust's `gain_heart` per-colour loop has NO C counterpart function at all (separate gap, not wiring)
- [ ] F3 misc.c thread `ability_text` into grant_blade/grant_heart_distribution so E13 can be wired (call sites misc.c:551, :558, :562, :571, :792 and :238)
- [ ] F4 dynamic_count.c `selected_card_score` + per_unit arms — DONE; verify
- [ ] F5 rules.c:1992 `r_yell_icons` + call at :2129 -> delete, call `rb_process_yell_revealed_card_icons` (exported). Same for phase.c:1369 `ptc_yell_had_draw_icon`. Collapses THREE hand-rolled copies of one decode.
- [ ] F6 `move.c` deck_bottom put-back — destination never arrives at rb_place_card_in_zone; `rb_effect_move_cards` dead body already removed; check whether `rb_zone_of_str` still folds deck_bottom
- [ ] F7 `move.c` `those_cards` / `revealed_cards` / `recently_moved` source dispatch arms
- [ ] F8 `move.c` multi-index stage answer — DONE by agent; verify

## G. Card model

- [ ] G1 card.c decode `effect_type` (card.rs:2580) — Rust-live, key absent from corpus
- [ ] G2 3 actions in `EffectKind::from_action` (card.rs:1277-1321) with no C literal: `"look"`, `"reveal_effect"`, `"set_card_identity_all_regions"` — decode-all tally first
- [ ] G3 delete `rb_condition_filter_subset` (card.c:1076) + rabuka.h:2369 together
- [ ] G4 `rb_card_fires_on_opponent_effects` fallback in game_state_abilities.c:2067 — delete the card-level fallback (4 card/effect pairs would arm that Rust would not)

## H. util.c

- [ ] H1 mirror the 6 new CardFilter carriers in `local_filter_matches` + `LocalCardFilter` (util.rs:1441-1498, :1518-1537, :1744/:1790). Then card_filter_from_effect can populate them.
- [ ] H2 export rb_group_name_first / rb_effect_group_name / rb_card_matches_effect_group so 4 local twins die (A20)

## I. Live / phases / engine

- [ ] I1 `rb_build_snapshot` / `rb_check_live_success` / `rb_enrich_from_applications` — 0 callers; `enrich` is empty because every field it writes is absent from RbLiveSnapshot (A19)
- [ ] I2 `discard_live_on_cannot_live` (live.rs:1832) — no C counterpart; `cannot_live` never reaches the C live path
- [ ] I3 `compute_snapshot_surplus` (live.rs:1122) — no C fn (pre-trigger form)
- [ ] I4 `stage_alloc` / `Allocation` (live.rs:34-55) — no C record type
- [ ] I5 phase.c rollover: add `player_cannot_activate` clear (phases.rs:279) and `baton_touch_used`
- [ ] I6 `rb_tick_gained` reader for `turn_area_movements` (abilities.rs:506-545) — A8 field exists, now written
- [ ] I7 engine.c rb_turn / rb_zone_of_str: `deck_bottom` + `energy_deck` arms
- [ ] I8 web_server.c: replace the inline 16-iter settle loop with `rb_setup_settle_single_player_state` (game_setup.rs:410, 500-iter guard, 6 phases)
- [ ] I9 web_server.c `parse_json_card_array` must call `rb_normalize_card_no` (web_server.rs:2360)

## J. Test-side (never weaken — align to Rust)

- [ ] J1 test_parity_draw_flat.c `dc_fill_decks_distinct` returns a LOCAL snap[64] that dies with the frame; all 6 callers read uninitialised stack (11 assertions)
- [ ] J2 test_parity_executor_dispatch.c:896 asserts the SINGLE-region spelling with all_regions=true; Rust's all-regions emits `card_identity:<card_id>:<id>`
- [ ] J3 tests/replay.c:28,109 assert `player.score` moved; Rust's execute_modify_score NEVER writes player.score
- [ ] J4 test_parity_cce_gaps e2/k7 demand an ENQUEUE refusal; Rust refuses at RESOLUTION (gates.rs:93-111) and E2's twin is a disjunction `is_err() || !has_pending_choice()`
- [ ] J5 test_parity_under_member.c:715/:1016 must call `rb_stage_position_change_with_bags(&tg.state.p[0], …)` — A20-style decl needed
- [ ] J6 test_parity_under_member.c:357-358 expects [a,c,b] for a centre-keeps-area plan; Rust's twin asserts [c,b,a]
- [ ] J7 test_parity_integration.c kinako positive: C stages id 2639 but flags moved_this_turn on id 2555; Rust assigns the same id
- [ ] J8 test_parity_zones_position.c:406 `TestGame tg;` NON-STATIC — 781 KB on the stack
- [ ] J9 test_parity_ability_mod* etc: `audit_fields.c:483-487` stale CHECK(0) markers re-verified by a peer
- [ ] J10 test_parity_small_clusters B2: 4 of 6 remaining need J1's re-prompt (see B2)
- [ ] J11 test_parity_hearts_required / small_clusters A4: R3BIRTH contamination
- [ ] J12 any `queued=1` HARD-CODED in a diagnostic — parity_wave1b had one masking a real failure

## K. Data / artifacts

- [ ] K1 `cards/build/abilities_strings.bin` must never be regenerated PARTIALLY. Currently: src/ = 726831 (matches gen_data.c), cards/build = 726831 as of main's fix. Verify before ANY build.
- [ ] K2 `bytecode_data.c` is 99 KB of dead .rodata in every binary — drop from SRC/OBJ_LIB
- [ ] K3 `gen_data_cdi.c` has `(void)len` (NULL deref) + a USE-AFTER-FREE (points g_* into buf, data.c frees buf)
- [ ] K4 `platforms/cdi/build_cdi.sh` omits gen_data.c; `build_genesis.sh` omits gen_data_rom.c — both platforms have undefined symbols
- [ ] K5 `tools/sweep_daemon.ps1` + `tools/queue_runner.ps1` run `make` against the shared tree concurrently and delete .o mid-run. Any in-tree measurement is unsound while they run.
- [ ] K6 `git stash@{0} session-wip` holds live work. A `stash pop` would clobber it. Work has been destroyed 3x by stray checkout/restore.

---

## ⚠ PATH CORRECTIONS (verified 2026-10-02 — earlier reports cited these WRONG)

Several earlier agent reports used shorthand that does not resolve. Verified against
the tree: `src/` has exactly four subdirs — `ability` (28 .c), `core` (14), `game` (3),
`turn` (3). Use these real paths:

| Cited as | Actually is |
|---|---|
| `src/util.c`, `src/core/util.c` | **`src/ability/util.c`** |
| `src/card.c`, `src/core/card.c` | **`src/core/card.c`** |
| `src/modifiers.c`, `src/core/modifiers.c` | **`src/core/modifiers.c`** |
| `src/move.c` | **`src/ability/effects/move.c`** |
| `src/misc.c` | **`src/ability/effects/misc.c`** |
| `src/rules.c` | **`src/ability/effects/rules.c`** |
| `src/phase.c` | **`src/turn/phase.c`** |
| `src/game_state_abilities.c` | **`src/core/game_state_abilities.c`** |
| `src/live.c` | **`src/turn/live.c`** |
| `src/executor.c`, `src/custom.c`, `src/dynamic_count.c` | **`src/ability/effects/…`** |
| `src/cost.c`, `src/condition.c`, `src/choice.c`, `src/vm.c` | **`src/ability/…`** (these were already right) |

ALWAYS confirm a path exists before editing. Item lines above inherited the wrong
path in places; the SYMBOL and Rust reference are correct, the path is not.

Also verified: `rb_get_selection_indices_filter` is declared non-static ONLY at
`src/ability/util.c:887` (tentative) with the single real definition at
`src/ability/util.c:2070`; `src/ability/effects/move.c:104` is a second
tentative declaration, not a duplicate definition. Do not "fix" this.

---

## Definition of done

1. `make clean && make all` green from scratch
2. `verify_artifacts.py` consistent
3. Every `[ ]` above closed or explicitly marked IMPOSSIBLE with a quoted reason
4. Whole-tree FAIL count taken in an ISOLATED snapshot, two agreeing runs
5. No test weakened, deleted, or ignored to reach a number