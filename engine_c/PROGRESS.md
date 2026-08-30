# engine_c — C port of the Rabuka engine

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
| `src/ability/effects/state.c` | `ability/effects/state.rs` + `misc.rs` | ⚠️ partial | `choose_required_hearts`; `set_heart_type placed_under` (still need-heart add) |
| `src/ability/effects/ability.c` | `ability/effects/ability_effects.rs` | ⚠️ partial | `gain_ability` expiry faithful (score-only approx); `activate_ability` source filter |
| `src/ability/effects/misc.c` | `ability/effects/misc.rs` | ⚠️ partial | `h_choice` → real `SelectCard`/`SelectTarget` emit; `h_play_baton_touch` redirect gate |
| `src/ability/effects/draw.c` | `ability/effects/draw.rs` | ✅ done | — |
| `src/ability/effects/score.c` | `ability/effects/score.rs` | ✅ done (faithful this session) | remaining 5 fns wired from `state.c`/`engine.c` — retire "simplified" comments |
| `src/core/card.c` | `core/card.rs` | ✅ done | `blade_heart`/`need_heart` split when Live needs it |
| `src/core/data.c` | data load | ✅ done | — |
| `src/core/alloc.c` | `core/pool.rs` | ✅ done (bump arena) | `rb_free` no-op on arena (intended) |
| `src/core/modifiers.c` | `core/game_modifiers.rs` + `modifiers.rs` | ✅ done | `recalculate_constants` per-card `heart_copy`/`multiplier` |
| `src/core/stats_pipeline.c` | `core/stats_pipeline.rs` | ✅ done | exact `Allocation` plan (greedy is approximate) |
| `src/core/game_state_abilities.c` | `core/game_state/abilities.rs` | ⚠️ partial | `rb_collect_live_modifiers` — phantom mapping (no such fn in this Rust rev); reconcile/remove |
| `src/core/tracking.c` | `core/game_state/tracking.rs` | ✅ done (yell_from_bottom ported this session) | — |
| `src/core/zones.c` | `core/zones.rs` + `player.rs` | ⚠️ partial | strict `stage[3]` + typed zones + cap enforcement |
| `src/turn/live.c` | `turn/live.rs` (2846 LOC) | ⚠️ partial | `BAll` doubling; `finalize_snapshot_fields`; `prohibition_effects` tie |
| `src/turn/phase.c` | `turn/phases.rs` (1685 LOC) | ⚠️ partial | mulligan choice; baton `last_vacated_stage_area`; delayed-modifier ticking |
| `src/turn/triggers.c` | `turn/triggers.rs` | ⚠️ partial | victory `prohibition_effects` tie-break; `check_expired_effects` full |
| `src/engine.c` | engine main loop + `turn/*` | ⚠️ partial | `set_heart_type`/`choose_required_hearts` property rewrites; unknown-verb no-ops |

## Sub-task queue (open placeholders, ready to copy)
1. `misc.rs` `execute_choice` / `play_baton_touch` → `effects/misc.c`
2. `score.rs` remaining 5 fns → `effects/score.c` (retire "simplified")
3. `state.rs` `choose_required_hearts` + `set_heart_type placed_under` → `effects/state.c`
4. `modifiers.rs` `refresh_yell_sources` (per-player scan) → `state.c`/`tracking.c`
5. `compound.rs` `conditional_on_*` / `repeat_procedure` feeding → `compound.c`
6. `move_cards.rs` `card_property` `has_all_blade` + placement edges → `effects/move.c`
7. `phases.rs` mulligan flow → `phase.c`
8. `live.rs` exact `Allocation` + `BAll` + `prohibition_effects` tie → `live.c`

## Translated this session
- `resolver.c` `rb_can_activate_effect` — now evaluates `eff->condition` (was Main-phase stub).
- `game_state_abilities.c` `rb_record_ability_use` — delegates to `rb_record_use` (was dead local log).
- `effects/score.c` `rb_execute_modify_score` — faithful (card_type/group/heart_colors/self_target/per_unit/floor).
- `tracking.c` `rb_perform_cheer_check` + `state.c` `modify_yell_source` — `yell_from_bottom` (G8) ported.
- `util.c` + `move.c` `card_property` filters — `has_blade_heart`/`has_score_icon` now faithful via `rb_card_has_blade_heart`/`rb_card_has_score_icon`; `has_all_blade` (BAll) implemented via `rb_card_has_all_blade` (was previously a silent `false`).

Hand-written suites green after every change (`rb_engine_test` / `rb_engine_replay` / `rb_engine_ported` 13/13).
