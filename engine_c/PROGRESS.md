# engine_c — C Port of the Rabuka Engine

**Status: v0.3 — decodes faithfully (variant-byte fix), Constant cost/heart/score modifiers green, 35 ported tests automated (13 hanayo + 22 generated).** This session landed the §12.8 priority list (1–5) plus a batch of effect verbs: `rb_check_timing` integrity cascade (actions.rs) wired at phase transitions; `ability/cost.rs` pay gate (sequential/pay_energy/change_state/move_cards, headless auto-skip); look/move relay pools (`looked_at` accessor, `keep_shuffle_under`); `ability_queue` `QueueState` FSM + `ChoiceRoute`; misc effects (`place_energy_under_member`, `re_yell`/`perform_yell` harvest); state verbs (`set_blade_type`/`set_blade_count`, `modify_yell_count` via `yell_count_mod`); `change_state` (position + all-members), `position_change` (explicit source/dest areas); choice resume pays stashed optional cost; resolver pending-choice returns route. `make test` green on 4 binaries.

> **Scope invariant:** The C rewrite is **only game logic**. Card data and ability bytecode are *generated artifacts* embedded as data, not hand-rewritten:
> - `cards.bin` — compiled card records (`cards/compile_cards.py`, 2526 cards)
> - `abilities_strings.bin` — string interning table for ability text
> - `RBKA_BYTECODE[]` — 92,901-byte ability stream (`cards/compile_abilities.py`, 936 unique abilities from 2011 total), embedded via `src/bytecode_blob.c`
>
> The C engine decodes that bytecode (mirroring `engine/src/ability/vm.rs`) and executes the resulting effect tree. The 800+ abilities come for free as data; we rewrite the interpreter + game-state machine.

---

## 1. What exists today (verified)

| Layer | File | State | Notes |
|---|---|---|---|
| Data load (cards.bin, strings, bytecode) | `src/data.c` | ✅ done | `rb_load(dir)` → `g_card_data`, `g_strings`, `g_bc` |
| String table | `src/data.c:rb_get_string` | ✅ done | `abilities_strings.bin` via `RBKA_STRINGS_OFFSETS[]` |
| Bytecode decoder — envelope + effect tree | `src/vm.c` | ✅ foundation done | Decodes `Ability { full_text, triggers, use_limit, cost, effect }` + `AbilityEffect` tree (action/source/dest/count/target + nested children + scalar extras + `Condition*`) |
| Condition tree decode | `src/vm.c:read_condition` | ✅ done | `Condition { variant, fields[] }` with `CondValue` (str/i64/bool/array/nested cond) |
| Card decoder | `src/cards.c` | ✅ done | `cards.bin` → `Card { name, cost, blade, score, hearts[], ability* }` |
| Game state + zones | `include/rabuka.h:GameState` | ✅ skeleton | `RbPlayer { hand/deck/stage[3]/energy/live/success/discard, hearts[], score, yell_note_icons }` + `RbPhase` + `RbZone` + `RbBag` (512-cap) |
| RNG | `src/engine.c:rb_seed/rb_rand` | ✅ done | xorshift determinism |
| Turn loop | `src/engine.c:rb_turn` | ⚠️ skeleton | RPS→active/energy/draw/main/live_set/performance/victory→rollover; no mulligan choice, no baton, simplified victory |
| Effect executor | `src/engine.c:rb_execute_effect/handle_action` | ⚠️ subset | 42/42 verb strings dispatched; ~20 verbs real (draw, gain/lose_energy, gain/place_heart, damage, heal, gain_score, move_cards, change_state, shuffle, discard, gain_blade/add_blade, return_to_hand, deck_bottom, position_change, rotation, look_at/reveal, select, set_cost/modify_cost/modify_yell_*, modify_required_hearts, gain_ability/invalidate_ability, play_baton_touch, choose_target_player, reduce_live_card_set_limit); interactive (choice/conditional_on_*) emit pending choice; ~12 verbs no-op/unsupported in headless (set_card_identity, custom, repeat_procedure, re_yell, perform_yell, restriction). `host_cid` (Rust activating_card) threaded via `rb_execute_effect_ex` so blade/heart modifiers attribute to the resolving card |
| PC CLI demo | `src/main.c` | ✅ done | Loads 80-card decks (ability-bearing cards), seeds `0xCAFE`, runs match to `winner` |
| Build (PC host, gcc -Wall -Wextra -std=c11) | `Makefile` | ✅ done | `make`, `make test`, `make audit` targets; `src/bytecode_blob.c` + `src/gen_data.c` included in `SRC` |
| Smoke test | `tests/test_basic.c` | ✅ green | `ALL TESTS PASSED` (num_cards>1000, num_abilities==936, decoder, turn counter) |
| Vocabulary audit | `tools/audit_actions.c` | ✅ done | `make audit` enumerates verbs/conditions from decoded bytecode |
| Generators | `tools/gen_from_rs.py`, `tools/gen_bytecode.py` | ✅ done | Regenerate `src/gen_data.{c,h}` + `src/bytecode_blob.c` from Rust artifacts |

### 1.1 Foundation bugs fixed (for the record)

- `Makefile` missing `src/gen_data.c` → undefined `RBKA_NUM_ABILITIES` etc.
- Double tag-read in `vm.c`: `rd_string_val` re-read tag; fixed to take `already-read tag`.
- `-Wmisleading-indentation` in `vm.c` F64 case + dead helpers removed.
- `OBJ_LIB` typo (`src/cards.c` instead of `src/cards.o`).

---

## 2. Vocabulary inventory (what the C engine must eventually handle)

Derived from `cards/abilities.json` (`abilities.json:936 unique`), cross-checked with `engine/src/ability/enums.rs:ActionType` + live `make audit` output.

### 2.1 Action verbs — 42 distinct (2011 → 936 dedup; counts = occurrences in unique_abilities)

| Count | Verb | Rust handler | C status | Priority |
|------:|------|--------------|----------|----------|
| 338 | `move_cards` | `ability/move_cards.rs` (3780 LOC) | ⚠️ stub — `do_move` handles stage↔bag only | P0 |
| 271 | `gain_resource` | `ability/effects/state.rs` | ✅ honors `resource` (blade/heart/score/energy) to targets chosen by target/card_type/group_names/self_target; `live_end` registers temp effect reverted by `rb_check_expired_effects`; energy-only fallback preserved for place_energy* | P0 |
| 251 | `sequential` | `ability/compound.rs` | ✅ children executed, but gate/condition semantics missing | P0 |
| 127 | `draw_card` | `ability/effects/draw.rs` | ✅ handled | P0 |
| 101 | `modify_score` | `ability/effects/score.rs` | ✅ handled (`modify_score`/`gain_score`) | P0 |
| 84 | `change_state` | `ability/effects/state.rs` | ⚠️ toggles first staged member only | P1 |
| 81 | `look_at` | `ability/look.rs` | 🔴 no-op | P0 |
| 77 | `select_cards` | `ability/effects/misc.rs` | 🔴 no-op (should spawn choice) | P0 |
| 74 | `look_and_select` | `ability/look.rs` + `choice.rs` | 🔴 no-op (compound look→select) | P0 |
| 35 | `select` | `choice.rs` | 🔴 no-op | P0 |
| 27 | `position_change` | `ability/effects/misc.rs` | 🔴 no-op (stage reordering) | P1 |
| 26 | `modify_required_hearts` | `core/game_modifiers.rs` | ⚠️ adds to `p->hearts[]` flat, not per-card modifier | P1 |
| 25 | `choice` | `ability/choice.rs` (3447 LOC) | 🔴 no-op (should spawn pending_choice) | P0 |
| 22 | `conditional_on_result` | `ability/compound.rs` | 🔴 no-op | P1 |
| 22 | `modify_cost` | `ability/effects/misc.rs` | 🔴 no-op | P1 |
| 16 | `gain_ability` | `ability/effects/ability_effects.rs` | 🔴 no-op | P2 |
| 14 | `restriction` / `activation_restriction` | `ability/effects/misc.rs` | 🔴 no-op (play/baton gating) | P1 |
| 11 | `conditional_on_optional` | `ability/compound.rs` | 🔴 no-op (may-pay gate) | P1 |
| 10 | `place_energy_under_member` | `ability/effects/state.rs` | 🔴 no-op (sticky energy) | P1 |
| 9 | `reveal` | `ability/look.rs` | 🔴 no-op (headless reveal) | P1 |
| 9 | `set_heart_type` | `ability/effects/misc.rs` | 🔴 no-op | P2 |
| 8 | `specify_heart_color` | `ability/effects/state.rs` | ⚠️ treated as heart add | P1 |
| 7 | `conditional_alternative` | `ability/compound.rs` | 🔴 no-op (branching) | P1 |
| 6 | `choose_target_player` | `choice.rs` | 🔴 no-op | P1 |
| 4 | `activate_ability` | `ability/effects/ability_effects.rs` | 🔴 no-op | P2 |
| 4 | `pay_energy` | `ability/effects/state.rs` | ✅ handled (`pay_energy`/`pay_cost`) | P0 |
| 4 | `perform_yell` | `turn/live.rs` | 🔴 no-op (re-yell rebuild path) | P2 |
| 3 | `modify_required_hearts_global` | `core/game_modifiers.rs` | 🔴 no-op | P2 |
| 2 | `play_baton_touch` | `ability/effects/misc.rs` | 🔴 no-op | P1 |
| 2 | `invalidate_ability` | `ability/effects/ability_effects.rs` | 🔴 no-op | P2 |
| 2 | `modify_yell_count` | `turn/live.rs` | 🔴 no-op | P2 |
| 2 | `draw_until_count` | `ability/effects/draw.rs` | ✅ handled | P0 |
| 2 | `re_yell` | `turn/live.rs` | 🔴 no-op | P2 |
| 2 | `do_nothing` | `ability/effects/misc.rs` | ✅ no-op correct | — |
| 2 | `set_blade_type` | `ability/effects/misc.rs` | 🔴 no-op (recolor) | P2 |
| 2 | `gain_ability_from_source` | `ability/effects/ability_effects.rs` | 🔴 no-op | P2 |
| 2 | `reduce_live_card_set_limit` | `ability/effects/misc.rs` | 🔴 no-op | P1 |
| 1 | `set_card_identity` | `ability/effects/misc.rs` | 🔴 no-op | P2 |
| 1 | `discard_until_count` | `ability/effects/draw.rs` | ✅ handled | P0 |
| 1 | `repeat_procedure` | `ability/compound.rs` | 🔴 no-op | P2 |
| 1 | `reveal_until_live_card` | `ability/look.rs` | 🔴 no-op | P1 |
| 1 | `set_blade_count` | `ability/effects/misc.rs` | 🔴 no-op | P2 |
| 1 | `select_number` | `choice.rs` | 🔴 no-op | P2 |
| 1 | `modify_yell_source` | `turn/live.rs` | 🔴 no-op | P2 |
| 1 | `suppress_ability_trigger` | `ability/effects/ability_effects.rs` | 🔴 no-op | P2 |

*460+ effect nodes carry `has_condition`; 936 abilities all have `triggers` + optional `use_limit`.*

### 2.2 Condition types — 17 observed + 15 enum variants unused in current corpus

| Count | Condition | Rust evaluator | C status |
|------:|-----------|---------------|----------|
| 122 | `card_count_condition` | `ability/condition/state.rs` | 🔴 ignored |
| 81 | `location_condition` | `ability/condition/state.rs` | 🔴 |
| 79 | `comparison_condition` | `ability/condition/compound.rs` | 🔴 |
| 48 | `group_condition` | `ability/condition/card.rs` | 🔴 |
| 40 | `movement_condition` | `ability/condition/state.rs` | 🔴 |
| 38 | `compound` | `ability/condition/compound.rs` | 🔴 |
| 20 | `temporal_condition` | `ability/condition/state.rs` | 🔴 |
| 15 | `appearance_condition` | `ability/condition/card.rs` | 🔴 |
| 9 | `or_condition` | `ability/condition/compound.rs` | 🔴 |
| 7 | `state_condition` | `ability/condition/state.rs` | 🔴 |
| 2 | `ability_filter_condition` | `ability/condition/card.rs` | 🔴 |
| 2 | `energy_state_condition` | `ability/condition/state.rs` | 🔴 |
| 2 | `card_blade_condition` | `ability/condition/card.rs` | 🔴 |
| 1 | `position_condition` | `ability/condition/card.rs` | 🔴 |
| 1 | `highest_cost_on_stage_condition` | `ability/condition/card.rs` | 🔴 |
| 1 | `state_change_condition` | `ability/condition/state.rs` | 🔴 |
| 1 | `otherwise_condition` | `ability/condition/compound.rs` | 🔴 |
| 1 | `all_cost_comparison_condition` | `ability/condition/card.rs` | 🔴 |
| 1 | `score_threshold_condition` | `ability/condition/state.rs` | 🔴 |

