# engine_c — C Port of the Rabuka Engine

**Status: Foundation v0 — compiles, decodes, and runs a demo match on PC.** Not yet a faithful port. The data pipeline + decoder + skeleton turn loop are proven; the remaining work — faithful effect execution, conditions, choices, triggers, and the full Live/performance machine — slots in without restructuring.

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
| Effect executor | `src/engine.c:rb_execute_effect/handle_action` | ⚠️ subset | ~10 verbs real (draw, gain/lose_energy, gain/place_heart, damage, heal, gain_score, move_cards, change_state, shuffle, discard); ~30 verbs stubbed as no-ops; children executed pre-order |
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
| 271 | `gain_resource` | `ability/effects/state.rs` | ⚠️ energy count only (no under-member/baton interaction) | P0 |
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
| 255 | `ライブ開始時` (LiveStart) | `triggers.rs` | 🔴 |
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
| `ability/vm.rs` + `ability/condition_decoder_gen.rs` + `ability/effect_decoder_gen.rs` | `src/vm.c` | ✅ foundation done; extend for any newly-added wire keys (e.g. `choice_maker`, `looked_at_deck_position`) |
| `core/card.rs` (4138 LOC) + `core/card_binary.rs` | `src/cards.c` | ✅ done; add `blade_heart` / `need_heart` split when Live phase needs it |
| `core/zones.rs` + `core/player.rs` + `core/constants.rs:MAX_LIVE_CARDS=3` | `include/rabuka.h:RbPlayer/RbBag/RbZone` + `src/engine.c:do_move` | ⚠️ bags are flat vectors; need `stage[3]` strict, waitroom/energy as typed zones, cap enforcement |
| `core/game_modifiers.rs` + `core/game_state/modifiers.rs` + `core/stats_pipeline.rs` | — (new `src/modifiers.c`) | 🔴 needed: per-card heart/score/blade/cost modifiers, constant abilities, timed-expiry |
| `core/pool.rs` + `core/types.rs` | — | 🔴 needed for heart pool / blade accounting in Live |
| `ability/effects/{draw,score,state,misc,ability_effects}.rs` | `src/engine.c:handle_action` | ⚠️ 10/42 verbs; remainder need individual `else if` clauses mirroring Rust handlers |
| `ability/condition/{card,compound,state}.rs` (condition.rs 1039 LOC) | `src/condition.c` (new) | 🔴 full Condition eval tree + `rb_eval_condition(g, actor, cond)` |
| `ability/compound.rs` + `ability/choice.rs` (3447 LOC) | `src/choice.c` (new) | 🔴 sequential gates, conditional_on_*, choice spawning, pay-skip gate, repeat_procedure |
| `ability/cost.rs` + `ability/resolver.rs` + `ability_queue.rs` + `triggers.rs` | `src/ability_queue.c` + `src/triggers.c` | 🔴 ability queue (debut/auto/live_start/live_success), cost payment, use_limit, cost_paid/effect_started flags |
| `turn/phases.rs` (1685 LOC) + `turn/actions.rs` + `turn/live.rs` (2846 LOC) + `turn/triggers.rs` | `src/engine.c` + new `src/live.c` + `src/phase.c` | 🔴 full phase machine (see §4), baton touch, yell → heart allocation → verdict → score → victory |
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

### Phase 1 — Core state & modifiers (est. 2–3 sessions)

**Scope:** Make `GameState` faithful enough that conditions and effects read correct state.

- `src/modifiers.c` — `RbMods { heart_modifiers, need_heart_modifiers, score_modifiers, blade_modifiers, cost_modifiers, orientation_modifiers, ... }` mirroring `core/game_modifiers.rs` (modifier stacking: set vs. additive, `ModifierEntry { set, additive, total() }`, `saturate_u8`).
- `core/constants.rs` → `include/constants.h` — `RB_MAX_LIVE_CARDS=3`, `RB_SCORE_WIN=7`, `RB_ENERGY_CAP=7`, `RB_STAGE_SIZE=3`, `RB_MAX_HAND=40`, zone caps, etc. (no magic numbers).
- Expand `RbPlayer` — split `hearts[col]` (ability-granted pool) from `stage_hearts` (computed pipeline) from `blade_hearts`; add `deployed_this_turn[]`, `debut_count_this_turn`, `stage_wait[]` vs. `orientation_modifiers`.
- `RbBag` helpers — cap-checked `bag_push/pop/remove_at`, typed `zone_bag(pl, zone)`, len tables.
- **Files to touch:** `include/rabuka.h`, `src/modifiers.c` (new), `src/engine.c` (zone helpers), `src/stats_pipeline.c` stub.
- **Dep of:** Phase 2 and 4.
- **Verify:** `tests/test_modifiers.c` — modifier add/remove/saturate, constant ability registration; `make test` still green.

