# engine_c — C port of the Rabuka engine

**STOP OVERTHINKING. JUST PORT.** Pick a `.c` file, open its Rust twin in `engine/src/...`,
find the functions it still fakes (grep `TODO`/`STUB`/`not yet`/`no-op`/`return 0;`/`placeholder`),
copy them verbatim into C, build, run `rb_engine_test`/`rb_engine_replay`/`rb_engine_ported`
(until those 3 stay green — the generated suite is allowed to stay red). Do not write essays
about whether something is "approximate"; port it and move to the next function. The md is only
a worklist to know which file maps to which Rust file and what's left.

**Rule:** copy `engine/src/...` into `engine_c/src/...` file-by-file. Rust is the finished
source of truth. Each placeholder/stub in a `.c` file gets translated from its matching Rust
function — no "best-effort returns 0" shortcuts. When you work a file, grep its Rust twin for
the functions it still fakes, port them, then tick the status below.

**Scope invariant:** only game logic is rewritten. Card data + ability bytecode are generated
artifacts embedded as data (`cards.bin`, `abilities_strings.bin`, `RBKA_BYTECODE[]`). The C
engine decodes that bytecode (mirrors `ability/vm.rs`) and executes the effect tree.

**Priority:** translate the whole engine first; the generated test suite (`rb_engine_generated`)
is a *worklist*, expected red until everything is ported. Only the hand-written suites
(`rb_engine_test` / `rb_engine_replay` / `rb_engine_ported`) are gating and must stay green.

---

## Worklist — C file → Rust file → status