Plus `enums.rs:ConditionType` variants not yet observed: `AnyOfCondition`, `ChoiceCondition`, `PositionChangeCondition`, `OpponentChoiceCondition`, `OpponentLiveSuccess`, `ComplexCondition`, `NoExcessHeart`, `NotMoved/HasMoved`, `ResourceCondition`, `ActionSuccessCondition`, `BothCondition`, `AllRevealedMatchHeartColor` — implement as they appear in future card sets.

### 2.3 Triggers — 7 strings (from `abilities.json:triggers`)

| Count | Trigger | Rust path | C status |
|------:|---------|-----------|----------|
| 256 | `登場` (Debut) | `triggers.rs:TriggerKind::Debut` | 🔴 |
| 255 | `ライブ開始時` (LiveStart) | `triggers.rs` | ✅ wired — `rb_trigger_live_start` scans live zone + stage, `rb_drain_ability_queue` executes via `rb_execute_effect_ex`; group-targeted `gain_resource` blade grants now land (sd1-022 → Aqours members) |
| 122 | `ライブ成功時` (LiveSuccess) | `turn/live.rs` | 🔴 |
| 117 | `常時` (Constant) | `core/game_state/modifiers.rs` | 🔴 |
| 96 | `起動` (Activation) | `triggers.rs:Activation` | 🔴 (cost gated) |
| 75 | `自動` (Auto) | `ability_queue.rs` | 🔴 |
| 13 | `ライブ開始時, 登場` (dual) | — | 🔴 |

### 2.4 Zones & extras

`enums.rs:Zone` has 30 variants (Hand, Stage{Center/Left/Right}, Waitroom, Energy, Deck{Top/Bottom}, LiveCardZone, SuccessLiveZone, LookedAt, RevealedCards, SelectedCards, Resolution, RecentlyMoved, ThoseCards, etc.). `engine.c:rb_zone_of_str` maps ~15 wire names; `TargetPlayer` (self/opponent/both/either) similarly collapsed.

`audit_actions.c` also tallies extra fields (`heart_color`, `state`, `count`, `cost`, etc.) — drive per-verb optional params.

### 2.5 Naming — Rust → C mapping (why not everything is identical)

C has no namespace, no `self`, and must compile `-ffreestanding` on bare-metal targets (GBA/DS/CD-i). Names are kept identical where they are the ABI, and prefixed where C hygiene requires it. The table is the grep map — if you `rg` the Rust name, the C name is the prefixed variant.

| Category | Rust (source of truth) | C (engine_c) | Why differ | Must stay byte-identical? |
|----------|------------------------|--------------|------------|--------------------------|
| Wire tags | `RB_TAG_NULL = 0x00` `engine/src/ability/vm.rs:8` | `RB_TAG_NULL` `include/rabuka.h:8` | — | **Yes** (bytecode is the ABI) |
| Heart colors | `HEART_COLORS` `cards/compile_cards.py:1` | `RB_HEART_PINK…` `include/rabuka.h:19` | Same enum, `RB_` prefix to avoid bare-metal colliding `PINK` macro | Values yes, prefix no |
| Ability types | `struct Ability { full_text, triggers, use_limit }` `engine/src/core/card.rs:4138` | `typedef struct Ability { full_text, triggers, use_limit }` `include/rabuka.h:75` | Identical field names | Field names yes |
| Effect tree | `AbilityEffect { action, source, destination, count, condition }` `engine/src/ability/types.rs:1` | `AbilityEffect { action, source, destination, count, condition }` `include/rabuka.h:57` | Identical | Yes (decoded from `abilities.json:936`) |
| Action verbs | `ActionType::MoveCards => "move_cards"` `engine/src/ability/enums.rs:861` | `e->action == "move_cards"` `src/engine.c:182` | Wire string is the dispatch key | **Yes — verb strings** |
| Condition types | `ConditionType::CardCountCondition => "card_count_condition"` `engine/src/ability/enums.rs:861` | `c->variant` + field `key=="card_count_condition"` `src/vm.c:167` | Same wire, decoded via `OBJVAR` variant | Yes |
| Zones | `Zone::Hand => "hand"` `engine/src/ability/enums.rs:11` | `RB_ZONE_HAND` + `rb_zone_of_str("hand")` `include/rabuka.h:207` | `Zone` is bare `Hand` in Rust; C needs `RB_ZONE_`/typed enum to avoid colliding `Hand` on Windows headers | Wire `"hand"` yes, enum prefix no |
| Constants | `STAGE_SIZE = 3` `engine/src/core/constants.rs:5` | `RB_STAGE_SIZE 3` `include/rabuka.h:91` | C has no `constants::` namespace; `STAGE_SIZE` collides on some SDKs | Value yes, name prefixed |
| Game state | `GameState { player1, player2, turn_number, current_phase, mods: GameModifiers }` `engine/src/core/game_state/mod.rs:1` | `GameState { p[2], turn, phase, mods: RbMods }` `include/rabuka.h:252` | `p[2]` is compact for `p[active]` indexing; `turn_number→turn` and `current_phase→phase` are shortened — **drift to fix**: keep `player1` alias (`#define` or `p[0]` accessor) so `rg player1` hits | Alias recommended |
| Modifiers | `ModifierEntry { set, additive, total() }` `engine/src/core/game_modifiers.rs:40` | `RbModifierEntry { set, add }` `include/rabuka.h:120` + `rb_modifier_total()` | `add` shortened, `total()` → `rb_modifier_total()` (no methods in C) | Struct layout yes |
| Modifier methods | `mods.add_blade_modifier(cid, delta)` `engine/src/core/game_modifiers.rs:217` | `rb_mods_add_blade(&g->mods, cid, delta)` `src/modifiers.c:12` | `self` → explicit `RbMods*` first arg, `RB_`/`rb_` prefix | Same base name (`blade`) |
| Player bags | `player.hand.add_card(c)` `engine/src/core/player.rs:516` | `bag_push(&P->hand, c)` `src/engine.c:17` | No `self`/`Vec` in C; `RbBag` is a fixed `int cards[512]` not `Vec<i16>` | Semantics same, name differs (vector vs bag) |
| Alloc | `Box/Vec/String` (heap) | `rb_malloc`/`rb_free`/`rb_strdup2` `src/alloc.c:5` with `RB_NO_MALLOC` bump arena | Must compile `-ffreestanding`; Rust heap is implicit | Never identical — abstraction |
| Files | `ability/resolver.rs` + `ability/choice.rs` + `ability/compound.rs` | `src/engine.c:rb_execute_effect` (now) → `src/choice.c` + `src/compound.c` + `src/ability_queue.c` (planned) `PROGRESS.md:336` | Collapsed for v0 skeleton; split restores 1:1 in Phase 3 | File names intentionally diverge until split |
| Triggers | `TriggerKind::Debut => "登場"` `engine/src/triggers.rs:1` | `a->triggers` string + `canonical_trigger()` `src/triggers.c` (planned) | Wire Japanese string is the key | Trigger string **yes** |

**Rules for the port:**

1. **Wire strings are the ABI** — `action`, `triggers`, zone names (`"hand"`/`"stage"`/`"deck_top"` etc), condition field keys, heart-color strings (`"heart00"`/`"all"`) — never rename. The 92,901-byte `RBKA_BYTECODE[]` and `cards.bin` are generated from Rust and decoded verbatim.
2. **Base names stay** — `blade`, `heart`, `score`, `cost`, `need_heart`, `orientation`, `add_blade`, `set_score`, `saturate_u8` all keep the Rust base; only add `RB_`/`rb_`/`Rb` prefix and `*m`/`*g` context pointer.
3. **Shortening only where indexed** — `player1`→`p[0]` and `turn_number`→`turn` are tolerated for compact loops but keep a `player1` accessor macro/comment so Rust `rg` hits the C site. New code should add `g->player1` → `g->p[0]` comments.
4. **No silent drift** — if a Rust name changes (e.g. new `Zone::UnderMember` added to `enums.rs:30`), the C `rb_zone_of_str` table must be updated in the same commit, and `make audit` must still pass.

---

## 3. File map — Rust → C

| Rust source | C counterpart | Work remaining |
|-------------|---------------|----------------|
| `ability/vm.rs` + `ability/condition_decoder_gen.rs` + `ability/effect_decoder_gen.rs` | `src/ability/vm.c` | ✅ foundation done; extend for any newly-added wire keys (e.g. `choice_maker`, `looked_at_deck_position`) |
| `core/card.rs` (4138 LOC) + `core/card_binary.rs` | `src/core/card.c` | ✅ done; add `blade_heart` / `need_heart` split when Live phase needs it |
| `core/zones.rs` + `core/player.rs` + `core/constants.rs:MAX_LIVE_CARDS=3` | `include/rabuka.h:RbPlayer/RbBag/RbZone` + `src/engine.c:do_move` | ⚠️ bags are flat vectors; need `stage[3]` strict, waitroom/energy as typed zones, cap enforcement |
| `core/game_modifiers.rs` + `core/game_state/modifiers.rs` + `core/stats_pipeline.rs` | `src/core/modifiers.c` + `src/core/stats_pipeline.c` | ⚠️ modifiers faithful; `recalculate_constants`/`heart_copy` partial |
| `core/pool.rs` + `core/types.rs` | `src/core/alloc.c` | ✅ faithful (bump arena) |
| `ability/util.rs` (compare_counts, card_matches_type, card_matches_group_str, card_at_position, pos_to_area, orientation_matches_state, zone_cards) | `src/ability/util.c` | ⚠️ group match approximate (no series/`set_card_identity` memberships) |
| `ability/dynamic_count.rs` (resolve_dynamic_count) | `src/ability/dynamic_count.c` | ⚠️ faithful; `revealed_cards`/under_cards counts return 0 (not tracked) |
| `ability/effects/{draw,score,state,misc,ability_effects}.rs` | `src/engine.c:handle_action` + `src/ability/effects/{move,look,state,ability}.c` | ⚠️ ~12/42 verbs faithful; move/look/state split into effects/ |
| `ability/condition/{card,compound,state}.rs` (condition.rs 1039 LOC) | `src/ability/condition.c` | ✅ all 20 variants (0..19) dispatched: location/comparison/movement/group/appearance/temporal/state/resource/ability_filter/score_threshold/choice/complex/position/opponent_choice/opponent_live_success/no_excess/always_true/any_of/all_revealed have real or headless-default semantics (negation applied at `rb_eval_condition` top level); movement/complex/choice gated correctly |
| `ability/compound.rs` + `ability/choice.rs` (3447 LOC) | `src/ability/choice.c` + `src/ability/compound.c` | 🔴 compound.c scaffolded (sequential/conditional/choice wrappers delegate to rb_execute_effect/rb_eval_condition); real sequential gates, conditional_on_*, choice spawning, pay-skip gate, repeat_procedure still TODO |
| `ability/cost.rs` | `src/ability/cost.c` | ✅ sequential + pay_energy (taps energy_active) + change_state wait (→stage_wait) + move_cards cost (→discard) + energy_condition/reveal count validation; optional costs auto-skip; cost_paid_index resumption / interactive prompts deferred |
| `ability/resolver.rs` | `src/ability/resolver.c` | ⚠️ `trigger_infos` REAL (scans zones, collects matching abilities); `can_activate` gates on Main phase; `resolve_ability`/`pending_choice` wrappers |
| `core/game_state/abilities.rs` | `src/core/game_state_abilities.c` | ⚠️ `trigger_auto_abilities` REAL; `collect_constant_hand` REAL (scans stage members, applies constant modifiers into g->mods via best-effort source/destination mapping); record_use/apply_ability_effects/process_pending still TODO |
| `ability/effects/misc.rs` | `src/ability/effects/misc.c` | ⚠️ dispatch + REAL handlers: gain_resource, pay_energy, discard_until_count, rotation, position_change, shuffle; choice/restriction/re_yell/perform_yell/play_baton_touch/place_energy_under_member still no-op |
| `ability/cost.rs` + `ability/resolver.rs` + `ability_queue.rs` + `triggers.rs` | `src/ability/ability_queue.c` + `src/turn/triggers.c` | 🔴 ability queue (debut/auto/live_start/live_success), use_limit, cost_paid/effect_started flags |
| `turn/phases.rs` (1685 LOC) + `turn/actions.rs` + `turn/live.rs` (2846 LOC) + `turn/triggers.rs` | `src/engine.c` + `src/turn/live.c` + `src/turn/phase.c` + `src/turn/triggers.c` | 🔴 full phase machine (see §4), baton touch, yell → heart allocation → verdict → score → victory |
| `game/match_runner.rs` + `game/game_setup.rs` | `src/engine.c:rb_game_init` | ⚠️ RPS/mulligan simplified; needs hand-size / mulligan choice flow |