### Phase 2 — Condition evaluation (est. 2 sessions, parallelizable with Phase 1 by a second worker)

**Scope:** Every condition-gated effect either fires or correctly skips.

- `src/condition.c` — `rb_eval_condition(g, actor, cond)` dispatching on `cond->variant` (0..19) and field keys, mirroring `ability/condition/{card,compound,state}.rs`:
  - `card_count_condition` / `location_condition` / `group_condition` / `movement_condition` / `appearance_condition` / `position_condition` — scan zones with typed `Zone` + `group_names` + `exclude_characters`.
  - `comparison_condition` / `or_condition` / `compound` — recursive eval + int/float compares.
  - `temporal_condition` / `state_condition` / `energy_state_condition` / `score_threshold_condition` — turn/phase/score/energy reads.
  - Cache semantics (`cache:true` memoization via `condition_cache[]` on queue entry).
- Wire into `rb_execute_effect` — `if (e->has_condition && !rb_eval_condition(g, actor, e->condition)) return;` before action dispatch; also gate `sequential` children individually (see `ability/compound.rs`).
- **Files:** `src/condition.c` (new), `src/vm.c` (variant mapping header), `src/engine.c` (gate).
- **Verify:** `tests/test_condition.c` — fixtures covering each condition type (group distinct, location empty_area, card_count with/without distinct, movement `has_moved` post-baton, score threshold). Parity with Rust `condition::*_test` via JSON fixtures.

### Phase 3 — Choice & ability queue (est. 3 sessions, depends on Phase 2)

**Scope:** Effects that pause for human/bot decisions correctly present options and resume.

- `src/choice.c` + `src/ability_queue.c`:
  - `Choice` enum (SelectCard { zone, card_type, count, allow_skip, heart_colors }, SelectHeartColor, SelectTarget { target=PAY_SKIP etc. }, SelectPosition, SelectAutoAbility ordering) — mirror `ability/choice.rs` + `ability/types.rs:Choice`.
  - `AbilityQueue { entries[] { ability, ability_index, activating_card, cost_paid, effect_started, choice_card_no, pending_repeat_actions, condition_cache } }` — echo `ability_queue.rs` (queue depth ~16).
  - `AbilityResolver { pending_choice, selected_cards[], moved_cards[], last_move_moved_any, formation_plan[], pending_deferred_costs[], ... }` — minimal field set to unblock `compound` sequential + `look_and_select` + `draw_any_number`.
  - `rb_resolve_ability(g, actor, ability)` — cost payment (`pay_cost` → optional `emit_pay_skip_gate`), `can_activate_effect` gate, `use_limit` tracking, `store_pending_choice` → `gs->pending_choice`.
  - `rb_resume_with_choice(g, choice_idx)` — route back into `handle_action`/`compound` continuation (deferred conditional gate: skip answer drops remaining sequential actions).
- **Files:** `src/choice.c` (new), `src/ability_queue.c` (new), `include/rabuka.h` (`Choice`, `AbilityQueue`, `pending_choice`), `src/engine.c` (integrate).
- **Verify:** `tests/test_choice.c` — `look_and_select` with skip/keep-N-shuffle-under, `select_cards` count validation, `pay_skip` gate, `conditional_on_optional` skip drops remainder, `ability_use_limit` consumption. Mirrors `resolver.rs:resolve_ability` lifecycle table (see that 300-line comment for recording semantics — preserve distinctions).

### Phase 4 — Full phase machine + triggers (est. 2–3 sessions, depends on Phase 1)

**Scope:** The match progresses through the same phase sequence as Rust, auto-abilities fire at the right timing, and victory is computed identically.

- `src/phase.c` (or expanded `src/engine.c`):
  - Two-turn-phase structure (`TurnPhase { FirstAttackerNormal, SecondAttackerNormal, Live }` + `RbPhase { RPS, MulliganFirst, MulliganSecond, Active, Energy, Draw, Main, LiveSetFirst, LiveSetSecond, FirstPerf, SecondPerf, LiveVictory }`).
  - `rb_advance_phase(g)` — `match turn_phase { Normal => Active→Energy→Draw→Main (×2), Live => LiveSetFirst→Second→FirstPerf→SecondPerf→Victory→rollover }` mirroring `turn/phases.rs:advance_phase` (including `check_timing` calls + `delayed_cannot_active` ticks + `deployed_this_turn` clears).
  - `rb_handle_mulligan(g, choice)`, `rb_handle_live_set(g, choice)` — typed stubs that the choice system can drive.
  - `src/triggers.c` — `TriggerKind` scan (`triggers.rs:canonical_trigger`) + `trigger_auto_abilities_for_player(g, pid)` + `trigger_live_start_abilities` + `trigger_live_success_abilities`.
  - Victory — `rb_check_victory(g)` = 3-success check + `RB_SCORE_WIN` + deck-out + two-draw, matching `turn/actions.rs::check_victory_condition` tie logic.