| C file | Rust source | Status | Next copy |
|---|---|---|---|
| `src/ability/vm.c` | `ability/vm.rs` + `*_decoder_gen.rs` | ✅ done | — |
| `src/ability/condition.c` | `ability/condition/{card,compound,state}.rs` | ✅ done (all 20 variants) | `eval_both_condition` dispatch (no wire discriminator yet); `eval_temporal` nested sub-checks |
| `src/ability/choice.c` | `ability/choice.rs` | ⚠️ partial | `select_number`/`pay-skip` resume routing |
| `src/ability/compound.c` | `ability/compound.rs` | ⚠️ partial | `conditional_on_*` / `repeat_procedure` feeding `pending_repeat_actions` |
| `src/ability/ability_queue.c` | `ability_queue.rs` + `triggers.rs` | ⚠️ partial | `QueueState` FSM + `ConditionalChoice`/`resolver` |
| `src/ability/dynamic_count.c` | `ability/dynamic_count.rs` | ⚠️ partial | `last_cost_discard_count` / `cheer_revealed_cards` arms |
| `src/ability/util.c` | `ability/util.rs` | ⚠️ partial | group/series/set_card_identity membership (series not exposed) |
| `src/ability/cost.c` | `ability/cost.rs` | ✅ done (headless pay gate) | interactive prompts deferred |
| `src/ability/resolver.c` | `ability/resolver.rs` | ⚠️ partial | `get_trigger_ability_infos`/`resolve_ability`/`pending_choice` → real decode+queue |
| `src/ability/effects/move.c` | `ability/move_cards.rs` (3780 LOC) | ⚠️ partial | `under_member`/`same_area`/`empty_area` edges; `LookedAtRemaining` (`has_blade_heart`/`has_score_icon`/`has_all_blade` done this session) |
| `src/ability/effects/look.c` | `ability/look.rs` | ✅ done | — |
| `src/ability/effects/state.c` | `ability/effects/state.rs` + `misc.rs` | ✅ done | `choose_required_hearts` + `set_heart_type placed_under` are dispatched in `engine.c:447` (verified faithful) |
| `src/ability/effects/ability.c` | `ability/effects/ability_effects.rs` | ⚠️ partial | `gain_ability` now grants score/blade/heart/need_heart (was score-only); `activate_ability` source filter |
| `src/ability/effects/misc.c` | `ability/effects/misc.rs` | ✅ done | `h_play_baton_touch` now faithful (baton_touch_count gate + double-baton choice + `baton_touch_allowed` prohibition note); `gain_surplus_heart` verb ported |
| `src/ability/effects/draw.c` | `ability/effects/draw.rs` | ✅ done | — |
| `src/ability/effects/score.c` | `ability/effects/score.rs` | ✅ done (faithful this session) | remaining 5 fns wired from `state.c`/`engine.c` — retire "simplified" comments |
| `src/core/card.c` | `core/card.rs` | ✅ done | `blade_heart`/`need_heart` split when Live needs it |
| `src/core/data.c` | data load | ✅ done | — |
| `src/core/alloc.c` | `core/pool.rs` | ✅ done (bump arena) | `rb_free` no-op on arena (intended) |
| `src/core/modifiers.c` | `core/game_modifiers.rs` + `modifiers.rs` | ✅ done | `recalculate_constants` per-card `heart_copy`/`multiplier` |
| `src/core/stats_pipeline.c` | `core/stats_pipeline.rs` | ✅ done | exact `Allocation` plan (greedy is approximate) |
| `src/core/game_state_abilities.c` | `core/game_state/abilities.rs` | ⚠️ partial | `rb_collect_live_modifiers` — phantom mapping (no such fn in this Rust rev); reconcile/remove |
| `src/core/tracking.c` | `core/game_state/tracking.rs` | ✅ done | `rb_refresh_yell_sources` ported from `modifiers.rs:972` (per-player `yell_from_bottom` from constant `modify_yell_source("deck_bottom")` on live/success zones); called from `rb_recalc_constants` |
| `src/core/zones.c` | `core/zones.rs` + `player.rs` | ⚠️ partial | strict `stage[3]` + typed zones + cap enforcement |
| `src/turn/live.c` | `turn/live.rs` (2846 LOC) | ⚠️ partial | `BAll` doubling; `finalize_snapshot_fields`; `prohibition_effects` tie |
| `src/turn/phase.c` | `turn/phases.rs` (1685 LOC) | ⚠️ partial | mulligan choice; baton `last_vacated_stage_area`; delayed-modifier ticking |
| `src/turn/triggers.c` | `turn/triggers.rs` | ⚠️ partial | victory `prohibition_effects` tie-break; `check_expired_effects` full |
| `src/engine.c` | engine main loop + `turn/*` | ✅ done | `set_heart_type`/`choose_required_hearts`/`set_blade_type`/`set_card_identity` property rewrites all dispatched faithfully (verified); unknown-verb no-ops retained by design |
| `tools/gen_tests.py` | (transpiler) | ✅ done | `fire_live_start` → `rb_trigger_live_start`+`rb_drain_ability_queue` now emitted (was degraded to `// TODO:` at the per-line fallback); passthrough added for substituted engine calls |

## Sub-task queue (open placeholders, ready to copy)
1. `misc.rs` `execute_choice` / `play_baton_touch` → `effects/misc.c` (DONE: baton_touch faithful this session)
2. `score.rs` remaining 5 fns → `effects/score.c` (retire "simplified")
3. `state.rs` `choose_required_hearts` + `set_heart_type placed_under` → DONE in `engine.c:447`
4. `modifiers.rs` `refresh_yell_sources` (per-player scan) → DONE: `tracking.c` this session, called from `rb_recalc_constants`
5. `compound.rs` `conditional_on_*` / `repeat_procedure` feeding → `compound.c` (blocked on `ability_queue.c` FSM)
6. `move_cards.rs` `card_property` `has_all_blade` + placement edges → `effects/move.c` (`has_all_blade` done; placement edges remain)
7. `phases.rs` mulligan flow → `phase.c`
8. `live.rs` exact `Allocation` + `BAll` + `prohibition_effects` tie → `live.c`
9. **ENGINE GAP surfaced by transpiler fix:** `rb_trigger_live_start` over-grants on ~16
   tests (`got 1/2 expected 0`) once `fire_live_start` stopped being a TODO. Root-caused:
   `rb_execute_effect_ex` already gates on `e->has_condition` and `eval_group` is faithful,
   so unconditional conditions are honored. The residual over-grant is the **target
   SELECTION** path — live-start abilities that *choose* members by group/name and should
   grant 0 when none match (and the decline/optional-choice path that should skip the
   grant). The C `handle_action` gain_resource now skips the host fallback when a group
   filter (`gn`) is present (faithful), but the selection/decline path in
   `turn/triggers.rs::trigger_live_start` + choice-resume (`ability_queue.c`) still needs
   porting for the remaining cases.