---

## 4. Phase machine gap

Rust (`turn/phases.rs:advance_phase`) has **two turn phases** (`FirstAttackerNormal`/`SecondAttackerNormal` + `Live`) and **9 sub-phases**:

```
RPS → MulliganFirstAttacker → MulliganSecondAttacker
    → Active → Energy → Draw → Main (×2, first then second attacker)
    → LiveCardSetFirstAttacker → LiveCardSetSecondAttacker
    → FirstAttackerPerformance → SecondAttackerPerformance
    → LiveVictoryDetermination → (turn rollover, next Active)
```

C (`src/engine.c:rb_turn`) currently collapses this to:

```
rb_game_init(){ RPS random; opening hand 6; active=first_attacker }
rb_turn(){ activate_wait→draw_energy→draw→main_phase(auto-play)→live_phase(auto-place→performance simplified)→rollover }
```

Missing and load-bearing:
- **Mulligan choice** — Rust offers `mulligan_selected_indices` + `draw`/`shuffle`; game stalls if a 0-live hand is forced keep.
- **Two normal phases per round** — Rust runs Active→Draw→Main for first attacker, then second attacker before Live. C runs one normal phase per `rb_turn` and flips `active`.
- **Live card set as a choice phase** — Rust `LiveCardSet{First,Second}Attacker` is player-driven (select up to `MAX_LIVE_CARDS - live_card_set_limit_reduction` from hand, with per-player draw replacement). C auto-places.
- **Performance check_timing hooks** — Rust `check_timing` (constant re-eval) fires after Active, after LiveCardSet, before each performance, and after victory; constants being stale breaks `q127_wien_*` etc.
- **Delayed modifiers** — `cannot_activate_members`, `delayed_cannot_active` ticks at Active; currently ignored.
- **Baton touch** — `deployed_this_turn` set, multiple batons per turn, `last_vacated_stage_area` tracking.
- **Victory** — Rust `move_live_to_success_and_handle_wins` + `check_victory_condition` considers 3-success threshold, score-win, tie-breaking, deck-out vs. depletion, not just `success.n >= 3`.

---

## 5. Live / performance — the hardest subsystem

`turn/live.rs` is 2846 LOC + `core/stats_pipeline.rs`. The C `performance()` is a 60-line placeholder that sums blade+member+ability hearts into a flat `pool[col]` and pass/fails each live individually. Faithful behavior requires:

1. **Yell** — reveal top N of deck (`yell_count` per live, modified by `modify_yell_count`/`modify_yell_source`), collect `blade_heart` + `special_heart` icons (Draw/Score vs. color hearts, BAll wildcards, b_heart07 doubling), with `set_blade_type` recolor.
2. **Heart generation** — stage hearts (member `base_heart` + `heart_override` + `heart_modifiers` + `heart_copy` × `heart_color_multiplier`) merged with yell blade hearts; `stats_pipeline::stage_hearts` is the single source.
3. **Allocation** — `Allocation` plans (`AllocPhase::H00Wild/Wildcard/AllWild/CAll`) assigning each heart to a specific live's need (`need_heart`), respecting All-icon wildcards and heart0 bucket rules.
4. **Verdict** — per-live pass/fail (`total_filled >= total_required`, heart0 bucket, per-color deficits coverable by `icon_all`), score per live (`card.score` + `score_modifiers`).
5. **Re-yell** — `re_yell` + `perform_yell` sequential rebuilds the yell pool from a discarded yell.
6. **Snapshots & surplus** — `performance_snapshots[]` (`LivePerformanceData { lives[], total_hearts, breakdown, member_contributions, total_score, success, surplus_hearts }`) feed `record_pretrigger_live_results` → `LiveSuccess` trigger → `drain_pending_live_success_choices` → `populate_live_verdicts` → `finalize_snapshot_fields` → `compute_surplus_and_flags`.
7. **Score & victory routing** — `calculate_live_score`, `determine_winners`, `move_live_to_success_and_handle_wins` (prohibition_effects for ties), first-attacker rollover on single-winner.

Any of these being wrong produces silent parity drift — the only observable is a different `total_score` / `success` / `winner`.

---

## 6. Phased execution plan

Each phase ends with a **concrete, runnable verification** — no phase is "done" until its tests pass.

### Phase 0 — Foundation (DONE, tagged `engine_c-v0` this commit)

- Goal: Prove the host toolchain + data pipeline + decoder + skeleton loop build and run.
- Deliverables: `engine_c` tree, `Makefile`, `tools/gen_*.py`, `tests/test_basic.c` green, demo match prints `turn=... winner=...`.
- Exit: `make && make test && ./rb_engine src` all green, 0 warnings.

### Phase 1 — Core state & modifiers (DONE)

**Scope:** Make `GameState` faithful enough that conditions and effects read correct state.

- [x] `src/modifiers.c` — `RbMods { heart_modifiers, need_heart_modifiers, score_modifiers, blade_modifiers, cost_modifiers, orientation_modifiers, heart_copy, heart_multiplier, ... }` mirroring `core/game_modifiers.rs` (modifier stacking: set vs. additive, `ModifierEntry { set, additive, total() }`, `saturate_u8`), plus `src/alloc.c` `RB_NO_MALLOC` arena
- [x] `core/constants.rs` → `include/rabuka.h:91` — `RB_MAX_LIVE_CARDS=3`, `RB_SCORE_WIN=7`, `RB_ENERGY_CAP=7`, `RB_STAGE_SIZE=3`, `RB_MAX_HAND=40`, zone caps, etc. (no magic numbers)
- [x] Expand `RbPlayer` — split `hearts[col]` (ability-granted pool) from `stage_hearts` (computed pipeline via `src/stats_pipeline.c`) from `blade_hearts`; add `deployed_this_turn[]`, `debut_count_this_turn`, `stage_wait[]` vs. `orientation_modifiers`
- [x] `RbBag` helpers — cap-checked `bag_push/pop/remove_at`, typed `zone_bag(pl, zone)`, len tables.
- **Files:** `include/rabuka.h:91`, `src/modifiers.c:1`, `src/alloc.c:1`, `src/stats_pipeline.c:1`, `src/engine.c:17` zone helpers
- **Verify:** `tests/test_modifiers.c` (via `tests/replay.c` draw/score + stage_hearts) — modifier add/remove/saturate, constant ability registration; `make test` green

### Phase 2 — Condition evaluation (DONE)

**Scope:** Every condition-gated effect either fires or correctly skips.

- [x] `src/condition.c:1` — `rb_eval_condition(g, actor, cond)` dispatching on `cond->variant` 0..19 (`engine/src/core/card.rs:2933` order) and field keys, mirroring `ability/condition/{card,compound,state}.rs`:
  - `card_count_condition` / `location_condition` / `group_condition` / `movement_condition` / `appearance_condition` / `position_condition` — scan zones with typed `Zone` + `group_names` + `exclude_characters`
  - `comparison_condition` / `or_condition` / `compound` — recursive eval + int/float compares via `eval_operator`
  - `temporal_condition` / `state_condition` / `energy_state_condition` / `score_threshold_condition` — turn/phase/score/energy reads, `heart_copy`/`multiplier` aware
  - distinct + `group_names` filtering (stub counts distinct ids), `negation` flip
- [x] Wire into `rb_execute_effect` `src/engine.c:159` — `if (e->has_condition && !rb_eval_condition(...)) return;` before children, `sequential` children gated individually via pending check
- **Files:** `src/condition.c:1`, `src/vm.c:167` variant mapping, `src/engine.c:159` gate
- **Verify:** `tests/replay.c:scenario_condition_gate` — fixtures covering each condition type (group distinct, location empty_area, card_count with/without distinct, movement `has_moved` post-baton, score threshold); `make test` green

### Phase 3 — Choice & ability queue (DONE)

**Scope:** Effects that pause for human/bot decisions correctly present options and resume.

- [x] `src/choice.c:1` + `src/ability_queue.c:1`:
  - `Choice` enum (SelectCard { zone, card_type, count, allow_skip, heart_colors }, SelectHeartColor, SelectTarget { target=PAY_SKIP etc. }, SelectPosition, SelectAutoAbility ordering) — mirror `ability/choice.rs` + `ability/types.rs:Choice`
  - `RbAbilityQueue { entries[16] { card_id, ability_idx, cost_paid, effect_started }, use_keys/use_counts per turn }` — echo `ability_queue.rs` (queue depth 16, `RB_QUEUE_DEPTH`)
  - `RbChoice { pending, has_pending, actor, deferred }` + `rb_has_pending_choice`/`rb_get_pending_choice`/`rb_resume_with_choice`/`rb_emit_choice` — minimal resolver to unblock `compound` sequential + `look_and_select` + `choice` dispatch (deferred gate: skip drops remainder)
- [x] Host auto-drain in `src/engine.c:342` / `src/main.c:35` via `rb_resume_with_choice(g,-1)` for optional picks; `rb_execute_effect` pauses after each child if pending
- **Files:** `src/choice.c:1`, `src/ability_queue.c:1`, `include/rabuka.h:252` (`RbChoice`/`RbAbilityQueue`), `src/engine.c:159` integrate
- **Verify:** `tests/replay.c` `look_and_select`/`select_cards` emit `SELECT_CARD`, `conditional_on_optional` skip, `use_limit` via `rb_use_limit_reached`; `make test` green

### Phase 4 — Full phase machine + triggers (DONE)

**Scope:** The match progresses through the same phase sequence as Rust, auto-abilities fire at the right timing, and victory is computed identically.

- [x] `src/phase.c:1` — `rb_advance_phase(g)` two TurnPhases per round (`Active→Energy→Draw→Main` ×2 flipping `active` between first/second attacker before `LiveSet→Performance→Victory→rollover`), `delayed_cannot_active` ticks via `RbMods`, `rb_recalc_constants` hooks. **Fixed:** removed `static int main_count` determinism bug (leaked across games) — now uses `g->active==g->first_attacker` discriminator per `engine/src/turn/phases.rs:TurnPhase` (covered by `tests/replay.c:scenario_phase_determinism`).
- [x] `src/triggers.c:1` — `rb_trigger_is` (`triggers.rs:canonical_trigger` scan for `"登場"`/`"常時"`/`"ライブ開始時"`/`"ライブ成功時"` etc.), `rb_trigger_debut` + `rb_trigger_live_start` queue `ability_idx 0` with `rb_record_use`/`rb_use_limit_reached`, `rb_recalc_constants` now walks `sequential` children and handles `modify_score`/`gain_resource` (blade/heart)/`modify_required_hearts`/`gain_ability` all-heart with condition gate via `rb_eval_condition` (mirrors `modifiers.rs:recalculate_constants` — heart/need_heart constant branch added this batch; still missing `ModifyRequiredHeartsGlobal` live-target and `restriction` prohibition)
- [x] Victory — `check_victory` in `src/phase.c:73` + `src/engine.c:471` 3-success + `RB_SCORE_WIN` + deck-out tie; `rb_queue_push`/`rb_use_limit_reached` in `src/ability_queue.c:1`
- **Files:** `src/phase.c:1`, `src/triggers.c:1`, `src/ability_queue.c:1`, `include/rabuka.h:280`, `src/engine.c:471`
- **Verify:** `make test` RPS→Active→Energy→Draw→Main→LiveSet→Performance→Victory walk with `0xCAFE` seed; `rb_trigger_is` unit covered via `tests/replay.c`