- **Files:** `src/phase.c` (new), `src/triggers.c` (new), `src/engine.c` (wire), `src/modifiers.c` (constant re-eval hook).
- **Verify:** `tests/test_phases.c` — RPS→mulligan→active→energy→draw→main→live-set→performance→victory walk with fixed RNG; `tests/test_triggers.c` — debut fires once, LiveStart queues before performance, `repeat_procedure` loops.

### Phase 5 — Live / performance (est. 4–5 sessions, depends on Phases 1 + 4)

**Scope:** A yelled Live produces the same hearts, allocations, verdicts, scores, and snapshots as Rust.

- `src/live.c` (≈ Rust `turn/live.rs` + `core/stats_pipeline.rs` + `core/pool.rs`):
  - `yell` — per live `yell_count` (+ `modify_yell_count`/`modify_yell_source`) deck reveals → `YellCardResult { card_id, blade_hearts[8], note_icons }` with `b_heart07` doubling + `set_blade_type` recolor + `BAll→icon_all[7]` wildcard.
  - `stage_hearts` — computed via `calc_stage_hearts(card_db, modifiers)` (single source; `player_perform_live` and `rebuild_stage_hearts_with_yell` both call it).
  - `allocation` — greedy `Allocation { target_idx, color, amount, phase }` planner (H00Wild→Wildcard→AllWild→CAll) filling each live's `required[8]` from `total_hearts[8]`.
  - `verdict` — per-live `filled[8]` vs `required[8]` with icon_all covering, heart0 bucket, `passed` flag, per-live `score`.
  - `re_yell` / `perform_yell` deferred rebuild (`pending_reyell_rebuild`).
  - Snapshots — `LivePerformanceData { player_id, turn, lives[], total_hearts[8], allocations[], breakdown, member_contributions[], total_score, success, surplus_hearts[8] }` pushed per performance.
  - Late scoring — `live_success` trigger bonuses via `p1_extra/p2_extra` delta, `drain_pending_live_success_choices`, `merge_late_score_apps`, `compute_surplus_and_flags`, `move_live_to_success_and_handle_wins` (prohibition_effects on ties), first-attacker flip.
- **Files:** `src/live.c` (new, ~1500 LOC target), `src/stats_pipeline.c` (new), `include/rabuka.h` (`LivePerformanceData`, `Allocation`, `YellCardResult`), `src/engine.c` (`performance()` replaced).
- **Verify:** `tests/test_live.c` — snapshot parity fixtures: known decks → known performance snapshots (allocations/required/filled/passed/scores/surplus) diffed against Rust oracle dumps. This is the single most sensitive subsystem; budget extra time and add `log::debug!`-style trace behind a `RB_TRACE` flag.

### Phase 6 — Effect verb completion (est. 3–4 sessions, parallelizable after Phase 5)

**Scope:** Remaining 30 verb handlers implemented to match `ability/effects/*.rs` + `ability/move_cards.rs`.

Grouped by implementation cluster:

- **Movement cluster** (`move_cards.rs`, ~3780 LOC) — `move_cards` with `source/destination` zone typed dispatch (Hand/Stage/Waitroom/Energy/Deck/DeckTop/Bottom/Success/Resolution/RecentlyMoved/ThoseCards/LookedAtRemaining/DeckTopOrBottom), `card_type` filters, `group` filters, `count` semantics, baton cost-reduction, `those_cards` retry, back-filling after Stage vacate.
- **Look/select cluster** — `look_at` / `look_and_select` / `select_cards` / `select` / `reveal*` creating `Choice::SelectCard` with `zone=looked_at` pools, `revealed_cards` vs. `looked_at` distinction, `keep_shuffle_under` 2-phase.
- **State cluster** — `change_state` + `position_change`/`rotation`/`formation_plan` swap batch, `place_energy_under_member`, `choose_required_hearts`, `set_blade_type`/`set_blade_count`/`set_heart_type` property rewrites (must persist via modifiers, not flat fields).
- **Cost/modifier cluster** — `modify_cost`/`set_cost`/`set_cost_to_use`/`modify_yell_*` (per-card cost/yell state, reset semantics), `modify_required_hearts`/`_global`/`_success` via `need_heart_modifiers`, `gain_resource`/`pay_energy` with under-member energy counting, `reduce_live_card_set_limit`.
- **Ability cluster** — `gain_ability`/`gain_ability_from_source` → `push_temporary_effect` with expiry, `invalidate_ability`/`suppress_ability_trigger` revocation maps, `activate_ability`, `gain_ability` delayed `EffectData`.
- **Compound control** — `sequential` (deferred gate on choice), `conditional_alternative` / `conditional_on_result` / `conditional_on_optional` (alternative selection), `repeat_procedure` (one-at-a-time feeding), `choice` dispatch, `restriction`/`modify_limit` enforcement.
- **Utility** — `shuffle` (Fisher-Yates, matches Rust `shuffle`), `discard_until_count`, `re_yell`/`perform_yell` deferred path, `play_baton_touch`/`double_baton_touch`, `choose_target_player`/`select_number`.