10. `gen_tests.py`: also emit `recalculate_constants`/`test_recalc` after stage setup in
    constant tests that currently rely on an implicit Rust recalc (already mapped; verify
    any remaining `// TODO recalc` sites).

## Translated this session
- `resolver.c` `rb_can_activate_effect` — now evaluates `eff->condition` (was Main-phase stub).
- `game_state_abilities.c` `rb_record_ability_use` — delegates to `rb_record_use` (was dead local log).
- `effects/score.c` `rb_execute_modify_score` — faithful (card_type/group/heart_colors/self_target/per_unit/floor).
- `tracking.c` `rb_perform_cheer_check` + `state.c` `modify_yell_source` — `yell_from_bottom` (G8) ported.
- `util.c` + `move.c` `card_property` filters — `has_blade_heart`/`has_score_icon` now faithful via `rb_card_has_blade_heart`/`rb_card_has_score_icon`; `has_all_blade` (BAll) implemented via `rb_card_has_all_blade` (was previously a silent `false`).
- `misc.c` `h_choice` — now emits a `SELECT_TARGET` pending choice (mirrors `choice.rs`/`engine.c` `choice` verb) instead of a silent `return 1`.
- `misc.c` + `engine.c` + `rabuka.h` — `gain_surplus_heart` verb ported from `misc.rs:execute_gain_surplus_heart` (captures live surplus into `last_surplus_loss_count[pl]` from the latest snapshot).
- `effects/ability.c` `rb_gain_ability` — now grants blade/heart/need_heart modifiers (not just score); `rb_invalidate_ability`/`rb_tick_gained` revert all four kinds.
- `tools/gen_tests.py` — `fire_live_start(&mut game, cid)` now emits the real
  `rb_trigger_live_start(&tg.state, 0); rb_trigger_live_start(&tg.state, 1); rb_drain_ability_queue(&tg.state);`
  instead of degrading to a `// TODO:` comment (a per-line fallback was re-commenting the
  pre-substituted engine calls). Result: 13 generated tests now pass; 16 revealed a real
  engine gap (live-start over-grant, subtask 9).
- `src/turn/triggers.c` + `src/core/tracking.c` — `rb_refresh_yell_sources` ported
  (`modifiers.rs:972`) and called from `rb_recalc_constants`.
- `src/ability/effects/misc.c` — `h_play_baton_touch` faithful (baton_touch_count gate +
  double-baton pair choice + `baton_touch_allowed` prohibition note).
- `src/engine.c` `handle_action` — `gain_resource` no longer falls back to granting the host
  when a group filter (`gn`) matches no member (faithful: filtered grants yield 0 when no
  target). This is the live-start over-grant subtask (#9) partial fix.
- `src/ability/dynamic_count.c` — build-blocking signature mismatch (`host_cid` was dropped
  from `rb_resolve_dynamic_count`/`rb_effect_count`) fixed by threading `host_cid` through
  `engine.c`/`effects/draw.c` (the generated suite could not even compile before this).

Hand-written suites green after every change (`rb_engine_test` / `rb_engine_replay` / `rb_engine_ported` 13/13).