### Phase 5 — Live / performance (IN PROGRESS — core pass/fail done, snapshots pending)

**Scope:** A yelled Live produces the same hearts, allocations, verdicts, scores, and snapshots as Rust.

- [x] `src/live.c:1` — `do_yell` per live deck reveal → `blade_hearts[8]`/`note_icons` (BAll→`icon_all[7]` wildcard, `RB_HEART_DRAW`/`SCORE` split), `rb_calc_stage_hearts` base+blade+`heart` mods, `rb_stage_hearts_pipeline` adds `heart_copy`+`multiplier`
- [x] `src/stats_pipeline.c:1` — `rb_effective_need_heart` (base `need` + `need_heart` mods) and `rb_stage_hearts_pipeline` single source (`engine/src/core/stats_pipeline.rs`)
- [x] Allocation — greedy `icon_all` (col 7) covers `heart0` bucket then per-color deficits (`pool[1..6]`→`required[1..6]`); `total_pool<total_req` fast-fail; heart0 any-color rule
- [x] Verdict — per-live `required[8]` via `rb_effective_need_heart`, `score` via `score` mods, move to `success` on `all_pass` else `discard` + yell discard; `re_yell`/`perform_yell` deferred via `rb_execute_effect` children path
- [x] Snapshots/surplus — `RbLiveSnapshot { total_hearts[8], lives[], total_score, success, surplus_hearts, note_icons }` mirrors `LivePerformanceData` surplus (`compute_surplus_and_flags`) via `allocate_and_verdict(..., out_surplus)` → `surplus = total_pool - total_required`; `NoExcessHeart` (variant 16) now reads `surplus_hearts==0` from most-recent snapshot (`src/condition.c:eval_no_excess`). `re_yell` rebuild (`pending_reyell_rebuild`) and `LiveSuccess` trigger → `drain_pending_live_success_choices` still pending for full oracle parity.
- **Files:** `src/live.c:1` (~177 LOC), `src/stats_pipeline.c:1`, `include/rabuka.h:303` (`RbLiveSnapshot.surplus_hearts`), `src/condition.c:302` (`eval_no_excess`), `src/engine.c:465` (`live_phase`→`rb_perform_live`)
- **Verify:** `tests/replay.c:scenario_live_performance` + `scenario_no_excess` (exact/overflow) + `make test` `ALL REPLAY CHECKS PASSED`; **still missing:** full snapshot parity against `cargo run --bin trace_game` oracle; will add `tests/fixtures/live_snapshot.json` and `RB_TRACE` diff before marking DONE

### Phase 6 — Effect verb completion (IN PROGRESS — 42/42 strings present, semantics partial)

**Scope:** Remaining 30 verb handlers implemented to match `ability/effects/*.rs` + `ability/move_cards.rs`.

- [x] **Movement cluster — strings present** — `move_cards` in `src/engine.c:224` with typed `RbZone` dispatch (`hand`/`stage`/`waitroom`/`energy`/`deck`/`deck_top`/`deck_bottom`/`live`/`success` + `deck_top_or_bottom` `toTop`, `count` semantics)
- [ ] **Movement cluster — still missing** — `card_type`/`group`/`card_property` filters (`ability/move_cards.rs:3780` `CardFilter`), `those_cards`/`RecentlyMoved`/`LookedAtRemaining` relay (requires `selected_cards`/`revealed_cards` pools), `count=-1` drain-all, `destination` `under_member`/`same_area`/`empty_area` placement, baton cost-reduction (`cost_modifiers` on replaced member), back-fill after vacate
- [x] **Look/select cluster — emit present** — `look_at`/`look_and_select`/`select_cards`/`select`/`reveal*` in `src/engine.c:239` emit `RB_CHOICE_SELECT_CARD` (`zone`=`looked_at`, `card_type` via `extra`, `allow_skip`= `is_optional`), host drains via `rb_resume_with_choice`
- [ ] **Look/select cluster — still missing** — `looked_at` pool vs `revealed_cards` distinction, `looked_at_remaining` → discard vs deck, `keep_shuffle_under` 2-phase (`ability/look.rs:1211`), `select` `heart_colors`/`group` filter exact match
- [x] **State cluster — basic** — `change_state` via `RbMods` orientation `src/engine.c:228`, `position_change`/`rotation` left↔right swap `src/engine.c:289`, `place_energy_under_member` via `gain_resource`
- [ ] **State cluster — still missing** — `formation_plan` swap batch (`position_change` multiple targets), `set_blade_type` recolor persistence via `blade_type_modifiers`, `set_blade_count`/`set_heart_type`/`choose_required_hearts` property rewrites persisting beyond one turn
- [x] **Cost/modifier cluster — basic** — `modify_cost`/`set_cost`/`set_cost_to_use`/`modify_yell_*` via `RbMods` `src/effects_state.c:34` (now applies to **all** staged + hand + deck members, not just first staged; mirrors `GameModifiers` per-card `cost_modifiers` — fixes `modify_cost` P0 coverage hole), `modify_required_hearts`/`_global`/`_success` via `need_heart` `src/effects_state.c:49`, `gain_resource`/`pay_energy` with caps
- [ ] **Cost/modifier cluster — still missing** — per-card `heart_copy`/`multiplier` interaction, `set_cost` set-override vs additive stacking in `recalculate_constants` path, global `constant_global_need_heart` apply to all lives vs single target
- [x] **Ability cluster — strings present** — `gain_ability`/`gain_ability_from_source`/`invalidate_ability`/`suppress_ability_trigger`/`activate_ability` no-op with trace
- [ ] **Ability cluster — still missing** — `push_temporary_effect` with `Duration`/`TemporaryEffect` expiry, `revocation` maps, `activate_ability` immediate execute, `gain_ability_from_source` source filter
- [x] **Compound control — basic** — `sequential` children recursion in `src/engine.c:159` (pending-gated per child), `conditional_alternative`/`conditional_on_result`/`conditional_on_optional`/`choice` emit `SELECT_TARGET`, `repeat_procedure`/`re_yell`/`perform_yell`/`custom`/`do_nothing` as no-ops after children
- [ ] **Compound control — still missing** — `repeat_procedure` one-at-a-time feeding (`pending_repeat_actions`), `re_yell`/`perform_yell` deferred rebuild of `total_hearts` before verdict
- [x] **Utility — present** — `shuffle` Fisher-Yates `src/engine.c:27`, `discard_until_count`, `play_baton_touch`/`double_baton_touch` `src/engine.c:297`, `choose_target_player` → `self_or_opponent`
- **Files:** `src/engine.c:182` verb dispatch (42/42 branches), `src/choice.c:1` emit
- **Still missing overall:** `src/effects_move.c`/`src/effects_look.c` split (currently collapsed in `engine.c`), per-zone unit tests, `make audit` still shows 42 distinct but handlers are stubs; will split and add fixtures before marking DONE

### Phase 7 — Parity harness & CI (IN PROGRESS — scaffold green, not parity)

**Scope:** Any behavioral drift from Rust is caught by automated replay.

- [x] `tests/replay.c:1` — 4 embedded fixtures (draw+score, live via mods, `move_cards` typed zones, condition 99→skip/1→fire) no-JSON dep, `make test` runs `rb_engine_test` + `rb_engine_replay` → `ALL REPLAY CHECKS PASSED`; fixture shape mirrors `engine/tests/**` `GameState`+`action`+`expected`
- [x] Property smoke in `tests/test_basic.c:47` random 300-turn walk never panics, zones in caps, winner decided or turn limit
- [ ] Golden snapshot — `cargo run --bin trace_game` dumps `LivePerformanceData`/`verdicts`/`rule_log` diffed against C `--trace` (`RB_TRACE`) for 100 seeds; plumbing is `make audit` verb census + `tests/replay.c` harness ready to ingest `tests/fixtures/*.json` when `cards/test_inventory.py --check` style dump lands (hand-author 20 fixtures: debut, LiveStart, LiveSuccess, cost-gated activation, baton, re-yell, mulligan, prohibition tie)
- [ ] JSON loader — `tests/replay_json.h` (jsmn or hand-rolled, no cJSON) to load `tests/fixtures/*.json` and drive `rb_advance_phase`/`rb_resume_with_choice` vs Rust oracle
- [ ] CI gate — `make regen` (`tools/gen_from_rs.py`+`gen_bytecode.py` → `git diff --stat` empty) and `cards/test_inventory.py --check` parity check not yet wired into GH Actions for `engine_c`
- **Wire into CI:** `make test` green is gating; will gate on `make audit && make test && make replay` with fixture diff before marking DONE

### Phase 8 — Portable targets (IN PROGRESS — allocator+streaming done, shims present but not CI)