**Files:** `src/effects_move.c`, `src/effects_look.c`, `src/effects_state.c`, `src/effects_ability.c`, `src/compound.c` (or grouped as `src/effects.c` + helpers).
**Verify:** Per-verb micro-fixtures + the replay harness (Phase 7).

### Phase 7 — Parity harness & CI (est. 2 sessions, start after Phase 2, finish after Phase 6)

**Scope:** Any behavioral drift from Rust is caught by automated replay.

Three tiers:

1. **JSON scenario-replay harness** (`tests/replay.c` + `tests/fixtures/*.json`):
   - Extract fixtures from Rust `engine/tests/**` — each test already sets up `GameState { player1, player2, card_database, turn_number, current_phase }` + an action sequence + expected final zone/score/success. Add a CI job `cargo test -- --format json` dumper or hand-author 20-30 representative fixtures (debut, LiveStart, LiveSuccess, cost-gated activation, baton, re-yell, mulligan, prohibition tie).
   - Harness loads fixture → `rb_load` → `rb_game_init` with explicit `GameState` seed (not demo decks) → drives `rb_advance_phase` / `rb_resume_with_choice` → asserts final snapshot equals expected (and equals Rust oracle run on same fixture).
2. **Property tests** — random deck generation + random legal-action walk, assert engine never panics / never produces impossible phase, zones stay in caps, winner eventually decided or turn limit.
3. **Golden snapshot** — `cargo run --bin trace_game` oracle dumps (performance snapshots, verdicts, rule_log) diffed against C `--trace` output for 100 random seeds.

**Wire into CI:** `make audit && make test && make replay` must be green before any `engine_c` PR merges. Document in `cards/test_inventory.py --check` style.

### Phase 8 — Portable targets (after Phase 5, parallelizable; est. 1–2 sessions)

General C — no platform-specific logic in `src/`; every port is a thin `platforms/<target>/` shim over the same engine.

- **Allocator abstraction** — `RB_NO_MALLOC` bump-alloc fallback (`src/alloc.c`): `rb_malloc`/`rb_free` route to `malloc` on hosted (PC), to a static arena on bare metal. `rb_unload` is a no-op on arena targets; PC build free-checks under ASan/Valgrind.
- **Data streaming** — `rb_load_streaming(dir, read_fn)` alternative to `rb_load(dir)`: cards/strings/bytecode can be `fread` from host FS *or* streamed from ROM/CD/flash sector-by-sector. 1 MB CD-i, 2 MB DS, etc. don't RAM-load all tables at once — stream `bytecode_blob` + `cards.bin` on demand (`rb_card_record` / `rb_bc_slice` backed by a read cache). See `docs/PORTS.md` for per-target budgets (CD-i 1 MB wall included as one data point, not the design center).
- **Platform shims** — `platforms/sdl/main.c` (hosted reference: window + input → `Choice` selection, mirrors `ports/3ds` pattern), `platforms/cdi/cdi_main.c` and `platforms/ds/main.c` etc. each only provide: `platform_read_file`, `platform_input_poll`, `platform_render_text`, `platform_random_seed`. Engine never calls `fopen`/`printf` directly outside `src/main.c`.
- **Toolchain** — `gcc ≥ 9` / `clang` on PC, `m68k-elf-gcc` (CD-i), `arm-none-eabi-gcc` (GBA/DS/3DS), `mipsel` etc. — all `-std=c11 -ffreestanding` clean. No C++ runtime, no external libs, no `fs` dependency in `src/vm.c`/`src/engine.c`.
- **Verify:** Each shim boots, seeds RNG, completes `make -C platforms/<target>` + runs a seeded match within its RAM budget (CD-i: trace mass must stream, not cache).

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

*This document is the single source of truth for the C rewrite. Every `-[ ]` above corresponds to a file/task that will land as a separate commit on top of `engine_c-v0`. Keep this file updated as each phase lands; the per-phase checklists are the PR checklists.*