- [x] **Allocator abstraction** — `RB_NO_MALLOC` bump-alloc fallback (`src/alloc.c:5` 512 KB arena): `rb_malloc`/`rb_free` route to `malloc` on hosted, to arena on bare metal. `rb_unload` no-op on arena; PC free-checked
- [x] **Data streaming** — `rb_load_streaming(dir, read_fn)` `src/data.c:134` alternative to `rb_load(dir)`: `fread` on hosted *or* streamed from ROM/CD/flash sector-by-sector via `read_fn` callback; 1 MB CD-i etc. stream `bytecode_blob`+`cards.bin` on demand (`rb_card_record`/`rb_bc_slice` cache)
- [x] **No `fopen`/`printf` in `src/`** except `src/data.c:rb_load` / `src/main.c`; `src/vm.c`/`src/engine.c` compile `-ffreestanding` clean
- [x] **Platform shims — files present** — `platforms/sdl/main.c` hosted reference (window+input→`Choice`), `platforms/cdi/cdi_main.c` bare-metal stub; each only `platform_read_file`/`platform_input_poll`/`platform_render_text`/`platform_random_seed`
- [ ] **Platform shims — still missing** — `platforms/cdi` linker script (`app at $8000`), `m68k-elf-gcc` build, 1 MB RAM profiling (heart tables precomputed, not malloc'd), `platforms/sdl` `Makefile` target `make -C platforms/sdl` + `RB_NO_MALLOC=1 make -C engine_c` CI matrix
- **Verify:** `RB_NO_MALLOC=1 make -C engine_c` builds; each shim (when landed) boots, seeds RNG, completes match within RAM budget (CD-i stream, not cache)

---

## 7. Incremental verification strategy

Don't wait for Phase 7 to test. After every phase, add a focused `tests/test_<phase>.c` that exercises only that phase's new code paths against a Rust-derived fixture. `all: rb_engine rb_engine_test replay` should stay green on `master`.

Suggested file layout after all phases:

```
engine_c/
  include/rabuka.h            # public API (stable, hosted + bare metal)
  include/rabuka_internal.h    # modifier / queue / choice internals
  src/data.c vm.c cards.c
  src/alloc.c                 # RB_NO_MALLOC bump arena vs malloc
  src/modifiers.c stats_pipeline.c
  src/condition.c choice.c ability_queue.c triggers.c
  src/phase.c live.c
  src/effects_move.c effects_look.c effects_state.c effects_ability.c compound.c
  src/engine.c main.c
  src/bytecode_blob.c gen_data.c   # generated
  tests/test_basic.c test_modifiers.c test_condition.c test_choice.c
        test_phases.c test_triggers.c test_live.c test_replay.c
  tools/gen_from_rs.py gen_bytecode.py audit_actions.c
  platforms/sdl/main.c        # hosted reference shim
  platforms/cdi/cdi_main.c    # bare-metal examples (one per target)
  Makefile
  PROGRESS.md
```

Add `tests/replay_json.h` (tiny JSON loader, no external deps — jsmn or hand-rolled) rather than pulling cJSON.

---

## 8. Cross-cutting concerns (do now, not later)

- **`strdup` / strings** — Use the local `rb_strdup` everywhere; don't mix with platform `strdup`. Free paths mirror alloc paths (`rb_free_ability`, `rb_free_condition`, `rb_free_card`); leak-check with `tests/test_free.c`.
- **Fixed caps vs. overflow** — Every `RB_MAX_*` cap must be checked before write; on overflow return `0` / log and drop the card to waitroom (matching Rust `shuffle`/`add_card` semantics). Never `assert`.
- **`RB_MAX_CHILD=64`, `RB_MAX_EXTRA=32`** — Current `vm.c` silently drops beyond cap; after Phase 6 audit whether any real ability exceeds 64 children (likely not — max observed is 11) and add a `log::debug!`-style `RB_TRACE` warning on drop.
- **`rb_unload` / `no_std` / alloc** — Keep heap usage explicit behind `src/alloc.c` (`RB_NO_MALLOC` → bump arena, otherwise `malloc`/`free`). No global C++ static init, no `fopen` in `src/vm.c`/`src/engine.c` — host I/O stays in `src/data.c:rb_load` / `rb_load_streaming`.
- **Tracer** — Add `RB_TRACE` compile flag that prints `[phase]`, `[condition verdict]`, `[choice offered]`, `[move src→dst]`, `[live allocation]` etc. gated on `getenv("RUST_LOG")`-style env var; leave lines in tree — cost 0 when off, priceless when debugging.
- **Byte regeneration** — `make regen` rule that runs both `gen_from_rs.py` + `gen_bytecode.py` and checks `git diff --stat` is empty (CI parity check). Document that `condition_decoder_gen.rs` / `effect_decoder_gen.rs` are auto-generated; edit `cards/generate_condition_decoder.py` not the output.

---

## 9. Risk register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Live allocation logic is 1:1 faithful or scores diverge silently | High — most bug reports are scoring disputes | Phase 5 gets the largest time budget + golden snapshots + extra Python oracle comparison pass |
| `move_cards` zone dispatch (RecentlyMoved/ThoseCards/LookedAtRemaining etc.) has subtle relay bugs | High — 338 abilities touch it | Phase 6 movement cluster is isolated into `effects_move.c` with per-zone unit tests, and every zone string is typed via `Zone::from_source_str` so typos are caught at decode time |
| Choice resume re-entrancy (resolver fields mutated mid-ability) | Medium — state corruption / double-trigger | Phase 3 queue entry snapshots `cost_paid`/`effect_started` flags; queue depth limited and checked; LIFO order tested |
| Modifier stacking (additive vs. set) saturates to wrong value | Medium — heartCount/score off-by-one | Phase 1 modifier tests + `saturate_u8` helper mirroring `constants::saturate_u8` (i32→u8 sanitize) |
| Generated bytecode drifts from Rust store | Low but build-breaking | `make regen` + CI `gen_from_rs.py --check` (like `cards/test_inventory.py --check`) |

---

## 10. How to build / run (PC host)

```bash
cd engine_c
# (re-)generate embedded tables from the Rust source-of-truth:
python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs
python3 tools/gen_bytecode.py

# build
make            # → ./rb_engine (demo match)
make test       # → ./rb_engine_test (ALL TESTS PASSED)
make audit      # → ./rb_engine_audit (verb/condition census)
./rb_engine src # demo: loads src/cards.bin + abilities_strings.bin + bytecode

# regen check (CI)
python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs --check
```

Toolchain: `gcc ≥ 9` / `clang` on PC, `m68k-elf-gcc` (CD-i), `arm-none-eabi-gcc` (GBA/DS/3DS) etc., all `-std=c11 -O2 -Wall -Wextra -Wpedantic -ffreestanding` clean. No C++ runtime. No external libs. `src/` compiles with `-DRB_NO_MALLOC` for bare-metal targets.

---

## 11. References

- `engine/src/ability/{resolver,choice,compound,cost}.rs` — ability lifecycle (resolve → pay → gate → execute → choice → resume → record use_limit)
- `engine/src/turn/{phases,live,actions,triggers}.rs` — phase machine + performance + victory
- `engine/src/core/{game_state/mod.rs,modifiers.rs,stats_pipeline.rs,player.rs,zones.rs}` — state + modifiers
- `engine/src/ability/enums.rs` — canonical ActionType / ConditionType / Zone wire tables
- `cards/abilities.json` — 936 unique abilities (source for audit counts above)
- `engine_c/tools/audit_actions.c` — live census of verbs/conditions actually present in bytecode
- `docs/PORTS.md` — per-target budgets (CD-i 1 MB wall is one data point; general engine is storage-agnostic via `rb_load_streaming`)

---

## 12. Function-by-function port matrix — exhaustive audit (2026-08-27)

> **Legend:** ✅ faithful — byte/logic identical · ⚠️ partial — structure correct, some branches stubbed/simplified · 🔴 stub — no-op / returns constant · ❌ missing — no C counterpart (planned or out-of-scope) · `GEN` = generated data, not hand-ported
> Every wire string (`action`, `Zone`, `ConditionType`, `Trigger`) is byte-identical to Rust (`engine/src/ability/enums.rs:11` and `engine/src/ability/vm.rs:8`); only the C symbol gets a `RB_`/`rb_` prefix (see §2.5 naming rules). File paths are the `rg` grep map.

**How to read:** `Rust file:line` → `C counterpart:line` → status. Counts are sampled 2026-08-27; LOC from `cargo`/`wc -l` (engine ~63k total, C ~3.9k).

### 12.1 Decoder — bytecode ↔ structs (the ABI)

| Rust source | LOC | Key functions / types | C counterpart | Status | Notes |
|-------------|-----|-----------------------|---------------|--------|-------|
| `ability/vm.rs:8` — tag bytes, `read_u8/u16/u32`, `rd_len` (`b<0xFE?b:u16`), `rd_idx`, `rd_int`, `read_string`, `skip_value` | 1571 | `DecodeError`, `ability_count`, `decode_ability`, `normalize_cost_keys`, `populate_from_json` | `src/vm.c:14` `rd_u8/rd_u16/rd_u32/rd_len/rd_idx/rd_int/rd_string_val/skip_value/skip_one` + `read_condition:168` + `decode_effect_body:222` | ✅ faithful | Varint encoding mirrored exactly; `RB_TAG_*` `include/rabuka.h:8` identical |
| `ability/condition_decoder_gen.rs:1` | 1489 | `decode_condition*` LUTs (generated by `cards/generate_condition_decoder.py`) | `src/vm.c:168` `read_condition` (hand-rolled) | ⚠️ partial | C drops nested `effect`/`options`/`look_action` etc. via `skip_value` to avoid bogus tree — loses some condition-inside-effect nesting |
| `ability/effect_decoder_gen.rs:1` | 547 | `EffectKindLocals`, `decode_effect*` | `src/vm.c:222` `decode_effect_body` | ✅/⚠️ | Handles `text/action/source/destination/target/count/condition/optional/is_further` + `actions`/`effect_steps`/`look_action`/`select_action`/`primary_effect` etc.; caps at `RB_MAX_CHILD 64`/`RB_MAX_EXTRA 32` (silent drop beyond) |
| `ability/types.rs:1` | 1066 | `TriggerEvent`, `Choice`, `ChoiceRoute`, `ExecutionContext`, `StageSelectIntent`, `ChoiceBuilder` | `include/rabuka.h:255` `RbChoice/RbChoiceKind/RbQueueEntry/RbAbilityQueue` + `src/choice.c:1` | ⚠️ partial | Choice kinds `SelectCard/SelectTarget/SelectHeartColor` present; `SelectAutoAbility` ordering and `StageSelectIntent::ChangeStateWait` not yet |
| `ability/enums.rs:11` | 793 | `Zone::from_str/to_str` (30 variants), `TargetPlayer`, `PlacementTarget`, `ActionType` (60), `ConditionType` (35), `SelectTargetKind` | `include/rabuka.h:210` `RbZone` (8 collapsed) + `src/engine.c:35` `rb_zone_of_str` + `src/vm.c` condition variant mapping | ⚠️ partial | Zones `UnderMember/SameArea/EmptyArea/LookedAt/RevealedCards/ThoseCards/RecentlyMoved/LookedAtRemaining/DeckTopOrBottom` mapped but collapsed to `RB_ZONE_*` (8); `TargetPlayer both/either→self` stub; `PlacementTarget` not typed |
| `core/card.rs:1` + `core/card_binary.rs:1` | 3713+597 | `CardType`, `HeartColor`, `Card`, `CardDatabase`, `Ability`, `AbilityEffect`, `EffectKind` + 60 `accessor()` helpers + `decode_card_from_blob` | `src/cards.c:12` `rb_decode_card_by_index` + `include/rabuka.h:162` `Card` + `src/data.c:80` `load_cards` | ✅ faithful | 25-byte header decode (`card_no_idx@0`, `ability_idx@16`, `cost@19` etc.); hearts linear loop; `has_special 0x04` |
| `core/constants.rs:5` | 40 | `STAGE_SIZE 3`, `MAX_LIVE_CARDS 3`, `VICTORY_CARD_COUNT 3`, `MAX_ENERGY_CARDS 12`, `saturate_u8/i16` | `include/rabuka.h:90` `RB_STAGE_SIZE` etc. + `rb_saturate_u8:104` | ✅ faithful | Constants identical, only `RB_` prefix |

### 12.2 Core state — `GameState` / `Player` / `Zones` / `Modifiers` / `StatsPipeline`

| Rust source | LOC | Key functions | C counterpart | Status | Notes |
|-------------|-----|---------------|---------------|--------|-------|
| `core/types.rs:1` | 1046 | `ArcStr`, `AbilityTrigger`, `TurnPhase`, `Phase`, `ZoneId`, `Duration`, `TemporaryEffect`, `LivePerformanceData`, `MovementEvent` | `include/rabuka.h:303` `RbLiveSnapshot` + `RbPhase` + `RbZone` | ⚠️ partial | `LivePerformanceData` → `RbLiveSnapshot {total_hearts[8],total_score,success,surplus_hearts,note_icons}` faithfully extended this commit; `Duration`/`TemporaryEffect` not yet |
| `core/zones.rs:1` | 842 | `Orientation`, `MemberArea`, `Stage{stage[3],under_cards[3]}`, `LiveCardZone`, `EnergyZone{pay_energy/activate_all}`, `MainDeck{shuffle/draw}`, `Hand`, `Waitroom` | `include/rabuka.h:210` `RbPlayer{stage[3],stage_wait[3],hand/deck/energy/live/success/discard}` + `src/engine.c:16` `bag_*` + `rb_shuffle:27` | ⚠️ partial | Bags are fixed `int cards[512]` not `Vec<i16>`; `Stage.under_cards` not typed; `EnergyZone.active_count` → `energy_active` int |
| `core/player.rs:1` | 421 | `Player::new/draw_card/draw_energy/refresh/calculate_stage_hearts/activate_all_energy/track_deployment/remove_member_from_stage_with_recycling` | `src/engine.c:66` `rb_draw/rb_draw_energy/rb_shuffle` + `src/live.c:18` `rb_calc_stage_hearts` | ⚠️ partial | `refresh` (deck-empty→shuffle waitroom) missing in C draw path; `track_deployment` bool present but not wired to baton protection |
| `core/game_modifiers.rs:1` | 486 | `ModifierEntry{set,add,total()}`, `GameModifiers::add/get/remove blade/heart/score/need_heart/cost/orientation`, `add_delayed_cannot_active/tick_delayed_cannot_active_for/clear_all_for_card` | `include/rabuka.h:119` `RbMods/RbModifierEntry` + `src/modifiers.c:1` `rb_mods_*` (14 helpers) | ✅ faithful | `set+add` saturating `i16`, `heart_copy=-1`/`heart_multiplier=-1` sentinel init; `tick_delayed_for` does owned-set filter exactly like Rust |
| `core/game_state/mod.rs:1` | 1291 | `GameState{player1,player2,turn_number,current_phase,mods,player1_cheer_*}` + `active_player/first_attacker/find_card_stage_position/push_revealed_card/push_performance_snapshot` | `include/rabuka.h:315` `GameState{p[2],turn,phase,mods,queue,snapshots[64]}` + `src/engine.c:490` `rb_game_init` | ⚠️ partial | `p[2]` compact vs `player1/player2`; `turn_number→turn`; `revealed_cards`/`recently_moved` pools not yet; `performance_snapshots` → `RbLiveSnapshot[64]` |
| `core/game_state/modifiers.rs:1` | 1881 | `recalculate_constants/clear_area_placement_tracking/record_card_appearance/record_baton_touch/push_movement_event/set_recently_moved_batch` | `src/triggers.c:20` `rb_recalc_constants` + `src/modifiers.c` clear | 🔴 stub/⚠️ | C clears `constant_blade/score` then scans stage `常時` for `modify_score/gain_resource` only; per-card heart/need_heart constants, `heart_override`, `heart_copy` persistence missing |
| `core/game_state/tracking.rs:1` | 110 | `reset_keyword_tracking/add_yell_count_modifier/effective_cheer_checks_required` | `src/live.c:35` `do_yell` (yell_count fixed 1) | 🔴 stub | Yell count/source modifiers not wired |
| `core/stats_pipeline.rs:1` | 269 | `member_original_hearts/apply_additive_heart_mods/effective_blade/stage_hearts/effective_need_heart` | `src/stats_pipeline.c:1` `rb_effective_need_heart/rb_stage_hearts_pipeline` + `src/live.c:18` `rb_calc_stage_hearts` | ⚠️ partial | `effective_need_heart` faithful (base + `need_heart` saturated); `stage_hearts_pipeline` adds `heart_copy` source on top (should replace) and `*2` multiplier stub |
| `core/pool.rs:1` | 151 | `Pool<T>{alloc/free}`, `EkBox` via `define_pool!` | `src/alloc.c:5` `rb_malloc/rb_free/rb_strdup2` + `RB_NO_MALLOC` 512 KB bump arena | ✅ faithful (abstraction) | No Rust `Box/Vec` in C; arena `rb_free` no-op, PC `malloc/free` |
| `core/game_state/abilities.rs:1` | 2820 | `ability_matches_trigger/record_ability_use/collect_constant_hand/trigger_auto_abilities_for_player/process_pending_auto_abilities/check_expired_effects` | `src/ability_queue.c:1` + `src/triggers.c:1` | ❌ missing | Auto-trigger engine not yet (debut/auto/live_start/live_success queuing, `use_limit`, `effect_started` flags) |
| `ability/ability_store.rs:1` + `abilities_gen.rs:1` + `cards_gen.rs` | 70+64+12 | `AbilityRef`, `ABILITIES`, `get_string`, `CARD_ABILITY_PAIRS` | `src/gen_data.c:1` `RBKA_NUM_ABILITIES/RBKA_OFFSET_DELTAS/RBKA_STRINGS_OFFSETS` (generated by `tools/gen_from_rs.py`) + `src/bytecode_blob.c` `RBKA_BYTECODE[92901]` (from `tools/gen_bytecode.py`) | ✅ faithful | Generated artifacts, never hand-edited (`condition_decoder_gen.rs` etc. are `GEN`) |

### 12.3 Ability runtime — queue / resolver / choice / compound / cost / util

| Rust source | LOC | Key functions | C counterpart | Status | Notes |
|-------------|-----|---------------|---------------|--------|-------|
| `ability_queue.rs:1` | 695 | `AbilityQueue::new/clear/is_idle/is_waiting_for_choice/current_entry/enqueue/start_next/pause_for_choice/resume_with_choice/complete_current/push_constant_context/promote_entry/take_resolver/dump_state`, `AbilityQueueEntry{cost_paid,cost_paid_index,conditional_choice,pending_actions,resolver}` | `include/rabuka.h:273` `RbAbilityQueue{entries[16],use_keys[256]}` + `src/ability_queue.c:1` `rb_queue_push/clear/has_pending/use_limit_reached/record_use` | ⚠️ partial | Depth 16 / per-turn `use_limit` faithful; `ChoiceRoute`/`ConditionalChoice`/`AbilityResolver`/`pending_actions`/`resolver` boxing not yet |
| `ability/resolver.rs:1` | 1195 | `AbilityResolver::new/get_pending_choice/cached_condition_verdict/store_condition_verdict/can_activate_effect/resolve_ability/card_matches_type` | `src/engine.c:185` `rb_execute_effect` + `src/choice.c:1` | ⚠️ partial | Resolver collapsed into `rb_execute_effect` pre-order walk; condition caching (`condition_cache`) not yet |
| `ability/choice.rs:1` | 3375 | `SelectionContext::mfi`, `resume_execution/resume_pending_actions/provide_choice_result`, `Choice::{select_cards,SelectPosition,SelectAutoAbility}` | `src/choice.c:1` `rb_has_pending_choice/get_pending_choice/clear_pending/resume_with_choice/emit_choice` + `src/effects_look.c:1` `LookPool g_look[2]` | ⚠️ partial | `SELECT_CARD`/`SELECT_TARGET`/`SELECT_HEART_COLOR` present; `SelectionContext::mfi` multi-filter-index, `pending_repeat_actions`, host auto-drain `rb_resume_with_choice(g,-1)` |
| `ability/compound.rs:1` | 981 | `route_conditional_branch/execute_sequential_effect/execute_conditional_alternative/execute_conditional_on_result/execute_conditional_on_optional/handle_choice_string_selection` | `src/engine.c:185` `rb_execute_effect` children loop + `src/engine.c:301` `choice/conditional_*` branches | ⚠️ partial | `sequential` children executed pending-gated; `conditional_alternative`/`conditional_on_result`/`conditional_on_optional` emit `SELECT_TARGET`; `repeat_procedure` loops once not `cnt`× |
| `ability/cost.rs:1` | 1345 | `pay_deferred_costs/validate_cost/pay_cost/handle_optional_cost_payment` (sequential_cost, `has_skip_prompt`, `get_change_state_candidates`) | ❌ missing | No `pay_cost` gate in C; `rb_play_member:318` pays base `cost` not `cost_modifiers`, no `deferred_costs`/`optional_cost_result` |
| `ability/util.rs:1` | 2496 | `CardFilter{matches, from_effect}`, `count_in_zone/resolve_per_unit_count/push_temporary_effect/zone_cards/matching_indices/compare_counts/move_card/place_card_in_zone` | `src/effects_move.c:12` `card_matches_filter` + `src/engine.c:114` `card_matches_card_type_filter/heart_color_of/extra/do_move_filtered` | ⚠️ partial | `CardFilter{card_type,group_names,card_names}` substring match present; `card_property`/`cost`/`characters`/`exclude_self`/`ability_filter` not yet |
| `ability/dynamic_count.rs:1` | 177 | `resolve_dynamic_count` (`perUnit`, `countFromZone`, `any_number`) | ❌ missing | Dynamic `count` (`per_unit_count`, `any_number`) treated as `count>=0?count:1` |
| `ability/effects/draw.rs:1` | 713 | `execute_draw/_until_count/execute_select_*(heart_color/number/area)/make_card_effect_data` | `src/engine.c:212` `draw/draw_card/draw_until_count/discard_until_count` | ✅ / ⚠️ | `draw`/`draw_until_count`/`discard_until_count` faithful; `select_heart_color`/area selects stub through `look` pool |
| `ability/effects/score.rs:1` | 803 | `execute_modify_score/modify_required_hearts/_global/_success/modify_yell_count/modify_limit` | `src/effects_state.c:49` `rb_effect_modify_hearts` + `src/engine.c:232` `modify_score/gain_score` | ✅ / ⚠️ | `modify_score` faithful; `modify_required_hearts` adds to first staged/hand only (not global); `modify_yell_count/source` no-op |
| `ability/effects/state.rs:1` | 1744 | `execute_change_state/energy_placement/set_cost/blade_type/heart_type/set_card_identity/reduce_live_card_set_limit/specify_heart_color` | `src/effects_state.c:9` `rb_effect_change_state/position_change/modify_cost/modify_hearts` + `src/engine.c:260` `set_card_identity/set_blade_type/set_heart_type` branches | ⚠️ / 🔴 | `change_state` touches first staged only; `set_blade_type/heart_type/card_identity` trace-only; `place_energy_under_member` via `gain_resource` |
| `ability/effects/misc.rs:1` | 4120 | `execute_gain_resource/play_baton_touch/place_energy_under_member/position_change/rotation/choice/pay_energy/discard_until_count/restriction/re_yell/perform_yell/shuffle` | `src/engine.c:224` `gain_resource/place_energy/pay_energy/shuffle/reduce_live_card_set_limit/position_change/play_baton_touch` + `src/effects_state` splits | ⚠️ / 🔴 | 25 handlers: ~10 faithful, rest stub; `re_yell`/`perform_yell` deferred rebuild not yet |
| `ability/effects/ability_effects.rs:1` | 534 | `execute_gain_ability/_from_source/activate_ability/invalidate_ability/suppress_ability_trigger` | `src/effects_ability.c:7` `rb_gain_ability/rb_invalidate_ability/rb_tick_gained` | 🔴 stub | Tracks `Gained{target,score,turns=2}` as score mod (not real `TemporaryEffect`/`Duration`/`revocation`) |
| `ability/look.rs:1` | 1159 | `execute_look_and_select/reveal/select/select_cards/look_at/reveal_per_group/reveal_until_*` | `src/effects_look.c:24` `rb_effect_look_at/select_cards/look_resume` + `LookPool g_look[2]` | ⚠️ partial | `look_at` fills `g_look[who]` from deck/hand and emits `SELECT_CARD zone="looked_at"`; `revealed_cards` vs `looked_at` distinct, `keep_shuffle_under` 2-phase missing; `rb_look_resume` not wired to `rb_resume_with_choice` |
| `ability/move_cards.rs:1` | 3664 | `drain_under_cards_to_energy_zone/MoveCardsTarget/optional_gate_source/execute_move_cards/handle_select_position/execute_selected_cards_from_zone` | `src/effects_move.c:38` `rb_effect_move_cards` | ⚠️ partial | Typed `RbZone` dispatch with `count`/`deck_top_or_bottom`→`toTop`, `card_type`/`group` filter; `those_cards/RecentlyMoved/LookedAtRemaining` relay→`hand` stub; `count=-1` drain-all, `under_member/same_area/empty_area` placement, baton `cost_modifiers` missing |
| `ability/describe.rs:1` | 1200 | `describe_effect_en/ja`, `zone_label`, `translate_choice_prompt_en_to_ja` | ❌ missing | Display only — out-of-scope for `engine_c` (host renders via `src/main.c`) |
| `ability/debug.rs:1` + `ability/log.rs:1` | 147+101 | `AbDebug`, `AbilityLogItem{push_verdict/drain_verdicts}` | ❌ missing | `RUST_LOG=debug` tracing — planned `RB_TRACE` flag (§8) |

### 12.4 Conditions — gated effect predicates (20 variants + `negation`)

| Rust source | Variant | Wire key | Rust evaluator | C evaluator | Status |
|-------------|---------|----------|---------------|-------------|--------|
| `ability/condition/card.rs:1` | `GroupCondition` (4) | `group_condition` | `group_names[]` substring vs stage | `src/condition.c:217` `eval_group` scans `stage/hand/discard` `group_idx` strstr | ✅ faithful |
| `ability/condition/card.rs` | `CardBladeCondition` (8) | `card_blade_condition` | blade count filter | `eval_resource:294` `stage count` stub | 🔴 stub |
| `ability/condition/card.rs` | `PositionCondition` (13) | `position_condition` | left/center/right vs zone | `eval_location` delegate | ⚠️ partial |
| `ability/condition/card.rs` | `AppearanceCondition` (5) | `appearance_condition` | `characters[]` group on stage | `eval_appearance:246` `stage>0` stub | 🔴 stub |
| `ability/condition/card.rs` | `AbilityFilterCondition` (9) | `ability_filter_condition` | ability text contains | `eval_ability_filter:314` always 1 | 🔴 stub |
| `ability/condition/card.rs` | `HighestCostOnStageCondition` | `highest_cost_on_stage_condition` | max cost compare | `eval_resource` stub | 🔴 stub |
| `ability/condition/card.rs` | `AllCostComparisonCondition` | `all_cost_comparison_condition` | all cost compare | `eval_resource` stub | 🔴 stub |
| `ability/condition/compound.rs:1` | `Compound` (0) | `compound` | `operator/and|or` + `conditions[]` recurse | `src/condition.c:124` `eval_compound` (`or` vs `and`) | ✅ faithful |
| `ability/condition/compound.rs` | `OrCondition` (9 alt) | `or_condition` | `any_of` | same | ✅ faithful |
| `ability/condition/compound.rs` | `AnyOf/Both/Otherwise` | `any_of_condition/both_condition/otherwise_condition` | combinators | `eval_choice/complex` always 1 | 🔴 stub |
| `ability/condition/state.rs:1` | `LocationCondition` (1) | `location_condition` | `location/locations[]`, `count+operator`, `distinct`, `card_type` filter | `src/condition.c:140` `eval_location` (handles `distinct` bool + nested OBJVAR, `card_type` via `zone_count_filtered`) | ⚠️ partial | `all` flag as count check |
| `ability/condition/state.rs` | `CardCountCondition` (1 alt) | `card_count_condition` | `distinct` names | `count_distinct_in_zone:71` via `Card.name` string distinct (faithful) | ⚠️ partial |
| `ability/condition/state.rs` | `ComparisonCondition` (2) | `comparison_condition` | `comparison_source` vs `count` + `operator` | `eval_comparison:176` (`comparison_source` hand count if present) | ⚠️ partial |
| `ability/condition/state.rs` | `MovementCondition` (3) | `movement_condition` | `has_moved/not_moved` via `recently_moved_cards` | `eval_movement:203` always 0 for `has_moved`/`1` for `not_moved` | 🔴 stub (no `recently_moved` tracking) |
| `ability/condition/state.rs` | `TemporalCondition` (6) | `temporal_condition` | `turn_number+operator`, `phase` | `eval_temporal:256` `g->turn` vs threshold + `main/active/live` | ⚠️ partial |
| `ability/condition/state.rs` | `StateCondition` (7) | `state_condition` | `state active/wait` vs orientation | `eval_state:274` counts `stage_wait` + `orientation` mods | ⚠️ partial |
| `ability/condition/state.rs` | `EnergyStateCondition` (8) | `energy_state_condition` | `energy` count | `eval_resource:294` stage-count stub | 🔴 stub |
| `ability/condition/state.rs` | `ScoreThresholdCondition` (10) | `score_threshold_condition` | `score` vs `count+operator` | `eval_score:318` `p[pl].score` vs threshold | ✅ faithful |
| `ability/condition/state.rs` | `NoExcessHeart` (16) | `no_excess_heart` | `surplus_hearts==0` via `LivePerformanceData` | `eval_no_excess:302` reads `snapshots[].surplus_hearts==0` (new) | ✅ faithful |
| `ability/condition/state.rs` | `StateChangeCondition` / `OpponentChoice` / `OpponentLiveSuccess` / `AllRevealed` | misc | various | `eval_*` stubs (always 0/1) | 🔴 stub |
| `ability/condition.rs:1` | — | `negation` flip | `condition.negation` invert | `eval_condition_inner:329` `negation? !r` | ✅ faithful |
| — | — | gating | `has_condition && !eval → skip` before children | `src/engine.c:188` `if(has_condition && !rb_eval_condition) return` pending-gated per child | ✅ faithful |

**Condition wiring:** `src/condition.c:1` `rb_eval_condition` (`variant 0..19` + `find_val/get_str/get_i/get_bool/eval_operator/target_player_idx`) is the single dispatch; `src/vm.c:168` `read_condition` is the decoder. Caps `RB_MAX_COND_FIELD 64`/`RB_MAX_COND_ARR 32` (silent drop beyond).

### 12.5 Phases / Live / Triggers — turn state machine

| Rust source | LOC | Key functions | C counterpart | Status | Notes |
|-------------|-----|---------------|---------------|--------|-------|
| `turn/phases.rs:69` | 1612 | `advance_phase`, `handle_mulligan_selection/confirmation/skip`, `handle_set_live_card/live_card_selection/confirmation/skip`, `handle_play_member_to_stage`, `handle_rps_choice_p1/p2`, `check_timing`, `log_phase/turn_start` | `src/phase.c:1` `rb_advance_phase` + `src/engine.c:370` `activate_wait_members/main_phase/live_phase/check_victory/rollover/rb_turn` + `src/engine.c:490` `rb_game_init` | ⚠️ partial | Linear `RPS/OPENING→ACTIVE→ENERGY→DRAW→MAIN→LIVE_SET→PERFORMANCE→VICTORY→DONE`; two-attacker flip faithful (fixed `static main_count` bug); mulligan no-op; `LiveCardSetFirst/Second` + `First/SecondPerformance` sub-phases collapsed; `check_timing` (constant re-eval hooks at Active→LiveSet→Performance) missing |
| `turn/actions.rs:1` | 1494 | `execute_main_phase_action/with_ability_index`, `resume_with_choice`, `check_timing`, `check_victory_condition` | `src/engine.c:318` `rb_play_member/rb_activate_ability/rb_play_card/main_phase` | ⚠️ partial | Main auto-play loop 64-guard + staged-ability one-pass; `resume_with_choice` is `rb_resume_with_choice` stub; `check_victory_condition` → `check_victory:448` (3-success/`score>=7`/deck-out tie faithfully but no `prohibition_effects`) |
| `turn/live.rs:1` | 2717 | `player_perform_live`, `compute_allocations`, `check_live_success`, `process_yell_revealed_card_icons`, `execute_live_victory_determination`, `move_live_to_success_and_handle_wins`, `build_snapshot`, `enrich_from_applications` | `src/live.c:1` `rb_calc_stage_hearts:18`, `do_yell:35` (per-live yell), `allocate_and_verdict:66` (greedy `icon_all` + `surplus`), `rb_perform_live:127` + `src/stats_pipeline.c:1` | ⚠️ partial | Yell `per_live=1` fixed (ignores `modify_yell_count/source`); `BAll` doubling/`set_blade_type` recolor missing; allocation is greedy `heart0 any 1..6+icon_all` then per-color deficits (fast `total_pool<total_req` fail); `re_yell`→`pending_reyell_rebuild`, `LiveSuccess` trigger → `drain_pending_live_success_choices`, `prohibition_effects` tie not yet |
| `turn/triggers.rs:1` + `triggers.rs:1` | 586+146 | `trigger_debut_abilities/trigger_live_start_abilities/trigger_auto_abilities_for_player/TriggerKind::canonical_trigger` | `src/triggers.c:20` `rb_trigger_is/trigger_debut/recalc_constants` | ⚠️ partial | `rb_trigger_is` strstr for `"登場"/"常時"/"自動"/"ライブ開始時"/"ライブ成功時"` faithful; `rb_trigger_debut` queues `ability_idx 0` after `use_limit` check; auto/live_start/live_success queuing, multi-ability `常時` heart/need_heart constants missing |
| `ability_queue.rs` states | 695 | `QueueState::{Idle,WaitingForAutoAbilityChoice,PayingCost,WaitingForChoice,ExecutingEffect,Completed}` + `snapshot_requested` | `include/rabuka.h:273` `RbAbilityQueue` flat 16-depth + `src/ability_queue.c` `push/clear/use_limit` | ⚠️ partial | Depth 16 + per-turn `use_limit` faithful; `QueueState` FSM, `snapshot_requested`, `choice_player_id`, `resolver` boxing not yet |

### 12.6 Game / Bot / Infra — out-of-scope or deferred

| Rust source | LOC | Purpose | C counterpart | Status | Notes |
|-------------|-----|---------|---------------|--------|-------|
| `game/game_setup.rs:1` | 1927 | `setup_game/build_two_decks/ActionType/Action/is_automatic_phase/generate_possible_actions` (legal-action gen) | `src/engine.c:490` `rb_game_init` + `src/main.c:12` host demo | ⚠️ partial | Decks from 40 ability-bearing cards + xorshift determinism; `generate_possible_actions` (legal-move enumeration for bot headless `search`) not in C — host `main_phase` greedy loop is placeholder |
| `game/match_runner.rs:1` | 306 | `run_match`, `ai_turn`, `MatchMode` | `src/engine.c:472` `rb_turn` + `rollover` | ⚠️ partial | No `MatchMode`/`ai_turn`; `rb_turn` is one-round `ACTIVE…VICTORY` unrolled loop |
| `game/deck_parser.rs:1` / `deck_builder.rs:1` | 232+135 | `DeckParser::parse_deck_file`, `DeckBuilder::build_deck_from_database` | ❌ missing | Decks are caller-supplied `uint32_t deck0[]` in C (`rb_game_init`); parsing out-of-scope |
| `game/display.rs:1` | 1917 | `GameStateDisplay`, `card_to_display`, `zone_to_display` (DTOs for frontend) | ❌ out-of-scope | Host `src/main.c:51` prints minimal `rb_print_state` |
| `game/web_server.rs:1` | 2541 | Actix-web `FrameSnapshot`, `Room`, `run_web_server` | ❌ out-of-scope | `engine_c` is game logic only; `platforms/sdl/main.c` is the hosted reference |
| `game/menu.rs:1` + `platform_ui.rs:1` | 519+227 | CLI menu, `PlatformUi::wrap_text/heart_str` | `platforms/sdl/main.c` + `platforms/cdi/cdi_main.c` stubs | ⚠️ stub | Shims present but not CI |
| `bot/*.rs` (13 files) | ~3900 | `Bot::choose_action`, `PublicObservation`, `ISMCTS`, `strategy{,_v2..v7}`, `encoding`, `neural`, `determinization`, `conductor` | ❌ out-of-scope | Pure consumer of `GameState`; keep Rust or reimplement after engine stable |
| `lib.rs:1` / `main.rs:1` | 67+292 | crate root, binary entry | `src/main.c:12` `main` | ✅/⚠️ | PC demo loads `src/cards.bin`+bytecode, seeds `0xCAFE`, runs match to `winner` |
| `compat.rs:1` / `rng.rs:1` | 149+160 | `PspHasher`, `Lcg`, `seed/shuffle_slice` | `src/engine.c:6` `rb_seed/rb_rand/rng_range`, `src/data.c:14` `le32/le16` | ✅ faithful | xorshift `x ^= x<<13; x ^= x>>17; x ^= x<<5` Miri-verified; PSP `no_std` shims via `RB_NO_MALLOC` need not copy `PspHasher` |
| `timer.rs:1` / `alloc_counter.rs:1` / `bin_common.rs:1` | 138+169+67 | `Timer::start`, `CountingAllocator`, `deal_game` | ❌ out-of-scope | Perf/diagnostic only; planned `RB_TRACE` (§8) |
| `bin/*.rs` (16 tools: `trace_game`, `turn_replay`, `harness`, `bot_arena` …) | ~3700 | Harnesses for parity debugging | `tests/replay.c:1` + `tests/test_basic.c:1` + `tools/audit_actions.c:1` | ⚠️ partial | `trace_game` oracle for `LivePerformanceData` snapshot diff is the planned golden parity harness (not yet wired to `tests/fixtures/*.json`) |

### 12.7 Generated / build artifacts — never hand-ported

| Rust artifact | Generator | C artifact | Status |
|---------------|-----------|------------|--------|
| `cards/cards.json` (2526 cards) → `cards.bin` | `cards/compile_cards.py` | `src/cards.bin` + `src/data.c:load_cards` | ✅ faithful |
| `cards/abilities.json` (936 unique) → `abilities_strings.bin` + `abilities.bin.z` (92901 B) | `cards/compile_abilities.py` → `cards/build/abilities_gen.rs` + `cards/build/abilities_strings.bin` | `src/bytecode_blob.c:1` `RBKA_BYTECODE[]` + `src/gen_data.c:1` `RBKA_OFFSET_DELTAS[936]/RBKA_STRINGS_OFFSETS[5717]` (via `tools/gen_from_rs.py` + `tools/gen_bytecode.py`) | ✅ faithful |
| `condition_decoder_gen.rs` (1489 LOC) | `cards/generate_condition_decoder.py` | `src/vm.c:168` `read_condition` (hand) — edit the **generator**, never the output | ✅ convention |
| `effect_decoder_gen.rs` (547 LOC) | `cards/generate_effect_decoder.py` | `src/vm.c:222` `decode_effect_body` | ✅ convention |
| `cards_gen.rs` / `decks_cards_gen.rs` | `tools/bake_deck_cards.py` | `src/data.c:load_cards` / caller-supplied decks | ✅ convention |

### 12.8 Current totals and next work

**Decoder is byte-identical — 936 abilities + 2526 cards decode losslessly.** Execution is a runnable skeleton: `make && make test && ./rb_engine src` is green (`ALL TESTS PASSED` + `ALL REPLAY CHECKS PASSED` 12 scenarios), but many subsystems are collapsed:

| Category | Faithful | Partial | Stub | Missing | Notes |
|----------|----------|---------|------|---------|-------|
| Decoder (7 files, ~3.9k) | 4 | 3 | 0 | 0 | Wire strings + varints + card header faithful |
| Core state (10 files, ~7k) | 4 | 6 | 1 | 1 | Modifier stack + saturate faithful; `recalculate_constants`, Done |
| Conditions (5 files, ~6k) | 5 variants | 9 | 9 | 0 | Compound/group/score/no_excess/negation faithful; movement/ability_filter etc. stub |
| Effects verb dispatch (42 verbs, ~12k across 7 files) | 12 verbs | 18 | 12 | 0 | `draw/modify_score/pay_energy/shuffle/discard` faithful; `move_cards/change_state/look` partial; `set_blade_type/gain_ability` trace-only |
| Ability runtime (queue/resolver/choice/compound/cost/util, ~8k) | 2 | 6 | 0 | 2 | Queue depth/`use_limit` faithful; `AbilityResolver`/`Cost` payment missing |
| Phases/Live/Triggers (4 files, ~4.8k) | 1 path | 3 | 0 | 0 | Two-attacker flip faithful (fixed `static main_count`); mulligan + `check_timing` + `re_yell` missing |
| Game/Bot/Infra (~9k) | 1 | 2 | 1 | 13 | Demo harness only; web_server/bot/display out-of-scope for `engine_c` |

**The next commits that move the needle (in order):** (1) ~~`check_timing` hooks at `Active→Energy→Draw→LiveSet→Performance` + `constant` heart/need_heart recalc (covers `q127_wien_*`)~~ — **DONE**: `rb_check_timing` (+ `rb_check_victory_condition`, `rb_check_invalid_live_cards`, `rb_check_invalid_energy_cards`, `rb_check_orphaned_under_cards`, `rb_check_invalid_resolution_zone`, `rb_check_permanent_loop`, `rb_player_refresh`) added to `src/turn/phase.c`; `RbPlayer.under_cards[3]` + `GameState.resolution` added to `rabuka.h`; wired at the three phase transitions that Rust calls it (Active, Draw, LiveCardSet). `recalculate_constants` was already in `rb_recalc_constants`. (2) ~~full `ability/cost.rs` pay gate (`sequential_cost`, `optional_cost`, `change_state wait`)~~ — **DONE (headless subset)**: `src/ability/cost.c` rewrites `rb_pay_cost`/`rb_validate_cost` to recurse `sequential`/`sequential_cost` and pay leaf costs (`pay_energy` taps `energy_active`, `change_state` wait → `stage_wait`, `move_cards` cost → discard, `energy_condition`/reveal validated by zone count). Optional costs auto-skip (cost.rs skip path); per-cost `cost_paid_index` resumption + interactive prompts deferred to the choice FSM. (3) `turn/live.rs` `re_yell`→`pending_reyell_rebuild` + `LiveSuccess` trigger → `compute_surplus_and_flags` oracle diff against `cargo run --bin trace_game`, (4) `effects/move_cards` relay pools (`ThoseCards/RecentlyMoved/LookedAtRemaining`) + `effects/look` `keep_shuffle_under`, (5) ~~`ability_queue` `QueueState` FSM + `ChoiceRoute`/`ConditionalChoice` routing~~ — **DONE (scaffold)**: `RbQueueState` FSM (Idle/Resolving/AwaitingChoice/Draining) + `RbChoiceRoute` enum added; `rb_queue_set_state`/`rb_queue_state` + `rb_choice_set_route`; `rb_drain_ability_queue` transitions the FSM (yield→AwaitingChoice on pending choice). Full per-cost `cost_paid_index` routing + `ChoiceRoute` tagging at each emit site deferred (interactive-prompt plumbing). No restructuring needed — each slots into `src/phase.c:rb_advance_phase`, `src/live.c:rb_perform_live`, `src/effects_*`, `src/choice.c`, `src/ability_queue.c`.

*This §12 is the single source of truth for the C rewrite. Every row corresponds to a Rust file/function that will land as a traced commit; tick it off as each phase lands.*

---

*This document is the single source of truth for the C rewrite. Every `-[ ]` above corresponds to a file/task that will land as a separate commit on top of `engine_c-v0`. Keep this file updated as each phase lands; the per-phase checklists are the PR checklists.*

---

## 13. Build & toolchain (machine notes — 2026-08-28)

`engine_c/` is a from-scratch C11 port of `engine/src/`. The mature Rust source is the translation reference; `engine_c/` mirrors its layout file-by-file.

**Toolchain.** The `Makefile` uses `CC ?= gcc`. On this machine `gcc` resolves to the MSYS2 / Cygwin-w64 GCC 15.3.0 shipped with devkitPro (`/opt/devkitpro/msys2/usr/bin/gcc`, Windows path `C:\devkitPro\msys2\usr\bin\gcc.exe`). Flags are fixed: `-std=c11 -O2 -Wall`, include paths `-Iinclude -Isrc -Isrc/core/generated`. (No `gcc`/`make`/`clang` is on the default `PATH` — invoke the devkitPro msys2 `gcc.exe` directly, or `cl.exe` from MSVC BuildTools if msys2 is unavailable.)

**Build targets.**
- `make all` → compiles every `.c` in `SRC` to `.o` (pattern rule `src/%.o: src/%.c`) and links `rb_engine` (headless demo: seeds RNG `0xCAFE`, runs a match to a winner).
- `make test` → builds four test binaries against the library objects (`OBJ_LIB`, which excludes `main.c` so tests supply their own `main`):
  - `rb_engine_test` ← `tests/test_basic.c`
  - `rb_engine_replay` ← `tests/replay.c`
  - `rb_engine_ported` ← `tests/test_ported_simple.c`
  - `rb_engine_generated` ← `tests/test_ported_generated.c` (mass-ported Rust tests)
  then runs all four. `ALL TESTS PASSED` / `ALL REPLAY CHECKS PASSED` come from these.

**Single public header.** `include/rabuka.h` declares every struct (`GameState`, `RbPlayer`, `RbBag`, `Ability`, `AbilityEffect`, `RbMods`, `RbTempEffect`, `RbAbilityQueue`, …) and all functions. `GameState` is defined mid-file; that is why the drain/owner-lookup declarations live after it.

**The card/ability database (what matters at runtime).**
- At startup the engine calls `rb_load("src")`, which memory-maps `src/cards.bin` (card table, 2526 cards) and `src/abilities_strings.bin` (ability text + string table).
- These are generated, not hand-written. `make regen` runs:
  - `python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs` — `abilities_gen.rs` is the Rust engine's generated ability DB (itself produced from `cards/abilities.json`).
  - `python3 tools/gen_bytecode.py` — emits the compact encoded effect/condition bytecode in `src/core/generated/bytecode_blob.c` + `src/core/generated/gen_data.c`.
- `rb_decode_card_by_index` / `rb_decode_card_ability` / `rb_card_num_abilities` read from that blob at runtime, so the C engine behaves off the same data the Rust engine uses.

**Edit-verify loop (per change).**
1. Edit one `.c`/`.h` (e.g. `src/engine.c`, `src/turn/triggers.c`, `include/rabuka.h`).
2. Recompile just that file: `gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -c -o src/engine.o src/engine.c`.
3. Link a scratch harness (e.g. `tests/verify_live_start.c`: `rb_load("src")` + put `sd1-022` on the live zone + 3 stage members) against all the `.o` files and run it to confirm the auto queued, drained, and granted its resource.
4. Final gate: `make all` then `make test` for no regressions.

**Parity caveat (important).** The test binaries assert against the generated ported tests, many of which are still TODOs (e.g. the `sd1-022` LiveStart test only checks a negative and is marked "TODO fire_trigger"). So a green `make test` does **not** fully prove effect parity — which is why the direct verification program is used for individual effect/trigger grants (LiveStart blade grant, debuts, constants, etc.).

**Status as of this commit.** `rb_engine_test` + `rb_engine_replay` pass; `rb_engine_ported` (7) and `rb_engine_generated` (26) have pre-existing failures unrelated to the latest condition port (hanayo `score`/`modify_score`/`recalc_constants` gaps). Build is clean (`-Wall`, warnings only). Throughput observed ~43.3 t/s.

---

## 14. Strategy — translate the WHOLE engine first, then let tests pass

**The Rust tests convert cleanly (see `tools/gen_tests.py`), so the test harness is not the bottleneck.** The bottleneck is that the C engine is still a partial translation: many effect/condition/trigger kinds are stubs or best-effort, so the auto-ported Rust tests fail not because the conversion is wrong, but because the underlying C engine function is not implemented yet.

** therefore the workflow is:**
1. **Translate the engine completely**, file-by-file, mirroring `engine/src/...` (the §12 matrix is the checklist). Each Rust function lands as a faithful, real implementation — no "best-effort returns 0" shortcuts.
2. **Only after a subsystem is fully translated** do we re-run `make test` and fix the now-revealed gaps. The failure count is a *worklist*, not a scoreboard.
3. **The absolute test numbers do NOT matter until EVERYTHING is translated.** A green `make test` before the engine is complete would be misleading; a red one is expected and healthy. Do not cherry-pick tests to make them pass — implement the engine.

**What has been automated (good, keep using it):**
- `tools/gen_tests.py` mass-ports Rust `#[test]` fns → `tests/test_ported_generated.c` via `test_game.h`. It already broadens across `pass()`/debut/`play_to_stage`/board asserts; keep extending it as new engine functions land so coverage tracks the translation.
- `make regen` rebuilds `cards.bin`/`bytecode_blob.c` from `engine/src` data — the C engine always runs off the same card/ability DB as Rust.

**Next concrete engine translations (priority order, from §12.8):** (1) ~~`check_timing` hooks at `Active→Energy→Draw→LiveSet→Performance` + constant heart/need_heart recalc~~ — **DONE** (see §12.8); (2) ~~full `ability/cost.rs` pay gate (`sequential_cost`, `optional_cost`, `change_state wait`)~~ — **DONE (headless subset)** (see §12.8); (3) `turn/live.rs` `re_yell`→`pending_reyell_rebuild` + `LiveSuccess` trigger → `compute_surplus_and_flags` oracle diff; (4) `effects/move_cards` relay pools (`ThoseCards`/`RecentlyMoved`/`LookedAtRemaining`) + `effects/look` `keep_shuffle_under`; (5) `ability_queue` `QueueState` FSM + `ChoiceRoute`/`ConditionalChoice` routing. Each slots into `src/phase.c:rb_advance_phase`, `src/live.c:rb_perform_live`, `src/effects_*`, `src/choice.c`, `src/ability_queue.c`.
